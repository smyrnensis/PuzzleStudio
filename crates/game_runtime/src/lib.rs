use puzzle_3d::{
    Coord3, Game3, InputId3, ObjectId as ObjectId3, ParsedPuzzle3, RuleId3, Size3, State3,
    transition_program_with_local_frame as transition_program_with_local_frame3,
    transition_program_without_input_with_local_frame,
};
use puzzle_core::{
    CompiledGame, InputId, ObjectId, RuleId, State as PuzzleState, transition_program,
};
use puzzle_lang::{
    ArrowKey, KeyTrigger, Level, LoadedDocumentModel, LoadedGame, ResourceSelection,
    SceneAlignXDef, SceneAlignYDef, SceneComponent, SceneDef, SceneEffect, SceneEffectParam,
    SceneExpr, SceneLayoutDef, ScenePuzzleInitializer, SceneStateLifetime, SceneTextContent,
    SceneTransitionTrigger, SceneValue, ThemeDef, ViewportModeDef, ViewportSizeDef, parse_game2d,
};
use puzzle_play::{
    AnimationEvent, GameSession, LevelProgressSaveData, MessageEvent, PersistentVarSaveData,
    ProgressSaveData, SoundEvent, WaitEvent, runtime_sounds_def,
};
use serde_json::{Value, json};

const PUZZLE3_SCENE_HOST_SOURCE: &str = r#"
title "__puzzle3_scene_host__"

puzzle scene_host {
layers {
__legacy_layer_0 = Marker
}
empty .
rules {

}
}

levels scene_host_levels of scene_host {
legend M = Marker
level scene_host {
M
}
}
"#;

pub struct StandaloneSessionBridge {
    loaded: LoadedGame,
    session: GameSession,
    has_progress_save: bool,
}

impl StandaloneSessionBridge {
    pub fn from_source(source: &str, puzzle_path: &str) -> Result<Self, String> {
        let document = puzzle_lang::parse_game_for_path(source, puzzle_path)
            .map_err(|error| error.to_string())?;
        let loaded = if document.models.len() > 1 {
            mixed_document_loaded_game(&document)?
        } else {
            match document.single_model() {
                Some(LoadedDocumentModel::Puzzle2d { game, .. }) => game.clone(),
                Some(LoadedDocumentModel::Puzzle3d { .. }) => {
                    puzzle3_document_scene_host_loaded_game(&document)?
                }
                None => return Err("standalone session bridge requires a puzzle model".to_string()),
            }
        };
        Ok(Self {
            session: GameSession::new(&loaded),
            loaded,
            has_progress_save: false,
        })
    }

