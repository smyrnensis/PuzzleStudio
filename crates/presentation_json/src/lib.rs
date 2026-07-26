<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
        "levelIndex": player.level_index,
        "levelCount": player.level_count,
=======
use std::collections::BTreeMap;

use puzzle_lang::{
    KeyTrigger, ResourceSelection, SceneAlignDef, SceneComponent, SceneDef, SceneDistributionDef,
    SceneLayoutDef, ScenePuzzleInitializer, SceneSpaceDef, SceneStateLifetime, SceneTextContent,
    SceneTransitionTrigger, SceneValue, ViewportProjectionDef,
};
use puzzle_session_contract::{
    RuntimeComponentPresentation, RuntimePresentationBackend, RuntimeRendererState,
    RuntimeResolvedScene, RuntimeResolvedSceneComponent, RuntimeSessionSnapshot,
    RuntimeSurfaceComponent, RuntimeViewportDimension,
};
use serde_json::{Map, Value, json};

#[derive(Default)]
pub struct JsonPresentationBackend;

impl RuntimePresentationBackend for JsonPresentationBackend {
    type Error = serde_json::Error;
    type Output = Value;

    fn present(&mut self, snapshot: &RuntimeSessionSnapshot) -> Result<Value, Self::Error> {
        to_value(snapshot)
    }
}

pub fn to_string(snapshot: &RuntimeSessionSnapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string(&to_value(snapshot)?)
}

pub fn renderer_to_string(state: &RuntimeRendererState) -> Result<String, serde_json::Error> {
    serde_json::to_string(&renderer_value(state)?)
}

pub fn to_value(snapshot: &RuntimeSessionSnapshot) -> Result<Value, serde_json::Error> {
    Ok(json!({
        "revision": snapshot.revision,
        "has_progress_save": snapshot.has_progress_save,
        "sounds": {
            "sfx": snapshot.sounds.sfx.iter().map(|sound| json!({
                "name": sound.name,
                "seed": sound.seed,
                "type": sound.type_target,
                "volume": sound.volume,
            })).collect::<Vec<_>>(),
            "music": snapshot.sounds.music.iter().map(|sound| json!({
                "name": sound.name,
                "seed": sound.seed,
                "height": sound.height,
                "bars": sound.bars,
                "bpm": sound.bpm,
                "volume": sound.volume,
            })).collect::<Vec<_>>(),
        },
        "theme": { "name": snapshot.theme.name, "variables": snapshot.theme.variables },
        "defaultWaitMs": snapshot.default_wait_ms,
        "inputBuffer": snapshot.input_buffer,
        "animation": snapshot.animation,
        "presentationEvents": snapshot.presentation_events,
        "levelIndex": snapshot.level_index,
        "levelCount": snapshot.level_count,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
        "levels": snapshot.levels.iter().map(|(name, level)| (name.clone(), json!({
            "id": level.id,
            "name": level.name,
            "puzzle": level.puzzle,
            "pack": level.pack,
            "ordinal": level.ordinal,
            "progress": { "cleared": level.cleared },
        }))).collect::<Map<_, _>>(),
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
        "scene": renderer_state_value(snapshot.scene.as_ref())?,
        "acceptsModelInput": snapshot.accepts_model_input,
        "gameState": scene_values_value(&snapshot.game_state),
        "sceneState": scene_values_value(&snapshot.scene_state),
        "scenePuzzles": snapshot.scene_puzzles,
        "scenePuzzleState": snapshot.scene_puzzle_state.iter()
            .map(|(name, state)| Ok((name.clone(), renderer_value(state)?)))
            .collect::<Result<Map<_, _>, serde_json::Error>>()?,
        "puzzle3AuthoringResources": snapshot.puzzle3_authoring_resources,
        "surface": {
            "root": snapshot.surface.root,
            "focus": snapshot.surface.focus,
            "components": snapshot.surface.components.iter()
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
                .map(surface_component_value)
                .collect::<Result<Vec<_>, _>>()?,
        },
        "solverState": snapshot.solver_state,
        "selectedLevelIndex": snapshot.selected_level_index,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
        "busy": player.busy,
        "canUndo": player.can_undo,
        "canRedo": player.can_redo,
        "inputs": snapshot.inputs.iter().map(|input| json!({
            "id": input.id,
            "name": input.name,
            "triggers": input.triggers,
        })).collect::<Vec<_>>(),
=======
        "busy": snapshot.busy,
        "canUndo": snapshot.can_undo,
        "canRedo": snapshot.can_redo,
        "inputs": snapshot.inputs.iter().map(|input| json!({
            "id": input.id,
            "name": input.name,
            "key": input.key,
            "arrow": input.arrow,
            "keys": input.keys,
        })).collect::<Vec<_>>(),
        "scenes": snapshot.scenes.iter().map(scene_def_value).collect::<Vec<_>>(),
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }))
}

