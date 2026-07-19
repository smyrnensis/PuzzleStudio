use crate::{
    ConditionId, GridCompiledGame, GridConditionValueKind, GridExecutableProgram, GridGuard,
    GridPatch, GridPatchError, GridPattern, GridPatternComponent, GridRule, GridRuleCondition,
    GridSize, GridState, GridWriteOp, InputId, MarkId, ObjectId, PatchOp, RuleId,
};
use puzzle_kernel::{
    ComparisonOp, ComponentPlacement, FnvBuilder, GridCoord, GridOffset as CoordOffset, LocalFrame,
    MarkValueMatch, MatchPlacement, ObjectBinding, ProgramApplyOutcome, ProgramBackend, RuleFiring,
    TransitionOutcome as KernelTransitionOutcome, bind_object,
    bound_object as bound_object_in_bindings,
    collect_component_placements as collect_component_placements_shared,
    complete_component_placements as complete_component_placements_shared, fnv_mix,
    placement_object_binding, write_position as write_position_shared,
};

type MarkPattern = puzzle_kernel::RuleMarkPattern<ObjectId, MarkId>;
type ObjectSetMarkPattern = puzzle_kernel::ObjectSetMarkPattern<MarkId>;
use puzzle_kernel::{RuleApplication, RuleEffect};

pub(crate) const UNTIL_STABLE_REPEAT_LIMIT: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridTransitionError<const D: usize> {
    Patch(GridPatchError<D>),
    OffsetOutOfBounds,
    RepeatUntilNoProgress,
    InvalidProgramContinuation,
    InvalidCommand(String),
    UnboundObjectSet { binding: u16 },
}

impl<const D: usize> From<GridPatchError<D>> for GridTransitionError<D> {
    fn from(value: GridPatchError<D>) -> Self {
        Self::Patch(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionCommand {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
}

pub type ProgramContinuation = puzzle_kernel::ProgramContinuation;

pub use puzzle_kernel::flattened_program_rules as flattened_rules;

pub type GridRuleFiring<const D: usize> = RuleFiring<RuleId, GridPatch<D>>;
pub type GridRuleFiringSummary = RuleFiring<RuleId, ()>;

#[derive(Clone, Default)]
struct GridExecutionEffects {
    commands: Vec<TransitionCommand>,
    cancelled: bool,
}

impl GridExecutionEffects {
    fn apply<const D: usize>(&mut self, rule: &GridRule<D>) {
        if rule_cancels(rule) {
            self.cancelled = true;
            return;
        }
        for effect in &rule.effects {
            match effect {
                RuleEffect::ObserveMatch => {}
                RuleEffect::Cancel => unreachable!("cancel handled before command emission"),
                RuleEffect::Win => self.commands.push(TransitionCommand::Win),
                RuleEffect::Restart => self.commands.push(TransitionCommand::Restart),
                RuleEffect::NextLevel => self.commands.push(TransitionCommand::NextLevel),
                RuleEffect::Again => self.commands.push(TransitionCommand::Again),
                RuleEffect::Checkpoint => self.commands.push(TransitionCommand::Checkpoint),
                RuleEffect::ClearCheckpoint => {
                    self.commands.push(TransitionCommand::ClearCheckpoint)
                }
                RuleEffect::UpdateVariable { .. } => {}
            }
        }
    }
}

#[derive(Clone)]
enum GridFiringCollector<const D: usize> {
    Discard,
    Summary(Vec<GridRuleFiringSummary>),
    Detailed(Vec<GridRuleFiring<D>>),
}

impl<const D: usize> Default for GridFiringCollector<D> {
    fn default() -> Self {
        Self::Detailed(Vec::new())
    }
}

impl<const D: usize> GridFiringCollector<D> {
    fn summary() -> Self {
        Self::Summary(Vec::new())
    }

    fn record(&mut self, rule: RuleId, patch: GridPatch<D>, progressed: bool, observable: bool) {
        match self {
            Self::Discard => {}
            Self::Summary(firings) => firings.push(GridRuleFiringSummary {
                rule,
                patch: (),
                progressed,
                observable,
            }),
            Self::Detailed(firings) => firings.push(GridRuleFiring::<D> {
                rule,
                patch,
                progressed,
                observable,
            }),
        }
    }

    fn detailed(&self) -> Option<&[GridRuleFiring<D>]> {
        match self {
            Self::Detailed(firings) => Some(firings),
            Self::Discard | Self::Summary(_) => None,
        }
    }

    fn into_detailed(self) -> Vec<GridRuleFiring<D>> {
        match self {
            Self::Detailed(firings) => firings,
            Self::Discard | Self::Summary(_) => {
                unreachable!("detailed firing collection was not requested")
            }
        }
    }

    fn into_summary(self) -> Vec<GridRuleFiringSummary> {
        match self {
            Self::Summary(firings) => firings,
            Self::Discard | Self::Detailed(_) => {
                unreachable!("summary firing collection was not requested")
            }
        }
    }
}

#[derive(Clone, Copy)]
enum FiringCollection {
    Discard,
    Summary,
    Detailed,
}

#[derive(Clone, Default)]
struct GridProgramContext<const D: usize> {
    effects: GridExecutionEffects,
    firings: GridFiringCollector<D>,
}

impl<const D: usize> GridProgramContext<D> {
    fn collecting(collection: FiringCollection) -> Self {
        Self {
            effects: GridExecutionEffects::default(),
            firings: match collection {
                FiringCollection::Discard => GridFiringCollector::Discard,
                FiringCollection::Summary => GridFiringCollector::summary(),
                FiringCollection::Detailed => GridFiringCollector::default(),
            },
        }
    }

    fn commit(
        &mut self,
        rule: &GridRule<D>,
        patch: GridPatch<D>,
        progressed: bool,
    ) -> ProgramApplyOutcome {
        let outcome = ProgramApplyOutcome {
            fired: true,
            progressed,
            observable: rule_has_observable_effect(rule),
            cancelled: rule_cancels(rule),
        };
        self.effects.apply(rule);
        self.firings
            .record(rule.id, patch, outcome.progressed, outcome.observable);
        outcome
    }
}

struct GridRuleTransition<State> {
    next_state: Option<State>,
    outcome: ProgramApplyOutcome,
}

impl<State> GridRuleTransition<State> {
    fn idle() -> Self {
        Self {
            next_state: None,
            outcome: ProgramApplyOutcome::default(),
        }
    }
}

pub type GridTransitionOutcome<const D: usize, Size> = KernelTransitionOutcome<
    Option<InputId>,
    GridState<D, Size>,
    TransitionCommand,
    RuleId,
    GridPatch<D>,
>;
pub type GridTransitionSummaryOutcome<const D: usize, Size> =
    KernelTransitionOutcome<Option<InputId>, GridState<D, Size>, TransitionCommand, RuleId, ()>;

struct GridInternalTransitionOutcome<const D: usize, Size: GridSize<D>> {
    input: Option<InputId>,
    next_state: GridState<D, Size>,
    progressed: bool,
    observable: bool,
    cancelled: bool,
    commands: Vec<TransitionCommand>,
    firings: GridFiringCollector<D>,
}

pub fn transition_state<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: InputId,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_program(game, state, game.executable_program(), input)
}

pub fn transition_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: InputId,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_outcome(game, state, game.executable_program(), input)
}

pub fn transition_solver_state<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: InputId,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_state(game, state, input)
}

pub fn transition_solver_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: InputId,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_outcome(game, state, input)
}

pub fn transition_trace<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: InputId,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_outcome(game, state, input)
}

pub struct GridProgramBoundarySnapshot<'a, const D: usize, Size: GridSize<D>> {
    pub input: Option<InputId>,
    pub next_state: &'a GridState<D, Size>,
    pub cancelled: bool,
    pub commands: &'a [TransitionCommand],
    pub firings: &'a [GridRuleFiring<D>],
}

pub struct GridProgramSegmentTrace<const D: usize, Size: GridSize<D>> {
    pub trace: GridTransitionOutcome<D, Size>,
    pub remaining_program: Option<puzzle_kernel::ProgramContinuation>,
}

