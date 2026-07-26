#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StandaloneHostMode {
    Export,
    EditorPreview,
}

#[derive(Clone, Copy, Debug)]
enum StandaloneRuntimeWasm<'a> {
    HostDefault,
    EmbeddedBase64 {
        module_source: &'a str,
        wasm_base64: &'a str,
    },
}

fn export_html_with_runtime_wasm(
    state: &EditorPreviewState,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    let runtime_export = runtime_export_json(&state.standalone_export)
        .expect("standalone runtime document should serialize");
    if host_mode == StandaloneHostMode::Export {
        return bevy_export_html(&runtime_export, runtime_wasm);
    }
    let mut boot_data = String::new();
    push_export_boot_data(
        &mut boot_data,
        host_mode == StandaloneHostMode::EditorPreview,
    );
    let html = standalone_html(
        &boot_data,
        &runtime_export,
        &state.game_css,
        &state.game_visuals_js,
        document_uses_puzzle3_renderer(preview_state_document(state)),
        host_mode,
        runtime_wasm,
    );
    if document_uses_puzzle3_renderer(preview_state_document(state)) {
        let fixtures = puzzle3_frame_fixtures_json(preview_state_document(state));
        inject_puzzle3_frame_assets(html, &fixtures, Some(&state.puzzle_path))
    } else {
        html
    }
}

fn bevy_export_html(runtime_export: &str, runtime_wasm: StandaloneRuntimeWasm<'_>) -> String {
    let runtime_export = escape_script_json(runtime_export);
    let runtime_wasm_js = standalone_runtime_wasm_script(StandaloneHostMode::Export, runtime_wasm);
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>PuzzleStudio HTML Export</title>
    <style>
      html, body {{
        width: 100%;
        height: 100%;
        margin: 0;
        overflow: hidden;
        background: #000;
      }}
      #puzzle-bevy {{
        display: block;
        width: 100%;
        height: 100%;
      }}
      #puzzle-bevy-fatal {{
        position: fixed;
        inset: 0;
        display: none;
        box-sizing: border-box;
        padding: 2rem;
        overflow: auto;
        white-space: pre-wrap;
        color: #fff;
        background: #280b0b;
        font: 16px/1.5 system-ui, sans-serif;
      }}
    </style>
  </head>
  <body>
    <canvas id="puzzle-bevy" aria-label="Puzzle game"></canvas>
    <pre id="puzzle-bevy-fatal" role="alert"></pre>
    <output id="puzzle-bevy-status" hidden data-state="starting"></output>
    <script>
window.PuzzleRuntimeExportJson = "{runtime_export}";
{runtime_wasm_js}
    </script>
    <script>
      (async () => {{
        const fatal = document.getElementById("puzzle-bevy-fatal");
        const status = document.getElementById("puzzle-bevy-status");
        try {{
          const player = await window.PuzzleRuntimeWasmLoader.load("bevy-player");
          if (typeof player.startStandalonePlayer !== "function") {{
            throw new Error("Standalone player WASM is missing startStandalonePlayer.");
          }}
          await player.startStandalonePlayer(window.PuzzleRuntimeExportJson, "#puzzle-bevy");
        }} catch (error) {{
          const message = error?.stack || error?.message || String(error);
          fatal.textContent = `PuzzleStudio standalone player failed:\n${{message}}`;
          fatal.style.display = "block";
          status.dataset.state = "fatal";
          console.error(message);
        }}
      }})();
    </script>
  </body>
</html>
"##
    )
}

