use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};

mod program;
pub use program::{
    ExecutableProgram, ProgramContinuation, ProgramNodeId, ProgramSegment, ProgramStateKey,
    execute_program_segment, resume_program_segment,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpatialVector<const D: usize>([i16; D]);

impl<const D: usize> SpatialVector<D> {
    pub const fn new(axes: [i16; D]) -> Self {
        Self(axes)
    }

    pub const fn axes(self) -> [i16; D] {
        self.0
    }
}

impl<const D: usize> From<[i16; D]> for SpatialVector<D> {
    fn from(axes: [i16; D]) -> Self {
        Self::new(axes)
    }
}

impl<const D: usize> Serialize for SpatialVector<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de, const D: usize> Deserialize<'de> for SpatialVector<D> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: serde::Deserializer<'de>,
    {
        let values = Vec::<i16>::deserialize(deserializer)?;
        let found = values.len();
        let axes = values
            .try_into()
            .map_err(|_| De::Error::custom(format!("expected {D} spatial axes, found {found}")))?;
        Ok(Self(axes))
    }
}

#[derive(Clone, Debug)]
pub struct TransitionOutcome<Input, State, Command, RuleId, Patch> {
    pub input: Input,
    pub next_state: State,
    pub cancelled: bool,
    pub commands: Vec<Command>,
    pub fired_rules: Vec<RuleId>,
    pub patches: Vec<Patch>,
}

pub trait KernelId: Copy {
    fn raw(self) -> u16;

