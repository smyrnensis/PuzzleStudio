fn wrap_rewrite_steps(
    application: RuleApplication,
    steps: Vec<CanonicalRuleStep>,
    preserve_once_group: bool,
) -> Vec<CanonicalRuleStep> {
    if application == RuleApplication::Once && preserve_once_group && steps.len() > 1 {
        return vec![once_rule_alternative_chain(steps)];
    }
    if matches!(
        application,
        RuleApplication::Random | RuleApplication::UntilStable
    ) {
        let steps = if application == RuleApplication::UntilStable {
            steps
                .into_iter()
                .map(|step| rewrite_step_with_application(step, RuleApplication::RepeatStep))
                .collect()
        } else {
            steps
        };
        vec![CanonicalRuleStep::Block {
            application,
            stop_condition: None,
            steps,
        }]
    } else {
        steps
    }
}

fn once_rule_alternative_chain(steps: Vec<CanonicalRuleStep>) -> CanonicalRuleStep {
    let alternatives = steps
        .into_iter()
        .map(|step| {
            let CanonicalRuleStep::Rule(rule) = step else {
                unreachable!("lowered rewrite alternatives must be rule steps");
            };
            (
                CanonicalRuleCondition::RuleMatches {
                    guards: rule.guards.clone(),
                    pattern: rule.pattern.clone(),
                },
                rule,
            )
        })
        .collect();
    puzzle_kernel::first_matching_program_alternative(alternatives)
        .expect("once alternative chain requires at least one rule")
}

fn rewrite_step_with_application(
    step: CanonicalRuleStep,
    application: RuleApplication,
) -> CanonicalRuleStep {
    match step {
        CanonicalRuleStep::Rule(mut rule) => {
            rule.application = application;
            CanonicalRuleStep::Rule(rule)
        }
        CanonicalRuleStep::ConditionalBlock { condition, steps } => {
            CanonicalRuleStep::ConditionalBlock {
                condition,
                steps: steps
                    .into_iter()
                    .map(|step| rewrite_step_with_application(step, application))
                    .collect(),
            }
        }
        CanonicalRuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => CanonicalRuleStep::ConditionalBranch {
            condition,
            then_steps: then_steps
                .into_iter()
                .map(|step| rewrite_step_with_application(step, application))
                .collect(),
            else_steps: else_steps
                .into_iter()
                .map(|step| rewrite_step_with_application(step, application))
                .collect(),
        },
        other => other,
    }
}

#[derive(Clone, Debug, Default)]
struct RuleBodyAlternative {
    guards: Vec<CanonicalGuard>,
    components: Vec<PatternComponentTemplate>,
    writes: Vec<WriteOpTemplate>,
    tag_captures: TagCaptureValues,
}

fn append_move_sound_effects(
    writes: &[WriteOpTemplate],
    triggers: &[ModelSoundTrigger],
    ordered_effects: &mut Vec<RuleEffect>,
) {
    if triggers.is_empty() {
        return;
    }
    for trigger in triggers {
        let matches_trigger = writes.iter().any(|write| match (trigger.kind, write) {
            (ModelSoundTriggerKind::Move, WriteOpTemplate::Move { object, .. }) => {
                trigger.objects.contains(object)
            }
            (ModelSoundTriggerKind::Move, WriteOpTemplate::MoveObjectSet { objects, .. }) => {
                objects
                    .iter()
                    .any(|object| trigger.objects.contains(object))
            }
            _ => false,
        });
        if !matches_trigger {
            continue;
        }
        let name = &trigger.sfx_name;
        if !ordered_effects.iter().any(
            |effect| matches!(effect, RuleEffect::PlaySfx { name: existing } if existing == name),
        ) {
            ordered_effects.push(RuleEffect::PlaySfx { name: name.clone() });
        }
    }
}

fn append_tween_rule_animations(
    writes: &[WriteOpTemplate],
    animation: &AnimationDef,
    direction_variant_pairs: &HashSet<(ObjectId, ObjectId)>,
    animations: &mut Vec<RuleAnimation>,
) {
    if !animation.tween.enabled {
        return;
    }
    let mut objects = Vec::new();
    let mut visual_rewrites = Vec::new();
    for write in writes {
        match write {
            WriteOpTemplate::Move { object, .. } => {
                if !objects.contains(object) {
                    objects.push(*object);
                }
            }
            WriteOpTemplate::MoveObjectSet {
                objects: moved_objects,
                ..
            } => {
                for object in moved_objects {
                    if !objects.contains(object) {
                        objects.push(*object);
                    }
                }
            }
            WriteOpTemplate::Replace {
                remove,
                add: object,
                ..
            } => {
                if !objects.contains(object) {
                    objects.push(*object);
                }
                if direction_variant_pairs.contains(&(*remove, *object)) {
                    visual_rewrites.push(RuleVisualRewrite {
                        remove: *remove,
                        add: *object,
                    });
                }
            }
            WriteOpTemplate::Add {
                component,
                offset,
                object: add,
            } => {
                let removed_objects = writes.iter().flat_map(|candidate| match candidate {
                    WriteOpTemplate::Remove {
                        component: remove_component,
                        offset: remove_offset,
                        object,
                    } if remove_component == component && remove_offset == offset => vec![*object],
                    WriteOpTemplate::RemoveObjectSet {
                        component: remove_component,
                        offset: remove_offset,
                        objects,
                        ..
                    } if remove_component == component && remove_offset == offset => {
                        objects.clone()
                    }
                    _ => Vec::new(),
                });
                for remove in removed_objects {
                    if direction_variant_pairs.contains(&(remove, *add))
                        && !visual_rewrites.iter().any(|rewrite: &RuleVisualRewrite| {
                            rewrite.remove == remove && rewrite.add == *add
                        })
                    {
                        visual_rewrites.push(RuleVisualRewrite { remove, add: *add });
                    }
                }
                if !visual_rewrites.is_empty() && !objects.contains(add) {
                    objects.push(*add);
                }
            }
            _ => {}
        }
    }
    if objects.is_empty() {
        return;
    }
    if animations.iter().any(|animation| {
        animation.trigger == RuleAnimationTrigger::Move
            && animation.name == "tween"
            && animation.objects == objects
    }) {
        return;
    }
    animations.push(RuleAnimation {
        trigger: RuleAnimationTrigger::Move,
        name: "tween".to_string(),
        objects,
        visual_rewrites,
    });
}

fn lower_inline_rewrite_syntax(
    syntax: &puzzle_authoring::UnresolvedRewriteSyntax,
    target: &puzzle_authoring::RuleStatementTargetSurface,
    rewrite_source: &str,
    source_line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<
    (
        PatternBlock,
        PatternBlock,
        Vec<EffectAst>,
        Vec<EffectAst>,
        Option<String>,
    ),
    DiagnosticReport,
> {
    let before = lower_unresolved_pattern(
        syntax.before.clone(),
        rewrite_source,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        variable_names,
    )?;
    let (after, effects, after_effects, after_call) = if let Some(after) = &syntax.after {
        let after = lower_unresolved_pattern(
            after.clone(),
            rewrite_source,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            variable_names,
        )?;
        let (after_effects, after_call) = match target {
            puzzle_authoring::RuleStatementTargetSurface::Empty => (Vec::new(), None),
            puzzle_authoring::RuleStatementTargetSurface::Call { name, .. } => {
                (Vec::new(), Some(name.clone()))
            }
            puzzle_authoring::RuleStatementTargetSurface::Effect { span } => (
                parse_rewrite_effect(&source_line[span.clone()], source_line)?,
                None,
            ),
            puzzle_authoring::RuleStatementTargetSurface::Invalid { span } => {
                return Err(parse_error(
                    source_line,
                    &format!("invalid rewrite suffix: {}", &source_line[span.clone()]),
                ));
            }
        };
        (after, Vec::new(), after_effects, after_call)
    } else {
        let effects = match target {
            puzzle_authoring::RuleStatementTargetSurface::Effect { span } => {
                parse_rewrite_effect(&source_line[span.clone()], source_line)?
            }
            puzzle_authoring::RuleStatementTargetSurface::Empty => {
                return Err(parse_error(source_line, "rewrite target cannot be empty"));
            }
            puzzle_authoring::RuleStatementTargetSurface::Call { name, .. } => {
                return Err(parse_error(
                    source_line,
                    &format!("routine call target `{name}` must lower as a conditional call"),
                ));
            }
            puzzle_authoring::RuleStatementTargetSurface::Invalid { span } => {
                return Err(parse_error(
                    source_line,
                    &format!("invalid rewrite target: {}", &source_line[span.clone()]),
                ));
            }
        };
        (before.clone(), effects, Vec::new(), None)
    };

    Ok((before, after, effects, after_effects, after_call))
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedRewriteEffect {
    pub(crate) surface: SurfaceRewriteEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

pub(crate) fn parse_rewrite_effect(
    suffix: &str,
    line: &str,
) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let parsed = parse_rewrite_effect_with_semantic_tokens(suffix, line)?;
    debug_assert!(
        parsed
            .semantic_tokens
            .iter()
            .all(|token| token.start < token.end)
    );
    Ok(parsed.surface.effects)
}

fn parse_rewrite_effect_with_semantic_tokens(
    suffix: &str,
    line: &str,
) -> Result<ParsedRewriteEffect, DiagnosticReport> {
    let surface = parse_surface_rewrite_effect(suffix, line)?;
    let semantic_tokens = project_surface_semantic_tokens(&surface.document.semantic_tokens);
    Ok(ParsedRewriteEffect {
        surface,
        semantic_tokens,
    })
}

fn parse_surface_rewrite_effect(
    suffix: &str,
    line: &str,
) -> Result<SurfaceRewriteEffect, DiagnosticReport> {
    let tokens = source_line_tokens(strip_line_comment(suffix), 0);
    let effects = parse_rewrite_effect_value(suffix, line)?;
    let recognition = rewrite_effect_parser_recognition(&tokens);
    let document = parser_recognition_surface_document(&recognition);
    Ok(SurfaceRewriteEffect { effects, document })
}

fn parse_rewrite_effect_value(
    suffix: &str,
    line: &str,
) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let suffix = suffix.trim();
    if suffix.strip_prefix("emit ").is_some() {
        return Err(parse_error(
            line,
            "`emit` is obsolete; write the presentation effect directly",
        ));
    }
    if let Some(text) = suffix.strip_prefix("message ") {
        let text = text.trim();
        if let Some(text) = parse_quoted_text(text) {
            return Ok(vec![EffectAst::PresentComponent {
                text,
                literal: true,
            }]);
        }
        if parse_view_path(text).is_some() {
            return Ok(vec![EffectAst::PresentComponent {
                text: text.to_string(),
                literal: false,
            }]);
        }
        return Err(parse_error(
            line,
            "message effect must be: message \"text\" or message <path>",
        ));
    }

    let tokens = split_header_tokens(suffix);
    match tokens.as_slice() {
        [visual] if puzzle_authoring::is_visual_emission_name(visual) => {
            Ok(vec![EffectAst::EmitVisual {
                name: visual[1..].to_string(),
            }])
        }
        [command] if command.eq_ignore_ascii_case("cancel") => Ok(vec![EffectAst::Cancel]),
        [command] if command.eq_ignore_ascii_case("win") => Ok(vec![EffectAst::Win]),
        [command] if command.eq_ignore_ascii_case("restart") => Ok(vec![EffectAst::Restart]),
        [command] if command.eq_ignore_ascii_case("next_level") => Ok(vec![EffectAst::NextLevel]),
        [command] if command.eq_ignore_ascii_case("again") => Ok(vec![EffectAst::Again]),
        [command] if command.eq_ignore_ascii_case("checkpoint") => Ok(vec![EffectAst::Checkpoint]),
        [command] if command.eq_ignore_ascii_case("clear_checkpoint") => {
            Ok(vec![EffectAst::ClearCheckpoint])
        }
        ["goto", ..] | ["start", ..] => Ok(vec![EffectAst::Scene(parse_puzzle_scene_effect(
            suffix, line,
        )?)]),
        tokens
            if tokens.len() > 2
                && tokens
                    .iter()
                    .any(|token| is_rewrite_effect_command_token(token)) =>
        {
            parse_simple_rewrite_effects(tokens, line)
        }
        ["wait"] => Ok(vec![EffectAst::WaitAnimation]),
        ["wait", "animation"] | ["wait", "tween"] => Ok(vec![EffectAst::WaitAnimation]),
        ["wait", duration] => Ok(vec![EffectAst::Wait {
            milliseconds: parse_wait_duration_ms(duration, line)?,
        }]),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(vec![EffectAst::PlaySfx {
                name: (*name).to_string(),
            }])
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::PlayMusic {
                name: (*name).to_string(),
            }])
        }
        ["pause_music"] => Ok(vec![EffectAst::PauseMusic { name: None }]),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::PauseMusic {
                name: Some((*name).to_string()),
            }])
        }
        ["resume_music"] => Ok(vec![EffectAst::ResumeMusic { name: None }]),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::ResumeMusic {
                name: Some((*name).to_string()),
            }])
        }
        ["stop_music"] => Ok(vec![EffectAst::StopMusic { name: None }]),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::StopMusic {
                name: Some((*name).to_string()),
            }])
        }
        [name, op, value] if is_variable_update_operator(op) => {
            Ok(vec![EffectAst::UpdateVariable {
                name: (*name).to_string(),
                op: parse_variable_update_op(op, line)?,
                value: parse_variable_update_value(value, line)?,
            }])
        }
        _ => Err(parse_error(
            line,
            "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, or <variable> <op> <value>",
        )),
    }
}

fn parse_puzzle_scene_effect(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    let effect = parse_scene_effect(value, line)?;
    validate_puzzle_scene_effect(&effect, line)?;
    Ok(effect)
}

fn validate_puzzle_scene_effect(effect: &SceneEffect, line: &str) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::Goto { .. } | SceneEffect::Reset { .. } => Ok(()),
        SceneEffect::Sequence { effects } => {
            for effect in effects {
                validate_puzzle_scene_effect(effect, line)?;
            }
            Ok(())
        }
        _ => Err(parse_error(
            line,
            "puzzle statement scene effects are limited to `goto <scene>` and `start <scene>`",
        )),
    }
}

fn parse_simple_rewrite_effects(
    tokens: &[&str],
    line: &str,
) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let mut effects = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if puzzle_authoring::is_visual_emission_name(tokens[index]) {
            effects.push(EffectAst::EmitVisual {
                name: tokens[index][1..].to_string(),
            });
            index += 1;
            continue;
        }
        match tokens[index].to_ascii_lowercase().as_str() {
            "cancel" => {
                effects.push(EffectAst::Cancel);
                index += 1;
            }
            "win" => {
                effects.push(EffectAst::Win);
                index += 1;
            }
            "restart" => {
                effects.push(EffectAst::Restart);
                index += 1;
            }
            "next_level" => {
                effects.push(EffectAst::NextLevel);
                index += 1;
            }
            "again" => {
                effects.push(EffectAst::Again);
                index += 1;
            }
            "checkpoint" => {
                effects.push(EffectAst::Checkpoint);
                index += 1;
            }
            "clear_checkpoint" => {
                effects.push(EffectAst::ClearCheckpoint);
                index += 1;
            }
            "wait" => {
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("animation") || token.eq_ignore_ascii_case("tween")
                }) {
                    effects.push(EffectAst::WaitAnimation);
                    index += 2;
                } else if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    effects.push(EffectAst::Wait {
                        milliseconds: parse_wait_duration_ms(tokens[index + 1], line)?,
                    });
                    index += 2;
                } else {
                    effects.push(EffectAst::WaitAnimation);
                    index += 1;
                }
            }
            "sfx" => {
                let Some(name) = tokens.get(index + 1) else {
                    return Err(parse_error(line, "sfx effect must include a name"));
                };
                validate_qualified_identifier(name, line, "sfx sounds name")?;
                effects.push(EffectAst::PlaySfx {
                    name: (*name).to_string(),
                });
                index += 2;
            }
            "play_music" => {
                let Some(name) = tokens.get(index + 1) else {
                    return Err(parse_error(line, "play_music effect must include a name"));
                };
                validate_qualified_identifier(name, line, "music sounds name")?;
                effects.push(EffectAst::PlayMusic {
                    name: (*name).to_string(),
                });
                index += 2;
            }
            "pause_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::PauseMusic { name });
            }
            "resume_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::ResumeMusic { name });
            }
            "stop_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::StopMusic { name });
            }
            name if index + 2 < tokens.len() && is_variable_update_operator(tokens[index + 1]) => {
                effects.push(EffectAst::UpdateVariable {
                    name: name.to_string(),
                    op: parse_variable_update_op(tokens[index + 1], line)?,
                    value: parse_variable_update_value(tokens[index + 2], line)?,
                });
                index += 3;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, or <variable> <op> <value>",
                ));
            }
        }
    }
    Ok(effects)
}

fn is_rewrite_effect_command_token(token: &str) -> bool {
    if puzzle_authoring::is_visual_emission_name(token) {
        return true;
    }
    matches!(
        token.to_ascii_lowercase().as_str(),
        "cancel"
            | "win"
            | "restart"
            | "next_level"
            | "again"
            | "checkpoint"
            | "clear_checkpoint"
            | "wait"
            | "sfx"
            | "play_music"
            | "pause_music"
            | "resume_music"
            | "stop_music"
    )
}

fn parse_variable_update_op(op: &str, line: &str) -> Result<VariableUpdateOp, DiagnosticReport> {
    match op {
        "=" => Ok(VariableUpdateOp::Set),
        "+=" => Ok(VariableUpdateOp::Add),
        "-=" => Ok(VariableUpdateOp::Subtract),
        "*=" => Ok(VariableUpdateOp::Multiply),
        "/=" => Ok(VariableUpdateOp::Divide),
        "%=" => Ok(VariableUpdateOp::Remainder),
        _ => Err(parse_error(line, "unknown variable update operator")),
    }
}

fn parse_variable_update_value(
    token: &str,
    line: &str,
) -> Result<VariableValueAst, DiagnosticReport> {
    if let Ok(value) = parse_variable_value(token, line) {
        return Ok(VariableValueAst::Literal(value));
    }
    validate_tag_capture_reference(token, line)?;
    Ok(VariableValueAst::TagCapture(token.to_string()))
}

fn validate_tag_capture_reference(token: &str, line: &str) -> Result<(), DiagnosticReport> {
    if puzzle_authoring::is_selector_wildcard(token) {
        return Ok(());
    }
    if let Some(label) = token
        .strip_prefix(puzzle_authoring::SELECTOR_WILDCARD)
        .and_then(|suffix| suffix.strip_prefix('#'))
    {
        return validate_tag_capture_label(label, line);
    }
    if let Some((name, label)) = token.split_once('#') {
        if !is_identifier(name) {
            return Err(parse_error(
                line,
                &format!(
                    "tag capture reference must be {0}, {0}#label, name, or name#label",
                    puzzle_authoring::SELECTOR_WILDCARD
                ),
            ));
        }
        return validate_tag_capture_label(label, line);
    }
    if is_identifier(token) {
        return Ok(());
    }
    Err(parse_error(
        line,
        "variable update value must be true, false, integer, or tag capture reference",
    ))
}

