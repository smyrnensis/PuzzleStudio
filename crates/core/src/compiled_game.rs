use crate::ids::{ConditionId, InputId, LayerId, MarkId, ObjectId, RuleId, VariableId};
pub use puzzle_kernel::{
    ComparisonOp, LocalFrame, LocalFrameExtent, MarkKind, MarkValueMatch, RuleApplication,
    VariableUpdateOp,
};
use puzzle_kernel::{ProgramStep, SpatialGapTerm, SpatialOffset, SpatialVector};
pub type GridConditionValueKind<const D: usize> =
    puzzle_kernel::ConditionValueKind<ObjectId, GridPattern<D>, InputId>;
pub type GridConditionDef<const D: usize> =
    puzzle_kernel::RuleConditionDef<ConditionId, GridConditionValueKind<D>>;
pub type GridGuard<const D: usize> =
    puzzle_kernel::RuleGuard<VariableId, ConditionId, GridConditionValueKind<D>, InputId>;
pub type GridMatchCell<const D: usize> =
    puzzle_kernel::RuleMatchCell<GridOffset<D>, ObjectId, LayerId, MarkId>;
pub type GridPatternComponent<const D: usize> =
    puzzle_kernel::RulePatternComponent<GridMatchCell<D>>;
pub type GridPattern<const D: usize> = puzzle_kernel::RulePattern<GridPatternComponent<D>>;
pub type GridWriteOp<const D: usize> = puzzle_kernel::RuleWriteOp<GridOffset<D>, ObjectId, MarkId>;
pub type GridRuleCondition<const D: usize> =
    puzzle_kernel::ProgramCondition<GridPattern<D>, GridGuard<D>>;
pub type GridRule<const D: usize> =
    puzzle_kernel::RuleModel<RuleId, GridGuard<D>, GridPattern<D>, GridWriteOp<D>, Effect>;
pub type GridRuleStep<const D: usize> =
    puzzle_kernel::ProgramStep<GridRule<D>, GridRuleCondition<D>, LocalFrame<ObjectId>>;
pub type GridExecutableProgram<const D: usize> =
    puzzle_kernel::ExecutableProgram<GridRule<D>, GridRuleCondition<D>, LocalFrame<ObjectId>>;
pub type GridCompiledGame<const D: usize> = puzzle_kernel::CompiledGameModel<
    GridConditionDef<D>,
    GridRule<D>,
    GridRuleCondition<D>,
    LocalFrame<ObjectId>,
>;

pub type ConditionDef = GridConditionDef<2>;
pub type Guard = GridGuard<2>;
pub type MarkPattern = puzzle_kernel::RuleMarkPattern<ObjectId, MarkId>;
pub type ObjectSetMatcher = puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>;
pub type ObjectSetMarkPattern = puzzle_kernel::ObjectSetMarkPattern<MarkId>;
pub type PatternComponent = GridPatternComponent<2>;
pub type MatchCell = GridMatchCell<2>;
pub type Rule = GridRule<2>;
pub type RuleStep = GridRuleStep<2>;
pub type ExecutableProgram = GridExecutableProgram<2>;
pub type WriteOp = GridWriteOp<2>;
pub type CompiledGame = GridCompiledGame<2>;

pub type ObjectDef = puzzle_kernel::ObjectDef;
pub type MarkDef = puzzle_kernel::MarkDef;
pub type RuleCondition = GridRuleCondition<2>;
pub type Effect = puzzle_kernel::RuleEffect;

pub type ConditionValueKind = GridConditionValueKind<2>;
pub type Pattern = GridPattern<2>;
pub type Pattern2 = Pattern;

pub type Offset = puzzle_kernel::SpatialOffset<2>;
pub type GapTerm = puzzle_kernel::SpatialGapTerm<2>;
pub type GridOffset<const D: usize> = puzzle_kernel::SpatialOffset<D>;
pub type GridGapTerm<const D: usize> = puzzle_kernel::SpatialGapTerm<D>;

