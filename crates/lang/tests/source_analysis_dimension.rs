use puzzle_lang::{SourceTargetKind, analyze_source};

const OWNER_DIMENSION_SOURCE: &str = r#"
puzzle sokoban {
dimension = 3
}

levels microban of sokoban {
legend {
. = empty
}
level "stacked" {
.
-
.
}
}

visuals basic of sokoban {
Floor {
colors = #ffffff
shape = {
0
-
.
}
}
}
"#;

#[test]
fn source_analysis_uses_the_puzzle_owner_dimension() {
    let level_cursor = OWNER_DIMENSION_SOURCE
        .find(".\n-\n.")
        .expect("stacked level body");
    let visual_cursor = OWNER_DIMENSION_SOURCE
        .find("0\n-\n.")
        .expect("stacked visual body");

    let analysis = analyze_source(OWNER_DIMENSION_SOURCE);
    assert_eq!(
        analysis
            .resolve_target(level_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Level),
    );
    assert_eq!(
        analysis
            .resolve_target(visual_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Visual),
    );
    assert_eq!(
        analysis
            .resolve_target(level_cursor)
            .and_then(|target| target.dimension),
        Some(puzzle_lang::ModelDimension::Three),
    );
    assert_eq!(
        analysis
            .resolve_target(visual_cursor)
            .and_then(|target| target.dimension),
        Some(puzzle_lang::ModelDimension::Three),
    );
}
