use crate::object_refs::{self, SolverPatternObjectRefs};
#[cfg(test)]
use puzzle_core::CompiledGame;
use puzzle_core::{GridCompiledGame, GridOffset, GridPattern, GridRuleStep};
use puzzle_kernel::{
    CompiledGameModel, ConditionId, ConditionValueKind, InputId, LocalFrame, MarkId, ObjectId,
    ProgramCondition, ProgramStep, RuleConditionDef, RuleEffect, RuleGuard, RuleId, RuleModel,
    RuleWriteOp, VariableId,
};
use std::collections::BTreeSet;

type RelevanceConditionValue<Pattern> = ConditionValueKind<ObjectId, Pattern, InputId>;
type RelevanceGuard<Pattern> =
    RuleGuard<VariableId, ConditionId, RelevanceConditionValue<Pattern>, InputId>;
type RelevanceCondition<Pattern> = ProgramCondition<Pattern, RelevanceGuard<Pattern>>;
type RelevanceRule<Pattern, Offset> = RuleModel<
    RuleId,
    RelevanceGuard<Pattern>,
    Pattern,
    RuleWriteOp<Offset, ObjectId, MarkId>,
    RuleEffect,
>;
type RelevanceStep<Pattern, Offset> =
    ProgramStep<RelevanceRule<Pattern, Offset>, RelevanceCondition<Pattern>, LocalFrame<ObjectId>>;
type RelevanceGame<Pattern, Offset> = CompiledGameModel<
    RuleConditionDef<ConditionId, RelevanceConditionValue<Pattern>>,
    RelevanceRule<Pattern, Offset>,
    RelevanceCondition<Pattern>,
    LocalFrame<ObjectId>,
>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverRelevance<ObjectId = puzzle_kernel::ObjectId> {
    relevant_objects: BTreeSet<ObjectId>,
    relevant_rules: BTreeSet<u16>,
}

impl<ObjectId> Default for SolverRelevance<ObjectId> {
    fn default() -> Self {
        Self {
            relevant_objects: BTreeSet::new(),
            relevant_rules: BTreeSet::new(),
        }
    }
}

impl<ObjectId: Copy + Ord> SolverRelevance<ObjectId> {
    pub(crate) fn from_roots(
        roots: impl IntoIterator<Item = ObjectId>,
        is_empty: impl Fn(ObjectId) -> bool,
    ) -> Self {
        let mut analysis = Self::default();
        for object in roots {
            analysis.insert_relevant_object(object, &is_empty);
        }
        analysis
    }

    pub fn contains_object(&self, object: ObjectId) -> bool {
        self.relevant_objects.contains(&object)
    }

    pub fn relevant_objects(&self) -> Vec<ObjectId> {
        self.relevant_objects.iter().copied().collect()
    }

    pub(crate) fn insert_relevant_rule_id(&mut self, rule: u16) -> bool {
        self.relevant_rules.insert(rule)
    }

    pub(crate) fn relevant_rule_ids(&self) -> Vec<u16> {
        self.relevant_rules.iter().copied().collect()
    }

    pub(crate) fn insert_relevant_objects<'a>(
        &mut self,
        objects: impl IntoIterator<Item = &'a ObjectId>,
        is_empty: &impl Fn(ObjectId) -> bool,
    ) -> bool
    where
        ObjectId: 'a,
    {
        let mut changed = false;
        for object in objects {
            changed |= self.insert_relevant_object(*object, is_empty);
        }
        changed
    }

    pub(crate) fn insert_relevant_object(
        &mut self,
        object: ObjectId,
        is_empty: &impl Fn(ObjectId) -> bool,
    ) -> bool {
        !is_empty(object) && self.relevant_objects.insert(object)
    }
}

impl SolverRelevance<ObjectId> {
    pub fn from_root_objects<const D: usize>(
        game: &GridCompiledGame<D>,
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        Self::from_game_program(game, game.program(), roots)
    }

    pub fn ignored_objects_for_game<const D: usize>(
        &self,
        game: &GridCompiledGame<D>,
    ) -> Vec<ObjectId> {
        self.ignored_objects(game)
    }

    pub fn relevant_rules(&self) -> Vec<RuleId> {
        self.relevant_rule_ids().into_iter().map(RuleId).collect()
    }

    pub fn from_program_roots<const D: usize>(
        game: &GridCompiledGame<D>,
        program: &[GridRuleStep<D>],
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        Self::from_game_program::<GridPattern<D>, GridOffset<D>>(game, program, roots)
    }

