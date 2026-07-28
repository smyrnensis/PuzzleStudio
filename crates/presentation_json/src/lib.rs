use puzzle_scene::{
    SceneAlign, SceneDistribution, SceneLayout, SceneSpace, SceneTextAlign, SceneTextRole,
};
use puzzle_session_contract::{
    RuntimeComponentPresentation, RuntimeDevelopmentRendererState,
    RuntimeDevelopmentSessionSnapshot, RuntimeInputBinding, RuntimeRendererState,
    RuntimeResolvedScene, RuntimeResolvedSceneComponent, RuntimeSurfaceComponent,
    RuntimeViewportDimension,
};
use serde::ser::Error as _;
use serde_json::{Map, Value, json};

pub fn to_string(
    snapshot: &RuntimeDevelopmentSessionSnapshot,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&to_value(snapshot)?)
}

pub fn to_value(snapshot: &RuntimeDevelopmentSessionSnapshot) -> Result<Value, serde_json::Error> {
    let player = &snapshot.player;
    Ok(json!({
        "sessionRevision": player.session_revision,
        "stateCommit": player.state_commit,
        "has_progress_save": player.has_progress_save,
        "theme": player.theme,
        "defaultWaitMs": player.default_wait_ms,
        "inputBuffer": player.input_buffer,
        "animation": player.animation,
        "presentation": player.presentation,
        "levelIndex": player.level_index,
        "levelCount": player.level_count,
        "queuedModelInput": player.queued_model_input,
        "levels": snapshot.levels.iter().map(|(name, level)| (name.clone(), json!({
            "id": level.id,
            "name": level.name,
            "puzzle": level.puzzle,
            "pack": level.pack,
            "ordinal": level.ordinal,
            "progress": { "cleared": level.cleared },
        }))).collect::<Map<_, _>>(),
        "acceptsModelInput": player.accepts_model_input,
        "viewportSources": player.viewport_sources.iter()
            .map(|(source, state)| Ok(json!({
                "id": source,
                "state": renderer_value(
                    state,
                    snapshot.viewport_sources.get(source),
                )?,
            })))
            .collect::<Result<Vec<_>, serde_json::Error>>()?,
        "surface": {
            "root": player.surface.root,
            "focus": player.surface.focus,
            "components": player.surface.components.iter()
                .map(surface_component_value)
                .collect::<Result<Vec<_>, _>>()?,
        },
        "solverState": snapshot.solver_state,
        "selectedLevelIndex": snapshot.selected_level_index,
        "busy": player.busy,
        "canUndo": player.can_undo,
        "canRedo": player.can_redo,
        "inputs": input_bindings_value(&snapshot.inputs),
    }))
}

/// Projects the development data consumed by the editor preview parent.
///
/// Player presentation remains a direct typed Rust path. This wire value
/// contains only editor-facing observation fields, so browser code does not
/// reconstruct focus, active viewports, inputs, or level identity by walking
/// the player surface.
pub fn to_editor_preview_state_value(
    snapshot: &RuntimeDevelopmentSessionSnapshot,
) -> Result<Value, serde_json::Error> {
    let player = &snapshot.player;
    let level_cells = editor_preview_level_cells(
        &player.surface.focus,
        &player.viewport_sources,
        &snapshot.viewport_sources,
    )?;
    let aspect_ratio =
        editor_preview_aspect_ratio(&player.surface.focus, &player.surface.components);
    let viewports = player
        .viewport_sources
        .iter()
        .map(|(source, state)| {
            Ok(json!({
                "id": source,
                "dimension": match state {
                    RuntimeRendererState::TwoD(_) => "2d",
                    RuntimeRendererState::ThreeD(_) => "3d",
                },
                "state": renderer_value(
                    state,
                    snapshot.viewport_sources.get(source),
                )?,
            }))
        })
        .collect::<Result<Vec<_>, serde_json::Error>>()?;
    let focused = editor_preview_focus_fields(
        &player.surface.focus,
        player.accepts_model_input,
        &viewports,
    );
    Ok(json!({
        "sessionRevision": player.session_revision,
        "stateCommit": player.state_commit,
        "screen": player.surface.focus,
        "focus": player.surface.focus,
        "activeModel": focused.active_model,
        "screenHasPuzzle": focused.screen_has_puzzle,
        "rawScene": focused.raw_scene,
        "scene": focused.scene,
        "levelCells": level_cells,
        "puzzle3Snapshot": focused.puzzle3_snapshot,
        "aspectRatio": aspect_ratio,
        "theme": player.theme,
        "levelIndex": player.level_index,
        "levelCount": player.level_count,
        "selectedLevelIndex": snapshot.selected_level_index,
        "acceptsModelInput": player.accepts_model_input,
        "busy": player.busy,
        "canUndo": player.can_undo,
        "canRedo": player.can_redo,
        "inputs": input_bindings_value(&snapshot.inputs),
    }))
}

