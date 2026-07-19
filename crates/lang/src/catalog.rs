use std::collections::HashMap;

use puzzle_core::{
    ConditionId, InputId, LayerId, MarkDef, MarkId, MarkKind, ObjectDef, ObjectId, VariableId,
};

#[derive(Clone, Debug)]
pub(crate) struct Catalog {
    pub(crate) dimension: crate::ModelDimension,
    pub(crate) layer_count: Option<u16>,
    pub(crate) named_layers: HashMap<String, u16>,
    pub(crate) value_sets: HashMap<String, Vec<String>>,
    pub(crate) object_axes: HashMap<String, Vec<String>>,
    pub(crate) axis_types: HashMap<String, ValueType>,
    pub(crate) maps: HashMap<String, ValueMap>,
    pub(crate) object_schemas: HashMap<String, ObjectSchema>,
    pub(crate) object_groups: HashMap<String, Vec<ObjectId>>,
    pub(crate) object_names: HashMap<String, ObjectId>,
    pub(crate) object_labels: HashMap<ObjectId, String>,
    pub(crate) object_layers: HashMap<ObjectId, LayerId>,
    pub(crate) object_defs: Vec<ObjectDef>,
    pub(crate) mark_defs: Vec<MarkDef>,
    pub(crate) mark_names: HashMap<String, MarkDef>,
    pub(crate) render_chars: HashMap<ObjectId, char>,
    pub(crate) char_objects: HashMap<char, Vec<ObjectId>>,
    pub(crate) input_names: HashMap<String, InputId>,
    pub(crate) input_labels: HashMap<InputId, String>,
    pub(crate) variable_names: HashMap<String, VariableId>,
    pub(crate) variable_labels: HashMap<VariableId, String>,
    pub(crate) variable_defaults: Vec<i64>,
    pub(crate) numeric_variable_defaults: HashMap<String, i64>,
    pub(crate) persistent_vars: Vec<VariableId>,
    pub(crate) constant_variables: Vec<VariableId>,
    pub(crate) condition_names: HashMap<String, ConditionId>,
    pub(crate) condition_labels: HashMap<ConditionId, String>,
}

