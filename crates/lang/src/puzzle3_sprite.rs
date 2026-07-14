use std::collections::{BTreeMap, HashMap};

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

pub(crate) fn parse_spatial_ops(
    syntax: &crate::sprite_authoring::SpriteNodeSyntax,
) -> Result<Vec<SpriteSpatialOp3>, crate::DiagnosticReport> {
    parse_spatial_ops_with_bindings(syntax, &HashMap::new())
}

pub(crate) fn parse_spatial_ops_with_bindings(
    syntax: &crate::sprite_authoring::SpriteNodeSyntax,
    bindings: &HashMap<String, String>,
) -> Result<Vec<SpriteSpatialOp3>, crate::DiagnosticReport> {
    let mut ops = Vec::new();
    for (property, line) in &syntax.properties {
        match property {
            crate::sprite_authoring::SpritePropertySyntax::Translate { space, value } => {
                ops.push(SpriteSpatialOp3::Translate {
                    space: sprite_space(*space),
                    value: parse_vec3(value).map_err(|error| {
                        crate::DiagnosticReport::error(format!("{line}: {error}"))
                    })?,
                });
            }
            crate::sprite_authoring::SpritePropertySyntax::Rotate {
                space,
                angle,
                from,
                axis,
            } => {
                let axis = axis.as_deref().unwrap_or("up");
                let mut degrees = parse_angle(angle, bindings).map_err(|error| {
                    crate::DiagnosticReport::error(format!("{line}: {error}"))
                })?;
                if let Some(from) = from {
                    degrees -= parse_angle(from, bindings).map_err(|error| {
                        crate::DiagnosticReport::error(format!("{line}: {error}"))
                    })?;
                }
                ops.push(SpriteSpatialOp3::Rotate {
                    space: sprite_space(*space),
                    axis: parse_axis(axis).map_err(|error| {
                        crate::DiagnosticReport::error(format!("{line}: {error}"))
                    })?,
                    degrees,
                });
            }
            crate::sprite_authoring::SpritePropertySyntax::Unknown(name) if name == "rotate" => {
                return Err(crate::DiagnosticReport::error(
                    "removed sprite rotation syntax; use rotate [world|local] <angle> [from <angle>] [around <axis>]".to_string(),
                ));
            }
            _ => {
                return Err(crate::DiagnosticReport::error(format!(
                    "sprite property is not supported by voxel sprites: {line}"
                )));
            }
        }
    }
    Ok(ops)
}

fn sprite_space(space: crate::sprite_authoring::SpriteSpaceSyntax) -> SpriteSpace3 {
    match space {
        crate::sprite_authoring::SpriteSpaceSyntax::World => SpriteSpace3::World,
        crate::sprite_authoring::SpriteSpaceSyntax::Local => SpriteSpace3::Local,
    }
}

fn parse_scalar(value: &str) -> Result<f64, String> {
    let value = value.trim();
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator
            .trim()
            .parse::<f64>()
            .map_err(|_| "sprite spatial value must be numeric".to_string())?;
        let denominator = denominator
            .trim()
            .parse::<f64>()
            .map_err(|_| "sprite spatial value must be numeric".to_string())?;
        if denominator == 0.0 {
            return Err("sprite spatial value cannot divide by zero".to_string());
        }
        return Ok(numerator / denominator);
    }
    value
        .parse::<f64>()
        .map_err(|_| "sprite spatial value must be numeric".to_string())
}

fn parse_vec3(value: &str) -> Result<[f64; 3], String> {
    let inner = value
        .trim()
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| "3D sprite translate requires a vec3".to_string())?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let [x, y, z] = parts.as_slice() else {
        return Err("3D sprite translate requires a vec3".to_string());
    };
    Ok([parse_scalar(x)?, parse_scalar(y)?, parse_scalar(z)?])
}

