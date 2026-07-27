use std::collections::{BTreeSet, HashSet};

use bevy::prelude::{Vec2, Vec3};
use puzzle_bevy_renderer::{
    PreparedBounds, PuzzleBevyCamera, puzzle_bevy_camera_ray, puzzle_bevy_point_to_visual,
    puzzle_visual_point_to_bevy,
};
use puzzle_editor_preview_contract::{
    EditorAuthoringHitTarget, EditorAuthoringInteraction, EditorAuthoringPresentation,
    EditorAuthoringSurface, EditorGrid3dSettings, EditorGridAxis, EditorGridPosition,
    EditorGridSide, EditorPaintOperation, EditorRendererStrategy, EditorResizeMode,
};
use puzzle_runtime_contract::{
    RuntimeLinearRgba, RuntimePuzzle3CameraProjection, RuntimePuzzle3Snapshot,
    RuntimeResolvedDecoration, RuntimeResolvedLineDepth3d, RuntimeResolvedLineLayer2d,
    RuntimeResolvedLineSegment2d, RuntimeResolvedLineSegment3d, RuntimeResolvedLineStyle,
    RuntimeResolvedStrokeWidth, RuntimeStateSnapshot, RuntimeTheme, RuntimeViewportSourceId,
    SolverStateSnapshot,
};
use puzzle_session_contract::{RuntimePuzzle2Snapshot, RuntimeRendererState};

#[derive(Clone)]
pub(crate) struct EditorAuthoringConfiguration {
    pub(crate) surface: EditorAuthoringSurface,
    pub(crate) renderer: EditorRendererStrategy,
    pub(crate) viewport_source: RuntimeViewportSourceId,
    pub(crate) decoration_base_len: usize,
    pub(crate) highlight: Option<EditorAuthoringHitTarget>,
}

#[derive(Clone)]
pub struct EditorAuthoringFrame {
    pub surface_id: String,
    pub revision: u64,
    pub css_size: Vec2,
    interaction: EditorAuthoringInteraction,
    geometry: EditorAuthoringFrameGeometry,
}

#[derive(Clone)]
enum EditorAuthoringFrameGeometry {
    Grid2d {
        origin: [i32; 2],
        size: [u16; 2],
        occupied: HashSet<[u16; 2]>,
    },
    Grid3d {
        camera: PuzzleBevyCamera,
        bounds: PreparedBounds,
        size: [u16; 3],
        occupied: HashSet<[u16; 3]>,
        slice_z: Option<u16>,
    },
}

impl EditorAuthoringConfiguration {
    pub(crate) fn new(
        presentation: EditorAuthoringPresentation,
        viewport_source: RuntimeViewportSourceId,
    ) -> Self {
        Self {
            surface: presentation.surface,
            renderer: presentation.renderer,
            viewport_source,
            decoration_base_len: 0,
            highlight: None,
        }
    }

    pub(crate) fn validate_for_state(&self, state: &RuntimeStateSnapshot) -> Result<(), String> {
        if self.surface.surface_id.trim().is_empty() {
            return Err("editor authoring surface id must not be empty".to_string());
        }
        match (&self.renderer, state) {
            (EditorRendererStrategy::Grid2d, RuntimeStateSnapshot::TwoD(state)) => {
                validate_grid_size([state.width, state.height])
            }
            (
                EditorRendererStrategy::Grid3d {
                    slice_z,
                    hidden_layers,
                    ..
                },
                RuntimeStateSnapshot::ThreeD(state),
            ) => {
                validate_grid_size([state.width, state.depth, state.height])?;
                validate_grid3d_selection(
                    *slice_z,
                    hidden_layers,
                    state.height,
                    state.layer_count,
                )?;
                self.validate_camera()
            }
            (EditorRendererStrategy::Grid2d, RuntimeStateSnapshot::ThreeD(_)) => Err(
                "editor grid2d renderer strategy cannot configure a grid3d runtime state"
                    .to_string(),
            ),
            (EditorRendererStrategy::Grid3d { .. }, RuntimeStateSnapshot::TwoD(_)) => Err(
                "editor grid3d renderer strategy cannot configure a grid2d runtime state"
                    .to_string(),
            ),
        }
    }

    pub(crate) fn accepts_model_input(&self) -> bool {
        self.surface.interaction == EditorAuthoringInteraction::Play
    }

    pub(crate) fn validate_for_solver_state(
        &self,
        state: &SolverStateSnapshot,
    ) -> Result<(), String> {
        match (&self.renderer, state) {
            (EditorRendererStrategy::Grid2d, SolverStateSnapshot::TwoD { width, height, .. }) => {
                validate_grid_size([*width, *height])
            }
            (
                EditorRendererStrategy::Grid3d {
                    slice_z,
                    hidden_layers,
                    ..
                },
                SolverStateSnapshot::ThreeD {
                    width,
                    depth,
                    height,
                    layer_count,
                    ..
                },
            ) => {
                validate_grid_size([*width, *depth, *height])?;
                validate_grid3d_selection(*slice_z, hidden_layers, *height, *layer_count)?;
                self.validate_camera()
            }
            (EditorRendererStrategy::Grid2d, SolverStateSnapshot::ThreeD { .. }) => Err(
                "editor grid2d renderer strategy cannot configure a grid3d runtime state"
                    .to_string(),
            ),
            (EditorRendererStrategy::Grid3d { .. }, SolverStateSnapshot::TwoD { .. }) => Err(
                "editor grid3d renderer strategy cannot configure a grid2d runtime state"
                    .to_string(),
            ),
        }
    }

