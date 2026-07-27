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