fn standalone_html(
    boot_data: &str,
    runtime_export: &str,
    game_css: &str,
    game_visuals_js: &str,
    uses_puzzle3_frames: bool,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    let boot_data = escape_script_json(boot_data);
    let runtime_export = escape_script_json(runtime_export);
    let app_css = escape_style(APP_CSS);
    let renderer_css = escape_style(RENDERER_CSS);
    let game_css = escape_style(game_css);
    let game_visuals_js = escape_script(game_visuals_js);
    let visual_tween_core_js = escape_script(VISUAL_TWEEN_CORE_JS);
    let renderer_js = escape_script(RENDERER_JS);
    let standalone_js_source = standalone_runtime_js(host_mode);
    let standalone_js = escape_script(&standalone_js_source);
    let runtime_wasm_js = standalone_runtime_wasm_script(host_mode, runtime_wasm);
    let app_js_source = standalone_host_js(uses_puzzle3_frames, host_mode);
    let app_js = escape_script(&app_js_source);

    let html = INDEX_HTML
        .replace(
            "<title>PuzzleStudio HTML Play</title>",
            "<title>PuzzleStudio HTML Export</title>",
        )
        .replace(
            r#"<link rel="stylesheet" href="/app.css">"#,
            &format!("<style>\n{app_css}\n</style>"),
        )
        .replace(
            r#"<link rel="stylesheet" href="/renderer.css">"#,
            &format!("<style>\n{renderer_css}\n</style>"),
        )
        .replace(
            r#"<link rel="stylesheet" href="/game.css">"#,
            &format!("<style>\n{game_css}\n</style>"),
        )
        .replace(
            r#"<script src="/game.visuals.js"></script>"#,
            &format!("<script>\n{game_visuals_js}\n</script>"),
        )
        .replace(
            r#"<script src="/app.js"></script>"#,
            &format!("<script>\n{app_js}\n</script>"),
        );
    let html = replace_required_script_asset(
        html,
        "/visual_tween_core.js",
        &format!("<script>\n{visual_tween_core_js}\n</script>"),
    );
    replace_required_script_asset(
        html,
        "/renderer.js",
        &format!(
            "<script>\nwindow.PuzzleBoot = JSON.parse(\"{boot_data}\");\nwindow.PuzzleRuntimeExportJson = \"{runtime_export}\";\n{runtime_wasm_js}\n</script>\n<script>\n{renderer_js}\n</script>\n<script>\n{standalone_js}\n</script>"
        ),
    )
}

fn replace_required_script_asset(html: String, asset_path: &str, replacement: &str) -> String {
    let prefix = format!(r#"<script src="{asset_path}"#);
    let count = html.matches(&prefix).count();
    assert_eq!(
        count, 1,
        "HTML export template must contain exactly one script for {asset_path}, found {count}"
    );
    let start = html
        .find(&prefix)
        .expect("checked renderer script tag is present");
    let end = html[start..]
        .find("></script>")
        .map(|offset| start + offset + "></script>".len())
        .expect("HTML export renderer script tag must close");
    let mut output = html;
    output.replace_range(start..end, replacement);
    output
}

fn standalone_host_js(uses_puzzle3_frames: bool, host_mode: StandaloneHostMode) -> String {
    let mut script = APP_JS.to_string();
    script = strip_optional_host_blocks(&script, "solver");
    if host_mode == StandaloneHostMode::Export {
        script = strip_optional_host_blocks(&script, "studio-bridge");
        script = strip_optional_host_blocks(&script, "scene-editor");
    }
    if !uses_puzzle3_frames {
        script = strip_optional_host_blocks(&script, "puzzle3");
    }
    remove_optional_host_markers(&script)
}

fn standalone_runtime_js(host_mode: StandaloneHostMode) -> String {
    let mut script = STANDALONE_JS.to_string();
    if host_mode == StandaloneHostMode::Export {
        script = strip_optional_host_blocks(&script, "editor-preview");
    }
    remove_optional_host_markers(&script)
}

fn strip_optional_host_blocks(source: &str, name: &str) -> String {
    let start_marker = format!("/* puzzle-host:optional:{name}:start */");
    let end_marker = format!("/* puzzle-host:optional:{name}:end */");
    let mut output = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find(&start_marker) {
        let line_start = rest[..start].rfind('\n').map_or(0, |index| index + 1);
        let block_start = rest[line_start..start]
            .chars()
            .all(char::is_whitespace)
            .then_some(line_start)
            .unwrap_or(start);
        output.push_str(&rest[..block_start]);
        let after_start = &rest[start + start_marker.len()..];
        let Some(end) = after_start.find(&end_marker) else {
            panic!("missing optional host end marker for {name}");
        };
        let after_end = &after_start[end + end_marker.len()..];
        let line_end = after_end
            .find('\n')
            .map_or(after_end.len(), |index| index + 1);
        rest = if after_end[..line_end].chars().all(char::is_whitespace) {
            &after_end[line_end..]
        } else {
            after_end
        };
    }

    if rest.contains(&end_marker) {
        panic!("missing optional host start marker for {name}");
    }

    output.push_str(rest);
    output
}

fn remove_optional_host_markers(source: &str) -> String {
    let mut script = source.to_string();
    for name in [
        "solver",
        "studio-bridge",
        "scene-editor",
        "puzzle3",
        "editor-preview",
    ] {
        script = script.replace(&format!("/* puzzle-host:optional:{name}:start */"), "");
        script = script.replace(&format!("/* puzzle-host:optional:{name}:end */"), "");
    }
    script
}

fn standalone_runtime_wasm_script(
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    if let StandaloneRuntimeWasm::EmbeddedBase64 {
        module_source,
        wasm_base64,
    } = runtime_wasm
    {
        return embedded_base64_wasm_loader_script(module_source, wasm_base64);
    }
    match host_mode {
        StandaloneHostMode::Export => embedded_player_wasm_script(),
        StandaloneHostMode::EditorPreview => editor_preview_runtime_wasm_script(),
    }
}

fn embedded_player_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_wasm_loader_script(PUZZLE_PLAYER_WASM_JS, PUZZLE_PLAYER_WASM_BG)
    }
    #[cfg(target_arch = "wasm32")]
    {
        missing_embedded_wasm_loader_script(
            "Standalone HTML export requires embedded puzzle_wasm_player assets.",
        )
    }
}