    pub(crate) fn apply_to_renderer(
        &mut self,
        renderer: &mut RuntimeRendererState,
        theme: &RuntimeTheme,
    ) -> Result<(), String> {
        match renderer {
            RuntimeRendererState::TwoD(scene) => self.apply_to_scene2d(scene, theme),
            RuntimeRendererState::ThreeD(scene) => self.apply_to_scene3d(scene, theme),
        }
    }

    fn apply_to_scene2d(
        &mut self,
        scene: &mut RuntimePuzzle2Snapshot,
        theme: &RuntimeTheme,
    ) -> Result<(), String> {
        let EditorRendererStrategy::Grid2d = self.renderer else {
            return Err(
                "editor grid3d renderer strategy cannot configure a grid2d scene".to_string(),
            );
        };
        self.decoration_base_len = scene.render_scene.decorations.len();
        if let Some(hit) = &self.highlight {
            append_highlight2d(
                &mut scene.render_scene.decorations,
                scene.view,
                hit,
                theme.accent,
            )?;
        }
        Ok(())
    }

    fn apply_to_scene3d(
        &mut self,
        scene: &mut RuntimePuzzle3Snapshot,
        theme: &RuntimeTheme,
    ) -> Result<(), String> {
        let EditorRendererStrategy::Grid3d {
            slice_z,
            hidden_layers,
            camera,
            view: _,
            settings,
        } = &self.renderer
        else {
            return Err(
                "grid2d editor renderer strategy cannot configure a grid3d scene".to_string(),
            );
        };
        let size = [scene.size.width, scene.size.depth, scene.size.height];
        if slice_z.is_some_and(|slice| slice >= scene.size.height) {
            return Err(format!(
                "editor grid3d sliceZ {} is outside height {}",
                slice_z.unwrap(),
                scene.size.height
            ));
        }
        let hidden = hidden_layers.iter().copied().collect::<HashSet<_>>();
        for cell in &mut scene.cells {
            cell.objects
                .retain(|object| !hidden.contains(&object.layer));
        }
        if let Some(slice_z) = *slice_z {
            scene.cells.retain(|cell| cell.position.z == Some(slice_z));
        }
        let visible_object_ids = scene
            .cells
            .iter()
            .flat_map(|cell| cell.objects.iter().map(|object| object.id))
            .collect::<HashSet<_>>();
        scene.render_scene.instances.retain(|instance| {
            let on_slice = slice_z.is_none_or(|slice| instance.cell[2] == i32::from(slice));
            on_slice
                && instance
                    .object_id
                    .is_none_or(|object| visible_object_ids.contains(&object))
        });
        let visible_instance_ids = scene
            .render_scene
            .instances
            .iter()
            .map(|instance| instance.id)
            .collect::<HashSet<_>>();
        for group in &mut scene.render_scene.composition_groups {
            group
                .instances
                .retain(|instance| visible_instance_ids.contains(instance));
        }
        scene
            .render_scene
            .composition_groups
            .retain(|group| !group.instances.is_empty());
        scene.render_scene.cells.retain_mut(|cell| {
            if slice_z.is_some_and(|slice| cell.position[2] != i32::from(slice)) {
                return false;
            }
            cell.object_ids
                .retain(|object| visible_object_ids.contains(object));
            !cell.object_ids.is_empty()
        });

        scene.render.camera.projection = camera.projection;
        scene.render.camera.zoom = camera.zoom;
        if !scene.render.camera.zoom.is_finite() || scene.render.camera.zoom <= 0.0 {
            return Err("editor authoring camera zoom must be positive".to_string());
        }
        self.decoration_base_len = scene.render_scene.decorations.len();
        append_authoring_decorations(
            &mut scene.render_scene.decorations,
            size,
            &scene.cells,
            *slice_z,
            settings,
            theme,
        )?;
        self.decoration_base_len = scene.render_scene.decorations.len();
        if let Some(hit) = &self.highlight {
            append_highlight3d(&mut scene.render_scene.decorations, size, hit, theme.accent)?;
        }
        Ok(())
    }

    fn validate_camera(&self) -> Result<(), String> {
        let mut camera = PuzzleBevyCamera::default();
        self.apply_camera(&mut camera)
    }

    pub(crate) fn apply_camera(&self, camera: &mut PuzzleBevyCamera) -> Result<(), String> {
        let EditorRendererStrategy::Grid3d {
            camera: source,
            view,
            ..
        } = &self.renderer
        else {
            return Err(
                "grid2d editor renderer strategy cannot configure a grid3d camera".to_string(),
            );
        };
        for (label, value) in [
            ("yawDegrees", source.yaw_degrees),
            ("pitchDegrees", source.pitch_degrees),
            ("rollDegrees", source.roll_degrees),
            ("camera.zoom", source.zoom),
            ("view.target.x", view.target.x),
            ("view.target.y", view.target.y),
            ("view.target.z", view.target.z),
        ] {
            if !value.is_finite() {
                return Err(format!("editor authoring {label} must be finite"));
            }
        }
        let zoom = source.zoom;
        if zoom <= 0.0 {
            return Err("editor authoring camera zoom must be positive".to_string());
        }
        camera.projection = match source.projection {
            RuntimePuzzle3CameraProjection::Perspective => {
                puzzle_bevy_renderer::PuzzleCameraProjection::Perspective
            }
            RuntimePuzzle3CameraProjection::Orthographic => {
                puzzle_bevy_renderer::PuzzleCameraProjection::Orthographic
            }
        };
        camera.yaw_degrees = source.yaw_degrees as f32;
        camera.pitch_degrees = source.pitch_degrees as f32;
        camera.roll_degrees = source.roll_degrees as f32;
        camera.distance_scale = 2.8 / zoom as f32;
        camera.target = Some(puzzle_visual_point_to_bevy(Vec3::new(
            view.target.x as f32,
            view.target.y as f32,
            view.target.z as f32,
        )));
        Ok(())
    }

