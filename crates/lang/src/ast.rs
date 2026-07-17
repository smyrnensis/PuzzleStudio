use puzzle_core::{ComparisonOp, ConditionId, ObjectId, RuleApplication, VariableUpdateOp};

use crate::{PatternBlock, loaded::SceneEffect};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectionName(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OrientationExpr {
    Neutral,
    Fixed(DirectionName),
    Input,
    InputSet(String),
}

#[derive(Clone, Debug)]
pub(crate) struct RuleDefinitionAst {
    pub(crate) name: String,
    pub(crate) application: RuleApplication,
    pub(crate) statements: Vec<StatementAst>,
}

#[derive(Clone, Debug)]
pub(crate) enum StatementAst {
    LocalRoutine {
        definition: RuleDefinitionAst,
        source_line: String,
        source_line_number: Option<usize>,
    },
    Call {
        name: String,
        source_line: String,
        source_line_number: Option<usize>,
    },
    Conditional {
        source_line: String,
        source_line_number: Option<usize>,
        condition: PatternConditionAst,
        then_statements: Vec<StatementAst>,
        else_statements: Vec<StatementAst>,
    },
    Block {
        application: RuleApplication,
        statements: Vec<StatementAst>,
    },
    RepeatUntil {
        source_line: String,
        source_line_number: Option<usize>,
        condition: ConditionAst,
        statements: Vec<StatementAst>,
    },
    Fix {
        defaults: FixDefaults,
        statements: Vec<StatementAst>,
    },
    If {
        source_line: String,
        source_line_number: Option<usize>,
        condition: ConditionAst,
        then_statements: Vec<StatementAst>,
        else_statements: Vec<StatementAst>,
    },
    Effect {
        source_line: String,
        source_line_number: Option<usize>,
        effects: Vec<EffectAst>,
    },
    Rewrite(OrientedRewriteAst),
}

#[derive(Clone, Debug)]
pub(crate) struct PatternConditionAst {
    pub(crate) predicate: PatternPredicateAst,
    pub(crate) orientation: OrientationExpr,
    pub(crate) pattern: PatternBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternPredicateAst {
    Some,
    None,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FixDefaults {
    pub(crate) application: Option<RuleApplication>,
    pub(crate) orientation: Option<OrientationExpr>,
}

#[derive(Clone, Debug)]
pub(crate) enum ConditionAst {
    All(Vec<ConditionAst>),
    Any(Vec<ConditionAst>),
    AllObjectsOn {
        subjects: Vec<ObjectId>,
        covers: Vec<ObjectId>,
    },
    InputIs(String),
    InputIn(String),
    VariableEquals {
        name: String,
        value: i64,
    },
    VariableCompare {
        name: String,
        op: ComparisonOp,
        value: i64,
    },
    ConditionEquals {
        name: String,
        value: i64,
    },
    ConditionNonZero(String),
    ConditionCompare {
        name: String,
        op: ComparisonOp,
        value: i64,
    },
    InlineConditionValueEquals {
        kind: ConditionValueAst,
        value: i64,
    },
    InlineConditionNonZero(ConditionValueAst),
    InlineConditionCompare {
        kind: ConditionValueAst,
        op: ComparisonOp,
        value: i64,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum EffectAst {
    Cancel,
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    PlaySfx {
        name: String,
    },
    PlayMusic {
        name: String,
    },
    PauseMusic {
        name: Option<String>,
    },
    ResumeMusic {
        name: Option<String>,
    },
    StopMusic {
        name: Option<String>,
    },
    Wait {
        milliseconds: u64,
    },
    WaitAnimation,
    Message {
        text: String,
        literal: bool,
    },
    Scene(SceneEffect),
    UpdateVariable {
        name: String,
        op: VariableUpdateOp,
        value: VariableValueAst,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum VariableValueAst {
    Literal(i64),
    TagCapture(String),
}

#[derive(Clone, Debug)]
pub(crate) struct ConditionDefinitionAst {
    pub(crate) id: ConditionId,
    pub(crate) kind: ConditionValueAst,
}

pub(crate) type QueryDefinitionAst = crate::solver_surface::SolverSurfaceQueryDefinition;

#[derive(Clone, Debug)]
pub(crate) enum ConditionValueAst {
    CountObjects(Vec<ObjectId>),
    ExistsObjects(Vec<ObjectId>),
    NoneObjects(Vec<ObjectId>),
    CountMatches(ConditionPatternAst),
    ExistsMatches(ConditionPatternAst),
    NoneMatches(ConditionPatternAst),
}

#[derive(Clone, Debug)]
pub(crate) struct ConditionPatternAst {
    pub(crate) orientation: OrientationExpr,
    pub(crate) pattern: PatternBlock,
}

pub(crate) type SolverStrategyAst = crate::solver_surface::SolverSurfaceStrategy;

#[derive(Clone, Debug)]
pub(crate) struct OrientedRewriteAst {
    pub(crate) source_line: String,
    pub(crate) source_line_number: Option<usize>,
    pub(crate) orientation: OrientationExpr,
    pub(crate) application: Option<RuleApplication>,
    pub(crate) before: PatternBlock,
    pub(crate) after: PatternBlock,
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) after_effects: Vec<EffectAst>,
    pub(crate) after_call: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct DirectionalInput {
    pub(crate) input: puzzle_core::InputId,
    pub(crate) direction: String,
}
