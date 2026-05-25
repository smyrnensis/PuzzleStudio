use crate::compiled_game::{CompiledGame, GlobalUpdateOp};
use crate::ids::{GlobalId, LayerId, ObjectId, RuleId, ScratchId};
use std::num::NonZeroU32;

#[derive(Clone, Debug)]
pub struct State {
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    cell_scratch_heads: Vec<Option<NonZeroU32>>,
    slot_scratch_heads: Vec<Option<NonZeroU32>>,
    scratch_entries: Vec<ScratchEntry>,
    free_scratch_entries: Vec<NonZeroU32>,
    visible_globals: Vec<i64>,
    level_fired_rules: Vec<RuleId>,
    derived_cache: DerivedCache,
    hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotScratch {
    pub scratch: ScratchId,
    pub value: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct ScratchEntry {
    scratch: SlotScratch,
    next: Option<NonZeroU32>,
}

#[derive(Clone)]
pub struct SlotScratchIter<'a> {
    entries: &'a [ScratchEntry],
    next: Option<NonZeroU32>,
}

impl Iterator for SlotScratchIter<'_> {
    type Item = SlotScratch;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        let entry = self.entries.get(scratch_entry_index(id))?;
        self.next = entry.next;
        Some(entry.scratch)
    }
}

#[derive(Clone, Debug)]
struct DerivedCache {
    object_counts: Vec<u32>,
    object_positions: Vec<Vec<usize>>,
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
            cell_scratch_heads: vec![None; cell_count],
            slot_scratch_heads: vec![None; slot_count],
            scratch_entries: Vec::new(),
            free_scratch_entries: Vec::new(),
            visible_globals,
            level_fired_rules: Vec::new(),
            derived_cache: DerivedCache {
                object_counts: vec![0; object_count + 1],
                object_positions: vec![Vec::new(); object_count + 1],
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
        (0..self.slots.len())
            .map(|index| self.slot_scratch_at(index).collect())
            .collect()
    }

    pub fn cell_scratch(&self) -> Vec<Vec<SlotScratch>> {
        (0..self.cell_scratch_heads.len())
            .map(|index| self.cell_scratch_at(index).collect())
            .collect()
    }

