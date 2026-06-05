use std::collections::HashMap;
use std::env;
use std::fmt;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::Command;

use puzzle_core::{InputId, State as PuzzleState, transition_program};
use puzzle_lang::{
    ArrowKey, ForSource, KeyTrigger, LoadedDocumentModel, LoadedGame, ResourceSelection,
    SceneComponent, SceneEffect, SceneExpr, SceneTextContent, SceneValue, VisualSpriteKind,
    discover_game_entries, parse_game_file, resolve_game_entry,
};
use puzzle_play::{GameSession, MessageEvent, SoundEvent, WaitEvent, cell_objects};
use puzzle3d_model::{
    Coord3, GameSession3, GameSessionError3, InputDef3, ObjectId as ObjectId3, ParsedPuzzle3,
    State3,
};

pub fn run_terminal_from_env() -> Result<(), AppError> {
    run_terminal_from_args(env::args().skip(1))
}

pub fn run_terminal_from_args(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    let path = select_puzzle_path(args)?;
    run_terminal_from_path(path)
}

pub fn run_terminal_from_path(path: impl Into<PathBuf>) -> Result<(), AppError> {
    let path =
        resolve_game_entry(path.into()).map_err(|error| AppError::Config(error.to_string()))?;
    let document = parse_game_file(&path)?;
    match document.single_model() {
        Some(LoadedDocumentModel::Puzzle2d { game, .. }) => run_terminal_2d(game),
        Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) => {
            run_terminal_3d(&document.title, puzzle)
        }
        None => Err(AppError::Config(
            "ascii-play requires a document with exactly one model".to_string(),
        )),
    }
}

fn run_terminal_2d(loaded: &LoadedGame) -> Result<(), AppError> {
    print_warnings(&loaded);

    let _terminal_mode = TerminalMode::enter();
    let mut stdin = io::stdin();
    let mut session = GameSession::new(&loaded);
    let mut events = TerminalEvents::take_from(&mut session);
    let mut ui_state = TerminalUiState::default();

    loop {
        clear_screen();
        print!("{}", render_terminal(&loaded, &session, &events, &ui_state));
        io::stdout().flush()?;

        let Some(key) = read_key(&mut stdin) else {
            break;
        };
        match key {
            TerminalKey::Interrupt => break,
            TerminalKey::Ignored => {}
            _ => {
                if let Some(command) = command_for_key(&loaded, &session, &mut ui_state, key) {
                    session
                        .apply_command(&loaded, &command)
                        .map_err(|error| AppError::Runtime(format!("{error:?}")))?;
                    events = TerminalEvents::take_from(&mut session);
                }
            }
        }
    }

    clear_screen();
    Ok(())
}

