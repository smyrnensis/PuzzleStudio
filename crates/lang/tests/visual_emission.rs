use puzzle_lang::{RuleEffect, SourceTargetKind, analyze_source, parse_game2d};

fn visual_emission_source(effect: &str) -> String {
    format!(
        r#"
const title = visual_emission

puzzle default {{
layers {{
actor = Player
!flash
}}
visuals {{
visual Player {{
colors = #ffffff
shape = {{
0
}}
}}
visual !flash {{
duration = 240ms
colors = #ff0000
shape = {{
0
>
.
}}
}}
}}
rules {{
[ Player ] -> [ Player ] {effect}
}}
levels {{
legend {{
P = Player
}}
level "start" {{
P
}}
}}
}}
"#
    )
}

#[test]
fn visuals_use_one_asset_shape_for_object_bindings_and_explicit_emissions() {
    let loaded = parse_game2d(&visual_emission_source("!flash wait animation"))
        .expect("visual assets and emission should compile");

    assert!(
        loaded
            .visuals
            .aliases
            .iter()
            .any(|alias| { alias.object == "Player" && alias.visual == "Player" })
    );
    assert!(
        !loaded
            .visuals
            .aliases
            .iter()
            .any(|alias| alias.object == "flash")
    );
    let flash = loaded
        .visuals
        .entries
        .iter()
        .find(|visual| visual.name == "flash")
        .expect("named visual should use the same compiled asset type");
    assert_eq!(flash.frames.len(), 2);
    assert_eq!(flash.animation_duration_ms, Some(240));
    assert!(loaded.rule_effects.values().any(|effects| {
        effects.iter().any(|effect| {
            matches!(
                effect,
                RuleEffect::Runtime(puzzle_lang::RuntimeEffect::EmitAnimation {
                    name,
                    component: 0,
                    ..
                }) if name == "flash"
            )
        })
    }));
}

#[test]
fn explicit_visual_emission_must_be_declared_in_layers() {
    let source = visual_emission_source("!flash wait animation").replacen(
        "actor = Player\n!flash",
        "actor = Player",
        1,
    );
    let error = parse_game2d(&source).expect_err("undeclared animation priority must fail");

    assert!(
        error
            .to_string()
            .contains("visual animation is not declared in layers: !flash"),
        "{error}"
    );
}

#[test]
fn explicit_emission_requires_a_declared_visual() {
    let error = parse_game2d(&visual_emission_source("!missing"))
        .expect_err("unknown named visual must fail")
        .to_string();

    assert!(
        error.contains("unknown visual animation: !missing"),
        "{error}"
    );
}

#[test]
fn named_animation_visual_is_a_regular_visual_editor_target() {
    let source = visual_emission_source("!flash");
    let cursor = source.find("duration = 240ms").expect("visual body");
    let analysis = analyze_source(&source);
    let target = analysis
        .resolve_target(cursor)
        .expect("named visual should resolve as an editor target");

    assert_eq!(target.kind, SourceTargetKind::Visual);
    assert_eq!(target.name, "!flash");
}

#[test]
fn explicit_visual_emission_requires_a_rewrite_position() {
    let source =
        visual_emission_source("!flash").replace("[ Player ] -> [ Player ] !flash", "!flash");
    let error = parse_game2d(&source)
        .expect_err("unpositioned visual emission must fail")
        .to_string();

    assert!(
        error.contains("visual animation emission requires a rewrite match position"),
        "{error}"
    );
}
