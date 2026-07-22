use crate::compiled_game::{MarkValueMatch, VariableUpdateOp};
use crate::ids::{LayerId, MarkId, ObjectId, VariableId};
use crate::state::{GridExecutionState, GridSize, GridState, GridStateError};
use puzzle_kernel::{CompiledGameModel, GridCoord, GridPatchOp};

pub type PatchOp<const D: usize = 2> = GridPatchOp<GridCoord<D>, ObjectId, VariableId, MarkId>;
pub type Patch = GridPatch<2>;
pub type PatchError = GridPatchError<2>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridPatch<const D: usize> {
    ops: Vec<PatchOp<D>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridPatchError<const D: usize> {
    State(GridStateError<D>),
    UnknownObject {
        object: ObjectId,
    },
    ExpectedObject {
        position: GridCoord<D>,
        layer: LayerId,
        expected: ObjectId,
        found: ObjectId,
    },
    LayerOccupied {
        position: GridCoord<D>,
        layer: LayerId,
        existing: ObjectId,
        attempted: ObjectId,
    },
}

impl<const D: usize> From<GridStateError<D>> for GridPatchError<D> {
    fn from(value: GridStateError<D>) -> Self {
        Self::State(value)
    }
}

impl<const D: usize> GridPatch<D> {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn from_ops(ops: Vec<PatchOp<D>>) -> Self {
        Self { ops }
    }

    pub fn push(&mut self, op: PatchOp<D>) {
        self.ops.push(op);
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn ops(&self) -> &[PatchOp<D>] {
        &self.ops
    }

    pub fn apply<Size, ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        state: &GridState<D, Size>,
    ) -> Result<GridState<D, Size>, GridPatchError<D>>
    where
        Size: GridSize<D>,
    {
        let mut execution = GridExecutionState::new(state.clone());
        self.apply_execution_in_place(game, &mut execution)?;
        Ok(execution.into_committed())
    }

    pub fn apply_in_place<Size, ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        state: &mut GridState<D, Size>,
    ) -> Result<bool, GridPatchError<D>>
    where
        Size: GridSize<D>,
    {
        let mut execution = GridExecutionState::new(state.clone());
        let changed = self.apply_execution_in_place(game, &mut execution)?;
        *state = execution.into_committed();
        Ok(changed)
    }

    pub(crate) fn apply_execution_in_place<Size, ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        state: &mut GridExecutionState<D, Size>,
    ) -> Result<bool, GridPatchError<D>>
    where
        Size: GridSize<D>,
    {
        let changed = self.validate_execution(game, state)?;
        apply_moves(game, state, &self.ops)?;

        for op in &self.ops {
            match *op {
                PatchOp::Add { position, object } => apply_add(game, state, position, object)?,
                PatchOp::Remove { position, object } => {
                    apply_remove(game, state, position, object)?
                }
                PatchOp::Move { .. } => {}
                PatchOp::Replace {
                    position,
                    remove,
                    add,
                } => {
                    apply_remove(game, state, position, remove)?;
                    apply_add(game, state, position, add)?;
                }
                PatchOp::UpdateVariable {
                    variable,
                    op,
                    value,
                } => state.update_visible_variable(variable, op, value)?,
                PatchOp::SetMark {
                    position,
                    object,
                    mark,
                    value,
                } => apply_set_mark(game, state, position, object, mark, value)?,
                PatchOp::RemoveMark {
                    position,
                    object,
                    mark,
                    value,
                    match_value,
                } => apply_remove_mark(game, state, position, object, mark, value, match_value)?,
            }
        }

        state.recompute_hash();
        Ok(changed)
    }

    pub fn validate<Size, ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        state: &GridState<D, Size>,
    ) -> Result<bool, GridPatchError<D>>
    where
        Size: GridSize<D>,
    {
        let execution = GridExecutionState::new(state.clone());
        self.validate_execution(game, &execution)
    }

    pub(crate) fn validate_execution<Size, ConditionDef, Rule, Condition, Frame>(
        &self,
        game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
        state: &GridExecutionState<D, Size>,
    ) -> Result<bool, GridPatchError<D>>
    where
        Size: GridSize<D>,
    {
        let mut slots = SlotOverlay::new();
        validate_moves(game, state, &self.ops, &mut slots)?;
        let mut changed = slots.changed(state)?;

        for op in &self.ops {
            match *op {
                PatchOp::Add { position, object } => {
                    let layer = object_layer(game, object)?;
                    let existing = slots.get(state, position, layer)?;
                    if existing == object {
                        continue;
                    }
                    slots.set(position, layer, object);
                    changed = true;
                }
                PatchOp::Remove { position, object } => {
                    let layer = object_layer(game, object)?;
                    let found = slots.get(state, position, layer)?;
                    if found != object {
                        return Err(GridPatchError::ExpectedObject {
                            position,
                            layer,
                            expected: object,
                            found,
                        });
                    }
                    slots.set(position, layer, ObjectId::EMPTY);
                    changed = true;
                }
                PatchOp::Move { .. } => {}
                PatchOp::Replace {
                    position,
                    remove,
                    add,
                } => {
                    let remove_layer = object_layer(game, remove)?;
                    let found = slots.get(state, position, remove_layer)?;
                    if found != remove {
                        return Err(GridPatchError::ExpectedObject {
                            position,
                            layer: remove_layer,
                            expected: remove,
                            found,
                        });
                    }
                    slots.set(position, remove_layer, ObjectId::EMPTY);

                    let add_layer = object_layer(game, add)?;
                    let existing = slots.get(state, position, add_layer)?;
                    if existing == add {
                        continue;
                    }
                    slots.set(position, add_layer, add);
                    changed = true;
                }
                PatchOp::UpdateVariable {
                    variable,
                    op,
                    value,
                } => {
                    let next = validate_variable_update(state, variable, op, value)?;
                    changed |= state.variable_value(variable) != Some(next);
                }
                PatchOp::SetMark {
                    position,
                    object,
                    mark,
                    value,
                } => {
                    if object.is_empty() {
                        changed |= !state.has_cell_mark_at(position, mark, value);
                    } else {
                        let layer =
                            expect_object_in_overlay(game, state, &slots, position, object)?;
                        if state
                            .get_layer_at(position, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= !state.has_mark_at(game, position, object, mark, value);
                        } else {
                            changed = true;
                        }
                    }
                }
                PatchOp::RemoveMark {
                    position,
                    object,
                    mark,
                    value,
                    match_value,
                } => {
                    let matched_value = match match_value {
                        MarkValueMatch::Any => None,
                        MarkValueMatch::Exact => value,
                    };
                    if object.is_empty() {
                        changed |= match match_value {
                            MarkValueMatch::Any => state.has_cell_mark_key_at(position, mark),
                            MarkValueMatch::Exact => {
                                state.has_cell_mark_at(position, mark, matched_value)
                            }
                        };
                    } else {
                        let layer = object_layer(game, object)?;
                        let found = slots.get(state, position, layer)?;
                        if found != object {
                            if state
                                .get_layer_at(position, layer)
                                .is_ok_and(|original| original == object)
                            {
                                changed = true;
                                continue;
                            }
                            return Err(GridPatchError::ExpectedObject {
                                position,
                                layer,
                                expected: object,
                                found,
                            });
                        }
                        if state
                            .get_layer_at(position, layer)
                            .is_ok_and(|found| found == object)
                        {
                            changed |= match match_value {
                                MarkValueMatch::Any => {
                                    state.has_mark_key_at(game, position, object, mark)
                                }
                                MarkValueMatch::Exact => {
                                    state.has_mark_at(game, position, object, mark, matched_value)
                                }
                            };
                        } else {
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(changed || slots.changed(state)?)
    }
}

#[derive(Default)]
struct SlotOverlay<const D: usize> {
    slots: Vec<(GridCoord<D>, LayerId, ObjectId)>,
}

impl<const D: usize> SlotOverlay<D> {
    fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn get<Size: GridSize<D>>(
        &self,
        state: &GridExecutionState<D, Size>,
        position: GridCoord<D>,
        layer: LayerId,
    ) -> Result<ObjectId, GridPatchError<D>> {
        self.slots
            .iter()
            .rev()
            .find_map(|(slot_position, slot_layer, object)| {
                (*slot_position == position && *slot_layer == layer).then_some(*object)
            })
            .map(Ok)
            .unwrap_or_else(|| {
                state
                    .get_layer_at(position, layer)
                    .map_err(GridPatchError::from)
            })
    }

    fn set(&mut self, position: GridCoord<D>, layer: LayerId, object: ObjectId) {
        if let Some((_, _, existing)) =
            self.slots
                .iter_mut()
                .rev()
                .find(|(slot_position, slot_layer, _)| {
                    *slot_position == position && *slot_layer == layer
                })
        {
            *existing = object;
            return;
        }
        self.slots.push((position, layer, object));
    }

    fn changed<Size: GridSize<D>>(
        &self,
        state: &GridExecutionState<D, Size>,
    ) -> Result<bool, GridPatchError<D>> {
        self.slots
            .iter()
            .try_fold(false, |changed, (position, layer, object)| {
                Ok(changed || state.get_layer_at(*position, *layer)? != *object)
            })
    }
}

fn object_layer<ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    object: ObjectId,
) -> Result<LayerId, GridPatchError<D>> {
    game.object_layer(object)
        .ok_or(GridPatchError::UnknownObject { object })
}

fn apply_add<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &mut GridExecutionState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let layer = object_layer(game, object)?;
    let existing = state.get_layer_at(position, layer)?;
    if existing != object {
        state.set_slot_unchecked(position, layer, object);
    }
    Ok(())
}

fn apply_remove<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &mut GridExecutionState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let layer = object_layer(game, object)?;
    let found = state.get_layer_at(position, layer)?;
    if found != object {
        return Err(GridPatchError::ExpectedObject {
            position,
            layer,
            expected: object,
            found,
        });
    }
    state.set_slot_unchecked(position, layer, ObjectId::EMPTY);
    Ok(())
}

fn validate_moves<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &GridExecutionState<D, Size>,
    ops: &[PatchOp<D>],
    slots: &mut SlotOverlay<D>,
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let moves = validated_moves(game, state, ops)?;

    for (from, _, layer, _) in &moves {
        slots.set(*from, *layer, ObjectId::EMPTY);
    }
    for (_, to, layer, object) in moves {
        slots.set(to, layer, object);
    }
    Ok(())
}

