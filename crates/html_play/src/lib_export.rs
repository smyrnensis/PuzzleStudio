#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StandaloneHostMode {
    Export,
    EditorPreview,
}

fn export_html(state: &ServerState) -> String {
    export_html_with_host_mode(state, StandaloneHostMode::Export)
}

fn export_editor_preview_html(state: &ServerState) -> String {
    export_html_with_host_mode(state, StandaloneHostMode::EditorPreview)
}

fn export_html_with_host_mode(state: &ServerState, host_mode: StandaloneHostMode) -> String {
    let mut data = String::new();
    push_export_data(&mut data, state);
    let data = escape_script_json(&data);
    let body_theme_attributes = preview_body_theme_attributes(&state.loaded.theme);
    let app_css = escape_style(APP_CSS);
    let theme_presets_css = escape_style(THEME_PRESETS_CSS);
    let renderer_css = escape_style(RENDERER_CSS);
    let game_css = escape_style(&state.game_css);
    let game_visuals_js = escape_script(&state.game_visuals_js);
    let renderer_js = escape_script(RENDERER_JS);
    let standalone_js = escape_script(STANDALONE_JS);
    let runtime_wasm_js = standalone_runtime_wasm_script(host_mode);
    let sound_tools_js = escape_script(&sound_tools_js());
    let app_js_source = standalone_host_js(state, host_mode);
    let app_js = escape_script(&app_js_source);

    INDEX_HTML
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
            r#"<script src="/renderer.js"></script>"#,
            &format!(
                "<script>\nwindow.PuzzleExport = JSON.parse(\"{data}\");\n{runtime_wasm_js}\n</script>\n<script>\n{renderer_js}\n</script>\n<script>\n{standalone_js}\n</script>"
            ),
        )
        .replace(
            r#"<script src="/app.js"></script>"#,
            &format!("<script>\n{app_js}\n</script>"),
        )
        .replace("<body>", &format!("<body{body_theme_attributes}>"))
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

fn standalone_runtime_wasm_script(host_mode: StandaloneHostMode) -> String {
    match host_mode {
        StandaloneHostMode::Export => embedded_standalone_wasm_script(),
        StandaloneHostMode::EditorPreview => editor_preview_runtime_wasm_script(),
    }
}

fn editor_preview_runtime_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_standalone_wasm_script()
    }
    #[cfg(target_arch = "wasm32")]
    {
        editor_preview_parent_wasm_loader_script()
    }
}

