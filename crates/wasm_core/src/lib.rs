use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionId, ConditionValueKind, Effect, GapTerm,
    GlobalId, GlobalUpdateOp, Guard, InputId, LayerId, LocalFrame, LocalFrameExtent, MatchCell,
    ObjectDef, ObjectId, ObjectSetMatcher, ObjectSetScratchPattern, Offset, Pattern,
    PatternComponent, Rule, RuleApplication, RuleCondition, RuleId, RuleStep, ScratchId,
    ScratchPattern, ScratchValueMatch, State, TransitionCommand, WriteOp,
};
use serde_json::Value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmCompiledCoreRuntime {
    engine: CompiledEngine,
    current_state: Option<State>,
    saved_states: Vec<Option<State>>,
}

#[wasm_bindgen]
impl WasmCompiledCoreRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new(engine_json: &str) -> Result<Self, JsValue> {
        Ok(Self {
            engine: decode_engine(engine_json).map_err(|error| JsValue::from_str(&error))?,
            current_state: None,
            saved_states: Vec::new(),
        })
    }

    pub fn set_state(&mut self, state_json: &str) -> Result<(), JsValue> {
        self.current_state = Some(
            decode_state(&self.engine.game, state_json)
                .map_err(|error| JsValue::from_str(&error))?,
        );
        Ok(())
    }

    pub fn current_state(&self) -> Result<String, JsValue> {
        let state = self.current_state.as_ref().ok_or_else(|| {
            JsValue::from_str("core runtime current state has not been initialized")
        })?;
        Ok(encode_state(state))
    }

    pub fn current_state_hash(&self) -> Result<String, JsValue> {
        let state = self.current_state.as_ref().ok_or_else(|| {
            JsValue::from_str("core runtime current state has not been initialized")
        })?;
        Ok(state.hash().to_string())
    }

    pub fn save_current_state(&mut self) -> Result<u32, JsValue> {
        let state = self.current_state.as_ref().ok_or_else(|| {
            JsValue::from_str("core runtime current state has not been initialized")
        })?;
        Ok(save_state(&mut self.saved_states, state.clone()))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), JsValue> {
        self.current_state = Some(restore_state(&self.saved_states, handle)?.clone());
        Ok(())
    }

    pub fn transition_current_outcome(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, JsValue> {
        let state = self.current_state.as_ref().ok_or_else(|| {
            JsValue::from_str("core runtime current state has not been initialized")
        })?;
        let program = self
            .engine
            .program(program_key, level_index)
            .ok_or_else(|| JsValue::from_str(&format!("unknown program: {program_key}")))?;
        let before = state.clone();
        let outcome = puzzle_core::transition_program_outcome(
            &self.engine.game,
            program,
            state,
            InputId(input),
        )
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        let previous_state_handle = if program_key == "main" && before != outcome.next_state {
            Some(save_state(&mut self.saved_states, before.clone()))
        } else {
            None
        };
        self.current_state = Some(outcome.next_state.clone());
        Ok(encode_outcome(
            &outcome.next_state,
            Some(&before),
            previous_state_handle,
            outcome.cancelled,
            &outcome.commands,
            &outcome.fired_rules,
        ))
    }
}

pub struct CompiledEngine {
    game: CompiledGame,
    level_start_program: Vec<RuleStep>,
    level_clear_program: Vec<RuleStep>,
    display_level_start_program: Vec<RuleStep>,
    display_level_clear_program: Vec<RuleStep>,
    display_program: Vec<RuleStep>,
    level_start_programs: Vec<Vec<RuleStep>>,
    level_clear_programs: Vec<Vec<RuleStep>>,
}

impl CompiledEngine {
    pub fn game(&self) -> &CompiledGame {
        &self.game
    }

