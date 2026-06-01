pub fn is_display_object_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('@') else {
        return false;
    };
    let without_scratch = rest.split_once('{').map_or(rest, |(base, _)| base);
    let base = without_scratch
        .split_once(':')
        .map_or(without_scratch, |(base, _)| base);
    is_identifier(base)
}

pub fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub fn split_object_spec(token: &str) -> Option<(&str, impl Iterator<Item = &str> + '_)> {
    let mut parts = token.split(':');
    let base = parts.next()?;
    (!base.is_empty()).then_some((base, parts))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchSugarKind {
    Movement,
    Bool,
    Int,
}

pub fn scratch_sugar_kind(token: &str) -> Option<ScratchSugarKind> {
    if matches!(
        token,
        ">" | "<"
            | "^"
            | "v"
            | "up"
            | "down"
            | "left"
            | "right"
            | "front"
            | "back"
            | "forward"
            | "backward"
            | "directions"
            | "horizontal"
            | "vertical"
            | "parallel"
            | "perpendicular"
    ) {
        Some(ScratchSugarKind::Movement)
    } else if matches!(token, "true" | "false") {
        Some(ScratchSugarKind::Bool)
    } else if token.parse::<i64>().is_ok() {
        Some(ScratchSugarKind::Int)
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellTokenError {
    UnmatchedCloseBrace,
    MissingCloseBrace,
}

pub fn split_cell_tokens(cell: &str) -> Result<Vec<String>, CellTokenError> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut brace_depth = 0_u16;
    for ch in cell.chars() {
        match ch {
            '{' => {
                brace_depth += 1;
                token.push(ch);
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseBrace);
                }
                brace_depth -= 1;
                token.push(ch);
            }
            ch if ch.is_whitespace() && brace_depth == 0 => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(ch),
        }
    }
    if brace_depth != 0 {
        return Err(CellTokenError::MissingCloseBrace);
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_name_marks_display_object_tokens() {
        assert!(is_display_object_token("@Trail"));
        assert!(is_display_object_token("@Trail:kind"));
        assert!(is_display_object_token("@Trail{right}"));
        assert!(!is_display_object_token("Trail"));
        assert!(!is_display_object_token("@"));
        assert!(!is_display_object_token("@:kind"));
    }

    #[test]
    fn shared_scratch_sugar_recognizes_2d_and_3d_direction_words() {
        assert_eq!(scratch_sugar_kind(">"), Some(ScratchSugarKind::Movement));
        assert_eq!(
            scratch_sugar_kind("front"),
            Some(ScratchSugarKind::Movement)
        );
        assert_eq!(scratch_sugar_kind("true"), Some(ScratchSugarKind::Bool));
        assert_eq!(scratch_sugar_kind("7"), Some(ScratchSugarKind::Int));
        assert_eq!(scratch_sugar_kind("Player"), None);
    }

    #[test]
    fn shared_cell_tokenizer_keeps_scratch_blocks_together() {
        assert_eq!(
            split_cell_tokens("Player{> no flag} no Wall").unwrap(),
            vec!["Player{> no flag}", "no", "Wall"]
        );
    }
}
