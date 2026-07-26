use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hydrate_render_scene_images(
    render_scene_json: &str,
    image_assets_json: &str,
) -> Result<String, JsValue> {
    let scene: puzzle_runtime_contract::RuntimeResolvedRenderScene =
        serde_json::from_str(render_scene_json).map_err(|error| {
            JsValue::from_str(&format!("invalid resolved render scene: {error}"))
        })?;
    let assets: Vec<puzzle_runtime_contract::RuntimeResolvedImageAsset> =
        serde_json::from_str(image_assets_json).map_err(|error| {
            JsValue::from_str(&format!("invalid decoded image assets: {error}"))
        })?;
    let hydrated = puzzle_presentation::hydrate_external_images(&scene, &assets)
        .map_err(|error| JsValue::from_str(&format!("render image hydration failed: {error:?}")))?;
    serde_json::to_string(&hydrated)
        .map_err(|error| JsValue::from_str(&format!("render scene serialization failed: {error}")))
}

#[wasm_bindgen]
pub fn resolve_render_frame(render_scene_json: &str, elapsed_ms: u32) -> Result<String, JsValue> {
    let scene: puzzle_runtime_contract::RuntimeResolvedRenderScene =
        serde_json::from_str(render_scene_json).map_err(|error| {
            JsValue::from_str(&format!("invalid resolved render scene: {error}"))
        })?;
    let frame = puzzle_presentation::resolve_render_frame(&scene, u64::from(elapsed_ms)).map_err(
        |error| JsValue::from_str(&format!("render frame resolution failed: {error:?}")),
    )?;
    serde_json::to_string(&frame)
        .map_err(|error| JsValue::from_str(&format!("render frame serialization failed: {error}")))
}

#[wasm_bindgen]
pub fn resolve_render_moment(
    render_scene_json: &str,
    render_moment_json: &str,
) -> Result<String, JsValue> {
    let scene: puzzle_runtime_contract::RuntimeResolvedRenderScene =
        serde_json::from_str(render_scene_json).map_err(|error| {
            JsValue::from_str(&format!("invalid resolved render scene: {error}"))
        })?;
    let moment: puzzle_runtime_contract::RuntimeResolvedRenderMoment =
        serde_json::from_str(render_moment_json).map_err(|error| {
            JsValue::from_str(&format!("invalid resolved render moment: {error}"))
        })?;
    let frame = puzzle_presentation::resolve_render_moment(&scene, &moment).map_err(|error| {
        JsValue::from_str(&format!("render moment resolution failed: {error:?}"))
    })?;
    serde_json::to_string(&frame)
        .map_err(|error| JsValue::from_str(&format!("render frame serialization failed: {error}")))
}

#[wasm_bindgen]
pub fn project_renderer_state(
    runtime_export_json: &str,
    state_json: &str,
    level_index: u32,
) -> Result<String, JsValue> {
    let level_index = usize::try_from(level_index)
        .map_err(|_| JsValue::from_str("renderer projection level index is out of range"))?;
    let mut session = puzzle_game_runtime::RuntimeSession::from_export_json(runtime_export_json)
        .map_err(|error| {
            JsValue::from_str(&format!("renderer projection export failed: {error}"))
        })?;
    session
        .set_current_state_json(state_json, level_index, false)
        .map_err(|error| {
            JsValue::from_str(&format!("renderer projection state failed: {error}"))
        })?;
    session
        .renderer_state_json()
        .map_err(|error| JsValue::from_str(&format!("renderer projection failed: {error}")))
}

#[wasm_bindgen]
pub struct WasmStandaloneSession {
    inner: puzzle_game_runtime::RuntimeSession,
}

#[wasm_bindgen]
impl WasmStandaloneSession {
    #[cfg(feature = "source-runtime")]
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, puzzle_path: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: puzzle_game_runtime::RuntimeSession::from_source(source, puzzle_path)
                .map_err(|error| JsValue::from_str(&error))?,
        })
    }

    #[wasm_bindgen(js_name = fromExport)]
    pub fn from_export(export_json: &str) -> Result<WasmStandaloneSession, JsValue> {
        Ok(Self {
            inner: puzzle_game_runtime::RuntimeSession::from_export_json(export_json)
                .map_err(|error| JsValue::from_str(&error))?,
        })
    }

    pub fn snapshot(&mut self) -> String {
        self.inner.snapshot_json()
    }

    pub fn resolve_scene_presentation(
        &self,
        scene_name: &str,
        state_json: &str,
    ) -> Result<String, JsValue> {
        self.inner
            .resolve_scene_presentation_json(scene_name, state_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn dispatch(&mut self, action_json: &str) -> Result<String, JsValue> {
        self.inner
            .dispatch_json(action_json)
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

    pub fn restore_progress_save(&mut self, save_json: &str) -> Result<(), JsValue> {
        self.inner
            .restore_progress_save_json(save_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn set_progress_persistence_enabled(&mut self, enabled: bool) {
        self.inner.set_progress_persistence_enabled(enabled);
    }

    pub fn progress_save_request(&self) -> Option<String> {
        self.inner.progress_save_request().map(|request| {
            serde_json::to_string(&request).expect("progress save request JSON should serialize")
        })
    }

    pub fn confirm_progress_save_written(&mut self, request_id: u32) -> Result<(), JsValue> {
        self.inner
            .confirm_progress_save_written(request_id)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn confirm_progress_save_cleared(&mut self) {
        self.inner.confirm_progress_save_cleared();
    }
}
