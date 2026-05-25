use crate::ids::{GlobalId, InputId, LayerId, ObjectId, QueryId, RuleId, ScratchId};

#[derive(Clone, Debug)]
pub struct CompiledGame {
    pub layer_count: u16,
    objects: Vec<ObjectDef>,
    scratch: Vec<ScratchDef>,
    queries: Vec<QueryDef>,
    rules: Vec<Rule>,
    program: Vec<RuleStep>,
    visual_objects: Vec<ObjectId>,
    visual_rules: Vec<RuleId>,
}

impl CompiledGame {
    pub fn new(layer_count: u16, objects: Vec<ObjectDef>, rules: Vec<Rule>) -> Self {
        let program = rules.iter().cloned().map(RuleStep::Rule).collect();
        Self {
            layer_count,
            objects,
            scratch: Vec::new(),
            queries: Vec::new(),
            rules,
            program,
            visual_objects: Vec::new(),
            visual_rules: Vec::new(),
        }
    }

    pub fn new_with_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        program: Vec<RuleStep>,
    ) -> Self {
        Self::new_with_queries_and_program(layer_count, objects, Vec::new(), program)
    }

    pub fn new_with_queries_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        queries: Vec<QueryDef>,
        program: Vec<RuleStep>,
    ) -> Self {
        Self::new_with_scratch_queries_and_program(
            layer_count,
            objects,
            Vec::new(),
            queries,
            program,
        )
    }

    pub fn new_with_scratch_queries_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        scratch: Vec<ScratchDef>,
        queries: Vec<QueryDef>,
        program: Vec<RuleStep>,
    ) -> Self {
        Self::new_with_scratch_queries_program_roles(
            layer_count,
            objects,
            scratch,
            queries,
            program,
            Vec::new(),
            Vec::new(),
        )
    }

    pub fn new_with_scratch_queries_program_roles(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        scratch: Vec<ScratchDef>,
        queries: Vec<QueryDef>,
        program: Vec<RuleStep>,
        mut visual_objects: Vec<ObjectId>,
        mut visual_rules: Vec<RuleId>,
    ) -> Self {
        let mut rules = Vec::new();
        collect_rules(&program, &mut rules);
        visual_objects.sort();
        visual_objects.dedup();
        visual_rules.sort();
        visual_rules.dedup();
        Self {
            layer_count,
            objects,
            scratch,
            queries,
            rules,
            program,
            visual_objects,
            visual_rules,
        }
    }

    #[inline]
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    #[inline]
    pub fn scratch(&self) -> &[ScratchDef] {
        &self.scratch
    }

    #[inline]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    #[inline]
    pub fn queries(&self) -> &[QueryDef] {
        &self.queries
    }

    #[inline]
    pub fn query(&self, query: QueryId) -> Option<&QueryDef> {
        self.queries.get(usize::from(query.0))
    }

    #[inline]
    pub fn program(&self) -> &[RuleStep] {
        &self.program
    }

    #[inline]
    pub fn object(&self, object: ObjectId) -> Option<&ObjectDef> {
        if object.is_empty() {
            return None;
        }

        self.objects.get(usize::from(object.0 - 1))
    }

    #[inline]
    pub fn is_visual_object(&self, object: ObjectId) -> bool {
        self.visual_objects.binary_search(&object).is_ok()
    }

    #[inline]
    pub fn visual_objects(&self) -> &[ObjectId] {
        &self.visual_objects
    }

    #[inline]
    pub fn is_main_object(&self, object: ObjectId) -> bool {
        !object.is_empty() && !self.is_visual_object(object)
    }

    #[inline]
    pub fn is_visual_rule(&self, rule: RuleId) -> bool {
        self.visual_rules.binary_search(&rule).is_ok()
    }

    pub fn main_layers(&self) -> Vec<LayerId> {
        let mut layers = self
            .objects
            .iter()
            .filter_map(|object| self.is_main_object(object.id).then_some(object.layer_id))
            .collect::<Vec<_>>();
        layers.sort();
        layers.dedup();
        layers
    }

    #[inline]
    pub fn object_layer(&self, object: ObjectId) -> Option<LayerId> {
        self.object(object).map(|def| def.layer_id)
    }

    pub fn solver_core(&self) -> Self {
        let program = filter_visual_steps(&self.program, &self.visual_rules);
        Self::new_with_scratch_queries_program_roles(
            self.layer_count,
            self.objects.clone(),
            self.scratch.clone(),
            self.queries.clone(),
            program,
            self.visual_objects.clone(),
            Vec::new(),
        )
    }
}

fn collect_rules(program: &[RuleStep], rules: &mut Vec<Rule>) {
    for step in program {
        match step {
            RuleStep::Rule(rule) => rules.push(rule.clone()),
            RuleStep::ConditionalBlock { steps, .. } => collect_rules(steps, rules),
            RuleStep::Block { steps, .. } => collect_rules(steps, rules),
        }
    }
}

fn filter_visual_steps(program: &[RuleStep], visual_rules: &[RuleId]) -> Vec<RuleStep> {
    program
        .iter()
        .filter_map(|step| filter_visual_step(step, visual_rules))
        .collect()
}

