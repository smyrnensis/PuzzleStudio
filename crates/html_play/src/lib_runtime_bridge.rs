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
        let programs = selected_rule_programs(&self.loaded, program_key, level_index)
            .map_err(|error| error.to_string())?;
        let outcome = puzzle_core::transition_program_sequence_outcome(
            &self.loaded.game,
            state,
            &programs,
            InputId(input),
        )
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
            &outcome.firings,
            include_state,
        )
        .map_err(|error| error.to_string())
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
    let programs = selected_rule_programs(loaded, program_key, level_index)?;
    let outcome = puzzle_core::transition_program_sequence_outcome(
        &loaded.game,
        &state,
        &programs,
        input,
    )?;
    runtime_transition_program_outcome_json(
        loaded,
        &outcome.next_state,
        outcome.cancelled,
        &outcome.commands,
        &outcome.firings,
    )
}

fn selected_rule_programs<'a>(
    loaded: &'a LoadedGame,
    program_key: &str,
    level_index: i32,
) -> Result<Vec<&'a puzzle_core::ExecutableProgram>, AppError> {
    match program_key {
        "main" | "run_rules_on_level_start" => {
            if level_index < 0 {
                return Ok(vec![loaded.game.executable_program()]);
            }
            let index = usize::try_from(level_index)
                .map_err(|_| AppError::Config("main program requires a level index".to_string()))?;
            loaded.programs_for_level(index).ok_or_else(|| {
                AppError::Config(format!("main program level index out of range: {index}"))
            })
        }
        "level_start" => loaded.level_start_program.as_ref().map(|program| vec![program]).ok_or_else(
            || AppError::Config("level_start program is not declared".to_string()),
        ),
        "level_clear" => loaded.level_clear_program.as_ref().map(|program| vec![program]).ok_or_else(
            || AppError::Config("level_clear program is not declared".to_string()),
        ),
        "level_start_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_start_local requires a level index".to_string())
            })?;
            loaded
                .level_start_program_for_level(index)
                .map(|program| vec![program])
                .ok_or_else(|| {
                    AppError::Config(format!(
                        "level_start_local program is not declared for level {index}"
                    ))
                })
        }
        "level_clear_local" => {
            let index = usize::try_from(level_index).map_err(|_| {
                AppError::Config("level_clear_local requires a level index".to_string())
            })?;
            loaded
                .level_clear_program_for_level(index)
                .map(|program| vec![program])
                .ok_or_else(|| {
                    AppError::Config(format!(
                        "level_clear_local program is not declared for level {index}"
                    ))
                })
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
    firings: &[RuleFiring],
) -> Result<String, AppError> {
    let animation_events = animation_events_for_trace(loaded, firings, state);
    RuntimeTransitionProgramOutcome {
        state: state_contract_2d(state),
        cancelled,
        completed: loaded.is_goal_complete(state),
        commands: transition_commands_contract(commands),
        effects: puzzle_play::runtime_effects_for_outcome(&loaded.rule_effects, commands, firings),
        firings: firings_contract_2d(firings),
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
    firings: &[RuleFiring],
    include_state: bool,
) -> Result<String, AppError> {
    let animation_events = animation_events_for_trace(loaded, firings, state);
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
        effects: puzzle_play::runtime_effects_for_outcome(&loaded.rule_effects, commands, firings),
        firings: firings_contract_2d(firings),
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
    RuntimeStateSnapshot::TwoD(RuntimeStateSnapshot2d::from_state(state))
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

fn firings_contract_2d(firings: &[RuleFiring]) -> Vec<RuntimeRuleFiring> {
    firings
        .iter()
        .map(|firing| {
            RuntimeRuleFiring {
                rule_id: firing.rule.0,
                patch: firing
                    .patch
                    .ops()
                    .iter()
                    .map(patch_op_contract_2d)
                    .collect(),
                progressed: firing.progressed,
                observable: firing.observable,
            }
        })
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

fn state_from_json(loaded: &LoadedGame, state_json: &str) -> Result<State, AppError> {
    let snapshot: RuntimeStateSnapshot2d = serde_json::from_str(state_json)
        .map_err(|error| AppError::Config(format!("runtime state contract is invalid: {error}")))?;
    snapshot
        .into_state(&loaded.game)
        .map_err(AppError::Config)
}
