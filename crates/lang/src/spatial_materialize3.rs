use std::collections::{BTreeMap, HashMap};

use puzzle_core::Coord3;
use puzzle_core::{
    GridCompiledGame, GridExecutableProgram, GridInput, GridProgramCatalog, GridState, Size3,
};

use crate::{
    Catalog, Controls, DiagnosticReport, LoadedGridLevel, PuzzleRenderDef, SpatialPresentation,
    VisualKind, VisualsDef, VoxelColor, VoxelFrame, VoxelVisual, VoxelVisualSet,
    model_syntax::PuzzleModelSyntax,
};

pub(crate) struct SpatialMaterialization {
    pub(crate) inputs: Vec<GridInput<3>>,
    pub(crate) program_catalog: GridProgramCatalog<3>,
    pub(crate) levels: Vec<LoadedGridLevel<3, Size3>>,
    pub(crate) presentation: SpatialPresentation,
}

pub(crate) fn materialize_spatial_model(
    model: &PuzzleModelSyntax,
    catalog: &Catalog,
    game: &GridCompiledGame<3>,
    programs: &mut crate::LoweredPrograms,
    controls: &Controls,
    input_labels: &HashMap<puzzle_core::InputId, String>,
    render: &PuzzleRenderDef,
    visuals: &VisualsDef,
) -> Result<SpatialMaterialization, DiagnosticReport> {
    let visual_set = materialize_visual_set(visuals)?;
    let (program_catalog, levels) = materialize_levels(model, catalog, programs, game)?;
    let viewport_focus_objects = materialize_viewport_focus(render, catalog);
    let rule_camera_effects = vec![Vec::new(); game.executable_program().rule_count()];
    Ok(SpatialMaterialization {
        inputs: crate::spatial_orientation::materialize_inputs(
            crate::ModelDimension::Three,
            controls,
            input_labels,
        )?,
        program_catalog,
        levels,
        presentation: SpatialPresentation {
            viewport_focus_objects,
            local_frame: None,
            rule_camera_effects,
            on_level_start_camera_effects: Vec::new(),
            visual_set,
            visual_order: visuals.order.clone(),
        },
    })
}

fn materialize_viewport_focus(
    render: &PuzzleRenderDef,
    catalog: &Catalog,
) -> Vec<puzzle_core::ObjectId> {
    let focus = render.viewport.focus.trim();
    if focus.is_empty() {
        return Vec::new();
    }
    if let Some(objects) = catalog.object_groups.get(focus) {
        return objects.clone();
    }
    if let Some(object) = catalog.object_names.get(focus) {
        return vec![*object];
    }
    Vec::new()
}

fn materialize_visual_set(
    visuals: &VisualsDef,
) -> Result<Option<VoxelVisualSet>, DiagnosticReport> {
    if visuals.entries.is_empty() {
        return Ok(None);
    }
    let mut entries = visuals
        .entries
        .iter()
        .map(materialize_visual)
        .collect::<Result<Vec<_>, _>>()?;
    for alias in &visuals.aliases {
        if entries.iter().any(|visual| visual.name == alias.object) {
            continue;
        }
        let source = entries
            .iter()
            .find(|visual| visual.name == alias.visual)
            .cloned()
            .ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "visual alias `{}` references unknown visual `{}`",
                    alias.object, alias.visual
                ))
            })?;
        entries.push(VoxelVisual {
            name: alias.object.clone(),
            ..source
        });
    }
    Ok(Some(VoxelVisualSet::new("canonical", None, entries)))
}

