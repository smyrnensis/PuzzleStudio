use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmPuzzle3Runtime {
    inner: puzzle_game_runtime::GridRuntimeBridge<3, puzzle_core::Size3>,
}

#[wasm_bindgen]
pub struct WasmStandaloneSession {
    inner: puzzle_game_runtime::StandaloneSessionBridge,
}

#[wasm_bindgen]
impl WasmPuzzle3Runtime {
    #[wasm_bindgen(js_name = fromFixture)]
    pub fn from_fixture(fixture_json: &str) -> Result<WasmPuzzle3Runtime, JsValue> {
        Ok(Self {
            inner: puzzle_game_runtime::GridRuntimeBridge::from_visual_fixture_json(fixture_json)
                .map_err(|error| JsValue::from_str(&error))?,
        })
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
    #[wasm_bindgen(js_name = fromExport)]
    pub fn from_export(export_json: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: puzzle_game_runtime::StandaloneSessionBridge::from_export_json(export_json)
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

    pub fn set_current_state(
        &mut self,
        state_json: &str,
        level_index: u32,
        materialize_level_start: bool,
    ) -> Result<(), JsValue> {
        let level_index = usize::try_from(level_index)
            .map_err(|_| JsValue::from_str("level index is out of range"))?;
        self.inner
            .set_current_state_json(state_json, level_index, materialize_level_start)
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
