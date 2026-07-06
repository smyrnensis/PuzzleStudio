use crate::source::{
    SourceContext, SourceContextLine, SourceScope, SourceStructureEvent, split_header_tokens,
    strip_line_comment,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct OutlineStackEntry {
    id: Option<String>,
    suppress_children: bool,
}

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
    source_outline_from_source(source)
}

pub(crate) fn source_outline_from_context(context: &SourceContext) -> Vec<SourceOutlineItem> {
    let mut items = Vec::new();
    let mut stack = Vec::<OutlineStackEntry>::new();
    let mut ids_by_item_key = HashMap::<(usize, String, String), String>::new();
    let mut next_id = 0usize;

    for line in &context.lines {
        if let Some((kind, label)) = source_outline_sprite_entry_header(line)
            && !source_outline_suppresses_children(&stack, line.scope)
        {
            push_source_outline_item(
                &mut items,
                &mut ids_by_item_key,
                &stack,
                &mut next_id,
                line.start,
                line_start_offset(&line.content, line.start),
                line.start + line.content.len(),
                kind,
                label,
            );
        }
        for event in &line.structural_events {
            let SourceStructureEvent::Open { header, scope } = event else {
                stack.pop();
                continue;
            };
            let tokens = split_header_tokens(header);
            if source_outline_suppresses_children(&stack, line.scope) {
                stack.push(OutlineStackEntry {
                    id: None,
                    suppress_children: true,
                });
                continue;
            }
            if let Some((kind, label)) = outline_header(&tokens, header, *scope) {
                let suppress_children = outline_block_suppresses_children(&tokens, *scope);
                let id = push_source_outline_item(
                    &mut items,
                    &mut ids_by_item_key,
                    &stack,
                    &mut next_id,
                    line.start,
                    line_start_offset(&line.content, line.start),
                    line.start + line.content.len(),
                    kind,
                    label,
                );
                stack.push(OutlineStackEntry {
                    id: Some(id),
                    suppress_children,
                });
            } else {
                stack.push(OutlineStackEntry {
                    id: None,
                    suppress_children: false,
                });
            }
        }
    }

    items
}

fn source_outline_from_source(source: &str) -> Vec<SourceOutlineItem> {
    let mut items = Vec::new();
    let mut stack = Vec::<OutlineStackEntry>::new();
    let mut scope_stack = Vec::<SourceScope>::new();
    let mut ids_by_item_key = HashMap::<(usize, String, String), String>::new();
    let mut next_id = 0usize;
    let mut offset = 0usize;

    for line in source.split_inclusive('\n') {
        let line_end = offset + line.len();
        let content_end = line_end - usize::from(line.ends_with('\n'));
        let content = &source[offset..content_end];
        let current = scope_stack.last().copied();
        let trimmed = strip_line_comment(content).trim();

        if trimmed.is_empty() {
            close_unbraced_level(&mut scope_stack, &mut stack);
            offset = line_end;
            continue;
        }

        if trimmed == "}" {
            close_outline_scope(&mut scope_stack, &mut stack);
            offset = line_end;
            continue;
        }

        let tokens = split_header_tokens(trimmed);
        if let Some((kind, label)) = outline_sprite_entry_header(current, trimmed, &tokens)
            && !source_outline_suppresses_children(&stack, current)
        {
            push_source_outline_item(
                &mut items,
                &mut ids_by_item_key,
                &stack,
                &mut next_id,
                offset,
                line_start_offset(content, offset),
                content_end,
                kind,
                label,
            );
        }

        let Some(opened) = thin_outline_opening_scope(trimmed, &tokens, current) else {
            offset = line_end;
            continue;
        };
        let header = structural_header(trimmed);
        if source_outline_suppresses_children(&stack, current) {
            scope_stack.push(opened);
            stack.push(OutlineStackEntry {
                id: None,
                suppress_children: true,
            });
            offset = line_end;
            continue;
        }
        if let Some((kind, label)) = outline_header(&tokens, &header, opened) {
            let suppress_children = outline_block_suppresses_children(&tokens, opened);
            let id = push_source_outline_item(
                &mut items,
                &mut ids_by_item_key,
                &stack,
                &mut next_id,
                offset,
                line_start_offset(content, offset),
                content_end,
                kind,
                label,
            );
            scope_stack.push(opened);
            stack.push(OutlineStackEntry {
                id: Some(id),
                suppress_children,
            });
        } else {
            scope_stack.push(opened);
            stack.push(OutlineStackEntry {
                id: None,
                suppress_children: false,
            });
        }

        offset = line_end;
    }

    items
}

fn close_unbraced_level(scope_stack: &mut Vec<SourceScope>, stack: &mut Vec<OutlineStackEntry>) {
    if scope_stack.last() == Some(&SourceScope::UnbracedLevel) {
        scope_stack.pop();
        stack.pop();
    }
}

