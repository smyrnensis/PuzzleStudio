use crate::compiled_game::{
    CompiledGame, ConditionValueKind, Effect, Guard, LocalFrame, MarkValueMatch, MatchCell, Offset,
    Pattern, PatternComponent, Rule, RuleApplication, RuleCondition, RuleStep, WriteOp,
};
use crate::ids::{ConditionId, InputId, MarkId, ObjectId, RuleId};
use crate::patch::{CorePatch, CorePatchOp, Patch, PatchError};
use crate::state::State;
use puzzle_kernel::{
    GridCoord, GridOffset, TransitionOutcome as KernelTransitionOutcome,
    bind_object as bind_object_shared, bound_object as bound_object_shared,
    collect_component_placements as collect_component_placements_shared,
    complete_component_placements as complete_component_placements_shared,
    placement_object_binding as placement_object_binding_shared,
    write_position as write_position_shared,
    write_position_for_components as write_position_for_components_shared,
};
use std::collections::{BTreeMap, BTreeSet};

const UNTIL_STABLE_REPEAT_LIMIT: usize = 200;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub type TransitionResult<T = State> = Result<T, TransitionError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError {
    Patch(PatchError),
    OffsetOutOfBounds,
    RepeatUntilNoProgress,
    InvalidCommand(String),
}

impl From<PatchError> for TransitionError {
    fn from(value: PatchError) -> Self {
        Self::Patch(value)
    }
}

pub type StepTrace = KernelTransitionOutcome<InputId, State, TransitionCommand, RuleId, Patch>;

pub struct ProgramBoundarySnapshot<'a> {
    pub input: InputId,
    pub next_state: &'a State,
    pub cancelled: bool,
    pub commands: &'a [TransitionCommand],
    pub fired_rules: &'a [RuleId],
    pub patches: &'a [Patch],
}

#[derive(Clone, Debug)]
pub struct ProgramSegmentTrace {
    pub trace: StepTrace,
    pub remaining_program: Option<ProgramContinuation>,
}

#[derive(Clone, Debug)]
pub struct ProgramContinuation {
    steps: Vec<ContinuationStep>,
}

#[derive(Clone, Debug)]
enum ContinuationStep {
    RuleStep(RuleStep),
    LocalFrame {
        frame: LocalFrame<ObjectId>,
        continuation: ProgramContinuation,
    },
    AfterTriggered {
        continuation: ProgramContinuation,
        then_steps: Vec<RuleStep>,
        fired_so_far: bool,
    },
    UntilStable {
        stop_condition: Option<RuleCondition>,
        steps: Vec<RuleStep>,
        before: State,
        before_hash: u64,
        seen_states: StateHistory,
        fired_any: bool,
        pass_fired: bool,
        repeat_count: usize,
        remaining_pass: ProgramContinuation,
    },
}

impl ProgramContinuation {
    fn empty() -> Self {
        Self { steps: Vec::new() }
    }

    fn from_step(step: ContinuationStep) -> Self {
        Self { steps: vec![step] }
    }

    fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    fn extend_rule_steps(&mut self, steps: &[RuleStep]) {
        self.steps
            .extend(steps.iter().cloned().map(ContinuationStep::RuleStep));
    }

    fn extend_continuation_steps(&mut self, steps: &[ContinuationStep]) {
        self.steps.extend_from_slice(steps);
    }
}

pub type TransitionOutcome = StepTrace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionCommand {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
}

pub fn transition_state(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    run_program_transition(game, game.program(), state, input, false)
        .map(|result| result.next_state)
}

pub fn transition_solver_state(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    transition_solver_outcome(game, state, input).map(|result| result.next_state)
}

pub fn transition_solver_outcome(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<TransitionOutcome> {
    run_program_transition(game, game.program(), state, input, true)
}

pub fn transition_outcome(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<TransitionOutcome> {
    run_program_transition(game, game.program(), state, input, true)
}

pub fn transition_trace(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<StepTrace> {
    run_program_transition(game, game.program(), state, input, true)
}

pub fn transition_program(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    run_program_transition(game, program, state, input, false).map(|result| result.next_state)
}

pub fn transition_program_outcome(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<TransitionOutcome> {
    run_program_transition(game, program, state, input, true)
}

pub fn transition_program_trace(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<StepTrace> {
    run_program_transition(game, program, state, input, true)
}

pub fn transition_program_segment_trace<F>(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
    mut should_stop: F,
) -> TransitionResult<ProgramSegmentTrace>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    run_program_transition_segment(game, program, state, input, &mut should_stop)
}

pub fn transition_program_continuation_segment_trace<F>(
    game: &CompiledGame,
    continuation: &ProgramContinuation,
    state: &State,
    input: InputId,
    mut should_stop: F,
) -> TransitionResult<ProgramSegmentTrace>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    run_program_continuation_segment(game, continuation, state, input, &mut should_stop)
}

fn run_program_transition(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
    collect_trace: bool,
) -> TransitionResult<StepTrace> {
    let mut original = state.clone();
    original.clear_mark();
    let mut current = original.clone();
    let mut fired_rules = Vec::new();
    let mut patches = Vec::new();
    let mut commands = Vec::new();
    let context = TransitionContext {
        game,
        input,
        local_frame: None,
    };

    for step in program {
        let outcome = apply_step(
            game,
            step,
            &context,
            &mut current,
            &mut fired_rules,
            &mut patches,
            &mut commands,
            collect_trace,
        )?;
        if outcome.cancelled {
            return Ok(StepTrace {
                input,
                next_state: original,
                cancelled: true,
                commands: Vec::new(),
                fired_rules,
                patches,
            });
        }
    }

    current.clear_mark();

    Ok(StepTrace {
        input,
        next_state: current,
        cancelled: false,
        commands,
        fired_rules,
        patches,
    })
}

fn run_program_transition_segment<F>(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
    should_stop: &mut F,
) -> TransitionResult<ProgramSegmentTrace>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let mut original = state.clone();
    original.clear_mark();
    let mut current = original.clone();
    let mut fired_rules = Vec::new();
    let mut patches = Vec::new();
    let mut commands = Vec::new();
    let context = TransitionContext {
        game,
        input,
        local_frame: None,
    };

    for (index, step) in program.iter().enumerate() {
        let outcome = apply_step_segment(
            game,
            step,
            &context,
            &mut current,
            &mut fired_rules,
            &mut patches,
            &mut commands,
            true,
            should_stop,
        )?;
        if let Some(mut remaining_program) = outcome.remaining_program {
            remaining_program.extend_rule_steps(&program[index + 1..]);
            current.clear_mark();
            return Ok(ProgramSegmentTrace {
                trace: StepTrace {
                    input,
                    next_state: current,
                    cancelled: false,
                    commands,
                    fired_rules,
                    patches,
                },
                remaining_program: Some(remaining_program),
            });
        }
        if outcome.cancelled {
            return Ok(ProgramSegmentTrace {
                trace: StepTrace {
                    input,
                    next_state: original,
                    cancelled: true,
                    commands: Vec::new(),
                    fired_rules,
                    patches,
                },
                remaining_program: None,
            });
        }
    }

    current.clear_mark();

    Ok(ProgramSegmentTrace {
        trace: StepTrace {
            input,
            next_state: current,
            cancelled: false,
            commands,
            fired_rules,
            patches,
        },
        remaining_program: None,
    })
}

fn run_program_continuation_segment<F>(
    game: &CompiledGame,
    continuation: &ProgramContinuation,
    state: &State,
    input: InputId,
    should_stop: &mut F,
) -> TransitionResult<ProgramSegmentTrace>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let mut original = state.clone();
    original.clear_mark();
    let mut current = original.clone();
    let mut fired_rules = Vec::new();
    let mut patches = Vec::new();
    let mut commands = Vec::new();
    let context = TransitionContext {
        game,
        input,
        local_frame: None,
    };

    let outcome = apply_continuation_segment(
        game,
        continuation,
        &context,
        &mut current,
        &mut fired_rules,
        &mut patches,
        &mut commands,
        true,
        should_stop,
    )?;
    if let Some(remaining_program) = outcome.remaining_program {
        current.clear_mark();
        return Ok(ProgramSegmentTrace {
            trace: StepTrace {
                input,
                next_state: current,
                cancelled: false,
                commands,
                fired_rules,
                patches,
            },
            remaining_program: Some(remaining_program),
        });
    }
    if outcome.cancelled {
        return Ok(ProgramSegmentTrace {
            trace: StepTrace {
                input,
                next_state: original,
                cancelled: true,
                commands: Vec::new(),
                fired_rules,
                patches,
            },
            remaining_program: None,
        });
    }

    current.clear_mark();

    Ok(ProgramSegmentTrace {
        trace: StepTrace {
            input,
            next_state: current,
            cancelled: false,
            commands,
            fired_rules,
            patches,
        },
        remaining_program: None,
    })
}

fn apply_step(
    game: &CompiledGame,
    step: &RuleStep,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    match step {
        RuleStep::Rule(rule) => apply_rule_step(
            game,
            rule,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleStep::ConditionalBlock { condition, steps } => {
            if condition_accepts(game, condition, context, current) {
                apply_block_once(
                    game,
                    steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                )
            } else {
                Ok(ApplyOutcome::idle())
            }
        }
        RuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            let selected_steps = if condition_accepts(game, condition, context, current) {
                then_steps
            } else {
                else_steps
            };
            apply_block_once(
                game,
                selected_steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            )
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => match application {
            RuleApplication::Once | RuleApplication::OnceAll | RuleApplication::OncePerLevel => {
                apply_block_once(
                    game,
                    steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                )
            }
            RuleApplication::Random => apply_block_random(
                game,
                steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            ),
            RuleApplication::UntilStable => apply_block_until_stable(
                game,
                stop_condition.as_ref(),
                steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            ),
        },
        RuleStep::AfterTriggered { steps, then_steps } => {
            let mut outcome = apply_block_once(
                game,
                steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            )?;
            if outcome.fired && !outcome.cancelled {
                let then_outcome = apply_block_once(
                    game,
                    then_steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                )?;
                outcome.merge(then_outcome);
            }
            Ok(outcome)
        }
        RuleStep::LocalFrame { frame, steps } => {
            let scoped_context = TransitionContext {
                local_frame: Some(frame),
                ..*context
            };
            apply_block_once(
                game,
                steps,
                &scoped_context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            )
        }
    }
}

fn apply_step_segment<F>(
    game: &CompiledGame,
    step: &RuleStep,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    match step {
        RuleStep::Rule(rule) => {
            let before_fired_len = fired_rules.len();
            let outcome = apply_rule_step(
                game,
                rule,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            )?;
            if outcome.fired
                && !outcome.cancelled
                && fired_rules.len() > before_fired_len
                && should_stop(ProgramBoundarySnapshot {
                    input: context.input,
                    next_state: current,
                    cancelled: false,
                    commands,
                    fired_rules,
                    patches,
                })
            {
                return Ok(SegmentApplyOutcome {
                    fired: true,
                    cancelled: false,
                    remaining_program: Some(ProgramContinuation::empty()),
                });
            }
            Ok(SegmentApplyOutcome::from_apply(outcome))
        }
        RuleStep::ConditionalBlock { condition, steps } => {
            if condition_accepts(game, condition, context, current) {
                apply_block_once_segment(
                    game,
                    steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                    should_stop,
                )
            } else {
                Ok(SegmentApplyOutcome::idle())
            }
        }
        RuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            let selected_steps = if condition_accepts(game, condition, context, current) {
                then_steps
            } else {
                else_steps
            };
            apply_block_once_segment(
                game,
                selected_steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            )
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => match application {
            RuleApplication::Once | RuleApplication::OnceAll | RuleApplication::OncePerLevel => {
                apply_block_once_segment(
                    game,
                    steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                    should_stop,
                )
            }
            RuleApplication::Random => {
                let before_fired_len = fired_rules.len();
                let outcome = apply_block_random(
                    game,
                    steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                )?;
                if outcome.fired
                    && !outcome.cancelled
                    && fired_rules.len() > before_fired_len
                    && should_stop(ProgramBoundarySnapshot {
                        input: context.input,
                        next_state: current,
                        cancelled: false,
                        commands,
                        fired_rules,
                        patches,
                    })
                {
                    return Ok(SegmentApplyOutcome {
                        fired: true,
                        cancelled: false,
                        remaining_program: Some(ProgramContinuation::empty()),
                    });
                }
                Ok(SegmentApplyOutcome::from_apply(outcome))
            }
            RuleApplication::UntilStable => apply_block_until_stable_segment(
                game,
                stop_condition.as_ref(),
                steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            ),
        },
        RuleStep::AfterTriggered { steps, then_steps } => {
            let mut outcome = apply_block_once_segment(
                game,
                steps,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            )?;
            if let Some(remaining_program) = outcome.remaining_program.take() {
                outcome.remaining_program = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        continuation: remaining_program,
                        then_steps: then_steps.clone(),
                        fired_so_far: outcome.fired,
                    },
                ));
                return Ok(outcome);
            }
            if outcome.fired && !outcome.cancelled {
                let then_outcome = apply_block_once_segment(
                    game,
                    then_steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                    should_stop,
                )?;
                outcome.merge(then_outcome);
            }
            Ok(outcome)
        }
        RuleStep::LocalFrame { frame, steps } => {
            let scoped_context = TransitionContext {
                local_frame: Some(frame),
                ..*context
            };
            let mut outcome = apply_block_once_segment(
                game,
                steps,
                &scoped_context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            )?;
            if let Some(remaining_steps) = outcome.remaining_program.take() {
                outcome.remaining_program = if remaining_steps.is_empty() {
                    Some(ProgramContinuation::empty())
                } else {
                    Some(ProgramContinuation::from_step(
                        ContinuationStep::LocalFrame {
                            frame: frame.clone(),
                            continuation: remaining_steps,
                        },
                    ))
                };
            }
            Ok(outcome)
        }
    }
}

