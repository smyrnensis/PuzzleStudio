use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{ProgramApplyOutcome, ProgramBackend, ProgramStep, RuleApplication};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramStateKey(Vec<u64>);

impl ProgramStateKey {
    pub fn from_words(words: Vec<u64>) -> Self {
        Self(words)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProgramNodeId(u32);

#[derive(Clone, Debug)]
enum ExecutableNode<Rule, Condition, Frame> {
    Rule(Rule),
    ConditionalBlock {
        condition: Condition,
        steps: Vec<ProgramNodeId>,
    },
    ConditionalBranch {
        condition: Condition,
        then_steps: Vec<ProgramNodeId>,
        else_steps: Vec<ProgramNodeId>,
    },
    Block {
        application: RuleApplication,
        stop_condition: Option<Condition>,
        steps: Vec<ProgramNodeId>,
    },
    AfterTriggered {
        steps: Vec<ProgramNodeId>,
        then_steps: Vec<ProgramNodeId>,
    },
    LocalFrame {
        frame: Frame,
        steps: Vec<ProgramNodeId>,
    },
}

/// Canonical runtime form of a program.
///
/// `ProgramStep` remains the nested semantic product used by analysis and
/// serialization. This product assigns every executable node a stable identity
/// once, so suspension and resumption never reconstruct a source-tree path.
#[derive(Clone, Debug)]
pub struct ExecutableProgram<Rule, Condition, Frame> {
    source: Vec<ProgramStep<Rule, Condition, Frame>>,
    roots: Vec<ProgramNodeId>,
    nodes: Vec<ExecutableNode<Rule, Condition, Frame>>,
}

impl<Rule, Condition, Frame> Default for ExecutableProgram<Rule, Condition, Frame> {
    fn default() -> Self {
        Self {
            source: Vec::new(),
            roots: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

impl<Rule, Condition, Frame> ExecutableProgram<Rule, Condition, Frame>
where
    Rule: Clone,
    Condition: Clone,
    Frame: Clone,
{
    pub fn new(source: Vec<ProgramStep<Rule, Condition, Frame>>) -> Self {
        let mut nodes = Vec::new();
        let roots = compile_block(&source, &mut nodes);
        Self {
            source,
            roots,
            nodes,
        }
    }
}

impl<Rule, Condition, Frame> ExecutableProgram<Rule, Condition, Frame> {
    pub fn as_steps(&self) -> &[ProgramStep<Rule, Condition, Frame>] {
        &self.source
    }

    pub fn rule_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, ExecutableNode::Rule(_)))
            .count()
    }

    fn node(&self, id: ProgramNodeId) -> Option<&ExecutableNode<Rule, Condition, Frame>> {
        self.nodes.get(id.0 as usize)
    }
}

impl<Rule, Condition, Frame> Deref for ExecutableProgram<Rule, Condition, Frame> {
    type Target = [ProgramStep<Rule, Condition, Frame>];

    fn deref(&self) -> &Self::Target {
        self.as_steps()
    }
}

impl<Rule, Condition, Frame> PartialEq for ExecutableProgram<Rule, Condition, Frame>
where
    ProgramStep<Rule, Condition, Frame>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl<Rule, Condition, Frame> Eq for ExecutableProgram<Rule, Condition, Frame> where
    ProgramStep<Rule, Condition, Frame>: Eq
{
}

impl<Rule, Condition, Frame> Serialize for ExecutableProgram<Rule, Condition, Frame>
where
    ProgramStep<Rule, Condition, Frame>: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.source.serialize(serializer)
    }
}

impl<'de, Rule, Condition, Frame> Deserialize<'de> for ExecutableProgram<Rule, Condition, Frame>
where
    Rule: Clone + Deserialize<'de>,
    Condition: Clone + Deserialize<'de>,
    Frame: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<ProgramStep<Rule, Condition, Frame>>::deserialize(deserializer).map(Self::new)
    }
}

