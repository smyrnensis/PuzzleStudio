use crate::compiled_game::{
    CompiledGame, Effect, Guard, LocalFrame, MatchCell, Offset, Pattern, PatternComponent,
    QueryKind, Rule, RuleApplication, RuleCondition, RuleStep, ScratchValueMatch, WriteOp,
};
use crate::ids::{InputId, ObjectId, QueryId, RuleId, ScratchId};
use crate::patch::{Patch, PatchError, PatchOp};
use crate::state::State;
use std::collections::{BTreeMap, BTreeSet};

const UNTIL_STABLE_REPEAT_LIMIT: usize = 200;

pub type TransitionResult<T = State> = Result<T, TransitionError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError {
    Patch(PatchError),
    OffsetOutOfBounds,
    RepeatUntilNoProgress,
}

impl From<PatchError> for TransitionError {
    fn from(value: PatchError) -> Self {
        Self::Patch(value)
    }
}

#[derive(Clone, Debug)]
pub struct StepTrace {
    pub input: InputId,
    pub next_state: State,
    pub cancelled: bool,
    pub commands: Vec<TransitionCommand>,
    pub fired_rules: Vec<RuleId>,
    pub patches: Vec<Patch>,
}

#[derive(Clone, Debug)]
pub struct TransitionOutcome {
    pub input: InputId,
    pub next_state: State,
    pub cancelled: bool,
    pub commands: Vec<TransitionCommand>,
    pub fired_rules: Vec<RuleId>,
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

pub fn transition_state(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    run_program_transition(game, game.program(), state, input, false, false)
        .map(|result| result.next_state)
}

pub fn transition_solver_state(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    let state = state.without_visual_objects(game);
    run_program_transition(game, game.program(), &state, input, false, true)
        .map(|result| result.next_state.without_visual_objects(game))
}

pub fn transition_outcome(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<TransitionOutcome> {
    run_program_transition(game, game.program(), state, input, false, false).map(|trace| {
        TransitionOutcome {
            input: trace.input,
            next_state: trace.next_state,
            cancelled: trace.cancelled,
            commands: trace.commands,
            fired_rules: trace.fired_rules,
        }
    })
}

pub fn transition_trace(
    game: &CompiledGame,
    state: &State,
    input: InputId,
) -> TransitionResult<StepTrace> {
    run_program_transition(game, game.program(), state, input, true, false)
}

pub fn transition_program(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<State> {
    run_program_transition(game, program, state, input, false, false)
        .map(|result| result.next_state)
}

pub fn transition_program_outcome(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<TransitionOutcome> {
    run_program_transition(game, program, state, input, false, false).map(|trace| {
        TransitionOutcome {
            input: trace.input,
            next_state: trace.next_state,
            cancelled: trace.cancelled,
            commands: trace.commands,
            fired_rules: trace.fired_rules,
        }
    })
}

pub fn transition_program_trace(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
) -> TransitionResult<StepTrace> {
    run_program_transition(game, program, state, input, true, false)
}

fn run_program_transition(
    game: &CompiledGame,
    program: &[RuleStep],
    state: &State,
    input: InputId,
    collect_trace: bool,
    skip_visual_rules: bool,
) -> TransitionResult<StepTrace> {
    let mut original = state.clone();
    original.clear_scratch();
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
            skip_visual_rules,
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

    current.clear_scratch();

    Ok(StepTrace {
        input,
        next_state: current,
        cancelled: false,
        commands,
        fired_rules,
        patches,
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
    skip_visual_rules: bool,
) -> TransitionResult<ApplyOutcome> {
    match step {
        RuleStep::Rule(rule) => {
            if skip_visual_rules && game.is_visual_rule(rule.id) {
                return Ok(ApplyOutcome::idle());
            }
            apply_rule_step(
                game,
                rule,
                context,
                current,
                fired_rules,
                patches,
                commands,
                collect_trace,
            )
        }
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
                    skip_visual_rules,
                )
            } else {
                Ok(ApplyOutcome::idle())
            }
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
                    skip_visual_rules,
                )
            }
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
                skip_visual_rules,
            ),
        },
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
                skip_visual_rules,
            )
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
    skip_visual_rules: bool,
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
            skip_visual_rules,
        )?;
        outcome.merge(step_outcome);
        if outcome.cancelled {
            break;
        }
    }
    Ok(outcome)
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
    skip_visual_rules: bool,
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
            skip_visual_rules,
        )?;
        if pass_outcome.cancelled {
            return Ok(pass_outcome);
        }
        if !pass_outcome.fired {
            break;
        }
        if current.hash() == before_hash && *current == before {
            break;
        }
        fired_any = true;
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
    let Some(placement) = find_first_match(game, current, rule, &scope) else {
        return Ok(ApplyOutcome::idle());
    };

    let patch = build_patch(rule, &placement)?;
    let cancels = rule
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::Cancel));
    if cancels {
        patch.validate(game, current)?;
    } else {
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

        let patch = build_patch(rule, &placement)?;
        let cancels = rule
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel));
        let applied = if cancels {
            match patch.validate(game, current) {
                Ok(_) => true,
                Err(error) if once_all_patch_became_stale(&error) => false,
                Err(error) => return Err(error.into()),
            }
        } else {
            match patch.apply_in_place(game, current) {
                Ok(_) => true,
                Err(error) if once_all_patch_became_stale(&error) => false,
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

fn once_all_patch_became_stale(error: &PatchError) -> bool {
    matches!(
        error,
        PatchError::ExpectedObject { .. } | PatchError::LayerOccupied { .. }
    )
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

fn push_rule_commands(rule: &Rule, commands: &mut Vec<TransitionCommand>) {
    for effect in &rule.effects {
        match effect {
            Effect::Win => commands.push(TransitionCommand::Win),
            Effect::Restart => commands.push(TransitionCommand::Restart),
            Effect::NextLevel => commands.push(TransitionCommand::NextLevel),
            Effect::Again => commands.push(TransitionCommand::Again),
            Effect::Checkpoint => commands.push(TransitionCommand::Checkpoint),
            Effect::ClearCheckpoint => commands.push(TransitionCommand::ClearCheckpoint),
            Effect::Cancel | Effect::UpdateGlobal { .. } => {}
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
        Guard::GlobalEquals { global, value } => state.global_value(*global) == Some(*value),
        Guard::GlobalCompare { global, op, value } => state
            .global_value(*global)
            .is_some_and(|found| compare_i64(found, *op, *value)),
        Guard::QueryEquals { query, value } => eval_query(context, state, *query) == Some(*value),
        Guard::QueryNonZero(query) => {
            eval_query(context, state, *query).is_some_and(|value| value != 0)
        }
        Guard::QueryCompare { query, op, value } => {
            eval_query(context, state, *query).is_some_and(|found| compare_i64(found, *op, *value))
        }
        Guard::QueryValue { kind, value } => eval_query_kind(context, state, kind) == *value,
        Guard::QueryValueNonZero(kind) => eval_query_kind(context, state, kind) != 0,
        Guard::QueryValueCompare { kind, op, value } => {
            compare_i64(eval_query_kind(context, state, kind), *op, *value)
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

fn eval_query(context: &TransitionContext, state: &State, query: QueryId) -> Option<i64> {
    let query = context.game.query(query)?;
    Some(eval_query_kind(context, state, &query.kind))
}

fn eval_query_kind(context: &TransitionContext, state: &State, kind: &QueryKind) -> i64 {
    match kind {
        QueryKind::CountObjects(objects) => objects
            .iter()
            .map(|object| i64::from(state.object_count(*object)))
            .sum(),
        QueryKind::ExistsObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                1
            } else {
                0
            }
        }
        QueryKind::NoneObjects(objects) => {
            if objects.iter().any(|object| state.object_count(*object) > 0) {
                0
            } else {
                1
            }
        }
        QueryKind::CountMatches(patterns) => patterns
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
        QueryKind::ExistsMatches(patterns) => {
            if patterns.iter().any(|pattern| {
                has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                1
            } else {
                0
            }
        }
        QueryKind::NoneMatches(patterns) => {
            if patterns.iter().any(|pattern| {
                has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                0
            } else {
                1
            }
        }
        QueryKind::CountInputMatches(patterns) => patterns
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
        QueryKind::ExistsInputMatches(patterns) => {
            if patterns.iter().any(|(input, pattern)| {
                *input == context.input
                    && has_pattern_match_in_scope(context.game, state, pattern, context.local_frame)
            }) {
                1
            } else {
                0
            }
        }
        QueryKind::NoneInputMatches(patterns) => {
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

#[derive(Clone, Debug)]
struct MatchPlacement {
    components: Vec<ComponentPlacement>,
}

#[derive(Clone, Debug)]
struct ComponentPlacement {
    origin_x: u16,
    origin_y: u16,
    gaps: Vec<u16>,
    object_bindings: Vec<ObjectBinding>,
}

#[derive(Clone, Copy, Debug)]
struct ObjectBinding {
    binding: u16,
    object: ObjectId,
}

fn find_first_match(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    scope: &LocalFrameScope2<'_>,
) -> Option<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return Some(MatchPlacement {
            components: Vec::new(),
        });
    }

    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0], scope) {
        if let Some(first) =
            component_placement_at(game, state, &rule.pattern.components[0], x, y, scope)
        {
            let mut components = vec![first];
            if complete_component_placements(game, state, rule, 1, &mut components, scope)
                && placement_writes_within_local_frame(rule, &components, scope)
            {
                return Some(MatchPlacement { components });
            }
        }
    }
    None
}

fn find_all_matches(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
    scope: &LocalFrameScope2<'_>,
) -> Vec<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return vec![MatchPlacement {
            components: Vec::new(),
        }];
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
    if component_index == rule.pattern.components.len() {
        return true;
    }

    let component = &rule.pattern.components[component_index];
    for (x, y) in component_candidate_origins(game, state, component, scope) {
        if let Some(placement) = component_placement_at(game, state, component, x, y, scope) {
            components.push(placement);
            if complete_component_placements(
                game,
                state,
                rule,
                component_index + 1,
                components,
                scope,
            ) {
                return true;
            }
            components.pop();
        }
    }

    false
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
    if component_index == rule.pattern.components.len() {
        if placement_writes_within_local_frame(rule, components, scope) {
            matches.push(MatchPlacement {
                components: components.clone(),
            });
        }
        return;
    }

    let component = &rule.pattern.components[component_index];
    for (x, y) in component_candidate_origins(game, state, component, scope) {
        if let Some(placement) = component_placement_at(game, state, component, x, y, scope) {
            components.push(placement);
            collect_component_placements(
                game,
                state,
                rule,
                component_index + 1,
                components,
                matches,
                scope,
            );
            components.pop();
        }
    }
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
            if cancels {
                fired_rules.push(rule.id);
                if collect_trace {
                    patches.push(patch);
                }
                return Ok(ApplyOutcome {
                    fired: true,
                    cancelled: true,
                });
            }
            if has_transition_command {
                fired_rules.push(rule.id);
                fired = true;
                if collect_trace {
                    patches.push(patch);
                }
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
        Some(MatchPlacement { components })
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
                placement.origin_x,
                placement.origin_y,
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
    SlotScratch {
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    },
    CellScratch {
        scratch: ScratchId,
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
        ComponentAnchorKind::SlotScratch {
            object,
            scratch,
            value,
        } => {
            if game.object_layer(*object).is_none() {
                return Vec::new();
            }
            state
                .scratch_positions(*object, *scratch, *value)
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect()
        }
        ComponentAnchorKind::CellScratch { scratch, value } => state
            .scratch_positions(ObjectId::EMPTY, *scratch, *value)
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
        for scratch in &cell.require_scratch {
            if scratch.match_value != ScratchValueMatch::Exact {
                continue;
            }
            let count = state
                .scratch_positions(scratch.object, scratch.scratch, scratch.value)
                .len() as u32;
            if best
                .as_ref()
                .is_none_or(|(best_count, _)| count < *best_count)
            {
                let kind = if scratch.object.is_empty() {
                    ComponentAnchorKind::CellScratch {
                        scratch: scratch.scratch,
                        value: scratch.value,
                    }
                } else {
                    ComponentAnchorKind::SlotScratch {
                        object: scratch.object,
                        scratch: scratch.scratch,
                        value: scratch.value,
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
            | WriteOp::SetScratch {
                component, offset, ..
            }
            | WriteOp::SetObjectSetScratch {
                component, offset, ..
            }
            | WriteOp::RemoveScratch {
                component, offset, ..
            }
            | WriteOp::RemoveObjectSetScratch {
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
        | WriteOp::SetScratch { component, .. }
        | WriteOp::SetObjectSetScratch { component, .. }
        | WriteOp::RemoveObjectSetScratch { component, .. }
        | WriteOp::RemoveScratch { component, .. } => *component,
    }
}

fn fixed_write_offset(write: &WriteOp) -> Option<(i16, i16)> {
    match write {
        WriteOp::Add { offset, .. }
        | WriteOp::AddObjectSet { offset, .. }
        | WriteOp::Remove { offset, .. }
        | WriteOp::RemoveObjectSet { offset, .. }
        | WriteOp::Replace { offset, .. }
        | WriteOp::SetScratch { offset, .. }
        | WriteOp::SetObjectSetScratch { offset, .. }
        | WriteOp::RemoveObjectSetScratch { offset, .. }
        | WriteOp::RemoveScratch { offset, .. } => fixed_offset(offset),
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
            return Some(ComponentPlacement {
                origin_x,
                origin_y,
                gaps,
                object_bindings,
            });
        }
        return None;
    }

    let max_gap = state.width.max(state.height);
    for total_gap in 0..=max_gap.saturating_mul(component.gap_count) {
        let mut gaps = Vec::with_capacity(usize::from(component.gap_count));
        if let Some(object_bindings) = find_gap_assignment(
            game, state, component, origin_x, origin_y, max_gap, total_gap, &mut gaps, scope,
        ) {
            return Some(ComponentPlacement {
                origin_x,
                origin_y,
                gaps,
                object_bindings,
            });
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
        return false;
    };
    if x >= state.width || y >= state.height {
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
        if !bind_object(object_bindings, object_set.binding, found) {
            return false;
        }
    }

    for scratch in &cell.require_scratch {
        let matched = match scratch.match_value {
            ScratchValueMatch::Any => {
                state.has_scratch_key(game, x, y, scratch.object, scratch.scratch)
            }
            ScratchValueMatch::Exact => {
                state.has_scratch(game, x, y, scratch.object, scratch.scratch, scratch.value)
            }
        };
        if !matched {
            return false;
        }
    }

    for scratch in &cell.require_object_set_scratch {
        let Some(object) = bound_object(object_bindings, scratch.binding) else {
            return false;
        };
        let matched = match scratch.match_value {
            ScratchValueMatch::Any => state.has_scratch_key(game, x, y, object, scratch.scratch),
            ScratchValueMatch::Exact => {
                state.has_scratch(game, x, y, object, scratch.scratch, scratch.value)
            }
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

    for scratch in &cell.forbid_object_set_scratch {
        let Some(object) = bound_object(object_bindings, scratch.binding) else {
            return false;
        };
        let matched = match scratch.match_value {
            ScratchValueMatch::Any => state.has_scratch_key(game, x, y, object, scratch.scratch),
            ScratchValueMatch::Exact => {
                state.has_scratch(game, x, y, object, scratch.scratch, scratch.value)
            }
        };
        if matched {
            return false;
        }
    }

    for scratch in &cell.forbid_scratch {
        let matched = match scratch.match_value {
            ScratchValueMatch::Any => {
                state.has_scratch_key(game, x, y, scratch.object, scratch.scratch)
            }
            ScratchValueMatch::Exact => {
                state.has_scratch(game, x, y, scratch.object, scratch.scratch, scratch.value)
            }
        };
        if matched {
            return false;
        }
    }

    true
}

fn bind_object(bindings: &mut Vec<ObjectBinding>, binding: u16, object: ObjectId) -> bool {
    if let Some(existing) = bindings
        .iter()
        .find(|existing| existing.binding == binding)
        .map(|existing| existing.object)
    {
        return existing == object;
    }
    bindings.push(ObjectBinding { binding, object });
    true
}

fn bound_object(bindings: &[ObjectBinding], binding: u16) -> Option<ObjectId> {
    bindings
        .iter()
        .find(|existing| existing.binding == binding)
        .map(|existing| existing.object)
}

fn build_patch(rule: &Rule, placement: &MatchPlacement) -> TransitionResult<Patch> {
    let mut patch = Patch::new();

    for write in &rule.writes {
        match write {
            WriteOp::Add {
                component,
                offset,
                object,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                patch.ops.push(PatchOp::Add {
                    x,
                    y,
                    object: *object,
                });
            }
            WriteOp::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                let object = placement_object_binding(placement, *binding)?;
                patch.ops.push(PatchOp::Add { x, y, object });
            }
            WriteOp::Remove {
                component,
                offset,
                object,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                patch.ops.push(PatchOp::Remove {
                    x,
                    y,
                    object: *object,
                });
            }
            WriteOp::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                let object = placement_object_binding(placement, *binding)?;
                patch.ops.push(PatchOp::Remove { x, y, object });
            }
            WriteOp::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                let (from_x, from_y) = write_position(placement, *component, from_offset)?;
                let (to_x, to_y) = write_position(placement, *component, to_offset)?;
                patch.ops.push(PatchOp::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    object: *object,
                });
            }
            WriteOp::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                let (from_x, from_y) = write_position(placement, *component, from_offset)?;
                let (to_x, to_y) = write_position(placement, *component, to_offset)?;
                let object = placement_object_binding(placement, *binding)?;
                patch.ops.push(PatchOp::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    object,
                });
            }
            WriteOp::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                patch.ops.push(PatchOp::Replace {
                    x,
                    y,
                    remove: *remove,
                    add: *add,
                });
            }
            WriteOp::SetScratch {
                component,
                offset,
                object,
                scratch,
                value,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                patch.ops.push(PatchOp::SetScratch {
                    x,
                    y,
                    object: *object,
                    scratch: *scratch,
                    value: *value,
                });
            }
            WriteOp::SetObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
                value,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                let object = placement_object_binding(placement, *binding)?;
                patch.ops.push(PatchOp::SetScratch {
                    x,
                    y,
                    object,
                    scratch: *scratch,
                    value: *value,
                });
            }
            WriteOp::RemoveScratch {
                component,
                offset,
                object,
                scratch,
                value,
                match_value,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                patch.ops.push(PatchOp::RemoveScratch {
                    x,
                    y,
                    object: *object,
                    scratch: *scratch,
                    value: *value,
                    match_value: *match_value,
                });
            }
            WriteOp::RemoveObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
                value,
                match_value,
            } => {
                let (x, y) = write_position(placement, *component, offset)?;
                let object = placement_object_binding(placement, *binding)?;
                patch.ops.push(PatchOp::RemoveScratch {
                    x,
                    y,
                    object,
                    scratch: *scratch,
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
            Effect::UpdateGlobal { global, op, value } => {
                patch.ops.push(PatchOp::UpdateGlobal {
                    global: *global,
                    op: *op,
                    value: *value,
                });
            }
        }
    }

    Ok(patch)
}

fn placement_object_binding(
    placement: &MatchPlacement,
    binding: u16,
) -> TransitionResult<ObjectId> {
    placement
        .components
        .iter()
        .flat_map(|component| &component.object_bindings)
        .find(|object_binding| object_binding.binding == binding)
        .map(|object_binding| object_binding.object)
        .ok_or(TransitionError::OffsetOutOfBounds)
}

fn write_position(
    placement: &MatchPlacement,
    component: u16,
    offset: &Offset,
) -> TransitionResult<(u16, u16)> {
    write_position_for_components(&placement.components, component, offset)
        .ok_or(TransitionError::OffsetOutOfBounds)
}

fn write_position_for_components(
    components: &[ComponentPlacement],
    component: u16,
    offset: &Offset,
) -> Option<(u16, u16)> {
    let placement = components.get(usize::from(component))?;
    let (dx, dy) = resolve_offset(offset, &placement.gaps)?;
    offset_pos(placement.origin_x, placement.origin_y, dx, dy)
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
        GlobalUpdateOp, Guard, MatchCell, ObjectDef, Offset, Pattern, PatternComponent, Rule,
        RuleApplication, ScratchDef, ScratchKind, ScratchPattern, WriteOp,
    };
    use crate::ids::{GlobalId, InputId, LayerId, ObjectId, RuleId};

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const MARK: ScratchId = ScratchId(1);
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
            require_objects,
            require_object_sets: Vec::new(),
            forbid_objects,
            require_scratch: Vec::new(),
            require_object_set_scratch: Vec::new(),
            forbid_scratch: Vec::new(),
            forbid_object_set_scratch: Vec::new(),
        }
    }

    fn scratch_cell(
        dx: i16,
        dy: i16,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    ) -> MatchCell {
        MatchCell {
            offset: fixed(dx, dy),
            require_objects: Vec::new(),
            require_object_sets: Vec::new(),
            forbid_objects: Vec::new(),
            require_scratch: vec![ScratchPattern {
                object,
                scratch,
                value,
                match_value: ScratchValueMatch::Exact,
            }],
            require_object_set_scratch: Vec::new(),
            forbid_scratch: Vec::new(),
            forbid_object_set_scratch: Vec::new(),
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

    fn global_rule(
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

    fn set_global(global: u16, value: i64) -> Effect {
        Effect::UpdateGlobal {
            global: GlobalId(global),
            op: GlobalUpdateOp::Set,
            value,
        }
    }

    fn add_global(global: u16, value: i64) -> Effect {
        Effect::UpdateGlobal {
            global: GlobalId(global),
            op: GlobalUpdateOp::Add,
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

    fn scratch_anchor_game() -> CompiledGame {
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
        let scratch = vec![ScratchDef {
            id: MARK,
            kind: ScratchKind::Marker,
            values: Vec::new(),
        }];
        CompiledGame::new_with_scratch_queries_and_program(3, objects, scratch, Vec::new(), vec![])
    }

    #[test]
    fn scratch_position_cache_tracks_slot_scratch_moves_and_clears() {
        let game = scratch_anchor_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();

        state.set_scratch_unchecked(1, 0, LayerId(2), MARK, Some(7));
        assert_eq!(
            state
                .scratch_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(1, 0)]
        );

        let scratch = state.take_slot_for_move_unchecked(1, 0, LayerId(2));
        state.place_moved_slot_unchecked(3, 0, LayerId(2), BOX, scratch);
        assert_eq!(
            state
                .scratch_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(3, 0)]
        );

        state.remove_scratch_unchecked(3, 0, LayerId(2), MARK, Some(7));
        assert!(state.scratch_positions(BOX, MARK, Some(7)).is_empty());

        state.set_scratch_unchecked(3, 0, LayerId(2), MARK, Some(9));
        state.clear_scratch();
        assert!(state.scratch_positions(BOX, MARK, Some(9)).is_empty());
    }

    #[test]
    fn component_candidates_anchor_on_rarest_required_exact_scratch() {
        let game = scratch_anchor_game();
        let mut state = State::empty(5, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();
        state.place_object(&game, 3, 0, PLAYER).unwrap();
        state.place_object(&game, 2, 0, BOX).unwrap();
        state.set_scratch_unchecked(2, 0, LayerId(2), MARK, Some(1));

        let component = PatternComponent {
            cells: vec![
                cell(0, 0, vec![PLAYER], Vec::new()),
                scratch_cell(1, 0, BOX, MARK, Some(1)),
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
    fn trace_reports_fired_rule_and_patch() {
        let game = push_game();
        let mut state = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, BOX).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.fired_rules, vec![RuleId(1)]);
        assert_eq!(trace.patches.len(), 1);
        assert_eq!(trace.patches[0].ops.len(), 4);
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
            guards: vec![Guard::GlobalEquals {
                global: GlobalId(0),
                value: 0,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateGlobal {
                global: GlobalId(0),
                op: GlobalUpdateOp::Set,
                value: 1,
            }],
        };
        let one_to_two = Rule {
            id: RuleId(8),
            guards: vec![Guard::GlobalEquals {
                global: GlobalId(0),
                value: 1,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateGlobal {
                global: GlobalId(0),
                op: GlobalUpdateOp::Set,
                value: 2,
            }],
        };
        let two_to_zero = Rule {
            id: RuleId(9),
            guards: vec![Guard::GlobalEquals {
                global: GlobalId(0),
                value: 2,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateGlobal {
                global: GlobalId(0),
                op: GlobalUpdateOp::Set,
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
        let state = State::empty_with_globals(1, 1, game.layer_count, game.object_count(), vec![0])
            .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_block_keeps_revisited_non_initial_state() {
        let value = GlobalId(0);
        let changed = GlobalId(1);
        let reset_changed = global_rule(
            20,
            Vec::new(),
            vec![set_global(1, 0)],
            RuleApplication::Once,
        );
        let two_to_one = global_rule(
            21,
            vec![
                Guard::GlobalEquals {
                    global: value,
                    value: 2,
                },
                Guard::GlobalEquals {
                    global: changed,
                    value: 0,
                },
            ],
            vec![set_global(0, 1), set_global(1, 1)],
            RuleApplication::Once,
        );
        let one_to_two = global_rule(
            22,
            vec![
                Guard::GlobalEquals {
                    global: value,
                    value: 1,
                },
                Guard::GlobalEquals {
                    global: changed,
                    value: 0,
                },
            ],
            vec![set_global(0, 2), set_global(1, 1)],
            RuleApplication::Once,
        );
        let zero_to_one = global_rule(
            23,
            vec![
                Guard::GlobalEquals {
                    global: value,
                    value: 0,
                },
                Guard::GlobalEquals {
                    global: changed,
                    value: 0,
                },
            ],
            vec![set_global(0, 1), set_global(1, 1)],
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
            State::empty_with_globals(1, 1, game.layer_count, game.object_count(), vec![0, 0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.global_value(value), Some(1));
        assert_eq!(next.global_value(changed), Some(1));
    }

    #[test]
    fn until_stable_block_budget_keeps_last_state_for_divergent_updates() {
        let counter = GlobalId(0);
        let increment = global_rule(
            24,
            Vec::new(),
            vec![add_global(0, 1)],
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
        let state = State::empty_with_globals(1, 1, game.layer_count, game.object_count(), vec![0])
            .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(
            next.global_value(counter),
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
                        effects: vec![Effect::UpdateGlobal {
                            global: GlobalId(0),
                            op: GlobalUpdateOp::Set,
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
                        effects: vec![Effect::UpdateGlobal {
                            global: GlobalId(0),
                            op: GlobalUpdateOp::Set,
                            value: 0,
                        }],
                    }),
                ],
            }],
        );
        let state = State::empty_with_globals(1, 1, game.layer_count, game.object_count(), vec![0])
            .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_rule_treats_idempotent_global_update_as_stable() {
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
            effects: vec![Effect::UpdateGlobal {
                global: GlobalId(0),
                op: GlobalUpdateOp::Set,
                value: 1,
            }],
        };
        let game = CompiledGame::new_with_program(2, objects, vec![RuleStep::Rule(rule)]);
        let mut state =
            State::empty_with_globals(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.global_value(GlobalId(0)), Some(1));
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