fn condition_accepts(
    game: &CompiledGame,
    condition: &RuleCondition,
    context: &TransitionContext,
    state: &State,
) -> bool {
    match condition {
        RuleCondition::AnyMatches(patterns) => patterns
            .iter()
            .any(|pattern| has_pattern_match_in_scope(game, state, pattern, context.local_frame)),
        RuleCondition::NoMatches(patterns) => patterns
            .iter()
            .all(|pattern| !has_pattern_match_in_scope(game, state, pattern, context.local_frame)),
        RuleCondition::AnyInputMatches(patterns) => patterns.iter().any(|(input, pattern)| {
            *input == context.input
                && has_pattern_match_in_scope(game, state, pattern, context.local_frame)
        }),
        RuleCondition::NoInputMatches(patterns) => patterns.iter().all(|(input, pattern)| {
            *input != context.input
                || !has_pattern_match_in_scope(game, state, pattern, context.local_frame)
        }),
        RuleCondition::GuardBranches(branches) => branches.iter().any(|branch| {
            branch
                .iter()
                .all(|guard| guard_accepts(guard, context, state))
        }),
    }
}

fn apply_rule_step(
    game: &CompiledGame,
    rule: &Rule,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    if !guards_accept(rule, context, current) {
        return Ok(ApplyOutcome::idle());
    }

    match rule.application {
        RuleApplication::Once => apply_rule_once(
            game,
            rule,
            context.local_frame,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::OnceAll => apply_rule_once_all(
            game,
            rule,
            context.local_frame,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::OncePerLevel => apply_rule_once_per_level(
            game,
            rule,
            context.local_frame,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::Random => apply_rule_random(
            game,
            rule,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::UntilStable => apply_until_stable(
            game,
            rule,
            context.local_frame,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
    }
}

fn apply_block_once(
    game: &CompiledGame,
    steps: &[RuleStep],
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let mut outcome = ApplyOutcome::idle();
    for step in steps {
        let step_outcome = apply_step(
            game,
            step,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        )?;
        outcome.merge(step_outcome);
        if outcome.cancelled {
            break;
        }
    }
    Ok(outcome)
}

fn apply_block_once_segment<F>(
    game: &CompiledGame,
    steps: &[RuleStep],
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let mut outcome = SegmentApplyOutcome::idle();
    for (index, step) in steps.iter().enumerate() {
        let mut step_outcome = apply_step_segment(
            game,
            step,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        )?;
        if let Some(mut remaining_program) = step_outcome.remaining_program.take() {
            remaining_program.extend_rule_steps(&steps[index + 1..]);
            step_outcome.remaining_program = Some(remaining_program);
            outcome.merge(step_outcome);
            return Ok(outcome);
        }
        outcome.merge(step_outcome);
        if outcome.cancelled {
            break;
        }
    }
    Ok(outcome)
}

fn apply_continuation_segment<F>(
    game: &CompiledGame,
    continuation: &ProgramContinuation,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let mut outcome = SegmentApplyOutcome::idle();
    for (index, step) in continuation.steps.iter().enumerate() {
        let mut step_outcome = apply_continuation_step_segment(
            game,
            step,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        )?;
        if let Some(mut remaining_program) = step_outcome.remaining_program.take() {
            remaining_program.extend_continuation_steps(&continuation.steps[index + 1..]);
            step_outcome.remaining_program = Some(remaining_program);
            outcome.merge(step_outcome);
            return Ok(outcome);
        }
        outcome.merge(step_outcome);
        if outcome.cancelled {
            break;
        }
    }
    Ok(outcome)
}

fn apply_continuation_step_segment<F>(
    game: &CompiledGame,
    step: &ContinuationStep,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    match step {
        ContinuationStep::RuleStep(step) => apply_step_segment(
            game,
            step,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        ),
        ContinuationStep::LocalFrame {
            frame,
            continuation,
        } => {
            let scoped_context = TransitionContext {
                local_frame: Some(frame),
                ..*context
            };
            let mut outcome = apply_continuation_segment(
                game,
                continuation,
                &scoped_context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            )?;
            if let Some(remaining) = outcome.remaining_program.take() {
                outcome.remaining_program = if remaining.is_empty() {
                    Some(ProgramContinuation::empty())
                } else {
                    Some(ProgramContinuation::from_step(
                        ContinuationStep::LocalFrame {
                            frame: frame.clone(),
                            continuation: remaining,
                        },
                    ))
                };
            }
            Ok(outcome)
        }
        ContinuationStep::AfterTriggered {
            continuation,
            then_steps,
            fired_so_far,
        } => {
            let mut outcome = apply_continuation_segment(
                game,
                continuation,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
                should_stop,
            )?;
            let fired = *fired_so_far || outcome.fired;
            if let Some(remaining) = outcome.remaining_program.take() {
                outcome.remaining_program = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        continuation: remaining,
                        then_steps: then_steps.clone(),
                        fired_so_far: fired,
                    },
                ));
                return Ok(outcome);
            }
            if fired && !outcome.cancelled {
                let then_outcome = apply_block_once_segment(
                    game,
                    then_steps,
                    context,
                    current,
                    fired_rules,
                    patches,
                    commands,
                    collect_trace,
                    should_stop,
                )?;
                outcome.merge(then_outcome);
            }
            Ok(outcome)
        }
        ContinuationStep::UntilStable {
            stop_condition,
            steps,
            before,
            before_hash,
            seen_states,
            fired_any,
            pass_fired,
            repeat_count,
            remaining_pass,
        } => apply_until_stable_continuation_segment(
            game,
            stop_condition.as_ref(),
            steps,
            before,
            *before_hash,
            seen_states.clone(),
            *fired_any,
            *pass_fired,
            *repeat_count,
            remaining_pass,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        ),
    }
}

#[derive(Clone, Debug)]
struct RandomBlockCandidate {
    next_state: State,
    fired_rules: Vec<RuleId>,
    patches: Vec<Patch>,
    commands: Vec<TransitionCommand>,
    cancelled: bool,
}

fn apply_block_random(
    game: &CompiledGame,
    steps: &[RuleStep],
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let mut candidates = Vec::new();
    for step in steps {
        let mut candidate_state = current.clone();
        let mut candidate_fired_rules = Vec::new();
        let mut candidate_patches = Vec::new();
        let mut candidate_commands = Vec::new();
        let outcome = apply_step(
            game,
            step,
            context,
            &mut candidate_state,
            &mut candidate_fired_rules,
            &mut candidate_patches,
            &mut candidate_commands,
            true,
        )?;
        if outcome.fired {
            candidates.push(RandomBlockCandidate {
                next_state: candidate_state,
                fired_rules: candidate_fired_rules,
                patches: candidate_patches,
                commands: candidate_commands,
                cancelled: outcome.cancelled,
            });
        }
    }
    if candidates.is_empty() {
        return Ok(ApplyOutcome::idle());
    }

    let index = random_choice_index(game, current, context.input, RuleId(0), candidates.len());
    let candidate = candidates.swap_remove(index);
    *current = candidate.next_state;
    fired_rules.extend(candidate.fired_rules);
    if collect_trace {
        patches.extend(candidate.patches);
    }
    if !candidate.cancelled {
        commands.extend(candidate.commands);
    }
    Ok(ApplyOutcome {
        fired: true,
        cancelled: candidate.cancelled,
    })
}

fn apply_block_until_stable(
    game: &CompiledGame,
    stop_condition: Option<&RuleCondition>,
    steps: &[RuleStep],
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let mut seen_states = StateHistory::from_current(current);
    let mut fired_any = false;
    let mut repeat_count = 0;

    loop {
        if stop_condition
            .is_some_and(|condition| condition_accepts(game, condition, context, current))
        {
            break;
        }
        let before_hash = current.hash();
        let before = current.clone();
        let pass_outcome = apply_block_once(
            game,
            steps,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        )?;
        if pass_outcome.cancelled {
            return Ok(pass_outcome);
        }
        if !pass_outcome.fired {
            break;
        }
        fired_any = true;
        if current.hash() == before_hash && *current == before {
            break;
        }
        if !seen_states.insert(current) {
            break;
        }
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
            break;
        }
    }

    Ok(ApplyOutcome {
        fired: fired_any,
        cancelled: false,
    })
}

fn apply_block_until_stable_segment<F>(
    game: &CompiledGame,
    stop_condition: Option<&RuleCondition>,
    steps: &[RuleStep],
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let mut seen_states = StateHistory::from_current(current);
    let mut fired_any = false;
    let mut repeat_count = 0;

    loop {
        if stop_condition
            .is_some_and(|condition| condition_accepts(game, condition, context, current))
        {
            break;
        }
        let before_hash = current.hash();
        let before = current.clone();
        let pass_outcome = apply_block_once_segment(
            game,
            steps,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass_outcome.remaining_program {
            return Ok(SegmentApplyOutcome {
                fired: pass_outcome.fired,
                cancelled: false,
                remaining_program: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        stop_condition: stop_condition.cloned(),
                        steps: steps.to_vec(),
                        before,
                        before_hash,
                        seen_states,
                        fired_any,
                        pass_fired: pass_outcome.fired,
                        repeat_count,
                        remaining_pass,
                    },
                )),
            });
        }
        if pass_outcome.cancelled {
            return Ok(pass_outcome);
        }
        if !pass_outcome.fired {
            break;
        }
        fired_any = true;
        if current.hash() == before_hash && *current == before {
            break;
        }
        if !seen_states.insert(current) {
            break;
        }
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
            break;
        }
    }

    Ok(SegmentApplyOutcome {
        fired: fired_any,
        cancelled: false,
        remaining_program: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_until_stable_continuation_segment<F>(
    game: &CompiledGame,
    stop_condition: Option<&RuleCondition>,
    steps: &[RuleStep],
    before: &State,
    before_hash: u64,
    mut seen_states: StateHistory,
    mut fired_any: bool,
    pass_fired_before_wait: bool,
    mut repeat_count: usize,
    remaining_pass: &ProgramContinuation,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
    should_stop: &mut F,
) -> TransitionResult<SegmentApplyOutcome>
where
    F: FnMut(ProgramBoundarySnapshot<'_>) -> bool,
{
    let remaining_outcome = apply_continuation_segment(
        game,
        remaining_pass,
        context,
        current,
        fired_rules,
        patches,
        commands,
        collect_trace,
        should_stop,
    )?;
    if let Some(remaining_pass) = remaining_outcome.remaining_program {
        return Ok(SegmentApplyOutcome {
            fired: pass_fired_before_wait || remaining_outcome.fired,
            cancelled: false,
            remaining_program: Some(ProgramContinuation::from_step(
                ContinuationStep::UntilStable {
                    stop_condition: stop_condition.cloned(),
                    steps: steps.to_vec(),
                    before: before.clone(),
                    before_hash,
                    seen_states,
                    fired_any,
                    pass_fired: pass_fired_before_wait || remaining_outcome.fired,
                    repeat_count,
                    remaining_pass,
                },
            )),
        });
    }
    if remaining_outcome.cancelled {
        return Ok(remaining_outcome);
    }

    let pass_fired = pass_fired_before_wait || remaining_outcome.fired;
    if !pass_fired {
        return Ok(SegmentApplyOutcome {
            fired: fired_any,
            cancelled: false,
            remaining_program: None,
        });
    }
    fired_any = true;
    if current.hash() == before_hash && *current == *before {
        return Ok(SegmentApplyOutcome {
            fired: true,
            cancelled: false,
            remaining_program: None,
        });
    }
    if !seen_states.insert(current) {
        return Ok(SegmentApplyOutcome {
            fired: true,
            cancelled: false,
            remaining_program: None,
        });
    }
    repeat_count += 1;
    if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
        return Ok(SegmentApplyOutcome {
            fired: true,
            cancelled: false,
            remaining_program: None,
        });
    }

    loop {
        if stop_condition
            .is_some_and(|condition| condition_accepts(game, condition, context, current))
        {
            break;
        }
        let before_hash = current.hash();
        let before = current.clone();
        let pass_outcome = apply_block_once_segment(
            game,
            steps,
            context,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass_outcome.remaining_program {
            return Ok(SegmentApplyOutcome {
                fired: pass_outcome.fired,
                cancelled: false,
                remaining_program: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        stop_condition: stop_condition.cloned(),
                        steps: steps.to_vec(),
                        before,
                        before_hash,
                        seen_states,
                        fired_any,
                        pass_fired: pass_outcome.fired,
                        repeat_count,
                        remaining_pass,
                    },
                )),
            });
        }
        if pass_outcome.cancelled {
            return Ok(pass_outcome);
        }
        if !pass_outcome.fired {
            break;
        }
        fired_any = true;
        if current.hash() == before_hash && *current == before {
            break;
        }
        if !seen_states.insert(current) {
            break;
        }
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
            break;
        }
    }

    Ok(SegmentApplyOutcome {
        fired: fired_any,
        cancelled: false,
        remaining_program: None,
    })
}

