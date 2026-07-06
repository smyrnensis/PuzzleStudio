use std::fmt;

use crate::{SceneBinaryOp, SceneEffect, SceneEffectParam, SceneExpr};
use puzzle_authoring::{
    CallSurface, find_top_level_char, is_identifier, is_qualified_identifier, matching_delimiter,
    parse_assignment_row, parse_call_argument_surfaces,
    parse_optional_call_surface_with_suffix as authoring_parse_optional_call_surface_with_suffix,
    parse_quoted_text, parse_view_path, split_header_tokens, split_top_level_keyword_once,
    split_top_level_operator_once,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneParseError {
    source_line: String,
    message: String,
}

impl SceneParseError {
    fn new(line: &str, message: impl Into<String>) -> Self {
        Self {
            source_line: line.to_string(),
            message: message.into(),
        }
    }

    pub fn source_line(&self) -> &str {
        &self.source_line
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SceneParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SceneParseError {}

#[derive(Clone, Debug)]
struct SourceToken {
    text: String,
    start: usize,
    end: usize,
}

pub fn parse_scene_effect(value: &str) -> Result<SceneEffect, SceneParseError> {
    parse_scene_effect_at(value, value)
}

pub fn parse_scene_effect_at(value: &str, line: &str) -> Result<SceneEffect, SceneParseError> {
    parse_scene_effect_value(value, line)
}

fn parse_scene_effect_value(value: &str, line: &str) -> Result<SceneEffect, SceneParseError> {
    if value.contains(" then ") {
        return Err(parse_error(
            line,
            "`then` effect sequences are not supported; use an effect block with one effect per line",
        ));
    }

    if let Some((condition, effect)) = value
        .strip_prefix("if ")
        .and_then(|rest| rest.split_once("->"))
    {
        return Ok(SceneEffect::Conditional {
            condition: parse_scene_expr(condition.trim(), line)?,
            effect: Box::new(parse_scene_effect_value(effect.trim(), line)?),
        });
    }

    if let Some(parts) = split_scene_effect_sequence(value) {
        let mut effects = Vec::new();
        for part in parts {
            effects.push(parse_scene_effect_value(part, line)?);
        }
        return match effects.len() {
            0 => unreachable!("scene effect sequence splitter returned no effects"),
            1 => Ok(effects.remove(0)),
            _ => Ok(SceneEffect::Sequence(effects)),
        };
    }

    if let Some(text) = value.strip_prefix("message ") {
        return Ok(SceneEffect::Message {
            text: parse_scene_expr(text.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("current_level = ") {
        return Ok(SceneEffect::SetCurrentLevel {
            level: parse_scene_level_expr(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level.cleared = ") {
        return Ok(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_scene_effect_bool(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("level(")
        && let Some((level, cleared)) = rest.split_once(").cleared = ")
    {
        return Ok(SceneEffect::SetLevelCleared {
            level: Some(parse_scene_level_expr(level.trim(), line)?),
            cleared: parse_scene_effect_bool(cleared.trim(), line)?,
        });
    }
    if let Some((name, rhs)) = parse_scene_variable_assignment(value) {
        return Ok(SceneEffect::SetVariable {
            name: name.to_string(),
            value: parse_scene_expr(rhs, line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("goto ") {
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Goto { scene, params });
    }
    if let Some(rest) = value.strip_prefix("start ") {
        if rest.starts_with("levels ") || rest.contains(" in ") {
            return Err(legacy_start_levels_error(line));
        }
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Sequence(vec![
            SceneEffect::Reset {
                scene: scene.clone(),
            },
            SceneEffect::Goto { scene, params },
        ]));
    }

    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["input", input] => Ok(SceneEffect::Input(
            parse_input_name(input, line)?.to_string(),
        )),
        ["component_effect", effect] => Ok(SceneEffect::ComponentEffect(
            parse_scene_signal_name(effect, line, "component effect")?.to_string(),
        )),
        ["apply", call, "to", target] => {
            validate_target_path(target, line, "apply target")?;
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: Some((*target).to_string()),
            })
        }
        ["apply", call] => {
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: None,
            })
        }
        ["copy", source, "to", target] => {
            validate_target_path(source, line, "copy source")?;
            validate_target_path(target, line, "copy target")?;
            Ok(SceneEffect::Copy {
                source: (*source).to_string(),
                target: (*target).to_string(),
            })
        }
        ["load", target, "from", source] => {
            validate_target_path(target, line, "load target")?;
            Ok(SceneEffect::LoadPuzzle {
                target: (*target).to_string(),
                source: (*source).to_string(),
            })
        }
        ["wait"] => Ok(SceneEffect::Wait { milliseconds: None }),
        ["wait", duration] => Ok(SceneEffect::Wait {
            milliseconds: Some(parse_wait_duration_ms_at(duration, line)?),
        }),
        ["clear_undo_history"] | ["clear_history"] => Ok(SceneEffect::ClearUndoHistory),
        ["clear_game_progress"] => Ok(SceneEffect::ClearGameProgress),
        ["clear", "current_level"] => Ok(SceneEffect::ClearCurrentLevel),
        ["reset", "persistent_vars"] => Ok(SceneEffect::ResetPersistentVars),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(SceneEffect::PlaySfx {
                name: (*name).to_string(),
            })
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PlayMusic {
                name: (*name).to_string(),
            })
        }
        ["pause_music"] => Ok(SceneEffect::PauseMusic { name: None }),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PauseMusic {
                name: Some((*name).to_string()),
            })
        }
        ["resume_music"] => Ok(SceneEffect::ResumeMusic { name: None }),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::ResumeMusic {
                name: Some((*name).to_string()),
            })
        }
        ["stop_music"] => Ok(SceneEffect::StopMusic { name: None }),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::StopMusic {
                name: Some((*name).to_string()),
            })
        }
        ["reset", target] if target.contains('.') => {
            validate_target_path(target, line, "reset target")?;
            Ok(SceneEffect::ResetPuzzle {
                target: (*target).to_string(),
            })
        }
        ["start", "levels", ..] | ["start", _, "in", _] | ["continue", "levels", ..] => {
            Err(legacy_start_levels_error(line))
        }
        [target_command, level] => {
            if let Some((target, command)) = parse_puzzle_command(target_command, line)?
                && command == "goto"
            {
                return Ok(SceneEffect::GotoLevel {
                    target,
                    level: parse_scene_level_expr(level, line)?,
                });
            }
            Err(parse_error(line, SCENE_EFFECT_USAGE_WITH_TARGET_COMMAND))
        }
        ["input"] => Err(parse_error(line, "input effect must name an input")),
        ["component_effect"] => Err(parse_error(
            line,
            "component_effect must name a component effect",
        )),
        ["next_level"] => Ok(SceneEffect::PuzzleNextLevel {
            target: String::new(),
        }),
        ["previous_level"] => Ok(SceneEffect::PuzzlePreviousLevel {
            target: String::new(),
        }),
        ["restart"] => Ok(SceneEffect::ResetPuzzle {
            target: String::new(),
        }),
        [command_text] => {
            if let Some((target, command)) = parse_puzzle_command(command_text, line)? {
                if command == "next_level" {
                    return Ok(SceneEffect::PuzzleNextLevel { target });
                }
                if command == "previous_level" {
                    return Ok(SceneEffect::PuzzlePreviousLevel { target });
                }
                if command == "restart" {
                    return Ok(SceneEffect::ResetPuzzle { target });
                }
            }
            if is_identifier(command_text) {
                return Ok(SceneEffect::RoutineCall((*command_text).to_string()));
            }
            Err(parse_error(
                line,
                "bare scene effect aliases were removed; use `input <name>`, `component_effect <name>`, a scene routine, or an explicit scene effect",
            ))
        }
        _ => Err(parse_error(line, SCENE_EFFECT_USAGE)),
    }
}

const SCENE_EFFECT_USAGE_WITH_TARGET_COMMAND: &str = "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | <scene>.goto <level> | copy <puzzle> to <puzzle>";
const SCENE_EFFECT_USAGE: &str = "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | copy <puzzle> to <puzzle>";

fn parse_scene_variable_assignment(value: &str) -> Option<(&str, &str)> {
    let (name, value) = parse_assignment_row(value)?;
    if value.is_empty() || !is_identifier(name) || reserved_scene_assignment_target(name) {
        return None;
    }
    Some((name, value))
}

fn reserved_scene_assignment_target(name: &str) -> bool {
    matches!(name, "current_level" | "level")
}

fn split_scene_effect_sequence(value: &str) -> Option<Vec<&str>> {
    let stripped = strip_line_comment(value);
    let tokens = source_line_tokens(stripped, 0);
    let parts = split_scene_effect_token_sequence(&tokens)?;
    Some(
        parts
            .into_iter()
            .map(|part| stripped[part.first().unwrap().start..part.last().unwrap().end].trim())
            .collect(),
    )
}

fn split_scene_effect_token_sequence(tokens: &[SourceToken]) -> Option<Vec<&[SourceToken]>> {
    let mut parts = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let length = scene_effect_token_length(&tokens[index..])?;
        parts.push(&tokens[index..index + length]);
        index += length;
    }
    (parts.len() > 1).then_some(parts)
}

fn scene_effect_token_length(tokens: &[SourceToken]) -> Option<usize> {
    let first = tokens.first()?.text.as_str();
    match first {
        "input" | "component_effect" | "sfx" | "play_music" => (tokens.len() >= 2).then_some(2),
        "pause_music" | "resume_music" | "stop_music" | "wait" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "clear_undo_history" | "clear_history" | "clear_game_progress" => Some(1),
        "clear" => (tokens.get(1)?.text == "current_level").then_some(2),
        "reset"
            if tokens
                .get(1)
                .is_some_and(|token| token.text == "persistent_vars") =>
        {
            Some(2)
        }
        "reset" if tokens.get(1).is_some_and(|token| token.text.contains('.')) => Some(2),
        "goto" | "start" => {
            if tokens.get(2).is_some_and(|token| token.text == "with") {
                None
            } else {
                (tokens.len() >= 2).then_some(2)
            }
        }
        _ if first.contains('.') => {
            let command = first.rsplit_once('.').map(|(_, command)| command)?;
            match command {
                "goto" | "goto_level" => (tokens.len() >= 2).then_some(2),
                "next_level" | "previous_level" | "restart" => Some(1),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_scene_effect_command_start(token: &str) -> bool {
    matches!(
        token,
        "input"
            | "component_effect"
            | "goto"
            | "start"
            | "sfx"
            | "play_music"
            | "pause_music"
            | "resume_music"
            | "stop_music"
            | "apply"
            | "clear_history"
            | "clear_undo_history"
            | "clear_game_progress"
            | "clear"
            | "copy"
            | "load"
            | "message"
            | "wait"
    ) || token.rsplit_once('.').is_some_and(|(_, command)| {
        matches!(
            command,
            "goto" | "goto_level" | "next_level" | "previous_level" | "restart"
        )
    })
}

fn parse_scene_effect_bool(value: &str, line: &str) -> Result<bool, SceneParseError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            line,
            "boolean progress value must be true or false",
        )),
    }
}

fn legacy_start_levels_error(line: &str) -> SceneParseError {
    parse_error(
        line,
        "`start levels ... in <scene>` and `continue levels ... in <scene>` are no longer supported; use `goto <puzzle>` for the default playable scene, `goto <puzzle>(<level>)` for a specific level, or `goto <scene>(<level>)` for an explicit level scene",
    )
}

pub fn parse_wait_duration_ms(value: &str) -> Result<u64, SceneParseError> {
    parse_wait_duration_ms_at(value, value)
}

pub fn parse_wait_duration_ms_at(value: &str, line: &str) -> Result<u64, SceneParseError> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return parse_whole_milliseconds(milliseconds, line);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return parse_seconds_duration_ms_at(seconds, line);
    }
    Err(parse_error(
        line,
        "wait duration must use seconds or milliseconds, for example `wait 0.1s` or `wait 100ms`",
    ))
}

