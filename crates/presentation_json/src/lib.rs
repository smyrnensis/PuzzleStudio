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
        "levels": snapshot.levels.iter().map(|(name, level)| (name.clone(), json!({
            "id": level.id,
            "name": level.name,
            "puzzle": level.puzzle,
            "pack": level.pack,
            "ordinal": level.ordinal,
            "progress": { "cleared": level.cleared },
        }))).collect::<Map<_, _>>(),
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
                .map(surface_component_value)
                .collect::<Result<Vec<_>, _>>()?,
        },
        "solverState": snapshot.solver_state,
        "selectedLevelIndex": snapshot.selected_level_index,
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
    }))
}

pub fn resolved_scene_to_value(scene: &RuntimeResolvedScene) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
    value.insert("name".into(), Value::String(scene.name.clone()));
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
                            "effect": binding.effect,
                            "keys": binding.keys,
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
                            }),
                        )
                    })
                    .collect(),
            ),
        );
    }
    Ok(Value::Object(value))
}

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
    }
}

fn surface_component_value(
    component: &RuntimeSurfaceComponent,
) -> Result<Value, serde_json::Error> {
    let mut value = Map::new();
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
        "placement".into(),
        serde_json::to_value(component.placement)?,
    );
    value.insert(
        "visibility".into(),
        serde_json::to_value(component.visibility)?,
    );
    value.insert("modal".into(), Value::Bool(component.modal));
    if !component.properties.is_empty() {
        value.insert(
            "properties".into(),
            scene_values_value(&component.properties),
        );
    }
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
        RuntimeResolvedSceneComponent::Conditional {
            condition,
            children,
            else_children,
        } => json!({
            "kind": "conditional", "condition": condition,
            "children": children.iter().map(resolved_component_value).collect::<Vec<_>>(),
            "elseChildren": else_children.iter().map(resolved_component_value).collect::<Vec<_>>(),
        }),
    }
}

fn container_value(
    kind: &str,
    layout: &SceneLayoutDef,
    children: &[RuntimeResolvedSceneComponent],
) -> Value {
    json!({ "kind": kind, "layout": scene_layout_value(layout),
        "children": children.iter().map(resolved_component_value).collect::<Vec<_>>() })
}

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
            }),
        );
    }
    if layout.scroll {
        value.insert("scroll".into(), Value::Bool(true));
    }
    Value::Object(value)
}

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
    }
}
