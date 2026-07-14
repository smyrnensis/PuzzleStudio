pub struct CoreRuntimeBridge {
    loaded: LoadedGame,
    current_state: Option<State>,
    saved_states: SavedStateStore<State>,
}

impl CoreRuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        Ok(Self {
            loaded: parse_game(source).map_err(|error| error.to_string())?,
            current_state: None,
            saved_states: SavedStateStore::new(),
        })
    }

    pub fn transition_program_outcome_json(
        &self,
        program_key: &str,
        level_index: i32,
        state_json: &str,
        input: u16,
    ) -> Result<String, String> {
        transition_program_outcome_json_inner(
            &self.loaded,
            program_key,
            level_index,
            state_json,
            InputId(input),
        )
        .map_err(|error| error.to_string())
    }

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        let state = state_from_json(&self.loaded, state_json).map_err(|error| error.to_string())?;
        self.current_state = Some(state);
        Ok(())
    }

    pub fn current_state_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state_data(&mut out, state);
        Ok(out)
    }

    pub fn current_state_hash_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        Ok(state.hash().to_string())
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state2_cells(&mut out, state, None);
        Ok(out)
    }

    pub fn save_current_state(&mut self) -> Result<u32, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        Ok(self.saved_states.save(state.clone()))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), String> {
        self.current_state = Some(self.saved_states.restore(handle)?.clone());
        Ok(())
    }

    pub fn transition_current_outcome_json(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, String> {
        self.transition_current_outcome_json_inner(program_key, level_index, input, false)
    }

    pub fn transition_current_state_outcome_json(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, String> {
        self.transition_current_outcome_json_inner(program_key, level_index, input, true)
    }

    fn transition_current_outcome_json_inner(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
        include_state: bool,
    ) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "2D runtime current state has not been initialized".to_string())?;
        let program = selected_rule_program(&self.loaded, program_key, level_index)
            .map_err(|error| error.to_string())?;
        let outcome = transition_program_trace(&self.loaded.game, program, state, InputId(input))
            .map_err(|error| format!("{error:?}"))?;
        let before = state.clone();
        let previous_state_handle = if program_key == "main" && before != outcome.next_state {
            Some(self.saved_states.save(before.clone()))
        } else {
            None
        };
        self.current_state = Some(outcome.next_state.clone());
        runtime_transition_current_outcome_json(
            &self.loaded,
            &outcome.next_state,
            Some(&before),
            previous_state_handle,
            outcome.cancelled,
            &outcome.commands,
            &outcome.fired_rules,
            &outcome.patches,
            include_state,
        )
        .map_err(|error| error.to_string())
    }
}

pub struct Puzzle3RuntimeBridge {
    inner: puzzle_game_runtime::Puzzle3RuntimeBridge,
}

impl Puzzle3RuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        Ok(Self {
            inner: puzzle_game_runtime::Puzzle3RuntimeBridge::from_source(source)?,
        })
    }

    pub fn transition_program_outcome_json(
        &self,
        program_key: &str,
        state_json: &str,
        input: u16,
    ) -> Result<String, String> {
        self.inner
            .transition_program_outcome_json(program_key, state_json, input)
    }

    pub fn is_complete_json(&self, state_json: &str) -> Result<bool, String> {
        self.inner.is_complete_json(state_json)
    }

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        self.inner.set_state_json(state_json)
    }

    pub fn current_state_json(&self) -> Result<String, String> {
        self.inner.current_state_json()
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        self.inner.current_cells_json()
    }

    pub fn save_current_state(&mut self) -> Result<u32, String> {
        self.inner.save_current_state()
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), String> {
        self.inner.restore_saved_state(handle)
    }

    pub fn transition_current_outcome_json(
        &mut self,
        program_key: &str,
        input: u16,
    ) -> Result<String, String> {
        self.inner
            .transition_current_outcome_json(program_key, input)
    }

    pub fn is_current_complete(&self) -> Result<bool, String> {
        self.inner.is_current_complete()
    }
}

struct SavedStateStore<T> {
    states: Vec<Option<T>>,
}

impl<T> SavedStateStore<T> {
    fn new() -> Self {
        Self { states: Vec::new() }
    }

    fn save(&mut self, state: T) -> u32 {
        if let Some(index) = self.states.iter().position(Option::is_none) {
            self.states[index] = Some(state);
            return index as u32;
        }
        self.states.push(Some(state));
        (self.states.len() - 1) as u32
    }

    fn restore(&self, handle: u32) -> Result<&T, String> {
        self.states
            .get(handle as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| format!("saved state handle {handle} does not exist"))
    }
}

pub fn transition_program_outcome_json_from_source(
    source: &str,
    program_key: &str,
    level_index: i32,
    state_json: &str,
    input: u16,
) -> Result<String, String> {
    let loaded = parse_game(source).map_err(|error| error.to_string())?;
    transition_program_outcome_json_inner(
        &loaded,
        program_key,
        level_index,
        state_json,
        InputId(input),
    )
    .map_err(|error| error.to_string())
}

fn transition_program_outcome_json_inner(
    loaded: &LoadedGame,
    program_key: &str,
    level_index: i32,
    state_json: &str,
    input: InputId,
) -> Result<String, AppError> {
    let state = state_from_json(loaded, state_json)?;
    let program = selected_rule_program(loaded, program_key, level_index)?;
    let outcome = transition_program_trace(&loaded.game, program, &state, input)?;
    runtime_transition_program_outcome_json(
        loaded,
        &outcome.next_state,
        outcome.cancelled,
        &outcome.commands,
        &outcome.fired_rules,
        &outcome.patches,
    )
}

fn selected_rule_program<'a>(
    loaded: &'a LoadedGame,
    program_key: &str,
    level_index: i32,
) -> Result<&'a [RuleStep], AppError> {
    match program_key {
        "main" | "run_rules_on_level_start" => {
            if level_index < 0 {
                return Ok(loaded.game.program());
            }
            let index = usize::try_from(level_index)
                .map_err(|_| AppError::Config("main program requires a level index".to_string()))?;
            loaded.program_for_level(index).ok_or_else(|| {
                AppError::Config(format!("main program level index out of range: {index}"))
            })
        }
        "level_start" => Ok(loaded.level_start_program.as_deref().unwrap_or(&[])),
        "level_clear" => Ok(loaded.level_clear_program.as_deref().unwrap_or(&[])),
        "level_start_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_start_local requires a level index".to_string())
            })?;
            Ok(loaded
                .levels
                .get(index)
                .and_then(|level| level.level_start_program.as_deref())
                .unwrap_or(&[]))
        }
        "level_clear_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_clear_local requires a level index".to_string())
            })?;
            Ok(loaded
                .levels
                .get(index)
                .and_then(|level| level.level_clear_program.as_deref())
                .unwrap_or(&[]))
        }
        other => Err(AppError::Config(format!(
            "unknown transition program selector: {other}"
        ))),
    }
}

fn runtime_transition_program_outcome_json(
    loaded: &LoadedGame,
    state: &State,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
) -> Result<String, AppError> {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
    RuntimeTransitionProgramOutcome {
        state: state_contract_2d(state),
        cancelled,
        completed: loaded.is_goal_complete(state),
        commands: transition_commands_contract(commands),
        fired_rules: fired_rules.iter().map(|rule| rule.0).collect(),
        patches: patches_contract_2d(patches),
        animation_events: animation_events_contract_2d(loaded, &animation_events),
    }
    .to_json_string()
    .map_err(|error| AppError::Config(error.to_string()))
}

