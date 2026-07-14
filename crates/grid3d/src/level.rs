use std::collections::BTreeSet;

use crate::{CompiledGame3, CompiledGameError3, Coord3, ObjectId, Size3, State3, StateError3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level3 {
    pub size: Size3,
    pub cells: Vec<LevelCell3>,
}

impl Level3 {
    pub fn new(size: Size3, cells: Vec<LevelCell3>) -> Self {
        Self { size, cells }
    }

    pub fn build_state(&self, game: &CompiledGame3) -> Result<State3, LevelError3> {
        game.validate()?;
        let mut state = State3::empty(self.size, game.layer_count)?;
        for cell in &self.cells {
            for object in &cell.objects {
                if object.is_empty() {
                    return Err(LevelError3::EmptyObject {
                        position: cell.position,
                    });
                }
                state.place_object(game, cell.position, *object)?;
            }
        }
        Ok(state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelEntry3 {
    pub name: String,
    pub level: Level3,
}

impl LevelEntry3 {
    pub fn new(name: impl Into<String>, level: Level3) -> Self {
        Self {
            name: name.into(),
            level,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelBundle3 {
    pub game: CompiledGame3,
    pub levels: Vec<LevelEntry3>,
}

impl LevelBundle3 {
    pub fn new(game: CompiledGame3, levels: Vec<LevelEntry3>) -> Self {
        Self { game, levels }
    }

    pub fn checked_new(
        game: CompiledGame3,
        levels: Vec<LevelEntry3>,
    ) -> Result<Self, LevelBundleError3> {
        let bundle = Self::new(game, levels);
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), LevelBundleError3> {
        self.game.validate()?;
        if self.levels.is_empty() {
            return Err(LevelBundleError3::EmptyLevels);
        }

        let mut names = BTreeSet::new();
        for (index, entry) in self.levels.iter().enumerate() {
            if entry.name.is_empty() {
                return Err(LevelBundleError3::EmptyLevelName { index });
            }
            if !names.insert(entry.name.clone()) {
                return Err(LevelBundleError3::DuplicateLevelName {
                    name: entry.name.clone(),
                });
            }
            entry
                .level
                .build_state(&self.game)
                .map_err(|source| LevelBundleError3::Level {
                    index,
                    name: entry.name.clone(),
                    source,
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

    pub fn level(&self, index: usize) -> Option<&LevelEntry3> {
        self.levels.get(index)
    }

    pub fn level_by_name(&self, name: &str) -> Option<(usize, &LevelEntry3)> {
        self.levels
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.name == name)
    }

    pub fn build_level_state(&self, index: usize) -> Result<State3, LevelBundleError3> {
        let entry = self
            .levels
            .get(index)
            .ok_or(LevelBundleError3::LevelIndexOutOfBounds {
                index,
                level_count: self.levels.len(),
            })?;
        entry
            .level
            .build_state(&self.game)
            .map_err(|source| LevelBundleError3::Level {
                index,
                name: entry.name.clone(),
                source,
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelCell3 {
    pub position: Coord3,
    pub objects: Vec<ObjectId>,
}

impl LevelCell3 {
    pub fn new(position: Coord3, objects: Vec<ObjectId>) -> Self {
        Self { position, objects }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelError3 {
    CompiledGame(CompiledGameError3),
    State(StateError3),
    EmptyObject { position: Coord3 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LevelBundleError3 {
    CompiledGame(CompiledGameError3),
    EmptyLevels,
    EmptyLevelName {
        index: usize,
    },
    DuplicateLevelName {
        name: String,
    },
    Level {
        index: usize,
        name: String,
        source: LevelError3,
    },
    LevelIndexOutOfBounds {
        index: usize,
        level_count: usize,
    },
}

impl From<CompiledGameError3> for LevelError3 {
    fn from(value: CompiledGameError3) -> Self {
        Self::CompiledGame(value)
    }
}

impl From<CompiledGameError3> for LevelBundleError3 {
    fn from(value: CompiledGameError3) -> Self {
        Self::CompiledGame(value)
    }
}

impl From<StateError3> for LevelError3 {
    fn from(value: StateError3) -> Self {
        Self::State(value)
    }
}
