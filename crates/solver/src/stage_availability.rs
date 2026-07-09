use puzzle_core::{
    CompiledGame, MatchCell, ObjectId, Pattern, Rule, RuleId, RuleStep, State, WriteOp,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolverStageAvailability {
    available_objects: BTreeSet<ObjectId>,
    available_rules: BTreeSet<RuleId>,
}

impl SolverStageAvailability {
    pub fn from_initial_state(game: &CompiledGame, initial: &State) -> Self {
        let mut available_objects = BTreeSet::new();
        for object in initial.slots() {
            if !object.is_empty() {
                available_objects.insert(*object);
            }
        }
        let mut availability = Self {
            available_objects,
            available_rules: BTreeSet::new(),
        };
        availability.propagate_program(game.program());
        availability
    }

    pub fn contains_object(&self, object: ObjectId) -> bool {
        self.available_objects.contains(&object)
    }

    pub fn available_objects(&self) -> Vec<ObjectId> {
        self.available_objects.iter().copied().collect()
    }

    pub fn contains_rule(&self, rule: RuleId) -> bool {
        self.available_rules.contains(&rule)
    }

    pub fn available_rules(&self) -> Vec<RuleId> {
        self.available_rules.iter().copied().collect()
    }

    fn propagate_program(&mut self, program: &[RuleStep]) {
        let mut changed = true;
        while changed {
            changed = false;
            for step in program {
                changed |= self.propagate_step(step);
            }
        }
    }

    fn propagate_step(&mut self, step: &RuleStep) -> bool {
        match step {
            RuleStep::Rule(rule) => self.propagate_rule(rule),
            RuleStep::ConditionalBlock { steps, .. }
            | RuleStep::Block { steps, .. }
            | RuleStep::LocalFrame { steps, .. } => {
                let mut changed = false;
                for step in steps {
                    changed |= self.propagate_step(step);
                }
                changed
            }
            RuleStep::ConditionalBranch {
                then_steps,
                else_steps,
                ..
            } => {
                let mut changed = false;
                for step in then_steps.iter().chain(else_steps) {
                    changed |= self.propagate_step(step);
                }
                changed
            }
            RuleStep::AfterTriggered { steps, then_steps } => {
                let mut changed = false;
                for step in steps.iter().chain(then_steps) {
                    changed |= self.propagate_step(step);
                }
                changed
            }
        }
    }

    fn propagate_rule(&mut self, rule: &Rule) -> bool {
        if !self.pattern_may_match(&rule.pattern) {
            return false;
        }

        let mut changed = self.available_rules.insert(rule.id);
        for write in &rule.writes {
            changed |= self.insert_write_outputs(write, &rule.pattern);
        }
        changed
    }

    fn pattern_may_match(&self, pattern: &Pattern) -> bool {
        pattern
            .components
            .iter()
            .flat_map(|component| &component.cells)
            .all(|cell| self.cell_may_match(cell))
    }

    fn cell_may_match(&self, cell: &MatchCell) -> bool {
        cell.require_objects
            .iter()
            .all(|object| self.contains_object(*object))
            && cell.require_object_sets.iter().all(|matcher| {
                matcher
                    .objects
                    .iter()
                    .any(|object| self.contains_object(*object))
            })
            && cell
                .require_mark
                .iter()
                .all(|mark| self.contains_object(mark.object))
    }

    fn insert_write_outputs(&mut self, write: &WriteOp, pattern: &Pattern) -> bool {
        match write {
            WriteOp::Add { object, .. } | WriteOp::Replace { add: object, .. } => {
                self.insert_available_object(*object)
            }
            WriteOp::AddObjectSet { binding, .. } => {
                let mut changed = false;
                for object in object_set_binding_objects(pattern, *binding) {
                    changed |= self.insert_available_object(object);
                }
                changed
            }
            WriteOp::Remove { .. }
            | WriteOp::RemoveObjectSet { .. }
            | WriteOp::Move { .. }
            | WriteOp::MoveObjectSet { .. }
            | WriteOp::SetMark { .. }
            | WriteOp::SetObjectSetMark { .. }
            | WriteOp::RemoveMark { .. }
            | WriteOp::RemoveObjectSetMark { .. } => false,
        }
    }

    fn insert_available_object(&mut self, object: ObjectId) -> bool {
        !object.is_empty() && self.available_objects.insert(object)
    }
}

fn object_set_binding_objects(pattern: &Pattern, binding: u16) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for cell in pattern
        .components
        .iter()
        .flat_map(|component| &component.cells)
    {
        for matcher in &cell.require_object_sets {
            if matcher.binding != binding {
                continue;
            }
            for object in &matcher.objects {
                if !objects.contains(object) {
                    objects.push(*object);
                }
            }
        }
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_core::{
        LayerId, ObjectDef, ObjectSetMatcher, Offset, PatternComponent, RuleApplication, WriteOp,
    };

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const KEY: ObjectId = ObjectId(3);
    const DOOR: ObjectId = ObjectId(4);
    const BATTERY: ObjectId = ObjectId(5);

    fn object(id: u16, layer: LayerId) -> ObjectDef {
        ObjectDef {
            id: ObjectId(id),
            layer_id: layer,
        }
    }

    fn fixed(dx: i16, dy: i16) -> Offset {
        Offset::Fixed { dx, dy }
    }

    fn cell(require_objects: Vec<ObjectId>) -> MatchCell {
        MatchCell {
            offset: fixed(0, 0),
            require_null: false,
            require_objects,
            require_object_sets: Vec::new(),
            forbid_objects: Vec::new(),
            require_mark: Vec::new(),
            require_object_set_mark: Vec::new(),
            forbid_mark: Vec::new(),
            forbid_object_set_mark: Vec::new(),
        }
    }

    fn pattern(require_objects: Vec<ObjectId>) -> Pattern {
        Pattern {
            components: vec![PatternComponent {
                cells: vec![cell(require_objects)],
                gap_count: 0,
            }],
        }
    }

    fn object_set_pattern(binding: u16, objects: Vec<ObjectId>) -> Pattern {
        Pattern {
            components: vec![PatternComponent {
                cells: vec![MatchCell {
                    offset: fixed(0, 0),
                    require_null: false,
                    require_objects: Vec::new(),
                    require_object_sets: vec![ObjectSetMatcher {
                        binding,
                        layer: LayerId(1),
                        objects,
                    }],
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

    fn add_object_set(binding: u16) -> WriteOp {
        WriteOp::AddObjectSet {
            component: 0,
            offset: fixed(0, 0),
            binding,
        }
    }

    fn rule(id: u16, reads: Vec<ObjectId>, writes: Vec<WriteOp>) -> Rule {
        Rule {
            id: RuleId(id),
            guards: Vec::new(),
            application: RuleApplication::Once,
            pattern: pattern(reads),
            writes,
            effects: Vec::new(),
        }
    }

    #[test]
    fn stage_availability_starts_from_initial_state_objects() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
            ],
            Vec::new(),
        );
        let mut initial = State::empty(2, 1, 2, 3).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();
        initial.place_object(&game, 1, 0, BOX).unwrap();

        let availability = SolverStageAvailability::from_initial_state(&game, &initial);

        assert!(availability.contains_object(PLAYER));
        assert!(availability.contains_object(BOX));
        assert!(!availability.contains_object(KEY));
        assert_eq!(availability.available_objects(), vec![PLAYER, BOX]);
    }

    #[test]
    fn stage_availability_propagates_rule_outputs_from_available_reads() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![
                RuleStep::Rule(rule(1, vec![PLAYER], vec![add(KEY)])),
                RuleStep::Rule(rule(2, vec![KEY], vec![add(DOOR)])),
            ],
        );
        let mut initial = State::empty(2, 1, 2, 4).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();

        let availability = SolverStageAvailability::from_initial_state(&game, &initial);

        assert!(availability.contains_object(PLAYER));
        assert!(availability.contains_object(KEY));
        assert!(availability.contains_object(DOOR));
        assert!(availability.contains_rule(RuleId(1)));
        assert!(availability.contains_rule(RuleId(2)));
        assert_eq!(availability.available_objects(), vec![PLAYER, KEY, DOOR]);
        assert_eq!(availability.available_rules(), vec![RuleId(1), RuleId(2)]);
    }

    #[test]
    fn stage_availability_does_not_propagate_outputs_from_unavailable_reads() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
                object(5, LayerId(1)),
            ],
            vec![RuleStep::Rule(rule(1, vec![BATTERY], vec![add(DOOR)]))],
        );
        let mut initial = State::empty(2, 1, 2, 5).unwrap();
        initial.place_object(&game, 0, 0, PLAYER).unwrap();

        let availability = SolverStageAvailability::from_initial_state(&game, &initial);

        assert!(availability.contains_object(PLAYER));
        assert!(!availability.contains_object(BATTERY));
        assert!(!availability.contains_object(DOOR));
        assert!(!availability.contains_rule(RuleId(1)));
        assert_eq!(availability.available_objects(), vec![PLAYER]);
        assert_eq!(availability.available_rules(), Vec::<RuleId>::new());
    }

    #[test]
    fn stage_availability_propagates_object_set_write_candidates() {
        let game = CompiledGame::new_with_program(
            2,
            vec![
                object(1, LayerId(0)),
                object(2, LayerId(1)),
                object(3, LayerId(1)),
                object(4, LayerId(1)),
            ],
            vec![RuleStep::Rule(Rule {
                id: RuleId(1),
                guards: Vec::new(),
                application: RuleApplication::Once,
                pattern: object_set_pattern(0, vec![BOX, KEY]),
                writes: vec![add_object_set(0)],
                effects: Vec::new(),
            })],
        );
        let mut initial = State::empty(2, 1, 2, 4).unwrap();
        initial.place_object(&game, 0, 0, BOX).unwrap();

        let availability = SolverStageAvailability::from_initial_state(&game, &initial);

        assert!(availability.contains_rule(RuleId(1)));
        assert!(availability.contains_object(BOX));
        assert!(availability.contains_object(KEY));
    }
}
