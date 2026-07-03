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
        let mut out = String::new();
        push_transition_current_outcome_json(
            &mut out,
            &self.loaded,
            &outcome.next_state,
            Some(&before),
            previous_state_handle,
            outcome.cancelled,
            &outcome.commands,
            &outcome.fired_rules,
            &outcome.patches,
            include_state,
        );
        Ok(out)
    }
}

pub struct Puzzle3RuntimeBridge {
    parsed: ParsedPuzzle3,
    animation: AnimationDef,
    current_state: Option<State3>,
    saved_states: SavedStateStore<State3>,
}

impl Puzzle3RuntimeBridge {
    pub fn from_source(source: &str) -> Result<Self, String> {
        if let Ok(parsed) = puzzle_3d::parse_puzzle3d(source) {
            return Ok(Self {
                parsed,
                animation: AnimationDef::default(),
                current_state: None,
                saved_states: SavedStateStore::new(),
            });
        }
        let document = puzzle_lang::parse_game(source).map_err(|error| error.to_string())?;
        let animation = document.animation.clone();
        let parsed = document
            .models
            .iter()
            .find_map(|model| match model {
                LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle.clone()),
                LoadedDocumentModel::Puzzle2d { .. } => None,
            })
            .ok_or_else(|| "3D runtime source does not contain a puzzle3 model".to_string())?;
        Ok(Self {
            parsed,
            animation,
            current_state: None,
            saved_states: SavedStateStore::new(),
        })
    }

    pub fn transition_program_outcome_json(
        &self,
        program_key: &str,
        state_json: &str,
        input: u16,
    ) -> Result<String, String> {
        transition_program3_outcome_json_inner(
            &self.parsed,
            program_key,
            state_json,
            InputId3(input),
        )
        .map_err(|error| error.to_string())
    }

    pub fn is_complete_json(&self, state_json: &str) -> Result<bool, String> {
        let state =
            state3_from_json(&self.parsed.game, state_json).map_err(|error| error.to_string())?;
        Ok(self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, &state)))
    }

    pub fn set_state_json(&mut self, state_json: &str) -> Result<(), String> {
        let state =
            state3_from_json(&self.parsed.game, state_json).map_err(|error| error.to_string())?;
        self.current_state = Some(state);
        Ok(())
    }

    pub fn current_state_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state3_data(&mut out, state);
        Ok(out)
    }

    pub fn current_cells_json(&self) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let mut out = String::new();
        push_state3_cells(&mut out, state, None);
        Ok(out)
    }

    pub fn save_current_state(&mut self) -> Result<u32, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        Ok(self.saved_states.save(state.clone()))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), String> {
        self.current_state = Some(self.saved_states.restore(handle)?.clone());
        Ok(())
    }

    pub fn transition_current_outcome_json(
        &mut self,
        program_key: &str,
        input: u16,
    ) -> Result<String, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        let before = state.clone();
        let next_state =
            transition_selected_program3(&self.parsed, program_key, state, InputId3(input))
                .map_err(|error| error.to_string())?;
        let completed = self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, &next_state));
        self.current_state = Some(next_state.clone());
        let mut out = String::new();
        out.push('{');
        push_json_bool(&mut out, "changed", before != next_state);
        out.push(',');
        push_json_bool(&mut out, "completed", completed);
        out.push_str(",\"stateHash\":");
        out.push_str(&next_state.hash().to_string());
        out.push_str(",\"changedCells\":");
        push_state3_cells(&mut out, &next_state, Some(&before));
        out.push_str(",\"animationEvents\":");
        push_animation_events3(&mut out, &self.animation, &before, &next_state);
        out.push_str(",\"commands\":[]}");
        Ok(out)
    }

    pub fn is_current_complete(&self) -> Result<bool, String> {
        let state = self
            .current_state
            .as_ref()
            .ok_or_else(|| "3D runtime current state has not been initialized".to_string())?;
        Ok(self
            .parsed
            .win_condition
            .as_ref()
            .is_some_and(|condition| condition.is_met(&self.parsed.game, state)))
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

fn transition_program3_outcome_json_inner(
    parsed: &ParsedPuzzle3,
    program_key: &str,
    state_json: &str,
    input: InputId3,
) -> Result<String, AppError> {
    let state = state3_from_json(&parsed.game, state_json)?;
    let next_state = transition_selected_program3(parsed, program_key, &state, input)?;
    let completed = parsed
        .win_condition
        .as_ref()
        .is_some_and(|condition| condition.is_met(&parsed.game, &next_state));
    let mut out = String::new();
    out.push('{');
    out.push_str("\"state\":");
    push_state3_data(&mut out, &next_state);
    out.push(',');
    push_json_bool(&mut out, "completed", completed);
    out.push_str(",\"commands\":[]}");
    Ok(out)
}

fn transition_selected_program3(
    parsed: &ParsedPuzzle3,
    program_key: &str,
    state: &State3,
    input: InputId3,
) -> Result<State3, AppError> {
    match program_key {
        "main" => transition_program_with_local_frame3(
            &parsed.game,
            state,
            &parsed.rules,
            input,
            parsed.local_frame.as_ref(),
        ),
        "level_start" => transition_program_without_input_with_local_frame(
            &parsed.game,
            state,
            &parsed.lifecycle.on_level_start,
            parsed.lifecycle.on_level_start_local_frame.as_ref(),
        ),
        other => {
            return Err(AppError::Config(format!(
                "unknown 3D transition program selector: {other}"
            )));
        }
    }
    .map_err(|error| AppError::Config(format!("{error:?}")))
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
    let mut out = String::new();
    push_transition_outcome_json(
        &mut out,
        loaded,
        &outcome.next_state,
        outcome.cancelled,
        &outcome.commands,
        &outcome.fired_rules,
        &outcome.patches,
    );
    Ok(out)
}

fn selected_rule_program<'a>(
    loaded: &'a LoadedGame,
    program_key: &str,
    level_index: i32,
) -> Result<&'a [RuleStep], AppError> {
    match program_key {
        "main" | "run_rules_on_level_start" => Ok(loaded.game.program()),
        "level_start" => Ok(loaded.level_start_program.as_deref().unwrap_or(&[])),
        "display_level_start" => Ok(loaded.display_level_start_program.as_deref().unwrap_or(&[])),
        "level_clear" => Ok(loaded.level_clear_program.as_deref().unwrap_or(&[])),
        "display_level_clear" => Ok(loaded.display_level_clear_program.as_deref().unwrap_or(&[])),
        "display" => Ok(loaded.display_program.as_deref().unwrap_or(&[])),
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

fn push_transition_outcome_json(
    out: &mut String,
    loaded: &LoadedGame,
    state: &State,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
) {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
    out.push('{');
    out.push_str("\"state\":");
    push_state_data(out, state);
    out.push(',');
    push_json_bool(out, "cancelled", cancelled);
    out.push(',');
    out.push_str("\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(
            out,
            "kind",
            match command {
                TransitionCommand::Win => "win",
                TransitionCommand::Restart => "restart",
                TransitionCommand::NextLevel => "next_level",
                TransitionCommand::Again => "again",
                TransitionCommand::Checkpoint => "checkpoint",
                TransitionCommand::ClearCheckpoint => "clear_checkpoint",
            },
        );
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"firedRules\":[");
    for (index, rule) in fired_rules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push_str(",\"patches\":");
    push_transition_patches(out, patches);
    out.push(',');
    push_animation_events(out, &animation_events);
    out.push('}');
}