fn parse_axis(value: &str) -> Result<[f64; 3], String> {
    let axis = match value.trim() {
        "right" => [1.0, 0.0, 0.0],
        "left" => [-1.0, 0.0, 0.0],
        "front" => [0.0, 1.0, 0.0],
        "back" => [0.0, -1.0, 0.0],
        "up" => [0.0, 0.0, 1.0],
        "down" => [0.0, 0.0, -1.0],
        _ => parse_vec3(value)?,
    };
    let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if length == 0.0 {
        return Err("3D sprite rotate axis cannot be zero".to_string());
    }
    Ok([axis[0] / length, axis[1] / length, axis[2] / length])
}

fn parse_angle(value: &str, bindings: &HashMap<String, String>) -> Result<f64, String> {
    let value = value.trim();
    let value = bindings.get(value).map(String::as_str).unwrap_or(value);
    if let Some(degrees) = horizontal_direction_degrees(value) {
        return Ok(degrees);
    }
    let degrees = value.strip_suffix("deg").ok_or_else(|| {
        "3D sprite rotate expression must resolve to an angle or horizontal direction".to_string()
    })?;
    parse_scalar(degrees)
}

fn horizontal_direction_degrees(value: &str) -> Option<f64> {
    Some(match value {
        "right" => 0.0,
        "front" => 90.0,
        "left" => 180.0,
        "back" => -90.0,
        _ => return None,
    })
}

pub(crate) fn is_palette_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace().peekable();
    tokens.peek().is_some()
        && tokens.all(|token| token == "transparent" || crate::is_visual_color_token(token))
}

pub(crate) fn parse_palette_line(
    line: &str,
) -> Result<BTreeMap<char, SpriteColor3>, crate::DiagnosticReport> {
    if !is_palette_line(line) {
        return Err(crate::DiagnosticReport::error(
            "sprite palette row must be whitespace-separated <color|transparent> values"
                .to_string(),
        ));
    }
    let mut palette = BTreeMap::new();
    for (index, token) in line.split_whitespace().enumerate() {
        const KEYS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let key = KEYS.chars().nth(index).ok_or_else(|| {
            crate::DiagnosticReport::error(
                "sprite palette supports at most 62 colors".to_string(),
            )
        })?;
        let color = if token == "transparent" {
            SpriteColor3::Transparent
        } else {
            SpriteColor3::Hex(token.to_string())
        };
        palette.insert(key, color);
    }
    Ok(palette)
}

pub(crate) fn parse_voxel_layers(
    sprite_name: &str,
    layers: &[Vec<String>],
    palette: &BTreeMap<char, SpriteColor3>,
) -> Result<SpriteVoxels3, crate::DiagnosticReport> {
    let Some(first_layer) = layers.first() else {
        return Err(crate::DiagnosticReport::error(format!(
            "sprite {sprite_name} requires at least one Z layer"
        )));
    };
    let Some(first_row) = first_layer.first() else {
        return Err(crate::DiagnosticReport::error(format!(
            "sprite {sprite_name} Z layer requires at least one row"
        )));
    };
    let height = first_layer.len();
    let width = first_row.chars().count();
    if width == 0 {
        return Err(crate::DiagnosticReport::error(format!(
            "sprite {sprite_name} has an empty row"
        )));
    }
    for layer in layers {
        if layer.is_empty() {
            return Err(crate::DiagnosticReport::error(format!(
                "sprite {sprite_name} Z layer requires at least one row"
            )));
        }
        if layer.len() != height {
            return Err(crate::DiagnosticReport::error(format!(
                "sprite {sprite_name} Z layers must have the same height"
            )));
        }
        for row in layer {
            if row.chars().count() != width {
                return Err(crate::DiagnosticReport::error(format!(
                    "sprite {sprite_name} Z layers must have the same width"
                )));
            }
            for ch in row.chars() {
                if ch != '.' && ch != ' ' && !palette.contains_key(&ch) {
                    return Err(crate::DiagnosticReport::error(format!(
                        "sprite {sprite_name} uses undefined color key: {ch}"
                    )));
                }
            }
        }
    }
    Ok(SpriteVoxels3::new(
        Size3::new(width as u16, height as u16, layers.len() as u16),
        layers.to_vec(),
    ))
}
