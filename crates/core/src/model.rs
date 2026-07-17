use std::collections::BTreeSet;

use puzzle_kernel::{CompiledGameError, GridShape, SpatialVector};
use serde::{Deserialize, Serialize};

use crate::{GridCompiledGame, GridExecutableProgram, GridSize, GridState, InputId, LayerId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridInput<const D: usize> {
    pub id: InputId,
    pub name: String,
    pub direction: Option<SpatialVector<D>>,
    pub keys: Vec<String>,
}

impl<const D: usize> GridInput<D> {
    pub fn directional(id: InputId, name: impl Into<String>, direction: SpatialVector<D>) -> Self {
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
pub struct GridLevel<const D: usize, Size: GridSize<D>> {
    pub name: String,
    pub initial_state: GridState<D, Size>,
    pub program: GridExecutableProgram<D>,
    pub level_start_program: Option<GridExecutableProgram<D>>,
    pub level_clear_program: Option<GridExecutableProgram<D>>,
}

impl<const D: usize, Size: GridSize<D>> GridLevel<D, Size> {
    pub fn new(
        name: impl Into<String>,
        initial_state: GridState<D, Size>,
        program: GridExecutableProgram<D>,
        level_start_program: Option<GridExecutableProgram<D>>,
        level_clear_program: Option<GridExecutableProgram<D>>,
    ) -> Self {
        Self {
            name: name.into(),
            initial_state,
            program,
            level_start_program,
            level_clear_program,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLevelBundle<const D: usize, Size: GridSize<D>> {
    pub game: GridCompiledGame<D>,
    pub levels: Vec<GridLevel<D, Size>>,
}

impl<const D: usize, Size: GridSize<D>> GridLevelBundle<D, Size> {
    pub fn new(game: GridCompiledGame<D>, levels: Vec<GridLevel<D, Size>>) -> Self {
        Self { game, levels }
    }

    pub fn checked_new(
        game: GridCompiledGame<D>,
        levels: Vec<GridLevel<D, Size>>,
    ) -> Result<Self, GridLevelBundleError> {
        let bundle = Self::new(game, levels);
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), GridLevelBundleError> {
        self.game.validate()?;
        if self.levels.is_empty() {
            return Err(GridLevelBundleError::EmptyLevels);
        }

        let mut names = BTreeSet::new();
        for (index, level) in self.levels.iter().enumerate() {
            if level.name.is_empty() {
                return Err(GridLevelBundleError::EmptyLevelName { index });
            }
            if !names.insert(level.name.clone()) {
                return Err(GridLevelBundleError::DuplicateLevelName {
                    name: level.name.clone(),
                });
            }
            validate_level_state(&self.game, &level.initial_state).map_err(|reason| {
                GridLevelBundleError::InvalidLevelState {
                    index,
                    name: level.name.clone(),
                    reason,
                }
            })?;
        }

        Ok(())
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub fn level(&self, index: usize) -> Option<&GridLevel<D, Size>> {
        self.levels.get(index)
    }

    pub fn level_by_name(&self, name: &str) -> Option<(usize, &GridLevel<D, Size>)> {
        self.levels
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.name == name)
    }

    pub fn build_level_state(
        &self,
        index: usize,
    ) -> Result<GridState<D, Size>, GridLevelBundleError> {
        let entry = self
            .levels
            .get(index)
            .ok_or(GridLevelBundleError::LevelIndexOutOfBounds {
                index,
                level_count: self.levels.len(),
            })?;
        Ok(entry.initial_state.clone())
    }
}

fn validate_level_state<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    state: &GridState<D, Size>,
) -> Result<(), String> {
    if state.layer_count != game.layer_count {
        return Err(format!(
            "state layer count {} does not match game layer count {}",
            state.layer_count, game.layer_count
        ));
    }
    let expected_slots = GridShape::<D>::new(state.size.axes(), state.layer_count)
        .and_then(|shape| shape.slot_count())
        .ok_or_else(|| "state dimensions exceed runtime limits".to_string())?;
    if state.slots().len() != expected_slots {
        return Err(format!(
            "state has {} slots, expected {expected_slots}",
            state.slots().len()
        ));
    }
    for (index, object) in state.slots().iter().copied().enumerate() {
        if object.is_empty() {
            continue;
        }
        let expected_layer = game
            .object_layer(object)
            .ok_or_else(|| format!("state contains unknown object {}", object.0))?;
        let actual_layer = LayerId((index % usize::from(state.layer_count)) as u16);
        if actual_layer != expected_layer {
            return Err(format!(
                "object {} is stored on layer {}, expected {}",
                object.0, actual_layer.0, expected_layer.0
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridLevelBundleError {
    CompiledGame(CompiledGameError),
    EmptyLevels,
    EmptyLevelName {
        index: usize,
    },
    DuplicateLevelName {
        name: String,
    },
    InvalidLevelState {
        index: usize,
        name: String,
        reason: String,
    },
    LevelIndexOutOfBounds {
        index: usize,
        level_count: usize,
    },
}

impl From<CompiledGameError> for GridLevelBundleError {
    fn from(value: CompiledGameError) -> Self {
        Self::CompiledGame(value)
    }
}