fn push_transition_current_outcome_json(
    out: &mut String,
    loaded: &LoadedGame,
    state: &State,
    before: Option<&State>,
    previous_state_handle: Option<u32>,
    cancelled: bool,
    commands: &[TransitionCommand],
    fired_rules: &[RuleId],
    patches: &[Patch],
    include_state: bool,
) {
    let animation_events = animation_events_for_trace(loaded, fired_rules, patches, state);
    out.push('{');
    push_json_bool(out, "cancelled", cancelled);
    out.push(',');
    push_json_bool(out, "changed", before.is_some_and(|before| before != state));
    if include_state {
        out.push_str(",\"state\":");
        push_state_data(out, state);
    }
    out.push(',');
    out.push_str("\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(
            out,
            "kind",
            match command {
                TransitionCommand::Win => "win",
                TransitionCommand::Restart => "restart",
                TransitionCommand::NextLevel => "next_level",
                TransitionCommand::Again => "again",
                TransitionCommand::Checkpoint => "checkpoint",
                TransitionCommand::ClearCheckpoint => "clear_checkpoint",
            },
        );
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"firedRules\":[");
    for (index, rule) in fired_rules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push_str(",\"patches\":");
    push_transition_patches(out, patches);
    out.push(',');
    push_animation_events(out, &animation_events);
    out.push_str(",\"stateHash\":");
    out.push_str(&state.hash().to_string());
    out.push_str(",\"stateHashKey\":\"");
    out.push_str(&state.hash().to_string());
    out.push('"');
    if let Some(handle) = previous_state_handle {
        out.push_str(",\"previousStateHandle\":");
        out.push_str(&handle.to_string());
    }
    out.push_str(",\"changedCells\":");
    push_state2_cells(out, state, before);
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
    out.push_str("]}");
}

