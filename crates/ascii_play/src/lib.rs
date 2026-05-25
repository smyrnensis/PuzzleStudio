use std::collections::HashMap;
use std::env;
use std::fmt;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::Command;

use puzzle_lang::{
    ArrowKey, ForSource, KeyTrigger, LoadedGame, MenuComponent, SceneComponent, SceneEffect,
    SceneExpr, SceneTextContent, SceneValue, discover_game_entries,
    parse_game2d_file as parse_game_file, resolve_game_entry,
};
use puzzle_play::{GameSession, render_ascii_top};

pub fn run_terminal_from_env() -> Result<(), AppError> {
    let path = select_puzzle_path()?;
    let loaded = parse_game_file(&path)?;
    print_warnings(&loaded);

    let _terminal_mode = TerminalMode::enter();
    let mut stdin = io::stdin();
    let mut session = GameSession::new(&loaded);

    loop {
        clear_screen();
        print!("{}", render_terminal(&loaded, &session));
        io::stdout().flush()?;

        let Some(key) = read_key(&mut stdin) else {
            break;
        };
        match key {
            TerminalKey::Interrupt => break,
            TerminalKey::Ignored => {}
            _ => {
                if let Some(command) = command_for_key(&loaded, &session, key) {
                    session
                        .apply_command(&loaded, &command)
                        .map_err(|error| AppError::Runtime(format!("{error:?}")))?;
                }
            }
        }
    }

    clear_screen();
    Ok(())
}

fn print_warnings(loaded: &LoadedGame) {
    for warning in &loaded.warnings {
        eprintln!("warning: {warning}");
    }
}

