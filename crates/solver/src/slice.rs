use crate::{SolverRelevance, SolverStageAvailability, SolverStateSlicer};
use puzzle_core::{
    GridCompiledGame, GridExecutableProgram, GridProgramCatalog, GridProgramRef, GridRule,
    GridRuleStep, GridSize, GridState, GridWriteOp, ObjectId, RuleId,
};
use puzzle_lang::LoadedGridGame;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverSlice {
    kept_objects: BTreeSet<ObjectId>,
    kept_rules: BTreeSet<RuleId>,
}

impl SolverSlice {
    pub fn from_loaded_level_roots<'a, const D: usize, Size: GridSize<D> + 'a>(
        loaded: &'a LoadedGridGame<D, Size>,
        level_index: usize,
        states: impl IntoIterator<Item = &'a GridState<D, Size>>,
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Option<Self> {
        let level = loaded.levels.get(level_index)?;
        let mut programs = loaded
            .programs_for_level(level_index)?
            .into_iter()
            .map(GridExecutableProgram::as_steps)
            .collect::<Vec<_>>();
        programs.extend(
            loaded
                .level_start_program
                .iter()
                .map(|program| program.as_steps()),
        );
        programs.extend(
            loaded
                .level_clear_program
                .iter()
                .map(|program| program.as_steps()),
        );
        programs.extend(
            loaded
                .last_level_clear_program
                .iter()
                .map(|program| program.as_steps()),
        );
        programs.extend(
            loaded
                .level_start_program_for_level(level_index)
                .map(GridExecutableProgram::as_steps),
        );
        programs.extend(
            loaded
                .level_clear_program_for_level(level_index)
                .map(GridExecutableProgram::as_steps),
        );
        let relevance =
            SolverRelevance::from_programs_roots(&loaded.game, programs.iter().copied(), roots);
        let availability = SolverStageAvailability::from_states_and_programs(
            states.into_iter().chain([&level.initial_state]),
            programs.iter().copied(),
        );
        Some(Self::from_relevance_and_availability(
            &relevance,
            &availability,
        ))
    }

    pub fn from_loaded_game_roots<'a, const D: usize, Size: GridSize<D> + 'a>(
        loaded: &'a LoadedGridGame<D, Size>,
        states: impl IntoIterator<Item = &'a GridState<D, Size>>,
        roots: impl IntoIterator<Item = ObjectId>,
    ) -> Self {
        let mut programs = vec![loaded.game.program()];
        programs.extend(
            loaded
                .program_catalog
                .programs()
                .iter()
                .map(GridExecutableProgram::as_steps),
        );
        programs.extend(
            loaded
                .level_start_program
                .iter()
                .map(|program| program.as_steps()),
        );
        programs.extend(
            loaded
                .level_clear_program
                .iter()
                .map(|program| program.as_steps()),
        );
        programs.extend(
            loaded
                .last_level_clear_program
                .iter()
                .map(|program| program.as_steps()),
        );
        let relevance =
            SolverRelevance::from_programs_roots(&loaded.game, programs.iter().copied(), roots);
        let availability = SolverStageAvailability::from_states_and_programs(
            states
                .into_iter()
                .chain(loaded.levels.iter().map(|level| &level.initial_state)),
            programs.iter().copied(),
        );
        Self::from_relevance_and_availability(&relevance, &availability)
    }

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

    pub fn project_game<const D: usize>(&self, game: &GridCompiledGame<D>) -> GridCompiledGame<D> {
        assert_unique_rule_ids(game.rules());
        GridCompiledGame::new_with_mark_condition_defs_and_program(
            game.layer_count,
            game.objects().to_vec(),
            game.mark().to_vec(),
            game.condition_defs().to_vec(),
            self.filter_program(game.program()),
        )
    }

    pub fn project_program<const D: usize>(
        &self,
        program: &GridExecutableProgram<D>,
    ) -> GridExecutableProgram<D> {
        GridExecutableProgram::new(self.filter_program(program.as_steps()))
    }

    pub fn project_loaded_game<const D: usize, Size: GridSize<D>>(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        state_slicer: &SolverStateSlicer<ObjectId>,
    ) -> LoadedGridGame<D, Size> {
        let mut projected = loaded.clone();
        projected.game = self.project_game(&loaded.game);
        projected.level_start_program = loaded
            .level_start_program
            .as_ref()
            .map(|program| self.project_program(program));
        projected.level_clear_program = loaded
            .level_clear_program
            .as_ref()
            .map(|program| self.project_program(program));
        projected.last_level_clear_program = loaded
            .last_level_clear_program
            .as_ref()
            .map(|program| self.project_program(program));
        projected.program_catalog = GridProgramCatalog::default();
        for (projected_level, source_level) in projected.levels.iter_mut().zip(&loaded.levels) {
            projected_level.initial_state = state_slicer.project_state(&source_level.initial_state);
            projected_level.program = source_level.program.map_references(|reference| {
                self.project_program_reference(loaded, reference, &mut projected.program_catalog)
            });
            projected_level.level_start_program =
                source_level.level_start_program.map(|reference| {
                    self.project_program_reference(
                        loaded,
                        reference,
                        &mut projected.program_catalog,
                    )
                });
            projected_level.level_clear_program =
                source_level.level_clear_program.map(|reference| {
                    self.project_program_reference(
                        loaded,
                        reference,
                        &mut projected.program_catalog,
                    )
                });
        }
        projected
    }

    fn project_program_reference<const D: usize, Size: GridSize<D>>(
        &self,
        loaded: &LoadedGridGame<D, Size>,
        reference: GridProgramRef,
        catalog: &mut GridProgramCatalog<D>,
    ) -> GridProgramRef {
        match reference {
            GridProgramRef::Main => GridProgramRef::Main,
            GridProgramRef::Catalog(_) => catalog.intern(
                self.project_program(
                    loaded
                        .resolve_program(reference)
                        .expect("loaded program reference is valid"),
                ),
            ),
        }
    }

    fn filter_program<const D: usize>(&self, program: &[GridRuleStep<D>]) -> Vec<GridRuleStep<D>> {
        program
            .iter()
            .filter_map(|step| self.filter_step(step))
            .collect()
    }

    fn filter_step<const D: usize>(&self, step: &GridRuleStep<D>) -> Option<GridRuleStep<D>> {
        match step {
            GridRuleStep::Rule(rule) => self
                .kept_rules
                .contains(&rule.id)
                .then(|| GridRuleStep::Rule(self.project_rule(rule))),
            GridRuleStep::ConditionalBlock { condition, steps } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| GridRuleStep::ConditionalBlock {
                    condition: condition.clone(),
                    steps,
                })
            }
            GridRuleStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let then_steps = self.filter_program(then_steps);
                let else_steps = self.filter_program(else_steps);
                (!then_steps.is_empty() || !else_steps.is_empty()).then(|| {
                    GridRuleStep::ConditionalBranch {
                        condition: condition.clone(),
                        then_steps,
                        else_steps,
                    }
                })
            }
            GridRuleStep::Block {
                application,
                stop_condition,
                steps,
            } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| GridRuleStep::Block {
                    application: *application,
                    stop_condition: stop_condition.clone(),
                    steps,
                })
            }
            GridRuleStep::AfterTriggered { steps, then_steps } => {
                let filtered_steps = self.filter_program(steps);
                let filtered_then_steps = self.filter_program(then_steps);
                if !filtered_then_steps.is_empty() {
                    // The complete trigger program determines whether the relevant
                    // continuation runs; pruning it would change that predicate.
                    Some(GridRuleStep::AfterTriggered {
                        steps: steps.clone(),
                        then_steps: filtered_then_steps,
                    })
                } else if !filtered_steps.is_empty() {
                    // The trigger itself can contain relevant writes even when its
                    // continuation is irrelevant. Keep the wrapper so application
                    // and fired-state semantics remain unchanged.
                    Some(GridRuleStep::AfterTriggered {
                        steps: filtered_steps,
                        then_steps: Vec::new(),
                    })
                } else {
                    None
                }
            }
            GridRuleStep::LocalFrame { frame, steps } => {
                let steps = self.filter_program(steps);
                (!steps.is_empty()).then(|| GridRuleStep::LocalFrame {
                    frame: frame.clone(),
                    steps,
                })
            }
        }
    }

    fn project_rule<const D: usize>(&self, rule: &GridRule<D>) -> GridRule<D> {
        let mut projected = rule.clone();
        for component in &mut projected.pattern.components {
            for cell in &mut component.cells {
                for object_set in &mut cell.require_object_sets {
                    object_set
                        .objects
                        .retain(|object| self.kept_objects.contains(object));
                }
                cell.forbid_objects
                    .retain(|object| self.kept_objects.contains(object));
                cell.forbid_mark
                    .retain(|mark| self.kept_objects.contains(&mark.object));
            }
        }
        projected.writes = rule
            .writes
            .iter()
            .filter_map(|write| self.project_write(write))
            .collect();
        projected
    }

    fn project_write<const D: usize>(&self, write: &GridWriteOp<D>) -> Option<GridWriteOp<D>> {
        match write.clone() {
            GridWriteOp::Add { object, .. }
            | GridWriteOp::Remove { object, .. }
            | GridWriteOp::Move { object, .. }
            | GridWriteOp::SetMark { object, .. }
            | GridWriteOp::RemoveMark { object, .. }
                if !self.kept_objects.contains(&object) =>
            {
                None
            }
            GridWriteOp::Replace {
                component,
                offset,
                remove,
                add,
            } => match (
                self.kept_objects.contains(&remove),
                self.kept_objects.contains(&add),
            ) {
                (true, true) => Some(GridWriteOp::Replace {
                    component,
                    offset,
                    remove,
                    add,
                }),
                (true, false) => Some(GridWriteOp::Remove {
                    component,
                    offset,
                    object: remove,
                }),
                (false, true) => Some(GridWriteOp::Add {
                    component,
                    offset,
                    object: add,
                }),
                (false, false) => None,
            },
            projected => Some(projected),
        }
    }
}

