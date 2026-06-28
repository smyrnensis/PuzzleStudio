#![cfg_attr(any(target_arch = "wasm32", not(feature = "solver")), allow(dead_code))]

#[cfg(feature = "solver")]
use std::collections::BTreeSet;
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

use puzzle_3d::{
    Coord3, Game3, InputId3, LifecycleCommand3, ObjectId as ObjectId3, ParsedPuzzle3, RuleId3,
    Size3, State3, transition_program_with_local_frame as transition_program_with_local_frame3,
    transition_program_without_input_with_local_frame,
};
#[cfg(feature = "solver")]
use puzzle_3d::{Rule3, WinCondition3, transition_program as transition_program3};
#[cfg(feature = "solver")]
use puzzle_core::transition_state;
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionValueKind, Effect, GlobalUpdateOp, Guard, InputId,
    LayerId, ObjectId, Offset, Patch, PatchOp, Pattern, Rule, RuleApplication, RuleCondition,
    RuleId, RuleStep, ScratchPattern, ScratchValueMatch, State, TransitionCommand, WriteOp,
    transition_program, transition_program_outcome, transition_program_trace,
};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::AssetsDef;
use puzzle_lang::{
    AnimationDef, ArrowKey, GoalCondition, GoalExpr, GoalValue, KeyTrigger, Level,
    LoadedDocumentModel, LoadedGame, ResourceSelection, RuleAnimation, RuleAnimationTrigger,
    RuleEffect, SceneAlignXDef, SceneAlignYDef, SceneComponent, SceneDef, SceneEffect, SceneExpr,
    SceneLayoutDef, ScenePuzzleInitializer, SceneTextContent, SceneTransitionTrigger, SceneValue,
    SoundsDef, ThemeDef, VisualSpriteDef, VisualSpriteKind, parse_game2d as parse_game,
};
use puzzle_lang::{AssetKind, DiagnosticReport};
#[cfg(not(target_arch = "wasm32"))]
use puzzle_lang::{discover_game_entries, expand_game_imports_for_file, resolve_game_entry};
use puzzle_play::{
    AnimationEvent, GameSession, LevelProgressSaveData, MessageEvent, PersistentVarSaveData,
    ProgressSaveData, SoundEvent, WaitEvent, animation_events_for_trace, runtime_sounds_def,
};
#[cfg(feature = "solver")]
use puzzle_solver::{
    Puzzle3Domain, PuzzleDomain, SearchBudget, SearchOutcome, SearchProgress, SearchStats,
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
const PUZZLE3_SCENE_HOST_SOURCE: &str = r#"
title "__puzzle3_scene_host__"

puzzle scene_host {
layers {
  marker = Marker
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
const SEEDED_SFX_JS: &str = include_str!("../../../tools/music_generator/seeded_sfx.mjs");
const SEEDED_MUSIC_JS: &str = include_str!("../../../tools/music_generator/seeded_music.mjs");
const SEEDED_MUSIC_PLAYER_JS: &str =
    include_str!("../../../tools/music_generator/seeded_music_player.mjs");
const SEEDED_TIMBRE_FIELDS_JS: &str =
    include_str!("../../../tools/music_generator/seeded_timbre_fields.mjs");

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
    puzzle_lang::validate_source_profile_for_path(&source, &config.puzzle_path)?;
    let profile = puzzle_lang::puzzle_source_profile_for_path(&config.puzzle_path)
        .ok_or_else(|| AppError::Config("game entry must be .puzzle or .puzzle3".to_string()))?;
    let source = match profile {
        puzzle_lang::PuzzleSourceProfile::Puzzle3d => source,
        puzzle_lang::PuzzleSourceProfile::Puzzle2d => {
            let expanded = expand_game_imports_for_file(&source, &config.puzzle_path)?;
            puzzle_lang::validate_source_profile_for_path(&expanded, &config.puzzle_path)?;
            expanded
        }
    };

    if profile == puzzle_lang::PuzzleSourceProfile::Puzzle3d {
        let document = puzzle_lang::parse_game_for_path(&source, &config.puzzle_path)
            .map_err(AppError::Lang)?;
        let game_css = load_asset_css(&config.puzzle_path, &document.assets)?;
        let output_path = config.output_path();
        let puzzle_path = config.puzzle_path.display().to_string();
        let html =
            export_puzzle3_document_html(&document, &source, &puzzle_path, &game_css, VISUALS_JS)
                .map_err(AppError::Config)?;
        if let Some(screenshot) = &config.screenshot {
            let scene = screenshot
                .scene
                .clone()
                .or_else(|| default_puzzle3_screenshot_scene(&document));
            capture_html_screenshot(&html, &screenshot.output_path, scene.as_deref(), screenshot)?;
            println!("screenshot {}", screenshot.output_path.display());
            return Ok(());
        }
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
        if let Some(screenshot) = &config.screenshot {
            capture_html_screenshot(
                &export_html(&state),
                &screenshot.output_path,
                screenshot.scene.as_deref(),
                screenshot,
            )?;
            println!("screenshot {}", screenshot.output_path.display());
            return Ok(());
        }
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
    print_wasm_freshness_status();

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
fn print_wasm_freshness_status() {
    print_wasm_artifact_status(
        "puzzle_wasm_game",
        &[
            Path::new("crates/html_play/static/wasm_game/puzzle_wasm_game.js"),
            Path::new("crates/html_play/static/wasm_game/puzzle_wasm_game_bg.wasm"),
        ],
        &[
            Path::new("crates/wasm_game/src"),
            Path::new("crates/wasm_game/Cargo.toml"),
            Path::new("crates/html_play/src"),
            Path::new("crates/core/src"),
            Path::new("crates/lang/src"),
            Path::new("crates/play/src"),
            Path::new("crates/puzzle_3d/src"),
            Path::new("crates/scene/src"),
            Path::new("crates/kernel/src"),
            Path::new("Cargo.lock"),
        ],
        "tools/build_wasm_game.sh",
    );
    print_wasm_artifact_status(
        "puzzle_core_wasm",
        &[
            Path::new("crates/wasm_core/static/puzzle_core_wasm.js"),
            Path::new("crates/wasm_core/static/puzzle_core_wasm_bg.wasm"),
        ],
        &[
            Path::new("crates/wasm_core/src"),
            Path::new("crates/wasm_core/Cargo.toml"),
            Path::new("crates/core/src"),
            Path::new("crates/kernel/src"),
            Path::new("Cargo.lock"),
        ],
        "tools/build_wasm_core.sh",
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn print_wasm_artifact_status(
    name: &str,
    artifacts: &[&Path],
    sources: &[&Path],
    rebuild_command: &str,
) {
    let Some(artifact_time) = oldest_existing_mtime(artifacts) else {
        eprintln!("warning: wasm: {name} artifacts are missing; run `{rebuild_command}`");
        return;
    };
    let Some(source_time) = newest_existing_mtime(sources) else {
        println!("wasm: {name} current (source freshness unknown)");
        return;
    };
    if source_time > artifact_time {
        eprintln!("warning: wasm: {name} may be stale; run `{rebuild_command}`");
    } else {
        println!("wasm: {name} current");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn oldest_existing_mtime(paths: &[&Path]) -> Option<SystemTime> {
    let mut oldest = None;
    for path in paths {
        let modified = fs::metadata(path).ok()?.modified().ok()?;
        oldest = Some(match oldest {
            Some(current) if current < modified => current,
            _ => modified,
        });
    }
    oldest
}

#[cfg(not(target_arch = "wasm32"))]
fn newest_existing_mtime(paths: &[&Path]) -> Option<SystemTime> {
    let mut newest = None;
    for path in paths {
        collect_newest_mtime(path, &mut newest);
    }
    newest
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_newest_mtime(path: &Path, newest: &mut Option<SystemTime>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        if let Ok(modified) = metadata.modified() {
            *newest = Some(match *newest {
                Some(current) if current > modified => current,
                _ => modified,
            });
        }
        return;
    }
    if !metadata.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        collect_newest_mtime(&entry.path(), newest);
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
    screenshot: Option<ScreenshotConfig>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct ScreenshotConfig {
    output_path: PathBuf,
    scene: Option<String>,
    width: u32,
    height: u32,
    timeout_ms: u64,
    browser_path: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Config {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, AppError> {
        let mut puzzle_path = None;
        let mut output_path = None;
        let mut serve = false;
        let mut port = 7878;
        let mut solver = SolverConfig::default();
        let mut screenshot = None::<ScreenshotConfig>;
        let mut screenshot_scene = None::<String>;
        let mut screenshot_width = 1280_u32;
        let mut screenshot_height = 720_u32;
        let mut screenshot_timeout_ms = 5000_u64;
        let mut screenshot_browser_path = None::<PathBuf>;
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
                    parse_solver_depth_arg(&mut solver, &mut args)?;
                }
                "--solver-nodes" => {
                    parse_solver_nodes_arg(&mut solver, &mut args)?;
                }
                "--solver-ms" => {
                    parse_solver_ms_arg(&mut solver, &mut args)?;
                }
                "--screenshot" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config(
                            "--screenshot requires a PNG output path".to_string(),
                        ));
                    };
                    screenshot = Some(ScreenshotConfig {
                        output_path: PathBuf::from(value),
                        scene: None,
                        width: screenshot_width,
                        height: screenshot_height,
                        timeout_ms: screenshot_timeout_ms,
                        browser_path: screenshot_browser_path.clone(),
                    });
                }
                "--scene" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config(
                            "--scene requires a scene name".to_string(),
                        ));
                    };
                    screenshot_scene = Some(value);
                }
                "--width" => {
                    screenshot_width = parse_arg(&mut args, "--width")?;
                }
                "--height" => {
                    screenshot_height = parse_arg(&mut args, "--height")?;
                }
                "--screenshot-timeout-ms" => {
                    screenshot_timeout_ms = parse_arg(&mut args, "--screenshot-timeout-ms")?;
                }
                "--browser" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config(
                            "--browser requires a browser executable path".to_string(),
                        ));
                    };
                    screenshot_browser_path = Some(PathBuf::from(value));
                }
                "--help" | "-h" => {
                    return Err(AppError::Config(
                        "usage: html-play [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [-o game.html] [--serve] [--port 7878] [--screenshot out.png] [--scene name] [--width 1280] [--height 720] [--browser path] [--solver-depth 128] [--solver-nodes 1000000] [--solver-ms 5000]".to_string(),
                    ));
                }
                value => puzzle_path = Some(PathBuf::from(value)),
            }
        }

        if screenshot_width == 0 || screenshot_height == 0 {
            return Err(AppError::Config(
                "screenshot width and height must be positive".to_string(),
            ));
        }
        if screenshot_timeout_ms == 0 {
            return Err(AppError::Config(
                "screenshot timeout must be positive".to_string(),
            ));
        }
        if let Some(config) = screenshot.as_mut() {
            config.scene = screenshot_scene;
            config.width = screenshot_width;
            config.height = screenshot_height;
            config.timeout_ms = screenshot_timeout_ms;
            config.browser_path = screenshot_browser_path;
            serve = false;
        } else if screenshot_scene.is_some() || screenshot_browser_path.is_some() {
            return Err(AppError::Config(
                "--scene and --browser are only valid with --screenshot".to_string(),
            ));
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
            screenshot,
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
fn capture_html_screenshot(
    html: &str,
    output_path: &Path,
    scene: Option<&str>,
    config: &ScreenshotConfig,
) -> Result<(), AppError> {
    let browser_path = resolve_screenshot_browser(config.browser_path.as_deref())?;
    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    if output_path.exists() {
        fs::remove_file(output_path)?;
    }

    let temp_root = env::temp_dir();
    let stem = format!("puzzlestudio-screenshot-{}", std::process::id());
    let html_path = temp_root.join(format!("{stem}.html"));
    let profile_path = temp_root.join(format!("{stem}-chrome-profile"));
    if profile_path.exists() {
        let _ = fs::remove_dir_all(&profile_path);
    }
    fs::create_dir_all(&profile_path)?;
    fs::write(&html_path, html)?;

    let mut url = file_url(&html_path);
    if let Some(scene) = scene.filter(|scene| !scene.is_empty()) {
        url.push_str("?scene=");
        url.push_str(&url_condition_value(scene));
    }

    let screenshot_arg = format!("--screenshot={}", output_path.display());
    let user_data_arg = format!("--user-data-dir={}", profile_path.display());
    let window_size_arg = format!("--window-size={},{}", config.width, config.height);
    let timeout_arg = format!("--timeout={}", config.timeout_ms);
    let mut child = Command::new(&browser_path)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-networking")
        .arg("--disable-component-update")
        .arg("--disable-sync")
        .arg("--disable-extensions")
        .arg("--disable-features=MediaRouter,OptimizationHints")
        .arg(user_data_arg)
        .arg(window_size_arg)
        .arg(timeout_arg)
        .arg(screenshot_arg)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            AppError::Config(format!(
                "failed to launch screenshot browser {}: {error}",
                browser_path.display()
            ))
        })?;

    let deadline = Instant::now() + Duration::from_millis(config.timeout_ms + 10_000);
    let mut result = Err(AppError::Config(format!(
        "screenshot timed out before writing {}",
        output_path.display()
    )));
    loop {
        if output_path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
        {
            std::thread::sleep(Duration::from_millis(200));
            result = Ok(());
            break;
        }
        if let Some(status) = child.try_wait()? {
            result = Err(AppError::Config(format!(
                "screenshot browser exited with {status} before writing {}",
                output_path.display()
            )));
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if child.try_wait()?.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = fs::remove_file(&html_path);
    let _ = fs::remove_dir_all(&profile_path);
    result
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_screenshot_browser(explicit: Option<&Path>) -> Result<PathBuf, AppError> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        return Err(AppError::Config(format!(
            "screenshot browser not found: {}",
            path.display()
        )));
    }
    if let Ok(value) = env::var("PUZZLESTUDIO_CHROME") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Ok(path);
        }
    }
    for path in [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ] {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }
    for command in ["google-chrome", "chromium", "chromium-browser", "chrome"] {
        if let Some(path) = find_command_on_path(command) {
            return Ok(path);
        }
    }
    Err(AppError::Config(
        "screenshot requires Chrome or Chromium; install one, pass --browser, or set PUZZLESTUDIO_CHROME"
            .to_string(),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn find_command_on_path(command: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|path| path.join(command))
        .find(|path| path.is_file())
}

#[cfg(not(target_arch = "wasm32"))]
fn file_url(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut out = String::from("file://");
    for byte in path.to_string_lossy().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn url_condition_value(value: &str) -> String {
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
fn default_puzzle3_screenshot_scene(document: &puzzle_lang::LoadedDocument) -> Option<String> {
    document
        .scenes
        .iter()
        .find(|scene| {
            scene
                .components
                .iter()
                .any(component_contains_puzzle3_frame)
        })
        .map(|scene| scene.name.clone())
}

fn component_contains_puzzle3_frame(component: &SceneComponent) -> bool {
    match component {
        SceneComponent::Frame(frame) => frame.kind == "puzzle3",
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => container
            .children
            .iter()
            .any(component_contains_puzzle3_frame),
        SceneComponent::Conditional(conditional) => conditional
            .children
            .iter()
            .chain(conditional.else_children.iter())
            .any(component_contains_puzzle3_frame),
        SceneComponent::For(component) => component
            .children
            .iter()
            .any(component_contains_puzzle3_frame),
        _ => false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn discover_default_puzzle_path() -> Result<PathBuf, AppError> {
    let candidates =
        discover_game_entries("games").map_err(|error| AppError::Config(error.to_string()))?;
    match candidates.len() {
        0 => Err(AppError::Config(
            "no games/*/game.puzzle or games/*/game.puzzle3 entries found. Pass a path: html-play <path/to/game-folder-or-game.puzzle-or-game.puzzle3>"
                .to_string(),
        )),
        1 => Ok(candidates[0].clone()),
        _ => Err(AppError::Config(format!(
            "multiple game entries found. Pass one explicitly: {}",
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
        push_visual_sprite(&mut sprites, sprite);
    }
    sprites.push('}');

    format!(
        "(() => {{\n  const previous = window.GameVisuals || {{}};\n  const createVisuals = window.PuzzleSpriteRegistry?.create || ((config = {{}}) => ({{\n    aliases: {{ ...(config.aliases || {{}}) }},\n    sprites: {{ ...(config.sprites || {{}}) }},\n    boardClass: config.boardClass || \"\",\n    themeClass: config.themeClass || \"\",\n    editorPuzzle: {{ ...(config.editorPuzzle || {{}}) }},\n    autoAdvanceDelayMs: config.autoAdvanceDelayMs,\n  }}));\n  window.GameVisuals = createVisuals({{\n    ...previous,\n    aliases: {{ ...(previous.aliases || {{}}), ...{aliases} }},\n    sprites: {{ ...(previous.sprites || {{}}), ...{sprites} }},\n  }});\n}})();"
    )
}

fn push_visual_sprite(out: &mut String, sprite: &VisualSpriteDef) {
    match &sprite.kind {
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
    if sprite.offset.x != 0 || sprite.offset.y != 0 || sprite.pixels_per_cell.is_some() {
        out.pop();
        if sprite.offset.x != 0 || sprite.offset.y != 0 {
            out.push_str(",\"offset\":{\"x\":");
            out.push_str(&sprite.offset.x.to_string());
            out.push_str(",\"y\":");
            out.push_str(&sprite.offset.y.to_string());
            out.push('}');
        }
        if let Some(pixels_per_cell) = sprite.pixels_per_cell {
            out.push_str(",\"pixelsPerCell\":{\"width\":");
            out.push_str(&pixels_per_cell.width.to_string());
            out.push_str(",\"height\":");
            out.push_str(&pixels_per_cell.height.to_string());
            out.push('}');
        }
        out.push('}');
    }
}

#[derive(Clone, Copy, Debug)]
struct SolverConfig {
    #[cfg(feature = "solver")]
    max_depth: u32,
    #[cfg(feature = "solver")]
    max_nodes: usize,
    #[cfg(feature = "solver")]
    max_duration: Duration,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "solver")]
            max_depth: 128,
            #[cfg(feature = "solver")]
            max_nodes: 1_000_000,
            #[cfg(feature = "solver")]
            max_duration: Duration::from_secs(5),
        }
    }
}

#[cfg(feature = "solver")]
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

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_depth_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    solver.max_depth = parse_arg(args, "--solver-depth")?;
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_depth_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-depth requires the html-play solver feature".to_string(),
    ))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_nodes_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    solver.max_nodes = parse_arg(args, "--solver-nodes")?;
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_nodes_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-nodes requires the html-play solver feature".to_string(),
    ))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "solver"))]