pub fn transition_program_segment_trace<
    const D: usize,
    Size: GridSize<D>,
    Stop: FnMut(GridProgramBoundarySnapshot<'_, D, Size>) -> bool,
>(
    game: &GridCompiledGame<D>,
    program: &GridExecutableProgram<D>,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    mut should_stop: Stop,
) -> Result<GridProgramSegmentTrace<D, Size>, GridTransitionError<D>> {
    let mut original = state.clone();
    original.clear_mark();
    let mut current = original.clone();
    let mut context = GridProgramContext::<D>::default();
    let segment = {
        let mut backend = GridProgramBackend {
            game,
            input,
            context: &mut context,
        };
        puzzle_kernel::execute_program_segment(
            &mut backend,
            &mut current,
            program,
            local_frame,
            UNTIL_STABLE_REPEAT_LIMIT,
            &mut |state, backend| {
                should_stop(GridProgramBoundarySnapshot {
                    input,
                    next_state: state,
                    cancelled: backend.context.effects.cancelled,
                    commands: &backend.context.effects.commands,
                    firings: backend
                        .context
                        .firings
                        .detailed()
                        .expect("program segment trace always collects firings"),
                })
            },
        )?
    };
    finish_program_segment(input, original, current, context, segment)
}

pub fn transition_program_continuation_segment_trace<
    const D: usize,
    Size: GridSize<D>,
    Stop: FnMut(GridProgramBoundarySnapshot<'_, D, Size>) -> bool,
>(
    game: &GridCompiledGame<D>,
    program: &GridExecutableProgram<D>,
    continuation: &puzzle_kernel::ProgramContinuation,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    mut should_stop: Stop,
) -> Result<GridProgramSegmentTrace<D, Size>, GridTransitionError<D>> {
    let mut original = state.clone();
    original.clear_mark();
    let mut current = original.clone();
    let mut context = GridProgramContext::<D>::default();
    let segment = {
        let mut backend = GridProgramBackend {
            game,
            input,
            context: &mut context,
        };
        puzzle_kernel::resume_program_segment(
            &mut backend,
            &mut current,
            program,
            continuation,
            local_frame,
            UNTIL_STABLE_REPEAT_LIMIT,
            &mut |state, backend| {
                should_stop(GridProgramBoundarySnapshot {
                    input,
                    next_state: state,
                    cancelled: backend.context.effects.cancelled,
                    commands: &backend.context.effects.commands,
                    firings: backend
                        .context
                        .firings
                        .detailed()
                        .expect("program segment trace always collects firings"),
                })
            },
        )?
    };
    finish_program_segment(input, original, current, context, segment)
}

fn finish_program_segment<const D: usize, Size: GridSize<D>>(
    input: Option<InputId>,
    original: GridState<D, Size>,
    mut current: GridState<D, Size>,
    context: GridProgramContext<D>,
    segment: puzzle_kernel::ProgramSegment,
) -> Result<GridProgramSegmentTrace<D, Size>, GridTransitionError<D>> {
    let cancelled = segment.outcome.cancelled;
    let (next_state, commands) = if cancelled {
        (original, Vec::new())
    } else {
        current.clear_mark();
        (current, context.effects.commands)
    };
    Ok(GridProgramSegmentTrace {
        trace: KernelTransitionOutcome {
            input,
            next_state,
            progressed: !cancelled && segment.outcome.progressed,
            observable: segment.outcome.observable,
            cancelled,
            commands,
            firings: context.firings.into_detailed(),
        },
        remaining_program: segment.continuation,
    })
}

pub fn transition_once<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut context = GridProgramContext::<D>::collecting(FiringCollection::Discard);
    let mut next = transition_rule_once(game, &scoped, rule, None, None, &mut context)?
        .next_state
        .unwrap_or(scoped);
    next.clear_mark();
    Ok(next)
}

pub fn transition_once_with_input<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: InputId,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut context = GridProgramContext::<D>::collecting(FiringCollection::Discard);
    let mut next = transition_rule_once(game, &scoped, rule, Some(input), None, &mut context)?
        .next_state
        .unwrap_or(scoped);
    next.clear_mark();
    Ok(next)
}

pub fn transition_program<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: InputId,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        Some(input),
        None,
        FiringCollection::Discard,
    )
    .map(|outcome| outcome.next_state)
}

pub fn transition_program_with_local_frame<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: InputId,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        Some(input),
        local_frame,
        FiringCollection::Discard,
    )
    .map(|outcome| outcome.next_state)
}

pub fn transition_program_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: InputId,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_outcome_with_local_frame(game, state, program, input, None)
}

pub fn transition_program_sequence_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    programs: &[&GridExecutableProgram<D>],
    input: InputId,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_sequence_inner(
        game,
        state,
        programs,
        Some(input),
        None,
        FiringCollection::Detailed,
    )
    .map(detailed_outcome)
}

pub fn transition_program_outcome_with_local_frame<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: InputId,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        Some(input),
        local_frame,
        FiringCollection::Detailed,
    )
    .map(detailed_outcome)
}

pub fn transition_program_summary_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: InputId,
) -> Result<GridTransitionSummaryOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        Some(input),
        None,
        FiringCollection::Summary,
    )
    .map(summary_outcome)
}

pub fn transition_program_sequence_summary_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    programs: &[&GridExecutableProgram<D>],
    input: InputId,
) -> Result<GridTransitionSummaryOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_sequence_inner(
        game,
        state,
        programs,
        Some(input),
        None,
        FiringCollection::Summary,
    )
    .map(summary_outcome)
}

fn transition_program_inner<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    firing_collection: FiringCollection,
) -> Result<GridInternalTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_sequence_inner(
        game,
        state,
        &[program],
        input,
        local_frame,
        firing_collection,
    )
}

fn transition_program_sequence_inner<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    programs: &[&GridExecutableProgram<D>],
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    firing_collection: FiringCollection,
) -> Result<GridInternalTransitionOutcome<D, Size>, GridTransitionError<D>> {
    let mut original = state.clone();
    original.clear_mark();
    let mut context = GridProgramContext::<D>::collecting(firing_collection);
    let mut next_state = original.clone();
    let mut progressed = false;
    let mut observable = false;
    for program in programs {
        let mut backend = GridProgramBackend {
            game,
            input,
            context: &mut context,
        };
        let program_outcome = puzzle_kernel::execute_program(
            &mut backend,
            &mut next_state,
            program,
            local_frame,
            UNTIL_STABLE_REPEAT_LIMIT,
        )?;
        progressed |= program_outcome.progressed;
        observable |= program_outcome.observable;
        if context.effects.cancelled {
            break;
        }
    }
    let mut outcome = GridInternalTransitionOutcome {
        input,
        next_state,
        progressed: !context.effects.cancelled && progressed,
        observable,
        cancelled: context.effects.cancelled,
        commands: context.effects.commands,
        firings: context.firings,
    };
    if outcome.cancelled {
        outcome.next_state = original;
        outcome.commands.clear();
    } else {
        outcome.next_state.clear_mark();
    }
    Ok(outcome)
}

fn detailed_outcome<const D: usize, Size: GridSize<D>>(
    outcome: GridInternalTransitionOutcome<D, Size>,
) -> GridTransitionOutcome<D, Size> {
    KernelTransitionOutcome {
        input: outcome.input,
        next_state: outcome.next_state,
        progressed: outcome.progressed,
        observable: outcome.observable,
        cancelled: outcome.cancelled,
        commands: outcome.commands,
        firings: outcome.firings.into_detailed(),
    }
}

fn summary_outcome<const D: usize, Size: GridSize<D>>(
    outcome: GridInternalTransitionOutcome<D, Size>,
) -> GridTransitionSummaryOutcome<D, Size> {
    KernelTransitionOutcome {
        input: outcome.input,
        next_state: outcome.next_state,
        progressed: outcome.progressed,
        observable: outcome.observable,
        cancelled: outcome.cancelled,
        commands: outcome.commands,
        firings: outcome.firings.into_summary(),
    }
}

pub fn transition_program_without_input<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_program_inner(game, state, program, None, None, FiringCollection::Discard)
        .map(|outcome| outcome.next_state)
}

pub fn transition_program_without_input_with_local_frame<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        None,
        local_frame,
        FiringCollection::Discard,
    )
    .map(|outcome| outcome.next_state)
}

pub fn transition_program_without_input_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_without_input_outcome_with_local_frame(game, state, program, None)
}