    pub(crate) fn set_highlight(
        &mut self,
        renderer: &mut RuntimeRendererState,
        hit: Option<EditorAuthoringHitTarget>,
        theme: &RuntimeTheme,
    ) -> Result<(), String> {
        match renderer {
            RuntimeRendererState::TwoD(scene) => {
                scene
                    .render_scene
                    .decorations
                    .truncate(self.decoration_base_len);
                if let Some(hit) = &hit {
                    append_highlight2d(
                        &mut scene.render_scene.decorations,
                        scene.view,
                        hit,
                        theme.accent,
                    )?;
                }
            }
            RuntimeRendererState::ThreeD(scene) => {
                scene
                    .render_scene
                    .decorations
                    .truncate(self.decoration_base_len);
                if let Some(hit) = &hit {
                    append_highlight3d(
                        &mut scene.render_scene.decorations,
                        [scene.size.width, scene.size.depth, scene.size.height],
                        hit,
                        theme.accent,
                    )?;
                }
            }
        }
        self.highlight = hit;
        Ok(())
    }

    pub(crate) fn frame3d(
        &self,
        scene: &RuntimePuzzle3Snapshot,
        revision: u64,
        css_size: Vec2,
        camera: PuzzleBevyCamera,
        bounds: PreparedBounds,
    ) -> Result<EditorAuthoringFrame, String> {
        let EditorRendererStrategy::Grid3d { slice_z, .. } = &self.renderer else {
            return Err("editor grid2d renderer strategy cannot commit a grid3d frame".to_string());
        };
        validate_css_size(css_size)?;
        let mut occupied = HashSet::new();
        for cell in scene.cells.iter().filter(|cell| !cell.objects.is_empty()) {
            let z = cell.position.z.ok_or_else(|| {
                "editor grid3d renderer emitted a cell without a z coordinate".to_string()
            })?;
            occupied.insert([cell.position.x, cell.position.y, z]);
        }
        Ok(EditorAuthoringFrame {
            surface_id: self.surface.surface_id.clone(),
            revision,
            css_size,
            interaction: self.surface.interaction,
            geometry: EditorAuthoringFrameGeometry::Grid3d {
                camera,
                bounds,
                size: [scene.size.width, scene.size.depth, scene.size.height],
                occupied,
                slice_z: *slice_z,
            },
        })
    }

    pub(crate) fn frame2d(
        &self,
        scene: &RuntimePuzzle2Snapshot,
        revision: u64,
        css_size: Vec2,
    ) -> Result<EditorAuthoringFrame, String> {
        let EditorRendererStrategy::Grid2d = &self.renderer else {
            return Err("editor grid3d renderer strategy cannot commit a grid2d frame".to_string());
        };
        validate_css_size(css_size)?;
        let mut occupied = HashSet::new();
        for cell in scene
            .render_scene
            .cells
            .iter()
            .filter(|cell| !cell.object_ids.is_empty())
        {
            let x = u16::try_from(cell.position[0]).map_err(|_| {
                "editor grid2d renderer emitted a cell outside authored x coordinates".to_string()
            })?;
            let y = u16::try_from(cell.position[1]).map_err(|_| {
                "editor grid2d renderer emitted a cell outside authored y coordinates".to_string()
            })?;
            occupied.insert([x, y]);
        }
        Ok(EditorAuthoringFrame {
            surface_id: self.surface.surface_id.clone(),
            revision,
            css_size,
            interaction: self.surface.interaction,
            geometry: EditorAuthoringFrameGeometry::Grid2d {
                origin: scene.view.origin,
                size: scene.view.size,
                occupied,
            },
        })
    }
}

impl EditorAuthoringFrame {
    pub(crate) fn validate_identity(
        &self,
        surface_id: &str,
        committed_revision: u64,
    ) -> Result<(), String> {
        if surface_id != self.surface_id {
            return Err(format!(
                "editor pointer surface `{surface_id}` does not match committed surface `{}`",
                self.surface_id
            ));
        }
        if committed_revision != self.revision {
            return Err(format!(
                "editor pointer frame revision {committed_revision} is stale; committed revision is {}",
                self.revision
            ));
        }
        Ok(())
    }

