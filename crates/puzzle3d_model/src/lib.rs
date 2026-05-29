mod ids;
mod level;
mod model;
mod parser;
mod patch;
mod selector;
mod session;
mod snapshot;
mod sprite;
mod state;
mod transition;
mod visual;
mod visual_fixture;
mod win;

pub use ids::{GlobalId3, InputId3, LayerId, ObjectId, RuleId3, ScratchId3};
pub use level::{Level3, LevelBundle3, LevelBundleError3, LevelCell3, LevelEntry3, LevelError3};
pub use model::{
    Axis3, Coord3, Direction3, DirectionSet3, Frame3, FrameChirality3, FrameError3, FrameExpr3,
    FrameSet3, FrameSlot3, Game3, GameError3, InputDef3, ObjectDef3, Offset3, Size3,
};
pub use parser::{
    CAMERA_ASSIGNMENT_OPTIONS3, CAMERA_BARE_OPTIONS3, CAMERA_OPTIONS3, CameraSettings3,
    GRID_BARE_OPTIONS3, GridSettings3, ModelSettings3, PIXELATE_ASSIGNMENT_OPTIONS3,
    PIXELATE_BARE_OPTIONS3, PIXELATE_OPTIONS3, ParseError3, ParsedPuzzle3, RENDER_BARE_OPTIONS3,
    RENDER_BLOCK_OPTIONS3, RENDER_OPTIONS3, ViewportFollow3, ViewportFraming3, ViewportHeight3,
    ViewportMode3, ViewportSettings3, parse_puzzle3d,
};
pub use patch::{Patch3, PatchError3, PatchOp3};
pub use puzzle_kernel::{
    GlobalUpdateOp, LocalFrame, LocalFrameExtent, ScratchKind, ScratchValueMatch,
};
pub use selector::{
    ConcreteObject3, DenseCell3, DensePattern3, DenseRow3, DenseRuleTemplate3, DenseSlice3,
    FrameOrientation3, LineMatchCellTemplate3, LineOrientation3, LinePatternTemplate3,
    LineRuleTemplate3, LineWriteOpTemplate3, LocalWriteOpTemplate3, MatchCellTemplate3,
    ObjectFamily3, ObjectSelector3, ObjectVariant3, PatternLoweringError3, PatternTemplate3,
    ResolvedSelector3, RuleLoweringError3, RuleTemplate3, SelectorCatalog3, SelectorCatalogError3,
    SelectorError3, SelectorGroup3, SelectorScratch3, SelectorTag3, SelectorTransform3,
    VariantAxis3, VariantValueSet3, WriteOpTemplate3, lower_dense_pattern, lower_dense_pattern_set,
    lower_dense_pattern_set_to_patterns, lower_dense_pattern_to_patterns,
    lower_dense_rule_template, lower_line_rule_template, lower_pattern_template,
    lower_rule_template,
};
pub use session::{
    GameSession3, GameSessionError3, Lifecycle3, LifecycleCommand3, SessionLifecycleResult3,
};
pub use snapshot::{BoardCell3, BoardSnapshot3};
pub use sprite::{Sprite3, SpriteColor3, SpriteSet3, SpriteVoxels3};
pub use state::{CellView3, SlotScratch3, State3, StateError3};
pub use transition::{
    Guard3, MatchCell3, ObjectSetMatcher3, ObjectSetScratchPattern3, Pattern3, QueryKind3, Rule3,
    RuleApplication3, RuleEffect3, ScratchPattern3, TransitionError3, WriteOp3,
    count_pattern_matches, eval_query_kind, has_pattern_match, transition_once,
    transition_once_all, transition_once_per_level, transition_once_with_input, transition_program,
    transition_program_with_local_frame, transition_program_without_input,
    transition_program_without_input_with_local_frame, transition_repeated,
    transition_solver_program,
};
pub use visual::{ObjectVisual3, VisualCell3, VisualObject3, VisualSnapshot3};
pub use visual_fixture::{
    VisualFixtureExportError3, export_visual_fixture_json, export_visual_fixture_json_with_title,
    export_visual_fixture_json_with_title_and_scenes,
};
pub use win::WinCondition3;

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const GOAL: ObjectId = ObjectId(4);
    const MARKER_LEFT: ObjectId = ObjectId(10);
    const MARKER_RIGHT: ObjectId = ObjectId(11);
    const MARKER_FORWARD: ObjectId = ObjectId(12);
    const MARKER_BACKWARD: ObjectId = ObjectId(13);
    const MARKER_UP: ObjectId = ObjectId(14);
    const MARKER_DOWN: ObjectId = ObjectId(15);
    const ACTOR: LayerId = LayerId(0);
    const FLOOR: LayerId = LayerId(1);
    const INPUT_LEFT: InputId3 = InputId3(0);
    const INPUT_RIGHT: InputId3 = InputId3(1);
    const INPUT_UP: InputId3 = InputId3(2);
    const INPUT_FORWARD: InputId3 = InputId3(4);
    const INPUT_BACKWARD: InputId3 = InputId3(5);
    const MICROBAN_FLOOR: ObjectId = ObjectId(1);
    const MICROBAN_GOAL: ObjectId = ObjectId(2);
    const MICROBAN_PLAYER: ObjectId = ObjectId(3);
    const MICROBAN_BOX: ObjectId = ObjectId(4);
    const MICROBAN_WALL: ObjectId = ObjectId(5);

    fn game() -> Game3 {
        Game3::new_with_inputs(
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
            vec![
                InputDef3::directional(INPUT_LEFT, "left", Direction3::LEFT),
                InputDef3::directional(INPUT_RIGHT, "right", Direction3::RIGHT),
                InputDef3::directional(INPUT_UP, "up", Direction3::UP),
                InputDef3::directional(InputId3(3), "down", Direction3::DOWN),
                InputDef3::directional(InputId3(4), "front", Direction3::FORWARD),
                InputDef3::directional(INPUT_BACKWARD, "back", Direction3::BACKWARD),
                InputDef3::action(InputId3(6), "restart"),
            ],
        )
    }

    fn empty_state(width: u16, depth: u16, height: u16) -> State3 {
        State3::empty(Size3::new(width, depth, height), 1).unwrap()
    }

    fn layered_game() -> Game3 {
        Game3::new(
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
        )
    }

    fn selector_catalog() -> SelectorCatalog3 {
        SelectorCatalog3::new(
            vec![
                ConcreteObject3::new(PLAYER, "Player"),
                ConcreteObject3::new(BOX, "Box"),
                ConcreteObject3::new(WALL, "Wall"),
            ],
            vec![ObjectFamily3::new(
                "Marker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Directions,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_RIGHT, vec!["right"]),
                    ObjectVariant3::new(MARKER_FORWARD, vec!["front"]),
                    ObjectVariant3::new(MARKER_BACKWARD, vec!["back"]),
                    ObjectVariant3::new(MARKER_UP, vec!["up"]),
                    ObjectVariant3::new(MARKER_DOWN, vec!["down"]),
                ],
            )],
            vec![SelectorGroup3::new(
                "solid",
                vec![
                    ObjectSelector3::object("Player"),
                    ObjectSelector3::object("Box"),
                    ObjectSelector3::object("Wall"),
                    ObjectSelector3::object("Box"),
                ],
            )],
        )
    }

    fn push_rule(direction: Direction3) -> Rule3 {
        let step = direction.offset;
        let two_steps = step.scale(2);
        Rule3::once(
            Pattern3::new(vec![
                MatchCell3::new(Offset3::ZERO).require(PLAYER),
                MatchCell3::new(step).require(BOX),
                MatchCell3::new(two_steps)
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![
                WriteOp3::Move {
                    from_offset: step,
                    to_offset: two_steps,
                    object: BOX,
                },
                WriteOp3::Move {
                    from_offset: Offset3::ZERO,
                    to_offset: step,
                    object: PLAYER,
                },
            ],
        )
    }

    fn move_rule(direction: Direction3) -> Rule3 {
        Rule3::once(
            Pattern3::new(vec![
                MatchCell3::new(Offset3::ZERO).require(PLAYER),
                MatchCell3::new(direction.offset)
                    .forbid(PLAYER)
                    .forbid(BOX)
                    .forbid(WALL),
            ]),
            vec![WriteOp3::Move {
                from_offset: Offset3::ZERO,
                to_offset: direction.offset,
                object: PLAYER,
            }],
        )
    }

    fn once_all_move_rule(direction: Direction3) -> Rule3 {
        let rule = move_rule(direction);
        Rule3::once_all(rule.pattern, rule.writes)
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestVoxelSprite3 {
        name: &'static str,
        size: Size3,
        palette: Vec<&'static str>,
        layers: Vec<Vec<&'static str>>,
    }

    impl TestVoxelSprite3 {
        fn filled_voxel_count(&self) -> usize {
            self.layers
                .iter()
                .flatten()
                .flat_map(|row| row.chars())
                .filter(|ch| *ch != '.' && *ch != ' ')
                .count()
        }
    }

    fn bottom_voxel_sprite(
        name: &'static str,
        palette: &[&'static str],
        bottom: &[&'static str],
    ) -> TestVoxelSprite3 {
        assert_eq!(bottom.len(), 5);
        assert!(bottom.iter().all(|row| row.chars().count() == 5));

        let mut layers = vec![bottom.to_vec()];
        for _ in 1..5 {
            layers.push(vec!["....."; 5]);
        }
        TestVoxelSprite3 {
            name,
            size: Size3::new(5, 5, 5),
            palette: palette.to_vec(),
            layers,
        }
    }

    fn microban_basic_sprites() -> Vec<TestVoxelSprite3> {
        vec![
            bottom_voxel_sprite(
                "@Floor",
                &["#90ee90", "#008000"],
                &["11111", "01111", "11101", "11111", "10111"],
            ),
            bottom_voxel_sprite(
                "Wall",
                &["#a46322", "#493c2b"],
                &["00010", "11111", "01000", "11111", "00010"],
            ),
            bottom_voxel_sprite(
                "Goal",
                &["#00008b"],
                &[".....", ".000.", ".0.0.", ".000.", "....."],
            ),
            bottom_voxel_sprite(
                "Box",
                &["#ffa500", "#ffff00"],
                &["00000", "0...0", "0...0", "0...0", "00000"],
            ),
            bottom_voxel_sprite(
                "Player",
                &["#000000", "#ffa500", "#ffffff", "#0000ff"],
                &[".000.", ".111.", "22222", ".333.", ".3.3."],
            ),
        ]
    }

    fn microban_basic_model() -> ParsedPuzzle3 {
        parse_puzzle3d(
            r#"
layers {
floor
target
solid
}

objects {
@Floor floor
Goal target
Player solid
Box solid
Wall solid
}

group solid = Player Box Wall

rules {
horizontal [ Player | Box | no solid ] -> [ | Player | Box ]
horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
        )
        .unwrap()
    }

    fn microban_basic_rules_with_input_guards(rules: &[Rule3]) -> Vec<Rule3> {
        rules
            .iter()
            .cloned()
            .map(|rule| {
                let direction = rule.pattern.cells[1].offset;
                rule.when_input(input_for_microban_offset(direction))
            })
            .collect()
    }

    fn input_for_microban_offset(offset: Offset3) -> InputId3 {
        if offset == Direction3::LEFT.offset {
            INPUT_LEFT
        } else if offset == Direction3::RIGHT.offset {
            INPUT_RIGHT
        } else if offset == Direction3::FORWARD.offset {
            INPUT_FORWARD
        } else if offset == Direction3::BACKWARD.offset {
            INPUT_BACKWARD
        } else {
            panic!("Microban Basic only uses horizontal movement, got {offset:?}");
        }
    }

    fn microban_basic_01_level() -> Level3 {
        microban_basic_level_from_rows(&[
            "####..", "#.G#..", "#..###", "#*P..#", "#..B.#", "#..###", "####..",
        ])
    }

    fn microban_basic_level_from_rows(rows: &[&str]) -> Level3 {
        let depth = rows.len() as u16;
        let width = rows
            .first()
            .expect("Microban fixture has rows")
            .chars()
            .count() as u16;
        let mut cells = Vec::new();

        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.chars().count(), usize::from(width));
            for (x, ch) in row.chars().enumerate() {
                let mut objects = vec![MICROBAN_FLOOR];
                match ch {
                    '.' => {}
                    'G' => objects.push(MICROBAN_GOAL),
                    'P' => objects.push(MICROBAN_PLAYER),
                    'B' => objects.push(MICROBAN_BOX),
                    '#' => objects.push(MICROBAN_WALL),
                    '*' => objects.extend([MICROBAN_GOAL, MICROBAN_BOX]),
                    '+' => objects.extend([MICROBAN_GOAL, MICROBAN_PLAYER]),
                    _ => panic!("unknown Microban Basic cell: {ch}"),
                }
                cells.push(LevelCell3::new(Coord3::new(x as u16, y as u16, 0), objects));
            }
        }

        Level3::new(Size3::new(width, depth, 1), cells)
    }

    #[test]
    fn direction_sets_use_absolute_3d_grid_axes() {
        let directions = Direction3::directions();
        assert_eq!(
            directions.map(|direction| direction.name),
            ["up", "down", "left", "right", "front", "back"]
        );
        assert_eq!(Direction3::UP.offset, Offset3::new(0, 0, 1));
        assert_eq!(Direction3::DOWN.offset, Offset3::new(0, 0, -1));
        assert_eq!(Direction3::LEFT.offset, Offset3::new(-1, 0, 0));
        assert_eq!(Direction3::RIGHT.offset, Offset3::new(1, 0, 0));
        assert_eq!(Direction3::FORWARD.offset, Offset3::new(0, 1, 0));
        assert_eq!(Direction3::BACKWARD.offset, Offset3::new(0, -1, 0));

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
            anti.to_world_offset(Offset3::new(1, 1, 1)),
            Offset3::new(1, 1, 1)
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
            frame.to_world_offset(Offset3::new(1, 0, 0)),
            Direction3::FORWARD.offset
        );
        assert_eq!(
            frame.to_world_offset(Offset3::new(0, 1, 0)),
            Direction3::UP.offset
        );
        assert_eq!(
            frame.to_world_offset(Offset3::new(0, 0, 1)),
            Direction3::RIGHT.offset
        );
        assert_eq!(
            frame.to_world_offset(Offset3::new(2, 1, 3)),
            Offset3::new(3, 2, 1)
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
    fn object_selector_resolves_concrete_object_names() {
        let catalog = selector_catalog();
        let resolved = catalog.resolve(&ObjectSelector3::object("Player")).unwrap();

        assert_eq!(resolved.token, "Player");
        assert_eq!(resolved.alternatives, vec![PLAYER]);
        assert_eq!(resolved.transform, None);
        assert_eq!(resolved.scratch, Vec::<SelectorScratch3>::new());
    }

    #[test]
    fn group_selector_expands_members_and_deduplicates_in_order() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog.resolve(&ObjectSelector3::group("solid")).unwrap(),
            ResolvedSelector3 {
                token: "solid".to_string(),
                alternatives: vec![PLAYER, BOX, WALL],
                transform: None,
                scratch: Vec::new(),
            }
        );
    }

    #[test]
    fn group_selector_can_expand_nested_selectors() {
        let catalog = SelectorCatalog3::new(
            vec![ConcreteObject3::new(WALL, "Wall")],
            vec![ObjectFamily3::new(
                "Marker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Directions,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_RIGHT, vec!["right"]),
                    ObjectVariant3::new(MARKER_UP, vec!["up"]),
                ],
            )],
            vec![SelectorGroup3::new(
                "blocked",
                vec![
                    ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
                    ObjectSelector3::object("Wall"),
                ],
            )],
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::group("blocked"))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_RIGHT, WALL]
        );
    }

    #[test]
    fn direction_variant_selector_matches_single_direction_value() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("right")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_RIGHT]
        );
    }

    #[test]
    fn direction_set_selector_matches_direction_subset() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("horizontal")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_RIGHT, MARKER_FORWARD, MARKER_BACKWARD]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("vertical")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_UP, MARKER_DOWN]
        );
    }

    #[test]
    fn explicit_any_selector_matches_all_variants() {
        let catalog = selector_catalog();
        let resolved = catalog
            .resolve(&ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            ))
            .unwrap();

        assert_eq!(resolved.token, "Marker:*");
        assert_eq!(
            resolved.alternatives,
            vec![
                MARKER_LEFT,
                MARKER_RIGHT,
                MARKER_FORWARD,
                MARKER_BACKWARD,
                MARKER_UP,
                MARKER_DOWN,
            ]
        );
    }

    #[test]
    fn any_selector_fills_all_variant_slots_for_multi_axis_family() {
        let target_a_on = ObjectId(20);
        let target_b_on = ObjectId(21);
        let target_a_off = ObjectId(22);
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "Target",
                vec![
                    VariantAxis3::named("kind", vec!["A", "B"]),
                    VariantAxis3::named("state", vec!["on", "off"]),
                ],
                vec![
                    ObjectVariant3::new(target_a_on, vec!["A", "on"]),
                    ObjectVariant3::new(target_b_on, vec!["B", "on"]),
                    ObjectVariant3::new(target_a_off, vec!["A", "off"]),
                ],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::any()]
                ))
                .unwrap()
                .alternatives,
            vec![target_a_on, target_b_on, target_a_off]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::value("A"), SelectorTag3::any()]
                ))
                .unwrap()
                .alternatives,
            vec![target_a_on, target_a_off]
        );
    }

    #[test]
    fn partial_multi_axis_variant_selector_is_rejected() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "Target",
                vec![
                    VariantAxis3::named("kind", vec!["A", "B"]),
                    VariantAxis3::named("state", vec!["on", "off"]),
                ],
                vec![ObjectVariant3::new(ObjectId(20), vec!["A", "on"])],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::value("A")]
                ))
                .unwrap_err(),
            SelectorError3::PartialVariantSelector {
                family: "Target".to_string(),
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn bare_variant_family_selector_is_rejected() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::object("Marker"))
                .unwrap_err(),
            SelectorError3::BareVariantFamily {
                family: "Marker".to_string(),
            }
        );
    }

    #[test]
    fn frame_set_names_are_not_direction_selector_tags() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("canonical")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "canonical".to_string(),
            }
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("mirrored")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "mirrored".to_string(),
            }
        );
    }

    #[test]
    fn direction_selector_respects_axis_subset() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "HorizontalMarker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Horizontal,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_FORWARD, vec!["front"]),
                ],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "HorizontalMarker",
                    vec![SelectorTag3::value("directions")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_FORWARD]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "HorizontalMarker",
                    vec![SelectorTag3::value("vertical")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "HorizontalMarker".to_string(),
                axis: "direction".to_string(),
                tag: "vertical".to_string(),
            }
        );
    }

    #[test]
    fn checked_selector_catalog_rejects_shadowed_selector_names() {
        assert_eq!(
            SelectorCatalog3::checked_new(
                vec![ConcreteObject3::new(PLAYER, "Marker")],
                vec![ObjectFamily3::new(
                    "Marker",
                    vec![VariantAxis3::directions(
                        "direction",
                        DirectionSet3::Directions
                    )],
                    vec![ObjectVariant3::new(MARKER_LEFT, vec!["left"])],
                )],
                Vec::new(),
            )
            .unwrap_err(),
            SelectorCatalogError3::FamilyNameShadowsObject {
                name: "Marker".to_string(),
            }
        );

        assert_eq!(
            SelectorCatalog3::checked_new(
                vec![ConcreteObject3::new(PLAYER, "Player")],
                Vec::new(),
                vec![SelectorGroup3::new(
                    "Player",
                    vec![ObjectSelector3::object("Player")]
                )],
            )
            .unwrap_err(),
            SelectorCatalogError3::GroupNameShadowsSelector {
                name: "Player".to_string(),
            }
        );
    }

    #[test]
    fn recursive_group_selector_is_rejected() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            Vec::new(),
            vec![SelectorGroup3::new(
                "loop",
                vec![ObjectSelector3::group("loop")],
            )],
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::group("loop"))
                .unwrap_err(),
            SelectorError3::RecursiveGroup {
                name: "loop".to_string(),
            }
        );
    }

    #[test]
    fn pattern_template_expands_required_selector_alternatives() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
        )]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 4);
        assert_eq!(
            patterns
                .iter()
                .map(|pattern| pattern.cells[0].require_objects.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![MARKER_LEFT],
                vec![MARKER_RIGHT],
                vec![MARKER_FORWARD],
                vec![MARKER_BACKWARD],
            ]
        );
    }

    #[test]
    fn pattern_template_collects_forbidden_selector_alternatives() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Direction3::RIGHT.offset)
                .forbid(ObjectSelector3::group("solid")),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].cells[0].offset, Direction3::RIGHT.offset);
        assert_eq!(patterns[0].cells[0].forbid_objects, vec![PLAYER, BOX, WALL]);
    }

    #[test]
    fn pattern_template_expands_required_selectors_as_cartesian_product() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("horizontal")],
            )),
            MatchCellTemplate3::new(Direction3::UP.offset).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("vertical")],
            )),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 8);
        assert_eq!(patterns[0].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[0].cells[1].require_objects, vec![MARKER_UP]);
        assert_eq!(patterns[1].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[1].cells[1].require_objects, vec![MARKER_DOWN]);
    }

    #[test]
    fn repeated_required_selector_token_preserves_assignment() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            )),
            MatchCellTemplate3::new(Direction3::RIGHT.offset).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            )),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 6);
        assert!(patterns.iter().all(|pattern| {
            pattern.cells[0].require_objects[0] == pattern.cells[1].require_objects[0]
        }));
        assert_eq!(
            patterns
                .iter()
                .map(|pattern| pattern.cells[0].require_objects[0])
                .collect::<Vec<_>>(),
            vec![
                MARKER_LEFT,
                MARKER_RIGHT,
                MARKER_FORWARD,
                MARKER_BACKWARD,
                MARKER_UP,
                MARKER_DOWN,
            ]
        );
    }

    #[test]
    fn pattern_template_reports_selector_errors() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("frames")]),
        )]);

        assert_eq!(
            lower_pattern_template(&catalog, &template).unwrap_err(),
            PatternLoweringError3::Selector(SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "frames".to_string(),
            })
        );
    }

    #[test]
    fn rule_template_lowers_selector_assignments_into_move_writes() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
        )]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
            }],
        )
        .with_id(RuleId3(42))
        .when_input(INPUT_RIGHT);

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].id, RuleId3(42));
        assert_eq!(rules[0].guards, vec![Guard3::InputIs(INPUT_RIGHT)]);
        assert_eq!(rules[0].application, RuleApplication3::Once);
        for rule in &rules {
            let required = rule.pattern.cells[0].require_objects[0];
            assert_eq!(
                rule.writes,
                vec![WriteOp3::Move {
                    from_offset: Offset3::ZERO,
                    to_offset: Direction3::RIGHT.offset,
                    object: required,
                }]
            );
        }
    }

    #[test]
    fn rule_template_allows_unassigned_singleton_write_selector() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::object("Player")),
        ]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Add {
                offset: Direction3::UP.offset,
                object: ObjectSelector3::object("Wall"),
            }],
        );

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Add {
                offset: Direction3::UP.offset,
                object: WALL,
            }]
        );
    }

    #[test]
    fn rule_template_rejects_ambiguous_unassigned_write_selector() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::object("Player")),
        ]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Add {
                offset: Direction3::UP.offset,
                object: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
            }],
        );

        assert_eq!(
            lower_rule_template(&catalog, &rule).unwrap_err(),
            RuleLoweringError3::AmbiguousWriteSelector {
                token: "Marker:horizontal".to_string(),
                alternatives: vec![MARKER_LEFT, MARKER_RIGHT, MARKER_FORWARD, MARKER_BACKWARD],
            }
        );
    }

    #[test]
    fn rule_template_lowers_replace_with_bound_remove_and_singleton_add() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("vertical")]),
        )]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Replace {
                offset: Offset3::ZERO,
                remove: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("vertical")]),
                add: ObjectSelector3::object("Wall"),
            }],
        );

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
                remove: MARKER_UP,
                add: WALL,
            }]
        );
        assert_eq!(
            rules[1].writes,
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
                remove: MARKER_DOWN,
                add: WALL,
            }]
        );
    }

    #[test]
    fn dense_pattern_lowers_columns_rows_and_slices_through_frame() {
        let dense = DensePattern3::new(vec![
            DenseSlice3::new(vec![
                DenseRow3::new(vec![
                    DenseCell3::require(ObjectSelector3::object("Player")),
                    DenseCell3::require(ObjectSelector3::object("Box")),
                ]),
                DenseRow3::new(vec![
                    DenseCell3::empty(),
                    DenseCell3::forbid(ObjectSelector3::group("solid")),
                ]),
            ]),
            DenseSlice3::new(vec![DenseRow3::new(vec![DenseCell3::require(
                ObjectSelector3::object("Wall"),
            )])]),
        ]);

        let template = lower_dense_pattern(Frame3::DEFAULT, &dense);

        assert_eq!(
            template
                .cells
                .iter()
                .map(|cell| cell.offset)
                .collect::<Vec<_>>(),
            vec![
                Offset3::ZERO,
                Direction3::RIGHT.offset,
                Direction3::RIGHT.offset.add(Direction3::BACKWARD.offset),
                Direction3::DOWN.offset,
            ]
        );
        assert_eq!(
            template.cells[0].require,
            vec![ObjectSelector3::object("Player")]
        );
        assert_eq!(
            template.cells[2].forbid,
            vec![ObjectSelector3::group("solid")]
        );
    }

    #[test]
    fn dense_pattern_uses_frame_orientation_for_world_offsets() {
        let frame = Frame3::canonical(Direction3::FORWARD, Direction3::UP).unwrap();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![
            DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ]),
            DenseRow3::new(vec![DenseCell3::require(ObjectSelector3::object("Wall"))]),
        ])]);

        let template = lower_dense_pattern(frame, &dense);

        assert_eq!(
            template
                .cells
                .iter()
                .map(|cell| cell.offset)
                .collect::<Vec<_>>(),
            vec![
                Offset3::ZERO,
                Direction3::FORWARD.offset,
                Direction3::UP.offset,
            ]
        );
    }

    #[test]
    fn dense_pattern_connects_to_selector_pattern_lowering() {
        let catalog = selector_catalog();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
            DenseCell3::require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("horizontal")],
            )),
            DenseCell3::forbid(ObjectSelector3::group("solid")),
        ])])]);

        let patterns = lower_dense_pattern_to_patterns(&catalog, Frame3::DEFAULT, &dense).unwrap();

        assert_eq!(patterns.len(), 4);
        assert_eq!(patterns[0].cells[0].offset, Offset3::ZERO);
        assert_eq!(patterns[0].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[0].cells[1].offset, Direction3::RIGHT.offset);
        assert_eq!(patterns[0].cells[1].forbid_objects, vec![PLAYER, BOX, WALL]);
    }

    #[test]
    fn dense_pattern_set_expands_all_frames_before_selector_lowering() {
        let catalog = selector_catalog();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
            DenseCell3::require(ObjectSelector3::object("Player")),
            DenseCell3::require(ObjectSelector3::object("Box")),
        ])])]);

        let patterns =
            lower_dense_pattern_set_to_patterns(&catalog, FrameSet3::Canonical, &dense).unwrap();

        assert_eq!(patterns.len(), 24);
        assert!(patterns.iter().any(|pattern| {
            pattern.cells[0].offset == Offset3::ZERO
                && pattern.cells[1].offset == Direction3::RIGHT.offset
        }));
        assert!(patterns.iter().any(|pattern| {
            pattern.cells[0].offset == Offset3::ZERO
                && pattern.cells[1].offset == Direction3::UP.offset
        }));
    }

    #[test]
    fn line_rule_template_expands_direction_set_sugar_to_concrete_rules() {
        let catalog = selector_catalog();
        let rule = LineRuleTemplate3::once(
            LineOrientation3::DirectionSet(DirectionSet3::Horizontal),
            LinePatternTemplate3::new(vec![
                LineMatchCellTemplate3::new(0).require(ObjectSelector3::object("Player")),
                LineMatchCellTemplate3::new(1).forbid(ObjectSelector3::group("solid")),
            ]),
            vec![LineWriteOpTemplate3::Move {
                from_step: 0,
                to_step: 1,
                object: ObjectSelector3::object("Player"),
            }],
        );

        let rules = lower_line_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 4);
        assert_eq!(
            rules
                .iter()
                .map(|rule| rule.pattern.cells[1].offset)
                .collect::<Vec<_>>(),
            vec![
                Direction3::LEFT.offset,
                Direction3::RIGHT.offset,
                Direction3::FORWARD.offset,
                Direction3::BACKWARD.offset,
            ]
        );
        assert_eq!(
            rules[1].writes,
            vec![WriteOp3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: PLAYER,
            }]
        );
    }

    #[test]
    fn dense_rule_template_transforms_local_writes_through_frame() {
        let catalog = selector_catalog();
        let frame = Frame3::canonical(Direction3::FORWARD, Direction3::UP).unwrap();
        let rule = DenseRuleTemplate3::once(
            FrameOrientation3::Frame(frame),
            DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ])])]),
            vec![LocalWriteOpTemplate3::Move {
                from_offset: Offset3::new(1, 0, 0),
                to_offset: Offset3::new(2, 0, 0),
                object: ObjectSelector3::object("Box"),
            }],
        );

        let rules = lower_dense_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].pattern.cells[1].offset, Direction3::FORWARD.offset);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Move {
                from_offset: Direction3::FORWARD.offset,
                to_offset: Direction3::FORWARD.offset.scale(2),
                object: BOX,
            }]
        );
    }

    #[test]
    fn dense_rule_template_expands_frame_set_sugar_to_concrete_rules() {
        let catalog = selector_catalog();
        let rule = DenseRuleTemplate3::once(
            FrameOrientation3::FrameSet(FrameSet3::Canonical),
            DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ])])]),
            vec![LocalWriteOpTemplate3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Offset3::new(1, 0, 0),
                object: ObjectSelector3::object("Player"),
            }],
        );

        let rules = lower_dense_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 24);
        assert!(rules.iter().any(|rule| {
            rule.pattern.cells[1].offset == Direction3::RIGHT.offset
                && rule.writes
                    == vec![WriteOp3::Move {
                        from_offset: Offset3::ZERO,
                        to_offset: Direction3::RIGHT.offset,
                        object: PLAYER,
                    }]
        }));
        assert!(rules.iter().any(|rule| {
            rule.pattern.cells[1].offset == Direction3::UP.offset
                && rule.writes
                    == vec![WriteOp3::Move {
                        from_offset: Offset3::ZERO,
                        to_offset: Direction3::UP.offset,
                        object: PLAYER,
                    }]
        }));
    }

    #[test]
    fn parser_lowers_minimal_line_rule_with_direction_set_sugar() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor
}

