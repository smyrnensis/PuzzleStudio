use puzzle_core::{Coord3, Size3};
use puzzle_lang::{LoadedDocumentModel, parse_game_for_path};

#[test]
fn puzzle3_ascii_columns_rows_and_slices_are_right_back_down_coordinates() {
    let source = r#"
puzzle coordinates {
dimension = 3
layers {
objects = A B C D E
}
rules {
restart
}
}

levels of coordinates {
legend {
A = A
B = B
C = C
D = D
E = E
. = empty
}

level "axes" {
AB
CD

E.
..
}
}
"#;
    let document = parse_game_for_path(source, "coordinates.puzzle").expect("3D axes parse");
    let game = match document.models.into_iter().next().expect("one model") {
        LoadedDocumentModel::Puzzle3d { game, .. } => game,
        LoadedDocumentModel::Puzzle2d { .. } => panic!("expected 3D model"),
    };
    let state = &game.levels[0].initial_state;
    let object = |name: &str| {
        game.object_labels
            .iter()
            .find_map(|(id, label)| (label == name).then_some(*id))
            .unwrap_or_else(|| panic!("missing object {name}"))
    };

    assert_eq!(state.size, Size3::new(2, 2, 2));
    assert!(state.has_object_at(&game.game, Coord3::new(0, 0, 0), object("A")));
    assert!(state.has_object_at(&game.game, Coord3::new(1, 0, 0), object("B")));
    assert!(state.has_object_at(&game.game, Coord3::new(0, 1, 0), object("C")));
    assert!(state.has_object_at(&game.game, Coord3::new(1, 1, 0), object("D")));
    assert!(state.has_object_at(&game.game, Coord3::new(0, 0, 1), object("E")));
}
