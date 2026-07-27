#[cfg(test)]
mod tests {
    use crate::compiled_game::{
        Effect, Guard, MarkDef, MarkKind, MarkPattern, MarkValueMatch, MatchCell, ObjectDef,
        Offset, Pattern, PatternComponent, Rule, RuleApplication, VariableUpdateOp, WriteOp,
    };
    use crate::ids::{InputId, LayerId, MarkId, ObjectId, RuleId, VariableId};
    use crate::state::GridExecutionState;
    use crate::{
        CompiledGame, RuleStep, State, replay_rule_firing_states,
        transition_program_continuation_segment_trace, transition_program_segment_trace,
        transition_state, transition_trace,
    };
    use puzzle_kernel::GridCoord;

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const MARK: MarkId = MarkId(1);
    const RIGHT: InputId = InputId(1);

    fn fired_rules(trace: &crate::TransitionOutcome) -> Vec<RuleId> {
        trace.firings.iter().map(|firing| firing.rule).collect()
    }

    fn fixed(dx: i16, dy: i16) -> Offset {
        Offset::Fixed {
            delta: [dx, dy].into(),
        }
    }

    fn cell(
        dx: i16,
        dy: i16,
        require_objects: Vec<ObjectId>,
        forbid_objects: Vec<ObjectId>,
    ) -> MatchCell {
        MatchCell {
            offset: fixed(dx, dy),
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

    fn variable_rule(
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

    fn set_variable(variable: u16, value: i64) -> Effect {
        Effect::UpdateVariable {
            variable: VariableId(variable),
            op: VariableUpdateOp::Set,
            value,
        }
    }

    fn add_variable(variable: u16, value: i64) -> Effect {
        Effect::UpdateVariable {
            variable: VariableId(variable),
            op: VariableUpdateOp::Add,
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
    fn random_rule_is_deterministic_for_same_state() {
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
                layer_id: LayerId(2),
            },
        ];
        let random_player_to_box = Rule {
            id: RuleId(7),
            guards: Vec::new(),
            application: RuleApplication::Random,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game =
            CompiledGame::new_with_program(3, objects, vec![RuleStep::Rule(random_player_to_box)]);
        let mut plain = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        plain.place_object(&game, 0, 0, PLAYER).unwrap();
        plain.place_object(&game, 2, 0, PLAYER).unwrap();

        let first = transition_state(&game, &plain, RIGHT).unwrap();
        let repeated = transition_state(&game, &plain, RIGHT).unwrap();

        assert_eq!(first, repeated);
        assert_eq!(first.object_count(BOX), 1);
        assert_eq!(first.object_count(PLAYER), 1);
    }

    #[test]
    fn random_rule_treats_idempotent_lhs_match_as_a_firing_candidate() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let identity = Rule {
            id: RuleId(29),
            guards: Vec::new(),
            application: RuleApplication::Random,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, PLAYER)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new(2, objects, vec![identity]);
        let mut state = State::empty(2, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.next_state, state);
        assert_eq!(fired_rules(&trace), vec![RuleId(29)]);
        assert!(!trace.progressed);
        assert!(!trace.observable);
    }

    #[test]
    fn random_block_applies_one_firing_step() {
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
        let left_player_to_box = Rule {
            id: RuleId(10),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let right_player_to_box = Rule {
            id: RuleId(11),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(1, 0, vec![PLAYER], vec![])]),
            writes: vec![replace(1, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new_with_program(
            2,
            objects,
            vec![RuleStep::Block {
                application: RuleApplication::Random,
                stop_condition: None,
                steps: vec![
                    RuleStep::Rule(left_player_to_box),
                    RuleStep::Rule(right_player_to_box),
                ],
            }],
        );
        let mut state = State::empty(2, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();
        state.place_object(&game, 1, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.object_count(BOX), 1);
        assert_eq!(next.object_count(PLAYER), 1);
    }

    fn mark_anchor_game() -> CompiledGame {
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
        let mark = vec![MarkDef {
            id: MARK,
            kind: MarkKind::Flag,
            values: Vec::new(),
        }];
        CompiledGame::new_with_mark_condition_defs_and_program(3, objects, mark, Vec::new(), vec![])
    }

    #[test]
    fn mark_position_cache_tracks_slot_mark_moves_and_clears() {
        let game = mark_anchor_game();
        let mut committed = State::empty(4, 1, game.layer_count, game.object_count()).unwrap();
        committed.place_object(&game, 1, 0, BOX).unwrap();
        let mut state = GridExecutionState::new(committed);

        state.set_mark_unchecked(GridCoord::new([1, 0]), LayerId(2), MARK, Some(7));
        assert_eq!(
            state
                .mark_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(1, 0)]
        );

        let mark = state.take_slot_for_move_unchecked(GridCoord::new([1, 0]), LayerId(2));
        state.place_moved_slot_unchecked(GridCoord::new([3, 0]), LayerId(2), BOX, mark);
        assert_eq!(
            state
                .mark_positions(BOX, MARK, Some(7))
                .iter()
                .filter_map(|slot| state.slot_position(*slot))
                .collect::<Vec<_>>(),
            vec![(3, 0)]
        );

        state.remove_mark_unchecked(GridCoord::new([3, 0]), LayerId(2), MARK, Some(7));
        assert!(state.mark_positions(BOX, MARK, Some(7)).is_empty());

        state.set_mark_unchecked(GridCoord::new([3, 0]), LayerId(2), MARK, Some(9));
        let resumed_without_scratch = GridExecutionState::new(state.into_committed());
        assert!(
            resumed_without_scratch
                .mark_positions(BOX, MARK, Some(9))
                .is_empty()
        );
    }

    #[test]
    fn program_continuation_keeps_scratch_marks_until_the_program_finishes() {
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
        let marks = vec![MarkDef {
            id: MARK,
            kind: MarkKind::Flag,
            values: Vec::new(),
        }];
        let set_mark = Rule {
            id: RuleId(40),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![WriteOp::SetMark {
                component: 0,
                offset: fixed(0, 0),
                object: PLAYER,
                mark: MARK,
                value: None,
            }],
            effects: Vec::new(),
        };
        let mut marked_player = cell(0, 0, vec![PLAYER], vec![]);
        marked_player.require_mark.push(MarkPattern {
            object: PLAYER,
            mark: MARK,
            value: None,
            match_value: MarkValueMatch::Exact,
        });
        let consume_mark = Rule {
            id: RuleId(41),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![marked_player]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new_with_mark_condition_defs_and_program(
            2,
            objects,
            marks,
            Vec::new(),
            vec![RuleStep::Rule(set_mark), RuleStep::Rule(consume_mark)],
        );
        let mut initial = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();

        let first = transition_program_segment_trace(
            &game,
            game.executable_program(),
            &initial,
            Some(RIGHT),
            None,
            |boundary| boundary.firings.len() == 1,
        )
        .unwrap();
        assert_eq!(first.trace.next_state, initial);
        let continuation = first
            .remaining_program
            .expect("program paused after mark rule");

        let resumed = transition_program_continuation_segment_trace(
            &game,
            game.executable_program(),
            &continuation,
            &first.trace.next_state,
            Some(RIGHT),
            None,
            |_| false,
        )
        .unwrap();
        assert_eq!(resumed.trace.next_state.object_count(PLAYER), 0);
        assert_eq!(resumed.trace.next_state.object_count(BOX), 1);
        assert!(resumed.remaining_program.is_none());
    }

    #[test]
    fn firing_cursor_replay_keeps_transient_marks_that_enable_later_firings() {
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
        let marks = vec![MarkDef {
            id: MARK,
            kind: MarkKind::Flag,
            values: Vec::new(),
        }];
        let set_mark = Rule {
            id: RuleId(40),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![WriteOp::SetMark {
                component: 0,
                offset: fixed(0, 0),
                object: PLAYER,
                mark: MARK,
                value: None,
            }],
            effects: Vec::new(),
        };
        let mut marked_player = cell(0, 0, vec![PLAYER], vec![]);
        marked_player.require_mark.push(MarkPattern {
            object: PLAYER,
            mark: MARK,
            value: None,
            match_value: MarkValueMatch::Exact,
        });
        let consume_mark = Rule {
            id: RuleId(41),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![marked_player]),
            writes: vec![replace(0, 0, PLAYER, BOX)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new_with_mark_condition_defs_and_program(
            2,
            objects,
            marks,
            Vec::new(),
            vec![RuleStep::Rule(set_mark), RuleStep::Rule(consume_mark)],
        );
        let mut initial = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        let trace = transition_trace(&game, &initial, RIGHT).unwrap();

        let cursors = replay_rule_firing_states(&game, &initial, &trace.firings).unwrap();

        assert_eq!(cursors.len(), 2);
        assert!(cursors[0].has_object(&game, 0, 0, PLAYER));
        assert!(cursors[1].has_object(&game, 0, 0, BOX));
        assert_eq!(cursors[1], trace.next_state);
    }

    #[test]
    fn firing_cursor_replay_captures_each_variable_update_state() {
        let game = CompiledGame::new_with_program(
            1,
            Vec::new(),
            vec![
                RuleStep::Rule(variable_rule(
                    50,
                    Vec::new(),
                    vec![set_variable(0, 2)],
                    RuleApplication::Once,
                )),
                RuleStep::Rule(variable_rule(
                    51,
                    Vec::new(),
                    vec![add_variable(0, 3)],
                    RuleApplication::Once,
                )),
            ],
        );
        let initial =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();
        let trace = transition_trace(&game, &initial, RIGHT).unwrap();

        let cursors = replay_rule_firing_states(&game, &initial, &trace.firings).unwrap();

        assert_eq!(cursors.len(), 2);
        assert_eq!(cursors[0].visible_variables(), &[2]);
        assert_eq!(cursors[1].visible_variables(), &[5]);
        assert_eq!(cursors[1], trace.next_state);
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

        assert_eq!(fired_rules(&trace), vec![RuleId(1)]);
        assert_eq!(trace.firings.len(), 1);
        assert_eq!(trace.firings[0].patch.ops().len(), 4);
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
        assert_eq!(fired_rules(&trace), vec![RuleId(3)]);
        assert_eq!(trace.firings.len(), 1);
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
        assert_eq!(fired_rules(&trace), vec![RuleId(1), RuleId(1), RuleId(1)]);
        assert_eq!(trace.firings.len(), 3);
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
        assert_eq!(fired_rules(&trace), vec![RuleId(1), RuleId(1)]);
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
        assert_eq!(fired_rules(&trace), vec![RuleId(8)]);
        assert_eq!(trace.firings.len(), 1);
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
        assert_eq!(fired_rules(&trace), vec![RuleId(10)]);
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
    fn once_per_level_keeps_first_match_semantics_for_idempotent_patch() {
        const DECORATION: ObjectId = ObjectId(4);
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: DECORATION,
                layer_id: LayerId(2),
            },
        ];
        let decorate_once = Rule {
            id: RuleId(28),
            guards: Vec::new(),
            application: RuleApplication::OncePerLevel,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, DECORATION)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new(3, objects, vec![decorate_once]);
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        for x in 0..3 {
            state.place_object(&game, x, 0, PLAYER).unwrap();
        }
        state.place_object(&game, 0, 0, DECORATION).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert!(next.level_rule_has_fired(RuleId(28)));
        assert!(!next.has_object(&game, 1, 0, DECORATION));
    }

    #[test]
    fn until_stable_block_skips_when_state_cycles() {
        let zero_to_one = Rule {
            id: RuleId(7),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 0,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            }],
        };
        let one_to_two = Rule {
            id: RuleId(8),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 1,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 2,
            }],
        };
        let two_to_zero = Rule {
            id: RuleId(9),
            guards: vec![Guard::VariableEquals {
                variable: VariableId(0),
                value: 2,
            }],
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
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
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_block_keeps_revisited_non_initial_state() {
        let value = VariableId(0);
        let changed = VariableId(1);
        let reset_changed = variable_rule(
            20,
            Vec::new(),
            vec![set_variable(1, 0)],
            RuleApplication::Once,
        );
        let two_to_one = variable_rule(
            21,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 2,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 1), set_variable(1, 1)],
            RuleApplication::Once,
        );
        let one_to_two = variable_rule(
            22,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 1,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 2), set_variable(1, 1)],
            RuleApplication::Once,
        );
        let zero_to_one = variable_rule(
            23,
            vec![
                Guard::VariableEquals {
                    variable: value,
                    value: 0,
                },
                Guard::VariableEquals {
                    variable: changed,
                    value: 0,
                },
            ],
            vec![set_variable(0, 1), set_variable(1, 1)],
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
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0, 0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.variable_value(value), Some(1));
        assert_eq!(next.variable_value(changed), Some(1));
    }