impl Catalog {
    pub(crate) fn for_dimension(dimension: crate::ModelDimension) -> Self {
        let dimensions = match dimension {
            crate::ModelDimension::Two => 2,
            crate::ModelDimension::Three => 3,
        };
        let mut value_sets = HashMap::new();
        let mut axis_types = HashMap::new();
        for name in puzzle_authoring::ABSOLUTE_DIRECTION_SET_NAMES {
            let Some(values) = puzzle_authoring::movement_mark_set_values(name, dimensions) else {
                continue;
            };
            value_sets.insert(
                (*name).to_string(),
                values.iter().copied().map(str::to_string).collect(),
            );
            axis_types.insert((*name).to_string(), ValueType::Direction);
        }
        let directions = value_sets
            .get("directions")
            .cloned()
            .expect("every model dimension defines directions");
        if dimension == crate::ModelDimension::Two {
            value_sets.insert(
                "parallel".to_string(),
                ["<", ">"].into_iter().map(str::to_string).collect(),
            );
            value_sets.insert(
                "perpendicular".to_string(),
                ["^", "v"].into_iter().map(str::to_string).collect(),
            );
        }

        let anonymous_movement = MarkDef {
            id: MarkId(0),
            kind: MarkKind::Enum,
            values: directions,
        };
        let anonymous_bool = MarkDef {
            id: MarkId(1),
            kind: MarkKind::Bool,
            values: ["false", "true"].into_iter().map(str::to_string).collect(),
        };
        let anonymous_int = MarkDef {
            id: MarkId(2),
            kind: MarkKind::Int,
            values: Vec::new(),
        };
        let action = MarkDef {
            id: MarkId(3),
            kind: MarkKind::Bool,
            values: Vec::new(),
        };
        let move_collision = MarkDef {
            id: MarkId(4),
            kind: MarkKind::Bool,
            values: Vec::new(),
        };

        let mut mark_names = HashMap::new();
        mark_names.insert("__move".to_string(), anonymous_movement.clone());
        mark_names.insert("__action".to_string(), action.clone());
        mark_names.insert("__move_collision".to_string(), move_collision.clone());

        Self {
            dimension,
            layer_count: None,
            named_layers: HashMap::new(),
            value_sets,
            object_axes: HashMap::new(),
            axis_types,
            maps: HashMap::new(),
            object_schemas: HashMap::new(),
            object_groups: HashMap::new(),
            object_names: HashMap::new(),
            object_labels: HashMap::new(),
            object_layers: HashMap::new(),
            object_defs: Vec::new(),
            mark_defs: vec![
                anonymous_movement,
                anonymous_bool,
                anonymous_int,
                action,
                move_collision,
            ],
            mark_names,
            render_chars: HashMap::new(),
            char_objects: HashMap::new(),
            input_names: HashMap::new(),
            input_labels: HashMap::new(),
            variable_names: HashMap::new(),
            variable_labels: HashMap::new(),
            variable_defaults: Vec::new(),
            numeric_variable_defaults: HashMap::new(),
            persistent_vars: Vec::new(),
            constant_variables: Vec::new(),
            condition_names: HashMap::new(),
            condition_labels: HashMap::new(),
        }
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::for_dimension(crate::ModelDimension::Two)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectSchema {
    pub(crate) axes: Vec<String>,
    pub(crate) axis_types: Vec<Option<ValueType>>,
    pub(crate) variants: Vec<ObjectVariant>,
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectVariant {
    pub(crate) values: Vec<String>,
    pub(crate) object: ObjectId,
}

#[derive(Clone, Debug)]
pub(crate) struct ValueMap {
    pub(crate) name: String,
    pub(crate) axis: String,
    pub(crate) values: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueType {
    Int,
    Rational,
    Bool,
    String,
    Angle,
    Vec2,
    Frame3,
    Direction,
    Nominal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Rational {
    pub(crate) numerator: i64,
    pub(crate) denominator: i64,
}

impl Rational {
    pub(crate) const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub(crate) fn new(numerator: i64, denominator: i64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let mut numerator = numerator;
        let mut denominator = denominator;
        if denominator < 0 {
            numerator = -numerator;
            denominator = -denominator;
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u64) as i64;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub(crate) fn integer(value: i64) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub(crate) fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.denominator + other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
        .expect("adding rationals with non-zero denominators keeps a non-zero denominator")
    }

    pub(crate) fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }

    pub(crate) fn mul(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
        .expect("multiplying rationals with non-zero denominators keeps a non-zero denominator")
    }

    pub(crate) fn neg(self) -> Self {
        Self {
            numerator: -self.numerator,
            denominator: self.denominator,
        }
    }

    pub(crate) fn cmp(self, other: Self) -> std::cmp::Ordering {
        (self.numerator * other.denominator).cmp(&(other.numerator * self.denominator))
    }

    pub(crate) fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub(crate) fn format(self) -> String {
        if self.denominator == 1 {
            self.numerator.to_string()
        } else {
            format!("{}/{}", self.numerator, self.denominator)
        }
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

#[cfg(test)]
mod tests {
    use super::{Catalog, ValueType};

    #[test]
    fn cartesian_direction_sets_follow_the_model_dimension() {
        let two = Catalog::for_dimension(crate::ModelDimension::Two);
        assert_eq!(two.value_sets["x_axis"], ["left", "right"]);
        assert_eq!(two.value_sets["y_axis"], ["up", "down"]);
        assert_eq!(two.value_sets["xy_plane"], ["up", "down", "left", "right"]);
        assert_eq!(two.value_sets["horizontal"], two.value_sets["x_axis"]);
        assert_eq!(two.value_sets["vertical"], two.value_sets["y_axis"]);
        assert_eq!(two.value_sets["directions"], two.value_sets["xy_plane"]);
        for unavailable in ["z_axis", "yz_plane", "xz_plane"] {
            assert!(!two.value_sets.contains_key(unavailable));
        }

        let three = Catalog::for_dimension(crate::ModelDimension::Three);
        assert_eq!(three.value_sets["x_axis"], ["left", "right"]);
        assert_eq!(three.value_sets["y_axis"], ["front", "back"]);
        assert_eq!(three.value_sets["z_axis"], ["up", "down"]);
        assert_eq!(
            three.value_sets["xy_plane"],
            ["left", "right", "front", "back"]
        );
        assert_eq!(
            three.value_sets["yz_plane"],
            ["up", "down", "front", "back"]
        );
        assert_eq!(
            three.value_sets["xz_plane"],
            ["up", "down", "left", "right"]
        );
        assert_eq!(three.value_sets["horizontal"], three.value_sets["xy_plane"]);
        assert_eq!(three.value_sets["vertical"], three.value_sets["z_axis"]);
        for name in puzzle_authoring::ABSOLUTE_DIRECTION_SET_NAMES {
            if three.value_sets.contains_key(*name) {
                assert_eq!(three.axis_types[*name], ValueType::Direction);
            }
        }
    }
}
