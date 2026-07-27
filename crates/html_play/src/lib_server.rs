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

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);
        if find_header_end(&bytes).is_some() {
            break;
        }
    }

    let Some(end) = find_header_end(&bytes) else {
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

fn visual_name(object_name: &str) -> String {
    let mut visual = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            visual.push(ch);
        } else if !visual.ends_with('-') {
            visual.push('-');
        }
    }
    let visual = visual.trim_matches('-').to_string();
    if visual.is_empty() {
        "unknown".to_string()
    } else {
        visual
    }
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
