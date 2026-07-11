use puzzle_core::InputId;
use puzzle_lang::{
    KeyTrigger, SceneComponent, SceneEffect, SceneEffectParam, SceneExpr, SceneTextContent,
    parse_game2d as parse_game,
};

#[test]
fn scene_and_model_inputs_are_owner_scoped() {
    let source = r##"
title = "scene inputs"

puzzle main {
layers {
  layer_1 = Player
}

keys {
  Enter Space Escape -> action
}

input action

sprites {
  Player
    #ffffff
}

rules {
}

levels {
  legend {
    . = empty
    P = Player
  }

  P
}
}

scene title {
  layout {
    button "Play" -> input confirm
  }
  keys {
    Enter Space x -> confirm
  }
  routine confirm {
    goto playing
  }
}

scene playing {
  layout {
    main
  }
  keys {
    Escape q -> back
  }
  rules {
    step main
  }
  routine back {
    goto title
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let action = input_named(&loaded, "action");

    assert_eq!(loaded.controls.named.get("Enter"), Some(&action));
    assert_eq!(loaded.controls.named.get("Space"), Some(&action));
    assert_eq!(loaded.controls.named.get("Escape"), Some(&action));

    let title = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .unwrap();
    assert_eq!(title.key_bindings.len(), 1);
    assert_eq!(
        title.key_bindings[0].keys,
        vec![
            KeyTrigger::Named("Enter".to_string()),
            KeyTrigger::Named("Space".to_string()),
            KeyTrigger::Char('x')
        ]
    );
    assert_eq!(
        title.key_bindings[0].effect,
        SceneEffect::RoutineCall("confirm".to_string())
    );
    assert!(matches!(
        title.components.iter().find(|component| matches!(component, SceneComponent::Button(_))),
        Some(SceneComponent::Button(button)) if button.effect == SceneEffect::Input("confirm".to_string())
    ));

    let playing = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "playing")
        .unwrap();
    assert_eq!(
        playing.key_bindings[0].effect,
        SceneEffect::RoutineCall("back".to_string())
    );
    assert_eq!(loaded.controls.keys.get(&b'q'), None);
}

#[test]
fn scene_choice_uses_arrow_effect_syntax() {
    let source = r##"
title = "choices"

puzzle main {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  P
}
}

scene title {
  layout {
    choice "Start" -> goto playing
    button "Help" -> message "help"
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let title = &loaded.scenes[0];
    assert!(matches!(
        title.components.first(),
        Some(SceneComponent::Choice(choice))
            if matches!(&choice.effect, SceneEffect::Goto { scene, params }
                if scene == "playing" && params.is_empty())
    ));
    assert!(matches!(
        title.components.get(1),
        Some(SceneComponent::Button(button))
            if matches!(button.effect, SceneEffect::Message { .. })
    ));
}

#[test]
fn scene_layout_if_else_controls_components() {
    let source = r##"
title = "conditional view"

puzzle main {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  P
}
}

scene title {
  layout {
    if game.has_progress_save {
      choice "Continue" -> input continue_game
    } else {
      choice "New Game" -> input new_game
    }
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let title = &loaded.scenes[0];
    let Some(SceneComponent::Conditional(conditional)) = title.components.first() else {
        panic!("expected conditional component");
    };
    assert!(matches!(
        &conditional.condition,
        SceneExpr::Path(path) if path == &vec!["game".to_string(), "has_progress_save".to_string()]
    ));
    assert!(matches!(
        conditional.children.as_slice(),
        [SceneComponent::Choice(choice)] if choice.effect == SceneEffect::Input("continue_game".to_string())
    ));
    assert!(matches!(
        conditional.else_children.as_slice(),
        [SceneComponent::Choice(choice)] if choice.effect == SceneEffect::Input("new_game".to_string())
    ));
}

#[test]
fn scene_if_is_runtime_value_and_fragment_syntax() {
    let source = r##"
title = "conditional values"

puzzle main {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  P
}
}

scene title {
  layout {
    text if game.has_progress_save { "Continue" } else { "New Game" }
    button join("Save: ", if game.has_progress_save { "yes" } else { "no" }) -> input confirm
    if game.has_progress_save {
      choice "Continue" -> input continue_game
    } else {
      choice "New Game" -> input new_game
    }
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let title = &loaded.scenes[0];
    assert!(matches!(
        &title.components[0],
        SceneComponent::Text(text)
            if matches!(&text.content, SceneTextContent::Expr(SceneExpr::If { .. }))
    ));
    assert!(matches!(
        &title.components[1],
        SceneComponent::Button(button)
            if matches!(&button.label, SceneExpr::Call { name, args }
                if name == "join" && args.iter().any(|arg| matches!(arg, SceneExpr::If { .. })))
    ));
    assert!(matches!(
        &title.components[2],
        SceneComponent::Conditional(conditional)
            if matches!(&conditional.condition, SceneExpr::Path(path)
                if path == &vec!["game".to_string(), "has_progress_save".to_string()])
    ));
}

#[test]
fn scene_if_value_uses_universal_balanced_brace_groups() {
    let source = r##"
title = "nested conditional values"

puzzle main {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  P
}
}

scene title {
  layout {
    text if game.has_progress_save { if game.has_progress_save { "Continue }" } else { "Resume" } } else { "New Game" }
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    assert!(matches!(
        &loaded.scenes[0].components[0],
        SceneComponent::Text(text)
            if matches!(&text.content,
                SceneTextContent::Expr(SceneExpr::If { then_branch, .. })
                    if matches!(then_branch.as_ref(), SceneExpr::If { .. }))
    ));
}

#[test]
fn scene_level_entry_uses_goto_scene_call_syntax() {
    let source = r##"
title = "level calls"

puzzle sokoban {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  level "one" {
    P
  }
}
}

scene title {
  layout {
    choice "Start" -> goto playing("one")
  }
}

scene playing(level) {
  layout {
    sokoban
  }
  rules {
    step sokoban
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let title = &loaded.scenes[0];
    assert!(matches!(
        title.components.first(),
        Some(SceneComponent::Choice(choice))
            if matches!(&choice.effect, SceneEffect::Goto { scene, params }
                if scene == "playing"
                    && matches!(params.as_slice(), [SceneEffectParam::Level(_)])
            )
    ));
    let playing = loaded
        .scenes
        .iter()
        .find(|scene| scene.name == "playing")
        .expect("playing scene");
    assert!(matches!(
        playing.state.puzzles.as_slice(),
        [puzzle] if puzzle.name == "sokoban" && puzzle.model == "sokoban"
    ));
}

#[test]
fn old_start_levels_syntax_reports_canonical_goto() {
    let source = r##"
title = "old level start"

puzzle main {
layers {
  layer_1 = Player
}
sprites {
  Player
    #ffffff
}
rules {
}
levels {
  legend {
    . = empty
    P = Player
  }
  P
}
}

scene title {
  layout {
    choice "Start" -> start levels in playing
  }
}
"##;

    let error = parse_game(source).unwrap_err().to_string();
    assert!(error.contains("no longer supported"));
    assert!(error.contains("goto <scene>(<level>)"));
}

fn input_named(loaded: &puzzle_lang::LoadedGame, name: &str) -> InputId {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == name).then_some(*id))
        .unwrap_or_else(|| panic!("missing input {name}"))
}
