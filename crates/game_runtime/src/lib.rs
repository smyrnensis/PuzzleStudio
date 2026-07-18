use puzzle_core::{
    CompiledGame, Coord3, GridCompiledGame, GridLevel, GridRuleFiring, GridSize, GridState,
    GridTransitionOutcome, InputId, MarkValueMatch as CoreMarkValueMatch, ObjectId, PatchOp,
    RuleId, Size2, Size3, State as PuzzleState, TransitionCommand,
};
#[cfg(feature = "editor-debug")]
use puzzle_core::{GridCoord, GridPatch, MarkId, VariableId, VariableUpdateOp};
use puzzle_lang::{
    ArrowKey, KeyTrigger, Level, LoadedDocument, LoadedDocumentModel, LoadedGame, LoadedGridGame,
    ResourceSelection, SceneAlignDef, SceneBinaryOp, SceneComponent, SceneDef,
    SceneDistributionDef, SceneEffect, SceneEffectParam, SceneExpr, SceneLayoutDef, SceneLevelKey,
    ScenePuzzleInitializer, SceneSpaceDef, SceneStateLifetime, SceneTextContent, SceneTextRoleDef,
    SceneTransitionTrigger, SceneValue, ThemeDef, ViewportModeDef, ViewportProjectionDef,
    ViewportSizeDef,
};
#[cfg(feature = "editor-debug")]
use puzzle_play::GridTransitionTrace;
use puzzle_play::{
    GameSession, GridGameSession, LevelProgressSaveData, PersistentVarSaveData,
    PresentationEventKind, ProgressSaveData, presentation_events_contract,
    runtime_effects_for_outcome, runtime_sounds_def,
};
use puzzle_runtime_contract::{
    CameraEffect, GridRuntimeModel, RuntimeAnimationEvent, RuntimeChangedCell, RuntimeContract,
    RuntimeCoord, RuntimeGridSize, RuntimeMarkValueMatch, RuntimePatchOp, RuntimeRuleFiring,
    RuntimeStateSnapshot, RuntimeStateSnapshot3d, RuntimeTransitionCommand, SessionAction,
    RuntimeTransitionCurrentOutcome, RuntimeTransitionProgramOutcome,
};
use serde_json::{Value, json};

pub struct StandaloneSessionBridge {
    model: Box<dyn StandaloneSessionModel>,
    has_progress_save: bool,
}

trait StandaloneSessionModel {
    fn snapshot_value(&mut self, has_progress_save: bool) -> Value;
    fn apply_input_name(&mut self, input_name: &str) -> Result<(), String>;
    #[cfg(feature = "editor-debug")]
    fn apply_debug_input_name_json(&mut self, input_name: &str) -> Result<String, String>;
    fn apply_command_name(&mut self, command_name: &str) -> Result<(), String>;
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
    fn decode_state(
        &self,
        game: &GridCompiledGame<D>,
        value: &Value,
    ) -> Result<GridState<D, Size>, String>;

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        session: &GridGameSession<D, Size>,
    ) -> ProjectedGridSnapshot;
}

struct ProjectedGridSnapshot {
    scene: Value,
    scene_puzzle_state: Value,
    scene_layers: Vec<Value>,
}

#[derive(Default)]
struct CanvasProjection;

