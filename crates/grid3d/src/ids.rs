use puzzle_kernel::KernelId;
pub use puzzle_kernel::{InputId, LayerId, ObjectId, VariableId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConditionId3(pub u16);

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RuleId3(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MarkId3(pub u16);

impl KernelId for MarkId3 {
    fn raw(self) -> u16 {
        self.0
    }
}
