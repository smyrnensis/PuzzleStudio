use serde_json::{Value, json};
use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

const WARMUP_RUNS: usize = 1;
const SAMPLE_RUNS: usize = 5;
const REFERENCE_JSON: &str = include_str!("puzzlescript_next_microban.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SolverRun {
    elapsed: Duration,
    depth: u64,
    visited: u64,
    expanded: u64,
}

fn main() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let puzzle_path = manifest_dir.join("../../games/microban/game.puzzle");
    let source = fs::read_to_string(&puzzle_path).expect("Microban source must be readable");
    let preview_html = html_play::export_editor_preview_html_from_source(
        &source,
        &puzzle_path.display().to_string(),
        "",
        "",
    )
    .expect("Microban editor preview must compile");
    let preview = embedded_runtime_export(&preview_html);
    let rules: Value = serde_json::from_str(
        &html_play::export_solver_rules_json_from_source(
            &source,
            &puzzle_path.display().to_string(),
        )
        .expect("Microban solver rules must compile"),
    )
    .expect("solver rules must be valid JSON");
    let reference: Value = serde_json::from_str(REFERENCE_JSON)
        .expect("PuzzleScriptNext reference must be valid JSON");

    let engine = reference["engine"]
        .as_object()
        .expect("reference engine must be an object");
    println!(
        "reference: {} build {}",
        engine["name"].as_str().expect("reference engine name"),
        engine["build"].as_str().expect("reference engine build")
    );

    for case in reference["cases"]
        .as_array()
        .expect("reference cases must be an array")
    {
        run_case(&preview, &rules, case);
    }
}

fn run_case(preview: &Value, rules: &Value, reference: &Value) {
    let case_id = reference["id"].as_str().expect("case id");
    let level_index = reference["levelIndex"].as_u64().expect("level index") as usize;
    let level = preview["levels"]
        .as_array()
        .expect("preview levels must be an array")
        .get(level_index)
        .unwrap_or_else(|| panic!("reference level index {level_index} is out of range"));
    assert_eq!(
        level["name"], case_id,
        "reference case must identify the same level"
    );

    let request = json!({
        "version": 1,
        "rules": {
            "compileId": format!("microban-bench-{level_index}"),
            "documentId": "microban-bench",
            "modelKind": rules["modelKind"].clone(),
            "compiledPlay": rules["compiledPlay"].clone(),
            "loadedGame": rules["loadedGame"].clone(),
            "runRulesOnLevelStart": rules["runRulesOnLevelStart"].clone(),
            "goal": rules["goal"].clone(),
            "lose": rules["lose"].clone(),
            "solverStrategy": rules["solverStrategy"].clone()
        },
        "target": {
            "origin": "preview-level",
            "compileId": format!("microban-bench-{level_index}"),
            "documentId": "microban-bench",
            "level": {"index": level_index, "levelName": level["name"].clone()},
            "state": {
                "kind": "compiled-start",
                "lifecycle": "playable-start",
                "data": level["initialState"].clone()
            }
        },
        "maxDepth": 512,
        "maxNodes": 100_000,
        "maxMs": 0
    })
    .to_string();

    for _ in 0..WARMUP_RUNS {
        black_box(run_solver(black_box(&request)));
    }

    let mut samples = Vec::with_capacity(SAMPLE_RUNS);
    for _ in 0..SAMPLE_RUNS {
        samples.push(run_solver(black_box(&request)));
    }

    let first = samples[0];
    assert!(
        samples.iter().all(|sample| {
            sample.depth == first.depth
                && sample.visited == first.visited
                && sample.expanded == first.expanded
        }),
        "solver search stats must be deterministic"
    );

    let mut elapsed = samples
        .iter()
        .map(|sample| sample.elapsed)
        .collect::<Vec<_>>();
    elapsed.sort_unstable();
    let median = elapsed[elapsed.len() / 2];
    let min = elapsed[0];
    let max = elapsed[elapsed.len() - 1];
    let reference_moves = reference["solutionMoves"]
        .as_u64()
        .expect("reference solution moves");
    let reference_positions = reference["exploredPositions"]
        .as_u64()
        .expect("reference explored positions");

    println!("\n{case_id}");
    println!(
        "  PuzzleStudio: depth={} visited={} expanded={} median={:.3}ms min={:.3}ms max={:.3}ms n={}",
        first.depth,
        first.visited,
        first.expanded,
        milliseconds(median),
        milliseconds(min),
        milliseconds(max),
        SAMPLE_RUNS
    );
    println!(
        "  PuzzleScriptNext reference: moves={} positions={}",
        reference_moves, reference_positions
    );
    println!(
        "  ratios: depth/moves={:.3} visited/positions={:.3} expanded/positions={:.3}",
        first.depth as f64 / reference_moves as f64,
        first.visited as f64 / reference_positions as f64,
        first.expanded as f64 / reference_positions as f64
    );
}

fn embedded_runtime_export(html: &str) -> Value {
    const MARKER: &str = "window.PuzzleRuntimeExportJson = \"";
    const TERMINATOR: &str = "\";";
    let start = html
        .find(MARKER)
        .expect("editor preview must embed PuzzleRuntimeExportJson")
        + MARKER.len();
    let rest = &html[start..];
    let end = rest
        .find(TERMINATOR)
        .expect("PuzzleRuntimeExportJson assignment must close");
    let encoded = &rest[..end];
    let json_text: String = serde_json::from_str(&format!("\"{encoded}\""))
        .expect("PuzzleRuntimeExportJson must be a JSON string literal");
    serde_json::from_str(&json_text).expect("PuzzleRuntimeExportJson must contain JSON")
}

fn run_solver(request: &str) -> SolverRun {
    let started_at = Instant::now();
    let response_text =
        html_play::solve_solver_task_json(request).expect("Microban solver request must succeed");
    let elapsed = started_at.elapsed();
    let response: Value =
        serde_json::from_str(&response_text).expect("solver response must be valid JSON");
    assert_eq!(response["result"], "solved", "solver must solve the case");
    let progress = response["observations"]
        .as_array()
        .and_then(|observations| observations.last())
        .and_then(|observation| observation.get("progress"))
        .expect("solved response must retain final search progress");
    SolverRun {
        elapsed,
        depth: response["depth"].as_u64().expect("solution depth"),
        visited: progress["visited"].as_u64().expect("visited count"),
        expanded: progress["expanded"].as_u64().expect("expanded count"),
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