fn neutral_direction(directions: &[OrientationEnvironment]) -> OrientationEnvironment {
    directions
        .iter()
        .copied()
        .find(|environment| environment.primary_name == "right")
        .unwrap_or_else(|| SpatialDomain::new(ModelDimension::Two).neutral())
}

fn rewrite_requires_implicit_cardinal_expansion(
    rewrite: &OrientedRewriteAst,
    value_sets: &HashMap<String, Vec<String>>,
) -> bool {
    pattern_block_requires_implicit_cardinal_expansion(&rewrite.before, value_sets)
        || pattern_block_requires_implicit_cardinal_expansion(&rewrite.after, value_sets)
}

fn pattern_block_requires_implicit_cardinal_expansion(
    block: &PatternBlock,
    value_sets: &HashMap<String, Vec<String>>,
) -> bool {
    block.components.iter().any(|component| {
        component.rows.len() > 1
            || component.rows.iter().any(|row| {
                row.len() > 1
                    || row.iter().any(|part| match part {
                        BlockPart::Cell(cell) => {
                            block_cell_has_relative_direction(cell, value_sets)
                        }
                        BlockPart::Ellipsis => true,
                    })
            })
    })
}

fn block_cell_has_relative_direction(
    cell: &BlockCell,
    value_sets: &HashMap<String, Vec<String>>,
) -> bool {
    cell.require
        .iter()
        .chain(&cell.forbid)
        .any(|selector| selector_has_relative_direction(selector, value_sets))
}

fn selector_has_relative_direction(
    selector: &ObjectSelector,
    value_sets: &HashMap<String, Vec<String>>,
) -> bool {
    if !selector.relative_constraints.is_empty() {
        return true;
    }
    selector.mark.iter().any(|mark| {
        mark.name.is_empty()
            && mark.value.as_deref().is_some_and(|value| {
                puzzle_authoring::mark_sugar_kind(value)
                    == Some(puzzle_authoring::MarkSugarKind::Movement)
                    && (parse_relative_direction_value(value).is_some()
                        || value_sets.get(value).is_some_and(|values| {
                            values
                                .iter()
                                .any(|value| parse_relative_direction_value(value).is_some())
                        }))
            })
    })
}

fn resolve_relative_selectors_in_block(
    block: &PatternBlock,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<PatternBlock, DiagnosticReport> {
    let mut block = block.clone();
    for component in &mut block.components {
        for row in &mut component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                for selector in &mut cell.require {
                    resolve_relative_selector(selector, direction, direction_expanded, line)?;
                }
                for selector in &mut cell.forbid {
                    resolve_relative_selector(selector, direction, direction_expanded, line)?;
                }
                cell.forbid
                    .retain(|selector| !selector.alternatives.is_empty());
            }
        }
    }
    Ok(block)
}

fn block_has_unavailable_required_selector(block: &PatternBlock) -> bool {
    block.components.iter().any(|component| {
        component.rows.iter().flatten().any(|part| {
            let BlockPart::Cell(cell) = part else {
                return false;
            };
            cell.require
                .iter()
                .any(|selector| selector.alternatives.is_empty())
        })
    })
}

