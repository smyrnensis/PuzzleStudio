use super::{ProgramApplyOutcome, ProgramBackend, ProgramStep, RuleApplication};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramStateKey(Vec<u64>);

impl ProgramStateKey {
    pub fn from_words(words: Vec<u64>) -> Self {
        Self(words)
    }
}

#[derive(Clone, Debug)]
pub struct ProgramContinuation {
    steps: Vec<ContinuationStep>,
}

#[derive(Clone, Debug)]
enum ContinuationStep {
    Step(ProgramPosition),
    LocalFrame {
        owner: ProgramPosition,
        continuation: ProgramContinuation,
    },
    AfterTriggered {
        owner: ProgramPosition,
        continuation: ProgramContinuation,
        fired_so_far: bool,
    },
    UntilStable {
        owner: ProgramPosition,
        before: ProgramStateKey,
        seen: Vec<ProgramStateKey>,
        fired_any: bool,
        pass_fired: bool,
        repeat_count: usize,
        remaining_pass: ProgramContinuation,
    },
}

#[derive(Clone, Debug, Default)]
struct ProgramBlockPath(Vec<ProgramBlockEdge>);

#[derive(Clone, Debug)]
struct ProgramBlockEdge {
    parent_index: usize,
    child: ProgramChildBlock,
}

#[derive(Clone, Copy, Debug)]
enum ProgramChildBlock {
    Conditional,
    Then,
    Else,
    Block,
    AfterBody,
    AfterThen,
    LocalFrame,
}

#[derive(Clone, Debug)]
struct ProgramPosition {
    block: ProgramBlockPath,
    index: usize,
}

impl ProgramBlockPath {
    fn child(&self, parent_index: usize, child: ProgramChildBlock) -> Self {
        let mut edges = self.0.clone();
        edges.push(ProgramBlockEdge {
            parent_index,
            child,
        });
        Self(edges)
    }
}

impl ProgramContinuation {
    fn empty() -> Self {
        Self { steps: Vec::new() }
    }

    fn from_step(step: ContinuationStep) -> Self {
        Self { steps: vec![step] }
    }

    fn extend_positions(&mut self, block: &ProgramBlockPath, range: std::ops::Range<usize>) {
        self.steps.extend(range.map(|index| {
            ContinuationStep::Step(ProgramPosition {
                block: block.clone(),
                index,
            })
        }));
    }

    fn extend_continuation_steps(&mut self, steps: &[ContinuationStep]) {
        self.steps.extend_from_slice(steps);
    }
}

#[derive(Clone, Debug)]
pub struct ProgramSegment {
    pub outcome: ProgramApplyOutcome,
    pub continuation: Option<ProgramContinuation>,
}

impl ProgramSegment {
    fn idle() -> Self {
        Self {
            outcome: ProgramApplyOutcome::default(),
            continuation: None,
        }
    }

    fn from_outcome(outcome: ProgramApplyOutcome) -> Self {
        Self {
            outcome,
            continuation: None,
        }
    }

    fn merge(&mut self, other: Self) {
        self.outcome.merge(other.outcome);
        if other.continuation.is_some() {
            self.continuation = other.continuation;
        }
    }
}

pub fn execute_program_segment<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    steps: &[ProgramStep<Rule, Condition, Frame>],
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    execute_block_once(
        backend,
        state,
        steps,
        steps,
        &ProgramBlockPath::default(),
        frame,
        repeat_limit,
        should_stop,
    )
}

