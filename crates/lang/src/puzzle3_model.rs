use std::collections::HashMap;

use puzzle_grid3d::{
    CompiledGame3, InputDef3, InputId, LevelBundle3, ObjectId, RuleStep3, WinCondition3,
};
use puzzle_kernel::LocalFrame;
use puzzle_runtime_contract::{Puzzle3CameraEffect, RuntimeLifecycle};

use crate::{SolverStrategy3, SpriteSet3, VisualOrderDef};

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedPuzzle3 {
    pub game: CompiledGame3,
    pub inputs: Vec<InputDef3>,
    pub object_labels: HashMap<ObjectId, String>,
    pub viewport_focus_objects: Vec<ObjectId>,
    pub settings: ModelSettings3,
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rule_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    pub level_bundle: Option<LevelBundle3>,
    pub level_packs: Vec<Option<String>>,
    pub win_condition: Option<WinCondition3>,
    pub solver_strategy: SolverStrategy3,
    pub lifecycle: RuntimeLifecycle<RuleStep3, LocalFrame<ObjectId>>,
    pub on_level_start_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    pub sprite_set: Option<SpriteSet3>,
    pub visual_order: VisualOrderDef,
}

impl ParsedPuzzle3 {
    pub fn input(&self, input: InputId) -> Option<&InputDef3> {
        self.inputs.iter().find(|def| def.id == input)
    }

    pub fn input_by_name(&self, name: &str) -> Option<&InputDef3> {
        self.inputs.iter().find(|def| def.name == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSettings3 {
    pub camera: CameraSettings3,
    pub grid: GridSettings3,
    pub sprite: SpriteRenderSettings3,
    pub shadow: bool,
    pub viewport: ViewportSettings3,
    pub pixelate: PixelateRenderSettings3,
}

impl Default for ModelSettings3 {
    fn default() -> Self {
        Self {
            camera: CameraSettings3::default(),
            grid: GridSettings3::default(),
            sprite: SpriteRenderSettings3::default(),
            shadow: false,
            viewport: ViewportSettings3::default(),
            pixelate: PixelateRenderSettings3::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraSettings3 {
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
            yaw_degrees: 0,
            pitch_degrees: 90,
            roll_degrees: 0,
            zoom_milli: 1000,
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
