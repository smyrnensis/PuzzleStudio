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
    let mut scripts = vec![
        asset_resolver_js(puzzle_path, &loaded.assets)?,
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
fn asset_resolver_js(puzzle_path: &Path, assets: &AssetsDef) -> Result<String, AppError> {
    let parent = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = String::new();
    files.push('{');
    let mut first = true;
    for asset in assets
        .entries
        .iter()
        .filter(|asset| asset.kind == AssetKind::File)
    {
        let path = resolve_asset_path(parent, &asset.path)?;
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
