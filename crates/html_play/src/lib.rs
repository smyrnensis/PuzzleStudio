#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

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
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use std::time::Duration;

use puzzle_core::{
    ComparisonOp, CompiledGame, Effect, GlobalUpdateOp, Guard, InputId, LayerId, ObjectId, Offset,
    Pattern, QueryKind, Rule, RuleApplication, RuleCondition, RuleId, RuleStep, ScratchPattern,
    ScratchValueMatch, State, TransitionCommand, WriteOp, transition_program,
    transition_program_outcome, transition_state,
};
use puzzle_lang::AssetKind;
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::AssetsDef;
use puzzle_lang::{
    ArrowKey, GoalCondition, GoalExpr, GoalValue, KeyTrigger, Level, LoadedDocumentModel,
    LoadedGame, MenuComponent, ResourceSelection, RuleEmission, SceneAlignXDef, SceneAlignYDef,
    SceneComponent, SceneDef, SceneEffect, SceneExpr, SceneLayoutDef, ScenePuzzleInitializer,
    SceneTextContent, SceneTransitionTrigger, SceneValue, SoundsDef, ThemeDef, VisualSpriteKind,
    parse_game2d as parse_game,
};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::{discover_game_entries, expand_game_imports_for_file, resolve_game_entry};
use puzzle_play::{GameSession, MessageEvent, SoundEvent, WaitEvent};
use puzzle_solver::{
    Puzzle3Domain, PuzzleDomain, SearchBudget, SearchOutcome, SearchStats,
    best_first_with_dead_states,
};
use puzzle3d_model::{
    Coord3, Game3, InputId3, LifecycleCommand3, ObjectId as ObjectId3, ParsedPuzzle3, Rule3,
    RuleId3, Size3, State3, WinCondition3, transition_program as transition_program3,
    transition_program_without_input,
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
const PUZZLE_WASM_JS: &str = include_str!("../../html_editor/static/wasm/puzzle_wasm.js");
#[cfg(not(target_arch = "wasm32"))]
const PUZZLE_WASM_BG: &[u8] = include_bytes!("../../html_editor/static/wasm/puzzle_wasm_bg.wasm");
const PUZZLE3_STYLE_CSS: &str = include_str!("../static/puzzle3.css");
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
const PUZZLE3_APP_JS: &str = include_str!("../static/puzzle3_app.js");
const SEEDED_SFX_JS: &str = include_str!("../../../tools/music_generator/seeded_sfx.mjs");
const SEEDED_MUSIC_JS: &str = include_str!("../../../tools/music_generator/seeded_music.mjs");

#[cfg(not(target_arch = "wasm32"))]
pub fn run_cli() {
    if let Err(error) = run_cli_with_args(env::args().skip(1)) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_cli_with_args(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    run(args).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn run(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    let config = Config::from_args(args)?;
    let source = fs::read_to_string(&config.puzzle_path)?;
    let source = if source_looks_puzzle3d(&source) {
        source
    } else {
        expand_game_imports_for_file(&source, &config.puzzle_path)?
    };

    if source_looks_puzzle3d(&source) {
        let document = puzzle_lang::parse_game(&source).map_err(AppError::Lang)?;
        let game_css = load_asset_css(&config.puzzle_path, &document.assets)?;
        let output_path = config.output_path();
        let html = if document.models.len() == 1 {
            let puzzle_path = config.puzzle_path.display().to_string();
            export_puzzle3_document_html(&document, &source, &puzzle_path, &game_css)
                .map_err(AppError::Config)?
        } else {
            let loaded = mixed_document_loaded_game(&document).map_err(AppError::Config)?;
            let game_visuals_js = load_game_visuals_js(&config.puzzle_path, &loaded)?;
            export_mixed_document_html(
                &document,
                loaded,
                source.clone(),
                config.puzzle_path.display().to_string(),
                game_css,
                game_visuals_js,
                config.solver,
            )
            .map_err(AppError::Config)?
        };
        if !config.serve {
            fs::write(&output_path, html)?;
            println!("exported {}", output_path.display());
            return Ok(());
        }
        return serve_static_html(html, &config.puzzle_path, config.port);
    }

    let loaded = parse_game(&source)?;
    let game_css = load_game_css(&config.puzzle_path, &loaded)?;
    print_warnings(&loaded);
    let game_visuals_js = load_game_visuals_js(&config.puzzle_path, &loaded)?;

    if !config.serve {
        let state = ServerState::new(
            loaded,
            source,
            config.puzzle_path.display().to_string(),
            game_css,
            game_visuals_js,
            config.solver,
        );
        let output_path = config.output_path();
        fs::write(&output_path, export_html(&state))?;
        println!("exported {}", output_path.display());
        return Ok(());
    }

    let state = Arc::new(Mutex::new(ServerState::new(
        loaded,
        source,
        config.puzzle_path.display().to_string(),
        game_css,
        game_visuals_js,
        config.solver,
    )));
    let (listener, port) = bind_listener(config.port)?;

    println!("html-play serving http://127.0.0.1:{port}");
    println!("puzzle: {}", config.puzzle_path.display());

    for stream in listener.incoming() {
        let stream = stream?;
        let state = Arc::clone(&state);
        if let Err(error) = handle_connection(stream, state) {
            eprintln!("request error: {error}");
        }
    }

    Ok(())
}

fn print_warnings(loaded: &LoadedGame) {
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct Config {
    puzzle_path: PathBuf,
    output_path: Option<PathBuf>,
    serve: bool,
    port: u16,
    solver: SolverConfig,
}

#[cfg(not(target_arch = "wasm32"))]
impl Config {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, AppError> {
        let mut puzzle_path = None;
        let mut output_path = None;
        let mut serve = false;
        let mut port = 7878;
        let mut solver = SolverConfig::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--serve" => {
                    serve = true;
                }
                "--output" | "-o" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config(format!("{arg} requires a value")));
                    };
                    output_path = Some(PathBuf::from(value));
                }
                "--port" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config("--port requires a value".to_string()));
                    };
                    serve = true;
                    port = value
                        .parse()
                        .map_err(|_| AppError::Config("port must be a u16".to_string()))?;
                }
                "--solver-depth" => {
                    solver.max_depth = parse_arg(&mut args, "--solver-depth")?;
                }
                "--solver-nodes" => {
                    solver.max_nodes = parse_arg(&mut args, "--solver-nodes")?;
                }
                "--solver-ms" => {
                    let milliseconds: u64 = parse_arg(&mut args, "--solver-ms")?;
                    solver.max_duration = Duration::from_millis(milliseconds);
                }
                "--help" | "-h" => {
                    return Err(AppError::Config(
                        "usage: html-play [path/to/game-folder-or-game.puzzle] [-o game.html] [--serve] [--port 7878] [--solver-depth 128] [--solver-nodes 1000000] [--solver-ms 5000]".to_string(),
                    ));
                }
                value => puzzle_path = Some(PathBuf::from(value)),
            }
        }

        let puzzle_path = match puzzle_path {
            Some(path) => {
                resolve_game_entry(path).map_err(|error| AppError::Config(error.to_string()))?
            }
            None => discover_default_puzzle_path()?,
        };

        Ok(Self {
            puzzle_path,
            output_path,
            serve,
            port,
            solver,
        })
    }

    fn output_path(&self) -> PathBuf {
        self.output_path.clone().unwrap_or_else(|| {
            self.puzzle_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("game.html")
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn discover_default_puzzle_path() -> Result<PathBuf, AppError> {
    let candidates =
        discover_game_entries("games").map_err(|error| AppError::Config(error.to_string()))?;
    match candidates.len() {
        0 => Err(AppError::Config(
            "no games/*/game.puzzle entries found. Pass a path: html-play <path/to/game-folder-or-game.puzzle>"
                .to_string(),
        )),
        1 => Ok(candidates[0].clone()),
        _ => Err(AppError::Config(format!(
            "multiple games/*/game.puzzle entries found. Pass one explicitly: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_game_css(puzzle_path: &Path, loaded: &LoadedGame) -> Result<String, AppError> {
    load_asset_css(puzzle_path, &loaded.assets)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_asset_css(puzzle_path: &Path, assets: &AssetsDef) -> Result<String, AppError> {
    let base_dir = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut parts = Vec::new();
    for asset in assets
        .entries
        .iter()
        .filter(|asset| asset.kind == AssetKind::Css)
    {
        let css_path = resolve_asset_path(base_dir, &asset.path)?;
        let css = fs::read_to_string(&css_path)?;
        parts.push(inline_css_urls(
            &css,
            css_path.parent().unwrap_or_else(|| Path::new(".")),
        )?);
    }
    Ok(parts.join("\n"))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_game_visuals_js(puzzle_path: &Path, loaded: &LoadedGame) -> Result<String, AppError> {
    let mut scripts = vec![asset_resolver_js(puzzle_path)?, VISUALS_JS.to_string()];
    let base_dir = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    for asset in loaded
        .assets
        .entries
        .iter()
        .filter(|asset| asset.kind == AssetKind::Script)
    {
        scripts.push(fs::read_to_string(resolve_asset_path(
            base_dir,
            &asset.path,
        )?)?);
    }
    let generated = generated_visuals_js(loaded);
    if !generated.is_empty() {
        scripts.push(generated);
    }
    Ok(scripts.join("\n"))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_asset_path(base_dir: &Path, asset_path: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(asset_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AppError::Config(format!(
            "asset path must be game-folder relative: {asset_path}"
        )));
    }
    let resolved = base_dir.join(path);
    if !resolved.exists() {
        return Err(AppError::Config(format!(
            "asset file not found: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

#[cfg(not(target_arch = "wasm32"))]
fn inline_css_urls(css: &str, base_dir: &Path) -> Result<String, AppError> {
    let mut out = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("url(") {
        let (before, after_start) = rest.split_at(start);
        out.push_str(before);
        let Some(end) = after_start.find(')') else {
            out.push_str(after_start);
            return Ok(out);
        };
        let raw = after_start[4..end].trim().trim_matches(['"', '\'']);
        if raw.starts_with("data:")
            || raw.starts_with("http:")
            || raw.starts_with("https:")
            || raw.starts_with('#')
        {
            out.push_str(&after_start[..=end]);
        } else {
            let asset_path = base_dir.join(raw);
            if asset_path.exists() {
                let mime_type = mime_type(&asset_path);
                let encoded = base64_encode(&fs::read(asset_path)?);
                out.push_str(&format!("url(\"data:{mime_type};base64,{encoded}\")"));
            } else {
                out.push_str(&after_start[..=end]);
            }
        }
        rest = &after_start[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn asset_resolver_js(puzzle_path: &Path) -> Result<String, AppError> {
    let parent = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = String::new();
    files.push('{');
    let mut first = true;
    collect_asset_resolver_entries(parent, parent, &mut files, &mut first)?;
    files.push('}');
    Ok(format!(
        "window.PuzzleAssets = {{ files: {files}, url(path) {{ return this.files[String(path || '').replaceAll('\\\\\\\\', '/')] || String(path || ''); }} }};"
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_asset_resolver_entries(
    root: &Path,
    dir: &Path,
    files: &mut String,
    first: &mut bool,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_asset_resolver_entries(root, &path, files, first)?;
            continue;
        }
        let file_name = path.file_name().and_then(|value| value.to_str());
        let extension = path.extension().and_then(|value| value.to_str());
        if !path.is_file() || file_name == Some("game.puzzle") || extension == Some("html") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let Some(name) = relative.to_str() else {
            continue;
        };
        if !*first {
            files.push(',');
        }
        *first = false;
        push_json_string(files, &name.replace('\\', "/"));
        files.push(':');
        let url = if is_text_file(&path) {
            format!(
                "data:{};charset=utf-8,{}",
                mime_type(&path),
                percent_encode(&fs::read_to_string(&path)?)
            )
        } else {
            format!(
                "data:{};base64,{}",
                mime_type(&path),
                base64_encode(&fs::read(&path)?)
            )
        };
        push_json_string(files, &url);
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
        "css" | "js" | "mjs" | "svg" | "json" | "txt" | "md"
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "css" => "text/css",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "wav" => "audio/wav",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn generated_visuals_js(loaded: &LoadedGame) -> String {
    if loaded.visuals.aliases.is_empty() && loaded.visuals.sprites.is_empty() {
        return String::new();
    }

    let mut aliases = String::new();
    aliases.push('{');
    for (index, alias) in loaded.visuals.aliases.iter().enumerate() {
        if index > 0 {
            aliases.push(',');
        }
        push_json_string(&mut aliases, &alias.object);
        aliases.push(':');
        push_json_string(&mut aliases, &alias.sprite);
    }
    aliases.push('}');

    let mut sprites = String::new();
    sprites.push('{');
    for (index, sprite) in loaded.visuals.sprites.iter().enumerate() {
        if index > 0 {
            sprites.push(',');
        }
        push_json_string(&mut sprites, &sprite.name);
        sprites.push(':');
        push_visual_sprite(&mut sprites, &sprite.kind);
    }
    sprites.push('}');

    format!(
        "(() => {{\n  const previous = window.GameVisuals || {{}};\n  const createVisuals = window.PuzzleSpriteRegistry?.create || ((config = {{}}) => ({{\n    aliases: {{ ...(config.aliases || {{}}) }},\n    sprites: {{ ...(config.sprites || {{}}) }},\n    boardClass: config.boardClass || \"\",\n    themeClass: config.themeClass || \"\",\n    editorPuzzle: {{ ...(config.editorPuzzle || {{}}) }},\n    autoAdvanceDelayMs: config.autoAdvanceDelayMs,\n  }}));\n  window.GameVisuals = createVisuals({{\n    ...previous,\n    aliases: {{ ...(previous.aliases || {{}}), ...{aliases} }},\n    sprites: {{ ...(previous.sprites || {{}}), ...{sprites} }},\n  }});\n}})();"
    )
}

fn push_visual_sprite(out: &mut String, kind: &VisualSpriteKind) {
    match kind {
        VisualSpriteKind::Solid(color) => {
            out.push_str("{\"colors\":{\"0\":");
            push_json_string(out, color);
            out.push_str("},\"pattern\":[\"0\"]}");
        }
        VisualSpriteKind::Image { source } => {
            out.push_str("{\"source\":");
            push_json_string(out, source);
            out.push('}');
        }
        VisualSpriteKind::Ascii { pattern, colors } => {
            out.push_str("{\"colors\":{");
            for (index, color) in colors.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_string(out, &color.token.to_string());
                out.push(':');
                push_json_string(out, &color.color);
            }
            out.push_str("},\"pattern\":[");
            for (index, row) in pattern.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_string(out, row);
            }
            out.push_str("]}");
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SolverConfig {
    max_depth: u32,
    max_nodes: usize,
    max_duration: Duration,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 1_000_000,
            max_duration: Duration::from_secs(5),
        }
    }
}

impl SolverConfig {
    fn budget(self) -> SearchBudget {
        SearchBudget::bounded(self.max_depth, self.max_nodes, self.max_duration)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_arg<T>(args: &mut impl Iterator<Item = String>, name: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    let Some(value) = args.next() else {
        return Err(AppError::Config(format!("{name} requires a value")));
    };
    value
        .parse()
        .map_err(|_| AppError::Config(format!("{name} has an invalid value")))
}

struct ServerState {
    loaded: LoadedGame,
    session: GameSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
}

impl ServerState {
    fn new(
        loaded: LoadedGame,
        source: String,
        puzzle_path: String,
        game_css: String,
        game_visuals_js: String,
        solver: SolverConfig,
    ) -> Self {
        let session = GameSession::new(&loaded);
        Self {
            loaded,
            session,
            source,
            puzzle_path,
            game_css,
            game_visuals_js,
            solver,
        }
    }

    fn snapshot_json(&mut self) -> String {
        let sound_events = self.session.take_sound_events();
        let message_events = self.session.take_message_events();
        let wait_events = self.session.take_wait_events();
        let mut out = String::new();
        out.push('{');
        push_game_context(&mut out, &self.loaded);
        out.push(',');
        push_export_sounds(&mut out, &self.loaded.sounds);
        out.push(',');
        push_export_theme(&mut out, &self.loaded.theme);
        out.push(',');
        push_json_number(&mut out, "defaultWaitMs", self.loaded.default_wait_ms);
        out.push(',');
        push_json_number(&mut out, "defaultAgainMs", 120);
        out.push(',');
        push_sound_events(&mut out, &sound_events);
        out.push(',');
        push_message_events(&mut out, &message_events);
        out.push(',');
        push_wait_events(&mut out, &wait_events);
        out.push(',');
        push_level_context(&mut out, &self.loaded, self.session.active_level_index());
        out.push(',');
        out.push_str("\"levelIndex\":");
        if let Some(level_index) = self.session.active_level_index() {
            out.push_str(&(level_index as u64).to_string());
        } else {
            out.push_str("null");
        }
        out.push(',');
        push_json_number(&mut out, "levelCount", self.loaded.levels.len() as u64);
        out.push(',');
        push_json_pair(&mut out, "scene", self.session.scene());
        out.push(',');
        push_json_pair(&mut out, "currentScene", self.session.scene());
        out.push(',');
        push_json_pair(&mut out, "focusedScreen", self.session.focused_scene());
        out.push(',');
        push_json_pair(&mut out, "focusedScene", self.session.focused_scene());
        out.push(',');
        push_visible_scenes_compat(&mut out, self.session.visible_scenes());
        out.push(',');
        push_visible_scenes(&mut out, self.session.visible_scenes());
        out.push(',');
        push_session_state(&mut out, self.session.session_values());
        out.push(',');
        push_scene_state_compat(&mut out, self.session.scene_state());
        out.push(',');
        push_scene_state(&mut out, self.session.scene_state());
        out.push(',');
        push_scene_puzzles_compat(&mut out, self.session.scene_state());
        out.push(',');
        push_scene_puzzles(&mut out, self.session.scene_state());
        out.push(',');
        push_scene_puzzle_state(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_json_number(
            &mut out,
            "selectedLevelIndex",
            self.session.selected_level_index() as u64,
        );
        out.push(',');
        push_json_bool(&mut out, "canUndo", self.session.can_undo());
        out.push(',');
        push_json_bool(&mut out, "canRedo", self.session.can_redo());
        out.push(',');
        let scene_state = focused_scene_state(&self.loaded, &self.session);
        let focused_scene = self
            .loaded
            .scenes
            .iter()
            .find(|scene| scene.name == self.session.focused_scene());
        if let Some(scene_state) = scene_state {
            push_scene(
                &mut out,
                &self.loaded,
                scene_state,
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else if self.loaded.scenes.is_empty() {
            push_scene(
                &mut out,
                &self.loaded,
                self.session.state(),
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("\"scene\":null");
        }
        out.push(',');
        push_scene_layers(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_inputs(&mut out, &self.loaded);
        out.push(',');
        push_levels(&mut out, &self.loaded, self.session.cleared_levels());
        out.push(',');
        push_menus(&mut out, &self.loaded);
        out.push(',');
        push_scenes(&mut out, "scenes", &self.loaded);
        out.push(',');
        push_scenes(&mut out, "screens", &self.loaded);
        out.push('}');
        out
    }

    fn scene_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        let scene_state = focused_scene_state(&self.loaded, &self.session);
        let focused_scene = self
            .loaded
            .scenes
            .iter()
            .find(|scene| scene.name == self.session.focused_scene());
        if let Some(scene_state) = scene_state {
            push_scene(
                &mut out,
                &self.loaded,
                scene_state,
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else if self.loaded.scenes.is_empty() {
            push_scene(
                &mut out,
                &self.loaded,
                self.session.state(),
                Some(self.session.current_level(&self.loaded)),
                focused_scene.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("\"scene\":null");
        }
        out.push('}');
        out
    }

    fn apply_input_name(&mut self, input_name: &str) -> Result<(), AppError> {
        let input = input_id_by_name(&self.loaded, input_name)
            .ok_or_else(|| AppError::Config(format!("unknown input: {input_name}")))?;
        self.session.apply_input(&self.loaded, input)?;
        Ok(())
    }

    fn apply_command_name(&mut self, command_name: &str) -> Result<(), AppError> {
        self.session.apply_command(&self.loaded, command_name)?;
        Ok(())
    }

    fn solve_json(&self) -> Result<String, AppError> {
        let response =
            solve_current_state(&self.loaded, self.session.state().clone(), self.solver)?;
        let mut out = String::new();
        push_solution_response(&mut out, &self.loaded, &response);
        Ok(out)
    }
}

#[derive(Clone, Debug)]
struct SolutionStep {
    index: usize,
    input: Option<InputId>,
    state: State,
}

#[derive(Clone, Debug)]
enum SolutionResponse {
    Solved {
        depth: u32,
        moves: Vec<InputId>,
        steps: Vec<SolutionStep>,
    },
    Exhausted(SearchStats),
    BudgetExceeded(SearchStats),
    Failed {
        depth: u32,
        error: String,
    },
}

#[derive(Clone, Debug)]
struct SolutionStep3 {
    index: usize,
    input: Option<InputId3>,
    state: State3,
    completed: bool,
}

#[derive(Clone, Debug)]
enum SolutionResponse3 {
    Solved {
        depth: u32,
        moves: Vec<InputId3>,
        steps: Vec<SolutionStep3>,
    },
    Exhausted(SearchStats),
    BudgetExceeded(SearchStats),
    Failed {
        depth: u32,
        error: String,
    },
}

fn solve_current_state(
    loaded: &LoadedGame,
    initial: State,
    solver: SolverConfig,
) -> Result<SolutionResponse, AppError> {
    solve_current_state_with_budget(loaded, initial, solver.budget())
}

fn solve_current_state_with_budget(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
) -> Result<SolutionResponse, AppError> {
    let inputs = solver_inputs(loaded);
    if inputs.is_empty() {
        return Err(AppError::Config("no model inputs available".to_string()));
    }

    let game = Arc::new(loaded.game.clone());
    let goal_game = loaded.clone();
    let mut domain = PuzzleDomain::new(game.clone(), inputs, move |state: &State| {
        goal_game.is_goal_complete(state)
    });
    let solver_initial = initial.without_visual_objects(domain.game());
    let score_game = loaded.clone();
    let lose_game = loaded.clone();
    let outcome = best_first_with_dead_states(
        &mut domain,
        solver_initial,
        budget,
        move |state| goal_score(&score_game, state),
        move |state| lose_game.is_lose_complete(state),
    );

    let response = match outcome {
        SearchOutcome::Solved(witness) => {
            let depth = witness.depth;
            let solution_inputs = witness.actions;
            SolutionResponse::Solved {
                depth,
                steps: solution_steps(&loaded.game, initial, &solution_inputs)?,
                moves: solution_inputs,
            }
        }
        SearchOutcome::Exhausted(stats) => SolutionResponse::Exhausted(stats),
        SearchOutcome::BudgetExceeded(stats) => SolutionResponse::BudgetExceeded(stats),
        SearchOutcome::Failed(failure) => SolutionResponse::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
        },
    };
    Ok(response)
}

fn solution_steps(
    game: &puzzle_core::CompiledGame,
    mut state: State,
    inputs: &[InputId],
) -> Result<Vec<SolutionStep>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep {
        index: 0,
        input: None,
        state: state.clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_state(game, &state, *input)?;
        steps.push(SolutionStep {
            index: index + 1,
            input: Some(*input),
            state: state.clone(),
        });
    }

    Ok(steps)
}

fn solve_current_state3_with_budget(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
) -> Result<SolutionResponse3, AppError> {
    let inputs = solver_inputs3(&parsed.game);
    if inputs.is_empty() {
        return Err(AppError::Config("no 3D model inputs available".to_string()));
    }
    let win_condition = parsed
        .win_condition
        .clone()
        .ok_or_else(|| AppError::Config("3D solver requires win_conditions".to_string()))?;

    let game = Arc::new(parsed.game.clone());
    let rules = parsed.rules.clone();
    let goal_game = Arc::clone(&game);
    let mut domain = Puzzle3Domain::new(
        Arc::clone(&game),
        rules.clone(),
        inputs,
        move |state: &State3| win_condition.is_met(&goal_game, state),
    );
    let outcome =
        best_first_with_dead_states(&mut domain, initial.clone(), budget, |_| 0, |_| false);

    let response = match outcome {
        SearchOutcome::Solved(witness) => {
            let depth = witness.depth;
            let solution_inputs = witness.actions;
            SolutionResponse3::Solved {
                depth,
                steps: solution_steps3(
                    &game,
                    &rules,
                    parsed.win_condition.as_ref(),
                    initial,
                    &solution_inputs,
                )?,
                moves: solution_inputs,
            }
        }
        SearchOutcome::Exhausted(stats) => SolutionResponse3::Exhausted(stats),
        SearchOutcome::BudgetExceeded(stats) => SolutionResponse3::BudgetExceeded(stats),
        SearchOutcome::Failed(failure) => SolutionResponse3::Failed {
            depth: failure.depth,
            error: format!("{:?}", failure.error),
        },
    };
    Ok(response)
}

fn solution_steps3(
    game: &Game3,
    rules: &[Rule3],
    win_condition: Option<&WinCondition3>,
    mut state: State3,
    inputs: &[InputId3],
) -> Result<Vec<SolutionStep3>, AppError> {
    let mut steps = Vec::with_capacity(inputs.len() + 1);
    steps.push(SolutionStep3 {
        index: 0,
        input: None,
        completed: win_condition.is_some_and(|condition| condition.is_met(game, &state)),
        state: state.clone(),
    });

    for (index, input) in inputs.iter().enumerate() {
        state = transition_program3(game, &state, rules, *input)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        steps.push(SolutionStep3 {
            index: index + 1,
            input: Some(*input),
            completed: win_condition.is_some_and(|condition| condition.is_met(game, &state)),
            state: state.clone(),
        });
    }

    Ok(steps)
}

fn goal_score(loaded: &LoadedGame, state: &State) -> i64 {
    loaded
        .goal
        .as_ref()
        .map(|goal| goal_expr_score(&loaded.game, state, &goal.expr))
        .unwrap_or(0)
}

fn goal_expr_score(game: &CompiledGame, state: &State, expr: &GoalExpr) -> i64 {
    match expr {
        GoalExpr::All(exprs) => exprs
            .iter()
            .map(|expr| goal_expr_score(game, state, expr))
            .sum(),
        GoalExpr::Any(exprs) => exprs
            .iter()
            .map(|expr| goal_expr_score(game, state, expr))
            .min()
            .unwrap_or(0),
        GoalExpr::Clause(clause) => {
            let value = goal_value(game, state, &clause.value);
            if compare_i64(value, clause.op, clause.expected) {
                0
            } else {
                goal_clause_score(game, state, &clause.value, value, clause.expected)
            }
        }
    }
}

fn goal_clause_score(
    game: &CompiledGame,
    state: &State,
    value: &GoalValue,
    current: i64,
    expected: i64,
) -> i64 {
    match value {
        GoalValue::Global(_) => current.abs_diff(expected) as i64,
        GoalValue::Query(query) => game
            .query(*query)
            .map(|query| query_kind_score(game, state, &query.kind, current, expected))
            .unwrap_or_else(|| current.abs_diff(expected) as i64),
        GoalValue::QueryValue(kind) => query_kind_score(game, state, kind, current, expected),
    }
}

fn query_kind_score(
    game: &CompiledGame,
    state: &State,
    kind: &QueryKind,
    current: i64,
    expected: i64,
) -> i64 {
    match kind {
        QueryKind::CountMatches(patterns) if expected == 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        QueryKind::NoneMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        QueryKind::ExistsMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .min()
            .unwrap_or(1),
        _ => current.abs_diff(expected) as i64,
    }
}

fn pattern_distance_score(game: &CompiledGame, state: &State, pattern: &Pattern) -> i64 {
    let Some(component) = pattern.components.first() else {
        return 0;
    };
    if pattern.components.len() != 1 || component.cells.len() != 1 {
        return i64::from(puzzle_core::count_pattern_matches(game, state, pattern));
    }
    let cell = &component.cells[0];
    if cell.require_objects.is_empty() || cell.forbid_objects.is_empty() {
        return i64::from(puzzle_core::count_pattern_matches(game, state, pattern));
    }

    let targets = object_positions(game, state, &cell.forbid_objects);
    let fallback = i64::from(state.width) + i64::from(state.height);
    let mut score = 0_i64;
    for y in 0..state.height {
        for x in 0..state.width {
            if !has_all_objects(game, state, x, y, &cell.require_objects) {
                continue;
            }
            if has_all_objects(game, state, x, y, &cell.forbid_objects) {
                continue;
            }
            let distance = targets
                .iter()
                .map(|(tx, ty)| manhattan(x, y, *tx, *ty))
                .min()
                .unwrap_or(fallback);
            score += distance.max(1);
        }
    }
    score
}

fn object_positions(game: &CompiledGame, state: &State, objects: &[ObjectId]) -> Vec<(u16, u16)> {
    if let [object] = objects {
        return state
            .object_positions(*object)
            .iter()
            .filter_map(|slot| state.slot_position(*slot))
            .collect();
    }

    let mut positions = Vec::new();
    for y in 0..state.height {
        for x in 0..state.width {
            if has_all_objects(game, state, x, y, objects) {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn has_all_objects(
    game: &CompiledGame,
    state: &State,
    x: u16,
    y: u16,
    objects: &[ObjectId],
) -> bool {
    objects
        .iter()
        .all(|object| state.has_object(game, x, y, *object))
}

fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> i64 {
    i64::from(ax.abs_diff(bx)) + i64::from(ay.abs_diff(by))
}

fn compare_i64(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}

fn goal_value(game: &CompiledGame, state: &State, value: &GoalValue) -> i64 {
    match value {
        GoalValue::Global(global) => state.global_value(*global).unwrap_or(0),
        GoalValue::Query(query) => game
            .query(*query)
            .map(|query| goal_query_kind(game, state, &query.kind))
            .unwrap_or(0),
        GoalValue::QueryValue(kind) => goal_query_kind(game, state, kind),
    }
}

fn goal_query_kind(game: &CompiledGame, state: &State, kind: &QueryKind) -> i64 {
    match kind {
        QueryKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        QueryKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        QueryKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        QueryKind::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| i64::from(puzzle_core::count_pattern_matches(game, state, pattern)))
            .sum(),
        QueryKind::ExistsMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                1
            } else {
                0
            }
        }
        QueryKind::NoneMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                0
            } else {
                1
            }
        }
        QueryKind::CountInputMatches(_)
        | QueryKind::ExistsInputMatches(_)
        | QueryKind::NoneInputMatches(_) => 0,
    }
}

fn export_html(state: &ServerState) -> String {
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
    let embedded_wasm_js = embedded_standalone_wasm_script();
    let sound_tools_js = escape_script(&sound_tools_js());
    let app_js = escape_script(APP_JS);

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
                "<script>\nwindow.PuzzleExport = JSON.parse(\"{data}\");\n{embedded_wasm_js}\n</script>\n<script>\n{renderer_js}\n</script>\n<script>\n{standalone_js}\n</script>"
            ),
        )
        .replace(
            r#"<script src="/app.js"></script>"#,
            &format!("<script>\n{app_js}\n</script>"),
        )
        .replace("<body>", &format!("<body{body_theme_attributes}>"))
}

fn embedded_standalone_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let module_source = escape_script_json(PUZZLE_WASM_JS);
        let wasm_base64 = base64_encode(PUZZLE_WASM_BG);
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
            await module.default(base64ToUint8Array(embedded.wasmBase64));
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
    {
        String::new()
    }
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

fn export_puzzle3_html(
    fixture_json: &str,
    source: &str,
    puzzle_path: &str,
    game_css: &str,
) -> String {
    let fixture_json = escape_script_json(fixture_json);
    let source_json = escape_script_json(source);
    let puzzle_path_json = escape_script_json(puzzle_path);
    let puzzle3_style_css = escape_style(PUZZLE3_STYLE_CSS);
    let game_css = escape_style(game_css);
    let embedded_wasm_js = embedded_standalone_wasm_script();
    let puzzle3_visual_core_js = escape_script(PUZZLE3_VISUAL_CORE_JS);
    let puzzle3_app_js = escape_script(PUZZLE3_APP_JS);

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>PuzzleStudio HTML Export</title>
    <style>
{puzzle3_style_css}
    </style>
    <style>
{game_css}
    </style>
  </head>
  <body class="theme-clean">
    <main id="screenView" class="scene">
      <div class="puzzle3-component">
        <canvas id="view" width="960" height="640" aria-label="Puzzle3 component"></canvas>
      </div>
    </main>
    <script>
window.Puzzle3DFixture = JSON.parse("{fixture_json}");
window.Puzzle3DSource = JSON.parse("{source_json}");
window.Puzzle3DPath = JSON.parse("{puzzle_path_json}");
{embedded_wasm_js}
    </script>
    <script>
{puzzle3_visual_core_js}
    </script>
    <script>
{puzzle3_app_js}
    </script>
  </body>
</html>"#
    )
}

fn export_puzzle3_document_html(
    document: &puzzle_lang::LoadedDocument,
    source: &str,
    puzzle_path: &str,
    game_css: &str,
) -> Result<String, String> {
    let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(document)
        .map_err(|error| error.to_string())?;
    Ok(export_puzzle3_html(
        &fixture_json,
        source,
        puzzle_path,
        game_css,
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
) -> Result<String, String> {
    let fixture_json = mixed_document_puzzle3_fixture_json(document)?;
    let puzzle3_source = source.clone();
    let puzzle3_path = puzzle_path.clone();
    let state = ServerState::new(
        loaded,
        source,
        puzzle_path,
        game_css,
        game_visuals_js,
        solver,
    );
    Ok(inject_puzzle3_frame_assets(
        export_html(&state),
        &fixture_json,
        &puzzle3_source,
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
    let puzzle3_style_css = escape_style(PUZZLE3_STYLE_CSS);
    let mut assets = String::new();
    assets.push('{');
    push_json_string(&mut assets, "styleCss");
    assets.push(':');
    push_json_string(&mut assets, PUZZLE3_STYLE_CSS);
    assets.push(',');
    push_json_string(&mut assets, "visualCoreJs");
    assets.push(':');
    push_json_string(&mut assets, PUZZLE3_VISUAL_CORE_JS);
    assets.push(',');
    push_json_string(&mut assets, "appJs");
    assets.push(':');
    push_json_string(&mut assets, PUZZLE3_APP_JS);
    assets.push(',');
    push_json_string(&mut assets, "embeddedWasmJs");
    assets.push(':');
    push_json_string(&mut assets, &embedded_standalone_wasm_script());
    assets.push(',');
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
    let html = html.replace(
        "</head>",
        &format!(
            "<style>\n{puzzle3_style_css}\n.puzzle3-frame {{ border: 0; display: block; inline-size: 100%; block-size: 100%; background: transparent; }}\n</style>\n</head>"
        ),
    );
    html.replace(
        "window.PuzzleExport = JSON.parse(",
        &format!(
            "window.Puzzle3DFrameFixture = JSON.parse(\"{fixture_json}\");\nwindow.Puzzle3DFrameAssets = {assets};\nwindow.PuzzleExport = JSON.parse("
        ),
    )
}

fn sound_tools_js() -> String {
    fn expose_module(source: &str, exports: &[&str]) -> String {
        let body = source
            .replace("export const ", "const ")
            .replace("export function ", "function ");
        format!("{body}\nreturn {{{}}};", exports.join(","))
    }

    format!(
        "(() => {{
  const sfx = (() => {{
{}
  }})();
  const music = (() => {{
{}
  }})();
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
        expose_module(
            SEEDED_MUSIC_JS,
            &["createPlayer", "generateSong", "randomPreset"],
        ),
    )
}

pub fn export_html_from_source(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, String> {
    let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
    if document.models.len() > 1 {
        let loaded = mixed_document_loaded_game(&document)?;
        let game_visuals_js = join_visuals_js(game_visuals_js, &generated_visuals_js(&loaded));
        return export_mixed_document_html(
            &document,
            loaded,
            source.to_string(),
            puzzle_path.to_string(),
            game_css.to_string(),
            game_visuals_js,
            SolverConfig::default(),
        );
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
            Ok(export_html(&state))
        }
        Some(LoadedDocumentModel::Puzzle3d { .. }) => {
            export_puzzle3_document_html(&document, source, puzzle_path, game_css)
        }
        None => Err("HTML export requires a single puzzle model".to_string()),
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

pub struct CoreRuntimeBridge {
    loaded: LoadedGame,
}

impl CoreRuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        Ok(Self {
            loaded: parse_game(source).map_err(|error| error.to_string())?,
        })
    }

    pub fn transition_program_outcome_json(
        &self,
        program_key: &str,
        level_index: i32,
        state_json: &str,
        input: u16,
    ) -> Result<String, String> {
        transition_program_outcome_json_inner(
            &self.loaded,
            program_key,
            level_index,
            state_json,
            InputId(input),
        )
        .map_err(|error| error.to_string())
    }
}

pub struct Puzzle3RuntimeBridge {
    parsed: ParsedPuzzle3,
}

impl Puzzle3RuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        if let Ok(parsed) = puzzle3d_model::parse_puzzle3d(source) {
            return Ok(Self { parsed });
        }
        let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
        let parsed = document
            .models
            .iter()
            .find_map(|model| match model {
                LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle.clone()),
                LoadedDocumentModel::Puzzle2d { .. } => None,
            })
            .ok_or_else(|| "3D runtime source does not contain a puzzle3 model".to_string())?;
        Ok(Self { parsed })
    }

    pub fn transition_program_outcome_json(
        &self,
        program_key: &str,
        state_json: &str,
        input: u16,
    ) -> Result<String, String> {
        transition_program3_outcome_json_inner(
            &self.parsed,
            program_key,
            state_json,
            InputId3(input),
        )
        .map_err(|error| error.to_string())
    }

    pub fn is_complete_json(&self, state_json: &str) -> Result<bool, String> {
        let state =
            state3_from_json(&self.parsed.game, state_json).map_err(|error| error.to_string())?;
        Ok(self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, &state)))
    }
}

pub fn transition_program_outcome_json_from_source(
    source: &str,
    program_key: &str,
    level_index: i32,
    state_json: &str,
    input: u16,
) -> Result<String, String> {
    let loaded = parse_game(source).map_err(|error| error.to_string())?;
    transition_program_outcome_json_inner(
        &loaded,
        program_key,
        level_index,
        state_json,
        InputId(input),
    )
    .map_err(|error| error.to_string())
}

fn transition_program3_outcome_json_inner(
    parsed: &ParsedPuzzle3,
    program_key: &str,
    state_json: &str,
    input: InputId3,
) -> Result<String, AppError> {
    let state = state3_from_json(&parsed.game, state_json)?;
    let next_state = match program_key {
        "main" => transition_program3(&parsed.game, &state, &parsed.rules, input),
        "level_start" => {
            transition_program_without_input(&parsed.game, &state, &parsed.lifecycle.on_level_start)
        }
        other => {
            return Err(AppError::Config(format!(
                "unknown 3D transition program selector: {other}"
            )));
        }
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    let completed = parsed
        .win_condition
        .as_ref()
        .is_some_and(|condition| condition.is_met(&parsed.game, &next_state));
    let mut out = String::new();
    out.push('{');
    out.push_str("\"state\":");
    push_state3_data(&mut out, &next_state);
    out.push(',');
    push_json_bool(&mut out, "completed", completed);
    out.push_str(",\"commands\":[]}");
    Ok(out)
}

fn transition_program_outcome_json_inner(
    loaded: &LoadedGame,
    program_key: &str,
    level_index: i32,
    state_json: &str,
    input: InputId,
) -> Result<String, AppError> {
    let state = state_from_json(loaded, state_json)?;
    let program = selected_rule_program(loaded, program_key, level_index)?;
    let outcome = transition_program_outcome(&loaded.game, program, &state, input)?;
    let mut out = String::new();
    push_transition_outcome_json(
        &mut out,
        &outcome.next_state,
        outcome.cancelled,
        &outcome.commands,
        &outcome.fired_rules,
    );
    Ok(out)
}

fn selected_rule_program<'a>(
    loaded: &'a LoadedGame,
    program_key: &str,
    level_index: i32,
) -> Result<&'a [RuleStep], AppError> {
    match program_key {
        "main" | "run_rules_on_level_start" => Ok(loaded.game.program()),
        "level_start" => Ok(loaded.level_start_program.as_deref().unwrap_or(&[])),
        "display_level_start" => Ok(loaded.display_level_start_program.as_deref().unwrap_or(&[])),
        "level_clear" => Ok(loaded.level_clear_program.as_deref().unwrap_or(&[])),
        "display_level_clear" => Ok(loaded.display_level_clear_program.as_deref().unwrap_or(&[])),
        "display" => Ok(loaded.display_program.as_deref().unwrap_or(&[])),
        "level_start_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_start_local requires a level index".to_string())
            })?;
            Ok(loaded
                .levels
                .get(index)
                .and_then(|level| level.level_start_program.as_deref())
                .unwrap_or(&[]))
        }
        "level_clear_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_clear_local requires a level index".to_string())
            })?;
            Ok(loaded
                .levels
                .get(index)
                .and_then(|level| level.level_clear_program.as_deref())
                .unwrap_or(&[]))
        }
        other => Err(AppError::Config(format!(
            "unknown transition program selector: {other}"
        ))),
    }
}

fn push_transition_outcome_json(
    out: &mut String,
    state: &State,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
) {
    out.push('{');
    out.push_str("\"state\":");
    push_state_data(out, state);
    out.push(',');
    push_json_bool(out, "cancelled", cancelled);
    out.push(',');
    out.push_str("\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(
            out,
            "kind",
            match command {
                TransitionCommand::Win => "win",
                TransitionCommand::Restart => "restart",
                TransitionCommand::NextLevel => "next_level",
                TransitionCommand::Again => "again",
            },
        );
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"firedRules\":[");
    for (index, rule) in fired_rules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn source_looks_puzzle3d(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
        matches!(
            tokens.as_slice(),
            ["puzzle3", ..] | ["levels3", ..] | ["sprites3", ..]
        )
    })
}

pub fn solve_state_json_from_source(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, String> {
    solve_state_json_from_source_inner(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes,
        max_ms,
    )
    .map_err(|error| error.to_string())
}

pub fn solve_state_json_from_source_with_progress(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
    _progress_interval_ms: u64,
    mut on_progress: impl FnMut(String),
) -> Result<String, String> {
    on_progress("{\"phase\":\"started\"}".to_string());
    let result = solve_state_json_from_source(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes,
        max_ms,
    );
    on_progress("{\"phase\":\"finished\"}".to_string());
    result
}

fn solve_state_json_from_source_inner(
    source: &str,
    _puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    if source_looks_puzzle3d(source) || state_json.contains("\"kind\":\"puzzle3d\"") {
        return solve_state3_json_from_source_inner(
            source, state_json, max_depth, max_nodes, max_ms,
        );
    }

    let loaded = parse_game(source)?;
    let state = state_from_json(&loaded, state_json)?;
    let state = match level_index_from_state_json(&loaded, state_json) {
        Some(level_index) => materialize_level_start_state(&loaded, state, level_index)?,
        None => state,
    };
    let solver = SolverConfig {
        max_depth,
        max_nodes,
        max_duration: if max_ms > 0 {
            Duration::from_millis(max_ms)
        } else {
            Duration::from_secs(24 * 60 * 60)
        },
    };
    let budget = if max_ms > 0 {
        solver.budget()
    } else {
        SearchBudget {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
            max_frontier: None,
            max_duration: None,
        }
    };
    let response = solve_current_state_with_budget(&loaded, state, budget)?;
    let mut out = String::new();
    push_solution_response(&mut out, &loaded, &response);
    Ok(out)
}

fn solve_state3_json_from_source_inner(
    source: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    let parsed = puzzle3d_model::parse_puzzle3d(source)
        .map_err(|error| AppError::Config(format!("{error:?}")))?;
    let state = state3_from_json(&parsed.game, state_json)?;
    let state = if level_index_from_state3_json(&parsed, state_json).is_some() {
        materialize_level_start_state3(&parsed, state)?
    } else {
        state
    };
    let budget = if max_ms > 0 {
        SearchBudget::bounded(max_depth, max_nodes, Duration::from_millis(max_ms))
    } else {
        SearchBudget {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
            max_frontier: None,
            max_duration: None,
        }
    };
    let response = solve_current_state3_with_budget(&parsed, state, budget)?;
    let mut out = String::new();
    push_solution_response3(&mut out, &parsed, &response);
    Ok(out)
}

fn state_from_json(loaded: &LoadedGame, state_json: &str) -> Result<State, AppError> {
    let width = json_u64_field(state_json, "width")
        .ok_or_else(|| AppError::Config("solver state missing width".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state width out of range".to_string()))?;
    let height = json_u64_field(state_json, "height")
        .ok_or_else(|| AppError::Config("solver state missing height".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state height out of range".to_string()))?;
    let layer_count = json_u64_field(state_json, "layerCount")
        .ok_or_else(|| AppError::Config("solver state missing layerCount".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state layerCount out of range".to_string()))?;
    let slots = json_u64_array_field(state_json, "slots")
        .ok_or_else(|| AppError::Config("solver state missing slots".to_string()))?;
    let globals = json_i64_array_field(state_json, "globals").unwrap_or_default();
    let fired_rules = json_u64_array_field(state_json, "levelFiredRules").unwrap_or_default();
    let expected_slots = usize::from(width) * usize::from(height) * usize::from(layer_count);
    if slots.len() != expected_slots {
        return Err(AppError::Config(format!(
            "solver state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        )));
    }

    let mut state = State::empty_with_globals(
        width,
        height,
        layer_count,
        loaded.game.object_count(),
        globals,
    )
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    for (index, object) in slots.into_iter().enumerate() {
        if object == 0 {
            continue;
        }
        let object: u16 = object
            .try_into()
            .map_err(|_| AppError::Config("solver state object id out of range".to_string()))?;
        let layer = index % usize::from(layer_count);
        let cell = index / usize::from(layer_count);
        let x = (cell % usize::from(width)) as u16;
        let y = (cell / usize::from(width)) as u16;
        let expected_layer = loaded
            .game
            .object_layer(ObjectId(object))
            .ok_or_else(|| AppError::Config(format!("solver state unknown object id {object}")))?;
        if usize::from(expected_layer.0) != layer {
            return Err(AppError::Config(format!(
                "solver state object {object} is in layer {layer}, expected {}",
                expected_layer.0
            )));
        }
        state
            .place_object(&loaded.game, x, y, ObjectId(object))
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
    }
    for rule in fired_rules {
        let rule: u16 = rule
            .try_into()
            .map_err(|_| AppError::Config("solver state rule id out of range".to_string()))?;
        state.mark_level_rule_fired(RuleId(rule));
    }
    Ok(state)
}

fn level_index_from_state_json(loaded: &LoadedGame, state_json: &str) -> Option<usize> {
    let index = usize::try_from(json_u64_field(state_json, "levelIndex")?).ok()?;
    (index < loaded.levels.len()).then_some(index)
}

fn materialize_level_start_state(
    loaded: &LoadedGame,
    state: State,
    level_index: usize,
) -> Result<State, AppError> {
    let mut state = state;
    let mut cancelled = false;
    if let Some(program) = loaded.level_start_program.as_deref() {
        let outcome = transition_program_outcome(&loaded.game, program, &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    } else if loaded.run_rules_on_level_start {
        let outcome =
            transition_program_outcome(&loaded.game, loaded.game.program(), &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    }
    if !cancelled {
        if let Some(program) = loaded
            .levels
            .get(level_index)
            .and_then(|level| level.level_start_program.as_deref())
        {
            let outcome = transition_program_outcome(&loaded.game, program, &state, InputId(0))?;
            state = outcome.next_state;
        }
    }
    Ok(state)
}

fn state3_from_json(game: &Game3, state_json: &str) -> Result<State3, AppError> {
    let width = json_u64_field(state_json, "width")
        .ok_or_else(|| AppError::Config("3D solver state missing width".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state width out of range".to_string()))?;
    let depth = json_u64_field(state_json, "depth")
        .ok_or_else(|| AppError::Config("3D solver state missing depth".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state depth out of range".to_string()))?;
    let height = json_u64_field(state_json, "height")
        .ok_or_else(|| AppError::Config("3D solver state missing height".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state height out of range".to_string()))?;
    let layer_count = json_u64_field(state_json, "layerCount")
        .map(u16::try_from)
        .transpose()
        .map_err(|_| AppError::Config("3D solver state layerCount out of range".to_string()))?
        .unwrap_or(game.layer_count);
    if layer_count != game.layer_count {
        return Err(AppError::Config(format!(
            "3D solver state layerCount mismatch: expected {}, got {layer_count}",
            game.layer_count
        )));
    }
    let slots = json_u64_array_field(state_json, "slots")
        .ok_or_else(|| AppError::Config("3D solver state missing slots".to_string()))?;
    let fired_rules = json_u64_array_field(state_json, "levelFiredRules").unwrap_or_default();
    let expected_slots = usize::from(width)
        .checked_mul(usize::from(depth))
        .and_then(|count| count.checked_mul(usize::from(height)))
        .and_then(|count| count.checked_mul(usize::from(layer_count)))
        .ok_or_else(|| AppError::Config("3D solver state dimensions are too large".to_string()))?;
    if slots.len() != expected_slots {
        return Err(AppError::Config(format!(
            "3D solver state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        )));
    }

    let mut state = State3::empty(Size3::new(width, depth, height), layer_count)
        .map_err(|error| AppError::Config(format!("{error:?}")))?;
    for (index, object) in slots.into_iter().enumerate() {
        if object == 0 {
            continue;
        }
        let object: u16 = object
            .try_into()
            .map_err(|_| AppError::Config("3D solver state object id out of range".to_string()))?;
        let layer = index % usize::from(layer_count);
        let cell = index / usize::from(layer_count);
        let x = (cell % usize::from(width)) as u16;
        let yz = cell / usize::from(width);
        let y = (yz % usize::from(depth)) as u16;
        let z = (yz / usize::from(depth)) as u16;
        let object = ObjectId3(object);
        let expected_layer = game.object_layer(object).ok_or_else(|| {
            AppError::Config(format!("3D solver state unknown object id {}", object.0))
        })?;
        if usize::from(expected_layer.0) != layer {
            return Err(AppError::Config(format!(
                "3D solver state object {} is in layer {layer}, expected {}",
                object.0, expected_layer.0
            )));
        }
        state
            .place_object(game, Coord3 { x, y, z }, object)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
    }
    for rule in fired_rules {
        let rule: u16 = rule
            .try_into()
            .map_err(|_| AppError::Config("3D solver state rule id out of range".to_string()))?;
        state.mark_level_rule_fired(RuleId3(rule));
    }
    Ok(state)
}

fn level_index_from_state3_json(parsed: &ParsedPuzzle3, state_json: &str) -> Option<usize> {
    let index = usize::try_from(json_u64_field(state_json, "levelIndex")?).ok()?;
    let level_count = parsed.level_bundle.as_ref()?.levels.len();
    (index < level_count).then_some(index)
}

fn materialize_level_start_state3(
    parsed: &ParsedPuzzle3,
    state: State3,
) -> Result<State3, AppError> {
    transition_program_without_input(&parsed.game, &state, &parsed.lifecycle.on_level_start)
        .map_err(|error| AppError::Config(format!("{error:?}")))
}

fn json_u64_field(source: &str, key: &str) -> Option<u64> {
    let mut value = json_value_after_key(source, key)?.trim_start();
    let end = value
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(value.len());
    value = &value[..end];
    (!value.is_empty()).then(|| value.parse().ok()).flatten()
}

fn json_u64_array_field(source: &str, key: &str) -> Option<Vec<u64>> {
    json_array_body(source, key).map(|body| {
        body.split(',')
            .filter_map(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty())
                    .then(|| trimmed.parse().ok())
                    .flatten()
            })
            .collect()
    })
}

fn json_i64_array_field(source: &str, key: &str) -> Option<Vec<i64>> {
    json_array_body(source, key).map(|body| {
        body.split(',')
            .filter_map(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty())
                    .then(|| trimmed.parse().ok())
                    .flatten()
            })
            .collect()
    })
}

fn json_array_body<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let value = json_value_after_key(source, key)?.trim_start();
    let rest = value.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(&rest[..end])
}

fn json_value_after_key<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let index = source.find(&needle)?;
    let after_key = &source[index + needle.len()..];
    after_key.trim_start().strip_prefix(':')
}

fn join_visuals_js(base: &str, generated: &str) -> String {
    match (base.trim().is_empty(), generated.is_empty()) {
        (true, true) => String::new(),
        (true, false) => generated.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}\n{generated}"),
    }
}

fn push_export_data(out: &mut String, state: &ServerState) {
    out.push('{');
    push_json_pair(out, "title", &state.loaded.title);
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = &state.loaded.subtitle {
        push_json_string(out, subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = &state.loaded.author {
        push_json_string(out, author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = &state.loaded.homepage {
        push_json_string(out, homepage);
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_json_pair(out, "source", &state.source);
    out.push(',');
    push_json_pair(out, "puzzlePath", &state.puzzle_path);
    out.push(',');
    push_json_pair(
        out,
        "saveKey",
        &progress_save_key(&state.loaded, &state.puzzle_path),
    );
    out.push(',');
    push_json_number(
        out,
        "progressSaveVersion",
        u64::from(puzzle_play::PROGRESS_SAVE_VERSION),
    );
    out.push(',');
    push_export_engine(out, &state.loaded);
    out.push(',');
    push_puzzle_screen(out, &state.loaded);
    out.push(',');
    push_export_levels(out, &state.loaded);
    out.push(',');
    push_inputs(out, &state.loaded);
    out.push(',');
    push_export_variables(out, &state.loaded.variables);
    out.push(',');
    push_scenes(out, "scenes", &state.loaded);
    out.push(',');
    push_scenes(out, "screens", &state.loaded);
    out.push(',');
    push_export_sounds(out, &state.loaded.sounds);
    out.push(',');
    push_export_theme(out, &state.loaded.theme);
    out.push(',');
    push_export_assets(out, &state.loaded);
    out.push(',');
    push_json_number(out, "defaultWaitMs", state.loaded.default_wait_ms);
    out.push(',');
    push_json_number(out, "defaultAgainMs", 120);
    out.push(',');
    push_export_goal(out, "goal", state.loaded.goal.as_ref());
    out.push(',');
    push_export_goal(out, "lose", state.loaded.lose.as_ref());
    out.push(',');
    push_export_conditions(out, &state.loaded);
    out.push('}');
}

fn progress_save_key(loaded: &LoadedGame, puzzle_path: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    progress_hash_str(&mut hash, puzzle_path);
    progress_hash_str(&mut hash, &loaded.title);
    for level in &loaded.levels {
        progress_hash_str(&mut hash, &level.name);
        hash = progress_hash_mix(hash, u64::from(level.initial_state.width));
        hash = progress_hash_mix(hash, u64::from(level.initial_state.height));
        hash = progress_hash_mix(hash, level.initial_state.hash());
    }
    format!("{}:{hash:016x}", loaded.title)
}

fn progress_hash_str(hash: &mut u64, value: &str) {
    *hash = progress_hash_mix(*hash, value.len() as u64);
    for byte in value.bytes() {
        *hash = progress_hash_mix(*hash, u64::from(byte));
    }
}

fn progress_hash_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}

fn push_export_assets(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"assets\":[");
    for (index, asset) in loaded.assets.entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(
            out,
            "kind",
            match asset.kind {
                AssetKind::Css => "css",
                AssetKind::Script => "script",
            },
        );
        out.push(',');
        push_json_pair(out, "path", &asset.path);
        out.push('}');
    }
    out.push(']');
}

fn push_export_theme(out: &mut String, theme: &ThemeDef) {
    out.push_str("\"theme\":{");
    out.push_str("\"name\":");
    if let Some(name) = &theme.name {
        push_json_string(out, name);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"variables\":{");
    for (index, variable) in theme.variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &variable.name);
        out.push(':');
        push_json_string(out, &variable.value);
    }
    out.push_str("}}");
}

fn push_export_sounds(out: &mut String, sounds: &SoundsDef) {
    out.push_str("\"sounds\":{");
    out.push_str("\"sfx\":[");
    for (index, sfx) in sounds.sfx.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &sfx.name);
        out.push(',');
        push_json_pair(out, "seed", &sfx.seed);
        out.push(',');
        push_json_pair(out, "type", &sfx.type_target);
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"music\":[");
    for (index, music) in sounds.music.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &music.name);
        out.push(',');
        push_json_pair(out, "seed", &music.seed);
        out.push(',');
        push_json_f64(out, "tone", music.tone);
        out.push(',');
        push_json_number(out, "bpm", u64::from(music.bpm));
        out.push(',');
        push_json_f64(out, "volume", music.volume);
        out.push('}');
    }
    out.push(']');
    out.push('}');
}

fn push_sound_events(out: &mut String, events: &[SoundEvent]) {
    out.push_str("\"soundEvents\":[");
    for (index, event) in events.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        match event {
            SoundEvent::PlaySfx { name } => {
                push_json_pair(out, "kind", "play_sfx");
                out.push(',');
                push_json_pair(out, "name", name);
            }
            SoundEvent::PlayMusic { name } => {
                push_json_pair(out, "kind", "play_music");
                out.push(',');
                push_json_pair(out, "name", name);
            }
            SoundEvent::PauseMusic { name } => {
                push_json_pair(out, "kind", "pause_music");
                out.push(',');
                out.push_str("\"name\":");
                if let Some(name) = name {
                    push_json_string(out, name);
                } else {
                    out.push_str("null");
                }
            }
            SoundEvent::ResumeMusic { name } => {
                push_json_pair(out, "kind", "resume_music");
                out.push(',');
                out.push_str("\"name\":");
                if let Some(name) = name {
                    push_json_string(out, name);
                } else {
                    out.push_str("null");
                }
            }
            SoundEvent::StopMusic { name } => {
                push_json_pair(out, "kind", "stop_music");
                out.push(',');
                out.push_str("\"name\":");
                if let Some(name) = name {
                    push_json_string(out, name);
                } else {
                    out.push_str("null");
                }
            }
        }
        out.push('}');
    }
    out.push(']');
}

fn push_message_events(out: &mut String, events: &[MessageEvent]) {
    out.push_str("\"messageEvents\":[");
    for (index, event) in events.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        match event {
            MessageEvent::Message { text } => {
                push_json_pair(out, "kind", "message");
                out.push(',');
                push_json_pair(out, "text", text);
            }
        }
        out.push('}');
    }
    out.push(']');
}

fn push_wait_events(out: &mut String, events: &[WaitEvent]) {
    out.push_str("\"waitEvents\":[");
    for (index, event) in events.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        match event {
            WaitEvent::Wait { milliseconds } => {
                push_json_pair(out, "kind", "wait");
                out.push(',');
                push_json_number(out, "milliseconds", *milliseconds);
            }
        }
        out.push('}');
    }
    out.push(']');
}

fn push_export_variables(out: &mut String, variables: &[puzzle_lang::SceneVarDef]) {
    out.push_str("\"variables\":[");
    for (index, variable) in variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_scene_var_def(out, variable);
    }
    out.push(']');
}

fn push_export_engine(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"engine\":{");
    push_json_number(out, "layerCount", loaded.game.layer_count as u64);
    out.push(',');
    push_export_objects(out, loaded);
    out.push(',');
    push_export_globals(out, loaded);
    out.push(',');
    push_export_queries(out, &loaded.game);
    out.push(',');
    out.push_str("\"visualObjects\":[");
    for (index, object) in loaded.game.visual_objects().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push(',');
    out.push_str("\"persistentVars\":[");
    for (index, var) in loaded.persistent_vars.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&var.0.to_string());
    }
    out.push(']');
    out.push(',');
    push_rule_emissions(out, loaded);
    out.push(',');
    out.push_str("\"program\":[");
    for (index, step) in loaded.game.program().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_rule_step(out, step);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"levelStartProgram\":");
    push_optional_rule_program(out, loaded.level_start_program.as_deref());
    out.push(',');
    out.push_str("\"runRulesOnLevelStart\":");
    out.push_str(if loaded.run_rules_on_level_start {
        "true"
    } else {
        "false"
    });
    out.push(',');
    out.push_str("\"displayLevelStartProgram\":");
    push_optional_rule_program(out, loaded.display_level_start_program.as_deref());
    out.push(',');
    out.push_str("\"levelClearProgram\":");
    push_optional_rule_program(out, loaded.level_clear_program.as_deref());
    out.push(',');
    out.push_str("\"displayLevelClearProgram\":");
    push_optional_rule_program(out, loaded.display_level_clear_program.as_deref());
    out.push(',');
    out.push_str("\"displayProgram\":");
    push_optional_rule_program(out, loaded.display_program.as_deref());
    out.push('}');
}

fn push_rule_emissions(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"ruleEmissions\":{");
    let mut entries = loaded.rule_emissions.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(rule, _)| rule.0);
    for (index, (rule, emissions)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &rule.0.to_string());
        out.push(':');
        out.push('[');
        for (emission_index, emission) in emissions.iter().enumerate() {
            if emission_index > 0 {
                out.push(',');
            }
            push_rule_emission(out, emission);
        }
        out.push(']');
    }
    out.push('}');
}

fn push_rule_emission(out: &mut String, emission: &RuleEmission) {
    out.push('{');
    match emission {
        RuleEmission::PlaySfx { name } => {
            push_json_pair(out, "kind", "play_sfx");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        RuleEmission::Wait { milliseconds } => {
            push_json_pair(out, "kind", "wait");
            out.push(',');
            push_json_number(out, "milliseconds", *milliseconds);
        }
        RuleEmission::Message { text, literal } => {
            push_json_pair(out, "kind", "message");
            out.push(',');
            push_json_pair(out, "text", text);
            out.push(',');
            push_json_bool(out, "literal", *literal);
        }
    }
    out.push('}');
}

fn push_export_globals(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"globals\":[");
    let mut entries = loaded.global_labels.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(global, _)| global.0);
    for (index, (global, name)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", global.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push('}');
    }
    out.push(']');
}

fn push_optional_rule_program(out: &mut String, program: Option<&[RuleStep]>) {
    out.push('[');
    if let Some(program) = program {
        for (index, step) in program.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_rule_step(out, step);
        }
    }
    out.push(']');
}

fn push_export_objects(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"objects\":[");
    for id in 1..=loaded.game.object_count() {
        if id > 1 {
            out.push(',');
        }
        let object_id = ObjectId(id as u16);
        let def = loaded
            .game
            .object(object_id)
            .expect("compiled object id should exist");
        let name = loaded.object_name(object_id);
        out.push('{');
        push_json_number(out, "id", def.id.0 as u64);
        out.push(',');
        push_json_number(out, "layer", def.layer_id.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push(',');
        push_json_pair(out, "sprite", &sprite_name(name));
        out.push('}');
    }
    out.push(']');
}

fn push_export_queries(out: &mut String, game: &CompiledGame) {
    out.push_str("\"queries\":[");
    for (index, query) in game.queries().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", query.id.0 as u64);
        out.push(',');
        push_query_kind(out, &query.kind);
        out.push('}');
    }
    out.push(']');
}

fn push_export_levels(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"levels\":[");
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", index as u64);
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
        out.push(',');
        push_scene_regions(out, Some(level));
        out.push(',');
        out.push_str("\"levelStartProgram\":");
        push_optional_rule_program(out, level.level_start_program.as_deref());
        out.push(',');
        out.push_str("\"levelClearProgram\":");
        push_optional_rule_program(out, level.level_clear_program.as_deref());
        out.push(',');
        out.push_str("\"initialState\":");
        push_state_data(out, &level.initial_state);
        out.push('}');
    }
    out.push(']');
}

fn push_state_data(out: &mut String, state: &State) {
    out.push('{');
    push_json_number(out, "width", state.width as u64);
    out.push(',');
    push_json_number(out, "height", state.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"slots\":[");
    for (index, object) in state.slots().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push(',');
    out.push_str("\"scratch\":[");
    for index in 0..state.slots().len() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        for (scratch_index, scratch) in state.slot_scratch_at(index).enumerate() {
            if scratch_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_number(out, "scratch", scratch.scratch.0 as u64);
            if let Some(value) = scratch.value {
                out.push(',');
                push_json_i64(out, "value", value);
            }
            out.push('}');
        }
        out.push(']');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"globals\":[");
    for (index, value) in state.visible_globals().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push_str("],");
    out.push_str("\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn push_state3_data(out: &mut String, state: &State3) {
    out.push('{');
    push_json_pair(out, "kind", "puzzle3d");
    out.push(',');
    push_json_number(out, "width", state.size.width as u64);
    out.push(',');
    push_json_number(out, "depth", state.size.depth as u64);
    out.push(',');
    push_json_number(out, "height", state.size.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"slots\":[");
    for (index, object) in state.slots().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push_str("],");
    out.push_str("\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn push_rule_step(out: &mut String, step: &RuleStep) {
    out.push('{');
    match step {
        RuleStep::Rule(rule) => {
            push_json_pair(out, "kind", "rule");
            out.push(',');
            out.push_str("\"rule\":");
            push_rule(out, rule);
        }
        RuleStep::ConditionalBlock { condition, steps } => {
            push_json_pair(out, "kind", "conditional");
            out.push(',');
            push_rule_condition(out, condition);
            out.push(',');
            out.push_str("\"steps\":[");
            for (index, step) in steps.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_rule_step(out, step);
            }
            out.push(']');
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => {
            push_json_pair(out, "kind", "block");
            out.push(',');
            push_rule_application(out, "application", *application);
            out.push(',');
            if let Some(condition) = stop_condition {
                push_rule_condition(out, condition);
            } else {
                out.push_str("\"condition\":null");
            }
            out.push(',');
            out.push_str("\"steps\":[");
            for (index, step) in steps.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_rule_step(out, step);
            }
            out.push(']');
        }
    }
    out.push('}');
}

fn push_rule_condition(out: &mut String, condition: &RuleCondition) {
    out.push_str("\"condition\":{");
    match condition {
        RuleCondition::AnyMatches(patterns) => {
            push_json_pair(out, "kind", "any_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        RuleCondition::NoMatches(patterns) => {
            push_json_pair(out, "kind", "no_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        RuleCondition::AnyInputMatches(patterns) => {
            push_json_pair(out, "kind", "any_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        RuleCondition::NoInputMatches(patterns) => {
            push_json_pair(out, "kind", "no_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        RuleCondition::GuardBranches(branches) => {
            push_json_pair(out, "kind", "guard_branches");
            out.push(',');
            out.push_str("\"branches\":[");
            for (branch_index, branch) in branches.iter().enumerate() {
                if branch_index > 0 {
                    out.push(',');
                }
                out.push('[');
                for (guard_index, guard) in branch.iter().enumerate() {
                    if guard_index > 0 {
                        out.push(',');
                    }
                    push_guard(out, guard);
                }
                out.push(']');
            }
            out.push(']');
        }
    }
    out.push('}');
}

fn push_rule(out: &mut String, rule: &Rule) {
    out.push('{');
    push_json_number(out, "id", rule.id.0 as u64);
    out.push(',');
    push_rule_application(out, "application", rule.application);
    out.push(',');
    out.push_str("\"guards\":[");
    for (index, guard) in rule.guards.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_guard(out, guard);
    }
    out.push(']');
    out.push(',');
    push_pattern(out, &rule.pattern);
    out.push(',');
    out.push_str("\"writes\":[");
    for (index, write) in rule.writes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_write(out, write);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"effects\":[");
    for (index, effect) in rule.effects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_rule_effect(out, effect);
    }
    out.push(']');
    out.push('}');
}

fn push_rule_application(out: &mut String, key: &str, application: RuleApplication) {
    push_json_pair(
        out,
        key,
        match application {
            RuleApplication::Once => "once",
            RuleApplication::OnceAll => "once_all",
            RuleApplication::OncePerLevel => "once_per_level",
            RuleApplication::UntilStable => "until_stable",
        },
    );
}

fn push_guard(out: &mut String, guard: &Guard) {
    out.push('{');
    match guard {
        Guard::InputIs(input) => {
            push_json_pair(out, "kind", "input_is");
            out.push(',');
            push_json_number(out, "input", input.0 as u64);
        }
        Guard::GlobalEquals { global, value } => {
            push_json_pair(out, "kind", "global_compare");
            out.push(',');
            push_json_number(out, "global", global.0 as u64);
            out.push(',');
            push_comparison_op(out, "op", ComparisonOp::Eq);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
        Guard::GlobalCompare { global, op, value } => {
            push_json_pair(out, "kind", "global_compare");
            out.push(',');
            push_json_number(out, "global", global.0 as u64);
            out.push(',');
            push_comparison_op(out, "op", *op);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
        Guard::QueryEquals { query, value } => {
            push_json_pair(out, "kind", "query_compare");
            out.push(',');
            push_json_number(out, "query", query.0 as u64);
            out.push(',');
            push_comparison_op(out, "op", ComparisonOp::Eq);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
        Guard::QueryNonZero(query) => {
            push_json_pair(out, "kind", "query_nonzero");
            out.push(',');
            push_json_number(out, "query", query.0 as u64);
        }
        Guard::QueryCompare { query, op, value } => {
            push_json_pair(out, "kind", "query_compare");
            out.push(',');
            push_json_number(out, "query", query.0 as u64);
            out.push(',');
            push_comparison_op(out, "op", *op);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
        Guard::QueryValue { kind, value } => {
            push_json_pair(out, "kind", "query_value_compare");
            out.push(',');
            push_query_kind(out, kind);
            out.push(',');
            push_comparison_op(out, "op", ComparisonOp::Eq);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
        Guard::QueryValueNonZero(kind) => {
            push_json_pair(out, "kind", "query_value_nonzero");
            out.push(',');
            push_query_kind(out, kind);
        }
        Guard::QueryValueCompare { kind, op, value } => {
            push_json_pair(out, "kind", "query_value_compare");
            out.push(',');
            push_query_kind(out, kind);
            out.push(',');
            push_comparison_op(out, "op", *op);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
    }
    out.push('}');
}

fn push_pattern(out: &mut String, pattern: &Pattern) {
    out.push_str("\"pattern\":{\"components\":[");
    for (component_index, component) in pattern.components.iter().enumerate() {
        if component_index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "gapCount", component.gap_count as u64);
        out.push(',');
        out.push_str("\"cells\":[");
        for (cell_index, cell) in component.cells.iter().enumerate() {
            if cell_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_offset_named(out, "offset", &cell.offset);
            out.push(',');
            push_object_ids(out, "requireObjects", &cell.require_objects);
            out.push(',');
            push_object_ids(out, "forbidObjects", &cell.forbid_objects);
            out.push(',');
            push_scratch_patterns(out, "requireScratch", &cell.require_scratch);
            out.push(',');
            push_scratch_patterns(out, "forbidScratch", &cell.forbid_scratch);
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
    out.push_str("]}");
}

fn push_scratch_patterns(out: &mut String, key: &str, scratch: &[ScratchPattern]) {
    push_json_string(out, key);
    out.push_str(":[");
    for (index, scratch) in scratch.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "object", scratch.object.0 as u64);
        out.push(',');
        push_json_number(out, "scratch", scratch.scratch.0 as u64);
        if let Some(value) = scratch.value {
            out.push(',');
            push_json_i64(out, "value", value);
        }
        out.push(',');
        push_json_pair(
            out,
            "match",
            match scratch.match_value {
                ScratchValueMatch::Any => "any",
                ScratchValueMatch::Exact => "exact",
            },
        );
        out.push('}');
    }
    out.push(']');
}

fn push_write(out: &mut String, write: &WriteOp) {
    out.push('{');
    match write {
        WriteOp::Add {
            component,
            offset,
            object,
        } => {
            push_json_pair(out, "kind", "add");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "offset", offset);
            out.push(',');
            push_json_number(out, "object", object.0 as u64);
        }
        WriteOp::Remove {
            component,
            offset,
            object,
        } => {
            push_json_pair(out, "kind", "remove");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "offset", offset);
            out.push(',');
            push_json_number(out, "object", object.0 as u64);
        }
        WriteOp::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            push_json_pair(out, "kind", "move");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "fromOffset", from_offset);
            out.push(',');
            push_offset_named(out, "toOffset", to_offset);
            out.push(',');
            push_json_number(out, "object", object.0 as u64);
        }
        WriteOp::Replace {
            component,
            offset,
            remove,
            add,
        } => {
            push_json_pair(out, "kind", "replace");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "offset", offset);
            out.push(',');
            push_json_number(out, "remove", remove.0 as u64);
            out.push(',');
            push_json_number(out, "add", add.0 as u64);
        }
        WriteOp::SetScratch {
            component,
            offset,
            object,
            scratch,
            value,
        } => {
            push_json_pair(out, "kind", "set_scratch");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "offset", offset);
            out.push(',');
            push_json_number(out, "object", object.0 as u64);
            out.push(',');
            push_json_number(out, "scratch", scratch.0 as u64);
            if let Some(value) = value {
                out.push(',');
                push_json_i64(out, "value", *value);
            }
        }
        WriteOp::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            value,
            match_value,
        } => {
            push_json_pair(out, "kind", "remove_scratch");
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push(',');
            push_offset_named(out, "offset", offset);
            out.push(',');
            push_json_number(out, "object", object.0 as u64);
            out.push(',');
            push_json_number(out, "scratch", scratch.0 as u64);
            if let Some(value) = value {
                out.push(',');
                push_json_i64(out, "value", *value);
            }
            out.push(',');
            push_json_pair(
                out,
                "match",
                match match_value {
                    ScratchValueMatch::Any => "any",
                    ScratchValueMatch::Exact => "exact",
                },
            );
        }
    }
    out.push('}');
}

fn push_rule_effect(out: &mut String, effect: &Effect) {
    out.push('{');
    match effect {
        Effect::Cancel => {
            push_json_pair(out, "kind", "cancel");
        }
        Effect::Win => {
            push_json_pair(out, "kind", "win");
        }
        Effect::Restart => {
            push_json_pair(out, "kind", "restart");
        }
        Effect::NextLevel => {
            push_json_pair(out, "kind", "next_level");
        }
        Effect::Again => {
            push_json_pair(out, "kind", "again");
        }
        Effect::UpdateGlobal { global, op, value } => {
            push_json_pair(out, "kind", "update_global");
            out.push(',');
            push_json_number(out, "global", global.0 as u64);
            out.push(',');
            push_global_update_op(out, "op", *op);
            out.push(',');
            push_json_i64(out, "value", *value);
        }
    }
    out.push('}');
}

fn push_query_kind(out: &mut String, kind: &QueryKind) {
    out.push_str("\"queryKind\":{");
    match kind {
        QueryKind::CountObjects(objects) => {
            push_json_pair(out, "kind", "count_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        QueryKind::ExistsObjects(objects) => {
            push_json_pair(out, "kind", "exists_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        QueryKind::NoneObjects(objects) => {
            push_json_pair(out, "kind", "none_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        QueryKind::CountMatches(patterns) => {
            push_json_pair(out, "kind", "count_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        QueryKind::ExistsMatches(patterns) => {
            push_json_pair(out, "kind", "exists_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        QueryKind::NoneMatches(patterns) => {
            push_json_pair(out, "kind", "none_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        QueryKind::CountInputMatches(patterns) => {
            push_json_pair(out, "kind", "count_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        QueryKind::ExistsInputMatches(patterns) => {
            push_json_pair(out, "kind", "exists_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        QueryKind::NoneInputMatches(patterns) => {
            push_json_pair(out, "kind", "none_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
    }
    out.push('}');
}

fn push_input_patterns(out: &mut String, patterns: &[(InputId, Pattern)]) {
    out.push_str("\"patterns\":[");
    for (index, (input, pattern)) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "input", input.0 as u64);
        out.push(',');
        push_pattern(out, pattern);
        out.push('}');
    }
    out.push(']');
}

fn push_patterns(out: &mut String, patterns: &[Pattern]) {
    out.push_str("\"patterns\":[");
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_pattern(out, pattern);
        out.push('}');
    }
    out.push(']');
}

fn push_export_goal(out: &mut String, key: &str, goal: Option<&GoalCondition>) {
    push_json_string(out, key);
    out.push(':');
    let Some(goal) = goal else {
        out.push_str("null");
        return;
    };
    out.push('{');
    push_json_pair(out, "description", &goal.description);
    out.push(',');
    push_goal_expr_named(out, "expr", &goal.expr);
    out.push('}');
}

fn push_export_conditions(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"conditions\":{");
    let mut entries = loaded.conditions.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, condition)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        out.push('{');
        push_json_pair(out, "description", &condition.description);
        out.push(',');
        push_goal_expr_named(out, "expr", &condition.expr);
        out.push('}');
    }
    out.push('}');
}

fn push_goal_expr_named(out: &mut String, key: &str, expr: &GoalExpr) {
    push_json_string(out, key);
    out.push(':');
    push_goal_expr(out, expr);
}

fn push_goal_expr(out: &mut String, expr: &GoalExpr) {
    out.push('{');
    match expr {
        GoalExpr::All(exprs) => {
            push_json_pair(out, "kind", "all");
            out.push(',');
            push_goal_exprs(out, exprs);
        }
        GoalExpr::Any(exprs) => {
            push_json_pair(out, "kind", "any");
            out.push(',');
            push_goal_exprs(out, exprs);
        }
        GoalExpr::Clause(clause) => {
            push_json_pair(out, "kind", "clause");
            out.push(',');
            out.push_str("\"value\":");
            push_goal_value(out, &clause.value);
            out.push(',');
            push_comparison_op(out, "op", clause.op);
            out.push(',');
            push_json_i64(out, "expected", clause.expected);
        }
    }
    out.push('}');
}

fn push_goal_exprs(out: &mut String, exprs: &[GoalExpr]) {
    out.push_str("\"exprs\":[");
    for (index, expr) in exprs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_goal_expr(out, expr);
    }
    out.push(']');
}

fn push_goal_value(out: &mut String, value: &GoalValue) {
    out.push('{');
    match value {
        GoalValue::Global(global) => {
            push_json_pair(out, "kind", "global");
            out.push(',');
            push_json_number(out, "global", global.0 as u64);
        }
        GoalValue::Query(query) => {
            push_json_pair(out, "kind", "query");
            out.push(',');
            push_json_number(out, "query", query.0 as u64);
        }
        GoalValue::QueryValue(kind) => {
            push_json_pair(out, "kind", "query_value");
            out.push(',');
            push_query_kind(out, kind);
        }
    }
    out.push('}');
}

fn push_object_ids(out: &mut String, key: &str, objects: &[ObjectId]) {
    push_json_string(out, key);
    out.push_str(":[");
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_offset_named(out: &mut String, key: &str, offset: &Offset) {
    push_json_string(out, key);
    out.push(':');
    out.push('{');
    match offset {
        Offset::Fixed { dx, dy } => {
            push_json_pair(out, "kind", "fixed");
            out.push(',');
            push_json_i64(out, "dx", i64::from(*dx));
            out.push(',');
            push_json_i64(out, "dy", i64::from(*dy));
        }
        Offset::Variable {
            base_dx,
            base_dy,
            gap_terms,
        } => {
            push_json_pair(out, "kind", "variable");
            out.push(',');
            push_json_i64(out, "baseDx", i64::from(*base_dx));
            out.push(',');
            push_json_i64(out, "baseDy", i64::from(*base_dy));
            out.push(',');
            out.push_str("\"gapTerms\":[");
            for (index, term) in gap_terms.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('{');
                push_json_number(out, "gapIndex", term.gap_index as u64);
                out.push(',');
                push_json_i64(out, "dx", i64::from(term.dx));
                out.push(',');
                push_json_i64(out, "dy", i64::from(term.dy));
                out.push('}');
            }
            out.push(']');
        }
    }
    out.push('}');
}

fn push_comparison_op(out: &mut String, key: &str, op: ComparisonOp) {
    push_json_pair(
        out,
        key,
        match op {
            ComparisonOp::Eq => "eq",
            ComparisonOp::NotEq => "not_eq",
            ComparisonOp::Greater => "greater",
            ComparisonOp::GreaterEq => "greater_eq",
            ComparisonOp::Less => "less",
            ComparisonOp::LessEq => "less_eq",
        },
    );
}

fn push_global_update_op(out: &mut String, key: &str, op: GlobalUpdateOp) {
    push_json_pair(
        out,
        key,
        match op {
            GlobalUpdateOp::Set => "set",
            GlobalUpdateOp::Add => "add",
            GlobalUpdateOp::Subtract => "subtract",
            GlobalUpdateOp::Multiply => "multiply",
            GlobalUpdateOp::Divide => "divide",
            GlobalUpdateOp::Remainder => "remainder",
        },
    );
}

fn push_solution_response(out: &mut String, loaded: &LoadedGame, response: &SolutionResponse) {
    out.push('{');
    match response {
        SolutionResponse::Solved {
            depth,
            moves,
            steps,
        } => {
            push_json_pair(out, "result", "solved");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_solution_moves(out, loaded, moves);
            out.push(',');
            push_solution_steps(out, loaded, steps);
        }
        SolutionResponse::Exhausted(stats) => {
            push_json_pair(out, "result", "exhausted");
            out.push(',');
            push_search_stats(out, stats);
        }
        SolutionResponse::BudgetExceeded(stats) => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_search_stats(out, stats);
        }
        SolutionResponse::Failed { depth, error } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
        }
    }
    out.push('}');
}

fn push_solution_response3(out: &mut String, parsed: &ParsedPuzzle3, response: &SolutionResponse3) {
    out.push('{');
    push_json_pair(out, "model", "puzzle3d");
    out.push(',');
    match response {
        SolutionResponse3::Solved {
            depth,
            moves,
            steps,
        } => {
            push_json_pair(out, "result", "solved");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_solution_moves3(out, parsed, moves);
            out.push(',');
            push_solution_steps3(out, parsed, steps);
        }
        SolutionResponse3::Exhausted(stats) => {
            push_json_pair(out, "result", "exhausted");
            out.push(',');
            push_search_stats(out, stats);
        }
        SolutionResponse3::BudgetExceeded(stats) => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_search_stats(out, stats);
        }
        SolutionResponse3::Failed { depth, error } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
        }
    }
    out.push('}');
}

#[cfg(not(target_arch = "wasm32"))]
fn bind_listener(preferred_port: u16) -> io::Result<(TcpListener, u16)> {
    for offset in 0..100 {
        let port = preferred_port.saturating_add(offset);
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return Ok((listener, port)),
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "no available port in requested range",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<ServerState>>,
) -> Result<(), AppError> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };

    let response = route(&request, state);
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn serve_static_html(html: String, puzzle_path: &Path, port: u16) -> Result<(), AppError> {
    let html = Arc::new(html);
    let (listener, port) = bind_listener(port)?;

    println!("html-play serving http://127.0.0.1:{port}");
    println!("puzzle: {}", puzzle_path.display());

    for stream in listener.incoming() {
        let stream = stream?;
        let html = Arc::clone(&html);
        if let Err(error) = handle_static_connection(stream, html) {
            eprintln!("request error: {error}");
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_static_connection(mut stream: TcpStream, html: Arc<String>) -> Result<(), AppError> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };

    let response = route_static_html(&request, &html);
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn route_static_html(request: &HttpRequest, html: &str) -> String {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => http_ok("text/html; charset=utf-8", html),
        _ => http_error(404, "not found"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn read_request(stream: &mut TcpStream) -> Result<Option<HttpRequest>, AppError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut header_end = None;
    let mut content_length = 0;

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);

        if header_end.is_none() {
            if let Some(end) = find_header_end(&bytes) {
                header_end = Some(end);
                content_length = parse_content_length(&bytes[..end]);
            }
        }

        if let Some(end) = header_end {
            if bytes.len() >= end + 4 + content_length {
                break;
            }
        }
    }

    let Some(end) = header_end else {
        return Err(AppError::Config("invalid HTTP request".to_string()));
    };
    let header = String::from_utf8_lossy(&bytes[..end]);
    let mut request_line = header.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let mut path = request_line.next().unwrap_or("/").to_string();
    if let Some(query_index) = path.find('?') {
        path.truncate(query_index);
    }

    Ok(Some(HttpRequest { method, path }))
}

#[cfg(not(target_arch = "wasm32"))]
fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_content_length(header: &[u8]) -> usize {
    String::from_utf8_lossy(header)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

#[cfg(not(target_arch = "wasm32"))]
fn route(request: &HttpRequest, state: Arc<Mutex<ServerState>>) -> String {
    let method = request.method.as_str();
    let path = request.path.as_str();

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => http_ok("text/html; charset=utf-8", INDEX_HTML),
        ("GET", "/app.css") => http_ok("text/css; charset=utf-8", APP_CSS),
        ("GET", "/theme-presets.css") => http_ok("text/css; charset=utf-8", THEME_PRESETS_CSS),
        ("GET", "/renderer.css") => http_ok("text/css; charset=utf-8", RENDERER_CSS),
        ("GET", "/game.css") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("text/css; charset=utf-8", &state.game_css)
        }
        ("GET", "/game.visuals.js") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("text/javascript; charset=utf-8", &state.game_visuals_js)
        }
        ("GET", "/sound-generator.js") => {
            let script = sound_tools_js();
            http_ok("text/javascript; charset=utf-8", &script)
        }
        ("GET", "/app.js") => http_ok("text/javascript; charset=utf-8", APP_JS),
        ("GET", "/renderer.js") => http_ok("text/javascript; charset=utf-8", RENDERER_JS),
        ("GET", "/api/state") => {
            let mut state = state.lock().expect("server state poisoned");
            http_ok("application/json; charset=utf-8", &state.snapshot_json())
        }
        ("GET", "/api/scene") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("application/json; charset=utf-8", &state.scene_json())
        }
        ("POST", "/api/solve") => {
            let state = state.lock().expect("server state poisoned");
            match state.solve_json() {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/command/undo") => mutate(state, |state| state.session.undo(&state.loaded)),
        ("POST", "/api/command/redo") => mutate(state, |state| state.session.redo(&state.loaded)),
        ("POST", "/api/command/restart") => {
            mutate(state, |state| state.session.restart_level(&state.loaded))
        }
        ("POST", "/api/command/next") => {
            mutate(state, |state| state.session.advance_level(&state.loaded))
        }
        ("POST", path) if path.starts_with("/api/input/") => {
            let input_name = percent_decode(&path["/api/input/".len()..]);
            let mut state = state.lock().expect("server state poisoned");
            match state.apply_input_name(&input_name) {
                Ok(()) => http_ok("application/json; charset=utf-8", &state.snapshot_json()),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", path) if path.starts_with("/api/command/") => {
            let command_name = percent_decode(&path["/api/command/".len()..]);
            let mut state = state.lock().expect("server state poisoned");
            match state.apply_command_name(&command_name) {
                Ok(()) => http_ok("application/json; charset=utf-8", &state.snapshot_json()),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        _ => http_error(404, "not found"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mutate<F>(state: Arc<Mutex<ServerState>>, update: F) -> String
where
    F: FnOnce(&mut ServerState),
{
    let mut state = state.lock().expect("server state poisoned");
    update(&mut state);
    http_ok("application/json; charset=utf-8", &state.snapshot_json())
}

fn input_id_by_name(loaded: &LoadedGame, input_name: &str) -> Option<InputId> {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

fn solver_inputs(loaded: &LoadedGame) -> Vec<InputId> {
    let mut inputs = loaded.input_labels.keys().copied().collect::<Vec<_>>();
    inputs.sort();
    inputs
}

fn solver_inputs3(game: &Game3) -> Vec<InputId3> {
    let mut inputs = game
        .inputs
        .iter()
        .filter(|input| {
            !matches!(
                input.name.as_str(),
                "undo" | "restart" | "next_level" | "previous_level"
            )
        })
        .map(|input| input.id)
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}

fn push_solution_moves(out: &mut String, loaded: &LoadedGame, inputs: &[InputId]) {
    out.push_str("\"moves\":[");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_input_move(out, loaded, *input);
    }
    out.push(']');
}

fn push_solution_steps(out: &mut String, loaded: &LoadedGame, steps: &[SolutionStep]) {
    out.push_str("\"steps\":[");
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_input_move(out, loaded, input);
        } else {
            out.push_str("\"move\":null");
        }
        out.push(',');
        push_scene(out, loaded, &step.state, None, None);
        out.push('}');
    }
    out.push(']');
}

fn push_solution_moves3(out: &mut String, parsed: &ParsedPuzzle3, inputs: &[InputId3]) {
    out.push_str("\"moves\":[");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_input_move3(out, parsed, *input);
    }
    out.push(']');
}

fn push_solution_steps3(out: &mut String, parsed: &ParsedPuzzle3, steps: &[SolutionStep3]) {
    out.push_str("\"steps\":[");
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_input_move3(out, parsed, input);
        } else {
            out.push_str("\"move\":null");
        }
        out.push(',');
        push_json_bool(out, "completed", step.completed);
        out.push(',');
        out.push_str("\"clearCommands\":");
        if step.completed {
            push_lifecycle_commands3(out, &parsed.lifecycle.on_level_clear);
        } else {
            out.push_str("[]");
        }
        out.push(',');
        push_state3_scene(out, parsed, &step.state);
        out.push('}');
    }
    out.push(']');
}

fn push_input_move(out: &mut String, loaded: &LoadedGame, input: InputId) {
    out.push('{');
    let name = loaded
        .input_labels
        .get(&input)
        .map(String::as_str)
        .unwrap_or("?");
    push_json_pair(out, "name", name);
    out.push(',');
    if let Some(key) = key_for_input(loaded, input) {
        push_json_pair(out, "key", &key);
    } else {
        out.push_str("\"key\":null");
    }
    out.push(',');
    if let Some(arrow) = arrow_for_input(loaded, input) {
        push_json_pair(out, "arrow", &arrow);
    } else {
        out.push_str("\"arrow\":null");
    }
    out.push('}');
}

fn push_input_move3(out: &mut String, parsed: &ParsedPuzzle3, input: InputId3) {
    out.push('{');
    let input_def = parsed.game.input(input);
    let name = input_def.map(|input| input.name.as_str()).unwrap_or("?");
    push_json_pair(out, "name", name);
    out.push(',');
    out.push_str("\"key\":null");
    out.push(',');
    out.push_str("\"arrow\":null");
    out.push(',');
    if let Some(direction) = input_def.and_then(|input| input.direction) {
        push_json_pair(out, "direction", direction.name);
    } else {
        out.push_str("\"direction\":null");
    }
    out.push('}');
}

fn push_lifecycle_commands3(out: &mut String, commands: &[LifecycleCommand3]) {
    out.push('[');
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match command {
            LifecycleCommand3::NextLevel => push_json_string(out, "next_level"),
        }
    }
    out.push(']');
}

fn push_state3_scene(out: &mut String, parsed: &ParsedPuzzle3, state: &State3) {
    out.push_str("\"scene\":{");
    push_json_pair(out, "kind", "puzzle3d");
    out.push(',');
    push_size3(out, state.size);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"cells\":[");
    let mut first = true;
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let position = Coord3 { x, y, z };
                let Ok(view) = state.cell_view(position) else {
                    continue;
                };
                if view.objects.is_empty() {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('{');
                push_coord3(out, position);
                out.push(',');
                out.push_str("\"objects\":[");
                for (object_index, object) in view.objects.iter().enumerate() {
                    if object_index > 0 {
                        out.push(',');
                    }
                    push_object3(out, parsed, *object);
                }
                out.push_str("]}");
            }
        }
    }
    out.push_str("]}");
}