fn editor_preview_runtime_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_game_wasm_script()
    }
    #[cfg(target_arch = "wasm32")]
    {
        editor_preview_parent_wasm_loader_script()
    }
}

fn embedded_game_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_wasm_loader_script(PUZZLE_GAME_WASM_JS, PUZZLE_GAME_WASM_BG)
    }
    #[cfg(target_arch = "wasm32")]
    {
        missing_embedded_wasm_loader_script(
            "Editor preview requires puzzle_wasm_game assets from its editor host.",
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn embedded_wasm_loader_script(module_source: &str, wasm: &[u8]) -> String {
    let wasm_base64 = base64_encode(wasm);
    embedded_base64_wasm_loader_script(module_source, &wasm_base64)
}

fn embedded_base64_wasm_loader_script(module_source: &str, wasm_base64: &str) -> String {
    let module_source = escape_script_json(module_source);
    let wasm_base64 = escape_script_json(wasm_base64);
    format!(
        r#"window.PuzzleStandaloneEmbeddedWasm = {{ moduleSource: "{module_source}", wasmBase64: "{wasm_base64}" }};
window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {{
  let modulePromise = null;
  function decodeWasmBase64(value) {{
    if (typeof Uint8Array.fromBase64 !== "function") {{
      throw new Error("Standalone HTML export requires Uint8Array.fromBase64 for embedded WASM decoding.");
    }}
    return Uint8Array.fromBase64(value);
  }}
  return {{
    async load(version = "embedded") {{
      if (!modulePromise) {{
        const embedded = window.PuzzleStandaloneEmbeddedWasm;
        const moduleUrl = URL.createObjectURL(new Blob([embedded.moduleSource], {{ type: "text/javascript" }}));
        modulePromise = import(`${{moduleUrl}}#${{encodeURIComponent(String(version))}}`)
          .then(async (module) => {{
            const wasmBytes = decodeWasmBase64(embedded.wasmBase64);
            embedded.wasmBase64 = "";
            await module.default({{ module_or_path: wasmBytes }});
            return module;
          }})
          .finally(() => URL.revokeObjectURL(moduleUrl));
      }}
      return modulePromise;
    }},
  }};
}})();"#
    )
}

#[cfg(target_arch = "wasm32")]
fn missing_embedded_wasm_loader_script(message: &str) -> String {
    let message = escape_script_json(message);
    format!(
        r#"window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || {{
  async load() {{
    throw new Error("{message}");
  }},
}};"#
    )
}