fn push_transition_patches(out: &mut String, patches: &[Patch]) {
    out.push('[');
    for (patch_index, patch) in patches.iter().enumerate() {
        if patch_index > 0 {
            out.push(',');
        }
        out.push('[');
        for (op_index, op) in patch.ops().iter().enumerate() {
            if op_index > 0 {
                out.push(',');
            }
            out.push('{');
            match op {
                PatchOp::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                    object,
                } => {
                    push_json_pair(out, "kind", "move");
                    out.push(',');
                    push_json_number(out, "fromX", *from_x as u64);
                    out.push(',');
                    push_json_number(out, "fromY", *from_y as u64);
                    out.push(',');
                    push_json_number(out, "toX", *to_x as u64);
                    out.push(',');
                    push_json_number(out, "toY", *to_y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::RemoveMark {
                    x,
                    y,
                    object,
                    mark,
                    ..
                } => {
                    push_json_pair(out, "kind", "remove_mark");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                    out.push(',');
                    push_json_number(out, "mark", mark.0 as u64);
                }
                PatchOp::Add { x, y, object } => {
                    push_json_pair(out, "kind", "add");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::Remove { x, y, object } => {
                    push_json_pair(out, "kind", "remove");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                }
                PatchOp::Replace { x, y, remove, add } => {
                    push_json_pair(out, "kind", "replace");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "remove", remove.0 as u64);
                    out.push(',');
                    push_json_number(out, "add", add.0 as u64);
                }
                PatchOp::SetMark {
                    x,
                    y,
                    object,
                    mark,
                    ..
                } => {
                    push_json_pair(out, "kind", "set_mark");
                    out.push(',');
                    push_json_number(out, "x", *x as u64);
                    out.push(',');
                    push_json_number(out, "y", *y as u64);
                    out.push(',');
                    push_json_number(out, "objectId", object.0 as u64);
                    out.push(',');
                    push_json_number(out, "mark", mark.0 as u64);
                }
                PatchOp::UpdateGlobal { global, .. } => {
                    push_json_pair(out, "kind", "update_global");
                    out.push(',');
                    push_json_number(out, "global", global.0 as u64);
                }
            }
            out.push('}');
        }
        out.push(']');
    }
    out.push(']');
}

fn source_looks_puzzle3d(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
        matches!(
            tokens.as_slice(),
            ["puzzle3", ..] | ["levels3", ..] | ["sprites3", ..]
        )
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
fn solve_solver_task_json_inner(request_json: &str) -> Result<String, AppError> {
    solve_solver_task_json_inner_with_progress(request_json, None::<fn(&str)>)
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
                decode_optional_goal_expr(rules.get("goal"))?,
                decode_optional_goal_expr(rules.get("lose"))?,
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
    let source = required_json_string(request, "source")?;
    let puzzle_path = required_json_string(request, "puzzlePath")?;
    let model_kind = required_json_string(request, "modelKind")?;
    let target = required_json_object(request, "target")?;
    let target_state = required_json_object(target, "state")?;
    let state_data = required_json_value(target_state, "data")?.to_string();
    let max_depth = json_u32_value(request.get("maxDepth"), "maxDepth")?;
    let max_nodes = json_usize_value(request.get("maxNodes"), "maxNodes")?;
    let max_ms = json_u64_value(request.get("maxMs"), "maxMs")?;

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
        ),
        "3d" => solve_request3_json_from_source_inner(
            source,
            target,
            target_state,
            &state_data,
            max_depth,
            max_nodes,
            max_ms,
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
) -> Result<String, AppError> {
    let loaded = parse_game(source)?;
    let level_index = validate_solver_request_level2d(&loaded, target)?;
    let mut state = state_from_json(&loaded, state_json)?;
    if solver_request_materializes_level_start(target_state)? {
        state = materialize_level_start_state(&loaded, state, level_index)?;
    }
    let budget = solver_request_budget(max_depth, max_nodes, max_ms)?;
    let response = solve_current_state_with_budget(&loaded, state, budget)?;
    let mut out = String::new();
    push_solution_response(&mut out, &loaded, &response);
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
) -> Result<String, AppError> {
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
        let outcome =
            transition_program_outcome(engine.game(), engine.game().program(), &state, InputId(0))?;
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
        "global" => Ok(GoalValue::Global(GlobalId(json_u16_value(
            required_json_value(object, "global")?,
            "goal global",
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
            dx: json_i16_value(required_json_value(object, "dx")?, "offset dx")?,
            dy: json_i16_value(required_json_value(object, "dy")?, "offset dy")?,
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
        .ok_or_else(|| AppError::Config("3D solver target requires levels3".to_string()))?;
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
        "compiled-start" | "editor-staged" => {}
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
    let response = solve_current_state_with_budget(&loaded, state, budget)?;
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
    match puzzle_3d::parse_puzzle3d(source) {
        Ok(parsed) => Ok(parsed),
        Err(raw_error) => {
            let document = puzzle_lang::parse_game(source)
                .map_err(|_| AppError::Config(format!("{raw_error:?}")))?;
            document
                .models
                .into_iter()
                .find_map(|model| match model {
                    LoadedDocumentModel::Puzzle3d { puzzle, .. } => Some(puzzle),
                    LoadedDocumentModel::Puzzle2d { .. } => None,
                })
                .ok_or_else(|| AppError::Config(format!("{raw_error:?}")))
        }
    }
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
    let globals = json_i64_array_field(state_json, "globals").unwrap_or_default();
    let fired_rules = json_u64_array_field(state_json, "levelFiredRules").unwrap_or_default();
    let expected_slots = usize::from(width) * usize::from(height) * usize::from(layer_count);
    if slots.len() != expected_slots {
        return Err(AppError::Config(format!(
            "solver state slots length mismatch: expected {expected_slots}, got {}",
            slots.len()
        )));
    }

    let mut state = State::empty_with_globals(
        width,
        height,
        layer_count,
        loaded.game.object_count(),
        globals,
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
        let outcome =
            transition_program_outcome(&loaded.game, loaded.game.program(), &state, InputId(0))?;
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

fn state3_from_json(game: &Game3, state_json: &str) -> Result<State3, AppError> {
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
            .place_object(game, Coord3 { x, y, z }, object)
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
