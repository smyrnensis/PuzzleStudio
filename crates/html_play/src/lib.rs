#![cfg_attr(any(target_arch = "wasm32", not(feature = "solver")), allow(dead_code))]

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
#[cfg(all(test, not(target_arch = "wasm32")))]
use std::process::Stdio;
#[cfg(any(not(target_arch = "wasm32"), feature = "solver"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(any(not(target_arch = "wasm32"), feature = "solver"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

use puzzle_assets::{EncodedVisualImageAsset, EncodedVisualImageBundle};
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionValueKind, GridSize, InputId, MarkPattern, MarkValueMatch,
    ObjectId, Offset, PatchOp, Pattern, RuleFiring, State, TransitionCommand,
};
pub use puzzle_game_runtime::RuntimeSession;
use puzzle_lang::{AssetKind, DiagnosticReport};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::{AssetsDef, VisualKind};
use puzzle_lang::{
    GoalCondition, GoalExpr, GoalValue, Level, LoadedDocumentModel, LoadedGame, RuleAnimation,
    RuleAnimationTrigger, SceneComponent, SceneValue, parse_game2d as parse_game,
};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::{discover_game_entries, expand_game_imports_for_file, resolve_game_entry};
use puzzle_play::{
    animation_events_contract_2d, animation_events_for_trace,
    loaded_document_scene_host_loaded_game,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
use puzzle_runtime_contract::SessionAction;
use puzzle_runtime_contract::{
    RuntimeChangedCell, RuntimeCoord, RuntimeMarkValueMatch, RuntimePatchOp, RuntimeRuleFiring,
    RuntimeStateSnapshot, RuntimeStateSnapshot2d, RuntimeTransitionCommand,
    RuntimeTransitionCurrentOutcome, RuntimeTransitionProgramOutcome, StandaloneProgressStorage,
    StandaloneRuntimeExport,
};
const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_CSS: &str = include_str!("../static/app.css");
const RENDERER_CSS: &str = include_str!("../static/renderer.css");
const VISUALS_JS: &str = include_str!("../static/visuals.js");
const APP_JS: &str = include_str!("../static/app.js");
const RENDERER_JS: &str = include_str!("../static/renderer.js");
const VISUAL_TWEEN_CORE_JS: &str = include_str!("../static/visual_tween_core.js");
const STANDALONE_JS: &str = include_str!("../static/standalone.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_PLAYER_WASM_JS: &str = include_str!("../static/wasm_player/puzzle_wasm_player.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_PLAYER_WASM_BG: &[u8] =
    include_bytes!("../static/wasm_player/puzzle_wasm_player_bg.wasm");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_JS: &str = include_str!("../static/wasm_game/puzzle_wasm_game.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_GAME_WASM_BG: &[u8] = include_bytes!("../static/wasm_game/puzzle_wasm_game_bg.wasm");
const PUZZLE3_STYLE_CSS: &str = include_str!("../static/puzzle3.css");
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
const PUZZLE3_THREE_RENDERER_JS: &str = include_str!("../static/puzzle3_three_renderer.js");
const PUZZLE3_COMPONENT_JS: &str = include_str!("../static/puzzle3_component.js");
const THREE_MODULE_JS: &str = include_str!("../static/vendor/three/three.module.min.js");

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
        "effects",
        "firings",
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

    fn first_viewport_state(snapshot: &Value) -> &Value {
        snapshot["viewportSources"]
            .as_array()
            .and_then(|sources| sources.first())
            .map(|source| &source["state"])
            .unwrap_or_else(|| panic!("snapshot should expose a viewport source: {snapshot}"))
    }

    fn embedded_puzzle_runtime_export_json(html: &str) -> Value {
        embedded_puzzle_json_assignment(
            html,
            "window.PuzzleRuntimeExportJson = \"",
            "\";",
            "PuzzleRuntimeExportJson",
        )
    }

    fn assert_official_export_uses_bevy_launcher(html: &str) {
        assert!(html.contains(r#"<canvas id="puzzle-bevy""#));
        assert!(html.contains(r#"<output id="puzzle-bevy-status" hidden data-state="starting">"#));
        assert!(html.contains(r#"status.dataset.state = "fatal";"#));
        assert!(html.contains("startStandalonePlayer"));
        assert!(html.contains("window.PuzzleRuntimeExportJson = "));
        assert!(!html.contains("window.PuzzleBoot"));
        assert!(!html.contains("window.Puzzle3DFrameFixture"));
        assert!(!html.contains("window.Puzzle3DFrameAssets"));
        assert!(!html.contains("window.Puzzle3ThreeModuleSource"));
        assert!(!html.contains("window.Puzzle3ThreeRenderer"));
        assert!(!html.contains("window.Puzzle3Component"));
        assert!(!html.contains("three.module"));
        assert!(!html.contains("renderer.js"));
        assert!(!html.contains("standalone.js"));
        assert!(!html.contains("app.js"));
    }

    fn embedded_editor_preview_export_json(html: &str) -> Value {
        embedded_puzzle_json_assignment(
            html,
            "window.PuzzleEditorPreviewExportJson = \"",
            "\";",
            "PuzzleEditorPreviewExportJson",
        )
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

    fn validate_puzzle3_fixture_with_browser_contract(fixture: &Value) -> Value {
        let script = format!(
            r#"
globalThis.window = globalThis;
{}
const fs = require("fs");
const fixture = JSON.parse(fs.readFileSync(0, "utf8"));
try {{
  const validated = window.Puzzle3Component.validateSnapshot(fixture);
  process.stdout.write(JSON.stringify({{
    size: validated.size,
    cellCount: validated.cells.length,
    inputCount: validated.inputs.length,
  }}));
}} catch (error) {{
  console.error(error?.stack || error?.message || String(error));
  process.exitCode = 1;
}}
"#,
            PUZZLE3_COMPONENT_JS
        );
        let mut child = Command::new("node")
            .arg("-e")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required for the Puzzle3 browser contract boundary test");
        child
            .stdin
            .as_mut()
            .expect("Node stdin is available")
            .write_all(fixture.to_string().as_bytes())
            .expect("write Puzzle3 fixture to Node");
        let output = child
            .wait_with_output()
            .expect("run Puzzle3 browser contract validator");
        assert!(
            output.status.success(),
            "Puzzle3 browser contract rejected the generated fixture:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("browser contract result is JSON")
    }

    fn evaluate_puzzle3_render_math(expression: &str) -> Value {
        let script = format!(
            r#"
globalThis.window = globalThis;
{}
{}
{}
const value = (() => {{ {} }})();
process.stdout.write(JSON.stringify(value));
"#,
            VISUAL_TWEEN_CORE_JS, PUZZLE3_VISUAL_CORE_JS, PUZZLE3_THREE_RENDERER_JS, expression
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the Puzzle3 render math test");
        assert!(
            output.status.success(),
            "Puzzle3 render math evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("Puzzle3 render math result is JSON")
    }

    fn evaluate_renderer(expression: &str) -> Value {
        let script = format!(
            r#"
globalThis.window = globalThis;
{}
{}
const value = (() => {{ {} }})();
process.stdout.write(JSON.stringify(value));
"#,
            VISUAL_TWEEN_CORE_JS, RENDERER_JS, expression
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the 2D renderer contract test");
        assert!(
            output.status.success(),
            "2D renderer evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("2D renderer result is JSON")
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
    fn standalone_export_omits_the_legacy_manifest_file_payload() {
        let dir = std::env::temp_dir().join(format!(
            "puzzle_assets_manifest_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("visuals")).expect("create asset fixture directory");
        std::fs::write(
            dir.join("visuals/player.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="#f00"/></svg>"##,
        )
        .expect("write declared asset");
        std::fs::write(dir.join("secret.pdf"), b"not declared").expect("write undeclared asset");

        let source = r#"
title = Manifest Assets

assets {
"visuals/player.svg"
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

        assert!(!html.contains("\"visuals/player.svg\":\"data:image/svg+xml;charset=utf-8,"));
        assert!(!html.contains("secret.pdf"));
        assert!(!html.contains("not declared"));
        assert!(!html.contains("PuzzleAssets.files"));
        assert!(!html.contains("Puzzle asset is not embedded"));
    }

    #[test]
    fn standalone_runtime_export_embeds_validated_visual_image_bundle_once() {
        const ONE_PIXEL_PNG: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00,
            0x00, 0xb5, 0x1c, 0x0c, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78,
            0xda, 0x63, 0x64, 0xf8, 0x0f, 0x00, 0x01, 0x05, 0x01, 0x01, 0x27, 0x18, 0xe3, 0x66,
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let dir = std::env::temp_dir().join(format!(
            "puzzle_visual_bundle_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("visuals")).expect("create visual fixture directory");
        let puzzle_path = dir.join("game.puzzle");
        std::fs::write(dir.join("visuals/tile.png"), ONE_PIXEL_PNG).expect("write visual fixture");
        let source = r#"
title = Visual Bundle

puzzle default {
layers {
actor = Tile
}
visuals {
Tile {
image = "visuals/tile.png"
}
}
rules {
}
level "one" {
.
}
}
"#;
        std::fs::write(&puzzle_path, source).expect("write puzzle fixture");

        let html = export_html_from_source(
            source,
            puzzle_path.to_str().expect("fixture path is UTF-8"),
            "",
            "",
        )
        .expect("export visual bundle");
        let export = embedded_puzzle_runtime_export_json(&html);
        let assets = export["visualImages"]["assets"]
            .as_array()
            .expect("visualImages.assets is an array");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0]["manifest"]["path"], "visuals/tile.png");
        assert_eq!(assets[0]["manifest"]["format"], "png");
        assert!(
            assets[0]["revision"]
                .as_str()
                .is_some_and(|revision| !revision.is_empty())
        );
        assert!(
            assets[0]["bytes"]
                .as_str()
                .is_some_and(|bytes| !bytes.is_empty())
        );
        let typed_export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument> =
            serde_json::from_value(export.clone()).expect("typed player export roundtrip");
        assert_eq!(typed_export.visual_images.assets[0].bytes, ONE_PIXEL_PNG);
        assert_eq!(html.matches("\\\"visualImages\\\"").count(), 1);
        assert!(!html.contains("data:image/png"));
        assert!(!html.contains("PuzzleAssets.files"));

        let file_html = export_html_file(&puzzle_path).expect("export visual bundle from file");
        let file_export = embedded_puzzle_runtime_export_json(&file_html);
        assert_eq!(
            file_export["progressStorage"], export["progressStorage"],
            "source and file export routes must share one progress identity"
        );

        std::fs::write(dir.join("visuals/tile.png"), b"not a PNG")
            .expect("replace visual fixture with invalid bytes");
        let error = export_html_from_source(
            source,
            puzzle_path.to_str().expect("fixture path is UTF-8"),
            "",
            "",
        )
        .expect_err("invalid visual bytes must reject export")
        .to_string();
        assert!(error.contains("failed to decode visual image `visuals/tile.png`"));

        std::fs::remove_dir_all(dir).expect("remove visual fixture directory");
    }

    #[test]
    fn stateful_core_runtime_exposes_changed_cells_for_2d() {
        let source = r#"
puzzle board {
  render {
    tween = true
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
                    "occurrenceId": 1,
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
    fn renderer_board_floor_is_transparent_by_default() {
        assert!(RENDERER_CSS.contains("--cell-background: transparent;"));
        assert!(RENDERER_JS.contains("floorColor && floorColor !== \"transparent\""));
    }

    #[test]
    fn puzzle3_runtime_advances_animated_visual_frames() {
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function currentRuntimeVisualLayers(visual, now = performance.now())")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("Math.floor(now / frameDuration) % frames.length"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function hasRuntimeVisualAnimation()"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains(
                "if (hasRuntimeVisualAnimation()) {\n    scheduleViewportAnimation();\n  }"
            )
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("throw new Error(\"Puzzle3 runtime visual frames are missing.\")")
        );
    }

    #[test]
    fn renderer_tween_keeps_compiled_visual_order() {
        let display_list = evaluate_renderer(
            r#"
const renderer = Object.create(window.PuzzleRenderer.prototype);
renderer.activeTriggerAnimations = [];
renderer.visuals = () => ({
  order: {
    direction_priority: ["down", "right"],
    priorities: [
      { objects: ["Low"] },
      { objects: ["High"] },
    ],
  },
});
const scene = {
  width: 1,
  height: 1,
  cells: [{
    x: 0,
    y: 0,
    renderOrder: 0,
    layers: [
      { object: "High", objectId: 2, layer: 1, renderPriority: 1, composition: "ordered" },
      { object: "Low", objectId: 1, layer: 0, renderPriority: 0, composition: "ordered" },
    ],
  }],
};
const animations = [{
  kind: "move",
  name: "tween",
  occurrenceId: 1,
  objectId: 1,
  from: { x: -1, y: 0 },
  to: { x: 0, y: 0 },
}];
return renderer.canvasDisplayList(scene, { x: 0, y: 0, width: 1, height: 1 }, 1, animations, 0.5)
  .map((item) => ({ object: item.layer.object, animated: Boolean(item.animation) }));
"#,
        );
        assert_eq!(
            display_list,
            json!([
                { "object": "Low", "animated": true },
                { "object": "High", "animated": false },
            ])
        );

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
            RENDERER_JS.contains(
                "canvasDisplayList(scene, frame, unit, animations = [], progress = 1, now = performance.now())"
            )
        );
        assert!(!RENDERER_JS.contains("const staticSurfaces = new Map();"));
        assert!(RENDERER_JS.contains("const items = [];"));
        assert!(RENDERER_JS.contains("let order = 0;"));
        assert!(RENDERER_JS.contains("layerOrder: this.layerRenderPriority(layer),"));
        assert!(!RENDERER_JS.contains("canvasSurfaceItemForLayer("));
        assert!(RENDERER_JS.contains(
            "const compare = (a, b) => a.cellOrder - b.cellOrder\n      || a.layerOrder - b.layerOrder\n      || a.sourceCellOrder - b.sourceCellOrder\n      || a.order - b.order;"
        ));
        assert!(RENDERER_JS.contains("return items.sort(compare);"));
        assert!(!RENDERER_JS.contains("paintCanvasSurface("));
        assert!(!RENDERER_JS.contains("paintCanvasPatternSurface("));
        assert!(!RENDERER_JS.contains("mergedCanvasRects("));
        assert!(RENDERER_JS.contains("animation: animation && progress < 1 ? animation : null"));
        assert!(RENDERER_JS.contains(
            "for (const item of this.canvasDisplayList(scene, frame, unit, animations, progress, now))"
        ));
        assert!(RENDERER_JS.contains(
            "paintCanvasItem(context, item, unit, progress = 1, now = performance.now())"
        ));
        assert!(RENDERER_JS.contains(
            "this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress, now);"
        ));
        assert_eq!(RENDERER_JS.matches("context.clip();").count(), 1);
        assert!(!RENDERER_JS.contains("visualBox("));
        assert!(RENDERER_JS.contains("canvasMetrics(canvas, scene, frame)"));
        assert!(RENDERER_JS.contains("canvasPresentationCellUnit()"));
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
        assert!(!RENDERER_JS.contains("visualPatternSize(frameDef)"));
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
    fn renderer_places_transient_animations_in_the_compiled_display_list_priority() {
        let display_list = evaluate_renderer(
            r#"
const renderer = Object.create(window.PuzzleRenderer.prototype);
renderer.activeTriggerAnimations = [{
  id: "flash:0",
  name: "Flash",
  x: 0,
  y: 0,
  startedAtMs: 0,
  durationMs: 100,
  renderPriority: 1,
  composition: "ordered",
}];
renderer.visuals = () => ({
  order: {
    direction_priority: ["down", "right"],
    priorities: [
      { objects: ["Floor"], animations: [], merge: false },
      { objects: [], animations: ["Flash"], merge: false },
      { objects: ["Foreground"], animations: [], merge: false },
    ],
  },
});
const scene = {
  width: 1,
  height: 1,
  cells: [{
    x: 0,
    y: 0,
    renderOrder: 0,
    layers: [
      { object: "Foreground", objectId: 2, layer: 1, renderPriority: 2, composition: "ordered" },
      { object: "Floor", objectId: 1, layer: 0, renderPriority: 0, composition: "ordered" },
    ],
  }],
};
return renderer.canvasDisplayList(
  scene,
  { x: 0, y: 0, width: 1, height: 1 },
  1,
  [],
  1,
  50,
).map((item) => item.kind === "trigger" ? `!${item.instance.name}` : item.layer.object);
"#,
        );

        assert_eq!(display_list, json!(["Floor", "!Flash", "Foreground"]));
        assert!(!RENDERER_JS.contains("paintTriggerAnimations("));
    }

    #[test]
    fn renderer_tween_composites_every_swept_cell_by_visual_priority() {
        let display_orders = evaluate_renderer(
            r#"
const renderer = Object.create(window.PuzzleRenderer.prototype);
renderer.activeTriggerAnimations = [];
renderer.visuals = () => ({
  order: {
    direction_priority: ["down", "right"],
    priorities: [
      { objects: ["Floor"] },
      { objects: ["Player"] },
      { objects: ["Foreground"] },
    ],
  },
});
function displayOrder(from, to) {
  const width = from.x === to.x ? 1 : 2;
  const height = from.y === to.y ? 1 : 2;
  const cells = [];
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const layers = [
        { object: "Floor", objectId: 1, layer: 0, renderPriority: 0, composition: "ordered" },
        { object: "Foreground", objectId: 3, layer: 2, renderPriority: 2, composition: "ordered" },
      ];
      if (x === to.x && y === to.y) {
        layers.push({ object: "Player", objectId: 2, layer: 1, renderPriority: 1, composition: "ordered" });
      }
      cells.push({ x, y, renderOrder: y * width + x, layers });
    }
  }
  const scene = { width, height, cells };
  const animations = [{
    kind: "move", name: "tween", occurrenceId: 1, objectId: 2, from, to,
  }];
  return renderer.canvasDisplayList(
    scene,
    { x: 0, y: 0, width, height },
    1,
    animations,
    0.5,
  ).map((item) => item.layer.object);
}
return {
  left: displayOrder({ x: 1, y: 0 }, { x: 0, y: 0 }),
  right: displayOrder({ x: 0, y: 0 }, { x: 1, y: 0 }),
  up: displayOrder({ x: 0, y: 1 }, { x: 0, y: 0 }),
  down: displayOrder({ x: 0, y: 0 }, { x: 0, y: 1 }),
};
"#,
        );

        let expected = json!(["Floor", "Floor", "Player", "Foreground", "Foreground"]);
        assert_eq!(display_orders["left"], expected);
        assert_eq!(display_orders["right"], expected);
        assert_eq!(display_orders["up"], expected);
        assert_eq!(display_orders["down"], expected);
    }

    #[test]
    fn renderer_consumes_canonical_visual_tween_state() {
        let state = evaluate_renderer(
            r#"
const renderer = Object.create(window.PuzzleRenderer.prototype);
return renderer.tweenedVisualState({}, {
  visualTween: {
    from: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 350 }] },
    to: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 10 }] },
  },
}, 0.5);
"#,
        );

        assert_eq!(state["transforms"][0]["degrees"].as_f64(), Some(360.0));
    }

    #[test]
    fn renderer_interpolates_rotation_and_translation_with_one_progress_value() {
        let state = evaluate_renderer(
            r#"
const renderer = Object.create(window.PuzzleRenderer.prototype);
return renderer.tweenedVisualState({}, {
  visualTween: {
    from: { transforms: [
      { kind: "translate", space: "world", value: [0, 0, 0] },
      { kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 0 },
    ] },
    to: { transforms: [
      { kind: "translate", space: "world", value: [2, 4, 0] },
      { kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 90 },
    ] },
  },
}, 0.5);
"#,
        );

        assert_eq!(state["transforms"][0]["x"].as_f64(), Some(1.0));
        assert_eq!(state["transforms"][0]["y"].as_f64(), Some(2.0));
        assert_eq!(state["transforms"][1]["degrees"].as_f64(), Some(45.0));
    }

    #[test]
    fn renderer_resolves_independent_visual_and_position_channels_for_one_occurrence() {
        let state = evaluate_renderer(
            r#"
const visual = {
  kind: "move", name: "tween", occurrenceId: 1, objectId: 7,
  from: { x: 2, y: 3 }, to: { x: 2, y: 3 },
  visualTween: {
    from: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 0 }] },
    to: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 90 }] },
  },
};
const position = {
  kind: "move", name: "tween", occurrenceId: 1, objectId: 7,
  from: { x: 2, y: 3 }, to: { x: 3, y: 3 },
};
const source = [visual, position];
const renderer = Object.create(window.PuzzleRenderer.prototype);
const resolved = renderer.prepareAnimations(source, { x: 0, y: 0, width: 8, height: 8 });
return {
  sourceCount: source.length,
  sourcePositionHasVisualTween: Boolean(source[1].visualTween),
  resolvedCount: resolved.length,
  resolvedFrom: resolved[0].from,
  resolvedTo: resolved[0].to,
  resolvedHasVisualTween: Boolean(resolved[0].visualTween),
};
"#,
        );

        assert_eq!(state["sourceCount"], json!(2));
        assert_eq!(state["sourcePositionHasVisualTween"], json!(false));
        assert_eq!(state["resolvedCount"], json!(1));
        assert_eq!(state["resolvedFrom"], json!({"x": 2, "y": 3}));
        assert_eq!(state["resolvedTo"], json!({"x": 3, "y": 3}));
        assert_eq!(state["resolvedHasVisualTween"], json!(true));
    }

    #[test]
    fn renderer_composes_chained_channels_by_occurrence_identity() {
        let state = evaluate_renderer(
            r#"
const events = [
  { kind: "move", name: "tween", occurrenceId: 1, objectId: 7,
    from: { x: 0, y: 0 }, to: { x: 1, y: 0 } },
  { kind: "move", name: "tween", occurrenceId: 2, objectId: 7,
    from: { x: 1, y: 0 }, to: { x: 2, y: 0 } },
  { kind: "move", name: "tween", occurrenceId: 2, objectId: 8,
    from: { x: 2, y: 0 }, to: { x: 2, y: 0 },
    visualTween: {
      from: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 0 }] },
      to: { transforms: [{ kind: "rotate", space: "world", axis: [0, 0, 1], degrees: 90 }] },
    } },
];
return window.PuzzleVisualTweenCore.resolveAnimationChannels(events);
"#,
        );

        assert_eq!(state.as_array().unwrap().len(), 2);
        assert!(state[0].get("visualTween").is_none());
        assert_eq!(state[1]["occurrenceId"], json!(2));
        assert_eq!(state[1]["from"], json!({"x": 1, "y": 0}));
        assert_eq!(state[1]["to"], json!({"x": 2, "y": 0}));
        assert!(state[1].get("visualTween").is_some());
    }

    #[test]
    fn renderer_does_not_draw_fallback_visuals() {
        assert!(RENDERER_JS.contains("return null;"));
        assert!(RENDERER_JS.contains("const visual = this.renderLayerVisual(layer);"));
        assert!(!RENDERER_JS.contains("visual.className = `visual ${layer.visual}`;"));
        assert!(!RENDERER_JS.contains("this.paintFallbackLayer("));
        assert!(!RENDERER_JS.contains("function paintFallbackLayer("));
        assert!(!RENDERER_JS.contains("function hashString("));
        assert!(RENDERER_CSS.contains(".visual {"));
        assert!(RENDERER_CSS.contains("position: absolute;"));
        assert!(!RENDERER_CSS.contains(".visual.unknown"));
    }

    #[test]
    fn renderer_requires_the_snapshot_owned_typed_visual_catalog() {
        assert!(RENDERER_JS.contains("const visuals = this.lastScene?.visuals;"));
        assert!(RENDERER_JS.contains("runtime scene is missing its typed 2D visual catalog"));
        assert!(!RENDERER_JS.contains("return window.GameVisuals || {};"));
    }

    #[test]
    fn renderer_applies_snapshot_owned_visual_transforms() {
        assert!(RENDERER_JS.contains(
            "applyCanvasVisualTransforms(context, definition, unit, animation = null, progress = 1"
        ));
        assert!(RENDERER_JS.contains("for (const transform of [...transforms].reverse())"));
        assert!(RENDERER_JS.contains("tweenedVisualState(definition, animation, progress)"));
        assert!(RENDERER_JS.contains("if (!animation?.visualTween || progress >= 1)"));
        assert!(RENDERER_JS.contains("PuzzleVisualTweenCore.interpolate("));
        assert!(VISUAL_TWEEN_CORE_JS.contains("function interpolateTransform("));
        assert!(
            VISUAL_TWEEN_CORE_JS
                .contains("source.kind === \"translate\" || source.kind === \"scale\"")
        );
        assert!(VISUAL_TWEEN_CORE_JS.contains("interpolateOptionalNumber(from.opacity"));
        assert!(RENDERER_JS.contains("scale(-1, -1)"));
        assert!(RENDERER_JS.contains("visualCanvasFit(definition, unit, sourceSize = null)"));
        assert!(RENDERER_JS.contains("visualDrawBox(definition)"));
        assert!(RENDERER_JS.contains("solidColor && this.canPaintAsFullCellSolid(definition)"));
        assert!(!RENDERER_JS.contains("unit = Math.max(unit, cellCols, cellRows);"));
        assert!(
            RENDERER_JS.contains("const presentationUnit = this.canvasPresentationCellUnit();")
        );
        assert!(
            RENDERER_JS.contains(
                "mode === \"cover\" ? Math.max(scaleX, scaleY) : Math.min(scaleX, scaleY)"
            )
        );
        assert!(RENDERER_JS.contains("context.drawImage(\n          image,"));
        assert!(RENDERER_CSS.contains("--visual-box-cols"));
        assert!(RENDERER_CSS.contains("background-size: contain;"));
        assert!(!RENDERER_JS.contains("leastCommonMultiple("));
        assert!(
            RENDERER_JS.contains(
                "const { cols: width, rows: height } = this.visualPatternSize(definition);"
            )
        );
        assert!(!RENDERER_JS.contains("domPatternCellUnit()"));
        assert!(!RENDERER_JS.contains("scaledPixelEdge(index, sourceUnits, targetPixels)"));
        assert!(!RENDERER_JS.contains("boundedLeastCommonMultiple"));
        assert!(RENDERER_CSS.contains("overflow: visible;"));
    }

    #[test]
    fn browser_assets_do_not_republish_compiled_visual_catalogs() {
        let assets_source = include_str!("lib_assets.rs");
        assert!(!assets_source.contains("generated_visuals_js"));
        assert!(!assets_source.contains("runtime_puzzle2_visual_catalog"));
        assert!(!assets_source.contains("puzzle2_visual_catalog_value"));
        assert!(!assets_source.contains("window.GameVisuals = createVisuals"));
    }

    #[test]
    fn canvas_patterns_do_not_cross_a_per_visual_raster_boundary() {
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
    fn runtime_json_consumes_resolved_2d_grid_decorations() {
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
        let runtime =
            RuntimeSession::from_source(source, "grid_render.puzzle").expect("compile 2D runtime");
        let snapshot: Value = serde_json::from_str(&runtime.snapshot_json()).unwrap();
        let state = first_viewport_state(&snapshot);
        assert!(
            state["settings"].get("grid").is_none(),
            "authoring grid settings must be resolved before the presentation contract"
        );
        let decorations = state["renderScene"]["decorations"]
            .as_array()
            .expect("resolved render scene must own its grid decorations");
        assert!(matches!(
            decorations.as_slice(),
            [decoration]
                if decoration["kind"] == "lines2d"
                    && decoration["segments"]
                        .as_array()
                        .is_some_and(|segments| !segments.is_empty())
        ));
        assert_eq!(
            decorations[0]["style"]["width"],
            json!({
                "kind": "cell_relative",
                "cell_fraction": 1.0 / 24.0,
                "min_physical_pixels": 1.0,
            })
        );
        assert!(!RENDERER_JS.contains("scene.settings?.grid"));
        assert!(RENDERER_JS.contains("paintCanvasDecorations(context, scene, frame, unit)"));
    }

    #[test]
    fn html_play_fits_the_logical_scene_root_not_individual_cells() {
        assert!(INDEX_HTML.contains(r#"<div id="screenFrame" class="screen-frame">"#));
        assert!(!APP_CSS.contains("--scene-layout-unit"));
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
        assert!(APP_JS.contains("function fitSceneViewport("));
        assert!(APP_JS.contains("function currentSceneAspectRatio("));
        assert!(APP_JS.contains("function visibleViewportSize()"));
        assert!(APP_JS.contains("const rect = element.getBoundingClientRect();"));
        assert!(APP_JS.contains("Math.min(rect.right, viewport.width) - Math.max(rect.left, 0)"));
        assert!(APP_JS.contains("Math.min(rect.bottom, viewport.height) - Math.max(rect.top, 0)"));
        assert!(APP_JS.contains("screenView.style.setProperty(\"--screen-scale\""));
        assert!(
            APP_JS
                .contains("screenFrame.style.width = `min(${Math.ceil(viewport.width)}px, 100%)`;")
        );
        assert!(
            APP_JS.contains(
                "screenFrame.style.height = `min(${Math.ceil(viewport.height)}px, 100%)`;"
            )
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
        assert!(APP_CSS.contains("grid-template-columns: minmax(0, 1fr);"));
        assert!(APP_CSS.contains("grid-template-rows: minmax(0, 1fr);"));
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
    fn presentation_wait_timer_serially_resumes_the_waiting_session() {
        assert!(APP_JS.contains("function applyPresentationEvents(events)"));
        assert!(APP_JS.contains("function dispatchNextPresentationEvent()"));
        assert!(APP_JS.contains("const event = pendingPresentationEvents.shift();"));
        assert!(APP_JS.contains("window.setTimeout(() => {\n    if (waitTimer.done)"));
        assert!(APP_JS.contains("while (pendingPresentationEvents.length > 0)"));
        assert!(APP_JS.contains("startPresentationWait(event);\n      return;"));
        assert!(APP_JS.contains("event.kind === \"animation_batch\""));
        assert!(APP_JS.contains("applyPresentationAnimations(event, event.animations);"));
        assert!(APP_JS.contains("currentState.levelIndex !== event.levelIndex"));
        assert!(APP_JS.contains("runtimeViewportSourceState(event.source)"));
        assert!(!APP_JS.contains("event.scene"));
        assert!(!APP_JS.contains("event.puzzle"));
        assert!(APP_JS.contains("await postSessionAction({ kind: \"resume\" });"));
        assert!(!APP_JS.contains("/api/resume"));
        assert!(APP_JS.contains("resumesSession: sessionWaiting"));
        assert!(
            APP_JS.contains(
                "pendingSessionResume = pendingSessionResume || waitTimer.resumesSession;"
            )
        );
        assert!(
            APP_JS
                .contains("pendingModelInput && config.queueDuringWait && config.fastForwardWait")
        );
    }

    #[test]
    fn presentation_dispatch_consumes_explicit_animation_batches() {
        assert!(APP_JS.contains("event.kind === \"animation_batch\""));
        assert!(
            APP_JS
                .contains("if (!Array.isArray(event.animations) || event.animations.length === 0)")
        );
        assert!(APP_JS.contains("applyPresentationAnimations(event, event.animations);"));
        assert!(!APP_JS.contains("samePresentationContext"));
        let start = APP_JS
            .find("function applyPresentationAnimations(event, animations)")
            .unwrap();
        let end = APP_JS[start..]
            .find("function startPresentationWait(event)")
            .unwrap()
            + start;
        let body = &APP_JS[start..end];
        assert!(body.contains("const puzzleSnapshot = runtimeViewportSourceState(event.source);"));
        assert!(body.contains("const batchId = ++presentationAnimationBatchId;"));
        assert!(body.contains("puzzleSnapshot.animationEvents = animations;"));
        assert!(body.contains("puzzleSnapshot.animationBatchId = batchId;"));
        assert!(!body.contains("scenePuzzleState"));
        assert!(!body.contains("currentState.scene"));
        assert!(!body.contains("layer.scene"));
        assert_eq!(body.matches("renderSurface(currentState);").count(), 1);
    }

    #[test]
    fn queued_model_input_fast_forwards_each_later_wait_with_current_contract() {
        let start = APP_JS
            .find("function startPresentationWait(event)")
            .unwrap();
        let end = APP_JS[start..]
            .find("function inputBufferConfig()")
            .unwrap()
            + start;
        let body = &APP_JS[start..end];
        assert!(body.contains("const config = inputBufferConfig();"));
        assert!(
            body.contains("pendingModelInput && config.queueDuringWait && config.fastForwardWait")
        );
        assert!(body.contains("config,"));
        assert!(APP_JS.contains("if (!config.fastForwardWait) {\n    return;"));
        assert!(APP_JS.contains("Math.max(0, waitTimer.config.minWaitMs - elapsed)"));
    }

    #[test]
    fn html_play_choice_navigation_uses_session_selection_and_tokens() {
        assert!(APP_JS.contains("function resolvedChoiceNodes(components, choices = [])"));
        assert!(APP_JS.contains("component.selected === true"));
        assert!(APP_JS.contains("sendSceneActionToken(component.actionToken);"));
        assert!(!APP_JS.contains("standardChoiceInputForKey"));
        assert!(!APP_JS.contains("kind: \"choice_move\""));
        assert!(!APP_JS.contains("function standardChoiceFocusCells("));
        assert!(!APP_JS.contains("standardChoiceCursors"));
        assert!(!APP_JS.contains("level_menu"));
        assert!(!APP_JS.contains("level_selector"));
        assert!(!APP_JS.contains("level-clear-mark"));
        assert!(!APP_JS.contains("scene_menu"));
        assert!(!APP_JS.contains("sceneMenu"));
        assert!(!APP_JS.contains("is-menu-scene"));
        assert!(APP_CSS.contains("button.standard-choice.is-selected"));
    }

    #[test]
    fn runtime_styles_have_no_level_menu_specific_surface() {
        for css in [APP_CSS, PUZZLE3_STYLE_CSS] {
            assert!(!css.contains(".level-menu"));
            assert!(!css.contains(".view-list"));
            assert!(!css.contains("level-clear-mark"));
            assert!(!css.contains("is-menu-scene"));
        }
        assert!(APP_CSS.contains("button.standard-choice"));
    }

    #[test]
    fn scrollable_common_container_uses_reachable_origin_and_keeps_selected_choice_visible() {
        assert!(APP_JS.contains("if (component.layout?.scroll)"));
        assert!(APP_JS.contains("container.classList.add(\"is-scroll\")"));
        assert!(APP_JS.contains("function scrollSelectedChoiceIntoView(root = screenView)"));
        assert!(APP_JS.contains("const scroll = selected?.closest?.(\".is-scroll\")"));
        assert!(APP_CSS.contains(".view-column.is-scroll,"));
        assert!(APP_CSS.contains(
            ".view-box.is-scroll {\n  max-height: min(100%, calc(var(--screen-virtual-height, 100vh) - 96px));\n  max-width: 100%;\n  justify-content: flex-start;"
        ));
        assert!(APP_CSS.contains("overflow-y: auto;"));
    }

    #[test]
    fn html_play_consumes_only_the_typed_runtime_theme_contract() {
        for field in [
            "background",
            "mutedText",
            "controlFocused",
            "controlSelected",
            "controlSelectedBorder",
            "typography",
            "controlLayout",
        ] {
            assert!(APP_JS.contains(field));
        }
        assert!(APP_JS.contains("applyTheme(state?.theme);"));
        assert!(APP_JS.contains("function normalizeRuntimeTheme(theme)"));
        assert!(APP_JS.contains("color(srgb-linear"));
        assert!(!APP_JS.contains("puzzleBoot.theme"));
        assert!(!APP_JS.contains("theme?.variables"));
        assert!(!APP_JS.contains("themeClassName"));
        assert!(!APP_JS.contains("theme-clean"));
        assert!(!APP_JS.contains("theme-puzzlescript"));
        for css in [APP_CSS, PUZZLE3_STYLE_CSS] {
            assert!(!css.contains("--background: #"));
            assert!(!css.contains("--text: #"));
            assert!(!css.contains("--accent:"));
            assert!(!css.contains("--radius-control:"));
        }
    }

    #[test]
    fn html_play_commits_snapshot_before_dispatching_presentation_events() {
        let render_start = APP_JS.find("function render(state) {").unwrap();
        let render_body = &APP_JS[render_start..];
        let scene_index = render_body.find("renderSurface(state);").unwrap();
        let presentation_index = render_body
            .find("applyPresentationEvents(presentationEvents);")
            .unwrap();
        assert!(scene_index < presentation_index);
    }

    #[test]
    fn html_play_buffers_one_busy_model_input_without_a_string_command_path() {
        assert!(APP_JS.contains("let pendingModelInput = null;"));
        assert!(APP_JS.contains("let drainingQueuedModelInput = false;"));
        assert!(!APP_JS.contains("pendingCommandQueue"));
        assert!(APP_JS.contains("pendingModelInput = input;"));
        assert!(APP_JS.contains("function drainQueuedModelInput()"));
        assert!(!APP_JS.contains("function sendCommand("));
        assert!(!APP_JS.contains("function sendCommandNow("));
        assert!(APP_JS.contains("currentState?.busy || clientPendingWaits > 0"));
        assert!(APP_JS.contains("function inputBufferConfig()"));
        assert!(APP_JS.contains("if (!config.queueDuringWait)"));
        assert!(APP_JS.contains("function fastForwardActiveWaitsForQueuedInput"));
        assert!(APP_JS.contains("typeof source.fastForwardWait !== \"boolean\""));
        assert!(APP_JS.contains("minWaitMs: source.minWaitMs"));
        assert!(
            !APP_JS.contains("if (currentState.busy) {\n    return;\n  }\n  broadcastPuzzle3Key")
        );
        assert!(!APP_JS.contains("clientPendingAnimations"));
        assert!(!APP_JS.contains("clientPendingCommands"));
    }

    #[test]
    fn html_play_leaves_modal_keyboard_priority_to_the_runtime_owner() {
        assert!(APP_JS.contains("postSessionAction({ kind: \"key\", trigger });"));
        assert!(APP_JS.contains("await postSessionAction({ kind: \"scene_action\", token });"));
        assert!(
            APP_JS.contains("function bindAwaitedComponentEvent(root, instance, presentation)")
        );
        assert!(APP_JS.contains("presentation.events?.[eventName]"));
        assert!(!APP_JS.contains("activeModalComponent"));
        assert!(!APP_JS.contains("componentEventAcceptsKey"));
        assert!(!APP_JS.contains("isModalDismissKey"));
        assert!(APP_CSS.contains(".scene-layer.is-modal:focus {\n  outline: none;\n}"));
    }

    #[test]
    fn html_play_dispatches_one_typed_key_without_adapter_semantics() {
        assert!(APP_JS.contains("function runtimeKeyTriggerFromEvent(event)"));
        assert!(APP_JS.contains("postSessionAction({ kind: \"key\", trigger });"));
        assert!(!APP_JS.contains("effectsForKey"));
        assert!(!APP_JS.contains("standardSessionActionForKey"));
        assert!(!APP_JS.contains("standardChoiceInputForKey"));
        assert!(!APP_JS.contains("runtimeKeyTriggerMatches"));
        assert!(!APP_JS.contains("kind: \"choice_move\""));
        assert!(!APP_JS.contains("key === \"z\""));
        assert!(!APP_JS.contains("key === \"y\""));
        assert!(!APP_JS.contains("kind: \"scene_effect\""));
    }

    #[test]
    fn html_play_converts_platform_keys_to_the_typed_trigger_wire_shape() {
        let start = APP_JS
            .find("function runtimeKeyTriggerFromEvent(event) {")
            .unwrap();
        let end = APP_JS[start..]
            .find("function inputByName(name) {")
            .map(|offset| start + offset)
            .unwrap();
        let mut script = APP_JS[start..end].to_string();
        script.push_str(
            r#"
const cases = [
  runtimeKeyTriggerFromEvent({ key: "X" }),
  runtimeKeyTriggerFromEvent({ key: "ArrowLeft" }),
  runtimeKeyTriggerFromEvent({ key: " " }),
  runtimeKeyTriggerFromEvent({ key: "。" }),
  runtimeKeyTriggerFromEvent({ key: "Enter", ctrlKey: true }),
  runtimeKeyTriggerFromEvent({ key: "Compose" }),
];
process.stdout.write(JSON.stringify(cases));
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the typed key trigger contract test");
        assert!(
            output.status.success(),
            "typed key trigger evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result,
            json!([
                {"kind": "character", "value": "X"},
                {"kind": "arrow_left"},
                {"kind": "space"},
                {"kind": "character", "value": "。"},
                null,
                null,
            ])
        );
    }

    #[test]
    fn html_play_resolves_viewports_by_exact_typed_registry_identity() {
        let start = APP_JS
            .find("function runtimeViewportSourceId(value) {")
            .unwrap();
        let end = APP_JS[start..]
            .find("function focusedComponentName(state = currentState) {")
            .map(|offset| start + offset)
            .unwrap();
        let mut script = String::from("let currentState = null;\n");
        script.push_str(&APP_JS[start..end]);
        script.push_str(
            r#"
const state = {
  viewportSources: [
    { id: { component: "left", source: "board" }, state: { marker: "left" } },
    { id: { component: "right", source: "board" }, state: { marker: "right" } },
  ],
};
const exact = runtimeViewportSourceState({ component: "right", source: "board" }, state).marker;
let missingError = "";
let registryError = "";
let untypedError = "";
try {
  runtimeViewportSourceState({ component: "other", source: "board" }, state);
} catch (error) { missingError = error.message; }
try {
  runtimeViewportSourceState({ component: "right", source: "board" }, {});
} catch (error) { registryError = error.message; }
try {
  runtimeViewportSourceState("board", state);
} catch (error) { untypedError = error.message; }
process.stdout.write(JSON.stringify({ exact, missingError, registryError, untypedError }));
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the typed viewport registry contract test");
        assert!(
            output.status.success(),
            "typed viewport registry evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["exact"], "right");
        assert_eq!(
            result["missingError"],
            "Viewport source is missing from the runtime registry: other/board"
        );
        assert_eq!(
            result["registryError"],
            "Runtime snapshot is missing the typed viewport source registry"
        );
        assert_eq!(
            result["untypedError"],
            "Resolved viewport is missing its typed component/source identity"
        );
    }

    #[test]
    fn browser_host_does_not_own_audio_generation_or_playback_semantics() {
        let crate_source = include_str!("lib.rs");
        let export_source = include_str!("lib_export.rs");
        let server_source = include_str!("lib_server.rs");
        assert!(!APP_JS.contains("PuzzleSoundRuntime"));
        assert!(!APP_JS.contains("AudioContext"));
        assert!(!APP_JS.contains("PuzzleSoundGenerator"));
        assert!(!APP_JS.contains("PuzzleSoundTools"));
        assert!(!APP_JS.contains("state?.sounds"));
        assert!(!INDEX_HTML.contains("/sound-generator.js"));
        assert!(!crate_source.contains(concat!("SEEDED_", "SFX_JS")));
        assert!(!crate_source.contains(concat!("SEEDED_", "MUSIC_JS")));
        assert!(!export_source.contains("fn sound_tools_js"));
        assert!(!server_source.contains("/sound-generator.js"));
    }

    #[test]
    fn browser_host_awaits_unlock_and_wakes_typed_audio_feedback_from_device_events() {
        assert!(APP_JS.contains("await standaloneRuntime.unlockAudio();"));
        assert!(APP_JS.contains("document.addEventListener(\"keydown\", async () => {"));
        assert!(APP_JS.contains("document.addEventListener(\"pointerdown\", async () => {"));
        assert!(APP_JS.contains("await unlockAudioFromGesture();"));
        assert!(APP_JS.contains("await standaloneRuntime.setAudioVisible(visible);"));
        assert!(APP_JS.contains("document.addEventListener(\"visibilitychange\", async () => {"));
        assert!(STANDALONE_JS.contains("await this.sessionRuntime.unlock_audio("));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.set_audio_visible("));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.set_audio_feedback_wakeup("));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.audio_feedback_event("));
        assert!(!APP_JS.contains("tickAudioConsumer"));
        assert!(!STANDALONE_JS.contains("audio_tick"));
        assert!(STANDALONE_JS.contains("console.error(`Audio consumer: ${diagnostic}`);"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn live_server_preserves_non_audio_events_and_rejects_audio_at_its_typed_boundary() {
        use puzzle_audio_contract::{AudioCommand, SfxAssetId};
        use puzzle_runtime_contract::{RuntimePresentationEvent, RuntimeViewportSourceId};

        let wait = RuntimePresentationEvent::Wait { milliseconds: 10 };
        let animation = RuntimePresentationEvent::AnimationBatch {
            source: RuntimeViewportSourceId {
                component: "board".to_string(),
                source: "puzzle".to_string(),
            },
            level_index: Some(0),
            animations: Vec::new(),
        };
        let (public, rejected_audio) = project_live_server_events(vec![
            wait.clone(),
            RuntimePresentationEvent::Audio {
                command: AudioCommand::PlaySfx {
                    asset: SfxAssetId(0),
                },
            },
            animation.clone(),
        ]);

        assert_eq!(public, vec![wait, animation]);
        assert_eq!(rejected_audio, 1);
    }

    #[test]
    fn standalone_export_keeps_audio_recipes_out_of_browser_boot_data() {
        let source = r#"
title = Sfx Volume

sounds {
  sfx click { seed = click; type = select; volume = 1.25 }
  music loop { seed = loop; bars = 16; height = 0.62; bpm = 104; volume = 1.5 }
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

        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        assert!(runtime_export.get("sounds").is_none());
        let rust_owned_sounds = &runtime_export["runtimeLoadedDocument"]["sounds"];
        assert_eq!(rust_owned_sounds["sfx"][0]["name"], "click");
        assert_eq!(rust_owned_sounds["music"][0]["name"], "loop");
        assert!(!html.contains("PuzzleSoundRuntime"));
        assert!(!html.contains("PuzzleSoundGenerator"));
        assert!(!html.contains("/sound-generator.js"));
    }

    #[test]
    fn html_play_consumes_only_resolved_surface_presentations() {
        assert!(APP_JS.contains("const presentation = layer.presentation;"));
        assert!(APP_JS.contains("const components = presentation.components || [];"));
        assert!(!APP_JS.contains("source?.scenes"));
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
    fn puzzle3_component_does_not_fallback_to_empty_snapshot_when_fixture_load_fails() {
        assert!(PUZZLE3_COMPONENT_JS.contains("async function loadInitialPuzzle3Snapshot()"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "throw new Error(\"Puzzle3 component requires an explicit view snapshot.\");"
        ));
        assert!(!PUZZLE3_COMPONENT_JS.contains("fetch(\"./fixture.json\""));
        assert!(!PUZZLE3_COMPONENT_JS.contains("window.Puzzle3DFixture"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function validatePuzzle3ViewSnapshot("));
        assert!(PUZZLE3_COMPONENT_JS.contains("function requireLoadedPuzzle3Snapshot("));
        assert!(PUZZLE3_COMPONENT_JS.contains("function showPuzzle3LoadError(error)"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("controllerApi.ready = loadPuzzle3ComponentSnapshot();")
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("catch {\n    nextSnapshot = fallbackSnapshot;"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("normalizeSnapshot(source || fallbackSnapshot)"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("snapshot || fallbackSnapshot"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("source || fallbackSnapshot"));
    }

    #[test]
    fn puzzle3_component_consumes_only_a_completed_view_snapshot() {
        assert!(PUZZLE3_COMPONENT_JS.contains("window.Puzzle3Component"));
        assert!(PUZZLE3_COMPONENT_JS.contains("validateSnapshot(source)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("controllerOptions.onError?.("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("runtimeContract"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("WasmPuzzle3Runtime"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("document.querySelector"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("URLSearchParams"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("postMessage"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("document.title"));
        assert!(APP_JS.contains("function reportPuzzle3ComponentError(failure = {})"));
    }

    #[test]
    fn puzzle3_session_projection_keeps_render_fixture_host_owned() {
        let source = r#"
title = "Runtime boundary"

puzzle cube {
  dimension = 3
  layers {
    actor = Player
  }
  rules {
  }
}

levels default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}

scene playing {
  layout {
    puzzle board = cube
  }
}
"#;
        let document = puzzle_lang::parse_game_for_path(source, "runtime_boundary.puzzle3")
            .expect("compile Puzzle3 document");
        let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(&document)
            .expect("export Puzzle3 render fixture");
        let fixture: Value = serde_json::from_str(&fixture_json).unwrap();
        let result = validate_puzzle3_fixture_with_browser_contract(&fixture);
        let bridge = RuntimeSession::from_source(source, "runtime_boundary.puzzle3")
            .expect("compile Puzzle3 session projection");
        let session: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        let view = first_viewport_state(&session);

        assert_eq!(result["size"], json!({"width": 1, "depth": 1, "height": 1}));
        assert_eq!(result["cellCount"], 1);
        assert!(result["inputCount"].as_u64().is_some_and(|count| count > 0));
        assert!(view.get("render").is_some());
        assert!(view.get("objects").is_none());
        assert!(view.get("visuals").is_none());
        assert!(
            view["cells"][0]["objects"][0].get("id").is_some(),
            "projected Puzzle3 session view: {view}"
        );
        assert!(APP_JS.contains("function mergePuzzle3SessionSnapshot(fixture, sessionSnapshot)"));
        assert!(APP_JS.contains("Puzzle3 session references unknown object id"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("model.kind"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("runtimeModel.rules"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("runtimeModel.winCondition"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("game.inputs"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("stateFromRuntimeCells"));
    }

    #[test]
    fn html_play_dispatches_resolved_scene_action_tokens() {
        assert!(APP_JS.contains("await postSessionAction({ kind: \"scene_action\", token });"));
        assert!(!APP_JS.contains("kind: \"scene_effect\""));
        assert!(!APP_JS.contains("function effectToCommand("));
        assert!(!APP_JS.contains("function exprSource("));
    }

    #[test]
    fn puzzle3_preview_updates_are_owned_by_the_browser_host() {
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
        assert!(APP_JS.contains(
            "window.applyPuzzleStudioPreviewSurfaceUpdate = applyPuzzleStudioPreviewSurfaceUpdate;"
        ));
        assert!(APP_JS.contains("entry.controller?.replaceSnapshot(snapshot);"));
        let stripped = strip_optional_host_blocks(APP_JS, "puzzle3");
        assert!(!stripped.contains("normalizePuzzle3PreviewSurface("));
        assert!(!stripped.contains("PuzzleStudioInitialPreviewSurfaceConsumed"));
        assert!(!stripped.contains("effectiveComponentEmbedMode"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("PuzzleStudioPreviewSurfaceUpdate"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("PuzzleStudioRenderPuzzle3ModelComponent"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("puzzle3PreviewSnapshot"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("applyPuzzle3PreviewResources"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("mergePuzzle3PreviewRender"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("update(update = {})"));
        assert!(PUZZLE3_COMPONENT_JS.contains("replaceSnapshot(nextSnapshot)"));
        assert!(PUZZLE3_COMPONENT_JS.contains(r#"coordinateSpace: "canvas-css-px""#));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "const target = source?.target || source?.origin || modelCenterForSize(size);"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains("function modelCenterForSize(size)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("view.originX = width / 2;"));
        assert!(!PUZZLE3_COMPONENT_JS.contains(") / 2 + (Number(target.x) || 0)"));
    }

    #[test]
    fn optional_host_blocks_remove_their_indented_marker_lines() {
        let source = "before\n    /* puzzle-host:optional:debug:start */\n    removed();\n    /* puzzle-host:optional:debug:end */\nafter\n";
        assert_eq!(
            strip_optional_host_blocks(source, "debug"),
            "before\nafter\n"
        );
    }

    #[test]
    fn puzzle3_preview_requires_typed_source_identity_without_defaults() {
        let source_id_start = APP_JS
            .find("function runtimeViewportSourceId(value) {")
            .unwrap();
        let source_id_end = APP_JS[source_id_start..]
            .find("function runtimeViewportSourceState(sourceId, state = currentState) {")
            .map(|offset| source_id_start + offset)
            .unwrap();
        let normalize_start = APP_JS
            .find("function normalizePuzzle3PreviewSurface(update = null) {")
            .unwrap();
        let normalize_end = APP_JS[normalize_start..]
            .find("function legacyPuzzle3LevelPreviewPayload(update = {}) {")
            .map(|offset| normalize_start + offset)
            .unwrap();
        let mut script = String::from(
            r#"
const PREVIEW_SURFACE_UPDATE_MESSAGE = "PuzzleStudioPreviewSurfaceUpdate";
const PUZZLE3_MODEL_COMPONENT_PREVIEW_MESSAGE = "PuzzleStudioRenderPuzzle3ModelComponent";
const PUZZLE3_LEVEL_PREVIEW_KIND = "puzzle3-level";
const ISOLATED_PREVIEW_MODE = "isolated";
"#,
        );
        script.push_str(&APP_JS[source_id_start..source_id_end]);
        script.push_str(&APP_JS[normalize_start..normalize_end]);
        script.push_str(
            r#"
const base = {
  type: PREVIEW_SURFACE_UPDATE_MESSAGE,
  kind: PUZZLE3_LEVEL_PREVIEW_KIND,
  mode: ISOLATED_PREVIEW_MODE,
  payload: {},
};
const valid = normalizePuzzle3PreviewSurface({
  ...base,
  scene: "playing",
  component: {
    kind: "puzzle3",
    source: { component: "playing", source: "board" },
  },
});
let stringError = "";
let missingError = "";
try {
  normalizePuzzle3PreviewSurface({
    ...base,
    component: { kind: "puzzle3", source: "board" },
  });
} catch (error) { stringError = error.message; }
try {
  normalizePuzzle3PreviewSurface(base);
} catch (error) { missingError = error.message; }
process.stdout.write(JSON.stringify({ valid, stringError, missingError }));
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the Puzzle3 preview source contract test");
        assert!(
            output.status.success(),
            "Puzzle3 preview source evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result["valid"]["component"]["source"],
            json!({ "component": "playing", "source": "board" })
        );
        assert_eq!(
            result["stringError"],
            "Resolved viewport is missing its typed component/source identity"
        );
        assert_eq!(
            result["missingError"],
            "Puzzle3 preview update is missing its typed component"
        );
        assert!(!APP_JS.contains("update.source || \"__editor_model_preview__\""));
        assert!(!APP_JS.contains("surface.component || {"));
    }

    #[test]
    fn puzzle3_component_does_not_own_scene_layout_rendering() {
        assert!(
            !PUZZLE3_COMPONENT_JS.contains("function renderSceneNode("),
            "puzzle3_component.js must render a puzzle3 component, not own the generic scene layout renderer"
        );
        assert!(
            !PUZZLE3_COMPONENT_JS.contains("function renderSceneContainer("),
            "generic scene containers belong to the shared scene renderer"
        );
        assert!(
            !PUZZLE3_COMPONENT_JS.contains("function measureSceneNode("),
            "generic scene measurement belongs to the shared scene renderer"
        );
        assert!(
            !PUZZLE3_COMPONENT_JS.contains("function renderSceneFor("),
            "generic scene for-loops belong to the shared scene renderer"
        );
    }

    #[test]
    fn puzzle3_component_supports_focus_relative_viewport_framing() {
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function fitProjectionToViewport(renderContext, options = {})")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "function viewportFramingProjectionBounds(size, camera, viewport, focusCell)"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains("viewport.framingBox.height === \"full\""));
        assert!(PUZZLE3_COMPONENT_JS.contains("function scheduleViewportAnimation()"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("target.follow !== \"smooth\" || view.viewportSnapNext")
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("function smoothViewportOrigin(nextX, nextY, target)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("function smoothViewportMaxLag(target)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const amount = 0.12;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function requestSceneViewportDraw()"));
        assert!(PUZZLE3_COMPONENT_JS.contains("if (smoothViewportActive())"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function smoothViewportActive()"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const advanceViewport = options.advanceViewport !== false;")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("fitProjectionToViewport(renderContext, { advanceViewport })")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("if (options.advanceViewport === false)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("target.cellScale * projectionZoom(camera) * 3.5"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const SCENE_DEFAULT_WIDTH = 16;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const SCENE_DEFAULT_HEIGHT = 12;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function puzzle3SceneDisplaySize()"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function currentPuzzle3IntrinsicSize()"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "function viewportFitForFrame(frame, viewportBounds, centerPoint = null, zoom = 1, follow = \"snap\")"
        ));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function viewportFramingProjectionCenter"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains(
                "function viewportFocusProjectionAnchor(size, camera, viewport, focusCell)"
            )
        );
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "function viewportFocusVisualProjectionAnchor(size, camera, viewport, focusCell)"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "for (const voxel of objectVoxels(focusCell.position || {}, object, sourceKey))"
        ));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function viewportFramingRanges(size, viewport, focusCell)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("function virtualCenteredCellRange(center, span)"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "const xRange = viewportCellRange(Number(position.x) || 0, viewport.framingBox.width, viewport.mode);"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "const yRange = viewportCellRange(Number(position.y) || 0, viewport.framingBox.depth, viewport.mode);"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            ": viewportCellRange(Number(position.z) || 0, viewport.framingBox.height, viewport.mode);"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains("function virtualPagedCellRange(center, span)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("viewport?.mode === \"centered\" || viewport?.mode === \"paged\"")
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("function centeredCellRange(center, span, limit)"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "const anchorPoint = viewportFocusProjectionAnchor(size, camera, viewport, focus);"
        ));
        assert!(
            PUZZLE3_COMPONENT_JS.contains(
                "const anchorX = Number.isFinite(centerX) ? centerX : (minX + maxX) / 2;"
            )
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("originY: frameHeight / 2 - anchorY * effectiveScale")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("viewportFitForFrame("));
        assert!(PUZZLE3_COMPONENT_JS.contains("function puzzle3RenderContext(width = canvas.clientWidth, height = canvas.clientHeight)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function canvasLayoutFrame()"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("Number(canvas.clientWidth) || Number(rect.width) || 1")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("const frame = canvasLayoutFrame();"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function normalizeFrame(frame)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function normalizeModelSize(size)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function fitScaleForProjectedBounds(frame, bounds, margin = 0)")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const candidates = renderCellCandidates(renderContext);")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function renderCellCandidates(renderContext = puzzle3RenderContext())")
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("function viewportRenderCullingEnabled(renderContext)")
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("function viewportRenderPixelMargin"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function projectedCellPixelMargin"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function cellProjectsIntoFrame(position, frame)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("cellProjectsIntoFrame(cell.position || {}, renderContext.frame)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("bounds.maxX >= 0"));
        assert!(PUZZLE3_COMPONENT_JS.contains("bounds.minX <= frame.width"));
        assert!(PUZZLE3_COMPONENT_JS.contains("bounds.maxY >= 0"));
        assert!(PUZZLE3_COMPONENT_JS.contains("bounds.minY <= frame.height"));
        assert!(PUZZLE3_COMPONENT_JS.contains("cellHasRenderableVoxels(cell)"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains(
                "const effectiveScale = baseScale * Puzzle3VisualCore.normalizeZoom(zoom);"
            )
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("cellScale: baseScale"));
    }

    #[test]
    fn browser_keyboard_dispatch_is_dimension_independent() {
        assert!(APP_JS.contains("function dispatchKeyboardInput(event)"));
        assert!(APP_JS.contains("const trigger = runtimeKeyTriggerFromEvent(event);"));
        assert!(APP_JS.contains("postSessionAction({ kind: \"key\", trigger });"));
        assert!(APP_JS.contains("if (dispatchKeyboardInput(event))"));
        assert!(APP_JS.contains("dispatchKeyboardInput(keyEvent);"));
        assert!(APP_JS.contains("repeat: event.data.repeat === true"));
        assert!(!APP_JS.contains("broadcastPuzzle3Key"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("applyKey(event)"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("releaseKey(event)"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("heldSceneInputs"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("queuedSceneInputs"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("enqueueSceneInput"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("inputForRawInput"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("onInput"));
    }

    #[test]
    fn puzzle3_component_does_not_render_missing_visual_fallback_cube() {
        assert!(PUZZLE3_COMPONENT_JS.contains("if (!object.visual) {\n    return [];\n  }"));
        assert!(PUZZLE3_COMPONENT_JS.contains("if (!template) {\n    return [];\n  }"));
        assert!(PUZZLE3_COMPONENT_JS.contains("if (!visual) {\n    return null;\n  }"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("cssVar(\"--top\") || \"#ffde8a\""));
        assert!(!PUZZLE3_COMPONENT_JS.contains("red_cube"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("Red Cube"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("Bumpy"));
    }

    #[test]
    fn puzzle3_component_culls_only_opaque_internal_voxel_faces_across_cells() {
        assert!(PUZZLE3_COMPONENT_JS.contains("function renderOpaqueOcclusion(renderContext)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("for (const cell of snapshot.cells || [])"));
        assert!(PUZZLE3_COMPONENT_JS.contains("renderContext.opaqueOcclusion = occupied;"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function cellVisibleVoxelsForRender(cell, renderContext = null)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("renderContext.visibleVoxelCells = new Map();"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("function isVoxelFaceOccluded(voxel, offset, occupied)")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("if (voxel.opaque !== false && occupied.opaque.has(adjacentKey))")
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("occupied.bySource.has(`${sourceKey}|${adjacentKey}`)")
        );
    }

    #[test]
    fn puzzle3_component_preserves_alpha_voxel_layers_for_depth_sorting() {
        assert!(PUZZLE3_COMPONENT_JS.contains("function visibleVoxelStack(stack)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const visible = [];"));
        assert!(PUZZLE3_COMPONENT_JS.contains("opaque: source.a >= 0.999"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("if (renderVoxel.opaque) {\n      visible.length = 0;")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("visible.push(renderVoxel);"));
        assert!(PUZZLE3_COMPONENT_JS.contains("voxels.push(...visibleStack);"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function compositeVoxelStack(stack)"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function compositeColor(source, destination)"));
    }

    #[test]
    fn puzzle3_component_caches_static_visual_voxel_templates() {
        assert!(PUZZLE3_COMPONENT_JS.contains("const visualVoxelTemplateCache = new WeakMap();"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function visualVoxelTemplate(visualName)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function buildVisualVoxelTemplate(visual)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function instantiateVisualVoxelTemplate(position, template, sourceKey = null, objectOrder = 0)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("visualVoxelTemplateCache.get(visual)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("visualVoxelTemplateCache.set(visual, template)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("localBounds: voxelBounds(localPosition, scale)"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "const sourceGrid = standardVisualGridPosition({ width, depth, height }, col, row, z);"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains("x: (sourceGrid.x + 0.5 - width / 2) * scale"));
        assert!(PUZZLE3_COMPONENT_JS.contains("y: (sourceGrid.y + 0.5 - depth / 2) * scale"));
        assert!(PUZZLE3_COMPONENT_JS.contains("z: (sourceGrid.z + 0.5 - height / 2) * scale"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("const source = voxel.color || parseColor(voxel.fill);")
        );
    }

    #[test]
    fn puzzle3_component_caches_render_geometry_by_dirty_cells() {
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const renderGeometryCache = createRenderGeometryCache();")
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("function syncRenderGeometryCache(renderContext = null)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("function renderCellSignature(cell)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function expandDirtyCellKeys(keys)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("for (const offset of faceNeighborOffsets())"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function rebuildVisibleCellGeometry(key, cell, signature)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("function rebuildCachedCellFaces(key, cell)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("renderGeometryCache.occupied = renderCachedOpaqueOcclusion();")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("function cellFaceGeometriesForRender(cell, renderContext = null)")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("faces.push(...cellFaceGeometriesForRender(cell, renderContext).map(projectFaceGeometry));"));
        assert!(PUZZLE3_COMPONENT_JS.contains("face: (group, rect) => faceGeometry("));
        assert!(PUZZLE3_COMPONENT_JS.contains("function projectFaceGeometry(geometry)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const primitive = geometry.primitive || {"));
        assert!(PUZZLE3_COMPONENT_JS.contains("geometry.primitive = primitive;"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("primitive.ownerCell = projectCellRenderOwner(geometry.ownerCell);")
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("compoundFace:"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function compoundPolygonPaths(paths, fill)"));
    }

    #[test]
    fn mixed_document_projects_each_viewport_from_its_world_dimension() {
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

puzzle cube {
  dimension = 3
  layers {
    actor = Player Box Wall
  }

  groups {
    solid = Player Box Wall
  }

  rules {

  }
}

levels cube_levels of cube {
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
      puzzle cube_board = cube
    }
}
}
"#;
        let document = puzzle_lang::parse_game(source).unwrap();
        assert!(matches!(
            document.models.as_slice(),
            [
                LoadedDocumentModel::Puzzle2d { name: flat, .. },
                LoadedDocumentModel::Puzzle3d { name: cube, .. }
            ] if flat == "flat" && cube == "cube"
        ));
        let mixed = document
            .scenes
            .iter()
            .find(|scene| scene.name == "mixed_play")
            .unwrap();
        assert!(matches!(
            mixed.state.puzzles.as_slice(),
            [flat, cube]
                if flat.model == "flat"
                    && cube.model == "cube"
        ));
        assert!(matches!(
            mixed.components.as_slice(),
            [SceneComponent::Row(row)]
                if matches!(
                    row.children.as_slice(),
                    [SceneComponent::Viewport(flat), SceneComponent::Viewport(cube)]
                        if flat.projection == puzzle_lang::ViewportProjectionDef::TwoD
                            && flat.source == "flat_board"
                            && cube.projection == puzzle_lang::ViewportProjectionDef::ThreeD
                            && cube.source == "cube_board"
                )
        ));
    }

    #[test]
    fn mixed_microban_keeps_scene_metadata_dimension_independent() {
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

puzzle microban3d {
dimension = 3
layers {
actor = Player
}
rules {

}
}

levels microban of microban3d {
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
heading "Microban"
column {
choice "microban_03" -> goto playing("microban_03")
choice "microban_04" -> goto playing("microban_04")
}
}
}

scene playing(level) {
layout {
text level.title
}
}
"#;
        let document = puzzle_lang::parse_game(source).unwrap();

        assert_eq!(document.models.len(), 2);
        assert!(matches!(
            document.models.as_slice(),
            [
                LoadedDocumentModel::Puzzle2d { .. },
                LoadedDocumentModel::Puzzle3d { .. }
            ]
        ));
        assert!(
            document
                .scenes
                .iter()
                .any(|scene| scene.name == "level_select")
        );
        assert!(document.scenes.iter().any(|scene| scene.name == "playing"));
    }

    #[test]
    fn puzzle3_component_does_not_own_scene_component_rendering() {
        assert!(!PUZZLE3_COMPONENT_JS.contains("function renderSceneNode("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function renderSceneContainer("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function renderSceneFor("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function measureSceneNode("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("scene-component-"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("component.kind === \"button\""));
        assert!(!PUZZLE3_COMPONENT_JS.contains("component.kind === \"choice\""));
    }

    #[test]
    fn puzzle3_component_requires_typed_runtime_source_identity() {
        let start = PUZZLE3_COMPONENT_JS
            .find("function puzzle3ComponentSourceIdentity(component, sceneName) {")
            .unwrap();
        let end = PUZZLE3_COMPONENT_JS[start..]
            .find("function applySceneComponentMetadata(component, sceneName) {")
            .map(|offset| start + offset)
            .unwrap();
        let mut script = PUZZLE3_COMPONENT_JS[start..end].to_string();
        script.push_str(
            r#"
const valid = puzzle3ComponentSourceIdentity(
  { kind: "puzzle3", source: { component: "playing", source: "board" } },
  "playing",
);
let stringError = "";
let mismatchError = "";
try {
  puzzle3ComponentSourceIdentity({ kind: "puzzle3", source: "board" }, "playing");
} catch (error) { stringError = error.message; }
try {
  puzzle3ComponentSourceIdentity(
    { kind: "puzzle3", source: { component: "overlay", source: "board" } },
    "playing",
  );
} catch (error) { mismatchError = error.message; }
process.stdout.write(JSON.stringify({ valid, stringError, mismatchError }));
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the Puzzle3 typed source contract test");
        assert!(
            output.status.success(),
            "Puzzle3 typed source evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result["valid"],
            json!({ "component": "playing", "source": "board" })
        );
        assert_eq!(
            result["stringError"],
            "Puzzle3 component is missing its typed component/source identity."
        );
        assert_eq!(
            result["mismatchError"],
            "Puzzle3 component source overlay/board does not belong to scene playing."
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("component?.source || \"board\""));
        assert!(!PUZZLE3_COMPONENT_JS.contains("mountedPuzzle3Component?.source || \"board\""));
    }

    #[test]
    fn puzzle3_lifecycle_effect_semantics_are_session_runtime_owned() {
        assert!(!PUZZLE3_COMPONENT_JS.contains("LifecycleEffects"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("onLifecycleEffects"));
        assert!(APP_JS.contains("applyPresentationEvents(presentationEvents);"));
        assert!(APP_JS.contains("await postSessionAction({ kind: \"scene_action\", token });"));
        assert!(!APP_JS.contains("kind: \"scene_effect\""));
        assert!(!APP_JS.contains("function puzzleEffectCommand("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("function applyRuntimeLifecycleEffect("));
        assert!(!PUZZLE3_COMPONENT_JS.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("effect.kind === \"message\""));
        assert!(!PUZZLE3_COMPONENT_JS.contains("message-popup"));
    }

    #[test]
    fn puzzle3_component_camera_pitch_allows_vertical_view() {
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const PUZZLE3_COMPONENT_CAMERA_MIN_PITCH_DEGREES = -90;")
        );
        assert!(
            PUZZLE3_COMPONENT_JS.contains("const PUZZLE3_COMPONENT_CAMERA_MAX_PITCH_DEGREES = 90;")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("PUZZLE3_COMPONENT_CAMERA_MAX_PITCH_DEGREES"));
        assert!(!PUZZLE3_COMPONENT_JS.contains("camera.pitchDegrees - deltaY * 0.25, -80, 80"));
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
        assert!(PUZZLE3_COMPONENT_JS.contains("primitives = orderScenePrimitives(primitives);"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("return Puzzle3VisualCore.comparePrimitiveOrder(a, b);")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("function orderScenePrimitives(primitives)"));
        assert!(PUZZLE3_COMPONENT_JS.contains(
            "view.primitiveSortCacheOrder.map((stableKey) => byStableKey.get(stableKey))"
        ));
        assert!(PUZZLE3_COMPONENT_JS.contains("primitive.frameIndex = index;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("primitive.stableKey = occurrence === 0 ? baseKey"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function primitiveSortCacheKey(primitives)"));
        assert!(PUZZLE3_COMPONENT_JS.contains("cameraOrderKey(),"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("compareNumber(a.frameIndex, b.frameIndex)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("return Puzzle3VisualCore.cameraOrderKey(puzzle3VisualView());")
        );
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("return Puzzle3VisualCore.faceGridOrder(corners, puzzle3VisualView());")
        );
        assert!(
            PUZZLE3_VISUAL_CORE_JS.contains("function evaluateSpatialVisualAffine(operations)")
        );
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function transformSpatialPoint(point, affine)"));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("Puzzle3VisualCore.evaluateSpatialVisualAffine(visual.spatialOps)")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("Puzzle3VisualCore.evaluateSpatialVisualAffine(visual.spatialOps)")
        );
    }

    #[test]
    fn puzzle3_component_applies_pixelate_as_canvas_postprocess() {
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const pixelateBuffer = document.createElement(\"canvas\");")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("applyPixelatePostprocess();"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function pixelateSettings()"));
        assert!(PUZZLE3_COMPONENT_JS.contains("const raw = snapshot.render.pixelate;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("function applyPixelatePostprocess()"));
        assert!(
            PUZZLE3_COMPONENT_JS.contains("bufferCtx.imageSmoothingEnabled = settings.smoothing;")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("ctx.imageSmoothingEnabled = false;"));
        assert!(PUZZLE3_COMPONENT_JS.contains("ctx.setTransform(1, 0, 0, 1, 0, 0);"));
    }

    #[test]
    fn standalone_again_turns_are_owned_by_the_session_runtime() {
        assert!(STANDALONE_JS.contains("this.sessionRuntime.dispatch("));
        assert!(!STANDALONE_JS.contains("scheduleAgainTurn"));
        assert!(!STANDALONE_JS.contains("runAgainTurn"));
        assert!(!STANDALONE_JS.contains("pendingAgainTurns"));
    }

    #[test]
    fn standalone_runtime_rejects_untyped_command_strings() {
        let server_source = include_str!("lib_server.rs");
        assert!(!server_source.contains("session_action_from_http"));
        assert!(!server_source.contains("/api/command/"));
        assert!(!server_source.contains("/api/input/"));
        assert!(!server_source.contains("/api/debug/input/"));
        assert!(!STANDALONE_JS.contains("applyCommandName(commandName)"));
        assert!(!STANDALONE_JS.contains("this.sessionRuntime.apply_command_name(commandName)"));
        assert!(!STANDALONE_JS.contains(r#"return { kind: "command", name };"#));
        assert!(!STANDALONE_JS.contains("/api/command/"));
        assert!(!STANDALONE_JS.contains("parseRuntimeSceneTarget(value)"));
        assert!(!STANDALONE_JS.contains("parseRuntimeExpr"));
    }

    #[test]
    fn editor_preview_input_bridge_has_no_string_command_fallback() {
        assert!(!APP_JS.contains("PuzzleStudioCommand"));
        assert!(!APP_JS.contains("PuzzleStudioInput"));
        assert!(!APP_JS.contains("function isStandaloneEditorSessionCommand("));
        assert!(!APP_JS.contains("function applyStandaloneEditorInput("));
        assert!(!APP_JS.contains("/api/command/"));
        assert!(APP_JS.contains("await postSessionAction(input.action);"));
    }

    #[test]
    fn standalone_runtime_requires_wasm_player_runtime_for_play() {
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
        assert!(APP_JS.contains(
            "|| (state.viewportSources || []).some((source) => source?.id?.component === state.surface?.focus)"
        ));
        assert!(APP_JS.contains("function currentSceneAcceptsModelInput()"));
        assert!(
            APP_JS.contains(
                "function sceneInteractionProfile(scene = currentSceneDef(), options = {})"
            )
        );
        assert!(APP_JS.contains("function stateAcceptsModelInput(state = currentState)"));
        assert!(APP_JS.contains("state?.acceptsModelInput === true"));
        assert!(APP_JS.contains("standaloneRuntime?.editorPreviewInputEnabled === true"));
        assert!(!APP_JS.contains("nonEmptyArray(layer?.scenePuzzles)"));
        assert!(!APP_JS.contains("function sceneChromeProfile(profile)"));
        assert!(!APP_JS.contains("menuFocusCells"));
        assert!(APP_JS.contains("await sendModelInput(input.name);"));
        assert!(APP_JS.contains("return postSessionAction({ kind: \"input\", name: input });"));
        assert!(!APP_JS.contains("/api/input/"));
        assert!(!APP_JS.contains("sceneIsMenuLike"));
        assert!(!APP_JS.contains("const hasPuzzle = sceneHasComponent(sceneDef, \"puzzle\") || sceneHasComponent(sceneDef, \"frame\")"));
        assert!(APP_JS.contains("acceptModelInput: event.data.acceptModelInput === true"));
        assert!(!APP_JS.contains("function applyStandaloneEditorInput("));
        assert!(STANDALONE_JS.contains("this.editorPreviewInputEnabled = false;"));
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
    fn standalone_export_embeds_player_wasm_runtime() {
        let source = r#"
	 title = Wasm Export

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

        assert_official_export_uses_bevy_launcher(&html);
        assert!(html.contains("window.PuzzleStandaloneEmbeddedWasm"));
        assert!(html.contains("PuzzleStudio standalone player failed:"));
        assert!(!html.contains("defaultAgainMs"));
        assert!(html.contains("\\\"runtimeLoadedDocument\\\""));
        assert!(html.contains("puzzle_wasm_player_bg.wasm"));
        assert!(!html.contains("puzzle_wasm_game_bg.wasm"));
        assert!(!html.contains("WasmStandaloneSession"));
        assert!(!html.contains("PuzzleAssets.files"));
        assert!(!html.contains("PuzzleStandaloneRuntime"));
        assert!(!html.contains("PuzzleRenderer"));
        assert!(!html.contains("Puzzle3Component"));
        assert!(!html.contains("visual_tween_core"));
        assert!(!html.contains("snapshot()"));
        assert!(!html.contains("dispatch(action_json)"));
        assert!(!html.contains("set_current_state"));
        assert!(!html.contains("setCurrentState"));
        assert!(!html.contains("WasmPuzzle3Runtime"));
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
        assert!(!html.contains("window.PuzzleExport = JSON.parse("));
        assert!(!html.contains("window.PuzzleExportJson = "));
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        let _: puzzle_runtime_contract::StandaloneRuntimeExport<puzzle_lang::LoadedDocument> =
            serde_json::from_value(runtime_export.clone())
                .expect("HTML runtime export must satisfy the standalone runtime schema");
        assert!(runtime_export["runtimeLoadedDocument"].is_object());
        assert_eq!(
            runtime_export["version"],
            json!(puzzle_runtime_contract::STANDALONE_PLAYER_EXPORT_VERSION)
        );
        let runtime_game =
            &runtime_export["runtimeLoadedDocument"]["models"][0]["Puzzle2d"]["game"];
        assert_eq!(runtime_game["levels"][0]["program"], json!(["Main"]));
        assert_eq!(runtime_game["program_catalog"]["programs"], json!([]));
        assert!(runtime_game.get("warnings").is_none());
        assert!(runtime_game.get("rule_debug_info").is_none());
        assert!(runtime_game.get("solver_strategy").is_none());
        assert!(runtime_export.get("compiledPlay").is_none());
        assert!(runtime_export.get("engine").is_none());
        assert!(runtime_export.get("source").is_none());
        assert_eq!(runtime_export["visualImages"]["assets"], json!([]));
        assert!(PUZZLE_PLAYER_WASM_JS.contains("startStandalonePlayer"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("WasmStandaloneSession"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("dispatch(action_json)"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("WasmPuzzle3Runtime"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("fromFixture"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("compile_preview"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("solve_state"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("solver"));
        assert!(!PUZZLE_PLAYER_WASM_JS.contains("puzzle_solver"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"/api/debug/input/"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"apply_traced_input"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"solve_state"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"puzzle_solver"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"SearchBudget"));
        assert!(!bytes_contain(PUZZLE_PLAYER_WASM_BG, b"best_first"));
    }

    #[test]
    fn runtime_export_size_does_not_multiply_common_program_by_level_count() {
        let rules = (0..256)
            .map(|_| "[ A ] -> [ A ]")
            .collect::<Vec<_>>()
            .join("\n");
        let levels = (0..64)
            .map(|index| format!("level \"level-{index}\" {{\nA\n}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!(
            r#"
title = Shared Program Size Gate
puzzle board {{
  layers {{ item = A }}
  rules {{
{rules}
  }}
}}
levels default of board {{
  legend A = A
{levels}
}}
"#
        );
        let document = puzzle_lang::parse_game_for_path(&source, "size_gate.puzzle").unwrap();
        let export = runtime_export_json(&StandaloneRuntimeExport::new(
            document,
            EncodedVisualImageBundle::default(),
            StandaloneProgressStorage {
                key: "size-gate".to_string(),
                save_version: puzzle_play::PROGRESS_SAVE_VERSION,
            },
        ))
        .unwrap();
        let value: Value = serde_json::from_str(&export).unwrap();
        let game = &value["runtimeLoadedDocument"]["models"][0]["Puzzle2d"]["game"];

        assert_eq!(game["levels"].as_array().unwrap().len(), 64);
        assert!(
            game["levels"]
                .as_array()
                .unwrap()
                .iter()
                .all(|level| level["program"] == json!(["Main"]))
        );
        assert!(
            game["program_catalog"]["programs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            export.len() < 2_000_000,
            "runtime export was {} bytes",
            export.len()
        );
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

        assert!(runtime_export["runtimeLoadedDocument"].is_object());
        assert!(
            html.contains(r#"\"kind\":\"input\",\"name\":\"continue_game\""#),
            "runtime bundle should encode the authored input command as a named payload"
        );
    }

    #[test]
    fn standalone_export_can_embed_browser_supplied_player_runtime_assets() {
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
        assert!(html.contains("startStandalonePlayer"));
        assert!(html.contains(r#"<canvas id="puzzle-bevy""#));
        assert!(!html.contains("window.PuzzleBoot"));
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
        assert!(!html.contains("PuzzleStudioCommand"));
        assert!(!html.contains("PuzzleStudioInput"));
        assert!(html.contains("PuzzleStudioPreviewState"));
        assert!(html.contains("PuzzleStudioSetPreviewDebugMode"));
        assert!(html.contains("PuzzleStudioPreviewDebugTrace"));
        assert!(html.contains("standaloneRuntime.applyDebugInputName(input)"));
        assert!(
            html.contains("this.editorPreviewDebugAvailable = bootData.editorPreview === true;")
        );
        assert!(html.contains("if (!this.sessionRuntime || !this.editorPreviewDebugAvailable)"));
        assert!(html.contains("apply_debug_input_name"));
        assert!(html.contains("PuzzleRuntimeWasmLoader"));
        assert!(html.contains("set_current_state("));
        assert!(APP_JS.contains("await standaloneRuntime.setCurrentState(event.data.state, {"));
        assert!(!html.contains("broadcastPuzzle3Key"));
        assert!(!html.contains("PuzzleStudioSolve"));
        assert!(!html.contains("loadWasmSolver"));
        let boot = embedded_puzzle_boot_json(&html);
        assert_eq!(boot["editorPreview"], json!(true));
        assert!(
            boot["source"]
                .as_str()
                .is_some_and(|value| value.contains("Editor Preview Export"))
        );
        assert_eq!(
            boot["puzzlePath"],
            json!("games/editor_preview/game.puzzle")
        );
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        let _: puzzle_runtime_contract::StandaloneRuntimeExport<puzzle_lang::LoadedDocument> =
            serde_json::from_value(runtime_export.clone())
                .expect("editor preview runtime export must satisfy the standalone runtime schema");
        assert!(runtime_export["runtimeLoadedDocument"].is_object());
        assert!(runtime_export.get("title").is_none());
        assert!(runtime_export.get("engine").is_none());
        assert!(runtime_export.get("source").is_none());
        let editor_preview_export = embedded_editor_preview_export_json(&html);
        assert_eq!(
            editor_preview_export["title"],
            json!("Editor Preview Export")
        );
        assert!(editor_preview_export["engine"].is_object());
        assert!(
            editor_preview_export["source"]
                .as_str()
                .is_some_and(|value| value.contains("Editor Preview Export"))
        );
        assert!(editor_preview_export.get("runtimeLoadedDocument").is_none());
    }

    #[test]
    fn wasm_editor_preview_loader_requests_parent_runtime_assets() {
        let source = include_str!("lib_export.rs");
        assert!(source.contains("PuzzleStudioRuntimeAssetRequest"));
        assert!(source.contains("PuzzleStudioRuntimeAssetResponse"));
        assert!(source.contains("puzzle_wasm_game.js"));
        assert!(source.contains("puzzle_wasm_game_bg.wasm.base64"));
        assert!(
            source.contains("Editor preview requires puzzle_wasm_game assets from its editor host")
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
        let document =
            puzzle_lang::parse_game_for_path(source, "games/export_test/game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let state = ServerState::new(
            document,
            loaded,
            source.to_string(),
            "games/export_test/game.puzzle".to_string(),
            EncodedVisualImageBundle::default(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_editor_preview_data(&mut data, &state);

        let export: serde_json::Value =
            serde_json::from_str(&data).expect("export data should be JSON");
        assert!(export.get("scenes").is_none());
        assert!(export.get("screens").is_none());
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
        let document =
            puzzle_lang::parse_game_for_path(source, "games/progress_export/game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let state = ServerState::new(
            document,
            loaded,
            source.to_string(),
            "games/progress_export/game.puzzle".to_string(),
            EncodedVisualImageBundle::default(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_editor_preview_data(&mut data, &state);
        let data_value: Value = serde_json::from_str(&data).unwrap();

        assert!(data_value.get("saveKey").is_none());
        assert!(data_value.get("progressSaveVersion").is_none());
        assert!(data.contains(r#""variables":[{"id":0,"name":"bonus"}]"#));
        assert!(data.contains(r#""persistentVars":[0]"#));
        assert!(
            data_value["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|input| input.get("triggers").is_some()
                    && input.get("key").is_none()
                    && input.get("arrow").is_none()
                    && input.get("keys").is_none())
        );
        assert!(STANDALONE_JS.contains("WasmStandaloneSession"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.dispatch("));
        assert!(STANDALONE_JS.contains("snapshot()"));
        assert!(STANDALONE_JS.contains("restoreSessionProgressSave()"));
        assert!(STANDALONE_JS.contains("writeSessionProgressSave()"));
        assert!(STANDALONE_JS.contains("progress_storage_save_version"));
        assert!(STANDALONE_JS.contains("progress_storage_key"));
        assert!(STANDALONE_JS.contains("saved progress was kept and was not overwritten"));
        assert!(STANDALONE_JS.contains("next.has_progress_save = true;"));
        assert!(APP_JS.contains("animationEvents: event.data.animationEvents"));
        assert!(APP_JS.contains("standaloneRuntime.snapshot({ forceJs: true })"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.progress_save_request()"));
        assert!(
            STANDALONE_JS
                .contains("this.sessionRuntime.confirm_progress_save_written(request.requestId)")
        );
        assert!(STANDALONE_JS.contains("this.sessionRuntime.confirm_progress_save_cleared()"));
        assert!(!STANDALONE_JS.contains("this.sessionRuntime.mark_progress_save_written()"));
        assert!(!STANDALONE_JS.contains("this.sessionRuntime.clear_progress_save()"));
        assert!(!STANDALONE_JS.contains("this.sessionRuntime.apply_input_name("));
        assert!(STANDALONE_JS.contains("PuzzleStudioPreviewProgressSave"));
        assert!(STANDALONE_JS.contains("PuzzleStudioEditorPreviewProgressSaves"));
        assert!(STANDALONE_JS.contains("window.localStorage.setItem"));
        assert!(STANDALONE_JS.contains("window.localStorage.getItem"));
        assert!(STANDALONE_JS.contains("Progress save could not be read"));
        assert!(!STANDALONE_JS.contains("catch (_error)"));
        assert!(!STANDALONE_JS.contains("progressSaveData()"));
        assert!(!STANDALONE_JS.contains("restoreProgressSave()"));
        assert!(!STANDALONE_JS.contains("writeProgressSave()"));
        assert!(!STANDALONE_JS.contains("clearedLevels[index]"));
        assert!(!STANDALONE_JS.contains("currentSaveLevelName()"));
        assert!(!STANDALONE_JS.contains("persistentVarSaveData()"));
        assert!(!STANDALONE_JS.contains("starting from defaults"));
    }

    #[test]
    fn editor_preview_does_not_restore_or_write_player_progress() {
        assert!(STANDALONE_JS.contains(
            "sessionProgressEnabled() {\n      return this.data?.editorPreview !== true;\n    }"
        ));
        assert!(STANDALONE_JS.contains(
            "if (!this.sessionRuntime || !this.sessionProgressEnabled()) {\n        return;\n      }"
        ));
        assert!(STANDALONE_JS.contains(
            "if (!this.sessionRuntime || !this.sessionProgressEnabled()) {\n        return false;\n      }"
        ));
        assert!(
            STANDALONE_JS.contains("if (method === \"POST\" && this.writeSessionProgressSave()) {")
        );
    }

    #[test]
    fn standalone_progress_persistence_acknowledges_exact_runtime_request_ids() {
        let mut script = String::from(
            r#"
const calls = [];
let pending = null;
class FakeSession {
  static fromExport() { return new FakeSession(); }
  progress_storage_key() { return "typed"; }
  progress_storage_save_version() { return 2; }
  set_audio_feedback_wakeup() {}
  set_progress_persistence_enabled(enabled) { calls.push(["persistence", enabled]); }
  dispatch(actionJson) {
    calls.push(["dispatch", JSON.parse(actionJson)]);
    pending = { requestId: 7, saveJson: "SAVE-7" };
    return JSON.stringify({ has_progress_save: false });
  }
  snapshot() { return "{}"; }
  progress_save_request() { return JSON.stringify(pending); }
  confirm_progress_save_written(id) {
    calls.push(["written", id]);
    if (!pending || pending.requestId !== id) throw new Error("stale acknowledgement");
    pending = null;
  }
  confirm_progress_save_cleared() { calls.push(["cleared"]); }
}
global.CustomEvent = class CustomEvent { constructor(type) { this.type = type; } };
global.window = {
  PuzzleRuntimeExportJson: "EXPORT",
  PuzzleRuntimeWasmLoader: {
    async load() { return { WasmStandaloneSession: FakeSession }; },
  },
  localStorage: {
    getItem() { return null; },
    setItem(key, value) { calls.push(["store", key, value]); },
    removeItem(key) { calls.push(["remove", key]); },
  },
  dispatchEvent(event) { calls.push(["event", event.type]); },
};
window.parent = window;
"#,
        );
        script.push_str(STANDALONE_JS);
        script.push_str(
            r#"
(async () => {
  const runtime = new window.PuzzleStandaloneRuntime({
    editorPreview: false,
    engineVersion: "test",
  }, "EXPORT");
  await runtime.ensureInitialized();
  const response = runtime.sessionRequestJson("POST", "/api/action", {
    body: JSON.stringify({ kind: "input", name: "right" }),
  });
  runtime.clearSessionProgressSave();
  process.stdout.write(JSON.stringify({ calls, response, pending }));
})().catch((error) => {
  console.error(error?.stack || error?.message || String(error));
  process.exitCode = 1;
});
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the progress persistence protocol test");
        assert!(
            output.status.success(),
            "progress persistence protocol evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["calls"][0], json!(["persistence", true]));
        assert_eq!(result["calls"][1][0], "dispatch");
        assert_eq!(
            result["calls"][1][1],
            json!({ "kind": "input", "name": "right" })
        );
        assert_eq!(result["calls"][2][0], "store");
        assert_eq!(result["calls"][2][2], "SAVE-7");
        assert_eq!(result["calls"][3], json!(["written", 7]));
        assert_eq!(result["calls"][4][0], "remove");
        assert_eq!(result["calls"][5], json!(["cleared"]));
        assert_eq!(result["response"]["has_progress_save"], true);
        assert!(result["pending"].is_null());
    }

    #[test]
    fn standalone_progress_restore_reports_storage_access_failure() {
        let mut script = String::from(
            r#"
class FakeSession {
  static fromExport() { return new FakeSession(); }
  progress_storage_key() { return "typed"; }
  progress_storage_save_version() { return 2; }
  set_audio_feedback_wakeup() {}
  set_progress_persistence_enabled() {}
  snapshot() { return "{}"; }
}
global.CustomEvent = class CustomEvent { constructor(type) { this.type = type; } };
global.window = {
  PuzzleRuntimeExportJson: "EXPORT",
  PuzzleRuntimeWasmLoader: {
    async load() { return { WasmStandaloneSession: FakeSession }; },
  },
  dispatchEvent() {},
};
Object.defineProperty(window, "localStorage", {
  get() { throw new Error("storage access denied"); },
});
window.parent = window;
"#,
        );
        script.push_str(STANDALONE_JS);
        script.push_str(
            r#"
(async () => {
  const runtime = new window.PuzzleStandaloneRuntime({
    editorPreview: false,
    engineVersion: "test",
  }, "EXPORT");
  try {
    await runtime.ensureInitialized();
    throw new Error("storage denial must reject initialization");
  } catch (error) {
    process.stdout.write(String(error?.message || error));
  }
})().catch((error) => {
  console.error(error?.stack || error?.message || String(error));
  process.exitCode = 1;
});
"#,
        );
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .expect("Node.js is required for the progress storage failure test");
        assert!(
            output.status.success(),
            "progress storage failure evaluation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let message = String::from_utf8(output.stdout).unwrap();
        assert!(
            message.contains("Progress save could not be read"),
            "{message}"
        );
        assert!(message.contains("storage access denied"), "{message}");
        assert!(
            message.contains("saved progress was not modified"),
            "{message}"
        );
    }

    #[test]
    fn dynamic_server_uses_the_shared_typed_runtime_snapshot() {
        let server_source = include_str!("lib_server.rs");
        let runtime_source = include_str!("lib_solver_runtime.rs");
        let export_source = include_str!("lib_json_export.rs");
        assert!(server_source.contains("state.runtime.dispatch_development_typed(action)"));
        assert!(
            runtime_source
                .contains("live_server_snapshot_json(self.runtime.development_snapshot())")
        );
        assert!(!server_source.contains("push_scene_object_body("));
        assert!(!server_source.contains("push_cells(out, loaded, state);"));
        assert!(!server_source.contains("scenePuzzleState"));
        assert!(!server_source.contains("scenePuzzles"));
        assert!(!server_source.contains("sceneState"));
        assert!(!export_source.contains("push_rule_effects"));
        assert!(!export_source.contains("write_scene_effect_json"));
    }

    #[test]
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
        let mut bridge = RuntimeSession::from_source(
            source,
            "crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle",
        )
        .unwrap();

        let initial = bridge.dispatch(SessionAction::Snapshot).unwrap();
        let initial: serde_json::Value = serde_json::from_str(&initial).unwrap();
        assert_eq!(initial["surface"]["focus"], "sokoban");
        assert!(initial.get("title").is_none());
        let initial = initial.as_object().unwrap();
        assert!(initial.contains_key("surface"));
        assert!(!initial.contains_key("visibleScenes"));
        assert!(!initial.contains_key("sceneLayers"));
        assert!(!initial.contains_key("currentScene"));
        assert!(!initial.contains_key("scene"));
        assert!(!initial.contains_key("sceneState"));
        assert!(!initial.contains_key("scenePuzzles"));
        assert!(!initial.contains_key("scenePuzzleState"));
        assert!(initial.contains_key("viewportSources"));
        assert!(!initial.contains_key("visibleScreens"));
        assert!(!initial.contains_key("screenState"));
        assert!(!initial.contains_key("screenPuzzles"));

        let initialized: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Initialize).unwrap()).unwrap();
        assert_eq!(initialized["surface"]["focus"], "sokoban");
        assert_eq!(initialized["levelIndex"], 0);

        let save: serde_json::Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(
            save["currentLevel"],
            puzzle_lang::LevelId::new("sokoban", "microban_01").record_key()
        );
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
        let mut bridge = RuntimeSession::from_source(source, "debug_trace.puzzle").unwrap();

        let body = bridge.apply_debug_input_name_json("right").unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();

        assert_eq!(body["snapshot"]["surface"]["focus"], "main");
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
    fn standalone_session_snapshot_preserves_2d_render_settings() {
        let source = include_str!("../tests/fixtures/locked.puzzle");
        let mut bridge = RuntimeSession::from_source(source, "games/TPGJ6/locked.puzzle").unwrap();

        let playing: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Initialize).unwrap()).unwrap();

        assert_eq!(playing["surface"]["focus"], "main");
        assert_eq!(
            first_viewport_state(&playing)["settings"]["render"],
            serde_json::json!({})
        );
        assert_eq!(
            first_viewport_state(&playing)["settings"]["inputBuffer"]["minWaitMs"],
            50
        );
        assert_eq!(
            first_viewport_state(&playing)["settings"]["animation"]["tween"]["intervalMs"],
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

        let mut bridge = RuntimeSession::from_source(source, "games/TPGJ6/locked.puzzle").unwrap();
        bridge
            .set_current_state_json(&state_json, level_index, true)
            .unwrap();
        let mut after_first: serde_json::Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "left".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        while after_first["busy"] == true {
            after_first =
                serde_json::from_str(&bridge.dispatch(SessionAction::Resume).unwrap()).unwrap();
        }
        assert!(cell_has_object(
            &first_viewport_state(&after_first)["cells"][69],
            "Player"
        ));

        let mut snapshot: serde_json::Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "left".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        while snapshot["busy"] == true {
            snapshot =
                serde_json::from_str(&bridge.dispatch(SessionAction::Resume).unwrap()).unwrap();
        }
        assert_eq!(snapshot["levelIndex"], level_index);
        assert_eq!(snapshot["surface"]["focus"], "playing");
        assert!(cell_has_object(
            &first_viewport_state(&snapshot)["cells"][68],
            "Player"
        ));
    }

    #[test]
    fn standalone_snapshot_reports_runtime_owned_model_input_acceptance() {
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
choice "Select level" -> goto level_select
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
        let mut bridge = RuntimeSession::from_source(source, "contract.puzzle").unwrap();

        let playing: Value =
            serde_json::from_str(&bridge.dispatch(SessionAction::Initialize).unwrap()).unwrap();
        assert_eq!(playing["surface"]["focus"], json!("default"));
        assert_eq!(playing["acceptsModelInput"], json!(true));

        let after_input: Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Input {
                    name: "down".to_string(),
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after_input["surface"]["focus"], json!("default"));
        assert_eq!(after_input["acceptsModelInput"], json!(true));
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
            RuntimeSession::from_source(source, "standalone_default_move_wait.puzzle").unwrap();

        bridge.dispatch(SessionAction::Initialize).unwrap();

        let moved = bridge
            .dispatch(SessionAction::Input {
                name: "right".to_string(),
            })
            .unwrap();
        let moved: serde_json::Value = serde_json::from_str(&moved).unwrap();
        assert_eq!(
            moved["inputBuffer"],
            json!({
                "queueDuringWait": true,
                "fastForwardWait": true,
                "minWaitMs": 75
            })
        );
        assert!(cell_has_object(
            &first_viewport_state(&moved)["cells"][1],
            "Player"
        ));
        assert!(!cell_has_object(
            &first_viewport_state(&moved)["cells"][1],
            "Done"
        ));
        assert_eq!(moved["busy"], true);
        assert_eq!(
            moved["presentationEvents"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["kind"],
            json!("wait")
        );
        let resumed: serde_json::Value = serde_json::from_str(
            &bridge
                .dispatch(SessionAction::Resume)
                .expect("the wait boundary should resume the same input turn"),
        )
        .unwrap();
        assert!(cell_has_object(
            &first_viewport_state(&resumed)["cells"][1],
            "Done"
        ));
        assert_eq!(resumed["busy"], false);
    }

    #[test]
    fn standalone_session_bridge_restores_progress_save() {
        let source =
            include_str!("../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle");
        let mut bridge = RuntimeSession::from_source(
            source,
            "crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle",
        )
        .unwrap();
        let level_key = puzzle_lang::LevelId::new("sokoban", "microban_01").record_key();
        bridge
            .restore_progress_save_json(
                &json!({
                    "version": 2,
                    "levels": [{"id": level_key, "cleared": true}],
                    "currentLevel": level_key,
                    "persistentVars": [],
                })
                .to_string(),
            )
            .unwrap();

        let snapshot = bridge.dispatch(SessionAction::Snapshot).unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 0);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(snapshot["levels"][&level_key]["progress"]["cleared"], true);
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
        let document = puzzle_lang::parse_game_for_path(source, "games/focus/game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let state = ServerState::new(
            document,
            loaded,
            source.to_string(),
            "games/focus/game.puzzle".to_string(),
            EncodedVisualImageBundle::default(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut data = String::new();
        push_editor_preview_data(&mut data, &state);

        assert!(data.contains(r#""viewportFocus":"player""#));
        assert!(data.contains(r#""viewportFocusObjects":[2,3]"#));
        assert!(RENDERER_JS.contains("focusObjects.has(Number(layer.objectId))"));
    }

    #[test]
    fn standalone_export_does_not_publish_raw_authored_theme() {
        let source = r##"
title = Theme Startup
theme {
  preset = "noir"
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

        assert!(html.contains("<body>"));
        assert!(!html.contains("theme-noir"));
        assert!(!html.contains(r##""variables":{"background":"#123456"}"##));

        let document =
            puzzle_lang::parse_game_for_path(source, "games/theme_startup/game.puzzle").unwrap();
        let loaded = loaded_document_scene_host_loaded_game(&document).unwrap();
        let state = ServerState::new(
            document,
            loaded,
            source.to_string(),
            "games/theme_startup/game.puzzle".to_string(),
            EncodedVisualImageBundle::default(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );
        let mut preview_data = String::new();
        push_editor_preview_data(&mut preview_data, &state);
        let preview: Value = serde_json::from_str(&preview_data).unwrap();
        let _: puzzle_runtime_contract::RuntimeTheme =
            serde_json::from_value(preview["theme"].clone())
                .expect("editor preview theme must satisfy the typed runtime contract");
        assert!(preview["theme"].get("name").is_none());
        assert!(preview["theme"].get("variables").is_none());
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

        assert_official_export_uses_bevy_launcher(&html);
        assert!(!html.contains("runtimeContractVersion"));
        assert!(!html.contains("runtimeContract"));
        assert!(html.contains("puzzle_wasm_player_bg.wasm"));
        assert!(!html.contains("WasmStandaloneSession"));
        assert!(!html.contains("replaceSnapshot"));
        assert!(!html.contains("delete this.data.runtimeLoadedDocument;"));
        assert!(!html.contains("runtimeLoadedGame"));
        assert!(!html.contains("onLifecycleEffects(effects)"));
        assert!(!html.contains("function sendPuzzle3LifecycleEffects("));
        assert!(!html.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!html.contains("puzzle_wasm_game_bg.wasm"));
        assert!(!html.contains("\\npuzzle3 microban3d"));

        let preview_html = export_editor_preview_html_from_source(
            source,
            "games/spec_3d.puzzle3",
            "body { --accent: #123456; }",
            "",
        )
        .expect("editor preview should embed preview runtime assets");

        assert!(preview_html.contains("window.Puzzle3DFrameFixture"));
        assert!(!preview_html.contains("window.Puzzle3DFixture"));
        assert!(!preview_html.contains("WasmPuzzle3Runtime"));
        assert!(preview_html.contains("WasmStandaloneSession"));
        assert!(preview_html.contains("window.Puzzle3Component"));
        assert!(!preview_html.contains("onLifecycleEffects(effects)"));
        assert!(!preview_html.contains("function sendPuzzle3LifecycleEffects("));
        assert!(!preview_html.contains("Unsupported Puzzle3 lifecycle effect"));
        assert!(!preview_html.contains("Puzzle3DTestRuntime"));
        assert!(html.contains("Microban 3D"));
        assert!(!html.contains("--accent: #123456"));
        let bridge = RuntimeSession::from_source(source, "games/spec_3d.puzzle3")
            .expect("single puzzle3 document should have a scene host game runtime");
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["surface"]["focus"], json!("sokoban"));
    }

    #[test]
    fn standalone_mixed_dimension_export_uses_bevy_without_puzzle3_fixture_projection() {
        let source = r#"
title = "Mixed Export"

puzzle flat {
  layers {
    actor = FlatPlayer
  }
  rules {
  }
}

levels flat_levels of flat {
  legend {
    P = FlatPlayer
  }
  level "flat" {
    P
  }
}

puzzle cube {
  dimension = 3
  layers {
    actor = CubePlayer
  }
  rules {
  }
}

levels cube_levels of cube {
  legend {
    P = CubePlayer
  }
  level "cube" {
    P
  }
}

scene playing {
  layout {
    row {
      puzzle flat_board = flat
      puzzle cube_board = cube
    }
  }
}
"#;

        let html = export_html_from_source(source, "games/mixed_export.puzzle", "", "")
            .expect("mixed standalone export should be owned by the Bevy launcher");

        assert_official_export_uses_bevy_launcher(&html);
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        let models = runtime_export["runtimeLoadedDocument"]["models"]
            .as_array()
            .expect("mixed runtime export should retain both typed models");
        assert_eq!(models.len(), 2);

        let export_json = serde_json::to_string(&runtime_export).unwrap();
        let decoded = puzzle_player_bootstrap::decode_standalone_player_export(&export_json)
            .expect("the exported payload must construct the real standalone player session");
        let (runtime, _, _) = decoded.into_parts();
        let snapshot = runtime.snapshot();
        assert!(matches!(
            snapshot
                .viewport_sources
                .get(&puzzle_runtime_contract::RuntimeViewportSourceId {
                    component: "playing".to_string(),
                    source: "flat_board".to_string(),
                }),
            Some(puzzle_session_contract::RuntimeRendererState::TwoD(_))
        ));
        assert!(matches!(
            snapshot
                .viewport_sources
                .get(&puzzle_runtime_contract::RuntimeViewportSourceId {
                    component: "playing".to_string(),
                    source: "cube_board".to_string(),
                }),
            Some(puzzle_session_contract::RuntimeRendererState::ThreeD(_))
        ));
    }

    #[test]
    fn puzzle3_source_free_export_keeps_runtime_semantics_out_of_the_visual_fixture() {
        let source = r#"title = "Local Frame"

puzzle cube {
  dimension = 3
  layers {
    actor = Player
  }

  rules local_frame 1 1 {
    [ Player ] -> [ Player ]
  }
}

scene playing {
  layout {
    puzzle board = cube
  }
}

levels default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"#;
        let html = export_html_from_source(source, "games/local_frame.puzzle3", "", "")
            .expect("local_frame should compile into the standalone session model");

        assert!(!html.contains("runtimeContract"));
        assert!(!html.contains("\\npuzzle3 cube"));
    }

    #[test]
    fn puzzle3_three_camera_frame_preserves_vertical_yaw_and_roll() {
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("function cameraRenderFrame(cameraSettings)"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("cameraSettings.rollDegrees ?? 0"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("camera.up.set(cameraFrame.up.x, cameraFrame.up.y, cameraFrame.up.z);")
        );
        assert!(
            !PUZZLE3_THREE_RENDERER_JS
                .contains("camera.up.set(0, 0, -Math.sign(Math.sin(pitch)) || -1);")
        );
    }

    #[test]
    fn puzzle3_three_camera_frame_preserves_explicit_zero_pitch() {
        let value = evaluate_puzzle3_render_math(
            r#"
const zero = window.Puzzle3ThreeRenderer.cameraRenderFrame({ pitchDegrees: 0 });
const omitted = window.Puzzle3ThreeRenderer.cameraRenderFrame({});
return { zero, omitted };
"#,
        );

        assert_eq!(value["zero"]["forward"], json!({ "x": 0, "y": 0, "z": -1 }));
        assert_eq!(value["zero"]["up"], json!({ "x": 0, "y": 1, "z": 0 }));
        assert_ne!(value["zero"]["forward"], value["omitted"]["forward"]);
    }

    #[test]
    fn puzzle3_zoom_clamps_non_positive_values_to_minimum() {
        let value = evaluate_puzzle3_render_math(
            r#"
return [undefined, 0, -2, 0.05, 2].map((zoom) => (
  window.Puzzle3VisualCore.normalizeZoom(zoom)
));
"#,
        );

        assert_eq!(value, json!([1, 0.1, 0.1, 0.1, 2]));
        assert!(
            PUZZLE3_COMPONENT_JS
                .contains("const cameraZoom = Puzzle3VisualCore.normalizeZoom(camera?.zoom);")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains(
                "const cameraValue = Puzzle3VisualCore.normalizeZoom(cameraSettings.zoom);"
            )
        );
    }

    #[test]
    fn puzzle3_three_tween_math_covers_xyz_and_shortest_rotation_path() {
        let value = evaluate_puzzle3_render_math(
            r#"
const offset = window.Puzzle3ThreeRenderer.animationOffset3(
  { animationProgress: 0.5 },
  { from: { x: 1, y: 2, z: 3 }, to: { x: 5, y: 6, z: 7 } },
);
const tween = {
  from: { transforms: [{ kind: "rotate", space: "local", axis: [0, 0, 1], degrees: 350 }] },
  to: { transforms: [{ kind: "rotate", space: "local", axis: [0, 0, 1], degrees: 10 }] },
};
const state = window.PuzzleVisualTweenCore.interpolate(tween, 0.5);
const halfway = window.Puzzle3VisualCore.evaluateSpatialVisualAffine(
  state.transforms.map((transform) => ({ ...transform, kind: "rotate3" })),
);
const point = window.Puzzle3VisualCore.transformSpatialPoint({ x: 1, y: 0, z: 0 }, halfway);
return { offset, point };
"#,
        );

        assert_eq!(value["offset"], json!({"x": -2, "y": -2, "z": 2}));
        assert!((value["point"]["x"].as_f64().unwrap() - 1.0).abs() < 0.000000001);
        assert!(value["point"]["y"].as_f64().unwrap().abs() < 0.000000001);
        assert!(value["point"]["z"].as_f64().unwrap().abs() < 0.000000001);
    }

    #[test]
    fn puzzle3_three_tween_rejects_incompatible_rotation_axes() {
        let value = evaluate_puzzle3_render_math(
            r#"
try {
  window.PuzzleVisualTweenCore.interpolate(
    { from: { transforms: [{ kind: "rotate", space: "local", axis: [0, 0, 1], degrees: 0 }] },
      to: { transforms: [{ kind: "rotate", space: "local", axis: [0, 1, 0], degrees: 90 }] } },
    0.5,
  );
  return { message: "" };
} catch (error) {
  return { message: String(error?.message || error) };
}
"#,
        );

        assert_eq!(value["message"], "Visual tween rotation axis 0 changes.");
    }

    #[test]
    fn visual_tween_core_exposes_extensible_scale_translation_and_opacity_channels() {
        let value = evaluate_puzzle3_render_math(
            r#"
return window.PuzzleVisualTweenCore.interpolate({
  from: {
    transforms: [
      { kind: "translate", space: "world", value: [0, 2, 4] },
      { kind: "scale", space: "local", value: [1, 1, 1] },
    ],
    opacity: 0,
  },
  to: {
    transforms: [
      { kind: "translate", space: "world", value: [2, 4, 6] },
      { kind: "scale", space: "local", value: [3, 2, 1] },
    ],
    opacity: 1,
  },
}, 0.5);
"#,
        );

        assert_eq!(value["transforms"][0]["value"], json!([1, 3, 5]));
        assert_eq!(value["transforms"][1]["value"], json!([2, 1.5, 1]));
        assert_eq!(value["opacity"], json!(0.5));
    }

    #[test]
    fn puzzle3_three_tween_requires_addressed_batches_and_animates_culling_bounds() {
        assert!(
            PUZZLE3_COMPONENT_JS.contains("animationBatchId must identify its Tween event batch")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("Puzzle3 Tween animation events require a positive animationBatchId.")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("key: events.length ? `batch:${batchId}` : \"idle\"")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("positions.push(animation.from);"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("PuzzleVisualTweenCore.interpolate("));
    }

    #[test]
    fn puzzle3_three_tween_reuses_frame_data_and_owns_one_animation_loop() {
        let value = evaluate_puzzle3_render_math(
            r##"
const snapshot = {
  size: { width: 2, depth: 1, height: 1 },
  render: {
    camera: {},
    animation: { tween: { enabled: true, intervalMs: 100 } },
    viewport: null,
  },
  objects: {
    A: { id: 1, name: "A", visual: "shared" },
    B: { id: 2, name: "B", visual: "shared" },
  },
  visuals: {
    shared: {
      palette: { "0": "#fff" },
      spatialOps: [],
      frames: [{ layers: [["0"]] }],
    },
  },
  cells: [
    { position: { x: 0, y: 0, z: 0 }, objects: [{ id: 1 }] },
    { position: { x: 1, y: 0, z: 0 }, objects: [{ id: 2 }] },
  ],
  animationEvents: [
    { kind: "move", name: "tween", occurrenceId: 1, objectId: 1, from: { x: -1, y: 0, z: 0 }, to: { x: 0, y: 0, z: 0 } },
    { kind: "move", name: "tween", occurrenceId: 2, objectId: 2, from: { x: 0, y: 0, z: 0 }, to: { x: 1, y: 0, z: 0 } },
  ],
  order: { direction_priority: ["down", "right", "front"], priorities: [{}] },
};
const frame = window.Puzzle3ThreeRenderer.buildPuzzleStudioThreeFrame(snapshot, { animationProgress: 0 });
const updated = window.Puzzle3ThreeRenderer.updatePuzzleStudioThreeFrame(frame, snapshot, {}, 0.5);
return {
  sameFrame: frame === updated,
  animationProgress: updated.animationProgress,
  cachedVisuals: updated.visualCache.size,
  indexedEvents: updated.animationEventIndex.size,
};
"##,
        );

        assert_eq!(
            value,
            json!({
                "sameFrame": true,
                "animationProgress": 0.5,
                "cachedVisuals": 1,
                "indexedEvents": 2
            })
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("const staticCellCache = frame.staticCellVisibilityCache || new Map();")
        );
        assert!(
            PUZZLE3_THREE_RENDERER_JS
                .contains("const boundsCache = frame.cellRenderBoundsCache || new Map();")
        );
        assert!(!PUZZLE3_COMPONENT_JS.contains("if (result?.animating)"));
        assert!(
            PUZZLE3_THREE_RENDERER_JS.contains(
                "frame.viewport?.follow === \"smooth\" && frame.viewportAnimating === true"
            )
        );
    }

    #[test]
    fn puzzle3_three_rotation_tween_keeps_adjacent_voxels_as_one_rigid_mesh() {
        let value = evaluate_puzzle3_render_math(
            r##"
const snapshot = {
  size: { width: 1, depth: 1, height: 1 },
  render: {
    camera: {},
    animation: { tween: { enabled: true, intervalMs: 100 } },
    viewport: null,
  },
  objects: { A: { id: 1, name: "A", visual: "bar" } },
  visuals: {
    bar: {
      palette: { "0": "#fff" },
      spatialOps: [{ kind: "rotate3", space: "local", axis: [0, 0, 1], degrees: 90 }],
      frames: [{ layers: [["00"]] }],
    },
  },
  cells: [{ position: { x: 0, y: 0, z: 0 }, objects: [{ id: 1 }] }],
  animationEvents: [{
    kind: "move",
    name: "tween",
    occurrenceId: 1,
    objectId: 1,
    from: { x: 0, y: 0, z: 0 },
    to: { x: 0, y: 0, z: 0 },
    visualTween: {
      from: { transforms: [{ kind: "rotate", space: "local", axis: [0, 0, 1], degrees: 0 }] },
      to: { transforms: [{ kind: "rotate", space: "local", axis: [0, 0, 1], degrees: 90 }] },
    },
  }],
  order: { direction_priority: ["down", "right", "front"], priorities: [{ objects: ["A"] }] },
};
const frame = window.Puzzle3ThreeRenderer.buildPuzzleStudioThreeFrame(
  snapshot,
  { animationProgress: 0.5 },
);
const visible = window.Puzzle3ThreeRenderer.frameVisibleVoxels(frame);
const faces = window.Puzzle3ThreeRenderer.mergedVoxelFaces(visible.voxels, visible.occupied);
return {
  voxels: visible.voxels.length,
  faces: faces.length,
  diagonalFace: faces.some((face) => face.corners.some((corner) => (
    Math.abs(corner.x) > 0.001 && Math.abs(corner.z) > 0.001
  ))),
};
"##,
        );

        assert_eq!(
            value,
            json!({
                "voxels": 2,
                "faces": 6,
                "diagonalFace": true,
            })
        );
    }

    #[test]
    fn puzzle3_export_uses_the_bevy_launcher_without_a_parallel_frame_fixture() {
        let source = r#"title = "Tiny"

puzzle cube {
  dimension = 3
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
    puzzle board = cube
  }
}

levels default of cube {
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

        assert_official_export_uses_bevy_launcher(&html);
        assert!(!html.contains("Puzzle3ComponentAutoBoot"));
        assert!(!html.contains("\"themeCss\""));
        assert!(!html.contains("case \"choice\""));
        let runtime_export = embedded_puzzle_runtime_export_json(&html);
        assert!(runtime_export["runtimeLoadedDocument"].is_object());
        assert!(runtime_export.get("engine").is_none());
        assert!(runtime_export.get("compiledPlay").is_none());
        assert!(!html.contains("\\npuzzle3 cube"));
        assert!(!html.contains("\"source\":\"title \\\\\\\"Tiny\\\\\\\"\\n"));
        assert!(!html.contains("window.Puzzle3DSource ="));
        assert!(!html.contains("window.Puzzle3DPath ="));
    }

    #[test]
    fn puzzle3_editor_preview_keeps_component_document_transparent() {
        let source = r##"title = "Themed 3D"
theme {
  preset = "clean"
  background_color = #123456
}

puzzle cube {
  dimension = 3
  layers {
    actor = Player
  }
  rules {
  }
}

scene playing {
  layout {
    puzzle board = cube
  }
}

levels default of cube {
  legend {
    P = Player
  }
  level "one" {
    P
  }
}
"##;
        let html =
            export_editor_preview_html_from_source(source, "games/themed_3d.puzzle3", "", "")
                .expect("editor preview should keep its component document");
        let boot = embedded_puzzle_boot_json(&html);
        let fixture = embedded_puzzle3_frame_fixture_json(&html);

        assert_eq!(boot.get("theme"), None);
        assert_eq!(fixture.get("theme"), None);
        assert!(html.contains("window.Puzzle3DFrameAssets = {"));
        assert!(!html.contains("Puzzle3ComponentAutoBoot"));
        assert!(html.contains("window.Puzzle3Component"));
        assert!(!html.contains("\"themeCss\""));
        assert!(!html.contains("theme-clean"));
        assert!(!html.contains("frame.style.backgroundColor"));
        assert!(!html.contains("<html lang=\"en\" style=\"background:transparent;\">"));
        assert!(
            !html.contains("<body class=\"is-component-embed\" style=\"background:transparent;\">")
        );
        assert!(PUZZLE3_COMPONENT_JS.contains("canvas.getContext(\"2d\", { alpha: true })"));
    }

    #[cfg(feature = "solver")]
    #[test]
    fn native_solver_route_uses_the_shared_typed_solver_result() {
        let source = r#"
title = shared_native_solver

puzzle default {
layers {
floor = Goal
actor = Player Box
}
keys {
d ArrowRight -> right
}
rules {
input right [ Player | Box | no actor ] -> [ | Player | Box ]
}
win_conditions {
all Goal on Box
}
}

levels tiny of default {
legend {
P = Player
B = Box
G = Goal
}
level "start" {
PBG
}
}
"#;
        let document = puzzle_lang::parse_game_for_path(source, "shared_native_solver.puzzle")
            .expect("compile native solver fixture");
        let loaded =
            loaded_document_scene_host_loaded_game(&document).expect("select native solver model");
        let mut state = ServerState::new(
            document,
            loaded,
            source.to_string(),
            "shared_native_solver.puzzle".to_string(),
            EncodedVisualImageBundle::default(),
            String::new(),
            String::new(),
            SolverConfig::default(),
        );

        let result: Value =
            serde_json::from_str(&state.solve_json().expect("solve native session"))
                .expect("native solver response is typed JSON");
        assert_eq!(result["model"], "2d");
        assert_eq!(result["result"], "solved");
        assert_eq!(result["depth"], 1);
        assert_eq!(result["steps"][0]["state"]["kind"], "2d");
        assert!(result["steps"][0].get("scene").is_none());

        state.solver.max_duration = Duration::ZERO;
        let limited: Value =
            serde_json::from_str(&state.solve_json().expect("bound native solver session"))
                .expect("bounded native solver response is typed JSON");
        assert_eq!(limited["result"], "budget_exceeded");
        assert!(limited["observations"].is_array());
    }

    #[cfg(feature = "solver")]
    #[test]
    fn html_play_contains_no_search_orchestration_owner() {
        let runtime = include_str!("lib_solver_runtime.rs");
        let bridge = include_str!("lib_runtime_bridge.rs");
        let manifest = include_str!("../Cargo.toml");
        assert!(!runtime.contains("best_first"));
        assert!(!runtime.contains("SearchBudget"));
        assert!(!bridge.contains("solve_request_json"));
        assert!(manifest.contains("puzzle-solver-runtime"));
        assert!(!manifest.contains("puzzle-solver ="));
    }

    #[test]
    fn screenshot_scene_override_is_not_a_player_contract() {
        let error = Config::from_args(["--scene".to_string(), "playing".to_string()])
            .expect_err("removed scene override must fail at CLI parsing")
            .to_string();
        assert_eq!(error, "unknown option: --scene");
    }

    #[test]
    fn screenshot_harness_waits_for_typed_ready_and_rejects_browser_failures() {
        let adapter = include_str!("lib_screenshot.rs");
        let harness = include_str!("../../../tools/standalone_player_browser_smoke.mjs");
        let shared_cdp = include_str!("../../../tools/editor_browser_smoke.mjs");

        assert!(adapter.contains("standalone_player_browser_smoke.mjs"));
        assert!(!adapter.contains("--screenshot="));
        assert!(!adapter.contains("metadata.len() > 0"));
        assert!(!adapter.contains("Stdio::null()"));
        assert!(!adapter.contains("remove_file(output_path)"));
        assert!(harness.contains(r#"status?.dataset.state === "ready""#));
        assert!(harness.contains("#puzzle-bevy-fatal"));
        assert!(harness.contains("page.pageErrors.length"));
        assert!(harness.contains(r#"page.send("Page.captureScreenshot""#));
        assert!(harness.contains("assertPngDimensions(png, width, height)"));
        assert!(harness.contains("requiredUnsignedInteger("));
        assert!(harness.contains("fs.renameSync(temporaryOutputPath, outputPath)"));
        assert!(harness.contains("enableGpu: true"));
        assert!(harness.contains("swiftShader: true"));
        assert!(shared_cdp.contains("export class Browser"));
        assert!(shared_cdp.contains("if (!this.options.enableGpu)"));
        assert!(shared_cdp.contains(r#""--enable-unsafe-swiftshader""#));
        assert!(shared_cdp.contains("if (isDirectInvocation)"));
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
#[cfg(feature = "solver")]
#[test]
fn solver_defaults_match_product_entry_points() {
    let defaults = SolverConfig::default();
    assert_eq!(defaults.max_nodes, 1_000_000);
    assert_eq!(defaults.max_duration, Duration::from_secs(5));
    let solve_start = APP_JS
        .find("async function solveStandaloneCurrentState(")
        .expect("standalone solve entry point");
    let solve_end = APP_JS[solve_start..]
        .find("\n}\n/* puzzle-host:optional:solver:end */")
        .map(|index| solve_start + index)
        .expect("standalone solve entry point end");
    let solve_source = &APP_JS[solve_start..solve_end];
    assert!(solve_source.contains("options.maxNodes ?? 5_000_000"));
    assert!(!solve_source.contains("options.maxNodes ?? 1000"));
}

#[cfg(feature = "solver")]
#[test]
fn solver_cli_can_override_the_wall_clock_limit() {
    let puzzle_path = format!("{}/../../games/spec_3d.puzzle3", env!("CARGO_MANIFEST_DIR"));
    let defaults = Config::from_args([puzzle_path.clone()]).unwrap();
    assert_eq!(defaults.solver.max_duration, Duration::from_secs(5));

    let help = Config::from_args(["--help".to_string()])
        .expect_err("help exits through the config diagnostic")
        .to_string();
    assert!(help.contains("[--solver-nodes 1000000]"));

    let bounded =
        Config::from_args([puzzle_path, "--solver-ms".to_string(), "250".to_string()]).unwrap();
    assert_eq!(bounded.solver.max_duration, Duration::from_millis(250));
}
