use std::fmt;

#[derive(Debug)]
pub enum AppError {
    CoreState(puzzle_core::StateError),
    Parse(String),
}

impl From<puzzle_core::StateError> for AppError {
    fn from(value: puzzle_core::StateError) -> Self {
        Self::CoreState(value)
    }
}

impl From<puzzle_scene::SceneBlockParseError> for AppError {
    fn from(value: puzzle_scene::SceneBlockParseError) -> Self {
        Self::Parse(value.to_string())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreState(error) => write!(f, "{error:?}"),
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AppError {}
