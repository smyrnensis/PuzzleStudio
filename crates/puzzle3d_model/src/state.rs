use crate::{Coord3, Game3, GlobalId3, LayerId, ObjectId, RuleId3, ScratchId3, Size3};
use puzzle_kernel::{
    FnvBuilder, GlobalUpdateOp, GlobalValueError, ObjectCellMask, ScratchSpace, ScratchValue,
    VisibleGlobals, fnv_mix,
};

#[derive(Clone, Debug)]
pub struct State3 {
    pub size: Size3,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    scratch: ScratchSpace<ScratchId3>,
    visible_globals: VisibleGlobals<GlobalId3>,
    level_fired_rules: Vec<RuleId3>,
    cell_object_masks: Vec<ObjectCellMask>,
    hash: u64,
}

pub type SlotScratch3 = ScratchValue<ScratchId3>;

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
    GlobalOutOfBounds {
        global: GlobalId3,
    },
    GlobalOverflow {
        global: GlobalId3,
    },
    GlobalDivisionByZero {
        global: GlobalId3,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellView3 {
    pub objects: Vec<ObjectId>,
}

impl State3 {
    pub fn empty(size: Size3, layer_count: u16) -> Result<Self, StateError3> {
        Self::empty_with_globals(size, layer_count, Vec::new())
    }

    pub fn empty_with_globals(
        size: Size3,
        layer_count: u16,
        visible_globals: Vec<i64>,
    ) -> Result<Self, StateError3> {
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
        let mut state = Self {
            size,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            scratch: ScratchSpace::new(cell_count, slot_count),
            visible_globals: VisibleGlobals::new(visible_globals),
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

    pub fn slot_scratch(&self) -> Vec<Vec<SlotScratch3>> {
        self.scratch.slot_values()
    }

    pub fn cell_scratch(&self) -> Vec<Vec<SlotScratch3>> {
        self.scratch.cell_values()
    }

    pub fn visible_globals(&self) -> &[i64] {
        self.visible_globals.as_slice()
    }

    pub fn global_value(&self, global: GlobalId3) -> Option<i64> {
        self.visible_globals.get(global)
    }

    pub fn set_visible_global(&mut self, global: GlobalId3, value: i64) -> Result<(), StateError3> {
        self.visible_globals
            .set(global, value)
            .map_err(map_global_error)?;
        self.recompute_hash();
        Ok(())
    }

    pub fn update_visible_global(
        &mut self,
        global: GlobalId3,
        op: GlobalUpdateOp,
        value: i64,
    ) -> Result<(), StateError3> {
        self.visible_globals
            .update(global, op, value)
            .map_err(map_global_error)?;
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

    pub fn without_visual_objects(&self, game: &Game3) -> Self {
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

    pub fn has_scratch(
        &self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
        scratch: ScratchId3,
        value: Option<i64>,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_scratch(position, scratch, value);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.has_slot(index, scratch, value)
    }

    pub fn has_scratch_key(
        &self,
        game: &Game3,
        position: Coord3,
        object: ObjectId,
        scratch: ScratchId3,
    ) -> bool {
        if object.is_empty() {
            return self.has_cell_scratch_key(position, scratch);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.has_slot_key(index, scratch)
    }

    pub fn has_cell_scratch(
        &self,
        position: Coord3,
        scratch: ScratchId3,
        value: Option<i64>,
    ) -> bool {
        if self.check_pos(position).is_err() {
            return false;
        }
        let index = self.cell_index_unchecked(position);
        self.scratch.has_cell(index, scratch, value)
    }

    pub fn has_cell_scratch_key(&self, position: Coord3, scratch: ScratchId3) -> bool {
        if self.check_pos(position).is_err() {
            return false;
        }
        let index = self.cell_index_unchecked(position);
        self.scratch.has_cell_key(index, scratch)
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
        self.scratch.clear_slot(index);
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
        self.scratch.clear_slot(index);
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
    ) -> Vec<SlotScratch3> {
        let index = self.slot_index_unchecked(position, layer);
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.scratch.take_slot(index)
    }

    pub(crate) fn place_moved_slot_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        object: ObjectId,
        scratch: Vec<SlotScratch3>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.set_slot_index_unchecked(index, object);
        self.scratch.replace_slot(index, scratch);
        self.recompute_hash();
    }

    pub(crate) fn set_scratch_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        scratch: ScratchId3,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.scratch.set_slot(index, scratch, value);
        self.recompute_hash();
    }

    pub(crate) fn set_cell_scratch_unchecked(
        &mut self,
        position: Coord3,
        scratch: ScratchId3,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.scratch.set_cell(index, scratch, value);
        self.recompute_hash();
    }

    pub(crate) fn remove_scratch_unchecked(
        &mut self,
        position: Coord3,
        layer: LayerId,
        scratch: ScratchId3,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        self.scratch.remove_slot(index, scratch, value);
        self.recompute_hash();
    }

    pub(crate) fn remove_cell_scratch_unchecked(
        &mut self,
        position: Coord3,
        scratch: ScratchId3,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.scratch.remove_cell(index, scratch, value);
        self.recompute_hash();
    }

    pub(crate) fn clear_scratch(&mut self) {
        if self.scratch.is_empty() {
            return;
        }
        self.scratch.clear_all();
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

fn map_global_error(error: GlobalValueError<GlobalId3>) -> StateError3 {
    match error {
        GlobalValueError::OutOfBounds { global } => StateError3::GlobalOutOfBounds { global },
        GlobalValueError::Overflow { global } => StateError3::GlobalOverflow { global },
        GlobalValueError::DivisionByZero { global } => StateError3::GlobalDivisionByZero { global },
    }
}

impl PartialEq for State3 {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.scratch == other.scratch
            && self.visible_globals == other.visible_globals
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl Eq for State3 {}
