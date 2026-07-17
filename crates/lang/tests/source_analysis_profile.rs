use puzzle_lang::{PuzzleSourceProfile, SourceTargetKind, analyze_source_for_profile};

const PROFILE_SENSITIVE_SOURCE: &str = r#"
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

sprites basic of sokoban {
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
fn source_analysis_profile_does_not_override_model_dimension() {
    let level_cursor = PROFILE_SENSITIVE_SOURCE
        .find(".\n-\n.")
        .expect("stacked level body");
    let sprite_cursor = PROFILE_SENSITIVE_SOURCE
        .find("0\n-\n.")
        .expect("stacked sprite body");

    let puzzle3 =
        analyze_source_for_profile(PROFILE_SENSITIVE_SOURCE, PuzzleSourceProfile::Puzzle3d);
    assert_eq!(
        puzzle3
            .resolve_target(level_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Level),
    );
    assert_eq!(
        puzzle3
            .resolve_target(sprite_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Sprite),
    );

    let puzzle2 =
        analyze_source_for_profile(PROFILE_SENSITIVE_SOURCE, PuzzleSourceProfile::Puzzle2d);
    assert_eq!(
        puzzle2
            .resolve_target(level_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Level),
    );
    assert_eq!(
        puzzle2
            .resolve_target(sprite_cursor)
            .map(|target| target.kind),
        Some(SourceTargetKind::Sprite),
    );
    assert_eq!(
        puzzle2
            .resolve_target(level_cursor)
            .and_then(|target| target.dimension),
        Some(puzzle_lang::ModelDimension::Three),
    );
    assert_eq!(
        puzzle2
            .resolve_target(sprite_cursor)
            .and_then(|target| target.dimension),
        Some(puzzle_lang::ModelDimension::Three),
    );
}
