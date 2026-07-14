use std::collections::{BTreeMap, HashMap, HashSet};

use puzzle_grid3d::{
    CompiledGame3, ConditionValueKind3, Coord3, Delta3, Direction3, DirectionSet3, FrameExpr3,
    FrameSet3, FrameSlot3, Guard3, InputDef3, InputId, Level3, LevelBundle3, LevelCell3,
    LevelEntry3, MarkDef3, MatchCell3, ObjectDef3, ObjectId, Pattern3, Rule3, RuleApplication3,
    RuleCondition3, RuleId3, RuleStep3, Size3, WinCondition3,
};
use puzzle_grid3d_authoring::{
    DenseCell3, DensePattern3, DenseRow3, DenseRuleTemplate3, DenseSlice3, FrameOrientation3,
    LineMatchCellTemplate3, LineOffsetTemplate3, LineOrientation3, LinePatternTemplate3,
    LineRuleTemplate3, LineWriteOpTemplate3, LocalWriteOpTemplate3, MatchCellTemplate3,
    ObjectSelector3, PatternTemplate3, ResolvedObjectSelector3, ResolvedSelectorMark3,
    RuleTemplate3, SelectorMark3, SelectorTag3, WriteOpTemplate3, lower_rule_template,
    project_dense_rule_template, project_line_rule_template,
};
use puzzle_kernel::{LocalFrame, LocalFrameExtent};
use puzzle_runtime_contract::{LifecycleCommand, Puzzle3CameraEffect, RuntimeLifecycle};

use crate::{
    DiagnosticReport, ModelSettings3, ParsedPuzzle3, SolverStrategy3, SolverSurfacePatternArg,
    SolverSurfaceQueryArg, Sprite3, SpriteColor3, SpriteSet3, SpriteVoxels3, ViewportFollow3,
    ViewportFraming3, ViewportHeight3, ViewportMode3, ViewportSettings3,
};

const DEFAULT_LINE_GAP_LIMIT3: u16 = 64;

pub type ParseError3 = DiagnosticReport;

pub fn parse_puzzle3d(source: &str) -> Result<ParsedPuzzle3, DiagnosticReport> {
    let document = crate::parse_surface_compile_document(source)?;
    let parts = crate::parse_document_source_parts_from_surface_document(&document)?;
    let [model] = parts.models.as_slice() else {
        return Err(message(
            "3D lowering requires exactly one `puzzle <name> { ... }` declaration",
        ));
    };
    let [catalog] = parts.model_catalogs.as_slice() else {
        return Err(message("3D lowering requires exactly one Catalog product"));
    };
    lower_puzzle3d_product(model, catalog.clone())
}

pub(crate) fn lower_puzzle3d_product(
    model: &crate::model_syntax::PuzzleModelSyntax,
    catalog: crate::Catalog,
) -> Result<ParsedPuzzle3, ParseError3> {
    if model.dimension != crate::ModelDimension::Three || !model.dimension_is_explicit {
        return Err(message(format!(
            "puzzle `{}` must explicitly declare `dimension = 3`",
            model.name
        )));
    }
    let layer_count = catalog.layer_count.unwrap_or(0);
    let mut lowering = Puzzle3Lowering::new(model.entries.clone(), catalog, layer_count);
    for entry in &model.level_resources {
        lowering.parse_levels_entry(entry)?;
    }
    for entry in &model.sprite_resources {
        lowering.parse_sprites_entry(entry)?;
    }
    lowering.lower()
}

struct Puzzle3Lowering {
    model_entries: Vec<crate::model_syntax::PuzzleEntrySyntax>,
    catalog: crate::Catalog,
    layer_count: u16,
    input_specs: Vec<InputSpec3>,
    legend_specs: Vec<LegendSpec3>,
    level_specs: Vec<LevelSpec3>,
    rule_lines: Vec<puzzle_authoring::RuleStatementSyntax>,
    local_frame_modifier: Option<String>,
    on_level_start_lines: Vec<puzzle_authoring::RuleStatementSyntax>,
    on_level_start_local_frame_modifier: Option<String>,
    on_level_clear_lines: Vec<String>,
    on_last_level_clear_lines: Option<Vec<String>>,
    win_condition_lines: Vec<String>,
    query_definitions: Vec<crate::solver_surface::SolverSurfaceQueryDefinition>,
    query_names: HashSet<String>,
    solver_strategy: Option<crate::solver_surface::SolverSurfaceStrategy>,
    settings: ModelSettings3,
    sprite_resource: Option<SpriteResourceSyntax3>,
}

struct SpriteResourceSyntax3 {
    name: String,
    model: Option<String>,
    shapes: HashMap<String, Vec<crate::sprite_authoring::SpriteFrameSyntax>>,
    attachments: Vec<crate::sprite_authoring::SpriteAttachmentSyntax>,
    order: Option<puzzle_authoring::SpriteOrderSurface>,
}

impl Puzzle3Lowering {
    fn new(
        model_entries: Vec<crate::model_syntax::PuzzleEntrySyntax>,
        catalog: crate::Catalog,
        layer_count: u16,
    ) -> Self {
        Self {
            model_entries,
            catalog,
            layer_count,
            input_specs: Vec::new(),
            legend_specs: Vec::new(),
            level_specs: Vec::new(),
            rule_lines: Vec::new(),
            local_frame_modifier: None,
            on_level_start_lines: Vec::new(),
            on_level_start_local_frame_modifier: None,
            on_level_clear_lines: Vec::new(),
            on_last_level_clear_lines: None,
            win_condition_lines: Vec::new(),
            query_definitions: Vec::new(),
            query_names: HashSet::new(),
            solver_strategy: None,
            settings: ModelSettings3::default(),
            sprite_resource: None,
        }
    }

