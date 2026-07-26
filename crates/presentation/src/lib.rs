use std::collections::{BTreeMap, BTreeSet, HashMap};

use puzzle_assets::{DecodedVisualImageAsset, DecodedVisualImageCatalog};
use puzzle_runtime_contract::{
    RuntimeAnimationEvent, RuntimeGridMode, RuntimeLinearRgba, RuntimePresentationEvent,
    RuntimePuzzle3SpatialOp, RuntimePuzzle3VisualSpace, RuntimeResolvedDecoration,
    RuntimeResolvedFitMode, RuntimeResolvedLineDepth3d, RuntimeResolvedLineLayer2d,
    RuntimeResolvedLineSegment2d, RuntimeResolvedLineSegment3d, RuntimeResolvedLineStyle,
    RuntimeResolvedPixelGeometry, RuntimeResolvedPlayback, RuntimeResolvedRect2d,
    RuntimeResolvedRenderBatch, RuntimeResolvedRenderBatchContent, RuntimeResolvedRenderCell,
    RuntimeResolvedRenderFrame, RuntimeResolvedRenderInstance, RuntimeResolvedRenderMoment,
    RuntimeResolvedRenderScene, RuntimeResolvedSampling, RuntimeResolvedStrokeWidth,
    RuntimeResolvedView2d, RuntimeResolvedVisualClip, RuntimeResolvedVisualFrame,
    RuntimeResolvedVisualOrder, RuntimeScalarTween, RuntimeTheme, RuntimeUiControlStyle,
    RuntimeUiTextStyle, RuntimeUiTypography, RuntimeVisualComposition, RuntimeVisualSpace,
    RuntimeVisualState, RuntimeVisualTransform, RuntimeVisualTween, RuntimeVisualTweenTransform,
};

#[derive(Clone, Copy, Debug)]
pub struct VisualPriorityRef<'a> {
    pub objects: &'a [String],
    pub animations: &'a [String],
    pub merge: bool,
}