    pub fn hit(
        &self,
        surface_id: &str,
        committed_revision: u64,
        point_css: Vec2,
    ) -> Result<Option<EditorAuthoringHitTarget>, String> {
        self.validate_identity(surface_id, committed_revision)?;
        validate_css_size(self.css_size)?;
        if !point_css.is_finite()
            || point_css.x < 0.0
            || point_css.y < 0.0
            || point_css.x >= self.css_size.x
            || point_css.y >= self.css_size.y
        {
            return Err("editor pointer coordinates are outside the committed surface".to_string());
        }
        let normalized = Vec2::new(point_css.x / self.css_size.x, point_css.y / self.css_size.y);
        match &self.geometry {
            EditorAuthoringFrameGeometry::Grid2d {
                origin,
                size,
                occupied,
            } => hit_grid2d(normalized, *origin, *size, occupied, self.interaction),
            EditorAuthoringFrameGeometry::Grid3d {
                camera,
                bounds,
                size,
                occupied,
                slice_z,
            } => {
                let ray = puzzle_bevy_camera_ray(
                    camera,
                    *bounds,
                    normalized,
                    self.css_size.x / self.css_size.y,
                )
                .map_err(|error| format!("editor pointer camera projection failed: {error}"))?;
                let origin = puzzle_bevy_point_to_visual(ray.origin);
                let direction = puzzle_bevy_point_to_visual(ray.direction);
                match self.interaction {
                    EditorAuthoringInteraction::Paint { operation } => {
                        Self::paint_hit3d(origin, direction, *size, occupied, *slice_z, operation)
                    }
                    EditorAuthoringInteraction::Resize { mode } => {
                        Self::resize_hit3d(origin, direction, *size, mode)
                    }
                    EditorAuthoringInteraction::Play | EditorAuthoringInteraction::Observe => {
                        Ok(None)
                    }
                }
            }
        }
    }

    pub(crate) fn hit_for_gesture(
        &self,
        surface_id: &str,
        committed_revision: u64,
        point_css: Vec2,
        gesture: puzzle_editor_preview_contract::EditorPointerGesture,
    ) -> Result<Option<EditorAuthoringHitTarget>, String> {
        match gesture {
            puzzle_editor_preview_contract::EditorPointerGesture::Move
            | puzzle_editor_preview_contract::EditorPointerGesture::Press
            | puzzle_editor_preview_contract::EditorPointerGesture::Release => {
                self.hit(surface_id, committed_revision, point_css)
            }
            puzzle_editor_preview_contract::EditorPointerGesture::Leave => {
                self.validate_identity(surface_id, committed_revision)?;
                Ok(None)
            }
        }
    }

    fn paint_hit3d(
        origin: Vec3,
        direction: Vec3,
        size: [u16; 3],
        occupied: &HashSet<[u16; 3]>,
        slice_z: Option<u16>,
        operation: EditorPaintOperation,
    ) -> Result<Option<EditorAuthoringHitTarget>, String> {
        if let Some(z) = slice_z {
            let Some(position) = ray_grid_plane(origin, direction, size, z) else {
                return Ok(None);
            };
            let occupied = occupied.contains(&position);
            return Ok(match (operation, occupied) {
                (EditorPaintOperation::Erase | EditorPaintOperation::Replace, true) => {
                    Some(EditorAuthoringHitTarget::Cell {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    })
                }
                (EditorPaintOperation::Add | EditorPaintOperation::Replace, false) => {
                    Some(EditorAuthoringHitTarget::Placement {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    })
                }
                (EditorPaintOperation::Add, true) | (EditorPaintOperation::Erase, false) => None,
            });
        }

        let occupied_hit = occupied
            .iter()
            .filter_map(|position| {
                ray_cell(origin, direction, *position)
                    .map(|(distance, face)| (distance, *position, face))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0));
        match operation {
            EditorPaintOperation::Erase => {
                Ok(
                    occupied_hit.map(|(_, position, _)| EditorAuthoringHitTarget::Cell {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    }),
                )
            }
            EditorPaintOperation::Replace => {
                if let Some((_, position, _)) = occupied_hit {
                    return Ok(Some(EditorAuthoringHitTarget::Cell {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    }));
                }
                Ok(ray_floor_cell(origin, direction, size).map(|position| {
                    EditorAuthoringHitTarget::Placement {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    }
                }))
            }
            EditorPaintOperation::Add => {
                if let Some((_, position, face)) = occupied_hit {
                    let candidate = [
                        i32::from(position[0]) + face[0],
                        i32::from(position[1]) + face[1],
                        i32::from(position[2]) + face[2],
                    ];
                    if let Some(candidate) = bounded_position(candidate, size)
                        && !occupied.contains(&candidate)
                    {
                        return Ok(Some(EditorAuthoringHitTarget::Placement {
                            position: EditorGridPosition::Grid3d(position3d(candidate)),
                        }));
                    }
                }
                Ok(ray_floor_cell(origin, direction, size)
                    .filter(|position| !occupied.contains(position))
                    .map(|position| EditorAuthoringHitTarget::Placement {
                        position: EditorGridPosition::Grid3d(position3d(position)),
                    }))
            }
        }
    }

    fn resize_hit3d(
        origin: Vec3,
        direction: Vec3,
        size: [u16; 3],
        mode: EditorResizeMode,
    ) -> Result<Option<EditorAuthoringHitTarget>, String> {
        let candidates = resize_boxes(size, mode);
        Ok(candidates
            .into_iter()
            .filter_map(|candidate| {
                ray_box(origin, direction, candidate.min, candidate.max)
                    .map(|(distance, _)| (distance, candidate))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, candidate)| EditorAuthoringHitTarget::Resize {
                mode,
                axis: candidate.axis,
                side: candidate.side,
            }))
    }
}