fn push_size3(out: &mut String, size: Size3) {
    out.push_str("\"size\":{");
    push_json_number(out, "width", size.width as u64);
    out.push(',');
    push_json_number(out, "depth", size.depth as u64);
    out.push(',');
    push_json_number(out, "height", size.height as u64);
    out.push('}');
}

fn push_coord3(out: &mut String, position: Coord3) {
    out.push_str("\"position\":{");
    push_json_number(out, "x", position.x as u64);
    out.push(',');
    push_json_number(out, "y", position.y as u64);
    out.push(',');
    push_json_number(out, "z", position.z as u64);
    out.push('}');
}

fn push_object3(out: &mut String, parsed: &ParsedPuzzle3, object: ObjectId3) {
    out.push('{');
    push_json_number(out, "id", object.0 as u64);
    out.push(',');
    let name = parsed
        .catalog
        .objects
        .iter()
        .find_map(|entry| (entry.id == object).then_some(entry.name.as_str()))
        .unwrap_or("?");
    push_json_pair(out, "name", name);
    out.push(',');
    if let Some(layer) = parsed.game.object_layer(object) {
        push_json_number(out, "layer", layer.0 as u64);
    } else {
        out.push_str("\"layer\":null");
    }
    out.push(',');
    push_json_pair(out, "sprite", name);
    out.push('}');
}

