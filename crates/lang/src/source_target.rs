use crate::source::{
    SourceContext, SourceContextLine, SourceScope, scan_source_context, strip_line_comment,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceTargetKind {
    Level,
    Level3d,
    Sprite,
    Sprite3d,
    Sounds,
}

impl SourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Level => "level",
            Self::Level3d => "level3d",
            Self::Sprite => "sprite",
            Self::Sprite3d => "sprite3d",
            Self::Sounds => "sounds",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundSourceTargetKind {
    Sfx,
    Music,
}

impl SoundSourceTargetKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sfx => "sfx",
            Self::Music => "music",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTarget {
    pub kind: SourceTargetKind,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub body_start: Option<usize>,
    pub body_end: Option<usize>,
    pub level_index: Option<usize>,
    pub sound_kind: Option<SoundSourceTargetKind>,
    pub params: Vec<(String, String)>,
}

pub fn resolve_source_target(source: &str, cursor_offset: usize) -> Option<SourceTarget> {
    let cursor = cursor_offset.min(source.len());
    let context = scan_source_context(source);
    resolve_sounds_target(&context, cursor)
        .or_else(|| resolve_level3d_target(source, &context, cursor))
        .or_else(|| resolve_level_target(source, &context, cursor))
        .or_else(|| resolve_sprite3d_target(source, &context, cursor))
        .or_else(|| resolve_sprite_target(source, &context, cursor))
}

pub fn source_target_json(target: Option<&SourceTarget>) -> String {
    let mut out = String::new();
    out.push_str("{\"target\":");
    match target {
        Some(target) => push_target_json(&mut out, target),
        None => out.push_str("null"),
    }
    out.push('}');
    out
}

fn push_target_json(out: &mut String, target: &SourceTarget) {
    out.push('{');
    push_json_string(out, "kind", target.kind.as_str());
    out.push(',');
    push_json_string(out, "name", &target.name);
    out.push(',');
    push_json_number(out, "start", target.start);
    out.push(',');
    push_json_number(out, "end", target.end);
    if let Some(body_start) = target.body_start {
        out.push(',');
        push_json_number(out, "bodyStart", body_start);
    }
    if let Some(body_end) = target.body_end {
        out.push(',');
        push_json_number(out, "bodyEnd", body_end);
    }
    if let Some(level_index) = target.level_index {
        out.push(',');
        push_json_number(out, "levelIndex", level_index);
    }
    if let Some(sound_kind) = &target.sound_kind {
        out.push(',');
        push_json_string(out, "soundKind", sound_kind.as_str());
    }
    if !target.params.is_empty() {
        out.push_str(",\"params\":{");
        for (index, (key, value)) in target.params.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_json_string_value(out, key);
            out.push(':');
            push_json_string_value(out, value);
        }
        out.push('}');
    }
    out.push('}');
}

fn resolve_sounds_target(context: &SourceContext, cursor: usize) -> Option<SourceTarget> {
    for line in &context.lines {
        let line_end = line_end(line);
        if cursor < line.start || cursor > line_end || line.scope != Some(SourceScope::Sounds) {
            continue;
        }
        let (sound_kind, name, params) = parse_sound_definition(line)?;
        return Some(SourceTarget {
            kind: SourceTargetKind::Sounds,
            name,
            start: line.start,
            end: line_end,
            body_start: None,
            body_end: None,
            level_index: None,
            sound_kind: Some(sound_kind),
            params,
        });
    }
    None
}

fn parse_sound_definition(
    line: &SourceContextLine,
) -> Option<(SoundSourceTargetKind, String, Vec<(String, String)>)> {
    let [kind, name, rest @ ..] = line.tokens.as_slice() else {
        return None;
    };
    let sound_kind = match kind.as_str() {
        "sfx" => SoundSourceTargetKind::Sfx,
        "music" => SoundSourceTargetKind::Music,
        _ => return None,
    };
    Some((sound_kind, name.clone(), parse_assignment_params(rest)))
}

fn parse_assignment_params(tokens: &[String]) -> Vec<(String, String)> {
    tokens
        .iter()
        .filter_map(|token| {
            let (key, value) = token.split_once('=')?;
            Some((key.to_string(), trim_quotes(value).to_string()))
        })
        .collect()
}