fn resolve_relative_selector(
    selector: &mut ObjectSelector,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if selector.relative_constraints.is_empty() {
        return Ok(());
    }
    if !direction_expanded {
        return Err(parse_error(
            line,
            "relative direction selector tag requires an oriented rule",
        ));
    }
    for constraint in &selector.relative_constraints {
        let absolute =
            resolve_relative_direction(constraint.relative, direction, direction_expanded, line)?;
        let value = direction_tag_name(direction, absolute, line)?;
        let Some(allowed) = constraint.alternatives_by_direction.get(value) else {
            selector.alternatives.clear();
            break;
        };
        selector
            .alternatives
            .retain(|object| allowed.contains(object));
    }
    selector.relative_constraints.clear();
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OffsetTemplate {
    oriented_x: i16,
    oriented_y: i16,
    gap_terms: Vec<u16>,
}

#[derive(Clone, Debug)]
struct PatternComponentTemplate {
    cells: Vec<MatchCellTemplate>,
    gap_count: u16,
}

#[derive(Clone, Debug)]
struct MatchCellTemplate {
    offset: OffsetTemplate,
    require_null: bool,
    require_objects: Vec<ObjectId>,
    require_object_sets: Vec<ObjectSetMatcher>,
    forbid_objects: Vec<ObjectId>,
    require_mark: Vec<MarkPatternTemplate>,
    require_object_set_mark: Vec<ObjectSetMarkPatternTemplate>,
    forbid_mark: Vec<MarkPatternTemplate>,
    forbid_object_set_mark: Vec<ObjectSetMarkPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MarkPatternTemplate {
    object: ObjectId,
    mark: MarkId,
    value: Option<MarkValueTemplate>,
    match_value: MarkValueMatch,
    is_flag: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjectSetMarkPatternTemplate {
    binding: u16,
    mark: MarkId,
    value: Option<MarkValueTemplate>,
    match_value: MarkValueMatch,
    is_flag: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MarkValueTemplate {
    Literal(i64),
    Relative(RelativeDirection),
}

type RelativeDirection = puzzle_authoring::RelativeDirection;

#[derive(Clone, Debug)]
enum WriteOpTemplate {
    Add {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
    },
    AddObjectSet {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
    },
    Remove {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
    },
    RemoveObjectSet {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        objects: Vec<ObjectId>,
    },
    Replace {
        component: u16,
        offset: OffsetTemplate,
        remove: ObjectId,
        add: ObjectId,
    },
    Move {
        component: u16,
        from_offset: OffsetTemplate,
        to_offset: OffsetTemplate,
        object: ObjectId,
    },
    MoveObjectSet {
        component: u16,
        from_offset: OffsetTemplate,
        to_offset: OffsetTemplate,
        binding: u16,
        objects: Vec<ObjectId>,
    },
    SetMark {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        mark: MarkId,
        value: Option<MarkValueTemplate>,
    },
    SetObjectSetMark {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        mark: MarkId,
        value: Option<MarkValueTemplate>,
    },
    RemoveMark {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        mark: MarkId,
        value: Option<MarkValueTemplate>,
        match_value: MarkValueMatch,
    },
    RemoveObjectSetMark {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        mark: MarkId,
        value: Option<MarkValueTemplate>,
        match_value: MarkValueMatch,
    },
}

#[derive(Clone, Debug)]
struct PatternBlock {
    components: Vec<BlockComponent>,
}

#[derive(Clone, Debug)]
struct BlockComponent {
    rows: Vec<Vec<BlockPart>>,
}

#[derive(Clone, Debug)]
enum BlockPart {
    Cell(BlockCell),
    Ellipsis,
}

#[derive(Clone, Debug, Default)]
struct BlockCell {
    keep: bool,
    require_null: bool,
    require: Vec<ObjectSelector>,
    forbid: Vec<ObjectSelector>,
    require_cell_mark: Vec<SelectorMark>,
    forbid_cell_mark: Vec<SelectorMark>,
}

#[derive(Clone, Debug)]
struct ObjectSelector {
    token: String,
    alternatives: Vec<ObjectId>,
    transform: Option<SelectorTransform>,
    family_wildcard: Option<FamilyWildcardSelector>,
    correspondence_source_token: Option<String>,
    relative_constraints: Vec<RelativeSelectorConstraint>,
    capture_requirements: HashMap<ObjectId, Vec<CaptureSelectorRequirement>>,
    dynamic_guards: HashMap<ObjectId, Vec<DynamicSelectorGuard>>,
    tag_captures: HashMap<ObjectId, Vec<TagCapture>>,
    mark: Vec<SelectorMark>,
    occurrence_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TagCapture {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Default)]
struct TagCaptureValues {
    values: HashMap<String, TagCaptureValue>,
}

#[derive(Clone, Debug)]
struct TagCaptureValue {
    value: String,
    duplicate: bool,
    conflict: bool,
}

impl TagCaptureValues {
    fn insert(&mut self, capture: &TagCapture) {
        self.values
            .entry(capture.key.clone())
            .and_modify(|existing| {
                if capture.key.contains('#') {
                    if existing.value != capture.value {
                        existing.conflict = true;
                    }
                } else {
                    existing.duplicate = true;
                }
            })
            .or_insert_with(|| TagCaptureValue {
                value: capture.value.clone(),
                duplicate: false,
                conflict: false,
            });
    }

    fn has_conflict(&self) -> bool {
        self.values.values().any(|value| value.conflict)
    }

    fn resolve(&self, key: &str, line: &str) -> Result<i64, DiagnosticReport> {
        let value = self.resolve_text(key, line)?;
        parse_variable_value(&value, line).map_err(|_| {
            parse_error(
                line,
                "tag capture values used in var updates must be true, false, or integers",
            )
        })
    }

    fn resolve_text(&self, key: &str, line: &str) -> Result<String, DiagnosticReport> {
        let Some(value) = self.values.get(key) else {
            return Err(parse_error(
                line,
                &format!("unknown tag capture reference: {key}"),
            ));
        };
        if value.conflict {
            return Err(parse_error(
                line,
                &format!("tag capture reference `{key}` is conflicting"),
            ));
        }
        if value.duplicate {
            return Err(parse_error(
                line,
                &format!("tag capture reference `{key}` is ambiguous"),
            ));
        }
        Ok(value.value.clone())
    }
}

#[derive(Clone, Debug)]
struct RelativeSelectorConstraint {
    relative: RelativeDirection,
    alternatives_by_direction: HashMap<String, Vec<ObjectId>>,
}

#[derive(Clone, Debug)]
enum CaptureSelectorRequirement {
    Direct {
        key: String,
        value: String,
    },
    Mapped {
        key: String,
        map_name: String,
        value: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DynamicSelectorGuard {
    name: String,
    variable: VariableId,
    value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct OccurrenceKey {
    token: String,
    ordinal: usize,
}

#[derive(Clone, Debug)]
struct ResolvedObjectOccurrence {
    token: String,
    matched: ResolvedObjectMatch,
    key: Option<OccurrenceKey>,
    from_multi_selector: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedObjectMatch {
    Object(ObjectId),
    ObjectSet {
        binding: u16,
        layer: LayerId,
        objects: Vec<ObjectId>,
    },
}

impl ResolvedObjectMatch {
    fn possible_objects(&self) -> Vec<ObjectId> {
        match self {
            ResolvedObjectMatch::Object(object) => vec![*object],
            ResolvedObjectMatch::ObjectSet { objects, .. } => objects.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct OccurrencePlacement {
    component: u16,
    offset: OffsetTemplate,
    matched: ResolvedObjectMatch,
    require_mark: Vec<MarkPatternTemplate>,
    require_object_set_mark: Vec<ObjectSetMarkPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RewritePosition2 {
    component: u16,
    offset: OffsetTemplate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RewriteMark2 {
    Object(MarkPatternTemplate),
    ObjectSet(ObjectSetMarkPatternTemplate),
}

#[derive(Clone, Debug)]
struct SelectorTransform {
    source_token: String,
    mapped_objects: HashMap<ObjectId, ObjectId>,
    preserves_once_group: bool,
}

#[derive(Clone, Debug)]
struct FamilyWildcardSelector {
    mapped_objects: HashMap<ObjectId, ObjectId>,
}

fn lower_unresolved_pattern(
    syntax: puzzle_authoring::UnresolvedPatternSyntax,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<PatternBlock, DiagnosticReport> {
    let components = syntax
        .components
        .into_iter()
        .map(|component| {
            let rows = component
                .lines
                .into_iter()
                .map(|pattern_line| match pattern_line {
                    puzzle_authoring::UnresolvedPatternLineSyntax::Blank => Err(parse_error(
                        line,
                        "blank lines inside patterns require a spatial dimension that defines them",
                    )),
                    puzzle_authoring::UnresolvedPatternLineSyntax::Cells(parts) => parts
                        .into_iter()
                        .map(|part| match part {
                            puzzle_authoring::UnresolvedPatternPartSyntax::Ellipsis => {
                                Ok(BlockPart::Ellipsis)
                            }
                            puzzle_authoring::UnresolvedPatternPartSyntax::Cell(cell) => {
                                lower_unresolved_cell(
                                    cell,
                                    line,
                                    object_names,
                                    object_schemas,
                                    value_sets,
                                    maps,
                                    object_groups,
                                    variable_names,
                                )
                                .map(BlockPart::Cell)
                            }
                        })
                        .collect(),
                })
                .collect::<Result<Vec<Vec<BlockPart>>, DiagnosticReport>>()?;
            validate_rectangular_ellipsis_layout(&rows, line)?;
            Ok(BlockComponent { rows })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    Ok(PatternBlock { components })
}

fn lower_pattern_source(
    source: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<PatternBlock, DiagnosticReport> {
    let syntax = puzzle_authoring::parse_unresolved_pattern_syntax(source)
        .map_err(|error| parse_error(line, error.message()))?;
    lower_unresolved_pattern(
        syntax,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        variable_names,
    )
}

fn lower_unresolved_cell(
    cell: puzzle_authoring::UnresolvedCellSyntax,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<BlockCell, DiagnosticReport> {
    let mut parsed = BlockCell {
        keep: cell.keep,
        require_null: cell.require_null,
        ..BlockCell::default()
    };
    for subject in cell.require {
        match subject {
            puzzle_authoring::UnresolvedCellSubjectSyntax::CellMarks(marks) => {
                parsed.require_cell_mark.extend(marks);
            }
            puzzle_authoring::UnresolvedCellSubjectSyntax::Selector(selector) => {
                parsed.require.push(resolve_object_selector_syntax(
                    selector,
                    line,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    variable_names,
                )?);
            }
        }
    }
    for subject in cell.forbid {
        match subject {
            puzzle_authoring::UnresolvedCellSubjectSyntax::CellMarks(marks) => {
                parsed.forbid_cell_mark.extend(marks);
            }
            puzzle_authoring::UnresolvedCellSubjectSyntax::Selector(selector) => {
                parsed.forbid.push(resolve_object_selector_syntax(
                    selector,
                    line,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    variable_names,
                )?);
            }
        }
    }
    Ok(parsed)
}

fn validate_rectangular_ellipsis_layout(
    rows: &[Vec<BlockPart>],
    line: &str,
) -> Result<(), DiagnosticReport> {
    if rows.len() <= 1
        || !rows
            .iter()
            .flatten()
            .any(|part| matches!(part, BlockPart::Ellipsis))
    {
        return Ok(());
    }

    let first = rows
        .first()
        .expect("canonical pattern syntax rejected empty blocks");
    for row in rows.iter().skip(1) {
        let same_ellipsis_columns = row.len() == first.len()
            && row.iter().zip(first).all(|(left, right)| {
                matches!(
                    (left, right),
                    (BlockPart::Ellipsis, BlockPart::Ellipsis)
                        | (BlockPart::Cell(_), BlockPart::Cell(_))
                )
            });
        if !same_ellipsis_columns {
            return Err(parse_error(
                line,
                "ellipsis inside rectangular blocks requires each row to use the same ellipsis columns",
            ));
        }
    }

    Ok(())
}

fn resolve_object_selector(
    selector: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let syntax = puzzle_authoring::parse_selector_syntax(selector)
        .map_err(|error| parse_error(line, error.message()))?;
    resolve_object_selector_syntax(
        syntax,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        variable_names,
    )
}

fn resolve_object_selector_syntax(
    syntax: puzzle_authoring::SelectorSyntax,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let selector_parts = std::iter::once(syntax.base.as_str())
        .chain(syntax.tags.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let mark = syntax.marks.clone();
    let occurrence_label = syntax.occurrence_label.clone();
    let selector = syntax.selector.as_str();
    let token = labeled_selector_token(selector, occurrence_label.as_deref());
    if !selector.contains(':')
        && let Some(object) = object_names.get(selector).copied()
    {
        return Ok(ObjectSelector {
            token,
            alternatives: vec![object],
            transform: None,
            family_wildcard: None,
            correspondence_source_token: None,
            relative_constraints: Vec::new(),
            capture_requirements: HashMap::new(),
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            mark,
            occurrence_label,
        });
    }

    if let Some(objects) = object_groups.get(selector) {
        return Ok(ObjectSelector {
            token,
            alternatives: objects.clone(),
            transform: None,
            family_wildcard: None,
            correspondence_source_token: None,
            relative_constraints: Vec::new(),
            capture_requirements: HashMap::new(),
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            mark,
            occurrence_label,
        });
    }

    let parts = selector_parts;
    if parts.len() == 1 && puzzle_authoring::is_selector_wildcard(parts[0]) {
        return resolve_any_object_selector(
            token,
            mark,
            occurrence_label,
            line,
            object_names,
            object_schemas,
        );
    }
    if parts
        .first()
        .copied()
        .is_some_and(puzzle_authoring::is_selector_wildcard)
    {
        return resolve_schema_family_wildcard_selector(
            &parts,
            token,
            mark,
            occurrence_label,
            line,
            object_schemas,
            value_sets,
            variable_names,
        );
    }
    let Some(schema) = object_schemas.get(parts[0]) else {
        if parts.len() > 1 && value_sets.contains_key(parts[0]) {
            return resolve_qualified_value_set_selector(
                selector,
                token,
                mark,
                occurrence_label,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                variable_names,
            );
        }
        return Err(parse_error(line, "unknown object selector"));
    };

    validate_schema_selector_arity(&parts, schema, line, "object selector")?;
    if parts.len() == 1 {
        return Err(parse_error(
            line,
            &format!(
                "object selector for variants must use :{} or explicit variant tags",
                puzzle_authoring::SELECTOR_WILDCARD
            ),
        ));
    }

    let mut source_token_parts = Vec::new();
    let constraints = schema
        .axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let Some(value) = schema_selector_part(&parts, schema, index) else {
                source_token_parts.push(axis.clone());
                return Ok(None);
            };
            let (value, tag_capture_key) =
                selector_tag_capture_key(value, axis, schema.axes.len(), line)?;
            if puzzle_authoring::is_selector_wildcard(value) {
                source_token_parts.push(puzzle_authoring::SELECTOR_WILDCARD.to_string());
                return Ok(tag_capture_key.map(|key| SelectorConstraint::Capture {
                    axis_index: index,
                    key,
                }));
            }
            if let Some(value_type @ (ValueType::Angle | ValueType::Vec2)) =
                schema.axis_types.get(index).copied().flatten()
                && let Some(expr) =
                    parse_axis_computed_selector_value(value, axis, value_type, line)?
            {
                source_token_parts.push(axis.clone());
                return Ok(Some(SelectorConstraint::AxisComputed {
                    axis_index: index,
                    expr,
                }));
            }
            if let Some(relative) = parse_relative_direction_value(value) {
                if schema.axis_types.get(index).copied().flatten() != Some(ValueType::Direction) {
                    return Err(parse_error(
                        line,
                        "relative direction selector tag requires a direction-typed tag slot",
                    ));
                }
                source_token_parts.push((*value).to_string());
                return Ok(Some(SelectorConstraint::Relative {
                    axis_index: index,
                    relative,
                }));
            }
            let axis_type = schema.axis_types.get(index).copied().flatten();
            if matches!(
                axis_type,
                Some(ValueType::Angle | ValueType::Vec2 | ValueType::Frame3)
            ) && value != axis
            {
                let value = normalize_axis_literal(value, schema, index, line)?;
                source_token_parts.push(value.clone());
                return Ok(Some(SelectorConstraint::Fixed(value)));
            }
            if matches!(axis_type, Some(ValueType::Int | ValueType::Rational))
                && parse_rational_value(value, line).is_ok()
            {
                let value = normalize_axis_literal(value, schema, index, line)?;
                source_token_parts.push(value.clone());
                return Ok(Some(SelectorConstraint::Fixed(value)));
            }
            let expr = parse_value_expr(value, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                if variable_names.contains_key(axis) {
                    return Err(ambiguous_selector_tag_error(axis, parts[0], axis, line));
                }
                source_token_parts.push(axis.clone());
                Ok(Some(SelectorConstraint::Capture {
                    axis_index: index,
                    key: tag_capture_key.unwrap_or_else(|| axis.clone()),
                }))
            } else if let ValueExpr::MapCall { arg, .. } = &expr {
                let arg_axis = map_argument_axis(arg);
                let axis_values = schema_axis_values(schema, index)?;
                if arg_axis != axis {
                    let Some(values) = value_sets.get(arg_axis) else {
                        return Err(parse_error(
                            line,
                            "map argument must match selector tag set",
                        ));
                    };
                    validate_selector_subset(arg_axis, values, &axis_values, parts[0], axis, line)?;
                }
                let ValueExpr::MapCall { name, .. } = &expr else {
                    unreachable!("map call branch only handles map calls");
                };
                let map = maps
                    .get(name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                if map.axis != *axis {
                    return Err(parse_error(line, "map tag set must match argument tag set"));
                }
                source_token_parts.push(arg.clone());
                Ok(Some(SelectorConstraint::Mapped {
                    axis_index: index,
                    expr,
                }))
            } else if let ValueExpr::Binding(name) = &expr {
                let axis_values = schema_axis_values(schema, index)?;
                let names_axis_value = axis_values.contains(name);
                let names_value_set = value_sets.contains_key(name);
                let variable = variable_names.get(name).copied();
                if (names_axis_value && names_value_set)
                    || (variable.is_some() && (names_axis_value || names_value_set))
                {
                    return Err(ambiguous_selector_tag_error(name, parts[0], axis, line));
                }
                if let Some(values) = value_sets.get(name) {
                    validate_selector_subset(name, values, &axis_values, parts[0], axis, line)?;
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::ValueSet(values.clone())))
                } else if names_axis_value {
                    let value = normalize_axis_literal(name, schema, index, line)?;
                    source_token_parts.push(value.clone());
                    Ok(Some(SelectorConstraint::Fixed(value)))
                } else if let Some(variable) = variable {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::DynamicVariable {
                        axis_index: index,
                        name: name.clone(),
                        variable,
                    }))
                } else {
                    let value = normalize_axis_literal(name, schema, index, line)?;
                    source_token_parts.push(value.clone());
                    Ok(Some(SelectorConstraint::Fixed(value)))
                }
            } else {
                let value = normalize_axis_literal(value, schema, index, line)?;
                source_token_parts.push(value.clone());
                Ok(Some(SelectorConstraint::Fixed(value)))
            }
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    validate_axis_computed_source_constraints(&constraints, schema, &source_token_parts, line)?;

    let alternatives = schema
        .variants
        .iter()
        .filter(|variant| {
            constraints
                .iter()
                .enumerate()
                .all(|(index, constraint)| match constraint {
                    Some(SelectorConstraint::Fixed(value)) => variant.values[index] == *value,
                    Some(SelectorConstraint::ValueSet(values)) => {
                        values.contains(&variant.values[index])
                    }
                    Some(SelectorConstraint::Relative { .. }) => true,
                    Some(SelectorConstraint::Capture { .. }) => true,
                    Some(SelectorConstraint::Mapped { .. })
                    | Some(SelectorConstraint::AxisComputed { .. })
                    | Some(SelectorConstraint::DynamicVariable { .. })
                    | None => true,
                })
        })
        .map(|variant| variant.object)
        .collect::<Vec<_>>();

    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }
    let relative_constraints = relative_selector_constraints(&constraints, schema, &alternatives)?;
    let capture_requirements = capture_selector_requirements(&constraints, schema, &alternatives)?;
    let correspondence_source_token = constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::Relative { .. })))
        .then(|| {
            let source_parts = constraints
                .iter()
                .zip(&source_token_parts)
                .map(|(constraint, value)| {
                    if matches!(constraint, Some(SelectorConstraint::Relative { .. })) {
                        puzzle_authoring::SELECTOR_WILDCARD.to_string()
                    } else {
                        value.clone()
                    }
                })
                .collect::<Vec<_>>();
            labeled_selector_token(
                &format!("{}:{}", parts[0], source_parts.join(":")),
                occurrence_label.as_deref(),
            )
        });

    if constraints
        .iter()
        .any(selector_constraint_needs_occurrence_transform)
    {
        let source_token = labeled_selector_token(
            &format!("{}:{}", parts[0], source_token_parts.join(":")),
            occurrence_label.as_deref(),
        );
        let preserves_once_group = constraints
            .iter()
            .any(|constraint| matches!(constraint, Some(SelectorConstraint::AxisComputed { .. })));
        let mut mapped_objects = HashMap::new();
        let mut target_objects = Vec::new();
        for source in &schema.variants {
            let mut values = source.values.clone();
            for constraint in constraints.iter().flatten() {
                match constraint {
                    SelectorConstraint::Mapped { axis_index, expr } => {
                        let ValueExpr::MapCall { arg, .. } = expr else {
                            unreachable!("mapped selector constraint must contain a map call");
                        };
                        let mut env = ValueEnv::default();
                        env.bind(arg, &schema.axes[*axis_index], &source.values[*axis_index]);
                        values[*axis_index] = eval_bound_value_expr(expr, &env, maps, line)?;
                    }
                    SelectorConstraint::AxisComputed { axis_index, expr } => {
                        values[*axis_index] =
                            eval_axis_computed_selector_value(expr, schema, source, line)?;
                    }
                    _ => {}
                }
            }
            let target = schema
                .variants
                .iter()
                .find(|variant| variant.values == values)
                .ok_or_else(|| parse_error(line, "mapped selector target not found"))?
                .object;
            mapped_objects.insert(source.object, target);
            if !target_objects.contains(&target) {
                target_objects.push(target);
            }
        }
        return Ok(ObjectSelector {
            token,
            alternatives: target_objects,
            transform: Some(SelectorTransform {
                source_token,
                mapped_objects,
                preserves_once_group,
            }),
            family_wildcard: None,
            correspondence_source_token,
            relative_constraints,
            capture_requirements: capture_requirements.clone(),
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            mark,
            occurrence_label,
        });
    }

    let dynamic_guards = dynamic_selector_guards(&constraints, schema, line)?;
    let tag_captures = selector_tag_captures(&constraints, schema, &alternatives)?;
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard: None,
        correspondence_source_token,
        relative_constraints,
        capture_requirements,
        dynamic_guards,
        tag_captures,
        mark,
        occurrence_label,
    })
}

fn resolve_any_object_selector(
    token: String,
    mark: Vec<SelectorMark>,
    occurrence_label: Option<String>,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let mut alternatives = object_names.values().copied().collect::<Vec<_>>();
    alternatives.extend(schema_wildcard_alternatives(object_schemas, |_, _| true));
    alternatives.sort_by_key(|object| object.0);
    alternatives.dedup();
    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard: None,
        correspondence_source_token: None,
        relative_constraints: Vec::new(),
        capture_requirements: HashMap::new(),
        dynamic_guards: HashMap::new(),
        tag_captures: HashMap::new(),
        mark,
        occurrence_label,
    })
}

fn resolve_qualified_value_set_selector(
    selector: &str,
    token: String,
    mark: Vec<SelectorMark>,
    occurrence_label: Option<String>,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let (name, suffix) = selector
        .split_once(':')
        .ok_or_else(|| parse_error(line, "qualified tag selector must use a suffix"))?;
    if suffix.is_empty() {
        return Err(parse_error(
            line,
            "qualified tag selector suffix must not be empty",
        ));
    }
    let atoms = value_sets
        .get(name)
        .ok_or_else(|| parse_error(line, "unknown tag set selector"))?;

    let mut alternatives = Vec::new();
    let mut dynamic_guards = HashMap::<ObjectId, Vec<DynamicSelectorGuard>>::new();
    let mut mapped_objects = HashMap::<ObjectId, ObjectId>::new();
    let mut can_map = true;
    for atom in atoms {
        validate_qualified_object_name_atom(atom, line, object_names, object_schemas)?;
        let expanded = qualify_object_name_atom(atom, suffix, line)?;
        let resolved = resolve_object_selector(
            &expanded,
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            variable_names,
        )?;
        if resolved.transform.is_some() || resolved.family_wildcard.is_some() {
            return Err(parse_error(
                line,
                "qualified tag selector cannot use mapped selector terms",
            ));
        }
        if resolved.dynamic_guards.is_empty() && resolved.alternatives.len() == 1 {
            let target = resolved.alternatives[0];
            for source in object_name_atom_source_objects(atom, object_names, object_schemas) {
                mapped_objects.insert(source, target);
            }
        } else {
            can_map = false;
        }
        for object in resolved.alternatives {
            if !alternatives.contains(&object) {
                alternatives.push(object);
            }
        }
        for (object, guards) in resolved.dynamic_guards {
            dynamic_guards.entry(object).or_default().extend(guards);
        }
    }
    if alternatives.is_empty() {
        return Err(parse_error(
            line,
            "qualified tag selector matched no objects",
        ));
    }
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard: (can_map && !mapped_objects.is_empty())
            .then_some(FamilyWildcardSelector { mapped_objects }),
        correspondence_source_token: None,
        relative_constraints: Vec::new(),
        capture_requirements: HashMap::new(),
        dynamic_guards,
        tag_captures: HashMap::new(),
        mark,
        occurrence_label,
    })
}

fn validate_qualified_object_name_atom(
    atom: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
) -> Result<(), DiagnosticReport> {
    if object_schemas.contains_key(atom) || object_names.contains_key(atom) {
        return Ok(());
    }
    Err(parse_error(
        line,
        "qualified tag selector values must name object families or objects",
    ))
}

fn object_name_atom_source_objects(
    atom: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
) -> Vec<ObjectId> {
    if let Some(schema) = object_schemas.get(atom) {
        return schema
            .variants
            .iter()
            .map(|variant| variant.object)
            .collect();
    }
    object_names.get(atom).copied().into_iter().collect()
}

fn qualify_object_name_atom(
    atom: &str,
    suffix: &str,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if atom.contains('{') || atom.contains('#') || atom.contains(':') {
        return Err(parse_error(
            line,
            "qualified tag selector cannot qualify non-atomic object names",
        ));
    }
    Ok(format!("{atom}:{suffix}"))
}

fn resolve_schema_family_wildcard_selector(
    parts: &[&str],
    token: String,
    mark: Vec<SelectorMark>,
    occurrence_label: Option<String>,
    line: &str,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    variable_names: &HashMap<String, VariableId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    if parts.len() != 2 {
        return Err(parse_error(
            line,
            &format!(
                "family wildcard object selector must be {}:<tag>",
                puzzle_authoring::SELECTOR_WILDCARD
            ),
        ));
    }
    let tag = parts[1];
    let (mut alternatives, family_wildcard) = if puzzle_authoring::is_selector_wildcard(tag) {
        (
            schema_wildcard_alternatives(object_schemas, |_, _| true),
            None,
        )
    } else {
        let expr = parse_value_expr(tag, line)?;
        let ValueExpr::Binding(name) = expr else {
            return Err(parse_error(
                line,
                "family wildcard object selector cannot use map calls",
            ));
        };
        let names_schema_tag = schema_wildcard_tag_value_exists(object_schemas, &name);
        let value_set = value_sets.get(&name);
        if variable_names.contains_key(&name) && (names_schema_tag || value_set.is_some()) {
            return Err(parse_error(
                line,
                &format!(
                    "selector tag {name} is ambiguous for family wildcard selector: it is both a schema tag and a variable"
                ),
            ));
        }
        if variable_names.contains_key(&name) {
            return Err(parse_error(
                line,
                "family wildcard object selector cannot use dynamic var tags",
            ));
        }
        if let Some(values) = value_set {
            for value in values {
                if !schema_wildcard_tag_value_exists(object_schemas, value) {
                    return Err(parse_error(
                        line,
                        &format!(
                            "tag set {name} contains value {value} that is not used by any schema object"
                        ),
                    ));
                }
            }
            (
                schema_wildcard_alternatives(object_schemas, |_, variant| {
                    variant.values.iter().any(|value| values.contains(value))
                }),
                Some(FamilyWildcardSelector {
                    mapped_objects: schema_wildcard_target_set_map(object_schemas, values, line)?,
                }),
            )
        } else {
            (
                schema_wildcard_alternatives(object_schemas, |_, variant| {
                    variant.values.iter().any(|value| value == &name)
                }),
                Some(FamilyWildcardSelector {
                    mapped_objects: schema_wildcard_target_map(object_schemas, &name, line)?,
                }),
            )
        }
    };

    alternatives.sort_by_key(|object| object.0);
    alternatives.dedup();
    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard,
        correspondence_source_token: None,
        relative_constraints: Vec::new(),
        capture_requirements: HashMap::new(),
        dynamic_guards: HashMap::new(),
        tag_captures: HashMap::new(),
        mark,
        occurrence_label,
    })
}

fn schema_wildcard_alternatives(
    object_schemas: &HashMap<String, ObjectSchema>,
    matches: impl Fn(&ObjectSchema, &ObjectVariant) -> bool,
) -> Vec<ObjectId> {
    let mut alternatives = Vec::new();
    for schema in object_schemas.values() {
        for variant in &schema.variants {
            if matches(schema, variant) {
                alternatives.push(variant.object);
            }
        }
    }
    alternatives
}

fn schema_wildcard_tag_value_exists(
    object_schemas: &HashMap<String, ObjectSchema>,
    tag: &str,
) -> bool {
    object_schemas.values().any(|schema| {
        schema
            .variants
            .iter()
            .any(|variant| variant.values.iter().any(|value| value == tag))
    })
}

fn schema_wildcard_target_map(
    object_schemas: &HashMap<String, ObjectSchema>,
    target_tag: &str,
    line: &str,
) -> Result<HashMap<ObjectId, ObjectId>, DiagnosticReport> {
    schema_wildcard_target_set_map(object_schemas, &[target_tag.to_string()], line)
}

fn schema_wildcard_target_set_map(
    object_schemas: &HashMap<String, ObjectSchema>,
    target_tags: &[String],
    line: &str,
) -> Result<HashMap<ObjectId, ObjectId>, DiagnosticReport> {
    let mut mapped = HashMap::new();
    for schema in object_schemas.values() {
        for source in &schema.variants {
            let mut targets = Vec::new();
            for axis_index in 0..schema.axes.len() {
                let axis_values = schema_axis_values(schema, axis_index)?;
                let target_axis_values = target_tags
                    .iter()
                    .filter(|target_tag| axis_values.iter().any(|value| value == *target_tag))
                    .collect::<Vec<_>>();
                if target_axis_values.is_empty() {
                    continue;
                }
                for target_tag in target_axis_values {
                    let mut target_values = source.values.clone();
                    target_values[axis_index] = (*target_tag).clone();
                    let Some(target) = schema
                        .variants
                        .iter()
                        .find(|variant| variant.values == target_values)
                        .map(|variant| variant.object)
                    else {
                        continue;
                    };
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
            match targets.as_slice() {
                [] => {}
                [target] => {
                    mapped.insert(source.object, *target);
                }
                _ => {
                    return Err(parse_error(
                        line,
                        "family wildcard target tag is ambiguous for a source object",
                    ));
                }
            }
        }
    }
    Ok(mapped)
}

fn selector_tag_capture_key<'a>(
    value: &'a str,
    axis: &str,
    axis_count: usize,
    line: &str,
) -> Result<(&'a str, Option<String>), DiagnosticReport> {
    if parse_map_call(value).is_some() {
        return Ok((value, None));
    }
    let Some((base, label)) = value.split_once('#') else {
        if value == axis {
            return Ok((value, Some(axis.to_string())));
        }
        if puzzle_authoring::is_selector_wildcard(value) && axis_count == 1 {
            return Ok((value, Some(puzzle_authoring::SELECTOR_WILDCARD.to_string())));
        }
        return Ok((value, None));
    };
    validate_tag_capture_label(label, line)?;
    if puzzle_authoring::is_selector_wildcard(base) {
        return Ok((
            base,
            Some(format!("{}#{label}", puzzle_authoring::SELECTOR_WILDCARD)),
        ));
    }
    if base == axis {
        return Ok((base, Some(format!("{axis}#{label}"))));
    }
    Err(parse_error(
        line,
        &format!(
            "tag capture labels must attach to {} or the schema tag slot name",
            puzzle_authoring::SELECTOR_WILDCARD
        ),
    ))
}

fn validate_tag_capture_label(label: &str, line: &str) -> Result<(), DiagnosticReport> {
    if label.is_empty()
        || !label
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err(parse_error(
            line,
            "tag capture label may only contain letters, numbers, and _",
        ));
    }
    Ok(())
}

fn selector_tag_captures(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    alternatives: &[ObjectId],
) -> Result<HashMap<ObjectId, Vec<TagCapture>>, DiagnosticReport> {
    let capture_constraints = constraints
        .iter()
        .flatten()
        .filter_map(|constraint| match constraint {
            SelectorConstraint::Capture { axis_index, key } => Some((*axis_index, key)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if capture_constraints.is_empty() {
        return Ok(HashMap::new());
    }
    let mut out = HashMap::<ObjectId, Vec<TagCapture>>::new();
    for variant in &schema.variants {
        if !alternatives.contains(&variant.object) {
            continue;
        }
        for (axis_index, key) in &capture_constraints {
            let value = variant.values.get(*axis_index).ok_or_else(|| {
                DiagnosticReport::error("internal schema variant missing tag value".to_string())
            })?;
            out.entry(variant.object).or_default().push(TagCapture {
                key: (*key).clone(),
                value: value.clone(),
            });
        }
    }
    Ok(out)
}

fn labeled_selector_token(selector: &str, occurrence_label: Option<&str>) -> String {
    match occurrence_label {
        Some(label) => format!("{selector}#{label}"),
        None => selector.to_string(),
    }
}

fn validate_mark_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return Err(parse_error(
            line,
            "mark name must start with an identifier and may use :value parts",
        ));
    };
    if !is_identifier(first) || !parts.all(is_mark_name_value_atom) {
        return Err(parse_error(
            line,
            "mark name must start with an identifier and may use :value parts",
        ));
    }
    Ok(())
}

fn is_mark_name_value_atom(value: &str) -> bool {
    is_value_atom(value) || matches!(value, ">" | "<" | "^" | "v")
}

#[derive(Clone, Debug)]
enum SelectorConstraint {
    Fixed(String),
    ValueSet(Vec<String>),
    Capture {
        axis_index: usize,
        key: String,
    },
    Relative {
        axis_index: usize,
        relative: RelativeDirection,
    },
    Mapped {
        axis_index: usize,
        expr: ValueExpr,
    },
    AxisComputed {
        axis_index: usize,
        expr: AxisComputedExpr,
    },
    DynamicVariable {
        axis_index: usize,
        name: String,
        variable: VariableId,
    },
}

#[derive(Clone, Debug)]
enum AxisComputedExpr {
    AngleDelta { delta: Rational },
    Vec2Delta { terms: Vec<Vec2DeltaTerm> },
}

#[derive(Clone, Debug)]
enum Vec2DeltaTerm {
    Coordinate {
        dx: Rational,
        dy: Rational,
    },
    AbsoluteDirection {
        amount: Rational,
        direction: AbsoluteOffsetDirection,
    },
    RelativeDirection {
        amount: Rational,
        direction: RelativeDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbsoluteOffsetDirection {
    Left,
    Right,
    Up,
    Down,
}

fn dynamic_selector_guards(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    line: &str,
) -> Result<HashMap<ObjectId, Vec<DynamicSelectorGuard>>, DiagnosticReport> {
    if !constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::DynamicVariable { .. })))
    {
        return Ok(HashMap::new());
    }

    let mut guards = HashMap::<ObjectId, Vec<DynamicSelectorGuard>>::new();
    for variant in &schema.variants {
        let mut variant_guards = Vec::new();
        for constraint in constraints.iter().flatten() {
            let SelectorConstraint::DynamicVariable {
                axis_index,
                name,
                variable,
            } = constraint
            else {
                continue;
            };
            let value = variant.values.get(*axis_index).ok_or_else(|| {
                DiagnosticReport::error("internal schema variant missing tag value".to_string())
            })?;
            variant_guards.push(DynamicSelectorGuard {
                name: name.clone(),
                variable: *variable,
                value: parse_variable_value(value, line).map_err(|_| {
                    parse_error(
                        line,
                        "dynamic selector tag slot values must be true, false, or integers",
                    )
                })?,
            });
        }
        guards.insert(variant.object, variant_guards);
    }
    Ok(guards)
}

fn map_argument_axis(arg: &str) -> &str {
    arg.split_once('#').map_or(arg, |(axis, _)| axis)
}

fn selector_constraint_needs_occurrence_transform(constraint: &Option<SelectorConstraint>) -> bool {
    match constraint {
        Some(SelectorConstraint::AxisComputed { .. }) => true,
        Some(SelectorConstraint::Mapped { expr, .. }) => !value_expr_uses_tag_capture_ref(expr),
        _ => false,
    }
}

fn value_expr_uses_tag_capture_ref(expr: &ValueExpr) -> bool {
    match expr {
        ValueExpr::Binding(value) => value.contains('#'),
        ValueExpr::MapCall { arg, .. } => arg.contains('#'),
    }
}

fn capture_selector_requirements(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    alternatives: &[ObjectId],
) -> Result<HashMap<ObjectId, Vec<CaptureSelectorRequirement>>, DiagnosticReport> {
    let mut requirements = HashMap::new();
    for variant in schema
        .variants
        .iter()
        .filter(|variant| alternatives.contains(&variant.object))
    {
        let mut object_requirements = Vec::new();
        for constraint in constraints.iter().flatten() {
            match constraint {
                SelectorConstraint::Capture { axis_index, key } if key.contains('#') => {
                    let value = variant.values.get(*axis_index).ok_or_else(|| {
                        DiagnosticReport::error("internal schema variant missing tag value")
                    })?;
                    object_requirements.push(CaptureSelectorRequirement::Direct {
                        key: key.clone(),
                        value: value.clone(),
                    });
                }
                SelectorConstraint::Mapped { axis_index, expr }
                    if value_expr_uses_tag_capture_ref(expr) =>
                {
                    let ValueExpr::MapCall { name, arg } = expr else {
                        unreachable!("mapped selector constraint must contain a map call");
                    };
                    let value = variant.values.get(*axis_index).ok_or_else(|| {
                        DiagnosticReport::error("internal schema variant missing tag value")
                    })?;
                    object_requirements.push(CaptureSelectorRequirement::Mapped {
                        key: arg.clone(),
                        map_name: name.clone(),
                        value: value.clone(),
                    });
                }
                _ => {}
            }
        }
        if !object_requirements.is_empty() {
            requirements.insert(variant.object, object_requirements);
        }
    }
    Ok(requirements)
}

fn validate_schema_selector_arity(
    parts: &[&str],
    schema: &ObjectSchema,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let slot_count = parts.len().saturating_sub(1);
    if slot_count > schema.axes.len() {
        return Err(parse_error(line, &format!("{label} has too many tags")));
    }
    if slot_count == 0 {
        return Ok(());
    }
    if slot_count == 1 && puzzle_authoring::is_selector_wildcard(parts[1]) {
        return Ok(());
    }
    if slot_count < schema.axes.len() {
        return Err(parse_error(
            line,
            &format!(
                "{label} must name every variant slot; use {} for unconstrained slots",
                puzzle_authoring::SELECTOR_WILDCARD
            ),
        ));
    }
    Ok(())
}

fn schema_selector_part<'a>(
    parts: &'a [&str],
    schema: &ObjectSchema,
    axis_index: usize,
) -> Option<&'a str> {
    if parts.len() == 2 && puzzle_authoring::is_selector_wildcard(parts[1]) && schema.axes.len() > 1
    {
        return Some(puzzle_authoring::SELECTOR_WILDCARD);
    }
    parts.get(axis_index + 1).copied()
}

fn schema_axis_values(
    schema: &ObjectSchema,
    axis_index: usize,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut values = Vec::new();
    for variant in &schema.variants {
        let value = variant.values.get(axis_index).ok_or_else(|| {
            DiagnosticReport::error("internal schema variant missing tag value".to_string())
        })?;
        if !values.contains(value) {
            values.push(value.clone());
        }
    }
    Ok(values)
}

fn validate_selector_subset(
    value_set_name: &str,
    values: &[String],
    axis_values: &[String],
    family: &str,
    axis: &str,
    line: &str,
) -> Result<(), DiagnosticReport> {
    for value in values {
        if !axis_values.contains(value) {
            return Err(parse_error(
                line,
                &format!(
                    "tag set {value_set_name} contains value {value}, which is not valid for {family} tag slot {axis}",
                ),
            ));
        }
    }
    Ok(())
}

fn ambiguous_selector_tag_error(
    tag: &str,
    family: &str,
    axis: &str,
    line: &str,
) -> DiagnosticReport {
    parse_error(
        line,
        &format!(
            "selector tag {tag} is ambiguous for {family} tag slot {axis}: it is both a concrete value and a tag set",
        ),
    )
}

fn parse_axis_computed_selector_value(
    value: &str,
    axis: &str,
    value_type: ValueType,
    line: &str,
) -> Result<Option<AxisComputedExpr>, DiagnosticReport> {
    let value = value.trim();
    let Some(inner) = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Ok(None);
    };
    let inner = inner.trim();
    let Some((expr_axis, _)) = split_axis_delta_expr(inner) else {
        if value_type == ValueType::Vec2 && parse_vec2_value(value, line).is_ok() {
            return Ok(None);
        }
        return Err(parse_error(
            line,
            "computed selector must start with its axis name",
        ));
    };
    if expr_axis != axis {
        return Err(parse_error(
            line,
            "computed selector axis name must match the selector slot",
        ));
    }
    let expr = match value_type {
        ValueType::Angle => AxisComputedExpr::AngleDelta {
            delta: parse_rotation_delta_expr(inner, line)?,
        },
        ValueType::Vec2 => AxisComputedExpr::Vec2Delta {
            terms: parse_vec2_delta_expr(inner, line)?,
        },
        _ => return Ok(None),
    };
    Ok(Some(expr))
}

fn parse_rotation_delta_expr(expr: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let Some((_, delta)) = split_axis_delta_expr(expr) else {
        return Err(parse_error(
            line,
            "angle computed selector must be: (axis + <deg>) or (axis - <deg>)",
        ));
    };
    parse_signed_degree_delta(delta, line)
}

fn parse_vec2_delta_expr(expr: &str, line: &str) -> Result<Vec<Vec2DeltaTerm>, DiagnosticReport> {
    let Some((_, delta)) = split_axis_delta_expr(expr) else {
        return Err(parse_error(
            line,
            "vec2 computed selector must start with axis + or axis -",
        ));
    };
    let delta = delta.trim();
    if delta
        .strip_prefix('+')
        .or_else(|| delta.strip_prefix('-'))
        .is_some_and(|value| value.trim_start().starts_with('('))
    {
        let (sign, value) = if let Some(value) = delta.strip_prefix('+') {
            (1, value)
        } else {
            (
                -1,
                delta
                    .strip_prefix('-')
                    .expect("vec2 delta sign was checked"),
            )
        };
        let (mut dx, mut dy) = parse_vec2_value(value.trim(), line)?;
        if sign < 0 {
            dx = dx.neg();
            dy = dy.neg();
        }
        return Ok(vec![Vec2DeltaTerm::Coordinate { dx, dy }]);
    }

    let mut terms = Vec::new();
    for term in split_signed_terms(delta) {
        let tokens = term.split_whitespace().collect::<Vec<_>>();
        let (amount, direction) = match tokens.as_slice() {
            [amount, direction] => ((*amount).to_string(), *direction),
            [sign @ ("+" | "-"), amount, direction] => (format!("{sign}{amount}"), *direction),
            _ => {
                return Err(parse_error(
                    line,
                    "vec2 direction delta must use terms like + 1/2 left",
                ));
            }
        };
        let amount = parse_signed_rational_delta(&amount, line)?;
        if let Some(direction) = parse_absolute_offset_direction(direction) {
            terms.push(Vec2DeltaTerm::AbsoluteDirection { amount, direction });
        } else if let Some(direction) = parse_relative_direction_value(direction) {
            terms.push(Vec2DeltaTerm::RelativeDirection { amount, direction });
        } else {
            return Err(parse_error(line, "unknown vec2 direction"));
        }
    }
    if terms.is_empty() {
        return Err(parse_error(line, "vec2 computed selector has no delta"));
    }
    Ok(terms)
}

fn split_axis_delta_expr(expr: &str) -> Option<(&str, &str)> {
    let expr = expr.trim();
    for (index, ch) in expr.char_indices() {
        if (ch == '+' || ch == '-') && index > 0 {
            let (axis, delta) = expr.split_at(index);
            let axis = axis.trim();
            if is_identifier(axis) {
                return Some((axis, delta.trim()));
            }
        }
    }
    None
}

fn split_signed_terms(value: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    for (index, ch) in value.char_indices() {
        if (ch == '+' || ch == '-') && index > 0 {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                terms.push(trimmed.to_string());
            }
            current.clear();
        }
        current.push(ch);
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        terms.push(trimmed.to_string());
    }
    terms
}

