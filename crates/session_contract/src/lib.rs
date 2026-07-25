use std::collections::BTreeMap;

pub use puzzle_runtime_contract::RuntimeTheme;
use puzzle_runtime_contract::{
    RuntimeKeyTrigger, RuntimePresentationEvent, RuntimePuzzle3Snapshot,
    RuntimeResolvedRenderScene, RuntimeResolvedView2d, RuntimeSceneActionToken,
    RuntimeViewportSourceId, RuntimeVisualComposition, SolverStateSnapshot,
};
use puzzle_scene::{
    ComponentPlacement, ComponentVisibility, SceneLayout as SceneLayoutDef,
    SceneTextAlign as SceneTextAlignDef, SceneTextRole as SceneTextRoleDef,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeRendererState {
    TwoD(RuntimePuzzle2Snapshot),
    ThreeD(RuntimePuzzle3Snapshot),
}

pub trait RuntimePresentationBackend {
    type Error;
    type Output;

    fn present(&mut self, snapshot: &RuntimeSessionSnapshot) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct RuntimeSessionSnapshot {
    pub revision: u64,
    pub has_progress_save: bool,
    pub theme: RuntimeTheme,
    pub default_wait_ms: u64,
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
    pub presentation_events: Vec<RuntimePresentationEvent>,
    pub level_index: Option<usize>,
    pub level_count: usize,
    pub accepts_model_input: bool,
    pub viewport_sources: BTreeMap<RuntimeViewportSourceId, RuntimeRendererState>,
    pub surface: RuntimeSurface,
    pub busy: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// Development-only state paired with a player snapshot for editor inspection.
///
/// Player backends must consume [`RuntimeSessionSnapshot`] directly. This
/// contract contains authored names, solver state, and editable 2D projections
/// that are useful to development tools but are not runtime presentation input.
#[derive(Clone, Debug)]
pub struct RuntimeDevelopmentSessionSnapshot {
    pub player: RuntimeSessionSnapshot,
    pub levels: BTreeMap<String, RuntimeLevelRecord>,
    pub solver_state: SolverStateSnapshot,
    pub selected_level_index: usize,
    pub inputs: Vec<RuntimeInputBinding>,
    pub viewport_sources: BTreeMap<RuntimeViewportSourceId, RuntimeDevelopmentRendererState>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeDevelopmentRendererState {
    TwoD(RuntimePuzzle2DevelopmentSnapshot),
    ThreeD,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurface {
    pub root: Option<String>,
    pub focus: String,
    pub components: Vec<RuntimeSurfaceComponent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSurfaceComponent {
    pub id: String,
    pub placement: ComponentPlacement,
    pub visibility: ComponentVisibility,
    pub modal: bool,
    pub await_event: Option<String>,
    pub presentation: RuntimeComponentPresentation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeComponentPresentation {
    Ready(RuntimeResolvedScene),
    Error { error: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResolvedScene {
    pub layout: SceneLayoutDef,
    pub components: Vec<RuntimeResolvedSceneComponent>,
    pub keys: Option<Vec<RuntimeKeyBinding>>,
    pub events: Option<BTreeMap<String, RuntimeResolvedEventBinding>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResolvedEventBinding {
    pub pointer: bool,
    pub keys: Vec<RuntimeKeyTrigger>,
    pub action: Option<RuntimeSceneActionToken>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeKeyBinding {
    pub keys: Vec<RuntimeKeyTrigger>,
    pub action: RuntimeSceneActionToken,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeResolvedSceneComponent {
    Viewport {
        dimension: RuntimeViewportDimension,
        source: RuntimeViewportSourceId,
        layout: SceneLayoutDef,
    },
    Frame {
        kind: String,
        source: String,
        layout: SceneLayoutDef,
    },
    Text {
        role: SceneTextRoleDef,
        value: String,
        text_align: Option<SceneTextAlignDef>,
        layout: SceneLayoutDef,
    },
    Button {
        label: String,
        action: Option<RuntimeSceneActionToken>,
        layout: SceneLayoutDef,
    },
    Choice {
        label: String,
        action: Option<RuntimeSceneActionToken>,
        selected: bool,
        layout: SceneLayoutDef,
    },
    Row {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
    Column {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
    Box {
        layout: SceneLayoutDef,
        children: Vec<RuntimeResolvedSceneComponent>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeViewportDimension {
    TwoD,
    ThreeD,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Snapshot {
    pub view: RuntimeResolvedView2d,
    pub render_scene: RuntimeResolvedRenderScene,
    pub display_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2DevelopmentSnapshot {
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    pub settings: RuntimePuzzle2Settings,
    pub animation: RuntimeAnimationSettings,
    pub regions: Vec<RuntimeRegion2d>,
    pub resources: RuntimePuzzle2Resources,
    pub cells: Vec<RuntimePuzzle2Cell>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Settings {
    pub render: RuntimeRender2d,
    pub input_buffer: RuntimeInputBufferSettings,
    pub animation: RuntimeAnimationSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRender2d {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeInputBufferSettings {
    pub queue_during_wait: bool,
    pub fast_forward_wait: bool,
    pub min_wait_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeAnimationSettings {
    pub tween: RuntimeTweenSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeTweenSettings {
    pub enabled: bool,
    pub interval_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeRegion2d {
    pub index: usize,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Resources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<RuntimeResourceSelection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visuals: Option<RuntimeResourceSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResourceSelection {
    pub mode: RuntimeResourceSelectionMode,
    pub names: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResourceSelectionMode {
    All,
    Named,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Cell {
    pub x: u16,
    pub y: u16,
    pub render_order: u64,
    pub layers: Vec<RuntimePuzzle2Layer>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePuzzle2Layer {
    pub layer: u16,
    pub object_id: u16,
    pub object: String,
    pub visual: String,
    pub render_priority: u16,
    pub render_order: u64,
    pub composition: RuntimeVisualComposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLevelRecord {
    pub id: String,
    pub name: String,
    pub puzzle: String,
    pub pack: Option<String>,
    pub ordinal: usize,
    pub cleared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInputBinding {
    pub id: u16,
    pub name: String,
    pub triggers: Vec<RuntimeKeyTrigger>,
}