fn filter_visual_step(step: &RuleStep, visual_rules: &[RuleId]) -> Option<RuleStep> {
    match step {
        RuleStep::Rule(rule) => visual_rules
            .binary_search(&rule.id)
            .is_err()
            .then(|| RuleStep::Rule(rule.clone())),
        RuleStep::ConditionalBlock { condition, steps } => {
            let steps = filter_visual_steps(steps, visual_rules);
            (!steps.is_empty()).then(|| RuleStep::ConditionalBlock {
                condition: condition.clone(),
                steps,
            })
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => {
            let steps = filter_visual_steps(steps, visual_rules);
            (!steps.is_empty()).then(|| RuleStep::Block {
                application: *application,
                stop_condition: stop_condition.clone(),
                steps,
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectDef {
    pub id: ObjectId,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug)]
pub struct ScratchDef {
    pub id: ScratchId,
    pub kind: ScratchKind,
    pub values: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchKind {
    Flag,
    Int,
    Enum,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: RuleId,
    pub guards: Vec<Guard>,
    pub application: RuleApplication,
    pub pattern: Pattern,
    pub writes: Vec<WriteOp>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug)]
pub enum RuleStep {
    Rule(Rule),
    ConditionalBlock {
        condition: RuleCondition,
        steps: Vec<RuleStep>,
    },
    Block {
        application: RuleApplication,
        stop_condition: Option<RuleCondition>,
        steps: Vec<RuleStep>,
    },
}

#[derive(Clone, Debug)]
pub enum RuleCondition {
    AnyMatches(Vec<Pattern>),
    NoMatches(Vec<Pattern>),
    AnyInputMatches(Vec<(InputId, Pattern)>),
    NoInputMatches(Vec<(InputId, Pattern)>),
    GuardBranches(Vec<Vec<Guard>>),
}

#[derive(Clone, Debug)]
pub enum Guard {
    InputIs(InputId),
    GlobalEquals {
        global: GlobalId,
        value: i64,
    },
    GlobalCompare {
        global: GlobalId,
        op: ComparisonOp,
        value: i64,
    },
    QueryEquals {
        query: QueryId,
        value: i64,
    },
    QueryNonZero(QueryId),
    QueryCompare {
        query: QueryId,
        op: ComparisonOp,
        value: i64,
    },
    QueryValue {
        kind: QueryKind,
        value: i64,
    },
    QueryValueNonZero(QueryKind),
    QueryValueCompare {
        kind: QueryKind,
        op: ComparisonOp,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchPattern {
    pub object: ObjectId,
    pub scratch: ScratchId,
    pub value: Option<i64>,
    pub match_value: ScratchValueMatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchValueMatch {
    Any,
    Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOp {
    Eq,
    NotEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Cancel,
    Win,
    Restart,
    NextLevel,
    Again,
    UpdateGlobal {
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalUpdateOp {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Clone, Debug)]
pub struct QueryDef {
    pub id: QueryId,
    pub kind: QueryKind,
}

#[derive(Clone, Debug)]
pub enum QueryKind {
    CountObjects(Vec<ObjectId>),
    ExistsObjects(Vec<ObjectId>),
    CountMatches(Vec<Pattern>),
    ExistsMatches(Vec<Pattern>),
    CountInputMatches(Vec<(InputId, Pattern)>),
    ExistsInputMatches(Vec<(InputId, Pattern)>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuleApplication {
    Once,
    OnceAll,
    OncePerLevel,
    #[default]
    UntilStable,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub components: Vec<PatternComponent>,
}

#[derive(Clone, Debug)]
pub struct PatternComponent {
    pub cells: Vec<MatchCell>,
    pub gap_count: u16,
}

#[derive(Clone, Debug)]
pub struct MatchCell {
    pub offset: Offset,
    pub require_objects: Vec<ObjectId>,
    pub forbid_objects: Vec<ObjectId>,
    pub require_scratch: Vec<ScratchPattern>,
    pub forbid_scratch: Vec<ScratchPattern>,
}

#[derive(Clone, Debug)]
pub enum Offset {
    Fixed {
        dx: i16,
        dy: i16,
    },
    Variable {
        base_dx: i16,
        base_dy: i16,
        gap_terms: Vec<GapTerm>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct GapTerm {
    pub gap_index: u16,
    pub dx: i16,
    pub dy: i16,
}

#[derive(Clone, Debug)]
pub enum WriteOp {
    Add {
        component: u16,
        offset: Offset,
        object: ObjectId,
    },
    Remove {
        component: u16,
        offset: Offset,
        object: ObjectId,
    },
    Move {
        component: u16,
        from_offset: Offset,
        to_offset: Offset,
        object: ObjectId,
    },
    Replace {
        component: u16,
        offset: Offset,
        remove: ObjectId,
        add: ObjectId,
    },
    SetScratch {
        component: u16,
        offset: Offset,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
    },
    RemoveScratch {
        component: u16,
        offset: Offset,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<i64>,
        match_value: ScratchValueMatch,
    },
}
