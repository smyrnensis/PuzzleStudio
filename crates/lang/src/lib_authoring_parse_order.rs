use std::collections::HashSet;

use puzzle_core::ObjectId;

use crate::{Catalog, DiagnosticReport, VisualOrderPriorityDef, parse_error};

pub(crate) fn default_direction_priority(catalog: &Catalog) -> Vec<String> {
    if catalog.dimension == crate::ModelDimension::Three {
        vec!["down".to_string(), "right".to_string(), "front".to_string()]
    } else {
        vec!["down".to_string(), "right".to_string()]
    }
}

pub(crate) fn validate_direction_priority(
    directions: &[String],
    catalog: &Catalog,
    source_line: &str,
) -> Result<(), DiagnosticReport> {
    let is_3d = catalog.dimension == crate::ModelDimension::Three;
    let expected = if is_3d { 3 } else { 2 };
    if directions.len() != expected {
        return Err(parse_error(
            source_line,
            &format!("layers priority requires exactly {expected} directions for this model"),
        ));
    }
    let mut axes = HashSet::new();
    for direction in directions {
        let axis = match direction.as_str() {
            "left" | "right" => "x",
            "up" | "down" => "y",
            "front" | "back" if is_3d => "z",
            _ => {
                return Err(parse_error(
                    source_line,
                    &format!("unknown layers priority direction: {direction}"),
                ));
            }
        };
        if !axes.insert(axis) {
            return Err(parse_error(
                source_line,
                "layers priority must name each coordinate axis exactly once",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_layer_priorities(
    priorities: &[VisualOrderPriorityDef],
    catalog: &Catalog,
    source_line: &str,
) -> Result<(), DiagnosticReport> {
    let mut covered_objects = HashSet::new();
    let mut covered_animations = HashSet::new();
    for priority in priorities {
        if priority.objects.is_empty() && priority.animations.is_empty() {
            return Err(parse_error(source_line, "render layer must not be empty"));
        }
        for name in &priority.objects {
            let object = catalog.object_names.get(name).ok_or_else(|| {
                parse_error(source_line, &format!("unknown layer object: {name}"))
            })?;
            if !covered_objects.insert(*object) {
                return Err(parse_error(
                    source_line,
                    &format!("layer object is declared in more than one render priority: {name}"),
                ));
            }
        }
        for name in &priority.animations {
            if !covered_animations.insert(name) {
                return Err(parse_error(
                    source_line,
                    &format!("animation is declared in more than one render priority: !{name}"),
                ));
            }
        }
    }
    let all_objects = catalog
        .object_defs
        .iter()
        .map(|object| object.id)
        .collect::<HashSet<_>>();
    if covered_objects != all_objects {
        let mut missing = all_objects
            .difference(&covered_objects)
            .copied()
            .collect::<Vec<_>>();
        missing.sort_by_key(|object| object.0);
        let missing = object_names(&missing, catalog, source_line)?.join(" ");
        return Err(parse_error(
            source_line,
            &format!("layers must cover every object; missing: {missing}"),
        ));
    }
    Ok(())
}

fn object_names(
    objects: &[ObjectId],
    catalog: &Catalog,
    source_line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    objects
        .iter()
        .map(|object| {
            catalog.object_labels.get(object).cloned().ok_or_else(|| {
                parse_error(source_line, "layer object is missing its canonical name")
            })
        })
        .collect()
}
