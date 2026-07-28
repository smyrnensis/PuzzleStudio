#[cfg(target_arch = "wasm32")]
use bevy::{
    app::{PostUpdate, Update},
    prelude::{IntoScheduleConfigs, Local, NonSend, NonSendMut, Res, Time},
};
#[cfg(target_arch = "wasm32")]
use puzzle_runtime_contract::RuntimeProgressPersistenceOperation;
#[cfg(any(target_arch = "wasm32", test))]
use puzzle_runtime_contract::StandaloneProgressStorage;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(any(target_arch = "wasm32", test))]
const STORAGE_RETRY_SECONDS: f64 = 5.0;
#[cfg(any(target_arch = "wasm32", test))]
const FATAL_DIAGNOSTIC_ELEMENT_ID: &str = "puzzle-bevy-fatal";
#[cfg(any(target_arch = "wasm32", test))]
const NONFATAL_DIAGNOSTIC_ELEMENT_ID: &str = "puzzle-bevy-diagnostic";
#[cfg(any(target_arch = "wasm32", test))]
const PLAYER_STATUS_ELEMENT_ID: &str = "puzzle-bevy-status";
#[cfg(any(target_arch = "wasm32", test))]
const FATAL_DIAGNOSTIC_VISIBLE_STYLE: &str = "display:block";

#[cfg(any(target_arch = "wasm32", test))]
struct BrowserProgressStorage {
    identity: StandaloneProgressStorage,
    #[cfg(target_arch = "wasm32")]
    storage: web_sys::Storage,
    failed_request_id: Option<u32>,
    next_retry_seconds: f64,
}

