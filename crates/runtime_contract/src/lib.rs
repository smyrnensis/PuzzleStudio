use puzzle_grid3d::{CompiledGame3, LevelBundle3, LocalFrame, ObjectId, WinCondition3};
pub use puzzle_scene::SceneEffect as LifecycleCommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUNTIME_CONTRACT_VERSION: u16 = 4;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTransitionProgramOutcome {
    pub state: RuntimeStateSnapshot,
    pub cancelled: bool,
    pub completed: bool,
    pub commands: Vec<RuntimeTransitionCommand>,
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
pub struct RuntimeLifecycle<Rule, Frame> {
    pub on_level_start: Vec<Rule>,
    pub on_level_start_local_frame: Option<Frame>,
    pub on_level_clear: Vec<LifecycleCommand>,
    pub on_last_level_clear: Option<Vec<LifecycleCommand>>,
}

impl<Rule, Frame> Default for RuntimeLifecycle<Rule, Frame> {
    fn default() -> Self {
        Self {
            on_level_start: Vec::new(),
            on_level_start_local_frame: None,
            on_level_clear: Vec::new(),
            on_last_level_clear: None,
        }
    }
}

impl<Rule, Frame> RuntimeLifecycle<Rule, Frame> {
    pub fn new(on_level_start: Vec<Rule>, on_level_clear: Vec<LifecycleCommand>) -> Self {
        Self {
            on_level_start,
            on_level_start_local_frame: None,
            on_level_clear,
            on_last_level_clear: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContract {
    pub version: u16,
    pub model: RuntimeModelContract,
}

impl RuntimeContract {
    pub fn checked_new(model: RuntimeModelContract) -> Result<Self, RuntimeContractError> {
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

    pub fn into_puzzle3_model(self) -> Result<Puzzle3RuntimeModel, RuntimeContractError> {
        match self.model {
            RuntimeModelContract::Puzzle3(model) => Ok(model),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeModelContract {
    #[serde(rename = "puzzle3")]
    Puzzle3(Puzzle3RuntimeModel),
}

impl RuntimeModelContract {
    fn validate(&self) -> Result<(), RuntimeContractError> {
        match self {
            Self::Puzzle3(model) => model.validate(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Puzzle3RuntimeModel {
    pub game: CompiledGame3,
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rule_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    pub level_bundle: LevelBundle3,
    pub win_condition: Option<WinCondition3>,
    pub lifecycle: RuntimeLifecycle<puzzle_grid3d::RuleStep3, LocalFrame<ObjectId>>,
    pub on_level_start_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Puzzle3CameraEffect {
    SetYaw(i16),
    SetPitch(i16),
    SetRoll(i16),
    SetZoom(u16),
    Reset,
}

impl Puzzle3RuntimeModel {
    pub fn checked_new(
        game: CompiledGame3,
        local_frame: Option<LocalFrame<ObjectId>>,
        rule_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
        level_bundle: LevelBundle3,
        win_condition: Option<WinCondition3>,
        lifecycle: RuntimeLifecycle<puzzle_grid3d::RuleStep3, LocalFrame<ObjectId>>,
        on_level_start_camera_effects: Vec<Vec<Puzzle3CameraEffect>>,
    ) -> Result<Self, RuntimeContractError> {
        let model = Self {
            game,
            local_frame,
            rule_camera_effects,
            level_bundle,
            win_condition,
            lifecycle,
            on_level_start_camera_effects,
        };
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> Result<(), RuntimeContractError> {
        self.game
            .validate()
            .map_err(|error| RuntimeContractError::InvalidGame(format!("{error:?}")))?;
        if self.level_bundle.game != self.game {
            return Err(RuntimeContractError::InvalidLevelBundle(
                "level bundle game does not match runtime contract game".to_string(),
            ));
        }
        self.level_bundle
            .validate()
            .map_err(|error| RuntimeContractError::InvalidLevelBundle(format!("{error:?}")))?;
        let rule_count = count_program_rules(self.game.program());
        let level_start_rule_count = count_program_rules(&self.lifecycle.on_level_start);
        if self.rule_camera_effects.len() != rule_count {
            return Err(RuntimeContractError::InvalidPresentationEffects {
                owner: "ruleCameraEffects".to_string(),
            });
        }
        if self.on_level_start_camera_effects.len() != level_start_rule_count {
            return Err(RuntimeContractError::InvalidPresentationEffects {
                owner: "onLevelStartCameraEffects".to_string(),
            });
        }
        Ok(())
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

pub fn runtime_contract_json(contract: &RuntimeContract) -> Result<String, RuntimeContractError> {
    contract.validate()?;
    serde_json::to_string(contract)
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))
}

pub fn runtime_contract_from_fixture_value(
    value: &Value,
) -> Result<RuntimeContract, RuntimeContractError> {
    let contract_value = value
        .get("runtimeContract")
        .ok_or(RuntimeContractError::MissingRuntimeContract)?;
    let contract: RuntimeContract = serde_json::from_value(contract_value.clone())
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))?;
    contract.validate()?;
    Ok(contract)
}

pub fn runtime_contract_from_fixture_json(
    fixture_json: &str,
) -> Result<RuntimeContract, RuntimeContractError> {
    let value: Value = serde_json::from_str(fixture_json)
        .map_err(|error| RuntimeContractError::InvalidJson(error.to_string()))?;
    runtime_contract_from_fixture_value(&value)
}

pub fn puzzle3_runtime_model_from_fixture_value(
    value: &Value,
) -> Result<Puzzle3RuntimeModel, RuntimeContractError> {
    runtime_contract_from_fixture_value(value)?.into_puzzle3_model()
}

pub fn puzzle3_runtime_model_from_fixture_json(
    fixture_json: &str,
) -> Result<Puzzle3RuntimeModel, RuntimeContractError> {
    runtime_contract_from_fixture_json(fixture_json)?.into_puzzle3_model()
}

fn count_program_rules(steps: &[puzzle_grid3d::RuleStep3]) -> usize {
    let mut rule_count = 0;
    for step in steps {
        match step {
            puzzle_grid3d::RuleStep3::Rule(rule) => {
                let _ = rule;
                rule_count += 1;
            }
            puzzle_grid3d::RuleStep3::ConditionalBlock { condition, steps } => {
                let _ = condition;
                rule_count += count_program_rules(steps);
            }
            puzzle_grid3d::RuleStep3::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                let _ = condition;
                rule_count += count_program_rules(then_steps);
                rule_count += count_program_rules(else_steps);
            }
            puzzle_grid3d::RuleStep3::Block {
                stop_condition,
                steps,
                ..
            } => {
                let _ = stop_condition;
                rule_count += count_program_rules(steps);
            }
            puzzle_grid3d::RuleStep3::AfterTriggered { steps, then_steps } => {
                rule_count += count_program_rules(steps);
                rule_count += count_program_rules(then_steps);
            }
            puzzle_grid3d::RuleStep3::LocalFrame { steps, .. } => {
                rule_count += count_program_rules(steps);
            }
        }
    }
    rule_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_grid3d::{
        ConditionDef3, ConditionId3, ConditionValueKind3, Coord3, Delta3, Guard3, InputDef3,
        InputId, LayerId, Level3, LevelCell3, LevelEntry3, MatchCell3, ObjectDef3, Pattern3, Rule3,
        RuleApplication3, RuleId3, Size3,
    };
    use serde_json::json;

    const PLAYER: ObjectId = ObjectId(1);

    fn support_game() -> CompiledGame3 {
        CompiledGame3::new_with_condition_defs(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: LayerId(0),
            }],
            vec![ConditionDef3 {
                id: ConditionId3(0),
                kind: ConditionValueKind3::ExistsObjects(vec![PLAYER]),
            }],
        )
    }

    fn support_level_bundle(game: &CompiledGame3) -> LevelBundle3 {
        LevelBundle3::checked_new(
            game.clone(),
            vec![LevelEntry3::new(
                "one",
                Level3::new(
                    Size3::new(1, 1, 1),
                    vec![LevelCell3::new(Coord3::new(0, 0, 0), vec![PLAYER])],
                ),
            )],
        )
        .expect("support level bundle is valid")
    }

    #[test]
    fn fixture_requires_runtime_contract_field() {
        let error = runtime_contract_from_fixture_value(&json!({})).unwrap_err();

        assert_eq!(error, RuntimeContractError::MissingRuntimeContract);
    }

    #[test]
    fn runtime_contract_round_trips_shared_guard_variants() {
        let rule = Rule3 {
            id: RuleId3(7),
            guards: vec![Guard3::ConditionNonZero(ConditionId3(0))],
            application: RuleApplication3::Once,
            pattern: Pattern3::new(vec![MatchCell3::new(Delta3::ZERO).require(PLAYER)]),
            writes: Vec::new(),
            effects: Vec::new(),
        };
        let game =
            support_game().clone_with_program(vec![puzzle_grid3d::RuleStep3::Rule(rule.clone())]);
        let model = Puzzle3RuntimeModel::checked_new(
            game.clone(),
            None,
            vec![Vec::new()],
            support_level_bundle(&game),
            None,
            RuntimeLifecycle::default(),
            Vec::new(),
        )
        .expect("runtime model is valid");
        let contract = RuntimeContract::checked_new(RuntimeModelContract::Puzzle3(model))
            .expect("runtime contract is valid");
        let fixture = json!({
            "runtimeContract": serde_json::to_value(&contract).expect("contract serializes"),
        });

        let decoded =
            puzzle3_runtime_model_from_fixture_value(&fixture).expect("runtime contract decodes");

        assert_eq!(decoded.game.rules()[0].guards, rule.guards);
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
