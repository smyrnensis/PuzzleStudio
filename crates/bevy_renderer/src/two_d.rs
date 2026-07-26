use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use bevy::{
    asset::RenderAssetUsages,
    camera::{ClearColorConfig, ScalingMode, visibility::RenderLayers},
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::Indices,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
    sprite_render::AlphaMode2d,
};
use puzzle_assets::{DecodedVisualImageCatalog, VisualImageAssetId, VisualImageAssetRevision};
use puzzle_runtime_contract::{
    RuntimeResolvedDecoration, RuntimeResolvedRenderBatchContent, RuntimeResolvedRenderFrame,
    RuntimeResolvedSampling, RuntimeResolvedStrokeWidth,
};

use super::{
    BevyRenderError, PuzzleBevyFramebufferRect, PuzzleBevyRenderView, PuzzleBevyRendererSystems,
    PuzzleBevyViewDimension, PuzzleBevyViewId, checked_transform, finite_unit, resolved_color,
    runtime_affine,
};

const MESH_Z_STEP: f32 = 0.001;
const FIRST_2D_RENDER_LAYER: usize = 3;

#[derive(Clone, Debug)]
pub struct PuzzleBevy2dView {
    pub active: bool,
    pub order: isize,
    pub framebuffer: PuzzleBevyFramebufferRect,
    pub clear_color: Color,
    pub origin: Vec2,
    pub size: Vec2,
}

impl Default for PuzzleBevy2dView {
    fn default() -> Self {
        Self {
            active: true,
            order: 0,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::ONE,
            },
            clear_color: Color::BLACK,
            origin: Vec2::ZERO,
            size: Vec2::ONE,
        }
    }
}

