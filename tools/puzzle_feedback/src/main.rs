use puzzle_core::ObjectId;
use puzzle_lang::LoadedGame;
use puzzle_play::{GameSession, cell_objects, render_ascii_top};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    path: PathBuf,
    level: usize,
    inputs: Vec<String>,
    watch: Vec<String>,
    cells: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1).collect())?;
    let entry = puzzle_lang::resolve_game_entry(&args.path).map_err(|error| error.to_string())?;
    let root = entry.parent().unwrap_or_else(|| std::path::Path::new("."));
    let document = puzzle_workspace::FileWorkspace::load(&entry, root)?.compile()
        .map_err(|error| error.to_string())?;
    let game = puzzle_play::loaded_document_scene_host_loaded_game(&document)?;
    if game.levels.is_empty() {
        return Err("game has no levels".to_string());
    }
    if args.level >= game.levels.len() {
        return Err(format!(
            "level index {} is out of range; game has {} levels",
            args.level,
            game.levels.len()
        ));
    }

    let mut session = GameSession::new(&game);
    session.start_level(&game, args.level);
    print_snapshot("after level_start", &game, &session, &args);

    for input in &args.inputs {
        session
            .apply_command(&game, input)
            .map_err(|error| format!("input `{input}` failed: {error:?}"))?;
        print_snapshot(&format!("after input `{input}`"), &game, &session, &args);
    }

    Ok(())
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    if raw.is_empty() || raw.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Err(usage());
    }

    let mut path = None;
    let mut level = 0usize;
    let mut inputs = Vec::new();
    let mut watch = vec![
        "Player".to_string(),
        "OnPlayer".to_string(),
        "CanEnter".to_string(),
        "Open".to_string(),
        "Room".to_string(),
        "Locked".to_string(),
        "Box:movable".to_string(),
        "Box:stack".to_string(),
        "Pull:movable".to_string(),
        "Pull:stack".to_string(),
    ];
    let mut cells = false;

    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--level" => {
                i += 1;
                let value = raw
                    .get(i)
                    .ok_or_else(|| "--level requires a zero-based index".to_string())?;
                level = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --level index: {value}"))?;
            }
            "--inputs" => {
                i += 1;
                let value = raw
                    .get(i)
                    .ok_or_else(|| "--inputs requires comma-separated input names".to_string())?;
                inputs.extend(split_csv(value));
            }
            "--watch" => {
                i += 1;
                let value = raw.get(i).ok_or_else(|| {
                    "--watch requires comma-separated object/group names".to_string()
                })?;
                watch = split_csv(value);
            }
            "--cells" => {
                cells = true;
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}\n\n{}", usage()));
            }
            value => {
                if path.is_some() {
                    return Err(format!("unexpected extra path/argument: {value}"));
                }
                path = Some(PathBuf::from(value));
            }
        }
        i += 1;
    }

    let path = path.ok_or_else(usage)?;
    Ok(Args {
        path,
        level,
        inputs,
        watch,
        cells,
    })
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn usage() -> String {
    "usage: cargo run --manifest-path tools/puzzle_feedback/Cargo.toml -- <game.puzzle> [--level N] [--inputs right,down] [--watch Locked,Open,Room] [--cells]".to_string()
}

fn print_snapshot(label: &str, game: &LoadedGame, session: &GameSession, args: &Args) {
    let state = active_state(game, session);
    println!("== {label} ==");
    println!(
        "scene={} level={} size={}x{} layers={}",
        session.scene(),
        session.level_index(),
        state.width,
        state.height,
        state.layer_count
    );
    print_variables(game, state.visible_variables());
    println!("ascii:");
    println!("{}", render_ascii_top(state, &game.legend));
    println!("watched:");
    for name in &args.watch {
        print_named_positions(game, state, name);
    }
    if args.cells {
        print_cells(game, state);
    }
    println!();
}

fn active_state<'a>(_game: &'a LoadedGame, session: &'a GameSession) -> &'a puzzle_core::State {
    if let Some(scene) = session.scene_state() {
        if scene.puzzles.len() == 1 {
            if let Some(puzzle) = scene.puzzles.values().next() {
                return &puzzle.state;
            }
        }
        if let Some(puzzle) = scene.puzzles.get("sokoban") {
            return &puzzle.state;
        }
    }
    session.state()
}

fn print_variables(game: &LoadedGame, values: &[i64]) {
    if values.is_empty() {
        return;
    }
    let names_by_index = game
        .variable_labels
        .iter()
        .map(|(id, name)| (usize::from(id.0), name.as_str()))
        .collect::<BTreeMap<_, _>>();
    let rendered = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let name = names_by_index.get(&index).copied().unwrap_or("<unnamed>");
            format!("{name}={value}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    println!("variables: {rendered}");
}

fn print_named_positions(game: &LoadedGame, state: &puzzle_core::State, name: &str) {
    let objects = resolve_name(game, name);
    if objects.is_empty() {
        println!("  {name}: <unknown object/group>");
        return;
    }

    let mut positions = BTreeSet::<(u16, u16)>::new();
    for object in objects {
        for slot in state.object_positions(object) {
            if let Some((x, y)) = state.slot_position(*slot) {
                positions.insert((x, y));
            }
        }
    }

    let coords = if positions.is_empty() {
        "-".to_string()
    } else {
        positions
            .into_iter()
            .map(|(x, y)| format!("({x},{y})"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    println!("  {name}: {coords}");
}

fn resolve_name(game: &LoadedGame, name: &str) -> Vec<ObjectId> {
    if let Some(objects) = game.object_groups.get(name) {
        return objects.clone();
    }
    game.object_labels
        .iter()
        .filter_map(|(object, label)| (label == name).then_some(*object))
        .collect()
}

fn print_cells(game: &LoadedGame, state: &puzzle_core::State) {
    println!("cells:");
    for y in 0..state.height {
        for x in 0..state.width {
            let names = cell_objects(state, x, y)
                .into_iter()
                .filter_map(|object| game.object_labels.get(&object))
                .map(String::as_str)
                .collect::<Vec<_>>();
            if !names.is_empty() {
                println!("  ({x},{y}): {}", names.join(" "));
            }
        }
    }
}
