use crate::InputId;
pub use puzzle_core::{Coord3, Delta3, Size3};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type Offset3 = puzzle_kernel::SpatialOffset<3>;

pub type GapTerm3 = puzzle_kernel::SpatialGapTerm<3>;

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

pub type ObjectDef3 = puzzle_kernel::ObjectDef;
pub type MarkDef3 = puzzle_kernel::MarkDef;

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

pub type CompiledGame3 = puzzle_core::GridCompiledGame<3>;
pub type CompiledGameError3 = puzzle_kernel::CompiledGameError;
