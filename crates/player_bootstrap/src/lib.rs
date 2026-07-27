use std::{collections::BTreeSet, error::Error, fmt, sync::Arc};

use puzzle_assets::{DecodedVisualImageCatalog, ImageAssetError, decode_visual_image_bundle};
use puzzle_game_runtime::RuntimeSession;
use puzzle_runtime_contract::{StandaloneProgressStorage, StandaloneRuntimeExport};
use serde_json::{Map, Value};

pub struct DecodedStandalonePlayerExport {
    runtime: RuntimeSession,
    visual_images: Arc<DecodedVisualImageCatalog>,
    progress_storage: StandaloneProgressStorage,
}

pub struct DecodedEditorPreviewExport {
    player: DecodedStandalonePlayerExport,
    document: puzzle_lang::LoadedDocument,
}

impl DecodedEditorPreviewExport {
    pub fn into_parts(
        self,
    ) -> (
        RuntimeSession,
        Arc<DecodedVisualImageCatalog>,
        StandaloneProgressStorage,
        puzzle_lang::LoadedDocument,
    ) {
        let (runtime, visual_images, progress_storage) = self.player.into_parts();
        (runtime, visual_images, progress_storage, self.document)
    }
}

impl DecodedStandalonePlayerExport {
    pub fn into_parts(
        self,
    ) -> (
        RuntimeSession,
        Arc<DecodedVisualImageCatalog>,
        StandaloneProgressStorage,
    ) {
        (self.runtime, self.visual_images, self.progress_storage)
    }
}

#[derive(Debug)]
pub enum PlayerBootstrapError {
    InvalidExport(serde_json::Error),
    VisualImageManifest(String),
    VisualImageSet {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    VisualImages(ImageAssetError),
    Runtime(String),
}

impl fmt::Display for PlayerBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExport(error) => {
                write!(formatter, "standalone player export is invalid: {error}")
            }
            Self::VisualImageManifest(error) => {
                write!(
                    formatter,
                    "runtime visual image manifest is invalid: {error}"
                )
            }
            Self::VisualImageSet {
                missing,
                unexpected,
            } => write!(
                formatter,
                "standalone player visual image set does not match runtime references (missing: [{}], unexpected: [{}])",
                missing.join(", "),
                unexpected.join(", ")
            ),
            Self::VisualImages(error) => {
                write!(
                    formatter,
                    "standalone player visual images are invalid: {error}"
                )
            }
            Self::Runtime(error) => write!(formatter, "game runtime failed: {error}"),
        }
    }
}

impl Error for PlayerBootstrapError {}

pub fn decode_standalone_player_export(
    export_json: &str,
) -> Result<DecodedStandalonePlayerExport, PlayerBootstrapError> {
    let export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument> =
        serde_json::from_str(export_json).map_err(PlayerBootstrapError::InvalidExport)?;
    decode_standalone_player_export_value(export)
}

pub fn decode_editor_preview_export(
    export_json: &str,
) -> Result<DecodedEditorPreviewExport, PlayerBootstrapError> {
    let export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument> =
        serde_json::from_str(export_json).map_err(PlayerBootstrapError::InvalidExport)?;
    let document = export.runtime_loaded_document.clone();
    let player = decode_standalone_player_export_value(export)?;
    Ok(DecodedEditorPreviewExport { player, document })
}

fn decode_standalone_player_export_value(
    export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument>,
) -> Result<DecodedStandalonePlayerExport, PlayerBootstrapError> {
    let expected =
        puzzle_lang::loaded_document_presentation_manifest(&export.runtime_loaded_document)
            .map_err(|error| PlayerBootstrapError::VisualImageManifest(error.to_string()))?
            .visual_image_assets
            .into_iter()
            .map(|asset| asset.id)
            .collect::<BTreeSet<_>>();
    let actual = export
        .visual_images
        .assets
        .iter()
        .map(|asset| asset.manifest.id.clone())
        .collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(PlayerBootstrapError::VisualImageSet {
            missing: expected
                .difference(&actual)
                .map(|id| id.0.clone())
                .collect(),
            unexpected: actual
                .difference(&expected)
                .map(|id| id.0.clone())
                .collect(),
        });
    }
    let visual_images = decode_visual_image_bundle(&export.visual_images)
        .map_err(PlayerBootstrapError::VisualImages)?;
    let runtime = RuntimeSession::from_document(export.runtime_loaded_document)
        .map_err(PlayerBootstrapError::Runtime)?;
    Ok(DecodedStandalonePlayerExport {
        runtime,
        visual_images: Arc::new(visual_images),
        progress_storage: export.progress_storage,
    })
}