fn apply_moves<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &mut GridExecutionState<D, Size>,
    ops: &[PatchOp<D>],
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let moves = validated_moves(game, state, ops)?;

    let mut moved = Vec::with_capacity(moves.len());
    for (from, to, layer, object) in moves {
        let mark = state.take_slot_for_move_unchecked(from, layer);
        moved.push((to, layer, object, mark));
    }
    for (to, layer, object, mark) in moved {
        state.place_moved_slot_unchecked(to, layer, object, mark);
    }
    Ok(())
}

fn validated_moves<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &GridExecutionState<D, Size>,
    ops: &[PatchOp<D>],
) -> Result<Vec<(GridCoord<D>, GridCoord<D>, LayerId, ObjectId)>, GridPatchError<D>>
where
    Size: GridSize<D>,
{
    ops.iter()
        .filter_map(|op| {
            let PatchOp::Move { from, to, object } = *op else {
                return None;
            };
            Some((from, to, object))
        })
        .map(|(from, to, object)| {
            let layer = object_layer(game, object)?;
            let found = state.get_layer_at(from, layer)?;
            if found != object {
                return Err(GridPatchError::ExpectedObject {
                    position: from,
                    layer,
                    expected: object,
                    found,
                });
            }
            Ok((from, to, layer, object))
        })
        .collect()
}

