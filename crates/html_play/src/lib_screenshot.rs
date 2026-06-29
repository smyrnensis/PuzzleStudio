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

