struct ProgramLowerer<'a> {
    definitions: HashMap<String, RuleDefinitionAst>,
    object_layers: &'a HashMap<ObjectId, LayerId>,
    input_names: &'a HashMap<String, InputId>,
    variable_names: &'a HashMap<String, VariableId>,
    constant_variables: &'a [VariableId],
    condition_names: &'a HashMap<String, ConditionId>,
    mark_names: &'a HashMap<String, MarkDef>,
    model_sound_triggers: &'a [ModelSoundTrigger],
    visual_names: &'a HashSet<String>,
    animation_visual_names: &'a HashSet<String>,
    animation: &'a AnimationDef,
    direction_variant_pairs: &'a HashSet<(ObjectId, ObjectId)>,
    value_sets: &'a HashMap<String, Vec<String>>,
    maps: &'a HashMap<String, ValueMap>,
    directions: &'a [OrientationEnvironment],
    next_rule_id: u16,
    rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
    rule_debug_info: HashMap<RuleId, RuleDebugInfo>,
}

#[derive(Clone, Debug, Default)]
struct StatementLoweringContext {
    guards: Vec<CanonicalGuard>,
    call_stack: Vec<String>,
    application: RuleApplication,
    application_fixed: bool,
    orientation: Option<OrientationExpr>,
    input_allowed: bool,
    input_forbidden_context: Option<&'static str>,
    local_definitions: Vec<HashMap<String, RuleDefinitionAst>>,
}

#[derive(Clone, Debug)]
struct ResolvedRoutineDefinition {
    definition: RuleDefinitionAst,
    is_local: bool,
}

struct LoweredPrograms {
    main: Vec<CanonicalRuleStep>,
    level_start: Option<Vec<CanonicalRuleStep>>,
    level_clear: Option<Vec<CanonicalRuleStep>>,
    last_level_clear: Option<Vec<CanonicalRuleStep>>,
    level_starts: Vec<Option<Vec<CanonicalRuleStep>>>,
    level_clears: Vec<Option<Vec<CanonicalRuleStep>>>,
    level_programs: Vec<LoweredLevelProgram>,
    rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
    rule_debug_info: HashMap<RuleId, RuleDebugInfo>,
}

enum LoweredLevelProgram {
    Main,
    WithSurrounding {
        before: Vec<CanonicalRuleStep>,
        after: Vec<CanonicalRuleStep>,
    },
}

#[derive(Clone, Debug, Default)]
struct LoweredEffects {
    core: Vec<Effect>,
    ordered: Vec<RuleEffect>,
}

impl LoweredEffects {
    fn mark_external_observation(&mut self) {
        if !self.ordered.is_empty()
            && !self
                .core
                .iter()
                .any(|effect| matches!(effect, Effect::ObserveMatch))
        {
            self.core.push(Effect::ObserveMatch);
        }
    }
}

fn lower_programs(
    definitions: Vec<RuleDefinitionAst>,
    main_statements: Option<Vec<StatementAst>>,
    main_local_frame: Option<LocalFrame<ObjectId>>,
    level_start_statements: Option<Vec<StatementAst>>,
    level_start_local_frame: Option<LocalFrame<ObjectId>>,
    level_clear_statements: Option<Vec<StatementAst>>,
    level_clear_local_frame: Option<LocalFrame<ObjectId>>,
    last_level_clear_statements: Option<Vec<StatementAst>>,
    last_level_clear_local_frame: Option<LocalFrame<ObjectId>>,
    level_bodies: &[PreparedLevelBody],
    object_layers: &HashMap<ObjectId, LayerId>,
    input_names: &HashMap<String, InputId>,
    variable_names: &HashMap<String, VariableId>,
    constant_variables: &[VariableId],
    condition_names: &HashMap<String, ConditionId>,
    mark_names: &HashMap<String, MarkDef>,
    model_sound_triggers: &[ModelSoundTrigger],
    visual_names: &HashSet<String>,
    animation_visual_names: &HashSet<String>,
    animation: &AnimationDef,
    direction_variant_pairs: &HashSet<(ObjectId, ObjectId)>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    directions: &[OrientationEnvironment],
) -> Result<LoweredPrograms, DiagnosticReport> {
    let mut definitions_by_name = HashMap::new();
    for definition in definitions {
        if definitions_by_name
            .insert(definition.name.clone(), definition)
            .is_some()
        {
            return Err(DiagnosticReport::error(
                "duplicate routine definition".to_string(),
            ));
        }
    }
    let Some(main_statements) = main_statements else {
        return Err(DiagnosticReport::error("missing puzzle rules".to_string()));
    };
    let mut diagnostics = collect_program_reference_diagnostics(
        &definitions_by_name,
        &main_statements,
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        last_level_clear_statements.as_deref(),
        level_bodies,
    );
    dedup_diagnostics(&mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    let mut lowerer = ProgramLowerer {
        definitions: definitions_by_name,
        object_layers,
        input_names,
        variable_names,
        constant_variables,
        condition_names,
        mark_names,
        model_sound_triggers,
        visual_names,
        animation_visual_names,
        animation,
        direction_variant_pairs,
        value_sets,
        maps,
        directions,
        next_rule_id: 1,
        rule_animations: HashMap::new(),
        rule_effects: HashMap::new(),
        rule_debug_info: HashMap::new(),
    };
    let mut diagnostics = Vec::new();
    let mut context = StatementLoweringContext::default();
    context.input_allowed = true;
    let program = match lowerer.lower_statements(&main_statements, &context) {
        Ok(steps) => Some(wrap_program_local_frame(steps, main_local_frame)),
        Err(report) => {
            diagnostics.extend(report.into_diagnostics());
            None
        }
    };
    let level_start = if let Some(statements) = level_start_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_level_start");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(steps, level_start_local_frame)),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let level_clear = if let Some(statements) = level_clear_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_level_clear");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(steps, level_clear_local_frame)),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let last_level_clear = if let Some(statements) = last_level_clear_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_last_level_clear");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(
                steps,
                last_level_clear_local_frame,
            )),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let mut level_starts = Vec::with_capacity(level_bodies.len());
    let mut level_clears = Vec::with_capacity(level_bodies.len());
    let mut level_programs = Vec::with_capacity(level_bodies.len());
    for level in level_bodies {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = true;
        let before = match lowerer.lower_statements(&level.rules_before_statements, &context) {
            Ok(steps) => steps,
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                Vec::new()
            }
        };
        let after = match lowerer.lower_statements(&level.rules_after_statements, &context) {
            Ok(steps) => steps,
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                Vec::new()
            }
        };
        if before.is_empty() && after.is_empty() {
            level_programs.push(LoweredLevelProgram::Main);
        } else {
            level_programs.push(LoweredLevelProgram::WithSurrounding { before, after });
        }

        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("level on_level_start");
        level_starts.push(if level.level_start_statements.is_empty() {
            None
        } else {
            match lowerer.lower_statements(&level.level_start_statements, &context) {
                Ok(steps) => Some(steps),
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    None
                }
            }
        });

        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("level on_level_clear");
        level_clears.push(if level.level_clear_statements.is_empty() {
            None
        } else {
            match lowerer.lower_statements(&level.level_clear_statements, &context) {
                Ok(steps) => Some(steps),
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    None
                }
            }
        });
    }
    dedup_diagnostics(&mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    Ok(LoweredPrograms {
        main: program.expect("main program lowered when no diagnostics were reported"),
        level_start,
        level_clear,
        last_level_clear,
        level_starts,
        level_clears,
        level_programs,
        rule_animations: lowerer.rule_animations,
        rule_effects: lowerer.rule_effects,
        rule_debug_info: lowerer.rule_debug_info,
    })
}

fn dedup_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    let mut deduped = Vec::<Diagnostic>::new();
    for diagnostic in diagnostics.drain(..) {
        if !deduped.contains(&diagnostic) {
            deduped.push(diagnostic);
        }
    }
    *diagnostics = deduped;
}