    fn lower(mut self) -> Result<ParsedPuzzle3, ParseError3> {
        self.lower_model_entries()?;

        if self.layer_count == 0 {
            return Err(message("missing slots block"));
        }
        let object_defs = self
            .catalog
            .object_defs
            .iter()
            .map(|object| ObjectDef3 {
                id: object.id,
                layer_id: object.layer_id,
            })
            .collect();
        let sprite_set = self
            .sprite_resource
            .as_ref()
            .map(|resource| lower_sprite_resource3(resource, &self.catalog))
            .transpose()?;
        let visual_order = crate::lib_authoring_parse_order::lower_sprite_order(
            self.sprite_resource
                .as_ref()
                .and_then(|resource| resource.order.as_ref()),
            &self.catalog,
            "sprites order",
        )?;
        let mark_defs = self
            .catalog
            .mark_defs
            .iter()
            .map(|mark| MarkDef3 {
                id: puzzle_grid3d::MarkId3(mark.id.0),
                kind: mark.kind,
                values: mark.values.clone(),
            })
            .collect();
        let inputs = inputs_from_specs(self.input_specs)?;
        let game = CompiledGame3::new_with_mark_condition_defs_and_program(
            self.layer_count,
            object_defs,
            mark_defs,
            Vec::new(),
            Vec::new(),
        );
        let local_frame = parse_optional_program_local_frame(
            self.local_frame_modifier.as_deref(),
            &self.catalog,
        )?;
        let on_level_start_local_frame = parse_optional_program_local_frame(
            self.on_level_start_local_frame_modifier.as_deref(),
            &self.catalog,
        )?;

        let line_gap_limit = line_gap_limit_from_levels(&self.level_specs);
        let mut rules = Vec::new();
        let mut rule_camera_effects = Vec::new();
        let mut next_main_rule_id = 0u16;
        for parsed in lower_rule_statement_syntax3(
            &self.rule_lines,
            &self.catalog,
            line_gap_limit,
            None,
            &[],
        )? {
            rules.push(program_step_for_rule_statement(
                parsed.rules,
                &mut next_main_rule_id,
            )?);
            rule_camera_effects.extend(parsed.camera_effects);
        }
        let game = game.clone_with_program(rules);
        let mut on_level_start = Vec::new();
        let mut on_level_start_camera_effects = Vec::new();
        let mut next_level_start_rule_id = 0u16;
        for parsed in lower_rule_statement_syntax3(
            &self.on_level_start_lines,
            &self.catalog,
            line_gap_limit,
            None,
            &[],
        )? {
            on_level_start.push(program_step_for_rule_statement(
                parsed.rules,
                &mut next_level_start_rule_id,
            )?);
            on_level_start_camera_effects.extend(parsed.camera_effects);
        }
        let on_level_clear = self
            .on_level_clear_lines
            .iter()
            .map(|line| parse_lifecycle_command_line(line))
            .collect::<Result<Vec<_>, ParseError3>>()?;
        let on_last_level_clear = self
            .on_last_level_clear_lines
            .as_ref()
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| parse_lifecycle_command_line(line))
                    .collect::<Result<Vec<_>, ParseError3>>()
            })
            .transpose()?;
        let mut lifecycle = RuntimeLifecycle::new(on_level_start, on_level_clear);
        lifecycle.on_level_start_local_frame = on_level_start_local_frame;
        lifecycle.on_last_level_clear = on_last_level_clear;
        let win_condition = if self.win_condition_lines.is_empty() {
            None
        } else {
            Some(lower_win_conditions(
                &self.catalog,
                &self.win_condition_lines,
                line_gap_limit,
            )?)
        };
        let level_bundle = if self.level_specs.is_empty() {
            None
        } else {
            Some(lower_level_bundle(
                game.clone(),
                &self.catalog,
                &self.legend_specs,
                &self.level_specs,
            )?)
        };
        validate_query_definitions3(&self.query_definitions, &self.catalog, line_gap_limit)?;
        let solver_strategy = lower_solver_strategy3(
            self.solver_strategy.clone(),
            &self.query_definitions,
            &self.catalog,
            line_gap_limit,
        )?;
        let object_labels = self.catalog.object_labels.clone();
        let viewport_focus_objects = if self.settings.viewport.mode == ViewportMode3::Full {
            Vec::new()
        } else {
            resolve_named_selector_objects3(&self.settings.viewport.focus, &self.catalog)?
        };

        Ok(ParsedPuzzle3 {
            game,
            inputs,
            object_labels,
            viewport_focus_objects,
            settings: self.settings,
            local_frame,
            rule_camera_effects,
            level_bundle,
            level_packs: self
                .level_specs
                .iter()
                .map(|spec| spec.pack.clone())
                .collect(),
            win_condition,
            solver_strategy,
            lifecycle,
            on_level_start_camera_effects,
            sprite_set,
            visual_order,
        })
    }

    fn lower_model_entries(&mut self) -> Result<(), ParseError3> {
        for entry in self.model_entries.clone() {
            let line = entry.header.text.clone();
            let directive = entry.directive;
            if directive == puzzle_authoring::PuzzleDirectiveSurface::Empty {
                continue;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Objects {
                return Err(message(
                    "`objects { ... }` is not shared authoring syntax; declare objects in `slots { ... }`",
                ));
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Keys
                && line == "keys {"
            {
                self.parse_keys_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Inputs {
                return Err(message(
                    "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                ));
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::SingularGroup {
                return Err(message(
                    "singular group syntax was removed; use `groups { name = selector... }`",
                ));
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Legend
                && line == "legend {"
            {
                self.parse_legend_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Levels {
                self.parse_levels_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::RuleProgram
                && let Some(block) = puzzle_authoring::rule_program_block_surface(&line)
            {
                self.parse_rule_program_entry(block, &entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::WinConditions {
                self.parse_win_conditions_entry(&entry);
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Query {
                self.parse_query_line(&line)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Solver {
                self.parse_solver_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Render {
                self.parse_render_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Sprites {
                self.parse_sprites_entry(&entry)?;
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Scene
                && parse_scene_header(&line).is_some()
            {
                continue;
            } else if legacy_model_setting_name(&line).is_some() {
                return Err(message(format!(
                    "legacy model setting is not supported: {line}"
                )));
            } else if directive == puzzle_authoring::PuzzleDirectiveSurface::Assignment {
                if puzzle_authoring::parse_assignment_row(&line)
                    .is_some_and(|(name, _)| name == "dimension")
                {
                    continue;
                }
                return Err(message(
                    "bare tag-set assignments are not shared authoring syntax; use `tags { name = value... }`",
                ));
            } else {
                return Err(message(format!("unknown model directive: {line}")));
            }
        }
        Ok(())
    }

    fn apply_model_setting(&mut self, setting: ModelSetting3) {
        match setting {
            ModelSetting3::CameraYaw(value) => self.settings.camera.yaw_degrees = value,
            ModelSetting3::CameraPitch(value) => self.settings.camera.pitch_degrees = value,
            ModelSetting3::CameraRoll(value) => self.settings.camera.roll_degrees = value,
            ModelSetting3::CameraZoom(value) => self.settings.camera.zoom_milli = value,
            ModelSetting3::InteractiveLook(value) => self.settings.camera.interactive_look = value,
            ModelSetting3::InteractiveZoom(value) => self.settings.camera.interactive_zoom = value,
            ModelSetting3::OccupiedCellGrid => self.settings.grid.occupied_cells = true,
            ModelSetting3::SpriteShade(value) => self.settings.sprite.shade = value,
            ModelSetting3::Shadow(value) => self.settings.shadow = value,
            ModelSetting3::PixelateEnabled(value) => self.settings.pixelate.enabled = value,
            ModelSetting3::PixelateScale(value) => self.settings.pixelate.scale = value,
            ModelSetting3::PixelateSmoothing(value) => self.settings.pixelate.smoothing = value,
        }
    }

    fn parse_render_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for child in crate::model_syntax::parse_child_entries(entry)? {
            match child.header.text.as_str() {
                "camera {" => self.parse_camera_entry(&child)?,
                "grid {" => self.parse_grid_entry(&child)?,
                "pixelate {" => self.parse_pixelate_entry(&child)?,
                "viewport {" => self.parse_viewport_entry(&child)?,
                line if child.body.is_empty() => {
                    let setting = parse_render_setting_line(line)?;
                    self.apply_model_setting(setting);
                }
                line => return Err(message(format!("unknown render directive: {line}"))),
            }
        }
        Ok(())
    }

    fn parse_camera_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            let setting = parse_camera_setting_line(line)?;
            self.apply_model_setting(setting);
        }
        Ok(())
    }

    fn parse_viewport_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            parse_viewport_directive(line, &mut self.settings.viewport)?;
        }
        Ok(())
    }

    fn parse_grid_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            let setting = parse_grid_setting_line(line)?;
            self.apply_model_setting(setting);
        }
        Ok(())
    }

    fn parse_pixelate_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            let setting = parse_pixelate_setting_line(line)?;
            self.apply_model_setting(setting);
        }
        Ok(())
    }

    fn parse_sprites_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        if self.sprite_resource.is_some() {
            return Err(message("duplicate sprites block"));
        }
        let header = puzzle_authoring::resource_header_surface(&entry.header.text, "sprites")
            .map_err(|error| message(error.message()))?;
        let name = header.name.unwrap_or("default").to_string();
        let model = header.owner.map(str::to_string);
        let mut shapes = HashMap::<String, Vec<crate::sprite_authoring::SpriteFrameSyntax>>::new();
        let mut attachments = Vec::new();
        let mut order = None;
        let lines = entry
            .body
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<_>>();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index].clone();
            if line.is_empty() {
                index += 1;
                continue;
            }
            if line == "shapes {" {
                let (shapes_entry, next) = crate::model_syntax::parse_child_entry_at(entry, index)?;
                let parsed_shapes = parse_sprite_shapes_entry(&shapes_entry)?;
                for (shape_name, shape) in parsed_shapes {
                    if shapes.insert(shape_name.clone(), shape).is_some() {
                        return Err(message(format!("duplicate sprite shape: {shape_name}")));
                    }
                }
                index = next;
                continue;
            }
            if line == "order {" {
                if order.is_some() {
                    return Err(message("duplicate sprite order block"));
                }
                let (parsed, next) = puzzle_authoring::parse_sprite_order_surface(&lines, index)
                    .map_err(|error| message(error.message()))?;
                order = Some(parsed);
                index = next;
                continue;
            }
            let attachment = crate::sprite_authoring::collect_sprite_attachment(&lines, index)
                .map_err(message)?;
            index = attachment.next_index;
            attachments.push(attachment);
        }
        self.sprite_resource = Some(SpriteResourceSyntax3 {
            name,
            model,
            shapes,
            attachments,
            order,
        });
        Ok(())
    }

    fn parse_keys_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            if line.ends_with('{') {
                return Err(message(format!(
                    "keys accepts rows, not nested blocks: {line}"
                )));
            }
            self.input_specs.push(
                input_spec_from_key_surface(line)
                    .map_err(|report| report.with_fallback_source_line(line))?,
            );
        }
        Ok(())
    }

    fn parse_legend_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        for line in entry.body.iter().map(|line| line.text.as_str()) {
            if line.ends_with('{') {
                return Err(message(format!(
                    "legend accepts rows, not nested blocks: {line}"
                )));
            }
            self.legend_specs.push(
                parse_legend_spec(line).map_err(|report| report.with_fallback_source_line(line))?,
            );
        }
        Ok(())
    }

    fn parse_levels_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        let header = puzzle_authoring::resource_header_surface(&entry.header.text, "levels")
            .map_err(|error| message(error.message()))?;
        let pack = header.name.map(str::to_string);
        let mut namespace_count = 0usize;
        let mut index = 0;
        while index < entry.body.len() {
            let line = entry.body[index].text.clone();
            if line.is_empty() {
                index += 1;
                continue;
            }
            if line == "legend {" {
                let (child, next) = crate::model_syntax::parse_child_entry_at(entry, index)?;
                self.parse_legend_entry(&child)?;
                index = next;
                continue;
            }
            if puzzle_authoring::is_braced_level_header(&line) {
                namespace_count += 1;
                let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                    pack.as_deref(),
                    self.level_specs.len(),
                    namespace_count,
                );
                let name = parse_level_header(&line, auto_name)?;
                let (child, next) = crate::model_syntax::parse_child_entry_at(entry, index)?;
                self.level_specs.push(LevelSpec3 {
                    name,
                    pack: pack.clone(),
                    rows: child.body.into_iter().map(|line| line.text).collect(),
                });
                index = next;
                continue;
            }
            if line == "{" {
                namespace_count += 1;
                let name = puzzle_authoring::namespaced_unnamed_level_name(
                    pack.as_deref(),
                    self.level_specs.len(),
                    namespace_count,
                );
                let (child, next) = crate::model_syntax::parse_child_entry_at(entry, index)?;
                self.level_specs.push(LevelSpec3 {
                    name,
                    pack: pack.clone(),
                    rows: child.body.into_iter().map(|line| line.text).collect(),
                });
                index = next;
                continue;
            }
            if line.trim_start().starts_with("level") {
                let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                    pack.as_deref(),
                    self.level_specs.len(),
                    namespace_count + 1,
                );
                parse_level_header(&line, auto_name)?;
                return Err(message("3D level header must open a block with {"));
            }
            return Err(message(format!("unknown levels directive: {line}")));
        }
        Ok(())
    }

    fn parse_rule_program_entry(
        &mut self,
        block: puzzle_authoring::RuleProgramBlockSurface<'_>,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        if matches!(
            block,
            puzzle_authoring::RuleProgramBlockSurface::OnLastLevelClear
        ) && self.on_last_level_clear_lines.is_some()
        {
            return Err(message(
                "multiple last_level_clear blocks are not supported",
            ));
        }

        let lines = entry
            .body
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<_>>();
        let body = puzzle_authoring::collect_rule_program_entry_body(&lines, block)
            .map_err(|error| message(error.message()))?;

        match block {
            puzzle_authoring::RuleProgramBlockSurface::Rules { modifier } => {
                self.local_frame_modifier =
                    (!modifier.trim().is_empty()).then(|| modifier.trim().to_string());
                let puzzle_authoring::RuleProgramBlockBody::RuleStatements(lines) = body else {
                    unreachable!("rules blocks collect rule statement bodies");
                };
                self.rule_lines.extend(lines);
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLevelStart { modifier } => {
                self.on_level_start_local_frame_modifier =
                    (!modifier.trim().is_empty()).then(|| modifier.trim().to_string());
                let puzzle_authoring::RuleProgramBlockBody::RuleStatements(lines) = body else {
                    unreachable!("on_level_start blocks collect rule statement bodies");
                };
                self.on_level_start_lines.extend(lines);
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLevelClear => {
                let puzzle_authoring::RuleProgramBlockBody::LifecycleCommands(lines) = body else {
                    unreachable!("on_level_clear blocks collect lifecycle command bodies");
                };
                self.on_level_clear_lines.extend(lines);
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLastLevelClear => {
                let puzzle_authoring::RuleProgramBlockBody::LifecycleCommands(lines) = body else {
                    unreachable!("on_last_level_clear blocks collect lifecycle command bodies");
                };
                self.on_last_level_clear_lines = Some(lines);
            }
        }

        Ok(())
    }

    fn parse_win_conditions_entry(&mut self, entry: &crate::model_syntax::PuzzleEntrySyntax) {
        for line in &entry.body {
            self.win_condition_lines.push(line.text.clone());
        }
    }

    fn parse_query_line(&mut self, line: &str) -> Result<(), ParseError3> {
        let definition = crate::solver_surface::parse_query_definition(line)
            .map_err(diagnostic_report_error3)?;
        if !self.query_names.insert(definition.name.clone()) {
            return Err(message_at_line("duplicate query", line));
        }
        self.query_definitions.push(definition);
        Ok(())
    }

    fn parse_solver_entry(
        &mut self,
        entry: &crate::model_syntax::PuzzleEntrySyntax,
    ) -> Result<(), ParseError3> {
        if self.solver_strategy.is_some() {
            return Err(message_at_line(
                "duplicate solver block",
                &entry.header.text,
            ));
        }
        let lines = entry
            .body
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<_>>();
        let strategy = crate::solver_surface::parse_solver_entry_body(&lines)
            .map_err(diagnostic_report_error3)?;
        self.solver_strategy = Some(strategy);
        Ok(())
    }
}

fn resolve_named_selector_objects3(
    focus: &str,
    catalog: &crate::Catalog,
) -> Result<Vec<ObjectId>, ParseError3> {
    let mut objects = resolve_shared_selector_objects3(catalog, focus, "viewport focus")?;
    objects.sort_by_key(|object| object.0);
    objects.dedup();
    Ok(objects)
}

fn resolve_shared_selector_objects3(
    catalog: &crate::Catalog,
    token: &str,
    context: &str,
) -> Result<Vec<ObjectId>, ParseError3> {
    crate::resolve_object_selector(
        token,
        token,
        &catalog.object_names,
        &catalog.object_schemas,
        &crate::catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &catalog.variable_names,
    )
    .map(|selector| selector.alternatives)
    .map_err(|error| {
        message(format!(
            "invalid {context} selector: {}",
            diagnostic_message(&error)
        ))
    })
}

fn resolve_pattern_selector3(
    catalog: &crate::Catalog,
    selector: &ObjectSelector3,
) -> Result<ResolvedObjectSelector3, ParseError3> {
    let token = selector.token();
    let alternatives = resolve_shared_selector_objects3(catalog, &token, "pattern")?;
    let base_token = token
        .split_once('#')
        .map_or(token.as_str(), |(base, _)| base);
    let runtime_object_set_layer = if !selector.has_occurrence_label()
        && catalog.object_groups.contains_key(base_token)
        && alternatives.len() > 1
    {
        let mut layers = alternatives
            .iter()
            .filter_map(|object| catalog.object_layers.get(object).copied());
        let layer = layers.next();
        layer.filter(|layer| layers.all(|candidate| candidate == *layer))
    } else {
        None
    };
    Ok(ResolvedObjectSelector3 {
        token,
        alternatives,
        mark: selector
            .mark()
            .iter()
            .map(|mark| resolve_selector_mark3(catalog, mark))
            .collect::<Result<Vec<_>, _>>()?,
        occurrence_labeled: selector.has_occurrence_label(),
        runtime_object_set_layer,
    })
}

fn resolve_selector_mark3(
    catalog: &crate::Catalog,
    mark: &SelectorMark3,
) -> Result<ResolvedSelectorMark3, ParseError3> {
    let (id, kind, values) = if mark.name.is_empty() {
        let value = mark.value.as_deref().unwrap_or_default();
        let index = match puzzle_authoring::mark_sugar_kind(value) {
            Some(puzzle_authoring::MarkSugarKind::Movement) => 0,
            None if value == "directions" => 0,
            Some(puzzle_authoring::MarkSugarKind::Bool) => 1,
            Some(puzzle_authoring::MarkSugarKind::Int) => 2,
            _ => return Err(message(format!("unknown anonymous mark value: {value}"))),
        };
        let def = catalog
            .mark_defs
            .get(index)
            .ok_or_else(|| message("canonical anonymous mark definition is missing"))?;
        (def.id, def.kind, def.values.as_slice())
    } else {
        let def = catalog
            .mark_names
            .get(&mark.name)
            .ok_or_else(|| message(format!("unknown mark: {}", mark.name)))?;
        (def.id, def.kind, def.values.as_slice())
    };
    let value = match kind {
        puzzle_core::MarkKind::Flag => {
            if mark.value.is_some() {
                return Err(message("flag mark cannot have a value"));
            }
            None
        }
        puzzle_core::MarkKind::Bool if mark.name.is_empty() => {
            Some(i64::from(mark.value.as_deref() == Some("true")))
        }
        puzzle_core::MarkKind::Bool => {
            if mark.value.is_some() {
                return Err(message(
                    "bool mark uses presence syntax; write `flag` or `no flag`",
                ));
            }
            Some(1)
        }
        puzzle_core::MarkKind::Int => mark
            .value
            .as_deref()
            .map(|value| {
                value
                    .parse::<i64>()
                    .map_err(|_| message("expected integer mark value"))
            })
            .transpose()?,
        puzzle_core::MarkKind::Enum => mark
            .value
            .as_deref()
            .map(|value| {
                values
                    .iter()
                    .position(|candidate| candidate == value)
                    .and_then(|index| i64::try_from(index).ok())
                    .ok_or_else(|| message(format!("unknown enum mark value: {value}")))
            })
            .transpose()?,
    };
    Ok(ResolvedSelectorMark3 {
        id: puzzle_grid3d::MarkId3(id.0),
        value,
        match_value: if value.is_some() {
            puzzle_grid3d::MarkValueMatch::Exact
        } else {
            puzzle_grid3d::MarkValueMatch::Any
        },
        negated: mark.negated,
    })
}

fn resolve_pattern_template3(
    catalog: &crate::Catalog,
    pattern: PatternTemplate3,
) -> Result<PatternTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>, ParseError3> {
    let gap_count = pattern.gap_count;
    Ok(PatternTemplate3::new(
        pattern
            .cells
            .into_iter()
            .map(|cell| {
                Ok(MatchCellTemplate3 {
                    offset: cell.offset,
                    require_null: cell.require_null,
                    require: cell
                        .require
                        .iter()
                        .map(|selector| resolve_pattern_selector3(catalog, selector))
                        .collect::<Result<Vec<_>, _>>()?,
                    forbid: cell
                        .forbid
                        .iter()
                        .map(|selector| resolve_pattern_selector3(catalog, selector))
                        .collect::<Result<Vec<_>, _>>()?,
                    require_cell_mark: cell
                        .require_cell_mark
                        .iter()
                        .map(|mark| resolve_selector_mark3(catalog, mark))
                        .collect::<Result<Vec<_>, _>>()?,
                    forbid_cell_mark: cell
                        .forbid_cell_mark
                        .iter()
                        .map(|mark| resolve_selector_mark3(catalog, mark))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, ParseError3>>()?,
    )
    .with_gap_count(gap_count))
}

fn resolve_write_template3(
    catalog: &crate::Catalog,
    write: WriteOpTemplate3,
) -> Result<WriteOpTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>, ParseError3> {
    Ok(match write {
        WriteOpTemplate3::Add { offset, object } => WriteOpTemplate3::Add {
            offset,
            object: resolve_pattern_selector3(catalog, &object)?,
        },
        WriteOpTemplate3::Remove { offset, object } => WriteOpTemplate3::Remove {
            offset,
            object: resolve_pattern_selector3(catalog, &object)?,
        },
        WriteOpTemplate3::Replace {
            offset,
            remove,
            add,
        } => WriteOpTemplate3::Replace {
            offset,
            remove: resolve_pattern_selector3(catalog, &remove)?,
            add: resolve_pattern_selector3(catalog, &add)?,
        },
        WriteOpTemplate3::Move {
            from_offset,
            to_offset,
            object,
        } => WriteOpTemplate3::Move {
            from_offset,
            to_offset,
            object: resolve_pattern_selector3(catalog, &object)?,
        },
        WriteOpTemplate3::SetMark {
            offset,
            object,
            mark,
        } => WriteOpTemplate3::SetMark {
            offset,
            object: resolve_pattern_selector3(catalog, &object)?,
            mark: resolve_selector_mark3(catalog, &mark)?,
        },
        WriteOpTemplate3::RemoveMark {
            offset,
            object,
            mark,
        } => WriteOpTemplate3::RemoveMark {
            offset,
            object: resolve_pattern_selector3(catalog, &object)?,
            mark: resolve_selector_mark3(catalog, &mark)?,
        },
    })
}

fn lower_projected_rule_template3(
    catalog: &crate::Catalog,
    rule: RuleTemplate3,
) -> Result<Vec<Rule3>, ParseError3> {
    let rule = RuleTemplate3 {
        id: rule.id,
        guards: rule.guards,
        application: rule.application,
        pattern: resolve_pattern_template3(catalog, rule.pattern)?,
        writes: rule
            .writes
            .into_iter()
            .map(|write| resolve_write_template3(catalog, write))
            .collect::<Result<Vec<_>, _>>()?,
    };
    lower_rule_template(&rule)
        .map_err(|error| message(format!("failed to lower resolved 3D rule: {error:?}")))
}

fn lower_line_rule_template3(
    catalog: &crate::Catalog,
    rule: &LineRuleTemplate3,
) -> Result<Vec<Rule3>, ParseError3> {
    project_line_rule_template(rule)
        .into_iter()
        .map(|rule| lower_projected_rule_template3(catalog, rule))
        .collect::<Result<Vec<_>, _>>()
        .map(|rules| rules.into_iter().flatten().collect())
}

fn lower_dense_rule_template3(
    catalog: &crate::Catalog,
    rule: &DenseRuleTemplate3,
) -> Result<Vec<Rule3>, ParseError3> {
    project_dense_rule_template(rule)
        .into_iter()
        .map(|rule| lower_projected_rule_template3(catalog, rule))
        .collect::<Result<Vec<_>, _>>()
        .map(|rules| rules.into_iter().flatten().collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputSpec3 {
    name: String,
    keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LegendSpec3 {
    ch: char,
    selectors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LevelSpec3 {
    name: String,
    pack: Option<String>,
    rows: Vec<String>,
}

fn line_gap_limit_from_levels(level_specs: &[LevelSpec3]) -> u16 {
    level_specs
        .iter()
        .filter_map(|spec| split_level_slices(&spec.rows).ok())
        .filter_map(|slices| {
            let height = slices.len();
            let depth = slices.first()?.len();
            let width = slices.first()?.first()?.chars().count();
            [width, depth, height].into_iter().max()
        })
        .max()
        .and_then(|value| u16::try_from(value.saturating_sub(1)).ok())
        .unwrap_or(DEFAULT_LINE_GAP_LIMIT3)
}

fn legacy_model_setting_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once('=')?;
    match name.trim() {
        "debug_camera" | "camera_yaw" | "camera_pitch" | "camera_roll" | "camera_zoom" => {
            Some(name.trim())
        }
        _ => None,
    }
}

enum ModelSetting3 {
    CameraYaw(i16),
    CameraPitch(i16),
    CameraRoll(i16),
    CameraZoom(u16),
    InteractiveLook(bool),
    InteractiveZoom(bool),
    OccupiedCellGrid,
    SpriteShade(bool),
    Shadow(bool),
    PixelateEnabled(bool),
    PixelateScale(u16),
    PixelateSmoothing(bool),
}

const GRID_TYPE_VALUES3: &[&str] = &["occupied_cells"];

fn parse_camera_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "camera setting")?;
    match name {
        "yaw" => Ok(ModelSetting3::CameraYaw(parse_degrees_setting(
            value, name,
        )?)),
        "pitch" => Ok(ModelSetting3::CameraPitch(parse_degrees_setting(
            value, name,
        )?)),
        "roll" => Ok(ModelSetting3::CameraRoll(parse_degrees_setting(
            value, name,
        )?)),
        "zoom" => Ok(ModelSetting3::CameraZoom(parse_zoom_milli_setting(
            value, name,
        )?)),
        "interactive_look" => Ok(ModelSetting3::InteractiveLook(parse_boolean_setting(
            value, name,
        )?)),
        "interactive_zoom" => Ok(ModelSetting3::InteractiveZoom(parse_boolean_setting(
            value, name,
        )?)),
        _ => Err(message(format!("unknown camera setting: {name}"))),
    }
}

fn parse_grid_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "grid setting")?;
    match name {
        "type" => parse_grid_type_setting(value),
        _ => Err(message(format!("unknown grid setting: {name}"))),
    }
}

fn parse_pixelate_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "pixelate setting")?;
    match name {
        "enabled" => Ok(ModelSetting3::PixelateEnabled(parse_boolean_setting(
            value, name,
        )?)),
        "scale" => Ok(ModelSetting3::PixelateScale(parse_viewport_size_value(
            value,
            "pixelate scale",
        )?)),
        "smoothing" => Ok(ModelSetting3::PixelateSmoothing(parse_boolean_setting(
            value, name,
        )?)),
        _ => Err(message(format!("unknown pixelate setting: {name}"))),
    }
}

fn parse_render_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "render setting")?;
    match name {
        "shade" => Ok(ModelSetting3::SpriteShade(parse_boolean_setting(
            value, name,
        )?)),
        "shadow" => Ok(ModelSetting3::Shadow(parse_boolean_setting(value, name)?)),
        _ => Err(message(format!("unknown render setting: {line}"))),
    }
}