fn parse_whole_milliseconds(value: &str, line: &str) -> Result<u64, SceneParseError> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait milliseconds must be a whole number",
        ));
    }
    value
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))
}

pub fn parse_seconds_duration_ms(value: &str) -> Result<u64, SceneParseError> {
    parse_seconds_duration_ms_at(value, value)
}

pub fn parse_seconds_duration_ms_at(value: &str, line: &str) -> Result<u64, SceneParseError> {
    let value = value.trim();
    let has_decimal = value.contains('.');
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if (has_decimal && fraction.is_empty()) || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if fraction.len() > 3 {
        return Err(parse_error(
            line,
            "wait seconds can use at most millisecond precision",
        ));
    }
    let whole_ms = whole
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))?
        .checked_mul(1000)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))?;
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{fraction:0<3}");
        padded
            .parse::<u64>()
            .map_err(|_| parse_error(line, "wait duration is too large"))?
    };
    whole_ms
        .checked_add(fraction_ms)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))
}

fn parse_rule_call_expr(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneExpr>), SceneParseError> {
    let value = value.trim();
    let Some(call) = parse_complete_call_surface(
        value,
        line,
        "rule call args must end with )",
        "rule call expression must not have trailing text",
    )?
    else {
        validate_qualified_identifier(value, line, "rule name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(call.name, line, "rule name")?;
    let args = parse_scene_call_arg_surfaces(&call.args, line)?;
    Ok((call.name.to_string(), args))
}

fn parse_scene_call_params(
    value: &str,
    line: &str,
) -> Result<Option<(String, Vec<SceneEffectParam>)>, SceneParseError> {
    let Some((call, suffix)) =
        parse_optional_call_surface_with_suffix(value, line, "scene call must close with `)`")?
    else {
        return Ok(None);
    };
    if !suffix.is_empty() {
        return Err(parse_error(line, "scene call must close with `)`"));
    }
    validate_qualified_identifier(call.name, line, "scene name")?;
    if call.args.is_empty() {
        return Ok(Some((call.name.to_string(), Vec::new())));
    }

    let params = if call.args.len() == 1 && parse_assignment_row(call.args[0]).is_none() {
        vec![SceneEffectParam::Level(parse_scene_level_expr(
            call.args[0],
            line,
        )?)]
    } else {
        parse_scene_named_params(&call.args, line)?
    };
    Ok(Some((call.name.to_string(), params)))
}

fn parse_scene_target_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneEffectParam>), SceneParseError> {
    let value = value.trim();
    if let Some((scene, params)) = value.split_once(" with ") {
        let scene = scene.trim();
        validate_qualified_identifier(scene, line, "scene name")?;
        let parts = parse_call_argument_surfaces(params);
        return Ok((scene.to_string(), parse_scene_named_params(&parts, line)?));
    }
    if let Some((scene, params)) = parse_scene_call_params(value, line)? {
        return Ok((scene, params));
    }
    validate_qualified_identifier(value, line, "scene name")?;
    Ok((value.to_string(), Vec::new()))
}

fn parse_scene_named_params(
    parts: &[&str],
    line: &str,
) -> Result<Vec<SceneEffectParam>, SceneParseError> {
    let mut params = Vec::new();
    for part in parts {
        let (name, value) =
            require_assignment_row(part, "scene params must be named `<name> = <expr>`")?;
        validate_identifier(name, line, "scene param name")?;
        params.push(SceneEffectParam::Named {
            name: name.to_string(),
            value: parse_scene_expr(value, line)?,
        });
    }
    Ok(params)
}

pub fn parse_scene_expression(value: &str) -> Result<SceneExpr, SceneParseError> {
    parse_scene_expr(value, value)
}

pub fn parse_scene_expression_at(value: &str, line: &str) -> Result<SceneExpr, SceneParseError> {
    parse_scene_expr(value, line)
}

pub fn parse_scene_expression_args(value: &str) -> Result<Vec<SceneExpr>, SceneParseError> {
    parse_scene_call_args(value.trim(), value)
}

pub fn parse_scene_effect_params(value: &str) -> Result<Vec<SceneEffectParam>, SceneParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(Vec::new());
    }
    parse_call_argument_surfaces(value)
        .into_iter()
        .map(|param| {
            if let Some((name, value)) = parse_assignment_row(param) {
                validate_identifier(name, param, "scene effect parameter name")?;
                return Ok(SceneEffectParam::Named {
                    name: name.to_string(),
                    value: parse_scene_expr(value, param)?,
                });
            }
            Ok(SceneEffectParam::Level(parse_scene_level_expr(
                param, param,
            )?))
        })
        .collect()
}

