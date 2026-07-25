use puzzle_assets::{EncodedVisualImageBundle, VisualImageAssetId, VisualImageAssetRevision};
pub use puzzle_audio_contract::AudioCommand;
use puzzle_core::{
    GridCompiledGame, GridCoord, GridSize, GridState, ObjectId, RuleId, Size2, Size3,
};
pub use puzzle_scene::{
    ComponentPlacement, STANDARD_MESSAGE_COMPONENT, STANDARD_MESSAGE_DISMISS_EVENT,
    STANDARD_MESSAGE_TEXT_PROPERTY,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, DeserializeOwned, MapAccess, Visitor},
};
use std::fmt;

pub const STANDALONE_PLAYER_EXPORT_VERSION: u16 = 3;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeViewportSourceId {
    pub component: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandaloneProgressStorage {
    pub key: String,
    pub save_version: u32,
}

impl<'de> Deserialize<'de> for StandaloneProgressStorage {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct SerializedStorage {
            key: String,
            save_version: u32,
        }

        let storage = SerializedStorage::deserialize(deserializer)?;
        if storage.key.trim().is_empty() {
            return Err(de::Error::custom(
                "standalone progress storage key must not be empty",
            ));
        }
        if storage.save_version == 0 {
            return Err(de::Error::custom(
                "standalone progress storage saveVersion must be greater than zero",
            ));
        }
        Ok(Self {
            key: storage.key,
            save_version: storage.save_version,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandaloneRuntimeExport<Document> {
    pub version: u16,
    pub runtime_loaded_document: Document,
    pub visual_images: EncodedVisualImageBundle,
    pub progress_storage: StandaloneProgressStorage,
}

impl<'de, Document> Deserialize<'de> for StandaloneRuntimeExport<Document>
where
    Document: DeserializeOwned + Serialize,
{
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        struct ExportVisitor<Document>(std::marker::PhantomData<Document>);

        impl<'de, Document> Visitor<'de> for ExportVisitor<Document>
        where
            Document: DeserializeOwned + Serialize,
        {
            type Value = StandaloneRuntimeExport<Document>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a versioned standalone player export")
            }

            fn visit_map<Access>(self, mut map: Access) -> Result<Self::Value, Access::Error>
            where
                Access: MapAccess<'de>,
            {
                let mut version = None;
                let mut runtime_loaded_document = None;
                let mut visual_images = None;
                let mut progress_storage = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "version" => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            let value = map.next_value::<u16>()?;
                            if value != STANDALONE_PLAYER_EXPORT_VERSION {
                                return Err(de::Error::custom(format!(
                                    "unsupported standalone player export version: {value}"
                                )));
                            }
                            version = Some(value);
                        }
                        "runtimeLoadedDocument" => {
                            if runtime_loaded_document.is_some() {
                                return Err(de::Error::duplicate_field("runtimeLoadedDocument"));
                            }
                            let serialized = map.next_value::<serde_json::Value>()?;
                            let document = serde_json::from_value::<Document>(serialized.clone())
                                .map_err(|error| {
                                de::Error::custom(format!("invalid runtimeLoadedDocument: {error}"))
                            })?;
                            let canonical = serde_json::to_value(&document).map_err(|error| {
                                de::Error::custom(format!(
                                    "runtimeLoadedDocument could not be validated: {error}"
                                ))
                            })?;
                            if canonical != serialized {
                                return Err(de::Error::custom(
                                    "runtimeLoadedDocument does not match its complete canonical schema",
                                ));
                            }
                            runtime_loaded_document = Some(document);
                        }
                        "visualImages" => {
                            if visual_images.is_some() {
                                return Err(de::Error::duplicate_field("visualImages"));
                            }
                            visual_images = Some(map.next_value()?);
                        }
                        "progressStorage" => {
                            if progress_storage.is_some() {
                                return Err(de::Error::duplicate_field("progressStorage"));
                            }
                            progress_storage = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                &field,
                                &[
                                    "version",
                                    "runtimeLoadedDocument",
                                    "visualImages",
                                    "progressStorage",
                                ],
                            ));
                        }
                    }
                }
                Ok(StandaloneRuntimeExport {
                    version: version.ok_or_else(|| de::Error::missing_field("version"))?,
                    runtime_loaded_document: runtime_loaded_document
                        .ok_or_else(|| de::Error::missing_field("runtimeLoadedDocument"))?,
                    visual_images: visual_images
                        .ok_or_else(|| de::Error::missing_field("visualImages"))?,
                    progress_storage: progress_storage
                        .ok_or_else(|| de::Error::missing_field("progressStorage"))?,
                })
            }
        }

        deserializer.deserialize_map(ExportVisitor(std::marker::PhantomData))
    }
}

