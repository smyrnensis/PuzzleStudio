use crate::{block_header_text, is_block_header_line, source::split_header_tokens};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisualNodeSyntax {
    pub(crate) selector: Option<String>,
    pub(crate) colors: Option<Vec<String>>,
    pub(crate) colors_body_line: Option<usize>,
    pub(crate) duration: Option<String>,
    pub(crate) duration_body_line: Option<usize>,
    pub(crate) frame_duration: Option<String>,
    pub(crate) frame_duration_body_line: Option<usize>,
    pub(crate) prelude_rows: Vec<String>,
    pub(crate) properties: Vec<(VisualPropertySyntax, String)>,
    pub(crate) property_body_lines: Vec<usize>,
    pub(crate) shape: Option<VisualShapeSyntax>,
    pub(crate) shape_body_line: Option<usize>,
    pub(crate) separator_body_lines: Vec<usize>,
    pub(crate) issues: Vec<VisualSyntaxIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisualPropertySyntax {
    Image(String),
    Sampling(String),
    Translate {
        space: VisualSpaceSyntax,
        value: String,
    },
    Rotate {
        space: VisualSpaceSyntax,
        angle: String,
        from: Option<String>,
        axis: Option<String>,
    },
    Flip(String),
    RemovedOffset,
    Unknown(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VisualSpaceSyntax {
    #[default]
    World,
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisualShapeSyntax {
    Reference(String),
    ExplicitInline(Vec<VisualFrameSyntax>),
    BareFrames(Vec<VisualFrameSyntax>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualFrameSyntax {
    pub(crate) layers: Vec<VisualLayerSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualLayerSyntax {
    pub(crate) rows: Vec<VisualShapeRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualShapeRow {
    pub(crate) text: String,
    pub(crate) body_line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualSyntaxIssue {
    pub(crate) line: String,
    pub(crate) message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualBodyError {
    pub(crate) line: String,
    pub(crate) message: String,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnalyzedVisualBody {
    pub(crate) syntax: VisualNodeSyntax,
    pub(crate) shape: ResolvedVisualShape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualBodyProduct {
    pub(crate) syntax: VisualNodeSyntax,
    pub(crate) shape: ResolvedVisualShape,
    pub(crate) error: Option<VisualBodyError>,
}

#[cfg(test)]
pub(crate) fn analyze_visual_body(
    header: Option<&str>,
    lines: &[impl AsRef<str>],
    is_known_shape: impl FnMut(&str) -> bool,
) -> Result<AnalyzedVisualBody, VisualBodyError> {
    let syntax = parse_visual_node(header, lines);
    analyze_visual_syntax(syntax, is_known_shape)
}

#[cfg(test)]
fn analyze_visual_syntax(
    syntax: VisualNodeSyntax,
    is_known_shape: impl FnMut(&str) -> bool,
) -> Result<AnalyzedVisualBody, VisualBodyError> {
    if let Some(issue) = syntax.issues.first() {
        return Err(VisualBodyError {
            line: issue.line.clone(),
            message: issue.message.to_string(),
        });
    }
    let shape = resolve_visual_shape(&syntax, is_known_shape);
    Ok(AnalyzedVisualBody { syntax, shape })
}

pub(crate) fn analyze_visual_body_product(
    header: Option<&crate::source::LogicalLine>,
    lines: &[crate::source::LogicalLine],
    is_known_shape: impl FnMut(&str) -> bool,
    resolve_display_color: impl FnMut(&str) -> Option<crate::SourceHighlightColor>,
) -> crate::surface::ParseProduct<VisualBodyProduct> {
    let syntax = parse_visual_node(header.map(AsRef::as_ref), lines);
    let shape = resolve_visual_shape(&syntax, is_known_shape);
    let mut error = syntax.issues.first().map(|issue| VisualBodyError {
        line: issue.line.clone(),
        message: issue.message.to_string(),
    });
    if error.is_none()
        && let ResolvedVisualShape::Inline(frames) = &shape
        && let Err(message) = validate_visual_frame_geometry(frames)
    {
        error = Some(VisualBodyError {
            line: lines
                .first()
                .map(|line| line.as_ref().to_string())
                .unwrap_or_default(),
            message: message.to_string(),
        });
    }
    let recognition = recognize_visual_display(
        &syntax,
        Some(&shape),
        header,
        lines,
        resolve_display_color,
    );
    crate::surface::ParseProduct::new(
        VisualBodyProduct {
            syntax,
            shape,
            error,
        },
        recognition,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisualFrameGeometry {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) layers: usize,
}

pub(crate) fn validate_visual_frame_geometry(
    frames: &[VisualFrameSyntax],
) -> Result<VisualFrameGeometry, &'static str> {
    let first_frame = frames.first().ok_or("visual requires at least one frame")?;
    let first_layer = first_frame
        .layers
        .first()
        .ok_or("visual frame requires at least one layer")?;
    let first_row = first_layer
        .rows
        .first()
        .ok_or("visual layer requires at least one row")?;
    let geometry = VisualFrameGeometry {
        width: first_row.text.chars().count(),
        height: first_layer.rows.len(),
        layers: first_frame.layers.len(),
    };
    if geometry.width == 0 {
        return Err("visual row must not be empty");
    }
    for frame in frames {
        if frame.layers.len() != geometry.layers {
            return Err("visual animation frames must have the same size");
        }
        for layer in &frame.layers {
            if layer.rows.len() != geometry.height
                || layer
                    .rows
                    .iter()
                    .any(|row| row.text.chars().count() != geometry.width)
            {
                return Err("visual animation frames must have the same size");
            }
        }
    }
    Ok(geometry)
}

pub(crate) fn parse_visual_shape_rows(
    lines: &[impl AsRef<str>],
) -> Result<Vec<VisualFrameSyntax>, VisualBodyError> {
    let mut frames = vec![empty_visual_frame()];
    let mut issues = Vec::new();
    for (body_line, line) in lines.iter().enumerate() {
        let line = line.as_ref().trim();
        if line.is_empty() {
            continue;
        }
        append_shape_item(line, body_line, &mut frames, &mut issues);
    }
    if let Some(issue) = issues.into_iter().next() {
        return Err(VisualBodyError {
            line: issue.line,
            message: issue.message.to_string(),
        });
    }
    validate_visual_frame_geometry(&frames).map_err(|message| VisualBodyError {
        line: lines
            .first()
            .map(|line| line.as_ref().to_string())
            .unwrap_or_default(),
        message: message.to_string(),
    })?;
    Ok(frames)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisualTiming {
    pub(crate) duration_ms: Option<u64>,
    pub(crate) frame_duration_ms: Option<u64>,
    pub(crate) total_duration_ms: Option<u64>,
}

pub(crate) fn resolve_visual_timing(
    frame_count: usize,
    duration: Option<&str>,
    frame_duration: Option<&str>,
) -> Result<VisualTiming, String> {
    let duration_ms = duration
        .map(|value| puzzle_scene::parse_wait_duration_ms_at(value, value))
        .transpose()
        .map_err(|error| error.to_string())?;
    let frame_duration_ms = frame_duration
        .map(|value| puzzle_scene::parse_wait_duration_ms_at(value, value))
        .transpose()
        .map_err(|error| error.to_string())?;
    if frame_count <= 1 {
        return Ok(VisualTiming {
            duration_ms,
            frame_duration_ms,
            total_duration_ms: duration_ms.or(frame_duration_ms),
        });
    }
    let count = u64::try_from(frame_count)
        .map_err(|_| "visual animation has too many frames".to_string())?;
    let total_duration_ms = match (duration_ms, frame_duration_ms) {
        (None, None) => {
            return Err("visual animation requires duration or frame_duration".to_string());
        }
        (Some(duration), Some(frame_duration)) => {
            let expected = frame_duration
                .checked_mul(count)
                .ok_or_else(|| "visual frame_duration is too large".to_string())?;
            if duration != expected {
                return Err(
                    "visual duration must equal frame_duration multiplied by frame count"
                        .to_string(),
                );
            }
            duration
        }
        (Some(duration), None) => duration,
        (None, Some(frame_duration)) => frame_duration
            .checked_mul(count)
            .ok_or_else(|| "visual frame_duration is too large".to_string())?,
    };
    Ok(VisualTiming {
        duration_ms,
        frame_duration_ms,
        total_duration_ms: Some(total_duration_ms),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisualAttachmentSyntax<Line> {
    pub(crate) header: String,
    pub(crate) header_line: Line,
    pub(crate) body_lines: Vec<Line>,
    pub(crate) closing_line: Option<Line>,
    pub(crate) next_index: usize,
}

pub(crate) fn collect_visual_attachment<Line>(
    lines: &[Line],
    start: usize,
    known_colors: &HashSet<String>,
) -> Result<VisualAttachmentSyntax<Line>, &'static str>
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
                    return Ok(VisualAttachmentSyntax {
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
        return Err("visual attachment missing closing brace");
    }

    let mut body_lines = Vec::new();
    let mut index = start + 1;
    let mut nested_depth = 0i32;
    let mut saw_color_row = visual_header_has_inline_body(&header, known_colors);
    while index < lines.len() {
        if lines[index].as_ref().trim() == "}" && nested_depth == 0 {
            break;
        }
        if nested_depth == 0
            && starts_next_unbraced_visual(lines, index, saw_color_row, known_colors)
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
        if nested_depth == 0 && is_visual_entry_start_color_row(lines[index].as_ref(), known_colors)
        {
            saw_color_row = true;
        }
        index += 1;
    }
    Ok(VisualAttachmentSyntax {
        header,
        header_line,
        body_lines,
        closing_line: None,
        next_index: index,
    })
}

fn starts_next_unbraced_visual<Line>(
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
                && is_visual_definition_name_token(selector)
                && (is_visual_image_source(source)
                    || is_visual_entry_start_color_token(source, known_colors)) =>
        {
            return true;
        }
        [selector] if saw_color_row && is_visual_definition_name_token(selector) => {
            if lines
                .iter()
                .skip(index + 1)
                .map(|line| line.as_ref().trim())
                .find(|line| !line.is_empty())
                .is_some_and(|next| {
                    next != "}"
                        && (is_visual_image_source(next)
                            || is_visual_entry_start_color_row(next, known_colors))
                })
            {
                return true;
            }
        }
        _ => {}
    }
    if is_visual_entry_start_color_row(line, known_colors) {
        return false;
    }
    !saw_color_row && matches!(tokens.as_slice(), [name] if is_visual_definition_name_token(name))
}

fn visual_header_has_inline_body(header: &str, known_colors: &HashSet<String>) -> bool {
    matches!(
        split_header_tokens(block_header_text(header)).as_slice(),
        [selector, source]
            if is_visual_definition_name_token(selector)
                && (is_visual_image_source(source)
                    || is_visual_entry_start_color_token(source, known_colors))
    )
}

fn is_visual_entry_start_color_row(line: &str, known_colors: &HashSet<String>) -> bool {
    let colors = visual_color_row_tokens(line);
    if colors.is_empty() || !colors.iter().all(|color| is_visual_color_expr(color)) {
        return false;
    }
    colors.len() > 1
        || colors.first().is_some_and(|color| {
            is_visual_color(color)
                || is_declared_color_ref(color, known_colors)
                || known_colors.contains(*color)
        })
}

fn visual_color_row_tokens(line: &str) -> Vec<&str> {
    let mut tokens = split_header_tokens(line);
    if tokens.first() == Some(&"colors") {
        tokens.remove(0);
    }
    if tokens.first() == Some(&"=") {
        tokens.remove(0);
    }
    tokens
}

fn is_visual_entry_start_color_token(token: &str, known_colors: &HashSet<String>) -> bool {
    is_visual_color(token)
        || is_declared_color_ref(token, known_colors)
        || known_colors.contains(token)
}

fn is_declared_color_ref(token: &str, known_colors: &HashSet<String>) -> bool {
    token
        .split_once(':')
        .is_some_and(|(name, value)| !value.is_empty() && known_colors.contains(name))
}

fn is_visual_color_expr(value: &str) -> bool {
    is_visual_color(value) || is_visual_color_ref(value)
}

fn is_visual_color(value: &str) -> bool {
    crate::syntax::is_visual_color_literal(value)
}

fn is_visual_color_ref(value: &str) -> bool {
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

fn is_visual_definition_name_token(value: &str) -> bool {
    puzzle_authoring::is_visual_definition_target(value)
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

pub(crate) fn parse_visual_node(
    header: Option<&str>,
    lines: &[impl AsRef<str>],
) -> VisualNodeSyntax {
    let mut syntax = VisualNodeSyntax {
        selector: owner_selector(header),
        ..VisualNodeSyntax::default()
    };
    let mut frames = vec![empty_visual_frame()];
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
                    "visual spatial property block missing closing brace",
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
                        "visual shape cannot contain blank lines; use `-` between Z layers or `>` between frames",
                    );
                    continue;
                }
                if is_removed_colon_translate_syntax(row) {
                    issue(
                        &mut syntax,
                        row,
                        "removed visual translate syntax; use translate (<x>, <y>)",
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
                    "visual shape block missing closing brace",
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
                    "visual shape cannot contain blank lines; use `-` between Z layers or `>` between frames",
                );
            }
            continue;
        }
        if is_removed_colon_translate_syntax(line) {
            issue(
                &mut syntax,
                line,
                "removed visual translate syntax; use translate (<x>, <y>)",
            );
            continue;
        }
        match tokens.as_slice() {
            ["selector", "=", _] | ["selector", _] => {
                issue(
                    &mut syntax,
                    line,
                    "`selector` is not a visual property; write visual <name> {",
                );
            }
            ["colors", "=", values @ ..] | ["colors", values @ ..] if !values.is_empty() => {
                let colors = values.iter().map(|value| (*value).to_string()).collect();
                if syntax.colors.replace(colors).is_some() {
                    issue(&mut syntax, line, "duplicate visual colors");
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
                    "duplicate visual duration",
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
                    "duplicate visual frame_duration",
                    &mut syntax.issues,
                );
            }
            ["shape", "="] => issue(
                &mut syntax,
                line,
                "inline visual shape must be `shape = { ... }` or bare ASCII rows",
            ),
            ["shape", "=", value] | ["shape", value] => {
                if syntax.shape.is_none() {
                    syntax.shape_body_line = Some(line_index);
                }
                set_shape_reference(&mut syntax, value, line)
            }
            [value] if syntax.colors.is_some() && !saw_shape && is_visual_duration_token(value) => {
                syntax.prelude_rows.push(line.to_string());
                if syntax.duration.is_none() {
                    syntax.duration_body_line = Some(line_index);
                }
                set_string_once(
                    &mut syntax.duration,
                    value,
                    line,
                    "duplicate visual duration",
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
                "visual ASCII row must be a single token row",
            ),
        }
    }
    if saw_shape {
        let shape = if explicit_shape {
            VisualShapeSyntax::ExplicitInline(frames)
        } else {
            VisualShapeSyntax::BareFrames(frames)
        };
        if syntax.shape.replace(shape).is_some() {
            issue(&mut syntax, "", "duplicate visual shape");
        }
    }
    syntax
}

fn recognize_visual_display(
    syntax: &VisualNodeSyntax,
    resolved: Option<&ResolvedVisualShape>,
    header: Option<&crate::source::LogicalLine>,
    lines: &[crate::source::LogicalLine],
    mut resolve_display_color: impl FnMut(&str) -> Option<crate::SourceHighlightColor>,
) -> crate::surface::ParserRecognition {
    use crate::surface::{ParserRecognition, SourceSpan, SurfaceDisplayFact};

    let mut recognition = ParserRecognition::default();
    recognize_visual_semantics(syntax, header, lines, &mut recognition);
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
    for frame in resolved_visual_frames(resolved) {
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
                        .push(SurfaceDisplayFact::VisualPixel {
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
                .push(SurfaceDisplayFact::VisualSeparator {
                    span: SourceSpan {
                        start: token.start,
                        end: token.end,
                    },
                });
        }
    }
    recognition
}

fn recognize_visual_semantics(
    syntax: &VisualNodeSyntax,
    header: Option<&crate::source::LogicalLine>,
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
    if let (Some(header), Some(selector)) = (header, syntax.selector.as_deref()) {
        mark_visual_compound(recognition, header, selector, SurfaceSemanticKind::Object);
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
            VisualPropertySyntax::Sampling(value) => {
                mark(recognition, line, value, SurfaceSemanticKind::Literal)
            }
            VisualPropertySyntax::Translate { space, value } => {
                mark(
                    recognition,
                    line,
                    match space {
                        VisualSpaceSyntax::World => "world",
                        VisualSpaceSyntax::Local => "local",
                    },
                    SurfaceSemanticKind::Keyword,
                );
                mark_visual_fragment_tokens(recognition, line, value, SurfaceSemanticKind::Binding);
            }
            VisualPropertySyntax::Rotate {
                space,
                angle,
                from,
                axis,
            } => {
                mark(
                    recognition,
                    line,
                    match space {
                        VisualSpaceSyntax::World => "world",
                        VisualSpaceSyntax::Local => "local",
                    },
                    SurfaceSemanticKind::Keyword,
                );
                mark_visual_fragment_tokens(recognition, line, angle, SurfaceSemanticKind::Binding);
                if let Some(from) = from {
                    mark(recognition, line, "from", SurfaceSemanticKind::Keyword);
                    mark(recognition, line, from, SurfaceSemanticKind::Variant);
                }
                if let Some(axis) = axis {
                    mark(recognition, line, "around", SurfaceSemanticKind::Keyword);
                    mark(recognition, line, axis, SurfaceSemanticKind::Variant);
                }
            }
            VisualPropertySyntax::Flip(value) => {
                mark_visual_fragment_tokens(recognition, line, value, SurfaceSemanticKind::Binding)
            }
            VisualPropertySyntax::Image(value) => {
                mark(recognition, line, value, SurfaceSemanticKind::Asset)
            }
            VisualPropertySyntax::RemovedOffset | VisualPropertySyntax::Unknown(_) => {}
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
    if let (Some(line_index), Some(VisualShapeSyntax::Reference(shape))) =
        (syntax.shape_body_line, &syntax.shape)
        && let Some(line) = lines.get(line_index)
    {
        mark(recognition, line, "shape", SurfaceSemanticKind::Setting);
        mark_visual_compound(recognition, line, shape, SurfaceSemanticKind::Asset);
    }
}

fn mark_visual_fragment_tokens(
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

fn mark_visual_compound(
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

fn resolved_visual_frames(resolved: Option<&ResolvedVisualShape>) -> &[VisualFrameSyntax] {
    match resolved {
        Some(ResolvedVisualShape::Inline(frames)) => frames,
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
pub(crate) enum ResolvedVisualShape {
    None,
    Reference(String),
    Inline(Vec<VisualFrameSyntax>),
    UnknownBareReference(String),
    AmbiguousBareRow(String),
}

pub(crate) fn into_single_layer_frames(
    frames: Vec<VisualFrameSyntax>,
) -> Result<Vec<Vec<VisualShapeRow>>, &'static str> {
    frames
        .into_iter()
        .map(|frame| {
            let [layer] = frame.layers.as_slice() else {
                return Err("2D visual cannot contain `-` Z-layer separators");
            };
            Ok(layer.rows.clone())
        })
        .collect()
}

pub(crate) fn resolve_visual_shape(
    syntax: &VisualNodeSyntax,
    mut is_known_shape: impl FnMut(&str) -> bool,
) -> ResolvedVisualShape {
    let Some(shape) = &syntax.shape else {
        return ResolvedVisualShape::None;
    };
    let frames = match shape {
        VisualShapeSyntax::Reference(reference) => {
            return ResolvedVisualShape::Reference(reference.clone());
        }
        VisualShapeSyntax::ExplicitInline(frames) => {
            return ResolvedVisualShape::Inline(frames.clone());
        }
        VisualShapeSyntax::BareFrames(frames) => frames,
    };
    let [frame] = frames.as_slice() else {
        return ResolvedVisualShape::Inline(frames.clone());
    };
    let [layer] = frame.layers.as_slice() else {
        return ResolvedVisualShape::Inline(frames.clone());
    };
    let [row] = layer.rows.as_slice() else {
        return ResolvedVisualShape::Inline(frames.clone());
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
        (true, true) => ResolvedVisualShape::AmbiguousBareRow(candidate.to_string()),
        (true, false) => ResolvedVisualShape::Reference(candidate.to_string()),
        (false, true) => ResolvedVisualShape::Inline(frames.clone()),
        (false, false) => ResolvedVisualShape::UnknownBareReference(candidate.to_string()),
    }
}

pub(crate) fn is_visual_property_tokens(tokens: &[&str]) -> bool {
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
        ["visual"] | [] => None,
        ["visual", selector] => Some((*selector).to_string()),
        [selector] => Some((*selector).to_string()),
        _ => None,
    }
}

fn property_syntax(tokens: &[&str]) -> Option<VisualPropertySyntax> {
    Some(match tokens {
        ["image", "=", source] | ["image", source] => {
            VisualPropertySyntax::Image((*source).to_string())
        }
        ["sampling", "=", value] | ["sampling", value] => {
            VisualPropertySyntax::Sampling((*value).to_string())
        }
        ["translate", value] => VisualPropertySyntax::Translate {
            space: VisualSpaceSyntax::World,
            value: (*value).to_string(),
        },
        ["translate", "world", value] => VisualPropertySyntax::Translate {
            space: VisualSpaceSyntax::World,
            value: (*value).to_string(),
        },
        ["translate", "local", value] => VisualPropertySyntax::Translate {
            space: VisualSpaceSyntax::Local,
            value: (*value).to_string(),
        },
        ["rotate", angle] => VisualPropertySyntax::Rotate {
            space: VisualSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: None,
            axis: None,
        },
        ["rotate", space @ ("world" | "local"), angle] => VisualPropertySyntax::Rotate {
            space: parse_space(space),
            angle: (*angle).to_string(),
            from: None,
            axis: None,
        },
        ["rotate", angle, "from", from] => VisualPropertySyntax::Rotate {
            space: VisualSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: Some((*from).to_string()),
            axis: None,
        },
        ["rotate", space @ ("world" | "local"), angle, "from", from] => {
            VisualPropertySyntax::Rotate {
                space: parse_space(space),
                angle: (*angle).to_string(),
                from: Some((*from).to_string()),
                axis: None,
            }
        }
        ["rotate", angle, "from", from, "around", axis] => VisualPropertySyntax::Rotate {
            space: VisualSpaceSyntax::World,
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
        ] => VisualPropertySyntax::Rotate {
            space: parse_space(space),
            angle: (*angle).to_string(),
            from: Some((*from).to_string()),
            axis: Some((*axis).to_string()),
        },
        ["rotate", angle, "around", axis] => VisualPropertySyntax::Rotate {
            space: VisualSpaceSyntax::World,
            angle: (*angle).to_string(),
            from: None,
            axis: Some((*axis).to_string()),
        },
        ["rotate", space @ ("world" | "local"), angle, "around", axis] => {
            VisualPropertySyntax::Rotate {
                space: parse_space(space),
                angle: (*angle).to_string(),
                from: None,
                axis: Some((*axis).to_string()),
            }
        }
        ["flip", value] => VisualPropertySyntax::Flip((*value).to_string()),
        ["offset", ..] => VisualPropertySyntax::RemovedOffset,
        [property, ..] if is_visual_property_tokens(tokens) => {
            VisualPropertySyntax::Unknown((*property).to_string())
        }
        _ => return None,
    })
}

fn parse_space(value: &str) -> VisualSpaceSyntax {
    if value == "local" {
        VisualSpaceSyntax::Local
    } else {
        VisualSpaceSyntax::World
    }
}

fn block_spatial_property(
    name: &str,
    rows: &[String],
) -> Result<VisualPropertySyntax, &'static str> {
    let mut space = VisualSpaceSyntax::World;
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
            _ => return Err("invalid visual spatial property block"),
        }
    }
    match name {
        "translate" => Ok(VisualPropertySyntax::Translate {
            space,
            value: value.ok_or("translate block requires value")?,
        }),
        "rotate" => Ok(VisualPropertySyntax::Rotate {
            space,
            angle: angle.ok_or("rotate block requires angle")?,
            from,
            axis,
        }),
        _ => unreachable!(),
    }
}

fn set_shape_reference(syntax: &mut VisualNodeSyntax, value: &str, line: &str) {
    if syntax
        .shape
        .replace(VisualShapeSyntax::Reference(value.to_string()))
        .is_some()
    {
        issue(syntax, line, "duplicate visual shape");
    }
}

fn empty_visual_frame() -> VisualFrameSyntax {
    VisualFrameSyntax {
        layers: vec![VisualLayerSyntax { rows: Vec::new() }],
    }
}

fn append_shape_item(
    line: &str,
    body_line: usize,
    frames: &mut Vec<VisualFrameSyntax>,
    issues: &mut Vec<VisualSyntaxIssue>,
) -> bool {
    match split_header_tokens(line).as_slice() {
        [] => false,
        [">"] => {
            frames.push(empty_visual_frame());
            true
        }
        ["-"] => {
            frames
                .last_mut()
                .expect("one frame exists")
                .layers
                .push(VisualLayerSyntax { rows: Vec::new() });
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
                .push(VisualShapeRow {
                    text: (*row).to_string(),
                    body_line,
                });
            false
        }
        _ => {
            issues.push(VisualSyntaxIssue {
                line: line.to_string(),
                message: "visual ASCII row must be a single token row",
            });
            false
        }
    }
}

pub(crate) fn is_visual_duration_token(value: &str) -> bool {
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
    issues: &mut Vec<VisualSyntaxIssue>,
) {
    if slot.replace(value.to_string()).is_some() {
        issues.push(VisualSyntaxIssue {
            line: line.to_string(),
            message,
        });
    }
}

fn issue(syntax: &mut VisualNodeSyntax, line: &str, message: &'static str) {
    syntax.issues.push(VisualSyntaxIssue {
        line: line.to_string(),
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::{
        ResolvedVisualShape, VisualPropertySyntax, VisualShapeSyntax, VisualSpaceSyntax,
        analyze_visual_body, collect_visual_attachment, is_visual_definition_name_token,
        parse_visual_node, resolve_visual_timing, validate_visual_frame_geometry,
    };

    #[test]
    fn attachment_collection_preserves_braced_and_unbraced_surface_forms() {
        let unbraced =
            ["Floor", "#8fcf6f", "0", "", "Wall {", "#333", "0", "}"].map(str::to_string);
        let known_colors = std::collections::HashSet::new();
        let first = collect_visual_attachment(&unbraced, 0, &known_colors).unwrap();
        assert_eq!(first.header, "Floor");
        assert_eq!(first.body_lines, ["#8fcf6f", "0"]);
        assert_eq!(first.next_index, 3);

        let braced = collect_visual_attachment(&unbraced, 4, &known_colors).unwrap();
        assert_eq!(braced.header, "Wall {");
        assert_eq!(braced.body_lines, ["#333", "0"]);
        assert_eq!(braced.next_index, 8);
    }

    #[test]
    fn visual_definition_names_share_the_symbol_name_grammar() {
        for name in ["Floor", "@Floor", "Floor:directions", "@Floor:directions"] {
            assert!(is_visual_definition_name_token(name), "{name}");
        }
        assert!(!is_visual_definition_name_token("@@Floor"));
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
            collect_visual_attachment(&lines, 0, &std::collections::HashSet::new()).unwrap();

        assert_eq!(first.body_lines, ["#fff #000", "shape_You_F"]);
        assert_eq!(first.next_index, 3);
    }

    #[test]
    fn full_header_and_bare_sugar_preserve_distinct_bodies_with_same_selector() {
        let explicit = parse_visual_node(
            Some("visual Player {"),
            &[
                "colors = #fff #000",
                "duration = 500ms",
                "shape = {",
                "010",
                "}",
            ]
            .map(str::to_string),
        );
        let shorthand = parse_visual_node(
            Some("Player {"),
            &["#fff #000", "500ms", "010"].map(str::to_string),
        );
        assert!(explicit.issues.is_empty() && shorthand.issues.is_empty());
        assert_eq!(explicit.selector, shorthand.selector);
        assert_eq!(explicit.colors, shorthand.colors);
        assert_eq!(explicit.duration, shorthand.duration);
        assert!(matches!(
            explicit.shape.as_ref(),
            Some(VisualShapeSyntax::ExplicitInline(_))
        ));
        assert!(matches!(
            shorthand.shape.as_ref(),
            Some(VisualShapeSyntax::BareFrames(_))
        ));
        let rows = |shape: VisualShapeSyntax| match shape {
            VisualShapeSyntax::ExplicitInline(frames) | VisualShapeSyntax::BareFrames(frames) => {
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
    fn selector_property_is_rejected_in_visual_body() {
        let syntax = parse_visual_node(
            Some("visual Player {"),
            &["selector = Player", "colors = #fff"].map(str::to_string),
        );

        assert!(syntax.issues.iter().any(|issue| {
            issue
                .message
                .contains("`selector` is not a visual property")
        }));
    }

    #[test]
    fn analyzed_body_owns_shared_shape_geometry_and_timing() {
        let body = analyze_visual_body(
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
        let ResolvedVisualShape::Inline(frames) = body.shape else {
            panic!("inline frames");
        };
        let geometry = validate_visual_frame_geometry(&frames).unwrap();
        assert_eq!(
            (geometry.width, geometry.height, geometry.layers),
            (2, 2, 1)
        );
        let timing = resolve_visual_timing(
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
        let explicit = parse_visual_node(
            Some("Player {"),
            &["  #fff", "  shape = {", "  0", "  }"].map(str::to_string),
        );
        assert!(explicit.issues.is_empty());
        let reference = parse_visual_node(
            Some("Player {"),
            &["#fff", "shape player_shape"].map(str::to_string),
        );
        assert!(
            matches!(reference.shape, Some(VisualShapeSyntax::Reference(name)) if name == "player_shape")
        );
    }

    #[test]
    fn human_spatial_syntax_uses_world_by_default_and_uniform_local_prefix() {
        let syntax = parse_visual_node(
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
            matches!(&syntax.properties[0].0, VisualPropertySyntax::Translate { space: VisualSpaceSyntax::Local, value } if value == "(1, 0, 0)")
        );
        assert!(
            matches!(&syntax.properties[1].0, VisualPropertySyntax::Rotate { space: VisualSpaceSyntax::World, angle, from: None, axis: Some(axis) } if angle == "45deg" && axis == "(1, 1, 0)")
        );
    }

    #[test]
    fn rotate_from_preserves_angle_origin_and_space() {
        let syntax = parse_visual_node(
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
            matches!(&syntax.properties[0].0, VisualPropertySyntax::Rotate { space: VisualSpaceSyntax::Local, angle, from: Some(from), axis: None } if angle == "directions" && from == "up")
        );
    }

    #[test]
    fn rotate_from_can_name_an_explicit_axis() {
        let syntax = parse_visual_node(
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
            matches!(&syntax.properties[0].0, VisualPropertySyntax::Rotate { space: VisualSpaceSyntax::Local, angle, from: Some(from), axis: Some(axis) } if angle == "horizontal" && from == "front" && axis == "up")
        );
    }

    #[test]
    fn script_spatial_blocks_are_explicit() {
        let syntax = parse_visual_node(
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
            VisualPropertySyntax::Translate {
                space: VisualSpaceSyntax::Local,
                ..
            }
        ));
        assert!(
            matches!(&syntax.properties[1].0, VisualPropertySyntax::Rotate { space: VisualSpaceSyntax::World, axis: Some(axis), .. } if axis == "up")
        );
    }

    #[test]
    fn removed_shape_rotation_syntax_is_not_recognized_as_spatial_rotation() {
        let syntax = parse_visual_node(
            Some("Player {"),
            &["colors = red", "rotate from up", "shape = {", "0", "}"].map(str::to_string),
        );
        assert!(
            matches!(&syntax.properties[0].0, VisualPropertySyntax::Unknown(name) if name == "rotate")
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
            let syntax = parse_visual_node(
                Some("Player {"),
                &lines.into_iter().map(str::to_string).collect::<Vec<_>>(),
            );
            assert!(
                syntax
                    .issues
                    .iter()
                    .any(|issue| { issue.message.contains("removed visual translate syntax") })
            );
        }
    }
}
