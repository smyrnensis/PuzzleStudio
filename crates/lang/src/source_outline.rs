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

/// Builds one canonical analysis and projects its cached outline product.
pub fn source_outline(source: &str) -> Vec<SourceOutlineItem> {
    crate::analyze_source(source).outline_items()
}

/// Serializes the cached outline projection.
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
    use crate::{authoring_grammar::AuthoringKind, parse_surface_structure_document};

    use super::{
        SourceOutlineItem, source_outline as project_source_outline,
        source_outline_json as project_source_outline_json,
    };

    fn source_outline(source: &str) -> Vec<SourceOutlineItem> {
        project_source_outline(source)
    }

    fn source_outline_json(source: &str) -> String {
        project_source_outline_json(source)
    }

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
  visuals {
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
                (1, "visuals".to_string()),
                (2, "shapes".to_string()),
            ]
        );
    }

    #[test]
    fn source_outline_uses_structural_tree_for_visual_entries() {
        let source = r##"
visuals {
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
                (0, "visuals".to_string(), "visuals".to_string()),
                (1, "visual".to_string(), "Player".to_string()),
                (1, "visual".to_string(), "Wall".to_string()),
                (1, "visual".to_string(), "Crate".to_string()),
                (1, "visual".to_string(), "Light".to_string()),
            ]
        );
    }

    #[test]
    fn bare_shape_reference_stays_inside_its_visual_outline_item() {
        let source = r##"
puzzle board {
layers {
marker = BlackMarker2
}
visuals {
shapes {
shape_WaterMarker {
0
}
}

BlackMarker2
#0000 #000
shape_WaterMarker
}
rules {
}
levels {
legend {
. = empty
B = BlackMarker2
}
level "one"
B
}
}
"##;
        let visual_labels = source_outline(source)
            .into_iter()
            .filter(|item| item.kind == "visual")
            .map(|item| item.label)
            .collect::<Vec<_>>();

        assert_eq!(visual_labels, vec!["BlackMarker2".to_string()]);
    }

    #[test]
    fn author_chosen_shape_names_never_become_outline_kinds() {
        let source = r#"
shapes {
  arrow {
    0
  }
  author_chosen_shape_name {
    0
  }
}
"#;
        let items = source_outline(source)
            .into_iter()
            .filter(|item| item.depth == 1)
            .map(|item| (item.kind, item.label))
            .collect::<Vec<_>>();

        assert_eq!(
            items,
            vec![
                ("shape".to_string(), "arrow".to_string()),
                ("shape".to_string(), "author_chosen_shape_name".to_string(),),
            ],
            "author-selected shape labels must not create new outline kinds or icon requirements"
        );
    }

    #[test]
    fn source_outline_names_visual_entries_from_full_headers() {
        let source = r#"
visuals {
  visual Sugar:flavor {
    colors = #f5f5f5
  }
  visual Honey {
    colors = #e3a018
  }
}
"#;
        let items = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![
                (0, "visuals".to_string(), "visuals".to_string()),
                (1, "visual".to_string(), "Sugar:flavor".to_string()),
                (1, "visual".to_string(), "Honey".to_string()),
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
    fn source_outline_authoring_blocks_carry_schema_kind() {
        let source = r#"
sounds {
  sfx beep {
    seed = coin
  }
}
"#;
        let document = parse_surface_structure_document(source);
        let authoring_blocks = document
            .structural_blocks
            .iter()
            .filter_map(|block| block.authoring_kind)
            .collect::<Vec<_>>();
        assert_eq!(
            authoring_blocks,
            vec![AuthoringKind::SoundsConfig, AuthoringKind::SfxSoundConfig]
        );

        let labels = source_outline(source)
            .into_iter()
            .map(|item| (item.depth, item.kind, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                (0, "sounds".to_string(), "sounds".to_string()),
                (1, "sfx".to_string(), "sfx beep".to_string()),
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
    fn source_outline_projection_contains_no_source_recognizer() {
        let source = include_str!("source_outline.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("outline production source");
        assert!(!production.contains("PuzzleSourceProfile"));
        for forbidden in [
            "SurfaceDocument",
            "structural_blocks",
            ".header",
            ".lines",
            ".content",
            "split_whitespace",
            "starts_with",
            "authoring_grammar",
            "AuthoringKind",
            "SourceScope",
            "scan_",
            "parse_surface",
            "build_surface_outline_items",
        ] {
            assert!(
                !production.contains(forbidden),
                "outline projection must consume parser-owned items, not recognize source via {forbidden}"
            );
        }
    }
}