#[cfg(target_arch = "wasm32")]
fn editor_preview_parent_wasm_loader_script() -> String {
    r#"window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {
  let modulePromise = null;
  let nextRequestId = 1;

  function reportProgress(stage) {
    window.parent.postMessage({
      type: "PuzzleStudioPreviewRuntimeProgress",
      stage,
    }, "*");
  }

  function requestAsset(kind) {
    return new Promise((resolve, reject) => {
      const requestId = `runtime-asset-${Date.now()}-${nextRequestId++}`;
      const timeout = window.setTimeout(() => {
        window.removeEventListener("message", handleMessage);
        reject(new Error(`Timed out waiting for editor preview runtime asset: ${kind}`));
      }, 15000);
      function handleMessage(event) {
        const data = event.data || {};
        if (data.type !== "PuzzleStudioRuntimeAssetResponse" || data.requestId !== requestId) {
          return;
        }
        window.clearTimeout(timeout);
        window.removeEventListener("message", handleMessage);
        if (!data.ok) {
          reject(new Error(data.error || `Editor preview runtime asset is unavailable: ${kind}`));
          return;
        }
        resolve(String(data.value || ""));
      }
      window.addEventListener("message", handleMessage);
      window.parent.postMessage({
        type: "PuzzleStudioRuntimeAssetRequest",
        requestId,
        kind,
      }, "*");
    });
  }

  function decodeWasmBase64(value) {
    if (typeof Uint8Array.fromBase64 !== "function") {
      throw new Error("Editor preview requires Uint8Array.fromBase64 for WASM decoding.");
    }
    return Uint8Array.fromBase64(value);
  }

  return {
    async load(version = "editor-preview") {
      if (!modulePromise) {
        modulePromise = Promise.all([
          requestAsset("puzzle_wasm_game.js"),
          requestAsset("puzzle_wasm_game_bg.wasm.base64"),
        ]).then(async ([moduleSource, wasmBase64]) => {
          reportProgress("assets-received");
          const moduleUrl = URL.createObjectURL(new Blob([moduleSource], { type: "text/javascript" }));
          try {
            const module = await import(`${moduleUrl}#${encodeURIComponent(String(version))}`);
            reportProgress("module-imported");
            const wasmBytes = decodeWasmBase64(wasmBase64);
            reportProgress("wasm-decoded");
            await module.default({ module_or_path: wasmBytes });
            reportProgress("wasm-initialized");
            return module;
          } finally {
            URL.revokeObjectURL(moduleUrl);
          }
        }).catch((error) => {
          modulePromise = null;
          throw error;
        });
      }
      return modulePromise;
    },
  };
})();"#.to_string()
}

fn export_bevy_document_html(
    document: &puzzle_lang::LoadedDocument,
    puzzle_path: &str,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, String> {
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)
        .map_err(|error| error.to_string())?;
    let progress_storage = standalone_progress_storage(document);
    let standalone_export =
        StandaloneRuntimeExport::new(document.clone(), visual_images, progress_storage);
    let runtime_export =
        runtime_export_json(&standalone_export).map_err(|error| error.to_string())?;
    Ok(bevy_export_html(&runtime_export, runtime_wasm))
}

fn document_uses_puzzle3_renderer(document: &puzzle_lang::LoadedDocument) -> bool {
    document
        .models
        .iter()
        .any(|model| matches!(model, LoadedDocumentModel::Puzzle3d { .. }))
}

fn puzzle3_frame_fixtures_json(document: &puzzle_lang::LoadedDocument) -> String {
    let mut fixtures = serde_json::Map::new();
    for model in &document.models {
        let LoadedDocumentModel::Puzzle3d {
            name,
            game,
            presentation,
        } = model
        else {
            continue;
        };
        let fixture = puzzle_lang::export_visual_fixture_json(game, presentation)
            .expect("validated 3D editor model must export its visual fixture");
        let fixture = serde_json::from_str(&fixture)
            .expect("typed 3D editor visual fixture must serialize as JSON");
        fixtures.insert(name.clone(), fixture);
    }
    serde_json::to_string(&fixtures).expect("typed 3D editor fixture map must serialize")
}

fn inject_puzzle3_frame_assets(
    html: String,
    fixtures_json: &str,
    puzzle_path: Option<&str>,
) -> String {
    let mut assets = String::new();
    assets.push('{');
    if let Some(puzzle_path) = puzzle_path {
        push_json_string(&mut assets, "puzzlePath");
        assets.push(':');
        push_json_string(&mut assets, puzzle_path);
    }
    assets.push('}');
    let assets = escape_script(&assets);
    let fixtures_json = escape_script_json(fixtures_json);
    let style_css = escape_style(PUZZLE3_STYLE_CSS);
    let visual_core_js = escape_script(PUZZLE3_VISUAL_CORE_JS);
    let three_renderer_js = escape_script(PUZZLE3_THREE_RENDERER_JS);
    let mut three_module_source = String::new();
    push_json_string(&mut three_module_source, THREE_MODULE_JS);
    let three_module_source = escape_script(&three_module_source);
    let puzzle3_component_js = escape_script(PUZZLE3_COMPONENT_JS);
    let html = html.replace(
        "</head>",
        &format!("<style>\n{style_css}\n</style>\n</head>"),
    );
    html.replace(
        "window.PuzzleBoot = JSON.parse(",
        &format!(
            "window.Puzzle3DFrameFixtures = JSON.parse(\"{fixtures_json}\");\nwindow.Puzzle3DFrameAssets = {assets};\nwindow.Puzzle3ThreeModuleSource = {three_module_source};\n{visual_core_js}\n{three_renderer_js}\n{puzzle3_component_js}\nwindow.PuzzleBoot = JSON.parse("
        ),
    )
}

