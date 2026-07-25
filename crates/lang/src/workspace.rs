use serde::{Deserialize, Serialize};

use puzzle_assets::VisualImageAssetManifestEntry;

use crate::{
    AssetKind, DiagnosticReport, LoadedDocument, LoadedDocumentModel, VisualKind,
    expand_game_imports_from_documents_with_origins, parse_game_for_path,
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
    pub visual_image_assets: Vec<VisualImageAssetManifestEntry>,
}

pub fn workspace_presentation_manifest(
    entry_path: &str,
    documents: &[WorkspaceSourceDocument],
) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
    let expanded = expand_game_imports_from_documents_with_origins(entry_path, documents)?;
    let document = parse_game_for_path(&expanded.source, entry_path)
        .map_err(|report| expanded.remap_diagnostic_report(report))?;
    loaded_document_presentation_manifest(&document)
}

pub fn loaded_document_presentation_manifest(
    document: &LoadedDocument,
) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
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

    let mut visual_image_assets = Vec::new();
    for model in &document.models {
        let visuals = match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => &game.visuals,
            LoadedDocumentModel::Puzzle3d { game, .. } => &game.visuals,
        };
        for visual in &visuals.entries {
            let VisualKind::Image { asset } = &visual.kind else {
                continue;
            };
            if visual_image_assets
                .iter()
                .all(|existing: &VisualImageAssetManifestEntry| existing.id != asset.id)
            {
                visual_image_assets.push(asset.clone());
            }
        }
    }

    Ok(WorkspacePresentationManifest {
        theme_name: document.theme.name.clone(),
        css_paths,
        script_paths,
        file_paths,
        visual_image_assets,
    })
}