fn parse_scene_level_expr(value: &str, line: &str) -> Result<SceneExpr, SceneParseError> {
    if let Some(expr) = parse_level_selector_expr(value, line)? {
        return Ok(expr);
    }
    parse_scene_expr(value, line)
}

fn parse_scene_expr(value: &str, line: &str) -> Result<SceneExpr, SceneParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(parse_error(line, "expression must not be empty"));
    }
    if let Some(expr) = parse_scene_if_expr(value, line)? {
        return Ok(expr);
    }
    if let Some((left, right)) = split_top_level_keyword_once(value, "and") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::And,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if let Some((left, right)) = split_top_level_operator_once(value, "==") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::Eq,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if let Some((left, right)) = split_top_level_operator_once(value, "!=") {
        return Ok(SceneExpr::Binary {
            op: SceneBinaryOp::NotEq,
            left: Box::new(parse_scene_expr(left.trim(), line)?),
            right: Box::new(parse_scene_expr(right.trim(), line)?),
        });
    }
    if value == "true" {
        return Ok(SceneExpr::Bool(true));
    }
    if value == "false" {
        return Ok(SceneExpr::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneExpr::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneExpr::Text(text));
    }
    if let Some(expr) = parse_level_selector_expr(value, line)? {
        return Ok(expr);
    }
    if value.contains('(') {
        let (name, args) = parse_rule_call_expr(value, line)?;
        return Ok(SceneExpr::Call { name, args });
    }
    if value.starts_with("join ") {
        return Err(parse_error(
            line,
            "`join` scene expression is not supported",
        ));
    }
    if let Some(path) = parse_view_path(value) {
        return Ok(SceneExpr::Path(path));
    }
    Err(parse_error(
        line,
        "expression must be true, false, integer, quoted text, path, call, or if expression",
    ))
}