pub fn transition_program_sequence_without_input_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    programs: &[&GridExecutableProgram<D>],
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    let mut outcome = KernelTransitionOutcome {
        input: None,
        next_state: state.clone(),
        progressed: false,
        observable: false,
        cancelled: false,
        commands: Vec::new(),
        firings: Vec::new(),
    };
    for program in programs {
        let next = transition_program_without_input_outcome(game, &outcome.next_state, program)?;
        outcome.next_state = next.next_state;
        outcome.progressed |= next.progressed;
        outcome.observable |= next.observable;
        outcome.commands.extend(next.commands);
        outcome.firings.extend(next.firings);
        if next.cancelled {
            outcome.cancelled = true;
            break;
        }
    }
    Ok(outcome)
}

pub fn transition_program_sequence_without_input_summary_outcome<
    const D: usize,
    Size: GridSize<D>,
>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    programs: &[&GridExecutableProgram<D>],
) -> Result<GridTransitionSummaryOutcome<D, Size>, GridTransitionError<D>> {
    let mut outcome = KernelTransitionOutcome {
        input: None,
        next_state: state.clone(),
        progressed: false,
        observable: false,
        cancelled: false,
        commands: Vec::new(),
        firings: Vec::new(),
    };
    for program in programs {
        let next =
            transition_program_without_input_summary_outcome(game, &outcome.next_state, program)?;
        outcome.next_state = next.next_state;
        outcome.progressed |= next.progressed;
        outcome.observable |= next.observable;
        outcome.commands.extend(next.commands);
        outcome.firings.extend(next.firings);
        if next.cancelled {
            outcome.cancelled = true;
            break;
        }
    }
    Ok(outcome)
}

pub fn transition_program_without_input_summary_outcome<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
) -> Result<GridTransitionSummaryOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_inner(game, state, program, None, None, FiringCollection::Summary)
        .map(summary_outcome)
}

pub fn transition_program_without_input_outcome_with_local_frame<
    const D: usize,
    Size: GridSize<D>,
>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    program: &GridExecutableProgram<D>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<GridTransitionOutcome<D, Size>, GridTransitionError<D>> {
    transition_program_inner(
        game,
        state,
        program,
        None,
        local_frame,
        FiringCollection::Detailed,
    )
    .map(detailed_outcome)
}

struct GridProgramBackend<'a, const D: usize> {
    game: &'a GridCompiledGame<D>,
    input: Option<InputId>,
    context: &'a mut GridProgramContext<D>,
}

impl<const D: usize, Size: GridSize<D>>
    ProgramBackend<GridRule<D>, GridRuleCondition<D>, LocalFrame<ObjectId>, GridState<D, Size>>
    for GridProgramBackend<'_, D>
{
    type Error = GridTransitionError<D>;
    type Snapshot = GridProgramContext<D>;

    fn condition_accepts(
        &mut self,
        state: &GridState<D, Size>,
        condition: &GridRuleCondition<D>,
        frame: Option<&LocalFrame<ObjectId>>,
    ) -> bool {
        rule_condition_accepts(self.game, state, condition, self.input, frame)
    }

    fn apply_rule(
        &mut self,
        state: &mut GridState<D, Size>,
        rule: &GridRule<D>,
        frame: Option<&LocalFrame<ObjectId>>,
    ) -> Result<ProgramApplyOutcome, Self::Error> {
        let outcome = if matches!(
            rule.application,
            RuleApplication::Once | RuleApplication::RepeatStep
        ) {
            transition_rule_once_in_place(self.game, state, rule, self.input, frame, self.context)?
        } else if rule.application == RuleApplication::OnceAll {
            transition_rule_once_all_in_place(
                self.game,
                state,
                rule,
                self.input,
                frame,
                self.context,
            )?
        } else {
            let transition = transition_rule_by_application(
                self.game,
                state,
                rule,
                self.input,
                frame,
                self.context,
            )?;
            if let Some(next_state) = transition.next_state {
                *state = next_state;
            }
            transition.outcome
        };
        Ok(outcome)
    }

    fn checkpoint(&self) -> Self::Snapshot {
        self.context.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        *self.context = snapshot.clone();
    }

    fn choose_random(&self, state: &GridState<D, Size>, candidate_count: usize) -> usize {
        grid_random_choice_index(self.game, state, self.input, RuleId(0), candidate_count)
    }

    fn state_key(&self, state: &GridState<D, Size>) -> puzzle_kernel::ProgramStateKey {
        state.program_state_key()
    }

    fn invalid_program_continuation(&self) -> Self::Error {
        GridTransitionError::InvalidProgramContinuation
    }
}

fn transition_rule_by_application<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    match rule.application {
        RuleApplication::Once => {
            transition_rule_once(game, state, rule, input, local_frame, context)
        }
        RuleApplication::RepeatStep => {
            transition_rule_once(game, state, rule, input, local_frame, context)
        }
        RuleApplication::OnceAll => {
            transition_rule_once_all(game, state, rule, input, local_frame, context)
        }
        RuleApplication::OncePerLevel => {
            transition_rule_once_per_level(game, state, rule, input, local_frame, context)
        }
        RuleApplication::UntilStable => {
            transition_rule_repeated(game, state, rule, input, local_frame, context)
        }
        RuleApplication::Random => {
            transition_rule_random(game, state, rule, input, local_frame, context)
        }
    }
}

fn rule_condition_accepts<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    condition: &GridRuleCondition<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    let scope = LocalFrameScope::new(state, local_frame);
    match condition {
        GridRuleCondition::<D>::AnyMatches(patterns) => patterns
            .iter()
            .any(|pattern| first_match(game, state, pattern, &scope).is_some()),
        GridRuleCondition::<D>::NoMatches(patterns) => patterns
            .iter()
            .all(|pattern| first_match(game, state, pattern, &scope).is_none()),
        GridRuleCondition::<D>::AnyInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && first_match(game, state, pattern, &scope).is_some()
            })
        }),
        GridRuleCondition::<D>::NoInputMatches(patterns) => input.is_none_or(|input| {
            patterns.iter().all(|(expected, pattern)| {
                *expected != input || first_match(game, state, pattern, &scope).is_none()
            })
        }),
        GridRuleCondition::<D>::GuardBranches(branches) => branches
            .iter()
            .any(|branch| guards_accept_all(branch, game, state, input, local_frame)),
    }
}

fn transition_rule_once<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    if rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(GridRuleTransition::idle());
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let placement =
        if rule.application == RuleApplication::RepeatStep && !rule_has_observable_effect(rule) {
            first_progressing_match(game, state, rule, &scope)?
        } else {
            first_writable_match(game, state, rule, &scope)?
        };
    let Some(placement) = placement else {
        return Ok(GridRuleTransition::idle());
    };
    let patch = build_patch(rule, &placement)?;
    if rule_cancels(rule) {
        patch.validate(game, state)?;
        let outcome = context.commit(rule, patch, false);
        return Ok(GridRuleTransition {
            next_state: None,
            outcome,
        });
    }
    let mut next = state.clone();
    let progressed = patch.apply_in_place(game, &mut next)?;
    let outcome = context.commit(rule, patch, progressed);
    Ok(GridRuleTransition {
        next_state: Some(next),
        outcome,
    })
}

fn transition_rule_once_in_place<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &mut GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<ProgramApplyOutcome, GridTransitionError<D>> {
    if rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(ProgramApplyOutcome::default());
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let placement =
        if rule.application == RuleApplication::RepeatStep && !rule_has_observable_effect(rule) {
            first_progressing_match(game, state, rule, &scope)?
        } else {
            first_writable_match(game, state, rule, &scope)?
        };
    let Some(placement) = placement else {
        return Ok(ProgramApplyOutcome::default());
    };
    let patch = build_patch(rule, &placement)?;
    let progressed = if rule_cancels(rule) {
        patch.validate(game, state)?;
        false
    } else {
        patch.apply_in_place(game, state)?
    };
    Ok(context.commit(rule, patch, progressed))
}

fn transition_rule_random<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    if rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(GridRuleTransition::idle());
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let placements = writable_matches(game, state, rule, &scope)?;
    if placements.is_empty() {
        return Ok(GridRuleTransition::idle());
    }
    let index = grid_random_choice_index(game, state, input, rule.id, placements.len());
    let placement = &placements[index];
    let patch = build_patch(rule, placement)?;
    if rule_cancels(rule) {
        patch.validate(game, state)?;
        let outcome = context.commit(rule, patch, false);
        return Ok(GridRuleTransition {
            next_state: None,
            outcome,
        });
    }
    let mut next = state.clone();
    let progressed = patch.apply_in_place(game, &mut next)?;
    let outcome = context.commit(rule, patch, progressed);
    Ok(GridRuleTransition {
        next_state: Some(next),
        outcome,
    })
}