struct SpatialProjection {
    fixture: Value,
}

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
        let export: Value = serde_json::from_str(export_json).map_err(|error| error.to_string())?;
        let runtime_bundle = export
            .get("runtimeLoadedDocument")
            .ok_or_else(|| "standalone export is missing runtimeLoadedDocument".to_string())?;
        let version = runtime_bundle
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| "runtimeLoadedDocument is missing version".to_string())?;
        if version != 1 {
            return Err(format!(
                "unsupported runtimeLoadedDocument version: {version}"
            ));
        }
        let document_value = runtime_bundle
            .get("document")
            .ok_or_else(|| "runtimeLoadedDocument is missing document".to_string())?;
        let document: LoadedDocument = serde_json::from_value(document_value.clone())
            .map_err(|error| format!("invalid runtimeLoadedDocument document: {error}"))?;
        Ok(Self {
            model: standalone_session_model(document)?,
            has_progress_save: false,
        })
    }

    pub fn snapshot_json(&mut self) -> String {
        serde_json::to_string(&self.model.snapshot_value(self.has_progress_save))
            .expect("snapshot JSON should serialize")
    }

    pub fn dispatch(&mut self, action: SessionAction) -> Result<String, String> {
        match action {
            SessionAction::Snapshot => Ok(self.snapshot_json()),
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
            SessionAction::DebugInput { name } => {
                self.apply_debug_input_name_json(&name)
            }
            #[cfg(not(feature = "editor-debug"))]
            SessionAction::DebugInput { .. } => {
                Err("debug input is unavailable in the player runtime".to_string())
            }
            SessionAction::Command { name } => {
                self.apply_command_name(&name)?;
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
    document: LoadedDocument,
) -> Result<Box<dyn StandaloneSessionModel>, String> {
    match document.single_model() {
        Some(LoadedDocumentModel::Puzzle2d { game, .. }) => {
            let loaded = game.clone();
            Ok(Box::new(GridSessionRuntime {
                session: GridGameSession::new(&loaded),
                loaded,
                projection: CanvasProjection,
            }))
        }
        Some(LoadedDocumentModel::Puzzle3d {
            game, presentation, ..
        }) => {
            let fixture = puzzle_lang::export_loaded_document_visual_fixture_json(&document)
                .map_err(|error| format!("spatial runtime fixture failed: {error:?}"))?;
            let fixture = serde_json::from_str(&fixture)
                .map_err(|error| format!("spatial runtime fixture is invalid JSON: {error}"))?;
            let loaded = game.clone();
            let _ = presentation;
            Ok(Box::new(GridSessionRuntime {
                session: GridGameSession::new(&loaded),
                loaded,
                projection: SpatialProjection { fixture },
            }))
        }
        None => Err(
            "a document with multiple puzzle worlds requires model-addressed session routing"
                .to_string(),
        ),
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
        let busy = presentation_events
            .iter()
            .any(|event| matches!(event.kind, PresentationEventKind::Wait(_)));
        let current_scene = self.session.focused_scene();
        let projected = self.projection.snapshot_grid(&self.loaded, &self.session);
        let scene_state = scene_state_value(self.session.scene_state());
        let scene_puzzles = scene_puzzles_value(self.session.scene_state());
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
            "presentationEvents": presentation_events_contract(&self.loaded, &presentation_events),
            "level": level_context_value(&self.loaded, &self.session),
            "levelIndex": self.session.active_level_index(),
            "levelCount": self.loaded.levels.len(),
            "scene": projected.scene,
            "currentScene": current_scene,
            "focusedScreen": current_scene,
            "focusedScene": current_scene,
            "acceptsModelInput": self.session.accepts_model_input(&self.loaded),
            "visibleScenes": self.session.visible_scenes(),
            "gameState": scene_values_value(self.session.session_values()),
            "sceneState": scene_state,
            "scenePuzzles": scene_puzzles,
            "scenePuzzleState": projected.scene_puzzle_state,
            "sceneLayers": projected.scene_layers,
            "selectedLevelIndex": self.session.selected_level_index(),
            "busy": busy,
            "canUndo": self.session.can_undo(),
            "canRedo": self.session.can_redo(),
            "inputs": inputs_value(&self.loaded),
            "levels": levels_value(&self.loaded, self.session.cleared_levels()),
            "scenes": scenes_value(&self.loaded),
            "screens": scenes_value(&self.loaded),
        })
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
        let value: Value = serde_json::from_str(state_json).map_err(|error| error.to_string())?;
        let state = self.projection.decode_state(&self.loaded.game, &value)?;
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

fn decode_state_value(game: &CompiledGame, value: &Value) -> Result<PuzzleState, String> {
    let width = u16_field(value, "width")?;
    let height = u16_field(value, "height")?;
    let layer_count = u16_field(value, "layerCount")?;
    let variables = value
        .get("variables")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_i64()
                        .ok_or_else(|| "variable must be an integer".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut state = PuzzleState::empty_with_variables(
        width,
        height,
        layer_count,
        game.object_count(),
        variables,
    )
    .map_err(|error| format!("{error:?}"))?;
    for (index, item) in array_field(value, "slots")?.iter().enumerate() {
        let object = ObjectId(u16_value(item, "slot")?);
        if object.is_empty() {
            continue;
        }
        let cell = index / usize::from(layer_count);
        let x = u16::try_from(cell % usize::from(width)).map_err(|_| "x out of range")?;
        let y = u16::try_from(cell / usize::from(width)).map_err(|_| "y out of range")?;
        state
            .place_object(game, x, y, object)
            .map_err(|error| format!("{error:?}"))?;
    }
    if let Some(rules) = value.get("levelFiredRules").and_then(Value::as_array) {
        for rule in rules {
            state.mark_level_rule_fired(RuleId(u16_value(rule, "levelFiredRules")?));
        }
    }
    Ok(state)
}

impl GridSessionProjection<2, Size2> for CanvasProjection {
    fn decode_state(
        &self,
        game: &GridCompiledGame<2>,
        value: &Value,
    ) -> Result<GridState<2, Size2>, String> {
        decode_state_value(game, value)
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
}

impl GridSessionProjection<3, Size3> for SpatialProjection {
    fn decode_state(
        &self,
        game: &GridCompiledGame<3>,
        value: &Value,
    ) -> Result<GridState<3, Size3>, String> {
        state3_from_json(game, &value.to_string())
    }

    fn snapshot_grid(
        &self,
        loaded: &LoadedGridGame<3, Size3>,
        session: &GridGameSession<3, Size3>,
    ) -> ProjectedGridSnapshot {
        let scene_puzzle_state = spatial_scene_puzzle_state_value(&self.fixture, loaded, session);
        let focused_scene = session.focused_scene();
        let scene = session
            .scene_state()
            .and_then(|state| state.puzzles.values().next())
            .map(|world| spatial_world_value(&self.fixture, loaded, focused_scene, world))
            .unwrap_or(Value::Null);
        let scene_layers = session
            .visible_scenes()
            .iter()
            .map(|name| {
                let state = session.scene_state_for(name);
                let projected = state
                    .and_then(|state| state.puzzles.values().next())
                    .map(|world| spatial_world_value(&self.fixture, loaded, name, world));
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
}

fn spatial_scene_puzzle_state_value(
    fixture: &Value,
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
                spatial_world_value(fixture, loaded, session.focused_scene(), world),
            )
        })
        .collect();
    Value::Object(entries)
}

fn spatial_world_value(
    fixture: &Value,
    loaded: &LoadedGridGame<3, Size3>,
    scene_name: &str,
    world: &puzzle_play::GridWorldInstanceState<3, Size3>,
) -> Value {
    let mut value = fixture.clone();
    let level_index = world.active_level_index.unwrap_or(0);
    let level = loaded.levels.get(level_index);
    let size = world.state.size;
    let cells = spatial_state_cells_value(fixture, &world.state);
    let Some(object) = value.as_object_mut() else {
        return Value::Null;
    };
    object.insert(
        "currentScene".to_string(),
        Value::String(scene_name.to_string()),
    );
    object.insert("levelIndex".to_string(), json!(level_index));
    object.insert("levelCount".to_string(), json!(loaded.levels.len()));
    object.insert(
        "levelName".to_string(),
        json!(level.map(|level| &level.name)),
    );
    object.insert(
        "size".to_string(),
        json!({
            "width": size.width,
            "depth": size.depth,
            "height": size.height,
        }),
    );
    object.insert("cells".to_string(), cells);
    object.insert(
        "completed".to_string(),
        json!(loaded.is_goal_complete(&world.state)),
    );
    object.insert(
        "hasNextLevel".to_string(),
        json!(level_index + 1 < loaded.levels.len()),
    );
    object.insert("hasPreviousLevel".to_string(), json!(level_index > 0));
    value
}

fn spatial_state_cells_value(fixture: &Value, state: &GridState<3, Size3>) -> Value {
    let objects = fixture.get("objects").and_then(Value::as_object);
    let mut by_id = std::collections::HashMap::new();
    for object in objects.into_iter().flat_map(|objects| objects.values()) {
        if let Some(id) = object.get("id").and_then(Value::as_u64) {
            by_id.insert(id as u16, object.clone());
        }
    }
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
                        (!object.is_empty()).then(|| {
                            by_id
                                .get(&object.0)
                                .cloned()
                                .unwrap_or_else(|| json!({"id": object.0, "layer": layer}))
                        })
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
    json!({
        "width": state.width,
        "height": state.height,
        "layerCount": state.layer_count,
        "slots": state.slots().iter().map(|object| object.0).collect::<Vec<_>>(),
        "mark": [],
        "variables": state.visible_variables(),
        "levelFiredRules": state.level_fired_rules().iter().map(|rule| rule.0).collect::<Vec<_>>(),
    })
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn u16_field(value: &Value, key: &str) -> Result<u16, String> {
    u16_value(
        value
            .get(key)
            .ok_or_else(|| format!("missing field: {key}"))?,
        key,
    )
}

fn u16_value(value: &Value, name: &str) -> Result<u16, String> {
    let raw = value
        .as_u64()
        .ok_or_else(|| format!("{name} must be an unsigned integer"))?;
    u16::try_from(raw).map_err(|_| format!("{name} out of range"))
}

function focused_scene_value(loaded: &LoadedGame, session: &GameSession) -> Value {
    if let Some((state, level)) = scene_puzzle_state(loaded, session, session.focused_scene()) {
        return scene_value_for_state(
            loaded,
            state,
            level,
            scene_resources(loaded, session.focused_scene()),
        );
    }
    if !loaded.scenes.is_empty() {
        return Value::Null;
    }
    scene_value_for_state(
        loaded,
        session.state(),
        Some(session.current_level(loaded)),
        scene_resources(loaded, session.focused_scene()),
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
                        "sprite": name,
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
        .visible_scenes()
        .iter()
        .map(|name| {
            let state = session.scene_state_for(name);
            let scene = scene_puzzle_state(loaded, session, name).map(|(puzzle_state, level)| {
                scene_value_for_state(loaded, puzzle_state, level, scene_resources(loaded, name))
            });
            json!({
                "name": name,
                "focused": name == session.focused_scene(),
                "scene": scene,
                "sceneState": scene_state_value(state),
                "scenePuzzles": scene_puzzles_value(state),
            })
        })
        .collect()
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
        let level_index = puzzle.active_level_index;
        let level = level_index.and_then(|index| loaded.levels.get(index));
        let mut entry = match scene_value_for_state(
            loaded,
            &puzzle.state,
            level,
            scene_resources(loaded, session.focused_scene()),
        ) {
            Value::Object(object) => object,
            _ => serde_json::Map::new(),
        };
        entry.insert(
            "level".to_string(),
            level_index
                .map(|level_index| {
                    level_ref_value(loaded, session, session.focused_scene(), level_index)
                })
                .unwrap_or(Value::Null),
        );
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
        "spritesMode": resource_selection_mode(&scene.resources.sprites),
        "sprites": resource_selection_names(&scene.resources.sprites),
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
            "condition": conditional.condition,
            "children": scene_component_list_value(&conditional.children),
            "elseChildren": scene_component_list_value(&conditional.else_children),
        }),
        SceneComponent::For(for_view) => json!({
            "kind": "for",
            "binding": for_view.binding,
            "source": for_view.source.as_str(),
            "children": scene_component_list_value(&for_view.children),
        }),
        SceneComponent::LevelMenu(menu) => json!({
            "kind": "level_menu",
            "showIndex": menu.show_index,
            "showCleared": menu.show_cleared,
            "columns": menu.columns,
            "wrap": menu.wrap,
            "source": menu.source,
            "action": menu.action.as_ref().map(scene_effect_value),
            "buttons": menu.buttons.iter().map(|button| json!({
                "label": scene_expr_value(&button.label),
                "effect": scene_effect_value(&button.effect),
            })).collect::<Vec<_>>(),
        }),
    }
}

fn scene_component_list_value(components: &[SceneComponent]) -> Vec<Value> {
    components.iter().map(scene_component_value).collect()
}

fn scene_effect_value(effect: &SceneEffect) -> Value {
    match effect {
        SceneEffect::Input(input) => json!({ "kind": "input", "name": input }),
        SceneEffect::ComponentEffect(name) => json!({ "kind": "component_effect", "name": name }),
        SceneEffect::RoutineCall(name) => json!({ "kind": "routine_call", "name": name }),
        SceneEffect::Message { text } => {
            json!({ "kind": "message", "text": scene_expr_value(text) })
        }
        SceneEffect::Wait { milliseconds } => {
            json!({ "kind": "wait", "milliseconds": milliseconds.unwrap_or(200) })
        }
        SceneEffect::Conditional { condition, effect } => json!({
            "kind": "conditional",
            "condition": condition,
            "effect": scene_effect_value(effect),
        }),
        SceneEffect::PlaySfx { name } => json!({ "kind": "play_sfx", "name": name }),
        SceneEffect::PlayMusic { name } => json!({ "kind": "play_music", "name": name }),
        SceneEffect::PauseMusic { name } => json!({ "kind": "pause_music", "name": name }),
        SceneEffect::ResumeMusic { name } => json!({ "kind": "resume_music", "name": name }),
        SceneEffect::StopMusic { name } => json!({ "kind": "stop_music", "name": name }),
        SceneEffect::Goto { scene, params } => scene_target_effect_value("goto", scene, params),
        SceneEffect::Enter { scene, params } => scene_target_effect_value("enter", scene, params),
        SceneEffect::Back => json!({ "kind": "back" }),
        SceneEffect::Create { scene } => scene_target_effect_value("create", scene, &[]),
        SceneEffect::Reset { scene } => scene_target_effect_value("reset", scene, &[]),
        SceneEffect::Delete { scene } => scene_target_effect_value("delete", scene, &[]),
        SceneEffect::Show { scene } => scene_target_effect_value("show", scene, &[]),
        SceneEffect::Hide { scene } => scene_target_effect_value("hide", scene, &[]),
        SceneEffect::Toggle { scene } => scene_target_effect_value("toggle", scene, &[]),
        SceneEffect::Focus { scene } => scene_target_effect_value("focus", scene, &[]),
        SceneEffect::PuzzleNextLevel { target } => {
            json!({ "kind": "puzzle_next_level", "target": target })
        }
        SceneEffect::PuzzlePreviousLevel { target } => {
            json!({ "kind": "puzzle_previous_level", "target": target })
        }
        SceneEffect::GotoLevel { target, level } => json!({
            "kind": "puzzle_goto_level",
            "target": target,
            "level": scene_expr_value(level),
        }),
        SceneEffect::ResetPuzzle { target } => json!({ "kind": "puzzle_reset", "target": target }),
        SceneEffect::LoadPuzzle { target, source } => json!({
            "kind": "puzzle_load",
            "target": target,
            "source": source,
        }),
        SceneEffect::Apply { rule, args, target } => {
            let mut value = serde_json::Map::new();
            value.insert("kind".to_string(), Value::String("apply".to_string()));
            value.insert("rule".to_string(), Value::String(rule.clone()));
            value.insert(
                "args".to_string(),
                Value::Array(args.iter().map(scene_expr_value).collect()),
            );
            if let Some(target) = target {
                value.insert("target".to_string(), Value::String(target.clone()));
            }
            Value::Object(value)
        }
        SceneEffect::Copy { source, target } => json!({
            "kind": "copy",
            "source": source,
            "target": target,
        }),
        SceneEffect::SetVariable { name, value } => json!({
            "kind": "set_variable",
            "name": name,
            "value": scene_expr_value(value),
        }),
        SceneEffect::ClearUndoHistory => json!({ "kind": "clear_undo_history" }),
        SceneEffect::ClearGameProgress => json!({ "kind": "clear_game_progress" }),
        SceneEffect::SetCurrentLevel { level } => json!({
            "kind": "set_current_level",
            "level": scene_expr_value(level),
        }),
        SceneEffect::ClearCurrentLevel => json!({ "kind": "clear_current_level" }),
        SceneEffect::SetLevelCleared { level, cleared } => {
            let mut value = serde_json::Map::new();
            value.insert(
                "kind".to_string(),
                Value::String("set_level_cleared".to_string()),
            );
            value.insert("cleared".to_string(), Value::Bool(*cleared));
            if let Some(level) = level {
                value.insert("level".to_string(), scene_expr_value(level));
            }
            Value::Object(value)
        }
        SceneEffect::ResetPersistentVars => json!({ "kind": "reset_persistent_vars" }),
        SceneEffect::Sequence { effects } => json!({
            "kind": "sequence",
            "effects": effects.iter().map(scene_effect_value).collect::<Vec<_>>(),
        }),
    }
}

fn scene_target_effect_value(kind: &str, scene: &str, params: &[SceneEffectParam]) -> Value {
    json!({
        "kind": kind,
        "screen": scene,
        "scene": scene,
        "params": params.iter().map(scene_effect_param_value).collect::<Vec<_>>(),
    })
}

fn scene_effect_param_value(param: &SceneEffectParam) -> Value {
    match param {
        SceneEffectParam::Level(value) => json!({
            "kind": "level",
            "value": scene_expr_value(value),
        }),
        SceneEffectParam::Named { name, value } => json!({
            "kind": "named",
            "name": name,
            "value": scene_expr_value(value),
        }),
    }
}

fn scene_expr_value(expr: &SceneExpr) -> Value {
    match expr {
        SceneExpr::Bool(value) => json!({ "kind": "bool", "value": value }),
        SceneExpr::Int(value) => json!({ "kind": "int", "value": value }),
        SceneExpr::Text(value) => json!({ "kind": "text", "value": value }),
        SceneExpr::Path(path) => json!({ "kind": "path", "path": path.join(".") }),
        SceneExpr::LevelSelector {
            collection,
            key,
            property,
        } => json!({
            "kind": "level_selector",
            "collection": collection,
            "key": scene_level_key_value(key),
            "property": property,
        }),
        SceneExpr::Call { name, args } => json!({
            "kind": "call",
            "name": name,
            "args": args.iter().map(scene_expr_value).collect::<Vec<_>>(),
        }),
        SceneExpr::Binary { op, left, right } => json!({
            "kind": "binary",
            "op": scene_binary_op_value(*op),
            "left": scene_expr_value(left),
            "right": scene_expr_value(right),
        }),
        SceneExpr::If {
            condition,
            then_branch,
            else_branch,
        } => json!({
            "kind": "if",
            "condition": scene_expr_value(condition),
            "then": scene_expr_value(then_branch),
            "else": scene_expr_value(else_branch),
        }),
    }
}

fn scene_level_key_value(key: &SceneLevelKey) -> Value {
    match key {
        SceneLevelKey::Index(value) => json!({ "kind": "index", "value": value }),
        SceneLevelKey::Id(value) => json!({ "kind": "id", "value": value }),
    }
}

fn scene_binary_op_value(op: SceneBinaryOp) -> &'static str {
    match op {
        SceneBinaryOp::And => "and",
        SceneBinaryOp::Eq => "eq",
        SceneBinaryOp::In => "in",
        SceneBinaryOp::NotEq => "neq",
    }
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
            SceneComponent::For(for_view) => {
                if let Some(name) = first_puzzle_component(&for_view.children) {
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

fn level_context_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    session: &GridGameSession<D, Size>,
) -> Value {
    let level = session.current_level(loaded);
    json!({
        "index": session.level_index(),
        "name": level.name,
        "pack": level.pack,
        "puzzle": level.puzzle,
        "cleared": session.cleared_levels().get(session.level_index()).copied().unwrap_or(false),
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

fn levels_value<const D: usize, Size: GridSize<D>>(
    loaded: &LoadedGridGame<D, Size>,
    cleared_levels: &[bool],
) -> Vec<Value> {
    loaded
        .levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            json!({
                "index": index,
                "name": level.name,
                "pack": level.pack,
                "puzzle": level.puzzle,
                "cleared": cleared_levels.get(index).copied().unwrap_or(false),
            })
        })
        .collect()
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

fn level_ref_value(
    loaded: &LoadedGame,
    session: &GameSession,
    scene_name: &str,
    level_index: usize,
) -> Value {
    let level = loaded.levels.get(level_index);
    json!({
        "kind": "level",
        "index": level_index,
        "num": level_index + 1,
        "number": level_index + 1,
        "name": level.map(|level| level.name.clone()),
        "label": level.map(|level| level.name.clone()),
        "title": level.map(|level| level.name.clone()),
        "puzzle": level.map(|level| level.puzzle.clone()),
        "pack": level.and_then(|level| level.pack.clone()),
        "cleared": session.cleared_levels().get(level_index).copied().unwrap_or(false),
        "solved": session.cleared_levels().get(level_index).copied().unwrap_or(false),
        "has_next": level_has_next_in_scene(loaded, scene_name, level_index),
        "last": !level_has_next_in_scene(loaded, scene_name, level_index),
    })
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
        "sprites": match &resources.sprites {
            ResourceSelection::All => json!({"mode": "all", "names": []}),
            ResourceSelection::Named(names) => json!({"mode": "named", "names": names}),
        },
    })
}

fn level_has_next_in_scene(loaded: &LoadedGame, scene_name: &str, level_index: usize) -> bool {
    let indices = scene_level_indices(loaded, scene_name);
    indices
        .iter()
        .position(|index| *index == level_index)
        .is_some_and(|position| position + 1 < indices.len())
}

fn scene_level_indices(loaded: &LoadedGame, scene_name: &str) -> Vec<usize> {
    let Some(scene) = loaded.scenes.iter().find(|scene| scene.name == scene_name) else {
        return (0..loaded.levels.len()).collect();
    };
    match &scene.resources.levels {
        ResourceSelection::All => (0..loaded.levels.len()).collect(),
        ResourceSelection::Named(names) => loaded
            .levels
            .iter()
            .enumerate()
            .filter_map(|(index, level)| {
                names
                    .iter()
                    .any(|name| level_resource_matches(name, &level.name))
                    .then_some(index)
            })
            .collect(),
    }
}

fn level_resource_matches(resource: &str, level_name: &str) -> bool {
    level_name == resource
        || level_name
            .strip_prefix(resource)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn progress_save_data_value(save: &ProgressSaveData) -> Value {
    json!({
        "version": save.version,
        "levels": save.levels.iter().map(|level| {
            json!({"name": level.name, "cleared": level.cleared})
        }).collect::<Vec<_>>(),
        "currentLevel": save.current_level,
        "persistentVars": save.persistent_vars.iter().map(|var| {
            json!({"name": var.name, "value": var.value})
        }).collect::<Vec<_>>(),
    })
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
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "progress save level is missing name".to_string())?
                    .to_string(),
                cleared: entry
                    .get("cleared")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let persistent_vars = value
        .get("persistentVars")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    Ok(PersistentVarSaveData {
                        name: entry
                            .get("name")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                "progress save persistent var is missing name".to_string()
                            })?
                            .to_string(),
                        value: entry.get("value").and_then(Value::as_i64).unwrap_or(0),
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(ProgressSaveData {
        version: u32::try_from(version).map_err(|_| "progress save version is too large")?,
        levels,
        current_level: value
            .get("currentLevel")
            .and_then(Value::as_str)
            .map(str::to_string),
        persistent_vars,
    })
}

fn state3_from_json(
    game: &GridCompiledGame<3>,
    state_json: &str,
) -> Result<GridState<3, Size3>, String> {
    serde_json::from_str::<RuntimeStateSnapshot3d>(state_json)
        .map_err(|error| error.to_string())?
        .into_state(game)
}

fn state3_value(state: &GridState<3, Size3>) -> Value {
    serde_json::to_value(RuntimeStateSnapshot3d::from_state(state))
        .expect("runtime state snapshot serializes")
}

fn commands3_contract(commands: &[TransitionCommand]) -> Vec<RuntimeTransitionCommand> {
    commands
        .iter()
        .map(|command| match command {
            TransitionCommand::Win => RuntimeTransitionCommand::Win,
            TransitionCommand::Restart => RuntimeTransitionCommand::Restart,
            TransitionCommand::NextLevel => RuntimeTransitionCommand::NextLevel,
            TransitionCommand::Again => RuntimeTransitionCommand::Again,
            TransitionCommand::Checkpoint => RuntimeTransitionCommand::Checkpoint,
            TransitionCommand::ClearCheckpoint => RuntimeTransitionCommand::ClearCheckpoint,
        })
        .collect()
}

fn state3_contract(state: &GridState<3, Size3>) -> RuntimeStateSnapshot {
    RuntimeStateSnapshot::ThreeD(RuntimeStateSnapshot3d::from_state(state))
}

fn state3_cells_value(state: &GridState<3, Size3>, before: Option<&GridState<3, Size3>>) -> Value {
    let mut cells = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let cell = ((usize::from(z) * usize::from(state.size.depth)) + usize::from(y))
                    * usize::from(state.size.width)
                    + usize::from(x);
                if before.is_some_and(|before| state3_cell_slots_equal(before, state, cell)) {
                    continue;
                }
                let mut objects = Vec::new();
                for layer in 0..state.layer_count {
                    let slot = (cell * usize::from(state.layer_count)) + usize::from(layer);
                    let object = state.slots()[slot];
                    if !object.is_empty() {
                        objects.push(object.0);
                    }
                }
                if before.is_none() && objects.is_empty() {
                    continue;
                }
                cells.push(json!({
                    "position": {"x": x, "y": y, "z": z},
                    "objects": objects,
                }));
            }
        }
    }
    Value::Array(cells)
}

fn changed_cells3_contract(
    state: &GridState<3, Size3>,
    before: Option<&GridState<3, Size3>>,
) -> Vec<RuntimeChangedCell> {
    let mut cells = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let cell = ((usize::from(z) * usize::from(state.size.depth)) + usize::from(y))
                    * usize::from(state.size.width)
                    + usize::from(x);
                if before.is_some_and(|before| state3_cell_slots_equal(before, state, cell)) {
                    continue;
                }
                let mut objects = Vec::new();
                for layer in 0..state.layer_count {
                    let slot = (cell * usize::from(state.layer_count)) + usize::from(layer);
                    let object = state.slots()[slot];
                    if !object.is_empty() {
                        objects.push(object.0);
                    }
                }
                if before.is_none() && objects.is_empty() {
                    continue;
                }
                cells.push(RuntimeChangedCell {
                    position: RuntimeCoord { x, y, z: Some(z) },
                    objects,
                });
            }
        }
    }
    cells
}