objects {
Player actor
Box actor
Wall actor
}

group solid = Player Box Wall

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.game.layer_count, 1);
        assert_eq!(parsed.game.objects.len(), 3);
        assert_eq!(parsed.rules.len(), 4);
        assert_eq!(
            parsed.rules[1].pattern.cells[1].offset,
            Direction3::RIGHT.offset
        );
        assert_eq!(
            parsed.rules[1].writes,
            vec![WriteOp3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: ObjectId(1),
            }]
        );
    }

    #[test]
    fn parser_lowers_3d_line_gap_rules_against_level_extent() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 gap3 {
layers {
floor = Goal
actor = Player
}

rules {
right [ Player | ... | Goal ] -> [ | ... | Player Goal ]
}
}

levels3 basic of gap3 {
legend {
. = empty
P = Player
G = Goal
}

level start {
P..G
}
}
"#,
        )
        .unwrap();
        let game = &parsed.game;
        let bundle = parsed.level_bundle.as_ref().unwrap();
        let state = bundle.build_level_state(0).unwrap();

        let next = transition_program(game, &state, &parsed.rules, InputId3(0)).unwrap();

        assert!(!next.has_object(game, Coord3::new(0, 0, 0), ObjectId(2)));
        assert!(next.has_object(game, Coord3::new(3, 0, 0), ObjectId(2)));
    }

    #[test]
    fn parser_keeps_render_settings_as_model_owned_display_state() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 camera_test {
render {
          camera yaw=90 pitch=42 zoom=1.25 interactive_look interactive_zoom
          grid occupied_cells
          pixelate scale=4 smoothing
          shade
        }

layers {
actor
}

objects {
Player actor
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.settings.camera.yaw_degrees, 90);
        assert_eq!(parsed.settings.camera.pitch_degrees, 42);
        assert_eq!(parsed.settings.camera.zoom_milli, 1250);
        assert!(parsed.settings.camera.interactive_look);
        assert!(parsed.settings.camera.interactive_zoom);
        assert!(parsed.settings.grid.occupied_cells);
        assert!(parsed.settings.sprite.shade);
        assert!(parsed.settings.pixelate.enabled);
        assert_eq!(parsed.settings.pixelate.scale, 4);
        assert!(parsed.settings.pixelate.smoothing);
    }

    #[test]
    fn parser_rejects_old_boolean_render_setting_assignments() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 camera_test {
render {
  camera {
    interactive_look = true
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
        );

        assert!(parsed.is_err());
    }

    #[test]
    fn parser_defaults_render_settings() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor
}

