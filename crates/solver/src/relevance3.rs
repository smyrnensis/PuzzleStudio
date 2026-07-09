use crate::{object_refs, relevance::SolverRelevance};
use puzzle_grid3d::{ConditionValueKind3, Game3, Guard3, ObjectId, Pattern3, Rule3, RuleId3};

impl SolverRelevance<ObjectId> {
    pub fn from_game3_root_objects(
        game: &Game3,
        rules: &[Rule3],
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        let mut analysis = Self::from_roots(roots, ObjectId::is_empty);

        let mut changed = true;
        while changed {
            changed = false;
            for rule in rules {
                changed |= analysis.propagate_rule(game, rule);
            }
        }

        analysis
    }

    pub fn ignored_objects_for_game(&self, game: &Game3) -> Vec<ObjectId> {
        game.objects
            .iter()
            .filter_map(|object| {
                (!object.id.is_empty() && !self.contains_object(object.id)).then_some(object.id)
            })
            .collect()
    }

    pub fn relevant_rules(&self) -> Vec<RuleId3> {
        self.relevant_rule_ids().into_iter().map(RuleId3).collect()
    }

    fn propagate_rule(&mut self, game: &Game3, rule: &Rule3) -> bool {
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

    fn insert_guard_objects(&mut self, game: &Game3, guard: &Guard3) -> bool {
        match guard {
            Guard3::ConditionEquals { condition, .. }
            | Guard3::ConditionNonZero(condition)
            | Guard3::ConditionCompare { condition, .. } => game
                .condition_def(*condition)
                .is_some_and(|condition| self.insert_condition_value_objects(&condition.kind)),
            Guard3::InlineConditionValue { kind, .. }
            | Guard3::InlineConditionNonZero(kind)
            | Guard3::InlineConditionCompare { kind, .. } => {
                self.insert_condition_value_objects(kind)
            }
            Guard3::InputIs(_) | Guard3::VariableEquals { .. } | Guard3::VariableCompare { .. } => {
                false
            }
        }
    }

    fn insert_condition_value_objects(&mut self, kind: &ConditionValueKind3) -> bool {
        object_refs::insert_condition_value_objects(self, kind)
    }

    fn insert_pattern_objects(&mut self, pattern: &Pattern3) -> bool {
        object_refs::insert_pattern_objects(self, pattern)
    }
}

fn rule_has_relevant_output(rule: &Rule3, relevance: &SolverRelevance<ObjectId>) -> bool {
    !rule.effects.is_empty()
        || rule.writes.iter().any(|write| {
            object_refs::write_touches_relevant_object(write, &rule.pattern, relevance)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_grid3d::{
        LayerId, MatchCell3, ObjectDef3, Offset3, RuleApplication3, RuleId3, VariableId,
        VariableUpdateOp, WriteOp3,
    };

    const SWITCH: ObjectId = ObjectId(1);
    const DOOR: ObjectId = ObjectId(2);
    const SPARKLE: ObjectId = ObjectId(3);

    fn rule(id: u16, pattern: Pattern3, writes: Vec<WriteOp3>) -> Rule3 {
        Rule3 {
            id: RuleId3(id),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }

    fn add(object: ObjectId) -> WriteOp3 {
        WriteOp3::Add {
            component: 0,
            offset: Offset3::ZERO,
            object,
        }
    }

    #[test]
    fn relevance3_back_propagates_from_root_writes_to_rule_reads() {
        let game = Game3::new(
            1,
            vec![
                ObjectDef3 {
                    id: SWITCH,
                    layer_id: LayerId(0),
                },
                ObjectDef3 {
                    id: DOOR,
                    layer_id: LayerId(0),
                },
                ObjectDef3 {
                    id: SPARKLE,
                    layer_id: LayerId(0),
                },
            ],
        );
        let rules = vec![
            rule(
                1,
                Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(SWITCH)]),
                vec![add(DOOR)],
            ),
            rule(
                2,
                Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(SPARKLE)]),
                Vec::new(),
            ),
        ];

        let relevance = SolverRelevance::from_game3_root_objects(&game, &rules, [DOOR]);

        assert!(relevance.contains_object(DOOR));
        assert!(relevance.contains_object(SWITCH));
        assert!(!relevance.contains_object(SPARKLE));
        assert_eq!(relevance.relevant_rules(), vec![RuleId3(1)]);
    }

    #[test]
    fn relevance3_treats_variable_effect_rules_as_solver_visible() {
        let game = Game3::new(
            1,
            vec![ObjectDef3 {
                id: SWITCH,
                layer_id: LayerId(0),
            }],
        );
        let mut rule = rule(
            1,
            Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(SWITCH)]),
            Vec::new(),
        );
        rule.effects
            .push(puzzle_grid3d::RuleEffect3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            });

        let relevance = SolverRelevance::from_game3_root_objects(&game, &[rule], []);

        assert!(relevance.contains_object(SWITCH));
    }
}
