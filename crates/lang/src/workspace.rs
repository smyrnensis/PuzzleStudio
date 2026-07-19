use serde::{Deserialize, Serialize};

use crate::{
    AssetKind, DiagnosticReport, LoadedDocumentModel, VisualSpriteKind,
    expand_game_imports_from_documents, parse_game_for_path,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSourceDocument {
    pub path: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePresentationManifest {
    pub theme_name: Option<String>,
    pub css_paths: Vec<String>,
    pub script_paths: Vec<String>,
    pub file_paths: Vec<String>,
    pub sprite_image_paths: Vec<String>,
}

pub fn workspace_presentation_manifest(
    entry_path: &str,
    documents: &[WorkspaceSourceDocument],
) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
    let source = expand_game_imports_from_documents(entry_path, documents)?;
    let document = parse_game_for_path(&source, entry_path)?;
    let mut css_paths = Vec::new();
    let mut script_paths = Vec::new();
    let mut file_paths = Vec::new();
    for asset in &document.assets.entries {
        match asset.kind {
            AssetKind::Css => css_paths.push(asset.path.clone()),
            AssetKind::Script => script_paths.push(asset.path.clone()),
            AssetKind::File => file_paths.push(asset.path.clone()),
        }
    }

    let mut sprite_image_paths = Vec::new();
    for model in &document.models {
        let visuals = match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => &game.visuals,
            LoadedDocumentModel::Puzzle3d { game, .. } => &game.visuals,
        };
        for sprite in &visuals.sprites {
            let VisualSpriteKind::Image { source } = &sprite.kind else {
                continue;
            };
            if !sprite_image_paths.contains(source) {
                sprite_image_paths.push(source.clone());
            }
        }
    }

    Ok(WorkspacePresentationManifest {
        theme_name: document.theme.name,
        css_paths,
        script_paths,
        file_paths,
        sprite_image_paths,
    })
}
