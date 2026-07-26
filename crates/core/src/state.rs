use crate::compiled_game::{CompiledGame, VariableUpdateOp};
use crate::ids::{LayerId, MarkId, ObjectId, RuleId, VariableId};
use puzzle_kernel::{
    CompiledGameModel, GridCoord, GridOffset, GridShape, MarkSpace, ObjectCellMask,
    VariableValueError, VisibleVariables, fnv_mix,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::ops::Deref;

pub trait GridSize<const D: usize>: Copy + Eq {
    fn axes(self) -> [u16; D];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Size2 {
    pub width: u16,
    pub height: u16,
}

impl Size2 {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

impl GridSize<2> for Size2 {
    fn axes(self) -> [u16; 2] {
        [self.width, self.height]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Size3 {
    pub width: u16,
    pub depth: u16,
    pub height: u16,
}

impl Size3 {
    pub const fn new(width: u16, depth: u16, height: u16) -> Self {
        Self {
            width,
            depth,
            height,
        }
    }
}

impl GridSize<3> for Size3 {
    fn axes(self) -> [u16; 3] {
        [self.width, self.depth, self.height]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coord3 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl Coord3 {
    pub const fn new(x: u16, y: u16, z: u16) -> Self {
        Self { x, y, z }
    }

    pub fn checked_offset(self, offset: Delta3) -> Option<Self> {
        GridCoord::<3>::from(self)
            .checked_offset(offset.into())
            .map(Self::from)
    }
}

impl From<Coord3> for GridCoord<3> {
    fn from(value: Coord3) -> Self {
        Self::new([value.x, value.y, value.z])
    }
}

impl From<GridCoord<3>> for Coord3 {
    fn from(value: GridCoord<3>) -> Self {
        let [x, y, z] = value.axes();
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Delta3 {
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}

impl Delta3 {
    pub const ZERO: Self = Self::new(0, 0, 0);

    pub const fn new(dx: i16, dy: i16, dz: i16) -> Self {
        Self { dx, dy, dz }
    }

    pub const fn scale(self, factor: i16) -> Self {
        Self::new(self.dx * factor, self.dy * factor, self.dz * factor)
    }

    pub const fn add(self, other: Self) -> Self {
        Self::new(self.dx + other.dx, self.dy + other.dy, self.dz + other.dz)
    }
}

impl From<Delta3> for GridOffset<3> {
    fn from(value: Delta3) -> Self {
        Self::new([value.dx, value.dy, value.dz])
    }
}

impl From<GridOffset<3>> for Delta3 {
    fn from(value: GridOffset<3>) -> Self {
        let [dx, dy, dz] = value.deltas();
        Self { dx, dy, dz }
    }
}

impl From<Delta3> for puzzle_kernel::SpatialOffset<3> {
    fn from(value: Delta3) -> Self {
        Self::Fixed {
            delta: puzzle_kernel::SpatialVector::new([value.dx, value.dy, value.dz]),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GridState<const D: usize, Size: GridSize<D>> {
    #[serde(flatten)]
    pub size: Size,
    pub layer_count: u16,
    slots: Vec<ObjectId>,
    visible_variables: VisibleVariables<VariableId>,
    level_fired_rules: Vec<RuleId>,
    #[serde(skip)]
    derived_cache: DerivedCache,
    #[serde(skip)]
    hash: u64,
}

pub type State = GridState<2, Size2>;

impl<const D: usize, Size: GridSize<D>> Deref for GridState<D, Size> {
    type Target = Size;

    fn deref(&self) -> &Self::Target {
        &self.size
    }
}

pub type SlotMark = puzzle_kernel::MarkValue<MarkId>;
pub type SlotMarkIter<'a> = puzzle_kernel::MarkIter<'a, MarkId>;

#[derive(Clone, Debug, Default)]
struct DerivedCache {
    object_counts: Vec<u32>,
    object_positions: Vec<Vec<usize>>,
    cell_object_masks: Vec<ObjectCellMask>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MarkPositionKey {
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MarkKey {
    mark: MarkId,
    value: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct GridMarkScratch {
    mark: MarkSpace<MarkId>,
    mark_positions: BTreeMap<MarkPositionKey, Vec<usize>>,
    slot_mark_positions: BTreeMap<MarkKey, Vec<usize>>,
}

impl GridMarkScratch {
    fn empty(cell_count: usize, slot_count: usize) -> Self {
        Self {
            mark: MarkSpace::new(cell_count, slot_count),
            mark_positions: BTreeMap::new(),
            slot_mark_positions: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GridExecutionState<const D: usize, Size: GridSize<D>> {
    committed: GridState<D, Size>,
    scratch: GridMarkScratch,
    hash: u64,
    program_state_key: puzzle_kernel::ProgramStateKey,
}

impl<const D: usize, Size: GridSize<D>> GridExecutionState<D, Size> {
    pub(crate) fn new(committed: GridState<D, Size>) -> Self {
        let shape = committed.shape();
        let scratch = GridMarkScratch::empty(
            shape
                .cell_count()
                .expect("state dimensions are validated at construction"),
            shape
                .slot_count()
                .expect("state dimensions are validated at construction"),
        );
        Self::with_scratch(committed, scratch)
            .expect("scratch dimensions are derived from the committed state")
    }

    pub(crate) fn with_scratch(
        committed: GridState<D, Size>,
        scratch: GridMarkScratch,
    ) -> Result<Self, GridStateError<D>> {
        let shape = committed.shape();
        if scratch.mark.cell_count() != shape.cell_count().unwrap_or(0)
            || scratch.mark.slot_count() != shape.slot_count().unwrap_or(0)
        {
            return Err(GridStateError::InvalidDimensions);
        }
        let mut state = Self {
            committed,
            scratch,
            hash: 0,
            program_state_key: puzzle_kernel::ProgramStateKey::default(),
        };
        state.recompute_hash();
        Ok(state)
    }

    pub(crate) fn committed(&self) -> &GridState<D, Size> {
        &self.committed
    }

    pub(crate) fn into_committed(self) -> GridState<D, Size> {
        self.committed
    }

    pub(crate) fn scratch(&self) -> &GridMarkScratch {
        &self.scratch
    }

    pub(crate) fn program_state_key(&self) -> puzzle_kernel::ProgramStateKey {
        self.program_state_key.clone()
    }

    pub(crate) fn mark_positions(
        &self,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    ) -> &[usize] {
        self.scratch
            .mark_positions
            .get(&MarkPositionKey {
                object,
                mark,
                value,
            })
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn slot_mark_positions(&self, mark: MarkId, value: Option<i64>) -> &[usize] {
        self.scratch
            .slot_mark_positions
            .get(&MarkKey { mark, value })
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn has_mark_at<ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    ) -> bool {
        let position = position.into();
        if object.is_empty() {
            return self.has_cell_mark_at(position, mark, value);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.mark.has_slot(index, mark, value)
    }

    pub(crate) fn has_mark_key_at<ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
        mark: MarkId,
    ) -> bool {
        let position = position.into();
        if object.is_empty() {
            return self.has_cell_mark_key_at(position, mark);
        }
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        let Ok(index) = self.slot_index(position, layer) else {
            return false;
        };
        self.slots[index] == object && self.scratch.mark.has_slot_key(index, mark)
    }

    pub(crate) fn has_cell_mark_at(
        &self,
        position: impl Into<GridCoord<D>>,
        mark: MarkId,
        value: Option<i64>,
    ) -> bool {
        self.cell_index(position.into())
            .is_ok_and(|index| self.scratch.mark.has_cell(index, mark, value))
    }

    pub(crate) fn has_cell_mark_key_at(
        &self,
        position: impl Into<GridCoord<D>>,
        mark: MarkId,
    ) -> bool {
        self.cell_index(position.into())
            .is_ok_and(|index| self.scratch.mark.has_cell_key(index, mark))
    }

    pub(crate) fn set_slot_unchecked(
        &mut self,
        position: GridCoord<D>,
        layer: LayerId,
        object: ObjectId,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        let existing = self.slots[index];
        if existing != object {
            self.clear_slot_mark_positions(index, existing);
            self.scratch.mark.clear_slot(index);
        }
        self.committed.set_slot_index_unchecked(index, object);
    }

    pub(crate) fn take_slot_for_move_unchecked(
        &mut self,
        position: GridCoord<D>,
        layer: LayerId,
    ) -> Vec<SlotMark> {
        let index = self.slot_index_unchecked(position, layer);
        let object = self.slots[index];
        self.clear_slot_mark_positions(index, object);
        self.committed
            .set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.scratch.mark.take_slot(index)
    }

    pub(crate) fn place_moved_slot_unchecked(
        &mut self,
        position: GridCoord<D>,
        layer: LayerId,
        object: ObjectId,
        mark: Vec<SlotMark>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        let existing = self.slots[index];
        self.clear_slot_mark_positions(index, existing);
        self.committed.set_slot_index_unchecked(index, object);
        self.scratch.mark.replace_slot(index, mark);
        self.add_slot_mark_positions(index, object);
    }

    pub(crate) fn set_mark_unchecked(
        &mut self,
        position: GridCoord<D>,
        layer: LayerId,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        let object = self.slots[index];
        self.remove_slot_mark_key_positions(index, object, mark);
        self.scratch.mark.set_slot(index, mark, value);
        self.add_mark_position(object, mark, value, index);
    }

    pub(crate) fn set_cell_mark_unchecked(
        &mut self,
        position: GridCoord<D>,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.remove_cell_mark_key_positions(index, mark);
        self.scratch.mark.set_cell(index, mark, value);
        self.add_mark_position(ObjectId::EMPTY, mark, value, index);
    }

    pub(crate) fn remove_mark_unchecked(
        &mut self,
        position: GridCoord<D>,
        layer: LayerId,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.slot_index_unchecked(position, layer);
        let object = self.slots[index];
        self.remove_matching_slot_mark_positions(index, object, mark, value);
        self.scratch.mark.remove_slot(index, mark, value);
    }

    pub(crate) fn remove_cell_mark_unchecked(
        &mut self,
        position: GridCoord<D>,
        mark: MarkId,
        value: Option<i64>,
    ) {
        let index = self.cell_index_unchecked(position);
        self.remove_matching_cell_mark_positions(index, mark, value);
        self.scratch.mark.remove_cell(index, mark, value);
    }

    pub(crate) fn mark_level_rule_fired(&mut self, rule: RuleId) {
        self.committed.mark_level_rule_fired(rule);
        self.recompute_hash();
    }

    pub(crate) fn update_visible_variable(
        &mut self,
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    ) -> Result<(), GridStateError<D>> {
        self.committed
            .update_visible_variable(variable, op, value)?;
        self.recompute_hash();
        Ok(())
    }

    pub(crate) fn recompute_hash(&mut self) {
        self.committed.recompute_hash();
        self.hash = self
            .scratch
            .mark
            .hash_into(self.committed.hash(), |mark| u64::from(mark.0));
        let mut words = self.committed.build_program_state_key_words();
        self.scratch
            .mark
            .append_key_words(&mut words, |mark| u64::from(mark.0));
        self.program_state_key =
            puzzle_kernel::ProgramStateKey::from_words_and_hash(words, self.hash);
    }

    fn add_slot_mark_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        for entry in self.scratch.mark.slot_at(index).collect::<Vec<_>>() {
            self.add_mark_position(object, entry.mark, entry.value, index);
        }
    }

    fn clear_slot_mark_positions(&mut self, index: usize, object: ObjectId) {
        if object.is_empty() {
            return;
        }
        for entry in self.scratch.mark.slot_at(index).collect::<Vec<_>>() {
            self.remove_mark_position(object, entry.mark, entry.value, index);
        }
    }

    fn remove_slot_mark_key_positions(&mut self, index: usize, object: ObjectId, mark: MarkId) {
        if object.is_empty() {
            return;
        }
        for entry in self.scratch.mark.slot_at(index).collect::<Vec<_>>() {
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
        for entry in self.scratch.mark.slot_at(index).collect::<Vec<_>>() {
            if entry.mark == mark && (value.is_none() || entry.value == value) {
                self.remove_mark_position(object, entry.mark, entry.value, index);
            }
        }
    }

    fn remove_cell_mark_key_positions(&mut self, index: usize, mark: MarkId) {
        for entry in self.scratch.mark.cell_at(index).collect::<Vec<_>>() {
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
        for entry in self.scratch.mark.cell_at(index).collect::<Vec<_>>() {
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
        let positions = self
            .scratch
            .mark_positions
            .entry(MarkPositionKey {
                object,
                mark,
                value,
            })
            .or_default();
        if let Err(insert_at) = positions.binary_search(&index) {
            positions.insert(insert_at, index);
        }
        if !object.is_empty() {
            let positions = self
                .scratch
                .slot_mark_positions
                .entry(MarkKey { mark, value })
                .or_default();
            if let Err(insert_at) = positions.binary_search(&index) {
                positions.insert(insert_at, index);
            }
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
        if let Some(positions) = self.scratch.mark_positions.get_mut(&key) {
            if let Ok(position_index) = positions.binary_search(&index) {
                positions.remove(position_index);
            }
            if positions.is_empty() {
                self.scratch.mark_positions.remove(&key);
            }
        }
        if !object.is_empty() {
            let key = MarkKey { mark, value };
            if let Some(positions) = self.scratch.slot_mark_positions.get_mut(&key) {
                if let Ok(position_index) = positions.binary_search(&index) {
                    positions.remove(position_index);
                }
                if positions.is_empty() {
                    self.scratch.slot_mark_positions.remove(&key);
                }
            }
        }
    }
}

impl<const D: usize, Size: GridSize<D>> Deref for GridExecutionState<D, Size> {
    type Target = GridState<D, Size>;

    fn deref(&self) -> &Self::Target {
        &self.committed
    }
}

impl<const D: usize, Size: GridSize<D>> PartialEq for GridExecutionState<D, Size> {
    fn eq(&self, other: &Self) -> bool {
        self.committed == other.committed && self.scratch.mark == other.scratch.mark
    }
}

impl<const D: usize, Size: GridSize<D>> Eq for GridExecutionState<D, Size> {}

impl<'de, const D: usize, Size> Deserialize<'de> for GridState<D, Size>
where
    Size: GridSize<D> + Deserialize<'de>,
{
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StateData<Size> {
            #[serde(flatten)]
            size: Size,
            layer_count: u16,
            slots: Vec<ObjectId>,
            visible_variables: VisibleVariables<VariableId>,
            level_fired_rules: Vec<RuleId>,
        }

        let data = StateData::<Size>::deserialize(deserializer)?;
        let mut state = GridState {
            size: data.size,
            layer_count: data.layer_count,
            slots: data.slots,
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
pub enum GridStateError<const D: usize> {
    InvalidDimensions,
    PositionOutOfBounds {
        position: GridCoord<D>,
    },
    LayerOutOfBounds {
        layer: LayerId,
    },
    UnknownObject {
        object: ObjectId,
    },
    LayerOccupied {
        position: GridCoord<D>,
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
    ObjectNotPresent {
        position: GridCoord<D>,
        object: ObjectId,
    },
}

pub type StateError = GridStateError<2>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellView {
    pub objects: Vec<ObjectId>,
}

impl<const D: usize, Size: GridSize<D>> GridState<D, Size> {
    pub fn empty_sized(
        size: Size,
        layer_count: u16,
        object_count: usize,
    ) -> Result<Self, GridStateError<D>> {
        Self::empty_sized_with_variables(size, layer_count, object_count, Vec::new())
    }

    pub fn empty_sized_with_variables(
        size: Size,
        layer_count: u16,
        object_count: usize,
        visible_variables: Vec<i64>,
    ) -> Result<Self, GridStateError<D>> {
        let shape = GridShape::<D>::new(size.axes(), layer_count)
            .ok_or(GridStateError::InvalidDimensions)?;
        let slot_count = shape
            .slot_count()
            .ok_or(GridStateError::InvalidDimensions)?;
        let cell_count = shape
            .cell_count()
            .ok_or(GridStateError::InvalidDimensions)?;

        let mut state = Self {
            size,
            layer_count,
            slots: vec![ObjectId::EMPTY; slot_count],
            visible_variables: VisibleVariables::new(visible_variables),
            level_fired_rules: Vec::new(),
            derived_cache: DerivedCache {
                object_counts: vec![0; object_count + 1],
                object_positions: vec![Vec::new(); object_count + 1],
                cell_object_masks: vec![ObjectCellMask::default(); cell_count],
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

    fn build_program_state_key_words(&self) -> Vec<u64> {
        let mut words =
            Vec::with_capacity(self.slots.len() + self.visible_variables.as_slice().len() + 16);
        words.extend(self.size.axes().into_iter().map(u64::from));
        words.push(u64::from(self.layer_count));
        words.push(self.slots.len() as u64);
        words.extend(self.slots.iter().map(|object| u64::from(object.0)));
        words.push(self.visible_variables.as_slice().len() as u64);
        words.extend(
            self.visible_variables
                .as_slice()
                .iter()
                .map(|value| *value as u64),
        );
        words.push(self.level_fired_rules.len() as u64);
        words.extend(self.level_fired_rules.iter().map(|rule| u64::from(rule.0)));
        words
    }

    #[inline]
    pub fn slots(&self) -> &[ObjectId] {
        &self.slots
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
    ) -> Result<(), GridStateError<D>> {
        self.visible_variables
            .set(variable, value)
            .map_err(map_variable_error::<D>)?;
        self.recompute_hash();
        Ok(())
    }

    pub fn update_visible_variable(
        &mut self,
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    ) -> Result<(), GridStateError<D>> {
        self.visible_variables
            .update(variable, op, value)
            .map_err(map_variable_error::<D>)?;
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
    pub fn slot_coord(&self, index: usize) -> Option<GridCoord<D>> {
        if index >= self.slots.len() {
            return None;
        }
        let cell = index / usize::from(self.layer_count);
        self.coord_from_cell_index(cell)
    }

    #[inline]
    pub fn cell_coord(&self, index: usize) -> Option<GridCoord<D>> {
        let cell_count = self.shape().cell_count()?;
        if index >= cell_count {
            return None;
        }
        self.coord_from_cell_index(index)
    }

    pub fn cell_view_at(
        &self,
        position: impl Into<GridCoord<D>>,
    ) -> Result<CellView, GridStateError<D>> {
        let position = position.into();
        self.check_pos(position)?;

        let mut objects = Vec::new();
        for layer in 0..self.layer_count {
            let object = self.slots[self.slot_index_unchecked(position, LayerId(layer))];
            if !object.is_empty() {
                objects.push(object);
            }
        }

        Ok(CellView { objects })
    }

    pub fn get_layer_at(
        &self,
        position: impl Into<GridCoord<D>>,
        layer: LayerId,
    ) -> Result<ObjectId, GridStateError<D>> {
        let index = self.slot_index(position.into(), layer)?;
        Ok(self.slots[index])
    }

    pub fn has_object_at<ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
    ) -> bool {
        let Some(layer) = game.object_layer(object) else {
            return false;
        };
        self.get_layer_at(position, layer)
            .is_ok_and(|found| found == object)
    }

    #[inline]
    pub fn cell_has_object_masked_at(
        &self,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
    ) -> Option<bool> {
        let index = self.cell_index(position.into()).ok()?;
        self.derived_cache.cell_object_masks[index].contains_raw(object.0)
    }

    pub fn object_count_masked(&self, object: ObjectId) -> Option<u32> {
        self.derived_cache
            .object_counts
            .get(usize::from(object.0))
            .copied()
    }

    pub fn place_object_at<ConditionDef, Rule, Condition, Frame>(
        &mut self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
    ) -> Result<(), GridStateError<D>> {
        let position = position.into();
        let layer = game
            .object_layer(object)
            .ok_or(GridStateError::UnknownObject { object })?;
        let index = self.slot_index(position, layer)?;
        let existing = self.slots[index];
        if !existing.is_empty() {
            return Err(GridStateError::LayerOccupied {
                position,
                layer,
                existing,
                attempted: object,
            });
        }

        self.set_slot_index_unchecked(index, object);
        self.recompute_hash();
        Ok(())
    }

    pub fn remove_object_at<ConditionDef, Rule, Condition, Frame>(
        &mut self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        position: impl Into<GridCoord<D>>,
        object: ObjectId,
    ) -> Result<(), GridStateError<D>> {
        let position = position.into();
        let layer = game
            .object_layer(object)
            .ok_or(GridStateError::UnknownObject { object })?;
        let index = self.slot_index(position, layer)?;
        if self.slots[index] != object {
            return Err(GridStateError::ObjectNotPresent { position, object });
        }
        self.set_slot_index_unchecked(index, ObjectId::EMPTY);
        self.recompute_hash();
        Ok(())
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
        let positions = &mut self.derived_cache.object_positions[index];
        match positions.binary_search(&slot_index) {
            Ok(_) => {}
            Err(insert_at) => positions.insert(insert_at, slot_index),
        }
    }

    fn rebuild_derived_cache(&mut self) {
        let object_count = self
            .slots
            .iter()
            .map(|object| usize::from(object.0))
            .max()
            .unwrap_or(0);
        let cell_count = self
            .shape()
            .cell_count()
            .expect("state dimensions are validated at construction");
        self.derived_cache = DerivedCache {
            object_counts: vec![0; object_count + 1],
            object_positions: vec![Vec::new(); object_count + 1],
            cell_object_masks: vec![ObjectCellMask::default(); cell_count],
        };
        let mut index = 0;
        while index < self.slots.len() {
            let object = self.slots[index];
            self.add_object_position(object, index);
            if !object.is_empty() {
                self.set_cell_object_mask(index, object);
            }
            index += 1;
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
            && let Ok(position_index) = positions.binary_search(&slot_index)
        {
            positions.remove(position_index);
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

    pub(crate) fn recompute_hash(&mut self) {
        let mut hash = puzzle_kernel::FnvBuilder::OFFSET;
        for axis in self.size.axes() {
            hash = fnv_mix(hash, u64::from(axis));
        }
        hash = fnv_mix(hash, u64::from(self.layer_count));
        for object in &self.slots {
            hash = fnv_mix(hash, u64::from(object.0));
        }
        for value in self.visible_variables.as_slice() {
            hash = fnv_mix(hash, *value as u64);
        }
        hash = fnv_mix(hash, self.level_fired_rules.len() as u64);
        for rule in &self.level_fired_rules {
            hash = fnv_mix(hash, u64::from(rule.0));
        }
        self.hash = hash;
    }

    pub(crate) fn check_pos(&self, position: GridCoord<D>) -> Result<(), GridStateError<D>> {
        if !self.shape().contains(position) {
            return Err(GridStateError::PositionOutOfBounds { position });
        }
        Ok(())
    }

    pub(crate) fn slot_index(
        &self,
        position: GridCoord<D>,
        layer: LayerId,
    ) -> Result<usize, GridStateError<D>> {
        self.check_pos(position)?;
        if layer.0 >= self.layer_count {
            return Err(GridStateError::LayerOutOfBounds { layer });
        }
        self.shape()
            .slot_index(position, layer.0)
            .ok_or(GridStateError::LayerOutOfBounds { layer })
    }

    pub(crate) fn cell_index(&self, position: GridCoord<D>) -> Result<usize, GridStateError<D>> {
        self.check_pos(position)?;
        Ok(self.shape().cell_index_unchecked(position))
    }

    #[inline]
    pub(crate) fn cell_index_unchecked(&self, position: GridCoord<D>) -> usize {
        self.shape().cell_index_unchecked(position)
    }

    #[inline]
    pub(crate) fn slot_index_unchecked(&self, position: GridCoord<D>, layer: LayerId) -> usize {
        self.shape().slot_index_unchecked(position, layer.0)
    }

    pub fn shape(&self) -> GridShape<D> {
        GridShape::new(self.size.axes(), self.layer_count)
            .expect("state dimensions are validated at construction")
    }

    fn coord_from_cell_index(&self, mut index: usize) -> Option<GridCoord<D>> {
        let axes = self.size.axes();
        let mut coord = [0u16; D];
        for axis in 0..D {
            let limit = usize::from(axes[axis]);
            coord[axis] = u16::try_from(index % limit).ok()?;
            index /= limit;
        }
        (index == 0).then_some(GridCoord::new(coord))
    }
}

impl GridState<2, Size2> {
    pub fn empty(
        width: u16,
        height: u16,
        layer_count: u16,
        object_count: usize,
    ) -> Result<Self, StateError> {
        Self::empty_sized(Size2::new(width, height), layer_count, object_count)
    }

    pub fn empty_with_variables(
        width: u16,
        height: u16,
        layer_count: u16,
        object_count: usize,
        visible_variables: Vec<i64>,
    ) -> Result<Self, StateError> {
        Self::empty_sized_with_variables(
            Size2::new(width, height),
            layer_count,
            object_count,
            visible_variables,
        )
    }

    pub fn slot_position(&self, index: usize) -> Option<(u16, u16)> {
        self.slot_coord(index).map(|position| {
            let [x, y] = position.axes();
            (x, y)
        })
    }

    pub fn cell_position(&self, index: usize) -> Option<(u16, u16)> {
        self.cell_coord(index).map(|position| {
            let [x, y] = position.axes();
            (x, y)
        })
    }

    pub fn cell_view(&self, x: u16, y: u16) -> Result<CellView, StateError> {
        self.cell_view_at(GridCoord::new([x, y]))
    }

    pub fn get_layer(&self, x: u16, y: u16, layer: LayerId) -> Result<ObjectId, StateError> {
        self.get_layer_at(GridCoord::new([x, y]), layer)
    }

    pub fn has_object(&self, game: &CompiledGame, x: u16, y: u16, object: ObjectId) -> bool {
        self.has_object_at(game, GridCoord::new([x, y]), object)
    }

    pub fn place_object(
        &mut self,
        game: &CompiledGame,
        x: u16,
        y: u16,
        object: ObjectId,
    ) -> Result<(), StateError> {
        self.place_object_at(game, GridCoord::new([x, y]), object)
    }
}

impl GridState<3, Size3> {
    pub fn empty(size: Size3, layer_count: u16) -> Result<Self, GridStateError<3>> {
        Self::empty_sized(size, layer_count, 0)
    }

    pub fn empty_with_variables(
        size: Size3,
        layer_count: u16,
        visible_variables: Vec<i64>,
    ) -> Result<Self, GridStateError<3>> {
        Self::empty_sized_with_variables(size, layer_count, 0, visible_variables)
    }
}

fn map_variable_error<const D: usize>(error: VariableValueError<VariableId>) -> GridStateError<D> {
    match error {
        VariableValueError::OutOfBounds { variable } => {
            GridStateError::VariableOutOfBounds { variable }
        }
        VariableValueError::Overflow { variable } => GridStateError::VariableOverflow { variable },
        VariableValueError::DivisionByZero { variable } => {
            GridStateError::VariableDivisionByZero { variable }
        }
    }
}

impl<const D: usize, Size: GridSize<D>> PartialEq for GridState<D, Size> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
            && self.layer_count == other.layer_count
            && self.slots == other.slots
            && self.visible_variables == other.visible_variables
            && self.level_fired_rules == other.level_fired_rules
    }
}

impl<const D: usize, Size: GridSize<D>> Eq for GridState<D, Size> {}
