use crate::{
    CompiledGame3, ConditionId3, Coord3, InputId, LayerId, MarkId3, ObjectId, Offset3, Patch3,
    PatchError3, PatchOp3, RuleId3, State3, StateError3, VariableId,
};
#[cfg(test)]
use puzzle_kernel::VariableUpdateOp;
use puzzle_kernel::{
    ComparisonOp, ComponentPlacement, FnvBuilder, GridOffset, LocalFrame, MarkValueMatch,
    MatchPlacement, ObjectBinding, ProgramApplyOutcome, ProgramBackend, ProgramStep,
    RuleApplication, TransitionOutcome as KernelTransitionOutcome, bind_object,
    bound_object as bound_object_in_bindings,
    collect_component_placements as collect_component_placements_shared,
    complete_component_placements as complete_component_placements_shared, fnv_mix,
    placement_object_binding, write_position as write_position_shared,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type ConditionValueKind3 = puzzle_kernel::ConditionValueKind<ObjectId, Pattern3, InputId>;
pub type ConditionDef3 = puzzle_kernel::RuleConditionDef<ConditionId3, ConditionValueKind3>;
pub type Guard3 = puzzle_kernel::RuleGuard<VariableId, ConditionId3, ConditionValueKind3, InputId>;
pub type MarkPattern3 = puzzle_kernel::RuleMarkPattern<ObjectId, MarkId3>;
pub type ObjectSetMatcher3 = puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>;
pub type ObjectSetMarkPattern3 = puzzle_kernel::ObjectSetMarkPattern<MarkId3>;
pub type PatternComponent3 = puzzle_kernel::RulePatternComponent<MatchCell3>;
pub type MatchCell3 = puzzle_kernel::RuleMatchCell<Offset3, ObjectId, LayerId, MarkId3>;
pub type Rule3 = puzzle_kernel::RuleModel<RuleId3, Guard3, Pattern3, WriteOp3, RuleEffect3>;
pub type RuleApplication3 = RuleApplication;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleEffect3 {
    Cancel,
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    UpdateVariable {
        variable: VariableId,
        op: puzzle_kernel::VariableUpdateOp,
        value: i64,
    },
}
pub type RuleStep3 = ProgramStep<Rule3, RuleCondition3, LocalFrame<ObjectId>>;
pub type WriteOp3 = puzzle_kernel::RuleWriteOp<Offset3, ObjectId, MarkId3>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionCommand3 {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
}
pub type TransitionOutcome3 =
    KernelTransitionOutcome<Option<InputId>, State3, TransitionCommand3, RuleId3, Patch3>;

const UNTIL_STABLE_REPEAT_LIMIT3: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCondition3 {
    AnyMatches(Vec<Pattern3>),
    NoMatches(Vec<Pattern3>),
    AnyInputMatches(Vec<(InputId, Pattern3)>),
    NoInputMatches(Vec<(InputId, Pattern3)>),
    GuardBranches(Vec<Vec<Guard3>>),
}

pub fn flattened_rules(program: &[RuleStep3]) -> Vec<Rule3> {
    fn collect(steps: &[RuleStep3], out: &mut Vec<Rule3>) {
        for step in steps {
            match step {
                RuleStep3::Rule(rule) => out.push(rule.clone()),
                RuleStep3::ConditionalBlock { steps, .. }
                | RuleStep3::Block { steps, .. }
                | RuleStep3::LocalFrame { steps, .. } => collect(steps, out),
                RuleStep3::ConditionalBranch {
                    then_steps,
                    else_steps,
                    ..
                } => {
                    collect(then_steps, out);
                    collect(else_steps, out);
                }
                RuleStep3::AfterTriggered { steps, then_steps } => {
                    collect(steps, out);
                    collect(then_steps, out);
                }
            }
        }
    }
    let mut rules = Vec::new();
    collect(program, &mut rules);
    rules
}

#[derive(Clone, Default)]
struct TransitionTrace3 {
    fired_rules: Vec<RuleId3>,
    patches: Vec<Patch3>,
    commands: Vec<TransitionCommand3>,
    cancelled: bool,
}

impl TransitionTrace3 {
    fn record(&mut self, rule: &Rule3, patch: Patch3) {
        self.fired_rules.push(rule.id);
        self.patches.push(patch);
        if rule
            .effects
            .iter()
            .any(|effect| matches!(effect, RuleEffect3::Cancel))
        {
            self.cancelled = true;
            return;
        }
        for effect in &rule.effects {
            match effect {
                RuleEffect3::Cancel => unreachable!("cancel handled before command emission"),
                RuleEffect3::Win => self.commands.push(TransitionCommand3::Win),
                RuleEffect3::Restart => self.commands.push(TransitionCommand3::Restart),
                RuleEffect3::NextLevel => self.commands.push(TransitionCommand3::NextLevel),
                RuleEffect3::Again => self.commands.push(TransitionCommand3::Again),
                RuleEffect3::Checkpoint => self.commands.push(TransitionCommand3::Checkpoint),
                RuleEffect3::ClearCheckpoint => {
                    self.commands.push(TransitionCommand3::ClearCheckpoint)
                }
                RuleEffect3::UpdateVariable { .. } => {}
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern3 {
    pub components: Vec<PatternComponent3>,
}

impl Pattern3 {
    pub fn new(cells: Vec<MatchCell3>) -> Self {
        Self::from_components(vec![PatternComponent3::new(cells)])
    }

    pub fn from_components(components: Vec<PatternComponent3>) -> Self {
        Self { components }
    }

    pub fn cells(&self) -> Vec<&MatchCell3> {
        self.components
            .iter()
            .flat_map(|component| &component.cells)
            .collect()
    }

    pub fn components(&self) -> &[PatternComponent3] {
        &self.components
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError3 {
    Patch(PatchError3),
    OffsetOutOfBounds,
    UnboundObjectSet { binding: u16 },
}

impl From<PatchError3> for TransitionError3 {
    fn from(value: PatchError3) -> Self {
        Self::Patch(value)
    }
}

pub fn transition_once(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut trace = TransitionTrace3::default();
    let mut next = transition_rule_once(game, &scoped, rule, None, None, &mut trace)?;
    next.clear_mark();
    Ok(next)
}

pub fn transition_once_with_input(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: InputId,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut trace = TransitionTrace3::default();
    let mut next = transition_rule_once(game, &scoped, rule, Some(input), None, &mut trace)?;
    next.clear_mark();
    Ok(next)
}

pub fn transition_program(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    input: InputId,
) -> Result<State3, TransitionError3> {
    transition_program_outcome_with_local_frame(game, state, program, input, None)
        .map(|outcome| outcome.next_state)
}

pub fn transition_program_with_local_frame(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    input: InputId,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    transition_program_outcome_with_local_frame(game, state, program, input, local_frame)
        .map(|outcome| outcome.next_state)
}

pub fn transition_program_outcome(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    input: InputId,
) -> Result<TransitionOutcome3, TransitionError3> {
    transition_program_outcome_with_local_frame(game, state, program, input, None)
}

pub fn transition_program_outcome_with_local_frame(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    input: InputId,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<TransitionOutcome3, TransitionError3> {
    transition_program_outcome_inner(game, state, program, Some(input), local_frame)
}

fn transition_program_outcome_inner(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<TransitionOutcome3, TransitionError3> {
    let mut current = state.clone();
    current.clear_mark();
    let mut trace = TransitionTrace3::default();
    current = transition_program_steps(game, &current, program, input, local_frame, &mut trace)?;
    current.clear_mark();
    Ok(TransitionOutcome3 {
        input,
        next_state: current,
        cancelled: trace.cancelled,
        commands: trace.commands,
        fired_rules: trace.fired_rules,
        patches: trace.patches,
    })
}

pub fn transition_program_without_input(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
) -> Result<State3, TransitionError3> {
    transition_program_without_input_outcome_with_local_frame(game, state, program, None)
        .map(|outcome| outcome.next_state)
}

pub fn transition_program_without_input_with_local_frame(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    transition_program_without_input_outcome_with_local_frame(game, state, program, local_frame)
        .map(|outcome| outcome.next_state)
}

pub fn transition_program_without_input_outcome(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
) -> Result<TransitionOutcome3, TransitionError3> {
    transition_program_without_input_outcome_with_local_frame(game, state, program, None)
}

pub fn transition_program_without_input_outcome_with_local_frame(
    game: &CompiledGame3,
    state: &State3,
    program: &[RuleStep3],
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<TransitionOutcome3, TransitionError3> {
    transition_program_outcome_inner(game, state, program, None, local_frame)
}

fn transition_program_steps(
    game: &CompiledGame3,
    state: &State3,
    steps: &[RuleStep3],
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    let mut backend = ProgramBackend3 { game, input, trace };
    puzzle_kernel::execute_program(
        &mut backend,
        &mut current,
        steps,
        local_frame,
        UNTIL_STABLE_REPEAT_LIMIT3,
    )?;
    Ok(current)
}

struct ProgramBackend3<'a> {
    game: &'a CompiledGame3,
    input: Option<InputId>,
    trace: &'a mut TransitionTrace3,
}

impl ProgramBackend<Rule3, RuleCondition3, LocalFrame<ObjectId>, State3> for ProgramBackend3<'_> {
    type Error = TransitionError3;
    type Snapshot = TransitionTrace3;

    fn condition_accepts(
        &mut self,
        state: &State3,
        condition: &RuleCondition3,
        frame: Option<&LocalFrame<ObjectId>>,
    ) -> bool {
        rule_condition_accepts(self.game, state, condition, self.input, frame)
    }

    fn apply_rule(
        &mut self,
        state: &mut State3,
        rule: &Rule3,
        frame: Option<&LocalFrame<ObjectId>>,
    ) -> Result<ProgramApplyOutcome, Self::Error> {
        let fired_before = self.trace.fired_rules.len();
        let cancelled_before = self.trace.cancelled;
        *state =
            transition_rule_by_application(self.game, state, rule, self.input, frame, self.trace)?;
        Ok(ProgramApplyOutcome {
            fired: self.trace.fired_rules.len() > fired_before,
            cancelled: !cancelled_before && self.trace.cancelled,
        })
    }

    fn checkpoint(&self) -> Self::Snapshot {
        self.trace.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        *self.trace = snapshot.clone();
    }

    fn choose_random(&self, state: &State3, candidate_count: usize) -> usize {
        random_choice_index(state, self.input, RuleId3(0), candidate_count)
    }
}

fn transition_rule_by_application(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    if !guards_accept(rule, game, state, input, local_frame) {
        return Ok(state.clone());
    }
    match rule.application {
        RuleApplication3::Once => {
            transition_rule_once(game, state, rule, input, local_frame, trace)
        }
        RuleApplication3::OnceAll => {
            transition_rule_once_all(game, state, rule, input, local_frame, trace)
        }
        RuleApplication3::OncePerLevel => {
            transition_rule_once_per_level(game, state, rule, input, local_frame, trace)
        }
        RuleApplication3::UntilStable => {
            transition_rule_repeated(game, state, rule, input, local_frame, trace)
        }
        RuleApplication3::Random => {
            transition_rule_random(game, state, rule, input, local_frame, trace)
        }
    }
}

fn rule_condition_accepts(
    game: &CompiledGame3,
    state: &State3,
    condition: &RuleCondition3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    let scope = LocalFrameScope::new(state, local_frame);
    match condition {
        RuleCondition3::AnyMatches(patterns) => patterns
            .iter()
            .any(|pattern| first_match(game, state, pattern, &scope).is_some()),
        RuleCondition3::NoMatches(patterns) => patterns
            .iter()
            .all(|pattern| first_match(game, state, pattern, &scope).is_none()),
        RuleCondition3::AnyInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && first_match(game, state, pattern, &scope).is_some()
            })
        }),
        RuleCondition3::NoInputMatches(patterns) => input.is_none_or(|input| {
            patterns.iter().all(|(expected, pattern)| {
                *expected != input || first_match(game, state, pattern, &scope).is_none()
            })
        }),
        RuleCondition3::GuardBranches(branches) => branches
            .iter()
            .any(|branch| guards_accept_all(branch, game, state, input, local_frame)),
    }
}

fn transition_rule_once(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if !guards_accept(rule, game, state, input, local_frame) {
        return Ok(next);
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let Some(placement) = first_match(game, state, &rule.pattern, &scope) else {
        return Ok(next);
    };
    if !writes_within_local_frame(&placement, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(rule, &placement)?;
    if rule_cancels(rule) {
        patch.validate(game, &next)?;
    } else {
        patch.apply_in_place(game, &mut next)?;
    }
    trace.record(rule, patch);
    Ok(next)
}

fn transition_rule_random(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if !guards_accept(rule, game, state, input, local_frame) {
        return Ok(next);
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let placements = all_matches(game, state, &rule.pattern, &scope);
    if placements.is_empty() {
        return Ok(next);
    }
    let index = random_choice_index(state, input, rule.id, placements.len());
    let placement = &placements[index];
    if !writes_within_local_frame(placement, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(rule, placement)?;
    if rule_cancels(rule) {
        patch.validate(game, &next)?;
    } else {
        patch.apply_in_place(game, &mut next)?;
    }
    trace.record(rule, patch);
    Ok(next)
}

pub fn transition_once_all(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut trace = TransitionTrace3::default();
    let mut next = transition_rule_once_all(game, &scoped, rule, None, None, &mut trace)?;
    next.clear_mark();
    Ok(next)
}

pub fn transition_once_per_level(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut trace = TransitionTrace3::default();
    let mut next = transition_rule_once_per_level(game, &scoped, rule, None, None, &mut trace)?;
    next.clear_mark();
    Ok(next)
}

fn transition_rule_once_all(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    if !guards_accept(rule, game, state, input, local_frame) {
        return Ok(state.clone());
    }

    let scope = LocalFrameScope::new(state, local_frame);
    let placements = all_matches(game, state, &rule.pattern, &scope);
    if placements.is_empty() {
        return Ok(state.clone());
    }

    let mut current = state.clone();
    let mut current_scope = LocalFrameScope::new(&current, local_frame);
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
            trace.record(rule, patch);
            return Ok(state.clone());
        }
        match patch.apply_in_place(game, &mut current) {
            Ok(_) => {
                trace.record(rule, patch);
                current_scope = LocalFrameScope::new(&current, local_frame);
            }
            Err(error) if once_all_patch_became_stale(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

fn transition_rule_once_per_level(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if next.level_rule_has_fired(rule.id) || !guards_accept(rule, game, state, input, local_frame) {
        return Ok(next);
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let Some(placement) = first_match(game, state, &rule.pattern, &scope) else {
        return Ok(next);
    };
    if !writes_within_local_frame(&placement, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(rule, &placement)?;
    if rule_cancels(rule) {
        patch.validate(game, &next)?;
    } else {
        patch.apply_in_place(game, &mut next)?;
    }
    trace.record(rule, patch);
    if !rule_cancels(rule) {
        next.mark_level_rule_fired(rule.id);
    }
    Ok(next)
}

pub fn transition_repeated(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_mark();
    let mut trace = TransitionTrace3::default();
    let mut next = transition_rule_repeated(game, &scoped, rule, None, None, &mut trace)?;
    next.clear_mark();
    Ok(next)
}

pub fn count_pattern_matches(game: &CompiledGame3, state: &State3, pattern: &Pattern3) -> u32 {
    let scope = LocalFrameScope::new(state, None);
    all_matches(game, state, pattern, &scope).len() as u32
}

pub fn has_pattern_match(game: &CompiledGame3, state: &State3, pattern: &Pattern3) -> bool {
    let scope = LocalFrameScope::new(state, None);
    first_match(game, state, pattern, &scope).is_some()
}

pub fn eval_condition_kind(
    game: &CompiledGame3,
    state: &State3,
    kind: &ConditionValueKind3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> i64 {
    let scope = LocalFrameScope::new(state, local_frame);
    match kind {
        ConditionValueKind3::CountObjects(objects) => objects
            .iter()
            .map(|object| count_object(game, state, *object))
            .sum::<u32>() as i64,
        ConditionValueKind3::ExistsObjects(objects) => objects
            .iter()
            .any(|object| count_object(game, state, *object) > 0)
            as i64,
        ConditionValueKind3::NoneObjects(objects) => objects
            .iter()
            .all(|object| count_object(game, state, *object) == 0)
            as i64,
        ConditionValueKind3::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| all_matches(game, state, pattern, &scope).len() as u32)
            .sum::<u32>() as i64,
        ConditionValueKind3::ExistsMatches(patterns) => patterns
            .iter()
            .any(|pattern| first_match(game, state, pattern, &scope).is_some())
            as i64,
        ConditionValueKind3::NoneMatches(patterns) => patterns
            .iter()
            .all(|pattern| first_match(game, state, pattern, &scope).is_none())
            as i64,
        ConditionValueKind3::CountInputMatches(patterns) => input
            .map(|input| {
                patterns
                    .iter()
                    .filter(|(expected, _)| *expected == input)
                    .map(|(_, pattern)| all_matches(game, state, pattern, &scope).len() as u32)
                    .sum::<u32>() as i64
            })
            .unwrap_or(0),
        ConditionValueKind3::ExistsInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && first_match(game, state, pattern, &scope).is_some()
            })
        }) as i64,
        ConditionValueKind3::NoneInputMatches(patterns) => input.is_some_and(|input| {
            patterns
                .iter()
                .filter(|(expected, _)| *expected == input)
                .all(|(_, pattern)| first_match(game, state, pattern, &scope).is_none())
        }) as i64,
    }
}

fn transition_rule_repeated(
    game: &CompiledGame3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
    trace: &mut TransitionTrace3,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    let mut seen = vec![current.clone()];
    let mut repeat_count = 0;
    loop {
        let next = transition_rule_once_all(game, &current, rule, input, local_frame, trace)?;
        if next == current {
            return Ok(current);
        }
        if seen.iter().any(|seen_state| *seen_state == next) {
            return Ok(next);
        }
        seen.push(next.clone());
        current = next;
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT3 {
            return Ok(current);
        }
    }
}

fn random_choice_index(
    state: &State3,
    input: Option<InputId>,
    rule: RuleId3,
    candidate_count: usize,
) -> usize {
    let mut hash = random_state_projection_hash(state);
    match input {
        Some(input) => {
            hash = fnv_mix(hash, 1);
            hash = fnv_mix(hash, u64::from(input.0));
        }
        None => {
            hash = fnv_mix(hash, 0);
        }
    }
    hash = fnv_mix(hash, u64::from(rule.0));
    hash = fnv_mix(hash, candidate_count as u64);
    (hash as usize) % candidate_count
}

fn random_state_projection_hash(state: &State3) -> u64 {
    let mut hash = FnvBuilder::OFFSET;
    hash = fnv_mix(hash, u64::from(state.size.width));
    hash = fnv_mix(hash, u64::from(state.size.depth));
    hash = fnv_mix(hash, u64::from(state.size.height));
    hash = fnv_mix(hash, u64::from(state.layer_count));
    for object in state.slots() {
        hash = fnv_mix(hash, u64::from(object.0));
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

fn guards_accept(
    rule: &Rule3,
    game: &CompiledGame3,
    state: &State3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    guards_accept_all(&rule.guards, game, state, input, local_frame)
}

fn guards_accept_all(
    guards: &[Guard3],
    game: &CompiledGame3,
    state: &State3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    guards
        .iter()
        .all(|guard| guard_accepts(guard, game, state, input, local_frame))
}

fn guard_accepts(
    guard: &Guard3,
    game: &CompiledGame3,
    state: &State3,
    input: Option<InputId>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> bool {
    match guard {
        Guard3::InputIs(expected) => input.is_some_and(|actual| actual == *expected),
        Guard3::VariableEquals { variable, value } => {
            state.variable_value(*variable) == Some(*value)
        }
        Guard3::VariableCompare {
            variable,
            op,
            value,
        } => state
            .variable_value(*variable)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        Guard3::ConditionEquals { condition, value } => {
            eval_condition_def(game, state, *condition, input, local_frame) == Some(*value)
        }
        Guard3::ConditionNonZero(condition) => {
            eval_condition_def(game, state, *condition, input, local_frame)
                .is_some_and(|value| value != 0)
        }
        Guard3::ConditionCompare {
            condition,
            op,
            value,
        } => eval_condition_def(game, state, *condition, input, local_frame)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        Guard3::InlineConditionValue { kind, value } => {
            eval_condition_kind(game, state, kind, input, local_frame) == *value
        }
        Guard3::InlineConditionNonZero(kind) => {
            eval_condition_kind(game, state, kind, input, local_frame) != 0
        }
        Guard3::InlineConditionCompare { kind, op, value } => compare_i64(
            eval_condition_kind(game, state, kind, input, local_frame),
            *op,
            *value,
        ),
    }
}

fn eval_condition_def(
    game: &CompiledGame3,
    state: &State3,
    condition: ConditionId3,
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

struct LocalFrameScope<'a> {
    frame: Option<&'a LocalFrame<ObjectId>>,
    focus_coords: Vec<Coord3>,
}

impl<'a> LocalFrameScope<'a> {
    fn new(state: &State3, frame: Option<&'a LocalFrame<ObjectId>>) -> Self {
        let focus_coords = frame
            .map(|frame| focus_coords(state, frame))
            .unwrap_or_default();
        Self {
            frame,
            focus_coords,
        }
    }

    fn contains_coord(&self, coord: Coord3) -> bool {
        let Some(frame) = self.frame else {
            return true;
        };
        self.focus_coords.iter().any(|focus| {
            let dx = i32::from(coord.x) - i32::from(focus.x);
            let dy = i32::from(coord.y) - i32::from(focus.y);
            let dz = i32::from(coord.z) - i32::from(focus.z);
            frame.contains_delta_3d(dx, dy, dz)
        })
    }

    fn origin_candidates(&self, state: &State3) -> Option<Vec<Coord3>> {
        let Some(frame) = self.frame else {
            return None;
        };
        let mut seen = HashSet::new();
        let mut coords = Vec::new();
        for focus in &self.focus_coords {
            let (x_range, y_range, z_range) = frame.ranges_3d(
                focus.x,
                focus.y,
                focus.z,
                state.size.width,
                state.size.depth,
                state.size.height,
            );
            for z in z_range {
                for y in y_range.clone() {
                    for x in x_range.clone() {
                        let coord = Coord3 { x, y, z };
                        if seen.insert(coord) {
                            coords.push(coord);
                        }
                    }
                }
            }
        }
        Some(coords)
    }
}

fn focus_coords(state: &State3, local_frame: &LocalFrame<ObjectId>) -> Vec<Coord3> {
    let mut coords = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let coord = Coord3 { x, y, z };
                if local_frame.focus_objects.iter().any(|object| {
                    state
                        .cell_view(coord)
                        .is_ok_and(|cell| cell.objects.contains(object))
                }) {
                    coords.push(coord);
                }
            }
        }
    }
    coords
}

fn writes_within_local_frame(
    placement: &MatchPlacement3,
    writes: &[WriteOp3],
    scope: &LocalFrameScope<'_>,
) -> Result<bool, TransitionError3> {
    if scope.frame.is_none() {
        return Ok(true);
    }
    for write in writes {
        match write.clone() {
            WriteOp3::Add {
                component, offset, ..
            }
            | WriteOp3::AddObjectSet {
                component, offset, ..
            }
            | WriteOp3::Remove {
                component, offset, ..
            }
            | WriteOp3::RemoveObjectSet {
                component, offset, ..
            }
            | WriteOp3::Replace {
                component, offset, ..
            }
            | WriteOp3::SetMark {
                component, offset, ..
            }
            | WriteOp3::SetObjectSetMark {
                component, offset, ..
            }
            | WriteOp3::RemoveMark {
                component, offset, ..
            }
            | WriteOp3::RemoveObjectSetMark {
                component, offset, ..
            } => {
                let position = write_position(placement, component, offset)?;
                if !scope.contains_coord(position) {
                    return Ok(false);
                }
            }
            WriteOp3::Move {
                component,
                from_offset,
                to_offset,
                ..
            }
            | WriteOp3::MoveObjectSet {
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

type MatchPlacement3 = MatchPlacement<3, ObjectId>;
type ComponentPlacement3 = ComponentPlacement<3, ObjectId>;

fn first_match(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    if pattern.components.is_empty() {
        return Some(MatchPlacement3::empty());
    }
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates.into_iter().find_map(|origin| {
            pattern_placement_from_first_origin(game, state, pattern, origin, scope)
        });
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if let Some(placement) =
                    pattern_placement_from_first_origin(game, state, pattern, origin, scope)
                {
                    return Some(placement);
                }
            }
        }
    }
    None
}

fn all_matches(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    scope: &LocalFrameScope<'_>,
) -> Vec<MatchPlacement3> {
    if pattern.components.is_empty() {
        return vec![MatchPlacement3::empty()];
    }
    let mut placements = Vec::new();
    if let Some(candidates) = scope.origin_candidates(state) {
        placements.extend(candidates.into_iter().flat_map(|origin| {
            component_placement_at(game, state, &pattern.components[0], origin, scope)
                .into_iter()
                .flat_map(|first| {
                    let mut components = vec![first];
                    collect_component_placements(game, state, pattern, 1, &mut components, scope)
                })
                .collect::<Vec<_>>()
        }));
        return placements;
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
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
        }
    }
    placements
}

fn pattern_placement_from_first_origin(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    origin: Coord3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    let first = component_placement_at(game, state, &pattern.components[0], origin, scope)?;
    let mut components = vec![first];
    complete_component_placements(game, state, pattern, 1, &mut components, scope)
        .then_some(MatchPlacement3::new(components))
}

fn complete_component_placements(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    component_index: usize,
    components: &mut Vec<ComponentPlacement3>,
    scope: &LocalFrameScope<'_>,
) -> bool {
    let mut candidate_origins =
        |_component: &PatternComponent3| component_origin_candidates(state, scope);
    let mut place_at = |component: &PatternComponent3, origin| {
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

fn collect_component_placements(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    component_index: usize,
    components: &mut Vec<ComponentPlacement3>,
    scope: &LocalFrameScope<'_>,
) -> Vec<MatchPlacement3> {
    let mut matches = Vec::new();
    let mut candidate_origins =
        |_component: &PatternComponent3| component_origin_candidates(state, scope);
    let mut place_at = |component: &PatternComponent3, origin| {
        component_placement_at(game, state, component, origin, scope)
    };
    let mut push_match = |matches: &mut Vec<MatchPlacement3>,
                          components: &[ComponentPlacement3]| {
        matches.push(MatchPlacement3::new(components.to_vec()));
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

fn component_origin_candidates(state: &State3, scope: &LocalFrameScope<'_>) -> Vec<Coord3> {
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates;
    }
    let mut candidates = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                candidates.push(Coord3 { x, y, z });
            }
        }
    }
    candidates
}

fn component_placement_at(
    game: &CompiledGame3,
    state: &State3,
    component: &PatternComponent3,
    origin: Coord3,
    scope: &LocalFrameScope<'_>,
) -> Option<ComponentPlacement3> {
    if component.gap_count == 0 {
        let gaps = Vec::new();
        let object_bindings =
            component_matches_with_gaps(game, state, component, origin, &gaps, scope)?;
        return Some(ComponentPlacement3::new(
            origin.into(),
            gaps,
            object_bindings,
        ));
    }

    let max_gap = state
        .size
        .width
        .max(state.size.depth)
        .max(state.size.height);
    for total_gap in 0..=max_gap.saturating_mul(component.gap_count) {
        let mut gaps = Vec::with_capacity(usize::from(component.gap_count));
        if let Some(object_bindings) = find_gap_assignment(
            game, state, component, origin, max_gap, total_gap, &mut gaps, scope,
        ) {
            return Some(ComponentPlacement3::new(
                origin.into(),
                gaps,
                object_bindings,
            ));
        }
    }
    None
}

fn find_gap_assignment(
    game: &CompiledGame3,
    state: &State3,
    component: &PatternComponent3,
    origin: Coord3,
    max_gap: u16,
    remaining_total: u16,
    gaps: &mut Vec<u16>,
    scope: &LocalFrameScope<'_>,
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

fn component_matches_with_gaps(
    game: &CompiledGame3,
    state: &State3,
    component: &PatternComponent3,
    origin: Coord3,
    gaps: &[u16],
    scope: &LocalFrameScope<'_>,
) -> Option<Vec<ObjectBinding<ObjectId>>> {
    let mut object_bindings = Vec::new();
    for cell in &component.cells {
        let position = resolve_offset(&cell.offset, gaps)
            .and_then(|offset| offset_pos(origin, offset))
            .filter(|position| state.check_pos(*position).is_ok());
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
            let Ok(found) = state.get_layer(position, object_set.layer) else {
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

fn placement_still_valid(
    game: &CompiledGame3,
    state: &State3,
    pattern: &Pattern3,
    placement: &MatchPlacement3,
    scope: &LocalFrameScope<'_>,
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
                object_bindings_match3(&bindings, &placed_component.object_bindings)
            })
        },
    )
}

fn object_bindings_match3(
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

fn bound_object(placement: &MatchPlacement3, binding: u16) -> Option<ObjectId> {
    placement_object_binding(placement, binding)
}

fn bound_object_in_component(
    object_bindings: &[ObjectBinding<ObjectId>],
    binding: u16,
) -> Option<ObjectId> {
    bound_object_in_bindings(object_bindings, binding)
}

fn mark_pattern_matches(
    game: &CompiledGame3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
    mark: &MarkPattern3,
) -> bool {
    match mark.match_value {
        MarkValueMatch::Any => state.has_mark_key(game, position, object, mark.mark),
        MarkValueMatch::Exact => state.has_mark(game, position, object, mark.mark, mark.value),
    }
}

fn mark_pattern_matches_bound(
    game: &CompiledGame3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
    mark: &ObjectSetMarkPattern3,
) -> bool {
    match mark.match_value {
        MarkValueMatch::Any => state.has_mark_key(game, position, object, mark.mark),
        MarkValueMatch::Exact => state.has_mark(game, position, object, mark.mark, mark.value),
    }
}

fn cell_requires_object(
    game: &CompiledGame3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
) -> bool {
    match state.cell_has_object_masked(position, object) {
        Some(found) => found,
        None => state.has_object(game, position, object),
    }
}

fn cell_forbids_object(
    game: &CompiledGame3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
) -> bool {
    match state.cell_has_object_masked(position, object) {
        Some(found) => !found,
        None => !state.has_object(game, position, object),
    }
}

fn count_object(game: &CompiledGame3, state: &State3, object: ObjectId) -> u32 {
    if let Some(count) = state.object_count_masked(object) {
        return count;
    }

    let mut count = 0;
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                if state.has_object(game, Coord3 { x, y, z }, object) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn build_patch(rule: &Rule3, placement: &MatchPlacement3) -> Result<Patch3, TransitionError3> {
    let mut patch = Patch3::new();
    for write in &rule.writes {
        match write.clone() {
            WriteOp3::Add {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp3::Add {
                    position: write_position(placement, component, offset)?,
                    object,
                });
            }
            WriteOp3::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp3::Add {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Remove {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp3::Remove {
                    position: write_position(placement, component, offset)?,
                    object,
                });
            }
            WriteOp3::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp3::Remove {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                patch.push(PatchOp3::Replace {
                    position: write_position(placement, component, offset)?,
                    remove,
                    add,
                });
            }
            WriteOp3::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                patch.push(PatchOp3::Move {
                    from: write_position(placement, component, from_offset)?,
                    to: write_position(placement, component, to_offset)?,
                    object,
                });
            }
            WriteOp3::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                patch.push(PatchOp3::Move {
                    from: write_position(placement, component, from_offset)?,
                    to: write_position(placement, component, to_offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::SetMark {
                component,
                offset,
                object,
                mark,
                value,
            } => {
                patch.push(PatchOp3::SetMark {
                    position: write_position(placement, component, offset)?,
                    object,
                    mark,
                    value,
                });
            }
            WriteOp3::SetObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
            } => {
                patch.push(PatchOp3::SetMark {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                    mark,
                    value,
                });
            }
            WriteOp3::RemoveMark {
                component,
                offset,
                object,
                mark,
                value,
                match_value,
            } => {
                patch.push(PatchOp3::RemoveMark {
                    position: write_position(placement, component, offset)?,
                    object,
                    mark,
                    value,
                    match_value,
                });
            }
            WriteOp3::RemoveObjectSetMark {
                component,
                offset,
                binding,
                mark,
                value,
                match_value,
            } => {
                patch.push(PatchOp3::RemoveMark {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                    mark,
                    value,
                    match_value,
                });
            }
        }
    }
    for effect in &rule.effects {
        match effect {
            RuleEffect3::UpdateVariable {
                variable,
                op,
                value,
            } => {
                patch.push(PatchOp3::UpdateVariable {
                    variable: *variable,
                    op: *op,
                    value: *value,
                });
            }
            RuleEffect3::Cancel
            | RuleEffect3::Win
            | RuleEffect3::Restart
            | RuleEffect3::NextLevel
            | RuleEffect3::Again
            | RuleEffect3::Checkpoint
            | RuleEffect3::ClearCheckpoint => {}
        }
    }
    Ok(patch)
}

fn rule_cancels(rule: &Rule3) -> bool {
    rule.effects
        .iter()
        .any(|effect| matches!(effect, RuleEffect3::Cancel))
}

fn write_position(
    placement: &MatchPlacement3,
    component: u16,
    offset: Offset3,
) -> Result<Coord3, TransitionError3> {
    write_position_shared(
        placement,
        component,
        &offset,
        |offset, gaps| resolve_grid_offset(offset, gaps),
        || TransitionError3::OffsetOutOfBounds,
    )
    .map(Coord3::from)
}

fn resolve_grid_offset(offset: &Offset3, gaps: &[u16]) -> Option<GridOffset<3>> {
    resolve_offset(offset, gaps).map(GridOffset::from)
}

fn resolve_offset(offset: &Offset3, gaps: &[u16]) -> Option<crate::Delta3> {
    match offset {
        Offset3::Fixed { dx, dy, dz } => Some(crate::Delta3::new(*dx, *dy, *dz)),
        Offset3::Variable {
            base_dx,
            base_dy,
            base_dz,
            gap_terms,
        } => {
            let mut dx = i32::from(*base_dx);
            let mut dy = i32::from(*base_dy);
            let mut dz = i32::from(*base_dz);
            for term in gap_terms {
                let gap = i32::from(*gaps.get(usize::from(term.gap_index))?);
                dx += i32::from(term.dx) * gap;
                dy += i32::from(term.dy) * gap;
                dz += i32::from(term.dz) * gap;
            }
            Some(crate::Delta3::new(
                i16::try_from(dx).ok()?,
                i16::try_from(dy).ok()?,
                i16::try_from(dz).ok()?,
            ))
        }
    }
}

fn once_all_patch_became_stale(error: &PatchError3) -> bool {
    matches!(
        error,
        PatchError3::State(StateError3::LayerOccupied { .. })
            | PatchError3::State(StateError3::ObjectNotPresent { .. })
    )
}

fn offset_pos(origin: Coord3, offset: crate::Delta3) -> Option<Coord3> {
    origin.checked_offset(offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConditionDef3, ConditionId3, Delta3, ObjectDef3, Size3};

    #[test]
    fn null_cell_uses_the_shared_out_of_bounds_match_contract() {
        let player = ObjectId(1);
        let game = CompiledGame3::checked_new(
            1,
            vec![ObjectDef3 {
                id: player,
                layer_id: LayerId(0),
            }],
        )
        .expect("valid game");
        let mut state = State3::empty(Size3::new(1, 1, 1), 1).expect("valid empty state");
        state
            .place_object(&game, Coord3::new(0, 0, 0), player)
            .expect("place player");

        let mut outside = MatchCell3::new(Delta3::new(0, -1, 0));
        outside.require_null = true;
        let boundary = Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(player), outside]);
        assert!(has_pattern_match(&game, &state, &boundary));

        let mut inside = MatchCell3::new(Delta3::ZERO);
        inside.require_null = true;
        let impossible = Pattern3::new(vec![inside]);
        assert!(!has_pattern_match(&game, &state, &impossible));
    }

    #[test]
    fn transition_accepts_shared_random_application() {
        let player = ObjectId(1);
        let crate_object = ObjectId(2);
        let game = CompiledGame3::checked_new(
            1,
            vec![
                ObjectDef3 {
                    id: player,
                    layer_id: LayerId(0),
                },
                ObjectDef3 {
                    id: crate_object,
                    layer_id: LayerId(0),
                },
            ],
        )
        .expect("valid game");
        let mut state = State3::empty(Size3::new(2, 1, 1), 1).expect("valid empty state");
        state
            .place_object(&game, Coord3::new(0, 0, 0), player)
            .expect("place first player");
        state
            .place_object(&game, Coord3::new(1, 0, 0), player)
            .expect("place second player");
        let mut rule = Rule3::once(
            Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(player)]),
            vec![WriteOp3::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: player,
                add: crate_object,
            }],
        )
        .with_id(RuleId3(7));
        rule.application = RuleApplication3::Random;

        let next = transition_program(&game, &state, &[RuleStep3::Rule(rule.clone())], InputId(0))
            .expect("random transition succeeds");
        let repeated = transition_program(&game, &state, &[RuleStep3::Rule(rule)], InputId(0))
            .expect("random transition is deterministic for the same state");

        assert_eq!(next, repeated);
        let positions = [Coord3::new(0, 0, 0), Coord3::new(1, 0, 0)];
        let crate_count = positions
            .iter()
            .filter(|position| {
                next.get_layer(**position, LayerId(0))
                    .is_ok_and(|object| object == crate_object)
            })
            .count();
        let player_count = positions
            .iter()
            .filter(|position| {
                next.get_layer(**position, LayerId(0))
                    .is_ok_and(|object| object == player)
            })
            .count();

        assert_eq!(crate_count, 1);
        assert_eq!(player_count, 1);
    }

    #[test]
    fn transition_accepts_shared_variable_guard() {
        let game = CompiledGame3::checked_new(1, Vec::new()).expect("valid empty game");
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![2])
            .expect("valid empty state");
        let mut rule = Rule3::once(Pattern3::new(Vec::new()), Vec::new()).with_effects(vec![
            RuleEffect3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 7,
            },
        ]);
        rule.guards.push(Guard3::VariableEquals {
            variable: VariableId(0),
            value: 2,
        });

        let next = transition_program_without_input(&game, &state, &[RuleStep3::Rule(rule)])
            .expect("transition succeeds");

        assert_eq!(next.variable_value(VariableId(0)), Some(7));
    }

    #[test]
    fn transition_accepts_shared_named_condition_guard() {
        let condition = ConditionId3(0);
        let game = CompiledGame3::new_with_condition_defs(
            1,
            Vec::new(),
            vec![ConditionDef3 {
                id: condition,
                kind: ConditionValueKind3::NoneObjects(vec![ObjectId(1)]),
            }],
        );
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![0])
            .expect("valid empty state");
        let mut rule = Rule3::once(Pattern3::new(Vec::new()), Vec::new()).with_effects(vec![
            RuleEffect3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 3,
            },
        ]);
        rule.guards.push(Guard3::ConditionEquals {
            condition,
            value: 1,
        });

        let next = transition_program_without_input(&game, &state, &[RuleStep3::Rule(rule)])
            .expect("transition succeeds");

        assert_eq!(next.variable_value(VariableId(0)), Some(3));
    }
}
