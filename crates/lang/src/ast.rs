use puzzle_core::{ComparisonOp, GlobalUpdateOp, ObjectId, QueryId, RuleApplication};

use crate::PatternBlock;

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
    pub(crate) role: RuleRole,
    pub(crate) application: RuleApplication,
    pub(crate) statements: Vec<StatementAst>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RuleRole {
    #[default]
    Main,
    Visual,
}

#[derive(Clone, Debug)]
pub(crate) enum StatementAst {
    Call(String),
    DisplayCall(String),
    DisplayRewrite(OrientedRewriteAst),
    DisplayBlock(Vec<StatementAst>),
    Conditional {
        condition: PatternConditionAst,
        then_statements: Vec<StatementAst>,
        else_statements: Vec<StatementAst>,
    },
    Block {
        application: RuleApplication,
        statements: Vec<StatementAst>,
    },
    RepeatUntil {
        condition: ConditionAst,
        statements: Vec<StatementAst>,
    },
    Fix {
        defaults: FixDefaults,
        statements: Vec<StatementAst>,
    },
    If {
        condition: ConditionAst,
        then_statements: Vec<StatementAst>,
        else_statements: Vec<StatementAst>,
    },
    Effect {
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

impl PatternPredicateAst {
    pub(crate) fn inverted(self) -> Self {
        match self {
            Self::Some => Self::None,
            Self::None => Self::Some,
        }
    }
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
    InputIs(String),
    InputIn(String),
    GlobalEquals {
        name: String,
        value: i64,
    },
    GlobalCompare {
        name: String,
        op: ComparisonOp,
        value: i64,
    },
    QueryEquals {
        name: String,
        value: i64,
    },
    QueryNonZero(String),
    QueryCompare {
        name: String,
        op: ComparisonOp,
        value: i64,
    },
    QueryValueEquals {
        kind: QueryKindAst,
        value: i64,
    },
    QueryValueNonZero(QueryKindAst),
    QueryValueCompare {
        kind: QueryKindAst,
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
    PlaySfx {
        name: String,
    },
    Wait {
        milliseconds: Option<u64>,
    },
    Message {
        text: String,
        literal: bool,
    },
    UpdateGlobal {
        name: String,
        op: GlobalUpdateOp,
        value: i64,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct QueryDefinitionAst {
    pub(crate) id: QueryId,
    pub(crate) kind: QueryKindAst,
}

#[derive(Clone, Debug)]
pub(crate) enum QueryKindAst {
    CountObjects(Vec<ObjectId>),
    ExistsObjects(Vec<ObjectId>),
    CountMatches(QueryPatternAst),
    ExistsMatches(QueryPatternAst),
}

#[derive(Clone, Debug)]
pub(crate) struct QueryPatternAst {
    pub(crate) orientation: OrientationExpr,
    pub(crate) pattern: PatternBlock,
}

#[derive(Clone, Debug)]
pub(crate) struct OrientedRewriteAst {
    pub(crate) source_line: String,
    pub(crate) orientation: OrientationExpr,
    pub(crate) application: Option<RuleApplication>,
    pub(crate) before: PatternBlock,
    pub(crate) after: PatternBlock,
    pub(crate) effects: Vec<EffectAst>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Direction {
    pub(crate) input: puzzle_core::InputId,
    pub(crate) dx: i16,
    pub(crate) dy: i16,
}