fn push_search_stats(out: &mut String, stats: &puzzle_solver::SearchStats) {
    out.push_str("\"stats\":{");
    push_json_number(out, "visited", stats.visited as u64);
    out.push(',');
    push_json_number(out, "expanded", stats.expanded as u64);
    out.push(',');
    push_json_number(out, "frontier", stats.frontier as u64);
    out.push(',');
    push_json_number(out, "maxDepthReached", stats.max_depth_reached as u64);
    out.push(',');
    push_json_number(out, "elapsedMs", stats.elapsed.as_millis() as u64);
    out.push('}');
}

fn push_scene_state_compat(out: &mut String, state: Option<&puzzle_play::SceneRuntimeState>) {
    out.push_str("\"screenState\":{");
    let Some(state) = state else {
        out.push('}');
        return;
    };
    let mut entries = state.values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, value);
    }
    out.push('}');
}

fn push_session_state(out: &mut String, values: &std::collections::HashMap<String, SceneValue>) {
    out.push_str("\"gameState\":{");
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, value);
    }
    out.push('}');
}

fn push_scene_puzzles_compat(out: &mut String, state: Option<&puzzle_play::SceneRuntimeState>) {
    out.push_str("\"screenPuzzles\":[");
    let Some(state) = state else {
        out.push(']');
        return;
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
    }
    out.push(']');
}

