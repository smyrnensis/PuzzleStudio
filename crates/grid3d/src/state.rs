use crate::{Coord3, Game3, LayerId, MarkId3, ObjectId, RuleId3, Size3, VariableId};
use puzzle_kernel::{
    FnvBuilder, GridShape, MarkSpace, MarkValue, ObjectCellMask, VariableUpdateOp,
    VariableValueError, VisibleVariables, fnv_mix,
};

#[derive(Clone, Debug)]
pub struct State3 {
    pub size: Size3,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    mark: MarkSpace<MarkId3>,
    visible_variables: VisibleVariables<VariableId>,
    level_fired_rules: Vec<RuleId3>,
    cell_object_masks: Vec<ObjectCellMask>,
    hash: u64,
}

pub type SlotMark3 = MarkValue<MarkId3>;

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
    VariableOutOfBounds {
        variable: VariableId,
    },
    VariableOverflow {
        variable: VariableId,
    },
    VariableDivisionByZero {
        variable: VariableId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellView3 {
    pub objects: Vec<ObjectId>,
}

impl State3 {
    pub fn empty(size: Size3, layer_count: u16) -> Result<Self, StateError3> {
        Self::empty_with_variables(size, layer_count, Vec::new())
    }

    pub fn empty_with_variables(
        size: Size3,
        layer_count: u16,
        visible_variables: Vec<i64>,
    ) -> Result<Self, StateError3> {
        let shape = grid_shape(size, layer_count).ok_or(StateError3::InvalidDimensions)?;
        let cell_count = shape.cell_count().ok_or(StateError3::InvalidDimensions)?;
        let slot_count = shape.slot_count().ok_or(StateError3::InvalidDimensions)?;
        let mut state = Self {
            size,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            mark: MarkSpace::new(cell_count, slot_count),
            visible_variables: VisibleVariables::new(visible_variables),
            level_fired_rules: Vec::new(),
            cell_object_masks: vec![ObjectCellMask::default(); cell_count],
            hash: 0,
        };
        state.recompute_hash();
        Ok(state)
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn slots(&self) -> &[ObjectId] {
        &self.slots
    }

    pub fn slot_mark(&self) -> Vec<Vec<SlotMark3>> {
        self.mark.slot_values()
    }

    pub fn cell_mark(&self) -> Vec<Vec<SlotMark3>> {
        self.mark.cell_values()
    }

    pub fn visible_variables(&self) -> &[i64] {
        self.visible_variables.as_slice()
    }

    pub fn variable_value(&self, variable: VariableId) -> Option<i64> {
        self.visible_variables.get(variable)
    }

    pub fn set_visible_variable(
        &mut self,
        variable: VariableId,
        value: i64,
    ) -> Result<(), StateError3> {
        self.visible_variables
            .set(variable, value)
            .map_err(map_variable_error)?;
        self.recompute_hash();
        Ok(())
    }

    pub fn update_visible_variable(
        &mut self,
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    ) -> Result<(), StateError3> {
        self.visible_variables
            .update(variable, op, value)
            .map_err(map_variable_error)?;
        self.recompute_hash();
        Ok(())
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
        self.recompute_hash();
    }

    pub fn without_objects(&self, objects: &[ObjectId]) -> Self {
        if objects.is_empty()
            || self
                .slots
                .iter()
                .all(|object| object.is_empty() || !objects.contains(object))
        {
            return self.clone();
        }

        let mut next = self.clone();
        for index in 0..next.slots.len() {
            let object = next.slots[index];
            if object.is_empty() || !objects.contains(&object) {
                continue;
            }
            next.set_slot_index_unchecked(index, ObjectId::EMPTY);
            next.mark.clear_slot(index);
        }
        next.recompute_hash();
        next
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

    #[inline]
    pub(crate) fn cell_has_object_masked(
        &self,
        position: Coord3,
        object: ObjectId,
    ) -> Option<bool> {
        if self.check_pos(position).is_err() {
            return None;
        }
        let index = self.cell_index_unchecked(position);
        self.cell_object_masks[index].contains_raw(object.0)
    }

    pub(crate) fn object_count_masked(&self, object: ObjectId) -> Option<u32> {
        let mut count = 0;
        for mask in &self.cell_object_masks {
            if mask.contains_raw(object.0)? {
                count += 1;
            }
        }
        Some(count)
    }

    pub fn has_mark(
        &self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
        mark: MarkId3,
        value: Option<i64>,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_mark(position, mark, value);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.mark.has_slot(index, mark, value)
    }

    pub fn has_mark_key(
        &self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
        mark: MarkId3,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_mark_key(position, mark);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.mark.has_slot_key(index, mark)
    }

    pub fn has_cell_mark(&self, position: Coord3, mark: MarkId3, value: Option<i64>) -> bool {
        if self.check_pos(position).is_err() {
            return false;
        }
        let index = self.cell_index_unchecked(position);
        self.mark.has_cell(index, mark, value)
    }

    pub fn has_cell_mark_key(&self, position: Coord3, mark: MarkId3) -> bool {
        if self.check_pos(position).is_err() {
            return false;
        }
        let index = self.cell_index_unchecked(position);
        self.mark.has_cell_key(index, mark)
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
        self.set_slot_index_unchecked(index, object);
        self.mark.clear_slot(index);
        self.recompute_hash();
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
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.mark.clear_slot(index);
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
    ) -> Vec<SlotMark3> {
        let index = self.slot_index_unchecked(position, layer);
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.mark.take_slot(index)
    }

    pub(crate) fn place_moved_slot_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        object: ObjectId,
        mark: Vec<SlotMark3>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.set_slot_index_unchecked(index, object);
        self.mark.replace_slot(index, mark);
        self.recompute_hash();
    }

    pub(crate) fn set_mark_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        mark: MarkId3,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.mark.set_slot(index, mark, value);
        self.recompute_hash();
    }

    pub(crate) fn set_cell_mark_unchecked(
        &mut self,
        position: Coord3,
        mark: MarkId3,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.mark.set_cell(index, mark, value);
        self.recompute_hash();
    }

    pub(crate) fn remove_mark_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        mark: MarkId3,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.mark.remove_slot(index, mark, value);
        self.recompute_hash();
    }

    pub(crate) fn remove_cell_mark_unchecked(
        &mut self,
        position: Coord3,
        mark: MarkId3,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.mark.remove_cell(index, mark, value);
        self.recompute_hash();
    }

    pub(crate) fn clear_mark(&mut self) {
        if self.mark.is_empty() {
            return;
        }
        self.mark.clear_all();
        self.recompute_hash();
    }

    pub(crate) fn recompute_hash(&mut self) {
        let mut hash = FnvBuilder::OFFSET;
        hash = fnv_mix(hash, u64::from(self.size.width));
        hash = fnv_mix(hash, u64::from(self.size.depth));
        hash = fnv_mix(hash, u64::from(self.size.height));
        hash = fnv_mix(hash, u64::from(self.layer_count));
        for object in &self.slots {
            hash = fnv_mix(hash, u64::from(object.0));
        }
        hash = self.mark.hash_into(hash, |mark| u64::from(mark.0));
        for value in self.visible_variables.as_slice() {
            hash = fnv_mix(hash, *value as u64);
        }
        hash = fnv_mix(hash, self.level_fired_rules.len() as u64);
        for rule in &self.level_fired_rules {
            hash = fnv_mix(hash, u64::from(rule.0));
        }
        self.hash = hash;
    }

    fn set_slot_index_unchecked(&mut self, index: usize, object: ObjectId) {
        let existing = self.slots[index];
        if existing == object {
            return;
        }
        self.clear_cell_object_mask(index, existing);
        self.slots[index] = object;
        self.set_cell_object_mask(index, object);
    }

    fn set_cell_object_mask(&mut self, slot_index: usize, object: ObjectId) {
        let cell_index = slot_index / usize::from(self.layer_count);
        self.cell_object_masks[cell_index].insert_raw(object.0);
    }

    fn clear_cell_object_mask(&mut self, slot_index: usize, object: ObjectId) {
        let cell_index = slot_index / usize::from(self.layer_count);
        self.cell_object_masks[cell_index].remove_raw(object.0);
    }

    pub(crate) fn check_pos(&self, position: Coord3) -> Result<(), StateError3> {
        if !self.shape().contains(position.into()) {
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
        self.shape()
            .slot_index(position.into(), layer.0)
            .ok_or(StateError3::LayerOutOfBounds { layer })
    }

    pub(crate) fn slot_index_unchecked(&self, position: Coord3, layer: LayerId) -> usize {
        self.shape().slot_index_unchecked(position.into(), layer.0)
    }

    pub(crate) fn cell_index_unchecked(&self, position: Coord3) -> usize {
        self.shape().cell_index_unchecked(position.into())
    }

    fn shape(&self) -> GridShape<3> {
        grid_shape(self.size, self.layer_count)
            .expect("state dimensions are validated at construction")
    }
}

fn grid_shape(size: Size3, layer_count: u16) -> Option<GridShape<3>> {
    GridShape::new([size.width, size.depth, size.height], layer_count)
}

fn checked_object_layer(game: &Game3, object: ObjectId) -> Result<LayerId, StateError3> {
    game.object_layer(object)
        .ok_or(StateError3::UnknownObject { object })
}

fn map_variable_error(error: VariableValueError<VariableId>) -> StateError3 {
    match error {
        VariableValueError::OutOfBounds { variable } => {
            StateError3::VariableOutOfBounds { variable }
        }
        VariableValueError::Overflow { variable } => StateError3::VariableOverflow { variable },
        VariableValueError::DivisionByZero { variable } => {
            StateError3::VariableDivisionByZero { variable }
        }
    }
}

impl PartialEq for State3 {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.mark == other.mark
            && self.visible_variables == other.visible_variables
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl Eq for State3 {}
