fn require_arrow_row<'a>(line: &'a str, message: &str) -> Result<(&'a str, &'a str), DiagnosticReport> {
    line.split_once("->")
        .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        .ok_or_else(|| parse_error(line, message))
}

fn parse_assignment_row(line: &str) -> Option<(&str, &str)> {
    line.split_once('=').map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
}

fn require_assignment_row<'a>(line: &'a str, message: &str) -> Result<(&'a str, &'a str), DiagnosticReport> {
    parse_assignment_row(line).ok_or_else(|| parse_error(line, message))
}

struct KeysSurfaceRow<'a> {
    keys: Vec<&'a str>,
    target: &'a str,
}

fn parse_keys_surface_row<'a>(
    line: &'a str,
    target: &str,
    reject_equals: bool,
) -> Result<KeysSurfaceRow<'a>, DiagnosticReport> {
    if reject_equals && line.contains('=') {
        return Err(parse_error(
            line,
            &format!("keys row must use `->`: <key...> -> <{target}>"),
        ));
    }
    let (keys_text, target_text) =
        require_arrow_row(line, &format!("keys row must be: <key...> -> <{target}>"))?;
    let keys = keys_text.split_whitespace().collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(parse_error(line, "keys row must name at least one key"));
    }
    Ok(KeysSurfaceRow {
        keys,
        target: target_text.trim(),
    })
}

fn parse_view_path(value: &str) -> Option<Vec<String>> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || !parts.iter().all(|part| is_qualified_identifier(part)) {
        return None;
    }
    Some(parts.into_iter().map(ToString::to_string).collect())
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for index in top_level_char_indexes(value, ',') {
        parts.push(&value[start..index]);
        start = index + 1;
    }
    parts.push(&value[start..]);
    parts
}

fn split_top_level_keyword_once<'a>(value: &'a str, keyword: &str) -> Option<(&'a str, &'a str)> {
    for (index, _) in top_level_scan(value) {
        if !value[index..].starts_with(keyword) {
            continue;
        }
        let before = value[..index].chars().next_back();
        let after = value[index + keyword.len()..].chars().next();
        if before.is_none_or(|ch| !is_identifier_continue(ch))
            && after.is_none_or(|ch| !is_identifier_continue(ch))
        {
            return Some((&value[..index], &value[index + keyword.len()..]));
        }
    }
    None
}

fn split_top_level_operator_once<'a>(value: &'a str, operator: &str) -> Option<(&'a str, &'a str)> {
    for (index, _) in top_level_scan(value) {
        if value[index..].starts_with(operator) {
            return Some((&value[..index], &value[index + operator.len()..]));
        }
    }
    None
}

fn find_top_level_char(value: &str, target: char) -> Option<usize> {
    top_level_char_indexes(value, target).into_iter().next()
}

fn top_level_char_indexes(value: &str, target: char) -> Vec<usize> {
    top_level_scan(value)
        .into_iter()
        .filter_map(|(index, ch)| (ch == target).then_some(index))
        .collect()
}

fn top_level_scan(value: &str) -> Vec<(usize, char)> {
    let mut out = Vec::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                paren_depth += 1;
            }
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                brace_depth += 1;
            }
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                bracket_depth += 1;
            }
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                out.push((index, ch));
            }
            _ => {}
        }
    }
    out
}

fn matching_delimiter(value: &str, open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value[open..].char_indices() {
        let index = open + index;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            _ if ch == open_ch => depth += 1,
            _ if ch == close_ch => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