fn parse_signed_degree_delta(value: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let value = normalize_signed_delta_text(value);
    parse_degree_value(&value, line)
}

fn parse_signed_rational_delta(value: &str, line: &str) -> Result<Rational, DiagnosticReport> {
    let value = normalize_signed_delta_text(value);
    parse_rational_value(&value, line)
}

fn normalize_signed_delta_text(value: &str) -> String {
    value
        .trim()
        .strip_prefix('+')
        .map(str::trim_start)
        .map(str::to_string)
        .unwrap_or_else(|| {
            value
                .trim()
                .strip_prefix('-')
                .map(str::trim_start)
                .map(|rest| format!("-{rest}"))
                .unwrap_or_else(|| value.trim().to_string())
        })
}

fn parse_absolute_offset_direction(value: &str) -> Option<AbsoluteOffsetDirection> {
    match value {
        "left" => Some(AbsoluteOffsetDirection::Left),
        "right" => Some(AbsoluteOffsetDirection::Right),
        "up" => Some(AbsoluteOffsetDirection::Up),
        "down" => Some(AbsoluteOffsetDirection::Down),
        _ => None,
    }
}

fn normalize_axis_literal(
    value: &str,
    schema: &ObjectSchema,
    axis_index: usize,
    line: &str,
) -> Result<String, DiagnosticReport> {
    match schema.axis_types.get(axis_index).copied().flatten() {
        Some(ValueType::Angle) => Ok(format!("{}deg", parse_degree_value(value, line)?.format())),
        Some(ValueType::Vec2) => normalize_vec2_value(value, line),
        Some(ValueType::Frame3) => crate::frame3_literal::normalize_frame3_literal(value)
            .map_err(|error| parse_error(line, &error)),
        Some(ValueType::Int | ValueType::Rational) => {
            Ok(parse_rational_value(value, line)?.format())
        }
        Some(_) => Ok(value.to_string()),
        None => Ok(value.to_string()),
    }
}

fn normalize_vec2_value(value: &str, line: &str) -> Result<String, DiagnosticReport> {
    let (x, y) = parse_vec2_value(value, line)?;
    Ok(format!("({},{})", x.format(), y.format()))
}

fn parse_vec2_value(value: &str, line: &str) -> Result<(Rational, Rational), DiagnosticReport> {
    let value = value.trim();
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| parse_error(line, "vec2 value must be parenthesized: (<x>, <y>)"))?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(parse_error(line, "vec2 value must be: (<x>, <y>)"));
    }
    Ok((
        parse_rational_value(parts[0], line)?,
        parse_rational_value(parts[1], line)?,
    ))
}

fn validate_axis_computed_source_constraints(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    source_token_parts: &[String],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let relative_translation = constraints.iter().flatten().any(|constraint| {
        matches!(
            constraint,
            SelectorConstraint::AxisComputed {
                expr: AxisComputedExpr::Vec2Delta { terms },
                ..
            } if terms
                .iter()
                .any(|term| matches!(term, Vec2DeltaTerm::RelativeDirection { .. }))
        )
    });
    if !relative_translation {
        return Ok(());
    }
    let Some(rotation_index) = schema
        .axis_types
        .iter()
        .position(|value_type| *value_type == Some(ValueType::Angle))
    else {
        return Err(parse_error(
            line,
            "relative vec2 arithmetic requires an angle tag on the same object",
        ));
    };
    if source_token_parts.get(rotation_index) != schema.axes.get(rotation_index) {
        return Err(parse_error(
            line,
            "relative vec2 arithmetic requires the angle tag to be captured",
        ));
    }
    Ok(())
}

fn eval_axis_computed_selector_value(
    expr: &AxisComputedExpr,
    schema: &ObjectSchema,
    source: &ObjectVariant,
    line: &str,
) -> Result<String, DiagnosticReport> {
    match expr {
        AxisComputedExpr::AngleDelta { delta } => {
            let Some(axis_index) = schema
                .axis_types
                .iter()
                .position(|value_type| *value_type == Some(ValueType::Angle))
            else {
                return Err(parse_error(
                    line,
                    "angle computed selector requires an angle tag",
                ));
            };
            let current = source
                .values
                .get(axis_index)
                .ok_or_else(|| parse_error(line, "internal schema variant missing axis value"))?;
            let target = parse_degree_value(current, line)?.add(*delta);
            resolve_rotation_axis_value(schema, axis_index, target, line)
        }
        AxisComputedExpr::Vec2Delta { terms } => {
            let Some(axis_index) = schema
                .axis_types
                .iter()
                .position(|value_type| *value_type == Some(ValueType::Vec2))
            else {
                return Err(parse_error(
                    line,
                    "vec2 computed selector requires a vec2 tag",
                ));
            };
            let current = source
                .values
                .get(axis_index)
                .ok_or_else(|| parse_error(line, "internal schema variant missing axis value"))?;
            let (mut x, mut y) = parse_vec2_value(current, line)?;
            let facing = source_angle_degrees(schema, source, line)?;
            for term in terms {
                let (dx, dy) = vec2_delta_vector(term, facing, line)?;
                x = x.add(dx);
                y = y.add(dy);
            }
            let target = format!("({},{})", x.format(), y.format());
            let axis_values = schema_axis_values(schema, axis_index)?;
            if axis_values.contains(&target) {
                Ok(target)
            } else {
                Err(parse_error(
                    line,
                    "vec2 computed selector target is not declared",
                ))
            }
        }
    }
}

fn resolve_rotation_axis_value(
    schema: &ObjectSchema,
    axis_index: usize,
    target: Rational,
    line: &str,
) -> Result<String, DiagnosticReport> {
    let axis_values = schema_axis_values(schema, axis_index)?;
    let exact = format!("{}deg", target.format());
    if axis_values.contains(&exact) {
        return Ok(exact);
    }
    for value in axis_values {
        let candidate = parse_degree_value(&value, line)?;
        if degrees_congruent(candidate, target) {
            return Ok(value);
        }
    }
    Err(parse_error(
        line,
        "angle computed selector target is not declared",
    ))
}

fn degrees_congruent(left: Rational, right: Rational) -> bool {
    let diff = left.sub(right);
    diff.numerator % (360 * diff.denominator) == 0
}

fn source_angle_degrees(
    schema: &ObjectSchema,
    source: &ObjectVariant,
    line: &str,
) -> Result<Option<Rational>, DiagnosticReport> {
    let Some(axis_index) = schema
        .axis_types
        .iter()
        .position(|value_type| *value_type == Some(ValueType::Angle))
    else {
        return Ok(None);
    };
    let value = source
        .values
        .get(axis_index)
        .ok_or_else(|| parse_error(line, "internal schema variant missing axis value"))?;
    Ok(Some(parse_degree_value(value, line)?))
}

fn vec2_delta_vector(
    term: &Vec2DeltaTerm,
    facing: Option<Rational>,
    line: &str,
) -> Result<(Rational, Rational), DiagnosticReport> {
    match term {
        Vec2DeltaTerm::Coordinate { dx, dy } => Ok((*dx, *dy)),
        Vec2DeltaTerm::AbsoluteDirection { amount, direction } => Ok(scale_offset_vector(
            *amount,
            absolute_offset_vector(*direction),
        )),
        Vec2DeltaTerm::RelativeDirection { amount, direction } => {
            let facing = facing.ok_or_else(|| {
                parse_error(
                    line,
                    "relative vec2 arithmetic requires an angle tag on the same object",
                )
            })?;
            let vector = relative_offset_vector(facing, *direction, line)?;
            Ok(scale_offset_vector(*amount, vector))
        }
    }
}

fn absolute_offset_vector(direction: AbsoluteOffsetDirection) -> (i64, i64) {
    match direction {
        AbsoluteOffsetDirection::Left => (-1, 0),
        AbsoluteOffsetDirection::Right => (1, 0),
        AbsoluteOffsetDirection::Up => (0, -1),
        AbsoluteOffsetDirection::Down => (0, 1),
    }
}