    pub fn from_export_json(export_json: &str) -> Result<Self, String> {
        let export: Value = serde_json::from_str(export_json).map_err(|error| error.to_string())?;
        let runtime_bundle = export
            .get("runtimeLoadedGame")
            .ok_or_else(|| "standalone export is missing runtimeLoadedGame".to_string())?;
        let version = runtime_bundle
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| "runtimeLoadedGame is missing version".to_string())?;
        if version != 1 {
            return Err(format!("unsupported runtimeLoadedGame version: {version}"));
        }
        let loaded_value = runtime_bundle
            .get("loaded")
            .ok_or_else(|| "runtimeLoadedGame is missing loaded game".to_string())?;
        let loaded: LoadedGame = serde_json::from_value(loaded_value.clone())
            .map_err(|error| format!("invalid runtimeLoadedGame loaded game: {error}"))?;
        Ok(Self {
            session: GameSession::new(&loaded),
            loaded,
            has_progress_save: false,
        })
    }

    pub fn snapshot_json(&mut self) -> String {
        let sound_events = self.session.take_sound_events();
        let message_events = self.session.take_message_events();
        let wait_events = self.session.take_wait_events();
        let animation_events = self.session.take_animation_events();
        serde_json::to_string(&self.snapshot_value(
            &sound_events,
            &message_events,
            &wait_events,
            &animation_events,
        ))
        .expect("snapshot JSON should serialize")
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, String> {
        match (method, url) {
            ("GET", "/api/state") => Ok(self.snapshot_json()),
            ("POST", "/api/command/undo") => {
                self.session.undo(&self.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/redo") => {
                self.session.redo(&self.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/restart") => {
                self.session.restart_level(&self.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/next") => {
                self.session.advance_level(&self.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/input/") => {
                self.apply_input_name(&percent_decode(&path["/api/input/".len()..]))?;
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/command/") => {
                self.apply_command_name(&percent_decode(&path["/api/command/".len()..]))?;
                Ok(self.snapshot_json())
            }
            _ => Err(format!("Unsupported exported HTML request: {method} {url}")),
        }
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.session
            .apply_input(&self.loaded, input)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.session
            .apply_command(&self.loaded, command_name)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        if level_index >= self.loaded.levels.len() {
            return Err(format!("level index out of range: {level_index}"));
        }
        let value: Value = serde_json::from_str(state_json).map_err(|error| error.to_string())?;
        let state = decode_state_value(&self.loaded.game, &value)?;
        self.session
            .start_level_from_state(&self.loaded, level_index, state, materialize_level_start)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn progress_save_json(&self) -> String {
        progress_save_data_value(&self.session.progress_save_data(&self.loaded)).to_string()
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        let save = progress_save_data_from_json(save_json)?;
        self.session
            .restore_progress_save_data(&self.loaded, &save)
            .map_err(|error| format!("{error:?}"))?;
        self.has_progress_save = true;
        Ok(())
    }

    pub fn mark_progress_save_written(&mut self) {
        self.has_progress_save = true;
    }

    pub fn clear_progress_save(&mut self) {
        self.has_progress_save = false;
    }

    fn snapshot_value(
        &self,
        sound_events: &[SoundEvent],
        message_events: &[MessageEvent],
        wait_events: &[WaitEvent],
        animation_events: &[AnimationEvent],
    ) -> Value {
        let current_scene = self.session.focused_scene();
        let scene = focused_scene_value(&self.loaded, &self.session);
        let scene_state = scene_state_value(self.session.scene_state());
        let scene_puzzles = scene_puzzles_value(self.session.scene_state());
        let scene_puzzle_state = scene_puzzle_state_value(&self.loaded, &self.session);
        json!({
            "title": self.loaded.title,
            "subtitle": self.loaded.subtitle,
            "author": self.loaded.author,
            "homepage": self.loaded.homepage,
            "has_progress_save": self.has_progress_save,
            "sounds": sounds_value(&self.loaded),
            "theme": theme_value(&self.loaded.theme),
            "defaultWaitMs": self.loaded.default_wait_ms,
            "defaultAgainMs": self.loaded.default_again_ms,
            "animation": animation_value(&self.loaded),
            "soundEvents": sound_events_value(sound_events),
            "messageEvents": message_events_value(message_events),
            "waitEvents": wait_events_value(wait_events),
            "animationEvents": animation_events_value(animation_events),
            "level": level_context_value(&self.loaded, &self.session),
            "levelIndex": self.session.active_level_index(),
            "levelCount": self.loaded.levels.len(),
            "scene": scene,
            "currentScene": current_scene,
            "focusedScreen": current_scene,
            "focusedScene": current_scene,
            "visibleScenes": self.session.visible_scenes(),
            "gameState": scene_values_value(self.session.session_values()),
            "sceneState": scene_state,
            "scenePuzzles": scene_puzzles,
            "scenePuzzleState": scene_puzzle_state,
            "sceneLayers": scene_layers_value(&self.loaded, &self.session),
            "selectedLevelIndex": self.session.selected_level_index(),
            "busy": !wait_events.is_empty(),
            "canUndo": self.session.can_undo(),
            "canRedo": self.session.can_redo(),
            "inputs": inputs_value(&self.loaded),
            "levels": levels_value(&self.loaded, self.session.cleared_levels()),
            "scenes": scenes_value(&self.loaded),
            "screens": scenes_value(&self.loaded),
        })
    }
}

fn decode_state_value(game: &CompiledGame, value: &Value) -> Result<PuzzleState, String> {
    let width = u16_field(value, "width")?;
    let height = u16_field(value, "height")?;
    let layer_count = u16_field(value, "layerCount")?;
    let globals = value
        .get("globals")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_i64()
                        .ok_or_else(|| "global must be an integer".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut state =
        PuzzleState::empty_with_globals(width, height, layer_count, game.object_count(), globals)
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

#[cfg(test)]
fn compiled_state_value(state: &PuzzleState) -> Value {
    json!({
        "width": state.width,
        "height": state.height,
        "layerCount": state.layer_count,
        "slots": state.slots().iter().map(|object| object.0).collect::<Vec<_>>(),
        "scratch": [],
        "globals": state.visible_globals(),
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

pub struct Puzzle3RuntimeBridge {
    parsed: ParsedPuzzle3,
    animation: puzzle_lang::AnimationDef,
    current_state: Option<State3>,
    saved_states: SavedStateStore<State3>,
}

impl Puzzle3RuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        if let Ok(parsed) = puzzle_3d::parse_puzzle3d(source) {
            return Ok(Self {
                parsed,
                animation: Default::default(),
                current_state: None,
                saved_states: SavedStateStore::new(),
            });
        }
        let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
        let animation = document.animation.clone();
        let parsed = document
            .models
            .iter()
            .find_map(|model| match model {
                LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle.clone()),
                LoadedDocumentModel::Puzzle2d { .. } => None,
            })
            .ok_or_else(|| "3D runtime source does not contain a puzzle3 model".to_string())?;
        Ok(Self {
            parsed,
            animation,
            current_state: None,
            saved_states: SavedStateStore::new(),
        })
    }

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        self.current_state = Some(state3_from_json(&self.parsed.game, state_json)?);
        Ok(())
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        Ok(state3_cells_value(state, None).to_string())
    }

    pub fn is_current_complete(&self) -> Result<bool, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        Ok(self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, state)))
    }

    pub fn save_current_state(&mut self) -> Result<u32, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        Ok(self.saved_states.save(state.clone()))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), String> {
        self.current_state = Some(self.saved_states.restore(handle)?.clone());
        Ok(())
    }

    pub fn transition_current_outcome_json(
        &mut self,
        program_key: &str,
        input: u16,
    ) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let before = state.clone();
        let next_state =
            transition_selected_program3(&self.parsed, program_key, state, InputId3(input))?;
        let completed = self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, &next_state));
        self.current_state = Some(next_state.clone());
        Ok(json!({
            "changed": before != next_state,
            "completed": completed,
            "stateHash": next_state.hash(),
            "changedCells": state3_cells_value(&next_state, Some(&before)),
            "animationEvents": animation_events3_value(&self.animation, &before, &next_state),
            "commands": [],
        })
        .to_string())
    }
}