impl PuzzleBevy2dView {
    fn validate(&self) -> Result<(), BevyRenderError> {
        self.framebuffer.validate()?;
        if !self.origin.is_finite() {
            return Err(BevyRenderError::InvalidViewGeometry { field: "origin" });
        }
        if !self.size.is_finite() || self.size.x <= 0.0 || self.size.y <= 0.0 {
            return Err(BevyRenderError::InvalidViewGeometry { field: "size" });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct PuzzleBevy2dPlugin;

impl Plugin for PuzzleBevy2dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyResolvedFrameQueue2d>()
            .init_resource::<RenderedFrameState2d>()
            .add_systems(Startup, setup_renderer_2d)
            .add_systems(
                PostUpdate,
                apply_pending_frame_2d.in_set(PuzzleBevyRendererSystems::ApplySubmittedFrames),
            );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedPixelMeshKey {
    pub batch_index: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedPixelMesh {
    pub key: PreparedPixelMeshKey,
    pub transform: Transform,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
    mesh: PreparedLogicalMesh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedLineMesh2dKey {
    pub decoration_index: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedLineMesh2d {
    pub key: PreparedLineMesh2dKey,
    mesh: PreparedLogicalMesh,
}

#[derive(Clone, Debug)]
pub struct PreparedBevy2dFrame {
    pub meshes: Vec<PreparedPixelMesh>,
    pub raster_quads: Vec<PreparedRasterQuad>,
    pub line_meshes: Vec<PreparedLineMesh2d>,
    pub continue_animation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedRasterQuadKey {
    pub batch_index: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedRasterQuad {
    pub key: PreparedRasterQuadKey,
    pub transform: Transform,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
    pub opacity: f32,
    texture: PreparedRasterTexture,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RasterTextureKey {
    asset: VisualImageAssetId,
    revision: VisualImageAssetRevision,
    sampling: RuntimeResolvedSampling,
}

#[derive(Clone, Debug)]
struct PreparedRasterTexture {
    key: RasterTextureKey,
    width: u16,
    height: u16,
}

#[derive(Clone, Debug)]
struct PreparedLogicalMesh {
    key: LogicalMeshKey,
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LogicalMeshKey {
    positions: Vec<[u32; 3]>,
    colors: Vec<[u32; 4]>,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RasterMeshKey {
    positions: Vec<[u32; 3]>,
    uvs: Vec<[u32; 2]>,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RasterMaterialKey {
    texture: RasterTextureKey,
    opacity: u32,
}

pub fn prepare_resolved_frame_2d(
    frame: &RuntimeResolvedRenderFrame,
    catalog: &DecodedVisualImageCatalog,
    view: &PuzzleBevy2dView,
) -> Result<PreparedBevy2dFrame, BevyRenderError> {
    view.validate()?;
    let mut meshes = Vec::with_capacity(frame.batches.len());
    let mut raster_quads = Vec::new();
    for (batch_index, batch) in frame.batches.iter().enumerate() {
        let opacity = finite_unit(batch.opacity)
            .ok_or(BevyRenderError::InvalidOpacity { batch_index })? as f32;
        match &batch.content {
            RuntimeResolvedRenderBatchContent::Pixels {
                width,
                height,
                pixels,
            } => meshes.push(prepare_pixel_mesh(
                batch_index,
                batch,
                opacity,
                *width,
                *height,
                pixels,
            )?),
            RuntimeResolvedRenderBatchContent::RasterImage {
                asset,
                revision,
                source_size,
                destination,
                uv,
                sampling,
            } => raster_quads.push(prepare_raster_quad(
                batch_index,
                batch,
                opacity,
                asset,
                revision,
                *source_size,
                *destination,
                *uv,
                *sampling,
                catalog,
            )?),
            RuntimeResolvedRenderBatchContent::Voxels { .. } => {
                return Err(BevyRenderError::UnsupportedPrimitive {
                    batch_index,
                    kind: "voxel",
                });
            }
        }
    }
    let line_meshes = prepare_line_meshes_2d(frame, view)?;
    Ok(PreparedBevy2dFrame {
        meshes,
        raster_quads,
        line_meshes,
        continue_animation: frame.continue_animation,
    })
}

fn prepare_pixel_mesh(
    batch_index: usize,
    batch: &puzzle_runtime_contract::RuntimeResolvedRenderBatch,
    opacity: f32,
    width: u16,
    height: u16,
    pixels: &[puzzle_runtime_contract::RuntimeResolvedPixel],
) -> Result<PreparedPixelMesh, BevyRenderError> {
    if width == 0 || height == 0 {
        return Err(BevyRenderError::InvalidPixelDimensions { batch_index });
    }
    let geometry = batch
        .pixel_geometry
        .ok_or(BevyRenderError::MissingPixelGeometry { batch_index })?;
    if !geometry.x.is_finite()
        || !geometry.y.is_finite()
        || !geometry.width.is_finite()
        || !geometry.height.is_finite()
        || geometry.width <= 0.0
        || geometry.height <= 0.0
    {
        return Err(BevyRenderError::InvalidPixelGeometry { batch_index });
    }
    // Build each axis once so adjacent logical pixels reference the exact
    // same floating-point edge value. Computing a pixel's far edge from
    // its near edge would introduce a second rounding path at shared edges.
    let x_edges = (0..=width)
        .map(|x| {
            geometry.x as f32 + geometry.width as f32 * (f32::from(x) / f32::from(width)) - 0.5
        })
        .collect::<Vec<_>>();
    let y_edges = (0..=height)
        .map(|y| {
            geometry.y as f32 + geometry.height as f32 * (f32::from(y) / f32::from(height)) - 0.5
        })
        .collect::<Vec<_>>();
    let mut occupied = HashSet::with_capacity(pixels.len());
    let mut positions = Vec::with_capacity(pixels.len() * 4);
    let mut colors = Vec::with_capacity(pixels.len() * 4);
    let mut indices = Vec::with_capacity(pixels.len() * 6);
    for (pixel_index, pixel) in pixels.iter().enumerate() {
        let [x, y] = pixel.position;
        if x < 0 || y < 0 || x >= i32::from(width) || y >= i32::from(height) {
            return Err(BevyRenderError::InvalidPixelPosition {
                batch_index,
                pixel_index,
            });
        }
        if !occupied.insert([x, y]) {
            return Err(BevyRenderError::DuplicatePixelPosition {
                batch_index,
                pixel_index,
            });
        }
        let color = resolved_color(pixel.color, opacity, batch_index, pixel_index)?;
        if color.alpha <= 0.0 {
            continue;
        }
        let x = usize::try_from(x).map_err(|_| BevyRenderError::InvalidPixelPosition {
            batch_index,
            pixel_index,
        })?;
        let y = usize::try_from(y).map_err(|_| BevyRenderError::InvalidPixelPosition {
            batch_index,
            pixel_index,
        })?;
        let mut left = x_edges[x];
        let mut right = x_edges[x + 1];
        let mut top = y_edges[y];
        let mut bottom = y_edges[y + 1];
        if let Some(clip) = geometry.clip {
            left = left.max(clip.x as f32 - 0.5);
            right = right.min((clip.x + clip.width) as f32 - 0.5);
            top = top.max(clip.y as f32 - 0.5);
            bottom = bottom.min((clip.y + clip.height) as f32 - 0.5);
            if left >= right || top >= bottom {
                continue;
            }
        }
        let first = u32::try_from(positions.len())
            .map_err(|_| BevyRenderError::InvalidPixelDimensions { batch_index })?;
        positions.extend_from_slice(&[
            [left, -top, 0.0],
            [right, -top, 0.0],
            [right, -bottom, 0.0],
            [left, -bottom, 0.0],
        ]);
        colors.extend_from_slice(&[color.to_f32_array(); 4]);
        indices.extend_from_slice(&[first, first + 2, first + 1, first, first + 3, first + 2]);
    }

    let mut transform = batch_transform(batch, batch_index)?;
    transform.translation.z = batch_index as f32 * MESH_Z_STEP;
    let key = LogicalMeshKey {
        positions: positions
            .iter()
            .map(|position| position.map(f32::to_bits))
            .collect(),
        colors: colors.iter().map(|color| color.map(f32::to_bits)).collect(),
        indices: indices.clone(),
    };
    Ok(PreparedPixelMesh {
        key: PreparedPixelMeshKey { batch_index },
        transform,
        render_order: batch.render_order,
        object_ids: batch.object_ids.clone(),
        mesh: PreparedLogicalMesh {
            key,
            positions,
            colors,
            indices,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_raster_quad(
    batch_index: usize,
    batch: &puzzle_runtime_contract::RuntimeResolvedRenderBatch,
    opacity: f32,
    asset_id: &VisualImageAssetId,
    revision: &VisualImageAssetRevision,
    source_size: [u16; 2],
    destination: puzzle_runtime_contract::RuntimeResolvedRect2d,
    uv: puzzle_runtime_contract::RuntimeResolvedRect2d,
    sampling: RuntimeResolvedSampling,
    catalog: &DecodedVisualImageCatalog,
) -> Result<PreparedRasterQuad, BevyRenderError> {
    if batch.pixel_geometry.is_some()
        || !valid_rect(destination)
        || !valid_rect(uv)
        || uv.x < 0.0
        || uv.y < 0.0
        || uv.x + uv.width > 1.0
        || uv.y + uv.height > 1.0
    {
        return Err(BevyRenderError::InvalidRasterGeometry { batch_index });
    }
    let asset = catalog
        .get(asset_id)
        .ok_or_else(|| BevyRenderError::MissingRasterAsset {
            batch_index,
            asset: asset_id.clone(),
        })?;
    if &asset.revision != revision {
        return Err(BevyRenderError::RasterAssetRevisionMismatch {
            batch_index,
            asset: asset_id.clone(),
            expected: revision.clone(),
            actual: asset.revision.clone(),
        });
    }
    let actual_size = [asset.width, asset.height];
    if source_size != actual_size {
        return Err(BevyRenderError::RasterAssetDimensionsMismatch {
            batch_index,
            asset: asset_id.clone(),
            expected: source_size,
            actual: actual_size,
        });
    }
    let left = destination.x as f32 - 0.5;
    let right = (destination.x + destination.width) as f32 - 0.5;
    let top = destination.y as f32 - 0.5;
    let bottom = (destination.y + destination.height) as f32 - 0.5;
    let uv_left = uv.x as f32;
    let uv_right = (uv.x + uv.width) as f32;
    let uv_top = uv.y as f32;
    let uv_bottom = (uv.y + uv.height) as f32;
    let mut transform = batch_transform(batch, batch_index)?;
    transform.translation.z = batch_index as f32 * MESH_Z_STEP;
    Ok(PreparedRasterQuad {
        key: PreparedRasterQuadKey { batch_index },
        transform,
        render_order: batch.render_order,
        object_ids: batch.object_ids.clone(),
        opacity,
        texture: PreparedRasterTexture {
            key: RasterTextureKey {
                asset: asset_id.clone(),
                revision: revision.clone(),
                sampling,
            },
            width: asset.width,
            height: asset.height,
        },
        positions: vec![
            [left, -top, 0.0],
            [right, -top, 0.0],
            [right, -bottom, 0.0],
            [left, -bottom, 0.0],
        ],
        uvs: vec![
            [uv_left, uv_top],
            [uv_right, uv_top],
            [uv_right, uv_bottom],
            [uv_left, uv_bottom],
        ],
        indices: vec![0, 2, 1, 0, 3, 2],
    })
}

fn valid_rect(rect: puzzle_runtime_contract::RuntimeResolvedRect2d) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f64::is_finite)
        && rect.width > 0.0
        && rect.height > 0.0
}

fn batch_transform(
    batch: &puzzle_runtime_contract::RuntimeResolvedRenderBatch,
    batch_index: usize,
) -> Result<Transform, BevyRenderError> {
    let affine = runtime_affine(batch.transform, batch_index)?;
    let y_flip = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
    let cell_center = Vec3::new(batch.cell[0] as f32 + 0.5, batch.cell[1] as f32 + 0.5, 0.0);
    checked_transform(
        y_flip * Mat4::from_translation(cell_center) * affine * y_flip,
        batch_index,
    )
}

fn prepare_line_meshes_2d(
    frame: &RuntimeResolvedRenderFrame,
    view: &PuzzleBevy2dView,
) -> Result<Vec<PreparedLineMesh2d>, BevyRenderError> {
    let scale = Vec2::new(
        view.framebuffer.physical_size.x as f32 / view.size.x,
        view.framebuffer.physical_size.y as f32 / view.size.y,
    );
    let decoration_z = (frame.batches.len() as f32 + 1.0) * MESH_Z_STEP;
    let mut prepared = Vec::with_capacity(frame.decorations.len());
    for (decoration_index, decoration) in frame.decorations.iter().enumerate() {
        let RuntimeResolvedDecoration::Lines2d {
            segments,
            style,
            layer: _,
        } = decoration
        else {
            return Err(BevyRenderError::UnsupportedDecoration {
                decoration_index,
                kind: "3D lines",
            });
        };
        let color = resolved_decoration_color(style.color, decoration_index)?;
        let width_pixels = match style.width {
            RuntimeResolvedStrokeWidth::CellRelative {
                cell_fraction,
                min_physical_pixels,
            } => {
                if !cell_fraction.is_finite()
                    || cell_fraction <= 0.0
                    || !min_physical_pixels.is_finite()
                    || min_physical_pixels <= 0.0
                {
                    return Err(BevyRenderError::InvalidDecoration {
                        decoration_index,
                        field: "width",
                    });
                }
                (cell_fraction as f32 * scale.x.min(scale.y)).max(min_physical_pixels as f32)
            }
            RuntimeResolvedStrokeWidth::PhysicalPixels { pixels } => {
                if !pixels.is_finite() || pixels <= 0.0 {
                    return Err(BevyRenderError::InvalidDecoration {
                        decoration_index,
                        field: "width",
                    });
                }
                pixels as f32
            }
        };
        let mut positions = Vec::with_capacity(segments.len() * 4);
        let mut colors = Vec::with_capacity(segments.len() * 4);
        let mut indices = Vec::with_capacity(segments.len() * 6);
        let mut vertical = Vec::new();
        let mut horizontal = Vec::new();
        for segment in segments {
            let start = Vec2::new(segment.start[0] as f32, segment.start[1] as f32);
            let end = Vec2::new(segment.end[0] as f32, segment.end[1] as f32);
            if !start.is_finite() || !end.is_finite() {
                return Err(BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "segment",
                });
            }
            if start.abs_diff_eq(end, f32::EPSILON) {
                return Err(BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "segment",
                });
            }
            if (start.x - end.x).abs() <= f32::EPSILON {
                let half_width = width_pixels * 0.5 / scale.x;
                vertical.push(LineRect2d {
                    min: Vec2::new(start.x - half_width, start.y.min(end.y)),
                    max: Vec2::new(start.x + half_width, start.y.max(end.y)),
                });
            } else if (start.y - end.y).abs() <= f32::EPSILON {
                let half_width = width_pixels * 0.5 / scale.y;
                horizontal.push(LineRect2d {
                    min: Vec2::new(start.x.min(end.x), start.y - half_width),
                    max: Vec2::new(start.x.max(end.x), start.y + half_width),
                });
            } else {
                return Err(BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "segment (2D line decorations must be axis-aligned)",
                });
            }
        }
        let mut covered = Vec::with_capacity(vertical.len() + horizontal.len());
        let mut partitioned = Vec::new();
        for rectangle in vertical.into_iter().chain(horizontal) {
            let mut pieces = vec![rectangle];
            for blocker in &covered {
                pieces = pieces
                    .into_iter()
                    .flat_map(|piece| subtract_rectangle(piece, *blocker))
                    .collect();
            }
            partitioned.extend(pieces);
            covered.push(rectangle);
        }
        for rectangle in partitioned {
            let corners = [
                rectangle.min,
                Vec2::new(rectangle.max.x, rectangle.min.y),
                rectangle.max,
                Vec2::new(rectangle.min.x, rectangle.max.y),
            ];
            let first =
                u32::try_from(positions.len()).map_err(|_| BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "segment_count",
                })?;
            positions.extend(corners.map(|point| [point.x, -point.y, decoration_z]));
            colors.extend_from_slice(&[color.to_f32_array(); 4]);
            indices.extend_from_slice(&[first, first + 2, first + 1, first, first + 3, first + 2]);
        }
        let key = LogicalMeshKey {
            positions: positions
                .iter()
                .map(|position| position.map(f32::to_bits))
                .collect(),
            colors: colors.iter().map(|color| color.map(f32::to_bits)).collect(),
            indices: indices.clone(),
        };
        prepared.push(PreparedLineMesh2d {
            key: PreparedLineMesh2dKey { decoration_index },
            mesh: PreparedLogicalMesh {
                key,
                positions,
                colors,
                indices,
            },
        });
    }
    Ok(prepared)
}

#[derive(Clone, Copy, Debug)]
struct LineRect2d {
    min: Vec2,
    max: Vec2,
}

fn subtract_rectangle(piece: LineRect2d, blocker: LineRect2d) -> Vec<LineRect2d> {
    let intersection = LineRect2d {
        min: piece.min.max(blocker.min),
        max: piece.max.min(blocker.max),
    };
    if intersection.min.x >= intersection.max.x || intersection.min.y >= intersection.max.y {
        return vec![piece];
    }
    [
        LineRect2d {
            min: piece.min,
            max: Vec2::new(intersection.min.x, piece.max.y),
        },
        LineRect2d {
            min: Vec2::new(intersection.max.x, piece.min.y),
            max: piece.max,
        },
        LineRect2d {
            min: Vec2::new(intersection.min.x, piece.min.y),
            max: Vec2::new(intersection.max.x, intersection.min.y),
        },
        LineRect2d {
            min: Vec2::new(intersection.min.x, intersection.max.y),
            max: Vec2::new(intersection.max.x, piece.max.y),
        },
    ]
    .into_iter()
    .filter(|rectangle| rectangle.min.x < rectangle.max.x && rectangle.min.y < rectangle.max.y)
    .collect()
}

fn resolved_decoration_color(
    color: puzzle_runtime_contract::RuntimeLinearRgba,
    decoration_index: usize,
) -> Result<LinearRgba, BevyRenderError> {
    let channels = [color.red, color.green, color.blue, color.alpha];
    if channels
        .iter()
        .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(channel))
    {
        return Err(BevyRenderError::InvalidDecoration {
            decoration_index,
            field: "color",
        });
    }
    Ok(LinearRgba::new(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32,
    ))
}

pub fn submit_resolved_frame_2d(
    world: &mut World,
    view_id: PuzzleBevyViewId,
    view: PuzzleBevy2dView,
    catalog: Arc<DecodedVisualImageCatalog>,
    frame: &RuntimeResolvedRenderFrame,
) -> Result<u64, BevyRenderError> {
    view_id.validate_dimension(PuzzleBevyViewDimension::TwoD)?;
    let Some(mut queue) = world.get_resource_mut::<BevyResolvedFrameQueue2d>() else {
        return Err(BevyRenderError::RendererNotInstalled);
    };
    queue.submit(view_id, view, catalog, frame)
}

pub fn submit_image_free_resolved_frame_2d(
    world: &mut World,
    view_id: PuzzleBevyViewId,
    view: PuzzleBevy2dView,
    frame: &RuntimeResolvedRenderFrame,
) -> Result<u64, BevyRenderError> {
    submit_resolved_frame_2d(
        world,
        view_id,
        view,
        Arc::new(DecodedVisualImageCatalog::default()),
        frame,
    )
}

pub fn remove_render_view_2d(
    world: &mut World,
    view_id: &PuzzleBevyViewId,
) -> Result<u64, BevyRenderError> {
    let Some(mut queue) = world.get_resource_mut::<BevyResolvedFrameQueue2d>() else {
        return Err(BevyRenderError::RendererNotInstalled);
    };
    queue.remove(view_id)
}

#[derive(Resource, Default)]
pub struct BevyResolvedFrameQueue2d {
    next_generation: u64,
    registry: super::BevyViewRegistry,
    pending: HashMap<PuzzleBevyViewId, PendingViewChange2d>,
}

impl BevyResolvedFrameQueue2d {
    pub fn reconcile_camera_orders(
        &mut self,
        desired: &BTreeMap<PuzzleBevyViewId, isize>,
    ) -> Result<(), BevyRenderError> {
        for view_id in desired.keys() {
            view_id.validate()?;
            view_id.validate_dimension(PuzzleBevyViewDimension::TwoD)?;
        }
        self.registry.reconcile_camera_orders(desired)
    }

    pub fn submit(
        &mut self,
        view_id: PuzzleBevyViewId,
        view: PuzzleBevy2dView,
        catalog: Arc<DecodedVisualImageCatalog>,
        frame: &RuntimeResolvedRenderFrame,
    ) -> Result<u64, BevyRenderError> {
        view_id.validate()?;
        view_id.validate_dimension(PuzzleBevyViewDimension::TwoD)?;
        let prepared = prepare_resolved_frame_2d(frame, &catalog, &view)?;
        self.submit_prepared(view_id, view, catalog, prepared)
    }

    pub fn submit_prepared(
        &mut self,
        view_id: PuzzleBevyViewId,
        view: PuzzleBevy2dView,
        catalog: Arc<DecodedVisualImageCatalog>,
        frame: PreparedBevy2dFrame,
    ) -> Result<u64, BevyRenderError> {
        view_id.validate()?;
        view_id.validate_dimension(PuzzleBevyViewDimension::TwoD)?;
        view.validate()?;
        let generation = self.next_generation()?;
        let (render_layer, camera_order) =
            self.registry
                .reserve(&view_id, view.order, FIRST_2D_RENDER_LAYER)?;
        self.pending.insert(
            view_id.clone(),
            PendingViewChange2d::Submit(SubmittedFrame2d {
                generation,
                view_id,
                render_layer,
                camera_order,
                view,
                catalog,
                frame,
            }),
        );
        Ok(generation)
    }

    pub fn remove(&mut self, view_id: &PuzzleBevyViewId) -> Result<u64, BevyRenderError> {
        view_id.validate_dimension(PuzzleBevyViewDimension::TwoD)?;
        self.registry.validate_removal(view_id)?;
        let generation = self.next_generation()?;
        self.registry.release_registered(view_id);
        self.pending
            .insert(view_id.clone(), PendingViewChange2d::Remove);
        Ok(generation)
    }

    fn next_generation(&mut self) -> Result<u64, BevyRenderError> {
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .ok_or(BevyRenderError::ViewGenerationExhausted)?;
        Ok(self.next_generation)
    }
}

struct SubmittedFrame2d {
    generation: u64,
    view_id: PuzzleBevyViewId,
    render_layer: usize,
    camera_order: isize,
    view: PuzzleBevy2dView,
    catalog: Arc<DecodedVisualImageCatalog>,
    frame: PreparedBevy2dFrame,
}

enum PendingViewChange2d {
    Submit(SubmittedFrame2d),
    Remove,
}

#[derive(Component, Clone, Debug)]
pub struct PuzzlePixelMesh {
    pub key: PreparedPixelMeshKey,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
}

#[derive(Component, Clone, Debug)]
pub struct PuzzleLineMesh2d {
    pub key: PreparedLineMesh2dKey,
}

#[derive(Component, Clone, Debug)]
pub struct PuzzleRasterQuad {
    pub key: PreparedRasterQuadKey,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
}

#[derive(Component)]
struct PuzzleRendererCamera2d;

#[derive(Resource)]
struct RenderAssets2d {
    material: Handle<ColorMaterial>,
    meshes: HashMap<LogicalMeshKey, Handle<Mesh>>,
    raster_meshes: HashMap<RasterMeshKey, Handle<Mesh>>,
    raster_textures: HashMap<RasterTextureKey, Handle<Image>>,
    raster_materials: HashMap<RasterMaterialKey, Handle<ColorMaterial>>,
}

struct RenderedViewState2d {
    generation: u64,
    entities: HashMap<PreparedPixelMeshKey, Entity>,
    raster_entities: HashMap<PreparedRasterQuadKey, Entity>,
    line_entities: HashMap<PreparedLineMesh2dKey, Entity>,
    mesh_keys: HashSet<LogicalMeshKey>,
    raster_mesh_keys: HashSet<RasterMeshKey>,
    raster_texture_keys: HashSet<RasterTextureKey>,
    raster_material_keys: HashSet<RasterMaterialKey>,
    camera: Entity,
    render_layer: usize,
}

#[derive(Resource, Default)]
struct RenderedFrameState2d {
    views: HashMap<PuzzleBevyViewId, RenderedViewState2d>,
}

fn setup_renderer_2d(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.insert_resource(RenderAssets2d {
        material: materials.add(ColorMaterial::default()),
        meshes: HashMap::new(),
        raster_meshes: HashMap::new(),
        raster_textures: HashMap::new(),
        raster_materials: HashMap::new(),
    });
}

fn apply_pending_frame_2d(
    mut commands: Commands,
    mut queue: ResMut<BevyResolvedFrameQueue2d>,
    mut state: ResMut<RenderedFrameState2d>,
    mut render_assets: ResMut<RenderAssets2d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if queue.pending.is_empty() {
        return;
    }
    let pending = std::mem::take(&mut queue.pending);
    for (view_id, change) in pending {
        match change {
            PendingViewChange2d::Remove => {
                if let Some(view) = state.views.remove(&view_id) {
                    for entity in view.entities.into_values() {
                        commands.entity(entity).despawn();
                    }
                    for entity in view.raster_entities.into_values() {
                        commands.entity(entity).despawn();
                    }
                    for entity in view.line_entities.into_values() {
                        commands.entity(entity).despawn();
                    }
                    commands.entity(view.camera).despawn();
                }
                queue.registry.finish_removal(&view_id);
            }
            PendingViewChange2d::Submit(submitted) => {
                let render_layers = RenderLayers::none().with(submitted.render_layer);
                let mut view = state.views.remove(&view_id).unwrap_or_else(|| {
                    let camera = commands
                        .spawn((
                            Camera2d,
                            PuzzleRendererCamera2d,
                            PuzzleBevyRenderView {
                                id: submitted.view_id.clone(),
                            },
                            render_layers.clone(),
                        ))
                        .id();
                    RenderedViewState2d {
                        generation: 0,
                        entities: HashMap::new(),
                        raster_entities: HashMap::new(),
                        line_entities: HashMap::new(),
                        mesh_keys: HashSet::new(),
                        raster_mesh_keys: HashSet::new(),
                        raster_texture_keys: HashSet::new(),
                        raster_material_keys: HashSet::new(),
                        camera,
                        render_layer: submitted.render_layer,
                    }
                });
                debug_assert_eq!(view.render_layer, submitted.render_layer);
                let mut retained = HashMap::with_capacity(submitted.frame.meshes.len());
                let mut used_meshes = HashSet::new();
                for prepared in submitted.frame.meshes {
                    let entity = view
                        .entities
                        .remove(&prepared.key)
                        .unwrap_or_else(|| commands.spawn_empty().id());
                    let mesh = mesh_for(&prepared.mesh, &mut render_assets, &mut meshes);
                    used_meshes.insert(prepared.mesh.key.clone());
                    commands.entity(entity).insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(render_assets.material.clone()),
                        prepared.transform,
                        render_layers.clone(),
                        PuzzleBevyRenderView {
                            id: submitted.view_id.clone(),
                        },
                        PuzzlePixelMesh {
                            key: prepared.key,
                            render_order: prepared.render_order,
                            object_ids: prepared.object_ids,
                        },
                    ));
                    retained.insert(prepared.key, entity);
                }
                let mut retained_rasters =
                    HashMap::with_capacity(submitted.frame.raster_quads.len());
                let mut used_raster_meshes = HashSet::new();
                let mut used_raster_textures = HashSet::new();
                let mut used_raster_materials = HashSet::new();
                for prepared in submitted.frame.raster_quads {
                    let entity = view
                        .raster_entities
                        .remove(&prepared.key)
                        .unwrap_or_else(|| commands.spawn_empty().id());
                    let mesh_key = raster_mesh_key(&prepared);
                    let mesh =
                        raster_mesh_for(&prepared, &mesh_key, &mut render_assets, &mut meshes);
                    let material_key = RasterMaterialKey {
                        texture: prepared.texture.key.clone(),
                        opacity: prepared.opacity.to_bits(),
                    };
                    let material = raster_material_for(
                        &prepared,
                        &material_key,
                        &submitted.catalog,
                        &mut render_assets,
                        &mut images,
                        &mut materials,
                    );
                    used_raster_meshes.insert(mesh_key);
                    used_raster_textures.insert(prepared.texture.key.clone());
                    used_raster_materials.insert(material_key);
                    commands.entity(entity).insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(material),
                        prepared.transform,
                        render_layers.clone(),
                        PuzzleBevyRenderView {
                            id: submitted.view_id.clone(),
                        },
                        PuzzleRasterQuad {
                            key: prepared.key,
                            render_order: prepared.render_order,
                            object_ids: prepared.object_ids,
                        },
                    ));
                    retained_rasters.insert(prepared.key, entity);
                }
                let mut retained_lines = HashMap::with_capacity(submitted.frame.line_meshes.len());
                for prepared in submitted.frame.line_meshes {
                    let entity = view
                        .line_entities
                        .remove(&prepared.key)
                        .unwrap_or_else(|| commands.spawn_empty().id());
                    let mesh = mesh_for(&prepared.mesh, &mut render_assets, &mut meshes);
                    used_meshes.insert(prepared.mesh.key.clone());
                    commands.entity(entity).insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(render_assets.material.clone()),
                        Transform::IDENTITY,
                        render_layers.clone(),
                        PuzzleBevyRenderView {
                            id: submitted.view_id.clone(),
                        },
                        PuzzleLineMesh2d { key: prepared.key },
                    ));
                    retained_lines.insert(prepared.key, entity);
                }
                for entity in view.entities.drain().map(|(_, entity)| entity) {
                    commands.entity(entity).despawn();
                }
                for entity in view.raster_entities.drain().map(|(_, entity)| entity) {
                    commands.entity(entity).despawn();
                }
                for entity in view.line_entities.drain().map(|(_, entity)| entity) {
                    commands.entity(entity).despawn();
                }
                commands.entity(view.camera).insert((
                    Camera {
                        is_active: submitted.view.active,
                        order: submitted.camera_order,
                        viewport: Some(submitted.view.framebuffer.viewport()),
                        clear_color: ClearColorConfig::Custom(submitted.view.clear_color.clone()),
                        ..default()
                    },
                    Projection::Orthographic(orthographic_projection(&submitted.view)),
                    camera_transform(&submitted.view),
                    render_layers,
                ));
                view.entities = retained;
                view.raster_entities = retained_rasters;
                view.line_entities = retained_lines;
                view.mesh_keys = used_meshes;
                view.raster_mesh_keys = used_raster_meshes;
                view.raster_texture_keys = used_raster_textures;
                view.raster_material_keys = used_raster_materials;
                view.generation = submitted.generation;
                state.views.insert(view_id, view);
            }
        }
    }
    let used_meshes = state
        .views
        .values()
        .flat_map(|view| view.mesh_keys.iter().cloned())
        .collect::<HashSet<_>>();
    render_assets.meshes.retain(|key, handle| {
        if used_meshes.contains(key) {
            true
        } else {
            meshes.remove(handle.id());
            false
        }
    });
    let used_raster_meshes = state
        .views
        .values()
        .flat_map(|view| view.raster_mesh_keys.iter().cloned())
        .collect::<HashSet<_>>();
    render_assets.raster_meshes.retain(|key, handle| {
        if used_raster_meshes.contains(key) {
            true
        } else {
            meshes.remove(handle.id());
            false
        }
    });
    let used_raster_materials = state
        .views
        .values()
        .flat_map(|view| view.raster_material_keys.iter().cloned())
        .collect::<HashSet<_>>();
    render_assets.raster_materials.retain(|key, handle| {
        if used_raster_materials.contains(key) {
            true
        } else {
            materials.remove(handle.id());
            false
        }
    });
    let used_raster_textures = state
        .views
        .values()
        .flat_map(|view| view.raster_texture_keys.iter().cloned())
        .collect::<HashSet<_>>();
    render_assets.raster_textures.retain(|key, handle| {
        if used_raster_textures.contains(key) {
            true
        } else {
            images.remove(handle.id());
            false
        }
    });
}

fn mesh_for(
    prepared: &PreparedLogicalMesh,
    render_assets: &mut RenderAssets2d,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    render_assets
        .meshes
        .entry(prepared.key.clone())
        .or_insert_with(|| {
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, prepared.positions.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, prepared.colors.clone());
            mesh.insert_indices(Indices::U32(prepared.indices.clone()));
            meshes.add(mesh)
        })
        .clone()
}

fn raster_mesh_key(prepared: &PreparedRasterQuad) -> RasterMeshKey {
    RasterMeshKey {
        positions: prepared
            .positions
            .iter()
            .map(|position| position.map(f32::to_bits))
            .collect(),
        uvs: prepared.uvs.iter().map(|uv| uv.map(f32::to_bits)).collect(),
        indices: prepared.indices.clone(),
    }
}

fn raster_mesh_for(
    prepared: &PreparedRasterQuad,
    key: &RasterMeshKey,
    render_assets: &mut RenderAssets2d,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    render_assets
        .raster_meshes
        .entry(key.clone())
        .or_insert_with(|| {
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, prepared.positions.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, prepared.uvs.clone());
            mesh.insert_indices(Indices::U32(prepared.indices.clone()));
            meshes.add(mesh)
        })
        .clone()
}

fn raster_material_for(
    prepared: &PreparedRasterQuad,
    material_key: &RasterMaterialKey,
    catalog: &DecodedVisualImageCatalog,
    render_assets: &mut RenderAssets2d,
    images: &mut Assets<Image>,
    materials: &mut Assets<ColorMaterial>,
) -> Handle<ColorMaterial> {
    let texture = render_assets
        .raster_textures
        .entry(prepared.texture.key.clone())
        .or_insert_with(|| {
            let asset = catalog
                .get(&prepared.texture.key.asset)
                .expect("prepared raster asset must remain in its submitted catalog");
            debug_assert_eq!(asset.revision, prepared.texture.key.revision);
            let mut image = Image::new(
                Extent3d {
                    width: u32::from(prepared.texture.width),
                    height: u32::from(prepared.texture.height),
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                asset.rgba8_srgb.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            let filter = match prepared.texture.key.sampling {
                RuntimeResolvedSampling::Pixelated => ImageFilterMode::Nearest,
                RuntimeResolvedSampling::Smooth => ImageFilterMode::Linear,
            };
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::ClampToEdge,
                address_mode_v: ImageAddressMode::ClampToEdge,
                address_mode_w: ImageAddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: filter,
                lod_min_clamp: 0.0,
                lod_max_clamp: 0.0,
                ..default()
            });
            images.add(image)
        })
        .clone();
    render_assets
        .raster_materials
        .entry(material_key.clone())
        .or_insert_with(|| {
            materials.add(ColorMaterial {
                color: Color::linear_rgba(1.0, 1.0, 1.0, prepared.opacity),
                alpha_mode: AlphaMode2d::Blend,
                texture: Some(texture),
                ..default()
            })
        })
        .clone()
}

fn orthographic_projection(settings: &PuzzleBevy2dView) -> OrthographicProjection {
    OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: settings.size.x,
            height: settings.size.y,
        },
        ..OrthographicProjection::default_2d()
    }
}

