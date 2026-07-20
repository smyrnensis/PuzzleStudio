/// Canonical rule-binding substitution, promoted from the original 2D parser.
///
/// It owns lexical behavior (identifiers, quoted text, comments, projections,
/// and call boundaries). Lowerers provide only the meaning of a binding
/// projection and a named call.
pub(crate) fn substitute_rule_binding_line<E>(
    line: &str,
    binding: &str,
    mut binding_value: impl FnMut(&[String]) -> Result<String, E>,
    mut call_value: impl FnMut(&str, &str) -> Result<Option<String>, E>,
) -> Result<String, E> {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '"' {
            copy_quoted_segment(&chars, &mut i, &mut out);
            continue;
        }
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            out.extend(chars[i..].iter());
            break;
        }
        if is_identifier_start(chars[i]) {
            let name_start = i;
            i += 1;
            while i < chars.len() && is_identifier_continue(chars[i]) {
                i += 1;
            }
            if i < chars.len() && chars[i] == '(' {
                let arg_start = i + 1;
                let mut arg_end = arg_start;
                while arg_end < chars.len() && is_identifier_continue(chars[arg_end]) {
                    arg_end += 1;
                }
                if arg_end > arg_start && arg_end < chars.len() && chars[arg_end] == ')' {
                    let name = chars[name_start..i].iter().collect::<String>();
                    let arg = chars[arg_start..arg_end].iter().collect::<String>();
                    if let Some(value) = call_value(&name, &arg)? {
                        out.push_str(&value);
                        i = arg_end + 1;
                        continue;
                    }
                }
            }
            let name = chars[name_start..i].iter().collect::<String>();
            if name == binding {
                let mut projection = Vec::new();
                while i < chars.len() && chars[i] == '.' {
                    let field_start = i + 1;
                    if field_start >= chars.len() || !is_identifier_start(chars[field_start]) {
                        break;
                    }
                    let mut field_end = field_start + 1;
                    while field_end < chars.len() && is_identifier_continue(chars[field_end]) {
                        field_end += 1;
                    }
                    projection.push(chars[field_start..field_end].iter().collect::<String>());
                    i = field_end;
                }
                out.push_str(&binding_value(&projection)?);
                continue;
            }
            out.extend(chars[name_start..i].iter());
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

fn copy_quoted_segment(chars: &[char], i: &mut usize, out: &mut String) {
    out.push(chars[*i]);
    *i += 1;
    let mut escaped = false;
    while *i < chars.len() {
        let ch = chars[*i];
        out.push(ch);
        *i += 1;
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_binding_substitution_preserves_strings_and_comments() {
        let expanded = substitute_rule_binding_line(
            r#"TEN:h message \"h\" // h"#,
            "h",
            |projection| {
                assert!(projection.is_empty());
                Ok::<_, ()>("right".to_string())
            },
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(expanded, r#"TEN:right message \"h\" // h"#);
    }

    #[test]
    fn canonical_binding_substitution_passes_the_complete_projection_path() {
        let expanded = substitute_rule_binding_line(
            "if level.progress.cleared { text level.name }",
            "level",
            |projection| Ok::<_, ()>(format!("<{}>", projection.join("/"))),
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(expanded, "if <progress/cleared> { text <name> }");
    }
}