fn runtime_transition_current_outcome_json(
    loaded: &LoadedGame,
    state: &State,
    before: Option<&State>,
    previous_state_handle: Option<u32>,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
    include_state: bool,
) -> Result<String, AppError> {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
    RuntimeTransitionCurrentOutcome {
        cancelled,
        changed: before.is_some_and(|before| before != state),
        completed: loaded.is_goal_complete(state),
        state: if include_state {
            Some(state_contract_2d(state))
        } else {
            None
        },
        commands: transition_commands_contract(commands),
        fired_rules: fired_rules.iter().map(|rule| rule.0).collect(),
        patches: patches_contract_2d(patches),
        animation_events: animation_events_contract_2d(loaded, &animation_events),
        state_hash: state.hash(),
        state_hash_key: state.hash().to_string(),
        previous_state_handle,
        changed_cells: changed_cells_contract_2d(state, before),
        variables: state.visible_variables().to_vec(),
        level_fired_rules: state
            .level_fired_rules()
            .iter()
            .map(|rule| rule.0)
            .collect(),
    }
    .to_json_string()
    .map_err(|error| AppError::Config(error.to_string()))
}

fn transition_commands_contract(commands: &[TransitionCommand]) -> Vec<RuntimeTransitionCommand> {
    commands
        .iter()
        .map(|command| match command {
            TransitionCommand::Win => RuntimeTransitionCommand::Win,
            TransitionCommand::Restart => RuntimeTransitionCommand::Restart,
            TransitionCommand::NextLevel => RuntimeTransitionCommand::NextLevel,
            TransitionCommand::Again => RuntimeTransitionCommand::Again,
            TransitionCommand::Checkpoint => RuntimeTransitionCommand::Checkpoint,
            TransitionCommand::ClearCheckpoint => RuntimeTransitionCommand::ClearCheckpoint,
        })
        .collect()
}

fn state_contract_2d(state: &State) -> RuntimeStateSnapshot {
    RuntimeStateSnapshot::TwoD(RuntimeStateSnapshot2d {
        kind: RuntimeModelKind::TwoD,
        width: state.width,
        height: state.height,
        layer_count: state.layer_count,
        slots: state.slots().iter().map(|object| object.0).collect(),
        slot_marks: (0..state.slots().len())
            .map(|index| {
                state
                    .slot_mark_at(index)
                    .map(|mark| RuntimeMarkValue {
                        mark: mark.mark.0,
                        value: mark.value,
                    })
                    .collect()
            })
            .collect(),
        variables: state.visible_variables().to_vec(),
        level_fired_rules: state
            .level_fired_rules()
            .iter()
            .map(|rule| rule.0)
            .collect(),
    })
}

fn changed_cells_contract_2d(state: &State, before: Option<&State>) -> Vec<RuntimeChangedCell> {
    let mut cells = Vec::new();
    for y in 0..state.height {
        for x in 0..state.width {
            let cell = usize::from(y) * usize::from(state.width) + usize::from(x);
            if before.is_some_and(|before| state2_cell_slots_equal(before, state, cell)) {
                continue;
            }
            let mut objects = Vec::new();
            for layer in 0..state.layer_count {
                let slot = (cell * usize::from(state.layer_count)) + usize::from(layer);
                let object = state.slots()[slot];
                if !object.is_empty() {
                    objects.push(object.0);
                }
            }
            if before.is_none() && objects.is_empty() {
                continue;
            }
            cells.push(RuntimeChangedCell {
                position: RuntimeCoord { x, y, z: None },
                objects,
            });
        }
    }
    cells
}

fn patches_contract_2d(patches: &[Patch]) -> Vec<Vec<RuntimePatchOp>> {
    patches
        .iter()
        .map(|patch| patch.ops().iter().map(patch_op_contract_2d).collect())
        .collect()
}

fn patch_op_contract_2d(op: &PatchOp) -> RuntimePatchOp {
    match *op {
        PatchOp::Add { position, object } => RuntimePatchOp::Add {
            position: runtime_coord2(position),
            object_id: object.0,
        },
        PatchOp::Remove { position, object } => RuntimePatchOp::Remove {
            position: runtime_coord2(position),
            object_id: object.0,
        },
        PatchOp::Move { from, to, object } => RuntimePatchOp::Move {
            from: runtime_coord2(from),
            to: runtime_coord2(to),
            object_id: object.0,
        },
        PatchOp::Replace {
            position,
            remove,
            add,
        } => RuntimePatchOp::Replace {
            position: runtime_coord2(position),
            remove: remove.0,
            add: add.0,
        },
        PatchOp::UpdateVariable { variable, .. } => RuntimePatchOp::UpdateVariable {
            variable: variable.0,
        },
        PatchOp::SetMark {
            position,
            object,
            mark,
            ..
        } => RuntimePatchOp::SetMark {
            position: runtime_coord2(position),
            object_id: object.0,
            mark: mark.0,
        },
        PatchOp::RemoveMark {
            position,
            object,
            mark,
            match_value,
            ..
        } => RuntimePatchOp::RemoveMark {
            position: runtime_coord2(position),
            object_id: object.0,
            mark: mark.0,
            match_value: runtime_mark_value_match(match_value),
        },
    }
}

fn runtime_coord2(position: puzzle_core::GridCoord<2>) -> RuntimeCoord {
    let [x, y] = position.axes();
    RuntimeCoord { x, y, z: None }
}

fn runtime_mark_value_match(match_value: MarkValueMatch) -> RuntimeMarkValueMatch {
    match match_value {
        MarkValueMatch::Any => RuntimeMarkValueMatch::Any,
        MarkValueMatch::Exact => RuntimeMarkValueMatch::Exact,
    }
}

fn source_looks_puzzle3d(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
        matches!(tokens.as_slice(), ["puzzle3", ..] | ["sprites3", ..])
    })
}

#[cfg(feature = "solver")]
pub fn solve_request_json(request_json: &str) -> Result<String, String> {
    solve_request_json_inner(request_json).map_err(|error| error.to_string())
}

#[cfg(feature = "solver")]
pub fn solve_solver_task_json(request_json: &str) -> Result<String, String> {
    solve_solver_task_json_inner(request_json).map_err(|error| error.to_string())
}

#[cfg(feature = "solver")]
pub fn solve_solver_task_json_with_progress<O>(
    request_json: &str,
    on_observation: O,
) -> Result<String, String>
where
    O: FnMut(&str),
{
    solve_solver_task_json_inner_with_progress(request_json, Some(on_observation))
        .map_err(|error| error.to_string())
}

#[cfg(feature = "solver")]
pub fn solver_task_initial_display_state_json(request_json: &str) -> Result<String, String> {
    solver_task_initial_display_state_json_inner(request_json).map_err(|error| error.to_string())
}

#[cfg(feature = "solver")]
fn solve_solver_task_json_inner(request_json: &str) -> Result<String, AppError> {
    solve_solver_task_json_inner_with_progress(request_json, None::<fn(&str)>)
}

