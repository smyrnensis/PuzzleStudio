use std::collections::HashMap;

use puzzle_core::{
    ConditionDef, ConditionValueKind, GridMatchCell, GridPattern, GridRule, GridRuleCondition,
    GridRuleStep, GridWriteOp, Guard, Offset, Pattern, RuleCondition, RuleStep, WriteOp,
};
use puzzle_grid3d::{
    CompiledGame3, ConditionDef3, ConditionValueKind3, Coord3, Direction3, Guard3, InputDef3,
    Level3, LevelBundle3, LevelCell3, LevelEntry3, Offset3, PatternComponent3, Size3,
    WinCondition3,
};
use puzzle_kernel::{ProgramStep, SpatialGapTerm, SpatialOffset, SpatialVector};

use crate::{
    ArrowKey, Catalog, ComparisonOp, DiagnosticReport, GoalClause, GoalExpr, GoalValue, LoadedGame,
    ModelDimension, ParsedPuzzle3, QueryExpr, QueryExpr3, SolverDeadendOf, SolverStrategy,
    SolverStrategy3, SolverStrategyTerm3, model_syntax::PuzzleModelSyntax,
};

pub(crate) fn materialize_puzzle3(
    shared: LoadedGame,
    models: &[PuzzleModelSyntax],
    catalogs: &[Catalog],
) -> Result<ParsedPuzzle3, DiagnosticReport> {
    let [model] = models else {
        return Err(DiagnosticReport::error(
            "3D materialization requires exactly one puzzle".to_string(),
        ));
    };
    let [catalog] = catalogs else {
        return Err(DiagnosticReport::error(
            "3D materialization requires exactly one canonical catalog".to_string(),
        ));
    };
    if model.dimension != ModelDimension::Three {
        return Err(DiagnosticReport::error(
            "3D materialization requires `dimension = 3`".to_string(),
        ));
    }
    if !shared.variables.is_empty() {
        return Err(DiagnosticReport::error(
            "canonical 3D runtime variable state materialization is not connected".to_string(),
        ));
    }
    if shared
        .rule_effects
        .values()
        .any(|effects| !effects.is_empty())
    {
        return Err(DiagnosticReport::error(
            "canonical 3D ordered rule-effect materialization is not connected".to_string(),
        ));
    }
    if shared.level_start_program.is_some()
        || shared.level_clear_program.is_some()
        || shared.last_level_clear_program.is_some()
    {
        return Err(DiagnosticReport::error(
            "canonical 3D lifecycle materialization is not connected".to_string(),
        ));
    }
    if !shared.visuals.aliases.is_empty() || !shared.visuals.sprites.is_empty() {
        return Err(DiagnosticReport::error(
            "canonical 3D sprite materialization is not connected".to_string(),
        ));
    }
    let game = materialize_game(&shared.game);
    let levels = materialize_levels(model, catalog, &shared, &game)?;
    let inputs = materialize_inputs(&shared);
    let win_condition = shared.goal.as_ref().map(materialize_goal).transpose()?;

    Ok(ParsedPuzzle3 {
        game: game.clone(),
        inputs,
        object_labels: shared.object_labels,
        viewport_focus_objects: Vec::new(),
        animation: shared.animation,
        render: shared.render,
        local_frame: None,
        rule_camera_effects: Vec::new(),
        level_bundle: Some(LevelBundle3::checked_new(game, levels).map_err(|error| {
            DiagnosticReport::error(format!("invalid canonical 3D level bundle: {error:?}"))
        })?),
        level_packs: model
            .body
            .levels
            .levels
            .iter()
            .map(|level| level.pack.clone())
            .collect(),
        win_condition,
        solver_strategy: materialize_solver_strategy(&shared.solver_strategy),
        lifecycle: Default::default(),
        on_level_start_camera_effects: Vec::new(),
        sprite_set: None,
        visual_order: shared.visuals.order,
    })
}