fn firings3_contract(firings: &[GridRuleFiring<3>]) -> Vec<RuntimeRuleFiring> {
    firings
        .iter()
        .map(|firing| RuntimeRuleFiring {
            rule_id: firing.rule.0,
            patch: firing.patch.ops().iter().map(patch_op3_contract).collect(),
            progressed: firing.progressed,
            observable: firing.observable,
        })
        .collect()
}

fn patch_op3_contract(op: &PatchOp<3>) -> RuntimePatchOp {
    match *op {
        PatchOp::<3>::Add { position, object } => RuntimePatchOp::Add {
            position: runtime_coord3(position.into()),
            object_id: object.0,
        },
        PatchOp::<3>::Remove { position, object } => RuntimePatchOp::Remove {
            position: runtime_coord3(position.into()),
            object_id: object.0,
        },
        PatchOp::<3>::Move { from, to, object } => RuntimePatchOp::Move {
            from: runtime_coord3(from.into()),
            to: runtime_coord3(to.into()),
            object_id: object.0,
        },
        PatchOp::<3>::Replace {
            position,
            remove,
            add,
        } => RuntimePatchOp::Replace {
            position: runtime_coord3(position.into()),
            remove: remove.0,
            add: add.0,
        },
        PatchOp::<3>::UpdateVariable { variable, .. } => RuntimePatchOp::UpdateVariable {
            variable: variable.0,
        },
        PatchOp::<3>::SetMark {
            position,
            object,
            mark,
            ..
        } => RuntimePatchOp::SetMark {
            position: runtime_coord3(position.into()),
            object_id: object.0,
            mark: mark.0,
        },
        PatchOp::<3>::RemoveMark {
            position,
            object,
            mark,
            match_value,
            ..
        } => RuntimePatchOp::RemoveMark {
            position: runtime_coord3(position.into()),
            object_id: object.0,
            mark: mark.0,
            match_value: runtime_mark_value_match3(match_value),
        },
    }
}

