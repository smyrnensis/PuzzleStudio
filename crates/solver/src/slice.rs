use crate::{SolverRelevance, SolverStageAvailability};
use puzzle_core::{CompiledGame, ObjectId, Rule, RuleId, RuleStep};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverSlice {
    kept_objects: BTreeSet<ObjectId>,
    kept_rules: BTreeSet<RuleId>,
}

impl SolverSlice {
    pub fn from_relevance_and_availability(
        relevance: &SolverRelevance,
        availability: &SolverStageAvailability,
    ) -> Self {
        let available_objects = availability
            .available_objects()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let available_rules = availability
            .available_rules()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let kept_objects = relevance
            .relevant_objects()
            .into_iter()
            .filter(|object| available_objects.contains(object))
            .collect();
        let kept_rules = relevance
            .relevant_rules()
            .into_iter()
            .filter(|rule| available_rules.contains(rule))
            .collect();
        Self {
            kept_objects,
            kept_rules,
        }
    }

    pub fn kept_objects(&self) -> &BTreeSet<ObjectId> {
        &self.kept_objects
    }

    pub fn kept_rules(&self) -> &BTreeSet<RuleId> {
        &self.kept_rules
    }

    pub fn project_game(&self, game: &CompiledGame) -> CompiledGame {
        assert_unique_rule_ids(game.rules());
        CompiledGame::new_with_mark_condition_defs_and_program(
            game.layer_count,
            game.objects().to_vec(),
            game.mark().to_vec(),
            game.condition_defs().to_vec(),
            self.filter_program(game.program()),
        )
    }

    fn filter_program(&self, program: &[RuleStep]) -> Vec<RuleStep> {
        program
            .iter()
            .filter_map(|step| self.filter_step(step))
            .collect()
    }

    fn filter_step(&self, step: &RuleStep) -> Option<RuleStep> {
        match step {
            RuleStep::Rule(rule) => self
                .kept_rules
                .contains(&rule.id)
                .then(|| RuleStep::Rule(rule.clone())),
            RuleStep::ConditionalBlock { condition, steps } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| RuleStep::ConditionalBlock {
                    condition: condition.clone(),
                    steps,
                })
            }
            RuleStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let then_steps = self.filter_program(then_steps);
                let else_steps = self.filter_program(else_steps);
                (!then_steps.is_empty() || !else_steps.is_empty()).then(|| {
                    RuleStep::ConditionalBranch {
                        condition: condition.clone(),
                        then_steps,
                        else_steps,
                    }
                })
            }
            RuleStep::Block {
                application,
                stop_condition,
                steps,
            } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| RuleStep::Block {
                    application: *application,
                    stop_condition: stop_condition.clone(),
                    steps,
                })
            }
            RuleStep::AfterTriggered { steps, then_steps } => {
                let filtered_steps = self.filter_program(steps);
                let filtered_then_steps = self.filter_program(then_steps);
                if !filtered_then_steps.is_empty() {
                    // The complete trigger program determines whether the relevant
                    // continuation runs; pruning it would change that predicate.
                    Some(RuleStep::AfterTriggered {
                        steps: steps.clone(),
                        then_steps: filtered_then_steps,
                    })
                } else if !filtered_steps.is_empty() {
                    // The trigger itself can contain relevant writes even when its
                    // continuation is irrelevant. Keep the wrapper so application
                    // and fired-state semantics remain unchanged.
                    Some(RuleStep::AfterTriggered {
                        steps: filtered_steps,
                        then_steps: Vec::new(),
                    })
                } else {
                    None
                }
            }
            RuleStep::LocalFrame { frame, steps } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| RuleStep::LocalFrame {
                    frame: frame.clone(),
                    steps,
                })
            }
        }
    }
}

