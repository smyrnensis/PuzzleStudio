use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Function")]
    pub type JsFunction;

    #[wasm_bindgen(method, catch, structural, js_name = call)]
    fn call1(this: &JsFunction, this_arg: &JsValue, arg: &JsValue) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
pub struct WasmCoreRuntime {
    inner: html_play::CoreRuntimeBridge,
}

#[wasm_bindgen]
pub struct WasmPuzzle3Runtime {
    inner: html_play::Puzzle3RuntimeBridge,
}

#[wasm_bindgen]
pub struct WasmStandaloneSession {
    inner: html_play::StandaloneSessionBridge,
}

#[wasm_bindgen]
impl WasmCoreRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmCoreRuntime, JsValue> {
        let _ = puzzle_path;
        Ok(Self {
            inner: html_play::CoreRuntimeBridge::from_source(source)
                .map_err(|error| JsValue::from_str(&error))?,
        })
    }

    pub fn transition_program_outcome(
        &self,
        program_key: &str,
        level_index: i32,
        state_json: &str,
        input: u16,
    ) -> Result<String, JsValue> {
        self.inner
            .transition_program_outcome_json(program_key, level_index, state_json, input)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn set_state(&mut self, state_json: &str) -> Result<(), JsValue> {
        self.inner
            .set_state_json(state_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_state(&self) -> Result<String, JsValue> {
        self.inner
            .current_state_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_state_hash(&self) -> Result<String, JsValue> {
        self.inner
            .current_state_hash_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_cells(&self) -> Result<String, JsValue> {
        self.inner
            .current_cells_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn save_current_state(&mut self) -> Result<u32, JsValue> {
        self.inner
            .save_current_state()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), JsValue> {
        self.inner
            .restore_saved_state(handle)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn transition_current_outcome(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, JsValue> {
        self.inner
            .transition_current_outcome_json(program_key, level_index, input)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn transition_current_state_outcome(
        &mut self,
        program_key: &str,
        level_index: i32,
        input: u16,
    ) -> Result<String, JsValue> {
        self.inner
            .transition_current_state_outcome_json(program_key, level_index, input)
            .map_err(|error| JsValue::from_str(&error))
    }
}

#[wasm_bindgen]
impl WasmPuzzle3Runtime {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmPuzzle3Runtime, JsValue> {
        let _ = puzzle_path;
        Ok(Self {
            inner: html_play::Puzzle3RuntimeBridge::from_source(source)
                .map_err(|error| JsValue::from_str(&error))?,
        })
    }

    pub fn transition_program_outcome(
        &self,
        program_key: &str,
        state_json: &str,
        input: u16,
    ) -> Result<String, JsValue> {
        self.inner
            .transition_program_outcome_json(program_key, state_json, input)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn set_state(&mut self, state_json: &str) -> Result<(), JsValue> {
        self.inner
            .set_state_json(state_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_state(&self) -> Result<String, JsValue> {
        self.inner
            .current_state_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_cells(&self) -> Result<String, JsValue> {
        self.inner
            .current_cells_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn save_current_state(&mut self) -> Result<u32, JsValue> {
        self.inner
            .save_current_state()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn restore_saved_state(&mut self, handle: u32) -> Result<(), JsValue> {
        self.inner
            .restore_saved_state(handle)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn transition_current_outcome(
        &mut self,
        program_key: &str,
        input: u16,
    ) -> Result<String, JsValue> {
        self.inner
            .transition_current_outcome_json(program_key, input)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn is_complete(&self, state_json: &str) -> Result<bool, JsValue> {
        self.inner
            .is_complete_json(state_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn is_current_complete(&self) -> Result<bool, JsValue> {
        self.inner
            .is_current_complete()
            .map_err(|error| JsValue::from_str(&error))
    }
}

#[wasm_bindgen]
impl WasmStandaloneSession {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: html_play::StandaloneSessionBridge::from_source(source, puzzle_path)
                .map_err(|error| JsValue::from_str(&error))?,
        })
    }

    pub fn snapshot(&mut self) -> String {
        self.inner.snapshot_json()
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, JsValue> {
        self.inner
            .request_json(method, url)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), JsValue> {
        self.inner
            .apply_input_name(input_name)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), JsValue> {
        self.inner
            .apply_command_name(command_name)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn progress_save(&self) -> String {
        self.inner.progress_save_json()
    }

    pub fn restore_progress_save(&mut self, save_json: &str) -> Result<(), JsValue> {
        self.inner
            .restore_progress_save_json(save_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn mark_progress_save_written(&mut self) {
        self.inner.mark_progress_save_written();
    }

    pub fn clear_progress_save(&mut self) {
        self.inner.clear_progress_save();
    }
}

#[wasm_bindgen]
pub fn transition_program_outcome(
    source: &str,
    program_key: &str,
    level_index: i32,
    state_json: &str,
    input: u16,
) -> Result<String, JsValue> {
    html_play::transition_program_outcome_json_from_source(
        source,
        program_key,
        level_index,
        state_json,
        input,
    )
    .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn compile_preview(
    source: &str,
    puzzle_path: &str,
    game_css: &str,
    game_visuals_js: &str,
) -> Result<String, JsValue> {
    let path = if puzzle_path.trim().is_empty() {
        "game.puzzle"
    } else {
        puzzle_path
    };
    html_play::export_html_from_source(source, path, game_css, game_visuals_js)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn generate_visuals_js(source: &str, base_visuals_js: &str) -> Result<String, JsValue> {
    html_play::export_visuals_js_from_source(source, base_visuals_js)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn highlight_source_html(source: &str) -> String {
    puzzle_lang::highlight_source(source).html
}

#[wasm_bindgen]
pub fn translate_puzzlescript(source: &str) -> Result<String, JsValue> {
    puzzle_lang::translate_puzzlescript_to_canonical(source)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn suggest_source_completions(source: &str, cursor_offset: usize) -> String {
    puzzle_lang::completion_list_json(&puzzle_lang::suggest_source_completions(
        source,
        cursor_offset,
    ))
}

#[wasm_bindgen]
pub fn resolve_source_target(source: &str, cursor_offset: usize) -> String {
    let target = puzzle_lang::resolve_source_target(source, cursor_offset);
    puzzle_lang::source_target_json(target.as_ref())
}

#[wasm_bindgen]
pub fn solve_state(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: u32,
    max_ms: u32,
) -> Result<String, JsValue> {
    html_play::solve_state_json_from_source(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes as usize,
        u64::from(max_ms),
    )
    .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen]
pub fn solve_state_with_progress(
    source: &str,
    puzzle_path: &str,
    state_json: &str,
    max_depth: u32,
    max_nodes: u32,
    max_ms: u32,
    progress_interval_ms: u32,
    progress_callback: JsFunction,
) -> Result<String, JsValue> {
    html_play::solve_state_json_from_source_with_progress(
        source,
        puzzle_path,
        state_json,
        max_depth,
        max_nodes as usize,
        u64::from(max_ms),
        u64::from(progress_interval_ms),
        |progress_json| {
            let _ = progress_callback.call1(&JsValue::NULL, &JsValue::from_str(&progress_json));
        },
    )
    .map_err(|error| JsValue::from_str(&error))
}
