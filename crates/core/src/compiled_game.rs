use crate::ids::{ConditionId, InputId, LayerId, MarkId, ObjectId, RuleId, VariableId};
pub use puzzle_kernel::{
    ComparisonOp, LocalFrame, LocalFrameExtent, MarkKind, MarkValueMatch, RuleApplication,
    VariableUpdateOp,
};
pub type GridConditionValueKind<const D: usize> =
    puzzle_kernel::ConditionValueKind<ObjectId, GridPattern<D>, InputId>;
pub type GridConditionDef<const D: usize> =
    puzzle_kernel::RuleConditionDef<ConditionId, GridConditionValueKind<D>>;
pub type GridGuard<const D: usize> =
    puzzle_kernel::RuleGuard<VariableId, ConditionId, GridConditionValueKind<D>, InputId>;
pub type GridMatchCell<const D: usize> =
    puzzle_kernel::RuleMatchCell<GridOffset<D>, ObjectId, LayerId, MarkId>;
pub type GridPatternComponent<const D: usize> =
    puzzle_kernel::RulePatternComponent<GridMatchCell<D>>;
pub type GridPattern<const D: usize> = puzzle_kernel::RulePattern<GridPatternComponent<D>>;
pub type GridWriteOp<const D: usize> = puzzle_kernel::RuleWriteOp<GridOffset<D>, ObjectId, MarkId>;
pub type GridRuleCondition<const D: usize> =
    puzzle_kernel::ProgramCondition<GridPattern<D>, GridGuard<D>>;
pub type GridRule<const D: usize> =
    puzzle_kernel::RuleModel<RuleId, GridGuard<D>, GridPattern<D>, GridWriteOp<D>, Effect>;
pub type GridRuleStep<const D: usize> =
    puzzle_kernel::ProgramStep<GridRule<D>, GridRuleCondition<D>, LocalFrame<ObjectId>>;
pub type GridCompiledGame<const D: usize> = puzzle_kernel::CompiledGameModel<
    GridConditionDef<D>,
    GridRule<D>,
    GridRuleCondition<D>,
    LocalFrame<ObjectId>,
>;

pub type ConditionDef = GridConditionDef<2>;
pub type Guard = GridGuard<2>;
pub type MarkPattern = puzzle_kernel::RuleMarkPattern<ObjectId, MarkId>;
pub type ObjectSetMatcher = puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>;
pub type ObjectSetMarkPattern = puzzle_kernel::ObjectSetMarkPattern<MarkId>;
pub type PatternComponent = GridPatternComponent<2>;
pub type MatchCell = GridMatchCell<2>;
pub type Rule = GridRule<2>;
pub type RuleStep = GridRuleStep<2>;
pub type WriteOp = GridWriteOp<2>;
pub type CompiledGame = GridCompiledGame<2>;

pub type ObjectDef = puzzle_kernel::ObjectDef;
pub type MarkDef = puzzle_kernel::MarkDef;
pub type RuleCondition = GridRuleCondition<2>;
pub type Effect = puzzle_kernel::RuleEffect;

pub type ConditionValueKind = GridConditionValueKind<2>;
pub type Pattern = GridPattern<2>;
pub type Pattern2 = Pattern;

pub type Offset = puzzle_kernel::SpatialOffset<2>;
pub type GapTerm = puzzle_kernel::SpatialGapTerm<2>;
pub type GridOffset<const D: usize> = puzzle_kernel::SpatialOffset<D>;
pub type GridGapTerm<const D: usize> = puzzle_kernel::SpatialGapTerm<D>;
