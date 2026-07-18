use crate::{block_header_text, is_block_header_line, source::split_header_tokens};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpriteNodeSyntax {
    pub(crate) selector: Option<String>,
    pub(crate) selector_body_line: Option<usize>,
    pub(crate) colors: Option<Vec<String>>,
    pub(crate) colors_body_line: Option<usize>,
    pub(crate) duration: Option<String>,
    pub(crate) duration_body_line: Option<usize>,
    pub(crate) frame_duration: Option<String>,
    pub(crate) frame_duration_body_line: Option<usize>,
    pub(crate) prelude_rows: Vec<String>,
    pub(crate) properties: Vec<(SpritePropertySyntax, String)>,
    pub(crate) property_body_lines: Vec<usize>,
    pub(crate) shape: Option<SpriteShapeSyntax>,
    pub(crate) shape_body_line: Option<usize>,
    pub(crate) separator_body_lines: Vec<usize>,
    pub(crate) issues: Vec<SpriteSyntaxIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpritePropertySyntax {
    Image(String),
    Sampling(String),
    Translate {
        space: SpriteSpaceSyntax,
        value: String,
    },
    Rotate {
        space: SpriteSpaceSyntax,
        angle: String,
        from: Option<String>,
        axis: Option<String>,
    },
    Flip(String),
    RemovedOffset,
    Unknown(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SpriteSpaceSyntax {
    #[default]
    World,
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpriteShapeSyntax {
    Reference(String),
    ExplicitInline(Vec<SpriteFrameSyntax>),
    BareFrames(Vec<SpriteFrameSyntax>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteFrameSyntax {
    pub(crate) layers: Vec<SpriteLayerSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteLayerSyntax {
    pub(crate) rows: Vec<SpriteShapeRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteShapeRow {
    pub(crate) text: String,
    pub(crate) body_line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteSyntaxIssue {
    pub(crate) line: String,
    pub(crate) message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteBodyError {
    pub(crate) line: String,
    pub(crate) message: String,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnalyzedSpriteBody {
    pub(crate) syntax: SpriteNodeSyntax,
    pub(crate) shape: ResolvedSpriteShape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteBodyProduct {
    pub(crate) syntax: SpriteNodeSyntax,
    pub(crate) shape: ResolvedSpriteShape,
    pub(crate) error: Option<SpriteBodyError>,
}

#[cfg(test)]
pub(crate) fn analyze_sprite_body(
    header: Option<&str>,
    lines: &[impl AsRef<str>],
    is_known_shape: impl FnMut(&str) -> bool,
) -> Result<AnalyzedSpriteBody, SpriteBodyError> {
    let syntax = parse_sprite_node(header, lines);
    analyze_sprite_syntax(syntax, is_known_shape)
}

#[cfg(test)]
fn analyze_sprite_syntax(
    syntax: SpriteNodeSyntax,
    is_known_shape: impl FnMut(&str) -> bool,
) -> Result<AnalyzedSpriteBody, SpriteBodyError> {
    if let Some(issue) = syntax.issues.first() {
        return Err(SpriteBodyError {
            line: issue.line.clone(),
            message: issue.message.to_string(),
        });
    }
    let shape = resolve_sprite_shape(&syntax, is_known_shape);
    Ok(AnalyzedSpriteBody { syntax, shape })
}

pub(crate) fn analyze_sprite_body_product(
    header: Option<&str>,
    lines: &[crate::source::LogicalLine],
    is_known_shape: impl FnMut(&str) -> bool,
    resolve_display_color: impl FnMut(&str) -> Option<crate::SourceHighlightColor>,
) -> crate::surface::ParseProduct<SpriteBodyProduct> {
    let syntax = parse_sprite_node(header, lines);
    let shape = resolve_sprite_shape(&syntax, is_known_shape);
    let mut error = syntax.issues.first().map(|issue| SpriteBodyError {
        line: issue.line.clone(),
        message: issue.message.to_string(),
    });
    if error.is_none()
        && let ResolvedSpriteShape::Inline(frames) = &shape
        && let Err(message) = validate_sprite_frame_geometry(frames)
    {
        error = Some(SpriteBodyError {
            line: lines
                .first()
                .map(|line| line.as_ref().to_string())
                .unwrap_or_default(),
            message: message.to_string(),
        });
    }
    let recognition = recognize_sprite_display(&syntax, Some(&shape), lines, resolve_display_color);
    crate::surface::ParseProduct::new(
        SpriteBodyProduct {
            syntax,
            shape,
            error,
        },
        recognition,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpriteFrameGeometry {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) layers: usize,
}

pub(crate) fn validate_sprite_frame_geometry(
    frames: &[SpriteFrameSyntax],
) -> Result<SpriteFrameGeometry, &'static str> {
    let first_frame = frames.first().ok_or("sprite requires at least one frame")?;
    let first_layer = first_frame
        .layers
        .first()
        .ok_or("sprite frame requires at least one layer")?;
    let first_row = first_layer
        .rows
        .first()
        .ok_or("sprite layer requires at least one row")?;
    let geometry = SpriteFrameGeometry {
        width: first_row.text.chars().count(),
        height: first_layer.rows.len(),
        layers: first_frame.layers.len(),
    };
    if geometry.width == 0 {
        return Err("sprite row must not be empty");
    }
    for frame in frames {
        if frame.layers.len() != geometry.layers {
            return Err("sprite animation frames must have the same size");
        }
        for layer in &frame.layers {
            if layer.rows.len() != geometry.height
                || layer
                    .rows
                    .iter()
                    .any(|row| row.text.chars().count() != geometry.width)
            {
                return Err("sprite animation frames must have the same size");
            }
        }
    }
    Ok(geometry)
}

pub(crate) fn parse_sprite_shape_rows(
    lines: &[impl AsRef<str>],
) -> Result<Vec<SpriteFrameSyntax>, SpriteBodyError> {
    let mut frames = vec![empty_sprite_frame()];
    let mut issues = Vec::new();
    for (body_line, line) in lines.iter().enumerate() {
        let line = line.as_ref().trim();
        if line.is_empty() {
            continue;
        }
        append_shape_item(line, body_line, &mut frames, &mut issues);
    }
    if let Some(issue) = issues.into_iter().next() {
        return Err(SpriteBodyError {
            line: issue.line,
            message: issue.message.to_string(),
        });
    }
    validate_sprite_frame_geometry(&frames).map_err(|message| SpriteBodyError {
        line: lines
            .first()
            .map(|line| line.as_ref().to_string())
            .unwrap_or_default(),
        message: message.to_string(),
    })?;
    Ok(frames)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpriteTiming {
    pub(crate) duration_ms: Option<u64>,
    pub(crate) frame_duration_ms: Option<u64>,
    pub(crate) total_duration_ms: Option<u64>,
}

pub(crate) fn resolve_sprite_timing(
    frame_count: usize,
    duration: Option<&str>,
    frame_duration: Option<&str>,
) -> Result<SpriteTiming, String> {
    let duration_ms = duration
        .map(|value| puzzle_scene::parse_wait_duration_ms_at(value, value))
        .transpose()
        .map_err(|error| error.to_string())?;
    let frame_duration_ms = frame_duration
        .map(|value| puzzle_scene::parse_wait_duration_ms_at(value, value))
        .transpose()
        .map_err(|error| error.to_string())?;
    if frame_count <= 1 {
        return Ok(SpriteTiming {
            duration_ms,
            frame_duration_ms,
            total_duration_ms: duration_ms.or(frame_duration_ms),
        });
    }
    let count = u64::try_from(frame_count)
        .map_err(|_| "sprite animation has too many frames".to_string())?;
    let total_duration_ms = match (duration_ms, frame_duration_ms) {
        (None, None) => {
            return Err("sprite animation requires duration or frame_duration".to_string());
        }
        (Some(duration), Some(frame_duration)) => {
            let expected = frame_duration
                .checked_mul(count)
                .ok_or_else(|| "sprite frame_duration is too large".to_string())?;
            if duration != expected {
                return Err(
                    "sprite duration must equal frame_duration multiplied by frame count"
                        .to_string(),
                );
            }
            duration
        }
        (Some(duration), None) => duration,
        (None, Some(frame_duration)) => frame_duration
            .checked_mul(count)
            .ok_or_else(|| "sprite frame_duration is too large".to_string())?,
    };
    Ok(SpriteTiming {
        duration_ms,
        frame_duration_ms,
        total_duration_ms: Some(total_duration_ms),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpriteAttachmentSyntax<Line> {
    pub(crate) header: String,
    pub(crate) header_line: Line,
    pub(crate) body_lines: Vec<Line>,
    pub(crate) closing_line: Option<Line>,
    pub(crate) next_index: usize,
}

pub(crate) fn collect_sprite_attachment<Line>(
    lines: &[Line],
    start: usize,
    known_colors: &HashSet<String>,
) -> Result<SpriteAttachmentSyntax<Line>, &'static str>
where
    Line: AsRef<str> + Clone,
{
    let header = lines[start].as_ref().to_string();
    let header_line = lines[start].clone();
    if is_block_header_line(&header) {
        let mut body_lines = Vec::new();
        let mut depth = 0usize;
        let mut index = start + 1;
        while index < lines.len() {
            if lines[index].as_ref().trim() == "}" {
                if depth == 0 {
                    return Ok(SpriteAttachmentSyntax {
                        header,
                        header_line,
                        body_lines,
                        closing_line: Some(lines[index].clone()),
                        next_index: index + 1,
                    });
                }
                depth -= 1;
                body_lines.push(lines[index].clone());
                index += 1;
                continue;
            }
            body_lines.push(lines[index].clone());
            if is_block_header_line(lines[index].as_ref()) {
                depth += 1;
            }
            index += 1;
        }
        return Err("sprite attachment missing closing brace");
    }

    let mut body_lines = Vec::new();
    let mut index = start + 1;
    let mut nested_depth = 0i32;
    let mut saw_color_row = sprite_header_has_inline_body(&header, known_colors);
    while index < lines.len() {
        if lines[index].as_ref().trim() == "}" && nested_depth == 0 {
            break;
        }
        if nested_depth == 0
            && starts_next_unbraced_sprite(lines, index, saw_color_row, known_colors)
        {
            break;
        }
        if split_header_tokens(lines[index].as_ref()).is_empty() {
            if nested_depth > 0 {
                body_lines.push(lines[index].clone());
                index += 1;
                continue;
            }
            break;
        }
        if lines[index].as_ref().trim() == "}" {
            nested_depth -= 1;
        }
        body_lines.push(lines[index].clone());
        if is_block_header_line(lines[index].as_ref()) {
            nested_depth += 1;
        }
        if nested_depth == 0 && is_sprite_entry_start_color_row(lines[index].as_ref(), known_colors)
        {
            saw_color_row = true;
        }
        index += 1;
    }
    Ok(SpriteAttachmentSyntax {
        header,
        header_line,
        body_lines,
        closing_line: None,
        next_index: index,
    })
}

fn starts_next_unbraced_sprite<Line>(
    lines: &[Line],
    index: usize,
    saw_color_row: bool,
    known_colors: &HashSet<String>,
) -> bool
where
    Line: AsRef<str>,
{
    let line = lines[index].as_ref();
    let tokens = split_header_tokens(block_header_text(line));
    if matches!(tokens.first(), Some(&("palette" | "shapes"))) && is_block_header_line(line) {
        return true;
    }
    match tokens.as_slice() {
        [selector, source]
            if saw_color_row
                && is_sprite_definition_name_token(selector)
                && (is_visual_image_source(source)
                    || is_sprite_entry_start_color_token(source, known_colors)) =>
        {
            return true;
        }
        [selector] if saw_color_row && is_sprite_definition_name_token(selector) => {
            if lines
                .iter()
                .skip(index + 1)
                .map(|line| line.as_ref().trim())
                .find(|line| !line.is_empty())
                .is_some_and(|next| {
                    next != "}"
                        && (is_visual_image_source(next)
                            || is_sprite_entry_start_color_row(next, known_colors))
                })
            {
                return true;
            }
        }
        _ => {}
    }
    if is_sprite_entry_start_color_row(line, known_colors) {
        return false;
    }
    !saw_color_row && matches!(tokens.as_slice(), [name] if is_sprite_definition_name_token(name))
}

fn sprite_header_has_inline_body(header: &str, known_colors: &HashSet<String>) -> bool {
    matches!(
        split_header_tokens(block_header_text(header)).as_slice(),
        [selector, source]
            if is_sprite_definition_name_token(selector)
                && (is_visual_image_source(source)
                    || is_sprite_entry_start_color_token(source, known_colors))
    )
}

fn is_sprite_entry_start_color_row(line: &str, known_colors: &HashSet<String>) -> bool {
    let colors = sprite_color_row_tokens(line);
    if colors.is_empty() || !colors.iter().all(|color| is_sprite_color_expr(color)) {
        return false;
    }
    colors.len() > 1
        || colors.first().is_some_and(|color| {
            is_sprite_color(color)
                || is_declared_color_ref(color, known_colors)
                || known_colors.contains(*color)
        })
}

fn sprite_color_row_tokens(line: &str) -> Vec<&str> {
    let mut tokens = split_header_tokens(line);
    if tokens.first() == Some(&"colors") {
        tokens.remove(0);
    }
    if tokens.first() == Some(&"=") {
        tokens.remove(0);
    }
    tokens
}

fn is_sprite_entry_start_color_token(token: &str, known_colors: &HashSet<String>) -> bool {
    is_sprite_color(token)
        || is_declared_color_ref(token, known_colors)
        || known_colors.contains(token)
}

fn is_declared_color_ref(token: &str, known_colors: &HashSet<String>) -> bool {
    token
        .split_once(':')
        .is_some_and(|(name, value)| !value.is_empty() && known_colors.contains(name))
}

fn is_sprite_color_expr(value: &str) -> bool {
    is_sprite_color(value) || is_sprite_color_ref(value)
}

fn is_sprite_color(value: &str) -> bool {
    crate::syntax::is_visual_color_literal(value)
}

fn is_sprite_color_ref(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier_token(first)
        && parts.all(|part| {
            !part.is_empty()
                && part.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '*' | '(' | ')')
                })
        })
}

fn is_identifier_token(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_sprite_definition_name_token(value: &str) -> bool {
    if matches!(
        value,
        "shape" | "shapes" | "palette" | "colors" | "ascii" | "sprites"
    ) {
        return false;
    }
    let cleaned = value.trim_start_matches('@');
    let Some(first) = cleaned.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && cleaned
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':'))
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".svg")
}

pub(crate) fn parse_sprite_node(
    header: Option<&str>,
    lines: &[impl AsRef<str>],
) -> SpriteNodeSyntax {
    let mut syntax = SpriteNodeSyntax {
        selector: owner_selector(header),
        ..SpriteNodeSyntax::default()
    };
    let mut frames = vec![empty_sprite_frame()];
    let mut explicit_shape = false;
    let mut saw_shape = false;
    let mut i = 0usize;
    while i < lines.len() {
        let original = lines[i].as_ref();
        let line = original.trim();
        let line_index = i;
        i += 1;
        let header_tokens = split_header_tokens(block_header_text(line));
        if is_block_header_line(line)
            && matches!(header_tokens.as_slice(), ["translate"] | ["rotate"])
        {
            let property_name = header_tokens[0];
            let mut rows = Vec::new();
            let mut closed = false;
            while i < lines.len() {
                let row = lines[i].as_ref().trim();
                i += 1;
                if row == "}" {
                    closed = true;
                    break;
                }
                if !row.is_empty() {
                    rows.push(row.to_string());
                }
            }
            if !closed {
                issue(
                    &mut syntax,
                    line,
                    "sprite spatial property block missing closing brace",
                );
                continue;
            }
            match block_spatial_property(property_name, &rows) {
                Ok(property) => {
                    syntax.prelude_rows.push(line.to_string());
                    syntax.properties.push((property, line.to_string()));
                    syntax.property_body_lines.push(line_index);
                }
                Err(message) => issue(&mut syntax, line, message),
            }
            continue;
        }
        if is_block_header_line(line) && header_tokens.as_slice() == ["shape", "="] {
            explicit_shape = true;
            saw_shape = true;
            let mut closed = false;
            while i < lines.len() {
                let row_index = i;
                let row = lines[i].as_ref().trim();
                i += 1;
                if row == "}" {
                    closed = true;
                    break;
                }
                if row.is_empty() {
                    issue(
                        &mut syntax,
                        original,
                        "sprite shape cannot contain blank lines; use `-` between Z layers or `>` between frames",
                    );
                    continue;
                }
                if is_removed_colon_translate_syntax(row) {
                    issue(
                        &mut syntax,
                        row,
                        "removed sprite translate syntax; use translate (<x>, <y>)",
                    );
                    continue;
                }
                if append_shape_item(row, row_index, &mut frames, &mut syntax.issues) {
                    syntax.separator_body_lines.push(row_index);
                }
            }
            if !closed {
                issue(
                    &mut syntax,
                    line,
                    "sprite shape block missing closing brace",
                );
            }
            continue;
        }
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            if saw_shape {
                issue(
                    &mut syntax,
                    original,
                    "sprite shape cannot contain blank lines; use `-` between Z layers or `>` between frames",
                );
            }
            continue;
        }
        if is_removed_colon_translate_syntax(line) {
            issue(
                &mut syntax,
                line,
                "removed sprite translate syntax; use translate (<x>, <y>)",
            );
            continue;
        }
        match tokens.as_slice() {
            ["selector", "=", value] | ["selector", value] => {
                syntax.prelude_rows.push(line.to_string());
                if syntax.selector.is_none() {
                    syntax.selector_body_line = Some(line_index);
                }
                set_string_once(
                    &mut syntax.selector,
                    value,
                    line,
                    "duplicate sprite selector",
                    &mut syntax.issues,
                );
            }
            ["colors", "=", values @ ..] | ["colors", values @ ..] if !values.is_empty() => {
                let colors = values.iter().map(|value| (*value).to_string()).collect();
                if syntax.colors.replace(colors).is_some() {
                    issue(&mut syntax, line, "duplicate sprite colors");
                } else {
                    syntax.colors_body_line = Some(line_index);
                }
            }
            ["duration", "=", value] | ["duration", value] => {
                syntax.prelude_rows.push(line.to_string());
                if syntax.duration.is_none() {
                    syntax.duration_body_line = Some(line_index);
                }
                set_string_once(
                    &mut syntax.duration,
                    value,
                    line,
                    "duplicate sprite duration",
                    &mut syntax.issues,
                );
            }
            ["frame_duration", "=", value] | ["frame_duration", value] => {
                syntax.prelude_rows.push(line.to_string());
                if syntax.frame_duration.is_none() {
                    syntax.frame_duration_body_line = Some(line_index);
                }
                set_string_once(
                    &mut syntax.frame_duration,
                    value,
                    line,
                    "duplicate sprite frame_duration",
                    &mut syntax.issues,
                );
            }
            ["shape", "="] => issue(
                &mut syntax,
                line,
                "inline sprite shape must be `shape = { ... }` or bare ASCII rows",
            ),
            ["shape", "=", value] | ["shape", value] => {
                if syntax.shape.is_none() {
                    syntax.shape_body_line = Some(line_index);
                }
                set_shape_reference(&mut syntax, value, line)
            }
            [value] if syntax.colors.is_some() && !saw_shape && is_sprite_duration_token(value) => {
                syntax.prelude_rows.push(line.to_string());
                if syntax.duration.is_none() {
                    syntax.duration_body_line = Some(line_index);
                }
                set_string_once(
                    &mut syntax.duration,
                    value,
                    line,
                    "duplicate sprite duration",
                    &mut syntax.issues,
                );
            }
            _ if property_syntax(&tokens).is_some() && !saw_shape => {
                syntax.prelude_rows.push(line.to_string());
                syntax.properties.push((
                    property_syntax(&tokens).expect("checked property"),
                    line.to_string(),
                ));
                syntax.property_body_lines.push(line_index);
            }
            _ if syntax.colors.is_none() && !saw_shape => {
                syntax.colors = Some(tokens.iter().map(|value| (*value).to_string()).collect());
                syntax.colors_body_line = Some(line_index);
            }
            [_] => {
                saw_shape = true;
                if append_shape_item(line, line_index, &mut frames, &mut syntax.issues) {
                    syntax.separator_body_lines.push(line_index);
                }
            }
            _ => issue(
                &mut syntax,
                line,
                "sprite ASCII row must be a single token row",
            ),
        }
    }
    if saw_shape {
        let shape = if explicit_shape {
            SpriteShapeSyntax::ExplicitInline(frames)
        } else {
            SpriteShapeSyntax::BareFrames(frames)
        };
        if syntax.shape.replace(shape).is_some() {
            issue(&mut syntax, "", "duplicate sprite shape");
        }
    }
    syntax
}

fn recognize_sprite_display(
    syntax: &SpriteNodeSyntax,
    resolved: Option<&ResolvedSpriteShape>,
    lines: &[crate::source::LogicalLine],
    mut resolve_display_color: impl FnMut(&str) -> Option<crate::SourceHighlightColor>,
) -> crate::surface::ParserRecognition {
    use crate::surface::{ParserRecognition, SourceSpan, SurfaceDisplayFact};

    let mut recognition = ParserRecognition::default();
    recognize_sprite_semantics(syntax, lines, &mut recognition);
    let colors = syntax.colors.as_deref().unwrap_or_default();
    if let Some(line_index) = syntax.colors_body_line
        && let Some(line) = lines.get(line_index)
    {
        let mut token_index = 0;
        for color in colors {
            if let Some((index, token)) = line
                .tokens
                .iter()
                .enumerate()
                .skip(token_index)
                .find(|(_, token)| token.text == *color)
            {
                token_index = index + 1;
                if let Some(color) = resolve_display_color(color) {
                    recognition.display_facts.push(SurfaceDisplayFact::Color {
                        span: SourceSpan {
                            start: token.start,
                            end: token.end,
                        },
                        color,
                    });
                }
            }
        }
    }
    for frame in resolved_sprite_frames(resolved) {
        for layer in &frame.layers {
            for row in &layer.rows {
                let Some(line) = lines.get(row.body_line) else {
                    continue;
                };
                let Some(token) = line.tokens.iter().find(|token| token.text == row.text) else {
                    continue;
                };
                for (byte_offset, pixel) in row.text.char_indices() {
                    let color = if pixel == '.' {
                        crate::SourceHighlightColor::parse("transparent")
                    } else {
                        visual_color_index(pixel)
                            .and_then(|index| colors.get(index))
                            .and_then(|color| resolve_display_color(color))
                    };
                    let transparent = pixel == '.'
                        || color
                            .as_ref()
                            .is_some_and(crate::SourceHighlightColor::is_transparent);
                    recognition
                        .display_facts
                        .push(SurfaceDisplayFact::SpritePixel {
                            span: SourceSpan {
                                start: token.start + byte_offset,
                                end: token.start + byte_offset + pixel.len_utf8(),
                            },
                            transparent,
                            color,
                        });
                }
            }
        }
    }
    for line_index in &syntax.separator_body_lines {
        if let Some(token) = lines.get(*line_index).and_then(|line| line.tokens.first()) {
            recognition
                .display_facts
                .push(SurfaceDisplayFact::SpriteSeparator {
                    span: SourceSpan {
                        start: token.start,
                        end: token.end,
                    },
                });
        }
    }
    recognition
}

fn recognize_sprite_semantics(
    syntax: &SpriteNodeSyntax,
    lines: &[crate::source::LogicalLine],
    recognition: &mut crate::surface::ParserRecognition,
) {
    use crate::surface::{SourceSpan, SurfaceSemanticKind};

    let mark = |recognition: &mut crate::surface::ParserRecognition,
                line: &crate::source::LogicalLine,
                text: &str,
                kind: SurfaceSemanticKind| {
        for token in &line.tokens {
            if token.text == text {
                recognition.mark(
                    SourceSpan {
                        start: token.start,
                        end: token.end,
                    },
                    kind,
                );
            }
        }
    };
    if let (Some(line_index), Some(selector)) = (syntax.selector_body_line, &syntax.selector)
        && let Some(line) = lines.get(line_index)
    {
        mark(recognition, line, "selector", SurfaceSemanticKind::Setting);
        mark_sprite_compound(recognition, line, selector, SurfaceSemanticKind::Object);
    }
    if let Some(line_index) = syntax.colors_body_line
        && let Some(line) = lines.get(line_index)
    {
        mark(recognition, line, "colors", SurfaceSemanticKind::Setting);
        for color in syntax.colors.as_deref().unwrap_or_default() {
            mark(recognition, line, color, SurfaceSemanticKind::Color);
        }
    }
    for (line_index, (property, _)) in syntax
        .property_body_lines
        .iter()
        .copied()
        .zip(&syntax.properties)
    {
        let Some(line) = lines.get(line_index) else {
            continue;
        };
        let Some(keyword) = line.tokens.first().map(|token| token.text.as_str()) else {
            continue;
        };
        mark(recognition, line, keyword, SurfaceSemanticKind::Setting);
        match property {
            SpritePropertySyntax::Sampling(value) => {
                mark(recognition, line, value, SurfaceSemanticKind::Literal)
            }
            SpritePropertySyntax::Translate { space, value } => {
                mark(
                    recognition,
                    line,
                    match space {
                        SpriteSpaceSyntax::World => "world",
                        SpriteSpaceSyntax::Local => "local",
                    },
                    SurfaceSemanticKind::Keyword,
                );
                mark_sprite_fragment_tokens(recognition, line, value, SurfaceSemanticKind::Binding);
            }
            SpritePropertySyntax::Rotate {
                space,
                angle,
                from,
                axis,
            } => {
                mark(
                    recognition,
                    line,
                    match space {
                        SpriteSpaceSyntax::World => "world",
                        SpriteSpaceSyntax::Local => "local",
                    },
                    SurfaceSemanticKind::Keyword,
                );
                mark_sprite_fragment_tokens(recognition, line, angle, SurfaceSemanticKind::Binding);
                if let Some(from) = from {
                    mark(recognition, line, "from", SurfaceSemanticKind::Keyword);
                    mark(recognition, line, from, SurfaceSemanticKind::Variant);
                }
                if let Some(axis) = axis {
                    mark(recognition, line, "around", SurfaceSemanticKind::Keyword);
                    mark(recognition, line, axis, SurfaceSemanticKind::Variant);
                }
            }
            SpritePropertySyntax::Flip(value) => {
                mark_sprite_fragment_tokens(recognition, line, value, SurfaceSemanticKind::Binding)
            }
            SpritePropertySyntax::Image(value) => {
                mark(recognition, line, value, SurfaceSemanticKind::Asset)
            }
            SpritePropertySyntax::RemovedOffset | SpritePropertySyntax::Unknown(_) => {}
        }
    }
    for (line_index, value) in [
        (syntax.duration_body_line, syntax.duration.as_deref()),
        (
            syntax.frame_duration_body_line,
            syntax.frame_duration.as_deref(),
        ),
    ] {
        if let (Some(line_index), Some(value)) = (line_index, value)
            && let Some(line) = lines.get(line_index)
        {
            if let Some(keyword) = line.tokens.first() {
                mark(
                    recognition,
                    line,
                    &keyword.text,
                    SurfaceSemanticKind::Setting,
                );
            }
            if let (Some(first), Some(last)) = (line.tokens.first(), line.tokens.last()) {
                recognition.mark(
                    SourceSpan {
                        start: first.start,
                        end: last.end,
                    },
                    SurfaceSemanticKind::Number,
                );
            } else {
                mark(recognition, line, value, SurfaceSemanticKind::Number);
            }
        }
    }
    if let (Some(line_index), Some(SpriteShapeSyntax::Reference(shape))) =
        (syntax.shape_body_line, &syntax.shape)
        && let Some(line) = lines.get(line_index)
    {
        mark(recognition, line, "shape", SurfaceSemanticKind::Setting);
        mark_sprite_compound(recognition, line, shape, SurfaceSemanticKind::Asset);
    }
}

fn mark_sprite_fragment_tokens(
    recognition: &mut crate::surface::ParserRecognition,
    line: &crate::source::LogicalLine,
    fragment: &str,
    kind: crate::surface::SurfaceSemanticKind,
) {
    for token in &line.tokens {
        if fragment.contains(&token.text) {
            recognition.mark(
                crate::surface::SourceSpan {
                    start: token.start,
                    end: token.end,
                },
                kind,
            );
        }
    }
}

fn mark_sprite_compound(
    recognition: &mut crate::surface::ParserRecognition,
    line: &crate::source::LogicalLine,
    value: &str,
    head_kind: crate::surface::SurfaceSemanticKind,
) {
    for token in &line.tokens {
        if token.text != value {
            continue;
        }
        let mut offset = 0usize;
        for (index, part) in value.split(':').enumerate() {
            recognition.mark(
                crate::surface::SourceSpan {
                    start: token.start + offset,
                    end: token.start + offset + part.len(),
                },
                if index == 0 {
                    head_kind
                } else {
                    crate::surface::SurfaceSemanticKind::Variant
                },
            );
            offset += part.len() + 1;
        }
    }
}

fn resolved_sprite_frames(resolved: Option<&ResolvedSpriteShape>) -> &[SpriteFrameSyntax] {
    match resolved {
        Some(ResolvedSpriteShape::Inline(frames)) => frames,
        _ => &[],
    }
}

fn visual_color_index(token: char) -> Option<usize> {
    (0..62).find(|index| crate::visual_color_token_for_index(*index) == Some(token))
}

fn is_removed_colon_translate_syntax(line: &str) -> bool {
    line.split_whitespace()
        .any(|token| token.starts_with("translate:"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedSpriteShape {
    None,
    Reference(String),
    Inline(Vec<SpriteFrameSyntax>),
    UnknownBareReference(String),
    AmbiguousBareRow(String),
}

pub(crate) fn into_single_layer_frames(
    frames: Vec<SpriteFrameSyntax>,
) -> Result<Vec<Vec<SpriteShapeRow>>, &'static str> {
    frames
        .into_iter()
        .map(|frame| {
            let [layer] = frame.layers.as_slice() else {
                return Err("2D sprite cannot contain `-` Z-layer separators");
            };
            Ok(layer.rows.clone())
        })
        .collect()
}

pub(crate) fn resolve_sprite_shape(
    syntax: &SpriteNodeSyntax,
    mut is_known_shape: impl FnMut(&str) -> bool,
) -> ResolvedSpriteShape {
    let Some(shape) = &syntax.shape else {
        return ResolvedSpriteShape::None;
    };
    let frames = match shape {
        SpriteShapeSyntax::Reference(reference) => {
            return ResolvedSpriteShape::Reference(reference.clone());
        }
        SpriteShapeSyntax::ExplicitInline(frames) => {
            return ResolvedSpriteShape::Inline(frames.clone());
        }
        SpriteShapeSyntax::BareFrames(frames) => frames,
    };
    let [frame] = frames.as_slice() else {
        return ResolvedSpriteShape::Inline(frames.clone());
    };
    let [layer] = frame.layers.as_slice() else {
        return ResolvedSpriteShape::Inline(frames.clone());
    };
    let [row] = layer.rows.as_slice() else {
        return ResolvedSpriteShape::Inline(frames.clone());
    };
    let candidate = row.text.as_str();
    let shape_name = candidate
        .split_once(':')
        .map_or(candidate, |(name, _)| name);
    let known_shape = is_known_shape(shape_name);
    let palette_size = syntax.colors.as_ref().map_or(0, Vec::len);
    let valid_inline_row = candidate.chars().all(|token| {
        token == '.'
            || (0..palette_size)
                .filter_map(crate::visual_color_token_for_index)
                .any(|palette_token| token == palette_token)
    });
    match (known_shape, valid_inline_row) {
        (true, true) => ResolvedSpriteShape::AmbiguousBareRow(candidate.to_string()),
        (true, false) => ResolvedSpriteShape::Reference(candidate.to_string()),
        (false, true) => ResolvedSpriteShape::Inline(frames.clone()),
        (false, false) => ResolvedSpriteShape::UnknownBareReference(candidate.to_string()),
    }
}

pub(crate) fn is_sprite_property_tokens(tokens: &[&str]) -> bool {
    matches!(
        tokens.first().copied(),
        Some(
            "selector"
                | "colors"
                | "duration"
                | "frame_duration"
                | "shape"
                | "image"
                | "contain"
                | "cover"
                | "stretch"
                | "offset"
                | "sampling"
                | "pixels_per_cell"
                | "translate"
                | "rotate"
                | "flip"
        )
    )
}

fn owner_selector(header: Option<&str>) -> Option<String> {
    let header = header?.trim();
    let tokens = split_header_tokens(block_header_text(header));
    match tokens.as_slice() {
        ["sprite"] | [] => None,
        [selector] => Some((*selector).to_string()),
        _ => None,
    }
}

fn property_syntax(tokens: &[&str]) -> Option<SpritePropertySyntax> {
    Some(match tokens {
        ["image", "=", source] | ["image", source] => {
            SpritePropertySyntax::Image((*source).to_string())
        }
        ["sampling", "=", value] | ["sampling", value] => {
            SpritePropertySyntax::Sampling((*value).to_string())
        }
        ["translate", value] => SpritePropertySyntax::Translate {
            space: SpriteSpaceSyntax::World,
            value: (*value).to_string(),
        },
        ["translate", "world", value] => SpritePropertySyntax::Translate {
            space: SpriteSpaceSyntax::World,
            value: (*value).to_string(),
        },
        ["translate", "local", value] => SpritePropertySyntax::Translate {
            space: SpriteSpaceSyntax::Local,
            value: (*value).to_string(),
        },
        ["rotate", angle] => SpritePropertySyntax::Rotate {
            space: SpriteSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: None,
            axis: None,
        },
        ["rotate", space @ ("world" | "local"), angle] => SpritePropertySyntax::Rotate {
            space: parse_space(space),
            angle: (*angle).to_string(),
            from: None,
            axis: None,
        },
        ["rotate", angle, "from", from] => SpritePropertySyntax::Rotate {
            space: SpriteSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: Some((*from).to_string()),
            axis: None,
        },
        ["rotate", space @ ("world" | "local"), angle, "from", from] => {
            SpritePropertySyntax::Rotate {
                space: parse_space(space),
                angle: (*angle).to_string(),
                from: Some((*from).to_string()),
                axis: None,
            }
        }
        ["rotate", angle, "from", from, "around", axis] => SpritePropertySyntax::Rotate {
            space: SpriteSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: Some((*from).to_string()),
            axis: Some((*axis).to_string()),
        },
        [
            "rotate",
            space @ ("world" | "local"),
            angle,
            "from",
            from,
            "around",
            axis,
        ] => SpritePropertySyntax::Rotate {
            space: parse_space(space),
            angle: (*angle).to_string(),
            from: Some((*from).to_string()),
            axis: Some((*axis).to_string()),
        },
        ["rotate", angle, "around", axis] => SpritePropertySyntax::Rotate {
            space: SpriteSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: None,
            axis: Some((*axis).to_string()),
        },
        ["rotate", space @ ("world" | "local"), angle, "around", axis] => {
            SpritePropertySyntax::Rotate {
                space: parse_space(space),
                angle: (*angle).to_string(),
                from: None,
                axis: Some((*axis).to_string()),
            }
        }
        ["flip", value] => SpritePropertySyntax::Flip((*value).to_string()),
        ["offset", ..] => SpritePropertySyntax::RemovedOffset,
        [property, ..] if is_sprite_property_tokens(tokens) => {
            SpritePropertySyntax::Unknown((*property).to_string())
        }
        _ => return None,
    })
}

fn parse_space(value: &str) -> SpriteSpaceSyntax {
    if value == "local" {
        SpriteSpaceSyntax::Local
    } else {
        SpriteSpaceSyntax::World
    }
}

fn block_spatial_property(
    name: &str,
    rows: &[String],
) -> Result<SpritePropertySyntax, &'static str> {
    let mut space = SpriteSpaceSyntax::World;
    let mut value = None;
    let mut angle = None;
    let mut from = None;
    let mut axis = None;
    for row in rows {
        let tokens = split_header_tokens(row);
        match tokens.as_slice() {
            ["space", "=", raw] if matches!(*raw, "world" | "local") => space = parse_space(raw),
            ["value", "=", raw] => value = Some((*raw).to_string()),
            ["angle", "=", raw] => angle = Some((*raw).to_string()),
            ["from", "=", raw] => from = Some((*raw).to_string()),
            ["axis", "=", raw] => axis = Some((*raw).to_string()),
            _ => return Err("invalid sprite spatial property block"),
        }
    }
    match name {
        "translate" => Ok(SpritePropertySyntax::Translate {
            space,
            value: value.ok_or("translate block requires value")?,
        }),
        "rotate" => Ok(SpritePropertySyntax::Rotate {
            space,
            angle: angle.ok_or("rotate block requires angle")?,
            from,
            axis,
        }),
        _ => unreachable!(),
    }
}

fn set_shape_reference(syntax: &mut SpriteNodeSyntax, value: &str, line: &str) {
    if syntax
        .shape
        .replace(SpriteShapeSyntax::Reference(value.to_string()))
        .is_some()
    {
        issue(syntax, line, "duplicate sprite shape");
    }
}

fn empty_sprite_frame() -> SpriteFrameSyntax {
    SpriteFrameSyntax {
        layers: vec![SpriteLayerSyntax { rows: Vec::new() }],
    }
}

fn append_shape_item(
    line: &str,
    body_line: usize,
    frames: &mut Vec<SpriteFrameSyntax>,
    issues: &mut Vec<SpriteSyntaxIssue>,
) -> bool {
    match split_header_tokens(line).as_slice() {
        [] => false,
        [">"] => {
            frames.push(empty_sprite_frame());
            true
        }
        ["-"] => {
            frames
                .last_mut()
                .expect("one frame exists")
                .layers
                .push(SpriteLayerSyntax { rows: Vec::new() });
            true
        }
        [row] => {
            frames
                .last_mut()
                .expect("one frame exists")
                .layers
                .last_mut()
                .expect("one layer exists")
                .rows
                .push(SpriteShapeRow {
                    text: (*row).to_string(),
                    body_line,
                });
            false
        }
        _ => {
            issues.push(SpriteSyntaxIssue {
                line: line.to_string(),
                message: "sprite ASCII row must be a single token row",
            });
            false
        }
    }
}

pub(crate) fn is_sprite_duration_token(value: &str) -> bool {
    value
        .strip_suffix("ms")
        .or_else(|| value.strip_suffix('s'))
        .is_some_and(|number| !number.is_empty() && number.chars().any(|ch| ch.is_ascii_digit()))
}

fn set_string_once(
    slot: &mut Option<String>,
    value: &str,
    line: &str,
    message: &'static str,
    issues: &mut Vec<SpriteSyntaxIssue>,
) {
    if slot.replace(value.to_string()).is_some() {
        issues.push(SpriteSyntaxIssue {
            line: line.to_string(),
            message,
        });
    }
}

fn issue(syntax: &mut SpriteNodeSyntax, line: &str, message: &'static str) {
    syntax.issues.push(SpriteSyntaxIssue {
        line: line.to_string(),
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::{
        ResolvedSpriteShape, SpritePropertySyntax, SpriteShapeSyntax, SpriteSpaceSyntax,
        analyze_sprite_body, collect_sprite_attachment, parse_sprite_node, resolve_sprite_timing,
        validate_sprite_frame_geometry,
    };

    #[test]
    fn attachment_collection_preserves_braced_and_unbraced_surface_forms() {
        let unbraced =
            ["Floor", "#8fcf6f", "0", "", "Wall {", "#333", "0", "}"].map(str::to_string);
        let known_colors = std::collections::HashSet::new();
        let first = collect_sprite_attachment(&unbraced, 0, &known_colors).unwrap();
        assert_eq!(first.header, "Floor");
        assert_eq!(first.body_lines, ["#8fcf6f", "0"]);
        assert_eq!(first.next_index, 3);

        let braced = collect_sprite_attachment(&unbraced, 4, &known_colors).unwrap();
        assert_eq!(braced.header, "Wall {");
        assert_eq!(braced.body_lines, ["#333", "0"]);
        assert_eq!(braced.next_index, 8);
    }

    #[test]
    fn tagged_object_selector_after_shape_reference_is_not_a_color_row() {
        let lines = [
            "You:F",
            "#fff #000",
            "shape_You_F",
            "",
            "You:B",
            "#000 #fff",
            "shape_You_F",
            "",
        ]
        .map(str::to_string);
        let first =
            collect_sprite_attachment(&lines, 0, &std::collections::HashSet::new()).unwrap();

        assert_eq!(first.body_lines, ["#fff #000", "shape_You_F"]);
        assert_eq!(first.next_index, 3);
    }

    #[test]
    fn explicit_and_bare_inline_rows_preserve_distinct_syntax_with_same_content() {
        let explicit = parse_sprite_node(
            Some("sprite {"),
            &[
                "selector = Player",
                "colors = #fff #000",
                "duration = 500ms",
                "shape = {",
                "010",
                "}",
            ]
            .map(str::to_string),
        );
        let shorthand = parse_sprite_node(
            Some("Player {"),
            &["#fff #000", "500ms", "010"].map(str::to_string),
        );
        assert!(explicit.issues.is_empty() && shorthand.issues.is_empty());
        assert_eq!(explicit.selector, shorthand.selector);
        assert_eq!(explicit.colors, shorthand.colors);
        assert_eq!(explicit.duration, shorthand.duration);
        assert!(matches!(
            explicit.shape.as_ref(),
            Some(SpriteShapeSyntax::ExplicitInline(_))
        ));
        assert!(matches!(
            shorthand.shape.as_ref(),
            Some(SpriteShapeSyntax::BareFrames(_))
        ));
        let rows = |shape: SpriteShapeSyntax| match shape {
            SpriteShapeSyntax::ExplicitInline(frames) | SpriteShapeSyntax::BareFrames(frames) => {
                frames
                    .into_iter()
                    .flat_map(|frame| frame.layers)
                    .flat_map(|layer| layer.rows)
                    .map(|row| row.text)
                    .collect::<Vec<_>>()
            }
            _ => panic!("inline"),
        };
        assert_eq!(
            rows(explicit.shape.unwrap()),
            rows(shorthand.shape.unwrap())
        );
    }

    #[test]
    fn analyzed_body_owns_shared_shape_geometry_and_timing() {
        let body = analyze_sprite_body(
            Some("Pulse {"),
            &[
                "colors = red transparent",
                "duration = 240ms",
                "frame_duration = 120ms",
                "shape = {",
                "0.",
                "..",
                ">",
                ".0",
                "..",
                "}",
            ]
            .map(str::to_string),
            |_| false,
        )
        .unwrap();
        let ResolvedSpriteShape::Inline(frames) = body.shape else {
            panic!("inline frames");
        };
        let geometry = validate_sprite_frame_geometry(&frames).unwrap();
        assert_eq!(
            (geometry.width, geometry.height, geometry.layers),
            (2, 2, 1)
        );
        let timing = resolve_sprite_timing(
            frames.len(),
            body.syntax.duration.as_deref(),
            body.syntax.frame_duration.as_deref(),
        )
        .unwrap();
        assert_eq!(timing.duration_ms, Some(240));
        assert_eq!(timing.frame_duration_ms, Some(120));
        assert_eq!(timing.total_duration_ms, Some(240));
    }

    #[test]
    fn indented_explicit_shape_and_bare_reference_are_node_owned() {
        let explicit = parse_sprite_node(
            Some("Player {"),
            &["  #fff", "  shape = {", "  0", "  }"].map(str::to_string),
        );
        assert!(explicit.issues.is_empty());
        let reference = parse_sprite_node(
            Some("Player {"),
            &["#fff", "shape player_shape"].map(str::to_string),
        );
        assert!(
            matches!(reference.shape, Some(SpriteShapeSyntax::Reference(name)) if name == "player_shape")
        );
    }

    #[test]
    fn human_spatial_syntax_uses_world_by_default_and_uniform_local_prefix() {
        let syntax = parse_sprite_node(
            Some("Player {"),
            &[
                "colors = red",
                "translate local (1, 0, 0)",
                "rotate 45deg around (1, 1, 0)",
                "shape = {",
                "0",
                "}",
            ]
            .map(str::to_string),
        );
        assert!(syntax.issues.is_empty(), "{:?}", syntax.issues);
        assert!(
            matches!(&syntax.properties[0].0, SpritePropertySyntax::Translate { space: SpriteSpaceSyntax::Local, value } if value == "(1, 0, 0)")
        );
        assert!(
            matches!(&syntax.properties[1].0, SpritePropertySyntax::Rotate { space: SpriteSpaceSyntax::World, angle, from: None, axis: Some(axis) } if angle == "45deg" && axis == "(1, 1, 0)")
        );
    }

    #[test]
    fn rotate_from_preserves_angle_origin_and_space() {
        let syntax = parse_sprite_node(
            Some("Player:directions {"),
            &[
                "colors = red",
                "rotate local directions from up",
                "shape = {",
                "0",
                "}",
            ]
            .map(str::to_string),
        );
        assert!(syntax.issues.is_empty(), "{:?}", syntax.issues);
        assert!(
            matches!(&syntax.properties[0].0, SpritePropertySyntax::Rotate { space: SpriteSpaceSyntax::Local, angle, from: Some(from), axis: None } if angle == "directions" && from == "up")
        );
    }

    #[test]
    fn rotate_from_can_name_an_explicit_axis() {
        let syntax = parse_sprite_node(
            Some("Player:horizontal {"),
            &[
                "colors = red",
                "rotate local horizontal from front around up",
                "shape = {",
                "0",
                "}",
            ]
            .map(str::to_string),
        );

        assert!(syntax.issues.is_empty(), "{:?}", syntax.issues);
        assert!(
            matches!(&syntax.properties[0].0, SpritePropertySyntax::Rotate { space: SpriteSpaceSyntax::Local, angle, from: Some(from), axis: Some(axis) } if angle == "horizontal" && from == "front" && axis == "up")
        );
    }

    #[test]
    fn script_spatial_blocks_are_explicit() {
        let syntax = parse_sprite_node(
            Some("Player {"),
            &[
                "colors = red",
                "translate {",
                "space = local",
                "value = (1, 0, 0)",
                "}",
                "rotate {",
                "space = world",
                "angle = 45deg",
                "axis = up",
                "}",
                "shape = {",
                "0",
                "}",
            ]
            .map(str::to_string),
        );
        assert!(syntax.issues.is_empty(), "{:?}", syntax.issues);
        assert!(matches!(
            &syntax.properties[0].0,
            SpritePropertySyntax::Translate {
                space: SpriteSpaceSyntax::Local,
                ..
            }
        ));
        assert!(
            matches!(&syntax.properties[1].0, SpritePropertySyntax::Rotate { space: SpriteSpaceSyntax::World, axis: Some(axis), .. } if axis == "up")
        );
    }

    #[test]
    fn removed_shape_rotation_syntax_is_not_recognized_as_spatial_rotation() {
        let syntax = parse_sprite_node(
            Some("Player {"),
            &["colors = red", "rotate from up", "shape = {", "0", "}"].map(str::to_string),
        );
        assert!(
            matches!(&syntax.properties[0].0, SpritePropertySyntax::Unknown(name) if name == "rotate")
        );
    }

    #[test]
    fn removed_colon_translate_syntax_is_rejected_before_shape_classification() {
        for lines in [
            vec!["colors = red", "0", "translate:right"],
            vec![
                "colors = red",
                "shape = {",
                "0",
                "translate:right:2 translate:up:1",
                "}",
            ],
        ] {
            let syntax = parse_sprite_node(
                Some("Player {"),
                &lines.into_iter().map(str::to_string).collect::<Vec<_>>(),
            );
            assert!(
                syntax
                    .issues
                    .iter()
                    .any(|issue| { issue.message.contains("removed sprite translate syntax") })
            );
        }
    }
}