#[derive(Clone, Debug)]
struct StateHistory {
    states_by_hash: BTreeMap<u64, Vec<State>>,
}

impl StateHistory {
    fn from_current(state: &State) -> Self {
        let mut history = Self {
            states_by_hash: BTreeMap::new(),
        };
        history
            .states_by_hash
            .entry(state.hash())
            .or_default()
            .push(state.clone());
        history
    }

    fn insert(&mut self, state: &State) -> bool {
        let states = self.states_by_hash.entry(state.hash()).or_default();
        if states.iter().any(|seen| seen == state) {
            return false;
        }
        states.push(state.clone());
        true
    }
}

fn deterministic_mix(hash: u64, value: u64) -> u64 {
    hash.wrapping_mul(FNV_PRIME) ^ value
}

fn random_state_projection_hash(game: &CompiledGame, state: &State) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = deterministic_mix(hash, u64::from(state.width));
    hash = deterministic_mix(hash, u64::from(state.height));
    hash = deterministic_mix(hash, u64::from(state.layer_count));

    let main_layers = game.main_layers();
    hash = deterministic_mix(hash, main_layers.len() as u64);
    for y in 0..state.height {
        for x in 0..state.width {
            for layer in &main_layers {
                let index = ((usize::from(y) * usize::from(state.width) + usize::from(x))
                    * usize::from(state.layer_count))
                    + usize::from(layer.0);
                let object = state.slots()[index];
                let object = if game.is_main_object(object) {
                    object
                } else {
                    ObjectId::EMPTY
                };
                hash = deterministic_mix(hash, u64::from(object.0));
            }
        }
    }
    hash = deterministic_mix(hash, state.visible_variables().len() as u64);
    for value in state.visible_variables() {
        hash = deterministic_mix(hash, *value as u64);
    }
    hash = deterministic_mix(hash, state.level_fired_rules().len() as u64);
    for rule in state.level_fired_rules() {
        hash = deterministic_mix(hash, u64::from(rule.0));
    }
    hash
}

fn random_choice_index(
    game: &CompiledGame,
    state: &State,
    input: InputId,
    rule: RuleId,
    candidate_count: usize,
) -> usize {
    let mut hash = random_state_projection_hash(game, state);
    hash = deterministic_mix(hash, u64::from(input.0));
    hash = deterministic_mix(hash, u64::from(rule.0));
    hash = deterministic_mix(hash, candidate_count as u64);
    (hash as usize) % candidate_count
}

fn apply_rule_once(
    game: &CompiledGame,
    rule: &Rule,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let scope = LocalFrameScope2::new(current, local_frame);
    let cancels = rule
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::Cancel));
    let mut selected_patch = None;
    for placement in find_all_matches(game, current, rule, &scope) {
        if overlapping_component_writes_conflict(game, rule, &placement)? {
            continue;
        }
        let patch = build_patch(rule, &placement)?;
        let applied = if cancels {
            match patch.validate(game, current) {
                Ok(_) => true,
                Err(error) if overlapping_writes_conflict(&error) => false,
                Err(error) => return Err(error.into()),
            }
        } else {
            match patch.apply_in_place(game, current) {
                Ok(_) => true,
                Err(error) if overlapping_writes_conflict(&error) => false,
                Err(error) => return Err(error.into()),
            }
        };
        if applied {
            selected_patch = Some(patch);
            break;
        }
    }
    let Some(patch) = selected_patch else {
        return Ok(ApplyOutcome::idle());
    };
    fired_rules.push(rule.id);
    if collect_trace {
        patches.push(patch);
    }
    if cancels {
        return Ok(ApplyOutcome {
            fired: true,
            cancelled: true,
        });
    }
    push_rule_commands(rule, commands);
    Ok(ApplyOutcome {
        fired: true,
        cancelled: false,
    })
}

fn apply_rule_random(
    game: &CompiledGame,
    rule: &Rule,
    context: &TransitionContext,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let scope = LocalFrameScope2::new(current, context.local_frame);
    let placements = find_all_matches(game, current, rule, &scope);
    let cancels = rule
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::Cancel));
    let mut applicable = Vec::new();
    for placement in placements {
        if overlapping_component_writes_conflict(game, rule, &placement)? {
            continue;
        }
        let patch = build_patch(rule, &placement)?;
        match patch.validate(game, current) {
            Ok(_) => applicable.push((placement, patch)),
            Err(error) if overlapping_writes_conflict(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }
    if applicable.is_empty() {
        return Ok(ApplyOutcome::idle());
    }
    let index = random_choice_index(game, current, context.input, rule.id, applicable.len());
    let (_, patch) = applicable.swap_remove(index);
    if !cancels {
        patch.apply_in_place(game, current)?;
    }
    fired_rules.push(rule.id);
    if collect_trace {
        patches.push(patch);
    }
    if cancels {
        return Ok(ApplyOutcome {
            fired: true,
            cancelled: true,
        });
    }
    push_rule_commands(rule, commands);
    Ok(ApplyOutcome {
        fired: true,
        cancelled: false,
    })
}

fn apply_rule_once_all(
    game: &CompiledGame,
    rule: &Rule,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let scope = LocalFrameScope2::new(current, local_frame);
    let placements = find_all_matches(game, current, rule, &scope);
    if placements.is_empty() {
        return Ok(ApplyOutcome::idle());
    }

    let mut fired = false;
    let mut current_scope = LocalFrameScope2::new(current, local_frame);
    for placement in placements {
        if !placement_matches(game, current, rule, &placement, &current_scope) {
            continue;
        }
        if overlapping_component_writes_conflict(game, rule, &placement)? {
            continue;
        }

        let patch = build_patch(rule, &placement)?;
        let cancels = rule
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel));
        let applied = if cancels {
            match patch.validate(game, current) {
                Ok(_) => true,
                Err(error) if overlapping_writes_conflict(&error) => false,
                Err(error) => return Err(error.into()),
            }
        } else {
            match patch.apply_in_place(game, current) {
                Ok(_) => true,
                Err(error) if overlapping_writes_conflict(&error) => false,
                Err(error) => return Err(error.into()),
            }
        };
        if !applied {
            continue;
        }

        fired = true;
        fired_rules.push(rule.id);
        if collect_trace {
            patches.push(patch);
        }
        if cancels {
            return Ok(ApplyOutcome {
                fired: true,
                cancelled: true,
            });
        }
        current_scope = LocalFrameScope2::new(current, local_frame);
    }

    if !fired {
        return Ok(ApplyOutcome::idle());
    }

    push_rule_commands(rule, commands);
    Ok(ApplyOutcome {
        fired: true,
        cancelled: false,
    })
}

fn overlapping_writes_conflict(error: &PatchError) -> bool {
    matches!(
        error,
        PatchError::ExpectedObject { .. } | PatchError::LayerOccupied { .. }
    )
}

