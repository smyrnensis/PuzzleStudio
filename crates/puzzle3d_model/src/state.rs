use crate::{Coord3, Game3, LayerId, ObjectId, RuleId3, Size3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State3 {
    pub size: Size3,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    level_fired_rules: Vec<RuleId3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateError3 {
    InvalidDimensions,
    PositionOutOfBounds {
        position: Coord3,
    },
    LayerOutOfBounds {
        layer: LayerId,
    },
    UnknownObject {
        object: ObjectId,
    },
    LayerOccupied {
        position: Coord3,
        layer: LayerId,
        existing: ObjectId,
        attempted: ObjectId,
    },
    ObjectNotPresent {
        position: Coord3,
        object: ObjectId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellView3 {
    pub objects: Vec<ObjectId>,
}

impl State3 {
    pub fn empty(size: Size3, layer_count: u16) -> Result<Self, StateError3> {
        if size.width == 0 || size.height == 0 || size.depth == 0 || layer_count == 0 {
            return Err(StateError3::InvalidDimensions);
        }
        let cell_count = usize::from(size.width)
            .checked_mul(usize::from(size.depth))
            .and_then(|count| count.checked_mul(usize::from(size.height)))
            .ok_or(StateError3::InvalidDimensions)?;
        let slot_count = cell_count
            .checked_mul(usize::from(layer_count))
            .ok_or(StateError3::InvalidDimensions)?;
        Ok(Self {
            size,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            level_fired_rules: Vec::new(),
        })
    }

    pub fn slots(&self) -> &[ObjectId] {
        &self.slots
    }

    pub fn level_fired_rules(&self) -> &[RuleId3] {
        &self.level_fired_rules
    }

    pub fn level_rule_has_fired(&self, rule: RuleId3) -> bool {
        self.level_fired_rules.binary_search(&rule).is_ok()
    }

    pub fn mark_level_rule_fired(&mut self, rule: RuleId3) {
        match self.level_fired_rules.binary_search(&rule) {
            Ok(_) => {}
            Err(index) => self.level_fired_rules.insert(index, rule),
        }
    }

    pub fn cell_view(&self, position: Coord3) -> Result<CellView3, StateError3> {
        self.check_pos(position)?;
        let mut objects = Vec::new();
        for layer in 0..self.layer_count {
            let object = self.get_layer(position, LayerId(layer))?;
            if !object.is_empty() {
                objects.push(object);
            }
        }
        Ok(CellView3 { objects })
    }

    pub fn get_layer(&self, position: Coord3, layer: LayerId) -> Result<ObjectId, StateError3> {
        let index = self.slot_index(position, layer)?;
        Ok(self.slots[index])
    }

    pub fn has_object(&self, game: &Game3, position: Coord3, object: ObjectId) -> bool {
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        self.get_layer(position, layer)
            .is_ok_and(|actual| actual == object)
    }

    pub fn place_object(
        &mut self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
    ) -> Result<(), StateError3> {
        let layer = checked_object_layer(game, object)?;
        let index = self.slot_index(position, layer)?;
        let existing = self.slots[index];
        if !existing.is_empty() && existing != object {
            return Err(StateError3::LayerOccupied {
                position,
                layer,
                existing,
                attempted: object,
            });
        }
        self.slots[index] = object;
        Ok(())
    }

    pub fn remove_object(
        &mut self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
    ) -> Result<(), StateError3> {
        let layer = checked_object_layer(game, object)?;
        let index = self.slot_index(position, layer)?;
        if self.slots[index] != object {
            return Err(StateError3::ObjectNotPresent { position, object });
        }
        self.slots[index] = ObjectId::EMPTY;
        Ok(())
    }

    pub(crate) fn check_pos(&self, position: Coord3) -> Result<(), StateError3> {
        if position.x >= self.size.width
            || position.y >= self.size.depth
            || position.z >= self.size.height
        {
            return Err(StateError3::PositionOutOfBounds { position });
        }
        Ok(())
    }

    pub(crate) fn slot_index(
        &self,
        position: Coord3,
        layer: LayerId,
    ) -> Result<usize, StateError3> {
        self.check_pos(position)?;
        if layer.0 >= self.layer_count {
            return Err(StateError3::LayerOutOfBounds { layer });
        }
        Ok(self.slot_index_unchecked(position, layer))
    }

    pub(crate) fn slot_index_unchecked(&self, position: Coord3, layer: LayerId) -> usize {
        (self.cell_index_unchecked(position) * usize::from(self.layer_count)) + usize::from(layer.0)
    }

    pub(crate) fn cell_index_unchecked(&self, position: Coord3) -> usize {
        ((usize::from(position.z) * usize::from(self.size.depth)) + usize::from(position.y))
            * usize::from(self.size.width)
            + usize::from(position.x)
    }
}

fn checked_object_layer(game: &Game3, object: ObjectId) -> Result<LayerId, StateError3> {
    game.object_layer(object)
        .ok_or(StateError3::UnknownObject { object })
}
