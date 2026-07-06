use puzzle_grid3d::{
    ConditionValueKind3, Game3, LevelBundle3, LocalFrame, ObjectId, Pattern3, Rule3, WinCondition3,
};
pub use puzzle_scene::SceneEffect as LifecycleCommand3;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PUZZLE3_RUNTIME_CONTRACT_VERSION: u16 = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lifecycle3 {
    pub on_level_start: Vec<Rule3>,
    pub on_level_start_local_frame: Option<LocalFrame<ObjectId>>,
    pub on_level_clear: Vec<LifecycleCommand3>,
    pub on_last_level_clear: Option<Vec<LifecycleCommand3>>,
}

impl Lifecycle3 {
    pub fn new(on_level_start: Vec<Rule3>, on_level_clear: Vec<LifecycleCommand3>) -> Self {
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
pub struct Puzzle3RuntimeContract {
    pub version: u16,
    pub game: Game3,
    pub local_frame: Option<LocalFrame<ObjectId>>,
    pub rules: Vec<Rule3>,
    pub level_bundle: LevelBundle3,
    pub win_condition: Option<WinCondition3>,
    pub lifecycle: Lifecycle3,
}

impl Puzzle3RuntimeContract {
    pub fn checked_new(
        game: Game3,
        local_frame: Option<LocalFrame<ObjectId>>,
        rules: Vec<Rule3>,
        level_bundle: LevelBundle3,
        win_condition: Option<WinCondition3>,
        lifecycle: Lifecycle3,
    ) -> Result<Self, Puzzle3RuntimeContractError> {
        let contract = Self {
            version: PUZZLE3_RUNTIME_CONTRACT_VERSION,
            game,
            local_frame,
            rules,
            level_bundle,
            win_condition,
            lifecycle,
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), Puzzle3RuntimeContractError> {
        if self.version != PUZZLE3_RUNTIME_CONTRACT_VERSION {
            return Err(Puzzle3RuntimeContractError::UnsupportedVersion {
                version: self.version,
            });
        }
        self.game
            .validate()
            .map_err(|error| Puzzle3RuntimeContractError::InvalidGame(format!("{error:?}")))?;
        if self.level_bundle.game != self.game {
            return Err(Puzzle3RuntimeContractError::InvalidLevelBundle(
                "level bundle game does not match runtime contract game".to_string(),
            ));
        }
        self.level_bundle.validate().map_err(|error| {
            Puzzle3RuntimeContractError::InvalidLevelBundle(format!("{error:?}"))
        })?;
        validate_rules("rules", &self.rules)?;
        validate_rules("lifecycle.onLevelStart", &self.lifecycle.on_level_start)?;
        for condition in self.game.condition_defs() {
            validate_condition_kind(
                &format!("conditionDefs[{}]", condition.id.0),
                &condition.kind,
            )?;
        }
        if let Some(condition) = self.win_condition.as_ref() {
            validate_win_condition("winCondition", condition)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Puzzle3RuntimeContractError {
    MissingRuntimeContract,
    InvalidJson(String),
    UnsupportedVersion { version: u16 },
    InvalidGame(String),
    InvalidLevelBundle(String),
    InvalidPatternCache { owner: String },
}

impl std::fmt::Display for Puzzle3RuntimeContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRuntimeContract => {
                write!(f, "Puzzle3 fixture is missing runtimeContract")
            }
            Self::InvalidJson(error) => {
                write!(f, "invalid Puzzle3 runtimeContract: {error}")
            }
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported Puzzle3 runtimeContract version: {version}")
            }
            Self::InvalidGame(error) => {
                write!(f, "invalid Puzzle3 runtimeContract game: {error}")
            }
            Self::InvalidLevelBundle(error) => {
                write!(f, "invalid Puzzle3 runtimeContract level bundle: {error}")
            }
            Self::InvalidPatternCache { owner } => {
                write!(
                    f,
                    "invalid Puzzle3 runtimeContract pattern cache at {owner}"
                )
            }
        }
    }
}

impl std::error::Error for Puzzle3RuntimeContractError {}

pub fn puzzle3_runtime_contract_json(
    contract: &Puzzle3RuntimeContract,
) -> Result<String, Puzzle3RuntimeContractError> {
    contract.validate()?;
    serde_json::to_string(contract)
        .map_err(|error| Puzzle3RuntimeContractError::InvalidJson(error.to_string()))
}

pub fn puzzle3_runtime_contract_from_fixture_value(
    value: &Value,
) -> Result<Puzzle3RuntimeContract, Puzzle3RuntimeContractError> {
    let contract_value = value
        .get("runtimeContract")
        .ok_or(Puzzle3RuntimeContractError::MissingRuntimeContract)?;
    let contract: Puzzle3RuntimeContract = serde_json::from_value(contract_value.clone())
        .map_err(|error| Puzzle3RuntimeContractError::InvalidJson(error.to_string()))?;
    contract.validate()?;
    Ok(contract)
}

pub fn puzzle3_runtime_contract_from_fixture_json(
    fixture_json: &str,
) -> Result<Puzzle3RuntimeContract, Puzzle3RuntimeContractError> {
    let value: Value = serde_json::from_str(fixture_json)
        .map_err(|error| Puzzle3RuntimeContractError::InvalidJson(error.to_string()))?;
    puzzle3_runtime_contract_from_fixture_value(&value)
}

fn validate_rules(owner: &str, rules: &[Rule3]) -> Result<(), Puzzle3RuntimeContractError> {
    for rule in rules {
        validate_pattern(&format!("{owner}[{}].pattern", rule.id.0), &rule.pattern)?;
        for (index, guard) in rule.guards.iter().enumerate() {
            match guard {
                puzzle_grid3d::Guard3::InlineConditionValue { kind, .. }
                | puzzle_grid3d::Guard3::InlineConditionCompare { kind, .. } => {
                    validate_condition_kind(
                        &format!("{owner}[{}].guards[{index}]", rule.id.0),
                        kind,
                    )?;
                }
                puzzle_grid3d::Guard3::InlineConditionNonZero(kind) => {
                    validate_condition_kind(
                        &format!("{owner}[{}].guards[{index}]", rule.id.0),
                        kind,
                    )?;
                }
                puzzle_grid3d::Guard3::InputIs(_)
                | puzzle_grid3d::Guard3::GlobalEquals { .. }
                | puzzle_grid3d::Guard3::GlobalCompare { .. }
                | puzzle_grid3d::Guard3::ConditionEquals { .. }
                | puzzle_grid3d::Guard3::ConditionNonZero(_)
                | puzzle_grid3d::Guard3::ConditionCompare { .. } => {}
            }
        }
    }
    Ok(())
}

fn validate_condition_kind(
    owner: &str,
    kind: &ConditionValueKind3,
) -> Result<(), Puzzle3RuntimeContractError> {
    match kind {
        ConditionValueKind3::CountMatches(patterns)
        | ConditionValueKind3::ExistsMatches(patterns)
        | ConditionValueKind3::NoneMatches(patterns) => {
            for (index, pattern) in patterns.iter().enumerate() {
                validate_pattern(&format!("{owner}.patterns[{index}]"), pattern)?;
            }
        }
        ConditionValueKind3::CountInputMatches(patterns)
        | ConditionValueKind3::ExistsInputMatches(patterns)
        | ConditionValueKind3::NoneInputMatches(patterns) => {
            for (index, (_, pattern)) in patterns.iter().enumerate() {
                validate_pattern(&format!("{owner}.patterns[{index}].pattern"), pattern)?;
            }
        }
        ConditionValueKind3::CountObjects(_)
        | ConditionValueKind3::ExistsObjects(_)
        | ConditionValueKind3::NoneObjects(_) => {}
    }
    Ok(())
}

fn validate_win_condition(
    owner: &str,
    condition: &WinCondition3,
) -> Result<(), Puzzle3RuntimeContractError> {
    match condition {
        WinCondition3::All(conditions) | WinCondition3::Any(conditions) => {
            for (index, condition) in conditions.iter().enumerate() {
                validate_win_condition(&format!("{owner}.conditions[{index}]"), condition)?;
            }
        }
        WinCondition3::SomePattern(pattern) | WinCondition3::NoPattern(pattern) => {
            validate_pattern(&format!("{owner}.pattern"), pattern)?;
        }
        WinCondition3::AllObjectsCoveredByPattern { cover_pattern, .. } => {
            validate_pattern(&format!("{owner}.coverPattern"), cover_pattern)?;
        }
        WinCondition3::SomeObject(_) | WinCondition3::NoObject(_) => {}
    }
    Ok(())
}

fn validate_pattern(owner: &str, pattern: &Pattern3) -> Result<(), Puzzle3RuntimeContractError> {
    let cells = pattern
        .components
        .iter()
        .flat_map(|component| component.cells.iter().cloned())
        .collect::<Vec<_>>();
    if cells != pattern.cells {
        return Err(Puzzle3RuntimeContractError::InvalidPatternCache {
            owner: owner.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_grid3d::{
        ConditionDef3, ConditionId3, ConditionValueKind3, Coord3, Guard3, InputDef3, InputId3,
        LayerId, Level3, LevelCell3, LevelEntry3, MatchCell3, ObjectDef3, Offset3,
        RuleApplication3, RuleId3, Size3,
    };
    use serde_json::json;

    const PLAYER: ObjectId = ObjectId(1);

    fn support_game() -> Game3 {
        Game3::new_with_condition_defs(
            1,
            vec![ObjectDef3 {
                id: PLAYER,
                layer_id: LayerId(0),
            }],
            vec![InputDef3::action(InputId3(0), "wait")],
            vec![ConditionDef3 {
                id: ConditionId3(0),
                kind: ConditionValueKind3::ExistsObjects(vec![PLAYER]),
            }],
        )
    }

    fn support_level_bundle(game: &Game3) -> LevelBundle3 {
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
        let error = puzzle3_runtime_contract_from_fixture_value(&json!({})).unwrap_err();

        assert_eq!(error, Puzzle3RuntimeContractError::MissingRuntimeContract);
    }

    #[test]
    fn runtime_contract_round_trips_shared_guard_variants() {
        let game = support_game();
        let rule = Rule3 {
            id: RuleId3(7),
            guards: vec![Guard3::ConditionNonZero(ConditionId3(0))],
            application: RuleApplication3::Once,
            pattern: Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(PLAYER)]),
            writes: Vec::new(),
            effects: Vec::new(),
        };
        let contract = Puzzle3RuntimeContract::checked_new(
            game.clone(),
            None,
            vec![rule.clone()],
            support_level_bundle(&game),
            None,
            Lifecycle3::default(),
        )
        .expect("runtime contract is valid");
        let fixture = json!({
            "runtimeContract": serde_json::to_value(&contract).expect("contract serializes"),
        });

        let decoded = puzzle3_runtime_contract_from_fixture_value(&fixture)
            .expect("runtime contract decodes");

        assert_eq!(decoded.rules[0].guards, rule.guards);
    }
}