fn relative_offset_vector(
    facing: Rational,
    direction: RelativeDirection,
    line: &str,
) -> Result<(i64, i64), DiagnosticReport> {
    let base = if degrees_congruent(facing, Rational::integer(0)) {
        (1, 0)
    } else if degrees_congruent(facing, Rational::integer(90)) {
        (0, -1)
    } else if degrees_congruent(facing, Rational::integer(180)) {
        (-1, 0)
    } else if degrees_congruent(facing, Rational::integer(270)) {
        (0, 1)
    } else {
        return Err(parse_error(
            line,
            "relative vec2 arithmetic requires cardinal angle values",
        ));
    };
    Ok(match direction {
        RelativeDirection::Forward => base,
        RelativeDirection::Backward => (-base.0, -base.1),
        RelativeDirection::Left => (base.1, -base.0),
        RelativeDirection::Right => (-base.1, base.0),
    })
}

fn scale_offset_vector(amount: Rational, vector: (i64, i64)) -> (Rational, Rational) {
    (
        Rational::integer(vector.0).mul(amount),
        Rational::integer(vector.1).mul(amount),
    )
}

fn parse_map_call(value: &str) -> Option<(&str, &str)> {
    let (name, rest) = value.split_once('(')?;
    let arg = rest.strip_suffix(')')?;
    Some((name, arg))
}

fn relative_selector_constraints(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    alternatives: &[ObjectId],
) -> Result<Vec<RelativeSelectorConstraint>, DiagnosticReport> {
    let mut out = Vec::new();
    for constraint in constraints.iter().flatten() {
        let SelectorConstraint::Relative {
            axis_index,
            relative,
        } = constraint
        else {
            continue;
        };
        let axis_values = schema_axis_values(schema, *axis_index)?;
        let mut alternatives_by_direction = HashMap::<String, Vec<ObjectId>>::new();
        for value in axis_values {
            let mut objects = schema
                .variants
                .iter()
                .filter(|variant| {
                    alternatives.contains(&variant.object) && variant.values[*axis_index] == value
                })
                .map(|variant| variant.object)
                .collect::<Vec<_>>();
            dedup_objects(&mut objects);
            alternatives_by_direction.insert(value, objects);
        }
        out.push(RelativeSelectorConstraint {
            relative: *relative,
            alternatives_by_direction,
        });
    }
    Ok(out)
}

fn compile_before_after_blocks_for_direction(
    before: &PatternBlock,
    after: &PatternBlock,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
    source_line_number: Option<usize>,
) -> Result<(PatternBlock, Vec<RuleBodyAlternative>), DiagnosticReport> {
    let before = resolve_relative_selectors_in_block(before, direction, direction_expanded, line)?;
    let after = resolve_relative_selectors_in_block(after, direction, direction_expanded, line)?;
    if block_has_unavailable_required_selector(&before)
        || block_has_unavailable_required_selector(&after)
    {
        return Ok((before, Vec::new()));
    }
    let alternatives = compile_before_after_blocks(
        &before,
        &after,
        object_layers,
        mark_names,
        value_sets,
        maps,
        line,
        source_line_number,
    )?;
    Ok((before, alternatives))
}

fn compile_before_after_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    _value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
    source_line_number: Option<usize>,
) -> Result<Vec<RuleBodyAlternative>, DiagnosticReport> {
    if before.components.len() != after.components.len() {
        return Err(parse_error(
            line,
            "before and after sides must have the same number of blocks",
        ));
    }
    for (before_component, after_component) in before.components.iter().zip(&after.components) {
        if !block_shapes_match(before_component, after_component) {
            return Err(parse_error(
                line,
                "before and after blocks must have matching cell and ellipsis layout",
            ));
        }
        validate_null_component_has_anchor_cell(before_component, line)?;
        validate_null_cell_rewrite(before_component, after_component, line)?;
    }
    let expanded_blocks = expand_movement_mark_sets(before, after, _value_sets, line)?;
    let mut alternatives = Vec::new();

    for (before, after) in expanded_blocks {
        let dynamic_blocks = expand_dynamic_selector_blocks(&before, &after);
        for (dynamic_guards, before, after) in dynamic_blocks {
            let before_occurrences = collect_before_occurrences(&before);
            reject_duplicate_labeled_occurrences(&before_occurrences, line)?;
            let mut assignments =
                expand_selector_assignments(&before_occurrences, object_layers, line)?;
            if before_occurrences
                .iter()
                .any(|occurrence| occurrence.occurrence_label.is_some())
            {
                assignments.reverse();
            }
            let before_by_token = before_occurrences_by_token(&before_occurrences);
            'assignment_loop: for assignment in assignments {
                let all_before_occurrences = &before_occurrences;
                let tag_captures = tag_captures_for_assignment(&before_occurrences, &assignment);
                if tag_captures.has_conflict() {
                    continue 'assignment_loop;
                }
                if !assignment_matches_capture_requirements(
                    &before_occurrences,
                    &assignment,
                    &tag_captures,
                    maps,
                    line,
                )? {
                    continue 'assignment_loop;
                }
                let mut components = Vec::new();
                let mut writes = Vec::new();
                let mut before_token_counts = HashMap::<String, usize>::new();
                let mut after_token_counts = HashMap::<String, usize>::new();
                let mut before_placements = HashMap::<OccurrenceKey, OccurrencePlacement>::new();
                let mut after_placements = HashMap::<OccurrenceKey, OccurrencePlacement>::new();
                let mut duplicate_after_keys = HashSet::<OccurrenceKey>::new();

                for (component_index, (before_component, after_component)) in
                    before.components.iter().zip(&after.components).enumerate()
                {
                    let component_index = component_index as u16;
                    let mut component_cells = Vec::new();
                    let shared_gap_indices = rectangular_shared_gap_indices(before_component);
                    let mut next_gap_index = shared_gap_indices.as_ref().map_or(0, |indices| {
                        indices.iter().filter(|index| index.is_some()).count() as u16
                    });

                    for (y, (before_row, after_row)) in before_component
                        .rows
                        .iter()
                        .zip(&after_component.rows)
                        .enumerate()
                    {
                        let mut concrete_x = 0_i16;
                        let mut active_gaps = Vec::<u16>::new();

                        for (part_index, (before_part, after_part)) in
                            before_row.iter().zip(after_row).enumerate()
                        {
                            if matches!(before_part, BlockPart::Ellipsis) {
                                let gap_index = shared_gap_indices
                                    .as_ref()
                                    .and_then(|indices| indices.get(part_index).copied().flatten())
                                    .unwrap_or_else(|| {
                                        let gap_index = next_gap_index;
                                        next_gap_index += 1;
                                        gap_index
                                    });
                                active_gaps.push(gap_index);
                                continue;
                            }

                            let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) =
                                (before_part, after_part)
                            else {
                                unreachable!(
                                    "block_shapes_match already validated matching part kinds"
                                );
                            };
                            let offset = OffsetTemplate {
                                oriented_x: concrete_x,
                                oriented_y: y as i16,
                                gap_terms: active_gaps.clone(),
                            };
                            let before_occurrences = block_cell_object_occurrences(
                                before_cell,
                                &assignment,
                                all_before_occurrences,
                                &before_by_token,
                                &mut before_token_counts,
                                &tag_captures,
                                maps,
                                line,
                                source_line_number,
                            )?;
                            let mut after_occurrences = block_cell_object_occurrences(
                                after_cell,
                                &assignment,
                                all_before_occurrences,
                                &before_by_token,
                                &mut after_token_counts,
                                &tag_captures,
                                maps,
                                line,
                                source_line_number,
                            )?;
                            prefer_same_cell_occurrence_keys(
                                &before_occurrences,
                                &mut after_occurrences,
                            );
                            if !validate_same_layer_cell_occurrences(
                                &before_occurrences,
                                object_layers,
                                line,
                            )? {
                                continue 'assignment_loop;
                            }
                            if !validate_same_layer_cell_occurrences(
                                &after_occurrences,
                                object_layers,
                                line,
                            )? {
                                continue 'assignment_loop;
                            }
                            let mut before_objects =
                                possible_objects_for_occurrences(&before_occurrences);
                            let mut after_objects =
                                possible_objects_for_occurrences(&after_occurrences);
                            let before_mark = block_cell_mark(
                                before_cell,
                                &before_occurrences,
                                mark_names,
                                line,
                            )?;
                            let after_mark =
                                block_cell_mark(after_cell, &after_occurrences, mark_names, line)?;
                            dedup_objects(&mut before_objects);
                            dedup_objects(&mut after_objects);
                            let require_objects =
                                concrete_objects_for_occurrences(&before_occurrences);
                            let require_object_sets =
                                object_sets_for_occurrences(&before_occurrences);

                            for occurrence in &before_occurrences {
                                if let Some(key) = &occurrence.key {
                                    before_placements.insert(
                                        key.clone(),
                                        OccurrencePlacement {
                                            component: component_index,
                                            offset: offset.clone(),
                                            matched: occurrence.matched.clone(),
                                            require_mark: before_mark
                                                .require
                                                .iter()
                                                .filter(|attr| {
                                                    occurrence
                                                        .matched
                                                        .possible_objects()
                                                        .contains(&attr.object)
                                                })
                                                .cloned()
                                                .collect(),
                                            require_object_set_mark: before_mark
                                                .require_object_set
                                                .iter()
                                                .filter(|attr| {
                                                    matches!(
                                                        &occurrence.matched,
                                                        ResolvedObjectMatch::ObjectSet {
                                                            binding,
                                                            ..
                                                        } if *binding == attr.binding
                                                    )
                                                })
                                                .cloned()
                                                .collect(),
                                        },
                                    );
                                }
                            }
                            for occurrence in &after_occurrences {
                                if let Some(key) = &occurrence.key {
                                    if duplicate_after_keys.contains(key) {
                                        continue;
                                    }
                                    let placement = OccurrencePlacement {
                                        component: component_index,
                                        offset: offset.clone(),
                                        matched: occurrence.matched.clone(),
                                        require_mark: after_mark
                                            .require
                                            .iter()
                                            .filter(|attr| {
                                                occurrence
                                                    .matched
                                                    .possible_objects()
                                                    .contains(&attr.object)
                                            })
                                            .cloned()
                                            .collect(),
                                        require_object_set_mark: after_mark
                                            .require_object_set
                                            .iter()
                                            .filter(|attr| {
                                                matches!(
                                                    &occurrence.matched,
                                                    ResolvedObjectMatch::ObjectSet {
                                                        binding,
                                                        ..
                                                    } if *binding == attr.binding
                                                )
                                            })
                                            .cloned()
                                            .collect(),
                                    };
                                    if after_placements.insert(key.clone(), placement).is_some() {
                                        after_placements.remove(key);
                                        duplicate_after_keys.insert(key.clone());
                                    }
                                }
                            }
                            let mut forbid_objects = block_cell_forbid_objects(before_cell);
                            dedup_objects(&mut forbid_objects);

                            component_cells.push(MatchCellTemplate {
                                offset: offset.clone(),
                                require_null: before_cell.require_null,
                                require_objects,
                                require_object_sets,
                                forbid_objects,
                                require_mark: before_mark.require.clone(),
                                require_object_set_mark: before_mark.require_object_set.clone(),
                                forbid_mark: before_mark.forbid.clone(),
                                forbid_object_set_mark: before_mark.forbid_object_set.clone(),
                            });

                            let before_object_set_objects =
                                object_set_objects_for_occurrences(&before_occurrences);
                            let after_object_set_objects =
                                object_set_objects_for_occurrences(&after_occurrences);
                            let replacements = same_cell_occurrence_replacements(
                                &before_occurrences,
                                &after_occurrences,
                            );

                            for object in before_objects.iter().filter(|object| {
                                !after_objects.contains(object)
                                    && !before_object_set_objects.contains(object)
                                    && !replacements.iter().any(|(remove, _)| remove == *object)
                            }) {
                                writes.push(WriteOpTemplate::Remove {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: *object,
                                });
                            }

                            for object in after_objects.iter().filter(|object| {
                                !before_objects.contains(object)
                                    && !after_object_set_objects.contains(object)
                                    && !replacements.iter().any(|(_, add)| add == *object)
                            }) {
                                writes.push(WriteOpTemplate::Add {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: *object,
                                });
                            }
                            for (remove, add) in replacements {
                                writes.push(WriteOpTemplate::Replace {
                                    component: component_index,
                                    offset: offset.clone(),
                                    remove,
                                    add,
                                });
                            }
                            append_object_set_presence_writes(
                                component_index,
                                &offset,
                                &before_occurrences,
                                &after_occurrences,
                                &mut writes,
                            );

                            for attr in
                                mark_to_set(&after_mark.require, &before_mark.require, line)?
                            {
                                writes.push(WriteOpTemplate::SetMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                });
                            }
                            for attr in mark_to_set_object_set(
                                &after_mark.require_object_set,
                                &before_mark.require_object_set,
                                line,
                            )? {
                                writes.push(WriteOpTemplate::SetObjectSetMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                });
                            }

                            for attr in mark_to_remove(&before_mark.require, &after_mark.require)
                                .into_iter()
                                .filter(|attr| {
                                    attr.object.is_empty() || after_objects.contains(&attr.object)
                                })
                            {
                                writes.push(WriteOpTemplate::RemoveMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in mark_to_remove_object_set(
                                &before_mark.require_object_set,
                                &after_mark.require_object_set,
                            )
                            .into_iter()
                            .filter(|attr| {
                                after_occurrences.iter().any(|occurrence| {
                                    matches!(
                                        &occurrence.matched,
                                        ResolvedObjectMatch::ObjectSet { binding, .. }
                                            if *binding == attr.binding
                                    )
                                })
                            }) {
                                writes.push(WriteOpTemplate::RemoveObjectSetMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }

                            for attr in &after_mark.forbid {
                                writes.push(WriteOpTemplate::RemoveMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in &after_mark.forbid_object_set {
                                writes.push(WriteOpTemplate::RemoveObjectSetMark {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    mark: attr.mark,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }

                            concrete_x += 1;
                        }
                    }

                    components.push(PatternComponentTemplate {
                        cells: component_cells,
                        gap_count: next_gap_index,
                    });
                }

                writes = normalize_occurrence_writes(
                    writes,
                    &before_placements,
                    &after_placements,
                    line,
                )?;

                alternatives.push(RuleBodyAlternative {
                    guards: dynamic_guards.clone(),
                    components,
                    writes,
                    tag_captures,
                });
            }
        }
    }

    Ok(alternatives)
}

fn pattern_block_preserves_once_group(block: &PatternBlock) -> bool {
    block.components.iter().any(|component| {
        component.rows.iter().any(|row| {
            row.iter().any(|part| {
                let BlockPart::Cell(cell) = part else {
                    return false;
                };
                cell.require.iter().any(|selector| {
                    !selector.relative_constraints.is_empty()
                        || selector
                            .transform
                            .as_ref()
                            .is_some_and(|transform| transform.preserves_once_group)
                })
            })
        })
    })
}

fn expand_dynamic_selector_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
) -> Vec<(Vec<CanonicalGuard>, PatternBlock, PatternBlock)> {
    let mut branches = vec![(Vec::new(), before.clone(), after.clone())];
    loop {
        let mut expanded = Vec::new();
        let mut changed = false;
        for (guards, before, after) in branches {
            if let Some(location) = first_dynamic_selector_location(&before) {
                changed = true;
                expand_dynamic_selector_branch(
                    guards,
                    before,
                    after,
                    true,
                    location,
                    &mut expanded,
                );
                continue;
            }
            if let Some(location) = first_dynamic_selector_location(&after) {
                changed = true;
                expand_dynamic_selector_branch(
                    guards,
                    before,
                    after,
                    false,
                    location,
                    &mut expanded,
                );
                continue;
            }
            expanded.push((guards, before, after));
        }
        if !changed {
            return expanded;
        }
        branches = expanded;
    }
}

#[derive(Clone, Copy)]
struct SelectorLocation {
    component: usize,
    row: usize,
    part: usize,
    require: bool,
    selector: usize,
}

fn first_dynamic_selector_location(block: &PatternBlock) -> Option<SelectorLocation> {
    for (component_index, component) in block.components.iter().enumerate() {
        for (row_index, row) in component.rows.iter().enumerate() {
            for (part_index, part) in row.iter().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                if let Some(selector_index) = cell
                    .require
                    .iter()
                    .position(|selector| !selector.dynamic_guards.is_empty())
                {
                    return Some(SelectorLocation {
                        component: component_index,
                        row: row_index,
                        part: part_index,
                        require: true,
                        selector: selector_index,
                    });
                }
                if let Some(selector_index) = cell
                    .forbid
                    .iter()
                    .position(|selector| !selector.dynamic_guards.is_empty())
                {
                    return Some(SelectorLocation {
                        component: component_index,
                        row: row_index,
                        part: part_index,
                        require: false,
                        selector: selector_index,
                    });
                }
            }
        }
    }
    None
}

fn expand_dynamic_selector_branch(
    guards: Vec<CanonicalGuard>,
    before: PatternBlock,
    after: PatternBlock,
    in_before: bool,
    location: SelectorLocation,
    out: &mut Vec<(Vec<CanonicalGuard>, PatternBlock, PatternBlock)>,
) {
    let selector = selector_at_location(if in_before { &before } else { &after }, location);
    for object in &selector.alternatives {
        let mut guards = guards.clone();
        if let Some(dynamic_guards) = selector.dynamic_guards.get(object) {
            guards.extend(
                dynamic_guards
                    .iter()
                    .map(|guard| CanonicalGuard::VariableEquals {
                        variable: guard.variable,
                        value: guard.value,
                    }),
            );
        }
        let mut before = before.clone();
        let mut after = after.clone();
        let target = if in_before { &mut before } else { &mut after };
        let selector = selector_at_location_mut(target, location);
        selector.alternatives = vec![*object];
        selector.dynamic_guards.clear();
        out.push((guards, before, after));
    }
}

fn selector_at_location(block: &PatternBlock, location: SelectorLocation) -> &ObjectSelector {
    let BlockPart::Cell(cell) =
        &block.components[location.component].rows[location.row][location.part]
    else {
        unreachable!("selector locations only point to cells");
    };
    if location.require {
        &cell.require[location.selector]
    } else {
        &cell.forbid[location.selector]
    }
}

fn selector_at_location_mut(
    block: &mut PatternBlock,
    location: SelectorLocation,
) -> &mut ObjectSelector {
    let BlockPart::Cell(cell) =
        &mut block.components[location.component].rows[location.row][location.part]
    else {
        unreachable!("selector locations only point to cells");
    };
    if location.require {
        &mut cell.require[location.selector]
    } else {
        &mut cell.forbid[location.selector]
    }
}

#[derive(Clone, Debug)]
struct MarkSetBinding {
    key: String,
    values: Vec<String>,
}

#[derive(Clone, Debug)]
struct MarkSetOccurrence {
    location: String,
    anchor: String,
    set: String,
    label: Option<String>,
    values: Vec<String>,
}

fn expand_movement_mark_sets(
    before: &PatternBlock,
    after: &PatternBlock,
    value_sets: &HashMap<String, Vec<String>>,
    line: &str,
) -> Result<Vec<(PatternBlock, PatternBlock)>, DiagnosticReport> {
    let before = expand_negated_movement_mark_sets(before, value_sets);
    let after = expand_negated_movement_mark_sets(after, value_sets);
    let before_occurrences = collect_movement_mark_set_occurrences(&before, "before", value_sets);
    let after_occurrences = collect_movement_mark_set_occurrences(&after, "after", value_sets);
    let (mut bindings, occurrence_bindings) =
        resolve_mark_set_bindings(&before_occurrences, &after_occurrences, line)?;
    dedup_mark_set_bindings(&mut bindings);

    if bindings.is_empty() {
        return Ok(vec![(before, after)]);
    }

    let mut assignments = Vec::<HashMap<String, String>>::new();
    expand_mark_set_assignments(&bindings, 0, &mut HashMap::new(), &mut assignments);
    Ok(assignments
        .into_iter()
        .map(|assignment| {
            (
                apply_movement_mark_set_assignment(
                    &before,
                    "before",
                    &occurrence_bindings,
                    &assignment,
                ),
                apply_movement_mark_set_assignment(
                    &after,
                    "after",
                    &occurrence_bindings,
                    &assignment,
                ),
            )
        })
        .collect())
}

fn expand_negated_movement_mark_sets(
    block: &PatternBlock,
    value_sets: &HashMap<String, Vec<String>>,
) -> PatternBlock {
    let mut block = block.clone();
    for component in &mut block.components {
        for row in &mut component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                expand_negated_movement_mark_set_list(&mut cell.require_cell_mark, value_sets);
                expand_negated_movement_mark_set_list(&mut cell.forbid_cell_mark, value_sets);
                for selector in &mut cell.require {
                    expand_negated_movement_mark_set_list(&mut selector.mark, value_sets);
                }
                for selector in &mut cell.forbid {
                    expand_negated_movement_mark_set_list(&mut selector.mark, value_sets);
                }
            }
        }
    }
    block
}

