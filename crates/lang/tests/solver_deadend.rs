use puzzle_lang::{SolverDeadendOf, SolverStrategyOf, parse_game2d, parse_puzzle3d};

#[test]
fn deadend_queries_lower_from_the_solver_block() {
    let source = r#"
title = solver_deadend

puzzle default {
layers {
floor = Goal
actor = Box
}

query blocked = exists([ Box Goal ])
query clear = none([ Box Goal ])

solver {
deadend any {
blocked
clear
}
deadend all {
blocked
clear
}
}

rules {
}

levels tiny of default {
legend {
. = empty
* = Box Goal
}

level "start" {
*
}
}
}
"#;

    let loaded = parse_game2d(source).expect("deadend query should lower");

    assert!(matches!(
        loaded.solver_strategy.deadends.as_slice(),
        [SolverDeadendOf::Any(values), SolverDeadendOf::All(all_values)]
            if values.len() == 2 && all_values.len() == 2
    ));
}

#[test]
fn deadend_queries_lower_for_puzzle3d() {
    let source = r#"
puzzle3 default {
layers {
floor = Goal
actor = Box
}

win_conditions {
some Goal
}

query blocked = exists([ Box Goal ])

solver {
deadend any {
blocked
}
}
}

levels3 tiny of default {
legend {
. = empty
* = Box Goal
}

level "start" {
*
}
}
"#;

    let parsed = parse_puzzle3d(source).expect("3D deadend query should lower");

    assert!(matches!(
        parsed.solver_strategy.deadends.as_slice(),
        [SolverDeadendOf::Any(values)] if values.len() == 1
    ));
}

#[test]
fn deadend_combinators_share_one_evaluator() {
    let strategy = SolverStrategyOf {
        terms: Vec::new(),
        deadends: vec![
            SolverDeadendOf::All(vec![false, true]),
            SolverDeadendOf::Any(vec![false, true]),
        ],
    };

    assert!(strategy.has_deadend_with(|value| *value));
}
