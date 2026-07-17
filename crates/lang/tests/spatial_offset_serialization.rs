use puzzle_core::{GridOffset, Offset};
use serde_json::json;

#[test]
fn serialized_offset_stores_only_the_axes_of_its_dimension() {
    let offset_2d = Offset::Fixed {
        delta: [4, -2].into(),
    };
    let offset_3d = GridOffset::<3>::Fixed {
        delta: [4, -2, 7].into(),
    };

    assert_eq!(
        serde_json::to_value(offset_2d).unwrap(),
        json!({ "Fixed": { "delta": [4, -2] } })
    );
    assert_eq!(
        serde_json::to_value(offset_3d).unwrap(),
        json!({ "Fixed": { "delta": [4, -2, 7] } })
    );
}

#[test]
fn deserialization_rejects_an_axis_count_from_another_dimension() {
    let error = serde_json::from_value::<Offset>(json!({
        "Fixed": { "delta": [4, -2, 0] }
    }))
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("expected 2 spatial axes, found 3")
    );
}