fn expand_negated_movement_mark_set_list(
    mark: &mut Vec<SelectorMark>,
    value_sets: &HashMap<String, Vec<String>>,
) {
    let mut expanded = Vec::with_capacity(mark.len());
    for mark in mark.drain(..) {
        if mark.negated
            && let Some(value) = mark.value.as_deref()
            && let Some(values) = movement_mark_set_values(value, value_sets)
        {
            expanded.extend(values.into_iter().map(|value| {
                let mut mark = mark.clone();
                mark.value = Some(value);
                mark.binding_label = None;
                mark
            }));
        } else {
            expanded.push(mark);
        }
    }
    *mark = expanded;
}

fn collect_movement_mark_set_occurrences(
    block: &PatternBlock,
    side: &str,
    value_sets: &HashMap<String, Vec<String>>,
) -> Vec<MarkSetOccurrence> {
    let mut occurrences = Vec::new();
    let mut selector_counts = HashMap::<String, usize>::new();
    for (component_index, component) in block.components.iter().enumerate() {
        for (row_index, row) in component.rows.iter().enumerate() {
            for (part_index, part) in row.iter().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                collect_cell_mark_set_occurrences(
                    &cell.require_cell_mark,
                    format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    side,
                    value_sets,
                    &mut occurrences,
                );
                collect_cell_mark_set_occurrences(
                    &cell.forbid_cell_mark,
                    format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    side,
                    value_sets,
                    &mut occurrences,
                );
                for selector in &cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    collect_cell_mark_set_occurrences(
                        &selector.mark,
                        format!("object:{}:{ordinal}", selector.token),
                        side,
                        value_sets,
                        &mut occurrences,
                    );
                }
            }
        }
    }
    occurrences
}

fn collect_cell_mark_set_occurrences(
    mark: &[SelectorMark],
    anchor: String,
    side: &str,
    value_sets: &HashMap<String, Vec<String>>,
    occurrences: &mut Vec<MarkSetOccurrence>,
) {
    for (mark_index, mark) in mark.iter().enumerate() {
        let Some((set, label, values)) = movement_mark_set_reference(mark, value_sets) else {
            continue;
        };
        occurrences.push(MarkSetOccurrence {
            location: format!("{side}:{anchor}:{mark_index}"),
            anchor: anchor.clone(),
            set: set.to_string(),
            label,
            values,
        });
    }
}

fn resolve_mark_set_bindings(
    before: &[MarkSetOccurrence],
    after: &[MarkSetOccurrence],
    line: &str,
) -> Result<(Vec<MarkSetBinding>, HashMap<String, String>), DiagnosticReport> {
    let mut bindings = Vec::new();
    let mut occurrence_bindings = HashMap::new();

    for occurrence in before {
        let key = occurrence.label.as_ref().map_or_else(
            || format!("implicit:{}:{}", occurrence.anchor, occurrence.set),
            |label| format!("labeled:{}#{label}", occurrence.set),
        );
        occurrence_bindings.insert(occurrence.location.clone(), key.clone());
        bindings.push(MarkSetBinding {
            key,
            values: occurrence.values.clone(),
        });
    }

    for occurrence in after {
        let candidates = before
            .iter()
            .filter(|candidate| candidate.set == occurrence.set)
            .filter(|candidate| match &occurrence.label {
                Some(label) => candidate.label.as_ref() == Some(label),
                None => true,
            })
            .collect::<Vec<_>>();
        let source = if occurrence.label.is_none() {
            candidates
                .iter()
                .find(|candidate| candidate.anchor == occurrence.anchor)
                .copied()
                .or_else(|| (candidates.len() == 1).then(|| candidates[0]))
        } else {
            (candidates.len() == 1).then(|| candidates[0])
        };
        let Some(source) = source else {
            let message = if candidates.is_empty() {
                format!("unbound movement set reference: {}", occurrence.set)
            } else {
                format!(
                    "ambiguous movement set reference `{}`; add a #label",
                    occurrence.set
                )
            };
            return Err(parse_error(line, &message));
        };
        let key = occurrence_bindings
            .get(&source.location)
            .expect("before movement binding was recorded")
            .clone();
        occurrence_bindings.insert(occurrence.location.clone(), key);
    }

    Ok((bindings, occurrence_bindings))
}

fn dedup_mark_set_bindings(bindings: &mut Vec<MarkSetBinding>) {
    let mut deduped = Vec::with_capacity(bindings.len());
    for binding in bindings.drain(..) {
        if !deduped
            .iter()
            .any(|existing: &MarkSetBinding| existing.key == binding.key)
        {
            deduped.push(binding);
        }
    }
    *bindings = deduped;
}

fn expand_mark_set_assignments(
    bindings: &[MarkSetBinding],
    index: usize,
    current: &mut HashMap<String, String>,
    out: &mut Vec<HashMap<String, String>>,
) {
    if index == bindings.len() {
        out.push(current.clone());
        return;
    }
    let binding = &bindings[index];
    for value in &binding.values {
        current.insert(binding.key.clone(), (*value).to_string());
        expand_mark_set_assignments(bindings, index + 1, current, out);
    }
    current.remove(&binding.key);
}

fn apply_movement_mark_set_assignment(
    block: &PatternBlock,
    side: &str,
    occurrence_bindings: &HashMap<String, String>,
    assignment: &HashMap<String, String>,
) -> PatternBlock {
    let mut block = block.clone();
    let mut selector_counts = HashMap::<String, usize>::new();
    for (component_index, component) in block.components.iter_mut().enumerate() {
        for (row_index, row) in component.rows.iter_mut().enumerate() {
            for (part_index, part) in row.iter_mut().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                apply_cell_mark_set_assignment(
                    &mut cell.require_cell_mark,
                    &format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    side,
                    occurrence_bindings,
                    assignment,
                );
                apply_cell_mark_set_assignment(
                    &mut cell.forbid_cell_mark,
                    &format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    side,
                    occurrence_bindings,
                    assignment,
                );
                for selector in &mut cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    apply_cell_mark_set_assignment(
                        &mut selector.mark,
                        &format!("object:{}:{ordinal}", selector.token),
                        side,
                        occurrence_bindings,
                        assignment,
                    );
                }
            }
        }
    }
    block
}

fn apply_cell_mark_set_assignment(
    mark: &mut [SelectorMark],
    anchor: &str,
    side: &str,
    occurrence_bindings: &HashMap<String, String>,
    assignment: &HashMap<String, String>,
) {
    for (mark_index, mark) in mark.iter_mut().enumerate() {
        let Some(value) = mark.value.as_deref() else {
            continue;
        };
        if !puzzle_authoring::is_movement_mark_set(value) {
            continue;
        }
        let location = format!("{side}:{anchor}:{mark_index}");
        if let Some(key) = occurrence_bindings.get(&location)
            && let Some(concrete) = assignment.get(key)
        {
            mark.value = Some(concrete.clone());
            mark.binding_label = None;
        }
    }
}

fn rectangular_shared_gap_indices(component: &BlockComponent) -> Option<Vec<Option<u16>>> {
    if component.rows.len() <= 1 {
        return None;
    }
    let first = component.rows.first()?;
    if !first.iter().any(|part| matches!(part, BlockPart::Ellipsis)) {
        return None;
    }

    let mut next_gap_index = 0_u16;
    Some(
        first
            .iter()
            .map(|part| {
                if matches!(part, BlockPart::Ellipsis) {
                    let gap_index = next_gap_index;
                    next_gap_index += 1;
                    Some(gap_index)
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn prefer_same_cell_occurrence_keys(
    before_occurrences: &[ResolvedObjectOccurrence],
    after_occurrences: &mut [ResolvedObjectOccurrence],
) {
    let mut used_after = Vec::<usize>::new();
    for before in before_occurrences {
        let Some(key) = before.key.clone() else {
            continue;
        };
        let Some((after_index, after)) =
            after_occurrences
                .iter_mut()
                .enumerate()
                .find(|(index, after)| {
                    !used_after.contains(index)
                        && after.matched == before.matched
                        && !occurrence_key_has_label(&key)
                        && !after.key.as_ref().is_some_and(occurrence_key_has_label)
                })
        else {
            continue;
        };
        after.key = Some(key);
        used_after.push(after_index);
    }
}

fn occurrence_key_has_label(key: &OccurrenceKey) -> bool {
    key.token.contains('#')
}

fn normalize_occurrence_writes(
    writes: Vec<WriteOpTemplate>,
    before_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    after_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    _line: &str,
) -> Result<Vec<WriteOpTemplate>, DiagnosticReport> {
    let before_occurrences = before_placements
        .iter()
        .map(|(key, placement)| puzzle_authoring::RewriteOccurrence {
            key: key.clone(),
            position: RewritePosition2 {
                component: placement.component,
                offset: placement.offset.clone(),
            },
            subject: placement.matched.clone(),
            require_marks: placement
                .require_mark
                .iter()
                .cloned()
                .map(RewriteMark2::Object)
                .chain(
                    placement
                        .require_object_set_mark
                        .iter()
                        .cloned()
                        .map(RewriteMark2::ObjectSet),
                )
                .collect(),
            forbid_marks: Vec::new(),
        })
        .collect::<Vec<_>>();
    let after_occurrences = after_placements
        .iter()
        .map(|(key, placement)| puzzle_authoring::RewriteOccurrence {
            key: key.clone(),
            position: RewritePosition2 {
                component: placement.component,
                offset: placement.offset.clone(),
            },
            subject: placement.matched.clone(),
            require_marks: placement
                .require_mark
                .iter()
                .cloned()
                .map(RewriteMark2::Object)
                .chain(
                    placement
                        .require_object_set_mark
                        .iter()
                        .cloned()
                        .map(RewriteMark2::ObjectSet),
                )
                .collect(),
            forbid_marks: Vec::new(),
        })
        .collect::<Vec<_>>();
    let deltas = puzzle_authoring::diff_rewrite_occurrences(
        &before_occurrences,
        &after_occurrences,
        |left, right| left == right,
    );
    let moves = deltas
        .iter()
        .filter_map(|delta| match delta {
            puzzle_authoring::RewriteOccurrenceDelta::Move { from, to, subject } => {
                Some((from.clone(), to.clone(), subject.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if moves.is_empty() {
        return Ok(writes);
    }

    let mut out = Vec::new();
    for (before, after, subject) in &moves {
        match subject {
            ResolvedObjectMatch::Object(object) => {
                out.push(WriteOpTemplate::Move {
                    component: before.component,
                    from_offset: before.offset.clone(),
                    to_offset: after.offset.clone(),
                    object: *object,
                });
            }
            ResolvedObjectMatch::ObjectSet { binding, .. } => {
                out.push(WriteOpTemplate::MoveObjectSet {
                    component: before.component,
                    from_offset: before.offset.clone(),
                    to_offset: after.offset.clone(),
                    binding: *binding,
                    objects: subject.possible_objects(),
                });
            }
        }
    }
    for delta in deltas {
        let puzzle_authoring::RewriteOccurrenceDelta::RemoveMark { at, subject, mark } = delta
        else {
            continue;
        };
        if !moves
            .iter()
            .any(|(_, destination, moved)| *destination == at && *moved == subject)
        {
            continue;
        }
        match mark {
            RewriteMark2::Object(attr) => out.push(WriteOpTemplate::RemoveMark {
                component: at.component,
                offset: at.offset,
                object: attr.object,
                mark: attr.mark,
                value: attr.value,
                match_value: attr.match_value,
            }),
            RewriteMark2::ObjectSet(attr) => out.push(WriteOpTemplate::RemoveObjectSetMark {
                component: at.component,
                offset: at.offset,
                binding: attr.binding,
                mark: attr.mark,
                value: attr.value,
                match_value: attr.match_value,
            }),
        }
    }

    let moved_placements = moves
        .iter()
        .filter_map(|(before, after, subject)| {
            let before = before_placements.values().find(|placement| {
                placement.component == before.component
                    && placement.offset == before.offset
                    && placement.matched == *subject
            })?;
            let after = after_placements.values().find(|placement| {
                placement.component == after.component
                    && placement.offset == after.offset
                    && placement.matched == *subject
            })?;
            Some((before, after))
        })
        .collect::<Vec<_>>();
    out.extend(writes.into_iter().filter(|write| {
        !moved_placements.iter().any(|(before, after)| {
            write_removes_match_at(write, before)
                || write_adds_match_at(write, after)
                || write_removes_moved_mark_at_before(write, before)
        })
    }));

    Ok(out)
}

fn write_removes_moved_mark_at_before(
    write: &WriteOpTemplate,
    placement: &OccurrencePlacement,
) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::RemoveObjectSetMark {
                component,
                offset,
                binding,
                mark,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
                && placement
                    .require_object_set_mark
                    .iter()
                    .any(|attr| attr.binding == *binding && attr.mark == *mark)
        }
        _ => false,
    }
}

fn write_removes_match_at(write: &WriteOpTemplate, placement: &OccurrencePlacement) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::Remove {
                component,
                offset,
                object,
            },
            ResolvedObjectMatch::Object(placement_object),
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && object == placement_object
        }
        (
            WriteOpTemplate::RemoveObjectSet {
                component,
                offset,
                binding,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
        }
        _ => false,
    }
}

fn write_adds_match_at(write: &WriteOpTemplate, placement: &OccurrencePlacement) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::Add {
                component,
                offset,
                object,
            },
            ResolvedObjectMatch::Object(placement_object),
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && object == placement_object
        }
        (
            WriteOpTemplate::AddObjectSet {
                component,
                offset,
                binding,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
        }
        _ => false,
    }
}

fn block_shapes_match(before: &BlockComponent, after: &BlockComponent) -> bool {
    before.rows.len() == after.rows.len()
        && before.rows.iter().zip(&after.rows).all(|(before, after)| {
            before.len() == after.len()
                && before.iter().zip(after).all(|(before, after)| {
                    matches!(
                        (before, after),
                        (BlockPart::Cell(_), BlockPart::Cell(_))
                            | (BlockPart::Ellipsis, BlockPart::Ellipsis)
                    )
                })
        })
}

fn validate_null_cell_rewrite(
    before: &BlockComponent,
    after: &BlockComponent,
    line: &str,
) -> Result<(), DiagnosticReport> {
    for (before_part, after_part) in before
        .rows
        .iter()
        .flatten()
        .zip(after.rows.iter().flatten())
    {
        let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) = (before_part, after_part)
        else {
            continue;
        };
        puzzle_authoring::validate_null_rewrite_cell(
            before_cell.require_null,
            after_cell.require_null,
            block_cell_is_empty_or_null(after_cell),
        )
        .map_err(|error| parse_error(line, error.message()))?;
    }
    Ok(())
}

fn validate_null_component_has_anchor_cell(
    component: &BlockComponent,
    line: &str,
) -> Result<(), DiagnosticReport> {
    puzzle_authoring::validate_null_pattern_cells(component.rows.iter().flatten().filter_map(
        |part| match part {
            BlockPart::Cell(cell) => Some(cell.require_null),
            BlockPart::Ellipsis => None,
        },
    ))
    .map_err(|error| parse_error(line, error.message()))
}

fn block_cell_is_empty_or_null(cell: &BlockCell) -> bool {
    !cell.keep
        && cell.require.is_empty()
        && cell.forbid.is_empty()
        && cell.require_cell_mark.is_empty()
        && cell.forbid_cell_mark.is_empty()
}

fn block_cell_forbid_objects(cell: &BlockCell) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for selector in &cell.forbid {
        objects.extend(selector.alternatives.iter().copied());
    }
    dedup_objects(&mut objects);
    objects
}

#[derive(Clone, Debug, Default)]
struct BlockCellMark {
    require: Vec<MarkPatternTemplate>,
    require_object_set: Vec<ObjectSetMarkPatternTemplate>,
    forbid: Vec<MarkPatternTemplate>,
    forbid_object_set: Vec<ObjectSetMarkPatternTemplate>,
}

fn block_cell_mark(
    cell: &BlockCell,
    occurrences: &[ResolvedObjectOccurrence],
    mark_names: &HashMap<String, MarkDef>,
    line: &str,
) -> Result<BlockCellMark, DiagnosticReport> {
    let mut out = BlockCellMark::default();
    for mark in &cell.require_cell_mark {
        let pattern = parsed_mark_pattern(ObjectId::EMPTY, mark, mark_names, line)?;
        if mark.negated {
            out.forbid.push(pattern);
        } else {
            out.require.push(pattern);
        }
    }
    for mark in &cell.forbid_cell_mark {
        let pattern = parsed_mark_pattern(ObjectId::EMPTY, mark, mark_names, line)?;
        out.forbid.push(pattern);
    }
    for (selector, occurrence) in cell.require.iter().zip(occurrences) {
        for mark in &selector.mark {
            match &occurrence.matched {
                ResolvedObjectMatch::Object(object) => {
                    let pattern = parsed_mark_pattern(*object, mark, mark_names, line)?;
                    if mark.negated {
                        out.forbid.push(pattern);
                    } else {
                        out.require.push(pattern);
                    }
                }
                ResolvedObjectMatch::ObjectSet { binding, .. } => {
                    let pattern = parsed_object_set_mark_pattern(*binding, mark, mark_names, line)?;
                    if mark.negated {
                        out.forbid_object_set.push(pattern);
                    } else {
                        out.require_object_set.push(pattern);
                    }
                }
            }
        }
    }
    dedup_mark_patterns(&mut out.require);
    dedup_mark_patterns(&mut out.forbid);
    dedup_object_set_mark_patterns(&mut out.require_object_set);
    dedup_object_set_mark_patterns(&mut out.forbid_object_set);
    reject_duplicate_mark_patterns(&out.require, line)?;
    reject_duplicate_object_set_mark_patterns(&out.require_object_set, line)?;
    Ok(out)
}

fn dedup_mark_patterns(patterns: &mut Vec<MarkPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn dedup_object_set_mark_patterns(patterns: &mut Vec<ObjectSetMarkPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn parsed_object_set_mark_pattern(
    binding: u16,
    mark: &SelectorMark,
    mark_names: &HashMap<String, MarkDef>,
    line: &str,
) -> Result<ObjectSetMarkPatternTemplate, DiagnosticReport> {
    let pattern = parsed_mark_pattern(ObjectId::EMPTY, mark, mark_names, line)?;
    Ok(ObjectSetMarkPatternTemplate {
        binding,
        mark: pattern.mark,
        value: pattern.value,
        match_value: pattern.match_value,
        is_flag: pattern.is_flag,
    })
}

fn parsed_mark_pattern(
    object: ObjectId,
    mark: &SelectorMark,
    mark_names: &HashMap<String, MarkDef>,
    line: &str,
) -> Result<MarkPatternTemplate, DiagnosticReport> {
    if mark.name.is_empty() {
        return parsed_anonymous_mark_pattern(object, mark, mark_names, line);
    }
    let def = mark_names
        .get(&mark.name)
        .ok_or_else(|| parse_error(line, "unknown mark"))?;
    let value = match def.kind {
        MarkKind::Flag => {
            if mark.value.is_some() {
                return Err(parse_error(line, "flag mark cannot have a value"));
            }
            None
        }
        MarkKind::Bool => {
            if mark.value.is_some() {
                return Err(parse_error(
                    line,
                    "bool mark uses presence syntax; write `flag` or `no flag`",
                ));
            }
            Some(MarkValueTemplate::Literal(1))
        }
        MarkKind::Int => mark
            .value
            .as_deref()
            .map(|value| {
                value
                    .parse::<i64>()
                    .map(MarkValueTemplate::Literal)
                    .map_err(|_| parse_error(line, "expected integer mark value"))
            })
            .transpose()?,
        MarkKind::Enum => mark
            .value
            .as_deref()
            .map(|value| parse_enum_mark_value(value, def, line))
            .transpose()?,
    };
    let match_value = if value.is_some() {
        MarkValueMatch::Exact
    } else {
        MarkValueMatch::Any
    };
    Ok(MarkPatternTemplate {
        object,
        mark: def.id,
        value,
        match_value,
        is_flag: matches!(def.kind, MarkKind::Flag | MarkKind::Bool),
    })
}

fn parsed_anonymous_mark_pattern(
    object: ObjectId,
    mark: &SelectorMark,
    mark_names: &HashMap<String, MarkDef>,
    line: &str,
) -> Result<MarkPatternTemplate, DiagnosticReport> {
    let value = mark
        .value
        .as_deref()
        .ok_or_else(|| parse_error(line, "anonymous mark must specify a value"))?;
    let kind = puzzle_authoring::mark_sugar_kind(value)
        .ok_or_else(|| parse_error(line, "unknown anonymous mark"))?;
    let (mark_id, value, match_value) = match kind {
        puzzle_authoring::MarkSugarKind::Movement if value == "directions" => {
            (ANONYMOUS_MOVEMENT_MARK, None, MarkValueMatch::Any)
        }
        puzzle_authoring::MarkSugarKind::Movement => (
            ANONYMOUS_MOVEMENT_MARK,
            Some(parse_anonymous_movement_value(value, mark_names, line)?),
            MarkValueMatch::Exact,
        ),
        puzzle_authoring::MarkSugarKind::Bool => (
            ANONYMOUS_BOOL_MARK,
            Some(MarkValueTemplate::Literal(match value {
                "false" => 0,
                "true" => 1,
                _ => return Err(parse_error(line, "expected boolean mark value")),
            })),
            MarkValueMatch::Exact,
        ),
        puzzle_authoring::MarkSugarKind::Int => (
            ANONYMOUS_INT_MARK,
            Some(MarkValueTemplate::Literal(value.parse::<i64>().map_err(
                |_| parse_error(line, "expected integer mark value"),
            )?)),
            MarkValueMatch::Exact,
        ),
    };
    Ok(MarkPatternTemplate {
        object,
        mark: mark_id,
        value,
        match_value,
        is_flag: false,
    })
}

fn parse_anonymous_movement_value(
    value: &str,
    mark_names: &HashMap<String, MarkDef>,
    line: &str,
) -> Result<MarkValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(MarkValueTemplate::Relative(relative));
    }
    let value = puzzle_authoring::canonical_3d_movement_direction_name(value);
    mark_names
        .get("__move")
        .and_then(|definition| {
            definition
                .values
                .iter()
                .position(|candidate| candidate == value)
        })
        .and_then(|index| i64::try_from(index).ok())
        .map(MarkValueTemplate::Literal)
        .ok_or_else(|| parse_error(line, "unknown movement mark value"))
}

fn movement_mark_set_values(
    value: &str,
    value_sets: &HashMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    value_sets.get(value).cloned()
}

fn movement_mark_set_reference<'a>(
    mark: &'a SelectorMark,
    value_sets: &HashMap<String, Vec<String>>,
) -> Option<(&'a str, Option<String>, Vec<String>)> {
    if !mark.name.is_empty() {
        return None;
    }
    let set = mark.value.as_deref()?;
    if puzzle_authoring::mark_sugar_kind(set) != Some(puzzle_authoring::MarkSugarKind::Movement) {
        return None;
    }
    let values = value_sets.get(set).cloned()?;
    if !puzzle_authoring::is_movement_mark_set(set) {
        return None;
    }
    Some((set, mark.binding_label.clone(), values))
}

fn parse_enum_mark_value(
    value: &str,
    def: &MarkDef,
    line: &str,
) -> Result<MarkValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(MarkValueTemplate::Relative(relative));
    }
    def.values
        .iter()
        .position(|candidate| candidate == value)
        .map(|index| MarkValueTemplate::Literal(index as i64))
        .ok_or_else(|| parse_error(line, "unknown enum mark value"))
}

