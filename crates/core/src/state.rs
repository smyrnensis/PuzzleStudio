use crate::compiled_game::{CompiledGame, GlobalUpdateOp};
use crate::ids::{GlobalId, LayerId, ObjectId, RuleId, ScratchId};
use puzzle_kernel::{GlobalValueError, ObjectCellMask, ScratchSpace, VisibleGlobals, fnv_mix};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct State {
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    scratch: ScratchSpace<ScratchId>,
    visible_globals: VisibleGlobals<GlobalId>,
    level_fired_rules: Vec<RuleId>,
    derived_cache: DerivedCache,
    hash: u64,
}

pub type SlotScratch = puzzle_kernel::ScratchValue<ScratchId>;
pub type SlotScratchIter<'a> = puzzle_kernel::ScratchIter<'a, ScratchId>;

#[derive(Clone, Debug)]
struct DerivedCache {
    object_counts: Vec<u32>,
    object_positions: Vec<Vec<usize>>,
    cell_object_masks: Vec<ObjectCellMask>,
    scratch_positions: BTreeMap<ScratchPositionKey, Vec<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ScratchPositionKey {
    object: ObjectId,
    scratch: ScratchId,
    value: Option<i64>,
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
    GlobalOutOfBounds {
        global: GlobalId,
    },
    GlobalOverflow {
        global: GlobalId,
    },
    GlobalDivisionByZero {
        global: GlobalId,
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
        Self::empty_with_globals(width, height, layer_count, object_count, Vec::new())
    }

    pub fn empty_with_globals(
        width: u16,
        height: u16,
        layer_count: u16,
        object_count: usize,
        visible_globals: Vec<i64>,
    ) -> Result<Self, StateError> {
        if width == 0 || height == 0 || layer_count == 0 {
            return Err(StateError::InvalidDimensions);
        }

        let slot_count = usize::from(width) * usize::from(height) * usize::from(layer_count);
        let cell_count = usize::from(width) * usize::from(height);
        let mut state = Self {
            width,
            height,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            scratch: ScratchSpace::new(cell_count, slot_count),
            visible_globals: VisibleGlobals::new(visible_globals),
            level_fired_rules: Vec::new(),
            derived_cache: DerivedCache {
                object_counts: vec![0; object_count + 1],
                object_positions: vec![Vec::new(); object_count + 1],
                cell_object_masks: vec![ObjectCellMask::default(); cell_count],
                scratch_positions: BTreeMap::new(),
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

    pub fn slot_scratch(&self) -> Vec<Vec<SlotScratch>> {
        self.scratch.slot_values()
    }

    pub fn cell_scratch(&self) -> Vec<Vec<SlotScratch>> {
        self.scratch.cell_values()
    }

    #[inline]
    pub fn cell_scratch_at(&self, index: usize) -> SlotScratchIter<'_> {
        self.scratch.cell_at(index)
    }

    #[inline]
    pub fn slot_scratch_at(&self, index: usize) -> SlotScratchIter<'_> {
        self.scratch.slot_at(index)
    }

    #[inline]
    pub fn visible_globals(&self) -> &[i64] {
        self.visible_globals.as_slice()
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

    pub fn without_visual_objects(&self, game: &CompiledGame) -> Self {
        if game.visual_objects().is_empty() {
            return self.clone();
        }
        if self
            .slots
            .iter()
            .all(|object| object.is_empty() || !game.is_visual_object(*object))
        {
            return self.clone();
        }

        let mut next = self.clone();
        for index in 0..next.slots.len() {
            let object = next.slots[index];
            if object.is_empty() || !game.is_visual_object(object) {
                continue;
            }
            next.set_slot_index_unchecked(index, ObjectId::EMPTY);
            next.scratch.clear_slot(index);
        }
        next.recompute_hash();
        next
    }

    #[inline]
    pub fn global_value(&self, global: GlobalId) -> Option<i64> {
        self.visible_globals.get(global)
    }

    pub fn set_visible_global(&mut self, global: GlobalId, value: i64) -> Result<(), StateError> {
        self.visible_globals
            .set(global, value)
            .map_err(map_global_error)?;
        self.recompute_hash();
        Ok(())
    }

    pub fn update_visible_global(
        &mut self,
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    ) -> Result<(), StateError> {
        self.visible_globals
            .update(global, op, value)
            .map_err(map_global_error)?;
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
    pub fn scratch_positions(
        &self,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    ) -> &[usize] {
        let key = ScratchPositionKey {
            object,
            scratch,
            value,
        };
        self.derived_cache
            .scratch_positions
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

    pub fn has_scratch(
        &self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_scratch(x, y, scratch, value);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(x, y, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.has_slot(index, scratch, value)
    }

    pub fn has_scratch_key(
        &self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
        scratch: ScratchId,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_scratch_key(x, y, scratch);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(x, y, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.has_slot_key(index, scratch)
    }

    pub fn has_cell_scratch(&self, x: u16, y: u16, scratch: ScratchId, value: Option<i64>) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.scratch.has_cell(index, scratch, value)
    }

    pub fn has_cell_scratch_key(&self, x: u16, y: u16, scratch: ScratchId) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.scratch.has_cell_key(index, scratch)
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
        self.clear_slot_scratch_positions(index, existing);
        self.scratch.clear_slot(index);
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn set_slot_unchecked(&mut self, x: u16, y: u16, layer: LayerId, object: ObjectId) {
        let index = self.slot_index_unchecked(x, y, layer);
        let existing = self.slots[index];
        if object.is_empty() {
            self.clear_slot_scratch_positions(index, existing);
            self.scratch.clear_slot(index);
        }
        self.set_slot_index_unchecked(index, object);
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
    ) -> Vec<SlotScratch> {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.clear_slot_scratch_positions(index, object);
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.scratch.take_slot(index)
    }

    pub(crate) fn place_moved_slot_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        object: ObjectId,
        scratch: Vec<SlotScratch>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let existing = self.slots[index];
        self.clear_slot_scratch_positions(index, existing);
        self.set_slot_index_unchecked(index, object);
        self.scratch.replace_slot(index, scratch);
        self.add_slot_scratch_positions(index, object);
    }

    pub(crate) fn set_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.remove_slot_scratch_key_positions(index, object, scratch);
        self.scratch.set_slot(index, scratch, value);
        self.add_scratch_position(object, scratch, value, index);
    }

    pub(crate) fn set_cell_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        self.remove_cell_scratch_key_positions(index, scratch);
        self.scratch.set_cell(index, scratch, value);
        self.add_scratch_position(ObjectId::EMPTY, scratch, value, index);
    }

    pub(crate) fn remove_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(x, y, layer);
        let object = self.slots[index];
        self.remove_matching_slot_scratch_positions(index, object, scratch, value);
        self.scratch.remove_slot(index, scratch, value);
    }

    pub(crate) fn remove_cell_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        self.remove_matching_cell_scratch_positions(index, scratch, value);
        self.scratch.remove_cell(index, scratch, value);
    }

    pub(crate) fn clear_scratch(&mut self) {
        if self.scratch.is_empty() {
            return;
        }
        self.scratch.clear_all();
        self.derived_cache.scratch_positions.clear();
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

    fn add_slot_scratch_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        let scratch = self.slot_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch {
            self.add_scratch_position(object, entry.scratch, entry.value, index);
        }
    }

    fn clear_slot_scratch_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        let scratch = self.slot_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch {
            self.remove_scratch_position(object, entry.scratch, entry.value, index);
        }
    }

