use crate::surface::{SurfaceDocument, SurfaceStructuralBlock, SurfaceStructuralBlockRole};
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
    let document = crate::parse_surface_structure_document(source);
    source_outline_from_document(&document)
}

pub(crate) fn source_outline_from_document(document: &SurfaceDocument) -> Vec<SourceOutlineItem> {
    let mut items = Vec::new();
    let mut stack = Vec::<OutlineStackEntry>::new();
    let mut ids_by_item_key = HashMap::<(usize, String, String), String>::new();
    let mut next_id = 0usize;

    for block in document.structural_blocks.iter().filter(|block| {
        matches!(block.role, SurfaceStructuralBlockRole::SourceTree)
    }) {
        while stack.len() > block.depth {
            stack.pop();
        }
        if source_outline_suppresses_children(&stack) {
            stack.push(OutlineStackEntry {
                id: None,
                suppress_children: true,
            });
            continue;
        }
        let suppress_children = outline_block_suppresses_children(block);
        let id = push_source_outline_item(
            &mut items,
            &mut ids_by_item_key,
            &stack,
            &mut next_id,
            block.start,
            block.end,
            outline_block_kind(block),
            block.header.clone(),
        );
        stack.push(OutlineStackEntry {
            id: Some(id),
            suppress_children,
        });
    }

    items
}

fn push_source_outline_item(
    items: &mut Vec<SourceOutlineItem>,
    ids_by_item_key: &mut HashMap<(usize, String, String), String>,
    stack: &[OutlineStackEntry],
    next_id: &mut usize,
    start: usize,
    end: usize,
    kind: String,
    label: String,
) -> String {
    let key = (start, kind.clone(), label.clone());
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

fn source_outline_suppresses_children(stack: &[OutlineStackEntry]) -> bool {
    stack.iter().any(|entry| entry.suppress_children)
}

fn outline_block_suppresses_children(block: &SurfaceStructuralBlock) -> bool {
    let first = outline_block_kind(block);
    matches!(
        first.as_str(),
        "keys" | "inputs" | "routine" | "query" | "fix"
    ) || first.starts_with("on_")
        || block.virtual_braces
}

fn outline_block_kind(block: &SurfaceStructuralBlock) -> String {
    block
        .header
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
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
                (2, "level \"First\"".to_string()),
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
                (1, "Player".to_string(), "Player".to_string()),
                (1, "Wall".to_string(), "Wall #000".to_string()),
                (1, "Crate".to_string(), "Crate".to_string()),
                (1, "Light".to_string(), "Light".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_keeps_statement_blocks_out_of_source_tree() {
        let source = r#"
puzzle board {
  rules {
    if some([ Player ]) {
      [ Player ] -> [ Player ]
    } else {
      [ Player ] -> [ ]
    }
  }
  levels {
    level "start"
    P
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
                (1, "levels".to_string(), "levels".to_string()),
                (2, "level".to_string(), "level \"start\"".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_uses_surface_structural_blocks_for_unbraced_level_entries() {
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
                (1, "level".to_string(), "level \"First\"".to_string()),
                (1, "level".to_string(), "level \"level 1\"".to_string()),
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

    #[test]
    fn source_outline_consumes_surface_structure_document() {
        let source = include_str!("source_outline.rs");
        let required = "parse_surface_structure_document";
        assert!(
            source.contains(required),
            "source_outline should consume the parser-owned thin structure product"
        );
        let forbidden_fragments = [
            ["scan_source", "_context"],
            ["Source", "Context"],
            ["SourceStructure", "Event"],
            ["SourceBlock", "Role"],
            ["outline", "_header"],
            ["level", "_name"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "source_outline must not rebuild grammar or scanner products via {forbidden}"
            );
        }
    }
}
