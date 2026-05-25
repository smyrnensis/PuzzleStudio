use crate::{Coord3, Game3, ObjectId, State3, StateError3};

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
        let mut next = state.clone();
        for op in &self.ops {
            apply_remove_phase(game, &mut next, op)?;
        }
        for op in &self.ops {
            apply_add_phase(game, &mut next, op)?;
        }
        *state = next;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchOp3 {
    Add {
        position: Coord3,
        object: ObjectId,
    },
    Remove {
        position: Coord3,
        object: ObjectId,
    },
    Replace {
        position: Coord3,
        remove: ObjectId,
        add: ObjectId,
    },
    Move {
        from: Coord3,
        to: Coord3,
        object: ObjectId,
    },
}

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
        PatchOp3::Add { .. } => {}
        PatchOp3::Remove { position, object }
        | PatchOp3::Move {
            from: position,
            object,
            ..
        } => {
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
        PatchOp3::Add { position, object }
        | PatchOp3::Move {
            to: position,
            object,
            ..
        } => {
            state.place_object(game, position, object)?;
        }
        PatchOp3::Remove { .. } => {}
        PatchOp3::Replace { position, add, .. } => {
            state.place_object(game, position, add)?;
        }
    }
    Ok(())
}