pub fn transition_once_all<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut context = GridProgramContext::<D>::default();
    let mut next = transition_rule_once_all(game, &scoped, rule, None, None, &mut context)?
        .next_state
        .unwrap_or(scoped);
    next.clear_mark();
    Ok(next)
}

pub fn transition_once_per_level<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut context = GridProgramContext::<D>::default();
    let mut next = transition_rule_once_per_level(game, &scoped, rule, None, None, &mut context)?
        .next_state
        .unwrap_or(scoped);
    next.clear_mark();
    Ok(next)
}

fn transition_rule_once_all<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    if rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(GridRuleTransition::idle());
    }

    let scope = LocalFrameScope::new(state, local_frame);
    let placements = all_matches(game, state, &rule.pattern, &scope);
    if placements.is_empty() {
        return Ok(GridRuleTransition::idle());
    }

    let mut current = state.clone();
    let mut current_scope = LocalFrameScope::new(&current, local_frame);
    let mut outcome = ProgramApplyOutcome::default();
    for placement in placements {
        if !placement_still_valid(game, &current, &rule.pattern, &placement, &current_scope) {
            continue;
        }
        if !writes_within_local_frame(&placement, &rule.writes, &current_scope)? {
            continue;
        };

        let patch = build_patch(rule, &placement)?;
        if rule_cancels(rule) {
            patch.validate(game, &current)?;
            outcome.merge(context.commit(rule, patch, false));
            return Ok(GridRuleTransition {
                next_state: None,
                outcome,
            });
        }
        match patch.apply_in_place(game, &mut current) {
            Ok(changed) => {
                outcome.merge(context.commit(rule, patch, changed));
                if changed {
                    current_scope = LocalFrameScope::new(&current, local_frame);
                }
            }
            Err(error) if once_all_patch_became_stale(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(GridRuleTransition {
        next_state: outcome.fired.then_some(current),
        outcome,
    })
}

fn transition_rule_once_all_in_place<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &mut GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<ProgramApplyOutcome, GridTransitionError<D>> {
    if rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(ProgramApplyOutcome::default());
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let placements = all_matches(game, state, &rule.pattern, &scope);
    let mut current_scope = scope;
    let mut outcome = ProgramApplyOutcome::default();
    for placement in placements {
        if !placement_still_valid(game, state, &rule.pattern, &placement, &current_scope)
            || !writes_within_local_frame(&placement, &rule.writes, &current_scope)?
        {
            continue;
        }
        let patch = build_patch(rule, &placement)?;
        if rule_cancels(rule) {
            patch.validate(game, state)?;
            outcome.merge(context.commit(rule, patch, false));
            return Ok(outcome);
        }
        let changed = patch.apply_in_place(game, state)?;
        outcome.merge(context.commit(rule, patch, changed));
        if changed {
            current_scope = LocalFrameScope::new(state, local_frame);
        }
    }
    Ok(outcome)
}

fn transition_rule_once_per_level<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    if state.level_rule_has_fired(rule.id)
        || rule_required_anchor_is_absent(state, rule)
        || !guards_accept(rule, game, state, input, local_frame)
    {
        return Ok(GridRuleTransition::idle());
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let Some(placement) = first_writable_match(game, state, rule, &scope)? else {
        return Ok(GridRuleTransition::idle());
    };
    let patch = build_patch(rule, &placement)?;
    if rule_cancels(rule) {
        patch.validate(game, state)?;
        let outcome = context.commit(rule, patch, false);
        return Ok(GridRuleTransition {
            next_state: None,
            outcome,
        });
    }
    let mut next = state.clone();
    patch.apply_in_place(game, &mut next)?;
    next.mark_level_rule_fired(rule.id);
    let outcome = context.commit(rule, patch, true);
    Ok(GridRuleTransition {
        next_state: Some(next),
        outcome,
    })
}

pub fn transition_repeated<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
) -> Result<GridState<D, Size>, GridTransitionError<D>> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut context = GridProgramContext::<D>::default();
    let mut next = transition_rule_repeated(game, &scoped, rule, None, None, &mut context)?
        .next_state
        .unwrap_or(scoped);
    next.clear_mark();
    Ok(next)
}

pub fn count_pattern_matches<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
) -> u32 {
    if pattern.components.is_empty() {
        return 0;
    }
    let scope = LocalFrameScope::new(state, None);
    all_matches(game, state, pattern, &scope).len() as u32
}

pub fn has_pattern_match<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
) -> bool {
    if pattern.components.is_empty() {
        return false;
    }
    let scope = LocalFrameScope::new(state, None);
    first_match(game, state, pattern, &scope).is_some()
}

pub fn eval_condition_kind<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    kind: &GridConditionValueKind<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> i64 {
    let scope = LocalFrameScope::new(state, local_frame);
    match kind {
        GridConditionValueKind::<D>::CountObjects(objects) => objects
            .iter()
            .map(|object| count_object(game, state, *object))
            .sum::<u32>() as i64,
        GridConditionValueKind::<D>::ExistsObjects(objects) => objects
            .iter()
            .any(|object| count_object(game, state, *object) > 0)
            as i64,
        GridConditionValueKind::<D>::NoneObjects(objects) => objects
            .iter()
            .all(|object| count_object(game, state, *object) == 0)
            as i64,
        GridConditionValueKind::<D>::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| all_matches(game, state, pattern, &scope).len() as u32)
            .sum::<u32>() as i64,
        GridConditionValueKind::<D>::ExistsMatches(patterns) => patterns
            .iter()
            .any(|pattern| first_match(game, state, pattern, &scope).is_some())
            as i64,
        GridConditionValueKind::<D>::NoneMatches(patterns) => patterns
            .iter()
            .all(|pattern| first_match(game, state, pattern, &scope).is_none())
            as i64,
        GridConditionValueKind::<D>::CountInputMatches(patterns) => input
            .map(|input| {
                patterns
                    .iter()
                    .filter(|(expected, _)| *expected == input)
                    .map(|(_, pattern)| all_matches(game, state, pattern, &scope).len() as u32)
                    .sum::<u32>() as i64
            })
            .unwrap_or(0),
        GridConditionValueKind::<D>::ExistsInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && first_match(game, state, pattern, &scope).is_some()
            })
        }) as i64,
        GridConditionValueKind::<D>::NoneInputMatches(patterns) => input.is_some_and(|input| {
            patterns
                .iter()
                .filter(|(expected, _)| *expected == input)
                .all(|(_, pattern)| first_match(game, state, pattern, &scope).is_none())
        }) as i64,
    }
}

fn transition_rule_repeated<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    context: &mut GridProgramContext<D>,
) -> Result<GridRuleTransition<GridState<D, Size>>, GridTransitionError<D>> {
    let mut current = state.clone();
    let mut seen = vec![current.clone()];
    let mut repeat_count = 0;
    let mut outcome = ProgramApplyOutcome::default();
    loop {
        let step = transition_rule_once_all(game, &current, rule, input, local_frame, context)?;
        let step_outcome = step.outcome;
        outcome.merge(step_outcome);
        let Some(next) = step.next_state else {
            return Ok(GridRuleTransition {
                next_state: outcome.progressed.then_some(current),
                outcome,
            });
        };
        if !step_outcome.progressed || next == current {
            return Ok(GridRuleTransition {
                next_state: outcome.progressed.then_some(current),
                outcome,
            });
        }
        if seen.iter().any(|seen_state| *seen_state == next) {
            return Ok(GridRuleTransition {
                next_state: Some(next),
                outcome,
            });
        }
        seen.push(next.clone());
        current = next;
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT {
            return Ok(GridRuleTransition {
                next_state: Some(current),
                outcome,
            });
        }
    }
}

pub(crate) fn grid_random_choice_index<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    rule: RuleId,
    candidate_count: usize,
) -> usize {
    let mut hash = random_state_projection_hash(game, state);
    match input {
        Some(input) => {
            hash = fnv_mix(hash, u64::from(input.0));
        }
        None => {
            hash = fnv_mix(hash, u64::MAX);
        }
    }
    hash = fnv_mix(hash, u64::from(rule.0));
    hash = fnv_mix(hash, candidate_count as u64);
    (hash as usize) % candidate_count
}

