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

    let document =
        puzzle_lang::parse_game_for_path(&source, &config.puzzle_path).map_err(AppError::Lang)?;
    if !config.serve || document_uses_puzzle3_renderer(&document) {
        let output_path = config.output_path();
        let puzzle_path = config.puzzle_path.display().to_string();
        let html =
            export_bevy_document_html(&document, &puzzle_path, StandaloneRuntimeWasm::HostDefault)
                .map_err(AppError::Config)?;
        if let Some(screenshot) = &config.screenshot {
            capture_html_screenshot(&html, &screenshot.output_path, screenshot)?;
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

    let loaded = loaded_document_scene_host_loaded_game(&document).map_err(AppError::Config)?;
    let game_css = load_game_css(&config.puzzle_path, &loaded)?;
    print_warnings(&loaded);
    let game_visuals_js = load_game_visuals_js(&config.puzzle_path, &loaded)?;

    let visual_images =
        load_visual_image_bundle_for_export(&document, &config.puzzle_path.display().to_string())?;

    let state = Arc::new(Mutex::new(ServerState::new(
        document,
        loaded,
        source,
        config.puzzle_path.display().to_string(),
        visual_images,
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
        "puzzle_wasm_player",
        &[
            Path::new("crates/html_play/static/wasm_player/puzzle_wasm_player.js"),
            Path::new("crates/html_play/static/wasm_player/puzzle_wasm_player_bg.wasm"),
        ],
        &[
            Path::new("crates/wasm_player/src"),
            Path::new("crates/wasm_player/Cargo.toml"),
            Path::new("crates/player_bootstrap/src"),
            Path::new("crates/player_bootstrap/Cargo.toml"),
            Path::new("crates/assets/src"),
            Path::new("crates/assets/Cargo.toml"),
            Path::new("crates/game_runtime/src"),
            Path::new("crates/core/src"),
            Path::new("crates/lang/src"),
            Path::new("crates/play/src"),
            Path::new("crates/runtime_contract/src"),
            Path::new("crates/scene/src"),
            Path::new("crates/kernel/src"),
            Path::new("Cargo.lock"),
        ],
        "tools/build_wasm_player.sh",
    );
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
            Path::new("crates/runtime_contract/src"),
            Path::new("crates/scene/src"),
            Path::new("crates/kernel/src"),
            Path::new("Cargo.lock"),
        ],
        "tools/build_wasm_game.sh",
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
                        width: screenshot_width,
                        height: screenshot_height,
                        timeout_ms: screenshot_timeout_ms,
                        browser_path: screenshot_browser_path.clone(),
                    });
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
                        "usage: html-play [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [-o game.html] [--serve] [--port 7878] [--screenshot out.png] [--width 1280] [--height 720] [--browser path] [--solver-depth 128] [--solver-nodes 1000000] [--solver-ms N]".to_string(),
                    ));
                }
                value if value.starts_with('-') => {
                    return Err(AppError::Config(format!("unknown option: {value}")));
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
            config.width = screenshot_width;
            config.height = screenshot_height;
            config.timeout_ms = screenshot_timeout_ms;
            config.browser_path = screenshot_browser_path;
            serve = false;
        } else if screenshot_browser_path.is_some() {
            return Err(AppError::Config(
                "--browser is only valid with --screenshot".to_string(),
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
