use puzzle_core::{
    GridCompiledGame, GridCoord, GridSize, GridState, LayerId, MarkId, ObjectId, RuleId, Size2,
    Size3,
};
pub use puzzle_scene::SceneEffect as LifecycleCommand;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use std::fmt;

pub const STANDALONE_RUNTIME_EXPORT_VERSION: u16 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeLoadedDocumentBundle<Document> {
    pub version: u16,
    pub document: Document,
}

impl<'de, Document> Deserialize<'de> for RuntimeLoadedDocumentBundle<Document>
where
    Document: Deserialize<'de>,
{
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        struct BundleVisitor<Document>(std::marker::PhantomData<Document>);

        impl<'de, Document> Visitor<'de> for BundleVisitor<Document>
        where
            Document: Deserialize<'de>,
        {
            type Value = RuntimeLoadedDocumentBundle<Document>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a versioned runtime loaded document bundle")
            }

            fn visit_map<Access>(self, mut map: Access) -> Result<Self::Value, Access::Error>
            where
                Access: MapAccess<'de>,
            {
                let mut version = None;
                let mut document = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "version" => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            let value = map.next_value::<u16>()?;
                            if value != STANDALONE_RUNTIME_EXPORT_VERSION {
                                return Err(de::Error::custom(format!(
                                    "unsupported runtimeLoadedDocument version: {value}"
                                )));
                            }
                            version = Some(value);
                        }
                        "document" => {
                            if document.is_some() {
                                return Err(de::Error::duplicate_field("document"));
                            }
                            document = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(&field, &["version", "document"]));
                        }
                    }
                }
                Ok(RuntimeLoadedDocumentBundle {
                    version: version.ok_or_else(|| de::Error::missing_field("version"))?,
                    document: document.ok_or_else(|| de::Error::missing_field("document"))?,
                })
            }
        }

        deserializer.deserialize_map(BundleVisitor(std::marker::PhantomData))
    }
}

