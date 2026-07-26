use puzzle_lang::source_outline;

#[test]
fn source_outline_omits_layers_merge_grouping() {
    let source = r#"
puzzle board {
  layers {
    Floor
    merge {
      actor = Player Box
      marker = Goal
    }
  }
  rules {
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
            (1, "layers".to_string()),
            (1, "rules".to_string()),
        ]
    );
}
