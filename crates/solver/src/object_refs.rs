use crate::relevance::SolverRelevance;
use puzzle_kernel::{
    ConditionValueKind, LayerId, MarkId, ObjectId, RuleMatchCell, RulePattern,
    RulePatternComponent, RuleWriteOp,
};

pub trait SolverObjectId: Copy + Ord {
    fn is_empty(self) -> bool;
}

impl SolverObjectId for ObjectId {
    fn is_empty(self) -> bool {
        self.is_empty()
    }
}

pub trait SolverPatternObjectRefs {
    type ObjectId: SolverObjectId;

    fn for_each_solver_object_ref(&self, f: &mut impl FnMut(Self::ObjectId));

    fn solver_object_set_binding_touches(
        &self,
        binding: u16,
        is_relevant: &impl Fn(Self::ObjectId) -> bool,
    ) -> bool;
}

pub(crate) fn insert_condition_value_objects<ObjectId, Pattern, InputId>(
    relevance: &mut SolverRelevance<ObjectId>,
    kind: &ConditionValueKind<ObjectId, Pattern, InputId>,
) -> bool
where
    ObjectId: SolverObjectId,
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    match kind {
        ConditionValueKind::CountObjects(objects)
        | ConditionValueKind::ExistsObjects(objects)
        | ConditionValueKind::NoneObjects(objects) => insert_objects(relevance, objects),
        ConditionValueKind::CountMatches(patterns)
        | ConditionValueKind::ExistsMatches(patterns)
        | ConditionValueKind::NoneMatches(patterns) => insert_patterns_objects(relevance, patterns),
        ConditionValueKind::CountInputMatches(patterns)
        | ConditionValueKind::ExistsInputMatches(patterns)
        | ConditionValueKind::NoneInputMatches(patterns) => {
            insert_patterns_objects(relevance, patterns.iter().map(|(_, pattern)| pattern))
        }
    }
}

pub(crate) fn insert_patterns_objects<'a, ObjectId, Pattern>(
    relevance: &mut SolverRelevance<ObjectId>,
    patterns: impl IntoIterator<Item = &'a Pattern>,
) -> bool
where
    ObjectId: SolverObjectId + 'a,
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId> + 'a,
{
    let mut changed = false;
    for pattern in patterns {
        changed |= insert_pattern_objects(relevance, pattern);
    }
    changed
}

pub(crate) fn insert_pattern_objects<ObjectId, Pattern>(
    relevance: &mut SolverRelevance<ObjectId>,
    pattern: &Pattern,
) -> bool
where
    ObjectId: SolverObjectId,
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    let mut changed = false;
    pattern.for_each_solver_object_ref(&mut |object| {
        changed |= relevance.insert_relevant_object(object, &ObjectId::is_empty);
    });
    changed
}

pub(crate) fn write_touches_relevant_object<Offset, ObjectId, MarkId, Pattern>(
    write: &RuleWriteOp<Offset, ObjectId, MarkId>,
    pattern: &Pattern,
    relevance: &SolverRelevance<ObjectId>,
) -> bool
where
    ObjectId: SolverObjectId,
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    match write {
        RuleWriteOp::Add { object, .. }
        | RuleWriteOp::Remove { object, .. }
        | RuleWriteOp::Move { object, .. }
        | RuleWriteOp::SetMark { object, .. }
        | RuleWriteOp::RemoveMark { object, .. } => relevance.contains_object(*object),
        RuleWriteOp::Replace { remove, add, .. } => {
            relevance.contains_object(*remove) || relevance.contains_object(*add)
        }
        RuleWriteOp::AddObjectSet { binding, .. }
        | RuleWriteOp::RemoveObjectSet { binding, .. }
        | RuleWriteOp::MoveObjectSet { binding, .. }
        | RuleWriteOp::SetObjectSetMark { binding, .. }
        | RuleWriteOp::RemoveObjectSetMark { binding, .. } => pattern
            .solver_object_set_binding_touches(*binding, &|object| {
                relevance.contains_object(object)
            }),
    }
}