objects {
Player actor
}
"#,
        )
        .unwrap();

        assert!(!parsed.settings.camera.interactive_look);
        assert!(!parsed.settings.camera.interactive_zoom);
        assert_eq!(parsed.settings.camera.yaw_degrees, 34);
        assert_eq!(parsed.settings.camera.pitch_degrees, 38);
        assert_eq!(parsed.settings.camera.zoom_milli, 1100);
        assert!(!parsed.settings.grid.occupied_cells);
        assert!(parsed.settings.sprite.shade);
        assert!(!parsed.settings.pixelate.enabled);
        assert_eq!(parsed.settings.pixelate.scale, 4);
        assert!(parsed.settings.pixelate.smoothing);
    }

    #[test]
    fn parser_defaults_3d_zoomscreen_height_to_full() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 viewport_test {
render {
  viewport {
    zoomscreen 7 5
    focus Player
  }
}

layers {
actor
}

objects {
Player actor
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.settings.viewport.mode, ViewportMode3::Centered);
        assert_eq!(parsed.settings.viewport.follow, ViewportFollow3::Snap);
        assert_eq!(parsed.settings.viewport.focus, "Player");
        assert_eq!(
            parsed.settings.viewport.framing,
            Some(ViewportFraming3 {
                width: 7,
                depth: 5,
                height: ViewportHeight3::Full,
            })
        );
    }

    #[test]
    fn parser_lowers_3d_local_radius_to_cubic_local_frame() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 local_radius_test {
layers {
actor
}

objects {
Player actor
Box actor
}

rules local_radius 6 {
right [ Player | Box ] -> [ | Player ]
}
}
"#,
        )
        .unwrap();

        let frame = parsed.local_frame.unwrap();
        assert_eq!(frame.x, LocalFrameExtent::Radius(6));
        assert_eq!(frame.y, LocalFrameExtent::Radius(6));
        assert_eq!(frame.z, LocalFrameExtent::Radius(6));
        assert_eq!(frame.focus_objects, vec![PLAYER]);
    }

    #[test]
    fn parser_keeps_smoothscreen_as_smooth_centered_viewport() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 viewport_test {
render {
  viewport {
    smoothscreen 9 7 3
    focus actor
  }
}

layers {
actor
}

objects {
Player actor
Box actor
}

group actor = Player Box
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.settings.viewport.mode, ViewportMode3::Centered);
        assert_eq!(parsed.settings.viewport.follow, ViewportFollow3::Smooth);
        assert_eq!(parsed.settings.viewport.focus, "actor");
        assert_eq!(
            parsed.settings.viewport.framing,
            Some(ViewportFraming3 {
                width: 9,
                depth: 7,
                height: ViewportHeight3::Size(3),
            })
        );
    }

    #[test]
    fn visual_fixture_exports_3d_viewport_contract() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 viewport_test {
render {
  viewport {
    smoothscreen 7 7
    focus actor
  }
}

layers {
actor
floor
}

objects {
Player actor
Box actor
Floor floor
}

group actor = Player Box
}

