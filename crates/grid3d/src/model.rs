use crate::{ConditionDef3, ConditionId3, Rule3, RuleStep3};
use crate::{InputId, LayerId, MarkId3, ObjectId};
use puzzle_kernel::{GridCoord, GridOffset};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coord3 {
    /// Horizontal X axis. `left` is -X and `right` is +X.
    pub x: u16,
    /// Horizontal depth axis. `back` is -Y and `front` is +Y.
    pub y: u16,
    /// Height axis. `down` is -Z and `up` is +Z.
    pub z: u16,
}

impl Coord3 {
    pub const fn new(x: u16, y: u16, z: u16) -> Self {
        Self { x, y, z }
    }

    pub fn from_standard_text_position(size: Size3, column: u16, row: u16, slice: u16) -> Self {
        Self {
            x: column,
            y: size.depth - 1 - row,
            z: size.height - 1 - slice,
        }
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
    pub const ZERO: Self = Self {
        dx: 0,
        dy: 0,
        dz: 0,
    };

    pub const fn new(dx: i16, dy: i16, dz: i16) -> Self {
        Self { dx, dy, dz }
    }

    pub const fn scale(self, factor: i16) -> Self {
        Self {
            dx: self.dx * factor,
            dy: self.dy * factor,
            dz: self.dz * factor,
        }
    }

    pub const fn add(self, other: Self) -> Self {
        Self {
            dx: self.dx + other.dx,
            dy: self.dy + other.dy,
            dz: self.dz + other.dz,
        }
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Offset3 {
    Fixed {
        dx: i16,
        dy: i16,
        dz: i16,
    },
    Variable {
        base_dx: i16,
        base_dy: i16,
        base_dz: i16,
        gap_terms: Vec<GapTerm3>,
    },
}

impl From<Delta3> for Offset3 {
    fn from(value: Delta3) -> Self {
        Self::Fixed {
            dx: value.dx,
            dy: value.dy,
            dz: value.dz,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapTerm3 {
    pub gap_index: u16,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Size3 {
    /// Number of cells along X.
    pub width: u16,
    /// Number of cells along the Y depth axis.
    pub depth: u16,
    /// Number of cells along the Z height axis.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Direction3 {
    pub name: &'static str,
    pub offset: Delta3,
}

impl Serialize for Direction3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name)
    }
}

impl<'de> Deserialize<'de> for Direction3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "up" => Ok(Self::UP),
            "down" => Ok(Self::DOWN),
            "left" => Ok(Self::LEFT),
            "right" => Ok(Self::RIGHT),
            "front" => Ok(Self::FORWARD),
            "back" => Ok(Self::BACKWARD),
            other => Err(D::Error::custom(format!(
                "unknown runtime contract direction: {other}"
            ))),
        }
    }
}

impl Direction3 {
    pub const UP: Self = Self {
        name: "up",
        offset: Delta3::new(0, 0, 1),
    };
    pub const DOWN: Self = Self {
        name: "down",
        offset: Delta3::new(0, 0, -1),
    };
    pub const LEFT: Self = Self {
        name: "left",
        offset: Delta3::new(-1, 0, 0),
    };
    pub const RIGHT: Self = Self {
        name: "right",
        offset: Delta3::new(1, 0, 0),
    };
    pub const FORWARD: Self = Self {
        name: "front",
        offset: Delta3::new(0, 1, 0),
    };
    pub const BACKWARD: Self = Self {
        name: "back",
        offset: Delta3::new(0, -1, 0),
    };

    pub const fn directions() -> [Self; 6] {
        [
            Self::UP,
            Self::DOWN,
            Self::LEFT,
            Self::RIGHT,
            Self::FORWARD,
            Self::BACKWARD,
        ]
    }

    pub const fn horizontal() -> [Self; 4] {
        [Self::LEFT, Self::RIGHT, Self::FORWARD, Self::BACKWARD]
    }

    pub const fn vertical() -> [Self; 2] {
        [Self::UP, Self::DOWN]
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self.name, "left" | "right" | "front" | "back")
    }

    pub fn is_vertical(self) -> bool {
        matches!(self.name, "up" | "down")
    }

    pub fn axis(self) -> Axis3 {
        if self.offset.dx != 0 {
            Axis3::X
        } else if self.offset.dy != 0 {
            Axis3::Y
        } else {
            Axis3::Z
        }
    }