fn parse_solver_ms_arg(
    solver: &mut SolverConfig,
    args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    let milliseconds: u64 = parse_arg(args, "--solver-ms")?;
    solver.max_duration = Duration::from_millis(milliseconds);
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "solver")))]
fn parse_solver_ms_arg(
    _solver: &mut SolverConfig,
    _args: &mut impl Iterator<Item = String>,
) -> Result<(), AppError> {
    Err(AppError::Config(
        "--solver-ms requires the html-play solver feature".to_string(),
    ))
}

struct ServerState {
    loaded: LoadedGame,
    session: GameSession,
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
    solver: SolverConfig,
    has_progress_save: bool,
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
            has_progress_save: false,
        }
    }

    fn snapshot_json(&mut self) -> String {
        let sound_events = self.session.take_sound_events();
        let message_events = self.session.take_message_events();
        let wait_events = self.session.take_wait_events();
        let animation_events = self.session.take_animation_events();
        let mut out = String::new();
        out.push('{');
        push_top_scope_context(&mut out, &self.loaded, self.has_progress_save);
        out.push(',');
        push_export_sounds(&mut out, &self.loaded.sounds);
        out.push(',');
        push_export_theme(&mut out, &self.loaded.theme);
        out.push(',');
        push_json_number(&mut out, "defaultWaitMs", self.loaded.default_wait_ms);
        out.push(',');
        push_json_number(&mut out, "defaultAgainMs", self.loaded.default_again_ms);
        out.push(',');
        push_export_animation(&mut out, &self.loaded);
        out.push(',');
        push_sound_events(&mut out, &sound_events);
        out.push(',');
        push_message_events(&mut out, &message_events);
        out.push(',');
        push_wait_events(&mut out, &wait_events);
        out.push(',');
        push_animation_events(&mut out, &animation_events);
        out.push(',');
        push_level_context(
            &mut out,
            &self.loaded,
            self.session.cleared_levels(),
            self.session.active_level_index(),
        );
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
        push_visible_scenes(&mut out, self.session.visible_scenes());
        out.push(',');
        push_session_state(&mut out, &self.loaded, &self.session);
        out.push(',');
        push_scene_state(&mut out, &self.loaded, &self.session);
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

    fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), AppError> {
        if level_index >= self.loaded.levels.len() {
            return Err(AppError::Config(format!(
                "level index out of range: {level_index}"
            )));
        }
        let state = state_from_json(&self.loaded, state_json)?;
        self.session
            .start_level_from_state(&self.loaded, level_index, state, materialize_level_start)
            .map_err(AppError::CoreTransition)
    }

    fn progress_save_json(&self) -> String {
        let save = self.session.progress_save_data(&self.loaded);
        let mut out = String::new();
        push_progress_save_data(&mut out, &save);
        out
    }

    fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), AppError> {
        let save = progress_save_data_from_json(save_json).map_err(AppError::Config)?;
        self.session
            .restore_progress_save_data(&self.loaded, &save)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
        self.has_progress_save = true;
        Ok(())
    }

    #[cfg(feature = "solver")]
    fn solve_json(&self) -> Result<String, AppError> {
        let response =
            solve_current_state(&self.loaded, self.session.state().clone(), self.solver)?;
        let mut out = String::new();
        push_solution_response(&mut out, &self.loaded, &response);
        Ok(out)
    }
}

