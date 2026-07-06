use crate::source::{SourceContext, scan_source_context, split_header_tokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOutlineItem {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub depth: usize,
    pub parent: Option<String>,
}

pub fn source_outline(source: &str) -> Vec<SourceOutlineItem> {
    let context = scan_source_context(source);
    source_outline_from_context(&context)
}

pub(crate) fn source_outline_from_context(context: &SourceContext) -> Vec<SourceOutlineItem> {
    let mut items = Vec::new();
    let mut stack = Vec::<Option<String>>::new();
    let mut next_id = 0usize;

    for line in &context.lines {
        for structural_line in &line.structural_lines {
            let trimmed = structural_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed == "}" {
                stack.pop();
                continue;
            }

            let Some(header) = trimmed.strip_suffix('{').map(str::trim) else {
                continue;
            };
            let tokens = split_header_tokens(header);
            if let Some((kind, label)) = outline_header(&tokens, header) {
                let id = format!("outline-{next_id}");
                next_id += 1;
                let parent = stack.iter().rev().find_map(Clone::clone);
                items.push(SourceOutlineItem {
                    id: id.clone(),
                    kind,
                    label,
                    start: line_start_offset(&line.content, line.start),
                    end: line.start + line.content.len(),
                    depth: stack.iter().filter(|entry| entry.is_some()).count(),
                    parent,
                });
                stack.push(Some(id));
            } else {
                stack.push(None);
            }
        }
    }

    items
}

pub fn source_outline_json(source: &str) -> String {
    let items = source_outline(source);
    let mut out = String::from("{\"items\":[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_item_json(&mut out, item);
    }
    out.push_str("]}");
    out
}

fn outline_header(tokens: &[&str], header: &str) -> Option<(String, String)> {
    let first = tokens.first().copied()?;
    if !is_outline_kind(first) {
        return None;
    }
    let name = outline_name(first, tokens, header);
    let label = if name.is_empty() {
        first.to_string()
    } else {
        format!("{first} {name}")
    };
    Some((first.to_string(), label))
}

fn is_outline_kind(kind: &str) -> bool {
    matches!(
        kind,
        "puzzle"
            | "puzzle3"
            | "metadata"
            | "theme"
            | "assets"
            | "colors"
            | "objects"
            | "object"
            | "legend"
            | "groups"
            | "layers"
            | "collision_layers"
            | "tags"
            | "sprites"
            | "sprite"
            | "sprites3"
            | "levels"
            | "level"
            | "levels3"
            | "rules"
            | "rule"
            | "win_conditions"
            | "lose_conditions"
            | "sounds"
            | "scene"
            | "screen"
            | "layout"
            | "keys"
            | "resources"
            | "level_menu"
            | "fix"
    ) || kind.starts_with("on_")
}

fn outline_name(kind: &str, tokens: &[&str], header: &str) -> String {
    match kind {
        "level" => level_name(header).unwrap_or_default(),
        "levels" | "levels3" => owner_name(tokens),
        "puzzle" | "puzzle3" | "theme" | "object" | "sprite" | "rule" | "scene" | "screen" => {
            tokens.get(1).copied().unwrap_or("").to_string()
        }
        "fix" => tokens.get(1).copied().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

fn owner_name(tokens: &[&str]) -> String {
    if let Some(of_index) = tokens.iter().position(|token| *token == "of") {
        return tokens.get(of_index + 1).copied().unwrap_or("").to_string();
    }
    String::new()
}

fn level_name(header: &str) -> Option<String> {
    let mut rest = header.trim().strip_prefix("level")?.trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(quoted) = rest.strip_prefix('"') {
        let mut out = String::new();
        let mut escaped = false;
        for ch in quoted.chars() {
            if escaped {
                out.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(out);
            } else {
                out.push(ch);
            }
        }
        return None;
    }
    if let Some(index) = rest.find(" of ") {
        rest = &rest[..index];
    }
    Some(rest.trim().to_string())
}

fn line_start_offset(content: &str, line_start: usize) -> usize {
    line_start + content.len() - content.trim_start().len()
}

fn push_item_json(out: &mut String, item: &SourceOutlineItem) {
    out.push('{');
    push_json_string(out, "id", &item.id);
    out.push(',');
    push_json_string(out, "kind", &item.kind);
    out.push(',');
    push_json_string(out, "label", &item.label);
    out.push(',');
    push_json_number(out, "start", item.start);
    out.push(',');
    push_json_number(out, "end", item.end);
    out.push(',');
    push_json_number(out, "depth", item.depth);
    out.push_str(",\"parent\":");
    match &item.parent {
        Some(parent) => push_json_string_value(out, parent),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_json_number(out: &mut String, key: &str, value: usize) {
    push_json_string_value(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_string(out: &mut String, key: &str, value: &str) {
    push_json_string_value(out, key);
    out.push(':');
    push_json_string_value(out, value);
}

fn push_json_string_value(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{source_outline, source_outline_json};

    #[test]
    fn source_outline_keeps_author_source_order() {
        let source = r#"
puzzle board {
  objects {
  }
  rules {
    rule push {
    }
  }
  levels {
    level "First" {
      @
    }
  }
}
"#;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "puzzle board".to_string()),
                (1, "objects".to_string()),
                (1, "rules".to_string()),
                (2, "rule push".to_string()),
                (1, "levels".to_string()),
                (2, "level First".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_json_escapes_labels() {
        let json = source_outline_json("puzzle \"quoted\" {\n}\n");
        assert!(json.contains(r#""label":"puzzle \"quoted\"""#));
        assert!(json.contains(r#""parent":null"#));
    }
}