pub fn encode_standalone_player_export(
    export: &StandaloneRuntimeExport<puzzle_lang::LoadedDocument>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&canonicalize_json_value(serde_json::to_value(export)?))
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonicalize_json_value).collect())
        }
        Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonicalize_json_value(value)))
                    .collect::<Map<_, _>>(),
            )
        }
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use puzzle_assets::{
        EncodedVisualImageAsset, EncodedVisualImageBundle, VisualImageAssetManifestEntry,
        VisualImageAssetRevision,
    };
    use puzzle_runtime_contract::{StandaloneProgressStorage, StandaloneRuntimeExport};

    use super::*;

    const TENETEN: &str = include_str!("../../../games/TENETEN.puzzle");
    const ONE_PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5,
        0x1c, 0x0c, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64,
        0xf8, 0x0f, 0x00, 0x01, 0x05, 0x01, 0x01, 0x27, 0x18, 0xe3, 0x66, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn export_with_images(visual_images: EncodedVisualImageBundle) -> String {
        let document = puzzle_lang::parse_game_for_path(TENETEN, "games/TENETEN.puzzle").unwrap();
        encode_standalone_player_export(&StandaloneRuntimeExport::new(
            document,
            visual_images,
            StandaloneProgressStorage {
                key: "TENETEN:revision".to_string(),
                save_version: 2,
            },
        ))
        .unwrap()
    }

    #[test]
    fn canonical_export_encoder_sorts_nested_map_keys_without_reordering_arrays() {
        let canonical = canonicalize_json_value(serde_json::json!({
            "z": {"beta": 2, "alpha": 1},
            "a": [{"right": 2, "left": 1}, 3]
        }));

        assert_eq!(
            serde_json::to_string(&canonical).unwrap(),
            r#"{"a":[{"left":1,"right":2},3],"z":{"alpha":1,"beta":2}}"#
        );
    }

    #[test]
    fn canonical_export_encoder_round_trips_through_player_bootstrap() {
        let encoded = export_with_images(EncodedVisualImageBundle::default());

        decode_standalone_player_export(&encoded).unwrap();
        assert_eq!(
            encode_standalone_player_export(
                &serde_json::from_str::<StandaloneRuntimeExport<puzzle_lang::LoadedDocument>>(
                    &encoded
                )
                .unwrap()
            )
            .unwrap(),
            encoded
        );
    }

    #[test]
    fn decodes_document_images_and_storage_as_one_player_bootstrap() {
        let decoded = decode_standalone_player_export(&export_with_images(
            EncodedVisualImageBundle::default(),
        ))
        .unwrap();
        let (runtime, visual_images, progress_storage) = decoded.into_parts();

        assert!(visual_images.is_empty());
        assert_eq!(progress_storage.key, "TENETEN:revision");
        assert!(runtime.snapshot().surface.root.is_some());
    }

    #[test]
    fn referenced_image_reaches_the_decoded_catalog_with_its_content_revision() {
        let source = r#"
const title = "image_bootstrap"
puzzle default {
layers {
actor = Tile
}
visuals {
Tile {
image = "visuals/tile.png"
}
}
rules {
}
levels {
legend {
. = empty
}
level "one" {
.
}
}
}
"#;
        let document = puzzle_lang::parse_game_for_path(source, "games/image/game.puzzle").unwrap();
        let manifest = puzzle_lang::loaded_document_presentation_manifest(&document)
            .unwrap()
            .visual_image_assets
            .into_iter()
            .next()
            .unwrap();
        let encoded = EncodedVisualImageAsset::new(manifest.clone(), ONE_PIXEL_PNG.to_vec())
            .expect("one-pixel PNG is a valid referenced image");
        let revision = encoded.revision.clone();
        let export = serde_json::to_string(&StandaloneRuntimeExport::new(
            document,
            EncodedVisualImageBundle {
                assets: vec![encoded],
            },
            StandaloneProgressStorage {
                key: "image:revision".to_string(),
                save_version: 2,
            },
        ))
        .unwrap();

        let (_, catalog, _) = decode_standalone_player_export(&export)
            .unwrap()
            .into_parts();
        let decoded = catalog
            .get(&manifest.id)
            .expect("referenced image must be present in the decoded catalog");
        assert_eq!(decoded.revision, revision);
        assert_eq!((decoded.width, decoded.height), (1, 1));
    }

    #[test]
    fn rejects_unreferenced_images_instead_of_constructing_a_mismatched_runtime() {
        let error =
            match decode_standalone_player_export(&export_with_images(EncodedVisualImageBundle {
                assets: vec![EncodedVisualImageAsset {
                    manifest: VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap(),
                    revision: VisualImageAssetRevision("declared-revision".to_string()),
                    bytes: vec![0, 1, 2, 3],
                }],
            })) {
                Ok(_) => panic!("unreferenced visual image must reject the whole player bootstrap"),
                Err(error) => error,
            };

        assert!(
            error
                .to_string()
                .contains("visual image set does not match runtime references"),
            "{error}"
        );
    }
}
