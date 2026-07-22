use puzzle_core::{
    GridCompiledGame, GridSize, GridState, InputId, ObjectId, Size2, Size3, State as PuzzleState,
};
#[cfg(feature = "editor-debug")]
use puzzle_core::{
    GridCoord, GridPatch, MarkId, MarkValueMatch as CoreMarkValueMatch, PatchOp, RuleId,
    TransitionCommand, VariableId, VariableUpdateOp,
};
use puzzle_lang::{
    ArrowKey, KeyTrigger, Level, LevelId, LoadedDocument, LoadedDocumentModel, LoadedGame,
    LoadedGridGame, ResourceSelection, SceneAlignDef, SceneComponent, SceneDef,
    SceneDistributionDef, SceneEffect, SceneExpr, SceneLayoutDef, ScenePuzzleInitializer,
    SceneSpaceDef, SceneStateLifetime, SceneTextContent, SceneTextRoleDef, SceneTransitionTrigger,
    SceneValue, ThemeDef, ViewportModeDef, ViewportProjectionDef, ViewportSizeDef,
};
#[cfg(feature = "editor-debug")]
use puzzle_play::GridTransitionTrace;
use puzzle_play::{
    GameSession, GridGameSession, LevelProgressSaveData, PersistentVarSaveData, ProgressSaveData,
    presentation_events_contract, runtime_sounds_def,
};
use puzzle_runtime_contract::{
    RuntimeStateSnapshot2d, RuntimeStateSnapshot3d, STANDALONE_RUNTIME_EXPORT_VERSION,
    SessionAction, SolverStateSnapshot, StandaloneRuntimeExport,
};
use serde_json::{Value, json};

pub struct StandaloneSessionBridge {
    model: Box<dyn StandaloneSessionModel>,
    has_progress_save: bool,
}

trait StandaloneSessionModel {
    fn snapshot_value(&mut self, has_progress_save: bool) -> Value;
    fn is_waiting(&self) -> bool;
    fn resume_wait(&mut self) -> Result<(), String>;
    fn apply_component_event(&mut self, instance: &str, event: &str) -> Result<(), String>;
    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String>;
    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String>;
    fn apply_command_name(&mut self, command_name: &str) -> Result<(), String>;
    fn apply_scene_effect(&mut self, effect: &SceneEffect) -> Result<(), String>;
    fn undo(&mut self);
    fn redo(&mut self);
    fn restart(&mut self) -> Result<(), String>;
    fn next_level(&mut self);
    fn previous_level(&mut self);
    fn goto_level(&mut self, level: usize) -> Result<(), String>;
    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String>;
    fn progress_save_json(&self) -> String;
    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String>;
}

struct GridSessionRuntime<const D: usize, Size: GridSize<D>, Projection> {
    loaded: LoadedGridGame<D, Size>,
    session: GridGameSession<D, Size>,
    projection: Projection,
}

trait GridSessionProjection<const D: usize, Size: GridSize<D>> {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<D>,
        state_json: &str,
    ) -> Result<GridState<D, Size>, String>;

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        session: &GridGameSession<D, Size>,
    ) -> ProjectedGridSnapshot;

    fn solver_state(&self, state: &GridState<D, Size>) -> SolverStateSnapshot;
}

struct ProjectedGridSnapshot {
    scene: Value,
    scene_puzzle_state: Value,
    scene_layers: Vec<Value>,
}

#[derive(Default)]
struct CanvasProjection;

#[derive(Default)]
struct SpatialProjection;

impl StandaloneSessionBridge {
    pub fn from_source(source: &str, puzzle_path: &str) -> Result<Self, String> {
        let document = puzzle_lang::parse_game_for_path(source, puzzle_path)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            model: standalone_session_model(document)?,
            has_progress_save: false,
        })
    }

    pub fn from_export_json(export_json: &str) -> Result<Self, String> {
        let export: StandaloneRuntimeExport<LoadedDocument> = serde_json::from_str(export_json)
            .map_err(|error| format!("invalid standalone runtime export: {error}"))?;
        let runtime_bundle = export.runtime_loaded_document;
        if runtime_bundle.version != STANDALONE_RUNTIME_EXPORT_VERSION {
            return Err(format!(
                "unsupported runtimeLoadedDocument version: {}",
                runtime_bundle.version
            ));
        }
        Ok(Self {
            model: standalone_session_model(runtime_bundle.document)?,
            has_progress_save: false,
        })
    }

    pub fn snapshot_json(&mut self) -> String {
        serde_json::to_string(&self.model.snapshot_value(self.has_progress_save))
            .expect("snapshot JSON should serialize")
    }

    pub fn dispatch(&mut self, action: SessionAction) -> Result<String, String> {
        if self.model.is_waiting()
            && !matches!(
                &action,
                SessionAction::Snapshot
                    | SessionAction::Resume
                    | SessionAction::ComponentEvent { .. }
            )
        {
            return Err("session action is unavailable while a turn is waiting".to_string());
        }
        match action {
            SessionAction::Snapshot => Ok(self.snapshot_json()),
            SessionAction::Resume => {
                self.model.resume_wait()?;
                Ok(self.snapshot_json())
            }
            SessionAction::ComponentEvent { instance, event } => {
                self.model.apply_component_event(&instance, &event)?;
                Ok(self.snapshot_json())
            }
            SessionAction::Undo => {
                self.model.undo();
                Ok(self.snapshot_json())
            }
            SessionAction::Redo => {
                self.model.redo();
                Ok(self.snapshot_json())
            }
            SessionAction::Restart => {
                self.model.restart()?;
                Ok(self.snapshot_json())
            }
            SessionAction::NextLevel => {
                self.model.next_level();
                Ok(self.snapshot_json())
            }
            SessionAction::PreviousLevel => {
                self.model.previous_level();
                Ok(self.snapshot_json())
            }
            SessionAction::GotoLevel { level } => {
                self.model.goto_level(level)?;
                Ok(self.snapshot_json())
            }
            SessionAction::Input { name } => {
                self.apply_input_name(&name)?;
                Ok(self.snapshot_json())
            }
            #[cfg(feature = "editor-debug")]
            SessionAction::DebugInput { name } => self.apply_debug_input_name_json(&name),
            #[cfg(not(feature = "editor-debug"))]
            SessionAction::DebugInput { .. } => {
                Err("debug input is unavailable in the player runtime".to_string())
            }
            SessionAction::Command { name } => {
                self.apply_command_name(&name)?;
                Ok(self.snapshot_json())
            }
            SessionAction::SceneEffect { effect } => {
                self.model.apply_scene_effect(&effect)?;
                Ok(self.snapshot_json())
            }
        }
    }

    pub fn dispatch_json(&mut self, action_json: &str) -> Result<String, String> {
        let action: SessionAction = serde_json::from_str(action_json)
            .map_err(|error| format!("invalid session action: {error}"))?;
        self.dispatch(action)
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        self.model.apply_input_name(input_name)
    }

    #[cfg(feature = "editor-debug")]
    pub fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        self.model.apply_debug_input_name_json(input_name)
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.model.apply_command_name(command_name)
    }

    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        self.model
            .set_current_state_json(state_json, level_index, materialize_level_start)
    }

    pub fn progress_save_json(&self) -> String {
        self.model.progress_save_json()
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        self.model.restore_progress_save_json(save_json)?;
        self.has_progress_save = true;
        Ok(())
    }

    pub fn mark_progress_save_written(&mut self) {
        self.has_progress_save = true;
    }

    pub fn clear_progress_save(&mut self) {
        self.has_progress_save = false;
    }
}

fn standalone_session_model(
    mut document: LoadedDocument,
) -> Result<Box<dyn StandaloneSessionModel>, String> {
    if document.models.len() != 1 {
        return Err(
            "a document with multiple puzzle worlds requires model-addressed session routing"
                .to_string(),
        );
    }
    match document.models.first().expect("single model was checked") {
        LoadedDocumentModel::Puzzle2d { game, .. } => game.validate_program_references()?,
        LoadedDocumentModel::Puzzle3d { game, .. } => game.validate_program_references()?,
    }
    match document.models.pop().expect("single model was checked") {
        LoadedDocumentModel::Puzzle2d { game: loaded, .. } => Ok(Box::new(GridSessionRuntime {
            session: GridGameSession::new(&loaded),
            loaded,
            projection: CanvasProjection,
        })),
        LoadedDocumentModel::Puzzle3d { game: loaded, .. } => Ok(Box::new(GridSessionRuntime {
            session: GridGameSession::new(&loaded),
            loaded,
            projection: SpatialProjection,
        })),
    }
}

impl<const D: usize, Size, Projection> StandaloneSessionModel
    for GridSessionRuntime<D, Size, Projection>