fn overlapping_component_writes_conflict(
    game: &CompiledGame,
    rule: &Rule,
    placement: &MatchPlacement,
) -> TransitionResult<bool> {
    let mut intents = BTreeMap::<(u16, u16, crate::ids::LayerId), (u16, ObjectId)>::new();

    for write in &rule.writes {
        match write {
            WriteOp::Add {
                component,
                offset,
                object,
            } => {
                let position =
                    write_position_for_components(&placement.components, *component, offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                if record_component_write_intent(game, &mut intents, *component, position, *object)
                {
                    return Ok(true);
                }
            }
            WriteOp::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                let position =
                    write_position_for_components(&placement.components, *component, offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                if record_component_write_intent(game, &mut intents, *component, position, object) {
                    return Ok(true);
                }
            }
            WriteOp::Remove {
                component,
                offset,
                object,
            } => {
                let position =
                    write_position_for_components(&placement.components, *component, offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let layer = game
                    .object_layer(*object)
                    .expect("compiled write object has a layer");
                if record_layer_write_intent(
                    &mut intents,
                    *component,
                    position,
                    layer,
                    ObjectId::EMPTY,
                ) {
                    return Ok(true);
                }
            }
            WriteOp::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                let position =
                    write_position_for_components(&placement.components, *component, offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let layer = game
                    .object_layer(object)
                    .expect("compiled write object has a layer");
                if record_layer_write_intent(
                    &mut intents,
                    *component,
                    position,
                    layer,
                    ObjectId::EMPTY,
                ) {
                    return Ok(true);
                }
            }
            WriteOp::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                let position =
                    write_position_for_components(&placement.components, *component, offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let remove_layer = game
                    .object_layer(*remove)
                    .expect("compiled write object has a layer");
                if record_layer_write_intent(
                    &mut intents,
                    *component,
                    position,
                    remove_layer,
                    ObjectId::EMPTY,
                ) {
                    return Ok(true);
                }
                if record_component_write_intent(game, &mut intents, *component, position, *add) {
                    return Ok(true);
                }
            }
            WriteOp::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                let from =
                    write_position_for_components(&placement.components, *component, from_offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let to =
                    write_position_for_components(&placement.components, *component, to_offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let layer = game
                    .object_layer(*object)
                    .expect("compiled write object has a layer");
                if record_layer_write_intent(&mut intents, *component, from, layer, ObjectId::EMPTY)
                {
                    return Ok(true);
                }
                if record_component_write_intent(game, &mut intents, *component, to, *object) {
                    return Ok(true);
                }
            }
            WriteOp::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                let from =
                    write_position_for_components(&placement.components, *component, from_offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let to =
                    write_position_for_components(&placement.components, *component, to_offset)
                        .ok_or(TransitionError::OffsetOutOfBounds)?;
                let layer = game
                    .object_layer(object)
                    .expect("compiled write object has a layer");
                if record_layer_write_intent(&mut intents, *component, from, layer, ObjectId::EMPTY)
                {
                    return Ok(true);
                }
                if record_component_write_intent(game, &mut intents, *component, to, object) {
                    return Ok(true);
                }
            }
            WriteOp::SetMark { .. }
            | WriteOp::SetObjectSetMark { .. }
            | WriteOp::RemoveMark { .. }
            | WriteOp::RemoveObjectSetMark { .. } => {}
        }
    }
    Ok(false)
}

fn record_component_write_intent(
    game: &CompiledGame,
    intents: &mut BTreeMap<(u16, u16, crate::ids::LayerId), (u16, ObjectId)>,
    component: u16,
    position: (u16, u16),
    object: ObjectId,
) -> bool {
    let Some(layer) = game.object_layer(object) else {
        return false;
    };
    record_layer_write_intent(intents, component, position, layer, object)
}

fn record_layer_write_intent(
    intents: &mut BTreeMap<(u16, u16, crate::ids::LayerId), (u16, ObjectId)>,
    component: u16,
    position: (u16, u16),
    layer: crate::ids::LayerId,
    object: ObjectId,
) -> bool {
    let key = (position.0, position.1, layer);
    if let Some((existing_component, existing_object)) = intents.get(&key) {
        if *existing_component != component && *existing_object != object {
            return true;
        }
    }
    intents.insert(key, (component, object));
    false
}

fn apply_rule_once_per_level(
    game: &CompiledGame,
    rule: &Rule,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    if current.level_rule_has_fired(rule.id) {
        return Ok(ApplyOutcome::idle());
    }
    let outcome = apply_rule_once(
        game,
        rule,
        local_frame,
        current,
        fired_rules,
        patches,
        commands,
        collect_trace,
    )?;
    if outcome.fired && !outcome.cancelled {
        current.mark_level_rule_fired(rule.id);
    }
    Ok(outcome)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApplyOutcome {
    fired: bool,
    cancelled: bool,
}

impl ApplyOutcome {
    fn idle() -> Self {
        Self {
            fired: false,
            cancelled: false,
        }
    }

    fn merge(&mut self, other: Self) {
        self.fired |= other.fired;
        self.cancelled |= other.cancelled;
    }
}

#[derive(Clone, Debug)]
struct SegmentApplyOutcome {
    fired: bool,
    cancelled: bool,
    remaining_program: Option<ProgramContinuation>,
}

impl SegmentApplyOutcome {
    fn idle() -> Self {
        Self {
            fired: false,
            cancelled: false,
            remaining_program: None,
        }
    }

    fn from_apply(outcome: ApplyOutcome) -> Self {
        Self {
            fired: outcome.fired,
            cancelled: outcome.cancelled,
            remaining_program: None,
        }
    }

    fn merge(&mut self, other: Self) {
        self.fired |= other.fired;
        self.cancelled |= other.cancelled;
        if other.remaining_program.is_some() {
            self.remaining_program = other.remaining_program;
        }
    }
}

fn push_rule_commands(rule: &Rule, commands: &mut Vec<TransitionCommand>) {
    for effect in &rule.effects {
        match effect {
            Effect::Win => commands.push(TransitionCommand::Win),
            Effect::Restart => commands.push(TransitionCommand::Restart),
            Effect::NextLevel => commands.push(TransitionCommand::NextLevel),
            Effect::Again => commands.push(TransitionCommand::Again),
            Effect::Checkpoint => commands.push(TransitionCommand::Checkpoint),
            Effect::ClearCheckpoint => commands.push(TransitionCommand::ClearCheckpoint),
            Effect::Cancel | Effect::UpdateVariable { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TransitionContext<'a> {
    game: &'a CompiledGame,
    input: InputId,
    local_frame: Option<&'a LocalFrame<crate::ids::ObjectId>>,
}

fn guards_accept(rule: &Rule, context: &TransitionContext, state: &State) -> bool {
    rule.guards
        .iter()
        .all(|guard| guard_accepts(guard, context, state))
}

fn guard_accepts(guard: &Guard, context: &TransitionContext, state: &State) -> bool {
    match guard {
        Guard::InputIs(expected) => context.input == *expected,
        Guard::VariableEquals { variable, value } => {
            state.variable_value(*variable) == Some(*value)
        }
        Guard::VariableCompare {
            variable,
            op,
            value,
        } => state
            .variable_value(*variable)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        Guard::ConditionEquals { condition, value } => {
            eval_condition_def(context, state, *condition) == Some(*value)
        }
        Guard::ConditionNonZero(condition) => {
            eval_condition_def(context, state, *condition).is_some_and(|value| value != 0)
        }
        Guard::ConditionCompare {
            condition,
            op,
            value,
        } => eval_condition_def(context, state, *condition)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        Guard::InlineConditionValue { kind, value } => {
            eval_condition_value_kind(context, state, kind) == *value
        }
        Guard::InlineConditionNonZero(kind) => eval_condition_value_kind(context, state, kind) != 0,
        Guard::InlineConditionCompare { kind, op, value } => {
            compare_i64(eval_condition_value_kind(context, state, kind), *op, *value)
        }
    }
}

fn compare_i64(left: i64, op: crate::ComparisonOp, right: i64) -> bool {
    match op {
        crate::ComparisonOp::Eq => left == right,
        crate::ComparisonOp::NotEq => left != right,
        crate::ComparisonOp::Greater => left > right,
        crate::ComparisonOp::GreaterEq => left >= right,
        crate::ComparisonOp::Less => left < right,
        crate::ComparisonOp::LessEq => left <= right,
    }
}

fn eval_condition_def(
    context: &TransitionContext,
    state: &State,
    condition: ConditionId,
) -> Option<i64> {
    let condition = context.game.condition_def(condition)?;
    Some(eval_condition_value_kind(context, state, &condition.kind))
}

fn eval_condition_value_kind(
    context: &TransitionContext,
    state: &State,
    kind: &ConditionValueKind,
) -> i64 {
    match kind {
        ConditionValueKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        ConditionValueKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| {
                i64::from(count_pattern_matches_in_scope(
                    context.game,
                    state,
                    pattern,
                    context.local_frame,
                ))
            })
            .sum(),
        ConditionValueKind::ExistsMatches(patterns) => {
            if patterns.iter().any(|pattern| {
                has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneMatches(patterns) => {
            if patterns.iter().any(|pattern| {
                has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                0
            } else {
                1
            }
        }
        ConditionValueKind::CountInputMatches(patterns) => patterns
            .iter()
            .filter(|(input, _)| *input == context.input)
            .map(|(_, pattern)| {
                i64::from(count_pattern_matches_in_scope(
                    context.game,
                    state,
                    pattern,
                    context.local_frame,
                ))
            })
            .sum(),
        ConditionValueKind::ExistsInputMatches(patterns) => {
            if patterns.iter().any(|(input, pattern)| {
                *input == context.input
                    && has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                1
            } else {
                0
            }
        }
        ConditionValueKind::NoneInputMatches(patterns) => {
            if patterns.iter().any(|(input, pattern)| {
                *input == context.input
                    && has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                0
            } else {
                1
            }
        }
    }
}

pub fn count_pattern_matches(game: &CompiledGame, state: &State, pattern: &Pattern) -> u32 {
    count_pattern_matches_in_scope(game, state, pattern, None)
}

fn count_pattern_matches_in_scope(
    game: &CompiledGame,
    state: &State,
    pattern: &Pattern,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
) -> u32 {
    if pattern.components.is_empty() {
        return 0;
    }

    let rule = Rule {
        id: RuleId(0),
        guards: Vec::new(),
        application: RuleApplication::Once,
        pattern: pattern.clone(),
        writes: Vec::new(),
        effects: Vec::new(),
    };
    let mut count = 0;
    let scope = LocalFrameScope2::new(state, local_frame);
    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0], &scope) {
        if match_from_first_origin(game, state, &rule, x, y, &scope).is_some() {
            count += 1;
        }
    }
    count
}

pub fn has_pattern_match(game: &CompiledGame, state: &State, pattern: &Pattern) -> bool {
    has_pattern_match_in_scope(game, state, pattern, None)
}

fn has_pattern_match_in_scope(
    game: &CompiledGame,
    state: &State,
    pattern: &Pattern,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
) -> bool {
    if pattern.components.is_empty() {
        return false;
    }

    let rule = Rule {
        id: RuleId(0),
        guards: Vec::new(),
        application: RuleApplication::Once,
        pattern: pattern.clone(),
        writes: Vec::new(),
        effects: Vec::new(),
    };
    let scope = LocalFrameScope2::new(state, local_frame);
    component_candidate_origins(game, state, &rule.pattern.components[0], &scope)
        .into_iter()
        .any(|(x, y)| match_from_first_origin(game, state, &rule, x, y, &scope).is_some())
}

type MatchPlacement = puzzle_kernel::MatchPlacement<2, ObjectId>;
type ComponentPlacement = puzzle_kernel::ComponentPlacement<2, ObjectId>;
type ObjectBinding = puzzle_kernel::ObjectBinding<ObjectId>;

fn find_all_matches(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    scope: &LocalFrameScope2<'_>,
) -> Vec<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return vec![MatchPlacement::empty()];
    }

    let mut matches = Vec::new();
    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0], scope) {
        if let Some(first) =
            component_placement_at(game, state, &rule.pattern.components[0], x, y, scope)
        {
            let mut components = vec![first];
            collect_component_placements(
                game,
                state,
                rule,
                1,
                &mut components,
                &mut matches,
                scope,
            );
        }
    }
    matches
}

fn complete_component_placements(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    component_index: usize,
    components: &mut Vec<ComponentPlacement>,
    scope: &LocalFrameScope2<'_>,
) -> bool {
    let mut candidate_origins =
        |component: &PatternComponent| component_candidate_origins(game, state, component, scope);
    let mut place_at = |component: &PatternComponent, (x, y)| {
        component_placement_at(game, state, component, x, y, scope)
    };
    complete_component_placements_shared(
        &rule.pattern.components,
        component_index,
        components,
        &mut candidate_origins,
        &mut place_at,
    )
}

fn collect_component_placements(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    component_index: usize,
    components: &mut Vec<ComponentPlacement>,
    matches: &mut Vec<MatchPlacement>,
    scope: &LocalFrameScope2<'_>,
) {
    let mut candidate_origins =
        |component: &PatternComponent| component_candidate_origins(game, state, component, scope);
    let mut place_at = |component: &PatternComponent, (x, y)| {
        component_placement_at(game, state, component, x, y, scope)
    };
    let mut push_match = |matches: &mut Vec<MatchPlacement>, components: &[ComponentPlacement]| {
        if placement_writes_within_local_frame(rule, components, scope) {
            matches.push(MatchPlacement::new(components.to_vec()));
        }
    };
    collect_component_placements_shared(
        &rule.pattern.components,
        component_index,
        components,
        matches,
        &mut candidate_origins,
        &mut place_at,
        &mut push_match,
    );
}

fn apply_until_stable(
    game: &CompiledGame,
    rule: &Rule,
    local_frame: Option<&LocalFrame<crate::ids::ObjectId>>,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let dirty_origin_deltas = dirty_origin_deltas(rule);
    let mut scope = LocalFrameScope2::new(current, local_frame);
    let mut candidates = first_component_candidate_origin_set(game, current, rule, &scope);
    let mut seen_states = StateHistory::from_current(current);
    let mut fired = false;
    let mut repeat_count = 0;

    while let Some((y, x)) = pop_first_origin(&mut candidates) {
        let Some(placement) = match_from_first_origin(game, current, rule, x, y, &scope) else {
            continue;
        };

        let patch = build_patch(rule, &placement)?;
        let cancels = rule
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel));
        let has_transition_command = rule.effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Win
                    | Effect::Restart
                    | Effect::NextLevel
                    | Effect::Again
                    | Effect::Checkpoint
                    | Effect::ClearCheckpoint
            )
        });
        let changed = if cancels {
            patch.validate(game, current)?
        } else {
            patch.apply_in_place(game, current)?
        };
        if !changed {
            fired_rules.push(rule.id);
            fired = true;
            if collect_trace {
                patches.push(patch);
            }
            if cancels {
                return Ok(ApplyOutcome {
                    fired: true,
                    cancelled: true,
                });
            }
            if has_transition_command {
                push_rule_commands(rule, commands);
            }
            continue;
        }
        fired_rules.push(rule.id);
        fired = true;
        if collect_trace {
            patches.push(patch);
        }
        if cancels {
            return Ok(ApplyOutcome {
                fired: true,
                cancelled: true,
            });
        }
        scope = LocalFrameScope2::new(current, local_frame);
        push_rule_commands(rule, commands);
        if !seen_states.insert(current) {
            break;
        }
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
            break;
        }
        if let Some(dirty_origin_deltas) = &dirty_origin_deltas {
            enqueue_dirty_origins(&mut candidates, x, y, dirty_origin_deltas, current);
        } else {
            candidates = first_component_candidate_origin_set(game, current, rule, &scope);
        }
    }

    Ok(ApplyOutcome {
        fired,
        cancelled: false,
    })
}