pub struct StandaloneSessionBridge {
    state: ServerState,
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
            state: ServerState::new(
                loaded,
                source.to_string(),
                puzzle_path.to_string(),
                String::new(),
                String::new(),
                SolverConfig::default(),
            ),
        })
    }

    pub fn snapshot_json(&mut self) -> String {
        self.state.snapshot_json()
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, String> {
        match (method, url) {
            ("GET", "/api/state") => Ok(self.snapshot_json()),
            ("POST", "/api/command/undo") => {
                self.state.session.undo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/redo") => {
                self.state.session.redo(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/restart") => {
                self.state.session.restart_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", "/api/command/next") => {
                self.state.session.advance_level(&self.state.loaded);
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/input/") => {
                let input_name = percent_decode(&path["/api/input/".len()..]);
                self.state
                    .apply_input_name(&input_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
            ("POST", path) if path.starts_with("/api/command/") => {
                let command_name = percent_decode(&path["/api/command/".len()..]);
                self.state
                    .apply_command_name(&command_name)
                    .map_err(|error| error.to_string())?;
                Ok(self.snapshot_json())
            }
            _ => Err(format!("Unsupported exported HTML request: {method} {url}")),
        }
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), String> {
        self.state
            .apply_input_name(input_name)
            .map_err(|error| error.to_string())
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), String> {
        self.state
            .apply_command_name(command_name)
            .map_err(|error| error.to_string())
    }

    pub fn set_current_state_json(
        &mut self,
        state_json: &str,
        level_index: usize,
        materialize_level_start: bool,
    ) -> Result<(), String> {
        self.state
            .set_current_state_json(state_json, level_index, materialize_level_start)
            .map_err(|error| error.to_string())
    }

    pub fn progress_save_json(&self) -> String {
        self.state.progress_save_json()
    }

    pub fn restore_progress_save_json(&mut self, save_json: &str) -> Result<(), String> {
        self.state
            .restore_progress_save_json(save_json)
            .map_err(|error| error.to_string())
    }

    pub fn mark_progress_save_written(&mut self) {
        self.state.has_progress_save = true;
    }

    pub fn clear_progress_save(&mut self) {
        self.state.has_progress_save = false;
    }
}

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SolutionStep {
    index: usize,
    input: Option<InputId>,
    state: State,
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
#[derive(Clone, Debug)]
struct SolutionStep3 {
    index: usize,
    input: Option<InputId3>,
    state: State3,
    completed: bool,
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn solve_current_state(
    loaded: &LoadedGame,
    initial: State,
    solver: SolverConfig,
) -> Result<SolutionResponse, AppError> {
    solve_current_state_with_budget(loaded, initial, solver.budget())
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
) -> Result<SolutionResponse, AppError> {
    solve_current_state_with_budget_inner(
        loaded,
        initial,
        budget,
        None::<fn(&State, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
fn solve_current_state_with_budget_inner<O>(
    loaded: &LoadedGame,
    initial: State,
    budget: SearchBudget,
    mut on_progress: Option<O>,
) -> Result<SolutionResponse, AppError>
where
    O: FnMut(&State, SearchProgress),
{
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
    let outcome = best_first_with_dead_states_and_progress(
        &mut domain,
        solver_initial,
        budget,
        move |state| goal_score(&score_game, state),
        move |state| lose_game.is_lose_complete(state),
        |state, progress| {
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(state, progress);
            }
        },
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn solve_current_state3_with_budget(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
) -> Result<SolutionResponse3, AppError> {
    solve_current_state3_with_budget_inner(
        parsed,
        initial,
        budget,
        None::<fn(&State3, SearchProgress)>,
    )
}

#[cfg(feature = "solver")]
fn solve_current_state3_with_budget_inner<O>(
    parsed: &ParsedPuzzle3,
    initial: State3,
    budget: SearchBudget,
    mut on_progress: Option<O>,
) -> Result<SolutionResponse3, AppError>
where
    O: FnMut(&State3, SearchProgress),
{
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
    let outcome = best_first_with_dead_states_and_progress(
        &mut domain,
        initial.clone(),
        budget,
        |_| 0,
        |_| false,
        |state, progress| {
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(state, progress);
            }
        },
    );

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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn goal_score(loaded: &LoadedGame, state: &State) -> i64 {
    loaded
        .goal
        .as_ref()
        .map(|goal| goal_expr_score(&loaded.game, state, &goal.expr))
        .unwrap_or(0)
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn goal_clause_score(
    game: &CompiledGame,
    state: &State,
    value: &GoalValue,
    current: i64,
    expected: i64,
) -> i64 {
    match value {
        GoalValue::Global(_) => current.abs_diff(expected) as i64,
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| {
                condition_value_kind_score(game, state, &condition.kind, current, expected)
            })
            .unwrap_or_else(|| current.abs_diff(expected) as i64),
        GoalValue::InlineConditionValue(kind) => {
            condition_value_kind_score(game, state, kind, current, expected)
        }
    }
}

#[cfg(feature = "solver")]
fn condition_value_kind_score(
    game: &CompiledGame,
    state: &State,
    kind: &ConditionValueKind,
    current: i64,
    expected: i64,
) -> i64 {
    match kind {
        ConditionValueKind::CountMatches(patterns) if expected == 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        ConditionValueKind::NoneMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) if expected != 0 => patterns
            .iter()
            .map(|pattern| pattern_distance_score(game, state, pattern))
            .min()
            .unwrap_or(1),
        _ => current.abs_diff(expected) as i64,
    }
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> i64 {
    i64::from(ax.abs_diff(bx)) + i64::from(ay.abs_diff(by))
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn goal_value(game: &CompiledGame, state: &State, value: &GoalValue) -> i64 {
    match value {
        GoalValue::Global(global) => state.global_value(*global).unwrap_or(0),
        GoalValue::Condition(condition) => game
            .condition_def(*condition)
            .map(|condition| goal_condition_value_kind(game, state, &condition.kind))
            .unwrap_or(0),
        GoalValue::InlineConditionValue(kind) => goal_condition_value_kind(game, state, kind),
    }
}

#[cfg(feature = "solver")]
fn goal_condition_value_kind(game: &CompiledGame, state: &State, kind: &ConditionValueKind) -> i64 {
    match kind {
        ConditionValueKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        ConditionValueKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| i64::from(puzzle_core::count_pattern_matches(game, state, pattern)))
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| puzzle_core::has_pattern_match(game, state, pattern))
            {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountInputMatches(_)
        | ConditionValueKind::ExistsInputMatches(_)
        | ConditionValueKind::NoneInputMatches(_) => 0,
    }
}

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
    let embedded_wasm_js = embedded_standalone_wasm_script();
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
                "<script>\nwindow.PuzzleExport = JSON.parse(\"{data}\");\n{embedded_wasm_js}\n</script>\n<script>\n{renderer_js}\n</script>\n<script>\n{standalone_js}\n</script>"
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

fn embedded_standalone_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_wasm_loader_script(PUZZLE_GAME_WASM_JS, PUZZLE_GAME_WASM_BG)
    }
    #[cfg(target_arch = "wasm32")]
    {
        String::new()
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

pub struct CoreRuntimeBridge {
    loaded: LoadedGame,
    current_state: Option<State>,
    saved_states: SavedStateStore<State>,
}

impl CoreRuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        Ok(Self {
            loaded: parse_game(source).map_err(|error| error.to_string())?,
            current_state: None,
            saved_states: SavedStateStore::new(),
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

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        let state = state_from_json(&self.loaded, state_json).map_err(|error| error.to_string())?;
        self.current_state = Some(state);
        Ok(())
    }

    pub fn current_state_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state_data(&mut out, state);
        Ok(out)
    }

    pub fn current_state_hash_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        Ok(state.hash().to_string())
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state2_cells(&mut out, state, None);
        Ok(out)
    }

    pub fn save_current_state(&mut self) -> Result<u32, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        Ok(self.saved_states.save(state.clone()))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), String> {
        self.current_state = Some(self.saved_states.restore(handle)?.clone());
        Ok(())
    }

    pub fn transition_current_outcome_json(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, String> {
        self.transition_current_outcome_json_inner(program_key, level_index, input, false)
    }

    pub fn transition_current_state_outcome_json(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, String> {
        self.transition_current_outcome_json_inner(program_key, level_index, input, true)
    }

    fn transition_current_outcome_json_inner(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
        include_state: bool,
    ) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let program = selected_rule_program(&self.loaded, program_key, level_index)
            .map_err(|error| error.to_string())?;
        let outcome = transition_program_trace(&self.loaded.game, program, state, InputId(input))
            .map_err(|error| format!("{error:?}"))?;
        let before = state.clone();
        let previous_state_handle = if program_key == "main" && before != outcome.next_state {
            Some(self.saved_states.save(before.clone()))
        } else {
            None
        };
        self.current_state = Some(outcome.next_state.clone());
        let mut out = String::new();
        push_transition_current_outcome_json(
            &mut out,
            &self.loaded,
            &outcome.next_state,
            Some(&before),
            previous_state_handle,
            outcome.cancelled,
            &outcome.commands,
            &outcome.fired_rules,
            &outcome.patches,
            include_state,
        );
        Ok(out)
    }
}

pub struct Puzzle3RuntimeBridge {
    parsed: ParsedPuzzle3,
    animation: AnimationDef,
    current_state: Option<State3>,
    saved_states: SavedStateStore<State3>,
}

impl Puzzle3RuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        if let Ok(parsed) = puzzle_3d::parse_puzzle3d(source) {
            return Ok(Self {
                parsed,
                animation: AnimationDef::default(),
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

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        let state =
            state3_from_json(&self.parsed.game, state_json).map_err(|error| error.to_string())?;
        self.current_state = Some(state);
        Ok(())
    }

    pub fn current_state_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state3_data(&mut out, state);
        Ok(out)
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state3_cells(&mut out, state, None);
        Ok(out)
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
            transition_selected_program3(&self.parsed, program_key, state, InputId3(input))
                .map_err(|error| error.to_string())?;
        let completed = self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, &next_state));
        self.current_state = Some(next_state.clone());
        let mut out = String::new();
        out.push('{');
        push_json_bool(&mut out, "changed", before != next_state);
        out.push(',');
        push_json_bool(&mut out, "completed", completed);
        out.push_str(",\"stateHash\":");
        out.push_str(&next_state.hash().to_string());
        out.push_str(",\"changedCells\":");
        push_state3_cells(&mut out, &next_state, Some(&before));
        out.push_str(",\"animationEvents\":");
        push_animation_events3(&mut out, &self.animation, &before, &next_state);
        out.push_str(",\"commands\":[]}");
        Ok(out)
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
    let next_state = transition_selected_program3(parsed, program_key, &state, input)?;
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

fn transition_selected_program3(
    parsed: &ParsedPuzzle3,
    program_key: &str,
    state: &State3,
    input: InputId3,
) -> Result<State3, AppError> {
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
        other => {
            return Err(AppError::Config(format!(
                "unknown 3D transition program selector: {other}"
            )));
        }
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))
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
    let outcome = transition_program_trace(&loaded.game, program, &state, input)?;
    let mut out = String::new();
    push_transition_outcome_json(
        &mut out,
        loaded,
        &outcome.next_state,
        outcome.cancelled,
        &outcome.commands,
        &outcome.fired_rules,
        &outcome.patches,
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
    loaded: &LoadedGame,
    state: &State,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
) {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
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
                TransitionCommand::Checkpoint => "checkpoint",
                TransitionCommand::ClearCheckpoint => "clear_checkpoint",
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
    out.push_str(",\"patches\":");
    push_transition_patches(out, patches);
    out.push(',');
    push_animation_events(out, &animation_events);
    out.push('}');
}

fn push_transition_current_outcome_json(
    out: &mut String,
    loaded: &LoadedGame,
    state: &State,
    before: Option<&State>,
    previous_state_handle: Option<u32>,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
    include_state: bool,
) {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
    out.push('{');
    push_json_bool(out, "cancelled", cancelled);
    out.push(',');
    push_json_bool(out, "changed", before.is_some_and(|before| before != state));
    if include_state {
        out.push_str(",\"state\":");
        push_state_data(out, state);
    }
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
                TransitionCommand::Checkpoint => "checkpoint",
                TransitionCommand::ClearCheckpoint => "clear_checkpoint",
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
    out.push_str(",\"patches\":");
    push_transition_patches(out, patches);
    out.push(',');
    push_animation_events(out, &animation_events);
    out.push_str(",\"stateHash\":");
    out.push_str(&state.hash().to_string());
    out.push_str(",\"stateHashKey\":\"");
    out.push_str(&state.hash().to_string());
    out.push('"');
    if let Some(handle) = previous_state_handle {
        out.push_str(",\"previousStateHandle\":");
        out.push_str(&handle.to_string());
    }
    out.push_str(",\"changedCells\":");
    push_state2_cells(out, state, before);
    out.push_str(",\"globals\":[");
    for (index, value) in state.visible_globals().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push_str("],\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push_str("]}");
}

fn push_transition_patches(out: &mut String, patches: &[Patch]) {
    out.push('[');
    for (patch_index, patch) in patches.iter().enumerate() {
        if patch_index > 0 {
            out.push(',');
        }
        out.push('[');
        for (op_index, op) in patch.ops().iter().enumerate() {
            if op_index > 0 {
                out.push(',');
            }
            out.push('{');
            match op {
                PatchOp::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    object,
                } => {
                    push_json_pair(out, "kind", "move");
                    out.push(',');
                    push_json_number(out, "fromX", *from_x as u64);
                    out.push(',');
                    push_json_number(out, "fromY", *from_y as u64);
                    out.push(',');
                    push_json_number(out, "toX", *to_x as u64);
                    out.push(',');
                    push_json_number(out, "toY", *to_y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::RemoveScratch {
                    x,
                    y,
                    object,
                    scratch,
                    ..
                } => {
                    push_json_pair(out, "kind", "remove_scratch");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                    out.push(',');
                    push_json_number(out, "scratch", scratch.0 as u64);
                }
                PatchOp::Add { x, y, object } => {
                    push_json_pair(out, "kind", "add");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::Remove { x, y, object } => {
                    push_json_pair(out, "kind", "remove");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::Replace { x, y, remove, add } => {
                    push_json_pair(out, "kind", "replace");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "remove", remove.0 as u64);
                    out.push(',');
                    push_json_number(out, "add", add.0 as u64);
                }
                PatchOp::SetScratch {
                    x,
                    y,
                    object,
                    scratch,
                    ..
                } => {
                    push_json_pair(out, "kind", "set_scratch");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                    out.push(',');
                    push_json_number(out, "scratch", scratch.0 as u64);
                }
                PatchOp::UpdateGlobal { global, .. } => {
                    push_json_pair(out, "kind", "update_global");
                    out.push(',');
                    push_json_number(out, "global", global.0 as u64);
                }
            }
            out.push('}');
        }
        out.push(']');
    }
    out.push(']');
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn solve_state_json_from_source_inner(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    puzzle_lang::validate_source_profile_for_path(source, puzzle_path)?;
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

#[cfg(feature = "solver")]
fn solve_state3_json_from_source_inner(
    source: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    let parsed = parse_puzzle3d_for_solver(source)?;
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

#[cfg(feature = "solver")]
fn parse_puzzle3d_for_solver(source: &str) -> Result<ParsedPuzzle3, AppError> {
    match puzzle_3d::parse_puzzle3d(source) {
        Ok(parsed) => Ok(parsed),
        Err(raw_error) => {
            let document = puzzle_lang::parse_game(source)
                .map_err(|_| AppError::Config(format!("{raw_error:?}")))?;
            document
                .models
                .into_iter()
                .find_map(|model| match model {
                    LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle),
                    LoadedDocumentModel::Puzzle2d { .. } => None,
                })
                .ok_or_else(|| AppError::Config(format!("{raw_error:?}")))
        }
    }
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
    transition_program_without_input_with_local_frame(
        &parsed.game,
        &state,
        &parsed.lifecycle.on_level_start,
        parsed.lifecycle.on_level_start_local_frame.as_ref(),
    )
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
    push_compiled_play_bundle(out, &state.loaded);
    out.push(',');
    push_runtime_loaded_game_bundle(out, &state.loaded);
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
    push_json_number(out, "defaultAgainMs", state.loaded.default_again_ms);
    out.push(',');
    push_export_animation(out, &state.loaded);
    out.push(',');
    push_export_goal(out, "goal", state.loaded.goal.as_ref());
    out.push(',');
    push_export_goal(out, "lose", state.loaded.lose.as_ref());
    out.push(',');
    push_export_conditions(out, &state.loaded);
    out.push('}');
}

fn push_progress_save_data(out: &mut String, save: &ProgressSaveData) {
    out.push('{');
    push_json_number(out, "version", u64::from(save.version));
    out.push_str(",\"levels\":[");
    for (index, level) in save.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_bool(out, "cleared", level.cleared);
        out.push('}');
    }
    out.push_str("],\"currentLevel\":");
    if let Some(current_level) = &save.current_level {
        push_json_string(out, current_level);
    } else {
        out.push_str("null");
    }
    out.push_str(",\"persistentVars\":[");
    for (index, var) in save.persistent_vars.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &var.name);
        out.push(',');
        push_json_i64(out, "value", var.value);
        out.push('}');
    }
    out.push_str("]}");
}

fn progress_save_data_from_json(raw: &str) -> Result<ProgressSaveData, String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "progress save is missing version".to_string())?;
    let levels = value
        .get("levels")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "progress save is missing levels".to_string())?
        .iter()
        .filter_map(|entry| {
            Some(LevelProgressSaveData {
                name: entry.get("name")?.as_str()?.to_string(),
                cleared: entry
                    .get("cleared")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect();
    let current_level = value
        .get("currentLevel")
        .or_else(|| value.get("current_level"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    let persistent_vars = value
        .get("persistentVars")
        .or_else(|| value.get("persistent_vars"))
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    Some(PersistentVarSaveData {
                        name: entry.get("name")?.as_str()?.to_string(),
                        value: entry
                            .get("value")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ProgressSaveData {
        version: u32::try_from(version).map_err(|_| "progress save version is too large")?,
        levels,
        current_level,
        persistent_vars,
    })
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
    out.push_str("\"sounds\":");
    let sounds_json = serde_json::to_string(&runtime_sounds_def(sounds))
        .expect("runtime sounds contract should serialize");
    out.push_str(&sounds_json);
}

fn push_export_animation(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"animation\":{");
    out.push_str("\"tween\":{");
    push_json_bool(out, "enabled", loaded.animation.tween.enabled);
    out.push(',');
    push_json_number(out, "intervalMs", loaded.animation.tween.interval_ms);
    out.push('}');
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
            WaitEvent::ContinueEffects { milliseconds } => {
                push_json_pair(out, "kind", "continue_effects");
                out.push(',');
                push_json_number(out, "milliseconds", *milliseconds);
            }
        }
        out.push('}');
    }
    out.push(']');
}

fn push_animation_events(out: &mut String, events: &[AnimationEvent]) {
    out.push_str("\"animationEvents\":[");
    for (index, event) in events.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        match event {
            AnimationEvent::Move {
                name,
                object,
                from_x,
                from_y,
                from_z,
                to_x,
                to_y,
                to_z,
            } => {
                push_json_pair(out, "kind", "move");
                out.push(',');
                push_json_pair(out, "name", name);
                out.push(',');
                push_json_number(out, "objectId", object.0 as u64);
                out.push(',');
                push_json_number(out, "fromX", *from_x as u64);
                out.push(',');
                push_json_number(out, "fromY", *from_y as u64);
                out.push(',');
                push_json_number(out, "fromZ", *from_z as u64);
                out.push(',');
                push_json_number(out, "toX", *to_x as u64);
                out.push(',');
                push_json_number(out, "toY", *to_y as u64);
                out.push(',');
                push_json_number(out, "toZ", *to_z as u64);
            }
            AnimationEvent::CantMove { name, object, x, y } => {
                push_json_pair(out, "kind", "cantmove");
                out.push(',');
                push_json_pair(out, "name", name);
                out.push(',');
                push_json_number(out, "objectId", object.0 as u64);
                out.push(',');
                push_json_number(out, "x", *x as u64);
                out.push(',');
                push_json_number(out, "y", *y as u64);
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
    push_rule_animations(out, loaded);
    out.push(',');
    push_rule_effects(out, loaded);
    out.push(',');
    out.push_str("\"program\":[]");
    out.push(',');
    out.push_str("\"levelStartProgram\":");
    push_empty_rule_program(out);
    out.push(',');
    out.push_str("\"runRulesOnLevelStart\":");
    out.push_str(if loaded.run_rules_on_level_start {
        "true"
    } else {
        "false"
    });
    out.push(',');
    out.push_str("\"displayLevelStartProgram\":");
    push_empty_rule_program(out);
    out.push(',');
    out.push_str("\"levelClearProgram\":");
    push_empty_rule_program(out);
    out.push(',');
    out.push_str("\"displayLevelClearProgram\":");
    push_empty_rule_program(out);
    out.push(',');
    out.push_str("\"displayProgram\":");
    push_empty_rule_program(out);
    out.push('}');
}

fn push_compiled_play_bundle(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"compiledPlay\":{");
    push_json_number(out, "version", 1);
    out.push(',');
    push_json_pair(out, "model", "grid2");
    out.push_str(",\"transition\":[");
    out.push_str(&loaded.game.layer_count.to_string());
    out.push(',');
    push_compact_objects(out, loaded);
    out.push(',');
    push_compact_queries(out, &loaded.game);
    out.push(',');
    out.push('[');
    for (index, object) in loaded.game.visual_objects().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push(',');
    push_compact_transition_programs(out, loaded);
    out.push(',');
    push_compact_level_programs(out, loaded);
    out.push_str("]}");
}

fn push_rule_effects(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"ruleEffects\":{");
    let mut entries = loaded.rule_effects.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(rule, _)| rule.0);
    for (index, (rule, effects)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &rule.0.to_string());
        out.push(':');
        out.push('[');
        for (effect_index, effect) in effects.iter().enumerate() {
            if effect_index > 0 {
                out.push(',');
            }
            push_ordered_rule_effect(out, effect);
        }
        out.push(']');
    }
    out.push('}');
}

fn push_ordered_rule_effect(out: &mut String, effect: &RuleEffect) {
    out.push('{');
    match effect {
        RuleEffect::Win => push_json_pair(out, "kind", "win"),
        RuleEffect::Restart => push_json_pair(out, "kind", "restart"),
        RuleEffect::NextLevel => push_json_pair(out, "kind", "next_level"),
        RuleEffect::Again => push_json_pair(out, "kind", "again"),
        RuleEffect::Checkpoint => push_json_pair(out, "kind", "checkpoint"),
        RuleEffect::ClearCheckpoint => push_json_pair(out, "kind", "clear_checkpoint"),
        RuleEffect::PlaySfx { name } => {
            push_json_pair(out, "kind", "play_sfx");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        RuleEffect::PlayMusic { name } => {
            push_json_pair(out, "kind", "play_music");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        RuleEffect::PauseMusic { name } => {
            push_json_pair(out, "kind", "pause_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::ResumeMusic { name } => {
            push_json_pair(out, "kind", "resume_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::StopMusic { name } => {
            push_json_pair(out, "kind", "stop_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::Wait { milliseconds } => {
            push_json_pair(out, "kind", "wait");
            out.push(',');
            push_json_number(out, "milliseconds", *milliseconds);
        }
        RuleEffect::WaitAnimation => push_json_pair(out, "kind", "wait_animation"),
        RuleEffect::Message { text, literal } => {
            push_json_pair(out, "kind", "message");
            out.push(',');
            push_json_pair(out, "text", text);
            out.push(',');
            push_json_bool(out, "literal", *literal);
        }
        RuleEffect::Scene(effect) => {
            push_json_effect_fields(out, effect);
        }
    }
    out.push('}');
}

fn push_rule_animations(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"ruleAnimations\":{");
    let mut entries = loaded.rule_animations.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(rule, _)| rule.0);
    for (index, (rule, animations)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &rule.0.to_string());
        out.push(':');
        out.push('[');
        for (animation_index, animation) in animations.iter().enumerate() {
            if animation_index > 0 {
                out.push(',');
            }
            push_rule_animation(out, animation);
        }
        out.push(']');
    }
    out.push('}');
}

fn push_rule_animation(out: &mut String, animation: &RuleAnimation) {
    out.push('{');
    push_json_pair(out, "kind", "animate");
    out.push(',');
    push_json_pair(
        out,
        "trigger",
        match animation.trigger {
            RuleAnimationTrigger::Move => "move",
            RuleAnimationTrigger::CantMove => "cantmove",
        },
    );
    out.push(',');
    push_json_pair(out, "name", &animation.name);
    out.push(',');
    out.push_str("\"objects\":[");
    for (index, object) in animation.objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn push_runtime_loaded_game_bundle(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"runtimeLoadedGame\":{");
    push_json_number(out, "version", 1);
    out.push_str(",\"loaded\":");
    let loaded_json =
        serde_json::to_string(loaded).expect("runtime loaded game bundle should serialize");
    out.push_str(&loaded_json);
    out.push('}');
}

fn push_compact_objects(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    for id in 1..=loaded.game.object_count() {
        if id > 1 {
            out.push(',');
        }
        let object_id = ObjectId(id as u16);
        let def = loaded
            .game
            .object(object_id)
            .expect("compiled object id should exist");
        out.push('[');
        out.push_str(&def.id.0.to_string());
        out.push(',');
        out.push_str(&def.layer_id.0.to_string());
        out.push(']');
    }
    out.push(']');
}

fn push_compact_queries(out: &mut String, game: &CompiledGame) {
    out.push('[');
    for (index, condition) in game.condition_defs().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&condition.id.0.to_string());
        out.push(',');
        push_compact_condition_value_kind(out, &condition.kind);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_transition_programs(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    push_compact_rule_program(out, loaded.game.program());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.level_start_program.as_deref());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.level_clear_program.as_deref());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.display_level_start_program.as_deref());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.display_level_clear_program.as_deref());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.display_program.as_deref());
    out.push(']');
}

fn push_compact_level_programs(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        push_compact_optional_rule_program(out, level.level_start_program.as_deref());
        out.push(',');
        push_compact_optional_rule_program(out, level.level_clear_program.as_deref());
        out.push(']');
    }
    out.push(']');
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

fn push_empty_rule_program(out: &mut String) {
    out.push_str("[]");
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
    for (index, condition) in game.condition_defs().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", condition.id.0 as u64);
        out.push(',');
        push_condition_value_kind(out, &condition.kind);
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
        push_empty_rule_program(out);
        out.push(',');
        out.push_str("\"levelClearProgram\":");
        push_empty_rule_program(out);
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

fn push_state2_cells(out: &mut String, state: &State, before: Option<&State>) {
    out.push('[');
    let mut first = true;
    for y in 0..state.height {
        for x in 0..state.width {
            let cell = usize::from(y) * usize::from(state.width) + usize::from(x);
            if before.is_some_and(|before| state2_cell_slots_equal(before, state, cell)) {
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
            if !first {
                out.push(',');
            }
            first = false;
            out.push('{');
            push_json_number(out, "x", x as u64);
            out.push(',');
            push_json_number(out, "y", y as u64);
            out.push_str(",\"objects\":[");
            for (index, object) in objects.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&object.to_string());
            }
            out.push_str("]}");
        }
    }
    out.push(']');
}

fn state2_cell_slots_equal(before: &State, after: &State, cell: usize) -> bool {
    if before.width != after.width
        || before.height != after.height
        || before.layer_count != after.layer_count
    {
        return false;
    }
    let layer_count = usize::from(after.layer_count);
    let start = cell * layer_count;
    before.slots()[start..start + layer_count] == after.slots()[start..start + layer_count]
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

fn push_state3_cells(out: &mut String, state: &State3, before: Option<&State3>) {
    out.push('[');
    let mut first = true;
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
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('{');
                out.push_str("\"position\":{");
                push_json_number(out, "x", x as u64);
                out.push(',');
                push_json_number(out, "y", y as u64);
                out.push(',');
                push_json_number(out, "z", z as u64);
                out.push_str("},\"objects\":[");
                for (index, object) in objects.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&object.to_string());
                }
                out.push_str("]}");
            }
        }
    }
    out.push(']');
}