fn run_terminal_3d(title: &str, parsed: &ParsedPuzzle3) -> Result<(), AppError> {
    let _terminal_mode = TerminalMode::enter();
    let mut stdin = io::stdin();
    let mut session = Puzzle3TerminalSession::new(parsed)?;
    let mut ui_state = TerminalUiState::default();

    loop {
        clear_screen();
        print!("{}", render_terminal_3d(title, parsed, &session, &ui_state));
        io::stdout().flush()?;

        let Some(key) = read_key(&mut stdin) else {
            break;
        };
        if session.apply_key(parsed, &mut ui_state, key)? {
            break;
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

fn render_terminal(
    loaded: &LoadedGame,
    session: &GameSession,
    events: &TerminalEvents,
    ui_state: &TerminalUiState,
) -> String {
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
            render_scene(loaded, session, screen, focused, ui_state, &mut out);
        }
    }
    events.render(&mut out);

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn render_terminal_3d(
    title: &str,
    parsed: &ParsedPuzzle3,
    session: &Puzzle3TerminalSession,
    ui_state: &TerminalUiState,
) -> String {
    let mut out = String::new();
    out.push_str(title);
    out.push('\n');
    if let Some(bundle) = &parsed.level_bundle {
        let level_index = session.session.current_level_index();
        let level_name = bundle
            .level(level_index)
            .map(|level| level.name.as_str())
            .unwrap_or("unknown");
        out.push_str(&format!(
            "level {}/{}: {}\n",
            level_index + 1,
            bundle.level_count(),
            level_name
        ));
    }
    out.push_str(&format!(
        "scene: {} | undo {} | complete {}\n",
        session.current_scene,
        yes_no(session.session.can_undo()),
        yes_no(session.session.completed())
    ));
    out.push_str(
        "controls: scene keys, wasd/arrows horizontal, e up, c down, z undo, r restart, Ctrl-C exit\n\n",
    );

    let _ = ui_state;
    out.push_str(&render_puzzle3_ascii_top_down(
        parsed,
        session.session.state(),
    ));
    out.push('\n');

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
    ui_state: &TerminalUiState,
    out: &mut String,
) {
    if !session.visible_scenes().is_empty() {
        out.push_str(if focused { "== " } else { "-- " });
        out.push_str(&screen.name);
        out.push_str(if focused { " ==\n" } else { " --\n" });
    }

    let scope = RenderScope::for_scene(&screen.name);
    let mut button_index = 0;
    render_components(
        loaded,
        session,
        &screen.components,
        &scope,
        ui_state,
        &mut button_index,
        out,
    );
}

fn render_components(
    loaded: &LoadedGame,
    session: &GameSession,
    components: &[SceneComponent],
    scope: &RenderScope,
    ui_state: &TerminalUiState,
    button_index: &mut usize,
    out: &mut String,
) {
    for component in components {
        render_component(
            loaded,
            session,
            component,
            scope,
            ui_state,
            button_index,
            out,
        );
    }
}

fn render_component(
    loaded: &LoadedGame,
    session: &GameSession,
    component: &SceneComponent,
    scope: &RenderScope,
    ui_state: &TerminalUiState,
    button_index: &mut usize,
    out: &mut String,
) {
    match component {
        SceneComponent::Frame(frame) => {
            if let Some(state) = session
                .scene_state_for(&scope.scene_name)
                .and_then(|scene_state| scene_state.puzzles.get(&frame.source))
            {
                out.push_str(&render_colored_ascii_top(loaded, state));
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
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            let current_button = *button_index;
            *button_index += 1;
            let marker = if current_button == ui_state.button_cursor(&scope.scene_name) {
                "> "
            } else {
                "  "
            };
            out.push_str(marker);
            out.push('[');
            out.push_str(&eval_expr(loaded, session, &button.label, scope));
            out.push_str("]\n");
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            render_components(
                loaded,
                session,
                &container.children,
                scope,
                ui_state,
                button_index,
                out,
            );
        }
        SceneComponent::Conditional(conditional) => {
            render_components(
                loaded,
                session,
                &conditional.children,
                scope,
                ui_state,
                button_index,
                out,
            );
        }
        SceneComponent::For(for_view) => match &for_view.source {
            ForSource::Levels => {
                for index in scene_level_indices(loaded, &scope.scene_name) {
                    let child_scope = scope.with_level(&for_view.binding, index);
                    render_components(
                        loaded,
                        session,
                        &for_view.children,
                        &child_scope,
                        ui_state,
                        button_index,
                        out,
                    );
                }
            }
            ForSource::State(_) => {}
        },
        SceneComponent::LevelMenu(menu) => {
            let level_indices = scene_level_indices(loaded, &scope.scene_name);
            let cursor = level_menu_cursor_position(loaded, session, &level_indices);
            for (position, index) in level_indices.iter().copied().enumerate() {
                let Some(level) = loaded.levels.get(index) else {
                    continue;
                };
                let selected = position == cursor;
                out.push_str(if selected { "> " } else { "  " });
                if menu.show_index {
                    out.push_str(&format!("{}. ", position + 1));
                }
                if menu.show_cleared {
                    let marker = if session
                        .cleared_levels()
                        .get(index)
                        .copied()
                        .unwrap_or(false)
                    {
                        "* "
                    } else {
                        "  "
                    };
                    out.push_str(marker);
                }
                out.push_str(&level.name);
                out.push('\n');
            }
        }
    }
}

fn render_colored_ascii_top(
    loaded: &LoadedGame,
    state: &puzzle_play::ScenePuzzleRuntimeState,
) -> String {
    let color_table = AsciiSpriteColorTable::from_loaded(loaded);
    let display_state = ascii_display_state(loaded, state);
    let mut out = String::new();

    for y in 0..display_state.height {
        for x in 0..display_state.width {
            let cell = cell_objects(&display_state, x, y);
            let ch = loaded.legend.char_for_cell(&cell);
            let color_objects = loaded.legend.legended_objects_for_cell(&cell);
            if let Some(color) = color_table.composited_color_for_object_names(
                color_objects
                    .iter()
                    .map(|object| loaded.object_name(*object)),
            ) {
                out.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    color.r, color.g, color.b, ch
                ));
            } else {
                out.push(ch);
            }
            out.push(' ');
        }
        if y + 1 < state.height {
            out.push('\n');
        }
    }

    out
}

fn ascii_display_state(
    loaded: &LoadedGame,
    state: &puzzle_play::ScenePuzzleRuntimeState,
) -> PuzzleState {
    let Some(program) = &loaded.display_program else {
        return state.state.clone();
    };
    transition_program(&loaded.game, program, &state.state, InputId(0))
        .unwrap_or_else(|_| state.state.clone())
}

#[derive(Clone, Debug)]
struct Puzzle3TerminalSession {
    current_scene: String,
    selected_level_index: usize,
    cleared_levels: Vec<bool>,
    session: GameSession3,
}

impl Puzzle3TerminalSession {
    fn new(parsed: &ParsedPuzzle3) -> Result<Self, AppError> {
        let bundle = parsed
            .level_bundle
            .as_ref()
            .ok_or_else(|| AppError::Config("3D ascii play requires levels3".to_string()))?;
        let mut session = GameSession3::new_with_lifecycle(bundle, &parsed.lifecycle)
            .map_err(|error| AppError::Runtime(format!("{error:?}")))?;
        if let Some(win_condition) = &parsed.win_condition {
            session.refresh_completed(bundle, win_condition);
        }
        Ok(Self {
            current_scene: "playing".to_string(),
            selected_level_index: 0,
            cleared_levels: vec![false; bundle.level_count()],
            session,
        })
    }

    fn apply_key(
        &mut self,
        parsed: &ParsedPuzzle3,
        ui_state: &mut TerminalUiState,
        key: TerminalKey,
    ) -> Result<bool, AppError> {
        if key == TerminalKey::Interrupt {
            return Ok(true);
        }

        let _ = ui_state;

        match key {
            TerminalKey::Char('z') => {
                self.session.undo();
                Ok(false)
            }
            TerminalKey::Char('r') => {
                let bundle = parsed.level_bundle.as_ref().ok_or_else(|| {
                    AppError::Config("3D ascii play requires levels3".to_string())
                })?;
                self.session
                    .restart_with_lifecycle(bundle, &parsed.lifecycle)
                    .map_err(AppError::from)?;
                Ok(false)
            }
            _ => {
                if let Some(input) = input3_for_key(parsed, &key) {
                    self.apply_input(parsed, input.id)?;
                }
                Ok(false)
            }
        }
    }

    fn apply_input(
        &mut self,
        parsed: &ParsedPuzzle3,
        input: puzzle3d_model::InputId3,
    ) -> Result<(), AppError> {
        let bundle = parsed
            .level_bundle
            .as_ref()
            .ok_or_else(|| AppError::Config("3D ascii play requires levels3".to_string()))?;
        let result = if let Some(win_condition) = &parsed.win_condition {
            self.session
                .apply_input_with_lifecycle(
                    bundle,
                    &parsed.rules,
                    input,
                    win_condition,
                    &parsed.lifecycle,
                )
                .map_err(AppError::from)?
        } else {
            let changed = self
                .session
                .apply_input(bundle, &parsed.rules, input)
                .map_err(AppError::from)?;
            puzzle3d_model::SessionLifecycleResult3 {
                changed,
                cleared: false,
                level_changed: false,
            }
        };
        if result.cleared {
            let index = if result.level_changed {
                self.session.current_level_index().saturating_sub(1)
            } else {
                self.session.current_level_index()
            };
            if let Some(cleared) = self.cleared_levels.get_mut(index) {
                *cleared = true;
            }
            self.selected_level_index = self.session.current_level_index();
        }
        Ok(())
    }
}

fn render_puzzle3_ascii_top_down(parsed: &ParsedPuzzle3, state: &State3) -> String {
    let mut out = String::new();
    for z in (0..state.size.height).rev() {
        out.push_str(&format!("z {z}\n"));
        for y in (0..state.size.depth).rev() {
            for x in 0..state.size.width {
                let ch = state
                    .cell_view(Coord3::new(x, y, z))
                    .ok()
                    .and_then(|cell| {
                        cell.objects
                            .iter()
                            .rev()
                            .copied()
                            .find(|object| !object.is_empty())
                    })
                    .map(|object| object3_ascii_char(parsed, object))
                    .unwrap_or('.');
                out.push(ch);
                out.push(' ');
            }
            if y > 0 {
                out.push('\n');
            }
        }
        if z > 0 {
            out.push_str("\n\n");
        }
    }
    out
}

fn object3_ascii_char(parsed: &ParsedPuzzle3, object: ObjectId3) -> char {
    parsed
        .catalog
        .objects
        .iter()
        .find(|entry| entry.id == object)
        .and_then(|entry| {
            entry
                .name
                .chars()
                .find(|ch| ch.is_ascii_alphanumeric())
                .map(|ch| ch.to_ascii_uppercase())
        })
        .unwrap_or('?')
}

#[derive(Clone, Debug)]
struct AsciiSpriteColorTable {
    aliases: HashMap<String, String>,
    sprites: HashMap<String, SpriteSample>,
}

impl AsciiSpriteColorTable {
    fn from_loaded(loaded: &LoadedGame) -> Self {
        let aliases = loaded
            .visuals
            .aliases
            .iter()
            .map(|alias| (alias.object.clone(), alias.sprite.clone()))
            .collect();
        let sprites = loaded
            .visuals
            .sprites
            .iter()
            .filter_map(|sprite| {
                sprite_sample(&sprite.kind).map(|sample| (sprite.name.clone(), sample))
            })
            .collect();
        Self { aliases, sprites }
    }

    fn composited_color_for_object_names<'a>(
        &self,
        object_names: impl Iterator<Item = &'a str>,
    ) -> Option<Rgb> {
        let sprites = object_names
            .filter_map(|object_name| self.sprite_for_object_name(object_name))
            .collect::<Vec<_>>();
        composite_sprite_samples(&sprites)
    }

    fn sprite_for_object_name(&self, object_name: &str) -> Option<&SpriteSample> {
        self.aliases
            .get(object_name)
            .and_then(|sprite| self.sprites.get(sprite))
            .or_else(|| self.sprites.get(object_name))
            .or_else(|| self.sprites.get(&sprite_name(object_name)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SpriteSample {
    Solid(Rgba),
    Pixels {
        width: usize,
        height: usize,
        pixels: Vec<Option<Rgba>>,
    },
}

impl SpriteSample {
    fn width(&self) -> usize {
        match self {
            Self::Solid(_) => 1,
            Self::Pixels { width, .. } => *width,
        }
    }

    fn height(&self) -> usize {
        match self {
            Self::Solid(_) => 1,
            Self::Pixels { height, .. } => *height,
        }
    }

    fn pixel_at(&self, x: usize, y: usize) -> Option<Rgba> {
        match self {
            Self::Solid(color) => Some(*color),
            Self::Pixels { width, pixels, .. } => pixels.get(y * *width + x).copied().flatten(),
        }
    }
}

fn sprite_sample(kind: &VisualSpriteKind) -> Option<SpriteSample> {
    match kind {
        VisualSpriteKind::Solid(color) => parse_rgba(color).map(SpriteSample::Solid),
        VisualSpriteKind::Image { .. } => None,
        VisualSpriteKind::Ascii { pattern, colors } => {
            let palette = colors
                .iter()
                .filter_map(|color| parse_rgba(&color.color).map(|sample| (color.token, sample)))
                .collect::<HashMap<_, _>>();
            let width = pattern
                .iter()
                .map(|row| row.chars().count())
                .max()
                .unwrap_or(0);
            let height = pattern.len();
            if width == 0 || height == 0 {
                return None;
            }
            let mut has_pixel = false;
            let mut pixels = Vec::with_capacity(width * height);
            for row in pattern {
                let mut row_chars = row.chars();
                for _ in 0..width {
                    let pixel = row_chars.next().and_then(|ch| palette.get(&ch).copied());
                    has_pixel |= pixel.is_some_and(|color| color.a > 0);
                    pixels.push(pixel.filter(|color| color.a > 0));
                }
            }
            has_pixel.then_some(SpriteSample::Pixels {
                width,
                height,
                pixels,
            })
        }
    }
}

#[cfg(test)]
fn sprite_color_sample(kind: &VisualSpriteKind) -> Option<ColorSample> {
    sprite_sample(kind).and_then(|sample| composite_sprite_samples(&[&sample]).map(color_sample))
}

fn composite_sprite_samples(sprites: &[&SpriteSample]) -> Option<Rgb> {
    let width = sprites
        .iter()
        .map(|sprite| sprite.width())
        .max()
        .unwrap_or(0);
    let height = sprites
        .iter()
        .map(|sprite| sprite.height())
        .max()
        .unwrap_or(0);
    if width == 0 || height == 0 {
        return None;
    }

    let mut total = ColorSample::default();
    for y in 0..height {
        for x in 0..width {
            let mut dst = PremultipliedRgba::default();
            for sprite in sprites {
                if let Some(src) = sprite.pixel_at(x, y) {
                    dst = dst.over(src);
                }
            }
            if let Some((rgb, alpha)) = dst.to_rgb_alpha() {
                total.add_rgb(rgb, u64::from(alpha));
            }
        }
    }
    total.average()
}

#[cfg(test)]
fn color_sample(rgb: Rgb) -> ColorSample {
    let mut sample = ColorSample::default();
    sample.add_rgb(rgb, 255);
    sample
}

#[derive(Clone, Copy, Debug, Default)]
struct PremultipliedRgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl PremultipliedRgba {
    fn over(self, src: Rgba) -> Self {
        let src_a = f64::from(src.a) / 255.0;
        let keep_dst = 1.0 - src_a;
        Self {
            r: f64::from(src.r) * src_a + self.r * keep_dst,
            g: f64::from(src.g) * src_a + self.g * keep_dst,
            b: f64::from(src.b) * src_a + self.b * keep_dst,
            a: src_a + self.a * keep_dst,
        }
    }

    fn to_rgb_alpha(self) -> Option<(Rgb, u8)> {
        if self.a <= 0.0 {
            return None;
        }
        let scale = 1.0 / self.a;
        Some((
            Rgb {
                r: ((self.r * scale).round()).clamp(0.0, 255.0) as u8,
                g: ((self.g * scale).round()).clamp(0.0, 255.0) as u8,
                b: ((self.b * scale).round()).clamp(0.0, 255.0) as u8,
            },
            (self.a * 255.0).round().clamp(0.0, 255.0) as u8,
        ))
    }
}

fn parse_rgba(color: &str) -> Option<Rgba> {
    let color = color.trim();
    let (rgb, alpha) = parse_hex_color(color).or_else(|| parse_named_color(color))?;
    (alpha > 0).then_some(Rgba {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
        a: alpha,
    })
}

fn parse_hex_color(color: &str) -> Option<(Rgb, u8)> {
    let hex = color.strip_prefix('#')?;
    match hex.len() {
        3 | 4 => {
            let mut values = hex
                .chars()
                .filter_map(|ch| ch.to_digit(16).map(|value| value as u8));
            let r = expand_hex_nibble(values.next()?);
            let g = expand_hex_nibble(values.next()?);
            let b = expand_hex_nibble(values.next()?);
            let a = values.next().map(expand_hex_nibble).unwrap_or(255);
            Some((Rgb { r, g, b }, a))
        }
        6 | 8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = if hex.len() == 8 {
                u8::from_str_radix(&hex[6..8], 16).ok()?
            } else {
                255
            };
            Some((Rgb { r, g, b }, a))
        }
        _ => None,
    }
}

fn expand_hex_nibble(value: u8) -> u8 {
    value * 17
}

fn parse_named_color(color: &str) -> Option<(Rgb, u8)> {
    let rgb = match color {
        "black" => Rgb { r: 0, g: 0, b: 0 },
        "silver" => Rgb {
            r: 192,
            g: 192,
            b: 192,
        },
        "gray" => Rgb {
            r: 128,
            g: 128,
            b: 128,
        },
        "white" => Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
        "maroon" => Rgb { r: 128, g: 0, b: 0 },
        "red" => Rgb { r: 255, g: 0, b: 0 },
        "purple" => Rgb {
            r: 128,
            g: 0,
            b: 128,
        },
        "fuchsia" => Rgb {
            r: 255,
            g: 0,
            b: 255,
        },
        "green" => Rgb { r: 0, g: 128, b: 0 },
        "lime" => Rgb { r: 0, g: 255, b: 0 },
        "olive" => Rgb {
            r: 128,
            g: 128,
            b: 0,
        },
        "yellow" => Rgb {
            r: 255,
            g: 255,
            b: 0,
        },
        "navy" => Rgb { r: 0, g: 0, b: 128 },
        "blue" => Rgb { r: 0, g: 0, b: 255 },
        "teal" => Rgb {
            r: 0,
            g: 128,
            b: 128,
        },
        "aqua" => Rgb {
            r: 0,
            g: 255,
            b: 255,
        },
        "orange" => Rgb {
            r: 255,
            g: 165,
            b: 0,
        },
        "transparent" | "currentColor" => return None,
        _ => return None,
    };
    Some((rgb, 255))
}

fn sprite_name(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "object".to_string()
    } else {
        sprite
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ColorSample {
    r: u64,
    g: u64,
    b: u64,
    weight: u64,
}

impl ColorSample {
    fn add_rgb(&mut self, rgb: Rgb, weight: u64) {
        self.r += u64::from(rgb.r) * weight;
        self.g += u64::from(rgb.g) * weight;
        self.b += u64::from(rgb.b) * weight;
        self.weight += weight;
    }

    fn average(self) -> Option<Rgb> {
        (self.weight > 0).then(|| Rgb {
            r: ((self.r + self.weight / 2) / self.weight) as u8,
            g: ((self.g + self.weight / 2) / self.weight) as u8,
            b: ((self.b + self.weight / 2) / self.weight) as u8,
        })
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
        [name] if name == "title" => loaded.title.clone(),
        [name] if name == "subtitle" => loaded.subtitle.clone().unwrap_or_default(),
        [name] if name == "author" => loaded.author.clone().unwrap_or_default(),
        [name] if name == "homepage" => loaded.homepage.clone().unwrap_or_default(),
        [name] if name == "level" => session.level_index().to_string(),
        [name] => scope
            .level_value(name, loaded)
            .or_else(|| scene_value(session, &scope.scene_name, name))
            .unwrap_or_else(|| name.clone()),
        [root, field] if root == "level" => current_level_field(loaded, session, field),
        [root, field] => scope
            .level_field(root, field, loaded, session)
            .or_else(|| scene_value_field(session, &scope.scene_name, root, field, loaded))
            .or_else(|| scene_value(session, &scope.scene_name, root))
            .unwrap_or_else(|| path.join(".")),
        _ => path.join("."),
    }
}

fn current_level_field(loaded: &LoadedGame, session: &GameSession, field: &str) -> String {
    match field {
        "index" => session.level_index().to_string(),
        "number" => (session.level_index() + 1).to_string(),
        "name" | "label" => session.current_level(loaded).name.clone(),
        "cleared" | "solved" => session
            .cleared_levels()
            .get(session.level_index())
            .copied()
            .unwrap_or(false)
            .to_string(),
        _ => String::new(),
    }
}

fn scene_value(session: &GameSession, scene_name: &str, name: &str) -> Option<String> {
    session
        .scene_state_for(scene_name)
        .and_then(|state| state.values.get(name))
        .map(scene_value_to_string)
}

fn scene_value_field(
    session: &GameSession,
    scene_name: &str,
    name: &str,
    field: &str,
    loaded: &LoadedGame,
) -> Option<String> {
    let value = session
        .scene_state_for(scene_name)
        .and_then(|state| state.values.get(name))
        .or_else(|| session.session_values().get(name))?;
    match value {
        SceneValue::LevelRef(index) => level_ref_field(loaded, session, *index, field),
        _ => None,
    }
}

fn scene_value_to_string(value: &SceneValue) -> String {
    match value {
        SceneValue::Bool(value) => value.to_string(),
        SceneValue::Int(value) => value.to_string(),
        SceneValue::Text(value) | SceneValue::Symbol(value) => value.clone(),
        SceneValue::LevelRef(index) => index.to_string(),
    }
}

fn level_ref_field(
    loaded: &LoadedGame,
    session: &GameSession,
    index: usize,
    field: &str,
) -> Option<String> {
    let level = loaded.levels.get(index)?;
    match field {
        "index" => Some(index.to_string()),
        "num" | "number" => Some((index + 1).to_string()),
        "name" | "label" | "title" => Some(level.name.clone()),
        "cleared" | "solved" => Some(
            session
                .cleared_levels()
                .get(index)
                .copied()
                .unwrap_or(false)
                .to_string(),
        ),
        _ => None,
    }
}

#[derive(Clone, Debug, Default)]
struct RenderScope {
    scene_name: String,
    levels: HashMap<String, usize>,
    level_index: Option<usize>,
}

impl RenderScope {
    fn for_scene(scene_name: &str) -> Self {
        Self {
            scene_name: scene_name.to_string(),
            ..Self::default()
        }
    }

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

    fn level_field(
        &self,
        binding: &str,
        field: &str,
        loaded: &LoadedGame,
        session: &GameSession,
    ) -> Option<String> {
        let index = *self.levels.get(binding)?;
        let level = loaded.levels.get(index)?;
        match field {
            "index" => Some(index.to_string()),
            "number" => Some((index + 1).to_string()),
            "name" | "label" => Some(level.name.clone()),
            "cleared" | "solved" => Some(
                session
                    .cleared_levels()
                    .get(index)
                    .copied()
                    .unwrap_or(false)
                    .to_string(),
            ),
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
                    b'\t' => TerminalKey::Named("Tab"),
                    0x7f | 0x08 => TerminalKey::Named("Backspace"),
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

fn command_for_key(
    loaded: &LoadedGame,
    session: &GameSession,
    ui_state: &mut TerminalUiState,
    key: TerminalKey,
) -> Option<String> {
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
            return effect_to_command(
                loaded,
                session,
                &binding.effect,
                &RenderScope::for_scene(&screen.name),
            );
        }
    }

    if first_level_menu(&screen.components).is_some() {
        if let Some(input) = menu_input_for_key(&key) {
            return Some(input.to_string());
        }
    }

    if screen.puzzle_rule.is_none() {
        if let Some(command) = scene_button_command_for_key(loaded, session, screen, ui_state, &key)
        {
            return Some(command);
        }
    }

    match key {
        TerminalKey::Char('z') => return Some("undo".to_string()),
        TerminalKey::Char('y') => return Some("redo".to_string()),
        _ => {}
    }

    input_command_for_key(loaded, key)
}

fn input3_for_key<'a>(parsed: &'a ParsedPuzzle3, key: &TerminalKey) -> Option<&'a InputDef3> {
    parsed
        .game
        .inputs
        .iter()
        .find(|input| {
            input
                .keys
                .iter()
                .any(|trigger| key_matches_name(key, trigger))
        })
        .or_else(|| {
            let name = default_input3_name_for_key(key)?;
            parsed.game.input_by_name(name)
        })
}

fn default_input3_name_for_key(key: &TerminalKey) -> Option<&'static str> {
    match key {
        TerminalKey::Char('a') | TerminalKey::Arrow(ArrowKey::Left) => Some("left"),
        TerminalKey::Char('d') | TerminalKey::Arrow(ArrowKey::Right) => Some("right"),
        TerminalKey::Char('w') | TerminalKey::Arrow(ArrowKey::Up) => Some("forward"),
        TerminalKey::Char('s') | TerminalKey::Arrow(ArrowKey::Down) => Some("backward"),
        TerminalKey::Char('e') => Some("up"),
        TerminalKey::Char('c') => Some("down"),
        _ => None,
    }
}

fn key_matches_name(key: &TerminalKey, expected: &str) -> bool {
    match key {
        TerminalKey::Char(actual) => expected
            .chars()
            .next()
            .is_some_and(|ch| expected.chars().count() == 1 && *actual == ch.to_ascii_lowercase()),
        TerminalKey::Named(actual) => *actual == expected,
        TerminalKey::Arrow(actual) => arrow_key_name(*actual) == expected,
        TerminalKey::Interrupt | TerminalKey::Ignored => false,
    }
}

fn scene_button_command_for_key(
    loaded: &LoadedGame,
    session: &GameSession,
    screen: &puzzle_lang::SceneDef,
    ui_state: &mut TerminalUiState,
    key: &TerminalKey,
) -> Option<String> {
    let buttons = scene_button_commands(loaded, session, screen);
    if buttons.is_empty() {
        return None;
    }

    match key {
        TerminalKey::Char('w')
        | TerminalKey::Char('a')
        | TerminalKey::Arrow(ArrowKey::Up)
        | TerminalKey::Arrow(ArrowKey::Left) => {
            ui_state.move_button_cursor(&screen.name, buttons.len(), -1);
            None
        }
        TerminalKey::Char('s')
        | TerminalKey::Char('d')
        | TerminalKey::Arrow(ArrowKey::Down)
        | TerminalKey::Arrow(ArrowKey::Right) => {
            ui_state.move_button_cursor(&screen.name, buttons.len(), 1);
            None
        }
        TerminalKey::Named("Enter") | TerminalKey::Named("Space") => {
            let cursor = ui_state.button_cursor(&screen.name).min(buttons.len() - 1);
            buttons.get(cursor).cloned().flatten()
        }
        TerminalKey::Char(ch) if ch.is_ascii_digit() && *ch != '0' => {
            let index = (*ch as u8 - b'1') as usize;
            buttons.get(index).cloned().flatten()
        }
        _ => None,
    }
}

fn scene_button_commands(
    loaded: &LoadedGame,
    session: &GameSession,
    screen: &puzzle_lang::SceneDef,
) -> Vec<Option<String>> {
    let mut commands = Vec::new();
    let scope = RenderScope::for_scene(&screen.name);
    collect_button_commands(loaded, session, &screen.components, &scope, &mut commands);
    commands
}

fn collect_button_commands(
    loaded: &LoadedGame,
    session: &GameSession,
    components: &[SceneComponent],
    scope: &RenderScope,
    commands: &mut Vec<Option<String>>,
) {
    for component in components {
        match component {
            SceneComponent::Button(button) | SceneComponent::Choice(button) => {
                commands.push(effect_to_command(loaded, session, &button.effect, scope));
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                collect_button_commands(loaded, session, &container.children, scope, commands);
            }
            SceneComponent::Conditional(conditional) => {
                collect_button_commands(loaded, session, &conditional.children, scope, commands);
            }
            SceneComponent::For(for_view) => {
                if for_view.source.is_levels() {
                    for index in scene_level_indices(loaded, &scope.scene_name) {
                        let child_scope = scope.with_level(&for_view.binding, index);
                        collect_button_commands(
                            loaded,
                            session,
                            &for_view.children,
                            &child_scope,
                            commands,
                        );
                    }
                }
            }
            SceneComponent::Frame(_)
            | SceneComponent::Title(_)
            | SceneComponent::Subtitle(_)
            | SceneComponent::Text(_)
            | SceneComponent::LevelMenu(_) => {}
        }
    }
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
        TerminalKey::Named("Enter") | TerminalKey::Named("Space") => Some("select"),
        TerminalKey::Named("Escape") | TerminalKey::Char('q') => Some("back"),
        _ => None,
    }
}

fn first_level_menu(components: &[SceneComponent]) -> Option<&puzzle_lang::LevelMenuDef> {
    for component in components {
        match component {
            SceneComponent::LevelMenu(menu) => return Some(menu),
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                if let Some(menu) = first_level_menu(&container.children) {
                    return Some(menu);
                }
            }
            SceneComponent::Conditional(conditional) => {
                if let Some(menu) = first_level_menu(&conditional.children) {
                    return Some(menu);
                }
                if let Some(menu) = first_level_menu(&conditional.else_children) {
                    return Some(menu);
                }
            }
            SceneComponent::For(for_view) => {
                if let Some(menu) = first_level_menu(&for_view.children) {
                    return Some(menu);
                }
            }
            _ => {}
        }
    }
    None
}