pub fn export_html_from_source(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_host_mode(
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::Export,
    )
}

pub fn export_html_from_source_with_embedded_wasm(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, DiagnosticReport> {
    if player_runtime_module_js.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player.js content.",
        ));
    }
    if player_runtime_wasm_base64.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player_bg.wasm content.",
        ));
    }
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    export_html_from_document_with_runtime_wasm(
        &document,
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::EmbeddedBase64 {
            module_source: player_runtime_module_js,
            wasm_base64: player_runtime_wasm_base64,
        },
    )
}

pub fn export_html_from_document_with_embedded_wasm(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, DiagnosticReport> {
    if player_runtime_module_js.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player.js content.",
        ));
    }
    if player_runtime_wasm_base64.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player_bg.wasm content.",
        ));
    }
    export_html_from_document_with_runtime_wasm(
        document,
        entry_source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::EmbeddedBase64 {
            module_source: player_runtime_module_js,
            wasm_base64: player_runtime_wasm_base64,
        },
    )
}

pub fn export_editor_preview_html_from_source(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_host_mode(
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::EditorPreview,
    )
}

pub fn export_editor_preview_html_from_document(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_document_with_runtime_wasm(
        document,
        entry_source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::EditorPreview,
        StandaloneRuntimeWasm::HostDefault,
    )
}

pub fn export_editor_preview_build_from_document(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, DiagnosticReport> {
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)?;
    let state = EditorPreviewState::new(
        document.clone(),
        entry_source.to_string(),
        puzzle_path.to_string(),
        visual_images,
        game_css.to_string(),
        game_visuals_js.to_string(),
    )
    .map_err(DiagnosticReport::error)?;
    let html = export_html_with_runtime_wasm(
        &state,
        StandaloneHostMode::EditorPreview,
        StandaloneRuntimeWasm::HostDefault,
    );
    Ok(editor_preview_build_json(&html, &state))
}

fn export_html_from_source_with_host_mode(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    host_mode: StandaloneHostMode,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_runtime_wasm(
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        host_mode,
        StandaloneRuntimeWasm::HostDefault,
    )
}

fn export_html_from_source_with_runtime_wasm(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, DiagnosticReport> {
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    export_html_from_document_with_runtime_wasm(
        &document,
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        host_mode,
        runtime_wasm,
    )
}

fn export_html_from_document_with_runtime_wasm(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, DiagnosticReport> {
    if host_mode == StandaloneHostMode::Export {
        return export_bevy_document_html(document, puzzle_path, runtime_wasm)
            .map_err(DiagnosticReport::error);
    }
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)?;
    let state = EditorPreviewState::new(
        document.clone(),
        entry_source.to_string(),
        puzzle_path.to_string(),
        visual_images,
        game_css.to_string(),
        game_visuals_js.to_string(),
    )
    .map_err(DiagnosticReport::error)?;
    Ok(export_html_with_runtime_wasm(
        &state,
        host_mode,
        runtime_wasm,
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_html_file(path: impl AsRef<Path>) -> Result<String, String> {
    let puzzle_path = resolve_game_entry(path).map_err(|error| error.to_string())?;
    let root = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let workspace = puzzle_workspace::FileWorkspace::load(&puzzle_path, root)?;
    let document = workspace.compile().map_err(|error| error.to_string())?;
    export_bevy_document_html(
        &document,
        &puzzle_path.display().to_string(),
        StandaloneRuntimeWasm::HostDefault,
    )
}

pub fn export_visuals_js_from_source(
    source: &str,
    base_visuals_js: &str,
) -> Result<String, String> {
    puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
    Ok(base_visuals_js.to_string())
}
