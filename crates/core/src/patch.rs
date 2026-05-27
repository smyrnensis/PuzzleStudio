use crate::compiled_game::{CompiledGame, GlobalUpdateOp, ScratchValueMatch};
use crate::ids::{GlobalId, LayerId, ObjectId, ScratchId};
use crate::state::{State, StateError};

#[derive(Clone, Debug, Default)]
pub struct Patch {
    pub ops: Vec<PatchOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchOp {
    Add {
        x: u16,
        y: u16,
        object: ObjectId,
    },
    Remove {
        x: u16,
        y: u16,
        object: ObjectId,
    },
    Move {
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        object: ObjectId,
    },
    Replace {
        x: u16,
        y: u16,
        remove: ObjectId,
        add: ObjectId,
    },
    UpdateGlobal {
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    },
    SetScratch {
        x: u16,
        y: u16,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    },
    RemoveScratch {
        x: u16,
        y: u16,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
        match_value: ScratchValueMatch,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchError {
    State(StateError),
    UnknownObject {
        object: ObjectId,
    },
    ExpectedObject {
        x: u16,
        y: u16,
        layer: LayerId,
        expected: ObjectId,
        found: ObjectId,
    },
    LayerOccupied {
        x: u16,
        y: u16,
        layer: LayerId,
        existing: ObjectId,
        attempted: ObjectId,
    },
}

impl From<StateError> for PatchError {
    fn from(value: StateError) -> Self {
        Self::State(value)
    }
}

impl Patch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, game: &CompiledGame, state: &State) -> Result<State, PatchError> {
        let mut next = state.clone();
        self.apply_in_place(game, &mut next)?;
        Ok(next)
    }

    pub(crate) fn apply_in_place(
        &self,
        game: &CompiledGame,
        state: &mut State,
    ) -> Result<bool, PatchError> {
        let changed = self.validate(game, state)?;

        apply_moves(game, state, &self.ops)?;

        for op in &self.ops {
            match *op {
                PatchOp::Add { x, y, object } => apply_add(game, state, x, y, object)?,
                PatchOp::Remove { x, y, object } => apply_remove(game, state, x, y, object)?,
                PatchOp::Move { .. } => {}
                PatchOp::Replace { x, y, remove, add } => {
                    apply_remove(game, state, x, y, remove)?;
                    apply_add(game, state, x, y, add)?;
                }
                PatchOp::UpdateGlobal { global, op, value } => {
                    state.update_visible_global(global, op, value)?;
                }
                PatchOp::SetScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                } => apply_set_scratch(game, state, x, y, object, scratch, value)?,
                PatchOp::RemoveScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                    match_value,
                } => apply_remove_scratch(game, state, x, y, object, scratch, value, match_value)?,
            }
        }

