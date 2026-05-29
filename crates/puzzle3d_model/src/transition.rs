use crate::{
    Coord3, Game3, InputId3, LayerId, ObjectId, Offset3, Patch3, PatchError3, PatchOp3, RuleId3,
    ScratchId3, State3, StateError3,
};
use puzzle_kernel::{LocalFrame, ScratchValueMatch};
use std::collections::HashSet;

pub type QueryKind3 = puzzle_kernel::QueryKind<ObjectId, Pattern3, InputId3>;
pub type ObjectSetMatcher3 = puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>;
pub type ObjectSetScratchPattern3 = puzzle_kernel::ObjectSetScratchPattern<ScratchId3>;

const UNTIL_STABLE_REPEAT_LIMIT3: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchCell3 {
    pub offset: Offset3,
    pub require_objects: Vec<ObjectId>,
    pub require_object_sets: Vec<ObjectSetMatcher3>,
    pub forbid_objects: Vec<ObjectId>,
    pub require_scratch: Vec<ScratchPattern3>,
    pub require_object_set_scratch: Vec<ObjectSetScratchPattern3>,
    pub forbid_scratch: Vec<ScratchPattern3>,
    pub forbid_object_set_scratch: Vec<ObjectSetScratchPattern3>,
}

impl MatchCell3 {
    pub fn new(offset: Offset3) -> Self {
        Self {
            offset,
            require_objects: Vec::new(),
            require_object_sets: Vec::new(),
            forbid_objects: Vec::new(),
            require_scratch: Vec::new(),
            require_object_set_scratch: Vec::new(),
            forbid_scratch: Vec::new(),
            forbid_object_set_scratch: Vec::new(),
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

    pub fn require_scratch(
        mut self,
        object: ObjectId,
        scratch: ScratchId3,
        value: Option<i64>,
    ) -> Self {
        self.require_scratch.push(ScratchPattern3 {
            object,
            scratch,
            value,
            match_value: ScratchValueMatch::Exact,
        });
        self
    }

    pub fn require_scratch_key(mut self, object: ObjectId, scratch: ScratchId3) -> Self {
        self.require_scratch.push(ScratchPattern3 {
            object,
            scratch,
            value: None,
            match_value: ScratchValueMatch::Any,
        });
        self
    }

    pub fn forbid_scratch(
        mut self,
        object: ObjectId,
        scratch: ScratchId3,
        value: Option<i64>,
    ) -> Self {
        self.forbid_scratch.push(ScratchPattern3 {
            object,
            scratch,
            value,
            match_value: ScratchValueMatch::Exact,
        });
        self
    }

    pub fn forbid_scratch_key(mut self, object: ObjectId, scratch: ScratchId3) -> Self {
        self.forbid_scratch.push(ScratchPattern3 {
            object,
            scratch,
            value: None,
            match_value: ScratchValueMatch::Any,
        });
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchPattern3 {
    pub object: ObjectId,
    pub scratch: ScratchId3,
    pub value: Option<i64>,
    pub match_value: ScratchValueMatch,
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
    AddObjectSet {
        offset: Offset3,
        binding: u16,
    },
    Remove {
        offset: Offset3,
        object: ObjectId,
    },
    RemoveObjectSet {
        offset: Offset3,
        binding: u16,
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
    MoveObjectSet {
        from_offset: Offset3,
        to_offset: Offset3,
        binding: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionError3 {
    Patch(PatchError3),
    OffsetOutOfBounds,
    UnboundObjectSet { binding: u16 },
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
    let mut scoped = state.clone();
    scoped.clear_scratch();
    let mut next = transition_rule_once(game, &scoped, rule, None, None)?;
    next.clear_scratch();
    Ok(next)
}

pub fn transition_once_with_input(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: InputId3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_scratch();
    let mut next = transition_rule_once(game, &scoped, rule, Some(input), None)?;
    next.clear_scratch();
    Ok(next)
}

pub fn transition_program(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
    input: InputId3,
) -> Result<State3, TransitionError3> {
    transition_program_with_local_frame(game, state, rules, input, None)
}

pub fn transition_program_with_local_frame(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
    input: InputId3,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    current.clear_scratch();
    for rule in rules {
        if !guards_accept(rule, Some(input)) {
            continue;
        }
        current = match rule.application {
            RuleApplication3::Once => {
                transition_rule_once(game, &current, rule, Some(input), local_frame)?
            }
            RuleApplication3::OnceAll => {
                transition_rule_once_all(game, &current, rule, Some(input), local_frame)?
            }
            RuleApplication3::OncePerLevel => {
                transition_rule_once_per_level(game, &current, rule, Some(input), local_frame)?
            }
            RuleApplication3::UntilStable => {
                transition_rule_repeated(game, &current, rule, Some(input), local_frame)?
            }
        };
    }
    current.clear_scratch();
    Ok(current)
}

pub fn transition_solver_program(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
    input: InputId3,
) -> Result<State3, TransitionError3> {
    let state = state.without_visual_objects(game);
    let visible_rules = rules
        .iter()
        .filter(|rule| !game.is_visual_rule(rule.id))
        .cloned()
        .collect::<Vec<_>>();
    transition_program(game, &state, &visible_rules, input)
        .map(|state| state.without_visual_objects(game))
}

pub fn transition_program_without_input(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
) -> Result<State3, TransitionError3> {
    transition_program_without_input_with_local_frame(game, state, rules, None)
}

pub fn transition_program_without_input_with_local_frame(
    game: &Game3,
    state: &State3,
    rules: &[Rule3],
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    current.clear_scratch();
    for rule in rules {
        if !guards_accept(rule, None) {
            continue;
        }
        current = match rule.application {
            RuleApplication3::Once => {
                transition_rule_once(game, &current, rule, None, local_frame)?
            }
            RuleApplication3::OnceAll => {
                transition_rule_once_all(game, &current, rule, None, local_frame)?
            }
            RuleApplication3::OncePerLevel => {
                transition_rule_once_per_level(game, &current, rule, None, local_frame)?
            }
            RuleApplication3::UntilStable => {
                transition_rule_repeated(game, &current, rule, None, local_frame)?
            }
        };
    }
    current.clear_scratch();
    Ok(current)
}

fn transition_rule_once(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if !guards_accept(rule, input) {
        return Ok(next);
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let Some(placement) = first_match(game, state, &rule.pattern, &scope) else {
        return Ok(next);
    };
    if !writes_within_local_frame(placement.origin, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(&placement, &rule.writes)?;
    patch.apply(game, &mut next)?;
    Ok(next)
}

pub fn transition_once_all(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_scratch();
    let mut next = transition_rule_once_all(game, &scoped, rule, None, None)?;
    next.clear_scratch();
    Ok(next)
}

pub fn transition_once_per_level(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_scratch();
    let mut next = transition_rule_once_per_level(game, &scoped, rule, None, None)?;
    next.clear_scratch();
    Ok(next)
}

fn transition_rule_once_all(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    if !guards_accept(rule, input) {
        return Ok(state.clone());
    }

    let scope = LocalFrameScope::new(state, local_frame);
    let placements = all_matches(game, state, &rule.pattern, &scope);
    if placements.is_empty() {
        return Ok(state.clone());
    }

    let mut current = state.clone();
    let mut current_scope = LocalFrameScope::new(&current, local_frame);
    for placement in placements {
        let Some(placement) = pattern_placement_at(
            game,
            &current,
            &rule.pattern,
            placement.origin,
            &current_scope,
        ) else {
            continue;
        };
        if !writes_within_local_frame(placement.origin, &rule.writes, &current_scope)? {
            continue;
        };

        let patch = build_patch(&placement, &rule.writes)?;
        match patch.apply(game, &mut current) {
            Ok(()) => {
                current_scope = LocalFrameScope::new(&current, local_frame);
            }
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
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    let mut next = state.clone();
    if next.level_rule_has_fired(rule.id) || !guards_accept(rule, input) {
        return Ok(next);
    }
    let scope = LocalFrameScope::new(state, local_frame);
    let Some(placement) = first_match(game, state, &rule.pattern, &scope) else {
        return Ok(next);
    };
    if !writes_within_local_frame(placement.origin, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(&placement, &rule.writes)?;
    patch.apply(game, &mut next)?;
    next.mark_level_rule_fired(rule.id);
    Ok(next)
}

pub fn transition_repeated(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
) -> Result<State3, TransitionError3> {
    let mut scoped = state.clone();
    scoped.clear_scratch();
    let mut next = transition_rule_repeated(game, &scoped, rule, None, None)?;
    next.clear_scratch();
    Ok(next)
}

pub fn count_pattern_matches(game: &Game3, state: &State3, pattern: &Pattern3) -> u32 {
    let scope = LocalFrameScope::new(state, None);
    all_matches(game, state, pattern, &scope).len() as u32
}

pub fn has_pattern_match(game: &Game3, state: &State3, pattern: &Pattern3) -> bool {
    let scope = LocalFrameScope::new(state, None);
    first_match(game, state, pattern, &scope).is_some()
}

pub fn eval_query_kind(
    game: &Game3,
    state: &State3,
    kind: &QueryKind3,
    input: Option<InputId3>,
) -> i64 {
    match kind {
        QueryKind3::CountObjects(objects) => objects
            .iter()
            .map(|object| count_object(game, state, *object))
            .sum::<u32>() as i64,
        QueryKind3::ExistsObjects(objects) => objects
            .iter()
            .any(|object| count_object(game, state, *object) > 0)
            as i64,
        QueryKind3::NoneObjects(objects) => objects
            .iter()
            .all(|object| count_object(game, state, *object) == 0)
            as i64,
        QueryKind3::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| count_pattern_matches(game, state, pattern))
            .sum::<u32>() as i64,
        QueryKind3::ExistsMatches(patterns) => patterns
            .iter()
            .any(|pattern| has_pattern_match(game, state, pattern))
            as i64,
        QueryKind3::NoneMatches(patterns) => patterns
            .iter()
            .all(|pattern| !has_pattern_match(game, state, pattern))
            as i64,
        QueryKind3::CountInputMatches(patterns) => input
            .map(|input| {
                patterns
                    .iter()
                    .filter(|(expected, _)| *expected == input)
                    .map(|(_, pattern)| count_pattern_matches(game, state, pattern))
                    .sum::<u32>() as i64
            })
            .unwrap_or(0),
        QueryKind3::ExistsInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && has_pattern_match(game, state, pattern)
            })
        }) as i64,
        QueryKind3::NoneInputMatches(patterns) => input.is_some_and(|input| {
            patterns
                .iter()
                .filter(|(expected, _)| *expected == input)
                .all(|(_, pattern)| !has_pattern_match(game, state, pattern))
        }) as i64,
    }
}

fn transition_rule_repeated(
    game: &Game3,
    state: &State3,
    rule: &Rule3,
    input: Option<InputId3>,
    local_frame: Option<&LocalFrame<ObjectId>>,
) -> Result<State3, TransitionError3> {
    let mut current = state.clone();
    let mut seen = vec![current.clone()];
    let mut repeat_count = 0;
    loop {
        let next = transition_rule_once_all(game, &current, rule, input, local_frame)?;
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

struct LocalFrameScope<'a> {
    frame: Option<&'a LocalFrame<ObjectId>>,
    focus_coords: Vec<Coord3>,
}

impl<'a> LocalFrameScope<'a> {
    fn new(state: &State3, frame: Option<&'a LocalFrame<ObjectId>>) -> Self {
        let focus_coords = frame
            .map(|frame| focus_coords(state, frame))
            .unwrap_or_default();
        Self {
            frame,
            focus_coords,
        }
    }

    fn contains_coord(&self, coord: Coord3) -> bool {
        let Some(frame) = self.frame else {
            return true;
        };
        self.focus_coords.iter().any(|focus| {
            let dx = i32::from(coord.x) - i32::from(focus.x);
            let dy = i32::from(coord.y) - i32::from(focus.y);
            let dz = i32::from(coord.z) - i32::from(focus.z);
            frame.contains_delta_3d(dx, dy, dz)
        })
    }

    fn origin_candidates(&self, state: &State3) -> Option<Vec<Coord3>> {
        let Some(frame) = self.frame else {
            return None;
        };
        let mut seen = HashSet::new();
        let mut coords = Vec::new();
        for focus in &self.focus_coords {
            let (x_range, y_range, z_range) = frame.ranges_3d(
                focus.x,
                focus.y,
                focus.z,
                state.size.width,
                state.size.depth,
                state.size.height,
            );
            for z in z_range {
                for y in y_range.clone() {
                    for x in x_range.clone() {
                        let coord = Coord3 { x, y, z };
                        if seen.insert(coord) {
                            coords.push(coord);
                        }
                    }
                }
            }
        }
        Some(coords)
    }
}

fn focus_coords(state: &State3, local_frame: &LocalFrame<ObjectId>) -> Vec<Coord3> {
    let mut coords = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let coord = Coord3 { x, y, z };
                if local_frame.focus_objects.iter().any(|object| {
                    state
                        .cell_view(coord)
                        .is_ok_and(|cell| cell.objects.contains(object))
                }) {
                    coords.push(coord);
                }
            }
        }
    }
    coords
}

fn writes_within_local_frame(
    origin: Coord3,
    writes: &[WriteOp3],
    scope: &LocalFrameScope<'_>,
) -> Result<bool, TransitionError3> {
    if scope.frame.is_none() {
        return Ok(true);
    }
    for write in writes {
        match *write {
            WriteOp3::Add { offset, .. }
            | WriteOp3::AddObjectSet { offset, .. }
            | WriteOp3::Remove { offset, .. }
            | WriteOp3::RemoveObjectSet { offset, .. }
            | WriteOp3::Replace { offset, .. } => {
                let Some(position) = offset_pos(origin, offset) else {
                    return Err(TransitionError3::OffsetOutOfBounds);
                };
                if !scope.contains_coord(position) {
                    return Ok(false);
                }
            }
            WriteOp3::Move {
                from_offset,
                to_offset,
                ..
            }
            | WriteOp3::MoveObjectSet {
                from_offset,
                to_offset,
                ..
            } => {
                let Some(from) = offset_pos(origin, from_offset) else {
                    return Err(TransitionError3::OffsetOutOfBounds);
                };
                let Some(to) = offset_pos(origin, to_offset) else {
                    return Err(TransitionError3::OffsetOutOfBounds);
                };
                if !scope.contains_coord(from) || !scope.contains_coord(to) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchPlacement3 {
    origin: Coord3,
    object_bindings: Vec<(u16, ObjectId)>,
}

fn first_match(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates
            .into_iter()
            .find_map(|origin| pattern_placement_at(game, state, pattern, origin, scope));
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if let Some(placement) = pattern_placement_at(game, state, pattern, origin, scope) {
                    return Some(placement);
                }
            }
        }
    }
    None
}

fn all_matches(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    scope: &LocalFrameScope<'_>,
) -> Vec<MatchPlacement3> {
    let mut placements = Vec::new();
    if let Some(candidates) = scope.origin_candidates(state) {
        placements.extend(
            candidates
                .into_iter()
                .filter_map(|origin| pattern_placement_at(game, state, pattern, origin, scope)),
        );
        return placements;
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if let Some(placement) = pattern_placement_at(game, state, pattern, origin, scope) {
                    placements.push(placement);
                }
            }
        }
    }
    placements
}

fn pattern_placement_at(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    origin: Coord3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    let mut object_bindings = Vec::new();
    for cell in &pattern.cells {
        let Some(position) = offset_pos(origin, cell.offset) else {
            return None;
        };
        if state.check_pos(position).is_err() {
            return None;
        }
        if !scope.contains_coord(position) {
            return None;
        }
        if !cell
            .require_objects
            .iter()
            .all(|object| cell_requires_object(game, state, position, *object))
            || !cell
                .forbid_objects
                .iter()
                .all(|object| cell_forbids_object(game, state, position, *object))
        {
            return None;
        }
        for object_set in &cell.require_object_sets {
            let Ok(found) = state.get_layer(position, object_set.layer) else {
                return None;
            };
            if found.is_empty() || !object_set.objects.contains(&found) {
                return None;
            }
            if let Some((_, existing)) = object_bindings
                .iter()
                .find(|(binding, _)| *binding == object_set.binding)
            {
                if *existing != found {
                    return None;
                }
            } else {
                object_bindings.push((object_set.binding, found));
            }
        }
        if !cell
            .require_scratch
            .iter()
            .all(|scratch| scratch_pattern_matches(game, state, position, scratch.object, scratch))
            || !cell.require_object_set_scratch.iter().all(|scratch| {
                let Some(object) = bound_object(&object_bindings, scratch.binding) else {
                    return false;
                };
                scratch_pattern_matches_bound(game, state, position, object, scratch)
            })
            || !cell.forbid_scratch.iter().all(|scratch| {
                !scratch_pattern_matches(game, state, position, scratch.object, scratch)
            })
            || !cell.forbid_object_set_scratch.iter().all(|scratch| {
                let Some(object) = bound_object(&object_bindings, scratch.binding) else {
                    return false;
                };
                !scratch_pattern_matches_bound(game, state, position, object, scratch)
            })
        {
            return None;
        }
    }
    Some(MatchPlacement3 {
        origin,
        object_bindings,
    })
}

fn bound_object(object_bindings: &[(u16, ObjectId)], binding: u16) -> Option<ObjectId> {
    object_bindings
        .iter()
        .find(|(candidate, _)| *candidate == binding)
        .map(|(_, object)| *object)
}

fn scratch_pattern_matches(
    game: &Game3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
    scratch: &ScratchPattern3,
) -> bool {
    match scratch.match_value {
        ScratchValueMatch::Any => state.has_scratch_key(game, position, object, scratch.scratch),
        ScratchValueMatch::Exact => {
            state.has_scratch(game, position, object, scratch.scratch, scratch.value)
        }
    }
}

fn scratch_pattern_matches_bound(
    game: &Game3,
    state: &State3,
    position: Coord3,
    object: ObjectId,
    scratch: &ObjectSetScratchPattern3,
) -> bool {
    match scratch.match_value {
        ScratchValueMatch::Any => state.has_scratch_key(game, position, object, scratch.scratch),
        ScratchValueMatch::Exact => {
            state.has_scratch(game, position, object, scratch.scratch, scratch.value)
        }
    }
}

fn cell_requires_object(game: &Game3, state: &State3, position: Coord3, object: ObjectId) -> bool {
    match state.cell_has_object_masked(position, object) {
        Some(found) => found,
        None => state.has_object(game, position, object),
    }
}

fn cell_forbids_object(game: &Game3, state: &State3, position: Coord3, object: ObjectId) -> bool {
    match state.cell_has_object_masked(position, object) {
        Some(found) => !found,
        None => !state.has_object(game, position, object),
    }
}

fn count_object(game: &Game3, state: &State3, object: ObjectId) -> u32 {
    if let Some(count) = state.object_count_masked(object) {
        return count;
    }

    let mut count = 0;
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                if state.has_object(game, Coord3 { x, y, z }, object) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn build_patch(
    placement: &MatchPlacement3,
    writes: &[WriteOp3],
) -> Result<Patch3, TransitionError3> {
    let mut patch = Patch3::new();
    for write in writes {
        match *write {
            WriteOp3::Add { offset, object } => {
                patch.push(PatchOp3::Add {
                    position: offset_pos(placement.origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
            WriteOp3::AddObjectSet { offset, binding } => {
                patch.push(PatchOp3::Add {
                    position: offset_pos(placement.origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object: bound_object(&placement.object_bindings, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Remove { offset, object } => {
                patch.push(PatchOp3::Remove {
                    position: offset_pos(placement.origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
            WriteOp3::RemoveObjectSet { offset, binding } => {
                patch.push(PatchOp3::Remove {
                    position: offset_pos(placement.origin, offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object: bound_object(&placement.object_bindings, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Replace {
                offset,
                remove,
                add,
            } => {
                patch.push(PatchOp3::Replace {
                    position: offset_pos(placement.origin, offset)
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
                    from: offset_pos(placement.origin, from_offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    to: offset_pos(placement.origin, to_offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object,
                });
            }
            WriteOp3::MoveObjectSet {
                from_offset,
                to_offset,
                binding,
            } => {
                patch.push(PatchOp3::Move {
                    from: offset_pos(placement.origin, from_offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    to: offset_pos(placement.origin, to_offset)
                        .ok_or(TransitionError3::OffsetOutOfBounds)?,
                    object: bound_object(&placement.object_bindings, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
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