impl<Document> RuntimeLoadedDocumentBundle<Document> {
    pub fn new(document: Document) -> Self {
        Self {
            version: STANDALONE_RUNTIME_EXPORT_VERSION,
            document,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandaloneRuntimeExport<Document> {
    pub runtime_loaded_document: RuntimeLoadedDocumentBundle<Document>,
}

impl<Document> StandaloneRuntimeExport<Document> {
    pub fn new(document: Document) -> Self {
        Self {
            runtime_loaded_document: RuntimeLoadedDocumentBundle::new(document),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionAction {
    Snapshot,
    Input { name: String },
    Resume,
    DebugInput { name: String },
    Undo,
    Redo,
    Restart,
    NextLevel,
    PreviousLevel,
    GotoLevel { level: usize },
    SceneEffect { effect: LifecycleCommand },
    Command { name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeModelKind {
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCoord {
    pub x: u16,
    pub y: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMarkValue {
    pub mark: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeStateSnapshot {
    TwoD(RuntimeStateSnapshot2d),
    ThreeD(RuntimeStateSnapshot3d),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeStateSnapshot2d {
    pub kind: RuntimeModelKind,
    pub width: u16,
    pub height: u16,
    pub layer_count: u16,
    pub slots: Vec<u16>,
    pub slot_marks: Vec<Vec<RuntimeMarkValue>>,
    pub cell_marks: Vec<Vec<RuntimeMarkValue>>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeStateSnapshot3d {
    pub kind: RuntimeModelKind,
    pub width: u16,
    pub depth: u16,
    pub height: u16,
    pub layer_count: u16,
    pub slots: Vec<u16>,
    pub slot_marks: Vec<Vec<RuntimeMarkValue>>,
    pub cell_marks: Vec<Vec<RuntimeMarkValue>>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum SolverStateSnapshot {
    #[serde(rename = "2d", rename_all = "camelCase")]
    TwoD {
        width: u16,
        height: u16,
        layer_count: u16,
        slots: Vec<u16>,
        slot_marks: Vec<Vec<RuntimeMarkValue>>,
        cell_marks: Vec<Vec<RuntimeMarkValue>>,
        variables: Vec<i64>,
        level_fired_rules: Vec<u16>,
    },
    #[serde(rename = "puzzle3d", rename_all = "camelCase")]
    ThreeD {
        width: u16,
        depth: u16,
        height: u16,
        layer_count: u16,
        slots: Vec<u16>,
        slot_marks: Vec<Vec<RuntimeMarkValue>>,
        cell_marks: Vec<Vec<RuntimeMarkValue>>,
        variables: Vec<i64>,
        level_fired_rules: Vec<u16>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverPreparedArtifact {
    pub artifact_id: String,
    pub model_kind: RuntimeModelKind,
    pub level_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchRequest {
    pub level_index: usize,
    pub state: SolverStateSnapshot,
    pub materialize_level_start: bool,
    pub max_depth: u32,
    pub max_nodes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverSearchStatus {
    Paused,
    Solved,
    Exhausted,
    BudgetExceeded,
    ResourceLimit,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchStats {
    pub visited: usize,
    pub expanded: usize,
    pub frontier: usize,
    pub max_depth_reached: u32,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverSearchProgress {
    pub visited: usize,
    pub expanded: usize,
    pub frontier: usize,
    pub max_depth_reached: u32,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverObservation {
    pub progress: SolverSearchProgress,
    pub state: SolverStateSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverMove {
    pub id: u16,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverStep {
    pub index: usize,
    #[serde(rename = "move")]
    pub input: Option<SolverMove>,
    pub state: SolverStateSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverResult {
    pub model: RuntimeModelKind,
    pub result: SolverSearchStatus,
    pub depth: Option<u32>,
    pub moves: Vec<SolverMove>,
    pub steps: Vec<SolverStep>,
    pub observations: Vec<SolverObservation>,
    pub stats: SolverSearchStats,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolverAdvanceResponse {
    pub status: SolverSearchStatus,
    pub stats: SolverSearchStats,
    pub observation: Option<SolverObservation>,
    pub result: Option<SolverResult>,
}

impl RuntimeStateSnapshot2d {
    pub fn from_state(state: &GridState<2, Size2>) -> Self {
        Self {
            kind: RuntimeModelKind::TwoD,
            width: state.size.width,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            slot_marks: runtime_marks(state.slot_mark()),
            cell_marks: runtime_marks(state.cell_mark()),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state(self, game: &GridCompiledGame<2>) -> Result<GridState<2, Size2>, String> {
        if self.kind != RuntimeModelKind::TwoD {
            return Err("runtime 2d state kind must be 2d".to_string());
        }
        decode_runtime_state(
            game,
            Size2::new(self.width, self.height),
            self.layer_count,
            self.slots,
            self.slot_marks,
            self.cell_marks,
            self.variables,
            self.level_fired_rules,
        )
    }
}

impl SolverStateSnapshot {
    pub fn from_state2(state: &GridState<2, Size2>) -> Self {
        Self::TwoD {
            width: state.size.width,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            slot_marks: runtime_marks(state.slot_mark()),
            cell_marks: runtime_marks(state.cell_mark()),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn from_state3(state: &GridState<3, Size3>) -> Self {
        Self::ThreeD {
            width: state.size.width,
            depth: state.size.depth,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            slot_marks: runtime_marks(state.slot_mark()),
            cell_marks: runtime_marks(state.cell_mark()),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state2(self, game: &GridCompiledGame<2>) -> Result<GridState<2, Size2>, String> {
        let Self::TwoD {
            width,
            height,
            layer_count,
            slots,
            slot_marks,
            cell_marks,
            variables,
            level_fired_rules,
        } = self
        else {
            return Err("solver state kind must be 2d".to_string());
        };
        decode_runtime_state(
            game,
            Size2::new(width, height),
            layer_count,
            slots,
            slot_marks,
            cell_marks,
            variables,
            level_fired_rules,
        )
    }

    pub fn into_state3(self, game: &GridCompiledGame<3>) -> Result<GridState<3, Size3>, String> {
        let Self::ThreeD {
            width,
            depth,
            height,
            layer_count,
            slots,
            slot_marks,
            cell_marks,
            variables,
            level_fired_rules,
        } = self
        else {
            return Err("solver state kind must be puzzle3d".to_string());
        };
        decode_runtime_state(
            game,
            Size3::new(width, depth, height),
            layer_count,
            slots,
            slot_marks,
            cell_marks,
            variables,
            level_fired_rules,
        )
    }
}

impl RuntimeStateSnapshot3d {
    pub fn from_state(state: &GridState<3, Size3>) -> Self {
        Self {
            kind: RuntimeModelKind::ThreeD,
            width: state.size.width,
            depth: state.size.depth,
            height: state.size.height,
            layer_count: state.layer_count,
            slots: state.slots().iter().map(|object| object.0).collect(),
            slot_marks: runtime_marks(state.slot_mark()),
            cell_marks: runtime_marks(state.cell_mark()),
            variables: state.visible_variables().to_vec(),
            level_fired_rules: state
                .level_fired_rules()
                .iter()
                .map(|rule| rule.0)
                .collect(),
        }
    }

    pub fn into_state(self, game: &GridCompiledGame<3>) -> Result<GridState<3, Size3>, String> {
        if self.kind != RuntimeModelKind::ThreeD {
            return Err("runtime 3d state kind must be 3d".to_string());
        }
        decode_runtime_state(
            game,
            Size3::new(self.width, self.depth, self.height),
            self.layer_count,
            self.slots,
            self.slot_marks,
            self.cell_marks,
            self.variables,
            self.level_fired_rules,
        )
    }
}

fn runtime_marks(marks: Vec<Vec<puzzle_core::SlotMark>>) -> Vec<Vec<RuntimeMarkValue>> {
    marks
        .into_iter()
        .map(|entries| {
            entries
                .into_iter()
                .map(|entry| RuntimeMarkValue {
                    mark: entry.mark.0,
                    value: entry.value,
                })
                .collect()
        })
        .collect()
}

fn decode_runtime_state<const D: usize, Size: GridSize<D>>(
    game: &GridCompiledGame<D>,
    size: Size,
    layer_count: u16,
    slots: Vec<u16>,
    slot_marks: Vec<Vec<RuntimeMarkValue>>,
    cell_marks: Vec<Vec<RuntimeMarkValue>>,
    variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
) -> Result<GridState<D, Size>, String> {
    if layer_count != game.layer_count {
        return Err(format!(
            "runtime state layerCount mismatch: expected {}, got {layer_count}",
            game.layer_count
        ));
    }
    let axes = size.axes();
    let cell_count = axes
        .iter()
        .try_fold(1usize, |count, axis| count.checked_mul(usize::from(*axis)));
    let cell_count =
        cell_count.ok_or_else(|| "runtime state dimensions are too large".to_string())?;
    let slot_count = cell_count
        .checked_mul(usize::from(layer_count))
        .ok_or_else(|| "runtime state dimensions are too large".to_string())?;
    if slots.len() != slot_count {
        return Err(format!(
            "runtime state slots length mismatch: expected {slot_count}, got {}",
            slots.len()
        ));
    }
    if slot_marks.len() != slot_count {
        return Err(format!(
            "runtime state slotMarks length mismatch: expected {slot_count}, got {}",
            slot_marks.len()
        ));
    }
    if cell_marks.len() != cell_count {
        return Err(format!(
            "runtime state cellMarks length mismatch: expected {cell_count}, got {}",
            cell_marks.len()
        ));
    }

    let mut state = GridState::<D, Size>::empty_sized_with_variables(
        size,
        layer_count,
        game.object_count(),
        variables,
    )
    .map_err(|error| format!("{error:?}"))?;
    for (slot, object) in slots.into_iter().enumerate() {
        let layer = slot % usize::from(layer_count);
        let cell = slot / usize::from(layer_count);
        let position = runtime_cell_position::<D>(&axes, cell)?;
        if object != 0 {
            let object = ObjectId(object);
            let expected_layer = game
                .object_layer(object)
                .ok_or_else(|| format!("runtime state contains unknown object {}", object.0))?;
            if usize::from(expected_layer.0) != layer {
                return Err(format!(
                    "runtime state object {} is in layer {layer}, expected {}",
                    object.0, expected_layer.0
                ));
            }
            state
                .place_object_at(game, position, object)
                .map_err(|error| format!("{error:?}"))?;
        }
        for mark in &slot_marks[slot] {
            state
                .set_slot_mark_at(
                    position,
                    LayerId(layer as u16),
                    MarkId(mark.mark),
                    mark.value,
                )
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    for (cell, marks) in cell_marks.into_iter().enumerate() {
        let position = runtime_cell_position::<D>(&axes, cell)?;
        for mark in marks {
            state
                .set_cell_mark_at(position, MarkId(mark.mark), mark.value)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    for rule in level_fired_rules {
        state.mark_level_rule_fired(RuleId(rule));
    }
    Ok(state)
}

fn runtime_cell_position<const D: usize>(
    axes: &[u16; D],
    mut cell: usize,
) -> Result<GridCoord<D>, String> {
    let mut position = [0u16; D];
    for (axis, length) in axes.iter().enumerate() {
        if *length == 0 {
            return Err("runtime state dimensions must be positive".to_string());
        }
        position[axis] = u16::try_from(cell % usize::from(*length))
            .map_err(|_| "runtime state coordinate is out of range".to_string())?;
        cell /= usize::from(*length);
    }
    Ok(GridCoord::new(position))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeChangedCell {
    pub position: RuntimeCoord,
    pub objects: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMarkValueMatch {
    Any,
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimePatchOp {
    Add {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Remove {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Move {
        from: RuntimeCoord,
        to: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
    },
    Replace {
        position: RuntimeCoord,
        remove: u16,
        add: u16,
    },
    UpdateVariable {
        variable: u16,
    },
    SetMark {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
        mark: u16,
    },
    RemoveMark {
        position: RuntimeCoord,
        #[serde(rename = "objectId")]
        object_id: u16,
        mark: u16,
        #[serde(rename = "matchValue")]
        match_value: RuntimeMarkValueMatch,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeVisualSpace {
    World,
    Local,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeVisualTransform {
    Rotate {
        degrees: f64,
        axis: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Translate {
        value: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Scale {
        value: [f64; 3],
        space: RuntimeVisualSpace,
    },
    Flip {
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVisualState {
    pub transforms: Vec<RuntimeVisualTransform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVisualTween {
    pub from: RuntimeVisualState,
    pub to: RuntimeVisualState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeAnimationEvent {
    Animation {
        name: String,
        position: RuntimeCoord,
    },
    Move {
        name: String,
        #[serde(rename = "objectId")]
        object_id: u16,
        #[serde(rename = "visualTween", skip_serializing_if = "Option::is_none")]
        visual_tween: Option<RuntimeVisualTween>,
        from: RuntimeCoord,
        to: RuntimeCoord,
    },
    CantMove {
        name: String,
        #[serde(rename = "objectId")]
        object_id: u16,
        position: RuntimeCoord,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePresentationEvent {
    pub scene: String,
    pub puzzle: String,
    pub level_index: Option<usize>,
    #[serde(flatten)]
    pub event: RuntimePresentationEventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimePresentationEventKind {
    PlaySfx { name: String },
    PlayMusic { name: String },
    PauseMusic { name: Option<String> },
    ResumeMusic { name: Option<String> },
    StopMusic { name: Option<String> },
    Message { text: String },
    Wait { milliseconds: u64 },
    Animation { animation: RuntimeAnimationEvent },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeTransitionCommand {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAnimationOffset {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeEffect {
    Win,
    Restart,
    NextLevel,
    Again,
    Checkpoint,
    ClearCheckpoint,
    PlaySfx {
        name: String,
    },
    PlayMusic {
        name: String,
    },
    PauseMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    ResumeMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    StopMusic {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Wait {
        milliseconds: u64,
    },
    WaitAnimation,
    EmitAnimation {
        name: String,
        component: u16,
        offset: RuntimeAnimationOffset,
    },
    Message {
        text: String,
        literal: bool,
    },
    Scene {
        effect: LifecycleCommand,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleFiring {
    pub rule_id: u16,
    pub patch: Vec<RuntimePatchOp>,
    pub progressed: bool,
    pub observable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTransitionProgramOutcome {
    pub state: RuntimeStateSnapshot,
    pub cancelled: bool,
    pub completed: bool,
    pub commands: Vec<RuntimeTransitionCommand>,
    pub effects: Vec<RuntimeEffect>,
    pub firings: Vec<RuntimeRuleFiring>,
    pub animation_events: Vec<RuntimeAnimationEvent>,
}

impl RuntimeTransitionProgramOutcome {
    pub fn to_json_string(&self) -> Result<String, RuntimeJsonError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeJsonError::InvalidJson(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTransitionCurrentOutcome {
    pub cancelled: bool,
    pub changed: bool,
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<RuntimeStateSnapshot>,
    pub commands: Vec<RuntimeTransitionCommand>,
    pub effects: Vec<RuntimeEffect>,
    pub firings: Vec<RuntimeRuleFiring>,
    pub animation_events: Vec<RuntimeAnimationEvent>,
    pub state_hash: u64,
    pub state_hash_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_state_handle: Option<u32>,
    pub changed_cells: Vec<RuntimeChangedCell>,
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
}

impl RuntimeTransitionCurrentOutcome {
    pub fn to_json_string(&self) -> Result<String, RuntimeJsonError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeJsonError::InvalidJson(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CameraEffect {
    SetYaw(i16),
    SetPitch(i16),
    SetRoll(i16),
    SetZoom(u16),
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeJsonError {
    InvalidJson(String),
}

impl std::fmt::Display for RuntimeJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => {
                write!(f, "invalid runtime JSON: {error}")
            }
        }
    }
}

impl std::error::Error for RuntimeJsonError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_tween_contract_keeps_extensible_visual_state_endpoints() {
        let tween = RuntimeVisualTween {
            from: RuntimeVisualState {
                transforms: vec![RuntimeVisualTransform::Scale {
                    value: [1.0, 1.0, 1.0],
                    space: RuntimeVisualSpace::Local,
                }],
                opacity: Some(0.0),
            },
            to: RuntimeVisualState {
                transforms: vec![RuntimeVisualTransform::Scale {
                    value: [2.0, 2.0, 1.0],
                    space: RuntimeVisualSpace::Local,
                }],
                opacity: Some(1.0),
            },
        };

        let value = serde_json::to_value(tween).unwrap();
        assert_eq!(value["from"]["transforms"][0]["kind"], "scale");
        assert_eq!(value["from"]["opacity"], 0.0);
        assert_eq!(value["to"]["opacity"], 1.0);
    }

    #[test]
    fn explicit_visual_animation_uses_the_renderer_animation_contract() {
        let value = serde_json::to_value(RuntimeAnimationEvent::Animation {
            name: "flash".to_string(),
            position: RuntimeCoord {
                x: 2,
                y: 3,
                z: None,
            },
        })
        .unwrap();

        assert_eq!(value["kind"], "animation");
        assert_eq!(value["name"], "flash");
        assert_eq!(value["position"]["x"], 2);
        assert_eq!(value["position"]["y"], 3);
    }
    use serde_json::json;

    #[test]
    fn session_action_uses_a_tagged_source_free_wire_contract() {
        let action = SessionAction::Input {
            name: "front".to_string(),
        };
        let value = serde_json::to_value(&action).unwrap();
        assert_eq!(value, json!({"kind": "input", "name": "front"}));
        assert_eq!(
            serde_json::from_value::<SessionAction>(value).unwrap(),
            action
        );
        let resume = serde_json::to_value(SessionAction::Resume).unwrap();
        assert_eq!(resume, json!({"kind": "resume"}));
        assert_eq!(
            serde_json::from_value::<SessionAction>(resume).unwrap(),
            SessionAction::Resume
        );
        assert!(
            serde_json::from_value::<SessionAction>(json!({
                "kind": "input",
                "name": "front",
                "url": "/api/input/front"
            }))
            .is_err()
        );
    }

    #[test]
    fn scene_effect_action_round_trips_typed_expression_without_text_conversion() {
        let action = SessionAction::SceneEffect {
            effect: LifecycleCommand::Goto {
                scene: "sokoban".to_string(),
                params: vec![puzzle_scene::SceneEffectParam::Level(
                    puzzle_scene::SceneExpr::Path(vec!["selected_level".to_string()]),
                )],
            },
        };

        let value = serde_json::to_value(&action).unwrap();
        assert_eq!(value["kind"], "scene_effect");
        assert_eq!(value["effect"]["kind"], "goto");
        assert_eq!(value["effect"]["params"][0]["value"]["kind"], "path");
        assert_eq!(
            serde_json::from_value::<SessionAction>(value).unwrap(),
            action
        );
    }

    #[test]
    fn solver_state_snapshot_preserves_slot_and_cell_marks() {
        let snapshot = SolverStateSnapshot::TwoD {
            width: 1,
            height: 1,
            layer_count: 1,
            slots: vec![7],
            slot_marks: vec![vec![RuntimeMarkValue {
                mark: 2,
                value: Some(11),
            }]],
            cell_marks: vec![vec![RuntimeMarkValue {
                mark: 3,
                value: None,
            }]],
            variables: vec![5],
            level_fired_rules: vec![13],
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["kind"], "2d");
        assert_eq!(value["slotMarks"][0][0], json!({"mark": 2, "value": 11}));
        assert_eq!(value["cellMarks"][0][0], json!({"mark": 3}));
        assert!(value.get("mark").is_none());
        assert_eq!(
            serde_json::from_value::<SolverStateSnapshot>(value).unwrap(),
            snapshot
        );
    }

    #[test]
    fn transition_program_outcome_serializes_owned_outer_schema() {
        let outcome = RuntimeTransitionProgramOutcome {
            state: RuntimeStateSnapshot::TwoD(RuntimeStateSnapshot2d {
                kind: RuntimeModelKind::TwoD,
                width: 2,
                height: 1,
                layer_count: 1,
                slots: vec![0, 1],
                slot_marks: vec![Vec::new(), Vec::new()],
                cell_marks: vec![Vec::new(), Vec::new()],
                variables: Vec::new(),
                level_fired_rules: Vec::new(),
            }),
            cancelled: false,
            completed: true,
            commands: vec![RuntimeTransitionCommand::Win],
            effects: vec![RuntimeEffect::Win],
            firings: vec![RuntimeRuleFiring {
                rule_id: 3,
                patch: vec![RuntimePatchOp::Remove {
                    position: RuntimeCoord {
                        x: 0,
                        y: 0,
                        z: None,
                    },
                    object_id: 1,
                }],
                progressed: true,
                observable: false,
            }],
            animation_events: vec![RuntimeAnimationEvent::Move {
                name: "tween".to_string(),
                object_id: 1,
                visual_tween: None,
                from: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                to: RuntimeCoord {
                    x: 1,
                    y: 0,
                    z: None,
                },
            }],
        };

        let value: serde_json::Value =
            serde_json::from_str(&outcome.to_json_string().expect("outcome serializes"))
                .expect("outcome JSON parses");

        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            [
                "animationEvents",
                "cancelled",
                "commands",
                "completed",
                "effects",
                "firings",
                "state"
            ]
        );
    }

    #[test]
    fn transition_current_outcome_serializes_owned_outer_schema() {
        let outcome = RuntimeTransitionCurrentOutcome {
            cancelled: false,
            changed: true,
            completed: false,
            state: None,
            commands: Vec::new(),
            effects: Vec::new(),
            firings: Vec::new(),
            animation_events: Vec::new(),
            state_hash: 12,
            state_hash_key: "12".to_string(),
            previous_state_handle: Some(4),
            changed_cells: vec![RuntimeChangedCell {
                position: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                objects: vec![1],
            }],
            variables: vec![9],
            level_fired_rules: vec![2],
        };

        let value: serde_json::Value =
            serde_json::from_str(&outcome.to_json_string().expect("outcome serializes"))
                .expect("outcome JSON parses");

        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            [
                "animationEvents",
                "cancelled",
                "changed",
                "changedCells",
                "commands",
                "completed",
                "effects",
                "firings",
                "levelFiredRules",
                "previousStateHandle",
                "stateHash",
                "stateHashKey",
                "variables"
            ]
        );
        assert!(value.get("state").is_none());
        assert!(value["changedCells"][0]["position"].get("z").is_none());
    }
}
#[test]
fn presentation_timeline_event_round_trips_with_origin_context() {
    let event = RuntimePresentationEvent {
        scene: "playing".to_string(),
        puzzle: "board".to_string(),
        level_index: Some(3),
        event: RuntimePresentationEventKind::Wait { milliseconds: 80 },
    };
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["kind"], "wait");
    assert_eq!(value["levelIndex"], 3);
    assert_eq!(
        serde_json::from_value::<RuntimePresentationEvent>(value).unwrap(),
        event
    );
}