#[derive(Clone, Debug)]
pub struct VisualOrderRef<'a> {
    pub direction_priority: &'a [String],
    pub priorities: Vec<VisualPriorityRef<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualComposition {
    Ordered,
    Average,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedVisualPriority {
    pub index: usize,
    pub composition: VisualComposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationError {
    MissingVisualOrder,
    UnknownObject(String),
    UnknownAnimation(String),
    InvalidDirection(String),
    InvalidOccurrenceId(u64),
    IncompatibleTransformCount,
    IncompatibleTransform { index: usize, reason: &'static str },
    IncompatibleOpacity,
    NonFiniteValue { label: &'static str },
    ZeroRotationAxis,
    InvalidColor(String),
    InvalidThemePreset(String),
    InvalidThemeSetting(String),
    InvalidThemeContract(&'static str),
    InvalidViewSize,
    UnknownPaletteToken(String),
    EmptyVisualClip(String),
    ZeroFrameDuration(String),
    UnknownVisual(String),
    UnknownRenderInstance(u64),
    MixedCompositionCells,
    MixedCompositionTransforms,
    NonLatticeCompositionTransform,
    IncompatibleCompositionFrames,
    RasterImageComposition,
    MissingImageAsset(String),
    InvalidImageAssetReference(String),
    ZeroAnimationDuration,
    MissingAnimationTarget { object_id: u16, cell: [i32; 3] },
    MissingRenderCell([i32; 3]),
    MissingResolvedAnimationVisual(String),
}

pub fn resolve_runtime_theme<'a>(
    preset: Option<&str>,
    variables: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<RuntimeTheme, PresentationError> {
    let preset = preset.unwrap_or("clean");
    let (background, text, accent, typography, control_layout) = match preset {
        "clean" => (
            "#f5f3ef",
            "#1f2428",
            "#1f2428",
            typography(30.0, 1.08, 24.0, 1.3, 16.0, 1.5, 12.0, 1.4),
            control_layout(10.0, 10.0, 4.0, 2.0, 6.0),
        ),
        "terminal" => (
            "#000000",
            "#ffffff",
            "#ffffff",
            typography(28.0, 1.25, 14.0, 1.1, 18.0, 1.1, 12.0, 1.2),
            control_layout(8.0, 5.0, 4.0, 0.0, 0.0),
        ),
        "paper" => (
            "#f4ecd9",
            "#2b2419",
            "#8d5d2a",
            typography(32.0, 1.2, 24.0, 1.3, 16.0, 1.5, 12.0, 1.4),
            control_layout(16.0, 10.0, 4.0, 1.0, 4.0),
        ),
        "pixel" => (
            "#08080c",
            "#f8f8f8",
            "#f8f8f8",
            typography(28.0, 1.2, 20.0, 1.2, 16.0, 1.25, 12.0, 1.2),
            control_layout(10.0, 8.0, 4.0, 4.0, 0.0),
        ),
        "puzzlescript" => (
            "#000000",
            "#ffffff",
            "#ffffff",
            typography(28.0, 1.2, 20.0, 1.2, 16.0, 1.25, 12.0, 1.2),
            control_layout(10.0, 8.0, 4.0, 4.0, 0.0),
        ),
        "candy" => (
            "#fff7fb",
            "#33404a",
            "#d76f97",
            typography(32.0, 1.2, 24.0, 1.3, 16.0, 1.5, 12.0, 1.4),
            control_layout(16.0, 10.0, 4.0, 1.0, 12.0),
        ),
        "blueprint" => (
            "#0d334e",
            "#e9f8ff",
            "#ffd166",
            typography(32.0, 1.2, 24.0, 1.3, 16.0, 1.5, 12.0, 1.4),
            control_layout(14.0, 9.0, 4.0, 1.0, 2.0),
        ),
        "noir" => (
            "#101010",
            "#f4f1e8",
            "#f2c14e",
            typography(32.0, 1.2, 24.0, 1.3, 16.0, 1.5, 12.0, 1.4),
            control_layout(14.0, 9.0, 4.0, 1.0, 0.0),
        ),
        name => return Err(PresentationError::InvalidThemePreset(name.to_string())),
    };
    let mut background = resolve_palette_color(background)?;
    let mut text = resolve_palette_color(text)?;
    let mut accent = resolve_palette_color(accent)?;
    for (name, value) in variables {
        let target = match name {
            "background" => &mut background,
            "text" => &mut text,
            "accent" => &mut accent,
            name => return Err(PresentationError::InvalidThemeSetting(name.to_string())),
        };
        *target = resolve_palette_color(value)?;
    }
    let theme = RuntimeTheme {
        background,
        text,
        muted_text: with_alpha(text, 0.66),
        accent,
        panel: with_alpha(background, 0.96),
        control: with_alpha(accent, 0.14),
        control_focused: with_alpha(accent, 0.22),
        control_selected: with_alpha(accent, 0.18),
        control_selected_border: with_alpha(accent, 0.42),
        typography,
        control_layout,
    };
    theme
        .validate()
        .map_err(PresentationError::InvalidThemeContract)?;
    Ok(theme)
}

fn with_alpha(mut color: RuntimeLinearRgba, alpha: f64) -> RuntimeLinearRgba {
    color.alpha *= alpha;
    color
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode2d {
    Paged,
    Centered,
}

pub fn resolve_view_2d(
    board_size: [u16; 2],
    requested_size: Option<[u16; 2]>,
    mode: ViewMode2d,
    focus: Option<[i32; 2]>,
) -> Result<RuntimeResolvedView2d, PresentationError> {
    let [board_width, board_height] = board_size;
    if board_width == 0 || board_height == 0 {
        return Err(PresentationError::InvalidViewSize);
    }
    let Some([requested_width, requested_height]) = requested_size else {
        return Ok(RuntimeResolvedView2d {
            origin: [0, 0],
            size: board_size,
        });
    };
    if requested_width == 0 || requested_height == 0 {
        return Err(PresentationError::InvalidViewSize);
    }
    let width = requested_width.min(board_width);
    let height = requested_height.min(board_height);
    let [focus_x, focus_y] = focus.unwrap_or([0, 0]);
    let origin = match mode {
        ViewMode2d::Paged => [
            focus_x
                .max(0)
                .div_euclid(i32::from(width))
                .saturating_mul(i32::from(width))
                .min(i32::from(board_width.saturating_sub(width))),
            focus_y
                .max(0)
                .div_euclid(i32::from(height))
                .saturating_mul(i32::from(height))
                .min(i32::from(board_height.saturating_sub(height))),
        ],
        ViewMode2d::Centered => [
            (focus_x - i32::from(width / 2)).clamp(0, i32::from(board_width.saturating_sub(width))),
            (focus_y - i32::from(height / 2))
                .clamp(0, i32::from(board_height.saturating_sub(height))),
        ],
    };
    Ok(RuntimeResolvedView2d {
        origin,
        size: [width, height],
    })
}

pub fn resolve_grid_decoration_2d(
    mode: RuntimeGridMode,
    view: RuntimeResolvedView2d,
    cells: &[RuntimeResolvedRenderCell],
    theme: &RuntimeTheme,
) -> Option<RuntimeResolvedDecoration> {
    let segments = match mode {
        RuntimeGridMode::Hidden => return None,
        RuntimeGridMode::OccupiedCells => occupied_grid_segments_2d(view, cells),
        RuntimeGridMode::AllCells => all_grid_segments_2d(view),
    };
    Some(RuntimeResolvedDecoration::Lines2d {
        segments,
        style: RuntimeResolvedLineStyle {
            color: with_alpha(theme.text, 0.34),
            width: RuntimeResolvedStrokeWidth::CellRelative {
                cell_fraction: 1.0 / 24.0,
                min_physical_pixels: 1.0,
            },
        },
        layer: RuntimeResolvedLineLayer2d::Overlay,
    })
}

pub fn resolve_grid_decoration_3d(
    mode: RuntimeGridMode,
    size: [u16; 3],
    cells: &[RuntimeResolvedRenderCell],
    theme: &RuntimeTheme,
) -> Option<RuntimeResolvedDecoration> {
    let segments = match mode {
        RuntimeGridMode::Hidden => return None,
        RuntimeGridMode::OccupiedCells => occupied_grid_segments_3d(cells),
        RuntimeGridMode::AllCells => all_grid_segments_3d(size),
    };
    Some(RuntimeResolvedDecoration::Lines3d {
        segments,
        style: RuntimeResolvedLineStyle {
            color: with_alpha(theme.text, 0.45),
            width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 1.0 },
        },
        depth: RuntimeResolvedLineDepth3d::Tested,
    })
}

fn occupied_grid_segments_2d(
    view: RuntimeResolvedView2d,
    cells: &[RuntimeResolvedRenderCell],
) -> Vec<RuntimeResolvedLineSegment2d> {
    let [origin_x, origin_y] = view.origin;
    let max_x = origin_x + i32::from(view.size[0]);
    let max_y = origin_y + i32::from(view.size[1]);
    let mut edges = BTreeSet::new();
    for cell in cells.iter().filter(|cell| {
        !cell.object_ids.is_empty()
            && cell.position[0] >= origin_x
            && cell.position[0] < max_x
            && cell.position[1] >= origin_y
            && cell.position[1] < max_y
    }) {
        let [x, y, _] = cell.position;
        insert_edge_2d(&mut edges, [x, y], [x + 1, y]);
        insert_edge_2d(&mut edges, [x, y], [x, y + 1]);
        insert_edge_2d(&mut edges, [x + 1, y], [x + 1, y + 1]);
        insert_edge_2d(&mut edges, [x, y + 1], [x + 1, y + 1]);
    }
    edges
        .into_iter()
        .map(|(start, end)| RuntimeResolvedLineSegment2d {
            start: start.map(f64::from),
            end: end.map(f64::from),
        })
        .collect()
}

fn all_grid_segments_2d(view: RuntimeResolvedView2d) -> Vec<RuntimeResolvedLineSegment2d> {
    let [origin_x, origin_y] = view.origin;
    let width = i32::from(view.size[0]);
    let height = i32::from(view.size[1]);
    let mut segments = Vec::with_capacity(
        usize::from(view.size[1]) * (usize::from(view.size[0]) + 1)
            + usize::from(view.size[0]) * (usize::from(view.size[1]) + 1),
    );
    for y in origin_y..origin_y + height {
        for x in origin_x..=origin_x + width {
            segments.push(RuntimeResolvedLineSegment2d {
                start: [f64::from(x), f64::from(y)],
                end: [f64::from(x), f64::from(y + 1)],
            });
        }
    }
    for y in origin_y..=origin_y + height {
        for x in origin_x..origin_x + width {
            segments.push(RuntimeResolvedLineSegment2d {
                start: [f64::from(x), f64::from(y)],
                end: [f64::from(x + 1), f64::from(y)],
            });
        }
    }
    segments
}

fn insert_edge_2d(edges: &mut BTreeSet<([i32; 2], [i32; 2])>, first: [i32; 2], second: [i32; 2]) {
    edges.insert(if first <= second {
        (first, second)
    } else {
        (second, first)
    });
}

fn occupied_grid_segments_3d(
    cells: &[RuntimeResolvedRenderCell],
) -> Vec<RuntimeResolvedLineSegment3d> {
    let mut edges = BTreeSet::new();
    for cell in cells.iter().filter(|cell| !cell.object_ids.is_empty()) {
        let [x, y, z] = cell.position.map(|value| value.saturating_mul(2));
        let [x0, x1] = [x - 1, x + 1];
        let [y0, y1] = [y - 1, y + 1];
        let [z0, z1] = [z - 1, z + 1];
        for y in [y0, y1] {
            for z in [z0, z1] {
                insert_edge_3d(&mut edges, [x0, y, z], [x1, y, z]);
            }
        }
        for x in [x0, x1] {
            for z in [z0, z1] {
                insert_edge_3d(&mut edges, [x, y0, z], [x, y1, z]);
            }
        }
        for x in [x0, x1] {
            for y in [y0, y1] {
                insert_edge_3d(&mut edges, [x, y, z0], [x, y, z1]);
            }
        }
    }
    edges
        .into_iter()
        .map(|(start, end)| RuntimeResolvedLineSegment3d {
            start: start.map(|value| f64::from(value) / 2.0),
            end: end.map(|value| f64::from(value) / 2.0),
        })
        .collect()
}

fn all_grid_segments_3d(size: [u16; 3]) -> Vec<RuntimeResolvedLineSegment3d> {
    let [width, depth, height] = size.map(i32::from);
    let half = |value: i32| f64::from(value.saturating_mul(2) - 1) / 2.0;
    let mut segments = Vec::new();
    for x in 0..width {
        for y in 0..=depth {
            for z in 0..=height {
                segments.push(RuntimeResolvedLineSegment3d {
                    start: [half(x), half(y), half(z)],
                    end: [half(x + 1), half(y), half(z)],
                });
            }
        }
    }
    for x in 0..=width {
        for y in 0..depth {
            for z in 0..=height {
                segments.push(RuntimeResolvedLineSegment3d {
                    start: [half(x), half(y), half(z)],
                    end: [half(x), half(y + 1), half(z)],
                });
            }
        }
    }
    for x in 0..=width {
        for y in 0..=depth {
            for z in 0..height {
                segments.push(RuntimeResolvedLineSegment3d {
                    start: [half(x), half(y), half(z)],
                    end: [half(x), half(y), half(z + 1)],
                });
            }
        }
    }
    segments
}

fn insert_edge_3d(edges: &mut BTreeSet<([i32; 3], [i32; 3])>, first: [i32; 3], second: [i32; 3]) {
    edges.insert(if first <= second {
        (first, second)
    } else {
        (second, first)
    });
}

fn typography(
    heading_size: f32,
    heading_line_height: f32,
    subheading_size: f32,
    subheading_line_height: f32,
    body_size: f32,
    body_line_height: f32,
    caption_size: f32,
    caption_line_height: f32,
) -> RuntimeUiTypography {
    RuntimeUiTypography {
        heading: RuntimeUiTextStyle {
            font_size_px: heading_size,
            line_height: heading_line_height,
        },
        subheading: RuntimeUiTextStyle {
            font_size_px: subheading_size,
            line_height: subheading_line_height,
        },
        body: RuntimeUiTextStyle {
            font_size_px: body_size,
            line_height: body_line_height,
        },
        caption: RuntimeUiTextStyle {
            font_size_px: caption_size,
            line_height: caption_line_height,
        },
    }
}

fn control_layout(
    padding_horizontal_px: f32,
    padding_vertical_px: f32,
    margin_px: f32,
    border_width_px: f32,
    corner_radius_px: f32,
) -> RuntimeUiControlStyle {
    RuntimeUiControlStyle {
        padding_horizontal_px,
        padding_vertical_px,
        margin_px,
        border_width_px,
        corner_radius_px,
    }
}

pub fn resolve_palette_color(color: &str) -> Result<RuntimeLinearRgba, PresentationError> {
    if color.eq_ignore_ascii_case("transparent") {
        return Ok(RuntimeLinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        });
    }
    let hex = color
        .strip_prefix('#')
        .ok_or_else(|| PresentationError::InvalidColor(color.to_string()))?;
    let expanded = match hex.len() {
        3 | 4 => hex
            .chars()
            .flat_map(|digit| [digit, digit])
            .collect::<String>(),
        6 | 8 => hex.to_string(),
        _ => return Err(PresentationError::InvalidColor(color.to_string())),
    };
    let channel = |offset| {
        u8::from_str_radix(&expanded[offset..offset + 2], 16)
            .map_err(|_| PresentationError::InvalidColor(color.to_string()))
    };
    let red = channel(0)?;
    let green = channel(2)?;
    let blue = channel(4)?;
    let alpha = if expanded.len() == 8 {
        channel(6)?
    } else {
        255
    };
    Ok(RuntimeLinearRgba {
        red: srgb_channel_to_linear(red),
        green: srgb_channel_to_linear(green),
        blue: srgb_channel_to_linear(blue),
        alpha: f64::from(alpha) / 255.0,
    })
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

pub fn resolve_pixel_frame(
    rows: &[String],
    palette: &BTreeMap<String, String>,
) -> Result<RuntimeResolvedVisualFrame, PresentationError> {
    let height =
        u16::try_from(rows.len()).map_err(|_| PresentationError::IncompatibleCompositionFrames)?;
    let width = rows.first().map(|row| row.chars().count()).unwrap_or(0);
    if rows.iter().any(|row| row.chars().count() != width) {
        return Err(PresentationError::IncompatibleCompositionFrames);
    }
    let width =
        u16::try_from(width).map_err(|_| PresentationError::IncompatibleCompositionFrames)?;
    let mut pixels = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, token) in row.chars().enumerate() {
            if token == '.' {
                continue;
            }
            let token = token.to_string();
            let color = palette
                .get(&token)
                .ok_or_else(|| PresentationError::UnknownPaletteToken(token.clone()))?;
            let color = resolve_palette_color(color)?;
            if color.alpha > 0.0 {
                pixels.push(puzzle_runtime_contract::RuntimeResolvedPixel {
                    position: [x as i32, y as i32],
                    color,
                });
            }
        }
    }
    Ok(RuntimeResolvedVisualFrame::Pixels {
        width,
        height,
        pixels,
    })
}

pub fn resolve_voxel_frame(
    layers: &[Vec<String>],
    palette: &BTreeMap<String, String>,
) -> Result<RuntimeResolvedVisualFrame, PresentationError> {
    let depth = layers.first().map(Vec::len).unwrap_or(0);
    let width = layers
        .first()
        .and_then(|layer| layer.first())
        .map(|row| row.chars().count())
        .unwrap_or(0);
    if layers
        .iter()
        .any(|layer| layer.len() != depth || layer.iter().any(|row| row.chars().count() != width))
    {
        return Err(PresentationError::IncompatibleCompositionFrames);
    }
    let mut voxels = Vec::new();
    for (z, layer) in layers.iter().enumerate() {
        for (y, row) in layer.iter().enumerate() {
            for (x, token) in row.chars().enumerate() {
                if token == '.' {
                    continue;
                }
                let token = token.to_string();
                let color = palette
                    .get(&token)
                    .ok_or_else(|| PresentationError::UnknownPaletteToken(token.clone()))?;
                let color = resolve_palette_color(color)?;
                if color.alpha > 0.0 {
                    voxels.push(puzzle_runtime_contract::RuntimeResolvedVoxel {
                        position: [
                            x as i32,
                            (depth - 1 - y) as i32,
                            (layers.len() - 1 - z) as i32,
                        ],
                        color,
                    });
                }
            }
        }
    }
    Ok(RuntimeResolvedVisualFrame::Voxels {
        width: u16::try_from(width)
            .map_err(|_| PresentationError::IncompatibleCompositionFrames)?,
        depth: u16::try_from(depth)
            .map_err(|_| PresentationError::IncompatibleCompositionFrames)?,
        height: u16::try_from(layers.len())
            .map_err(|_| PresentationError::IncompatibleCompositionFrames)?,
        voxels,
    })
}

pub fn resolve_render_frame(
    scene: &RuntimeResolvedRenderScene,
    assets: &DecodedVisualImageCatalog,
    elapsed_ms: u64,
) -> Result<RuntimeResolvedRenderFrame, PresentationError> {
    resolve_render_moment(
        scene,
        assets,
        &RuntimeResolvedRenderMoment {
            clip_elapsed_ms: elapsed_ms,
            animation_elapsed_ms: 0,
            animations: Vec::new(),
        },
    )
}

pub fn resolve_image_free_render_frame(
    scene: &RuntimeResolvedRenderScene,
    elapsed_ms: u64,
) -> Result<RuntimeResolvedRenderFrame, PresentationError> {
    resolve_render_frame(scene, &DecodedVisualImageCatalog::default(), elapsed_ms)
}

pub fn resolve_render_moment(
    scene: &RuntimeResolvedRenderScene,
    assets: &DecodedVisualImageCatalog,
    moment: &RuntimeResolvedRenderMoment,
) -> Result<RuntimeResolvedRenderFrame, PresentationError> {
    let animation_progress = if moment.animations.is_empty() {
        1.0
    } else {
        if scene.animation_duration_ms == 0 {
            return Err(PresentationError::ZeroAnimationDuration);
        }
        (moment.animation_elapsed_ms as f64 / scene.animation_duration_ms as f64).clamp(0.0, 1.0)
    };
    let clips = scene
        .clips
        .iter()
        .map(|clip| (clip.id.as_str(), clip))
        .collect::<HashMap<_, _>>();
    let (animated_instances, animated_groups) =
        animated_scene(scene, moment, animation_progress, &clips)?;
    let instances = animated_instances
        .iter()
        .map(|instance| (instance.id, instance))
        .collect::<HashMap<_, _>>();
    let mut batches = Vec::new();
    for group in &animated_groups {
        let members = group
            .instances
            .iter()
            .map(|id| {
                instances
                    .get(id)
                    .copied()
                    .ok_or(PresentationError::UnknownRenderInstance(*id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match group.composition {
            RuntimeVisualComposition::Ordered => {
                for instance in members {
                    batches.push(instance_batch(
                        instance,
                        &clips,
                        assets,
                        moment.clip_elapsed_ms,
                    )?);
                }
            }
            RuntimeVisualComposition::Average => {
                batches.push(average_batch(
                    &members,
                    &clips,
                    group.render_order,
                    moment.clip_elapsed_ms,
                )?);
            }
        }
    }
    batches.sort_by_key(|batch| batch.render_order);
    let continue_animation = animated_instances.iter().any(|instance| {
        instance.playback == RuntimeResolvedPlayback::Loop
            && clips
                .get(instance.visual.as_str())
                .is_some_and(|clip| clip.frames.len() > 1)
    }) || moment.animations.iter().any(|animation| match animation {
        RuntimeAnimationEvent::Move { .. } | RuntimeAnimationEvent::CantMove { .. } => {
            animation_progress < 1.0
        }
        RuntimeAnimationEvent::Animation { name, .. } => {
            clips
                .get(name.as_str())
                .and_then(|clip| {
                    clip.frame_duration_ms
                        .and_then(|frame| frame.checked_mul(clip.frames.len() as u64))
                })
                .unwrap_or(250)
                > moment.animation_elapsed_ms
        }
    });
    Ok(RuntimeResolvedRenderFrame {
        batches,
        decorations: scene.decorations.clone(),
        continue_animation,
    })
}

pub fn resolve_image_free_render_moment(
    scene: &RuntimeResolvedRenderScene,
    moment: &RuntimeResolvedRenderMoment,
) -> Result<RuntimeResolvedRenderFrame, PresentationError> {
    resolve_render_moment(scene, &DecodedVisualImageCatalog::default(), moment)
}

fn animated_scene(
    scene: &RuntimeResolvedRenderScene,
    moment: &RuntimeResolvedRenderMoment,
    animation_progress: f64,
    clips: &HashMap<&str, &puzzle_runtime_contract::RuntimeResolvedVisualClip>,
) -> Result<
    (
        Vec<RuntimeResolvedRenderInstance>,
        Vec<puzzle_runtime_contract::RuntimeResolvedCompositionGroup>,
    ),
    PresentationError,
> {
    let mut instances = scene.instances.clone();
    let mut groups = scene.composition_groups.clone();
    let mut next_id = instances
        .iter()
        .map(|instance| instance.id)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(PresentationError::InvalidOccurrenceId(u64::MAX))?;
    for animation in &moment.animations {
        match animation {
            RuntimeAnimationEvent::Move {
                object_id,
                from,
                to,
                visual_tween,
                name,
                ..
            } => {
                let target_cell = runtime_coord(*to);
                let instance = instances
                    .iter_mut()
                    .find(|instance| {
                        instance.object_id == Some(*object_id) && instance.cell == target_cell
                    })
                    .ok_or(PresentationError::MissingAnimationTarget {
                        object_id: *object_id,
                        cell: target_cell,
                    })?;
                let progress = animation_progress;
                let delta = [
                    f64::from(from.x) - f64::from(to.x),
                    f64::from(from.y) - f64::from(to.y),
                    f64::from(from.z.unwrap_or(0)) - f64::from(to.z.unwrap_or(0)),
                ];
                let mut movement = identity_affine3();
                if animation_names(name).contains("slide")
                    || animation_names(name).contains("tween")
                    || name.is_empty()
                {
                    movement[0][3] = delta[0] * (1.0 - progress);
                    movement[1][3] = delta[1] * (1.0 - progress);
                    movement[2][3] = delta[2] * (1.0 - progress);
                }
                let names = animation_names(name);
                if names.contains("zoom") {
                    let scale = 0.85 + 0.15 * progress;
                    movement = multiply_affine3(movement, scale_affine(scale));
                }
                if names.contains("fade") {
                    instance.opacity *= 0.35 + 0.65 * progress;
                }
                instance.transform = multiply_affine3(movement, instance.transform);
                if let Some(tween) = visual_tween {
                    let state = sample_visual_tween(tween, progress);
                    instance.transform =
                        multiply_affine3(movement, resolve_visual_affine(&state.transforms)?);
                    instance.opacity *= state.opacity.unwrap_or(1.0);
                }
            }
            RuntimeAnimationEvent::CantMove {
                object_id,
                position,
                name,
            } => {
                let cell = runtime_coord(*position);
                let instance = instances
                    .iter_mut()
                    .find(|instance| {
                        instance.object_id == Some(*object_id) && instance.cell == cell
                    })
                    .ok_or(PresentationError::MissingAnimationTarget {
                        object_id: *object_id,
                        cell,
                    })?;
                let progress = animation_progress;
                let names = animation_names(name);
                let mut effect = identity_affine3();
                if names.contains("slide") || names.contains("tween") || name.is_empty() {
                    effect[0][3] = (progress * std::f64::consts::TAU).sin() * 0.12;
                }
                if names.contains("zoom") {
                    effect = multiply_affine3(
                        effect,
                        scale_affine(1.0 + (progress * std::f64::consts::PI).sin() * 0.18),
                    );
                }
                if names.contains("fade") {
                    instance.opacity *= 1.0 - (progress * std::f64::consts::PI).sin() * 0.45;
                }
                instance.transform = multiply_affine3(effect, instance.transform);
            }
            RuntimeAnimationEvent::Animation {
                name,
                position,
                resolved_visual,
            } => {
                let resolved = resolved_visual.as_ref().ok_or_else(|| {
                    PresentationError::MissingResolvedAnimationVisual(name.clone())
                })?;
                let clip = clips
                    .get(name.as_str())
                    .copied()
                    .ok_or_else(|| PresentationError::UnknownVisual(name.clone()))?;
                let duration = clip
                    .frame_duration_ms
                    .and_then(|frame| frame.checked_mul(clip.frames.len() as u64))
                    .unwrap_or(250);
                if moment.animation_elapsed_ms >= duration {
                    continue;
                }
                let cell = runtime_coord(*position);
                let cell_order = scene
                    .cells
                    .iter()
                    .find(|candidate| candidate.position == cell)
                    .map(|candidate| candidate.render_order)
                    .ok_or(PresentationError::MissingRenderCell(cell))?;
                let render_order = cell_order
                    .saturating_mul(u64::from(scene.render_priority_count))
                    .saturating_add(u64::from(resolved.render_priority));
                instances.push(RuntimeResolvedRenderInstance {
                    id: next_id,
                    object_id: None,
                    visual: name.clone(),
                    cell,
                    transform: identity_affine3(),
                    opacity: 1.0,
                    frame_elapsed_ms: Some(moment.animation_elapsed_ms),
                    playback: RuntimeResolvedPlayback::Once,
                    render_order,
                });
                let group = groups.iter_mut().find(|group| {
                    group.render_order == render_order
                        && group.composition == resolved.composition
                        && group.instances.iter().any(|id| {
                            instances
                                .iter()
                                .find(|instance| instance.id == *id)
                                .is_some_and(|instance| instance.cell == cell)
                        })
                });
                if let Some(group) = group {
                    group.instances.push(next_id);
                } else {
                    groups.push(puzzle_runtime_contract::RuntimeResolvedCompositionGroup {
                        render_order,
                        composition: resolved.composition,
                        instances: vec![next_id],
                    });
                }
                next_id = next_id
                    .checked_add(1)
                    .ok_or(PresentationError::InvalidOccurrenceId(u64::MAX))?;
            }
        }
    }
    Ok((instances, groups))
}

fn runtime_coord(coord: puzzle_runtime_contract::RuntimeCoord) -> [i32; 3] {
    [
        i32::from(coord.x),
        i32::from(coord.y),
        i32::from(coord.z.unwrap_or(0)),
    ]
}

fn animation_names(name: &str) -> std::collections::HashSet<&str> {
    name.split(':')
        .filter_map(|part| part.split('=').next())
        .filter(|part| !part.is_empty())
        .collect()
}

fn scale_affine(scale: f64) -> [[f64; 4]; 4] {
    let mut matrix = identity_affine3();
    matrix[0][0] = scale;
    matrix[1][1] = scale;
    matrix[2][2] = scale;
    matrix
}

fn selected_frame<'a>(
    visual: &str,
    clips: &'a HashMap<&str, &'a puzzle_runtime_contract::RuntimeResolvedVisualClip>,
    elapsed_ms: u64,
    playback: RuntimeResolvedPlayback,
) -> Result<&'a RuntimeResolvedVisualFrame, PresentationError> {
    let clip = clips
        .get(visual)
        .copied()
        .ok_or_else(|| PresentationError::UnknownVisual(visual.to_string()))?;
    if clip.frames.is_empty() {
        return Err(PresentationError::EmptyVisualClip(clip.id.clone()));
    }
    let index = match clip.frame_duration_ms {
        None => 0,
        Some(0) => return Err(PresentationError::ZeroFrameDuration(clip.id.clone())),
        Some(duration) => match playback {
            RuntimeResolvedPlayback::Loop => {
                ((elapsed_ms / duration) % clip.frames.len() as u64) as usize
            }
            RuntimeResolvedPlayback::Once => {
                ((elapsed_ms / duration) as usize).min(clip.frames.len() - 1)
            }
        },
    };
    Ok(&clip.frames[index])
}

fn instance_batch(
    instance: &RuntimeResolvedRenderInstance,
    clips: &HashMap<&str, &puzzle_runtime_contract::RuntimeResolvedVisualClip>,
    assets: &DecodedVisualImageCatalog,
    elapsed_ms: u64,
) -> Result<RuntimeResolvedRenderBatch, PresentationError> {
    let clip = clips
        .get(instance.visual.as_str())
        .copied()
        .ok_or_else(|| PresentationError::UnknownVisual(instance.visual.clone()))?;
    let frame = selected_frame(
        &instance.visual,
        clips,
        instance.frame_elapsed_ms.unwrap_or(elapsed_ms),
        instance.playback,
    )?;
    Ok(RuntimeResolvedRenderBatch {
        render_order: instance.render_order,
        object_ids: instance.object_id.into_iter().collect(),
        cell: instance.cell,
        transform: instance.transform,
        opacity: instance.opacity,
        pixel_geometry: pixel_geometry(frame, clip),
        content: frame_content(frame, clip, assets)?,
    })
}

fn frame_content(
    frame: &RuntimeResolvedVisualFrame,
    clip: &RuntimeResolvedVisualClip,
    assets: &DecodedVisualImageCatalog,
) -> Result<RuntimeResolvedRenderBatchContent, PresentationError> {
    Ok(match frame {
        RuntimeResolvedVisualFrame::Pixels {
            width,
            height,
            pixels,
        } => RuntimeResolvedRenderBatchContent::Pixels {
            width: *width,
            height: *height,
            pixels: pixels.clone(),
        },
        RuntimeResolvedVisualFrame::Voxels {
            width,
            depth,
            height,
            voxels,
        } => RuntimeResolvedRenderBatchContent::Voxels {
            width: *width,
            depth: *depth,
            height: *height,
            voxels: voxels.clone(),
        },
        RuntimeResolvedVisualFrame::RasterImage { asset, sampling } => raster_content(
            asset,
            *sampling,
            clip,
            assets
                .get(asset)
                .ok_or_else(|| PresentationError::MissingImageAsset(asset.0.clone()))?,
        ),
    })
}

fn raster_content(
    asset_id: &puzzle_assets::VisualImageAssetId,
    sampling: RuntimeResolvedSampling,
    clip: &RuntimeResolvedVisualClip,
    asset: &DecodedVisualImageAsset,
) -> RuntimeResolvedRenderBatchContent {
    let fit = resolved_fit_geometry(asset.width, asset.height, clip);
    RuntimeResolvedRenderBatchContent::RasterImage {
        asset: asset_id.clone(),
        revision: asset.revision.clone(),
        source_size: [asset.width, asset.height],
        destination: fit.raster_destination,
        uv: fit.uv,
        sampling,
    }
}

fn pixel_geometry(
    frame: &RuntimeResolvedVisualFrame,
    clip: &RuntimeResolvedVisualClip,
) -> Option<RuntimeResolvedPixelGeometry> {
    let RuntimeResolvedVisualFrame::Pixels { width, height, .. } = frame else {
        return None;
    };
    let fit = resolved_fit_geometry(*width, *height, clip);
    Some(RuntimeResolvedPixelGeometry {
        x: fit.logical_content.x,
        y: fit.logical_content.y,
        width: fit.logical_content.width,
        height: fit.logical_content.height,
        clip: fit.clip,
    })
}

#[derive(Clone, Copy)]
struct ResolvedFitGeometry {
    logical_content: RuntimeResolvedRect2d,
    clip: Option<RuntimeResolvedRect2d>,
    raster_destination: RuntimeResolvedRect2d,
    uv: RuntimeResolvedRect2d,
}

fn resolved_fit_geometry(
    source_width: u16,
    source_height: u16,
    clip: &RuntimeResolvedVisualClip,
) -> ResolvedFitGeometry {
    let source_width = f64::from(source_width);
    let source_height = f64::from(source_height);
    let box_width = f64::from(clip.layout.width);
    let box_height = f64::from(clip.layout.height);
    let box_rect = RuntimeResolvedRect2d {
        x: (1.0 - box_width) / 2.0,
        y: (1.0 - box_height) / 2.0,
        width: box_width,
        height: box_height,
    };
    let scale_x = box_width / source_width;
    let scale_y = box_height / source_height;
    let scale = match clip.layout.fit {
        RuntimeResolvedFitMode::Contain => scale_x.min(scale_y),
        RuntimeResolvedFitMode::Cover => scale_x.max(scale_y),
        RuntimeResolvedFitMode::Stretch => 1.0,
    };
    let width = if clip.layout.fit == RuntimeResolvedFitMode::Stretch {
        box_width
    } else {
        source_width * scale
    };
    let height = if clip.layout.fit == RuntimeResolvedFitMode::Stretch {
        box_height
    } else {
        source_height * scale
    };
    let logical_content = RuntimeResolvedRect2d {
        x: (1.0 - box_width) / 2.0 + (box_width - width) / 2.0,
        y: (1.0 - box_height) / 2.0 + (box_height - height) / 2.0,
        width,
        height,
    };
    let (raster_destination, uv) = if clip.layout.fit == RuntimeResolvedFitMode::Cover {
        (
            box_rect,
            RuntimeResolvedRect2d {
                x: (1.0 - box_width / width) / 2.0,
                y: (1.0 - box_height / height) / 2.0,
                width: box_width / width,
                height: box_height / height,
            },
        )
    } else {
        (
            logical_content,
            RuntimeResolvedRect2d {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        )
    };
    ResolvedFitGeometry {
        logical_content,
        clip: (clip.layout.fit == RuntimeResolvedFitMode::Cover).then_some(box_rect),
        raster_destination,
        uv,
    }
}

fn average_batch(
    members: &[&RuntimeResolvedRenderInstance],
    clips: &HashMap<&str, &puzzle_runtime_contract::RuntimeResolvedVisualClip>,
    render_order: u64,
    elapsed_ms: u64,
) -> Result<RuntimeResolvedRenderBatch, PresentationError> {
    let first = members
        .first()
        .copied()
        .ok_or(PresentationError::IncompatibleCompositionFrames)?;
    if members.iter().any(|member| member.cell != first.cell) {
        return Err(PresentationError::MixedCompositionCells);
    }
    let frames = members
        .iter()
        .map(|member| {
            selected_frame(
                &member.visual,
                clips,
                member.frame_elapsed_ms.unwrap_or(elapsed_ms),
                member.playback,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first_clip = clips
        .get(first.visual.as_str())
        .copied()
        .ok_or_else(|| PresentationError::UnknownVisual(first.visual.clone()))?;
    if members.iter().any(|member| {
        clips.get(member.visual.as_str()).map_or(true, |clip| {
            clip.layout.fit != first_clip.layout.fit
                || clip.layout.width != first_clip.layout.width
                || clip.layout.height != first_clip.layout.height
        })
    }) {
        return Err(PresentationError::IncompatibleCompositionFrames);
    }
    let geometry = pixel_geometry(frames[0], first_clip);
    let content = average_frames(&frames, members, geometry)?;
    Ok(RuntimeResolvedRenderBatch {
        render_order,
        object_ids: members
            .iter()
            .filter_map(|member| member.object_id)
            .collect(),
        cell: first.cell,
        transform: identity_affine3(),
        opacity: 1.0,
        pixel_geometry: geometry,
        content,
    })
}

fn average_frames(
    frames: &[&RuntimeResolvedVisualFrame],
    instances: &[&RuntimeResolvedRenderInstance],
    pixel_geometry: Option<RuntimeResolvedPixelGeometry>,
) -> Result<RuntimeResolvedRenderBatchContent, PresentationError> {
    match frames.first().copied() {
        Some(RuntimeResolvedVisualFrame::Pixels { width, height, .. }) => {
            let mut colors = BTreeMap::<[i32; 2], Vec<RuntimeLinearRgba>>::new();
            for (frame, instance) in frames.iter().zip(instances) {
                let RuntimeResolvedVisualFrame::Pixels {
                    width: candidate_width,
                    height: candidate_height,
                    pixels,
                } = frame
                else {
                    return Err(PresentationError::IncompatibleCompositionFrames);
                };
                if candidate_width != width || candidate_height != height {
                    return Err(PresentationError::IncompatibleCompositionFrames);
                }
                let geometry =
                    pixel_geometry.ok_or(PresentationError::IncompatibleCompositionFrames)?;
                let sample_scale = [
                    geometry.width / f64::from(*width),
                    geometry.height / f64::from(*height),
                ];
                if !is_pixel_lattice_isometry(instance.transform, sample_scale) {
                    return Err(PresentationError::NonLatticeCompositionTransform);
                }
                for pixel in pixels {
                    let position = transform_pixel_position(
                        pixel.position,
                        [*width, *height],
                        sample_scale,
                        instance.transform,
                    )?;
                    let mut color = pixel.color;
                    color.alpha *= instance.opacity;
                    colors.entry(position).or_default().push(color);
                }
            }
            Ok(RuntimeResolvedRenderBatchContent::Pixels {
                width: *width,
                height: *height,
                pixels: colors
                    .into_iter()
                    .map(
                        |(position, colors)| puzzle_runtime_contract::RuntimeResolvedPixel {
                            position,
                            color: average_colors(&colors),
                        },
                    )
                    .collect(),
            })
        }
        Some(RuntimeResolvedVisualFrame::Voxels {
            width,
            depth,
            height,
            ..
        }) => {
            let mut colors = BTreeMap::<[i32; 3], Vec<RuntimeLinearRgba>>::new();
            for (frame, instance) in frames.iter().zip(instances) {
                let RuntimeResolvedVisualFrame::Voxels {
                    width: candidate_width,
                    depth: candidate_depth,
                    height: candidate_height,
                    voxels,
                } = frame
                else {
                    return Err(PresentationError::IncompatibleCompositionFrames);
                };
                if candidate_width != width
                    || candidate_depth != depth
                    || candidate_height != height
                {
                    return Err(PresentationError::IncompatibleCompositionFrames);
                }
                let sample_scale = 1.0 / f64::from((*width).max(*depth).max(*height));
                if !is_voxel_lattice_isometry(instance.transform, sample_scale) {
                    return Err(PresentationError::NonLatticeCompositionTransform);
                }
                for voxel in voxels {
                    let position = transform_voxel_position(
                        voxel.position,
                        [*width, *depth, *height],
                        sample_scale,
                        instance.transform,
                    )?;
                    let mut color = voxel.color;
                    color.alpha *= instance.opacity;
                    colors.entry(position).or_default().push(color);
                }
            }
            Ok(RuntimeResolvedRenderBatchContent::Voxels {
                width: *width,
                depth: *depth,
                height: *height,
                voxels: colors
                    .into_iter()
                    .map(
                        |(position, colors)| puzzle_runtime_contract::RuntimeResolvedVoxel {
                            position,
                            color: average_colors(&colors),
                        },
                    )
                    .collect(),
            })
        }
        Some(RuntimeResolvedVisualFrame::RasterImage { .. }) => {
            Err(PresentationError::RasterImageComposition)
        }
        None => Err(PresentationError::IncompatibleCompositionFrames),
    }
}

fn is_pixel_lattice_isometry(transform: [[f64; 4]; 4], scale: [f64; 2]) -> bool {
    is_signed_permutation(&[
        [transform[0][0], transform[0][1] * scale[1] / scale[0]],
        [transform[1][0] * scale[0] / scale[1], transform[1][1]],
    ]) && lattice_value(transform[0][3] / scale[0]).is_some()
        && lattice_value(transform[1][3] / scale[1]).is_some()
        && approximately(transform[2][3], 0.0)
        && approximately(transform[2][0], 0.0)
        && approximately(transform[2][1], 0.0)
        && approximately(transform[0][2], 0.0)
        && approximately(transform[1][2], 0.0)
        && approximately(transform[2][2], 1.0)
}

fn is_voxel_lattice_isometry(transform: [[f64; 4]; 4], scale: f64) -> bool {
    is_signed_permutation(&[
        [transform[0][0], transform[0][1], transform[0][2]],
        [transform[1][0], transform[1][1], transform[1][2]],
        [transform[2][0], transform[2][1], transform[2][2]],
    ]) && (0..3).all(|axis| lattice_value(transform[axis][3] / scale).is_some())
}

fn is_signed_permutation<const D: usize>(matrix: &[[f64; D]; D]) -> bool {
    (0..D).all(|row| {
        (0..D)
            .filter(|column| approximately(matrix[row][*column].abs(), 1.0))
            .count()
            == 1
            && (0..D).all(|column| {
                approximately(matrix[row][column], 0.0)
                    || approximately(matrix[row][column].abs(), 1.0)
            })
    }) && (0..D).all(|column| {
        (0..D)
            .filter(|row| approximately(matrix[*row][column].abs(), 1.0))
            .count()
            == 1
    })
}

fn transform_pixel_position(
    position: [i32; 2],
    size: [u16; 2],
    scale: [f64; 2],
    transform: [[f64; 4]; 4],
) -> Result<[i32; 2], PresentationError> {
    let centered = [
        (f64::from(position[0]) + 0.5 - f64::from(size[0]) / 2.0) * scale[0],
        (f64::from(position[1]) + 0.5 - f64::from(size[1]) / 2.0) * scale[1],
        0.0,
        1.0,
    ];
    let transformed = multiply_point(transform, centered);
    if !approximately(transformed[2], 0.0) {
        return Err(PresentationError::NonLatticeCompositionTransform);
    }
    lattice_position([
        transformed[0] / scale[0] + f64::from(size[0]) / 2.0 - 0.5,
        transformed[1] / scale[1] + f64::from(size[1]) / 2.0 - 0.5,
    ])
}

fn transform_voxel_position(
    position: [i32; 3],
    size: [u16; 3],
    scale: f64,
    transform: [[f64; 4]; 4],
) -> Result<[i32; 3], PresentationError> {
    let centered = [
        (f64::from(position[0]) + 0.5 - f64::from(size[0]) / 2.0) * scale,
        (f64::from(position[1]) + 0.5 - f64::from(size[1]) / 2.0) * scale,
        (f64::from(position[2]) + 0.5 - f64::from(size[2]) / 2.0) * scale,
        1.0,
    ];
    let transformed = multiply_point(transform, centered);
    lattice_position([
        transformed[0] / scale + f64::from(size[0]) / 2.0 - 0.5,
        transformed[1] / scale + f64::from(size[1]) / 2.0 - 0.5,
        transformed[2] / scale + f64::from(size[2]) / 2.0 - 0.5,
    ])
}

fn multiply_point(transform: [[f64; 4]; 4], point: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|row| {
        (0..4)
            .map(|column| transform[row][column] * point[column])
            .sum()
    })
}

fn lattice_value(value: f64) -> Option<i32> {
    let rounded = value.round();
    (approximately(value, rounded)
        && rounded >= f64::from(i32::MIN)
        && rounded <= f64::from(i32::MAX))
    .then_some(rounded as i32)
}

fn lattice_position<const D: usize>(position: [f64; D]) -> Result<[i32; D], PresentationError> {
    let mut resolved = [0; D];
    for (index, value) in position.into_iter().enumerate() {
        let rounded = value.round();
        if !approximately(value, rounded)
            || rounded < f64::from(i32::MIN)
            || rounded > f64::from(i32::MAX)
        {
            return Err(PresentationError::NonLatticeCompositionTransform);
        }
        resolved[index] = rounded as i32;
    }
    Ok(resolved)
}

fn approximately(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.000000001
}

fn average_colors(colors: &[RuntimeLinearRgba]) -> RuntimeLinearRgba {
    let count = colors.len() as f64;
    RuntimeLinearRgba {
        red: colors.iter().map(|color| color.red).sum::<f64>() / count,
        green: colors.iter().map(|color| color.green).sum::<f64>() / count,
        blue: colors.iter().map(|color| color.blue).sum::<f64>() / count,
        alpha: colors.iter().map(|color| color.alpha).sum::<f64>() / count,
    }
}

pub fn resolve_spatial_affine(
    operations: &[RuntimePuzzle3SpatialOp],
) -> Result<[[f64; 4]; 4], PresentationError> {
    let mut result = identity_affine3();
    for operation in operations {
        let (space, matrix) = match operation {
            RuntimePuzzle3SpatialOp::Translate { space, value } => {
                let [x, y, z] = finite_vector(*value, "translation")?;
                let mut matrix = identity_affine3();
                matrix[0][3] = x;
                matrix[1][3] = y;
                matrix[2][3] = z;
                (*space, matrix)
            }
            RuntimePuzzle3SpatialOp::Rotate {
                space,
                axis,
                degrees,
            } => (*space, rotation_affine3(*axis, *degrees)?),
            RuntimePuzzle3SpatialOp::Flip { enabled: false } => continue,
            RuntimePuzzle3SpatialOp::Flip { enabled: true } => {
                let mut matrix = identity_affine3();
                matrix[0][0] = -1.0;
                (RuntimePuzzle3VisualSpace::Local, matrix)
            }
        };
        result = match space {
            RuntimePuzzle3VisualSpace::World => multiply_affine3(matrix, result),
            RuntimePuzzle3VisualSpace::Local => multiply_affine3(result, matrix),
        };
    }
    Ok(result)
}

pub fn resolve_visual_affine(
    operations: &[RuntimeVisualTransform],
) -> Result<[[f64; 4]; 4], PresentationError> {
    let mut result = identity_affine3();
    for operation in operations {
        let (space, matrix) = match operation {
            RuntimeVisualTransform::Translate { value, space } => {
                let [x, y, z] = finite_vector(*value, "translation")?;
                let mut matrix = identity_affine3();
                matrix[0][3] = x;
                matrix[1][3] = y;
                matrix[2][3] = z;
                (*space, matrix)
            }
            RuntimeVisualTransform::Rotate {
                degrees,
                axis,
                space,
            } => (*space, rotation_affine3(*axis, *degrees)?),
            RuntimeVisualTransform::Scale { value, space } => {
                let [x, y, z] = finite_vector(*value, "scale")?;
                let mut matrix = identity_affine3();
                matrix[0][0] = x;
                matrix[1][1] = y;
                matrix[2][2] = z;
                (*space, matrix)
            }
            RuntimeVisualTransform::Flip { enabled: false } => continue,
            RuntimeVisualTransform::Flip { enabled: true } => {
                let mut matrix = identity_affine3();
                matrix[0][0] = -1.0;
                (RuntimeVisualSpace::Local, matrix)
            }
        };
        result = match space {
            RuntimeVisualSpace::World => multiply_affine3(matrix, result),
            RuntimeVisualSpace::Local => multiply_affine3(result, matrix),
        };
    }
    Ok(result)
}

fn identity_affine3() -> [[f64; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn rotation_affine3(axis: [f64; 3], degrees: f64) -> Result<[[f64; 4]; 4], PresentationError> {
    let [x, y, z] = finite_vector(axis, "rotation axis")?;
    let length = x.hypot(y).hypot(z);
    if length == 0.0 {
        return Err(PresentationError::ZeroRotationAxis);
    }
    let [x, y, z] = [x / length, y / length, z / length];
    let radians = finite(degrees, "rotation")?.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    let complement = 1.0 - cosine;
    Ok([
        [
            complement * x * x + cosine,
            complement * x * y - sine * z,
            complement * x * z + sine * y,
            0.0,
        ],
        [
            complement * x * y + sine * z,
            complement * y * y + cosine,
            complement * y * z - sine * x,
            0.0,
        ],
        [
            complement * x * z - sine * y,
            complement * y * z + sine * x,
            complement * z * z + cosine,
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

fn multiply_affine3(left: [[f64; 4]; 4], right: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..4)
                .map(|index| left[row][index] * right[index][column])
                .sum()
        })
    })
}

pub fn resolve_object_priority(
    order: &VisualOrderRef<'_>,
    object: &str,
) -> Result<ResolvedVisualPriority, PresentationError> {
    resolve_priority(order, |entry| {
        entry.objects.iter().any(|name| name == object)
    })
    .ok_or_else(|| PresentationError::UnknownObject(object.to_string()))
}

pub fn resolve_animation_priority(
    order: &VisualOrderRef<'_>,
    animation: &str,
) -> Result<ResolvedVisualPriority, PresentationError> {
    resolve_priority(order, |entry| {
        entry.animations.iter().any(|name| name == animation)
    })
    .ok_or_else(|| PresentationError::UnknownAnimation(animation.to_string()))
}

fn resolve_priority(
    order: &VisualOrderRef<'_>,
    matches: impl Fn(&VisualPriorityRef<'_>) -> bool,
) -> Option<ResolvedVisualPriority> {
    order
        .priorities
        .iter()
        .enumerate()
        .find(|(_, entry)| matches(entry))
        .map(|(index, entry)| ResolvedVisualPriority {
            index,
            composition: if entry.merge {
                VisualComposition::Average
            } else {
                VisualComposition::Ordered
            },
        })
}

pub fn cell_render_order_2d(
    order: &VisualOrderRef<'_>,
    width: u16,
    height: u16,
    x: u16,
    y: u16,
) -> Result<u64, PresentationError> {
    cell_render_order(
        order,
        &[
            ("right", u64::from(x), u64::from(width)),
            ("left", u64::from(width - 1 - x), u64::from(width)),
            ("down", u64::from(y), u64::from(height)),
            ("up", u64::from(height - 1 - y), u64::from(height)),
        ],
    )
}

pub fn cell_render_order_3d(
    order: &VisualOrderRef<'_>,
    size: [u16; 3],
    position: [u16; 3],
) -> Result<u64, PresentationError> {
    let [width, depth, height] = size;
    let [x, y, z] = position;
    cell_render_order(
        order,
        &[
            ("right", u64::from(x), u64::from(width)),
            ("left", u64::from(width - 1 - x), u64::from(width)),
            ("front", u64::from(y), u64::from(depth)),
            ("back", u64::from(depth - 1 - y), u64::from(depth)),
            ("up", u64::from(z), u64::from(height)),
            ("down", u64::from(height - 1 - z), u64::from(height)),
        ],
    )
}

fn cell_render_order(
    order: &VisualOrderRef<'_>,
    coordinates: &[(&str, u64, u64)],
) -> Result<u64, PresentationError> {
    if order.direction_priority.is_empty() || order.priorities.is_empty() {
        return Err(PresentationError::MissingVisualOrder);
    }
    order
        .direction_priority
        .iter()
        .try_fold(0u64, |index, direction| {
            let (_, value, span) = coordinates
                .iter()
                .find(|(candidate, _, _)| candidate == direction)
                .ok_or_else(|| PresentationError::InvalidDirection(direction.clone()))?;
            Ok(index.saturating_mul(*span).saturating_add(*value))
        })
}

pub fn resolve_animation_channels(
    events: &[RuntimeAnimationEvent],
) -> Result<Vec<RuntimeAnimationEvent>, PresentationError> {
    enum OrderedEntry {
        Event(RuntimeAnimationEvent),
        Occurrence(u64),
    }

    let mut occurrences = HashMap::<u64, Vec<RuntimeAnimationEvent>>::new();
    let mut ordered = Vec::<OrderedEntry>::new();
    for event in events {
        let RuntimeAnimationEvent::Move {
            name,
            occurrence_id,
            ..
        } = event
        else {
            ordered.push(OrderedEntry::Event(event.clone()));
            continue;
        };
        if name != "tween" {
            ordered.push(OrderedEntry::Event(event.clone()));
            continue;
        }
        if *occurrence_id == 0 {
            return Err(PresentationError::InvalidOccurrenceId(*occurrence_id));
        }
        let entry = occurrences.entry(*occurrence_id).or_default();
        if entry.is_empty() {
            ordered.push(OrderedEntry::Occurrence(*occurrence_id));
        }
        entry.push(event.clone());
    }

    ordered
        .into_iter()
        .map(|entry| match entry {
            OrderedEntry::Event(event) => Ok(event),
            OrderedEntry::Occurrence(id) => compose_occurrence(
                occurrences
                    .remove(&id)
                    .expect("ordered animation occurrence must exist"),
            ),
        })
        .collect()
}

pub fn resolve_presentation_events(
    mut events: Vec<RuntimePresentationEvent>,
    order: &VisualOrderRef<'_>,
) -> Result<Vec<RuntimePresentationEvent>, PresentationError> {
    for event in &mut events {
        if let RuntimePresentationEvent::AnimationBatch { animations, .. } = event {
            *animations = resolve_animation_channels(animations)?;
            for animation in animations {
                if let RuntimeAnimationEvent::Animation {
                    name,
                    resolved_visual,
                    ..
                } = animation
                {
                    let resolved = resolve_animation_priority(order, name)?;
                    *resolved_visual = Some(RuntimeResolvedVisualOrder {
                        render_priority: u16::try_from(resolved.index)
                            .expect("validated visual priority must fit u16"),
                        composition: match resolved.composition {
                            VisualComposition::Ordered => RuntimeVisualComposition::Ordered,
                            VisualComposition::Average => RuntimeVisualComposition::Average,
                        },
                    });
                }
            }
        }
    }
    Ok(events)
}

fn compose_occurrence(
    events: Vec<RuntimeAnimationEvent>,
) -> Result<RuntimeAnimationEvent, PresentationError> {
    let mut position_from = None;
    let mut position_to = None;
    let mut visual_tweens = Vec::new();
    let mut final_event = None;

    for event in &events {
        let RuntimeAnimationEvent::Move {
            from,
            to,
            visual_tween,
            ..
        } = event
        else {
            unreachable!("animation occurrence contains only tween move events");
        };
        if from != to {
            position_from.get_or_insert_with(|| from.clone());
            position_to = Some(to.clone());
        }
        if let Some(tween) = visual_tween {
            visual_tweens.push(tween.clone());
        }
        final_event = Some(event.clone());
    }

    let RuntimeAnimationEvent::Move {
        name,
        occurrence_id,
        object_id,
        from,
        to,
        ..
    } = final_event.expect("animation occurrence must contain at least one event")
    else {
        unreachable!();
    };
    Ok(RuntimeAnimationEvent::Move {
        name,
        occurrence_id,
        object_id,
        from: position_from.unwrap_or(from),
        to: position_to.unwrap_or(to),
        visual_tween: compose_visual_tweens(&visual_tweens)?,
    })
}

fn compose_visual_tweens(
    tweens: &[RuntimeVisualTween],
) -> Result<Option<RuntimeVisualTween>, PresentationError> {
    let Some(first) = tweens.first() else {
        return Ok(None);
    };
    let last = tweens
        .last()
        .expect("non-empty visual tween sequence must have a last item");
    prepare_visual_tween(
        &sample_visual_tween(first, 0.0),
        &sample_visual_tween(last, 1.0),
    )
    .map(Some)
}

pub fn prepare_visual_tween(
    from: &RuntimeVisualState,
    to: &RuntimeVisualState,
) -> Result<RuntimeVisualTween, PresentationError> {
    if from.transforms.len() != to.transforms.len() {
        return Err(PresentationError::IncompatibleTransformCount);
    }
    let transforms = from
        .transforms
        .iter()
        .zip(&to.transforms)
        .enumerate()
        .map(|(index, (from, to))| prepare_transform(from, to, index))
        .collect::<Result<Vec<_>, _>>()?;
    let opacity = match (from.opacity, to.opacity) {
        (None, None) => None,
        (Some(from), Some(to)) => Some(prepare_scalar(from, to, "opacity")?),
        _ => return Err(PresentationError::IncompatibleOpacity),
    };
    Ok(RuntimeVisualTween {
        transforms,
        opacity,
    })
}

fn prepare_transform(
    from: &RuntimeVisualTransform,
    to: &RuntimeVisualTransform,
    index: usize,
) -> Result<RuntimeVisualTweenTransform, PresentationError> {
    match (from, to) {
        (
            RuntimeVisualTransform::Rotate {
                degrees: from_degrees,
                axis: from_axis,
                space: from_space,
            },
            RuntimeVisualTransform::Rotate {
                degrees: to_degrees,
                axis: to_axis,
                space: to_space,
            },
        ) if from_axis == to_axis && from_space == to_space => {
            Ok(RuntimeVisualTweenTransform::Rotate {
                start_degrees: finite(*from_degrees, "rotation")?,
                delta_degrees: shortest_angle_delta(*from_degrees, *to_degrees)?,
                axis: *from_axis,
                space: *from_space,
            })
        }
        (
            RuntimeVisualTransform::Translate {
                value: from_value,
                space: from_space,
            },
            RuntimeVisualTransform::Translate {
                value: to_value,
                space: to_space,
            },
        ) if from_space == to_space => Ok(RuntimeVisualTweenTransform::Translate {
            start: finite_vector(*from_value, "translation")?,
            delta: vector_delta(*from_value, *to_value, "translation")?,
            space: *from_space,
        }),
        (
            RuntimeVisualTransform::Scale {
                value: from_value,
                space: from_space,
            },
            RuntimeVisualTransform::Scale {
                value: to_value,
                space: to_space,
            },
        ) if from_space == to_space => Ok(RuntimeVisualTweenTransform::Scale {
            start: finite_vector(*from_value, "scale")?,
            delta: vector_delta(*from_value, *to_value, "scale")?,
            space: *from_space,
        }),
        (
            RuntimeVisualTransform::Flip {
                enabled: from_enabled,
            },
            RuntimeVisualTransform::Flip {
                enabled: to_enabled,
            },
        ) => Ok(RuntimeVisualTweenTransform::Scale {
            start: [if *from_enabled { -1.0 } else { 1.0 }, 1.0, 1.0],
            delta: [
                (if *to_enabled { -1.0 } else { 1.0 }) - (if *from_enabled { -1.0 } else { 1.0 }),
                0.0,
                0.0,
            ],
            space: RuntimeVisualSpace::Local,
        }),
        _ => Err(PresentationError::IncompatibleTransform {
            index,
            reason: "kind, axis, or space differs",
        }),
    }
}

pub fn sample_visual_tween(prepared: &RuntimeVisualTween, progress: f64) -> RuntimeVisualState {
    let amount = progress.clamp(0.0, 1.0);
    RuntimeVisualState {
        transforms: prepared
            .transforms
            .iter()
            .map(|transform| match transform {
                RuntimeVisualTweenTransform::Rotate {
                    start_degrees,
                    delta_degrees,
                    axis,
                    space,
                } => RuntimeVisualTransform::Rotate {
                    degrees: start_degrees + delta_degrees * amount,
                    axis: *axis,
                    space: *space,
                },
                RuntimeVisualTweenTransform::Translate {
                    start,
                    delta,
                    space,
                } => RuntimeVisualTransform::Translate {
                    value: sample_vector(*start, *delta, amount),
                    space: *space,
                },
                RuntimeVisualTweenTransform::Scale {
                    start,
                    delta,
                    space,
                } => RuntimeVisualTransform::Scale {
                    value: sample_vector(*start, *delta, amount),
                    space: *space,
                },
            })
            .collect(),
        opacity: prepared
            .opacity
            .map(|channel| channel.start + channel.delta * amount),
    }
}

fn prepare_scalar(
    from: f64,
    to: f64,
    label: &'static str,
) -> Result<RuntimeScalarTween, PresentationError> {
    let from = finite(from, label)?;
    let to = finite(to, label)?;
    Ok(RuntimeScalarTween {
        start: from,
        delta: to - from,
    })
}

fn shortest_angle_delta(from: f64, to: f64) -> Result<f64, PresentationError> {
    let from = finite(from, "rotation")?;
    let to = finite(to, "rotation")?;
    let mut delta = (to - from + 180.0).rem_euclid(360.0) - 180.0;
    if delta == -180.0 {
        delta = 180.0;
    }
    Ok(delta)
}

fn vector_delta(
    from: [f64; 3],
    to: [f64; 3],
    label: &'static str,
) -> Result<[f64; 3], PresentationError> {
    let from = finite_vector(from, label)?;
    let to = finite_vector(to, label)?;
    Ok(std::array::from_fn(|index| to[index] - from[index]))
}

fn finite_vector(value: [f64; 3], label: &'static str) -> Result<[f64; 3], PresentationError> {
    for component in value {
        finite(component, label)?;
    }
    Ok(value)
}

fn finite(value: f64, label: &'static str) -> Result<f64, PresentationError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or(PresentationError::NonFiniteValue { label })
}

fn sample_vector(start: [f64; 3], delta: [f64; 3], amount: f64) -> [f64; 3] {
    std::array::from_fn(|index| start[index] + delta[index] * amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, codecs::png::PngEncoder};
    use puzzle_assets::{
        EncodedVisualImageAsset, EncodedVisualImageBundle, VisualImageAssetManifestEntry,
        decode_visual_image_bundle,
    };
    use puzzle_runtime_contract::{
        RuntimeCoord, RuntimeResolvedCompositionGroup, RuntimeResolvedPixel,
        RuntimeResolvedSampling, RuntimeResolvedVisualClip, RuntimeResolvedVisualLayout,
    };

    fn test_layout() -> RuntimeResolvedVisualLayout {
        RuntimeResolvedVisualLayout {
            fit: RuntimeResolvedFitMode::Contain,
            width: 1,
            height: 1,
        }
    }

    fn decoded_png_catalog(
        path: &str,
        pixels: &[[u8; 4]],
        width: u32,
        height: u32,
    ) -> DecodedVisualImageCatalog {
        let raw = pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect::<Vec<_>>();
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&raw, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        let manifest = VisualImageAssetManifestEntry::from_path(path).unwrap();
        let asset = EncodedVisualImageAsset::new(manifest, encoded).unwrap();
        decode_visual_image_bundle(&EncodedVisualImageBundle {
            assets: vec![asset],
        })
        .unwrap()
    }

    fn order_parts() -> (Vec<String>, Vec<(Vec<String>, Vec<String>, bool)>) {
        (
            vec!["down".to_string(), "right".to_string()],
            vec![
                (vec!["Floor".to_string()], Vec::new(), false),
                (
                    vec!["Red".to_string(), "Blue".to_string()],
                    vec!["flash".to_string()],
                    true,
                ),
            ],
        )
    }

    fn order_ref<'a>(
        directions: &'a [String],
        priorities: &'a [(Vec<String>, Vec<String>, bool)],
    ) -> VisualOrderRef<'a> {
        VisualOrderRef {
            direction_priority: directions,
            priorities: priorities
                .iter()
                .map(|(objects, animations, merge)| VisualPriorityRef {
                    objects,
                    animations,
                    merge: *merge,
                })
                .collect(),
        }
    }

    #[test]
    fn resolves_priority_composition_and_cell_order_without_renderer_knowledge() {
        let (directions, priorities) = order_parts();
        let order = order_ref(&directions, &priorities);
        assert_eq!(
            resolve_object_priority(&order, "Blue"),
            Ok(ResolvedVisualPriority {
                index: 1,
                composition: VisualComposition::Average,
            })
        );
        assert_eq!(cell_render_order_2d(&order, 4, 3, 2, 1), Ok(6));
    }

    #[test]
    fn voxel_frames_resolve_authored_rows_and_layers_to_canonical_xyz() {
        let palette = BTreeMap::from([
            ("a".to_string(), "#ff0000".to_string()),
            ("b".to_string(), "#00ff00".to_string()),
            ("c".to_string(), "#0000ff".to_string()),
        ]);
        let frame = resolve_voxel_frame(
            &[
                vec!["a.".to_string(), ".b".to_string()],
                vec!["c.".to_string(), "..".to_string()],
            ],
            &palette,
        )
        .unwrap();
        let RuntimeResolvedVisualFrame::Voxels {
            width,
            depth,
            height,
            voxels,
        } = frame
        else {
            panic!("3D authored frames must resolve to voxels");
        };

        assert_eq!([width, depth, height], [2, 2, 2]);
        assert_eq!(
            voxels
                .into_iter()
                .map(|voxel| voxel.position)
                .collect::<Vec<_>>(),
            vec![[0, 1, 1], [1, 0, 1], [0, 1, 0]]
        );
    }

    #[test]
    fn spatial_affine_owns_world_and_local_application_order() {
        let translation = RuntimePuzzle3SpatialOp::Translate {
            space: RuntimePuzzle3VisualSpace::Local,
            value: [1.0, 0.0, 0.0],
        };
        let rotation = |space| RuntimePuzzle3SpatialOp::Rotate {
            space,
            axis: [0.0, 0.0, 1.0],
            degrees: 90.0,
        };
        let world = resolve_spatial_affine(&[
            translation.clone(),
            rotation(RuntimePuzzle3VisualSpace::World),
        ])
        .unwrap();
        let local =
            resolve_spatial_affine(&[translation, rotation(RuntimePuzzle3VisualSpace::Local)])
                .unwrap();

        assert!(world[0][3].abs() < 0.000000001);
        assert!((world[1][3] - 1.0).abs() < 0.000000001);
        assert!((local[0][3] - 1.0).abs() < 0.000000001);
        assert!(local[1][3].abs() < 0.000000001);
    }

    #[test]
    fn occurrence_resolution_composes_position_and_visual_channels_once() {
        let move_event = |from: [u16; 2], to: [u16; 2], visual_tween| RuntimeAnimationEvent::Move {
            name: "tween".to_string(),
            occurrence_id: 7,
            object_id: 2,
            visual_tween,
            from: RuntimeCoord {
                x: from[0],
                y: from[1],
                z: None,
            },
            to: RuntimeCoord {
                x: to[0],
                y: to[1],
                z: None,
            },
        };
        let tween = prepare_visual_tween(
            &RuntimeVisualState {
                transforms: vec![RuntimeVisualTransform::Flip { enabled: false }],
                opacity: None,
            },
            &RuntimeVisualState {
                transforms: vec![RuntimeVisualTransform::Flip { enabled: true }],
                opacity: None,
            },
        )
        .unwrap();
        let resolved = resolve_animation_channels(&[
            move_event([0, 0], [1, 0], None),
            move_event([1, 0], [1, 0], Some(tween.clone())),
        ])
        .unwrap();
        assert_eq!(resolved, vec![move_event([0, 0], [1, 0], Some(tween))]);
    }

    #[test]
    fn prepared_tween_owns_shortest_rotation_and_flip_expansion() {
        let from = RuntimeVisualState {
            transforms: vec![
                RuntimeVisualTransform::Rotate {
                    degrees: 350.0,
                    axis: [0.0, 0.0, 1.0],
                    space: RuntimeVisualSpace::Local,
                },
                RuntimeVisualTransform::Flip { enabled: false },
            ],
            opacity: Some(0.25),
        };
        let to = RuntimeVisualState {
            transforms: vec![
                RuntimeVisualTransform::Rotate {
                    degrees: 10.0,
                    axis: [0.0, 0.0, 1.0],
                    space: RuntimeVisualSpace::Local,
                },
                RuntimeVisualTransform::Flip { enabled: true },
            ],
            opacity: Some(0.75),
        };
        let prepared = prepare_visual_tween(&from, &to).unwrap();
        assert_eq!(
            sample_visual_tween(&prepared, 0.5),
            RuntimeVisualState {
                transforms: vec![
                    RuntimeVisualTransform::Rotate {
                        degrees: 360.0,
                        axis: [0.0, 0.0, 1.0],
                        space: RuntimeVisualSpace::Local,
                    },
                    RuntimeVisualTransform::Scale {
                        value: [0.0, 1.0, 1.0],
                        space: RuntimeVisualSpace::Local,
                    },
                ],
                opacity: Some(0.5),
            }
        );
    }

    #[test]
    fn palette_rows_become_sparse_linear_pixels() {
        let palette = BTreeMap::from([
            ("r".to_string(), "#ff0000".to_string()),
            ("h".to_string(), "#80808080".to_string()),
        ]);

        let frame = resolve_pixel_frame(&["r.h".to_string()], &palette).unwrap();

        let RuntimeResolvedVisualFrame::Pixels {
            width,
            height,
            pixels,
        } = frame
        else {
            panic!("pixel rows must resolve to a pixel frame");
        };
        assert_eq!([width, height], [3, 1]);
        assert_eq!(pixels.len(), 2);
        assert_eq!(pixels[0].position, [0, 0]);
        assert_eq!(pixels[0].color.red, 1.0);
        assert!((pixels[1].color.red - 0.21586050011389926).abs() < 0.000000000001);
        assert!((pixels[1].color.alpha - 128.0 / 255.0).abs() < 0.000000000001);
    }

    #[test]
    fn render_frame_selects_animation_then_averages_in_linear_space() {
        let pixel_frame = |color| RuntimeResolvedVisualFrame::Pixels {
            width: 1,
            height: 1,
            pixels: vec![RuntimeResolvedPixel {
                position: [0, 0],
                color,
            }],
        };
        let red = RuntimeLinearRgba {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        };
        let green = RuntimeLinearRgba {
            red: 0.0,
            green: 1.0,
            blue: 0.0,
            alpha: 1.0,
        };
        let blue = RuntimeLinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 1.0,
            alpha: 1.0,
        };
        let scene = RuntimeResolvedRenderScene {
            clips: vec![
                RuntimeResolvedVisualClip {
                    id: "animated".to_string(),
                    frames: vec![pixel_frame(red), pixel_frame(green)],
                    frame_duration_ms: Some(10),
                    layout: test_layout(),
                },
                RuntimeResolvedVisualClip {
                    id: "blue".to_string(),
                    frames: vec![pixel_frame(blue)],
                    frame_duration_ms: None,
                    layout: test_layout(),
                },
            ],
            instances: vec![
                RuntimeResolvedRenderInstance {
                    id: 1,
                    object_id: None,
                    visual: "animated".to_string(),
                    cell: [4, 2, 0],
                    transform: identity_affine3(),
                    opacity: 1.0,
                    frame_elapsed_ms: None,
                    playback: RuntimeResolvedPlayback::Loop,
                    render_order: 7,
                },
                RuntimeResolvedRenderInstance {
                    id: 2,
                    object_id: None,
                    visual: "blue".to_string(),
                    cell: [4, 2, 0],
                    transform: identity_affine3(),
                    opacity: 1.0,
                    frame_elapsed_ms: None,
                    playback: RuntimeResolvedPlayback::Loop,
                    render_order: 7,
                },
            ],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 7,
                composition: RuntimeVisualComposition::Average,
                instances: vec![1, 2],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_image_free_render_frame(&scene, 10).unwrap();
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &frame.batches[0].content
        else {
            panic!("averaged pixels must stay renderer-neutral pixels");
        };
        assert_eq!(frame.batches[0].cell, [4, 2, 0]);
        assert_eq!(pixels.len(), 1);
        assert_eq!(
            pixels[0].color,
            RuntimeLinearRgba {
                red: 0.0,
                green: 0.5,
                blue: 0.5,
                alpha: 1.0,
            }
        );
    }

    #[test]
    fn average_rejects_raster_images_instead_of_entering_logical_pixel_composition() {
        let manifest = VisualImageAssetManifestEntry::from_path("sprite.png").unwrap();
        let scene = RuntimeResolvedRenderScene {
            clips: vec![RuntimeResolvedVisualClip {
                id: "image".to_string(),
                frames: vec![RuntimeResolvedVisualFrame::RasterImage {
                    asset: manifest.id,
                    sampling: RuntimeResolvedSampling::Smooth,
                }],
                frame_duration_ms: None,
                layout: RuntimeResolvedVisualLayout {
                    width: 2,
                    ..test_layout()
                },
            }],
            instances: vec![RuntimeResolvedRenderInstance {
                id: 1,
                object_id: None,
                visual: "image".to_string(),
                cell: [0, 0, 0],
                transform: identity_affine3(),
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: 0,
            }],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 0,
                composition: RuntimeVisualComposition::Average,
                instances: vec![1],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        assert_eq!(
            resolve_image_free_render_frame(&scene, 0),
            Err(PresentationError::RasterImageComposition)
        );
    }

    #[test]
    fn ordered_raster_image_requires_its_decoded_catalog_entry() {
        let manifest = VisualImageAssetManifestEntry::from_path("sprite.png").unwrap();
        let scene = RuntimeResolvedRenderScene {
            clips: vec![RuntimeResolvedVisualClip {
                id: "image".to_string(),
                frames: vec![RuntimeResolvedVisualFrame::RasterImage {
                    asset: manifest.id,
                    sampling: RuntimeResolvedSampling::Pixelated,
                }],
                frame_duration_ms: None,
                layout: test_layout(),
            }],
            instances: vec![RuntimeResolvedRenderInstance {
                id: 1,
                object_id: None,
                visual: "image".to_string(),
                cell: [0, 0, 0],
                transform: identity_affine3(),
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: 0,
            }],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 0,
                composition: RuntimeVisualComposition::Ordered,
                instances: vec![1],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        assert_eq!(
            resolve_image_free_render_frame(&scene, 0),
            Err(PresentationError::MissingImageAsset(
                "sprite.png".to_string()
            ))
        );
    }

    #[test]
    fn pixelated_png_and_same_color_ascii_resolve_to_equivalent_visual_observables() {
        let rgba = [
            [255, 0, 128, 255],
            [1, 2, 3, 128],
            [32, 64, 96, 255],
            [250, 240, 230, 64],
        ];
        let assets = decoded_png_catalog("visuals/tile.png", &rgba, 2, 2);
        let asset_id = VisualImageAssetManifestEntry::from_path("visuals/tile.png")
            .unwrap()
            .id;
        let layout = RuntimeResolvedVisualLayout {
            fit: RuntimeResolvedFitMode::Contain,
            width: 1,
            height: 1,
        };
        let instance = RuntimeResolvedRenderInstance {
            id: 9,
            object_id: Some(4),
            visual: "tile".to_string(),
            cell: [3, 5, 0],
            transform: [
                [1.0, 0.0, 0.0, 0.125],
                [0.0, 1.0, 0.0, -0.25],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            opacity: 0.75,
            frame_elapsed_ms: None,
            playback: RuntimeResolvedPlayback::Loop,
            render_order: 12,
        };
        let scene = |frame| RuntimeResolvedRenderScene {
            clips: vec![RuntimeResolvedVisualClip {
                id: "tile".to_string(),
                frames: vec![frame],
                frame_duration_ms: None,
                layout,
            }],
            instances: vec![instance.clone()],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 12,
                composition: RuntimeVisualComposition::Ordered,
                instances: vec![9],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };
        let ascii_frame = resolve_pixel_frame(
            &["ab".to_string(), "cd".to_string()],
            &BTreeMap::from([
                ("a".to_string(), "#ff0080ff".to_string()),
                ("b".to_string(), "#01020380".to_string()),
                ("c".to_string(), "#204060ff".to_string()),
                ("d".to_string(), "#faf0e640".to_string()),
            ]),
        )
        .unwrap();
        let logical = resolve_image_free_render_frame(&scene(ascii_frame), 0).unwrap();
        let raster = resolve_render_frame(
            &scene(RuntimeResolvedVisualFrame::RasterImage {
                asset: asset_id.clone(),
                sampling: RuntimeResolvedSampling::Pixelated,
            }),
            &assets,
            0,
        )
        .unwrap();
        let logical_batch = &logical.batches[0];
        let raster_batch = &raster.batches[0];
        assert_eq!(logical_batch.render_order, raster_batch.render_order);
        assert_eq!(logical_batch.object_ids, raster_batch.object_ids);
        assert_eq!(logical_batch.cell, raster_batch.cell);
        assert_eq!(logical_batch.transform, raster_batch.transform);
        assert_eq!(logical_batch.opacity, raster_batch.opacity);
        let geometry = logical_batch.pixel_geometry.unwrap();
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &logical_batch.content
        else {
            panic!("ASCII must remain logical pixels");
        };
        let RuntimeResolvedRenderBatchContent::RasterImage {
            asset,
            source_size,
            destination,
            uv,
            sampling,
            ..
        } = &raster_batch.content
        else {
            panic!("external image must remain a raster image");
        };
        assert_eq!(asset, &asset_id);
        assert_eq!(*source_size, [2, 2]);
        assert_eq!(
            *destination,
            RuntimeResolvedRect2d {
                x: geometry.x,
                y: geometry.y,
                width: geometry.width,
                height: geometry.height,
            }
        );
        assert_eq!(geometry.clip, None);
        assert_eq!(
            *uv,
            RuntimeResolvedRect2d {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }
        );
        assert_eq!(*sampling, RuntimeResolvedSampling::Pixelated);
        let decoded = assets.get(asset).unwrap();
        let decoded_colors = decoded
            .rgba8_srgb
            .chunks_exact(4)
            .map(|rgba| RuntimeLinearRgba {
                red: srgb_channel_to_linear(rgba[0]),
                green: srgb_channel_to_linear(rgba[1]),
                blue: srgb_channel_to_linear(rgba[2]),
                alpha: f64::from(rgba[3]) / 255.0,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            pixels.iter().map(|pixel| pixel.color).collect::<Vec<_>>(),
            decoded_colors
        );
    }

    #[test]
    fn cover_crops_raster_uv_to_the_same_visible_edges_as_logical_clip() {
        let assets = decoded_png_catalog("visuals/wide.png", &[[255, 0, 0, 255]; 8], 4, 2);
        let asset_id = VisualImageAssetManifestEntry::from_path("visuals/wide.png")
            .unwrap()
            .id;
        let clip = RuntimeResolvedVisualClip {
            id: "wide".to_string(),
            frames: vec![RuntimeResolvedVisualFrame::RasterImage {
                asset: asset_id.clone(),
                sampling: RuntimeResolvedSampling::Pixelated,
            }],
            frame_duration_ms: None,
            layout: RuntimeResolvedVisualLayout {
                fit: RuntimeResolvedFitMode::Cover,
                width: 1,
                height: 1,
            },
        };
        let logical_geometry = pixel_geometry(
            &RuntimeResolvedVisualFrame::Pixels {
                width: 4,
                height: 2,
                pixels: Vec::new(),
            },
            &clip,
        )
        .unwrap();
        let RuntimeResolvedRenderBatchContent::RasterImage {
            destination, uv, ..
        } = raster_content(
            &asset_id,
            RuntimeResolvedSampling::Pixelated,
            &clip,
            assets.get(&asset_id).unwrap(),
        )
        else {
            unreachable!()
        };
        assert_eq!(Some(destination), logical_geometry.clip);
        assert_eq!(
            uv,
            RuntimeResolvedRect2d {
                x: 0.25,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            }
        );
        assert_eq!(logical_geometry.x, -0.5);
        assert_eq!(logical_geometry.width, 2.0);
    }

    #[test]
    fn average_normalizes_grid_preserving_affines_before_color_composition() {
        let red = RuntimeLinearRgba {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        };
        let blue = RuntimeLinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 1.0,
            alpha: 1.0,
        };
        let clip = |id: &str, position, color| RuntimeResolvedVisualClip {
            id: id.to_string(),
            frames: vec![RuntimeResolvedVisualFrame::Pixels {
                width: 2,
                height: 1,
                pixels: vec![RuntimeResolvedPixel { position, color }],
            }],
            frame_duration_ms: None,
            layout: test_layout(),
        };
        let mut flipped = identity_affine3();
        flipped[0][0] = -1.0;
        let scene = RuntimeResolvedRenderScene {
            clips: vec![clip("left", [0, 0], red), clip("right", [1, 0], blue)],
            instances: vec![
                RuntimeResolvedRenderInstance {
                    id: 1,
                    object_id: None,
                    visual: "left".to_string(),
                    cell: [0, 0, 0],
                    transform: identity_affine3(),
                    opacity: 1.0,
                    frame_elapsed_ms: None,
                    playback: RuntimeResolvedPlayback::Loop,
                    render_order: 0,
                },
                RuntimeResolvedRenderInstance {
                    id: 2,
                    object_id: None,
                    visual: "right".to_string(),
                    cell: [0, 0, 0],
                    transform: flipped,
                    opacity: 1.0,
                    frame_elapsed_ms: None,
                    playback: RuntimeResolvedPlayback::Loop,
                    render_order: 0,
                },
            ],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 0,
                composition: RuntimeVisualComposition::Average,
                instances: vec![1, 2],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_image_free_render_frame(&scene, 0).unwrap();
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &frame.batches[0].content
        else {
            panic!("pixel composition must remain pixels");
        };
        assert_eq!(pixels.len(), 1);
        assert_eq!(pixels[0].position, [0, 0]);
        assert_eq!(pixels[0].color.red, 0.5);
        assert_eq!(pixels[0].color.blue, 0.5);
        assert_eq!(frame.batches[0].transform, identity_affine3());
    }

    #[test]
    fn average_converts_cell_translation_to_the_canonical_sample_lattice() {
        let clip = |id: &str, position, color| RuntimeResolvedVisualClip {
            id: id.to_string(),
            frames: vec![RuntimeResolvedVisualFrame::Pixels {
                width: 2,
                height: 1,
                pixels: vec![RuntimeResolvedPixel { position, color }],
            }],
            frame_duration_ms: None,
            layout: test_layout(),
        };
        let red = RuntimeLinearRgba {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        };
        let blue = RuntimeLinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 1.0,
            alpha: 1.0,
        };
        let mut translated = identity_affine3();
        translated[0][3] = 1.0;
        let instance =
            |id: u64, visual: &str, transform: [[f64; 4]; 4]| RuntimeResolvedRenderInstance {
                id,
                object_id: None,
                visual: visual.to_string(),
                cell: [0, 0, 0],
                transform,
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: 0,
            };
        let scene = RuntimeResolvedRenderScene {
            clips: vec![
                clip("overflow", [2, 0], red),
                clip("translated", [0, 0], blue),
            ],
            instances: vec![
                instance(1, "overflow", identity_affine3()),
                instance(2, "translated", translated),
            ],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 0,
                composition: RuntimeVisualComposition::Average,
                instances: vec![1, 2],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_image_free_render_frame(&scene, 0).unwrap();
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &frame.batches[0].content
        else {
            panic!("translated average must stay pixels");
        };
        assert_eq!(pixels.len(), 1);
        assert_eq!(pixels[0].position, [2, 0]);
        assert_eq!(pixels[0].color.red, 0.5);
        assert_eq!(pixels[0].color.blue, 0.5);
    }

    #[test]
    fn render_moment_owns_move_and_trigger_animation_sampling() {
        let pixel = |red, blue| RuntimeResolvedVisualFrame::Pixels {
            width: 1,
            height: 1,
            pixels: vec![RuntimeResolvedPixel {
                position: [0, 0],
                color: RuntimeLinearRgba {
                    red,
                    green: 0.0,
                    blue,
                    alpha: 1.0,
                },
            }],
        };
        let scene = RuntimeResolvedRenderScene {
            clips: vec![
                RuntimeResolvedVisualClip {
                    id: "actor".to_string(),
                    frames: vec![pixel(1.0, 0.0)],
                    frame_duration_ms: None,
                    layout: test_layout(),
                },
                RuntimeResolvedVisualClip {
                    id: "flash".to_string(),
                    frames: vec![pixel(1.0, 0.0), pixel(0.0, 1.0)],
                    frame_duration_ms: Some(10),
                    layout: test_layout(),
                },
            ],
            instances: vec![RuntimeResolvedRenderInstance {
                id: 1,
                object_id: Some(7),
                visual: "actor".to_string(),
                cell: [1, 0, 0],
                transform: identity_affine3(),
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: 4,
            }],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 4,
                composition: RuntimeVisualComposition::Ordered,
                instances: vec![1],
            }],
            cells: vec![puzzle_runtime_contract::RuntimeResolvedRenderCell {
                position: [1, 0, 0],
                render_order: 2,
                object_ids: vec![7],
            }],
            decorations: Vec::new(),
            render_priority_count: 2,
            animation_duration_ms: 30,
        };
        let frame = resolve_image_free_render_moment(
            &scene,
            &RuntimeResolvedRenderMoment {
                clip_elapsed_ms: 0,
                animation_elapsed_ms: 15,
                animations: vec![
                    RuntimeAnimationEvent::Move {
                        name: "tween".to_string(),
                        occurrence_id: 1,
                        object_id: 7,
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
                        resolved_visual: Some(RuntimeResolvedVisualOrder {
                            render_priority: 1,
                            composition: RuntimeVisualComposition::Ordered,
                        }),
                    },
                ],
            },
        )
        .unwrap();

        assert_eq!(frame.batches.len(), 2);
        assert_eq!(frame.batches[0].object_ids, vec![7]);
        assert_eq!(frame.batches[0].transform[0][3], -0.5);
        assert_eq!(frame.batches[1].render_order, 5);
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &frame.batches[1].content
        else {
            panic!("trigger clip must resolve to pixels");
        };
        assert_eq!(pixels[0].color.blue, 1.0);
    }

    #[test]
    fn resolves_paged_and_centered_2d_views_without_adapter_state() {
        assert_eq!(
            resolve_view_2d([10, 7], Some([4, 3]), ViewMode2d::Paged, Some([9, 6])).unwrap(),
            RuntimeResolvedView2d {
                origin: [6, 4],
                size: [4, 3],
            }
        );
        assert_eq!(
            resolve_view_2d([10, 7], Some([4, 3]), ViewMode2d::Centered, Some([9, 6])).unwrap(),
            RuntimeResolvedView2d {
                origin: [6, 4],
                size: [4, 3],
            }
        );
        assert_eq!(
            resolve_view_2d([10, 7], None, ViewMode2d::Paged, None).unwrap(),
            RuntimeResolvedView2d {
                origin: [0, 0],
                size: [10, 7],
            }
        );
        assert_eq!(
            resolve_view_2d([0, 7], None, ViewMode2d::Paged, None),
            Err(PresentationError::InvalidViewSize)
        );
    }

    #[test]
    fn occupied_2d_grid_deduplicates_shared_edges_and_clips_to_resolved_view() {
        let cells = vec![
            RuntimeResolvedRenderCell {
                position: [2, 3, 0],
                render_order: 0,
                object_ids: vec![1],
            },
            RuntimeResolvedRenderCell {
                position: [3, 3, 0],
                render_order: 1,
                object_ids: vec![2],
            },
            RuntimeResolvedRenderCell {
                position: [4, 3, 0],
                render_order: 2,
                object_ids: vec![3],
            },
            RuntimeResolvedRenderCell {
                position: [2, 4, 0],
                render_order: 3,
                object_ids: Vec::new(),
            },
        ];
        let theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        let RuntimeResolvedDecoration::Lines2d {
            segments,
            style,
            layer,
        } = resolve_grid_decoration_2d(
            RuntimeGridMode::OccupiedCells,
            RuntimeResolvedView2d {
                origin: [2, 3],
                size: [2, 2],
            },
            &cells,
            &theme,
        )
        .unwrap()
        else {
            panic!("2D grid mode must resolve to 2D lines");
        };
        assert_eq!(segments.len(), 7);
        assert_eq!(layer, RuntimeResolvedLineLayer2d::Overlay);
        assert_eq!(style.color.alpha, theme.text.alpha * 0.34);
        assert_eq!(
            style.width,
            RuntimeResolvedStrokeWidth::CellRelative {
                cell_fraction: 1.0 / 24.0,
                min_physical_pixels: 1.0,
            }
        );
        assert!(!segments.iter().any(|segment| segment.start[0] > 4.0));
    }

    #[test]
    fn all_cells_2d_grid_generates_each_lattice_edge_once_in_stable_order() {
        let theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        let RuntimeResolvedDecoration::Lines2d { segments, .. } = resolve_grid_decoration_2d(
            RuntimeGridMode::AllCells,
            RuntimeResolvedView2d {
                origin: [5, 7],
                size: [2, 1],
            },
            &[],
            &theme,
        )
        .unwrap() else {
            panic!("2D grid mode must resolve to 2D lines");
        };
        assert_eq!(segments.len(), 7);
        assert_eq!(segments[0].start, [5.0, 7.0]);
        assert_eq!(segments[0].end, [5.0, 8.0]);
        assert_eq!(segments[3].start, [5.0, 7.0]);
        assert_eq!(segments[3].end, [6.0, 7.0]);
    }

    #[test]
    fn occupied_and_all_cells_3d_grids_share_canonical_wire_edges() {
        let theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        let cells = vec![
            RuntimeResolvedRenderCell {
                position: [0, 0, 0],
                render_order: 0,
                object_ids: vec![1],
            },
            RuntimeResolvedRenderCell {
                position: [1, 0, 0],
                render_order: 1,
                object_ids: vec![2],
            },
        ];
        let RuntimeResolvedDecoration::Lines3d {
            segments,
            style,
            depth,
        } = resolve_grid_decoration_3d(RuntimeGridMode::OccupiedCells, [2, 1, 1], &cells, &theme)
            .unwrap()
        else {
            panic!("3D grid mode must resolve to 3D lines");
        };
        assert_eq!(segments.len(), 20);
        assert_eq!(depth, RuntimeResolvedLineDepth3d::Tested);
        assert_eq!(
            style.width,
            RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 1.0 }
        );

        let RuntimeResolvedDecoration::Lines3d { segments, .. } =
            resolve_grid_decoration_3d(RuntimeGridMode::AllCells, [2, 1, 1], &[], &theme).unwrap()
        else {
            panic!("3D grid mode must resolve to 3D lines");
        };
        assert_eq!(segments.len(), 20);
        assert_eq!(segments[0].start, [-0.5, -0.5, -0.5]);
        assert_eq!(segments[0].end, [0.5, -0.5, -0.5]);
    }

    #[test]
    fn runtime_theme_resolves_preset_and_authored_overrides_to_linear_values() {
        let theme = resolve_runtime_theme(
            Some("clean"),
            [
                ("background", "#000"),
                ("text", "#fff"),
                ("accent", "#ff000080"),
            ],
        )
        .unwrap();

        assert_eq!(theme.background, resolve_palette_color("#000").unwrap());
        assert_eq!(theme.text, resolve_palette_color("#fff").unwrap());
        assert_eq!(theme.accent, resolve_palette_color("#ff000080").unwrap());
        assert_eq!(theme.control.alpha, theme.accent.alpha * 0.14);
        assert_eq!(theme.typography.heading.font_size_px, 30.0);
        assert_eq!(theme.control_layout.corner_radius_px, 6.0);
    }

    #[test]
    fn runtime_theme_rejects_unresolved_preset_and_setting_tokens() {
        assert_eq!(
            resolve_runtime_theme(Some("unknown"), std::iter::empty()).unwrap_err(),
            PresentationError::InvalidThemePreset("unknown".to_string())
        );
        assert_eq!(
            resolve_runtime_theme(Some("clean"), [("shadow", "#000")]).unwrap_err(),
            PresentationError::InvalidThemeSetting("shadow".to_string())
        );
        assert_eq!(
            resolve_runtime_theme(Some("clean"), [("accent", "red")]).unwrap_err(),
            PresentationError::InvalidColor("red".to_string())
        );
    }

    #[test]
    fn runtime_theme_contract_rejects_non_finite_or_negative_fields() {
        let mut theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        theme.typography.body.font_size_px = f32::NAN;
        assert_eq!(theme.validate(), Err("theme.typography.body"));

        let mut theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        theme.control_layout.margin_px = -1.0;
        assert_eq!(theme.validate(), Err("theme.control_layout.margin_px"));

        let mut theme = resolve_runtime_theme(Some("clean"), std::iter::empty()).unwrap();
        theme.accent.alpha = 1.1;
        assert_eq!(theme.validate(), Err("theme.accent"));
    }
}