fn materialize_visual(visual: &crate::VisualDef) -> Result<VoxelVisual, DiagnosticReport> {
    let (palette, fallback_rows) = match &visual.kind {
        VisualKind::Solid(color) => (
            BTreeMap::from([('0', visual_color(color))]),
            vec!["0".to_string()],
        ),
        VisualKind::Ascii { colors } => (
            colors
                .iter()
                .map(|color| (color.token, visual_color(&color.color)))
                .collect(),
            Vec::new(),
        ),
        VisualKind::Image { .. } => {
            return Err(DiagnosticReport::error(format!(
                "3D voxel renderer cannot materialize image visual `{}`",
                visual.name
            )));
        }
    };
    let spatial_frames = if visual.frames.is_empty() {
        vec![vec![fallback_rows]]
    } else {
        visual
            .frames
            .iter()
            .map(|frame| frame.planes.clone())
            .collect()
    };
    let frames = spatial_frames
        .into_iter()
        .map(|slices| materialize_voxels(&visual.name, slices))
        .collect::<Result<_, _>>()?;
    let mut materialized = VoxelVisual::new(
        visual.name.clone(),
        palette,
        frames,
        visual.animation_duration_ms,
        None,
    );
    materialized.transforms = visual.transforms.clone();
    Ok(materialized)
}

fn visual_color(color: &str) -> VoxelColor {
    if color.eq_ignore_ascii_case("transparent") {
        VoxelColor::Transparent
    } else {
        VoxelColor::Hex(color.to_string())
    }
}

fn materialize_voxels(
    visual_name: &str,
    slices: Vec<Vec<String>>,
) -> Result<VoxelFrame, DiagnosticReport> {
    let height = slices.len();
    let depth = slices.first().map_or(0, Vec::len);
    let width = slices
        .first()
        .and_then(|slice| slice.first())
        .map_or(0, |row| row.chars().count());
    if height == 0
        || depth == 0
        || width == 0
        || slices.iter().any(|slice| {
            slice.len() != depth || slice.iter().any(|row| row.chars().count() != width)
        })
    {
        return Err(DiagnosticReport::error(format!(
            "visual `{visual_name}` must be rectangular in every spatial frame"
        )));
    }
    let size = Size3::new(
        u16::try_from(width)
            .map_err(|_| DiagnosticReport::error("visual width exceeds u16".to_string()))?,
        u16::try_from(depth)
            .map_err(|_| DiagnosticReport::error("visual depth exceeds u16".to_string()))?,
        u16::try_from(height)
            .map_err(|_| DiagnosticReport::error("visual height exceeds u16".to_string()))?,
    );
    Ok(VoxelFrame::new(size, slices))
}