fn push_scene_state(out: &mut String, state: Option<&puzzle_play::SceneRuntimeState>) {
    out.push_str("\"sceneState\":{");
    let Some(state) = state else {
        out.push('}');
        return;
    };
    let mut entries = state.values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, value);
    }
    out.push('}');
}

fn push_scene_puzzles(out: &mut String, state: Option<&puzzle_play::SceneRuntimeState>) {
    out.push_str("\"scenePuzzles\":[");
    let Some(state) = state else {
        out.push(']');
        return;
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
    }
    out.push(']');
}

fn push_scene_puzzle_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"scenePuzzleState\":{");
    let Some(state) = session.scene_state() else {
        out.push('}');
        return;
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push_str(":{");
        if let Some(level_index) = state
            .puzzles
            .get(name)
            .and_then(|puzzle| puzzle.level_index)
        {
            out.push_str("\"level\":");
            push_level_ref(out, loaded, session.focused_scene(), level_index);
        } else {
            out.push_str("\"level\":null");
        }
        out.push('}');
    }
    out.push('}');
}

fn push_level_ref(out: &mut String, loaded: &LoadedGame, scene_name: &str, level_index: usize) {
    out.push('{');
    push_json_number(out, "index", level_index as u64);
    if let Some(level) = loaded.levels.get(level_index) {
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "label", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
    }
    out.push(',');
    push_json_bool(
        out,
        "has_next",
        level_has_next_in_scene(loaded, scene_name, level_index),
    );
    out.push(',');
    push_json_bool(
        out,
        "last",
        !level_has_next_in_scene(loaded, scene_name, level_index),
    );
    out.push('}');
}

