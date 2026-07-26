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
    body: Vec<u8>,
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

    let body_start = end + 4;
    let body_end = body_start + content_length;
    let body = bytes
        .get(body_start..body_end)
        .ok_or_else(|| AppError::Config("incomplete HTTP request body".to_string()))?
        .to_vec();

    Ok(Some(HttpRequest { method, path, body }))
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
        ("GET", "/visual_tween_core.js") => {
            http_ok("text/javascript; charset=utf-8", VISUAL_TWEEN_CORE_JS)
        }
        ("GET", "/renderer.js") => http_ok("text/javascript; charset=utf-8", RENDERER_JS),
        ("GET", "/render_asset_decoder.js") => {
            http_ok("text/javascript; charset=utf-8", RENDER_ASSET_DECODER_JS)
        }
        ("GET", "/api/scene") => {
            let state = state.lock().expect("server state poisoned");
            http_ok("application/json; charset=utf-8", &state.scene_json())
        }
        #[cfg(feature = "solver")]
        ("POST", "/api/solve") => {
            let mut state = state.lock().expect("server state poisoned");
            match state.solve_json() {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/action") => {
            let action = match serde_json::from_slice::<puzzle_runtime_contract::SessionAction>(
                &request.body,
            ) {
                Ok(action) => action,
                Err(error) => return http_error(400, &format!("invalid session action: {error}")),
            };
            handle_session_action(state, action)
        }
        _ => http_error(404, "not found"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_session_action(
    state: Arc<Mutex<ServerState>>,
    action: puzzle_runtime_contract::SessionAction,
) -> String {
    let mut state = state.lock().expect("server state poisoned");
    match state.runtime.dispatch(action) {
        Ok(body) => http_ok("application/json; charset=utf-8", &body),
        Err(error) => http_error(400, &error),
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
        "visualsMode",
        resource_selection_mode(&resources.visuals),
    );
    out.push(',');
    push_resource_names(out, "visuals", &resources.visuals);
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