fn trim_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|stripped| stripped.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn resolve_level_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let level3d_blocks = level3d_blocks(source, context);
    let mut level_index = 0usize;
    for (index, line) in context.lines.iter().enumerate() {
        if level3d_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let Some(name) = level_name(line) else {
            continue;
        };
        let start = line.start;
        let (end, body_start, body_end) = level_range(source, context, index);
        let current_index = level_index;
        level_index += 1;
        if cursor < start || cursor > end {
            continue;
        }
        return Some(SourceTarget {
            kind: SourceTargetKind::Level,
            name,
            start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: Some(current_index),
            sound_kind: None,
            params: Vec::new(),
        });
    }
    None
}

fn resolve_level3d_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let mut level_index = 0usize;
    for block in level3d_blocks(source, context) {
        let bundle = block.bundle.clone();
        let model = block.model.clone();
        if cursor <= block.open_index || cursor >= block.close_index {
            level_index += context
                .lines
                .iter()
                .filter(|line| line.start > block.open_index && line.start < block.close_index)
                .filter(|line| level_name(line).is_some())
                .count();
            continue;
        }
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = level_name(line) else {
                continue;
            };
            let start = line.start;
            let (end, body_start, body_end) = level_range(source, context, index);
            let current_index = level_index;
            level_index += 1;
            if cursor < start || cursor > end {
                continue;
            }
            return Some(SourceTarget {
                kind: SourceTargetKind::Level3d,
                name,
                start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: Some(current_index),
                sound_kind: None,
                params: vec![("bundle".to_string(), bundle), ("model".to_string(), model)],
            });
        }
    }
    None
}

fn level_name(line: &SourceContextLine) -> Option<String> {
    match line.tokens.as_slice() {
        [keyword, name, ..] if keyword == "level" => Some(clean_name_token(name)),
        _ => None,
    }
}

fn level_range(source: &str, context: &SourceContext, start_index: usize) -> (usize, usize, usize) {
    let line = &context.lines[start_index];
    let header_end = line_end(line);
    if let Some(open_index) = source[line.start..header_end]
        .find('{')
        .map(|offset| line.start + offset)
    {
        let end = find_matching_brace(source, open_index)
            .map(|index| index + 1)
            .unwrap_or(header_end);
        return (end, open_index + 1, end.saturating_sub(1));
    }

    let body_start = header_end;
    let mut end = header_end;
    let mut body_end = body_start;
    for next in context.lines.iter().skip(start_index + 1) {
        if level_name(next).is_some() {
            break;
        }
        let next_trimmed = code_trim(&next.content);
        let in_level = matches!(
            next.scope,
            Some(SourceScope::Level | SourceScope::UnbracedLevel | SourceScope::Legend)
        );
        if !in_level && !next_trimmed.is_empty() {
            break;
        }
        end = line_end(next);
        if !next_trimmed.is_empty() {
            body_end = line_end(next);
        }
        if next.scope == Some(SourceScope::UnbracedLevel) && next_trimmed.is_empty() {
            break;
        }
    }
    (end, body_start, body_end)
}

fn resolve_sprite_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    let sprite3d_blocks = sprite3d_blocks(source, context);
    for (index, line) in context.lines.iter().enumerate() {
        if !sprite_header_scope(line.scope) {
            continue;
        }
        if sprite3d_blocks
            .iter()
            .any(|block| line.start > block.open_index && line.start < block.close_index)
        {
            continue;
        }
        let Some(name) = sprite_name(line) else {
            continue;
        };
        let line_end = line_end(line);
        if let Some(open_index) = source[line.start..line_end]
            .find('{')
            .map(|offset| line.start + offset)
        {
            let end = find_matching_brace(source, open_index)
                .map(|index| index + 1)
                .unwrap_or_else(|| sprite_range_fallback_end(context, index));
            if cursor < line.start || cursor > end {
                continue;
            }
            return Some(SourceTarget {
                kind: SourceTargetKind::Sprite,
                name,
                start: line.start,
                end,
                body_start: Some(open_index + 1),
                body_end: Some(end.saturating_sub(1)),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
            });
        }
        let Some((end, body_start, body_end)) = unbraced_sprite_range(context, index) else {
            continue;
        };
        if cursor < line.start || cursor > end {
            continue;
        }
        return Some(SourceTarget {
            kind: SourceTargetKind::Sprite,
            name,
            start: line.start,
            end,
            body_start: Some(body_start),
            body_end: Some(body_end),
            level_index: None,
            sound_kind: None,
            params: Vec::new(),
        });
    }
    None
}

