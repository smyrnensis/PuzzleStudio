#[cfg(not(target_arch = "wasm32"))]
fn load_game_css(puzzle_path: &Path, loaded: &LoadedGame) -> Result<String, AppError> {
    load_asset_css(puzzle_path, &loaded.assets)
}

fn load_visual_image_bundle_for_export(
    document: &puzzle_lang::LoadedDocument,
    puzzle_path: &str,
) -> Result<EncodedVisualImageBundle, DiagnosticReport> {
    let manifest = puzzle_lang::loaded_document_presentation_manifest(document)?;
    if manifest.visual_image_assets.is_empty() {
        return Ok(EncodedVisualImageBundle::default());
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = puzzle_path;
        Err(DiagnosticReport::error(
            "standalone export with visual images requires a filesystem asset host",
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let entry = fs::canonicalize(puzzle_path).map_err(|error| {
            DiagnosticReport::error(format!(
                "standalone visual image export could not resolve puzzle entry `{puzzle_path}`: {error}"
            ))
        })?;
        let game_root = entry.parent().ok_or_else(|| {
            DiagnosticReport::error(format!(
                "standalone visual image export puzzle entry has no game root: `{}`",
                entry.display()
            ))
        })?;
        let mut assets = Vec::with_capacity(manifest.visual_image_assets.len());
        for image in manifest.visual_image_assets {
            let requested = game_root.join(&image.path);
            let resolved = fs::canonicalize(&requested).map_err(|error| {
                DiagnosticReport::error(format!(
                    "standalone visual image `{}` could not be resolved under game root `{}`: {error}",
                    image.path,
                    game_root.display()
                ))
            })?;
            if !resolved.starts_with(game_root) {
                return Err(DiagnosticReport::error(format!(
                    "standalone visual image `{}` resolves outside game root `{}`: {}",
                    image.path,
                    game_root.display(),
                    resolved.display()
                )));
            }
            let bytes = fs::read(&resolved).map_err(|error| {
                DiagnosticReport::error(format!(
                    "standalone visual image `{}` could not be read: {error}",
                    image.path
                ))
            })?;
            assets.push(
                EncodedVisualImageAsset::new(image, bytes)
                    .map_err(|error| DiagnosticReport::error(error.to_string()))?,
            );
        }
        Ok(EncodedVisualImageBundle { assets })
    }
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
    let mut scripts = vec![
        asset_resolver_js(puzzle_path, loaded)?,
        VISUALS_JS.to_string(),
    ];
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
fn asset_resolver_js(puzzle_path: &Path, loaded: &LoadedGame) -> Result<String, AppError> {
    let parent = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = String::new();
    files.push('{');
    let mut first = true;
    let mut paths = loaded
        .assets
        .entries
        .iter()
        .filter(|asset| asset.kind == AssetKind::File)
        .map(|asset| asset.path.clone())
        .collect::<Vec<_>>();
    for visual in &loaded.visuals.entries {
        if let VisualKind::Image { asset } = &visual.kind
            && !paths.iter().any(|path| path == &asset.path)
        {
            paths.push(asset.path.clone());
        }
    }
    for asset_path in paths {
        let path = resolve_asset_path(parent, &asset_path)?;
        push_asset_resolver_entry(parent, &path, &mut files, &mut first)?;
    }
    files.push('}');
    Ok(format!(
        "window.PuzzleAssets = {{ files: {files}, url(path) {{ const key = String(path || '').replaceAll('\\\\\\\\', '/'); if (Object.prototype.hasOwnProperty.call(this.files, key)) return this.files[key]; if (/^(?:data:|https?:|#)/.test(key)) return key; throw new Error(`Puzzle asset is not embedded: ${{key}}. Declare it with file \\\"${{key}}\\\" in assets.`); }} }};"
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn push_asset_resolver_entry(
    root: &Path,
    path: &Path,
    files: &mut String,
    first: &mut bool,
) -> Result<(), AppError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        AppError::Config(format!(
            "asset file is outside game folder: {}",
            path.display()
        ))
    })?;
    let name = relative.to_str().ok_or_else(|| {
        AppError::Config(format!("asset path is not valid UTF-8: {}", path.display()))
    })?;
    if !*first {
        files.push(',');
    }
    *first = false;
    push_json_string(files, &name.replace('\\', "/"));
    files.push(':');
    let url = if is_text_file(path) {
        format!(
            "data:{};charset=utf-8,{}",
            mime_type(path),
            percent_encode(&fs::read_to_string(path)?)
        )
    } else {
        format!(
            "data:{};base64,{}",
            mime_type(path),
            base64_encode(&fs::read(path)?)
        )
    };
    push_json_string(files, &url);
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
