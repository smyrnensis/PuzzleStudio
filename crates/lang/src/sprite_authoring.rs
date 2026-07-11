use crate::{block_header_text, is_block_header_line, source::split_header_tokens};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpriteNodeSyntax {
    pub(crate) selector: Option<String>,
    pub(crate) colors: Option<Vec<String>>,
    pub(crate) duration: Option<String>,
    pub(crate) frame_duration: Option<String>,
    pub(crate) prelude_rows: Vec<String>,
    pub(crate) properties: Vec<(SpritePropertySyntax, String)>,
    pub(crate) shape: Option<SpriteShapeSyntax>,
    pub(crate) separator_body_lines: Vec<usize>,
    pub(crate) issues: Vec<SpriteSyntaxIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpritePropertySyntax {
    Image(String),
    Sampling(String),
    Translate(String),
    Rotate { value: String, from: Option<String> },
    RotateFrom(String),
    RotateUsing { map: String, from: String },
    Flip(String),
    RemovedOffset,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SpriteShapeSyntax {
    Reference(String),
    ExplicitInline(Vec<Vec<SpriteShapeRow>>),
    BareFrames(Vec<Vec<SpriteShapeRow>>),
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

pub(crate) fn parse_sprite_node(header: Option<&str>, lines: &[String]) -> SpriteNodeSyntax {
    let mut syntax = SpriteNodeSyntax {
        selector: owner_selector(header),
        ..SpriteNodeSyntax::default()
    };
    let mut frames = vec![Vec::new()];
    let mut explicit_shape = false;
    let mut saw_shape = false;
    let mut i = 0usize;
    while i < lines.len() {
        let original = &lines[i];
        let line = original.trim();
        let line_index = i;
        i += 1;
        let header_tokens = split_header_tokens(block_header_text(line));
        if is_block_header_line(line) && header_tokens.as_slice() == ["shape", "="] {
            explicit_shape = true;
            saw_shape = true;
            let mut closed = false;
            while i < lines.len() {
                let row_index = i;
                let row = lines[i].trim();
                i += 1;
                if row == "}" {
                    closed = true;
                    break;
                }
                if append_shape_row(row, row_index, &mut frames, &mut syntax.issues) {
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
            continue;
        }
        match tokens.as_slice() {
            ["selector", "=", value] | ["selector", value] => {
                syntax.prelude_rows.push(line.to_string());
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
                }
            }
            ["duration", "=", value] | ["duration", value] => {
                syntax.prelude_rows.push(line.to_string());
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
                set_shape_reference(&mut syntax, value, line)
            }
            [value] if syntax.colors.is_some() && !saw_shape && is_sprite_duration_token(value) => {
                syntax.prelude_rows.push(line.to_string());
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
            }
            _ if syntax.colors.is_none() && !saw_shape => {
                syntax.colors = Some(tokens.iter().map(|value| (*value).to_string()).collect());
            }
            [_] => {
                saw_shape = true;
                if append_shape_row(line, line_index, &mut frames, &mut syntax.issues) {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedSpriteShape {
    None,
    Reference(String),
    Inline(Vec<Vec<SpriteShapeRow>>),
    UnknownBareReference(String),
    AmbiguousBareRow(String),
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
    let [row] = frame.as_slice() else {
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
        ["translate", value] => SpritePropertySyntax::Translate((*value).to_string()),
        ["rotate", "from", from] => SpritePropertySyntax::RotateFrom((*from).to_string()),
        ["rotate", "using", map, "from", from] => SpritePropertySyntax::RotateUsing {
            map: (*map).to_string(),
            from: (*from).to_string(),
        },
        ["rotate", value] => SpritePropertySyntax::Rotate {
            value: (*value).to_string(),
            from: None,
        },
        ["rotate", value, "from", from] => SpritePropertySyntax::Rotate {
            value: (*value).to_string(),
            from: Some((*from).to_string()),
        },
        ["flip", value] => SpritePropertySyntax::Flip((*value).to_string()),
        ["offset", ..] => SpritePropertySyntax::RemovedOffset,
        [property, ..] if is_sprite_property_tokens(tokens) => {
            SpritePropertySyntax::Unknown((*property).to_string())
        }
        _ => return None,
    })
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

fn append_shape_row(
    line: &str,
    body_line: usize,
    frames: &mut Vec<Vec<SpriteShapeRow>>,
    issues: &mut Vec<SpriteSyntaxIssue>,
) -> bool {
    match split_header_tokens(line).as_slice() {
        [] => false,
        [">"] => {
            frames.push(Vec::new());
            true
        }
        [row] => {
            frames
                .last_mut()
                .expect("one frame exists")
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
    use super::{SpriteShapeSyntax, parse_sprite_node};

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
                    .flatten()
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
}
