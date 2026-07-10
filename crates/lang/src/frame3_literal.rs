use std::collections::HashSet;

use puzzle_grid3d::{Direction3, Frame3};

pub(crate) fn parse_frame3_literal(value: &str) -> Result<Frame3, String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| {
            "frame3 value must be parenthesized: (<primary>, <secondary>)".to_string()
        })?;
    parse_frame3_components(inner)
}

pub(crate) fn parse_frame3_orientation_sugar(value: &str) -> Result<Frame3, String> {
    parse_frame3_components(value.trim())
}

pub(crate) fn normalize_frame3_literal(value: &str) -> Result<String, String> {
    parse_frame3_literal(value).map(format_frame3)
}

pub(crate) fn parse_frame3_domain(body: &str) -> Result<Option<Vec<String>>, String> {
    let mut cursor = 0usize;
    let mut values = Vec::new();
    let mut seen = HashSet::new();
    let mut recognized = false;

    while let Some(start) = next_non_whitespace(body, cursor) {
        let Some((frame, end)) = parse_domain_item(body, start)? else {
            if recognized {
                return Err("frame3 domain may contain only frame3 values".to_string());
            }
            return Ok(None);
        };
        recognized = true;
        let value = format_frame3(frame);
        if !seen.insert(value.clone()) {
            return Err("frame3 domain contains a duplicate orientation".to_string());
        }
        values.push(value);
        cursor = end;
    }

    Ok(recognized.then_some(values))
}

pub(crate) fn format_frame3(frame: Frame3) -> String {
    if frame.is_canonical_chiral() {
        format!("({},{})", frame.primary.name, frame.secondary.name)
    } else {
        format!(
            "({},{},{})",
            frame.primary.name, frame.secondary.name, frame.depth.name
        )
    }
}

pub(crate) fn split_frame3_components(value: &str) -> Result<Vec<&str>, String> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|part| part.is_empty()) {
        return Err("frame3 value must have two or three direction components".to_string());
    }
    Ok(parts)
}

fn parse_frame3_components(value: &str) -> Result<Frame3, String> {
    let parts = split_frame3_components(value)?;
    let directions = parts
        .iter()
        .map(|part| {
            Direction3::by_name(part).ok_or_else(|| format!("unknown frame3 direction: {part}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if directions.len() == 2 {
        Frame3::canonical(directions[0], directions[1])
    } else {
        Frame3::explicit(directions[0], directions[1], directions[2])
    }
    .map_err(|error| format!("invalid frame3 orientation: {error:?}"))
}

fn parse_domain_item(body: &str, start: usize) -> Result<Option<(Frame3, usize)>, String> {
    if body[start..].starts_with('(') {
        let Some(relative_end) = body[start + 1..].find(')') else {
            return Err("frame3 value is missing closing )".to_string());
        };
        let end = start + 1 + relative_end + 1;
        let item = &body[start..end];
        if !parenthesized_item_starts_with_direction(item) {
            return Ok(None);
        }
        return parse_frame3_literal(item).map(|frame| Some((frame, end)));
    }

    let Some((first, mut cursor)) = parse_identifier(body, start) else {
        return Ok(None);
    };
    if Direction3::by_name(first).is_none() {
        return Ok(None);
    }
    cursor = skip_whitespace(body, cursor);
    if body.as_bytes().get(cursor) != Some(&b',') {
        return Ok(None);
    }
    cursor = skip_whitespace(body, cursor + 1);
    let Some((second, after_second)) = parse_identifier(body, cursor) else {
        return Err("frame3 value is missing its secondary direction".to_string());
    };
    cursor = skip_whitespace(body, after_second);
    let mut item = format!("{first},{second}");
    if body.as_bytes().get(cursor) == Some(&b',') {
        cursor = skip_whitespace(body, cursor + 1);
        let Some((third, after_third)) = parse_identifier(body, cursor) else {
            return Err("frame3 value is missing its third direction".to_string());
        };
        item.push(',');
        item.push_str(third);
        cursor = after_third;
    } else {
        cursor = after_second;
    }
    parse_frame3_orientation_sugar(&item).map(|frame| Some((frame, cursor)))
}

fn parenthesized_item_starts_with_direction(value: &str) -> bool {
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value);
    inner
        .split_once(',')
        .and_then(|(first, _)| Direction3::by_name(first.trim()))
        .is_some()
}

fn next_non_whitespace(value: &str, start: usize) -> Option<usize> {
    let next = skip_whitespace(value, start);
    (next < value.len()).then_some(next)
}

fn skip_whitespace(value: &str, mut cursor: usize) -> usize {
    while let Some(ch) = value[cursor..].chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn parse_identifier(value: &str, start: usize) -> Option<(&str, usize)> {
    let mut end = start;
    for ch in value[start..].chars() {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            break;
        }
        end += ch.len_utf8();
    }
    (end > start).then_some((&value[start..end], end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_canonical_and_mirrored_frames() {
        assert_eq!(
            normalize_frame3_literal("(right, front)").unwrap(),
            "(right,front)"
        );
        assert_eq!(
            normalize_frame3_literal("(right, front, up)").unwrap(),
            "(right,front)"
        );
        assert_eq!(
            normalize_frame3_literal("(right, front, down)").unwrap(),
            "(right,front,down)"
        );
    }

    #[test]
    fn parses_parenthesized_and_bare_domain_items() {
        assert_eq!(
            parse_frame3_domain("(right, front) (front, left)").unwrap(),
            Some(vec![
                "(right,front)".to_string(),
                "(front,left)".to_string()
            ])
        );
        assert_eq!(
            parse_frame3_domain("right, front front, left").unwrap(),
            Some(vec![
                "(right,front)".to_string(),
                "(front,left)".to_string()
            ])
        );
        assert_eq!(parse_frame3_domain("(0, 1) (1, 0)").unwrap(), None);
    }
}
