mod ids;
mod level;
mod model;
mod patch;
mod state;
mod win;

#[cfg(test)]
mod transition_tests;

pub use ids::{ConditionId3, InputId, LayerId, MarkId3, ObjectId, RuleId3, VariableId};
pub use level::{Level3, LevelBundle3, LevelBundleError3, LevelCell3, LevelEntry3, LevelError3};
pub use model::{
    Axis3, CompiledGame3, CompiledGameError3, Coord3, Delta3, Direction3, DirectionSet3, Frame3,
    FrameChirality3, FrameError3, FrameExpr3, FrameSet3, FrameSlot3, GapTerm3, InputDef3, MarkDef3,
    ObjectDef3, Offset3, Size3,
};
pub use patch::{Patch3, PatchError3, PatchOp3};
pub use puzzle_core::grid_transition::{
    count_pattern_matches, eval_condition_kind, flattened_rules, has_pattern_match,
    transition_once, transition_once_all, transition_once_per_level, transition_once_with_input,
    transition_program, transition_program_outcome, transition_program_outcome_with_local_frame,
    transition_program_with_local_frame, transition_program_without_input,
    transition_program_without_input_outcome,
    transition_program_without_input_outcome_with_local_frame,
    transition_program_without_input_with_local_frame, transition_repeated,
};
pub use puzzle_core::{
    GridMatchCell, GridPattern, GridRule, GridRuleCondition, GridRuleStep, GridWriteOp,
};
pub type ConditionDef3 = puzzle_core::GridConditionDef<3>;
pub type ConditionValueKind3 = puzzle_core::GridConditionValueKind<3>;
pub type Guard3 = puzzle_core::GridGuard<3>;
pub type MarkPattern3 = puzzle_core::MarkPattern;
pub type ObjectSetMarkPattern3 = puzzle_core::ObjectSetMarkPattern;
pub type ObjectSetMatcher3 = puzzle_core::ObjectSetMatcher;
pub type PatternComponent3 = puzzle_core::GridPatternComponent<3>;
pub type RuleApplication3 = puzzle_core::RuleApplication;
pub type RuleEffect3 = puzzle_core::Effect;
pub type TransitionCommand3 = puzzle_core::TransitionCommand;
pub type TransitionError3 = puzzle_core::GridTransitionError<3>;
pub type TransitionOutcome3 = puzzle_core::grid_transition::GridTransitionOutcome<3, Size3>;
pub use puzzle_kernel::{LocalFrame, LocalFrameExtent, MarkKind, MarkValueMatch, VariableUpdateOp};
pub use state::{CellView3, SlotMark3, State3, StateError3};
pub use win::WinCondition3;

#[cfg(test)]
mod tests {
    use super::*;

    type Rule = puzzle_core::GridRule<3>;

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const GOAL: ObjectId = ObjectId(4);
    const ACTOR: LayerId = LayerId(0);
    const FLOOR: LayerId = LayerId(1);
    const INPUT_LEFT: InputId = InputId(0);
    const INPUT_RIGHT: InputId = InputId(1);