fn random_state_projection_hash<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
) -> u64 {
    let mut hash = FnvBuilder::OFFSET;
    for axis in state.size.axes() {
        hash = fnv_mix(hash, u64::from(axis));
    }
    hash = fnv_mix(hash, u64::from(state.layer_count));
    let main_layers = game.main_layers();
    hash = fnv_mix(hash, main_layers.len() as u64);
    for coord in all_coords(state) {
        for layer in &main_layers {
            let object = state.get_layer_at(coord, *layer).unwrap_or(ObjectId::EMPTY);
            let object = if game.is_main_object(object) {
                object
            } else {
                ObjectId::EMPTY
            };
            hash = fnv_mix(hash, u64::from(object.0));
        }
    }
    hash = fnv_mix(hash, state.visible_variables().len() as u64);
    for value in state.visible_variables() {
        hash = fnv_mix(hash, *value as u64);
    }
    hash = fnv_mix(hash, state.level_fired_rules().len() as u64);
    for rule in state.level_fired_rules() {
        hash = fnv_mix(hash, u64::from(rule.0));
    }
    hash
}

fn guards_accept<const D: usize, Size: GridSize<D>>(
    rule: &GridRule<D>,
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    guards_accept_all(&rule.guards, game, state, input, local_frame)
}

fn guards_accept_all<const D: usize, Size: GridSize<D>>(
    guards: &[GridGuard<D>],
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    guards
        .iter()
        .all(|guard| guard_accepts(guard, game, state, input, local_frame))
}

fn guard_accepts<const D: usize, Size: GridSize<D>>(
    guard: &GridGuard<D>,
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    match guard {
        GridGuard::<D>::InputIs(expected) => input.is_some_and(|actual| actual == *expected),
        GridGuard::<D>::VariableEquals { variable, value } => {
            state.variable_value(*variable) == Some(*value)
        }
        GridGuard::<D>::VariableCompare {
            variable,
            op,
            value,
        } => state
            .variable_value(*variable)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        GridGuard::<D>::ConditionEquals { condition, value } => {
            eval_condition_def(game, state, *condition, input, local_frame) == Some(*value)
        }
        GridGuard::<D>::ConditionNonZero(condition) => {
            eval_condition_def(game, state, *condition, input, local_frame)
                .is_some_and(|value| value != 0)
        }
        GridGuard::<D>::ConditionCompare {
            condition,
            op,
            value,
        } => eval_condition_def(game, state, *condition, input, local_frame)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        GridGuard::<D>::InlineConditionValue { kind, value } => {
            eval_condition_kind(game, state, kind, input, local_frame) == *value
        }
        GridGuard::<D>::InlineConditionNonZero(kind) => {
            eval_condition_kind(game, state, kind, input, local_frame) != 0
        }
        GridGuard::<D>::InlineConditionCompare { kind, op, value } => compare_i64(
            eval_condition_kind(game, state, kind, input, local_frame),
            *op,
            *value,
        ),
    }
}

fn eval_condition_def<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    condition: ConditionId,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Option<i64> {
    let condition = game.condition_def(condition)?;
    Some(eval_condition_kind(
        game,
        state,
        &condition.kind,
        input,
        local_frame,
    ))
}

fn compare_i64(left: i64, op: ComparisonOp, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::NotEq => left != right,
        ComparisonOp::Greater => left > right,
        ComparisonOp::GreaterEq => left >= right,
        ComparisonOp::Less => left < right,
        ComparisonOp::LessEq => left <= right,
    }
}

struct LocalFrameScope<'a, const D: usize> {
    frame: Option<&'a LocalFrame<ObjectId>>,
    focus_coords: Vec<GridCoord<D>>,
}

impl<'a, const D: usize> LocalFrameScope<'a, D> {
    fn new<Size: GridSize<D>>(
        state: &GridState<D, Size>,
        frame: Option<&'a LocalFrame<ObjectId>>,
    ) -> Self {
        let focus_coords = frame
            .map(|frame| focus_coords(state, frame))
            .unwrap_or_default();
        Self {
            frame,
            focus_coords,
        }
    }

    fn contains_coord(&self, coord: GridCoord<D>) -> bool {
        let Some(frame) = self.frame else {
            return true;
        };
        self.focus_coords.iter().any(|focus| {
            coord
                .axes()
                .iter()
                .zip(focus.axes())
                .enumerate()
                .all(|(axis, (coord, focus))| {
                    let delta = i32::from(*coord) - i32::from(focus);
                    match axis {
                        0 => frame.x.contains_delta(delta),
                        1 => frame.y.contains_delta(delta),
                        2 => frame.z.contains_delta(delta),
                        _ => false,
                    }
                })
        })
    }

    fn origin_candidates<Size: GridSize<D>>(
        &self,
        state: &GridState<D, Size>,
    ) -> Option<Vec<GridCoord<D>>> {
        self.frame.as_ref()?;
        if self.focus_coords.is_empty() {
            return Some(Vec::new());
        }
        Some(
            all_coords(state)
                .into_iter()
                .filter(|coord| self.contains_coord(*coord))
                .collect(),
        )
    }
}

fn all_coords<const D: usize, Size: GridSize<D>>(state: &GridState<D, Size>) -> Vec<GridCoord<D>> {
    let Some(cell_count) = state.shape().cell_count() else {
        return Vec::new();
    };
    (0..cell_count)
        .filter_map(|index| state.cell_coord(index))
        .collect()
}

fn focus_coords<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    local_frame: &LocalFrame<ObjectId>,
) -> Vec<GridCoord<D>> {
    all_coords(state)
        .into_iter()
        .filter(|coord| {
            local_frame.focus_objects.iter().any(|object| {
                state
                    .cell_view_at(*coord)
                    .is_ok_and(|cell| cell.objects.contains(object))
            })
        })
        .collect()
}

