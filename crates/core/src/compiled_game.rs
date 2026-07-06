use crate::ids::{ConditionId, GlobalId, InputId, LayerId, MarkId, ObjectId, RuleId};
pub use puzzle_kernel::{
    ComparisonOp, GlobalUpdateOp, LocalFrame, LocalFrameExtent, MarkKind, MarkValueMatch,
    RuleApplication,
};
use serde::{Deserialize, Serialize};
pub type ConditionDef = puzzle_kernel::RuleConditionDef<ConditionId, ConditionValueKind>;
pub type Guard = puzzle_kernel::RuleGuard<GlobalId, ConditionId, ConditionValueKind, InputId>;
pub type MarkPattern = puzzle_kernel::RuleMarkPattern<ObjectId, MarkId>;
pub type ObjectSetMatcher = puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>;
pub type ObjectSetMarkPattern = puzzle_kernel::ObjectSetMarkPattern<MarkId>;
pub type PatternComponent = puzzle_kernel::RulePatternComponent<MatchCell>;
pub type Rule = puzzle_kernel::RuleModel<RuleId, Guard, Pattern, WriteOp, Effect>;
pub type WriteOp = puzzle_kernel::RuleWriteOp<Offset, ObjectId, MarkId>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledGame {
    pub layer_count: u16,
    objects: Vec<ObjectDef>,
    mark: Vec<MarkDef>,
    condition_defs: Vec<ConditionDef>,
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
            mark: Vec::new(),
            condition_defs: Vec::new(),
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
        Self::new_with_condition_defs_and_program(layer_count, objects, Vec::new(), program)
    }

    pub fn new_with_condition_defs_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        condition_defs: Vec<ConditionDef>,
        program: Vec<RuleStep>,
    ) -> Self {
        Self::new_with_mark_condition_defs_and_program(
            layer_count,
            objects,
            Vec::new(),
            condition_defs,
            program,
        )
    }

    pub fn new_with_mark_condition_defs_and_program(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        mark: Vec<MarkDef>,
        condition_defs: Vec<ConditionDef>,
        program: Vec<RuleStep>,
    ) -> Self {
        Self::new_with_mark_condition_defs_program_roles(
            layer_count,
            objects,
            mark,
            condition_defs,
            program,
            Vec::new(),
            Vec::new(),
        )
    }

    pub fn new_with_mark_condition_defs_program_roles(
        layer_count: u16,
        objects: Vec<ObjectDef>,
        mark: Vec<MarkDef>,
        condition_defs: Vec<ConditionDef>,
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
            mark,
            condition_defs,
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
    pub fn mark(&self) -> &[MarkDef] {
        &self.mark
    }

    #[inline]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    #[inline]
    pub fn condition_defs(&self) -> &[ConditionDef] {
        &self.condition_defs
    }

    #[inline]
    pub fn condition_def(&self, condition: ConditionId) -> Option<&ConditionDef> {
        self.condition_defs.get(usize::from(condition.0))
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
        Self::new_with_mark_condition_defs_program_roles(
            self.layer_count,
            self.objects.clone(),
            self.mark.clone(),
            self.condition_defs.clone(),
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
            RuleStep::ConditionalBranch {
                then_steps,
                else_steps,
                ..
            } => {
                collect_rules(then_steps, rules);
                collect_rules(else_steps, rules);
            }
            RuleStep::Block { steps, .. } => collect_rules(steps, rules),
            RuleStep::AfterTriggered { steps, then_steps } => {
                collect_rules(steps, rules);
                collect_rules(then_steps, rules);
            }
            RuleStep::LocalFrame { steps, .. } => collect_rules(steps, rules),
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
        RuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            let then_steps = filter_visual_steps(then_steps, visual_rules);
            let else_steps = filter_visual_steps(else_steps, visual_rules);
            (!then_steps.is_empty() || !else_steps.is_empty()).then(|| {
                RuleStep::ConditionalBranch {
                    condition: condition.clone(),
                    then_steps,
                    else_steps,
                }
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
        RuleStep::AfterTriggered { steps, then_steps } => {
            let steps = filter_visual_steps(steps, visual_rules);
            let then_steps = filter_visual_steps(then_steps, visual_rules);
            (!steps.is_empty()).then(|| RuleStep::AfterTriggered { steps, then_steps })
        }
        RuleStep::LocalFrame { frame, steps } => {
            let steps = filter_visual_steps(steps, visual_rules);
            (!steps.is_empty()).then(|| RuleStep::LocalFrame {
                frame: frame.clone(),
                steps,
            })
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectDef {
    pub id: ObjectId,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkDef {
    pub id: MarkId,
    pub kind: MarkKind,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuleStep {
    Rule(Rule),
    ConditionalBlock {
        condition: RuleCondition,
        steps: Vec<RuleStep>,
    },
    ConditionalBranch {
        condition: RuleCondition,
        then_steps: Vec<RuleStep>,
        else_steps: Vec<RuleStep>,
    },
    Block {
        application: RuleApplication,
        stop_condition: Option<RuleCondition>,
        steps: Vec<RuleStep>,
    },
    AfterTriggered {
        steps: Vec<RuleStep>,
        then_steps: Vec<RuleStep>,
    },
    LocalFrame {
        frame: LocalFrame<ObjectId>,
        steps: Vec<RuleStep>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuleCondition {
    AnyMatches(Vec<Pattern>),
    NoMatches(Vec<Pattern>),
    AnyInputMatches(Vec<(InputId, Pattern)>),
    NoInputMatches(Vec<(InputId, Pattern)>),
    GuardBranches(Vec<Vec<Guard>>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Cancel,
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    UpdateGlobal {
        global: GlobalId,
        op: GlobalUpdateOp,
        value: i64,
    },
}

pub type ConditionValueKind = puzzle_kernel::ConditionValueKind<ObjectId, Pattern, InputId>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub components: Vec<PatternComponent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchCell {
    pub offset: Offset,
    pub require_null: bool,
    pub require_objects: Vec<ObjectId>,
    pub require_object_sets: Vec<ObjectSetMatcher>,
    pub forbid_objects: Vec<ObjectId>,
    pub require_mark: Vec<MarkPattern>,
    pub require_object_set_mark: Vec<ObjectSetMarkPattern>,
    pub forbid_mark: Vec<MarkPattern>,
    pub forbid_object_set_mark: Vec<ObjectSetMarkPattern>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapTerm {
    pub gap_index: u16,
    pub dx: i16,
    pub dy: i16,
}