pub fn resume_program_segment<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    steps: &[ProgramStep<Rule, Condition, Frame>],
    continuation: &ProgramContinuation,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    execute_continuation(
        backend,
        state,
        steps,
        continuation,
        frame,
        repeat_limit,
        should_stop,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_block_once<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    steps: &[ProgramStep<Rule, Condition, Frame>],
    block: &ProgramBlockPath,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let mut segment = ProgramSegment::idle();
    for (index, step) in steps.iter().enumerate() {
        let position = ProgramPosition {
            block: block.clone(),
            index,
        };
        let mut next = execute_step(
            backend,
            state,
            root,
            step,
            &position,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(mut continuation) = next.continuation.take() {
            continuation.extend_positions(block, index + 1..steps.len());
            next.continuation = Some(continuation);
            segment.merge(next);
            return Ok(segment);
        }
        segment.merge(next);
        if segment.outcome.cancelled {
            break;
        }
    }
    Ok(segment)
}

#[allow(clippy::too_many_arguments)]
fn execute_step<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    step: &ProgramStep<Rule, Condition, Frame>,
    position: &ProgramPosition,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    match step {
        ProgramStep::Rule(rule) => {
            let outcome = backend.apply_rule(state, rule, frame)?;
            if outcome.fired && !outcome.cancelled && should_stop(state, backend) {
                return Ok(ProgramSegment {
                    outcome,
                    continuation: Some(ProgramContinuation::empty()),
                });
            }
            Ok(ProgramSegment::from_outcome(outcome))
        }
        ProgramStep::ConditionalBlock { condition, steps } => {
            if !backend.condition_accepts(state, condition, frame) {
                return Ok(ProgramSegment::idle());
            }
            let child = position
                .block
                .child(position.index, ProgramChildBlock::Conditional);
            execute_block_once(
                backend,
                state,
                root,
                steps,
                &child,
                frame,
                repeat_limit,
                should_stop,
            )
        }
        ProgramStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            let (selected, child_kind) = if backend.condition_accepts(state, condition, frame) {
                (then_steps, ProgramChildBlock::Then)
            } else {
                (else_steps, ProgramChildBlock::Else)
            };
            let child = position.block.child(position.index, child_kind);
            execute_block_once(
                backend,
                state,
                root,
                selected,
                &child,
                frame,
                repeat_limit,
                should_stop,
            )
        }
        ProgramStep::Block {
            application,
            stop_condition,
            steps,
        } => {
            let child = position
                .block
                .child(position.index, ProgramChildBlock::Block);
            match application {
                RuleApplication::Once
                | RuleApplication::OnceAll
                | RuleApplication::OncePerLevel => execute_block_once(
                    backend,
                    state,
                    root,
                    steps,
                    &child,
                    frame,
                    repeat_limit,
                    should_stop,
                ),
                RuleApplication::Random => execute_random(
                    backend,
                    state,
                    root,
                    steps,
                    &child,
                    frame,
                    repeat_limit,
                    should_stop,
                ),
                RuleApplication::UntilStable => execute_until_stable(
                    backend,
                    state,
                    root,
                    position,
                    stop_condition.as_ref(),
                    steps,
                    &child,
                    frame,
                    repeat_limit,
                    should_stop,
                ),
            }
        }
        ProgramStep::AfterTriggered { steps, then_steps } => {
            let body = position
                .block
                .child(position.index, ProgramChildBlock::AfterBody);
            let mut segment = execute_block_once(
                backend,
                state,
                root,
                steps,
                &body,
                frame,
                repeat_limit,
                should_stop,
            )?;
            if let Some(continuation) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        owner: position.clone(),
                        continuation,
                        fired_so_far: segment.outcome.fired,
                    },
                ));
                return Ok(segment);
            }
            if segment.outcome.fired && !segment.outcome.cancelled {
                let then_block = position
                    .block
                    .child(position.index, ProgramChildBlock::AfterThen);
                let then_segment = execute_block_once(
                    backend,
                    state,
                    root,
                    then_steps,
                    &then_block,
                    frame,
                    repeat_limit,
                    should_stop,
                )?;
                segment.merge(then_segment);
            }
            Ok(segment)
        }
        ProgramStep::LocalFrame {
            frame: local_frame,
            steps,
        } => {
            let child = position
                .block
                .child(position.index, ProgramChildBlock::LocalFrame);
            let mut segment = execute_block_once(
                backend,
                state,
                root,
                steps,
                &child,
                Some(local_frame),
                repeat_limit,
                should_stop,
            )?;
            if let Some(continuation) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::LocalFrame {
                        owner: position.clone(),
                        continuation,
                    },
                ));
            }
            Ok(segment)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_random<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    steps: &[ProgramStep<Rule, Condition, Frame>],
    block: &ProgramBlockPath,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let base_state = state.clone();
    let base_snapshot = backend.checkpoint();
    let mut candidates = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        *state = base_state.clone();
        backend.restore(&base_snapshot);
        let position = ProgramPosition {
            block: block.clone(),
            index,
        };
        let outcome = match execute_step_to_completion(
            backend,
            state,
            root,
            step,
            &position,
            frame,
            repeat_limit,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                *state = base_state;
                backend.restore(&base_snapshot);
                return Err(error);
            }
        };
        if outcome.fired {
            candidates.push((state.clone(), backend.checkpoint(), outcome));
        }
    }
    *state = base_state;
    backend.restore(&base_snapshot);
    if candidates.is_empty() {
        return Ok(ProgramSegment::idle());
    }
    let index = backend.choose_random(state, candidates.len());
    let (selected_state, selected_snapshot, outcome) = candidates.swap_remove(index);
    *state = selected_state;
    backend.restore(&selected_snapshot);
    let continuation = (outcome.fired && !outcome.cancelled && should_stop(state, backend))
        .then(ProgramContinuation::empty);
    Ok(ProgramSegment {
        outcome,
        continuation,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_step_to_completion<Rule, Condition, Frame, State, Backend>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    step: &ProgramStep<Rule, Condition, Frame>,
    position: &ProgramPosition,
    frame: Option<&Frame>,
    repeat_limit: usize,
) -> Result<ProgramApplyOutcome, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
{
    let mut never_stop = |_: &State, _: &Backend| false;
    execute_step(
        backend,
        state,
        root,
        step,
        position,
        frame,
        repeat_limit,
        &mut never_stop,
    )
    .map(|segment| segment.outcome)
}

#[allow(clippy::too_many_arguments)]
fn execute_until_stable<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    owner: &ProgramPosition,
    stop_condition: Option<&Condition>,
    steps: &[ProgramStep<Rule, Condition, Frame>],
    block: &ProgramBlockPath,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let mut seen = vec![backend.state_key(state)];
    let mut fired_any = false;
    let mut repeat_count = 0;
    loop {
        if stop_condition
            .is_some_and(|condition| backend.condition_accepts(state, condition, frame))
        {
            break;
        }
        let before = backend.state_key(state);
        let mut pass = execute_block_once(
            backend,
            state,
            root,
            steps,
            block,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass.continuation.take() {
            return Ok(ProgramSegment {
                outcome: pass.outcome,
                continuation: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        owner: owner.clone(),
                        before,
                        seen,
                        fired_any,
                        pass_fired: pass.outcome.fired,
                        repeat_count,
                        remaining_pass,
                    },
                )),
            });
        }
        if pass.outcome.cancelled {
            return Ok(pass);
        }
        if !pass.outcome.fired {
            break;
        }
        fired_any = true;
        let current = backend.state_key(state);
        if current == before || seen.contains(&current) {
            break;
        }
        seen.push(current);
        repeat_count += 1;
        if repeat_count >= repeat_limit {
            break;
        }
    }
    Ok(ProgramSegment::from_outcome(ProgramApplyOutcome {
        fired: fired_any,
        cancelled: false,
    }))
}

