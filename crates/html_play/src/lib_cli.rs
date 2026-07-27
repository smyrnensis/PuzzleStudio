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
    let root = config
        .puzzle_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let workspace = puzzle_workspace::FileWorkspace::load(&config.puzzle_path, root)
        .map_err(AppError::Config)?;
    let document = workspace.compile().map_err(AppError::Lang)?;
    let output_path = config.output_path();
    let puzzle_path = config.puzzle_path.display().to_string();
    let html = export_bevy_document_html(
        &document,
        &puzzle_path,
        StandaloneRuntimeWasm::HostDefault,
    )
    .map_err(AppError::Config)?;
    if let Some(screenshot) = &config.screenshot {
        capture_html_screenshot(&html, &screenshot.output_path, screenshot)?;
        println!("screenshot {}", screenshot.output_path.display());
        return Ok(());
    }
    if config.serve {
        return serve_static_html(html, &config.puzzle_path, config.port);
    }
    fs::write(&output_path, html)?;
    println!("exported {}", output_path.display());
    Ok(())
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
                        "usage: html-play [path/to/game-folder-or-game.puzzle-or-game.puzzle] [-o game.html] [--serve] [--port 7878] [--screenshot out.png] [--width 1280] [--height 720] [--browser path]".to_string(),
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
            None => {
                return Err(AppError::Config(
                    "html-play requires an explicit .puzzle file path".to_string(),
                ));
            }
        };

        Ok(Self {
            puzzle_path,
            output_path,
            serve,
            port,
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