fn compile_block<Rule: Clone, Condition: Clone, Frame: Clone>(
    steps: &[ProgramStep<Rule, Condition, Frame>],
    nodes: &mut Vec<ExecutableNode<Rule, Condition, Frame>>,
) -> Vec<ProgramNodeId> {
    steps
        .iter()
        .map(|step| {
            let node = match step {
                ProgramStep::Rule(rule) => ExecutableNode::Rule(rule.clone()),
                ProgramStep::ConditionalBlock { condition, steps } => {
                    ExecutableNode::ConditionalBlock {
                        condition: condition.clone(),
                        steps: compile_block(steps, nodes),
                    }
                }
                ProgramStep::ConditionalBranch {
                    condition,
                    then_steps,
                    else_steps,
                } => ExecutableNode::ConditionalBranch {
                    condition: condition.clone(),
                    then_steps: compile_block(then_steps, nodes),
                    else_steps: compile_block(else_steps, nodes),
                },
                ProgramStep::Block {
                    application,
                    stop_condition,
                    steps,
                } => ExecutableNode::Block {
                    application: *application,
                    stop_condition: stop_condition.clone(),
                    steps: compile_block(steps, nodes),
                },
                ProgramStep::AfterTriggered { steps, then_steps } => {
                    ExecutableNode::AfterTriggered {
                        steps: compile_block(steps, nodes),
                        then_steps: compile_block(then_steps, nodes),
                    }
                }
                ProgramStep::LocalFrame { frame, steps } => ExecutableNode::LocalFrame {
                    frame: frame.clone(),
                    steps: compile_block(steps, nodes),
                },
            };
            let index = nodes.len();
            let id = ProgramNodeId(
                index
                    .try_into()
                    .expect("executable program exceeds u32 node capacity"),
            );
            nodes.push(node);
            id
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct ProgramContinuation {
    steps: Vec<ContinuationStep>,
}

#[derive(Clone, Debug)]
enum ContinuationStep {
    Step(ProgramNodeId),
    LocalFrame {
        owner: ProgramNodeId,
        continuation: ProgramContinuation,
    },
    AfterTriggered {
        owner: ProgramNodeId,
        continuation: ProgramContinuation,
        fired_so_far: bool,
    },
    UntilStable {
        owner: ProgramNodeId,
        before: ProgramStateKey,
        seen: Vec<ProgramStateKey>,
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

    fn extend_nodes(&mut self, nodes: &[ProgramNodeId]) {
        self.steps
            .extend(nodes.iter().copied().map(ContinuationStep::Step));
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
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
        program,
        &program.roots,
        frame,
        repeat_limit,
        should_stop,
    )
}

pub fn resume_program_segment<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    program: &ExecutableProgram<Rule, Condition, Frame>,
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
        program,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
    nodes: &[ProgramNodeId],
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
    for (index, node) in nodes.iter().copied().enumerate() {
        let mut next = execute_node(
            backend,
            state,
            program,
            node,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(mut continuation) = next.continuation.take() {
            continuation.extend_nodes(&nodes[index + 1..]);
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
fn execute_node<Rule, Condition, Frame, State, Backend, Stop>(
    backend: &mut Backend,
    state: &mut State,
    program: &ExecutableProgram<Rule, Condition, Frame>,
    node_id: ProgramNodeId,
    frame: Option<&Frame>,
    repeat_limit: usize,
    should_stop: &mut Stop,
) -> Result<ProgramSegment, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
    Stop: FnMut(&State, &Backend) -> bool,
{
    let Some(node) = program.node(node_id) else {
        return Err(backend.invalid_program_continuation());
    };
    match node {
        ExecutableNode::Rule(rule) => {
            let outcome = backend.apply_rule(state, rule, frame)?;
            if outcome.fired && !outcome.cancelled && should_stop(state, backend) {
                return Ok(ProgramSegment {
                    outcome,
                    continuation: Some(ProgramContinuation::empty()),
                });
            }
            Ok(ProgramSegment::from_outcome(outcome))
        }
        ExecutableNode::ConditionalBlock { condition, steps } => {
            if !backend.condition_accepts(state, condition, frame) {
                return Ok(ProgramSegment::idle());
            }
            execute_block_once(
                backend,
                state,
                program,
                steps,
                frame,
                repeat_limit,
                should_stop,
            )
        }
        ExecutableNode::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            let selected = if backend.condition_accepts(state, condition, frame) {
                then_steps
            } else {
                else_steps
            };
            execute_block_once(
                backend,
                state,
                program,
                selected,
                frame,
                repeat_limit,
                should_stop,
            )
        }
        ExecutableNode::Block {
            application,
            stop_condition,
            steps,
        } => match application {
            RuleApplication::Once | RuleApplication::OnceAll | RuleApplication::OncePerLevel => {
                execute_block_once(
                    backend,
                    state,
                    program,
                    steps,
                    frame,
                    repeat_limit,
                    should_stop,
                )
            }
            RuleApplication::Random => execute_random(
                backend,
                state,
                program,
                steps,
                frame,
                repeat_limit,
                should_stop,
            ),
            RuleApplication::UntilStable => execute_until_stable(
                backend,
                state,
                program,
                node_id,
                stop_condition.as_ref(),
                steps,
                frame,
                repeat_limit,
                should_stop,
            ),
        },
        ExecutableNode::AfterTriggered { steps, then_steps } => {
            let mut segment = execute_block_once(
                backend,
                state,
                program,
                steps,
                frame,
                repeat_limit,
                should_stop,
            )?;
            if let Some(continuation) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        owner: node_id,
                        continuation,
                        fired_so_far: segment.outcome.fired,
                    },
                ));
                return Ok(segment);
            }
            if segment.outcome.fired && !segment.outcome.cancelled {
                segment.merge(execute_block_once(
                    backend,
                    state,
                    program,
                    then_steps,
                    frame,
                    repeat_limit,
                    should_stop,
                )?);
            }
            Ok(segment)
        }
        ExecutableNode::LocalFrame {
            frame: local_frame,
            steps,
        } => {
            let mut segment = execute_block_once(
                backend,
                state,
                program,
                steps,
                Some(local_frame),
                repeat_limit,
                should_stop,
            )?;
            if let Some(continuation) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::LocalFrame {
                        owner: node_id,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
    nodes: &[ProgramNodeId],
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
    for node in nodes.iter().copied() {
        *state = base_state.clone();
        backend.restore(&base_snapshot);
        let outcome =
            match execute_node_to_completion(backend, state, program, node, frame, repeat_limit) {
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

fn execute_node_to_completion<Rule, Condition, Frame, State, Backend>(
    backend: &mut Backend,
    state: &mut State,
    program: &ExecutableProgram<Rule, Condition, Frame>,
    node: ProgramNodeId,
    frame: Option<&Frame>,
    repeat_limit: usize,
) -> Result<ProgramApplyOutcome, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
{
    let mut never_stop = |_: &State, _: &Backend| false;
    execute_node(
        backend,
        state,
        program,
        node,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
    owner: ProgramNodeId,
    stop_condition: Option<&Condition>,
    nodes: &[ProgramNodeId],
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
            program,
            nodes,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass.continuation.take() {
            return Ok(ProgramSegment {
                outcome: pass.outcome,
                continuation: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        owner,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
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
            program,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
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
        ContinuationStep::Step(node) => execute_node(
            backend,
            state,
            program,
            *node,
            frame,
            repeat_limit,
            should_stop,
        ),
        ContinuationStep::LocalFrame {
            owner,
            continuation,
        } => {
            let Some(ExecutableNode::LocalFrame {
                frame: local_frame, ..
            }) = program.node(*owner)
            else {
                return Err(backend.invalid_program_continuation());
            };
            let mut segment = execute_continuation(
                backend,
                state,
                program,
                continuation,
                Some(local_frame),
                repeat_limit,
                should_stop,
            )?;
            if let Some(remaining) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::LocalFrame {
                        owner: *owner,
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
            let Some(ExecutableNode::AfterTriggered { then_steps, .. }) = program.node(*owner)
            else {
                return Err(backend.invalid_program_continuation());
            };
            let mut segment = execute_continuation(
                backend,
                state,
                program,
                continuation,
                frame,
                repeat_limit,
                should_stop,
            )?;
            if let Some(remaining) = segment.continuation.take() {
                segment.continuation = Some(ProgramContinuation::from_step(
                    ContinuationStep::AfterTriggered {
                        owner: *owner,
                        continuation: remaining,
                        fired_so_far: *fired_so_far || segment.outcome.fired,
                    },
                ));
                return Ok(segment);
            }
            segment.outcome.fired |= *fired_so_far;
            if segment.outcome.fired && !segment.outcome.cancelled {
                segment.merge(execute_block_once(
                    backend,
                    state,
                    program,
                    then_steps,
                    frame,
                    repeat_limit,
                    should_stop,
                )?);
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
            program,
            *owner,
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
    program: &ExecutableProgram<Rule, Condition, Frame>,
    owner: ProgramNodeId,
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
    let Some(ExecutableNode::Block {
        application: RuleApplication::UntilStable,
        stop_condition,
        steps,
    }) = program.node(owner)
    else {
        return Err(backend.invalid_program_continuation());
    };
    let mut remaining = execute_continuation(
        backend,
        state,
        program,
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
                    owner,
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
            program,
            steps,
            frame,
            repeat_limit,
            should_stop,
        )?;
        if let Some(remaining_pass) = pass.continuation.take() {
            return Ok(ProgramSegment {
                outcome: pass.outcome,
                continuation: Some(ProgramContinuation::from_step(
                    ContinuationStep::UntilStable {
                        owner,
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