fn camera_transform(settings: &PuzzleBevy2dView) -> Transform {
    Transform::from_xyz(
        settings.origin.x + settings.size.x * 0.5,
        -(settings.origin.y + settings.size.y * 0.5),
        1000.0,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use bevy::{
        camera::RenderTarget,
        log::LogPlugin,
        render::{
            RenderPlugin,
            gpu_readback::{Readback, ReadbackComplete},
            render_resource::{PollType, TextureUsages},
            renderer::RenderDevice,
        },
        window::{ExitCondition, WindowPlugin},
        winit::WinitPlugin,
    };
    use image::{ImageEncoder, codecs::png::PngEncoder};
    use puzzle_assets::{
        EncodedVisualImageAsset, EncodedVisualImageBundle, VisualImageAssetManifestEntry,
        decode_visual_image_bundle,
    };
    use puzzle_presentation::resolve_palette_color;
    use puzzle_runtime_contract::{
        RuntimeLinearRgba, RuntimeResolvedDecoration, RuntimeResolvedLineLayer2d,
        RuntimeResolvedLineSegment2d, RuntimeResolvedLineStyle, RuntimeResolvedPixel,
        RuntimeResolvedPixelGeometry, RuntimeResolvedRect2d, RuntimeResolvedRenderBatch,
        RuntimeResolvedSampling, RuntimeResolvedStrokeWidth,
    };

    use super::*;

    const GPU_FRAMEBUFFER_PREREQUISITE_DIAGNOSTIC: &str = "GPU framebuffer contract test requires a native wgpu adapter; no compatible adapter was available";

    fn gpu_framebuffer_prerequisite<T, E>(probe: Result<T, E>) -> Result<T, &'static str> {
        probe.map_err(|_| GPU_FRAMEBUFFER_PREREQUISITE_DIAGNOSTIC)
    }

    fn require_gpu_framebuffer_adapter() {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::from_env().unwrap_or_default(),
            force_fallback_adapter: std::env::var("WGPU_FORCE_FALLBACK_ADAPTER")
                .is_ok_and(|value| !(value.is_empty() || value == "0" || value == "false")),
            compatible_surface: None,
        };
        let probe = bevy::tasks::block_on(instance.request_adapter(&options));
        gpu_framebuffer_prerequisite(probe).unwrap_or_else(|diagnostic| panic!("{diagnostic}"));
    }

    fn identity() -> [[f64; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn pixel_frame() -> RuntimeResolvedRenderFrame {
        RuntimeResolvedRenderFrame {
            batches: vec![RuntimeResolvedRenderBatch {
                render_order: 9,
                object_ids: vec![3],
                cell: [2, 4, 0],
                transform: identity(),
                opacity: 0.5,
                pixel_geometry: Some(RuntimeResolvedPixelGeometry {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                    clip: Some(RuntimeResolvedRect2d {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    }),
                }),
                content: RuntimeResolvedRenderBatchContent::Pixels {
                    width: 2,
                    height: 1,
                    pixels: vec![RuntimeResolvedPixel {
                        position: [1, 0],
                        color: RuntimeLinearRgba {
                            red: 1.0,
                            green: 0.0,
                            blue: 0.0,
                            alpha: 1.0,
                        },
                    }],
                },
            }],
            decorations: Vec::new(),
            continue_animation: false,
        }
    }

    fn view_2d(position: UVec2) -> PuzzleBevy2dView {
        PuzzleBevy2dView {
            active: true,
            order: 0,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position: position,
                physical_size: UVec2::new(320, 240),
            },
            clear_color: Color::BLACK,
            origin: Vec2::ZERO,
            size: Vec2::new(10.0, 8.0),
        }
    }

    fn empty_catalog() -> DecodedVisualImageCatalog {
        DecodedVisualImageCatalog::default()
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

    fn raster_frame(catalog: &DecodedVisualImageCatalog) -> RuntimeResolvedRenderFrame {
        let asset_id = VisualImageAssetManifestEntry::from_path("visuals/tile.png")
            .unwrap()
            .id;
        let asset = catalog.get(&asset_id).unwrap();
        RuntimeResolvedRenderFrame {
            batches: vec![RuntimeResolvedRenderBatch {
                render_order: 9,
                object_ids: vec![3],
                cell: [2, 4, 0],
                transform: identity(),
                opacity: 0.5,
                pixel_geometry: None,
                content: RuntimeResolvedRenderBatchContent::RasterImage {
                    asset: asset_id,
                    revision: asset.revision.clone(),
                    source_size: [asset.width, asset.height],
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
                },
            }],
            decorations: Vec::new(),
            continue_animation: false,
        }
    }

    #[test]
    fn prepares_direct_colored_geometry_for_logical_pixels() {
        let prepared =
            prepare_resolved_frame_2d(&pixel_frame(), &empty_catalog(), &view_2d(UVec2::ZERO))
                .unwrap();
        assert_eq!(prepared.meshes.len(), 1);
        let pixel_mesh = &prepared.meshes[0];
        assert_eq!(pixel_mesh.render_order, 9);
        assert_eq!(pixel_mesh.object_ids, vec![3]);
        assert_eq!(pixel_mesh.mesh.positions.len(), 4);
        assert_eq!(pixel_mesh.mesh.indices, vec![0, 2, 1, 0, 3, 2]);
        assert_eq!(pixel_mesh.mesh.colors[0], [1.0, 0.0, 0.0, 0.5]);
        assert!(
            pixel_mesh
                .transform
                .translation
                .abs_diff_eq(Vec3::new(2.5, -4.5, 0.0), super::super::MATRIX_EPSILON)
        );
    }

    #[test]
    fn raster_and_same_lattice_logical_pixels_share_geometry_and_instance_observables() {
        let rgba = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 128],
        ];
        let catalog = decoded_png_catalog("visuals/tile.png", &rgba, 2, 2);
        let raster =
            prepare_resolved_frame_2d(&raster_frame(&catalog), &catalog, &view_2d(UVec2::ZERO))
                .unwrap();
        let mut logical_frame = pixel_frame();
        logical_frame.batches[0].content = RuntimeResolvedRenderBatchContent::Pixels {
            width: 2,
            height: 2,
            pixels: (0..2)
                .flat_map(|y| {
                    (0..2).map(move |x| RuntimeResolvedPixel {
                        position: [x, y],
                        color: RuntimeLinearRgba {
                            red: 1.0,
                            green: 1.0,
                            blue: 1.0,
                            alpha: 1.0,
                        },
                    })
                })
                .collect(),
        };
        let logical =
            prepare_resolved_frame_2d(&logical_frame, &empty_catalog(), &view_2d(UVec2::ZERO))
                .unwrap();
        let raster = &raster.raster_quads[0];
        let logical = &logical.meshes[0];
        assert_eq!(raster.transform, logical.transform);
        assert_eq!(raster.render_order, logical.render_order);
        assert_eq!(raster.object_ids, logical.object_ids);
        assert_eq!(raster.opacity, 0.5);
        let bounds = |positions: &[[f32; 3]]| {
            positions.iter().fold(
                [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
                |[left, top, right, bottom], position| {
                    [
                        left.min(position[0]),
                        top.min(position[1]),
                        right.max(position[0]),
                        bottom.max(position[1]),
                    ]
                },
            )
        };
        assert_eq!(bounds(&raster.positions), bounds(&logical.mesh.positions));
        assert_eq!(
            raster.uvs,
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        );
        assert_eq!(
            raster.texture.key.sampling,
            RuntimeResolvedSampling::Pixelated
        );
        assert_eq!(
            catalog.get(&raster.texture.key.asset).unwrap().rgba8_srgb,
            rgba.into_iter().flatten().collect::<Vec<_>>()
        );
    }

    #[test]
    fn logical_cover_clip_matches_raster_destination_edges() {
        let mut logical_frame = pixel_frame();
        logical_frame.batches[0].pixel_geometry = Some(RuntimeResolvedPixelGeometry {
            x: -0.5,
            y: 0.0,
            width: 2.0,
            height: 1.0,
            clip: Some(RuntimeResolvedRect2d {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }),
        });
        logical_frame.batches[0].content = RuntimeResolvedRenderBatchContent::Pixels {
            width: 1,
            height: 1,
            pixels: vec![RuntimeResolvedPixel {
                position: [0, 0],
                color: RuntimeLinearRgba {
                    red: 1.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
        };
        let prepared =
            prepare_resolved_frame_2d(&logical_frame, &empty_catalog(), &view_2d(UVec2::ZERO))
                .unwrap();
        assert_eq!(
            prepared.meshes[0].mesh.positions,
            vec![
                [-0.5, 0.5, 0.0],
                [0.5, 0.5, 0.0],
                [0.5, -0.5, 0.0],
                [-0.5, -0.5, 0.0],
            ]
        );
    }

    #[test]
    fn raster_prepare_rejects_missing_and_stale_catalog_entries() {
        let catalog = decoded_png_catalog("visuals/tile.png", &[[255, 0, 0, 255]], 1, 1);
        let frame = raster_frame(&catalog);
        let asset_id = VisualImageAssetManifestEntry::from_path("visuals/tile.png")
            .unwrap()
            .id;
        assert_eq!(
            prepare_resolved_frame_2d(&frame, &empty_catalog(), &view_2d(UVec2::ZERO)).unwrap_err(),
            BevyRenderError::MissingRasterAsset {
                batch_index: 0,
                asset: asset_id.clone(),
            }
        );
        let mut stale = frame;
        let RuntimeResolvedRenderBatchContent::RasterImage { revision, .. } =
            &mut stale.batches[0].content
        else {
            unreachable!()
        };
        let actual = revision.clone();
        *revision = VisualImageAssetRevision("stale".to_string());
        assert_eq!(
            prepare_resolved_frame_2d(&stale, &catalog, &view_2d(UVec2::ZERO)).unwrap_err(),
            BevyRenderError::RasterAssetRevisionMismatch {
                batch_index: 0,
                asset: asset_id,
                expected: VisualImageAssetRevision("stale".to_string()),
                actual,
            }
        );
    }

    #[test]
    fn adjacent_pixels_share_exact_mesh_edge_coordinates() {
        let mut frame = pixel_frame();
        frame.batches[0].content = RuntimeResolvedRenderBatchContent::Pixels {
            width: 7,
            height: 1,
            pixels: vec![
                RuntimeResolvedPixel {
                    position: [2, 0],
                    color: RuntimeLinearRgba {
                        red: 1.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 1.0,
                    },
                },
                RuntimeResolvedPixel {
                    position: [3, 0],
                    color: RuntimeLinearRgba {
                        red: 1.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 1.0,
                    },
                },
            ],
        };

        let prepared =
            prepare_resolved_frame_2d(&frame, &empty_catalog(), &view_2d(UVec2::ZERO)).unwrap();
        let positions = &prepared.meshes[0].mesh.positions;
        assert_eq!(positions[1][0].to_bits(), positions[4][0].to_bits());
        assert_eq!(positions[2][0].to_bits(), positions[7][0].to_bits());
    }

    #[test]
    fn rejects_voxels_at_the_2d_backend_boundary() {
        let mut frame = pixel_frame();
        frame.batches[0].pixel_geometry = None;
        frame.batches[0].content = RuntimeResolvedRenderBatchContent::Voxels {
            width: 1,
            depth: 1,
            height: 1,
            voxels: Vec::new(),
        };
        assert_eq!(
            prepare_resolved_frame_2d(&frame, &empty_catalog(), &view_2d(UVec2::ZERO)).unwrap_err(),
            BevyRenderError::UnsupportedPrimitive {
                batch_index: 0,
                kind: "voxel",
            }
        );
    }

    #[test]
    fn resolves_overlay_quads_to_the_requested_physical_width_on_both_axes() {
        let mut frame = pixel_frame();
        frame.batches.clear();
        frame.decorations = vec![RuntimeResolvedDecoration::Lines2d {
            segments: vec![
                RuntimeResolvedLineSegment2d {
                    start: [2.0, 1.0],
                    end: [2.0, 6.0],
                },
                RuntimeResolvedLineSegment2d {
                    start: [1.0, 7.0],
                    end: [8.0, 7.0],
                },
            ],
            style: RuntimeResolvedLineStyle {
                color: RuntimeLinearRgba {
                    red: 0.25,
                    green: 0.5,
                    blue: 0.75,
                    alpha: 0.5,
                },
                width: RuntimeResolvedStrokeWidth::CellRelative {
                    cell_fraction: 0.01,
                    min_physical_pixels: 3.0,
                },
            },
            layer: RuntimeResolvedLineLayer2d::Overlay,
        }];
        let view = view_2d(UVec2::ZERO);

        let prepared = prepare_resolved_frame_2d(&frame, &empty_catalog(), &view).unwrap();

        assert_eq!(prepared.line_meshes.len(), 1);
        let mesh = &prepared.line_meshes[0].mesh;
        let vertical_world_width = mesh.positions[1][0] - mesh.positions[0][0];
        let horizontal_world_width = mesh.positions[4][1] - mesh.positions[7][1];
        assert!((vertical_world_width.abs() * 32.0 - 3.0).abs() < 0.000_1);
        assert!((horizontal_world_width.abs() * 30.0 - 3.0).abs() < 0.000_1);
        assert_eq!(mesh.colors[0], [0.25, 0.5, 0.75, 0.5]);
        assert!(
            mesh.positions
                .iter()
                .all(|position| position[2] == MESH_Z_STEP)
        );
    }

    #[test]
    fn translucent_grid_intersections_have_single_fragment_coverage() {
        let mut frame = pixel_frame();
        frame.batches.clear();
        frame.decorations = vec![RuntimeResolvedDecoration::Lines2d {
            segments: vec![
                RuntimeResolvedLineSegment2d {
                    start: [2.0, 1.0],
                    end: [2.0, 6.0],
                },
                RuntimeResolvedLineSegment2d {
                    start: [1.0, 3.0],
                    end: [8.0, 3.0],
                },
            ],
            style: RuntimeResolvedLineStyle {
                color: RuntimeLinearRgba {
                    red: 1.0,
                    green: 1.0,
                    blue: 1.0,
                    alpha: 0.34,
                },
                width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 4.0 },
            },
            layer: RuntimeResolvedLineLayer2d::Overlay,
        }];
        let view = view_2d(UVec2::ZERO);
        let prepared = prepare_resolved_frame_2d(&frame, &empty_catalog(), &view).unwrap();
        let positions = &prepared.line_meshes[0].mesh.positions;
        let rectangles = positions
            .chunks_exact(4)
            .map(|corners| {
                let min = Vec2::new(
                    corners
                        .iter()
                        .map(|point| point[0])
                        .fold(f32::INFINITY, f32::min),
                    corners
                        .iter()
                        .map(|point| -point[1])
                        .fold(f32::INFINITY, f32::min),
                );
                let max = Vec2::new(
                    corners
                        .iter()
                        .map(|point| point[0])
                        .fold(f32::NEG_INFINITY, f32::max),
                    corners
                        .iter()
                        .map(|point| -point[1])
                        .fold(f32::NEG_INFINITY, f32::max),
                );
                LineRect2d { min, max }
            })
            .collect::<Vec<_>>();
        for (index, rectangle) in rectangles.iter().enumerate() {
            for other in &rectangles[index + 1..] {
                let overlap = (rectangle.max - other.min)
                    .min(other.max - rectangle.min)
                    .max(Vec2::ZERO);
                assert!(overlap.x == 0.0 || overlap.y == 0.0);
            }
        }
        let scale = Vec2::new(32.0, 30.0);
        let covered_physical_area = rectangles
            .iter()
            .map(|rectangle| {
                let size = (rectangle.max - rectangle.min) * scale;
                size.x * size.y
            })
            .sum::<f32>();
        assert!((covered_physical_area - (600.0 + 896.0 - 16.0)).abs() < 0.01);
    }

    #[test]
    fn resolved_view_uses_both_authored_axes_and_origin() {
        let view = PuzzleBevy2dView {
            origin: Vec2::new(4.0, 7.0),
            size: Vec2::new(6.0, 3.0),
            ..view_2d(UVec2::ZERO)
        };
        assert!(matches!(
            orthographic_projection(&view).scaling_mode,
            ScalingMode::Fixed { width, height } if width == 6.0 && height == 3.0
        ));
        assert_eq!(
            camera_transform(&view).translation,
            Vec3::new(7.0, -8.5, 1000.0)
        );
    }

    #[test]
    fn keyed_views_keep_overlay_lines_on_disjoint_render_layers() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_plugins(PuzzleBevy2dPlugin);
        let mut frame = pixel_frame();
        frame.batches.clear();
        frame.decorations = vec![RuntimeResolvedDecoration::Lines2d {
            segments: vec![RuntimeResolvedLineSegment2d {
                start: [0.0, 0.0],
                end: [1.0, 0.0],
            }],
            style: RuntimeResolvedLineStyle {
                color: RuntimeLinearRgba {
                    red: 1.0,
                    green: 1.0,
                    blue: 1.0,
                    alpha: 1.0,
                },
                width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 1.0 },
            },
            layer: RuntimeResolvedLineLayer2d::Overlay,
        }];
        let left = PuzzleBevyViewId::two_d("left", "main");
        let right = PuzzleBevyViewId::two_d("right", "main");
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            left.clone(),
            view_2d(UVec2::ZERO),
            &frame,
        )
        .unwrap();
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            right.clone(),
            PuzzleBevy2dView {
                order: 1,
                ..view_2d(UVec2::new(320, 0))
            },
            &frame,
        )
        .unwrap();
        app.update();

        let layers = app
            .world_mut()
            .query::<(&PuzzleBevyRenderView, &PuzzleLineMesh2d, &RenderLayers)>()
            .iter(app.world())
            .map(|(view, _, layers)| (view.id.clone(), layers.iter().collect::<Vec<_>>()))
            .collect::<HashMap<_, _>>();
        assert_ne!(layers[&left], layers[&right]);

        remove_render_view_2d(app.world_mut(), &left).unwrap();
        app.update();
        let remaining = app
            .world_mut()
            .query::<(&PuzzleBevyRenderView, &PuzzleLineMesh2d)>()
            .iter(app.world())
            .map(|(view, _)| view.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![right]);
    }

    #[test]
    fn ecs_sync_reuses_mesh_entities_and_assets() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_plugins(PuzzleBevy2dPlugin::default());
        let frame = pixel_frame();
        let view_id = PuzzleBevyViewId::two_d("board", "main");
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            view_id.clone(),
            view_2d(UVec2::ZERO),
            &frame,
        )
        .unwrap();
        app.update();
        let first = app
            .world_mut()
            .query_filtered::<Entity, With<PuzzlePixelMesh>>()
            .single(app.world())
            .unwrap();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);

        submit_image_free_resolved_frame_2d(app.world_mut(), view_id, view_2d(UVec2::ZERO), &frame)
            .unwrap();
        app.update();
        let second = app
            .world_mut()
            .query_filtered::<Entity, With<PuzzlePixelMesh>>()
            .single(app.world())
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
    }

    #[test]
    fn raster_upload_is_srgb_straight_alpha_explicitly_sampled_and_revision_cached() {
        let first_rgba = [[255, 0, 128, 255], [1, 2, 3, 128]];
        let first_catalog = Arc::new(decoded_png_catalog("visuals/tile.png", &first_rgba, 2, 1));
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_plugins(PuzzleBevy2dPlugin::default());
        let view_id = PuzzleBevyViewId::two_d("board", "main");
        submit_resolved_frame_2d(
            app.world_mut(),
            view_id.clone(),
            view_2d(UVec2::ZERO),
            first_catalog.clone(),
            &raster_frame(&first_catalog),
        )
        .unwrap();
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .iter()
            .next()
            .unwrap()
            .1;
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(
            image.data.as_deref(),
            Some(
                first_rgba
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .as_slice()
            )
        );
        let ImageSampler::Descriptor(sampler) = &image.sampler else {
            panic!("raster image must own an explicit sampler");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::ClampToEdge);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::ClampToEdge);
        assert_eq!(sampler.mag_filter, ImageFilterMode::Nearest);
        assert_eq!(sampler.min_filter, ImageFilterMode::Nearest);
        assert_eq!(sampler.mipmap_filter, ImageFilterMode::Nearest);
        assert_eq!(sampler.lod_min_clamp, 0.0);
        assert_eq!(sampler.lod_max_clamp, 0.0);
        let material = app
            .world()
            .resource::<Assets<ColorMaterial>>()
            .iter()
            .find_map(|(_, material)| material.texture.is_some().then_some(material))
            .unwrap();
        assert_eq!(material.color, Color::linear_rgba(1.0, 1.0, 1.0, 0.5));
        assert_eq!(material.alpha_mode, AlphaMode2d::Blend);

        submit_resolved_frame_2d(
            app.world_mut(),
            view_id.clone(),
            view_2d(UVec2::ZERO),
            first_catalog.clone(),
            &raster_frame(&first_catalog),
        )
        .unwrap();
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);

        let second_rgba = [[0, 255, 0, 255], [0, 0, 255, 255]];
        let second_catalog = Arc::new(decoded_png_catalog("visuals/tile.png", &second_rgba, 2, 1));
        submit_resolved_frame_2d(
            app.world_mut(),
            view_id,
            view_2d(UVec2::ZERO),
            second_catalog.clone(),
            &raster_frame(&second_catalog),
        )
        .unwrap();
        app.update();
        let images = app.world().resource::<Assets<Image>>();
        assert_eq!(images.len(), 1);
        assert_eq!(
            images.iter().next().unwrap().1.data.as_deref(),
            Some(
                second_rgba
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .as_slice()
            )
        );
    }

    #[test]
    fn keyed_views_own_disjoint_cameras_meshes_and_removal() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_plugins(PuzzleBevy2dPlugin::default());
        let left = PuzzleBevyViewId::two_d("left-board", "main");
        let right = PuzzleBevyViewId::two_d("right-board", "main");
        let frame = pixel_frame();
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            left.clone(),
            view_2d(UVec2::ZERO),
            &frame,
        )
        .unwrap();
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            right.clone(),
            PuzzleBevy2dView {
                order: 1,
                ..view_2d(UVec2::new(320, 0))
            },
            &frame,
        )
        .unwrap();
        app.update();

        let right_entity = app
            .world_mut()
            .query::<(Entity, &PuzzleBevyRenderView, &PuzzlePixelMesh)>()
            .iter(app.world())
            .find_map(|(entity, view, _)| (view.id == right).then_some(entity))
            .unwrap();
        let cameras = app
            .world_mut()
            .query_filtered::<
                (&Camera, &PuzzleBevyRenderView, &RenderLayers),
                With<PuzzleRendererCamera2d>,
            >()
            .iter(app.world())
            .map(|(camera, view, layers)| {
                (
                    view.id.clone(),
                    camera.viewport.as_ref().unwrap().physical_position,
                    camera.order,
                    layers.iter().collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(cameras.len(), 2);
        assert!(
            cameras
                .iter()
                .any(|(id, position, _, _)| { id == &left && *position == UVec2::ZERO })
        );
        assert!(
            cameras
                .iter()
                .any(|(id, position, _, _)| { id == &right && *position == UVec2::new(320, 0) })
        );
        assert_ne!(cameras[0].2, cameras[1].2);
        assert_ne!(cameras[0].3, cameras[1].3);
        let right_entity_layers = app
            .world_mut()
            .query::<(&PuzzleBevyRenderView, &PuzzlePixelMesh, &RenderLayers)>()
            .iter(app.world())
            .find_map(|(view, _, layers)| {
                (view.id == right).then(|| layers.iter().collect::<Vec<_>>())
            })
            .unwrap();
        let right_camera_layers = cameras
            .iter()
            .find_map(|(id, _, _, layers)| (id == &right).then_some(layers))
            .unwrap();
        assert_eq!(&right_entity_layers, right_camera_layers);

        remove_render_view_2d(app.world_mut(), &left).unwrap();
        app.update();
        {
            let queue = app.world().resource::<BevyResolvedFrameQueue2d>();
            assert!(!queue.registry.view_layers.contains_key(&left));
            assert_eq!(queue.registry.free_layers.len(), 1);
        }

        let remaining = app
            .world_mut()
            .query::<(Entity, &PuzzleBevyRenderView, &PuzzlePixelMesh)>()
            .iter(app.world())
            .map(|(entity, view, _)| (entity, view.id.clone()))
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![(right_entity, right.clone())]);
        let remaining_cameras = app
            .world_mut()
            .query_filtered::<&PuzzleBevyRenderView, With<PuzzleRendererCamera2d>>()
            .iter(app.world())
            .map(|view| view.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(remaining_cameras, vec![right]);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
        assert_eq!(
            remove_render_view_2d(app.world_mut(), &left),
            Err(BevyRenderError::UnknownView { view_id: left })
        );
    }

    #[test]
    fn rejects_zero_sized_framebuffer_before_queueing_a_view() {
        let mut queue = BevyResolvedFrameQueue2d::default();
        let error = queue
            .submit(
                PuzzleBevyViewId::two_d("board", "main"),
                PuzzleBevy2dView {
                    framebuffer: PuzzleBevyFramebufferRect {
                        physical_position: UVec2::ZERO,
                        physical_size: UVec2::ZERO,
                    },
                    ..default()
                },
                Arc::new(empty_catalog()),
                &pixel_frame(),
            )
            .unwrap_err();

        assert_eq!(error, BevyRenderError::InvalidFramebufferRect);
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn removal_releases_2d_view_reservations_before_deferred_ecs_cleanup() {
        let mut queue = BevyResolvedFrameQueue2d::default();
        let outgoing = PuzzleBevyViewId::two_d("outgoing", "main");
        let incoming = PuzzleBevyViewId::two_d("incoming", "main");
        let catalog = Arc::new(empty_catalog());

        queue
            .submit(
                outgoing.clone(),
                view_2d(UVec2::ZERO),
                catalog.clone(),
                &pixel_frame(),
            )
            .unwrap();
        queue.remove(&outgoing).unwrap();
        queue
            .submit(
                incoming.clone(),
                view_2d(UVec2::ZERO),
                catalog,
                &pixel_frame(),
            )
            .unwrap();

        assert!(!queue.registry.registered_views.contains(&outgoing));
        assert!(queue.registry.registered_views.contains(&incoming));
        assert_eq!(
            queue.registry.order_owners.get(&1),
            Some(&incoming),
            "the accepted removal must end logical camera ownership immediately"
        );
    }

    #[test]
    fn retained_2d_views_can_swap_camera_orders_in_one_submission() {
        let mut queue = BevyResolvedFrameQueue2d::default();
        let left = PuzzleBevyViewId::two_d("left", "main");
        let right = PuzzleBevyViewId::two_d("right", "main");
        let catalog = Arc::new(empty_catalog());
        queue
            .submit(
                left.clone(),
                view_2d(UVec2::ZERO),
                catalog.clone(),
                &pixel_frame(),
            )
            .unwrap();
        queue
            .submit(
                right.clone(),
                PuzzleBevy2dView {
                    order: 1,
                    ..view_2d(UVec2::new(320, 0))
                },
                catalog.clone(),
                &pixel_frame(),
            )
            .unwrap();

        queue
            .reconcile_camera_orders(&BTreeMap::from([(left.clone(), 1), (right.clone(), 0)]))
            .unwrap();
        queue
            .submit(
                left.clone(),
                PuzzleBevy2dView {
                    order: 1,
                    ..view_2d(UVec2::ZERO)
                },
                catalog.clone(),
                &pixel_frame(),
            )
            .unwrap();
        queue
            .submit(
                right.clone(),
                view_2d(UVec2::new(320, 0)),
                catalog,
                &pixel_frame(),
            )
            .unwrap();
        assert_eq!(queue.registry.order_owners.get(&1), Some(&right));
        assert_eq!(queue.registry.order_owners.get(&3), Some(&left));
    }

    #[test]
    fn gpu_framebuffer_prerequisite_reports_a_stable_missing_adapter_diagnostic() {
        assert_eq!(
            gpu_framebuffer_prerequisite::<(), ()>(Err(())),
            Err(GPU_FRAMEBUFFER_PREREQUISITE_DIAGNOSTIC)
        );
    }

    #[test]
    fn gpu_framebuffer_matches_for_png_and_the_same_ascii_colors_under_cover() {
        require_gpu_framebuffer_adapter();

        const VIEW_SIZE: u32 = 8;
        const TARGET_WIDTH: u32 = VIEW_SIZE * 2;
        const TARGET_HEIGHT: u32 = VIEW_SIZE;

        let declarations = [
            "#ff0000ff",
            "#336699ff",
            "#ff800080",
            "#0000ffff",
            "#ffff00ff",
            "#00ff40c0",
            "#00000000",
            "#ff00ffff",
        ];
        let rgba = [
            [0xff, 0x00, 0x00, 0xff],
            [0x33, 0x66, 0x99, 0xff],
            [0xff, 0x80, 0x00, 0x80],
            [0x00, 0x00, 0xff, 0xff],
            [0xff, 0xff, 0x00, 0xff],
            [0x00, 0xff, 0x40, 0xc0],
            [0x00, 0x00, 0x00, 0x00],
            [0xff, 0x00, 0xff, 0xff],
        ];
        let catalog = Arc::new(decoded_png_catalog("visuals/equivalent.png", &rgba, 4, 2));
        let asset_id = VisualImageAssetManifestEntry::from_path("visuals/equivalent.png")
            .unwrap()
            .id;
        let asset = catalog.get(&asset_id).unwrap();
        let common_batch = RuntimeResolvedRenderBatch {
            render_order: 0,
            object_ids: vec![1],
            cell: [0, 0, 0],
            transform: identity(),
            opacity: 1.0,
            pixel_geometry: None,
            content: RuntimeResolvedRenderBatchContent::Pixels {
                width: 1,
                height: 1,
                pixels: Vec::new(),
            },
        };
        let logical_frame = RuntimeResolvedRenderFrame {
            batches: vec![RuntimeResolvedRenderBatch {
                pixel_geometry: Some(RuntimeResolvedPixelGeometry {
                    x: -0.5,
                    y: 0.0,
                    width: 2.0,
                    height: 1.0,
                    clip: Some(RuntimeResolvedRect2d {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    }),
                }),
                content: RuntimeResolvedRenderBatchContent::Pixels {
                    width: 4,
                    height: 2,
                    pixels: declarations
                        .iter()
                        .enumerate()
                        .map(|(index, declaration)| RuntimeResolvedPixel {
                            position: [(index % 4) as i32, (index / 4) as i32],
                            color: resolve_palette_color(declaration).unwrap(),
                        })
                        .collect(),
                },
                ..common_batch.clone()
            }],
            decorations: Vec::new(),
            continue_animation: false,
        };
        let raster_frame = RuntimeResolvedRenderFrame {
            batches: vec![RuntimeResolvedRenderBatch {
                content: RuntimeResolvedRenderBatchContent::RasterImage {
                    asset: asset_id,
                    revision: asset.revision.clone(),
                    source_size: [asset.width, asset.height],
                    destination: RuntimeResolvedRect2d {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    },
                    uv: RuntimeResolvedRect2d {
                        x: 0.25,
                        y: 0.0,
                        width: 0.5,
                        height: 1.0,
                    },
                    sampling: RuntimeResolvedSampling::Pixelated,
                },
                ..common_batch
            }],
            decorations: Vec::new(),
            continue_animation: false,
        };

        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .disable::<LogPlugin>()
                .disable::<WinitPlugin>(),
        )
        .add_plugins(PuzzleBevy2dPlugin);
        app.finish();
        app.cleanup();

        let mut render_target = Image::new_target_texture(
            TARGET_WIDTH,
            TARGET_HEIGHT,
            TextureFormat::Rgba8UnormSrgb,
            None,
        );
        render_target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let render_target = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(render_target);

        let captures = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let observer_captures = captures.clone();
        app.world_mut()
            .spawn(Readback::texture(render_target.clone()))
            .observe(move |event: On<ReadbackComplete>| {
                observer_captures.lock().unwrap().push(event.data.clone());
            });

        let equivalence_view = |physical_position| PuzzleBevy2dView {
            active: true,
            order: 0,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position,
                physical_size: UVec2::splat(VIEW_SIZE),
            },
            clear_color: Color::BLACK,
            origin: Vec2::ZERO,
            size: Vec2::ONE,
        };
        submit_image_free_resolved_frame_2d(
            app.world_mut(),
            PuzzleBevyViewId::two_d("ascii", "main"),
            equivalence_view(UVec2::ZERO),
            &logical_frame,
        )
        .unwrap();
        submit_resolved_frame_2d(
            app.world_mut(),
            PuzzleBevyViewId::two_d("png", "main"),
            PuzzleBevy2dView {
                order: 1,
                ..equivalence_view(UVec2::new(VIEW_SIZE, 0))
            },
            catalog,
            &raster_frame,
        )
        .unwrap();

        // The first update materializes the backend-owned cameras. Both cameras
        // then render to the same texture through disjoint physical viewports.
        app.update();
        let cameras = app
            .world_mut()
            .query_filtered::<Entity, With<PuzzleRendererCamera2d>>()
            .iter(app.world())
            .collect::<Vec<_>>();
        for camera in cameras {
            app.world_mut()
                .entity_mut(camera)
                .insert((RenderTarget::Image(render_target.clone().into()), Msaa::Off));
        }

        let packed_row_bytes = TARGET_WIDTH as usize * 4;
        let padded_row_bytes = RenderDevice::align_copy_bytes_per_row(packed_row_bytes);
        let mut observed = None;
        for _ in 0..120 {
            app.update();
            app.world()
                .resource::<RenderDevice>()
                .wgpu_device()
                .poll(PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .unwrap();
            let guard = captures.lock().unwrap();
            for capture in guard.iter().rev() {
                let packed = capture
                    .chunks_exact(padded_row_bytes)
                    .take(TARGET_HEIGHT as usize)
                    .flat_map(|row| row[..packed_row_bytes].iter().copied())
                    .collect::<Vec<_>>();
                let rows_match = packed
                    .chunks_exact(packed_row_bytes)
                    .all(|row| row[..VIEW_SIZE as usize * 4] == row[VIEW_SIZE as usize * 4..]);
                let distinct_left_pixels = packed
                    .chunks_exact(packed_row_bytes)
                    .flat_map(|row| row[..VIEW_SIZE as usize * 4].chunks_exact(4))
                    .map(<[u8; 4]>::try_from)
                    .collect::<Result<HashSet<_>, _>>()
                    .unwrap();
                if rows_match && distinct_left_pixels.len() >= 4 {
                    observed = Some(packed);
                    break;
                }
            }
            if observed.is_some() {
                break;
            }
        }

        let observed = observed.expect(
            "GPU readback must observe a rendered frame whose PNG and ASCII viewports match",
        );
        let distinct_left_pixels = observed
            .chunks_exact(packed_row_bytes)
            .flat_map(|row| row[..VIEW_SIZE as usize * 4].chunks_exact(4))
            .map(<[u8; 4]>::try_from)
            .collect::<Result<HashSet<_>, _>>()
            .unwrap();
        assert!(
            distinct_left_pixels.len() >= 4,
            "the comparison must contain the authored opaque, translucent, and transparent colors; got {distinct_left_pixels:?}"
        );
    }
}