fn level_has_next_in_scene(loaded: &LoadedGame, scene_name: &str, level_index: usize) -> bool {
    let indices = html_scene_level_indices(loaded, scene_name);
    indices
        .iter()
        .position(|index| *index == level_index)
        .is_some_and(|position| position + 1 < indices.len())
}

fn html_scene_level_indices(loaded: &LoadedGame, scene_name: &str) -> Vec<usize> {
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
                    .any(|name| html_level_resource_matches(name, &level.name))
                    .then_some(index)
            })
            .collect(),
    }
}

fn html_level_resource_matches(resource: &str, level_name: &str) -> bool {
    level_name == resource
        || level_name
            .strip_prefix(resource)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn push_visible_scenes_compat(out: &mut String, scenes: &[String]) {
    out.push_str("\"visibleScreens\":[");
    for (index, scene) in scenes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, scene);
    }
    out.push(']');
}

fn push_visible_scenes(out: &mut String, scenes: &[String]) {
    out.push_str("\"visibleScenes\":[");
    for (index, scene) in scenes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, scene);
    }
    out.push(']');
}

fn push_scene_value(out: &mut String, value: &SceneValue) {
    match value {
        SceneValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        SceneValue::Int(value) => out.push_str(&value.to_string()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => push_json_string(out, value),
    }
}

fn push_scene_layers(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"sceneLayers\":[");
    for (index, name) in session.visible_scenes().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let state = session.scene_state_for(name);
        out.push('{');
        push_json_pair(out, "name", name);
        out.push(',');
        push_json_bool(out, "focused", name == session.focused_scene());
        out.push(',');
        push_scene_state(out, state);
        out.push(',');
        push_scene_puzzles(out, state);
        out.push(',');
        out.push_str("\"scene\":");
        if let Some((puzzle_state, level)) = scene_puzzle_state(loaded, session, name) {
            let scene_def = loaded.scenes.iter().find(|scene| scene.name == *name);
            push_scene_object(
                out,
                loaded,
                puzzle_state,
                level,
                scene_def.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("null");
        }
        out.push('}');
    }
    out.push(']');
}

