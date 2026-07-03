use crate::compiled_game::{CompiledGame, GlobalUpdateOp, MarkValueMatch};
use crate::ids::{GlobalId, LayerId, MarkId, ObjectId};
use crate::state::{State, StateError};
use puzzle_kernel::{GridCoord, GridPatchOp};

#[derive(Clone, Debug, Default)]
pub struct Patch {
    core: CorePatch,
    ops: Vec<PatchOp>,
}

pub(crate) type CorePatchOp = GridPatchOp<GridCoord<2>, ObjectId, GlobalId, MarkId>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CorePatch {
    pub(crate) ops: Vec<CorePatchOp>,
}

impl CorePatch {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, op: CorePatchOp) {
        self.ops.push(op);
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
                CorePatchOp::Add { position, object } => {
                    let (x, y) = coord_xy(position);
                    apply_add(game, state, x, y, object)?;
                }
                CorePatchOp::Remove { position, object } => {
                    let (x, y) = coord_xy(position);
                    apply_remove(game, state, x, y, object)?;
                }
                CorePatchOp::Move { .. } => {}
                CorePatchOp::Replace {
                    position,
                    remove,
                    add,
                } => {
                    let (x, y) = coord_xy(position);
                    apply_remove(game, state, x, y, remove)?;
                    apply_add(game, state, x, y, add)?;
                }
                CorePatchOp::UpdateGlobal { global, op, value } => {
                    state.update_visible_global(global, op, value)?;
                }
                CorePatchOp::SetMark {
                    position,
                    object,
                    mark,
                    value,
                } => {
                    let (x, y) = coord_xy(position);
                    apply_set_mark(game, state, x, y, object, mark, value)?;
                }
                CorePatchOp::RemoveMark {
                    position,
                    object,
                    mark,
                    value,
                    match_value,
                } => {
                    let (x, y) = coord_xy(position);
                    apply_remove_mark(game, state, x, y, object, mark, value, match_value)?;
                }
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
                CorePatchOp::Add { position, object } => {
                    let (x, y) = coord_xy(position);
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
                CorePatchOp::Remove { position, object } => {
                    let (x, y) = coord_xy(position);
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
                CorePatchOp::Move { .. } => {}
                CorePatchOp::Replace {
                    position,
                    remove,
                    add,
                } => {
                    let (x, y) = coord_xy(position);
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
                CorePatchOp::UpdateGlobal { global, op, value } => {
                    let next = validate_global_update(state, global, op, value)?;
                    changed |= state.global_value(global) != Some(next);
                }
                CorePatchOp::SetMark {
                    position,
                    object,
                    mark,
                    value,
                } => {
                    let (x, y) = coord_xy(position);
                    if object.is_empty() {
                        changed |= !state.has_cell_mark(x, y, mark, value);
                    } else {
                        let layer = expect_object_in_overlay(game, state, &slots, x, y, object)?;
                        if state
                            .get_layer(x, y, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= !state.has_mark(game, x, y, object, mark, value);
                        } else {
                            changed = true;
                        }
                    }
                }
                CorePatchOp::RemoveMark {
                    position,
                    object,
                    mark,
                    value,
                    match_value,
                } => {
                    let (x, y) = coord_xy(position);
                    let value = match match_value {
                        MarkValueMatch::Any => None,
                        MarkValueMatch::Exact => value,
                    };
                    if object.is_empty() {
                        changed |= match match_value {
                            MarkValueMatch::Any => state.has_cell_mark_key(x, y, mark),
                            MarkValueMatch::Exact => state.has_cell_mark(x, y, mark, value),
                        };
                    } else {
                        let layer = game
                            .object_layer(object)
                            .ok_or(PatchError::UnknownObject { object })?;
                        let found = slots.get(game, state, x, y, layer)?;
                        if found != object {
                            if state
                                .get_layer(x, y, layer)
                                .is_ok_and(|original| original == object)
                            {
                                changed = true;
                                continue;
                            }
                            return Err(PatchError::ExpectedObject {
                                x,
                                y,
                                layer,
                                expected: object,
                                found,
                            });
                        }
                        if state
                            .get_layer(x, y, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= match match_value {
                                MarkValueMatch::Any => state.has_mark_key(game, x, y, object, mark),
                                MarkValueMatch::Exact => {
                                    state.has_mark(game, x, y, object, mark, value)
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
    SetMark {
        x: u16,
        y: u16,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    },
    RemoveMark {
        x: u16,
        y: u16,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
        match_value: MarkValueMatch,
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

    pub fn ops(&self) -> &[PatchOp] {
        &self.ops
    }

    pub fn from_ops(ops: Vec<PatchOp>) -> Self {
        Self {
            core: CorePatch {
                ops: ops.iter().cloned().map(CorePatchOp::from).collect(),
            },
            ops,
        }
    }

    pub(crate) fn from_core(core: CorePatch) -> Self {
        let ops = core.ops.iter().cloned().map(PatchOp::from).collect();
        Self { core, ops }
    }

    #[allow(dead_code)]
    pub(crate) fn to_core(&self) -> CorePatch {
        self.core.clone()
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
        self.core.apply_in_place(game, state)
    }

    pub(crate) fn validate(&self, game: &CompiledGame, state: &State) -> Result<bool, PatchError> {
        self.core.validate(game, state)
    }
}

impl From<CorePatchOp> for PatchOp {
    fn from(value: CorePatchOp) -> Self {
        match value {
            CorePatchOp::Add { position, object } => {
                let [x, y] = position.axes();
                Self::Add { x, y, object }
            }
            CorePatchOp::Remove { position, object } => {
                let [x, y] = position.axes();
                Self::Remove { x, y, object }
            }
            CorePatchOp::Move { from, to, object } => {
                let [from_x, from_y] = from.axes();
                let [to_x, to_y] = to.axes();
                Self::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    object,
                }
            }
            CorePatchOp::Replace {
                position,
                remove,
                add,
            } => {
                let [x, y] = position.axes();
                Self::Replace { x, y, remove, add }
            }
            CorePatchOp::UpdateGlobal { global, op, value } => {
                Self::UpdateGlobal { global, op, value }
            }
            CorePatchOp::SetMark {
                position,
                object,
                mark,
                value,
            } => {
                let [x, y] = position.axes();
                Self::SetMark {
                    x,
                    y,
                    object,
                    mark,
                    value,
                }
            }
            CorePatchOp::RemoveMark {
                position,
                object,
                mark,
                value,
                match_value,
            } => {
                let [x, y] = position.axes();
                Self::RemoveMark {
                    x,
                    y,
                    object,
                    mark,
                    value,
                    match_value,
                }
            }
        }
    }
}

impl From<PatchOp> for CorePatchOp {
    fn from(value: PatchOp) -> Self {
        match value {
            PatchOp::Add { x, y, object } => Self::Add {
                position: GridCoord::new([x, y]),
                object,
            },
            PatchOp::Remove { x, y, object } => Self::Remove {
                position: GridCoord::new([x, y]),
                object,
            },
            PatchOp::Move {
                from_x,
                from_y,
                to_x,
                to_y,
                object,
            } => Self::Move {
                from: GridCoord::new([from_x, from_y]),
                to: GridCoord::new([to_x, to_y]),
                object,
            },
            PatchOp::Replace { x, y, remove, add } => Self::Replace {
                position: GridCoord::new([x, y]),
                remove,
                add,
            },
            PatchOp::UpdateGlobal { global, op, value } => Self::UpdateGlobal { global, op, value },
            PatchOp::SetMark {
                x,
                y,
                object,
                mark,
                value,
            } => Self::SetMark {
                position: GridCoord::new([x, y]),
                object,
                mark,
                value,
            },
            PatchOp::RemoveMark {
                x,
                y,
                object,
                mark,
                value,
                match_value,
            } => Self::RemoveMark {
                position: GridCoord::new([x, y]),
                object,
                mark,
                value,
                match_value,
            },
        }
    }
}

fn coord_xy(coord: GridCoord<2>) -> (u16, u16) {
    let [x, y] = coord.axes();
    (x, y)
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
    ops: &[CorePatchOp],
    slots: &mut SlotOverlay,
) -> Result<(), PatchError> {
    let mut sources = Vec::new();
    let mut destinations = Vec::new();
    let mut moves = Vec::new();

    for op in ops {
        let CorePatchOp::Move { from, to, object } = *op else {
            continue;
        };
        let (from_x, from_y) = coord_xy(from);
        let (to_x, to_y) = coord_xy(to);
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

fn apply_moves(
    game: &CompiledGame,
    state: &mut State,
    ops: &[CorePatchOp],
) -> Result<(), PatchError> {
    let mut moves = Vec::new();
    let mut sources = Vec::new();
    let mut destinations = Vec::new();

    for op in ops {
        let CorePatchOp::Move { from, to, object } = *op else {
            continue;
        };
        let (from_x, from_y) = coord_xy(from);
        let (to_x, to_y) = coord_xy(to);
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
        let mark = state.take_slot_for_move_unchecked(from_x, from_y, layer);
        moved.push((to_x, to_y, layer, object, mark));
    }
    for (to_x, to_y, layer, object, mark) in moved {
        state.place_moved_slot_unchecked(to_x, to_y, layer, object, mark);
    }
    Ok(())
}

fn apply_set_mark(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
) -> Result<(), PatchError> {
    if object.is_empty() {
        state.set_cell_mark_unchecked(x, y, mark, value);
        return Ok(());
    }
    let layer = expect_object_at(game, state, x, y, object)?;
    state.set_mark_unchecked(x, y, layer, mark, value);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_remove_mark(
    game: &CompiledGame,
    state: &mut State,
    x: u16,
    y: u16,
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
    match_value: MarkValueMatch,
) -> Result<(), PatchError> {
    if object.is_empty() {
        let value = match match_value {
            MarkValueMatch::Any => None,
            MarkValueMatch::Exact => value,
        };
        state.remove_cell_mark_unchecked(x, y, mark, value);
        return Ok(());
    }
    let Some(layer) = game.object_layer(object) else {
        return Err(PatchError::UnknownObject { object });
    };
    if state.get_layer(x, y, layer)? != object {
        return Ok(());
    }
    let value = match match_value {
        MarkValueMatch::Any => None,
        MarkValueMatch::Exact => value,
    };
    state.remove_mark_unchecked(x, y, layer, mark, value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled_game::ObjectDef;

    #[test]
    fn patch_round_trips_through_core_patch_ops() {
        let patch = Patch::from_ops(vec![
            PatchOp::Move {
                from_x: 1,
                from_y: 2,
                to_x: 3,
                to_y: 4,
                object: ObjectId(5),
            },
            PatchOp::RemoveMark {
                x: 6,
                y: 7,
                object: ObjectId(8),
                mark: MarkId(9),
                value: Some(10),
                match_value: MarkValueMatch::Exact,
            },
            PatchOp::UpdateGlobal {
                global: GlobalId(11),
                op: GlobalUpdateOp::Add,
                value: 12,
            },
        ]);

        assert_eq!(Patch::from_core(patch.to_core()).ops(), patch.ops());
    }

    #[test]
    fn mark_cleanup_after_same_patch_object_removal_is_noop() {
        let object = ObjectId(1);
        let mark = MarkId(0);
        let game = CompiledGame::new(
            1,
            vec![ObjectDef {
                id: object,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let mut state = State::empty(1, 1, 1, 1).unwrap();
        state.place_object(&game, 0, 0, object).unwrap();
        state.set_mark_unchecked(0, 0, LayerId(0), mark, Some(7));

        let patch = Patch::from_ops(vec![
            PatchOp::Remove { x: 0, y: 0, object },
            PatchOp::RemoveMark {
                x: 0,
                y: 0,
                object,
                mark,
                value: Some(7),
                match_value: MarkValueMatch::Exact,
            },
        ]);

        let next = patch.apply(&game, &state).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert!(next.slot_mark().iter().all(Vec::is_empty));
    }

    #[test]
    fn mark_cleanup_still_rejects_initially_missing_object() {
        let object = ObjectId(1);
        let game = CompiledGame::new(
            1,
            vec![ObjectDef {
                id: object,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let state = State::empty(1, 1, 1, 1).unwrap();
        let patch = Patch::from_ops(vec![PatchOp::RemoveMark {
            x: 0,
            y: 0,
            object,
            mark: MarkId(0),
            value: Some(7),
            match_value: MarkValueMatch::Exact,
        }]);

        assert!(matches!(
            patch.validate(&game, &state),
            Err(PatchError::ExpectedObject {
                x: 0,
                y: 0,
                layer: LayerId(0),
                expected,
                found: ObjectId::EMPTY,
            }) if expected == object
        ));
    }
}