pub fn resolved_scene_to_value(scene: &RuntimeResolvedScene) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    value.insert("name".into(), Value::String(scene.name.clone()));
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
                            "keys": binding.keys,
                            "actionToken": binding.action,
=======
                            "effect": binding.effect,
                            "keys": binding.keys,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
                                "actionToken": binding.action,
=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
                            }),
                        )
                    })
                    .collect(),
            ),
        );
    }
    Ok(Value::Object(value))
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
fn renderer_state_value(state: Option<&RuntimeRendererState>) -> Result<Value, serde_json::Error> {
    state
        .map(renderer_value)
        .transpose()
        .map(|value| value.unwrap_or(Value::Null))
}

fn renderer_value(state: &RuntimeRendererState) -> Result<Value, serde_json::Error> {
    match state {
        RuntimeRendererState::TwoD(state) => serde_json::to_value(state),
        RuntimeRendererState::ThreeD(state) => serde_json::to_value(state),
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }
}

fn surface_component_value(
    component: &RuntimeSurfaceComponent,
) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    value.insert("id".into(), Value::String(component.id.clone()));
    value.insert(
=======
    if let Some(authored) = &component.authored_projection {
        value.insert("name".into(), Value::String(authored.name.clone()));
        value.insert("focused".into(), Value::Bool(authored.focused));
        value.insert("choiceCursor".into(), json!(authored.choice_cursor));
        value.insert(
            "scene".into(),
            renderer_state_value(authored.scene.as_ref())?,
        );
        value.insert(
            "sceneState".into(),
            scene_values_value(&authored.scene_state),
        );
        value.insert("scenePuzzles".into(), json!(authored.scene_puzzles));
    }
    value.insert("id".into(), Value::String(component.id.clone()));
    value.insert(
        "definition".into(),
        Value::String(component.definition.clone()),
    );
    value.insert(
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
        "placement".into(),
        serde_json::to_value(component.placement)?,
    );
    value.insert(
        "visibility".into(),
        serde_json::to_value(component.visibility)?,
    );
    value.insert("modal".into(), Value::Bool(component.modal));
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
    if !component.properties.is_empty() {
        value.insert(
            "properties".into(),
            scene_values_value(&component.properties),
        );
    }
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
fn scene_values_value(values: &BTreeMap<String, SceneValue>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(name, value)| (name.clone(), scene_value(value)))
            .collect(),
    )
}

fn scene_value(value: &SceneValue) -> Value {
    match value {
        SceneValue::Bool(value) => Value::Bool(*value),
        SceneValue::Int(value) => json!(value),
        SceneValue::Text(value) | SceneValue::Symbol(value) => Value::String(value.clone()),
        SceneValue::LevelRef(value) => json!(value),
    }
}

>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
            effect,
            layout,
        } => json!({
            "kind": "button", "label": label, "effect": effect, "layout": scene_layout_value(layout),
        }),
        RuntimeResolvedSceneComponent::Choice {
            label,
            effect,
            layout,
        } => json!({
            "kind": "choice", "label": label, "effect": effect, "layout": scene_layout_value(layout),
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
=======
        RuntimeResolvedSceneComponent::Conditional {
            condition,
            children,
            else_children,
        } => json!({
            "kind": "conditional", "condition": condition,
            "children": children.iter().map(resolved_component_value).collect::<Vec<_>>(),
            "elseChildren": else_children.iter().map(resolved_component_value).collect::<Vec<_>>(),
        }),
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }
}