fn collect_program_reference_diagnostics(
    definitions_by_name: &HashMap<String, RuleDefinitionAst>,
    main_statements: &[StatementAst],
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for definition in definitions_by_name.values() {
        collect_statement_reference_diagnostics(
            &definition.statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
    }
    collect_statement_reference_diagnostics(
        main_statements,
        definitions_by_name,
        &mut Vec::new(),
        &mut diagnostics,
    );
    for statements in [
        level_start_statements,
        level_clear_statements,
        last_level_clear_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_statement_reference_diagnostics(
            statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
    }
    for level in level_bodies {
        collect_statement_reference_diagnostics(
            &level.rules_before_statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
        collect_statement_reference_diagnostics(
            &level.rules_after_statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
        collect_statement_reference_diagnostics(
            &level.level_start_statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
        collect_statement_reference_diagnostics(
            &level.level_clear_statements,
            definitions_by_name,
            &mut Vec::new(),
            &mut diagnostics,
        );
    }
    diagnostics
}

fn collect_statement_reference_diagnostics(
    statements: &[StatementAst],
    definitions_by_name: &HashMap<String, RuleDefinitionAst>,
    local_scopes: &mut Vec<HashMap<String, RuleDefinitionAst>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let local_definitions = local_routine_definitions_with_diagnostics(statements, diagnostics);
    let has_local_definitions = !local_definitions.is_empty();
    if has_local_definitions {
        local_scopes.push(local_definitions);
    }

    for statement in statements {
        match statement {
            StatementAst::LocalRoutine { definition, .. } => {
                collect_statement_reference_diagnostics(
                    &definition.statements,
                    definitions_by_name,
                    local_scopes,
                    diagnostics,
                );
            }
            StatementAst::Call {
                name,
                source_line,
                source_line_number,
            } => {
                if !routine_definition_exists(name, definitions_by_name, local_scopes) {
                    diagnostics.push(diagnostic_at_source_line_number(
                        format!("unknown routine call: {name}"),
                        source_line,
                        *source_line_number,
                    ));
                }
            }
            StatementAst::Block { statements, .. } => {
                collect_statement_reference_diagnostics(
                    statements,
                    definitions_by_name,
                    local_scopes,
                    diagnostics,
                );
            }
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            }
            | StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_statement_reference_diagnostics(
                    then_statements,
                    definitions_by_name,
                    local_scopes,
                    diagnostics,
                );
                collect_statement_reference_diagnostics(
                    else_statements,
                    definitions_by_name,
                    local_scopes,
                    diagnostics,
                );
            }
            StatementAst::RepeatUntil { statements, .. } | StatementAst::Fix { statements, .. } => {
                collect_statement_reference_diagnostics(
                    statements,
                    definitions_by_name,
                    local_scopes,
                    diagnostics,
                );
            }
            StatementAst::Rewrite(rewrite) => {
                if let Some(name) = &rewrite.after_call {
                    if !routine_definition_exists(name, definitions_by_name, local_scopes) {
                        diagnostics.push(diagnostic_at_source_line_number(
                            format!("unknown routine call: {name}"),
                            &rewrite.source_line,
                            rewrite.source_line_number,
                        ));
                    }
                }
            }
            StatementAst::Effect { .. } => {}
        }
    }
    if has_local_definitions {
        local_scopes.pop();
    }
}

fn local_routine_definitions(statements: &[StatementAst]) -> HashMap<String, RuleDefinitionAst> {
    let mut definitions = HashMap::new();
    for statement in statements {
        if let StatementAst::LocalRoutine { definition, .. } = statement {
            definitions
                .entry(definition.name.clone())
                .or_insert_with(|| definition.clone());
        }
    }
    definitions
}

fn local_routine_definitions_with_diagnostics(
    statements: &[StatementAst],
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<String, RuleDefinitionAst> {
    let mut definitions = HashMap::new();
    for statement in statements {
        if let StatementAst::LocalRoutine {
            definition,
            source_line,
            source_line_number,
        } = statement
        {
            if definitions
                .insert(definition.name.clone(), definition.clone())
                .is_some()
            {
                diagnostics.push(diagnostic_at_source_line_number(
                    format!("duplicate local routine definition: {}", definition.name),
                    source_line,
                    *source_line_number,
                ));
            }
        }
    }
    definitions
}

fn routine_definition_exists(
    name: &str,
    definitions_by_name: &HashMap<String, RuleDefinitionAst>,
    local_scopes: &[HashMap<String, RuleDefinitionAst>],
) -> bool {
    local_scopes
        .iter()
        .rev()
        .any(|scope| scope.contains_key(name))
        || definitions_by_name.contains_key(name)
}

fn wrap_program_local_frame(
    steps: Vec<CanonicalRuleStep>,
    local_frame: Option<LocalFrame<ObjectId>>,
) -> Vec<CanonicalRuleStep> {
    match local_frame {
        Some(frame) => vec![CanonicalRuleStep::LocalFrame { frame, steps }],
        None => steps,
    }
}

fn input_dependency_error(
    context: &StatementLoweringContext,
    source_line: &str,
    source_line_number: Option<usize>,
) -> DiagnosticReport {
    let scope = context.input_forbidden_context.unwrap_or("this program");
    if source_line.trim().is_empty() {
        return DiagnosticReport::error(format!("{scope} cannot depend on input"));
    }
    report_at_source_line_number(
        format!("{scope} cannot depend on input"),
        source_line,
        source_line_number,
    )
}

fn diagnostic_at_source_line_number(
    message: impl Into<String>,
    source_line: &str,
    source_line_number: Option<usize>,
) -> Diagnostic {
    let diagnostic = Diagnostic::error(message);
    match source_line_number {
        Some(line) => diagnostic.with_source_line_number(source_line.to_string(), line),
        None => diagnostic.with_source_line(source_line.to_string()),
    }
}

fn report_at_source_line_number(
    message: impl Into<String>,
    source_line: &str,
    source_line_number: Option<usize>,
) -> DiagnosticReport {
    DiagnosticReport::from_diagnostic(diagnostic_at_source_line_number(
        message,
        source_line,
        source_line_number,
    ))
}

fn lower_condition_defs(
    definitions: Vec<ConditionDefinitionAst>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<Vec<CanonicalConditionDef>, DiagnosticReport> {
    definitions
        .into_iter()
        .map(|definition| {
            let kind = lower_condition_value_kind(
                &definition.kind,
                input_names,
                object_layers,
                mark_names,
                value_sets,
                maps,
                directions,
            )?;
            Ok(CanonicalConditionDef {
                id: definition.id,
                kind,
            })
        })
        .collect()
}

fn lower_condition_value_kind(
    kind: &ConditionValueAst,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    directions: &[OrientationEnvironment],
) -> Result<CanonicalConditionValueKind, DiagnosticReport> {
    match kind {
        ConditionValueAst::CountObjects(objects) => {
            Ok(CanonicalConditionValueKind::CountObjects(objects.clone()))
        }
        ConditionValueAst::ExistsObjects(objects) => {
            Ok(CanonicalConditionValueKind::ExistsObjects(objects.clone()))
        }
        ConditionValueAst::NoneObjects(objects) => {
            Ok(CanonicalConditionValueKind::NoneObjects(objects.clone()))
        }
        ConditionValueAst::CountMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::Count,
            input_names,
            object_layers,
            mark_names,
            value_sets,
            maps,
            directions,
        ),
        ConditionValueAst::ExistsMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::Exists,
            input_names,
            object_layers,
            mark_names,
            value_sets,
            maps,
            directions,
        ),
        ConditionValueAst::NoneMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::None,
            input_names,
            object_layers,
            mark_names,
            value_sets,
            maps,
            directions,
        ),
    }
}

#[derive(Clone, Copy)]
enum ConditionMatchKind {
    Count,
    Exists,
    None,
}

fn lower_condition_match_kind(
    condition_pattern: &ConditionPatternAst,
    kind: ConditionMatchKind,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    directions: &[OrientationEnvironment],
) -> Result<CanonicalConditionValueKind, DiagnosticReport> {
    if matches!(
        condition_pattern.orientation,
        OrientationExpr::Input | OrientationExpr::InputSet(_)
    ) {
        let patterns = lower_condition_input_patterns(
            condition_pattern,
            input_names,
            object_layers,
            mark_names,
            value_sets,
            maps,
            directions,
        )?;
        return Ok(match kind {
            ConditionMatchKind::Count => CanonicalConditionValueKind::CountInputMatches(patterns),
            ConditionMatchKind::Exists => CanonicalConditionValueKind::ExistsInputMatches(patterns),
            ConditionMatchKind::None => CanonicalConditionValueKind::NoneInputMatches(patterns),
        });
    }
    let patterns = lower_condition_patterns(
        condition_pattern,
        object_layers,
        mark_names,
        value_sets,
        maps,
        input_names,
        directions,
    )?;
    Ok(match kind {
        ConditionMatchKind::Count => CanonicalConditionValueKind::CountMatches(patterns),
        ConditionMatchKind::Exists => CanonicalConditionValueKind::ExistsMatches(patterns),
        ConditionMatchKind::None => CanonicalConditionValueKind::NoneMatches(patterns),
    })
}

fn lower_condition_patterns(
    condition_pattern: &ConditionPatternAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<Vec<CanonicalPattern>, DiagnosticReport> {
    let block = &condition_pattern.pattern;
    match &condition_pattern.orientation {
        OrientationExpr::Neutral => {
            if pattern_block_requires_implicit_cardinal_expansion(block, value_sets) {
                let implicit_directions =
                    implicit_spatial_directions(input_names, value_sets, directions)?;
                return lower_condition_patterns_for_directions(
                    block,
                    object_layers,
                    mark_names,
                    value_sets,
                    maps,
                    &implicit_directions,
                    true,
                );
            }
            lower_condition_patterns_for_directions(
                block,
                object_layers,
                mark_names,
                value_sets,
                maps,
                &[neutral_direction(directions)],
                false,
            )
        }
        OrientationExpr::Input => lower_condition_patterns_for_directions(
            block,
            object_layers,
            mark_names,
            value_sets,
            maps,
            directions,
            true,
        ),
        OrientationExpr::InputSet(axis) => {
            let directions =
                directions_for_orientation_name(axis, input_names, value_sets, directions)?
                    .ok_or_else(|| {
                        DiagnosticReport::error(format!("unknown input orientation set: {axis}"))
                    })?;
            lower_condition_patterns_for_directions(
                block,
                object_layers,
                mark_names,
                value_sets,
                maps,
                &directions,
                true,
            )
        }
        OrientationExpr::Fixed(direction_name) => {
            let directions = directions_for_orientation_name(
                &direction_name.0,
                input_names,
                value_sets,
                directions,
            )?
            .ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "unknown condition pattern orientation: {}",
                    direction_name.0
                ))
            })?;
            lower_condition_patterns_for_directions(
                block,
                object_layers,
                mark_names,
                value_sets,
                maps,
                &directions,
                true,
            )
        }
    }
}