fn hit_grid2d(
    normalized: Vec2,
    origin: [i32; 2],
    size: [u16; 2],
    occupied: &HashSet<[u16; 2]>,
    interaction: EditorAuthoringInteraction,
) -> Result<Option<EditorAuthoringHitTarget>, String> {
    if matches!(
        interaction,
        EditorAuthoringInteraction::Play | EditorAuthoringInteraction::Observe
    ) {
        return Ok(None);
    }
    if let EditorAuthoringInteraction::Resize { mode } = interaction {
        let candidates = [
            (
                normalized.x,
                EditorGridAxis::X,
                EditorGridSide::Min,
                size[0],
            ),
            (
                1.0 - normalized.x,
                EditorGridAxis::X,
                EditorGridSide::Max,
                size[0],
            ),
            (
                normalized.y,
                EditorGridAxis::Y,
                EditorGridSide::Min,
                size[1],
            ),
            (
                1.0 - normalized.y,
                EditorGridAxis::Y,
                EditorGridSide::Max,
                size[1],
            ),
        ];
        return Ok(candidates
            .into_iter()
            .filter(|(_, _, _, length)| mode == EditorResizeMode::Expand || *length > 1)
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .filter(|(distance, _, _, _)| *distance <= 0.12)
            .map(|(_, axis, side, _)| EditorAuthoringHitTarget::Resize { mode, axis, side }));
    }
    let local_x = (normalized.x * f32::from(size[0])).floor() as i32;
    let local_y = (normalized.y * f32::from(size[1])).floor() as i32;
    if local_x < 0 || local_y < 0 || local_x >= i32::from(size[0]) || local_y >= i32::from(size[1])
    {
        return Ok(None);
    }
    let x = u16::try_from(origin[0] + local_x)
        .map_err(|_| "editor grid2d pointer resolved outside authored coordinates".to_string())?;
    let y = u16::try_from(origin[1] + local_y)
        .map_err(|_| "editor grid2d pointer resolved outside authored coordinates".to_string())?;
    let position = [x, y];
    let occupied = occupied.contains(&position);
    let EditorAuthoringInteraction::Paint { operation } = interaction else {
        unreachable!("play, observe, and resize interactions returned before paint");
    };
    let position = EditorGridPosition::Grid2d(puzzle_authoring::EditorDraftPosition2d { x, y });
    Ok(match (operation, occupied) {
        (EditorPaintOperation::Erase | EditorPaintOperation::Replace, true) => {
            Some(EditorAuthoringHitTarget::Cell { position })
        }
        (EditorPaintOperation::Add | EditorPaintOperation::Replace, false) => {
            Some(EditorAuthoringHitTarget::Placement { position })
        }
        (EditorPaintOperation::Add, true) | (EditorPaintOperation::Erase, false) => None,
    })
}

fn position3d(value: [u16; 3]) -> puzzle_authoring::EditorDraftPosition3d {
    puzzle_authoring::EditorDraftPosition3d {
        x: value[0],
        y: value[1],
        z: value[2],
    }
}

fn validate_grid_size<const DIMENSIONS: usize>(size: [u16; DIMENSIONS]) -> Result<(), String> {
    if size.into_iter().any(|extent| extent == 0) {
        return Err("editor authoring grid dimensions must be positive".to_string());
    }
    Ok(())
}

fn validate_grid3d_selection(
    slice_z: Option<u16>,
    hidden_layers: &[u16],
    height: u16,
    layer_count: u16,
) -> Result<(), String> {
    if let Some(slice) = slice_z
        && slice >= height
    {
        return Err(format!(
            "editor grid3d sliceZ {slice} is outside height {height}"
        ));
    }
    if let Some(layer) = hidden_layers
        .iter()
        .copied()
        .find(|layer| *layer >= layer_count)
    {
        return Err(format!(
            "editor grid3d hidden layer {layer} is outside layer count {layer_count}"
        ));
    }
    Ok(())
}