#[cfg(feature = "solver")]
fn solver_task_initial_display_state_json_inner(request_json: &str) -> Result<String, AppError> {
    let request: serde_json::Value = serde_json::from_str(request_json)
        .map_err(|error| AppError::Config(format!("solver task JSON is invalid: {error}")))?;
    let request = json_object(&request, "solver task")?;
    let rules = required_json_object(request, "rules")?;
    let model_kind = required_json_string(rules, "modelKind")?;
    let target = required_json_object(request, "target")?;
    validate_solver_target_origin(target)?;
    required_json_object(target, "level")?;
    let target_state = required_json_object(target, "state")?;
    let state_data = required_json_value(target_state, "data")?.to_string();

    match model_kind {
        "2d" => {
            let compiled_play = required_json_value(rules, "compiledPlay")?;
            let engine =
                puzzle_core_wasm::decode_compiled_play(compiled_play).map_err(AppError::Config)?;
            if required_json_string(target_state, "kind")? == "level-ascii" {
                return Err(AppError::Config(
                    "compiled solver task display materialization does not support level-ascii target states"
                        .to_string(),
                ));
            }
            let mut state = puzzle_core_wasm::decode_state(engine.game(), &state_data)
                .map_err(AppError::Config)?;
            if solver_request_materializes_level_start(target_state)? {
                let level_index = solver_task_level_index(target)?;
                state = materialize_compiled_level_start_state(
                    &engine,
                    state,
                    level_index,
                    rules
                        .get("runRulesOnLevelStart")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                )?;
            }
            state = materialize_compiled_display_state(&engine, &state)?;
            let mut out = String::new();
            push_state_data(&mut out, &state);
            Ok(out)
        }
        "3d" => Err(AppError::Config(
            "compiled 3D solver task display materialization is not implemented".to_string(),
        )),
        other => Err(AppError::Config(format!(
            "unsupported solver task modelKind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn solve_solver_task_json_inner_with_progress<O>(
    request_json: &str,
    mut on_observation: Option<O>,
) -> Result<String, AppError>
where
    O: FnMut(&str),
{
    let request: serde_json::Value = serde_json::from_str(request_json)
        .map_err(|error| AppError::Config(format!("solver task JSON is invalid: {error}")))?;
    let request = json_object(&request, "solver task")?;
    let rules = required_json_object(request, "rules")?;
    let model_kind = required_json_string(rules, "modelKind")?;
    let target = required_json_object(request, "target")?;
    validate_solver_target_origin(target)?;
    required_json_object(target, "level")?;
    let target_state = required_json_object(target, "state")?;
    let state_data = required_json_value(target_state, "data")?.to_string();
    let max_depth = json_u32_value(request.get("maxDepth"), "maxDepth")?;
    let max_nodes = json_usize_value(request.get("maxNodes"), "maxNodes")?;
    let max_ms = json_u64_value(request.get("maxMs"), "maxMs")?;

    match model_kind {
        "2d" => {
            let compiled_play = required_json_value(rules, "compiledPlay")?;
            let input_labels = decode_compiled_input_labels(compiled_play)?;
            let engine =
                puzzle_core_wasm::decode_compiled_play(compiled_play).map_err(AppError::Config)?;
            validate_compiled_solver_input_labels(engine.game(), &input_labels)?;
            if required_json_string(target_state, "kind")? == "level-ascii" {
                return Err(AppError::Config(
                    "compiled solver task does not support level-ascii target states".to_string(),
                ));
            }
            let mut state = puzzle_core_wasm::decode_state(engine.game(), &state_data)
                .map_err(AppError::Config)?;
            if solver_request_materializes_level_start(target_state)? {
                let level_index = solver_task_level_index(target)?;
                state = materialize_compiled_level_start_state(
                    &engine,
                    state,
                    level_index,
                    rules
                        .get("runRulesOnLevelStart")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                )?;
            }
            let mut progress_json = |state: &State, progress: SearchProgress| {
                if let Some(on_observation) = on_observation.as_mut() {
                    let mut out = String::new();
                    push_search_observation(&mut out, state, &progress);
                    on_observation(&out);
                }
            };
            let response = solve_compiled_state_with_budget_and_progress(
                &engine,
                solver_task_level_index(target)?,
                serde_json::from_value(required_json_value(rules, "solverStrategy")?.clone())
                    .map_err(|error| {
                        AppError::Config(format!("solver task strategy is invalid: {error}"))
                    })?,
                decode_optional_goal_expr(rules.get("goal"))?,
                decode_optional_goal_expr(rules.get("lose"))?,
                optional_json_bool(request.get("acceptWinCommand"), true)?,
                state,
                solver_request_budget(max_depth, max_nodes, max_ms)?,
                Some(&mut progress_json),
            )?;
            let mut out = String::new();
            push_compiled_solution_response(&mut out, &response, &input_labels)?;
            Ok(out)
        }
        "3d" => Err(AppError::Config(
            "compiled 3D solver task decoding is not implemented".to_string(),
        )),
        other => Err(AppError::Config(format!(
            "unsupported solver task modelKind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn solve_request_json_inner(request_json: &str) -> Result<String, AppError> {
    let request: serde_json::Value = serde_json::from_str(request_json)
        .map_err(|error| AppError::Config(format!("solver request JSON is invalid: {error}")))?;
    let request = json_object(&request, "solver request")?;
    let task = decode_solver_request_task(request.get("task"))?;
    let source = required_json_string(request, "source")?;
    let puzzle_path = required_json_string(request, "puzzlePath")?;
    let model_kind = required_json_string(request, "modelKind")?;
    let target = required_json_object(request, "target")?;
    let target_state = required_json_object(target, "state")?;
    let state_data = required_json_value(target_state, "data")?.to_string();
    let max_depth = json_u32_value(request.get("maxDepth"), "maxDepth")?;
    let max_nodes = json_usize_value(request.get("maxNodes"), "maxNodes")?;
    let max_ms = json_u64_value(request.get("maxMs"), "maxMs")?;
    let goal = request.get("goal");
    let collect = request.get("collect");
    let lose = decode_optional_goal_expr(request.get("lose"))?;
    let accept_win_command = optional_json_bool(request.get("acceptWinCommand"), true)?;

    puzzle_lang::validate_source_profile_for_path(source, puzzle_path)?;
    match model_kind {
        "2d" => solve_request2_json_from_source_inner(
            source,
            puzzle_path,
            target,
            target_state,
            &state_data,
            max_depth,
            max_nodes,
            max_ms,
            task,
            goal,
            collect,
            lose,
            accept_win_command,
        ),
        "3d" => solve_request3_json_from_source_inner(
            source,
            target,
            target_state,
            &state_data,
            max_depth,
            max_nodes,
            max_ms,
            task,
            decode_optional_goal_expr(goal)?,
            lose,
        ),
        other => Err(AppError::Config(format!(
            "unsupported solver request modelKind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn solve_request2_json_from_source_inner(
    source: &str,
    _puzzle_path: &str,
    target: &serde_json::Map<String, serde_json::Value>,
    target_state: &serde_json::Map<String, serde_json::Value>,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
    task: SolverRequestTask,
    goal: Option<&serde_json::Value>,
    collect: Option<&serde_json::Value>,
    lose: Option<GoalExpr>,
    accept_win_command: bool,
) -> Result<String, AppError> {
    if matches!(task, SolverRequestTask::Reachability) && goal.is_none() {
        return Err(AppError::Config(
            "solver reachability requests require an explicit goal".to_string(),
        ));
    }
    if matches!(task, SolverRequestTask::Collect) && goal.is_some() {
        return Err(AppError::Config(
            "solver collect requests use collect, not goal".to_string(),
        ));
    }
    let loaded = parse_game(source)?;
    let level_index = validate_solver_request_level2d(&loaded, target)?;
    let mut state = state2_from_solver_target(&loaded, target_state, level_index, state_json)?;
    if solver_request_materializes_level_start(target_state)? {
        state = materialize_level_start_state(&loaded, state, level_index)?;
    }
    let budget = solver_request_budget(max_depth, max_nodes, max_ms)?;
    if matches!(task, SolverRequestTask::Collect) {
        let (selector, max_results) = decode_solver_collect2(collect)?;
        let response = solve_current_state_collect_with_budget_inner(
            &loaded,
            level_index,
            selector,
            lose,
            accept_win_command,
            state,
            budget,
            max_results,
            None::<fn(&State, SearchProgress)>,
        )?;
        let mut out = String::new();
        push_collect_response(&mut out, &loaded, &response);
        return Ok(out);
    }
    let solver_goal = decode_optional_solver_goal2(&loaded, goal, level_index)?;
    let response = solve_current_state_with_goal_with_budget_inner(
        &loaded,
        level_index,
        solver_goal,
        lose,
        accept_win_command,
        state,
        budget,
        None::<fn(&State, SearchProgress)>,
    )?;
    let mut out = String::new();
    match task {
        SolverRequestTask::Solve => push_solution_response(&mut out, &loaded, &response),
        SolverRequestTask::Reachability => push_reachability_response(&mut out, &loaded, &response),
        SolverRequestTask::Collect => unreachable!("collect task returned before solve response"),
    }
    Ok(out)
}

#[cfg(feature = "solver")]
fn solve_request3_json_from_source_inner(
    source: &str,
    target: &serde_json::Map<String, serde_json::Value>,
    target_state: &serde_json::Map<String, serde_json::Value>,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
    task: SolverRequestTask,
    goal: Option<GoalExpr>,
    lose: Option<GoalExpr>,
) -> Result<String, AppError> {
    if matches!(task, SolverRequestTask::Reachability) {
        return Err(AppError::Config(
            "3D source solver request does not support reachability tasks yet".to_string(),
        ));
    }
    if matches!(task, SolverRequestTask::Collect) {
        return Err(AppError::Config(
            "3D source solver request does not support collect tasks yet".to_string(),
        ));
    }
    if goal.is_some() || lose.is_some() {
        return Err(AppError::Config(
            "3D source solver request does not support explicit goal or lose conditions yet"
                .to_string(),
        ));
    }
    let parsed = parse_puzzle3d_for_solver(source)?;
    validate_solver_request_level3d(&parsed, target)?;
    let mut state = state3_from_json(&parsed.game, state_json)?;
    if solver_request_materializes_level_start(target_state)? {
        state = materialize_level_start_state3(&parsed, state)?;
    }
    let budget = solver_request_budget(max_depth, max_nodes, max_ms)?;
    let response = solve_current_state3_with_budget(&parsed, state, budget)?;
    let mut out = String::new();
    push_solution_response3(&mut out, &parsed, &response);
    Ok(out)
}

#[cfg(feature = "solver")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SolverRequestTask {
    Solve,
    Reachability,
    Collect,
}

#[cfg(feature = "solver")]
fn decode_solver_request_task(
    value: Option<&serde_json::Value>,
) -> Result<SolverRequestTask, AppError> {
    let Some(value) = value else {
        return Ok(SolverRequestTask::Solve);
    };
    match value
        .as_str()
        .ok_or_else(|| AppError::Config("solver request task must be a string".to_string()))?
    {
        "solve" => Ok(SolverRequestTask::Solve),
        "reachability" => Ok(SolverRequestTask::Reachability),
        "collect" => Ok(SolverRequestTask::Collect),
        other => Err(AppError::Config(format!(
            "unsupported solver request task {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn solver_request_budget(
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<SearchBudget, AppError> {
    #[cfg(target_arch = "wasm32")]
    if max_ms > 0 {
        return Err(AppError::Config(
            "WASM solver does not support maxMs time budgets".to_string(),
        ));
    }
    let budget = if max_ms > 0 {
        SearchBudget::bounded(max_depth, max_nodes, Duration::from_millis(max_ms))
    } else {
        SearchBudget {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
            max_frontier: None,
            max_duration: None,
        }
    };
    Ok(budget)
}

#[cfg(feature = "solver")]
fn solver_task_level_index(
    target: &serde_json::Map<String, serde_json::Value>,
) -> Result<usize, AppError> {
    let level = required_json_object(target, "level")?;
    json_usize_value(level.get("index"), "level.index")
}

#[cfg(feature = "solver")]
fn materialize_compiled_level_start_state(
    engine: &puzzle_core_wasm::CompiledEngine,
    state: State,
    level_index: usize,
    run_rules_on_level_start: bool,
) -> Result<State, AppError> {
    let mut state = state;
    let mut cancelled = false;
    let level_start = engine.program("level_start", -1).ok_or_else(|| {
        AppError::Config("compiled solver task missing level_start program".to_string())
    })?;
    if !level_start.is_empty() {
        let outcome = transition_program_outcome(engine.game(), level_start, &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    } else if run_rules_on_level_start {
        let level_index = i32::try_from(level_index)
            .map_err(|_| AppError::Config("solver task level index out of range".to_string()))?;
        let program = engine
            .program("run_rules_on_level_start", level_index)
            .ok_or_else(|| {
                AppError::Config(format!(
                    "compiled solver task missing main program for level {level_index}"
                ))
            })?;
        let outcome = transition_program_outcome(engine.game(), program, &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    }
    if !cancelled {
        let level_index = i32::try_from(level_index)
            .map_err(|_| AppError::Config("solver task level index out of range".to_string()))?;
        let local = engine
            .program("level_start_local", level_index)
            .ok_or_else(|| {
                AppError::Config(format!(
                    "compiled solver task missing local level_start program for level {level_index}"
                ))
            })?;
        if !local.is_empty() {
            let outcome = transition_program_outcome(engine.game(), local, &state, InputId(0))?;
            state = outcome.next_state;
        }
    }
    Ok(state)
}

#[cfg(feature = "solver")]
fn decode_optional_goal_expr(
    value: Option<&serde_json::Value>,
) -> Result<Option<GoalExpr>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let goal = json_object(value, "solver task goal")?;
    let expr = required_json_value(goal, "expr")?;
    decode_goal_expr(expr).map(Some)
}

#[cfg(feature = "solver")]
fn decode_optional_solver_goal2(
    loaded: &LoadedGame,
    value: Option<&serde_json::Value>,
    level_index: usize,
) -> Result<SolverGoal2, AppError> {
    let Some(value) = value else {
        return Ok(SolverGoal2::BuiltIn);
    };
    if value.is_null() {
        return Ok(SolverGoal2::BuiltIn);
    }
    let goal = json_object(value, "solver request goal")?;
    if let Some(expr) = goal.get("expr") {
        return decode_goal_expr(expr).map(SolverGoal2::Expr);
    }
    match required_json_string(goal, "kind")? {
        "exact-state" => {
            let state_spec = required_json_object(goal, "state")?;
            state2_from_solver_state_spec(loaded, state_spec, level_index)
                .map(SolverGoal2::ExactState)
        }
        other => Err(AppError::Config(format!(
            "unsupported solver goal kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn decode_solver_collect2(
    value: Option<&serde_json::Value>,
) -> Result<(SolverCollectSelector2, usize), AppError> {
    let value = value
        .ok_or_else(|| AppError::Config("solver collect requests require collect".to_string()))?;
    let collect = json_object(value, "solver collect")?;
    let max_results = json_usize_value(collect.get("maxResults"), "collect.maxResults")?;
    if max_results == 0 {
        return Err(AppError::Config(
            "solver collect maxResults must be greater than zero".to_string(),
        ));
    }
    let selector = match required_json_string(collect, "kind")? {
        "predicate" => {
            let predicate = required_json_object(collect, "predicate")?;
            let expr = decode_goal_expr(required_json_value(predicate, "expr")?)?;
            SolverCollectSelector2::Predicate(expr)
        }
        "maximize" => {
            let objective = required_json_object(collect, "objective")?;
            let value = decode_goal_value(required_json_value(objective, "value")?)?;
            SolverCollectSelector2::Maximize(value)
        }
        other => {
            return Err(AppError::Config(format!(
                "unsupported solver collect kind {other:?}"
            )));
        }
    };
    Ok((selector, max_results))
}

#[cfg(feature = "solver")]
fn decode_goal_expr(value: &serde_json::Value) -> Result<GoalExpr, AppError> {
    let object = json_object(value, "goal expr")?;
    match required_json_string(object, "kind")? {
        "all" => Ok(GoalExpr::All(decode_goal_exprs(required_json_value(
            object, "exprs",
        )?)?)),
        "any" => Ok(GoalExpr::Any(decode_goal_exprs(required_json_value(
            object, "exprs",
        )?)?)),
        "clause" => Ok(GoalExpr::Clause(GoalClause {
            value: decode_goal_value(required_json_value(object, "value")?)?,
            op: decode_json_comparison(required_json_string(object, "op")?)?,
            expected: required_json_value(object, "expected")?
                .as_i64()
                .ok_or_else(|| AppError::Config("goal expected must be an integer".to_string()))?,
        })),
        other => Err(AppError::Config(format!(
            "unsupported goal expr kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn decode_goal_exprs(value: &serde_json::Value) -> Result<Vec<GoalExpr>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Config("goal exprs must be an array".to_string()))?
        .iter()
        .map(decode_goal_expr)
        .collect()
}

#[cfg(feature = "solver")]
fn decode_goal_value(value: &serde_json::Value) -> Result<GoalValue, AppError> {
    let object = json_object(value, "goal value")?;
    match required_json_string(object, "kind")? {
        "variable" => Ok(GoalValue::Variable(VariableId(json_u16_value(
            required_json_value(object, "variable")?,
            "goal variable",
        )?))),
        "condition" => Ok(GoalValue::Condition(ConditionId(json_u16_value(
            required_json_value(object, "condition")?,
            "goal condition",
        )?))),
        "condition_value" => Ok(GoalValue::InlineConditionValue(
            decode_condition_value_kind(required_json_value(object, "conditionValueKind")?)?,
        )),
        other => Err(AppError::Config(format!(
            "unsupported goal value kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn decode_condition_value_kind(value: &serde_json::Value) -> Result<ConditionValueKind, AppError> {
    let object = json_object(value, "condition value kind")?;
    match required_json_string(object, "kind")? {
        "count_objects" => Ok(ConditionValueKind::CountObjects(decode_object_ids(
            required_json_value(object, "objects")?,
        )?)),
        "exists_objects" => Ok(ConditionValueKind::ExistsObjects(decode_object_ids(
            required_json_value(object, "objects")?,
        )?)),
        "none_objects" => Ok(ConditionValueKind::NoneObjects(decode_object_ids(
            required_json_value(object, "objects")?,
        )?)),
        "count_matches" => Ok(ConditionValueKind::CountMatches(decode_patterns(
            required_json_value(object, "patterns")?,
        )?)),
        "exists_matches" => Ok(ConditionValueKind::ExistsMatches(decode_patterns(
            required_json_value(object, "patterns")?,
        )?)),
        "none_matches" => Ok(ConditionValueKind::NoneMatches(decode_patterns(
            required_json_value(object, "patterns")?,
        )?)),
        "count_input_matches" => Ok(ConditionValueKind::CountInputMatches(
            decode_input_patterns(required_json_value(object, "patterns")?)?,
        )),
        "exists_input_matches" => Ok(ConditionValueKind::ExistsInputMatches(
            decode_input_patterns(required_json_value(object, "patterns")?)?,
        )),
        "none_input_matches" => Ok(ConditionValueKind::NoneInputMatches(decode_input_patterns(
            required_json_value(object, "patterns")?,
        )?)),
        other => Err(AppError::Config(format!(
            "unsupported condition value kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn decode_patterns(value: &serde_json::Value) -> Result<Vec<Pattern>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Config("patterns must be an array".to_string()))?
        .iter()
        .map(decode_pattern)
        .collect()
}

#[cfg(feature = "solver")]
fn decode_input_patterns(value: &serde_json::Value) -> Result<Vec<(InputId, Pattern)>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Config("input patterns must be an array".to_string()))?
        .iter()
        .map(|value| {
            let object = json_object(value, "input pattern")?;
            Ok((
                InputId(json_u16_value(
                    required_json_value(object, "input")?,
                    "input pattern input",
                )?),
                decode_pattern(required_json_value(object, "pattern")?)?,
            ))
        })
        .collect()
}

#[cfg(feature = "solver")]
fn decode_pattern(value: &serde_json::Value) -> Result<Pattern, AppError> {
    let object = json_object(value, "pattern")?;
    let object = match object.get("pattern") {
        Some(pattern) => json_object(pattern, "pattern")?,
        None => object,
    };
    let components = required_json_value(object, "components")?
        .as_array()
        .ok_or_else(|| AppError::Config("pattern components must be an array".to_string()))?
        .iter()
        .map(|value| {
            let object = json_object(value, "pattern component")?;
            Ok(PatternComponent {
                gap_count: json_u16_value(
                    required_json_value(object, "gapCount")?,
                    "pattern gapCount",
                )?,
                cells: required_json_value(object, "cells")?
                    .as_array()
                    .ok_or_else(|| {
                        AppError::Config("pattern component cells must be an array".to_string())
                    })?
                    .iter()
                    .map(decode_match_cell)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Pattern { components })
}

#[cfg(feature = "solver")]
fn decode_match_cell(value: &serde_json::Value) -> Result<MatchCell, AppError> {
    let object = json_object(value, "match cell")?;
    let require_null = required_json_value(object, "requireNull")?
        .as_bool()
        .ok_or_else(|| AppError::Config("match cell requireNull must be a bool".to_string()))?;
    Ok(MatchCell {
        offset: decode_offset(required_json_value(object, "offset")?)?,
        require_null,
        require_objects: decode_object_ids(required_json_value(object, "requireObjects")?)?,
        require_object_sets: Vec::new(),
        forbid_objects: decode_object_ids(required_json_value(object, "forbidObjects")?)?,
        require_mark: decode_mark_patterns(required_json_value(object, "requireMark")?)?,
        require_object_set_mark: Vec::new(),
        forbid_mark: decode_mark_patterns(required_json_value(object, "forbidMark")?)?,
        forbid_object_set_mark: Vec::new(),
    })
}

#[cfg(feature = "solver")]
fn decode_offset(value: &serde_json::Value) -> Result<Offset, AppError> {
    let object = json_object(value, "offset")?;
    match required_json_string(object, "kind")? {
        "fixed" => Ok(Offset::Fixed {
            delta: [
                json_i16_value(required_json_value(object, "dx")?, "offset dx")?,
                json_i16_value(required_json_value(object, "dy")?, "offset dy")?,
            ]
            .into(),
        }),
        other => Err(AppError::Config(format!(
            "unsupported offset kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn decode_object_ids(value: &serde_json::Value) -> Result<Vec<ObjectId>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Config("object ids must be an array".to_string()))?
        .iter()
        .map(|value| Ok(ObjectId(json_u16_value(value, "object id")?)))
        .collect()
}

#[cfg(feature = "solver")]
fn decode_mark_patterns(value: &serde_json::Value) -> Result<Vec<MarkPattern>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Config("mark patterns must be an array".to_string()))?
        .iter()
        .map(|value| {
            let object = json_object(value, "mark pattern")?;
            Ok(MarkPattern {
                object: ObjectId(json_u16_value(
                    required_json_value(object, "object")?,
                    "mark object",
                )?),
                mark: MarkId(json_u16_value(
                    required_json_value(object, "mark")?,
                    "mark id",
                )?),
                value: object
                    .get("value")
                    .map(|value| {
                        value.as_i64().ok_or_else(|| {
                            AppError::Config("mark value must be an integer".to_string())
                        })
                    })
                    .transpose()?,
                match_value: match required_json_string(object, "match")? {
                    "any" => MarkValueMatch::Any,
                    "exact" => MarkValueMatch::Exact,
                    other => {
                        return Err(AppError::Config(format!(
                            "unsupported mark match {other:?}"
                        )));
                    }
                },
            })
        })
        .collect()
}

#[cfg(feature = "solver")]
fn decode_json_comparison(value: &str) -> Result<ComparisonOp, AppError> {
    match value {
        "eq" => Ok(ComparisonOp::Eq),
        "not_eq" => Ok(ComparisonOp::NotEq),
        "greater" => Ok(ComparisonOp::Greater),
        "greater_eq" => Ok(ComparisonOp::GreaterEq),
        "less" => Ok(ComparisonOp::Less),
        "less_eq" => Ok(ComparisonOp::LessEq),
        other => Err(AppError::Config(format!(
            "unsupported comparison op {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn validate_solver_request_level2d(
    loaded: &LoadedGame,
    target: &serde_json::Map<String, serde_json::Value>,
) -> Result<usize, AppError> {
    validate_solver_target_origin(target)?;
    let level = required_json_object(target, "level")?;
    let index = json_usize_value(level.get("index"), "level.index")?;
    let expected = loaded.levels.get(index).ok_or_else(|| {
        AppError::Config(format!("solver target level index out of range: {index}"))
    })?;
    let name = required_json_string(level, "levelName")?;
    if name != expected.name {
        return Err(AppError::Config(format!(
            "solver target levelName mismatch: expected {:?}, got {:?}",
            expected.name, name
        )));
    }
    let puzzle = required_json_string(level, "levelPuzzle")?;
    if puzzle != expected.puzzle {
        return Err(AppError::Config(format!(
            "solver target levelPuzzle mismatch: expected {:?}, got {:?}",
            expected.puzzle, puzzle
        )));
    }
    let pack = required_json_value(level, "levelPack")?;
    match (&expected.pack, pack) {
        (Some(expected), serde_json::Value::String(actual)) if actual == expected => {}
        (None, serde_json::Value::Null) => {}
        (Some(expected), serde_json::Value::String(actual)) => {
            return Err(AppError::Config(format!(
                "solver target levelPack mismatch: expected {:?}, got {:?}",
                expected, actual
            )));
        }
        (Some(expected), _) => {
            return Err(AppError::Config(format!(
                "solver target levelPack mismatch: expected {:?}",
                expected
            )));
        }
        (None, _) => {
            return Err(AppError::Config(
                "solver target levelPack mismatch: expected null".to_string(),
            ));
        }
    }
    Ok(index)
}

#[cfg(feature = "solver")]
fn validate_solver_request_level3d(
    parsed: &ParsedPuzzle3,
    target: &serde_json::Map<String, serde_json::Value>,
) -> Result<usize, AppError> {
    validate_solver_target_origin(target)?;
    let level = required_json_object(target, "level")?;
    let index = json_usize_value(level.get("index"), "level.index")?;
    let bundle = parsed
        .level_bundle
        .as_ref()
        .ok_or_else(|| AppError::Config("3D solver target requires levels".to_string()))?;
    let expected = bundle.levels.get(index).ok_or_else(|| {
        AppError::Config(format!(
            "3D solver target level index out of range: {index}"
        ))
    })?;
    let name = required_json_string(level, "levelName")?;
    if name != expected.name {
        return Err(AppError::Config(format!(
            "3D solver target levelName mismatch: expected {:?}, got {:?}",
            expected.name, name
        )));
    }
    Ok(index)
}

#[cfg(feature = "solver")]
fn decode_compiled_input_labels(
    compiled_play: &serde_json::Value,
) -> Result<Vec<(InputId, String)>, AppError> {
    let compiled_play = json_object(compiled_play, "compiledPlay")?;
    let labels = required_json_object(compiled_play, "inputLabels")?;
    let mut out = Vec::with_capacity(labels.len());
    for (key, value) in labels {
        let id = key.parse::<u16>().map_err(|error| {
            AppError::Config(format!(
                "compiled input label id {key:?} is invalid: {error}"
            ))
        })?;
        let label = value.as_str().ok_or_else(|| {
            AppError::Config(format!("compiled input label {key:?} must be a string"))
        })?;
        if label.trim().is_empty() {
            return Err(AppError::Config(format!(
                "compiled input label {key:?} must not be empty"
            )));
        }
        out.push((InputId(id), label.to_string()));
    }
    out.sort_by_key(|(id, _)| *id);
    Ok(out)
}

#[cfg(feature = "solver")]
fn validate_compiled_solver_input_labels(
    game: &CompiledGame,
    input_labels: &[(InputId, String)],
) -> Result<(), AppError> {
    let mut inputs = BTreeSet::new();
    collect_solver_inputs(game.program(), &mut inputs);
    for input in inputs {
        if !input_labels.iter().any(|(id, _)| *id == input) {
            return Err(AppError::Config(format!(
                "compiled solver input label is missing for input {}",
                input.0
            )));
        }
    }
    Ok(())
}

#[cfg(feature = "solver")]
fn validate_solver_target_origin(
    target: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    match required_json_string(target, "origin")? {
        "preview-level" | "level-editor" => Ok(()),
        other => Err(AppError::Config(format!(
            "unsupported solver target origin {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn solver_request_materializes_level_start(
    target_state: &serde_json::Map<String, serde_json::Value>,
) -> Result<bool, AppError> {
    match required_json_string(target_state, "kind")? {
        "compiled-start" | "editor-staged" | "level-ascii" => {}
        other => {
            return Err(AppError::Config(format!(
                "unsupported solver target state kind {other:?}"
            )));
        }
    }
    match required_json_string(target_state, "lifecycle")? {
        "playable-start" => Ok(true),
        "already-materialized" => Ok(false),
        other => Err(AppError::Config(format!(
            "unsupported solver target state lifecycle {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn state2_from_solver_target(
    loaded: &LoadedGame,
    target_state: &serde_json::Map<String, serde_json::Value>,
    level_index: usize,
    state_json: &str,
) -> Result<State, AppError> {
    state2_from_solver_state_spec_with_data(loaded, target_state, level_index, state_json)
}

#[cfg(feature = "solver")]
fn state2_from_solver_state_spec(
    loaded: &LoadedGame,
    state_spec: &serde_json::Map<String, serde_json::Value>,
    level_index: usize,
) -> Result<State, AppError> {
    let state_json = required_json_value(state_spec, "data")?.to_string();
    state2_from_solver_state_spec_with_data(loaded, state_spec, level_index, &state_json)
}

#[cfg(feature = "solver")]
fn state2_from_solver_state_spec_with_data(
    loaded: &LoadedGame,
    state_spec: &serde_json::Map<String, serde_json::Value>,
    level_index: usize,
    state_json: &str,
) -> Result<State, AppError> {
    match required_json_string(state_spec, "kind")? {
        "raw" | "compiled-start" | "editor-staged" => state_from_json(loaded, state_json),
        "level-ascii" => {
            let data = required_json_value(state_spec, "data")?;
            state_from_level_ascii_json(loaded, data, level_index)
        }
        other => Err(AppError::Config(format!(
            "unsupported solver state kind {other:?}"
        ))),
    }
}

#[cfg(feature = "solver")]
fn state_from_level_ascii_json(
    loaded: &LoadedGame,
    data: &serde_json::Value,
    level_index: usize,
) -> Result<State, AppError> {
    let object = json_object(data, "solver level-ascii state")?;
    let empty = single_char_json_string(required_json_value(object, "empty")?, "empty")?;
    let lines = required_json_value(object, "lines")?
        .as_array()
        .ok_or_else(|| AppError::Config("solver level-ascii lines must be an array".to_string()))?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                AppError::Config("solver level-ascii line must be a string".to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let legend = json_object(
        required_json_value(object, "legend")?,
        "solver level-ascii legend",
    )?;
    let mut char_objects = HashMap::<char, Vec<ObjectId>>::new();
    for (raw_char, raw_objects) in legend {
        let ch = single_char(raw_char, "solver level-ascii legend key")?;
        let objects = decode_object_ids(raw_objects)?;
        char_objects.insert(ch, objects);
    }
    let variable_defaults = loaded
        .levels
        .get(level_index)
        .ok_or_else(|| {
            AppError::Config(format!(
                "solver target level index out of range: {level_index}"
            ))
        })?
        .initial_state
        .visible_variables()
        .to_vec();
    let (state, _) = puzzle_lang::parse_level_ascii_state(
        &loaded.game,
        &lines,
        empty,
        &char_objects,
        &variable_defaults,
    )
    .map_err(|error| AppError::Config(error.to_string()))?;
    Ok(state)
}

#[cfg(feature = "solver")]
fn json_object<'a>(
    value: &'a serde_json::Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, AppError> {
    value
        .as_object()
        .ok_or_else(|| AppError::Config(format!("{label} must be a JSON object")))
}

#[cfg(feature = "solver")]
fn required_json_object<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, AppError> {
    json_object(required_json_value(object, key)?, key)
}

#[cfg(feature = "solver")]
fn required_json_value<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a serde_json::Value, AppError> {
    object
        .get(key)
        .ok_or_else(|| AppError::Config(format!("solver request missing {key}")))
}

#[cfg(feature = "solver")]
fn required_json_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str, AppError> {
    required_json_value(object, key)?
        .as_str()
        .ok_or_else(|| AppError::Config(format!("solver request {key} must be a string")))
}

#[cfg(feature = "solver")]
fn single_char_json_string(value: &serde_json::Value, label: &str) -> Result<char, AppError> {
    let raw = value
        .as_str()
        .ok_or_else(|| AppError::Config(format!("solver request {label} must be a string")))?;
    single_char(raw, label)
}

#[cfg(feature = "solver")]
fn single_char(value: &str, label: &str) -> Result<char, AppError> {
    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        return Err(AppError::Config(format!(
            "solver request {label} must be one character"
        )));
    };
    if chars.next().is_some() {
        return Err(AppError::Config(format!(
            "solver request {label} must be one character"
        )));
    }
    Ok(ch)
}

#[cfg(feature = "solver")]
fn json_u32_value(value: Option<&serde_json::Value>, key: &str) -> Result<u32, AppError> {
    let raw = value.and_then(serde_json::Value::as_u64).ok_or_else(|| {
        AppError::Config(format!("solver request {key} must be an unsigned integer"))
    })?;
    u32::try_from(raw)
        .map_err(|_| AppError::Config(format!("solver request {key} is out of range")))
}

#[cfg(feature = "solver")]
fn json_usize_value(value: Option<&serde_json::Value>, key: &str) -> Result<usize, AppError> {
    let raw = value.and_then(serde_json::Value::as_u64).ok_or_else(|| {
        AppError::Config(format!("solver request {key} must be an unsigned integer"))
    })?;
    usize::try_from(raw)
        .map_err(|_| AppError::Config(format!("solver request {key} is out of range")))
}

#[cfg(feature = "solver")]
fn json_u64_value(value: Option<&serde_json::Value>, key: &str) -> Result<u64, AppError> {
    value.and_then(serde_json::Value::as_u64).ok_or_else(|| {
        AppError::Config(format!("solver request {key} must be an unsigned integer"))
    })
}

#[cfg(feature = "solver")]
fn optional_json_bool(value: Option<&serde_json::Value>, default: bool) -> Result<bool, AppError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .as_bool()
        .ok_or_else(|| AppError::Config("solver request acceptWinCommand must be a boolean".into()))
}

#[cfg(feature = "solver")]
fn json_u16_value(value: &serde_json::Value, key: &str) -> Result<u16, AppError> {
    let raw = value
        .as_u64()
        .ok_or_else(|| AppError::Config(format!("{key} must be an unsigned integer")))?;
    u16::try_from(raw).map_err(|_| AppError::Config(format!("{key} is out of range")))
}

#[cfg(feature = "solver")]
fn json_i16_value(value: &serde_json::Value, key: &str) -> Result<i16, AppError> {
    let raw = value
        .as_i64()
        .ok_or_else(|| AppError::Config(format!("{key} must be an integer")))?;
    i16::try_from(raw).map_err(|_| AppError::Config(format!("{key} is out of range")))
}

#[cfg(feature = "solver")]
pub fn solve_state_json_from_source(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, String> {
    solve_state_json_from_source_inner(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes,
        max_ms,
    )
    .map_err(|error| error.to_string())
}

#[cfg(feature = "solver")]
fn solve_state_json_from_source_inner(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    puzzle_lang::validate_source_profile_for_path(source, puzzle_path)?;
    if source_looks_puzzle3d(source) || state_json.contains("\"kind\":\"puzzle3d\"") {
        return solve_state3_json_from_source_inner(
            source, state_json, max_depth, max_nodes, max_ms,
        );
    }

    let loaded = parse_game(source)?;
    let state = state_from_json(&loaded, state_json)?;
    let state = match level_index_from_state_json(&loaded, state_json) {
        Some(level_index) => materialize_level_start_state(&loaded, state, level_index)?,
        None => state,
    };
    let solver = SolverConfig {
        max_depth,
        max_nodes,
        max_duration: if max_ms > 0 {
            Duration::from_millis(max_ms)
        } else {
            Duration::from_secs(24 * 60 * 60)
        },
    };
    let budget = if max_ms > 0 {
        solver.budget()
    } else {
        SearchBudget {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
            max_frontier: None,
            max_duration: None,
        }
    };
    let level_index = level_index_from_state_json(&loaded, state_json)
        .ok_or_else(|| AppError::Config("solver state requires a valid levelIndex".to_string()))?;
    let response = solve_current_state_with_budget(&loaded, level_index, state, budget)?;
    let mut out = String::new();
    push_solution_response(&mut out, &loaded, &response);
    Ok(out)
}

#[cfg(feature = "solver")]
fn solve_state3_json_from_source_inner(
    source: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: usize,
    max_ms: u64,
) -> Result<String, AppError> {
    let parsed = parse_puzzle3d_for_solver(source)?;
    let state = state3_from_json(&parsed.game, state_json)?;
    let state = if level_index_from_state3_json(&parsed, state_json).is_some() {
        materialize_level_start_state3(&parsed, state)?
    } else {
        state
    };
    let budget = if max_ms > 0 {
        SearchBudget::bounded(max_depth, max_nodes, Duration::from_millis(max_ms))
    } else {
        SearchBudget {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
            max_frontier: None,
            max_duration: None,
        }
    };
    let response = solve_current_state3_with_budget(&parsed, state, budget)?;
    let mut out = String::new();
    push_solution_response3(&mut out, &parsed, &response);
    Ok(out)
}

#[cfg(feature = "solver")]
fn parse_puzzle3d_for_solver(source: &str) -> Result<ParsedPuzzle3, AppError> {
    let document = puzzle_lang::parse_game_for_path(source, "solver.puzzle3")?;
    document
        .models
        .into_iter()
        .find_map(|model| match model {
            LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle),
            LoadedDocumentModel::Puzzle2d { .. } => None,
        })
        .ok_or_else(|| AppError::Config("3D solver source does not contain a puzzle3 model".into()))
}

fn state_from_json(loaded: &LoadedGame, state_json: &str) -> Result<State, AppError> {
    let width = json_u64_field(state_json, "width")
        .ok_or_else(|| AppError::Config("solver state missing width".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state width out of range".to_string()))?;
    let height = json_u64_field(state_json, "height")
        .ok_or_else(|| AppError::Config("solver state missing height".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state height out of range".to_string()))?;
    let layer_count = json_u64_field(state_json, "layerCount")
        .ok_or_else(|| AppError::Config("solver state missing layerCount".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("solver state layerCount out of range".to_string()))?;
    let slots = json_u64_array_field(state_json, "slots")
        .ok_or_else(|| AppError::Config("solver state missing slots".to_string()))?;
    let variables = json_i64_array_field(state_json, "variables").unwrap_or_default();
    let fired_rules = json_u64_array_field(state_json, "levelFiredRules").unwrap_or_default();
    let expected_slots = usize::from(width) * usize::from(height) * usize::from(layer_count);
    if slots.len() != expected_slots {
        return Err(AppError::Config(format!(
            "solver state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        )));
    }

    let mut state = State::empty_with_variables(
        width,
        height,
        layer_count,
        loaded.game.object_count(),
        variables,
    )
    .map_err(|error| AppError::Config(format!("{error:?}")))?;
    for (index, object) in slots.into_iter().enumerate() {
        if object == 0 {
            continue;
        }
        let object: u16 = object
            .try_into()
            .map_err(|_| AppError::Config("solver state object id out of range".to_string()))?;
        let layer = index % usize::from(layer_count);
        let cell = index / usize::from(layer_count);
        let x = (cell % usize::from(width)) as u16;
        let y = (cell / usize::from(width)) as u16;
        let expected_layer = loaded
            .game
            .object_layer(ObjectId(object))
            .ok_or_else(|| AppError::Config(format!("solver state unknown object id {object}")))?;
        if usize::from(expected_layer.0) != layer {
            return Err(AppError::Config(format!(
                "solver state object {object} is in layer {layer}, expected {}",
                expected_layer.0
            )));
        }
        state
            .place_object(&loaded.game, x, y, ObjectId(object))
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
    }
    for rule in fired_rules {
        let rule: u16 = rule
            .try_into()
            .map_err(|_| AppError::Config("solver state rule id out of range".to_string()))?;
        state.mark_level_rule_fired(RuleId(rule));
    }
    Ok(state)
}

fn level_index_from_state_json(loaded: &LoadedGame, state_json: &str) -> Option<usize> {
    let index = usize::try_from(json_u64_field(state_json, "levelIndex")?).ok()?;
    (index < loaded.levels.len()).then_some(index)
}

fn materialize_level_start_state(
    loaded: &LoadedGame,
    state: State,
    level_index: usize,
) -> Result<State, AppError> {
    let mut state = state;
    let mut cancelled = false;
    if let Some(program) = loaded.level_start_program.as_deref() {
        let outcome = transition_program_outcome(&loaded.game, program, &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    } else if loaded.run_rules_on_level_start {
        let program = loaded.program_for_level(level_index).ok_or_else(|| {
            AppError::Config(format!("level start index out of range: {level_index}"))
        })?;
        let outcome = transition_program_outcome(&loaded.game, program, &state, InputId(0))?;
        state = outcome.next_state;
        cancelled |= outcome.cancelled;
    }
    if !cancelled {
        if let Some(program) = loaded
            .levels
            .get(level_index)
            .and_then(|level| level.level_start_program.as_deref())
        {
            let outcome = transition_program_outcome(&loaded.game, program, &state, InputId(0))?;
            state = outcome.next_state;
        }
    }
    Ok(state)
}

fn state3_from_json(game: &CompiledGame3, state_json: &str) -> Result<State3, AppError> {
    let width = json_u64_field(state_json, "width")
        .ok_or_else(|| AppError::Config("3D solver state missing width".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state width out of range".to_string()))?;
    let depth = json_u64_field(state_json, "depth")
        .ok_or_else(|| AppError::Config("3D solver state missing depth".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state depth out of range".to_string()))?;
    let height = json_u64_field(state_json, "height")
        .ok_or_else(|| AppError::Config("3D solver state missing height".to_string()))?
        .try_into()
        .map_err(|_| AppError::Config("3D solver state height out of range".to_string()))?;
    let layer_count = json_u64_field(state_json, "layerCount")
        .map(u16::try_from)
        .transpose()
        .map_err(|_| AppError::Config("3D solver state layerCount out of range".to_string()))?
        .unwrap_or(game.layer_count);
    if layer_count != game.layer_count {
        return Err(AppError::Config(format!(
            "3D solver state layerCount mismatch: expected {}, got {layer_count}",
            game.layer_count
        )));
    }
    let slots = json_u64_array_field(state_json, "slots")
        .ok_or_else(|| AppError::Config("3D solver state missing slots".to_string()))?;
    let fired_rules = json_u64_array_field(state_json, "levelFiredRules").unwrap_or_default();
    let expected_slots = usize::from(width)
        .checked_mul(usize::from(depth))
        .and_then(|count| count.checked_mul(usize::from(height)))
        .and_then(|count| count.checked_mul(usize::from(layer_count)))
        .ok_or_else(|| AppError::Config("3D solver state dimensions are too large".to_string()))?;
    if slots.len() != expected_slots {
        return Err(AppError::Config(format!(
            "3D solver state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        )));
    }

    let mut state = State3::empty(Size3::new(width, depth, height), layer_count)
        .map_err(|error| AppError::Config(format!("{error:?}")))?;
    for (index, object) in slots.into_iter().enumerate() {
        if object == 0 {
            continue;
        }
        let object: u16 = object
            .try_into()
            .map_err(|_| AppError::Config("3D solver state object id out of range".to_string()))?;
        let layer = index % usize::from(layer_count);
        let cell = index / usize::from(layer_count);
        let x = (cell % usize::from(width)) as u16;
        let yz = cell / usize::from(width);
        let y = (yz % usize::from(depth)) as u16;
        let z = (yz / usize::from(depth)) as u16;
        let object = ObjectId3(object);
        let expected_layer = game.object_layer(object).ok_or_else(|| {
            AppError::Config(format!("3D solver state unknown object id {}", object.0))
        })?;
        if usize::from(expected_layer.0) != layer {
            return Err(AppError::Config(format!(
                "3D solver state object {} is in layer {layer}, expected {}",
                object.0, expected_layer.0
            )));
        }
        state
            .place_object_at(game, Coord3 { x, y, z }, object)
            .map_err(|error| AppError::Config(format!("{error:?}")))?;
    }
    for rule in fired_rules {
        let rule: u16 = rule
            .try_into()
            .map_err(|_| AppError::Config("3D solver state rule id out of range".to_string()))?;
        state.mark_level_rule_fired(RuleId3(rule));
    }
    Ok(state)
}

fn level_index_from_state3_json(parsed: &ParsedPuzzle3, state_json: &str) -> Option<usize> {
    let index = usize::try_from(json_u64_field(state_json, "levelIndex")?).ok()?;
    let level_count = parsed.level_bundle.as_ref()?.levels.len();
    (index < level_count).then_some(index)
}

fn materialize_level_start_state3(
    parsed: &ParsedPuzzle3,
    state: State3,
) -> Result<State3, AppError> {
    transition_program_without_input_with_local_frame(
        &parsed.game,
        &state,
        &parsed.lifecycle.on_level_start,
        parsed.lifecycle.on_level_start_local_frame.as_ref(),
    )
    .map_err(|error| AppError::Config(format!("{error:?}")))
}
