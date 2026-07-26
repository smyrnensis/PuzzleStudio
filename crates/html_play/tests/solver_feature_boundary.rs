#[test]
fn solver_session_surface_is_forwarded_only_by_the_solver_feature() {
    let manifest = include_str!("../Cargo.toml");
    let solver_feature = manifest
        .lines()
        .find(|line| line.starts_with("solver = "))
        .expect("html-play must declare its solver feature");
    let runtime_dependency = manifest
        .lines()
        .find(|line| line.starts_with("puzzle-game-runtime = "))
        .expect("html-play must declare its runtime dependency");

    assert!(solver_feature.contains("\"puzzle-game-runtime/solver-session\""));
    assert!(!runtime_dependency.contains("solver-session"));
}
