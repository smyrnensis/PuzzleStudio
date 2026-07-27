#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

#[cfg(not(target_arch = "wasm32"))]
use puzzle_assets::EncodedVisualImageAsset;
use puzzle_assets::EncodedVisualImageBundle;
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionValueKind, GridSize, InputId, MarkPattern, MarkValueMatch,
    ObjectId, Offset, Pattern, State,
};
pub use puzzle_game_runtime::RuntimeSession;
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::resolve_game_entry;
use puzzle_lang::DiagnosticReport;
use puzzle_lang::{
    GoalCondition, GoalExpr, GoalValue, Level, LoadedDocumentModel, LoadedGame, RuleAnimation,
    RuleAnimationTrigger, SceneValue,
};
use puzzle_play::scene_value_to_string;
use puzzle_runtime_contract::{
    RuntimeStateSnapshot2d, StandaloneProgressStorage, StandaloneRuntimeExport,
};

#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_PLAYER_WASM_JS: &str = include_str!("../static/wasm_player/puzzle_wasm_player.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_PLAYER_WASM_BG: &[u8] =
    include_bytes!("../static/wasm_player/puzzle_wasm_player_bg.wasm");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_JS: &str = include_str!("../static/wasm_game/puzzle_wasm_game.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_BG: &[u8] = include_bytes!("../static/wasm_game/puzzle_wasm_game_bg.wasm");

include!("lib_cli.rs");
include!("lib_screenshot.rs");
include!("lib_assets.rs");
include!("lib_editor_preview_state.rs");
include!("lib_export.rs");
include!("lib_json_export.rs");
include!("lib_server.rs");

#[cfg(all(test, not(target_arch = "wasm32")))]
include!("lib_tests.rs");

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Io(error) => write!(f, "{error}"),
            Self::Lang(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}