where
    Size: GridSize<D> + 'static,
    Projection: GridSessionProjection<D, Size> + 'static,
{
    fn snapshot_value(&mut self, has_progress_save: bool) -> Value {
        let presentation_events = self.session.take_presentation_events();
        let busy = self.session.is_waiting();
        let projected = self.projection.snapshot_grid(&self.loaded, &self.session);
        let surface = surface_value(&self.session, projected.scene_layers.clone());
        let solver_state = self.projection.solver_state(self.session.state());
        let scene_state = scene_state_value(self.session.scene_state());
        let scene_puzzles = scene_puzzles_value(self.session.scene_state());
        let levels = level_records_value(&self.loaded, &self.session);
        json!({
            "title": self.loaded.title,
            "subtitle": self.loaded.subtitle,
            "author": self.loaded.author,
            "homepage": self.loaded.homepage,
            "has_progress_save": has_progress_save,
            "sounds": sounds_value(&self.loaded),
            "theme": theme_value(&self.loaded.theme),
            "defaultWaitMs": self.loaded.default_wait_ms,
            "inputBuffer": {
                "queueDuringWait": self.loaded.input_buffer.queue_during_wait,
                "fastForwardWait": self.loaded.input_buffer.fast_forward_wait,
                "minWaitMs": self.loaded.input_buffer.min_wait_ms,
            },
            "animation": animation_value(&self.loaded),
            "presentationEvents": presentation_events_contract::<D>(&presentation_events),
            "levelIndex": self.session.active_level_index(),
            "levelCount": self.loaded.levels.len(),
            "levels": levels,
            "scene": projected.scene,
            "acceptsModelInput": self.session.accepts_model_input(&self.loaded),
            "gameState": scene_values_value(self.session.session_values()),
            "sceneState": scene_state,
            "scenePuzzles": scene_puzzles,
            "scenePuzzleState": projected.scene_puzzle_state,
            "surface": surface,
            "solverState": solver_state,
            "selectedLevelIndex": self.session.selected_level_index(),
            "busy": busy,
            "canUndo": self.session.can_undo(),
            "canRedo": self.session.can_redo(),
            "inputs": inputs_value(&self.loaded),
            "scenes": scenes_value(&self.loaded),
        })
    }

    fn is_waiting(&self) -> bool {
        self.session.is_waiting()
    }

    fn resume_wait(&mut self) -> Result<(), String> {
        self.session
            .resume_wait(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_component_event(&mut self, instance: &str, event: &str) -> Result<(), String> {
        self.session
            .apply_component_event(&self.loaded, instance, event)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))
    }

    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_traced_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))?;
        let debug = self.session.last_transition_trace().cloned();
        Ok(json!({
            "snapshot": self.snapshot_value(false),
            "debug": debug_transition_value_grid(&self.loaded, debug.as_ref()),
        })
        .to_string())
    }

    fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.session
            .apply_command(&self.loaded, command_name)
            .map_err(|error| format!("{error:?}"))
    }

    fn apply_scene_effect(&mut self, effect: &SceneEffect) -> Result<(), String> {
        self.session
            .apply_scene_effect(&self.loaded, effect)
            .map_err(|error| format!("{error:?}"))
    }

    fn undo(&mut self) {
        self.session.undo(&self.loaded);
    }

    fn redo(&mut self) {
        self.session.redo(&self.loaded);
    }

    fn restart(&mut self) -> Result<(), String> {
        self.session
            .restart_level(&self.loaded)
            .map_err(|error| format!("{error:?}"))
    }

    fn next_level(&mut self) {
        self.session.advance_level(&self.loaded);
    }

    fn previous_level(&mut self) {
        self.session.previous_level(&self.loaded);
    }

    fn goto_level(&mut self, level: usize) -> Result<(), String> {
        if level >= self.loaded.levels.len() {
            return Err(format!("level index out of range: {level}"));
        }
        self.session.start_level(&self.loaded, level);
        Ok(())
    }

    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        if level_index >= self.loaded.levels.len() {
            return Err(format!("level index out of range: {level_index}"));
        }
        let state = self
            .projection
            .decode_state_json(&self.loaded.game, state_json)?;
        self.session
            .start_level_from_state(&self.loaded, level_index, state, materialize_level_start)
            .map_err(|error| format!("{error:?}"))
    }

    fn progress_save_json(&self) -> String {
        progress_save_data_value(&self.session.progress_save_data(&self.loaded)).to_string()
    }

    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        let save = progress_save_data_from_json(save_json)?;
        self.session
            .restore_progress_save_data(&self.loaded, &save)
            .map_err(|error| format!("{error:?}"))
    }
}

impl GridSessionProjection<2, Size2> for CanvasProjection {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<2>,
        state_json: &str,
    ) -> Result<GridState<2, Size2>, String> {
        serde_json::from_str::<RuntimeStateSnapshot2d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn snapshot_grid(
        &self,
        loaded: &LoadedGame,
        session: &GridGameSession<2, Size2>,
    ) -> ProjectedGridSnapshot {
        ProjectedGridSnapshot {
            scene: focused_scene_value(loaded, session),
            scene_puzzle_state: scene_puzzle_state_value(loaded, session),
            scene_layers: scene_layers_value(loaded, session),
        }
    }

    fn solver_state(&self, state: &GridState<2, Size2>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state2(state)
    }
}

impl GridSessionProjection<3, Size3> for SpatialProjection {
    fn decode_state_json(
        &self,
        game: &GridCompiledGame<3>,
        state_json: &str,
    ) -> Result<GridState<3, Size3>, String> {
        serde_json::from_str::<RuntimeStateSnapshot3d>(state_json)
            .map_err(|error| error.to_string())?
            .into_state(game)
    }

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<3, Size3>,
        session: &GridGameSession<3, Size3>,
    ) -> ProjectedGridSnapshot {
        let scene_puzzle_state = spatial_scene_puzzle_state_value(loaded, session);
        let focused_scene = session.surface_state().focused_component();
        let scene = session
            .scene_state()
            .and_then(|state| state.puzzles.values().next())
            .map(|world| spatial_world_value(loaded, focused_scene, world))
            .unwrap_or(Value::Null);
        let scene_layers = session
            .surface_state()
            .components()
            .iter()
            .filter(|component| component.id == component.definition)
            .map(|component| {
                let name = &component.definition;
                let state = session.scene_state_for(name);
                let projected = state
                    .and_then(|state| state.puzzles.values().next())
                    .map(|world| spatial_world_value(loaded, name, world));
                json!({
                    "name": name,
                    "focused": name == focused_scene,
                    "scene": projected,
                    "sceneState": scene_state_value(state),
                    "scenePuzzles": scene_puzzles_value(state),
                })
            })
            .collect();
        ProjectedGridSnapshot {
            scene,
            scene_puzzle_state,
            scene_layers,
        }
    }

    fn solver_state(&self, state: &GridState<3, Size3>) -> SolverStateSnapshot {
        SolverStateSnapshot::from_state3(state)
    }
}

fn spatial_scene_puzzle_state_value(
    loaded: &LoadedGridGame<3, Size3>,
    session: &GridGameSession<3, Size3>,
) -> Value {
    let Some(state) = session.scene_state() else {
        return json!({});
    };
    let entries = state
        .puzzles
        .iter()
        .map(|(name, world)| {
            (
                name.clone(),
                spatial_world_value(loaded, session.surface_state().focused_component(), world),
            )
        })
        .collect();
    Value::Object(entries)
}

fn spatial_world_value(
    loaded: &LoadedGridGame<3, Size3>,
    scene_name: &str,
    world: &puzzle_play::GridWorldInstanceState<3, Size3>,
) -> Value {
    let level_index = world.active_level_index.unwrap_or(0);
    let level = loaded.levels.get(level_index);
    let size = world.state.size;
    json!({
        "component": scene_name,
        "levelIndex": level_index,
        "levelCount": loaded.levels.len(),
        "levelName": level.map(|level| &level.name),
        "size": {
            "width": size.width,
            "depth": size.depth,
            "height": size.height,
        },
        "cells": spatial_state_cells_value(&world.state),
        "completed": loaded.is_goal_complete(&world.state),
        "hasNextLevel": level_index + 1 < loaded.levels.len(),
        "hasPreviousLevel": level_index > 0,
    })
}

fn spatial_state_cells_value(state: &GridState<3, Size3>) -> Value {
    let mut cells = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let cell = ((usize::from(z) * usize::from(state.size.depth)) + usize::from(y))
                    * usize::from(state.size.width)
                    + usize::from(x);
                let cell_objects = (0..state.layer_count)
                    .filter_map(|layer| {
                        let slot = cell * usize::from(state.layer_count) + usize::from(layer);
                        let object = state.slots()[slot];
                        (!object.is_empty()).then(|| json!({"id": object.0, "layer": layer}))
                    })
                    .collect::<Vec<_>>();
                if !cell_objects.is_empty() {
                    cells.push(json!({
                        "position": {"x": x, "y": y, "z": z},
                        "objects": cell_objects,
                    }));
                }
            }
        }
    }
    Value::Array(cells)
}