fn parse_relative_direction_value(value: &str) -> Option<RelativeDirection> {
    puzzle_authoring::relative_direction(value)
}

fn reject_duplicate_mark_patterns(
    mark: &[MarkPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(ObjectId, MarkId)>::new();
    for attr in mark {
        let key = (attr.object, attr.mark);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same mark twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn reject_duplicate_object_set_mark_patterns(
    mark: &[ObjectSetMarkPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(u16, MarkId)>::new();
    for attr in mark {
        let key = (attr.binding, attr.mark);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same mark twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn mark_to_set(
    after: &[MarkPatternTemplate],
    before: &[MarkPatternTemplate],
    line: &str,
) -> Result<Vec<MarkPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_flag && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS mark must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn mark_to_set_object_set(
    after: &[ObjectSetMarkPatternTemplate],
    before: &[ObjectSetMarkPatternTemplate],
    line: &str,
) -> Result<Vec<ObjectSetMarkPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_flag && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS mark must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn mark_to_remove(
    before: &[MarkPatternTemplate],
    after: &[MarkPatternTemplate],
) -> Vec<MarkPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn mark_to_remove_object_set(
    before: &[ObjectSetMarkPatternTemplate],
    after: &[ObjectSetMarkPatternTemplate],
) -> Vec<ObjectSetMarkPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn dedup_objects(objects: &mut Vec<ObjectId>) {
    objects.sort_unstable();
    objects.dedup();
}

#[derive(Clone, Debug)]
struct SelectorOccurrence {
    token: String,
    alternatives: Vec<ObjectId>,
    occurrence_label: Option<String>,
    tag_captures: HashMap<ObjectId, Vec<TagCapture>>,
    capture_requirements: HashMap<ObjectId, Vec<CaptureSelectorRequirement>>,
    cell_index: usize,
    binding: u16,
}

#[derive(Clone, Debug)]
enum SelectorAssignmentValue {
    Object(ObjectId),
    ObjectSet {
        binding: u16,
        layer: LayerId,
        objects: Vec<ObjectId>,
    },
}

fn collect_before_occurrences(block: &PatternBlock) -> Vec<SelectorOccurrence> {
    let mut occurrences = Vec::new();
    let mut cell_index = 0usize;
    let mut next_binding = 0u16;
    for component in &block.components {
        for row in &component.rows {
            for part in row {
                if let BlockPart::Cell(cell) = part {
                    for selector in &cell.require {
                        occurrences.push(SelectorOccurrence {
                            token: selector.token.clone(),
                            alternatives: selector.alternatives.clone(),
                            occurrence_label: selector.occurrence_label.clone(),
                            tag_captures: selector.tag_captures.clone(),
                            capture_requirements: selector.capture_requirements.clone(),
                            cell_index,
                            binding: next_binding,
                        });
                        next_binding = next_binding.saturating_add(1);
                    }
                    cell_index += 1;
                }
            }
        }
    }
    occurrences
}

fn reject_duplicate_labeled_occurrences(
    occurrences: &[SelectorOccurrence],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<String>::new();
    for occurrence in occurrences {
        if occurrence.occurrence_label.is_none() {
            continue;
        }
        if seen.contains(&occurrence.token) {
            return Err(parse_error(
                line,
                "selector occurrence label must be unique within the before pattern",
            ));
        }
        seen.push(occurrence.token.clone());
    }
    Ok(())
}

fn expand_selector_assignments(
    occurrences: &[SelectorOccurrence],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<Vec<Vec<SelectorAssignmentValue>>, DiagnosticReport> {
    let mut assignments = vec![Vec::<SelectorAssignmentValue>::new()];
    for (index, occurrence) in occurrences.iter().enumerate() {
        if occurrence.occurrence_label.is_none()
            && !occurrence.token.contains('*')
            && !occurrence.token.contains(':')
            && let Some(layer) = same_layer_alternatives(&occurrence.alternatives, object_layers)
            && selector_occurrence_can_use_object_set(occurrences, index, layer, object_layers)
        {
            let mut next = Vec::new();
            for prefix in &assignments {
                if !selector_assignment_value_is_possible(
                    occurrences,
                    prefix,
                    index,
                    layer,
                    &occurrence.alternatives,
                    object_layers,
                    line,
                )? {
                    continue;
                }
                let mut assignment = prefix.clone();
                assignment.push(SelectorAssignmentValue::ObjectSet {
                    binding: occurrence.binding,
                    layer,
                    objects: occurrence.alternatives.clone(),
                });
                next.push(assignment);
            }
            assignments = next;
            continue;
        }
        let mut next = Vec::new();
        for prefix in &assignments {
            for object in &occurrence.alternatives {
                if !selector_assignment_object_is_possible(
                    occurrences,
                    prefix,
                    index,
                    *object,
                    object_layers,
                    line,
                )? {
                    continue;
                }
                let mut assignment = prefix.clone();
                assignment.push(SelectorAssignmentValue::Object(*object));
                next.push(assignment);
            }
        }
        assignments = next;
    }
    Ok(assignments)
}

fn selector_occurrence_can_use_object_set(
    occurrences: &[SelectorOccurrence],
    index: usize,
    layer: LayerId,
    object_layers: &HashMap<ObjectId, LayerId>,
) -> bool {
    let occurrence = &occurrences[index];
    !occurrences.iter().enumerate().any(|(other_index, other)| {
        other_index != index
            && other.cell_index == occurrence.cell_index
            && other.alternatives.len() > 1
            && (same_layer_alternatives(&other.alternatives, object_layers).is_none()
                || same_layer_alternatives(&other.alternatives, object_layers) == Some(layer))
    })
}

fn same_layer_alternatives(
    alternatives: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
) -> Option<LayerId> {
    if alternatives.len() <= 1 {
        return None;
    }
    puzzle_kernel::object_set_matcher_for_same_layer(0, alternatives, |object| {
        object_layers.get(&object).copied()
    })
    .map(|matcher| matcher.layer)
}

fn selector_assignment_value_is_possible(
    occurrences: &[SelectorOccurrence],
    prefix: &[SelectorAssignmentValue],
    index: usize,
    layer: LayerId,
    objects: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let occurrence = &occurrences[index];
    for (previous_index, previous_value) in prefix.iter().enumerate() {
        let previous = &occurrences[previous_index];
        if previous.cell_index != occurrence.cell_index {
            continue;
        }
        let previous_layer = match previous_value {
            SelectorAssignmentValue::Object(object) => {
                let Some(previous_layer) = object_layers.get(object).copied() else {
                    continue;
                };
                previous_layer
            }
            SelectorAssignmentValue::ObjectSet { layer, .. } => *layer,
        };
        if previous_layer != layer {
            continue;
        }
        if previous.alternatives.len() > 1 || objects.len() > 1 {
            return Ok(false);
        }
        return Err(parse_error(
            line,
            &format!(
                "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                previous.token, occurrence.token
            ),
        ));
    }
    Ok(true)
}

fn selector_assignment_object_is_possible(
    occurrences: &[SelectorOccurrence],
    prefix: &[SelectorAssignmentValue],
    index: usize,
    object: ObjectId,
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let occurrence = &occurrences[index];
    let Some(layer) = object_layers.get(&object) else {
        return Ok(true);
    };
    for (previous_index, previous_value) in prefix.iter().enumerate() {
        let previous = &occurrences[previous_index];
        if previous.cell_index != occurrence.cell_index {
            continue;
        }
        let previous_layer = match previous_value {
            SelectorAssignmentValue::Object(previous_object) => {
                let Some(previous_layer) = object_layers.get(previous_object) else {
                    continue;
                };
                *previous_layer
            }
            SelectorAssignmentValue::ObjectSet { layer, .. } => *layer,
        };
        if previous_layer != *layer {
            continue;
        }
        if previous.alternatives.len() > 1 || occurrence.alternatives.len() > 1 {
            return Ok(false);
        }
        if matches!(previous_value, SelectorAssignmentValue::Object(previous_object) if *previous_object == object)
        {
            continue;
        }
        return Err(parse_error(
            line,
            &format!(
                "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                previous.token, occurrence.token
            ),
        ));
    }
    Ok(true)
}

fn before_occurrences_by_token(occurrences: &[SelectorOccurrence]) -> HashMap<String, Vec<usize>> {
    let mut by_token = HashMap::<String, Vec<usize>>::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        by_token
            .entry(occurrence.token.clone())
            .or_default()
            .push(index);
    }
    by_token
}

fn tag_captures_for_assignment(
    occurrences: &[SelectorOccurrence],
    assignment: &[SelectorAssignmentValue],
) -> TagCaptureValues {
    let mut captures = TagCaptureValues::default();
    for (occurrence, value) in occurrences.iter().zip(assignment) {
        let Some(object) = assignment_concrete_object(value) else {
            continue;
        };
        if let Some(object_captures) = occurrence.tag_captures.get(&object) {
            for capture in object_captures {
                captures.insert(capture);
            }
        }
    }
    captures
}

fn assignment_matches_capture_requirements(
    occurrences: &[SelectorOccurrence],
    assignment: &[SelectorAssignmentValue],
    tag_captures: &TagCaptureValues,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    for (occurrence, value) in occurrences.iter().zip(assignment) {
        let Some(object) = assignment_concrete_object(value) else {
            continue;
        };
        let selector = SelectorRequirementView {
            capture_requirements: &occurrence.capture_requirements,
        };
        if !selector_object_matches_capture_requirements(
            &selector,
            object,
            tag_captures,
            maps,
            line,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

struct SelectorRequirementView<'a> {
    capture_requirements: &'a HashMap<ObjectId, Vec<CaptureSelectorRequirement>>,
}

trait SelectorRequirementSource {
    fn capture_requirements(&self) -> &HashMap<ObjectId, Vec<CaptureSelectorRequirement>>;
}

impl SelectorRequirementSource for ObjectSelector {
    fn capture_requirements(&self) -> &HashMap<ObjectId, Vec<CaptureSelectorRequirement>> {
        &self.capture_requirements
    }
}

impl SelectorRequirementSource for SelectorRequirementView<'_> {
    fn capture_requirements(&self) -> &HashMap<ObjectId, Vec<CaptureSelectorRequirement>> {
        self.capture_requirements
    }
}

fn selector_object_matches_capture_requirements(
    selector: &impl SelectorRequirementSource,
    object: ObjectId,
    tag_captures: &TagCaptureValues,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let Some(requirements) = selector.capture_requirements().get(&object) else {
        return Ok(selector.capture_requirements().is_empty());
    };
    for requirement in requirements {
        match requirement {
            CaptureSelectorRequirement::Direct { key, value } => {
                if tag_captures.resolve_text(key, line)? != *value {
                    return Ok(false);
                }
            }
            CaptureSelectorRequirement::Mapped {
                key,
                map_name,
                value,
            } => {
                let captured = tag_captures.resolve_text(key, line)?;
                let map = maps
                    .get(map_name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                let Some(mapped) = map.values.get(&captured) else {
                    return Err(parse_error(line, "map missing input value"));
                };
                if mapped != value {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn block_cell_object_occurrences(
    cell: &BlockCell,
    assignment: &[SelectorAssignmentValue],
    before_occurrences: &[SelectorOccurrence],
    before_by_token: &HashMap<String, Vec<usize>>,
    token_counts: &mut HashMap<String, usize>,
    tag_captures: &TagCaptureValues,
    maps: &HashMap<String, ValueMap>,
    line: &str,
    source_line_number: Option<usize>,
) -> Result<Vec<ResolvedObjectOccurrence>, DiagnosticReport> {
    cell.require
        .iter()
        .map(|selector| {
            let ordinal = if selector.occurrence_label.is_some() {
                0
            } else {
                let ordinal = *token_counts.get(&selector.token).unwrap_or(&0);
                token_counts.insert(selector.token.clone(), ordinal + 1);
                ordinal
            };
            if let Some(transform) = &selector.transform {
                let source_indices =
                    before_by_token
                        .get(&transform.source_token)
                        .ok_or_else(|| {
                            parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "mapped selector source must appear in before",
                            )
                        })?;
                let before_index = source_indices.get(ordinal).ok_or_else(|| {
                    parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "mapped selector source occurrence missing",
                    )
                })?;
                let source_object = assignment
                    .get(*before_index)
                    .and_then(assignment_concrete_object)
                    .ok_or_else(|| {
                        parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "internal selector assignment missing",
                        )
                    })?;
                let source = before_occurrences.get(*before_index).ok_or_else(|| {
                    parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "mapped selector source occurrence missing",
                    )
                })?;
                let source_ordinal = before_by_token
                    .get(&source.token)
                    .and_then(|indices| indices.iter().position(|index| index == before_index))
                    .ok_or_else(|| {
                        parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "mapped selector source key missing",
                        )
                    })?;
                return transform
                    .mapped_objects
                    .get(&source_object)
                    .copied()
                    .map(|object| ResolvedObjectOccurrence {
                        token: selector.token.clone(),
                        matched: ResolvedObjectMatch::Object(object),
                        key: Some(OccurrenceKey {
                            token: source.token.clone(),
                            ordinal: source_ordinal,
                        }),
                        from_multi_selector: selector.alternatives.len() > 1,
                    })
                    .ok_or_else(|| {
                        parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "mapped selector source object missing",
                        )
                    });
            }
            if let Some(source_token) = &selector.correspondence_source_token
                && let Some(source_indices) = before_by_token.get(source_token)
            {
                let source_index = source_indices.get(ordinal).ok_or_else(|| {
                    parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "relative selector source occurrence missing",
                    )
                })?;
                let source = before_occurrences.get(*source_index).ok_or_else(|| {
                    parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "relative selector source occurrence missing",
                    )
                })?;
                let target = match selector.alternatives.as_slice() {
                    [target] => *target,
                    _ => {
                        return Err(parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "relative selector target must resolve to one object",
                        ));
                    }
                };
                return Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(target),
                    key: Some(OccurrenceKey {
                        token: source.token.clone(),
                        ordinal,
                    }),
                    from_multi_selector: selector.alternatives.len() > 1,
                });
            }
            if let Some(before_occurrences) = before_by_token.get(&selector.token) {
                if let Some(before_index) = before_occurrences.get(ordinal) {
                    return assignment
                        .get(*before_index)
                        .map(|value| ResolvedObjectOccurrence {
                            token: selector.token.clone(),
                            matched: assignment_value_to_match(value),
                            key: Some(OccurrenceKey {
                                token: selector.token.clone(),
                                ordinal,
                            }),
                            from_multi_selector: selector.alternatives.len() > 1,
                        })
                        .ok_or_else(|| {
                            parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "internal selector assignment missing",
                            )
                        });
                }
                if selector.alternatives.len() > 1
                    && selector.occurrence_label.is_none()
                    && before_occurrences.len() == 1
                {
                    return assignment
                        .get(before_occurrences[0])
                        .map(|value| ResolvedObjectOccurrence {
                            token: selector.token.clone(),
                            matched: assignment_value_to_match(value),
                            key: Some(OccurrenceKey {
                                token: selector.token.clone(),
                                ordinal: 0,
                            }),
                            from_multi_selector: true,
                        })
                        .ok_or_else(|| {
                            parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "internal selector assignment missing",
                            )
                        });
                }
            }
            if !selector.capture_requirements.is_empty() {
                let mut candidates = Vec::new();
                for object in &selector.alternatives {
                    if selector_object_matches_capture_requirements(
                        selector,
                        *object,
                        tag_captures,
                        maps,
                        line,
                    )? {
                        candidates.push(*object);
                    }
                }
                match candidates.as_slice() {
                    [object] => {
                        return Ok(ResolvedObjectOccurrence {
                            token: selector.token.clone(),
                            matched: ResolvedObjectMatch::Object(*object),
                            key: None,
                            from_multi_selector: selector.alternatives.len() > 1,
                        });
                    }
                    [] => {
                        return Err(parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "capture-dependent selector matched no objects",
                        ));
                    }
                    _ => {
                        return Err(parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "capture-dependent selector is ambiguous",
                        ));
                    }
                }
            }
            if let Some(family_wildcard) = &selector.family_wildcard {
                let candidates = before_occurrences
                    .iter()
                    .enumerate()
                    .filter_map(|(index, occurrence)| {
                        if let Some(label) = &selector.occurrence_label
                            && occurrence.occurrence_label.as_ref() != Some(label)
                        {
                            return None;
                        }
                        let source = assignment.get(index).and_then(assignment_concrete_object)?;
                        let target = family_wildcard.mapped_objects.get(&source).copied()?;
                        Some((index, target))
                    })
                    .collect::<Vec<_>>();
                let (source_index, target) = if selector.occurrence_label.is_some() {
                    match candidates.as_slice() {
                        [candidate] => *candidate,
                        [] => {
                            return Err(parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "mapped selector source occurrence missing",
                            ));
                        }
                        _ => {
                            return Err(parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "family wildcard selector source is ambiguous",
                            ));
                        }
                    }
                } else {
                    *candidates.get(ordinal).ok_or_else(|| {
                        parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "mapped selector source occurrence missing",
                        )
                    })?
                };
                let source = before_occurrences.get(source_index).ok_or_else(|| {
                    parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "family wildcard selector source occurrence missing",
                    )
                })?;
                let source_ordinal = before_by_token
                    .get(&source.token)
                    .and_then(|indices| indices.iter().position(|index| *index == source_index))
                    .ok_or_else(|| {
                        parse_error_at_source_line_number(
                            line,
                            source_line_number,
                            "family wildcard selector source key missing",
                        )
                    })?;
                return Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(target),
                    key: Some(OccurrenceKey {
                        token: source.token.clone(),
                        ordinal: source_ordinal,
                    }),
                    from_multi_selector: selector.alternatives.len() > 1,
                });
            }
            if selector.alternatives.len() == 1 {
                if selector.occurrence_label.is_some() {
                    return Err(parse_error_at_source_line_number(
                        line,
                        source_line_number,
                        "after selector with an occurrence label must also appear in before",
                    ));
                }
                Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(selector.alternatives[0]),
                    key: None,
                    from_multi_selector: false,
                })
            } else {
                Err(parse_error_at_source_line_number(
                    line,
                    source_line_number,
                    "after selector with alternatives must also appear in before",
                ))
            }
        })
        .collect()
}