fn apply_set_mark<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &mut GridExecutionState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    if object.is_empty() {
        state.set_cell_mark_unchecked(position, mark, value);
        return Ok(());
    }
    let layer = expect_object_at(game, state, position, object)?;
    state.set_mark_unchecked(position, layer, mark, value);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_remove_mark<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &mut GridExecutionState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
    mark: MarkId,
    value: Option<i64>,
    match_value: MarkValueMatch,
) -> Result<(), GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let value = match match_value {
        MarkValueMatch::Any => None,
        MarkValueMatch::Exact => value,
    };
    if object.is_empty() {
        state.remove_cell_mark_unchecked(position, mark, value);
        return Ok(());
    }
    let layer = object_layer(game, object)?;
    if state.get_layer_at(position, layer)? == object {
        state.remove_mark_unchecked(position, layer, mark, value);
    }
    Ok(())
}

fn expect_object_at<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &GridExecutionState<D, Size>,
    position: GridCoord<D>,
    object: ObjectId,
) -> Result<LayerId, GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let layer = object_layer(game, object)?;
    let found = state.get_layer_at(position, layer)?;
    if found != object {
        return Err(GridPatchError::ExpectedObject {
            position,
            layer,
            expected: object,
            found,
        });
    }
    Ok(layer)
}