fn writes_within_local_frame<const D: usize>(
    placement: &GridMatchPlacement<D>,
    writes: &[GridWriteOp<D>],
    scope: &LocalFrameScope<'_, D>,
) -> Result<bool, GridTransitionError<D>> {
    if scope.frame.is_none() {
        return Ok(true);
    }
    for write in writes {
        match write.clone() {
            GridWriteOp::<D>::Add {
                component, offset, ..
            }
            | GridWriteOp::<D>::AddObjectSet {
                component, offset, ..
            }
            | GridWriteOp::<D>::Remove {
                component, offset, ..
            }
            | GridWriteOp::<D>::RemoveObjectSet {
                component, offset, ..
            }
            | GridWriteOp::<D>::Replace {
                component, offset, ..
            }
            | GridWriteOp::<D>::SetMark {
                component, offset, ..
            }
            | GridWriteOp::<D>::SetObjectSetMark {
                component, offset, ..
            }
            | GridWriteOp::<D>::RemoveMark {
                component, offset, ..
            }
            | GridWriteOp::<D>::RemoveObjectSetMark {
                component, offset, ..
            } => {
                let position = write_position(placement, component, offset)?;
                if !scope.contains_coord(position) {
                    return Ok(false);
                }
            }
            GridWriteOp::<D>::Move {
                component,
                from_offset,
                to_offset,
                ..
            }
            | GridWriteOp::<D>::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                ..
            } => {
                let from = write_position(placement, component, from_offset)?;
                let to = write_position(placement, component, to_offset)?;
                if !scope.contains_coord(from) || !scope.contains_coord(to) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn first_writable_match<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Result<Option<GridMatchPlacement<D>>, GridTransitionError<D>> {
    for placement in all_matches(game, state, &rule.pattern, scope) {
        if writes_within_local_frame(&placement, &rule.writes, scope)? {
            return Ok(Some(placement));
        }
    }
    Ok(None)
}

fn first_progressing_match<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Result<Option<GridMatchPlacement<D>>, GridTransitionError<D>> {
    let mut first_match = None;
    for placement in all_matches(game, state, &rule.pattern, scope) {
        if !writes_within_local_frame(&placement, &rule.writes, scope)? {
            continue;
        }
        if first_match.is_none() {
            first_match = Some(placement.clone());
        }
        let patch = build_patch(rule, &placement)?;
        if patch.validate(game, state)? {
            return Ok(Some(placement));
        }
    }
    Ok(first_match)
}

fn writable_matches<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Result<Vec<GridMatchPlacement<D>>, GridTransitionError<D>> {
    let mut placements = Vec::new();
    for placement in all_matches(game, state, &rule.pattern, scope) {
        if !writes_within_local_frame(&placement, &rule.writes, scope)? {
            continue;
        }
        placements.push(placement);
    }
    Ok(placements)
}

type GridMatchPlacement<const D: usize> = MatchPlacement<D, ObjectId>;
type GridComponentPlacement<const D: usize> = ComponentPlacement<D, ObjectId>;

#[derive(Clone, Copy)]
enum ComponentAnchor<'a, const D: usize> {
    Object(&'a crate::GridOffset<D>, ObjectId),
    ObjectSet(&'a crate::GridOffset<D>, &'a [ObjectId]),
    Mark(&'a crate::GridOffset<D>, ObjectId, MarkId, Option<i64>),
    ObjectSetMark(
        &'a crate::GridOffset<D>,
        &'a [ObjectId],
        MarkId,
        Option<i64>,
    ),
}

fn first_match<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<GridMatchPlacement<D>> {
    if pattern.components.is_empty() {
        return Some(GridMatchPlacement::<D>::empty());
    }
    if let Some(placement) = first_anchored_match(game, state, pattern, scope) {
        return placement;
    }
    component_origin_candidates(state, &pattern.components[0], scope)
        .into_iter()
        .find_map(|origin| pattern_placement_from_first_origin(game, state, pattern, origin, scope))
}

fn first_anchored_match<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<Option<GridMatchPlacement<D>>> {
    let anchor = component_anchor(&pattern.components[0])?;
    let inverse = inverse_resolved_offset(match &anchor {
        ComponentAnchor::Object(offset, _)
        | ComponentAnchor::ObjectSet(offset, _)
        | ComponentAnchor::Mark(offset, ..)
        | ComponentAnchor::ObjectSetMark(offset, ..) => offset,
    })?;
    match anchor {
        ComponentAnchor::Object(_, object) | ComponentAnchor::Mark(_, object, ..) => {
            let positions = match anchor {
                ComponentAnchor::Object(..) => state.object_positions(object),
                ComponentAnchor::Mark(_, _, mark, value) => {
                    state.mark_positions(object, mark, value)
                }
                _ => unreachable!(),
            };
            for index in positions {
                let Some(position) = cached_position_coord(state, object, *index) else {
                    continue;
                };
                let Some(origin) = position.checked_offset(inverse) else {
                    continue;
                };
                if !scope.contains_coord(origin) {
                    continue;
                }
                if let Some(placement) =
                    pattern_placement_from_first_origin(game, state, pattern, origin, scope)
                {
                    return Some(Some(placement));
                }
            }
        }
        ComponentAnchor::ObjectSetMark(_, objects, mark, value) => {
            for slot in state.slot_mark_positions(mark, value) {
                if !state
                    .slots()
                    .get(*slot)
                    .is_some_and(|object| objects.contains(object))
                {
                    continue;
                }
                let Some(position) = state.slot_coord(*slot) else {
                    continue;
                };
                let Some(origin) = position.checked_offset(inverse) else {
                    continue;
                };
                if !scope.contains_coord(origin) {
                    continue;
                }
                if let Some(placement) =
                    pattern_placement_from_first_origin(game, state, pattern, origin, scope)
                {
                    return Some(Some(placement));
                }
            }
        }
        ComponentAnchor::ObjectSet(_, _) => {
            for origin in anchored_component_origins(state, &pattern.components[0], scope)? {
                if let Some(placement) =
                    pattern_placement_from_first_origin(game, state, pattern, origin, scope)
                {
                    return Some(Some(placement));
                }
            }
        }
    }
    Some(None)
}

fn all_matches<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Vec<GridMatchPlacement<D>> {
    if pattern.components.is_empty() {
        return vec![GridMatchPlacement::<D>::empty()];
    }
    if component_anchor(&pattern.components[0])
        .is_some_and(|anchor| !component_anchor_has_candidates(state, anchor))
    {
        return Vec::new();
    }
    let mut placements = Vec::new();
    for origin in component_origin_candidates(state, &pattern.components[0], scope) {
        if let Some(first) =
            component_placement_at(game, state, &pattern.components[0], origin, scope)
        {
            let mut components = vec![first];
            placements.extend(collect_component_placements(
                game,
                state,
                pattern,
                1,
                &mut components,
                scope,
            ));
        }
    }
    placements
}

fn pattern_placement_from_first_origin<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    origin: GridCoord<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<GridMatchPlacement<D>> {
    let first = component_placement_at(game, state, &pattern.components[0], origin, scope)?;
    let mut components = vec![first];
    complete_component_placements(game, state, pattern, 1, &mut components, scope)
        .then_some(GridMatchPlacement::<D>::new(components))
}

fn complete_component_placements<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    component_index: usize,
    components: &mut Vec<GridComponentPlacement<D>>,
    scope: &LocalFrameScope<'_, D>,
) -> bool {
    let mut candidate_origins =
        |component: &GridPatternComponent<D>| component_origin_candidates(state, component, scope);
    let mut place_at = |component: &GridPatternComponent<D>, origin| {
        component_placement_at(game, state, component, origin, scope)
    };
    complete_component_placements_shared(
        &pattern.components,
        component_index,
        components,
        &mut candidate_origins,
        &mut place_at,
    )
}

fn collect_component_placements<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    component_index: usize,
    components: &mut Vec<GridComponentPlacement<D>>,
    scope: &LocalFrameScope<'_, D>,
) -> Vec<GridMatchPlacement<D>> {
    let mut matches = Vec::new();
    let mut candidate_origins =
        |component: &GridPatternComponent<D>| component_origin_candidates(state, component, scope);
    let mut place_at = |component: &GridPatternComponent<D>, origin| {
        component_placement_at(game, state, component, origin, scope)
    };
    let mut push_match = |matches: &mut Vec<GridMatchPlacement<D>>,
                          components: &[GridComponentPlacement<D>]| {
        matches.push(GridMatchPlacement::<D>::new(components.to_vec()));
    };
    collect_component_placements_shared(
        &pattern.components,
        component_index,
        components,
        &mut matches,
        &mut candidate_origins,
        &mut place_at,
        &mut push_match,
    );
    matches
}

fn component_origin_candidates<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    component: &GridPatternComponent<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Vec<GridCoord<D>> {
    if let Some(candidates) = anchored_component_origins(state, component, scope) {
        return candidates;
    }
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates;
    }
    all_coords(state)
}

fn anchored_component_origins<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    component: &GridPatternComponent<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<Vec<GridCoord<D>>> {
    let anchor = component_anchor(component)?;
    let (offset, objects, mark, all_slot_marks, merge_required): (
        &crate::GridOffset<D>,
        &[ObjectId],
        Option<(MarkId, Option<i64>)>,
        bool,
        bool,
    ) = match &anchor {
        ComponentAnchor::Object(offset, object) => {
            (offset, std::slice::from_ref(object), None, false, false)
        }
        ComponentAnchor::ObjectSet(offset, objects) => (offset, objects, None, false, true),
        ComponentAnchor::Mark(offset, object, mark, value) => (
            offset,
            std::slice::from_ref(object),
            Some((*mark, *value)),
            false,
            false,
        ),
        ComponentAnchor::ObjectSetMark(offset, objects, mark, value) => {
            (offset, objects, Some((*mark, *value)), true, false)
        }
    };
    let inverse = inverse_resolved_offset(offset)?;
    let mut candidates = Vec::new();
    if all_slot_marks {
        let (mark, value) = mark.expect("object-set mark anchor carries a mark");
        for slot in state.slot_mark_positions(mark, value) {
            if !state
                .slots()
                .get(*slot)
                .is_some_and(|object| objects.contains(object))
            {
                continue;
            }
            let Some(position) = state.slot_coord(*slot) else {
                continue;
            };
            let Some(origin) = position.checked_offset(inverse) else {
                continue;
            };
            if scope.contains_coord(origin) {
                candidates.push(origin);
            }
        }
        return Some(candidates);
    }
    for object in objects {
        let positions = mark.map_or_else(
            || state.object_positions(*object),
            |(mark, value)| state.mark_positions(*object, mark, value),
        );
        for index in positions {
            let Some(position) = cached_position_coord(state, *object, *index) else {
                continue;
            };
            let Some(origin) = position.checked_offset(inverse) else {
                continue;
            };
            if scope.contains_coord(origin) {
                candidates.push(origin);
            }
        }
    }
    if merge_required {
        candidates.sort_unstable_by_key(|coord| state.shape().cell_index_unchecked(*coord));
        candidates.dedup();
    }
    Some(candidates)
}

fn component_anchor<const D: usize>(
    component: &GridPatternComponent<D>,
) -> Option<ComponentAnchor<'_, D>> {
    for cell in &component.cells {
        if resolve_offset(&cell.offset, &[]).is_none() {
            continue;
        }
        if let Some(mark) = cell
            .require_mark
            .iter()
            .find(|mark| mark.match_value == MarkValueMatch::Exact)
        {
            return Some(ComponentAnchor::Mark(
                &cell.offset,
                mark.object,
                mark.mark,
                mark.value,
            ));
        }
        for mark in cell
            .require_object_set_mark
            .iter()
            .filter(|mark| mark.match_value == MarkValueMatch::Exact)
        {
            if let Some(objects) = component
                .cells
                .iter()
                .flat_map(|cell| &cell.require_object_sets)
                .find(|matcher| matcher.binding == mark.binding)
                .map(|matcher| matcher.objects.as_slice())
            {
                return Some(ComponentAnchor::ObjectSetMark(
                    &cell.offset,
                    objects,
                    mark.mark,
                    mark.value,
                ));
            }
        }
    }
    for cell in &component.cells {
        if resolve_offset(&cell.offset, &[]).is_none() {
            continue;
        }
        if let Some(object) = cell.require_objects.first() {
            return Some(ComponentAnchor::Object(&cell.offset, *object));
        }
        if let Some(object_set) = cell.require_object_sets.first() {
            return Some(ComponentAnchor::ObjectSet(
                &cell.offset,
                &object_set.objects,
            ));
        }
    }
    None
}

