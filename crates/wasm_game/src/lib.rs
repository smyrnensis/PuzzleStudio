//! Editor-only browser host for the canonical Bevy player.
//!
//! This artifact intentionally does not expose a parallel session or renderer.
//! Editor controls enter through a request queue; the Bevy app applies them to
//! `PuzzleBevyPlayerHost`, and the normal player systems render the resulting
//! typed player snapshot.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use bevy::prelude::{NonSend, NonSendMut, Res, Time};
use puzzle_bevy_player::PuzzleBevyPlayerHost;
use puzzle_editor_preview_contract::{EditorPreviewControlRequest, EditorPreviewObservation};
use serde_json::{Map, Value};

#[cfg(target_arch = "wasm32")]
use bevy::{
    app::{App, Update},
    prelude::{DefaultPlugins, PluginGroup},
    window::{Window, WindowPlugin},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
struct QueuedEditorPreviewObservation {
    required_revision: Option<u64>,
    observation: EditorPreviewObservation,
}

#[derive(Clone, Default)]
struct EditorPreviewBridge {
    requests: Rc<RefCell<VecDeque<EditorPreviewControlRequest>>>,
    observations: Rc<RefCell<VecDeque<QueuedEditorPreviewObservation>>>,
    ready_pending: Rc<RefCell<bool>>,
}

struct EditorPreviewDocument(puzzle_lang::LoadedDocument);

impl EditorPreviewBridge {
    fn submit_json(&self, request_json: &str) -> Result<(), String> {
        let request: EditorPreviewControlRequest = serde_json::from_str(request_json)
            .map_err(|error| format!("invalid editor preview control request: {error}"))?;
        request
            .validate()
            .map_err(|error| format!("invalid editor preview control request: {error}"))?;
        self.requests.borrow_mut().push_back(request);
        Ok(())
    }
}

fn inspect_host(host: &PuzzleBevyPlayerHost) -> Result<Value, String> {
    puzzle_presentation_json::to_editor_preview_state_value(&host.editor_development_snapshot())
        .map_err(|error| format!("editor development snapshot serialization failed: {error}"))
}

fn editor_state_fields(state: Value) -> Result<Map<String, Value>, String> {
    state
        .as_object()
        .cloned()
        .ok_or_else(|| "editor preview state projection must be a JSON object".to_string())
}

fn apply_editor_control(
    host: &mut PuzzleBevyPlayerHost,
    document: &puzzle_lang::LoadedDocument,
    request: EditorPreviewControlRequest,
    now_seconds: f64,
) -> EditorPreviewObservation {
    let command_id = request.command_id();
    let result = match request {
        EditorPreviewControlRequest::HydrateState {
            state,
            level_index,
            materialize_level_start,
            ..
        } => usize::try_from(level_index)
            .map_err(|_| "editor preview level index is out of range".to_string())
            .and_then(|level_index| {
                host.hydrate_editor_state(
                    &state.to_string(),
                    level_index,
                    materialize_level_start,
                    now_seconds,
                )
                .map_err(|error| error.to_string())
            })
            .and_then(|()| inspect_host(host))
            .map(|snapshot| (snapshot, None)),
        EditorPreviewControlRequest::HydrateDraft {
            model,
            level_index,
            draft,
            presentation: _,
            ..
        } => usize::try_from(level_index)
            .map_err(|_| "editor preview level index is out of range".to_string())
            .and_then(|level_index| {
                puzzle_lang::resolve_editor_draft(document, &model, level_index, &draft)
                    .map(|state| (state, level_index))
            })
            .and_then(|(state, level_index)| {
                serde_json::to_string(&state)
                    .map_err(|error| format!("editor draft state serialization failed: {error}"))
                    .map(|state| (state, level_index))
            })
            .and_then(|(state, level_index)| {
                host.hydrate_editor_state(&state, level_index, false, now_seconds)
                    .map_err(|error| error.to_string())
            })
            .and_then(|()| inspect_host(host))
            .map(|snapshot| (snapshot, None)),
        EditorPreviewControlRequest::SyntheticKey {
            key,
            alt_key,
            ctrl_key,
            meta_key,
            code,
            repeat,
            shift_key,
            trace,
            ..
        } => {
            let _physical_metadata = (code, repeat, shift_key);
            let trigger = (!alt_key && !ctrl_key && !meta_key)
                .then(|| puzzle_bevy_player::runtime_key_trigger_for_logical_key(&key))
                .flatten();
            match trigger {
                Some(trigger) => host
                    .dispatch_editor_key(trigger, trace, now_seconds)
                    .map_err(|error| error.to_string())
                    .and_then(|trace| inspect_host(host).map(|snapshot| (snapshot, trace))),
                None => inspect_host(host).map(|snapshot| (snapshot, None)),
            }
        }
        EditorPreviewControlRequest::RequestSnapshot { .. } => {
            inspect_host(host).map(|snapshot| (snapshot, None))
        }
    };

    match result {
        Ok((state, Some(trace))) => EditorPreviewObservation::DebugTrace {
            command_id,
            debug: trace,
            snapshot: state,
        },
        Ok((state, None)) => match editor_state_fields(state) {
            Ok(state) => EditorPreviewObservation::State {
                command_id: Some(command_id),
                state,
            },
            Err(message) => EditorPreviewObservation::RuntimeError {
                command_id,
                label: "state projection failed",
                message,
            },
        },
        Err(message) => EditorPreviewObservation::RuntimeError {
            command_id,
            label: "runtime failed",
            message,
        },
    }
}

fn process_editor_preview_controls(
    time: Res<Time>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
    document: NonSend<EditorPreviewDocument>,
    bridge: NonSendMut<EditorPreviewBridge>,
    committed: Res<puzzle_bevy_player::PuzzleBevyPlayerObservationState>,
) {
    if *bridge.ready_pending.borrow() && committed.latest().is_some() {
        *bridge.ready_pending.borrow_mut() = false;
        let observations = match inspect_host(&host).and_then(editor_state_fields) {
            Ok(state) => vec![
                EditorPreviewObservation::RuntimeReady,
                EditorPreviewObservation::State {
                    command_id: None,
                    state,
                },
            ],
            Err(message) => vec![EditorPreviewObservation::RuntimeError {
                command_id: 0,
                label: "state projection failed",
                message,
            }],
        };
        bridge
            .observations
            .borrow_mut()
            .extend(
                observations
                    .into_iter()
                    .map(|observation| QueuedEditorPreviewObservation {
                        required_revision: Some(host.snapshot().revision),
                        observation,
                    }),
            );
    }
    let requests = bridge.requests.borrow_mut().drain(..).collect::<Vec<_>>();
    let now_seconds = time.elapsed_secs_f64();
    let observations = requests.into_iter().map(|request| {
        let observation = apply_editor_control(&mut host, &document.0, request, now_seconds);
        let required_revision =
            if matches!(observation, EditorPreviewObservation::RuntimeError { .. }) {
                None
            } else {
                Some(host.snapshot().revision)
            };
        QueuedEditorPreviewObservation {
            required_revision,
            observation,
        }
    });
    bridge.observations.borrow_mut().extend(observations);
    #[cfg(target_arch = "wasm32")]
    dispatch_committed_observations(&bridge, committed.latest().map(|value| value.revision));
}

fn take_committed_observations(
    bridge: &EditorPreviewBridge,
    committed_revision: Option<u64>,
) -> Vec<EditorPreviewObservation> {
    let mut pending = bridge.observations.borrow_mut();
    let mut retained = VecDeque::new();
    let mut ready = Vec::new();
    while let Some(queued) = pending.pop_front() {
        let committed = queued.required_revision.is_none()
            || committed_revision.is_some_and(|revision| {
                queued
                    .required_revision
                    .is_some_and(|required| revision >= required)
            });
        if committed {
            ready.push(queued.observation);
        } else {
            retained.push_back(queued);
        }
    }
    *pending = retained;
    ready
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static ACTIVE_EDITOR_PREVIEW_BRIDGE: RefCell<Option<EditorPreviewBridge>> =
        const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = startEditorPreview)]