    #[inline]
    pub fn cell_scratch_at(&self, index: usize) -> SlotScratchIter<'_> {
        SlotScratchIter {
            entries: &self.scratch_entries,
            next: self.cell_scratch_heads.get(index).copied().flatten(),
        }
    }

    #[inline]
    pub fn slot_scratch_at(&self, index: usize) -> SlotScratchIter<'_> {
        SlotScratchIter {
            entries: &self.scratch_entries,
            next: self.slot_scratch_heads.get(index).copied().flatten(),
        }
    }

    #[inline]
    pub fn visible_globals(&self) -> &[i64] {
        &self.visible_globals
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
            next.clear_slot_scratch(index);
        }
        next.recompute_hash();
        next
    }

    #[inline]
    pub fn global_value(&self, global: GlobalId) -> Option<i64> {
        self.visible_globals.get(usize::from(global.0)).copied()
    }

    pub fn set_visible_global(&mut self, global: GlobalId, value: i64) -> Result<(), StateError> {
        let slot = self
            .visible_globals
            .get_mut(usize::from(global.0))
            .ok_or(StateError::GlobalOutOfBounds { global })?;
        *slot = value;
        self.recompute_hash();
        Ok(())
    }

    pub fn update_visible_global(
        &mut self,
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    ) -> Result<(), StateError> {
        let slot = self
            .visible_globals
            .get_mut(usize::from(global.0))
            .ok_or(StateError::GlobalOutOfBounds { global })?;
        *slot = apply_global_update(*slot, op, value, global)?;
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
        self.slots[index] == object
            && self
                .slot_scratch_at(index)
                .any(|entry| entry.scratch == scratch && entry.value == value)
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
        self.slots[index] == object
            && self
                .slot_scratch_at(index)
                .any(|entry| entry.scratch == scratch)
    }

    pub fn has_cell_scratch(&self, x: u16, y: u16, scratch: ScratchId, value: Option<i64>) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.cell_scratch_at(index)
            .any(|entry| entry.scratch == scratch && entry.value == value)
    }

    pub fn has_cell_scratch_key(&self, x: u16, y: u16, scratch: ScratchId) -> bool {
        let Ok(index) = self.cell_index(x, y) else {
            return false;
        };
        self.cell_scratch_at(index)
            .any(|entry| entry.scratch == scratch)
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
        self.clear_slot_scratch(index);
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn set_slot_unchecked(&mut self, x: u16, y: u16, layer: LayerId, object: ObjectId) {
        let index = self.slot_index_unchecked(x, y, layer);
        self.set_slot_index_unchecked(index, object);
        if object.is_empty() {
            self.clear_slot_scratch(index);
        }
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        x: u16,
        y: u16,
        layer: LayerId,
    ) -> Vec<SlotScratch> {
        let index = self.slot_index_unchecked(x, y, layer);
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.take_slot_scratch(index)
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
        self.set_slot_index_unchecked(index, object);
        self.set_slot_scratch(index, scratch);
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
        let mut current = self.slot_scratch_heads[index];
        while let Some(id) = current {
            let entry_index = scratch_entry_index(id);
            let entry = &mut self.scratch_entries[entry_index];
            if entry.scratch.scratch == scratch {
                entry.scratch.value = value;
                return;
            }
            current = entry.next;
        }
        self.push_slot_scratch(index, SlotScratch { scratch, value });
    }

    pub(crate) fn set_cell_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        let mut current = self.cell_scratch_heads[index];
        while let Some(id) = current {
            let entry_index = scratch_entry_index(id);
            let entry = &mut self.scratch_entries[entry_index];
            if entry.scratch.scratch == scratch {
                entry.scratch.value = value;
                return;
            }
            current = entry.next;
        }
        self.push_cell_scratch(index, SlotScratch { scratch, value });
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
        let mut entries = self.take_slot_scratch(index);
        entries.retain(|entry| {
            if entry.scratch != scratch {
                return true;
            }
            value.is_some_and(|value| entry.value != Some(value))
        });
        self.set_slot_scratch(index, entries);
    }

    pub(crate) fn remove_cell_scratch_unchecked(
        &mut self,
        x: u16,
        y: u16,
        scratch: ScratchId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(x, y);
        let mut entries = self.take_cell_scratch(index);
        entries.retain(|entry| {
            if entry.scratch != scratch {
                return true;
            }
            value.is_some_and(|value| entry.value != Some(value))
        });
        self.set_cell_scratch(index, entries);
    }

    pub(crate) fn clear_scratch(&mut self) {
        if self.slot_scratch_heads.iter().all(Option::is_none)
            && self.cell_scratch_heads.iter().all(Option::is_none)
        {
            return;
        }
        self.slot_scratch_heads.fill(None);
        self.cell_scratch_heads.fill(None);
        self.scratch_entries.clear();
        self.free_scratch_entries.clear();
        self.recompute_hash();
    }

    fn take_cell_scratch(&mut self, index: usize) -> Vec<SlotScratch> {
        let scratch = self.cell_scratch_at(index).collect::<Vec<_>>();
        self.clear_cell_scratch(index);
        scratch
    }

    fn set_cell_scratch(&mut self, index: usize, scratch: Vec<SlotScratch>) {
        self.clear_cell_scratch(index);
        for scratch in scratch {
            self.push_cell_scratch(index, scratch);
        }
    }

    fn clear_cell_scratch(&mut self, index: usize) {
        let mut current = self.cell_scratch_heads[index].take();
        while let Some(id) = current {
            let entry_index = scratch_entry_index(id);
            current = self.scratch_entries[entry_index].next;
            self.scratch_entries[entry_index].next = None;
            self.free_scratch_entries.push(id);
        }
    }

    fn take_slot_scratch(&mut self, index: usize) -> Vec<SlotScratch> {
        let scratch = self.slot_scratch_at(index).collect::<Vec<_>>();
        self.clear_slot_scratch(index);
        scratch
    }

    fn set_slot_scratch(&mut self, index: usize, scratch: Vec<SlotScratch>) {
        self.clear_slot_scratch(index);
        for scratch in scratch {
            self.push_slot_scratch(index, scratch);
        }
    }

    fn clear_slot_scratch(&mut self, index: usize) {
        let mut current = self.slot_scratch_heads[index].take();
        while let Some(id) = current {
            let entry_index = scratch_entry_index(id);
            current = self.scratch_entries[entry_index].next;
            self.scratch_entries[entry_index].next = None;
            self.free_scratch_entries.push(id);
        }
    }

    fn push_slot_scratch(&mut self, slot_index: usize, scratch: SlotScratch) {
        let new_id = self.allocate_scratch_entry(scratch);
        let Some(mut current) = self.slot_scratch_heads[slot_index] else {
            self.slot_scratch_heads[slot_index] = Some(new_id);
            return;
        };

        loop {
            let entry_index = scratch_entry_index(current);
            let Some(next) = self.scratch_entries[entry_index].next else {
                self.scratch_entries[entry_index].next = Some(new_id);
                return;
            };
            current = next;
        }
    }

    fn push_cell_scratch(&mut self, cell_index: usize, scratch: SlotScratch) {
        let new_id = self.allocate_scratch_entry(scratch);
        let Some(mut current) = self.cell_scratch_heads[cell_index] else {
            self.cell_scratch_heads[cell_index] = Some(new_id);
            return;
        };

        loop {
            let entry_index = scratch_entry_index(current);
            let Some(next) = self.scratch_entries[entry_index].next else {
                self.scratch_entries[entry_index].next = Some(new_id);
                return;
            };
            current = next;
        }
    }

    fn allocate_scratch_entry(&mut self, scratch: SlotScratch) -> NonZeroU32 {
        if let Some(id) = self.free_scratch_entries.pop() {
            let index = scratch_entry_index(id);
            self.scratch_entries[index] = ScratchEntry {
                scratch,
                next: None,
            };
            return id;
        }

        let raw_index = self.scratch_entries.len();
        let id = scratch_entry_id(raw_index);
        self.scratch_entries.push(ScratchEntry {
            scratch,
            next: None,
        });
        id
    }

    fn set_slot_index_unchecked(&mut self, index: usize, object: ObjectId) {
        let existing = self.slots[index];
        if existing == object {
            return;
        }
        self.remove_object_position(existing, index);
        self.slots[index] = object;
        self.add_object_position(object, index);
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

    pub(crate) fn recompute_hash(&mut self) {
        let mut hash = 0xcbf29ce484222325_u64;
        hash = fnv_mix(hash, u64::from(self.width));
        hash = fnv_mix(hash, u64::from(self.height));
        hash = fnv_mix(hash, u64::from(self.layer_count));
        for object in &self.slots {
            hash = fnv_mix(hash, u64::from(object.0));
        }
        for index in 0..self.cell_scratch_heads.len() {
            let scratch = self.cell_scratch_at(index);
            let count = scratch.clone().count();
            hash = fnv_mix(hash, count as u64);
            for scratch in scratch {
                hash = fnv_mix(hash, u64::from(scratch.scratch.0));
                hash = fnv_mix(hash, scratch.value.unwrap_or(i64::MIN) as u64);
            }
        }
        for index in 0..self.slots.len() {
            let scratch = self.slot_scratch_at(index);
            let count = scratch.clone().count();
            hash = fnv_mix(hash, count as u64);
            for scratch in scratch {
                hash = fnv_mix(hash, u64::from(scratch.scratch.0));
                hash = fnv_mix(hash, scratch.value.unwrap_or(i64::MIN) as u64);
            }
        }
        for value in &self.visible_globals {
            hash = fnv_mix(hash, *value as u64);
        }
        hash = fnv_mix(hash, self.level_fired_rules.len() as u64);
        for rule in &self.level_fired_rules {
            hash = fnv_mix(hash, u64::from(rule.0));
        }
        self.hash = hash;
    }

    fn slot_scratch_equal(&self, other: &Self) -> bool {
        if self.slots.len() != other.slots.len() {
            return false;
        }
        (0..self.slots.len())
            .all(|index| self.slot_scratch_at(index).eq(other.slot_scratch_at(index)))
    }

    fn cell_scratch_equal(&self, other: &Self) -> bool {
        if self.cell_scratch_heads.len() != other.cell_scratch_heads.len() {
            return false;
        }
        (0..self.cell_scratch_heads.len())
            .all(|index| self.cell_scratch_at(index).eq(other.cell_scratch_at(index)))
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

fn apply_global_update(
    current: i64,
    op: GlobalUpdateOp,
    value: i64,
    global: GlobalId,
) -> Result<i64, StateError> {
    match op {
        GlobalUpdateOp::Set => Ok(value),
        GlobalUpdateOp::Add => current
            .checked_add(value)
            .ok_or(StateError::GlobalOverflow { global }),
        GlobalUpdateOp::Subtract => current
            .checked_sub(value)
            .ok_or(StateError::GlobalOverflow { global }),
        GlobalUpdateOp::Multiply => current
            .checked_mul(value)
            .ok_or(StateError::GlobalOverflow { global }),
        GlobalUpdateOp::Divide => {
            if value == 0 {
                return Err(StateError::GlobalDivisionByZero { global });
            }
            current
                .checked_div(value)
                .ok_or(StateError::GlobalOverflow { global })
        }
        GlobalUpdateOp::Remainder => {
            if value == 0 {
                return Err(StateError::GlobalDivisionByZero { global });
            }
            current
                .checked_rem(value)
                .ok_or(StateError::GlobalOverflow { global })
        }
    }
}

fn scratch_entry_index(id: NonZeroU32) -> usize {
    usize::try_from(id.get() - 1).expect("scratch entry id must fit usize")
}

fn scratch_entry_id(index: usize) -> NonZeroU32 {
    let raw = u32::try_from(index + 1).expect("too many slot scratch entries");
    NonZeroU32::new(raw).expect("scratch entry ids are one-based")
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.cell_scratch_equal(other)
            && self.slot_scratch_equal(other)
            && self.visible_globals == other.visible_globals
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl Eq for State {}

#[inline]
fn fnv_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}
