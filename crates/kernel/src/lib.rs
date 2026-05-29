use std::marker::PhantomData;
use std::num::NonZeroU32;

pub trait KernelId: Copy {
    fn raw(self) -> u16;

    #[inline]
    fn index(self) -> usize {
        usize::from(self.raw())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectCellMask(u64);

impl ObjectCellMask {
    #[inline]
    pub fn can_represent_raw(object: u16) -> bool {
        object != 0 && object < 64
    }

    #[inline]
    pub fn contains_raw(self, object: u16) -> Option<bool> {
        object_bit(object).map(|bit| (self.0 & bit) != 0)
    }

    #[inline]
    pub fn insert_raw(&mut self, object: u16) {
        if let Some(bit) = object_bit(object) {
            self.0 |= bit;
        }
    }

    #[inline]
    pub fn remove_raw(&mut self, object: u16) {
        if let Some(bit) = object_bit(object) {
            self.0 &= !bit;
        }
    }
}

#[inline]
fn object_bit(object: u16) -> Option<u64> {
    ObjectCellMask::can_represent_raw(object).then(|| 1u64 << u32::from(object))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalFrameExtent {
    Radius(u16),
    Full,
}

impl LocalFrameExtent {
    #[inline]
    pub fn contains_delta(self, delta: i32) -> bool {
        match self {
            Self::Radius(radius) => delta.unsigned_abs() <= u32::from(radius),
            Self::Full => true,
        }
    }

    #[inline]
    pub fn bounded_range(self, center: u16, limit: u16) -> std::ops::RangeInclusive<u16> {
        match self {
            Self::Full => 0..=limit.saturating_sub(1),
            Self::Radius(radius) => {
                let min = center.saturating_sub(radius);
                let max = center.saturating_add(radius).min(limit.saturating_sub(1));
                min..=max
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFrame<ObjectId> {
    pub x: LocalFrameExtent,
    pub y: LocalFrameExtent,
    pub z: LocalFrameExtent,
    pub focus_objects: Vec<ObjectId>,
}

impl<ObjectId> LocalFrame<ObjectId> {
    pub fn new(
        x: LocalFrameExtent,
        y: LocalFrameExtent,
        z: LocalFrameExtent,
        focus_objects: Vec<ObjectId>,
    ) -> Self {
        Self {
            x,
            y,
            z,
            focus_objects,
        }
    }

    #[inline]
    pub fn contains_delta_2d(&self, dx: i32, dy: i32) -> bool {
        self.x.contains_delta(dx) && self.y.contains_delta(dy)
    }

    #[inline]
    pub fn contains_delta_3d(&self, dx: i32, dy: i32, dz: i32) -> bool {
        self.x.contains_delta(dx) && self.y.contains_delta(dy) && self.z.contains_delta(dz)
    }

    #[inline]
    pub fn ranges_2d(
        &self,
        focus_x: u16,
        focus_y: u16,
        width: u16,
        height: u16,
    ) -> (std::ops::RangeInclusive<u16>, std::ops::RangeInclusive<u16>) {
        (
            self.x.bounded_range(focus_x, width),
            self.y.bounded_range(focus_y, height),
        )
    }

    #[inline]
    pub fn ranges_3d(
        &self,
        focus_x: u16,
        focus_y: u16,
        focus_z: u16,
        width: u16,
        depth: u16,
        height: u16,
    ) -> (
        std::ops::RangeInclusive<u16>,
        std::ops::RangeInclusive<u16>,
        std::ops::RangeInclusive<u16>,
    ) {
        (
            self.x.bounded_range(focus_x, width),
            self.y.bounded_range(focus_y, depth),
            self.z.bounded_range(focus_z, height),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalUpdateOp {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalValueError<GlobalId> {
    OutOfBounds { global: GlobalId },
    Overflow { global: GlobalId },
    DivisionByZero { global: GlobalId },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisibleGlobals<GlobalId> {
    values: Vec<i64>,
    _id: PhantomData<GlobalId>,
}

impl<GlobalId: KernelId> VisibleGlobals<GlobalId> {
    pub fn new(values: Vec<i64>) -> Self {
        Self {
            values,
            _id: PhantomData,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[i64] {
        &self.values
    }

    #[inline]
    pub fn get(&self, global: GlobalId) -> Option<i64> {
        self.values.get(global.index()).copied()
    }

    pub fn set(&mut self, global: GlobalId, value: i64) -> Result<(), GlobalValueError<GlobalId>> {
        let slot = self
            .values
            .get_mut(global.index())
            .ok_or(GlobalValueError::OutOfBounds { global })?;
        *slot = value;
        Ok(())
    }

    pub fn update(
        &mut self,
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    ) -> Result<(), GlobalValueError<GlobalId>> {
        let slot = self
            .values
            .get_mut(global.index())
            .ok_or(GlobalValueError::OutOfBounds { global })?;
        *slot = apply_global_update(*slot, op, value, global)?;
        Ok(())
    }
}

fn apply_global_update<GlobalId: Copy>(
    current: i64,
    op: GlobalUpdateOp,
    value: i64,
    global: GlobalId,
) -> Result<i64, GlobalValueError<GlobalId>> {
    match op {
        GlobalUpdateOp::Set => Ok(value),
        GlobalUpdateOp::Add => current
            .checked_add(value)
            .ok_or(GlobalValueError::Overflow { global }),
        GlobalUpdateOp::Subtract => current
            .checked_sub(value)
            .ok_or(GlobalValueError::Overflow { global }),
        GlobalUpdateOp::Multiply => current
            .checked_mul(value)
            .ok_or(GlobalValueError::Overflow { global }),
        GlobalUpdateOp::Divide => {
            if value == 0 {
                return Err(GlobalValueError::DivisionByZero { global });
            }
            current
                .checked_div(value)
                .ok_or(GlobalValueError::Overflow { global })
        }
        GlobalUpdateOp::Remainder => {
            if value == 0 {
                return Err(GlobalValueError::DivisionByZero { global });
            }
            current
                .checked_rem(value)
                .ok_or(GlobalValueError::Overflow { global })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchKind {
    Marker,
    Bool,
    Int,
    Enum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchValueMatch {
    Any,
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectSetMatcher<ObjectId, LayerId> {
    pub binding: u16,
    pub layer: LayerId,
    pub objects: Vec<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectSetScratchPattern<ScratchId> {
    pub binding: u16,
    pub scratch: ScratchId,
    pub value: Option<i64>,
    pub match_value: ScratchValueMatch,
}

pub fn object_set_matcher_for_same_layer<ObjectId, LayerId>(
    binding: u16,
    objects: &[ObjectId],
    mut object_layer: impl FnMut(ObjectId) -> Option<LayerId>,
) -> Option<ObjectSetMatcher<ObjectId, LayerId>>
where
    ObjectId: Copy,
    LayerId: Copy + Eq,
{
    let (&first, rest) = objects.split_first()?;
    let layer = object_layer(first)?;
    for object in rest {
        if object_layer(*object)? != layer {
            return None;
        }
    }
    Some(ObjectSetMatcher {
        binding,
        layer,
        objects: objects.to_vec(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchValue<ScratchId> {
    pub scratch: ScratchId,
    pub value: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct ScratchEntry<ScratchId> {
    scratch: ScratchValue<ScratchId>,
    next: Option<NonZeroU32>,
}

#[derive(Clone)]
pub struct ScratchIter<'a, ScratchId> {
    entries: &'a [ScratchEntry<ScratchId>],
    next: Option<NonZeroU32>,
}

impl<ScratchId: Copy> Iterator for ScratchIter<'_, ScratchId> {
    type Item = ScratchValue<ScratchId>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        let entry = self.entries.get(scratch_entry_index(id))?;
        self.next = entry.next;
        Some(entry.scratch)
    }
}

#[derive(Clone, Debug)]
pub struct ScratchSpace<ScratchId> {
    cell_heads: Vec<Option<NonZeroU32>>,
    slot_heads: Vec<Option<NonZeroU32>>,
    entries: Vec<ScratchEntry<ScratchId>>,
    free_entries: Vec<NonZeroU32>,
}

impl<ScratchId> ScratchSpace<ScratchId> {
    pub fn new(cell_count: usize, slot_count: usize) -> Self {
        Self {
            cell_heads: vec![None; cell_count],
            slot_heads: vec![None; slot_count],
            entries: Vec::new(),
            free_entries: Vec::new(),
        }
    }

    #[inline]
    pub fn cell_count(&self) -> usize {
        self.cell_heads.len()
    }

    #[inline]
    pub fn slot_count(&self) -> usize {
        self.slot_heads.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slot_heads.iter().all(Option::is_none) && self.cell_heads.iter().all(Option::is_none)
    }
}

impl<ScratchId: Copy> ScratchSpace<ScratchId> {
    pub fn cell_values(&self) -> Vec<Vec<ScratchValue<ScratchId>>> {
        (0..self.cell_heads.len())
            .map(|index| self.cell_at(index).collect())
            .collect()
    }

    pub fn slot_values(&self) -> Vec<Vec<ScratchValue<ScratchId>>> {
        (0..self.slot_heads.len())
            .map(|index| self.slot_at(index).collect())
            .collect()
    }

    #[inline]
    pub fn cell_at(&self, index: usize) -> ScratchIter<'_, ScratchId> {
        ScratchIter {
            entries: &self.entries,
            next: self.cell_heads.get(index).copied().flatten(),
        }
    }

    #[inline]
    pub fn slot_at(&self, index: usize) -> ScratchIter<'_, ScratchId> {
        ScratchIter {
            entries: &self.entries,
            next: self.slot_heads.get(index).copied().flatten(),
        }
    }

    pub fn has_cell(&self, index: usize, scratch: ScratchId, value: Option<i64>) -> bool
    where
        ScratchId: PartialEq,
    {
        self.cell_at(index)
            .any(|entry| entry.scratch == scratch && entry.value == value)
    }

    pub fn has_cell_key(&self, index: usize, scratch: ScratchId) -> bool
    where
        ScratchId: PartialEq,
    {
        self.cell_at(index).any(|entry| entry.scratch == scratch)
    }

    pub fn has_slot(&self, index: usize, scratch: ScratchId, value: Option<i64>) -> bool
    where
        ScratchId: PartialEq,
    {
        self.slot_at(index)
            .any(|entry| entry.scratch == scratch && entry.value == value)
    }

    pub fn has_slot_key(&self, index: usize, scratch: ScratchId) -> bool
    where
        ScratchId: PartialEq,
    {
        self.slot_at(index).any(|entry| entry.scratch == scratch)
    }

    pub fn set_cell(&mut self, index: usize, scratch: ScratchId, value: Option<i64>)
    where
        ScratchId: PartialEq,
    {
        set_scratch(
            &mut self.cell_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            scratch,
            value,
        );
    }

    pub fn set_slot(&mut self, index: usize, scratch: ScratchId, value: Option<i64>)
    where
        ScratchId: PartialEq,
    {
        set_scratch(
            &mut self.slot_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            scratch,
            value,
        );
    }

    pub fn remove_cell(&mut self, index: usize, scratch: ScratchId, value: Option<i64>)
    where
        ScratchId: PartialEq,
    {
        let mut entries = self.take_cell(index);
        retain_scratch(&mut entries, scratch, value);
        self.replace_cell(index, entries);
    }

    pub fn remove_slot(&mut self, index: usize, scratch: ScratchId, value: Option<i64>)
    where
        ScratchId: PartialEq,
    {
        let mut entries = self.take_slot(index);
        retain_scratch(&mut entries, scratch, value);
        self.replace_slot(index, entries);
    }

    pub fn take_cell(&mut self, index: usize) -> Vec<ScratchValue<ScratchId>> {
        let scratch = self.cell_at(index).collect::<Vec<_>>();
        self.clear_cell(index);
        scratch
    }

    pub fn take_slot(&mut self, index: usize) -> Vec<ScratchValue<ScratchId>> {
        let scratch = self.slot_at(index).collect::<Vec<_>>();
        self.clear_slot(index);
        scratch
    }

    pub fn replace_cell(&mut self, index: usize, scratch: Vec<ScratchValue<ScratchId>>) {
        self.clear_cell(index);
        for scratch in scratch {
            self.push_cell(index, scratch);
        }
    }

    pub fn replace_slot(&mut self, index: usize, scratch: Vec<ScratchValue<ScratchId>>) {
        self.clear_slot(index);
        for scratch in scratch {
            self.push_slot(index, scratch);
        }
    }

    pub fn clear_cell(&mut self, index: usize) {
        clear_head(
            &mut self.cell_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
        );
    }

    pub fn clear_slot(&mut self, index: usize) {
        clear_head(
            &mut self.slot_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
        );
    }

    pub fn clear_all(&mut self) {
        self.slot_heads.fill(None);
        self.cell_heads.fill(None);
        self.entries.clear();
        self.free_entries.clear();
    }

    pub fn hash_into<F>(&self, mut hash: u64, mut scratch_raw: F) -> u64
    where
        F: FnMut(ScratchId) -> u64,
    {
        for index in 0..self.cell_heads.len() {
            hash = hash_scratch_iter(hash, self.cell_at(index), &mut scratch_raw);
        }
        for index in 0..self.slot_heads.len() {
            hash = hash_scratch_iter(hash, self.slot_at(index), &mut scratch_raw);
        }
        hash
    }

    fn push_cell(&mut self, index: usize, scratch: ScratchValue<ScratchId>) {
        push_scratch(
            &mut self.cell_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            scratch,
        );
    }

    fn push_slot(&mut self, index: usize, scratch: ScratchValue<ScratchId>) {
        push_scratch(
            &mut self.slot_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            scratch,
        );
    }
}

impl<ScratchId: Copy + PartialEq> PartialEq for ScratchSpace<ScratchId> {
    fn eq(&self, other: &Self) -> bool {
        self.cell_heads.len() == other.cell_heads.len()
            && self.slot_heads.len() == other.slot_heads.len()
            && (0..self.cell_heads.len()).all(|index| self.cell_at(index).eq(other.cell_at(index)))
            && (0..self.slot_heads.len()).all(|index| self.slot_at(index).eq(other.slot_at(index)))
    }
}

impl<ScratchId: Copy + Eq> Eq for ScratchSpace<ScratchId> {}

fn set_scratch<ScratchId: Copy + PartialEq>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut Vec<ScratchEntry<ScratchId>>,
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
    scratch: ScratchId,
    value: Option<i64>,
) {
    let mut current = heads[index];
    while let Some(id) = current {
        let entry_index = scratch_entry_index(id);
        let entry = &mut entries[entry_index];
        if entry.scratch.scratch == scratch {
            entry.scratch.value = value;
            return;
        }
        current = entry.next;
    }
    push_scratch(
        heads,
        entries,
        free_entries,
        index,
        ScratchValue { scratch, value },
    );
}

fn retain_scratch<ScratchId: PartialEq>(
    entries: &mut Vec<ScratchValue<ScratchId>>,
    scratch: ScratchId,
    value: Option<i64>,
) {
    entries.retain(|entry| {
        if entry.scratch != scratch {
            return true;
        }
        value.is_some_and(|value| entry.value != Some(value))
    });
}

fn push_scratch<ScratchId: Copy>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut Vec<ScratchEntry<ScratchId>>,
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
    scratch: ScratchValue<ScratchId>,
) {
    let new_id = allocate_scratch_entry(entries, free_entries, scratch);
    let Some(mut current) = heads[index] else {
        heads[index] = Some(new_id);
        return;
    };

    loop {
        let entry_index = scratch_entry_index(current);
        let Some(next) = entries[entry_index].next else {
            entries[entry_index].next = Some(new_id);
            return;
        };
        current = next;
    }
}

fn clear_head<ScratchId>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut [ScratchEntry<ScratchId>],
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
) {
    let mut current = heads[index].take();
    while let Some(id) = current {
        let entry_index = scratch_entry_index(id);
        current = entries[entry_index].next;
        entries[entry_index].next = None;
        free_entries.push(id);
    }
}

fn allocate_scratch_entry<ScratchId: Copy>(
    entries: &mut Vec<ScratchEntry<ScratchId>>,
    free_entries: &mut Vec<NonZeroU32>,
    scratch: ScratchValue<ScratchId>,
) -> NonZeroU32 {
    if let Some(id) = free_entries.pop() {
        let index = scratch_entry_index(id);
        entries[index] = ScratchEntry {
            scratch,
            next: None,
        };
        return id;
    }

    let raw_index = entries.len();
    let id = scratch_entry_id(raw_index);
    entries.push(ScratchEntry {
        scratch,
        next: None,
    });
    id
}

fn hash_scratch_iter<ScratchId: Copy, F>(
    mut hash: u64,
    scratch: ScratchIter<'_, ScratchId>,
    scratch_raw: &mut F,
) -> u64
where
    F: FnMut(ScratchId) -> u64,
{
    let count = scratch.clone().count();
    hash = fnv_mix(hash, count as u64);
    for scratch in scratch {
        hash = fnv_mix(hash, scratch_raw(scratch.scratch));
        hash = fnv_mix(hash, scratch.value.unwrap_or(i64::MIN) as u64);
    }
    hash
}

fn scratch_entry_index(id: NonZeroU32) -> usize {
    usize::try_from(id.get() - 1).expect("scratch entry id must fit usize")
}

fn scratch_entry_id(index: usize) -> NonZeroU32 {
    let raw = u32::try_from(index + 1).expect("too many scratch entries");
    NonZeroU32::new(raw).expect("scratch entry ids are one-based")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryKind<ObjectId, Pattern, InputId> {
    CountObjects(Vec<ObjectId>),
    ExistsObjects(Vec<ObjectId>),
    NoneObjects(Vec<ObjectId>),
    CountMatches(Vec<Pattern>),
    ExistsMatches(Vec<Pattern>),
    NoneMatches(Vec<Pattern>),
    CountInputMatches(Vec<(InputId, Pattern)>),
    ExistsInputMatches(Vec<(InputId, Pattern)>),
    NoneInputMatches(Vec<(InputId, Pattern)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRoleSet<Id> {
    ids: Vec<Id>,
}

impl<Id: Copy + Ord> ObjectRoleSet<Id> {
    pub fn new(mut ids: Vec<Id>) -> Self {
        ids.sort();
        ids.dedup();
        Self { ids }
    }

    pub fn contains(&self, id: Id) -> bool {
        self.ids.binary_search(&id).is_ok()
    }

    pub fn as_slice(&self) -> &[Id] {
        &self.ids
    }
}

impl<Id> Default for ObjectRoleSet<Id> {
    fn default() -> Self {
        Self { ids: Vec::new() }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FnvBuilder {
    hash: u64,
}

impl FnvBuilder {
    pub const OFFSET: u64 = 0xcbf29ce484222325;

    pub fn new() -> Self {
        Self { hash: Self::OFFSET }
    }

    pub fn push(&mut self, value: u64) {
        self.hash = fnv_mix(self.hash, value);
    }

    pub fn finish(self) -> u64 {
        self.hash
    }
}

impl Default for FnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn fnv_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestId(u16);

    impl KernelId for TestId {
        fn raw(self) -> u16 {
            self.0
        }
    }

    #[test]
    fn visible_globals_update_with_checked_arithmetic() {
        let mut globals = VisibleGlobals::new(vec![4]);
        globals.update(TestId(0), GlobalUpdateOp::Add, 3).unwrap();
        assert_eq!(globals.get(TestId(0)), Some(7));
        assert!(matches!(
            globals.update(TestId(0), GlobalUpdateOp::Divide, 0),
            Err(GlobalValueError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn scratch_space_moves_values_without_preserving_free_list_identity() {
        let mut scratch = ScratchSpace::new(2, 2);
        scratch.set_slot(0, TestId(1), Some(9));
        let moved = scratch.take_slot(0);
        scratch.replace_slot(1, moved);

        let mut expected = ScratchSpace::new(2, 2);
        expected.set_slot(1, TestId(1), Some(9));

        assert_eq!(scratch, expected);
        assert!(scratch.has_slot(1, TestId(1), Some(9)));
    }
}
