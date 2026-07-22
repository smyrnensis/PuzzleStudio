use puzzle_core::ObjectId;
use puzzle_kernel::LocalFrame;
use puzzle_runtime_contract::CameraEffect;

use crate::{VisualOrderDef, VoxelVisualSet};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpatialPresentation {
    pub viewport_focus_objects: Vec<ObjectId>,
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rule_camera_effects: Vec<Vec<CameraEffect>>,
    pub on_level_start_camera_effects: Vec<Vec<CameraEffect>>,
    pub visual_set: Option<VoxelVisualSet>,
    pub visual_order: VisualOrderDef,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CameraSettings3 {
    pub projection: CameraProjection3,
    pub yaw_degrees: i16,
    pub pitch_degrees: i16,
    pub roll_degrees: i16,
    pub zoom_milli: u16,
    pub interactive_look: bool,
    pub interactive_zoom: bool,
}

impl Default for CameraSettings3 {
    fn default() -> Self {
        Self {
            projection: CameraProjection3::Perspective,
            yaw_degrees: 0,
            pitch_degrees: 90,
            roll_degrees: 0,
            zoom_milli: 1000,
            interactive_look: false,
            interactive_zoom: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraProjection3 {
    Perspective,
    Orthographic,
}

impl CameraProjection3 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Perspective => "perspective",
            Self::Orthographic => "orthographic",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VisualRenderSettings3 {
    pub shade: bool,
}

impl Default for VisualRenderSettings3 {
    fn default() -> Self {
        Self { shade: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PixelateRenderSettings3 {
    pub enabled: bool,
    pub scale: u16,
    pub smoothing: bool,
}

impl Default for PixelateRenderSettings3 {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 4,
            smoothing: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViewportSettings3 {
    pub mode: ViewportMode3,
    pub follow: ViewportFollow3,
    pub framing: Option<ViewportFraming3>,
    pub focus: String,
}

impl Default for ViewportSettings3 {
    fn default() -> Self {
        Self {
            mode: ViewportMode3::Full,
            follow: ViewportFollow3::Snap,
            framing: None,
            focus: "Player".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewportMode3 {
    Full,
    Centered,
    Paged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewportFollow3 {
    Snap,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViewportFraming3 {
    pub width: u16,
    pub depth: u16,
    pub height: ViewportHeight3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewportHeight3 {
    Full,
    Size(u16),
}