fn render_terminal(loaded: &LoadedGame, session: &GameSession) -> String {
    let mut out = String::new();
    out.push_str(&loaded.title);
    out.push('\n');
    out.push_str(&format!(
        "level {}/{}: {}\n",
        session.level_index() + 1,
        loaded.levels.len(),
        session.current_level(loaded).name
    ));
    out.push_str(&format!(
        "scene: {} | undo {} | redo {}\n",
        session.focused_scene(),
        yes_no(session.can_undo()),
        yes_no(session.can_redo())
    ));
    if let Some(goal) = &loaded.goal {
        out.push_str(&format!("goal: {}\n", goal.description));
    }
    out.push_str(
        "controls: scene keys, configured input keys/arrows, z undo, y redo, Ctrl-C exit\n\n",
    );

    for scene_name in session.visible_scenes() {
        if let Some(screen) = loaded
            .scenes
            .iter()
            .find(|screen| &screen.name == scene_name)
        {
            let focused = screen.name == session.focused_scene();
            render_scene(loaded, session, screen, focused, &mut out);
        }
    }

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn render_scene(
    loaded: &LoadedGame,
    session: &GameSession,
    screen: &puzzle_lang::SceneDef,
    focused: bool,
    out: &mut String,
) {
    if !session.visible_scenes().is_empty() {
        out.push_str(if focused { "== " } else { "-- " });
        out.push_str(&screen.name);
        out.push_str(if focused { " ==\n" } else { " --\n" });
    }

    let scope = RenderScope::default();
    render_components(loaded, session, &screen.components, &scope, out);
}

fn render_components(
    loaded: &LoadedGame,
    session: &GameSession,
    components: &[SceneComponent],
    scope: &RenderScope,
    out: &mut String,
) {
    for component in components {
        render_component(loaded, session, component, scope, out);
    }
}

fn render_component(
    loaded: &LoadedGame,
    session: &GameSession,
    component: &SceneComponent,
    scope: &RenderScope,
    out: &mut String,
) {
    match component {
        SceneComponent::PuzzleState(name) => {
            if let Some(state) = session
                .scene_state()
                .and_then(|scene_state| scene_state.puzzles.get(name))
            {
                out.push_str(&render_ascii_top(state, &loaded.legend));
                out.push('\n');
            }
        }
        SceneComponent::Title(title) => {
            out.push_str(&eval_expr(loaded, session, &title.content, scope));
            out.push('\n');
        }
        SceneComponent::Subtitle(subtitle) => {
            out.push_str(&eval_expr(loaded, session, &subtitle.content, scope));
            out.push('\n');
        }
        SceneComponent::Text(text) => {
            out.push_str(&eval_text(loaded, session, &text.content, scope));
            out.push('\n');
        }
        SceneComponent::Button(button) => {
            let marker = scope
                .level_index
                .is_some_and(|index| index == session.selected_level_index())
                .then_some("> ")
                .unwrap_or("  ");
            out.push_str(marker);
            out.push('[');
            out.push_str(&eval_expr(loaded, session, &button.label, scope));
            out.push_str("]\n");
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            render_components(loaded, session, &container.children, scope, out);
        }
        SceneComponent::For(for_view) => match &for_view.source {
            ForSource::Levels => {
                for index in 0..loaded.levels.len() {
                    let child_scope = scope.with_level(&for_view.binding, index);
                    render_components(loaded, session, &for_view.children, &child_scope, out);
                }
            }
            ForSource::State(_) => {}
        },
        SceneComponent::LevelMenu(menu) => {
            for (index, level) in loaded.levels.iter().enumerate() {
                let selected = index == session.selected_level_index();
                out.push_str(if selected { "> " } else { "  " });
                if menu.show_index {
                    out.push_str(&format!("{}. ", index + 1));
                }
                out.push_str(&level.name);
                out.push('\n');
            }
        }
        SceneComponent::Menu(instance) => {
            if let Some(menu) = loaded.menus.iter().find(|menu| menu.name == instance.menu) {
                let mut scope = scope.clone();
                scope.menu_instance = Some(instance.name.clone());
                scope.menu_cursor = menu_cursor(session, &instance.name);
                let mut button_index = 0;
                render_menu_components(loaded, session, &menu.view, &scope, &mut button_index, out);
            }
        }
    }
}

fn render_menu_components(
    loaded: &LoadedGame,
    session: &GameSession,
    components: &[MenuComponent],
    scope: &RenderScope,
    button_index: &mut usize,
    out: &mut String,
) {
    for component in components {
        match component {
            MenuComponent::Text(text) => {
                out.push_str(&eval_text(loaded, session, &text.content, scope));
                out.push('\n');
            }
            MenuComponent::Button(button) => {
                let selected = *button_index == scope.menu_cursor;
                *button_index += 1;
                out.push_str(if selected { "> [" } else { "  [" });
                out.push_str(&eval_expr(loaded, session, &button.label, scope));
                out.push_str("]\n");
            }
            MenuComponent::Row(container)
            | MenuComponent::Column(container)
            | MenuComponent::Box(container) => {
                render_menu_components(
                    loaded,
                    session,
                    &container.children,
                    scope,
                    button_index,
                    out,
                );
            }
            MenuComponent::For(for_view) => {
                if for_view.source.is_levels() {
                    for index in 0..loaded.levels.len() {
                        let child_scope = scope.with_level(&for_view.binding, index);
                        render_menu_components(
                            loaded,
                            session,
                            &for_view.children,
                            &child_scope,
                            button_index,
                            out,
                        );
                    }
                }
            }
        }
    }
}

fn eval_text(
    loaded: &LoadedGame,
    session: &GameSession,
    content: &SceneTextContent,
    scope: &RenderScope,
) -> String {
    match content {
        SceneTextContent::Literal(value) => value.clone(),
        SceneTextContent::Path(path) => resolve_path(loaded, session, path, scope),
    }
}

fn eval_expr(
    loaded: &LoadedGame,
    session: &GameSession,
    expr: &SceneExpr,
    scope: &RenderScope,
) -> String {
    match expr {
        SceneExpr::Bool(value) => value.to_string(),
        SceneExpr::Int(value) => value.to_string(),
        SceneExpr::Text(value) => value.clone(),
        SceneExpr::Path(path) => resolve_path(loaded, session, path, scope),
        SceneExpr::Call { name, args } => {
            let args = args
                .iter()
                .map(|arg| expr_source(loaded, session, arg, scope))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({args})")
        }
    }
}

fn resolve_path(
    loaded: &LoadedGame,
    session: &GameSession,
    path: &[String],
    scope: &RenderScope,
) -> String {
    match path {
        [name] if name == "game" => loaded.title.clone(),
        [name] if name == "level" => session.level_index().to_string(),
        [name] => scope
            .level_value(name, loaded)
            .or_else(|| scene_value(session, name))
            .unwrap_or_else(|| name.clone()),
        [root, field] if root == "game" && field == "title" => loaded.title.clone(),
        [root, field] if root == "game" && field == "subtitle" => {
            loaded.subtitle.clone().unwrap_or_default()
        }
        [root, field] if root == "game" && field == "author" => {
            loaded.author.clone().unwrap_or_default()
        }
        [root, field] if root == "game" && field == "homepage" => {
            loaded.homepage.clone().unwrap_or_default()
        }
        [root, field] if root == "level" => current_level_field(loaded, session, field),
        [root, field] => scope
            .level_field(root, field, loaded)
            .or_else(|| scene_value(session, root))
            .unwrap_or_else(|| path.join(".")),
        _ => path.join("."),
    }
}

fn current_level_field(loaded: &LoadedGame, session: &GameSession, field: &str) -> String {
    match field {
        "index" => session.level_index().to_string(),
        "number" => (session.level_index() + 1).to_string(),
        "name" | "label" => session.current_level(loaded).name.clone(),
        _ => String::new(),
    }
}

fn scene_value(session: &GameSession, name: &str) -> Option<String> {
    session
        .scene_state()
        .and_then(|state| state.values.get(name))
        .map(scene_value_to_string)
}

fn scene_value_to_string(value: &SceneValue) -> String {
    match value {
        SceneValue::Bool(value) => value.to_string(),
        SceneValue::Int(value) => value.to_string(),
        SceneValue::Text(value) | SceneValue::Symbol(value) => value.clone(),
    }
}

fn menu_cursor(session: &GameSession, instance: &str) -> usize {
    session
        .scene_state()
        .and_then(|state| state.values.get(&format!("__menu_{instance}_cursor")))
        .and_then(|value| match value {
            SceneValue::Int(value) => usize::try_from(*value).ok(),
            _ => None,
        })
        .unwrap_or(0)
}

#[derive(Clone, Debug, Default)]
struct RenderScope {
    levels: HashMap<String, usize>,
    level_index: Option<usize>,
    menu_instance: Option<String>,
    menu_cursor: usize,
}

impl RenderScope {
    fn with_level(&self, binding: &str, index: usize) -> Self {
        let mut next = self.clone();
        next.levels.insert(binding.to_string(), index);
        next.level_index = Some(index);
        next
    }

    fn level_value(&self, binding: &str, loaded: &LoadedGame) -> Option<String> {
        let index = *self.levels.get(binding)?;
        loaded.levels.get(index).map(|_| index.to_string())
    }

    fn level_field(&self, binding: &str, field: &str, loaded: &LoadedGame) -> Option<String> {
        let index = *self.levels.get(binding)?;
        let level = loaded.levels.get(index)?;
        match field {
            "index" => Some(index.to_string()),
            "number" => Some((index + 1).to_string()),
            "name" | "label" => Some(level.name.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TerminalKey {
    Char(char),
    Named(&'static str),
    Arrow(ArrowKey),
    Interrupt,
    Ignored,
}

fn read_key(stdin: &mut io::Stdin) -> Option<TerminalKey> {
    let interactive = io::stdin().is_terminal();
    loop {
        let mut byte = [0_u8; 1];
        match stdin.read(&mut byte).ok()? {
            0 if interactive => continue,
            0 => return None,
            _ => {
                return Some(match byte[0] {
                    0x03 | 0x04 => TerminalKey::Interrupt,
                    b'\r' | b'\n' => TerminalKey::Named("Enter"),
                    b' ' => TerminalKey::Named("Space"),
                    0x1b => read_escape_key(stdin),
                    ch if ch.is_ascii() => TerminalKey::Char(char::from(ch).to_ascii_lowercase()),
                    _ => TerminalKey::Ignored,
                });
            }
        }
    }
}

fn read_escape_key(stdin: &mut io::Stdin) -> TerminalKey {
    let Some(first) = read_optional_byte(stdin) else {
        return TerminalKey::Named("Escape");
    };
    if first != b'[' {
        return TerminalKey::Ignored;
    }

    match read_optional_byte(stdin) {
        Some(b'A') => TerminalKey::Arrow(ArrowKey::Up),
        Some(b'B') => TerminalKey::Arrow(ArrowKey::Down),
        Some(b'C') => TerminalKey::Arrow(ArrowKey::Right),
        Some(b'D') => TerminalKey::Arrow(ArrowKey::Left),
        _ => TerminalKey::Ignored,
    }
}

fn read_optional_byte(stdin: &mut io::Stdin) -> Option<u8> {
    let mut byte = [0_u8; 1];
    match stdin.read(&mut byte).ok()? {
        0 => None,
        _ => Some(byte[0]),
    }
}

fn command_for_key(loaded: &LoadedGame, session: &GameSession, key: TerminalKey) -> Option<String> {
    let screen = loaded
        .scenes
        .iter()
        .find(|screen| screen.name == session.focused_scene())?;

    for binding in &screen.key_bindings {
        if binding
            .keys
            .iter()
            .any(|trigger| key_matches(&key, trigger))
        {
            return effect_to_command(loaded, session, &binding.effect, &RenderScope::default());
        }
    }

    if let Some(menu) = first_menu_instance(&screen.components) {
        if let Some(input) = menu_input_for_key(&key) {
            return Some(format!("{}.{}", menu.name, input));
        }
    }

    match key {
        TerminalKey::Char('z') => return Some("undo".to_string()),
        TerminalKey::Char('y') => return Some("redo".to_string()),
        _ => {}
    }

    input_command_for_key(loaded, key)
}

fn key_matches(key: &TerminalKey, trigger: &KeyTrigger) -> bool {
    match (key, trigger) {
        (TerminalKey::Char(actual), KeyTrigger::Char(expected)) => {
            *actual == expected.to_ascii_lowercase()
        }
        (TerminalKey::Named(actual), KeyTrigger::Named(expected)) => *actual == expected,
        (TerminalKey::Arrow(actual), KeyTrigger::Named(expected)) => {
            arrow_key_name(*actual) == expected
        }
        _ => false,
    }
}

fn arrow_key_name(key: ArrowKey) -> &'static str {
    match key {
        ArrowKey::Up => "ArrowUp",
        ArrowKey::Down => "ArrowDown",
        ArrowKey::Left => "ArrowLeft",
        ArrowKey::Right => "ArrowRight",
    }
}

fn input_command_for_key(loaded: &LoadedGame, key: TerminalKey) -> Option<String> {
    let input = match key {
        TerminalKey::Char(ch) if ch.is_ascii() => loaded.controls.keys.get(&(ch as u8)).copied(),
        TerminalKey::Arrow(arrow) => loaded.controls.arrows.get(&arrow).copied(),
        _ => None,
    }?;
    loaded.input_labels.get(&input).cloned()
}

fn menu_input_for_key(key: &TerminalKey) -> Option<&'static str> {
    match key {
        TerminalKey::Char('w') | TerminalKey::Arrow(ArrowKey::Up) => Some("up"),
        TerminalKey::Char('s') | TerminalKey::Arrow(ArrowKey::Down) => Some("down"),
        TerminalKey::Char('a') | TerminalKey::Arrow(ArrowKey::Left) => Some("left"),
        TerminalKey::Char('d') | TerminalKey::Arrow(ArrowKey::Right) => Some("right"),
        TerminalKey::Named("Enter") | TerminalKey::Named("Space") => Some("enter"),
        TerminalKey::Named("Escape") | TerminalKey::Char('q') => Some("back"),
        _ => None,
    }
}

fn first_menu_instance(components: &[SceneComponent]) -> Option<&puzzle_lang::MenuInstanceDef> {
    for component in components {
        match component {
            SceneComponent::Menu(instance) => return Some(instance),
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(instance) = first_menu_instance(&container.children) {
                    return Some(instance);
                }
            }
            SceneComponent::For(for_view) => {
                if let Some(instance) = first_menu_instance(&for_view.children) {
                    return Some(instance);
                }
            }
            _ => {}
        }
    }
    None
}

fn effect_to_command(
    loaded: &LoadedGame,
    session: &GameSession,
    effect: &SceneEffect,
    scope: &RenderScope,
) -> Option<String> {
    match effect {
        SceneEffect::Input(input) | SceneEffect::ComponentEffect(input) => {
            Some(command_with_scope(loaded, session, input, scope))
        }
        SceneEffect::Message { text } => Some(format!(
            "message {}",
            expr_source(loaded, session, text, scope)
        )),
        SceneEffect::Wait { .. } => None,
        SceneEffect::Conditional { .. } => None,
        SceneEffect::PlaySfx { name } => Some(format!("play_sfx {name}")),
        SceneEffect::PlayMusic { name } => Some(format!("play_music {name}")),
        SceneEffect::PauseMusic { name } => name
            .as_ref()
            .map(|name| format!("pause_music {name}"))
            .or_else(|| Some("pause_music".to_string())),
        SceneEffect::ResumeMusic { name } => name
            .as_ref()
            .map(|name| format!("resume_music {name}"))
            .or_else(|| Some("resume_music".to_string())),
        SceneEffect::StopMusic { name } => name
            .as_ref()
            .map(|name| format!("stop_music {name}"))
            .or_else(|| Some("stop_music".to_string())),
        SceneEffect::Goto { scene, params } => {
            Some(scene_command(loaded, session, "goto", scene, params, scope))
        }
        SceneEffect::Enter { scene, params } => Some(scene_command(
            loaded, session, "enter", scene, params, scope,
        )),
        SceneEffect::Back => Some("back".to_string()),
        SceneEffect::Create { scene } => Some(format!("create {scene}")),
        SceneEffect::Reset { scene } => Some(format!("reset {scene}")),
        SceneEffect::Delete { scene } => Some(format!("delete {scene}")),
        SceneEffect::Show { scene } => Some(format!("show {scene}")),
        SceneEffect::Hide { scene } => Some(format!("hide {scene}")),
        SceneEffect::Toggle { scene } => Some(format!("toggle {scene}")),
        SceneEffect::Focus { scene } => Some(format!("focus {scene}")),
        SceneEffect::StartLevel { scene, scope } => scope
            .as_ref()
            .map(|scope| format!("start levels {scope} in {scene}"))
            .or_else(|| Some(format!("start levels in {scene}"))),
        SceneEffect::ContinueLevel { scene, scope } => scope
            .as_ref()
            .map(|scope| format!("continue levels {scope} in {scene}"))
            .or_else(|| Some(format!("continue levels in {scene}"))),
        SceneEffect::PuzzleNextLevel { target } => Some(format!("{target}.next_level")),
        SceneEffect::PuzzlePreviousLevel { target } => Some(format!("{target}.previous_level")),
        SceneEffect::GotoLevel { target, level } => Some(format!(
            "{target}.goto {}",
            expr_source(loaded, session, level, scope)
        )),
        SceneEffect::ResetPuzzle { target } => Some(format!("reset {target}")),
        SceneEffect::LoadPuzzle { target, source } => Some(format!("load {target} from {source}")),
        SceneEffect::Copy { source, target } => Some(format!("copy {source} to {target}")),
        SceneEffect::ClearHistory => Some("clear_history".to_string()),
        SceneEffect::Apply { args, .. } => args
            .first()
            .map(|arg| eval_expr(loaded, session, arg, scope))
            .filter(|value| !value.is_empty()),
        SceneEffect::Sequence(_) => None,
    }
}

fn scene_command(
    loaded: &LoadedGame,
    session: &GameSession,
    command: &str,
    screen: &str,
    params: &[puzzle_lang::SceneEffectParam],
    scope: &RenderScope,
) -> String {
    if params.is_empty() {
        return format!("{command} {screen}");
    }

    let params = params
        .iter()
        .map(|param| {
            format!(
                "{} = {}",
                param.name,
                expr_source(loaded, session, &param.value, scope)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{command} {screen} with {params}")
}

fn command_with_scope(
    loaded: &LoadedGame,
    session: &GameSession,
    command: &str,
    scope: &RenderScope,
) -> String {
    let Some((name, payload)) = command.split_once(':') else {
        return command.to_string();
    };
    let path = payload
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let value = resolve_path(loaded, session, &path, scope);
    if value.is_empty() {
        command.to_string()
    } else {
        format!("{name}:{value}")
    }
}

fn expr_source(
    loaded: &LoadedGame,
    session: &GameSession,
    expr: &SceneExpr,
    scope: &RenderScope,
) -> String {
    match expr {
        SceneExpr::Bool(value) => value.to_string(),
        SceneExpr::Int(value) => value.to_string(),
        SceneExpr::Text(value) => value.clone(),
        SceneExpr::Path(path) => resolve_path(loaded, session, path, scope),
        SceneExpr::Call { name, args } => {
            let args = args
                .iter()
                .map(|arg| expr_source(loaded, session, arg, scope))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({args})")
        }
    }
}

fn select_puzzle_path() -> Result<PathBuf, AppError> {
    if let Some(path) = env::args().nth(1) {
        return resolve_game_entry(PathBuf::from(path))
            .map_err(|error| AppError::Config(error.to_string()));
    }

    let candidates =
        discover_game_entries("games").map_err(|error| AppError::Config(error.to_string()))?;
    match candidates.len() {
        0 => Err(AppError::Config(
            "no games/*/game.puzzle entries found. Pass a path: ascii-play <path/to/game-folder-or-game.puzzle>"
                .to_string(),
        )),
        1 => Ok(candidates[0].clone()),
        _ => prompt_puzzle_choice(&candidates),
    }
}

fn prompt_puzzle_choice(candidates: &[PathBuf]) -> Result<PathBuf, AppError> {
    println!("Select a puzzle:");
    for (index, path) in candidates.iter().enumerate() {
        println!("  {}. {}", index + 1, path.display());
    }
    print!("number: ");
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let choice = line
        .trim()
        .parse::<usize>()
        .map_err(|_| AppError::Config("expected a puzzle number".to_string()))?;

    candidates
        .get(choice.saturating_sub(1))
        .cloned()
        .ok_or_else(|| AppError::Config(format!("puzzle number out of range: {choice}")))
}

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

struct TerminalMode {
    saved: Option<String>,
}

impl TerminalMode {
    fn enter() -> Self {
        if !io::stdin().is_terminal() {
            return Self { saved: None };
        }
        let saved = Command::new("stty")
            .arg("-g")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
        let _ = Command::new("stty")
            .args(["-icanon", "-echo", "-isig", "min", "0", "time", "1"])
            .status();
        Self { saved }
    }
}

impl Drop for TerminalMode {
    fn drop(&mut self) {
        if !io::stdin().is_terminal() {
            return;
        }
        if let Some(saved) = &self.saved {
            let _ = Command::new("stty").arg(saved).status();
        } else {
            let _ = Command::new("stty").arg("sane").status();
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Lang(puzzle_lang::AppError),
    Runtime(String),
    Config(String),
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<puzzle_lang::AppError> for AppError {
    fn from(value: puzzle_lang::AppError) -> Self {
        Self::Lang(value)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Lang(error) => write!(f, "{error}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}