fn close_outline_scope(scope_stack: &mut Vec<SourceScope>, stack: &mut Vec<OutlineStackEntry>) {
    if scope_stack.last() == Some(&SourceScope::UnbracedLevel) {
        scope_stack.pop();
        stack.pop();
        if scope_stack.last() == Some(&SourceScope::Levels) {
            scope_stack.pop();
            stack.pop();
        }
        return;
    }
    scope_stack.pop();
    stack.pop();
}

fn push_source_outline_item(
    items: &mut Vec<SourceOutlineItem>,
    ids_by_item_key: &mut HashMap<(usize, String, String), String>,
    stack: &[OutlineStackEntry],
    next_id: &mut usize,
    line_start: usize,
    start: usize,
    end: usize,
    kind: String,
    label: String,
) -> String {
    let key = (line_start, kind.clone(), label.clone());
    if let Some(id) = ids_by_item_key.get(&key) {
        return id.clone();
    }
    let id = format!("outline-{next_id}");
    *next_id += 1;
    let parent = stack.iter().rev().find_map(|entry| entry.id.clone());
    items.push(SourceOutlineItem {
        id: id.clone(),
        kind,
        label,
        start,
        end,
        depth: stack.iter().filter(|entry| entry.id.is_some()).count(),
        parent,
    });
    ids_by_item_key.insert(key, id.clone());
    id
}

fn source_outline_sprite_entry_header(line: &SourceContextLine) -> Option<(String, String)> {
    if !matches!(line.scope, Some(SourceScope::Visuals)) || code_trim(&line.content).ends_with('{')
    {
        return None;
    }
    let Some(name) = line.tokens.first() else {
        return None;
    };
    if !sprite_definition_name_token(name) {
        return None;
    }
    Some(("sprite".to_string(), name.to_string()))
}

fn outline_sprite_entry_header(
    scope: Option<SourceScope>,
    trimmed: &str,
    tokens: &[&str],
) -> Option<(String, String)> {
    if scope != Some(SourceScope::Visuals) || trimmed.ends_with('{') {
        return None;
    }
    let Some(name) = tokens.first() else {
        return None;
    };
    if !sprite_definition_name_token(name) {
        return None;
    }
    Some(("sprite".to_string(), (*name).to_string()))
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

fn outline_header(tokens: &[&str], header: &str, scope: SourceScope) -> Option<(String, String)> {
    let first = tokens.first().copied()?;
    if is_statement_control_flow(first) || is_implicit_level_body_header(tokens, scope) {
        return None;
    }
    if first == "level" {
        return Some((
            "level".to_string(),
            level_name(header).unwrap_or_else(|| "level".to_string()),
        ));
    }
    Some((first.to_string(), header.to_string()))
}

fn thin_outline_opening_scope(
    line: &str,
    tokens: &[&str],
    current: Option<SourceScope>,
) -> Option<SourceScope> {
    if current == Some(SourceScope::Levels) && !tokens.is_empty() {
        return match tokens {
            ["legend"] if line.ends_with('{') => Some(SourceScope::Legend),
            ["{"] | ["level", ..] if line.ends_with('{') => Some(SourceScope::Level),
            [..] => Some(SourceScope::UnbracedLevel),
        };
    }
    if current == Some(SourceScope::Visuals) && line.ends_with('{') {
        return match tokens {
            ["colors"] => Some(SourceScope::VisualColorTable),
            ["shapes"] => Some(SourceScope::VisualShapeTable),
            [..] => Some(SourceScope::VisualShapeEntry),
        };
    }
    if matches!(
        current,
        Some(SourceScope::VisualShapeTable | SourceScope::VisualShapeEntry)
    ) && line.ends_with('{')
    {
        return Some(SourceScope::VisualShapeEntry);
    }
    if !line.ends_with('{') && !thin_outline_bare_block_header(tokens) {
        return None;
    }
    match tokens {
        ["sounds"] => Some(SourceScope::Sounds),
        ["assets"] => Some(SourceScope::Assets),
        ["scene", ..] => Some(SourceScope::Scene),
        ["puzzle", ..] | ["puzzle3", ..] => Some(SourceScope::Puzzle),
        ["tags"] => Some(SourceScope::Tags),
        ["layers"] | ["collision_layers"] => Some(SourceScope::Layers),
        ["groups"] => Some(SourceScope::Group),
        ["marks"] => Some(SourceScope::Mark),
        ["keys"] | ["inputs"] => Some(SourceScope::Keys),
        ["legend"] => Some(SourceScope::Legend),
        ["levels", ..] | ["levels3", ..] => Some(SourceScope::Levels),
        ["level", ..] => Some(SourceScope::Level),
        ["sprites", ..] | ["sprite", ..] | ["sprites3", ..] => Some(SourceScope::Visuals),
        ["colors"] => Some(SourceScope::VisualColorTable),
        ["shapes"] => Some(SourceScope::VisualShapeTable),
        ["render"] | ["camera"] | ["rules"] | ["resources"] => Some(SourceScope::Other),
        [..] => line.ends_with('{').then_some(SourceScope::Other),
    }
}

fn thin_outline_bare_block_header(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["sounds"]
            | ["assets"]
            | ["groups"]
            | ["legend"]
            | ["levels", ..]
            | ["levels3", ..]
            | ["tags"]
            | ["layers"]
            | ["collision_layers"]
            | ["marks"]
            | ["keys"]
            | ["inputs"]
            | ["resources"]
            | ["sprites", ..]
            | ["sprites3", ..]
            | ["colors"]
            | ["shapes"]
            | ["render"]
            | ["camera"]
            | ["scene", ..]
            | ["puzzle", ..]
            | ["puzzle3", ..]
            | ["rules"]
    )
}