pub fn input_bindings_value(inputs: &[RuntimeInputBinding]) -> Value {
    Value::Array(
        inputs
            .iter()
            .map(|input| {
                json!({
                    "id": input.id,
                    "name": input.name,
                    "triggers": input.triggers,
                })
            })
            .collect(),
    )
}

fn editor_preview_level_cells(
    focus: &str,
    player_viewports: &std::collections::BTreeMap<
        puzzle_runtime_contract::RuntimeViewportSourceId,
        RuntimeRendererState,
    >,
    development_viewports: &std::collections::BTreeMap<
        puzzle_runtime_contract::RuntimeViewportSourceId,
        RuntimeDevelopmentRendererState,
    >,
) -> Result<Option<Vec<Vec<u16>>>, serde_json::Error> {
    let focused = player_viewports
        .iter()
        .filter(|(source, _)| source.component == focus)
        .collect::<Vec<_>>();
    let [(source, renderer)] = focused.as_slice() else {
        return Ok(None);
    };
    if !matches!(renderer, RuntimeRendererState::TwoD(_)) {
        return Ok(None);
    }
    let Some(RuntimeDevelopmentRendererState::TwoD(development)) =
        development_viewports.get(*source)
    else {
        return Err(serde_json::Error::custom(
            "focused 2D player viewport is missing its development snapshot",
        ));
    };
    let cell_count = usize::from(development.width)
        .checked_mul(usize::from(development.height))
        .ok_or_else(|| serde_json::Error::custom("focused 2D viewport cell count overflowed"))?;
    if development.cells.len() != cell_count {
        return Err(serde_json::Error::custom(format!(
            "focused 2D viewport has {} cells, expected {cell_count}",
            development.cells.len()
        )));
    }
    let layer_count = usize::from(development.layer_count);
    let mut cells = vec![vec![0; layer_count]; cell_count];
    let mut occupied = vec![vec![false; layer_count]; cell_count];
    let mut coordinates = vec![false; cell_count];
    for cell in &development.cells {
        if cell.x >= development.width || cell.y >= development.height {
            return Err(serde_json::Error::custom(format!(
                "focused 2D viewport cell ({}, {}) is outside {}x{}",
                cell.x, cell.y, development.width, development.height
            )));
        }
        let index = usize::from(cell.y) * usize::from(development.width) + usize::from(cell.x);
        if std::mem::replace(&mut coordinates[index], true) {
            return Err(serde_json::Error::custom(format!(
                "focused 2D viewport repeats cell ({}, {})",
                cell.x, cell.y
            )));
        }
        for layer in &cell.layers {
            let layer_index = usize::from(layer.layer);
            if layer_index >= layer_count {
                return Err(serde_json::Error::custom(format!(
                    "focused 2D viewport cell ({}, {}) contains layer {} outside layer count {}",
                    cell.x, cell.y, layer.layer, development.layer_count
                )));
            }
            if std::mem::replace(&mut occupied[index][layer_index], true) {
                return Err(serde_json::Error::custom(format!(
                    "focused 2D viewport cell ({}, {}) repeats layer {}",
                    cell.x, cell.y, layer.layer
                )));
            }
            cells[index][layer_index] = layer.object_id;
        }
    }
    Ok(Some(cells))
}

fn editor_preview_aspect_ratio(
    focus: &str,
    components: &[RuntimeSurfaceComponent],
) -> Option<puzzle_scene::SceneAspectRatio> {
    let focused = components
        .iter()
        .filter(|component| component.id == focus)
        .collect::<Vec<_>>();
    let [component] = focused.as_slice() else {
        return None;
    };
    let RuntimeComponentPresentation::Ready(scene) = &component.presentation else {
        return None;
    };
    scene.layout.aspect_ratio
}

