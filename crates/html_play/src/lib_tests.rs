#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use serde_json::Value;

    const SOURCE_2D: &str = r#"
const title = static_bevy_server
puzzle board {
  layers { item = A }
  rules { [ A ] -> [ A ] }
}
levels default of board {
  legend A = A
  level "first" {
    A
  }
}
"#;

    const SOURCE_3D: &str = r#"
const title = static_bevy_server_3d
puzzle cube {
  dimension = 3
  layers { item = A }
  rules { }
}
levels default of cube {
  legend { A = A }
  level "first" {
    A
  }
}
"#;

    fn player_html(source: &str, path: &str) -> String {
        let document =
            puzzle_lang::parse_game_for_path(source, path).expect("compile player fixture");
        export_bevy_document_html(&document, path, StandaloneRuntimeWasm::HostDefault)
            .expect("build Bevy player page")
    }

    fn without_runtime_payload(mut html: String) -> String {
        let prefix = "window.PuzzleRuntimeExportJson = \"";
        let start = html.find(prefix).expect("runtime payload start") + prefix.len();
        let end = html[start..]
            .find("\";\n")
            .map(|offset| start + offset)
            .expect("runtime payload end");
        html.replace_range(start..end, "<runtime-export>");
        html
    }

    #[test]
    fn all_documents_use_one_bevy_player_scaffold() {
        let html_2d = player_html(SOURCE_2D, "static_bevy_server.puzzle");
        let html_3d = player_html(SOURCE_3D, "static_bevy_server_3d.puzzle");

        assert_eq!(
            without_runtime_payload(html_2d.clone()),
            without_runtime_payload(html_3d),
            "renderer dimension belongs to the typed payload, not a parallel host"
        );
        assert!(html_2d.contains(r#"<canvas id="puzzle-bevy""#));
        assert!(html_2d.contains("startStandalonePlayer"));
        assert!(html_2d.contains("window.PuzzleStandaloneEmbeddedWasm"));
        assert!(html_2d.contains("Standalone player WASM is missing startStandalonePlayer."));
        for forbidden in [
            "WasmStandaloneSession",
            "PuzzleRenderer",
            "Puzzle3DFixture",
            "PuzzleStudioSetState",
            "PuzzleStudioKey",
            "PuzzleStudioRequestPreviewState",
            "/api/solve",
        ] {
            assert!(
                !html_2d.contains(forbidden),
                "standalone scaffold retained legacy runtime contract {forbidden}"
            );
        }
    }

    #[test]
    fn static_server_exposes_only_the_compiled_bevy_document() {
        let html = player_html(SOURCE_2D, "static_bevy_server.puzzle");
        for path in ["/", "/index.html"] {
            let response = route_static_html(
                &HttpRequest {
                    method: "GET".to_string(),
                    path: path.to_string(),
                },
                &html,
            );
            assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        }
        for (method, path) in [
            ("GET", "/app.js"),
            ("GET", "/renderer.js"),
            ("GET", "/standalone.js"),
            ("GET", "/api/state"),
            ("POST", "/api/action"),
            ("POST", "/api/render-moment"),
            ("POST", "/api/solve"),
        ] {
            let response = route_static_html(
                &HttpRequest {
                    method: method.to_string(),
                    path: path.to_string(),
                },
                &html,
            );
            assert!(
                response.starts_with("HTTP/1.1 404 Not Found\r\n"),
                "{method} {path} unexpectedly remained reachable"
            );
        }
    }

    #[test]
    fn editor_preview_uses_only_the_typed_bevy_command_bridge() {
        let document =
            puzzle_lang::parse_game_for_path(SOURCE_2D, "editor-preview.puzzle").unwrap();
        let build = export_editor_preview_build_from_document(
            &document,
            SOURCE_2D,
            "editor-preview.puzzle",
        )
        .expect("editor preview build");
        let build: Value = serde_json::from_str(&build).unwrap();
        let html = build["html"].as_str().unwrap();

        for required in [
            "startEditorPreview",
            "dispatchEditorPreviewCommand",
            "PuzzleStudioEditorPreviewCommand",
            "PuzzleStudioEditorPreviewObservation",
            "PuzzleStudioEditorPointer",
            "PuzzleStudioPreviewRuntimeError",
            "PuzzleStudioRuntimeAssetRequest",
        ] {
            assert!(
                html.contains(required),
                "editor bridge is missing typed contract {required}"
            );
        }
        for forbidden in [
            "PuzzleStudioSetState",
            "PuzzleStudioKey",
            "PuzzleStudioRequestPreviewState",
            "WasmStandaloneSession",
            "PuzzleRenderer",
            "Puzzle3DFixture",
            "window.PuzzleStandaloneEmbeddedWasm",
        ] {
            assert!(
                !html.contains(forbidden),
                "editor bridge retained legacy contract {forbidden}"
            );
        }
    }

    #[test]
    fn standalone_progress_identity_is_path_independent_and_content_addressed() {
        let first = puzzle_lang::parse_game_for_path(SOURCE_2D, "/checkout-a/game.puzzle").unwrap();
        let second =
            puzzle_lang::parse_game_for_path(SOURCE_2D, "/checkout-b/game.puzzle").unwrap();
        assert_eq!(
            standalone_progress_storage(&first),
            standalone_progress_storage(&second)
        );

        let changed_source = SOURCE_2D.replacen("\n    A\n", "\n    AA\n", 1);
        let changed =
            puzzle_lang::parse_game_for_path(&changed_source, "/checkout-c/game.puzzle").unwrap();
        assert_ne!(
            standalone_progress_storage(&first),
            standalone_progress_storage(&changed)
        );
    }

    #[test]
    fn removed_solver_cli_options_fail_at_the_cli_owner() {
        for option in ["--solver-depth", "--solver-nodes", "--solver-ms"] {
            let error = Config::from_args([
                "game.puzzle".to_string(),
                option.to_string(),
                "1".to_string(),
            ])
            .expect_err("removed solver option must be rejected")
            .to_string();
            assert_eq!(error, format!("unknown option: {option}"));
        }
        let manifest = include_str!("../Cargo.toml");
        assert!(!manifest.contains("puzzle-solver-runtime"));
        assert!(!manifest.contains("solver-session"));
    }

    #[test]
    fn screenshot_adapter_waits_for_ready_or_fatal_before_capture() {
        let adapter = include_str!("lib_screenshot.rs");
        let harness = include_str!("../../../tools/standalone_player_browser_smoke.mjs");

        assert!(adapter.contains("standalone_player_browser_smoke.mjs"));
        assert!(harness.contains(r#"status?.dataset.state === "ready""#));
        assert!(harness.contains(r#"status?.dataset.state === "fatal""#));
        assert!(harness.contains("#puzzle-bevy-fatal"));
        assert!(harness.contains("page.pageErrors.length"));
        assert!(harness.contains(r#"page.send("Page.captureScreenshot""#));
        assert!(harness.contains("assertPngDimensions(png, captureWidth, captureHeight)"));
        assert!(harness.contains("fs.renameSync(temporaryOutputPath, outputPath)"));
    }
}
