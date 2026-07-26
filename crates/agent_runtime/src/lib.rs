use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::fs;

use puzzle_solver_runtime::investigation::InvestigationService;
pub use puzzle_solver_runtime::investigation::{
    DeriveStateRequest, InvestigationCommand as AgentCommand, InvestigationError as AgentError,
    ObservationMode, ObservationRequest, SearchSessionAllowance, SearchSessionLimits,
    SemanticGoalArtifact, SemanticGoalSearchAlgorithm, SemanticGoalSearchBudget,
    SemanticLegendMeaning, SemanticObjectPositions, SemanticStateArtifact, SemanticStateAssertion,
    SemanticVariableValue, StartLevelFromStateRequest,
};

pub const AGENT_PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequest {
    pub version: u32,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(flatten)]
    pub command: AgentCommand,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentResponse {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AgentError>,
}

#[derive(Default)]
pub struct AgentServer {
    inner: InvestigationService,
}

impl AgentServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_line(&mut self, line: &str) -> String {
        let parsed = serde_json::from_str::<AgentRequest>(line);
        let response = match parsed {
            Ok(request) => self.handle(request),
            Err(error) => failure(
                request_id_from_invalid_json(line),
                agent_error(
                    "invalid_request",
                    format!("agent request JSON is invalid: {error}"),
                ),
            ),
        };
        serde_json::to_string(&response).expect("agent response must serialize")
    }

    pub fn handle(&mut self, request: AgentRequest) -> AgentResponse {
        let request_id = request.request_id;
        if request.version != AGENT_PROTOCOL_VERSION {
            return failure(
                request_id,
                agent_error(
                    "contract_version_mismatch",
                    format!(
                        "unsupported agent protocol version {}; expected {}",
                        request.version, AGENT_PROTOCOL_VERSION
                    ),
                ),
            );
        }
        match self.inner.dispatch(request.command) {
            Ok(data) => success(request_id, data),
            Err(error) => failure(request_id, error),
        }
    }
}

fn agent_error(code: impl Into<String>, message: impl Into<String>) -> AgentError {
    AgentError {
        code: code.into(),
        message: message.into(),
        details: None,
    }
}