fn lower_condition_patterns_for_directions(
    block: &PatternBlock,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    directions: &[OrientationEnvironment],
    direction_expanded: bool,
) -> Result<Vec<CanonicalPattern>, DiagnosticReport> {
    let mut patterns = Vec::new();
    for direction in directions {
        let (_, alternatives) = compile_before_after_blocks_for_direction(
            block,
            block,
            object_layers,
            mark_names,
            value_sets,
            maps,
            *direction,
            direction_expanded,
            "condition pattern",
            None,
        )?;
        patterns.extend(patterns_from_alternatives(
            &alternatives,
            &[*direction],
            direction_expanded,
            "condition pattern",
        )?);
    }
    Ok(patterns)
}

fn lower_condition_input_patterns(
    condition_pattern: &ConditionPatternAst,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    directions: &[OrientationEnvironment],
) -> Result<Vec<(InputId, CanonicalPattern)>, DiagnosticReport> {
    let block = &condition_pattern.pattern;
    let mut patterns = Vec::new();
    let input_directions = match &condition_pattern.orientation {
        OrientationExpr::Input => directions.to_vec(),
        OrientationExpr::InputSet(axis) => {
            directions_for_orientation_name(axis, input_names, value_sets, directions)?.ok_or_else(
                || DiagnosticReport::error(format!("unknown input orientation set: {axis}")),
            )?
        }
        OrientationExpr::Neutral | OrientationExpr::Fixed(_) => Vec::new(),
    };
    for direction in &input_directions {
        let (_, alternatives) = compile_before_after_blocks_for_direction(
            block,
            block,
            object_layers,
            mark_names,
            value_sets,
            maps,
            *direction,
            true,
            "condition pattern",
            None,
        )?;
        for pattern in
            patterns_from_alternatives(&alternatives, &[*direction], true, "condition pattern")?
        {
            patterns.push((direction.input, pattern));
        }
    }
    Ok(patterns)
}

fn directions_for_orientation_name(
    name: &str,
    input_names: &HashMap<String, InputId>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[OrientationEnvironment],
) -> Result<Option<Vec<OrientationEnvironment>>, DiagnosticReport> {
    let direct = direction_by_name(name, input_names, directions);
    if !direct.is_empty() {
        return Ok(Some(direct));
    }
    if name.contains(',') {
        let Some(domain) = directions.first().copied() else {
            return Err(DiagnosticReport::error(
                "frame orientation requires a spatial domain".to_string(),
            ));
        };
        return domain.expand_selector(name, InputId(0)).map(Some);
    }
    let Some(values) = value_sets.get(name) else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err(DiagnosticReport::error(format!(
            "empty orientation set: {name}"
        )));
    }
    let mut expanded = Vec::new();
    for value in values {
        let variants = direction_by_name(value, input_names, directions);
        if variants.is_empty() {
            return Err(DiagnosticReport::error(format!(
                "orientation set {name} contains non-direction value: {value}"
            )));
        }
        expanded.extend(variants);
    }
    Ok(Some(expanded))
}

fn implicit_spatial_directions(
    input_names: &HashMap<String, InputId>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[OrientationEnvironment],
) -> Result<Vec<OrientationEnvironment>, DiagnosticReport> {
    if !directions
        .first()
        .is_some_and(|direction| direction.dimension() == ModelDimension::Three)
    {
        return Ok(directions.to_vec());
    }
    directions_for_orientation_name("horizontal", input_names, value_sets, directions)?.ok_or_else(
        || {
            DiagnosticReport::error(
                "3D implicit spatial orientation requires horizontal".to_string(),
            )
        },
    )
}