#[allow(clippy::too_many_arguments)]
fn execute_continuation<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    continuation: &ProgramContinuation,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let mut segment = ProgramSegment::idle();
    for (index, step) in continuation.steps.iter().enumerate() {
        let mut next = execute_continuation_step(
            backend,
            state,
            root,
            step,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(mut remaining) = next.continuation.take() {
            remaining.extend_continuation_steps(&continuation.steps[index + 1..]);
            next.continuation = Some(remaining);
            segment.merge(next);
            return Ok(segment);
        }
        segment.merge(next);
        if segment.outcome.cancelled {
            break;
        }
    }
    Ok(segment)
}

#[allow(clippy::too_many_arguments)]
fn execute_continuation_step<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    step: &ContinuationStep,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    match step {
        ContinuationStep::Step(position) => {
            let Some(step) = resolve_step(root, position) else {
                return Err(backend.invalid_program_continuation());
            };
            execute_step(
                backend,
                state,
                root,
                step,
                position,
                frame,
                repeat_limit,
                should_stop,
            )
        }
        ContinuationStep::LocalFrame {
            owner,
            continuation,
        } => {
            let Some(ProgramStep::LocalFrame {
                frame: local_frame, ..
            }) = resolve_step(root, owner)
            else {
                return Err(backend.invalid_program_continuation());
            };
            let mut segment = execute_continuation(
                backend,
                state,
                root,
                continuation,
                Some(local_frame),
                repeat_limit,
                should_stop,
            )?;
            if let Some(remaining) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::LocalFrame {
                        owner: owner.clone(),
                        continuation: remaining,
                    },
                ));
            }
            Ok(segment)
        }
        ContinuationStep::AfterTriggered {
            owner,
            continuation,
            fired_so_far,
        } => {
            let Some(ProgramStep::AfterTriggered { then_steps, .. }) = resolve_step(root, owner)
            else {
                return Err(backend.invalid_program_continuation());
            };
            let mut segment = execute_continuation(
                backend,
                state,
                root,
                continuation,
                frame,
                repeat_limit,
                should_stop,
            )?;
            if let Some(remaining) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        owner: owner.clone(),
                        continuation: remaining,
                        fired_so_far: *fired_so_far || segment.outcome.fired,
                    },
                ));
                return Ok(segment);
            }
            segment.outcome.fired |= *fired_so_far;
            if segment.outcome.fired && !segment.outcome.cancelled {
                let then_block = owner.block.child(owner.index, ProgramChildBlock::AfterThen);
                let then_segment = execute_block_once(
                    backend,
                    state,
                    root,
                    then_steps,
                    &then_block,
                    frame,
                    repeat_limit,
                    should_stop,
                )?;
                segment.merge(then_segment);
            }
            Ok(segment)
        }
        ContinuationStep::UntilStable {
            owner,
            before,
            seen,
            fired_any,
            pass_fired,
            repeat_count,
            remaining_pass,
        } => resume_until_stable(
            backend,
            state,
            root,
            owner,
            before,
            seen.clone(),
            *fired_any,
            *pass_fired,
            *repeat_count,
            remaining_pass,
            frame,
            repeat_limit,
            should_stop,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn resume_until_stable<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    root: &[ProgramStep<Rule, Condition, Frame>],
    owner: &ProgramPosition,
    before: &ProgramStateKey,
    mut seen: Vec<ProgramStateKey>,
    mut fired_any: bool,
    pass_fired_before_pause: bool,
    mut repeat_count: usize,
    remaining_pass: &ProgramContinuation,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let Some(ProgramStep::Block {
        application: RuleApplication::UntilStable,
        stop_condition,
        steps,
    }) = resolve_step(root, owner)
    else {
        return Err(backend.invalid_program_continuation());
    };
    let block = owner.block.child(owner.index, ProgramChildBlock::Block);
    let mut remaining = execute_continuation(
        backend,
        state,
        root,
        remaining_pass,
        frame,
        repeat_limit,
        should_stop,
    )?;
    if let Some(next_remaining) = remaining.continuation.take() {
        let pass_fired = pass_fired_before_pause || remaining.outcome.fired;
        return Ok(ProgramSegment {
            outcome: ProgramApplyOutcome {
                fired: pass_fired,
                cancelled: false,
            },
            continuation: Some(ProgramContinuation::from_step(
                ContinuationStep::UntilStable {
                    owner: owner.clone(),
                    before: before.clone(),
                    seen,
                    fired_any,
                    pass_fired,
                    repeat_count,
                    remaining_pass: next_remaining,
                },
            )),
        });
    }
    if remaining.outcome.cancelled {
        return Ok(remaining);
    }

    let pass_fired = pass_fired_before_pause || remaining.outcome.fired;
    if !pass_fired {
        return Ok(ProgramSegment::from_outcome(ProgramApplyOutcome {
            fired: fired_any,
            cancelled: false,
        }));
    }
    fired_any = true;
    let current = backend.state_key(state);
    if &current == before || seen.contains(&current) {
        return Ok(ProgramSegment::from_outcome(ProgramApplyOutcome {
            fired: true,
            cancelled: false,
        }));
    }
    seen.push(current);
    repeat_count += 1;
    if repeat_count >= repeat_limit {
        return Ok(ProgramSegment::from_outcome(ProgramApplyOutcome {
            fired: true,
            cancelled: false,
        }));
    }

    loop {
        if stop_condition
            .as_ref()
            .is_some_and(|condition| backend.condition_accepts(state, condition, frame))
        {
            break;
        }
        let before = backend.state_key(state);
        let mut pass = execute_block_once(
            backend,
            state,
            root,
            steps,
            &block,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass.continuation.take() {
            return Ok(ProgramSegment {
                outcome: pass.outcome,
                continuation: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        owner: owner.clone(),
                        before,
                        seen,
                        fired_any,
                        pass_fired: pass.outcome.fired,
                        repeat_count,
                        remaining_pass,
                    },
                )),
            });
        }
        if pass.outcome.cancelled {
            return Ok(pass);
        }
        if !pass.outcome.fired {
            break;
        }
        fired_any = true;
        let current = backend.state_key(state);
        if current == before || seen.contains(&current) {
            break;
        }
        seen.push(current);
        repeat_count += 1;
        if repeat_count >= repeat_limit {
            break;
        }
    }
    Ok(ProgramSegment::from_outcome(ProgramApplyOutcome {
        fired: fired_any,
        cancelled: false,
    }))
}