levels3 test of viewport_test {
legend {
. = empty
P = Player
}

level one {
P
}
}
"#,
        )
        .unwrap();
        let fixture = export_visual_fixture_json(&parsed).unwrap();

        assert!(fixture.contains(
            r#""viewport": { "mode": "centered", "follow": "smooth", "focus": "actor", "focusObjects": [1, 2], "framingBox": { "width": 7, "depth": 7, "height": "full" } }"#
        ));
    }

    #[test]
    fn visual_fixture_does_not_assign_implicit_sprites() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 no_sprites {
layers {
actor
}

objects {
Player actor
}
}

levels3 test of no_sprites {
legend {
P = Player
}

level one {
P
}
}
"#,
        )
        .unwrap();
        let fixture = export_visual_fixture_json(&parsed).unwrap();

        assert!(fixture.contains(r#""Player": { "id": 1, "name": "Player", "sprite": null"#));
        assert!(fixture.contains(r#"{ "id": 1, "name": "Player", "sprite": null }"#));
    }

    #[test]
    fn parser_rejects_legacy_top_level_camera_settings() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 camera_test {
debug_camera = true
camera_yaw = 90
camera_pitch = 42
camera_zoom = 1.25

layers {
actor
}

objects {
Player actor
}
}
"#,
        );

        assert!(parsed.is_err());
    }

    #[test]
    fn parser_lowers_input_guarded_direction_set_rule() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor = Player Box Wall
}

group solid = Player Box Wall

rules {
input horizontal [ Player | no solid ] -> [ | Player ]
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.rules.len(), 4);
        assert_eq!(parsed.rules[0].guards, vec![Guard3::InputIs(INPUT_LEFT)]);
        assert_eq!(parsed.rules[1].guards, vec![Guard3::InputIs(INPUT_RIGHT)]);
        assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
        assert_eq!(
            parsed.rules[3].guards,
            vec![Guard3::InputIs(INPUT_BACKWARD)]
        );
    }

    #[test]
    fn parser_lowers_input_rule_without_set_as_directions_sugar() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor = Player Wall
}

group solid = Player Wall

rules {
input [ Player | no solid ] -> [ | Player ]
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.rules.len(), 6);
        let guards = parsed
            .rules
            .iter()
            .map(|rule| rule.guards.as_slice())
            .collect::<Vec<_>>();
        for input in [
            INPUT_LEFT,
            INPUT_RIGHT,
            INPUT_UP,
            InputId3(3),
            INPUT_FORWARD,
            INPUT_BACKWARD,
        ] {
            assert!(guards.contains(&[Guard3::InputIs(input)].as_slice()));
        }
    }

    #[test]
    fn parser_lowers_camera_variable_effects_from_rules() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor = Player Wall
}

rules {
right [ Player | no Wall ] -> [ | Player ] set yaw = 100
set zoom = 1.5
reset_camera
}