fn request_id_from_invalid_json(line: &str) -> Option<String> {
    serde_json::from_str::<Value>(line).ok().and_then(|value| {
        value
            .get("requestId")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn success(request_id: Option<String>, data: Value) -> AgentResponse {
    AgentResponse {
        version: AGENT_PROTOCOL_VERSION,
        request_id,
        ok: true,
        data: Some(data),
        error: None,
    }
}

fn failure(request_id: Option<String>, error: AgentError) -> AgentResponse {
    AgentResponse {
        version: AGENT_PROTOCOL_VERSION,
        request_id,
        ok: false,
        data: None,
        error: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    const SOURCE: &str = r#"
const title = agent_session

puzzle board {
layers {
floor = Goal Trail
actor = Player
}
keys {
d ArrowRight -> right
}
rules {
input right [ Player ] -> [ Player ]
}
}

levels tiny of board {
legend {
. = empty
P = Player
G = Goal
}
level "start" {
rules before {
input right [ Player | no actor ] -> [ Trail | Player ]
}
P.G
}
}
"#;

    const VARIABLE_SOURCE: &str = r#"
const title = agent_variable_state

puzzle board {
var count = 0
persistent var saved = 0

layers {
actor = Player
}
keys {
d ArrowRight -> right
}
rules {
input right [ Player ] -> [ Player ]
}
}

levels tiny of board {
legend {
P = Player
}
level "start" {
P
}
}
"#;

    const LEVEL_START_SOURCE: &str = r#"
const title = agent_level_start_state

puzzle board {
persistent var starts = 0
var count = 0

layers {
floor = Started
actor = Player
}
keys {
d ArrowRight -> right
}
on_level_start {
starts += 1
count += 1
once [ Player no Started ] -> [ Player Started ]
}
rules {
input right [ Player | no actor ] -> [ | Player ]
}
}

levels tiny of board {
legend {
. = empty
P = Player
}
level "start" {
P..
}
}
"#;

    const FAILING_LEVEL_START_SOURCE: &str = r#"
const title = failing_agent_level_start

puzzle board {
var count = 1
layers { actor = Player }
empty .
on_level_start {
count /= 0
}
rules {}
levels {
legend { P = Player }
level "start" { P }
}
}
"#;

    const DERIVED_CHECKPOINT_SOURCE: &str = r#"
const title = derived_checkpoint_state

puzzle board {
layers { actor = Player }
empty .
input save
rules {
if input == right {
once right [ Player | no Player ] -> [ | Player ]
}
if input == save {
checkpoint
}
}
levels {
legend { P = Player }
level "start" { P.. }
}
}
"#;

    fn write_source() -> std::path::PathBuf {
        write_source_text(SOURCE)
    }

    fn write_source_text(source: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "puzzle-agent-runtime-{}-{}.puzzle",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("game.puzzle");
        fs::write(&path, source).unwrap();
        path
    }

    fn remove_source(path: std::path::PathBuf) {
        let directory = path.parent().unwrap().to_path_buf();
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    fn request(server: &mut AgentServer, value: Value) -> Value {
        serde_json::from_str(&server.handle_line(&value.to_string())).unwrap()
    }

    #[test]
    fn protocol_v2_rejects_v1_requests() {
        let mut server = AgentServer::new();
        let response: Value = serde_json::from_str(
            &server.handle_line(r#"{"version":1,"op":"manifest","sessionId":"missing"}"#),
        )
        .unwrap();

        assert_eq!(response["version"], AGENT_PROTOCOL_VERSION);
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "contract_version_mismatch");
    }

    #[test]
    fn one_shot_search_requires_explicit_stored_node_limit() {
        let mut server = AgentServer::new();
        let response: Value = serde_json::from_str(&server.handle_line(
            r#"{"version":2,"op":"solve_semantic_goal","sessionId":"session-1","goalId":"goal-1","fromStateId":"state-1","algorithm":"best_first","budget":{"maxDepth":1,"maxNodes":10,"maxMillis":1000}}"#,
        ))
        .unwrap();

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid_request");
        assert!(
            response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("maxStoredNodes"))
        );
    }

    #[test]
    fn compiled_session_runs_sequences_and_inspects_selected_points() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "requestId": "c", "op": "compile", "path": path }),
        );
        assert_eq!(compiled["ok"], true);
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "right"],
                "observation": { "mode": "events" }
            }),
        );
        assert_eq!(run["ok"], true, "{run}");
        assert_eq!(run["data"]["result"], "incomplete");
        assert_eq!(run["data"]["executedInputs"], 2);
        assert_eq!(run["data"]["observations"].as_array().unwrap().len(), 0);

        let repeated = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "right"]
            }),
        );
        assert_eq!(repeated["ok"], true, "{repeated}");
        assert_eq!(repeated["data"]["events"], Value::Null);
        assert_eq!(
            repeated["data"]["terminalHash"],
            run["data"]["terminalHash"]
        );

        let compared = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "compare_states",
                "sessionId": session,
                "leftStateId": run["data"]["terminalStateId"],
                "rightStateId": repeated["data"]["terminalStateId"]
            }),
        );
        assert_eq!(compared["data"]["same"], true, "{compared}");

        let inspected = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_run",
                "sessionId": session,
                "runId": run["data"]["runId"],
                "at": [0, 2],
                "includeTrace": true
            }),
        );
        assert_eq!(inspected["ok"], true, "{inspected}");
        assert_eq!(inspected["data"]["points"].as_array().unwrap().len(), 2);
        remove_source(path);
    }

    #[test]
    fn selected_level_run_is_independent_of_intro_presentation() {
        let path = write_source_text(
            r#"
const title = agent_selected_level
puzzle board {
layers { actor = Player }
empty .
keys { d ArrowRight -> right }
rules {
input right [ Player | no actor ] -> [ | Player ]
}
levels {
legend {
. = empty
P = Player
}
level "intro" {
message "intro"
P.
}
level "target" {
message "target"
P.
}
}
}
"#,
        );
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path, "model": "board" }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let target = compiled["data"]["initialStates"][1]["stateId"]
            .as_str()
            .unwrap();

        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": target,
                "inputs": ["right"]
            }),
        );
        assert_eq!(run["ok"], true, "{run}");
        assert_eq!(run["data"]["executedInputs"], 1, "{run}");
        remove_source(path);
    }

    #[test]
    fn locked_stuck_room_sequence_reaches_win_in_five_inputs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../games/TPGJ6/locked.puzzle");
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "compile",
                "path": path,
                "model": "main"
            }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "right", "left", "left", "left"]
            }),
        );

        assert_eq!(run["ok"], true, "{run}");
        assert_eq!(run["data"]["result"], "solved", "{run}");
        assert_eq!(run["data"]["executedInputs"], 5);
    }

    #[test]
    fn semantic_search_replays_one_again_input_through_play() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../play/tests/fixtures/again_atomic.puzzle");
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "compile",
                "path": path,
                "model": "board"
            }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let exported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "export_semantic_state",
                "sessionId": session,
                "stateId": initial
            }),
        );
        let mut goal = exported["data"].clone();
        goal["kind"] = json!("puzzle2d-semantic-goal");
        goal.as_object_mut().unwrap().remove("variables");
        goal["legend"]["D"] = json!({ "kind": "exact", "objects": ["Done"] });
        goal["lines"] = json!(["D"]);
        let imported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");
        let goal_id = imported["data"]["goalId"].as_str().unwrap();

        let solved = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": goal_id,
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 1, "maxStoredNodes": 10, "maxMillis": 1000 }
            }),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
        assert_eq!(solved["data"]["solutionDepth"], 1);
        assert_eq!(solved["data"]["inputs"], json!(["right"]));
        assert_eq!(solved["data"]["result"], "semantic_goal_reached");
    }

    #[test]
    fn semantic_search_matches_completion_observation_before_next_level() {
        let path = write_source_text(
            r#"
const title = semantic_completion_observation
puzzle board {
layers {
floor = Goal
actor = Player Box Wall
}
groups { solid = Player Box Wall }
keys { d ArrowRight -> right }
win_conditions {
some Goal
all Goal on Box
}
on_level_clear { next_level }
rules {
once input directions [ Player | Box | no solid ] -> [ | Player | Box ]
once input directions [ Player | no solid ] -> [ | Player ]
}
levels {
legend {
. = empty
G = Goal
P = Player
B = Box
# = Wall
}
level "first" {
#####
#PBG#
#####
}
level "second" {
#####
#P.G#
#####
}
}
}
"#,
        );
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path, "model": "board" }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let exported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "export_semantic_state",
                "sessionId": session,
                "stateId": initial
            }),
        );
        let mut goal = exported["data"].clone();
        goal["kind"] = json!("puzzle2d-semantic-goal");
        goal.as_object_mut().unwrap().remove("variables");
        goal["legend"] = json!({
            "?": { "kind": "unknown" },
            "X": { "kind": "contains", "objects": ["Goal", "Box"] }
        });
        goal["lines"] = json!(["?????", "???X?", "?????"]);
        let imported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");

        let solved = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": imported["data"]["goalId"],
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 1, "maxStoredNodes": 10, "maxMillis": 1000 }
            }),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
        assert_eq!(solved["data"]["result"], "semantic_goal_reached");
        assert_eq!(solved["data"]["inputs"], json!(["right"]));
        let terminal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": solved["data"]["terminalStateId"]
            }),
        );
        assert_eq!(terminal["data"]["levelIndex"], 1, "{terminal}");
        let continued = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": solved["data"]["terminalStateId"],
                "inputs": ["right"]
            }),
        );
        assert_eq!(continued["ok"], true, "{continued}");
        remove_source(path);
    }

    #[test]
    fn invalid_input_does_not_create_a_partial_run() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let failed = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "missing", "right"]
            }),
        );
        assert_eq!(failed["error"]["code"], "unknown_input");
        let manifest = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "manifest", "sessionId": session }),
        );
        assert_eq!(manifest["ok"], true);
        remove_source(path);
    }

    #[test]
    fn semantic_state_round_trips_and_imports_an_ai_edited_hypothesis() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let exported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "export_semantic_state",
                "sessionId": session,
                "stateId": initial
            }),
        );
        assert_eq!(exported["ok"], true, "{exported}");
        assert_eq!(exported["data"]["kind"], "puzzle2d-semantic-state");

        let mut stale = exported["data"].clone();
        stale["baseStateHash"] = Value::String("0000000000000000".to_string());
        let rejected = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_state",
                "sessionId": session,
                "artifact": stale
            }),
        );
        assert_eq!(rejected["error"]["code"], "semantic_state_base_mismatch");

        let mut state_with_unknown = exported["data"].clone();
        state_with_unknown["legend"]["?"] = json!({ "kind": "unknown" });
        let rejected_unknown = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_state",
                "sessionId": session,
                "artifact": state_with_unknown
            }),
        );
        assert_eq!(
            rejected_unknown["error"]["code"],
            "semantic_state_unknown_reserved"
        );

        let mut state_with_predicate = exported["data"].clone();
        state_with_predicate["legend"]["C"] = json!({ "kind": "contains", "objects": ["Player"] });
        let rejected_predicate = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_state",
                "sessionId": session,
                "artifact": state_with_predicate
            }),
        );
        assert_eq!(
            rejected_predicate["error"]["code"],
            "semantic_state_predicate_not_allowed"
        );

        let round_trip = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_state",
                "sessionId": session,
                "artifact": exported["data"].clone()
            }),
        );
        assert_eq!(round_trip["ok"], true, "{round_trip}");
        assert_eq!(round_trip["data"]["provenance"]["kind"], "hypothetical");
        assert!(
            round_trip["data"]["diff"]["objects"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        let player_char = exported["data"]["legend"]
            .as_object()
            .unwrap()
            .iter()
            .find_map(|(ch, meaning)| {
                meaning["objects"]
                    .as_array()
                    .is_some_and(|objects| objects.iter().any(|name| name == "Player"))
                    .then_some(ch.clone())
            })
            .expect("exported legend must name Player");
        let empty = exported["data"]["empty"].as_str().unwrap();
        let mut edited = exported["data"].clone();
        let line = edited["lines"][0].as_str().unwrap().to_string();
        assert_eq!(line.chars().next().unwrap().to_string(), player_char);
        edited["lines"][0] = Value::String(format!("{empty}{player_char}G"));
        let imported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_state",
                "sessionId": session,
                "artifact": edited
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");
        let changes = imported["data"]["diff"]["objects"].as_array().unwrap();
        let player_change = changes
            .iter()
            .find(|change| change["object"] == "Player")
            .expect("Player diff must be reported");
        assert_eq!(player_change["removed"], json!([[0, 0]]));
        assert_eq!(player_change["added"], json!([[1, 0]]));

        let hypothetical = imported["data"]["stateId"].as_str().unwrap();
        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": hypothetical,
                "inputs": ["right"]
            }),
        );
        assert_eq!(run["ok"], true, "{run}");

        let mut goal_artifact = exported["data"].clone();
        goal_artifact["kind"] = Value::String("puzzle2d-semantic-goal".to_string());
        goal_artifact.as_object_mut().unwrap().remove("variables");
        goal_artifact["legend"]["?"] = json!({ "kind": "unknown" });
        goal_artifact["lines"] = json!([format!("?{player_char}?")]);
        let goal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal_artifact
            }),
        );
        assert_eq!(goal["ok"], true, "{goal}");
        assert_eq!(goal["data"]["unknownCells"], 2);
        assert_eq!(goal["data"]["bindingCount"], 0);
        let goal_id = goal["data"]["goalId"].as_str().unwrap();

        let initial_evaluation = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "evaluate_semantic_goal",
                "sessionId": session,
                "goalId": goal_id,
                "stateId": initial
            }),
        );
        assert_eq!(initial_evaluation["data"]["matches"], false);
        let hypothetical_evaluation = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "evaluate_semantic_goal",
                "sessionId": session,
                "goalId": goal_id,
                "stateId": hypothetical
            }),
        );
        assert_eq!(hypothetical_evaluation["data"]["matches"], true);
        assert_eq!(hypothetical_evaluation["data"]["unknownCells"], 2);
        assert_eq!(hypothetical_evaluation["data"]["bindingCount"], 0);

        let mut excludes_artifact = exported["data"].clone();
        excludes_artifact["kind"] = Value::String("puzzle2d-semantic-goal".to_string());
        excludes_artifact
            .as_object_mut()
            .unwrap()
            .remove("variables");
        excludes_artifact["legend"]["?"] = json!({ "kind": "unknown" });
        excludes_artifact["legend"]["E"] = json!({ "kind": "excludes", "objects": ["Player"] });
        excludes_artifact["lines"] = json!(["?E?"]);
        let excludes_goal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": excludes_artifact
            }),
        );
        assert_eq!(excludes_goal["ok"], true, "{excludes_goal}");
        let excludes_goal_id = excludes_goal["data"]["goalId"].as_str().unwrap();
        let excludes_initial = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "evaluate_semantic_goal",
                "sessionId": session,
                "goalId": excludes_goal_id,
                "stateId": initial
            }),
        );
        assert_eq!(excludes_initial["data"]["matches"], true);
        let excludes_hypothetical = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "evaluate_semantic_goal",
                "sessionId": session,
                "goalId": excludes_goal_id,
                "stateId": hypothetical
            }),
        );
        assert_eq!(excludes_hypothetical["data"]["matches"], false);
        assert_eq!(
            excludes_hypothetical["data"]["mismatches"][0]["predicate"],
            "excludes"
        );

        let mut search_goal_artifact = exported["data"].clone();
        search_goal_artifact["kind"] = Value::String("puzzle2d-semantic-goal".to_string());
        search_goal_artifact
            .as_object_mut()
            .unwrap()
            .remove("variables");
        search_goal_artifact["legend"]["?"] = json!({ "kind": "unknown" });
        search_goal_artifact["legend"]["C"] = json!({ "kind": "contains", "objects": ["Player"] });
        search_goal_artifact["lines"] = json!(["??C"]);
        let search_goal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": search_goal_artifact
            }),
        );
        assert_eq!(search_goal["ok"], true, "{search_goal}");
        let search_goal_id = search_goal["data"]["goalId"].as_str().unwrap();

        for algorithm in ["bfs", "best_first"] {
            let solved = request(
                &mut server,
                json!({
                    "version": AGENT_PROTOCOL_VERSION,
                    "op": "solve_semantic_goal",
                    "sessionId": session,
                    "goalId": search_goal_id,
                    "fromStateId": initial,
                    "algorithm": algorithm,
                    "budget": {
                        "maxDepth": 4,
                        "maxStoredNodes": 100,
                        "maxMillis": 1000
                    }
                }),
            );
            assert_eq!(solved["ok"], true, "{solved}");
            assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
            assert_eq!(solved["data"]["result"], "semantic_goal_reached");
            assert_eq!(solved["data"]["solutionDepth"], 1);
            assert_eq!(solved["data"]["inputs"], json!(["right"]));
            assert!(solved["data"]["runId"].is_string());
            let terminal = solved["data"]["terminalStateId"].as_str().unwrap();
            let evaluated = request(
                &mut server,
                json!({
                    "version": AGENT_PROTOCOL_VERSION,
                    "op": "evaluate_semantic_goal",
                    "sessionId": session,
                    "goalId": search_goal_id,
                    "stateId": terminal
                }),
            );
            assert_eq!(evaluated["data"]["matches"], true, "{evaluated}");
        }

        let resumable = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "create_search",
                "sessionId": session,
                "goalId": goal_id,
                "fromStateId": initial,
                "algorithm": "best_first",
                "limits": { "maxDepth": 4, "maxStoredNodes": 100 }
            }),
        );
        assert_eq!(resumable["ok"], true, "{resumable}");
        assert_eq!(resumable["data"]["status"], "ready");
        let resumable_id = resumable["data"]["searchId"].as_str().unwrap();

        let first_advance = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "advance_search",
                "sessionId": session,
                "searchId": resumable_id,
                "allowance": { "maxExpandedNodes": 1, "maxMillis": 1000 }
            }),
        );
        assert_eq!(first_advance["ok"], true, "{first_advance}");
        assert_eq!(first_advance["data"]["status"], "paused");
        assert_eq!(first_advance["data"]["stats"]["expanded"], 1);

        let inspected_search = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_search",
                "sessionId": session,
                "searchId": resumable_id,
                "candidateLimit": 10
            }),
        );
        assert_eq!(inspected_search["ok"], true, "{inspected_search}");
        assert_eq!(inspected_search["data"]["status"], "paused");
        assert_eq!(
            inspected_search["data"]["candidates"][1]["candidateId"],
            "candidate-1"
        );

        let materialized = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "materialize_search_candidate",
                "sessionId": session,
                "searchId": resumable_id,
                "candidateId": "candidate-1"
            }),
        );
        assert_eq!(materialized["ok"], true, "{materialized}");
        assert_eq!(materialized["data"]["inputs"], json!(["right"]));
        assert!(materialized["data"]["terminalStateId"].is_string());

        let second_advance = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "advance_search",
                "sessionId": session,
                "searchId": resumable_id,
                "allowance": { "maxExpandedNodes": 1, "maxMillis": 1000 }
            }),
        );
        assert_eq!(second_advance["ok"], true, "{second_advance}");
        assert_eq!(second_advance["data"]["status"], "paused");
        assert_eq!(second_advance["data"]["stats"]["expanded"], 2);
        let exhausted = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "advance_search",
                "sessionId": session,
                "searchId": resumable_id,
                "allowance": { "maxExpandedNodes": 1, "maxMillis": 1000 }
            }),
        );
        assert_eq!(exhausted["data"]["status"], "exhausted");
        assert_eq!(exhausted["data"]["stats"]["expanded"], 2);

        let solved_search = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "create_search",
                "sessionId": session,
                "goalId": search_goal_id,
                "fromStateId": initial,
                "algorithm": "bfs",
                "limits": { "maxDepth": 4, "maxStoredNodes": 100 }
            }),
        );
        let solved_search_id = solved_search["data"]["searchId"].as_str().unwrap();
        let solved_advance = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "advance_search",
                "sessionId": session,
                "searchId": solved_search_id,
                "allowance": { "maxExpandedNodes": 1, "maxMillis": 1000 }
            }),
        );
        assert_eq!(
            solved_advance["data"]["status"], "solved",
            "{solved_advance}"
        );
        assert_eq!(solved_advance["data"]["solutionCandidateId"], "candidate-1");
        let rejected_terminal_advance = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "advance_search",
                "sessionId": session,
                "searchId": solved_search_id,
                "allowance": { "maxExpandedNodes": 1, "maxMillis": 1000 }
            }),
        );
        assert_eq!(
            rejected_terminal_advance["error"]["code"],
            "search_not_advanceable"
        );

        let closed_search = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "close_search",
                "sessionId": session,
                "searchId": resumable_id
            }),
        );
        assert_eq!(closed_search["data"]["closed"], true);
        let closed_inspect = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_search",
                "sessionId": session,
                "searchId": resumable_id,
                "candidateLimit": 1
            }),
        );
        assert_eq!(closed_inspect["error"]["code"], "unknown_search");

        let invalid_budget = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": search_goal_id,
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 0, "maxStoredNodes": 100, "maxMillis": 1000 }
            }),
        );
        assert_eq!(invalid_budget["error"]["code"], "invalid_search_budget");
        remove_source(path);
    }

    #[test]
    fn derive_state_replaces_named_object_positions_without_ascii_authoring() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let initial_hash = compiled["data"]["initialStates"][0]["stateHash"]
            .as_str()
            .unwrap();

        let derived = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "expectedBaseHash": initial_hash,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [1, 0], "objects": ["Player"] },
                    { "kind": "excludes", "position": [0, 0], "objects": ["Player"] }
                ]
            }),
        );
        assert_eq!(derived["ok"], true, "{derived}");
        assert_eq!(derived["data"]["provenance"]["kind"], "hypothetical");
        assert_eq!(derived["data"]["provenance"]["origin"], "derived");
        assert_eq!(
            derived["data"]["provenance"]["inputCountAfterDerivation"],
            0
        );
        assert_eq!(
            derived["data"]["applied"]["setObjectPositions"],
            json!([{ "object": "Player", "positions": [[1, 0]] }])
        );
        assert_eq!(
            derived["data"]["diff"]["objects"],
            json!([{
                "object": "Player",
                "removed": [[0, 0]],
                "added": [[1, 0]]
            }])
        );
        let derived_state = derived["data"]["stateId"].as_str().unwrap();

        let original = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": initial
            }),
        );
        let original_player = original["data"]["state"]["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["name"] == "Player")
            .unwrap();
        assert_eq!(original_player["positions"], json!([[0, 0]]));

        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": derived_state,
                "inputs": ["right"]
            }),
        );
        assert_eq!(run["ok"], true, "{run}");

        let exported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "export_semantic_state",
                "sessionId": session,
                "stateId": derived_state
            }),
        );
        let mut goal = exported["data"].clone();
        goal["kind"] = json!("puzzle2d-semantic-goal");
        goal.as_object_mut().unwrap().remove("variables");
        goal["legend"] = json!({
            "?": { "kind": "unknown" },
            "P": { "kind": "contains", "objects": ["Player"] }
        });
        goal["lines"] = json!(["??P"]);
        let imported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");
        let solved = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": imported["data"]["goalId"],
                "fromStateId": derived_state,
                "algorithm": "best_first",
                "budget": { "maxDepth": 2, "maxStoredNodes": 20, "maxMillis": 1000 }
            }),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
        assert_eq!(solved["data"]["inputs"], json!(["right"]));
        let terminal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": solved["data"]["terminalStateId"]
            }),
        );
        assert_eq!(terminal["data"]["provenance"]["kind"], "hypothetical");
        assert_eq!(terminal["data"]["provenance"]["origin"], "derived");
        assert_eq!(
            terminal["data"]["provenance"]["inputCountAfterDerivation"],
            1
        );

        let stale = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "expectedBaseHash": "0000000000000000",
                "setObjectPositions": []
            }),
        );
        assert_eq!(stale["error"]["code"], "derived_state_base_mismatch");

        let conflict = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Trail", "positions": [[2, 0]] }
                ]
            }),
        );
        assert_eq!(conflict["error"]["code"], "derived_state_layer_conflict");

        let duplicate_object = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[0, 0]] },
                    { "object": "Player", "positions": [[1, 0]] }
                ]
            }),
        );
        assert_eq!(
            duplicate_object["error"]["code"],
            "duplicate_derived_state_object"
        );

        let duplicate_position = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0], [1, 0]] }
                ]
            }),
        );
        assert_eq!(
            duplicate_position["error"]["code"],
            "duplicate_derived_state_position"
        );

        let removed = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [] }
                ],
                "assert": [
                    { "kind": "exact", "position": [0, 0], "objects": [] }
                ]
            }),
        );
        assert_eq!(removed["ok"], true, "{removed}");
        assert_eq!(
            removed["data"]["diff"]["objects"],
            json!([{
                "object": "Player",
                "removed": [[0, 0]],
                "added": []
            }])
        );

        let same_layer_swap = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Goal", "positions": [[1, 0]] },
                    { "object": "Trail", "positions": [[2, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [1, 0], "objects": ["Goal"] },
                    { "kind": "contains", "position": [2, 0], "objects": ["Trail"] }
                ]
            }),
        );
        assert_eq!(same_layer_swap["ok"], true, "{same_layer_swap}");

        let failed_assertion = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[0, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [0, 0], "objects": ["Goal"] }
                ]
            }),
        );
        assert_eq!(
            failed_assertion["error"]["code"],
            "derived_state_assertion_failed"
        );

        let impossible_contains = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[0, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [2, 0], "objects": ["Goal", "Trail"] }
                ]
            }),
        );
        assert_eq!(
            impossible_contains["error"]["code"],
            "invalid_derived_state_assertion"
        );

        let impossible_exact = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[0, 0]] }
                ],
                "assert": [
                    { "kind": "exact", "position": [2, 0], "objects": ["Goal", "Trail"] }
                ]
            }),
        );
        assert_eq!(
            impossible_exact["error"]["code"],
            "invalid_derived_state_assertion"
        );

        let unknown_object = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Missing", "positions": [[0, 0]] }
                ]
            }),
        );
        assert_eq!(unknown_object["error"]["code"], "unknown_object");

        let object_out_of_bounds = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[3, 0]] }
                ]
            }),
        );
        assert_eq!(
            object_out_of_bounds["error"]["code"],
            "semantic_position_out_of_bounds"
        );

        let assertion_out_of_bounds = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[0, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [3, 0], "objects": ["Player"] }
                ]
            }),
        );
        assert_eq!(
            assertion_out_of_bounds["error"]["code"],
            "semantic_position_out_of_bounds"
        );

        let typo = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPosition": [
                    { "object": "Player", "positions": [[1, 0]] }
                ]
            }),
        );
        assert_eq!(typo["error"]["code"], "invalid_request", "{typo}");

        let nested_typo = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]], "position": [1, 0] }
                ]
            }),
        );
        assert_eq!(nested_typo["error"]["code"], "invalid_request");

        let empty = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial
            }),
        );
        assert_eq!(empty["error"]["code"], "empty_derived_state_patch");

        remove_source(path);
    }

    #[test]
    fn derive_state_changes_only_named_non_persistent_variables() {
        let path = write_source_text(VARIABLE_SOURCE);
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let derived = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setVariables": [{ "variable": "count", "value": 3 }]
            }),
        );
        assert_eq!(derived["ok"], true, "{derived}");
        assert_eq!(derived["data"]["state"]["variables"]["count"], 3);
        assert_eq!(derived["data"]["state"]["variables"]["saved"], 0);
        assert_eq!(
            derived["data"]["diff"]["variables"],
            json!([{ "variable": "count", "before": 0, "after": 3 }])
        );

        let persistent = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setVariables": [{ "variable": "saved", "value": 1 }]
            }),
        );
        assert_eq!(
            persistent["error"]["code"],
            "derived_state_persistent_variable_change"
        );

        let unknown = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setVariables": [{ "variable": "missing", "value": 1 }]
            }),
        );
        assert_eq!(unknown["error"]["code"], "unknown_variable");

        let duplicate = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setVariables": [
                    { "variable": "count", "value": 1 },
                    { "variable": "count", "value": 2 }
                ]
            }),
        );
        assert_eq!(
            duplicate["error"]["code"],
            "duplicate_derived_state_variable"
        );

        remove_source(path);
    }

    #[test]
    fn start_level_from_state_applies_the_patch_before_level_start_once() {
        let path = write_source_text(LEVEL_START_SOURCE);
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let derived = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": initial,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]] }
                ]
            }),
        );
        assert_eq!(derived["ok"], true, "{derived}");
        let derived_objects = derived["data"]["state"]["objects"].as_array().unwrap();
        let derived_started = derived_objects
            .iter()
            .find(|object| object["name"] == "Started")
            .unwrap();
        assert_eq!(derived_started["positions"], json!([[0, 0]]));
        assert_eq!(derived["data"]["state"]["variables"]["starts"], 1);
        assert_eq!(derived["data"]["state"]["variables"]["count"], 1);

        let started = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "start_level_from_state",
                "sessionId": session,
                "levelIndex": 0,
                "expectedLevelName": "start",
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]] }
                ],
                "setVariables": [
                    { "variable": "count", "value": 10 }
                ],
                "assert": [
                    { "kind": "contains", "position": [1, 0], "objects": ["Player", "Started"] },
                    { "kind": "excludes", "position": [0, 0], "objects": ["Started"] }
                ]
            }),
        );
        assert_eq!(started["ok"], true, "{started}");
        assert_eq!(started["data"]["state"]["variables"]["starts"], 1);
        assert_eq!(started["data"]["state"]["variables"]["count"], 11);
        assert_eq!(started["data"]["provenance"]["origin"], "level_start");
        assert_eq!(started["data"]["provenance"]["inputCountAfterStart"], 0);
        let started_state = started["data"]["stateId"].as_str().unwrap();

        let run = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": started_state,
                "inputs": ["right"]
            }),
        );
        assert_eq!(run["ok"], true, "{run}");
        let terminal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": run["data"]["terminalStateId"]
            }),
        );
        assert_eq!(terminal["data"]["state"]["variables"]["starts"], 1);
        assert_eq!(terminal["data"]["state"]["variables"]["count"], 11);
        assert_eq!(terminal["data"]["provenance"]["origin"], "level_start");
        assert_eq!(terminal["data"]["provenance"]["inputCountAfterStart"], 1);

        let exported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "export_semantic_state",
                "sessionId": session,
                "stateId": started_state
            }),
        );
        let mut goal = exported["data"].clone();
        goal["kind"] = json!("puzzle2d-semantic-goal");
        goal.as_object_mut().unwrap().remove("variables");
        goal["legend"] = json!({
            "?": { "kind": "unknown" },
            "P": { "kind": "contains", "objects": ["Player"] }
        });
        goal["lines"] = json!(["??P"]);
        let imported = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");
        let solved = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": imported["data"]["goalId"],
                "fromStateId": started_state,
                "algorithm": "best_first",
                "budget": { "maxDepth": 1, "maxStoredNodes": 10, "maxMillis": 1000 }
            }),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
        assert_eq!(solved["data"]["inputs"], json!(["right"]));

        let mismatched_level = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "start_level_from_state",
                "sessionId": session,
                "levelIndex": 0,
                "expectedLevelName": "other",
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]] }
                ]
            }),
        );
        assert_eq!(
            mismatched_level["error"]["code"],
            "level_start_state_level_mismatch"
        );

        let empty = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "start_level_from_state",
                "sessionId": session,
                "levelIndex": 0
            }),
        );
        assert_eq!(empty["error"]["code"], "empty_level_start_state_patch");

        let persistent = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "start_level_from_state",
                "sessionId": session,
                "levelIndex": 0,
                "setVariables": [
                    { "variable": "starts", "value": 2 }
                ]
            }),
        );
        assert_eq!(
            persistent["error"]["code"],
            "level_start_state_persistent_variable_change"
        );

        let failed_assertion = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "start_level_from_state",
                "sessionId": session,
                "levelIndex": 0,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[1, 0]] }
                ],
                "assert": [
                    { "kind": "contains", "position": [0, 0], "objects": ["Player"] }
                ]
            }),
        );
        assert_eq!(
            failed_assertion["error"]["code"],
            "level_start_state_assertion_failed"
        );

        remove_source(path);
    }

    #[test]
    fn compile_reports_authoritative_level_start_failure() {
        let path = write_source_text(FAILING_LEVEL_START_SOURCE);
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );

        assert_eq!(compiled["ok"], false, "{compiled}");
        assert_eq!(compiled["error"]["code"], "level_start_failed");
        assert!(
            compiled["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("VariableDivisionByZero")),
            "{compiled}"
        );

        remove_source(path);
    }

    #[test]
    fn derive_state_preserves_the_base_checkpoint_restart_anchor() {
        let path = write_source_text(DERIVED_CHECKPOINT_SOURCE);
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": AGENT_PROTOCOL_VERSION, "op": "compile", "path": path }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let checkpointed = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "save"]
            }),
        );
        assert_eq!(checkpointed["ok"], true, "{checkpointed}");
        let checkpointed_state = checkpointed["data"]["terminalStateId"].as_str().unwrap();

        let derived = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "derive_state",
                "sessionId": session,
                "baseStateId": checkpointed_state,
                "setObjectPositions": [
                    { "object": "Player", "positions": [[2, 0]] }
                ]
            }),
        );
        assert_eq!(derived["ok"], true, "{derived}");

        let restarted = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "run",
                "sessionId": session,
                "fromStateId": derived["data"]["stateId"],
                "inputs": ["restart"]
            }),
        );
        assert_eq!(restarted["ok"], true, "{restarted}");
        let terminal = request(
            &mut server,
            json!({
                "version": AGENT_PROTOCOL_VERSION,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": restarted["data"]["terminalStateId"]
            }),
        );
        let objects = terminal["data"]["state"]["objects"].as_array().unwrap();
        let player = objects
            .iter()
            .find(|object| object["name"] == "Player")
            .unwrap();
        assert_eq!(player["positions"], json!([[1, 0]]));

        remove_source(path);
    }

    #[test]
    fn malformed_json_does_not_poison_the_server() {
        let mut server = AgentServer::new();
        let failed: Value = serde_json::from_str(&server.handle_line("{")).unwrap();
        assert_eq!(failed["error"]["code"], "invalid_request");
        let next: Value = serde_json::from_str(
            &server.handle_line(r#"{"version":2,"op":"manifest","sessionId":"missing"}"#),
        )
        .unwrap();
        assert_eq!(next["error"]["code"], "unknown_session");
    }
}