    fn game() -> CompiledGame3 {
        CompiledGame3::new(
            1,
            vec![
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: BOX,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: WALL,
                    layer_id: ACTOR,
                },
            ],
            Vec::new(),
        )
    }

    fn empty_state(width: u16, depth: u16, height: u16) -> State3 {
        State3::empty(Size3::new(width, depth, height), 1).unwrap()
    }

    fn layered_game() -> CompiledGame3 {
        CompiledGame3::new(
            2,
            vec![
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: BOX,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: GOAL,
                    layer_id: FLOOR,
                },
            ],
            Vec::new(),
        )
    }

    fn push_rule(direction: Direction3) -> Rule {
        let step = direction.offset;
        let two_steps = step.scale(2);
        Rule::once(
            GridPattern::<3>::new(vec![
                GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
                GridMatchCell::<3>::new(step).require(BOX),
                GridMatchCell::<3>::new(two_steps)
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![
                GridWriteOp::<3>::Move {
                    component: 0,
                    from_offset: step.into(),
                    to_offset: two_steps.into(),
                    object: BOX,
                },
                GridWriteOp::<3>::Move {
                    component: 0,
                    from_offset: Delta3::ZERO.into(),
                    to_offset: step.into(),
                    object: PLAYER,
                },
            ],
        )
    }

    fn move_rule(direction: Direction3) -> Rule {
        Rule::once(
            GridPattern::<3>::new(vec![
                GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
                GridMatchCell::<3>::new(direction.offset)
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![GridWriteOp::<3>::Move {
                component: 0,
                from_offset: Delta3::ZERO.into(),
                to_offset: direction.offset.into(),
                object: PLAYER,
            }],
        )
    }

    fn once_all_move_rule(direction: Direction3) -> Rule {
        let rule = move_rule(direction);
        Rule::once_all(rule.pattern, rule.writes)
    }

    #[test]
    fn transition_program_outcome_reports_fired_rules_and_patches() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let rule = move_rule(Direction3::RIGHT);

        let outcome = transition_program_outcome(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule)],
            INPUT_RIGHT,
        )
        .unwrap();

        assert_eq!(outcome.input, Some(INPUT_RIGHT));
        assert_eq!(outcome.fired_rules, vec![RuleId3(0)]);
        assert_eq!(outcome.commands, Vec::<TransitionCommand3>::new());
        assert_eq!(outcome.patches.len(), 1);
        assert_eq!(
            outcome.patches[0].ops(),
            vec![PatchOp3::Move {
                from: Coord3::new(0, 0, 0).into(),
                to: Coord3::new(1, 0, 0).into(),
                object: PLAYER,
            }]
            .as_slice()
        );
        assert_eq!(
            outcome
                .next_state
                .cell_view_at(Coord3::new(1, 0, 0))
                .unwrap()
                .objects,
            vec![PLAYER]
        );
    }

    #[test]
    fn direction_sets_use_absolute_3d_grid_axes() {
        let directions = Direction3::directions();
        assert_eq!(
            directions.map(|direction| direction.name),
            ["up", "down", "left", "right", "front", "back"]
        );
        assert_eq!(Direction3::UP.offset, Delta3::new(0, 0, 1));
        assert_eq!(Direction3::DOWN.offset, Delta3::new(0, 0, -1));
        assert_eq!(Direction3::LEFT.offset, Delta3::new(-1, 0, 0));
        assert_eq!(Direction3::RIGHT.offset, Delta3::new(1, 0, 0));
        assert_eq!(Direction3::FORWARD.offset, Delta3::new(0, 1, 0));
        assert_eq!(Direction3::BACKWARD.offset, Delta3::new(0, -1, 0));

        assert_eq!(
            Direction3::horizontal().map(|direction| direction.name),
            ["left", "right", "front", "back"]
        );
        assert_eq!(
            Direction3::vertical().map(|direction| direction.name),
            ["up", "down"]
        );
        assert!(Direction3::FORWARD.is_horizontal());
        assert!(!Direction3::UP.is_horizontal());
        assert!(Direction3::UP.is_vertical());
    }

    #[test]
    fn pattern3_new_creates_single_core_shaped_component() {
        let pattern = GridPattern::<3>::new(vec![
            GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
            GridMatchCell::<3>::new(Direction3::RIGHT.offset).require(BOX),
        ]);

        assert_eq!(pattern.components.len(), 1);
        assert_eq!(pattern.components[0].gap_count, 0);
        assert_eq!(pattern.components[0].cells.len(), pattern.cells().len());
    }

    #[test]
    fn pattern3_components_keep_compatibility_cells_view() {
        let pattern = GridPattern::<3>::from_components(vec![
            PatternComponent3::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            PatternComponent3::new(vec![
                GridMatchCell::<3>::new(Direction3::RIGHT.offset).require(BOX),
            ]),
        ]);

        assert_eq!(pattern.components.len(), 2);
        assert_eq!(pattern.cells().len(), 2);
        assert_eq!(pattern.cells()[0].require_objects, vec![PLAYER]);
        assert_eq!(pattern.cells()[1].require_objects, vec![BOX]);
    }

    #[test]
    fn direction_and_frame_are_distinct_orientation_types() {
        assert_eq!(Direction3::RIGHT.axis(), Axis3::X);
        assert_eq!(Direction3::FORWARD.axis(), Axis3::Y);
        assert_eq!(Direction3::UP.axis(), Axis3::Z);
        assert_eq!(Direction3::RIGHT.opposite(), Direction3::LEFT);

        let frame = Frame3::canonical(Direction3::RIGHT, Direction3::BACKWARD).unwrap();

        assert_eq!(frame.primary, Direction3::RIGHT);
        assert_eq!(frame.secondary, Direction3::BACKWARD);
        assert_eq!(frame.depth, Direction3::DOWN);
    }

    #[test]
    fn frame_shorthand_uses_canonical_chirality() {
        assert_eq!(
            Frame3::canonical(Direction3::RIGHT, Direction3::BACKWARD).unwrap(),
            Frame3::explicit(Direction3::RIGHT, Direction3::BACKWARD, Direction3::DOWN).unwrap()
        );
        assert_eq!(
            Frame3::canonical(Direction3::LEFT, Direction3::UP)
                .unwrap()
                .depth,
            Direction3::FORWARD
        );
        assert_eq!(
            Frame3::canonical(Direction3::FORWARD, Direction3::UP)
                .unwrap()
                .depth,
            Direction3::RIGHT
        );
        assert_eq!(
            Frame3::canonical(Direction3::BACKWARD, Direction3::UP)
                .unwrap()
                .depth,
            Direction3::LEFT
        );
    }

    #[test]
    fn explicit_frame_can_represent_anti_chiral_orientation() {
        let anti =
            Frame3::explicit(Direction3::RIGHT, Direction3::UP, Direction3::FORWARD).unwrap();

        assert!(!anti.is_canonical_chiral());
        assert_eq!(
            anti.to_world_offset(Delta3::new(1, 1, 1)),
            Delta3::new(1, 1, 1)
        );
    }

    #[test]
    fn frame_rejects_repeated_axes() {
        assert_eq!(
            Frame3::canonical(Direction3::RIGHT, Direction3::LEFT).unwrap_err(),
            FrameError3::RepeatedAxis {
                first: Direction3::RIGHT,
                second: Direction3::LEFT,
            }
        );
        assert_eq!(
            Frame3::explicit(Direction3::RIGHT, Direction3::UP, Direction3::DOWN).unwrap_err(),
            FrameError3::RepeatedAxis {
                first: Direction3::UP,
                second: Direction3::DOWN,
            }
        );
    }

    #[test]
    fn horizontal_frames_expand_direction_set_with_fixed_secondary_axis() {
        let frames = Frame3::horizontal(Direction3::UP).unwrap();

        assert_eq!(
            frames
                .iter()
                .map(|frame| (frame.primary.name, frame.secondary.name, frame.depth.name))
                .collect::<Vec<_>>(),
            vec![
                ("left", "up", "front"),
                ("right", "up", "back"),
                ("front", "up", "right"),
                ("back", "up", "left"),
            ]
        );
    }

    #[test]
    fn frame_transforms_local_dense_pattern_offsets_to_world_offsets() {
        let frame = Frame3::canonical(Direction3::FORWARD, Direction3::UP).unwrap();

        assert_eq!(
            frame.to_world_offset(Delta3::new(1, 0, 0)),
            Direction3::FORWARD.offset
        );
        assert_eq!(
            frame.to_world_offset(Delta3::new(0, 1, 0)),
            Direction3::UP.offset
        );
        assert_eq!(
            frame.to_world_offset(Delta3::new(0, 0, 1)),
            Direction3::RIGHT.offset
        );
        assert_eq!(
            frame.to_world_offset(Delta3::new(2, 1, 3)),
            Delta3::new(3, 2, 1)
        );
    }

    #[test]
    fn primitive_frame_sets_partition_all_valid_frames() {
        let frames = Frame3::frames();
        let canonical = Frame3::canonical_frames();
        let mirrored = Frame3::mirrored_frames();

        assert_eq!(frames.len(), 48);
        assert_eq!(canonical.len(), 24);
        assert_eq!(mirrored.len(), 24);
        assert!(canonical.iter().all(|frame| frame.is_canonical_chiral()));
        assert!(mirrored.iter().all(|frame| !frame.is_canonical_chiral()));
        assert!(canonical.iter().all(|frame| frames.contains(frame)));
        assert!(mirrored.iter().all(|frame| frames.contains(frame)));
        assert!(canonical.iter().all(|frame| !mirrored.contains(frame)));
    }

    #[test]
    fn frame_expression_with_full_direction_sets_keeps_both_chiralities() {
        let expr = FrameExpr3::new(
            FrameSlot3::DirectionSet(DirectionSet3::Horizontal),
            FrameSlot3::DirectionSet(DirectionSet3::Horizontal),
            FrameSlot3::Direction(Direction3::UP),
        );
        let frames = expr.expand();

        assert_eq!(frames.len(), 8);
        assert!(frames.contains(
            &Frame3::explicit(Direction3::RIGHT, Direction3::FORWARD, Direction3::UP).unwrap()
        ));
        assert!(frames.contains(
            &Frame3::explicit(Direction3::RIGHT, Direction3::BACKWARD, Direction3::UP).unwrap()
        ));
        assert_eq!(
            frames
                .iter()
                .filter(|frame| frame.is_canonical_chiral())
                .count(),
            4
        );
    }

    #[test]
    fn frame_expression_completion_fixes_canonical_chirality() {
        let expr = FrameExpr3::new(
            FrameSlot3::DirectionSet(DirectionSet3::Horizontal),
            FrameSlot3::CompleteCanonical,
            FrameSlot3::Direction(Direction3::UP),
        );
        let frames = expr.expand();

        assert_eq!(frames.len(), 4);
        assert!(frames.iter().all(|frame| frame.is_canonical_chiral()));
        assert_eq!(
            frames
                .iter()
                .map(|frame| (frame.primary.name, frame.secondary.name, frame.depth.name))
                .collect::<Vec<_>>(),
            vec![
                ("left", "back", "up"),
                ("right", "front", "up"),
                ("front", "left", "up"),
                ("back", "right", "up"),
            ]
        );
    }

    #[test]
    fn two_slot_frame_expression_completes_the_depth_axis() {
        let expr = FrameExpr3::from_two(
            FrameSlot3::Direction(Direction3::RIGHT),
            FrameSlot3::Direction(Direction3::UP),
        );

        assert_eq!(
            expr.expand(),
            vec![
                Frame3::explicit(Direction3::RIGHT, Direction3::UP, Direction3::BACKWARD).unwrap()
            ]
        );
    }

    #[test]
    fn completion_can_fill_any_single_frame_slot() {
        let missing_primary = FrameExpr3::new(
            FrameSlot3::CompleteCanonical,
            FrameSlot3::Direction(Direction3::BACKWARD),
            FrameSlot3::Direction(Direction3::DOWN),
        );
        let missing_secondary = FrameExpr3::new(
            FrameSlot3::Direction(Direction3::RIGHT),
            FrameSlot3::CompleteCanonical,
            FrameSlot3::Direction(Direction3::DOWN),
        );
        let missing_depth = FrameExpr3::new(
            FrameSlot3::Direction(Direction3::RIGHT),
            FrameSlot3::Direction(Direction3::BACKWARD),
            FrameSlot3::CompleteCanonical,
        );

        let expected = vec![
            Frame3::explicit(Direction3::RIGHT, Direction3::BACKWARD, Direction3::DOWN).unwrap(),
        ];
        assert_eq!(missing_primary.expand(), expected);
        assert_eq!(missing_secondary.expand(), expected);
        assert_eq!(missing_depth.expand(), expected);
    }

    #[test]
    fn frame_set_expression_can_filter_by_canonical_or_mirrored_chirality() {
        static HORIZONTAL_UP: FrameExpr3 = FrameExpr3::new(
            FrameSlot3::DirectionSet(DirectionSet3::Horizontal),
            FrameSlot3::DirectionSet(DirectionSet3::Horizontal),
            FrameSlot3::Direction(Direction3::UP),
        );

        let canonical = FrameSet3::ExprChirality {
            expr: &HORIZONTAL_UP,
            chirality: FrameChirality3::Canonical,
        }
        .frames();
        let mirrored = FrameSet3::ExprChirality {
            expr: &HORIZONTAL_UP,
            chirality: FrameChirality3::Mirrored,
        }
        .frames();

        assert_eq!(canonical.len(), 4);
        assert_eq!(mirrored.len(), 4);
        assert!(canonical.iter().all(|frame| frame.is_canonical_chiral()));
        assert!(mirrored.iter().all(|frame| !frame.is_canonical_chiral()));
    }

    #[test]
    fn game_validation_accepts_well_formed_definitions() {
        let game = CompiledGame3::checked_new(
            2,
            vec![
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: GOAL,
                    layer_id: FLOOR,
                },
            ],
        )
        .unwrap();

        assert_eq!(game.layer_count, 2);
        assert_eq!(game.objects().len(), 2);
    }

    #[test]
    fn game_validation_rejects_zero_layers() {
        let err = CompiledGame3::checked_new(0, Vec::new()).unwrap_err();

        assert_eq!(err, CompiledGameError3::InvalidLayerCount);
    }

    #[test]
    fn game_validation_rejects_empty_object_id() {
        let err = CompiledGame3::checked_new(
            1,
            vec![ObjectDef3 {
                id: ObjectId::EMPTY,
                layer_id: ACTOR,
            }],
        )
        .unwrap_err();

        assert_eq!(err, CompiledGameError3::EmptyObjectId);
    }

    #[test]
    fn game_validation_rejects_duplicate_object_ids() {
        let err = CompiledGame3::checked_new(
            1,
            vec![
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: PLAYER,
                    layer_id: ACTOR,
                },
            ],
        )
        .unwrap_err();

        assert_eq!(
            err,
            CompiledGameError3::DuplicateObjectId { object: PLAYER }
        );
    }

    #[test]
    fn game_validation_rejects_object_layer_outside_layer_count() {
        let err = CompiledGame3::checked_new(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: FLOOR,
            }],
        )
        .unwrap_err();

        assert_eq!(
            err,
            CompiledGameError3::ObjectLayerOutOfBounds {
                object: PLAYER,
                layer: FLOOR,
            }
        );
    }

    #[test]
    fn state_uses_z_y_x_slot_order_with_layers_inside_cells() {
        let game = game();
        let mut state = empty_state(2, 2, 2);
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 1, 1), BOX)
            .unwrap();

        assert_eq!(state.slots()[1], PLAYER);
        assert_eq!(state.slots()[6], BOX);
    }

    #[test]
    fn level_builds_state_from_discrete_cells() {
        let game = game();
        let level = Level3::new(
            Size3::new(3, 2, 2),
            vec![
                LevelCell3::new(Coord3::new(1, 0, 0), vec![PLAYER]),
                LevelCell3::new(Coord3::new(2, 1, 1), vec![BOX]),
            ],
        );

        let state = level.build_state(&game).unwrap();

        assert!(state.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(state.has_object_at(&game, Coord3::new(2, 1, 1), BOX));
    }

    #[test]
    fn level_bundle_owns_game_and_named_levels() {
        let game = game();
        let first = Level3::new(
            Size3::new(3, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER])],
        );
        let second = Level3::new(
            Size3::new(3, 1, 1),
            vec![LevelCell3::new(Coord3::new(1, 0, 0), vec![BOX])],
        );
        let bundle = LevelBundle3::checked_new(
            game.clone(),
            vec![
                LevelEntry3::new("microban_01", first, Vec::new()),
                LevelEntry3::new("microban_02", second, Vec::new()),
            ],
        )
        .unwrap();

        assert_eq!(bundle.level_count(), 2);
        assert_eq!(bundle.level_by_name("microban_02").unwrap().0, 1);

        let state = bundle.build_level_state(1).unwrap();

        assert!(state.has_object_at(&game, Coord3::new(1, 0, 0), BOX));
    }

    #[test]
    fn level_bundle_rejects_empty_level_list() {
        let err = LevelBundle3::checked_new(game(), Vec::new()).unwrap_err();

        assert_eq!(err, LevelBundleError3::EmptyLevels);
    }

    #[test]
    fn level_bundle_rejects_empty_level_name() {
        let level = Level3::new(Size3::new(1, 1, 1), Vec::new());
        let err = LevelBundle3::checked_new(game(), vec![LevelEntry3::new("", level, Vec::new())])
            .unwrap_err();

        assert_eq!(err, LevelBundleError3::EmptyLevelName { index: 0 });
    }

    #[test]
    fn level_bundle_rejects_duplicate_level_names() {
        let level = Level3::new(Size3::new(1, 1, 1), Vec::new());
        let err = LevelBundle3::checked_new(
            game(),
            vec![
                LevelEntry3::new("microban_01", level.clone(), Vec::new()),
                LevelEntry3::new("microban_01", level, Vec::new()),
            ],
        )
        .unwrap_err();

        assert_eq!(
            err,
            LevelBundleError3::DuplicateLevelName {
                name: "microban_01".to_string(),
            }
        );
    }

    #[test]
    fn level_bundle_wraps_level_build_errors_with_level_identity() {
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(2, 0, 0), vec![PLAYER])],
        );
        let err =
            LevelBundle3::checked_new(game(), vec![LevelEntry3::new("bad", level, Vec::new())])
                .unwrap_err();

        assert_eq!(
            err,
            LevelBundleError3::Level {
                index: 0,
                name: "bad".to_string(),
                source: LevelError3::State(StateError3::PositionOutOfBounds {
                    position: Coord3::new(2, 0, 0).into(),
                }),
            }
        );
    }

    #[test]
    fn level_bundle_rejects_missing_level_index_when_building_state() {
        let bundle = LevelBundle3::checked_new(
            game(),
            vec![LevelEntry3::new(
                "microban_01",
                Level3::new(Size3::new(1, 1, 1), Vec::new()),
                Vec::new(),
            )],
        )
        .unwrap();

        let err = bundle.build_level_state(1).unwrap_err();

        assert_eq!(
            err,
            LevelBundleError3::LevelIndexOutOfBounds {
                index: 1,
                level_count: 1,
            }
        );
    }

    #[test]
    fn level_allows_split_cell_entries_when_layers_do_not_collide() {
        let game = layered_game();
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![
                LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER]),
                LevelCell3::new(Coord3::new(0, 0, 0), vec![GOAL]),
            ],
        );

        let state = level.build_state(&game).unwrap();

        assert!(state.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(state.has_object_at(&game, Coord3::new(0, 0, 0), GOAL));
    }

    #[test]
    fn level_rejects_same_layer_collision() {
        let game = game();
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER, BOX])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::State(StateError3::LayerOccupied {
                position: Coord3::new(0, 0, 0).into(),
                layer: ACTOR,
                existing: PLAYER,
                attempted: BOX,
            })
        );
    }

    #[test]
    fn level_rejects_unknown_object() {
        let game = game();
        let unknown = ObjectId(99);
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![unknown])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::State(StateError3::UnknownObject { object: unknown })
        );
    }

    #[test]
    fn level_rejects_empty_object() {
        let game = game();
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![ObjectId::EMPTY])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::EmptyObject {
                position: Coord3::new(0, 0, 0),
            }
        );
    }

    #[test]
    fn level_rejects_out_of_bounds_position() {
        let game = game();
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(2, 0, 0), vec![PLAYER])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::State(StateError3::PositionOutOfBounds {
                position: Coord3::new(2, 0, 0).into(),
            })
        );
    }

    #[test]
    fn level_rejects_object_layer_outside_game_layer_count() {
        let game = CompiledGame3::new(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: FLOOR,
            }],
            Vec::new(),
        );
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::CompiledGame(CompiledGameError3::ObjectLayerOutOfBounds {
                object: PLAYER,
                layer: FLOOR,
            })
        );
    }

    #[test]
    fn transition_pushes_right_in_x_axis() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), BOX)
            .unwrap();

        let next = transition_once(&game, &state, &push_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(2, 0, 0), BOX));
    }

    #[test]
    fn transition_pushes_forward_in_y_axis() {
        let game = game();
        let mut state = empty_state(1, 3, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 1, 0), BOX)
            .unwrap();

        let next = transition_once(&game, &state, &push_rule(Direction3::FORWARD)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(0, 1, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(0, 2, 0), BOX));
    }

    #[test]
    fn vertical_movement_uses_z_axis() {
        let game = game();
        let mut state = empty_state(1, 1, 2);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::UP)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(0, 0, 1), PLAYER));
    }

    #[test]
    fn transition_does_not_match_out_of_bounds_target() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::DOWN)).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn transition_does_not_match_upper_out_of_bounds_for_forbid_only_cell() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::RIGHT)).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn program_applies_only_rules_guarded_by_current_input() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();

        let rules = vec![
            GridRuleStep::<3>::Rule(move_rule(Direction3::LEFT).when_input(INPUT_LEFT)),
            GridRuleStep::<3>::Rule(move_rule(Direction3::RIGHT).when_input(INPUT_RIGHT)),
        ];

        let next = transition_program(&game, &state, &rules, INPUT_LEFT).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
    }

    #[test]
    fn until_stable_repeats_sweeps_until_state_stops_changing() {
        let game = game();
        let mut state = empty_state(4, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_repeated(&game, &state, &move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(3, 0, 0), PLAYER));
    }

    #[test]
    fn until_stable_finishes_when_a_3d_rule_leaves_state_unchanged() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let no_progress = Rule::repeated(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: PLAYER,
                add: PLAYER,
            }],
        );

        let next = transition_repeated(&game, &state, &no_progress).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn write_component_selects_the_matching_3d_component_origin() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(2, 0, 0), BOX)
            .unwrap();

        let replace_second_component = Rule::once(
            GridPattern::<3>::from_components(vec![
                PatternComponent3::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
                PatternComponent3::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(BOX)]),
            ]),
            vec![GridWriteOp::<3>::Replace {
                component: 1,
                offset: Delta3::ZERO.into(),
                remove: BOX,
                add: WALL,
            }],
        );

        let next = transition_once(&game, &state, &replace_second_component).unwrap();

        assert!(next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(!next.has_object_at(&game, Coord3::new(2, 0, 0), BOX));
        assert!(next.has_object_at(&game, Coord3::new(2, 0, 0), WALL));
    }

    #[test]
    fn once_all_applies_each_initial_3d_match_once() {
        let game = game();
        let mut state = empty_state(5, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(3, 0, 0), PLAYER)
            .unwrap();

        let next =
            transition_once_all(&game, &state, &once_all_move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(!next.has_object_at(&game, Coord3::new(2, 0, 0), PLAYER));
        assert!(!next.has_object_at(&game, Coord3::new(3, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(4, 0, 0), PLAYER));
    }

    #[test]
    fn once_all_does_not_chain_into_3d_matches_created_during_sweep() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next =
            transition_once_all(&game, &state, &once_all_move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(!next.has_object_at(&game, Coord3::new(2, 0, 0), PLAYER));
    }

    #[test]
    fn once_all_skips_3d_matches_invalidated_during_sweep() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(2, 0, 0), PLAYER)
            .unwrap();

        let consume_pair = Rule::once_all(
            GridPattern::<3>::new(vec![
                GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
                GridMatchCell::<3>::new(Direction3::RIGHT.offset).require(PLAYER),
            ]),
            vec![
                GridWriteOp::<3>::Replace {
                    component: 0,
                    offset: Delta3::ZERO.into(),
                    remove: PLAYER,
                    add: BOX,
                },
                GridWriteOp::<3>::Remove {
                    component: 0,
                    offset: Direction3::RIGHT.offset.into(),
                    object: PLAYER,
                },
            ],
        );

        let next = transition_once_all(&game, &state, &consume_pair).unwrap();

        assert!(next.has_object_at(&game, Coord3::new(0, 0, 0), BOX));
        assert_eq!(
            next.get_layer_at(Coord3::new(1, 0, 0), ACTOR).unwrap(),
            ObjectId::EMPTY
        );
        assert!(next.has_object_at(&game, Coord3::new(2, 0, 0), PLAYER));
    }

    #[test]
    fn once_per_level_fires_only_once_for_current_3d_level_state() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();

        let player_to_box = Rule::once_per_level(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: PLAYER,
                add: BOX,
            }],
        )
        .with_id(RuleId3(7));

        let first = transition_program(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(player_to_box.clone())],
            INPUT_RIGHT,
        )
        .unwrap();
        let second = transition_program(
            &game,
            &first,
            &[GridRuleStep::<3>::Rule(player_to_box)],
            INPUT_RIGHT,
        )
        .unwrap();

        assert!(first.has_object_at(&game, Coord3::new(0, 0, 0), BOX));
        assert!(first.has_object_at(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(first.level_rule_has_fired(RuleId3(7)));
        assert_eq!(second, first);
    }

    #[test]
    fn once_per_level_does_not_mark_rule_when_no_3d_match_exists() {
        let game = game();
        let state = empty_state(1, 1, 1);
        let player_to_box = Rule::once_per_level(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: PLAYER,
                add: BOX,
            }],
        )
        .with_id(RuleId3(8));

        let next = transition_program(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(player_to_box)],
            INPUT_RIGHT,
        )
        .unwrap();

        assert_eq!(next, state);
        assert!(!next.level_rule_has_fired(RuleId3(8)));
    }

    #[test]
    fn patch_application_is_all_or_nothing_on_collision() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), WALL)
            .unwrap();

        let patch = Patch3::from_ops(vec![PatchOp3::Move {
            from: Coord3::new(0, 0, 0).into(),
            to: Coord3::new(1, 0, 0).into(),
            object: PLAYER,
        }]);

        assert!(patch.apply_in_place(&game, &mut state).is_err());
        assert!(state.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(state.has_object_at(&game, Coord3::new(1, 0, 0), WALL));
    }

    #[test]
    fn patch_can_update_3d_visible_variables_and_mark() {
        let game = game();
        let mut state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![2]).unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let patch = Patch3::from_ops(vec![
            PatchOp3::UpdateVariable {
                variable: VariableId(0),
                op: VariableUpdateOp::Add,
                value: 3,
            },
            PatchOp3::SetMark {
                position: Coord3::new(0, 0, 0).into(),
                object: PLAYER,
                mark: MarkId3(1),
                value: Some(7),
            },
            PatchOp3::SetMark {
                position: Coord3::new(0, 0, 0).into(),
                object: ObjectId::EMPTY,
                mark: MarkId3(2),
                value: None,
            },
        ]);

        patch.apply_in_place(&game, &mut state).unwrap();

        assert_eq!(state.variable_value(VariableId(0)), Some(5));
        assert!(state.has_mark_at(&game, Coord3::new(0, 0, 0), PLAYER, MarkId3(1), Some(7)));
        assert!(state.has_cell_mark_key_at(Coord3::new(0, 0, 0), MarkId3(2)));
    }

    #[test]
    fn rule_effect_can_update_3d_visible_variable_like_core() {
        let game = game();
        let mut state = State3::empty_with_variables(Size3::new(1, 1, 1), 1, vec![2]).unwrap();
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let rule = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            Vec::new(),
        )
        .with_effects(vec![RuleEffect3::UpdateVariable {
            variable: VariableId(0),
            op: VariableUpdateOp::Add,
            value: 4,
        }]);

        let next = transition_once(&game, &state, &rule).unwrap();

        assert_eq!(next.variable_value(VariableId(0)), Some(6));
    }

    #[test]
    fn write_op3_can_set_mark_for_later_rule_like_core() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let mark_player = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            vec![GridWriteOp::<3>::SetMark {
                component: 0,
                offset: Delta3::ZERO.into(),
                object: PLAYER,
                mark: MarkId3(1),
                value: Some(7),
            }],
        );
        let consume_mark = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require_mark(
                PLAYER,
                MarkId3(1),
                Some(7),
            )]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: PLAYER,
                add: WALL,
            }],
        );

        let next = transition_program_without_input(
            &game,
            &state,
            &[
                GridRuleStep::<3>::Rule(mark_player),
                GridRuleStep::<3>::Rule(consume_mark),
            ],
        )
        .unwrap();

        assert!(!next.has_object_at(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object_at(&game, Coord3::new(0, 0, 0), WALL));
        assert!(!next.has_mark_at(&game, Coord3::new(0, 0, 0), PLAYER, MarkId3(1), Some(7)));
    }

    #[test]
    fn variable_offset3_resolves_against_runtime_gap_assignment() {
        let game = game();
        let mut state = empty_state(4, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(3, 0, 0), BOX)
            .unwrap();
        let mut component = PatternComponent3::new(vec![
            GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
            GridMatchCell::<3>::new(Offset3::Variable {
                base: [1, 0, 0].into(),
                gap_terms: vec![GapTerm3 {
                    gap_index: 0,
                    delta: [1, 0, 0].into(),
                }],
            })
            .require(BOX),
        ]);
        component.gap_count = 1;
        let pattern = GridPattern::<3>::from_components(vec![component]);

        assert_eq!(count_pattern_matches(&game, &state, &pattern), 1);
    }

    #[test]
    fn once_all_revalidates_the_saved_gap_assignment() {
        let game = layered_game();
        let mut state = State3::empty(Size3::new(11, 1, 1), 2).unwrap();
        for x in [0, 4] {
            state
                .place_object_at(&game, Coord3::new(x, 0, 0), PLAYER)
                .unwrap();
        }
        for x in [2, 6, 7, 10] {
            state
                .place_object_at(&game, Coord3::new(x, 0, 0), BOX)
                .unwrap();
        }
        let mut component = PatternComponent3::new(vec![
            GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
            GridMatchCell::<3>::new(Offset3::Variable {
                base: [1, 0, 0].into(),
                gap_terms: vec![GapTerm3 {
                    gap_index: 0,
                    delta: [1, 0, 0].into(),
                }],
            })
            .require(BOX),
        ]);
        component.gap_count = 1;
        let rule = Rule::once_all(
            GridPattern::<3>::from_components(vec![component]),
            vec![
                GridWriteOp::<3>::Remove {
                    component: 0,
                    offset: Delta3::new(6, 0, 0).into(),
                    object: BOX,
                },
                GridWriteOp::<3>::Add {
                    component: 0,
                    offset: Delta3::ZERO.into(),
                    object: GOAL,
                },
            ],
        );

        let next = transition_once_all(&game, &state, &rule).unwrap();

        assert!(next.has_object_at(&game, Coord3::new(0, 0, 0), GOAL));
        assert!(!next.has_object_at(&game, Coord3::new(4, 0, 0), GOAL));
        assert!(next.has_object_at(&game, Coord3::new(10, 0, 0), BOX));
    }

    #[test]
    fn cancel_effect_validates_but_does_not_apply_patch_or_emit_commands() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let rule = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: PLAYER,
                add: WALL,
            }],
        )
        .with_effects(vec![RuleEffect3::Cancel, RuleEffect3::Win]);

        let outcome = transition_program_without_input_outcome(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule)],
        )
        .unwrap();

        assert!(outcome.cancelled);
        assert!(outcome.commands.is_empty());
        assert!(
            outcome
                .next_state
                .has_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
        );
        assert!(
            !outcome
                .next_state
                .has_object_at(&game, Coord3::new(0, 0, 0), WALL)
        );
    }

    #[test]
    fn rule_effect3_emits_the_same_runtime_commands_as_effect() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let rule = Rule::once(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER)]),
            Vec::new(),
        )
        .with_effects(vec![
            RuleEffect3::Win,
            RuleEffect3::Restart,
            RuleEffect3::NextLevel,
            RuleEffect3::Again,
            RuleEffect3::Checkpoint,
            RuleEffect3::ClearCheckpoint,
        ]);

        let outcome = transition_program_without_input_outcome(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule)],
        )
        .unwrap();

        assert_eq!(
            outcome.commands,
            vec![
                TransitionCommand3::Win,
                TransitionCommand3::Restart,
                TransitionCommand3::NextLevel,
                TransitionCommand3::Again,
                TransitionCommand3::Checkpoint,
                TransitionCommand3::ClearCheckpoint,
            ]
        );
    }

    #[test]
    fn compiled_game3_owns_mark_definitions_like_compiled_game() {
        let game = CompiledGame3::new_with_mark_condition_defs_and_program(
            1,
            Vec::new(),
            vec![MarkDef3 {
                id: MarkId3(7),
                kind: MarkKind::Enum,
                values: vec!["open".to_string(), "closed".to_string()],
            }],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(game.mark().len(), 1);
        assert_eq!(game.mark()[0].id, MarkId3(7));
    }

    #[test]
    fn move_patch_preserves_3d_slot_mark() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        Patch3::from_ops(vec![PatchOp3::SetMark {
            position: Coord3::new(0, 0, 0).into(),
            object: PLAYER,
            mark: MarkId3(1),
            value: Some(9),
        }])
        .apply_in_place(&game, &mut state)
        .unwrap();

        Patch3::from_ops(vec![PatchOp3::Move {
            from: Coord3::new(0, 0, 0).into(),
            to: Coord3::new(1, 0, 0).into(),
            object: PLAYER,
        }])
        .apply_in_place(&game, &mut state)
        .unwrap();

        assert!(!state.has_mark_at(&game, Coord3::new(0, 0, 0), PLAYER, MarkId3(1), Some(9)));
        assert!(state.has_mark_at(&game, Coord3::new(1, 0, 0), PLAYER, MarkId3(1), Some(9)));
    }

    #[test]
    fn condition_kind_evaluates_3d_objects_and_patterns() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 0), BOX)
            .unwrap();
        let push_pattern = GridPattern::<3>::new(vec![
            GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
            GridMatchCell::<3>::new(Direction3::RIGHT.offset).require(BOX),
        ]);

        assert_eq!(
            eval_condition_kind(
                &game,
                &state,
                &ConditionValueKind3::CountObjects(vec![PLAYER, BOX]),
                None,
                None,
            ),
            2
        );
        assert_eq!(
            eval_condition_kind(
                &game,
                &state,
                &ConditionValueKind3::ExistsMatches(vec![push_pattern.clone()]),
                None,
                None,
            ),
            1
        );
        assert_eq!(
            eval_condition_kind(
                &game,
                &state,
                &ConditionValueKind3::CountInputMatches(vec![(INPUT_RIGHT, push_pattern)]),
                Some(INPUT_RIGHT),
                None,
            ),
            1
        );
    }

    #[test]
    fn local_frame_full_height_limits_3d_rules_by_horizontal_frame_only() {
        let game = game();
        let mut state = empty_state(4, 1, 3);
        state
            .place_object_at(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(1, 0, 2), BOX)
            .unwrap();
        state
            .place_object_at(&game, Coord3::new(3, 0, 0), BOX)
            .unwrap();
        let rule = Rule::once_all(
            GridPattern::<3>::new(vec![GridMatchCell::<3>::new(Delta3::ZERO).require(BOX)]),
            vec![GridWriteOp::<3>::Replace {
                component: 0,
                offset: Delta3::ZERO.into(),
                remove: BOX,
                add: WALL,
            }],
        );
        let frame = LocalFrame::new(
            LocalFrameExtent::Radius(1),
            LocalFrameExtent::Radius(1),
            LocalFrameExtent::Full,
            vec![PLAYER],
        );

        let next = transition_program_with_local_frame(
            &game,
            &state,
            &[GridRuleStep::<3>::Rule(rule)],
            INPUT_RIGHT,
            Some(&frame),
        )
        .unwrap();

        assert!(next.has_object_at(&game, Coord3::new(1, 0, 2), WALL));
        assert!(next.has_object_at(&game, Coord3::new(3, 0, 0), BOX));
    }
}