fn runtime_coord3(position: Coord3) -> RuntimeCoord {
    RuntimeCoord {
        x: position.x,
        y: position.y,
        z: Some(position.z),
    }
}

fn runtime_mark_value_match3(match_value: CoreMarkValueMatch) -> RuntimeMarkValueMatch {
    match match_value {
        CoreMarkValueMatch::Any => RuntimeMarkValueMatch::Any,
        CoreMarkValueMatch::Exact => RuntimeMarkValueMatch::Exact,
    }
}

fn state3_cell_slots_equal(
    before: &GridState<3, Size3>,
    after: &GridState<3, Size3>,
    cell: usize,
) -> bool {
    if before.size != after.size || before.layer_count != after.layer_count {
        return false;
    }
    let layer_count = usize::from(after.layer_count);
    let start = cell * layer_count;
    before.slots()[start..start + layer_count] == after.slots()[start..start + layer_count]
}

fn animation_events3_contract(
    animation: &puzzle_lang::AnimationDef,
    before: &GridState<3, Size3>,
    after: &GridState<3, Size3>,
) -> Vec<RuntimeAnimationEvent> {
    if !animation.tween.enabled
        || before.size != after.size
        || before.layer_count != after.layer_count
    {
        return Vec::new();
    }
    let mut events = Vec::new();
    for object in changed_object_ids3(before, after) {
        let mut sources = changed_positions_for_object3(before, after, object, false);
        let targets = changed_positions_for_object3(before, after, object, true);
        for target in targets {
            if let Some(source_index) = sources
                .iter()
                .position(|source| adjacent_coord3(*source, target))
            {
                let source = sources.remove(source_index);
                events.push(RuntimeAnimationEvent::Move {
                    name: "tween".to_string(),
                    object_id: object.0,
                    from_object: None,
                    from: runtime_coord3(source),
                    to: runtime_coord3(target),
                });
            }
        }
    }
    events
}