fn validate_css_size(css_size: Vec2) -> Result<(), String> {
    if !css_size.is_finite() || css_size.x <= 0.0 || css_size.y <= 0.0 {
        return Err(
            "editor authoring committed CSS size must be finite and greater than zero".to_string(),
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ResizeBox {
    min: Vec3,
    max: Vec3,
    axis: EditorGridAxis,
    side: EditorGridSide,
}

fn resize_boxes(size: [u16; 3], mode: EditorResizeMode) -> Vec<ResizeBox> {
    let max = Vec3::new(
        f32::from(size[0]) - 0.5,
        f32::from(size[1]) - 0.5,
        f32::from(size[2]) - 0.5,
    );
    let mut boxes = Vec::new();
    let thickness = 0.12;
    let offset = if mode == EditorResizeMode::Expand {
        1.0
    } else {
        0.0
    };
    let mut push = |grid_axis, side, axis: usize, high: bool| {
        if mode == EditorResizeMode::Shrink && size[axis] <= 1 {
            return;
        }
        let mut min = Vec3::splat(-0.5);
        let mut bound = max;
        let plane = if high {
            max[axis] + offset
        } else {
            -0.5 - offset
        };
        min[axis] = plane - thickness;
        bound[axis] = plane + thickness;
        boxes.push(ResizeBox {
            min,
            max: bound,
            axis: grid_axis,
            side,
        });
    };
    push(EditorGridAxis::X, EditorGridSide::Min, 0, false);
    push(EditorGridAxis::X, EditorGridSide::Max, 0, true);
    push(EditorGridAxis::Y, EditorGridSide::Min, 1, false);
    push(EditorGridAxis::Y, EditorGridSide::Max, 1, true);
    push(EditorGridAxis::Z, EditorGridSide::Min, 2, false);
    push(EditorGridAxis::Z, EditorGridSide::Max, 2, true);
    boxes
}

fn ray_cell(origin: Vec3, direction: Vec3, position: [u16; 3]) -> Option<(f32, [i32; 3])> {
    let center = Vec3::new(
        f32::from(position[0]),
        f32::from(position[1]),
        f32::from(position[2]),
    );
    ray_box(
        origin,
        direction,
        center - Vec3::splat(0.5),
        center + Vec3::splat(0.5),
    )
}

fn ray_box(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<(f32, [i32; 3])> {
    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;
    let mut face = [0; 3];
    for axis in 0..3 {
        if direction[axis].abs() < 0.000_001 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let first = (min[axis] - origin[axis]) / direction[axis];
        let second = (max[axis] - origin[axis]) / direction[axis];
        let (axis_near, axis_far, normal) = if first <= second {
            (first, second, -1)
        } else {
            (second, first, 1)
        };
        if axis_near > near {
            near = axis_near;
            face = [0; 3];
            face[axis] = normal;
        }
        far = far.min(axis_far);
        if near > far {
            return None;
        }
    }
    let distance = if near >= 0.0 { near } else { far };
    (distance >= 0.0).then_some((distance, face))
}

fn ray_grid_plane(origin: Vec3, direction: Vec3, size: [u16; 3], z: u16) -> Option<[u16; 3]> {
    let distance = (f32::from(z) + 0.5 - origin.z) / direction.z;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    let point = origin + direction * distance;
    bounded_position(
        [point.x.floor() as i32, point.y.floor() as i32, i32::from(z)],
        size,
    )
}

fn ray_floor_cell(origin: Vec3, direction: Vec3, size: [u16; 3]) -> Option<[u16; 3]> {
    let distance = (-0.5 - origin.z) / direction.z;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    let point = origin + direction * distance;
    bounded_position([point.x.floor() as i32, point.y.floor() as i32, 0], size)
}

fn bounded_position(position: [i32; 3], size: [u16; 3]) -> Option<[u16; 3]> {
    let position = position.map(u16::try_from);
    let [Ok(x), Ok(y), Ok(z)] = position else {
        return None;
    };
    (x < size[0] && y < size[1] && z < size[2]).then_some([x, y, z])
}

fn append_authoring_decorations(
    decorations: &mut Vec<RuntimeResolvedDecoration>,
    size: [u16; 3],
    cells: &[puzzle_runtime_contract::RuntimePuzzle3Cell],
    slice_z: Option<u16>,
    settings: &EditorGrid3dSettings,
    theme: &RuntimeTheme,
) -> Result<(), String> {
    if settings.stage_frame {
        decorations.push(lines_decoration(
            box_edges(
                Vec3::splat(-0.5),
                Vec3::new(
                    f32::from(size[0]) - 0.5,
                    f32::from(size[1]) - 0.5,
                    f32::from(size[2]) - 0.5,
                ),
            ),
            theme.muted_text,
        ));
    }
    if settings.grid_visible {
        let segments = if let Some(z) = slice_z {
            layer_grid_edges(size, z)
        } else {
            stage_grid_edges(size)
        };
        decorations.push(lines_decoration(
            segments,
            translucent(theme.muted_text, 0.32),
        ));
    }
    if settings.occupied_cell_frames {
        let mut segments = Vec::new();
        for cell in cells.iter().filter(|cell| !cell.objects.is_empty()) {
            let z = cell.position.z.ok_or_else(|| {
                "editor grid3d renderer emitted a cell without a z coordinate".to_string()
            })?;
            let center = Vec3::new(
                f32::from(cell.position.x),
                f32::from(cell.position.y),
                f32::from(z),
            );
            segments.extend(box_edges(
                center - Vec3::splat(0.5),
                center + Vec3::splat(0.5),
            ));
        }
        decorations.push(lines_decoration(segments, translucent(theme.text, 0.55)));
    }
    Ok(())
}

fn append_highlight3d(
    decorations: &mut Vec<RuntimeResolvedDecoration>,
    size: [u16; 3],
    hit: &EditorAuthoringHitTarget,
    color: RuntimeLinearRgba,
) -> Result<(), String> {
    let segments = match hit {
        EditorAuthoringHitTarget::Cell {
            position: EditorGridPosition::Grid3d(position),
        }
        | EditorAuthoringHitTarget::Placement {
            position: EditorGridPosition::Grid3d(position),
        } => {
            let center = Vec3::new(
                f32::from(position.x),
                f32::from(position.y),
                f32::from(position.z),
            );
            box_edges(center - Vec3::splat(0.53), center + Vec3::splat(0.53))
        }
        EditorAuthoringHitTarget::Resize {
            mode, axis, side, ..
        } => {
            let candidate = resize_boxes(size, *mode)
                .into_iter()
                .find(|candidate| candidate.axis == *axis && candidate.side == *side)
                .ok_or_else(|| {
                    format!(
                        "editor grid3d resize highlight has no {axis:?} {side:?} face for {mode:?}"
                    )
                })?;
            box_edges(candidate.min, candidate.max)
        }
        EditorAuthoringHitTarget::Cell {
            position: EditorGridPosition::Grid2d(_),
        }
        | EditorAuthoringHitTarget::Placement {
            position: EditorGridPosition::Grid2d(_),
        } => {
            return Err("editor grid2d hit target cannot highlight a grid3d renderer".to_string());
        }
    };
    decorations.push(lines_decoration(segments, color));
    Ok(())
}

fn append_highlight2d(
    decorations: &mut Vec<RuntimeResolvedDecoration>,
    view: puzzle_runtime_contract::RuntimeResolvedView2d,
    hit: &EditorAuthoringHitTarget,
    color: RuntimeLinearRgba,
) -> Result<(), String> {
    let [origin_x, origin_y] = view.origin;
    let max_x = origin_x + i32::from(view.size[0]);
    let max_y = origin_y + i32::from(view.size[1]);
    let segments = match hit {
        EditorAuthoringHitTarget::Cell {
            position: EditorGridPosition::Grid2d(position),
        }
        | EditorAuthoringHitTarget::Placement {
            position: EditorGridPosition::Grid2d(position),
        } => {
            let x = i32::from(position.x);
            let y = i32::from(position.y);
            vec![
                line2d([x, y], [x + 1, y]),
                line2d([x + 1, y], [x + 1, y + 1]),
                line2d([x + 1, y + 1], [x, y + 1]),
                line2d([x, y + 1], [x, y]),
            ]
        }
        EditorAuthoringHitTarget::Resize { axis, side, .. } => {
            let (start, end) = match (axis, side) {
                (EditorGridAxis::X, EditorGridSide::Min) => {
                    ([origin_x, origin_y], [origin_x, max_y])
                }
                (EditorGridAxis::X, EditorGridSide::Max) => ([max_x, origin_y], [max_x, max_y]),
                (EditorGridAxis::Y, EditorGridSide::Min) => {
                    ([origin_x, origin_y], [max_x, origin_y])
                }
                (EditorGridAxis::Y, EditorGridSide::Max) => ([origin_x, max_y], [max_x, max_y]),
                (EditorGridAxis::Z, _) => {
                    return Err("editor grid2d resize hit cannot target the z axis".to_string());
                }
            };
            vec![line2d(start, end)]
        }
        EditorAuthoringHitTarget::Cell {
            position: EditorGridPosition::Grid3d(_),
        }
        | EditorAuthoringHitTarget::Placement {
            position: EditorGridPosition::Grid3d(_),
        } => {
            return Err("editor grid3d hit target cannot highlight a grid2d renderer".to_string());
        }
    };
    decorations.push(RuntimeResolvedDecoration::Lines2d {
        segments,
        style: RuntimeResolvedLineStyle {
            color,
            width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 2.0 },
        },
        layer: RuntimeResolvedLineLayer2d::Overlay,
    });
    Ok(())
}

fn line2d(start: [i32; 2], end: [i32; 2]) -> RuntimeResolvedLineSegment2d {
    RuntimeResolvedLineSegment2d {
        start: start.map(f64::from),
        end: end.map(f64::from),
    }
}

fn stage_grid_edges(size: [u16; 3]) -> Vec<RuntimeResolvedLineSegment3d> {
    let mut segments = BTreeSet::new();
    for x in 0..size[0] {
        for y in 0..size[1] {
            for z in 0..size[2] {
                for segment in box_edges(
                    Vec3::new(f32::from(x) - 0.5, f32::from(y) - 0.5, f32::from(z) - 0.5),
                    Vec3::new(f32::from(x) + 0.5, f32::from(y) + 0.5, f32::from(z) + 0.5),
                ) {
                    let mut endpoints = [
                        segment.start.map(f64::to_bits),
                        segment.end.map(f64::to_bits),
                    ];
                    endpoints.sort();
                    segments.insert(endpoints);
                }
            }
        }
    }
    segments
        .into_iter()
        .map(|[start, end]| RuntimeResolvedLineSegment3d {
            start: start.map(f64::from_bits),
            end: end.map(f64::from_bits),
        })
        .collect()
}

fn layer_grid_edges(size: [u16; 3], z: u16) -> Vec<RuntimeResolvedLineSegment3d> {
    let z = f64::from(z) + 0.51;
    let mut segments = Vec::new();
    for x in 0..=size[0] {
        segments.push(RuntimeResolvedLineSegment3d {
            start: [f64::from(x) - 0.5, -0.5, z],
            end: [f64::from(x) - 0.5, f64::from(size[1]) - 0.5, z],
        });
    }
    for y in 0..=size[1] {
        segments.push(RuntimeResolvedLineSegment3d {
            start: [-0.5, f64::from(y) - 0.5, z],
            end: [f64::from(size[0]) - 0.5, f64::from(y) - 0.5, z],
        });
    }
    segments
}

fn box_edges(min: Vec3, max: Vec3) -> Vec<RuntimeResolvedLineSegment3d> {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ]
    .into_iter()
    .map(|(start, end)| RuntimeResolvedLineSegment3d {
        start: corners[start].as_dvec3().to_array(),
        end: corners[end].as_dvec3().to_array(),
    })
    .collect()
}

fn lines_decoration(
    segments: Vec<RuntimeResolvedLineSegment3d>,
    color: RuntimeLinearRgba,
) -> RuntimeResolvedDecoration {
    RuntimeResolvedDecoration::Lines3d {
        segments,
        style: RuntimeResolvedLineStyle {
            color,
            width: RuntimeResolvedStrokeWidth::PhysicalPixels { pixels: 1.0 },
        },
        depth: RuntimeResolvedLineDepth3d::Tested,
    }
}

fn translucent(mut color: RuntimeLinearRgba, alpha: f64) -> RuntimeLinearRgba {
    color.alpha *= alpha;
    color
}

#[cfg(test)]
mod tests {
    use super::{
        EditorAuthoringFrame, EditorAuthoringFrameGeometry, bounded_position, hit_grid2d, ray_box,
        resize_boxes, validate_css_size, validate_grid3d_selection,
    };
    use bevy::prelude::{Vec2, Vec3};
    use puzzle_editor_preview_contract::{
        EditorAuthoringHitTarget, EditorAuthoringInteraction, EditorGridAxis, EditorGridPosition,
        EditorGridSide, EditorPaintOperation, EditorPointerGesture, EditorResizeMode,
    };
    use std::collections::HashSet;

    #[test]
    fn ray_box_reports_the_entered_face_for_add_placement() {
        let (distance, face) = ray_box(
            Vec3::new(0.0, 0.0, 4.0),
            Vec3::NEG_Z,
            Vec3::splat(-0.5),
            Vec3::splat(0.5),
        )
        .unwrap();
        assert_eq!(distance, 3.5);
        assert_eq!(face, [0, 0, 1]);
    }

    #[test]
    fn shrink_faces_exclude_unit_dimensions() {
        let faces = resize_boxes([1, 2, 1], EditorResizeMode::Shrink);
        assert_eq!(faces.len(), 2);
        assert!(faces.iter().all(|face| {
            face.axis == EditorGridAxis::Y
                && matches!(face.side, EditorGridSide::Min | EditorGridSide::Max)
        }));
    }

    #[test]
    fn bounded_position_rejects_negative_and_upper_edges() {
        assert_eq!(bounded_position([0, 1, 2], [1, 2, 3]), Some([0, 1, 2]));
        assert_eq!(bounded_position([-1, 0, 0], [1, 2, 3]), None);
        assert_eq!(bounded_position([1, 0, 0], [1, 2, 3]), None);
    }

    #[test]
    fn grid2d_hit_and_occupied_cells_share_authored_coordinates() {
        let hit = hit_grid2d(
            Vec2::new(0.75, 0.25),
            [5, 7],
            [2, 2],
            &HashSet::from([[6, 7]]),
            EditorAuthoringInteraction::Paint {
                operation: EditorPaintOperation::Replace,
            },
        )
        .unwrap();
        assert_eq!(
            hit,
            Some(EditorAuthoringHitTarget::Cell {
                position: EditorGridPosition::Grid2d(puzzle_authoring::EditorDraftPosition2d {
                    x: 6,
                    y: 7
                })
            })
        );
    }

    #[test]
    fn committed_frame_rejects_stale_identity_and_outside_coordinates() {
        let frame = EditorAuthoringFrame {
            surface_id: "stage".to_string(),
            revision: 9,
            css_size: Vec2::new(200.0, 100.0),
            interaction: EditorAuthoringInteraction::Observe,
            geometry: EditorAuthoringFrameGeometry::Grid2d {
                origin: [0, 0],
                size: [2, 1],
                occupied: HashSet::new(),
            },
        };
        assert!(
            frame
                .hit("other", 9, Vec2::ZERO)
                .unwrap_err()
                .contains("surface")
        );
        assert!(
            frame
                .hit("stage", 8, Vec2::ZERO)
                .unwrap_err()
                .contains("stale")
        );
        assert!(
            frame
                .hit("stage", 9, Vec2::new(200.0, 10.0))
                .unwrap_err()
                .contains("outside")
        );
    }

    #[test]
    fn release_resolves_against_the_same_committed_frame_as_press_and_move() {
        let frame = EditorAuthoringFrame {
            surface_id: "stage".to_string(),
            revision: 9,
            css_size: Vec2::new(200.0, 100.0),
            interaction: EditorAuthoringInteraction::Paint {
                operation: EditorPaintOperation::Replace,
            },
            geometry: EditorAuthoringFrameGeometry::Grid2d {
                origin: [0, 0],
                size: [2, 1],
                occupied: HashSet::from([[0, 0]]),
            },
        };
        let point = Vec2::new(25.0, 50.0);
        let expected = frame
            .hit_for_gesture("stage", 9, point, EditorPointerGesture::Press)
            .unwrap();
        assert_eq!(
            frame
                .hit_for_gesture("stage", 9, point, EditorPointerGesture::Move)
                .unwrap(),
            expected
        );
        assert_eq!(
            frame
                .hit_for_gesture("stage", 9, point, EditorPointerGesture::Release)
                .unwrap(),
            expected
        );
    }

    #[test]
    fn committed_css_size_must_be_positive_and_finite() {
        assert!(validate_css_size(Vec2::new(1.0, 1.0)).is_ok());
        for invalid in [
            Vec2::ZERO,
            Vec2::new(-1.0, 1.0),
            Vec2::new(f32::INFINITY, 1.0),
            Vec2::new(1.0, f32::NAN),
        ] {
            assert!(validate_css_size(invalid).is_err());
        }
    }

    #[test]
    fn hidden_layers_validate_against_runtime_layer_count() {
        assert!(validate_grid3d_selection(None, &[2], 4, 3).is_ok());
        assert_eq!(
            validate_grid3d_selection(None, &[3], 4, 3).unwrap_err(),
            "editor grid3d hidden layer 3 is outside layer count 3"
        );
    }
}