fn parse_scene_if_expr(value: &str, line: &str) -> Result<Option<SceneExpr>, SceneParseError> {
    let Some(rest) = value.strip_prefix("if ") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(open) = find_top_level_char(rest, '{') else {
        return Err(parse_error(
            line,
            "scene if expression must be: if <bool> { <value> } else { <value> }",
        ));
    };
    let condition = rest[..open].trim();
    if condition.is_empty() {
        return Err(parse_error(
            line,
            "scene if expression requires a condition",
        ));
    }
    let close = matching_delimiter(rest, open, '{', '}')
        .ok_or_else(|| parse_error(line, "scene if expression branch must close with `}`"))?;
    let then_branch = rest[open + 1..close].trim();
    let after_then = rest[close + 1..].trim_start();
    let Some(after_else) = after_then.strip_prefix("else") else {
        return Err(parse_error(
            line,
            "scene if expression requires an else branch",
        ));
    };
    let after_else = after_else.trim_start();
    if !after_else.starts_with('{') {
        return Err(parse_error(
            line,
            "scene if expression else branch must start with `{`",
        ));
    }
    let else_close = matching_delimiter(after_else, 0, '{', '}')
        .ok_or_else(|| parse_error(line, "scene if expression else branch must close with `}`"))?;
    if !after_else[else_close + 1..].trim().is_empty() {
        return Err(parse_error(
            line,
            "scene if expression must not have trailing text after else branch",
        ));
    }
    let else_branch = after_else[1..else_close].trim();
    Ok(Some(SceneExpr::If {
        condition: Box::new(parse_scene_expr(condition, line)?),
        then_branch: Box::new(parse_scene_expr(then_branch, line)?),
        else_branch: Box::new(parse_scene_expr(else_branch, line)?),
    }))
}