fn materialize_solver_strategy(strategy: &SolverStrategy) -> SolverStrategy3 {
    SolverStrategy3 {
        terms: strategy
            .terms
            .iter()
            .map(|term| SolverStrategyTerm3 {
                direction: term.direction,
                value: materialize_query(&term.value),
                weight: term.weight,
            })
            .collect(),
        deadends: strategy
            .deadends
            .iter()
            .map(|deadend| match deadend {
                SolverDeadendOf::All(values) => {
                    SolverDeadendOf::All(values.iter().map(materialize_query).collect())
                }
                SolverDeadendOf::Any(values) => {
                    SolverDeadendOf::Any(values.iter().map(materialize_query).collect())
                }
            })
            .collect(),
    }
}

fn materialize_query(query: &QueryExpr) -> QueryExpr3 {
    match query {
        QueryExpr::Variable(variable) => QueryExpr3::Variable(*variable),
        QueryExpr::Value(value) => QueryExpr3::Value(materialize_condition_value(value)),
        QueryExpr::Distance { from, to } => QueryExpr3::Distance {
            from: from.clone(),
            to: to.clone(),
        },
        QueryExpr::AllOnDistance { subjects, covers } => QueryExpr3::AllOnDistance {
            subjects: subjects.clone(),
            covers: covers.clone(),
        },
        QueryExpr::Compare { left, op, right } => QueryExpr3::Compare {
            left: Box::new(materialize_query(left)),
            op: *op,
            right: *right,
        },
    }
}

fn materialize_game(game: &puzzle_core::CompiledGame) -> CompiledGame3 {
    CompiledGame3::new_with_mark_condition_defs_and_program(
        game.layer_count,
        game.objects().to_vec(),
        game.mark().to_vec(),
        game.condition_defs()
            .iter()
            .map(materialize_condition_def)
            .collect(),
        game.program().iter().map(materialize_step).collect(),
    )
}

fn materialize_condition_def(definition: &ConditionDef) -> ConditionDef3 {
    ConditionDef3 {
        id: definition.id,
        kind: materialize_condition_value(&definition.kind),
    }
}

fn materialize_step(step: &RuleStep) -> GridRuleStep<3> {
    match step {
        ProgramStep::Rule(rule) => ProgramStep::Rule(materialize_rule(rule)),
        ProgramStep::ConditionalBlock { condition, steps } => ProgramStep::ConditionalBlock {
            condition: materialize_condition(condition),
            steps: steps.iter().map(materialize_step).collect(),
        },
        ProgramStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => ProgramStep::ConditionalBranch {
            condition: materialize_condition(condition),
            then_steps: then_steps.iter().map(materialize_step).collect(),
            else_steps: else_steps.iter().map(materialize_step).collect(),
        },
        ProgramStep::Block {
            application,
            stop_condition,
            steps,
        } => ProgramStep::Block {
            application: *application,
            stop_condition: stop_condition.as_ref().map(materialize_condition),
            steps: steps.iter().map(materialize_step).collect(),
        },
        ProgramStep::AfterTriggered { steps, then_steps } => ProgramStep::AfterTriggered {
            steps: steps.iter().map(materialize_step).collect(),
            then_steps: then_steps.iter().map(materialize_step).collect(),
        },
        ProgramStep::LocalFrame { frame, steps } => ProgramStep::LocalFrame {
            frame: frame.clone(),
            steps: steps.iter().map(materialize_step).collect(),
        },
    }
}

fn materialize_rule(rule: &puzzle_core::Rule) -> GridRule<3> {
    GridRule::<3> {
        id: rule.id,
        guards: rule.guards.iter().map(materialize_guard).collect(),
        application: rule.application,
        pattern: materialize_pattern(&rule.pattern),
        writes: rule.writes.iter().map(materialize_write).collect(),
        effects: rule.effects.clone(),
    }
}