    pub fn opposite(self) -> Self {
        match self.name {
            "up" => Self::DOWN,
            "down" => Self::UP,
            "left" => Self::RIGHT,
            "right" => Self::LEFT,
            "front" => Self::BACKWARD,
            "back" => Self::FORWARD,
            _ => unreachable!("built-in directions are exhaustive"),
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "forward" => return Some(Self::FORWARD),
            "backward" => return Some(Self::BACKWARD),
            _ => {}
        }
        Self::directions()
            .into_iter()
            .find(|direction| direction.name == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DirectionSet3 {
    Directions,
    Horizontal,
    Vertical,
}

impl DirectionSet3 {
    pub fn directions(self) -> Vec<Direction3> {
        match self {
            Self::Directions => Direction3::directions().to_vec(),
            Self::Horizontal => Direction3::horizontal().to_vec(),
            Self::Vertical => Direction3::vertical().to_vec(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Frame3 {
    pub primary: Direction3,
    pub secondary: Direction3,
    pub depth: Direction3,
}

impl Frame3 {
    pub const DEFAULT: Self = Self {
        primary: Direction3::RIGHT,
        secondary: Direction3::BACKWARD,
        depth: Direction3::DOWN,
    };

    pub fn canonical(primary: Direction3, secondary: Direction3) -> Result<Self, FrameError3> {
        let depth = canonical_depth(primary, secondary)?;
        Ok(Self {
            primary,
            secondary,
            depth,
        })
    }

    pub fn explicit(
        primary: Direction3,
        secondary: Direction3,
        depth: Direction3,
    ) -> Result<Self, FrameError3> {
        if primary.axis() == secondary.axis() {
            return Err(FrameError3::RepeatedAxis {
                first: primary,
                second: secondary,
            });
        }
        if primary.axis() == depth.axis() {
            return Err(FrameError3::RepeatedAxis {
                first: primary,
                second: depth,
            });
        }
        if secondary.axis() == depth.axis() {
            return Err(FrameError3::RepeatedAxis {
                first: secondary,
                second: depth,
            });
        }
        Ok(Self {
            primary,
            secondary,
            depth,
        })
    }

    pub fn is_canonical_chiral(self) -> bool {
        canonical_depth(self.primary, self.secondary).is_ok_and(|depth| depth == self.depth)
    }

    pub fn to_world_offset(self, local: Delta3) -> Delta3 {
        self.primary
            .offset
            .scale(local.dx)
            .add(self.secondary.offset.scale(local.dy))
            .add(self.depth.offset.scale(local.dz))
    }

    pub fn horizontal(secondary: Direction3) -> Result<Vec<Self>, FrameError3> {
        Direction3::horizontal()
            .into_iter()
            .map(|primary| Self::canonical(primary, secondary))
            .collect()
    }

    pub fn frames() -> Vec<Self> {
        FrameSet3::Frames.frames()
    }

    pub fn canonical_frames() -> Vec<Self> {
        FrameSet3::Canonical.frames()
    }

    pub fn mirrored_frames() -> Vec<Self> {
        FrameSet3::Mirrored.frames()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameSlot3 {
    Direction(Direction3),
    DirectionSet(DirectionSet3),
    CompleteCanonical,
}

impl FrameSlot3 {
    fn directions(self) -> Option<Vec<Direction3>> {
        match self {
            Self::Direction(direction) => Some(vec![direction]),
            Self::DirectionSet(set) => Some(set.directions()),
            Self::CompleteCanonical => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameExpr3 {
    pub primary: FrameSlot3,
    pub secondary: FrameSlot3,
    pub depth: FrameSlot3,
}

impl FrameExpr3 {
    pub const fn new(primary: FrameSlot3, secondary: FrameSlot3, depth: FrameSlot3) -> Self {
        Self {
            primary,
            secondary,
            depth,
        }
    }

    pub fn from_two(primary: FrameSlot3, secondary: FrameSlot3) -> Self {
        Self::new(primary, secondary, FrameSlot3::CompleteCanonical)
    }

    pub fn expand(&self) -> Vec<Frame3> {
        let slots = [self.primary, self.secondary, self.depth];
        let complete_count = slots
            .iter()
            .filter(|slot| matches!(slot, FrameSlot3::CompleteCanonical))
            .count();
        if complete_count > 1 {
            return Vec::new();
        }

        if complete_count == 1 {
            return expand_completed_frame(slots);
        }

        expand_explicit_frames(slots)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameSet3 {
    Frames,
    Canonical,
    Mirrored,
    ExprChirality {
        expr: &'static FrameExpr3,
        chirality: FrameChirality3,
    },
}

impl FrameSet3 {
    pub fn frames(self) -> Vec<Frame3> {
        match self {
            Self::Frames => all_explicit_frames(),
            Self::Canonical => all_explicit_frames()
                .into_iter()
                .filter(|frame| frame.is_canonical_chiral())
                .collect(),
            Self::Mirrored => all_explicit_frames()
                .into_iter()
                .filter(|frame| !frame.is_canonical_chiral())
                .collect(),
            Self::ExprChirality { expr, chirality } => expr
                .expand()
                .into_iter()
                .filter(|frame| chirality.accepts(*frame))
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameChirality3 {
    Canonical,
    Mirrored,
}

impl FrameChirality3 {
    fn accepts(self, frame: Frame3) -> bool {
        match self {
            Self::Canonical => frame.is_canonical_chiral(),
            Self::Mirrored => !frame.is_canonical_chiral(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError3 {
    RepeatedAxis {
        first: Direction3,
        second: Direction3,
    },
}

fn canonical_depth(primary: Direction3, secondary: Direction3) -> Result<Direction3, FrameError3> {
    if primary.axis() == secondary.axis() {
        return Err(FrameError3::RepeatedAxis {
            first: primary,
            second: secondary,
        });
    }

    // Engine chirality is defined so the shorthand right:back resolves to
    // right:back:down in z-up world coordinates.
    let cross = cross(primary.offset, secondary.offset);
    let depth = Direction3::directions()
        .into_iter()
        .find(|direction| direction.offset == cross)
        .expect("cross product of orthogonal built-in directions is a built-in direction");
    Ok(depth)
}

fn cross(a: Delta3, b: Delta3) -> Delta3 {
    Delta3::new(
        (a.dy * b.dz) - (a.dz * b.dy),
        (a.dz * b.dx) - (a.dx * b.dz),
        (a.dx * b.dy) - (a.dy * b.dx),
    )
}

fn expand_completed_frame(slots: [FrameSlot3; 3]) -> Vec<Frame3> {
    let complete_index = slots
        .iter()
        .position(|slot| matches!(slot, FrameSlot3::CompleteCanonical))
        .expect("caller checked that exactly one slot is canonical completion");
    let fixed = slots.map(|slot| slot.directions());
    let first = fixed[(complete_index + 1) % 3].as_ref().unwrap();
    let second = fixed[(complete_index + 2) % 3].as_ref().unwrap();

    let mut frames = Vec::new();
    for first_direction in first {
        for second_direction in second {
            if first_direction.axis() == second_direction.axis() {
                continue;
            }
            let Some(completed) =
                complete_frame_slot(slots, complete_index, *first_direction, *second_direction)
            else {
                continue;
            };
            push_unique_frame(&mut frames, completed);
        }
    }
    frames
}

fn complete_frame_slot(
    slots: [FrameSlot3; 3],
    complete_index: usize,
    first: Direction3,
    second: Direction3,
) -> Option<Frame3> {
    let completed_direction = canonical_depth(first, second).ok()?;
    let mut directions = [Direction3::RIGHT; 3];
    directions[(complete_index + 1) % 3] = first;
    directions[(complete_index + 2) % 3] = second;
    directions[complete_index] = completed_direction;

    for index in 0..3 {
        if let FrameSlot3::Direction(expected) = slots[index] {
            if directions[index] != expected {
                return None;
            }
        }
    }

    Frame3::explicit(directions[0], directions[1], directions[2]).ok()
}

fn expand_explicit_frames(slots: [FrameSlot3; 3]) -> Vec<Frame3> {
    let Some(primary) = slots[0].directions() else {
        return Vec::new();
    };
    let Some(secondary) = slots[1].directions() else {
        return Vec::new();
    };
    let Some(depth) = slots[2].directions() else {
        return Vec::new();
    };

    let mut frames = Vec::new();
    for primary in &primary {
        for secondary in &secondary {
            for depth in &depth {
                let Ok(frame) = Frame3::explicit(*primary, *secondary, *depth) else {
                    continue;
                };
                push_unique_frame(&mut frames, frame);
            }
        }
    }
    frames
}

fn all_explicit_frames() -> Vec<Frame3> {
    FrameExpr3::new(
        FrameSlot3::DirectionSet(DirectionSet3::Directions),
        FrameSlot3::DirectionSet(DirectionSet3::Directions),
        FrameSlot3::DirectionSet(DirectionSet3::Directions),
    )
    .expand()
}

fn push_unique_frame(frames: &mut Vec<Frame3>, frame: Frame3) {
    if !frames.contains(&frame) {
        frames.push(frame);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDef3 {
    pub id: ObjectId,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDef3 {
    pub id: MarkId3,
    pub kind: crate::MarkKind,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputDef3 {
    pub id: InputId,
    pub name: String,
    pub direction: Option<Direction3>,
    pub keys: Vec<String>,
}

impl InputDef3 {
    pub fn directional(id: InputId, name: impl Into<String>, direction: Direction3) -> Self {
        Self {
            id,
            name: name.into(),
            direction: Some(direction),
            keys: Vec::new(),
        }
    }

    pub fn action(id: InputId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            direction: None,
            keys: Vec::new(),
        }
    }

    pub fn with_keys(mut self, keys: Vec<String>) -> Self {
        self.keys = keys;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledGame3 {
    pub layer_count: u16,
    objects: Vec<ObjectDef3>,
    mark: Vec<MarkDef3>,
    condition_defs: Vec<ConditionDef3>,
    rules: Vec<Rule3>,
    program: Vec<RuleStep3>,
}

impl CompiledGame3 {
    pub fn new(layer_count: u16, objects: Vec<ObjectDef3>, rules: Vec<Rule3>) -> Self {
        let program = rules.iter().cloned().map(RuleStep3::Rule).collect();
        Self {
            layer_count,
            objects,
            mark: Vec::new(),
            condition_defs: Vec::new(),
            rules,
            program,
        }
    }

    pub fn new_with_program(
        layer_count: u16,
        objects: Vec<ObjectDef3>,
        program: Vec<RuleStep3>,
    ) -> Self {
        Self::new_with_condition_defs_and_program(layer_count, objects, Vec::new(), program)
    }

    pub fn new_with_condition_defs(
        layer_count: u16,
        objects: Vec<ObjectDef3>,
        condition_defs: Vec<ConditionDef3>,
    ) -> Self {
        Self::new_with_condition_defs_and_program(layer_count, objects, condition_defs, Vec::new())
    }

    pub fn new_with_condition_defs_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef3>,
        condition_defs: Vec<ConditionDef3>,
        program: Vec<RuleStep3>,
    ) -> Self {
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
        objects: Vec<ObjectDef3>,
        mark: Vec<MarkDef3>,
        condition_defs: Vec<ConditionDef3>,
        program: Vec<RuleStep3>,
    ) -> Self {
        let rules = crate::flattened_rules(&program);
        Self {
            layer_count,
            objects,
            mark,
            condition_defs,
            rules,
            program,
        }
    }

    pub fn clone_with_program(&self, program: Vec<RuleStep3>) -> Self {
        Self::new_with_mark_condition_defs_and_program(
            self.layer_count,
            self.objects.clone(),
            self.mark.clone(),
            self.condition_defs.clone(),
            program,
        )
    }

    #[inline]
    pub fn rules(&self) -> &[Rule3] {
        &self.rules
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn objects(&self) -> &[ObjectDef3] {
        &self.objects
    }

    pub fn mark(&self) -> &[MarkDef3] {
        &self.mark
    }

    #[inline]
    pub fn program(&self) -> &[RuleStep3] {
        &self.program
    }

    pub fn checked_new(
        layer_count: u16,
        objects: Vec<ObjectDef3>,
    ) -> Result<Self, CompiledGameError3> {
        let game = Self::new(layer_count, objects, Vec::new());
        game.validate()?;
        Ok(game)
    }

    pub fn validate(&self) -> Result<(), CompiledGameError3> {
        if self.layer_count == 0 {
            return Err(CompiledGameError3::InvalidLayerCount);
        }

        let mut object_ids = BTreeSet::new();
        for object in &self.objects {
            if object.id.is_empty() {
                return Err(CompiledGameError3::EmptyObjectId);
            }
            if !object_ids.insert(object.id) {
                return Err(CompiledGameError3::DuplicateObjectId { object: object.id });
            }
            if object.layer_id.0 >= self.layer_count {
                return Err(CompiledGameError3::ObjectLayerOutOfBounds {
                    object: object.id,
                    layer: object.layer_id,
                });
            }
        }

        let mut condition_ids = BTreeSet::new();
        for condition in &self.condition_defs {
            if !condition_ids.insert(condition.id) {
                return Err(CompiledGameError3::DuplicateConditionId {
                    condition: condition.id,
                });
            }
        }

        Ok(())
    }

    pub fn object_layer(&self, object: ObjectId) -> Option<LayerId> {
        if object.is_empty() {
            return None;
        }
        self.objects
            .iter()
            .find(|def| def.id == object)
            .map(|def| def.layer_id)
    }

    pub fn condition_defs(&self) -> &[ConditionDef3] {
        &self.condition_defs
    }

    pub fn condition_def(&self, condition: ConditionId3) -> Option<&ConditionDef3> {
        self.condition_defs.get(usize::from(condition.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompiledGameError3 {
    InvalidLayerCount,
    EmptyObjectId,
    DuplicateObjectId { object: ObjectId },
    ObjectLayerOutOfBounds { object: ObjectId, layer: LayerId },
    DuplicateConditionId { condition: ConditionId3 },
}
