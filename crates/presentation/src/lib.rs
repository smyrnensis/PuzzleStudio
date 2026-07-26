use std::collections::{BTreeMap, HashMap};

use puzzle_runtime_contract::{
    RuntimeAnimationEvent, RuntimeLinearRgba, RuntimePresentationEvent,
    RuntimePresentationEventKind, RuntimePuzzle3SpatialOp, RuntimePuzzle3VisualSpace,
    RuntimeResolvedFitMode, RuntimeResolvedImageAsset, RuntimeResolvedPixelGeometry,
    RuntimeResolvedPlayback, RuntimeResolvedRenderBatch, RuntimeResolvedRenderBatchContent,
    RuntimeResolvedRenderFrame, RuntimeResolvedRenderInstance, RuntimeResolvedRenderMoment,
    RuntimeResolvedRenderScene, RuntimeResolvedVisualClip, RuntimeResolvedVisualFrame,
    RuntimeResolvedVisualOrder, RuntimeScalarTween, RuntimeVisualComposition, RuntimeVisualSpace,
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
    UnknownPaletteToken(String),
    EmptyVisualClip(String),
    ZeroFrameDuration(String),
    UnknownVisual(String),
    UnknownRenderInstance(u64),
    MixedCompositionCells,
    MixedCompositionTransforms,
    NonLatticeCompositionTransform,
    IncompatibleCompositionFrames,
    ExternalImageComposition,
    MissingImageAsset(String),
    InvalidImageAsset(String),
    ZeroAnimationDuration,
    MissingAnimationTarget { object_id: u16, cell: [i32; 3] },
    MissingRenderCell([i32; 3]),
    MissingResolvedAnimationVisual(String),
}

pub fn hydrate_external_images(
    scene: &RuntimeResolvedRenderScene,
    assets: &[RuntimeResolvedImageAsset],
) -> Result<RuntimeResolvedRenderScene, PresentationError> {
    let assets = assets
        .iter()
        .map(|asset| (asset.source.as_str(), asset))
        .collect::<HashMap<_, _>>();
    let mut hydrated = scene.clone();
    for clip in &mut hydrated.clips {
        for frame in &mut clip.frames {
            let RuntimeResolvedVisualFrame::ExternalImage { source } = frame else {
                continue;
            };
            let asset = assets
                .get(source.as_str())
                .copied()
                .ok_or_else(|| PresentationError::MissingImageAsset(source.clone()))?;
            let expected_len = usize::from(asset.width)
                .checked_mul(usize::from(asset.height))
                .and_then(|pixels| pixels.checked_mul(4));
            if asset.width == 0 || asset.height == 0 || expected_len != Some(asset.rgba8_srgb.len())
            {
                return Err(PresentationError::InvalidImageAsset(source.clone()));
            }
            let pixels = asset
                .rgba8_srgb
                .chunks_exact(4)
                .enumerate()
                .filter_map(|(index, rgba)| {
                    (rgba[3] > 0).then(|| puzzle_runtime_contract::RuntimeResolvedPixel {
                        position: [
                            i32::try_from(index % usize::from(asset.width))
                                .expect("validated image width must fit i32"),
                            i32::try_from(index / usize::from(asset.width))
                                .expect("validated image height must fit i32"),
                        ],
                        color: RuntimeLinearRgba {
                            red: srgb_channel_to_linear(rgba[0]),
                            green: srgb_channel_to_linear(rgba[1]),
                            blue: srgb_channel_to_linear(rgba[2]),
                            alpha: f64::from(rgba[3]) / 255.0,
                        },
                    })
                })
                .collect();
            *frame = RuntimeResolvedVisualFrame::Pixels {
                width: asset.width,
                height: asset.height,
                pixels,
            };
        }
    }
    Ok(hydrated)
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
                        position: [x as i32, y as i32, z as i32],
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
    elapsed_ms: u64,
) -> Result<RuntimeResolvedRenderFrame, PresentationError> {
    resolve_render_moment(
        scene,
        &RuntimeResolvedRenderMoment {
            clip_elapsed_ms: elapsed_ms,
            animation_elapsed_ms: 0,
            animations: Vec::new(),
        },
    )
}

pub fn resolve_render_moment(
    scene: &RuntimeResolvedRenderScene,
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
                    batches.push(instance_batch(instance, &clips, moment.clip_elapsed_ms)?);
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
        continue_animation,
    })
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
        content: frame_content(frame)?,
    })
}

fn frame_content(
    frame: &RuntimeResolvedVisualFrame,
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
        RuntimeResolvedVisualFrame::ExternalImage { source } => {
            return Err(PresentationError::MissingImageAsset(source.clone()));
        }
    })
}

