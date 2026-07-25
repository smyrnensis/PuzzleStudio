use std::{collections::BTreeMap, fmt, io::Cursor, path::Path};

use image::{ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualImageFormat {
    Png,
    Jpeg,
}

impl VisualImageFormat {
    pub fn from_path(path: &str) -> Result<Self, ImageAssetError> {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("png") => Ok(Self::Png),
            Some("jpg" | "jpeg") => Ok(Self::Jpeg),
            _ => Err(ImageAssetError::UnsupportedFormat {
                path: path.to_string(),
            }),
        }
    }

    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualImageAssetManifestEntry {
    pub id: VisualImageAssetId,
    pub path: String,
    pub format: VisualImageFormat,
}

impl<'de> Deserialize<'de> for VisualImageAssetManifestEntry {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct SerializedManifest {
            id: VisualImageAssetId,
            path: String,
            format: VisualImageFormat,
        }

        let serialized = SerializedManifest::deserialize(deserializer)?;
        let expected =
            Self::from_path(serialized.path.clone()).map_err(serde::de::Error::custom)?;
        if serialized.id != expected.id || serialized.format != expected.format {
            return Err(serde::de::Error::custom(ImageAssetError::InvalidManifest {
                path: serialized.path,
            }));
        }
        Ok(expected)
    }
}

