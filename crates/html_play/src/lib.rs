#![cfg_attr(any(target_arch = "wasm32", not(feature = "solver")), allow(dead_code))]

#[cfg(feature = "solver")]
use std::collections::{BTreeSet, HashMap};
#[cfg(not(target_arch = "wasm32"))]
use std::env;
use std::fmt::Write as FmtWrite;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
#[cfg(any(not(target_arch = "wasm32"), feature = "solver"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(any(not(target_arch = "wasm32"), feature = "solver"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

#[cfg(feature = "solver")]
use puzzle_core::transition_state;
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionValueKind, Effect, Guard, InputId, LayerId, MarkPattern,
    MarkValueMatch, ObjectId, Offset, Patch, PatchOp, Pattern, Rule, RuleApplication,
    RuleCondition, RuleId, RuleStep, State, TransitionCommand, TransitionError, VariableUpdateOp,
    WriteOp, transition_program, transition_program_outcome, transition_program_trace,
};
#[cfg(feature = "solver")]
use puzzle_core::{ConditionId, MarkId, MatchCell, PatternComponent, VariableId};
use puzzle_grid3d::{
    Coord3, Game3, ObjectId as ObjectId3, RuleId3, Size3, State3,
    transition_program_without_input_with_local_frame,
};
#[cfg(feature = "solver")]
use puzzle_grid3d::{
    Rule3, WinCondition3, eval_condition_kind, transition_program as transition_program3,
};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::AssetsDef;
#[cfg(feature = "solver")]
use puzzle_lang::GoalClause;
use puzzle_lang::ParsedPuzzle3;
use puzzle_lang::{
    ArrowKey, GoalCondition, GoalExpr, GoalValue, KeyTrigger, Level, LoadedDocumentModel,
    LoadedGame, ResourceSelection, RuleAnimation, RuleAnimationTrigger, RuleEffect, SceneComponent,
    SceneEffect, SceneExpr, SceneLayoutDef, ScenePuzzleInitializer, SceneTextContent,
    SceneTransitionTrigger, SceneValue, SoundsDef, ThemeDef, VisualSpriteDef, VisualSpriteKind,
    VisualSpriteTransform, parse_game2d as parse_game,
};
use puzzle_lang::{AssetKind, DiagnosticReport};
#[cfg(feature = "solver")]
use puzzle_lang::{
    QueryExpr, QueryExpr3, QueryExprOf, SolverStrategy, SolverStrategy3, SolverStrategyDirection,
};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::{discover_game_entries, expand_game_imports_for_file, resolve_game_entry};
use puzzle_play::{
    AnimationEvent, GameSession, LevelProgressSaveData, MessageEvent, PersistentVarSaveData,
    ProgressSaveData, SoundEvent, WaitEvent, animation_events_contract_2d,
    animation_events_for_trace, loaded_document_scene_host_loaded_game, runtime_sounds_def,
};
use puzzle_runtime_contract::{
    LifecycleCommand, RuntimeChangedCell, RuntimeCoord, RuntimeMarkValue, RuntimeMarkValueMatch,
    RuntimeModelKind, RuntimePatchOp, RuntimeStateSnapshot, RuntimeStateSnapshot2d,
    RuntimeTransitionCommand, RuntimeTransitionCurrentOutcome, RuntimeTransitionProgramOutcome,
};
#[cfg(feature = "solver")]
use puzzle_solver::{
    Puzzle3Domain, PuzzleDomain, ScanControl, ScanOutcome, SearchBudget, SearchOutcome,
    SearchProgress, SearchStats, best_first_scan_with_dead_states_and_progress,
    best_first_with_dead_states_and_progress,
};

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_CSS: &str = include_str!("../static/app.css");
const THEME_PRESETS_CSS: &str = include_str!("../static/theme_presets.css");
const RENDERER_CSS: &str = include_str!("../static/renderer.css");
const VISUALS_JS: &str = include_str!("../static/visuals.js");
const APP_JS: &str = include_str!("../static/app.js");
const RENDERER_JS: &str = include_str!("../static/renderer.js");
const STANDALONE_JS: &str = include_str!("../static/standalone.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_JS: &str = include_str!("../static/wasm_game/puzzle_wasm_game.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_BG: &[u8] = include_bytes!("../static/wasm_game/puzzle_wasm_game_bg.wasm");
const PUZZLE3_STYLE_CSS: &str = include_str!("../static/puzzle3.css");
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
const PUZZLE3_THREE_RENDERER_JS: &str = include_str!("../static/puzzle3_three_renderer.js");
const PUZZLE3_APP_JS: &str = include_str!("../static/puzzle3_app.js");
const THREE_MODULE_JS: &str = include_str!("../static/vendor/three/three.module.min.js");
const SEEDED_SFX_JS: &str = include_str!("../../../tools/music_generator/seeded_sfx.mjs");
const SEEDED_MUSIC_JS: &str = include_str!("../../../tools/music_generator/seeded_music.mjs");
const SEEDED_MUSIC_PLAYER_JS: &str =
    include_str!("../../../tools/music_generator/seeded_music_player.mjs");
const SEEDED_TIMBRE_FIELDS_JS: &str =
    include_str!("../../../tools/music_generator/seeded_timbre_fields.mjs");

include!("lib_cli.rs");
include!("lib_screenshot.rs");
include!("lib_assets.rs");
include!("lib_solver_runtime.rs");
include!("lib_export.rs");
include!("lib_runtime_bridge.rs");
include!("lib_json_export.rs");
include!("lib_server.rs");

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const RUNTIME_CURRENT_OUTCOME_COMMON_KEYS: &[&str] = &[
        "cancelled",
        "changed",
        "completed",
        "commands",
        "firedRules",
        "patches",
        "stateHash",
        "stateHashKey",
        "changedCells",
        "animationEvents",
        "variables",
        "levelFiredRules",
        "previousStateHandle",
    ];

    fn parse_json_object(source: &str) -> Value {
        serde_json::from_str(source).expect("runtime outcome should be valid JSON")
    }

    fn assert_has_object_keys(value: &Value, keys: &[&str]) {
        let object = value.as_object().expect("value should be a JSON object");
        for key in keys {
            assert!(object.contains_key(*key), "missing JSON key {key}");
        }
    }

    fn cell_has_object(cell: &Value, object: &str) -> bool {
        cell.get("layers")
            .and_then(Value::as_array)
            .is_some_and(|layers| {
                layers
                    .iter()
                    .any(|layer| layer.get("object").and_then(Value::as_str) == Some(object))
            })
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_strategy_score_reads_queries_variables_and_distance() {
        let source = r#"
title = solver_strategy_score

puzzle default {
layers {
floor = Goal
actor = Player Box
}

var pressure = 2
query boxes_on_goal = count([ Box Goal ])
query player_to_goal = distance(Player, Goal)
query near = distance(Player, Goal) <= 3

solver {
strategy {
maximize boxes_on_goal weight 50
minimize pressure weight 2
prefer near weight 10
minimize player_to_goal weight 3
}
}

rules {
}

levels tiny of default {
legend {
. = empty
P = Player
* = Box Goal
}

level "start" {
P*
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let state = &loaded.levels[0].initial_state;

        assert_eq!(solver_strategy_score(&loaded, state), -43);
    }

    fn embedded_puzzle_runtime_export_json(html: &str) -> Value {
        embedded_puzzle_json_assignment(
            html,
            "window.PuzzleRuntimeExportJson = \"",
            "\";",
            "PuzzleRuntimeExportJson",
        )
    }

    fn prepared_editor_solver_rules_json(source: &str, path: &str) -> Value {
        serde_json::from_str(
            &export_solver_rules_json_from_source(source, path).expect("prepared solver rules"),
        )
        .expect("prepared solver rules json")
    }

    fn embedded_puzzle_boot_json(html: &str) -> Value {
        embedded_puzzle_json_assignment(
            html,
            "window.PuzzleBoot = JSON.parse(\"",
            "\");",
            "PuzzleBoot",
        )
    }

    fn embedded_puzzle3_frame_fixture_json(html: &str) -> Value {
        embedded_puzzle_json_assignment(
            html,
            "window.Puzzle3DFrameFixture = JSON.parse(\"",
            "\");",
            "Puzzle3DFrameFixture",
        )
    }

    fn embedded_puzzle_json_assignment(
        html: &str,
        marker: &str,
        terminator: &str,
        name: &str,
    ) -> Value {
        let start = html
            .find(marker)
            .unwrap_or_else(|| panic!("html should embed raw {name} JSON"))
            + marker.len();
        let rest = &html[start..];
        let end = rest
            .find(terminator)
            .unwrap_or_else(|| panic!("{name} assignment should close"));
        let encoded = &rest[..end];
        let json_text: String = serde_json::from_str(&format!("\"{encoded}\""))
            .unwrap_or_else(|_| panic!("{name} should be a JSON string literal"));
        serde_json::from_str(&json_text).unwrap_or_else(|_| panic!("{name} should contain JSON"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn standalone_export_embeds_only_manifest_file_assets() {
        let dir = std::env::temp_dir().join(format!(
            "puzzle_assets_manifest_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("sprites")).expect("create asset fixture directory");
        std::fs::write(
            dir.join("sprites/player.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="#f00"/></svg>"##,
        )
        .expect("write declared asset");
        std::fs::write(dir.join("secret.pdf"), b"not declared").expect("write undeclared asset");

        let source = r#"
title = Manifest Assets

assets {
file "sprites/player.svg"
}

puzzle default {
layers {
actor = Player
}
rules {
}
}

levels {
legend {
. = empty
P = Player
}
level "one"
P
}

scene playing {
layout {
default
}
rules {
step default
}
}
"#;

        let game_path = dir.join("game.puzzle");
        std::fs::write(&game_path, source).expect("write game source");
        let html = export_html_file(&game_path).expect("export with manifest asset");

        assert!(html.contains("\"sprites/player.svg\":\"data:image/svg+xml;charset=utf-8,"));
        assert!(!html.contains("secret.pdf"));
        assert!(!html.contains("not declared"));
        assert!(html.contains("Puzzle asset is not embedded"));
    }

    #[test]
    fn stateful_core_runtime_exposes_changed_cells_for_2d() {
        let source = r#"
puzzle board {
  render {
    tween
  }
  layers {
    actor = Player
  }
  empty .
  rules {
    once [ Player | no Player ] -> [ | Player ]
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P.
  }
}
"#;
        let mut runtime = CoreRuntimeBridge::from_source(source).expect("load 2D runtime");
        let mut state_json = String::new();
        push_state_data(&mut state_json, &runtime.loaded.levels[0].initial_state);
        runtime
            .set_state_json(&state_json)
            .expect("set current state");
        let saved = runtime.save_current_state().expect("save current state");

        let outcome = runtime
            .transition_current_outcome_json("main", -1, 4)
            .expect("transition current state");
        let outcome_json = parse_json_object(&outcome);
        let outcome_contract: RuntimeTransitionCurrentOutcome =
            serde_json::from_str(&outcome).expect("2D current outcome should match contract");

        assert_has_object_keys(&outcome_json, RUNTIME_CURRENT_OUTCOME_COMMON_KEYS);
        assert!(!outcome_contract.completed);
        assert_eq!(
            outcome_json["changedCells"],
            json!([
                { "position": { "x": 0, "y": 0 }, "objects": [] },
                { "position": { "x": 1, "y": 0 }, "objects": [1] }
            ])
        );
        assert_eq!(
            outcome_json["animationEvents"],
            json!([
                {
                    "kind": "move",
                    "name": "tween",
                    "objectId": 1,
                    "from": { "x": 0, "y": 0 },
                    "to": { "x": 1, "y": 0 }
                }
            ])
        );
        assert!(outcome_json.get("state").is_none());
        assert!(outcome_json["previousStateHandle"].is_u64());
        assert_eq!(outcome_json["variables"], json!([]));
        assert!(outcome_json["levelFiredRules"].is_array());
        runtime
            .restore_saved_state(saved)
            .expect("restore saved current state");
        assert_eq!(runtime.current_state_json().unwrap(), state_json);

        let state_outcome = runtime
            .transition_current_state_outcome_json("main", -1, 4)
            .expect("transition current state with state payload");
        let state_outcome_json = parse_json_object(&state_outcome);
        assert_eq!(state_outcome_json["state"]["width"], 2);
        assert_eq!(state_outcome_json["state"]["height"], 1);
        assert_eq!(
            state_outcome_json["changedCells"],
            outcome_json["changedCells"]
        );
        assert_eq!(
            state_outcome_json["animationEvents"],
            outcome_json["animationEvents"]
        );
    }

    #[test]
    fn stateful_puzzle3_runtime_exposes_changed_cells_without_state_payload() {
        let source = r#"
puzzle3 board {
  layers {
    actor = Player
  }
  rules {
    horizontal [ Player | no Player ] -> [ | Player ]
  }
}

levels3 default of board {
  legend {
    . = empty
    P = Player
  }
  level "one" {
    P.
  }
}
"#;
        let mut runtime = Puzzle3RuntimeBridge::from_source(source).expect("load 3D runtime");
        let mut owner_runtime = puzzle_game_runtime::Puzzle3RuntimeBridge::from_source(source)
            .expect("load owner 3D runtime");
        let state_json = r#"{"kind":"puzzle3d","width":2,"depth":1,"height":1,"layerCount":1,"slots":[1,0],"levelFiredRules":[]}"#;
        runtime
            .set_state_json(state_json)
            .expect("set current state");
        owner_runtime
            .set_state_json(state_json)
            .expect("set owner current state");

        let outcome = runtime
            .transition_current_outcome_json("main", 4)
            .expect("transition current state");
        let owner_outcome = owner_runtime
            .transition_current_outcome_json("main", 4)
            .expect("transition owner current state");
        assert_eq!(
            parse_json_object(&outcome),
            parse_json_object(&owner_outcome)
        );
        let outcome_json = parse_json_object(&outcome);
        let outcome_contract: RuntimeTransitionCurrentOutcome =
            serde_json::from_str(&outcome).expect("3D current outcome should match contract");

        assert_has_object_keys(&outcome_json, RUNTIME_CURRENT_OUTCOME_COMMON_KEYS);
        assert_eq!(outcome_json["changed"], true);
        assert!(!outcome_contract.completed);
        assert!(outcome_json.get("state").is_none());
        assert_eq!(outcome_json["cancelled"], false);
        assert_eq!(outcome_json["commands"], json!([]));
        assert_eq!(outcome_json["firedRules"], json!([0]));
        assert_eq!(
            outcome_json["patches"],
            json!([
                [
                    {
                        "kind": "move",
                        "from": { "x": 0, "y": 0, "z": 0 },
                        "to": { "x": 1, "y": 0, "z": 0 },
                        "objectId": 1
                    }
                ]
            ])
        );
        assert!(outcome_json["stateHash"].is_u64());
        assert!(outcome_json["stateHashKey"].is_string());
        assert!(outcome_json["previousStateHandle"].is_u64());
        assert_eq!(outcome_json["variables"], json!([]));
        assert_eq!(outcome_json["levelFiredRules"], json!([]));
        assert_eq!(
            outcome_json["changedCells"],
            json!([
                { "position": { "x": 0, "y": 0, "z": 0 }, "objects": [] },
                { "position": { "x": 1, "y": 0, "z": 0 }, "objects": [1] }
            ])
        );
        assert!(PUZZLE3_APP_JS.contains("\"runtime current outcome.changedCells\""));
        assert!(PUZZLE3_APP_JS.contains("\"runtime current outcome.animationEvents\""));
        assert!(!PUZZLE3_APP_JS.contains("this.applyRuntimeCells(outcome.changedCells || []);"));
    }

    #[test]
    fn renderer_board_floor_is_transparent_by_default() {
        assert!(RENDERER_CSS.contains("--cell-background: transparent;"));
        assert!(RENDERER_JS.contains("floorColor && floorColor !== \"transparent\""));
    }

    #[test]
    fn renderer_paints_tween_layers_after_static_board_layers() {
        assert!(RENDERER_JS.contains("let startedAt = null;"));
        assert!(RENDERER_JS.contains("let animationFrameIndex = 0;"));
        assert!(RENDERER_JS.contains("if (!this.root.isConnected)"));
        assert!(RENDERER_JS.contains("startedAt ??= performance.now();"));
        assert!(RENDERER_JS.contains(
            "this.animationProgressForFrame(performance.now() - startedAt, duration, animationFrameIndex)"
        ));
        assert!(RENDERER_JS.contains("animationFrameIndex += 1;"));
        assert!(
            RENDERER_JS.contains("animationProgressForFrame(elapsedMs, durationMs, frameIndex)")
        );
        assert!(
            RENDERER_JS.contains("const finalFrameIndex = this.minimumAnimationFrameCount() - 1;")
        );
        assert!(RENDERER_JS.contains("if (frameIndex < finalFrameIndex && timeProgress >= 1)"));
        assert!(RENDERER_JS.contains("return frameIndex / finalFrameIndex;"));
        assert!(RENDERER_JS.contains("minimumAnimationFrameCount()"));
        assert!(RENDERER_JS.contains("return 3;"));
        assert!(RENDERER_JS.contains("requestAnimationFrame(draw);"));
        assert!(
            RENDERER_JS
                .contains("canvasDisplayList(scene, frame, unit, animations = [], progress = 1)")
        );
        assert!(!RENDERER_JS.contains("const staticSurfaces = new Map();"));
        assert!(RENDERER_JS.contains("const staticItems = [];"));
        assert!(RENDERER_JS.contains("const animatedItems = [];"));
        assert!(RENDERER_JS.contains("let order = 0;"));
        assert!(RENDERER_JS.contains("layerOrder: Number(layer.layer) || 0,"));
        assert!(!RENDERER_JS.contains("canvasSurfaceItemForLayer("));
        assert!(RENDERER_JS.contains(
            "const compare = (a, b) => a.layerOrder - b.layerOrder || a.order - b.order;"
        ));
        assert!(
            RENDERER_JS
                .contains("return [...staticItems.sort(compare), ...animatedItems.sort(compare)];")
        );
        assert!(!RENDERER_JS.contains("paintCanvasSurface("));
        assert!(!RENDERER_JS.contains("paintCanvasPatternSurface("));
        assert!(!RENDERER_JS.contains("mergedCanvasRects("));
        assert!(RENDERER_JS.contains("animation: animation && progress < 1 ? animation : null"));
        assert!(RENDERER_JS.contains(
            "for (const item of this.canvasDisplayList(scene, frame, unit, animations, progress))"
        ));
        assert!(RENDERER_JS.contains(
            "paintCanvasItem(context, item, unit, progress = 1, now = performance.now())"
        ));
        assert!(RENDERER_JS.contains(
            "this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress, now);"
        ));
        assert_eq!(RENDERER_JS.matches("context.clip();").count(), 1);
        assert!(!RENDERER_JS.contains("visualSpriteBox("));
        assert!(RENDERER_JS.contains("canvasMetrics(canvas, scene, frame)"));
        assert!(RENDERER_JS.contains("canvasPresentationCellUnit(scene)"));
        assert!(
            RENDERER_JS
                .contains("context.setTransform(metrics.scaleX, 0, 0, metrics.scaleY, 0, 0);")
        );
        assert!(RENDERER_JS.contains("context.__puzzleCanvasScaleX = metrics.scaleX;"));
        assert!(RENDERER_JS.contains("fillCanvasRect(context, x, y, width, height)"));
        assert!(RENDERER_JS.contains("canvasPixelEdge(context, value, axis)"));
        assert!(RENDERER_JS.contains("scaleX: pixelWidth / cssWidth,"));
        assert!(RENDERER_JS.contains("scaleY: pixelHeight / cssHeight,"));
        assert!(RENDERER_JS.contains("const rect = canvas.getBoundingClientRect();"));
        assert!(!RENDERER_JS.contains("canvasCellUnit(scene, frame)"));
        assert!(!RENDERER_JS.contains("spritePatternSize(frameDef)"));
        assert!(!RENDERER_JS.contains("return hasImage ? Math.max(unit, 32) : unit;"));
        assert!(!RENDERER_JS.contains("leastCommonMultiple("));
        assert!(!RENDERER_JS.contains("maximumCanvasCellUnit()"));
        assert!(!RENDERER_JS.contains("scaledPixelEdge(index, sourceUnits, targetPixels)"));
        assert!(!RENDERER_JS.contains("animationForVisualCompanion"));
        assert!(RENDERER_JS.contains("requiresCanvasTransformStack(transform)"));
        assert!(RENDERER_JS.contains("x += transform.x;"));
        assert!(RENDERER_JS.contains("y += transform.y;"));
        assert!(!RENDERER_JS.contains("x = Math.round(x + transform.x);"));
        assert!(!RENDERER_JS.contains("y = Math.round(y + transform.y);"));
    }

    #[test]
    fn renderer_does_not_draw_fallback_sprites() {
        assert!(RENDERER_JS.contains("return null;"));
        assert!(RENDERER_JS.contains("const sprite = this.renderSprite(layer);"));
        assert!(!RENDERER_JS.contains("sprite.className = `sprite ${layer.sprite}`;"));
        assert!(!RENDERER_JS.contains("this.paintFallbackLayer("));
        assert!(!RENDERER_JS.contains("function paintFallbackLayer("));
        assert!(!RENDERER_JS.contains("function hashString("));
        assert!(RENDERER_CSS.contains(".sprite {"));
        assert!(RENDERER_CSS.contains("position: absolute;"));
        assert!(!RENDERER_CSS.contains(".sprite.unknown"));
    }

    #[test]
    fn generated_visuals_include_ordered_sprite_transforms() {
        let source = r#"
title = sprite_translate
puzzle default {
layers {
actor = Player
}
sprites {
Player {
translate (0.5, -0.25)
rotate 90deg
flip true
#fff
00000
00000
00000
00000
00000
}
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let visuals = generated_visuals_js(&loaded);

        assert!(visuals.contains("\"transforms\":[{\"kind\":\"translate\",\"x\":0.5,\"y\":-0.25},{\"kind\":\"rotate\",\"degrees\":90},{\"kind\":\"flip\",\"enabled\":true}]"));
        assert!(!visuals.contains("\"pixelsPerCell\""));
        assert!(RENDERER_JS.contains(
            "applyCanvasVisualTransforms(context, definition, unit, animation = null, progress = 1"
        ));
        assert!(RENDERER_JS.contains("for (const transform of [...transforms].reverse())"));
        assert!(
            RENDERER_JS.contains("tweenedVisualTransforms(definition, animation, progress, now)")
        );
        assert!(RENDERER_JS.contains("if (!animation?.fromObject || progress >= 1)"));
        assert!(RENDERER_JS.contains("rotationTweenDeltaDegrees(fromDegrees, toDegrees)"));
        assert!(RENDERER_JS.contains("if (transform?.kind !== \"rotate\")"));
        assert!(RENDERER_JS.contains("return transform;"));
        assert!(RENDERER_JS.contains("if (delta === -180)"));
        assert!(RENDERER_JS.contains("delta = 180;"));
        assert!(RENDERER_JS.contains("scale(-1, -1)"));
        assert!(RENDERER_JS.contains("visualSpriteFit(definition, unit, sourceSize = null)"));
        assert!(RENDERER_JS.contains("spriteDrawBox(definition)"));
        assert!(RENDERER_JS.contains("solidColor && this.canPaintAsFullCellSolid(definition)"));
        assert!(!RENDERER_JS.contains("unit = Math.max(unit, cellCols, cellRows);"));
        assert!(
            RENDERER_JS
                .contains("const presentationUnit = this.canvasPresentationCellUnit(scene);")
        );
        assert!(
            RENDERER_JS.contains(
                "mode === \"cover\" ? Math.max(scaleX, scaleY) : Math.min(scaleX, scaleY)"
            )
        );
        assert!(RENDERER_JS.contains("context.drawImage(\n          image,"));
        assert!(RENDERER_CSS.contains("--sprite-box-cols"));
        assert!(RENDERER_CSS.contains("background-size: contain;"));
        assert!(!RENDERER_JS.contains("leastCommonMultiple("));
        assert!(
            RENDERER_JS.contains(
                "const { cols: width, rows: height } = this.spritePatternSize(definition);"
            )
        );
        assert!(!RENDERER_JS.contains("domPatternCellUnit()"));
        assert!(!RENDERER_JS.contains("scaledPixelEdge(index, sourceUnits, targetPixels)"));
        assert!(!RENDERER_JS.contains("boundedLeastCommonMultiple"));
        assert!(RENDERER_CSS.contains("overflow: visible;"));
    }

    #[test]
    fn canvas_patterns_do_not_cross_a_per_sprite_raster_boundary() {
        assert!(!RENDERER_JS.contains("cachedPatternBitmap"));
        assert!(!RENDERER_JS.contains("cachedDomPatternBitmap"));
        assert!(RENDERER_JS.contains("domPatternDataUrl(definition)"));
        assert_eq!(RENDERER_JS.matches("this.domPatternDataUrl(").count(), 2);
        assert!(RENDERER_JS.contains("bitmapContext.fillRect(colIndex, rowIndex, 1, 1)"));
        assert!(RENDERER_JS.contains("const url = bitmap.toDataURL(\"image/png\");"));
        assert!(RENDERER_JS.contains("cache.set(key, url);\n    return url;"));

        assert!(!RENDERER_JS.contains("context.drawImage(bitmap,"));
        assert!(RENDERER_JS.contains(
            "this.paintLogicalPatternToCanvas(context, frame, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight)"
        ));
        assert!(RENDERER_JS.contains(
            "this.paintLogicalPatternToCanvas(context, definition, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight)"
        ));
        assert!(RENDERER_JS.contains(
            "paintLogicalPatternToCanvas(context, definition, x, y, pixelWidth, pixelHeight = pixelWidth)"
        ));
        assert!(RENDERER_JS.contains("const left = x + colIndex * pixelWidth;"));
        assert!(
            RENDERER_JS
                .contains("this.fillCanvasRect(context, left, top, right - left, bottom - top);")
        );
        assert!(!RENDERER_JS.contains("Math.round(x + colIndex * pixelWidth)"));
    }

    #[test]
    fn renderer_consumes_2d_render_grid_settings() {
        let source = r#"
title = grid_render
puzzle default {
layers {
actor = Player
}
render {
grid {
type = "all_cells"
}
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level "start" {
P
}
}
}
"#;
        let loaded = puzzle_lang::parse_game2d(source).expect("parse 2D grid render settings");
        let mut scene = String::new();
        push_scene_object(
            &mut scene,
            &loaded,
            &loaded.levels[0].initial_state,
            Some(&loaded.levels[0]),
            None,
        );

        assert!(scene.contains(
            r#""settings":{"render":{},"grid":{"visibility":1,"occupied_cells":false,"all_cells":true},"inputBuffer":{"queueDuringWait":true,"fastForwardWait":true,"minWaitMs":50},"animation":{"tween":{"enabled":false,"intervalMs":250}}}"#
        ));
        assert!(RENDERER_JS.contains("gridSettings(scene)"));
        assert!(RENDERER_JS.contains("scene.settings?.grid"));
        assert!(RENDERER_JS.contains("has-occupied-cell-grid"));
        assert!(RENDERER_JS.contains("has-all-cell-grid"));
        assert!(RENDERER_JS.contains("raw.all_cells ?? raw.allCells"));
        assert!(RENDERER_JS.contains("!grid.allCells && !cell.layers?.length"));
        assert!(RENDERER_CSS.contains(".board.has-occupied-cell-grid .cell.has-objects"));
        assert!(RENDERER_CSS.contains(".board.has-all-cell-grid .cell"));
    }

    #[test]
    fn html_play_fits_the_logical_scene_root_not_individual_cells() {
        assert!(INDEX_HTML.contains(r#"<div id="screenFrame" class="screen-frame">"#));
        assert!(APP_CSS.contains("--scene-layout-unit: 180px;"));
        assert!(APP_CSS.contains("--scene-layout-gap-unit: 1px;"));
        assert!(APP_CSS.contains("width: 100vw;\n  height: 100vh;"));
        assert!(APP_CSS.contains("max-width: 100vw;\n  height: 100vh;"));
        assert!(APP_CSS.contains("max-height: 100vh;"));
        assert!(
            APP_CSS.contains("width: 100%;\n  height: 100%;\n  min-width: 0;\n  min-height: 0;")
        );
        assert!(APP_CSS.contains("max-width: 100%;\n  max-height: 100%;"));
        assert!(APP_JS.contains("function syncScreenScale()"));
        assert!(APP_JS.contains("function clampScreenScaleToFrame()"));
        assert!(APP_JS.contains("clampScreenScaleToFrame();"));
        assert!(APP_JS.contains(
            "Math.min(currentScale, frame.width / virtualWidth, frame.height / virtualHeight)"
        ));
        assert!(APP_JS.contains("function installScreenScaleResizeHooks()"));
        assert!(APP_JS.contains(
            "throw new Error(\"PuzzleStudio HTML play requires ResizeObserver for responsive screen scaling.\");"
        ));
        assert!(APP_JS.contains(
            "const resizeObserver = new ResizeObserver(() => scheduleScreenScaleSync(4));"
        ));
        assert!(APP_JS.contains("resizeObserver.observe(shell);"));
        assert!(APP_JS.contains("resizeObserver.observe(playSurface);"));
        assert!(
            APP_JS
                .contains("window.addEventListener(\"resize\", () => scheduleScreenScaleSync(4));")
        );
        assert!(APP_JS.contains(
            "window.addEventListener(\"orientationchange\", () => scheduleScreenScaleSync(6));"
        ));
        assert!(APP_JS.contains(
            "document.addEventListener(\"fullscreenchange\", () => scheduleScreenScaleSync(6));"
        ));
        assert!(APP_JS.contains(
            "window.visualViewport?.addEventListener(\"resize\", () => scheduleScreenScaleSync(6));"
        ));
        assert!(!APP_JS.contains("visualViewport?.addEventListener(\"scroll\""));
        assert!(APP_JS.contains("const defaultSceneLogicalSize = { width: 4, height: 3 };"));
        assert!(APP_JS.contains("function fitLogicalSceneSize("));
        assert!(APP_JS.contains("function visibleViewportSize()"));
        assert!(APP_JS.contains("const rect = element.getBoundingClientRect();"));
        assert!(APP_JS.contains("Math.min(rect.right, viewport.width) - Math.max(rect.left, 0)"));
        assert!(APP_JS.contains("Math.min(rect.bottom, viewport.height) - Math.max(rect.top, 0)"));
        assert!(APP_JS.contains("const defaultSceneLayoutUnit = 180;"));
        assert!(APP_JS.contains("function virtualSceneSize("));
        assert!(APP_JS.contains("screenView.style.setProperty(\"--screen-scale\""));
        assert!(
            APP_JS.contains("screenFrame.style.width = `min(${Math.ceil(fit.width)}px, 100%)`;")
        );
        assert!(
            APP_JS.contains("screenFrame.style.height = `min(${Math.ceil(fit.height)}px, 100%)`;")
        );
        assert!(APP_CSS.contains("zoom: var(--screen-scale, 1);"));
        assert!(!APP_CSS.contains("transform: scale(var(--screen-scale, 1));"));
        assert!(APP_CSS.contains("body.is-component-embed .screen-view {"));
        assert!(APP_CSS.contains("zoom: 1;"));
        assert!(APP_CSS.contains("justify-content: center;"));
        assert!(APP_CSS.contains("display: flex;"));
        assert!(APP_CSS.contains("flex-direction: column;"));
        assert!(APP_JS.contains("function componentSizingKind(component)"));
        assert!(APP_JS.contains("function componentContainsSizingKind(component, sizing)"));
        assert!(APP_JS.contains("function renderRatioComponent(component, scope = {})"));
        assert!(APP_JS.contains("function markSingleFrameComponentLayer("));
        assert!(APP_JS.contains("function fitPuzzleFrameComponents("));
        assert!(APP_JS.contains("Math.min(frame.width / cols, frame.height / rows)"));
        assert!(APP_JS.contains(r#"root.dataset.frameComponent = "true";"#));
        assert!(APP_JS.contains(r#"slot.dataset.sceneSizing = "ratio";"#));
        assert!(APP_CSS.contains(".scene-layer.has-single-frame-component"));
        assert!(APP_CSS.contains(".scene-ratio-slot"));
        assert!(APP_CSS.contains("flex: 1 1 auto;"));
        assert!(APP_CSS.contains(".scene-flow"));
        assert!(APP_CSS.contains(".screen-view .view-row > .scene-flow"));
        assert!(APP_CSS.contains("flex: 0 1 auto;"));
        assert!(!APP_JS.contains(r#""has-puzzle-scene""#));
        assert!(!APP_CSS.contains(".scene-layer.has-puzzle-scene"));
        assert!(!APP_CSS.contains("justify-content: space-between;"));
        assert!(APP_JS.contains(r#"renderMode: "canvas""#));
        assert!(
            RENDERER_CSS.contains("grid-template-columns: repeat(var(--cols), var(--cell-size));")
        );
        assert!(RENDERER_JS.contains("this.root.style.setProperty(\"--rows\", viewport.height);"));
        assert!(!RENDERER_JS.contains("renderCellSize(scene)"));
        assert!(!RENDERER_JS.contains("this.root.dataset.cellSize"));
        assert!(RENDERER_JS.contains("this.root.classList.toggle(\"is-canvas-renderer\""));
        assert!(RENDERER_JS.contains("scene.screen?.viewportFocusObjects"));
        assert!(RENDERER_JS.contains("focusObjects.has(Number(layer.objectId))"));
        assert!(RENDERER_CSS.contains(".scene-layer > .board.is-canvas-renderer:only-child"));
        assert!(RENDERER_CSS.contains("grid-template-columns: minmax(0, 1fr);"));
        assert!(RENDERER_CSS.contains("object-fit: contain;"));
        assert!(!APP_CSS.contains("grid-auto-flow: row;"));
        assert!(!RENDERER_CSS.contains("minmax(24px, 1fr)"));
    }

    #[test]
    fn html_play_does_not_force_focus_during_load_or_render() {
        assert!(INDEX_HTML.contains(r#"<main id="shell" class="shell" tabindex="0">"#));
        assert!(!INDEX_HTML.contains("autofocus"));
        assert!(APP_JS.contains("document.addEventListener(\"pointerdown\", focusShell);"));
        assert!(APP_JS.contains(
            "if (!shell || document.activeElement === shell || shell.contains(document.activeElement))"
        ));
        assert!(!APP_JS.contains("notifyPreviewState(state);\n  focusShell();"));
        assert!(!APP_JS.contains("document.addEventListener(\"DOMContentLoaded\", focusShell);"));
        assert!(!APP_JS.contains("window.addEventListener(\"focus\", focusShell);"));
        assert!(!APP_JS.contains("requestAnimationFrame(focusShell);"));
        assert!(!APP_JS.contains("setTimeout(focusShell, 0);"));
    }

    #[test]
    fn wait_continuation_timer_does_not_depend_on_animation_frame() {
        assert!(APP_JS.contains("function applyWaitEvents(events)"));
        assert!(APP_JS.contains("window.setTimeout(() => {\n      if (waitTimer.done)"));
        assert!(!APP_JS.contains("requestAnimationFrame(() => {\n      if (waitTimer.done)"));
    }

    #[test]
    fn html_play_standard_choice_focus_uses_logical_grid() {
        assert!(APP_JS.contains("function standardChoiceFocusCells(scene = currentSceneDef())"));
        assert!(APP_JS.contains("function sceneMenuFocusCells(scene = currentSceneDef())"));
        assert!(APP_JS.contains("function levelMenuFocusFootprint(component, context = {})"));
        assert!(
            APP_JS.contains("const hasMenuController = sceneHasComponent(scene, \"level_menu\");")
        );
        assert!(!APP_JS.contains("const hasMenuController = sceneHasComponent(scene, \"level_menu\") || sceneHasComponent(scene, \"choice\");"));
        assert!(
            APP_JS.contains(
                "if (!binding && chrome === \"menu\" && profile.menuFocusCells.length > 0)"
            )
        );
        assert!(APP_JS.contains("effects.push({ kind: \"scene_menu\", input: menuInput });"));
        assert!(APP_JS.contains("syncSceneMenuSelection(screenView);"));
        assert!(APP_JS.contains("assignSceneMenuControl(button, scope);"));
        assert!(
            APP_JS.contains(
                "const itemTop = list.scrollTop + (itemRect.top - listRect.top) * scale;"
            )
        );
        assert!(!APP_JS.contains("const itemTop = item.offsetTop;"));
        assert!(APP_JS.contains("function isControlPointerTarget(target)"));
        assert!(APP_JS.contains("[role='option'], .scene-menu-control"));
        assert!(!APP_JS.contains("[role='button'], [tabindex]"));
        assert!(APP_JS.contains("if (isControlPointerTarget(event.target))"));
        assert!(APP_JS.contains("function componentRowFootprint(components, context = {})"));
        assert!(APP_JS.contains("function componentColumnFootprint(components, context = {})"));
        assert!(APP_JS.contains("component.kind === \"choice\""));
        assert!(APP_JS.contains("if (focusKind === \"choice\" && component.kind === \"choice\")"));
        assert!(!APP_JS.contains("(focusKind === \"menu\" && component.kind === \"button\")"));
        assert!(APP_JS.contains("focusKind === \"menu\" && component.kind === \"level_menu\""));
        assert!(APP_JS.contains("function resolveLevelCollectionSource(source, entries)"));
        assert!(APP_JS.contains("for source `levels` is ambiguous; use a level collection name"));
        assert!(APP_JS.contains("function levelMatchesResource(resource, level)"));
        assert!(APP_JS.contains("throw new Error(`Unknown level collection: ${source}`);"));
        assert!(APP_JS.contains("function stackColumnFootprints(footprints)"));
        assert!(APP_JS.contains("viewItems(component, context.scope || {}).map((item)"));
        assert!(APP_JS.contains("[component.binding]: item"));
        assert!(APP_JS.contains("component.kind === \"conditional\""));
        assert!(APP_JS.contains("return emptyCellFootprint();"));
        assert!(
            APP_JS.contains("function standardChoiceDirectionalTarget(cells, cursor, direction)")
        );
        assert!(APP_JS.contains("cell.y === current.y && cell.x > current.x"));
        assert!(APP_JS.contains("cell.x === current.x && cell.y > current.y"));
        assert!(APP_JS.contains("if (candidates.length === 0)"));
        assert!(APP_JS.contains("return null;"));
        assert!(
            APP_JS.contains("effects.push({ kind: \"standard_choice\", input: standardInput });")
        );
        assert!(APP_JS.contains("return JSON.stringify(String(value.name));"));
        assert!(APP_JS.contains("|| key === \"x\"\n    || code === \"KeyX\";"));
        assert!(!APP_JS.contains("theme-puzzlescript\") && (key === \"x\""));
        assert!(APP_CSS.contains("button.standard-choice.is-selected"));
    }

    #[test]
    fn html_play_level_menu_uses_select_command() {
        assert!(APP_JS.contains("sendCommand(`select:${position}`)"));
        assert!(APP_JS.contains(
            "if (isStandardMenuConfirmKey(key, rawKey, code)) {\n    return \"enter\";\n  }"
        ));
        assert!(APP_JS.contains("\"select\","));
        assert!(APP_JS.contains("String(command).split(\":\", 1)[0] === \"select\""));
        assert!(!APP_JS.contains("sendCommand(`enter:${position}`)"));
        assert!(!APP_JS.contains("enter: \"enter\""));
        assert!(!APP_JS.contains("enter: \"select\""));
    }

    #[test]
    fn clean_theme_removes_button_drop_shadows_and_unifies_vertical_control_width() {
        assert!(APP_JS.contains("function syncCleanControlGroupWidths(root = screenView)"));
        assert!(APP_JS.contains("group.style.removeProperty(\"--clean-control-width\");"));
        assert!(APP_JS.contains("Math.max(max, cleanControlNaturalWidth(control))"));
        assert!(
            APP_JS.contains("group.style.setProperty(\"--clean-control-width\", `${maxWidth}px`);")
        );
        assert!(APP_JS.contains("child.matches(\"button, .level-menu\")"));
        assert!(!APP_JS.contains("function cleanLevelMenuNaturalWidth(list)"));
        assert!(!APP_JS.contains("labelWidth + chromeWidth"));
        assert!(APP_CSS.contains("--button-shadow:"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow);"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow-hover);"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow-active);"));
        assert!(!APP_CSS.contains("--menu-control-width: 420px;"));
        assert!(APP_CSS.contains("--accent: var(--text);"));
        assert!(PUZZLE3_STYLE_CSS.contains("--accent: var(--text);"));
        assert!(THEME_PRESETS_CSS.contains("body.theme-clean {"));
        assert!(THEME_PRESETS_CSS.contains("--accent: var(--text);"));
        assert!(!APP_CSS.contains("--accent: #2f7ebc;"));
        assert!(!PUZZLE3_STYLE_CSS.contains("--accent: #2f7ebc;"));
        assert!(!THEME_PRESETS_CSS.contains("--accent: #2f7ebc;"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow-hover: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow-active: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-hover-transform: none;"));
        assert!(!THEME_PRESETS_CSS.contains("--clean-menu-control-width:"));
        assert!(!THEME_PRESETS_CSS.contains("--menu-control-width: 420px;"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .scene-layer > button,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .scene-layer > .level-menu,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .view-column > button,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .view-column > .level-menu,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .view-box > button,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .view-box > .level-menu {"));
        assert!(
            THEME_PRESETS_CSS
                .contains("width: var(--clean-control-width, auto);\n  max-width: 100%;")
        );
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .level-menu {"));
        assert!(THEME_PRESETS_CSS.contains("scrollbar-width: none;"));
        assert!(!THEME_PRESETS_CSS.contains("max-height: min(62vh"));
        assert!(
            !THEME_PRESETS_CSS
                .contains(".theme-clean .level-menu li,\n.theme-clean .is-menu-scene button")
        );
        assert!(!THEME_PRESETS_CSS.contains("text-overflow: ellipsis;"));
        assert!(
            THEME_PRESETS_CSS
                .contains(".theme-clean .level-menu::-webkit-scrollbar {\n  display: none;\n}")
        );
        assert!(
            THEME_PRESETS_CSS.contains(
                ".theme-clean .level-menu .ps-control-label {\n  white-space: nowrap;\n}"
            )
        );
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .is-menu-scene button {\n  justify-items: center;\n  text-align: center;\n}"));
        assert!(THEME_PRESETS_CSS.contains(".theme-clean .is-menu-scene button .ps-control-label {\n  justify-self: center;\n  text-align: center;\n}"));
    }

    #[test]
    fn puzzlescript_theme_reserves_terminal_control_width_for_confirm_glyphs() {
        assert!(APP_JS.contains("function setControlLabel(control, label)"));
        assert!(APP_JS.contains("function controlLabelNodes(label)"));
        assert!(APP_JS.contains("choice.dataset.standardChoiceIndex = String(index);"));
        assert!(APP_JS.contains("function syncStandardChoiceSelection(choice, selectedIndex)"));
        assert!(APP_JS.contains("syncStandardChoiceSelection(choice, index);"));
        assert!(APP_JS.contains("left.className = \"ps-control-edge is-left\";"));
        assert!(APP_JS.contains("text.className = \"ps-control-label\";"));
        assert!(APP_JS.contains("right.className = \"ps-control-edge is-right\";"));
        assert!(APP_JS.contains("item.append(...controlLabelNodes("));
        assert!(
            APP_CSS.contains(".ps-control-edge {\n  display: none;\n  pointer-events: none;\n}")
        );
        assert!(APP_JS.contains("function puzzlescriptConfirmFill(target)"));
        assert!(APP_JS.contains("function puzzlescriptControlCharWidth(target)"));
        assert!(APP_JS.contains("target.style.setProperty(\"--ps-confirm-fill\""));
        assert!(APP_JS.contains("rect.width / charWidth"));
        assert!(!APP_JS.contains("const puzzlescriptTerminalWidth"));
        assert!(!APP_JS.contains("const sideCount = Math.floor(hashCount / 2);"));
        assert!(!APP_JS.contains("target.style.setProperty(\"--ps-confirm-label-width\""));
        assert!(!APP_JS.contains("target.style.setProperty(\"--ps-confirm-left\""));
        assert!(!APP_JS.contains("target.style.setProperty(\"--ps-confirm-right\""));
        assert!(!APP_JS.contains("line.className = \"ps-confirm-line\";"));
        assert!(!APP_JS.contains("target.replaceChildren(line);"));
        assert!(!APP_JS.contains("const spacer = hashCount % 2 === 0 ? \"\" : \" \";"));
        assert!(!APP_JS.contains("target.style.setProperty(\"--ps-confirm-line\""));
        assert!(!APP_JS.contains("--ps-confirm-before"));
        assert!(!APP_JS.contains("--ps-confirm-after"));
        assert!(THEME_PRESETS_CSS.contains("--ps-terminal-control-width: 36ch;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-title-font-size: 48px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-title-line-height: 60px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-body-font-size: 24px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-body-line-height: 36px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-control-font-size: 24px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-control-line-height: 36px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-message-font-size: 24px;"));
        assert!(THEME_PRESETS_CSS.contains("--ps-message-line-height: 36px;"));
        assert!(!THEME_PRESETS_CSS.contains("--ps-line-height:"));
        assert!(THEME_PRESETS_CSS.contains("font-size: var(--ps-title-font-size);"));
        assert!(THEME_PRESETS_CSS.contains("font-size: var(--ps-body-font-size);"));
        assert!(THEME_PRESETS_CSS.contains("font-size: var(--ps-control-font-size);"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .screen-view .view-text {"));
        assert!(
            !THEME_PRESETS_CSS
                .contains("--ps-confirm-fill: \"####################################\";")
        );
        assert!(THEME_PRESETS_CSS.contains("width: min(100%, var(--ps-terminal-control-width));"));
        assert!(THEME_PRESETS_CSS.contains("white-space: nowrap;"));
        assert!(THEME_PRESETS_CSS.contains("position: relative;"));
        assert!(THEME_PRESETS_CSS.contains("display: grid;"));
        assert!(THEME_PRESETS_CSS.contains(
            "grid-template-columns: minmax(0, 1fr) minmax(0, max-content) minmax(0, 1fr);"
        ));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .ps-control-label {"));
        assert!(
            !THEME_PRESETS_CSS
                .contains(".theme-puzzlescript .level-menu li > span:not(.level-clear-mark)")
        );
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .ps-control-edge {"));
        assert!(THEME_PRESETS_CSS.contains("display: block;"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .ps-control-edge.is-left {"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .ps-control-edge.is-right {"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .level-menu li::before,"));
        assert!(THEME_PRESETS_CSS.contains("display: none;"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .level-menu li {\n  width: 100%;"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript button,\n.theme-puzzlescript .level-menu li {\n  overflow: hidden;\n}"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript button:active,"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript button.is-confirming {"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .level-clear-mark {"));
        assert!(THEME_PRESETS_CSS.contains("right: 1ch;"));
        assert!(THEME_PRESETS_CSS.contains("width: 1ch;"));
        assert!(THEME_PRESETS_CSS.contains("content: var(--ps-confirm-fill, \"#\");"));
        assert!(THEME_PRESETS_CSS.contains("content: \"\";"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .view-list {"));
        assert!(THEME_PRESETS_CSS.contains("width: min(100%, var(--ps-terminal-control-width));"));
        assert!(THEME_PRESETS_CSS.contains(".theme-puzzlescript .view-list > li {"));
        assert!(THEME_PRESETS_CSS.contains("justify-items: center;"));
        assert!(THEME_PRESETS_CSS.contains("scrollbar-width: none;"));
        assert!(
            THEME_PRESETS_CSS.contains(
                ".theme-puzzlescript .level-menu::-webkit-scrollbar {\n  display: none;\n}"
            )
        );
    }

    #[test]
    fn html_play_commits_snapshot_before_showing_message_events() {
        let render_start = APP_JS.find("function render(state) {").unwrap();
        let render_body = &APP_JS[render_start..];
        let scene_index = render_body.find("renderSceneStack(state);").unwrap();
        let message_index = render_body
            .find("applyMessageEvents(state?.messageEvents || []);")
            .unwrap();
        assert!(scene_index < message_index);
    }

    #[test]
    fn html_play_buffers_one_busy_model_input_without_queueing_commands() {
        assert!(APP_JS.contains("let pendingModelInput = null;"));
        assert!(APP_JS.contains("let drainingQueuedModelInput = false;"));
        assert!(!APP_JS.contains("pendingCommandQueue"));
        assert!(APP_JS.contains("pendingModelInput = input;"));
        assert!(APP_JS.contains("function drainQueuedModelInput()"));
        assert!(APP_JS.contains("if (currentState?.busy || clientPendingWaits > 0) {\n    return undefined;\n  }\n  return sendCommandNow(command);"));
        assert!(APP_JS.contains(
            "const dispatchEffects = event.repeat && (currentState?.busy || clientPendingWaits > 0)"
        ));
        assert!(APP_JS.contains("effects.filter((effect) => effect?.kind !== \"model_input\")"));
        assert!(APP_JS.contains("currentState?.busy || clientPendingWaits > 0"));
        assert!(APP_JS.contains("function inputBufferConfig()"));
        assert!(APP_JS.contains("if (!config.queueDuringWait)"));
        assert!(APP_JS.contains("function fastForwardActiveWaitsForQueuedInput"));
        assert!(APP_JS.contains("source.fastForwardWait !== false"));
        assert!(APP_JS.contains("Number(source.minWaitMs ?? 50)"));
        assert!(
            !APP_JS.contains("if (currentState.busy) {\n    return;\n  }\n  broadcastPuzzle3Key")
        );
        assert!(!APP_JS.contains("clientPendingAnimations"));
        assert!(!APP_JS.contains("clientPendingCommands"));
    }

    #[test]
    fn html_play_message_popup_accepts_default_and_game_input_dismiss_keys() {
        assert!(APP_JS.contains("function isMessageDismissKey(event)"));
        assert!(APP_JS.contains("rawKey === \"Enter\""));
        assert!(APP_JS.contains("rawKey === \" \""));
        assert!(APP_JS.contains("key === \"x\""));
        assert!(APP_JS.contains("return effectsForKey(event).length > 0;"));
        assert!(APP_JS.contains("if (messagePopup) {\n    event.preventDefault();\n    if (isMessageDismissKey(event)) {\n      closeMessagePopup();\n    }\n    return;\n  }"));
        assert!(APP_JS.contains("if (messagePopup) {\n      if (isMessageDismissKey(keyEvent)) {\n        closeMessagePopup();\n      }\n      return;\n    }"));
        assert!(!APP_JS.contains("backdrop.addEventListener(\"click\", closeMessagePopup);"));
        assert!(!APP_JS.contains("ShowMessage"));
        assert!(!APP_JS.contains("CloseMessage"));
        assert!(!APP_JS.contains("hasSfx"));
        assert!(APP_CSS.contains(".message-popup-backdrop:focus {\n  outline: none;\n}"));
    }

    #[test]
    fn html_play_consumes_sound_events_during_render() {
        let render_start = APP_JS.find("function render(state) {").unwrap();
        let render_body = &APP_JS[render_start..];
        let sound_index = render_body
            .find("soundRuntime.applyEvents(state?.soundEvents || []);")
            .unwrap();
        let clear_index = render_body.find("state.soundEvents = [];").unwrap();
        assert!(sound_index < clear_index);
    }

    #[test]
    fn html_play_does_not_fallback_to_synthetic_sound_when_generator_is_missing() {
        assert!(APP_JS.contains("warnSoundIssue"));
        assert!(APP_JS.contains("sound generator is unavailable"));
        assert!(!APP_JS.contains("playMusicNote("));
        assert!(!APP_JS.contains("this.seedValue("));
        assert!(!APP_JS.contains("this.seededRandom("));
    }

    #[test]
    fn html_play_passes_sfx_volume_to_sound_generator() {
        assert!(APP_JS.contains("const volume = Number(def.volume ?? 1);"));
        assert!(APP_JS.contains("createSfxPlayer(context, effect, { volume })"));
        assert!(APP_JS.contains("player.start(context.currentTime);"));
        assert!(!APP_JS.contains("def.type === \"puzzlescript\""));
        assert!(!APP_JS.contains("createPuzzleScriptSfxPlayer"));
        assert!(!APP_JS.contains("generatePuzzleScriptSoundEffect"));
    }

    #[test]
    fn html_play_primes_audio_for_forwarded_editor_inputs() {
        assert!(APP_JS.contains("this.sfxEffectCache = new Map();"));
        assert!(APP_JS.contains("primePlayback()"));
        assert!(APP_JS.contains("document.addEventListener(\"keydown\", () => soundRuntime.primePlayback(), { capture: true });"));
        assert!(APP_JS.contains("document.addEventListener(\"pointerdown\", () => soundRuntime.primePlayback(), { capture: true });"));
        assert!(APP_JS.contains(
            "if (event.data?.type === \"PuzzleStudioKey\") {\n    soundRuntime.primePlayback();"
        ));
        assert!(APP_JS.contains("if (command) {\n    soundRuntime.primePlayback();"));
        assert!(APP_JS.contains("const effect = this.sfxEffect(api, def);"));
        assert!(APP_JS.contains("effect = api.generateSoundEffect(def.seed, { type });"));
        assert!(APP_JS.contains("this.activeSfx = new Map();"));
        assert!(APP_JS.contains("this.replaceActiveSfx(name, player);"));
        assert!(APP_JS.contains("this.activeSfx.get(name)?.stop();"));
        assert!(!APP_JS.contains("stopActiveSfx"));
        assert!(!APP_JS.contains("sfxQueue"));
        assert!(!APP_JS.contains("soundQueue"));
    }

    #[test]
    fn standalone_export_includes_sfx_volume() {
        let source = r#"
title = Sfx Volume

sounds {
  sfx click seed=click type=select volume=1.25
  music loop seed=loop bars=16 height=0.62 bpm=104 volume=1.5
}

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
    [ Player ] -> [ Player ] sfx click
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let html = export_html_from_source(source, "games/sfx_volume.puzzle", "", "")
            .expect("export should succeed");

        assert!(html.contains(r#"\"sfx\":[{\"name\":\"click\",\"seed\":\"click\",\"type\":\"select\",\"volume\":1.25}]"#));
        assert!(html.contains(r#"\"music\":[{\"name\":\"loop\",\"seed\":\"loop\",\"height\":0.62,\"bars\":16,\"bpm\":104,\"volume\":1.5}]"#));
    }

    #[test]
    fn html_play_does_not_fallback_scene_definitions_to_variable_export() {
        assert!(APP_JS.contains(
            "return nonEmptyArray(source?.scenes) || nonEmptyArray(source?.screens) || [];"
        ));
        assert!(!APP_JS.contains("window.PuzzleExport?.scenes"));
        assert!(!APP_JS.contains("window.PuzzleExport?.screens"));
    }

    #[test]
    fn html_play_does_not_read_screen_named_scene_compat_state() {
        assert!(!APP_JS.contains("screenState"));
        assert!(!APP_JS.contains("screenPuzzles"));
        assert!(!APP_JS.contains("visibleScreens"));
    }

    #[test]
    fn puzzle3_app_does_not_fallback_to_empty_snapshot_when_fixture_load_fails() {
        assert!(PUZZLE3_APP_JS.contains("async function loadInitialPuzzle3Snapshot()"));
        assert!(PUZZLE3_APP_JS.contains(
            "throw new Error(`Could not load Puzzle3 fixture ./fixture.json (${status})`);"
        ));
        assert!(PUZZLE3_APP_JS.contains("function requirePuzzle3Snapshot("));
        assert!(PUZZLE3_APP_JS.contains("function requireLoadedPuzzle3Snapshot("));
        assert!(PUZZLE3_APP_JS.contains("function showPuzzle3LoadError(error)"));
        assert!(PUZZLE3_APP_JS.contains("controllerApi.ready = loadPuzzle3ControllerSnapshot();"));
        assert!(!PUZZLE3_APP_JS.contains("catch {\n    nextSnapshot = fallbackSnapshot;"));
        assert!(!PUZZLE3_APP_JS.contains("normalizeSnapshot(source || fallbackSnapshot)"));
        assert!(!PUZZLE3_APP_JS.contains("snapshot || fallbackSnapshot"));
        assert!(!PUZZLE3_APP_JS.contains("source || fallbackSnapshot"));
    }

    #[test]
    fn puzzle3_app_requires_current_runtime_contract_version() {
        let expected = format!(
            "const PUZZLE3_RUNTIME_CONTRACT_VERSION = {};",
            puzzle_runtime_contract::RUNTIME_CONTRACT_VERSION
        );
        assert!(PUZZLE3_APP_JS.contains(&expected));
        assert!(
            PUZZLE3_APP_JS
                .contains("Number(contract.version) !== PUZZLE3_RUNTIME_CONTRACT_VERSION")
        );
        assert!(!PUZZLE3_APP_JS.contains("Number(contract.version) !== 2"));
    }

    #[test]
    fn html_play_serializes_level_refs_as_unquoted_scene_args() {
        assert!(APP_JS.contains("function exprValueSource(value)"));
        assert!(APP_JS.contains("value?.kind === \"level\""));
        assert!(APP_JS.contains("return String(value.name);"));
    }

    #[test]
    fn puzzle3_app_exposes_editor_preview_update_contract() {
        assert!(APP_JS.contains(
            "const PREVIEW_SURFACE_UPDATE_MESSAGE = \"PuzzleStudioPreviewSurfaceUpdate\";"
        ));
        assert!(APP_JS.contains("const PUZZLE3_LEVEL_PREVIEW_KIND = \"puzzle3-level\";"));
        assert!(APP_JS.contains("const ISOLATED_PREVIEW_MODE = \"isolated\";"));
        assert!(APP_JS.contains("const PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE = \"PuzzleStudioRenderPuzzle3ModelComponent\";"));
        assert!(APP_JS.contains("let initialPuzzle3PreviewSurface = null;"));
        assert!(APP_JS.contains("initialPuzzle3PreviewSurface = normalizePuzzle3PreviewSurface("));
        assert!(APP_JS.contains("let puzzle3PreviewSurface = initialPuzzle3PreviewSurface;"));
        assert!(APP_JS.contains("function normalizePuzzle3PreviewSurface(update = null)"));
        assert!(APP_JS.contains("function puzzle3PreviewSurfaceFixture(source, sceneName)"));
        assert!(APP_JS.contains(
            "function puzzle3PreviewSurfaceControllerUpdate(surface = puzzle3PreviewSurface)"
        ));
        assert!(APP_JS.contains("camera: payload.camera"));
        assert!(APP_JS.contains("view: payload.view"));
        assert!(APP_JS.contains("settings: payload.settings || {}"));
        assert!(PUZZLE3_APP_JS.contains(
            "const PREVIEW_SURFACE_UPDATE_MESSAGE = \"PuzzleStudioPreviewSurfaceUpdate\";"
        ));
        assert!(PUZZLE3_APP_JS.contains("function puzzle3PreviewUpdateFromSurface(update = {})"));
        assert!(
            PUZZLE3_APP_JS.contains("if (event.data?.type === PREVIEW_SURFACE_UPDATE_MESSAGE)")
        );
        assert!(PUZZLE3_APP_JS.contains("levelIndex: payload.levelIndex"));
        assert!(PUZZLE3_APP_JS.contains("camera: payload.camera"));
        assert!(PUZZLE3_APP_JS.contains("view: payload.view"));
        assert!(PUZZLE3_APP_JS.contains("settings: payload.settings || {}"));
        assert!(APP_JS.contains("if (puzzle3PreviewSurface) {\n    return puzzle3PreviewSurfaceFixture(fixture, sceneName);\n  }"));
        assert!(
            APP_JS.contains("if (componentEmbedMode && renderEmbeddedPuzzleComponent(layers))")
        );
        assert!(
            APP_JS.contains("if (puzzle3PreviewSurface && renderEmbeddedPuzzleComponent(layers))")
        );
        assert!(
            APP_JS.contains("if (event.data?.type === PREVIEW_SURFACE_UPDATE_MESSAGE || event.data?.type === PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE)")
        );
        assert!(APP_JS.contains("if (!puzzle3PreviewSurface && !currentSceneHasPuzzle3())"));
        assert!(APP_JS.contains(
            "window.applyPuzzleStudioPreviewSurfaceUpdate = applyPuzzleStudioPreviewSurfaceUpdate;"
        ));
        let stripped = strip_optional_host_blocks(APP_JS, "puzzle3");
        assert!(!stripped.contains("normalizePuzzle3PreviewSurface("));
        assert!(!stripped.contains("PuzzleStudioInitialPreviewSurfaceConsumed"));
        assert!(!stripped.contains("effectiveComponentEmbedMode"));
        assert!(PUZZLE3_APP_JS.contains("function applyPuzzle3PreviewUpdate(update = {})"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioUpdatePuzzle3Preview"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioRenderPuzzle3ModelComponent"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioInitialModelComponentPreview"));
        assert!(
            PUZZLE3_APP_JS
                .contains("function applyPuzzle3ModelComponentPreviewUpdate(update = {})")
        );
        assert!(PUZZLE3_APP_JS.contains(
            "const initialModelPreview = window.PuzzleStudioInitialModelComponentPreview;"
        ));
        assert!(PUZZLE3_APP_JS.contains(
            "await loadSnapshotData(next, puzzle3ModelComponentPreviewLoadOptions(initialModelPreview));"
        ));
        assert!(
            PUZZLE3_APP_JS
                .contains("window.PuzzleStudioInitialModelComponentPreviewConsumed = true;")
        );
        assert!(PUZZLE3_APP_JS.contains(
            "function puzzle3PreviewSnapshot(update = {}, source = requireLoadedPuzzle3Snapshot(\"Puzzle3 preview source snapshot\"))"
        ));
        assert!(!PUZZLE3_APP_JS.contains(
            "await loadSnapshotData(nextSnapshot);\n  if (window.PuzzleStudioInitialModelComponentPreview"
        ));
        assert!(PUZZLE3_APP_JS.contains("modelComponentPreview: {"));
        assert!(PUZZLE3_APP_JS.contains("if (editorModelComponentPreview)"));
        assert!(PUZZLE3_APP_JS.contains("mergePuzzle3PreviewSettings"));
        assert!(
            PUZZLE3_APP_JS
                .contains("applyPuzzle3PreviewResources(next, update.resources || update)")
        );
        assert!(PUZZLE3_APP_JS.contains("function applyPuzzle3PreviewResources"));
        assert!(
            PUZZLE3_APP_JS
                .contains("target.sprites = JSON.parse(JSON.stringify(resources.sprites));")
        );
        assert!(PUZZLE3_APP_JS.contains("next.levels[levelIndex]"));
        assert!(APP_JS.contains("zoom: view.zoom,"));
        assert!(PUZZLE3_APP_JS.contains("zoom: update.camera.zoom ?? update.view?.zoom,"));
        assert!(PUZZLE3_APP_JS.contains("next.settings = mergePuzzle3PreviewSettings"));
        assert!(PUZZLE3_APP_JS.contains(r#"coordinateSpace: "canvas-css-px""#));
        assert!(PUZZLE3_APP_JS.contains(
            "const target = source?.target || source?.origin || modelCenterForSize(size);"
        ));
        assert!(PUZZLE3_APP_JS.contains("function modelCenterForSize(size)"));
        assert!(PUZZLE3_APP_JS.contains("view.originX = width / 2;"));
        assert!(!PUZZLE3_APP_JS.contains(") / 2 + (Number(target.x) || 0)"));
    }

    #[test]
    fn puzzle3_app_does_not_own_scene_layout_rendering() {
        assert!(
            !PUZZLE3_APP_JS.contains("function renderSceneNode("),
            "puzzle3_app.js must render a puzzle3 component, not own the generic scene layout renderer"
        );
        assert!(
            !PUZZLE3_APP_JS.contains("function renderSceneContainer("),
            "generic scene containers belong to the shared scene renderer"
        );
        assert!(
            !PUZZLE3_APP_JS.contains("function measureSceneNode("),
            "generic scene measurement belongs to the shared scene renderer"
        );
        assert!(
            !PUZZLE3_APP_JS.contains("function renderSceneFor("),
            "generic scene for-loops belong to the shared scene renderer"
        );
    }

    #[test]
    fn puzzle3_app_supports_focus_relative_viewport_framing() {
        assert!(
            PUZZLE3_APP_JS
                .contains("function fitProjectionToViewport(renderContext, options = {})")
        );
        assert!(PUZZLE3_APP_JS.contains(
            "function viewportFramingProjectionBounds(size, camera, viewport, focusCell)"
        ));
        assert!(PUZZLE3_APP_JS.contains("viewport.framingBox.height === \"full\""));
        assert!(PUZZLE3_APP_JS.contains("function scheduleViewportAnimation()"));
        assert!(PUZZLE3_APP_JS.contains("target.follow !== \"smooth\" || view.viewportSnapNext"));
        assert!(PUZZLE3_APP_JS.contains("function smoothViewportOrigin(nextX, nextY, target)"));
        assert!(PUZZLE3_APP_JS.contains("function smoothViewportMaxLag(target)"));
        assert!(PUZZLE3_APP_JS.contains("const amount = 0.12;"));
        assert!(PUZZLE3_APP_JS.contains("function requestSceneViewportDraw()"));
        assert!(PUZZLE3_APP_JS.contains("if (smoothViewportActive())"));
        assert!(PUZZLE3_APP_JS.contains("function smoothViewportActive()"));
        assert!(PUZZLE3_APP_JS.contains("requestSceneViewportDraw();"));
        assert!(
            PUZZLE3_APP_JS.contains("const advanceViewport = options.advanceViewport !== false;")
        );
        assert!(
            PUZZLE3_APP_JS.contains("fitProjectionToViewport(renderContext, { advanceViewport })")
        );
        assert!(PUZZLE3_APP_JS.contains("if (options.advanceViewport === false)"));
        assert!(PUZZLE3_APP_JS.contains("target.cellScale * projectionZoom(camera) * 3.5"));
        assert!(PUZZLE3_APP_JS.contains("const SCENE_DEFAULT_WIDTH = 16;"));
        assert!(PUZZLE3_APP_JS.contains("const SCENE_DEFAULT_HEIGHT = 12;"));
        assert!(PUZZLE3_APP_JS.contains("function puzzle3SceneDisplaySize()"));
        assert!(!PUZZLE3_APP_JS.contains("function currentPuzzle3IntrinsicSize()"));
        assert!(PUZZLE3_APP_JS.contains(
            "function viewportFitForFrame(frame, viewportBounds, centerPoint = null, zoom = 1, follow = \"snap\")"
        ));
        assert!(!PUZZLE3_APP_JS.contains("function viewportFramingProjectionCenter"));
        assert!(
            PUZZLE3_APP_JS.contains(
                "function viewportFocusProjectionAnchor(size, camera, viewport, focusCell)"
            )
        );
        assert!(PUZZLE3_APP_JS.contains(
            "function viewportFocusVisualProjectionAnchor(size, camera, viewport, focusCell)"
        ));
        assert!(PUZZLE3_APP_JS.contains(
            "for (const voxel of objectVoxels(focusCell.position || {}, object, sourceKey))"
        ));
        assert!(
            PUZZLE3_APP_JS.contains("function viewportFramingRanges(size, viewport, focusCell)")
        );
        assert!(PUZZLE3_APP_JS.contains("function virtualCenteredCellRange(center, span)"));
        assert!(PUZZLE3_APP_JS.contains(
            "const xRange = viewportCellRange(Number(position.x) || 0, viewport.framingBox.width, viewport.mode);"
        ));
        assert!(PUZZLE3_APP_JS.contains(
            "const yRange = viewportCellRange(Number(position.y) || 0, viewport.framingBox.depth, viewport.mode);"
        ));
        assert!(PUZZLE3_APP_JS.contains(
            ": viewportCellRange(Number(position.z) || 0, viewport.framingBox.height, viewport.mode);"
        ));
        assert!(PUZZLE3_APP_JS.contains("function virtualPagedCellRange(center, span)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("viewport?.mode === \"centered\" || viewport?.mode === \"paged\"")
        );
        assert!(!PUZZLE3_APP_JS.contains("function centeredCellRange(center, span, limit)"));
        assert!(PUZZLE3_APP_JS.contains(
            "const anchorPoint = viewportFocusProjectionAnchor(size, camera, viewport, focus);"
        ));
        assert!(
            PUZZLE3_APP_JS.contains(
                "const anchorX = Number.isFinite(centerX) ? centerX : (minX + maxX) / 2;"
            )
        );
        assert!(PUZZLE3_APP_JS.contains("originY: frameHeight / 2 - anchorY * effectiveScale"));
        assert!(PUZZLE3_APP_JS.contains("viewportFitForFrame("));
        assert!(PUZZLE3_APP_JS.contains("function puzzle3RenderContext(width = canvas.clientWidth, height = canvas.clientHeight)"));
        assert!(PUZZLE3_APP_JS.contains("function canvasLayoutFrame()"));
        assert!(PUZZLE3_APP_JS.contains("Number(canvas.clientWidth) || Number(rect.width) || 1"));
        assert!(PUZZLE3_APP_JS.contains("const frame = canvasLayoutFrame();"));
        assert!(PUZZLE3_APP_JS.contains("function normalizeFrame(frame)"));
        assert!(PUZZLE3_APP_JS.contains("function normalizeModelSize(size)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("function fitScaleForProjectedBounds(frame, bounds, margin = 0)")
        );
        assert!(PUZZLE3_APP_JS.contains("const candidates = renderCellCandidates(renderContext);"));
        assert!(
            PUZZLE3_APP_JS
                .contains("function renderCellCandidates(renderContext = puzzle3RenderContext())")
        );
        assert!(PUZZLE3_APP_JS.contains("function viewportRenderCullingEnabled(renderContext)"));
        assert!(!PUZZLE3_APP_JS.contains("function viewportRenderPixelMargin"));
        assert!(!PUZZLE3_APP_JS.contains("function projectedCellPixelMargin"));
        assert!(PUZZLE3_APP_JS.contains("function cellProjectsIntoFrame(position, frame)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("cellProjectsIntoFrame(cell.position || {}, renderContext.frame)")
        );
        assert!(PUZZLE3_APP_JS.contains("bounds.maxX >= 0"));
        assert!(PUZZLE3_APP_JS.contains("bounds.minX <= frame.width"));
        assert!(PUZZLE3_APP_JS.contains("bounds.maxY >= 0"));
        assert!(PUZZLE3_APP_JS.contains("bounds.minY <= frame.height"));
        assert!(PUZZLE3_APP_JS.contains("cellHasRenderableVoxels(cell)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("const effectiveScale = baseScale * Math.max(0.1, Number(zoom) || 1);")
        );
        assert!(PUZZLE3_APP_JS.contains("cellScale: baseScale"));
    }

    #[test]
    fn app_forwards_puzzle3_keys_while_busy_so_inputs_can_queue() {
        assert!(APP_JS.contains(
            "if (!currentState) {\n    return;\n  }\n  /* puzzle-host:optional:puzzle3:start */\n  if (broadcastPuzzle3Key(event, \"down\"))"
        ));
        assert!(APP_JS.contains(
            "if (broadcastPuzzle3Key(event, \"down\")) {\n    event.preventDefault();\n    return;\n  }"
        ));
        assert!(
            !APP_JS.contains("if (currentState.busy) {\n    return;\n  }\n  broadcastPuzzle3Key")
        );
        assert!(APP_JS.contains("document.addEventListener(\"keyup\", (event) => {"));
        assert!(APP_JS.contains("broadcastPuzzle3Key(event, \"up\");"));
        assert!(PUZZLE3_APP_JS.contains("function handleComponentEmbedKeydown(event)"));
        assert!(PUZZLE3_APP_JS.contains(
            "if (inlineComponentMount) {\n  // Inline controllers receive input through the host controller contract.\n} else if (!effectiveComponentEmbedMode()) {"
        ));
        assert!(
            PUZZLE3_APP_JS
                .contains("window.addEventListener(\"keydown\", handleComponentEmbedKeydown);")
        );
        assert!(PUZZLE3_APP_JS.contains("function handleComponentEmbedKeyup(event)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("window.addEventListener(\"keyup\", handleComponentEmbedKeyup);")
        );
        assert!(PUZZLE3_APP_JS.contains("function startHeldSceneInput(holdId, input)"));
        assert!(PUZZLE3_APP_JS.contains("heldSceneInputs.set(holdId, input);"));
        assert!(PUZZLE3_APP_JS.contains("function applyPuzzle3CommandKey(event)"));
        assert!(PUZZLE3_APP_JS.contains(
            "return applyPuzzle3CommandKey(event || {}) || puzzle3Component.handleKey(event || {});"
        ));
        assert!(!PUZZLE3_APP_JS.contains("SCENE_INPUT_REPEAT_INTERVAL_MS"));
        assert!(!PUZZLE3_APP_JS.contains("setInterval(() => enqueueSceneInput"));
    }

    #[test]
    fn puzzle3_app_does_not_render_missing_sprite_fallback_cube() {
        assert!(PUZZLE3_APP_JS.contains("if (!object.sprite) {\n    return [];\n  }"));
        assert!(PUZZLE3_APP_JS.contains("if (!template) {\n    return [];\n  }"));
        assert!(PUZZLE3_APP_JS.contains("if (!sprite) {\n    return null;\n  }"));
        assert!(!PUZZLE3_APP_JS.contains("cssVar(\"--top\") || \"#ffde8a\""));
        assert!(!PUZZLE3_APP_JS.contains("red_cube"));
        assert!(!PUZZLE3_APP_JS.contains("Red Cube"));
        assert!(!PUZZLE3_APP_JS.contains("Bumpy"));
    }

    #[test]
    fn puzzle3_app_culls_only_opaque_internal_voxel_faces_across_cells() {
        assert!(PUZZLE3_APP_JS.contains("function renderOpaqueOcclusion(renderContext)"));
        assert!(PUZZLE3_APP_JS.contains("for (const cell of snapshot.cells || [])"));
        assert!(PUZZLE3_APP_JS.contains("renderContext.opaqueOcclusion = occupied;"));
        assert!(
            PUZZLE3_APP_JS
                .contains("function cellVisibleVoxelsForRender(cell, renderContext = null)")
        );
        assert!(PUZZLE3_APP_JS.contains("renderContext.visibleVoxelCells = new Map();"));
        assert!(PUZZLE3_APP_JS.contains("function isVoxelFaceOccluded(voxel, offset, occupied)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("if (voxel.opaque !== false && occupied.opaque.has(adjacentKey))")
        );
        assert!(PUZZLE3_APP_JS.contains("occupied.bySource.has(`${sourceKey}|${adjacentKey}`)"));
    }

    #[test]
    fn puzzle3_app_preserves_alpha_voxel_layers_for_depth_sorting() {
        assert!(PUZZLE3_APP_JS.contains("function visibleVoxelStack(stack)"));
        assert!(PUZZLE3_APP_JS.contains("const visible = [];"));
        assert!(PUZZLE3_APP_JS.contains("opaque: source.a >= 0.999"));
        assert!(PUZZLE3_APP_JS.contains("if (renderVoxel.opaque) {\n      visible.length = 0;"));
        assert!(PUZZLE3_APP_JS.contains("visible.push(renderVoxel);"));
        assert!(PUZZLE3_APP_JS.contains("voxels.push(...visibleStack);"));
        assert!(!PUZZLE3_APP_JS.contains("function compositeVoxelStack(stack)"));
        assert!(!PUZZLE3_APP_JS.contains("function compositeColor(source, destination)"));
    }

    #[test]
    fn puzzle3_app_caches_static_sprite_voxel_templates() {
        assert!(PUZZLE3_APP_JS.contains("const spriteVoxelTemplateCache = new WeakMap();"));
        assert!(PUZZLE3_APP_JS.contains("function spriteVoxelTemplate(spriteName)"));
        assert!(PUZZLE3_APP_JS.contains("function buildSpriteVoxelTemplate(sprite)"));
        assert!(PUZZLE3_APP_JS.contains("function instantiateSpriteVoxelTemplate(position, template, sourceKey = null, objectOrder = 0)"));
        assert!(PUZZLE3_APP_JS.contains("spriteVoxelTemplateCache.get(sprite)"));
        assert!(PUZZLE3_APP_JS.contains("spriteVoxelTemplateCache.set(sprite, template)"));
        assert!(PUZZLE3_APP_JS.contains("localBounds: voxelBounds(localPosition, scale)"));
        assert!(PUZZLE3_APP_JS.contains("x: (grid.x + 0.5 - width / 2) * scale"));
        assert!(PUZZLE3_APP_JS.contains("y: (grid.y + 0.5 - depth / 2) * scale"));
        assert!(PUZZLE3_APP_JS.contains("z: (grid.z + 0.5 - height / 2) * scale"));
        assert!(PUZZLE3_APP_JS.contains("const source = voxel.color || parseColor(voxel.fill);"));
    }

    #[test]
    fn puzzle3_app_caches_render_geometry_by_dirty_cells() {
        assert!(
            PUZZLE3_APP_JS.contains("const renderGeometryCache = createRenderGeometryCache();")
        );
        assert!(PUZZLE3_APP_JS.contains("function syncRenderGeometryCache(renderContext = null)"));
        assert!(PUZZLE3_APP_JS.contains("function renderCellSignature(cell)"));
        assert!(PUZZLE3_APP_JS.contains("function expandDirtyCellKeys(keys)"));
        assert!(PUZZLE3_APP_JS.contains("for (const offset of faceNeighborOffsets())"));
        assert!(
            PUZZLE3_APP_JS.contains("function rebuildVisibleCellGeometry(key, cell, signature)")
        );
        assert!(PUZZLE3_APP_JS.contains("function rebuildCachedCellFaces(key, cell)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("renderGeometryCache.occupied = renderCachedOpaqueOcclusion();")
        );
        assert!(
            PUZZLE3_APP_JS
                .contains("function cellFaceGeometriesForRender(cell, renderContext = null)")
        );
        assert!(PUZZLE3_APP_JS.contains("faces.push(...cellFaceGeometriesForRender(cell, renderContext).map(projectFaceGeometry));"));
        assert!(PUZZLE3_APP_JS.contains("face: (group, rect) => faceGeometry("));
        assert!(PUZZLE3_APP_JS.contains("function projectFaceGeometry(geometry)"));
        assert!(PUZZLE3_APP_JS.contains("const primitive = geometry.primitive || {"));
        assert!(PUZZLE3_APP_JS.contains("geometry.primitive = primitive;"));
        assert!(
            PUZZLE3_APP_JS
                .contains("primitive.ownerCell = projectCellRenderOwner(geometry.ownerCell);")
        );
        assert!(!PUZZLE3_APP_JS.contains("compoundFace:"));
        assert!(!PUZZLE3_APP_JS.contains("function compoundPolygonPaths(paths, fill)"));
    }

    #[test]
    fn mixed_export_rejects_puzzle3_scene_component() {
        let source = r#"
title = Mixed Game

puzzle flat {
layers {
actor = Player
}
rules {

}
}

levels flat_levels of flat {
legend {
. = empty
P = Player
}
level "start" {
P
}
}

puzzle3 cube {
  layers {
    actor = Player Box Wall
  }

  groups {
    solid = Player Box Wall
  }

  rules {

  }
}

levels3 cube_levels of cube {
  legend {
    . = empty
    P = Player
  }

  level "start" {
    P
  }
}

scene mixed_play {
  layout {
    row {
      puzzle flat_board = flat
      puzzle3 cube_board = cube
    }
}
}
"#;
        let error = puzzle_lang::parse_game(source).unwrap_err().to_string();

        assert!(error.contains("mixed 2D/3D documents are no longer supported"));
    }

    #[test]
    fn mixed_microban_scene_metadata_is_rejected_before_export() {
        let source = r#"
title = Mixed Microban

puzzle microban2d {
layers {
actor = Player
}
rules {

}
}

levels microban of microban2d {
legend {
. = empty
P = Player
}

level "microban_01" {
P.
}

level "microban_02" {
.P
}
}

puzzle3 microban3d {
layers {
actor = Player
}
rules {

}
}

levels3 microban of microban3d {
legend {
. = empty
P = Player
}

level "microban_03" {
P.
}

level "microban_04" {
.P
}
}

scene level_select {
layout {
title = "Microban"
column {
for level in levels {
choice join(level.num, ". ", level.title) -> goto playing(level)
}
}
}
}

scene playing(level) {
layout {
text level.title
}
}
"#;
        let error = puzzle_lang::parse_game(source).unwrap_err().to_string();

        assert!(error.contains("mixed 2D/3D documents are no longer supported"));
    }

    #[test]
    fn puzzle3_app_does_not_own_scene_component_rendering() {
        assert!(!PUZZLE3_APP_JS.contains("function renderSceneNode("));
        assert!(!PUZZLE3_APP_JS.contains("function renderSceneContainer("));
        assert!(!PUZZLE3_APP_JS.contains("function renderSceneFor("));
        assert!(!PUZZLE3_APP_JS.contains("function measureSceneNode("));
        assert!(!PUZZLE3_APP_JS.contains("scene-component-"));
        assert!(!PUZZLE3_APP_JS.contains("component.kind === \"button\""));
        assert!(!PUZZLE3_APP_JS.contains("component.kind === \"choice\""));
    }

    #[test]
    fn puzzle3_lifecycle_effect_semantics_are_host_owned() {
        assert!(PUZZLE3_APP_JS.contains("function emitPuzzle3LifecycleEffects("));
        assert!(PUZZLE3_APP_JS.contains("controllerOptions.onLifecycleEffects"));
        assert!(APP_JS.contains("function sendPuzzle3LifecycleEffects("));
        assert!(APP_JS.contains("await sendEffect(effect?.effect || effect, scope);"));
        assert!(APP_JS.contains("function puzzleEffectCommand("));
        assert!(!PUZZLE3_APP_JS.contains("function applyRuntimeLifecycleEffect("));
        assert!(!PUZZLE3_APP_JS.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!PUZZLE3_APP_JS.contains("effect.kind === \"message\""));
        assert!(!PUZZLE3_APP_JS.contains("message-popup"));
    }

    #[test]
    fn puzzle3_app_camera_pitch_allows_vertical_view() {
        assert!(PUZZLE3_APP_JS.contains("const PUZZLE3_APP_CAMERA_MIN_PITCH_DEGREES = -90;"));
        assert!(PUZZLE3_APP_JS.contains("const PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES = 90;"));
        assert!(PUZZLE3_APP_JS.contains("PUZZLE3_APP_CAMERA_MAX_PITCH_DEGREES"));
        assert!(!PUZZLE3_APP_JS.contains("camera.pitchDegrees - deltaY * 0.25, -80, 80"));
    }

    #[test]
    fn puzzle3_visual_core_owns_render_order_helpers() {
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function comparePrimitiveOrder(a, b)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function faceGridOrder(corners, view)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function directionDepth(vector, view)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function cameraOrderKey(view)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function cameraOrderBasis(view)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("plane: signed.x + signed.y + signed.z"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("const axes = [\"x\", \"y\", \"z\"].sort"));
        assert!(
            PUZZLE3_VISUAL_CORE_JS
                .contains("const faceRects = adapter.rectsFromCells || rectsFromCells;")
        );
        assert!(!PUZZLE3_VISUAL_CORE_JS.contains("adapter.compoundFace"));
        assert!(!PUZZLE3_VISUAL_CORE_JS.contains("const depthDiff ="));
        assert!(PUZZLE3_APP_JS.contains("primitives = orderScenePrimitives(primitives);"));
        assert!(PUZZLE3_APP_JS.contains("return Puzzle3VisualCore.comparePrimitiveOrder(a, b);"));
        assert!(PUZZLE3_APP_JS.contains("function orderScenePrimitives(primitives)"));
        assert!(PUZZLE3_APP_JS.contains(
            "view.primitiveSortCacheOrder.map((stableKey) => byStableKey.get(stableKey))"
        ));
        assert!(PUZZLE3_APP_JS.contains("primitive.frameIndex = index;"));
        assert!(PUZZLE3_APP_JS.contains("primitive.stableKey = occurrence === 0 ? baseKey"));
        assert!(PUZZLE3_APP_JS.contains("function primitiveSortCacheKey(primitives)"));
        assert!(PUZZLE3_APP_JS.contains("cameraOrderKey(),"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("compareNumber(a.frameIndex, b.frameIndex)"));
        assert!(
            PUZZLE3_APP_JS
                .contains("return Puzzle3VisualCore.cameraOrderKey(puzzle3VisualView());")
        );
        assert!(
            PUZZLE3_APP_JS
                .contains("return Puzzle3VisualCore.faceGridOrder(corners, puzzle3VisualView());")
        );
    }

    #[test]
    fn puzzle3_app_applies_pixelate_as_canvas_postprocess() {
        assert!(
            PUZZLE3_APP_JS.contains("const pixelateBuffer = document.createElement(\"canvas\");")
        );
        assert!(PUZZLE3_APP_JS.contains("applyPixelatePostprocess();"));
        assert!(PUZZLE3_APP_JS.contains("function pixelateSettings()"));
        assert!(PUZZLE3_APP_JS.contains(
            "const raw = snapshot.settings?.pixelate ?? snapshot.settings?.pixel ?? false;"
        ));
        assert!(PUZZLE3_APP_JS.contains("function applyPixelatePostprocess()"));
        assert!(PUZZLE3_APP_JS.contains("bufferCtx.imageSmoothingEnabled = settings.smoothing;"));
        assert!(PUZZLE3_APP_JS.contains("ctx.imageSmoothingEnabled = false;"));
        assert!(PUZZLE3_APP_JS.contains("ctx.setTransform(1, 0, 0, 1, 0, 0);"));
    }

    #[test]
    fn standalone_again_turns_are_scheduled_between_snapshots() {
        assert!(STANDALONE_JS.contains("this.sessionRuntime.request_json(method, url)"));
        assert!(!STANDALONE_JS.contains("scheduleAgainTurn"));
        assert!(!STANDALONE_JS.contains("runAgainTurn"));
        assert!(!STANDALONE_JS.contains("pendingAgainTurns"));
    }

    #[test]
    fn standalone_runtime_accepts_parenthesized_level_goto_commands() {
        assert!(STANDALONE_JS.contains("applyCommandName(commandName)"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.apply_command_name(commandName)"));
        assert!(!STANDALONE_JS.contains("parseRuntimeSceneTarget(value)"));
        assert!(!STANDALONE_JS.contains("parseRuntimeExpr"));
    }

    #[test]
    fn editor_preview_input_hook_does_not_swallow_session_commands() {
        assert!(APP_JS.contains("function isStandaloneEditorSessionCommand(command)"));
        assert!(APP_JS.contains(r#"name === "undo" || name === "redo" || name === "restart""#));
        assert!(APP_JS.contains("if (isStandaloneEditorSessionCommand(command))"));
    }

    #[test]
    fn standalone_runtime_requires_wasm_game_runtime_for_play() {
        let load_index = STANDALONE_JS
            .find("await this.loadRuntimeModule();")
            .unwrap();
        let session_index = STANDALONE_JS
            .find("this.initializeSessionRuntime()")
            .unwrap();
        assert!(load_index < session_index);
        assert!(STANDALONE_JS.contains("Puzzle game WASM runtime is unavailable."));
        assert!(!STANDALONE_JS.contains("this.initializeCoreRuntime();"));
        assert!(!STANDALONE_JS.contains("WasmCoreRuntime"));
        assert!(!STANDALONE_JS.contains("WasmCompiledCoreRuntime"));
        assert!(!STANDALONE_JS.contains("using JavaScript transition fallback"));
        assert!(!STANDALONE_JS.contains("projection failed; using source state"));
        assert!(!STANDALONE_JS.contains("JavaScript transition programs are unsupported."));
        assert!(!STANDALONE_JS.contains("materializeDisplayProgram"));
        assert!(!STANDALONE_JS.contains("presentationSnapshotForState"));
        assert!(!STANDALONE_JS.contains("normalizeAnimationEvents"));
        assert!(!STANDALONE_JS.contains("animationsForCoreOutcome"));
        assert!(!STANDALONE_JS.contains("animateEmissions"));
        assert!(
            APP_JS.contains(
                "screenHasPuzzle: currentSceneAcceptsModelInput() || Boolean(state.scene)"
            )
        );
        assert!(APP_JS.contains("function currentSceneAcceptsModelInput()"));
        assert!(
            APP_JS.contains(
                "function sceneInteractionProfile(scene = currentSceneDef(), options = {})"
            )
        );
        assert!(
            APP_JS.contains(
                "function stateAcceptsModelInput(state = currentState || puzzleBoot || {})"
            )
        );
        assert!(APP_JS.contains("state?.acceptsModelInput === true"));
        assert!(APP_JS.contains("standaloneRuntime?.editorPreviewInputEnabled === true"));
        assert!(!APP_JS.contains("nonEmptyArray(layer?.scenePuzzles)"));
        assert!(APP_JS.contains("function sceneChromeProfile(profile)"));
        assert!(APP_JS.contains("effects.push({ kind: \"model_input\", name: input.name });"));
        assert!(APP_JS.contains("await sendModelInput(effect.name);"));
        assert!(APP_JS.contains("return post(`/api/input/${encodeURIComponent(input)}`);"));
        assert!(!APP_JS.contains("sceneIsMenuLike"));
        assert!(!APP_JS.contains("const hasPuzzle = sceneHasComponent(sceneDef, \"puzzle\") || sceneHasComponent(sceneDef, \"frame\")"));
        assert!(APP_JS.contains("acceptModelInput: event.data.acceptModelInput === true"));
        assert!(APP_JS.contains("function applyStandaloneEditorInput(command)"));
        assert!(
            APP_JS.contains(
                "const acceptsEditorInput = standaloneRuntime?.editorPreviewInputEnabled"
            )
        );
        assert!(APP_JS.contains("standaloneRuntime?.inputIdsByName?.has(command)"));
        assert!(APP_JS.contains("standaloneRuntime.applyInputName(command);"));
        assert!(STANDALONE_JS.contains("this.editorPreviewInputEnabled = false;"));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_task_request_solves_compiled_2d_without_source_fallback() {
        let source = r#"
title = solver_task_compiled

puzzle board {
  layers {
    floor = Goal
    actor = Player Box Wall
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Box | Goal no actor ] -> [ | Player | Goal Box ]
  }
  win_conditions {
    all Goal on Box
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    B = Box
    G = Goal
  }
  level "start" {
    PBG
  }
}
"#;
        let html = export_editor_preview_html_from_source(source, "game.puzzle", "", "")
            .expect("preview export");
        let standalone =
            export_html_from_source(source, "game.puzzle", "", "").expect("standalone export");
        assert!(!html.contains("PuzzleEditorSolverRulesJson"));
        assert!(!standalone.contains("PuzzleEditorSolverRulesJson"));
        let export = embedded_puzzle_runtime_export_json(&html);
        let solver_rules = prepared_editor_solver_rules_json(source, "game.puzzle");
        assert!(export["runtimeLoadedGame"]["loaded"]["solver_strategy"].is_null());
        assert!(
            export["compiledPlay"]["inputLabels"]
                .as_object()
                .expect("compiled input labels")
                .values()
                .any(|label| label == "right")
        );
        let level = &export["levels"][0];
        let request = json!({
            "version": 1,
            "rules": {
                "compileId": "test-compile",
                "documentId": "test-document",
                "modelKind": solver_rules["modelKind"].clone(),
                "compiledPlay": solver_rules["compiledPlay"].clone(),
                "runRulesOnLevelStart": solver_rules["runRulesOnLevelStart"].clone(),
                "goal": solver_rules["goal"].clone(),
                "lose": solver_rules["lose"].clone(),
                "solverStrategy": solver_rules["solverStrategy"].clone()
            },
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": level["name"].clone()
                },
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": level["initialState"].clone()
                }
            },
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0
        });

        let response = solve_solver_task_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#), "{response}");
        assert!(response.contains(r#""depth":1"#));
        assert!(response.contains(r#""name":"right""#));
        assert!(!response.contains("input_"));
        assert!(!request.to_string().contains("source"));
    }

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    #[test]
    fn editor_solver_task_solves_microban_5_with_generated_strategy() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../games/microban/game.puzzle");
        let source = std::fs::read_to_string(&path).expect("microban source");
        let html =
            export_editor_preview_html_from_source(&source, &path.display().to_string(), "", "")
                .expect("microban editor preview");
        let export = embedded_puzzle_runtime_export_json(&html);
        let solver_rules = prepared_editor_solver_rules_json(&source, &path.display().to_string());
        let level = &export["levels"][4];
        assert_eq!(level["name"], "microban_05");
        assert_eq!(
            solver_rules["solverStrategy"]["terms"][0]["value"],
            json!({"AllOnDistance":{"subjects":[2],"covers":[4]}})
        );
        let request = json!({
            "version": 1,
            "rules": {
                "compileId": "microban-5",
                "documentId": "microban",
                "modelKind": solver_rules["modelKind"].clone(),
                "compiledPlay": solver_rules["compiledPlay"].clone(),
                "runRulesOnLevelStart": solver_rules["runRulesOnLevelStart"].clone(),
                "goal": solver_rules["goal"].clone(),
                "lose": solver_rules["lose"].clone(),
                "solverStrategy": solver_rules["solverStrategy"].clone()
            },
            "target": {
                "origin": "preview-level",
                "compileId": "microban-5",
                "documentId": "microban",
                "level": {"index": 4, "levelName": "microban_05"},
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": level["initialState"].clone()
                }
            },
            "maxDepth": 512,
            "maxNodes": 100_000,
            "maxMs": 0
        });

        let response = solve_solver_task_json(&request.to_string()).expect("editor solver task");
        assert!(response.contains(r#""result":"solved""#), "{response}");
        let response_json: Value = serde_json::from_str(&response).expect("solver response JSON");
        let expanded = response_json["observations"]
            .as_array()
            .and_then(|observations| observations.last())
            .and_then(|observation| observation["progress"]["expanded"].as_u64())
            .expect("solved response should retain final search progress");
        let visited = response_json["observations"]
            .as_array()
            .and_then(|observations| observations.last())
            .and_then(|observation| observation["progress"]["visited"].as_u64())
            .expect("solved response should retain visited search progress");
        eprintln!("microban_05 editor solver visited={visited} expanded={expanded}");
        assert!(visited < 35_000, "visited {visited} positions");
        assert!(expanded < 100_000, "expanded {expanded} positions");
    }

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    #[test]
    fn editor_solver_task_preserves_teneten_routine_control_flow() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/TENETEN.puzzle");
        let source = std::fs::read_to_string(&path).expect("TENETEN source");
        let html =
            export_editor_preview_html_from_source(&source, &path.display().to_string(), "", "")
                .expect("TENETEN editor preview");
        let export = embedded_puzzle_runtime_export_json(&html);
        let solver_rules = prepared_editor_solver_rules_json(&source, &path.display().to_string());
        let level = &export["levels"][1];
        assert_eq!(level["name"], "1-2");
        let request = json!({
            "version": 1,
            "rules": {
                "compileId": "teneten-1-2",
                "documentId": "teneten",
                "modelKind": solver_rules["modelKind"].clone(),
                "compiledPlay": solver_rules["compiledPlay"].clone(),
                "runRulesOnLevelStart": solver_rules["runRulesOnLevelStart"].clone(),
                "goal": solver_rules["goal"].clone(),
                "lose": solver_rules["lose"].clone(),
                "solverStrategy": solver_rules["solverStrategy"].clone()
            },
            "target": {
                "origin": "preview-level",
                "compileId": "teneten-1-2",
                "documentId": "teneten",
                "level": {"index": 1, "levelName": "1-2"},
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": level["initialState"].clone()
                }
            },
            "maxDepth": 128,
            "maxNodes": 100_000,
            "maxMs": 0
        });

        let response = solve_solver_task_json(&request.to_string()).expect("TENETEN solver task");
        let response: Value = serde_json::from_str(&response).expect("solver response JSON");
        assert_eq!(response["result"], "solved", "{response}");
        assert_eq!(response["depth"], 14);
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_compiled_slicer_uses_stage_availability() {
        let source = r#"
title = solver_compiled_stage_availability

puzzle board {
  layers {
    floor = Goal
    actor = Player Box
    item = Battery Door
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Box | Goal no actor ] -> [ | Player | Goal Box ]
    [ Battery ] -> [ Door ]
  }
  win_conditions {
    all Goal on Box
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    B = Box
    G = Goal
    X = Battery
  }
  level "start" {
    PBG.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_named = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(*id))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let battery = object_named("Battery");
        let initial = &loaded.levels[0].initial_state;
        let goal = loaded.goal.as_ref().map(|goal| &goal.expr);
        let lose = loaded.lose.as_ref().map(|lose| &lose.expr);
        let (solver_game, slicer) = solver_game_and_state_slicer_for_compiled(
            loaded.game.clone(),
            initial,
            goal,
            lose,
            &loaded.solver_strategy,
        );
        let mut probe = initial.clone();
        probe.place_object(&solver_game, 3, 0, battery).unwrap();

        let projected = slicer.project_state(&probe);

        assert!(!projected.has_object(&solver_game, 3, 0, battery));
        let rule_ids = solver_game
            .rules()
            .iter()
            .map(|rule| rule.id)
            .collect::<Vec<_>>();
        let battery_rule = loaded
            .rule_debug_info
            .iter()
            .find_map(|(rule, info)| {
                (info.source_line == "[ Battery ] -> [ Door ]").then_some(*rule)
            })
            .expect("battery rule debug info");
        assert!(!rule_ids.contains(&battery_rule));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_task_request_treats_win_command_as_goal_without_win_conditions() {
        let source = r#"
title = solver_task_win_command

puzzle board {
  layers {
    actor = Player Exit
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Exit ] -> win
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    E = Exit
  }
  level "start" {
    PE
  }
}
"#;
        let html = export_editor_preview_html_from_source(source, "game.puzzle", "", "")
            .expect("preview export");
        let export = embedded_puzzle_runtime_export_json(&html);
        assert!(export["goal"].is_null());
        assert!(export["compiledPlay"].is_object());
        assert!(export["engine"]["objects"].is_array());
        let level = &export["levels"][0];
        let request = json!({
            "version": 1,
            "rules": {
                "compileId": "test-compile",
                "documentId": "test-document",
                "modelKind": "2d",
                "compiledPlay": export["compiledPlay"].clone(),
                "runRulesOnLevelStart": export["engine"]["runRulesOnLevelStart"].clone(),
                "goal": export["goal"].clone(),
                "lose": export["lose"].clone()
            },
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": level["name"].clone()
                },
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": level["initialState"].clone()
                }
            },
            "maxDepth": 2,
            "maxNodes": 100,
            "maxMs": 0
        });

        let response = solve_solver_task_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#), "{response}");
        assert!(response.contains(r#""depth":1"#));
        assert!(!request.to_string().contains("source"));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_task_accepts_locked_win_command_sample() {
        let source = include_str!("../tests/fixtures/locked.puzzle");
        let html = export_editor_preview_html_from_source(source, "locked.puzzle", "", "")
            .expect("preview export");
        let export = embedded_puzzle_runtime_export_json(&html);
        assert!(export["goal"].is_null());
        assert!(export["compiledPlay"].is_object());
        let level = &export["levels"][0];
        let request = json!({
            "version": 1,
            "rules": {
                "compileId": "test-compile",
                "documentId": "test-document",
                "modelKind": "2d",
                "compiledPlay": export["compiledPlay"].clone(),
                "runRulesOnLevelStart": export["engine"]["runRulesOnLevelStart"].clone(),
                "goal": export["goal"].clone(),
                "lose": export["lose"].clone()
            },
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": level["name"].clone()
                },
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": level["initialState"].clone()
                }
            },
            "maxDepth": 80,
            "maxNodes": 200_000,
            "maxMs": 0
        });

        let response = solve_solver_task_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""observations":["#));
        assert!(response.contains(r#""progress":{"#));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_materializes_level_start_for_editor_state_with_level_index() {
        let source = r#"
title = solver_level_start

puzzle board {
  layers {
    floor = Goal
    actor = Player
  }
  keys {
    Space -> noop
  }
  rules {
    if input == noop {
      [ Player ] -> [ Player ]
    }
  }
  on_level_start {
    [ Goal no Player ] -> [ Goal Player ]
  }
  win_conditions {
    all Goal on Player
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    G = Goal
  }
  level "start" {
    PG
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);
        state_json.pop();
        write!(&mut state_json, r#","levelIndex":0}}"#).unwrap();

        let response =
            solve_state_json_from_source(source, "game.puzzle", &state_json, 0, 1000, 0).unwrap();

        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""depth":0"#));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_materializes_level_start_from_explicit_target() {
        let source = r#"
title = solver_request_level_start

puzzle board {
  layers {
    floor = Goal
    actor = Player
  }
  keys {
    Space -> noop
  }
  rules {
    if input == noop {
      [ Player ] -> [ Player ]
    }
  }
  on_level_start {
    [ Goal no Player ] -> [ Goal Player ]
  }
  win_conditions {
    all Goal on Player
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    G = Goal
  }
  level "start" {
    PG
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);
        let state: Value = serde_json::from_str(&state_json).unwrap();
        let request = json!({
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "level-editor",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "editor-staged",
                    "lifecycle": "playable-start",
                    "data": state,
                },
            },
            "maxDepth": 0,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""depth":0"#));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_accepts_level_ascii_state() {
        let source = r#"
title = solver_request_level_ascii

puzzle board {
  layers {
    floor = Goal
    actor = Player Box Wall
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Box | Goal no actor ] -> [ | Player | Goal Box ]
  }
  win_conditions {
    all Goal on Box
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    B = Box
    G = Goal
  }
  level "start" {
    PBG
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_id = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(id.0))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let request = json!({
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [object_id("Player")],
                            "B": [object_id("Box")],
                            "G": [object_id("Goal")]
                        },
                        "lines": ["PBG"]
                    },
                },
            },
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#), "{response}");
        assert!(response.contains(r#""depth":1"#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_accepts_exact_state_goal() {
        let source = r#"
title = solver_request_exact_state_goal

puzzle board {
  layers {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_id = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(id.0))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let player = object_id("Player");
        let state_spec = |lines: Vec<&str>| {
            json!({
                "kind": "level-ascii",
                "data": {
                    "empty": ".",
                    "legend": {
                        ".": [],
                        "P": [player]
                    },
                    "lines": lines
                }
            })
        };
        let request = json!({
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [player]
                        },
                        "lines": ["P."]
                    },
                },
            },
            "goal": {
                "kind": "exact-state",
                "state": state_spec(vec![".P"])
            },
            "acceptWinCommand": false,
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"solved""#), "{response}");
        assert!(response.contains(r#""depth":1"#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn exact_state_heuristic_uses_start_goal_object_distance() {
        let source = r#"
title = solver_exact_state_heuristic

puzzle board {
  layers {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P..
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let player = loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == "Player").then_some(*id))
            .unwrap_or_else(|| panic!("missing object Player"));
        let mut legend = HashMap::new();
        legend.insert('.', Vec::new());
        legend.insert('P', vec![player]);
        let parse_state = |line: &str| {
            puzzle_lang::parse_level_ascii_state(
                &loaded.game,
                &[line.to_string()],
                '.',
                &legend,
                loaded.levels[0].initial_state.visible_variables(),
            )
            .map(|(state, _)| state)
            .unwrap()
        };
        let initial = parse_state("P..");
        let middle = parse_state(".P.");
        let goal = parse_state("..P");
        let heuristic = ExactStateHeuristic::new(&loaded.game, &initial, &goal);

        assert!(heuristic.score(&middle) < heuristic.score(&initial));
        assert_eq!(heuristic.score(&goal), 0);
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_loaded_slicer_uses_stage_availability() {
        let source = r#"
title = solver_stage_availability

puzzle board {
  layers {
    actor = Player
    item = Switch Battery Door
  }
  rules {
    [ Switch ] -> [ Door ]
    [ Battery ] -> [ Door ]
  }
  win_conditions {
    some Door
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    S = Switch
    B = Battery
  }
  level "start" {
    PS.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_named = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(*id))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let switch = object_named("Switch");
        let battery = object_named("Battery");
        let solver_game = loaded.solver_game();
        let initial = &loaded.levels[0].initial_state;
        let (solver_game, slicer) = solver_game_and_state_slicer_for_loaded(
            &loaded,
            solver_game,
            initial,
            None,
            None,
            None,
        );
        let mut probe = initial.clone();
        probe.place_object(&solver_game, 2, 0, battery).unwrap();

        let projected = slicer.project_state(&probe);

        assert!(projected.has_object(&solver_game, 1, 0, switch));
        assert!(!projected.has_object(&solver_game, 2, 0, battery));
        let rule_ids = solver_game
            .rules()
            .iter()
            .map(|rule| rule.id)
            .collect::<Vec<_>>();
        assert_eq!(rule_ids.len(), 2);
        assert!(rule_ids.contains(&RuleId(1)));
        assert!(rule_ids.contains(&RuleId(3)));
        assert!(!rule_ids.contains(&RuleId(2)));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_collect_slicer_uses_stage_availability() {
        let source = r#"
title = solver_collect_stage_availability

puzzle board {
  layers {
    actor = Player
    item = Switch Battery Door
  }
  rules {
    [ Switch ] -> [ Door ]
    [ Battery ] -> [ Door ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    S = Switch
    B = Battery
  }
  level "start" {
    PS.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_named = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(*id))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let switch = object_named("Switch");
        let battery = object_named("Battery");
        let door = object_named("Door");
        let selector = SolverCollectSelector2::Maximize(GoalValue::InlineConditionValue(
            ConditionValueKind::CountObjects(vec![door]),
        ));
        let initial = &loaded.levels[0].initial_state;
        let (solver_game, slicer) = solver_game_and_state_slicer_for_collect(
            &loaded,
            loaded.solver_game(),
            initial,
            &selector,
            None,
        );
        let mut probe = initial.clone();
        probe.place_object(&solver_game, 2, 0, battery).unwrap();

        let projected = slicer.project_state(&probe);

        assert!(projected.has_object(&solver_game, 1, 0, switch));
        assert!(!projected.has_object(&solver_game, 2, 0, battery));
        let rule_ids = solver_game
            .rules()
            .iter()
            .map(|rule| rule.id)
            .collect::<Vec<_>>();
        assert_eq!(rule_ids.len(), 2);
        assert!(rule_ids.contains(&RuleId(1)));
        assert!(rule_ids.contains(&RuleId(3)));
        assert!(!rule_ids.contains(&RuleId(2)));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_exact_state_slicer_uses_stage_availability() {
        let source = r#"
title = solver_exact_stage_availability

puzzle board {
  layers {
    actor = Player
    item = Battery Door
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
    [ Battery ] -> [ Door ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    B = Battery
  }
  level "start" {
    P..
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_named = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(*id))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let player = object_named("Player");
        let battery = object_named("Battery");
        let mut legend = HashMap::new();
        legend.insert('.', Vec::new());
        legend.insert('P', vec![player]);
        let (goal, _) = puzzle_lang::parse_level_ascii_state(
            &loaded.game,
            &[".P.".to_string()],
            '.',
            &legend,
            loaded.levels[0].initial_state.visible_variables(),
        )
        .unwrap();
        let initial = &loaded.levels[0].initial_state;
        let (solver_game, slicer) = solver_game_and_state_slicer_for_loaded(
            &loaded,
            loaded.solver_game(),
            initial,
            Some(&goal),
            None,
            None,
        );
        let mut probe = initial.clone();
        probe.place_object(&solver_game, 2, 0, battery).unwrap();

        let projected = slicer.project_state(&probe);

        assert!(projected.has_object(&solver_game, 0, 0, player));
        assert!(!projected.has_object(&solver_game, 2, 0, battery));
        let rule_ids = solver_game
            .rules()
            .iter()
            .map(|rule| rule.id)
            .collect::<Vec<_>>();
        assert!(rule_ids.contains(&RuleId(1)));
        assert!(rule_ids.contains(&RuleId(3)));
        assert!(!rule_ids.contains(&RuleId(2)));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_supports_reachability_task() {
        let source = r#"
title = solver_request_reachability

puzzle board {
  layers {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let player = loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == "Player").then_some(id.0))
            .unwrap_or_else(|| panic!("missing object Player"));
        let state_spec = |lines: Vec<&str>| {
            json!({
                "kind": "level-ascii",
                "data": {
                    "empty": ".",
                    "legend": {
                        ".": [],
                        "P": [player]
                    },
                    "lines": lines
                }
            })
        };
        let request = json!({
            "task": "reachability",
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [player]
                        },
                        "lines": ["P."]
                    },
                },
            },
            "goal": {
                "kind": "exact-state",
                "state": state_spec(vec![".P"])
            },
            "acceptWinCommand": false,
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""task":"reachability""#), "{response}");
        assert!(response.contains(r#""result":"reachable""#), "{response}");
        assert!(response.contains(r#""reachable":true"#), "{response}");
        assert!(response.contains(r#""cost":{"steps":1}"#), "{response}");
        assert!(response.contains(r#""path":["#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_reachability_requires_explicit_goal() {
        let source = r#"
title = solver_request_reachability_requires_goal

puzzle board {
  layers {
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P.
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let player = loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == "Player").then_some(id.0))
            .unwrap_or_else(|| panic!("missing object Player"));
        let request = json!({
            "task": "reachability",
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [player]
                        },
                        "lines": ["P."]
                    },
                },
            },
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let error = solve_request_json(&request.to_string()).unwrap_err();

        assert!(error.contains("reachability requests require an explicit goal"));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_collects_predicate_matches() {
        let source = r#"
title = solver_request_collect_predicate

puzzle board {
  layers {
    floor = Trail
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ Trail | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P...
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_id = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(id.0))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let player = object_id("Player");
        let trail = object_id("Trail");
        let count_trails = json!({
            "kind": "condition_value",
            "conditionValueKind": {
                "kind": "count_objects",
                "objects": [trail]
            }
        });
        let request = json!({
            "task": "collect",
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [player]
                        },
                        "lines": ["P..."]
                    },
                },
            },
            "collect": {
                "kind": "predicate",
                "maxResults": 1,
                "predicate": {
                    "expr": {
                        "kind": "clause",
                        "value": count_trails,
                        "op": "greater_eq",
                        "expected": 1
                    }
                }
            },
            "acceptWinCommand": false,
            "maxDepth": 4,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""task":"collect""#), "{response}");
        assert!(
            response.contains(r#""result":"limit_reached""#),
            "{response}"
        );
        assert!(response.contains(r#""count":1"#), "{response}");
        assert!(response.contains(r#""matches":["#), "{response}");
        assert!(response.contains(r#""score":null"#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_collects_maximized_objective_matches() {
        let source = r#"
title = solver_request_collect_maximize

puzzle board {
  layers {
    floor = Trail
    actor = Player
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | no actor ] -> [ Trail | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
  }
  level "start" {
    P...
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let object_id = |name: &str| {
            loaded
                .object_labels
                .iter()
                .find_map(|(id, label)| (label == name).then_some(id.0))
                .unwrap_or_else(|| panic!("missing object {name}"))
        };
        let player = object_id("Player");
        let trail = object_id("Trail");
        let count_trails = json!({
            "kind": "condition_value",
            "conditionValueKind": {
                "kind": "count_objects",
                "objects": [trail]
            }
        });
        let request = json!({
            "task": "collect",
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "level-ascii",
                    "lifecycle": "already-materialized",
                    "data": {
                        "empty": ".",
                        "legend": {
                            ".": [],
                            "P": [player]
                        },
                        "lines": ["P..."]
                    },
                },
            },
            "collect": {
                "kind": "maximize",
                "maxResults": 1,
                "objective": {
                    "value": count_trails
                }
            },
            "acceptWinCommand": false,
            "maxDepth": 3,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""task":"collect""#), "{response}");
        assert!(response.contains(r#""count":1"#), "{response}");
        assert!(response.contains(r#""score":3"#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_treats_win_command_as_goal() {
        let source = r#"
title = solver_win_command

puzzle board {
  layers {
    actor = Player Exit
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Exit ] -> win
    input right [ Player | no actor ] -> [ | Player ]
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    E = Exit
  }
  level "start" {
    PE
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);

        let response =
            solve_state_json_from_source(source, "game.puzzle", &state_json, 2, 100, 0).unwrap();

        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""depth":1"#));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_can_disable_win_command_for_explicit_goal() {
        let source = r#"
title = solver_explicit_goal_without_win

puzzle board {
  layers {
    floor = Flag
    actor = Player Exit
  }
  keys {
    d ArrowRight -> right
  }
  rules {
    input right [ Player | Exit ] -> win
    input right [ Player | no actor ] -> [ | Player ]
  }
  win_conditions {
    all Flag on Player
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    E = Exit
    F = Flag
  }
  level "start" {
    PEF
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let html = export_editor_preview_html_from_source(source, "game.puzzle", "", "")
            .expect("preview export");
        let export = embedded_puzzle_runtime_export_json(&html);
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);
        let state: Value = serde_json::from_str(&state_json).unwrap();
        let request = json!({
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": loaded.levels[0].name,
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "already-materialized",
                    "data": state,
                },
            },
            "goal": export["goal"].clone(),
            "acceptWinCommand": false,
            "maxDepth": 1,
            "maxNodes": 100,
            "maxMs": 0,
        });

        let response = solve_request_json(&request.to_string()).unwrap();

        assert!(response.contains(r#""result":"exhausted""#), "{response}");
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_request_rejects_mismatched_level_identity() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);
        let state: Value = serde_json::from_str(&state_json).unwrap();
        let request = json!({
            "source": source,
            "puzzlePath": "game.puzzle",
            "modelKind": "2d",
            "target": {
                "origin": "preview-level",
                "compileId": "test-compile",
                "documentId": "test-document",
                "level": {
                    "index": 0,
                    "levelName": "not_the_compiled_level",
                    "levelPuzzle": loaded.levels[0].puzzle,
                    "levelPack": loaded.levels[0].pack,
                },
                "state": {
                    "kind": "compiled-start",
                    "lifecycle": "playable-start",
                    "data": state,
                },
            },
            "maxDepth": 8,
            "maxNodes": 1000,
            "maxMs": 0,
        });

        let error = solve_request_json(&request.to_string()).unwrap_err();

        assert!(error.contains("solver target levelName mismatch"));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_inputs_use_model_inputs_not_scene_or_control_inputs() {
        let source = r#"
title = solver_input_scope

puzzle board {
  layers {
    floor = Goal
    actor = Player Box Wall
  }
  keys {
    w ArrowUp -> up
    s ArrowDown -> down
    a ArrowLeft -> left
    d ArrowRight -> right
    r -> restart
  }
  rules {
    input directions [ Player | Box | no actor ] -> [ | Player | Box ]
    input directions [ Player | no actor ] -> [ | Player ]
  }
  win_conditions {
    all Goal on Box
  }
}

levels default of board {
  legend {
    . = empty
    P = Player
    B = Box
    G = Goal
  }
  level "one" {
    PBG
  }
}

scene title {
  layout {
    choice "New Game" -> input new_game
  }
  keys {
    n -> new_game
  }
  routine new_game {
    goto playing
  }
}

scene playing {
  keys {
    Escape -> back
  }
  routine back {
    goto title
  }
  layout {
    puzzle board = board
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let labels = solver_inputs(&loaded)
            .into_iter()
            .map(|input| loaded.input_labels.get(&input).unwrap().as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["up", "down", "left", "right"]);
    }

    #[cfg(feature = "solver")]
    #[test]
    fn solver_accepts_puzzle3d_state_and_returns_replay_steps() {
        let source = r#"
title = "Themed 3D Solver"

puzzle3 push3 {
layers {
floor = Goal
solid = Player Box Wall
}

groups {
solid = Player Box Wall
}

rules {
input right [ Player | Box | no solid ] -> [ | Player | Box ]
input right [ Player | no solid ] -> [ | Player ]
}

on_level_clear {
if win_conditions -> next_level
}

win_conditions {
some Goal
no down [ no Box | Goal ]
}

query box_to_goal = distance(Box, Goal)
query has_box = exists(Box)
query box_goal_lines = count([ Box Goal ])

solver {
strategy {
maximize box_goal_lines weight 2
minimize box_to_goal weight 3
prefer has_box
}
}
}

levels3 tiny of push3 {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level "one" {
PB.

..G
}
}
"#;

        let parsed = parse_puzzle3d_for_solver(source).unwrap();
        assert_eq!(parsed.solver_strategy.terms.len(), 3);
        let state = parsed
            .level_bundle
            .as_ref()
            .unwrap()
            .build_level_state(0)
            .unwrap();
        let slots = state
            .slots()
            .iter()
            .map(|object| object.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let state_json = format!(
            r#"{{"kind":"puzzle3d","width":{},"depth":{},"height":{},"layerCount":{},"slots":[{}]}}"#,
            state.size.width, state.size.depth, state.size.height, state.layer_count, slots
        );

        let response =
            solve_state_json_from_source(source, "game.puzzle3", &state_json, 4, 1000, 0).unwrap();

        assert!(response.contains(r#""model":"puzzle3d""#));
        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""name":"right""#));
        assert!(response.contains(r#""direction":"right""#));
        assert!(response.contains(r#""completed":true"#));
        assert!(response.contains(r#""clearCommands":[{"#));
        assert!(response.contains(r#""kind": "puzzle_next_level""#));
        assert!(response.contains(r#""scene":{"kind":"puzzle3d""#));
        assert!(!response.contains(r#""name":"restart""#));
    }

    #[test]
    fn core_runtime_bridge_uses_core_once_all_semantics() {
        let source = r#"
title = once_all_overlap

puzzle board {
  layers {
    tiles = A B
  }
  empty .
  rules {
    once_all [ A | A ] -> [ | B ]
  }
}

levels default of board {
  legend A = A
  legend B = B
  level "start" {
    AAA
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;
        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);

        let outcome =
            transition_program_outcome_json_from_source(source, "main", -1, &state_json, 0)
                .unwrap();
        let outcome_contract: RuntimeTransitionProgramOutcome =
            serde_json::from_str(&outcome).expect("2D program outcome should match contract");

        assert!(outcome.contains(r#""slots":[2,0,1]"#));
        assert!(!outcome_contract.completed);
    }

    #[test]
    fn standalone_export_embeds_game_wasm_runtime() {
        let source = r#"
	title = Wasm Export
again_interval = 90ms

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
    [ Player ] -> [ Player ]
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let html = export_html_from_source(source, "games/wasm_export/game.puzzle", "", "")
            .expect("export should succeed");

        assert!(html.contains("window.PuzzleStandaloneEmbeddedWasm"));
        assert!(html.contains("\\\"defaultAgainMs\\\":90"));
        assert!(html.contains("\\\"runtimeLoadedGame\\\""));
        assert!(html.contains("puzzle_wasm_game_bg.wasm"));
        assert!(html.contains("WasmStandaloneSession"));
        assert!(html.contains("WasmPuzzle3Runtime"));
        assert!(!html.contains("WasmCoreRuntime"));
        assert!(!html.contains("compile_preview"));
        assert!(!html.contains("highlight_source_html"));
        assert!(!html.contains("suggest_source_completions"));
        assert!(!html.contains("solve_state"));
        assert!(!html.contains("solve_state_with_progress"));
        assert!(!html.contains("puzzle_solver"));
        assert!(!html.contains("SearchBudget"));
        assert!(!html.contains("best_first"));
        assert!(html.contains("Uint8Array.fromBase64"));
        assert!(html.contains("embedded.wasmBase64 = \"\";"));
        assert!(!html.contains("atob("));
        assert!(!html.contains("base64ToUint8Array"));
        assert!(!html.contains("PuzzleStudioSolve"));
        assert!(!html.contains("PuzzleStudioPreviewState"));
        assert!(!html.contains("PuzzleStudioScenePreview"));
        assert!(!html.contains("loadWasmSolver"));
        assert!(!html.contains("renderPuzzle3Frame"));
        assert!(!html.contains("Puzzle3DFrameAssets"));
        assert!(html.contains("window.PuzzleBoot = JSON.parse("));
        assert!(html.contains("window.PuzzleRuntimeExportJson = "));
        assert!(!html.contains("window.PuzzleExport = JSON.parse("));
        assert!(!html.contains("window.PuzzleExportJson = "));
        let boot = embedded_puzzle_boot_json(&html);
        assert!(boot.get("runtimeLoadedGame").is_none());
        assert!(boot.get("compiledPlay").is_none());
        assert!(boot.get("engine").is_none());
        assert!(boot.get("source").is_none());
        assert!(boot.get("puzzlePath").is_none());
        assert!(boot["inputs"].is_array());
        assert!(boot["theme"].is_object());
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        assert!(runtime_export["runtimeLoadedGame"].is_object());
        assert!(runtime_export.get("compiledPlay").is_none());
        assert!(runtime_export.get("engine").is_none());
        assert!(runtime_export.get("source").is_none());
        assert!(STANDALONE_JS.contains("loadRuntimeModule()"));
        assert!(STANDALONE_JS.contains("initializeSessionRuntime()"));
        assert!(STANDALONE_JS.contains("WasmStandaloneSession.fromExport(this.exportJson)"));
        assert!(STANDALONE_JS.contains("releaseWasmOwnedExportPayload()"));
        assert!(STANDALONE_JS.contains("delete this.data.runtimeLoadedGame;"));
        assert!(!STANDALONE_JS.contains("WasmStandaloneSession.fromExport(JSON.stringify"));
        assert!(!STANDALONE_JS.contains("new this.wasmModule.WasmStandaloneSession("));
        assert!(STANDALONE_JS.contains("Puzzle game WASM runtime is unavailable."));
        assert!(STANDALONE_JS.contains("async setCurrentState(state, options = {})"));
        assert!(STANDALONE_JS.contains("await this.ensureInitialized();"));
        assert!(STANDALONE_JS.contains("set_current_state("));
        assert!(STANDALONE_JS.contains("Editor preview state requires a valid level index."));
        assert!(!STANDALONE_JS.contains("this.initializeCoreRuntime();"));
        assert!(!STANDALONE_JS.contains("WasmCoreRuntime"));
        assert!(!STANDALONE_JS.contains("WasmCompiledCoreRuntime"));
        assert!(PUZZLE_GAME_WASM_JS.contains("WasmStandaloneSession"));
        assert!(PUZZLE_GAME_WASM_JS.contains("WasmPuzzle3Runtime"));
        assert!(PUZZLE_GAME_WASM_JS.contains("fromFixture"));
        assert!(!PUZZLE_GAME_WASM_JS.contains("compile_preview"));
        assert!(!PUZZLE_GAME_WASM_JS.contains("solve_state"));
        assert!(!PUZZLE_GAME_WASM_JS.contains("solver"));
        assert!(!PUZZLE_GAME_WASM_JS.contains("puzzle_solver"));
        assert!(!bytes_contain(PUZZLE_GAME_WASM_BG, b"solve_state"));
        assert!(!bytes_contain(PUZZLE_GAME_WASM_BG, b"puzzle_solver"));
        assert!(!bytes_contain(PUZZLE_GAME_WASM_BG, b"SearchBudget"));
        assert!(!bytes_contain(PUZZLE_GAME_WASM_BG, b"best_first"));
    }

    #[test]
    fn editor_preview_runtime_bundle_serializes_scene_input_effects() {
        let source = r#"
title = Scene Input Runtime Bundle

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}

scene title {
  layout {
    choice "Continue" -> input continue_game
  }
  keys {
    Enter -> continue_game
  }
  routine continue_game {
    goto playing
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let html = export_editor_preview_html_from_source(source, "game.puzzle", "", "")
            .expect("editor preview should serialize scene input effects");
        let runtime_export = embedded_puzzle_runtime_export_json(&html);

        assert!(runtime_export["runtimeLoadedGame"].is_object());
        assert!(
            html.contains(r#"\"kind\":\"input\",\"name\":\"continue_game\""#),
            "runtime bundle should encode SceneEffect::Input as a named payload"
        );
    }

    #[test]
    fn standalone_export_can_embed_browser_supplied_game_runtime_assets() {
        let source = r#"
title = Browser Supplied Runtime

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let html = export_html_from_source_with_embedded_wasm(
            source,
            "game.puzzle",
            "",
            "",
            "export const runtimeMarker = 1;",
            "AA==",
        )
        .expect("browser-supplied runtime export should succeed");

        assert!(html.contains("window.PuzzleStandaloneEmbeddedWasm"));
        assert!(html.contains("export const runtimeMarker = 1;"));
        assert!(html.contains(r#"wasmBase64: "AA==""#));
        assert!(!html.contains("PuzzleStudioSetPreviewDebugMode"));
        assert!(!html.contains("PuzzleStudioPreviewDebugTrace"));
        assert!(!html.contains("/api/debug/input/"));
        assert!(!html.contains("PuzzleStudioRuntimeAssetRequest"));
        assert!(!html.contains("Timed out waiting for editor preview runtime asset"));
    }

    #[test]
    fn editor_preview_export_keeps_studio_bridge_for_state_control() {
        let source = r#"
title = Editor Preview Export

puzzle board {
  layers {
    tiles = Player
  }
  empty .
  rules {
    [ Player ] -> [ Player ]
  }
}

levels default of board {
  legend P = Player
  level "one" {
    P
  }
}

scene playing {
  layout {
    puzzle board
  }
}
"#;

        let html = export_editor_preview_html_from_source(
            source,
            "games/editor_preview/game.puzzle",
            "",
            "",
        )
        .expect("editor preview export should succeed");

        assert!(html.contains("PuzzleStudioSetState"));
        assert!(html.contains("PuzzleStudioKey"));
        assert!(html.contains("PuzzleStudioCommand"));
        assert!(html.contains("PuzzleStudioPreviewState"));
        assert!(html.contains("PuzzleStudioSetPreviewDebugMode"));
        assert!(html.contains("PuzzleStudioPreviewDebugTrace"));
        assert!(html.contains("/api/debug/input/"));
        assert!(html.contains("PuzzleRuntimeWasmLoader"));
        assert!(html.contains("set_current_state("));
        assert!(APP_JS.contains("await standaloneRuntime.setCurrentState(event.data.state, {"));
        assert!(html.contains("ui-tap"));
        assert!(html.contains("buildSelectLayers"));
        assert!(!html.contains("broadcastPuzzle3Key"));
        assert!(!html.contains("PuzzleStudioSolve"));
        assert!(!html.contains("loadWasmSolver"));
        let boot = embedded_puzzle_boot_json(&html);
        assert!(
            boot["source"]
                .as_str()
                .is_some_and(|value| value.contains("Editor Preview Export"))
        );
        assert_eq!(
            boot["puzzlePath"],
            json!("games/editor_preview/game.puzzle")
        );
    }

    #[test]
    fn wasm_editor_preview_loader_requests_parent_runtime_assets() {
        let source = include_str!("lib_export.rs");
        assert!(source.contains("PuzzleStudioRuntimeAssetRequest"));
        assert!(source.contains("PuzzleStudioRuntimeAssetResponse"));
        assert!(source.contains("puzzle_wasm_game.js"));
        assert!(source.contains("puzzle_wasm_game_bg.wasm.base64"));
        assert!(
            source.contains("Standalone HTML export requires embedded puzzle_wasm_game assets")
        );
        assert!(source.contains("missing_embedded_wasm_loader_script"));
    }

    #[test]
    fn standalone_export_includes_scene_and_screen_keys() {
        let source = r#"
title = Export Test

puzzle default {
layers {
actor = Player
}

levels {
    legend {
        . = empty
        P = Player
    }

    level "one"
    P
}

rules {
    [ Player ] -> [ Player ]
}
}

scene playing {
    layout {
        puzzle board = default
    }
}
"#;
        let loaded = parse_game(source).unwrap();
        let state = ServerState::new(
            loaded,
            source.to_string(),
            "games/export_test/game.puzzle".to_string(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_export_data(&mut data, &state);

        let export: serde_json::Value =
            serde_json::from_str(&data).expect("export data should be JSON");
        assert!(export.get("scenes").is_some());
        assert!(export.get("screens").is_some());
        assert!(
            export
                .get("engine")
                .and_then(|engine| engine.get("persistentVars"))
                .is_some()
        );
    }

    #[test]
    fn standalone_export_includes_progress_savedata_contract() {
        let source = r#"
title = Progress Export

puzzle default {
persistent var bonus = 0

layers {
actor = Player
}

levels {
    legend {
        . = empty
        P = Player
    }

    level "one"
    P

    level "two"
    P
}

rules {
    [ Player ] -> [ Player ] bonus = 1
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let state = ServerState::new(
            loaded,
            source.to_string(),
            "games/progress_export/game.puzzle".to_string(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_export_data(&mut data, &state);

        assert!(data.contains(r#""saveKey":"Progress Export:"#));
        assert!(data.contains(r#""progressSaveVersion":1"#));
        assert!(data.contains(r#""variables":[{"id":0,"name":"bonus"}]"#));
        assert!(data.contains(r#""persistentVars":[0]"#));
        assert!(STANDALONE_JS.contains("WasmStandaloneSession"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.request_json(method, url)"));
        assert!(STANDALONE_JS.contains("snapshot()"));
        assert!(STANDALONE_JS.contains("restoreSessionProgressSave()"));
        assert!(STANDALONE_JS.contains("writeSessionProgressSave()"));
        assert!(STANDALONE_JS.contains("saved progress was kept and was not overwritten"));
        assert!(STANDALONE_JS.contains("next.has_progress_save = true;"));
        assert!(APP_JS.contains("animationEvents: event.data.animationEvents"));
        assert!(APP_JS.contains("standaloneRuntime.snapshot({ forceJs: true })"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.progress_save()"));
        assert!(STANDALONE_JS.contains("PuzzleStudioPreviewProgressSave"));
        assert!(STANDALONE_JS.contains("PuzzleStudioEditorPreviewProgressSaves"));
        assert!(STANDALONE_JS.contains("window.localStorage?.setItem"));
        assert!(STANDALONE_JS.contains("window.localStorage?.getItem"));
        assert!(!STANDALONE_JS.contains("progressSaveData()"));
        assert!(!STANDALONE_JS.contains("restoreProgressSave()"));
        assert!(!STANDALONE_JS.contains("writeProgressSave()"));
        assert!(!STANDALONE_JS.contains("clearedLevels[index]"));
        assert!(!STANDALONE_JS.contains("currentSaveLevelName()"));
        assert!(!STANDALONE_JS.contains("persistentVarSaveData()"));
        assert!(!STANDALONE_JS.contains("starting from defaults"));
    }

    #[test]
    fn standalone_export_surfaces_display_projection_errors_without_raw_fallback() {
        let server_source = include_str!("lib_server.rs");
        assert!(server_source.contains("push_display_error_scene_object"));
        assert!(server_source.contains(r#"\"cells\":[],\"displayError\":"#));
        assert!(
            !server_source
                .contains("transition_program(&loaded.game, program, state, InputId(0)).ok()")
        );
        assert!(APP_JS.contains("function firstDisplayError(state)"));
        assert!(APP_JS.contains("showError(new Error(displayError));"));
        assert!(APP_CSS.contains(".runtime-error"));
    }

    #[test]
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
        let mut bridge = StandaloneSessionBridge::from_source(
            source,
            "crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle",
        )
        .unwrap();

        let initial = bridge.request_json("GET", "/api/state").unwrap();
        let initial: serde_json::Value = serde_json::from_str(&initial).unwrap();
        assert_eq!(initial["currentScene"], "title");
        assert_eq!(initial["title"], "Microban");
        let initial = initial.as_object().unwrap();
        assert!(initial.contains_key("visibleScenes"));
        assert!(initial.contains_key("sceneState"));
        assert!(initial.contains_key("scenePuzzles"));
        assert!(!initial.contains_key("visibleScreens"));
        assert!(!initial.contains_key("screenState"));
        assert!(!initial.contains_key("screenPuzzles"));

        let playing = bridge
            .request_json("POST", "/api/command/goto%20playing")
            .unwrap();
        let playing: serde_json::Value = serde_json::from_str(&playing).unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let save: serde_json::Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(save["currentLevel"], "microban_01");
    }

    #[test]
    fn standalone_debug_input_endpoint_reports_rule_trace() {
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
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "debug_trace.puzzle").unwrap();

        let body = bridge
            .request_json("POST", "/api/debug/input/right")
            .unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();

        assert_eq!(body["snapshot"]["currentScene"], "main");
        assert_eq!(body["debug"]["input"], "right");
        assert_eq!(body["debug"]["executions"].as_array().unwrap().len(), 1);
        assert_eq!(
            body["debug"]["executions"][0]["rule"]["sourceLine"],
            "[ Player ] -> [ ]"
        );
        assert_eq!(body["debug"]["executions"][0]["patch"][0]["kind"], "remove");
        assert_eq!(
            body["debug"]["executions"][0]["patch"][0]["object"],
            "Player"
        );
    }

    #[test]
    fn standalone_session_scene_preserves_2d_render_settings_after_goto() {
        let source = include_str!("../tests/fixtures/locked.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/TPGJ6/locked.puzzle").unwrap();

        let playing = bridge
            .request_json("POST", "/api/command/goto%20playing")
            .unwrap();
        let playing: serde_json::Value = serde_json::from_str(&playing).unwrap();

        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["scene"]["settings"]["render"]["cellSize"], 40);
        assert_eq!(playing["scene"]["settings"]["inputBuffer"]["minWaitMs"], 50);
        assert_eq!(
            playing["scene"]["settings"]["animation"]["tween"]["intervalMs"],
            50
        );
    }

    #[test]
    fn standalone_editor_state_continues_locked_room_level_after_repeated_moves() {
        let source = include_str!("../tests/fixtures/locked.puzzle");
        let document =
            puzzle_lang::parse_game_for_path(source, "games/TPGJ6/locked.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let level_index = loaded
            .levels
            .iter()
            .position(|level| level.name == "level 5")
            .expect("locked room level 5 should exist");
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[level_index].initial_state);

        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/TPGJ6/locked.puzzle").unwrap();
        bridge
            .set_current_state_json(&state_json, level_index, true)
            .unwrap();
        bridge.apply_input_name("left").unwrap();
        bridge.apply_command_name("__continue_effects").unwrap();
        let after_first: serde_json::Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert!(cell_has_object(
            &after_first["scene"]["cells"][69],
            "Player"
        ));

        bridge.apply_input_name("left").unwrap();
        bridge.apply_command_name("__continue_effects").unwrap();

        let snapshot: serde_json::Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["levelIndex"], level_index);
        assert_eq!(snapshot["currentScene"], "playing");
        assert!(cell_has_object(&snapshot["scene"]["cells"][68], "Player"));
    }

    #[test]
    fn standalone_snapshot_reports_level_select_model_input_contract() {
        let source = r#"
title = level_select_input_contract

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
level "first" {
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
        let mut bridge = StandaloneSessionBridge::from_source(source, "contract.puzzle").unwrap();

        let playing: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(playing["currentScene"], json!("playing"));
        assert_eq!(playing["acceptsModelInput"], json!(true));

        let select: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20level_select")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(select["currentScene"], json!("level_select"));
        assert_eq!(select["acceptsModelInput"], json!(false));

        let after_input: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/down").unwrap()).unwrap();
        assert_eq!(after_input["currentScene"], json!("level_select"));
        assert_eq!(after_input["acceptsModelInput"], json!(false));
        assert_eq!(after_input["canUndo"], json!(false));
    }

    #[test]
    fn standalone_level_menu_position_select_restarts_current_level_state() {
        let source = r#"
title = level_menu_position_restart

puzzle default {
layers {
actor = Player
}
empty .
rules {
right [ Player | no Player ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "first" {
P.
}
level "second" {
P.
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
            StandaloneSessionBridge::from_source(source, "level_menu_restart.puzzle").unwrap();

        let playing: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20playing(second)")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(playing["currentScene"], json!("playing"));
        assert_eq!(playing["levelIndex"], json!(1));

        let moved: Value =
            serde_json::from_str(&bridge.request_json("POST", "/api/input/right").unwrap())
                .unwrap();
        assert!(cell_has_object(&moved["scene"]["cells"][1], "Player"));

        let select: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/goto%20level_select")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(select["currentScene"], json!("level_select"));

        let restarted: Value = serde_json::from_str(
            &bridge
                .request_json("POST", "/api/command/select:1")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(restarted["currentScene"], json!("playing"));
        assert_eq!(restarted["levelIndex"], json!(1));
        assert!(cell_has_object(&restarted["scene"]["cells"][0], "Player"));
        assert!(!cell_has_object(&restarted["scene"]["cells"][1], "Player"));
        assert_eq!(restarted["canUndo"], json!(false));
    }

    #[test]
    fn standalone_session_bridge_emits_tween_on_first_input() {
        let source = r#"
title = "Standalone Tween Fixture"

puzzle board {
  render {
    tween = true
    tween_duration = 300ms
  }
  layers {
    actor = Player
  }
  rules {
    input right [ Player | no Player ] -> [ | Player ]
  }
}

levels default of board {
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
    puzzle board = board
  }
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "standalone_tween_fixture.puzzle")
                .unwrap();

        let playing = bridge
            .request_json("POST", "/api/command/goto%20playing(%22first%22)")
            .unwrap();
        let playing: serde_json::Value = serde_json::from_str(&playing).unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved = bridge.request_json("POST", "/api/input/right").unwrap();
        let moved: serde_json::Value = serde_json::from_str(&moved).unwrap();
        assert_eq!(
            moved["animationEvents"],
            json!([
                {
                    "kind": "move",
                    "name": "tween",
                    "objectId": 1,
                    "from": { "x": 0, "y": 0 },
                    "to": { "x": 1, "y": 0 }
                }
            ])
        );
    }

    #[test]
    fn standalone_session_bridge_exposes_default_move_wait_boundary() {
        let source = r#"
title = "Standalone Default Move Wait Fixture"

input_buffer {
  queue_during_wait = true
  fast_forward_wait = true
  min_wait = 75ms
}

puzzle board {
  render {
    tween = true
    tween_duration = 80ms
  }
  layers {
    solid = Player
    marker = Done
  }
  rules {
    input right [ Player ] -> [ > Player ]
    [ > solid | no solid ] -> [ | solid ]
    [ > solid ] -> [ solid ]
    wait animation
    [ Player no Done ] -> [ Player Done ]
  }
}

levels default of board {
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
    puzzle board = board
  }
}
"#;
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "standalone_default_move_wait.puzzle")
                .unwrap();

        bridge
            .request_json("POST", "/api/command/goto%20playing(%22first%22)")
            .unwrap();

        let moved = bridge.request_json("POST", "/api/input/right").unwrap();
        let moved: serde_json::Value = serde_json::from_str(&moved).unwrap();
        assert_eq!(
            moved["inputBuffer"],
            json!({
                "queueDuringWait": true,
                "fastForwardWait": true,
                "minWaitMs": 75
            })
        );
        assert!(cell_has_object(&moved["scene"]["cells"][1], "Player"));
        assert!(!cell_has_object(&moved["scene"]["cells"][1], "Done"));
        assert_eq!(
            moved["waitEvents"],
            json!([
                {
                    "kind": "continue_effects",
                    "milliseconds": 80
                }
            ])
        );

        let continued = bridge
            .request_json("POST", "/api/command/__continue_effects")
            .unwrap();
        let continued: serde_json::Value = serde_json::from_str(&continued).unwrap();
        assert!(cell_has_object(&continued["scene"]["cells"][1], "Player"));
        assert!(cell_has_object(&continued["scene"]["cells"][1], "Done"));
        assert_eq!(continued["waitEvents"], json!([]));
    }

    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
        let mut bridge = StandaloneSessionBridge::from_source(
            source,
            "crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle",
        )
        .unwrap();
        bridge
            .restore_progress_save_json(
                r#"{"version":1,"levels":[{"name":"microban_01","cleared":true}],"currentLevel":"microban_01","persistentVars":[]}"#,
            )
            .unwrap();

        let snapshot = bridge.request_json("GET", "/api/state").unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 0);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(snapshot["levels"][0]["cleared"], true);
    }

    #[test]
    fn standalone_export_resolves_viewport_focus_group_objects() {
        let source = r#"
title = Flickscreen Focus

puzzle default {
layers {
  floor = Background
  actor = Player1 Player2
}
empty .
groups {
  player = Player1 Player2
}
flickscreen 5 5
screen_focus player

levels {
  legend {
    . = Background
    P = Player1
  }
  P....
}

rules {
  [ Player1 ] -> [ Player1 ]
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let state = ServerState::new(
            loaded,
            source.to_string(),
            "games/focus/game.puzzle".to_string(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_export_data(&mut data, &state);

        assert!(data.contains(r#""viewportFocus":"player""#));
        assert!(data.contains(r#""viewportFocusObjects":[2,3]"#));
        assert!(RENDERER_JS.contains("focusObjects.has(Number(layer.objectId))"));
    }

    #[test]
    fn standalone_export_initial_body_uses_loaded_theme() {
        let source = r##"
title = Theme Startup
theme noir {
  background_color = #123456
}

puzzle default {
layers {
actor = Player
}

levels {
    legend {
        . = empty
        P = Player
    }

    level "one"
    P
}

rules {
    [ Player ] -> [ Player ]
}
}
"##;
        let html = export_html_from_source(source, "games/theme_startup/game.puzzle", "", "")
            .expect("export themed document");

        assert!(html.contains(r#"<body class="theme-noir" style="--background:#123456;">"#));
    }

    #[test]
    fn standalone_export_supports_single_puzzle3_document() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3");
        let html = export_html_from_source(
            source,
            "games/spec_3d.puzzle3",
            "body { --accent: #123456; }",
            "",
        )
        .expect("release export should use source-free 3D runtime");

        assert!(html.contains("window.Puzzle3DFrameFixture"));
        assert!(html.contains("runtimeContractVersion"));
        assert!(html.contains("puzzle_wasm_game_bg.wasm"));
        assert!(html.contains("WasmPuzzle3Runtime"));
        assert!(html.contains("fromFixture"));
        assert!(html.contains("WasmStandaloneSession"));
        assert!(html.contains("onLifecycleEffects(effects)"));
        assert!(html.contains("function sendPuzzle3LifecycleEffects("));
        assert!(!html.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!html.contains("puzzle_wasm_player_bg.wasm"));
        assert!(!html.contains("\\npuzzle3 microban3d"));

        let preview_html = export_editor_preview_html_from_source(
            source,
            "games/spec_3d.puzzle3",
            "body { --accent: #123456; }",
            "",
        )
        .expect("editor preview should embed preview runtime assets");

        assert!(preview_html.contains("window.Puzzle3DFixture"));
        assert!(preview_html.contains("WasmPuzzle3Runtime"));
        assert!(preview_html.contains("WasmStandaloneSession"));
        assert!(preview_html.contains("onLifecycleEffects(effects)"));
        assert!(preview_html.contains("function sendPuzzle3LifecycleEffects("));
        assert!(!preview_html.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!preview_html.contains("Puzzle3DTestRuntime"));
        assert!(html.contains("Microban 3D"));
        assert!(html.contains("--accent: #123456"));
        let mut bridge = StandaloneSessionBridge::from_source(source, "games/spec_3d.puzzle3")
            .expect("single puzzle3 document should have a scene host game runtime");
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["currentScene"], json!("title"));
    }

    #[test]
    fn puzzle3_source_free_export_embeds_local_frame_runtime_contract() {
        let source = r#"title = "Local Frame"

puzzle3 cube {
  layers {
    actor = Player
  }

  rules local_frame 1 1 full {
    [ Player ] -> [ Player ]
  }
}

scene playing {
  layout {
    puzzle3 board = cube
  }
}

levels3 default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"#;
        let html = export_html_from_source(source, "games/local_frame.puzzle3", "", "")
            .expect("local_frame should be part of the source-free runtime contract");

        assert!(html.contains("runtimeContract"));
        assert!(html.contains("localFrame"));
        assert!(!html.contains("\\npuzzle3 cube"));
    }

    #[test]
    fn puzzle3_export_embeds_source_free_frame_fixture_and_path() {
        let source = r#"title = "Tiny"

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}

scene title {
  layout {
    choice "Play" -> goto playing
  }
}

scene playing {
  layout {
    puzzle3 board = cube
  }
}

levels3 default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"#;
        let html = export_html_from_source(source, "games/tiny.puzzle3", "", "")
            .expect("release puzzle3 document should use source-free runtime");

        assert!(html.contains("window.Puzzle3DFrameFixture = JSON.parse"));
        assert!(html.contains("window.Puzzle3DFrameAssets = {"));
        assert!(html.contains("window.Puzzle3ControllerAutoBoot = false"));
        assert!(html.contains("window.Puzzle3ThreeModuleSource = "));
        assert!(html.contains("window.Puzzle3ThreeRenderer"));
        assert!(html.contains("window.Puzzle3Controller"));
        assert!(!html.contains("\"themeCss\""));
        assert!(!APP_JS.contains("assets.themeCss"));
        assert!(!APP_JS.contains("body.is-component-embed[class]"));
        assert!(!APP_JS.contains("frame.setAttribute(\"allowtransparency\", \"true\");"));
        assert!(!APP_JS.contains("frame.style.backgroundColor"));
        assert!(!APP_JS.contains("<html lang=\"en\" style=\"background:transparent;\">"));
        assert!(
            !APP_JS
                .contains("<body class=\"is-component-embed\" style=\"background:transparent;\">")
        );
        assert!(
            PUZZLE3_APP_JS.contains(
                "const ctx = puzzle3RendererMode === \"three\" ? null : canvas.getContext(\"2d\", { alpha: true });"
            )
        );
        assert!(PUZZLE3_APP_JS.contains("function drawWithThree()"));
        assert!(PUZZLE3_APP_JS.contains("function resolvePuzzle3RendererMode(value)"));
        assert!(PUZZLE3_APP_JS.contains("return text === \"canvas\" ? \"canvas\" : \"three\";"));
        assert!(!PUZZLE3_APP_JS.contains("function puzzle3ThreeRendererAvailable()"));
        assert!(PUZZLE3_APP_JS.contains("const PUZZLE3_RENDERER_CONTRACT_VERSION = 1;"));
        assert!(PUZZLE3_APP_JS.contains("function puzzle3RendererContractInput(width, height)"));
        assert!(PUZZLE3_APP_JS.contains(
            "snapshot: cloneRuntimeSnapshot(requireLoadedPuzzle3Snapshot(\"Puzzle3 renderer snapshot\"))"
        ));
        assert!(PUZZLE3_APP_JS.contains("renderer.render(input.snapshot, input.view)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("const PUZZLE3_THREE_RENDERER_CONTRACT = "));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("input: [\"snapshot\", \"view\"]"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("contract: PUZZLE3_THREE_RENDERER_CONTRACT"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("buildPuzzleStudioThreeFrame"));
        assert!(PUZZLE3_APP_JS.contains("next.projection = \"orthographic\";"));
        assert!(!PUZZLE3_APP_JS.contains("debugAsymmetricSprites"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("debugAsymmetric"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("new THREE.PerspectiveCamera"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("new THREE.OrthographicCamera"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("projection === \"orthographic\""));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("const yaw = degreesToRadians(cameraSettings.yawDegrees ?? 0);")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("const pitch = degreesToRadians(clamp(Number(cameraSettings.pitchDegrees ?? 35) || 35, -90, 90));"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("camera.up.set(0, 1, 0);"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("targetPoint.x - Math.sin(yaw) * horizontal * distance")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("targetPoint.y + Math.sin(pitch) * distance"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("targetPoint.z + Math.cos(yaw) * horizontal * distance")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function threeBackground(THREE, value)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("disposeScene(this.scene);"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function addGrid(THREE, scene, frame)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function frameVisibleVoxels(frame)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function visibleVoxelStack(stack)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function mergedVoxelFaces(voxels, occupied)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("Puzzle3VisualCore.mergeVoxelFaces(voxels"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function isVoxelFaceOccluded(voxel, offset, occupied)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("if (voxel.opaque !== false && occupied.opaque.has(adjacentKey))")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("occupied.bySource.has(`${sourceKey}|${adjacentKey}`)")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function faceBufferGeometry(THREE, faces)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function parseColor(fill)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("opaque: !source || source.a >= 0.999"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("if (renderVoxel.opaque) {\n      visible.length = 0;")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("transparent: alpha < 0.999"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("opacity: Math.max(0, Math.min(1, alpha))"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("depthWrite: alpha >= 0.999"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("new THREE.BoxGeometry"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("new THREE.InstancedMesh"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains("const visual = spriteVisual(sprites[spriteName]);")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("|| !visual) {\n    return null;\n  }"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("return [];\n}"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("fallbackVisual"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("function cubeInstance"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("function colorForObject"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("kind: \"cube\""));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("x: position.x - (frame.size.width - 1) / 2"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("y: position.z - (frame.size.height - 1) / 2"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("z: (frame.size.depth - 1) / 2 - position.y"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function spriteVoxelLocalPosition(voxel, size, step)")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("x: (voxel.x + 0.5 - size.width / 2) * step"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("y: (voxel.y + 0.5 - size.depth / 2) * step"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("z: (voxel.z + 0.5 - size.height / 2) * step"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("const layerY = object.layer * 0.08;"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("y: base.y + layerY + local.z"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("z: base.z - local.y"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function viewportRanges(frame)"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function viewportProjectedVisibleHeight(frame, target, aspect)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function smoothViewportTarget(next, target, frame)")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function smoothViewportMaxLag(frame)"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains("const catchUp = (distance - maxLag) / distance;")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function viewportProjectedBounds(frame)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function viewportFocusRenderTarget(frame)"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains("function viewportFocusVisualRenderBounds(frame)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function projectRenderPointForCamera(point, cameraSettings)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("applyProjectedRenderCulling(THREE, frame, camera, this.canvas);")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function applyProjectedRenderCulling(THREE, frame, camera, canvas)")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("camera.updateMatrixWorld?.();"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains("function projectedRenderCullingEnabled(frame)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function cellCoordinateRenderBounds(frame, cell, extent")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function conservativeCellRenderExtent(frame)"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("function projectedRenderBounds(THREE, bounds, camera)")
        );
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("function cellRenderBounds(frame, cell)"));
        assert!(
            !PUZZLE3_THREE_RENDERER_JS.contains("objectVoxels(frame, cell.position || {}, object")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function cameraZoom(frame)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("mode === \"paged\""));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("frame.renderCells = cells;"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("function renderRanges(frame)"));
        assert!(!PUZZLE3_THREE_RENDERER_JS.contains("function cellInRanges(cell, ranges)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains(
            "if ((!Number.isFinite(Number(merged.id)) && !name && !spriteName) || !visual)"
        ));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("id: Number.isFinite(Number(merged.id)) ? Number(merged.id) : name")
        );
        assert!(PUZZLE3_APP_JS.contains("viewportSnapNext: view.viewportSnapNext"));
        assert!(PUZZLE3_APP_JS.contains("pixelateBuffer.getContext(\"2d\", { alpha: true })"));
        assert!(APP_JS.contains("window.Puzzle3Controller.attach(canvas"));
        assert!(APP_CSS.contains(
            ".scene-ratio-slot > [data-frame-component=\"true\"] {\n  width: 100%;\n  height: 100%;"
        ));
        assert!(!APP_CSS.contains(".puzzle3-component[data-frame-component=\"true\"] > canvas"));
        assert!(
            PUZZLE3_STYLE_CSS
                .contains(".puzzle3-component > canvas {\n  position: absolute;\n  inset: 0;")
        );
        assert!(APP_CSS.contains(
            ".scene-layer.has-ratio-content > :not(.has-ratio-content):not(.scene-ratio-slot),"
        ));
        assert!(APP_CSS.contains(
            ".view-row.has-ratio-content > :not(.has-ratio-content):not(.scene-ratio-slot) {\n  flex: 0 0 auto;\n}"
        ));
        assert!(APP_CSS.contains(".scene-ratio-slot > iframe[data-frame-component=\"true\"] {\n  width: 100%;\n  height: 100%;\n  border: 0;\n}"));
        assert!(!html.contains(
            ".puzzle3-frame { border: 0; display: block; inline-size: 100%; block-size: 100%;"
        ));
        assert!(!html.contains("iframe.puzzle3-frame"));
        assert!(html.contains("case \"choice\""));
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        assert!(runtime_export["runtimeLoadedGame"].is_object());
        assert!(runtime_export.get("engine").is_none());
        assert!(runtime_export.get("compiledPlay").is_none());
        assert!(!html.contains("\\npuzzle3 cube"));
        assert!(!html.contains("\"source\":\"title \\\\\\\"Tiny\\\\\\\"\\n"));
        assert!(html.contains("\"puzzlePath\":\"games/tiny.puzzle3\""));
        assert!(!html.contains("window.Puzzle3DSource ="));
        assert!(!html.contains("window.Puzzle3DPath ="));
    }

    #[test]
    fn puzzle3_frame_export_keeps_component_document_transparent() {
        let source = r##"title = "Themed 3D"
theme clean {
  background_color = #123456
}

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}

scene playing {
  layout {
    puzzle3 board = cube
  }
}

levels3 default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"##;
        let html = export_html_from_source(source, "games/themed_3d.puzzle3", "", "")
            .expect("release themed puzzle3 document should use source-free runtime");
        let boot = embedded_puzzle_boot_json(&html);
        let fixture = embedded_puzzle3_frame_fixture_json(&html);

        assert!(html.contains(r#"<body class="theme-clean" style="--background:#123456;">"#));
        assert_eq!(boot["theme"]["name"], json!("clean"));
        assert_eq!(boot["theme"]["variables"]["background"], json!("#123456"));
        assert_eq!(fixture["theme"]["name"], json!("clean"));
        assert_eq!(
            fixture["theme"]["variables"][0]["name"],
            json!("background")
        );
        assert_eq!(fixture["theme"]["variables"][0]["value"], json!("#123456"));
        assert!(html.contains("window.Puzzle3DFrameAssets = {"));
        assert!(html.contains("window.Puzzle3ControllerAutoBoot = false"));
        assert!(html.contains("window.Puzzle3Controller"));
        assert!(!html.contains("\"themeCss\""));
        assert!(!html.contains("theme-clean is-component-embed"));
        assert!(!html.contains("frame.style.backgroundColor"));
        assert!(!html.contains("<html lang=\"en\" style=\"background:transparent;\">"));
        assert!(
            !html.contains("<body class=\"is-component-embed\" style=\"background:transparent;\">")
        );
        assert!(PUZZLE3_APP_JS.contains("canvas.getContext(\"2d\", { alpha: true })"));
    }

    #[test]
    fn puzzle3_screenshot_default_scene_prefers_model_component_scene() {
        let source = r#"
title = "Screenshot"

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}

scene title {
  layout {
    title = "Screenshot"
    button "Play" -> goto playing
  }
}

scene playing {
  layout {
    puzzle3 board = cube
  }
}

levels3 basic of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"#;
        let document = puzzle_lang::parse_game(source).expect("parse puzzle3 document");
        assert_eq!(
            default_puzzle3_screenshot_scene(&document).as_deref(),
            Some("playing")
        );
    }

    #[test]
    fn screenshot_file_url_encodes_path_for_browser() {
        let path = Path::new("/tmp/Puzzle Studio/screen one.html");
        assert_eq!(
            file_url(path),
            "file:///tmp/Puzzle%20Studio/screen%20one.html"
        );
        assert_eq!(url_condition_value("level one"), "level%20one");
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Io(error) => write!(f, "{error}"),
            Self::Lang(error) => write!(f, "{error}"),
            Self::CoreTransition(error) => write!(f, "{error:?}"),
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}
