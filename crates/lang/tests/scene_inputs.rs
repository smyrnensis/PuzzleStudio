use puzzle_core::InputId;
use puzzle_lang::{KeyTrigger, SceneComponent, SceneEffect, parse_game2d as parse_game};

#[test]
fn scene_and_model_inputs_are_owner_scoped() {
    let source = r##"
title "scene inputs"

puzzle main {
layers {
  layer_1 = Player
}

inputs {
  action <- Enter Space Escape
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
  inputs {
    confirm <- Enter Space x
  }
  button "Play" -> input confirm
  rules {
    if input == confirm -> start levels in playing
  }
}

scene playing {
  state {
    board = puzzle main
  }
  view {
    puzzle board
  }
  inputs {
    back <- Escape q
  }
  rules {
    board.rules
    if input == back -> goto title
  }
}
"##;

    let loaded = parse_game(source).unwrap();
    let action = input_named(&loaded, "action");

    assert_eq!(loaded.controls.named.get("Enter"), Some(&action));
    assert_eq!(loaded.controls.named.get("Space"), Some(&action));
    assert_eq!(loaded.controls.named.get("Escape"), Some(&action));

    let title = &loaded.scenes[0];
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
        SceneEffect::Input("confirm".to_string())
    );
    assert!(matches!(
        title.components.iter().find(|component| matches!(component, SceneComponent::Button(_))),
        Some(SceneComponent::Button(button)) if button.effect == SceneEffect::Input("confirm".to_string())
    ));

    let playing = &loaded.scenes[1];
    assert_eq!(
        playing.key_bindings[0].effect,
        SceneEffect::Input("back".to_string())
    );
    assert_eq!(loaded.controls.keys.get(&b'q'), None);
}

fn input_named(loaded: &puzzle_lang::LoadedGame, name: &str) -> InputId {
    loaded
        .input_labels
        .iter()
        .find_map(|(id, label)| (label == name).then_some(*id))
        .unwrap_or_else(|| panic!("missing input {name}"))
}
