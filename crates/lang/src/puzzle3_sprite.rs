use std::collections::BTreeMap;

use puzzle_grid3d::Size3;

#[derive(Clone, Debug, PartialEq)]
pub struct SpriteSet3 {
    pub name: String,
    pub model: Option<String>,
    pub sprites: Vec<Sprite3>,
}

impl SpriteSet3 {
    pub fn new(name: impl Into<String>, model: Option<String>, sprites: Vec<Sprite3>) -> Self {
        Self {
            name: name.into(),
            model,
            sprites,
        }
    }

    pub fn sprite(&self, name: &str) -> Option<&Sprite3> {
        self.sprites.iter().find(|sprite| sprite.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite3 {
    pub name: String,
    pub palette: BTreeMap<char, SpriteColor3>,
    pub frames: Vec<SpriteVoxels3>,
    pub duration_ms: Option<u64>,
    pub frame_duration_ms: Option<u64>,
    pub spatial_ops: Vec<SpriteSpatialOp3>,
}

impl Sprite3 {
    pub fn new(
        name: impl Into<String>,
        palette: BTreeMap<char, SpriteColor3>,
        frames: Vec<SpriteVoxels3>,
        duration_ms: Option<u64>,
        frame_duration_ms: Option<u64>,
    ) -> Self {
        Self {
            name: name.into(),
            palette,
            frames,
            duration_ms,
            frame_duration_ms,
            spatial_ops: Vec::new(),
        }
    }

    pub fn first_frame(&self) -> &SpriteVoxels3 {
        self.frames
            .first()
            .expect("checked sprite has at least one frame")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteSpace3 {
    World,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteSpatialOp3 {
    Translate {
        space: SpriteSpace3,
        value: [f64; 3],
    },
    Rotate {
        space: SpriteSpace3,
        axis: [f64; 3],
        degrees: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpriteColor3 {
    Transparent,
    Hex(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteVoxels3 {
    pub size: Size3,
    pub slices: Vec<Vec<String>>,
}

impl SpriteVoxels3 {
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