fn patterns_from_alternatives(
    alternatives: &[RuleBodyAlternative],
    directions: &[OrientationEnvironment],
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<CanonicalPattern>, DiagnosticReport> {
    let mut patterns = Vec::new();
    for direction in directions {
        for alternative in alternatives {
            if !alternative.guards.is_empty() {
                return Err(DiagnosticReport::error(
                    "dynamic object selectors are not supported in condition patterns yet"
                        .to_string(),
                ));
            }
            patterns.push(pattern_from_alternative(
                alternative,
                *direction,
                direction_expanded,
                line,
            )?);
        }
    }
    Ok(patterns)
}

fn pattern_from_alternative(
    alternative: &RuleBodyAlternative,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<CanonicalPattern, DiagnosticReport> {
    let components = alternative
        .components
        .iter()
        .map(|component| {
            let cells = component
                .cells
                .iter()
                .map(|cell| {
                    Ok(CanonicalMatchCell {
                        offset: resolve_offset(
                            cell.offset.clone(),
                            direction,
                            direction_expanded,
                            line,
                        )?,
                        require_null: cell.require_null,
                        require_objects: cell.require_objects.clone(),
                        require_object_sets: cell.require_object_sets.clone(),
                        forbid_objects: cell.forbid_objects.clone(),
                        require_mark: resolve_mark_patterns(
                            cell.require_mark.clone(),
                            direction,
                            direction_expanded,
                            line,
                        )?,
                        require_object_set_mark: resolve_object_set_mark_patterns(
                            cell.require_object_set_mark.clone(),
                            direction,
                            direction_expanded,
                            line,
                        )?,
                        forbid_mark: resolve_mark_patterns(
                            cell.forbid_mark.clone(),
                            direction,
                            direction_expanded,
                            line,
                        )?,
                        forbid_object_set_mark: resolve_object_set_mark_patterns(
                            cell.forbid_object_set_mark.clone(),
                            direction,
                            direction_expanded,
                            line,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            Ok(CanonicalPatternComponent {
                cells,
                gap_count: component.gap_count,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    Ok(CanonicalPattern { components })
}

fn lower_goal_condition(
    description: String,
    condition: &ConditionAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    variable_names: &HashMap<String, VariableId>,
    condition_names: &HashMap<String, ConditionId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<CanonicalGoalCondition, DiagnosticReport> {
    Ok(CanonicalGoalCondition {
        description,
        expr: lower_goal_expr(
            condition,
            object_layers,
            variable_names,
            condition_names,
            mark_names,
            value_sets,
            maps,
            input_names,
            directions,
        )?,
    })
}

fn lower_query_definitions(
    definitions: &[QueryDefinitionAst],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<HashMap<String, CanonicalQueryExpr>, DiagnosticReport> {
    let context = QueryLoweringContext {
        object_names,
        object_schemas,
        maps,
        object_groups,
        variable_names,
        object_layers,
        mark_names,
        value_sets,
        input_names,
        directions,
    };
    crate::solver_surface::lower_query_definitions_with::<QueryLoweringAdapter2d, _>(
        definitions,
        &context,
    )
}

struct QueryLoweringContext<'a> {
    object_names: &'a HashMap<String, ObjectId>,
    object_schemas: &'a HashMap<String, ObjectSchema>,
    maps: &'a HashMap<String, ValueMap>,
    object_groups: &'a HashMap<String, Vec<ObjectId>>,
    variable_names: &'a HashMap<String, VariableId>,
    object_layers: &'a HashMap<ObjectId, LayerId>,
    mark_names: &'a HashMap<String, MarkDef>,
    value_sets: &'a HashMap<String, Vec<String>>,
    input_names: &'a HashMap<String, InputId>,
    directions: &'a [OrientationEnvironment],
}

struct QueryLoweringAdapter2d;

impl<'a> crate::solver_surface::SolverQueryLoweringAdapter<QueryLoweringContext<'a>>
    for QueryLoweringAdapter2d
{
    type Object = ObjectId;
    type Value = CanonicalConditionValueKind;
    type Variable = VariableId;
    type Error = DiagnosticReport;

    fn lower_variable(
        name: &str,
        _source_line: &str,
        context: &QueryLoweringContext<'a>,
    ) -> Result<Option<Self::Variable>, Self::Error> {
        Ok(context.variable_names.get(name).copied())
    }

    fn lower_distance_selector(
        selector: &SolverSurfaceQueryArg,
        source_line: &str,
        context: &QueryLoweringContext<'a>,
    ) -> Result<Vec<Self::Object>, Self::Error> {
        let SolverSurfaceQueryArg::Selector(selector) = selector else {
            return Err(DiagnosticReport::error_at_line(
                "distance query must be: distance(<selector>, <selector>)",
                source_line,
            ));
        };
        lower_query_selector2d(selector, source_line, context)
    }

    fn lower_selector_query_value(
        kind: crate::solver_surface::SolverQueryCallKind,
        selector: &str,
        source_line: &str,
        context: &QueryLoweringContext<'a>,
    ) -> Result<Self::Value, Self::Error> {
        let objects = lower_query_selector2d(selector, source_line, context)?;
        Ok(match kind {
            crate::solver_surface::SolverQueryCallKind::Count => {
                CanonicalConditionValueKind::CountObjects(objects)
            }
            crate::solver_surface::SolverQueryCallKind::Exists => {
                CanonicalConditionValueKind::ExistsObjects(objects)
            }
            crate::solver_surface::SolverQueryCallKind::None => {
                CanonicalConditionValueKind::NoneObjects(objects)
            }
        })
    }

    fn lower_pattern_query_value(
        kind: crate::solver_surface::SolverQueryCallKind,
        pattern: &SolverSurfacePatternArg,
        _source_line: &str,
        context: &QueryLoweringContext<'a>,
    ) -> Result<Self::Value, Self::Error> {
        let pattern = lower_condition_pattern_arg2d(pattern, context)?;
        let kind = match kind {
            crate::solver_surface::SolverQueryCallKind::Count => {
                ConditionValueAst::CountMatches(pattern)
            }
            crate::solver_surface::SolverQueryCallKind::Exists => {
                ConditionValueAst::ExistsMatches(pattern)
            }
            crate::solver_surface::SolverQueryCallKind::None => {
                ConditionValueAst::NoneMatches(pattern)
            }
        };
        lower_condition_value_kind(
            &kind,
            context.input_names,
            context.object_layers,
            context.mark_names,
            context.value_sets,
            context.maps,
            context.directions,
        )
    }

    fn query_call_error(message: &'static str, source_line: &str) -> Self::Error {
        DiagnosticReport::error_at_line(message, source_line)
    }

    fn cycle_error(cycle: Vec<String>, source_line: &str) -> Self::Error {
        DiagnosticReport::error_at_line(
            format!("query definitions contain a cycle: {}", cycle.join(" -> ")),
            source_line,
        )
    }

    fn unknown_query_error(name: &str, source_line: &str) -> Self::Error {
        DiagnosticReport::error_at_line(
            format!("unknown query or variable in query expression: {name}"),
            source_line,
        )
    }
}

fn lower_condition_pattern_arg2d(
    pattern: &SolverSurfacePatternArg,
    context: &QueryLoweringContext<'_>,
) -> Result<ConditionPatternAst, DiagnosticReport> {
    parse_condition_pattern_surface_arg(
        pattern,
        context.object_names,
        context.object_schemas,
        context.value_sets,
        context.maps,
        context.object_groups,
    )
}

fn lower_query_selector2d(
    selector: &str,
    source_line: &str,
    context: &QueryLoweringContext<'_>,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    Ok(resolve_object_selector(
        selector,
        source_line,
        context.object_names,
        context.object_schemas,
        context.value_sets,
        context.maps,
        context.object_groups,
        &HashMap::new(),
    )?
    .alternatives)
}

fn lower_solver_strategy(
    strategy: Option<SolverStrategyAst>,
    query_definitions: &[QueryDefinitionAst],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<CanonicalSolverStrategy, DiagnosticReport> {
    let context = QueryLoweringContext {
        object_names,
        object_schemas,
        maps,
        object_groups,
        variable_names,
        object_layers,
        mark_names,
        value_sets,
        input_names,
        directions,
    };
    crate::solver_surface::lower_solver_strategy_with::<QueryLoweringAdapter2d, _>(
        strategy,
        query_definitions,
        &context,
    )
}

fn lower_goal_expr(
    condition: &ConditionAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    variable_names: &HashMap<String, VariableId>,
    condition_names: &HashMap<String, ConditionId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Result<CanonicalGoalExpr, DiagnosticReport> {
    match condition {
        ConditionAst::All(conditions) => Ok(CanonicalGoalExpr::All(
            conditions
                .iter()
                .map(|condition| {
                    lower_goal_expr(
                        condition,
                        object_layers,
                        variable_names,
                        condition_names,
                        mark_names,
                        value_sets,
                        maps,
                        input_names,
                        directions,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        )),
        ConditionAst::Any(conditions) => Ok(CanonicalGoalExpr::Any(
            conditions
                .iter()
                .map(|condition| {
                    lower_goal_expr(
                        condition,
                        object_layers,
                        variable_names,
                        condition_names,
                        mark_names,
                        value_sets,
                        maps,
                        input_names,
                        directions,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        )),
        ConditionAst::AllObjectsOn { subjects, covers } => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::InlineConditionValue(all_objects_on_condition_kind(
                    subjects, covers,
                )),
                op: ComparisonOp::NotEq,
                expected: 0,
            }))
        }
        ConditionAst::VariableEquals { name, value } => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::Variable(resolve_variable_for_goal(
                    name,
                    variable_names,
                )?),
                op: ComparisonOp::Eq,
                expected: *value,
            }))
        }
        ConditionAst::VariableCompare { name, op, value } => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::Variable(resolve_variable_for_goal(
                    name,
                    variable_names,
                )?),
                op: *op,
                expected: *value,
            }))
        }
        ConditionAst::ConditionEquals { name, value } => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::Condition(resolve_condition_for_goal(
                    name,
                    condition_names,
                )?),
                op: ComparisonOp::Eq,
                expected: *value,
            }))
        }
        ConditionAst::ConditionNonZero(name) => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::Condition(resolve_condition_for_goal(
                    name,
                    condition_names,
                )?),
                op: ComparisonOp::NotEq,
                expected: 0,
            }))
        }
        ConditionAst::ConditionCompare { name, op, value } => {
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::Condition(resolve_condition_for_goal(
                    name,
                    condition_names,
                )?),
                op: *op,
                expected: *value,
            }))
        }
        ConditionAst::InlineConditionValueEquals { kind, value } => {
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                mark_names,
                value_sets,
                maps,
                directions,
            )?;
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::InlineConditionValue(kind),
                op: ComparisonOp::Eq,
                expected: *value,
            }))
        }
        ConditionAst::InlineConditionNonZero(kind) => {
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                mark_names,
                value_sets,
                maps,
                directions,
            )?;
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::InlineConditionValue(kind),
                op: ComparisonOp::NotEq,
                expected: 0,
            }))
        }
        ConditionAst::InlineConditionCompare { kind, op, value } => {
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                mark_names,
                value_sets,
                maps,
                directions,
            )?;
            Ok(CanonicalGoalExpr::Clause(CanonicalGoalClause {
                value: CanonicalGoalValue::InlineConditionValue(kind),
                op: *op,
                expected: *value,
            }))
        }
        ConditionAst::InputIs(_) | ConditionAst::InputIn(_) => Err(DiagnosticReport::error(
            "goal cannot depend on input".to_string(),
        )),
    }
}