#[cfg(test)]
fn compiled_state_value(state: &PuzzleState) -> Value {
    serde_json::to_value(RuntimeStateSnapshot2d::from_state(state))
        .expect("runtime state snapshot serializes")
}

fn focused_scene_value(loaded: &LoadedGame, session: &GameSession) -> Value {
    if let Some((state, level)) =
        scene_puzzle_state(loaded, session, session.surface_state().focused_component())
    {
        return scene_value_for_state(
            loaded,
            state,
            level,
            scene_resources(loaded, session.surface_state().focused_component()),
        );
    }
    if !loaded.scenes.is_empty() {
        return Value::Null;
    }
    scene_value_for_state(
        loaded,
        session.state(),
        Some(session.current_level(loaded)),
        scene_resources(loaded, session.surface_state().focused_component()),
    )
}

fn scene_value_for_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) -> Value {
    scene_value_for_materialized_state(loaded, state, level, resources, None)
}

fn scene_value_for_materialized_state(
    loaded: &LoadedGame,
    state: &PuzzleState,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
    display_error: Option<String>,
) -> Value {
    let mut cells = Vec::new();
    if display_error.is_none() {
        for y in 0..state.height {
            for x in 0..state.width {
                let mut layers = Vec::new();
                for layer in 0..state.layer_count {
                    let slot = ((usize::from(y) * usize::from(state.width)) + usize::from(x))
                        * usize::from(state.layer_count)
                        + usize::from(layer);
                    let object = state.slots()[slot];
                    if object.is_empty() {
                        continue;
                    }
                    let name = object_name(loaded, object);
                    layers.push(json!({
                        "layer": layer,
                        "objectId": object.0,
                        "object": name,
                        "visual": name,
                    }));
                }
                cells.push(json!({ "x": x, "y": y, "layers": layers }));
            }
        }
    }
    let regions = level
        .map(|level| {
            level
                .regions
                .iter()
                .map(|region| {
                    json!({
                        "index": region.index,
                        "x": region.x,
                        "y": region.y,
                        "width": region.width,
                        "height": region.height,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "width": state.width,
        "height": state.height,
        "layerCount": state.layer_count,
        "settings": puzzle_settings_value(loaded),
        "animation": animation_value(loaded),
        "screen": screen_value(loaded),
        "regions": regions,
        "resources": scene_resources_value(resources),
        "cells": cells,
        "displayError": display_error,
    })
}

fn scene_layers_value(loaded: &LoadedGame, session: &GameSession) -> Vec<Value> {
    session
        .surface_state()
        .components()
        .iter()
        .filter(|component| component.id == component.definition)
        .map(|component| {
            let name = &component.definition;
            let state = session.scene_state_for(name);
            let scene = scene_puzzle_state(loaded, session, name).map(|(puzzle_state, level)| {
                scene_value_for_state(loaded, puzzle_state, level, scene_resources(loaded, name))
            });
            json!({
                "name": name,
                "focused": name == session.surface_state().focused_component(),
                "scene": scene,
                "sceneState": scene_state_value(state),
                "scenePuzzles": scene_puzzles_value(state),
            })
        })
        .collect()
}

fn surface_value<const D: usize, Size: GridSize<D>>(
    session: &GridGameSession<D, Size>,
    authored_components: Vec<Value>,
) -> Value {
    let components = session
        .surface_state()
        .components()
        .iter()
        .map(|component| {
            let mut value = (component.id == component.definition)
                .then(|| {
                    authored_components.iter().find(|candidate| {
                        candidate["name"].as_str() == Some(component.definition.as_str())
                    })
                })
                .flatten()
                .cloned()
                .unwrap_or_else(|| json!({}));
            let object = value
                .as_object_mut()
                .expect("surface component projection must be an object");
            object.insert("id".to_string(), Value::String(component.id.clone()));
            object.insert(
                "definition".to_string(),
                Value::String(component.definition.clone()),
            );
            object.insert(
                "placement".to_string(),
                serde_json::to_value(component.placement)
                    .expect("component placement should serialize"),
            );
            object.insert(
                "visibility".to_string(),
                serde_json::to_value(component.visibility)
                    .expect("component visibility should serialize"),
            );
            object.insert("modal".to_string(), Value::Bool(component.modal));
            if !component.properties.is_empty() {
                object.insert(
                    "properties".to_string(),
                    Value::Object(
                        component
                            .properties
                            .iter()
                            .map(|(name, value)| (name.clone(), scene_value_atom(value)))
                            .collect(),
                    ),
                );
            }
            if let Some(event) = &component.awaited_event {
                object.insert("awaitEvent".to_string(), Value::String(event.clone()));
            }
            value
        })
        .collect::<Vec<_>>();
    json!({
        "root": session.surface_state().root(),
        "focus": session.surface_state().focused_component(),
        "components": components,
    })
}

fn scene_puzzles_value<const D: usize, Size: GridSize<D>>(
    state: Option<&puzzle_play::GridSceneRuntimeState<D, Size>>,
) -> Vec<Value> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    names
        .into_iter()
        .map(|name| Value::String(name.clone()))
        .collect()
}

fn scene_puzzle_state_value(loaded: &LoadedGame, session: &GameSession) -> Value {
    let Some(state) = session.scene_state() else {
        return json!({});
    };
    let mut entries = serde_json::Map::new();
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        let Some(puzzle) = state.puzzles.get(name) else {
            continue;
        };
        let level = puzzle
            .active_level_index
            .and_then(|index| loaded.levels.get(index));
        let entry = match scene_value_for_state(
            loaded,
            &puzzle.state,
            level,
            scene_resources(loaded, session.surface_state().focused_component()),
        ) {
            Value::Object(object) => object,
            _ => serde_json::Map::new(),
        };
        entries.insert(name.clone(), Value::Object(entry));
    }
    Value::Object(entries)
}

fn scenes_value<const D: usize, Size: GridSize<D>>(loaded: &LoadedGridGame<D, Size>) -> Value {
    Value::Array(loaded.scenes.iter().map(scene_def_value).collect())
}

fn scene_def_value(scene: &SceneDef) -> Value {
    json!({
        "name": scene.name,
        "layout": scene_layout_value(&scene.layout),
        "resources": scene_def_resources_value(scene),
        "state": scene_state_def_value(scene),
        "puzzleRule": scene.puzzle_rule.as_ref().map(|rule| json!({
            "target": rule.target,
            "rule": rule.rule,
        })),
        "components": scene.components.iter().map(scene_component_value).collect::<Vec<_>>(),
        "keys": scene.key_bindings.iter().map(|binding| json!({
            "effect": scene_effect_value(&binding.effect),
            "keys": binding.keys.iter().map(key_trigger_name).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "routines": scene.routines.iter().map(|routine| json!({
            "name": routine.name,
            "effect": scene_effect_value(&routine.effect),
        })).collect::<Vec<_>>(),
        "transitions": scene.transitions.iter().map(|transition| {
            let mut value = serde_json::Map::new();
            match &transition.trigger {
                SceneTransitionTrigger::Condition(condition) => {
                    value.insert("condition".to_string(), scene_expr_value(condition));
                }
                SceneTransitionTrigger::Signal(condition) => {
                    value.insert("signal".to_string(), scene_expr_value(condition));
                }
                SceneTransitionTrigger::SceneStart => {
                    value.insert("lifecycle".to_string(), Value::String("scene_start".to_string()));
                }
                SceneTransitionTrigger::LevelStart => {
                    value.insert("lifecycle".to_string(), Value::String("level_start".to_string()));
                }
            }
            value.insert("effect".to_string(), scene_effect_value(&transition.effect));
            Value::Object(value)
        }).collect::<Vec<_>>(),
    })
}

fn scene_def_resources_value(scene: &SceneDef) -> Value {
    json!({
        "levelsMode": resource_selection_mode(&scene.resources.levels),
        "levels": resource_selection_names(&scene.resources.levels),
        "visualsMode": resource_selection_mode(&scene.resources.visuals),
        "visuals": resource_selection_names(&scene.resources.visuals),
    })
}

fn resource_selection_mode(selection: &ResourceSelection) -> &'static str {
    match selection {
        ResourceSelection::All => "all",
        ResourceSelection::Named(_) => "named",
    }
}

fn resource_selection_names(selection: &ResourceSelection) -> Vec<String> {
    match selection {
        ResourceSelection::All => Vec::new(),
        ResourceSelection::Named(names) => names.clone(),
    }
}

fn scene_state_def_value(scene: &SceneDef) -> Value {
    json!({
        "variables": scene.state.variables.iter().map(|variable| json!({
            "name": variable.name,
            "default": scene_default_value(&variable.default),
            "lifetime": scene_state_lifetime_name(variable.lifetime),
            "mutable": variable.mutable,
        })).collect::<Vec<_>>(),
        "puzzles": scene.state.puzzles.iter().map(|puzzle| {
            let mut value = serde_json::Map::new();
            value.insert("name".to_string(), Value::String(puzzle.name.clone()));
            value.insert("model".to_string(), Value::String(puzzle.model.clone()));
            match &puzzle.initializer {
                ScenePuzzleInitializer::CurrentLevel => {
                    value.insert("initializer".to_string(), Value::String("current_level".to_string()));
                }
                ScenePuzzleInitializer::Level(level) => {
                    value.insert("initializer".to_string(), Value::String("level".to_string()));
                    value.insert("level".to_string(), Value::String(level.clone()));
                }
            }
            Value::Object(value)
        }).collect::<Vec<_>>(),
    })
}

fn scene_default_value(value: &SceneValue) -> Value {
    match value {
        SceneValue::Bool(value) => Value::Bool(*value),
        SceneValue::Int(value) => json!(value),
        SceneValue::Text(value) | SceneValue::Symbol(value) => Value::String(value.clone()),
        SceneValue::LevelRef(index) => json!(index),
    }
}

fn scene_state_lifetime_name(lifetime: SceneStateLifetime) -> &'static str {
    match lifetime {
        SceneStateLifetime::Instance => "instance",
        SceneStateLifetime::ResetOnStart => "reset_on_start",
        SceneStateLifetime::Persistent => "persistent",
    }
}

fn scene_layout_value(layout: &SceneLayoutDef) -> Value {
    let mut value = serde_json::Map::new();
    if let SceneSpaceDef::Fill { weight } = layout.space {
        value.insert(
            "space".to_string(),
            json!({ "kind": "fill", "weight": weight }),
        );
    }
    if let Some(ratio) = layout.aspect_ratio {
        value.insert(
            "aspectRatio".to_string(),
            json!({ "width": ratio.width, "height": ratio.height }),
        );
    }
    if let Some(gap) = layout.gap {
        value.insert("gap".to_string(), json!(gap));
    }
    if layout.align != SceneLayoutDef::default().align {
        value.insert(
            "align".to_string(),
            json!(match layout.align {
                SceneAlignDef::Start => "start",
                SceneAlignDef::Center => "center",
                SceneAlignDef::End => "end",
                SceneAlignDef::Stretch => "stretch",
            }),
        );
    }
    if layout.distribute != SceneLayoutDef::default().distribute {
        value.insert(
            "distribute".to_string(),
            json!(match layout.distribute {
                SceneDistributionDef::Start => "start",
                SceneDistributionDef::Center => "center",
                SceneDistributionDef::End => "end",
                SceneDistributionDef::Between => "between",
            }),
        );
    }
    if layout.scroll {
        value.insert("scroll".to_string(), Value::Bool(true));
    }
    Value::Object(value)
}

fn scene_component_value(component: &SceneComponent) -> Value {
    match component {
        SceneComponent::Viewport(viewport) => json!({
            "kind": match viewport.projection {
                ViewportProjectionDef::TwoD => "puzzle",
                ViewportProjectionDef::ThreeD => "puzzle3",
            },
            "source": viewport.source,
            "layout": scene_layout_value(&viewport.layout),
        }),
        SceneComponent::Frame(frame) => json!({
            "kind": frame.kind,
            "source": frame.source,
            "layout": scene_layout_value(&frame.layout),
        }),
        SceneComponent::Text(text) => {
            let mut value = serde_json::Map::new();
            value.insert("kind".to_string(), Value::String("text".to_string()));
            value.insert(
                "role".to_string(),
                Value::String(
                    match text.role {
                        SceneTextRoleDef::Heading => "heading",
                        SceneTextRoleDef::Subheading => "subheading",
                        SceneTextRoleDef::Body => "body",
                        SceneTextRoleDef::Caption => "caption",
                    }
                    .to_string(),
                ),
            );
            match &text.content {
                SceneTextContent::Literal(text) => {
                    value.insert("source".to_string(), Value::String("literal".to_string()));
                    value.insert("value".to_string(), Value::String(text.clone()));
                }
                SceneTextContent::Path(path) => {
                    value.insert("source".to_string(), Value::String("path".to_string()));
                    value.insert("path".to_string(), Value::String(path.join(".")));
                }
                SceneTextContent::Expr(expr) => {
                    value.insert("source".to_string(), Value::String("expr".to_string()));
                    value.insert("content".to_string(), scene_expr_value(expr));
                }
            }
            Value::Object(value)
        }
        SceneComponent::Button(button) => json!({
            "kind": "button",
            "label": scene_expr_value(&button.label),
            "effect": scene_effect_value(&button.effect),
        }),
        SceneComponent::Choice(choice) => json!({
            "kind": "choice",
            "label": scene_expr_value(&choice.label),
            "effect": scene_effect_value(&choice.effect),
        }),
        SceneComponent::Row(container) => json!({
            "kind": "row",
            "layout": scene_layout_value(&container.layout),
            "children": scene_component_list_value(&container.children),
        }),
        SceneComponent::Column(container) => json!({
            "kind": "column",
            "layout": scene_layout_value(&container.layout),
            "children": scene_component_list_value(&container.children),
        }),
        SceneComponent::Box(container) => json!({
            "kind": "box",
            "layout": scene_layout_value(&container.layout),
            "children": scene_component_list_value(&container.children),
        }),
        SceneComponent::Conditional(conditional) => json!({
            "kind": "conditional",
            "condition": scene_expr_value(&conditional.condition),
            "children": scene_component_list_value(&conditional.children),
            "elseChildren": scene_component_list_value(&conditional.else_children),
        }),
    }
}

fn scene_component_list_value(components: &[SceneComponent]) -> Vec<Value> {
    components.iter().map(scene_component_value).collect()
}

fn scene_effect_value(effect: &SceneEffect) -> Value {
    serde_json::to_value(effect).expect("validated scene effect should serialize")
}

fn scene_expr_value(expr: &SceneExpr) -> Value {
    puzzle_scene::scene_expr_json_value(expr)
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}

fn scene_puzzle_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
    scene_name: &str,
) -> Option<(&'a PuzzleState, Option<&'a Level>)> {
    let scene = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)?;
    let state = session.scene_state_for(scene_name)?;
    let puzzle = if let Some(rule) = &scene.puzzle_rule {
        rule.target
            .split('.')
            .next_back()
            .and_then(|puzzle_name| state.puzzles.get(puzzle_name))
    } else {
        first_puzzle_component(&scene.components)
            .and_then(|puzzle_name| state.puzzles.get(puzzle_name))
    }?;
    let level = puzzle
        .active_level_index
        .and_then(|index| loaded.levels.get(index));
    Some((&puzzle.state, level))
}

fn first_puzzle_component(components: &[SceneComponent]) -> Option<&str> {
    for component in components {
        match component {
            SceneComponent::Viewport(viewport) => {
                return Some(viewport.source.as_str());
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(name) = first_puzzle_component(&container.children) {
                    return Some(name);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(name) = first_puzzle_component(&conditional.children) {
                    return Some(name);
                }
                if let Some(name) = first_puzzle_component(&conditional.else_children) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

fn input_id_by_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input_name: &str,
) -> Option<InputId> {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

#[cfg(feature = "editor-debug")]
pub fn debug_transition_value(
    loaded: &LoadedGame,
    debug: Option<&puzzle_play::TransitionTrace>,
) -> Value {
    debug_transition_value_grid(loaded, debug)
}

#[cfg(feature = "editor-debug")]
pub fn debug_transition_value_grid<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    debug: Option<&GridTransitionTrace<D>>,
) -> Value {
    let Some(debug) = debug else {
        return Value::Null;
    };
    json!({
        "kind": "model_input",
        "inputId": debug.input.0,
        "input": loaded.input_labels.get(&debug.input).map(String::as_str).unwrap_or(""),
        "progressed": debug.progressed,
        "observable": debug.observable,
        "cancelled": debug.cancelled,
        "target": debug.target,
        "commands": debug.commands.iter().map(debug_command_value).collect::<Vec<_>>(),
        "executions": debug.firings.iter().enumerate().map(|(index, firing)| {
            json!({
                "index": index,
                "ruleId": firing.rule.0,
                "rule": debug_rule_value(loaded, firing.rule),
                "progressed": firing.progressed,
                "observable": firing.observable,
                "patch": debug_patch_value(loaded, &firing.patch),
            })
        }).collect::<Vec<_>>(),
    })
}

#[cfg(feature = "editor-debug")]
fn debug_rule_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    rule: RuleId,
) -> Value {
    let Some(info) = loaded.rule_debug_info.get(&rule) else {
        return json!({ "id": rule.0 });
    };
    json!({
        "id": rule.0,
        "sourceLine": info.source_line,
        "sourceLineNumber": info.source_line_number,
        "routineStack": info.routine_stack,
    })
}

#[cfg(feature = "editor-debug")]
fn debug_command_value(command: &TransitionCommand) -> Value {
    json!({
        "kind": match command {
            TransitionCommand::Win => "win",
            TransitionCommand::Restart => "restart",
            TransitionCommand::NextLevel => "next_level",
            TransitionCommand::Again => "again",
            TransitionCommand::Checkpoint => "checkpoint",
            TransitionCommand::ClearCheckpoint => "clear_checkpoint",
        }
    })
}

#[cfg(feature = "editor-debug")]
fn debug_patch_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    patch: &GridPatch<D>,
) -> Vec<Value> {
    patch
        .ops()
        .iter()
        .map(|op| debug_patch_op_value(loaded, op))
        .collect()
}

#[cfg(feature = "editor-debug")]
fn debug_patch_op_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    op: &PatchOp<D>,
) -> Value {
    match op {
        PatchOp::Add { position, object } => json!({
            "kind": "add",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Remove { position, object } => json!({
            "kind": "remove",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Move { from, to, object } => json!({
            "kind": "move",
            "from": position_value_from_grid(*from),
            "to": position_value_from_grid(*to),
            "objectId": object.0,
            "object": object_name(loaded, *object),
        }),
        PatchOp::Replace {
            position,
            remove,
            add,
        } => json!({
            "kind": "replace",
            "position": position_value_from_grid(*position),
            "remove": remove.0,
            "add": add.0,
            "removeObject": object_name(loaded, *remove),
            "addObject": object_name(loaded, *add),
        }),
        PatchOp::UpdateVariable {
            variable,
            op,
            value,
        } => json!({
            "kind": "update_variable",
            "variableId": variable.0,
            "variable": variable_name(loaded, *variable),
            "op": match op {
                VariableUpdateOp::Set => "set",
                VariableUpdateOp::Add => "add",
                VariableUpdateOp::Subtract => "subtract",
                VariableUpdateOp::Multiply => "multiply",
                VariableUpdateOp::Divide => "divide",
                VariableUpdateOp::Remainder => "remainder",
            },
            "value": value,
        }),
        PatchOp::SetMark {
            position,
            object,
            mark,
            value,
        } => json!({
            "kind": "set_mark",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
            "mark": mark.0,
            "markName": mark_name(loaded, *mark),
            "value": value,
        }),
        PatchOp::RemoveMark {
            position,
            object,
            mark,
            value,
            match_value,
        } => json!({
            "kind": "remove_mark",
            "position": position_value_from_grid(*position),
            "objectId": object.0,
            "object": object_name(loaded, *object),
            "mark": mark.0,
            "markName": mark_name(loaded, *mark),
            "value": value,
            "match": match match_value {
                CoreMarkValueMatch::Any => "any",
                CoreMarkValueMatch::Exact => "exact",
            },
        }),
    }
}

#[cfg(feature = "editor-debug")]
fn position_value_from_grid<const D: usize>(position: GridCoord<D>) -> Value {
    let axes = position.axes();
    let mut value = serde_json::Map::new();
    value.insert("x".to_string(), json!(axes.first().copied().unwrap_or(0)));
    value.insert("y".to_string(), json!(axes.get(1).copied().unwrap_or(0)));
    if let Some(z) = axes.get(2) {
        value.insert("z".to_string(), json!(z));
    }
    Value::Object(value)
}

fn object_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    object: ObjectId,
) -> String {
    loaded.object_name(object).to_string()
}

#[cfg(feature = "editor-debug")]
fn mark_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    mark: MarkId,
) -> String {
    loaded
        .mark_labels
        .get(&mark)
        .cloned()
        .unwrap_or_else(|| format!("mark#{}", mark.0))
}

#[cfg(feature = "editor-debug")]
fn variable_name<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    variable: VariableId,
) -> String {
    loaded
        .variable_labels
        .get(&variable)
        .cloned()
        .unwrap_or_else(|| format!("var#{}", variable.0))
}

fn object_id_by_name(loaded: &LoadedGame, object_name: &str) -> Option<ObjectId> {
    loaded
        .object_labels
        .iter()
        .find_map(|(id, label)| (label == object_name).then_some(*id))
}

fn sounds_value<const D: usize, Size: GridSize<D>>(loaded: &LoadedGridGame<D, Size>) -> Value {
    serde_json::to_value(runtime_sounds_def(&loaded.sounds))
        .expect("runtime sounds contract should serialize")
}

fn theme_value(theme: &ThemeDef) -> Value {
    let variables = theme
        .variables
        .iter()
        .map(|variable| (variable.name.clone(), Value::String(variable.value.clone())))
        .collect::<serde_json::Map<_, _>>();
    json!({
        "name": theme.name,
        "variables": variables,
    })
}

fn animation_value<const D: usize, Size: GridSize<D>>(loaded: &LoadedGridGame<D, Size>) -> Value {
    json!({
        "tween": {
            "enabled": loaded.animation.tween.enabled,
            "intervalMs": loaded.animation.tween.interval_ms,
        }
    })
}

fn puzzle_settings_value(loaded: &LoadedGame) -> Value {
    json!({
        "render": {},
        "grid": {
            "visibility": loaded.render.grid.occupied_cells || loaded.render.grid.all_cells,
            "occupied_cells": loaded.render.grid.occupied_cells,
            "all_cells": loaded.render.grid.all_cells,
        },
        "inputBuffer": {
            "queueDuringWait": loaded.input_buffer.queue_during_wait,
            "fastForwardWait": loaded.input_buffer.fast_forward_wait,
            "minWaitMs": loaded.input_buffer.min_wait_ms,
        },
        "animation": animation_value(loaded),
    })
}

fn screen_value(loaded: &LoadedGame) -> Value {
    let viewport_size = match loaded.screen.viewport_size {
        ViewportSizeDef::Full => json!({"kind": "full"}),
        ViewportSizeDef::Size { width, height } => {
            json!({"kind": "size", "width": width, "height": height})
        }
    };
    let viewport_mode = match loaded.screen.viewport_mode {
        ViewportModeDef::Paged => "paged",
        ViewportModeDef::Centered => "centered",
    };
    let viewport_focus_objects = loaded
        .object_groups
        .get(&loaded.screen.viewport_focus)
        .cloned()
        .or_else(|| object_id_by_name(loaded, &loaded.screen.viewport_focus).map(|id| vec![id]))
        .unwrap_or_default()
        .into_iter()
        .map(|id| id.0)
        .collect::<Vec<_>>();
    json!({
        "viewportSize": viewport_size,
        "viewportFocus": loaded.screen.viewport_focus,
        "viewportFocusObjects": viewport_focus_objects,
        "viewportMode": viewport_mode,
    })
}

fn inputs_value<const D: usize, Size: GridSize<D>>(loaded: &LoadedGridGame<D, Size>) -> Vec<Value> {
    let mut inputs = loaded.input_labels.iter().collect::<Vec<_>>();
    inputs.sort_by_key(|(id, _)| id.0);
    inputs
        .into_iter()
        .map(|(id, name)| {
            json!({
                "id": id.0,
                "name": name,
                "key": key_for_input(loaded, *id),
                "arrow": arrow_for_input(loaded, *id),
                "keys": key_triggers_for_input(loaded, *id),
            })
        })
        .collect()
}

fn key_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Option<String> {
    loaded
        .controls
        .keys
        .iter()
        .find_map(|(key, id)| (*id == input).then_some(char::from(*key).to_string()))
}

fn arrow_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Option<String> {
    loaded
        .controls
        .arrows
        .iter()
        .find_map(|(arrow, id)| (*id == input).then_some(arrow_name(*arrow).to_string()))
}

fn key_triggers_for_input<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    input: InputId,
) -> Vec<String> {
    let mut keys = Vec::new();
    for (key, id) in &loaded.controls.keys {
        if *id == input {
            keys.push(char::from(*key).to_string());
        }
    }
    for (arrow, id) in &loaded.controls.arrows {
        if *id == input {
            keys.push(arrow_name(*arrow).to_string());
        }
    }
    for (name, id) in &loaded.controls.named {
        if *id == input {
            keys.push(name.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn arrow_name(arrow: ArrowKey) -> &'static str {
    match arrow {
        ArrowKey::Up => "ArrowUp",
        ArrowKey::Down => "ArrowDown",
        ArrowKey::Left => "ArrowLeft",
        ArrowKey::Right => "ArrowRight",
    }
}

fn scene_values_value(values: &std::collections::HashMap<String, SceneValue>) -> Value {
    let entries = values
        .iter()
        .map(|(key, value)| (key.clone(), scene_value_atom(value)))
        .collect::<serde_json::Map<_, _>>();
    Value::Object(entries)
}

fn scene_state_value<const D: usize, Size: GridSize<D>>(
    state: Option<&puzzle_play::GridSceneRuntimeState<D, Size>>,
) -> Value {
    state
        .map(|state| scene_values_value(&state.values))
        .unwrap_or_else(|| json!({}))
}

fn scene_value_atom(value: &SceneValue) -> Value {
    match value {
        SceneValue::Bool(value) => Value::Bool(*value),
        SceneValue::Int(value) => json!(value),
        SceneValue::Text(value) | SceneValue::Symbol(value) => Value::String(value.clone()),
        SceneValue::LevelRef(value) => json!(value),
    }
}

fn scene_resources<'a>(
    loaded: &'a LoadedGame,
    scene_name: &str,
) -> Option<&'a puzzle_lang::SceneResources> {
    loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)
        .map(|scene| &scene.resources)
}

fn scene_resources_value(resources: Option<&puzzle_lang::SceneResources>) -> Value {
    let Some(resources) = resources else {
        return json!({});
    };
    json!({
        "levels": match &resources.levels {
            ResourceSelection::All => json!({"mode": "all", "names": []}),
            ResourceSelection::Named(names) => json!({"mode": "named", "names": names}),
        },
        "visuals": match &resources.visuals {
            ResourceSelection::All => json!({"mode": "all", "names": []}),
            ResourceSelection::Named(names) => json!({"mode": "named", "names": names}),
        },
    })
}

fn progress_save_data_value(save: &ProgressSaveData) -> Value {
    json!({
        "version": save.version,
        "levels": save.levels.iter().map(|level| {
            json!({"id": level.id, "cleared": level.cleared})
        }).collect::<Vec<_>>(),
        "currentLevel": save.current_level,
        "persistentVars": save.persistent_vars.iter().map(|var| {
            json!({"name": var.name, "value": var.value})
        }).collect::<Vec<_>>(),
    })
}

fn level_records_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
) -> Value {
    assert_eq!(
        loaded.levels.len(),
        session.cleared_levels().len(),
        "loaded levels and session progress must have the same length"
    );
    Value::Object(
        loaded
            .levels
            .iter()
            .zip(session.cleared_levels())
            .enumerate()
            .map(|(index, (level, cleared))| {
                let id = LevelId::new(&level.puzzle, &level.name);
                let key = id.record_key();
                (
                    key.clone(),
                    json!({
                        "id": key,
                        "name": level.name,
                        "puzzle": level.puzzle,
                        "pack": level.pack,
                        "ordinal": index + 1,
                        "progress": { "cleared": cleared },
                    }),
                )
            })
            .collect(),
    )
}

fn progress_save_data_from_json(raw: &str) -> Result<ProgressSaveData, String> {
    let value: Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| "progress save is missing version".to_string())?;
    let levels = value
        .get("levels")
        .and_then(Value::as_array)
        .ok_or_else(|| "progress save is missing levels".to_string())?
        .iter()
        .map(|entry| {
            Ok(LevelProgressSaveData {
                id: entry
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "progress save level is missing id".to_string())?
                    .to_string(),
                cleared: entry
                    .get("cleared")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| "progress save level is missing cleared".to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let persistent_vars = value
        .get("persistentVars")
        .and_then(Value::as_array)
        .ok_or_else(|| "progress save is missing persistentVars".to_string())?
        .iter()
        .map(|entry| {
            Ok(PersistentVarSaveData {
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "progress save persistent var is missing name".to_string())?
                    .to_string(),
                value: entry
                    .get("value")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| "progress save persistent var is missing value".to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let current_level = match value.get("currentLevel") {
        Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| "progress save currentLevel must be a level id".to_string())?
                .to_string(),
        ),
        None => return Err("progress save is missing currentLevel".to_string()),
    };
    Ok(ProgressSaveData {
        version: u32::try_from(version).map_err(|_| "progress save version is too large")?,
        levels,
        current_level,
        persistent_vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cell_has_object(cell: &Value, object: &str) -> bool {
        cell["layers"]
            .as_array()
            .is_some_and(|layers| layers.iter().any(|layer| layer["object"] == object))
    }

    fn standalone_export(source: &str) -> Value {
        let document = puzzle_lang::parse_game_for_path(source, "export_test.puzzle").unwrap();
        serde_json::to_value(StandaloneRuntimeExport::new(document)).unwrap()
    }

    fn runtime_scene_fixture_source() -> &'static str {
        r#"
title = "Runtime Scene Fixture"

puzzle board {
layers {
@Floor
solid = Player Box Wall
}
input up
input down
input left
input right
rules {
input right [ Player | no solid ] -> [ | Player ]
input left [ no solid | Player ] -> [ Player | ]
input down [ Player ; no solid ] -> [ ; Player ]
input up [ no solid ; Player ] -> [ Player ; ]
}
}

levels microban of board {
legend {
. = empty
P = Player @Floor
B = Box @Floor
# = Wall
}
level "microban.1" {
#######
#P...B#
#.....#
#.....#
#.....#
#######
}
level "microban.2" {
#######
#.P..B#
#.....#
#.....#
#.....#
#######
}
}

scene title {
layout {
heading title
choice "New Game" -> goto playing("microban.1")
if has_progress_save {
choice "Continue" -> goto playing
}
}
}

scene playing {
layout {
puzzle board = board
}
rules {
step board
}
}
"#
    }

    #[test]
    fn standalone_session_from_export_requires_runtime_loaded_document() {
        let export = json!({
            "source": "title invalid\nlevels {\nlegend {\nP = Player\n}\nP\n}\n",
            "puzzlePath": "compiled_export.puzzle",
        });

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("export without runtimeLoadedDocument should be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("runtimeLoadedDocument"));
    }

    #[test]
    fn standalone_session_from_export_rejects_compiler_fields() {
        let source = r#"
title = export_runtime_bundle
puzzle default {
layers {
__legacy_layer_0 = Player
}
empty .
rules {
}
levels {
legend {
P = Player
}
level "start" {
P
}
}
}
"#;
        let mut export = standalone_export(source);
        export["source"] = json!("this is not puzzle syntax");
        export["puzzlePath"] = json!("bad_source_path.puzzle");

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("compiler fields must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn standalone_session_from_export_rejects_unsupported_runtime_version() {
        let mut export = standalone_export(runtime_scene_fixture_source());
        export["runtimeLoadedDocument"]["version"] = json!(
            STANDALONE_RUNTIME_EXPORT_VERSION
                .checked_add(1)
                .expect("test version remains in range")
        );

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("unsupported runtime version must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("unsupported runtimeLoadedDocument version"));
    }

    #[test]
    fn standalone_session_from_export_rejects_dangling_program_reference() {
        let mut export = standalone_export(runtime_scene_fixture_source());
        export["runtimeLoadedDocument"]["document"]["models"][0]["Puzzle2d"]["game"]["levels"][0]
            ["program"] = json!([{"Catalog": 999}, "Main"]);

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("dangling program reference must be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("invalid program reference"), "{error}");
    }

    #[test]
    fn standalone_session_resumes_rules_after_presentation_wait() {
        let source = r#"
title = export_wait_segments
puzzle default {
layers {
__legacy_layer_0 = A B C
}
empty .
rules {
[ A ] -> [ C ]
fall
}
routine fall {
[ C ] -> wait 100ms
[ C ] -> [ B ]
}
levels {
legend {
A = A
B = B
C = C
}
level "start" {
A
}
}
}
"#;
        let export = standalone_export(source);
        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string()).unwrap();

        let waiting: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("input should complete with a presentation wait"),
        )
        .unwrap();
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "A"));
        assert!(cell_has_object(&waiting["scene"]["cells"][0], "C"));
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "B"));
        assert_eq!(waiting["busy"], true);
        assert_eq!(waiting["presentationEvents"][0]["kind"], json!("wait"));
        assert_eq!(waiting["presentationEvents"][0]["milliseconds"], 100);
        assert_eq!(
            bridge.dispatch(SessionAction::Undo).unwrap_err(),
            "session action is unavailable while a turn is waiting"
        );

        let resumed: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Resume)
                .expect("wait completion should resume the same turn"),
        )
        .unwrap();
        assert!(!cell_has_object(&resumed["scene"]["cells"][0], "C"));
        assert!(cell_has_object(&resumed["scene"]["cells"][0], "B"));
        assert_eq!(resumed["busy"], false);
        assert!(
            bridge
                .dispatch(SessionAction::Resume)
                .unwrap_err()
                .contains("no turn is waiting")
        );

        let undone: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Undo)
                .expect("undo should return to pre-input state"),
        )
        .unwrap();
        assert!(cell_has_object(&undone["scene"]["cells"][0], "A"));
        assert!(!cell_has_object(&undone["scene"]["cells"][0], "B"));
        assert!(!cell_has_object(&undone["scene"]["cells"][0], "C"));
    }

    #[test]
    fn standalone_snapshot_serializes_mixed_presentation_events_in_authored_order() {
        let source = r#"
title = runtime_mixed_presentation
default_wait_time = 40ms
sounds {
sfx tick {
seed = tick01
type = hit
}
}
puzzle default {
render {
tween = true
tween_duration = 80ms
}
layers {
actor = Player
marker = Done
}
empty .
rules {
wait 10ms
message "ready"
input right [ Player | no Player ] -> [ | Player ] sfx tick
wait animation
[ Player no Done ] -> [ Player Done ]
}
levels {
legend {
. = empty
P = Player
}
level "first" {
P.
}
}
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_mixed_presentation.puzzle")
                .unwrap();

        let mut snapshot: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        let mut event_kinds = Vec::new();
        loop {
            let events = snapshot["presentationEvents"].as_array().unwrap();
            assert!(events.iter().all(|event| event["levelIndex"] == 0));
            event_kinds.extend(
                events
                    .iter()
                    .map(|event| event["kind"].as_str().unwrap().to_string()),
            );
            if snapshot["busy"] != true {
                break;
            }
            let modal = snapshot["surface"]["components"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|component| component["modal"] == true);
            snapshot = if let Some(component) = modal {
                event_kinds.push(format!(
                    "component:{}",
                    component["definition"].as_str().unwrap()
                ));
                serde_json::from_str(
                    &bridge
                        .dispatch(SessionAction::ComponentEvent {
                            instance: component["id"].as_str().unwrap().to_string(),
                            event: component["awaitEvent"].as_str().unwrap().to_string(),
                        })
                        .unwrap(),
                )
                .unwrap()
            } else {
                serde_json::from_str(&bridge.dispatch(SessionAction::Resume).unwrap()).unwrap()
            };
        }
        assert_eq!(
            event_kinds,
            vec![
                "wait",
                "component:standard.message",
                "animation_batch",
                "play_sfx",
                "wait"
            ]
        );
        assert!(cell_has_object(&snapshot["scene"]["cells"][1], "Done"));
    }

    #[cfg(not(feature = "editor-debug"))]
    #[test]
    fn player_session_contract_rejects_editor_debug_input() {
        let mut bridge = StandaloneSessionBridge::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let error = bridge
            .dispatch(SessionAction::DebugInput {
                name: "right".to_string(),
            })
            .unwrap_err();
        assert_eq!(error, "debug input is unavailable in the player runtime");
    }

    #[cfg(feature = "editor-debug")]
    #[test]
    fn standalone_session_debug_input_reports_rule_trace() {
        let source = r#"
title = "Debug Trace"

puzzle main {
  layers {
    actor = Player
  }

  rules {
    [ Player ] -> [ ]
  }
}

levels main of main {
  legend {
    . = empty
    P = Player
  }

  level "one"
  P
}
"#;
        let export = standalone_export(source);
        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string()).unwrap();

        let body: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::DebugInput {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();

        assert_eq!(body["snapshot"]["surface"]["focus"], "main");
        assert_eq!(body["debug"]["input"], "right");
        assert_eq!(body["debug"]["executions"].as_array().unwrap().len(), 1);
        assert_eq!(body["debug"]["executions"][0]["patch"][0]["kind"], "remove");
    }

    #[test]
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_scene_fixture.puzzle").unwrap();

        let initial: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(initial["surface"]["focus"], "board");
        assert_eq!(initial["title"], "Runtime Scene Fixture");
        assert_eq!(initial["solverState"]["kind"], "2d");
        assert!(initial["solverState"]["slots"].is_array());
        assert!(initial["solverState"].get("slotMarks").is_none());
        assert!(initial["solverState"].get("cellMarks").is_none());
        let title = initial["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["name"] == "title")
            .unwrap();
        assert_eq!(title["components"][0]["kind"], "text");
        assert_eq!(title["components"][0]["role"], "heading");

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let save: Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(
            save["currentLevel"],
            LevelId::new("board", "microban.1").record_key()
        );
    }

    #[test]
    fn snapshot_uses_scene_expression_contract_for_conditional_components() {
        let mut bridge = StandaloneSessionBridge::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();

        let snapshot: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        let title = snapshot["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["name"] == "title")
            .unwrap();
        let conditional = title["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|component| component["kind"] == "conditional")
            .unwrap();

        assert_eq!(
            conditional["condition"],
            json!({ "kind": "path", "path": "has_progress_save" })
        );
    }

    #[test]
    fn standalone_session_bridge_reports_no_scene_for_non_model_focus() {
        let source = r#"
title = runtime_focus
puzzle default {
layers {
actor = Player
}
empty .
rules {
down [ Player | no Player ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
.
}
}
}

scene playing {
layout {
puzzle board = default
}
rules {
step board
}
}

scene level_select {
layout {
text "Select"
}
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_focus.puzzle").unwrap();

        let select: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto level_select".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(select["surface"]["focus"], json!("level_select"));
        assert!(select["scene"].is_null());
        assert!(select["surface"]["components"][0]["scene"].is_null());

        let after_input: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "down".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after_input["surface"]["focus"], json!("level_select"));
        assert!(after_input["scene"].is_null());
        assert_eq!(after_input["presentationEvents"], json!([]));
    }

    #[test]
    fn standalone_session_bridge_starts_from_editor_state() {
        let source = r#"
title = editor_state_start

puzzle board {
layers {
actor = Player
}
empty .
input right
rules {
input right [ Player | no Player ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "authored" {
.P
}
level "editor_state" {
P.
}
}
}

scene playing {
layout {
puzzle board = board
}
rules {
step board
}
}
"#;
        let loaded = puzzle_lang::parse_game2d(source).unwrap();
        let editor_state = compiled_state_value(&loaded.levels[1].initial_state).to_string();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "editor_state_start.puzzle").unwrap();

        bridge
            .set_current_state_json(&editor_state, 0, false)
            .expect("editor state should start level 0");
        let started: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(started["levelIndex"], json!(0));
        assert!(cell_has_object(&started["scene"]["cells"][0], "Player"));

        bridge
            .apply_input_name("right")
            .expect("right should move from editor state");
        let moved: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert!(cell_has_object(&moved["scene"]["cells"][1], "Player"));

        bridge
            .apply_command_name("restart")
            .expect("restart should use editor start state");
        let restarted: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert!(cell_has_object(&restarted["scene"]["cells"][0], "Player"));

        assert!(
            bridge
                .set_current_state_json(&editor_state, 99, false)
                .unwrap_err()
                .contains("level index out of range: 99")
        );
    }

    #[test]
    fn spec_2d_new_game_uses_scene_input_and_scene_puzzle_state() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_scene_fixture.puzzle").unwrap();

        let title: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert!(
            title["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|input| { input["name"] == "up" && input["arrow"] == "ArrowUp" })
        );

        let playing = start_spec_2d_new_game(&mut bridge);
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);
        assert_eq!(playing["scenePuzzles"], json!(["board"]));
        let playing_object = playing.as_object().unwrap();
        assert!(playing_object.contains_key("surface"));
        assert!(!playing_object.contains_key("visibleScenes"));
        assert!(!playing_object.contains_key("sceneLayers"));
        assert!(!playing_object.contains_key("currentScene"));
        assert!(playing_object.contains_key("sceneState"));
        assert!(playing_object.contains_key("scenePuzzles"));
        assert!(!playing_object.contains_key("visibleScreens"));
        assert!(!playing_object.contains_key("screenState"));
        assert!(!playing_object.contains_key("screenPuzzles"));
        assert!(
            playing["scenePuzzleState"]["board"]
                .as_object()
                .is_some_and(|state| !state.contains_key("level"))
        );
        assert_eq!(playing["scene"]["cells"].as_array().unwrap().len(), 42);
        assert!(
            playing["scene"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|cell| cell["layers"].as_array().unwrap())
                .any(|layer| layer["object"] == "@Floor")
        );
    }

    #[test]
    fn transition_flickscreen_focuses_player_group() {
        let puzzlescript =
            include_str!("../../../crates/lang/tests/fixtures/puzzlescript/gallery/transition.ps");
        let source = puzzle_lang::translate_puzzlescript_to_canonical(puzzlescript).unwrap();
        let mut bridge =
            StandaloneSessionBridge::from_source(&source, "fixtures/transition.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing(0)".to_string(),
                })
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            playing["scene"]["screen"]["viewportSize"],
            json!({"kind": "size", "width": 13, "height": 13})
        );
        assert_eq!(playing["scene"]["screen"]["viewportFocus"], "player");
        assert!(
            playing["scene"]["screen"]["viewportFocusObjects"]
                .as_array()
                .unwrap()
                .iter()
                .all(|id| id.as_u64().is_some_and(|id| id > 0))
        );
        assert!(
            playing["scene"]["screen"]["viewportFocusObjects"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }

    #[test]
    fn spec_2d_direction_input_changes_state_after_new_game() {
        let source = runtime_scene_fixture_source();
        let mut changed_input = None;

        for input in ["up", "down", "left", "right"] {
            let mut bridge =
                StandaloneSessionBridge::from_source(source, "runtime_scene_fixture.puzzle")
                    .unwrap();
            let before = start_spec_2d_new_game(&mut bridge);
            let after: Value = serde_json::from_str(
                &bridge
                    .dispatch(SessionAction::Input {
                        name: input.to_string(),
                    })
                    .unwrap(),
            )
            .unwrap();

            if before["scene"] != after["scene"] || before["canUndo"] != after["canUndo"] {
                changed_input = Some(input);
                break;
            }
        }

        assert!(changed_input.is_some());
    }

    fn start_spec_2d_new_game(bridge: &mut StandaloneSessionBridge) -> Value {
        bridge
            .dispatch(SessionAction::Command {
                name: "clear_game_progress".to_string(),
            })
            .unwrap();
        serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing(\"microban.1\")".to_string(),
                })
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn standalone_session_bridge_emits_tween_on_first_input() {
        let source = r#"
title = "Runtime Tween Fixture"

puzzle mover {
  render {
    tween = true
    tween_duration = 300ms
  }
  layers {
    actor = Player
  }
  rules {
    input directions [ Player | no Player ] -> [ | Player ]
  }
}

levels default of mover {
  legend {
    . = empty
    P = Player
  }
  level "first" {
    P.
  }
}

scene playing {
  rules {
    step board
  }
  layout {
    puzzle board = mover
  }
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_tween_fixture.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Command {
                    name: "goto playing".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["surface"]["focus"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(moved["presentationEvents"][0]["kind"], "animation_batch");
        assert_eq!(
            moved["presentationEvents"][0]["animations"][0],
            json!({
                "kind": "move",
                "name": "tween",
                "occurrenceId": 1,
                "objectId": 1,
                "from": { "x": 0, "y": 0 },
                "to": { "x": 1, "y": 0 }
            }),
            "unexpected moved snapshot: {moved}"
        );
    }

    #[test]
    fn spatial_session_bridge_emits_dimensioned_tween_for_addressed_puzzle() {
        let source = r#"
title = "Spatial Tween Fixture"

puzzle mover {
  dimension = 3
  render {
    tween = true
    tween_duration = 120ms
  }
  layers {
    actor = Player
  }
  rules {
    input right [ Player | no Player ] -> [ | Player ]
  }
}

levels default of mover {
  legend {
    . = empty
    P = Player
  }
  level "first" {
    P.
  }
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "spatial_tween_fixture.puzzle3")
                .expect("compile spatial tween fixture");

        let moved: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "right".to_string(),
                })
                .expect("apply spatial model input"),
        )
        .unwrap();

        assert_eq!(moved["animation"]["tween"]["intervalMs"], 120);
        assert_eq!(
            moved["presentationEvents"],
            json!([{
                "scene": "mover",
                "puzzle": "mover",
                "levelIndex": 0,
                "kind": "animation_batch",
                "animations": [{
                    "kind": "move",
                    "name": "tween",
                    "occurrenceId": 1,
                    "objectId": 1,
                    "from": { "x": 0, "y": 0, "z": 0 },
                    "to": { "x": 1, "y": 0, "z": 0 }
                }]
            }])
        );
        assert_eq!(
            moved["scenePuzzleState"]["mover"]["cells"],
            json!([{
                "position": { "x": 1, "y": 0, "z": 0 },
                "objects": [{ "id": 1, "layer": 0 }]
            }])
        );
    }

    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_scene_fixture.puzzle").unwrap();
        let restored_level_key = LevelId::new("board", "microban.2").record_key();
        bridge
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [{"id": restored_level_key, "cleared": true}],
                    "currentLevel": restored_level_key,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();

        let snapshot: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Snapshot).unwrap()).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 1);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(
            snapshot["levels"][&restored_level_key]["progress"]["cleared"],
            true
        );
        let save: Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert!(save["levels"].as_array().is_some_and(|levels| {
            levels
                .iter()
                .any(|level| level["id"] == restored_level_key && level["cleared"] == true)
        }));
    }

    #[test]
    fn standalone_session_bridge_rejects_name_only_progress_entries() {
        let mut bridge = StandaloneSessionBridge::from_source(
            runtime_scene_fixture_source(),
            "runtime_scene_fixture.puzzle",
        )
        .unwrap();
        let error = bridge
            .restore_progress_save_json(
                r#"{"version":2,"levels":[{"name":"microban.2","cleared":true}],"currentLevel":null,"persistentVars":[]}"#,
            )
            .unwrap_err();
        assert!(
            error.contains("progress save level is missing id"),
            "{error}"
        );
    }

    #[test]
    fn standalone_session_runs_spatial_document_through_shared_scene_session() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3");
        let mut bridge = StandaloneSessionBridge::from_source(source, "spec_3d_full.puzzle3")
            .expect("spatial document should use the shared grid session");

        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["surface"]["focus"], "sokoban");
        assert_eq!(snapshot["solverState"]["kind"], "puzzle3d");
        assert!(snapshot["solverState"].get("slotMarks").is_none());
        assert!(snapshot["solverState"].get("cellMarks").is_none());
        assert!(snapshot["scenePuzzleState"]["sokoban"]["cells"].is_array());
        assert!(
            snapshot["scenePuzzleState"]["sokoban"]
                .get("render")
                .is_none()
        );
        assert!(
            snapshot["scenePuzzleState"]["sokoban"]
                .get("visuals")
                .is_none()
        );
        let second: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::GotoLevel { level: 1 })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(second["levelIndex"], 1);
        let first: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::PreviousLevel).unwrap()).unwrap();
        assert_eq!(first["levelIndex"], 0);
        bridge
            .dispatch(SessionAction::Input {
                name: "right".to_string(),
            })
            .expect("spatial model input should route through the shared session");
    }

    #[derive(Debug, PartialEq, Eq)]
    struct DimensionIndependentSessionTrace {
        slots: Vec<u16>,
        can_undo: bool,
        can_redo: bool,
        active_level_index: Option<usize>,
    }

    fn session_trace<const D: usize, Size: GridSize<D>>(
        loaded: &LoadedGridGame<D, Size>,
    ) -> Vec<DimensionIndependentSessionTrace> {
        fn capture<const D: usize, Size: GridSize<D>>(
            session: &GridGameSession<D, Size>,
        ) -> DimensionIndependentSessionTrace {
            DimensionIndependentSessionTrace {
                slots: session
                    .state()
                    .slots()
                    .iter()
                    .map(|object| object.0)
                    .collect(),
                can_undo: session.can_undo(),
                can_redo: session.can_redo(),
                active_level_index: session.active_level_index(),
            }
        }

        let mut session = GridGameSession::new(loaded);
        let mut trace = vec![capture(&session)];
        session.apply_input(loaded, InputId(0)).unwrap();
        trace.push(capture(&session));
        session.undo(loaded);
        trace.push(capture(&session));
        session.redo(loaded);
        trace.push(capture(&session));
        session.restart_level(loaded).unwrap();
        trace.push(capture(&session));
        trace
    }

    #[test]
    fn shared_session_actions_have_dimension_independent_semantics() {
        let model = |dimension: &str, path: &str| {
            puzzle_lang::parse_game_for_path(
                &format!(
                    r#"
title = Session parity
puzzle board {{
  {dimension}
  layers {{ actor = A B C D E }}
  rules {{ [ C ] -> [ D ] }}
}}
levels default of board {{
  legend {{ A = A }}
  level "one" {{
    on_level_start {{ [ A ] -> [ B ] }}
    rules before {{ [ B ] -> [ C ] }}
    A
    rules {{ [ D ] -> [ E ] }}
  }}
}}
"#
                ),
                path,
            )
            .unwrap()
        };
        let flat = model("", "session_parity.puzzle");
        let spatial = model("dimension = 3", "session_parity.puzzle3");
        let Some(LoadedDocumentModel::Puzzle2d { game: flat, .. }) = flat.single_model() else {
            panic!("expected 2D model");
        };
        let Some(LoadedDocumentModel::Puzzle3d { game: spatial, .. }) = spatial.single_model()
        else {
            panic!("expected 3D model");
        };

        assert_eq!(flat.levels[0].program.references().len(), 3);
        assert!(matches!(
            flat.levels[0].program.references(),
            [
                puzzle_core::GridProgramRef::Catalog(_),
                puzzle_core::GridProgramRef::Main,
                puzzle_core::GridProgramRef::Catalog(_)
            ]
        ));
        assert_eq!(flat.levels[0].program, spatial.levels[0].program);
        assert_eq!(
            flat.levels[0].level_start_program,
            spatial.levels[0].level_start_program
        );
        assert_eq!(
            flat.levels[0].level_clear_program,
            spatial.levels[0].level_clear_program
        );
        assert_eq!(
            flat.program_catalog.programs().len(),
            spatial.program_catalog.programs().len()
        );
        assert_eq!(session_trace(flat), session_trace(spatial));
    }
}