fn expect_object_in_overlay<Size, ConditionDef, Rule, Condition, Frame, const D: usize>(
    game: &CompiledGameModel<ConditionDef, Rule, Condition, Frame>,
    state: &GridExecutionState<D, Size>,
    slots: &SlotOverlay<D>,
    position: GridCoord<D>,
    object: ObjectId,
) -> Result<LayerId, GridPatchError<D>>
where
    Size: GridSize<D>,
{
    let layer = object_layer(game, object)?;
    let found = slots.get(state, position, layer)?;
    if found != object {
        return Err(GridPatchError::ExpectedObject {
            position,
            layer,
            expected: object,
            found,
        });
    }
    Ok(layer)
}

fn validate_variable_update<Size: GridSize<D>, const D: usize>(
    state: &GridExecutionState<D, Size>,
    variable: VariableId,
    op: VariableUpdateOp,
    value: i64,
) -> Result<i64, GridPatchError<D>> {
    let current = state
        .variable_value(variable)
        .ok_or(GridStateError::VariableOutOfBounds { variable })?;
    let next = match op {
        VariableUpdateOp::Set => Some(value),
        VariableUpdateOp::Add => current.checked_add(value),
        VariableUpdateOp::Subtract => current.checked_sub(value),
        VariableUpdateOp::Multiply => current.checked_mul(value),
        VariableUpdateOp::Divide => {
            if value == 0 {
                return Err(GridStateError::VariableDivisionByZero { variable }.into());
            }
            current.checked_div(value)
        }
        VariableUpdateOp::Remainder => {
            if value == 0 {
                return Err(GridStateError::VariableDivisionByZero { variable }.into());
            }
            current.checked_rem(value)
        }
    }
    .ok_or(GridStateError::VariableOverflow { variable })?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled_game::{CompiledGame, ObjectDef};
    use crate::state::State;

    fn position(x: u16, y: u16) -> GridCoord<2> {
        GridCoord::new([x, y])
    }

    #[test]
    fn mark_cleanup_after_same_patch_object_removal_is_noop() {
        let object = ObjectId(1);
        let mark = MarkId(0);
        let game = CompiledGame::new(
            1,
            vec![ObjectDef {
                id: object,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let mut committed = State::empty(1, 1, 1, 1).unwrap();
        committed.place_object(&game, 0, 0, object).unwrap();
        let mut state = GridExecutionState::new(committed);
        state.set_mark_unchecked(position(0, 0), LayerId(0), mark, Some(7));

        let patch = Patch::from_ops(vec![
            PatchOp::Remove {
                position: position(0, 0),
                object,
            },
            PatchOp::RemoveMark {
                position: position(0, 0),
                object,
                mark,
                value: Some(7),
                match_value: MarkValueMatch::Exact,
            },
        ]);

        let mut next = state.clone();
        patch.apply_execution_in_place(&game, &mut next).unwrap();
        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert!(!next.has_mark_at(&game, position(0, 0), object, mark, Some(7)));
    }

    #[test]
    fn mark_cleanup_still_rejects_initially_missing_object() {
        let object = ObjectId(1);
        let game = CompiledGame::new(
            1,
            vec![ObjectDef {
                id: object,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let state = State::empty(1, 1, 1, 1).unwrap();
        let patch = Patch::from_ops(vec![PatchOp::RemoveMark {
            position: position(0, 0),
            object,
            mark: MarkId(0),
            value: Some(7),
            match_value: MarkValueMatch::Exact,
        }]);

        assert!(matches!(
            patch.validate(&game, &state),
            Err(PatchError::ExpectedObject {
                position: found_position,
                layer: LayerId(0),
                expected,
                found: ObjectId::EMPTY,
            }) if found_position == position(0, 0) && expected == object
        ));
    }

    #[test]
    fn add_sets_the_layer_slot_and_clears_marks_from_the_displaced_object() {
        let existing = ObjectId(1);
        let added = ObjectId(2);
        let mark = MarkId(0);
        let game = CompiledGame::new(
            1,
            vec![
                ObjectDef {
                    id: existing,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: added,
                    layer_id: LayerId(0),
                },
            ],
            Vec::new(),
        );
        let mut committed = State::empty(1, 1, 1, 2).unwrap();
        committed.place_object(&game, 0, 0, existing).unwrap();
        let mut state = GridExecutionState::new(committed);
        state.set_mark_unchecked(position(0, 0), LayerId(0), mark, Some(7));

        let patch = Patch::from_ops(vec![PatchOp::Add {
            position: position(0, 0),
            object: added,
        }]);
        let mut next = state.clone();
        patch.apply_execution_in_place(&game, &mut next).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), added);
        assert_eq!(next.object_count(existing), 0);
        assert_eq!(next.object_count(added), 1);
        assert!(!next.has_mark_at(&game, position(0, 0), added, mark, Some(7)));
    }

    #[test]
    fn idempotent_add_preserves_marks_on_the_existing_object() {
        let object = ObjectId(1);
        let mark = MarkId(0);
        let game = CompiledGame::new(
            1,
            vec![ObjectDef {
                id: object,
                layer_id: LayerId(0),
            }],
            Vec::new(),
        );
        let mut committed = State::empty(1, 1, 1, 1).unwrap();
        committed.place_object(&game, 0, 0, object).unwrap();
        let mut state = GridExecutionState::new(committed);
        state.set_mark_unchecked(position(0, 0), LayerId(0), mark, Some(7));

        let patch = Patch::from_ops(vec![PatchOp::Add {
            position: position(0, 0),
            object,
        }]);
        let mut next = state.clone();
        patch.apply_execution_in_place(&game, &mut next).unwrap();
        assert!(next.has_mark_at(&game, position(0, 0), object, mark, Some(7)));
    }

    #[test]
    fn ordered_adds_set_the_slot_to_the_last_object() {
        let first = ObjectId(1);
        let second = ObjectId(2);
        let game = CompiledGame::new(
            1,
            vec![
                ObjectDef {
                    id: first,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: second,
                    layer_id: LayerId(0),
                },
            ],
            Vec::new(),
        );
        let state = State::empty(1, 1, 1, 2).unwrap();
        let patch = Patch::from_ops(vec![
            PatchOp::Add {
                position: position(0, 0),
                object: first,
            },
            PatchOp::Add {
                position: position(0, 0),
                object: second,
            },
        ]);
        let next = patch.apply(&game, &state).unwrap();
        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), second);
    }

    #[test]
    fn replace_overwrites_an_occupied_add_layer() {
        let removed = ObjectId(1);
        let existing = ObjectId(2);
        let added = ObjectId(3);
        let game = CompiledGame::new(
            2,
            vec![
                ObjectDef {
                    id: removed,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: existing,
                    layer_id: LayerId(1),
                },
                ObjectDef {
                    id: added,
                    layer_id: LayerId(1),
                },
            ],
            Vec::new(),
        );
        let mut state = State::empty(1, 1, 2, 3).unwrap();
        state.place_object(&game, 0, 0, removed).unwrap();
        state.place_object(&game, 0, 0, existing).unwrap();

        let patch = Patch::from_ops(vec![PatchOp::Replace {
            position: position(0, 0),
            remove: removed,
            add: added,
        }]);
        let next = patch.apply(&game, &state).unwrap();
        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(0, 0, LayerId(1)).unwrap(), added);
    }

    #[test]
    fn move_overwrites_an_occupied_destination_and_transfers_only_the_moved_marks() {
        let moved = ObjectId(1);
        let displaced = ObjectId(2);
        let mark = MarkId(0);
        let game = CompiledGame::new(
            1,
            vec![
                ObjectDef {
                    id: moved,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: displaced,
                    layer_id: LayerId(0),
                },
            ],
            Vec::new(),
        );
        let mut committed = State::empty(2, 1, 1, 2).unwrap();
        committed.place_object(&game, 0, 0, moved).unwrap();
        committed.place_object(&game, 1, 0, displaced).unwrap();
        let mut state = GridExecutionState::new(committed);
        state.set_mark_unchecked(position(0, 0), LayerId(0), mark, Some(1));
        state.set_mark_unchecked(position(1, 0), LayerId(0), mark, Some(2));

        let patch = Patch::from_ops(vec![PatchOp::Move {
            from: position(0, 0),
            to: position(1, 0),
            object: moved,
        }]);
        let mut next = state.clone();
        patch.apply_execution_in_place(&game, &mut next).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(1, 0, LayerId(0)).unwrap(), moved);
        assert_eq!(next.object_count(moved), 1);
        assert_eq!(next.object_count(displaced), 0);
        assert!(next.has_mark_at(&game, position(1, 0), moved, mark, Some(1)));
        assert!(!next.has_mark_at(&game, position(1, 0), moved, mark, Some(2)));
    }

    #[test]
    fn ordered_moves_to_one_destination_keep_the_last_write() {
        let first = ObjectId(1);
        let second = ObjectId(2);
        let displaced = ObjectId(3);
        let game = CompiledGame::new(
            1,
            vec![
                ObjectDef {
                    id: first,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: second,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: displaced,
                    layer_id: LayerId(0),
                },
            ],
            Vec::new(),
        );
        let mut state = State::empty(3, 1, 1, 3).unwrap();
        state.place_object(&game, 0, 0, first).unwrap();
        state.place_object(&game, 1, 0, second).unwrap();
        state.place_object(&game, 2, 0, displaced).unwrap();

        let patch = Patch::from_ops(vec![
            PatchOp::Move {
                from: position(0, 0),
                to: position(2, 0),
                object: first,
            },
            PatchOp::Move {
                from: position(1, 0),
                to: position(2, 0),
                object: second,
            },
        ]);
        let next = patch.apply(&game, &state).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(1, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(2, 0, LayerId(0)).unwrap(), second);
        assert_eq!(next.object_count(first), 0);
        assert_eq!(next.object_count(second), 1);
        assert_eq!(next.object_count(displaced), 0);
    }

    #[test]
    fn chained_moves_read_all_sources_before_writing_destinations() {
        let first = ObjectId(1);
        let second = ObjectId(2);
        let game = CompiledGame::new(
            1,
            vec![
                ObjectDef {
                    id: first,
                    layer_id: LayerId(0),
                },
                ObjectDef {
                    id: second,
                    layer_id: LayerId(0),
                },
            ],
            Vec::new(),
        );
        let mut state = State::empty(3, 1, 1, 2).unwrap();
        state.place_object(&game, 0, 0, first).unwrap();
        state.place_object(&game, 1, 0, second).unwrap();

        let patch = Patch::from_ops(vec![
            PatchOp::Move {
                from: position(0, 0),
                to: position(1, 0),
                object: first,
            },
            PatchOp::Move {
                from: position(1, 0),
                to: position(2, 0),
                object: second,
            },
        ]);
        let next = patch.apply(&game, &state).unwrap();

        assert_eq!(next.get_layer(0, 0, LayerId(0)).unwrap(), ObjectId::EMPTY);
        assert_eq!(next.get_layer(1, 0, LayerId(0)).unwrap(), first);
        assert_eq!(next.get_layer(2, 0, LayerId(0)).unwrap(), second);
    }
}