    pub fn program(&self, key: &str, level_index: i32) -> Option<&[RuleStep]> {
        match key {
            "main" | "run_rules_on_level_start" => Some(self.game.program()),
            "level_start" => Some(&self.level_start_program),
            "level_clear" => Some(&self.level_clear_program),
            "display_level_start" => Some(&self.display_level_start_program),
            "display_level_clear" => Some(&self.display_level_clear_program),
            "display" => Some(&self.display_program),
            "level_start_local" => level_program(&self.level_start_programs, level_index),
            "level_clear_local" => level_program(&self.level_clear_programs, level_index),
            _ => None,
        }
    }
}

fn level_program(programs: &[Vec<RuleStep>], level_index: i32) -> Option<&[RuleStep]> {
    let index = usize::try_from(level_index).ok()?;
    programs.get(index).map(Vec::as_slice)
}

fn save_state(states: &mut Vec<Option<State>>, state: State) -> u32 {
    if let Some(index) = states.iter().position(Option::is_none) {
        states[index] = Some(state);
        return index as u32;
    }
    states.push(Some(state));
    (states.len() - 1) as u32
}

fn restore_state(states: &[Option<State>], handle: u32) -> Result<&State, JsValue> {
    states
        .get(handle as usize)
        .and_then(Option::as_ref)
        .ok_or_else(|| JsValue::from_str(&format!("saved state handle {handle} does not exist")))
}

fn decode_engine(source: &str) -> Result<CompiledEngine, String> {
    let root: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
    if let Some(compiled) = root.get("compiledPlay") {
        return decode_compiled_play(compiled);
    }
    Err("compiled play export is missing compiledPlay".to_string())
}

pub fn decode_compiled_play(value: &Value) -> Result<CompiledEngine, String> {
    let model = string_field(value, "model")?;
    if model != "grid2" {
        return Err(format!("unsupported compiled play model: {model}"));
    }
    let data = value_array(
        object_field(value, "transition")?,
        "compiled play transition",
    )?;
    let layer_count = u16_at(data, 0, "transition layer count")?;
    let objects = array_at(data, 1, "transition objects")?
        .iter()
        .map(decode_compact_object)
        .collect::<Result<Vec<_>, _>>()?;
    let queries = array_at(data, 2, "transition queries")?
        .iter()
        .map(decode_compact_condition)
        .collect::<Result<Vec<_>, _>>()?;
    let visual_objects = array_at(data, 3, "transition visual objects")?
        .iter()
        .map(|item| Ok(ObjectId(u16_value(item, "visual object")?)))
        .collect::<Result<Vec<_>, String>>()?;
    let programs = array_at(data, 4, "transition programs")?;
    let program = decode_compact_program(value_at(programs, 0, "main program")?)?;
    let game = CompiledGame::new_with_scratch_condition_defs_program_roles(
        layer_count,
        objects,
        Vec::new(),
        queries,
        program,
        visual_objects,
        Vec::new(),
    );
    let level_programs = array_at(data, 5, "transition level programs")?;
    let mut level_start_programs = Vec::with_capacity(level_programs.len());
    let mut level_clear_programs = Vec::with_capacity(level_programs.len());
    for (index, entry) in level_programs.iter().enumerate() {
        let entry = value_array(entry, &format!("level program {index}"))?;
        level_start_programs.push(decode_compact_program(value_at(
            entry,
            0,
            "level start local program",
        )?)?);
        level_clear_programs.push(decode_compact_program(value_at(
            entry,
            1,
            "level clear local program",
        )?)?);
    }
    Ok(CompiledEngine {
        game,
        level_start_program: decode_compact_program(value_at(programs, 1, "level start program")?)?,
        level_clear_program: decode_compact_program(value_at(programs, 2, "level clear program")?)?,
        display_level_start_program: decode_compact_program(value_at(
            programs,
            3,
            "display level start program",
        )?)?,
        display_level_clear_program: decode_compact_program(value_at(
            programs,
            4,
            "display level clear program",
        )?)?,
        display_program: decode_compact_program(value_at(programs, 5, "display program")?)?,
        level_start_programs,
        level_clear_programs,
    })
}

fn decode_compact_object(value: &Value) -> Result<ObjectDef, String> {
    let items = value_array(value, "compact object")?;
    Ok(ObjectDef {
        id: ObjectId(u16_at(items, 0, "object id")?),
        layer_id: LayerId(u16_at(items, 1, "object layer")?),
    })
}

