use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

use puzzle_lang::WorkspaceSourceDocument;
use puzzle_runtime_contract::{SolverSearchRequest, SolverSearchStatus, SolverStateSnapshot};
use puzzle_solver_runtime::SolverService;
use serde_json::Value;

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
    let puzzle_path = puzzle_path
        .to_str()
        .expect("Microban path must be valid UTF-8");
    let source = fs::read_to_string(puzzle_path).expect("Microban source must be readable");
    let document = puzzle_lang::parse_game_for_path(&source, puzzle_path)
        .expect("Microban source must compile");
    let loaded = puzzle_play::loaded_document_scene_host_loaded_game(&document)
        .expect("Microban must provide a 2D solver model");
    let reference: Value = serde_json::from_str(REFERENCE_JSON)
        .expect("PuzzleScriptNext reference must be valid JSON");
    let mut service = SolverService::new();
    let prepared = service
        .prepare_workspace(
            puzzle_path,
            vec![WorkspaceSourceDocument {
                path: puzzle_path.to_string(),
                source,
            }],
            0,
        )
        .expect("Microban solver artifact must prepare");

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
        run_case(&mut service, &prepared.artifact_id, &loaded, case);
    }
}

fn run_case(
    service: &mut SolverService,
    artifact_id: &str,
    loaded: &puzzle_lang::LoadedGame,
    reference: &Value,
) {
    let case_id = reference["id"].as_str().expect("case id");
    let level_index = reference["levelIndex"].as_u64().expect("level index") as usize;
    let level = loaded
        .levels
        .get(level_index)
        .unwrap_or_else(|| panic!("reference level index {level_index} is out of range"));
    assert_eq!(
        level.name, case_id,
        "reference case must identify the same level"
    );
    let state = SolverStateSnapshot::from_state2(&level.initial_state);

    for _ in 0..WARMUP_RUNS {
        black_box(run_solver(
            service,
            artifact_id,
            level_index,
            black_box(state.clone()),
        ));
    }

    let mut samples = Vec::with_capacity(SAMPLE_RUNS);
    for _ in 0..SAMPLE_RUNS {
        samples.push(run_solver(
            service,
            artifact_id,
            level_index,
            black_box(state.clone()),
        ));
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

fn run_solver(
    service: &mut SolverService,
    artifact_id: &str,
    level_index: usize,
    state: SolverStateSnapshot,
) -> SolverRun {
    let started_at = Instant::now();
    let search_id = service
        .start(
            artifact_id,
            SolverSearchRequest {
                level_index,
                state,
                materialize_level_start: true,
                max_depth: 512,
                max_stored_nodes: 100_000,
            },
            0,
        )
        .expect("Microban search must start");
    let result = loop {
        let response = service
            .advance(search_id, 100_000, 60_000, 0)
            .expect("Microban search must advance");
        if response.status != SolverSearchStatus::Paused {
            break response.result.expect("terminal search must have a result");
        }
    };
    let elapsed = started_at.elapsed();
    assert_eq!(
        result.result,
        SolverSearchStatus::Solved,
        "solver must solve the case"
    );
    SolverRun {
        elapsed,
        depth: u64::from(result.depth.expect("solution depth")),
        visited: result.stats.visited as u64,
        expanded: result.stats.expanded as u64,
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