fn focused_scene_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
) -> Option<&'a puzzle_core::State> {
    scene_puzzle_state(loaded, session, session.focused_scene()).map(|(state, _)| state)
}

fn scene_puzzle_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
    scene_name: &str,
) -> Option<(&'a puzzle_core::State, Option<&'a Level>)> {
    let scene = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)?;
    let state = session.scene_state_for(scene_name)?;
    let puzzle = if let Some(rule) = &scene.puzzle_rule {
        if let Some(puzzle_name) = rule.target.split('.').next_back() {
            if let Some(puzzle) = state.puzzles.get(puzzle_name) {
                Some(puzzle)
            } else {
                None
            }
        } else {
            None
        }
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

fn push_scene(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) {
    out.push_str("\"scene\":");
    push_scene_object(out, loaded, state, level, resources);
}

fn push_scene_object(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) {
    let display_state = materialize_display_state(loaded, state);
    let state = display_state.as_ref().unwrap_or(state);
    out.push('{');
    push_json_number(out, "width", state.width as u64);
    out.push(',');
    push_json_number(out, "height", state.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    push_puzzle_screen(out, loaded);
    out.push(',');
    out.push_str("\"resources\":");
    if let Some(resources) = resources {
        out.push('{');
        push_scene_resources_object(out, resources);
        out.push('}');
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_scene_regions(out, level);
    out.push(',');
    push_cells(out, loaded, state);
    out.push('}');
}

fn materialize_display_state(loaded: &LoadedGame, state: &puzzle_core::State) -> Option<State> {
    let program = loaded.display_program.as_deref()?;
    transition_program(&loaded.game, program, state, InputId(0)).ok()
}

fn push_puzzle_screen(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"screen\":{");
    out.push_str("\"viewportSize\":");
    match &loaded.screen.viewport_size {
        puzzle_lang::ViewportSizeDef::Full => {
            out.push('{');
            push_json_pair(out, "kind", "full");
            out.push('}');
        }
        puzzle_lang::ViewportSizeDef::Size { width, height } => {
            out.push('{');
            push_json_pair(out, "kind", "size");
            out.push(',');
            push_json_number(out, "width", *width as u64);
            out.push(',');
            push_json_number(out, "height", *height as u64);
            out.push('}');
        }
    }
    out.push(',');
    push_json_pair(out, "viewportFocus", &loaded.screen.viewport_focus);
    out.push(',');
    out.push_str("\"viewportFocusObjects\":");
    push_viewport_focus_objects(out, loaded);
    out.push(',');
    let mode = match loaded.screen.viewport_mode {
        puzzle_lang::ViewportModeDef::Paged => "paged",
        puzzle_lang::ViewportModeDef::Centered => "centered",
    };
    push_json_pair(out, "viewportMode", mode);
    out.push('}');
}

fn push_viewport_focus_objects(out: &mut String, loaded: &LoadedGame) {
    let focus = &loaded.screen.viewport_focus;
    let mut objects = loaded
        .object_groups
        .get(focus)
        .cloned()
        .or_else(|| {
            loaded.object_groups.iter().find_map(|(name, objects)| {
                name.eq_ignore_ascii_case(focus).then(|| objects.clone())
            })
        })
        .unwrap_or_else(|| {
            loaded
                .object_labels
                .iter()
                .filter_map(|(object, name)| name.eq_ignore_ascii_case(focus).then_some(*object))
                .collect()
        });
    objects.sort_by_key(|object| object.0);
    objects.dedup();
    out.push('[');
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_scene_regions(out: &mut String, level: Option<&Level>) {
    out.push_str("\"regions\":[");
    if let Some(level) = level {
        for (index, region) in level.regions.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_number(out, "index", region.index as u64);
            out.push(',');
            push_json_number(out, "x", region.x as u64);
            out.push(',');
            push_json_number(out, "y", region.y as u64);
            out.push(',');
            push_json_number(out, "width", region.width as u64);
            out.push(',');
            push_json_number(out, "height", region.height as u64);
            out.push('}');
        }
    }
    out.push(']');
}

fn push_cells(out: &mut String, loaded: &LoadedGame, state: &puzzle_core::State) {
    out.push_str("\"cells\":[");
    let mut first_cell = true;

    for y in 0..state.height {
        for x in 0..state.width {
            if !first_cell {
                out.push(',');
            }
            first_cell = false;

            out.push('{');
            push_json_number(out, "x", x as u64);
            out.push(',');
            push_json_number(out, "y", y as u64);
            out.push(',');
            out.push_str("\"layers\":[");

            let mut first_layer = true;
            for layer in 0..state.layer_count {
                let layer_id = LayerId(layer);
                let object = state.get_layer(x, y, layer_id).unwrap_or(ObjectId::EMPTY);
                if object.is_empty() {
                    continue;
                }

                if !first_layer {
                    out.push(',');
                }
                first_layer = false;

                let object_name = loaded.object_name(object);
                out.push('{');
                push_json_number(out, "layer", layer as u64);
                out.push(',');
                push_json_number(out, "objectId", object.0 as u64);
                out.push(',');
                push_json_pair(out, "object", object_name);
                out.push(',');
                push_json_pair(out, "sprite", &sprite_name(object_name));
                out.push('}');
            }

            out.push(']');
            out.push('}');
        }
    }

    out.push(']');
}

fn sprite_name(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "unknown".to_string()
    } else {
        sprite
    }
}

fn push_inputs(out: &mut String, loaded: &LoadedGame) {
    let mut inputs = loaded
        .input_labels
        .iter()
        .map(|(id, name)| (*id, name.as_str()))
        .collect::<Vec<_>>();
    inputs.sort_by_key(|(id, _)| *id);

    out.push_str("\"inputs\":[");
    for (index, (id, name)) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", id.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push(',');
        if let Some(key) = key_for_input(loaded, *id) {
            push_json_pair(out, "key", &key);
        } else {
            out.push_str("\"key\":null");
        }
        out.push(',');
        if let Some(arrow) = arrow_for_input(loaded, *id) {
            push_json_pair(out, "arrow", &arrow);
        } else {
            out.push_str("\"arrow\":null");
        }
        out.push(',');
        out.push_str("\"keys\":[");
        for (key_index, key) in key_triggers_for_input(loaded, *id).iter().enumerate() {
            if key_index > 0 {
                out.push(',');
            }
            push_json_string(out, key);
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
}

fn push_levels(out: &mut String, loaded: &LoadedGame, cleared_levels: &[bool]) {
    out.push_str("\"levels\":[");
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", index as u64);
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
        out.push(',');
        push_json_bool(
            out,
            "cleared",
            cleared_levels.get(index).copied().unwrap_or(false),
        );
        out.push('}');
    }
    out.push(']');
}

fn push_game_context(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"game\":{");
    push_json_pair(out, "title", &loaded.title);
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = &loaded.subtitle {
        push_json_string(out, subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = &loaded.author {
        push_json_string(out, author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = &loaded.homepage {
        push_json_string(out, homepage);
    } else {
        out.push_str("null");
    }
    out.push('}');
}

fn push_level_context(out: &mut String, loaded: &LoadedGame, level_index: Option<usize>) {
    let Some(level_index) = level_index else {
        out.push_str("\"level\":null");
        return;
    };
    let Some(level) = loaded.levels.get(level_index) else {
        out.push_str("\"level\":null");
        return;
    };
    out.push_str("\"level\":{");
    push_json_number(out, "index", level_index as u64);
    out.push(',');
    push_json_number(out, "number", level_index as u64 + 1);
    out.push(',');
    push_json_number(out, "count", loaded.levels.len() as u64);
    out.push(',');
    push_json_pair(out, "name", &level.name);
    out.push(',');
    push_json_pair(out, "label", &level.name);
    out.push(',');
    push_json_pair(out, "puzzle", &level.puzzle);
    out.push(',');
    if let Some(pack) = &level.pack {
        push_json_pair(out, "pack", pack);
    } else {
        out.push_str("\"pack\":null");
    }
    out.push('}');
}

fn push_menus(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"menus\":[");
    for (index, menu) in loaded.menus.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &menu.name);
        out.push(',');
        out.push_str("\"view\":[");
        for (component_index, component) in menu.view.iter().enumerate() {
            if component_index > 0 {
                out.push(',');
            }
            push_menu_component(out, component);
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
}

fn push_menu_component(out: &mut String, component: &MenuComponent) {
    out.push('{');
    match component {
        MenuComponent::Text(text) => {
            push_json_pair(out, "kind", "text");
            out.push(',');
            match &text.content {
                SceneTextContent::Literal(value) => {
                    push_json_pair(out, "source", "literal");
                    out.push(',');
                    push_json_pair(out, "value", value);
                }
                SceneTextContent::Path(path) => {
                    push_json_pair(out, "source", "path");
                    out.push(',');
                    push_json_pair(out, "path", &path.join("."));
                }
            }
        }
        MenuComponent::Button(button) => {
            push_json_pair(out, "kind", "button");
            out.push(',');
            push_json_expr_named(out, "label", &button.label);
            out.push(',');
            push_json_expr_named(out, "value", &button.value);
        }
        MenuComponent::Row(container) => {
            push_json_pair(out, "kind", "row");
            out.push(',');
            push_menu_children(out, &container.children);
        }
        MenuComponent::Column(container) => {
            push_json_pair(out, "kind", "column");
            out.push(',');
            push_menu_children(out, &container.children);
        }
        MenuComponent::Box(container) => {
            push_json_pair(out, "kind", "box");
            out.push(',');
            push_menu_children(out, &container.children);
        }
        MenuComponent::For(for_view) => {
            push_json_pair(out, "kind", "for");
            out.push(',');
            push_json_pair(out, "binding", &for_view.binding);
            out.push(',');
            push_json_pair(out, "source", for_view.source.as_str());
            out.push(',');
            push_menu_children(out, &for_view.children);
        }
    }
    out.push('}');
}

fn push_menu_children(out: &mut String, children: &[MenuComponent]) {
    out.push_str("\"children\":[");
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_menu_component(out, child);
    }
    out.push(']');
}

fn push_scenes(out: &mut String, key: &str, loaded: &LoadedGame) {
    push_json_string(out, key);
    out.push_str(":[");
    for (screen_index, scene) in loaded.scenes.iter().enumerate() {
        if screen_index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &scene.name);
        out.push(',');
        push_scene_layout(out, &scene.layout);
        out.push(',');
        push_scene_resources(out, scene);
        out.push(',');
        push_scene_state_def(out, scene);
        out.push(',');
        out.push_str("\"puzzleRule\":");
        if let Some(rule) = &scene.puzzle_rule {
            out.push('{');
            push_json_pair(out, "target", &rule.target);
            out.push(',');
            push_json_pair(out, "rule", &rule.rule);
            out.push('}');
        } else {
            out.push_str("null");
        }
        out.push(',');
        out.push_str("\"components\":[");
        for (component_index, component) in scene.components.iter().enumerate() {
            if component_index > 0 {
                out.push(',');
            }
            push_scene_component(out, component);
        }
        out.push(']');
        out.push(',');
        out.push_str("\"keys\":[");
        for (binding_index, binding) in scene.key_bindings.iter().enumerate() {
            if binding_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_effect(out, &binding.effect);
            out.push(',');
            out.push_str("\"keys\":[");
            for (key_index, key) in binding.keys.iter().enumerate() {
                if key_index > 0 {
                    out.push(',');
                }
                push_json_string(out, &key_trigger_name(key));
            }
            out.push(']');
            out.push('}');
        }
        out.push(']');
        out.push(',');
        out.push_str("\"transitions\":[");
        for (transition_index, transition) in scene.transitions.iter().enumerate() {
            if transition_index > 0 {
                out.push(',');
            }
            out.push('{');
            match &transition.trigger {
                SceneTransitionTrigger::Condition(condition) => {
                    push_json_pair(out, "condition", condition);
                }
                SceneTransitionTrigger::SceneStart => {
                    push_json_pair(out, "lifecycle", "scene_start");
                }
                SceneTransitionTrigger::LevelStart => {
                    push_json_pair(out, "lifecycle", "level_start");
                }
            }
            out.push(',');
            push_json_effect(out, &transition.effect);
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
}

fn push_scene_resources(out: &mut String, scene: &puzzle_lang::SceneDef) {
    out.push_str("\"resources\":{");
    push_scene_resources_object(out, &scene.resources);
    out.push('}');
}

fn push_scene_resources_object(out: &mut String, resources: &puzzle_lang::SceneResources) {
    push_json_pair(
        out,
        "levelsMode",
        resource_selection_mode(&resources.levels),
    );
    out.push(',');
    push_resource_names(out, "levels", &resources.levels);
    out.push(',');
    push_json_pair(
        out,
        "spritesMode",
        resource_selection_mode(&resources.sprites),
    );
    out.push(',');
    push_resource_names(out, "sprites", &resources.sprites);
}

fn resource_selection_mode(selection: &ResourceSelection) -> &'static str {
    match selection {
        ResourceSelection::All => "all",
        ResourceSelection::Named(_) => "named",
    }
}

fn push_resource_names(out: &mut String, name: &str, selection: &ResourceSelection) {
    out.push('"');
    out.push_str(name);
    out.push_str("\":[");
    if let ResourceSelection::Named(names) = selection {
        for (index, value) in names.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_json_string(out, value);
        }
    }
    out.push(']');
}

fn push_scene_state_def(out: &mut String, scene: &puzzle_lang::SceneDef) {
    out.push_str("\"state\":{");
    out.push_str("\"variables\":[");
    for (index, variable) in scene.state.variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_scene_var_def(out, variable);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"puzzles\":[");
    for (index, puzzle) in scene.state.puzzles.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &puzzle.name);
        out.push(',');
        push_json_pair(out, "kind", &puzzle.kind);
        out.push(',');
        push_json_pair(out, "model", &puzzle.model);
        out.push(',');
        match &puzzle.initializer {
            ScenePuzzleInitializer::CurrentLevel => {
                push_json_pair(out, "initializer", "current_level")
            }
            ScenePuzzleInitializer::Level(level_name) => {
                push_json_pair(out, "initializer", "level");
                out.push(',');
                push_json_pair(out, "level", level_name);
            }
        }
        out.push('}');
    }
    out.push(']');
    out.push('}');
}