pub fn start_editor_preview(export_json: &str, canvas_selector: &str) -> Result<(), JsValue> {
    validate_canvas_selector(canvas_selector).map_err(js_error)?;
    let decoded = puzzle_player_bootstrap::decode_editor_preview_export(export_json)
        .map_err(|error| js_error(format!("editor preview export is invalid: {error}")))?;
    let (mut runtime, visual_images, _, document) = decoded.into_parts();
    runtime.set_progress_persistence_enabled(false);
    let host = PuzzleBevyPlayerHost::from_runtime_with_visual_images(runtime, visual_images)
        .map_err(|error| js_error(format!("editor preview initialization failed: {error}")))?;
    let bridge = EditorPreviewBridge::default();
    *bridge.ready_pending.borrow_mut() = true;
    ACTIVE_EDITOR_PREVIEW_BRIDGE.with(|active| {
        let mut active = active.borrow_mut();
        if active.is_some() {
            return Err(js_error(
                "an editor preview Bevy app is already active in this browser context",
            ));
        }
        *active = Some(bridge.clone());
        Ok(())
    })?;

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "PuzzleStudio Editor Preview".to_string(),
            canvas: Some(canvas_selector.to_string()),
            fit_canvas_to_parent: true,
            ..Default::default()
        }),
        ..Default::default()
    }));
    puzzle_bevy_player::install_puzzle_bevy_player(&mut app, host);
    app.insert_non_send(EditorPreviewDocument(document))
        .insert_non_send(bridge)
        .add_systems(Update, process_editor_preview_controls);
    app.run();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = dispatchEditorPreviewCommand)]