fn component_anchor_has_candidates<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    anchor: ComponentAnchor<'_, D>,
) -> bool {
    match anchor {
        ComponentAnchor::Object(_, object) => !state.object_positions(object).is_empty(),
        ComponentAnchor::ObjectSet(_, objects) => objects
            .iter()
            .any(|object| !state.object_positions(*object).is_empty()),
        ComponentAnchor::Mark(_, object, mark, value) => {
            !state.mark_positions(object, mark, value).is_empty()
        }
        ComponentAnchor::ObjectSetMark(_, objects, mark, value) => {
            state.slot_mark_positions(mark, value).iter().any(|slot| {
                state
                    .slots()
                    .get(*slot)
                    .is_some_and(|object| objects.contains(object))
            })
        }
    }
}

fn rule_required_anchor_is_absent<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    rule: &GridRule<D>,
) -> bool {
    let Some(component) = rule.pattern.components.first() else {
        return false;
    };
    component_anchor(component)
        .is_some_and(|anchor| !component_anchor_has_candidates(state, anchor))
}

fn cached_position_coord<const D: usize, Size: GridSize<D>>(
    state: &GridState<D, Size>,
    object: ObjectId,
    index: usize,
) -> Option<GridCoord<D>> {
    if object.is_empty() {
        state.cell_coord(index)
    } else {
        state.slot_coord(index)
    }
}

fn inverse_resolved_offset<const D: usize>(
    offset: &crate::GridOffset<D>,
) -> Option<CoordOffset<D>> {
    let resolved = resolve_offset(offset, &[])?;
    let mut inverse = [0_i16; D];
    for (target, delta) in inverse.iter_mut().zip(resolved.deltas()) {
        *target = delta.checked_neg()?;
    }
    Some(CoordOffset::new(inverse))
}

fn component_placement_at<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    component: &GridPatternComponent<D>,
    origin: GridCoord<D>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<GridComponentPlacement<D>> {
    if component.gap_count == 0 {
        let gaps = Vec::new();
        let object_bindings =
            component_matches_with_gaps(game, state, component, origin, &gaps, scope)?;
        return Some(GridComponentPlacement::<D>::new(
            origin.into(),
            gaps,
            object_bindings,
        ));
    }

    let max_gap = state.size.axes().into_iter().max().unwrap_or(0);
    for total_gap in 0..=max_gap.saturating_mul(component.gap_count) {
        let mut gaps = Vec::with_capacity(usize::from(component.gap_count));
        if let Some(object_bindings) = find_gap_assignment(
            game, state, component, origin, max_gap, total_gap, &mut gaps, scope,
        ) {
            return Some(GridComponentPlacement::<D>::new(
                origin.into(),
                gaps,
                object_bindings,
            ));
        }
    }
    None
}

fn find_gap_assignment<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    component: &GridPatternComponent<D>,
    origin: GridCoord<D>,
    max_gap: u16,
    remaining_total: u16,
    gaps: &mut Vec<u16>,
    scope: &LocalFrameScope<'_, D>,
) -> Option<Vec<ObjectBinding<ObjectId>>> {
    if gaps.len() == usize::from(component.gap_count) {
        return (remaining_total == 0)
            .then(|| component_matches_with_gaps(game, state, component, origin, gaps, scope))
            .flatten();
    }
    for gap in 0..=max_gap.min(remaining_total) {
        gaps.push(gap);
        if let Some(bindings) = find_gap_assignment(
            game,
            state,
            component,
            origin,
            max_gap,
            remaining_total - gap,
            gaps,
            scope,
        ) {
            return Some(bindings);
        }
        gaps.pop();
    }
    None
}

fn component_matches_with_gaps<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    component: &GridPatternComponent<D>,
    origin: GridCoord<D>,
    gaps: &[u16],
    scope: &LocalFrameScope<'_, D>,
) -> Option<Vec<ObjectBinding<ObjectId>>> {
    let mut object_bindings = Vec::new();
    for cell in &component.cells {
        let position = resolve_offset(&cell.offset, gaps)
            .and_then(|offset| offset_pos(origin, offset))
            .filter(|position| state.check_pos((*position).into()).is_ok());
        match puzzle_kernel::match_cell_bounds(cell.require_null, position.is_some()) {
            puzzle_kernel::CellBoundsMatch::MatchedNull => continue,
            puzzle_kernel::CellBoundsMatch::Rejected => return None,
            puzzle_kernel::CellBoundsMatch::Continue => {}
        }
        let position = position.expect("in-bounds cell decision preserves its position");
        if !scope.contains_coord(position) {
            return None;
        }
        if !cell
            .require_objects
            .iter()
            .all(|object| cell_requires_object(game, state, position, *object))
            || !cell
                .forbid_objects
                .iter()
                .all(|object| cell_forbids_object(game, state, position, *object))
        {
            return None;
        }
        for object_set in &cell.require_object_sets {
            let Ok(found) = state.get_layer_at(position, object_set.layer) else {
                return None;
            };
            if found.is_empty() || !object_set.objects.contains(&found) {
                return None;
            }
            if !bind_object(&mut object_bindings, object_set.binding, found) {
                return None;
            }
        }
        if !cell
            .require_mark
            .iter()
            .all(|mark| mark_pattern_matches(game, state, position, mark.object, mark))
            || !cell.require_object_set_mark.iter().all(|mark| {
                let Some(object) = bound_object_in_component(&object_bindings, mark.binding) else {
                    return false;
                };
                mark_pattern_matches_bound(game, state, position, object, mark)
            })
            || !cell
                .forbid_mark
                .iter()
                .all(|mark| !mark_pattern_matches(game, state, position, mark.object, mark))
            || !cell.forbid_object_set_mark.iter().all(|mark| {
                let Some(object) = bound_object_in_component(&object_bindings, mark.binding) else {
                    return false;
                };
                !mark_pattern_matches_bound(game, state, position, object, mark)
            })
        {
            return None;
        }
    }
    Some(object_bindings)
}

fn placement_still_valid<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    pattern: &GridPattern<D>,
    placement: &GridMatchPlacement<D>,
    scope: &LocalFrameScope<'_, D>,
) -> bool {
    pattern.components.iter().zip(&placement.components).all(
        |(pattern_component, placed_component)| {
            component_matches_with_gaps(
                game,
                state,
                pattern_component,
                placed_component.origin.into(),
                &placed_component.gaps,
                scope,
            )
            .is_some_and(|bindings| {
                object_bindings_match(&bindings, &placed_component.object_bindings)
            })
        },
    )
}