    #[test]
    fn until_stable_block_budget_keeps_last_state_for_divergent_updates() {
        let counter = VariableId(0);
        let increment = variable_rule(
            24,
            Vec::new(),
            vec![add_variable(0, 1)],
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
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(
            next.variable_value(counter),
            Some(crate::grid_transition::UNTIL_STABLE_REPEAT_LIMIT as i64)
        );
    }

    #[test]
    fn until_stable_block_skips_idempotent_first_match_and_reaches_later_matches() {
        const DECORATION: ObjectId = ObjectId(4);
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: DECORATION,
                layer_id: LayerId(2),
            },
        ];
        let decorate_one = Rule {
            id: RuleId(25),
            guards: Vec::new(),
            application: RuleApplication::RepeatStep,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, DECORATION)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new_with_program(
            3,
            objects,
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![RuleStep::Rule(decorate_one)],
            }],
        );
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        for x in 0..3 {
            state.place_object(&game, x, 0, PLAYER).unwrap();
        }

        let state_only = transition_state(&game, &state, RIGHT).unwrap();
        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(state_only, trace.next_state);
        for x in 0..3 {
            assert!(trace.next_state.has_object(&game, x, 0, DECORATION));
        }
        assert_eq!(fired_rules(&trace), vec![RuleId(25); 4]);
        assert!(trace.progressed);
        assert!(!trace.observable);
        assert_eq!(
            trace
                .firings
                .iter()
                .map(|firing| firing.progressed)
                .collect::<Vec<_>>(),
            vec![true, true, true, false]
        );
        assert!(trace.firings.iter().all(|firing| !firing.observable));
    }

    #[test]
    fn once_keeps_first_match_semantics_when_first_patch_is_idempotent() {
        const DECORATION: ObjectId = ObjectId(4);
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: DECORATION,
                layer_id: LayerId(2),
            },
        ];
        let decorate_once = Rule {
            id: RuleId(27),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, DECORATION)],
            effects: Vec::new(),
        };
        let game = CompiledGame::new(3, objects, vec![decorate_once]);
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        for x in 0..3 {
            state.place_object(&game, x, 0, PLAYER).unwrap();
        }
        state.place_object(&game, 0, 0, DECORATION).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.next_state, state);
        assert_eq!(fired_rules(&trace), vec![RuleId(27)]);
        assert!(!trace.progressed);
        assert!(!trace.observable);
        assert!(!trace.firings[0].progressed);
        assert!(!trace.firings[0].observable);
        assert!(!trace.next_state.has_object(&game, 1, 0, DECORATION));
    }

    #[test]
    fn observable_effect_only_match_fires_without_state_progress() {
        let objects = vec![ObjectDef {
            id: PLAYER,
            layer_id: LayerId(1),
        }];
        let effect_only = Rule {
            id: RuleId(26),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: Vec::new(),
            effects: vec![Effect::ObserveMatch],
        };
        let game = CompiledGame::new(2, objects, vec![effect_only]);
        let mut state = State::empty(1, 1, game.layer_count, game.object_count()).unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.next_state, state);
        assert_eq!(fired_rules(&trace), vec![RuleId(26)]);
        assert!(!trace.progressed);
        assert!(trace.observable);
        assert!(!trace.firings[0].progressed);
        assert!(trace.firings[0].observable);
    }

    #[test]
    fn observable_repeat_step_preserves_first_match_priority() {
        const DECORATION: ObjectId = ObjectId(4);
        let objects = vec![
            ObjectDef {
                id: PLAYER,
                layer_id: LayerId(1),
            },
            ObjectDef {
                id: DECORATION,
                layer_id: LayerId(2),
            },
        ];
        let observable_decorate = Rule {
            id: RuleId(30),
            guards: Vec::new(),
            application: RuleApplication::RepeatStep,
            pattern: pattern(vec![cell(0, 0, vec![PLAYER], vec![])]),
            writes: vec![add(0, 0, DECORATION)],
            effects: vec![Effect::ObserveMatch],
        };
        let game = CompiledGame::new_with_program(
            3,
            objects,
            vec![RuleStep::Block {
                application: RuleApplication::UntilStable,
                stop_condition: None,
                steps: vec![RuleStep::Rule(observable_decorate)],
            }],
        );
        let mut state = State::empty(3, 1, game.layer_count, game.object_count()).unwrap();
        for x in 0..3 {
            state.place_object(&game, x, 0, PLAYER).unwrap();
        }
        state.place_object(&game, 0, 0, DECORATION).unwrap();

        let trace = transition_trace(&game, &state, RIGHT).unwrap();

        assert_eq!(trace.next_state, state);
        assert_eq!(fired_rules(&trace), vec![RuleId(30)]);
        assert!(!trace.next_state.has_object(&game, 1, 0, DECORATION));
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
                        effects: vec![Effect::UpdateVariable {
                            variable: VariableId(0),
                            op: VariableUpdateOp::Set,
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
                        effects: vec![Effect::UpdateVariable {
                            variable: VariableId(0),
                            op: VariableUpdateOp::Set,
                            value: 0,
                        }],
                    }),
                ],
            }],
        );
        let state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn until_stable_rule_treats_idempotent_variable_update_as_stable() {
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
            effects: vec![Effect::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 1,
            }],
        };
        let game = CompiledGame::new_with_program(2, objects, vec![RuleStep::Rule(rule)]);
        let mut state =
            State::empty_with_variables(1, 1, game.layer_count, game.object_count(), vec![0])
                .unwrap();
        state.place_object(&game, 0, 0, PLAYER).unwrap();

        let next = transition_state(&game, &state, RIGHT).unwrap();

        assert_eq!(next.variable_value(VariableId(0)), Some(1));
        assert!(next.has_object(&game, 0, 0, PLAYER));
    }
}