#[derive(Clone, Copy, Debug)]
struct Sprite3dBlock {
    open_index: usize,
    close_index: usize,
}

#[derive(Clone, Debug)]
struct Level3dBlock {
    open_index: usize,
    close_index: usize,
    bundle: String,
    model: String,
}

fn level3d_blocks(source: &str, context: &SourceContext) -> Vec<Level3dBlock> {
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "levels3"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            let (bundle, model) = parse_levels3_tokens(&line.tokens);
            Some(Level3dBlock {
                open_index,
                close_index,
                bundle,
                model,
            })
        })
        .collect()
}

fn parse_levels3_tokens(tokens: &[String]) -> (String, String) {
    let bundle = tokens
        .get(1)
        .filter(|token| token.as_str() != "of" && token.as_str() != "{")
        .cloned()
        .unwrap_or_else(|| "levels".to_string());
    let model = tokens
        .windows(2)
        .find_map(|pair| (pair[0] == "of").then(|| pair[1].clone()))
        .unwrap_or_default();
    (clean_name_token(&bundle), clean_name_token(&model))
}

fn resolve_sprite3d_target(
    source: &str,
    context: &SourceContext,
    cursor: usize,
) -> Option<SourceTarget> {
    for block in sprite3d_blocks(source, context) {
        if cursor <= block.open_index || cursor >= block.close_index {
            continue;
        }
        for (index, line) in context.lines.iter().enumerate() {
            if line.start <= block.open_index || line.start >= block.close_index {
                continue;
            }
            let Some(name) = sprite3d_name(line) else {
                continue;
            };
            if !next_sprite3d_palette_line(context, index, block) {
                continue;
            }
            let start = line.start;
            let body_start = line_end(line);
            let mut end = body_start;
            let mut body_end = body_start;
            for (next_index, next) in context.lines.iter().enumerate().skip(index + 1) {
                if next.start >= block.close_index {
                    break;
                }
                if sprite3d_name(next).is_some()
                    && next_sprite3d_palette_line(context, next_index, block)
                {
                    break;
                }
                end = line_end(next);
                if !code_trim(&next.content).is_empty() {
                    body_end = line_end(next);
                }
            }
            if cursor < start || cursor > end {
                continue;
            }
            return Some(SourceTarget {
                kind: SourceTargetKind::Sprite3d,
                name,
                start,
                end,
                body_start: Some(body_start),
                body_end: Some(body_end),
                level_index: None,
                sound_kind: None,
                params: Vec::new(),
            });
        }
    }
    None
}

fn sprite3d_blocks(source: &str, context: &SourceContext) -> Vec<Sprite3dBlock> {
    context
        .lines
        .iter()
        .filter(|line| line.tokens.first().is_some_and(|token| token == "sprites3"))
        .filter_map(|line| {
            let header_end = line_end(line);
            let open_index = source[line.start..header_end]
                .find('{')
                .map(|offset| line.start + offset)?;
            let close_index = find_matching_brace(source, open_index)?;
            Some(Sprite3dBlock {
                open_index,
                close_index,
            })
        })
        .collect()
}

fn sprite3d_name(line: &SourceContextLine) -> Option<String> {
    let [name] = line.tokens.as_slice() else {
        return None;
    };
    if sprite_definition_name_token(name) && !is_sprite_color_row(code_trim(&line.content)) {
        return Some(clean_name_token(name));
    }
    None
}

fn next_sprite3d_palette_line(
    context: &SourceContext,
    start_index: usize,
    block: Sprite3dBlock,
) -> bool {
    context
        .lines
        .iter()
        .skip(start_index + 1)
        .take_while(|line| line.start < block.close_index)
        .find_map(|line| {
            let trimmed = code_trim(&line.content);
            (!trimmed.is_empty()).then_some(is_sprite_color_row(trimmed))
        })
        .unwrap_or(false)
}

fn sprite_header_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(SourceScope::Visuals | SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
    )
}

fn sprite_name(line: &SourceContextLine) -> Option<String> {
    match line.tokens.as_slice() {
        [keyword, table_ref, ..] if keyword == "shape" || keyword == "colors" => {
            Some(clean_table_name(table_ref))
        }
        [first, ..]
            if first != "}"
                && first != "{"
                && !first.contains('=')
                && (line.content.trim_end().ends_with('{') || is_unbraced_sprite_header(line)) =>
        {
            Some(clean_table_name(first))
        }
        _ => None,
    }
}