fn decode_compact_condition(value: &Value) -> Result<ConditionDef, String> {
    let items = value_array(value, "compact condition")?;
    Ok(ConditionDef {
        id: ConditionId(u16_at(items, 0, "condition id")?),
        kind: decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
    })
}

fn decode_compact_program(value: &Value) -> Result<Vec<RuleStep>, String> {
    value_array(value, "compact program")?
        .iter()
        .map(decode_compact_rule_step)
        .collect()
}

fn decode_compact_rule_step(value: &Value) -> Result<RuleStep, String> {
    let items = value_array(value, "compact rule step")?;
    match tag_at(items, 0, "rule step tag")? {
        0 => Ok(RuleStep::Rule(decode_compact_rule(value_at(
            items, 1, "rule",
        )?)?)),
        1 => Ok(RuleStep::ConditionalBlock {
            condition: decode_compact_rule_condition(value_at(items, 1, "condition")?)?,
            steps: decode_compact_program(value_at(items, 2, "steps")?)?,
        }),
        2 => Ok(RuleStep::Block {
            application: decode_compact_application(u16_at(items, 1, "application")?)?,
            stop_condition: match value_at(items, 2, "condition")? {
                Value::Null => None,
                condition => Some(decode_compact_rule_condition(condition)?),
            },
            steps: decode_compact_program(value_at(items, 3, "steps")?)?,
        }),
        3 => Ok(RuleStep::LocalFrame {
            frame: decode_compact_local_frame(value_at(items, 1, "local frame")?)?,
            steps: decode_compact_program(value_at(items, 2, "steps")?)?,
        }),
        4 => Ok(RuleStep::AfterTriggered {
            steps: decode_compact_program(value_at(items, 1, "steps")?)?,
            then_steps: decode_compact_program(value_at(items, 2, "then steps")?)?,
        }),
        5 => Ok(RuleStep::ConditionalBranch {
            condition: decode_compact_rule_condition(value_at(items, 1, "condition")?)?,
            then_steps: decode_compact_program(value_at(items, 2, "then steps")?)?,
            else_steps: decode_compact_program(value_at(items, 3, "else steps")?)?,
        }),
        tag => Err(format!("unknown compact rule step tag: {tag}")),
    }
}

