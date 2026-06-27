use puzzle_3d::{
    Coord3, Game3, InputId3, ObjectId as ObjectId3, ParsedPuzzle3, RuleId3, Size3, State3,
    transition_program_with_local_frame as transition_program_with_local_frame3,
    transition_program_without_input_with_local_frame,
};
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionId, ConditionValueKind, Effect, GapTerm,
    GlobalId, GlobalUpdateOp, Guard, InputId, LayerId, LocalFrame, LocalFrameExtent, MatchCell,
    ObjectDef, ObjectId, ObjectSetMatcher, ObjectSetScratchPattern, Offset, Pattern,
    PatternComponent, Rule, RuleApplication, RuleCondition, RuleId, RuleStep, ScratchId,
    ScratchPattern, ScratchValueMatch, State as PuzzleState, TransitionCommand, WriteOp,
    transition_program, transition_program_outcome,
};
use puzzle_lang::{
    ArrowKey, KeyTrigger, Level, LoadedDocumentModel, LoadedGame, ResourceSelection,
    SceneAlignXDef, SceneAlignYDef, SceneComponent, SceneDef, SceneEffect, SceneEffectParam,
    SceneExpr, SceneLayoutDef, ScenePuzzleInitializer, SceneStateLifetime, SceneTextContent,
    SceneTransitionTrigger, SceneValue, ThemeDef, ViewportModeDef, ViewportSizeDef, parse_game2d,
};
use puzzle_play::{
    AnimationEvent, GameSession, LevelProgressSaveData, MessageEvent, PersistentVarSaveData,
    ProgressSaveData, SoundEvent, WaitEvent,
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
        let source = export
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| "standalone export is missing source".to_string())?;
        let puzzle_path = export
            .get("puzzlePath")
            .and_then(Value::as_str)
            .ok_or_else(|| "standalone export is missing puzzlePath".to_string())?;
        Self::from_source(source, puzzle_path)
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

pub struct CompiledStandaloneSessionBridge {
    export: Value,
    engine: CompiledEngine,
    current_state: PuzzleState,
    current_level_index: usize,
    current_scene: String,
    cleared_levels: Vec<bool>,
    undo_stack: Vec<PuzzleState>,
    redo_stack: Vec<PuzzleState>,
    has_progress_save: bool,
    pending_animation_events: Vec<Value>,
}

impl CompiledStandaloneSessionBridge {
    pub fn from_export_json(export_json: &str) -> Result<Self, String> {
        let export: Value = serde_json::from_str(export_json).map_err(|error| error.to_string())?;
        let engine = decode_export_engine(&export)?;
        let level_count = compiled_export_levels(&export)?.len();
        if level_count == 0 {
            return Err("compiled standalone session requires at least one level".to_string());
        }
        let current_state = decode_level_initial_state(&engine.game, &export, 0)?;
        let current_scene = initial_export_scene(&export);
        let mut session = Self {
            export,
            engine,
            current_state,
            current_level_index: 0,
            current_scene,
            cleared_levels: vec![false; level_count],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            has_progress_save: false,
            pending_animation_events: Vec::new(),
        };
        session.load_level(0, false)?;
        Ok(session)
    }

