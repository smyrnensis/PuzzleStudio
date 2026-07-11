use crate::compiled_game::{CompiledGame, VariableUpdateOp};
use crate::ids::{LayerId, MarkId, ObjectId, RuleId, VariableId};
use puzzle_kernel::{
    GridCoord, GridShape, MarkSpace, ObjectCellMask, VariableValueError, VisibleVariables, fnv_mix,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct State {
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    mark: MarkSpace<MarkId>,
    visible_variables: VisibleVariables<VariableId>,
    level_fired_rules: Vec<RuleId>,
    #[serde(skip)]
    derived_cache: DerivedCache,
    #[serde(skip)]
    hash: u64,
}

pub type SlotMark = puzzle_kernel::MarkValue<MarkId>;
pub type SlotMarkIter<'a> = puzzle_kernel::MarkIter<'a, MarkId>;

#[derive(Clone, Debug, Default)]
struct DerivedCache {
    object_counts: Vec<u32>,
    object_positions: Vec<Vec<usize>>,
    cell_object_masks: Vec<ObjectCellMask>,
    mark_positions: BTreeMap<MarkPositionKey, Vec<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MarkPositionKey {
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StateData {
            width: u16,
            height: u16,
            layer_count: u16,
            slots: Vec<ObjectId>,
            mark: MarkSpace<MarkId>,
            visible_variables: VisibleVariables<VariableId>,
            level_fired_rules: Vec<RuleId>,
        }

        let data = StateData::deserialize(deserializer)?;
        let mut state = State {
            width: data.width,
            height: data.height,
            layer_count: data.layer_count,
            slots: data.slots,
            mark: data.mark,
            visible_variables: data.visible_variables,
            level_fired_rules: data.level_fired_rules,
            derived_cache: DerivedCache::default(),
            hash: 0,
        };
        state.rebuild_derived_cache();
        state.recompute_hash();
        Ok(state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateError {
    InvalidDimensions,
    PositionOutOfBounds {
        x: u16,
        y: u16,
    },
    LayerOutOfBounds {
        layer: LayerId,
    },
    UnknownObject {
        object: ObjectId,
    },
    LayerOccupied {
        x: u16,
        y: u16,
        layer: LayerId,
        existing: ObjectId,
        attempted: ObjectId,
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
pub struct CellView {
    pub objects: Vec<ObjectId>,
}

impl State {
    pub fn empty(
        width: u16,
        height: u16,
        layer_count: u16,
        object_count: usize,
    ) -> Result<Self, StateError> {
        Self::empty_with_variables(width, height, layer_count, object_count, Vec::new())
    }

    pub fn empty_with_variables(
        width: u16,
        height: u16,
        layer_count: u16,
        object_count: usize,
        visible_variables: Vec<i64>,
    ) -> Result<Self, StateError> {
        let shape = GridShape::<2>::new([width, height], layer_count)
            .ok_or(StateError::InvalidDimensions)?;
        let slot_count = shape.slot_count().ok_or(StateError::InvalidDimensions)?;
        let cell_count = shape.cell_count().ok_or(StateError::InvalidDimensions)?;
        if width == 0 || height == 0 || layer_count == 0 {
            return Err(StateError::InvalidDimensions);
        }

        let mut state = Self {
            width,
            height,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            mark: MarkSpace::new(cell_count, slot_count),
            visible_variables: VisibleVariables::new(visible_variables),
            level_fired_rules: Vec::new(),
            derived_cache: DerivedCache {
                object_counts: vec![0; object_count + 1],
                object_positions: vec![Vec::new(); object_count + 1],
                cell_object_masks: vec![ObjectCellMask::default(); cell_count],
                mark_positions: BTreeMap::new(),
            },
            hash: 0,
        };
        state.recompute_hash();
        Ok(state)
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn slots(&self) -> &[ObjectId] {
        &self.slots
    }

    pub fn slot_mark(&self) -> Vec<Vec<SlotMark>> {
        self.mark.slot_values()
    }

    pub fn cell_mark(&self) -> Vec<Vec<SlotMark>> {
        self.mark.cell_values()
    }

    #[inline]
    pub fn cell_mark_at(&self, index: usize) -> SlotMarkIter<'_> {
        self.mark.cell_at(index)
    }

    #[inline]
    pub fn slot_mark_at(&self, index: usize) -> SlotMarkIter<'_> {
        self.mark.slot_at(index)
    }

    #[inline]
    pub fn visible_variables(&self) -> &[i64] {
        self.visible_variables.as_slice()
    }

    #[inline]
    pub fn level_fired_rules(&self) -> &[RuleId] {
        &self.level_fired_rules
    }

    #[inline]
    pub fn level_rule_has_fired(&self, rule: RuleId) -> bool {
        self.level_fired_rules.binary_search(&rule).is_ok()
    }

    pub fn mark_level_rule_fired(&mut self, rule: RuleId) {
        match self.level_fired_rules.binary_search(&rule) {
            Ok(_) => {}
            Err(index) => self.level_fired_rules.insert(index, rule),
        }
        self.recompute_hash();
    }

    pub fn without_objects(&self, objects: &[ObjectId]) -> Self {
        if objects.is_empty() {
            return self.clone();
        }
        if self
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

    #[inline]
    pub fn variable_value(&self, variable: VariableId) -> Option<i64> {
        self.visible_variables.get(variable)
    }

    pub fn set_visible_variable(
        &mut self,
        variable: VariableId,
        value: i64,
    ) -> Result<(), StateError> {
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
    ) -> Result<(), StateError> {
        self.visible_variables
            .update(variable, op, value)
            .map_err(map_variable_error)?;
        self.recompute_hash();
        Ok(())
    }

    #[inline]
    pub fn object_count(&self, object: ObjectId) -> u32 {
        self.derived_cache
            .object_counts
            .get(usize::from(object.0))
            .copied()
            .unwrap_or(0)
    }

    #[inline]
    pub fn object_positions(&self, object: ObjectId) -> &[usize] {
        self.derived_cache
            .object_positions
            .get(usize::from(object.0))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[inline]
    pub fn slot_position(&self, index: usize) -> Option<(u16, u16)> {
        if index >= self.slots.len() {
            return None;
        }
        let cell = index / usize::from(self.layer_count);
        let x = cell % usize::from(self.width);
        let y = cell / usize::from(self.width);
        Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
    }

    #[inline]
    pub fn cell_position(&self, index: usize) -> Option<(u16, u16)> {
        let cell_count = usize::from(self.width) * usize::from(self.height);
        if index >= cell_count {
            return None;
        }
        let x = index % usize::from(self.width);
        let y = index / usize::from(self.width);
        Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
    }

    #[inline]
    pub fn mark_positions(&self, object: ObjectId, mark: MarkId, value: Option<i64>) -> &[usize] {
        let key = MarkPositionKey {
            object,
            mark,
            value,
        };
        self.derived_cache
            .mark_positions
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn cell_view(&self, x: u16, y: u16) -> Result<CellView, StateError> {
        self.check_pos(x, y)?;

        let mut objects = Vec::new();
        for layer in 0..self.layer_count {
            let object = self.slots[self.slot_index_unchecked(x, y, LayerId(layer))];
            if !object.is_empty() {
                objects.push(object);
            }
        }

        Ok(CellView { objects })
    }

    pub fn get_layer(&self, x: u16, y: u16, layer: LayerId) -> Result<ObjectId, StateError> {
        let index = self.slot_index(x, y, layer)?;
        Ok(self.slots[index])
    }

    pub fn has_object(&self, game: &CompiledGame, x: u16, y: u16, object: ObjectId) -> bool {
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        self.get_layer(x, y, layer)
            .is_ok_and(|found| found == object)
    }

    #[inline]
    pub(crate) fn cell_has_object_masked(&self, x: u16, y: u16, object: ObjectId) -> Option<bool> {
        let index = self.cell_index(x, y).ok()?;
        self.derived_cache.cell_object_masks[index].contains_raw(object.0)
    }

    pub fn has_mark(
        &self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_mark(x, y, mark, value);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(x, y, layer) else {
            return false;
        };
        self.slots[index] == object && self.mark.has_slot(index, mark, value)
    }

    pub fn has_mark_key(
        &self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
        mark: MarkId,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_mark_key(x, y, mark);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(x, y, layer) else {
            return false;
        };
        self.slots[index] == object && self.mark.has_slot_key(index, mark)
    }

    pub fn has_cell_mark(&self, x: u16, y: u16, mark: MarkId, value: Option<i64>) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.mark.has_cell(index, mark, value)
    }

    pub fn has_cell_mark_key(&self, x: u16, y: u16, mark: MarkId) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.mark.has_cell_key(index, mark)
    }

    pub fn place_object(
        &mut self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
    ) -> Result<(), StateError> {
        let layer = game
            .object_layer(object)
            .ok_or(StateError::UnknownObject { object })?;
        let index = self.slot_index(x, y, layer)?;
        let existing = self.slots[index];
        if !existing.is_empty() {
            return Err(StateError::LayerOccupied {
                x,
                y,
                layer,
                existing,
                attempted: object,
            });
        }

        self.set_slot_index_unchecked(index, object);
        self.clear_slot_mark_positions(index, existing);
        self.mark.clear_slot(index);
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn set_slot_unchecked(&mut self, x: u16, y: u16, layer: LayerId, object: ObjectId) {
        let index = self.slot_index_unchecked(x, y, layer);
        let existing = self.slots[index];
        if existing != object {
            self.clear_slot_mark_positions(index, existing);
            self.mark.clear_slot(index);
        }
        self.set_slot_index_unchecked(index, object);
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
    ) -> Vec<SlotMark> {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.clear_slot_mark_positions(index, object);
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.mark.take_slot(index)
    }

    pub(crate) fn place_moved_slot_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        object: ObjectId,
        mark: Vec<SlotMark>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let existing = self.slots[index];
        self.clear_slot_mark_positions(index, existing);
        self.set_slot_index_unchecked(index, object);
        self.mark.replace_slot(index, mark);
        self.add_slot_mark_positions(index, object);
    }

    pub(crate) fn set_mark_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.remove_slot_mark_key_positions(index, object, mark);
        self.mark.set_slot(index, mark, value);
        self.add_mark_position(object, mark, value, index);
    }

    pub(crate) fn set_cell_mark_unchecked(
        &mut self,
        x: u16,
        y: u16,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        self.remove_cell_mark_key_positions(index, mark);
        self.mark.set_cell(index, mark, value);
        self.add_mark_position(ObjectId::EMPTY, mark, value, index);
    }

    pub(crate) fn remove_mark_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.remove_matching_slot_mark_positions(index, object, mark, value);
        self.mark.remove_slot(index, mark, value);
    }

    pub(crate) fn remove_cell_mark_unchecked(
        &mut self,
        x: u16,
        y: u16,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        self.remove_matching_cell_mark_positions(index, mark, value);
        self.mark.remove_cell(index, mark, value);
    }

    pub(crate) fn clear_mark(&mut self) {
        if self.mark.is_empty() {
            return;
        }
        self.mark.clear_all();
        self.derived_cache.mark_positions.clear();
        self.recompute_hash();
    }

    fn set_slot_index_unchecked(&mut self, index: usize, object: ObjectId) {
        let existing = self.slots[index];
        if existing == object {
            return;
        }
        self.remove_object_position(existing, index);
        self.clear_cell_object_mask(index, existing);
        self.slots[index] = object;
        self.add_object_position(object, index);
        self.set_cell_object_mask(index, object);
    }

    fn add_object_position(&mut self, object: ObjectId, slot_index: usize) {
        if object.is_empty() {
            return;
        }
        let index = usize::from(object.0);
        if index >= self.derived_cache.object_counts.len() {
            self.derived_cache.object_counts.resize(index + 1, 0);
            self.derived_cache
                .object_positions
                .resize(index + 1, Vec::new());
        }
        self.derived_cache.object_counts[index] += 1;
        self.derived_cache.object_positions[index].push(slot_index);
    }

    fn rebuild_derived_cache(&mut self) {
        let object_count = self
            .slots
            .iter()
            .map(|object| usize::from(object.0))
            .max()
            .unwrap_or(0);
        let cell_count = usize::from(self.width) * usize::from(self.height);
        self.derived_cache = DerivedCache {
            object_counts: vec![0; object_count + 1],
            object_positions: vec![Vec::new(); object_count + 1],
            cell_object_masks: vec![ObjectCellMask::default(); cell_count],
            mark_positions: BTreeMap::new(),
        };
        let mut index = 0;
        while index < self.slots.len() {
            let object = self.slots[index];
            self.add_object_position(object, index);
            if !object.is_empty() {
                self.set_cell_object_mask(index, object);
            }
            self.add_slot_mark_positions(index, object);
            index += 1;
        }
        for cell_index in 0..cell_count {
            let entries = self.cell_mark_at(cell_index).collect::<Vec<_>>();
            for entry in entries {
                self.add_mark_position(ObjectId::EMPTY, entry.mark, entry.value, cell_index);
            }
        }
    }

    fn remove_object_position(&mut self, object: ObjectId, slot_index: usize) {
        if object.is_empty() {
            return;
        }
        let index = usize::from(object.0);
        if let Some(count) = self.derived_cache.object_counts.get_mut(index) {
            *count = count.saturating_sub(1);
        }
        if let Some(positions) = self.derived_cache.object_positions.get_mut(index)
            && let Some(position_index) = positions
                .iter()
                .position(|position| *position == slot_index)
        {
            positions.swap_remove(position_index);
        }
    }

    fn set_cell_object_mask(&mut self, slot_index: usize, object: ObjectId) {
        let cell_index = slot_index / usize::from(self.layer_count);
        self.derived_cache.cell_object_masks[cell_index].insert_raw(object.0);
    }

    fn clear_cell_object_mask(&mut self, slot_index: usize, object: ObjectId) {
        let cell_index = slot_index / usize::from(self.layer_count);
        self.derived_cache.cell_object_masks[cell_index].remove_raw(object.0);
    }

    fn add_slot_mark_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        let mark = self.slot_mark_at(index).collect::<Vec<_>>();
        for entry in mark {
            self.add_mark_position(object, entry.mark, entry.value, index);
        }
    }

    fn clear_slot_mark_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        let mark = self.slot_mark_at(index).collect::<Vec<_>>();
        for entry in mark {
            self.remove_mark_position(object, entry.mark, entry.value, index);
        }
    }

    fn remove_slot_mark_key_positions(&mut self, index: usize, object: ObjectId, mark: MarkId) {
        if object.is_empty() {
            return;
        }
        let mark_entries = self.slot_mark_at(index).collect::<Vec<_>>();
        for entry in mark_entries {
            if entry.mark == mark {
                self.remove_mark_position(object, entry.mark, entry.value, index);
            }
        }
    }

    fn remove_matching_slot_mark_positions(
        &mut self,
        index: usize,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    ) {
        if object.is_empty() {
            return;
        }
        let mark_entries = self.slot_mark_at(index).collect::<Vec<_>>();
        for entry in mark_entries {
            if entry.mark == mark && (value.is_none() || entry.value == value) {
                self.remove_mark_position(object, entry.mark, entry.value, index);
            }
        }
    }

    fn remove_cell_mark_key_positions(&mut self, index: usize, mark: MarkId) {
        let mark_entries = self.cell_mark_at(index).collect::<Vec<_>>();
        for entry in mark_entries {
            if entry.mark == mark {
                self.remove_mark_position(ObjectId::EMPTY, entry.mark, entry.value, index);
            }
        }
    }

    fn remove_matching_cell_mark_positions(
        &mut self,
        index: usize,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let mark_entries = self.cell_mark_at(index).collect::<Vec<_>>();
        for entry in mark_entries {
            if entry.mark == mark && (value.is_none() || entry.value == value) {
                self.remove_mark_position(ObjectId::EMPTY, entry.mark, entry.value, index);
            }
        }
    }

    fn add_mark_position(
        &mut self,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
        index: usize,
    ) {
        let key = MarkPositionKey {
            object,
            mark,
            value,
        };
        let positions = self.derived_cache.mark_positions.entry(key).or_default();
        if !positions.contains(&index) {
            positions.push(index);
        }
    }

    fn remove_mark_position(
        &mut self,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
        index: usize,
    ) {
        let key = MarkPositionKey {
            object,
            mark,
            value,
        };
        if let Some(positions) = self.derived_cache.mark_positions.get_mut(&key) {
            if let Some(position_index) = positions.iter().position(|position| *position == index) {
                positions.swap_remove(position_index);
            }
            if positions.is_empty() {
                self.derived_cache.mark_positions.remove(&key);
            }
        }
    }

    pub(crate) fn recompute_hash(&mut self) {
        let mut hash = puzzle_kernel::FnvBuilder::OFFSET;
        hash = fnv_mix(hash, u64::from(self.width));
        hash = fnv_mix(hash, u64::from(self.height));
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

    pub(crate) fn check_pos(&self, x: u16, y: u16) -> Result<(), StateError> {
        if !self.shape().contains(GridCoord::new([x, y])) {
            return Err(StateError::PositionOutOfBounds { x, y });
        }
        Ok(())
    }

    pub(crate) fn slot_index(&self, x: u16, y: u16, layer: LayerId) -> Result<usize, StateError> {
        self.check_pos(x, y)?;
        if layer.0 >= self.layer_count {
            return Err(StateError::LayerOutOfBounds { layer });
        }
        self.shape()
            .slot_index(GridCoord::new([x, y]), layer.0)
            .ok_or(StateError::LayerOutOfBounds { layer })
    }

    pub(crate) fn cell_index(&self, x: u16, y: u16) -> Result<usize, StateError> {
        self.check_pos(x, y)?;
        Ok(self.shape().cell_index_unchecked(GridCoord::new([x, y])))
    }

    #[inline]
    pub(crate) fn cell_index_unchecked(&self, x: u16, y: u16) -> usize {
        self.shape().cell_index_unchecked(GridCoord::new([x, y]))
    }

    #[inline]
    pub(crate) fn slot_index_unchecked(&self, x: u16, y: u16, layer: LayerId) -> usize {
        self.shape()
            .slot_index_unchecked(GridCoord::new([x, y]), layer.0)
    }

    fn shape(&self) -> GridShape<2> {
        GridShape::new([self.width, self.height], self.layer_count)
            .expect("state dimensions are validated at construction")
    }
}

fn map_variable_error(error: VariableValueError<VariableId>) -> StateError {
    match error {
        VariableValueError::OutOfBounds { variable } => {
            StateError::VariableOutOfBounds { variable }
        }
        VariableValueError::Overflow { variable } => StateError::VariableOverflow { variable },
        VariableValueError::DivisionByZero { variable } => {
            StateError::VariableDivisionByZero { variable }
        }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.mark == other.mark
            && self.visible_variables == other.visible_variables
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl Eq for State {}