fn match_from_first_origin(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    origin_x: u16,
    origin_y: u16,
    scope: &LocalFrameScope2<'_>,
) -> Option<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return None;
    }

    let first = component_placement_at(
        game,
        state,
        &rule.pattern.components[0],
        origin_x,
        origin_y,
        scope,
    )?;
    let mut components = vec![first];
    if complete_component_placements(game, state, rule, 1, &mut components, scope)
        && placement_writes_within_local_frame(rule, &components, scope)
    {
        Some(MatchPlacement::new(components))
    } else {
        None
    }
}

fn placement_matches(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    placement: &MatchPlacement,
    scope: &LocalFrameScope2<'_>,
) -> bool {
    if placement.components.len() != rule.pattern.components.len() {
        return false;
    }

    rule.pattern
        .components
        .iter()
        .zip(&placement.components)
        .all(|(component, placement)| {
            component_matches_with_gaps(
                game,
                state,
                component,
                placement.origin.axes()[0],
                placement.origin.axes()[1],
                &placement.gaps,
                scope,
            )
            .is_some_and(|bindings| object_bindings_match(&bindings, &placement.object_bindings))
        })
        && placement_writes_within_local_frame(rule, &placement.components, scope)
}

fn object_bindings_match(left: &[ObjectBinding], right: &[ObjectBinding]) -> bool {
    left.len() == right.len()
        && left.iter().all(|binding| {
            right.iter().any(|existing| {
                existing.binding == binding.binding && existing.object == binding.object
            })
        })
}

struct LocalFrameScope2<'a> {
    frame: Option<&'a LocalFrame<ObjectId>>,
    focus_cells: Vec<(u16, u16)>,
}

impl<'a> LocalFrameScope2<'a> {
    fn new(state: &State, frame: Option<&'a LocalFrame<ObjectId>>) -> Self {
        let focus_cells = frame
            .map(|frame| local_frame_focus_cells(state, frame))
            .unwrap_or_default();
        Self { frame, focus_cells }
    }

    fn contains_cell(&self, x: u16, y: u16) -> bool {
        let Some(frame) = self.frame else {
            return true;
        };
        self.focus_cells.iter().any(|(focus_x, focus_y)| {
            let dx = i32::from(x) - i32::from(*focus_x);
            let dy = i32::from(y) - i32::from(*focus_y);
            frame.contains_delta_2d(dx, dy)
        })
    }

    fn origin_candidates(&self, state: &State) -> Option<Vec<(u16, u16)>> {
        let frame = self.frame?;
        let mut origins = BTreeSet::new();
        for (focus_x, focus_y) in &self.focus_cells {
            let (x_range, y_range) = frame.ranges_2d(*focus_x, *focus_y, state.width, state.height);
            for y in y_range {
                for x in x_range.clone() {
                    origins.insert((y, x));
                }
            }
        }
        Some(origins.into_iter().map(|(y, x)| (x, y)).collect())
    }
}

fn local_frame_focus_cells(state: &State, local_frame: &LocalFrame<ObjectId>) -> Vec<(u16, u16)> {
    let mut cells = Vec::new();
    for object in &local_frame.focus_objects {
        for slot in state.object_positions(*object) {
            if let Some(position) = state.slot_position(*slot) {
                cells.push(position);
            }
        }
    }
    cells
}

#[derive(Clone, Debug)]
struct ComponentAnchor {
    kind: ComponentAnchorKind,
    dx: i16,
    dy: i16,
}

#[derive(Clone, Debug)]
enum ComponentAnchorKind {
    Object(ObjectId),
    ObjectSet {
        objects: Vec<ObjectId>,
    },
    SlotMark {
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    },
    CellMark {
        mark: MarkId,
        value: Option<i64>,
    },
}

fn first_component_candidate_origin_set(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    scope: &LocalFrameScope2<'_>,
) -> BTreeSet<(u16, u16)> {
    let Some(component) = rule.pattern.components.first() else {
        return BTreeSet::new();
    };
    component_candidate_origins(game, state, component, scope)
        .into_iter()
        .map(|(x, y)| (y, x))
        .collect()
}

fn component_candidate_origins(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
    scope: &LocalFrameScope2<'_>,
) -> Vec<(u16, u16)> {
    let Some(anchor) = component_anchor(game, state, component) else {
        return all_origin_vec(state, scope);
    };
    let mut origins = BTreeSet::new();
    for (x, y) in anchor_positions(game, state, &anchor) {
        let Some((origin_x, origin_y)) = offset_pos(x, y, -anchor.dx, -anchor.dy) else {
            continue;
        };
        if origin_x < state.width
            && origin_y < state.height
            && scope.contains_cell(origin_x, origin_y)
        {
            origins.insert((origin_y, origin_x));
        }
    }

    origins.into_iter().map(|(y, x)| (x, y)).collect()
}

fn anchor_positions(
    game: &CompiledGame,
    state: &State,
    anchor: &ComponentAnchor,
) -> Vec<(u16, u16)> {
    match &anchor.kind {
        ComponentAnchorKind::Object(object) => {
            if game.object_layer(*object).is_none() {
                return Vec::new();
            }
            state
                .object_positions(*object)
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect()
        }
        ComponentAnchorKind::ObjectSet { objects, .. } => {
            let mut positions = BTreeSet::new();
            for object in objects {
                if game.object_layer(*object).is_none() {
                    continue;
                }
                for slot in state.object_positions(*object) {
                    if let Some((x, y)) = state.slot_position(*slot) {
                        positions.insert((y, x));
                    }
                }
            }
            positions.into_iter().map(|(y, x)| (x, y)).collect()
        }
        ComponentAnchorKind::SlotMark {
            object,
            mark,
            value,
        } => {
            if game.object_layer(*object).is_none() {
                return Vec::new();
            }
            state
                .mark_positions(*object, *mark, *value)
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect()
        }
        ComponentAnchorKind::CellMark { mark, value } => state
            .mark_positions(ObjectId::EMPTY, *mark, *value)
            .iter()
            .filter_map(|cell| state.cell_position(*cell))
            .collect(),
    }
}

fn all_origin_vec(state: &State, scope: &LocalFrameScope2<'_>) -> Vec<(u16, u16)> {
    if let Some(origins) = scope.origin_candidates(state) {
        return origins;
    }
    let mut origins =
        Vec::with_capacity(usize::from(state.width).saturating_mul(usize::from(state.height)));
    for y in 0..state.height {
        for x in 0..state.width {
            origins.push((x, y));
        }
    }
    origins
}

fn component_anchor(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
) -> Option<ComponentAnchor> {
    let mut best = None;
    for cell in &component.cells {
        let Offset::Fixed { dx, dy } = cell.offset else {
            continue;
        };
        for object in &cell.require_objects {
            if game.object_layer(*object).is_none() {
                continue;
            }
            let count = state.object_count(*object);
            if best
                .as_ref()
                .is_none_or(|(best_count, _)| count < *best_count)
            {
                best = Some((
                    count,
                    ComponentAnchor {
                        kind: ComponentAnchorKind::Object(*object),
                        dx,
                        dy,
                    },
                ));
            }
        }
        for object_set in &cell.require_object_sets {
            let count = object_set
                .objects
                .iter()
                .map(|object| state.object_count(*object))
                .sum();
            if best
                .as_ref()
                .is_none_or(|(best_count, _)| count < *best_count)
            {
                best = Some((
                    count,
                    ComponentAnchor {
                        kind: ComponentAnchorKind::ObjectSet {
                            objects: object_set.objects.clone(),
                        },
                        dx,
                        dy,
                    },
                ));
            }
        }
        for mark in &cell.require_mark {
            if mark.match_value != MarkValueMatch::Exact {
                continue;
            }
            let count = state
                .mark_positions(mark.object, mark.mark, mark.value)
                .len() as u32;
            if best
                .as_ref()
                .is_none_or(|(best_count, _)| count < *best_count)
            {
                let kind = if mark.object.is_empty() {
                    ComponentAnchorKind::CellMark {
                        mark: mark.mark,
                        value: mark.value,
                    }
                } else {
                    ComponentAnchorKind::SlotMark {
                        object: mark.object,
                        mark: mark.mark,
                        value: mark.value,
                    }
                };
                best = Some((count, ComponentAnchor { kind, dx, dy }));
            }
        }
    }

    best.map(|(_, anchor)| anchor)
}

