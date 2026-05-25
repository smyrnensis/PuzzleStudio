use crate::compiled_game::{
    CompiledGame, Effect, Guard, MatchCell, Offset, Pattern, PatternComponent, QueryKind, Rule,
    RuleApplication, RuleCondition, RuleStep, ScratchValueMatch, WriteOp,
};
use crate::ids::{InputId, QueryId, RuleId};
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
    let context = TransitionContext { game, input };

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
            .any(|pattern| has_pattern_match(game, state, pattern)),
        RuleCondition::NoMatches(patterns) => patterns
            .iter()
            .all(|pattern| !has_pattern_match(game, state, pattern)),
        RuleCondition::AnyInputMatches(patterns) => patterns.iter().any(|(input, pattern)| {
            *input == context.input && has_pattern_match(game, state, pattern)
        }),
        RuleCondition::NoInputMatches(patterns) => patterns.iter().all(|(input, pattern)| {
            *input != context.input || !has_pattern_match(game, state, pattern)
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
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::OnceAll => apply_rule_once_all(
            game,
            rule,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::OncePerLevel => apply_rule_once_per_level(
            game,
            rule,
            current,
            fired_rules,
            patches,
            commands,
            collect_trace,
        ),
        RuleApplication::UntilStable => apply_until_stable(
            game,
            rule,
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
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let Some(placement) = find_first_match(game, current, rule) else {
        return Ok(ApplyOutcome::idle());
    };

    let patch = build_patch(rule, &placement)?;
    let next = patch.apply(game, current)?;
    fired_rules.push(rule.id);
    if collect_trace {
        patches.push(patch);
    }
    if rule
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::Cancel))
    {
        return Ok(ApplyOutcome {
            fired: true,
            cancelled: true,
        });
    }
    *current = next;
    push_rule_commands(rule, commands);
    Ok(ApplyOutcome {
        fired: true,
        cancelled: false,
    })
}

fn apply_rule_once_all(
    game: &CompiledGame,
    rule: &Rule,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let placements = find_all_matches(game, current, rule);
    if placements.is_empty() {
        return Ok(ApplyOutcome::idle());
    }

    let mut fired = false;
    for placement in placements {
        if !placement_matches(game, current, rule, &placement) {
            continue;
        }

        let patch = build_patch(rule, &placement)?;
        let next = match patch.apply(game, current) {
            Ok(next) => next,
            Err(error) if once_all_patch_became_stale(&error) => continue,
            Err(error) => return Err(error.into()),
        };

        fired = true;
        fired_rules.push(rule.id);
        if collect_trace {
            patches.push(patch);
        }
        if rule
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel))
        {
            return Ok(ApplyOutcome {
                fired: true,
                cancelled: true,
            });
        }
        *current = next;
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
            Effect::Cancel | Effect::UpdateGlobal { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TransitionContext<'a> {
    game: &'a CompiledGame,
    input: InputId,
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
            .map(|pattern| i64::from(count_pattern_matches(context.game, state, pattern)))
            .sum(),
        QueryKind::ExistsMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| has_pattern_match(context.game, state, pattern))
            {
                1
            } else {
                0
            }
        }
        QueryKind::NoneMatches(patterns) => {
            if patterns
                .iter()
                .any(|pattern| has_pattern_match(context.game, state, pattern))
            {
                0
            } else {
                1
            }
        }
        QueryKind::CountInputMatches(patterns) => patterns
            .iter()
            .filter(|(input, _)| *input == context.input)
            .map(|(_, pattern)| i64::from(count_pattern_matches(context.game, state, pattern)))
            .sum(),
        QueryKind::ExistsInputMatches(patterns) => {
            if patterns.iter().any(|(input, pattern)| {
                *input == context.input && has_pattern_match(context.game, state, pattern)
            }) {
                1
            } else {
                0
            }
        }
        QueryKind::NoneInputMatches(patterns) => {
            if patterns.iter().any(|(input, pattern)| {
                *input == context.input && has_pattern_match(context.game, state, pattern)
            }) {
                0
            } else {
                1
            }
        }
    }
}

pub fn count_pattern_matches(game: &CompiledGame, state: &State, pattern: &Pattern) -> u32 {
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
    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0]) {
        if match_from_first_origin(game, state, &rule, x, y).is_some() {
            count += 1;
        }
    }
    count
}

pub fn has_pattern_match(game: &CompiledGame, state: &State, pattern: &Pattern) -> bool {
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
    component_candidate_origins(game, state, &rule.pattern.components[0])
        .into_iter()
        .any(|(x, y)| match_from_first_origin(game, state, &rule, x, y).is_some())
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
}