fn parse_level_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, SceneParseError> {
    let value = value.trim();
    if let Some(expr) = parse_level_call_selector_expr(value, line)? {
        return Ok(Some(expr));
    }
    parse_pack_level_selector_expr(value, line)
}

fn parse_level_call_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, SceneParseError> {
    let Some((call, suffix)) =
        parse_optional_call_surface_with_suffix(value, line, "level selector must close with `)`")?
    else {
        return Ok(None);
    };
    if call.name != "level" {
        return Ok(None);
    }
    let args = parse_scene_call_arg_surfaces(&call.args, line)?;
    let name = if suffix.is_empty() {
        "level".to_string()
    } else if let Some(field) = suffix.strip_prefix('.') {
        validate_identifier(field, line, "level property")?;
        format!("level.{field}")
    } else {
        return Err(parse_error(
            line,
            "level selector suffix must be empty or `.property`",
        ));
    };
    Ok(Some(SceneExpr::Call { name, args }))
}

fn parse_pack_level_selector_expr(
    value: &str,
    line: &str,
) -> Result<Option<SceneExpr>, SceneParseError> {
    let Some(open) = value.find('[') else {
        return Ok(None);
    };
    let Some(close) = value[open + 1..].find(']').map(|offset| open + 1 + offset) else {
        return Err(parse_error(line, "level pack selector must close with `]`"));
    };
    let pack = value[..open].trim();
    if pack.is_empty() || !is_qualified_identifier(pack) {
        return Ok(None);
    }
    let key = value[open + 1..close].trim();
    let suffix = value[close + 1..].trim();
    let mut args = vec![SceneExpr::Path(vec![pack.to_string()])];
    let base = if let Some(id) = parse_quoted_text(key) {
        args.push(SceneExpr::Text(id));
        "level_in"
    } else if let Ok(index) = key.parse::<i64>() {
        args.push(SceneExpr::Int(index));
        "level_at"
    } else {
        return Err(parse_error(
            line,
            "level pack selector key must be a quoted id or integer index",
        ));
    };
    let name = if suffix.is_empty() {
        base.to_string()
    } else if let Some(field) = suffix.strip_prefix('.') {
        validate_identifier(field, line, "level property")?;
        format!("{base}.{field}")
    } else {
        return Err(parse_error(
            line,
            "level pack selector suffix must be empty or `.property`",
        ));
    };
    Ok(Some(SceneExpr::Call { name, args }))
}

