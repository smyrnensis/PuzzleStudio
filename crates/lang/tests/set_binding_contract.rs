use puzzle_lang::parse_game2d;

fn game_with_rule(rule: &str) -> Result<puzzle_lang::LoadedGame, puzzle_lang::DiagnosticReport> {
    parse_game2d(&format!(
        r#"
title = set_binding_contract

puzzle default {{
slots {{
a = A
b = B
c = C
}}
empty .

rules {{
{rule}
}}

level "start" {{
legend {{
X = A B C
}}
X
}}
}}
"#
    ))
}

fn lowered_rule_count(rule: &str) -> usize {
    let baseline = game_with_rule("").unwrap().game.rules().len();
    game_with_rule(rule).unwrap().game.rules().len() - baseline
}

#[test]
fn one_unlabeled_movement_capture_is_shared_by_all_rhs_references() {
    let count =
        lowered_rule_count("once [ directions A B ] -> [ directions A directions B directions C ]");

    assert_eq!(count, 4);
}

#[test]
fn labeled_movement_captures_remain_independent() {
    let count = lowered_rule_count(
        "once [ directions#1 A directions#2 B ] -> [ directions#2 A directions#1 B C ]",
    );

    assert_eq!(count, 16);
}

#[test]
fn unlabeled_movement_reference_with_multiple_sources_is_ambiguous() {
    let error = game_with_rule("once [ directions A directions B ] -> [ A B directions C ]")
        .expect_err("two compatible captures require a label")
        .to_string();

    assert!(error.contains("ambiguous movement set reference `directions`; add a #label"));
}

#[test]
fn rhs_movement_set_without_a_capture_is_rejected() {
    let error = game_with_rule("once [ A ] -> [ A directions B ]")
        .expect_err("RHS set references require a LHS capture")
        .to_string();

    assert!(error.contains("unbound movement set reference: directions"));
}