fn embedded_standalone_wasm_script() -> String {
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
    let module_source = escape_script_json(module_source);
    let wasm_base64 = base64_encode(wasm);
    format!(
        r#"window.PuzzleStandaloneEmbeddedWasm = {{ moduleSource: "{module_source}", wasmBase64: "{wasm_base64}" }};
window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {{
  let modulePromise = null;
  function base64ToUint8Array(value) {{
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {{
      bytes[index] = binary.charCodeAt(index);
    }}
    return bytes;
  }}
  return {{
    async load(version = "embedded") {{
      if (!modulePromise) {{
        const embedded = window.PuzzleStandaloneEmbeddedWasm;
        const moduleUrl = URL.createObjectURL(new Blob([embedded.moduleSource], {{ type: "text/javascript" }}));
        modulePromise = import(`${{moduleUrl}}#${{encodeURIComponent(String(version))}}`)
          .then(async (module) => {{
            await module.default({{ module_or_path: base64ToUint8Array(embedded.wasmBase64) }});
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

  function base64ToUint8Array(value) {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
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
            await module.default({ module_or_path: base64ToUint8Array(wasmBase64) });
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
    let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(document)
        .map_err(|error| error.to_string())?;
    let runtime_sources =
        puzzle_lang::split_document_runtime_sources(source).map_err(|error| error.to_string())?;
    let loaded = puzzle3_document_scene_host_loaded_game(document)?;
    let state = ServerState::new(
        loaded,
        source.to_string(),
        puzzle_path.to_string(),
        game_css.to_string(),
        game_visuals_js.to_string(),
        SolverConfig::default(),
    );
    Ok(inject_puzzle3_frame_assets(
        export_html(&state),
        &fixture_json,
        &runtime_sources.model_3d,
        puzzle_path,
    ))
}

fn puzzle3_document_scene_host_loaded_game(
    document: &puzzle_lang::LoadedDocument,
) -> Result<LoadedGame, String> {
    let mut loaded = parse_game(PUZZLE3_SCENE_HOST_SOURCE).map_err(|error| error.to_string())?;
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

fn export_mixed_document_html(
    document: &puzzle_lang::LoadedDocument,
    loaded: LoadedGame,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    host_mode: StandaloneHostMode,
) -> Result<String, String> {
    let fixture_json = mixed_document_puzzle3_fixture_json(document)?;
    let runtime_sources =
        puzzle_lang::split_document_runtime_sources(&source).map_err(|error| error.to_string())?;
    let puzzle3_path = puzzle_path.clone();
    let state = ServerState::new(
        loaded,
        runtime_sources.model_2d,
        puzzle_path,
        game_css,
        game_visuals_js,
        solver,
    );
    let html = match host_mode {
        StandaloneHostMode::Export => export_html(&state),
        StandaloneHostMode::EditorPreview => export_editor_preview_html(&state),
    };
    Ok(inject_puzzle3_frame_assets(
        html,
        &fixture_json,
        &runtime_sources.model_3d,
        &puzzle3_path,
    ))
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
    source: &str,
    puzzle_path: &str,
) -> String {
    let mut assets = String::new();
    assets.push('{');
    push_json_string(&mut assets, "source");
    assets.push(':');
    push_json_string(&mut assets, source);
    assets.push(',');
    push_json_string(&mut assets, "puzzlePath");
    assets.push(':');
    push_json_string(&mut assets, puzzle_path);
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
        "window.PuzzleExport = JSON.parse(",
        &format!(
            "window.Puzzle3DFrameFixture = JSON.parse(\"{fixture_json}\");\nwindow.Puzzle3DFrameAssets = {assets};\nwindow.Puzzle3ControllerAutoBoot = false;\nwindow.Puzzle3ThreeModuleSource = {three_module_source};\n{visual_core_js}\n{three_renderer_js}\n{puzzle3_app_js}\nwindow.PuzzleExport = JSON.parse("
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
                "generateSoundEffect",
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

fn export_html_from_source_with_host_mode(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
    host_mode: StandaloneHostMode,
) -> Result<String, DiagnosticReport> {
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    if document.models.len() > 1 {
        let loaded = mixed_document_loaded_game(&document).map_err(DiagnosticReport::error)?;
        let game_visuals_js = join_visuals_js(game_visuals_js, &generated_visuals_js(&loaded));
        return export_mixed_document_html(
            &document,
            loaded,
            source.to_string(),
            puzzle_path.to_string(),
            game_css.to_string(),
            game_visuals_js,
            SolverConfig::default(),
            host_mode,
        )
        .map_err(DiagnosticReport::error);
    }
    match document.single_model() {
        Some(LoadedDocumentModel::Puzzle2d { game, .. }) => {
            let game_visuals_js = join_visuals_js(game_visuals_js, &generated_visuals_js(game));
            let state = ServerState::new(
                game.clone(),
                source.to_string(),
                puzzle_path.to_string(),
                game_css.to_string(),
                game_visuals_js,
                SolverConfig::default(),
            );
            Ok(match host_mode {
                StandaloneHostMode::Export => export_html(&state),
                StandaloneHostMode::EditorPreview => export_editor_preview_html(&state),
            })
        }
        Some(LoadedDocumentModel::Puzzle3d { .. }) => {
            export_puzzle3_document_html(&document, source, puzzle_path, game_css, game_visuals_js)
                .map_err(DiagnosticReport::error)
        }
        None => Err(DiagnosticReport::error(
            "HTML export requires a single puzzle model",
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_html_file(path: impl AsRef<Path>) -> Result<String, String> {
    let puzzle_path = resolve_game_entry(path).map_err(|error| error.to_string())?;
    let source = fs::read_to_string(&puzzle_path).map_err(|error| error.to_string())?;
    let source = if source_looks_puzzle3d(&source) {
        source
    } else {
        expand_game_imports_for_file(&source, &puzzle_path).map_err(|error| error.to_string())?
    };
    let document = puzzle_lang::parse_game(&source).map_err(|error| error.to_string())?;
    let game_css =
        load_asset_css(&puzzle_path, &document.assets).map_err(|error| error.to_string())?;

    if document.models.len() > 1 {
        let loaded = mixed_document_loaded_game(&document)?;
        let game_visuals_js =
            load_game_visuals_js(&puzzle_path, &loaded).map_err(|error| error.to_string())?;
        return export_mixed_document_html(
            &document,
            loaded,
            source,
            puzzle_path.display().to_string(),
            game_css,
            game_visuals_js,
            SolverConfig::default(),
            StandaloneHostMode::Export,
        );
    }

    match document.single_model() {
        Some(LoadedDocumentModel::Puzzle2d { game, .. }) => {
            let game_visuals_js =
                load_game_visuals_js(&puzzle_path, game).map_err(|error| error.to_string())?;
            let state = ServerState::new(
                game.clone(),
                source,
                puzzle_path.display().to_string(),
                game_css,
                game_visuals_js,
                SolverConfig::default(),
            );
            Ok(export_html(&state))
        }
        Some(LoadedDocumentModel::Puzzle3d { .. }) => export_puzzle3_document_html(
            &document,
            &source,
            &puzzle_path.display().to_string(),
            &game_css,
            VISUALS_JS,
        ),
        None => Err("HTML export requires a single puzzle model".to_string()),
    }
}

pub fn export_visuals_js_from_source(
    source: &str,
    base_visuals_js: &str,
) -> Result<String, String> {
    let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
    if document.models.len() > 1 {
        let loaded = mixed_document_loaded_game(&document)?;
        return Ok(join_visuals_js(
            base_visuals_js,
            &generated_visuals_js(&loaded),
        ));
    }
    match document.single_model() {
        Some(LoadedDocumentModel::Puzzle2d { game, .. }) => Ok(join_visuals_js(
            base_visuals_js,
            &generated_visuals_js(game),
        )),
        Some(LoadedDocumentModel::Puzzle3d { .. }) => Ok(base_visuals_js.to_string()),
        None => Err("visual export requires a single puzzle model".to_string()),
    }
}