fn assert_unique_rule_ids<const D: usize>(rules: &[GridRule<D>]) {
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
        CompiledGame, Effect, GridCompiledGame, GridCoord, GridMatchCell, GridOffset, GridPattern,
        GridPatternComponent, GridRule, GridRuleStep, GridState, GridWriteOp, InputId, LayerId,
        MatchCell, ObjectDef, Offset, Pattern, PatternComponent, Rule, RuleApplication, RuleId,
        RuleStep, Size3, State, WriteOp, transition_state,
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

    fn fixed3(dx: i16, dy: i16, dz: i16) -> GridOffset<3> {
        GridOffset::Fixed {
            delta: [dx, dy, dz].into(),
        }
    }

    fn rule3(id: u16, read: ObjectId, write: ObjectId) -> GridRule<3> {
        GridRule {
            id: RuleId(id),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: GridPattern {
                components: vec![GridPatternComponent {
                    cells: vec![GridMatchCell {
                        offset: fixed3(0, 0, 0),
                        require_null: false,
                        require_objects: vec![read],
                        require_object_sets: Vec::new(),
                        forbid_objects: Vec::new(),
                        require_mark: Vec::new(),
                        require_object_set_mark: Vec::new(),
                        forbid_mark: Vec::new(),
                        forbid_object_set_mark: Vec::new(),
                    }],
                    gap_count: 0,
                }],
            },
            writes: vec![GridWriteOp::Add {
                component: 0,
                offset: fixed3(0, 0, 0),
                object: write,
            }],
            effects: Vec::new(),
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
    fn solver_slice_projects_3d_game_with_the_shared_analysis() {
        let game = GridCompiledGame::<3>::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![
                GridRuleStep::Rule(rule3(1, SWITCH, DOOR)),
                GridRuleStep::Rule(rule3(2, BATTERY, DOOR)),
            ],
        );
        let mut initial = GridState::<3, Size3>::empty_sized(Size3::new(2, 1, 1), 2, 4).unwrap();
        initial
            .place_object_at(&game, GridCoord::new([0, 0, 0]), PLAYER)
            .unwrap();
        initial
            .place_object_at(&game, GridCoord::new([1, 0, 0]), SWITCH)
            .unwrap();
        let relevance = SolverRelevance::from_root_objects(&game, [DOOR]);
        let availability = SolverStageAvailability::from_initial_state(&game, &initial);
        let slice = SolverSlice::from_relevance_and_availability(&relevance, &availability);

        let projected = slice.project_game(&game);

        assert_eq!(projected.rules().len(), 1);
        assert_eq!(projected.rules()[0].id, RuleId(1));
        assert!(slice.kept_objects().contains(&SWITCH));
        assert!(!slice.kept_objects().contains(&BATTERY));
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