fn materialize_condition(condition: &RuleCondition) -> GridRuleCondition<3> {
    match condition {
        RuleCondition::AnyMatches(patterns) => {
            GridRuleCondition::<3>::AnyMatches(patterns.iter().map(materialize_pattern).collect())
        }
        RuleCondition::NoMatches(patterns) => {
            GridRuleCondition::<3>::NoMatches(patterns.iter().map(materialize_pattern).collect())
        }
        RuleCondition::AnyInputMatches(patterns) => GridRuleCondition::<3>::AnyInputMatches(
            patterns
                .iter()
                .map(|(input, pattern)| (*input, materialize_pattern(pattern)))
                .collect(),
        ),
        RuleCondition::NoInputMatches(patterns) => GridRuleCondition::<3>::NoInputMatches(
            patterns
                .iter()
                .map(|(input, pattern)| (*input, materialize_pattern(pattern)))
                .collect(),
        ),
        RuleCondition::GuardBranches(branches) => GridRuleCondition::<3>::GuardBranches(
            branches
                .iter()
                .map(|branch| branch.iter().map(materialize_guard).collect())
                .collect(),
        ),
    }
}

fn materialize_guard(guard: &Guard) -> Guard3 {
    match guard {
        Guard::InputIs(input) => Guard3::InputIs(*input),
        Guard::VariableEquals { variable, value } => Guard3::VariableEquals {
            variable: *variable,
            value: *value,
        },
        Guard::VariableCompare {
            variable,
            op,
            value,
        } => Guard3::VariableCompare {
            variable: *variable,
            op: *op,
            value: *value,
        },
        Guard::ConditionEquals { condition, value } => Guard3::ConditionEquals {
            condition: *condition,
            value: *value,
        },
        Guard::ConditionNonZero(condition) => Guard3::ConditionNonZero(*condition),
        Guard::ConditionCompare {
            condition,
            op,
            value,
        } => Guard3::ConditionCompare {
            condition: *condition,
            op: *op,
            value: *value,
        },
        Guard::InlineConditionValue { kind, value } => Guard3::InlineConditionValue {
            kind: materialize_condition_value(kind),
            value: *value,
        },
        Guard::InlineConditionNonZero(kind) => {
            Guard3::InlineConditionNonZero(materialize_condition_value(kind))
        }
        Guard::InlineConditionCompare { kind, op, value } => Guard3::InlineConditionCompare {
            kind: materialize_condition_value(kind),
            op: *op,
            value: *value,
        },
    }
}

fn materialize_condition_value(value: &ConditionValueKind) -> ConditionValueKind3 {
    match value {
        ConditionValueKind::CountObjects(objects) => {
            ConditionValueKind3::CountObjects(objects.clone())
        }
        ConditionValueKind::ExistsObjects(objects) => {
            ConditionValueKind3::ExistsObjects(objects.clone())
        }
        ConditionValueKind::NoneObjects(objects) => {
            ConditionValueKind3::NoneObjects(objects.clone())
        }
        ConditionValueKind::CountMatches(patterns) => {
            ConditionValueKind3::CountMatches(patterns.iter().map(materialize_pattern).collect())
        }
        ConditionValueKind::ExistsMatches(patterns) => {
            ConditionValueKind3::ExistsMatches(patterns.iter().map(materialize_pattern).collect())
        }
        ConditionValueKind::NoneMatches(patterns) => {
            ConditionValueKind3::NoneMatches(patterns.iter().map(materialize_pattern).collect())
        }
        ConditionValueKind::CountInputMatches(patterns) => ConditionValueKind3::CountInputMatches(
            patterns
                .iter()
                .map(|(input, pattern)| (*input, materialize_pattern(pattern)))
                .collect(),
        ),
        ConditionValueKind::ExistsInputMatches(patterns) => {
            ConditionValueKind3::ExistsInputMatches(
                patterns
                    .iter()
                    .map(|(input, pattern)| (*input, materialize_pattern(pattern)))
                    .collect(),
            )
        }
        ConditionValueKind::NoneInputMatches(patterns) => ConditionValueKind3::NoneInputMatches(
            patterns
                .iter()
                .map(|(input, pattern)| (*input, materialize_pattern(pattern)))
                .collect(),
        ),
    }
}

