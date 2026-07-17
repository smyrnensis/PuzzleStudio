use puzzle_core::{
    GridCompiledGame, GridExecutableProgram, GridGoalCondition, GridInput, GridLevelBundle,
    GridSize, LocalFrame, ObjectId,
};
pub use puzzle_scene::SceneEffect as LifecycleCommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUNTIME_CONTRACT_VERSION: u16 = 6;

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
    pub variables: Vec<i64>,
    pub level_fired_rules: Vec<u16>,
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
pub struct RuntimeTransitionProgramOutcome {
    pub state: RuntimeStateSnapshot,
    pub cancelled: bool,
    pub completed: bool,
    pub commands: Vec<RuntimeTransitionCommand>,
    pub effects: Vec<RuntimeEffect>,
    pub fired_rules: Vec<u16>,
    pub patches: Vec<Vec<RuntimePatchOp>>,
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
    pub fired_rules: Vec<u16>,
    pub patches: Vec<Vec<RuntimePatchOp>>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridRuntimeModel<const D: usize, Size: GridSize<D>, CameraEffect> {
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

impl<const D: usize, Size: GridSize<D>, PresentationEffect>
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

impl<const D: usize, Size: GridSize<D>, PresentationEffect> RuntimeModel
    for GridRuntimeModel<D, Size, PresentationEffect>
{
    fn validate(&self) -> Result<(), RuntimeContractError> {
        validate_grid_runtime_model(self)
    }
}

fn validate_grid_runtime_model<const D: usize, Size: GridSize<D>, PresentationEffect>(
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

        let decoded: RuntimeContract<GridRuntimeModel<3, Size3, CameraEffect>> =
            runtime_contract_from_fixture_value(&fixture).expect("runtime contract decodes");

        assert_eq!(decoded.model.game.rules()[0].guards, rule.guards);
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
                variables: Vec::new(),
                level_fired_rules: Vec::new(),
            }),
            cancelled: false,
            completed: true,
            commands: vec![RuntimeTransitionCommand::Win],
            effects: vec![RuntimeEffect::Win],
            fired_rules: vec![3],
            patches: vec![vec![RuntimePatchOp::Remove {
                position: RuntimeCoord {
                    x: 0,
                    y: 0,
                    z: None,
                },
                object_id: 1,
            }]],
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
                "firedRules",
                "patches",
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
            fired_rules: Vec::new(),
            patches: Vec::new(),
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
                "firedRules",
                "levelFiredRules",
                "patches",
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
