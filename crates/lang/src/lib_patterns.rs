fn wrap_rewrite_steps(application: RuleApplication, steps: Vec<RuleStep>) -> Vec<RuleStep> {
    if matches!(
        application,
        RuleApplication::Random | RuleApplication::UntilStable
    ) {
        vec![RuleStep::Block {
            application,
            stop_condition: None,
            steps,
        }]
    } else {
        steps
    }
}

fn validate_visual_effects(
    effects: &LoweredEffects,
    source_line: &str,
) -> Result<(), DiagnosticReport> {
    if !lowered_effects_change_gameplay(effects) {
        return Ok(());
    }
    Err(DiagnosticReport::error_at_line(
        "display block rewrites cannot use gameplay effects",
        source_line,
    ))
}

fn lowered_effects_change_gameplay(effects: &LoweredEffects) -> bool {
    !effects.core.is_empty() || effects.ordered.iter().any(rule_effect_changes_gameplay)
}

fn rule_effect_changes_gameplay(effect: &RuleEffect) -> bool {
    matches!(
        effect,
        RuleEffect::Win
            | RuleEffect::Restart
            | RuleEffect::NextLevel
            | RuleEffect::Again
            | RuleEffect::Checkpoint
            | RuleEffect::ClearCheckpoint
            | RuleEffect::Scene(_)
    )
}

fn validate_visual_writes(
    writes: &[WriteOp],
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    for write in writes {
        match write {
            WriteOp::Add { object, .. }
            | WriteOp::Remove { object, .. }
            | WriteOp::Move { object, .. } => {
                ensure_visual_write_object(*object, visual_objects)?;
            }
            WriteOp::AddObjectSet { .. }
            | WriteOp::RemoveObjectSet { .. }
            | WriteOp::MoveObjectSet { .. }
            | WriteOp::SetObjectSetScratch { .. }
            | WriteOp::RemoveObjectSetScratch { .. } => {}
            WriteOp::Replace { remove, add, .. } => {
                ensure_visual_write_object(*remove, visual_objects)?;
                ensure_visual_write_object(*add, visual_objects)?;
            }
            WriteOp::SetScratch { object, .. } | WriteOp::RemoveScratch { object, .. } => {
                if !object.is_empty() {
                    ensure_visual_write_object(*object, visual_objects)?;
                }
            }
        }
    }
    Ok(())
}

fn ensure_visual_write_object(
    object: ObjectId,
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    if visual_objects.contains(&object) {
        return Ok(());
    }
    Err(DiagnosticReport::error(
        "display block can read main objects but can only write display objects".to_string(),
    ))
}

#[derive(Clone, Debug, Default)]
struct RuleBodyAlternative {
    guards: Vec<Guard>,
    components: Vec<PatternComponentTemplate>,
    writes: Vec<WriteOpTemplate>,
    tag_captures: TagCaptureValues,
}

fn append_move_sound_effects(
    components: &[PatternComponentTemplate],
    writes: &[WriteOpTemplate],
    triggers: &[ModelSoundTrigger],
    ordered_effects: &mut Vec<RuleEffect>,
) {
    if triggers.is_empty() {
        return;
    }
    for trigger in triggers {
        let moves_trigger_object = writes.iter().any(|write| {
            matches!(
                write,
                WriteOpTemplate::Move { object, .. } if trigger.objects.contains(object)
            ) || matches!(
                write,
                WriteOpTemplate::MoveObjectSet { objects, .. }
                    if objects.iter().any(|object| trigger.objects.contains(object))
            )
        });
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
        let matches_cantmove_intent = trigger.kind == ModelSoundTriggerKind::CantMove
            && !moves_trigger_object
            && cantmove_intent_is_consumed(components, writes, trigger);
        let matches_trigger = matches_trigger || matches_cantmove_intent;
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

fn cantmove_intent_is_consumed(
    components: &[PatternComponentTemplate],
    writes: &[WriteOpTemplate],
    trigger: &ModelSoundTrigger,
) -> bool {
    writes.iter().any(|write| match write {
        WriteOpTemplate::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            ..
        } => {
            *scratch == ANONYMOUS_MOVEMENT_SCRATCH
                && trigger.objects.contains(object)
                && component_cell_has_object_movement_intent(
                    components, *component, offset, *object,
                )
        }
        WriteOpTemplate::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            ..
        } => {
            *scratch == ANONYMOUS_MOVEMENT_SCRATCH
                && component_cell_has_object_set_movement_intent(
                    components, *component, offset, *binding, trigger,
                )
        }
        _ => false,
    })
}

fn component_cell_has_object_movement_intent(
    components: &[PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
    object: ObjectId,
) -> bool {
    component_cell(components, component, offset).is_some_and(|cell| {
        cell.require_scratch.iter().any(|scratch| {
            scratch.scratch == ANONYMOUS_MOVEMENT_SCRATCH && scratch.object == object
        })
    })
}

fn component_cell_has_object_set_movement_intent(
    components: &[PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
    binding: u16,
    trigger: &ModelSoundTrigger,
) -> bool {
    component_cell(components, component, offset).is_some_and(|cell| {
        let binding_matches_trigger = cell.require_object_sets.iter().any(|object_set| {
            object_set.binding == binding
                && object_set
                    .objects
                    .iter()
                    .any(|object| trigger.objects.contains(object))
        });
        binding_matches_trigger
            && cell.require_object_set_scratch.iter().any(|scratch| {
                scratch.scratch == ANONYMOUS_MOVEMENT_SCRATCH && scratch.binding == binding
            })
    })
}

fn component_cell<'a>(
    components: &'a [PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
) -> Option<&'a MatchCellTemplate> {
    components
        .get(component as usize)?
        .cells
        .iter()
        .find(|cell| cell.offset == *offset)
}

fn append_tween_rule_animations(
    writes: &[WriteOpTemplate],
    animation: &AnimationDef,
    animations: &mut Vec<RuleAnimation>,
) {
    if !animation.tween.enabled {
        return;
    }
    let mut objects = Vec::new();
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
    });
}

fn parse_inline_rewrite(
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
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
    let (before, after) = line
        .split_once("->")
        .ok_or_else(|| parse_error(line, "inline rewrite must contain ->"))?;
    let before = parse_pattern_side(
        before.trim(),
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
        false,
    )?;
    let (after, effects, after_effects, after_call) = split_rewrite_suffix(after.trim(), line)?;
    let after = if after.is_empty() {
        before.clone()
    } else {
        let after = parse_pattern_side(
            after,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
            true,
        )?;
        normalize_rhs_keep_cells(&before, after, line)?
    };

    Ok((before, after, effects, after_effects, after_call))
}

fn split_rewrite_suffix<'a>(
    after: &'a str,
    line: &str,
) -> Result<(&'a str, Vec<EffectAst>, Vec<EffectAst>, Option<String>), DiagnosticReport> {
    let Some(last_block_end) = after.rfind(']') else {
        return parse_rewrite_effect(after, line).map(|effects| ("", effects, Vec::new(), None));
    };
    let pattern = after[..=last_block_end].trim();
    let suffix = after[last_block_end + 1..].trim();
    if suffix.is_empty() {
        return Ok((pattern, Vec::new(), Vec::new(), None));
    }

    let tokens = split_header_tokens(suffix);
    if matches!(tokens.as_slice(), [name] if is_qualified_identifier(name))
        && !is_builtin_rewrite_effect_text(suffix)
    {
        return Ok((pattern, Vec::new(), Vec::new(), Some(suffix.to_string())));
    }

    parse_rewrite_effect(suffix, line).map(|effects| (pattern, Vec::new(), effects, None))
}