fn container_value(
    kind: &str,
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    layout: &SceneLayout,
=======
    layout: &SceneLayoutDef,
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    children: &[RuntimeResolvedSceneComponent],
) -> Value {
    json!({ "kind": kind, "layout": scene_layout_value(layout),
        "children": children.iter().map(resolved_component_value).collect::<Vec<_>>() })
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
fn scene_layout_value(layout: &SceneLayout) -> Value {
    let mut value = Map::new();
    if let SceneSpace::Fill { weight } = layout.space {
=======
fn scene_def_value(scene: &SceneDef) -> Value {
    json!({
        "name": scene.name,
        "layout": scene_layout_value(&scene.layout),
        "resources": {
            "levelsMode": resource_mode(&scene.resources.levels),
            "levels": resource_names(&scene.resources.levels),
            "visualsMode": resource_mode(&scene.resources.visuals),
            "visuals": resource_names(&scene.resources.visuals),
        },
        "state": {
            "variables": scene.state.variables.iter().map(|variable| json!({
                "name": variable.name, "default": scene_value(&variable.default),
                "lifetime": lifetime_name(variable.lifetime), "mutable": variable.mutable,
            })).collect::<Vec<_>>(),
            "puzzles": scene.state.puzzles.iter().map(|puzzle| {
                let mut value = json!({ "name": puzzle.name, "model": puzzle.model });
                match &puzzle.initializer {
                    ScenePuzzleInitializer::CurrentLevel => value["initializer"] = json!("current_level"),
                    ScenePuzzleInitializer::Level(level) => { value["initializer"] = json!("level"); value["level"] = json!(level); }
                }
                value
            }).collect::<Vec<_>>(),
        },
        "puzzleRule": scene.puzzle_rule.as_ref().map(|rule| json!({ "target": rule.target, "rule": rule.rule })),
        "components": scene.components.iter().map(scene_component_value).collect::<Vec<_>>(),
        "keys": scene.key_bindings.iter().map(|binding| json!({
            "effect": binding.effect,
            "keys": binding.keys.iter().map(key_name).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "routines": scene.routines.iter().map(|routine| json!({ "name": routine.name, "effect": routine.effect })).collect::<Vec<_>>(),
        "transitions": scene.transitions.iter().map(|transition| {
            let mut value = Map::new();
            match &transition.trigger {
                SceneTransitionTrigger::Condition(expr) => { value.insert("condition".into(), puzzle_scene::scene_expr_json_value(expr)); }
                SceneTransitionTrigger::Signal(expr) => { value.insert("signal".into(), puzzle_scene::scene_expr_json_value(expr)); }
                SceneTransitionTrigger::SceneStart => { value.insert("lifecycle".into(), json!("scene_start")); }
                SceneTransitionTrigger::LevelStart => { value.insert("lifecycle".into(), json!("level_start")); }
            }
            value.insert("effect".into(), json!(transition.effect));
            Value::Object(value)
        }).collect::<Vec<_>>(),
    })
}

fn scene_component_value(component: &SceneComponent) -> Value {
    match component {
        SceneComponent::Viewport(viewport) => json!({
            "kind": match viewport.projection { ViewportProjectionDef::TwoD => "puzzle", ViewportProjectionDef::ThreeD => "puzzle3" },
            "source": viewport.source, "layout": scene_layout_value(&viewport.layout),
        }),
        SceneComponent::Frame(frame) => {
            json!({ "kind": frame.kind, "source": frame.source, "layout": scene_layout_value(&frame.layout) })
        }
        SceneComponent::Text(text) => {
            let mut value = Map::new();
            value.insert("kind".into(), json!("text"));
            value.insert("role".into(), json!(role_name(text.role)));
            match &text.content {
                SceneTextContent::Literal(text) => {
                    value.insert("source".into(), json!("literal"));
                    value.insert("value".into(), json!(text));
                }
                SceneTextContent::Path(path) => {
                    value.insert("source".into(), json!("path"));
                    value.insert("path".into(), json!(path.join(".")));
                }
                SceneTextContent::Expr(expr) => {
                    value.insert("source".into(), json!("expr"));
                    value.insert("content".into(), puzzle_scene::scene_expr_json_value(expr));
                }
            }
            Value::Object(value)
        }
        SceneComponent::Button(button) => {
            json!({ "kind": "button", "label": puzzle_scene::scene_expr_json_value(&button.label), "effect": button.effect })
        }
        SceneComponent::Choice(choice) => {
            json!({ "kind": "choice", "label": puzzle_scene::scene_expr_json_value(&choice.label), "effect": choice.effect })
        }
        SceneComponent::Row(container) => {
            authored_container_value("row", &container.layout, &container.children)
        }
        SceneComponent::Column(container) => {
            authored_container_value("column", &container.layout, &container.children)
        }
        SceneComponent::Box(container) => {
            authored_container_value("box", &container.layout, &container.children)
        }
        SceneComponent::Conditional(conditional) => json!({
            "kind": "conditional", "condition": puzzle_scene::scene_expr_json_value(&conditional.condition),
            "children": conditional.children.iter().map(scene_component_value).collect::<Vec<_>>(),
            "elseChildren": conditional.else_children.iter().map(scene_component_value).collect::<Vec<_>>(),
        }),
    }
}

fn authored_container_value(
    kind: &str,
    layout: &SceneLayoutDef,
    children: &[SceneComponent],
) -> Value {
    json!({ "kind": kind, "layout": scene_layout_value(layout),
        "children": children.iter().map(scene_component_value).collect::<Vec<_>>() })
}

fn scene_layout_value(layout: &SceneLayoutDef) -> Value {
    let mut value = Map::new();
    if let SceneSpaceDef::Fill { weight } = layout.space {
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
    if layout.align != SceneLayoutDef::default().align {
        value.insert(
            "align".into(),
            json!(match layout.align {
                SceneAlignDef::Start => "start",
                SceneAlignDef::Center => "center",
                SceneAlignDef::End => "end",
                SceneAlignDef::Stretch => "stretch",
            }),
        );
    }
    if layout.distribute != SceneLayoutDef::default().distribute {
        value.insert(
            "distribute".into(),
            json!(match layout.distribute {
                SceneDistributionDef::Start => "start",
                SceneDistributionDef::Center => "center",
                SceneDistributionDef::End => "end",
                SceneDistributionDef::Between => "between",
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
            }),
        );
    }
    if layout.scroll {
        value.insert("scroll".into(), Value::Bool(true));
    }
    Value::Object(value)
}

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
fn resource_mode(selection: &ResourceSelection) -> &'static str {
    match selection {
        ResourceSelection::All => "all",
        ResourceSelection::Named(_) => "named",
    }
}
fn resource_names(selection: &ResourceSelection) -> Vec<String> {
    match selection {
        ResourceSelection::All => Vec::new(),
        ResourceSelection::Named(names) => names.clone(),
    }
}
fn lifetime_name(value: SceneStateLifetime) -> &'static str {
    match value {
        SceneStateLifetime::Instance => "instance",
        SceneStateLifetime::ResetOnStart => "reset_on_start",
        SceneStateLifetime::Persistent => "persistent",
    }
}
fn key_name(value: &KeyTrigger) -> String {
    match value {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}
fn role_name(value: puzzle_lang::SceneTextRoleDef) -> &'static str {
    match value {
        puzzle_lang::SceneTextRoleDef::Heading => "heading",
        puzzle_lang::SceneTextRoleDef::Subheading => "subheading",
        puzzle_lang::SceneTextRoleDef::Body => "body",
        puzzle_lang::SceneTextRoleDef::Caption => "caption",
    }
}
fn text_align_name(value: puzzle_lang::SceneTextAlignDef) -> &'static str {
    match value {
        puzzle_lang::SceneTextAlignDef::Start => "start",
        puzzle_lang::SceneTextAlignDef::Center => "center",
        puzzle_lang::SceneTextAlignDef::End => "end",
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }
}