pub fn collect_condition_value_roots<ObjectId, Pattern, InputId>(
    kind: &ConditionValueKind<ObjectId, Pattern, InputId>,
    roots: &mut std::collections::BTreeSet<ObjectId>,
) where
    ObjectId: SolverObjectId,
    Pattern: SolverPatternObjectRefs<ObjectId = ObjectId>,
{
    match kind {
        ConditionValueKind::CountObjects(objects)
        | ConditionValueKind::ExistsObjects(objects)
        | ConditionValueKind::NoneObjects(objects) => collect_objects(objects, roots),
        ConditionValueKind::CountMatches(patterns)
        | ConditionValueKind::ExistsMatches(patterns)
        | ConditionValueKind::NoneMatches(patterns) => collect_patterns_roots(patterns, roots),
        ConditionValueKind::CountInputMatches(patterns)
        | ConditionValueKind::ExistsInputMatches(patterns)
        | ConditionValueKind::NoneInputMatches(patterns) => {
            collect_patterns_roots(patterns.iter().map(|(_, pattern)| pattern), roots)
        }
    }
}

pub fn collect_pattern_roots<Pattern>(
    pattern: &Pattern,
    roots: &mut std::collections::BTreeSet<Pattern::ObjectId>,
) where
    Pattern: SolverPatternObjectRefs,
{
    pattern.for_each_solver_object_ref(&mut |object| {
        if !object.is_empty() {
            roots.insert(object);
        }
    });
}

fn insert_objects<ObjectId>(relevance: &mut SolverRelevance<ObjectId>, objects: &[ObjectId]) -> bool
where
    ObjectId: SolverObjectId,
{
    relevance.insert_relevant_objects(objects, &ObjectId::is_empty)
}

fn collect_objects<ObjectId>(objects: &[ObjectId], roots: &mut std::collections::BTreeSet<ObjectId>)
where
    ObjectId: SolverObjectId,
{
    roots.extend(objects.iter().copied().filter(|object| !object.is_empty()));
}

fn collect_patterns_roots<'a, Pattern>(
    patterns: impl IntoIterator<Item = &'a Pattern>,
    roots: &mut std::collections::BTreeSet<Pattern::ObjectId>,
) where
    Pattern: SolverPatternObjectRefs + 'a,
{
    for pattern in patterns {
        collect_pattern_roots(pattern, roots);
    }
}

impl<Offset> SolverPatternObjectRefs
    for RulePattern<RulePatternComponent<RuleMatchCell<Offset, ObjectId, LayerId, MarkId>>>
{
    type ObjectId = ObjectId;

    fn for_each_solver_object_ref(&self, f: &mut impl FnMut(Self::ObjectId)) {
        for cell in self
            .components
            .iter()
            .flat_map(|component| &component.cells)
        {
            collect_cell_object_refs(cell, f);
        }
    }

    fn solver_object_set_binding_touches(
        &self,
        binding: u16,
        is_relevant: &impl Fn(Self::ObjectId) -> bool,
    ) -> bool {
        self.components
            .iter()
            .flat_map(|component| &component.cells)
            .any(|cell| cell_binding_touches(cell, binding, is_relevant))
    }
}

fn collect_cell_object_refs<Offset>(
    cell: &RuleMatchCell<Offset, ObjectId, LayerId, MarkId>,
    f: &mut impl FnMut(ObjectId),
) {
    for object in cell.require_objects.iter().chain(&cell.forbid_objects) {
        f(*object);
    }
    for matcher in &cell.require_object_sets {
        for object in &matcher.objects {
            f(*object);
        }
    }
    for mark in cell.require_mark.iter().chain(&cell.forbid_mark) {
        f(mark.object);
    }
}

fn cell_binding_touches<Offset>(
    cell: &RuleMatchCell<Offset, ObjectId, LayerId, MarkId>,
    binding: u16,
    is_relevant: &impl Fn(ObjectId) -> bool,
) -> bool {
    cell.require_object_sets.iter().any(|matcher| {
        matcher.binding == binding && matcher.objects.iter().copied().any(is_relevant)
    })
}