struct SavedStateStore<T> {
    states: Vec<Option<T>>,
}

impl<T> SavedStateStore<T> {
    fn new() -> Self {
        Self { states: Vec::new() }
    }

    fn save(&mut self, state: T) -> u32 {
        if let Some(index) = self.states.iter().position(Option::is_none) {
            self.states[index] = Some(state);
            return index as u32;
        }
        self.states.push(Some(state));
        (self.states.len() - 1) as u32
    }

    fn restore(&self, handle: u32) -> Result<&T, String> {
        self.states
            .get(handle as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| format!("saved state handle {handle} does not exist"))
    }
}

fn focused_scene_value(loaded: &LoadedGame, session: &GameSession) -> Value {
    if let Some((state, level)) = scene_puzzle_state(loaded, session, session.focused_scene()) {
        return scene_value_for_state(
            loaded,
            state,
            level,
            scene_resources(loaded, session.focused_scene()),
        );
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
    let display_state = materialize_display_state(loaded, state);
    let state = display_state.as_ref().unwrap_or(state);
    let mut cells = Vec::new();
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
        "settings": {
            "animation": animation_value(loaded),
        },
        "animation": animation_value(loaded),
        "screen": screen_value(loaded),
        "regions": regions,
        "resources": scene_resources_value(resources),
        "cells": cells,
    })
}

fn materialize_display_state(loaded: &LoadedGame, state: &PuzzleState) -> Option<PuzzleState> {
    let program = loaded.display_program.as_deref()?;
    transition_program(&loaded.game, program, state, InputId(0)).ok()
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

fn scene_puzzles_value(state: Option<&puzzle_play::SceneRuntimeState>) -> Vec<Value> {
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
        let level = state
            .puzzles
            .get(name)
            .and_then(|puzzle| puzzle.level_index)
            .map(|level_index| {
                level_ref_value(loaded, session, session.focused_scene(), level_index)
            });
        entries.insert(name.clone(), json!({ "level": level }));
    }
    Value::Object(entries)
}