fn clean_table_name(value: &str) -> String {
    clean_name_token(value.split_once(':').map_or(value, |(name, _)| name))
}

fn clean_name_token(value: &str) -> String {
    value
        .trim_matches(|ch: char| matches!(ch, '{' | '}' | '"' | '\''))
        .to_string()
}

fn unbraced_sprite_range(
    context: &SourceContext,
    start_index: usize,
) -> Option<(usize, usize, usize)> {
    let line = &context.lines[start_index];
    if !is_unbraced_sprite_header(line) {
        return None;
    }
    let next_content = context.lines.iter().skip(start_index + 1).find(|next| {
        next.scope == Some(SourceScope::Visuals) && !code_trim(&next.content).is_empty()
    })?;
    if !is_sprite_color_row(code_trim(&next_content.content)) {
        return None;
    }
    let body_start = line_end(line);
    let mut end = body_start;
    let mut body_end = body_start;
    for next in context.lines.iter().skip(start_index + 1) {
        if next.scope != Some(SourceScope::Visuals) {
            break;
        }
        let trimmed = code_trim(&next.content);
        if !trimmed.is_empty()
            && next.start != next_content.start
            && is_visual_sprite_entry_boundary(
                next,
                context
                    .lines
                    .iter()
                    .skip_while(|line| line.start < next.start),
            )
        {
            break;
        }
        end = line_end(next);
        if !trimmed.is_empty() {
            body_end = line_end(next);
        }
    }
    (body_end > body_start).then_some((end, body_start, body_end))
}

fn is_unbraced_sprite_header(line: &SourceContextLine) -> bool {
    matches!(line.scope, Some(SourceScope::Visuals))
        && matches!(line.tokens.as_slice(), [name] if sprite_definition_name_token(name))
        && !line.content.trim_end().ends_with('{')
}

fn is_visual_sprite_entry_boundary<'a>(
    line: &SourceContextLine,
    following: impl Iterator<Item = &'a SourceContextLine>,
) -> bool {
    if !matches!(line.scope, Some(SourceScope::Visuals)) {
        return false;
    }
    match line.tokens.as_slice() {
        [keyword, ..] if keyword == "colors" || keyword == "palettes" || keyword == "shapes" => {
            true
        }
        [selector, source]
            if sprite_definition_name_token(selector)
                && !is_sprite_color(selector)
                && (is_visual_image_source(source)
                    || is_sprite_color(source)
                    || sprite_definition_name_token(source)) =>
        {
            true
        }
        [selector] if sprite_definition_name_token(selector) => following
            .skip(1)
            .find(|next| {
                next.scope == Some(SourceScope::Visuals) && !code_trim(&next.content).is_empty()
            })
            .is_some_and(|next| {
                let next_trimmed = code_trim(&next.content);
                is_visual_image_source(next_trimmed) || is_sprite_color_row(next_trimmed)
            }),
        _ => false,
    }
}

fn sprite_definition_name_token(value: &str) -> bool {
    let cleaned = value.trim_start_matches('@');
    let Some(first) = cleaned.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && cleaned
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':'))
}

fn is_sprite_color_row(line: &str) -> bool {
    let colors = line.split_whitespace().collect::<Vec<_>>();
    !colors.is_empty() && colors.iter().all(|color| is_sprite_color(color))
}

