use crate::{
    count_pattern_matches, has_pattern_match, Game3, MatchCell3, ObjectId, Offset3, Pattern3,
    State3,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WinCondition3 {
    All(Vec<WinCondition3>),
    Any(Vec<WinCondition3>),
    SomeObject(ObjectId),
    NoObject(ObjectId),
    SomePattern(Pattern3),
    NoPattern(Pattern3),
    AllObjectsCoveredByPattern {
        object: ObjectId,
        cover_pattern: Pattern3,
    },
}

impl WinCondition3 {
    pub fn is_met(&self, game: &Game3, state: &State3) -> bool {
        match self {
            Self::All(conditions) => conditions
                .iter()
                .all(|condition| condition.is_met(game, state)),
            Self::Any(conditions) => conditions
                .iter()
                .any(|condition| condition.is_met(game, state)),
            Self::SomeObject(object) => {
                has_pattern_match(game, state, &single_object_pattern(*object))
            }
            Self::NoObject(object) => {
                !has_pattern_match(game, state, &single_object_pattern(*object))
            }
            Self::SomePattern(pattern) => has_pattern_match(game, state, pattern),
            Self::NoPattern(pattern) => !has_pattern_match(game, state, pattern),
            Self::AllObjectsCoveredByPattern {
                object,
                cover_pattern,
            } => {
                let object_count =
                    count_pattern_matches(game, state, &single_object_pattern(*object));
                object_count == count_pattern_matches(game, state, cover_pattern)
            }
        }
    }
}

fn single_object_pattern(object: ObjectId) -> Pattern3 {
    Pattern3::new(vec![MatchCell3::new(Offset3::ZERO).require(object)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord3, Direction3, LayerId, ObjectDef3, Size3};

    const BOX: ObjectId = ObjectId(1);
    const GOAL: ObjectId = ObjectId(2);
    const ACTOR: LayerId = LayerId(0);
    const FLOOR: LayerId = LayerId(1);

    fn support_game() -> Game3 {
        Game3::new(
            2,
            vec![
                ObjectDef3 {
                    id: BOX,
                    layer_id: ACTOR,
                },
                ObjectDef3 {
                    id: GOAL,
                    layer_id: FLOOR,
                },
            ],
        )
    }

    fn box_supported_by_goal_pattern() -> Pattern3 {
        Pattern3::new(vec![
            MatchCell3::new(Offset3::ZERO).require(BOX),
            MatchCell3::new(Direction3::DOWN.offset).require(GOAL),
        ])
    }

    #[test]
    fn down_pattern_matches_box_supported_by_goal() {
        let game = support_game();
        let mut state = State3::empty(Size3::new(3, 1, 2), game.layer_count).unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 1), BOX)
            .unwrap();
        state
            .place_object(&game, Coord3::new(1, 0, 0), GOAL)
            .unwrap();

        let pattern = box_supported_by_goal_pattern();

        assert!(has_pattern_match(&game, &state, &pattern));
        assert_eq!(count_pattern_matches(&game, &state, &pattern), 1);
    }

    #[test]
    fn all_objects_covered_by_pattern_uses_pattern_counts() {
        let game = support_game();
        let mut solved = State3::empty(Size3::new(3, 1, 2), game.layer_count).unwrap();
        solved
            .place_object(&game, Coord3::new(1, 0, 1), BOX)
            .unwrap();
        solved
            .place_object(&game, Coord3::new(1, 0, 0), GOAL)
            .unwrap();
        let win = WinCondition3::All(vec![
            WinCondition3::SomeObject(GOAL),
            WinCondition3::AllObjectsCoveredByPattern {
                object: GOAL,
                cover_pattern: box_supported_by_goal_pattern(),
            },
        ]);

        assert!(win.is_met(&game, &solved));

        let mut unsolved = solved.clone();
        unsolved
            .place_object(&game, Coord3::new(2, 0, 0), GOAL)
            .unwrap();

        assert!(!win.is_met(&game, &unsolved));
    }
}