fn changed_object_ids3(before: &GridState<3, Size3>, after: &GridState<3, Size3>) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for (before, after) in before.slots().iter().zip(after.slots().iter()) {
        for object in [*before, *after] {
            if !object.is_empty() && !objects.contains(&object) {
                objects.push(object);
            }
        }
    }
    objects.sort_by_key(|object| object.0);
    objects
}

fn changed_positions_for_object3(
    before: &GridState<3, Size3>,
    after: &GridState<3, Size3>,
    object: ObjectId,
    present_after: bool,
) -> Vec<Coord3> {
    let mut positions = Vec::new();
    for z in 0..after.size.height {
        for y in 0..after.size.depth {
            for x in 0..after.size.width {
                let coord = Coord3 { x, y, z };
                let had = state3_has_object(before, coord, object);
                let has = state3_has_object(after, coord, object);
                if had != has && has == present_after {
                    positions.push(coord);
                }
            }
        }
    }
    positions
}

fn state3_has_object(state: &GridState<3, Size3>, coord: Coord3, object: ObjectId) -> bool {
    let cell = ((usize::from(coord.z) * usize::from(state.size.depth)) + usize::from(coord.y))
        * usize::from(state.size.width)
        + usize::from(coord.x);
    let start = cell * usize::from(state.layer_count);
    state.slots()[start..start + usize::from(state.layer_count)].contains(&object)
}