fn find_first_match(game: &CompiledGame, state: &State, rule: &Rule) -> Option<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return Some(MatchPlacement {
            components: Vec::new(),
        });
    }

    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0]) {
        if let Some(first) = component_placement_at(game, state, &rule.pattern.components[0], x, y)
        {
            let mut components = vec![first];
            if complete_component_placements(game, state, rule, 1, &mut components) {
                return Some(MatchPlacement { components });
            }
        }
    }
    None
}

fn find_all_matches(game: &CompiledGame, state: &State, rule: &Rule) -> Vec<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return vec![MatchPlacement {
            components: Vec::new(),
        }];
    }

    let mut matches = Vec::new();
    for (x, y) in component_candidate_origins(game, state, &rule.pattern.components[0]) {
        if let Some(first) = component_placement_at(game, state, &rule.pattern.components[0], x, y)
        {
            let mut components = vec![first];
            collect_component_placements(game, state, rule, 1, &mut components, &mut matches);
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
) -> bool {
    if component_index == rule.pattern.components.len() {
        return true;
    }

    let component = &rule.pattern.components[component_index];
    for (x, y) in component_candidate_origins(game, state, component) {
        if let Some(placement) = component_placement_at(game, state, component, x, y) {
            components.push(placement);
            if complete_component_placements(game, state, rule, component_index + 1, components) {
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
) {
    if component_index == rule.pattern.components.len() {
        matches.push(MatchPlacement {
            components: components.clone(),
        });
        return;
    }

    let component = &rule.pattern.components[component_index];
    for (x, y) in component_candidate_origins(game, state, component) {
        if let Some(placement) = component_placement_at(game, state, component, x, y) {
            components.push(placement);
            collect_component_placements(
                game,
                state,
                rule,
                component_index + 1,
                components,
                matches,
            );
            components.pop();
        }
    }
}

fn apply_until_stable(
    game: &CompiledGame,
    rule: &Rule,
    current: &mut State,
    fired_rules: &mut Vec<RuleId>,
    patches: &mut Vec<Patch>,
    commands: &mut Vec<TransitionCommand>,
    collect_trace: bool,
) -> TransitionResult<ApplyOutcome> {
    let dirty_origin_deltas = dirty_origin_deltas(rule);
    let mut candidates = first_component_candidate_origin_set(game, current, rule);
    let mut seen_states = StateHistory::from_current(current);
    let mut fired = false;
    let mut repeat_count = 0;

    while let Some((y, x)) = pop_first_origin(&mut candidates) {
        let Some(placement) = match_from_first_origin(game, current, rule, x, y) else {
            continue;
        };

        let patch = build_patch(rule, &placement)?;
        let next = patch.apply(game, current)?;
        let cancels = rule
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel));
        let has_transition_command = rule.effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Win | Effect::Restart | Effect::NextLevel | Effect::Again
            )
        });
        if next == *current {
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
        *current = next;
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
            candidates = first_component_candidate_origin_set(game, current, rule);
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
) -> Option<MatchPlacement> {
    if rule.pattern.components.is_empty() {
        return None;
    }

    let first =
        component_placement_at(game, state, &rule.pattern.components[0], origin_x, origin_y)?;
    let mut components = vec![first];
    if complete_component_placements(game, state, rule, 1, &mut components) {
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
            )
        })
}

#[derive(Clone, Copy, Debug)]
struct ComponentAnchor {
    object: crate::ids::ObjectId,
    dx: i16,
    dy: i16,
}

fn first_component_candidate_origin_set(
    game: &CompiledGame,
    state: &State,
    rule: &Rule,
) -> BTreeSet<(u16, u16)> {
    let Some(component) = rule.pattern.components.first() else {
        return BTreeSet::new();
    };
    component_candidate_origins(game, state, component)
        .into_iter()
        .map(|(x, y)| (y, x))
        .collect()
}

fn component_candidate_origins(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
) -> Vec<(u16, u16)> {
    let Some(anchor) = component_anchor(game, state, component) else {
        return all_origin_vec(state);
    };
    if game.object_layer(anchor.object).is_none() {
        return all_origin_vec(state);
    }

    let mut origins = BTreeSet::new();
    for slot in state.object_positions(anchor.object) {
        let Some((x, y)) = state.slot_position(*slot) else {
            continue;
        };
        let Some((origin_x, origin_y)) = offset_pos(x, y, -anchor.dx, -anchor.dy) else {
            continue;
        };
        if origin_x < state.width && origin_y < state.height {
            origins.insert((origin_y, origin_x));
        }
    }

    origins.into_iter().map(|(y, x)| (x, y)).collect()
}