fn structural_header(line: &str) -> String {
    line.strip_suffix('{')
        .map(str::trim)
        .unwrap_or_else(|| line.trim())
        .to_string()
}

fn source_outline_suppresses_children(
    stack: &[OutlineStackEntry],
    scope: Option<SourceScope>,
) -> bool {
    stack.iter().any(|entry| entry.suppress_children)
        || matches!(scope, Some(SourceScope::Keys | SourceScope::SceneKeys))
}

fn outline_block_suppresses_children(tokens: &[&str], scope: SourceScope) -> bool {
    matches!(
        tokens,
        ["keys"] | ["inputs"] | ["routine", ..] | ["condition", ..] | ["fix", ..]
    ) || tokens.first().is_some_and(|token| token.starts_with("on_"))
        || matches!(scope, SourceScope::VisualShapeEntry)
}

fn is_implicit_level_body_header(tokens: &[&str], scope: SourceScope) -> bool {
    scope == SourceScope::UnbracedLevel && !matches!(tokens, ["level", ..])
}

fn is_statement_control_flow(kind: &str) -> bool {
    matches!(kind, "repeat" | "if" | "else" | "for")
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

fn sprite_definition_name_token(value: &str) -> bool {
    if matches!(
        value,
        "shape"
            | "shapes"
            | "colors"
            | "ascii"
            | "sprites"
            | "sprites3"
            | "rotate"
            | "pixels_per_cell"
            | "offset"
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

fn code_trim(line: &str) -> &str {
    crate::source::strip_line_comment(line).trim()
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
    routine push {
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
                (2, "routine push".to_string()),
                (1, "levels".to_string()),
                (2, "First".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_follows_source_structural_blocks() {
        let source = r#"
puzzle board {
  render {
    camera {
    }
  }
  sprites {
    shapes {
      Box
      aaa
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
                (1, "render".to_string()),
                (2, "camera".to_string()),
                (1, "sprites".to_string()),
                (2, "shapes".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_uses_structural_tree_for_sprite_entries() {
        let source = r##"
sprites {
Player
#fff
000
Wall #000

Crate {
#f3a002 #b38002
00000
01110
}

Light
#ffffcc #000000
010
111
}
"##;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "sprites".to_string(), "sprites".to_string()),
                (1, "sprite".to_string(), "Player".to_string()),
                (1, "sprite".to_string(), "Wall".to_string()),
                (1, "Crate".to_string(), "Crate".to_string()),
                (1, "sprite".to_string(), "Light".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_uses_source_context_tree_for_unbraced_level_entries() {
        let source = r#"
levels {
level "First"
P.

level "level 1"
..

level
..
}
"#;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "levels".to_string(), "levels".to_string()),
                (1, "level".to_string(), "First".to_string()),
                (1, "level".to_string(), "level 1".to_string()),
                (1, "level".to_string(), "level".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_treats_routines_as_leaf_items() {
        let source = r#"
puzzle board {
  rules {
    routine open_gate {
      repeat {
        for n in 1...5 {
          if some([ Gate:n{checked} ]) {
            [ Gate:n ] -> [ Gate:open ]
          } else {
            [ Gate:n ] -> [ Gate:closed ]
          }
        }
      }
    }
  }
}
"#;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "puzzle".to_string(), "puzzle board".to_string()),
                (1, "rules".to_string(), "rules".to_string()),
                (2, "routine".to_string(), "routine open_gate".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_names_maps() {
        let source = r#"
puzzle board {
  tags {
    directions: up right down left
  }
  map rotate directions {
    up -> right
    right -> down
    down -> left
    left -> up
  }
}
"#;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "puzzle".to_string(), "puzzle board".to_string()),
                (1, "tags".to_string(), "tags".to_string()),
                (1, "map".to_string(), "map rotate directions".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_does_not_create_key_binding_children() {
        let source = r#"
puzzle board {
  keys {
    Enter -> restart
    Space -> {
      restart
    }
  }
  scene title {
    keys {
      Enter -> {
        continue_game
      }
    }
  }
}
"#;
        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "puzzle".to_string(), "puzzle board".to_string()),
                (1, "keys".to_string(), "keys".to_string()),
                (1, "scene".to_string(), "scene title".to_string()),
                (2, "keys".to_string(), "keys".to_string()),
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