fn resolve_variable_for_goal(
    name: &str,
    variable_names: &HashMap<String, VariableId>,
) -> Result<VariableId, DiagnosticReport> {
    variable_names
        .get(name)
        .copied()
        .ok_or_else(|| DiagnosticReport::error(format!("unknown variable in goal: {name}")))
}

fn resolve_condition_for_goal(
    name: &str,
    condition_names: &HashMap<String, ConditionId>,
) -> Result<ConditionId, DiagnosticReport> {
    condition_names
        .get(name)
        .copied()
        .ok_or_else(|| DiagnosticReport::error(format!("unknown condition in goal: {name}")))
}

fn all_objects_on_condition_kind(
    subjects: &[ObjectId],
    covers: &[ObjectId],
) -> CanonicalConditionValueKind {
    CanonicalConditionValueKind::NoneMatches(
        subjects
            .iter()
            .map(|subject| CanonicalPattern {
                components: vec![CanonicalPatternComponent::new(vec![CanonicalMatchCell {
                    offset: CanonicalOffset::Fixed {
                        delta: [0, 0, 0].into(),
                    },
                    require_null: false,
                    require_objects: vec![*subject],
                    require_object_sets: Vec::new(),
                    forbid_objects: covers.to_vec(),
                    require_mark: Vec::new(),
                    require_object_set_mark: Vec::new(),
                    forbid_mark: Vec::new(),
                    forbid_object_set_mark: Vec::new(),
                }])],
            })
            .collect(),
    )
}

impl<'a> ProgramLowerer<'a> {
    fn lower_statements(
        &mut self,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut scoped_context = context.clone();
        let local_definitions = local_routine_definitions(statements);
        if !local_definitions.is_empty() {
            scoped_context.local_definitions.push(local_definitions);
        }

        let mut rules = Vec::new();
        for statement in statements {
            rules.extend(self.lower_statement(statement, &scoped_context)?);
        }
        Ok(rules)
    }

    fn lower_statement(
        &mut self,
        statement: &StatementAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        match statement {
            StatementAst::LocalRoutine { .. } => Ok(Vec::new()),
            StatementAst::Call {
                name,
                source_line,
                source_line_number,
            } => self.lower_call(name, source_line, *source_line_number, context),
            StatementAst::Conditional {
                source_line,
                source_line_number,
                condition,
                then_statements,
                else_statements,
            } => self.lower_conditional(
                source_line,
                *source_line_number,
                condition,
                then_statements,
                else_statements,
                context,
            ),
            StatementAst::Block {
                application,
                statements,
            } => self.lower_block(*application, statements, context),
            StatementAst::RepeatUntil {
                source_line,
                source_line_number,
                condition,
                statements,
            } => self.lower_repeat_until(
                source_line,
                *source_line_number,
                condition,
                statements,
                context,
            ),
            StatementAst::Fix {
                defaults,
                statements,
            } => self.lower_fix(defaults, statements, context),
            StatementAst::If {
                source_line,
                source_line_number,
                condition,
                then_statements,
                else_statements,
            } => self.lower_if(
                source_line,
                *source_line_number,
                condition,
                then_statements,
                else_statements,
                context,
            ),
            StatementAst::Effect {
                source_line,
                source_line_number,
                effects,
            } => self.lower_effect_statement(source_line, *source_line_number, effects, context),
            StatementAst::Rewrite(rewrite) => self.lower_rewrite(rewrite, context),
        }
    }

