use std::collections::HashMap;

use puzzle_core::{
    GlobalId, InputId, LayerId, ObjectDef, ObjectId, QueryId, ScratchDef, ScratchId, ScratchKind,
};

#[derive(Clone, Debug)]
pub(crate) struct Catalog {
    pub(crate) value_sets: HashMap<String, Vec<String>>,
    pub(crate) object_axes: HashMap<String, Vec<String>>,
    pub(crate) maps: HashMap<String, ValueMap>,
    pub(crate) object_schemas: HashMap<String, ObjectSchema>,
    pub(crate) object_groups: HashMap<String, Vec<ObjectId>>,
    pub(crate) object_names: HashMap<String, ObjectId>,
    pub(crate) object_labels: HashMap<ObjectId, String>,
    pub(crate) object_layers: HashMap<ObjectId, LayerId>,
    pub(crate) object_defs: Vec<ObjectDef>,
    pub(crate) visual_objects: Vec<ObjectId>,
    pub(crate) scratch_defs: Vec<ScratchDef>,
    pub(crate) scratch_names: HashMap<String, ScratchDef>,
    pub(crate) render_chars: HashMap<ObjectId, char>,
    pub(crate) char_objects: HashMap<char, Vec<ObjectId>>,
    pub(crate) input_names: HashMap<String, InputId>,
    pub(crate) input_labels: HashMap<InputId, String>,
    pub(crate) global_names: HashMap<String, GlobalId>,
    pub(crate) global_labels: HashMap<GlobalId, String>,
    pub(crate) global_defaults: Vec<i64>,
    pub(crate) persistent_vars: Vec<GlobalId>,
    pub(crate) constant_globals: Vec<GlobalId>,
    pub(crate) query_names: HashMap<String, QueryId>,
    pub(crate) query_labels: HashMap<QueryId, String>,
}

impl Default for Catalog {
    fn default() -> Self {
        let mut value_sets = HashMap::new();
        value_sets.insert(
            "directions".to_string(),
            ["up", "down", "left", "right"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        );
        value_sets.insert(
            "horizontal".to_string(),
            ["left", "right"].into_iter().map(str::to_string).collect(),
        );
        value_sets.insert(
            "vertical".to_string(),
            ["up", "down"].into_iter().map(str::to_string).collect(),
        );

        let anonymous_movement = ScratchDef {
            id: ScratchId(0),
            kind: ScratchKind::Enum,
            values: ["up", "down", "left", "right"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };
        let anonymous_bool = ScratchDef {
            id: ScratchId(1),
            kind: ScratchKind::Bool,
            values: ["false", "true"].into_iter().map(str::to_string).collect(),
        };
        let anonymous_int = ScratchDef {
            id: ScratchId(2),
            kind: ScratchKind::Int,
            values: Vec::new(),
        };
        let action = ScratchDef {
            id: ScratchId(3),
            kind: ScratchKind::Bool,
            values: Vec::new(),
        };
        let move_collision = ScratchDef {
            id: ScratchId(4),
            kind: ScratchKind::Bool,
            values: Vec::new(),
        };

        let mut scratch_names = HashMap::new();
        scratch_names.insert("__move".to_string(), anonymous_movement.clone());
        scratch_names.insert("__action".to_string(), action.clone());
        scratch_names.insert("__move_collision".to_string(), move_collision.clone());

        Self {
            value_sets,
            object_axes: HashMap::new(),
            maps: HashMap::new(),
            object_schemas: HashMap::new(),
            object_groups: HashMap::new(),
            object_names: HashMap::new(),
            object_labels: HashMap::new(),
            object_layers: HashMap::new(),
            object_defs: Vec::new(),
            visual_objects: Vec::new(),
            scratch_defs: vec![
                anonymous_movement,
                anonymous_bool,
                anonymous_int,
                action,
                move_collision,
            ],
            scratch_names,
            render_chars: HashMap::new(),
            char_objects: HashMap::new(),
            input_names: HashMap::new(),
            input_labels: HashMap::new(),
            global_names: HashMap::new(),
            global_labels: HashMap::new(),
            global_defaults: Vec::new(),
            persistent_vars: Vec::new(),
            constant_globals: Vec::new(),
            query_names: HashMap::new(),
            query_labels: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ObjectSchema {
    pub(crate) axes: Vec<String>,
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