impl<Document> StandaloneRuntimeExport<Document> {
    pub fn new(
        document: Document,
        visual_images: EncodedVisualImageBundle,
        progress_storage: StandaloneProgressStorage,
    ) -> Self {
        Self {
            version: STANDALONE_PLAYER_EXPORT_VERSION,
            runtime_loaded_document: document,
            visual_images,
            progress_storage,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionAction {
    Initialize,
    Snapshot,
    Key { trigger: RuntimeKeyTrigger },
    Input { name: String },
    ChoiceMove { direction: RuntimeChoiceDirection },
    Resume,
    Undo,
    Redo,
    Restart,
    NextLevel,
    PreviousLevel,
    GotoLevel { level: usize },
    SceneAction { token: RuntimeSceneActionToken },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSceneActionToken {
    pub component: String,
    pub action: RuntimeSceneActionId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeSceneActionId {
    Node { ordinal: u32 },
    Key { ordinal: u32 },
    Event { name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeChoiceDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeKeyTrigger {
    Character { value: char },
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Space,
    Escape,
    Tab,
    Backspace,
    AnyInput,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeProgressSaveRequest {
    pub request_id: u32,
    pub operation: RuntimeProgressPersistenceOperation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RuntimeProgressPersistenceOperation {
    Write { save_json: String },
    Delete,
}

#[test]
fn progress_persistence_requests_serialize_an_explicit_storage_operation() {
    let write = RuntimeProgressSaveRequest {
        request_id: 3,
        operation: RuntimeProgressPersistenceOperation::Write {
            save_json: r#"{"currentLevel":2}"#.to_string(),
        },
    };
    let delete = RuntimeProgressSaveRequest {
        request_id: 4,
        operation: RuntimeProgressPersistenceOperation::Delete,
    };

    assert_eq!(
        serde_json::to_value(&write).unwrap(),
        serde_json::json!({
            "requestId": 3,
            "operation": {
                "kind": "write",
                "saveJson": r#"{"currentLevel":2}"#,
            },
        })
    );
    assert_eq!(
        serde_json::to_value(&delete).unwrap(),
        serde_json::json!({
            "requestId": 4,
            "operation": {
                "kind": "delete",
            },
        })
    );
    assert_eq!(
        serde_json::from_value::<RuntimeProgressSaveRequest>(
            serde_json::to_value(write.clone()).unwrap()
        )
        .unwrap(),
        write
    );
    assert_eq!(
        serde_json::from_value::<RuntimeProgressSaveRequest>(
            serde_json::to_value(delete.clone()).unwrap()
        )
        .unwrap(),
        delete
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeModelKind {
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCoord {
    pub x: u16,
    pub y: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeStateSnapshot {
    TwoD(RuntimeStateSnapshot2d),
    ThreeD(RuntimeStateSnapshot3d),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeStateSnapshot2d {
    pub kind: RuntimeModelKind,
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    pub slots: Vec<u16>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeStateSnapshot3d {
    pub kind: RuntimeModelKind,
    pub width: u16,
    pub depth: u16,
    pub height: u16,
    pub layer_count: u16,
    pub slots: Vec<u16>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePuzzle3CameraProjection {
    Perspective,
    Orthographic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePuzzle3ViewportMode {
    Full,
    Centered,
    Paged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePuzzle3ViewportFollow {
    Snap,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimePuzzle3ViewportHeight {
    Full,
    Size(u16),
}

impl Serialize for RuntimePuzzle3ViewportHeight {
    fn serialize<SerializerType>(
        &self,
        serializer: SerializerType,
    ) -> Result<SerializerType::Ok, SerializerType::Error>
    where
        SerializerType: serde::Serializer,
    {
        match self {
            Self::Full => serializer.serialize_str("full"),
            Self::Size(height) => serializer.serialize_u16(*height),
        }
    }
}

impl<'de> Deserialize<'de> for RuntimePuzzle3ViewportHeight {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SerializedHeight {
            Name(String),
            Size(u16),
        }

        match SerializedHeight::deserialize(deserializer)? {
            SerializedHeight::Name(name) if name == "full" => Ok(Self::Full),
            SerializedHeight::Name(name) => Err(de::Error::custom(format!(
                "unsupported Puzzle3 viewport height: {name}"
            ))),
            SerializedHeight::Size(height) if height > 0 => Ok(Self::Size(height)),
            SerializedHeight::Size(_) => Err(de::Error::custom(
                "Puzzle3 viewport height must be positive",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePuzzle3VisualSpace {
    World,
    Local,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RuntimePuzzle3SpatialOp {
    #[serde(rename = "rotate3")]
    Rotate {
        space: RuntimePuzzle3VisualSpace,
        axis: [f64; 3],
        degrees: f64,
    },
    #[serde(rename = "translate3")]
    Translate {
        space: RuntimePuzzle3VisualSpace,
        value: [f64; 3],
    },
    #[serde(rename = "flip3")]
    Flip { enabled: bool },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Size {
    pub width: u16,
    pub depth: u16,
    pub height: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Input {
    pub id: u16,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Object {
    pub id: u16,
    pub name: String,
    pub visual: Option<String>,
    pub layer: u16,
    pub render_priority: u16,
    pub composition: RuntimeVisualComposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeVisualComposition {
    Ordered,
    Average,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeLinearRgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeUiTextStyle {
    pub font_size_px: f32,
    pub line_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeUiTypography {
    pub heading: RuntimeUiTextStyle,
    pub subheading: RuntimeUiTextStyle,
    pub body: RuntimeUiTextStyle,
    pub caption: RuntimeUiTextStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeUiControlStyle {
    pub padding_horizontal_px: f32,
    pub padding_vertical_px: f32,
    pub margin_px: f32,
    pub border_width_px: f32,
    pub corner_radius_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeTheme {
    pub background: RuntimeLinearRgba,
    pub text: RuntimeLinearRgba,
    pub muted_text: RuntimeLinearRgba,
    pub accent: RuntimeLinearRgba,
    pub panel: RuntimeLinearRgba,
    pub control: RuntimeLinearRgba,
    pub control_focused: RuntimeLinearRgba,
    pub control_selected: RuntimeLinearRgba,
    pub control_selected_border: RuntimeLinearRgba,
    pub typography: RuntimeUiTypography,
    pub control_layout: RuntimeUiControlStyle,
}

impl RuntimeTheme {
    pub fn validate(&self) -> Result<(), &'static str> {
        for (label, color) in [
            ("theme.background", self.background),
            ("theme.text", self.text),
            ("theme.muted_text", self.muted_text),
            ("theme.accent", self.accent),
            ("theme.panel", self.panel),
            ("theme.control", self.control),
            ("theme.control_focused", self.control_focused),
            ("theme.control_selected", self.control_selected),
            (
                "theme.control_selected_border",
                self.control_selected_border,
            ),
        ] {
            if ![color.red, color.green, color.blue, color.alpha]
                .into_iter()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
            {
                return Err(label);
            }
        }
        for (label, style) in [
            ("theme.typography.heading", self.typography.heading),
            ("theme.typography.subheading", self.typography.subheading),
            ("theme.typography.body", self.typography.body),
            ("theme.typography.caption", self.typography.caption),
        ] {
            if !style.font_size_px.is_finite()
                || style.font_size_px <= 0.0
                || !style.line_height.is_finite()
                || style.line_height <= 0.0
            {
                return Err(label);
            }
        }
        for (label, value) in [
            (
                "theme.control_layout.padding_horizontal_px",
                self.control_layout.padding_horizontal_px,
            ),
            (
                "theme.control_layout.padding_vertical_px",
                self.control_layout.padding_vertical_px,
            ),
            (
                "theme.control_layout.margin_px",
                self.control_layout.margin_px,
            ),
            (
                "theme.control_layout.border_width_px",
                self.control_layout.border_width_px,
            ),
            (
                "theme.control_layout.corner_radius_px",
                self.control_layout.corner_radius_px,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(label);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedPixel {
    pub position: [i32; 2],
    pub color: RuntimeLinearRgba,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedVoxel {
    pub position: [i32; 3],
    pub color: RuntimeLinearRgba,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeResolvedVisualFrame {
    Pixels {
        width: u16,
        height: u16,
        pixels: Vec<RuntimeResolvedPixel>,
    },
    Voxels {
        width: u16,
        depth: u16,
        height: u16,
        voxels: Vec<RuntimeResolvedVoxel>,
    },
    RasterImage {
        asset: VisualImageAssetId,
        sampling: RuntimeResolvedSampling,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedVisualClip {
    pub id: String,
    pub frames: Vec<RuntimeResolvedVisualFrame>,
    pub frame_duration_ms: Option<u64>,
    pub layout: RuntimeResolvedVisualLayout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolvedFitMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolvedSampling {
    Pixelated,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedVisualLayout {
    pub fit: RuntimeResolvedFitMode,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderInstance {
    pub id: u64,
    pub object_id: Option<u16>,
    pub visual: String,
    pub cell: [i32; 3],
    pub transform: [[f64; 4]; 4],
    pub opacity: f64,
    pub frame_elapsed_ms: Option<u64>,
    pub playback: RuntimeResolvedPlayback,
    pub render_order: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolvedPlayback {
    Loop,
    Once,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedCompositionGroup {
    pub render_order: u64,
    pub composition: RuntimeVisualComposition,
    pub instances: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderScene {
    pub clips: Vec<RuntimeResolvedVisualClip>,
    pub instances: Vec<RuntimeResolvedRenderInstance>,
    pub composition_groups: Vec<RuntimeResolvedCompositionGroup>,
    pub cells: Vec<RuntimeResolvedRenderCell>,
    pub decorations: Vec<RuntimeResolvedDecoration>,
    pub render_priority_count: u16,
    pub animation_duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderCell {
    pub position: [i32; 3],
    pub render_order: u64,
    pub object_ids: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeGridMode {
    Hidden,
    OccupiedCells,
    AllCells,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedView2d {
    pub origin: [i32; 2],
    pub size: [u16; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedLineSegment2d {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedLineSegment3d {
    pub start: [f64; 3],
    pub end: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeResolvedStrokeWidth {
    CellRelative {
        cell_fraction: f64,
        min_physical_pixels: f64,
    },
    PhysicalPixels {
        pixels: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedLineStyle {
    pub color: RuntimeLinearRgba,
    pub width: RuntimeResolvedStrokeWidth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolvedLineLayer2d {
    Overlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResolvedLineDepth3d {
    Tested,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeResolvedDecoration {
    Lines2d {
        segments: Vec<RuntimeResolvedLineSegment2d>,
        style: RuntimeResolvedLineStyle,
        layer: RuntimeResolvedLineLayer2d,
    },
    Lines3d {
        segments: Vec<RuntimeResolvedLineSegment3d>,
        style: RuntimeResolvedLineStyle,
        depth: RuntimeResolvedLineDepth3d,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderMoment {
    pub clip_elapsed_ms: u64,
    pub animation_elapsed_ms: u64,
    pub animations: Vec<RuntimeAnimationEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedPixelGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub clip: Option<RuntimeResolvedRect2d>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRect2d {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeResolvedRenderBatchContent {
    Pixels {
        width: u16,
        height: u16,
        pixels: Vec<RuntimeResolvedPixel>,
    },
    Voxels {
        width: u16,
        depth: u16,
        height: u16,
        voxels: Vec<RuntimeResolvedVoxel>,
    },
    RasterImage {
        asset: VisualImageAssetId,
        revision: VisualImageAssetRevision,
        #[serde(rename = "sourceSize")]
        source_size: [u16; 2],
        destination: RuntimeResolvedRect2d,
        uv: RuntimeResolvedRect2d,
        sampling: RuntimeResolvedSampling,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderBatch {
    pub render_order: u64,
    pub object_ids: Vec<u16>,
    pub cell: [i32; 3],
    pub transform: [[f64; 4]; 4],
    pub opacity: f64,
    pub pixel_geometry: Option<RuntimeResolvedPixelGeometry>,
    pub content: RuntimeResolvedRenderBatchContent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedRenderFrame {
    pub batches: Vec<RuntimeResolvedRenderBatch>,
    pub decorations: Vec<RuntimeResolvedDecoration>,
    pub continue_animation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResolvedVisualOrder {
    pub render_priority: u16,
    pub composition: RuntimeVisualComposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3ObjectRef {
    pub id: u16,
    pub layer: u16,
    pub render_order: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Cell {
    pub position: RuntimeCoord,
    pub objects: Vec<RuntimePuzzle3ObjectRef>,
    pub render_order: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3VisualFrame {
    pub layers: Vec<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Visual {
    pub palette: std::collections::BTreeMap<String, String>,
    pub frames: Vec<RuntimePuzzle3VisualFrame>,
    pub duration_ms: Option<u64>,
    pub frame_duration_ms: Option<u64>,
    pub spatial_affine: [[f64; 4]; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Camera {
    pub projection: RuntimePuzzle3CameraProjection,
    pub yaw_degrees: i16,
    pub pitch_degrees: i16,
    pub roll_degrees: i16,
    pub zoom: f64,
    pub interactive_look: bool,
    pub interactive_zoom: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3VisualRender {
    pub shade: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3PixelateRender {
    pub enabled: bool,
    pub scale: u16,
    pub smoothing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3TweenRender {
    pub enabled: bool,
    pub interval_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3AnimationRender {
    pub tween: RuntimePuzzle3TweenRender,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3ViewportFraming {
    pub width: u16,
    pub depth: u16,
    pub height: RuntimePuzzle3ViewportHeight,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Viewport {
    pub mode: RuntimePuzzle3ViewportMode,
    pub follow: RuntimePuzzle3ViewportFollow,
    pub focus: String,
    pub focus_objects: Vec<u16>,
    pub framing_box: RuntimePuzzle3ViewportFraming,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Render {
    pub camera: RuntimePuzzle3Camera,
    pub lighting: RuntimePuzzle3Lighting,
    pub visual: RuntimePuzzle3VisualRender,
    pub shadow: bool,
    pub pixelate: RuntimePuzzle3PixelateRender,
    pub animation: RuntimePuzzle3AnimationRender,
    pub viewport: Option<RuntimePuzzle3Viewport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Lighting {
    pub intensity: f64,
    pub ambient: f64,
    pub yaw_degrees: i16,
    pub pitch_degrees: i16,
    pub color: RuntimeLinearRgba,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3VisualOrderPriority {
    pub objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<String>,
    pub merge: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePuzzle3VisualOrder {
    pub direction_priority: Vec<String>,
    pub priorities: Vec<RuntimePuzzle3VisualOrderPriority>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Resources {
    pub layer_count: u16,
    pub inputs: Vec<RuntimePuzzle3Input>,
    pub objects: std::collections::BTreeMap<String, RuntimePuzzle3Object>,
    pub visuals: std::collections::BTreeMap<String, RuntimePuzzle3Visual>,
    pub render: RuntimePuzzle3Render,
    pub order: RuntimePuzzle3VisualOrder,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle3Snapshot {
    pub level_index: usize,
    pub level_count: usize,
    pub level_name: Option<String>,
    pub size: RuntimePuzzle3Size,
    pub cells: Vec<RuntimePuzzle3Cell>,
    pub completed: bool,
    pub has_next_level: bool,
    pub has_previous_level: bool,
    pub render: RuntimePuzzle3Render,
    pub render_scene: RuntimeResolvedRenderScene,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum SolverStateSnapshot {
    #[serde(rename = "2d", rename_all = "camelCase")]
    TwoD {
        width: u16,
        height: u16,
        layer_count: u16,
        slots: Vec<u16>,
        variables: Vec<i64>,
        level_fired_rules: Vec<u16>,
    },
    #[serde(rename = "puzzle3d", rename_all = "camelCase")]
    ThreeD {
        width: u16,
        depth: u16,
        height: u16,
        layer_count: u16,
        slots: Vec<u16>,
        variables: Vec<i64>,
        level_fired_rules: Vec<u16>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverPreparedArtifact {
    pub artifact_id: String,
    pub model_kind: RuntimeModelKind,
    pub level_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchRequest {
    pub level_index: usize,
    pub state: SolverStateSnapshot,
    pub materialize_level_start: bool,
    pub max_depth: u32,
    pub max_stored_nodes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverSearchStatus {
    Paused,
    Solved,
    Exhausted,
    BudgetExceeded,
    ResourceLimit,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchStats {
    pub visited: usize,
    pub expanded: usize,
    pub frontier: usize,
    pub max_depth_reached: u32,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchProgress {
    pub visited: usize,
    pub expanded: usize,
    pub frontier: usize,
    pub max_depth_reached: u32,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverObservation {
    pub progress: SolverSearchProgress,
    pub state: SolverStateSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverMove {
    pub id: u16,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverStep {
    pub index: usize,
    #[serde(rename = "move")]
    pub input: Option<SolverMove>,
    pub state: SolverStateSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverResult {
    pub model: RuntimeModelKind,
    pub result: SolverSearchStatus,
    pub depth: Option<u32>,
    pub moves: Vec<SolverMove>,
    pub steps: Vec<SolverStep>,
    pub observations: Vec<SolverObservation>,
    pub stats: SolverSearchStats,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverAdvanceResponse {
    pub status: SolverSearchStatus,
    pub stats: SolverSearchStats,
    pub observation: Option<SolverObservation>,
    pub result: Option<SolverResult>,
}

impl RuntimeStateSnapshot2d {
    pub fn from_state(state: &GridState<2, Size2>) -> Self {
        Self {
            kind: RuntimeModelKind::TwoD,
            width: state.size.width,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state(self, game: &GridCompiledGame<2>) -> Result<GridState<2, Size2>, String> {
        if self.kind != RuntimeModelKind::TwoD {
            return Err("runtime 2d state kind must be 2d".to_string());
        }
        decode_runtime_state(
            game,
            Size2::new(self.width, self.height),
            self.layer_count,
            self.slots,
            self.variables,
            self.level_fired_rules,
        )
    }
}

impl SolverStateSnapshot {
    pub fn from_state2(state: &GridState<2, Size2>) -> Self {
        Self::TwoD {
            width: state.size.width,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn from_state3(state: &GridState<3, Size3>) -> Self {
        Self::ThreeD {
            width: state.size.width,
            depth: state.size.depth,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state2(self, game: &GridCompiledGame<2>) -> Result<GridState<2, Size2>, String> {
        let Self::TwoD {
            width,
            height,
            layer_count,
            slots,
            variables,
            level_fired_rules,
        } = self
        else {
            return Err("solver state kind must be 2d".to_string());
        };
        decode_runtime_state(
            game,
            Size2::new(width, height),
            layer_count,
            slots,
            variables,
            level_fired_rules,
        )
    }

    pub fn into_state3(self, game: &GridCompiledGame<3>) -> Result<GridState<3, Size3>, String> {
        let Self::ThreeD {
            width,
            depth,
            height,
            layer_count,
            slots,
            variables,
            level_fired_rules,
        } = self
        else {
            return Err("solver state kind must be puzzle3d".to_string());
        };
        decode_runtime_state(
            game,
            Size3::new(width, depth, height),
            layer_count,
            slots,
            variables,
            level_fired_rules,
        )
    }
}

impl RuntimeStateSnapshot3d {
    pub fn from_state(state: &GridState<3, Size3>) -> Self {
        Self {
            kind: RuntimeModelKind::ThreeD,
            width: state.size.width,
            depth: state.size.depth,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state(self, game: &GridCompiledGame<3>) -> Result<GridState<3, Size3>, String> {
        if self.kind != RuntimeModelKind::ThreeD {
            return Err("runtime 3d state kind must be 3d".to_string());
        }
        decode_runtime_state(
            game,
            Size3::new(self.width, self.depth, self.height),
            self.layer_count,
            self.slots,
            self.variables,
            self.level_fired_rules,
        )
    }
}

fn decode_runtime_state<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    size: Size,
    layer_count: u16,
    slots: Vec<u16>,
    variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
) -> Result<GridState<D, Size>, String> {
    if layer_count != game.layer_count {
        return Err(format!(
            "runtime state layerCount mismatch: expected {}, got {layer_count}",
            game.layer_count
        ));
    }
    let axes = size.axes();
    let cell_count = axes
        .iter()
        .try_fold(1usize, |count, axis| count.checked_mul(usize::from(*axis)));
    let cell_count =
        cell_count.ok_or_else(|| "runtime state dimensions are too large".to_string())?;
    let slot_count = cell_count
        .checked_mul(usize::from(layer_count))
        .ok_or_else(|| "runtime state dimensions are too large".to_string())?;
    if slots.len() != slot_count {
        return Err(format!(
            "runtime state slots length mismatch: expected {slot_count}, got {}",
            slots.len()
        ));
    }
    let mut state = GridState::<D, Size>::empty_sized_with_variables(
        size,
        layer_count,
        game.object_count(),
        variables,
    )
    .map_err(|error| format!("{error:?}"))?;
    for (slot, object) in slots.into_iter().enumerate() {
        let layer = slot % usize::from(layer_count);
        let cell = slot / usize::from(layer_count);
        let position = runtime_cell_position::<D>(&axes, cell)?;
        if object != 0 {
            let object = ObjectId(object);
            let expected_layer = game
                .object_layer(object)
                .ok_or_else(|| format!("runtime state contains unknown object {}", object.0))?;
            if usize::from(expected_layer.0) != layer {
                return Err(format!(
                    "runtime state object {} is in layer {layer}, expected {}",
                    object.0, expected_layer.0
                ));
            }
            state
                .place_object_at(game, position, object)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    for rule in level_fired_rules {
        state.mark_level_rule_fired(RuleId(rule));
    }
    Ok(state)
}

fn runtime_cell_position<const D: usize>(
    axes: &[u16; D],
    mut cell: usize,
) -> Result<GridCoord<D>, String> {
    let mut position = [0u16; D];
    for (axis, length) in axes.iter().enumerate() {
        if *length == 0 {
            return Err("runtime state dimensions must be positive".to_string());
        }
        position[axis] = u16::try_from(cell % usize::from(*length))
            .map_err(|_| "runtime state coordinate is out of range".to_string())?;
        cell /= usize::from(*length);
    }
    Ok(GridCoord::new(position))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeChangedCell {
    pub position: RuntimeCoord,
    pub objects: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMarkValueMatch {
    Any,
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimePatchOp {
    Add {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Remove {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Move {
        from: RuntimeCoord,
        to: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Replace {
        position: RuntimeCoord,
        remove: u16,
        add: u16,
    },
    UpdateVariable {
        variable: u16,
    },
    SetMark {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
        mark: u16,
    },
    RemoveMark {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
        mark: u16,
        #[serde(rename = "matchValue")]
        match_value: RuntimeMarkValueMatch,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeVisualSpace {
    World,
    Local,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeVisualTransform {
    Rotate {
        degrees: f64,
        axis: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Translate {
        value: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Scale {
        value: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Flip {
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVisualState {
    pub transforms: Vec<RuntimeVisualTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVisualTween {
    pub transforms: Vec<RuntimeVisualTweenTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<RuntimeScalarTween>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeVisualTweenTransform {
    Rotate {
        #[serde(rename = "startDegrees")]
        start_degrees: f64,
        #[serde(rename = "deltaDegrees")]
        delta_degrees: f64,
        axis: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Translate {
        start: [f64; 3],
        delta: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Scale {
        start: [f64; 3],
        delta: [f64; 3],
        space: RuntimeVisualSpace,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeScalarTween {
    pub start: f64,
    pub delta: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeAnimationEvent {
    Animation {
        name: String,
        position: RuntimeCoord,
        #[serde(rename = "resolvedVisual", skip_serializing_if = "Option::is_none")]
        resolved_visual: Option<RuntimeResolvedVisualOrder>,
    },
    Move {
        name: String,
        #[serde(rename = "occurrenceId")]
        occurrence_id: u64,
        #[serde(rename = "objectId")]
        object_id: u16,
        #[serde(rename = "visualTween", skip_serializing_if = "Option::is_none")]
        visual_tween: Option<RuntimeVisualTween>,
        from: RuntimeCoord,
        to: RuntimeCoord,
    },
    CantMove {
        name: String,
        #[serde(rename = "objectId")]
        object_id: u16,
        position: RuntimeCoord,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RuntimePresentationEvent {
    Audio {
        command: AudioCommand,
    },
    Wait {
        milliseconds: u64,
    },
    AnimationBatch {
        source: RuntimeViewportSourceId,
        level_index: Option<usize>,
        animations: Vec<RuntimeAnimationEvent>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeTransitionCommand {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAnimationOffset {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// A source-free effect that may cross a runtime/export boundary.
pub enum RuntimeEffect {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    PlaySfx {
        name: String,
    },
    PlayMusic {
        name: String,
    },
    PauseMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    ResumeMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    StopMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Wait {
        milliseconds: u64,
    },
    WaitAnimation,
    EmitAnimation {
        name: String,
        component: u16,
        offset: RuntimeAnimationOffset,
    },
    PresentComponent {
        definition: String,
        properties: Vec<RuntimeComponentProperty>,
        placement: ComponentPlacement,
        #[serde(rename = "awaitEvent")]
        await_event: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeComponentProperty {
    pub name: String,
    pub value: String,
    pub literal: bool,
}

pub fn standard_message_effect(value: String, literal: bool) -> RuntimeEffect {
    RuntimeEffect::PresentComponent {
        definition: STANDARD_MESSAGE_COMPONENT.to_string(),
        properties: vec![RuntimeComponentProperty {
            name: STANDARD_MESSAGE_TEXT_PROPERTY.to_string(),
            value,
            literal,
        }],
        placement: ComponentPlacement::Overlay,
        await_event: Some(STANDARD_MESSAGE_DISMISS_EVENT.to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleFiring {
    pub rule_id: u16,
    pub patch: Vec<RuntimePatchOp>,
    pub progressed: bool,
    pub observable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTransitionProgramOutcome {
    pub state: RuntimeStateSnapshot,
    pub cancelled: bool,
    pub completed: bool,
    pub commands: Vec<RuntimeTransitionCommand>,
    pub effects: Vec<RuntimeEffect>,
    pub firings: Vec<RuntimeRuleFiring>,
    pub animation_events: Vec<RuntimeAnimationEvent>,
}

impl RuntimeTransitionProgramOutcome {
    pub fn to_json_string(&self) -> Result<String, RuntimeJsonError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeJsonError::InvalidJson(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTransitionCurrentOutcome {
    pub cancelled: bool,
    pub changed: bool,
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<RuntimeStateSnapshot>,
    pub commands: Vec<RuntimeTransitionCommand>,
    pub effects: Vec<RuntimeEffect>,
    pub firings: Vec<RuntimeRuleFiring>,
    pub animation_events: Vec<RuntimeAnimationEvent>,
    pub state_hash: u64,
    pub state_hash_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_state_handle: Option<u32>,
    pub changed_cells: Vec<RuntimeChangedCell>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

impl RuntimeTransitionCurrentOutcome {
    pub fn to_json_string(&self) -> Result<String, RuntimeJsonError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeJsonError::InvalidJson(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CameraEffect {
    SetYaw(i16),
    SetPitch(i16),
    SetRoll(i16),
    SetZoom(u16),
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeJsonError {
    InvalidJson(String),
}

impl std::fmt::Display for RuntimeJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => {
                write!(f, "invalid runtime JSON: {error}")
            }
        }
    }
}

impl std::error::Error for RuntimeJsonError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_player_export_versions_document_assets_and_storage_together() {
        let export = StandaloneRuntimeExport::new(
            "typed-document".to_string(),
            EncodedVisualImageBundle::default(),
            StandaloneProgressStorage {
                key: "game:revision".to_string(),
                save_version: 2,
            },
        );
        let value = serde_json::to_value(&export).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "version": STANDALONE_PLAYER_EXPORT_VERSION,
                "runtimeLoadedDocument": "typed-document",
                "visualImages": {"assets": []},
                "progressStorage": {
                    "key": "game:revision",
                    "saveVersion": 2,
                },
            })
        );
        assert_eq!(
            serde_json::from_value::<StandaloneRuntimeExport<String>>(value).unwrap(),
            export
        );
    }

    #[test]
    fn standalone_player_export_rejects_an_unsupported_whole_bundle_version() {
        let error = serde_json::from_value::<StandaloneRuntimeExport<String>>(serde_json::json!({
            "version": STANDALONE_PLAYER_EXPORT_VERSION + 1,
            "runtimeLoadedDocument": "typed-document",
            "visualImages": {"assets": []},
            "progressStorage": {
                "key": "game:revision",
                "saveVersion": 2,
            },
        }))
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("unsupported standalone player export version"),
            "{error}"
        );
    }

    #[test]
    fn standalone_player_export_rejects_nested_default_and_unknown_field_compatibility() {
        #[derive(Debug, Serialize, Deserialize)]
        struct CompatibilityDocument {
            name: String,
            #[serde(default)]
            behavior: u32,
        }

        for runtime_loaded_document in [
            serde_json::json!({"name": "game"}),
            serde_json::json!({"name": "game", "behavior": 0, "legacy": true}),
        ] {
            let error = serde_json::from_value::<StandaloneRuntimeExport<CompatibilityDocument>>(
                serde_json::json!({
                    "version": STANDALONE_PLAYER_EXPORT_VERSION,
                    "runtimeLoadedDocument": runtime_loaded_document,
                    "visualImages": {"assets": []},
                    "progressStorage": {
                        "key": "game:revision",
                        "saveVersion": 2,
                    },
                }),
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("does not match its complete canonical schema"),
                "{error}"
            );
        }
    }

    #[test]
    fn standalone_player_export_rejects_invalid_progress_storage_identity() {
        for storage in [
            serde_json::json!({"key": "", "saveVersion": 2}),
            serde_json::json!({"key": "game:revision", "saveVersion": 0}),
        ] {
            assert!(
                serde_json::from_value::<StandaloneRuntimeExport<String>>(serde_json::json!({
                    "version": STANDALONE_PLAYER_EXPORT_VERSION,
                    "runtimeLoadedDocument": "typed-document",
                    "visualImages": {"assets": []},
                    "progressStorage": storage,
                }))
                .is_err()
            );
        }
    }

    #[test]
    fn solver_search_request_names_the_stored_node_limit() {
        let request = serde_json::json!({
            "levelIndex": 0,
            "state": {
                "kind": "2d",
                "width": 1,
                "height": 1,
                "layerCount": 1,
                "slots": [0],
                "variables": [],
                "levelFiredRules": []
            },
            "materializeLevelStart": false,
            "maxDepth": 8,
            "maxStoredNodes": 1000
        });
        let decoded: SolverSearchRequest = serde_json::from_value(request.clone()).unwrap();
        assert_eq!(decoded.max_stored_nodes, 1000);

        let mut legacy = request;
        let stored_node_limit = legacy["maxStoredNodes"].take();
        legacy["maxNodes"] = stored_node_limit;
        legacy.as_object_mut().unwrap().remove("maxStoredNodes");
        assert!(serde_json::from_value::<SolverSearchRequest>(legacy).is_err());
    }

    #[test]
    fn visual_tween_contract_exposes_prepared_renderer_channels() {
        let tween = RuntimeVisualTween {
            transforms: vec![RuntimeVisualTweenTransform::Scale {
                start: [1.0, 1.0, 1.0],
                delta: [1.0, 1.0, 0.0],
                space: RuntimeVisualSpace::Local,
            }],
            opacity: Some(RuntimeScalarTween {
                start: 0.0,
                delta: 1.0,
            }),
        };

        let value = serde_json::to_value(tween).unwrap();
        assert_eq!(value["transforms"][0]["kind"], "scale");
        assert_eq!(value["transforms"][0]["start"][0], 1.0);
        assert_eq!(value["transforms"][0]["delta"][0], 1.0);
        assert_eq!(value["opacity"]["delta"], 1.0);
    }

    #[test]
    fn explicit_visual_animation_uses_the_renderer_animation_contract() {
        let value = serde_json::to_value(RuntimeAnimationEvent::Animation {
            name: "flash".to_string(),
            position: RuntimeCoord {
                x: 2,
                y: 3,
                z: None,
            },
            resolved_visual: None,
        })
        .unwrap();

        assert_eq!(value["kind"], "animation");
        assert_eq!(value["name"], "flash");
        assert_eq!(value["position"]["x"], 2);
        assert_eq!(value["position"]["y"], 3);
    }
    use serde_json::json;

    #[test]
    fn session_action_uses_a_tagged_source_free_wire_contract() {
        let action = SessionAction::Input {
            name: "front".to_string(),
        };
        let value = serde_json::to_value(&action).unwrap();
        assert_eq!(value, json!({"kind": "input", "name": "front"}));
        assert_eq!(
            serde_json::from_value::<SessionAction>(value).unwrap(),
            action
        );
        let resume = serde_json::to_value(SessionAction::Resume).unwrap();
        assert_eq!(resume, json!({"kind": "resume"}));
        assert_eq!(
            serde_json::from_value::<SessionAction>(resume).unwrap(),
            SessionAction::Resume
        );
        assert!(
            serde_json::from_value::<SessionAction>(
                json!({"kind": "debug_input", "name": "right"})
            )
            .is_err(),
            "editor debug input must not be part of the player action contract"
        );
        let key = SessionAction::Key {
            trigger: RuntimeKeyTrigger::ArrowLeft,
        };
        let key_value = serde_json::to_value(&key).unwrap();
        assert_eq!(
            key_value,
            json!({"kind": "key", "trigger": {"kind": "arrow_left"}})
        );
        assert_eq!(
            serde_json::from_value::<SessionAction>(key_value).unwrap(),
            key
        );
        let choice_move = SessionAction::ChoiceMove {
            direction: RuntimeChoiceDirection::Left,
        };
        let choice_move_value = serde_json::to_value(&choice_move).unwrap();
        assert_eq!(
            choice_move_value,
            json!({"kind": "choice_move", "direction": "left"})
        );
        assert_eq!(
            serde_json::from_value::<SessionAction>(choice_move_value).unwrap(),
            choice_move
        );
        let scene_action = SessionAction::SceneAction {
            token: RuntimeSceneActionToken {
                component: "title".to_string(),
                action: RuntimeSceneActionId::Node { ordinal: 2 },
            },
        };
        let scene_action_value = serde_json::to_value(&scene_action).unwrap();
        assert_eq!(
            scene_action_value,
            json!({
                "kind": "scene_action",
                "token": {
                    "component": "title",
                    "action": { "kind": "node", "ordinal": 2 }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SessionAction>(scene_action_value).unwrap(),
            scene_action
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "input",
                "name": "front",
                "url": "/api/input/front"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "choice_activate",
                "index": 0
            }))
            .is_err(),
            "index-based choice activation must not re-enter the player contract"
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "scene_effect",
                "effect": { "kind": "goto", "scene": "playing", "params": [] }
            }))
            .is_err(),
            "raw authored scene effects must not re-enter the player contract"
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "component_event",
                "instance": "dialog",
                "event": "complete"
            }))
            .is_err(),
            "component events must be invoked through a resolved scene action token"
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "command",
                "name": "goto playing"
            }))
            .is_err(),
            "authored command strings must not enter the player contract"
        );
    }

    #[test]
    fn solver_state_snapshot_contains_only_committed_state() {
        let snapshot = SolverStateSnapshot::TwoD {
            width: 1,
            height: 1,
            layer_count: 1,
            slots: vec![7],
            variables: vec![5],
            level_fired_rules: vec![13],
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["kind"], "2d");
        assert!(value.get("slotMarks").is_none());
        assert!(value.get("cellMarks").is_none());
        assert_eq!(
            serde_json::from_value::<SolverStateSnapshot>(value).unwrap(),
            snapshot
        );
    }

    #[test]
    fn transition_program_outcome_serializes_owned_outer_schema() {
        let outcome = RuntimeTransitionProgramOutcome {
            state: RuntimeStateSnapshot::TwoD(RuntimeStateSnapshot2d {
                kind: RuntimeModelKind::TwoD,
                width: 2,
                height: 1,
                layer_count: 1,
                slots: vec![0, 1],
                variables: Vec::new(),
                level_fired_rules: Vec::new(),
            }),
            cancelled: false,
            completed: true,
            commands: vec![RuntimeTransitionCommand::Win],
            effects: vec![RuntimeEffect::Win],
            firings: vec![RuntimeRuleFiring {
                rule_id: 3,
                patch: vec![RuntimePatchOp::Remove {
                    position: RuntimeCoord {
                        x: 0,
                        y: 0,
                        z: None,
                    },
                    object_id: 1,
                }],
                progressed: true,
                observable: false,
            }],
            animation_events: vec![RuntimeAnimationEvent::Move {
                name: "tween".to_string(),
                occurrence_id: 1,
                object_id: 1,
                visual_tween: None,
                from: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                to: RuntimeCoord {
                    x: 1,
                    y: 0,
                    z: None,
                },
            }],
        };

        let value: serde_json::Value =
            serde_json::from_str(&outcome.to_json_string().expect("outcome serializes"))
                .expect("outcome JSON parses");

        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            [
                "animationEvents",
                "cancelled",
                "commands",
                "completed",
                "effects",
                "firings",
                "state"
            ]
        );
    }

    #[test]
    fn runtime_effect_contract_rejects_authored_scene_effects() {
        assert!(
            serde_json::from_value::<RuntimeEffect>(json!({
                "kind": "scene",
                "effect": {
                    "kind": "goto",
                    "scene": "playing",
                    "params": []
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn transition_current_outcome_serializes_owned_outer_schema() {
        let outcome = RuntimeTransitionCurrentOutcome {
            cancelled: false,
            changed: true,
            completed: false,
            state: None,
            commands: Vec::new(),
            effects: Vec::new(),
            firings: Vec::new(),
            animation_events: Vec::new(),
            state_hash: 12,
            state_hash_key: "12".to_string(),
            previous_state_handle: Some(4),
            changed_cells: vec![RuntimeChangedCell {
                position: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                objects: vec![1],
            }],
            variables: vec![9],
            level_fired_rules: vec![2],
        };

        let value: serde_json::Value =
            serde_json::from_str(&outcome.to_json_string().expect("outcome serializes"))
                .expect("outcome JSON parses");

        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            [
                "animationEvents",
                "cancelled",
                "changed",
                "changedCells",
                "commands",
                "completed",
                "effects",
                "firings",
                "levelFiredRules",
                "previousStateHandle",
                "stateHash",
                "stateHashKey",
                "variables"
            ]
        );
        assert!(value.get("state").is_none());
        assert!(value["changedCells"][0]["position"].get("z").is_none());
    }
}
#[test]
fn wait_presentation_event_has_no_viewport_identity() {
    let event = RuntimePresentationEvent::Wait { milliseconds: 80 };
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["kind"], "wait");
    assert!(value.get("levelIndex").is_none());
    assert!(value.get("source").is_none());
    assert!(value.get("scene").is_none());
    assert!(value.get("puzzle").is_none());
    assert_eq!(
        serde_json::from_value::<RuntimePresentationEvent>(value).unwrap(),
        event
    );
}

#[test]
fn audio_presentation_event_carries_only_a_typed_asset_command() {
    let event = RuntimePresentationEvent::Audio {
        command: AudioCommand::PlaySfx {
            asset: puzzle_audio_contract::SfxAssetId(7),
        },
    };
    let value = serde_json::to_value(&event).unwrap();

    assert_eq!(
        value,
        serde_json::json!({
            "kind": "audio",
            "command": {
                "kind": "play_sfx",
                "asset": 7,
            },
        })
    );
    assert_eq!(
        serde_json::from_value::<RuntimePresentationEvent>(value).unwrap(),
        event
    );
}

#[test]
fn presentation_event_rejects_legacy_scene_and_puzzle_identity() {
    assert!(
        serde_json::from_value::<RuntimePresentationEvent>(serde_json::json!({
        "scene": "playing",
        "puzzle": "board",
        "levelIndex": 0,
        "kind": "wait",
        "milliseconds": 80
        }))
        .is_err()
    );
}

#[test]
fn presentation_animation_batch_round_trips_as_one_runtime_occurrence() {
    let event = RuntimePresentationEvent::AnimationBatch {
        source: RuntimeViewportSourceId {
            component: "playing".to_string(),
            source: "board".to_string(),
        },
        level_index: Some(0),
        animations: vec![
            RuntimeAnimationEvent::Move {
                name: "tween".to_string(),
                occurrence_id: 1,
                object_id: 1,
                visual_tween: None,
                from: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                to: RuntimeCoord {
                    x: 1,
                    y: 0,
                    z: None,
                },
            },
            RuntimeAnimationEvent::Animation {
                name: "flash".to_string(),
                position: RuntimeCoord {
                    x: 1,
                    y: 0,
                    z: None,
                },
                resolved_visual: None,
            },
        ],
    };

    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["kind"], "animation_batch");
    assert_eq!(value["animations"].as_array().unwrap().len(), 2);
    assert_eq!(
        serde_json::from_value::<RuntimePresentationEvent>(value).unwrap(),
        event
    );
}

#[test]
fn raster_batch_serializes_identity_and_geometry_without_asset_bytes() {
    let content = RuntimeResolvedRenderBatchContent::RasterImage {
        asset: VisualImageAssetId("visuals/tile.png".to_string()),
        revision: VisualImageAssetRevision("canonical-rgba-revision".to_string()),
        source_size: [2, 2],
        destination: RuntimeResolvedRect2d {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        uv: RuntimeResolvedRect2d {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        sampling: RuntimeResolvedSampling::Pixelated,
    };

    let value = serde_json::to_value(&content).unwrap();
    assert_eq!(value["kind"], "raster_image");
    assert_eq!(value["asset"], "visuals/tile.png");
    assert_eq!(value["revision"], "canonical-rgba-revision");
    assert_eq!(value["sourceSize"], serde_json::json!([2, 2]));
    assert!(value.get("rgba8Srgb").is_none());
    assert!(value.get("source").is_none());
    assert_eq!(
        serde_json::from_value::<RuntimeResolvedRenderBatchContent>(value).unwrap(),
        content
    );
}