fn decode_compact_rule(value: &Value) -> Result<Rule, String> {
    let items = value_array(value, "compact rule")?;
    Ok(Rule {
        id: RuleId(u16_at(items, 0, "rule id")?),
        application: decode_compact_application(u16_at(items, 1, "application")?)?,
        guards: array_at(items, 2, "guards")?
            .iter()
            .map(decode_compact_guard)
            .collect::<Result<Vec<_>, _>>()?,
        pattern: decode_compact_pattern(value_at(items, 3, "pattern")?)?,
        writes: array_at(items, 4, "writes")?
            .iter()
            .map(decode_compact_write)
            .collect::<Result<Vec<_>, _>>()?,
        effects: array_at(items, 5, "effects")?
            .iter()
            .map(decode_compact_effect)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn decode_compact_application(value: u16) -> Result<RuleApplication, String> {
    match value {
        0 => Ok(RuleApplication::Once),
        1 => Ok(RuleApplication::OnceAll),
        2 => Ok(RuleApplication::OncePerLevel),
        3 => Ok(RuleApplication::UntilStable),
        4 => Ok(RuleApplication::Random),
        other => Err(format!("unknown compact rule application: {other}")),
    }
}

fn decode_compact_rule_condition(value: &Value) -> Result<RuleCondition, String> {
    let items = value_array(value, "compact rule condition")?;
    match tag_at(items, 0, "condition tag")? {
        0 => Ok(RuleCondition::AnyMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        1 => Ok(RuleCondition::NoMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        2 => Ok(RuleCondition::AnyInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        3 => Ok(RuleCondition::NoInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        4 => Ok(RuleCondition::GuardBranches(
            value_array(value_at(items, 1, "guard branches")?, "guard branches")?
                .iter()
                .map(|branch| {
                    value_array(branch, "guard branch")?
                        .iter()
                        .map(decode_compact_guard)
                        .collect()
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        tag => Err(format!("unknown compact condition tag: {tag}")),
    }
}

fn decode_compact_guard(value: &Value) -> Result<Guard, String> {
    let items = value_array(value, "compact guard")?;
    match tag_at(items, 0, "guard tag")? {
        0 => Ok(Guard::InputIs(InputId(u16_at(items, 1, "input")?))),
        1 => Ok(Guard::GlobalCompare {
            global: GlobalId(u16_at(items, 1, "global")?),
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        2 => Ok(Guard::ConditionCompare {
            condition: ConditionId(u16_at(items, 1, "condition")?),
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        3 => Ok(Guard::ConditionNonZero(ConditionId(u16_at(
            items,
            1,
            "condition",
        )?))),
        4 => Ok(Guard::InlineConditionCompare {
            kind: decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
            op: decode_compact_comparison(u16_at(items, 2, "comparison")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        5 => Ok(Guard::InlineConditionNonZero(
            decode_compact_condition_value_kind(value_at(items, 1, "condition kind")?)?,
        )),
        tag => Err(format!("unknown compact guard tag: {tag}")),
    }
}

fn decode_compact_condition_value_kind(value: &Value) -> Result<ConditionValueKind, String> {
    let items = value_array(value, "compact condition kind")?;
    match tag_at(items, 0, "condition kind tag")? {
        0 => Ok(ConditionValueKind::CountObjects(decode_compact_object_ids(
            value_at(items, 1, "objects")?,
        )?)),
        1 => Ok(ConditionValueKind::ExistsObjects(
            decode_compact_object_ids(value_at(items, 1, "objects")?)?,
        )),
        2 => Ok(ConditionValueKind::NoneObjects(decode_compact_object_ids(
            value_at(items, 1, "objects")?,
        )?)),
        3 => Ok(ConditionValueKind::CountMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        4 => Ok(ConditionValueKind::ExistsMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        5 => Ok(ConditionValueKind::NoneMatches(decode_compact_patterns(
            value_at(items, 1, "patterns")?,
        )?)),
        6 => Ok(ConditionValueKind::CountInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        7 => Ok(ConditionValueKind::ExistsInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        8 => Ok(ConditionValueKind::NoneInputMatches(
            decode_compact_input_patterns(value_at(items, 1, "input patterns")?)?,
        )),
        tag => Err(format!("unknown compact condition kind tag: {tag}")),
    }
}

fn decode_compact_patterns(value: &Value) -> Result<Vec<Pattern>, String> {
    value_array(value, "compact patterns")?
        .iter()
        .map(decode_compact_pattern)
        .collect()
}

fn decode_compact_input_patterns(value: &Value) -> Result<Vec<(InputId, Pattern)>, String> {
    value_array(value, "compact input patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "input pattern")?;
            Ok((
                InputId(u16_at(entry, 0, "input")?),
                decode_compact_pattern(value_at(entry, 1, "pattern")?)?,
            ))
        })
        .collect()
}

fn decode_compact_pattern(value: &Value) -> Result<Pattern, String> {
    Ok(Pattern {
        components: value_array(value, "compact pattern")?
            .iter()
            .map(|component| {
                let component = value_array(component, "pattern component")?;
                Ok(PatternComponent {
                    gap_count: u16_at(component, 0, "gap count")?,
                    cells: value_array(value_at(component, 1, "cells")?, "cells")?
                        .iter()
                        .map(decode_compact_match_cell)
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn decode_compact_match_cell(value: &Value) -> Result<MatchCell, String> {
    let items = value_array(value, "match cell")?;
    if items.len() != 9 {
        return Err(format!(
            "match cell must have 9 fields, got {}",
            items.len()
        ));
    }
    Ok(MatchCell {
        offset: decode_compact_offset(value_at(items, 0, "offset")?)?,
        require_objects: decode_compact_object_ids(value_at(items, 1, "require objects")?)?,
        require_object_sets: decode_compact_object_sets(value_at(
            items,
            2,
            "require object sets",
        )?)?,
        forbid_objects: decode_compact_object_ids(value_at(items, 3, "forbid objects")?)?,
        require_scratch: decode_compact_scratch_patterns(value_at(items, 4, "require scratch")?)?,
        require_object_set_scratch: decode_compact_object_set_scratch_patterns(value_at(
            items,
            5,
            "require object set scratch",
        )?)?,
        forbid_scratch: decode_compact_scratch_patterns(value_at(items, 6, "forbid scratch")?)?,
        forbid_object_set_scratch: decode_compact_object_set_scratch_patterns(value_at(
            items,
            7,
            "forbid object set scratch",
        )?)?,
        require_null: match u16_at(items, 8, "require null")? {
            0 => false,
            1 => true,
            other => return Err(format!("unknown compact require null value: {other}")),
        },
    })
}

fn decode_compact_offset(value: &Value) -> Result<Offset, String> {
    let items = value_array(value, "offset")?;
    match tag_at(items, 0, "offset tag")? {
        0 => Ok(Offset::Fixed {
            dx: i16_at(items, 1, "dx")?,
            dy: i16_at(items, 2, "dy")?,
        }),
        1 => Ok(Offset::Variable {
            base_dx: i16_at(items, 1, "base dx")?,
            base_dy: i16_at(items, 2, "base dy")?,
            gap_terms: value_array(value_at(items, 3, "gap terms")?, "gap terms")?
                .iter()
                .map(|term| {
                    let term = value_array(term, "gap term")?;
                    Ok(GapTerm {
                        gap_index: u16_at(term, 0, "gap index")?,
                        dx: i16_at(term, 1, "dx")?,
                        dy: i16_at(term, 2, "dy")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        }),
        tag => Err(format!("unknown compact offset tag: {tag}")),
    }
}

fn decode_compact_write(value: &Value) -> Result<WriteOp, String> {
    let items = value_array(value, "write")?;
    match tag_at(items, 0, "write tag")? {
        0 => Ok(WriteOp::Add {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
        }),
        1 => Ok(WriteOp::Remove {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
        }),
        2 => Ok(WriteOp::Move {
            component: u16_at(items, 1, "component")?,
            from_offset: decode_compact_offset(value_at(items, 2, "from offset")?)?,
            to_offset: decode_compact_offset(value_at(items, 3, "to offset")?)?,
            object: ObjectId(u16_at(items, 4, "object")?),
        }),
        3 => Ok(WriteOp::Replace {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            remove: ObjectId(u16_at(items, 3, "remove")?),
            add: ObjectId(u16_at(items, 4, "add")?),
        }),
        4 => Ok(WriteOp::SetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
        }),
        5 => Ok(WriteOp::RemoveScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            object: ObjectId(u16_at(items, 3, "object")?),
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
            match_value: decode_compact_scratch_match(u16_at(items, 6, "scratch match")?)?,
        }),
        6 => Ok(WriteOp::AddObjectSet {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
        }),
        7 => Ok(WriteOp::RemoveObjectSet {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
        }),
        8 => Ok(WriteOp::MoveObjectSet {
            component: u16_at(items, 1, "component")?,
            from_offset: decode_compact_offset(value_at(items, 2, "from offset")?)?,
            to_offset: decode_compact_offset(value_at(items, 3, "to offset")?)?,
            binding: u16_at(items, 4, "binding")?,
        }),
        9 => Ok(WriteOp::SetObjectSetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
        }),
        10 => Ok(WriteOp::RemoveObjectSetScratch {
            component: u16_at(items, 1, "component")?,
            offset: decode_compact_offset(value_at(items, 2, "offset")?)?,
            binding: u16_at(items, 3, "binding")?,
            scratch: ScratchId(u16_at(items, 4, "scratch")?),
            value: optional_i64_at(items, 5, "scratch value")?,
            match_value: decode_compact_scratch_match(u16_at(items, 6, "scratch match")?)?,
        }),
        tag => Err(format!("unknown compact write tag: {tag}")),
    }
}

fn decode_compact_effect(value: &Value) -> Result<Effect, String> {
    let items = value_array(value, "effect")?;
    match tag_at(items, 0, "effect tag")? {
        0 => Ok(Effect::Cancel),
        1 => Ok(Effect::Win),
        2 => Ok(Effect::Restart),
        3 => Ok(Effect::NextLevel),
        4 => Ok(Effect::Again),
        5 => Ok(Effect::Checkpoint),
        6 => Ok(Effect::ClearCheckpoint),
        7 => Ok(Effect::UpdateGlobal {
            global: GlobalId(u16_at(items, 1, "global")?),
            op: decode_compact_global_update(u16_at(items, 2, "global update")?)?,
            value: i64_at(items, 3, "value")?,
        }),
        tag => Err(format!("unknown compact effect tag: {tag}")),
    }
}

fn decode_compact_scratch_patterns(value: &Value) -> Result<Vec<ScratchPattern>, String> {
    value_array(value, "scratch patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "scratch pattern")?;
            Ok(ScratchPattern {
                object: ObjectId(u16_at(entry, 0, "object")?),
                scratch: ScratchId(u16_at(entry, 1, "scratch")?),
                value: optional_i64_at(entry, 2, "value")?,
                match_value: decode_compact_scratch_match(u16_at(entry, 3, "scratch match")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_sets(value: &Value) -> Result<Vec<ObjectSetMatcher>, String> {
    value_array(value, "object sets")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "object set")?;
            Ok(ObjectSetMatcher {
                binding: u16_at(entry, 0, "binding")?,
                layer: LayerId(u16_at(entry, 1, "layer")?),
                objects: decode_compact_object_ids(value_at(entry, 2, "objects")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_set_scratch_patterns(
    value: &Value,
) -> Result<Vec<ObjectSetScratchPattern>, String> {
    value_array(value, "object set scratch patterns")?
        .iter()
        .map(|entry| {
            let entry = value_array(entry, "object set scratch pattern")?;
            Ok(ObjectSetScratchPattern {
                binding: u16_at(entry, 0, "binding")?,
                scratch: ScratchId(u16_at(entry, 1, "scratch")?),
                value: optional_i64_at(entry, 2, "value")?,
                match_value: decode_compact_scratch_match(u16_at(entry, 3, "scratch match")?)?,
            })
        })
        .collect()
}

fn decode_compact_object_ids(value: &Value) -> Result<Vec<ObjectId>, String> {
    value_array(value, "object ids")?
        .iter()
        .map(|item| Ok(ObjectId(u16_value(item, "object id")?)))
        .collect()
}

fn decode_compact_local_frame(value: &Value) -> Result<LocalFrame<ObjectId>, String> {
    let items = value_array(value, "local frame")?;
    Ok(LocalFrame {
        x: decode_compact_local_frame_extent(value_at(items, 0, "frame x")?)?,
        y: decode_compact_local_frame_extent(value_at(items, 1, "frame y")?)?,
        z: decode_compact_local_frame_extent(value_at(items, 2, "frame z")?)?,
        focus_objects: decode_compact_object_ids(value_at(items, 3, "focus objects")?)?,
    })
}

fn decode_compact_local_frame_extent(value: &Value) -> Result<LocalFrameExtent, String> {
    if value.is_null() {
        return Ok(LocalFrameExtent::Full);
    }
    Ok(LocalFrameExtent::Radius(u16_value(value, "frame extent")?))
}

fn decode_compact_comparison(value: u16) -> Result<ComparisonOp, String> {
    match value {
        0 => Ok(ComparisonOp::Eq),
        1 => Ok(ComparisonOp::NotEq),
        2 => Ok(ComparisonOp::Greater),
        3 => Ok(ComparisonOp::GreaterEq),
        4 => Ok(ComparisonOp::Less),
        5 => Ok(ComparisonOp::LessEq),
        other => Err(format!("unknown compact comparison op: {other}")),
    }
}

fn decode_compact_global_update(value: u16) -> Result<GlobalUpdateOp, String> {
    match value {
        0 => Ok(GlobalUpdateOp::Set),
        1 => Ok(GlobalUpdateOp::Add),
        2 => Ok(GlobalUpdateOp::Subtract),
        3 => Ok(GlobalUpdateOp::Multiply),
        4 => Ok(GlobalUpdateOp::Divide),
        5 => Ok(GlobalUpdateOp::Remainder),
        other => Err(format!("unknown compact global update op: {other}")),
    }
}

fn decode_compact_scratch_match(value: u16) -> Result<ScratchValueMatch, String> {
    match value {
        0 => Ok(ScratchValueMatch::Any),
        1 => Ok(ScratchValueMatch::Exact),
        other => Err(format!("unknown compact scratch match: {other}")),
    }
}

pub fn decode_state(game: &CompiledGame, source: &str) -> Result<State, String> {
    let value: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let width = u16_field(&value, "width")?;
    let height = u16_field(&value, "height")?;
    let layer_count = u16_field(&value, "layerCount")?;
    let globals = value
        .get("globals")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_i64()
                        .ok_or_else(|| "global must be an integer".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut state =
        State::empty_with_globals(width, height, layer_count, game.object_count(), globals)
            .map_err(|error| format!("{error:?}"))?;
    for (index, item) in array_field(&value, "slots")?.iter().enumerate() {
        let object = ObjectId(u16_value(item, "slot")?);
        if object.is_empty() {
            continue;
        }
        let cell = index / usize::from(layer_count);
        let x = u16::try_from(cell % usize::from(width)).map_err(|_| "x out of range")?;
        let y = u16::try_from(cell / usize::from(width)).map_err(|_| "y out of range")?;
        state
            .place_object(game, x, y, object)
            .map_err(|error| format!("{error:?}"))?;
    }
    if let Some(rules) = value.get("levelFiredRules").and_then(Value::as_array) {
        for rule in rules {
            state.mark_level_rule_fired(RuleId(u16_value(rule, "levelFiredRules")?));
        }
    }
    Ok(state)
}

fn encode_state(state: &State) -> String {
    let mut out = String::new();
    out.push('{');
    number(&mut out, "width", state.width as u64);
    out.push(',');
    number(&mut out, "height", state.height as u64);
    out.push(',');
    number(&mut out, "layerCount", state.layer_count as u64);
    out.push_str(",\"slots\":[");
    for (index, object) in state.slots().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push_str("],\"scratch\":[],\"globals\":[");
    for (index, value) in state.visible_globals().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push_str("],\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push_str("]}");
    out
}

fn encode_outcome(
    state: &State,
    before: Option<&State>,
    previous_state_handle: Option<u32>,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
) -> String {
    let mut out = String::new();
    out.push('{');
    bool_field(
        &mut out,
        "changed",
        before.is_some_and(|before| before != state),
    );
    out.push(',');
    bool_field(&mut out, "cancelled", cancelled);
    out.push(',');
    out.push_str("\"state\":");
    out.push_str(&encode_state(state));
    out.push(',');
    number(&mut out, "stateHash", state.hash());
    out.push_str(",\"stateHashKey\":");
    string(&mut out, &state.hash().to_string());
    out.push_str(",\"changedCells\":");
    encode_changed_cells(&mut out, state, before);
    out.push_str(",\"globals\":[");
    for (index, value) in state.visible_globals().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push_str("],\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push_str("],\"firedRules\":[");
    for (index, rule) in fired_rules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push_str("],\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        encode_command(&mut out, command);
    }
    out.push_str("],\"animationEvents\":[]");
    if let Some(handle) = previous_state_handle {
        out.push(',');
        number(&mut out, "previousStateHandle", handle as u64);
    }
    out.push('}');
    out
}

fn encode_changed_cells(out: &mut String, state: &State, before: Option<&State>) {
    out.push('[');
    let mut first = true;
    for y in 0..state.height {
        for x in 0..state.width {
            let cell = usize::from(y) * usize::from(state.width) + usize::from(x);
            let start = cell * usize::from(state.layer_count);
            let end = start + usize::from(state.layer_count);
            if before.is_some_and(|before| before.slots()[start..end] == state.slots()[start..end])
            {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            out.push('{');
            number(out, "x", x as u64);
            out.push(',');
            number(out, "y", y as u64);
            out.push_str(",\"objects\":[");
            let mut first_object = true;
            for object in &state.slots()[start..end] {
                if object.is_empty() {
                    continue;
                }
                if !first_object {
                    out.push(',');
                }
                first_object = false;
                out.push_str(&object.0.to_string());
            }
            out.push_str("]}");
        }
    }
    out.push(']');
}

fn encode_command(out: &mut String, command: &TransitionCommand) {
    out.push('{');
    let kind = match command {
        TransitionCommand::Win => "win",
        TransitionCommand::Restart => "restart",
        TransitionCommand::NextLevel => "next_level",
        TransitionCommand::Again => "again",
        TransitionCommand::Checkpoint => "checkpoint",
        TransitionCommand::ClearCheckpoint => "clear_checkpoint",
    };
    json_string_field(out, "kind", kind);
    out.push('}');
}

fn object_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn value_array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{name} must be an array"))
}

fn value_at<'a>(items: &'a [Value], index: usize, name: &str) -> Result<&'a Value, String> {
    items
        .get(index)
        .ok_or_else(|| format!("missing {name} at index {index}"))
}

fn array_at<'a>(items: &'a [Value], index: usize, name: &str) -> Result<&'a [Value], String> {
    value_array(value_at(items, index, name)?, name)
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn optional_i64_at(items: &[Value], index: usize, name: &str) -> Result<Option<i64>, String> {
    let value = value_at(items, index, name)?;
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{name} must be an integer or null"))
}

fn tag_at(items: &[Value], index: usize, name: &str) -> Result<u16, String> {
    u16_at(items, index, name)
}

fn u16_at(items: &[Value], index: usize, name: &str) -> Result<u16, String> {
    u16_value(value_at(items, index, name)?, name)
}

fn i16_at(items: &[Value], index: usize, name: &str) -> Result<i16, String> {
    let raw = i64_at(items, index, name)?;
    i16::try_from(raw).map_err(|_| format!("{name} out of range"))
}

fn i64_at(items: &[Value], index: usize, name: &str) -> Result<i64, String> {
    value_at(items, index, name)?
        .as_i64()
        .ok_or_else(|| format!("{name} must be an integer"))
}

fn u16_field(value: &Value, key: &str) -> Result<u16, String> {
    u16_value(
        value
            .get(key)
            .ok_or_else(|| format!("missing field: {key}"))?,
        key,
    )
}

fn u16_value(value: &Value, name: &str) -> Result<u16, String> {
    let raw = value
        .as_u64()
        .ok_or_else(|| format!("{name} must be an unsigned integer"))?;
    u16::try_from(raw).map_err(|_| format!("{name} out of range"))
}

fn number(out: &mut String, key: &str, value: u64) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn bool_field(out: &mut String, key: &str, value: bool) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(if value { "true" } else { "false" });
}

fn json_string_field(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    string(out, value);
}

fn string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_outcome_includes_state_payload() {
        let state = State::empty_with_globals(2, 1, 1, 2, vec![7]).expect("state");
        let outcome = encode_outcome(&state, None, None, false, &[], &[]);
        let parsed: Value = serde_json::from_str(&outcome).expect("outcome json");

        assert_eq!(parsed["state"]["width"], 2);
        assert_eq!(parsed["state"]["height"], 1);
        assert_eq!(parsed["state"]["layerCount"], 1);
        assert_eq!(parsed["state"]["slots"].as_array().expect("slots").len(), 2);
        assert_eq!(parsed["state"]["globals"][0], 7);
    }
}
