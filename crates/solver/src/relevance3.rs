use crate::{object_refs, relevance::SolverRelevance};
use puzzle_grid3d::{
    CompiledGame3, ConditionValueKind3, Guard3, ObjectId, Pattern3, Rule3, RuleCondition3, RuleId3,
    RuleStep3,
};

impl SolverRelevance<ObjectId> {
    pub fn from_game3_root_objects(
        game: &CompiledGame3,
        program: &[RuleStep3],
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        let mut analysis = Self::from_roots(roots, ObjectId::is_empty);

        let mut changed = true;
        while changed {
            changed = false;
            for step in program {
                changed |= analysis.propagate_step3(game, step);
            }
        }

        analysis
    }

    pub fn ignored_objects_for_game3(&self, game: &CompiledGame3) -> Vec<ObjectId> {
        game.objects()
            .iter()
            .filter_map(|object| {
                (!object.id.is_empty() && !self.contains_object(object.id)).then_some(object.id)
            })
            .collect()
    }

    pub fn relevant_rules3(&self) -> Vec<RuleId3> {
        self.relevant_rule_ids().into_iter().map(RuleId3).collect()
    }

    fn propagate_step3(&mut self, game: &CompiledGame3, step: &RuleStep3) -> bool {
        match step {
            RuleStep3::Rule(rule) => self.propagate_rule3(game, rule),
            RuleStep3::ConditionalBlock { condition, steps } => {
                let mut changed = false;
                if steps
                    .iter()
                    .any(|step| step_has_relevant_write3(step, self))
                {
                    changed |= self.insert_condition_objects3(game, condition);
                }
                for step in steps {
                    changed |= self.propagate_step3(game, step);
                }
                changed
            }
            RuleStep3::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let mut changed = false;
                if then_steps
                    .iter()
                    .chain(else_steps)
                    .any(|step| step_has_relevant_write3(step, self))
                {
                    changed |= self.insert_condition_objects3(game, condition);
                }
                for step in then_steps.iter().chain(else_steps) {
                    changed |= self.propagate_step3(game, step);
                }
                changed
            }
            RuleStep3::Block {
                stop_condition,
                steps,
                ..
            } => {
                let mut changed = false;
                if let Some(condition) = stop_condition
                    && steps
                        .iter()
                        .any(|step| step_has_relevant_write3(step, self))
                {
                    changed |= self.insert_condition_objects3(game, condition);
                }
                for step in steps {
                    changed |= self.propagate_step3(game, step);
                }
                changed
            }
            RuleStep3::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                if then_steps
                    .iter()
                    .any(|step| step_has_relevant_write3(step, self))
                {
                    for step in steps {
                        changed |= self.insert_step_read_objects3(game, step);
                    }
                }
                for step in steps.iter().chain(then_steps) {
                    changed |= self.propagate_step3(game, step);
                }
                changed
            }
            RuleStep3::LocalFrame { frame, steps } => {
                let mut changed = false;
                if steps
                    .iter()
                    .any(|step| step_has_relevant_write3(step, self))
                {
                    changed |=
                        self.insert_relevant_objects(&frame.focus_objects, &ObjectId::is_empty);
                }
                for step in steps {
                    changed |= self.propagate_step3(game, step);
                }
                changed
            }
        }
    }

    fn propagate_rule3(&mut self, game: &CompiledGame3, rule: &Rule3) -> bool {
        if !rule_has_relevant_output(rule, self) {
            return false;
        }

        let mut changed = self.insert_relevant_rule_id(rule.id.0);
        changed |= self.insert_pattern_objects3(&rule.pattern);
        for guard in &rule.guards {
            changed |= self.insert_guard_objects3(game, guard);
        }
        changed
    }

    fn insert_step_read_objects3(&mut self, game: &CompiledGame3, step: &RuleStep3) -> bool {
        match step {
            RuleStep3::Rule(rule) => {
                let mut changed = self.insert_relevant_rule_id(rule.id.0);
                changed |= self.insert_pattern_objects3(&rule.pattern);
                for guard in &rule.guards {
                    changed |= self.insert_guard_objects3(game, guard);
                }
                changed
            }
            RuleStep3::ConditionalBlock { condition, steps } => {
                let mut changed = self.insert_condition_objects3(game, condition);
                for step in steps {
                    changed |= self.insert_step_read_objects3(game, step);
                }
                changed
            }
            RuleStep3::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let mut changed = self.insert_condition_objects3(game, condition);
                for step in then_steps.iter().chain(else_steps) {
                    changed |= self.insert_step_read_objects3(game, step);
                }
                changed
            }
            RuleStep3::Block {
                stop_condition,
                steps,
                ..
            } => {
                let mut changed = stop_condition
                    .as_ref()
                    .is_some_and(|condition| self.insert_condition_objects3(game, condition));
                for step in steps {
                    changed |= self.insert_step_read_objects3(game, step);
                }
                changed
            }
            RuleStep3::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                for step in steps.iter().chain(then_steps) {
                    changed |= self.insert_step_read_objects3(game, step);
                }
                changed
            }
            RuleStep3::LocalFrame { frame, steps } => {
                let mut changed =
                    self.insert_relevant_objects(&frame.focus_objects, &ObjectId::is_empty);
                for step in steps {
                    changed |= self.insert_step_read_objects3(game, step);
                }
                changed
            }
        }
    }

    fn insert_guard_objects3(&mut self, game: &CompiledGame3, guard: &Guard3) -> bool {
        match guard {
            Guard3::ConditionEquals { condition, .. }
            | Guard3::ConditionNonZero(condition)
            | Guard3::ConditionCompare { condition, .. } => game
                .condition_def(*condition)
                .is_some_and(|condition| self.insert_condition_value_objects3(&condition.kind)),
            Guard3::InlineConditionValue { kind, .. }
            | Guard3::InlineConditionNonZero(kind)
            | Guard3::InlineConditionCompare { kind, .. } => {
                self.insert_condition_value_objects3(kind)
            }
            Guard3::InputIs(_) | Guard3::VariableEquals { .. } | Guard3::VariableCompare { .. } => {
                false
            }
        }
    }

    fn insert_condition_objects3(
        &mut self,
        game: &CompiledGame3,
        condition: &RuleCondition3,
    ) -> bool {
        match condition {
            RuleCondition3::AnyMatches(patterns) | RuleCondition3::NoMatches(patterns) => {
                object_refs::insert_patterns_objects(self, patterns)
            }
            RuleCondition3::AnyInputMatches(patterns)
            | RuleCondition3::NoInputMatches(patterns) => object_refs::insert_patterns_objects(
                self,
                patterns.iter().map(|(_, pattern)| pattern),
            ),
            RuleCondition3::GuardBranches(branches) => {
                branches.iter().flatten().fold(false, |changed, guard| {
                    changed | self.insert_guard_objects3(game, guard)
                })
            }
        }
    }

    fn insert_condition_value_objects3(&mut self, kind: &ConditionValueKind3) -> bool {
        object_refs::insert_condition_value_objects(self, kind)
    }

    fn insert_pattern_objects3(&mut self, pattern: &Pattern3) -> bool {
        object_refs::insert_pattern_objects(self, pattern)
    }
}