#[cfg(any(target_arch = "wasm32", test))]
impl BrowserProgressStorage {
    #[cfg(target_arch = "wasm32")]
    fn open(identity: StandaloneProgressStorage) -> Result<Self, String> {
        let window = web_sys::window()
            .ok_or_else(|| storage_read_failure(&identity, "browser window is unavailable"))?;
        let storage = window
            .local_storage()
            .map_err(|error| storage_read_failure(&identity, &js_error(&error)))?
            .ok_or_else(|| storage_read_failure(&identity, "localStorage is unavailable"))?;
        Ok(Self {
            identity,
            storage,
            failed_request_id: None,
            next_retry_seconds: 0.0,
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn read(&self) -> Result<Option<String>, String> {
        self.storage
            .get_item(&self.identity.key)
            .map_err(|error| storage_read_failure(&self.identity, &js_error(&error)))
    }

    #[cfg(target_arch = "wasm32")]
    fn apply(&self, operation: &RuntimeProgressPersistenceOperation) -> Result<(), String> {
        match operation {
            RuntimeProgressPersistenceOperation::Write { save_json } => self
                .storage
                .set_item(&self.identity.key, save_json)
                .map_err(|error| storage_write_failure(&self.identity, &js_error(&error))),
            RuntimeProgressPersistenceOperation::Delete => self
                .storage
                .remove_item(&self.identity.key)
                .map_err(|error| storage_delete_failure(&self.identity, &js_error(&error))),
        }
    }

    fn should_attempt(&self, request_id: u32, now_seconds: f64) -> bool {
        self.failed_request_id != Some(request_id) || now_seconds >= self.next_retry_seconds
    }

    fn mark_failed(&mut self, request_id: u32, now_seconds: f64) {
        self.failed_request_id = Some(request_id);
        self.next_retry_seconds = now_seconds + STORAGE_RETRY_SECONDS;
    }

    fn mark_applied(&mut self) -> bool {
        let recovered = self.failed_request_id.take().is_some();
        self.next_retry_seconds = 0.0;
        recovered
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = startStandalonePlayer)]
pub fn start_standalone_player(export_json: &str, canvas_selector: &str) -> Result<(), JsValue> {
    let decoded = puzzle_player_bootstrap::decode_standalone_player_export(export_json)
        .map_err(|error| js_diagnostic(format!("standalone player export is invalid: {error}")))?;
    let (mut runtime, visual_images, progress_identity) = decoded.into_parts();
    let progress_storage =
        BrowserProgressStorage::open(progress_identity).map_err(js_diagnostic)?;

    if let Some(save_json) = progress_storage.read().map_err(js_diagnostic)? {
        runtime
            .restore_progress_save_json(&save_json)
            .map_err(|error| {
                js_diagnostic(storage_restore_failure(&progress_storage.identity, &error))
            })?;
    }

    let host = puzzle_bevy_player::PuzzleBevyPlayerHost::from_runtime_with_visual_images(
        runtime,
        visual_images,
    )
    .map_err(|error| js_diagnostic(format!("Bevy player initialization failed: {error}")))?;

    let mut app =
        puzzle_bevy_player::build_browser_player_app(host, canvas_selector, "PuzzleStudio")
            .map_err(js_diagnostic)?;
    app.insert_non_send(progress_storage).add_systems(
        Update,
        (persist_pending_progress, surface_player_fatal_diagnostic),
    );
    app.add_systems(
        PostUpdate,
        surface_player_observation
            .in_set(puzzle_bevy_player::PuzzleBevyPlayerSystems::ObservationReady),
    );
    app.run();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn persist_pending_progress(
    time: bevy::prelude::Res<Time>,
    mut host: NonSendMut<puzzle_bevy_player::PuzzleBevyPlayerHost>,
    mut storage: NonSendMut<BrowserProgressStorage>,
) {
    let Some(request) = host.pending_progress_save() else {
        if storage.mark_applied() {
            clear_nonfatal_diagnostic();
        }
        return;
    };
    let now_seconds = time.elapsed_secs_f64();
    if !storage.should_attempt(request.request_id, now_seconds) {
        return;
    }

    if let Err(error) = storage.apply(&request.operation) {
        storage.mark_failed(request.request_id, now_seconds);
        report_nonfatal_diagnostic(&format!(
            "{error} The typed save request remains pending and will be retried."
        ));
        return;
    }

    if let Err(error) = host.confirm_progress_persistence_applied(request.request_id, now_seconds) {
        storage.mark_failed(request.request_id, now_seconds);
        report_nonfatal_diagnostic(&format!(
            "Progress persistence operation for `{}` was applied, but exact acknowledgement {} failed: {error}",
            storage.identity.key, request.request_id
        ));
        return;
    }
    if storage.mark_applied() {
        clear_nonfatal_diagnostic();
    }
}

#[cfg(target_arch = "wasm32")]
fn surface_player_fatal_diagnostic(
    host: NonSend<puzzle_bevy_player::PuzzleBevyPlayerHost>,
    mut last_reported: Local<Option<String>>,
) {
    let Some(message) = host.fatal_error() else {
        return;
    };
    if last_reported.as_deref() == Some(message) {
        return;
    }
    report_fatal_diagnostic(message);
    *last_reported = Some(message.to_string());
}

#[cfg(target_arch = "wasm32")]
fn surface_player_observation(
    host: NonSend<puzzle_bevy_player::PuzzleBevyPlayerHost>,
    observation: Res<puzzle_bevy_player::PuzzleBevyPlayerObservationState>,
    mut last_submission_sequence: Local<u64>,
) {
    if host.fatal_error().is_some() {
        return;
    }
    let Some(latest) = observation.latest() else {
        return;
    };
    if *last_submission_sequence == latest.submission_sequence {
        return;
    }
    *last_submission_sequence = latest.submission_sequence;
    if let Err(error) = write_player_observation(latest) {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Player observation surface failed: {error}"
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn write_player_observation(
    observation: &puzzle_bevy_player::PuzzleBevyPlayerObservation,
) -> Result<(), String> {
    let element = player_status_element()?;
    if element.get_attribute("data-state").as_deref() == Some("fatal") {
        return Ok(());
    }
    for (name, value) in [
        ("data-state", "ready".to_string()),
        ("data-sequence", observation.sequence.to_string()),
        (
            "data-submission-sequence",
            observation.submission_sequence.to_string(),
        ),
        ("data-revision", observation.revision.to_string()),
        ("data-surface-focus", observation.surface_focus.clone()),
        (
            "data-viewport-count",
            observation.viewport_count.to_string(),
        ),
        (
            "data-submission-interval-micros",
            observation.submission_interval_micros.to_string(),
        ),
        (
            "data-presentation-cpu-micros",
            observation
                .presentation_cpu_micros
                .map_or_else(String::new, |value| value.to_string()),
        ),
        (
            "data-wasm-linear-memory-bytes",
            observation
                .wasm_linear_memory_bytes
                .map_or_else(String::new, |value| value.to_string()),
        ),
        (
            "data-progress-fingerprint",
            observation.progress_fingerprint.to_string(),
        ),
        (
            "data-audio-capability",
            observation.audio_capability_label().to_string(),
        ),
    ] {
        element.set_attribute(name, &value).map_err(|error| {
            format!(
                "could not write `{name}` on `#{PLAYER_STATUS_ELEMENT_ID}`: {}",
                js_error(&error)
            )
        })?;
    }
    Ok(())
}

#[cfg(any(target_arch = "wasm32", test))]
fn storage_read_failure(identity: &StandaloneProgressStorage, cause: &str) -> String {
    format!(
        "Progress save could not be read for `{}` (save version {}). Saved progress was not modified. {cause}",
        identity.key, identity.save_version
    )
}

#[cfg(any(target_arch = "wasm32", test))]
fn storage_restore_failure(identity: &StandaloneProgressStorage, cause: &str) -> String {
    format!(
        "Progress save could not be restored for `{}` (save version {}). The saved value was kept and startup was aborted. {cause}",
        identity.key, identity.save_version
    )
}

#[cfg(target_arch = "wasm32")]
fn storage_write_failure(identity: &StandaloneProgressStorage, cause: &str) -> String {
    format!(
        "Progress save could not be written for `{}` (save version {}). {cause}",
        identity.key, identity.save_version
    )
}

#[cfg(target_arch = "wasm32")]
fn storage_delete_failure(identity: &StandaloneProgressStorage, cause: &str) -> String {
    format!(
        "Progress save could not be deleted for `{}` (save version {}). {cause}",
        identity.key, identity.save_version
    )
}

#[cfg(target_arch = "wasm32")]
fn js_error(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(target_arch = "wasm32")]
fn js_diagnostic(message: impl Into<String>) -> JsValue {
    let message = message.into();
    report_fatal_diagnostic(&message);
    JsValue::from_str(&message)
}

#[cfg(target_arch = "wasm32")]
fn report_fatal_diagnostic(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
    if let Err(surface_error) = write_player_status_state("fatal") {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Player status surface failed: {surface_error}"
        )));
    }
    if let Err(surface_error) =
        write_existing_browser_diagnostic(FATAL_DIAGNOSTIC_ELEMENT_ID, message)
    {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Player fatal diagnostic surface failed: {surface_error}"
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn write_player_status_state(state: &str) -> Result<(), String> {
    player_status_element()?
        .set_attribute("data-state", state)
        .map_err(|error| {
            format!(
                "could not write `data-state` on `#{PLAYER_STATUS_ELEMENT_ID}`: {}",
                js_error(&error)
            )
        })
}

#[cfg(target_arch = "wasm32")]
fn player_status_element() -> Result<web_sys::Element, String> {
    browser_document()?
        .get_element_by_id(PLAYER_STATUS_ELEMENT_ID)
        .ok_or_else(|| format!("required browser element `#{PLAYER_STATUS_ELEMENT_ID}` is missing"))
}

#[cfg(target_arch = "wasm32")]
fn report_nonfatal_diagnostic(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
    if let Err(surface_error) = write_nonfatal_browser_diagnostic(message) {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Player nonfatal diagnostic surface failed: {surface_error}"
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn write_existing_browser_diagnostic(element_id: &str, message: &str) -> Result<(), String> {
    let document = browser_document()?;
    let element = document
        .get_element_by_id(element_id)
        .ok_or_else(|| format!("required browser element `#{element_id}` is missing"))?;
    element.set_text_content(Some(message));
    element
        .set_attribute("style", FATAL_DIAGNOSTIC_VISIBLE_STYLE)
        .map_err(|error| {
            format!(
                "could not make `#{element_id}` visible: {}",
                js_error(&error)
            )
        })?;
    element
        .remove_attribute("hidden")
        .map_err(|error| format!("could not show `#{element_id}`: {}", js_error(&error)))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn write_nonfatal_browser_diagnostic(message: &str) -> Result<(), String> {
    let document = browser_document()?;
    let element = if let Some(element) = document.get_element_by_id(NONFATAL_DIAGNOSTIC_ELEMENT_ID)
    {
        element
    } else {
        let element = document.create_element("pre").map_err(|error| {
            format!("could not create browser diagnostic: {}", js_error(&error))
        })?;
        element.set_id(NONFATAL_DIAGNOSTIC_ELEMENT_ID);
        element.set_attribute("role", "status").map_err(|error| {
            format!(
                "could not configure browser diagnostic: {}",
                js_error(&error)
            )
        })?;
        element
            .set_attribute("aria-live", "polite")
            .map_err(|error| {
                format!(
                    "could not configure browser diagnostic: {}",
                    js_error(&error)
                )
            })?;
        element
            .set_attribute(
                "style",
                "position:fixed;left:1rem;right:1rem;bottom:1rem;z-index:2147483647;\
                 margin:0;padding:.75rem 1rem;white-space:pre-wrap;\
                 background:#302400;color:#ffe9a8;border:1px solid #d3a928;\
                 font:13px/1.4 ui-monospace,SFMono-Regular,Menlo,monospace",
            )
            .map_err(|error| format!("could not style browser diagnostic: {}", js_error(&error)))?;
        document
            .body()
            .ok_or_else(|| "browser document has no body for the nonfatal diagnostic".to_string())?
            .append_child(&element)
            .map_err(|error| format!("could not mount browser diagnostic: {}", js_error(&error)))?;
        element
    };
    element.set_text_content(Some(message));
    element
        .remove_attribute("hidden")
        .map_err(|error| format!("could not show browser diagnostic: {}", js_error(&error)))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn clear_nonfatal_diagnostic() {
    let Ok(document) = browser_document() else {
        return;
    };
    let Some(element) = document.get_element_by_id(NONFATAL_DIAGNOSTIC_ELEMENT_ID) else {
        return;
    };
    element.set_text_content(None);
    if let Err(error) = element.set_attribute("hidden", "") {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Player nonfatal diagnostic could not be cleared: {}",
            js_error(&error)
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_document() -> Result<web_sys::Document, String> {
    web_sys::window()
        .ok_or_else(|| "browser window is unavailable".to_string())?
        .document()
        .ok_or_else(|| "browser document is unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> StandaloneProgressStorage {
        StandaloneProgressStorage {
            key: "puzzlestudio:progress:game-id:v7".to_string(),
            save_version: 7,
        }
    }

    #[test]
    fn storage_namespace_is_the_export_owned_key_without_adapter_rewriting() {
        let storage = BrowserProgressStorage {
            identity: identity(),
            failed_request_id: None,
            next_retry_seconds: 0.0,
        };

        assert_eq!(storage.identity.key, "puzzlestudio:progress:game-id:v7");
        assert_eq!(storage.identity.save_version, 7);
    }

    #[test]
    fn startup_diagnostics_preserve_the_key_version_and_saved_value() {
        let read = storage_read_failure(&identity(), "permission denied");
        let restore = storage_restore_failure(&identity(), "invalid checksum");

        assert!(read.contains("puzzlestudio:progress:game-id:v7"));
        assert!(read.contains("save version 7"));
        assert!(read.contains("Saved progress was not modified"));
        assert!(restore.contains("The saved value was kept and startup was aborted"));
    }

    #[test]
    fn failed_write_retains_the_request_and_retries_after_the_typed_interval() {
        let mut storage = BrowserProgressStorage {
            identity: identity(),
            failed_request_id: None,
            next_retry_seconds: 0.0,
        };

        assert!(storage.should_attempt(4, 10.0));
        storage.mark_failed(4, 10.0);
        assert!(!storage.should_attempt(4, 14.999));
        assert!(storage.should_attempt(4, 15.0));
        assert!(storage.should_attempt(5, 10.1));
        assert!(storage.mark_applied());
        assert!(!storage.mark_applied());
    }

    #[test]
    fn fatal_and_nonfatal_browser_diagnostics_have_separate_owned_surfaces() {
        assert_eq!(FATAL_DIAGNOSTIC_ELEMENT_ID, "puzzle-bevy-fatal");
        assert_eq!(NONFATAL_DIAGNOSTIC_ELEMENT_ID, "puzzle-bevy-diagnostic");
        assert_eq!(PLAYER_STATUS_ELEMENT_ID, "puzzle-bevy-status");
        assert_ne!(
            FATAL_DIAGNOSTIC_ELEMENT_ID, NONFATAL_DIAGNOSTIC_ELEMENT_ID,
            "recoverable persistence failures must not occupy the player fatal surface"
        );
        assert_ne!(PLAYER_STATUS_ELEMENT_ID, FATAL_DIAGNOSTIC_ELEMENT_ID);
        assert_ne!(PLAYER_STATUS_ELEMENT_ID, NONFATAL_DIAGNOSTIC_ELEMENT_ID);
        assert!(
            FATAL_DIAGNOSTIC_VISIBLE_STYLE.contains("display:block"),
            "the official fatal element is hidden by stylesheet display:none, so the adapter must override display"
        );
    }
}
