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

fn export_html(state: &ServerState) -> String {
    export_html_with_host_mode(state, StandaloneHostMode::Export)
}

fn export_html_with_host_mode(state: &ServerState, host_mode: StandaloneHostMode) -> String {
    export_html_with_runtime_wasm(state, host_mode, StandaloneRuntimeWasm::HostDefault)
}

fn export_html_with_runtime_wasm(
    state: &ServerState,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    let mut data = String::new();
    if host_mode == StandaloneHostMode::Export {
        push_runtime_export_data(&mut data, state);
    } else {
        push_export_data_with_source(&mut data, state, true);
    }
    let data = escape_script_json(&data);
    let mut boot_data = String::new();
    push_export_boot_data(
        &mut boot_data,
        state,
        host_mode == StandaloneHostMode::EditorPreview,
        host_mode == StandaloneHostMode::EditorPreview,
    );
    let boot_data = escape_script_json(&boot_data);
    let body_theme_attributes = preview_body_theme_attributes(&state.loaded.theme);
    let app_css = escape_style(APP_CSS);
    let theme_presets_css = escape_style(THEME_PRESETS_CSS);
    let renderer_css = escape_style(RENDERER_CSS);
    let game_css = escape_style(&state.game_css);
    let game_visuals_js = escape_script(&state.game_visuals_js);
    let renderer_js = escape_script(RENDERER_JS);
    let standalone_js = escape_script(STANDALONE_JS);
    let runtime_wasm_js = standalone_runtime_wasm_script(host_mode, runtime_wasm);
    let sound_tools_js = escape_script(&sound_tools_js());
    let app_js_source = standalone_host_js(state, host_mode);
    let app_js = escape_script(&app_js_source);

    let html = INDEX_HTML
        .replace("<title>PuzzleStudio HTML Play</title>", "<title>PuzzleStudio HTML Export</title>")
        .replace(
            r#"<link rel="stylesheet" href="/app.css">"#,
            &format!("<style>\n{app_css}\n</style>"),
        )
        .replace(
            r#"<link rel="stylesheet" href="/theme-presets.css">"#,
            &format!("<style>\n{theme_presets_css}\n</style>"),
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
            r#"<script src="/sound-generator.js"></script>"#,
            &format!("<script>\n{sound_tools_js}\n</script>"),
        )
        .replace(
            r#"<script src="/app.js"></script>"#,
            &format!("<script>\n{app_js}\n</script>"),
        )
        .replace("<body>", &format!("<body{body_theme_attributes}>"));
    replace_required_script_asset(
        html,
        "/renderer.js",
        &format!(
            "<script>\nwindow.PuzzleBoot = JSON.parse(\"{boot_data}\");\nwindow.PuzzleRuntimeExportJson = \"{data}\";\n{runtime_wasm_js}\n</script>\n<script>\n{renderer_js}\n</script>\n<script>\n{standalone_js}\n</script>"
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

fn standalone_host_js(state: &ServerState, host_mode: StandaloneHostMode) -> String {
    let mut script = APP_JS.to_string();
    script = strip_optional_host_blocks(&script, "solver");
    if host_mode == StandaloneHostMode::Export {
        script = strip_optional_host_blocks(&script, "studio-bridge");
        script = strip_optional_host_blocks(&script, "scene-editor");
    }
    if !loaded_uses_puzzle3_frames(&state.loaded) {
        script = strip_optional_host_blocks(&script, "puzzle3");
    }
    remove_optional_host_markers(&script)
}

fn loaded_uses_puzzle3_frames(loaded: &LoadedGame) -> bool {
    loaded.scenes.iter().any(|scene| {
        scene
            .components
            .iter()
            .any(component_contains_puzzle3_frame)
    })
}

fn strip_optional_host_blocks(source: &str, name: &str) -> String {
    let start_marker = format!("/* puzzle-host:optional:{name}:start */");
    let end_marker = format!("/* puzzle-host:optional:{name}:end */");
    let mut output = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find(&start_marker) {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + start_marker.len()..];
        let Some(end) = after_start.find(&end_marker) else {
            panic!("missing optional host end marker for {name}");
        };
        rest = &after_start[end + end_marker.len()..];
    }

    if rest.contains(&end_marker) {
        panic!("missing optional host start marker for {name}");
    }

    output.push_str(rest);
    output
}

fn remove_optional_host_markers(source: &str) -> String {
    let mut script = source.to_string();
    for name in ["solver", "studio-bridge", "scene-editor", "puzzle3"] {
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
        StandaloneHostMode::Export => embedded_game_wasm_script(),
        StandaloneHostMode::EditorPreview => editor_preview_runtime_wasm_script(),
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
            "Standalone HTML export requires embedded puzzle_wasm_game assets; the browser editor preview compiler cannot embed them.",
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
          const moduleUrl = URL.createObjectURL(new Blob([moduleSource], { type: "text/javascript" }));
          try {
            const module = await import(`${moduleUrl}#${encodeURIComponent(String(version))}`);
            const wasmBytes = decodeWasmBase64(wasmBase64);
            await module.default({ module_or_path: wasmBytes });
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

fn preview_body_theme_attributes(theme: &ThemeDef) -> String {
    let class_name = theme_class_name(theme.name.as_deref().unwrap_or("clean"));
    let mut attributes = String::new();
    if !class_name.is_empty() {
        let _ = write!(attributes, " class=\"{class_name}\"");
    }
    if !theme.variables.is_empty() {
        attributes.push_str(" style=\"");
        for variable in &theme.variables {
            attributes.push_str("--");
            attributes.push_str(&escape_html_attr(&variable.name));
            attributes.push(':');
            attributes.push_str(&escape_html_attr(&variable.value));
            attributes.push(';');
        }
        attributes.push('"');
    }
    attributes
}

fn theme_class_name(name: &str) -> String {
    let mut normalized = String::new();
    let mut previous_dash = false;
    for ch in name.chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if next == '-' {
            if normalized.is_empty() || previous_dash {
                previous_dash = true;
                continue;
            }
            previous_dash = true;
        } else {
            previous_dash = false;
        }
        normalized.push(next);
    }
    while normalized.ends_with('-') {
        normalized.pop();
    }
    if normalized.is_empty() {
        String::new()
    } else {
        format!("theme-{normalized}")
    }
}

fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn export_puzzle3_document_html(
    document: &puzzle_lang::LoadedDocument,
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, String> {
    export_puzzle3_document_html_with_runtime_wasm(
        document,
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::HostDefault,
    )
}

fn export_puzzle3_document_html_with_runtime_wasm(
    document: &puzzle_lang::LoadedDocument,
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, String> {
    let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(document)
        .map_err(|error| error.to_string())?;
    let runtime_sources = if host_mode == StandaloneHostMode::EditorPreview {
        Some(puzzle_lang::split_document_runtime_sources(source).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let loaded = loaded_document_scene_host_loaded_game(document)?;
    let state_source = if host_mode == StandaloneHostMode::EditorPreview {
        source.to_string()
    } else {
        String::new()
    };
    let state = ServerState::new(
        loaded,
        state_source,
        puzzle_path.to_string(),
        game_css.to_string(),
        game_visuals_js.to_string(),
        SolverConfig::default(),
    );
    Ok(inject_puzzle3_frame_assets(
        export_html_with_runtime_wasm(&state, host_mode, runtime_wasm),
        &fixture_json,
        runtime_sources
            .as_ref()
            .map(|sources| sources.model_3d.as_str()),
        Some(puzzle_path),
    ))
}

fn export_mixed_document_html(
    document: &puzzle_lang::LoadedDocument,
    loaded: LoadedGame,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, String> {
    let fixture_json = mixed_document_puzzle3_fixture_json(document)?;
    let runtime_sources = if host_mode == StandaloneHostMode::EditorPreview {
        Some(puzzle_lang::split_document_runtime_sources(&source).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let puzzle3_path = puzzle_path.clone();
    let state_source = runtime_sources
        .as_ref()
        .map(|sources| sources.model_2d.clone())
        .unwrap_or_default();
    let state = ServerState::new(
        loaded,
        state_source,
        puzzle_path,
        game_css,
        game_visuals_js,
        solver,
    );
    let html = export_html_with_runtime_wasm(&state, host_mode, runtime_wasm);
    Ok(inject_puzzle3_frame_assets(
        html,
        &fixture_json,
        runtime_sources
            .as_ref()
            .map(|sources| sources.model_3d.as_str()),
        Some(&puzzle3_path),
    ))
}

fn mixed_document_puzzle3_fixture_json(
    document: &puzzle_lang::LoadedDocument,
) -> Result<String, String> {
    let Some(LoadedDocumentModel::Puzzle3d { name, puzzle }) = document
        .models
        .iter()
        .find(|model| matches!(model, LoadedDocumentModel::Puzzle3d { .. }))
    else {
        return Err("mixed HTML export requires a 3D puzzle model".to_string());
    };
    let puzzle3_document = puzzle_lang::LoadedDocument {
        title: document.title.clone(),
        subtitle: document.subtitle.clone(),
        author: document.author.clone(),
        homepage: document.homepage.clone(),
        default_wait_ms: document.default_wait_ms,
        default_again_ms: document.default_again_ms,
        input_buffer: document.input_buffer.clone(),
        animation: document.animation.clone(),
        sounds: document.sounds.clone(),
        theme: document.theme.clone(),
        assets: document.assets.clone(),
        scenes: document.scenes.clone(),
        models: vec![LoadedDocumentModel::Puzzle3d {
            name: name.clone(),
            puzzle: puzzle.clone(),
        }],
    };
    puzzle_lang::export_loaded_document_visual_fixture_json(&puzzle3_document)
        .map_err(|error| error.to_string())
}

fn inject_puzzle3_frame_assets(
    html: String,
    fixture_json: &str,
    source: Option<&str>,
    puzzle_path: Option<&str>,
) -> String {
    let mut assets = String::new();
    assets.push('{');
    let mut needs_comma = false;
    if let Some(source) = source {
        push_json_string(&mut assets, "source");
        assets.push(':');
        push_json_string(&mut assets, source);
        needs_comma = true;
    }
    if let Some(puzzle_path) = puzzle_path {
        if needs_comma {
            assets.push(',');
        }
        push_json_string(&mut assets, "puzzlePath");
        assets.push(':');
        push_json_string(&mut assets, puzzle_path);
    }
    assets.push('}');
    let assets = escape_script(&assets);
    let fixture_json = escape_script_json(fixture_json);
    let style_css = escape_style(PUZZLE3_STYLE_CSS);
    let visual_core_js = escape_script(PUZZLE3_VISUAL_CORE_JS);
    let three_renderer_js = escape_script(PUZZLE3_THREE_RENDERER_JS);
    let mut three_module_source = String::new();
    push_json_string(&mut three_module_source, THREE_MODULE_JS);
    let three_module_source = escape_script(&three_module_source);
    let puzzle3_app_js = escape_script(PUZZLE3_APP_JS);
    let html = html.replace(
        "</head>",
        &format!("<style>\n{style_css}\n</style>\n</head>"),
    );
    html.replace(
        "window.PuzzleBoot = JSON.parse(",
        &format!(
            "window.Puzzle3DFrameFixture = JSON.parse(\"{fixture_json}\");\nwindow.Puzzle3DFrameAssets = {assets};\nwindow.Puzzle3ControllerAutoBoot = false;\nwindow.Puzzle3ThreeModuleSource = {three_module_source};\n{visual_core_js}\n{three_renderer_js}\n{puzzle3_app_js}\nwindow.PuzzleBoot = JSON.parse("
        ),
    )
}

fn sound_tools_js() -> String {
    fn module_body(source: &str) -> String {
        source
            .lines()
            .filter(|line| !line.trim_start().starts_with("import "))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("export const ", "const ")
            .replace("export async function ", "async function ")
            .replace("export function ", "function ")
    }

    fn expose_module(source: &str, exports: &[&str]) -> String {
        let body = module_body(source);
        format!("{body}\nreturn {{{}}};", exports.join(","))
    }

    format!(
        "(() => {{
  const sfx = (() => {{
{}
  }})();
  const musicPlayer = (() => {{
{}
  }})();
  const musicGenerator = (() => {{
{}
{}
  }})();
  const music = {{ createPlayer: musicPlayer.createPlayer, generateSong: musicGenerator.generateSong, randomPreset: musicGenerator.randomPreset }};
  window.PuzzleSoundTools = {{ ...(window.PuzzleSoundTools || {{}}), ...sfx, ...music }};
  window.PuzzleSoundGenerator = window.PuzzleSoundTools;
  window.dispatchEvent(new CustomEvent(\"PuzzleSoundToolsReady\"));
}})();",
        expose_module(
            SEEDED_SFX_JS,
            &[
                "SFX_TYPE_OPTIONS",
                "createSfxPlayer",
                "createPuzzleScriptSfxPlayer",
                "generateSoundEffect",
                "generatePuzzleScriptSoundEffect",
                "randomSfxPreset",
            ],
        ),
        expose_module(SEEDED_MUSIC_PLAYER_JS, &["createPlayer"]),
        module_body(SEEDED_TIMBRE_FIELDS_JS),
        expose_module(SEEDED_MUSIC_JS, &["generateSong", "randomPreset"]),
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
    game_runtime_module_js: &str,
    game_runtime_wasm_base64: &str,
) -> Result<String, DiagnosticReport> {
    if game_runtime_module_js.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_game.js content.",
        ));
    }
    if game_runtime_wasm_base64.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_game_bg.wasm content.",
        ));
    }
    export_html_from_source_with_runtime_wasm(
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::EmbeddedBase64 {
            module_source: game_runtime_module_js,
            wasm_base64: game_runtime_wasm_base64,
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

pub fn export_solver_rules_json_from_source(
    source: &str,
    puzzle_path: &str,
) -> Result<String, DiagnosticReport> {
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    if matches!(document.single_model(), Some(LoadedDocumentModel::Puzzle3d { .. })) {
        return Err(DiagnosticReport::error(
            "prepared solver rules currently require a 2D puzzle".to_string(),
        ));
    }
    let loaded = loaded_document_scene_host_loaded_game(&document)
        .map_err(DiagnosticReport::error)?;
    let mut out = String::new();
    push_editor_solver_rules(&mut out, &loaded);
    Ok(out)
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
    if matches!(document.single_model(), Some(LoadedDocumentModel::Puzzle3d { .. })) {
        return export_puzzle3_document_html_with_runtime_wasm(
            &document,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
            host_mode,
            runtime_wasm,
        )
        .map_err(DiagnosticReport::error);
    }
    let loaded =
        loaded_document_scene_host_loaded_game(&document).map_err(DiagnosticReport::error)?;
    runtime_loaded_game_json(&loaded).map_err(|error| {
        DiagnosticReport::error(format!(
            "runtime loaded game bundle failed to serialize: {error}"
        ))
    })?;
    let game_visuals_js = join_visuals_js(game_visuals_js, &generated_visuals_js(&loaded));
    if document.models.len() > 1 {
        export_mixed_document_html(
            &document,
            loaded,
            source.to_string(),
            puzzle_path.to_string(),
            game_css.to_string(),
            game_visuals_js,
            SolverConfig::default(),
            host_mode,
            runtime_wasm,
        )
        .map_err(DiagnosticReport::error)
    } else {
        let state = ServerState::new(
            loaded,
            source.to_string(),
            puzzle_path.to_string(),
            game_css.to_string(),
            game_visuals_js,
            SolverConfig::default(),
        );
        Ok(export_html_with_runtime_wasm(&state, host_mode, runtime_wasm))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_html_file(path: impl AsRef<Path>) -> Result<String, String> {
    let puzzle_path = resolve_game_entry(path).map_err(|error| error.to_string())?;
    let source = fs::read_to_string(&puzzle_path).map_err(|error| error.to_string())?;
    let source = if puzzle_lang::puzzle_source_profile_for_path(&puzzle_path)
        == Some(puzzle_lang::PuzzleSourceProfile::Puzzle3d)
    {
        source
    } else {
        expand_game_imports_for_file(&source, &puzzle_path).map_err(|error| error.to_string())?
    };
    let document = puzzle_lang::parse_game_for_path(&source, &puzzle_path)
        .map_err(|error| error.to_string())?;
    let game_css =
        load_asset_css(&puzzle_path, &document.assets).map_err(|error| error.to_string())?;

    if matches!(document.single_model(), Some(LoadedDocumentModel::Puzzle3d { .. })) {
        return export_puzzle3_document_html(
            &document,
            &source,
            &puzzle_path.display().to_string(),
            &game_css,
            VISUALS_JS,
        );
    }

    let loaded = loaded_document_scene_host_loaded_game(&document)?;
    let game_visuals_js =
        load_game_visuals_js(&puzzle_path, &loaded).map_err(|error| error.to_string())?;
    if document.models.len() > 1 {
        export_mixed_document_html(
            &document,
            loaded,
            source,
            puzzle_path.display().to_string(),
            game_css,
            game_visuals_js,
            SolverConfig::default(),
            StandaloneHostMode::Export,
            StandaloneRuntimeWasm::HostDefault,
        )
    } else {
        let state = ServerState::new(
            loaded,
            source,
            puzzle_path.display().to_string(),
            game_css,
            game_visuals_js,
            SolverConfig::default(),
        );
        Ok(export_html(&state))
    }
}

pub fn export_visuals_js_from_source(
    source: &str,
    base_visuals_js: &str,
) -> Result<String, String> {
    let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
    if matches!(document.single_model(), Some(LoadedDocumentModel::Puzzle3d { .. })) {
        Ok(base_visuals_js.to_string())
    } else {
        let loaded = loaded_document_scene_host_loaded_game(&document)?;
        Ok(join_visuals_js(
            base_visuals_js,
            &generated_visuals_js(&loaded),
        ))
    }
}