    pub fn snapshot_json(&mut self) -> String {
        self.snapshot_value().to_string()
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, String> {
        match (method, url) {
            ("GET", "/api/state") => Ok(self.snapshot_json()),
            ("POST", "/api/command/undo") => {
                self.undo();
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/redo") => {
                self.redo();
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/restart") => {
                self.load_level(self.current_level_index, true)?;
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/next") => {
                self.advance_level()?;
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
        let input = self
            .input_id_by_name(input_name)
            .ok_or_else(|| format!("unknown input: {input_name}"))?;
        self.apply_main_input(input)
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        if command_name == "__continue_effects" {
            return Ok(());
        }
        if let Some(position) = command_name.strip_prefix("select:") {
            let index = position
                .parse::<usize>()
                .map_err(|_| format!("invalid level selection: {position}"))?;
            return self.load_level(index, true);
        }
        match command_name {
            "undo" => self.undo(),
            "redo" => self.redo(),
            "restart" => self.load_level(self.current_level_index, true)?,
            "next" => self.advance_level()?,
            other => {
                if let Some(scene) = other.strip_prefix("goto ") {
                    self.current_scene = scene.to_string();
                } else if let Some(input) = self.input_id_by_name(other) {
                    self.apply_main_input(input)?;
                } else {
                    return Err(format!("unsupported compiled export command: {other}"));
                }
            }
        }
        Ok(())
    }

    pub fn progress_save_json(&self) -> String {
        json!({
            "version": 1,
            "compiledSession": {
                "levelIndex": self.current_level_index,
                "state": compiled_state_value(&self.current_state),
                "clearedLevels": self.cleared_levels,
            }
        })
        .to_string()
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        let save: Value = serde_json::from_str(save_json).map_err(|error| error.to_string())?;
        let session = save
            .get("compiledSession")
            .ok_or_else(|| "progress save is not a compiled standalone session".to_string())?;
        let level_index = session
            .get("levelIndex")
            .and_then(Value::as_u64)
            .ok_or_else(|| "compiled progress save is missing levelIndex".to_string())?
            as usize;
        if level_index >= compiled_export_levels(&self.export)?.len() {
            return Err("compiled progress save level index is out of range".to_string());
        }
        self.current_level_index = level_index;
        self.current_state = decode_state_value(
            &self.engine.game,
            session
                .get("state")
                .ok_or_else(|| "compiled progress save is missing state".to_string())?,
        )?;
        if let Some(items) = session.get("clearedLevels").and_then(Value::as_array) {
            self.cleared_levels = items
                .iter()
                .map(|item| item.as_bool().unwrap_or(false))
                .collect();
            self.cleared_levels
                .resize(compiled_export_levels(&self.export)?.len(), false);
        }
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.has_progress_save = true;
        Ok(())
    }

    pub fn mark_progress_save_written(&mut self) {
        self.has_progress_save = true;
    }

    pub fn clear_progress_save(&mut self) {
        self.has_progress_save = false;
    }

    fn apply_main_input(&mut self, input: InputId) -> Result<(), String> {
        let before = self.current_state.clone();
        let commands = self.transition_program("main", input)?;
        if before != self.current_state {
            self.undo_stack.push(before);
            self.redo_stack.clear();
        }
        self.apply_transition_commands(&commands)
    }

    fn transition_program(
        &mut self,
        program_key: &str,
        input: InputId,
    ) -> Result<Vec<TransitionCommand>, String> {
        let program = self
            .engine
            .program(program_key, self.current_level_index)
            .ok_or_else(|| format!("unknown compiled program: {program_key}"))?
            .to_vec();
        if program.is_empty() {
            return Ok(Vec::new());
        }
        let before = self.current_state.clone();
        let outcome = transition_program_outcome(&self.engine.game, &program, &before, input)
            .map_err(|error| format!("{error:?}"))?;
        self.pending_animation_events = compiled_animation_events(&before, &outcome.next_state);
        self.current_state = outcome.next_state;
        Ok(outcome.commands)
    }

    fn apply_transition_commands(&mut self, commands: &[TransitionCommand]) -> Result<(), String> {
        for command in commands {
            match command {
                TransitionCommand::Win => {
                    if let Some(cleared) = self.cleared_levels.get_mut(self.current_level_index) {
                        *cleared = true;
                    }
                }
                TransitionCommand::Restart => self.load_level(self.current_level_index, true)?,
                TransitionCommand::NextLevel => self.advance_level()?,
                TransitionCommand::Again
                | TransitionCommand::Checkpoint
                | TransitionCommand::ClearCheckpoint => {}
            }
        }
        Ok(())
    }

    fn load_level(&mut self, index: usize, clear_history: bool) -> Result<(), String> {
        if index >= compiled_export_levels(&self.export)?.len() {
            return Err(format!("level index out of range: {index}"));
        }
        self.current_level_index = index;
        self.current_state = decode_level_initial_state(&self.engine.game, &self.export, index)?;
        if clear_history {
            self.undo_stack.clear();
            self.redo_stack.clear();
        }
        if self.engine.has_program("level_start") {
            let commands = self.transition_program("level_start", InputId(0))?;
            self.apply_transition_commands(&commands)?;
        } else if self
            .export
            .get("engine")
            .and_then(|engine| engine.get("runRulesOnLevelStart"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let commands = self.transition_program("run_rules_on_level_start", InputId(0))?;
            self.apply_transition_commands(&commands)?;
        }
        if self.engine.has_level_program("level_start_local", index) {
            let commands = self.transition_program("level_start_local", InputId(0))?;
            self.apply_transition_commands(&commands)?;
        }
        Ok(())
    }

    fn advance_level(&mut self) -> Result<(), String> {
        let level_count = compiled_export_levels(&self.export)?.len();
        if self.current_level_index + 1 < level_count {
            self.load_level(self.current_level_index + 1, true)?;
        }
        Ok(())
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.current_state.clone());
            self.current_state = previous;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.current_state.clone());
            self.current_state = next;
        }
    }

    fn input_id_by_name(&self, input_name: &str) -> Option<InputId> {
        self.export
            .get("inputs")
            .and_then(Value::as_array)?
            .iter()
            .find_map(|input| {
                (input.get("name").and_then(Value::as_str) == Some(input_name)).then(|| {
                    input
                        .get("id")
                        .and_then(Value::as_u64)
                        .and_then(|id| u16::try_from(id).ok())
                        .map(InputId)
                })?
            })
    }

    fn snapshot_value(&mut self) -> Value {
        let mut value = self.export.clone();
        let level = compiled_export_levels(&self.export)
            .ok()
            .and_then(|levels| levels.get(self.current_level_index))
            .cloned()
            .unwrap_or(Value::Null);
        set_json_field(
            &mut value,
            "has_progress_save",
            Value::Bool(self.has_progress_save),
        );
        set_json_field(&mut value, "levelIndex", json!(self.current_level_index));
        set_json_field(
            &mut value,
            "selectedLevelIndex",
            json!(self.current_level_index),
        );
        set_json_field(&mut value, "busy", Value::Bool(false));
        set_json_field(
            &mut value,
            "canUndo",
            Value::Bool(!self.undo_stack.is_empty()),
        );
        set_json_field(
            &mut value,
            "canRedo",
            Value::Bool(!self.redo_stack.is_empty()),
        );
        set_json_field(&mut value, "soundEvents", Value::Array(Vec::new()));
        set_json_field(&mut value, "messageEvents", Value::Array(Vec::new()));
        set_json_field(&mut value, "waitEvents", Value::Array(Vec::new()));
        set_json_field(
            &mut value,
            "animationEvents",
            Value::Array(std::mem::take(&mut self.pending_animation_events)),
        );
        set_json_field(&mut value, "gameState", json!({}));
        set_json_field(&mut value, "sceneState", json!({}));
        set_json_field(&mut value, "scenePuzzles", json!([]));
        set_json_field(&mut value, "scenePuzzleState", json!({}));
        set_json_field(&mut value, "currentScene", json!(self.current_scene));
        set_json_field(&mut value, "focusedScreen", json!(self.current_scene));
        set_json_field(&mut value, "focusedScene", json!(self.current_scene));
        set_json_field(&mut value, "visibleScenes", json!([self.current_scene]));
        set_json_field(&mut value, "level", self.level_context_value(&level));
        set_json_field(&mut value, "levels", self.levels_value());
        let scene = self.scene_value(&level);
        let focused_scene = if self.current_scene_has_puzzle() {
            scene.clone()
        } else if self.export_has_scenes() {
            Value::Null
        } else {
            scene.clone()
        };
        set_json_field(&mut value, "scene", focused_scene.clone());
        set_json_field(
            &mut value,
            "sceneLayers",
            json!([{ "name": self.current_scene, "focused": true, "sceneState": {}, "scenePuzzles": [], "scene": focused_scene }]),
        );
        value
    }

    fn export_has_scenes(&self) -> bool {
        self.export
            .get("scenes")
            .and_then(Value::as_array)
            .is_some_and(|scenes| !scenes.is_empty())
    }

    fn current_scene_has_puzzle(&self) -> bool {
        let Some(scene) = self
            .export
            .get("scenes")
            .and_then(Value::as_array)
            .and_then(|scenes| {
                scenes.iter().find(|scene| {
                    scene.get("name").and_then(Value::as_str) == Some(self.current_scene.as_str())
                })
            })
        else {
            return false;
        };
        scene
            .get("components")
            .and_then(Value::as_array)
            .is_some_and(|components| components_contain_puzzle(components))
    }

    fn level_context_value(&self, level: &Value) -> Value {
        json!({
            "index": self.current_level_index,
            "name": level.get("name").and_then(Value::as_str).unwrap_or("level"),
            "pack": level.get("pack").cloned().unwrap_or(Value::Null),
            "puzzle": level.get("puzzle").and_then(Value::as_str).unwrap_or("default"),
            "cleared": self.cleared_levels.get(self.current_level_index).copied().unwrap_or(false),
        })
    }

    fn levels_value(&self) -> Value {
        let levels = compiled_export_levels(&self.export)
            .map(|levels| {
                levels
                    .iter()
                    .enumerate()
                    .map(|(index, level)| {
                        let mut next = level.clone();
                        set_json_field(
                            &mut next,
                            "cleared",
                            Value::Bool(self.cleared_levels.get(index).copied().unwrap_or(false)),
                        );
                        next
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Value::Array(levels)
    }

    fn scene_value(&self, level: &Value) -> Value {
        json!({
            "width": self.current_state.width,
            "height": self.current_state.height,
            "layerCount": self.current_state.layer_count,
            "settings": {
                "grid": { "visibility": 0, "occupied_cells": false, "all_cells": false },
                "animation": self.export.get("animation").cloned().unwrap_or_else(|| json!({ "tween": { "enabled": false, "intervalMs": 250 }})),
            },
            "screen": self.export.get("screen").cloned().unwrap_or_else(|| json!({})),
            "resources": Value::Null,
            "regions": level.get("regions").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "cells": self.cells_value(),
        })
    }

    fn cells_value(&self) -> Value {
        let mut cells = Vec::new();
        let objects = self
            .export
            .get("engine")
            .and_then(|engine| engine.get("objects"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for y in 0..self.current_state.height {
            for x in 0..self.current_state.width {
                let mut layers = Vec::new();
                for layer in 0..self.current_state.layer_count {
                    let object = self
                        .current_state
                        .get_layer(x, y, LayerId(layer))
                        .unwrap_or(ObjectId::EMPTY);
                    if object.is_empty() {
                        continue;
                    }
                    let object_def = objects.iter().find(|candidate| {
                        candidate.get("id").and_then(Value::as_u64) == Some(u64::from(object.0))
                    });
                    let object_name = object_def
                        .and_then(|candidate| candidate.get("name").and_then(Value::as_str))
                        .unwrap_or("unknown");
                    let sprite = object_def
                        .and_then(|candidate| candidate.get("sprite").and_then(Value::as_str))
                        .unwrap_or("unknown");
                    layers.push(json!({
                        "layer": layer,
                        "objectId": object.0,
                        "object": object_name,
                        "sprite": sprite,
                    }));
                }
                cells.push(json!({ "x": x, "y": y, "layers": layers }));
            }
        }
        Value::Array(cells)
    }
}

fn set_json_field(value: &mut Value, key: &str, field: Value) {
    if let Value::Object(map) = value {
        map.insert(key.to_string(), field);
    }
}

fn initial_export_scene(export: &Value) -> String {
    export
        .get("scenes")
        .and_then(Value::as_array)
        .and_then(|scenes| scenes.first())
        .and_then(|scene| scene.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("playing")
        .to_string()
}

fn components_contain_puzzle(components: &[Value]) -> bool {
    components.iter().any(component_contains_puzzle)
}

fn component_contains_puzzle(component: &Value) -> bool {
    matches!(
        component.get("kind").and_then(Value::as_str),
        Some("puzzle" | "frame")
    ) || component
        .get("children")
        .and_then(Value::as_array)
        .is_some_and(|components| components_contain_puzzle(components))
        || component
            .get("elseChildren")
            .and_then(Value::as_array)
            .is_some_and(|components| components_contain_puzzle(components))
}

struct CompiledEngine {
    game: CompiledGame,
    level_start_program: Vec<RuleStep>,
    level_clear_program: Vec<RuleStep>,
    display_level_start_program: Vec<RuleStep>,
    display_level_clear_program: Vec<RuleStep>,
    display_program: Vec<RuleStep>,
    level_start_programs: Vec<Vec<RuleStep>>,
    level_clear_programs: Vec<Vec<RuleStep>>,
}

impl CompiledEngine {
    fn program(&self, key: &str, level_index: usize) -> Option<&[RuleStep]> {
        match key {
            "main" | "run_rules_on_level_start" => Some(self.game.program()),
            "level_start" => Some(&self.level_start_program),
            "level_clear" => Some(&self.level_clear_program),
            "display_level_start" => Some(&self.display_level_start_program),
            "display_level_clear" => Some(&self.display_level_clear_program),
            "display" => Some(&self.display_program),
            "level_start_local" => self
                .level_start_programs
                .get(level_index)
                .map(Vec::as_slice),
            "level_clear_local" => self
                .level_clear_programs
                .get(level_index)
                .map(Vec::as_slice),
            _ => None,
        }
    }

    fn has_program(&self, key: &str) -> bool {
        self.program(key, 0)
            .is_some_and(|program| !program.is_empty())
    }

    fn has_level_program(&self, key: &str, level_index: usize) -> bool {
        self.program(key, level_index)
            .is_some_and(|program| !program.is_empty())
    }
}

fn decode_export_engine(export: &Value) -> Result<CompiledEngine, String> {
    let compiled = export
        .get("compiledPlay")
        .ok_or_else(|| "compiled play export is missing compiledPlay".to_string())?;
    decode_compiled_play(compiled)
}

fn decode_compiled_play(value: &Value) -> Result<CompiledEngine, String> {
    let model = string_field(value, "model")?;
    if model != "grid2" {
        return Err(format!("unsupported compiled play model: {model}"));
    }
    let data = value_array(
        object_field(value, "transition")?,
        "compiled play transition",
    )?;
    let layer_count = u16_at(data, 0, "transition layer count")?;
    let objects = array_at(data, 1, "transition objects")?
        .iter()
        .map(decode_compact_object)
        .collect::<Result<Vec<_>, _>>()?;
    let queries = array_at(data, 2, "transition queries")?
        .iter()
        .map(decode_compact_condition)
        .collect::<Result<Vec<_>, _>>()?;
    let visual_objects = array_at(data, 3, "transition visual objects")?
        .iter()
        .map(|item| Ok(ObjectId(u16_value(item, "visual object")?)))
        .collect::<Result<Vec<_>, String>>()?;
    let programs = array_at(data, 4, "transition programs")?;
    let program = decode_compact_program(value_at(programs, 0, "main program")?)?;
    let game = CompiledGame::new_with_scratch_condition_defs_program_roles(
        layer_count,
        objects,
        Vec::new(),
        queries,
        program,
        visual_objects,
        Vec::new(),
    );
    let level_programs = array_at(data, 5, "transition level programs")?;
    let mut level_start_programs = Vec::with_capacity(level_programs.len());
    let mut level_clear_programs = Vec::with_capacity(level_programs.len());
    for (index, entry) in level_programs.iter().enumerate() {
        let entry = value_array(entry, &format!("level program {index}"))?;
        level_start_programs.push(decode_compact_program(value_at(
            entry,
            0,
            "level start local program",
        )?)?);
        level_clear_programs.push(decode_compact_program(value_at(
            entry,
            1,
            "level clear local program",
        )?)?);
    }
    Ok(CompiledEngine {
        game,
        level_start_program: decode_compact_program(value_at(programs, 1, "level start program")?)?,
        level_clear_program: decode_compact_program(value_at(programs, 2, "level clear program")?)?,
        display_level_start_program: decode_compact_program(value_at(
            programs,
            3,
            "display level start program",
        )?)?,
        display_level_clear_program: decode_compact_program(value_at(
            programs,
            4,
            "display level clear program",
        )?)?,
        display_program: decode_compact_program(value_at(programs, 5, "display program")?)?,
        level_start_programs,
        level_clear_programs,
    })
}

fn decode_compact_object(value: &Value) -> Result<ObjectDef, String> {
    let items = value_array(value, "compact object")?;
    Ok(ObjectDef {
        id: ObjectId(u16_at(items, 0, "object id")?),
        layer_id: LayerId(u16_at(items, 1, "object layer")?),
    })
}

fn decode_compact_condition(value: &Value) -> Result<ConditionDef, String> {
    let items = value_array(value, "compact condition")?;
    Ok(ConditionDef {
        id: ConditionId(u16_at(items, 0, "condition id")?),
        kind: decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
    })
}

fn decode_compact_program(value: &Value) -> Result<Vec<RuleStep>, String> {
    value_array(value, "compact program")?
        .iter()
        .map(decode_compact_rule_step)
        .collect()
}

fn decode_compact_rule_step(value: &Value) -> Result<RuleStep, String> {
    let items = value_array(value, "compact rule step")?;
    match tag_at(items, 0, "rule step tag")? {
        0 => Ok(RuleStep::Rule(decode_compact_rule(value_at(
            items, 1, "rule",
        )?)?)),
        1 => Ok(RuleStep::ConditionalBlock {
            condition: decode_compact_rule_condition(value_at(items, 1, "condition")?)?,
            steps: decode_compact_program(value_at(items, 2, "steps")?)?,
        }),
        2 => Ok(RuleStep::Block {
            application: decode_compact_application(u16_at(items, 1, "application")?)?,
            stop_condition: match value_at(items, 2, "condition")? {
                Value::Null => None,
                condition => Some(decode_compact_rule_condition(condition)?),
            },
            steps: decode_compact_program(value_at(items, 3, "steps")?)?,
        }),
        3 => Ok(RuleStep::LocalFrame {
            frame: decode_compact_local_frame(value_at(items, 1, "local frame")?)?,
            steps: decode_compact_program(value_at(items, 2, "steps")?)?,
        }),
        4 => Ok(RuleStep::AfterTriggered {
            steps: decode_compact_program(value_at(items, 1, "steps")?)?,
            then_steps: decode_compact_program(value_at(items, 2, "then steps")?)?,
        }),
        5 => Ok(RuleStep::ConditionalBranch {
            condition: decode_compact_rule_condition(value_at(items, 1, "condition")?)?,
            then_steps: decode_compact_program(value_at(items, 2, "then steps")?)?,
            else_steps: decode_compact_program(value_at(items, 3, "else steps")?)?,
        }),
        tag => Err(format!("unknown compact rule step tag: {tag}")),
    }
}

fn decode_compact_rule(value: &Value) -> Result<Rule, String> {
    let items = value_array(value, "compact rule")?;
    Ok(Rule {
        id: RuleId(u16_at(items, 0, "rule id")?),
        application: decode_compact_application(u16_at(items, 1, "application")?)?,
        guards: array_at(items, 2, "guards")?
            .iter()
            .map(decode_compact_guard)
            .collect::<Result<Vec<_>, _>>()?,
        pattern: decode_compact_pattern(value_at(items, 3, "pattern")?)?,
        writes: array_at(items, 4, "writes")?
            .iter()
            .map(decode_compact_write)
            .collect::<Result<Vec<_>, _>>()?,
        effects: array_at(items, 5, "effects")?
            .iter()
            .map(decode_compact_effect)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn decode_compact_application(value: u16) -> Result<RuleApplication, String> {
    match value {
        0 => Ok(RuleApplication::Once),
        1 => Ok(RuleApplication::OnceAll),
        2 => Ok(RuleApplication::OncePerLevel),
        3 => Ok(RuleApplication::UntilStable),
        other => Err(format!("unknown compact rule application: {other}")),
    }
}

fn decode_compact_rule_condition(value: &Value) -> Result<RuleCondition, String> {
    let items = value_array(value, "compact rule condition")?;
    match tag_at(items, 0, "condition tag")? {
        0 => Ok(RuleCondition::AnyMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        1 => Ok(RuleCondition::NoMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        2 => Ok(RuleCondition::AnyInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        3 => Ok(RuleCondition::NoInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        4 => Ok(RuleCondition::GuardBranches(
            value_array(value_at(items, 1, "guard branches")?, "guard branches")?
                .iter()
                .map(|branch| {
                    value_array(branch, "guard branch")?
                        .iter()
                        .map(decode_compact_guard)
                        .collect()
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        tag => Err(format!("unknown compact condition tag: {tag}")),
    }
}

fn decode_compact_guard(value: &Value) -> Result<Guard, String> {
    let items = value_array(value, "compact guard")?;
    match tag_at(items, 0, "guard tag")? {
        0 => Ok(Guard::InputIs(InputId(u16_at(items, 1, "input")?))),
        1 => Ok(Guard::GlobalCompare {
            global: GlobalId(u16_at(items, 1, "global")?),
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        2 => Ok(Guard::ConditionCompare {
            condition: ConditionId(u16_at(items, 1, "condition")?),
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        3 => Ok(Guard::ConditionNonZero(ConditionId(u16_at(
            items,
            1,
            "condition",
        )?))),
        4 => Ok(Guard::InlineConditionCompare {
            kind: decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        5 => Ok(Guard::InlineConditionNonZero(
            decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
        )),
        tag => Err(format!("unknown compact guard tag: {tag}")),
    }
}

fn decode_compact_condition_value_kind(value: &Value) -> Result<ConditionValueKind, String> {
    let items = value_array(value, "compact condition kind")?;
    match tag_at(items, 0, "condition kind tag")? {
        0 => Ok(ConditionValueKind::CountObjects(decode_compact_object_ids(
            value_at(items, 1, "objects")?,
        )?)),
        1 => Ok(ConditionValueKind::ExistsObjects(
            decode_compact_object_ids(value_at(items, 1, "objects")?)?,
        )),
        2 => Ok(ConditionValueKind::NoneObjects(decode_compact_object_ids(
            value_at(items, 1, "objects")?,
        )?)),
        3 => Ok(ConditionValueKind::CountMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        4 => Ok(ConditionValueKind::ExistsMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        5 => Ok(ConditionValueKind::NoneMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        6 => Ok(ConditionValueKind::CountInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        7 => Ok(ConditionValueKind::ExistsInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        8 => Ok(ConditionValueKind::NoneInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        tag => Err(format!("unknown compact condition kind tag: {tag}")),
    }
}

fn decode_compact_patterns(value: &Value) -> Result<Vec<Pattern>, String> {
    value_array(value, "compact patterns")?
        .iter()
        .map(decode_compact_pattern)
        .collect()
}

fn decode_compact_input_patterns(value: &Value) -> Result<Vec<(InputId, Pattern)>, String> {
    value_array(value, "input patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "input pattern")?;
            Ok((
                InputId(u16_at(entry, 0, "input")?),
                decode_compact_pattern(value_at(entry, 1, "pattern")?)?,
            ))
        })
        .collect()
}

fn decode_compact_pattern(value: &Value) -> Result<Pattern, String> {
    Ok(Pattern {
        components: value_array(value, "compact pattern")?
            .iter()
            .map(|component| {
                let component = value_array(component, "pattern component")?;
                Ok(PatternComponent {
                    gap_count: u16_at(component, 0, "gap count")?,
                    cells: value_array(value_at(component, 1, "cells")?, "cells")?
                        .iter()
                        .map(decode_compact_match_cell)
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn decode_compact_match_cell(value: &Value) -> Result<MatchCell, String> {
    let items = value_array(value, "match cell")?;
    Ok(MatchCell {
        offset: decode_compact_offset(value_at(items, 0, "offset")?)?,
        require_objects: decode_compact_object_ids(value_at(items, 1, "require objects")?)?,
        require_object_sets: if items.len() >= 8 {
            decode_compact_object_sets(value_at(items, 2, "require object sets")?)?
        } else {
            Vec::new()
        },
        forbid_objects: decode_compact_object_ids(value_at(
            items,
            if items.len() >= 8 { 3 } else { 2 },
            "forbid objects",
        )?)?,
        require_scratch: decode_compact_scratch_patterns(value_at(
            items,
            if items.len() >= 8 { 4 } else { 3 },
            "require scratch",
        )?)?,
        require_object_set_scratch: if items.len() >= 8 {
            decode_compact_object_set_scratch_patterns(value_at(
                items,
                5,
                "require object set scratch",
            )?)?
        } else {
            Vec::new()
        },
        forbid_scratch: decode_compact_scratch_patterns(value_at(
            items,
            if items.len() >= 8 { 6 } else { 4 },
            "forbid scratch",
        )?)?,
        forbid_object_set_scratch: if items.len() >= 8 {
            decode_compact_object_set_scratch_patterns(value_at(
                items,
                7,
                "forbid object set scratch",
            )?)?
        } else {
            Vec::new()
        },
    })
}

fn decode_compact_offset(value: &Value) -> Result<Offset, String> {
    let items = value_array(value, "offset")?;
    match tag_at(items, 0, "offset tag")? {
        0 => Ok(Offset::Fixed {
            dx: i16_at(items, 1, "dx")?,
            dy: i16_at(items, 2, "dy")?,
        }),
        1 => Ok(Offset::Variable {
            base_dx: i16_at(items, 1, "base dx")?,
            base_dy: i16_at(items, 2, "base dy")?,
            gap_terms: value_array(value_at(items, 3, "gap terms")?, "gap terms")?
                .iter()
                .map(|term| {
                    let term = value_array(term, "gap term")?;
                    Ok(GapTerm {
                        gap_index: u16_at(term, 0, "gap index")?,
                        dx: i16_at(term, 1, "dx")?,
                        dy: i16_at(term, 2, "dy")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        }),
        tag => Err(format!("unknown compact offset tag: {tag}")),
    }
}

fn decode_compact_write(value: &Value) -> Result<WriteOp, String> {
    let items = value_array(value, "write")?;
    match tag_at(items, 0, "write tag")? {
        0 => Ok(WriteOp::Add {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
        }),
        1 => Ok(WriteOp::Remove {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
        }),
        2 => Ok(WriteOp::Move {
            component: u16_at(items, 1, "component")?,
            from_offset: decode_compact_offset(value_at(items, 2, "from offset")?)?,
            to_offset: decode_compact_offset(value_at(items, 3, "to offset")?)?,
            object: ObjectId(u16_at(items, 4, "object")?),
        }),
        3 => Ok(WriteOp::Replace {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            remove: ObjectId(u16_at(items, 3, "remove")?),
            add: ObjectId(u16_at(items, 4, "add")?),
        }),
        4 => Ok(WriteOp::SetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
        }),
        5 => Ok(WriteOp::RemoveScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
            match_value: decode_compact_scratch_match(u16_at(items, 6, "scratch match")?)?,
        }),
        6 => Ok(WriteOp::AddObjectSet {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
        }),
        7 => Ok(WriteOp::RemoveObjectSet {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
        }),
        8 => Ok(WriteOp::MoveObjectSet {
            component: u16_at(items, 1, "component")?,
            from_offset: decode_compact_offset(value_at(items, 2, "from offset")?)?,
            to_offset: decode_compact_offset(value_at(items, 3, "to offset")?)?,
            binding: u16_at(items, 4, "binding")?,
        }),
        9 => Ok(WriteOp::SetObjectSetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
        }),
        10 => Ok(WriteOp::RemoveObjectSetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
            match_value: decode_compact_scratch_match(u16_at(items, 6, "scratch match")?)?,
        }),
        tag => Err(format!("unknown compact write tag: {tag}")),
    }
}

fn decode_compact_effect(value: &Value) -> Result<Effect, String> {
    let items = value_array(value, "effect")?;
    match tag_at(items, 0, "effect tag")? {
        0 => Ok(Effect::Cancel),
        1 => Ok(Effect::Win),
        2 => Ok(Effect::Restart),
        3 => Ok(Effect::NextLevel),
        4 => Ok(Effect::Again),
        5 => Ok(Effect::Checkpoint),
        6 => Ok(Effect::ClearCheckpoint),
        7 => Ok(Effect::UpdateGlobal {
            global: GlobalId(u16_at(items, 1, "global")?),
            op: decode_compact_global_update(u16_at(items, 2, "global update")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        tag => Err(format!("unknown compact effect tag: {tag}")),
    }
}

fn decode_compact_scratch_patterns(value: &Value) -> Result<Vec<ScratchPattern>, String> {
    value_array(value, "scratch patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "scratch pattern")?;
            Ok(ScratchPattern {
                object: ObjectId(u16_at(entry, 0, "object")?),
                scratch: ScratchId(u16_at(entry, 1, "scratch")?),
                value: optional_i64_at(entry, 2, "value")?,
                match_value: decode_compact_scratch_match(u16_at(entry, 3, "scratch match")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_sets(value: &Value) -> Result<Vec<ObjectSetMatcher>, String> {
    value_array(value, "object sets")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "object set")?;
            Ok(ObjectSetMatcher {
                binding: u16_at(entry, 0, "binding")?,
                layer: LayerId(u16_at(entry, 1, "layer")?),
                objects: decode_compact_object_ids(value_at(entry, 2, "objects")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_set_scratch_patterns(
    value: &Value,
) -> Result<Vec<ObjectSetScratchPattern>, String> {
    value_array(value, "object set scratch patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "object set scratch pattern")?;
            Ok(ObjectSetScratchPattern {
                binding: u16_at(entry, 0, "binding")?,
                scratch: ScratchId(u16_at(entry, 1, "scratch")?),
                value: optional_i64_at(entry, 2, "value")?,
                match_value: decode_compact_scratch_match(u16_at(entry, 3, "scratch match")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_ids(value: &Value) -> Result<Vec<ObjectId>, String> {
    value_array(value, "object ids")?
        .iter()
        .map(|item| Ok(ObjectId(u16_value(item, "object id")?)))
        .collect()
}

fn decode_compact_local_frame(value: &Value) -> Result<LocalFrame<ObjectId>, String> {
    let items = value_array(value, "local frame")?;
    Ok(LocalFrame {
        x: decode_compact_local_frame_extent(value_at(items, 0, "frame x")?)?,
        y: decode_compact_local_frame_extent(value_at(items, 1, "frame y")?)?,
        z: decode_compact_local_frame_extent(value_at(items, 2, "frame z")?)?,
        focus_objects: decode_compact_object_ids(value_at(items, 3, "focus objects")?)?,
    })
}

fn decode_compact_local_frame_extent(value: &Value) -> Result<LocalFrameExtent, String> {
    if value.is_null() {
        return Ok(LocalFrameExtent::Full);
    }
    Ok(LocalFrameExtent::Radius(u16_value(value, "frame extent")?))
}

fn decode_compact_comparison(value: u16) -> Result<ComparisonOp, String> {
    match value {
        0 => Ok(ComparisonOp::Eq),
        1 => Ok(ComparisonOp::NotEq),
        2 => Ok(ComparisonOp::Greater),
        3 => Ok(ComparisonOp::GreaterEq),
        4 => Ok(ComparisonOp::Less),
        5 => Ok(ComparisonOp::LessEq),
        other => Err(format!("unknown compact comparison op: {other}")),
    }
}

fn decode_compact_global_update(value: u16) -> Result<GlobalUpdateOp, String> {
    match value {
        0 => Ok(GlobalUpdateOp::Set),
        1 => Ok(GlobalUpdateOp::Add),
        2 => Ok(GlobalUpdateOp::Subtract),
        3 => Ok(GlobalUpdateOp::Multiply),
        4 => Ok(GlobalUpdateOp::Divide),
        5 => Ok(GlobalUpdateOp::Remainder),
        other => Err(format!("unknown compact global update op: {other}")),
    }
}

fn decode_compact_scratch_match(value: u16) -> Result<ScratchValueMatch, String> {
    match value {
        0 => Ok(ScratchValueMatch::Any),
        1 => Ok(ScratchValueMatch::Exact),
        other => Err(format!("unknown compact scratch match: {other}")),
    }
}

fn compiled_export_levels(export: &Value) -> Result<&[Value], String> {
    array_field(export, "levels")
}

fn decode_level_initial_state(
    game: &CompiledGame,
    export: &Value,
    level_index: usize,
) -> Result<PuzzleState, String> {
    let levels = compiled_export_levels(export)?;
    let level = levels
        .get(level_index)
        .ok_or_else(|| format!("level index out of range: {level_index}"))?;
    decode_state_value(
        game,
        level
            .get("initialState")
            .ok_or_else(|| "level is missing initialState".to_string())?,
    )
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

fn compiled_animation_events(_before: &PuzzleState, _after: &PuzzleState) -> Vec<Value> {
    Vec::new()
}

fn object_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn value_array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{name} must be an array"))
}

fn value_at<'a>(items: &'a [Value], index: usize, name: &str) -> Result<&'a Value, String> {
    items
        .get(index)
        .ok_or_else(|| format!("missing {name} at index {index}"))
}

fn array_at<'a>(items: &'a [Value], index: usize, name: &str) -> Result<&'a [Value], String> {
    value_array(value_at(items, index, name)?, name)
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn optional_i64_at(items: &[Value], index: usize, name: &str) -> Result<Option<i64>, String> {
    let value = value_at(items, index, name)?;
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{name} must be an integer or null"))
}

fn tag_at(items: &[Value], index: usize, name: &str) -> Result<u16, String> {
    u16_at(items, index, name)
}

fn u16_at(items: &[Value], index: usize, name: &str) -> Result<u16, String> {
    u16_value(value_at(items, index, name)?, name)
}

fn i16_at(items: &[Value], index: usize, name: &str) -> Result<i16, String> {
    let raw = i64_at(items, index, name)?;
    i16::try_from(raw).map_err(|_| format!("{name} out of range"))
}

fn i64_at(items: &[Value], index: usize, name: &str) -> Result<i64, String> {
    value_at(items, index, name)?
        .as_i64()
        .ok_or_else(|| format!("{name} must be an integer"))
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
    json!({
        "sfx": loaded.sounds.sfx.iter().map(|sfx| {
            json!({"name": sfx.name, "seed": sfx.seed, "type": sfx.type_target})
        }).collect::<Vec<_>>(),
        "music": loaded.sounds.music.iter().map(|music| {
            json!({
                "name": music.name,
                "seed": music.seed,
                "height": music.height,
                "bars": music.bars,
                "bpm": music.bpm,
                "volume": music.volume,
            })
        }).collect::<Vec<_>>(),
    })
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

    #[test]
    fn standalone_session_from_export_uses_game_session_wait_continuation() {
        let source = r#"
title export_wait

puzzle board {
layers {
actor = Player
}
input clear
rules {
if input == clear -> win
}
on_level_clear {
wait 1s
next_level
}
levels {
legend {
. = empty
P = Player
}
P

P
}
}
"#;
        let export = json!({
            "source": source,
            "puzzlePath": "games/export_wait.puzzle"
        });
        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string())
            .expect("export should initialize from embedded source");

        let state: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/input/clear")
                .expect("clear input should run"),
        )
        .expect("snapshot json");

        assert_eq!(state["levelIndex"], json!(0));
        assert_eq!(
            state["waitEvents"],
            json!([{ "kind": "continue_effects", "milliseconds": 1000 }])
        );

        let state: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/__continue_effects")
                .expect("continuation should run"),
        )
        .expect("snapshot json");

        assert_eq!(state["levelIndex"], json!(1));
    }

    #[test]
    fn standalone_session_state_exports_sfx_type_for_browser_runtime() {
        let source = r#"
title sound_export

sounds {
sfx push seed=push type=hit
}

puzzle board {
layers {
actor = Player
}
input clear
rules {
if input == clear -> win
}
levels {
legend {
. = empty
P = Player
}
P
}
}
"#;
        let export = json!({
            "source": source,
            "puzzlePath": "games/sound_export.puzzle"
        });
        let mut bridge = StandaloneSessionBridge::from_export_json(&export.to_string())
            .expect("export should initialize from embedded source");

        let state: Value =
            serde_json::from_str(&bridge.request_json("GET", "/api/state").unwrap()).unwrap();
        let sfx = &state["sounds"]["sfx"][0];
        assert_eq!(sfx["name"], json!("push"));
        assert_eq!(sfx["seed"], json!("push"));
        assert_eq!(sfx["type"], json!("hit"));
        assert!(sfx.get("typeTarget").is_none());
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