    fn lower_effect_statement(
        &mut self,
        source_line: &str,
        source_line_number: Option<usize>,
        effects: &[EffectAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        if effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::EmitVisual { .. }))
        {
            return Err(report_at_source_line_number(
                "visual animation emission requires a rewrite match position",
                source_line,
                source_line_number,
            ));
        }
        let effects = self.lower_effects(effects)?;
        let id = RuleId(self.next_rule_id);
        self.next_rule_id += 1;
        if !effects.ordered.is_empty() {
            self.rule_effects.insert(id, effects.ordered.clone());
        }
        self.record_rule_debug_info(id, source_line, source_line_number, context);
        Ok(vec![CanonicalRuleStep::Rule(CanonicalRule {
            id,
            guards: context.guards.clone(),
            application: RuleApplication::Once,
            pattern: CanonicalPattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: effects.core,
        })])
    }

    fn lower_conditional(
        &mut self,
        source_line: &str,
        source_line_number: Option<usize>,
        condition: &PatternConditionAst,
        then_statements: &[StatementAst],
        else_statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        if else_statements.is_empty() {
            return Ok(vec![CanonicalRuleStep::ConditionalBlock {
                condition: self.lower_pattern_condition(
                    condition,
                    context,
                    source_line,
                    source_line_number,
                )?,
                steps: self.lower_statements(then_statements, context)?,
            }]);
        }
        Ok(vec![CanonicalRuleStep::ConditionalBranch {
            condition: self.lower_pattern_condition(
                condition,
                context,
                source_line,
                source_line_number,
            )?,
            then_steps: self.lower_statements(then_statements, context)?,
            else_steps: self.lower_statements(else_statements, context)?,
        }])
    }

    fn lower_block(
        &mut self,
        application: RuleApplication,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if !nested_context.application_fixed {
            nested_context.application = if application == RuleApplication::Random {
                RuleApplication::Once
            } else {
                RuleApplication::UntilStable
            };
        }
        let steps = self.lower_statements(statements, &nested_context)?;
        Ok(vec![CanonicalRuleStep::Block {
            application,
            stop_condition: None,
            steps,
        }])
    }

    fn lower_repeat_until(
        &mut self,
        source_line: &str,
        source_line_number: Option<usize>,
        condition: &ConditionAst,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if !nested_context.application_fixed {
            nested_context.application = RuleApplication::UntilStable;
        }
        let stop_condition =
            self.lower_guard_condition(condition, context, source_line, source_line_number)?;
        let steps = self.lower_statements(statements, &nested_context)?;
        Ok(vec![CanonicalRuleStep::Block {
            application: RuleApplication::UntilStable,
            stop_condition: Some(stop_condition),
            steps,
        }])
    }

    fn lower_fix(
        &mut self,
        defaults: &FixDefaults,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if let Some(application) = defaults.application {
            nested_context.application = application;
            nested_context.application_fixed = true;
        }
        if let Some(orientation) = &defaults.orientation {
            nested_context.orientation = Some(orientation.clone());
        }
        self.lower_statements(statements, &nested_context)
    }

    fn lower_call(
        &mut self,
        name: &str,
        source_line: &str,
        source_line_number: Option<usize>,
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        if context.call_stack.iter().any(|active| active == name) {
            return Err(report_at_source_line_number(
                format!("recursive routine call: {name}"),
                source_line,
                source_line_number,
            ));
        }
        let resolved =
            self.resolve_routine_definition(name, source_line, source_line_number, context)?;
        let definition = resolved.definition;
        let mut nested_context = context.clone();
        if !resolved.is_local {
            nested_context.local_definitions.clear();
        }
        nested_context.call_stack.push(name.to_string());
        nested_context.application = if definition.application == RuleApplication::Random {
            RuleApplication::Once
        } else {
            RuleApplication::UntilStable
        };
        nested_context.application_fixed = false;
        nested_context.orientation = None;
        let steps = self.lower_statements(&definition.statements, &nested_context)?;
        Ok(vec![CanonicalRuleStep::Block {
            application: definition.application,
            stop_condition: None,
            steps,
        }])
    }

    fn resolve_routine_definition(
        &self,
        name: &str,
        source_line: &str,
        source_line_number: Option<usize>,
        context: &StatementLoweringContext,
    ) -> Result<ResolvedRoutineDefinition, DiagnosticReport> {
        for scope in context.local_definitions.iter().rev() {
            if let Some(definition) = scope.get(name) {
                return Ok(ResolvedRoutineDefinition {
                    definition: definition.clone(),
                    is_local: true,
                });
            }
        }
        self.definitions
            .get(name)
            .cloned()
            .map(|definition| ResolvedRoutineDefinition {
                definition,
                is_local: false,
            })
            .ok_or_else(|| {
                report_at_source_line_number(
                    format!("unknown routine call: {name}"),
                    source_line,
                    source_line_number,
                )
            })
    }

    fn lower_pattern_condition(
        &self,
        condition: &PatternConditionAst,
        context: &StatementLoweringContext,
        source_line: &str,
        source_line_number: Option<usize>,
    ) -> Result<CanonicalRuleCondition, DiagnosticReport> {
        let orientation = if matches!(condition.orientation, OrientationExpr::Neutral) {
            context
                .orientation
                .as_ref()
                .unwrap_or(&condition.orientation)
        } else {
            &condition.orientation
        };
        let patterns = match orientation {
            OrientationExpr::Neutral => {
                if pattern_block_requires_implicit_cardinal_expansion(
                    &condition.pattern,
                    self.value_sets,
                ) {
                    let implicit_directions = implicit_spatial_directions(
                        self.input_names,
                        self.value_sets,
                        self.directions,
                    )?;
                    self.condition_patterns_for_directions(
                        &condition.pattern,
                        &implicit_directions,
                        true,
                        "implicit directional pattern condition",
                    )?
                } else {
                    self.condition_patterns(
                        &condition.pattern,
                        neutral_direction(self.directions),
                        false,
                        "neutral pattern condition",
                    )?
                }
            }
            OrientationExpr::Input => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        source_line,
                        source_line_number,
                    ));
                }
                let mut patterns = Vec::new();
                for direction in self.directions {
                    for pattern in self.condition_patterns(
                        &condition.pattern,
                        *direction,
                        true,
                        "input pattern condition",
                    )? {
                        patterns.push((direction.input, pattern));
                    }
                }
                return Ok(match condition.predicate {
                    PatternPredicateAst::Some => CanonicalRuleCondition::AnyInputMatches(patterns),
                    PatternPredicateAst::None => CanonicalRuleCondition::NoInputMatches(patterns),
                });
            }
            OrientationExpr::InputSet(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        source_line,
                        source_line_number,
                    ));
                }
                let directions = self.directions_for_orientation_name(axis)?.ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown input orientation set: {axis}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                let mut patterns = Vec::new();
                for direction in directions {
                    for pattern in self.condition_patterns(
                        &condition.pattern,
                        direction,
                        true,
                        "input pattern condition",
                    )? {
                        patterns.push((direction.input, pattern));
                    }
                }
                return Ok(match condition.predicate {
                    PatternPredicateAst::Some => CanonicalRuleCondition::AnyInputMatches(patterns),
                    PatternPredicateAst::None => CanonicalRuleCondition::NoInputMatches(patterns),
                });
            }
            OrientationExpr::Fixed(direction_name) => {
                let directions = self
                    .directions_for_orientation_name(&direction_name.0)?
                    .ok_or_else(|| {
                        report_at_source_line_number(
                            format!(
                                "unknown pattern condition orientation: {}",
                                direction_name.0
                            ),
                            source_line,
                            source_line_number,
                        )
                    })?;
                self.condition_patterns_for_directions(
                    &condition.pattern,
                    &directions,
                    true,
                    "fixed pattern condition",
                )?
            }
        };

        let condition = match condition.predicate {
            PatternPredicateAst::Some => CanonicalRuleCondition::AnyMatches(patterns),
            PatternPredicateAst::None => CanonicalRuleCondition::NoMatches(patterns),
        };
        Ok(condition)
    }

    fn lower_guard_condition(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
        source_line: &str,
        source_line_number: Option<usize>,
    ) -> Result<CanonicalRuleCondition, DiagnosticReport> {
        Ok(CanonicalRuleCondition::GuardBranches(
            self.lower_condition_branches(condition, context, source_line, source_line_number)?,
        ))
    }

    fn condition_patterns(
        &self,
        pattern: &PatternBlock,
        direction: OrientationEnvironment,
        direction_expanded: bool,
        line: &str,
    ) -> Result<Vec<CanonicalPattern>, DiagnosticReport> {
        let alternatives = compile_before_after_blocks(
            pattern,
            pattern,
            self.object_layers,
            self.mark_names,
            self.value_sets,
            self.maps,
            line,
            None,
        )?;
        patterns_from_alternatives(&alternatives, &[direction], direction_expanded, line)
    }

    fn condition_patterns_for_directions(
        &self,
        pattern: &PatternBlock,
        directions: &[OrientationEnvironment],
        direction_expanded: bool,
        line: &str,
    ) -> Result<Vec<CanonicalPattern>, DiagnosticReport> {
        let mut patterns = Vec::new();
        for direction in directions {
            let (_, alternatives) = compile_before_after_blocks_for_direction(
                pattern,
                pattern,
                self.object_layers,
                self.mark_names,
                self.value_sets,
                self.maps,
                *direction,
                direction_expanded,
                line,
                None,
            )?;
            patterns.extend(patterns_from_alternatives(
                &alternatives,
                &[*direction],
                direction_expanded,
                line,
            )?);
        }
        Ok(patterns)
    }

    fn lower_if(
        &mut self,
        source_line: &str,
        source_line_number: Option<usize>,
        condition: &ConditionAst,
        then_statements: &[StatementAst],
        else_statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        if !else_statements.is_empty() {
            return Ok(vec![CanonicalRuleStep::ConditionalBranch {
                condition: self.lower_guard_condition(
                    condition,
                    context,
                    source_line,
                    source_line_number,
                )?,
                then_steps: self.lower_statements(then_statements, context)?,
                else_steps: self.lower_statements(else_statements, context)?,
            }]);
        }
        Ok(vec![CanonicalRuleStep::ConditionalBlock {
            condition: self.lower_guard_condition(
                condition,
                context,
                source_line,
                source_line_number,
            )?,
            steps: self.lower_statements(then_statements, context)?,
        }])
    }

    fn input_ids_for_value_set(
        &self,
        name: &str,
        source_line: &str,
        source_line_number: Option<usize>,
    ) -> Result<Vec<InputId>, DiagnosticReport> {
        let values = self.value_sets.get(name).ok_or_else(|| {
            report_at_source_line_number(
                format!("unknown input tag set: {name}"),
                source_line,
                source_line_number,
            )
        })?;
        if values.is_empty() {
            return Err(report_at_source_line_number(
                format!("empty input tag set: {name}"),
                source_line,
                source_line_number,
            ));
        }
        values
            .iter()
            .map(|value| {
                self.input_names.get(value).copied().ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown input in tag set: {value}"),
                        source_line,
                        source_line_number,
                    )
                })
            })
            .collect()
    }

    fn directions_for_orientation_name(
        &self,
        name: &str,
    ) -> Result<Option<Vec<OrientationEnvironment>>, DiagnosticReport> {
        directions_for_orientation_name(name, self.input_names, self.value_sets, self.directions)
    }

    fn lower_condition_branches(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
        source_line: &str,
        source_line_number: Option<usize>,
    ) -> Result<Vec<Vec<CanonicalGuard>>, DiagnosticReport> {
        match condition {
            ConditionAst::All(conditions) => {
                let mut branches = vec![Vec::<CanonicalGuard>::new()];
                for condition in conditions {
                    let next_branches = self.lower_condition_branches(
                        condition,
                        context,
                        source_line,
                        source_line_number,
                    )?;
                    let mut combined = Vec::new();
                    for branch in &branches {
                        for next_branch in &next_branches {
                            let mut merged = branch.clone();
                            merged.extend(next_branch.clone());
                            combined.push(merged);
                        }
                    }
                    branches = combined;
                }
                Ok(branches)
            }
            ConditionAst::Any(conditions) => {
                let mut branches = Vec::new();
                for condition in conditions {
                    branches.extend(self.lower_condition_branches(
                        condition,
                        context,
                        source_line,
                        source_line_number,
                    )?);
                }
                Ok(branches)
            }
            ConditionAst::InputIn(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        source_line,
                        source_line_number,
                    ));
                }
                Ok(self
                    .input_ids_for_value_set(axis, source_line, source_line_number)?
                    .into_iter()
                    .map(|input| vec![CanonicalGuard::InputIs(input)])
                    .collect())
            }
            _ => Ok(vec![vec![self.lower_condition_clause(
                condition,
                context,
                source_line,
                source_line_number,
            )?]]),
        }
    }

    fn lower_condition_clause(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
        source_line: &str,
        source_line_number: Option<usize>,
    ) -> Result<CanonicalGuard, DiagnosticReport> {
        match condition {
            ConditionAst::InputIs(input_name) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        source_line,
                        source_line_number,
                    ));
                }
                let input = *self.input_names.get(input_name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown input: {input_name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::InputIs(input))
            }
            ConditionAst::InputIn(_) => Err(report_at_source_line_number(
                "input tag-set condition was not expanded",
                source_line,
                source_line_number,
            )),
            ConditionAst::VariableEquals { name, value } => {
                let variable = *self.variable_names.get(name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown variable: {name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::VariableEquals {
                    variable,
                    value: *value,
                })
            }
            ConditionAst::VariableCompare { name, op, value } => {
                let variable = *self.variable_names.get(name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown variable: {name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::VariableCompare {
                    variable,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::ConditionEquals { name, value } => {
                let condition = *self.condition_names.get(name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown condition: {name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::ConditionEquals {
                    condition,
                    value: *value,
                })
            }
            ConditionAst::ConditionNonZero(name) => {
                let condition = *self.condition_names.get(name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown condition: {name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::ConditionNonZero(condition))
            }
            ConditionAst::ConditionCompare { name, op, value } => {
                let condition = *self.condition_names.get(name).ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown condition: {name}"),
                        source_line,
                        source_line_number,
                    )
                })?;
                Ok(CanonicalGuard::ConditionCompare {
                    condition,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::InlineConditionValueEquals { kind, value } => {
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.mark_names,
                    self.value_sets,
                    self.maps,
                    self.directions,
                )?;
                Ok(CanonicalGuard::InlineConditionValue {
                    kind,
                    value: *value,
                })
            }
            ConditionAst::InlineConditionNonZero(kind) => {
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.mark_names,
                    self.value_sets,
                    self.maps,
                    self.directions,
                )?;
                Ok(CanonicalGuard::InlineConditionNonZero(kind))
            }
            ConditionAst::InlineConditionCompare { kind, op, value } => {
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.mark_names,
                    self.value_sets,
                    self.maps,
                    self.directions,
                )?;
                Ok(CanonicalGuard::InlineConditionCompare {
                    kind,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::All(_) | ConditionAst::Any(_) => Err(report_at_source_line_number(
                "nested condition expression was not expanded",
                source_line,
                source_line_number,
            )),
            ConditionAst::AllObjectsOn { subjects, covers } => {
                Ok(CanonicalGuard::InlineConditionNonZero(
                    all_objects_on_condition_kind(subjects, covers),
                ))
            }
        }
    }

    fn lower_rewrite(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut anchored_rewrite = rewrite.clone();
        let mut followup_effects = Vec::new();
        for effect in &rewrite.after_effects {
            if matches!(effect, EffectAst::EmitVisual { .. }) {
                anchored_rewrite.effects.push(effect.clone());
            } else {
                followup_effects.push(effect.clone());
            }
        }
        anchored_rewrite.after_effects.clear();
        let steps = self.lower_rewrite_core(&anchored_rewrite, context)?;
        if followup_effects.is_empty() && rewrite.after_call.is_none() {
            return Ok(steps);
        }

        let mut then_steps = Vec::new();
        if !followup_effects.is_empty() {
            then_steps.extend(self.lower_effect_statement(
                &rewrite.source_line,
                rewrite.source_line_number,
                &followup_effects,
                context,
            )?);
        }
        if let Some(after_call) = &rewrite.after_call {
            then_steps.extend(self.lower_call(
                after_call,
                &rewrite.source_line,
                rewrite.source_line_number,
                context,
            )?);
        }
        Ok(vec![CanonicalRuleStep::AfterTriggered {
            steps,
            then_steps,
        }])
    }

    fn lower_rewrite_core(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let application = if rewrite
            .effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Win | EffectAst::NextLevel))
        {
            RuleApplication::Once
        } else {
            rewrite.application.unwrap_or(context.application)
        };
        let orientation = if matches!(rewrite.orientation, OrientationExpr::Neutral) {
            context.orientation.as_ref().unwrap_or(&rewrite.orientation)
        } else {
            &rewrite.orientation
        };
        let preserve_once_group = pattern_block_preserves_once_group(&rewrite.after);
        match orientation {
            OrientationExpr::Neutral => {
                if rewrite_requires_implicit_cardinal_expansion(rewrite, self.value_sets) {
                    let mut rules = Vec::new();
                    let implicit_directions = implicit_spatial_directions(
                        self.input_names,
                        self.value_sets,
                        self.directions,
                    )?;
                    for direction in &implicit_directions {
                        rules.extend(self.lower_rewrite_rules_for_direction(
                            rewrite,
                            context,
                            &rewrite.effects,
                            application,
                            *direction,
                            true,
                            context.guards.clone(),
                        )?);
                    }
                    self.dedup_orientation_rules(&mut rules);
                    return Ok(wrap_rewrite_steps(
                        application,
                        rules,
                        preserve_once_group,
                    ));
                }
                self.lower_rewrite_rules_for_direction(
                    rewrite,
                    context,
                    &rewrite.effects,
                    application,
                    neutral_direction(self.directions),
                    false,
                    context.guards.clone(),
                )
                .map(|rules| wrap_rewrite_steps(application, rules, preserve_once_group))
            }
            OrientationExpr::Input => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        &rewrite.source_line,
                        rewrite.source_line_number,
                    ));
                }
                let mut rules = Vec::new();
                for direction in self.directions {
                    let mut guards = context.guards.clone();
                    guards.push(CanonicalGuard::InputIs(direction.input));
                    rules.extend(self.lower_rewrite_rules_for_direction(
                        rewrite,
                        context,
                        &rewrite.effects,
                        application,
                        *direction,
                        true,
                        guards,
                    )?);
                }
                self.dedup_orientation_rules(&mut rules);
                Ok(wrap_rewrite_steps(
                    application,
                    rules,
                    preserve_once_group,
                ))
            }
            OrientationExpr::InputSet(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(
                        context,
                        &rewrite.source_line,
                        rewrite.source_line_number,
                    ));
                }
                let directions = self.directions_for_orientation_name(axis)?.ok_or_else(|| {
                    report_at_source_line_number(
                        format!("unknown input orientation set: {axis}"),
                        &rewrite.source_line,
                        rewrite.source_line_number,
                    )
                })?;
                let mut rules = Vec::new();
                for direction in directions {
                    let mut guards = context.guards.clone();
                    guards.push(CanonicalGuard::InputIs(direction.input));
                    rules.extend(self.lower_rewrite_rules_for_direction(
                        rewrite,
                        context,
                        &rewrite.effects,
                        application,
                        direction,
                        true,
                        guards,
                    )?);
                }
                self.dedup_orientation_rules(&mut rules);
                Ok(wrap_rewrite_steps(
                    application,
                    rules,
                    preserve_once_group,
                ))
            }
            OrientationExpr::Fixed(direction_name) => {
                let directions = self
                    .directions_for_orientation_name(&direction_name.0)?
                    .ok_or_else(|| {
                        report_at_source_line_number(
                            format!("unknown orientation: {}", direction_name.0),
                            &rewrite.source_line,
                            rewrite.source_line_number,
                        )
                    })?;
                let mut rules = Vec::new();
                for direction in directions {
                    rules.extend(self.lower_rewrite_rules_for_direction(
                        rewrite,
                        context,
                        &rewrite.effects,
                        application,
                        direction,
                        true,
                        context.guards.clone(),
                    )?);
                }
                self.dedup_orientation_rules(&mut rules);
                Ok(wrap_rewrite_steps(
                    application,
                    rules,
                    preserve_once_group,
                ))
            }
        }
    }

    fn dedup_orientation_rules(&mut self, rules: &mut Vec<CanonicalRuleStep>) {
        fn same_writes(left: &[CanonicalWriteOp], right: &[CanonicalWriteOp]) -> bool {
            let mut left = left.to_vec();
            let mut right = right.to_vec();
            left.sort_unstable();
            right.sort_unstable();
            left == right
        }

        let mut unique = Vec::<CanonicalRule>::new();
        rules.retain(|step| {
            let CanonicalRuleStep::Rule(rule) = step else {
                return true;
            };
            let duplicate = unique.iter().any(|existing| {
                existing.guards == rule.guards
                    && existing.application == rule.application
                    && existing.pattern == rule.pattern
                    && same_writes(&existing.writes, &rule.writes)
                    && existing.effects == rule.effects
            });
            if duplicate {
                self.rule_animations.remove(&rule.id);
                self.rule_effects.remove(&rule.id);
                self.rule_debug_info.remove(&rule.id);
                false
            } else {
                unique.push(rule.clone());
                true
            }
        });
    }

    fn lower_rewrite_rules_for_direction(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
        effects: &[EffectAst],
        application: RuleApplication,
        direction: OrientationEnvironment,
        direction_expanded: bool,
        guards: Vec<CanonicalGuard>,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let (_before, alternatives) = compile_before_after_blocks_for_direction(
            &rewrite.before,
            &rewrite.after,
            self.object_layers,
            self.mark_names,
            self.value_sets,
            self.maps,
            direction,
            direction_expanded,
            &rewrite.source_line,
            rewrite.source_line_number,
        )?;
        self.rules_from_alternatives(
            alternatives,
            direction,
            direction_expanded,
            guards,
            effects,
            application,
            &rewrite.source_line,
            rewrite.source_line_number,
            context,
        )
    }

    fn record_rule_debug_info(
        &mut self,
        id: RuleId,
        source_line: &str,
        source_line_number: Option<usize>,
        context: &StatementLoweringContext,
    ) {
        self.rule_debug_info.insert(
            id,
            RuleDebugInfo {
                source_line: source_line.trim().to_string(),
                source_line_number,
                routine_stack: context.call_stack.clone(),
            },
        );
    }

    fn lower_effects(&self, effects: &[EffectAst]) -> Result<LoweredEffects, DiagnosticReport> {
        let mut lowered = LoweredEffects::default();
        self.lower_effects_into(effects, None, &mut lowered)?;
        lowered.mark_external_observation();
        Ok(lowered)
    }

    fn lower_effects_for_rewrite(
        &self,
        effects: &[EffectAst],
        tag_captures: &TagCaptureValues,
    ) -> Result<LoweredEffects, DiagnosticReport> {
        let mut lowered = LoweredEffects::default();
        self.lower_effects_into(effects, Some(tag_captures), &mut lowered)?;
        lowered.mark_external_observation();
        Ok(lowered)
    }

    fn lower_effects_into(
        &self,
        effects: &[EffectAst],
        tag_captures: Option<&TagCaptureValues>,
        lowered: &mut LoweredEffects,
    ) -> Result<(), DiagnosticReport> {
        for effect in effects {
            match effect {
                EffectAst::Cancel => lowered.core.push(Effect::Cancel),
                EffectAst::Win => {
                    lowered.core.push(Effect::Win);
                    lowered.ordered.push(RuleEffect::Win);
                }
                EffectAst::Restart => {
                    lowered.core.push(Effect::Restart);
                    lowered.ordered.push(RuleEffect::Restart);
                }
                EffectAst::NextLevel => {
                    lowered.core.push(Effect::NextLevel);
                    lowered.ordered.push(RuleEffect::NextLevel);
                }
                EffectAst::Again => {
                    lowered.core.push(Effect::Again);
                    lowered.ordered.push(RuleEffect::Again);
                }
                EffectAst::Checkpoint => {
                    lowered.core.push(Effect::Checkpoint);
                    lowered.ordered.push(RuleEffect::Checkpoint);
                }
                EffectAst::ClearCheckpoint => {
                    lowered.core.push(Effect::ClearCheckpoint);
                    lowered.ordered.push(RuleEffect::ClearCheckpoint);
                }
                EffectAst::PlaySfx { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PlaySfx { name: name.clone() });
                }
                EffectAst::PlayMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PlayMusic { name: name.clone() });
                }
                EffectAst::PauseMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PauseMusic { name: name.clone() });
                }
                EffectAst::ResumeMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::ResumeMusic { name: name.clone() });
                }
                EffectAst::StopMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::StopMusic { name: name.clone() });
                }
                EffectAst::Wait { milliseconds } => {
                    lowered.ordered.push(RuleEffect::Wait {
                        milliseconds: *milliseconds,
                    });
                }
                EffectAst::WaitAnimation => {
                    lowered.ordered.push(RuleEffect::WaitAnimation);
                }
                EffectAst::EmitVisual { name } => {
                    if !self.visual_names.contains(name) {
                        return Err(DiagnosticReport::error(format!(
                            "unknown visual animation: !{name}"
                        )));
                    }
                    if !self.animation_visual_names.contains(name) {
                        return Err(DiagnosticReport::error(format!(
                            "visual animation is not declared in layers: !{name}"
                        )));
                    }
                    lowered.ordered.push(RuleEffect::EmitAnimation {
                        name: name.clone(),
                        component: 0,
                        offset: puzzle_runtime_contract::RuntimeAnimationOffset { x: 0, y: 0 },
                    });
                }
                EffectAst::PresentComponent { text, literal } => {
                    lowered.ordered.push(RuleEffect::PresentComponent {
                        definition: "standard.message".to_string(),
                        properties: vec![
                            puzzle_runtime_contract::RuntimeComponentProperty {
                                name: "text".to_string(),
                                value: text.clone(),
                                literal: *literal,
                            },
                        ],
                        placement: puzzle_runtime_contract::ComponentPlacement::Overlay,
                        await_event: Some("dismiss".to_string()),
                    });
                }
                EffectAst::Scene(effect) => {
                    lowered.ordered.push(RuleEffect::Scene {
                        effect: effect.clone(),
                    });
                }
                EffectAst::UpdateVariable { name, op, value } => {
                    let variable = *self.variable_names.get(name).ok_or_else(|| {
                        DiagnosticReport::error(format!("unknown variable in effect: {name}"))
                    })?;
                    if self.constant_variables.contains(&variable) {
                        return Err(DiagnosticReport::error(format!(
                            "cannot update const: {name}"
                        )));
                    }
                    let value = match value {
                        VariableValueAst::Literal(value) => *value,
                        VariableValueAst::TagCapture(key) => {
                            let captures = tag_captures.ok_or_else(|| {
                                DiagnosticReport::error(format!(
                                    "tag capture reference `{key}` can only be used in rewrite effects"
                                ))
                            })?;
                            captures.resolve(key, "rewrite effect")?
                        }
                    };
                    lowered.core.push(Effect::UpdateVariable {
                        variable,
                        op: *op,
                        value,
                    });
                }
            }
        }
        Ok(())
    }

    fn rules_from_alternatives(
        &mut self,
        alternatives: Vec<RuleBodyAlternative>,
        direction: OrientationEnvironment,
        direction_expanded: bool,
        guards: Vec<CanonicalGuard>,
        effects: &[EffectAst],
        application: RuleApplication,
        source_line: &str,
        source_line_number: Option<usize>,
        context: &StatementLoweringContext,
    ) -> Result<Vec<CanonicalRuleStep>, DiagnosticReport> {
        let mut rules = Vec::with_capacity(alternatives.len());
        for alternative in alternatives {
            let lowered_effects =
                self.lower_effects_for_rewrite(effects, &alternative.tag_captures)?;
            let mut guards = guards.clone();
            guards.extend(alternative.guards.clone());
            let mut rule_effects = lowered_effects.ordered.clone();
            append_move_sound_effects(
                &alternative.writes,
                self.model_sound_triggers,
                &mut rule_effects,
            );
            let mut rule_animations = Vec::new();
            append_tween_rule_animations(
                &alternative.writes,
                self.animation,
                self.direction_variant_pairs,
                &mut rule_animations,
            );
            let compiled_components = alternative
                .components
                .iter()
                .map(|component| {
                    let cells = component
                        .cells
                        .iter()
                        .map(|cell| {
                            Ok(CanonicalMatchCell {
                                offset: resolve_offset(
                                    cell.offset.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                require_null: cell.require_null,
                                require_objects: cell.require_objects.clone(),
                                require_object_sets: cell.require_object_sets.clone(),
                                forbid_objects: cell.forbid_objects.clone(),
                                require_mark: resolve_mark_patterns(
                                    cell.require_mark.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                require_object_set_mark: resolve_object_set_mark_patterns(
                                    cell.require_object_set_mark.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                forbid_mark: resolve_mark_patterns(
                                    cell.forbid_mark.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                forbid_object_set_mark: resolve_object_set_mark_patterns(
                                    cell.forbid_object_set_mark.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                            })
                        })
                        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
                    Ok(CanonicalPatternComponent {
                        cells,
                        gap_count: component.gap_count,
                    })
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            let compiled_writes = alternative
                .writes
                .iter()
                .map(|write| resolve_write(write, direction, direction_expanded, "statement"))
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            let id = RuleId(self.next_rule_id);
            self.next_rule_id += 1;
            if !rule_animations.is_empty() {
                self.rule_animations.insert(id, rule_animations);
            }
            if !rule_effects.is_empty() {
                self.rule_effects.insert(id, rule_effects);
            }
            self.record_rule_debug_info(id, source_line, source_line_number, context);
            rules.push(CanonicalRuleStep::Rule(CanonicalRule {
                id,
                guards,
                application,
                pattern: CanonicalPattern {
                    components: compiled_components,
                },
                writes: compiled_writes,
                effects: lowered_effects.core,
            }));
        }
        Ok(rules)
    }
}