fn state3_cell_slots_equal(before: &State3, after: &State3, cell: usize) -> bool {
    if before.size != after.size || before.layer_count != after.layer_count {
        return false;
    }
    let layer_count = usize::from(after.layer_count);
    let start = cell * layer_count;
    before.slots()[start..start + layer_count] == after.slots()[start..start + layer_count]
}

fn push_animation_events3(
    out: &mut String,
    animation: &AnimationDef,
    before: &State3,
    after: &State3,
) {
    out.push('[');
    if !animation.tween.enabled
        || before.size != after.size
        || before.layer_count != after.layer_count
    {
        out.push(']');
        return;
    }
    let mut first = true;
    for object in changed_object_ids3(before, after) {
        let mut sources = changed_positions_for_object3(before, after, object, false);
        let targets = changed_positions_for_object3(before, after, object, true);
        for target in targets {
            let Some(source_index) = sources
                .iter()
                .position(|source| adjacent_coord3(*source, target))
            else {
                continue;
            };
            let source = sources.remove(source_index);
            if !first {
                out.push(',');
            }
            first = false;
            out.push('{');
            push_json_pair(out, "kind", "move");
            out.push(',');
            push_json_pair(out, "name", "tween");
            out.push(',');
            push_json_number(out, "objectId", object.0 as u64);
            out.push(',');
            push_json_number(out, "fromX", source.x as u64);
            out.push(',');
            push_json_number(out, "fromY", source.y as u64);
            out.push(',');
            push_json_number(out, "fromZ", source.z as u64);
            out.push(',');
            push_json_number(out, "toX", target.x as u64);
            out.push(',');
            push_json_number(out, "toY", target.y as u64);
            out.push(',');
            push_json_number(out, "toZ", target.z as u64);
            out.push('}');
        }
    }
    out.push(']');
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

fn push_compact_optional_rule_program(out: &mut String, program: Option<&[RuleStep]>) {
    match program {
        Some(program) => push_compact_rule_program(out, program),
        None => out.push_str("[]"),
    }
}

fn push_compact_rule_program(out: &mut String, program: &[RuleStep]) {
    out.push('[');
    for (index, step) in program.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_rule_step(out, step);
    }
    out.push(']');
}