fn materialize_pattern(pattern: &Pattern) -> GridPattern<3> {
    GridPattern::<3>::from_components(
        pattern
            .components
            .iter()
            .map(|component| PatternComponent3 {
                gap_count: component.gap_count,
                cells: component
                    .cells
                    .iter()
                    .map(|cell| GridMatchCell::<3> {
                        offset: materialize_offset(&cell.offset),
                        require_null: cell.require_null,
                        require_objects: cell.require_objects.clone(),
                        require_object_sets: cell.require_object_sets.clone(),
                        forbid_objects: cell.forbid_objects.clone(),
                        require_mark: cell.require_mark.clone(),
                        require_object_set_mark: cell.require_object_set_mark.clone(),
                        forbid_mark: cell.forbid_mark.clone(),
                        forbid_object_set_mark: cell.forbid_object_set_mark.clone(),
                    })
                    .collect(),
            })
            .collect(),
    )
}

fn materialize_offset(offset: &Offset) -> Offset3 {
    fn vector(value: SpatialVector<2>) -> SpatialVector<3> {
        let [x, authored_down] = value.axes();
        SpatialVector::new([x, 0, -authored_down])
    }
    match offset {
        SpatialOffset::Fixed { delta } => SpatialOffset::Fixed {
            delta: vector(*delta),
        },
        SpatialOffset::Variable { base, gap_terms } => SpatialOffset::Variable {
            base: vector(*base),
            gap_terms: gap_terms
                .iter()
                .map(|term| SpatialGapTerm {
                    gap_index: term.gap_index,
                    delta: vector(term.delta),
                })
                .collect(),
        },
    }
}

fn materialize_write(write: &WriteOp) -> GridWriteOp<3> {
    match write {
        WriteOp::Add {
            component,
            offset,
            object,
        } => GridWriteOp::<3>::Add {
            component: *component,
            offset: materialize_offset(offset),
            object: *object,
        },
        WriteOp::AddObjectSet {
            component,
            offset,
            binding,
        } => GridWriteOp::<3>::AddObjectSet {
            component: *component,
            offset: materialize_offset(offset),
            binding: *binding,
        },
        WriteOp::Remove {
            component,
            offset,
            object,
        } => GridWriteOp::<3>::Remove {
            component: *component,
            offset: materialize_offset(offset),
            object: *object,
        },
        WriteOp::RemoveObjectSet {
            component,
            offset,
            binding,
        } => GridWriteOp::<3>::RemoveObjectSet {
            component: *component,
            offset: materialize_offset(offset),
            binding: *binding,
        },
        WriteOp::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => GridWriteOp::<3>::Move {
            component: *component,
            from_offset: materialize_offset(from_offset),
            to_offset: materialize_offset(to_offset),
            object: *object,
        },
        WriteOp::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
        } => GridWriteOp::<3>::MoveObjectSet {
            component: *component,
            from_offset: materialize_offset(from_offset),
            to_offset: materialize_offset(to_offset),
            binding: *binding,
        },
        WriteOp::Replace {
            component,
            offset,
            remove,
            add,
        } => GridWriteOp::<3>::Replace {
            component: *component,
            offset: materialize_offset(offset),
            remove: *remove,
            add: *add,
        },
        WriteOp::SetMark {
            component,
            offset,
            object,
            mark,
            value,
        } => GridWriteOp::<3>::SetMark {
            component: *component,
            offset: materialize_offset(offset),
            object: *object,
            mark: *mark,
            value: *value,
        },
        WriteOp::SetObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
        } => GridWriteOp::<3>::SetObjectSetMark {
            component: *component,
            offset: materialize_offset(offset),
            binding: *binding,
            mark: *mark,
            value: *value,
        },
        WriteOp::RemoveMark {
            component,
            offset,
            object,
            mark,
            value,
            match_value,
        } => GridWriteOp::<3>::RemoveMark {
            component: *component,
            offset: materialize_offset(offset),
            object: *object,
            mark: *mark,
            value: *value,
            match_value: *match_value,
        },
        WriteOp::RemoveObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
            match_value,
        } => GridWriteOp::<3>::RemoveObjectSetMark {
            component: *component,
            offset: materialize_offset(offset),
            binding: *binding,
            mark: *mark,
            value: *value,
            match_value: *match_value,
        },
    }
}