pub fn try_project_grid_compiled_game<const FROM: usize, const TO: usize, Error>(
    game: &GridCompiledGame<FROM>,
    mut project_vector: impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridCompiledGame<TO>, Error> {
    let condition_defs = game
        .condition_defs()
        .iter()
        .map(|condition| project_condition_def(condition, &mut project_vector))
        .collect::<Result<_, _>>()?;
    let program = project_steps(game.program(), &mut project_vector)?;
    Ok(GridCompiledGame::new_with_mark_condition_defs_and_program(
        game.layer_count,
        game.objects().to_vec(),
        game.mark().to_vec(),
        condition_defs,
        program,
    ))
}

pub fn try_project_grid_program<const FROM: usize, const TO: usize, Error>(
    program: &[GridRuleStep<FROM>],
    mut project_vector: impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<Vec<GridRuleStep<TO>>, Error> {
    project_steps(program, &mut project_vector)
}

pub fn try_project_grid_condition_value<const FROM: usize, const TO: usize, Error>(
    value: &GridConditionValueKind<FROM>,
    mut project_vector: impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridConditionValueKind<TO>, Error> {
    project_condition_value(value, &mut project_vector)
}

fn project_condition_def<const FROM: usize, const TO: usize, Error>(
    value: &GridConditionDef<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridConditionDef<TO>, Error> {
    Ok(GridConditionDef {
        id: value.id,
        kind: project_condition_value(&value.kind, project_vector)?,
    })
}

fn project_condition_value<const FROM: usize, const TO: usize, Error>(
    value: &GridConditionValueKind<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridConditionValueKind<TO>, Error> {
    Ok(match value {
        GridConditionValueKind::CountObjects(objects) => {
            GridConditionValueKind::CountObjects(objects.clone())
        }
        GridConditionValueKind::ExistsObjects(objects) => {
            GridConditionValueKind::ExistsObjects(objects.clone())
        }
        GridConditionValueKind::NoneObjects(objects) => {
            GridConditionValueKind::NoneObjects(objects.clone())
        }
        GridConditionValueKind::CountMatches(patterns) => GridConditionValueKind::CountMatches(
            patterns
                .iter()
                .map(|pattern| project_pattern(pattern, project_vector))
                .collect::<Result<_, _>>()?,
        ),
        GridConditionValueKind::ExistsMatches(patterns) => GridConditionValueKind::ExistsMatches(
            patterns
                .iter()
                .map(|pattern| project_pattern(pattern, project_vector))
                .collect::<Result<_, _>>()?,
        ),
        GridConditionValueKind::NoneMatches(patterns) => GridConditionValueKind::NoneMatches(
            patterns
                .iter()
                .map(|pattern| project_pattern(pattern, project_vector))
                .collect::<Result<_, _>>()?,
        ),
        GridConditionValueKind::CountInputMatches(patterns) => {
            GridConditionValueKind::CountInputMatches(project_input_patterns(
                patterns,
                project_vector,
            )?)
        }
        GridConditionValueKind::ExistsInputMatches(patterns) => {
            GridConditionValueKind::ExistsInputMatches(project_input_patterns(
                patterns,
                project_vector,
            )?)
        }
        GridConditionValueKind::NoneInputMatches(patterns) => {
            GridConditionValueKind::NoneInputMatches(project_input_patterns(
                patterns,
                project_vector,
            )?)
        }
    })
}

fn project_input_patterns<const FROM: usize, const TO: usize, Error>(
    values: &[(InputId, GridPattern<FROM>)],
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<Vec<(InputId, GridPattern<TO>)>, Error> {
    values
        .iter()
        .map(|(input, pattern)| Ok((*input, project_pattern(pattern, project_vector)?)))
        .collect()
}

fn project_steps<const FROM: usize, const TO: usize, Error>(
    values: &[GridRuleStep<FROM>],
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<Vec<GridRuleStep<TO>>, Error> {
    values
        .iter()
        .map(|step| project_step(step, project_vector))
        .collect()
}

fn project_step<const FROM: usize, const TO: usize, Error>(
    value: &GridRuleStep<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridRuleStep<TO>, Error> {
    Ok(match value {
        ProgramStep::Rule(rule) => ProgramStep::Rule(project_rule(rule, project_vector)?),
        ProgramStep::ConditionalBlock { condition, steps } => ProgramStep::ConditionalBlock {
            condition: project_rule_condition(condition, project_vector)?,
            steps: project_steps(steps, project_vector)?,
        },
        ProgramStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => ProgramStep::ConditionalBranch {
            condition: project_rule_condition(condition, project_vector)?,
            then_steps: project_steps(then_steps, project_vector)?,
            else_steps: project_steps(else_steps, project_vector)?,
        },
        ProgramStep::Block {
            application,
            stop_condition,
            steps,
        } => ProgramStep::Block {
            application: *application,
            stop_condition: stop_condition
                .as_ref()
                .map(|condition| project_rule_condition(condition, project_vector))
                .transpose()?,
            steps: project_steps(steps, project_vector)?,
        },
        ProgramStep::AfterTriggered { steps, then_steps } => ProgramStep::AfterTriggered {
            steps: project_steps(steps, project_vector)?,
            then_steps: project_steps(then_steps, project_vector)?,
        },
        ProgramStep::LocalFrame { frame, steps } => ProgramStep::LocalFrame {
            frame: frame.clone(),
            steps: project_steps(steps, project_vector)?,
        },
    })
}

fn project_rule<const FROM: usize, const TO: usize, Error>(
    value: &GridRule<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridRule<TO>, Error> {
    Ok(GridRule {
        id: value.id,
        guards: value
            .guards
            .iter()
            .map(|guard| project_guard(guard, project_vector))
            .collect::<Result<_, _>>()?,
        application: value.application,
        pattern: project_pattern(&value.pattern, project_vector)?,
        writes: value
            .writes
            .iter()
            .map(|write| project_write(write, project_vector))
            .collect::<Result<_, _>>()?,
        effects: value.effects.clone(),
    })
}

fn project_rule_condition<const FROM: usize, const TO: usize, Error>(
    value: &GridRuleCondition<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridRuleCondition<TO>, Error> {
    Ok(match value {
        GridRuleCondition::AnyMatches(patterns) => GridRuleCondition::AnyMatches(
            patterns
                .iter()
                .map(|pattern| project_pattern(pattern, project_vector))
                .collect::<Result<_, _>>()?,
        ),
        GridRuleCondition::NoMatches(patterns) => GridRuleCondition::NoMatches(
            patterns
                .iter()
                .map(|pattern| project_pattern(pattern, project_vector))
                .collect::<Result<_, _>>()?,
        ),
        GridRuleCondition::AnyInputMatches(patterns) => {
            GridRuleCondition::AnyInputMatches(project_input_patterns(patterns, project_vector)?)
        }
        GridRuleCondition::NoInputMatches(patterns) => {
            GridRuleCondition::NoInputMatches(project_input_patterns(patterns, project_vector)?)
        }
        GridRuleCondition::GuardBranches(branches) => GridRuleCondition::GuardBranches(
            branches
                .iter()
                .map(|branch| {
                    branch
                        .iter()
                        .map(|guard| project_guard(guard, project_vector))
                        .collect()
                })
                .collect::<Result<_, _>>()?,
        ),
    })
}

fn project_guard<const FROM: usize, const TO: usize, Error>(
    value: &GridGuard<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridGuard<TO>, Error> {
    Ok(match value {
        GridGuard::InputIs(input) => GridGuard::InputIs(*input),
        GridGuard::VariableEquals { variable, value } => GridGuard::VariableEquals {
            variable: *variable,
            value: *value,
        },
        GridGuard::VariableCompare {
            variable,
            op,
            value,
        } => GridGuard::VariableCompare {
            variable: *variable,
            op: *op,
            value: *value,
        },
        GridGuard::ConditionEquals { condition, value } => GridGuard::ConditionEquals {
            condition: *condition,
            value: *value,
        },
        GridGuard::ConditionNonZero(condition) => GridGuard::ConditionNonZero(*condition),
        GridGuard::ConditionCompare {
            condition,
            op,
            value,
        } => GridGuard::ConditionCompare {
            condition: *condition,
            op: *op,
            value: *value,
        },
        GridGuard::InlineConditionValue { kind, value } => GridGuard::InlineConditionValue {
            kind: project_condition_value(kind, project_vector)?,
            value: *value,
        },
        GridGuard::InlineConditionNonZero(kind) => {
            GridGuard::InlineConditionNonZero(project_condition_value(kind, project_vector)?)
        }
        GridGuard::InlineConditionCompare { kind, op, value } => {
            GridGuard::InlineConditionCompare {
                kind: project_condition_value(kind, project_vector)?,
                op: *op,
                value: *value,
            }
        }
    })
}

fn project_pattern<const FROM: usize, const TO: usize, Error>(
    value: &GridPattern<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridPattern<TO>, Error> {
    Ok(GridPattern::from_components(
        value
            .components
            .iter()
            .map(|component| {
                Ok(GridPatternComponent {
                    gap_count: component.gap_count,
                    cells: component
                        .cells
                        .iter()
                        .map(|cell| {
                            Ok(GridMatchCell {
                                offset: project_offset(&cell.offset, project_vector)?,
                                require_null: cell.require_null,
                                require_objects: cell.require_objects.clone(),
                                require_object_sets: cell.require_object_sets.clone(),
                                forbid_objects: cell.forbid_objects.clone(),
                                require_mark: cell.require_mark.clone(),
                                require_object_set_mark: cell.require_object_set_mark.clone(),
                                forbid_mark: cell.forbid_mark.clone(),
                                forbid_object_set_mark: cell.forbid_object_set_mark.clone(),
                            })
                        })
                        .collect::<Result<_, Error>>()?,
                })
            })
            .collect::<Result<_, Error>>()?,
    ))
}

fn project_offset<const FROM: usize, const TO: usize, Error>(
    value: &SpatialOffset<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<SpatialOffset<TO>, Error> {
    Ok(match value {
        SpatialOffset::Fixed { delta } => SpatialOffset::Fixed {
            delta: project_vector(*delta)?,
        },
        SpatialOffset::Variable { base, gap_terms } => SpatialOffset::Variable {
            base: project_vector(*base)?,
            gap_terms: gap_terms
                .iter()
                .map(|term| {
                    Ok(SpatialGapTerm {
                        gap_index: term.gap_index,
                        delta: project_vector(term.delta)?,
                    })
                })
                .collect::<Result<_, Error>>()?,
        },
    })
}

fn project_write<const FROM: usize, const TO: usize, Error>(
    value: &GridWriteOp<FROM>,
    project_vector: &mut impl FnMut(SpatialVector<FROM>) -> Result<SpatialVector<TO>, Error>,
) -> Result<GridWriteOp<TO>, Error> {
    Ok(match value {
        GridWriteOp::Add {
            component,
            offset,
            object,
        } => GridWriteOp::Add {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            object: *object,
        },
        GridWriteOp::AddObjectSet {
            component,
            offset,
            binding,
        } => GridWriteOp::AddObjectSet {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            binding: *binding,
        },
        GridWriteOp::Remove {
            component,
            offset,
            object,
        } => GridWriteOp::Remove {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            object: *object,
        },
        GridWriteOp::RemoveObjectSet {
            component,
            offset,
            binding,
        } => GridWriteOp::RemoveObjectSet {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            binding: *binding,
        },
        GridWriteOp::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => GridWriteOp::Move {
            component: *component,
            from_offset: project_offset(from_offset, project_vector)?,
            to_offset: project_offset(to_offset, project_vector)?,
            object: *object,
        },
        GridWriteOp::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
        } => GridWriteOp::MoveObjectSet {
            component: *component,
            from_offset: project_offset(from_offset, project_vector)?,
            to_offset: project_offset(to_offset, project_vector)?,
            binding: *binding,
        },
        GridWriteOp::Replace {
            component,
            offset,
            remove,
            add,
        } => GridWriteOp::Replace {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            remove: *remove,
            add: *add,
        },
        GridWriteOp::SetMark {
            component,
            offset,
            object,
            mark,
            value,
        } => GridWriteOp::SetMark {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            object: *object,
            mark: *mark,
            value: *value,
        },
        GridWriteOp::SetObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
        } => GridWriteOp::SetObjectSetMark {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            binding: *binding,
            mark: *mark,
            value: *value,
        },
        GridWriteOp::RemoveMark {
            component,
            offset,
            object,
            mark,
            value,
            match_value,
        } => GridWriteOp::RemoveMark {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            object: *object,
            mark: *mark,
            value: *value,
            match_value: *match_value,
        },
        GridWriteOp::RemoveObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
            match_value,
        } => GridWriteOp::RemoveObjectSetMark {
            component: *component,
            offset: project_offset(offset, project_vector)?,
            binding: *binding,
            mark: *mark,
            value: *value,
            match_value: *match_value,
        },
    })
}
