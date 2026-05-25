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

    pub fn is_complete(&self, state_json: &str) -> Result<bool, JsValue> {
        self.inner
            .is_complete_json(state_json)
            .map_err(|error| JsValue::from_str(&error))
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