fn materialize_inputs(shared: &LoadedGame) -> Vec<InputDef3> {
    let mut keys = HashMap::<_, Vec<String>>::new();
    for (key, input) in &shared.controls.keys {
        keys.entry(*input)
            .or_default()
            .push(char::from(*key).to_string());
    }
    for (key, input) in &shared.controls.arrows {
        let name = match key {
            ArrowKey::Up => "ArrowUp",
            ArrowKey::Down => "ArrowDown",
            ArrowKey::Left => "ArrowLeft",
            ArrowKey::Right => "ArrowRight",
        };
        keys.entry(*input).or_default().push(name.to_string());
    }
    for (key, input) in &shared.controls.named {
        keys.entry(*input).or_default().push(key.clone());
    }
    let mut inputs = shared
        .input_labels
        .iter()
        .map(|(id, name)| InputDef3 {
            id: *id,
            name: name.clone(),
            direction: direction3(name),
            keys: keys.remove(id).unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    inputs.sort_by_key(|input| input.id.0);
    inputs
}

fn direction3(name: &str) -> Option<Direction3> {
    match name {
        "up" => Some(Direction3::UP),
        "down" => Some(Direction3::DOWN),
        "left" => Some(Direction3::LEFT),
        "right" => Some(Direction3::RIGHT),
        "front" => Some(Direction3::FORWARD),
        "back" => Some(Direction3::BACKWARD),
        _ => None,
    }
}

fn materialize_levels(
    model: &PuzzleModelSyntax,
    catalog: &Catalog,
    shared: &LoadedGame,
    game: &CompiledGame3,
) -> Result<Vec<LevelEntry3>, DiagnosticReport> {
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
    model
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
            let mut cells = Vec::new();
            for (slice_index, slice) in slices.iter().enumerate() {
                for (row_index, row) in slice.iter().enumerate() {
                    for (column, ch) in row.text.chars().enumerate() {
                        if ch == '.' {
                            continue;
                        }
                        let objects = local.get(&ch).cloned().ok_or_else(|| {
                            DiagnosticReport::error_at_source_line_number(
                                format!("unknown level char `{ch}`"),
                                row.text.clone(),
                                row.line,
                            )
                        })?;
                        if objects.is_empty() {
                            continue;
                        }
                        cells.push(LevelCell3::new(
                            Coord3::from_standard_text_position(
                                size,
                                column as u16,
                                row_index as u16,
                                slice_index as u16,
                            ),
                            objects,
                        ));
                    }
                }
            }
            let shared_level = shared.levels.get(index).ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "canonical 3D level `{}` has no matching lowered program",
                    level.name
                ))
            })?;
            let program = shared_level.program.iter().map(materialize_step).collect();
            let entry = LevelEntry3::new(level.name.clone(), Level3::new(size, cells), program);
            entry.level.build_state(game).map_err(|error| {
                DiagnosticReport::error(format!("invalid 3D level `{}`: {error:?}", level.name))
            })?;
            Ok(entry)
        })
        .collect()
}

fn materialize_goal(goal: &crate::GoalCondition) -> Result<WinCondition3, DiagnosticReport> {
    materialize_goal_expr(&goal.expr)
}

fn materialize_goal_expr(expr: &GoalExpr) -> Result<WinCondition3, DiagnosticReport> {
    match expr {
        GoalExpr::All(values) => Ok(WinCondition3::All(
            values
                .iter()
                .map(materialize_goal_expr)
                .collect::<Result<_, _>>()?,
        )),
        GoalExpr::Any(values) => Ok(WinCondition3::Any(
            values
                .iter()
                .map(materialize_goal_expr)
                .collect::<Result<_, _>>()?,
        )),
        GoalExpr::Clause(clause) => materialize_goal_clause(clause),
    }
}

