use puzzle_core::{
    GridCompiledGame, GridCoord, GridExecutableProgram, GridGoalCondition, GridInput, GridLevel,
    GridLevelBundle, GridSize, GridState, LayerId, LocalFrame, MarkId, ObjectId, RuleId, Size2,
    Size3,
};
pub use puzzle_scene::SceneEffect as LifecycleCommand;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned, ser::SerializeStruct,
};
use serde_json::Value;

pub const RUNTIME_CONTRACT_VERSION: u16 = 7;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionAction {
    Snapshot,
    Input { name: String },
    DebugInput { name: String },
    Undo,
    Redo,
    Restart,
    NextLevel,
    PreviousLevel,
    GotoLevel { level: usize },
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

pub trait RuntimeGridSize<const D: usize>: GridSize<D> + Serialize + DeserializeOwned {
    type Snapshot: Clone + Serialize + DeserializeOwned;

    fn snapshot(state: &GridState<D, Self>) -> Self::Snapshot;

    fn state(
        game: &GridCompiledGame<D>,
        snapshot: Self::Snapshot,
    ) -> Result<GridState<D, Self>, String>;
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

impl RuntimeGridSize<2> for Size2 {
    type Snapshot = RuntimeStateSnapshot2d;

    fn snapshot(state: &GridState<2, Self>) -> Self::Snapshot {
        RuntimeStateSnapshot2d::from_state(state)
    }

    fn state(
        game: &GridCompiledGame<2>,
        snapshot: Self::Snapshot,
    ) -> Result<GridState<2, Self>, String> {
        snapshot.into_state(game)
    }
}

impl RuntimeGridSize<3> for Size3 {
    type Snapshot = RuntimeStateSnapshot3d;

    fn snapshot(state: &GridState<3, Self>) -> Self::Snapshot {
        RuntimeStateSnapshot3d::from_state(state)
    }

    fn state(
        game: &GridCompiledGame<3>,
        snapshot: Self::Snapshot,
    ) -> Result<GridState<3, Self>, String> {
        snapshot.into_state(game)
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeAnimationEvent {
    Move {
        name: String,
        #[serde(rename = "objectId")]
        object_id: u16,
        #[serde(rename = "fromObject", skip_serializing_if = "Option::is_none")]
        from_object: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePresentationEvent {
    pub scene: String,
    pub puzzle: String,
    pub level_index: Option<usize>,
    #[serde(flatten)]
    pub event: RuntimePresentationEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleEffects {
    pub rule: u16,
    pub effects: Vec<RuntimeEffect>,
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
    pub fn to_json_string(&self) -> Result<String, RuntimeContractError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))
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
    pub fn to_json_string(&self) -> Result<String, RuntimeContractError> {
        serde_json::to_string(self)
            .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLifecycle<Program> {
    pub on_level_start: Option<Program>,
    pub on_level_clear: Option<Program>,
    pub on_last_level_clear: Option<Program>,
}

impl<Program> Default for RuntimeLifecycle<Program> {
    fn default() -> Self {
        Self {
            on_level_start: None,
            on_level_clear: None,
            on_last_level_clear: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContract<Model> {
    pub version: u16,
    pub model: Model,
}

impl<Model: RuntimeModel> RuntimeContract<Model> {
    pub fn checked_new(model: Model) -> Result<Self, RuntimeContractError> {
        let contract = Self {
            version: RUNTIME_CONTRACT_VERSION,
            model,
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), RuntimeContractError> {
        if self.version != RUNTIME_CONTRACT_VERSION {
            return Err(RuntimeContractError::UnsupportedVersion {
                version: self.version,
            });
        }
        self.model.validate()
    }
}

pub trait RuntimeModel {
    fn validate(&self) -> Result<(), RuntimeContractError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridPresentation<CameraEffect> {
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rule_camera_effects: Vec<Vec<CameraEffect>>,
    pub on_level_start_camera_effects: Vec<Vec<CameraEffect>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridRuntimeModel<const D: usize, Size: RuntimeGridSize<D>, CameraEffect> {
    pub game: GridCompiledGame<D>,
    pub inputs: Vec<GridInput<D>>,
    pub level_bundle: GridLevelBundle<D, Size>,
    pub goal: Option<GridGoalCondition<D>>,
    pub rule_effects: Vec<RuntimeRuleEffects>,
    pub lifecycle: RuntimeLifecycle<GridExecutableProgram<D>>,
    pub presentation: GridPresentation<CameraEffect>,
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

impl<const D: usize, Size: RuntimeGridSize<D>, PresentationEffect>
    GridRuntimeModel<D, Size, PresentationEffect>
{
    pub fn checked_new(
        game: GridCompiledGame<D>,
        inputs: Vec<GridInput<D>>,
        level_bundle: GridLevelBundle<D, Size>,
        goal: Option<GridGoalCondition<D>>,
        rule_effects: Vec<RuntimeRuleEffects>,
        lifecycle: RuntimeLifecycle<GridExecutableProgram<D>>,
        presentation: GridPresentation<PresentationEffect>,
    ) -> Result<Self, RuntimeContractError> {
        let model = Self {
            game,
            inputs,
            level_bundle,
            goal,
            rule_effects,
            lifecycle,
            presentation,
        };
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> Result<(), RuntimeContractError> {
        validate_grid_runtime_model(self)
    }
}

impl<const D: usize, Size: RuntimeGridSize<D>, PresentationEffect> RuntimeModel
    for GridRuntimeModel<D, Size, PresentationEffect>
{
    fn validate(&self) -> Result<(), RuntimeContractError> {
        validate_grid_runtime_model(self)
    }
}

fn validate_grid_runtime_model<const D: usize, Size: RuntimeGridSize<D>, PresentationEffect>(
    model: &GridRuntimeModel<D, Size, PresentationEffect>,
) -> Result<(), RuntimeContractError> {
    model
        .game
        .validate()
        .map_err(|error| RuntimeContractError::InvalidGame(format!("{error:?}")))?;
    if model.level_bundle.game != model.game {
        return Err(RuntimeContractError::InvalidLevelBundle(
            "level bundle game does not match runtime contract game".to_string(),
        ));
    }
    model
        .level_bundle
        .validate()
        .map_err(|error| RuntimeContractError::InvalidLevelBundle(format!("{error:?}")))?;
    let rule_count = model.game.executable_program().rule_count();
    let level_start_rule_count = model
        .lifecycle
        .on_level_start
        .as_ref()
        .map_or(0, |program| program.rule_count());
    if model.presentation.rule_camera_effects.len() != rule_count {
        return Err(RuntimeContractError::InvalidPresentationEffects {
            owner: "ruleCameraEffects".to_string(),
        });
    }
    if model.presentation.on_level_start_camera_effects.len() != level_start_rule_count {
        return Err(RuntimeContractError::InvalidPresentationEffects {
            owner: "onLevelStartCameraEffects".to_string(),
        });
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(
    serialize = "Snapshot: Serialize",
    deserialize = "Snapshot: DeserializeOwned"
))]
struct RuntimeGridLevelBundleWire<const D: usize, Snapshot> {
    levels: Vec<RuntimeGridLevelWire<D, Snapshot>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(
    serialize = "Snapshot: Serialize",
    deserialize = "Snapshot: DeserializeOwned"
))]
struct RuntimeGridLevelWire<const D: usize, Snapshot> {
    name: String,
    initial_state: Snapshot,
    program: GridExecutableProgram<D>,
    level_start_program: Option<GridExecutableProgram<D>>,
    level_clear_program: Option<GridExecutableProgram<D>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "Snapshot: DeserializeOwned, CameraEffect: DeserializeOwned"))]
struct RuntimeGridModelWire<const D: usize, Snapshot, CameraEffect> {
    game: GridCompiledGame<D>,
    inputs: Vec<GridInput<D>>,
    level_bundle: RuntimeGridLevelBundleWire<D, Snapshot>,
    goal: Option<GridGoalCondition<D>>,
    rule_effects: Vec<RuntimeRuleEffects>,
    lifecycle: RuntimeLifecycle<GridExecutableProgram<D>>,
    presentation: GridPresentation<CameraEffect>,
}

fn runtime_level_bundle_wire<const D: usize, Size>(
    bundle: &GridLevelBundle<D, Size>,
) -> RuntimeGridLevelBundleWire<D, Size::Snapshot>
where
    Size: RuntimeGridSize<D>,
{
    RuntimeGridLevelBundleWire::<D, Size::Snapshot> {
        levels: bundle
            .levels
            .iter()
            .map(|level| RuntimeGridLevelWire {
                name: level.name.clone(),
                initial_state: Size::snapshot(&level.initial_state),
                program: level.program.clone(),
                level_start_program: level.level_start_program.clone(),
                level_clear_program: level.level_clear_program.clone(),
            })
            .collect(),
    }
}

fn runtime_level_bundle_from_wire<const D: usize, Size>(
    game: &GridCompiledGame<D>,
    wire: RuntimeGridLevelBundleWire<D, Size::Snapshot>,
) -> Result<GridLevelBundle<D, Size>, String>
where
    Size: RuntimeGridSize<D>,
{
    let levels = wire
        .levels
        .into_iter()
        .map(|level| {
            let initial_state = Size::state(game, level.initial_state)?;
            Ok(GridLevel::new(
                level.name,
                initial_state,
                level.program,
                level.level_start_program,
                level.level_clear_program,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    GridLevelBundle::checked_new(game.clone(), levels).map_err(|error| format!("{error:?}"))
}

impl<const D: usize, Size, CameraEffect> Serialize for GridRuntimeModel<D, Size, CameraEffect>
where
    Size: RuntimeGridSize<D>,
    CameraEffect: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let level_bundle = runtime_level_bundle_wire(&self.level_bundle);
        let mut wire = serializer.serialize_struct("GridRuntimeModel", 7)?;
        wire.serialize_field("game", &self.game)?;
        wire.serialize_field("inputs", &self.inputs)?;
        wire.serialize_field("levelBundle", &level_bundle)?;
        wire.serialize_field("goal", &self.goal)?;
        wire.serialize_field("ruleEffects", &self.rule_effects)?;
        wire.serialize_field("lifecycle", &self.lifecycle)?;
        wire.serialize_field("presentation", &self.presentation)?;
        wire.end()
    }
}

impl<'de, const D: usize, Size, CameraEffect> Deserialize<'de>
    for GridRuntimeModel<D, Size, CameraEffect>
where
    Size: RuntimeGridSize<D>,
    CameraEffect: DeserializeOwned,
{
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: Deserializer<'de>,
    {
        let wire =
            RuntimeGridModelWire::<D, Size::Snapshot, CameraEffect>::deserialize(deserializer)?;
        let level_bundle = runtime_level_bundle_from_wire(&wire.game, wire.level_bundle)
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            game: wire.game,
            inputs: wire.inputs,
            level_bundle,
            goal: wire.goal,
            rule_effects: wire.rule_effects,
            lifecycle: wire.lifecycle,
            presentation: wire.presentation,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeContractError {
    MissingRuntimeContract,
    InvalidJson(String),
    UnsupportedVersion { version: u16 },
    UnsupportedModelKind(String),
    InvalidGame(String),
    InvalidLevelBundle(String),
    InvalidPresentationEffects { owner: String },
}

impl std::fmt::Display for RuntimeContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRuntimeContract => {
                write!(f, "runtime fixture is missing runtimeContract")
            }
            Self::InvalidJson(error) => {
                write!(f, "invalid runtimeContract: {error}")
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported runtimeContract version: {version}")
            }
            Self::UnsupportedModelKind(kind) => {
                write!(f, "unsupported runtimeContract model kind: {kind}")
            }
            Self::InvalidGame(error) => {
                write!(f, "invalid runtimeContract model game: {error}")
            }
            Self::InvalidLevelBundle(error) => {
                write!(f, "invalid runtimeContract model level bundle: {error}")
            }
            Self::InvalidPresentationEffects { owner } => {
                write!(f, "invalid runtimeContract presentation effects at {owner}")
            }
        }
    }
}

impl std::error::Error for RuntimeContractError {}

pub fn runtime_contract_json<Model>(
    contract: &RuntimeContract<Model>,
) -> Result<String, RuntimeContractError>
where
    Model: RuntimeModel + Serialize,
{
    contract.validate()?;
    serde_json::to_string(contract)
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))
}

pub fn runtime_contract_from_fixture_value<Model>(
    value: &Value,
) -> Result<RuntimeContract<Model>, RuntimeContractError>
where
    Model: RuntimeModel + serde::de::DeserializeOwned,
{
    let contract_value = value
        .get("runtimeContract")
        .ok_or(RuntimeContractError::MissingRuntimeContract)?;
    let contract: RuntimeContract<Model> = serde_json::from_value(contract_value.clone())
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))?;
    contract.validate()?;
    Ok(contract)
}

pub fn runtime_contract_from_fixture_json<Model>(
    fixture_json: &str,
) -> Result<RuntimeContract<Model>, RuntimeContractError>
where
    Model: RuntimeModel + serde::de::DeserializeOwned,
{
    let value: Value = serde_json::from_str(fixture_json)
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))?;
    runtime_contract_from_fixture_value(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_core::{
        ConditionId, Delta3, GridConditionDef, GridConditionValueKind, GridExecutableProgram,
        GridGuard, GridLevel, GridLevelBundle, GridMatchCell, GridPattern, GridRule, GridRuleStep,
        GridState, LayerId, ObjectDef, RuleApplication, RuleId, Size3,
    };
    use serde_json::json;

    type Rule = GridRule<3>;

    const PLAYER: ObjectId = ObjectId(1);

    fn support_game() -> GridCompiledGame<3> {
        GridCompiledGame::<3>::new_with_condition_defs(
            1,
            vec![ObjectDef {
                id: PLAYER,
                layer_id: LayerId(0),
            }],
            vec![GridConditionDef::<3> {
                id: ConditionId(0),
                kind: GridConditionValueKind::<3>::ExistsObjects(vec![PLAYER]),
            }],
        )
    }

    fn support_level_bundle(game: &GridCompiledGame<3>) -> GridLevelBundle<3, Size3> {
        let mut state = GridState::empty_sized(Size3::new(1, 1, 1), 1, game.object_count())
            .expect("support state dimensions are valid");
        state
            .place_object_at(game, puzzle_core::GridCoord::new([0, 0, 0]), PLAYER)
            .expect("support object placement is valid");
        state
            .set_slot_mark_at(
                puzzle_core::GridCoord::new([0, 0, 0]),
                LayerId(0),
                MarkId(2),
                Some(7),
            )
            .expect("support slot mark is valid");
        state
            .set_cell_mark_at(puzzle_core::GridCoord::new([0, 0, 0]), MarkId(3), None)
            .expect("support cell mark is valid");
        state.mark_level_rule_fired(RuleId(9));
        GridLevelBundle::checked_new(
            game.clone(),
            vec![GridLevel::new(
                "one",
                state,
                puzzle_core::GridExecutableProgram::new(Vec::new()),
                None,
                None,
            )],
        )
        .expect("support level bundle is valid")
    }

    #[test]
    fn fixture_requires_runtime_contract_field() {
        let error =
            runtime_contract_from_fixture_value::<GridRuntimeModel<3, Size3, CameraEffect>>(
                &json!({}),
            )
            .unwrap_err();

        assert_eq!(error, RuntimeContractError::MissingRuntimeContract);
    }

    #[test]
    fn runtime_contract_round_trips_shared_guard_variants() {
        let rule = Rule {
            id: RuleId(7),
            guards: vec![GridGuard::<3>::ConditionNonZero(ConditionId(0))],
            application: RuleApplication::Once,
            pattern: GridPattern::<3>::new(vec![
                GridMatchCell::<3>::new(Delta3::ZERO).require(PLAYER),
            ]),
            writes: Vec::new(),
            effects: Vec::new(),
        };
        let game = support_game().clone_with_executable_program(GridExecutableProgram::new(vec![
            GridRuleStep::<3>::Rule(rule.clone()),
        ]));
        let model = GridRuntimeModel::checked_new(
            game.clone(),
            Vec::new(),
            support_level_bundle(&game),
            None,
            Vec::new(),
            RuntimeLifecycle::default(),
            GridPresentation::<CameraEffect> {
                local_frame: None,
                rule_camera_effects: vec![Vec::new()],
                on_level_start_camera_effects: Vec::new(),
            },
        )
        .expect("runtime model is valid");
        let contract = RuntimeContract::checked_new(model).expect("runtime contract is valid");
        let fixture = json!({
            "runtimeContract": serde_json::to_value(&contract).expect("contract serializes"),
        });
        assert!(fixture["runtimeContract"]["model"]["game"].is_object());
        assert!(
            fixture["runtimeContract"]["model"]["levelBundle"]
                .get("game")
                .is_none()
        );
        let initial_state =
            &fixture["runtimeContract"]["model"]["levelBundle"]["levels"][0]["initialState"];
        assert_eq!(initial_state["kind"], "3d");
        assert_eq!(initial_state["layerCount"], 1);
        assert_eq!(initial_state["slots"], json!([1]));
        assert_eq!(
            initial_state["slotMarks"],
            json!([[{"mark": 2, "value": 7}]])
        );
        assert_eq!(initial_state["cellMarks"], json!([[{"mark": 3}]]));
        assert!(initial_state.get("layer_count").is_none());
        assert!(initial_state.get("visible_variables").is_none());

        let decoded: RuntimeContract<GridRuntimeModel<3, Size3, CameraEffect>> =
            runtime_contract_from_fixture_value(&fixture).expect("runtime contract decodes");

        assert_eq!(decoded.model.game.rules()[0].guards, rule.guards);
        assert_eq!(
            decoded.model.level_bundle.levels[0].initial_state,
            contract.model.level_bundle.levels[0].initial_state
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
                from_object: None,
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