struct EditorPreviewFocusFields {
    active_model: Option<String>,
    screen_has_puzzle: bool,
    raw_scene: Option<Value>,
    scene: Option<Value>,
    puzzle3_snapshot: Option<Value>,
}

fn editor_preview_focus_fields(
    focus: &str,
    accepts_model_input: bool,
    viewports: &[Value],
) -> EditorPreviewFocusFields {
    let focused = viewports
        .iter()
        .filter(|viewport| viewport["id"]["component"].as_str() == Some(focus))
        .collect::<Vec<_>>();
    let focused_viewport = (focused.len() == 1).then(|| focused[0].clone());
    let active_model = focused_viewport
        .as_ref()
        .and_then(|viewport| viewport["id"]["model"].as_str())
        .map(str::to_string);
    let raw_scene = focused_viewport
        .as_ref()
        .map(|viewport| viewport["state"].clone());
    let scene = focused_viewport
        .as_ref()
        .filter(|viewport| viewport["dimension"] == "2d")
        .map(|viewport| viewport["state"]["renderScene"].clone());
    let puzzle3_snapshot = focused_viewport
        .as_ref()
        .filter(|viewport| viewport["dimension"] == "3d")
        .map(|viewport| viewport["state"].clone());
    EditorPreviewFocusFields {
        active_model,
        screen_has_puzzle: accepts_model_input || !focused.is_empty(),
        raw_scene,
        scene,
        puzzle3_snapshot,
    }
}

pub fn resolved_scene_to_value(scene: &RuntimeResolvedScene) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
    value.insert("layout".into(), scene_layout_value(&scene.layout));
    value.insert(
        "components".into(),
        Value::Array(
            scene
                .components
                .iter()
                .map(resolved_component_value)
                .collect(),
        ),
    );
    if let Some(keys) = &scene.keys {
        value.insert(
            "keys".into(),
            Value::Array(
                keys.iter()
                    .map(|binding| {
                        json!({
                            "keys": binding.keys,
                            "actionToken": binding.action,
                        })
                    })
                    .collect(),
            ),
        );
    }
    if let Some(events) = &scene.events {
        value.insert(
            "events".into(),
            Value::Object(
                events
                    .iter()
                    .map(|(name, binding)| {
                        (
                            name.clone(),
                            json!({
                                "pointer": binding.pointer,
                                "keys": binding.keys,
                                "actionToken": binding.action,
                            }),
                        )
                    })
                    .collect(),
            ),
        );
    }
    Ok(Value::Object(value))
}

fn renderer_value(
    state: &RuntimeRendererState,
    development: Option<&RuntimeDevelopmentRendererState>,
) -> Result<Value, serde_json::Error> {
    match state {
        RuntimeRendererState::TwoD(state) => {
            let Some(RuntimeDevelopmentRendererState::TwoD(development)) = development else {
                return Err(serde_json::Error::custom(
                    "2D player viewport is missing its development snapshot",
                ));
            };
            let mut value = serde_json::to_value(development)?;
            let Value::Object(ref mut fields) = value else {
                unreachable!("2D development snapshot serializes as an object");
            };
            fields.insert("view".into(), serde_json::to_value(state.view)?);
            fields.insert(
                "renderScene".into(),
                serde_json::to_value(&state.render_scene)?,
            );
            fields.insert(
                "displayError".into(),
                serde_json::to_value(&state.display_error)?,
            );
            Ok(value)
        }
        RuntimeRendererState::ThreeD(state) => {
            if !matches!(development, Some(RuntimeDevelopmentRendererState::ThreeD)) {
                return Err(serde_json::Error::custom(
                    "3D player viewport is missing its development marker",
                ));
            }
            serde_json::to_value(state)
        }
    }
}

fn surface_component_value(
    component: &RuntimeSurfaceComponent,
) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
    value.insert("id".into(), Value::String(component.id.clone()));
    value.insert(
        "placement".into(),
        serde_json::to_value(component.placement)?,
    );
    value.insert(
        "visibility".into(),
        serde_json::to_value(component.visibility)?,
    );
    value.insert("modal".into(), Value::Bool(component.modal));
    if let Some(event) = &component.await_event {
        value.insert("awaitEvent".into(), Value::String(event.clone()));
    }
    value.insert(
        "presentation".into(),
        match &component.presentation {
            RuntimeComponentPresentation::Ready(scene) => resolved_scene_to_value(scene)?,
            RuntimeComponentPresentation::Error { error } => json!({ "error": error }),
        },
    );
    Ok(Value::Object(value))
}

