use puzzle_core::{
    CompiledGame, ConditionValueKind, Effect, Guard, MatchCell, ObjectId, Pattern, Rule, RuleStep,
    WriteOp,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverRelevance {
    relevant_objects: BTreeSet<ObjectId>,
}

impl SolverRelevance {
    pub fn from_root_objects(
        game: &CompiledGame,
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        let mut analysis = Self::default();
        for object in roots {
            analysis.insert_object(object);
        }

        let mut changed = true;
        while changed {
            changed = false;
            for step in game.program() {
                changed |= analysis.propagate_step(game, step);
            }
        }

        analysis
    }

    pub fn contains_object(&self, object: ObjectId) -> bool {
        self.relevant_objects.contains(&object)
    }

    pub fn relevant_objects(&self) -> Vec<ObjectId> {
        self.relevant_objects.iter().copied().collect()
    }

    pub fn ignored_objects_for_game(&self, game: &CompiledGame) -> Vec<ObjectId> {
        game.objects()
            .iter()
            .filter_map(|object| {
                (!object.id.is_empty() && !self.contains_object(object.id)).then_some(object.id)
            })
            .collect()
    }

    fn propagate_step(&mut self, game: &CompiledGame, step: &RuleStep) -> bool {
        match step {
            RuleStep::Rule(rule) => self.propagate_rule(game, rule),
            RuleStep::ConditionalBlock { condition, steps } => {
                let mut changed = false;
                if steps.iter().any(|step| step_has_relevant_write(step, self)) {
                    changed |= self.insert_condition_objects(game, condition);
                }
                for step in steps {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
            RuleStep::ConditionalBranch {
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
            RuleStep::Block {
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
            RuleStep::AfterTriggered { steps, then_steps } => {
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
            RuleStep::LocalFrame { steps, .. } => {
                let mut changed = false;
                for step in steps {
                    changed |= self.propagate_step(game, step);
                }
                changed
            }
        }
    }

    fn propagate_rule(&mut self, game: &CompiledGame, rule: &Rule) -> bool {
        if !rule_has_relevant_output(rule, self) {
            return false;
        }

        let mut changed = self.insert_pattern_objects(&rule.pattern);
        for guard in &rule.guards {
            changed |= self.insert_guard_objects(game, guard);
        }
        changed
    }

    fn insert_step_read_objects(&mut self, game: &CompiledGame, step: &RuleStep) -> bool {
        match step {
            RuleStep::Rule(rule) => {
                let mut changed = self.insert_pattern_objects(&rule.pattern);
                for guard in &rule.guards {
                    changed |= self.insert_guard_objects(game, guard);
                }
                changed
            }
            RuleStep::ConditionalBlock { condition, steps } => {
                let mut changed = self.insert_condition_objects(game, condition);
                for step in steps {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            RuleStep::ConditionalBranch {
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
            RuleStep::Block {
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
            RuleStep::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                for step in steps.iter().chain(then_steps) {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
            RuleStep::LocalFrame { steps, .. } => {
                let mut changed = false;
                for step in steps {
                    changed |= self.insert_step_read_objects(game, step);
                }
                changed
            }
        }
    }

    fn insert_guard_objects(&mut self, game: &CompiledGame, guard: &Guard) -> bool {
        match guard {
            Guard::ConditionEquals { condition, .. }
            | Guard::ConditionNonZero(condition)
            | Guard::ConditionCompare { condition, .. } => game
                .condition_def(*condition)
                .is_some_and(|condition| self.insert_condition_value_objects(&condition.kind)),
            Guard::InlineConditionValue { kind, .. }
            | Guard::InlineConditionNonZero(kind)
            | Guard::InlineConditionCompare { kind, .. } => {
                self.insert_condition_value_objects(kind)
            }
            Guard::InputIs(_) | Guard::VariableEquals { .. } | Guard::VariableCompare { .. } => {
                false
            }
        }
    }

    fn insert_condition_objects(
        &mut self,
        game: &CompiledGame,
        condition: &puzzle_core::RuleCondition,
    ) -> bool {
        match condition {
            puzzle_core::RuleCondition::AnyMatches(patterns)
            | puzzle_core::RuleCondition::NoMatches(patterns) => {
                self.insert_patterns_objects(patterns)
            }
            puzzle_core::RuleCondition::AnyInputMatches(patterns)
            | puzzle_core::RuleCondition::NoInputMatches(patterns) => {
                self.insert_patterns_objects(patterns.iter().map(|(_, pattern)| pattern))
            }
            puzzle_core::RuleCondition::GuardBranches(branches) => {
                let mut changed = false;
                for guard in branches.iter().flatten() {
                    changed |= self.insert_guard_objects(game, guard);
                }
                changed
            }
        }
    }

    fn insert_condition_value_objects(&mut self, kind: &ConditionValueKind) -> bool {
        match kind {
            ConditionValueKind::CountObjects(objects)
            | ConditionValueKind::ExistsObjects(objects)
            | ConditionValueKind::NoneObjects(objects) => self.insert_objects(objects),
            ConditionValueKind::CountMatches(patterns)
            | ConditionValueKind::ExistsMatches(patterns)
            | ConditionValueKind::NoneMatches(patterns) => self.insert_patterns_objects(patterns),
            ConditionValueKind::CountInputMatches(patterns)
            | ConditionValueKind::ExistsInputMatches(patterns)
            | ConditionValueKind::NoneInputMatches(patterns) => {
                self.insert_patterns_objects(patterns.iter().map(|(_, pattern)| pattern))
            }
        }
    }

    fn insert_patterns_objects<'a>(
        &mut self,
        patterns: impl IntoIterator<Item = &'a Pattern>,
    ) -> bool {
        let mut changed = false;
        for pattern in patterns {
            changed |= self.insert_pattern_objects(pattern);
        }
        changed
    }

    fn insert_pattern_objects(&mut self, pattern: &Pattern) -> bool {
        let mut changed = false;
        for component in &pattern.components {
            for cell in &component.cells {
                changed |= self.insert_match_cell_objects(cell);
            }
        }
        changed
    }

    fn insert_match_cell_objects(&mut self, cell: &MatchCell) -> bool {
        let mut changed = false;
        changed |= self.insert_objects(&cell.require_objects);
        changed |= self.insert_objects(&cell.forbid_objects);
        for matcher in &cell.require_object_sets {
            changed |= self.insert_objects(&matcher.objects);
        }
        for mark in cell.require_mark.iter().chain(&cell.forbid_mark) {
            changed |= self.insert_object(mark.object);
        }
        changed
    }

    fn insert_objects<'a>(&mut self, objects: impl IntoIterator<Item = &'a ObjectId>) -> bool {
        let mut changed = false;
        for object in objects {
            changed |= self.insert_object(*object);
        }
        changed
    }

    fn insert_object(&mut self, object: ObjectId) -> bool {
        !object.is_empty() && self.relevant_objects.insert(object)
    }
}

fn step_has_relevant_write(step: &RuleStep, relevance: &SolverRelevance) -> bool {
    match step {
        RuleStep::Rule(rule) => rule_has_relevant_output(rule, relevance),
        RuleStep::ConditionalBlock { steps, .. }
        | RuleStep::Block { steps, .. }
        | RuleStep::LocalFrame { steps, .. } => steps
            .iter()
            .any(|step| step_has_relevant_write(step, relevance)),
        RuleStep::ConditionalBranch {
            then_steps,
            else_steps,
            ..
        } => then_steps
            .iter()
            .chain(else_steps)
            .any(|step| step_has_relevant_write(step, relevance)),
        RuleStep::AfterTriggered { steps, then_steps } => steps
            .iter()
            .chain(then_steps)
            .any(|step| step_has_relevant_write(step, relevance)),
    }
}

fn rule_has_relevant_output(rule: &Rule, relevance: &SolverRelevance) -> bool {
    rule.effects.iter().any(effect_is_solver_visible)
        || rule
            .writes
            .iter()
            .any(|write| write_touches_relevant_object(write, &rule.pattern, relevance))
}

fn effect_is_solver_visible(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::Win
            | Effect::Restart
            | Effect::NextLevel
            | Effect::Checkpoint
            | Effect::ClearCheckpoint
            | Effect::UpdateVariable { .. }
    )
}

fn write_touches_relevant_object(
    write: &WriteOp,
    pattern: &Pattern,
    relevance: &SolverRelevance,
) -> bool {
    match write {
        WriteOp::Add { object, .. }
        | WriteOp::Remove { object, .. }
        | WriteOp::Move { object, .. }
        | WriteOp::SetMark { object, .. }
        | WriteOp::RemoveMark { object, .. } => relevance.contains_object(*object),
        WriteOp::Replace { remove, add, .. } => {
            relevance.contains_object(*remove) || relevance.contains_object(*add)
        }
        WriteOp::AddObjectSet { binding, .. }
        | WriteOp::RemoveObjectSet { binding, .. }
        | WriteOp::MoveObjectSet { binding, .. }
        | WriteOp::SetObjectSetMark { binding, .. }
        | WriteOp::RemoveObjectSetMark { binding, .. } => {
            binding_touches_relevant_object(pattern, *binding, relevance)
        }
    }
}

fn binding_touches_relevant_object(
    pattern: &Pattern,
    binding: u16,
    relevance: &SolverRelevance,
) -> bool {
    pattern.components.iter().any(|component| {
        component.cells.iter().any(|cell| {
            cell.require_object_sets.iter().any(|matcher| {
                matcher.binding == binding
                    && matcher
                        .objects
                        .iter()
                        .any(|object| relevance.contains_object(*object))
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_core::ObjectId;
    use puzzle_lang::{LoadedGame, parse_game2d as parse_game};

    fn object_named(loaded: &LoadedGame, name: &str) -> ObjectId {
        loaded
            .object_labels
            .iter()
            .find_map(|(id, label)| (label == name).then_some(*id))
            .unwrap_or_else(|| panic!("missing object {name}"))
    }

    #[test]
    fn relevance_back_propagates_from_root_writes_to_rule_reads() {
        let source = r#"
title = relevance_backprop

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
title = relevance_projection

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
}