fn pop_first_origin(origins: &mut BTreeSet<(u16, u16)>) -> Option<(u16, u16)> {
    let origin = origins.iter().next().copied()?;
    origins.remove(&origin);
    Some(origin)
}

fn placement_writes_within_local_frame(
    rule: &Rule,
    components: &[ComponentPlacement],
    scope: &LocalFrameScope2<'_>,
) -> bool {
    if scope.frame.is_none() {
        return true;
    }
    for write in &rule.writes {
        match write {
            WriteOp::Add {
                component, offset, ..
            }
            | WriteOp::AddObjectSet {
                component, offset, ..
            }
            | WriteOp::Remove {
                component, offset, ..
            }
            | WriteOp::RemoveObjectSet {
                component, offset, ..
            }
            | WriteOp::Replace {
                component, offset, ..
            }
            | WriteOp::SetMark {
                component, offset, ..
            }
            | WriteOp::SetObjectSetMark {
                component, offset, ..
            }
            | WriteOp::RemoveMark {
                component, offset, ..
            }
            | WriteOp::RemoveObjectSetMark {
                component, offset, ..
            } => {
                let Some((x, y)) = write_position_for_components(components, *component, offset)
                else {
                    return false;
                };
                if !scope.contains_cell(x, y) {
                    return false;
                }
            }
            WriteOp::Move {
                component,
                from_offset,
                to_offset,
                ..
            }
            | WriteOp::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                ..
            } => {
                let Some((from_x, from_y)) =
                    write_position_for_components(components, *component, from_offset)
                else {
                    return false;
                };
                let Some((to_x, to_y)) =
                    write_position_for_components(components, *component, to_offset)
                else {
                    return false;
                };
                if !scope.contains_cell(from_x, from_y) || !scope.contains_cell(to_x, to_y) {
                    return false;
                }
            }
        }
    }
    true
}

fn dirty_origin_deltas(rule: &Rule) -> Option<BTreeSet<(i32, i32)>> {
    if rule.pattern.components.len() != 1
        || rule.pattern.components[0].gap_count != 0
        || rule.writes.iter().any(|write| {
            matches!(write, WriteOp::Move { .. } | WriteOp::MoveObjectSet { .. })
                || write_component(write) != 0
                || fixed_write_offset(write).is_none()
        })
    {
        return None;
    }

    let mut deltas = BTreeSet::new();
    for write in &rule.writes {
        let (write_dx, write_dy) = fixed_write_offset(write)?;
        for cell in &rule.pattern.components[0].cells {
            let Offset::Fixed {
                dx: cell_dx,
                dy: cell_dy,
            } = cell.offset
            else {
                return None;
            };
            deltas.insert((
                i32::from(write_dx) - i32::from(cell_dx),
                i32::from(write_dy) - i32::from(cell_dy),
            ));
        }
    }
    Some(deltas)
}

fn write_component(write: &WriteOp) -> u16 {
    match write {
        WriteOp::Add { component, .. }
        | WriteOp::AddObjectSet { component, .. }
        | WriteOp::Remove { component, .. }
        | WriteOp::RemoveObjectSet { component, .. }
        | WriteOp::Move { component, .. }
        | WriteOp::MoveObjectSet { component, .. }
        | WriteOp::Replace { component, .. }
        | WriteOp::SetMark { component, .. }
        | WriteOp::SetObjectSetMark { component, .. }
        | WriteOp::RemoveObjectSetMark { component, .. }
        | WriteOp::RemoveMark { component, .. } => *component,
    }
}

fn fixed_write_offset(write: &WriteOp) -> Option<(i16, i16)> {
    match write {
        WriteOp::Add { offset, .. }
        | WriteOp::AddObjectSet { offset, .. }
        | WriteOp::Remove { offset, .. }
        | WriteOp::RemoveObjectSet { offset, .. }
        | WriteOp::Replace { offset, .. }
        | WriteOp::SetMark { offset, .. }
        | WriteOp::SetObjectSetMark { offset, .. }
        | WriteOp::RemoveObjectSetMark { offset, .. }
        | WriteOp::RemoveMark { offset, .. } => fixed_offset(offset),
        WriteOp::Move { to_offset, .. } | WriteOp::MoveObjectSet { to_offset, .. } => {
            fixed_offset(to_offset)
        }
    }
}

fn enqueue_dirty_origins(
    candidates: &mut BTreeSet<(u16, u16)>,
    origin_x: u16,
    origin_y: u16,
    dirty_origin_deltas: &BTreeSet<(i32, i32)>,
    state: &State,
) {
    for (dx, dy) in dirty_origin_deltas {
        let x = i32::from(origin_x) + dx;
        let y = i32::from(origin_y) + dy;
        if x >= 0 && y >= 0 && x < i32::from(state.width) && y < i32::from(state.height) {
            candidates.insert((y as u16, x as u16));
        }
    }
}

fn component_placement_at(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
    origin_x: u16,
    origin_y: u16,
    scope: &LocalFrameScope2<'_>,
) -> Option<ComponentPlacement> {
    if component.gap_count == 0 {
        let gaps = Vec::new();
        if let Some(object_bindings) =
            component_matches_with_gaps(game, state, component, origin_x, origin_y, &gaps, scope)
        {
            return Some(ComponentPlacement::new(
                GridCoord::new([origin_x, origin_y]),
                gaps,
                object_bindings,
            ));
        }
        return None;
    }

    let max_gap = state.width.max(state.height);
    for total_gap in 0..=max_gap.saturating_mul(component.gap_count) {
        let mut gaps = Vec::with_capacity(usize::from(component.gap_count));
        if let Some(object_bindings) = find_gap_assignment(
            game, state, component, origin_x, origin_y, max_gap, total_gap, &mut gaps, scope,
        ) {
            return Some(ComponentPlacement::new(
                GridCoord::new([origin_x, origin_y]),
                gaps,
                object_bindings,
            ));
        }
    }

    None
}

fn find_gap_assignment(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
    origin_x: u16,
    origin_y: u16,
    max_gap: u16,
    remaining_total: u16,
    gaps: &mut Vec<u16>,
    scope: &LocalFrameScope2<'_>,
) -> Option<Vec<ObjectBinding>> {
    if gaps.len() == usize::from(component.gap_count) {
        return (remaining_total == 0)
            .then(|| {
                component_matches_with_gaps(game, state, component, origin_x, origin_y, gaps, scope)
            })
            .flatten();
    }

    for gap in 0..=max_gap.min(remaining_total) {
        gaps.push(gap);
        if let Some(object_bindings) = find_gap_assignment(
            game,
            state,
            component,
            origin_x,
            origin_y,
            max_gap,
            remaining_total - gap,
            gaps,
            scope,
        ) {
            return Some(object_bindings);
        }
        gaps.pop();
    }

    None
}

fn component_matches_with_gaps(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
    origin_x: u16,
    origin_y: u16,
    gaps: &[u16],
    scope: &LocalFrameScope2<'_>,
) -> Option<Vec<ObjectBinding>> {
    let mut object_bindings = Vec::new();
    if component.cells.iter().all(|cell| {
        match_cell(
            game,
            state,
            origin_x,
            origin_y,
            gaps,
            cell,
            scope,
            &mut object_bindings,
        )
    }) {
        Some(object_bindings)
    } else {
        None
    }
}

fn match_cell(
    game: &CompiledGame,
    state: &State,
    origin_x: u16,
    origin_y: u16,
    gaps: &[u16],
    cell: &MatchCell,
    scope: &LocalFrameScope2<'_>,
    object_bindings: &mut Vec<ObjectBinding>,
) -> bool {
    let Some((dx, dy)) = resolve_offset(&cell.offset, gaps) else {
        return false;
    };
    let Some((x, y)) = offset_pos(origin_x, origin_y, dx, dy) else {
        return cell.require_null;
    };
    if x >= state.width || y >= state.height {
        return cell.require_null;
    }
    if cell.require_null {
        return false;
    }
    if !scope.contains_cell(x, y) {
        return false;
    }

    for object in &cell.require_objects {
        match state.cell_has_object_masked(x, y, *object) {
            Some(true) => {}
            Some(false) => return false,
            None => {
                if !state.has_object(game, x, y, *object) {
                    return false;
                }
            }
        }
    }

    for object_set in &cell.require_object_sets {
        let Ok(found) = state.get_layer(x, y, object_set.layer) else {
            return false;
        };
        if found.is_empty() || !object_set.objects.contains(&found) {
            return false;
        }
        if !bind_object_shared(object_bindings, object_set.binding, found) {
            return false;
        }
    }

    for mark in &cell.require_mark {
        let matched = match mark.match_value {
            MarkValueMatch::Any => state.has_mark_key(game, x, y, mark.object, mark.mark),
            MarkValueMatch::Exact => state.has_mark(game, x, y, mark.object, mark.mark, mark.value),
        };
        if !matched {
            return false;
        }
    }

    for mark in &cell.require_object_set_mark {
        let Some(object) = bound_object_shared(object_bindings, mark.binding) else {
            return false;
        };
        let matched = match mark.match_value {
            MarkValueMatch::Any => state.has_mark_key(game, x, y, object, mark.mark),
            MarkValueMatch::Exact => state.has_mark(game, x, y, object, mark.mark, mark.value),
        };
        if !matched {
            return false;
        }
    }

    for object in &cell.forbid_objects {
        match state.cell_has_object_masked(x, y, *object) {
            Some(true) => return false,
            Some(false) => {}
            None => {
                if state.has_object(game, x, y, *object) {
                    return false;
                }
            }
        }
    }

    for mark in &cell.forbid_object_set_mark {
        let Some(object) = bound_object_shared(object_bindings, mark.binding) else {
            return false;
        };
        let matched = match mark.match_value {
            MarkValueMatch::Any => state.has_mark_key(game, x, y, object, mark.mark),
            MarkValueMatch::Exact => state.has_mark(game, x, y, object, mark.mark, mark.value),
        };
        if matched {
            return false;
        }
    }

    for mark in &cell.forbid_mark {
        let matched = match mark.match_value {
            MarkValueMatch::Any => state.has_mark_key(game, x, y, mark.object, mark.mark),
            MarkValueMatch::Exact => state.has_mark(game, x, y, mark.object, mark.mark, mark.value),
        };
        if matched {
            return false;
        }
    }

    true
}

