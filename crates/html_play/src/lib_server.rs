#[cfg(not(target_arch = "wasm32"))]
fn bind_listener(preferred_port: u16) -> io::Result<(TcpListener, u16)> {
    for offset in 0..100 {
        let port = preferred_port.saturating_add(offset);
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return Ok((listener, port)),
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "no available port in requested range",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<ServerState>>,
) -> Result<(), AppError> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };

    let response = route(&request, state);
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn serve_static_html(html: String, puzzle_path: &Path, port: u16) -> Result<(), AppError> {
    let html = Arc::new(html);
    let (listener, port) = bind_listener(port)?;

    println!("html-play serving http://127.0.0.1:{port}");
    println!("puzzle: {}", puzzle_path.display());
    print_wasm_freshness_status();

    for stream in listener.incoming() {
        let stream = stream?;
        let html = Arc::clone(&html);
        if let Err(error) = handle_static_connection(stream, html) {
            eprintln!("request error: {error}");
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_static_connection(mut stream: TcpStream, html: Arc<String>) -> Result<(), AppError> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };

    let response = route_static_html(&request, &html);
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn route_static_html(request: &HttpRequest, html: &str) -> String {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => http_ok("text/html; charset=utf-8", html),
        _ => http_error(404, "not found"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn read_request(stream: &mut TcpStream) -> Result<Option<HttpRequest>, AppError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut header_end = None;
    let mut content_length = 0;

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);

        if header_end.is_none() {
            if let Some(end) = find_header_end(&bytes) {
                header_end = Some(end);
                content_length = parse_content_length(&bytes[..end]);
            }
        }

        if let Some(end) = header_end {
            if bytes.len() >= end + 4 + content_length {
                break;
            }
        }
    }

    let Some(end) = header_end else {
        return Err(AppError::Config("invalid HTTP request".to_string()));
    };
    let header = String::from_utf8_lossy(&bytes[..end]);
    let mut request_line = header.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let mut path = request_line.next().unwrap_or("/").to_string();
    if let Some(query_index) = path.find('?') {
        path.truncate(query_index);
    }

    Ok(Some(HttpRequest { method, path }))
}

#[cfg(not(target_arch = "wasm32"))]
fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_content_length(header: &[u8]) -> usize {
    String::from_utf8_lossy(header)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

#[cfg(not(target_arch = "wasm32"))]
fn route(request: &HttpRequest, state: Arc<Mutex<ServerState>>) -> String {
    let method = request.method.as_str();
    let path = request.path.as_str();

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => http_ok("text/html; charset=utf-8", INDEX_HTML),
        ("GET", "/app.css") => http_ok("text/css; charset=utf-8", APP_CSS),
        ("GET", "/theme-presets.css") => http_ok("text/css; charset=utf-8", THEME_PRESETS_CSS),
        ("GET", "/renderer.css") => http_ok("text/css; charset=utf-8", RENDERER_CSS),
        ("GET", "/game.css") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("text/css; charset=utf-8", &state.game_css)
        }
        ("GET", "/game.visuals.js") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("text/javascript; charset=utf-8", &state.game_visuals_js)
        }
        ("GET", "/sound-generator.js") => {
            let script = sound_tools_js();
            http_ok("text/javascript; charset=utf-8", &script)
        }
        ("GET", "/app.js") => http_ok("text/javascript; charset=utf-8", APP_JS),
        ("GET", "/renderer.js") => http_ok("text/javascript; charset=utf-8", RENDERER_JS),
        ("GET", "/api/scene") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("application/json; charset=utf-8", &state.scene_json())
        }
        #[cfg(feature = "solver")]
        ("POST", "/api/solve") => {
            let state = state.lock().expect("server state poisoned");
            match state.solve_json() {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        (method, path) => match puzzle_game_runtime::standalone_session_request(method, path) {
            Ok(request) => handle_standalone_session_request(state, request),
            Err(_) => http_error(404, "not found"),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_standalone_session_request(
    state: Arc<Mutex<ServerState>>,
    request: puzzle_game_runtime::StandaloneSessionRequest,
) -> String {
    match request {
        puzzle_game_runtime::StandaloneSessionRequest::State => {
            let mut state = state.lock().expect("server state poisoned");
            http_ok("application/json; charset=utf-8", &state.snapshot_json())
        }
        puzzle_game_runtime::StandaloneSessionRequest::Undo => {
            mutate(state, |state| state.session.undo(&state.loaded))
        }
        puzzle_game_runtime::StandaloneSessionRequest::Redo => {
            mutate(state, |state| state.session.redo(&state.loaded))
        }
        puzzle_game_runtime::StandaloneSessionRequest::Restart => {
            let mut state = state.lock().expect("server state poisoned");
            let result = {
                let ServerState {
                    session, loaded, ..
                } = &mut *state;
                session.restart_level(loaded)
            };
            match result {
                Ok(()) => http_ok("application/json; charset=utf-8", &state.snapshot_json()),
                Err(error) => http_error(400, &format!("{error:?}")),
            }
        }
        puzzle_game_runtime::StandaloneSessionRequest::Next => {
            mutate(state, |state| state.session.advance_level(&state.loaded))
        }
        puzzle_game_runtime::StandaloneSessionRequest::Input(input_name) => {
            let mut state = state.lock().expect("server state poisoned");
            match state.apply_input_name(&input_name) {
                Ok(()) => http_ok("application/json; charset=utf-8", &state.snapshot_json()),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        puzzle_game_runtime::StandaloneSessionRequest::DebugInput(input_name) => {
            let mut state = state.lock().expect("server state poisoned");
            match state.apply_debug_input_name_json(&input_name) {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        puzzle_game_runtime::StandaloneSessionRequest::Command(command_name) => {
            let mut state = state.lock().expect("server state poisoned");
            match state.apply_command_name(&command_name) {
                Ok(()) => http_ok("application/json; charset=utf-8", &state.snapshot_json()),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mutate<F>(state: Arc<Mutex<ServerState>>, update: F) -> String
where
    F: FnOnce(&mut ServerState),
{
    let mut state = state.lock().expect("server state poisoned");
    update(&mut state);
    http_ok("application/json; charset=utf-8", &state.snapshot_json())
}

fn input_id_by_name(loaded: &LoadedGame, input_name: &str) -> Option<InputId> {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == input_name).then_some(*id))
}

#[cfg(feature = "solver")]
fn solver_inputs_for_program(loaded: &LoadedGame, program: &[RuleStep]) -> Vec<InputId> {
    let mut inputs = BTreeSet::new();
    collect_solver_inputs(program, &mut inputs);

    let mut inputs = if inputs.is_empty() {
        loaded.input_labels.keys().copied().collect::<Vec<_>>()
    } else {
        inputs.into_iter().collect()
    };
    inputs.retain(|input| {
        loaded
            .input_labels
            .get(input)
            .is_none_or(|name| !is_solver_control_input(name))
    });
    inputs.sort();
    inputs
}

#[cfg(feature = "solver")]
fn collect_solver_inputs(program: &[RuleStep], inputs: &mut BTreeSet<InputId>) {
    for step in program {
        match step {
            RuleStep::Rule(rule) => {
                for guard in &rule.guards {
                    collect_solver_inputs_from_guard(guard, inputs);
                }
            }
            RuleStep::ConditionalBlock { condition, steps } => {
                collect_solver_inputs_from_condition(condition, inputs);
                collect_solver_inputs(steps, inputs);
            }
            RuleStep::ConditionalBranch {
                condition,
                then_steps,
                else_steps,
            } => {
                collect_solver_inputs_from_condition(condition, inputs);
                collect_solver_inputs(then_steps, inputs);
                collect_solver_inputs(else_steps, inputs);
            }
            RuleStep::Block {
                stop_condition,
                steps,
                ..
            } => {
                if let Some(condition) = stop_condition {
                    collect_solver_inputs_from_condition(condition, inputs);
                }
                collect_solver_inputs(steps, inputs);
            }
            RuleStep::LocalFrame { steps, .. } => collect_solver_inputs(steps, inputs),
            RuleStep::AfterTriggered { steps, then_steps } => {
                collect_solver_inputs(steps, inputs);
                collect_solver_inputs(then_steps, inputs);
            }
        }
    }
}

#[cfg(feature = "solver")]
fn collect_solver_inputs_from_condition(condition: &RuleCondition, inputs: &mut BTreeSet<InputId>) {
    match condition {
        RuleCondition::AnyInputMatches(matches) | RuleCondition::NoInputMatches(matches) => {
            for (input, _) in matches {
                inputs.insert(*input);
            }
        }
        RuleCondition::GuardBranches(branches) => {
            for branch in branches {
                for guard in branch {
                    collect_solver_inputs_from_guard(guard, inputs);
                }
            }
        }
        RuleCondition::AnyMatches(_) | RuleCondition::NoMatches(_) => {}
    }
}

#[cfg(feature = "solver")]
fn collect_solver_inputs_from_guard(guard: &Guard, inputs: &mut BTreeSet<InputId>) {
    if let Guard::InputIs(input) = guard {
        inputs.insert(*input);
    }
}

#[cfg(feature = "solver")]
fn is_solver_control_input(name: &str) -> bool {
    matches!(name, "undo" | "restart" | "next_level" | "previous_level")
}

#[cfg(feature = "solver")]
fn solver_inputs3(game: &Game3) -> Vec<InputId> {
    let mut inputs = game
        .inputs
        .iter()
        .filter(|input| !is_solver_control_input(&input.name))
        .map(|input| input.id)
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}

#[cfg(feature = "solver")]
fn push_solution_moves(out: &mut String, loaded: &LoadedGame, inputs: &[InputId]) {
    out.push_str("\"moves\":[");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_input_move(out, loaded, *input);
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_solution_steps(out: &mut String, loaded: &LoadedGame, steps: &[PuzzleSolutionStep]) {
    out.push_str("\"steps\":[");
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_input_move(out, loaded, input);
        } else {
            out.push_str("\"move\":null");
        }
        out.push(',');
        push_scene(out, loaded, &step.state, None, None);
        out.push('}');
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_solution_moves3(out: &mut String, parsed: &ParsedPuzzle3, inputs: &[InputId]) {
    out.push_str("\"moves\":[");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_input_move3(out, parsed, *input);
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_solution_steps3(out: &mut String, parsed: &ParsedPuzzle3, steps: &[Puzzle3SolutionStep]) {
    out.push_str("\"steps\":[");
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_input_move3(out, parsed, input);
        } else {
            out.push_str("\"move\":null");
        }
        out.push(',');
        push_json_bool(out, "completed", step.completed);
        out.push(',');
        out.push_str("\"clearCommands\":");
        if step.completed {
            push_lifecycle_commands3(out, &parsed.lifecycle.on_level_clear);
        } else {
            out.push_str("[]");
        }
        out.push(',');
        push_state3_scene(out, parsed, &step.state);
        out.push('}');
    }
    out.push(']');
}

fn push_input_move(out: &mut String, loaded: &LoadedGame, input: InputId) {
    out.push('{');
    let name = loaded
        .input_labels
        .get(&input)
        .map(String::as_str)
        .unwrap_or_else(|| panic!("compiled input {} is missing its required label", input.0));
    push_json_pair(out, "name", name);
    out.push(',');
    if let Some(key) = key_for_input(loaded, input) {
        push_json_pair(out, "key", &key);
    } else {
        out.push_str("\"key\":null");
    }
    out.push(',');
    if let Some(arrow) = arrow_for_input(loaded, input) {
        push_json_pair(out, "arrow", &arrow);
    } else {
        out.push_str("\"arrow\":null");
    }
    out.push('}');
}

fn push_input_move3(out: &mut String, parsed: &ParsedPuzzle3, input: InputId) {
    out.push('{');
    let input_def = parsed.game.input(input);
    let name = input_def
        .map(|input| input.name.as_str())
        .unwrap_or_else(|| panic!("compiled 3D input {} is missing its definition", input.0));
    push_json_pair(out, "name", name);
    out.push(',');
    out.push_str("\"key\":null");
    out.push(',');
    out.push_str("\"arrow\":null");
    out.push(',');
    if let Some(direction) = input_def.and_then(|input| input.direction) {
        push_json_pair(out, "direction", direction.name);
    } else {
        out.push_str("\"direction\":null");
    }
    out.push('}');
}

fn push_lifecycle_commands3(out: &mut String, commands: &[LifecycleCommand]) {
    out.push('[');
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_effect_fields(out, command);
        out.push('}');
    }
    out.push(']');
}

fn push_state3_scene(out: &mut String, parsed: &ParsedPuzzle3, state: &State3) {
    out.push_str("\"scene\":{");
    push_json_pair(out, "kind", "puzzle3d");
    out.push(',');
    push_size3(out, state.size);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"cells\":[");
    let mut first = true;
    for z in 0..state.size.height {
        for y in 0..state.size.depth {
            for x in 0..state.size.width {
                let position = Coord3 { x, y, z };
                let Ok(view) = state.cell_view(position) else {
                    continue;
                };
                if view.objects.is_empty() {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('{');
                push_coord3(out, position);
                out.push(',');
                out.push_str("\"objects\":[");
                for (object_index, object) in view.objects.iter().enumerate() {
                    if object_index > 0 {
                        out.push(',');
                    }
                    push_object3(out, parsed, *object);
                }
                out.push_str("]}");
            }
        }
    }
    out.push_str("]}");
}

fn push_size3(out: &mut String, size: Size3) {
    out.push_str("\"size\":{");
    push_json_number(out, "width", size.width as u64);
    out.push(',');
    push_json_number(out, "depth", size.depth as u64);
    out.push(',');
    push_json_number(out, "height", size.height as u64);
    out.push('}');
}

fn push_coord3(out: &mut String, position: Coord3) {
    out.push_str("\"position\":{");
    push_json_number(out, "x", position.x as u64);
    out.push(',');
    push_json_number(out, "y", position.y as u64);
    out.push(',');
    push_json_number(out, "z", position.z as u64);
    out.push('}');
}

fn push_object3(out: &mut String, parsed: &ParsedPuzzle3, object: ObjectId3) {
    out.push('{');
    push_json_number(out, "id", object.0 as u64);
    out.push(',');
    let name = parsed
        .catalog
        .objects
        .iter()
        .find_map(|entry| (entry.id == object).then_some(entry.name.as_str()))
        .unwrap_or_else(|| {
            panic!(
                "compiled 3D object {} is missing its required catalog entry",
                object.0
            )
        });
    push_json_pair(out, "name", name);
    out.push(',');
    if let Some(layer) = parsed.game.object_layer(object) {
        push_json_number(out, "layer", layer.0 as u64);
    } else {
        out.push_str("\"layer\":null");
    }
    out.push(',');
    push_json_pair(out, "sprite", name);
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_search_stats(out: &mut String, stats: &puzzle_solver::SearchStats) {
    out.push_str("\"stats\":{");
    push_json_number(out, "visited", stats.visited as u64);
    out.push(',');
    push_json_number(out, "expanded", stats.expanded as u64);
    out.push(',');
    push_json_number(out, "frontier", stats.frontier as u64);
    out.push(',');
    push_json_number(out, "maxDepthReached", stats.max_depth_reached as u64);
    out.push(',');
    push_json_number(out, "elapsedMs", stats.elapsed.as_millis() as u64);
    out.push('}');
}

fn push_session_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"gameState\":{");
    let values = session.session_values();
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, loaded, session, session.focused_scene(), value);
    }
    out.push('}');
}

fn push_scene_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    push_scene_state_for(
        out,
        loaded,
        session,
        session.focused_scene(),
        session.scene_state(),
    );
}

fn push_scene_state_for(
    out: &mut String,
    loaded: &LoadedGame,
    session: &GameSession,
    scene_name: &str,
    state: Option<&puzzle_play::SceneRuntimeState>,
) {
    out.push_str("\"sceneState\":{");
    let Some(state) = state else {
        out.push('}');
        return;
    };
    let mut entries = state.values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        push_scene_value(out, loaded, session, scene_name, value);
    }
    out.push('}');
}

fn push_scene_puzzles(out: &mut String, state: Option<&puzzle_play::SceneRuntimeState>) {
    out.push_str("\"scenePuzzles\":[");
    let Some(state) = state else {
        out.push(']');
        return;
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
    }
    out.push(']');
}

fn push_scene_puzzle_state(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"scenePuzzleState\":{");
    let Some(state) = session.scene_state() else {
        out.push('}');
        return;
    };
    let mut names = state.puzzles.keys().collect::<Vec<_>>();
    names.sort();
    for (index, name) in names.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push_str(":{");
        if let Some(level_index) = state
            .puzzles
            .get(name)
            .and_then(|puzzle| puzzle.level_index)
        {
            out.push_str("\"level\":");
            push_level_ref(
                out,
                loaded,
                session.cleared_levels(),
                session.focused_scene(),
                level_index,
            );
        } else {
            out.push_str("\"level\":null");
        }
        if let Some(puzzle) = state.puzzles.get(name) {
            out.push(',');
            let level = puzzle
                .level_index
                .and_then(|index| loaded.levels.get(index));
            push_scene_object_body(
                out,
                loaded,
                &puzzle.state,
                level,
                scene_resources(loaded, session.focused_scene()),
            );
        }
        out.push('}');
    }
    out.push('}');
}

fn scene_resources<'a>(
    loaded: &'a LoadedGame,
    scene_name: &str,
) -> Option<&'a puzzle_lang::SceneResources> {
    loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)
        .map(|scene| &scene.resources)
}