fn scenes_value(loaded: &LoadedGame) -> Value {
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
                    value.insert("condition".to_string(), Value::String(condition.clone()));
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
            value.insert("kind".to_string(), Value::String(puzzle.kind.clone()));
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
    if let Some(size) = layout.size {
        value.insert(
            "size".to_string(),
            json!({ "width": size.width, "height": size.height }),
        );
    }
    if let Some(gap) = layout.gap {
        value.insert("gap".to_string(), json!(gap));
    }
    if layout.align != SceneLayoutDef::default().align {
        value.insert(
            "align".to_string(),
            json!({
                "x": match layout.align.x {
                    SceneAlignXDef::Left => "left",
                    SceneAlignXDef::Center => "center",
                    SceneAlignXDef::Right => "right",
                },
                "y": match layout.align.y {
                    SceneAlignYDef::Top => "top",
                    SceneAlignYDef::Center => "center",
                    SceneAlignYDef::Bottom => "bottom",
                },
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
        SceneComponent::Frame(frame) => json!({
            "kind": frame.kind,
            "source": frame.source,
            "layout": scene_layout_value(&frame.layout),
        }),
        SceneComponent::Title(title) => json!({
            "kind": "title",
            "content": scene_expr_value(&title.content),
        }),
        SceneComponent::Subtitle(subtitle) => json!({
            "kind": "subtitle",
            "content": scene_expr_value(&subtitle.content),
        }),
        SceneComponent::Text(text) => {
            let mut value = serde_json::Map::new();
            value.insert("kind".to_string(), Value::String("text".to_string()));
            match &text.content {
                SceneTextContent::Literal(text) => {
                    value.insert("source".to_string(), Value::String("literal".to_string()));
                    value.insert("value".to_string(), Value::String(text.clone()));
                }
                SceneTextContent::Path(path) => {
                    value.insert("source".to_string(), Value::String("path".to_string()));
                    value.insert("path".to_string(), Value::String(path.join(".")));
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
        SceneEffect::Sequence(effects) => json!({
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
        SceneExpr::Call { name, args } => json!({
            "kind": "call",
            "name": name,
            "args": args.iter().map(scene_expr_value).collect::<Vec<_>>(),
        }),
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
        .level_index
        .and_then(|index| loaded.levels.get(index));
    Some((&puzzle.state, level))
}

fn first_puzzle_component(components: &[SceneComponent]) -> Option<&str> {
    for component in components {
        match component {
            SceneComponent::Frame(frame) if frame.kind == "puzzle" || frame.kind == "frame" => {
                return Some(frame.source.as_str());
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

fn mixed_document_loaded_game(
    document: &puzzle_lang::LoadedDocument,
) -> Result<LoadedGame, String> {
    let Some(LoadedDocumentModel::Puzzle2d { game, .. }) = document
        .models
        .iter()
        .find(|model| matches!(model, LoadedDocumentModel::Puzzle2d { .. }))
    else {
        return Err("mixed HTML export requires a 2D puzzle model host".to_string());
    };
    let mut loaded = game.clone();
    loaded.title = document.title.clone();
    loaded.subtitle = document.subtitle.clone();
    loaded.author = document.author.clone();
    loaded.homepage = document.homepage.clone();
    loaded.default_wait_ms = document.default_wait_ms;
    loaded.default_again_ms = document.default_again_ms;
    loaded.sounds = document.sounds.clone();
    loaded.theme = document.theme.clone();
    loaded.assets = document.assets.clone();
    loaded.scenes = document
        .scenes
        .iter()
        .cloned()
        .map(scene_with_only_2d_puzzle_state)
        .collect();
    Ok(loaded)
}

fn puzzle3_document_scene_host_loaded_game(
    document: &puzzle_lang::LoadedDocument,
) -> Result<LoadedGame, String> {
    let mut loaded = parse_game2d(PUZZLE3_SCENE_HOST_SOURCE).map_err(|error| error.to_string())?;
    let prototype_level = loaded
        .levels
        .first()
        .cloned()
        .ok_or_else(|| "puzzle3 scene host must contain a prototype level".to_string())?;
    let Some(LoadedDocumentModel::Puzzle3d { name, puzzle }) = document
        .models
        .iter()
        .find(|model| matches!(model, LoadedDocumentModel::Puzzle3d { .. }))
    else {
        return Err("puzzle3 scene host requires a 3D puzzle model".to_string());
    };
    let Some(bundle) = puzzle.level_bundle.as_ref() else {
        return Err("puzzle3 scene host requires 3D levels".to_string());
    };

    loaded.title = document.title.clone();
    loaded.subtitle = document.subtitle.clone();
    loaded.author = document.author.clone();
    loaded.homepage = document.homepage.clone();
    loaded.default_wait_ms = document.default_wait_ms;
    loaded.default_again_ms = document.default_again_ms;
    loaded.animation = document.animation.clone();
    loaded.sounds = document.sounds.clone();
    loaded.theme = document.theme.clone();
    loaded.assets = document.assets.clone();
    loaded.scenes = document
        .scenes
        .iter()
        .cloned()
        .map(scene_without_model_puzzle_state)
        .collect();
    loaded.levels = bundle
        .levels
        .iter()
        .map(|entry| Level {
            name: entry.name.clone(),
            pack: None,
            puzzle: name.clone(),
            initial_state: prototype_level.initial_state.clone(),
            regions: Vec::new(),
            level_start_program: None,
            level_clear_program: None,
        })
        .collect();
    Ok(loaded)
}

fn scene_with_only_2d_puzzle_state(mut scene: SceneDef) -> SceneDef {
    scene.state.puzzles.retain(|puzzle| puzzle.kind == "puzzle");
    if let Some(rule) = &scene.puzzle_rule {
        let target = rule
            .target
            .split('.')
            .next_back()
            .unwrap_or(rule.target.as_str());
        if !scene
            .state
            .puzzles
            .iter()
            .any(|puzzle| puzzle.name == target)
        {
            scene.puzzle_rule = None;
        }
    }
    scene
}

fn scene_without_model_puzzle_state(mut scene: SceneDef) -> SceneDef {
    scene.state.puzzles.clear();
    scene.puzzle_rule = None;
    scene
}

fn input_id_by_name(loaded: &LoadedGame, input_name: &str) -> Option<InputId> {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

fn object_name(loaded: &LoadedGame, object: ObjectId) -> String {
    loaded
        .object_labels
        .get(&object)
        .cloned()
        .unwrap_or_else(|| "?".to_string())
}

fn object_id_by_name(loaded: &LoadedGame, object_name: &str) -> Option<ObjectId> {
    loaded
        .object_labels
        .iter()
        .find_map(|(id, label)| (label == object_name).then_some(*id))
}

fn sounds_value(loaded: &LoadedGame) -> Value {
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

fn animation_value(loaded: &LoadedGame) -> Value {
    json!({
        "tween": {
            "enabled": loaded.animation.tween.enabled,
            "intervalMs": loaded.animation.tween.interval_ms,
        }
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

fn sound_events_value(events: &[SoundEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| match event {
            SoundEvent::PlaySfx { name } => json!({"kind": "play_sfx", "name": name}),
            SoundEvent::PlayMusic { name } => json!({"kind": "play_music", "name": name}),
            SoundEvent::PauseMusic { name } => json!({"kind": "pause_music", "name": name}),
            SoundEvent::ResumeMusic { name } => json!({"kind": "resume_music", "name": name}),
            SoundEvent::StopMusic { name } => json!({"kind": "stop_music", "name": name}),
        })
        .collect()
}

fn message_events_value(events: &[MessageEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| match event {
            MessageEvent::Message { text } => json!({"kind": "message", "text": text}),
        })
        .collect()
}

fn wait_events_value(events: &[WaitEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| match event {
            WaitEvent::Wait { milliseconds } => {
                json!({"kind": "wait", "milliseconds": milliseconds})
            }
            WaitEvent::ContinueEffects { milliseconds } => {
                json!({"kind": "continue_effects", "milliseconds": milliseconds})
            }
        })
        .collect()
}

fn animation_events_value(events: &[AnimationEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| match event {
            AnimationEvent::Move {
                name,
                object,
                from_x,
                from_y,
                from_z,
                to_x,
                to_y,
                to_z,
            } => json!({
                "kind": "move",
                "name": name,
                "objectId": object.0,
                "fromX": from_x,
                "fromY": from_y,
                "fromZ": from_z,
                "toX": to_x,
                "toY": to_y,
                "toZ": to_z,
            }),
            AnimationEvent::CantMove { name, object, x, y } => json!({
                "kind": "cant_move",
                "name": name,
                "objectId": object.0,
                "x": x,
                "y": y,
            }),
        })
        .collect()
}

fn level_context_value(loaded: &LoadedGame, session: &GameSession) -> Value {
    let level = session.current_level(loaded);
    json!({
        "index": session.level_index(),
        "name": level.name,
        "pack": level.pack,
        "puzzle": level.puzzle,
        "cleared": session.cleared_levels().get(session.level_index()).copied().unwrap_or(false),
    })
}

fn inputs_value(loaded: &LoadedGame) -> Vec<Value> {
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

fn key_for_input(loaded: &LoadedGame, input: InputId) -> Option<String> {
    loaded
        .controls
        .keys
        .iter()
        .find_map(|(key, id)| (*id == input).then_some(char::from(*key).to_string()))
}

fn arrow_for_input(loaded: &LoadedGame, input: InputId) -> Option<String> {
    loaded
        .controls
        .arrows
        .iter()
        .find_map(|(arrow, id)| (*id == input).then_some(arrow_name(*arrow).to_string()))
}

fn key_triggers_for_input(loaded: &LoadedGame, input: InputId) -> Vec<String> {
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

fn levels_value(loaded: &LoadedGame, cleared_levels: &[bool]) -> Vec<Value> {
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

fn scene_state_value(state: Option<&puzzle_play::SceneRuntimeState>) -> Value {
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

fn percent_decode(value: &str) -> String {
    let mut out = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte as char);
                    index += 3;
                    continue;
                }
            }
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

fn state3_from_json(game: &Game3, state_json: &str) -> Result<State3, String> {
    let value: Value = serde_json::from_str(state_json).map_err(|error| error.to_string())?;
    let width = json_u16(&value, "width")?;
    let depth = json_u16(&value, "depth")?;
    let height = json_u16(&value, "height")?;
    let layer_count = value
        .get("layerCount")
        .and_then(Value::as_u64)
        .map(u16::try_from)
        .transpose()
        .map_err(|_| "3D state layerCount out of range".to_string())?
        .unwrap_or(game.layer_count);
    if layer_count != game.layer_count {
        return Err(format!(
            "3D state layerCount mismatch: expected {}, got {layer_count}",
            game.layer_count
        ));
    }
    let slots = value
        .get("slots")
        .and_then(Value::as_array)
        .ok_or_else(|| "3D state missing slots".to_string())?;
    let expected_slots = usize::from(width)
        .checked_mul(usize::from(depth))
        .and_then(|count| count.checked_mul(usize::from(height)))
        .and_then(|count| count.checked_mul(usize::from(layer_count)))
        .ok_or_else(|| "3D state dimensions are too large".to_string())?;
    if slots.len() != expected_slots {
        return Err(format!(
            "3D state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        ));
    }
    let mut state = State3::empty(Size3::new(width, depth, height), layer_count)
        .map_err(|error| format!("{error:?}"))?;
    for (index, object) in slots.iter().enumerate() {
        let object = object
            .as_u64()
            .ok_or_else(|| "3D state slot is not a number".to_string())?;
        if object == 0 {
            continue;
        }
        let object: u16 = object
            .try_into()
            .map_err(|_| "3D state object id out of range".to_string())?;
        let layer = index % usize::from(layer_count);
        let cell = index / usize::from(layer_count);
        let x = (cell % usize::from(width)) as u16;
        let yz = cell / usize::from(width);
        let y = (yz % usize::from(depth)) as u16;
        let z = (yz / usize::from(depth)) as u16;
        let object = ObjectId3(object);
        let expected_layer = game
            .object_layer(object)
            .ok_or_else(|| format!("3D state unknown object id {}", object.0))?;
        if usize::from(expected_layer.0) != layer {
            return Err(format!(
                "3D state object {} is in layer {layer}, expected {}",
                object.0, expected_layer.0
            ));
        }
        state
            .place_object(game, Coord3 { x, y, z }, object)
            .map_err(|error| format!("{error:?}"))?;
    }
    if let Some(fired_rules) = value.get("levelFiredRules").and_then(Value::as_array) {
        for rule in fired_rules {
            let rule = rule
                .as_u64()
                .ok_or_else(|| "3D state fired rule is not a number".to_string())?;
            let rule: u16 = rule
                .try_into()
                .map_err(|_| "3D state rule id out of range".to_string())?;
            state.mark_level_rule_fired(RuleId3(rule));
        }
    }
    Ok(state)
}

fn json_u16(value: &Value, key: &str) -> Result<u16, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("3D state missing {key}"))?
        .try_into()
        .map_err(|_| format!("3D state {key} out of range"))
}

fn transition_selected_program3(
    parsed: &ParsedPuzzle3,
    program_key: &str,
    state: &State3,
    input: InputId3,
) -> Result<State3, String> {
    match program_key {
        "main" => transition_program_with_local_frame3(
            &parsed.game,
            state,
            &parsed.rules,
            input,
            parsed.local_frame.as_ref(),
        ),
        "level_start" => transition_program_without_input_with_local_frame(
            &parsed.game,
            state,
            &parsed.lifecycle.on_level_start,
            parsed.lifecycle.on_level_start_local_frame.as_ref(),
        ),
        other => return Err(format!("unknown 3D transition program selector: {other}")),
    }
    .map_err(|error| format!("{error:?}"))
}

fn state3_cells_value(state: &State3, before: Option<&State3>) -> Value {
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

fn state3_cell_slots_equal(before: &State3, after: &State3, cell: usize) -> bool {
    if before.size != after.size || before.layer_count != after.layer_count {
        return false;
    }
    let layer_count = usize::from(after.layer_count);
    let start = cell * layer_count;
    before.slots()[start..start + layer_count] == after.slots()[start..start + layer_count]
}

fn animation_events3_value(
    animation: &puzzle_lang::AnimationDef,
    before: &State3,
    after: &State3,
) -> Value {
    if !animation.tween.enabled
        || before.size != after.size
        || before.layer_count != after.layer_count
    {
        return json!([]);
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
                events.push(json!({
                    "kind": "move",
                    "name": "tween",
                    "objectId": object.0,
                    "fromX": source.x,
                    "fromY": source.y,
                    "fromZ": source.z,
                    "toX": target.x,
                    "toY": target.y,
                    "toZ": target.z,
                }));
            }
        }
    }
    Value::Array(events)
}

fn changed_object_ids3(before: &State3, after: &State3) -> Vec<ObjectId3> {
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
    before: &State3,
    after: &State3,
    object: ObjectId3,
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

fn state3_has_object(state: &State3, coord: Coord3, object: ObjectId3) -> bool {
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

    fn standalone_export(source: &str) -> Value {
        let document =
            puzzle_lang::parse_game_for_path(source, "games/export_test.puzzle").unwrap();
        let loaded = match document.single_model() {
            Some(LoadedDocumentModel::Puzzle2d { game, .. }) => game.clone(),
            Some(LoadedDocumentModel::Puzzle3d { .. }) => {
                puzzle3_document_scene_host_loaded_game(&document).unwrap()
            }
            None => panic!("test export requires a puzzle model"),
        };
        json!({
            "runtimeLoadedGame": {
                "version": 1,
                "loaded": serde_json::to_value(&loaded).unwrap(),
            },
        })
    }

    #[test]
    fn standalone_session_from_export_requires_runtime_loaded_game() {
        let export = json!({
            "source": "title invalid\nlevels {\nlegend {\nP = Player\n}\nP\n}\n",
            "puzzlePath": "games/compiled_export.puzzle",
            "compiledPlay": {"version": 1, "model": "grid2", "transition": [1, [[1, 0]], [], [], [[], [], [], [], [], []], [[[], []]]]},
        });

        let error = match StandaloneSessionBridge::from_export_json(&export.to_string()) {
            Ok(_) => panic!("export without runtimeLoadedGame should be rejected"),
            Err(error) => error,
        };
        assert!(error.contains("runtimeLoadedGame"));
    }

    #[test]
    fn standalone_session_from_export_does_not_parse_embedded_source() {
        let source = r#"
title export_runtime_bundle
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
level start {
P
}
}
}
"#;
        let mut export = standalone_export(source);
        export["source"] = json!("this is not puzzle syntax");
        export["puzzlePath"] = json!("games/bad_source_path.puzzle");

        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string()).unwrap();
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();

        assert_eq!(snapshot["title"], "export_runtime_bundle");
    }

    #[test]
    fn standalone_session_from_export_uses_game_session_wait_continuation() {
        let source = r#"
title export_wait_segments
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
level start {
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
                .expect("input should start wait continuation"),
        )
        .unwrap();
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "A"));
        assert!(cell_has_object(&waiting["scene"]["cells"][0], "C"));
        assert!(!cell_has_object(&waiting["scene"]["cells"][0], "B"));
        assert_eq!(
            waiting["waitEvents"],
            json!([{ "kind": "continue_effects", "milliseconds": 100 }])
        );

        let continued: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/__continue_effects")
                .expect("continuation should finish wait segment"),
        )
        .unwrap();
        assert!(!cell_has_object(&continued["scene"]["cells"][0], "A"));
        assert!(!cell_has_object(&continued["scene"]["cells"][0], "C"));
        assert!(cell_has_object(&continued["scene"]["cells"][0], "B"));

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
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();

        let title: Value =
            serde_json::from_str(&bridge.request_json("GET", "/api/state").unwrap()).unwrap();
        assert_eq!(title["currentScene"], "title");
        assert_eq!(title["title"], "Microban Basic");
        assert_eq!(title["scenes"][0]["name"], "title");
        assert_eq!(title["scenes"][0]["components"][0]["kind"], "title");

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
    fn standalone_session_bridge_starts_from_editor_state() {
        let source = r#"
title editor_state_start

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
level authored {
.P
}
level editor_state {
P.
}
}
}

scene playing {
state {
board = puzzle board
}
layout {
board
}
rules {
step board
}
}
"#;
        let loaded = parse_game2d(source).unwrap();
        let editor_state = compiled_state_value(&loaded.levels[1].initial_state).to_string();
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/editor_state_start.puzzle")
                .unwrap();

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
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();

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
        let source = include_str!("../../../games/Transition.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/Transition.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/new_game")
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
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut changed_input = None;

        for input in ["up", "down", "left", "right"] {
            let mut bridge =
                StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();
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
                .request_json("POST", "/api/command/goto%20playing(microban.1)")
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn standalone_session_bridge_emits_fixban_tween_on_first_input() {
        let source = include_str!("../../../games/fixban_tween.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/fixban_tween.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20playing(fixban.level_1)")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/up").unwrap()).unwrap();
        assert_eq!(
            moved["animationEvents"],
            json!([
                {
                    "kind": "move",
                    "name": "tween",
                    "objectId": 19,
                    "fromX": 2,
                    "fromY": 5,
                    "fromZ": 0,
                    "toX": 2,
                    "toY": 4,
                    "toZ": 0
                }
            ])
        );
    }

    #[test]
    fn puzzle3_runtime_bridge_emits_tween_move_events() {
        let source = r#"
title "3D Tween"

animation {
  tween {
    duration = 300ms
  }
}

puzzle3 sokoban {
layers {
  solid = Player
}

rules {
  input horizontal [ Player | no Player ] -> [ | Player ]
}
}
"#;
        let mut bridge = Puzzle3RuntimeBridge::from_source(source).unwrap();
        bridge
            .set_state_json(
                r#"{"kind":"puzzle3d","width":2,"depth":1,"height":1,"layerCount":1,"slots":[1,0],"levelFiredRules":[]}"#,
            )
            .unwrap();

        let moved: Value =
            serde_json::from_str(&bridge.transition_current_outcome_json("main", 1).unwrap())
                .unwrap();

        assert_eq!(
            moved["animationEvents"],
            json!([
                {
                    "kind": "move",
                    "name": "tween",
                    "objectId": 1,
                    "fromX": 0,
                    "fromY": 0,
                    "fromZ": 0,
                    "toX": 1,
                    "toY": 0,
                    "toZ": 0
                }
            ])
        );
    }

    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();
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
    fn standalone_session_bridge_supports_single_puzzle3_document() {
        let source = include_str!("../../../games/spec_3d.puzzle3");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_3d.puzzle3").unwrap();
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["currentScene"], json!("title"));
        assert_eq!(snapshot["title"], json!("Microban 3D"));
    }
}