fn assignment_concrete_object(value: &SelectorAssignmentValue) -> Option<ObjectId> {
    match value {
        SelectorAssignmentValue::Object(object) => Some(*object),
        SelectorAssignmentValue::ObjectSet { .. } => None,
    }
}

fn assignment_value_to_match(value: &SelectorAssignmentValue) -> ResolvedObjectMatch {
    match value {
        SelectorAssignmentValue::Object(object) => ResolvedObjectMatch::Object(*object),
        SelectorAssignmentValue::ObjectSet {
            binding,
            layer,
            objects,
        } => ResolvedObjectMatch::ObjectSet {
            binding: *binding,
            layer: *layer,
            objects: objects.clone(),
        },
    }
}

fn validate_same_layer_cell_occurrences(
    occurrences: &[ResolvedObjectOccurrence],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let mut seen = Vec::<(LayerId, &ResolvedObjectOccurrence)>::new();
    for occurrence in occurrences {
        let layer = match &occurrence.matched {
            ResolvedObjectMatch::Object(object) => {
                let Some(layer) = object_layers.get(object).copied() else {
                    continue;
                };
                layer
            }
            ResolvedObjectMatch::ObjectSet { layer, .. } => *layer,
        };
        if let Some((_, existing)) = seen
            .iter()
            .find(|(existing_layer, _)| *existing_layer == layer)
        {
            if existing.from_multi_selector || occurrence.from_multi_selector {
                return Ok(false);
            }
            if resolved_occurrences_may_be_same_object(existing, occurrence) {
                continue;
            }
            return Err(parse_error(
                line,
                &format!(
                    "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                    existing.token, occurrence.token
                ),
            ));
        }
        seen.push((layer, occurrence));
    }
    Ok(true)
}

fn possible_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .flat_map(|occurrence| occurrence.matched.possible_objects())
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn concrete_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .filter_map(|occurrence| match occurrence.matched {
            ResolvedObjectMatch::Object(object) => Some(object),
            ResolvedObjectMatch::ObjectSet { .. } => None,
        })
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn object_sets_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectSetMatcher> {
    let mut out = Vec::new();
    for occurrence in occurrences {
        let ResolvedObjectMatch::ObjectSet {
            binding,
            layer,
            objects,
        } = &occurrence.matched
        else {
            continue;
        };
        if out
            .iter()
            .any(|existing: &ObjectSetMatcher| existing.binding == *binding)
        {
            continue;
        }
        out.push(ObjectSetMatcher {
            binding: *binding,
            layer: *layer,
            objects: objects.clone(),
        });
    }
    out
}

fn object_set_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .flat_map(|occurrence| match &occurrence.matched {
            ResolvedObjectMatch::Object(_) => Vec::new(),
            ResolvedObjectMatch::ObjectSet { objects, .. } => objects.clone(),
        })
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn append_object_set_presence_writes(
    component: u16,
    offset: &OffsetTemplate,
    before_occurrences: &[ResolvedObjectOccurrence],
    after_occurrences: &[ResolvedObjectOccurrence],
    writes: &mut Vec<WriteOpTemplate>,
) {
    for before in before_occurrences {
        let ResolvedObjectMatch::ObjectSet { binding, .. } = &before.matched else {
            continue;
        };
        if before.key.is_some()
            && after_occurrences
                .iter()
                .any(|after| after.key == before.key && after.matched == before.matched)
        {
            continue;
        }
        writes.push(WriteOpTemplate::RemoveObjectSet {
            component,
            offset: offset.clone(),
            binding: *binding,
            objects: before.matched.possible_objects(),
        });
    }
    for after in after_occurrences {
        let ResolvedObjectMatch::ObjectSet { binding, .. } = &after.matched else {
            continue;
        };
        if after.key.is_some()
            && before_occurrences
                .iter()
                .any(|before| before.key == after.key && before.matched == after.matched)
        {
            continue;
        }
        writes.push(WriteOpTemplate::AddObjectSet {
            component,
            offset: offset.clone(),
            binding: *binding,
        });
    }
}

fn resolved_occurrences_may_be_same_object(
    left: &ResolvedObjectOccurrence,
    right: &ResolvedObjectOccurrence,
) -> bool {
    left.matched
        .possible_objects()
        .iter()
        .any(|object| right.matched.possible_objects().contains(object))
}

fn same_cell_occurrence_replacements(
    before_occurrences: &[ResolvedObjectOccurrence],
    after_occurrences: &[ResolvedObjectOccurrence],
) -> Vec<(ObjectId, ObjectId)> {
    let mut replacements = Vec::new();
    for before in before_occurrences {
        let (Some(key), ResolvedObjectMatch::Object(remove)) = (&before.key, &before.matched)
        else {
            continue;
        };
        for after in after_occurrences {
            let (Some(after_key), ResolvedObjectMatch::Object(add)) = (&after.key, &after.matched)
            else {
                continue;
            };
            if key == after_key && remove != add && !replacements.contains(&(*remove, *add)) {
                replacements.push((*remove, *add));
            }
        }
    }
    replacements
}

fn direction_by_name(
    name: &str,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
) -> Vec<OrientationEnvironment> {
    let Some(input) = input_names.get(name) else {
        return Vec::new();
    };
    directions
        .iter()
        .copied()
        .filter(|direction| direction.input == *input)
        .collect()
}

fn resolve_write(
    write: &WriteOpTemplate,
    direction: OrientationEnvironment,
    dir_any: bool,
    line: &str,
) -> Result<CanonicalWriteOp, DiagnosticReport> {
    match write {
        WriteOpTemplate::Add {
            component,
            offset,
            object,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::Add {
                component: *component,
                offset,
                object: *object,
            })
        }
        WriteOpTemplate::AddObjectSet {
            component,
            offset,
            binding,
            ..
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::AddObjectSet {
                component: *component,
                offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::Remove {
            component,
            offset,
            object,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::Remove {
                component: *component,
                offset,
                object: *object,
            })
        }
        WriteOpTemplate::RemoveObjectSet {
            component,
            offset,
            binding,
            ..
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::RemoveObjectSet {
                component: *component,
                offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::Replace {
            component,
            offset,
            remove,
            add,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::Replace {
                component: *component,
                offset,
                remove: *remove,
                add: *add,
            })
        }
        WriteOpTemplate::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            let from_offset = resolve_offset(from_offset.clone(), direction, dir_any, line)?;
            let to_offset = resolve_offset(to_offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::Move {
                component: *component,
                from_offset,
                to_offset,
                object: *object,
            })
        }
        WriteOpTemplate::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
            ..
        } => {
            let from_offset = resolve_offset(from_offset.clone(), direction, dir_any, line)?;
            let to_offset = resolve_offset(to_offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::MoveObjectSet {
                component: *component,
                from_offset,
                to_offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::SetMark {
            component,
            offset,
            object,
            mark,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::SetMark {
                component: *component,
                offset,
                object: *object,
                mark: *mark,
                value: resolve_mark_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::SetObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::SetObjectSetMark {
                component: *component,
                offset,
                binding: *binding,
                mark: *mark,
                value: resolve_mark_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::RemoveMark {
            component,
            offset,
            object,
            mark,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::RemoveMark {
                component: *component,
                offset,
                object: *object,
                mark: *mark,
                value: resolve_mark_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
        WriteOpTemplate::RemoveObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(CanonicalWriteOp::RemoveObjectSetMark {
                component: *component,
                offset,
                binding: *binding,
                mark: *mark,
                value: resolve_mark_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
    }
}

fn resolve_mark_patterns(
    patterns: Vec<MarkPatternTemplate>,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<MarkPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(MarkPattern {
                object: pattern.object,
                mark: pattern.mark,
                value: resolve_mark_value(
                    pattern.value.as_ref(),
                    direction,
                    direction_expanded,
                    line,
                )?,
                match_value: pattern.match_value,
            })
        })
        .collect()
}

fn resolve_object_set_mark_patterns(
    patterns: Vec<ObjectSetMarkPatternTemplate>,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<ObjectSetMarkPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(ObjectSetMarkPattern {
                binding: pattern.binding,
                mark: pattern.mark,
                value: resolve_mark_value(
                    pattern.value.as_ref(),
                    direction,
                    direction_expanded,
                    line,
                )?,
                match_value: pattern.match_value,
            })
        })
        .collect()
}

fn resolve_mark_value(
    value: Option<&MarkValueTemplate>,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<Option<i64>, DiagnosticReport> {
    match value {
        Some(MarkValueTemplate::Literal(value)) => Ok(Some(*value)),
        Some(MarkValueTemplate::Relative(relative)) => {
            let absolute =
                resolve_relative_direction(*relative, direction, direction_expanded, line)?;
            Ok(Some(direction_value(direction, absolute)?))
        }
        None => Ok(None),
    }
}

fn resolve_relative_direction(
    relative: RelativeDirection,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<puzzle_kernel::SpatialVector<3>, DiagnosticReport> {
    if !direction_expanded {
        return Err(parse_error(
            line,
            "relative direction mark value requires an oriented rule",
        ));
    }
    Ok(direction.relative_vector(relative))
}

fn direction_value(
    environment: OrientationEnvironment,
    direction: puzzle_kernel::SpatialVector<3>,
) -> Result<i64, DiagnosticReport> {
    environment.direction_value(direction).ok_or_else(|| {
        DiagnosticReport::error("relative mark resolved outside the direction domain".to_string())
    })
}

fn direction_tag_name(
    environment: OrientationEnvironment,
    direction: puzzle_kernel::SpatialVector<3>,
    line: &str,
) -> Result<&'static str, DiagnosticReport> {
    environment.direction_name(direction).ok_or_else(|| {
        parse_error(
            line,
            "relative selector resolved outside the direction domain",
        )
    })
}

fn resolve_offset(
    offset: OffsetTemplate,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<CanonicalOffset, DiagnosticReport> {
    let base = resolve_oriented_xy(
        offset.oriented_x,
        offset.oriented_y,
        direction,
        direction_expanded,
        line,
    )?;
    if offset.gap_terms.is_empty() {
        return Ok(CanonicalOffset::Fixed { delta: base });
    }

    let step = resolve_oriented_xy(1, 0, direction, direction_expanded, line)?;
    Ok(CanonicalOffset::Variable {
        base,
        gap_terms: offset
            .gap_terms
            .iter()
            .copied()
            .map(|gap_index| CanonicalGapTerm {
                gap_index,
                delta: step,
            })
            .collect(),
    })
}

fn resolve_oriented_xy(
    x: i16,
    y: i16,
    direction: OrientationEnvironment,
    direction_expanded: bool,
    line: &str,
) -> Result<puzzle_kernel::SpatialVector<3>, DiagnosticReport> {
    if !direction_expanded {
        return Ok(puzzle_kernel::SpatialVector::new([x, y, 0]));
    }
    let _ = line;
    Ok(direction.frame.project_xy(x, y))
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_qualified_identifier(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier(first) && parts.all(is_identifier)
}

fn is_value_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_char(token: Option<&&str>, line: &str, message: &str) -> Result<char, DiagnosticReport> {
    let value = expect(token, line, message)?;
    let mut chars = value.chars();
    let ch = chars.next().ok_or_else(|| parse_error(line, message))?;
    if chars.next().is_some() {
        return Err(parse_error(line, "expected single character"));
    }
    Ok(ch)
}

fn parse_u16(token: Option<&&str>, line: &str, message: &str) -> Result<u16, DiagnosticReport> {
    expect(token, line, message)?
        .parse()
        .map_err(|_| parse_error(line, "expected u16"))
}

fn parse_variable_value(token: &str, line: &str) -> Result<i64, DiagnosticReport> {
    match token {
        "true" => Ok(1),
        "false" => Ok(0),
        _ => token
            .parse()
            .map_err(|_| parse_error(line, "expected true, false, or integer")),
    }
}

fn expect<'a>(
    token: Option<&'a &str>,
    line: &str,
    message: &str,
) -> Result<&'a str, DiagnosticReport> {
    token.copied().ok_or_else(|| parse_error(line, message))
}

trait DiagnosticSourceLine {
    fn diagnostic_text(&self) -> &str;

    fn diagnostic_line_number(&self) -> Option<usize> {
        None
    }
}

impl DiagnosticSourceLine for str {
    fn diagnostic_text(&self) -> &str {
        self
    }
}

impl DiagnosticSourceLine for String {
    fn diagnostic_text(&self) -> &str {
        self
    }
}

impl<T: DiagnosticSourceLine + ?Sized> DiagnosticSourceLine for &T {
    fn diagnostic_text(&self) -> &str {
        (*self).diagnostic_text()
    }

    fn diagnostic_line_number(&self) -> Option<usize> {
        (*self).diagnostic_line_number()
    }
}

impl DiagnosticSourceLine for source::LogicalLine {
    fn diagnostic_text(&self) -> &str {
        &self.text
    }

    fn diagnostic_line_number(&self) -> Option<usize> {
        Some(self.line)
    }
}

fn parse_error(line: &(impl DiagnosticSourceLine + ?Sized), message: &str) -> DiagnosticReport {
    match line.diagnostic_line_number() {
        Some(number) => {
            DiagnosticReport::error_at_source_line_number(message, line.diagnostic_text(), number)
        }
        None => DiagnosticReport::error_at_line(message, line.diagnostic_text()),
    }
}

fn parse_error_at_source_line_number(
    line: &str,
    source_line_number: Option<usize>,
    message: &str,
) -> DiagnosticReport {
    match source_line_number {
        Some(source_line_number) => {
            DiagnosticReport::error_at_source_line_number(message, line, source_line_number)
        }
        None => parse_error(line, message),
    }
}