fn materialize_levels(
    model: &PuzzleModelSyntax,
    catalog: &Catalog,
    programs: &mut crate::LoweredPrograms,
    game: &GridCompiledGame<3>,
) -> Result<(GridProgramCatalog<3>, Vec<LoadedGridLevel<3, Size3>>), DiagnosticReport> {
    let mut legends = HashMap::<char, Vec<_>>::new();
    for legend in &model.body.levels.legends {
        let mut objects = Vec::new();
        for selector in &legend.selectors {
            if selector == "empty" {
                continue;
            }
            let object = catalog.object_names.get(selector).copied().ok_or_else(|| {
                DiagnosticReport::error_at_source_line_number(
                    format!("unknown level object `{selector}`"),
                    legend.source.text.clone(),
                    legend.source.line,
                )
            })?;
            objects.push(object);
        }
        legends.insert(legend.ch, objects);
    }
    let mut program_catalog = GridProgramCatalog::default();
    let levels = model
        .body
        .levels
        .levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            let mut local = legends.clone();
            for legend in &level.legends {
                let mut objects = Vec::new();
                for selector in &legend.selectors {
                    if selector == "empty" {
                        continue;
                    }
                    let object = catalog.object_names.get(selector).copied().ok_or_else(|| {
                        DiagnosticReport::error_at_source_line_number(
                            format!("unknown level object `{selector}`"),
                            legend.source.text.clone(),
                            legend.source.line,
                        )
                    })?;
                    objects.push(object);
                }
                local.insert(legend.ch, objects);
            }
            let slices = crate::level::split_spatial_level_slices(&level.lines)?;
            let height = u16::try_from(slices.len())
                .map_err(|_| DiagnosticReport::error("3D level height exceeds u16".to_string()))?;
            let depth = u16::try_from(slices.iter().map(|slice| slice.len()).max().unwrap_or(0))
                .map_err(|_| DiagnosticReport::error("3D level depth exceeds u16".to_string()))?;
            let width = u16::try_from(
                slices
                    .iter()
                    .flat_map(|slice| slice.iter())
                    .map(|row| row.text.chars().count())
                    .max()
                    .unwrap_or(0),
            )
            .map_err(|_| DiagnosticReport::error("3D level width exceeds u16".to_string()))?;
            if slices.iter().any(|slice| {
                slice.len() != usize::from(depth)
                    || slice
                        .iter()
                        .any(|row| row.text.chars().count() != usize::from(width))
            }) {
                return Err(DiagnosticReport::error(format!(
                    "3D level `{}` must be rectangular in every slice",
                    level.name
                )));
            }
            let size = Size3::new(width, depth, height);
            let mut cells = Vec::<(puzzle_core::GridCoord<3>, Vec<puzzle_core::ObjectId>)>::new();
            for (slice_index, slice) in slices.iter().enumerate() {
                for (row_index, row) in slice.iter().enumerate() {
                    for (column, ch) in row.text.chars().enumerate() {
                        let objects = crate::level::level_cell_objects(
                            ch,
                            Some(crate::syntax::DEFAULT_LEVEL_EMPTY_CHAR),
                            &local,
                        )
                        .map_err(|_| {
                            DiagnosticReport::error_at_source_line_number(
                                format!("unknown level char `{ch}`"),
                                row.text.clone(),
                                row.line,
                            )
                        })?;
                        let Some(objects) = objects else {
                            continue;
                        };
                        if objects.is_empty() {
                            continue;
                        }
                        cells.push((
                            Coord3::from_standard_text_position(
                                size,
                                column as u16,
                                row_index as u16,
                                slice_index as u16,
                            )
                            .into(),
                            objects.to_vec(),
                        ));
                    }
                }
            }
            let program = programs.level_programs.get_mut(index).ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "canonical 3D level `{}` has no matching lowered program",
                    level.name
                ))
            })?;
            let mut initial_state = GridState::empty_sized_with_variables(
                size,
                game.layer_count,
                game.object_count(),
                catalog.variable_defaults.clone(),
            )
            .map_err(|error| {
                DiagnosticReport::error(format!(
                    "invalid 3D level `{}` dimensions: {error:?}",
                    level.name
                ))
            })?;
            for (position, objects) in cells {
                for object in objects {
                    initial_state
                        .place_object_at(game, position, object)
                        .map_err(|error| {
                            DiagnosticReport::error(format!(
                                "invalid 3D level `{}`: {error:?}",
                                level.name
                            ))
                        })?;
                }
            }
            let program = match std::mem::replace(program, crate::LoweredLevelProgram::Main) {
                crate::LoweredLevelProgram::Main => puzzle_core::GridProgramSequence::main(),
                crate::LoweredLevelProgram::WithSurrounding { before, after } => {
                    let before = (!before.is_empty())
                        .then(|| program_catalog.intern(GridExecutableProgram::new(before)));
                    let after = (!after.is_empty())
                        .then(|| program_catalog.intern(GridExecutableProgram::new(after)));
                    puzzle_core::GridProgramSequence::with_surrounding(before, after)
                }
            };
            let level_start_program = programs
                .level_starts
                .get_mut(index)
                .ok_or_else(|| {
                    DiagnosticReport::error(format!(
                        "canonical 3D level `{}` has no matching level-start program slot",
                        level.name
                    ))
                })?
                .take()
                .map(GridExecutableProgram::new)
                .map(|program| program_catalog.intern(program));
            let level_clear_program = programs
                .level_clears
                .get_mut(index)
                .ok_or_else(|| {
                    DiagnosticReport::error(format!(
                        "canonical 3D level `{}` has no matching level-clear program slot",
                        level.name
                    ))
                })?
                .take()
                .map(GridExecutableProgram::new)
                .map(|program| program_catalog.intern(program));
            Ok(LoadedGridLevel {
                name: level.name.clone(),
                pack: level.pack.clone(),
                puzzle: model.name.clone(),
                initial_state,
                regions: Vec::new(),
                program,
                level_start_program,
                level_clear_program,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    Ok((program_catalog, levels))
}