fn parse_scene_call_args(value: &str, line: &str) -> Result<Vec<SceneExpr>, SceneParseError> {
    let args = parse_call_argument_surfaces(value);
    parse_scene_call_arg_surfaces(&args, line)
}

fn parse_scene_call_arg_surfaces(
    args: &[&str],
    line: &str,
) -> Result<Vec<SceneExpr>, SceneParseError> {
    args.iter()
        .map(|arg| parse_scene_expr(arg.trim(), line))
        .collect()
}

fn parse_input_name<'a>(value: &'a str, line: &str) -> Result<&'a str, SceneParseError> {
    validate_identifier(value, line, "input name")?;
    Ok(value)
}

fn parse_scene_signal_name<'a>(
    value: &'a str,
    line: &str,
    label: &str,
) -> Result<&'a str, SceneParseError> {
    validate_qualified_identifier(value, line, label)?;
    Ok(value)
}

fn parse_puzzle_command<'a>(
    value: &'a str,
    line: &str,
) -> Result<Option<(String, &'a str)>, SceneParseError> {
    let Some((target, command)) = value.split_once('.') else {
        return Ok(None);
    };
    validate_qualified_identifier(target, line, "puzzle target")?;
    validate_identifier(command, line, "puzzle command")?;
    Ok(Some((target.to_string(), command)))
}

fn validate_target_path(value: &str, line: &str, label: &str) -> Result<(), SceneParseError> {
    if parse_view_path(value).is_some() {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be an identifier path"),
        ))
    }
}

