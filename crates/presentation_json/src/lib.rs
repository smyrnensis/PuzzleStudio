use puzzle_scene::{
    SceneAlign, SceneDistribution, SceneLayout, SceneSpace, SceneTextAlign, SceneTextRole,
};
use puzzle_session_contract::{
    RuntimeComponentPresentation, RuntimeDevelopmentRendererState,
    RuntimeDevelopmentSessionSnapshot, RuntimeRendererState, RuntimeResolvedScene,
    RuntimeResolvedSceneComponent, RuntimeSurfaceComponent, RuntimeViewportDimension,
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
        "revision": player.revision,
        "has_progress_save": player.has_progress_save,
        "theme": player.theme,
        "defaultWaitMs": player.default_wait_ms,
        "inputBuffer": player.input_buffer,
        "animation": player.animation,
        "presentationEvents": player.presentation_events,
        "presentationContinuation": player.presentation_continuation,
        "levelIndex": player.level_index,
        "levelCount": player.level_count,
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
        "inputs": snapshot.inputs.iter().map(|input| json!({
            "id": input.id,
            "name": input.name,
            "triggers": input.triggers,
        })).collect::<Vec<_>>(),
    }))
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
        RuntimeSceneActionToken,
    };
    use puzzle_session_contract::{
        RuntimeAnimationSettings, RuntimeKeyBinding, RuntimePuzzle2DevelopmentSnapshot,
        RuntimePuzzle2Resources, RuntimePuzzle2Settings, RuntimePuzzle2Snapshot,
        RuntimeResolvedEventBinding, RuntimeResolvedScene, RuntimeResolvedSceneComponent,
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