fn is_sprite_color(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower == "transparent"
        || lower == "currentcolor"
        || matches!(
            lower.as_str(),
            "black"
                | "silver"
                | "gray"
                | "white"
                | "maroon"
                | "red"
                | "purple"
                | "fuchsia"
                | "green"
                | "lime"
                | "olive"
                | "yellow"
                | "navy"
                | "blue"
                | "teal"
                | "aqua"
                | "orange"
        )
        || is_hex_color(value)
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn sprite_range_fallback_end(context: &SourceContext, start_index: usize) -> usize {
    context
        .lines
        .iter()
        .skip(start_index + 1)
        .take_while(|line| {
            matches!(
                line.scope,
                Some(SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
            ) || code_trim(&line.content).is_empty()
        })
        .last()
        .map_or_else(|| line_end(&context.lines[start_index]), line_end)
}

fn line_end(line: &SourceContextLine) -> usize {
    line.start + line.content.len()
}

fn code_trim(line: &str) -> &str {
    strip_line_comment(line).trim()
}

fn find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None::<char>;
    let mut escaped = false;
    let mut in_comment = false;
    let mut iter = source[open_index..].char_indices().peekable();
    while let Some((relative, ch)) = iter.next() {
        let index = open_index + relative;
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if let Some(quote_ch) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_ch {
                quote = None;
            }
            continue;
        }
        if ch == '/' && iter.peek().is_some_and(|(_, next)| *next == '/') {
            in_comment = true;
            iter.next();
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn push_json_string(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    push_json_string_value(out, value);
}

fn push_json_string_value(out: &mut String, value: &str) {
    out.push('"');
    escape_json_string(out, value);
    out.push('"');
}

fn escape_json_string(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundSourceTargetKind, SourceTargetKind, resolve_source_target};

    #[test]
    fn resolves_level_body_to_named_level() {
        let source = r#"
levels microban of sokoban {
level microban_01
#####
#@$.#
#####

level microban_02
#####
#.@ #
#####
}
"#;
        let cursor = source.find("#@$.#").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Level);
        assert_eq!(target.name, "microban_01");
        assert_eq!(target.level_index, Some(0));
    }

    #[test]
    fn resolves_levels3_body_to_3d_level() {
        let source = r#"
levels3 basic of push3d {
level push3d_01 {
___
_P_
}
}
"#;
        let cursor = source.find("_P_").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Level3d);
        assert_eq!(target.name, "push3d_01");
        assert_eq!(target.level_index, Some(0));
        assert!(
            target
                .params
                .iter()
                .any(|param| param == &("bundle".to_string(), "basic".to_string()))
        );
    }

    #[test]
    fn resolves_sound_definition_with_params() {
        let source = r#"
sounds {
sfx clear seed=clear01 type=jump
music music_name seed=test1 bars=8 height=0 bpm=100
}
"#;
        let cursor = source.find("height=0").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sounds);
        assert_eq!(target.name, "music_name");
        assert_eq!(target.sound_kind, Some(SoundSourceTargetKind::Music));
        assert!(
            target
                .params
                .iter()
                .any(|param| param == &("bpm".to_string(), "100".to_string()))
        );
    }

    #[test]
    fn resolves_sprite_entry_body() {
        let source = r#"
sprites {
Player {
#000 #fff
.0.
111
}
}
"#;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        assert!(target.body_start.unwrap() < cursor);
        assert!(target.body_end.unwrap() > cursor);
    }

    #[test]
    fn resolves_unbraced_at_sprite_before_next_at_sprite() {
        let source = r##"
sprites {
@Floor
#cfcfcf #c5c5c5
00000
00100

@Shade
#0002
.....
}
"##;
        let cursor = source.find("00100").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "@Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#cfcfcf"));
        assert!(!body.contains("@Shade"));
    }

    #[test]
    fn resolves_unbraced_sprite_before_line_style_image_sprite() {
        let source = r##"
sprites {
Player
#000 #fff
.0.
111
Box sprites/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box sprites/box.png"));
    }

    #[test]
    fn resolves_unbraced_sprite_before_split_line_image_sprite() {
        let source = r##"
sprites {
Player
#000 #fff
.0.
111
Box
sprites/box.png
}
"##;
        let cursor = source.find(".0.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite);
        assert_eq!(target.name, "Player");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("111"));
        assert!(!body.contains("Box"));
        assert!(!body.contains("sprites/box.png"));
    }

    #[test]
    fn resolves_sprites3_entry_as_sprite3d() {
        let source = r##"
sprites3 basic of push3d {
Floor
#90ee90 #008000
.....
..0..

11111

Goal
#00008b
.....
.000.
}
"##;
        let cursor = source.find("..0..").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Floor");
        let body = &source[target.body_start.unwrap()..target.body_end.unwrap()];
        assert!(body.contains("#90ee90"));
        assert!(body.contains("11111"));
        assert!(!body.contains("Goal"));
    }

    #[test]
    fn resolves_second_sprites3_entry_as_sprite3d() {
        let source = r##"
sprites3 basic {
Floor
#90ee90
.....

Goal
#00008b
.000.
}
"##;
        let cursor = source.find(".000.").unwrap();
        let target = resolve_source_target(source, cursor).unwrap();

        assert_eq!(target.kind, SourceTargetKind::Sprite3d);
        assert_eq!(target.name, "Goal");
    }
}