fn normalize_rhs_keep_cells(
    before: &PatternBlock,
    mut after: PatternBlock,
    line: &str,
) -> Result<PatternBlock, DiagnosticReport> {
    if before.components.len() != after.components.len() {
        return Err(parse_error(
            line,
            "before and after sides must have the same number of blocks",
        ));
    }

    for (before_component, after_component) in before.components.iter().zip(&mut after.components) {
        if !block_shapes_match(before_component, after_component) {
            return Err(parse_error(
                line,
                "before and after blocks must have matching cell and ellipsis layout",
            ));
        }
        for (before_row, after_row) in before_component.rows.iter().zip(&mut after_component.rows) {
            for (before_part, after_part) in before_row.iter().zip(after_row) {
                let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) =
                    (before_part, after_part)
                else {
                    continue;
                };
                if after_cell.keep {
                    *after_cell = before_cell.clone();
                }
            }
        }
    }

    Ok(after)
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedRewriteEffect {
    pub(crate) surface: SurfaceRewriteEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

fn parse_rewrite_effect(suffix: &str, line: &str) -> Result<Vec<EffectAst>, DiagnosticReport> {
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
    let document = rewrite_effect_surface_document(&tokens);
    let effects = parse_rewrite_effect_value(suffix, line)?;
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
            return Ok(vec![EffectAst::Message {
                text,
                literal: true,
            }]);
        }
        if parse_view_path(text).is_some() {
            return Ok(vec![EffectAst::Message {
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
        ["wait"] => Ok(vec![EffectAst::Wait { milliseconds: None }]),
        ["wait", "animation"] | ["wait", "tween"] => Ok(vec![EffectAst::WaitAnimation]),
        ["wait", duration] => Ok(vec![EffectAst::Wait {
            milliseconds: Some(parse_wait_duration_ms(duration, line)?),
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
        [name, op, value] if is_global_update_operator(op) => Ok(vec![EffectAst::UpdateGlobal {
            name: (*name).to_string(),
            op: parse_global_update_op(op, line)?,
            value: parse_global_update_value(value, line)?,
        }]),
        _ => Err(parse_error(
            line,
            "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, or <global> <op> <value>",
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
        SceneEffect::Sequence(effects) => {
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
                        milliseconds: Some(parse_wait_duration_ms(tokens[index + 1], line)?),
                    });
                    index += 2;
                } else {
                    effects.push(EffectAst::Wait { milliseconds: None });
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
            name if index + 2 < tokens.len() && is_global_update_operator(tokens[index + 1]) => {
                effects.push(EffectAst::UpdateGlobal {
                    name: name.to_string(),
                    op: parse_global_update_op(tokens[index + 1], line)?,
                    value: parse_global_update_value(tokens[index + 2], line)?,
                });
                index += 3;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, or <global> <op> <value>",
                ));
            }
        }
    }
    Ok(effects)
}

fn is_rewrite_effect_command_token(token: &str) -> bool {
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

fn is_builtin_rewrite_effect_text(suffix: &str) -> bool {
    if suffix.strip_prefix("message ").is_some() || suffix.strip_prefix("emit ").is_some() {
        return true;
    }
    let tokens = split_header_tokens(suffix);
    matches!(
        tokens.as_slice(),
        [command] if command.eq_ignore_ascii_case("cancel") || command.eq_ignore_ascii_case("win") || command.eq_ignore_ascii_case("restart") || command.eq_ignore_ascii_case("next_level") || command.eq_ignore_ascii_case("again") || command.eq_ignore_ascii_case("checkpoint") || command.eq_ignore_ascii_case("clear_checkpoint")
    ) || matches!(tokens.as_slice(), ["goto", ..] | ["start", ..])
        || matches!(
            tokens.as_slice(),
            ["sfx", _]
                | ["play_music", _]
                | ["pause_music"]
                | ["pause_music", _]
                | ["resume_music"]
                | ["resume_music", _]
                | ["stop_music"]
                | ["stop_music", _]
                | ["wait"]
                | ["wait", _]
        )
        || matches!(tokens.as_slice(), [_, op, _] if is_global_update_operator(op))
}

fn is_global_update_operator(op: &str) -> bool {
    matches!(op, "=" | "+=" | "-=" | "*=" | "/=" | "%=")
}

fn parse_global_update_op(op: &str, line: &str) -> Result<GlobalUpdateOp, DiagnosticReport> {
    match op {
        "=" => Ok(GlobalUpdateOp::Set),
        "+=" => Ok(GlobalUpdateOp::Add),
        "-=" => Ok(GlobalUpdateOp::Subtract),
        "*=" => Ok(GlobalUpdateOp::Multiply),
        "/=" => Ok(GlobalUpdateOp::Divide),
        "%=" => Ok(GlobalUpdateOp::Remainder),
        _ => Err(parse_error(line, "unknown global update operator")),
    }
}

fn parse_global_update_value(token: &str, line: &str) -> Result<GlobalValueAst, DiagnosticReport> {
    if let Ok(value) = parse_global_value(token, line) {
        return Ok(GlobalValueAst::Literal(value));
    }
    validate_tag_capture_reference(token, line)?;
    Ok(GlobalValueAst::TagCapture(token.to_string()))
}

fn validate_tag_capture_reference(token: &str, line: &str) -> Result<(), DiagnosticReport> {
    if token == "*" {
        return Ok(());
    }
    if let Some(label) = token.strip_prefix("*#") {
        return validate_tag_capture_label(label, line);
    }
    if let Some((name, label)) = token.split_once('#') {
        if !is_identifier(name) {
            return Err(parse_error(
                line,
                "tag capture reference must be *, *#label, name, or name#label",
            ));
        }
        return validate_tag_capture_label(label, line);
    }
    if is_identifier(token) {
        return Ok(());
    }
    Err(parse_error(
        line,
        "global update value must be true, false, integer, or tag capture reference",
    ))
}

fn neutral_direction() -> Direction {
    Direction {
        input: InputId(0),
        dx: 1,
        dy: 0,
    }
}

fn rewrite_requires_implicit_cardinal_expansion(rewrite: &OrientedRewriteAst) -> bool {
    pattern_block_requires_implicit_cardinal_expansion(&rewrite.before)
        || pattern_block_requires_implicit_cardinal_expansion(&rewrite.after)
}

fn pattern_block_requires_implicit_cardinal_expansion(block: &PatternBlock) -> bool {
    block.components.iter().any(|component| {
        component.rows.len() > 1
            || component.rows.iter().any(|row| {
                row.len() > 1
                    || row.iter().any(|part| match part {
                        BlockPart::Cell(cell) => block_cell_has_relative_direction(cell),
                        BlockPart::Ellipsis => true,
                    })
            })
    })
}

fn block_cell_has_relative_direction(cell: &BlockCell) -> bool {
    cell.require
        .iter()
        .chain(&cell.forbid)
        .any(selector_has_relative_direction)
}

fn selector_has_relative_direction(selector: &ObjectSelector) -> bool {
    if !selector.relative_constraints.is_empty() {
        return true;
    }
    selector.scratch.iter().any(|scratch| {
        scratch.value.as_deref().is_some_and(|value| {
            parse_relative_direction_value(value).is_some()
                || movement_scratch_set_values(value).is_some()
        })
    })
}

fn resolve_relative_selectors_in_block(
    block: &PatternBlock,
    direction: Direction,
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
            }
        }
    }
    Ok(block)
}

fn resolve_relative_selector(
    selector: &mut ObjectSelector,
    direction: Direction,
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
        let value = direction_tag_name(absolute, line)?;
        let allowed = constraint
            .alternatives_by_direction
            .get(value)
            .ok_or_else(|| parse_error(line, "relative direction selector target is unknown"))?;
        selector
            .alternatives
            .retain(|object| allowed.contains(object));
    }
    if selector.alternatives.is_empty() {
        return Err(parse_error(
            line,
            "relative direction selector matched no objects",
        ));
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
    require_scratch: Vec<ScratchPatternTemplate>,
    require_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
    forbid_scratch: Vec<ScratchPatternTemplate>,
    forbid_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScratchPatternTemplate {
    object: ObjectId,
    scratch: ScratchId,
    value: Option<ScratchValueTemplate>,
    match_value: ScratchValueMatch,
    is_marker: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjectSetScratchPatternTemplate {
    binding: u16,
    scratch: ScratchId,
    value: Option<ScratchValueTemplate>,
    match_value: ScratchValueMatch,
    is_marker: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScratchValueTemplate {
    Literal(i64),
    Relative(RelativeDirection),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelativeDirection {
    Forward,
    Backward,
    Left,
    Right,
}

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
        objects: Vec<ObjectId>,
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
    SetScratch {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
    },
    SetObjectSetScratch {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
    },
    RemoveScratch {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
        match_value: ScratchValueMatch,
    },
    RemoveObjectSetScratch {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
        match_value: ScratchValueMatch,
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
    require_cell_scratch: Vec<ParsedScratch>,
    forbid_cell_scratch: Vec<ParsedScratch>,
}

#[derive(Clone, Debug)]
struct ObjectSelector {
    token: String,
    alternatives: Vec<ObjectId>,
    transform: Option<SelectorTransform>,
    family_wildcard: Option<FamilyWildcardSelector>,
    relative_constraints: Vec<RelativeSelectorConstraint>,
    dynamic_guards: HashMap<ObjectId, Vec<DynamicSelectorGuard>>,
    tag_captures: HashMap<ObjectId, Vec<TagCapture>>,
    scratch: Vec<ParsedScratch>,
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
    count: usize,
    value: String,
}

impl TagCaptureValues {
    fn insert(&mut self, capture: &TagCapture) {
        self.values
            .entry(capture.key.clone())
            .and_modify(|existing| {
                existing.count += 1;
            })
            .or_insert_with(|| TagCaptureValue {
                count: 1,
                value: capture.value.clone(),
            });
    }

    fn resolve(&self, key: &str, line: &str) -> Result<i64, DiagnosticReport> {
        let Some(value) = self.values.get(key) else {
            return Err(parse_error(
                line,
                &format!("unknown tag capture reference: {key}"),
            ));
        };
        if value.count != 1 {
            return Err(parse_error(
                line,
                &format!("tag capture reference `{key}` is ambiguous"),
            ));
        }
        parse_global_value(&value.value, line).map_err(|_| {
            parse_error(
                line,
                "tag capture values used in var updates must be true, false, or integers",
            )
        })
    }
}

#[derive(Clone, Debug)]
struct RelativeSelectorConstraint {
    relative: RelativeDirection,
    alternatives_by_direction: HashMap<String, Vec<ObjectId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DynamicSelectorGuard {
    name: String,
    global: GlobalId,
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
    require_scratch: Vec<ScratchPatternTemplate>,
    require_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedScratch {
    name: String,
    value: Option<String>,
    negated: bool,
    anonymous: Option<AnonymousScratch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AnonymousScratch {
    Movement,
    Bool,
    Int,
}

impl ParsedScratch {
    fn named(name: &str, value: Option<&str>, negated: bool) -> Self {
        Self {
            name: name.to_string(),
            value: value.map(str::to_string),
            negated,
            anonymous: None,
        }
    }

    fn anonymous(kind: AnonymousScratch, value: &str, negated: bool) -> Self {
        Self {
            name: String::new(),
            value: Some(value.to_string()),
            negated,
            anonymous: Some(kind),
        }
    }
}

#[derive(Clone, Debug)]
struct SelectorTransform {
    source_token: String,
    mapped_objects: HashMap<ObjectId, ObjectId>,
}

#[derive(Clone, Debug)]
struct FamilyWildcardSelector {
    mapped_objects: HashMap<ObjectId, ObjectId>,
}

fn parse_pattern_side(
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<PatternBlock, DiagnosticReport> {
    let mut components = Vec::new();
    let mut rest = line.trim();

    while !rest.is_empty() {
        let Some(inner_start) = rest.strip_prefix('[') else {
            return Err(parse_error(
                line,
                "pattern side must contain bracketed blocks",
            ));
        };
        let Some(close_index) = inner_start.find(']') else {
            return Err(parse_error(line, "pattern block missing ]"));
        };
        let inner = &inner_start[..close_index];
        components.push(BlockComponent {
            rows: parse_block_rows(
                inner,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
                allow_keep_marker,
            )?,
        });
        rest = inner_start[close_index + 1..].trim_start();
    }

    if components.is_empty() {
        return Err(parse_error(
            line,
            "pattern side must contain at least one block",
        ));
    }

    Ok(PatternBlock { components })
}

fn parse_block_rows(
    inner: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<Vec<Vec<BlockPart>>, DiagnosticReport> {
    let rows = inner
        .split(';')
        .map(str::trim)
        .map(|row| {
            parse_block_parts(
                row,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
                allow_keep_marker,
            )
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    if rows.is_empty() {
        return Err(parse_error(
            line,
            "pattern block must contain at least one row",
        ));
    }
    validate_rectangular_ellipsis_layout(&rows, line)?;

    Ok(rows)
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
        .expect("parse_block_rows already rejected empty blocks");
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

fn parse_block_parts(
    inner: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<Vec<BlockPart>, DiagnosticReport> {
    let parts = inner
        .split('|')
        .map(str::trim)
        .map(|cell| {
            if cell == "..." {
                Ok(BlockPart::Ellipsis)
            } else {
                Ok(BlockPart::Cell(parse_block_cell(
                    cell,
                    line,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                    allow_keep_marker,
                )?))
            }
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    if parts.is_empty() {
        return Err(parse_error(
            line,
            "pattern block must contain at least one cell",
        ));
    }

    Ok(parts)
}

fn parse_block_cell(
    cell: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<BlockCell, DiagnosticReport> {
    let mut parsed = BlockCell::default();
    let cell_tokens = split_cell_tokens(cell, line)?;
    if cell_tokens.iter().any(|token| token == "=") {
        if !allow_keep_marker {
            return Err(parse_error(line, "`=` is only valid as a RHS cell"));
        }
        if cell_tokens.len() != 1 {
            return Err(parse_error(
                line,
                "`=` RHS cell cannot contain other tokens",
            ));
        }
        parsed.keep = true;
        return Ok(parsed);
    }
    let mut tokens = cell_tokens.iter().map(String::as_str).peekable();
    while let Some(token) = tokens.next() {
        if let Some(scratch) = parse_cell_scratch_token(token, line)? {
            if parsed.require_null {
                return Err(parse_error(line, "`null` cell pattern cannot contain other tokens"));
            }
            parsed.require_cell_scratch.extend(scratch);
            continue;
        }
        if let Some(anonymous) = anonymous_scratch_for_token(token) {
            if parsed.require_null {
                return Err(parse_error(line, "`null` cell pattern cannot contain other tokens"));
            }
            let selector = tokens
                .next()
                .ok_or_else(|| parse_error(line, "scratch sugar must be followed by a selector"))?;
            if selector == "no" || anonymous_scratch_for_token(selector).is_some() {
                return Err(parse_error(
                    line,
                    "scratch sugar must be followed by a selector",
                ));
            }
            let mut selector = resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?;
            selector
                .scratch
                .push(ParsedScratch::anonymous(anonymous, token, false));
            parsed.require.push(selector);
            continue;
        }
        if token == "no" {
            if parsed.require_null {
                return Err(parse_error(line, "`null` cell pattern cannot contain other tokens"));
            }
            let selector = tokens
                .next()
                .ok_or_else(|| parse_error(line, "`no` must be followed by a selector"))?;
            if selector == "no" {
                return Err(parse_error(line, "`no no` is not a valid cell pattern"));
            }
            if selector == "null" {
                return Err(parse_error(line, "`no null` is not a valid cell pattern"));
            }
            if let Some(scratch) = parse_cell_scratch_token(selector, line)? {
                parsed.forbid_cell_scratch.extend(scratch);
                continue;
            }
            parsed.forbid.push(resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?);
        } else {
            if token == "null" {
                if parsed.require_null
                    || !parsed.require.is_empty()
                    || !parsed.forbid.is_empty()
                    || !parsed.require_cell_scratch.is_empty()
                    || !parsed.forbid_cell_scratch.is_empty()
                {
                    return Err(parse_error(
                        line,
                        "`null` cell pattern cannot contain other tokens",
                    ));
                }
                parsed.require_null = true;
                continue;
            }
            if parsed.require_null {
                return Err(parse_error(line, "`null` cell pattern cannot contain other tokens"));
            }
            parsed.require.push(resolve_object_selector(
                token,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?);
        }
    }

    Ok(parsed)
}

fn parse_cell_scratch_token(
    token: &str,
    line: &str,
) -> Result<Option<Vec<ParsedScratch>>, DiagnosticReport> {
    let Some(inner) = token
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return Ok(None);
    };
    Ok(Some(parse_selector_scratch(inner, line)?))
}

fn anonymous_scratch_for_token(token: &str) -> Option<AnonymousScratch> {
    match puzzle_authoring::scratch_sugar_kind(token)? {
        puzzle_authoring::ScratchSugarKind::Movement => Some(AnonymousScratch::Movement),
        puzzle_authoring::ScratchSugarKind::Bool => Some(AnonymousScratch::Bool),
        puzzle_authoring::ScratchSugarKind::Int => Some(AnonymousScratch::Int),
    }
}

fn split_cell_tokens(cell: &str, line: &str) -> Result<Vec<String>, DiagnosticReport> {
    puzzle_authoring::split_cell_tokens(cell).map_err(|error| match error {
        puzzle_authoring::CellTokenError::UnmatchedCloseBrace => {
            parse_error(line, "scratch block has unmatched }")
        }
        puzzle_authoring::CellTokenError::MissingCloseBrace => {
            parse_error(line, "scratch block missing }")
        }
    })
}

fn resolve_object_selector(
    selector: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let (selector, scratch) = split_selector_scratch(selector, line)?;
    let (selector, occurrence_label) = split_selector_occurrence_label(selector, line)?;
    let token = labeled_selector_token(selector, occurrence_label.as_deref());
    if !selector.contains(':')
        && let Some(object) = object_names.get(selector).copied()
    {
        return Ok(ObjectSelector {
            token,
            alternatives: vec![object],
            transform: None,
            family_wildcard: None,
            relative_constraints: Vec::new(),
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            scratch,
            occurrence_label,
        });
    }

    if let Some(objects) = object_groups.get(selector) {
        return Ok(ObjectSelector {
            token,
            alternatives: objects.clone(),
            transform: None,
            family_wildcard: None,
            relative_constraints: Vec::new(),
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            scratch,
            occurrence_label,
        });
    }

    let parts = selector.split(':').collect::<Vec<_>>();
    if parts.first().copied() == Some("*") {
        return resolve_schema_family_wildcard_selector(
            &parts,
            token,
            scratch,
            occurrence_label,
            line,
            object_schemas,
            value_sets,
            global_names,
        );
    }
    let Some(schema) = object_schemas.get(parts[0]) else {
        if parts.len() > 1 && value_sets.contains_key(parts[0]) {
            return resolve_qualified_value_set_selector(
                selector,
                token,
                scratch,
                occurrence_label,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            );
        }
        return Err(parse_error(line, "unknown object selector"));
    };

    validate_schema_selector_arity(&parts, schema, line, "object selector")?;
    if parts.len() == 1 {
        return Err(parse_error(
            line,
            "object selector for variants must use :* or explicit variant tags",
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
            if value == "*" {
                source_token_parts.push("*".to_string());
                return Ok(tag_capture_key.map(|key| SelectorConstraint::Capture {
                    axis_index: index,
                    key,
                }));
            }
            if let Some(relative) = parse_relative_direction_value(value) {
                if axis != "directions" {
                    return Err(parse_error(
                        line,
                        "relative direction selector tag requires a directions tag slot",
                    ));
                }
                source_token_parts.push((*value).to_string());
                return Ok(Some(SelectorConstraint::Relative {
                    axis_index: index,
                    relative,
                }));
            }
            let expr = parse_value_expr(value, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                if global_names.contains_key(axis) {
                    return Err(ambiguous_selector_tag_error(axis, parts[0], axis, line));
                }
                source_token_parts.push(axis.clone());
                Ok(Some(SelectorConstraint::Capture {
                    axis_index: index,
                    key: tag_capture_key.unwrap_or_else(|| axis.clone()),
                }))
            } else if let ValueExpr::MapCall { arg, .. } = &expr {
                if arg != axis {
                    return Err(parse_error(
                        line,
                        "map argument must match selector tag set",
                    ));
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
                source_token_parts.push(axis.clone());
                Ok(Some(SelectorConstraint::Mapped {
                    axis_index: index,
                    expr,
                }))
            } else if let ValueExpr::Binding(name) = &expr {
                let axis_values = schema_axis_values(schema, index)?;
                let names_axis_value = axis_values.contains(name);
                let names_value_set = value_sets.contains_key(name);
                let global = global_names.get(name).copied();
                if (names_axis_value && names_value_set)
                    || (global.is_some() && (names_axis_value || names_value_set))
                {
                    return Err(ambiguous_selector_tag_error(name, parts[0], axis, line));
                }
                if let Some(values) = value_sets.get(name) {
                    validate_selector_subset(name, values, &axis_values, parts[0], axis, line)?;
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::ValueSet(values.clone())))
                } else if names_axis_value {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::Fixed(name.clone())))
                } else if let Some(global) = global {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::DynamicGlobal {
                        axis_index: index,
                        name: name.clone(),
                        global,
                    }))
                } else {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::Fixed(name.clone())))
                }
            } else {
                source_token_parts.push((*value).to_string());
                Ok(Some(SelectorConstraint::Fixed((*value).to_string())))
            }
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

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
                    | Some(SelectorConstraint::DynamicGlobal { .. })
                    | None => true,
                })
        })
        .map(|variant| variant.object)
        .collect::<Vec<_>>();

    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }
    let relative_constraints = relative_selector_constraints(&constraints, schema, &alternatives)?;

    if constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::Mapped { .. })))
    {
        let source_token = labeled_selector_token(
            &format!("{}:{}", parts[0], source_token_parts.join(":")),
            occurrence_label.as_deref(),
        );
        let mut mapped_objects = HashMap::new();
        let mut target_objects = Vec::new();
        for source in &schema.variants {
            let mut values = source.values.clone();
            for constraint in constraints.iter().flatten() {
                if let SelectorConstraint::Mapped { axis_index, expr } = constraint {
                    let axis = &schema.axes[*axis_index];
                    let mut env = ValueEnv::default();
                    env.bind(axis, axis, &source.values[*axis_index]);
                    values[*axis_index] = eval_bound_value_expr(expr, &env, maps, line)?;
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
            }),
            family_wildcard: None,
            relative_constraints,
            dynamic_guards: HashMap::new(),
            tag_captures: HashMap::new(),
            scratch,
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
        relative_constraints,
        dynamic_guards,
        tag_captures,
        scratch,
        occurrence_label,
    })
}

fn resolve_qualified_value_set_selector(
    selector: &str,
    token: String,
    scratch: Vec<ParsedScratch>,
    occurrence_label: Option<String>,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
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
            global_names,
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
        relative_constraints: Vec::new(),
        dynamic_guards,
        tag_captures: HashMap::new(),
        scratch,
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
    scratch: Vec<ParsedScratch>,
    occurrence_label: Option<String>,
    line: &str,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    if parts.len() != 2 {
        return Err(parse_error(
            line,
            "family wildcard object selector must be *:<tag>",
        ));
    }
    let tag = parts[1];
    if tag == "_" {
        return Err(parse_error(
            line,
            "object selector wildcard must use *; _ is reserved for completion",
        ));
    }

    let (mut alternatives, family_wildcard) = if tag == "*" {
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
        if global_names.contains_key(&name) && (names_schema_tag || value_set.is_some()) {
            return Err(parse_error(
                line,
                &format!(
                    "selector tag {name} is ambiguous for family wildcard selector: it is both a schema tag and a global"
                ),
            ));
        }
        if global_names.contains_key(&name) {
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
        relative_constraints: Vec::new(),
        dynamic_guards: HashMap::new(),
        tag_captures: HashMap::new(),
        scratch,
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

fn split_selector_occurrence_label<'a>(
    selector: &'a str,
    line: &str,
) -> Result<(&'a str, Option<String>), DiagnosticReport> {
    if selector.contains(':') {
        return Ok((selector, None));
    }
    let Some((base, label)) = selector.split_once('#') else {
        return Ok((selector, None));
    };
    if base.is_empty() || label.is_empty() || label.contains('#') {
        return Err(parse_error(
            line,
            "selector occurrence label must be: selector#label",
        ));
    }
    if !label
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err(parse_error(
            line,
            "selector occurrence label may only contain letters, numbers, and _",
        ));
    }
    Ok((base, Some(label.to_string())))
}

fn selector_tag_capture_key<'a>(
    value: &'a str,
    axis: &str,
    axis_count: usize,
    line: &str,
) -> Result<(&'a str, Option<String>), DiagnosticReport> {
    let Some((base, label)) = value.split_once('#') else {
        if value == axis {
            return Ok((value, Some(axis.to_string())));
        }
        if value == "*" && axis_count == 1 {
            return Ok((value, Some("*".to_string())));
        }
        return Ok((value, None));
    };
    validate_tag_capture_label(label, line)?;
    if base == "*" {
        return Ok((base, Some(format!("*#{label}"))));
    }
    if base == axis {
        return Ok((base, Some(format!("{axis}#{label}"))));
    }
    Err(parse_error(
        line,
        "tag capture labels must attach to * or the schema tag slot name",
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

fn split_selector_scratch<'a>(
    selector: &'a str,
    line: &str,
) -> Result<(&'a str, Vec<ParsedScratch>), DiagnosticReport> {
    let Some(open_index) = selector.find('{') else {
        return Ok((selector, Vec::new()));
    };
    let base = &selector[..open_index];
    let attrs = selector[open_index + 1..]
        .strip_suffix('}')
        .ok_or_else(|| parse_error(line, "scratch selector must end with }"))?;
    if base.is_empty() {
        return Err(parse_error(
            line,
            "scratch selector must attach to an object",
        ));
    }
    let attrs = parse_selector_scratch(attrs, line)?;
    Ok((base, attrs))
}

fn parse_selector_scratch(attrs: &str, line: &str) -> Result<Vec<ParsedScratch>, DiagnosticReport> {
    let mut parsed = Vec::new();
    let mut tokens = attrs.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        let (negated, spec) = if token == "no" {
            let spec = tokens
                .next()
                .ok_or_else(|| parse_error(line, "`no` must be followed by an scratch"))?;
            (true, spec)
        } else {
            (false, token)
        };
        if let Some(anonymous) = anonymous_scratch_for_token(spec) {
            parsed.push(ParsedScratch::anonymous(anonymous, spec, negated));
            continue;
        }
        let (name, value) = spec
            .split_once('=')
            .map_or((spec, None), |(name, value)| (name, Some(value)));
        validate_scratch_name(name, line)?;
        if value.is_some_and(str::is_empty) {
            return Err(parse_error(line, "scratch value must not be empty"));
        }
        parsed.push(ParsedScratch::named(name, value, negated));
    }
    Ok(parsed)
}

fn validate_scratch_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return Err(parse_error(
            line,
            "scratch name must start with an identifier and may use :value parts",
        ));
    };
    if !is_identifier(first) || !parts.all(is_scratch_name_value_atom) {
        return Err(parse_error(
            line,
            "scratch name must start with an identifier and may use :value parts",
        ));
    }
    Ok(())
}

fn is_scratch_name_value_atom(value: &str) -> bool {
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
    DynamicGlobal {
        axis_index: usize,
        name: String,
        global: GlobalId,
    },
}

fn dynamic_selector_guards(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    line: &str,
) -> Result<HashMap<ObjectId, Vec<DynamicSelectorGuard>>, DiagnosticReport> {
    if !constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::DynamicGlobal { .. })))
    {
        return Ok(HashMap::new());
    }

    let mut guards = HashMap::<ObjectId, Vec<DynamicSelectorGuard>>::new();
    for variant in &schema.variants {
        let mut variant_guards = Vec::new();
        for constraint in constraints.iter().flatten() {
            let SelectorConstraint::DynamicGlobal {
                axis_index,
                name,
                global,
            } = constraint
            else {
                continue;
            };
            let value = variant.values.get(*axis_index).ok_or_else(|| {
                DiagnosticReport::error("internal schema variant missing tag value".to_string())
            })?;
            variant_guards.push(DynamicSelectorGuard {
                name: name.clone(),
                global: *global,
                value: parse_global_value(value, line).map_err(|_| {
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

fn validate_schema_selector_arity(
    parts: &[&str],
    schema: &ObjectSchema,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let slot_count = parts.len().saturating_sub(1);
    if parts.iter().skip(1).any(|part| *part == "_") {
        return Err(parse_error(
            line,
            &format!("{label} wildcard must use *; _ is reserved for completion"),
        ));
    }
    if slot_count > schema.axes.len() {
        return Err(parse_error(line, &format!("{label} has too many tags")));
    }
    if slot_count == 0 {
        return Ok(());
    }
    if slot_count == 1 && parts[1] == "*" {
        return Ok(());
    }
    if slot_count < schema.axes.len() {
        return Err(parse_error(
            line,
            &format!("{label} must name every variant slot; use * for unconstrained slots"),
        ));
    }
    Ok(())
}

fn schema_selector_part<'a>(
    parts: &'a [&str],
    schema: &ObjectSchema,
    axis_index: usize,
) -> Option<&'a str> {
    if parts.len() == 2 && parts[1] == "*" && schema.axes.len() > 1 {
        return Some("*");
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
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
    source_line_number: Option<usize>,
) -> Result<(PatternBlock, Vec<RuleBodyAlternative>), DiagnosticReport> {
    let before = resolve_relative_selectors_in_block(before, direction, direction_expanded, line)?;
    let after = resolve_relative_selectors_in_block(after, direction, direction_expanded, line)?;
    let alternatives = compile_before_after_blocks(
        &before,
        &after,
        object_layers,
        scratch_names,
        value_sets,
        line,
        source_line_number,
    )?;
    Ok((before, alternatives))
}

fn compile_before_after_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    _value_sets: &HashMap<String, Vec<String>>,
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
    let occupancy_objects = object_layers
        .iter()
        .filter_map(|(object, layer)| (layer.0 > 0).then_some(*object))
        .collect::<Vec<_>>();

    let expanded_blocks = expand_movement_scratch_sets(before, after);
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
                                line,
                                source_line_number,
                            )?;
                            let mut after_occurrences = block_cell_object_occurrences(
                                after_cell,
                                &assignment,
                                all_before_occurrences,
                                &before_by_token,
                                &mut after_token_counts,
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
                            let before_scratch = block_cell_scratch(
                                before_cell,
                                &before_occurrences,
                                scratch_names,
                                line,
                            )?;
                            let after_scratch = block_cell_scratch(
                                after_cell,
                                &after_occurrences,
                                scratch_names,
                                line,
                            )?;
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
                                            require_scratch: before_scratch
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
                                            require_object_set_scratch: before_scratch
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
                                        require_scratch: after_scratch
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
                                        require_object_set_scratch: after_scratch
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
                            forbid_objects.extend(implicit_layer_forbids(
                                &before_objects,
                                &after_objects,
                                object_layers,
                                &occupancy_objects,
                            ));
                            dedup_objects(&mut forbid_objects);

                            component_cells.push(MatchCellTemplate {
                                offset: offset.clone(),
                                require_null: before_cell.require_null,
                                require_objects,
                                require_object_sets,
                                forbid_objects,
                                require_scratch: before_scratch.require.clone(),
                                require_object_set_scratch: before_scratch
                                    .require_object_set
                                    .clone(),
                                forbid_scratch: before_scratch.forbid.clone(),
                                forbid_object_set_scratch: before_scratch.forbid_object_set.clone(),
                            });

                            let before_object_set_objects =
                                object_set_objects_for_occurrences(&before_occurrences);
                            let after_object_set_objects =
                                object_set_objects_for_occurrences(&after_occurrences);

                            for object in before_objects.iter().filter(|object| {
                                !after_objects.contains(object)
                                    && !before_object_set_objects.contains(object)
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
                            }) {
                                writes.push(WriteOpTemplate::Add {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: *object,
                                });
                            }
                            append_object_set_presence_writes(
                                component_index,
                                &offset,
                                &before_occurrences,
                                &after_occurrences,
                                &mut writes,
                            );

                            for attr in scratch_to_set(
                                &after_scratch.require,
                                &before_scratch.require,
                                line,
                            )? {
                                writes.push(WriteOpTemplate::SetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                });
                            }
                            for attr in scratch_to_set_object_set(
                                &after_scratch.require_object_set,
                                &before_scratch.require_object_set,
                                line,
                            )? {
                                writes.push(WriteOpTemplate::SetObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                });
                            }

                            for attr in
                                scratch_to_remove(&before_scratch.require, &after_scratch.require)
                                    .into_iter()
                                    .filter(|attr| {
                                        attr.object.is_empty()
                                            || after_objects.contains(&attr.object)
                                    })
                            {
                                writes.push(WriteOpTemplate::RemoveScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in scratch_to_remove_object_set(
                                &before_scratch.require_object_set,
                                &after_scratch.require_object_set,
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
                                writes.push(WriteOpTemplate::RemoveObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }

                            for attr in &after_scratch.forbid {
                                writes.push(WriteOpTemplate::RemoveScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in &after_scratch.forbid_object_set {
                                writes.push(WriteOpTemplate::RemoveObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
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

                writes = preserve_moved_occurrence_scratch(
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

fn expand_dynamic_selector_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
) -> Vec<(Vec<Guard>, PatternBlock, PatternBlock)> {
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
    guards: Vec<Guard>,
    before: PatternBlock,
    after: PatternBlock,
    in_before: bool,
    location: SelectorLocation,
    out: &mut Vec<(Vec<Guard>, PatternBlock, PatternBlock)>,
) {
    let selector = selector_at_location(if in_before { &before } else { &after }, location);
    for object in &selector.alternatives {
        let mut guards = guards.clone();
        if let Some(dynamic_guards) = selector.dynamic_guards.get(object) {
            guards.extend(dynamic_guards.iter().map(|guard| Guard::GlobalEquals {
                global: guard.global,
                value: guard.value,
            }));
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
struct ScratchSetBinding {
    key: String,
    values: &'static [&'static str],
}

fn expand_movement_scratch_sets(
    before: &PatternBlock,
    after: &PatternBlock,
) -> Vec<(PatternBlock, PatternBlock)> {
    let before = expand_negated_movement_scratch_sets(before);
    let after = expand_negated_movement_scratch_sets(after);
    let mut bindings = Vec::<ScratchSetBinding>::new();
    collect_movement_scratch_set_bindings(&before, &mut bindings);
    collect_movement_scratch_set_bindings(&after, &mut bindings);
    dedup_scratch_set_bindings(&mut bindings);

    if bindings.is_empty() {
        return vec![(before, after)];
    }

    let mut assignments = Vec::<HashMap<String, String>>::new();
    expand_scratch_set_assignments(&bindings, 0, &mut HashMap::new(), &mut assignments);
    assignments
        .into_iter()
        .map(|assignment| {
            (
                apply_movement_scratch_set_assignment(&before, &assignment),
                apply_movement_scratch_set_assignment(&after, &assignment),
            )
        })
        .collect()
}

fn expand_negated_movement_scratch_sets(block: &PatternBlock) -> PatternBlock {
    let mut block = block.clone();
    for component in &mut block.components {
        for row in &mut component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                expand_negated_movement_scratch_set_list(&mut cell.require_cell_scratch);
                expand_negated_movement_scratch_set_list(&mut cell.forbid_cell_scratch);
                for selector in &mut cell.require {
                    expand_negated_movement_scratch_set_list(&mut selector.scratch);
                }
                for selector in &mut cell.forbid {
                    expand_negated_movement_scratch_set_list(&mut selector.scratch);
                }
            }
        }
    }
    block
}

fn expand_negated_movement_scratch_set_list(scratch: &mut Vec<ParsedScratch>) {
    let mut expanded = Vec::with_capacity(scratch.len());
    for scratch in scratch.drain(..) {
        if scratch.negated
            && let Some(value) = scratch.value.as_deref()
            && let Some(values) = movement_scratch_set_values(value)
        {
            expanded.extend(values.iter().map(|value| {
                let mut scratch = scratch.clone();
                scratch.value = Some((*value).to_string());
                scratch
            }));
        } else {
            expanded.push(scratch);
        }
    }
    *scratch = expanded;
}

fn collect_movement_scratch_set_bindings(
    block: &PatternBlock,
    bindings: &mut Vec<ScratchSetBinding>,
) {
    let mut selector_counts = HashMap::<String, usize>::new();
    for (component_index, component) in block.components.iter().enumerate() {
        for (row_index, row) in component.rows.iter().enumerate() {
            for (part_index, part) in row.iter().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                collect_cell_scratch_set_bindings(
                    &cell.require_cell_scratch,
                    format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    bindings,
                );
                collect_cell_scratch_set_bindings(
                    &cell.forbid_cell_scratch,
                    format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    bindings,
                );
                for selector in &cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    collect_cell_scratch_set_bindings(
                        &selector.scratch,
                        format!("object:{}:{ordinal}", selector.token),
                        bindings,
                    );
                }
            }
        }
    }
}

fn collect_cell_scratch_set_bindings(
    scratch: &[ParsedScratch],
    anchor: String,
    bindings: &mut Vec<ScratchSetBinding>,
) {
    for (scratch_index, scratch) in scratch.iter().enumerate() {
        let Some(value) = scratch.value.as_deref() else {
            continue;
        };
        let Some(values) = movement_scratch_set_values(value) else {
            continue;
        };
        bindings.push(ScratchSetBinding {
            key: format!("{anchor}:{scratch_index}:{value}"),
            values,
        });
    }
}

fn dedup_scratch_set_bindings(bindings: &mut Vec<ScratchSetBinding>) {
    let mut deduped = Vec::with_capacity(bindings.len());
    for binding in bindings.drain(..) {
        if !deduped
            .iter()
            .any(|existing: &ScratchSetBinding| existing.key == binding.key)
        {
            deduped.push(binding);
        }
    }
    *bindings = deduped;
}

fn expand_scratch_set_assignments(
    bindings: &[ScratchSetBinding],
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
        expand_scratch_set_assignments(bindings, index + 1, current, out);
    }
    current.remove(&binding.key);
}

fn apply_movement_scratch_set_assignment(
    block: &PatternBlock,
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
                apply_cell_scratch_set_assignment(
                    &mut cell.require_cell_scratch,
                    &format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    assignment,
                );
                apply_cell_scratch_set_assignment(
                    &mut cell.forbid_cell_scratch,
                    &format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    assignment,
                );
                for selector in &mut cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    apply_cell_scratch_set_assignment(
                        &mut selector.scratch,
                        &format!("object:{}:{ordinal}", selector.token),
                        assignment,
                    );
                }
            }
        }
    }
    block
}

fn apply_cell_scratch_set_assignment(
    scratch: &mut [ParsedScratch],
    anchor: &str,
    assignment: &HashMap<String, String>,
) {
    for (scratch_index, scratch) in scratch.iter_mut().enumerate() {
        let Some(value) = scratch.value.as_deref() else {
            continue;
        };
        if movement_scratch_set_values(value).is_none() {
            continue;
        }
        let key = format!("{anchor}:{scratch_index}:{value}");
        if let Some(concrete) = assignment.get(&key) {
            scratch.value = Some(concrete.clone());
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

fn preserve_moved_occurrence_scratch(
    writes: Vec<WriteOpTemplate>,
    before_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    after_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    _line: &str,
) -> Result<Vec<WriteOpTemplate>, DiagnosticReport> {
    let moves = before_placements
        .iter()
        .filter_map(|(key, before)| {
            let after = after_placements.get(key)?;
            (before.matched == after.matched
                && before.component == after.component
                && before.offset != after.offset)
                .then_some((before, after))
        })
        .collect::<Vec<_>>();

    if moves.is_empty() {
        return Ok(writes);
    }

    let mut out = Vec::new();
    for (before, after) in &moves {
        match &before.matched {
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
                    objects: before.matched.possible_objects(),
                });
            }
        }

        for attr in scratch_to_remove(&before.require_scratch, &after.require_scratch) {
            out.push(WriteOpTemplate::RemoveScratch {
                component: after.component,
                offset: after.offset.clone(),
                object: attr.object,
                scratch: attr.scratch,
                value: attr.value,
                match_value: attr.match_value,
            });
        }
        for attr in scratch_to_remove_object_set(
            &before.require_object_set_scratch,
            &after.require_object_set_scratch,
        ) {
            out.push(WriteOpTemplate::RemoveObjectSetScratch {
                component: after.component,
                offset: after.offset.clone(),
                binding: attr.binding,
                scratch: attr.scratch,
                value: attr.value,
                match_value: attr.match_value,
            });
        }
    }

    out.extend(writes.into_iter().filter(|write| {
        !moves.iter().any(|(before, after)| {
            write_removes_match_at(write, before)
                || write_adds_match_at(write, after)
                || write_removes_moved_scratch_at_before(write, before)
        })
    }));

    Ok(out)
}

fn write_removes_moved_scratch_at_before(
    write: &WriteOpTemplate,
    placement: &OccurrencePlacement,
) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::RemoveObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
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
                    .require_object_set_scratch
                    .iter()
                    .any(|attr| attr.binding == *binding && attr.scratch == *scratch)
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
    for (before_part, after_part) in before.rows.iter().flatten().zip(after.rows.iter().flatten()) {
        let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) =
            (before_part, after_part)
        else {
            continue;
        };
        if after_cell.require_null && !before_cell.require_null {
            return Err(parse_error(
                line,
                "`null` can only be matched on the before side of a rewrite",
            ));
        }
        if before_cell.require_null && !block_cell_is_empty_or_null(after_cell) {
            return Err(parse_error(
                line,
                "`null` matched cells cannot be written to",
            ));
        }
    }
    Ok(())
}

fn validate_null_component_has_anchor_cell(
    component: &BlockComponent,
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut has_null = false;
    let mut has_non_null_cell = false;
    for part in component.rows.iter().flatten() {
        let BlockPart::Cell(cell) = part else {
            continue;
        };
        if cell.require_null {
            has_null = true;
        } else {
            has_non_null_cell = true;
        }
    }
    if has_null && !has_non_null_cell {
        return Err(parse_error(
            line,
            "`null` patterns must include at least one non-null cell",
        ));
    }
    Ok(())
}

fn block_cell_is_empty_or_null(cell: &BlockCell) -> bool {
    !cell.keep
        && cell.require.is_empty()
        && cell.forbid.is_empty()
        && cell.require_cell_scratch.is_empty()
        && cell.forbid_cell_scratch.is_empty()
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
struct BlockCellScratch {
    require: Vec<ScratchPatternTemplate>,
    require_object_set: Vec<ObjectSetScratchPatternTemplate>,
    forbid: Vec<ScratchPatternTemplate>,
    forbid_object_set: Vec<ObjectSetScratchPatternTemplate>,
}

fn block_cell_scratch(
    cell: &BlockCell,
    occurrences: &[ResolvedObjectOccurrence],
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<BlockCellScratch, DiagnosticReport> {
    let mut out = BlockCellScratch::default();
    for scratch in &cell.require_cell_scratch {
        let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
        if scratch.negated {
            out.forbid.push(pattern);
        } else {
            out.require.push(pattern);
        }
    }
    for scratch in &cell.forbid_cell_scratch {
        let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
        out.forbid.push(pattern);
    }
    for (selector, occurrence) in cell.require.iter().zip(occurrences) {
        for scratch in &selector.scratch {
            match &occurrence.matched {
                ResolvedObjectMatch::Object(object) => {
                    let pattern = parsed_scratch_pattern(*object, scratch, scratch_names, line)?;
                    if scratch.negated {
                        out.forbid.push(pattern);
                    } else {
                        out.require.push(pattern);
                    }
                }
                ResolvedObjectMatch::ObjectSet { binding, .. } => {
                    let pattern =
                        parsed_object_set_scratch_pattern(*binding, scratch, scratch_names, line)?;
                    if scratch.negated {
                        out.forbid_object_set.push(pattern);
                    } else {
                        out.require_object_set.push(pattern);
                    }
                }
            }
        }
    }
    dedup_scratch_patterns(&mut out.require);
    dedup_scratch_patterns(&mut out.forbid);
    dedup_object_set_scratch_patterns(&mut out.require_object_set);
    dedup_object_set_scratch_patterns(&mut out.forbid_object_set);
    reject_duplicate_scratch_patterns(&out.require, line)?;
    reject_duplicate_object_set_scratch_patterns(&out.require_object_set, line)?;
    Ok(out)
}

fn dedup_scratch_patterns(patterns: &mut Vec<ScratchPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn dedup_object_set_scratch_patterns(patterns: &mut Vec<ObjectSetScratchPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn parsed_object_set_scratch_pattern(
    binding: u16,
    scratch: &ParsedScratch,
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<ObjectSetScratchPatternTemplate, DiagnosticReport> {
    let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
    Ok(ObjectSetScratchPatternTemplate {
        binding,
        scratch: pattern.scratch,
        value: pattern.value,
        match_value: pattern.match_value,
        is_marker: pattern.is_marker,
    })
}

fn parsed_scratch_pattern(
    object: ObjectId,
    scratch: &ParsedScratch,
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<ScratchPatternTemplate, DiagnosticReport> {
    if let Some(anonymous) = &scratch.anonymous {
        return parsed_anonymous_scratch_pattern(object, anonymous, scratch, line);
    }
    let def = scratch_names
        .get(&scratch.name)
        .ok_or_else(|| parse_error(line, "unknown scratch"))?;
    let value = match def.kind {
        ScratchKind::Marker => {
            if scratch.value.is_some() {
                return Err(parse_error(line, "marker scratch cannot have a value"));
            }
            None
        }
        ScratchKind::Bool => {
            if scratch.value.is_some() {
                return Err(parse_error(
                    line,
                    "bool scratch uses presence syntax; write `flag` or `no flag`",
                ));
            }
            Some(ScratchValueTemplate::Literal(1))
        }
        ScratchKind::Int => scratch
            .value
            .as_deref()
            .map(|value| {
                value
                    .parse::<i64>()
                    .map(ScratchValueTemplate::Literal)
                    .map_err(|_| parse_error(line, "expected integer scratch value"))
            })
            .transpose()?,
        ScratchKind::Enum => scratch
            .value
            .as_deref()
            .map(|value| parse_enum_scratch_value(value, def, line))
            .transpose()?,
    };
    let match_value = if value.is_some() {
        ScratchValueMatch::Exact
    } else {
        ScratchValueMatch::Any
    };
    Ok(ScratchPatternTemplate {
        object,
        scratch: def.id,
        value,
        match_value,
        is_marker: matches!(def.kind, ScratchKind::Marker | ScratchKind::Bool),
    })
}

fn parsed_anonymous_scratch_pattern(
    object: ObjectId,
    anonymous: &AnonymousScratch,
    scratch: &ParsedScratch,
    line: &str,
) -> Result<ScratchPatternTemplate, DiagnosticReport> {
    let value = scratch
        .value
        .as_deref()
        .ok_or_else(|| parse_error(line, "anonymous scratch must specify a value"))?;
    let (scratch_id, value, match_value) = match anonymous {
        AnonymousScratch::Movement if value == "directions" => {
            (ANONYMOUS_MOVEMENT_SCRATCH, None, ScratchValueMatch::Any)
        }
        AnonymousScratch::Movement => (
            ANONYMOUS_MOVEMENT_SCRATCH,
            Some(parse_anonymous_movement_value(value, line)?),
            ScratchValueMatch::Exact,
        ),
        AnonymousScratch::Bool => (
            ANONYMOUS_BOOL_SCRATCH,
            Some(ScratchValueTemplate::Literal(match value {
                "false" => 0,
                "true" => 1,
                _ => return Err(parse_error(line, "expected boolean scratch value")),
            })),
            ScratchValueMatch::Exact,
        ),
        AnonymousScratch::Int => (
            ANONYMOUS_INT_SCRATCH,
            Some(ScratchValueTemplate::Literal(
                value
                    .parse::<i64>()
                    .map_err(|_| parse_error(line, "expected integer scratch value"))?,
            )),
            ScratchValueMatch::Exact,
        ),
    };
    Ok(ScratchPatternTemplate {
        object,
        scratch: scratch_id,
        value,
        match_value,
        is_marker: false,
    })
}

fn parse_anonymous_movement_value(
    value: &str,
    line: &str,
) -> Result<ScratchValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(ScratchValueTemplate::Relative(relative));
    }
    puzzle_authoring::movement_scratch_index(value, puzzle_authoring::MOVEMENT_DIRECTIONS_2D)
        .map(|index| ScratchValueTemplate::Literal(i64::from(index)))
        .ok_or_else(|| parse_error(line, "unknown movement scratch value"))
}

fn movement_scratch_set_values(value: &str) -> Option<&'static [&'static str]> {
    puzzle_authoring::movement_scratch_set_values(value, 2)
}

fn parse_enum_scratch_value(
    value: &str,
    def: &ScratchDef,
    line: &str,
) -> Result<ScratchValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(ScratchValueTemplate::Relative(relative));
    }
    def.values
        .iter()
        .position(|candidate| candidate == value)
        .map(|index| ScratchValueTemplate::Literal(index as i64))
        .ok_or_else(|| parse_error(line, "unknown enum scratch value"))
}

fn parse_relative_direction_value(value: &str) -> Option<RelativeDirection> {
    match value {
        ">" => Some(RelativeDirection::Forward),
        "<" => Some(RelativeDirection::Backward),
        "^" => Some(RelativeDirection::Left),
        "v" => Some(RelativeDirection::Right),
        _ => None,
    }
}

fn reject_duplicate_scratch_patterns(
    scratch: &[ScratchPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(ObjectId, ScratchId)>::new();
    for attr in scratch {
        let key = (attr.object, attr.scratch);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same scratch twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn reject_duplicate_object_set_scratch_patterns(
    scratch: &[ObjectSetScratchPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(u16, ScratchId)>::new();
    for attr in scratch {
        let key = (attr.binding, attr.scratch);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same scratch twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn scratch_to_set(
    after: &[ScratchPatternTemplate],
    before: &[ScratchPatternTemplate],
    line: &str,
) -> Result<Vec<ScratchPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_marker && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS scratch must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn scratch_to_set_object_set(
    after: &[ObjectSetScratchPatternTemplate],
    before: &[ObjectSetScratchPatternTemplate],
    line: &str,
) -> Result<Vec<ObjectSetScratchPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_marker && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS scratch must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn scratch_to_remove(
    before: &[ScratchPatternTemplate],
    after: &[ScratchPatternTemplate],
) -> Vec<ScratchPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn scratch_to_remove_object_set(
    before: &[ObjectSetScratchPatternTemplate],
    after: &[ObjectSetScratchPatternTemplate],
) -> Vec<ObjectSetScratchPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn implicit_layer_forbids(
    before_objects: &[ObjectId],
    after_objects: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
    occupancy_objects: &[ObjectId],
) -> Vec<ObjectId> {
    let mut forbids = Vec::new();
    for after_object in after_objects {
        if before_objects.contains(after_object) {
            continue;
        }
        let Some(after_layer) = object_layers.get(after_object) else {
            continue;
        };
        forbids.extend(occupancy_objects.iter().filter_map(|object| {
            let object_layer = object_layers.get(object)?;
            (object_layer == after_layer
                && !before_objects.contains(object)
                && !after_objects.contains(object))
            .then_some(*object)
        }));
    }
    dedup_objects(&mut forbids);
    forbids
}

fn dedup_objects(objects: &mut Vec<ObjectId>) {
    let mut deduped = Vec::with_capacity(objects.len());
    for object in objects.drain(..) {
        if !deduped.contains(&object) {
            deduped.push(object);
        }
    }
    *objects = deduped;
}

#[derive(Clone, Debug)]
struct SelectorOccurrence {
    token: String,
    alternatives: Vec<ObjectId>,
    occurrence_label: Option<String>,
    tag_captures: HashMap<ObjectId, Vec<TagCapture>>,
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

fn block_cell_object_occurrences(
    cell: &BlockCell,
    assignment: &[SelectorAssignmentValue],
    before_occurrences: &[SelectorOccurrence],
    before_by_token: &HashMap<String, Vec<usize>>,
    token_counts: &mut HashMap<String, usize>,
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
                let before_occurrences =
                    before_by_token
                        .get(&transform.source_token)
                        .ok_or_else(|| {
                            parse_error_at_source_line_number(
                                line,
                                source_line_number,
                                "mapped selector source must appear in before",
                            )
                        })?;
                let before_index = before_occurrences.get(ordinal).ok_or_else(|| {
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
                return transform
                    .mapped_objects
                    .get(&source_object)
                    .copied()
                    .map(|object| ResolvedObjectOccurrence {
                        token: selector.token.clone(),
                        matched: ResolvedObjectMatch::Object(object),
                        key: None,
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
                        Some(target)
                    })
                    .collect::<Vec<_>>();
                let target = if selector.occurrence_label.is_some() {
                    match candidates.as_slice() {
                        [target] => *target,
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
                return Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(target),
                    key: None,
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
            objects: after.matched.possible_objects(),
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

fn direction_by_name(
    name: &str,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Option<Direction> {
    let input = input_names.get(name)?;
    directions
        .iter()
        .copied()
        .find(|direction| direction.input == *input)
}

fn resolve_write(
    write: &WriteOpTemplate,
    direction: Direction,
    dir_any: bool,
    line: &str,
) -> Result<WriteOp, DiagnosticReport> {
    match write {
        WriteOpTemplate::Add {
            component,
            offset,
            object,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::Add {
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
            Ok(WriteOp::AddObjectSet {
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
            Ok(WriteOp::Remove {
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
            Ok(WriteOp::RemoveObjectSet {
                component: *component,
                offset,
                binding: *binding,
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
            Ok(WriteOp::Move {
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
            Ok(WriteOp::MoveObjectSet {
                component: *component,
                from_offset,
                to_offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::SetScratch {
            component,
            offset,
            object,
            scratch,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::SetScratch {
                component: *component,
                offset,
                object: *object,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::SetObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::SetObjectSetScratch {
                component: *component,
                offset,
                binding: *binding,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::RemoveScratch {
                component: *component,
                offset,
                object: *object,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
        WriteOpTemplate::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::RemoveObjectSetScratch {
                component: *component,
                offset,
                binding: *binding,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
    }
}

fn resolve_scratch_patterns(
    patterns: Vec<ScratchPatternTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<ScratchPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(ScratchPattern {
                object: pattern.object,
                scratch: pattern.scratch,
                value: resolve_scratch_value(
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

fn resolve_object_set_scratch_patterns(
    patterns: Vec<ObjectSetScratchPatternTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<ObjectSetScratchPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(ObjectSetScratchPattern {
                binding: pattern.binding,
                scratch: pattern.scratch,
                value: resolve_scratch_value(
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

fn resolve_scratch_value(
    value: Option<&ScratchValueTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Option<i64>, DiagnosticReport> {
    match value {
        Some(ScratchValueTemplate::Literal(value)) => Ok(Some(*value)),
        Some(ScratchValueTemplate::Relative(relative)) => {
            let direction =
                resolve_relative_direction(*relative, direction, direction_expanded, line)?;
            Ok(Some(direction_value(direction)?))
        }
        None => Ok(None),
    }
}

fn resolve_relative_direction(
    relative: RelativeDirection,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Direction, DiagnosticReport> {
    if !direction_expanded {
        return Err(parse_error(
            line,
            "relative direction scratch value requires an oriented rule",
        ));
    }
    let (dx, dy) = match relative {
        RelativeDirection::Forward => (direction.dx, direction.dy),
        RelativeDirection::Backward => (-direction.dx, -direction.dy),
        RelativeDirection::Left => (direction.dy, -direction.dx),
        RelativeDirection::Right => (-direction.dy, direction.dx),
    };
    Ok(Direction {
        input: InputId(0),
        dx,
        dy,
    })
}

fn direction_value(direction: Direction) -> Result<i64, DiagnosticReport> {
    match (direction.dx, direction.dy) {
        (0, -1) => Ok(0),
        (0, 1) => Ok(1),
        (-1, 0) => Ok(2),
        (1, 0) => Ok(3),
        _ => Err(DiagnosticReport::error(
            "unsupported direction scratch".to_string(),
        )),
    }
}

fn direction_tag_name(direction: Direction, line: &str) -> Result<&'static str, DiagnosticReport> {
    match (direction.dx, direction.dy) {
        (0, -1) => Ok("up"),
        (0, 1) => Ok("down"),
        (-1, 0) => Ok("left"),
        (1, 0) => Ok("right"),
        _ => Err(parse_error(
            line,
            "relative direction selector only supports cardinal directions",
        )),
    }
}

fn resolve_offset(
    offset: OffsetTemplate,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Offset, DiagnosticReport> {
    let (base_dx, base_dy) = resolve_oriented_xy(
        offset.oriented_x,
        offset.oriented_y,
        direction,
        direction_expanded,
        line,
    )?;
    if offset.gap_terms.is_empty() {
        return Ok(Offset::Fixed {
            dx: base_dx,
            dy: base_dy,
        });
    }

    let (step_dx, step_dy) = resolve_oriented_xy(1, 0, direction, direction_expanded, line)?;
    Ok(Offset::Variable {
        base_dx,
        base_dy,
        gap_terms: offset
            .gap_terms
            .iter()
            .copied()
            .map(|gap_index| GapTerm {
                gap_index,
                dx: step_dx,
                dy: step_dy,
            })
            .collect(),
    })
}

fn resolve_oriented_xy(
    x: i16,
    y: i16,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<(i16, i16), DiagnosticReport> {
    if !direction_expanded {
        return Ok((x, y));
    }

    Ok(match (direction.dx, direction.dy) {
        (1, 0) => (x, y),
        (-1, 0) => (-x, -y),
        (0, -1) => (y, -x),
        (0, 1) => (-y, x),
        _ => return Err(parse_error(line, "unsupported direction")),
    })
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

fn parse_global_value(token: &str, line: &str) -> Result<i64, DiagnosticReport> {
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

fn parse_error(line: &str, message: &str) -> DiagnosticReport {
    DiagnosticReport::error_at_line(message, line)
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
