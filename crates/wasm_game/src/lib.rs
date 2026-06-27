use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmPuzzle3Runtime {
    inner: puzzle_game_runtime::Puzzle3RuntimeBridge,
}

#[wasm_bindgen]
pub struct WasmStandaloneSession {
    inner: WasmStandaloneSessionInner,
}

enum WasmStandaloneSessionInner {
    Source(puzzle_game_runtime::StandaloneSessionBridge),
    Export(puzzle_game_runtime::StandaloneSessionBridge),
}

#[wasm_bindgen]
impl WasmPuzzle3Runtime {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmPuzzle3Runtime, JsValue> {
        let _ = puzzle_path;
        Ok(Self {
            inner: puzzle_game_runtime::Puzzle3RuntimeBridge::from_source(source)
                .map_err(|error| JsValue::from_str(&error))?,
        })
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

    pub fn set_state(&mut self, state_json: &str) -> Result<(), JsValue> {
        self.inner
            .set_state_json(state_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn current_cells(&self) -> Result<String, JsValue> {
        self.inner
            .current_cells_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn is_current_complete(&self) -> Result<bool, JsValue> {
        self.inner
            .is_current_complete()
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
}

#[wasm_bindgen]
impl WasmStandaloneSession {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: WasmStandaloneSessionInner::Source(
                puzzle_game_runtime::StandaloneSessionBridge::from_source(source, puzzle_path)
                    .map_err(|error| JsValue::from_str(&error))?,
            ),
        })
    }

    #[wasm_bindgen(js_name = fromExport)]
    pub fn from_export(export_json: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: WasmStandaloneSessionInner::Export(
                puzzle_game_runtime::StandaloneSessionBridge::from_export_json(export_json)
                    .map_err(|error| JsValue::from_str(&error))?,
            ),
        })
    }

    pub fn snapshot(&mut self) -> String {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.snapshot_json(),
            WasmStandaloneSessionInner::Export(inner) => inner.snapshot_json(),
        }
    }

    pub fn request_json(&mut self, method: &str, url: &str) -> Result<String, JsValue> {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.request_json(method, url),
            WasmStandaloneSessionInner::Export(inner) => inner.request_json(method, url),
        }
        .map_err(|error| JsValue::from_str(&error))
    }

    pub fn apply_input_name(&mut self, input_name: &str) -> Result<(), JsValue> {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.apply_input_name(input_name),
            WasmStandaloneSessionInner::Export(inner) => inner.apply_input_name(input_name),
        }
        .map_err(|error| JsValue::from_str(&error))
    }

    pub fn apply_command_name(&mut self, command_name: &str) -> Result<(), JsValue> {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.apply_command_name(command_name),
            WasmStandaloneSessionInner::Export(inner) => inner.apply_command_name(command_name),
        }
        .map_err(|error| JsValue::from_str(&error))
    }

    pub fn progress_save(&self) -> String {
        match &self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.progress_save_json(),
            WasmStandaloneSessionInner::Export(inner) => inner.progress_save_json(),
        }
    }

    pub fn restore_progress_save(&mut self, save_json: &str) -> Result<(), JsValue> {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => {
                inner.restore_progress_save_json(save_json)
            }
            WasmStandaloneSessionInner::Export(inner) => {
                inner.restore_progress_save_json(save_json)
            }
        }
        .map_err(|error| JsValue::from_str(&error))
    }

    pub fn mark_progress_save_written(&mut self) {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.mark_progress_save_written(),
            WasmStandaloneSessionInner::Export(inner) => inner.mark_progress_save_written(),
        }
    }

    pub fn clear_progress_save(&mut self) {
        match &mut self.inner {
            WasmStandaloneSessionInner::Source(inner) => inner.clear_progress_save(),
            WasmStandaloneSessionInner::Export(inner) => inner.clear_progress_save(),
        }
    }
}