fn push_scene_var_def(out: &mut String, variable: &puzzle_lang::SceneVarDef) {
    out.push('{');
    push_json_pair(out, "name", &variable.name);
    out.push(',');
    out.push_str("\"default\":");
    push_scene_value(out, &variable.default);
    out.push(',');
    push_json_pair(
        out,
        "lifetime",
        match variable.lifetime {
            puzzle_lang::SceneStateLifetime::Instance => "instance",
            puzzle_lang::SceneStateLifetime::ResetOnStart => "reset_on_start",
            puzzle_lang::SceneStateLifetime::Persistent => "persistent",
        },
    );
    out.push(',');
    push_json_bool(out, "mutable", variable.mutable);
    out.push('}');
}

fn push_scene_layout(out: &mut String, layout: &SceneLayoutDef) {
    out.push_str("\"layout\":{");
    let mut wrote = false;
    if let Some(size) = layout.size {
        out.push_str("\"size\":{");
        push_json_number(out, "width", size.width as u64);
        out.push(',');
        push_json_number(out, "height", size.height as u64);
        out.push('}');
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push(',');
        }
        push_json_number(out, "gap", gap as u64);
        wrote = true;
    }
    if layout.align != SceneLayoutDef::default().align {
        if wrote {
            out.push(',');
        }
        out.push_str("\"align\":{");
        push_json_pair(
            out,
            "x",
            match layout.align.x {
                SceneAlignXDef::Left => "left",
                SceneAlignXDef::Center => "center",
                SceneAlignXDef::Right => "right",
            },
        );
        out.push(',');
        push_json_pair(
            out,
            "y",
            match layout.align.y {
                SceneAlignYDef::Top => "top",
                SceneAlignYDef::Center => "center",
                SceneAlignYDef::Bottom => "bottom",
            },
        );
        out.push('}');
        wrote = true;
    }
    if layout.scroll {
        if wrote {
            out.push(',');
        }
        push_json_bool(out, "scroll", true);
    }
    out.push('}');
}

fn push_scene_component(out: &mut String, component: &SceneComponent) {
    out.push('{');
    match component {
        SceneComponent::Frame(frame) => {
            push_json_pair(out, "kind", &frame.kind);
            out.push(',');
            push_json_pair(out, "source", &frame.source);
            out.push(',');
            push_scene_layout(out, &frame.layout);
        }
        SceneComponent::Title(title) => {
            push_json_pair(out, "kind", "title");
            out.push(',');
            push_json_expr_named(out, "content", &title.content);
        }
        SceneComponent::Subtitle(subtitle) => {
            push_json_pair(out, "kind", "subtitle");
            out.push(',');
            push_json_expr_named(out, "content", &subtitle.content);
        }
        SceneComponent::Text(text) => {
            push_json_pair(out, "kind", "text");
            out.push(',');
            match &text.content {
                SceneTextContent::Literal(value) => {
                    push_json_pair(out, "source", "literal");
                    out.push(',');
                    push_json_pair(out, "value", value);
                }
                SceneTextContent::Path(path) => {
                    push_json_pair(out, "source", "path");
                    out.push(',');
                    push_json_pair(out, "path", &path.join("."));
                }
            }
        }
        SceneComponent::Button(button) => {
            push_json_pair(out, "kind", "button");
            out.push(',');
            push_json_expr_named(out, "label", &button.label);
            out.push(',');
            push_json_effect(out, &button.effect);
        }
        SceneComponent::Row(container) => {
            push_json_pair(out, "kind", "row");
            out.push(',');
            push_scene_layout(out, &container.layout);
            out.push(',');
            push_scene_children(out, &container.children);
        }
        SceneComponent::Column(container) => {
            push_json_pair(out, "kind", "column");
            out.push(',');
            push_scene_layout(out, &container.layout);
            out.push(',');
            push_scene_children(out, &container.children);
        }
        SceneComponent::Box(container) => {
            push_json_pair(out, "kind", "box");
            out.push(',');
            push_scene_layout(out, &container.layout);
            out.push(',');
            push_scene_children(out, &container.children);
        }
        SceneComponent::For(for_view) => {
            push_json_pair(out, "kind", "for");
            out.push(',');
            push_json_pair(out, "binding", &for_view.binding);
            out.push(',');
            push_json_pair(out, "source", for_view.source.as_str());
            out.push(',');
            push_scene_children(out, &for_view.children);
        }
        SceneComponent::LevelMenu(menu) => {
            push_json_pair(out, "kind", "level_menu");
            out.push(',');
            push_json_bool(out, "showIndex", menu.show_index);
            out.push(',');
            push_json_bool(out, "showCleared", menu.show_cleared);
            out.push(',');
            match menu.columns {
                Some(columns) => push_json_number(out, "columns", columns as u64),
                None => out.push_str("\"columns\":null"),
            }
            out.push(',');
            push_json_bool(out, "wrap", menu.wrap);
            out.push(',');
            out.push_str("\"source\":");
            if let Some(source) = &menu.source {
                push_json_string(out, source);
            } else {
                out.push_str("null");
            }
            out.push(',');
            out.push_str("\"action\":");
            if let Some(effect) = &menu.action {
                out.push('{');
                push_json_effect(out, effect);
                out.push('}');
            } else {
                out.push_str("null");
            }
            out.push(',');
            out.push_str("\"buttons\":[");
            for (index, button) in menu.buttons.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('{');
                push_json_expr_named(out, "label", &button.label);
                out.push(',');
                push_json_effect(out, &button.effect);
                out.push('}');
            }
            out.push(']');
        }
        SceneComponent::Menu(menu) => {
            push_json_pair(out, "kind", "menu");
            out.push(',');
            push_json_pair(out, "name", &menu.name);
            out.push(',');
            push_json_pair(out, "menu", &menu.menu);
            out.push(',');
            out.push_str("\"data\":{");
            for (index, binding) in menu.data.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_string(out, &binding.name);
                out.push(':');
                push_json_expr_object(out, &binding.value);
            }
            out.push('}');
        }
    }
    out.push('}');
}

fn push_json_effect(out: &mut String, effect: &SceneEffect) {
    out.push_str("\"effect\":{");
    push_json_effect_fields(out, effect);
    out.push('}');
}