levels3 test {
legend {
P = Player
}

level one {
P
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.rules.len(), 3);
        assert_eq!(
            parsed.rules[0].effects,
            vec![RuleEffect3::SetCameraYaw(100)]
        );
        assert_eq!(
            parsed.rules[1].effects,
            vec![RuleEffect3::SetCameraZoom(1500)]
        );
        assert!(parsed.rules[1].pattern.cells.is_empty());
        assert!(parsed.rules[1].writes.is_empty());
        assert_eq!(parsed.rules[2].effects, vec![RuleEffect3::ResetCamera]);

        let fixture = export_visual_fixture_json(&parsed).unwrap();
        assert!(fixture.contains(r#""effects": ["#));
        assert!(fixture.contains(r#""kind": "set_camera""#));
        assert!(fixture.contains(r#""variable": "yaw""#));
        assert!(fixture.contains(r#""kind": "reset_camera""#));
    }

    #[test]
    fn parser_lowers_variant_selector_assignment() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor
}

objects {
Marker:directions actor
}

rules {
directions [ Marker:* | ] -> [ | Marker:* ]
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.game.objects.len(), 6);
        assert_eq!(parsed.rules.len(), 36);
        assert!(parsed.rules.iter().all(|rule| {
            rule.pattern.cells[0].require_objects[0]
                == match rule.writes[0] {
                    WriteOp3::Move { object, .. } => object,
                    _ => ObjectId::EMPTY,
                }
        }));
    }

    #[test]
    fn parser_lowers_group_selector_to_runtime_object_set_matcher() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 group_move {
layers {
actor
}

objects {
Box actor
Crate actor
}

group solid = Box Crate

rules {
right [ solid | ] -> [ | solid ]
}
}

levels3 basic of group_move {
legend {
. = empty
B = Box
C = Crate
}

level start {
B.C
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.rules.len(), 1);
        let rule = &parsed.rules[0];
        assert!(rule.pattern.cells[0].require_objects.is_empty());
        assert_eq!(rule.pattern.cells[0].require_object_sets.len(), 1);
        assert_eq!(
            rule.pattern.cells[0].require_object_sets[0].objects,
            vec![ObjectId(1), ObjectId(2)]
        );
        assert!(matches!(
            rule.writes.as_slice(),
            [WriteOp3::MoveObjectSet { binding: 0, .. }]
        ));

        let state = parsed
            .level_bundle
            .as_ref()
            .unwrap()
            .build_level_state(0)
            .unwrap();
        let next = transition_program_without_input(&parsed.game, &state, &parsed.rules).unwrap();
        assert!(!next.has_object(&parsed.game, Coord3::new(0, 0, 0), ObjectId(1)));
        assert!(next.has_object(&parsed.game, Coord3::new(1, 0, 0), ObjectId(1)));
    }

    #[test]
    fn parser_lowers_selector_occurrence_labels_for_group_swap() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor = Box Crate
}

group solid = Box Crate

rules {
right [ solid#1 | solid#2 ] -> [ solid#2 | solid#1 ]
}
"#,
        )
        .unwrap();

        assert!(parsed.rules.iter().any(|rule| {
            rule.pattern.cells[0].require_objects == vec![ObjectId(1)]
                && rule.pattern.cells[1].require_objects == vec![ObjectId(2)]
                && rule.writes.contains(&WriteOp3::Move {
                    from_offset: Offset3::ZERO,
                    to_offset: Direction3::RIGHT.offset,
                    object: ObjectId(1),
                })
                && rule.writes.contains(&WriteOp3::Move {
                    from_offset: Direction3::RIGHT.offset,
                    to_offset: Offset3::ZERO,
                    object: ObjectId(2),
                })
        }));
    }

    #[test]
    fn parser_rejects_duplicate_selector_occurrence_labels() {
        let err = parse_puzzle3d(
            r#"
layers {
actor = Box Crate
}

group solid = Box Crate

rules {
right [ solid#1 | solid#1 ] -> [ solid#1 | solid#1 ]
}
"#,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ParseError3::Message(message)
                if message.contains("DuplicateSelectorOccurrenceLabel")
        ));
    }

    #[test]
    fn parser_lowers_dense_frame_rule() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor
}

objects {
Player actor
Box actor
}

rules {
right:up [ Player | Box ] -> [ | Player | Box ]
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(
            parsed.rules[0].pattern.cells[1].offset,
            Direction3::RIGHT.offset
        );
        assert_eq!(
            parsed.rules[0].writes[0],
            WriteOp3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: ObjectId(1),
            }
        );
    }

    #[test]
    fn parser_lowers_layers_legend_and_levels_to_level_bundle() {
        let parsed = parse_puzzle3d(
            r#"
layers {
floor = Goal
actor = Player Box Wall
}

group solid = Player Box Wall

rules {
horizontal [ Player | no solid ] -> [ | Player ]
}

levels3 {
legend {
P = Player
B = Box
# = Wall
G = Goal
* = Goal Box
}

level stacked {
...
.G.
...

###
#PB
###
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.game.layer_count, 2);
        assert_eq!(parsed.game.objects.len(), 4);

        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        assert_eq!(bundle.level_count(), 1);
        assert_eq!(bundle.level(0).unwrap().name, "stacked");
        assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(3, 3, 2));

        let state = bundle.build_level_state(0).unwrap();

        assert!(state.has_object(&bundle.game, Coord3::new(1, 1, 1), ObjectId(1)));
        assert!(state.has_object(&bundle.game, Coord3::new(1, 1, 0), ObjectId(2)));
        assert!(state.has_object(&bundle.game, Coord3::new(2, 1, 0), ObjectId(3)));
        assert!(state.has_object(&bundle.game, Coord3::new(0, 2, 0), ObjectId(4)));
    }

    #[test]
    fn parser_uses_dot_as_default_empty_char_for_3d_levels() {
        let parsed = parse_puzzle3d(
            r#"
layers {
actor = Player
}

levels3 {
legend {
P = Player
}

level default_dot {
P.
}
}
"#,
        )
        .unwrap();

        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        let state = bundle.build_level_state(0).unwrap();

        assert!(state.has_object(&bundle.game, Coord3::new(0, 0, 0), ObjectId(1)));
        assert_eq!(
            state.cell_view(Coord3::new(1, 0, 0)).unwrap().objects,
            Vec::<ObjectId>::new()
        );
    }

    #[test]
    fn parser_rejects_non_dot_empty_char_for_3d_levels() {
        let err = parse_puzzle3d(
            r#"
layers {
actor = Player
}

levels3 {
legend {
_ = empty
P = Player
}

level override_empty {
P.
}
}
"#,
        )
        .unwrap_err();

        assert!(
            matches!(err, ParseError3::Message(message) if message.contains("3D levels use `.` for empty"))
        );
    }

    #[test]
    fn parser_lowers_canonical_sprites3_entries() {
        let parsed = parse_puzzle3d(
            r##"
layers {
floor = Floor
}

sprites3 basic {
Floor
#90ee90 #008000 transparent
.....
..1..
.....

00000
0...0
00000
}
"##,
        )
        .unwrap();

        let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
        let floor = sprites.sprite("Floor").expect("Floor sprite exists");

        assert_eq!(sprites.name, "basic");
        assert_eq!(
            floor.palette.get(&'0'),
            Some(&SpriteColor3::Hex("#90ee90".to_string()))
        );
        assert_eq!(
            floor.palette.get(&'1'),
            Some(&SpriteColor3::Hex("#008000".to_string()))
        );
        assert_eq!(floor.palette.get(&'2'), Some(&SpriteColor3::Transparent));
        assert_eq!(floor.voxels.size, Size3::new(5, 3, 2));
    }

    #[test]
    fn parser_lowers_canonical_sprites3_shape_refs() {
        let parsed = parse_puzzle3d(
            r##"
layers {
floor = Floor
}

sprites3 basic {
shape flat {
.....
..1..
.....

00000
0...0
00000
}

Floor
#90ee90 #008000
flat
}
"##,
        )
        .unwrap();

        let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
        let floor = sprites.sprite("Floor").expect("Floor sprite exists");

        assert_eq!(
            floor.palette.get(&'0'),
            Some(&SpriteColor3::Hex("#90ee90".to_string()))
        );
        assert_eq!(
            floor.palette.get(&'1'),
            Some(&SpriteColor3::Hex("#008000".to_string()))
        );
        assert_eq!(floor.voxels.size, Size3::new(5, 3, 2));
    }

    #[test]
    fn parser_lowers_color_only_sprites3_entry_to_filled_cube() {
        let parsed = parse_puzzle3d(
            r##"
layers {
floor = Floor
target = Goal
}

sprites3 basic {
Floor
#90ee90

Goal
#00008b
}
"##,
        )
        .unwrap();

        let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
        let floor = sprites.sprite("Floor").expect("Floor sprite exists");
        let goal = sprites.sprite("Goal").expect("Goal sprite exists");

        assert_eq!(floor.voxels.size, Size3::new(1, 1, 1));
        assert_eq!(floor.voxels.slices.as_slice(), &[vec!["0".to_string()]]);
        assert_eq!(goal.voxels.size, Size3::new(1, 1, 1));
    }

    #[test]
    fn parser_rejects_prefixed_sprites3_shape_refs() {
        let err = parse_puzzle3d(
            r##"
layers {
floor = Floor
}

sprites3 basic {
shape flat {
0
}

Floor
#90ee90
shape flat
}
"##,
        )
        .unwrap_err();

        assert!(
            matches!(err, ParseError3::Message(message) if message.contains("shape refs are bare"))
        );
    }

    #[test]
    fn parser_rejects_legacy_sprites3_blocks() {
        let err = parse_puzzle3d(
            r##"
layers {
floor = Floor
}

sprites3 basic {
sprite Floor {
colors {
0 = #90ee90
}

voxels {
0
}
}
}
"##,
        )
        .unwrap_err();

        assert!(matches!(err, ParseError3::Message(message) if message.contains("canonical form")));
    }

    #[test]
    fn parser_rejects_unknown_level_legend_char() {
        let err = parse_puzzle3d(
            r#"
layers {
actor = Player
}

levels3 {
legend {
. = empty
P = Player
}

level bad {
PX
}
}
"#,
        )
        .unwrap_err();

        assert!(
            matches!(err, ParseError3::Message(message) if message.contains("unknown legend char: X"))
        );
    }

    #[test]
    fn parser_lowers_model_wrapped_win_conditions_and_named_level_pack() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Player Box
}

win_conditions {
some Goal
no down [ no Box | Goal ]
}
}

levels3 basic of push3d {
legend {
. = empty
G = Goal
B = Box
}

level solved {
...
.B.
...

...
.G.
...
}

level unsolved {
...
..B
...

...
.G.
...
}
}

"#,
        )
        .unwrap();

        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        let win = parsed.win_condition.as_ref().expect("win condition exists");

        assert_eq!(bundle.level_count(), 2);

        let solved = bundle.build_level_state(0).unwrap();
        let unsolved = bundle.build_level_state(1).unwrap();

        assert!(win.is_met(&bundle.game, &solved));
        assert!(!win.is_met(&bundle.game, &unsolved));
    }

    #[test]
    fn parser_rejects_all_on_oriented_win_pattern() {
        let err = parse_puzzle3d(
            r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Box
}

win_conditions {
some Goal
all Goal on down [ Box | Goal ]
}
}
"#,
        )
        .unwrap_err();

        assert!(
            matches!(err, ParseError3::Message(message) if message.contains("all <selector> on <pattern> is not valid"))
        );
    }

    #[test]
    fn parser_accepts_function_style_3d_win_conditions() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 push3d {
layers {
floor = Goal
actor = Box
}

win_conditions {
exists(Goal)
none(down [ no Box | Goal ])
}
}

