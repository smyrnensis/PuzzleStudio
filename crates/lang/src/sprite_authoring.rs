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
    Translate {
        space: SpriteSpaceSyntax,
        value: String,
    },
    Rotate {
        space: SpriteSpaceSyntax,
        angle: String,
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

pub(crate) fn parse_sprite_node(header: Option<&str>, lines: &[String]) -> SpriteNodeSyntax {
    let mut syntax = SpriteNodeSyntax {
        selector: owner_selector(header),
        ..SpriteNodeSyntax::default()
    };
    let mut frames = vec![empty_sprite_frame()];
    let mut explicit_shape = false;
    let mut saw_shape = false;
    let mut i = 0usize;
    while i < lines.len() {
        let original = &lines[i];
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
                let row = lines[i].trim();
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
                let row = lines[i].trim();
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
            axis: None,
        },
        ["rotate", space @ ("world" | "local"), angle] => SpritePropertySyntax::Rotate {
            space: parse_space(space),
            angle: (*angle).to_string(),
            axis: None,
        },
        ["rotate", angle, "around", axis] => SpritePropertySyntax::Rotate {
            space: SpriteSpaceSyntax::World,
            angle: (*angle).to_string(),
            axis: Some((*axis).to_string()),
        },
        ["rotate", space @ ("world" | "local"), angle, "around", axis] => {
            SpritePropertySyntax::Rotate {
                space: parse_space(space),
                angle: (*angle).to_string(),
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
    let mut axis = None;
    for row in rows {
        let tokens = split_header_tokens(row);
        match tokens.as_slice() {
            ["space", "=", raw] if matches!(*raw, "world" | "local") => space = parse_space(raw),
            ["value", "=", raw] => value = Some((*raw).to_string()),
            ["angle", "=", raw] => angle = Some((*raw).to_string()),
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
    use super::{SpritePropertySyntax, SpriteShapeSyntax, SpriteSpaceSyntax, parse_sprite_node};

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
            matches!(&syntax.properties[1].0, SpritePropertySyntax::Rotate { space: SpriteSpaceSyntax::World, angle, axis: Some(axis) } if angle == "45deg" && axis == "(1, 1, 0)")
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
}