fn step_has_relevant_write3(step: &RuleStep3, relevance: &SolverRelevance<ObjectId>) -> bool {
    match step {
        RuleStep3::Rule(rule) => rule_has_relevant_output(rule, relevance),
        RuleStep3::ConditionalBlock { steps, .. }
        | RuleStep3::Block { steps, .. }
        | RuleStep3::LocalFrame { steps, .. } => steps
            .iter()
            .any(|step| step_has_relevant_write3(step, relevance)),
        RuleStep3::ConditionalBranch {
            then_steps,
            else_steps,
            ..
        } => then_steps
            .iter()
            .chain(else_steps)
            .any(|step| step_has_relevant_write3(step, relevance)),
        RuleStep3::AfterTriggered { steps, then_steps } => steps
            .iter()
            .chain(then_steps)
            .any(|step| step_has_relevant_write3(step, relevance)),
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
        Delta3, LayerId, MatchCell3, ObjectDef3, RuleApplication3, RuleId3, VariableId,
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
            offset: Delta3::ZERO,
            object,
        }
    }

    #[test]
    fn relevance3_back_propagates_from_root_writes_to_rule_reads() {
        let game = CompiledGame3::new(
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
            Vec::new(),
        );
        let rules = vec![
            RuleStep3::Rule(rule(
                1,
                Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(SWITCH)]),
                vec![add(DOOR)],
            )),
            RuleStep3::Rule(rule(
                2,
                Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(SPARKLE)]),
                Vec::new(),
            )),
        ];

        let relevance = SolverRelevance::from_game3_root_objects(&game, &rules, [DOOR]);

        assert!(relevance.contains_object(DOOR));
        assert!(relevance.contains_object(SWITCH));
        assert!(!relevance.contains_object(SPARKLE));
        assert_eq!(relevance.relevant_rules3(), vec![RuleId3(1)]);
    }

    #[test]
    fn relevance3_treats_variable_effect_rules_as_solver_visible() {
        let game = CompiledGame3::new(
            1,
            vec![ObjectDef3 {
                id: SWITCH,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let mut rule = rule(
            1,
            Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(SWITCH)]),
            Vec::new(),
        );
        rule.effects
            .push(puzzle_grid3d::RuleEffect3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            });

        let relevance =
            SolverRelevance::from_game3_root_objects(&game, &[RuleStep3::Rule(rule)], []);

        assert!(relevance.contains_object(SWITCH));
    }

    #[test]
    fn relevance3_keeps_objects_used_only_by_program_conditions() {
        let game = CompiledGame3::new(
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
            Vec::new(),
        );
        let program = vec![RuleStep3::ConditionalBlock {
            condition: RuleCondition3::AnyMatches(vec![Pattern3::new(vec![
                MatchCell3::new(Delta3::ZERO).require(SPARKLE),
            ])]),
            steps: vec![RuleStep3::Rule(rule(
                1,
                Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(SWITCH)]),
                vec![add(DOOR)],
            ))],
        }];

        let relevance = SolverRelevance::from_game3_root_objects(&game, &program, [DOOR]);

        assert!(relevance.contains_object(SPARKLE));
        assert!(relevance.contains_object(SWITCH));
    }
}