fn parse_grid_type_setting(value: &str) -> Result<ModelSetting3, ParseError3> {
    match value {
        "\"occupied_cells\"" => Ok(ModelSetting3::OccupiedCellGrid),
        _ => Err(message(format!(
            "grid type must be one of: {}",
            GRID_TYPE_VALUES3.join(", ")
        ))),
    }
}

fn parse_viewport_directive(
    line: &str,
    viewport: &mut ViewportSettings3,
) -> Result<(), ParseError3> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        ["flickscreen", width, depth] => {
            parse_paged_viewport_directive(viewport, "flickscreen", width, depth, None)?
        }
        ["flickscreen", width, depth, height] => {
            parse_paged_viewport_directive(viewport, "flickscreen", width, depth, Some(height))?
        }
        ["zoomscreen", width, depth] => parse_centered_viewport_directive(
            viewport,
            "zoomscreen",
            ViewportFollow3::Snap,
            width,
            depth,
            None,
        )?,
        ["zoomscreen", width, depth, height] => parse_centered_viewport_directive(
            viewport,
            "zoomscreen",
            ViewportFollow3::Snap,
            width,
            depth,
            Some(height),
        )?,
        ["smoothscreen", width, depth] => parse_centered_viewport_directive(
            viewport,
            "smoothscreen",
            ViewportFollow3::Smooth,
            width,
            depth,
            None,
        )?,
        ["smoothscreen", width, depth, height] => parse_centered_viewport_directive(
            viewport,
            "smoothscreen",
            ViewportFollow3::Smooth,
            width,
            depth,
            Some(height),
        )?,
        ["focus", selector] => {
            viewport.focus = (*selector).to_string();
        }
        [other, ..] => {
            return Err(message(format!("unknown viewport directive: {other}")));
        }
        [] => {}
    }
    Ok(())
}

fn parse_paged_viewport_directive(
    viewport: &mut ViewportSettings3,
    directive: &str,
    width: &str,
    depth: &str,
    height: Option<&str>,
) -> Result<(), ParseError3> {
    viewport.mode = ViewportMode3::Paged;
    viewport.follow = ViewportFollow3::Snap;
    viewport.framing = Some(ViewportFraming3 {
        width: parse_viewport_size_value(width, &format!("{directive} width"))?,
        depth: parse_viewport_size_value(depth, &format!("{directive} depth"))?,
        height: match height {
            Some(value) => parse_viewport_height_value(value)?,
            None => ViewportHeight3::Full,
        },
    });
    Ok(())
}

fn parse_centered_viewport_directive(
    viewport: &mut ViewportSettings3,
    directive: &str,
    follow: ViewportFollow3,
    width: &str,
    depth: &str,
    height: Option<&str>,
) -> Result<(), ParseError3> {
    viewport.mode = ViewportMode3::Centered;
    viewport.follow = follow;
    viewport.framing = Some(ViewportFraming3 {
        width: parse_viewport_size_value(width, &format!("{directive} width"))?,
        depth: parse_viewport_size_value(depth, &format!("{directive} depth"))?,
        height: match height {
            Some(value) => parse_viewport_height_value(value)?,
            None => ViewportHeight3::Full,
        },
    });
    Ok(())
}

fn parse_viewport_height_value(value: &str) -> Result<ViewportHeight3, ParseError3> {
    if value == "full" {
        return Ok(ViewportHeight3::Full);
    }
    Ok(ViewportHeight3::Size(parse_viewport_size_value(
        value,
        "viewport height",
    )?))
}

fn parse_viewport_size_value(value: &str, name: &str) -> Result<u16, ParseError3> {
    let size = value
        .parse::<u16>()
        .map_err(|_| message(format!("{name} must be a positive integer")))?;
    if size == 0 {
        return Err(message(format!("{name} must be greater than zero")));
    }
    Ok(size)
}

fn parse_setting_assignment<'a>(
    line: &'a str,
    context: &str,
) -> Result<(&'a str, &'a str), ParseError3> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        [name, "=", value] => Ok((*name, *value)),
        _ => Err(message(format!("{context} must be: <name> = <value>"))),
    }
}

fn parse_boolean_setting(value: &str, name: &str) -> Result<bool, ParseError3> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(message(format!("{name} must be true or false"))),
    }
}

fn parse_degrees_setting(value: &str, name: &str) -> Result<i16, ParseError3> {
    value
        .parse::<i16>()
        .map_err(|_| message(format!("{name} must be an integer degree value")))
}

fn parse_zoom_milli_setting(value: &str, name: &str) -> Result<u16, ParseError3> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err(message(format!(
            "{name} must be a positive number with at most three decimal places",
        )));
    }
    let whole = whole
        .parse::<u32>()
        .map_err(|_| message(format!("{name} must be a positive number")))?;
    let fraction = format!("{fraction:0<3}")
        .parse::<u32>()
        .map_err(|_| message(format!("{name} must be a positive number")))?;
    let milli = whole
        .checked_mul(1000)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| message(format!("{name} is too large")))?;
    if milli == 0 || milli > u32::from(u16::MAX) {
        return Err(message(format!(
            "{name} must be greater than 0 and not too large"
        )));
    }
    Ok(milli as u16)
}

fn input_spec_from_key_surface(line: &str) -> Result<InputSpec3, ParseError3> {
    let surface =
        puzzle_authoring::key_binding_surface(line).map_err(|error| message(error.message()))?;
    Ok(InputSpec3 {
        name: surface.target.to_string(),
        keys: surface.keys.into_iter().map(str::to_string).collect(),
    })
}

fn parse_legend_spec(line: &str) -> Result<LegendSpec3, ParseError3> {
    let assignment = puzzle_authoring::selector_assignment_surface(line)
        .ok_or_else(|| message("legend row must be: <char> = selector..."))?;
    let ch = parse_legend_char(assignment.name)?;
    let selectors = assignment
        .selectors
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if selectors.iter().any(|selector| selector == "empty") && selectors.len() > 1 {
        return Err(message("empty cannot be mixed with object selectors"));
    }
    Ok(LegendSpec3 { ch, selectors })
}

fn parse_legend_char(token: &str) -> Result<char, ParseError3> {
    let mut chars = token.chars();
    let Some(ch) = chars.next() else {
        return Err(message("legend row must start with a character"));
    };
    if chars.next().is_some() {
        return Err(message(format!(
            "legend key must be one character: {token}"
        )));
    }
    Ok(ch)
}

fn parse_level_header(line: &str, auto_name: String) -> Result<String, ParseError3> {
    puzzle_authoring::parse_level_header_name_or_auto(line, auto_name)
        .map_err(|error| message(format!("{}: {line}", error.message())))
}

fn parse_scene_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("scene ")?;
    let name = rest.strip_suffix('{')?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn parse_sprite_shapes_entry(
    entry: &crate::model_syntax::PuzzleEntrySyntax,
) -> Result<HashMap<String, Vec<crate::sprite_authoring::SpriteFrameSyntax>>, ParseError3> {
    let mut shapes = HashMap::new();
    let mut index = 0;
    while index < entry.body.len() {
        let line = entry.body[index].text.clone();
        if line.is_empty() {
            index += 1;
            continue;
        }
        let Some(name) = line.strip_suffix('{').map(str::trim) else {
            return Err(message(format!("invalid shape entry: {line}")));
        };
        if !is_canonical_sprite_name(name) {
            return Err(message(format!("invalid sprite shape name: {name}")));
        }
        let (shape, next) = crate::model_syntax::parse_child_entry_at(entry, index)?;
        let rows = shape
            .body
            .into_iter()
            .map(|line| line.text)
            .collect::<Vec<_>>();
        let mut body = Vec::with_capacity(rows.len() + 2);
        body.push("shape = {".to_string());
        body.extend(rows);
        body.push("}".to_string());
        let analyzed = crate::sprite_authoring::analyze_sprite_body(None, &body, |_| false)
            .map_err(|error| message_at_source_line(error.message, &error.line))?;
        let crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) = analyzed.shape else {
            return Err(message(format!("sprite shape {name} requires ASCII rows")));
        };
        crate::sprite_authoring::validate_sprite_frame_geometry(&frames)
            .map_err(|error| message(format!("sprite shape {name}: {error}")))?;
        if shapes.insert(name.to_string(), frames).is_some() {
            return Err(message(format!("duplicate sprite shape: {name}")));
        }
        index = next;
    }
    Ok(shapes)
}

fn lower_sprite_resource3(
    resource: &SpriteResourceSyntax3,
    catalog: &crate::Catalog,
) -> Result<SpriteSet3, ParseError3> {
    let mut sprites = Vec::new();
    let mut names = HashSet::new();
    for attachment in &resource.attachments {
        let syntax = crate::sprite_authoring::parse_sprite_node(
            Some(&attachment.header),
            &attachment.body_lines,
        );
        let selector = syntax
            .selector
            .as_deref()
            .ok_or_else(|| message("sprite entry missing selector"))?;
        for target in expand_sprite_selector3(selector, catalog)? {
            if !names.insert(target.name.clone()) {
                return Err(message(format!(
                    "multiple sprite selectors resolve to {}",
                    target.name
                )));
            }
            sprites.push(parse_sprite3_from_shared_syntax(
                &attachment.header,
                &attachment.body_lines,
                &resource.shapes,
                &target,
            )?);
        }
    }
    Ok(SpriteSet3::new(
        &resource.name,
        resource.model.clone(),
        sprites,
    ))
}

struct SpriteSelectorTarget3 {
    name: String,
    bindings: HashMap<String, String>,
}

fn expand_sprite_selector3(
    selector: &str,
    catalog: &crate::Catalog,
) -> Result<Vec<SpriteSelectorTarget3>, ParseError3> {
    resolve_shared_selector_objects3(catalog, selector, "sprite")?
        .into_iter()
        .map(|object| sprite_selector_target3(object, catalog))
        .collect()
}

fn sprite_selector_target3(
    object: ObjectId,
    catalog: &crate::Catalog,
) -> Result<SpriteSelectorTarget3, ParseError3> {
    for (family_name, family) in &catalog.object_schemas {
        let Some(variant) = family
            .variants
            .iter()
            .find(|candidate| candidate.object == object)
        else {
            continue;
        };
        let bindings = family
            .axes
            .iter()
            .zip(&variant.values)
            .map(|(axis, value)| (axis.clone(), value.clone()))
            .collect();
        return Ok(SpriteSelectorTarget3 {
            name: format!("{family_name}:{}", variant.values.join(":")),
            bindings,
        });
    }
    catalog
        .object_labels
        .get(&object)
        .cloned()
        .map(|name| SpriteSelectorTarget3 {
            name,
            bindings: HashMap::new(),
        })
        .ok_or_else(|| message("sprite selector resolved to an unknown object"))
}

