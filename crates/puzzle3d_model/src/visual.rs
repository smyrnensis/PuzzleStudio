use crate::{BoardSnapshot3, Coord3, ObjectId, Size3, State3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectVisual3 {
    pub id: ObjectId,
    pub name: String,
    pub sprite: String,
}

impl ObjectVisual3 {
    pub fn new(id: ObjectId, name: impl Into<String>, sprite: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            sprite: sprite.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualSnapshot3 {
    pub size: Size3,
    pub cells: Vec<VisualCell3>,
}

impl VisualSnapshot3 {
    pub fn from_state(state: &State3, visuals: &[ObjectVisual3]) -> Self {
        Self::from_board(&BoardSnapshot3::from_state(state), visuals)
    }

    pub fn from_board(board: &BoardSnapshot3, visuals: &[ObjectVisual3]) -> Self {
        let cells = board
            .cells
            .iter()
            .map(|cell| {
                let objects = cell
                    .objects
                    .iter()
                    .copied()
                    .map(|id| visual_object(id, visuals))
                    .collect();
                VisualCell3 {
                    position: cell.position,
                    objects,
                }
            })
            .collect();
        Self {
            size: board.size,
            cells,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualCell3 {
    pub position: Coord3,
    pub objects: Vec<VisualObject3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualObject3 {
    pub id: ObjectId,
    pub name: String,
    pub sprite: Option<String>,
}

fn visual_object(id: ObjectId, visuals: &[ObjectVisual3]) -> VisualObject3 {
    if let Some(visual) = visuals.iter().find(|visual| visual.id == id) {
        return VisualObject3 {
            id,
            name: visual.name.clone(),
            sprite: Some(visual.sprite.clone()),
        };
    }
    VisualObject3 {
        id,
        name: format!("object_{}", id.0),
        sprite: None,
    }
}