fn pixel_geometry(
    frame: &RuntimeResolvedVisualFrame,
    clip: &RuntimeResolvedVisualClip,
) -> Option<RuntimeResolvedPixelGeometry> {
    let RuntimeResolvedVisualFrame::Pixels { width, height, .. } = frame else {
        return None;
    };
    let source_width = f64::from(*width);
    let source_height = f64::from(*height);
    let box_width = f64::from(clip.layout.width);
    let box_height = f64::from(clip.layout.height);
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
    Some(RuntimeResolvedPixelGeometry {
        x: (1.0 - box_width) / 2.0 + (box_width - width) / 2.0,
        y: (1.0 - box_height) / 2.0 + (box_height - height) / 2.0,
        width,
        height,
        sampling: clip.layout.sampling,
        raster: clip.layout.raster,
    })
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
                || clip.layout.sampling != first_clip.layout.sampling
        })
    }) {
        return Err(PresentationError::IncompatibleCompositionFrames);
    }
    let mut geometry = pixel_geometry(frames[0], first_clip);
    let content = average_frames(&frames, members, geometry)?;
    if let Some(geometry) = &mut geometry {
        geometry.raster = members.iter().any(|member| {
            clips
                .get(member.visual.as_str())
                .is_some_and(|clip| clip.layout.raster)
        });
    }
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
        Some(RuntimeResolvedVisualFrame::ExternalImage { .. }) => {
            Err(PresentationError::ExternalImageComposition)
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
            ("back", u64::from(y), u64::from(depth)),
            ("front", u64::from(depth - 1 - y), u64::from(depth)),
            ("down", u64::from(z), u64::from(height)),
            ("up", u64::from(height - 1 - z), u64::from(height)),
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
        if let RuntimePresentationEventKind::AnimationBatch { animations } = &mut event.event {
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
    use puzzle_runtime_contract::{
        RuntimeCoord, RuntimeResolvedCompositionGroup, RuntimeResolvedPixel,
        RuntimeResolvedSampling, RuntimeResolvedVisualClip, RuntimeResolvedVisualLayout,
    };

    fn test_layout() -> RuntimeResolvedVisualLayout {
        RuntimeResolvedVisualLayout {
            fit: RuntimeResolvedFitMode::Contain,
            width: 1,
            height: 1,
            sampling: RuntimeResolvedSampling::Pixelated,
            raster: false,
        }
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
            ("d".to_string(), "#ffffff".to_string()),
        ]);
        let frame = resolve_voxel_frame(
            &[
                vec!["ab".to_string(), "cd".to_string()],
                vec!["a.".to_string(), "..".to_string()],
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
            vec![[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0], [0, 0, 1]]
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
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_render_frame(&scene, 10).unwrap();
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
    fn average_rejects_external_images_without_decoded_rgba() {
        let scene = RuntimeResolvedRenderScene {
            clips: vec![RuntimeResolvedVisualClip {
                id: "image".to_string(),
                frames: vec![RuntimeResolvedVisualFrame::ExternalImage {
                    source: "sprite.png".to_string(),
                }],
                frame_duration_ms: None,
                layout: RuntimeResolvedVisualLayout {
                    width: 2,
                    sampling: RuntimeResolvedSampling::Smooth,
                    raster: true,
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
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        assert_eq!(
            resolve_render_frame(&scene, 0),
            Err(PresentationError::ExternalImageComposition)
        );

        let hydrated = hydrate_external_images(
            &scene,
            &[RuntimeResolvedImageAsset {
                source: "sprite.png".to_string(),
                width: 1,
                height: 1,
                rgba8_srgb: vec![255, 0, 255, 255],
            }],
        )
        .unwrap();
        let frame = resolve_render_frame(&hydrated, 0).unwrap();
        let RuntimeResolvedRenderBatchContent::Pixels { pixels, .. } = &frame.batches[0].content
        else {
            panic!("decoded external image must enter pixel composition");
        };
        assert_eq!(pixels[0].color.blue, 1.0);
        assert_eq!(
            frame.batches[0].pixel_geometry,
            Some(RuntimeResolvedPixelGeometry {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                sampling: RuntimeResolvedSampling::Smooth,
                raster: true,
            })
        );
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
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_render_frame(&scene, 0).unwrap();
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
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let frame = resolve_render_frame(&scene, 0).unwrap();
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
            render_priority_count: 2,
            animation_duration_ms: 30,
        };
        let frame = resolve_render_moment(
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
}