    fn remove_slot_scratch_key_positions(
        &mut self,
        index: usize,
        object: ObjectId,
        scratch: ScratchId,
    ) {
        if object.is_empty() {
            return;
        }
        let scratch_entries = self.slot_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch_entries {
            if entry.scratch == scratch {
                self.remove_scratch_position(object, entry.scratch, entry.value, index);
            }
        }
    }

    fn remove_matching_slot_scratch_positions(
        &mut self,
        index: usize,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        if object.is_empty() {
            return;
        }
        let scratch_entries = self.slot_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch_entries {
            if entry.scratch == scratch && (value.is_none() || entry.value == value) {
                self.remove_scratch_position(object, entry.scratch, entry.value, index);
            }
        }
    }

    fn remove_cell_scratch_key_positions(&mut self, index: usize, scratch: ScratchId) {
        let scratch_entries = self.cell_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch_entries {
            if entry.scratch == scratch {
                self.remove_scratch_position(ObjectId::EMPTY, entry.scratch, entry.value, index);
            }
        }
    }

    fn remove_matching_cell_scratch_positions(
        &mut self,
        index: usize,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let scratch_entries = self.cell_scratch_at(index).collect::<Vec<_>>();
        for entry in scratch_entries {
            if entry.scratch == scratch && (value.is_none() || entry.value == value) {
                self.remove_scratch_position(ObjectId::EMPTY, entry.scratch, entry.value, index);
            }
        }
    }