    pub fn from_programs_roots<'a, const D: usize>(
        game: &GridCompiledGame<D>,
        programs: impl IntoIterator<Item = &'a [GridRuleStep<D>]>,
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        let programs = programs.into_iter().collect::<Vec<_>>();
        let mut analysis = Self::from_roots(roots, ObjectId::is_empty);
        let mut changed = true;
        while changed {
            changed = false;
            for step in programs.iter().flat_map(|program| program.iter()) {
                changed |= analysis.propagate_step(game, step);
            }
        }
        analysis
    }

    fn from_game_program<Pattern, Offset>(
        game: &RelevanceGame<Pattern, Offset>,
        program: &[RelevanceStep<Pattern, Offset>],
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        let mut analysis = Self::from_roots(roots, ObjectId::is_empty);
        let mut changed = true;
        while changed {
            changed = false;
            for step in program {
                changed |= analysis.propagate_step(game, step);
            }
        }
        analysis
    }

    fn ignored_objects<Pattern, Offset>(
        &self,
        game: &RelevanceGame<Pattern, Offset>,
    ) -> Vec<ObjectId>
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        game.objects()
            .iter()
            .filter_map(|object| {
                (!object.id.is_empty() && !self.contains_object(object.id)).then_some(object.id)
            })
            .collect()
    }

    fn propagate_step<Pattern, Offset>(
        &mut self,
        game: &RelevanceGame<Pattern, Offset>,
        step: &RelevanceStep<Pattern, Offset>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        match step {
            ProgramStep::Rule(rule) => self.propagate_rule(game, rule),
            ProgramStep::ConditionalBlock { condition, steps } => {
                let mut changed = false;
                if steps.iter().any(|step| step_has_relevant_write(step, self)) {
                    changed |= self.insert_condition_objects(game, condition);
                }
                for step in steps {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
            ProgramStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let mut changed = false;
                if then_steps
                    .iter()
                    .chain(else_steps)
                    .any(|step| step_has_relevant_write(step, self))
                {
                    changed |= self.insert_condition_objects(game, condition);
                }
                for step in then_steps.iter().chain(else_steps) {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
            ProgramStep::Block {
                stop_condition,
                steps,
                ..
            } => {
                let mut changed = false;
                if let Some(stop_condition) = stop_condition
                    && steps.iter().any(|step| step_has_relevant_write(step, self))
                {
                    changed |= self.insert_condition_objects(game, stop_condition);
                }
                for step in steps {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
            ProgramStep::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                if then_steps
                    .iter()
                    .any(|step| step_has_relevant_write(step, self))
                {
                    for step in steps {
                        changed |= self.insert_step_read_objects(game, step);
                    }
                }
                for step in steps.iter().chain(then_steps) {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
            ProgramStep::LocalFrame { frame, steps } => {
                let mut changed = false;
                if steps.iter().any(|step| step_has_relevant_write(step, self)) {
                    changed |=
                        self.insert_relevant_objects(&frame.focus_objects, &ObjectId::is_empty);
                }
                for step in steps {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
        }
    }

    fn propagate_rule<Pattern, Offset>(
        &mut self,
        game: &RelevanceGame<Pattern, Offset>,
        rule: &RelevanceRule<Pattern, Offset>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        if !rule_has_relevant_output(rule, self) {
            return false;
        }

        let mut changed = self.insert_relevant_rule_id(rule.id.0);
        changed |= self.insert_pattern_objects(&rule.pattern);
        for guard in &rule.guards {
            changed |= self.insert_guard_objects(game, guard);
        }
        changed
    }

    fn insert_step_read_objects<Pattern, Offset>(
        &mut self,
        game: &RelevanceGame<Pattern, Offset>,
        step: &RelevanceStep<Pattern, Offset>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        match step {
            ProgramStep::Rule(rule) => {
                let mut changed = self.insert_relevant_rule_id(rule.id.0);
                changed |= self.insert_pattern_objects(&rule.pattern);
                for guard in &rule.guards {
                    changed |= self.insert_guard_objects(game, guard);
                }
                changed
            }
            ProgramStep::ConditionalBlock { condition, steps } => {
                let mut changed = self.insert_condition_objects(game, condition);
                for step in steps {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            ProgramStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let mut changed = self.insert_condition_objects(game, condition);
                for step in then_steps.iter().chain(else_steps) {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            ProgramStep::Block {
                stop_condition,
                steps,
                ..
            } => {
                let mut changed = stop_condition
                    .as_ref()
                    .is_some_and(|condition| self.insert_condition_objects(game, condition));
                for step in steps {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            ProgramStep::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                for step in steps.iter().chain(then_steps) {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            ProgramStep::LocalFrame { frame, steps } => {
                let mut changed =
                    self.insert_relevant_objects(&frame.focus_objects, &ObjectId::is_empty);
                for step in steps {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
        }
    }

    fn insert_guard_objects<Pattern, Offset>(
        &mut self,
        game: &RelevanceGame<Pattern, Offset>,
        guard: &RelevanceGuard<Pattern>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        match guard {
            RuleGuard::ConditionEquals { condition, .. }
            | RuleGuard::ConditionNonZero(condition)
            | RuleGuard::ConditionCompare { condition, .. } => game
                .condition_def(*condition)
                .is_some_and(|condition| self.insert_condition_value_objects(&condition.kind)),
            RuleGuard::InlineConditionValue { kind, .. }
            | RuleGuard::InlineConditionNonZero(kind)
            | RuleGuard::InlineConditionCompare { kind, .. } => {
                self.insert_condition_value_objects(kind)
            }
            RuleGuard::InputIs(_)
            | RuleGuard::VariableEquals { .. }
            | RuleGuard::VariableCompare { .. } => false,
        }
    }

    fn insert_condition_objects<Pattern, Offset>(
        &mut self,
        game: &RelevanceGame<Pattern, Offset>,
        condition: &RelevanceCondition<Pattern>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        match condition {
            ProgramCondition::AnyMatches(patterns) | ProgramCondition::NoMatches(patterns) => {
                object_refs::insert_patterns_objects(self, patterns)
            }
            ProgramCondition::AnyInputMatches(patterns)
            | ProgramCondition::NoInputMatches(patterns) => object_refs::insert_patterns_objects(
                self,
                patterns.iter().map(|(_, pattern)| pattern),
            ),
            ProgramCondition::GuardBranches(branches) => {
                let mut changed = false;
                for guard in branches.iter().flatten() {
                    changed |= self.insert_guard_objects(game, guard);
                }
                changed
            }
            ProgramCondition::RuleMatches { guards, pattern } => {
                let mut changed = self.insert_pattern_objects(pattern);
                for guard in guards {
                    changed |= self.insert_guard_objects(game, guard);
                }
                changed
            }
        }
    }

    fn insert_condition_value_objects<Pattern>(
        &mut self,
        kind: &RelevanceConditionValue<Pattern>,
    ) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        object_refs::insert_condition_value_objects(self, kind)
    }

    fn insert_pattern_objects<Pattern>(&mut self, pattern: &Pattern) -> bool
    where
        Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
    {
        object_refs::insert_pattern_objects(self, pattern)
    }
}

fn step_has_relevant_write<Pattern, Offset>(
    step: &RelevanceStep<Pattern, Offset>,
    relevance: &SolverRelevance,
) -> bool
where
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    match step {
        ProgramStep::Rule(rule) => rule_has_relevant_output(rule, relevance),
        ProgramStep::ConditionalBlock { steps, .. }
        | ProgramStep::Block { steps, .. }
        | ProgramStep::LocalFrame { steps, .. } => steps
            .iter()
            .any(|step| step_has_relevant_write(step, relevance)),
        ProgramStep::ConditionalBranch {
            then_steps,
            else_steps,
            ..
        } => then_steps
            .iter()
            .chain(else_steps)
            .any(|step| step_has_relevant_write(step, relevance)),
        ProgramStep::AfterTriggered { steps, then_steps } => steps
            .iter()
            .chain(then_steps)
            .any(|step| step_has_relevant_write(step, relevance)),
    }
}

fn rule_has_relevant_output<Pattern, Offset>(
    rule: &RelevanceRule<Pattern, Offset>,
    relevance: &SolverRelevance,
) -> bool
where
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    !rule.effects.is_empty()
        || rule.writes.iter().any(|write| {
            object_refs::write_touches_relevant_object(write, &rule.pattern, relevance)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_core::{
        ConditionDef, ConditionId, Effect, Guard, LayerId, MatchCell, ObjectDef, ObjectId,
        ObjectSetMatcher, Offset, Pattern, PatternComponent, Rule, RuleApplication, RuleCondition,
        RuleId, RuleStep, WriteOp,
    };
    use puzzle_lang::{LoadedGame, parse_game2d as parse_game};

    const ACTOR: LayerId = LayerId(1);
    const ITEM: LayerId = LayerId(2);

    fn object_named(loaded: &LoadedGame, name: &str) -> ObjectId {
        loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == name).then_some(*id))
            .unwrap_or_else(|| panic!("missing object {name}"))
    }

    fn object(id: u16, layer: LayerId) -> ObjectDef {
        ObjectDef {
            id: ObjectId(id),
            layer_id: layer,
        }
    }

    fn fixed(dx: i16, dy: i16) -> Offset {
        Offset::Fixed {
            delta: [dx, dy].into(),
        }
    }

    fn cell(require_objects: Vec<ObjectId>, forbid_objects: Vec<ObjectId>) -> MatchCell {
        MatchCell {
            offset: fixed(0, 0),
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

    fn object_set_cell(binding: u16, layer: LayerId, objects: Vec<ObjectId>) -> MatchCell {
        MatchCell {
            offset: fixed(0, 0),
            require_null: false,
            require_objects: Vec::new(),
            require_object_sets: vec![ObjectSetMatcher {
                binding,
                layer,
                objects,
            }],
            forbid_objects: Vec::new(),
            require_mark: Vec::new(),
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

    fn rule(
        id: u16,
        pattern: Pattern,
        guards: Vec<Guard>,
        writes: Vec<WriteOp>,
        effects: Vec<Effect>,
    ) -> Rule {
        Rule {
            id: RuleId(id),
            guards,
            application: RuleApplication::Once,
            pattern,
            writes,
            effects,
        }
    }

    fn add(object: ObjectId) -> WriteOp {
        WriteOp::Add {
            component: 0,
            offset: fixed(0, 0),
            object,
        }
    }

    fn move_object_set(binding: u16) -> WriteOp {
        WriteOp::MoveObjectSet {
            component: 0,
            from_offset: fixed(0, 0),
            to_offset: fixed(1, 0),
            binding,
        }
    }

    fn assert_relevant(relevance: &SolverRelevance, object: ObjectId) {
        assert!(
            relevance.contains_object(object),
            "expected object {object:?} to be solver-relevant"
        );
    }

    fn assert_pruned(relevance: &SolverRelevance, object: ObjectId) {
        assert!(
            !relevance.contains_object(object),
            "expected object {object:?} to be solver-pruned"
        );
    }

    #[test]
    fn relevance_back_propagates_from_root_writes_to_rule_reads() {
        let source = r#"
const title = relevance_backprop

puzzle default {
layers {
actor = Switch Door
}

rules {
[ Switch ] -> [ Door ]
}

levels tiny of default {
legend {
. = empty
S = Switch
D = Door
}

level "start" {
S
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let switch = object_named(&loaded, "Switch");
        let door = object_named(&loaded, "Door");

        let relevance = SolverRelevance::from_root_objects(&loaded.game, [door]);

        assert!(relevance.contains_object(door));
        assert!(relevance.contains_object(switch));
    }

    #[test]
    fn relevance_does_not_keep_self_maintaining_projection_without_root() {
        let source = r#"
const title = relevance_projection

puzzle default {
layers {
floor = Floor
actor = Player
}

rules {
[ no Floor ] -> [ Floor ]
}

levels tiny of default {
legend {
. = empty
P = Player
}

level "start" {
P
}
}
}
"#;
        let loaded = parse_game(source).unwrap();
        let player = object_named(&loaded, "Player");
        let floor = object_named(&loaded, "Floor");

        let relevance = SolverRelevance::from_root_objects(&loaded.game, [player]);

        assert!(relevance.contains_object(player));
        assert!(!relevance.contains_object(floor));
    }

    #[test]
    fn win_effect_roots_rule_reads_without_promoting_projection_objects() {
        const PLAYER: ObjectId = ObjectId(1);
        const GOAL: ObjectId = ObjectId(2);
        const FLOOR: ObjectId = ObjectId(3);

        let projection = rule(
            1,
            pattern(vec![cell(Vec::new(), vec![FLOOR])]),
            Vec::new(),
            vec![add(FLOOR)],
            Vec::new(),
        );
        let win = rule(
            2,
            pattern(vec![
                cell(vec![PLAYER], Vec::new()),
                cell(vec![GOAL], Vec::new()),
            ]),
            Vec::new(),
            Vec::new(),
            vec![Effect::Win],
        );
        let game = CompiledGame::new_with_program(
            2,
            vec![object(1, ACTOR), object(2, ITEM), object(3, ITEM)],
            vec![RuleStep::Rule(projection), RuleStep::Rule(win)],
        );

        let relevance = SolverRelevance::from_root_objects(&game, []);

        assert_relevant(&relevance, PLAYER);
        assert_relevant(&relevance, GOAL);
        assert_pruned(&relevance, FLOOR);
        assert_eq!(relevance.relevant_rules(), vec![RuleId(2)]);
    }

    #[test]
    fn transition_control_effects_are_solver_relevant() {
        const PLAYER: ObjectId = ObjectId(1);
        for effect in [Effect::Cancel, Effect::Again] {
            let game = CompiledGame::new_with_program(
                1,
                vec![object(1, ACTOR)],
                vec![RuleStep::Rule(rule(
                    1,
                    pattern(vec![cell(vec![PLAYER], Vec::new())]),
                    Vec::new(),
                    Vec::new(),
                    vec![effect],
                ))],
            );
            let relevance = SolverRelevance::from_root_objects(&game, []);
            assert_relevant(&relevance, PLAYER);
            assert_eq!(relevance.relevant_rules(), vec![RuleId(1)]);
        }
    }

    #[test]
    fn local_frame_focus_objects_are_control_dependencies() {
        const PLAYER: ObjectId = ObjectId(1);
        const FOCUS: ObjectId = ObjectId(2);
        const GOAL: ObjectId = ObjectId(3);
        let game = CompiledGame::new_with_program(
            3,
            vec![object(1, ACTOR), object(2, ITEM), object(3, ITEM)],
            vec![RuleStep::LocalFrame {
                frame: puzzle_core::LocalFrame::new(
                    puzzle_core::LocalFrameExtent::Full,
                    puzzle_core::LocalFrameExtent::Full,
                    puzzle_core::LocalFrameExtent::Full,
                    vec![FOCUS],
                ),
                steps: vec![RuleStep::Rule(rule(
                    1,
                    pattern(vec![cell(vec![PLAYER], Vec::new())]),
                    Vec::new(),
                    vec![add(GOAL)],
                    Vec::new(),
                ))],
            }],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [GOAL]);

        assert_relevant(&relevance, PLAYER);
        assert_relevant(&relevance, FOCUS);
        assert_relevant(&relevance, GOAL);
    }

    #[test]
    fn conditional_branch_propagates_gate_and_named_guard_but_not_dead_branch() {
        const KEY: ObjectId = ObjectId(1);
        const SWITCH: ObjectId = ObjectId(2);
        const BATTERY: ObjectId = ObjectId(3);
        const GATE: ObjectId = ObjectId(4);
        const SPARKLE: ObjectId = ObjectId(5);
        const DECOY: ObjectId = ObjectId(6);

        let condition_defs = vec![ConditionDef {
            id: ConditionId(0),
            kind: ConditionValueKind::ExistsObjects(vec![BATTERY]),
        }];
        let relevant_branch = rule(
            1,
            pattern(vec![cell(vec![SWITCH], Vec::new())]),
            vec![Guard::ConditionNonZero(ConditionId(0))],
            vec![add(GATE)],
            Vec::new(),
        );
        let dead_branch = rule(
            2,
            pattern(vec![cell(vec![SPARKLE], Vec::new())]),
            Vec::new(),
            vec![add(DECOY)],
            Vec::new(),
        );
        let game = CompiledGame::new_with_condition_defs_and_program(
            2,
            vec![
                object(1, ITEM),
                object(2, ACTOR),
                object(3, ITEM),
                object(4, ITEM),
                object(5, ITEM),
                object(6, ITEM),
            ],
            condition_defs,
            vec![RuleStep::ConditionalBranch {
                condition: RuleCondition::AnyMatches(vec![pattern(vec![cell(
                    vec![KEY],
                    Vec::new(),
                )])]),
                then_steps: vec![RuleStep::Rule(relevant_branch)],
                else_steps: vec![RuleStep::Rule(dead_branch)],
            }],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [GATE]);

        assert_relevant(&relevance, GATE);
        assert_relevant(&relevance, SWITCH);
        assert_relevant(&relevance, KEY);
        assert_relevant(&relevance, BATTERY);
        assert_pruned(&relevance, SPARKLE);
        assert_pruned(&relevance, DECOY);
    }

    #[test]
    fn after_triggered_propagates_trigger_reads_without_promoting_trigger_writes() {
        const PRESSURE_PLATE: ObjectId = ObjectId(1);
        const LATCH: ObjectId = ObjectId(2);
        const DOOR: ObjectId = ObjectId(3);
        const FLASH: ObjectId = ObjectId(4);

        let trigger = rule(
            1,
            pattern(vec![cell(vec![PRESSURE_PLATE], Vec::new())]),
            Vec::new(),
            vec![add(FLASH)],
            Vec::new(),
        );
        let consequence = rule(
            2,
            pattern(vec![cell(vec![LATCH], Vec::new())]),
            Vec::new(),
            vec![add(DOOR)],
            Vec::new(),
        );
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, ACTOR),
                object(2, ITEM),
                object(3, ITEM),
                object(4, ITEM),
            ],
            vec![RuleStep::AfterTriggered {
                steps: vec![RuleStep::Rule(trigger)],
                then_steps: vec![RuleStep::Rule(consequence)],
            }],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);

        assert_relevant(&relevance, DOOR);
        assert_relevant(&relevance, LATCH);
        assert_relevant(&relevance, PRESSURE_PLATE);
        assert_pruned(&relevance, FLASH);
    }

    #[test]
    fn object_set_write_promotes_every_candidate_for_a_relevant_binding() {
        const BOX: ObjectId = ObjectId(1);
        const CRATE: ObjectId = ObjectId(2);
        const COIN: ObjectId = ObjectId(3);

        let move_solid = rule(
            1,
            pattern(vec![object_set_cell(0, ACTOR, vec![BOX, CRATE])]),
            Vec::new(),
            vec![move_object_set(0)],
            Vec::new(),
        );
        let game = CompiledGame::new_with_program(
            1,
            vec![object(1, ACTOR), object(2, ACTOR), object(3, ITEM)],
            vec![RuleStep::Rule(move_solid)],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [BOX]);

        assert_relevant(&relevance, BOX);
        assert_relevant(&relevance, CRATE);
        assert_pruned(&relevance, COIN);
    }

    #[test]
    fn projection_object_becomes_relevant_when_it_gates_a_relevant_write() {
        const FLOOR: ObjectId = ObjectId(1);
        const SWITCH: ObjectId = ObjectId(2);
        const DOOR: ObjectId = ObjectId(3);

        let projection = rule(
            1,
            pattern(vec![cell(Vec::new(), vec![FLOOR])]),
            Vec::new(),
            vec![add(FLOOR)],
            Vec::new(),
        );
        let open_door = rule(
            2,
            pattern(vec![cell(vec![SWITCH], Vec::new())]),
            Vec::new(),
            vec![add(DOOR)],
            Vec::new(),
        );
        let game = CompiledGame::new_with_program(
            2,
            vec![object(1, ITEM), object(2, ACTOR), object(3, ITEM)],
            vec![
                RuleStep::Rule(projection),
                RuleStep::ConditionalBlock {
                    condition: RuleCondition::AnyMatches(vec![pattern(vec![cell(
                        vec![FLOOR],
                        Vec::new(),
                    )])]),
                    steps: vec![RuleStep::Rule(open_door)],
                },
            ],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);

        assert_relevant(&relevance, DOOR);
        assert_relevant(&relevance, SWITCH);
        assert_relevant(&relevance, FLOOR);
    }

    #[test]
    fn rule_match_condition_keeps_pattern_and_guard_objects_relevant() {
        const KEY: ObjectId = ObjectId(1);
        const BATTERY: ObjectId = ObjectId(2);
        const SWITCH: ObjectId = ObjectId(3);
        const DOOR: ObjectId = ObjectId(4);

        let condition_defs = vec![ConditionDef {
            id: ConditionId(0),
            kind: ConditionValueKind::ExistsObjects(vec![BATTERY]),
        }];
        let open_door = rule(
            1,
            pattern(vec![cell(vec![SWITCH], Vec::new())]),
            Vec::new(),
            vec![add(DOOR)],
            Vec::new(),
        );
        let game = CompiledGame::new_with_condition_defs_and_program(
            2,
            vec![
                object(1, ITEM),
                object(2, ITEM),
                object(3, ACTOR),
                object(4, ITEM),
            ],
            condition_defs,
            vec![RuleStep::ConditionalBlock {
                condition: RuleCondition::RuleMatches {
                    guards: vec![Guard::ConditionNonZero(ConditionId(0))],
                    pattern: pattern(vec![cell(vec![KEY], Vec::new())]),
                },
                steps: vec![RuleStep::Rule(open_door)],
            }],
        );

        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);

        assert_relevant(&relevance, DOOR);
        assert_relevant(&relevance, SWITCH);
        assert_relevant(&relevance, KEY);
        assert_relevant(&relevance, BATTERY);
    }
}