levels3 basic of push3d {
legend {
. = empty
G = Goal
B = Box
}

level solved {
...
.B.
...

...
.G.
...
}
}
"#,
        )
        .unwrap();

        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        let win = parsed.win_condition.as_ref().expect("win condition exists");
        let solved = bundle.build_level_state(0).unwrap();

        assert!(win.is_met(&bundle.game, &solved));
    }

    #[test]
    fn parser_rejects_2d_model_keyword_in_3d_parser() {
        let err = parse_puzzle3d(
            r#"
puzzle push3d {
layers {
actor = Player
}
}
"#,
        )
        .unwrap_err();

        assert!(
            matches!(err, ParseError3::Message(message) if message.contains("unknown 3D puzzle directive"))
        );
    }

    #[test]
    fn sokoban_literally_in_3d_recreates_microban_level_1() {
        let parsed =
            parse_puzzle3d(include_str!("../games/sokoban_literally_in_3d.puzzle")).unwrap();
        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        let win = parsed.win_condition.as_ref().expect("win condition exists");

        assert_eq!(bundle.level_count(), 3);
        assert_eq!(bundle.level(0).unwrap().name, "microban_01");
        assert_eq!(bundle.level(1).unwrap().name, "microban_02");
        assert_eq!(bundle.level(2).unwrap().name, "microban_03");
        assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(6, 7, 2));
        assert_eq!(bundle.level(1).unwrap().level.size, Size3::new(6, 7, 2));
        assert_eq!(bundle.level(2).unwrap().level.size, Size3::new(6, 7, 2));
        assert_eq!(
            parsed.lifecycle.on_level_clear,
            vec![LifecycleCommand3::NextLevel]
        );
        let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");
        assert_eq!(sprites.name, "basic");
        assert_eq!(sprites.model.as_deref(), Some("sokoban_literally_in_3d"));
        assert_eq!(sprites.sprites.len(), 5);
        assert_eq!(
            sprites.sprite("Floor").unwrap().voxels.size,
            Size3::new(5, 5, 5)
        );
        assert_eq!(
            sprites.sprite("Box").unwrap().voxels.size,
            Size3::new(5, 5, 5)
        );
        assert_eq!(
            sprites.sprite("Player").unwrap().voxels.size,
            Size3::new(5, 5, 5)
        );
        assert_eq!(
            sprites.sprite("Wall").unwrap().voxels.size,
            Size3::new(5, 5, 3)
        );
        let fixture_json = export_visual_fixture_json(&parsed).unwrap();
        assert!(fixture_json.contains("\"title\": \"Sokoban Literally in 3D\""));
        assert!(fixture_json.contains("\"grid\": { \"visibility\": 1, \"occupied_cells\": true }"));
        assert!(fixture_json.contains("\"shade\": true"));
        assert!(
            fixture_json.contains(
                "\"pixelate\": { \"enabled\": false, \"scale\": 4, \"smoothing\": true }"
            )
        );
        assert!(fixture_json.contains("\"rules\": ["));
        assert!(fixture_json.contains("\"onLevelClear\": [\"next_level\"]"));
        assert!(fixture_json.contains("\"kind\": \"no_pattern\""));
        assert!(fixture_json.contains("\"Box\": {"));
        assert!(fixture_json.contains("\"bitmap\": ["));

        let initial = bundle.build_level_state(0).unwrap();
        let snapshot = BoardSnapshot3::from_state(&initial);
        let floor_cells = snapshot
            .cells
            .iter()
            .filter(|cell| cell.position.z == 0 && cell.objects.contains(&ObjectId(1)))
            .count();
        assert_eq!(floor_cells, 42);
        assert!(initial.has_object(&bundle.game, Coord3::new(0, 0, 0), ObjectId(1)));
        assert!(initial.has_object(&bundle.game, Coord3::new(5, 0, 0), ObjectId(1)));
        assert!(initial.has_object(&bundle.game, Coord3::new(2, 3, 0), ObjectId(1)));
        assert!(initial.has_object(&bundle.game, Coord3::new(2, 5, 0), ObjectId(2)));
        assert!(initial.has_object(&bundle.game, Coord3::new(1, 3, 0), ObjectId(2)));
        assert!(initial.has_object(&bundle.game, Coord3::new(2, 3, 1), ObjectId(3)));
        assert!(initial.has_object(&bundle.game, Coord3::new(1, 3, 1), ObjectId(4)));
        assert!(initial.has_object(&bundle.game, Coord3::new(3, 2, 1), ObjectId(4)));
        assert!(!win.is_met(&bundle.game, &initial));

        let second_initial = bundle.build_level_state(1).unwrap();
        assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 4, 1), ObjectId(3)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(2, 3, 1), ObjectId(4)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 3, 1), ObjectId(4)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 2, 1), ObjectId(4)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 3, 0), ObjectId(2)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(2, 2, 0), ObjectId(2)));
        assert!(second_initial.has_object(&bundle.game, Coord3::new(3, 2, 0), ObjectId(2)));

        let mut session = GameSession3::new(bundle).unwrap();
        let solution = [
            Direction3::BACKWARD,
            Direction3::LEFT,
            Direction3::FORWARD,
            Direction3::RIGHT,
            Direction3::RIGHT,
            Direction3::RIGHT,
            Direction3::BACKWARD,
            Direction3::LEFT,
            Direction3::FORWARD,
            Direction3::LEFT,
            Direction3::LEFT,
            Direction3::BACKWARD,
            Direction3::BACKWARD,
            Direction3::RIGHT,
            Direction3::FORWARD,
            Direction3::LEFT,
            Direction3::FORWARD,
            Direction3::RIGHT,
            Direction3::FORWARD,
            Direction3::FORWARD,
            Direction3::LEFT,
            Direction3::BACKWARD,
            Direction3::RIGHT,
            Direction3::BACKWARD,
            Direction3::BACKWARD,
            Direction3::RIGHT,
            Direction3::RIGHT,
            Direction3::FORWARD,
            Direction3::LEFT,
            Direction3::BACKWARD,
            Direction3::LEFT,
            Direction3::FORWARD,
            Direction3::FORWARD,
        ];
        for direction in solution {
            assert!(
                session
                    .move_direction_with_win_condition(bundle, &parsed.rules, direction, win)
                    .unwrap()
            );
        }

        assert!(session.completed());
        assert_eq!(session.move_count(), 33);
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(2, 5, 1), ObjectId(4))
        );
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(1, 3, 1), ObjectId(4))
        );

        assert!(session.has_next_level(bundle));
        assert!(session.next_level(bundle).unwrap());
        assert_eq!(session.current_level_index(), 1);
        assert_eq!(session.move_count(), 0);
        assert!(!session.completed());
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(3, 4, 1), ObjectId(3))
        );
        assert!(session.has_next_level(bundle));
        assert!(session.next_level(bundle).unwrap());
        assert_eq!(session.current_level_index(), 2);
        assert!(
            session
                .state()
                .has_object(&bundle.game, Coord3::new(2, 4, 1), ObjectId(3))
        );
        assert!(!session.has_next_level(bundle));

        let mut lifecycle_session = GameSession3::new_with_lifecycle(bundle, &parsed.lifecycle)
            .expect("lifecycle session starts");
        let mut last_result = SessionLifecycleResult3::default();
        for direction in solution {
            last_result = lifecycle_session
                .apply_input_with_lifecycle(
                    bundle,
                    &parsed.rules,
                    input_for_microban_offset(direction.offset),
                    win,
                    &parsed.lifecycle,
                )
                .unwrap();
        }
        assert!(last_result.cleared);
        assert!(last_result.level_changed);
        assert_eq!(lifecycle_session.current_level_index(), 1);
        assert_eq!(lifecycle_session.move_count(), 0);
        assert!(!lifecycle_session.completed());
    }

    #[test]
    fn handmade_3d_sokoban_can_be_authored_from_puzzle_file() {
        let parsed =
            parse_puzzle3d(include_str!("../games/from_puzzle_sokoban_3d.puzzle")).unwrap();
        let bundle = parsed.level_bundle.as_ref().expect("level bundle exists");
        let win = parsed.win_condition.as_ref().expect("win condition exists");
        let sprites = parsed.sprite_set.as_ref().expect("sprite set exists");

        assert_eq!(bundle.level_count(), 2);
        assert_eq!(bundle.level(0).unwrap().name, "push_once");
        assert_eq!(bundle.level(1).unwrap().name, "corner_lift");
        assert_eq!(bundle.level(0).unwrap().level.size, Size3::new(5, 5, 2));
        assert_eq!(
            sprites.sprite("Floor").unwrap().voxels.size,
            Size3::new(3, 3, 1)
        );
        assert_eq!(
            sprites.sprite("Box").unwrap().voxels.size,
            Size3::new(2, 2, 2)
        );

        let fixture_json = export_visual_fixture_json(&parsed).unwrap();
        assert!(fixture_json.contains("\"title\": \"From Puzzle Sokoban 3D\""));
        assert!(fixture_json.contains("\"currentScene\": \"playing\""));
        assert!(fixture_json.contains("\"kind\": \"puzzle3\""));
        assert!(fixture_json.contains("\"levels\""));

        let mut session =
            GameSession3::new_with_lifecycle(bundle, &parsed.lifecycle).expect("session starts");
        let first_clear = session
            .apply_input_with_lifecycle(
                bundle,
                &parsed.rules,
                input_for_microban_offset(Direction3::RIGHT.offset),
                win,
                &parsed.lifecycle,
            )
            .unwrap();
        assert!(first_clear.cleared);
        assert!(first_clear.level_changed);
        assert_eq!(session.current_level_index(), 1);

        let second_clear = session
            .apply_input_with_lifecycle(
                bundle,
                &parsed.rules,
                input_for_microban_offset(Direction3::FORWARD.offset),
                win,
                &parsed.lifecycle,
            )
            .unwrap();
        assert!(second_clear.cleared);
        assert!(!second_clear.level_changed);
        assert!(session.completed());
    }

    #[test]
    fn microban_basic_01_is_a_single_layer_3d_level() {
        let parsed = microban_basic_model();
        let rules = microban_basic_rules_with_input_guards(&parsed.rules);
        let level = microban_basic_01_level();

        assert_eq!(level.size, Size3::new(6, 7, 1));

        let state = level.build_state(&parsed.game).unwrap();
        let snapshot = BoardSnapshot3::from_state(&state);

        assert_eq!(snapshot.size, Size3::new(6, 7, 1));
        assert!(snapshot.cells.iter().all(|cell| cell.position.z == 0));
        assert_eq!(snapshot.cells.len(), 42);
        assert!(state.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_GOAL));
        assert!(state.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_BOX));
        assert!(state.has_object(&parsed.game, Coord3::new(2, 3, 0), MICROBAN_PLAYER));
        assert!(state.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_BOX));

        let moved_down = transition_program(&parsed.game, &state, &rules, INPUT_FORWARD).unwrap();
        let pushed_right =
            transition_program(&parsed.game, &moved_down, &rules, INPUT_RIGHT).unwrap();

        assert!(pushed_right.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_PLAYER));
        assert!(pushed_right.has_object(&parsed.game, Coord3::new(4, 4, 0), MICROBAN_BOX));
        assert!(!pushed_right.has_object(&parsed.game, Coord3::new(3, 4, 0), MICROBAN_BOX));
        assert!(pushed_right.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_GOAL));
        assert!(pushed_right.has_object(&parsed.game, Coord3::new(1, 3, 0), MICROBAN_BOX));
    }

    #[test]
    fn microban_basic_sprites_are_flat_bottom_5x5x1_voxel_slices() {
        let sprites = microban_basic_sprites();

        assert_eq!(sprites.len(), 5);
        for sprite in &sprites {
            assert_eq!(sprite.size, Size3::new(5, 5, 5));
            assert_eq!(sprite.layers.len(), 5);
            assert!(sprite.layers.iter().all(|layer| layer.len() == 5));
            assert!(
                sprite
                    .layers
                    .iter()
                    .all(|layer| layer.iter().all(|row| row.chars().count() == 5))
            );
            assert!(
                sprite.layers[1..]
                    .iter()
                    .flatten()
                    .all(|row| *row == ".....")
            );
        }

        let player = sprites
            .iter()
            .find(|sprite| sprite.name == "Player")
            .expect("Microban Basic player sprite exists");
        assert_eq!(
            player.layers[0],
            vec![".000.", ".111.", "22222", ".333.", ".3.3."]
        );
        assert_eq!(
            player.palette,
            vec!["#000000", "#ffa500", "#ffffff", "#0000ff"]
        );

        let goal = sprites
            .iter()
            .find(|sprite| sprite.name == "Goal")
            .expect("Microban Basic goal sprite exists");
        assert_eq!(goal.filled_voxel_count(), 8);
    }

    #[test]
    fn semantic_inputs_can_resolve_to_absolute_directions() {
        let game = game();

        assert_eq!(
            game.input_by_name("right").map(|input| input.id),
            Some(INPUT_RIGHT)
        );
        assert_eq!(
            game.direction_for_input(INPUT_RIGHT),
            Some(Direction3::RIGHT)
        );
        assert_eq!(game.direction_for_input(InputId3(6)), None);
    }

    #[test]
    fn parser_accepts_owner_scoped_inputs_for_3d_models() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 scoped_inputs {
layers {
solid = Player
}

inputs {
right <- d ArrowRight
restart <- r
}

rules {
input right [ Player | ] -> [ | Player ]
}
}
"#,
        )
        .unwrap();

        assert_eq!(parsed.game.inputs.len(), 2);
        let right = parsed.game.input_by_name("right").unwrap();
        assert_eq!(right.id, INPUT_RIGHT);
        assert_eq!(right.direction, Some(Direction3::RIGHT));
        assert_eq!(right.keys, vec!["d", "ArrowRight"]);
        let restart = parsed.game.input_by_name("restart").unwrap();
        assert_eq!(restart.direction, None);
        assert_eq!(restart.keys, vec!["r"]);
    }

    #[test]
    fn parser_accepts_front_back_as_canonical_3d_directions() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 front_back {
layers {
solid = Player
}

inputs {
front <- w ArrowUp
back <- s ArrowDown
}

rules {
front [ Player | ] -> [ | Player ]
back [ Player | ] -> [ | Player ]
input front [ Player | ] -> [ | Player ]
input back [ Player | ] -> [ | Player ]
}
}
"#,
        )
        .unwrap();

        assert_eq!(
            parsed.game.input_by_name("front").map(|input| input.id),
            Some(INPUT_FORWARD)
        );
        assert_eq!(
            parsed.game.input_by_name("back").map(|input| input.id),
            Some(INPUT_BACKWARD)
        );
        assert_eq!(parsed.rules.len(), 4);
        assert_eq!(parsed.rules[0].pattern.cells[0].offset, Offset3::ZERO);
        assert_eq!(
            parsed.rules[0].pattern.cells[1].offset,
            Direction3::FORWARD.offset
        );
        assert_eq!(
            parsed.rules[1].pattern.cells[1].offset,
            Direction3::BACKWARD.offset
        );
        assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
        assert_eq!(
            parsed.rules[3].guards,
            vec![Guard3::InputIs(INPUT_BACKWARD)]
        );
    }

    #[test]
    fn parser_keeps_forward_backward_as_3d_direction_aliases() {
        let parsed = parse_puzzle3d(
            r#"
puzzle3 legacy_forward_backward {
layers {
solid = Player
}

inputs {
forward <- w ArrowUp
backward <- s ArrowDown
}

rules {
forward [ Player | ] -> [ | Player ]
backward [ Player | ] -> [ | Player ]
input forward [ Player | ] -> [ | Player ]
input backward [ Player | ] -> [ | Player ]
}
}
"#,
        )
        .unwrap();

        assert!(parsed.game.input_by_name("forward").is_none());
        assert!(parsed.game.input_by_name("backward").is_none());
        assert_eq!(
            parsed
                .game
                .input_by_name("front")
                .map(|input| input.keys.clone()),
            Some(vec!["w".to_string(), "ArrowUp".to_string()])
        );
        assert_eq!(
            parsed
                .game
                .input_by_name("back")
                .map(|input| input.keys.clone()),
            Some(vec!["s".to_string(), "ArrowDown".to_string()])
        );
        assert_eq!(parsed.rules.len(), 4);
        assert_eq!(
            parsed.rules[0].pattern.cells[1].offset,
            Direction3::FORWARD.offset
        );
        assert_eq!(
            parsed.rules[1].pattern.cells[1].offset,
            Direction3::BACKWARD.offset
        );
        assert_eq!(parsed.rules[2].guards, vec![Guard3::InputIs(INPUT_FORWARD)]);
        assert_eq!(
            parsed.rules[3].guards,
            vec![Guard3::InputIs(INPUT_BACKWARD)]
        );
    }

    #[test]
    fn game_validation_accepts_well_formed_definitions() {
        let game = Game3::checked_new_with_inputs(
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
            vec![
                InputDef3::directional(INPUT_LEFT, "left", Direction3::LEFT),
                InputDef3::action(InputId3(6), "restart"),
            ],
        )
        .unwrap();

        assert_eq!(game.layer_count, 2);
        assert_eq!(game.objects.len(), 2);
        assert_eq!(game.inputs.len(), 2);
    }

    #[test]
    fn game_validation_rejects_zero_layers() {
        let err = Game3::checked_new(0, Vec::new()).unwrap_err();

        assert_eq!(err, GameError3::InvalidLayerCount);
    }

    #[test]
    fn game_validation_rejects_empty_object_id() {
        let err = Game3::checked_new(
            1,
            vec![ObjectDef3 {
                id: ObjectId::EMPTY,
                layer_id: ACTOR,
            }],
        )
        .unwrap_err();

        assert_eq!(err, GameError3::EmptyObjectId);
    }

    #[test]
    fn game_validation_rejects_duplicate_object_ids() {
        let err = Game3::checked_new(
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

        assert_eq!(err, GameError3::DuplicateObjectId { object: PLAYER });
    }

    #[test]
    fn game_validation_rejects_object_layer_outside_layer_count() {
        let err = Game3::checked_new(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: FLOOR,
            }],
        )
        .unwrap_err();

        assert_eq!(
            err,
            GameError3::ObjectLayerOutOfBounds {
                object: PLAYER,
                layer: FLOOR,
            }
        );
    }

    #[test]
    fn game_validation_rejects_duplicate_input_ids() {
        let err = Game3::checked_new_with_inputs(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: ACTOR,
            }],
            vec![
                InputDef3::directional(INPUT_LEFT, "left", Direction3::LEFT),
                InputDef3::directional(INPUT_LEFT, "west", Direction3::LEFT),
            ],
        )
        .unwrap_err();

        assert_eq!(err, GameError3::DuplicateInputId { input: INPUT_LEFT });
    }

    #[test]
    fn game_validation_rejects_duplicate_input_names() {
        let err = Game3::checked_new_with_inputs(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: ACTOR,
            }],
            vec![
                InputDef3::directional(INPUT_LEFT, "left", Direction3::LEFT),
                InputDef3::directional(INPUT_RIGHT, "left", Direction3::RIGHT),
            ],
        )
        .unwrap_err();

        assert_eq!(
            err,
            GameError3::DuplicateInputName {
                name: "left".to_string(),
            }
        );
    }

    #[test]
    fn state_uses_z_y_x_slot_order_with_layers_inside_cells() {
        let game = game();
        let mut state = empty_state(2, 2, 2);
        state
            .place_object(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(0, 1, 1), BOX)
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

        assert!(state.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(state.has_object(&game, Coord3::new(2, 1, 1), BOX));
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
                LevelEntry3::new("microban_01", first),
                LevelEntry3::new("microban_02", second),
            ],
        )
        .unwrap();

        assert_eq!(bundle.level_count(), 2);
        assert_eq!(bundle.level_by_name("microban_02").unwrap().0, 1);

        let state = bundle.build_level_state(1).unwrap();

        assert!(state.has_object(&game, Coord3::new(1, 0, 0), BOX));
    }

    #[test]
    fn level_bundle_rejects_empty_level_list() {
        let err = LevelBundle3::checked_new(game(), Vec::new()).unwrap_err();

        assert_eq!(err, LevelBundleError3::EmptyLevels);
    }

    #[test]
    fn level_bundle_rejects_empty_level_name() {
        let level = Level3::new(Size3::new(1, 1, 1), Vec::new());
        let err = LevelBundle3::checked_new(game(), vec![LevelEntry3::new("", level)]).unwrap_err();

        assert_eq!(err, LevelBundleError3::EmptyLevelName { index: 0 });
    }

    #[test]
    fn level_bundle_rejects_duplicate_level_names() {
        let level = Level3::new(Size3::new(1, 1, 1), Vec::new());
        let err = LevelBundle3::checked_new(
            game(),
            vec![
                LevelEntry3::new("microban_01", level.clone()),
                LevelEntry3::new("microban_01", level),
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
            LevelBundle3::checked_new(game(), vec![LevelEntry3::new("bad", level)]).unwrap_err();

        assert_eq!(
            err,
            LevelBundleError3::Level {
                index: 0,
                name: "bad".to_string(),
                source: LevelError3::State(StateError3::PositionOutOfBounds {
                    position: Coord3::new(2, 0, 0),
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

        assert!(state.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(state.has_object(&game, Coord3::new(0, 0, 0), GOAL));
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
                position: Coord3::new(0, 0, 0),
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
                position: Coord3::new(2, 0, 0),
            })
        );
    }

    #[test]
    fn level_rejects_object_layer_outside_game_layer_count() {
        let game = Game3::new(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: FLOOR,
            }],
        );
        let level = Level3::new(
            Size3::new(2, 1, 1),
            vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER])],
        );

        let err = level.build_state(&game).unwrap_err();

        assert_eq!(
            err,
            LevelError3::Game(GameError3::ObjectLayerOutOfBounds {
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
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), BOX)
            .unwrap();

        let next = transition_once(&game, &state, &push_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(2, 0, 0), BOX));
    }

    #[test]
    fn transition_pushes_forward_in_y_axis() {
        let game = game();
        let mut state = empty_state(1, 3, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(0, 1, 0), BOX)
            .unwrap();

        let next = transition_once(&game, &state, &push_rule(Direction3::FORWARD)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(0, 1, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(0, 2, 0), BOX));
    }

    #[test]
    fn vertical_movement_uses_z_axis() {
        let game = game();
        let mut state = empty_state(1, 1, 2);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::UP)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(0, 0, 1), PLAYER));
    }

    #[test]
    fn transition_does_not_match_out_of_bounds_target() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::DOWN)).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn transition_does_not_match_upper_out_of_bounds_for_forbid_only_cell() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_once(&game, &state, &move_rule(Direction3::RIGHT)).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn program_applies_only_rules_guarded_by_current_input() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();

        let rules = vec![
            move_rule(Direction3::LEFT).when_input(INPUT_LEFT),
            move_rule(Direction3::RIGHT).when_input(INPUT_RIGHT),
        ];

        let next = transition_program(&game, &state, &rules, INPUT_LEFT).unwrap();

        assert!(!next.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
    }

    #[test]
    fn until_stable_repeats_sweeps_until_state_stops_changing() {
        let game = game();
        let mut state = empty_state(4, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next = transition_repeated(&game, &state, &move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(3, 0, 0), PLAYER));
    }

    #[test]
    fn until_stable_finishes_when_a_3d_rule_leaves_state_unchanged() {
        let game = game();
        let mut state = empty_state(1, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        let no_progress = Rule3::repeated(
            Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(PLAYER)]),
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
                remove: PLAYER,
                add: PLAYER,
            }],
        );

        let next = transition_repeated(&game, &state, &no_progress).unwrap();

        assert_eq!(next, state);
    }

    #[test]
    fn once_all_applies_each_initial_3d_match_once() {
        let game = game();
        let mut state = empty_state(5, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(3, 0, 0), PLAYER)
            .unwrap();

        let next =
            transition_once_all(&game, &state, &once_all_move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(!next.has_object(&game, Coord3::new(2, 0, 0), PLAYER));
        assert!(!next.has_object(&game, Coord3::new(3, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(4, 0, 0), PLAYER));
    }

    #[test]
    fn once_all_does_not_chain_into_3d_matches_created_during_sweep() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let next =
            transition_once_all(&game, &state, &once_all_move_rule(Direction3::RIGHT)).unwrap();

        assert!(!next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(next.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(!next.has_object(&game, Coord3::new(2, 0, 0), PLAYER));
    }

    #[test]
    fn once_all_skips_3d_matches_invalidated_during_sweep() {
        let game = game();
        let mut state = empty_state(3, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(2, 0, 0), PLAYER)
            .unwrap();

        let consume_pair = Rule3::once_all(
            Pattern3::new(vec![
                MatchCell3::new(Offset3::ZERO).require(PLAYER),
                MatchCell3::new(Direction3::RIGHT.offset).require(PLAYER),
            ]),
            vec![
                WriteOp3::Replace {
                    offset: Offset3::ZERO,
                    remove: PLAYER,
                    add: BOX,
                },
                WriteOp3::Remove {
                    offset: Direction3::RIGHT.offset,
                    object: PLAYER,
                },
            ],
        );

        let next = transition_once_all(&game, &state, &consume_pair).unwrap();

        assert!(next.has_object(&game, Coord3::new(0, 0, 0), BOX));
        assert_eq!(
            next.get_layer(Coord3::new(1, 0, 0), ACTOR).unwrap(),
            ObjectId::EMPTY
        );
        assert!(next.has_object(&game, Coord3::new(2, 0, 0), PLAYER));
    }

    #[test]
    fn once_per_level_fires_only_once_for_current_3d_level_state() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), PLAYER)
            .unwrap();

        let player_to_box = Rule3::once_per_level(
            Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(PLAYER)]),
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
                remove: PLAYER,
                add: BOX,
            }],
        )
        .with_id(RuleId3(7));

        let first =
            transition_program(&game, &state, &[player_to_box.clone()], INPUT_RIGHT).unwrap();
        let second = transition_program(&game, &first, &[player_to_box], INPUT_RIGHT).unwrap();

        assert!(first.has_object(&game, Coord3::new(0, 0, 0), BOX));
        assert!(first.has_object(&game, Coord3::new(1, 0, 0), PLAYER));
        assert!(first.level_rule_has_fired(RuleId3(7)));
        assert_eq!(second, first);
    }

    #[test]
    fn once_per_level_does_not_mark_rule_when_no_3d_match_exists() {
        let game = game();
        let state = empty_state(1, 1, 1);
        let player_to_box = Rule3::once_per_level(
            Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(PLAYER)]),
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
                remove: PLAYER,
                add: BOX,
            }],
        )
        .with_id(RuleId3(8));

        let next = transition_program(&game, &state, &[player_to_box], INPUT_RIGHT).unwrap();

        assert_eq!(next, state);
        assert!(!next.level_rule_has_fired(RuleId3(8)));
    }

    #[test]
    fn board_snapshot_is_core_discrete_state_without_sprite_data() {
        let game = game();
        let mut state = empty_state(3, 3, 3);
        state
            .place_object(&game, Coord3::new(1, 1, 1), PLAYER)
            .unwrap();

        let snapshot = BoardSnapshot3::from_state(&state);

        assert_eq!(snapshot.size, Size3::new(3, 3, 3));
        assert_eq!(snapshot.cells.len(), 1);
        assert_eq!(snapshot.cells[0].position, Coord3::new(1, 1, 1));
        assert_eq!(snapshot.cells[0].objects, vec![PLAYER]);
    }

    #[test]
    fn patch_application_is_all_or_nothing_on_collision() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), WALL)
            .unwrap();

        let patch = Patch3 {
            ops: vec![PatchOp3::Move {
                from: Coord3::new(0, 0, 0),
                to: Coord3::new(1, 0, 0),
                object: PLAYER,
            }],
        };

        assert!(patch.apply(&game, &mut state).is_err());
        assert!(state.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(state.has_object(&game, Coord3::new(1, 0, 0), WALL));
    }

    #[test]
    fn patch_can_update_3d_visible_globals_and_scratch() {
        let game = game();
        let mut state = State3::empty_with_globals(Size3::new(1, 1, 1), 1, vec![2]).unwrap();
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();

        let patch = Patch3 {
            ops: vec![
                PatchOp3::UpdateGlobal {
                    global: GlobalId3(0),
                    op: GlobalUpdateOp::Add,
                    value: 3,
                },
                PatchOp3::SetScratch {
                    position: Coord3::new(0, 0, 0),
                    object: PLAYER,
                    scratch: ScratchId3(1),
                    value: Some(7),
                },
                PatchOp3::SetScratch {
                    position: Coord3::new(0, 0, 0),
                    object: ObjectId::EMPTY,
                    scratch: ScratchId3(2),
                    value: None,
                },
            ],
        };

        patch.apply(&game, &mut state).unwrap();

        assert_eq!(state.global_value(GlobalId3(0)), Some(5));
        assert!(state.has_scratch(&game, Coord3::new(0, 0, 0), PLAYER, ScratchId3(1), Some(7)));
        assert!(state.has_cell_scratch_key(Coord3::new(0, 0, 0), ScratchId3(2)));
    }

    #[test]
    fn move_patch_preserves_3d_slot_scratch() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        Patch3 {
            ops: vec![PatchOp3::SetScratch {
                position: Coord3::new(0, 0, 0),
                object: PLAYER,
                scratch: ScratchId3(1),
                value: Some(9),
            }],
        }
        .apply(&game, &mut state)
        .unwrap();

        Patch3 {
            ops: vec![PatchOp3::Move {
                from: Coord3::new(0, 0, 0),
                to: Coord3::new(1, 0, 0),
                object: PLAYER,
            }],
        }
        .apply(&game, &mut state)
        .unwrap();

        assert!(!state.has_scratch(&game, Coord3::new(0, 0, 0), PLAYER, ScratchId3(1), Some(9)));
        assert!(state.has_scratch(&game, Coord3::new(1, 0, 0), PLAYER, ScratchId3(1), Some(9)));
    }

    #[test]
    fn query_kind_evaluates_3d_objects_and_patterns() {
        let game = game();
        let mut state = empty_state(2, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), BOX)
            .unwrap();
        let push_pattern = Pattern3::new(vec![
            MatchCell3::new(Offset3::ZERO).require(PLAYER),
            MatchCell3::new(Direction3::RIGHT.offset).require(BOX),
        ]);

        assert_eq!(
            eval_query_kind(
                &game,
                &state,
                &QueryKind3::CountObjects(vec![PLAYER, BOX]),
                None
            ),
            2
        );
        assert_eq!(
            eval_query_kind(
                &game,
                &state,
                &QueryKind3::ExistsMatches(vec![push_pattern.clone()]),
                None
            ),
            1
        );
        assert_eq!(
            eval_query_kind(
                &game,
                &state,
                &QueryKind3::CountInputMatches(vec![(INPUT_RIGHT, push_pattern)]),
                Some(INPUT_RIGHT)
            ),
            1
        );
    }

    #[test]
    fn transition_solver_program_skips_visual_rules_and_objects() {
        let game = Game3::new_with_inputs_and_roles(
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
            ],
            vec![InputDef3::directional(
                INPUT_RIGHT,
                "right",
                Direction3::RIGHT,
            )],
            vec![BOX],
            vec![RuleId3(99)],
        );
        let mut state = empty_state(3, 1, 1);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), BOX)
            .unwrap();
        let visual_move = move_rule(Direction3::RIGHT).with_id(RuleId3(99));

        let next = transition_solver_program(&game, &state, &[visual_move], INPUT_RIGHT).unwrap();

        assert!(next.has_object(&game, Coord3::new(0, 0, 0), PLAYER));
        assert!(!next.has_object(&game, Coord3::new(1, 0, 0), BOX));
    }

    #[test]
    fn visual_snapshot_extracts_only_visible_non_empty_cells() {
        let game = game();
        let mut state = empty_state(3, 3, 3);
        state
            .place_object(&game, Coord3::new(1, 1, 1), PLAYER)
            .unwrap();

        let snapshot = VisualSnapshot3::from_state(
            &state,
            &[
                ObjectVisual3::new(PLAYER, "Player", "cube"),
                ObjectVisual3::new(BOX, "Box", "cube"),
            ],
        );

        assert_eq!(snapshot.size, Size3::new(3, 3, 3));
        assert_eq!(snapshot.cells.len(), 1);
        assert_eq!(snapshot.cells[0].position, Coord3::new(1, 1, 1));
        assert_eq!(snapshot.cells[0].objects.len(), 1);
        assert_eq!(snapshot.cells[0].objects[0].id, PLAYER);
        assert_eq!(snapshot.cells[0].objects[0].name, "Player");
        assert_eq!(snapshot.cells[0].objects[0].sprite, "cube");
    }

    #[test]
    fn local_frame_full_height_limits_3d_rules_by_horizontal_frame_only() {
        let game = game();
        let mut state = empty_state(4, 1, 3);
        state
            .place_object(&game, Coord3::new(0, 0, 0), PLAYER)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 2), BOX)
            .unwrap();
        state
            .place_object(&game, Coord3::new(3, 0, 0), BOX)
            .unwrap();
        let rule = Rule3::once_all(
            Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(BOX)]),
            vec![WriteOp3::Replace {
                offset: Offset3::ZERO,
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

        let next =
            transition_program_with_local_frame(&game, &state, &[rule], INPUT_RIGHT, Some(&frame))
                .unwrap();

        assert!(next.has_object(&game, Coord3::new(1, 0, 2), WALL));
        assert!(next.has_object(&game, Coord3::new(3, 0, 0), BOX));
    }
}