    fn add_scratch_position(
        &mut self,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
        index: usize,
    ) {
        let key = ScratchPositionKey {
            object,
            scratch,
            value,
        };
        let positions = self.derived_cache.scratch_positions.entry(key).or_default();
        if !positions.contains(&index) {
            positions.push(index);
        }
    }

    fn remove_scratch_position(
        &mut self,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
        index: usize,
    ) {
        let key = ScratchPositionKey {
            object,
            scratch,
            value,
        };
        if let Some(positions) = self.derived_cache.scratch_positions.get_mut(&key) {
            if let Some(position_index) = positions.iter().position(|position| *position == index) {
                positions.swap_remove(position_index);
            }
            if positions.is_empty() {
                self.derived_cache.scratch_positions.remove(&key);
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
        hash = self.scratch.hash_into(hash, |scratch| u64::from(scratch.0));
        for value in self.visible_globals.as_slice() {
            hash = fnv_mix(hash, *value as u64);
        }
        hash = fnv_mix(hash, self.level_fired_rules.len() as u64);
        for rule in &self.level_fired_rules {
            hash = fnv_mix(hash, u64::from(rule.0));
        }
        self.hash = hash;
    }

    pub(crate) fn check_pos(&self, x: u16, y: u16) -> Result<(), StateError> {
        if x >= self.width || y >= self.height {
            return Err(StateError::PositionOutOfBounds { x, y });
        }
        Ok(())
    }

    pub(crate) fn slot_index(&self, x: u16, y: u16, layer: LayerId) -> Result<usize, StateError> {
        self.check_pos(x, y)?;
        if layer.0 >= self.layer_count {
            return Err(StateError::LayerOutOfBounds { layer });
        }
        Ok(self.slot_index_unchecked(x, y, layer))
    }

    pub(crate) fn cell_index(&self, x: u16, y: u16) -> Result<usize, StateError> {
        self.check_pos(x, y)?;
        Ok(self.cell_index_unchecked(x, y))
    }

    #[inline]
    pub(crate) fn cell_index_unchecked(&self, x: u16, y: u16) -> usize {
        usize::from(y) * usize::from(self.width) + usize::from(x)
    }

    #[inline]
    pub(crate) fn slot_index_unchecked(&self, x: u16, y: u16, layer: LayerId) -> usize {
        ((usize::from(y) * usize::from(self.width) + usize::from(x))
            * usize::from(self.layer_count))
            + usize::from(layer.0)
    }
}

fn map_global_error(error: GlobalValueError<GlobalId>) -> StateError {
    match error {
        GlobalValueError::OutOfBounds { global } => StateError::GlobalOutOfBounds { global },
        GlobalValueError::Overflow { global } => StateError::GlobalOverflow { global },
        GlobalValueError::DivisionByZero { global } => StateError::GlobalDivisionByZero { global },
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.scratch == other.scratch
            && self.visible_globals == other.visible_globals
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl Eq for State {}
