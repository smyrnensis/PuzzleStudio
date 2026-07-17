use std::collections::BTreeMap;

use puzzle_core::Size3;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VoxelSpriteSet {
    pub name: String,
    pub model: Option<String>,
    pub sprites: Vec<VoxelSprite>,
}

impl VoxelSpriteSet {
    pub fn new(name: impl Into<String>, model: Option<String>, sprites: Vec<VoxelSprite>) -> Self {
        Self {
            name: name.into(),
            model,
            sprites,
        }
    }

    pub fn sprite(&self, name: &str) -> Option<&VoxelSprite> {
        self.sprites.iter().find(|sprite| sprite.name == name)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VoxelSprite {
    pub name: String,
    pub palette: BTreeMap<char, VoxelColor>,
    pub frames: Vec<VoxelFrame>,
    pub duration_ms: Option<u64>,
    pub frame_duration_ms: Option<u64>,
    pub transforms: Vec<crate::VisualSpriteTransform>,
}

impl VoxelSprite {
    pub fn new(
        name: impl Into<String>,
        palette: BTreeMap<char, VoxelColor>,
        frames: Vec<VoxelFrame>,
        duration_ms: Option<u64>,
        frame_duration_ms: Option<u64>,
    ) -> Self {
        Self {
            name: name.into(),
            palette,
            frames,
            duration_ms,
            frame_duration_ms,
            transforms: Vec::new(),
        }
    }

    pub fn first_frame(&self) -> &VoxelFrame {
        self.frames
            .first()
            .expect("checked sprite has at least one frame")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VoxelColor {
    Transparent,
    Hex(String),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VoxelFrame {
    pub size: Size3,
    pub slices: Vec<Vec<String>>,
}

impl VoxelFrame {
    pub fn new(size: Size3, slices: Vec<Vec<String>>) -> Self {
        Self { size, slices }
    }

    pub fn height(&self) -> u16 {
        self.size.height
    }

    pub fn depth(&self) -> u16 {
        self.size.depth
    }

    pub fn width(&self) -> u16 {
        self.size.width
    }
}