fn all_origin_vec(state: &State) -> Vec<(u16, u16)> {
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
    if component.gap_count != 0 {
        return None;
    }

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
                        object: *object,
                        dx,
                        dy,
                    },
                ));
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

fn dirty_origin_deltas(rule: &Rule) -> Option<BTreeSet<(i32, i32)>> {
    if rule.pattern.components.len() != 1
        || rule.pattern.components[0].gap_count != 0
        || rule.writes.iter().any(|write| {
            matches!(write, WriteOp::Move { .. })
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
        | WriteOp::Remove { component, .. }
        | WriteOp::Move { component, .. }
        | WriteOp::Replace { component, .. }
        | WriteOp::SetScratch { component, .. }
        | WriteOp::RemoveScratch { component, .. } => *component,
    }
}

fn fixed_write_offset(write: &WriteOp) -> Option<(i16, i16)> {
    match write {
        WriteOp::Add { offset, .. }
        | WriteOp::Remove { offset, .. }
        | WriteOp::Replace { offset, .. }
        | WriteOp::SetScratch { offset, .. }
        | WriteOp::RemoveScratch { offset, .. } => fixed_offset(offset),
        WriteOp::Move { to_offset, .. } => fixed_offset(to_offset),
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
) -> Option<ComponentPlacement> {
    if component.gap_count == 0 {
        let gaps = Vec::new();
        if component_matches_with_gaps(game, state, component, origin_x, origin_y, &gaps) {
            return Some(ComponentPlacement {
                origin_x,
                origin_y,
                gaps,
            });
        }
        return None;
    }

    let max_gap = state.width.max(state.height);
    for total_gap in 0..=max_gap.saturating_mul(component.gap_count) {
        let mut gaps = Vec::with_capacity(usize::from(component.gap_count));
        if find_gap_assignment(
            game, state, component, origin_x, origin_y, max_gap, total_gap, &mut gaps,
        ) {
            return Some(ComponentPlacement {
                origin_x,
                origin_y,
                gaps,
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
) -> bool {
    if gaps.len() == usize::from(component.gap_count) {
        return remaining_total == 0
            && component_matches_with_gaps(game, state, component, origin_x, origin_y, gaps);
    }

    for gap in 0..=max_gap.min(remaining_total) {
        gaps.push(gap);
        if find_gap_assignment(
            game,
            state,
            component,
            origin_x,
            origin_y,
            max_gap,
            remaining_total - gap,
            gaps,
        ) {
            return true;
        }
        gaps.pop();
    }

    false
}

fn component_matches_with_gaps(
    game: &CompiledGame,
    state: &State,
    component: &PatternComponent,
    origin_x: u16,
    origin_y: u16,
    gaps: &[u16],
) -> bool {
    component
        .cells
        .iter()
        .all(|cell| match_cell(game, state, origin_x, origin_y, gaps, cell))
}

fn match_cell(
    game: &CompiledGame,
    state: &State,
    origin_x: u16,
    origin_y: u16,
    gaps: &[u16],
    cell: &MatchCell,
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

    for object in &cell.require_objects {
        if !state.has_object(game, x, y, *object) {
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

    for object in &cell.forbid_objects {
        if state.has_object(game, x, y, *object) {
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
        }
    }
    for effect in &rule.effects {
        match effect {
            Effect::Cancel => {}
            Effect::Win | Effect::Restart | Effect::NextLevel | Effect::Again => {}
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

fn write_position(
    placement: &MatchPlacement,
    component: u16,
    offset: &Offset,
) -> TransitionResult<(u16, u16)> {
    let placement = placement
        .components
        .get(usize::from(component))
        .ok_or(TransitionError::OffsetOutOfBounds)?;
    let (dx, dy) =
        resolve_offset(offset, &placement.gaps).ok_or(TransitionError::OffsetOutOfBounds)?;
    offset_pos(placement.origin_x, placement.origin_y, dx, dy)
        .ok_or(TransitionError::OffsetOutOfBounds)
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
        RuleApplication, WriteOp,
    };
    use crate::ids::{GlobalId, InputId, LayerId, ObjectId, RuleId};

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
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
            forbid_objects,
            require_scratch: Vec::new(),
            forbid_scratch: Vec::new(),
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