fn push_compact_rule_step(out: &mut String, step: &RuleStep) {
    out.push('[');
    match step {
        RuleStep::Rule(rule) => {
            out.push('0');
            out.push(',');
            push_compact_rule(out, rule);
        }
        RuleStep::ConditionalBlock { condition, steps } => {
            out.push('1');
            out.push(',');
            push_compact_rule_condition(out, condition);
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            out.push('5');
            out.push(',');
            push_compact_rule_condition(out, condition);
            out.push(',');
            push_compact_rule_program(out, then_steps);
            out.push(',');
            push_compact_rule_program(out, else_steps);
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => {
            out.push('2');
            out.push(',');
            push_compact_rule_application(out, *application);
            out.push(',');
            if let Some(condition) = stop_condition {
                push_compact_rule_condition(out, condition);
            } else {
                out.push_str("null");
            }
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::LocalFrame { frame, steps } => {
            out.push('3');
            out.push(',');
            push_compact_local_frame(out, frame);
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::AfterTriggered { steps, then_steps } => {
            out.push('4');
            out.push(',');
            push_compact_rule_program(out, steps);
            out.push(',');
            push_compact_rule_program(out, then_steps);
        }
    }
    out.push(']');
}

fn push_compact_rule(out: &mut String, rule: &Rule) {
    out.push('[');
    out.push_str(&rule.id.0.to_string());
    out.push(',');
    push_compact_rule_application(out, rule.application);
    out.push(',');
    out.push('[');
    for (index, guard) in rule.guards.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_guard(out, guard);
    }
    out.push(']');
    out.push(',');
    push_compact_pattern(out, &rule.pattern);
    out.push(',');
    out.push('[');
    for (index, write) in rule.writes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_write(out, write);
    }
    out.push(']');
    out.push(',');
    out.push('[');
    for (index, effect) in rule.effects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_effect(out, effect);
    }
    out.push_str("]]");
}

fn push_compact_rule_application(out: &mut String, application: RuleApplication) {
    out.push_str(match application {
        RuleApplication::Once => "0",
        RuleApplication::OnceAll => "1",
        RuleApplication::OncePerLevel => "2",
        RuleApplication::UntilStable => "3",
    });
}