fn materialize_goal_clause(clause: &GoalClause) -> Result<WinCondition3, DiagnosticReport> {
    let GoalValue::InlineConditionValue(value) = &clause.value else {
        return Err(DiagnosticReport::error(
            "3D win materialization requires a spatial object or pattern condition".to_string(),
        ));
    };
    let truth = comparison_accepts_boolean(clause.op, clause.expected)?;
    let positive = match value {
        ConditionValueKind::ExistsObjects(objects) => combine_objects(objects, true),
        ConditionValueKind::NoneObjects(objects) => combine_objects(objects, false),
        ConditionValueKind::ExistsMatches(patterns) => combine_patterns(patterns, true),
        ConditionValueKind::NoneMatches(patterns) => combine_patterns(patterns, false),
        ConditionValueKind::CountObjects(objects) => combine_objects(objects, true),
        ConditionValueKind::CountMatches(patterns) => combine_patterns(patterns, true),
        _ => {
            return Err(DiagnosticReport::error(
                "input-dependent win conditions cannot be materialized as persistent 3D goals"
                    .to_string(),
            ));
        }
    };
    if truth {
        Ok(positive)
    } else {
        Ok(negate_win(positive))
    }
}

fn comparison_accepts_boolean(op: ComparisonOp, expected: i64) -> Result<bool, DiagnosticReport> {
    let zero = compare(0, op, expected);
    let one = compare(1, op, expected);
    match (zero, one) {
        (false, true) => Ok(true),
        (true, false) => Ok(false),
        _ => Err(DiagnosticReport::error(
            "3D win comparison is not a boolean predicate".to_string(),
        )),
    }
}

fn compare(actual: i64, op: ComparisonOp, expected: i64) -> bool {
    match op {
        ComparisonOp::Eq => actual == expected,
        ComparisonOp::NotEq => actual != expected,
        ComparisonOp::Greater => actual > expected,
        ComparisonOp::GreaterEq => actual >= expected,
        ComparisonOp::Less => actual < expected,
        ComparisonOp::LessEq => actual <= expected,
    }
}

fn combine_objects(objects: &[puzzle_core::ObjectId], exists: bool) -> WinCondition3 {
    let values = objects
        .iter()
        .copied()
        .map(|object| {
            if exists {
                WinCondition3::SomeObject(object)
            } else {
                WinCondition3::NoObject(object)
            }
        })
        .collect();
    if exists {
        WinCondition3::Any(values)
    } else {
        WinCondition3::All(values)
    }
}

fn combine_patterns(patterns: &[Pattern], exists: bool) -> WinCondition3 {
    let values = patterns
        .iter()
        .map(|pattern| {
            if exists {
                WinCondition3::SomePattern(materialize_pattern(pattern))
            } else {
                WinCondition3::NoPattern(materialize_pattern(pattern))
            }
        })
        .collect();
    if exists {
        WinCondition3::Any(values)
    } else {
        WinCondition3::All(values)
    }
}

fn negate_win(value: WinCondition3) -> WinCondition3 {
    match value {
        WinCondition3::All(values) => {
            WinCondition3::Any(values.into_iter().map(negate_win).collect())
        }
        WinCondition3::Any(values) => {
            WinCondition3::All(values.into_iter().map(negate_win).collect())
        }
        WinCondition3::SomeObject(object) => WinCondition3::NoObject(object),
        WinCondition3::NoObject(object) => WinCondition3::SomeObject(object),
        WinCondition3::SomePattern(pattern) => WinCondition3::NoPattern(pattern),
        WinCondition3::NoPattern(pattern) => WinCondition3::SomePattern(pattern),
        WinCondition3::AllObjectsCoveredByPattern { .. } => unreachable!(
            "canonical materialization does not construct legacy covered-by-pattern goals"
        ),
    }
}
