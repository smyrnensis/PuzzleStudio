use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use bevy::prelude::*;
use puzzle_assets::{DecodedVisualImageCatalog, decode_visual_image_files};
use puzzle_bevy_player::{PuzzleBevyPlayerHost, install_puzzle_bevy_player};
use puzzle_lang::{WorkspaceSourceDocument, workspace_presentation_manifest};

fn main() -> Result<(), Box<dyn Error>> {
    let puzzle_path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: puzzle-bevy-player <game.puzzle|game.puzzle3>",
        )
    })?;
    let loaded = load_native_game(Path::new(&puzzle_path))?;
    let host = PuzzleBevyPlayerHost::from_source_with_visual_images(
        &loaded.expanded_source,
        &loaded.canonical_entry_path,
        Arc::new(loaded.image_catalog),
    )?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("PuzzleStudio Bevy — {puzzle_path}"),
            canvas: Some("#puzzle-bevy".to_string()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));
    install_puzzle_bevy_player(&mut app, host);
    app.run();
    Ok(())
}

struct LoadedNativeGame {
    canonical_entry_path: String,
    expanded_source: String,
    image_catalog: DecodedVisualImageCatalog,
}

fn load_native_game(entry_path: &Path) -> Result<LoadedNativeGame, Box<dyn Error>> {
    let canonical_entry = fs::canonicalize(entry_path)
        .map_err(|error| contextual_io_error("resolve puzzle entry", entry_path, error))?;
    let game_root = canonical_entry.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "puzzle entry `{}` has no game root directory",
                canonical_entry.display()
            ),
        )
    })?;
    let canonical_entry_path = canonical_entry
        .to_str()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "puzzle entry path is not valid UTF-8: {}",
                    canonical_entry.display()
                ),
            )
        })?
        .to_string();
    let source = fs::read_to_string(&canonical_entry)
        .map_err(|error| contextual_io_error("read puzzle entry", &canonical_entry, error))?;
    let expanded_source =
        puzzle_lang::expand_game_imports_for_file_under_root(&source, &canonical_entry, game_root)?;
    let manifest = workspace_presentation_manifest(
        &canonical_entry_path,
        &[WorkspaceSourceDocument {
            path: canonical_entry_path.clone(),
            source: expanded_source.clone(),
        }],
    )?;
    let image_files = manifest
        .visual_image_assets
        .into_iter()
        .map(|asset| {
            let asset_path = resolve_game_asset_path(game_root, &asset.path)?;
            let bytes = fs::read(&asset_path)
                .map_err(|error| contextual_io_error("read visual image", &asset_path, error))?;
            Ok((asset, bytes))
        })
        .collect::<Result<Vec<_>, io::Error>>()?;
    let image_catalog = decode_visual_image_files(image_files)?;
    Ok(LoadedNativeGame {
        canonical_entry_path,
        expanded_source,
        image_catalog,
    })
}

fn resolve_game_asset_path(game_root: &Path, game_relative_path: &str) -> io::Result<PathBuf> {
    let requested = game_root.join(game_relative_path);
    let resolved = fs::canonicalize(&requested)
        .map_err(|error| contextual_io_error("resolve visual image", &requested, error))?;
    if !resolved.starts_with(game_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "visual image `{game_relative_path}` resolves outside game root `{}`: {}",
                game_root.display(),
                resolved.display()
            ),
        ));
    }
    Ok(resolved)
}

fn contextual_io_error(action: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{action} `{}` failed: {error}", path.display()),
    )
}