fn build_patch(rule: &Rule, placement: &MatchPlacement) -> TransitionResult<Patch> {
    let mut patch = CorePatch::new();

    for write in &rule.writes {
        match write {
            WriteOp::Add {
                component,
                offset,
                object,
            } => {
                patch.push(CorePatchOp::Add {
                    position: write_grid_position(placement, *component, offset)?,
                    object: *object,
                });
            }
            WriteOp::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                patch.push(CorePatchOp::Add {
                    position: write_grid_position(placement, *component, offset)?,
                    object,
                });
            }
            WriteOp::Remove {
                component,
                offset,
                object,
            } => {
                patch.push(CorePatchOp::Remove {
                    position: write_grid_position(placement, *component, offset)?,
                    object: *object,
                });
            }
            WriteOp::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                patch.push(CorePatchOp::Remove {
                    position: write_grid_position(placement, *component, offset)?,
                    object,
                });
            }
            WriteOp::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                patch.push(CorePatchOp::Move {
                    from: write_grid_position(placement, *component, from_offset)?,
                    to: write_grid_position(placement, *component, to_offset)?,
                    object: *object,
                });
            }
            WriteOp::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                patch.push(CorePatchOp::Move {
                    from: write_grid_position(placement, *component, from_offset)?,
                    to: write_grid_position(placement, *component, to_offset)?,
                    object,
                });
            }
            WriteOp::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                patch.push(CorePatchOp::Replace {
                    position: write_grid_position(placement, *component, offset)?,
                    remove: *remove,
                    add: *add,
                });
            }
            WriteOp::SetMark {
                component,
                offset,
                object,
                mark,
                value,
            } => {
                patch.push(CorePatchOp::SetMark {
                    position: write_grid_position(placement, *component, offset)?,
                    object: *object,
                    mark: *mark,
                    value: *value,
                });
            }
            WriteOp::SetObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                patch.push(CorePatchOp::SetMark {
                    position: write_grid_position(placement, *component, offset)?,
                    object,
                    mark: *mark,
                    value: *value,
                });
            }
            WriteOp::RemoveMark {
                component,
                offset,
                object,
                mark,
                value,
                match_value,
            } => {
                patch.push(CorePatchOp::RemoveMark {
                    position: write_grid_position(placement, *component, offset)?,
                    object: *object,
                    mark: *mark,
                    value: *value,
                    match_value: *match_value,
                });
            }
            WriteOp::RemoveObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
                match_value,
            } => {
                let object = placement_object_binding(placement, *binding)?;
                patch.push(CorePatchOp::RemoveMark {
                    position: write_grid_position(placement, *component, offset)?,
                    object,
                    mark: *mark,
                    value: *value,
                    match_value: *match_value,
                });
            }
        }
    }
    for effect in &rule.effects {
        match effect {
            Effect::Cancel => {}
            Effect::Win
            | Effect::Restart
            | Effect::NextLevel
            | Effect::Again
            | Effect::Checkpoint
            | Effect::ClearCheckpoint => {}
            Effect::UpdateVariable {
                variable,
                op,
                value,
            } => {
                patch.push(CorePatchOp::UpdateVariable {
                    variable: *variable,
                    op: *op,
                    value: *value,
                });
            }
        }
    }

    Ok(Patch::from_core(patch))
}

fn placement_object_binding(
    placement: &MatchPlacement,
    binding: u16,
) -> TransitionResult<ObjectId> {
    placement_object_binding_shared(placement, binding).ok_or(TransitionError::OffsetOutOfBounds)
}

fn write_grid_position(
    placement: &MatchPlacement,
    component: u16,
    offset: &Offset,
) -> TransitionResult<GridCoord<2>> {
    write_position_shared(placement, component, offset, resolve_grid_offset, || {
        TransitionError::OffsetOutOfBounds
    })
}

fn write_position_for_components(
    components: &[ComponentPlacement],
    component: u16,
    offset: &Offset,
) -> Option<(u16, u16)> {
    write_position_for_components_shared(components, component, offset, resolve_grid_offset)
        .map(grid_coord_to_xy)
}

fn resolve_grid_offset(offset: &Offset, gaps: &[u16]) -> Option<GridOffset<2>> {
    let (dx, dy) = resolve_offset(offset, gaps)?;
    Some(GridOffset::new([dx, dy]))
}

fn grid_coord_to_xy(coord: GridCoord<2>) -> (u16, u16) {
    let [x, y] = coord.axes();
    (x, y)
}

fn fixed_offset(offset: &Offset) -> Option<(i16, i16)> {
    match *offset {
        Offset::Fixed { dx, dy } => Some((dx, dy)),
        Offset::Variable { .. } => None,
    }
}

fn resolve_offset(offset: &Offset, gaps: &[u16]) -> Option<(i16, i16)> {
    match offset {
        Offset::Fixed { dx, dy } => Some((*dx, *dy)),
        Offset::Variable {
            base_dx,
            base_dy,
            gap_terms,
        } => {
            let mut dx = i32::from(*base_dx);
            let mut dy = i32::from(*base_dy);
            for term in gap_terms {
                let gap = *gaps.get(usize::from(term.gap_index))?;
                dx += i32::from(term.dx) * i32::from(gap);
                dy += i32::from(term.dy) * i32::from(gap);
            }
            if dx < i32::from(i16::MIN)
                || dx > i32::from(i16::MAX)
                || dy < i32::from(i16::MIN)
                || dy > i32::from(i16::MAX)
            {
                return None;
            }
            Some((dx as i16, dy as i16))
        }
    }
}

