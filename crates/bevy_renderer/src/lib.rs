use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fmt,
};

use bevy::{
    asset::RenderAssetUsages,
    camera::{ClearColorConfig, ScalingMode, Viewport, visibility::RenderLayers},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use puzzle_assets::{VisualImageAssetId, VisualImageAssetRevision};
use puzzle_runtime_contract::{
    RuntimeLinearRgba, RuntimeResolvedDecoration, RuntimeResolvedRenderBatchContent,
    RuntimeResolvedRenderFrame, RuntimeResolvedStrokeWidth,
};

mod pixelate;
mod two_d;

pub use two_d::{
    BevyResolvedFrameQueue2d, PreparedBevy2dFrame, PreparedLineMesh2d, PreparedLineMesh2dKey,
    PreparedPixelMesh, PreparedPixelMeshKey, PreparedRasterQuad, PreparedRasterQuadKey,
    PuzzleBevy2dPlugin, PuzzleBevy2dView, PuzzleLineMesh2d, PuzzlePixelMesh, PuzzleRasterQuad,
    prepare_resolved_frame_2d, remove_render_view_2d, submit_image_free_resolved_frame_2d,
    submit_resolved_frame_2d,
};

const OPAQUE_ALPHA: f32 = 0.999;
const MATRIX_EPSILON: f32 = 0.000_1;
const FIRST_3D_RENDER_LAYER: usize = 2;
const RENDER_LAYER_STRIDE: usize = 2;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PuzzleBevyRendererSystems {
    ApplySubmittedFrames,
}

fn camera_order(
    logical_order: isize,
    dimension: PuzzleBevyViewDimension,
) -> Result<isize, BevyRenderError> {
    logical_order
        .checked_mul(2)
        .and_then(|order| {
            order.checked_add(match dimension {
                PuzzleBevyViewDimension::ThreeD => 0,
                PuzzleBevyViewDimension::TwoD => 1,
            })
        })
        .ok_or(BevyRenderError::InvalidCameraOrder)
}

#[derive(Default)]
struct BevyViewRegistry {
    next_layer_slot: usize,
    free_layers: Vec<usize>,
    view_layers: HashMap<PuzzleBevyViewId, usize>,
    view_orders: HashMap<PuzzleBevyViewId, isize>,
    order_owners: HashMap<isize, PuzzleBevyViewId>,
    registered_views: HashSet<PuzzleBevyViewId>,
}

impl BevyViewRegistry {
    fn reconcile_camera_orders(
        &mut self,
        desired: &BTreeMap<PuzzleBevyViewId, isize>,
    ) -> Result<(), BevyRenderError> {
        let mut view_orders = HashMap::with_capacity(desired.len());
        let mut order_owners = HashMap::with_capacity(desired.len());
        for (view_id, logical_order) in desired {
            let order = camera_order(*logical_order, view_id.dimension)?;
            if order_owners.insert(order, view_id.clone()).is_some() {
                return Err(BevyRenderError::DuplicateCameraOrder { order });
            }
            view_orders.insert(view_id.clone(), order);
        }
        self.view_orders = view_orders;
        self.order_owners = order_owners;
        Ok(())
    }

    fn reserve(
        &mut self,
        view_id: &PuzzleBevyViewId,
        logical_order: isize,
        first_render_layer: usize,
    ) -> Result<(usize, isize), BevyRenderError> {
        let allocated_layer = !self.view_layers.contains_key(view_id);
        let render_layer = self.render_layer(view_id, first_render_layer)?;
        let camera_order = match self.reserve_camera_order(view_id, logical_order) {
            Ok(order) => order,
            Err(error) => {
                if allocated_layer && let Some(layer) = self.view_layers.remove(view_id) {
                    self.free_layers.push(layer);
                }
                return Err(error);
            }
        };
        self.registered_views.insert(view_id.clone());
        Ok((render_layer, camera_order))
    }

    fn validate_removal(&self, view_id: &PuzzleBevyViewId) -> Result<(), BevyRenderError> {
        if !self.registered_views.contains(view_id) {
            return Err(BevyRenderError::UnknownView {
                view_id: view_id.clone(),
            });
        }
        Ok(())
    }

    fn release_registered(&mut self, view_id: &PuzzleBevyViewId) {
        let removed = self.registered_views.remove(view_id);
        debug_assert!(removed);
        self.release_camera_order(view_id);
    }

    fn render_layer(
        &mut self,
        view_id: &PuzzleBevyViewId,
        first_render_layer: usize,
    ) -> Result<usize, BevyRenderError> {
        if let Some(layer) = self.view_layers.get(view_id) {
            return Ok(*layer);
        }
        let layer = if let Some(layer) = self.free_layers.pop() {
            layer
        } else {
            let layer = self
                .next_layer_slot
                .checked_mul(RENDER_LAYER_STRIDE)
                .and_then(|offset| first_render_layer.checked_add(offset))
                .ok_or(BevyRenderError::ViewLayerExhausted)?;
            self.next_layer_slot = self
                .next_layer_slot
                .checked_add(1)
                .ok_or(BevyRenderError::ViewLayerExhausted)?;
            layer
        };
        self.view_layers.insert(view_id.clone(), layer);
        Ok(layer)
    }

    fn reserve_camera_order(
        &mut self,
        view_id: &PuzzleBevyViewId,
        logical_order: isize,
    ) -> Result<isize, BevyRenderError> {
        let order = camera_order(logical_order, view_id.dimension)?;
        if let Some(owner) = self.order_owners.get(&order)
            && owner != view_id
        {
            return Err(BevyRenderError::DuplicateCameraOrder { order });
        }
        if let Some(previous) = self.view_orders.insert(view_id.clone(), order)
            && previous != order
        {
            self.order_owners.remove(&previous);
        }
        self.order_owners.insert(order, view_id.clone());
        Ok(order)
    }

    fn finish_removal(&mut self, view_id: &PuzzleBevyViewId) {
        debug_assert!(!self.registered_views.contains(view_id));
        if let Some(layer) = self.view_layers.remove(view_id) {
            self.free_layers.push(layer);
        }
    }

    fn release_camera_order(&mut self, view_id: &PuzzleBevyViewId) {
        if let Some(order) = self.view_orders.remove(view_id) {
            self.order_owners.remove(&order);
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PuzzleBevyViewId {
    pub component_instance: String,
    pub source: String,
    pub dimension: PuzzleBevyViewDimension,
}

impl PuzzleBevyViewId {
    pub fn two_d(component_instance: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            component_instance: component_instance.into(),
            source: source.into(),
            dimension: PuzzleBevyViewDimension::TwoD,
        }
    }

    pub fn three_d(component_instance: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            component_instance: component_instance.into(),
            source: source.into(),
            dimension: PuzzleBevyViewDimension::ThreeD,
        }
    }

    fn validate(&self) -> Result<(), BevyRenderError> {
        if self.component_instance.is_empty() {
            return Err(BevyRenderError::InvalidViewId {
                field: "component_instance",
            });
        }
        if self.source.is_empty() {
            return Err(BevyRenderError::InvalidViewId { field: "source" });
        }
        Ok(())
    }

    fn validate_dimension(&self, expected: PuzzleBevyViewDimension) -> Result<(), BevyRenderError> {
        if self.dimension != expected {
            return Err(BevyRenderError::ViewDimensionMismatch {
                view_id: self.clone(),
                expected,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PuzzleBevyFramebufferRect {
    pub physical_position: UVec2,
    pub physical_size: UVec2,
}

impl PuzzleBevyFramebufferRect {
    fn validate(self) -> Result<(), BevyRenderError> {
        if self.physical_size.x == 0 || self.physical_size.y == 0 {
            return Err(BevyRenderError::InvalidFramebufferRect);
        }
        Ok(())
    }

    fn viewport(self) -> Viewport {
        Viewport {
            physical_position: self.physical_position,
            physical_size: self.physical_size,
            ..default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PuzzleBevyViewDimension {
    TwoD,
    ThreeD,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct PuzzleBevyRenderView {
    pub id: PuzzleBevyViewId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzleCameraProjection {
    Perspective,
    Orthographic,
}

#[derive(Clone, Debug)]
pub struct PuzzleBevyCamera {
    pub projection: PuzzleCameraProjection,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub roll_degrees: f32,
    pub distance_scale: f32,
}

impl Default for PuzzleBevyCamera {
    fn default() -> Self {
        Self {
            projection: PuzzleCameraProjection::Perspective,
            yaw_degrees: 35.0,
            pitch_degrees: 35.0,
            roll_degrees: 0.0,
            distance_scale: 2.8,
        }
    }
}

impl PuzzleBevyCamera {
    fn validate(&self) -> Result<(), BevyRenderError> {
        for (field, value) in [
            ("yaw_degrees", self.yaw_degrees),
            ("pitch_degrees", self.pitch_degrees),
            ("roll_degrees", self.roll_degrees),
            ("distance_scale", self.distance_scale),
        ] {
            if !value.is_finite() {
                return Err(BevyRenderError::InvalidCamera { field });
            }
        }
        if self.pitch_degrees <= -89.0 || self.pitch_degrees >= 89.0 {
            return Err(BevyRenderError::InvalidCamera {
                field: "pitch_degrees",
            });
        }
        if self.distance_scale <= 0.0 {
            return Err(BevyRenderError::InvalidCamera {
                field: "distance_scale",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PuzzleBevyLighting {
    pub intensity: f32,
    pub ambient: f32,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PuzzleBevyLightingError {
    pub field: &'static str,
}

impl fmt::Display for PuzzleBevyLightingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "PuzzleBevyLighting.{} must be a finite non-negative value",
            self.field
        )
    }
}

impl Error for PuzzleBevyLightingError {}

impl PuzzleBevyLighting {
    pub fn validate(&self) -> Result<(), PuzzleBevyLightingError> {
        for (field, value) in [("intensity", self.intensity), ("ambient", self.ambient)] {
            if !value.is_finite() || value < 0.0 {
                return Err(PuzzleBevyLightingError { field });
            }
        }
        for (field, value) in [
            ("yaw_degrees", self.yaw_degrees),
            ("pitch_degrees", self.pitch_degrees),
        ] {
            if !value.is_finite() {
                return Err(PuzzleBevyLightingError { field });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PuzzleBevy3dView {
    pub active: bool,
    pub order: isize,
    pub framebuffer: PuzzleBevyFramebufferRect,
    pub clear_color: Color,
    pub camera: PuzzleBevyCamera,
    pub lighting: PuzzleBevyLighting,
    pub shadows_enabled: bool,
    pub render_settings: PuzzleBevy3dRenderSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PuzzleBevy3dRenderSettings {
    pub shade: bool,
    pub pixelate: PuzzleBevyPixelate,
}

impl Default for PuzzleBevy3dRenderSettings {
    fn default() -> Self {
        Self {
            shade: true,
            pixelate: PuzzleBevyPixelate::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PuzzleBevyPixelate {
    pub enabled: bool,
    pub scale: u16,
    pub smoothing: bool,
}

impl Default for PuzzleBevyPixelate {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 4,
            smoothing: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PuzzleBevy3dPlugin;

impl Plugin for PuzzleBevy3dPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlobalAmbientLight::NONE)
            .add_plugins(pixelate::PuzzlePixelatePlugin)
            .init_resource::<BevyResolvedFrameQueue>()
            .init_resource::<RenderedFrameState>()
            .add_systems(Startup, setup_renderer)
            .add_systems(
                PostUpdate,
                apply_pending_frame.in_set(PuzzleBevyRendererSystems::ApplySubmittedFrames),
            );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BevyRenderError {
    RendererNotInstalled,
    InvalidFramebufferRect,
    InvalidViewId {
        field: &'static str,
    },
    InvalidViewGeometry {
        field: &'static str,
    },
    InvalidCamera {
        field: &'static str,
    },
    InvalidLighting {
        field: &'static str,
    },
    InvalidPixelate {
        field: &'static str,
    },
    ViewLayerExhausted,
    ViewGenerationExhausted,
    InvalidCameraOrder,
    DuplicateCameraOrder {
        order: isize,
    },
    ViewDimensionMismatch {
        view_id: PuzzleBevyViewId,
        expected: PuzzleBevyViewDimension,
    },
    UnknownView {
        view_id: PuzzleBevyViewId,
    },
    UnsupportedPrimitive {
        batch_index: usize,
        kind: &'static str,
    },
    UnsupportedDecoration {
        decoration_index: usize,
        kind: &'static str,
    },
    InvalidDecoration {
        decoration_index: usize,
        field: &'static str,
    },
    InvalidVoxelDimensions {
        batch_index: usize,
    },
    InvalidVoxelPosition {
        batch_index: usize,
        voxel_index: usize,
    },
    MissingPixelGeometry {
        batch_index: usize,
    },
    InvalidPixelGeometry {
        batch_index: usize,
    },
    InvalidRasterGeometry {
        batch_index: usize,
    },
    MissingRasterAsset {
        batch_index: usize,
        asset: VisualImageAssetId,
    },
    RasterAssetRevisionMismatch {
        batch_index: usize,
        asset: VisualImageAssetId,
        expected: VisualImageAssetRevision,
        actual: VisualImageAssetRevision,
    },
    RasterAssetDimensionsMismatch {
        batch_index: usize,
        asset: VisualImageAssetId,
        expected: [u16; 2],
        actual: [u16; 2],
    },
    InvalidPixelDimensions {
        batch_index: usize,
    },
    InvalidPixelPosition {
        batch_index: usize,
        pixel_index: usize,
    },
    DuplicatePixelPosition {
        batch_index: usize,
        pixel_index: usize,
    },
    InvalidOpacity {
        batch_index: usize,
    },
    InvalidColor {
        batch_index: usize,
        voxel_index: usize,
    },
    InvalidAffine {
        batch_index: usize,
    },
    UnsupportedAffineShear {
        batch_index: usize,
    },
}

impl fmt::Display for BevyRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RendererNotInstalled => {
                write!(formatter, "Bevy renderer plugin is not installed")
            }
            Self::InvalidFramebufferRect => {
                write!(formatter, "render view framebuffer size must be non-zero")
            }
            Self::InvalidViewId { field } => {
                write!(formatter, "render view id {field} must be non-empty")
            }
            Self::InvalidViewGeometry { field } => {
                write!(formatter, "render view {field} is invalid")
            }
            Self::InvalidCamera { field } => {
                write!(formatter, "render view camera {field} is invalid")
            }
            Self::InvalidLighting { field } => {
                write!(formatter, "render view lighting {field} is invalid")
            }
            Self::InvalidPixelate { field } => {
                write!(formatter, "render view pixelate {field} is invalid")
            }
            Self::ViewLayerExhausted => {
                write!(formatter, "Bevy render view layer namespace is exhausted")
            }
            Self::ViewGenerationExhausted => {
                write!(formatter, "Bevy render view generation is exhausted")
            }
            Self::InvalidCameraOrder => {
                write!(
                    formatter,
                    "Bevy render view camera order is outside the supported range"
                )
            }
            Self::DuplicateCameraOrder { order } => {
                write!(
                    formatter,
                    "Bevy render view camera order {order} is already owned"
                )
            }
            Self::ViewDimensionMismatch { view_id, expected } => write!(
                formatter,
                "render view '{}::{}' has dimension {:?}, expected {:?}",
                view_id.component_instance, view_id.source, view_id.dimension, expected
            ),
            Self::UnknownView { view_id } => write!(
                formatter,
                "render view '{}::{}' is not registered",
                view_id.component_instance, view_id.source
            ),
            Self::UnsupportedPrimitive { batch_index, kind } => write!(
                formatter,
                "resolved batch {batch_index} contains unsupported {kind} content"
            ),
            Self::UnsupportedDecoration {
                decoration_index,
                kind,
            } => write!(
                formatter,
                "resolved decoration {decoration_index} contains unsupported {kind}"
            ),
            Self::InvalidDecoration {
                decoration_index,
                field,
            } => write!(
                formatter,
                "resolved decoration {decoration_index} has invalid {field}"
            ),
            Self::InvalidVoxelDimensions { batch_index } => {
                write!(
                    formatter,
                    "resolved voxel batch {batch_index} has invalid dimensions"
                )
            }
            Self::InvalidVoxelPosition {
                batch_index,
                voxel_index,
            } => write!(
                formatter,
                "resolved voxel {voxel_index} in batch {batch_index} is outside its frame"
            ),
            Self::MissingPixelGeometry { batch_index } => write!(
                formatter,
                "resolved pixel batch {batch_index} is missing pixel geometry"
            ),
            Self::InvalidRasterGeometry { batch_index } => write!(
                formatter,
                "resolved raster batch {batch_index} has invalid destination or UV geometry"
            ),
            Self::MissingRasterAsset { batch_index, asset } => write!(
                formatter,
                "resolved raster batch {batch_index} references missing decoded asset `{}`",
                asset.0
            ),
            Self::RasterAssetRevisionMismatch {
                batch_index,
                asset,
                expected,
                actual,
            } => write!(
                formatter,
                "resolved raster batch {batch_index} asset `{}` revision mismatch: expected {}, catalog has {}",
                asset.0, expected.0, actual.0
            ),
            Self::RasterAssetDimensionsMismatch {
                batch_index,
                asset,
                expected,
                actual,
            } => write!(
                formatter,
                "resolved raster batch {batch_index} asset `{}` dimensions mismatch: expected {}x{}, catalog has {}x{}",
                asset.0, expected[0], expected[1], actual[0], actual[1]
            ),
            Self::InvalidPixelGeometry { batch_index } => write!(
                formatter,
                "resolved pixel batch {batch_index} has invalid pixel geometry"
            ),
            Self::InvalidPixelDimensions { batch_index } => write!(
                formatter,
                "resolved pixel batch {batch_index} has invalid dimensions"
            ),
            Self::InvalidPixelPosition {
                batch_index,
                pixel_index,
            } => write!(
                formatter,
                "resolved pixel {pixel_index} in batch {batch_index} is outside its frame"
            ),
            Self::DuplicatePixelPosition {
                batch_index,
                pixel_index,
            } => write!(
                formatter,
                "resolved pixel {pixel_index} in batch {batch_index} duplicates a frame position"
            ),
            Self::InvalidOpacity { batch_index } => {
                write!(
                    formatter,
                    "resolved batch {batch_index} has invalid opacity"
                )
            }
            Self::InvalidColor {
                batch_index,
                voxel_index,
            } => write!(
                formatter,
                "resolved voxel {voxel_index} in batch {batch_index} has invalid linear RGBA"
            ),
            Self::InvalidAffine { batch_index } => {
                write!(
                    formatter,
                    "resolved batch {batch_index} has an invalid affine"
                )
            }
            Self::UnsupportedAffineShear { batch_index } => write!(
                formatter,
                "resolved batch {batch_index} contains shear that Bevy Transform cannot represent"
            ),
        }
    }
}

impl Error for BevyRenderError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedVoxelKey {
    pub batch_index: usize,
    pub voxel_index: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedVoxel {
    pub key: PreparedVoxelKey,
    pub transform: Transform,
    pub color: LinearRgba,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedLineMesh3dKey {
    pub decoration_index: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedLineMesh3d {
    pub key: PreparedLineMesh3dKey,
    pub color: LinearRgba,
    mesh: PreparedLineGeometry3d,
}

#[derive(Clone, Debug)]
struct PreparedLineGeometry3d {
    key: LineGeometryKey3d,
    positions: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LineGeometryKey3d {
    positions: Vec<[u32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl PreparedBounds {
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extent(self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Clone, Debug)]
pub struct PreparedBevyFrame {
    pub voxels: Vec<PreparedVoxel>,
    pub line_meshes: Vec<PreparedLineMesh3d>,
    pub bounds: PreparedBounds,
    pub continue_animation: bool,
}

pub fn prepare_resolved_frame(
    frame: &RuntimeResolvedRenderFrame,
) -> Result<PreparedBevyFrame, BevyRenderError> {
    let mut visible: Vec<Option<PreparedVoxel>> = Vec::new();
    let mut stacks: HashMap<WorldVoxelKey, Vec<usize>> = HashMap::new();

    for (batch_index, batch) in frame.batches.iter().enumerate() {
        let opacity = finite_unit(batch.opacity)
            .ok_or(BevyRenderError::InvalidOpacity { batch_index })? as f32;
        let RuntimeResolvedRenderBatchContent::Voxels {
            width,
            depth,
            height,
            voxels,
        } = &batch.content
        else {
            let kind = match &batch.content {
                RuntimeResolvedRenderBatchContent::Pixels { .. } => "pixel",
                RuntimeResolvedRenderBatchContent::RasterImage { .. } => "raster image",
                RuntimeResolvedRenderBatchContent::Voxels { .. } => unreachable!(),
            };
            return Err(BevyRenderError::UnsupportedPrimitive { batch_index, kind });
        };
        if *width == 0 || *depth == 0 || *height == 0 {
            return Err(BevyRenderError::InvalidVoxelDimensions { batch_index });
        }
        let affine = runtime_affine(batch.transform, batch_index)?;
        let cell = Vec3::new(
            batch.cell[0] as f32,
            batch.cell[1] as f32,
            batch.cell[2] as f32,
        );
        let step = 1.0 / f32::from((*width).max(*depth).max(*height));

        for (voxel_index, voxel) in voxels.iter().enumerate() {
            if voxel.position[0] < 0
                || voxel.position[1] < 0
                || voxel.position[2] < 0
                || voxel.position[0] >= i32::from(*width)
                || voxel.position[1] >= i32::from(*depth)
                || voxel.position[2] >= i32::from(*height)
            {
                return Err(BevyRenderError::InvalidVoxelPosition {
                    batch_index,
                    voxel_index,
                });
            }
            let color = resolved_color(voxel.color, opacity, batch_index, voxel_index)?;
            if color.alpha <= 0.0 {
                continue;
            }
            let local = Vec3::new(
                (voxel.position[0] as f32 + 0.5 - f32::from(*width) / 2.0) * step,
                (voxel.position[1] as f32 + 0.5 - f32::from(*depth) / 2.0) * step,
                (voxel.position[2] as f32 + 0.5 - f32::from(*height) / 2.0) * step,
            );
            let visual_from_cube = Mat4::from_translation(cell)
                * affine
                * Mat4::from_translation(local)
                * Mat4::from_scale(Vec3::splat(step));
            let bevy_from_cube =
                visual_to_bevy_basis() * visual_from_cube * visual_to_bevy_basis().inverse();
            let transform = checked_transform(bevy_from_cube, batch_index)?;
            let prepared = PreparedVoxel {
                key: PreparedVoxelKey {
                    batch_index,
                    voxel_index,
                },
                transform,
                color,
                render_order: batch.render_order,
                object_ids: batch.object_ids.clone(),
            };
            let world_key = WorldVoxelKey::from_matrix(bevy_from_cube);
            let stack = stacks.entry(world_key).or_default();
            if color.alpha >= OPAQUE_ALPHA {
                for prior in stack.drain(..) {
                    visible[prior] = None;
                }
            }
            stack.push(visible.len());
            visible.push(Some(prepared));
        }
    }

    let voxels = visible.into_iter().flatten().collect::<Vec<_>>();
    let line_meshes = prepare_line_meshes_3d(frame)?;
    let bounds = frame_bounds(&voxels, &line_meshes);
    Ok(PreparedBevyFrame {
        voxels,
        line_meshes,
        bounds,
        continue_animation: frame.continue_animation,
    })
}

fn prepare_line_meshes_3d(
    frame: &RuntimeResolvedRenderFrame,
) -> Result<Vec<PreparedLineMesh3d>, BevyRenderError> {
    let mut prepared = Vec::with_capacity(frame.decorations.len());
    for (decoration_index, decoration) in frame.decorations.iter().enumerate() {
        let RuntimeResolvedDecoration::Lines3d {
            segments,
            style,
            depth: _,
        } = decoration
        else {
            return Err(BevyRenderError::UnsupportedDecoration {
                decoration_index,
                kind: "2D lines",
            });
        };
        match style.width {
            RuntimeResolvedStrokeWidth::PhysicalPixels { pixels }
                if pixels.is_finite() && pixels == 1.0 => {}
            RuntimeResolvedStrokeWidth::PhysicalPixels { .. }
            | RuntimeResolvedStrokeWidth::CellRelative { .. } => {
                return Err(BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "width (Bevy LineList supports exactly 1 physical pixel)",
                });
            }
        }
        let color = resolved_decoration_color(style.color, decoration_index)?;
        let mut positions = Vec::with_capacity(segments.len() * 2);
        for segment in segments {
            let start = Vec3::new(
                segment.start[0] as f32,
                segment.start[1] as f32,
                segment.start[2] as f32,
            );
            let end = Vec3::new(
                segment.end[0] as f32,
                segment.end[1] as f32,
                segment.end[2] as f32,
            );
            if !start.is_finite() || !end.is_finite() || start == end {
                return Err(BevyRenderError::InvalidDecoration {
                    decoration_index,
                    field: "segment",
                });
            }
            positions.push(visual_to_bevy_basis().transform_point3(start).to_array());
            positions.push(visual_to_bevy_basis().transform_point3(end).to_array());
        }
        let key = LineGeometryKey3d {
            positions: positions
                .iter()
                .map(|position| position.map(f32::to_bits))
                .collect(),
        };
        prepared.push(PreparedLineMesh3d {
            key: PreparedLineMesh3dKey { decoration_index },
            color,
            mesh: PreparedLineGeometry3d { key, positions },
        });
    }
    Ok(prepared)
}

fn resolved_decoration_color(
    color: RuntimeLinearRgba,
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

pub fn submit_resolved_frame(
    world: &mut World,
    view_id: PuzzleBevyViewId,
    view: PuzzleBevy3dView,
    frame: &RuntimeResolvedRenderFrame,
) -> Result<u64, BevyRenderError> {
    let Some(mut queue) = world.get_resource_mut::<BevyResolvedFrameQueue>() else {
        return Err(BevyRenderError::RendererNotInstalled);
    };
    queue.submit(view_id, view, frame)
}

pub fn remove_render_view(
    world: &mut World,
    view_id: &PuzzleBevyViewId,
) -> Result<u64, BevyRenderError> {
    view_id.validate_dimension(PuzzleBevyViewDimension::ThreeD)?;
    let Some(mut queue) = world.get_resource_mut::<BevyResolvedFrameQueue>() else {
        return Err(BevyRenderError::RendererNotInstalled);
    };
    queue.remove(view_id)
}

#[derive(Component, Clone, Debug)]
pub struct PuzzleVoxel {
    pub key: PreparedVoxelKey,
    pub render_order: u64,
    pub object_ids: Vec<u16>,
}

#[derive(Component, Clone, Debug)]
pub struct PuzzleLineMesh3d {
    pub key: PreparedLineMesh3dKey,
}

#[derive(Component)]
struct PuzzleRendererCamera;

#[derive(Component)]
struct PuzzleRendererLight;

#[derive(Resource, Default)]
pub struct BevyResolvedFrameQueue {
    next_generation: u64,
    registry: BevyViewRegistry,
    pending: HashMap<PuzzleBevyViewId, PendingViewChange>,
}

impl BevyResolvedFrameQueue {
    pub fn reconcile_camera_orders(
        &mut self,
        desired: &BTreeMap<PuzzleBevyViewId, isize>,
    ) -> Result<(), BevyRenderError> {
        for view_id in desired.keys() {
            view_id.validate()?;
            view_id.validate_dimension(PuzzleBevyViewDimension::ThreeD)?;
        }
        self.registry.reconcile_camera_orders(desired)
    }

    pub fn submit(
        &mut self,
        view_id: PuzzleBevyViewId,
        view: PuzzleBevy3dView,
        frame: &RuntimeResolvedRenderFrame,
    ) -> Result<u64, BevyRenderError> {
        view_id.validate()?;
        view_id.validate_dimension(PuzzleBevyViewDimension::ThreeD)?;
        self.submit_prepared(view_id, view, prepare_resolved_frame(frame)?)
    }

    pub fn submit_prepared(
        &mut self,
        view_id: PuzzleBevyViewId,
        view: PuzzleBevy3dView,
        frame: PreparedBevyFrame,
    ) -> Result<u64, BevyRenderError> {
        view_id.validate()?;
        view_id.validate_dimension(PuzzleBevyViewDimension::ThreeD)?;
        view.framebuffer.validate()?;
        view.camera.validate()?;
        view.lighting
            .validate()
            .map_err(|error| BevyRenderError::InvalidLighting { field: error.field })?;
        if view.render_settings.pixelate.scale == 0 {
            return Err(BevyRenderError::InvalidPixelate { field: "scale" });
        }
        let generation = self.next_generation()?;
        let (render_layer, camera_order) =
            self.registry
                .reserve(&view_id, view.order, FIRST_3D_RENDER_LAYER)?;
        self.pending.insert(
            view_id.clone(),
            PendingViewChange::Submit(SubmittedFrame {
                generation,
                view_id,
                render_layer,
                camera_order,
                view,
                frame,
            }),
        );
        Ok(generation)
    }

    pub fn remove(&mut self, view_id: &PuzzleBevyViewId) -> Result<u64, BevyRenderError> {
        view_id.validate_dimension(PuzzleBevyViewDimension::ThreeD)?;
        self.registry.validate_removal(view_id)?;
        let generation = self.next_generation()?;
        self.registry.release_registered(view_id);
        self.pending
            .insert(view_id.clone(), PendingViewChange::Remove);
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

struct SubmittedFrame {
    generation: u64,
    view_id: PuzzleBevyViewId,
    render_layer: usize,
    camera_order: isize,
    view: PuzzleBevy3dView,
    frame: PreparedBevyFrame,
}

enum PendingViewChange {
    Submit(SubmittedFrame),
    Remove,
}

#[derive(Resource)]
struct BevyRenderAssets {
    cube: Handle<Mesh>,
    materials: HashMap<MaterialKey, Handle<StandardMaterial>>,
    line_meshes: HashMap<LineGeometryKey3d, Handle<Mesh>>,
    line_materials: HashMap<MaterialKey, Handle<StandardMaterial>>,
}

#[derive(Resource)]
struct RenderedViewState {
    generation: u64,
    entities: HashMap<PreparedVoxelKey, Entity>,
    line_entities: HashMap<PreparedLineMesh3dKey, Entity>,
    material_keys: HashSet<MaterialKey>,
    line_mesh_keys: HashSet<LineGeometryKey3d>,
    line_material_keys: HashSet<MaterialKey>,
    bounds: PreparedBounds,
    camera: Entity,
    light: Entity,
    render_layer: usize,
}

#[derive(Resource, Default)]
struct RenderedFrameState {
    views: HashMap<PuzzleBevyViewId, RenderedViewState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MaterialKey {
    color: [u32; 4],
    shaded: bool,
}

impl MaterialKey {
    fn new(color: LinearRgba, shaded: bool) -> Self {
        Self {
            color: [
                color.red.to_bits(),
                color.green.to_bits(),
                color.blue.to_bits(),
                color.alpha.to_bits(),
            ],
            shaded,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WorldVoxelKey([u32; 16]);

impl WorldVoxelKey {
    fn from_matrix(matrix: Mat4) -> Self {
        Self(matrix.to_cols_array().map(f32::to_bits))
    }
}

fn setup_renderer(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(BevyRenderAssets {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        materials: HashMap::new(),
        line_meshes: HashMap::new(),
        line_materials: HashMap::new(),
    });
}

fn lighting_transform(lighting: &PuzzleBevyLighting) -> Transform {
    let yaw = lighting.yaw_degrees.to_radians();
    let pitch = lighting.pitch_degrees.to_radians();
    let source = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    Transform::from_translation(source).looking_at(Vec3::ZERO, Vec3::Y)
}

fn apply_pending_frame(
    mut commands: Commands,
    mut queue: ResMut<BevyResolvedFrameQueue>,
    mut state: ResMut<RenderedFrameState>,
    mut render_assets: ResMut<BevyRenderAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if queue.pending.is_empty() {
        return;
    }
    let pending = std::mem::take(&mut queue.pending);
    for (view_id, change) in pending {
        match change {
            PendingViewChange::Remove => {
                if let Some(view) = state.views.remove(&view_id) {
                    for entity in view.entities.into_values() {
                        commands.entity(entity).despawn();
                    }
                    for entity in view.line_entities.into_values() {
                        commands.entity(entity).despawn();
                    }
                    commands.entity(view.camera).despawn();
                    commands.entity(view.light).despawn();
                }
                queue.registry.finish_removal(&view_id);
            }
            PendingViewChange::Submit(submitted) => {
                let render_layers = RenderLayers::none().with(submitted.render_layer);
                let mut view = state.views.remove(&view_id).unwrap_or_else(|| {
                    let camera = commands
                        .spawn((
                            Camera3d::default(),
                            PuzzleRendererCamera,
                            PuzzleBevyRenderView {
                                id: submitted.view_id.clone(),
                            },
                            render_layers.clone(),
                        ))
                        .id();
                    let light = commands
                        .spawn((
                            PuzzleRendererLight,
                            PuzzleBevyRenderView {
                                id: submitted.view_id.clone(),
                            },
                            render_layers.clone(),
                        ))
                        .id();
                    RenderedViewState {
                        generation: 0,
                        entities: HashMap::new(),
                        line_entities: HashMap::new(),
                        material_keys: HashSet::new(),
                        line_mesh_keys: HashSet::new(),
                        line_material_keys: HashSet::new(),
                        bounds: submitted.frame.bounds,
                        camera,
                        light,
                        render_layer: submitted.render_layer,
                    }
                });
                debug_assert_eq!(view.render_layer, submitted.render_layer);
                let mut retained = HashMap::with_capacity(submitted.frame.voxels.len());
                let mut used_materials = HashSet::new();
                for voxel in submitted.frame.voxels {
                    let entity = view
                        .entities
                        .remove(&voxel.key)
                        .unwrap_or_else(|| commands.spawn_empty().id());
                    let material_key =
                        MaterialKey::new(voxel.color, submitted.view.render_settings.shade);
                    let material = material_for(
                        material_key,
                        voxel.color,
                        submitted.view.render_settings.shade,
                        &mut render_assets,
                        &mut materials,
                    );
                    used_materials.insert(material_key);
                    commands.entity(entity).insert((
                        Mesh3d(render_assets.cube.clone()),
                        MeshMaterial3d(material),
                        voxel.transform,
                        render_layers.clone(),
                        PuzzleBevyRenderView {
                            id: submitted.view_id.clone(),
                        },
                        PuzzleVoxel {
                            key: voxel.key,
                            render_order: voxel.render_order,
                            object_ids: voxel.object_ids,
                        },
                    ));
                    retained.insert(voxel.key, entity);
                }
                for entity in view.entities.drain().map(|(_, entity)| entity) {
                    commands.entity(entity).despawn();
                }
                let mut retained_lines = HashMap::with_capacity(submitted.frame.line_meshes.len());
                let mut used_line_meshes = HashSet::new();
                let mut used_line_materials = HashSet::new();
                for prepared in submitted.frame.line_meshes {
                    let entity = view
                        .line_entities
                        .remove(&prepared.key)
                        .unwrap_or_else(|| commands.spawn_empty().id());
                    let mesh = line_mesh_for(&prepared.mesh, &mut render_assets, &mut meshes);
                    let material_key = MaterialKey::new(prepared.color, false);
                    let material = line_material_for(
                        material_key,
                        prepared.color,
                        &mut render_assets,
                        &mut materials,
                    );
                    used_line_meshes.insert(prepared.mesh.key.clone());
                    used_line_materials.insert(material_key);
                    commands.entity(entity).insert((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                        render_layers.clone(),
                        PuzzleBevyRenderView {
                            id: submitted.view_id.clone(),
                        },
                        PuzzleLineMesh3d { key: prepared.key },
                    ));
                    retained_lines.insert(prepared.key, entity);
                }
                for entity in view.line_entities.drain().map(|(_, entity)| entity) {
                    commands.entity(entity).despawn();
                }
                view.entities = retained;
                view.line_entities = retained_lines;
                view.material_keys = used_materials;
                view.line_mesh_keys = used_line_meshes;
                view.line_material_keys = used_line_materials;
                view.bounds = submitted.frame.bounds;
                view.generation = submitted.generation;
                update_3d_view_entities(
                    &mut commands,
                    view.camera,
                    view.light,
                    &submitted.view,
                    submitted.camera_order,
                    view.bounds,
                    render_layers,
                );
                state.views.insert(view_id, view);
            }
        }
    }
    let used_materials = state
        .views
        .values()
        .flat_map(|view| view.material_keys.iter().copied())
        .collect::<HashSet<_>>();
    render_assets.materials.retain(|key, handle| {
        if used_materials.contains(key) {
            true
        } else {
            materials.remove(handle.id());
            false
        }
    });
    let used_line_meshes = state
        .views
        .values()
        .flat_map(|view| view.line_mesh_keys.iter().cloned())
        .collect::<HashSet<_>>();
    render_assets.line_meshes.retain(|key, handle| {
        if used_line_meshes.contains(key) {
            true
        } else {
            meshes.remove(handle.id());
            false
        }
    });
    let used_line_materials = state
        .views
        .values()
        .flat_map(|view| view.line_material_keys.iter().copied())
        .collect::<HashSet<_>>();
    render_assets.line_materials.retain(|key, handle| {
        if used_line_materials.contains(key) {
            true
        } else {
            materials.remove(handle.id());
            false
        }
    });
}

fn line_mesh_for(
    prepared: &PreparedLineGeometry3d,
    render_assets: &mut BevyRenderAssets,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    render_assets
        .line_meshes
        .entry(prepared.key.clone())
        .or_insert_with(|| {
            let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, prepared.positions.clone());
            meshes.add(mesh)
        })
        .clone()
}

fn line_material_for(
    key: MaterialKey,
    color: LinearRgba,
    render_assets: &mut BevyRenderAssets,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    render_assets
        .line_materials
        .entry(key)
        .or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::LinearRgba(color),
                alpha_mode: if color.alpha >= OPAQUE_ALPHA {
                    AlphaMode::Opaque
                } else {
                    AlphaMode::Blend
                },
                unlit: true,
                cull_mode: None,
                ..default()
            })
        })
        .clone()
}

fn update_3d_view_entities(
    commands: &mut Commands,
    camera_entity: Entity,
    light_entity: Entity,
    view: &PuzzleBevy3dView,
    camera_order: isize,
    bounds: PreparedBounds,
    render_layers: RenderLayers,
) {
    let target = bounds.center();
    let radius = (bounds.extent().length() * 0.5).max(0.75);
    let yaw = view.camera.yaw_degrees.to_radians();
    let pitch = view.camera.pitch_degrees.to_radians();
    let direction = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    )
    .normalize_or_zero();
    let mut transform =
        Transform::from_translation(target + direction * radius * view.camera.distance_scale);
    transform.look_at(target, Vec3::Y);
    transform.rotate_local_z(view.camera.roll_degrees.to_radians());
    let projection = match view.camera.projection {
        PuzzleCameraProjection::Perspective => {
            Projection::Perspective(PerspectiveProjection::default())
        }
        PuzzleCameraProjection::Orthographic => Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: radius * 2.5,
            },
            ..OrthographicProjection::default_3d()
        }),
    };
    let mut camera_commands = commands.entity(camera_entity);
    camera_commands.insert((
        Camera {
            is_active: view.active,
            order: camera_order,
            viewport: Some(view.framebuffer.viewport()),
            clear_color: ClearColorConfig::Custom(view.clear_color.clone()),
            ..default()
        },
        projection,
        transform,
        AmbientLight {
            color: view.lighting.color.clone(),
            brightness: 400.0 * view.lighting.ambient,
            ..default()
        },
        render_layers.clone(),
    ));
    if view.render_settings.pixelate.enabled && view.render_settings.pixelate.scale > 1 {
        camera_commands.insert(pixelate::PuzzlePixelatePostProcess::new(
            view.framebuffer.physical_position,
            view.framebuffer.physical_size,
            view.render_settings.pixelate.scale,
            view.render_settings.pixelate.smoothing,
        ));
    } else {
        camera_commands.remove::<pixelate::PuzzlePixelatePostProcess>();
    }
    commands.entity(light_entity).insert((
        DirectionalLight {
            shadow_maps_enabled: view.shadows_enabled,
            color: view.lighting.color.clone(),
            illuminance: 2_000.0 * view.lighting.intensity,
            ..default()
        },
        lighting_transform(&view.lighting),
        render_layers,
    ));
}

fn material_for(
    key: MaterialKey,
    color: LinearRgba,
    shaded: bool,
    render_assets: &mut BevyRenderAssets,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    render_assets
        .materials
        .entry(key)
        .or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::LinearRgba(color),
                alpha_mode: if color.alpha >= OPAQUE_ALPHA {
                    AlphaMode::Opaque
                } else {
                    AlphaMode::Blend
                },
                perceptual_roughness: 0.82,
                unlit: !shaded,
                ..default()
            })
        })
        .clone()
}

fn runtime_affine(value: [[f64; 4]; 4], batch_index: usize) -> Result<Mat4, BevyRenderError> {
    if value.iter().flatten().any(|entry| !entry.is_finite())
        || value[3][0].abs() > f64::from(MATRIX_EPSILON)
        || value[3][1].abs() > f64::from(MATRIX_EPSILON)
        || value[3][2].abs() > f64::from(MATRIX_EPSILON)
        || (value[3][3] - 1.0).abs() > f64::from(MATRIX_EPSILON)
    {
        return Err(BevyRenderError::InvalidAffine { batch_index });
    }
    Ok(Mat4::from_cols_array_2d(&[
        [
            value[0][0] as f32,
            value[1][0] as f32,
            value[2][0] as f32,
            value[3][0] as f32,
        ],
        [
            value[0][1] as f32,
            value[1][1] as f32,
            value[2][1] as f32,
            value[3][1] as f32,
        ],
        [
            value[0][2] as f32,
            value[1][2] as f32,
            value[2][2] as f32,
            value[3][2] as f32,
        ],
        [
            value[0][3] as f32,
            value[1][3] as f32,
            value[2][3] as f32,
            value[3][3] as f32,
        ],
    ]))
}

fn visual_to_bevy_basis() -> Mat4 {
    Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::W,
    )
}

fn checked_transform(matrix: Mat4, batch_index: usize) -> Result<Transform, BevyRenderError> {
    if !matrix.is_finite() {
        return Err(BevyRenderError::InvalidAffine { batch_index });
    }
    let transform = Transform::from_matrix(matrix);
    let reconstructed = transform.to_matrix().to_cols_array();
    let original = matrix.to_cols_array();
    if reconstructed
        .iter()
        .zip(original)
        .any(|(left, right)| (*left - right).abs() > MATRIX_EPSILON)
    {
        return Err(BevyRenderError::UnsupportedAffineShear { batch_index });
    }
    Ok(transform)
}

fn resolved_color(
    color: RuntimeLinearRgba,
    opacity: f32,
    batch_index: usize,
    voxel_index: usize,
) -> Result<LinearRgba, BevyRenderError> {
    let channels = [color.red, color.green, color.blue, color.alpha];
    if channels
        .iter()
        .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(channel))
    {
        return Err(BevyRenderError::InvalidColor {
            batch_index,
            voxel_index,
        });
    }
    Ok(LinearRgba::new(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32 * opacity,
    ))
}

fn finite_unit(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}

fn frame_bounds(voxels: &[PreparedVoxel], lines: &[PreparedLineMesh3d]) -> PreparedBounds {
    if voxels.is_empty() && lines.iter().all(|line| line.mesh.positions.is_empty()) {
        return PreparedBounds {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        };
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for voxel in voxels {
        let matrix = voxel.transform.to_matrix();
        for x in [-0.5, 0.5] {
            for y in [-0.5, 0.5] {
                for z in [-0.5, 0.5] {
                    let point = matrix.transform_point3(Vec3::new(x, y, z));
                    min = min.min(point);
                    max = max.max(point);
                }
            }
        }
    }
    for point in lines
        .iter()
        .flat_map(|line| line.mesh.positions.iter().copied())
        .map(Vec3::from_array)
    {
        min = min.min(point);
        max = max.max(point);
    }
    let minimum_extent = Vec3::splat(0.01);
    let center = (min + max) * 0.5;
    let half_extent = ((max - min) * 0.5).max(minimum_extent);
    min = center - half_extent;
    max = center + half_extent;
    PreparedBounds { min, max }
}

#[cfg(test)]
mod tests {
    use puzzle_runtime_contract::{
        RuntimeResolvedCompositionGroup, RuntimeResolvedFitMode, RuntimeResolvedLineDepth3d,
        RuntimeResolvedLineSegment3d, RuntimeResolvedLineStyle, RuntimeResolvedPlayback,
        RuntimeResolvedRenderBatch, RuntimeResolvedRenderBatchContent,
        RuntimeResolvedRenderInstance, RuntimeResolvedRenderScene, RuntimeResolvedVisualClip,
        RuntimeResolvedVisualFrame, RuntimeResolvedVisualLayout, RuntimeResolvedVoxel,
        RuntimeVisualComposition,
    };

    use super::*;

    fn color(red: f64, green: f64, blue: f64, alpha: f64) -> RuntimeLinearRgba {
        RuntimeLinearRgba {
            red,
            green,
            blue,
            alpha,
        }
    }

    fn identity() -> [[f64; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn voxel_batch(
        render_order: u64,
        cell: [i32; 3],
        rgba: RuntimeLinearRgba,
    ) -> RuntimeResolvedRenderBatch {
        RuntimeResolvedRenderBatch {
            render_order,
            object_ids: vec![render_order as u16],
            cell,
            transform: identity(),
            opacity: 1.0,
            pixel_geometry: None,
            content: RuntimeResolvedRenderBatchContent::Voxels {
                width: 1,
                depth: 1,
                height: 1,
                voxels: vec![RuntimeResolvedVoxel {
                    position: [0, 0, 0],
                    color: rgba,
                }],
            },
        }
    }

    fn view_3d(position: UVec2) -> PuzzleBevy3dView {
        PuzzleBevy3dView {
            active: true,
            order: 0,
            framebuffer: PuzzleBevyFramebufferRect {
                physical_position: position,
                physical_size: UVec2::new(320, 240),
            },
            clear_color: Color::linear_rgb(0.025, 0.03, 0.045),
            camera: PuzzleBevyCamera::default(),
            lighting: PuzzleBevyLighting {
                intensity: 1.0,
                ambient: 1.0,
                yaw_degrees: 53.0,
                pitch_degrees: 56.0,
                color: Color::WHITE,
            },
            shadows_enabled: true,
            render_settings: PuzzleBevy3dRenderSettings::default(),
        }
    }

    #[test]
    fn projects_canonical_xyz_and_object_ids_without_visual_authoring_data() {
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(7, [2, 3, 4], color(1.0, 0.0, 0.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };

        let prepared = prepare_resolved_frame(&frame).unwrap();

        assert_eq!(prepared.voxels.len(), 1);
        assert_eq!(prepared.voxels[0].object_ids, vec![7]);
        assert_eq!(prepared.voxels[0].render_order, 7);
        assert!(
            prepared.voxels[0]
                .transform
                .translation
                .abs_diff_eq(Vec3::new(2.0, 4.0, -3.0), MATRIX_EPSILON)
        );
    }

    #[test]
    fn a_later_opaque_resolved_layer_replaces_the_same_world_voxel() {
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![
                voxel_batch(1, [0, 0, 0], color(1.0, 0.0, 0.0, 0.5)),
                voxel_batch(2, [0, 0, 0], color(0.0, 0.0, 1.0, 1.0)),
            ],
            decorations: Vec::new(),
            continue_animation: false,
        };

        let prepared = prepare_resolved_frame(&frame).unwrap();

        assert_eq!(prepared.voxels.len(), 1);
        assert_eq!(prepared.voxels[0].render_order, 2);
        assert_eq!(prepared.voxels[0].color, LinearRgba::BLUE);
    }

    #[test]
    fn rejects_pixel_content_at_the_3d_backend_boundary() {
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![RuntimeResolvedRenderBatch {
                render_order: 0,
                object_ids: Vec::new(),
                cell: [0, 0, 0],
                transform: identity(),
                opacity: 1.0,
                pixel_geometry: None,
                content: RuntimeResolvedRenderBatchContent::Pixels {
                    width: 1,
                    height: 1,
                    pixels: Vec::new(),
                },
            }],
            decorations: Vec::new(),
            continue_animation: false,
        };

        assert_eq!(
            prepare_resolved_frame(&frame).unwrap_err(),
            BevyRenderError::UnsupportedPrimitive {
                batch_index: 0,
                kind: "pixel",
            }
        );
    }

    #[test]
    fn rejects_shear_instead_of_silently_approximating_it() {
        let mut batch = voxel_batch(1, [0, 0, 0], color(1.0, 1.0, 1.0, 1.0));
        batch.transform[0][1] = 0.5;
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![batch],
            decorations: Vec::new(),
            continue_animation: false,
        };

        assert_eq!(
            prepare_resolved_frame(&frame).unwrap_err(),
            BevyRenderError::UnsupportedAffineShear { batch_index: 0 }
        );
    }

    fn line_decoration_3d() -> RuntimeResolvedDecoration {
        RuntimeResolvedDecoration::Lines3d {
            segments: vec![RuntimeResolvedLineSegment3d {
                start: [1.0, 2.0, 3.0],
                end: [4.0, 5.0, 6.0],
            }],
            style: RuntimeResolvedLineStyle {
                color: color(0.25, 0.5, 0.75, 0.5),
                width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 1.0 },
            },
            depth: RuntimeResolvedLineDepth3d::Tested,
        }
    }

    #[test]
    fn projects_resolved_3d_lines_into_bevy_basis_and_frame_bounds() {
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: vec![line_decoration_3d()],
            continue_animation: false,
        };

        let prepared = prepare_resolved_frame(&frame).unwrap();

        assert_eq!(prepared.line_meshes.len(), 1);
        assert_eq!(
            prepared.line_meshes[0].mesh.positions,
            vec![[1.0, 3.0, -2.0], [4.0, 6.0, -5.0]]
        );
        assert_eq!(
            prepared.line_meshes[0].color,
            LinearRgba::new(0.25, 0.5, 0.75, 0.5)
        );
        assert_eq!(prepared.bounds.min, Vec3::new(1.0, 3.0, -5.0));
        assert_eq!(prepared.bounds.max, Vec3::new(4.0, 6.0, -2.0));
    }

    #[test]
    fn line_list_entities_are_unlit_depth_tested_view_local_geometry() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin);
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: vec![line_decoration_3d()],
            continue_animation: false,
        };
        let left = PuzzleBevyViewId::three_d("left", "main");
        let right = PuzzleBevyViewId::three_d("right", "main");
        submit_resolved_frame(app.world_mut(), left.clone(), view_3d(UVec2::ZERO), &frame).unwrap();
        submit_resolved_frame(
            app.world_mut(),
            right.clone(),
            PuzzleBevy3dView {
                order: 1,
                ..view_3d(UVec2::new(320, 0))
            },
            &frame,
        )
        .unwrap();
        app.update();

        let lines = app
            .world_mut()
            .query::<(
                &PuzzleBevyRenderView,
                &PuzzleLineMesh3d,
                &Mesh3d,
                &MeshMaterial3d<StandardMaterial>,
                &RenderLayers,
            )>()
            .iter(app.world())
            .map(|(view, _, mesh, material, layers)| {
                (
                    view.id.clone(),
                    mesh.0.clone(),
                    material.0.clone(),
                    layers.iter().collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_ne!(lines[0].3, lines[1].3);
        for (_, mesh, material, _) in &lines {
            assert_eq!(
                app.world()
                    .resource::<Assets<Mesh>>()
                    .get(mesh.id())
                    .unwrap()
                    .primitive_topology(),
                PrimitiveTopology::LineList
            );
            let materials = app.world().resource::<Assets<StandardMaterial>>();
            let material = materials.get(material.id()).unwrap();
            assert!(material.unlit);
            assert_eq!(material.alpha_mode, AlphaMode::Blend);
        }

        remove_render_view(app.world_mut(), &left).unwrap();
        app.update();
        let remaining = app
            .world_mut()
            .query::<(&PuzzleBevyRenderView, &PuzzleLineMesh3d)>()
            .iter(app.world())
            .map(|(view, _)| view.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![right]);
    }

    #[test]
    fn rejects_3d_stroke_widths_that_line_list_cannot_represent() {
        let mut decoration = line_decoration_3d();
        let RuntimeResolvedDecoration::Lines3d { style, .. } = &mut decoration else {
            unreachable!()
        };
        style.width = RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 2.0 };
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: vec![decoration],
            continue_animation: false,
        };
        assert_eq!(
            prepare_resolved_frame(&frame).unwrap_err(),
            BevyRenderError::InvalidDecoration {
                decoration_index: 0,
                field: "width (Bevy LineList supports exactly 1 physical pixel)",
            }
        );
    }

    #[test]
    fn lighting_resource_uses_normalized_strength_direction_and_color() {
        let lighting = PuzzleBevyLighting {
            intensity: 0.75,
            ambient: 1.25,
            yaw_degrees: -20.0,
            pitch_degrees: 60.0,
            color: Color::linear_rgb(1.0, 0.5, 0.25),
        };
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin);
        let mut view = view_3d(UVec2::ZERO);
        view.lighting = lighting.clone();
        submit_resolved_frame(
            app.world_mut(),
            PuzzleBevyViewId::three_d("board", "main"),
            view,
            &RuntimeResolvedRenderFrame {
                batches: vec![voxel_batch(1, [0, 0, 0], color(1.0, 1.0, 1.0, 1.0))],
                decorations: Vec::new(),
                continue_animation: false,
            },
        )
        .unwrap();
        app.update();

        let global = app.world().resource::<GlobalAmbientLight>();
        assert_eq!(global.brightness, GlobalAmbientLight::NONE.brightness);
        assert_eq!(global.color, GlobalAmbientLight::NONE.color);
        let ambient = app
            .world_mut()
            .query_filtered::<&AmbientLight, With<PuzzleRendererCamera>>()
            .single(app.world())
            .unwrap();
        assert_eq!(ambient.brightness, 500.0);
        assert_eq!(ambient.color, lighting.color);
        let (directional, transform) = app
            .world_mut()
            .query_filtered::<(&DirectionalLight, &Transform), With<PuzzleRendererLight>>()
            .single(app.world())
            .unwrap();
        assert_eq!(directional.illuminance, 1_500.0);
        assert_eq!(directional.color, lighting.color);
        assert!(transform.translation.abs_diff_eq(
            Vec3::new(
                (-20.0_f32).to_radians().sin() * 60.0_f32.to_radians().cos(),
                60.0_f32.to_radians().sin(),
                (-20.0_f32).to_radians().cos() * 60.0_f32.to_radians().cos(),
            ),
            MATRIX_EPSILON,
        ));
    }

    #[test]
    fn lighting_resource_rejects_invalid_strength_instead_of_clamping_it() {
        let lighting = PuzzleBevyLighting {
            intensity: -0.1,
            ambient: 1.0,
            yaw_degrees: 0.0,
            pitch_degrees: 45.0,
            color: Color::WHITE,
        };

        assert_eq!(
            lighting.validate(),
            Err(PuzzleBevyLightingError { field: "intensity" })
        );
    }

    #[test]
    fn keyed_views_apply_shading_without_cross_view_material_reuse() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin);
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(0.25, 0.5, 0.75, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        let shaded_id = PuzzleBevyViewId::three_d("shaded", "main");
        let unshaded_id = PuzzleBevyViewId::three_d("unshaded", "main");
        let mut shaded = view_3d(UVec2::ZERO);
        shaded.render_settings.shade = true;
        let mut unshaded = view_3d(UVec2::new(320, 0));
        unshaded.order = 1;
        unshaded.render_settings.shade = false;
        submit_resolved_frame(app.world_mut(), shaded_id.clone(), shaded, &frame).unwrap();
        submit_resolved_frame(app.world_mut(), unshaded_id.clone(), unshaded, &frame).unwrap();
        app.update();

        let handles = app
            .world_mut()
            .query::<(
                &PuzzleBevyRenderView,
                &PuzzleVoxel,
                &MeshMaterial3d<StandardMaterial>,
            )>()
            .iter(app.world())
            .map(|(view, _, material)| (view.id.clone(), material.0.clone()))
            .collect::<HashMap<_, _>>();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert!(!materials.get(handles[&shaded_id].id()).unwrap().unlit);
        assert!(materials.get(handles[&unshaded_id].id()).unwrap().unlit);
        assert_ne!(handles[&shaded_id], handles[&unshaded_id]);
    }

    #[test]
    fn pixelate_settings_are_view_owned_and_removed_when_disabled() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin);
        let view_id = PuzzleBevyViewId::three_d("board", "main");
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: Vec::new(),
            continue_animation: false,
        };
        let mut view = view_3d(UVec2::new(13, 29));
        view.framebuffer.physical_size = UVec2::new(321, 243);
        view.render_settings.pixelate = PuzzleBevyPixelate {
            enabled: true,
            scale: 6,
            smoothing: false,
        };
        submit_resolved_frame(app.world_mut(), view_id.clone(), view, &frame).unwrap();
        app.update();

        let settings = app
            .world_mut()
            .query_filtered::<&pixelate::PuzzlePixelatePostProcess, With<PuzzleRendererCamera>>()
            .single(app.world())
            .unwrap();
        assert_eq!(settings.viewport, Vec4::new(13.0, 29.0, 321.0, 243.0));
        assert_eq!(settings.parameters, Vec4::new(6.0, 0.0, 0.0, 0.0));

        let mut disabled = view_3d(UVec2::new(13, 29));
        disabled.framebuffer.physical_size = UVec2::new(321, 243);
        submit_resolved_frame(app.world_mut(), view_id, disabled, &frame).unwrap();
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<&pixelate::PuzzlePixelatePostProcess, With<PuzzleRendererCamera>>(
                )
                .iter(app.world())
                .next()
                .is_none()
        );
    }

    #[test]
    fn rejects_zero_pixelate_scale_before_queueing_a_view() {
        let mut queue = BevyResolvedFrameQueue::default();
        let mut view = view_3d(UVec2::ZERO);
        view.render_settings.pixelate.scale = 0;
        let error = queue
            .submit(
                PuzzleBevyViewId::three_d("board", "main"),
                view,
                &RuntimeResolvedRenderFrame {
                    batches: Vec::new(),
                    decorations: Vec::new(),
                    continue_animation: false,
                },
            )
            .unwrap_err();
        assert_eq!(error, BevyRenderError::InvalidPixelate { field: "scale" });
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn rejects_invalid_camera_settings_before_queueing_a_view() {
        let mut queue = BevyResolvedFrameQueue::default();
        let mut view = view_3d(UVec2::ZERO);
        view.camera.pitch_degrees = 90.0;
        let error = queue
            .submit(
                PuzzleBevyViewId::three_d("board", "main"),
                view,
                &RuntimeResolvedRenderFrame {
                    batches: Vec::new(),
                    decorations: Vec::new(),
                    continue_animation: false,
                },
            )
            .unwrap_err();

        assert_eq!(
            error,
            BevyRenderError::InvalidCamera {
                field: "pitch_degrees"
            }
        );
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn ecs_sync_reuses_stable_voxel_entities_and_removes_stale_ones() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin::default());
        let first = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(1.0, 0.0, 0.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        let view_id = PuzzleBevyViewId::three_d("board", "main");
        submit_resolved_frame(
            app.world_mut(),
            view_id.clone(),
            view_3d(UVec2::ZERO),
            &first,
        )
        .unwrap();
        app.update();
        let first_entity = app
            .world_mut()
            .query_filtered::<Entity, With<PuzzleVoxel>>()
            .single(app.world())
            .unwrap();

        let second = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(0.0, 1.0, 0.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        submit_resolved_frame(
            app.world_mut(),
            view_id.clone(),
            view_3d(UVec2::ZERO),
            &second,
        )
        .unwrap();
        app.update();
        let second_entity = app
            .world_mut()
            .query_filtered::<Entity, With<PuzzleVoxel>>()
            .single(app.world())
            .unwrap();
        assert_eq!(first_entity, second_entity);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);

        submit_resolved_frame(
            app.world_mut(),
            view_id,
            view_3d(UVec2::ZERO),
            &RuntimeResolvedRenderFrame {
                batches: Vec::new(),
                decorations: Vec::new(),
                continue_animation: false,
            },
        )
        .unwrap();
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<PuzzleVoxel>>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }

    #[test]
    fn keyed_views_own_disjoint_cameras_entities_and_removal() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin::default());
        let left = PuzzleBevyViewId::three_d("left-board", "main");
        let right = PuzzleBevyViewId::three_d("right-board", "main");
        let left_frame = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(1.0, 0.0, 0.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        let right_frame = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(0.0, 0.0, 1.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        submit_resolved_frame(
            app.world_mut(),
            left.clone(),
            view_3d(UVec2::ZERO),
            &left_frame,
        )
        .unwrap();
        submit_resolved_frame(
            app.world_mut(),
            right.clone(),
            PuzzleBevy3dView {
                order: 1,
                ..view_3d(UVec2::new(320, 0))
            },
            &right_frame,
        )
        .unwrap();
        app.update();

        let right_entity = app
            .world_mut()
            .query::<(Entity, &PuzzleBevyRenderView, &PuzzleVoxel)>()
            .iter(app.world())
            .find_map(|(entity, view, _)| (view.id == right).then_some(entity))
            .unwrap();
        let cameras = app
            .world_mut()
            .query_filtered::<(&Camera, &PuzzleBevyRenderView, &RenderLayers), With<PuzzleRendererCamera>>()
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
            .query::<(&PuzzleBevyRenderView, &PuzzleVoxel, &RenderLayers)>()
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

        remove_render_view(app.world_mut(), &left).unwrap();
        app.update();
        {
            let queue = app.world().resource::<BevyResolvedFrameQueue>();
            assert!(!queue.registry.view_layers.contains_key(&left));
            assert_eq!(queue.registry.free_layers.len(), 1);
        }

        let remaining = app
            .world_mut()
            .query::<(Entity, &PuzzleBevyRenderView, &PuzzleVoxel)>()
            .iter(app.world())
            .map(|(entity, view, _)| (entity, view.id.clone()))
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![(right_entity, right.clone())]);
        let remaining_cameras = app
            .world_mut()
            .query_filtered::<&PuzzleBevyRenderView, With<PuzzleRendererCamera>>()
            .iter(app.world())
            .map(|view| view.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(remaining_cameras, vec![right]);
        let remaining_lights = app
            .world_mut()
            .query_filtered::<&PuzzleBevyRenderView, With<PuzzleRendererLight>>()
            .iter(app.world())
            .map(|view| view.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            remaining_lights,
            vec![PuzzleBevyViewId::three_d("right-board", "main")]
        );
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
        assert_eq!(
            remove_render_view(app.world_mut(), &left),
            Err(BevyRenderError::UnknownView { view_id: left })
        );
    }

    #[test]
    fn keyed_views_preserve_distinct_directional_and_ambient_lighting() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PuzzleBevy3dPlugin);
        let left_id = PuzzleBevyViewId::three_d("left-board", "main");
        let right_id = PuzzleBevyViewId::three_d("right-board", "main");
        let mut left_view = view_3d(UVec2::ZERO);
        left_view.lighting = PuzzleBevyLighting {
            intensity: 0.5,
            ambient: 0.25,
            yaw_degrees: -30.0,
            pitch_degrees: 40.0,
            color: Color::linear_rgb(1.0, 0.0, 0.0),
        };
        let mut right_view = view_3d(UVec2::new(320, 0));
        right_view.order = 1;
        right_view.lighting = PuzzleBevyLighting {
            intensity: 1.5,
            ambient: 0.75,
            yaw_degrees: 30.0,
            pitch_degrees: 50.0,
            color: Color::linear_rgb(0.0, 0.0, 1.0),
        };
        let frame = RuntimeResolvedRenderFrame {
            batches: vec![voxel_batch(1, [0, 0, 0], color(1.0, 1.0, 1.0, 1.0))],
            decorations: Vec::new(),
            continue_animation: false,
        };
        submit_resolved_frame(app.world_mut(), left_id.clone(), left_view, &frame).unwrap();
        submit_resolved_frame(app.world_mut(), right_id.clone(), right_view, &frame).unwrap();
        app.update();

        let lights = app
            .world_mut()
            .query_filtered::<(&PuzzleBevyRenderView, &DirectionalLight), With<PuzzleRendererLight>>()
            .iter(app.world())
            .map(|(view, light)| {
                (
                    view.id.clone(),
                    (light.color.clone(), light.illuminance),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(
            lights[&left_id],
            (Color::linear_rgb(1.0, 0.0, 0.0), 1_000.0)
        );
        assert_eq!(
            lights[&right_id],
            (Color::linear_rgb(0.0, 0.0, 1.0), 3_000.0)
        );
        let ambient = app
            .world_mut()
            .query_filtered::<(&PuzzleBevyRenderView, &AmbientLight), With<PuzzleRendererCamera>>()
            .iter(app.world())
            .map(|(view, light)| (view.id.clone(), (light.color.clone(), light.brightness)))
            .collect::<HashMap<_, _>>();
        assert_eq!(ambient[&left_id], (Color::linear_rgb(1.0, 0.0, 0.0), 100.0));
        assert_eq!(
            ambient[&right_id],
            (Color::linear_rgb(0.0, 0.0, 1.0), 300.0)
        );
    }

    #[test]
    fn rejects_wrong_dimension_and_duplicate_camera_order() {
        let mut queue = BevyResolvedFrameQueue::default();
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: Vec::new(),
            continue_animation: false,
        };
        let wrong_dimension = PuzzleBevyViewId::two_d("board", "main");
        assert_eq!(
            queue.submit(wrong_dimension.clone(), view_3d(UVec2::ZERO), &frame),
            Err(BevyRenderError::ViewDimensionMismatch {
                view_id: wrong_dimension,
                expected: PuzzleBevyViewDimension::ThreeD,
            })
        );

        queue
            .submit(
                PuzzleBevyViewId::three_d("left", "main"),
                view_3d(UVec2::ZERO),
                &frame,
            )
            .unwrap();
        assert_eq!(
            queue.submit(
                PuzzleBevyViewId::three_d("right", "main"),
                view_3d(UVec2::new(320, 0)),
                &frame,
            ),
            Err(BevyRenderError::DuplicateCameraOrder { order: 0 })
        );
        assert_eq!(queue.registry.view_layers.len(), 1);
    }

    #[test]
    fn removal_releases_3d_view_reservations_before_deferred_ecs_cleanup() {
        let mut queue = BevyResolvedFrameQueue::default();
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: Vec::new(),
            continue_animation: false,
        };
        let outgoing = PuzzleBevyViewId::three_d("outgoing", "main");
        let incoming = PuzzleBevyViewId::three_d("incoming", "main");

        queue
            .submit(outgoing.clone(), view_3d(UVec2::ZERO), &frame)
            .unwrap();
        queue.remove(&outgoing).unwrap();
        queue
            .submit(incoming.clone(), view_3d(UVec2::ZERO), &frame)
            .unwrap();

        assert!(!queue.registry.registered_views.contains(&outgoing));
        assert!(queue.registry.registered_views.contains(&incoming));
        assert_eq!(
            queue.registry.order_owners.get(&0),
            Some(&incoming),
            "the accepted removal must end logical camera ownership immediately"
        );
    }

    #[test]
    fn retained_3d_views_can_swap_camera_orders_in_one_submission() {
        let mut queue = BevyResolvedFrameQueue::default();
        let frame = RuntimeResolvedRenderFrame {
            batches: Vec::new(),
            decorations: Vec::new(),
            continue_animation: false,
        };
        let left = PuzzleBevyViewId::three_d("left", "main");
        let right = PuzzleBevyViewId::three_d("right", "main");
        queue
            .submit(left.clone(), view_3d(UVec2::ZERO), &frame)
            .unwrap();
        queue
            .submit(
                right.clone(),
                PuzzleBevy3dView {
                    order: 1,
                    ..view_3d(UVec2::new(320, 0))
                },
                &frame,
            )
            .unwrap();

        queue
            .reconcile_camera_orders(&BTreeMap::from([(left.clone(), 1), (right.clone(), 0)]))
            .unwrap();
        queue
            .submit(
                left.clone(),
                PuzzleBevy3dView {
                    order: 1,
                    ..view_3d(UVec2::ZERO)
                },
                &frame,
            )
            .unwrap();
        queue
            .submit(right.clone(), view_3d(UVec2::new(320, 0)), &frame)
            .unwrap();
        assert_eq!(queue.registry.order_owners.get(&0), Some(&right));
        assert_eq!(queue.registry.order_owners.get(&2), Some(&left));
    }

    #[test]
    fn desired_camera_order_reconciliation_rejects_duplicates_without_partial_commit() {
        let mut queue = BevyResolvedFrameQueue::default();
        let left = PuzzleBevyViewId::three_d("left", "main");
        let right = PuzzleBevyViewId::three_d("right", "main");
        queue
            .reconcile_camera_orders(&BTreeMap::from([(left.clone(), 0), (right.clone(), 1)]))
            .unwrap();

        assert_eq!(
            queue
                .reconcile_camera_orders(&BTreeMap::from([(left.clone(), 0), (right.clone(), 0),])),
            Err(BevyRenderError::DuplicateCameraOrder { order: 0 })
        );
        assert_eq!(queue.registry.order_owners.get(&0), Some(&left));
        assert_eq!(queue.registry.order_owners.get(&2), Some(&right));
    }

    #[test]
    fn consumes_the_presentation_owner_frame_without_json_or_a_second_projection() {
        let scene = RuntimeResolvedRenderScene {
            clips: vec![RuntimeResolvedVisualClip {
                id: "Actor".to_string(),
                frames: vec![RuntimeResolvedVisualFrame::Voxels {
                    width: 1,
                    depth: 1,
                    height: 1,
                    voxels: vec![RuntimeResolvedVoxel {
                        position: [0, 0, 0],
                        color: color(0.25, 0.5, 0.75, 1.0),
                    }],
                }],
                frame_duration_ms: None,
                layout: RuntimeResolvedVisualLayout {
                    fit: RuntimeResolvedFitMode::Contain,
                    width: 1,
                    height: 1,
                },
            }],
            instances: vec![RuntimeResolvedRenderInstance {
                id: 1,
                object_id: Some(42),
                visual: "Actor".to_string(),
                cell: [1, 2, 3],
                transform: identity(),
                opacity: 1.0,
                frame_elapsed_ms: None,
                playback: RuntimeResolvedPlayback::Loop,
                render_order: 9,
            }],
            composition_groups: vec![RuntimeResolvedCompositionGroup {
                render_order: 9,
                composition: RuntimeVisualComposition::Ordered,
                instances: vec![1],
            }],
            cells: Vec::new(),
            decorations: Vec::new(),
            render_priority_count: 1,
            animation_duration_ms: 250,
        };

        let resolved = puzzle_presentation::resolve_image_free_render_frame(&scene, 0).unwrap();
        let prepared = prepare_resolved_frame(&resolved).unwrap();

        assert_eq!(prepared.voxels.len(), 1);
        assert_eq!(prepared.voxels[0].object_ids, vec![42]);
        assert_eq!(
            prepared.voxels[0].color,
            LinearRgba::new(0.25, 0.5, 0.75, 1.0)
        );
    }
}
