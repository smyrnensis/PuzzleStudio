use crate::{
    Coord3, Game3, InputId3, ObjectId, Offset3, Patch3, PatchError3, PatchOp3, RuleId3, State3,
    StateError3,
};

const UNTIL_STABLE_REPEAT_LIMIT3: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchCell3 {
    pub offset: Offset3,
    pub require_objects: Vec<ObjectId>,
    pub forbid_objects: Vec<ObjectId>,
}

impl MatchCell3 {
    pub fn new(offset: Offset3) -> Self {
        Self {
            offset,
            require_objects: Vec::new(),
            forbid_objects: Vec::new(),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern3 {
    pub cells: Vec<MatchCell3>,
}

impl Pattern3 {
    pub fn new(cells: Vec<MatchCell3>) -> Self {
        Self { cells }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule3 {
    pub id: RuleId3,
    pub guards: Vec<Guard3>,
    pub application: RuleApplication3,
    pub pattern: Pattern3,
    pub writes: Vec<WriteOp3>,
    pub effects: Vec<RuleEffect3>,
}

impl Rule3 {
    pub fn once(pattern: Pattern3, writes: Vec<WriteOp3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }

    pub fn repeated(pattern: Pattern3, writes: Vec<WriteOp3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::UntilStable,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }

    pub fn once_all(pattern: Pattern3, writes: Vec<WriteOp3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OnceAll,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }

    pub fn once_per_level(pattern: Pattern3, writes: Vec<WriteOp3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OncePerLevel,
            pattern,
            writes,
            effects: Vec::new(),
        }
    }

    pub fn with_id(mut self, id: RuleId3) -> Self {
        self.id = id;
        self
    }

    pub fn when_input(mut self, input: InputId3) -> Self {
        self.guards.push(Guard3::InputIs(input));
        self
    }

    pub fn with_effects(mut self, effects: Vec<RuleEffect3>) -> Self {
        self.effects = effects;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleApplication3 {
    Once,
    OnceAll,
    OncePerLevel,
    UntilStable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guard3 {
    InputIs(InputId3),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleEffect3 {
    SetCameraYaw(i16),
    SetCameraPitch(i16),
    SetCameraZoom(u16),
    ResetCamera,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteOp3 {
    Add {
        offset: Offset3,
        object: ObjectId,
    },
    Remove {
        offset: Offset3,
        object: ObjectId,
    },
    Replace {
        offset: Offset3,
        remove: ObjectId,
        add: ObjectId,
    },
    Move {
        from_offset: Offset3,
        to_offset: Offset3,
        object: ObjectId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError3 {
    Patch(PatchError3),
    OffsetOutOfBounds,
}

impl From<PatchError3> for TransitionError3 {
    fn from(value: PatchError3) -> Self {
        Self::Patch(value)
    }
}

pub fn transition_once(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    transition_rule_once(game, state, rule, None)
}

pub fn transition_once_with_input(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: InputId3,
) -> Result<State3, TransitionError3> {
    transition_rule_once(game, state, rule, Some(input))
}

pub fn transition_program(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
    input: InputId3,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    for rule in rules {
        if !guards_accept(rule, Some(input)) {
            continue;
        }
        current = match rule.application {
            RuleApplication3::Once => transition_rule_once(game, &current, rule, Some(input))?,
            RuleApplication3::OnceAll => {
                transition_rule_once_all(game, &current, rule, Some(input))?
            }
            RuleApplication3::OncePerLevel => {
                transition_rule_once_per_level(game, &current, rule, Some(input))?
            }
            RuleApplication3::UntilStable => {
                transition_rule_repeated(game, &current, rule, Some(input))?
            }
        };
    }
    Ok(current)
}

pub fn transition_program_without_input(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    for rule in rules {
        if !guards_accept(rule, None) {
            continue;
        }
        current = match rule.application {
            RuleApplication3::Once => transition_rule_once(game, &current, rule, None)?,
            RuleApplication3::OnceAll => transition_rule_once_all(game, &current, rule, None)?,
            RuleApplication3::OncePerLevel => {
                transition_rule_once_per_level(game, &current, rule, None)?
            }
            RuleApplication3::UntilStable => transition_rule_repeated(game, &current, rule, None)?,
        };
    }
    Ok(current)
}

fn transition_rule_once(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if !guards_accept(rule, input) {
        return Ok(next);
    }
    let Some(origin) = first_match(game, state, &rule.pattern) else {
        return Ok(next);
    };
    let patch = build_patch(origin, &rule.writes)?;
    patch.apply(game, &mut next)?;
    Ok(next)
}

pub fn transition_once_all(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    transition_rule_once_all(game, state, rule, None)
}

pub fn transition_once_per_level(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    transition_rule_once_per_level(game, state, rule, None)
}

fn transition_rule_once_all(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
) -> Result<State3, TransitionError3> {
    if !guards_accept(rule, input) {
        return Ok(state.clone());
    }

    let origins = all_matches(game, state, &rule.pattern);
    if origins.is_empty() {
        return Ok(state.clone());
    }

    let mut current = state.clone();
    for origin in origins {
        if !pattern_matches_at(game, &current, &rule.pattern, origin) {
            continue;
        }

        let patch = build_patch(origin, &rule.writes)?;
        match patch.apply(game, &mut current) {
            Ok(()) => {}
            Err(error) if once_all_patch_became_stale(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

fn transition_rule_once_per_level(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if next.level_rule_has_fired(rule.id) || !guards_accept(rule, input) {
        return Ok(next);
    }
    let Some(origin) = first_match(game, state, &rule.pattern) else {
        return Ok(next);
    };
    let patch = build_patch(origin, &rule.writes)?;
    patch.apply(game, &mut next)?;
    next.mark_level_rule_fired(rule.id);
    Ok(next)
}

pub fn transition_repeated(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    transition_rule_repeated(game, state, rule, None)
}

pub fn count_pattern_matches(game: &Game3, state: &State3, pattern: &Pattern3) -> u32 {
    all_matches(game, state, pattern).len() as u32
}

pub fn has_pattern_match(game: &Game3, state: &State3, pattern: &Pattern3) -> bool {
    first_match(game, state, pattern).is_some()
}

fn transition_rule_repeated(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    let mut seen = vec![current.clone()];
    let mut repeat_count = 0;
    loop {
        let next = transition_rule_once_all(game, &current, rule, input)?;
        if next == current {
            return Ok(current);
        }
        if seen.iter().any(|seen_state| *seen_state == next) {
            return Ok(next);
        }
        seen.push(next.clone());
        current = next;
        repeat_count += 1;
        if repeat_count >= UNTIL_STABLE_REPEAT_LIMIT3 {
            return Ok(current);
        }
    }
}

fn guards_accept(rule: &Rule3, input: Option<InputId3>) -> bool {
    rule.guards.iter().all(|guard| match *guard {
        Guard3::InputIs(expected) => input.is_some_and(|actual| actual == expected),
    })
}

fn first_match(game: &Game3, state: &State3, pattern: &Pattern3) -> Option<Coord3> {
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if pattern_matches_at(game, state, pattern, origin) {
                    return Some(origin);
                }
            }
        }
    }
    None
}

fn all_matches(game: &Game3, state: &State3, pattern: &Pattern3) -> Vec<Coord3> {
    let mut origins = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if pattern_matches_at(game, state, pattern, origin) {
                    origins.push(origin);
                }
            }
        }
    }
    origins
}

fn pattern_matches_at(game: &Game3, state: &State3, pattern: &Pattern3, origin: Coord3) -> bool {
    pattern.cells.iter().all(|cell| {
        let Some(position) = offset_pos(origin, cell.offset) else {
            return false;
        };
        if state.check_pos(position).is_err() {
            return false;
        }
        cell.require_objects
            .iter()
            .all(|object| state.has_object(game, position, *object))
            && cell
                .forbid_objects
                .iter()
                .all(|object| !state.has_object(game, position, *object))
    })
}

fn build_patch(origin: Coord3, writes: &[WriteOp3]) -> Result<Patch3, TransitionError3> {
    let mut patch = Patch3::new();
    for write in writes {
        match *write {
            WriteOp3::Add { offset, object } => {
                patch.push(PatchOp3::Add {
                    position: offset_pos(origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
            WriteOp3::Remove { offset, object } => {
                patch.push(PatchOp3::Remove {
                    position: offset_pos(origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
            WriteOp3::Replace {
                offset,
                remove,
                add,
            } => {
                patch.push(PatchOp3::Replace {
                    position: offset_pos(origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    remove,
                    add,
                });
            }
            WriteOp3::Move {
                from_offset,
                to_offset,
                object,
            } => {
                patch.push(PatchOp3::Move {
                    from: offset_pos(origin, from_offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    to: offset_pos(origin, to_offset).ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
        }
    }
    Ok(patch)
}

fn once_all_patch_became_stale(error: &PatchError3) -> bool {
    matches!(
        error,
        PatchError3::State(StateError3::LayerOccupied { .. })
            | PatchError3::State(StateError3::ObjectNotPresent { .. })
    )
}

fn offset_pos(origin: Coord3, offset: Offset3) -> Option<Coord3> {
    origin.checked_offset(offset)
}