fn object_bindings_match(
    left: &[ObjectBinding<ObjectId>],
    right: &[ObjectBinding<ObjectId>],
) -> bool {
    left.len() == right.len()
        && left.iter().all(|binding| {
            right.iter().any(|existing| {
                existing.binding == binding.binding && existing.object == binding.object
            })
        })
}

fn bound_object<const D: usize>(
    placement: &GridMatchPlacement<D>,
    binding: u16,
) -> Option<ObjectId> {
    placement_object_binding(placement, binding)
}

fn bound_object_in_component(
    object_bindings: &[ObjectBinding<ObjectId>],
    binding: u16,
) -> Option<ObjectId> {
    bound_object_in_bindings(object_bindings, binding)
}

fn mark_pattern_matches<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
    mark: &MarkPattern,
) -> bool {
    match mark.match_value {
        MarkValueMatch::Any => state.has_mark_key_at(game, position, object, mark.mark),
        MarkValueMatch::Exact => state.has_mark_at(game, position, object, mark.mark, mark.value),
    }
}

fn mark_pattern_matches_bound<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
    mark: &ObjectSetMarkPattern,
) -> bool {
    match mark.match_value {
        MarkValueMatch::Any => state.has_mark_key_at(game, position, object, mark.mark),
        MarkValueMatch::Exact => state.has_mark_at(game, position, object, mark.mark, mark.value),
    }
}

fn cell_requires_object<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
) -> bool {
    match state.cell_has_object_masked_at(position, object) {
        Some(found) => found,
        None => state.has_object_at(game, position, object),
    }
}

fn cell_forbids_object<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
) -> bool {
    match state.cell_has_object_masked_at(position, object) {
        Some(found) => !found,
        None => !state.has_object_at(game, position, object),
    }
}

fn count_object<const D: usize, Size: GridSize<D>>(
    _game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
    object: ObjectId,
) -> u32 {
    state.object_count(object)
}

fn build_patch<const D: usize>(
    rule: &GridRule<D>,
    placement: &GridMatchPlacement<D>,
) -> Result<GridPatch<D>, GridTransitionError<D>> {
    let mut patch = GridPatch::<D>::new();
    for write in &rule.writes {
        match write.clone() {
            GridWriteOp::<D>::Add {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp::<D>::Add {
                    position: write_position(placement, component, offset)?.into(),
                    object,
                });
            }
            GridWriteOp::<D>::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp::<D>::Add {
                    position: write_position(placement, component, offset)?.into(),
                    object: bound_object(placement, binding)
                        .ok_or(GridTransitionError::<D>::UnboundObjectSet { binding })?,
                });
            }
            GridWriteOp::<D>::Remove {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp::<D>::Remove {
                    position: write_position(placement, component, offset)?.into(),
                    object,
                });
            }
            GridWriteOp::<D>::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp::<D>::Remove {
                    position: write_position(placement, component, offset)?.into(),
                    object: bound_object(placement, binding)
                        .ok_or(GridTransitionError::<D>::UnboundObjectSet { binding })?,
                });
            }
            GridWriteOp::<D>::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                patch.push(PatchOp::<D>::Replace {
                    position: write_position(placement, component, offset)?.into(),
                    remove,
                    add,
                });
            }
            GridWriteOp::<D>::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                patch.push(PatchOp::<D>::Move {
                    from: write_position(placement, component, from_offset)?.into(),
                    to: write_position(placement, component, to_offset)?.into(),
                    object,
                });
            }
            GridWriteOp::<D>::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                patch.push(PatchOp::<D>::Move {
                    from: write_position(placement, component, from_offset)?.into(),
                    to: write_position(placement, component, to_offset)?.into(),
                    object: bound_object(placement, binding)
                        .ok_or(GridTransitionError::<D>::UnboundObjectSet { binding })?,
                });
            }
            GridWriteOp::<D>::SetMark {
                component,
                offset,
                object,
                mark,
                value,
            } => {
                patch.push(PatchOp::<D>::SetMark {
                    position: write_position(placement, component, offset)?.into(),
                    object,
                    mark,
                    value,
                });
            }
            GridWriteOp::<D>::SetObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
            } => {
                patch.push(PatchOp::<D>::SetMark {
                    position: write_position(placement, component, offset)?.into(),
                    object: bound_object(placement, binding)
                        .ok_or(GridTransitionError::<D>::UnboundObjectSet { binding })?,
                    mark,
                    value,
                });
            }
            GridWriteOp::<D>::RemoveMark {
                component,
                offset,
                object,
                mark,
                value,
                match_value,
            } => {
                patch.push(PatchOp::<D>::RemoveMark {
                    position: write_position(placement, component, offset)?.into(),
                    object,
                    mark,
                    value,
                    match_value,
                });
            }
            GridWriteOp::<D>::RemoveObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
                match_value,
            } => {
                patch.push(PatchOp::<D>::RemoveMark {
                    position: write_position(placement, component, offset)?.into(),
                    object: bound_object(placement, binding)
                        .ok_or(GridTransitionError::<D>::UnboundObjectSet { binding })?,
                    mark,
                    value,
                    match_value,
                });
            }
        }
    }
    for effect in &rule.effects {
        match effect {
            RuleEffect::ObserveMatch => {}
            RuleEffect::UpdateVariable {
                variable,
                op,
                value,
            } => {
                patch.push(PatchOp::<D>::UpdateVariable {
                    variable: *variable,
                    op: *op,
                    value: *value,
                });
            }
            RuleEffect::Cancel
            | RuleEffect::Win
            | RuleEffect::Restart
            | RuleEffect::NextLevel
            | RuleEffect::Again
            | RuleEffect::Checkpoint
            | RuleEffect::ClearCheckpoint => {}
        }
    }
    Ok(patch)
}

fn rule_cancels<const D: usize>(rule: &GridRule<D>) -> bool {
    rule.effects
        .iter()
        .any(|effect| matches!(effect, RuleEffect::Cancel))
}

fn rule_has_observable_effect<const D: usize>(rule: &GridRule<D>) -> bool {
    rule.effects
        .iter()
        .any(|effect| !matches!(effect, RuleEffect::UpdateVariable { .. }))
}

fn write_position<const D: usize>(
    placement: &GridMatchPlacement<D>,
    component: u16,
    offset: crate::GridOffset<D>,
) -> Result<GridCoord<D>, GridTransitionError<D>> {
    write_position_shared(
        placement,
        component,
        &offset,
        |offset, gaps| resolve_grid_offset(offset, gaps),
        || GridTransitionError::<D>::OffsetOutOfBounds,
    )
}

fn resolve_grid_offset<const D: usize>(
    offset: &crate::GridOffset<D>,
    gaps: &[u16],
) -> Option<CoordOffset<D>> {
    resolve_offset(offset, gaps)
}

fn resolve_offset<const D: usize>(
    offset: &crate::GridOffset<D>,
    gaps: &[u16],
) -> Option<CoordOffset<D>> {
    match offset {
        crate::GridOffset::Fixed { delta } => Some(CoordOffset::new(delta.axes())),
        crate::GridOffset::Variable { base, gap_terms } => {
            let mut deltas = base.axes().map(i32::from);
            for term in gap_terms {
                let gap = i32::from(*gaps.get(usize::from(term.gap_index))?);
                for (delta, term_delta) in deltas.iter_mut().zip(term.delta.axes()) {
                    *delta += i32::from(term_delta) * gap;
                }
            }
            let mut resolved = [0; D];
            for (target, delta) in resolved.iter_mut().zip(deltas) {
                *target = i16::try_from(delta).ok()?;
            }
            Some(CoordOffset::new(resolved))
        }
    }
}

fn once_all_patch_became_stale<const D: usize>(error: &GridPatchError<D>) -> bool {
    matches!(
        error,
        GridPatchError::<D>::ExpectedObject { .. } | GridPatchError::<D>::LayerOccupied { .. }
    )
}

fn offset_pos<const D: usize>(
    origin: GridCoord<D>,
    offset: CoordOffset<D>,
) -> Option<GridCoord<D>> {
    origin.checked_offset(offset)
}