fn resolved_component_value(component: &RuntimeResolvedSceneComponent) -> Value {
    match component {
        RuntimeResolvedSceneComponent::Viewport {
            dimension,
            source,
            layout,
        } => json!({
            "kind": match dimension { RuntimeViewportDimension::TwoD => "puzzle", RuntimeViewportDimension::ThreeD => "puzzle3" },
            "source": source,
            "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Frame {
            kind,
            source,
            layout,
        } => json!({
            "kind": kind, "source": source, "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Text {
            role,
            value,
            text_align,
            layout,
        } => json!({
            "kind": "text", "role": role_name(*role), "value": value,
            "textAlign": text_align.map(text_align_name), "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Button {
            label,
            action,
            layout,
        } => json!({
            "kind": "button", "label": label, "actionToken": action,
            "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Choice {
            label,
            action,
            selected,
            layout,
        } => json!({
            "kind": "choice", "label": label, "actionToken": action, "selected": selected,
            "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Row { layout, children } => {
            container_value("row", layout, children)
        }
        RuntimeResolvedSceneComponent::Column { layout, children } => {
            container_value("column", layout, children)
        }
        RuntimeResolvedSceneComponent::Box { layout, children } => {
            container_value("box", layout, children)
        }
    }
}

fn container_value(
    kind: &str,
    layout: &SceneLayout,
    children: &[RuntimeResolvedSceneComponent],
) -> Value {
    json!({ "kind": kind, "layout": scene_layout_value(layout),
        "children": children.iter().map(resolved_component_value).collect::<Vec<_>>() })
}

fn scene_layout_value(layout: &SceneLayout) -> Value {
    let mut value = Map::new();
    if let SceneSpace::Fill { weight } = layout.space {
        value.insert("space".into(), json!({ "kind": "fill", "weight": weight }));
    }
    if let Some(ratio) = layout.aspect_ratio {
        value.insert(
            "aspectRatio".into(),
            json!({ "width": ratio.width, "height": ratio.height }),
        );
    }
    if let Some(gap) = layout.gap {
        value.insert("gap".into(), json!(gap));
    }
    if layout.align != SceneLayout::default().align {
        value.insert(
            "align".into(),
            json!(match layout.align {
                SceneAlign::Start => "start",
                SceneAlign::Center => "center",
                SceneAlign::End => "end",
                SceneAlign::Stretch => "stretch",
            }),
        );
    }
    if layout.distribute != SceneLayout::default().distribute {
        value.insert(
            "distribute".into(),
            json!(match layout.distribute {
                SceneDistribution::Start => "start",
                SceneDistribution::Center => "center",
                SceneDistribution::End => "end",
                SceneDistribution::Between => "between",
            }),
        );
    }
    if layout.scroll {
        value.insert("scroll".into(), Value::Bool(true));
    }
    Value::Object(value)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use puzzle_runtime_contract::{
        RuntimeKeyTrigger, RuntimeResolvedRenderScene, RuntimeResolvedView2d, RuntimeSceneActionId,
        RuntimeSceneActionToken, RuntimeViewportSourceId, RuntimeVisualComposition,
    };
    use puzzle_session_contract::{
        RuntimeAnimationSettings, RuntimeKeyBinding, RuntimePuzzle2Cell,
        RuntimePuzzle2DevelopmentSnapshot, RuntimePuzzle2Layer, RuntimePuzzle2Resources,
        RuntimePuzzle2Settings, RuntimePuzzle2Snapshot, RuntimeResolvedEventBinding,
        RuntimeResolvedScene, RuntimeResolvedSceneComponent,
    };

    use super::*;

    #[test]
    fn resolved_scene_wire_carries_tokens_and_node_local_selection_without_effects() {
        let choice_action = RuntimeSceneActionToken {
            component: "title".to_string(),
            action: RuntimeSceneActionId::Node { ordinal: 0 },
        };
        let scene = RuntimeResolvedScene {
            layout: SceneLayout::default(),
            components: vec![RuntimeResolvedSceneComponent::Choice {
                label: "New Game".to_string(),
                action: Some(choice_action.clone()),
                selected: true,
                layout: SceneLayout::default(),
            }],
            keys: Some(vec![RuntimeKeyBinding {
                keys: vec![RuntimeKeyTrigger::Enter],
                action: RuntimeSceneActionToken {
                    component: "title".to_string(),
                    action: RuntimeSceneActionId::Key { ordinal: 0 },
                },
            }]),
            events: Some(BTreeMap::from([(
                "dismiss".to_string(),
                RuntimeResolvedEventBinding {
                    pointer: true,
                    keys: vec![RuntimeKeyTrigger::AnyInput],
                    action: Some(RuntimeSceneActionToken {
                        component: "title".to_string(),
                        action: RuntimeSceneActionId::Event {
                            name: "dismiss".to_string(),
                        },
                    }),
                },
            )])),
        };

        let value = resolved_scene_to_value(&scene).unwrap();
        assert!(value.get("name").is_none());
        assert_eq!(value["components"][0]["selected"], true);
        assert_eq!(
            value["components"][0]["actionToken"],
            serde_json::to_value(choice_action).unwrap()
        );
        assert!(value["components"][0].get("effect").is_none());
        assert_eq!(value["keys"][0]["actionToken"]["action"]["kind"], "key");
        assert!(value["keys"][0].get("effect").is_none());
        assert_eq!(
            value["events"]["dismiss"]["actionToken"]["action"]["kind"],
            "event"
        );
    }

    #[test]
    fn inactive_event_binding_serializes_an_explicit_null_action_capability() {
        let scene = RuntimeResolvedScene {
            layout: SceneLayout::default(),
            components: Vec::new(),
            keys: None,
            events: Some(BTreeMap::from([(
                "dismiss".to_string(),
                RuntimeResolvedEventBinding {
                    pointer: true,
                    keys: vec![RuntimeKeyTrigger::AnyInput],
                    action: None,
                },
            )])),
        };

        let value = resolved_scene_to_value(&scene).unwrap();
        let binding = &value["events"]["dismiss"];
        assert!(
            binding.get("actionToken").is_some(),
            "inactive capability must remain distinct from a malformed missing wire field"
        );
        assert!(
            binding["actionToken"].is_null(),
            "inactive capability must serialize as explicit null"
        );
    }

    #[test]
    fn two_dimensional_development_wire_requires_the_separate_debug_projection() {
        let player = RuntimeRendererState::TwoD(RuntimePuzzle2Snapshot {
            view: RuntimeResolvedView2d {
                origin: [0, 0],
                size: [2, 1],
            },
            render_scene: RuntimeResolvedRenderScene {
                clips: Vec::new(),
                instances: Vec::new(),
                composition_groups: Vec::new(),
                cells: Vec::new(),
                decorations: Vec::new(),
                render_priority_count: 0,
                animation_duration_ms: 0,
            },
            display_error: None,
        });

        let error = renderer_value(&player, None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("missing its development snapshot")
        );

        let development =
            RuntimeDevelopmentRendererState::TwoD(RuntimePuzzle2DevelopmentSnapshot {
                width: 2,
                height: 1,
                layer_count: 1,
                settings: RuntimePuzzle2Settings::default(),
                animation: RuntimeAnimationSettings::default(),
                regions: Vec::new(),
                resources: RuntimePuzzle2Resources::default(),
                cells: Vec::new(),
            });
        let value = renderer_value(&player, Some(&development)).unwrap();
        assert_eq!(value["width"], 2);
        assert_eq!(value["view"]["size"], json!([2, 1]));
        assert!(value.get("renderScene").is_some());
    }

    #[test]
    fn editor_preview_focus_does_not_treat_a_viewport_behind_a_modal_as_active() {
        let playing_viewport = json!({
            "id": {
                "component": "playing",
                "model": "main",
                "source": "board",
            },
            "dimension": "2d",
            "state": {"kind": "2d"},
        });
        let modal = editor_preview_focus_fields("message:1", false, &[playing_viewport]);
        assert_eq!(modal.active_model, None);
        assert!(!modal.screen_has_puzzle);
        assert_eq!(modal.raw_scene, None);

        let modal_viewport = json!({
            "id": {
                "component": "message:1",
                "model": "preview-model",
                "source": "preview",
            },
            "dimension": "3d",
            "state": {"kind": "3d"},
        });
        let focused = editor_preview_focus_fields("message:1", false, &[modal_viewport.clone()]);
        assert_eq!(focused.active_model.as_deref(), Some("preview-model"));
        assert!(focused.screen_has_puzzle);
        assert_eq!(focused.raw_scene, Some(modal_viewport["state"].clone()));
    }

    #[test]
    fn editor_preview_projects_frontmost_aspect_and_ordered_2d_layer_slots() {
        let board_source = RuntimeViewportSourceId {
            model: "main".to_string(),
            component: "playing".to_string(),
            source: "board".to_string(),
        };
        let player_viewports = BTreeMap::from([(
            board_source.clone(),
            RuntimeRendererState::TwoD(RuntimePuzzle2Snapshot {
                view: RuntimeResolvedView2d {
                    origin: [0, 0],
                    size: [2, 1],
                },
                render_scene: RuntimeResolvedRenderScene {
                    clips: Vec::new(),
                    instances: Vec::new(),
                    composition_groups: Vec::new(),
                    cells: Vec::new(),
                    decorations: Vec::new(),
                    render_priority_count: 0,
                    animation_duration_ms: 0,
                },
                display_error: None,
            }),
        )]);
        let layer = |layer, object_id| RuntimePuzzle2Layer {
            layer,
            object_id,
            object: format!("object-{object_id}"),
            visual: format!("visual-{object_id}"),
            render_priority: 0,
            render_order: 0,
            composition: RuntimeVisualComposition::Ordered,
        };
        let development_viewports = BTreeMap::from([(
            board_source,
            RuntimeDevelopmentRendererState::TwoD(RuntimePuzzle2DevelopmentSnapshot {
                width: 2,
                height: 1,
                layer_count: 2,
                settings: RuntimePuzzle2Settings::default(),
                animation: RuntimeAnimationSettings::default(),
                regions: Vec::new(),
                resources: RuntimePuzzle2Resources::default(),
                cells: vec![
                    RuntimePuzzle2Cell {
                        x: 1,
                        y: 0,
                        render_order: 0,
                        layers: vec![layer(1, 7), layer(0, 4)],
                    },
                    RuntimePuzzle2Cell {
                        x: 0,
                        y: 0,
                        render_order: 0,
                        layers: vec![layer(0, 1)],
                    },
                ],
            }),
        )]);
        assert_eq!(
            editor_preview_level_cells("playing", &player_viewports, &development_viewports)
                .unwrap(),
            Some(vec![vec![1, 0], vec![4, 7]])
        );
        assert_eq!(
            editor_preview_level_cells("message:1", &player_viewports, &development_viewports)
                .unwrap(),
            None,
            "a modal focus must not expose the viewport behind it"
        );

        let scene = |aspect_ratio| RuntimeResolvedScene {
            layout: SceneLayout {
                aspect_ratio,
                ..SceneLayout::default()
            },
            components: Vec::new(),
            keys: None,
            events: None,
        };
        let components = vec![
            RuntimeSurfaceComponent {
                id: "playing".to_string(),
                placement: puzzle_scene::ComponentPlacement::Root,
                visibility: puzzle_scene::ComponentVisibility::Visible,
                modal: false,
                await_event: None,
                presentation: RuntimeComponentPresentation::Ready(scene(Some(
                    puzzle_scene::SceneAspectRatio::new(16, 9),
                ))),
            },
            RuntimeSurfaceComponent {
                id: "message:1".to_string(),
                placement: puzzle_scene::ComponentPlacement::Overlay,
                visibility: puzzle_scene::ComponentVisibility::Visible,
                modal: true,
                await_event: Some("dismiss".to_string()),
                presentation: RuntimeComponentPresentation::Ready(scene(Some(
                    puzzle_scene::SceneAspectRatio::new(3, 2),
                ))),
            },
        ];
        assert_eq!(
            editor_preview_aspect_ratio("message:1", &components),
            Some(puzzle_scene::SceneAspectRatio::new(3, 2))
        );
    }
}
fn role_name(value: SceneTextRole) -> &'static str {
    match value {
        SceneTextRole::Heading => "heading",
        SceneTextRole::Subheading => "subheading",
        SceneTextRole::Body => "body",
        SceneTextRole::Caption => "caption",
    }
}
fn text_align_name(value: SceneTextAlign) -> &'static str {
    match value {
        SceneTextAlign::Start => "start",
        SceneTextAlign::Center => "center",
        SceneTextAlign::End => "end",
    }
}