    #[inline]
    fn index(self) -> usize {
        usize::from(self.raw())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VariableId(pub u16);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RuleId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConditionId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MarkId(pub u16);

impl KernelId for MarkId {
    fn raw(self) -> u16 {
        self.0
    }
}

impl KernelId for VariableId {
    fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InputId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ObjectId(pub u16);

impl ObjectId {
    pub const EMPTY: Self = Self(0);

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct LayerId(pub u16);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDef {
    pub id: ObjectId,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDef {
    pub id: MarkId,
    pub kind: MarkKind,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleEffect {
    Cancel,
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    UpdateVariable {
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramCondition<Pattern, Guard> {
    AnyMatches(Vec<Pattern>),
    NoMatches(Vec<Pattern>),
    AnyInputMatches(Vec<(InputId, Pattern)>),
    NoInputMatches(Vec<(InputId, Pattern)>),
    GuardBranches(Vec<Vec<Guard>>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpatialOffset<const D: usize> {
    Fixed {
        delta: SpatialVector<D>,
    },
    Variable {
        base: SpatialVector<D>,
        gap_terms: Vec<SpatialGapTerm<D>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpatialGapTerm<const D: usize> {
    pub gap_index: u16,
    pub delta: SpatialVector<D>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridCoord<const D: usize> {
    axes: [u16; D],
}

impl<const D: usize> Serialize for GridCoord<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.axes.as_slice().serialize(serializer)
    }
}

impl<'de, const D: usize> Deserialize<'de> for GridCoord<D> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: serde::Deserializer<'de>,
    {
        let values = Vec::<u16>::deserialize(deserializer)?;
        let found = values.len();
        let axes = values
            .try_into()
            .map_err(|_| De::Error::custom(format!("expected {D} grid axes, found {found}")))?;
        Ok(Self { axes })
    }
}

impl<const D: usize> GridCoord<D> {
    pub const fn new(axes: [u16; D]) -> Self {
        Self { axes }
    }

    pub const fn axes(self) -> [u16; D] {
        self.axes
    }

    pub fn checked_offset(self, offset: GridOffset<D>) -> Option<Self> {
        let mut axes = self.axes;
        let deltas = offset.deltas();
        let mut index = 0;
        while index < D {
            axes[index] = offset_axis(axes[index], deltas[index])?;
            index += 1;
        }
        Some(Self { axes })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridOffset<const D: usize> {
    deltas: [i16; D],
}

impl<const D: usize> GridOffset<D> {
    pub const ZERO: Self = Self { deltas: [0; D] };

    pub const fn new(deltas: [i16; D]) -> Self {
        Self { deltas }
    }

    pub const fn deltas(self) -> [i16; D] {
        self.deltas
    }

    pub fn scale(self, factor: i16) -> Self {
        Self {
            deltas: self.deltas.map(|delta| delta * factor),
        }
    }

    pub fn add(self, other: Self) -> Self {
        let mut deltas = self.deltas;
        let other = other.deltas;
        let mut index = 0;
        while index < D {
            deltas[index] += other[index];
            index += 1;
        }
        Self { deltas }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridShape<const D: usize> {
    axes: [u16; D],
    layer_count: u16,
}

impl<const D: usize> GridShape<D> {
    pub fn new(axes: [u16; D], layer_count: u16) -> Option<Self> {
        if layer_count == 0 || axes.iter().any(|axis| *axis == 0) {
            return None;
        }
        let shape = Self { axes, layer_count };
        shape.slot_count()?;
        Some(shape)
    }

    pub const fn axes(self) -> [u16; D] {
        self.axes
    }

    pub const fn layer_count(self) -> u16 {
        self.layer_count
    }

    pub fn cell_count(self) -> Option<usize> {
        self.axes
            .iter()
            .try_fold(1usize, |count, axis| count.checked_mul(usize::from(*axis)))
    }

    pub fn slot_count(self) -> Option<usize> {
        self.cell_count()?
            .checked_mul(usize::from(self.layer_count))
    }

    pub fn contains(self, coord: GridCoord<D>) -> bool {
        coord
            .axes()
            .iter()
            .zip(self.axes.iter())
            .all(|(value, limit)| value < limit)
    }

    pub fn cell_index(self, coord: GridCoord<D>) -> Option<usize> {
        self.contains(coord)
            .then(|| self.cell_index_unchecked(coord))
    }

    pub fn slot_index(self, coord: GridCoord<D>, layer: u16) -> Option<usize> {
        if layer >= self.layer_count {
            return None;
        }
        Some(self.slot_index_unchecked(coord, layer))
    }

    pub fn cell_index_unchecked(self, coord: GridCoord<D>) -> usize {
        let axes = coord.axes();
        let mut index = 0usize;
        let mut stride = 1usize;
        let mut axis_index = 0;
        while axis_index < D {
            index += usize::from(axes[axis_index]) * stride;
            stride *= usize::from(self.axes[axis_index]);
            axis_index += 1;
        }
        index
    }

    pub fn slot_index_unchecked(self, coord: GridCoord<D>, layer: u16) -> usize {
        self.cell_index_unchecked(coord) * usize::from(self.layer_count) + usize::from(layer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridPatchOp<Position, ObjectId, VariableId, MarkId> {
    Add {
        position: Position,
        object: ObjectId,
    },
    Remove {
        position: Position,
        object: ObjectId,
    },
    Move {
        from: Position,
        to: Position,
        object: ObjectId,
    },
    Replace {
        position: Position,
        remove: ObjectId,
        add: ObjectId,
    },
    UpdateVariable {
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    },
    SetMark {
        position: Position,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    },
    RemoveMark {
        position: Position,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
        match_value: MarkValueMatch,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectBinding<ObjectId> {
    pub binding: u16,
    pub object: ObjectId,
}

pub fn bind_object<ObjectId: Copy + Eq>(
    bindings: &mut Vec<ObjectBinding<ObjectId>>,
    binding: u16,
    object: ObjectId,
) -> bool {
    if let Some(existing) = bindings
        .iter()
        .find(|existing| existing.binding == binding)
        .map(|existing| existing.object)
    {
        return existing == object;
    }
    bindings.push(ObjectBinding { binding, object });
    true
}

pub fn bound_object<ObjectId: Copy>(
    bindings: &[ObjectBinding<ObjectId>],
    binding: u16,
) -> Option<ObjectId> {
    bindings
        .iter()
        .find(|existing| existing.binding == binding)
        .map(|existing| existing.object)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchPlacement<const D: usize, ObjectId> {
    pub components: Vec<ComponentPlacement<D, ObjectId>>,
}

impl<const D: usize, ObjectId> MatchPlacement<D, ObjectId> {
    pub fn empty() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn new(components: Vec<ComponentPlacement<D, ObjectId>>) -> Self {
        Self { components }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentPlacement<const D: usize, ObjectId> {
    pub origin: GridCoord<D>,
    pub gaps: Vec<u16>,
    pub object_bindings: Vec<ObjectBinding<ObjectId>>,
}

impl<const D: usize, ObjectId> ComponentPlacement<D, ObjectId> {
    pub fn new(
        origin: GridCoord<D>,
        gaps: Vec<u16>,
        object_bindings: Vec<ObjectBinding<ObjectId>>,
    ) -> Self {
        Self {
            origin,
            gaps,
            object_bindings,
        }
    }
}

pub fn placement_object_binding<const D: usize, ObjectId: Copy>(
    placement: &MatchPlacement<D, ObjectId>,
    binding: u16,
) -> Option<ObjectId> {
    placement
        .components
        .iter()
        .flat_map(|component| &component.object_bindings)
        .find(|object_binding| object_binding.binding == binding)
        .map(|object_binding| object_binding.object)
}

pub fn write_position_for_components<const D: usize, ObjectId, Offset>(
    components: &[ComponentPlacement<D, ObjectId>],
    component: u16,
    offset: &Offset,
    resolve_offset: impl FnOnce(&Offset, &[u16]) -> Option<GridOffset<D>>,
) -> Option<GridCoord<D>> {
    let placement = components.get(usize::from(component))?;
    let offset = resolve_offset(offset, &placement.gaps)?;
    placement.origin.checked_offset(offset)
}

pub fn write_position<const D: usize, ObjectId, Offset, Error>(
    placement: &MatchPlacement<D, ObjectId>,
    component: u16,
    offset: &Offset,
    resolve_offset: impl FnOnce(&Offset, &[u16]) -> Option<GridOffset<D>>,
    error: impl FnOnce() -> Error,
) -> Result<GridCoord<D>, Error> {
    write_position_for_components(&placement.components, component, offset, resolve_offset)
        .ok_or_else(error)
}

pub fn complete_component_placements<Component, Placement, Candidate>(
    components: &[Component],
    component_index: usize,
    placements: &mut Vec<Placement>,
    candidate_origins: &mut impl FnMut(&Component) -> Vec<Candidate>,
    place_at: &mut impl FnMut(&Component, Candidate) -> Option<Placement>,
) -> bool {
    if component_index == components.len() {
        return true;
    }

    let component = &components[component_index];
    for origin in candidate_origins(component) {
        if let Some(placement) = place_at(component, origin) {
            placements.push(placement);
            if complete_component_placements(
                components,
                component_index + 1,
                placements,
                candidate_origins,
                place_at,
            ) {
                return true;
            }
            placements.pop();
        }
    }

    false
}

pub fn collect_component_placements<Component, Placement, Candidate, Match>(
    components: &[Component],
    component_index: usize,
    placements: &mut Vec<Placement>,
    matches: &mut Vec<Match>,
    candidate_origins: &mut impl FnMut(&Component) -> Vec<Candidate>,
    place_at: &mut impl FnMut(&Component, Candidate) -> Option<Placement>,
    push_match: &mut impl FnMut(&mut Vec<Match>, &[Placement]),
) {
    if component_index == components.len() {
        push_match(matches, placements);
        return;
    }

    let component = &components[component_index];
    for origin in candidate_origins(component) {
        if let Some(placement) = place_at(component, origin) {
            placements.push(placement);
            collect_component_placements(
                components,
                component_index + 1,
                placements,
                matches,
                candidate_origins,
                place_at,
                push_match,
            );
            placements.pop();
        }
    }
}

fn offset_axis(value: u16, delta: i16) -> Option<u16> {
    let next = i32::from(value) + i32::from(delta);
    if next < 0 || next > i32::from(u16::MAX) {
        return None;
    }
    Some(next as u16)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableUpdateOp {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableEffect<VariableId> {
    UpdateVariable {
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleApplication {
    Once,
    OnceAll,
    OncePerLevel,
    Random,
    #[default]
    UntilStable,
}

/// Dimension-independent program structure.
///
/// Spatial backends own `Rule`, `Condition`, and `Frame`; the source statement
/// boundary and its application semantics stay shared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramStep<Rule, Condition, Frame> {
    Rule(Rule),
    ConditionalBlock {
        condition: Condition,
        steps: Vec<Self>,
    },
    ConditionalBranch {
        condition: Condition,
        then_steps: Vec<Self>,
        else_steps: Vec<Self>,
    },
    Block {
        application: RuleApplication,
        stop_condition: Option<Condition>,
        steps: Vec<Self>,
    },
    AfterTriggered {
        steps: Vec<Self>,
        then_steps: Vec<Self>,
    },
    LocalFrame {
        frame: Frame,
        steps: Vec<Self>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(
    deserialize = "ConditionDef: Deserialize<'de>, Rule: Clone + Deserialize<'de>, Condition: Clone + Deserialize<'de>, Frame: Clone + Deserialize<'de>"
))]
pub struct CompiledGameModel<ConditionDef, Rule, Condition, Frame> {
    pub layer_count: u16,
    objects: Vec<ObjectDef>,
    mark: Vec<MarkDef>,
    condition_defs: Vec<ConditionDef>,
    rules: Vec<Rule>,
    program: ExecutableProgram<Rule, Condition, Frame>,
}

impl<ConditionDef, Rule, Condition, Frame> CompiledGameModel<ConditionDef, Rule, Condition, Frame> {
    pub fn new(layer_count: u16, objects: Vec<ObjectDef>, rules: Vec<Rule>) -> Self
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        let source = rules.iter().cloned().map(ProgramStep::Rule).collect();
        Self {
            layer_count,
            objects,
            mark: Vec::new(),
            condition_defs: Vec::new(),
            rules,
            program: ExecutableProgram::new(source),
        }
    }

    pub fn new_with_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        program: Vec<ProgramStep<Rule, Condition, Frame>>,
    ) -> Self
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        Self::new_with_condition_defs_and_program(layer_count, objects, Vec::new(), program)
    }

    pub fn new_with_condition_defs(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        condition_defs: Vec<ConditionDef>,
    ) -> Self
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        Self::new_with_condition_defs_and_program(layer_count, objects, condition_defs, Vec::new())
    }

    pub fn new_with_condition_defs_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        condition_defs: Vec<ConditionDef>,
        program: Vec<ProgramStep<Rule, Condition, Frame>>,
    ) -> Self
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        Self::new_with_mark_condition_defs_and_program(
            layer_count,
            objects,
            Vec::new(),
            condition_defs,
            program,
        )
    }

    pub fn new_with_mark_condition_defs_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        mark: Vec<MarkDef>,
        condition_defs: Vec<ConditionDef>,
        program: Vec<ProgramStep<Rule, Condition, Frame>>,
    ) -> Self
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        let mut rules = Vec::new();
        collect_program_rules(&program, &mut rules);
        Self {
            layer_count,
            objects,
            mark,
            condition_defs,
            rules,
            program: ExecutableProgram::new(program),
        }
    }

    pub fn clone_with_executable_program(
        &self,
        program: ExecutableProgram<Rule, Condition, Frame>,
    ) -> Self
    where
        ConditionDef: Clone,
        Rule: Clone,
    {
        let mut rules = Vec::new();
        collect_program_rules(program.as_steps(), &mut rules);
        Self {
            layer_count: self.layer_count,
            objects: self.objects.clone(),
            mark: self.mark.clone(),
            condition_defs: self.condition_defs.clone(),
            rules,
            program,
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn objects(&self) -> &[ObjectDef] {
        &self.objects
    }

    pub fn mark(&self) -> &[MarkDef] {
        &self.mark
    }

    pub fn condition_defs(&self) -> &[ConditionDef] {
        &self.condition_defs
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn program(&self) -> &[ProgramStep<Rule, Condition, Frame>] {
        self.program.as_steps()
    }

    pub fn executable_program(&self) -> &ExecutableProgram<Rule, Condition, Frame> {
        &self.program
    }

    pub fn object(&self, object: ObjectId) -> Option<&ObjectDef> {
        if object.is_empty() {
            return None;
        }
        self.objects
            .iter()
            .find(|definition| definition.id == object)
    }

    pub fn object_layer(&self, object: ObjectId) -> Option<LayerId> {
        self.object(object).map(|definition| definition.layer_id)
    }

    pub fn is_main_object(&self, object: ObjectId) -> bool {
        !object.is_empty()
    }

    pub fn main_layers(&self) -> Vec<LayerId> {
        let mut layers = self
            .objects
            .iter()
            .filter_map(|object| self.is_main_object(object.id).then_some(object.layer_id))
            .collect::<Vec<_>>();
        layers.sort();
        layers.dedup();
        layers
    }
}

impl<ConditionKind, Rule, Condition, Frame>
    CompiledGameModel<RuleConditionDef<ConditionId, ConditionKind>, Rule, Condition, Frame>
{
    pub fn checked_new(layer_count: u16, objects: Vec<ObjectDef>) -> Result<Self, CompiledGameError>
    where
        Rule: Clone,
        Condition: Clone,
        Frame: Clone,
    {
        let game = Self::new(layer_count, objects, Vec::new());
        game.validate()?;
        Ok(game)
    }

    pub fn validate(&self) -> Result<(), CompiledGameError> {
        if self.layer_count == 0 {
            return Err(CompiledGameError::InvalidLayerCount);
        }

        let mut object_ids = BTreeSet::new();
        for object in &self.objects {
            if object.id.is_empty() {
                return Err(CompiledGameError::EmptyObjectId);
            }
            if !object_ids.insert(object.id) {
                return Err(CompiledGameError::DuplicateObjectId { object: object.id });
            }
            if object.layer_id.0 >= self.layer_count {
                return Err(CompiledGameError::ObjectLayerOutOfBounds {
                    object: object.id,
                    layer: object.layer_id,
                });
            }
        }

        let mut condition_ids = BTreeSet::new();
        for condition in &self.condition_defs {
            if !condition_ids.insert(condition.id) {
                return Err(CompiledGameError::DuplicateConditionId {
                    condition: condition.id,
                });
            }
        }
        Ok(())
    }

    pub fn condition_def(
        &self,
        condition: ConditionId,
    ) -> Option<&RuleConditionDef<ConditionId, ConditionKind>> {
        self.condition_defs.get(usize::from(condition.0))
    }
}

fn collect_program_rules<Rule: Clone, Condition, Frame>(
    program: &[ProgramStep<Rule, Condition, Frame>],
    rules: &mut Vec<Rule>,
) {
    for step in program {
        match step {
            ProgramStep::Rule(rule) => rules.push(rule.clone()),
            ProgramStep::ConditionalBlock { steps, .. } | ProgramStep::Block { steps, .. } => {
                collect_program_rules(steps, rules)
            }
            ProgramStep::ConditionalBranch {
                then_steps,
                else_steps,
                ..
            } => {
                collect_program_rules(then_steps, rules);
                collect_program_rules(else_steps, rules);
            }
            ProgramStep::AfterTriggered { steps, then_steps } => {
                collect_program_rules(steps, rules);
                collect_program_rules(then_steps, rules);
            }
            ProgramStep::LocalFrame { steps, .. } => collect_program_rules(steps, rules),
        }
    }
}

pub fn flattened_program_rules<Rule: Clone, Condition, Frame>(
    program: &[ProgramStep<Rule, Condition, Frame>],
) -> Vec<Rule> {
    let mut rules = Vec::new();
    collect_program_rules(program, &mut rules);
    rules
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompiledGameError {
    InvalidLayerCount,
    EmptyObjectId,
    DuplicateObjectId { object: ObjectId },
    ObjectLayerOutOfBounds { object: ObjectId, layer: LayerId },
    DuplicateConditionId { condition: ConditionId },
}

/// Preserves one source-statement boundary across lowered alternatives.
///
/// Conditions are evaluated in authored alternative order and only the first
/// matching rule is selected.
pub fn first_matching_program_alternative<Rule, Condition, Frame>(
    alternatives: Vec<(Condition, Rule)>,
) -> Option<ProgramStep<Rule, Condition, Frame>> {
    let mut else_steps = Vec::new();
    for (condition, rule) in alternatives.into_iter().rev() {
        else_steps = vec![ProgramStep::ConditionalBranch {
            condition,
            then_steps: vec![ProgramStep::Rule(rule)],
            else_steps,
        }];
    }
    else_steps.pop()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgramApplyOutcome {
    pub fired: bool,
    pub cancelled: bool,
}

impl ProgramApplyOutcome {
    pub fn merge(&mut self, other: Self) {
        self.fired |= other.fired;
        self.cancelled |= other.cancelled;
    }
}

pub trait ProgramBackend<Rule, Condition, Frame, State> {
    type Error;
    type Snapshot: Clone;

    fn condition_accepts(
        &mut self,
        state: &State,
        condition: &Condition,
        frame: Option<&Frame>,
    ) -> bool;

    fn apply_rule(
        &mut self,
        state: &mut State,
        rule: &Rule,
        frame: Option<&Frame>,
    ) -> Result<ProgramApplyOutcome, Self::Error>;

    fn checkpoint(&self) -> Self::Snapshot;
    fn restore(&mut self, snapshot: &Self::Snapshot);
    fn choose_random(&self, state: &State, candidate_count: usize) -> usize;
    fn state_key(&self, state: &State) -> ProgramStateKey;
    fn invalid_program_continuation(&self) -> Self::Error;
}

pub fn execute_program<Rule, Condition, Frame, State, Backend>(
    backend: &mut Backend,
    state: &mut State,
    program: &ExecutableProgram<Rule, Condition, Frame>,
    frame: Option<&Frame>,
    repeat_limit: usize,
) -> Result<ProgramApplyOutcome, Backend::Error>
where
    State: Clone,
    Backend: ProgramBackend<Rule, Condition, Frame, State>,
{
    let mut never_stop = |_: &State, _: &Backend| false;
    execute_program_segment(
        backend,
        state,
        program,
        frame,
        repeat_limit,
        &mut never_stop,
    )
    .map(|segment| segment.outcome)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    NotEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
}

pub trait RuleInputGuard<InputId> {
    fn input_is(input: InputId) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleGuard<VariableId, ConditionId, ConditionValueKind, InputId> {
    InputIs(InputId),
    VariableEquals {
        variable: VariableId,
        value: i64,
    },
    VariableCompare {
        variable: VariableId,
        op: ComparisonOp,
        value: i64,
    },
    ConditionEquals {
        condition: ConditionId,
        value: i64,
    },
    ConditionNonZero(ConditionId),
    ConditionCompare {
        condition: ConditionId,
        op: ComparisonOp,
        value: i64,
    },
    InlineConditionValue {
        kind: ConditionValueKind,
        value: i64,
    },
    InlineConditionNonZero(ConditionValueKind),
    InlineConditionCompare {
        kind: ConditionValueKind,
        op: ComparisonOp,
        value: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleConditionDef<ConditionId, ConditionValueKind> {
    pub id: ConditionId,
    pub kind: ConditionValueKind,
}

impl<VariableId, ConditionId, ConditionValueKind, InputId> RuleInputGuard<InputId>
    for RuleGuard<VariableId, ConditionId, ConditionValueKind, InputId>
{
    fn input_is(input: InputId) -> Self {
        Self::InputIs(input)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleModel<RuleId, Guard, Pattern, WriteOp, Effect> {
    pub id: RuleId,
    pub guards: Vec<Guard>,
    pub application: RuleApplication,
    pub pattern: Pattern,
    pub writes: Vec<WriteOp>,
    pub effects: Vec<Effect>,
}

impl<RuleId, Guard, Pattern, WriteOp, Effect> RuleModel<RuleId, Guard, Pattern, WriteOp, Effect>
where
    RuleId: Default,
{
    pub fn once(pattern: Pattern, writes: Vec<WriteOp>) -> Self {
        Self::with_application(RuleApplication::Once, pattern, writes)
    }

    pub fn repeated(pattern: Pattern, writes: Vec<WriteOp>) -> Self {
        Self::with_application(RuleApplication::UntilStable, pattern, writes)
    }

    pub fn once_all(pattern: Pattern, writes: Vec<WriteOp>) -> Self {
        Self::with_application(RuleApplication::OnceAll, pattern, writes)
    }

    pub fn once_per_level(pattern: Pattern, writes: Vec<WriteOp>) -> Self {
        Self::with_application(RuleApplication::OncePerLevel, pattern, writes)
    }

    pub fn with_application(
        application: RuleApplication,
        pattern: Pattern,
        writes: Vec<WriteOp>,
    ) -> Self {
        Self {
            id: RuleId::default(),
            guards: Vec::new(),
            application,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }
}

impl<RuleId, Guard, Pattern, WriteOp, Effect> RuleModel<RuleId, Guard, Pattern, WriteOp, Effect> {
    pub fn with_id(mut self, id: RuleId) -> Self {
        self.id = id;
        self
    }

    pub fn when_input<InputId>(mut self, input: InputId) -> Self
    where
        Guard: RuleInputGuard<InputId>,
    {
        self.guards.push(Guard::input_is(input));
        self
    }

    pub fn with_effects(mut self, effects: Vec<Effect>) -> Self {
        self.effects = effects;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableValueError<VariableId> {
    OutOfBounds { variable: VariableId },
    Overflow { variable: VariableId },
    DivisionByZero { variable: VariableId },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleVariables<VariableId> {
    values: Vec<i64>,
    _id: PhantomData<VariableId>,
}

impl<VariableId: KernelId> VisibleVariables<VariableId> {
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
    pub fn get(&self, variable: VariableId) -> Option<i64> {
        self.values.get(variable.index()).copied()
    }

    pub fn set(
        &mut self,
        variable: VariableId,
        value: i64,
    ) -> Result<(), VariableValueError<VariableId>> {
        let slot = self
            .values
            .get_mut(variable.index())
            .ok_or(VariableValueError::OutOfBounds { variable })?;
        *slot = value;
        Ok(())
    }

    pub fn update(
        &mut self,
        variable: VariableId,
        op: VariableUpdateOp,
        value: i64,
    ) -> Result<(), VariableValueError<VariableId>> {
        let slot = self
            .values
            .get_mut(variable.index())
            .ok_or(VariableValueError::OutOfBounds { variable })?;
        *slot = apply_variable_update(*slot, op, value, variable)?;
        Ok(())
    }
}

fn apply_variable_update<VariableId: Copy>(
    current: i64,
    op: VariableUpdateOp,
    value: i64,
    variable: VariableId,
) -> Result<i64, VariableValueError<VariableId>> {
    match op {
        VariableUpdateOp::Set => Ok(value),
        VariableUpdateOp::Add => current
            .checked_add(value)
            .ok_or(VariableValueError::Overflow { variable }),
        VariableUpdateOp::Subtract => current
            .checked_sub(value)
            .ok_or(VariableValueError::Overflow { variable }),
        VariableUpdateOp::Multiply => current
            .checked_mul(value)
            .ok_or(VariableValueError::Overflow { variable }),
        VariableUpdateOp::Divide => {
            if value == 0 {
                return Err(VariableValueError::DivisionByZero { variable });
            }
            current
                .checked_div(value)
                .ok_or(VariableValueError::Overflow { variable })
        }
        VariableUpdateOp::Remainder => {
            if value == 0 {
                return Err(VariableValueError::DivisionByZero { variable });
            }
            current
                .checked_rem(value)
                .ok_or(VariableValueError::Overflow { variable })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkKind {
    Flag,
    Bool,
    Int,
    Enum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MarkValueMatch {
    Any,
    Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleMarkPattern<ObjectId, MarkId> {
    pub object: ObjectId,
    pub mark: MarkId,
    pub value: Option<i64>,
    pub match_value: MarkValueMatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleMatchCell<Offset, ObjectId, LayerId, MarkId> {
    pub offset: Offset,
    pub require_null: bool,
    pub require_objects: Vec<ObjectId>,
    pub require_object_sets: Vec<ObjectSetMatcher<ObjectId, LayerId>>,
    pub forbid_objects: Vec<ObjectId>,
    pub require_mark: Vec<RuleMarkPattern<ObjectId, MarkId>>,
    pub require_object_set_mark: Vec<ObjectSetMarkPattern<MarkId>>,
    pub forbid_mark: Vec<RuleMarkPattern<ObjectId, MarkId>>,
    pub forbid_object_set_mark: Vec<ObjectSetMarkPattern<MarkId>>,
}

impl<Offset, ObjectId, LayerId, MarkId> RuleMatchCell<Offset, ObjectId, LayerId, MarkId> {
    pub fn new(offset: impl Into<Offset>) -> Self {
        Self {
            offset: offset.into(),
            require_null: false,
            require_objects: Vec::new(),
            require_object_sets: Vec::new(),
            forbid_objects: Vec::new(),
            require_mark: Vec::new(),
            require_object_set_mark: Vec::new(),
            forbid_mark: Vec::new(),
            forbid_object_set_mark: Vec::new(),
        }
    }

    pub fn require(mut self, object: ObjectId) -> Self {
        self.require_objects.push(object);
        self
    }

    pub fn forbid(mut self, object: ObjectId) -> Self {
        self.forbid_objects.push(object);
        self
    }

    pub fn require_mark(mut self, object: ObjectId, mark: MarkId, value: Option<i64>) -> Self {
        self.require_mark.push(RuleMarkPattern {
            object,
            mark,
            value,
            match_value: MarkValueMatch::Exact,
        });
        self
    }

    pub fn require_mark_key(mut self, object: ObjectId, mark: MarkId) -> Self {
        self.require_mark.push(RuleMarkPattern {
            object,
            mark,
            value: None,
            match_value: MarkValueMatch::Any,
        });
        self
    }

    pub fn forbid_mark(mut self, object: ObjectId, mark: MarkId, value: Option<i64>) -> Self {
        self.forbid_mark.push(RuleMarkPattern {
            object,
            mark,
            value,
            match_value: MarkValueMatch::Exact,
        });
        self
    }

    pub fn forbid_mark_key(mut self, object: ObjectId, mark: MarkId) -> Self {
        self.forbid_mark.push(RuleMarkPattern {
            object,
            mark,
            value: None,
            match_value: MarkValueMatch::Any,
        });
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellBoundsMatch {
    Continue,
    MatchedNull,
    Rejected,
}

pub const fn match_cell_bounds(require_null: bool, in_bounds: bool) -> CellBoundsMatch {
    match (require_null, in_bounds) {
        (false, true) => CellBoundsMatch::Continue,
        (true, false) => CellBoundsMatch::MatchedNull,
        _ => CellBoundsMatch::Rejected,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePatternComponent<MatchCell> {
    pub cells: Vec<MatchCell>,
    pub gap_count: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePattern<Component> {
    pub components: Vec<Component>,
}

impl<Component> RulePattern<Component> {
    pub fn from_components(components: Vec<Component>) -> Self {
        Self { components }
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

impl<MatchCell> RulePattern<RulePatternComponent<MatchCell>> {
    pub fn new(cells: Vec<MatchCell>) -> Self {
        Self::from_components(vec![RulePatternComponent::new(cells)])
    }

    pub fn cells(&self) -> Vec<&MatchCell> {
        self.components
            .iter()
            .flat_map(|component| &component.cells)
            .collect()
    }
}

impl<MatchCell> RulePatternComponent<MatchCell> {
    pub fn new(cells: Vec<MatchCell>) -> Self {
        Self {
            cells,
            gap_count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuleWriteOp<Offset, ObjectId, MarkId> {
    Add {
        component: u16,
        offset: Offset,
        object: ObjectId,
    },
    AddObjectSet {
        component: u16,
        offset: Offset,
        binding: u16,
    },
    Remove {
        component: u16,
        offset: Offset,
        object: ObjectId,
    },
    RemoveObjectSet {
        component: u16,
        offset: Offset,
        binding: u16,
    },
    Move {
        component: u16,
        from_offset: Offset,
        to_offset: Offset,
        object: ObjectId,
    },
    MoveObjectSet {
        component: u16,
        from_offset: Offset,
        to_offset: Offset,
        binding: u16,
    },
    Replace {
        component: u16,
        offset: Offset,
        remove: ObjectId,
        add: ObjectId,
    },
    SetMark {
        component: u16,
        offset: Offset,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
    },
    SetObjectSetMark {
        component: u16,
        offset: Offset,
        binding: u16,
        mark: MarkId,
        value: Option<i64>,
    },
    RemoveMark {
        component: u16,
        offset: Offset,
        object: ObjectId,
        mark: MarkId,
        value: Option<i64>,
        match_value: MarkValueMatch,
    },
    RemoveObjectSetMark {
        component: u16,
        offset: Offset,
        binding: u16,
        mark: MarkId,
        value: Option<i64>,
        match_value: MarkValueMatch,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSetMatcher<ObjectId, LayerId> {
    pub binding: u16,
    pub layer: LayerId,
    pub objects: Vec<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSetMarkPattern<MarkId> {
    pub binding: u16,
    pub mark: MarkId,
    pub value: Option<i64>,
    pub match_value: MarkValueMatch,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkValue<MarkId> {
    pub mark: MarkId,
    pub value: Option<i64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct MarkEntry<MarkId> {
    mark: MarkValue<MarkId>,
    next: Option<NonZeroU32>,
}

#[derive(Clone)]
pub struct MarkIter<'a, MarkId> {
    entries: &'a [MarkEntry<MarkId>],
    next: Option<NonZeroU32>,
}

impl<MarkId: Copy> Iterator for MarkIter<'_, MarkId> {
    type Item = MarkValue<MarkId>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        let entry = self.entries.get(mark_entry_index(id))?;
        self.next = entry.next;
        Some(entry.mark)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkSpace<MarkId> {
    cell_heads: Vec<Option<NonZeroU32>>,
    slot_heads: Vec<Option<NonZeroU32>>,
    entries: Vec<MarkEntry<MarkId>>,
    free_entries: Vec<NonZeroU32>,
}

impl<MarkId> MarkSpace<MarkId> {
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

impl<MarkId: Copy> MarkSpace<MarkId> {
    pub fn cell_values(&self) -> Vec<Vec<MarkValue<MarkId>>> {
        (0..self.cell_heads.len())
            .map(|index| self.cell_at(index).collect())
            .collect()
    }

    pub fn slot_values(&self) -> Vec<Vec<MarkValue<MarkId>>> {
        (0..self.slot_heads.len())
            .map(|index| self.slot_at(index).collect())
            .collect()
    }

    #[inline]
    pub fn cell_at(&self, index: usize) -> MarkIter<'_, MarkId> {
        MarkIter {
            entries: &self.entries,
            next: self.cell_heads.get(index).copied().flatten(),
        }
    }

    #[inline]
    pub fn slot_at(&self, index: usize) -> MarkIter<'_, MarkId> {
        MarkIter {
            entries: &self.entries,
            next: self.slot_heads.get(index).copied().flatten(),
        }
    }

    pub fn has_cell(&self, index: usize, mark: MarkId, value: Option<i64>) -> bool
    where
        MarkId: PartialEq,
    {
        self.cell_at(index)
            .any(|entry| entry.mark == mark && entry.value == value)
    }

    pub fn has_cell_key(&self, index: usize, mark: MarkId) -> bool
    where
        MarkId: PartialEq,
    {
        self.cell_at(index).any(|entry| entry.mark == mark)
    }

    pub fn has_slot(&self, index: usize, mark: MarkId, value: Option<i64>) -> bool
    where
        MarkId: PartialEq,
    {
        self.slot_at(index)
            .any(|entry| entry.mark == mark && entry.value == value)
    }

    pub fn has_slot_key(&self, index: usize, mark: MarkId) -> bool
    where
        MarkId: PartialEq,
    {
        self.slot_at(index).any(|entry| entry.mark == mark)
    }

    pub fn set_cell(&mut self, index: usize, mark: MarkId, value: Option<i64>)
    where
        MarkId: PartialEq,
    {
        set_mark(
            &mut self.cell_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            mark,
            value,
        );
    }

    pub fn set_slot(&mut self, index: usize, mark: MarkId, value: Option<i64>)
    where
        MarkId: PartialEq,
    {
        set_mark(
            &mut self.slot_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            mark,
            value,
        );
    }

    pub fn remove_cell(&mut self, index: usize, mark: MarkId, value: Option<i64>)
    where
        MarkId: PartialEq,
    {
        let mut entries = self.take_cell(index);
        retain_mark(&mut entries, mark, value);
        self.replace_cell(index, entries);
    }

    pub fn remove_slot(&mut self, index: usize, mark: MarkId, value: Option<i64>)
    where
        MarkId: PartialEq,
    {
        let mut entries = self.take_slot(index);
        retain_mark(&mut entries, mark, value);
        self.replace_slot(index, entries);
    }

    pub fn take_cell(&mut self, index: usize) -> Vec<MarkValue<MarkId>> {
        let mark = self.cell_at(index).collect::<Vec<_>>();
        self.clear_cell(index);
        mark
    }

    pub fn take_slot(&mut self, index: usize) -> Vec<MarkValue<MarkId>> {
        let mark = self.slot_at(index).collect::<Vec<_>>();
        self.clear_slot(index);
        mark
    }

    pub fn replace_cell(&mut self, index: usize, mark: Vec<MarkValue<MarkId>>) {
        self.clear_cell(index);
        for mark in mark {
            self.push_cell(index, mark);
        }
    }

    pub fn replace_slot(&mut self, index: usize, mark: Vec<MarkValue<MarkId>>) {
        self.clear_slot(index);
        for mark in mark {
            self.push_slot(index, mark);
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

    pub fn hash_into<F>(&self, mut hash: u64, mut mark_raw: F) -> u64
    where
        F: FnMut(MarkId) -> u64,
    {
        for index in 0..self.cell_heads.len() {
            hash = hash_mark_iter(hash, self.cell_at(index), &mut mark_raw);
        }
        for index in 0..self.slot_heads.len() {
            hash = hash_mark_iter(hash, self.slot_at(index), &mut mark_raw);
        }
        hash
    }

    fn push_cell(&mut self, index: usize, mark: MarkValue<MarkId>) {
        push_mark(
            &mut self.cell_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            mark,
        );
    }

    fn push_slot(&mut self, index: usize, mark: MarkValue<MarkId>) {
        push_mark(
            &mut self.slot_heads,
            &mut self.entries,
            &mut self.free_entries,
            index,
            mark,
        );
    }
}

impl<MarkId: Copy + PartialEq> PartialEq for MarkSpace<MarkId> {
    fn eq(&self, other: &Self) -> bool {
        self.cell_heads.len() == other.cell_heads.len()
            && self.slot_heads.len() == other.slot_heads.len()
            && (0..self.cell_heads.len()).all(|index| self.cell_at(index).eq(other.cell_at(index)))
            && (0..self.slot_heads.len()).all(|index| self.slot_at(index).eq(other.slot_at(index)))
    }
}

impl<MarkId: Copy + Eq> Eq for MarkSpace<MarkId> {}

fn set_mark<MarkId: Copy + PartialEq>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut Vec<MarkEntry<MarkId>>,
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
    mark: MarkId,
    value: Option<i64>,
) {
    let mut current = heads[index];
    while let Some(id) = current {
        let entry_index = mark_entry_index(id);
        let entry = &mut entries[entry_index];
        if entry.mark.mark == mark {
            entry.mark.value = value;
            return;
        }
        current = entry.next;
    }
    push_mark(
        heads,
        entries,
        free_entries,
        index,
        MarkValue { mark, value },
    );
}

fn retain_mark<MarkId: PartialEq>(
    entries: &mut Vec<MarkValue<MarkId>>,
    mark: MarkId,
    value: Option<i64>,
) {
    entries.retain(|entry| {
        if entry.mark != mark {
            return true;
        }
        value.is_some_and(|value| entry.value != Some(value))
    });
}

fn push_mark<MarkId: Copy>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut Vec<MarkEntry<MarkId>>,
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
    mark: MarkValue<MarkId>,
) {
    let new_id = allocate_mark_entry(entries, free_entries, mark);
    let Some(mut current) = heads[index] else {
        heads[index] = Some(new_id);
        return;
    };

    loop {
        let entry_index = mark_entry_index(current);
        let Some(next) = entries[entry_index].next else {
            entries[entry_index].next = Some(new_id);
            return;
        };
        current = next;
    }
}

fn clear_head<MarkId>(
    heads: &mut [Option<NonZeroU32>],
    entries: &mut [MarkEntry<MarkId>],
    free_entries: &mut Vec<NonZeroU32>,
    index: usize,
) {
    let mut current = heads[index].take();
    while let Some(id) = current {
        let entry_index = mark_entry_index(id);
        current = entries[entry_index].next;
        entries[entry_index].next = None;
        free_entries.push(id);
    }
}

fn allocate_mark_entry<MarkId: Copy>(
    entries: &mut Vec<MarkEntry<MarkId>>,
    free_entries: &mut Vec<NonZeroU32>,
    mark: MarkValue<MarkId>,
) -> NonZeroU32 {
    if let Some(id) = free_entries.pop() {
        let index = mark_entry_index(id);
        entries[index] = MarkEntry { mark, next: None };
        return id;
    }

    let raw_index = entries.len();
    let id = mark_entry_id(raw_index);
    entries.push(MarkEntry { mark, next: None });
    id
}

fn hash_mark_iter<MarkId: Copy, F>(
    mut hash: u64,
    mark: MarkIter<'_, MarkId>,
    mark_raw: &mut F,
) -> u64
where
    F: FnMut(MarkId) -> u64,
{
    let count = mark.clone().count();
    hash = fnv_mix(hash, count as u64);
    for mark in mark {
        hash = fnv_mix(hash, mark_raw(mark.mark));
        hash = fnv_mix(hash, mark.value.unwrap_or(i64::MIN) as u64);
    }
    hash
}

fn mark_entry_index(id: NonZeroU32) -> usize {
    usize::try_from(id.get() - 1).expect("mark entry id must fit usize")
}

fn mark_entry_id(index: usize) -> NonZeroU32 {
    let raw = u32::try_from(index + 1).expect("too many mark entries");
    NonZeroU32::new(raw).expect("mark entry ids are one-based")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionValueKind<ObjectId, Pattern, InputId> {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

    #[derive(Clone, Copy)]
    enum TestProgramRule {
        Add,
        Fail,
    }

    #[derive(Default)]
    struct TestProgramBackend {
        trace: i32,
    }

    impl ProgramBackend<TestProgramRule, bool, (), i32> for TestProgramBackend {
        type Error = &'static str;
        type Snapshot = i32;

        fn condition_accepts(
            &mut self,
            _state: &i32,
            condition: &bool,
            _frame: Option<&()>,
        ) -> bool {
            *condition
        }

        fn apply_rule(
            &mut self,
            state: &mut i32,
            rule: &TestProgramRule,
            _frame: Option<&()>,
        ) -> Result<ProgramApplyOutcome, Self::Error> {
            *state += 1;
            self.trace += 1;
            match rule {
                TestProgramRule::Add => Ok(ProgramApplyOutcome {
                    fired: true,
                    cancelled: false,
                }),
                TestProgramRule::Fail => Err("candidate failed"),
            }
        }

        fn checkpoint(&self) -> Self::Snapshot {
            self.trace
        }

        fn restore(&mut self, snapshot: &Self::Snapshot) {
            self.trace = *snapshot;
        }

        fn choose_random(&self, _state: &i32, _candidate_count: usize) -> usize {
            0
        }

        fn state_key(&self, state: &i32) -> ProgramStateKey {
            ProgramStateKey::from_words(vec![*state as u64])
        }

        fn invalid_program_continuation(&self) -> Self::Error {
            "program continuation does not match its program"
        }
    }

    #[test]
    fn visible_variables_update_with_checked_arithmetic() {
        let mut variables = VisibleVariables::new(vec![4]);
        variables
            .update(TestId(0), VariableUpdateOp::Add, 3)
            .unwrap();
        assert_eq!(variables.get(TestId(0)), Some(7));
        assert!(matches!(
            variables.update(TestId(0), VariableUpdateOp::Divide, 0),
            Err(VariableValueError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn grid_coord_applies_checked_offsets_by_dimension() {
        let coord = GridCoord::<3>::new([2, 3, 4]);

        assert_eq!(
            coord.checked_offset(GridOffset::new([-1, 2, 0])),
            Some(GridCoord::new([1, 5, 4]))
        );
        assert_eq!(coord.checked_offset(GridOffset::new([-3, 0, 0])), None);
        assert_eq!(
            GridCoord::<2>::new([0, u16::MAX]).checked_offset(GridOffset::new([0, 1])),
            None
        );
    }

    #[test]
    fn write_position_uses_component_origin_and_gap_resolver() {
        let placement = MatchPlacement::<2, TestId>::new(vec![
            ComponentPlacement::new(GridCoord::new([1, 1]), Vec::new(), Vec::new()),
            ComponentPlacement::new(GridCoord::new([4, 5]), vec![2], Vec::new()),
        ]);

        let fixed = write_position(
            &placement,
            0,
            &[1, 0],
            |offset, _gaps| Some(GridOffset::new([offset[0], offset[1]])),
            || "out",
        )
        .unwrap();
        let with_gap = write_position(
            &placement,
            1,
            &[1, 0],
            |offset, gaps| Some(GridOffset::new([offset[0] + gaps[0] as i16, offset[1]])),
            || "out",
        )
        .unwrap();

        assert_eq!(fixed, GridCoord::new([2, 1]));
        assert_eq!(with_gap, GridCoord::new([7, 5]));
        assert_eq!(
            write_position(
                &placement,
                2,
                &[0, 0],
                |offset, _gaps| { Some(GridOffset::new([offset[0], offset[1]])) },
                || "out"
            ),
            Err("out")
        );
    }

    #[test]
    fn grid_shape_indexes_cells_and_slots_by_dimension() {
        let shape2 = GridShape::<2>::new([4, 3], 2).unwrap();
        assert_eq!(shape2.cell_count(), Some(12));
        assert_eq!(shape2.slot_count(), Some(24));
        assert_eq!(shape2.cell_index(GridCoord::new([2, 1])), Some(6));
        assert_eq!(shape2.slot_index(GridCoord::new([2, 1]), 1), Some(13));
        assert_eq!(shape2.cell_index(GridCoord::new([4, 1])), None);
        assert_eq!(shape2.slot_index(GridCoord::new([2, 1]), 2), None);

        let shape3 = GridShape::<3>::new([4, 3, 2], 2).unwrap();
        assert_eq!(shape3.cell_count(), Some(24));
        assert_eq!(shape3.cell_index(GridCoord::new([2, 1, 1])), Some(18));
        assert_eq!(shape3.slot_index(GridCoord::new([2, 1, 1]), 1), Some(37));
    }

    #[test]
    fn grid_shape_rejects_empty_or_overflowing_shapes() {
        assert_eq!(GridShape::<2>::new([0, 3], 1), None);
        assert_eq!(GridShape::<2>::new([3, 3], 0), None);
        assert_eq!(GridShape::<8>::new([u16::MAX; 8], u16::MAX), None);
    }

    #[test]
    fn object_bindings_reject_conflicting_rebinds() {
        let mut bindings = Vec::new();

        assert!(bind_object(&mut bindings, 0, TestId(2)));
        assert!(bind_object(&mut bindings, 0, TestId(2)));
        assert!(!bind_object(&mut bindings, 0, TestId(3)));
        assert!(bind_object(&mut bindings, 1, TestId(3)));

        assert_eq!(bound_object(&bindings, 0), Some(TestId(2)));
        assert_eq!(bound_object(&bindings, 1), Some(TestId(3)));
        assert_eq!(bound_object(&bindings, 2), None);
    }

    #[test]
    fn placement_object_binding_searches_across_components() {
        let placement = MatchPlacement::<2, TestId>::new(vec![
            ComponentPlacement::new(
                GridCoord::new([0, 0]),
                Vec::new(),
                vec![ObjectBinding {
                    binding: 0,
                    object: TestId(4),
                }],
            ),
            ComponentPlacement::new(
                GridCoord::new([1, 0]),
                Vec::new(),
                vec![ObjectBinding {
                    binding: 1,
                    object: TestId(5),
                }],
            ),
        ]);

        assert_eq!(placement_object_binding(&placement, 0), Some(TestId(4)));
        assert_eq!(placement_object_binding(&placement, 1), Some(TestId(5)));
        assert_eq!(placement_object_binding(&placement, 2), None);
    }

    #[test]
    fn complete_component_placements_backtracks_to_later_candidates() {
        let components = [0, 1, 2];
        let mut placements = Vec::new();
        let mut candidate_origins = |component: &i32| match *component {
            0 => vec![0],
            1 => vec![10, 11],
            2 => vec![20],
            _ => Vec::new(),
        };
        let mut place_at = |component: &i32, origin| {
            if *component == 1 && origin == 10 {
                None
            } else {
                Some((*component, origin))
            }
        };

        assert!(complete_component_placements(
            &components,
            0,
            &mut placements,
            &mut candidate_origins,
            &mut place_at,
        ));
        assert_eq!(placements, vec![(0, 0), (1, 11), (2, 20)]);
    }

    #[test]
    fn collect_component_placements_emits_every_combination() {
        let components = [0, 1];
        let mut placements = Vec::new();
        let mut matches = Vec::new();
        let mut candidate_origins = |component: &i32| match *component {
            0 => vec![0, 1],
            1 => vec![10, 11],
            _ => Vec::new(),
        };
        let mut place_at = |component: &i32, origin| Some((*component, origin));
        let mut push_match = |matches: &mut Vec<Vec<(i32, i32)>>, placements: &[(i32, i32)]| {
            matches.push(placements.to_vec());
        };

        collect_component_placements(
            &components,
            0,
            &mut placements,
            &mut matches,
            &mut candidate_origins,
            &mut place_at,
            &mut push_match,
        );

        assert_eq!(
            matches,
            vec![
                vec![(0, 0), (1, 10)],
                vec![(0, 0), (1, 11)],
                vec![(0, 1), (1, 10)],
                vec![(0, 1), (1, 11)],
            ]
        );
    }

    #[test]
    fn first_matching_program_alternative_preserves_authored_order() {
        let step: ProgramStep<&str, bool, ()> =
            first_matching_program_alternative(vec![(false, "first"), (true, "second")]).unwrap();

        let ProgramStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } = step
        else {
            panic!("expected an alternative chain");
        };
        assert!(!condition);
        assert_eq!(then_steps, vec![ProgramStep::Rule("first")]);
        assert!(matches!(
            else_steps.as_slice(),
            [ProgramStep::ConditionalBranch {
                condition: true,
                then_steps,
                else_steps,
            }] if then_steps == &[ProgramStep::Rule("second")] && else_steps.is_empty()
        ));
    }

    #[test]
    fn random_program_restores_state_and_trace_when_a_candidate_errors() {
        let program =
            ExecutableProgram::<TestProgramRule, bool, ()>::new(vec![ProgramStep::Block {
                application: RuleApplication::Random,
                stop_condition: None,
                steps: vec![
                    ProgramStep::Rule(TestProgramRule::Add),
                    ProgramStep::Rule(TestProgramRule::Fail),
                ],
            }]);
        let mut backend = TestProgramBackend::default();
        let mut state = 0;

        let result = execute_program(&mut backend, &mut state, &program, None, 10);

        assert_eq!(result, Err("candidate failed"));
        assert_eq!(state, 0);
        assert_eq!(backend.trace, 0);
    }

    #[test]
    fn mark_space_moves_values_without_preserving_free_list_identity() {
        let mut mark = MarkSpace::new(2, 2);
        mark.set_slot(0, TestId(1), Some(9));
        let moved = mark.take_slot(0);
        mark.replace_slot(1, moved);

        let mut expected = MarkSpace::new(2, 2);
        expected.set_slot(1, TestId(1), Some(9));

        assert_eq!(mark, expected);
        assert!(mark.has_slot(1, TestId(1), Some(9)));
    }
}