impl VisualImageAssetManifestEntry {
    pub fn from_path(path: impl Into<String>) -> Result<Self, ImageAssetError> {
        let path = path.into();
        let id = normalized_asset_id(&path)?;
        Ok(Self {
            id,
            format: VisualImageFormat::from_path(&path)?,
            path,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VisualImageAssetId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VisualImageAssetRevision(pub String);

impl VisualImageAssetRevision {
    pub fn for_decoded(width: u16, height: u16, rgba8_srgb: &[u8]) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"rgba8-srgb-straight");
        digest.update([0]);
        digest.update(width.to_le_bytes());
        digest.update(height.to_le_bytes());
        digest.update(rgba8_srgb);
        Self(format!("{:x}", digest.finalize()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodedVisualImageAsset {
    pub manifest: VisualImageAssetManifestEntry,
    pub revision: VisualImageAssetRevision,
    #[serde(with = "base64_bytes")]
    pub bytes: Vec<u8>,
}

mod base64_bytes {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer, de};

    pub fn serialize<SerializerType>(
        bytes: &[u8],
        serializer: SerializerType,
    ) -> Result<SerializerType::Ok, SerializerType::Error>
    where
        SerializerType: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Vec<u8>, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(de::Error::custom)
    }
}

impl EncodedVisualImageAsset {
    pub fn new(
        manifest: VisualImageAssetManifestEntry,
        bytes: Vec<u8>,
    ) -> Result<Self, ImageAssetError> {
        let revision = decode_visual_image(&manifest, &bytes)?.revision;
        Ok(Self {
            manifest,
            revision,
            bytes,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodedVisualImageBundle {
    pub assets: Vec<EncodedVisualImageAsset>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedVisualImageAsset {
    pub id: VisualImageAssetId,
    pub revision: VisualImageAssetRevision,
    pub width: u16,
    pub height: u16,
    /// Canonical straight-alpha RGBA8 samples in the sRGB color space.
    /// Fully transparent samples have canonical zero RGB channels.
    pub rgba8_srgb: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DecodedVisualImageCatalog {
    assets: BTreeMap<VisualImageAssetId, DecodedVisualImageAsset>,
}

impl DecodedVisualImageCatalog {
    pub fn get(&self, id: &VisualImageAssetId) -> Option<&DecodedVisualImageAsset> {
        self.assets.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DecodedVisualImageAsset> {
        self.assets.values()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageAssetError {
    InvalidPath {
        path: String,
    },
    InvalidManifest {
        path: String,
    },
    UnsupportedFormat {
        path: String,
    },
    Decode {
        path: String,
        reason: String,
    },
    InvalidDimensions {
        path: String,
        width: u32,
        height: u32,
    },
    DuplicateAssetId {
        id: VisualImageAssetId,
    },
    RevisionMismatch {
        id: VisualImageAssetId,
        declared: VisualImageAssetRevision,
        actual: VisualImageAssetRevision,
    },
}

impl fmt::Display for ImageAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath { path } => write!(
                formatter,
                "visual image asset path must be a non-empty game-relative path without parent traversal: `{path}`"
            ),
            Self::InvalidManifest { path } => write!(
                formatter,
                "visual image asset manifest id or format does not match its path: `{path}`"
            ),
            Self::UnsupportedFormat { path } => {
                write!(formatter, "visual image `{path}` must use PNG or JPEG")
            }
            Self::Decode { path, reason } => {
                write!(
                    formatter,
                    "failed to decode visual image `{path}`: {reason}"
                )
            }
            Self::InvalidDimensions {
                path,
                width,
                height,
            } => write!(
                formatter,
                "visual image `{path}` dimensions must be between 1 and {} on each axis, got {width}x{height}",
                u16::MAX
            ),
            Self::DuplicateAssetId { id } => {
                write!(
                    formatter,
                    "visual image bundle contains duplicate asset id `{}`",
                    id.0
                )
            }
            Self::RevisionMismatch {
                id,
                declared,
                actual,
            } => write!(
                formatter,
                "visual image asset `{}` revision mismatch: declared {}, computed {}",
                id.0, declared.0, actual.0
            ),
        }
    }
}

impl std::error::Error for ImageAssetError {}

fn normalized_asset_id(path: &str) -> Result<VisualImageAssetId, ImageAssetError> {
    if path.is_empty() || path.contains('\\') || path.contains("://") {
        return Err(ImageAssetError::InvalidPath {
            path: path.to_string(),
        });
    }
    let mut parts = Vec::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => {
                let part = part.to_str().ok_or_else(|| ImageAssetError::InvalidPath {
                    path: path.to_string(),
                })?;
                parts.push(part);
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(ImageAssetError::InvalidPath {
                    path: path.to_string(),
                });
            }
        }
    }
    if parts.is_empty() {
        return Err(ImageAssetError::InvalidPath {
            path: path.to_string(),
        });
    }
    Ok(VisualImageAssetId(parts.join("/")))
}

pub fn decode_visual_image_bundle(
    bundle: &EncodedVisualImageBundle,
) -> Result<DecodedVisualImageCatalog, ImageAssetError> {
    let mut assets = BTreeMap::new();
    for encoded in &bundle.assets {
        let decoded = decode_visual_image(&encoded.manifest, &encoded.bytes)?;
        if decoded.revision != encoded.revision {
            return Err(ImageAssetError::RevisionMismatch {
                id: decoded.id,
                declared: encoded.revision.clone(),
                actual: decoded.revision,
            });
        }
        insert_decoded_asset(&mut assets, decoded)?;
    }
    Ok(DecodedVisualImageCatalog { assets })
}

/// Decodes host-supplied image files directly into the immutable catalog.
///
/// Native hosts use this boundary because they do not need to construct an
/// encoded export bundle and then decode the same bytes a second time.
pub fn decode_visual_image_files<B>(
    files: impl IntoIterator<Item = (VisualImageAssetManifestEntry, B)>,
) -> Result<DecodedVisualImageCatalog, ImageAssetError>
where
    B: AsRef<[u8]>,
{
    let mut assets = BTreeMap::new();
    for (manifest, bytes) in files {
        let decoded = decode_visual_image(&manifest, bytes.as_ref())?;
        insert_decoded_asset(&mut assets, decoded)?;
    }
    Ok(DecodedVisualImageCatalog { assets })
}

fn insert_decoded_asset(
    assets: &mut BTreeMap<VisualImageAssetId, DecodedVisualImageAsset>,
    decoded: DecodedVisualImageAsset,
) -> Result<(), ImageAssetError> {
    let id = decoded.id.clone();
    if assets.insert(id.clone(), decoded).is_some() {
        return Err(ImageAssetError::DuplicateAssetId { id });
    }
    Ok(())
}

pub fn decode_visual_image(
    manifest: &VisualImageAssetManifestEntry,
    encoded: &[u8],
) -> Result<DecodedVisualImageAsset, ImageAssetError> {
    let expected = VisualImageAssetManifestEntry::from_path(manifest.path.clone())?;
    if manifest.id != expected.id || manifest.format != expected.format {
        return Err(ImageAssetError::InvalidManifest {
            path: manifest.path.clone(),
        });
    }
    let dimensions = image_reader(manifest, encoded)
        .into_dimensions()
        .map_err(|error| ImageAssetError::Decode {
            path: manifest.path.clone(),
            reason: error.to_string(),
        })?;
    let (width, height) = dimensions;
    if width == 0 || height == 0 || width > u32::from(u16::MAX) || height > u32::from(u16::MAX) {
        return Err(ImageAssetError::InvalidDimensions {
            path: manifest.path.clone(),
            width,
            height,
        });
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(u32::from(u16::MAX));
    limits.max_image_height = Some(u32::from(u16::MAX));
    let mut reader = image_reader(manifest, encoded);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| ImageAssetError::Decode {
        path: manifest.path.clone(),
        reason: error.to_string(),
    })?;
    let mut rgba8_srgb = image.into_rgba8().into_raw();
    for rgba in rgba8_srgb.chunks_exact_mut(4) {
        if rgba[3] == 0 {
            rgba[..3].fill(0);
        }
    }
    let revision = VisualImageAssetRevision::for_decoded(width as u16, height as u16, &rgba8_srgb);
    Ok(DecodedVisualImageAsset {
        id: manifest.id.clone(),
        revision,
        width: width as u16,
        height: height as u16,
        rgba8_srgb,
    })
}

fn image_reader<'a>(
    manifest: &VisualImageAssetManifestEntry,
    encoded: &'a [u8],
) -> ImageReader<Cursor<&'a [u8]>> {
    let mut reader = ImageReader::new(Cursor::new(encoded));
    reader.set_format(manifest.format.image_format());
    reader
}

#[cfg(test)]
mod tests {
    use image::{
        ImageEncoder,
        codecs::{jpeg::JpegEncoder, png::PngEncoder},
    };

    use super::*;

    fn png_bytes(pixels: &[[u8; 4]], width: u32, height: u32) -> Vec<u8> {
        let raw = pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect::<Vec<_>>();
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&raw, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        encoded
    }

    #[test]
    fn encoded_bundle_uses_compact_base64_bytes_and_round_trips() {
        let asset = EncodedVisualImageAsset {
            manifest: VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap(),
            revision: VisualImageAssetRevision("revision".to_string()),
            bytes: vec![0, 1, 2, 253, 254, 255],
        };

        let value = serde_json::to_value(&asset).unwrap();
        assert_eq!(value["bytes"], "AAEC/f7/");
        assert_eq!(
            serde_json::from_value::<EncodedVisualImageAsset>(value).unwrap(),
            asset
        );
    }

    #[test]
    fn manifest_deserialization_rejects_stale_derived_identity() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        let mut value = serde_json::to_value(manifest).unwrap();
        value["id"] = serde_json::Value::String("visuals/other.png".to_string());

        let error = serde_json::from_value::<VisualImageAssetManifestEntry>(value).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("manifest id or format does not match")
        );
    }

    #[test]
    fn public_decode_boundary_rejects_stale_manifest_identity() {
        let manifest = VisualImageAssetManifestEntry {
            id: VisualImageAssetId("visuals/other.png".to_string()),
            path: "visuals/tile.png".to_string(),
            format: VisualImageFormat::Png,
        };
        let error =
            decode_visual_image(&manifest, &png_bytes(&[[255, 0, 0, 255]], 1, 1)).unwrap_err();
        assert!(matches!(error, ImageAssetError::InvalidManifest { .. }));
    }

    #[test]
    fn decodes_png_to_exact_rgba8_srgb_lattice() {
        let expected = [[255, 0, 128, 255], [1, 2, 3, 128], [7, 8, 9, 0]];
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        let decoded = decode_visual_image(&manifest, &png_bytes(&expected, 3, 1)).unwrap();
        assert_eq!(decoded.width, 3);
        assert_eq!(decoded.height, 1);
        assert_eq!(
            decoded.rgba8_srgb,
            [
                255, 0, 128, 255, //
                1, 2, 3, 128, //
                0, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn decodes_jpeg_with_opaque_straight_alpha() {
        let mut encoded = Vec::new();
        JpegEncoder::new_with_quality(&mut encoded, 100)
            .write_image(&[20, 40, 60], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.jpeg").unwrap();
        let decoded = decode_visual_image(&manifest, &encoded).unwrap();
        assert_eq!((decoded.width, decoded.height), (1, 1));
        assert_eq!(decoded.rgba8_srgb.len(), 4);
        assert_eq!(decoded.rgba8_srgb[3], 255);
    }

    #[test]
    fn rejects_svg_visual_assets_before_decode() {
        assert_eq!(
            VisualImageAssetManifestEntry::from_path("visuals/tile.svg").unwrap_err(),
            ImageAssetError::UnsupportedFormat {
                path: "visuals/tile.svg".to_string()
            }
        );
    }

    #[test]
    fn rejects_payload_that_does_not_match_declared_format() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.jpg").unwrap();
        let png = png_bytes(&[[255, 0, 0, 255]], 1, 1);
        assert!(matches!(
            decode_visual_image(&manifest, &png),
            Err(ImageAssetError::Decode { .. })
        ));
    }

    #[test]
    fn rejects_corrupt_payload() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        assert!(matches!(
            decode_visual_image(&manifest, b"not a PNG"),
            Err(ImageAssetError::Decode { .. })
        ));
    }

    #[test]
    fn rejects_oversized_dimensions_before_rgba_decode() {
        let width = u32::from(u16::MAX) + 1;
        let raw = vec![0; usize::try_from(width).unwrap() * 4];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&raw, width, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/wide.png").unwrap();
        assert_eq!(
            decode_visual_image(&manifest, &encoded).unwrap_err(),
            ImageAssetError::InvalidDimensions {
                path: "visuals/wide.png".to_string(),
                width,
                height: 1,
            }
        );
    }

    #[test]
    fn bundle_rejects_stale_revision() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        let mut asset =
            EncodedVisualImageAsset::new(manifest, png_bytes(&[[255, 0, 0, 255]], 1, 1)).unwrap();
        asset.bytes = png_bytes(&[[0, 0, 255, 255]], 1, 1);
        let error = decode_visual_image_bundle(&EncodedVisualImageBundle {
            assets: vec![asset],
        })
        .unwrap_err();
        assert!(matches!(error, ImageAssetError::RevisionMismatch { .. }));
    }

    #[test]
    fn bundle_rejects_duplicate_stable_ids() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        let asset =
            EncodedVisualImageAsset::new(manifest, png_bytes(&[[255, 0, 0, 255]], 1, 1)).unwrap();
        let error = decode_visual_image_bundle(&EncodedVisualImageBundle {
            assets: vec![asset.clone(), asset],
        })
        .unwrap_err();
        assert!(matches!(error, ImageAssetError::DuplicateAssetId { .. }));
    }

    #[test]
    fn native_files_decode_directly_into_a_validated_catalog() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        let catalog =
            decode_visual_image_files([(manifest.clone(), png_bytes(&[[12, 34, 56, 255]], 1, 1))])
                .unwrap();
        assert!(!catalog.is_empty());
        assert_eq!(
            catalog.get(&manifest.id).unwrap().rgba8_srgb,
            [12, 34, 56, 255]
        );

        let duplicate = decode_visual_image_files([
            (manifest.clone(), png_bytes(&[[12, 34, 56, 255]], 1, 1)),
            (manifest, png_bytes(&[[12, 34, 56, 255]], 1, 1)),
        ])
        .unwrap_err();
        assert!(matches!(
            duplicate,
            ImageAssetError::DuplicateAssetId { .. }
        ));
    }

    #[test]
    fn stable_revision_depends_only_on_canonical_decoded_content() {
        let red = [255, 0, 0, 255];
        let revision = VisualImageAssetRevision::for_decoded(1, 1, &red);
        assert_eq!(revision, VisualImageAssetRevision::for_decoded(1, 1, &red));
        assert_ne!(
            revision,
            VisualImageAssetRevision::for_decoded(1, 1, &[0, 0, 255, 255])
        );
        assert_ne!(revision, VisualImageAssetRevision::for_decoded(2, 1, &red));
    }

    #[test]
    fn manifest_id_is_the_stable_normalized_asset_path() {
        let manifest = VisualImageAssetManifestEntry::from_path("visuals/tile.png").unwrap();
        assert_eq!(
            manifest.id,
            VisualImageAssetId("visuals/tile.png".to_string())
        );
        let dotted = VisualImageAssetManifestEntry::from_path("./visuals/tile.png").unwrap();
        assert_eq!(manifest.id, dotted.id);
    }

    #[test]
    fn manifest_rejects_paths_outside_the_game_asset_root() {
        assert!(matches!(
            VisualImageAssetManifestEntry::from_path("../tile.png"),
            Err(ImageAssetError::InvalidPath { .. })
        ));
        assert!(matches!(
            VisualImageAssetManifestEntry::from_path("https://example.com/tile.png"),
            Err(ImageAssetError::InvalidPath { .. })
        ));
    }
}