pub fn dispatch_editor_preview_command(request_json: &str) -> Result<(), JsValue> {
    with_active_bridge(|bridge| bridge.submit_json(request_json)).map_err(js_error)
}

#[cfg(target_arch = "wasm32")]
fn dispatch_committed_observations(bridge: &EditorPreviewBridge, committed_revision: Option<u64>) {
    for observation in take_committed_observations(bridge, committed_revision) {
        if let Err(error) = dispatch_observation_event(&observation) {
            web_sys::console::error_1(&JsValue::from_str(&error));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn dispatch_observation_event(observation: &EditorPreviewObservation) -> Result<(), String> {
    let json = serde_json::to_string(observation)
        .map_err(|error| format!("editor preview observation serialization failed: {error}"))?;
    let detail = js_sys::JSON::parse(&json)
        .map_err(|error| format!("editor preview observation JSON conversion failed: {error:?}"))?;
    let init = web_sys::CustomEventInit::new();
    init.set_detail(&detail);
    let event = web_sys::CustomEvent::new_with_event_init_dict(
        "PuzzleStudioEditorPreviewObservation",
        &init,
    )
    .map_err(|error| format!("editor preview observation event failed: {error:?}"))?;
    web_sys::window()
        .ok_or_else(|| "editor preview observation requires a browser window".to_string())?
        .dispatch_event(&event)
        .map_err(|error| format!("editor preview observation dispatch failed: {error:?}"))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn with_active_bridge(
    operation: impl FnOnce(&EditorPreviewBridge) -> Result<(), String>,
) -> Result<(), String> {
    ACTIVE_EDITOR_PREVIEW_BRIDGE.with(|active| {
        let active = active.borrow();
        let bridge = active
            .as_ref()
            .ok_or_else(|| "editor preview Bevy app is not active".to_string())?;
        operation(bridge)
    })
}

#[cfg(target_arch = "wasm32")]
fn validate_canvas_selector(canvas_selector: &str) -> Result<(), String> {
    if canvas_selector.trim().is_empty() {
        return Err("editor preview canvas selector must not be empty".to_string());
    }
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| "editor preview requires a browser document".to_string())?;
    let element = document
        .query_selector(canvas_selector)
        .map_err(|error| format!("invalid editor preview canvas selector: {error:?}"))?
        .ok_or_else(|| {
            format!("editor preview canvas selector `{canvas_selector}` matched no element")
        })?;
    if !element.is_instance_of::<web_sys::HtmlCanvasElement>() {
        return Err(format!(
            "editor preview canvas selector `{canvas_selector}` must match a canvas element"
        ));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
const title = "Editor Bevy Host"

puzzle default {
layers {
Player
}
rules {
right [ Player | ] -> [ | Player ]
}
}

visuals {
Player {
#fff
0
}
}

levels {
legend {
P = Player
. = empty
}
level "one" {
P.
}
}
"#;

    fn document() -> puzzle_lang::LoadedDocument {
        puzzle_lang::parse_game_for_path(SOURCE, "editor_bevy_host.puzzle")
            .expect("editor fixture should parse")
    }

    #[test]
    fn traced_input_updates_the_same_snapshot_consumed_by_bevy() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(SOURCE, "editor_bevy_host.puzzle")
                .expect("editor fixture should initialize");
        let before_revision = host.snapshot().revision;

        let observation = apply_editor_control(
            &mut host,
            &document(),
            EditorPreviewControlRequest::SyntheticKey {
                command_id: 7,
                key: "ArrowRight".to_string(),
                code: "ArrowRight".to_string(),
                repeat: false,
                alt_key: false,
                ctrl_key: false,
                meta_key: false,
                shift_key: false,
                trace: true,
            },
            0.0,
        );

        let EditorPreviewObservation::DebugTrace {
            command_id,
            snapshot,
            debug,
        } = observation
        else {
            panic!("traced input must emit a trace observation");
        };
        assert_eq!(command_id, 7);
        assert!(!debug.is_null());
        assert_eq!(host.snapshot().revision, before_revision + 1);
        assert_eq!(snapshot["revision"], host.snapshot().revision);
    }

    #[test]
    fn hydration_updates_the_snapshot_observed_by_the_same_bevy_host() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(SOURCE, "editor_bevy_host.puzzle")
                .expect("editor fixture should initialize");
        let before_revision = host.snapshot().revision;

        let observation = apply_editor_control(
            &mut host,
            &document(),
            EditorPreviewControlRequest::HydrateState {
                command_id: 9,
                state: serde_json::json!({
                    "kind": "2d",
                    "width": 2,
                    "height": 1,
                    "layerCount": 1,
                    "slots": [0, 1],
                    "variables": [],
                    "levelFiredRules": [],
                }),
                level_index: 0,
                materialize_level_start: false,
            },
            0.0,
        );

        let EditorPreviewObservation::State {
            command_id: Some(command_id),
            state,
        } = observation
        else {
            panic!("valid hydration must emit a state observation");
        };
        assert_eq!(command_id, 9);
        assert_eq!(host.snapshot().revision, before_revision + 1);
        assert_eq!(state["revision"], host.snapshot().revision);
        assert_eq!(state["scene"]["cells"][1]["objectIds"][0], 1);
    }

    #[test]
    fn draft_hydration_resolves_compiled_symbols_before_updating_the_bevy_host() {
        let mut host =
            PuzzleBevyPlayerHost::from_image_free_source(SOURCE, "editor_bevy_host.puzzle")
                .expect("editor fixture should initialize");
        let observation = apply_editor_control(
            &mut host,
            &document(),
            EditorPreviewControlRequest::HydrateDraft {
                command_id: 10,
                model: "default".to_string(),
                level_index: 0,
                draft: puzzle_authoring::EditorDraftState::Grid2d(
                    puzzle_authoring::EditorDraftLevel2d {
                        size: puzzle_authoring::EditorDraftSize2d {
                            width: 3,
                            height: 1,
                        },
                        cells: vec![puzzle_authoring::EditorDraftCell2d {
                            position: puzzle_authoring::EditorDraftPosition2d { x: 2, y: 0 },
                            symbol: "P".to_string(),
                        }],
                    },
                ),
                presentation: puzzle_editor_preview_contract::EditorDraftPresentation::Grid2d {
                    surface_id: "main".to_string(),
                },
            },
            0.0,
        );

        let EditorPreviewObservation::State {
            command_id: Some(10),
            state,
        } = observation
        else {
            panic!("valid draft hydration must emit a state observation");
        };
        assert_eq!(state["levelCells"].as_array().map(Vec::len), Some(3));
        assert_eq!(state["levelCells"][2][0], 1);
    }

    #[test]
    fn bridge_accepts_only_the_named_editor_control_contract() {
        let bridge = EditorPreviewBridge::default();
        bridge
            .submit_json(r#"{"type":"requestSnapshot","commandId":1}"#)
            .expect("typed inspect request should be accepted");
        let error = bridge
            .submit_json(r#"{"type":"dispatch","commandId":2,"action":"right"}"#)
            .expect_err("unowned dispatch shortcut must be rejected");
        assert!(error.contains("unknown variant"));
    }

    #[test]
    fn command_and_observation_envelopes_have_exact_browser_names() {
        let command = serde_json::from_str::<EditorPreviewControlRequest>(
            r#"{"type":"syntheticKey","commandId":4,"key":"ArrowRight","code":"ArrowRight","repeat":false,"altKey":false,"ctrlKey":false,"metaKey":false,"shiftKey":false,"trace":true}"#,
        )
        .expect("typed key command should deserialize");
        assert!(matches!(
            command,
            EditorPreviewControlRequest::SyntheticKey {
                command_id: 4,
                key,
                trace: true,
                ..
            } if key == "ArrowRight"
        ));

        let observation = EditorPreviewObservation::RuntimeReady;
        assert_eq!(
            serde_json::to_value(observation).unwrap(),
            serde_json::json!({
                "type": "PuzzleStudioPreviewRuntimeReady",
            })
        );

        let observation = EditorPreviewObservation::State {
            command_id: Some(4),
            state: Map::from_iter([
                ("revision".to_string(), serde_json::json!(3)),
                ("screen".to_string(), serde_json::json!("playing")),
            ]),
        };
        assert_eq!(
            serde_json::to_value(observation).unwrap(),
            serde_json::json!({
                "type": "PuzzleStudioPreviewState",
                "commandId": 4,
                "revision": 3,
                "screen": "playing",
            })
        );
    }

    #[test]
    fn observations_wait_for_the_committed_bevy_revision() {
        let bridge = EditorPreviewBridge::default();
        bridge
            .observations
            .borrow_mut()
            .push_back(QueuedEditorPreviewObservation {
                required_revision: Some(4),
                observation: EditorPreviewObservation::RuntimeReady,
            });

        assert!(take_committed_observations(&bridge, Some(3)).is_empty());
        assert_eq!(bridge.observations.borrow().len(), 1);
        assert!(matches!(
            take_committed_observations(&bridge, Some(4)).as_slice(),
            [EditorPreviewObservation::RuntimeReady]
        ));
    }
}