fn adjacent_coord3(left: Coord3, right: Coord3) -> bool {
    let distance = left.x.abs_diff(right.x) + left.y.abs_diff(right.y) + left.z.abs_diff(right.z);
    distance == 1
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

    fn runtime_state_json(state: &GridState<3, Size3>) -> String {
        serde_json::to_string(&RuntimeStateSnapshot3d::from_state(state)).unwrap()
    }

    fn standalone_export(source: &str) -> Value {
        let document = puzzle_lang::parse_game_for_path(source, "export_test.puzzle").unwrap();
        json!({
            "runtimeLoadedDocument": {
                "version": 1,
                "document": serde_json::to_value(&document).unwrap(),
            },
        })
    }

    fn runtime_scene_fixture_source() -> &'static str {
        r#"
title = "Runtime Scene Fixture"

puzzle board {
slots {
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
            "compiledPlay": {"version": 2, "model": "grid2", "transition": [1, [[1, 0]], [], [[], [], []], [[[], [], []]]]},
        });

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("export without runtimeLoadedDocument should be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("runtimeLoadedDocument"));
    }

    #[test]
    fn standalone_session_from_export_does_not_parse_embedded_source() {
        let source = r#"
title = export_runtime_bundle
puzzle default {
slots {
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

        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string()).unwrap();
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();

        assert_eq!(snapshot["title"], "export_runtime_bundle");
    }

    #[test]
    fn standalone_session_from_export_reports_atomic_state_with_presentation_wait() {
        let source = r#"
title = export_wait_segments
puzzle default {
slots {
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
                .request_json("POST", "/api/input/right")
                .expect("input should complete with a presentation wait"),
        )
        .unwrap();
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "A"));
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "C"));
        assert!(cell_has_object(&waiting["scene"]["cells"][0], "B"));
        assert_eq!(waiting["presentationEvents"][0]["kind"], json!("wait"));
        assert_eq!(waiting["presentationEvents"][0]["milliseconds"], 100);

        let undone: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/undo")
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
slots {
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

        let moved: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/right").unwrap())
                .unwrap();
        let events = moved["presentationEvents"].as_array().unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event["kind"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["wait", "message", "wait", "animation", "play_sfx", "wait"]
        );
        assert!(events.iter().all(|event| event["levelIndex"] == 0));
        assert!(cell_has_object(&moved["scene"]["cells"][1], "Done"));
    }

    #[cfg(not(feature = "editor-debug"))]
    #[test]
    fn player_request_surface_rejects_editor_debug_input() {
        let error = standalone_session_request("POST", "/api/debug/input/right").unwrap_err();
        assert_eq!(
            error,
            "Unsupported exported HTML request: POST /api/debug/input/right"
        );
    }

    #[cfg(feature = "editor-debug")]
    #[test]
    fn standalone_session_debug_input_reports_rule_trace() {
        let source = r#"
title = "Debug Trace"

puzzle main {
  slots {
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
                .request_json("POST", "/api/debug/input/right")
                .unwrap(),
        )
        .unwrap();

        assert_eq!(body["snapshot"]["currentScene"], "main");
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
            serde_json::from_str(&bridge.request_json("GET", "/api/state").unwrap()).unwrap();
        assert_eq!(initial["currentScene"], "board");
        assert_eq!(initial["title"], "Runtime Scene Fixture");
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
                .request_json("POST", "/api/command/goto%20playing")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let save: Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(save["currentLevel"], "microban.1");
    }

    #[test]
    fn standalone_session_bridge_reports_no_scene_for_non_model_focus() {
        let source = r#"
title = runtime_focus
puzzle default {
slots {
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
level_menu
}
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_focus.puzzle").unwrap();

        let select: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20level_select")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(select["currentScene"], json!("level_select"));
        assert!(select["scene"].is_null());
        assert!(select["sceneLayers"][0]["scene"].is_null());

        let after_input: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/down").unwrap()).unwrap();
        assert_eq!(after_input["currentScene"], json!("level_select"));
        assert!(after_input["scene"].is_null());
        assert_eq!(after_input["presentationEvents"], json!([]));
    }

    #[test]
    fn standalone_session_bridge_starts_from_editor_state() {
        let source = r#"
title = editor_state_start

puzzle board {
slots {
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
            serde_json::from_str(&bridge.request_json("GET", "/api/state").unwrap()).unwrap();
        assert!(
            title["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|input| { input["name"] == "up" && input["arrow"] == "ArrowUp" })
        );

        let playing = start_spec_2d_new_game(&mut bridge);
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);
        assert_eq!(playing["scenePuzzles"], json!(["board"]));
        let playing_object = playing.as_object().unwrap();
        assert!(playing_object.contains_key("visibleScenes"));
        assert!(playing_object.contains_key("sceneState"));
        assert!(playing_object.contains_key("scenePuzzles"));
        assert!(!playing_object.contains_key("visibleScreens"));
        assert!(!playing_object.contains_key("screenState"));
        assert!(!playing_object.contains_key("screenPuzzles"));
        assert_eq!(
            playing["scenePuzzleState"]["board"]["level"]["name"],
            "microban.1"
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
                .request_json("POST", "/api/command/goto%20playing(0)")
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
                    .request_json("POST", &format!("/api/input/{input}"))
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
            .request_json("POST", "/api/command/clear_game_progress")
            .unwrap();
        serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20playing(%22microban.1%22)")
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
  slots {
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
                .request_json("POST", "/api/command/goto%20playing")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/right").unwrap())
                .unwrap();
        assert_eq!(
            moved["presentationEvents"][0]["animation"],
            json!({
                "kind": "move",
                "name": "tween",
                "objectId": 1,
                "from": { "x": 0, "y": 0 },
                "to": { "x": 1, "y": 0 }
            }),
            "unexpected moved snapshot: {moved}"
        );
    }





    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source = runtime_scene_fixture_source();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "runtime_scene_fixture.puzzle").unwrap();
        bridge
            .restore_progress_save_json(
                r#"{"version":1,"levels":[{"name":"microban.2","cleared":true}],"currentLevel":"microban.2","persistentVars":[]}"#,
            )
            .unwrap();

        let snapshot: Value =
            serde_json::from_str(&bridge.request_json("GET", "/api/state").unwrap()).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 1);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(snapshot["levels"][1]["cleared"], true);
    }

    #[test]
    fn standalone_session_runs_spatial_document_through_shared_scene_session() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3");
        let mut bridge = StandaloneSessionBridge::from_source(source, "spec_3d_full.puzzle3")
            .expect("spatial document should use the shared grid session");

        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["currentScene"], "sokoban");
        assert!(snapshot["scenePuzzleState"]["sokoban"]["cells"].is_array());
        bridge
            .request_json("POST", "/api/input/right")
            .expect("spatial model input should route through the shared session");
    }
}
