use puzzle_grid3d::{Game3, LevelBundle3, ObjectId, Rule3, WinCondition3};
use puzzle_grid3d_authoring::SelectorCatalog3;
use puzzle_kernel::LocalFrame;
use puzzle_runtime_contract::{Puzzle3CameraEffect, RuntimeLifecycle};

use crate::{SolverStrategy3, SpriteSet3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedPuzzle3 {
    pub game: Game3,
    pub catalog: SelectorCatalog3,
    pub settings: ModelSettings3,
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rules: Vec<Rule3>,
    pub display_objects: Vec<ObjectId>,
    pub rule_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    pub level_bundle: Option<LevelBundle3>,
    pub level_packs: Vec<Option<String>>,
    pub win_condition: Option<WinCondition3>,
    pub solver_strategy: SolverStrategy3,
    pub lifecycle: RuntimeLifecycle<Rule3, LocalFrame<ObjectId>>,
    pub on_level_start_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    pub sprite_set: Option<SpriteSet3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSettings3 {
    pub camera: CameraSettings3,
    pub grid: GridSettings3,
    pub sprite: SpriteRenderSettings3,
    pub viewport: ViewportSettings3,
    pub pixelate: PixelateRenderSettings3,
}

impl Default for ModelSettings3 {
    fn default() -> Self {
        Self {
            camera: CameraSettings3::default(),
            grid: GridSettings3::default(),
            sprite: SpriteRenderSettings3::default(),
            viewport: ViewportSettings3::default(),
            pixelate: PixelateRenderSettings3::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraSettings3 {
    pub yaw_degrees: i16,
    pub pitch_degrees: i16,
    pub zoom_milli: u16,
    pub interactive_look: bool,
    pub interactive_zoom: bool,
}

impl Default for CameraSettings3 {
    fn default() -> Self {
        Self {
            yaw_degrees: 34,
            pitch_degrees: 38,
            zoom_milli: 1100,
            interactive_look: false,
            interactive_zoom: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridSettings3 {
    pub occupied_cells: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteRenderSettings3 {
    pub shade: bool,
}

impl Default for SpriteRenderSettings3 {
    fn default() -> Self {
        Self { shade: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportMode3 {
    Full,
    Centered,
    Paged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportFollow3 {
    Snap,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportFraming3 {
    pub width: u16,
    pub depth: u16,
    pub height: ViewportHeight3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportHeight3 {
    Full,
    Size(u16),
}