#[inline]
fn offset_pos(x: u16, y: u16, dx: i16, dy: i16) -> Option<(u16, u16)> {
    let next_x = i32::from(x) + i32::from(dx);
    let next_y = i32::from(y) + i32::from(dy);
    if next_x < 0 || next_y < 0 || next_x > i32::from(u16::MAX) || next_y > i32::from(u16::MAX) {
        return None;
    }
    Some((next_x as u16, next_y as u16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled_game::{
        Guard, MarkDef, MarkKind, MarkPattern, MatchCell, ObjectDef, Offset, Pattern,
        PatternComponent, Rule, RuleApplication, VariableUpdateOp, WriteOp,
    };
    use crate::ids::{InputId, LayerId, ObjectId, RuleId, VariableId};

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const MARK: MarkId = MarkId(1);
    const RIGHT: InputId = InputId(1);

    fn fixed(dx: i16, dy: i16) -> Offset {
        Offset::Fixed { dx, dy }
    }

    fn cell(
        dx: i16,
        dy: i16,
        require_objects: Vec<ObjectId>,
        forbid_objects: Vec<ObjectId>,
    ) -> MatchCell {
        MatchCell {
            offset: fixed(dx, dy),
            require_null: false,
            require_objects,
            require_object_sets: Vec::new(),
            forbid_objects,
            require_mark: Vec::new(),
            require_object_set_mark: Vec::new(),
            forbid_mark: Vec::new(),
            forbid_object_set_mark: Vec::new(),
        }
    }

    fn mark_cell(
        dx: i16,
        dy: i16,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    ) -> MatchCell {
        MatchCell {
            offset: fixed(dx, dy),
            require_null: false,
            require_objects: Vec::new(),
            require_object_sets: Vec::new(),
            forbid_objects: Vec::new(),
            require_mark: vec![MarkPattern {
                object,
                mark,
                value,
                match_value: MarkValueMatch::Exact,
            }],
            require_object_set_mark: Vec::new(),
            forbid_mark: Vec::new(),
            forbid_object_set_mark: Vec::new(),
        }
    }

    fn pattern(cells: Vec<MatchCell>) -> Pattern {
        Pattern {
            components: vec![PatternComponent {
                cells,
                gap_count: 0,
            }],
        }
    }

    fn add(dx: i16, dy: i16, object: ObjectId) -> WriteOp {
        WriteOp::Add {
            component: 0,
            offset: fixed(dx, dy),
            object,
        }
    }

    fn remove(dx: i16, dy: i16, object: ObjectId) -> WriteOp {
        WriteOp::Remove {
            component: 0,
            offset: fixed(dx, dy),
            object,
        }
    }

    fn replace(dx: i16, dy: i16, remove: ObjectId, add: ObjectId) -> WriteOp {
        WriteOp::Replace {
            component: 0,
            offset: fixed(dx, dy),
            remove,
            add,
        }
    }

    fn variable_rule(
        id: u16,
        guards: Vec<Guard>,
        effects: Vec<Effect>,
        application: RuleApplication,
    ) -> Rule {
        Rule {
            id: RuleId(id),
            guards,
            application,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects,
        }
    }

    fn set_variable(variable: u16, value: i64) -> Effect {
        Effect::UpdateVariable {
            variable: VariableId(variable),
            op: VariableUpdateOp::Set,
            value,
        }
    }

    fn add_variable(variable: u16, value: i64) -> Effect {
        Effect::UpdateVariable {
            variable: VariableId(variable),
            op: VariableUpdateOp::Add,
            value,
        }
    }

    fn push_game() -> CompiledGame {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: WALL,
                layer_id: LayerId(1),
            },
        ];

        let push_right = Rule {
            id: RuleId(1),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::Once,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![BOX], vec![]),
                cell(2, 0, vec![], vec![PLAYER, BOX, WALL]),
            ]),
            writes: vec![
                remove(0, 0, PLAYER),
                remove(1, 0, BOX),
                add(1, 0, PLAYER),
                add(2, 0, BOX),
            ],
            effects: vec![],
        };

        CompiledGame::new(2, objects, vec![push_right])
    }

    #[test]
    fn random_rule_is_deterministic_for_same_state() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: WALL,
                layer_id: LayerId(2),
            },
        ];
        let random_player_to_box = Rule {
            id: RuleId(7),
            guards: Vec::new(),
            application: RuleApplication::Random,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game =
            CompiledGame::new_with_program(3, objects, vec![RuleStep::Rule(random_player_to_box)]);
        let mut plain = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        plain.place_object(&game, 0, 0, PLAYER).unwrap();
        plain.place_object(&game, 2, 0, PLAYER).unwrap();

        let first = transition_state(&game, &plain, RIGHT).unwrap();
        let repeated = transition_state(&game, &plain, RIGHT).unwrap();

        assert_eq!(first, repeated);
        assert_eq!(first.object_count(BOX), 1);
        assert_eq!(first.object_count(PLAYER), 1);
    }

    #[test]
    fn random_block_applies_one_firing_step() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
        ];
        let left_player_to_box = Rule {
            id: RuleId(10),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let right_player_to_box = Rule {
            id: RuleId(11),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(1, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(1, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new_with_program(
            2,
            objects,
            vec![RuleStep::Block {
                application: RuleApplication::Random,
                stop_condition: None,
                steps: vec![
                    RuleStep::Rule(left_player_to_box),
                    RuleStep::Rule(right_player_to_box),
                ],
            }],
        );
        let mut state = State::empty(2, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.object_count(BOX), 1);
        assert_eq!(next.object_count(PLAYER), 1);
    }

    fn mark_anchor_game() -> CompiledGame {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(2),
            },
        ];
        let mark = vec![MarkDef {
            id: MARK,
            kind: MarkKind::Flag,
            values: Vec::new(),
        }];
        CompiledGame::new_with_mark_condition_defs_and_program(3, objects, mark, Vec::new(), vec![])
    }

    #[test]
    fn mark_position_cache_tracks_slot_mark_moves_and_clears() {
        let game = mark_anchor_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();

        state.set_mark_unchecked(1, 0, LayerId(2), MARK, Some(7));
        assert_eq!(
            state
                .mark_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(1, 0)]
        );

        let mark = state.take_slot_for_move_unchecked(1, 0, LayerId(2));
        state.place_moved_slot_unchecked(3, 0, LayerId(2), BOX, mark);
        assert_eq!(
            state
                .mark_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(3, 0)]
        );

        state.remove_mark_unchecked(3, 0, LayerId(2), MARK, Some(7));
        assert!(state.mark_positions(BOX, MARK, Some(7)).is_empty());

        state.set_mark_unchecked(3, 0, LayerId(2), MARK, Some(9));
        state.clear_mark();
        assert!(state.mark_positions(BOX, MARK, Some(9)).is_empty());
    }

    #[test]
    fn component_candidates_anchor_on_rarest_required_exact_mark() {
        let game = mark_anchor_game();
        let mut state = State::empty(5, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();
        state.place_object(&game, 3, 0, PLAYER).unwrap();
        state.place_object(&game, 2, 0, BOX).unwrap();
        state.set_mark_unchecked(2, 0, LayerId(2), MARK, Some(1));

        let component = PatternComponent {
            cells: vec![
                cell(0, 0, vec![PLAYER], Vec::new()),
                mark_cell(1, 0, BOX, MARK, Some(1)),
            ],
            gap_count: 0,
        };
        let scope = LocalFrameScope2::new(&state, None);

        let origins = component_candidate_origins(&game, &state, &component, &scope);

        assert_eq!(origins, vec![(1, 0)]);
    }

    #[test]
    fn pushes_box_right() {
        let game = push_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(1)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(1, 0, LayerId(1)).unwrap(), PLAYER);
        assert_eq!(next.get_layer(2, 0, LayerId(1)).unwrap(), BOX);
        assert_eq!(next.object_count(PLAYER), 1);
        assert_eq!(next.object_count(BOX), 1);
        assert_eq!(
            next.object_positions(PLAYER)
                .iter()
                .filter_map(|slot| next.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(1, 0)]
        );
        assert_eq!(
            next.object_positions(BOX)
                .iter()
                .filter_map(|slot| next.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(2, 0)]
        );
    }

    #[test]
    fn blocked_push_does_not_move() {
        let game = push_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();
        state.place_object(&game, 2, 0, WALL).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn adding_same_object_to_same_layer_is_idempotent() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let rule = Rule {
            id: RuleId(1),
            guards: vec![],
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, PLAYER)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![rule]);
        let mut state = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert!(next.has_object(&game, 0, 0, PLAYER));
        assert_eq!(next.object_count(PLAYER), 1);
    }

    #[test]
    fn overlapping_components_deduplicate_identical_removes() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let component = PatternComponent {
            cells: vec![cell(0, 0, vec![PLAYER], vec![])],
            gap_count: 0,
        };
        let rule = Rule {
            id: RuleId(1),
            guards: vec![],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: vec![component.clone(), component],
            },
            writes: vec![
                WriteOp::Remove {
                    component: 0,
                    offset: fixed(0, 0),
                    object: PLAYER,
                },
                WriteOp::Remove {
                    component: 1,
                    offset: fixed(0, 0),
                    object: PLAYER,
                },
            ],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![rule]);
        let mut state = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert!(!next.has_object(&game, 0, 0, PLAYER));
    }

    #[test]
    fn conflicting_overlapping_component_writes_skip_the_placement() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: WALL,
                layer_id: LayerId(1),
            },
        ];
        let component = PatternComponent {
            cells: vec![cell(0, 0, vec![PLAYER], vec![])],
            gap_count: 0,
        };
        let rule = Rule {
            id: RuleId(1),
            guards: vec![],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: vec![component.clone(), component],
            },
            writes: vec![
                WriteOp::Replace {
                    component: 0,
                    offset: fixed(0, 0),
                    remove: PLAYER,
                    add: BOX,
                },
                WriteOp::Replace {
                    component: 1,
                    offset: fixed(0, 0),
                    remove: PLAYER,
                    add: WALL,
                },
            ],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![rule]);
        let mut state = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn trace_reports_fired_rule_and_patch() {
        let game = push_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.fired_rules, vec![RuleId(1)]);
        assert_eq!(trace.patches.len(), 1);
        assert_eq!(trace.patches[0].ops().len(), 4);
    }

    #[test]
    fn cancel_effect_reverts_entire_transition() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
        ];
        let move_then_cancel = Rule {
            id: RuleId(3),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::Once,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![], vec![PLAYER, BOX]),
            ]),
            writes: vec![remove(0, 0, PLAYER), add(1, 0, PLAYER)],
            effects: vec![Effect::Cancel],
        };
        let game = CompiledGame::new(2, objects, vec![move_then_cancel]);
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.next_state, state);
        assert_eq!(trace.fired_rules, vec![RuleId(3)]);
        assert_eq!(trace.patches.len(), 1);
    }

    #[test]
    fn until_stable_reapplies_rule_until_no_dirty_origin_matches() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let slide_right = Rule {
            id: RuleId(1),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::UntilStable,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![], vec![PLAYER]),
            ]),
            writes: vec![remove(0, 0, PLAYER), add(1, 0, PLAYER)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![slide_right]);
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert!(trace.next_state.has_object(&game, 3, 0, PLAYER));
        assert_eq!(trace.fired_rules, vec![RuleId(1), RuleId(1), RuleId(1)]);
        assert_eq!(trace.patches.len(), 3);
    }

    #[test]
    fn once_all_applies_to_each_initial_match_once() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let slide_right = Rule {
            id: RuleId(1),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::OnceAll,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![], vec![PLAYER]),
            ]),
            writes: vec![remove(0, 0, PLAYER), add(1, 0, PLAYER)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![slide_right]);
        let mut state = State::empty(5, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 3, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert!(trace.next_state.has_object(&game, 1, 0, PLAYER));
        assert!(trace.next_state.has_object(&game, 4, 0, PLAYER));
        assert!(!trace.next_state.has_object(&game, 2, 0, PLAYER));
        assert_eq!(trace.fired_rules, vec![RuleId(1), RuleId(1)]);
    }

    #[test]
    fn once_all_skips_initial_matches_that_have_been_invalidated_during_sweep() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
        ];
        let consume_pair = Rule {
            id: RuleId(8),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::OnceAll,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![PLAYER], vec![]),
            ]),
            writes: vec![replace(0, 0, PLAYER, BOX), remove(1, 0, PLAYER)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![consume_pair]);
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();
        state.place_object(&game, 2, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert!(trace.next_state.has_object(&game, 0, 0, BOX));
        assert_eq!(
            trace.next_state.get_layer(1, 0, LayerId(1)).unwrap(),
            ObjectId::EMPTY
        );
        assert!(trace.next_state.has_object(&game, 2, 0, PLAYER));
        assert_eq!(trace.fired_rules, vec![RuleId(8)]);
        assert_eq!(trace.patches.len(), 1);
    }

    #[test]
    fn once_all_does_not_chain_into_matches_created_during_the_same_sweep() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let slide_right = Rule {
            id: RuleId(10),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::OnceAll,
            pattern: pattern(vec![
                cell(0, 0, vec![PLAYER], vec![]),
                cell(1, 0, vec![], vec![PLAYER]),
            ]),
            writes: vec![remove(0, 0, PLAYER), add(1, 0, PLAYER)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![slide_right]);
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert!(!trace.next_state.has_object(&game, 0, 0, PLAYER));
        assert!(trace.next_state.has_object(&game, 1, 0, PLAYER));
        assert!(!trace.next_state.has_object(&game, 2, 0, PLAYER));
        assert_eq!(trace.fired_rules, vec![RuleId(10)]);
    }

    #[test]
    fn once_per_level_fires_only_once_across_transitions() {
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: BOX,
                layer_id: LayerId(1),
            },
        ];
        let player_to_box = Rule {
            id: RuleId(9),
            guards: vec![Guard::InputIs(RIGHT)],
            application: RuleApplication::OncePerLevel,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: vec![],
        };
        let game = CompiledGame::new(2, objects, vec![player_to_box]);
        let mut state = State::empty(2, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();

        let first = transition_state(&game, &state, RIGHT).unwrap();
        let second = transition_state(&game, &first, RIGHT).unwrap();

        assert!(first.has_object(&game, 0, 0, BOX));
        assert!(first.level_rule_has_fired(RuleId(9)));
        assert_eq!(second, first);
    }

    #[test]
    fn until_stable_block_skips_when_state_cycles() {
        let zero_to_one = Rule {
            id: RuleId(7),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 0,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            }],
        };
        let one_to_two = Rule {
            id: RuleId(8),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 1,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 2,
            }],
        };
        let two_to_zero = Rule {
            id: RuleId(9),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 2,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 0,
            }],
        };
        let game = CompiledGame::new_with_program(
            1,
            Vec::new(),
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![
                    RuleStep::Rule(one_to_two),
                    RuleStep::Rule(zero_to_one),
                    RuleStep::Rule(two_to_zero),
                ],
            }],
        );
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_block_keeps_revisited_non_initial_state() {
        let value = VariableId(0);
        let changed = VariableId(1);
        let reset_changed = variable_rule(
            20,
            Vec::new(),
            vec![set_variable(1, 0)],
            RuleApplication::Once,
        );
        let two_to_one = variable_rule(
            21,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 2,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 1), set_variable(1, 1)],
            RuleApplication::Once,
        );
        let one_to_two = variable_rule(
            22,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 1,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 2), set_variable(1, 1)],
            RuleApplication::Once,
        );
        let zero_to_one = variable_rule(
            23,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 0,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 1), set_variable(1, 1)],
            RuleApplication::Once,
        );
        let game = CompiledGame::new_with_program(
            1,
            Vec::new(),
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![
                    RuleStep::Rule(reset_changed),
                    RuleStep::Rule(two_to_one),
                    RuleStep::Rule(one_to_two),
                    RuleStep::Rule(zero_to_one),
                ],
            }],
        );
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0, 0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.variable_value(value), Some(1));
        assert_eq!(next.variable_value(changed), Some(1));
    }

    #[test]
    fn until_stable_block_budget_keeps_last_state_for_divergent_updates() {
        let counter = VariableId(0);
        let increment = variable_rule(
            24,
            Vec::new(),
            vec![add_variable(0, 1)],
            RuleApplication::Once,
        );
        let game = CompiledGame::new_with_program(
            1,
            Vec::new(),
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![RuleStep::Rule(increment)],
            }],
        );
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(
            next.variable_value(counter),
            Some(UNTIL_STABLE_REPEAT_LIMIT as i64)
        );
    }

    #[test]
    fn repeated_rule_group_is_stable_when_one_sweep_returns_to_start() {
        let game = CompiledGame::new_with_program(
            1,
            Vec::new(),
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![
                    RuleStep::Rule(Rule {
                        id: RuleId(11),
                        guards: Vec::new(),
                        application: RuleApplication::Once,
                        pattern: Pattern {
                            components: Vec::new(),
                        },
                        writes: Vec::new(),
                        effects: vec![Effect::UpdateVariable {
                            variable: VariableId(0),
                            op: VariableUpdateOp::Set,
                            value: 1,
                        }],
                    }),
                    RuleStep::Rule(Rule {
                        id: RuleId(12),
                        guards: Vec::new(),
                        application: RuleApplication::Once,
                        pattern: Pattern {
                            components: Vec::new(),
                        },
                        writes: Vec::new(),
                        effects: vec![Effect::UpdateVariable {
                            variable: VariableId(0),
                            op: VariableUpdateOp::Set,
                            value: 0,
                        }],
                    }),
                ],
            }],
        );
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_rule_treats_idempotent_variable_update_as_stable() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let rule = Rule {
            id: RuleId(15),
            guards: Vec::new(),
            application: RuleApplication::UntilStable,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            }],
        };
        let game = CompiledGame::new_with_program(2, objects, vec![RuleStep::Rule(rule)]);
        let mut state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.variable_value(VariableId(0)), Some(1));
        assert!(next.has_object(&game, 0, 0, PLAYER));
    }

    #[test]
    fn dirty_origin_deltas_cover_rectangular_footprints() {
        let rule = Rule {
            id: RuleId(1),
            guards: vec![],
            application: RuleApplication::UntilStable,
            pattern: pattern(vec![
                cell(0, 0, vec![], vec![]),
                cell(1, 0, vec![], vec![]),
                cell(0, 1, vec![], vec![]),
                cell(1, 1, vec![], vec![]),
                cell(0, 2, vec![], vec![]),
                cell(1, 2, vec![], vec![]),
            ]),
            writes: vec![add(1, 2, PLAYER)],
            effects: vec![],
        };

        let deltas = dirty_origin_deltas(&rule).unwrap();

        assert_eq!(deltas.len(), 6);
        assert!(deltas.contains(&(1, 2)));
        assert!(deltas.contains(&(0, 2)));
        assert!(deltas.contains(&(1, 1)));
        assert!(deltas.contains(&(0, 1)));
        assert!(deltas.contains(&(1, 0)));
        assert!(deltas.contains(&(0, 0)));
    }
}