fn resolve_step<'a, Rule, Condition, Frame>(
    root: &'a [ProgramStep<Rule, Condition, Frame>],
    position: &ProgramPosition,
) -> Option<&'a ProgramStep<Rule, Condition, Frame>> {
    resolve_block(root, &position.block)?.get(position.index)
}

fn resolve_block<'a, Rule, Condition, Frame>(
    root: &'a [ProgramStep<Rule, Condition, Frame>],
    path: &ProgramBlockPath,
) -> Option<&'a [ProgramStep<Rule, Condition, Frame>]> {
    let mut block = root;
    for edge in &path.0 {
        let parent = block.get(edge.parent_index)?;
        block = match (edge.child, parent) {
            (ProgramChildBlock::Conditional, ProgramStep::ConditionalBlock { steps, .. }) => steps,
            (ProgramChildBlock::Then, ProgramStep::ConditionalBranch { then_steps, .. }) => {
                then_steps
            }
            (ProgramChildBlock::Else, ProgramStep::ConditionalBranch { else_steps, .. }) => {
                else_steps
            }
            (ProgramChildBlock::Block, ProgramStep::Block { steps, .. }) => steps,
            (ProgramChildBlock::AfterBody, ProgramStep::AfterTriggered { steps, .. }) => steps,
            (ProgramChildBlock::AfterThen, ProgramStep::AfterTriggered { then_steps, .. }) => {
                then_steps
            }
            (ProgramChildBlock::LocalFrame, ProgramStep::LocalFrame { steps, .. }) => steps,
            _ => return None,
        };
    }
    Some(block)
}
