use crate::{Coord3, Game3, LayerId, MarkId3, ObjectId, State3, StateError3, VariableId};
use puzzle_kernel::{GridPatchOp, MarkValueMatch, VariableUpdateOp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch3 {
    pub ops: Vec<PatchOp3>,
}

impl Patch3 {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn push(&mut self, op: PatchOp3) {
        self.ops.push(op);
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn apply(&self, game: &Game3, state: &mut State3) -> Result<(), PatchError3> {
        self.apply_in_place(game, state).map(|_| ())
    }

    pub(crate) fn apply_in_place(
        &self,
        game: &Game3,
        state: &mut State3,
    ) -> Result<bool, PatchError3> {
        let changed = self.validate(game, state)?;
        apply_moves(game, state, &self.ops)?;
        for op in &self.ops {
            apply_remove_phase(game, state, op)?;
        }
        for op in &self.ops {
            apply_add_phase(game, state, op)?;
        }
        state.recompute_hash();
        Ok(changed)
    }

    pub(crate) fn validate(&self, game: &Game3, state: &State3) -> Result<bool, PatchError3> {
        let mut slots = SlotOverlay3::new();
        validate_moves(game, state, &self.ops, &mut slots)?;
        let mut changed = slots.changed(state)?;

        for op in &self.ops {
            match *op {
                PatchOp3::Add { position, object } => {
                    let layer = checked_object_layer(game, object)?;
                    let existing = slots.get(state, position, layer)?;
                    if existing == object {
                        continue;
                    }
                    if !existing.is_empty() {
                        return Err(StateError3::LayerOccupied {
                            position,
                            layer,
                            existing,
                            attempted: object,
                        }
                        .into());
                    }
                    slots.set(position, layer, object);
                    changed = true;
                }
                PatchOp3::Remove { position, object } => {
                    let layer = checked_object_layer(game, object)?;
                    let found = slots.get(state, position, layer)?;
                    if found != object {
                        return Err(StateError3::ObjectNotPresent { position, object }.into());
                    }
                    slots.set(position, layer, ObjectId::EMPTY);
                    changed = true;
                }
                PatchOp3::Replace {
                    position,
                    remove,
                    add,
                } => {
                    let remove_layer = checked_object_layer(game, remove)?;
                    let found = slots.get(state, position, remove_layer)?;
                    if found != remove {
                        return Err(StateError3::ObjectNotPresent {
                            position,
                            object: remove,
                        }
                        .into());
                    }
                    slots.set(position, remove_layer, ObjectId::EMPTY);

                    let add_layer = checked_object_layer(game, add)?;
                    let existing = slots.get(state, position, add_layer)?;
                    if existing == add {
                        continue;
                    }
                    if !existing.is_empty() {
                        return Err(StateError3::LayerOccupied {
                            position,
                            layer: add_layer,
                            existing,
                            attempted: add,
                        }
                        .into());
                    }
                    slots.set(position, add_layer, add);
                    changed = true;
                }
                PatchOp3::Move { .. } => {}
                PatchOp3::UpdateVariable {
                    variable,
                    op,
                    value,
                } => {
                    let next = validate_variable_update(state, variable, op, value)?;
                    changed |= state.variable_value(variable) != Some(next);
                }
                PatchOp3::SetMark {
                    position,
                    object,
                    mark,
                    value,
                } => {
                    if object.is_empty() {
                        changed |= !state.has_cell_mark(position, mark, value);
                    } else {
                        let layer =
                            expect_object_in_overlay(game, state, &slots, position, object)?;
                        if state
                            .get_layer(position, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= !state.has_mark(game, position, object, mark, value);
                        } else {
                            changed = true;
                        }
                    }
                }
                PatchOp3::RemoveMark {
                    position,
                    object,
                    mark,
                    value,
                    match_value,
                } => {
                    let value = match match_value {
                        MarkValueMatch::Any => None,
                        MarkValueMatch::Exact => value,
                    };
                    if object.is_empty() {
                        changed |= match match_value {
                            MarkValueMatch::Any => state.has_cell_mark_key(position, mark),
                            MarkValueMatch::Exact => state.has_cell_mark(position, mark, value),
                        };
                    } else {
                        let layer =
                            expect_object_in_overlay(game, state, &slots, position, object)?;
                        if state
                            .get_layer(position, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= match match_value {
                                MarkValueMatch::Any => {
                                    state.has_mark_key(game, position, object, mark)
                                }
                                MarkValueMatch::Exact => {
                                    state.has_mark(game, position, object, mark, value)
                                }
                            };
                        } else {
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(changed || slots.changed(state)?)
    }
}

#[derive(Default)]
struct SlotOverlay3 {
    slots: Vec<(Coord3, LayerId, ObjectId)>,
}

impl SlotOverlay3 {
    fn new() -> Self {
        Self::default()
    }

    fn get(
        &self,
        state: &State3,
        position: Coord3,
        layer: LayerId,
    ) -> Result<ObjectId, PatchError3> {
        self.slots
            .iter()
            .rev()
            .find_map(|(slot_position, slot_layer, object)| {
                (*slot_position == position && *slot_layer == layer).then_some(*object)
            })
            .map(Ok)
            .unwrap_or_else(|| state.get_layer(position, layer).map_err(PatchError3::from))
    }

    fn set(&mut self, position: Coord3, layer: LayerId, object: ObjectId) {
        if let Some((_, _, existing)) =
            self.slots
                .iter_mut()
                .rev()
                .find(|(slot_position, slot_layer, _)| {
                    *slot_position == position && *slot_layer == layer
                })
        {
            *existing = object;
            return;
        }
        self.slots.push((position, layer, object));
    }

    fn changed(&self, state: &State3) -> Result<bool, PatchError3> {
        self.slots
            .iter()
            .try_fold(false, |changed, (position, layer, object)| {
                Ok(changed || state.get_layer(*position, *layer)? != *object)
            })
    }
}

pub type PatchOp3 = GridPatchOp<Coord3, ObjectId, VariableId, MarkId3>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchError3 {
    State(StateError3),
}

impl From<StateError3> for PatchError3 {
    fn from(value: StateError3) -> Self {
        Self::State(value)
    }
}

fn apply_remove_phase(game: &Game3, state: &mut State3, op: &PatchOp3) -> Result<(), PatchError3> {
    match *op {
        PatchOp3::Add { .. }
        | PatchOp3::Move { .. }
        | PatchOp3::UpdateVariable { .. }
        | PatchOp3::SetMark { .. }
        | PatchOp3::RemoveMark { .. } => {}
        PatchOp3::Remove { position, object } => {
            state.remove_object(game, position, object)?;
        }
        PatchOp3::Replace {
            position, remove, ..
        } => {
            state.remove_object(game, position, remove)?;
        }
    }
    Ok(())
}

fn apply_add_phase(game: &Game3, state: &mut State3, op: &PatchOp3) -> Result<(), PatchError3> {
    match *op {
        PatchOp3::Add { position, object } => {
            state.place_object(game, position, object)?;
        }
        PatchOp3::Move { .. } => {}
        PatchOp3::Remove { .. } => {}
        PatchOp3::Replace { position, add, .. } => {
            state.place_object(game, position, add)?;
        }
        PatchOp3::UpdateVariable {
            variable,
            op,
            value,
        } => {
            state.update_visible_variable(variable, op, value)?;
        }
        PatchOp3::SetMark {
            position,
            object,
            mark,
            value,
        } => {
            apply_set_mark(game, state, position, object, mark, value)?;
        }
        PatchOp3::RemoveMark {
            position,
            object,
            mark,
            value,
            match_value,
        } => {
            if matches!(match_value, MarkValueMatch::Any) {
                apply_remove_mark(game, state, position, object, mark, None)?;
            } else {
                apply_remove_mark(game, state, position, object, mark, value)?;
            }
        }
    }
    Ok(())
}

fn apply_moves(game: &Game3, state: &mut State3, ops: &[PatchOp3]) -> Result<(), PatchError3> {
    let mut moves = Vec::new();
    let mut sources = Vec::new();
    let mut destinations = Vec::new();

    for op in ops {
        let PatchOp3::Move { from, to, object } = *op else {
            continue;
        };
        let layer = checked_object_layer(game, object)?;
        let found = state.get_layer(from, layer)?;
        if found != object {
            return Err(StateError3::ObjectNotPresent {
                position: from,
                object,
            }
            .into());
        }
        if destinations.contains(&(to, layer)) {
            return Err(StateError3::LayerOccupied {
                position: to,
                layer,
                existing: object,
                attempted: object,
            }
            .into());
        }
        sources.push((from, layer));
        destinations.push((to, layer));
        moves.push((from, to, layer, object));
    }

    for (_, to, layer, object) in &moves {
        let existing = state.get_layer(*to, *layer)?;
        if !existing.is_empty() && !sources.contains(&(*to, *layer)) {
            return Err(StateError3::LayerOccupied {
                position: *to,
                layer: *layer,
                existing,
                attempted: *object,
            }
            .into());
        }
    }

    let mut moved = Vec::with_capacity(moves.len());
    for (from, to, layer, object) in moves {
        let mark = state.take_slot_for_move_unchecked(from, layer);
        moved.push((to, layer, object, mark));
    }
    for (to, layer, object, mark) in moved {
        state.place_moved_slot_unchecked(to, layer, object, mark);
    }
    Ok(())
}

fn validate_moves(
    game: &Game3,
    state: &State3,
    ops: &[PatchOp3],
    slots: &mut SlotOverlay3,
) -> Result<(), PatchError3> {
    let mut moves = Vec::new();
    let mut sources = Vec::new();
    let mut destinations = Vec::new();

    for op in ops {
        let PatchOp3::Move { from, to, object } = *op else {
            continue;
        };
        let layer = checked_object_layer(game, object)?;
        let found = state.get_layer(from, layer)?;
        if found != object {
            return Err(StateError3::ObjectNotPresent {
                position: from,
                object,
            }
            .into());
        }
        if destinations.contains(&(to, layer)) {
            return Err(StateError3::LayerOccupied {
                position: to,
                layer,
                existing: object,
                attempted: object,
            }
            .into());
        }
        sources.push((from, layer));
        destinations.push((to, layer));
        moves.push((from, to, layer, object));
    }

    for (_, to, layer, object) in &moves {
        let existing = state.get_layer(*to, *layer)?;
        if !existing.is_empty() && !sources.contains(&(*to, *layer)) {
            return Err(StateError3::LayerOccupied {
                position: *to,
                layer: *layer,
                existing,
                attempted: *object,
            }
            .into());
        }
    }

    for (from, _, layer, _) in &moves {
        slots.set(*from, *layer, ObjectId::EMPTY);
    }
    for (_, to, layer, object) in moves {
        slots.set(to, layer, object);
    }

    Ok(())
}

fn apply_set_mark(
    game: &Game3,
    state: &mut State3,
    position: Coord3,
    object: ObjectId,
    mark: MarkId3,
    value: Option<i64>,
) -> Result<(), PatchError3> {
    if object.is_empty() {
        state.set_cell_mark_unchecked(position, mark, value);
        return Ok(());
    }
    let layer = checked_object_layer(game, object)?;
    let found = state.get_layer(position, layer)?;
    if found != object {
        return Err(StateError3::ObjectNotPresent { position, object }.into());
    }
    state.set_mark_unchecked(position, layer, mark, value);
    Ok(())
}

fn apply_remove_mark(
    game: &Game3,
    state: &mut State3,
    position: Coord3,
    object: ObjectId,
    mark: MarkId3,
    value: Option<i64>,
) -> Result<(), PatchError3> {
    if object.is_empty() {
        state.remove_cell_mark_unchecked(position, mark, value);
        return Ok(());
    }
    let layer = checked_object_layer(game, object)?;
    let found = state.get_layer(position, layer)?;
    if found != object {
        return Err(StateError3::ObjectNotPresent { position, object }.into());
    }
    state.remove_mark_unchecked(position, layer, mark, value);
    Ok(())
}

fn checked_object_layer(game: &Game3, object: ObjectId) -> Result<LayerId, StateError3> {
    game.object_layer(object)
        .ok_or(StateError3::UnknownObject { object })
}

fn expect_object_in_overlay(
    game: &Game3,
    state: &State3,
    slots: &SlotOverlay3,
    position: Coord3,
    object: ObjectId,
) -> Result<LayerId, PatchError3> {
    let layer = checked_object_layer(game, object)?;
    let found = slots.get(state, position, layer)?;
    if found != object {
        return Err(StateError3::ObjectNotPresent { position, object }.into());
    }
    Ok(layer)
}

fn validate_variable_update(
    state: &State3,
    variable: VariableId,
    op: VariableUpdateOp,
    value: i64,
) -> Result<i64, PatchError3> {
    let current = state
        .variable_value(variable)
        .ok_or(StateError3::VariableOutOfBounds { variable })?;
    let next = match op {
        VariableUpdateOp::Set => Some(value),
        VariableUpdateOp::Add => current.checked_add(value),
        VariableUpdateOp::Subtract => current.checked_sub(value),
        VariableUpdateOp::Multiply => current.checked_mul(value),
        VariableUpdateOp::Divide => {
            if value == 0 {
                return Err(StateError3::VariableDivisionByZero { variable }.into());
            }
            current.checked_div(value)
        }
        VariableUpdateOp::Remainder => {
            if value == 0 {
                return Err(StateError3::VariableDivisionByZero { variable }.into());
            }
            current.checked_rem(value)
        }
    }
    .ok_or(StateError3::VariableOverflow { variable })?;
    Ok(next)
}
