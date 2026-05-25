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

        apply_moves(game, &mut next, &self.ops)?;

        for op in &self.ops {
            match *op {
                PatchOp::Add { x, y, object } => apply_add(game, &mut next, x, y, object)?,
                PatchOp::Remove { x, y, object } => apply_remove(game, &mut next, x, y, object)?,
                PatchOp::Move { .. } => {}
                PatchOp::Replace { x, y, remove, add } => {
                    apply_remove(game, &mut next, x, y, remove)?;
                    apply_add(game, &mut next, x, y, add)?;
                }
                PatchOp::UpdateGlobal { global, op, value } => {
                    next.update_visible_global(global, op, value)?;
                }
                PatchOp::SetScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                } => apply_set_scratch(game, &mut next, x, y, object, scratch, value)?,
                PatchOp::RemoveScratch {
                    x,
                    y,
                    object,
                    scratch,
                    value,
                    match_value,
                } => apply_remove_scratch(
                    game,
                    &mut next,
                    x,
                    y,
                    object,
                    scratch,
                    value,
                    match_value,
                )?,
            }
        }

        next.recompute_hash();
        Ok(next)
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