fn push_json_effect_fields(out: &mut String, effect: &SceneEffect) {
    match effect {
        SceneEffect::Input(input) => {
            push_json_pair(out, "kind", "input");
            out.push(',');
            push_json_pair(out, "name", input);
        }
        SceneEffect::ComponentEffect(effect) => {
            push_json_pair(out, "kind", "component_effect");
            out.push(',');
            push_json_pair(out, "name", effect);
        }
        SceneEffect::Message { text } => {
            push_json_pair(out, "kind", "message");
            out.push(',');
            push_json_expr_named(out, "text", text);
        }
        SceneEffect::Wait { milliseconds } => {
            push_json_pair(out, "kind", "wait");
            out.push(',');
            push_json_number(out, "milliseconds", milliseconds.unwrap_or(200));
        }
        SceneEffect::Conditional { condition, effect } => {
            push_json_pair(out, "kind", "conditional");
            out.push(',');
            push_json_pair(out, "condition", condition);
            out.push(',');
            out.push_str("\"effect\":{");
            push_json_effect_fields(out, effect);
            out.push('}');
        }
        SceneEffect::PlaySfx { name } => {
            push_json_pair(out, "kind", "play_sfx");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        SceneEffect::PlayMusic { name } => {
            push_json_pair(out, "kind", "play_music");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        SceneEffect::PauseMusic { name } => {
            push_json_pair(out, "kind", "pause_music");
            out.push(',');
            out.push_str("\"name\":");
            if let Some(name) = name {
                push_json_string(out, name);
            } else {
                out.push_str("null");
            }
        }
        SceneEffect::ResumeMusic { name } => {
            push_json_pair(out, "kind", "resume_music");
            out.push(',');
            out.push_str("\"name\":");
            if let Some(name) = name {
                push_json_string(out, name);
            } else {
                out.push_str("null");
            }
        }
        SceneEffect::StopMusic { name } => {
            push_json_pair(out, "kind", "stop_music");
            out.push(',');
            out.push_str("\"name\":");
            if let Some(name) = name {
                push_json_string(out, name);
            } else {
                out.push_str("null");
            }
        }
        SceneEffect::Goto { scene, params } => {
            push_json_pair(out, "kind", "goto");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
            out.push(',');
            push_json_params(out, params);
        }
        SceneEffect::Enter { scene, params } => {
            push_json_pair(out, "kind", "enter");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
            out.push(',');
            push_json_params(out, params);
        }
        SceneEffect::Back => {
            push_json_pair(out, "kind", "back");
        }
        SceneEffect::Create { scene } => {
            push_json_pair(out, "kind", "create");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Reset { scene } => {
            push_json_pair(out, "kind", "reset");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Delete { scene } => {
            push_json_pair(out, "kind", "delete");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Show { scene } => {
            push_json_pair(out, "kind", "show");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Hide { scene } => {
            push_json_pair(out, "kind", "hide");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Toggle { scene } => {
            push_json_pair(out, "kind", "toggle");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::Focus { scene } => {
            push_json_pair(out, "kind", "focus");
            out.push(',');
            push_json_pair(out, "screen", scene);
            out.push(',');
            push_json_pair(out, "scene", scene);
        }
        SceneEffect::StartLevel { scene, scope } => {
            push_json_pair(out, "kind", "start_level");
            out.push(',');
            push_json_pair(out, "scene", scene);
            if let Some(scope) = scope {
                out.push(',');
                push_json_pair(out, "scope", scope);
            }
        }
        SceneEffect::ContinueLevel { scene, scope } => {
            push_json_pair(out, "kind", "continue_level");
            out.push(',');
            push_json_pair(out, "scene", scene);
            if let Some(scope) = scope {
                out.push(',');
                push_json_pair(out, "scope", scope);
            }
        }
        SceneEffect::PuzzleNextLevel { target } => {
            push_json_pair(out, "kind", "puzzle_next_level");
            out.push(',');
            push_json_pair(out, "target", target);
        }
        SceneEffect::PuzzlePreviousLevel { target } => {
            push_json_pair(out, "kind", "puzzle_previous_level");
            out.push(',');
            push_json_pair(out, "target", target);
        }
        SceneEffect::GotoLevel { target, level } => {
            push_json_pair(out, "kind", "puzzle_goto_level");
            out.push(',');
            push_json_pair(out, "target", target);
            out.push(',');
            push_json_expr_named(out, "level", level);
        }
        SceneEffect::ResetPuzzle { target } => {
            push_json_pair(out, "kind", "puzzle_reset");
            out.push(',');
            push_json_pair(out, "target", target);
        }
        SceneEffect::LoadPuzzle { target, source } => {
            push_json_pair(out, "kind", "puzzle_load");
            out.push(',');
            push_json_pair(out, "target", target);
            out.push(',');
            push_json_pair(out, "source", source);
        }
        SceneEffect::Apply { rule, args, target } => {
            push_json_pair(out, "kind", "apply");
            out.push(',');
            push_json_pair(out, "rule", rule);
            out.push(',');
            out.push_str("\"args\":[");
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_expr_object(out, arg);
            }
            out.push(']');
            if let Some(target) = target {
                out.push(',');
                push_json_pair(out, "target", target);
            }
        }
        SceneEffect::Copy { source, target } => {
            push_json_pair(out, "kind", "copy");
            out.push(',');
            push_json_pair(out, "source", source);
            out.push(',');
            push_json_pair(out, "target", target);
        }
        SceneEffect::ClearHistory => {
            push_json_pair(out, "kind", "clear_history");
        }
        SceneEffect::Sequence(effects) => {
            push_json_pair(out, "kind", "sequence");
            out.push(',');
            out.push_str("\"effects\":[");
            for (index, effect) in effects.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('{');
                push_json_effect(out, effect);
                out.push('}');
            }
            out.push(']');
        }
    }
}

fn push_json_params(out: &mut String, params: &[puzzle_lang::SceneEffectParam]) {
    out.push_str("\"params\":[");
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &param.name);
        out.push(',');
        push_json_expr(out, &param.value);
        out.push('}');
    }
    out.push(']');
}

fn push_json_expr(out: &mut String, expr: &SceneExpr) {
    push_json_expr_named(out, "value", expr);
}

fn push_json_expr_object(out: &mut String, expr: &SceneExpr) {
    out.push('{');
    push_json_expr_fields(out, expr);
    out.push('}');
}

fn push_json_expr_named(out: &mut String, name: &str, expr: &SceneExpr) {
    push_json_string(out, name);
    out.push_str(":{");
    push_json_expr_fields(out, expr);
    out.push('}');
}

fn push_json_expr_fields(out: &mut String, expr: &SceneExpr) {
    match expr {
        SceneExpr::Bool(value) => {
            push_json_pair(out, "kind", "bool");
            out.push(',');
            push_json_bool(out, "value", *value);
        }
        SceneExpr::Int(value) => {
            push_json_pair(out, "kind", "int");
            out.push(',');
            out.push_str("\"value\":");
            out.push_str(&value.to_string());
        }
        SceneExpr::Text(value) => {
            push_json_pair(out, "kind", "text");
            out.push(',');
            push_json_pair(out, "value", value);
        }
        SceneExpr::Path(path) => {
            push_json_pair(out, "kind", "path");
            out.push(',');
            push_json_pair(out, "path", &path.join("."));
        }
        SceneExpr::Call { name, args } => {
            push_json_pair(out, "kind", "call");
            out.push(',');
            push_json_pair(out, "name", name);
            out.push(',');
            out.push_str("\"args\":[");
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json_expr_object(out, arg);
            }
            out.push(']');
        }
    }
}

fn push_scene_children(out: &mut String, children: &[SceneComponent]) {
    out.push_str("\"children\":[");
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_scene_component(out, child);
    }
    out.push(']');
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
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

fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_number(out: &mut String, key: &str, value: u64) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_i64(out: &mut String, key: &str, value: i64) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_f64(out: &mut String, key: &str, value: f64) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_bool(out: &mut String, key: &str, value: bool) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn escape_script_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn escape_script(value: &str) -> String {
    value.replace("</script", "<\\/script")
}

fn escape_style(value: &str) -> String {
    value.replace("</style", "<\\/style")
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[i + 1..i + 3], 16) {
                decoded.push(hex);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(not(target_arch = "wasm32"))]
fn http_ok(content_type: &str, body: &str) -> String {
    http_response(200, "OK", content_type, body)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_error(status: u16, message: &str) -> String {
    let mut body = String::new();
    body.push_str("{\"error\":");
    push_json_string(&mut body, message);
    body.push('}');
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    http_response(status, reason, "application/json; charset=utf-8", &body)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_response(status: u16, reason: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[derive(Debug)]
enum AppError {
    #[cfg(not(target_arch = "wasm32"))]
    Io(io::Error),
    Lang(puzzle_lang::AppError),
    CoreTransition(puzzle_core::TransitionError),
    Config(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<puzzle_lang::AppError> for AppError {
    fn from(value: puzzle_lang::AppError) -> Self {
        Self::Lang(value)
    }
}

impl From<puzzle_core::TransitionError> for AppError {
    fn from(value: puzzle_core::TransitionError) -> Self {
        Self::CoreTransition(value)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn renderer_board_floor_is_transparent_by_default() {
        assert!(RENDERER_CSS.contains("--cell-background: transparent;"));
        assert!(RENDERER_JS.contains("floorColor && floorColor !== \"transparent\""));
    }

    #[test]
    fn html_play_fits_the_logical_scene_root_not_individual_cells() {
        assert!(INDEX_HTML.contains(r#"<div id="screenFrame" class="screen-frame">"#));
        assert!(APP_CSS.contains("--scene-layout-unit: 180px;"));
        assert!(APP_CSS.contains("--scene-layout-gap-unit: 1px;"));
        assert!(APP_JS.contains("function syncScreenScale()"));
        assert!(APP_JS.contains("const defaultSceneLogicalSize = { width: 4, height: 3 };"));
        assert!(APP_JS.contains("function fitLogicalSceneSize("));
        assert!(APP_JS.contains("const defaultSceneLayoutUnit = 180;"));
        assert!(APP_JS.contains("function virtualSceneSize("));
        assert!(APP_JS.contains("screenView.style.setProperty(\"--screen-scale\""));
        assert!(APP_JS.contains("screenFrame.style.width"));
        assert!(APP_JS.contains("screenFrame.style.height"));
        assert!(APP_CSS.contains("transform: scale(var(--screen-scale, 1));"));
        assert!(APP_CSS.contains("grid-auto-rows: max-content;"));
        assert!(APP_CSS.contains("place-content: center;"));
        assert!(APP_CSS.contains("justify-content: center;"));
        assert!(APP_JS.contains("function markSingleFrameComponentLayer("));
        assert!(APP_JS.contains("function fitPuzzleFrameComponents("));
        assert!(APP_JS.contains("Math.min(frame.width / cols, frame.height / rows)"));
        assert!(APP_JS.contains(r#"root.dataset.frameComponent = "true";"#));
        assert!(APP_CSS.contains(".scene-layer.has-single-frame-component"));
        assert!(APP_CSS.contains("grid-template: minmax(0, 1fr) / minmax(0, 1fr);"));
        assert!(APP_CSS.contains(
            ".scene-layer.has-single-frame-component > .board[data-frame-component=\"true\"]"
        ));
        assert!(!APP_JS.contains(r#""has-puzzle-scene""#));
        assert!(!APP_CSS.contains(".scene-layer.has-puzzle-scene"));
        assert!(!APP_CSS.contains("justify-content: space-between;"));
        assert!(APP_JS.contains(r#"renderMode: "canvas""#));
        assert!(
            RENDERER_CSS.contains("grid-template-columns: repeat(var(--cols), var(--cell-size));")
        );
        assert!(RENDERER_JS.contains("this.root.style.setProperty(\"--rows\", viewport.height);"));
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
    fn puzzle3_app_exposes_editor_preview_update_contract() {
        assert!(PUZZLE3_APP_JS.contains("function applyPuzzle3PreviewUpdate(update = {})"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioUpdatePuzzle3Preview"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioRenderPuzzle3ModelComponent"));
        assert!(PUZZLE3_APP_JS.contains("PuzzleStudioInitialModelComponentPreview"));
        assert!(
            PUZZLE3_APP_JS
                .contains("function applyPuzzle3ModelComponentPreviewUpdate(update = {})")
        );
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
        assert!(PUZZLE3_APP_JS.contains("next.camera = cloneCamera(update.camera);"));
        assert!(PUZZLE3_APP_JS.contains("next.settings = mergePuzzle3PreviewSettings"));
        assert!(PUZZLE3_APP_JS.contains(r#"coordinateSpace: "canvas-css-px""#));
    }

    #[test]
    fn mixed_export_hosts_puzzle3_as_scene_component() {
        let source = r#"
title Mixed Game

puzzle flat {
layers 1
empty .
object Player 0
rules {

}
}

levels flat_levels of flat {
legend P = Player
level start {
P
}
}

puzzle3 cube {
  layers {
    actor = Player Box Wall
  }

  group solid = Player Box Wall

  rules {

  }
}

levels3 cube_levels of cube {
  legend {
    . = empty
    P = Player
  }

  level start {
    P
  }
}

scene mixed_play {
  state {
    flat_board = puzzle flat
    cube_board = puzzle3 cube
  }
  view size 4 3 {
    row {
      puzzle flat_board
      puzzle3 cube_board
    }
  }
}
"#;
        Puzzle3RuntimeBridge::from_source(source).expect("mixed source should expose a 3D runtime");
        let document = puzzle_lang::parse_game(source).unwrap();
        let loaded = mixed_document_loaded_game(&document).unwrap();
        let mixed_scene = loaded
            .scenes
            .iter()
            .find(|scene| scene.name == "mixed_play")
            .unwrap();
        assert_eq!(mixed_scene.state.puzzles.len(), 1);
        assert_eq!(mixed_scene.state.puzzles[0].name, "flat_board");

        let html = export_mixed_document_html(
            &document,
            loaded,
            source.to_string(),
            "mixed.puzzle".to_string(),
            String::new(),
            VISUALS_JS.to_string(),
            SolverConfig::default(),
        )
        .unwrap();
        assert!(html.contains("window.Puzzle3DFrameFixture"));
        assert!(html.contains("window.Puzzle3DFrameAssets"));
        assert!(html.contains("case \"puzzle3\""));
        assert!(html.contains("iframe.puzzle3-frame"));
        assert!(html.contains("\\\"kind\\\":\\\"puzzle3\\\""));
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
        assert!(PUZZLE3_APP_JS.contains("primitives.sort(comparePrimitiveOrder);"));
        assert!(PUZZLE3_APP_JS.contains("return Puzzle3VisualCore.comparePrimitiveOrder(a, b);"));
        assert!(
            PUZZLE3_APP_JS
                .contains("return Puzzle3VisualCore.faceGridOrder(corners, puzzle3VisualView());")
        );
    }

    #[test]
    fn standalone_until_stable_skips_idempotent_matches() {
        assert!(STANDALONE_JS.contains("const placements = this.findAllMatches(current, rule);"));
        assert!(STANDALONE_JS.contains("const ruleCommands = this.ruleCommands(rule);"));
        assert!(STANDALONE_JS.contains("if (key === currentKey)"));
        assert!(STANDALONE_JS.contains(
            "console.warn(`until-stable cycle in rule ${rule.id}; ending repeat at current state`)"
        ));
        assert!(STANDALONE_JS.contains(
            "console.warn(\"until-stable block cycle; ending repeat at current state\")"
        ));
        assert!(STANDALONE_JS.contains("const UNTIL_STABLE_REPEAT_LIMIT = 200;"));
        assert!(STANDALONE_JS.contains("continue;"));
    }

    #[test]
    fn standalone_again_turns_are_scheduled_between_snapshots() {
        assert!(
            STANDALONE_JS
                .contains("this.defaultAgainMs = Number(exportData.defaultAgainMs ?? 120);")
        );
        assert!(STANDALONE_JS.contains("this.scheduleAgainTurn(target, 0, token);"));
        assert!(STANDALONE_JS.contains("setTimeout(() => {"));
        assert!(STANDALONE_JS.contains("this.notifyStateChanged();"));
        assert!(STANDALONE_JS.contains("this.runAgainTurn(target);"));
        assert!(STANDALONE_JS.contains("(this.pendingAgainTurns || 0) > 0"));
    }

    #[test]
    fn standalone_display_projection_errors_warn_and_fallback() {
        assert!(STANDALONE_JS.contains("materializeDisplayProgram(program, state, programKey"));
        assert!(STANDALONE_JS.contains("projection failed; using source state"));
        assert!(STANDALONE_JS.contains("return this.cloneState(state);"));
        assert!(STANDALONE_JS.contains("const presentation = focusedPuzzle"));
        assert!(STANDALONE_JS.contains("presentationSnapshotForState(state, options = {})"));
        assert!(
            APP_JS.contains("screenHasPuzzle: currentSceneHasPuzzle() || Boolean(state.scene)")
        );
        assert!(APP_JS.contains("if (!currentSceneHasPuzzle() && !currentState.scene)"));
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
        assert!(
            STANDALONE_JS
                .contains("this.currentSceneAcceptsModelInput() || this.editorPreviewInputEnabled")
        );
        assert!(STANDALONE_JS.contains(
            "(options.materializeDisplay || options.materializeTurnStart) && options.acceptModelInput !== true"
        ));
        assert!(STANDALONE_JS.contains(
            "this.materializeDisplayProgram(displayProgram, state, \"display_level_clear\")"
        ));
        assert!(
            STANDALONE_JS
                .contains("this.materializeDisplayProgram(displayProgram, state, \"display\")")
        );
    }

    #[test]
    fn solver_solution_steps_materialize_display_objects_for_display() {
        let source = r#"
title display_solver

puzzle board {
  layers {
    actor = Player
    cursor = @Cursor
  }
  empty .
  rules {
    [ Player ] -> [ Player ]
  }
  routine @paint once {
    [ Player no @Cursor ] -> [ Player @Cursor ]
  }
  on_display {
    @paint
  }
  win_conditions {
    some Player
  }
}

levels default of board {
  legend P = Player
  level start {
    P
  }
}

scene playing {
  view {
    puzzle board
  }
}
"#;

        let loaded = parse_game(source).unwrap();
        let mut state_json = String::new();
        push_state_data(&mut state_json, &loaded.levels[0].initial_state);

        let response =
            solve_state_json_from_source(source, "game.puzzle", &state_json, 8, 1000, 0).unwrap();

        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""object":"@Cursor""#));
    }

    #[test]
    fn solver_materializes_level_start_for_editor_state_with_level_index() {
        let source = r#"
title solver_level_start

puzzle board {
  layers {
    floor = Goal
    actor = Player
  }
  inputs {
    noop <- Space
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
  level start {
    PG
  }
}

scene playing {
  view {
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

    #[test]
    fn solver_accepts_puzzle3d_state_and_returns_replay_steps() {
        let source = r#"
puzzle3 push3 {
layers {
floor = Goal
solid = Player Box Wall
}

inputs {
right <- d ArrowRight
restart <- r
}

group solid = Player Box Wall

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
}

levels3 tiny of push3 {
legend {
. = empty
P = Player
B = Box
G = Goal
}

level one {
PB.

..G
}
}
"#;

        let parsed = puzzle3d_model::parse_puzzle3d(source).unwrap();
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
            solve_state_json_from_source(source, "game.puzzle", &state_json, 4, 1000, 0).unwrap();

        assert!(response.contains(r#""model":"puzzle3d""#));
        assert!(response.contains(r#""result":"solved""#));
        assert!(response.contains(r#""name":"right""#));
        assert!(response.contains(r#""direction":"right""#));
        assert!(response.contains(r#""completed":true"#));
        assert!(response.contains(r#""clearCommands":["next_level"]"#));
        assert!(response.contains(r#""scene":{"kind":"puzzle3d""#));
        assert!(!response.contains(r#""name":"restart""#));
    }

    #[test]
    fn core_runtime_bridge_uses_core_once_all_semantics() {
        let source = r#"
title once_all_overlap

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
  level start {
    AAA
  }
}

scene playing {
  view {
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

        assert!(outcome.contains(r#""slots":[2,0,1]"#));
    }

    #[test]
    fn standalone_export_embeds_core_wasm_runtime() {
        let source = r#"
title Wasm Export

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
  level one {
    P
  }
}

scene playing {
  view {
    puzzle board
  }
}
"#;

        let html = export_html_from_source(source, "games/wasm_export/game.puzzle", "", "")
            .expect("export should succeed");

        assert!(html.contains("window.PuzzleStandaloneEmbeddedWasm"));
        assert!(html.contains("WasmCoreRuntime"));
        assert!(STANDALONE_JS.contains("coreTransitionProgramOutcome("));
        assert!(STANDALONE_JS.contains("transition_program_outcome("));
    }

    #[test]
    fn standalone_export_includes_scene_and_screen_keys() {
        let source = r#"
title Export Test

puzzle default {
layers 1
empty .
object Player 0

levels {
    legend P = Player

    level one
    P
}

rules {
    [ Player ] -> [ Player ]
}
}

scene playing {
    state {
        board = puzzle default
    }
    view {
        puzzle board
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

        assert_eq!(data.matches("\"scenes\":[").count(), 1);
        assert_eq!(data.matches("\"screens\":[").count(), 1);
        assert!(data.contains("\"persistentVars\":["));
    }

    #[test]
    fn standalone_export_resolves_viewport_focus_group_objects() {
        let source = r#"
title Flickscreen Focus

puzzle default {
layers {
  floor = Background
  actor = Player1 Player2
}
empty .
group player = Player1 Player2
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
title Theme Startup
theme noir {
  background_color #123456
}

puzzle default {
layers 1
empty .
object Player 0

levels {
    legend P = Player

    level one
    P
}

rules {
    [ Player ] -> [ Player ]
}
}
"##;
        let html = export_html_from_source(source, "games/theme_startup/game.puzzle", "", "")
            .expect("export themed document");

        assert!(html.contains(r#"<body class="theme-noir" style="--bg:#123456;">"#));
    }

    #[test]
    fn standalone_export_supports_single_puzzle3_document() {
        let source = include_str!("../../puzzle3d_model/games/sokoban_literally_in_3d.puzzle");
        let html = export_html_from_source(
            source,
            "games/spec_3d.puzzle",
            "body { --accent: #123456; }",
            "",
        )
        .expect("export puzzle3 document");

        assert!(html.contains("window.Puzzle3DFixture"));
        assert!(html.contains("WasmPuzzle3Runtime"));
        assert!(!html.contains("Puzzle3DTestRuntime"));
        assert!(html.contains("Microban Basic 3D"));
        assert!(html.contains("--accent: #123456"));
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
