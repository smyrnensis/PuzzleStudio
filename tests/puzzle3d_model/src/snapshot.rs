use crate::{Coord3, ObjectId, Size3, State3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardSnapshot3 {
    pub size: Size3,
    pub cells: Vec<BoardCell3>,
}

impl BoardSnapshot3 {
    pub fn from_state(state: &State3) -> Self {
        let mut cells = Vec::new();
        for z in 0..state.size.height {
            for y in 0..state.size.depth {
                for x in 0..state.size.width {
                    let position = Coord3 { x, y, z };
                    let objects = state
                        .cell_view(position)
                        .expect("scan only visits positions inside the state")
                        .objects;
                    if !objects.is_empty() {
                        cells.push(BoardCell3 { position, objects });
                    }
                }
            }
        }
        Self {
            size: state.size,
            cells,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardCell3 {
    pub position: Coord3,
    pub objects: Vec<ObjectId>,
}
