use crate::{
    Coord3, Game3, GlobalId3, InputId3, LayerId, ObjectId, Offset3, Patch3, PatchError3, PatchOp3,
    RuleId3, ScratchId3, State3, StateError3,
};
use puzzle_kernel::{
    ComponentPlacement, GlobalUpdateOp, GridOffset, LocalFrame, MatchPlacement, ObjectBinding,
    ScratchValueMatch, bind_object, bound_object as bound_object_in_bindings,
    collect_component_placements as collect_component_placements_shared,
    complete_component_placements as complete_component_placements_shared,
    placement_object_binding, write_position as write_position_shared,
};
use std::collections::HashSet;

pub type ConditionValueKind3 = puzzle_kernel::ConditionValueKind<ObjectId, Pattern3, InputId3>;
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
    pub components: Vec<PatternComponent3>,
    pub cells: Vec<MatchCell3>,
}

impl Pattern3 {
    pub fn new(cells: Vec<MatchCell3>) -> Self {
        Self::from_components(vec![PatternComponent3::new(cells)])
    }

    pub fn from_components(components: Vec<PatternComponent3>) -> Self {
        let cells = components
            .iter()
            .flat_map(|component| component.cells.iter().cloned())
            .collect();
        Self { components, cells }
    }

    pub fn cells(&self) -> &[MatchCell3] {
        &self.cells
    }