fn assert_unique_rule_ids(rules: &[Rule]) {
    let mut seen = BTreeSet::new();
    for rule in rules {
        assert!(
            seen.insert(rule.id),
            "solver rule pruning requires unique RuleId values; duplicate {:?}",
            rule.id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SolverRelevance, SolverStageAvailability};
    use puzzle_core::{
        CompiledGame, Effect, InputId, LayerId, MatchCell, ObjectDef, Offset, Pattern,
        PatternComponent, Rule, RuleApplication, RuleId, RuleStep, State, WriteOp,
        transition_state,
    };

    const PLAYER: ObjectId = ObjectId(1);
    const SWITCH: ObjectId = ObjectId(2);
    const BATTERY: ObjectId = ObjectId(3);
    const DOOR: ObjectId = ObjectId(4);

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

    fn pattern(object: ObjectId) -> Pattern {
        Pattern {
            components: vec![PatternComponent {
                cells: vec![MatchCell {
                    offset: fixed(0, 0),
                    require_null: false,
                    require_objects: vec![object],
                    require_object_sets: Vec::new(),
                    forbid_objects: Vec::new(),
                    require_mark: Vec::new(),
                    require_object_set_mark: Vec::new(),
                    forbid_mark: Vec::new(),
                    forbid_object_set_mark: Vec::new(),
                }],
                gap_count: 0,
            }],
        }
    }

    fn add(object: ObjectId) -> WriteOp {
        WriteOp::Add {
            component: 0,
            offset: fixed(0, 0),
            object,
        }
    }

    fn rule(id: u16, read: ObjectId, write: ObjectId) -> Rule {
        Rule {
            id: RuleId(id),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(read),
            writes: vec![add(write)],
            effects: Vec::new(),
        }
    }

    fn effect_rule(id: u16, effect: Effect) -> Rule {
        Rule {
            id: RuleId(id),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![effect],
        }
    }

    #[test]
    fn solver_slice_intersects_relevance_with_stage_availability() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![
                RuleStep::Rule(rule(1, SWITCH, DOOR)),
                RuleStep::Rule(rule(2, BATTERY, DOOR)),
            ],
        );
        let mut initial = State::empty(3, 1, 2, 4).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        initial.place_object(&game, 1, 0, SWITCH).unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);

        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);

        assert!(slice.kept_objects().contains(&SWITCH));
        assert!(slice.kept_objects().contains(&DOOR));
        assert!(!slice.kept_objects().contains(&BATTERY));
        assert!(slice.kept_rules().contains(&RuleId(1)));
        assert!(!slice.kept_rules().contains(&RuleId(2)));
    }

    #[test]
    fn solver_slice_projects_game_program_to_kept_rules() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![
                RuleStep::Rule(rule(1, SWITCH, DOOR)),
                RuleStep::Rule(rule(2, BATTERY, DOOR)),
            ],
        );
        let mut initial = State::empty(3, 1, 2, 4).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        initial.place_object(&game, 1, 0, SWITCH).unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);
        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);

        let projected = slice.project_game(&game);

        assert_eq!(projected.rules().len(), 1);
        assert_eq!(projected.rules()[0].id, RuleId(1));
        assert_eq!(projected.program().len(), 1);
    }

    #[test]
    fn solver_slice_preserves_after_triggered_trigger_steps_for_relevant_then_steps() {
        let game = CompiledGame::new_with_program(
            2,
            vec![object(1, LayerId(0)), object(2, LayerId(1))],
            vec![RuleStep::AfterTriggered {
                steps: vec![RuleStep::Rule(rule(1, PLAYER, SWITCH))],
                then_steps: vec![RuleStep::Rule(effect_rule(2, Effect::Win))],
            }],
        );
        let mut initial = State::empty(2, 1, 2, 2).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, []);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);
        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);

        let projected = slice.project_game(&game);

        assert_eq!(projected.rules().len(), 2);
        assert!(projected.rules().iter().any(|rule| rule.id == RuleId(1)));
        assert!(projected.rules().iter().any(|rule| rule.id == RuleId(2)));
        let [RuleStep::AfterTriggered { steps, then_steps }] = projected.program() else {
            panic!("expected projected after-triggered step");
        };
        assert_eq!(steps.len(), 1);
        assert_eq!(then_steps.len(), 1);
    }

    #[test]
    fn solver_slice_keeps_relevant_after_triggered_steps_when_continuation_is_pruned() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
            ],
            vec![RuleStep::AfterTriggered {
                steps: vec![RuleStep::Rule(rule(1, PLAYER, SWITCH))],
                then_steps: vec![RuleStep::Rule(rule(2, PLAYER, BATTERY))],
            }],
        );
        let mut initial = State::empty(2, 1, 2, 3).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, [SWITCH]);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);
        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);

        let projected = slice.project_game(&game);

        let [RuleStep::AfterTriggered { steps, then_steps }] = projected.program() else {
            panic!("expected projected after-triggered step");
        };
        assert_eq!(steps.len(), 1);
        assert!(then_steps.is_empty());
        let outcome = transition_state(&projected, &initial, InputId(0)).unwrap();
        assert_eq!(outcome.object_count(SWITCH), 1);
        assert_eq!(outcome.object_count(BATTERY), 0);
    }

    #[test]
    fn solver_slice_keeps_once_per_level_state_for_relevant_available_rule() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![RuleStep::Rule(Rule {
                application: RuleApplication::OncePerLevel,
                ..rule(1, PLAYER, DOOR)
            })],
        );
        let mut initial = State::empty(1, 1, 2, 4).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);
        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);
        let projected = slice.project_game(&game);

        let first = transition_state(&projected, &initial, InputId(0)).unwrap();
        let second = transition_state(&projected, &first, InputId(0)).unwrap();

        assert!(first.level_rule_has_fired(RuleId(1)));
        assert_eq!(first.object_count(DOOR), 1);
        assert_eq!(second.object_count(DOOR), 1);
    }

    #[test]
    #[should_panic(expected = "solver rule pruning requires unique RuleId values")]
    fn solver_slice_rejects_duplicate_rule_ids() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![
                RuleStep::Rule(rule(1, SWITCH, DOOR)),
                RuleStep::Rule(rule(1, BATTERY, DOOR)),
            ],
        );
        let slice = SolverSlice {
            kept_objects: BTreeSet::from([SWITCH, BATTERY, DOOR]),
            kept_rules: BTreeSet::from([RuleId(1)]),
        };

        let _ = slice.project_game(&game);
    }
}