fn push_level_ref(
    out: &mut String,
    loaded: &LoadedGame,
    cleared_levels: &[bool],
    scene_name: &str,
    level_index: usize,
) {
    out.push('{');
    push_json_number(out, "index", level_index as u64);
    if let Some(level) = loaded.levels.get(level_index) {
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "label", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
    }
    out.push(',');
    push_json_bool(
        out,
        "cleared",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push(',');
    push_json_bool(
        out,
        "solved",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push(',');
    push_json_bool(
        out,
        "has_next",
        level_has_next_in_scene(loaded, scene_name, level_index),
    );
    out.push(',');
    push_json_bool(
        out,
        "last",
        !level_has_next_in_scene(loaded, scene_name, level_index),
    );
    out.push('}');
}

fn level_has_next_in_scene(loaded: &LoadedGame, scene_name: &str, level_index: usize) -> bool {
    let indices = html_scene_level_indices(loaded, scene_name);
    indices
        .iter()
        .position(|index| *index == level_index)
        .is_some_and(|position| position + 1 < indices.len())
}

fn html_scene_level_indices(loaded: &LoadedGame, scene_name: &str) -> Vec<usize> {
    let Some(scene) = loaded.scenes.iter().find(|scene| scene.name == scene_name) else {
        return (0..loaded.levels.len()).collect();
    };
    match &scene.resources.levels {
        ResourceSelection::All => (0..loaded.levels.len()).collect(),
        ResourceSelection::Named(names) => loaded
            .levels
            .iter()
            .enumerate()
            .filter_map(|(index, level)| {
                names
                    .iter()
                    .any(|name| html_level_resource_matches(name, &level.name))
                    .then_some(index)
            })
            .collect(),
    }
}

fn html_level_resource_matches(resource: &str, level_name: &str) -> bool {
    level_name == resource
        || level_name
            .strip_prefix(resource)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn push_visible_scenes(out: &mut String, scenes: &[String]) {
    out.push_str("\"visibleScenes\":[");
    for (index, scene) in scenes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, scene);
    }
    out.push(']');
}

fn push_scene_value(
    out: &mut String,
    loaded: &LoadedGame,
    session: &GameSession,
    scene_name: &str,
    value: &SceneValue,
) {
    match value {
        SceneValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        SceneValue::Int(value) => out.push_str(&value.to_string()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => push_json_string(out, value),
        SceneValue::LevelRef(index) => {
            push_level_ref(out, loaded, session.cleared_levels(), scene_name, *index)
        }
    }
}

fn push_scene_layers(out: &mut String, loaded: &LoadedGame, session: &GameSession) {
    out.push_str("\"sceneLayers\":[");
    for (index, name) in session.visible_scenes().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let state = session.scene_state_for(name);
        out.push('{');
        push_json_pair(out, "name", name);
        out.push(',');
        push_json_bool(out, "focused", name == session.focused_scene());
        out.push(',');
        push_scene_state_for(out, loaded, session, name, state);
        out.push(',');
        push_scene_puzzles(out, state);
        out.push(',');
        out.push_str("\"scene\":");
        if let Some((puzzle_state, level)) = scene_puzzle_state(loaded, session, name) {
            let scene_def = loaded.scenes.iter().find(|scene| scene.name == *name);
            push_scene_object(
                out,
                loaded,
                puzzle_state,
                level,
                scene_def.map(|scene| &scene.resources),
            );
        } else {
            out.push_str("null");
        }
        out.push('}');
    }
    out.push(']');
}

fn focused_scene_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
) -> Option<&'a puzzle_core::State> {
    scene_puzzle_state(loaded, session, session.focused_scene()).map(|(state, _)| state)
}