    pub fn components(&self) -> &[PatternComponent3] {
        &self.components
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternComponent3 {
    pub cells: Vec<MatchCell3>,
    pub gap_count: u16,
}

impl PatternComponent3 {
    pub fn new(cells: Vec<MatchCell3>) -> Self {
        Self {
            cells,
            gap_count: 0,
        }
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
    UpdateGlobal {
        global: GlobalId3,
        op: GlobalUpdateOp,
        value: i64,
    },
    SetCameraYaw(i16),
    SetCameraPitch(i16),
    SetCameraZoom(u16),
    ResetCamera,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteOp3 {
    Add {
        component: u16,
        offset: Offset3,
        object: ObjectId,
    },
    AddObjectSet {
        component: u16,
        offset: Offset3,
        binding: u16,
    },
    Remove {
        component: u16,
        offset: Offset3,
        object: ObjectId,
    },
    RemoveObjectSet {
        component: u16,
        offset: Offset3,
        binding: u16,
    },
    Replace {
        component: u16,
        offset: Offset3,
        remove: ObjectId,
        add: ObjectId,
    },
    Move {
        component: u16,
        from_offset: Offset3,
        to_offset: Offset3,
        object: ObjectId,
    },
    MoveObjectSet {
        component: u16,
        from_offset: Offset3,
        to_offset: Offset3,
        binding: u16,
    },
    SetScratch {
        component: u16,
        offset: Offset3,
        object: ObjectId,
        scratch: ScratchId3,
        value: Option<i64>,
    },
    SetObjectSetScratch {
        component: u16,
        offset: Offset3,
        binding: u16,
        scratch: ScratchId3,
        value: Option<i64>,
    },
    RemoveScratch {
        component: u16,
        offset: Offset3,
        object: ObjectId,
        scratch: ScratchId3,
        value: Option<i64>,
        match_value: ScratchValueMatch,
    },
    RemoveObjectSetScratch {
        component: u16,
        offset: Offset3,
        binding: u16,
        scratch: ScratchId3,
        value: Option<i64>,
        match_value: ScratchValueMatch,
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
    if !writes_within_local_frame(&placement, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(rule, &placement)?;
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
        if !placement_still_valid(game, &current, &rule.pattern, &placement, &current_scope) {
            continue;
        }
        if !writes_within_local_frame(&placement, &rule.writes, &current_scope)? {
            continue;
        };

        let patch = build_patch(rule, &placement)?;
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
    if !writes_within_local_frame(&placement, &rule.writes, &scope)? {
        return Ok(next);
    }
    let patch = build_patch(rule, &placement)?;
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

pub fn eval_condition_kind(
    game: &Game3,
    state: &State3,
    kind: &ConditionValueKind3,
    input: Option<InputId3>,
) -> i64 {
    match kind {
        ConditionValueKind3::CountObjects(objects) => objects
            .iter()
            .map(|object| count_object(game, state, *object))
            .sum::<u32>() as i64,
        ConditionValueKind3::ExistsObjects(objects) => objects
            .iter()
            .any(|object| count_object(game, state, *object) > 0)
            as i64,
        ConditionValueKind3::NoneObjects(objects) => objects
            .iter()
            .all(|object| count_object(game, state, *object) == 0)
            as i64,
        ConditionValueKind3::CountMatches(patterns) => patterns
            .iter()
            .map(|pattern| count_pattern_matches(game, state, pattern))
            .sum::<u32>() as i64,
        ConditionValueKind3::ExistsMatches(patterns) => patterns
            .iter()
            .any(|pattern| has_pattern_match(game, state, pattern))
            as i64,
        ConditionValueKind3::NoneMatches(patterns) => patterns
            .iter()
            .all(|pattern| !has_pattern_match(game, state, pattern))
            as i64,
        ConditionValueKind3::CountInputMatches(patterns) => input
            .map(|input| {
                patterns
                    .iter()
                    .filter(|(expected, _)| *expected == input)
                    .map(|(_, pattern)| count_pattern_matches(game, state, pattern))
                    .sum::<u32>() as i64
            })
            .unwrap_or(0),
        ConditionValueKind3::ExistsInputMatches(patterns) => input.is_some_and(|input| {
            patterns.iter().any(|(expected, pattern)| {
                *expected == input && has_pattern_match(game, state, pattern)
            })
        }) as i64,
        ConditionValueKind3::NoneInputMatches(patterns) => input.is_some_and(|input| {
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
    placement: &MatchPlacement3,
    writes: &[WriteOp3],
    scope: &LocalFrameScope<'_>,
) -> Result<bool, TransitionError3> {
    if scope.frame.is_none() {
        return Ok(true);
    }
    for write in writes {
        match *write {
            WriteOp3::Add {
                component, offset, ..
            }
            | WriteOp3::AddObjectSet {
                component, offset, ..
            }
            | WriteOp3::Remove {
                component, offset, ..
            }
            | WriteOp3::RemoveObjectSet {
                component, offset, ..
            }
            | WriteOp3::Replace {
                component, offset, ..
            }
            | WriteOp3::SetScratch {
                component, offset, ..
            }
            | WriteOp3::SetObjectSetScratch {
                component, offset, ..
            }
            | WriteOp3::RemoveScratch {
                component, offset, ..
            }
            | WriteOp3::RemoveObjectSetScratch {
                component, offset, ..
            } => {
                let position = write_position(placement, component, offset)?;
                if !scope.contains_coord(position) {
                    return Ok(false);
                }
            }
            WriteOp3::Move {
                component,
                from_offset,
                to_offset,
                ..
            }
            | WriteOp3::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                ..
            } => {
                let from = write_position(placement, component, from_offset)?;
                let to = write_position(placement, component, to_offset)?;
                if !scope.contains_coord(from) || !scope.contains_coord(to) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

type MatchPlacement3 = MatchPlacement<3, ObjectId>;
type ComponentPlacement3 = ComponentPlacement<3, ObjectId>;

fn first_match(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    if pattern.components.is_empty() {
        return Some(MatchPlacement3::empty());
    }
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates.into_iter().find_map(|origin| {
            pattern_placement_from_first_origin(game, state, pattern, origin, scope)
        });
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if let Some(placement) =
                    pattern_placement_from_first_origin(game, state, pattern, origin, scope)
                {
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
    if pattern.components.is_empty() {
        return vec![MatchPlacement3::empty()];
    }
    let mut placements = Vec::new();
    if let Some(candidates) = scope.origin_candidates(state) {
        placements.extend(candidates.into_iter().flat_map(|origin| {
            component_placement_at(game, state, &pattern.components[0], origin, scope)
                .into_iter()
                .flat_map(|first| {
                    let mut components = vec![first];
                    collect_component_placements(game, state, pattern, 1, &mut components, scope)
                })
                .collect::<Vec<_>>()
        }));
        return placements;
    }
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let origin = Coord3 { x, y, z };
                if let Some(first) =
                    component_placement_at(game, state, &pattern.components[0], origin, scope)
                {
                    let mut components = vec![first];
                    placements.extend(collect_component_placements(
                        game,
                        state,
                        pattern,
                        1,
                        &mut components,
                        scope,
                    ));
                }
            }
        }
    }
    placements
}

fn pattern_placement_from_first_origin(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    origin: Coord3,
    scope: &LocalFrameScope<'_>,
) -> Option<MatchPlacement3> {
    let first = component_placement_at(game, state, &pattern.components[0], origin, scope)?;
    let mut components = vec![first];
    complete_component_placements(game, state, pattern, 1, &mut components, scope)
        .then_some(MatchPlacement3::new(components))
}

fn complete_component_placements(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    component_index: usize,
    components: &mut Vec<ComponentPlacement3>,
    scope: &LocalFrameScope<'_>,
) -> bool {
    let mut candidate_origins =
        |_component: &PatternComponent3| component_origin_candidates(state, scope);
    let mut place_at = |component: &PatternComponent3, origin| {
        component_placement_at(game, state, component, origin, scope)
    };
    complete_component_placements_shared(
        &pattern.components,
        component_index,
        components,
        &mut candidate_origins,
        &mut place_at,
    )
}

fn collect_component_placements(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    component_index: usize,
    components: &mut Vec<ComponentPlacement3>,
    scope: &LocalFrameScope<'_>,
) -> Vec<MatchPlacement3> {
    let mut matches = Vec::new();
    let mut candidate_origins =
        |_component: &PatternComponent3| component_origin_candidates(state, scope);
    let mut place_at = |component: &PatternComponent3, origin| {
        component_placement_at(game, state, component, origin, scope)
    };
    let mut push_match = |matches: &mut Vec<MatchPlacement3>,
                          components: &[ComponentPlacement3]| {
        matches.push(MatchPlacement3::new(components.to_vec()));
    };
    collect_component_placements_shared(
        &pattern.components,
        component_index,
        components,
        &mut matches,
        &mut candidate_origins,
        &mut place_at,
        &mut push_match,
    );
    matches
}

fn component_origin_candidates(state: &State3, scope: &LocalFrameScope<'_>) -> Vec<Coord3> {
    if let Some(candidates) = scope.origin_candidates(state) {
        return candidates;
    }
    let mut candidates = Vec::new();
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                candidates.push(Coord3 { x, y, z });
            }
        }
    }
    candidates
}

fn component_placement_at(
    game: &Game3,
    state: &State3,
    component: &PatternComponent3,
    origin: Coord3,
    scope: &LocalFrameScope<'_>,
) -> Option<ComponentPlacement3> {
    let mut object_bindings = Vec::new();
    for cell in &component.cells {
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
            if !bind_object(&mut object_bindings, object_set.binding, found) {
                return None;
            }
        }
        if !cell
            .require_scratch
            .iter()
            .all(|scratch| scratch_pattern_matches(game, state, position, scratch.object, scratch))
            || !cell.require_object_set_scratch.iter().all(|scratch| {
                let Some(object) = bound_object_in_component(&object_bindings, scratch.binding)
                else {
                    return false;
                };
                scratch_pattern_matches_bound(game, state, position, object, scratch)
            })
            || !cell.forbid_scratch.iter().all(|scratch| {
                !scratch_pattern_matches(game, state, position, scratch.object, scratch)
            })
            || !cell.forbid_object_set_scratch.iter().all(|scratch| {
                let Some(object) = bound_object_in_component(&object_bindings, scratch.binding)
                else {
                    return false;
                };
                !scratch_pattern_matches_bound(game, state, position, object, scratch)
            })
        {
            return None;
        }
    }
    Some(ComponentPlacement3::new(
        origin.into(),
        Vec::new(),
        object_bindings,
    ))
}

fn placement_still_valid(
    game: &Game3,
    state: &State3,
    pattern: &Pattern3,
    placement: &MatchPlacement3,
    scope: &LocalFrameScope<'_>,
) -> bool {
    pattern.components.iter().zip(&placement.components).all(
        |(pattern_component, placed_component)| {
            component_placement_at(
                game,
                state,
                pattern_component,
                placed_component.origin.into(),
                scope,
            )
            .is_some_and(|current| current.object_bindings == placed_component.object_bindings)
        },
    )
}

fn bound_object(placement: &MatchPlacement3, binding: u16) -> Option<ObjectId> {
    placement_object_binding(placement, binding)
}

fn bound_object_in_component(
    object_bindings: &[ObjectBinding<ObjectId>],
    binding: u16,
) -> Option<ObjectId> {
    bound_object_in_bindings(object_bindings, binding)
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

fn build_patch(rule: &Rule3, placement: &MatchPlacement3) -> Result<Patch3, TransitionError3> {
    let mut patch = Patch3::new();
    for write in &rule.writes {
        match *write {
            WriteOp3::Add {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp3::Add {
                    position: write_position(placement, component, offset)?,
                    object,
                });
            }
            WriteOp3::AddObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp3::Add {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Remove {
                component,
                offset,
                object,
            } => {
                patch.push(PatchOp3::Remove {
                    position: write_position(placement, component, offset)?,
                    object,
                });
            }
            WriteOp3::RemoveObjectSet {
                component,
                offset,
                binding,
            } => {
                patch.push(PatchOp3::Remove {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::Replace {
                component,
                offset,
                remove,
                add,
            } => {
                patch.push(PatchOp3::Replace {
                    position: write_position(placement, component, offset)?,
                    remove,
                    add,
                });
            }
            WriteOp3::Move {
                component,
                from_offset,
                to_offset,
                object,
            } => {
                patch.push(PatchOp3::Move {
                    from: write_position(placement, component, from_offset)?,
                    to: write_position(placement, component, to_offset)?,
                    object,
                });
            }
            WriteOp3::MoveObjectSet {
                component,
                from_offset,
                to_offset,
                binding,
            } => {
                patch.push(PatchOp3::Move {
                    from: write_position(placement, component, from_offset)?,
                    to: write_position(placement, component, to_offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                });
            }
            WriteOp3::SetScratch {
                component,
                offset,
                object,
                scratch,
                value,
            } => {
                patch.push(PatchOp3::SetScratch {
                    position: write_position(placement, component, offset)?,
                    object,
                    scratch,
                    value,
                });
            }
            WriteOp3::SetObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
                value,
            } => {
                patch.push(PatchOp3::SetScratch {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                    scratch,
                    value,
                });
            }
            WriteOp3::RemoveScratch {
                component,
                offset,
                object,
                scratch,
                value,
                match_value,
            } => {
                patch.push(PatchOp3::RemoveScratch {
                    position: write_position(placement, component, offset)?,
                    object,
                    scratch,
                    value,
                    match_value,
                });
            }
            WriteOp3::RemoveObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
                value,
                match_value,
            } => {
                patch.push(PatchOp3::RemoveScratch {
                    position: write_position(placement, component, offset)?,
                    object: bound_object(placement, binding)
                        .ok_or(TransitionError3::UnboundObjectSet { binding })?,
                    scratch,
                    value,
                    match_value,
                });
            }
        }
    }
    for effect in &rule.effects {
        match effect {
            RuleEffect3::UpdateGlobal { global, op, value } => {
                patch.push(PatchOp3::UpdateGlobal {
                    global: *global,
                    op: *op,
                    value: *value,
                });
            }
            RuleEffect3::SetCameraYaw(_)
            | RuleEffect3::SetCameraPitch(_)
            | RuleEffect3::SetCameraZoom(_)
            | RuleEffect3::ResetCamera => {}
        }
    }
    Ok(patch)
}

fn write_position(
    placement: &MatchPlacement3,
    component: u16,
    offset: Offset3,
) -> Result<Coord3, TransitionError3> {
    write_position_shared(
        placement,
        component,
        &offset,
        |offset, _gaps| Some(GridOffset::from(*offset)),
        || TransitionError3::OffsetOutOfBounds,
    )
    .map(Coord3::from)
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
