#[cfg(not(target_arch = "wasm32"))]
fn capture_html_screenshot(
    html: &str,
    output_path: &Path,
    config: &ScreenshotConfig,
) -> Result<(), AppError> {
    let browser_path = resolve_screenshot_browser(config.browser_path.as_deref())?;
    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temp_root = env::temp_dir();
    let stem = format!("puzzlestudio-screenshot-{}", std::process::id());
    let html_path = temp_root.join(format!("{stem}.html"));
    fs::write(&html_path, html)?;

    let browser_harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/standalone_player_browser_smoke.mjs");
    let output = Command::new("node")
        .arg(&browser_harness)
        .arg("--html")
        .arg(&html_path)
        .arg("--output")
        .arg(output_path)
        .arg("--chrome")
        .arg(&browser_path)
        .arg("--width")
        .arg(config.width.to_string())
        .arg("--height")
        .arg(config.height.to_string())
        .arg("--timeout")
        .arg(config.timeout_ms.to_string())
        .output()
        .map_err(|error| {
            AppError::Config(format!(
                "failed to launch standalone browser harness {}: {error}",
                browser_harness.display()
            ))
        })?;

    let _ = fs::remove_file(&html_path);
    if !output.status.success() {
        return Err(AppError::Config(format!(
            "standalone browser screenshot failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
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