#[derive(Clone, Copy)]
enum NameClass {
    Identifier,
    Qualified,
}

fn validate_name(
    value: &str,
    class: NameClass,
    line: &str,
    label: &str,
) -> Result<(), SceneParseError> {
    let valid = match class {
        NameClass::Identifier => is_identifier(value),
        NameClass::Qualified => is_qualified_identifier(value),
    };
    if valid {
        Ok(())
    } else {
        let expected = match class {
            NameClass::Identifier => "an identifier",
            NameClass::Qualified => "a qualified identifier",
        };
        Err(parse_error(line, &format!("{label} must be {expected}")))
    }
}

fn validate_identifier(value: &str, line: &str, label: &str) -> Result<(), SceneParseError> {
    validate_name(value, NameClass::Identifier, line, label)
}

fn validate_qualified_identifier(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), SceneParseError> {
    validate_name(value, NameClass::Qualified, line, label)
}

fn parse_optional_call_surface_with_suffix<'a>(
    value: &'a str,
    line: &str,
    close_message: &str,
) -> Result<Option<(CallSurface<'a>, &'a str)>, SceneParseError> {
    authoring_parse_optional_call_surface_with_suffix(value)
        .map_err(|()| parse_error(line, close_message))
}

fn parse_complete_call_surface<'a>(
    value: &'a str,
    line: &str,
    close_message: &str,
    trailing_message: &str,
) -> Result<Option<CallSurface<'a>>, SceneParseError> {
    let Some((call, suffix)) = parse_optional_call_surface_with_suffix(value, line, close_message)?
    else {
        return Ok(None);
    };
    if !suffix.is_empty() {
        return Err(parse_error(line, trailing_message));
    }
    Ok(Some(call))
}

fn require_assignment_row<'a>(
    line: &'a str,
    message: &str,
) -> Result<(&'a str, &'a str), SceneParseError> {
    parse_assignment_row(line).ok_or_else(|| parse_error(line, message))
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let mut previous = None;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if previous == Some('/') && ch == '/' {
            return &line[..index - 1];
        }
        previous = Some(ch);
    }
    line
}

fn source_line_tokens(line: &str, line_offset: usize) -> Vec<SourceToken> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if let Some(token_start) = start.take() {
                tokens.push(SourceToken {
                    text: line[token_start..index].to_string(),
                    start: line_offset + token_start,
                    end: line_offset + index,
                });
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(token_start) = start {
        tokens.push(SourceToken {
            text: line[token_start..].to_string(),
            start: line_offset + token_start,
            end: line_offset + line.len(),
        });
    }
    tokens
}

fn parse_error(line: &str, message: impl Into<String>) -> SceneParseError {
    SceneParseError::new(line, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_effect_parser_handles_lifecycle_effects() {
        assert_eq!(
            parse_scene_effect("next_level").unwrap(),
            SceneEffect::PuzzleNextLevel {
                target: String::new()
            }
        );
        assert_eq!(
            parse_scene_effect("game.next_level").unwrap(),
            SceneEffect::PuzzleNextLevel {
                target: "game".to_string()
            }
        );
        assert_eq!(
            parse_scene_effect("if win_conditions -> next_level").unwrap(),
            SceneEffect::Conditional {
                condition: SceneExpr::Path(vec!["win_conditions".to_string()]),
                effect: Box::new(SceneEffect::PuzzleNextLevel {
                    target: String::new()
                }),
            }
        );
        assert_eq!(
            parse_scene_effect("message \"END\"").unwrap(),
            SceneEffect::Message {
                text: SceneExpr::Text("END".to_string())
            }
        );
    }

    #[test]
    fn scene_expression_parser_handles_level_selectors() {
        assert_eq!(
            parse_scene_expression(r#"packs["one"].cleared"#).unwrap(),
            SceneExpr::Call {
                name: "level_in.cleared".to_string(),
                args: vec![
                    SceneExpr::Path(vec!["packs".to_string()]),
                    SceneExpr::Text("one".to_string())
                ],
            }
        );
    }
}
