#[cfg(test)]
mod tests {
    use crate::*;
    use puzzle_kernel::VariableUpdateOp;

    type Rule = puzzle_core::GridRule<3>;

    #[test]
    fn null_cell_uses_the_shared_out_of_bounds_match_contract() {
        let player = ObjectId(1);
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(
            1,
            vec![ObjectDef3 {
                id: player,
                layer_id: LayerId(0),
            }],
        )
        .unwrap();
        let mut state = State3::empty(Size3::new(1, 1, 1), 1).unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), player)
            .unwrap();

        let mut outside = GridMatchCell::<3>::new(Delta3::new(0, -1, 0));
        outside.require_null = true;
        let boundary = GridPattern::<3>::new(vec![
            GridMatchCell::<3>::new(Delta3::ZERO).require(player),
            outside,
        ]);
        assert!(has_pattern_match(&game, &state, &boundary));

        let mut inside = GridMatchCell::<3>::new(Delta3::ZERO);
        inside.require_null = true;
        assert!(!has_pattern_match(
            &game,
            &state,
            &GridPattern::<3>::new(vec![inside])
        ));
    }

    #[test]
    fn transition_accepts_shared_random_application() {
        let player = ObjectId(1);
        let crate_object = ObjectId(2);
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(
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
        .unwrap();
        let mut state = State3::empty(Size3::new(2, 1, 1), 1).unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), player)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), player)
            .unwrap();
        let mut rule = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(player)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: player,
                add: crate_object,
            }],
        )
        .with_id(RuleId3(7));
        rule.application = RuleApplication3::Random;

        let next = transition_program(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule.clone())],
            crate::InputId(0),
        )
        .unwrap();
        let repeated = transition_program(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule)],
            crate::InputId(0),
        )
        .unwrap();
        assert_eq!(next, repeated);
        assert_eq!(next.object_count(crate_object), 1);
        assert_eq!(next.object_count(player), 1);
    }

    #[test]
    fn transition_accepts_shared_variable_guard() {
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(1, Vec::new()).unwrap();
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![2]).unwrap();
        let mut rule =
            Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new()).with_effects(vec![
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

        let next =
            transition_program_without_input(&game, &state, &[GridRuleStep::<3>::Rule(rule)])
                .unwrap();
        assert_eq!(next.variable_value(VariableId(0)), Some(7));
    }

    #[test]
    fn transition_accepts_shared_named_condition_guard() {
        let condition = ConditionId3(0);
        let game = puzzle_core::GridCompiledGame::<3>::new_with_condition_defs(
            1,
            Vec::new(),
            vec![ConditionDef3 {
                id: condition,
                kind: ConditionValueKind3::NoneObjects(vec![ObjectId(1)]),
            }],
        );
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![0]).unwrap();
        let mut rule =
            Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new()).with_effects(vec![
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

        let next =
            transition_program_without_input(&game, &state, &[GridRuleStep::<3>::Rule(rule)])
                .unwrap();
        assert_eq!(next.variable_value(VariableId(0)), Some(3));
    }

    #[test]
    fn cancel_reverts_the_whole_3d_program_like_2d() {
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(1, Vec::new()).unwrap();
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![0]).unwrap();
        let update = Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new()).with_effects(vec![
            RuleEffect3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Set,
                value: 7,
            },
        ]);
        let cancel = Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new())
            .with_effects(vec![RuleEffect3::Cancel]);

        let outcome = transition_program_without_input_outcome(
            &game,
            &state,
            &[
                GridRuleStep::<3>::Rule(update),
                GridRuleStep::<3>::Rule(cancel),
            ],
        )
        .unwrap();

        assert!(outcome.cancelled);
        assert_eq!(outcome.next_state.variable_value(VariableId(0)), Some(0));
        assert!(outcome.commands.is_empty());
    }

    #[test]
    fn shared_program_continuation_resumes_a_3d_program() {
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(1, Vec::new()).unwrap();
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![0, 0]).unwrap();
        let set_variable = |variable, value| {
            Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new()).with_effects(vec![
                RuleEffect3::UpdateVariable {
                    variable: VariableId(variable),
                    op: VariableUpdateOp::Set,
                    value,
                },
            ])
        };
        let program = vec![
            GridRuleStep::<3>::Rule(set_variable(0, 1)),
            GridRuleStep::<3>::Rule(set_variable(1, 2)),
        ];

        let first = puzzle_core::grid_transition::transition_program_segment_trace(
            &game,
            &program,
            &state,
            None,
            None,
            |boundary| boundary.fired_rules.len() == 1,
        )
        .unwrap();
        assert_eq!(
            first.trace.next_state.variable_value(VariableId(0)),
            Some(1)
        );
        assert_eq!(
            first.trace.next_state.variable_value(VariableId(1)),
            Some(0)
        );

        let continuation = first.remaining_program.expect("program must pause");
        let resumed = puzzle_core::grid_transition::transition_program_continuation_segment_trace(
            &game,
            &program,
            &continuation,
            &first.trace.next_state,
            None,
            None,
            |_| false,
        )
        .unwrap();
        assert!(resumed.remaining_program.is_none());
        assert_eq!(
            resumed.trace.next_state.variable_value(VariableId(0)),
            Some(1)
        );
        assert_eq!(
            resumed.trace.next_state.variable_value(VariableId(1)),
            Some(2)
        );
    }

    #[test]
    fn segmented_3d_until_stable_matches_uninterrupted_execution() {
        let game = puzzle_core::GridCompiledGame::<3>::checked_new(1, Vec::new()).unwrap();
        let state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![0, 0]).unwrap();
        let set_variable = |variable, value| {
            Rule::once(GridPattern::<3>::new(Vec::new()), Vec::new()).with_effects(vec![
                RuleEffect3::UpdateVariable {
                    variable: VariableId(variable),
                    op: VariableUpdateOp::Set,
                    value,
                },
            ])
        };
        let program = vec![GridRuleStep::<3>::Block {
            application: RuleApplication3::UntilStable,
            stop_condition: None,
            steps: vec![
                GridRuleStep::<3>::Rule(set_variable(0, 1)),
                GridRuleStep::<3>::Rule(set_variable(1, 2)),
            ],
        }];
        let uninterrupted = transition_program_without_input(&game, &state, &program).unwrap();

        let mut segment = puzzle_core::grid_transition::transition_program_segment_trace(
            &game,
            &program,
            &state,
            None,
            None,
            |boundary| boundary.fired_rules.len() == 1,
        )
        .unwrap();
        let mut segment_count = 1;
        while let Some(continuation) = segment.remaining_program.take() {
            segment = puzzle_core::grid_transition::transition_program_continuation_segment_trace(
                &game,
                &program,
                &continuation,
                &segment.trace.next_state,
                None,
                None,
                |boundary| boundary.fired_rules.len() == 1,
            )
            .unwrap();
            segment_count += 1;
            assert!(
                segment_count < 10,
                "until-stable continuation did not finish"
            );
        }

        assert!(segment_count > 2, "test must pause inside repeated passes");
        assert_eq!(segment.trace.next_state, uninterrupted);
    }
}