fn scene_level_indices(loaded: &LoadedGame, scene_name: &str) -> Vec<usize> {
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
                    .any(|name| level_resource_matches(name, &level.name))
                    .then_some(index)
            })
            .collect(),
    }
}

fn level_resource_matches(resource: &str, level_name: &str) -> bool {
    level_name == resource
        || level_name
            .strip_prefix(resource)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn level_menu_cursor_position(
    loaded: &LoadedGame,
    session: &GameSession,
    level_indices: &[usize],
) -> usize {
    let selected = session.selected_level_index();
    level_indices
        .iter()
        .position(|index| *index == selected)
        .or_else(|| {
            (selected >= loaded.levels.len())
                .then(|| level_indices.len() + selected - loaded.levels.len())
        })
        .unwrap_or(0)
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
        SceneEffect::PlaySfx { name } => Some(format!("sfx {name}")),
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
        SceneEffect::PuzzleNextLevel { target } => Some(format!("{target}.next_level")),
        SceneEffect::PuzzlePreviousLevel { target } => Some(format!("{target}.previous_level")),
        SceneEffect::GotoLevel { target, level } => Some(format!(
            "{target}.goto {}",
            expr_source(loaded, session, level, scope)
        )),
        SceneEffect::ResetPuzzle { target } => Some(format!("{target}.restart")),
        SceneEffect::LoadPuzzle { target, source } => Some(format!("load {target} from {source}")),
        SceneEffect::Copy { source, target } => Some(format!("copy {source} to {target}")),
        SceneEffect::ClearUndoHistory => Some("clear_undo_history".to_string()),
        SceneEffect::ClearGameProgress => Some("clear_game_progress".to_string()),
        SceneEffect::SetCurrentLevel { level } => Some(format!(
            "set current_level = {}",
            expr_source(loaded, session, level, scope)
        )),
        SceneEffect::ClearCurrentLevel => Some("clear current_level".to_string()),
        SceneEffect::SetLevelCleared { level, cleared } => level
            .as_ref()
            .map(|level| {
                format!(
                    "set level({}).cleared = {cleared}",
                    expr_source(loaded, session, level, scope)
                )
            })
            .or_else(|| Some(format!("set level.cleared = {cleared}"))),
        SceneEffect::ResetPersistentVars => Some("reset persistent_vars".to_string()),
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
    if let [puzzle_lang::SceneEffectParam::Level(value)] = params {
        return format!(
            "{command} {screen}({})",
            expr_source(loaded, session, value, scope)
        );
    }

    let params = params
        .iter()
        .filter_map(|param| match param {
            puzzle_lang::SceneEffectParam::Level(value) => {
                Some(expr_source(loaded, session, value, scope))
            }
            puzzle_lang::SceneEffectParam::Named { name, value } => Some(format!(
                "{} = {}",
                name,
                expr_source(loaded, session, value, scope)
            )),
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

#[derive(Clone, Debug, Default)]
struct TerminalEvents {
    lines: Vec<String>,
}

impl TerminalEvents {
    fn take_from(session: &mut GameSession) -> Self {
        let mut lines = Vec::new();
        for event in session.take_message_events() {
            match event {
                MessageEvent::Message { text } => lines.push(format!("message: {text}")),
            }
        }
        for event in session.take_sound_events() {
            lines.push(match event {
                SoundEvent::PlaySfx { name } => format!("sfx: {name}"),
                SoundEvent::PlayMusic { name } => format!("music: play {name}"),
                SoundEvent::PauseMusic { name } => optional_sound_line("music: pause", name),
                SoundEvent::ResumeMusic { name } => optional_sound_line("music: resume", name),
                SoundEvent::StopMusic { name } => optional_sound_line("music: stop", name),
            });
        }
        for event in session.take_wait_events() {
            match event {
                WaitEvent::Wait { milliseconds } => lines.push(format!("wait: {milliseconds}ms")),
                WaitEvent::ContinueEffects { milliseconds } => {
                    lines.push(format!("continue effects after: {milliseconds}ms"))
                }
            }
        }
        Self { lines }
    }

    fn render(&self, out: &mut String) {
        if self.lines.is_empty() {
            return;
        }
        for line in &self.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
}

fn optional_sound_line(prefix: &str, name: Option<String>) -> String {
    match name {
        Some(name) => format!("{prefix} {name}"),
        None => prefix.to_string(),
    }
}

#[derive(Clone, Debug, Default)]
struct TerminalUiState {
    button_cursors: HashMap<String, usize>,
}

impl TerminalUiState {
    fn button_cursor(&self, scene_name: &str) -> usize {
        self.button_cursors.get(scene_name).copied().unwrap_or(0)
    }

    fn move_button_cursor(&mut self, scene_name: &str, button_count: usize, delta: isize) {
        if button_count == 0 {
            self.button_cursors.insert(scene_name.to_string(), 0);
            return;
        }
        let current = self.button_cursor(scene_name).min(button_count - 1);
        let next = (current as isize + delta).clamp(0, button_count as isize - 1) as usize;
        self.button_cursors.insert(scene_name.to_string(), next);
    }
}

fn select_puzzle_path(args: impl IntoIterator<Item = String>) -> Result<PathBuf, AppError> {
    let mut args = args.into_iter();
    if let Some(path) = args.next() {
        if path == "--help" || path == "-h" {
            return Err(AppError::Config(
                "usage: ascii-play [path/to/game-folder-or-game.puzzle-or-game.puzzle3]"
                    .to_string(),
            ));
        }
        if args.next().is_some() {
            return Err(AppError::Config(
                "ascii-play accepts at most one path".to_string(),
            ));
        }
        return resolve_game_entry(PathBuf::from(path))
            .map_err(|error| AppError::Config(error.to_string()));
    }

    let candidates =
        discover_game_entries("games").map_err(|error| AppError::Config(error.to_string()))?;
    match candidates.len() {
        0 => Err(AppError::Config(
            "no games/*/game.puzzle or games/*/game.puzzle3 entries found. Pass a path: ascii-play <path/to/game-folder-or-game.puzzle-or-game.puzzle3>"
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

#[cfg(test)]
mod tests {
    use super::*;
    use puzzle_lang::parse_game2d as parse_game;

    #[test]
    fn ascii_board_colors_legend_chars_from_sprite_average() {
        let loaded = parse_game(
            r##"
title colored_ascii

puzzle default {
layers {
solid = Player
}
sprites {
Player {
#ff0000 #0000ff
01
}
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level start
P
}
}
"##,
        )
        .unwrap();
        let session = GameSession::new(&loaded);
        let state = &session.scene_state().unwrap().puzzles["default"];

        assert_eq!(
            render_colored_ascii_top(&loaded, state),
            "\x1b[38;2;128;0;128mP\x1b[0m "
        );
    }

    #[test]
    fn transparent_sprite_pixels_do_not_weight_terminal_color() {
        let sample = sprite_color_sample(&VisualSpriteKind::Ascii {
            pattern: vec!["01".to_string()],
            colors: vec![
                puzzle_lang::VisualColorDef {
                    token: '0',
                    color: "transparent".to_string(),
                },
                puzzle_lang::VisualColorDef {
                    token: '1',
                    color: "#0f0".to_string(),
                },
            ],
        })
        .unwrap();

        assert_eq!(sample.average(), Some(Rgb { r: 0, g: 255, b: 0 }));
    }

    #[test]
    fn overlap_text_uses_top_layer_and_color_uses_composited_sprite() {
        let loaded = parse_game(
            r##"
title overlap_color

puzzle default {
layers {
target = Goal
solid = Box
}
sprites {
Goal #00ff00
Box {
#ff0000
0.
}
}
rules {
}
levels {
legend {
. = empty
G = Goal
B = Box
* = Goal Box
}
level start
*
}
}
"##,
        )
        .unwrap();
        let session = GameSession::new(&loaded);
        let state = &session.scene_state().unwrap().puzzles["default"];

        assert_eq!(
            render_colored_ascii_top(&loaded, state),
            "\x1b[38;2;128;128;0mB\x1b[0m "
        );
    }

    #[test]
    fn empty_char_ignores_unlegended_display_floor_color() {
        let loaded = parse_game(
            r##"
title floor_dot_color

puzzle default {
layers {
@display_floor = @Floor
solid = Player
}
sprites {
@Floor #00ff00
Player #ff0000
}
routine @fill_floor repeat {
[ no @Floor ] -> [ @Floor ]
}
on_display {
@fill_floor
}
rules {
}
levels {
legend {
. = empty
P = Player
}
level start
.
}
}
"##,
        )
        .unwrap();
        let session = GameSession::new(&loaded);
        let state = &session.scene_state().unwrap().puzzles["default"];

        assert_eq!(render_colored_ascii_top(&loaded, state), ". ");
    }

    #[test]
    fn puzzle3_ascii_renders_z_slices_from_top_to_bottom() {
        let parsed = puzzle3d_model::parse_puzzle3d(
            r#"
layers {
actor = Top Bottom
}

rules {
}

levels3 stack {
legend {
. = empty
T = Top
B = Bottom
}

level one {
T.

.B
}
}
"#,
        )
        .unwrap();
        let state = parsed
            .level_bundle
            .as_ref()
            .unwrap()
            .build_level_state(0)
            .unwrap();

        assert_eq!(
            render_puzzle3_ascii_top_down(&parsed, &state),
            "z 1\nT . \n\nz 0\n. B "
        );
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

impl From<GameSessionError3> for AppError {
    fn from(value: GameSessionError3) -> Self {
        Self::Runtime(format!("{value:?}"))
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