fn scene_puzzle_state<'a>(
    loaded: &'a LoadedGame,
    session: &'a GameSession,
    scene_name: &str,
) -> Option<(&'a puzzle_core::State, Option<&'a Level>)> {
    let scene = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == scene_name)?;
    let state = session.scene_state_for(scene_name)?;
    let puzzle = if let Some(rule) = &scene.puzzle_rule {
        if let Some(puzzle_name) = rule.target.split('.').next_back() {
            if let Some(puzzle) = state.puzzles.get(puzzle_name) {
                Some(puzzle)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        first_puzzle_component(&scene.components)
            .and_then(|puzzle_name| state.puzzles.get(puzzle_name))
    }?;
    let level = puzzle
        .level_index
        .and_then(|index| loaded.levels.get(index));
    Some((&puzzle.state, level))
}

fn first_puzzle_component(components: &[SceneComponent]) -> Option<&str> {
    for component in components {
        match component {
            SceneComponent::Frame(frame) if frame.kind == "puzzle" || frame.kind == "frame" => {
                return Some(frame.source.as_str());
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(name) = first_puzzle_component(&container.children) {
                    return Some(name);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(name) = first_puzzle_component(&conditional.children) {
                    return Some(name);
                }
                if let Some(name) = first_puzzle_component(&conditional.else_children) {
                    return Some(name);
                }
            }
            SceneComponent::For(for_view) => {
                if let Some(name) = first_puzzle_component(&for_view.children) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

fn push_scene(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) {
    out.push_str("\"scene\":");
    push_scene_object(out, loaded, state, level, resources);
}

fn push_scene_object(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) {
    out.push('{');
    push_scene_object_body(out, loaded, state, level, resources);
    out.push('}');
}

fn push_scene_object_body(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
) {
    let display_state = match materialize_display_state(loaded, state) {
        Ok(display_state) => display_state,
        Err(error) => {
            push_display_error_scene_object_body(out, loaded, state, level, resources, &error);
            return;
        }
    };
    let state = display_state.as_ref().unwrap_or(state);
    push_json_number(out, "width", state.width as u64);
    out.push(',');
    push_json_number(out, "height", state.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    push_puzzle_settings(out, loaded);
    out.push(',');
    push_puzzle_screen(out, loaded);
    out.push(',');
    out.push_str("\"resources\":");
    if let Some(resources) = resources {
        out.push('{');
        push_scene_resources_object(out, resources);
        out.push('}');
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_scene_regions(out, level);
    out.push(',');
    push_cells(out, loaded, state);
}

fn push_display_error_scene_object_body(
    out: &mut String,
    loaded: &LoadedGame,
    state: &puzzle_core::State,
    level: Option<&Level>,
    resources: Option<&puzzle_lang::SceneResources>,
    error: &TransitionError,
) {
    push_json_number(out, "width", state.width as u64);
    out.push(',');
    push_json_number(out, "height", state.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    push_puzzle_settings(out, loaded);
    out.push(',');
    push_puzzle_screen(out, loaded);
    out.push(',');
    out.push_str("\"resources\":");
    if let Some(resources) = resources {
        out.push('{');
        push_scene_resources_object(out, resources);
        out.push('}');
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_scene_regions(out, level);
    out.push(',');
    out.push_str("\"cells\":[],\"displayError\":");
    push_json_string(out, &format!("Display program failed: {error:?}"));
}

fn push_puzzle_settings(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"settings\":{");
    out.push_str("\"render\":{");
    if let Some(cell_size) = loaded.render.cell_size {
        push_json_number(out, "cellSize", u64::from(cell_size));
    }
    out.push('}');
    out.push(',');
    out.push_str("\"grid\":{");
    let occupied_cells = loaded.render.grid.occupied_cells;
    let all_cells = loaded.render.grid.all_cells;
    push_json_number(out, "visibility", u64::from(occupied_cells || all_cells));
    out.push(',');
    push_json_bool(out, "occupied_cells", occupied_cells);
    out.push(',');
    push_json_bool(out, "all_cells", all_cells);
    out.push('}');
    out.push(',');
    out.push_str("\"inputBuffer\":{");
    push_json_bool(
        out,
        "queueDuringWait",
        loaded.input_buffer.queue_during_wait,
    );
    out.push(',');
    push_json_bool(
        out,
        "fastForwardWait",
        loaded.input_buffer.fast_forward_wait,
    );
    out.push(',');
    push_json_number(out, "minWaitMs", loaded.input_buffer.min_wait_ms);
    out.push('}');
    out.push(',');
    out.push_str("\"animation\":{");
    out.push_str("\"tween\":{");
    push_json_bool(out, "enabled", loaded.animation.tween.enabled);
    out.push(',');
    push_json_number(out, "intervalMs", loaded.animation.tween.interval_ms);
    out.push('}');
    out.push('}');
    out.push('}');
}

fn materialize_display_state(
    loaded: &LoadedGame,
    state: &puzzle_core::State,
) -> Result<Option<State>, TransitionError> {
    let Some(program) = loaded.display_program.as_deref() else {
        return Ok(None);
    };
    transition_program(&loaded.game, program, state, InputId(0)).map(Some)
}

fn push_puzzle_screen(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"screen\":{");
    out.push_str("\"viewportSize\":");
    match &loaded.screen.viewport_size {
        puzzle_lang::ViewportSizeDef::Full => {
            out.push('{');
            push_json_pair(out, "kind", "full");
            out.push('}');
        }
        puzzle_lang::ViewportSizeDef::Size { width, height } => {
            out.push('{');
            push_json_pair(out, "kind", "size");
            out.push(',');
            push_json_number(out, "width", *width as u64);
            out.push(',');
            push_json_number(out, "height", *height as u64);
            out.push('}');
        }
    }
    out.push(',');
    push_json_pair(out, "viewportFocus", &loaded.screen.viewport_focus);
    out.push(',');
    out.push_str("\"viewportFocusObjects\":");
    push_viewport_focus_objects(out, loaded);
    out.push(',');
    let mode = match loaded.screen.viewport_mode {
        puzzle_lang::ViewportModeDef::Paged => "paged",
        puzzle_lang::ViewportModeDef::Centered => "centered",
    };
    push_json_pair(out, "viewportMode", mode);
    out.push('}');
}

fn push_viewport_focus_objects(out: &mut String, loaded: &LoadedGame) {
    let focus = &loaded.screen.viewport_focus;
    let mut objects = loaded
        .object_groups
        .get(focus)
        .cloned()
        .or_else(|| {
            loaded.object_groups.iter().find_map(|(name, objects)| {
                name.eq_ignore_ascii_case(focus).then(|| objects.clone())
            })
        })
        .unwrap_or_else(|| {
            loaded
                .object_labels
                .iter()
                .filter_map(|(object, name)| name.eq_ignore_ascii_case(focus).then_some(*object))
                .collect()
        });
    objects.sort_by_key(|object| object.0);
    objects.dedup();
    out.push('[');
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_scene_regions(out: &mut String, level: Option<&Level>) {
    out.push_str("\"regions\":[");
    if let Some(level) = level {
        for (index, region) in level.regions.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_number(out, "index", region.index as u64);
            out.push(',');
            push_json_number(out, "x", region.x as u64);
            out.push(',');
            push_json_number(out, "y", region.y as u64);
            out.push(',');
            push_json_number(out, "width", region.width as u64);
            out.push(',');
            push_json_number(out, "height", region.height as u64);
            out.push('}');
        }
    }
    out.push(']');
}

fn push_cells(out: &mut String, loaded: &LoadedGame, state: &puzzle_core::State) {
    out.push_str("\"cells\":[");
    let mut first_cell = true;

    for y in 0..state.height {
        for x in 0..state.width {
            if !first_cell {
                out.push(',');
            }
            first_cell = false;

            out.push('{');
            push_json_number(out, "x", x as u64);
            out.push(',');
            push_json_number(out, "y", y as u64);
            out.push(',');
            out.push_str("\"layers\":[");

            let mut first_layer = true;
            for layer in 0..state.layer_count {
                let layer_id = LayerId(layer);
                let object = state.get_layer(x, y, layer_id).unwrap_or(ObjectId::EMPTY);
                if object.is_empty() {
                    continue;
                }

                if !first_layer {
                    out.push(',');
                }
                first_layer = false;

                let object_name = loaded.object_name(object);
                out.push('{');
                push_json_number(out, "layer", layer as u64);
                out.push(',');
                push_json_number(out, "objectId", object.0 as u64);
                out.push(',');
                push_json_pair(out, "object", object_name);
                out.push(',');
                push_json_pair(out, "sprite", &sprite_name(object_name));
                out.push('}');
            }

            out.push(']');
            out.push('}');
        }
    }

    out.push(']');
}

fn sprite_name(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "unknown".to_string()
    } else {
        sprite
    }
}

fn push_inputs(out: &mut String, loaded: &LoadedGame) {
    let mut inputs = loaded
        .input_labels
        .iter()
        .map(|(id, name)| (*id, name.as_str()))
        .collect::<Vec<_>>();
    inputs.sort_by_key(|(id, _)| *id);

    out.push_str("\"inputs\":[");
    for (index, (id, name)) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", id.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push(',');
        if let Some(key) = key_for_input(loaded, *id) {
            push_json_pair(out, "key", &key);
        } else {
            out.push_str("\"key\":null");
        }
        out.push(',');
        if let Some(arrow) = arrow_for_input(loaded, *id) {
            push_json_pair(out, "arrow", &arrow);
        } else {
            out.push_str("\"arrow\":null");
        }
        out.push(',');
        out.push_str("\"keys\":[");
        for (key_index, key) in key_triggers_for_input(loaded, *id).iter().enumerate() {
            if key_index > 0 {
                out.push(',');
            }
            push_json_string(out, key);
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
}

fn push_levels(out: &mut String, loaded: &LoadedGame, cleared_levels: &[bool]) {
    out.push_str("\"levels\":[");
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", index as u64);
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
        out.push(',');
        push_json_bool(
            out,
            "cleared",
            cleared_levels.get(index).copied().unwrap_or(false),
        );
        out.push('}');
    }
    out.push(']');
}

fn push_top_scope_context(out: &mut String, loaded: &LoadedGame, has_progress_save: bool) {
    push_json_pair(out, "title", &loaded.title);
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = &loaded.subtitle {
        push_json_string(out, subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = &loaded.author {
        push_json_string(out, author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = &loaded.homepage {
        push_json_string(out, homepage);
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_json_bool(out, "has_progress_save", has_progress_save);
}

fn push_level_context(
    out: &mut String,
    loaded: &LoadedGame,
    cleared_levels: &[bool],
    level_index: Option<usize>,
) {
    let Some(level_index) = level_index else {
        out.push_str("\"level\":null");
        return;
    };
    let Some(level) = loaded.levels.get(level_index) else {
        out.push_str("\"level\":null");
        return;
    };
    out.push_str("\"level\":{");
    push_json_number(out, "index", level_index as u64);
    out.push(',');
    push_json_number(out, "number", level_index as u64 + 1);
    out.push(',');
    push_json_number(out, "count", loaded.levels.len() as u64);
    out.push(',');
    push_json_pair(out, "name", &level.name);
    out.push(',');
    push_json_pair(out, "label", &level.name);
    out.push(',');
    push_json_pair(out, "puzzle", &level.puzzle);
    out.push(',');
    if let Some(pack) = &level.pack {
        push_json_pair(out, "pack", pack);
    } else {
        out.push_str("\"pack\":null");
    }
    out.push(',');
    push_json_bool(
        out,
        "cleared",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push(',');
    push_json_bool(
        out,
        "solved",
        cleared_levels.get(level_index).copied().unwrap_or(false),
    );
    out.push('}');
}

fn push_scenes(out: &mut String, key: &str, loaded: &LoadedGame) {
    push_json_string(out, key);
    out.push_str(":[");
    for (screen_index, scene) in loaded.scenes.iter().enumerate() {
        if screen_index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &scene.name);
        out.push(',');
        push_scene_layout(out, &scene.layout);
        out.push(',');
        push_scene_resources(out, scene);
        out.push(',');
        push_scene_state_def(out, scene);
        out.push(',');
        out.push_str("\"puzzleRule\":");
        if let Some(rule) = &scene.puzzle_rule {
            out.push('{');
            push_json_pair(out, "target", &rule.target);
            out.push(',');
            push_json_pair(out, "rule", &rule.rule);
            out.push('}');
        } else {
            out.push_str("null");
        }
        out.push(',');
        out.push_str("\"components\":[");
        for (component_index, component) in scene.components.iter().enumerate() {
            if component_index > 0 {
                out.push(',');
            }
            push_scene_component(out, component);
        }
        out.push(']');
        out.push(',');
        out.push_str("\"keys\":[");
        for (binding_index, binding) in scene.key_bindings.iter().enumerate() {
            if binding_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_effect(out, &binding.effect);
            out.push(',');
            out.push_str("\"keys\":[");
            for (key_index, key) in binding.keys.iter().enumerate() {
                if key_index > 0 {
                    out.push(',');
                }
                push_json_string(out, &key_trigger_name(key));
            }
            out.push(']');
            out.push('}');
        }
        out.push(']');
        out.push(',');
        out.push_str("\"routines\":[");
        for (routine_index, routine) in scene.routines.iter().enumerate() {
            if routine_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_pair(out, "name", &routine.name);
            out.push(',');
            push_json_effect(out, &routine.effect);
            out.push('}');
        }
        out.push(']');
        out.push(',');
        out.push_str("\"transitions\":[");
        for (transition_index, transition) in scene.transitions.iter().enumerate() {
            if transition_index > 0 {
                out.push(',');
            }
            out.push('{');
            match &transition.trigger {
                SceneTransitionTrigger::Condition(condition) => {
                    push_json_expr_named(out, "condition", condition);
                }
                SceneTransitionTrigger::Signal(condition) => {
                    push_json_expr_named(out, "signal", condition);
                }
                SceneTransitionTrigger::SceneStart => {
                    push_json_pair(out, "lifecycle", "scene_start");
                }
                SceneTransitionTrigger::LevelStart => {
                    push_json_pair(out, "lifecycle", "level_start");
                }
            }
            out.push(',');
            push_json_effect(out, &transition.effect);
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
}

fn push_scene_resources(out: &mut String, scene: &puzzle_lang::SceneDef) {
    out.push_str("\"resources\":{");
    push_scene_resources_object(out, &scene.resources);
    out.push('}');
}

fn push_scene_resources_object(out: &mut String, resources: &puzzle_lang::SceneResources) {
    push_json_pair(
        out,
        "levelsMode",
        resource_selection_mode(&resources.levels),
    );
    out.push(',');
    push_resource_names(out, "levels", &resources.levels);
    out.push(',');
    push_json_pair(
        out,
        "spritesMode",
        resource_selection_mode(&resources.sprites),
    );
    out.push(',');
    push_resource_names(out, "sprites", &resources.sprites);
}

fn resource_selection_mode(selection: &ResourceSelection) -> &'static str {
    match selection {
        ResourceSelection::All => "all",
        ResourceSelection::Named(_) => "named",
    }
}

fn push_resource_names(out: &mut String, name: &str, selection: &ResourceSelection) {
    out.push('"');
    out.push_str(name);
    out.push_str("\":[");
    if let ResourceSelection::Named(names) = selection {
        for (index, value) in names.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_json_string(out, value);
        }
    }
    out.push(']');
}

fn push_scene_state_def(out: &mut String, scene: &puzzle_lang::SceneDef) {
    out.push_str("\"state\":{");
    out.push_str("\"variables\":[");
    for (index, variable) in scene.state.variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_scene_var_def(out, variable);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"puzzles\":[");
    for (index, puzzle) in scene.state.puzzles.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(out, "name", &puzzle.name);
        out.push(',');
        push_json_pair(out, "kind", &puzzle.kind);
        out.push(',');
        push_json_pair(out, "model", &puzzle.model);
        out.push(',');
        match &puzzle.initializer {
            ScenePuzzleInitializer::CurrentLevel => {
                push_json_pair(out, "initializer", "current_level")
            }
            ScenePuzzleInitializer::Level(level_name) => {
                push_json_pair(out, "initializer", "level");
                out.push(',');
                push_json_pair(out, "level", level_name);
            }
        }
        out.push('}');
    }
    out.push(']');
    out.push('}');
}

fn push_scene_var_def(out: &mut String, variable: &puzzle_lang::SceneVarDef) {
    out.push('{');
    push_json_pair(out, "name", &variable.name);
    out.push(',');
    push_json_pair(
        out,
        "kind",
        match variable.kind {
            puzzle_lang::SceneVarKind::Value => "value",
            puzzle_lang::SceneVarKind::Signal => "signal",
        },
    );
    out.push(',');
    out.push_str("\"default\":");
    push_scene_default_value(out, &variable.default);
    out.push(',');
    push_json_pair(
        out,
        "lifetime",
        match variable.lifetime {
            puzzle_lang::SceneStateLifetime::Instance => "instance",
            puzzle_lang::SceneStateLifetime::ResetOnStart => "reset_on_start",
            puzzle_lang::SceneStateLifetime::Persistent => "persistent",
        },
    );
    out.push(',');
    push_json_bool(out, "mutable", variable.mutable);
    out.push('}');
}

fn push_scene_default_value(out: &mut String, value: &SceneValue) {
    match value {
        SceneValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        SceneValue::Int(value) => out.push_str(&value.to_string()),
        SceneValue::Text(value) | SceneValue::Symbol(value) => push_json_string(out, value),
        SceneValue::LevelRef(index) => out.push_str(&index.to_string()),
    }
}

fn push_scene_layout(out: &mut String, layout: &SceneLayoutDef) {
    out.push_str("\"layout\":");
    puzzle_scene::write_scene_layout_json(out, layout);
}

fn push_scene_component(out: &mut String, component: &SceneComponent) {
    let mut note_level_source: fn(&str) = ignore_scene_level_source;
    puzzle_scene::write_scene_component_fixture_json(
        out,
        component,
        puzzle_scene::SceneFixtureJsonOptions::default(),
        push_scene_text_content_fields,
        &mut note_level_source,
    );
}

fn push_json_effect(out: &mut String, effect: &SceneEffect) {
    out.push_str("\"effect\":");
    puzzle_scene::write_scene_effect_json(out, effect);
}

fn push_json_effect_fields(out: &mut String, effect: &SceneEffect) {
    puzzle_scene::write_scene_effect_json_fields(out, effect);
}

fn push_json_expr_named(out: &mut String, name: &str, expr: &SceneExpr) {
    push_json_string(out, name);
    out.push(':');
    puzzle_scene::write_scene_expr_json(out, expr);
}

fn ignore_scene_level_source(_: &str) {}

fn push_scene_text_content_fields(out: &mut String, content: &SceneTextContent) {
    match content {
        SceneTextContent::Literal(value) => {
            out.push_str("\"source\": \"literal\", \"value\": ");
            push_json_string(out, value);
        }
        SceneTextContent::Path(path) => {
            out.push_str("\"source\": \"path\", \"path\": ");
            push_json_string(out, &path.join("."));
        }
        SceneTextContent::Expr(expr) => {
            out.push_str("\"source\": \"expr\", \"content\": ");
            puzzle_scene::write_scene_expr_json(out, expr);
        }
    }
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}

fn key_for_input(loaded: &LoadedGame, input: InputId) -> Option<String> {
    loaded
        .controls
        .keys
        .iter()
        .find_map(|(key, id)| (*id == input).then_some(char::from(*key).to_string()))
}

fn arrow_for_input(loaded: &LoadedGame, input: InputId) -> Option<String> {
    loaded
        .controls
        .arrows
        .iter()
        .find_map(|(arrow, id)| (*id == input).then_some(arrow_name(*arrow).to_string()))
}

fn key_triggers_for_input(loaded: &LoadedGame, input: InputId) -> Vec<String> {
    let mut keys = Vec::new();
    for (key, id) in &loaded.controls.keys {
        if *id == input {
            keys.push(char::from(*key).to_string());
        }
    }
    for (arrow, id) in &loaded.controls.arrows {
        if *id == input {
            keys.push(arrow_name(*arrow).to_string());
        }
    }
    for (name, id) in &loaded.controls.named {
        if *id == input {
            keys.push(name.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn arrow_name(arrow: ArrowKey) -> &'static str {
    match arrow {
        ArrowKey::Up => "ArrowUp",
        ArrowKey::Down => "ArrowDown",
        ArrowKey::Left => "ArrowLeft",
        ArrowKey::Right => "ArrowRight",
    }
}

fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_number(out: &mut String, key: &str, value: u64) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_i64(out: &mut String, key: &str, value: i64) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_bool(out: &mut String, key: &str, value: bool) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn push_json_string(out: &mut String, value: &str) {
    puzzle_scene::write_json_string(out, value);
}

fn escape_script_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn escape_script(value: &str) -> String {
    value.replace("</script", "<\\/script")
}

fn escape_style(value: &str) -> String {
    value.replace("</style", "<\\/style")
}

#[cfg(not(target_arch = "wasm32"))]
fn http_ok(content_type: &str, body: &str) -> String {
    http_response(200, "OK", content_type, body)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_error(status: u16, message: &str) -> String {
    let mut body = String::new();
    body.push_str("{\"error\":");
    push_json_string(&mut body, message);
    body.push('}');
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    http_response(status, reason, "application/json; charset=utf-8", &body)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_response(status: u16, reason: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[derive(Debug)]
enum AppError {
    #[cfg(not(target_arch = "wasm32"))]
    Io(io::Error),
    Lang(DiagnosticReport),
    CoreTransition(puzzle_core::TransitionError),
    Config(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DiagnosticReport> for AppError {
    fn from(value: DiagnosticReport) -> Self {
        Self::Lang(value)
    }
}

impl From<puzzle_core::TransitionError> for AppError {
    fn from(value: puzzle_core::TransitionError) -> Self {
        Self::CoreTransition(value)
    }
}