fn push_compact_rule_condition(out: &mut String, condition: &RuleCondition) {
    out.push('[');
    match condition {
        RuleCondition::AnyMatches(patterns) => {
            out.push('0');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        RuleCondition::NoMatches(patterns) => {
            out.push('1');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        RuleCondition::AnyInputMatches(patterns) => {
            out.push('2');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        RuleCondition::NoInputMatches(patterns) => {
            out.push('3');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        RuleCondition::GuardBranches(branches) => {
            out.push('4');
            out.push(',');
            out.push('[');
            for (branch_index, branch) in branches.iter().enumerate() {
                if branch_index > 0 {
                    out.push(',');
                }
                out.push('[');
                for (guard_index, guard) in branch.iter().enumerate() {
                    if guard_index > 0 {
                        out.push(',');
                    }
                    push_compact_guard(out, guard);
                }
                out.push(']');
            }
            out.push(']');
        }
    }
    out.push(']');
}

fn push_compact_guard(out: &mut String, guard: &Guard) {
    out.push('[');
    match guard {
        Guard::InputIs(input) => {
            out.push('0');
            out.push(',');
            out.push_str(&input.0.to_string());
        }
        Guard::GlobalEquals { global, value } => {
            out.push('1');
            out.push(',');
            out.push_str(&global.0.to_string());
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::GlobalCompare { global, op, value } => {
            out.push('1');
            out.push(',');
            out.push_str(&global.0.to_string());
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::ConditionEquals { condition, value } => {
            out.push('2');
            out.push(',');
            out.push_str(&condition.0.to_string());
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::ConditionNonZero(condition) => {
            out.push('3');
            out.push(',');
            out.push_str(&condition.0.to_string());
        }
        Guard::ConditionCompare {
            condition,
            op,
            value,
        } => {
            out.push('2');
            out.push(',');
            out.push_str(&condition.0.to_string());
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::InlineConditionValue { kind, value } => {
            out.push('4');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::InlineConditionNonZero(kind) => {
            out.push('5');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
        }
        Guard::InlineConditionCompare { kind, op, value } => {
            out.push('4');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
    }
    out.push(']');
}

fn push_compact_condition_value_kind(out: &mut String, kind: &ConditionValueKind) {
    out.push('[');
    match kind {
        ConditionValueKind::CountObjects(objects) => {
            out.push('0');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::ExistsObjects(objects) => {
            out.push('1');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::NoneObjects(objects) => {
            out.push('2');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::CountMatches(patterns) => {
            out.push('3');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::ExistsMatches(patterns) => {
            out.push('4');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::NoneMatches(patterns) => {
            out.push('5');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::CountInputMatches(patterns) => {
            out.push('6');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        ConditionValueKind::ExistsInputMatches(patterns) => {
            out.push('7');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        ConditionValueKind::NoneInputMatches(patterns) => {
            out.push('8');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
    }
    out.push(']');
}

fn push_compact_patterns(out: &mut String, patterns: &[Pattern]) {
    out.push('[');
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_pattern(out, pattern);
    }
    out.push(']');
}

fn push_compact_input_patterns(out: &mut String, patterns: &[(InputId, Pattern)]) {
    out.push('[');
    for (index, (input, pattern)) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&input.0.to_string());
        out.push(',');
        push_compact_pattern(out, pattern);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_pattern(out: &mut String, pattern: &Pattern) {
    out.push('[');
    for (component_index, component) in pattern.components.iter().enumerate() {
        if component_index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&component.gap_count.to_string());
        out.push(',');
        out.push('[');
        for (cell_index, cell) in component.cells.iter().enumerate() {
            if cell_index > 0 {
                out.push(',');
            }
            push_compact_match_cell(out, cell);
        }
        out.push_str("]]");
    }
    out.push(']');
}

fn push_compact_match_cell(out: &mut String, cell: &puzzle_core::MatchCell) {
    out.push('[');
    push_compact_offset(out, &cell.offset);
    out.push(',');
    push_compact_object_ids(out, &cell.require_objects);
    out.push(',');
    push_compact_object_sets(out, &cell.require_object_sets);
    out.push(',');
    push_compact_object_ids(out, &cell.forbid_objects);
    out.push(',');
    push_compact_scratch_patterns(out, &cell.require_scratch);
    out.push(',');
    push_compact_object_set_scratch_patterns(out, &cell.require_object_set_scratch);
    out.push(',');
    push_compact_scratch_patterns(out, &cell.forbid_scratch);
    out.push(',');
    push_compact_object_set_scratch_patterns(out, &cell.forbid_object_set_scratch);
    out.push(']');
}

fn push_compact_object_sets(out: &mut String, object_sets: &[puzzle_core::ObjectSetMatcher]) {
    out.push('[');
    for (index, object_set) in object_sets.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&object_set.binding.to_string());
        out.push(',');
        out.push_str(&object_set.layer.0.to_string());
        out.push(',');
        push_compact_object_ids(out, &object_set.objects);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_object_set_scratch_patterns(
    out: &mut String,
    scratch: &[puzzle_core::ObjectSetScratchPattern],
) {
    out.push('[');
    for (index, pattern) in scratch.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&pattern.binding.to_string());
        out.push(',');
        out.push_str(&pattern.scratch.0.to_string());
        out.push(',');
        push_compact_optional_i64(out, pattern.value);
        out.push(',');
        push_compact_scratch_match(out, pattern.match_value);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_scratch_patterns(out: &mut String, scratch: &[ScratchPattern]) {
    out.push('[');
    for (index, pattern) in scratch.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&pattern.object.0.to_string());
        out.push(',');
        out.push_str(&pattern.scratch.0.to_string());
        out.push(',');
        push_compact_optional_i64(out, pattern.value);
        out.push(',');
        push_compact_scratch_match(out, pattern.match_value);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_offset(out: &mut String, offset: &Offset) {
    out.push('[');
    match offset {
        Offset::Fixed { dx, dy } => {
            out.push('0');
            out.push(',');
            out.push_str(&dx.to_string());
            out.push(',');
            out.push_str(&dy.to_string());
        }
        Offset::Variable {
            base_dx,
            base_dy,
            gap_terms,
        } => {
            out.push('1');
            out.push(',');
            out.push_str(&base_dx.to_string());
            out.push(',');
            out.push_str(&base_dy.to_string());
            out.push(',');
            out.push('[');
            for (index, term) in gap_terms.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('[');
                out.push_str(&term.gap_index.to_string());
                out.push(',');
                out.push_str(&term.dx.to_string());
                out.push(',');
                out.push_str(&term.dy.to_string());
                out.push(']');
            }
            out.push(']');
        }
    }
    out.push(']');
}

fn push_compact_write(out: &mut String, write: &WriteOp) {
    out.push('[');
    match write {
        WriteOp::Add {
            component,
            offset,
            object,
        } => {
            out.push('0');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::AddObjectSet {
            component,
            offset,
            binding,
        } => {
            out.push('6');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Remove {
            component,
            offset,
            object,
        } => {
            out.push('1');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::RemoveObjectSet {
            component,
            offset,
            binding,
        } => {
            out.push('7');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            out.push('2');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, from_offset);
            out.push(',');
            push_compact_offset(out, to_offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
        } => {
            out.push('8');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, from_offset);
            out.push(',');
            push_compact_offset(out, to_offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Replace {
            component,
            offset,
            remove,
            add,
        } => {
            out.push('3');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&remove.0.to_string());
            out.push(',');
            out.push_str(&add.0.to_string());
        }
        WriteOp::SetScratch {
            component,
            offset,
            object,
            scratch,
            value,
        } => {
            out.push('4');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
            out.push(',');
            out.push_str(&scratch.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
        }
        WriteOp::SetObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
        } => {
            out.push('9');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
            out.push(',');
            out.push_str(&scratch.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
        }
        WriteOp::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            value,
            match_value,
        } => {
            out.push('5');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
            out.push(',');
            out.push_str(&scratch.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
            out.push(',');
            push_compact_scratch_match(out, *match_value);
        }
        WriteOp::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
            match_value,
        } => {
            out.push_str("10");
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
            out.push(',');
            out.push_str(&scratch.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
            out.push(',');
            push_compact_scratch_match(out, *match_value);
        }
    }
    out.push(']');
}

fn push_compact_effect(out: &mut String, effect: &Effect) {
    out.push('[');
    match effect {
        Effect::Cancel => out.push('0'),
        Effect::Win => out.push('1'),
        Effect::Restart => out.push('2'),
        Effect::NextLevel => out.push('3'),
        Effect::Again => out.push('4'),
        Effect::Checkpoint => out.push('5'),
        Effect::ClearCheckpoint => out.push('6'),
        Effect::UpdateGlobal { global, op, value } => {
            out.push('7');
            out.push(',');
            out.push_str(&global.0.to_string());
            out.push(',');
            push_compact_global_update(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
    }
    out.push(']');
}

fn push_compact_local_frame(out: &mut String, frame: &puzzle_core::LocalFrame<ObjectId>) {
    out.push('[');
    push_compact_local_frame_extent(out, frame.x);
    out.push(',');
    push_compact_local_frame_extent(out, frame.y);
    out.push(',');
    push_compact_local_frame_extent(out, frame.z);
    out.push(',');
    push_compact_object_ids(out, &frame.focus_objects);
    out.push(']');
}

fn push_compact_local_frame_extent(out: &mut String, extent: puzzle_core::LocalFrameExtent) {
    match extent {
        puzzle_core::LocalFrameExtent::Radius(radius) => out.push_str(&radius.to_string()),
        puzzle_core::LocalFrameExtent::Full => out.push_str("null"),
    }
}

fn push_compact_object_ids(out: &mut String, objects: &[ObjectId]) {
    out.push('[');
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_compact_scratch_match(out: &mut String, value: ScratchValueMatch) {
    out.push_str(match value {
        ScratchValueMatch::Any => "0",
        ScratchValueMatch::Exact => "1",
    });
}

fn push_compact_comparison(out: &mut String, op: ComparisonOp) {
    out.push_str(match op {
        ComparisonOp::Eq => "0",
        ComparisonOp::NotEq => "1",
        ComparisonOp::Greater => "2",
        ComparisonOp::GreaterEq => "3",
        ComparisonOp::Less => "4",
        ComparisonOp::LessEq => "5",
    });
}

fn push_compact_global_update(out: &mut String, op: GlobalUpdateOp) {
    out.push_str(match op {
        GlobalUpdateOp::Set => "0",
        GlobalUpdateOp::Add => "1",
        GlobalUpdateOp::Subtract => "2",
        GlobalUpdateOp::Multiply => "3",
        GlobalUpdateOp::Divide => "4",
        GlobalUpdateOp::Remainder => "5",
    });
}

fn push_compact_optional_i64(out: &mut String, value: Option<i64>) {
    if let Some(value) = value {
        out.push_str(&value.to_string());
    } else {
        out.push_str("null");
    }
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

fn push_condition_value_kind(out: &mut String, kind: &ConditionValueKind) {
    out.push_str("\"conditionValueKind\":{");
    match kind {
        ConditionValueKind::CountObjects(objects) => {
            push_json_pair(out, "kind", "count_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::ExistsObjects(objects) => {
            push_json_pair(out, "kind", "exists_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::NoneObjects(objects) => {
            push_json_pair(out, "kind", "none_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::CountMatches(patterns) => {
            push_json_pair(out, "kind", "count_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::ExistsMatches(patterns) => {
            push_json_pair(out, "kind", "exists_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::NoneMatches(patterns) => {
            push_json_pair(out, "kind", "none_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::CountInputMatches(patterns) => {
            push_json_pair(out, "kind", "count_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        ConditionValueKind::ExistsInputMatches(patterns) => {
            push_json_pair(out, "kind", "exists_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        ConditionValueKind::NoneInputMatches(patterns) => {
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
        GoalValue::Condition(condition) => {
            push_json_pair(out, "kind", "condition");
            out.push(',');
            push_json_number(out, "condition", condition.0 as u64);
        }
        GoalValue::InlineConditionValue(kind) => {
            push_json_pair(out, "kind", "condition_value");
            out.push(',');
            push_condition_value_kind(out, kind);
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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
    print_wasm_freshness_status();

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
        #[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
fn solver_inputs(loaded: &LoadedGame) -> Vec<InputId> {
    let solver_game = loaded.game.solver_core();
    let mut inputs = BTreeSet::new();
    collect_solver_inputs(solver_game.program(), &mut inputs);

    let mut inputs = if inputs.is_empty() {
        loaded.input_labels.keys().copied().collect::<Vec<_>>()
    } else {
        inputs.into_iter().collect()
    };
    inputs.retain(|input| {
        loaded
            .input_labels
            .get(input)
            .is_none_or(|name| !is_solver_control_input(name))
    });
    inputs.sort();
    inputs
}

#[cfg(feature = "solver")]
fn collect_solver_inputs(program: &[RuleStep], inputs: &mut BTreeSet<InputId>) {
    for step in program {
        match step {
            RuleStep::Rule(rule) => {
                for guard in &rule.guards {
                    collect_solver_inputs_from_guard(guard, inputs);
                }
            }
            RuleStep::ConditionalBlock { condition, steps } => {
                collect_solver_inputs_from_condition(condition, inputs);
                collect_solver_inputs(steps, inputs);
            }
            RuleStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                collect_solver_inputs_from_condition(condition, inputs);
                collect_solver_inputs(then_steps, inputs);
                collect_solver_inputs(else_steps, inputs);
            }
            RuleStep::Block {
                stop_condition,
                steps,
                ..
            } => {
                if let Some(condition) = stop_condition {
                    collect_solver_inputs_from_condition(condition, inputs);
                }
                collect_solver_inputs(steps, inputs);
            }
            RuleStep::LocalFrame { steps, .. } => collect_solver_inputs(steps, inputs),
            RuleStep::AfterTriggered { steps, then_steps } => {
                collect_solver_inputs(steps, inputs);
                collect_solver_inputs(then_steps, inputs);
            }
        }
    }
}

#[cfg(feature = "solver")]
fn collect_solver_inputs_from_condition(condition: &RuleCondition, inputs: &mut BTreeSet<InputId>) {
    match condition {
        RuleCondition::AnyInputMatches(matches) | RuleCondition::NoInputMatches(matches) => {
            for (input, _) in matches {
                inputs.insert(*input);
            }
        }
        RuleCondition::GuardBranches(branches) => {
            for branch in branches {
                for guard in branch {
                    collect_solver_inputs_from_guard(guard, inputs);
                }
            }
        }
        RuleCondition::AnyMatches(_) | RuleCondition::NoMatches(_) => {}
    }
}

#[cfg(feature = "solver")]
fn collect_solver_inputs_from_guard(guard: &Guard, inputs: &mut BTreeSet<InputId>) {
    if let Guard::InputIs(input) = guard {
        inputs.insert(*input);
    }
}

#[cfg(feature = "solver")]
fn is_solver_control_input(name: &str) -> bool {
    matches!(name, "undo" | "restart" | "next_level" | "previous_level")
}

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

#[cfg(feature = "solver")]
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

fn push_session_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"gameState\":{");
    let values = session.session_values();
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, loaded, session, session.focused_scene(), value);
    }
    out.push('}');
}

fn push_scene_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    push_scene_state_for(
        out,
        loaded,
        session,
        session.focused_scene(),
        session.scene_state(),
    );
}

fn push_scene_state_for(
    out: &mut String,
    loaded: &LoadedGame,
    session: &GameSession,
    scene_name: &str,
    state: Option<&puzzle_play::SceneRuntimeState>,
) {
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
        push_scene_value(out, loaded, session, scene_name, value);
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
            push_level_ref(
                out,
                loaded,
                session.cleared_levels(),
                session.focused_scene(),
                level_index,
            );
        } else {
            out.push_str("\"level\":null");
        }
        out.push('}');
    }
    out.push('}');
}

fn push_level_ref(
    out: &mut String,
    loaded: &LoadedGame,
    cleared_levels: &[bool],
    scene_name: &str,
    level_index: usize,
) {
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
        "cleared",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push(',');
    push_json_bool(
        out,
        "solved",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
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

fn push_scene_value(
    out: &mut String,
    loaded: &LoadedGame,
    session: &GameSession,
    scene_name: &str,
    value: &SceneValue,
) {
    match value {
        SceneValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        SceneValue::Int(value) => out.push_str(&value.to_string()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => push_json_string(out, value),
        SceneValue::LevelRef(index) => {
            push_level_ref(out, loaded, session.cleared_levels(), scene_name, *index)
        }
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
        push_scene_state_for(out, loaded, session, name, state);
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
    push_puzzle_settings(out, loaded);
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

fn push_puzzle_settings(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"settings\":{");
    out.push_str("\"grid\":{");
    let occupied_cells = loaded.render.grid.occupied_cells;
    let all_cells = loaded.render.grid.all_cells;
    push_json_number(out, "visibility", u64::from(occupied_cells || all_cells));
    out.push(',');
    push_json_bool(out, "occupied_cells", occupied_cells);
    out.push(',');
    push_json_bool(out, "all_cells", all_cells);
    out.push('}');
    out.push(',');
    out.push_str("\"animation\":{");
    out.push_str("\"tween\":{");
    push_json_bool(out, "enabled", loaded.animation.tween.enabled);
    out.push(',');
    push_json_number(out, "intervalMs", loaded.animation.tween.interval_ms);
    out.push('}');
    out.push('}');
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

fn push_top_scope_context(out: &mut String, loaded: &LoadedGame, has_progress_save: bool) {
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
    out.push(',');
    push_json_bool(out, "has_progress_save", has_progress_save);
}

fn push_level_context(
    out: &mut String,
    loaded: &LoadedGame,
    cleared_levels: &[bool],
    level_index: Option<usize>,
) {
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
    out.push(',');
    push_json_bool(
        out,
        "cleared",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push(',');
    push_json_bool(
        out,
        "solved",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push('}');
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
        out.push_str("\"routines\":[");
        for (routine_index, routine) in scene.routines.iter().enumerate() {
            if routine_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_pair(out, "name", &routine.name);
            out.push(',');
            push_json_effect(out, &routine.effect);
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
    push_scene_default_value(out, &variable.default);
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

fn push_scene_default_value(out: &mut String, value: &SceneValue) {
    match value {
        SceneValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        SceneValue::Int(value) => out.push_str(&value.to_string()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => push_json_string(out, value),
        SceneValue::LevelRef(index) => out.push_str(&index.to_string()),
    }
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
        SceneComponent::Choice(choice) => {
            push_json_pair(out, "kind", "choice");
            out.push(',');
            push_json_expr_named(out, "label", &choice.label);
            out.push(',');
            push_json_effect(out, &choice.effect);
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
        SceneComponent::Conditional(conditional) => {
            push_json_pair(out, "kind", "conditional");
            out.push(',');
            push_json_pair(out, "condition", &conditional.condition);
            out.push(',');
            push_scene_children(out, &conditional.children);
            out.push(',');
            out.push_str("\"elseChildren\":");
            push_scene_component_list(out, &conditional.else_children);
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
        SceneEffect::RoutineCall(name) => {
            push_json_pair(out, "kind", "routine_call");
            out.push(',');
            push_json_pair(out, "name", name);
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
        SceneEffect::SetVariable { name, value } => {
            push_json_pair(out, "kind", "set_variable");
            out.push(',');
            push_json_pair(out, "name", name);
            out.push(',');
            push_json_expr_named(out, "value", value);
        }
        SceneEffect::ClearUndoHistory => {
            push_json_pair(out, "kind", "clear_undo_history");
        }
        SceneEffect::ClearGameProgress => {
            push_json_pair(out, "kind", "clear_game_progress");
        }
        SceneEffect::SetCurrentLevel { level } => {
            push_json_pair(out, "kind", "set_current_level");
            out.push(',');
            push_json_expr_named(out, "level", level);
        }
        SceneEffect::ClearCurrentLevel => {
            push_json_pair(out, "kind", "clear_current_level");
        }
        SceneEffect::SetLevelCleared { level, cleared } => {
            push_json_pair(out, "kind", "set_level_cleared");
            out.push(',');
            push_json_bool(out, "cleared", *cleared);
            if let Some(level) = level {
                out.push(',');
                push_json_expr_named(out, "level", level);
            }
        }
        SceneEffect::ResetPersistentVars => {
            push_json_pair(out, "kind", "reset_persistent_vars");
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
        match param {
            puzzle_lang::SceneEffectParam::Level(value) => {
                push_json_pair(out, "kind", "level");
                out.push(',');
                push_json_expr_named(out, "value", value);
            }
            puzzle_lang::SceneEffectParam::Named { name, value } => {
                push_json_pair(out, "kind", "named");
                out.push(',');
                push_json_pair(out, "name", name);
                out.push(',');
                push_json_expr_named(out, "value", value);
            }
        }
        out.push('}');
    }
    out.push(']');
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
    out.push_str("\"children\":");
    push_scene_component_list(out, children);
}

fn push_scene_component_list(out: &mut String, children: &[SceneComponent]) {
    out.push('[');
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
    Lang(DiagnosticReport),
    CoreTransition(puzzle_core::TransitionError),
    Config(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DiagnosticReport> for AppError {
    fn from(value: DiagnosticReport) -> Self {
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
    use serde_json::{Value, json};

    fn parse_json_object(source: &str) -> Value {
        serde_json::from_str(source).expect("runtime outcome should be valid JSON")
    }

    fn embedded_puzzle_export_json(html: &str) -> Value {
        let marker = "window.PuzzleExport = JSON.parse(\"";
        let start = html.find(marker).expect("html should embed PuzzleExport") + marker.len();
        let rest = &html[start..];
        let end = rest
            .find("\");")
            .expect("PuzzleExport JSON.parse should close");
        let encoded = &rest[..end];
        let json_text: String = serde_json::from_str(&format!("\"{encoded}\""))
            .expect("PuzzleExport should be a JSON string literal");
        serde_json::from_str(&json_text).expect("PuzzleExport should contain JSON")
    }

    #[test]
    fn puzzle3_scene_host_source_uses_current_2d_syntax() {
        let loaded =
            parse_game(PUZZLE3_SCENE_HOST_SOURCE).expect("puzzle3 scene host source should parse");
        assert_eq!(loaded.levels.len(), 1);
    }

    #[test]
    fn stateful_core_runtime_exposes_changed_cells_for_2d() {
        let source = r#"
animation {
  tween
}

puzzle board {
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
  level one {
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

        assert_eq!(
            outcome_json["changedCells"],
            json!([
                { "x": 0, "y": 0, "objects": [] },
                { "x": 1, "y": 0, "objects": [1] }
            ])
        );
        assert_eq!(
            outcome_json["animationEvents"],
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
        assert!(outcome_json.get("state").is_none());
        assert!(outcome_json["previousStateHandle"].is_u64());
        assert_eq!(outcome_json["globals"], json!([]));
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
  level one {
    P.
  }
}
"#;
        let mut runtime = Puzzle3RuntimeBridge::from_source(source).expect("load 3D runtime");
        let state = runtime
            .parsed
            .level_bundle
            .as_ref()
            .expect("level bundle")
            .levels[0]
            .level
            .build_state(&runtime.parsed.game)
            .expect("build state");
        let mut state_json = String::new();
        push_state3_data(&mut state_json, &state);
        runtime
            .set_state_json(&state_json)
            .expect("set current state");

        let outcome = runtime
            .transition_current_outcome_json("main", 4)
            .expect("transition current state");
        let outcome_json = parse_json_object(&outcome);

        assert_eq!(outcome_json["changed"], true);
        assert!(outcome_json.get("state").is_none());
        assert_eq!(
            outcome_json["changedCells"],
            json!([
                { "position": { "x": 0, "y": 0, "z": 0 }, "objects": [] },
                { "position": { "x": 1, "y": 0, "z": 0 }, "objects": [1] }
            ])
        );
        assert!(PUZZLE3_APP_JS.contains("this.applyRuntimeCells(outcome.changedCells || []);"));
    }

    #[test]
    fn renderer_board_floor_is_transparent_by_default() {
        assert!(RENDERER_CSS.contains("--cell-background: transparent;"));
        assert!(RENDERER_JS.contains("floorColor && floorColor !== \"transparent\""));
    }

    #[test]
    fn renderer_paints_tween_layers_after_static_board_layers() {
        assert!(RENDERER_JS.contains("let startedAt = null;"));
        assert!(RENDERER_JS.contains("if (!this.root.isConnected)"));
        assert!(RENDERER_JS.contains("startedAt ??= performance.now();"));
        assert!(RENDERER_JS.contains("requestAnimationFrame(draw);"));
        assert!(RENDERER_JS.contains("const animatedLayers = [];"));
        assert!(RENDERER_JS.contains("animatedLayers.push({ layer, x, y, animation });"));
        assert!(RENDERER_JS.contains("for (const item of animatedLayers)"));
        assert!(RENDERER_JS.contains(
            "this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress);"
        ));
        assert!(!RENDERER_JS.contains("animationForVisualCompanion"));
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
    fn generated_visuals_include_sprite_translate_offset() {
        let source = r#"
title sprite_translate
puzzle default {
layers {
actor = Player
}
sprites {
Player {
pixels_per_cell 5 5
offset 2 -1
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
level start {
P
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let visuals = generated_visuals_js(&loaded);

        assert!(visuals.contains("\"offset\":{\"x\":2,\"y\":-1}"));
        assert!(visuals.contains("\"pixelsPerCell\":{\"width\":5,\"height\":5}"));
        assert!(RENDERER_JS.contains("visualSpriteOffset(definition, unit)"));
        assert!(RENDERER_JS.contains("definition.pixelsPerCell?.width"));
        assert!(RENDERER_JS.contains("solidColor && this.canPaintAsFullCellSolid(definition)"));
        assert!(RENDERER_JS.contains("unit = this.leastCommonMultiple(unit, cellCols);"));
        assert!(!RENDERER_JS.contains("boundedLeastCommonMultiple"));
        assert!(RENDERER_CSS.contains("overflow: visible;"));
    }

    #[test]
    fn renderer_consumes_2d_render_grid_settings() {
        let source = r#"
title grid_render
puzzle default {
layers {
actor = Player
}
render {
grid occupied_cells all_cells
}
rules {

}
levels {
legend {
. = empty
P = Player
}
level start {
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
            r#""settings":{"grid":{"visibility":1,"occupied_cells":true,"all_cells":true},"animation":{"tween":{"enabled":false,"intervalMs":250}}}"#
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
        assert!(APP_JS.contains("function syncScreenScale()"));
        assert!(APP_JS.contains("const defaultSceneLogicalSize = { width: 4, height: 3 };"));
        assert!(APP_JS.contains("function fitLogicalSceneSize("));
        assert!(APP_JS.contains("const defaultSceneLayoutUnit = 180;"));
        assert!(APP_JS.contains("function virtualSceneSize("));
        assert!(APP_JS.contains("screenView.style.setProperty(\"--screen-scale\""));
        assert!(APP_JS.contains("screenFrame.style.width"));
        assert!(APP_JS.contains("screenFrame.style.height"));
        assert!(APP_CSS.contains("transform: scale(var(--screen-scale, 1));"));
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
    fn html_play_standard_choice_focus_uses_logical_grid() {
        assert!(APP_JS.contains("function standardChoiceFocusCells(scene = currentSceneDef())"));
        assert!(APP_JS.contains("function sceneMenuFocusCells(scene = currentSceneDef())"));
        assert!(APP_JS.contains("function levelMenuFocusFootprint(component, context = {})"));
        assert!(APP_JS.contains("const hasMenuController = sceneHasComponent(scene, \"level_menu\") || sceneHasComponent(scene, \"choice\");"));
        assert!(
            APP_JS.contains(
                "if (!binding && chrome === \"menu\" && profile.menuFocusCells.length > 0)"
            )
        );
        assert!(APP_JS.contains("effects.push({ kind: \"scene_menu\", input: menuInput });"));
        assert!(APP_JS.contains("syncSceneMenuSelection(screenView);"));
        assert!(APP_JS.contains("assignSceneMenuControl(button, scope);"));
        assert!(APP_JS.contains("function isControlPointerTarget(target)"));
        assert!(APP_JS.contains("if (isControlPointerTarget(event.target))"));
        assert!(APP_JS.contains("function componentRowFootprint(components, context = {})"));
        assert!(APP_JS.contains("function componentColumnFootprint(components, context = {})"));
        assert!(APP_JS.contains("component.kind === \"choice\""));
        assert!(APP_JS.contains("(focusKind === \"menu\" && component.kind === \"button\")"));
        assert!(APP_JS.contains("focusKind === \"menu\" && component.kind === \"level_menu\""));
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
        assert!(APP_CSS.contains("--button-shadow:"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow);"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow-hover);"));
        assert!(APP_CSS.contains("box-shadow: var(--button-shadow-active);"));
        assert!(!APP_CSS.contains("--menu-control-width: 420px;"));
        assert!(THEME_PRESETS_CSS.contains("body.theme-clean {"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow-hover: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-shadow-active: none;"));
        assert!(THEME_PRESETS_CSS.contains("--button-hover-transform: none;"));
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
    fn html_play_queues_busy_inputs_instead_of_dropping_them() {
        assert!(APP_JS.contains("pendingCommandQueue.push(command);"));
        assert!(
            APP_JS.contains("pendingCommandQueue.push({ kind: \"model_input\", name: input });")
        );
        assert!(APP_JS.contains("currentState?.busy || clientPendingWaits > 0"));
        assert!(
            !APP_JS.contains("if (currentState.busy) {\n    return;\n  }\n  broadcastPuzzle3Key")
        );
        assert!(!APP_JS.contains("clientPendingAnimations"));
        assert!(!APP_JS.contains("clientPendingCommands"));
    }

    #[test]
    fn html_play_message_popup_uses_explicit_default_dismiss_keys() {
        assert!(APP_JS.contains("function isMessageDismissKey(event)"));
        assert!(APP_JS.contains("rawKey === \"Enter\""));
        assert!(APP_JS.contains("rawKey === \" \""));
        assert!(APP_JS.contains("key === \"x\""));
        assert!(APP_JS.contains("if (messagePopup) {\n    event.preventDefault();\n    if (isMessageDismissKey(event)) {\n      closeMessagePopup();\n    }\n    return;\n  }"));
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
        assert!(!APP_JS.contains("createPuzzleScriptSfxPlayer"));
        assert!(!APP_JS.contains("generatePuzzleScriptSoundEffect"));
    }

    #[test]
    fn standalone_export_includes_sfx_volume() {
        let source = r#"
title Sfx Volume

sounds {
  sfx click seed=click type=select volume=0.25
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
  level one {
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

        assert!(html.contains(r#"\"sfx\":[{\"name\":\"click\",\"seed\":\"click\",\"type\":\"select\",\"volume\":0.25}]"#));
    }

    #[test]
    fn html_play_does_not_fallback_scene_definitions_to_global_export() {
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
    fn mixed_export_hosts_puzzle3_as_scene_component() {
        let source = r#"
title Mixed Game

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
  layout size 4 3 {
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
            StandaloneHostMode::Export,
        )
        .unwrap();
        assert!(html.contains("window.Puzzle3DFrameFixture"));
        assert!(html.contains("window.Puzzle3DFrameAssets"));
        assert!(!html.contains("\"themeCss\""));
        assert!(html.contains("case \"puzzle3\""));
        assert!(html.contains("window.Puzzle3ControllerAutoBoot = false"));
        assert!(html.contains("window.Puzzle3Controller"));
        assert!(!html.contains("iframe.puzzle3-frame"));
        assert!(html.contains("\\\"kind\\\":\\\"puzzle3\\\""));
        assert!(html.contains("\\npuzzle3 cube"));
        assert!(!html.contains("\\nscene mixed_play"));
        assert!(!html.contains("\\npuzzle3 cube_board"));
    }

    #[test]
    fn mixed_microban_scene_metadata_stays_model_agnostic() {
        let source = r#"
title Mixed Microban

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

level microban_01 {
P.
}

level microban_02 {
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

level microban_03 {
P.
}

level microban_04 {
.P
}
}

scene level_select {
layout {
title "Microban"
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
        let document = puzzle_lang::parse_game(source).unwrap();
        assert_eq!(document.models.len(), 2);
        assert!(matches!(
            &document.models[0],
            LoadedDocumentModel::Puzzle2d { name, game }
                if name == "microban2d"
                    && game.levels.iter().map(|level| level.name.as_str()).collect::<Vec<_>>()
                        == ["microban.microban_01", "microban.microban_02"]
        ));
        assert!(matches!(
            &document.models[1],
            LoadedDocumentModel::Puzzle3d { name, puzzle }
                if name == "microban3d"
                    && puzzle.level_bundle.as_ref().is_some_and(|bundle| {
                        bundle.level(0).is_some_and(|level| level.name == "microban_03")
                            && bundle.level(1).is_some_and(|level| level.name == "microban_04")
                    })
        ));

        let level_select = document
            .scenes
            .iter()
            .find(|scene| scene.name == "level_select")
            .expect("expected level_select scene");
        assert!(level_select.state.puzzles.is_empty());
        assert!(matches!(
            level_select.components.as_slice(),
            [
                SceneComponent::Title(_),
                SceneComponent::Column(column),
            ] if matches!(
                column.children.as_slice(),
                [SceneComponent::For(for_view)]
                    if for_view.source.as_str() == "levels"
                        && matches!(
                            for_view.children.as_slice(),
                            [SceneComponent::Choice(_)]
                        )
            )
        ));

        let loaded = mixed_document_loaded_game(&document).unwrap();
        assert_eq!(
            loaded
                .levels
                .iter()
                .map(|level| level.name.as_str())
                .collect::<Vec<_>>(),
            ["microban.microban_01", "microban.microban_02"]
        );

        let mut host_data = String::new();
        push_export_data(
            &mut host_data,
            &ServerState::new(
                loaded.clone(),
                source.to_string(),
                "mixed_microban.puzzle".to_string(),
                String::new(),
                VISUALS_JS.to_string(),
                SolverConfig::default(),
            ),
        );
        let host_json: Value = serde_json::from_str(&host_data).unwrap();
        let host_level_select = host_json["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["name"] == "level_select")
            .expect("host export should keep level_select");
        assert_eq!(
            host_level_select["components"][1]["children"][0]["kind"],
            "for"
        );
        assert_eq!(
            host_level_select["components"][1]["children"][0]["children"][0]["kind"],
            "choice"
        );

        let fixture_json = mixed_document_puzzle3_fixture_json(&document).unwrap();
        let fixture: Value = serde_json::from_str(&fixture_json).unwrap();
        assert_eq!(
            fixture["levels"]
                .as_array()
                .unwrap()
                .iter()
                .map(|level| level["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["microban_03", "microban_04"]
        );
        let fixture_level_select = fixture["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["name"] == "level_select")
            .expect("3D fixture should keep level_select");
        assert_eq!(
            fixture_level_select["components"][1]["children"][0]["kind"],
            "for"
        );
        assert_eq!(
            fixture_level_select["components"][1]["children"][0]["children"][0]["kind"],
            "choice"
        );
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
        assert!(APP_JS.contains("function sceneHasModelInputTarget(scene, state = currentState"));
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
    fn solver_solution_steps_materialize_display_objects_for_display() {
        let source = r#"
title display_solver

puzzle board {
  layers {
    actor = Player
    @cursor = @Cursor
  }
  empty .
  rules {
    input right [ Player ] -> [ Player ]
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
  layout {
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

    #[cfg(feature = "solver")]
    #[test]
    fn solver_materializes_level_start_for_editor_state_with_level_index() {
        let source = r#"
title solver_level_start

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
  level start {
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
    fn solver_inputs_use_model_inputs_not_scene_or_control_inputs() {
        let source = r#"
title solver_input_scope

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
  level one {
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
  state {
    puzzle board
  }
  keys {
    Escape -> back
  }
  routine back {
    goto title
  }
  layout {
    puzzle board
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
title "Themed 3D Solver"

puzzle3 push3 {
layers {
floor = Goal
solid = Player Box Wall
}

keys {
d ArrowRight -> right
r -> restart
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

        let parsed = parse_puzzle3d_for_solver(source).unwrap();
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

        assert!(outcome.contains(r#""slots":[2,0,1]"#));
    }

    #[test]
    fn standalone_export_embeds_game_wasm_runtime() {
        let source = r#"
	title Wasm Export
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
  level one {
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
        assert!(html.contains("WasmStandaloneSession"));
        assert!(!html.contains("WasmCoreRuntime"));
        assert!(!html.contains("compile_preview"));
        assert!(!html.contains("highlight_source_html"));
        assert!(!html.contains("suggest_source_completions"));
        assert!(!html.contains("solve_state"));
        assert!(!html.contains("solve_state_with_progress"));
        assert!(!html.contains("PuzzleStudioSolve"));
        assert!(!html.contains("PuzzleStudioPreviewState"));
        assert!(!html.contains("PuzzleStudioScenePreview"));
        assert!(!html.contains("loadWasmSolver"));
        assert!(!html.contains("renderPuzzle3Frame"));
        assert!(!html.contains("Puzzle3DFrameAssets"));
        assert!(STANDALONE_JS.contains("loadRuntimeModule()"));
        assert!(STANDALONE_JS.contains("initializeSessionRuntime()"));
        assert!(STANDALONE_JS.contains("WasmStandaloneSession.fromExport(JSON.stringify"));
        assert!(!STANDALONE_JS.contains("new this.wasmModule.WasmStandaloneSession("));
        assert!(STANDALONE_JS.contains("Puzzle game WASM runtime is unavailable."));
        assert!(STANDALONE_JS.contains("set_current_state("));
        assert!(STANDALONE_JS.contains("Editor preview state requires a valid level index."));
        assert!(!STANDALONE_JS.contains("this.initializeCoreRuntime();"));
        assert!(!STANDALONE_JS.contains("WasmCoreRuntime"));
        assert!(!STANDALONE_JS.contains("WasmCompiledCoreRuntime"));
    }

    #[test]
    fn editor_preview_export_keeps_studio_bridge_for_state_control() {
        let source = r#"
title Editor Preview Export

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
        assert!(html.contains("set_current_state("));
        assert!(!html.contains("broadcastPuzzle3Key"));
        assert!(!html.contains("PuzzleStudioSolve"));
        assert!(!html.contains("loadWasmSolver"));
    }

    #[test]
    fn standalone_export_includes_scene_and_screen_keys() {
        let source = r#"
title Export Test

puzzle default {
layers {
actor = Player
}

levels {
    legend {
        . = empty
        P = Player
    }

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
    layout {
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
title Progress Export

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

    level one
    P

    level two
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
        assert!(data.contains(r#""globals":[{"id":0,"name":"bonus"}]"#));
        assert!(data.contains(r#""persistentVars":[0]"#));
        assert!(STANDALONE_JS.contains("WasmStandaloneSession"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.request_json(method, url)"));
        assert!(STANDALONE_JS.contains("snapshot()"));
        assert!(STANDALONE_JS.contains("restoreSessionProgressSave()"));
        assert!(STANDALONE_JS.contains("writeSessionProgressSave()"));
        assert!(APP_JS.contains("animationEvents: event.data.animationEvents"));
        assert!(APP_JS.contains("standaloneRuntime.snapshot({ forceJs: true })"));
        assert!(STANDALONE_JS.contains("this.sessionRuntime.progress_save()"));
        assert!(STANDALONE_JS.contains("window.localStorage?.setItem"));
        assert!(STANDALONE_JS.contains("window.localStorage?.getItem"));
        assert!(!STANDALONE_JS.contains("progressSaveData()"));
        assert!(!STANDALONE_JS.contains("restoreProgressSave()"));
        assert!(!STANDALONE_JS.contains("writeProgressSave()"));
        assert!(!STANDALONE_JS.contains("clearedLevels[index]"));
        assert!(!STANDALONE_JS.contains("currentSaveLevelName()"));
        assert!(!STANDALONE_JS.contains("persistentVarSaveData()"));
    }

    #[test]
    fn standalone_session_bridge_uses_rust_session_for_requests() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();

        let title = bridge.request_json("GET", "/api/state").unwrap();
        let title: serde_json::Value = serde_json::from_str(&title).unwrap();
        assert_eq!(title["currentScene"], "title");
        assert_eq!(title["title"], "Microban Basic");
        let title = title.as_object().unwrap();
        assert!(title.contains_key("visibleScenes"));
        assert!(title.contains_key("sceneState"));
        assert!(title.contains_key("scenePuzzles"));
        assert!(!title.contains_key("visibleScreens"));
        assert!(!title.contains_key("screenState"));
        assert!(!title.contains_key("screenPuzzles"));

        let playing = bridge
            .request_json("POST", "/api/command/goto%20playing")
            .unwrap();
        let playing: serde_json::Value = serde_json::from_str(&playing).unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let save: serde_json::Value = serde_json::from_str(&bridge.progress_save_json()).unwrap();
        assert_eq!(save["currentLevel"], "microban.1");
    }

    #[test]
    fn standalone_session_bridge_emits_fixban_tween_on_first_input() {
        let source = include_str!("../../../games/fixban_tween.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/fixban_tween.puzzle").unwrap();

        let playing = bridge
            .request_json("POST", "/api/command/goto%20playing(fixban.level_1)")
            .unwrap();
        let playing: serde_json::Value = serde_json::from_str(&playing).unwrap();
        assert_eq!(playing["currentScene"], "playing");
        assert_eq!(playing["levelIndex"], 0);

        let moved = bridge.request_json("POST", "/api/input/up").unwrap();
        let moved: serde_json::Value = serde_json::from_str(&moved).unwrap();
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
    fn standalone_session_bridge_restores_progress_save() {
        let source = include_str!("../../../games/spec_2d.puzzle");
        let mut bridge =
            StandaloneSessionBridge::from_source(source, "games/spec_2d.puzzle").unwrap();
        bridge
            .restore_progress_save_json(
                r#"{"version":1,"levels":[{"name":"microban.2","cleared":true}],"currentLevel":"microban.2","persistentVars":[]}"#,
            )
            .unwrap();

        let snapshot = bridge.request_json("GET", "/api/state").unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(snapshot["selectedLevelIndex"], 1);
        assert_eq!(snapshot["has_progress_save"], true);
        assert_eq!(snapshot["levels"][1]["cleared"], true);
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
layers {
actor = Player
}

levels {
    legend {
        . = empty
        P = Player
    }

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

        assert!(html.contains(r#"<body class="theme-noir" style="--background:#123456;">"#));
    }

    #[test]
    fn standalone_export_supports_single_puzzle3_document() {
        let source = include_str!("../../../games/spec_3d.puzzle3");
        let html = export_html_from_source(
            source,
            "games/spec_3d.puzzle3",
            "body { --accent: #123456; }",
            "",
        )
        .expect("export puzzle3 document");

        assert!(html.contains("window.Puzzle3DFixture"));
        assert!(html.contains("WasmPuzzle3Runtime"));
        assert!(html.contains("WasmStandaloneSession"));
        assert!(!html.contains("Puzzle3DTestRuntime"));
        assert!(html.contains("Microban 3D"));
        assert!(html.contains("--accent: #123456"));
        let mut bridge = StandaloneSessionBridge::from_source(source, "games/spec_3d.puzzle3")
            .expect("single puzzle3 document should have a scene host game runtime");
        let snapshot: Value = serde_json::from_str(&bridge.snapshot_json()).unwrap();
        assert_eq!(snapshot["currentScene"], json!("title"));
    }

    #[test]
    fn puzzle3_export_embeds_frame_source_and_path_as_strings() {
        let source = r#"title "Tiny"

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
  state {
    board = puzzle3 cube
  }
  layout {
    puzzle3 board
  }
}

levels3 default of cube {
  legend {
    P = Player
  }
  level one {
    P
  }
}
"#;
        let html = export_html_from_source(source, "games/tiny.puzzle3", "", "")
            .expect("export puzzle3 document");

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
            PUZZLE3_THREE_RENDERER_JS.contains("function spriteVoxelLocalPosition(voxel, step)")
        );
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("x: (voxel.x + 0.5) * step - 0.5"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("y: (voxel.y + 0.5) * step - 0.5"));
        assert!(PUZZLE3_THREE_RENDERER_JS.contains("z: (voxel.z + 0.5) * step - 0.5"));
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
        assert!(html.contains("\\\"kind\\\":\\\"choice\\\""));
        assert!(html.contains("\\\"kind\\\":\\\"puzzle3\\\""));
        assert!(html.contains("\\npuzzle3 cube"));
        assert!(!html.contains("\"source\":\"title \\\\\\\"Tiny\\\\\\\"\\n"));
        assert!(html.contains("\"puzzlePath\":\"games/tiny.puzzle3\""));
        assert!(!html.contains("window.Puzzle3DSource ="));
        assert!(!html.contains("window.Puzzle3DPath ="));
    }

    #[test]
    fn puzzle3_frame_export_keeps_component_document_transparent() {
        let source = r##"title "Themed 3D"
theme clean {
  background_color #123456
}

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}

scene playing {
  state {
    board = puzzle3 cube
  }
  layout {
    puzzle3 board
  }
}

levels3 default of cube {
  legend {
    P = Player
  }
  level one {
    P
  }
}
"##;
        let html = export_html_from_source(source, "games/themed_3d.puzzle3", "", "")
            .expect("export themed puzzle3 document");
        let export = embedded_puzzle_export_json(&html);

        assert!(html.contains(r#"<body class="theme-clean" style="--background:#123456;">"#));
        assert_eq!(export["theme"]["name"], json!("clean"));
        assert_eq!(export["theme"]["variables"]["background"], json!("#123456"));
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
title "Screenshot"

puzzle3 cube {
  layers {
    actor = Player
  }
  rules {
  }
}

scene title {
  layout {
    title "Screenshot"
    button "Play" -> goto playing
  }
}

scene playing {
  state {
    board = puzzle3 cube
  }
  layout {
    puzzle3 board
  }
}

levels3 basic of cube {
  legend {
    P = Player
  }
  level one {
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
