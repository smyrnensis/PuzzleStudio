fn require_arrow_row<'a>(
    line: &'a str,
    message: &str,
) -> Result<(&'a str, &'a str), DiagnosticReport> {
    line.split_once("->")
        .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        .ok_or_else(|| parse_error(line, message))
}

fn parse_assignment_row(line: &str) -> Option<(&str, &str)> {
    puzzle_authoring::parse_assignment_row(line)
}

fn require_assignment_row<'a>(
    line: &'a str,
    message: &str,
) -> Result<(&'a str, &'a str), DiagnosticReport> {
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
    let (keys_text, target_text) =
        require_arrow_row(line, &format!("keys row must be: <key...> -> <{target}>"))?;
    if reject_equals && parse_assignment_row(keys_text).is_some() {
        return Err(parse_error(
            line,
            &format!("keys row must use `->`: <key...> -> <{target}>"),
        ));
    }
    let keys = keys_text.split_whitespace().collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(parse_error(line, "keys row must name at least one key"));
    }
    Ok(KeysSurfaceRow {
        keys,
        target: target_text,
    })
}

fn parse_optional_call_surface_with_suffix<'a>(
    value: &'a str,
    line: &str,
    close_message: &str,
) -> Result<Option<(puzzle_authoring::CallSurface<'a>, &'a str)>, DiagnosticReport> {
    puzzle_authoring::parse_optional_call_surface_with_suffix(value)
        .map_err(|()| parse_error(line, close_message))
}

fn require_call_surface_with_suffix<'a>(
    value: &'a str,
    line: &str,
    missing_message: &str,
    close_message: &str,
) -> Result<(puzzle_authoring::CallSurface<'a>, &'a str), DiagnosticReport> {
    parse_optional_call_surface_with_suffix(value, line, close_message)?
        .ok_or_else(|| parse_error(line, missing_message))
}

fn parse_view_path(value: &str) -> Option<Vec<String>> {
    puzzle_authoring::parse_view_path(value)
}

#[cfg(test)]
mod authoring_parse_syntax_tests {
    use super::*;

    #[test]
    fn call_surface_splits_only_top_level_arguments() {
        let (call, suffix) = require_call_surface_with_suffix(
            r#"foo(a, bar("x, y"), if ready { level(0) } else { level(1) })"#,
            "test line",
            "missing call",
            "call must close",
        )
        .expect("call surface");

        assert_eq!(call.name, "foo");
        assert_eq!(suffix, "");
        assert_eq!(
            call.args,
            vec![
                "a",
                r#"bar("x, y")"#,
                "if ready { level(0) } else { level(1) }"
            ]
        );
    }

    #[test]
    fn call_surface_preserves_suffix_for_owner_handling() {
        let (call, suffix) = require_call_surface_with_suffix(
            "level(0).title",
            "test line",
            "missing call",
            "call must close",
        )
        .expect("call surface");

        assert_eq!(call.name, "level");
        assert_eq!(call.args, vec!["0"]);
        assert_eq!(suffix, ".title");
    }
}