fn parse_sprite3_from_shared_syntax(
    header: &str,
    body: &[String],
    shapes: &HashMap<String, Vec<crate::sprite_authoring::SpriteFrameSyntax>>,
    target: &SpriteSelectorTarget3,
) -> Result<Sprite3, ParseError3> {
    let analyzed = crate::sprite_authoring::analyze_sprite_body(Some(header), body, |shape_name| {
        shapes.contains_key(shape_name)
    })
    .map_err(|error| message_at_source_line(error.message, &error.line))?;
    let syntax = analyzed.syntax;
    let spatial_ops = parse_sprite_spatial_ops3_with_bindings(&syntax, &target.bindings)?;
    let name = target.name.clone();
    let colors = syntax
        .colors
        .clone()
        .ok_or_else(|| message(format!("sprite {name} missing colors")))?;
    let palette = parse_canonical_sprite_palette_line(&colors.join(" "))?;
    let frames = match analyzed.shape {
        crate::sprite_authoring::ResolvedSpriteShape::Reference(reference) => shapes
            .get(&reference)
            .cloned()
            .ok_or_else(|| message(format!("unknown sprite shape `{reference}`")))?,
        crate::sprite_authoring::ResolvedSpriteShape::Inline(frames) => frames,
        crate::sprite_authoring::ResolvedSpriteShape::None => {
            vec![crate::sprite_authoring::SpriteFrameSyntax {
                layers: vec![crate::sprite_authoring::SpriteLayerSyntax {
                    rows: vec![crate::sprite_authoring::SpriteShapeRow {
                        text: "0".to_string(),
                        body_line: 0,
                    }],
                }],
            }]
        }
        crate::sprite_authoring::ResolvedSpriteShape::UnknownBareReference(reference) => {
            return Err(message(format!("unknown sprite shape `{reference}`")));
        }
        crate::sprite_authoring::ResolvedSpriteShape::AmbiguousBareRow(value) => {
            return Err(message(format!(
                "ambiguous sprite shape `{value}`; use `shape = <name>` or `shape = {{ ... }}`"
            )));
        }
    };
    crate::sprite_authoring::validate_sprite_frame_geometry(&frames)
        .map_err(|error| message(format!("sprite {name}: {error}")))?;
    let timing = crate::sprite_authoring::resolve_sprite_timing(
        frames.len(),
        syntax.duration.as_deref(),
        syntax.frame_duration.as_deref(),
    )
    .map_err(|error| message(format!("sprite {name}: {error}")))?;
    let mut compiled_frames = Vec::with_capacity(frames.len());
    for frame in frames {
        let layers = frame
            .layers
            .into_iter()
            .map(|layer| {
                layer
                    .rows
                    .into_iter()
                    .map(|row| row.text)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        compiled_frames.push(parse_sprite_layers(&name, &layers, &palette)?);
    }
    let mut sprite = Sprite3::new(
        name,
        palette,
        compiled_frames,
        timing.duration_ms,
        timing.frame_duration_ms,
    );
    sprite.spatial_ops = spatial_ops;
    Ok(sprite)
}

pub(crate) fn parse_sprite_spatial_ops3(
    syntax: &crate::sprite_authoring::SpriteNodeSyntax,
) -> Result<Vec<crate::SpriteSpatialOp3>, ParseError3> {
    parse_sprite_spatial_ops3_with_bindings(syntax, &HashMap::new())
}

fn parse_sprite_spatial_ops3_with_bindings(
    syntax: &crate::sprite_authoring::SpriteNodeSyntax,
    bindings: &HashMap<String, String>,
) -> Result<Vec<crate::SpriteSpatialOp3>, ParseError3> {
    let mut ops = Vec::new();
    for (property, line) in &syntax.properties {
        match property {
            crate::sprite_authoring::SpritePropertySyntax::Translate { space, value } => {
                ops.push(crate::SpriteSpatialOp3::Translate {
                    space: sprite_space3(*space),
                    value: parse_sprite_vec3(value)
                        .map_err(|error| message(format!("{line}: {error}")))?,
                })
            }
            crate::sprite_authoring::SpritePropertySyntax::Rotate {
                space,
                angle,
                from,
                axis,
            } => {
                // The axis-less surface is the shared 2D form. In a 3D sprite it
                // means a rotation in the XY plane, whose normal is +Z (`up`).
                let axis = axis.as_deref().unwrap_or("up");
                let mut degrees = parse_sprite_angle3(angle, bindings)
                    .map_err(|error| message(format!("{line}: {error}")))?;
                if let Some(from) = from {
                    degrees -= parse_sprite_angle3(from, bindings)
                        .map_err(|error| message(format!("{line}: {error}")))?;
                }
                ops.push(crate::SpriteSpatialOp3::Rotate {
                    space: sprite_space3(*space),
                    axis: parse_sprite_axis3(axis)
                        .map_err(|error| message(format!("{line}: {error}")))?,
                    degrees,
                });
            }
            crate::sprite_authoring::SpritePropertySyntax::Unknown(name) if name == "rotate" => {
                return Err(message(
                    "removed sprite rotation syntax; use rotate [world|local] <angle> [from <angle>] [around <axis>]",
                ));
            }
            _ => {
                return Err(message(format!(
                    "sprite property is not supported by voxel sprites: {line}"
                )));
            }
        }
    }
    Ok(ops)
}

fn sprite_space3(space: crate::sprite_authoring::SpriteSpaceSyntax) -> crate::SpriteSpace3 {
    match space {
        crate::sprite_authoring::SpriteSpaceSyntax::World => crate::SpriteSpace3::World,
        crate::sprite_authoring::SpriteSpaceSyntax::Local => crate::SpriteSpace3::Local,
    }
}

fn parse_sprite_scalar3(value: &str) -> Result<f64, String> {
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

fn parse_sprite_vec3(value: &str) -> Result<[f64; 3], String> {
    let inner = value
        .trim()
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| "3D sprite translate requires a vec3".to_string())?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    let [x, y, z] = parts.as_slice() else {
        return Err("3D sprite translate requires a vec3".to_string());
    };
    Ok([
        parse_sprite_scalar3(x)?,
        parse_sprite_scalar3(y)?,
        parse_sprite_scalar3(z)?,
    ])
}

fn parse_sprite_axis3(value: &str) -> Result<[f64; 3], String> {
    let axis = match value.trim() {
        "right" => [1.0, 0.0, 0.0],
        "left" => [-1.0, 0.0, 0.0],
        "front" => [0.0, 1.0, 0.0],
        "back" => [0.0, -1.0, 0.0],
        "up" => [0.0, 0.0, 1.0],
        "down" => [0.0, 0.0, -1.0],
        _ => parse_sprite_vec3(value)?,
    };
    let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if length == 0.0 {
        return Err("3D sprite rotate axis cannot be zero".to_string());
    }
    Ok([axis[0] / length, axis[1] / length, axis[2] / length])
}

fn parse_sprite_angle3(value: &str, bindings: &HashMap<String, String>) -> Result<f64, String> {
    let value = value.trim();
    let value = bindings.get(value).map(String::as_str).unwrap_or(value);
    if let Some(degrees) = sprite_horizontal_direction_degrees3(value) {
        return Ok(degrees);
    }
    let degrees = value.strip_suffix("deg").ok_or_else(|| {
        "3D sprite rotate expression must resolve to an angle or horizontal direction".to_string()
    })?;
    parse_sprite_scalar3(degrees)
}

fn sprite_horizontal_direction_degrees3(value: &str) -> Option<f64> {
    Some(match value {
        "right" => 0.0,
        "front" => 90.0,
        "left" => 180.0,
        "back" => -90.0,
        _ => return None,
    })
}

fn lower_level_bundle(
    game: CompiledGame3,
    catalog: &crate::Catalog,
    legend_specs: &[LegendSpec3],
    level_specs: &[LevelSpec3],
) -> Result<LevelBundle3, ParseError3> {
    let legend = lower_legend(catalog, legend_specs)?;
    let levels = level_specs
        .iter()
        .map(|spec| {
            Ok(LevelEntry3::new(
                spec.name.clone(),
                lower_level_spec(&legend, spec)?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError3>>()?;
    LevelBundle3::checked_new(game, levels)
        .map_err(|error| message(format!("invalid level bundle: {error:?}")))
}

fn lower_legend(
    catalog: &crate::Catalog,
    specs: &[LegendSpec3],
) -> Result<BTreeMap<char, Vec<ObjectId>>, ParseError3> {
    let mut legend = BTreeMap::new();
    legend.insert('.', Vec::new());
    for spec in specs {
        let mut objects = Vec::new();
        if spec.selectors.len() == 1 && spec.selectors[0] == "empty" {
            if spec.ch != '.' {
                return Err(message(format!(
                    "levels use `.` for empty; remove `{}` = empty",
                    spec.ch
                )));
            }
            continue;
        }
        if spec.ch == '.' {
            return Err(message(
                "levels reserve `.` for empty; use another legend char for objects",
            ));
        }
        if legend.contains_key(&spec.ch) {
            return Err(message(format!("duplicate legend char: {}", spec.ch)));
        } else {
            for token in &spec.selectors {
                for object in resolve_shared_selector_objects3(catalog, token, "legend")? {
                    push_unique_object(&mut objects, object);
                }
            }
        }
        legend.insert(spec.ch, objects);
    }
    Ok(legend)
}

fn lower_level_spec(
    legend: &BTreeMap<char, Vec<ObjectId>>,
    spec: &LevelSpec3,
) -> Result<Level3, ParseError3> {
    let slices = split_level_slices(&spec.rows)?;
    let depth = slices[0].len();
    let width = slices[0][0].chars().count();
    if width == 0 {
        return Err(message(format!("level {} has an empty row", spec.name)));
    }

    let size = Size3::new(width as u16, depth as u16, slices.len() as u16);
    let mut cells = Vec::new();
    for (z, slice) in slices.iter().enumerate() {
        if slice.len() != depth {
            return Err(message(format!(
                "level {} slices must have the same depth",
                spec.name
            )));
        }
        for (y, row) in slice.iter().enumerate() {
            if row.chars().count() != width {
                return Err(message(format!(
                    "level {} slices must have the same width",
                    spec.name
                )));
            }
            for (x, ch) in row.chars().enumerate() {
                let objects = legend.get(&ch).ok_or_else(|| {
                    message(format!(
                        "level {} uses unknown legend char: {ch}",
                        spec.name
                    ))
                })?;
                if objects.is_empty() {
                    continue;
                }
                cells.push(LevelCell3::new(
                    Coord3::from_standard_text_position(size, x as u16, y as u16, z as u16),
                    objects.clone(),
                ));
            }
        }
    }

    Ok(Level3::new(size, cells))
}

fn is_canonical_sprite_name(line: &str) -> bool {
    !line.is_empty()
        && !line.contains(char::is_whitespace)
        && !line.contains('{')
        && !line.contains('}')
        && !line.contains('=')
        && !is_canonical_sprite_palette_line(line)
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '@'))
}

pub(crate) fn is_canonical_sprite_palette_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace().peekable();
    tokens.peek().is_some()
        && tokens.all(|token| token == "transparent" || crate::is_visual_color_token(token))
}

pub(crate) fn parse_canonical_sprite_palette_line(
    line: &str,
) -> Result<BTreeMap<char, SpriteColor3>, ParseError3> {
    if !is_canonical_sprite_palette_line(line) {
        return Err(message(
            "sprite palette row must be whitespace-separated <color|transparent> values",
        ));
    }
    let mut palette = BTreeMap::new();
    for (index, token) in line.split_whitespace().enumerate() {
        let key = sprite_palette_key(index)?;
        let color = if token == "transparent" {
            SpriteColor3::Transparent
        } else {
            SpriteColor3::Hex(token.to_string())
        };
        palette.insert(key, color);
    }
    Ok(palette)
}

fn sprite_palette_key(index: usize) -> Result<char, ParseError3> {
    const KEYS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    KEYS.chars()
        .nth(index)
        .ok_or_else(|| message("sprite palette supports at most 62 colors"))
}

pub(crate) fn parse_sprite_layers(
    sprite_name: &str,
    layers: &[Vec<String>],
    palette: &BTreeMap<char, SpriteColor3>,
) -> Result<SpriteVoxels3, ParseError3> {
    let Some(first_layer) = layers.first() else {
        return Err(message(format!(
            "sprite {sprite_name} requires at least one Z layer"
        )));
    };
    let Some(first_row) = first_layer.first() else {
        return Err(message(format!(
            "sprite {sprite_name} Z layer requires at least one row"
        )));
    };
    let height = first_layer.len();
    let width = first_row.chars().count();
    if width == 0 {
        return Err(message(format!("sprite {sprite_name} has an empty row")));
    }
    for layer in layers {
        if layer.is_empty() {
            return Err(message(format!(
                "sprite {sprite_name} Z layer requires at least one row"
            )));
        }
        if layer.len() != height {
            return Err(message(format!(
                "sprite {sprite_name} Z layers must have the same height"
            )));
        }
        for row in layer {
            if row.chars().count() != width {
                return Err(message(format!(
                    "sprite {sprite_name} Z layers must have the same width"
                )));
            }
            for ch in row.chars() {
                if is_implicit_transparent_sprite_char(ch) {
                    continue;
                }
                if !palette.contains_key(&ch) {
                    return Err(message(format!(
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

fn is_implicit_transparent_sprite_char(ch: char) -> bool {
    ch == '.' || ch == ' '
}

fn split_level_slices(rows: &[String]) -> Result<Vec<Vec<String>>, ParseError3> {
    let mut slices = Vec::<Vec<String>>::new();
    let mut current = Vec::<String>::new();
    for row in rows {
        if row.trim().is_empty() {
            if !current.is_empty() {
                slices.push(std::mem::take(&mut current));
            }
            continue;
        }
        if row == "-" {
            if current.is_empty() {
                return Err(message(
                    "3D level slice separator requires a preceding ASCII slice",
                ));
            }
            slices.push(std::mem::take(&mut current));
            continue;
        }
        current.push(row.clone());
    }
    if current.is_empty() && rows.last().is_some_and(|row| row == "-") {
        return Err(message(
            "3D level slice separator requires a following ASCII slice",
        ));
    }
    if !current.is_empty() {
        slices.push(current);
    }
    if slices.is_empty() {
        return Err(message("level requires at least one height slice"));
    }
    Ok(slices)
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn parse_lifecycle_command_line(line: &str) -> Result<LifecycleCommand, ParseError3> {
    puzzle_scene::parse_scene_effect_at(line, line)
        .map_err(|error| message_at_source_line(error.message(), error.source_line()))
}

fn parse_optional_program_local_frame(
    modifier: Option<&str>,
    catalog: &crate::Catalog,
) -> Result<Option<LocalFrame<ObjectId>>, ParseError3> {
    let Some(modifier) = modifier else {
        return Ok(None);
    };
    let tokens = modifier.split_whitespace().collect::<Vec<_>>();
    let focus_objects = default_local_frame_focus_objects(catalog)?;
    match tokens.as_slice() {
        ["local_radius", radius] => {
            let radius = parse_u16_token(radius, "local_radius")?;
            Ok(Some(LocalFrame::new(
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                focus_objects,
            )))
        }
        ["local_frame", x, y, z] => Ok(Some(LocalFrame::new(
            parse_local_frame_extent3(x)?,
            parse_local_frame_extent3(y)?,
            parse_local_frame_extent3(z)?,
            focus_objects,
        ))),
        _ => Err(message(
            "transition block header must use local_radius <n> or local_frame <x> <y> <z>",
        )),
    }
}

fn parse_local_frame_extent3(token: &str) -> Result<LocalFrameExtent, ParseError3> {
    if token == "full" {
        return Ok(LocalFrameExtent::Full);
    }
    parse_u16_token(token, "local_frame").map(LocalFrameExtent::Radius)
}

fn parse_u16_token(token: &str, context: &str) -> Result<u16, ParseError3> {
    token
        .parse::<u16>()
        .map_err(|_| message(format!("{context} value must be a non-negative integer")))
}

fn default_local_frame_focus_objects(
    catalog: &crate::Catalog,
) -> Result<Vec<ObjectId>, ParseError3> {
    for name in ["Player", "player"] {
        if let Some(object) = catalog.object_names.get(name) {
            return Ok(vec![*object]);
        }
    }
    Err(message(
        "local_frame/local_radius requires an object named Player",
    ))
}

struct ParsedRuleLine3 {
    rules: Vec<Rule3>,
    camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
}

impl ParsedRuleLine3 {
    fn new(rules: Vec<Rule3>, camera_effects: Vec<Vec<Puzzle3CameraEffect>>) -> Self {
        debug_assert_eq!(rules.len(), camera_effects.len());
        Self {
            rules,
            camera_effects,
        }
    }
}

fn lower_rule_statement_syntax3(
    statements: &[puzzle_authoring::RuleStatementSyntax],
    catalog: &crate::Catalog,
    line_gap_limit: u16,
    input_guard: Option<&str>,
    bindings: &[(String, String)],
) -> Result<Vec<ParsedRuleLine3>, ParseError3> {
    let mut lowered = Vec::new();
    for statement in statements {
        match statement {
            puzzle_authoring::RuleStatementSyntax::Line(line) => {
                let line = substitute_rule_bindings3(line, bindings)?;
                let guarded;
                let line = if let Some(input) = input_guard {
                    guarded = format!("input {input} {line}");
                    guarded.as_str()
                } else {
                    line.as_str()
                };
                lowered.push(parse_rule_line(line, catalog, line_gap_limit)?);
            }
            puzzle_authoring::RuleStatementSyntax::Block { header, statements } => {
                let header = substitute_rule_bindings3(header, bindings)?;
                if let Some(for_syntax) =
                    crate::rule_syntax::parse_rule_for_syntax(&header).map_err(message)?
                {
                    let source_refs = for_syntax
                        .sources
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>();
                    let values = crate::for_expansion_values(
                        &source_refs,
                        &crate::catalog_value_sets(catalog),
                        &catalog.numeric_variable_defaults,
                        &header,
                    )?;
                    for value in values {
                        let mut nested_bindings = bindings.to_vec();
                        nested_bindings
                            .push((for_syntax.binding.clone(), value.value().to_string()));
                        lowered.extend(lower_rule_statement_syntax3(
                            statements,
                            catalog,
                            line_gap_limit,
                            input_guard,
                            &nested_bindings,
                        )?);
                    }
                    continue;
                }
                let tokens = puzzle_authoring::split_header_tokens(&header);
                match tokens.as_slice() {
                    ["if", "input", "==", input] => {
                        lowered.extend(lower_rule_statement_syntax3(
                            statements,
                            catalog,
                            line_gap_limit,
                            Some(input),
                            bindings,
                        )?);
                    }
                    _ => {
                        return Err(message(format!(
                            "3D lowering does not support rule block `{header}`"
                        )));
                    }
                }
            }
        }
    }
    Ok(lowered)
}

fn substitute_rule_bindings3(
    line: &str,
    bindings: &[(String, String)],
) -> Result<String, ParseError3> {
    let mut expanded = line.to_string();
    for (binding, value) in bindings {
        expanded = crate::rule_syntax::substitute_rule_binding_line(
            &expanded,
            binding,
            |projection| match projection {
                None => Ok(value.clone()),
                Some(attr) => Err(message(format!(
                    "3D rule expansion does not define projection `{binding}.{attr}`"
                ))),
            },
            |_, _| Ok(None),
        )?;
    }
    Ok(expanded)
}

fn parse_rule_line(
    line: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<ParsedRuleLine3, ParseError3> {
    if let Some(effect) = parse_camera_rule_effect_line(line)? {
        return Ok(ParsedRuleLine3::new(
            vec![Rule3::once(Pattern3::new(Vec::new()), Vec::new())],
            vec![vec![effect]],
        ));
    }

    let surface =
        puzzle_authoring::rule_statement_surface(line).map_err(|error| message(error.message()))?;
    let surface = match surface {
        puzzle_authoring::RuleStatementSurface::RuleLine(surface) => surface,
        puzzle_authoring::RuleStatementSurface::ApplicationBlock { .. } => {
            return Err(message(
                "3D rule blocks do not support nested application blocks",
            ));
        }
        puzzle_authoring::RuleStatementSurface::Call { .. } => {
            return Err(message("3D rule blocks do not support routine calls"));
        }
    };
    let (application, prefix, rest) = match surface {
        puzzle_authoring::RuleLineSurface::InputRewrite {
            application,
            surface,
        } => {
            let orientation = match surface.orientation {
                Some(orientation) => parse_line_orientation(orientation)?,
                None => LineOrientation3::DirectionSet(DirectionSet3::Directions),
            };
            let (lhs, rhs, effects) = unresolved_rewrite3(surface.rewrite, catalog)?;
            let mut rules =
                lower_input_line_rewrite(catalog, orientation, &lhs, &rhs, line_gap_limit)
                    .map_err(|error| {
                        message(format!("failed to lower input line rule: {error}"))
                    })?;
            apply_rule_application(&mut rules, application)?;
            return Ok(parsed_rules_with_effects(rules, &effects));
        }
        puzzle_authoring::RuleLineSurface::NeutralRewrite {
            application,
            rewrite,
        } => {
            let (lhs, rhs, effects) = unresolved_rewrite3(rewrite, catalog)?;
            let mut rules = lower_line_rewrite(
                catalog,
                LineOrientation3::DirectionSet(DirectionSet3::Directions),
                &lhs,
                &rhs,
                line_gap_limit,
            )
            .map_err(|error| message(format!("failed to lower line rule: {error}")))?;
            apply_rule_application(&mut rules, application)?;
            return Ok(parsed_rules_with_effects(rules, &effects));
        }
        puzzle_authoring::RuleLineSurface::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => (application, orientation, rewrite),
    };
    let (lhs, rhs, effects) = unresolved_rewrite3(rest, catalog)?;
    if prefix.contains(',') || matches!(prefix, "frames" | "canonical" | "mirrored") {
        let orientation = parse_frame_orientation(prefix)?;
        let rule = DenseRuleTemplate3::once(
            orientation,
            materialize_dense_pattern3(&lhs, catalog)?,
            infer_dense_writes(&lhs, &rhs, catalog)?,
        );
        let mut rules = lower_dense_rule_template3(catalog, &rule)?;
        apply_rule_application(&mut rules, application)?;
        return Ok(parsed_rules_with_effects(rules, &effects));
    }

    let orientation = parse_line_orientation(prefix)?;
    let mut rules = lower_line_rewrite(catalog, orientation, &lhs, &rhs, line_gap_limit)
        .map_err(|error| message(format!("failed to lower line rule: {error}")))?;
    apply_rule_application(&mut rules, application)?;
    Ok(parsed_rules_with_effects(rules, &effects))
}

fn apply_rule_application(
    rules: &mut [Rule3],
    application: Option<puzzle_authoring::RuleApplicationSurface>,
) -> Result<(), ParseError3> {
    let application = match application.unwrap_or(puzzle_authoring::RuleApplicationSurface::Repeat)
    {
        puzzle_authoring::RuleApplicationSurface::Once => RuleApplication3::Once,
        puzzle_authoring::RuleApplicationSurface::OnceAll => RuleApplication3::OnceAll,
        puzzle_authoring::RuleApplicationSurface::OncePerLevel => RuleApplication3::OncePerLevel,
        puzzle_authoring::RuleApplicationSurface::Random => RuleApplication3::Random,
        puzzle_authoring::RuleApplicationSurface::Repeat => RuleApplication3::UntilStable,
    };
    for rule in rules {
        rule.application = application;
    }
    Ok(())
}

fn program_step_for_rule_statement(
    mut rules: Vec<Rule3>,
    next_rule_id: &mut u16,
) -> Result<RuleStep3, ParseError3> {
    if rules.is_empty() {
        return Err(message("3D rewrite statement lowered to no alternatives"));
    }
    let application = rules
        .first()
        .map(|rule| rule.application)
        .expect("non-empty 3D rewrite alternatives have an application");
    if rules.iter().any(|rule| rule.application != application) {
        return Err(message(
            "one 3D rewrite statement lowered to mixed rule applications",
        ));
    }
    for rule in &mut rules {
        rule.id = RuleId3(*next_rule_id);
        *next_rule_id = next_rule_id
            .checked_add(1)
            .ok_or_else(|| message("too many lowered 3D rules"))?;
    }

    match application {
        RuleApplication3::Once => once_alternative_chain3(rules, application)
            .ok_or_else(|| message("3D once statement lowered to no alternatives")),
        RuleApplication3::OnceAll | RuleApplication3::OncePerLevel => {
            let steps = rules
                .into_iter()
                .map(|mut rule| {
                    rule.application = application;
                    RuleStep3::Rule(rule)
                })
                .collect();
            Ok(RuleStep3::Block {
                application,
                stop_condition: None,
                steps,
            })
        }
        RuleApplication3::Random | RuleApplication3::UntilStable => {
            let nested_application = if application == RuleApplication3::Random {
                RuleApplication3::Random
            } else {
                RuleApplication3::Once
            };
            let steps = rules
                .into_iter()
                .map(|mut rule| {
                    rule.application = nested_application;
                    RuleStep3::Rule(rule)
                })
                .collect();
            Ok(RuleStep3::Block {
                application,
                stop_condition: None,
                steps,
            })
        }
    }
}

fn once_alternative_chain3(rules: Vec<Rule3>, application: RuleApplication3) -> Option<RuleStep3> {
    let alternatives = rules
        .into_iter()
        .map(|mut rule| {
            let mut guards = rule.guards.clone();
            guards.push(Guard3::InlineConditionNonZero(
                ConditionValueKind3::ExistsMatches(vec![rule.pattern.clone()]),
            ));
            let condition = RuleCondition3::GuardBranches(vec![guards]);
            rule.application = application;
            (condition, rule)
        })
        .collect();
    puzzle_kernel::first_matching_program_alternative(alternatives)
}

fn parsed_rules_without_camera_effects(rules: Vec<Rule3>) -> ParsedRuleLine3 {
    let camera_effects = vec![Vec::new(); rules.len()];
    ParsedRuleLine3::new(rules, camera_effects)
}

fn parsed_rules_with_camera_effects(
    rules: Vec<Rule3>,
    effects: &[Puzzle3CameraEffect],
) -> ParsedRuleLine3 {
    if effects.is_empty() {
        return parsed_rules_without_camera_effects(rules);
    }
    let camera_effects = vec![effects.to_vec(); rules.len()];
    ParsedRuleLine3::new(rules, camera_effects)
}

fn parsed_rules_with_effects(mut rules: Vec<Rule3>, effects: &RuleEffects3) -> ParsedRuleLine3 {
    for rule in &mut rules {
        rule.effects.extend(effects.core.iter().cloned());
    }
    parsed_rules_with_camera_effects(rules, &effects.camera)
}

fn input_for_direction(direction: Direction3) -> InputId {
    match direction.name {
        "left" => InputId(0),
        "right" => InputId(1),
        "up" => InputId(2),
        "down" => InputId(3),
        "front" => InputId(4),
        "back" => InputId(5),
        _ => unreachable!("built-in directions are exhaustive"),
    }
}

struct QueryLoweringContext3d<'a> {
    catalog: &'a crate::Catalog,
    line_gap_limit: u16,
}

fn validate_query_definitions3(
    definitions: &[crate::solver_surface::SolverSurfaceQueryDefinition],
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<(), ParseError3> {
    let context = QueryLoweringContext3d {
        catalog,
        line_gap_limit,
    };
    crate::solver_surface::lower_query_definitions_with::<QueryLoweringAdapter3d, _>(
        definitions,
        &context,
    )
    .map(|_| ())
}

fn lower_solver_strategy3(
    strategy: Option<crate::solver_surface::SolverSurfaceStrategy>,
    query_definitions: &[crate::solver_surface::SolverSurfaceQueryDefinition],
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<SolverStrategy3, ParseError3> {
    let context = QueryLoweringContext3d {
        catalog,
        line_gap_limit,
    };
    crate::solver_surface::lower_solver_strategy_with::<QueryLoweringAdapter3d, _>(
        strategy,
        query_definitions,
        &context,
    )
}

struct QueryLoweringAdapter3d;

impl<'a> crate::solver_surface::SolverQueryLoweringAdapter<QueryLoweringContext3d<'a>>
    for QueryLoweringAdapter3d
{
    type Object = ObjectId;
    type Value = ConditionValueKind3;
    type Variable = puzzle_grid3d::VariableId;
    type Error = ParseError3;

    fn lower_distance_selector(
        selector: &SolverSurfaceQueryArg,
        source_line: &str,
        context: &QueryLoweringContext3d<'a>,
    ) -> Result<Vec<Self::Object>, Self::Error> {
        let SolverSurfaceQueryArg::Selector(selector) = selector else {
            return Err(message_at_source_line(
                "distance query must be: distance(<selector>, <selector>)",
                source_line,
            ));
        };
        resolve_query_selector_objects(context.catalog, selector, source_line)
    }

    fn lower_selector_query_value(
        kind: crate::solver_surface::SolverQueryCallKind,
        selector: &str,
        source_line: &str,
        context: &QueryLoweringContext3d<'a>,
    ) -> Result<Self::Value, Self::Error> {
        let objects = resolve_query_selector_objects(context.catalog, selector, source_line)?;
        Ok(match kind {
            crate::solver_surface::SolverQueryCallKind::Count => {
                ConditionValueKind3::CountObjects(objects)
            }
            crate::solver_surface::SolverQueryCallKind::Exists => {
                ConditionValueKind3::ExistsObjects(objects)
            }
            crate::solver_surface::SolverQueryCallKind::None => {
                ConditionValueKind3::NoneObjects(objects)
            }
        })
    }

    fn lower_pattern_query_value(
        kind: crate::solver_surface::SolverQueryCallKind,
        pattern: &SolverSurfacePatternArg,
        _source_line: &str,
        context: &QueryLoweringContext3d<'a>,
    ) -> Result<Self::Value, Self::Error> {
        let patterns =
            lower_surface_pattern_arg3d(pattern, context.catalog, context.line_gap_limit)?;
        Ok(match kind {
            crate::solver_surface::SolverQueryCallKind::Count => {
                ConditionValueKind3::CountMatches(patterns)
            }
            crate::solver_surface::SolverQueryCallKind::Exists => {
                ConditionValueKind3::ExistsMatches(patterns)
            }
            crate::solver_surface::SolverQueryCallKind::None => {
                ConditionValueKind3::NoneMatches(patterns)
            }
        })
    }

    fn query_call_error(message: &'static str, source_line: &str) -> Self::Error {
        message_at_source_line(message, source_line)
    }

    fn cycle_error(cycle: Vec<String>, source_line: &str) -> Self::Error {
        message_at_source_line(
            format!("query definitions contain a cycle: {}", cycle.join(" -> ")),
            source_line,
        )
    }

    fn unknown_query_error(name: &str, source_line: &str) -> Self::Error {
        message_at_source_line(
            format!("unknown query in query expression: {name}"),
            source_line,
        )
    }
}

fn lower_win_conditions(
    catalog: &crate::Catalog,
    lines: &[String],
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    let conditions = lines
        .iter()
        .map(|line| parse_win_condition_line(line, catalog, line_gap_limit))
        .collect::<Result<Vec<_>, ParseError3>>()?;
    if conditions.is_empty() {
        return Err(message(
            "win_conditions block must contain at least one condition",
        ));
    }
    if conditions.len() == 1 {
        Ok(conditions.into_iter().next().unwrap())
    } else {
        Ok(WinCondition3::All(conditions))
    }
}

fn parse_win_condition_line(
    line: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    match puzzle_authoring::win_condition_row_surface(line).map_err(message)? {
        puzzle_authoring::WinConditionRowSurface::Query {
            quantifier: puzzle_authoring::WinConditionQuantifier::Exists,
            argument,
        } => parse_some_condition(argument, catalog, line_gap_limit),
        puzzle_authoring::WinConditionRowSurface::Query {
            quantifier: puzzle_authoring::WinConditionQuantifier::None,
            argument,
        } => parse_no_condition(argument, catalog, line_gap_limit),
        puzzle_authoring::WinConditionRowSurface::SomeOn { subject, cover } => {
            parse_some_condition(&format!("[ {subject} {cover} ]"), catalog, line_gap_limit)
        }
        puzzle_authoring::WinConditionRowSurface::AllOn { subject, cover } => {
            parse_all_on_condition(&format!("{subject} on {cover}"), catalog)
        }
        puzzle_authoring::WinConditionRowSurface::Expression(_) => {
            Err(message(format!("unknown win condition: {line}")))
        }
    }
}

fn parse_some_condition(
    rest: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, line_gap_limit, WinCondition3::SomePattern);
    }
    let objects = resolve_shared_selector_objects3(catalog, rest, "win")?;
    Ok(any_object_condition(objects, WinCondition3::SomeObject))
}

fn parse_no_condition(
    rest: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, line_gap_limit, WinCondition3::NoPattern);
    }
    let objects = resolve_shared_selector_objects3(catalog, rest, "win")?;
    Ok(all_object_condition(objects, WinCondition3::NoObject))
}

fn parse_all_on_condition(
    rest: &str,
    catalog: &crate::Catalog,
) -> Result<WinCondition3, ParseError3> {
    let (object_selector, cover) = rest
        .split_once(" on ")
        .ok_or_else(|| message("all condition must be: all <selector> on <selector>"))?;
    let object = resolve_single_selector_object(catalog, object_selector.trim())?;
    if cover.contains('[') {
        return Err(message(
            "all <selector> on <pattern> is not valid; use some/no <orientation> [ ... ] for spatial win patterns",
        ));
    }
    let cover_object = resolve_single_selector_object(catalog, cover.trim())?;
    Ok(WinCondition3::AllObjectsCoveredByPattern {
        object,
        cover_pattern: Pattern3::new(vec![
            MatchCell3::new(Delta3::ZERO)
                .require(object)
                .require(cover_object),
        ]),
    })
}

fn pattern_conditions(
    rest: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
    wrap: fn(Pattern3) -> WinCondition3,
) -> Result<WinCondition3, ParseError3> {
    let patterns = parse_oriented_patterns(rest, catalog, line_gap_limit)?;
    if patterns.len() == 1 {
        Ok(wrap(patterns.into_iter().next().unwrap()))
    } else {
        Ok(WinCondition3::Any(patterns.into_iter().map(wrap).collect()))
    }
}

fn any_object_condition(
    objects: Vec<ObjectId>,
    wrap: fn(ObjectId) -> WinCondition3,
) -> WinCondition3 {
    if objects.len() == 1 {
        wrap(objects[0])
    } else {
        WinCondition3::Any(objects.into_iter().map(wrap).collect())
    }
}

fn all_object_condition(
    objects: Vec<ObjectId>,
    wrap: fn(ObjectId) -> WinCondition3,
) -> WinCondition3 {
    if objects.len() == 1 {
        wrap(objects[0])
    } else {
        WinCondition3::All(objects.into_iter().map(wrap).collect())
    }
}

fn resolve_query_selector_objects(
    catalog: &crate::Catalog,
    token: &str,
    source_line: &str,
) -> Result<Vec<ObjectId>, ParseError3> {
    resolve_shared_selector_objects3(catalog, token, "query")
        .map_err(|error| message_at_source_line(diagnostic_message(&error), source_line))
}

fn resolve_single_selector_object(
    catalog: &crate::Catalog,
    token: &str,
) -> Result<ObjectId, ParseError3> {
    let objects = resolve_shared_selector_objects3(catalog, token, "win")?;
    if objects.len() != 1 {
        return Err(message(format!(
            "win selector must resolve to one object: {token}"
        )));
    }
    Ok(objects[0])
}

fn parse_oriented_patterns(
    value: &str,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<Vec<Pattern3>, ParseError3> {
    let Some(surface) = crate::solver_surface::oriented_pattern_arg_surface(value, value)
        .map_err(diagnostic_report_error3)?
    else {
        return Err(message("pattern condition must be: <orientation> [ ... ]"));
    };
    let pattern = SolverSurfacePatternArg {
        source: value.to_string(),
        orientation: match surface.orientation {
            crate::solver_surface::OrientedPatternArgOrientationSurface::Neutral => {
                crate::solver_surface::SolverSurfacePatternOrientation::Neutral
            }
            crate::solver_surface::OrientedPatternArgOrientationSurface::Input { axis, .. } => {
                crate::solver_surface::SolverSurfacePatternOrientation::Input {
                    axis: axis.map(|axis| value[axis].to_string()),
                }
            }
            crate::solver_surface::OrientedPatternArgOrientationSurface::Orientation {
                orientation,
            } => crate::solver_surface::SolverSurfacePatternOrientation::Orientation(
                value[orientation].to_string(),
            ),
        },
        pattern: value[surface.pattern].to_string(),
    };
    lower_surface_pattern_arg3d(&pattern, catalog, line_gap_limit)
}

fn lower_surface_pattern_arg3d(
    pattern: &SolverSurfacePatternArg,
    catalog: &crate::Catalog,
    line_gap_limit: u16,
) -> Result<Vec<Pattern3>, ParseError3> {
    let syntax = puzzle_authoring::parse_unresolved_pattern_syntax(pattern.pattern.trim())
        .map_err(|error| message(error.message()))?;
    let Some(orientation) = surface_pattern_orientation3d(pattern)? else {
        return lower_line_patterns(
            catalog,
            LineOrientation3::DirectionSet(DirectionSet3::Directions),
            &syntax,
            line_gap_limit,
        )
        .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
        .map_err(|error| message(format!("failed to lower pattern: {error}")));
    };
    if orientation.contains(',')
        || matches!(orientation.as_str(), "frames" | "canonical" | "mirrored")
    {
        let orientation = parse_frame_orientation(&orientation)?;
        let rule = DenseRuleTemplate3::once(
            orientation,
            materialize_dense_pattern3(&syntax, catalog)?,
            Vec::new(),
        );
        return lower_dense_rule_template3(catalog, &rule)
            .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
            .map_err(|error| message(format!("failed to lower pattern: {error:?}")));
    }
    let orientation = parse_line_orientation(&orientation)?;
    lower_line_patterns(catalog, orientation, &syntax, line_gap_limit)
        .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
        .map_err(|error| message(format!("failed to lower pattern: {error}")))
}

fn surface_pattern_orientation3d(
    pattern: &SolverSurfacePatternArg,
) -> Result<Option<String>, ParseError3> {
    match &pattern.orientation {
        crate::solver_surface::SolverSurfacePatternOrientation::Neutral => Ok(None),
        crate::solver_surface::SolverSurfacePatternOrientation::Input { axis } => Ok(Some(
            axis.clone().unwrap_or_else(|| "directions".to_string()),
        )),
        crate::solver_surface::SolverSurfacePatternOrientation::Orientation(orientation) => {
            Ok(Some(orientation.clone()))
        }
    }
}

fn unresolved_rewrite3(
    source: &str,
    catalog: &crate::Catalog,
) -> Result<
    (
        puzzle_authoring::UnresolvedPatternSyntax,
        puzzle_authoring::UnresolvedPatternSyntax,
        RuleEffects3,
    ),
    ParseError3,
> {
    let syntax = puzzle_authoring::parse_unresolved_rewrite_syntax(source)
        .map_err(|error| message(error.message()))?;
    let effects = parse_rule_effect_suffix(&syntax.suffix, catalog)?;
    let after = syntax.after.unwrap_or_else(|| syntax.before.clone());
    Ok((syntax.before, after, effects))
}

#[derive(Clone, Debug, Default)]
struct RuleEffects3 {
    core: Vec<puzzle_grid3d::RuleEffect3>,
    camera: Vec<Puzzle3CameraEffect>,
}

fn parse_rule_effect_suffix(
    suffix: &str,
    catalog: &crate::Catalog,
) -> Result<RuleEffects3, ParseError3> {
    if suffix.is_empty() {
        return Ok(RuleEffects3::default());
    }
    if let Some(effect) = parse_camera_rule_effect_line(suffix)? {
        return Ok(RuleEffects3 {
            camera: vec![effect],
            ..RuleEffects3::default()
        });
    }
    let effects =
        crate::parse_rewrite_effect(suffix, suffix).map_err(|error| message(error.to_string()))?;
    let core = effects
        .into_iter()
        .map(|effect| lower_rule_effect3(effect, catalog))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RuleEffects3 {
        core,
        camera: Vec::new(),
    })
}

fn lower_rule_effect3(
    effect: crate::EffectAst,
    catalog: &crate::Catalog,
) -> Result<puzzle_grid3d::RuleEffect3, ParseError3> {
    use crate::{EffectAst, VariableValueAst};
    use puzzle_grid3d::RuleEffect3;
    Ok(match effect {
        EffectAst::Cancel => RuleEffect3::Cancel,
        EffectAst::Win => RuleEffect3::Win,
        EffectAst::Restart => RuleEffect3::Restart,
        EffectAst::NextLevel => RuleEffect3::NextLevel,
        EffectAst::Again => RuleEffect3::Again,
        EffectAst::Checkpoint => RuleEffect3::Checkpoint,
        EffectAst::ClearCheckpoint => RuleEffect3::ClearCheckpoint,
        EffectAst::UpdateVariable { name, op, value } => {
            let variable = catalog
                .variable_names
                .get(&name)
                .copied()
                .ok_or_else(|| message(format!("unknown variable in effect: {name}")))?;
            if catalog.constant_variables.contains(&variable) {
                return Err(message(format!("cannot update const: {name}")));
            }
            let VariableValueAst::Literal(value) = value else {
                return Err(message(
                    "tag capture values require canonical rewrite capture lowering",
                ));
            };
            RuleEffect3::UpdateVariable {
                variable: puzzle_grid3d::VariableId(variable.0),
                op,
                value,
            }
        }
        EffectAst::PlaySfx { .. }
        | EffectAst::PlayMusic { .. }
        | EffectAst::PauseMusic { .. }
        | EffectAst::ResumeMusic { .. }
        | EffectAst::StopMusic { .. }
        | EffectAst::Wait { .. }
        | EffectAst::WaitAnimation
        | EffectAst::Message { .. }
        | EffectAst::Scene(_) => {
            return Err(message(
                "presentation effects require the shared ordered-effect contract",
            ));
        }
    })
}

fn parse_camera_rule_effect_line(line: &str) -> Result<Option<Puzzle3CameraEffect>, ParseError3> {
    if line.trim() == "reset_camera" {
        return Ok(Some(Puzzle3CameraEffect::Reset));
    }
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let ["set", name, "=", value] = tokens.as_slice() else {
        return Ok(None);
    };
    match *name {
        "yaw" => Ok(Some(Puzzle3CameraEffect::SetYaw(parse_degrees_setting(
            value, name,
        )?))),
        "pitch" => Ok(Some(Puzzle3CameraEffect::SetPitch(parse_degrees_setting(
            value, name,
        )?))),
        "roll" => Ok(Some(Puzzle3CameraEffect::SetRoll(parse_degrees_setting(
            value, name,
        )?))),
        "zoom" => Ok(Some(Puzzle3CameraEffect::SetZoom(
            parse_zoom_milli_setting(value, name)?,
        ))),
        _ => Err(message(format!("unknown 3D view variable: {name}"))),
    }
}

fn parse_line_orientation(prefix: &str) -> Result<LineOrientation3, ParseError3> {
    if let Some(direction) = Direction3::by_name(prefix) {
        return Ok(LineOrientation3::Direction(direction));
    }
    let set = parse_direction_set(prefix)?;
    Ok(LineOrientation3::DirectionSet(set))
}

fn parse_frame_orientation(prefix: &str) -> Result<FrameOrientation3, ParseError3> {
    match prefix {
        "frames" => return Ok(FrameOrientation3::FrameSet(FrameSet3::Frames)),
        "canonical" => return Ok(FrameOrientation3::FrameSet(FrameSet3::Canonical)),
        "mirrored" => return Ok(FrameOrientation3::FrameSet(FrameSet3::Mirrored)),
        _ => {}
    }
    let prefix = prefix.trim();
    let body = if prefix.starts_with('(') || prefix.ends_with(')') {
        prefix
            .strip_prefix('(')
            .and_then(|prefix| prefix.strip_suffix(')'))
            .ok_or_else(|| message("frame3 orientation has unmatched parentheses"))?
    } else {
        prefix
    };
    let parts = crate::frame3_literal::split_frame3_components(body).map_err(message)?;
    let expr = if parts.len() == 2 {
        FrameExpr3::from_two(parse_frame_slot(parts[0])?, parse_frame_slot(parts[1])?)
    } else {
        FrameExpr3::new(
            parse_frame_slot(parts[0])?,
            parse_frame_slot(parts[1])?,
            parse_frame_slot(parts[2])?,
        )
    };
    let frames = expr.expand();
    if frames.len() == 1 {
        Ok(FrameOrientation3::Frame(frames[0]))
    } else {
        Ok(FrameOrientation3::Frames(frames))
    }
}

fn parse_frame_slot(value: &str) -> Result<FrameSlot3, ParseError3> {
    if value == "_" {
        return Ok(FrameSlot3::CompleteCanonical);
    }
    if let Some(direction) = Direction3::by_name(value) {
        return Ok(FrameSlot3::Direction(direction));
    }
    Ok(FrameSlot3::DirectionSet(parse_direction_set(value)?))
}

fn parse_direction_set(value: &str) -> Result<DirectionSet3, ParseError3> {
    match value {
        "directions" => Ok(DirectionSet3::Directions),
        "horizontal" => Ok(DirectionSet3::Horizontal),
        "vertical" => Ok(DirectionSet3::Vertical),
        _ => Err(message(format!("unknown direction set: {value}"))),
    }
}

fn lower_line_patterns(
    catalog: &crate::Catalog,
    orientation: LineOrientation3,
    pattern: &puzzle_authoring::UnresolvedPatternSyntax,
    _line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let pattern =
        materialize_line_pattern_with_gaps3(pattern, catalog).map_err(parse_error_message)?;
    validate_line_null_pattern(&pattern)?;
    let rule = LineRuleTemplate3::once(orientation, pattern.lower(), Vec::new());
    lower_line_rule_template3(catalog, &rule).map_err(parse_error_message)
}

fn parse_error_message(error: ParseError3) -> String {
    diagnostic_message(&error)
}

fn lower_line_rewrite(
    catalog: &crate::Catalog,
    orientation: LineOrientation3,
    lhs: &puzzle_authoring::UnresolvedPatternSyntax,
    rhs: &puzzle_authoring::UnresolvedPatternSyntax,
    _line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let before = materialize_line_pattern_with_gaps3(lhs, catalog).map_err(parse_error_message)?;
    let after = materialize_line_pattern_with_gaps3(rhs, catalog).map_err(parse_error_message)?;
    validate_line_null_rewrite(&before, &after)?;
    if before.gap_count != after.gap_count {
        return Err("line rewrite sides must contain the same number of ... gaps".to_string());
    }
    let mut rules = Vec::new();
    for (before, after) in expand_line_movement_mark_sets3(&before, &after) {
        let writes = infer_line_writes_from_patterns(&before, &after);
        let rule = LineRuleTemplate3::once(
            orientation.clone(),
            before.lower(),
            lower_line_writes(&writes),
        );
        rules.extend(lower_line_rule_template3(catalog, &rule).map_err(parse_error_message)?);
    }
    Ok(rules)
}

fn lower_input_line_rewrite(
    catalog: &crate::Catalog,
    orientation: LineOrientation3,
    lhs: &puzzle_authoring::UnresolvedPatternSyntax,
    rhs: &puzzle_authoring::UnresolvedPatternSyntax,
    _line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let before = materialize_line_pattern_with_gaps3(lhs, catalog).map_err(parse_error_message)?;
    let after = materialize_line_pattern_with_gaps3(rhs, catalog).map_err(parse_error_message)?;
    validate_line_null_rewrite(&before, &after)?;
    if before.gap_count != after.gap_count {
        return Err("line rewrite sides must contain the same number of ... gaps".to_string());
    }
    let mut rules = Vec::new();
    for (before, after) in expand_line_movement_mark_sets3(&before, &after) {
        let writes = infer_line_writes_from_patterns(&before, &after);
        for direction in directions_for_line_orientation(orientation.clone()) {
            let rule = LineRuleTemplate3::once(
                LineOrientation3::Direction(direction),
                before.lower(),
                lower_line_writes(&writes),
            );
            let input = input_for_direction(direction);
            let mut lowered =
                lower_line_rule_template3(catalog, &rule).map_err(parse_error_message)?;
            for rule in &mut lowered {
                rule.guards.push(Guard3::InputIs(input));
            }
            rules.extend(lowered);
        }
    }
    Ok(rules)
}

fn directions_for_line_orientation(orientation: LineOrientation3) -> Vec<Direction3> {
    match orientation {
        LineOrientation3::Direction(direction) => vec![direction],
        LineOrientation3::DirectionSet(set) => set.directions(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MarkSetBinding3 {
    key: String,
    values: &'static [&'static str],
}

fn expand_line_movement_mark_sets3(
    before: &LinePatternWithGaps3,
    after: &LinePatternWithGaps3,
) -> Vec<(LinePatternWithGaps3, LinePatternWithGaps3)> {
    let mut bindings = Vec::<MarkSetBinding3>::new();
    collect_line_movement_mark_set_bindings3(before, &mut bindings);
    collect_line_movement_mark_set_bindings3(after, &mut bindings);
    dedup_line_mark_set_bindings3(&mut bindings);

    if bindings.is_empty() {
        return vec![(before.clone(), after.clone())];
    }

    let mut assignments = Vec::<HashMap<String, String>>::new();
    expand_line_mark_set_assignments3(&bindings, 0, &mut HashMap::new(), &mut assignments);
    assignments
        .into_iter()
        .map(|assignment| {
            (
                apply_line_movement_mark_set_assignment3(before, &assignment),
                apply_line_movement_mark_set_assignment3(after, &assignment),
            )
        })
        .collect()
}

fn collect_line_movement_mark_set_bindings3(
    pattern: &LinePatternWithGaps3,
    bindings: &mut Vec<MarkSetBinding3>,
) {
    let mut selector_counts = HashMap::<String, usize>::new();
    for (cell_index, cell) in pattern.cells.iter().enumerate() {
        for selector in &cell.require {
            let ordinal = *selector_counts.get(&selector.token()).unwrap_or(&0);
            selector_counts.insert(selector.token(), ordinal + 1);
            collect_selector_mark_set_bindings3(
                selector,
                &format!("cell:{cell_index}:require:{}:{ordinal}", selector.token()),
                bindings,
            );
        }
        for selector in &cell.forbid {
            let ordinal = *selector_counts.get(&selector.token()).unwrap_or(&0);
            selector_counts.insert(selector.token(), ordinal + 1);
            collect_selector_mark_set_bindings3(
                selector,
                &format!("cell:{cell_index}:forbid:{}:{ordinal}", selector.token()),
                bindings,
            );
        }
    }
}

fn collect_selector_mark_set_bindings3(
    selector: &ObjectSelector3,
    anchor: &str,
    bindings: &mut Vec<MarkSetBinding3>,
) {
    match selector {
        ObjectSelector3::Labeled { selector, .. } => {
            collect_selector_mark_set_bindings3(selector, anchor, bindings);
        }
        ObjectSelector3::WithMark { selector, mark } => {
            collect_selector_mark_set_bindings3(selector, anchor, bindings);
            for (mark_index, mark) in mark.iter().enumerate() {
                let Some(value) = mark.value.as_deref() else {
                    continue;
                };
                let Some(values) = line_movement_mark_set_values3(value) else {
                    continue;
                };
                bindings.push(MarkSetBinding3 {
                    key: format!("{anchor}:mark:{mark_index}:{value}"),
                    values,
                });
            }
        }
        ObjectSelector3::Object(_)
        | ObjectSelector3::Group(_)
        | ObjectSelector3::Any
        | ObjectSelector3::Variant { .. } => {}
    }
}

fn dedup_line_mark_set_bindings3(bindings: &mut Vec<MarkSetBinding3>) {
    let mut deduped = Vec::with_capacity(bindings.len());
    for binding in bindings.drain(..) {
        if !deduped
            .iter()
            .any(|existing: &MarkSetBinding3| existing.key == binding.key)
        {
            deduped.push(binding);
        }
    }
    *bindings = deduped;
}

fn expand_line_mark_set_assignments3(
    bindings: &[MarkSetBinding3],
    index: usize,
    current: &mut HashMap<String, String>,
    out: &mut Vec<HashMap<String, String>>,
) {
    if index == bindings.len() {
        out.push(current.clone());
        return;
    }
    let binding = &bindings[index];
    for value in binding.values {
        current.insert(binding.key.clone(), (*value).to_string());
        expand_line_mark_set_assignments3(bindings, index + 1, current, out);
    }
    current.remove(&binding.key);
}

fn apply_line_movement_mark_set_assignment3(
    pattern: &LinePatternWithGaps3,
    assignment: &HashMap<String, String>,
) -> LinePatternWithGaps3 {
    let mut pattern = pattern.clone();
    let mut selector_counts = HashMap::<String, usize>::new();
    for (cell_index, cell) in pattern.cells.iter_mut().enumerate() {
        for selector in &mut cell.require {
            let token = selector.token();
            let ordinal = *selector_counts.get(&token).unwrap_or(&0);
            selector_counts.insert(token.clone(), ordinal + 1);
            apply_selector_mark_set_assignment3(
                selector,
                &format!("cell:{cell_index}:require:{token}:{ordinal}"),
                assignment,
            );
        }
        for selector in &mut cell.forbid {
            let token = selector.token();
            let ordinal = *selector_counts.get(&token).unwrap_or(&0);
            selector_counts.insert(token.clone(), ordinal + 1);
            apply_selector_mark_set_assignment3(
                selector,
                &format!("cell:{cell_index}:forbid:{token}:{ordinal}"),
                assignment,
            );
        }
    }
    pattern
}

fn apply_selector_mark_set_assignment3(
    selector: &mut ObjectSelector3,
    anchor: &str,
    assignment: &HashMap<String, String>,
) {
    match selector {
        ObjectSelector3::Labeled { selector, .. } => {
            apply_selector_mark_set_assignment3(selector, anchor, assignment);
        }
        ObjectSelector3::WithMark { selector, mark } => {
            apply_selector_mark_set_assignment3(selector, anchor, assignment);
            for (mark_index, mark) in mark.iter_mut().enumerate() {
                let Some(value) = mark.value.as_deref() else {
                    continue;
                };
                if line_movement_mark_set_values3(value).is_none() {
                    continue;
                }
                let key = format!("{anchor}:mark:{mark_index}:{value}");
                if let Some(concrete) = assignment.get(&key) {
                    mark.value = Some(concrete.clone());
                }
            }
        }
        ObjectSelector3::Object(_)
        | ObjectSelector3::Group(_)
        | ObjectSelector3::Any
        | ObjectSelector3::Variant { .. } => {}
    }
}

fn line_movement_mark_set_values3(value: &str) -> Option<&'static [&'static str]> {
    match value {
        "horizontal" | "vertical" => puzzle_authoring::movement_mark_set_values(value, 3),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LinePatternWithGaps3 {
    cells: Vec<LineCellWithGapStep3>,
    gap_count: u16,
}

impl LinePatternWithGaps3 {
    fn lower(&self) -> LinePatternTemplate3 {
        LinePatternTemplate3::new(
            self.cells
                .iter()
                .map(|cell| LineMatchCellTemplate3 {
                    step: cell.step.clone(),
                    require_null: cell.require_null,
                    require: cell.require.clone(),
                    forbid: cell.forbid.clone(),
                    require_cell_mark: cell.require_cell_mark.clone(),
                    forbid_cell_mark: cell.forbid_cell_mark.clone(),
                })
                .collect(),
        )
        .with_gap_count(self.gap_count)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LineCellWithGapStep3 {
    step: LineOffsetTemplate3,
    require_null: bool,
    require: Vec<ObjectSelector3>,
    forbid: Vec<ObjectSelector3>,
    require_cell_mark: Vec<SelectorMark3>,
    forbid_cell_mark: Vec<SelectorMark3>,
}

fn materialize_line_pattern_with_gaps3(
    pattern: &puzzle_authoring::UnresolvedPatternSyntax,
    catalog: &crate::Catalog,
) -> Result<LinePatternWithGaps3, ParseError3> {
    let [component] = pattern.components.as_slice() else {
        return Err(message(
            "3D line patterns require exactly one pattern block",
        ));
    };
    let [puzzle_authoring::UnresolvedPatternLineSyntax::Cells(parts)] = component.lines.as_slice()
    else {
        return Err(message(
            "line patterns must contain exactly one non-blank line",
        ));
    };
    let mut cells = Vec::new();
    let mut visible_step = 0_i16;
    let mut gap_count = 0_u16;
    for part in parts {
        if matches!(
            part,
            puzzle_authoring::UnresolvedPatternPartSyntax::Ellipsis
        ) {
            gap_count = gap_count
                .checked_add(1)
                .ok_or_else(|| message("too many 3D line gaps"))?;
            continue;
        }
        let puzzle_authoring::UnresolvedPatternPartSyntax::Cell(cell) = part else {
            unreachable!()
        };
        let parsed = materialize_cell3(cell, catalog)?;
        cells.push(LineCellWithGapStep3 {
            step: LineOffsetTemplate3 {
                base: visible_step,
                gap_terms: (0..gap_count).collect(),
            },
            require_null: parsed.require_null,
            require: parsed.require,
            forbid: parsed.forbid,
            require_cell_mark: parsed.require_cell_mark,
            forbid_cell_mark: parsed.forbid_cell_mark,
        });
        visible_step = visible_step
            .checked_add(1)
            .ok_or_else(|| message("3D line pattern is too long"))?;
    }
    Ok(LinePatternWithGaps3 { cells, gap_count })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LineWriteWithGapStep3 {
    Add {
        to: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    Remove {
        from: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    Move {
        from: LineOffsetTemplate3,
        to: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    SetMark {
        at: LineOffsetTemplate3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
    RemoveMark {
        at: LineOffsetTemplate3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
}

fn infer_line_writes_from_patterns(
    before: &LinePatternWithGaps3,
    after: &LinePatternWithGaps3,
) -> Vec<LineWriteWithGapStep3> {
    let before = puzzle_authoring::selector_rewrite_occurrences(
        before
            .cells
            .iter()
            .map(|cell| (cell.step.clone(), cell.require.as_slice())),
    );
    let after = puzzle_authoring::selector_rewrite_occurrences(
        after
            .cells
            .iter()
            .map(|cell| (cell.step.clone(), cell.require.as_slice())),
    );
    puzzle_authoring::diff_rewrite_occurrences(&before, &after, |left, right| left == right)
        .into_iter()
        .map(|delta| match delta {
            puzzle_authoring::RewriteOccurrenceDelta::Add { at, subject } => {
                LineWriteWithGapStep3::Add {
                    to: at,
                    object: subject,
                }
            }
            puzzle_authoring::RewriteOccurrenceDelta::Remove { at, subject } => {
                LineWriteWithGapStep3::Remove {
                    from: at,
                    object: subject,
                }
            }
            puzzle_authoring::RewriteOccurrenceDelta::Move { from, to, subject } => {
                LineWriteWithGapStep3::Move {
                    from,
                    to,
                    object: subject,
                }
            }
            puzzle_authoring::RewriteOccurrenceDelta::SetMark { at, subject, mark } => {
                LineWriteWithGapStep3::SetMark {
                    at,
                    object: subject,
                    mark,
                }
            }
            puzzle_authoring::RewriteOccurrenceDelta::RemoveMark { at, subject, mark } => {
                LineWriteWithGapStep3::RemoveMark {
                    at,
                    object: subject,
                    mark,
                }
            }
        })
        .collect()
}

fn validate_line_null_rewrite(
    before: &LinePatternWithGaps3,
    after: &LinePatternWithGaps3,
) -> Result<(), String> {
    validate_line_null_pattern(before)?;
    for after_cell in &after.cells {
        let before_null = before
            .cells
            .iter()
            .any(|before_cell| before_cell.step == after_cell.step && before_cell.require_null);
        puzzle_authoring::validate_null_rewrite_cell(
            before_null,
            after_cell.require_null,
            after_cell.require.is_empty() && after_cell.forbid.is_empty(),
        )
        .map_err(|error| error.message().to_string())?;
    }
    for before_cell in &before.cells {
        if !before_cell.require_null {
            continue;
        }
        let after_cell = after
            .cells
            .iter()
            .find(|after_cell| after_cell.step == before_cell.step);
        puzzle_authoring::validate_null_rewrite_cell(
            true,
            after_cell.is_some_and(|cell| cell.require_null),
            after_cell.is_none_or(|cell| cell.require.is_empty() && cell.forbid.is_empty()),
        )
        .map_err(|error| error.message().to_string())?;
    }
    Ok(())
}

fn validate_line_null_pattern(pattern: &LinePatternWithGaps3) -> Result<(), String> {
    puzzle_authoring::validate_null_pattern_cells(
        pattern.cells.iter().map(|cell| cell.require_null),
    )
    .map_err(|error| error.message().to_string())
}

fn lower_line_writes(writes: &[LineWriteWithGapStep3]) -> Vec<LineWriteOpTemplate3> {
    writes
        .iter()
        .map(|write| match write {
            LineWriteWithGapStep3::Add { to, object } => LineWriteOpTemplate3::Add {
                step: to.clone(),
                object: object.clone(),
            },
            LineWriteWithGapStep3::Remove { from, object } => LineWriteOpTemplate3::Remove {
                step: from.clone(),
                object: object.clone(),
            },
            LineWriteWithGapStep3::Move { from, to, object } => LineWriteOpTemplate3::Move {
                from_step: from.clone(),
                to_step: to.clone(),
                object: object.clone(),
            },
            LineWriteWithGapStep3::SetMark { at, object, mark } => LineWriteOpTemplate3::SetMark {
                step: at.clone(),
                object: object.clone(),
                mark: mark.clone(),
            },
            LineWriteWithGapStep3::RemoveMark { at, object, mark } => {
                LineWriteOpTemplate3::RemoveMark {
                    step: at.clone(),
                    object: object.clone(),
                    mark: mark.clone(),
                }
            }
        })
        .collect()
}

fn materialize_dense_pattern3(
    syntax: &puzzle_authoring::UnresolvedPatternSyntax,
    catalog: &crate::Catalog,
) -> Result<DensePattern3, ParseError3> {
    let [component] = syntax.components.as_slice() else {
        return Err(message(
            "3D dense patterns require exactly one pattern block",
        ));
    };
    let mut slices = Vec::new();
    let mut rows = Vec::new();
    for line in &component.lines {
        match line {
            puzzle_authoring::UnresolvedPatternLineSyntax::Blank => {
                if rows.is_empty() {
                    return Err(message("3D dense pattern contains an empty depth slice"));
                }
                slices.push(DenseSlice3::new(std::mem::take(&mut rows)));
            }
            puzzle_authoring::UnresolvedPatternLineSyntax::Cells(parts) => {
                let cells = parts
                    .iter()
                    .map(|part| {
                        let puzzle_authoring::UnresolvedPatternPartSyntax::Cell(cell) = part else {
                            return Err(message("ellipsis is only valid in line patterns"));
                        };
                        let parsed = materialize_cell3(cell, catalog)?;
                        Ok(DenseCell3 {
                            require_null: parsed.require_null,
                            require: parsed.require,
                            forbid: parsed.forbid,
                            require_cell_mark: parsed.require_cell_mark,
                            forbid_cell_mark: parsed.forbid_cell_mark,
                        })
                    })
                    .collect::<Result<Vec<_>, ParseError3>>()?;
                rows.push(DenseRow3::new(cells));
            }
        }
    }
    if rows.is_empty() {
        return Err(message("3D dense pattern contains an empty depth slice"));
    }
    slices.push(DenseSlice3::new(rows));
    let pattern = DensePattern3::new(slices);
    validate_dense_null_pattern(&pattern).map_err(message)?;
    Ok(pattern)
}

fn validate_dense_null_pattern(pattern: &DensePattern3) -> Result<(), String> {
    let cells = pattern
        .slices
        .iter()
        .flat_map(|slice| &slice.rows)
        .flat_map(|row| &row.cells)
        .collect::<Vec<_>>();
    puzzle_authoring::validate_null_pattern_cells(cells.into_iter().map(|cell| cell.require_null))
        .map_err(|error| error.message().to_string())
}

fn validate_dense_null_rewrite(
    before: &DensePattern3,
    after: &DensePattern3,
) -> Result<(), String> {
    for (depth, slice) in after.slices.iter().enumerate() {
        for (row_index, row) in slice.rows.iter().enumerate() {
            for (column, after_cell) in row.cells.iter().enumerate() {
                let before_cell = before
                    .slices
                    .get(depth)
                    .and_then(|slice| slice.rows.get(row_index))
                    .and_then(|row| row.cells.get(column));
                puzzle_authoring::validate_null_rewrite_cell(
                    before_cell.is_some_and(|cell| cell.require_null),
                    after_cell.require_null,
                    after_cell.require.is_empty() && after_cell.forbid.is_empty(),
                )
                .map_err(|error| error.message().to_string())?;
            }
        }
    }
    for (depth, slice) in before.slices.iter().enumerate() {
        for (row_index, row) in slice.rows.iter().enumerate() {
            for (column, before_cell) in row.cells.iter().enumerate() {
                if !before_cell.require_null {
                    continue;
                }
                let after_cell = after
                    .slices
                    .get(depth)
                    .and_then(|slice| slice.rows.get(row_index))
                    .and_then(|row| row.cells.get(column));
                puzzle_authoring::validate_null_rewrite_cell(
                    true,
                    after_cell.is_some_and(|cell| cell.require_null),
                    after_cell.is_none_or(|cell| cell.require.is_empty() && cell.forbid.is_empty()),
                )
                .map_err(|error| error.message().to_string())?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedCell3 {
    require_null: bool,
    require: Vec<ObjectSelector3>,
    forbid: Vec<ObjectSelector3>,
    require_cell_mark: Vec<SelectorMark3>,
    forbid_cell_mark: Vec<SelectorMark3>,
}

fn materialize_cell3(
    cell: &puzzle_authoring::UnresolvedCellSyntax,
    catalog: &crate::Catalog,
) -> Result<ParsedCell3, ParseError3> {
    if cell.keep {
        return Err(message(
            "3D pattern materialization received an unresolved `=` cell",
        ));
    }
    let require_null = cell.require_null;
    let mut require = Vec::new();
    let mut forbid = Vec::new();
    let mut require_cell_mark = Vec::new();
    let mut forbid_cell_mark = Vec::new();
    for subject in &cell.require {
        match subject {
            puzzle_authoring::UnresolvedCellSubjectSyntax::Selector(selector) => {
                require.push(resolve_selector_syntax3(selector, catalog)?);
            }
            puzzle_authoring::UnresolvedCellSubjectSyntax::CellMarks(marks) => {
                require_cell_mark.extend(marks.clone());
            }
        }
    }
    for subject in &cell.forbid {
        match subject {
            puzzle_authoring::UnresolvedCellSubjectSyntax::Selector(selector) => {
                forbid.push(resolve_selector_syntax3(selector, catalog)?);
            }
            puzzle_authoring::UnresolvedCellSubjectSyntax::CellMarks(marks) => {
                forbid_cell_mark.extend(marks.clone());
            }
        }
    }
    Ok(ParsedCell3 {
        require_null,
        require,
        forbid,
        require_cell_mark,
        forbid_cell_mark,
    })
}

fn infer_dense_writes(
    lhs: &puzzle_authoring::UnresolvedPatternSyntax,
    rhs: &puzzle_authoring::UnresolvedPatternSyntax,
    catalog: &crate::Catalog,
) -> Result<Vec<LocalWriteOpTemplate3>, ParseError3> {
    let before_pattern = materialize_dense_pattern3(lhs, catalog)?;
    let after_pattern = materialize_dense_pattern3(rhs, catalog)?;
    validate_dense_null_rewrite(&before_pattern, &after_pattern).map_err(message)?;
    let before = dense_rewrite_occurrences3(&before_pattern);
    let after = dense_rewrite_occurrences3(&after_pattern);
    Ok(
        puzzle_authoring::diff_rewrite_occurrences(&before, &after, |left, right| left == right)
            .into_iter()
            .map(|delta| match delta {
                puzzle_authoring::RewriteOccurrenceDelta::Add { at, subject } => {
                    LocalWriteOpTemplate3::Add {
                        offset: at,
                        object: subject,
                    }
                }
                puzzle_authoring::RewriteOccurrenceDelta::Remove { at, subject } => {
                    LocalWriteOpTemplate3::Remove {
                        offset: at,
                        object: subject,
                    }
                }
                puzzle_authoring::RewriteOccurrenceDelta::Move { from, to, subject } => {
                    LocalWriteOpTemplate3::Move {
                        from_offset: from,
                        to_offset: to,
                        object: subject,
                    }
                }
                puzzle_authoring::RewriteOccurrenceDelta::SetMark { at, subject, mark } => {
                    LocalWriteOpTemplate3::SetMark {
                        offset: at,
                        object: subject,
                        mark,
                    }
                }
                puzzle_authoring::RewriteOccurrenceDelta::RemoveMark { at, subject, mark } => {
                    LocalWriteOpTemplate3::RemoveMark {
                        offset: at,
                        object: subject,
                        mark,
                    }
                }
            })
            .collect(),
    )
}

fn dense_rewrite_occurrences3(
    pattern: &DensePattern3,
) -> Vec<puzzle_authoring::RewriteOccurrence<(String, usize), Delta3, ObjectSelector3, SelectorMark3>>
{
    puzzle_authoring::selector_rewrite_occurrences(pattern.slices.iter().enumerate().flat_map(
        |(depth, slice)| {
            slice
                .rows
                .iter()
                .enumerate()
                .flat_map(move |(row, dense_row)| {
                    dense_row
                        .cells
                        .iter()
                        .enumerate()
                        .map(move |(column, cell)| {
                            (
                                Delta3::new(column as i16, row as i16, depth as i16),
                                cell.require.as_slice(),
                            )
                        })
                })
        },
    ))
}

fn resolve_selector_syntax3(
    syntax: &puzzle_authoring::SelectorSyntax,
    catalog: &crate::Catalog,
) -> Result<ObjectSelector3, ParseError3> {
    let selector = syntax.selector.as_str();
    let mark = syntax.marks.clone();
    let occurrence_label = syntax.occurrence_label.clone();
    let mut tags = syntax.tags.clone();
    for part in &mut tags {
        if part.contains(',') && !(part.starts_with('(') && part.ends_with(')')) {
            return Err(message(
                "frame3 object slot must be parenthesized: Object:(primary, secondary)",
            ));
        }
        if part.starts_with('(') && part.ends_with(')') {
            match crate::frame3_literal::parse_frame3_domain(part).map_err(message)? {
                Some(values) if values.len() == 1 => *part = values[0].clone(),
                Some(_) => unreachable!("one parenthesized frame3 literal yields one value"),
                None => {}
            }
        }
    }
    let parsed = if syntax.base == "*" && tags.is_empty() {
        ObjectSelector3::any()
    } else if !tags.is_empty() {
        ObjectSelector3::variant(
            &syntax.base,
            tags.iter()
                .map(|part| {
                    if part == "*" {
                        SelectorTag3::any()
                    } else {
                        SelectorTag3::value(part)
                    }
                })
                .collect(),
        )
    } else {
        if catalog.object_schemas.contains_key(&syntax.base) {
            return Err(message(format!(
                "variant selector must use explicit tags: {selector}"
            )));
        }
        if catalog.object_groups.contains_key(&syntax.base) {
            ObjectSelector3::group(&syntax.base)
        } else {
            ObjectSelector3::object(&syntax.base)
        }
    };
    let parsed = match occurrence_label {
        Some(label) => ObjectSelector3::labeled(format!("{selector}#{label}"), parsed),
        None => parsed,
    };
    Ok(ObjectSelector3::with_mark(parsed, mark))
}

fn default_inputs() -> Vec<InputDef3> {
    vec![
        InputDef3::directional(InputId(0), "left", Direction3::LEFT),
        InputDef3::directional(InputId(1), "right", Direction3::RIGHT),
        InputDef3::directional(InputId(2), "up", Direction3::UP),
        InputDef3::directional(InputId(3), "down", Direction3::DOWN),
        InputDef3::directional(InputId(4), "front", Direction3::FORWARD),
        InputDef3::directional(InputId(5), "back", Direction3::BACKWARD),
    ]
}

fn canonical_input_name3(name: &str) -> &str {
    puzzle_authoring::canonical_3d_movement_direction_name(name)
}

fn inputs_from_specs(specs: Vec<InputSpec3>) -> Result<Vec<InputDef3>, ParseError3> {
    if specs.is_empty() {
        return Ok(default_inputs());
    }

    let defaults = default_inputs();
    let mut next_id = defaults
        .iter()
        .map(|input| input.id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut inputs = Vec::new();
    for spec in specs {
        let canonical_name = canonical_input_name3(&spec.name);
        if inputs
            .iter()
            .any(|input: &InputDef3| input.name == canonical_name)
        {
            return Err(message(format!("duplicate input: {}", spec.name)));
        }
        let input =
            if let Some(default) = defaults.iter().find(|input| input.name == canonical_name) {
                default.clone().with_keys(spec.keys)
            } else {
                let id = InputId(next_id);
                next_id = next_id.saturating_add(1);
                InputDef3::action(id, spec.name).with_keys(spec.keys)
            };
        inputs.push(input);
    }
    Ok(inputs)
}

fn message(message: impl Into<String>) -> ParseError3 {
    DiagnosticReport::error(message)
}

fn diagnostic_message(report: &DiagnosticReport) -> String {
    report
        .diagnostics()
        .first()
        .map(|diagnostic| diagnostic.message.clone())
        .unwrap_or_else(|| "empty diagnostic report".to_string())
}

fn diagnostic_report_error3(report: DiagnosticReport) -> ParseError3 {
    report
}

fn message_at_line(message: impl Into<String>, source_line: &str) -> ParseError3 {
    message_at_source_line(message, source_line)
}

fn message_at_source_line(message: impl Into<String>, source_line: &str) -> ParseError3 {
    DiagnosticReport::error_at_line(message, source_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puzzle3_lowers_shared_nested_rule_syntax() {
        let parsed = parse_puzzle3d(
            r#"
puzzle board {
dimension = 3
slots {
actor = TEN:horizontal
}
rules {
input [ TEN:horizontal ] -> [ > TEN:horizontal ]
for h in horizontal {
if input == h {
[ TEN:horizontal ] -> [ TEN:h ]
}
}
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.game.program().len(), 5);
    }

    #[test]
    fn puzzle3_line_patterns_consume_shared_null_cell_syntax() {
        let catalog = crate::Catalog::for_dimension(crate::ModelDimension::Three);
        let pattern = |inner: &str| {
            puzzle_authoring::parse_unresolved_pattern_syntax(&format!("[{inner}]"))
                .expect("shared pattern syntax parses")
        };

        lower_line_rewrite(
            &catalog,
            LineOrientation3::Direction(Direction3::DOWN),
            &pattern("| null"),
            &pattern("|"),
            0,
        )
        .expect("null boundary detection lowers in 3D");

        let error = lower_line_rewrite(
            &catalog,
            LineOrientation3::Direction(Direction3::DOWN),
            &pattern("|"),
            &pattern("| null"),
            0,
        )
        .expect_err("RHS cannot introduce null");
        assert!(
            error.contains("`null` can only be matched on the before side"),
            "{error}"
        );

        let dense_error = infer_dense_writes(&pattern("|"), &pattern("| null"), &catalog)
            .expect_err("dense RHS cannot introduce null")
            .to_string();
        assert!(
            dense_error.contains("`null` can only be matched on the before side"),
            "{dense_error}"
        );
    }

    #[test]
    fn puzzle3_level_dash_separates_height_slices() {
        let rows = ["AB", "CD", "-", "EF", "GH"].map(str::to_string);

        let slices = super::split_level_slices(&rows).expect("split 3D level slices");

        assert_eq!(
            slices,
            vec![
                vec!["AB".to_string(), "CD".to_string()],
                vec!["EF".to_string(), "GH".to_string()],
            ]
        );
    }

    #[test]
    fn puzzle3_level_dash_requires_slices_on_both_sides() {
        for (rows, expected) in [
            (
                vec!["-".to_string(), "AB".to_string()],
                "preceding ASCII slice",
            ),
            (
                vec!["AB".to_string(), "-".to_string()],
                "following ASCII slice",
            ),
        ] {
            let error = super::split_level_slices(&rows).expect_err("reject bare separator");
            let message = error.to_string();
            assert!(message.contains(expected), "{message}");
        }
    }

    #[test]
    fn puzzle3_sprite_animation_preserves_frames_and_timing() {
        let parsed = super::parse_puzzle3d(
            r#"
sprites {
Pulse {
colors = #ff0000 transparent
duration = 240ms
frame_duration = 120ms
shape = {
0.
..
>
.0
..
}
}
}

puzzle board {
dimension = 3
slots {
objects = Pulse
}
rules {
}
}

levels demo of board {
legend {
. = empty
P = Pulse
}
level "start" {
P
}
}
"#,
        )
        .expect("parse animated 3D sprite");

        let fixture: serde_json::Value = serde_json::from_str(
            &crate::export_visual_fixture_json(&parsed).expect("export animated 3D fixture"),
        )
        .expect("animated 3D fixture JSON");
        assert_eq!(
            fixture["sprites"]["Pulse"]["frames"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(fixture["sprites"]["Pulse"]["durationMs"], 240);
        assert_eq!(fixture["sprites"]["Pulse"]["frameDurationMs"], 120);

        let sprites = parsed.sprite_set.expect("sprites");
        let sprite = sprites.sprite("Pulse").expect("Pulse sprite");
        assert_eq!(sprite.frames.len(), 2);
        assert_eq!(sprite.duration_ms, Some(240));
        assert_eq!(sprite.frame_duration_ms, Some(120));
        assert_ne!(sprite.frames[0].slices, sprite.frames[1].slices);
        assert_eq!(sprite.frames[0].size, sprite.frames[1].size);
    }

    #[test]
    fn puzzle3_sprite_animation_requires_timing() {
        let error = super::parse_puzzle3d(
            r#"
sprites {
Pulse {
colors = #ff0000
shape = {
0
>
0
}
}
}

puzzle board {
dimension = 3
slots {
objects = Pulse
}
rules {
}
}
"#,
        )
        .expect_err("animated 3D sprite without timing must fail");

        let message = error.to_string();
        assert!(
            message.contains("sprite Pulse: sprite animation requires duration or frame_duration"),
            "{message}"
        );
    }

    #[test]
    fn puzzle3_accepts_the_shared_unbraced_sprite_surface() {
        let parsed = super::parse_puzzle3d(
            r#"
sprites {
Floor
#8fcf6f
0

Floor2 {
#0000000a
0
}
}

puzzle board {
dimension = 3
slots {
floor = Floor Floor2
}
rules {
}
}
"#,
        )
        .unwrap();

        let sprites = parsed.sprite_set.expect("sprites");
        assert_eq!(sprites.sprites.len(), 2);
        assert_eq!(sprites.sprites[0].name, "Floor");
        assert_eq!(sprites.sprites[1].name, "Floor2");
    }

    #[test]
    fn puzzle3_axisless_sprite_rotation_defaults_to_z_axis() {
        let syntax = crate::sprite_authoring::parse_sprite_node(
            Some("Arrow {"),
            &["rotate 90deg"].map(str::to_string),
        );

        let ops = super::parse_sprite_spatial_ops3(&syntax).unwrap();

        assert_eq!(
            ops,
            vec![crate::SpriteSpatialOp3::Rotate {
                space: crate::SpriteSpace3::World,
                axis: [0.0, 0.0, 1.0],
                degrees: 90.0,
            }]
        );
    }

    #[test]
    fn puzzle3_sprite_rotation_combines_from_with_an_explicit_axis() {
        let syntax = crate::sprite_authoring::parse_sprite_node(
            Some("Arrow {"),
            &["rotate local 90deg from 30deg around right"].map(str::to_string),
        );

        let ops = super::parse_sprite_spatial_ops3(&syntax).unwrap();

        assert_eq!(
            ops,
            vec![crate::SpriteSpatialOp3::Rotate {
                space: crate::SpriteSpace3::Local,
                axis: [1.0, 0.0, 0.0],
                degrees: 60.0,
            }]
        );
    }

    #[test]
    fn puzzle3_axisless_rotation_resolves_horizontal_sprite_bindings_from_front() {
        let parsed = super::parse_puzzle3d(
            r#"
sprites {
Arrow:horizontal {
colors = #ffffff
rotate horizontal from front
0
}
}

puzzle board {
dimension = 3
slots {
actor = Arrow:horizontal
}
rules {
}
}
"#,
        )
        .unwrap();
        let sprites = parsed.sprite_set.unwrap();

        for (name, degrees) in [
            ("Arrow:right", -90.0),
            ("Arrow:front", 0.0),
            ("Arrow:left", 90.0),
            ("Arrow:back", -180.0),
        ] {
            assert_eq!(
                sprites.sprite(name).unwrap().spatial_ops,
                vec![crate::SpriteSpatialOp3::Rotate {
                    space: crate::SpriteSpace3::World,
                    axis: [0.0, 0.0, 1.0],
                    degrees,
                }]
            );
        }
    }
}
