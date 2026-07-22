use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::fs;

use puzzle_solver_runtime::investigation::InvestigationService;
pub use puzzle_solver_runtime::investigation::{
    InvestigationCommand as AgentCommand, InvestigationError as AgentError, ObservationMode,
    ObservationRequest, SearchSessionAllowance, SearchSessionLimits, SemanticGoalArtifact,
    SemanticGoalSearchAlgorithm, SemanticGoalSearchBudget, SemanticLegendMeaning,
    SemanticStateArtifact,
};

pub const AGENT_PROTOCOL_VERSION: u32 = 1;

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
title = agent_session

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

    fn write_source() -> std::path::PathBuf {
        write_source_text(SOURCE)
    }

    fn write_source_text(source: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "puzzle-agent-runtime-{}-{}.puzzle",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, source).unwrap();
        path
    }

    fn request(server: &mut AgentServer, value: Value) -> Value {
        serde_json::from_str(&server.handle_line(&value.to_string())).unwrap()
    }

    #[test]
    fn compiled_session_runs_sequences_and_inspects_selected_points() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": 1, "requestId": "c", "op": "compile", "path": path }),
        );
        assert_eq!(compiled["ok"], true);
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();

        let run = request(
            &mut server,
            json!({
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
                "op": "inspect_run",
                "sessionId": session,
                "runId": run["data"]["runId"],
                "at": [0, 2],
                "includeTrace": true
            }),
        );
        assert_eq!(inspected["ok"], true, "{inspected}");
        assert_eq!(inspected["data"]["points"].as_array().unwrap().len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn locked_stuck_room_sequence_reaches_win_in_five_inputs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../games/TPGJ6/locked.puzzle");
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": goal_id,
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 1, "maxNodes": 10, "maxMillis": 1000 }
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
title = semantic_completion_observation
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
            json!({ "version": 1, "op": "compile", "path": path, "model": "board" }),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let exported = request(
            &mut server,
            json!({
                "version": 1,
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
                "version": 1,
                "op": "import_semantic_goal",
                "sessionId": session,
                "artifact": goal
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");

        let solved = request(
            &mut server,
            json!({
                "version": 1,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": imported["data"]["goalId"],
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 1, "maxNodes": 10, "maxMillis": 1000 }
            }),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(solved["data"]["searchOutcome"], "solved", "{solved}");
        assert_eq!(solved["data"]["result"], "semantic_goal_reached");
        assert_eq!(solved["data"]["inputs"], json!(["right"]));
        let terminal = request(
            &mut server,
            json!({
                "version": 1,
                "op": "inspect_state",
                "sessionId": session,
                "stateId": solved["data"]["terminalStateId"]
            }),
        );
        assert_eq!(terminal["data"]["levelIndex"], 1, "{terminal}");
        let continued = request(
            &mut server,
            json!({
                "version": 1,
                "op": "run",
                "sessionId": session,
                "fromStateId": solved["data"]["terminalStateId"],
                "inputs": ["right"]
            }),
        );
        assert_eq!(continued["ok"], true, "{continued}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_input_does_not_create_a_partial_run() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": 1, "op": "compile", "path": path }),
        );
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let failed = request(
            &mut server,
            json!({
                "version": 1,
                "op": "run",
                "sessionId": session,
                "fromStateId": initial,
                "inputs": ["right", "missing", "right"]
            }),
        );
        assert_eq!(failed["error"]["code"], "unknown_input");
        let manifest = request(
            &mut server,
            json!({ "version": 1, "op": "manifest", "sessionId": session }),
        );
        assert_eq!(manifest["ok"], true);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn semantic_state_round_trips_and_imports_an_ai_edited_hypothesis() {
        let path = write_source();
        let mut server = AgentServer::new();
        let compiled = request(
            &mut server,
            json!({ "version": 1, "op": "compile", "path": path }),
        );
        let session = compiled["data"]["sessionId"].as_str().unwrap();
        let initial = compiled["data"]["initialStates"][0]["stateId"]
            .as_str()
            .unwrap();
        let exported = request(
            &mut server,
            json!({
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                    "version": 1,
                    "op": "solve_semantic_goal",
                    "sessionId": session,
                    "goalId": search_goal_id,
                    "fromStateId": initial,
                    "algorithm": algorithm,
                    "budget": {
                        "maxDepth": 4,
                        "maxNodes": 100,
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
                    "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
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
                "version": 1,
                "op": "close_search",
                "sessionId": session,
                "searchId": resumable_id
            }),
        );
        assert_eq!(closed_search["data"]["closed"], true);
        let closed_inspect = request(
            &mut server,
            json!({
                "version": 1,
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
                "version": 1,
                "op": "solve_semantic_goal",
                "sessionId": session,
                "goalId": search_goal_id,
                "fromStateId": initial,
                "algorithm": "bfs",
                "budget": { "maxDepth": 0, "maxNodes": 100, "maxMillis": 1000 }
            }),
        );
        assert_eq!(invalid_budget["error"]["code"], "invalid_search_budget");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn malformed_json_does_not_poison_the_server() {
        let mut server = AgentServer::new();
        let failed: Value = serde_json::from_str(&server.handle_line("{")).unwrap();
        assert_eq!(failed["error"]["code"], "invalid_request");
        let next: Value = serde_json::from_str(
            &server.handle_line(r#"{"version":1,"op":"manifest","sessionId":"missing"}"#),
        )
        .unwrap();
        assert_eq!(next["error"]["code"], "unknown_session");
    }
}