        state.recompute_hash();
        Ok(changed)
    }

    pub(crate) fn validate(&self, game: &CompiledGame, state: &State) -> Result<bool, PatchError> {
        let mut slots = SlotOverlay::new();
        validate_moves(game, state, &self.ops, &mut slots)?;
        let mut changed = slots.changed(game, state)?;

        for op in &self.ops {
            match *op {
                PatchOp::Add { x, y, object } => {
                    let layer = game
                        .object_layer(object)
                        .ok_or(PatchError::UnknownObject { object })?;
                    let existing = slots.get(game, state, x, y, layer)?;
                    if existing == object {
                        continue;
                    }
                    if !existing.is_empty() {
                        return Err(PatchError::LayerOccupied {
                            x,
                            y,
                            layer,
                            existing,
                            attempted: object,
                        });
                    }
                    slots.set(x, y, layer, object);
                    changed = true;
                }
                PatchOp::Remove { x, y, object } => {
                    let layer = game
                        .object_layer(object)
                        .ok_or(PatchError::UnknownObject { object })?;
                    let found = slots.get(game, state, x, y, layer)?;
                    if found != object {
                        return Err(PatchError::ExpectedObject {
                            x,
                            y,
                            layer,
                            expected: object,
                            found,
                        });
                    }
                    slots.set(x, y, layer, ObjectId::EMPTY);
                    changed = true;
                }
                PatchOp::Move { .. } => {}
                PatchOp::Replace { x, y, remove, add } => {
                    let remove_layer = game
                        .object_layer(remove)
                        .ok_or(PatchError::UnknownObject { object: remove })?;
                    let found = slots.get(game, state, x, y, remove_layer)?;
                    if found != remove {
                        return Err(PatchError::ExpectedObject {
                            x,
                            y,
                            layer: remove_layer,
                            expected: remove,
                            found,
                        });
                    }
                    slots.set(x, y, remove_layer, ObjectId::EMPTY);

                    let add_layer = game
                        .object_layer(add)
                        .ok_or(PatchError::UnknownObject { object: add })?;
                    let existing = slots.get(game, state, x, y, add_layer)?;
                    if existing == add {
                        continue;
                    }
                    if !existing.is_empty() {
                        return Err(PatchError::LayerOccupied {
                            x,
                            y,
                            layer: add_layer,
                            existing,
                            attempted: add,
                        });
                    }
                    slots.set(x, y, add_layer, add);
                    changed = true;
                }
                PatchOp::UpdateGlobal { global, op, value } => {
                    let next = validate_global_update(state, global, op, value)?;
                    changed |= state.global_value(global) != Some(next);
                }
                PatchOp::SetScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                } => {
                    if object.is_empty() {
                        changed |= !state.has_cell_scratch(x, y, scratch, value);
                    } else {
                        let layer = expect_object_in_overlay(game, state, &slots, x, y, object)?;
                        if state
                            .get_layer(x, y, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= !state.has_scratch(game, x, y, object, scratch, value);
                        } else {
                            changed = true;
                        }
                    }
                }
                PatchOp::RemoveScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                    match_value,
                } => {
                    let value = match match_value {
                        ScratchValueMatch::Any => None,
                        ScratchValueMatch::Exact => value,
                    };
                    if object.is_empty() {
                        changed |= match match_value {
                            ScratchValueMatch::Any => state.has_cell_scratch_key(x, y, scratch),
                            ScratchValueMatch::Exact => {
                                state.has_cell_scratch(x, y, scratch, value)
                            }
                        };
                    } else {
                        let layer = expect_object_in_overlay(game, state, &slots, x, y, object)?;
                        if state
                            .get_layer(x, y, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= match match_value {
                                ScratchValueMatch::Any => {
                                    state.has_scratch_key(game, x, y, object, scratch)
                                }
                                ScratchValueMatch::Exact => {
                                    state.has_scratch(game, x, y, object, scratch, value)
                                }
                            };
                        } else {
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(changed || slots.changed(game, state)?)
    }
}

#[derive(Default)]
struct SlotOverlay {
    slots: Vec<(u16, u16, LayerId, ObjectId)>,
}

impl SlotOverlay {
    fn new() -> Self {
        Self::default()
    }

    fn get(
        &self,
        _game: &CompiledGame,
        state: &State,
        x: u16,
        y: u16,
        layer: LayerId,
    ) -> Result<ObjectId, PatchError> {
        self.slots
            .iter()
            .rev()
            .find_map(|(slot_x, slot_y, slot_layer, object)| {
                (*slot_x == x && *slot_y == y && *slot_layer == layer).then_some(*object)
            })
            .map(Ok)
            .unwrap_or_else(|| state.get_layer(x, y, layer).map_err(PatchError::from))
    }

    fn set(&mut self, x: u16, y: u16, layer: LayerId, object: ObjectId) {
        if let Some((_, _, _, existing)) =
            self.slots
                .iter_mut()
                .rev()
                .find(|(slot_x, slot_y, slot_layer, _)| {
                    *slot_x == x && *slot_y == y && *slot_layer == layer
                })
        {
            *existing = object;
            return;
        }
        self.slots.push((x, y, layer, object));
    }

    fn changed(&self, _game: &CompiledGame, state: &State) -> Result<bool, PatchError> {
        self.slots
            .iter()
            .try_fold(false, |changed, (x, y, layer, object)| {
                Ok(changed || state.get_layer(*x, *y, *layer)? != *object)
            })
    }
}

fn apply_add(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
) -> Result<(), PatchError> {
    let layer = game
        .object_layer(object)
        .ok_or(PatchError::UnknownObject { object })?;
    let existing = state.get_layer(x, y, layer)?;
    if existing == object {
        return Ok(());
    }
    if !existing.is_empty() {
        return Err(PatchError::LayerOccupied {
            x,
            y,
            layer,
            existing,
            attempted: object,
        });
    }

    state.set_slot_unchecked(x, y, layer, object);
    Ok(())
}

fn validate_moves(
    game: &CompiledGame,
    state: &State,
    ops: &[PatchOp],
    slots: &mut SlotOverlay,
) -> Result<(), PatchError> {
    let mut sources = Vec::new();
    let mut destinations = Vec::new();
    let mut moves = Vec::new();

    for op in ops {
        let PatchOp::Move {
            from_x,
            from_y,
            to_x,
            to_y,
            object,
        } = *op
        else {
            continue;
        };
        let layer = game
            .object_layer(object)
            .ok_or(PatchError::UnknownObject { object })?;
        let found = state.get_layer(from_x, from_y, layer)?;
        if found != object {
            return Err(PatchError::ExpectedObject {
                x: from_x,
                y: from_y,
                layer,
                expected: object,
                found,
            });
        }
        if destinations.contains(&(to_x, to_y, layer)) {
            return Err(PatchError::LayerOccupied {
                x: to_x,
                y: to_y,
                layer,
                existing: object,
                attempted: object,
            });
        }
        sources.push((from_x, from_y, layer));
        destinations.push((to_x, to_y, layer));
        moves.push((from_x, from_y, to_x, to_y, layer, object));
    }

    for (_, _, to_x, to_y, layer, object) in &moves {
        let existing = state.get_layer(*to_x, *to_y, *layer)?;
        if !existing.is_empty() && !sources.contains(&(*to_x, *to_y, *layer)) {
            return Err(PatchError::LayerOccupied {
                x: *to_x,
                y: *to_y,
                layer: *layer,
                existing,
                attempted: *object,
            });
        }
    }

    for (from_x, from_y, _, _, layer, _) in &moves {
        slots.set(*from_x, *from_y, *layer, ObjectId::EMPTY);
    }
    for (_, _, to_x, to_y, layer, object) in moves {
        slots.set(to_x, to_y, layer, object);
    }

    Ok(())
}

fn apply_remove(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
) -> Result<(), PatchError> {
    let layer = game
        .object_layer(object)
        .ok_or(PatchError::UnknownObject { object })?;
    let found = state.get_layer(x, y, layer)?;
    if found != object {
        return Err(PatchError::ExpectedObject {
            x,
            y,
            layer,
            expected: object,
            found,
        });
    }

    state.set_slot_unchecked(x, y, layer, ObjectId::EMPTY);
    Ok(())
}

fn apply_moves(game: &CompiledGame, state: &mut State, ops: &[PatchOp]) -> Result<(), PatchError> {
    let mut moves = Vec::new();
    let mut sources = Vec::new();
    let mut destinations = Vec::new();

    for op in ops {
        let PatchOp::Move {
            from_x,
            from_y,
            to_x,
            to_y,
            object,
        } = *op
        else {
            continue;
        };
        let layer = game
            .object_layer(object)
            .ok_or(PatchError::UnknownObject { object })?;
        let found = state.get_layer(from_x, from_y, layer)?;
        if found != object {
            return Err(PatchError::ExpectedObject {
                x: from_x,
                y: from_y,
                layer,
                expected: object,
                found,
            });
        }
        if destinations.contains(&(to_x, to_y, layer)) {
            return Err(PatchError::LayerOccupied {
                x: to_x,
                y: to_y,
                layer,
                existing: object,
                attempted: object,
            });
        }
        sources.push((from_x, from_y, layer));
        destinations.push((to_x, to_y, layer));
        moves.push((from_x, from_y, to_x, to_y, layer, object));
    }

    for (_, _, to_x, to_y, layer, object) in &moves {
        let existing = state.get_layer(*to_x, *to_y, *layer)?;
        if !existing.is_empty() && !sources.contains(&(*to_x, *to_y, *layer)) {
            return Err(PatchError::LayerOccupied {
                x: *to_x,
                y: *to_y,
                layer: *layer,
                existing,
                attempted: *object,
            });
        }
    }

    let mut moved = Vec::with_capacity(moves.len());
    for (from_x, from_y, to_x, to_y, layer, object) in moves {
        let scratch = state.take_slot_for_move_unchecked(from_x, from_y, layer);
        moved.push((to_x, to_y, layer, object, scratch));
    }
    for (to_x, to_y, layer, object, scratch) in moved {
        state.place_moved_slot_unchecked(to_x, to_y, layer, object, scratch);
    }
    Ok(())
}

fn apply_set_scratch(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
    scratch: ScratchId,
    value: Option<i64>,
) -> Result<(), PatchError> {
    if object.is_empty() {
        state.set_cell_scratch_unchecked(x, y, scratch, value);
        return Ok(());
    }
    let layer = expect_object_at(game, state, x, y, object)?;
    state.set_scratch_unchecked(x, y, layer, scratch, value);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_remove_scratch(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
    scratch: ScratchId,
    value: Option<i64>,
    match_value: ScratchValueMatch,
) -> Result<(), PatchError> {
    if object.is_empty() {
        let value = match match_value {
            ScratchValueMatch::Any => None,
            ScratchValueMatch::Exact => value,
        };
        state.remove_cell_scratch_unchecked(x, y, scratch, value);
        return Ok(());
    }
    let layer = expect_object_at(game, state, x, y, object)?;
    let value = match match_value {
        ScratchValueMatch::Any => None,
        ScratchValueMatch::Exact => value,
    };
    state.remove_scratch_unchecked(x, y, layer, scratch, value);
    Ok(())
}

fn expect_object_at(
    game: &CompiledGame,
    state: &State,
    x: u16,
    y: u16,
    object: ObjectId,
) -> Result<LayerId, PatchError> {
    let layer = game
        .object_layer(object)
        .ok_or(PatchError::UnknownObject { object })?;
    let found = state.get_layer(x, y, layer)?;
    if found != object {
        return Err(PatchError::ExpectedObject {
            x,
            y,
            layer,
            expected: object,
            found,
        });
    }
    Ok(layer)
}

fn expect_object_in_overlay(
    game: &CompiledGame,
    state: &State,
    slots: &SlotOverlay,
    x: u16,
    y: u16,
    object: ObjectId,
) -> Result<LayerId, PatchError> {
    let layer = game
        .object_layer(object)
        .ok_or(PatchError::UnknownObject { object })?;
    let found = slots.get(game, state, x, y, layer)?;
    if found != object {
        return Err(PatchError::ExpectedObject {
            x,
            y,
            layer,
            expected: object,
            found,
        });
    }
    Ok(layer)
}

fn validate_global_update(
    state: &State,
    global: GlobalId,
    op: GlobalUpdateOp,
    value: i64,
) -> Result<i64, PatchError> {
    let current = state
        .global_value(global)
        .ok_or(StateError::GlobalOutOfBounds { global })?;
    let next = match op {
        GlobalUpdateOp::Set => Some(value),
        GlobalUpdateOp::Add => current.checked_add(value),
        GlobalUpdateOp::Subtract => current.checked_sub(value),
        GlobalUpdateOp::Multiply => current.checked_mul(value),
        GlobalUpdateOp::Divide => {
            if value == 0 {
                return Err(StateError::GlobalDivisionByZero { global }.into());
            }
            current.checked_div(value)
        }
        GlobalUpdateOp::Remainder => {
            if value == 0 {
                return Err(StateError::GlobalDivisionByZero { global }.into());
            }
            current.checked_rem(value)
        }
    }
    .ok_or(StateError::GlobalOverflow { global })?;
    Ok(next)
}
