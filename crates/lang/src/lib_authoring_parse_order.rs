use std::collections::{BTreeMap, HashSet};

use puzzle_core::{LayerId, ObjectId};

use crate::{Catalog, DiagnosticReport, VisualOrderDef, VisualOrderPriorityDef, parse_error};

pub(crate) fn lower_sprite_order(
    surface: Option<&puzzle_authoring::SpriteOrderSurface>,
    catalog: &Catalog,
    source_line: &str,
) -> Result<VisualOrderDef, DiagnosticReport> {
    let Some(surface) = surface else {
        return generated_order_from_slots(catalog, source_line);
    };

    validate_direction_priority(&surface.priority, catalog, source_line)?;
    if surface.items.is_empty() {
        let mut generated = generated_order_from_slots(catalog, source_line)?;
        if !surface.priority.is_empty() {
            generated.direction_priority = surface.priority.clone();
        }
        return Ok(generated);
    }
    let mut covered = HashSet::<ObjectId>::new();
    let mut priorities = Vec::new();
    for item in &surface.items {
        let (selectors, merge) = match item {
            puzzle_authoring::SpriteOrderItemSurface::Priority(selectors) => (selectors, false),
            puzzle_authoring::SpriteOrderItemSurface::Merge(selectors) => (selectors, true),
        };
        let mut objects = Vec::new();
        for selector in selectors {
            for object in resolve_order_selector(selector, catalog, source_line)? {
                if !objects.contains(&object) {
                    objects.push(object);
                }
            }
        }
        if objects.is_empty() {
            return Err(parse_error(
                source_line,
                "sprite order priority matched no objects",
            ));
        }
        if !merge && !objects_share_slot(&objects, catalog) {
            return Err(parse_error(
                source_line,
                "plain sprite order priority may contain only objects from one slot; use + or merge { ... } for cross-slot composition",
            ));
        }
        for object in &objects {
            if !covered.insert(*object) {
                return Err(parse_error(
                    source_line,
                    "sprite order object is declared in more than one priority",
                ));
            }
        }
        let mut names = object_names(&objects, catalog, source_line)?;
        if merge {
            names.sort();
        }
        priorities.push(VisualOrderPriorityDef {
            objects: names,
            merge,
        });
    }

    let all_objects = catalog
        .object_defs
        .iter()
        .map(|object| object.id)
        .collect::<HashSet<_>>();
    if covered != all_objects {
        let mut missing = all_objects
            .difference(&covered)
            .copied()
            .collect::<Vec<_>>();
        missing.sort_by_key(|object| object.0);
        let missing = object_names(&missing, catalog, source_line)?.join(" ");
        return Err(parse_error(
            source_line,
            &format!("explicit sprite order must cover every object; missing: {missing}"),
        ));
    }

    Ok(VisualOrderDef {
        direction_priority: if surface.priority.is_empty() {
            default_direction_priority(catalog)
        } else {
            surface.priority.clone()
        },
        priorities,
    })
}

fn generated_order_from_slots(
    catalog: &Catalog,
    source_line: &str,
) -> Result<VisualOrderDef, DiagnosticReport> {
    let mut slots = BTreeMap::<u16, Vec<ObjectId>>::new();
    for object in &catalog.object_defs {
        let layer = catalog.object_layers.get(&object.id).ok_or_else(|| {
            parse_error(
                source_line,
                "cannot generate sprite order for object without a slot",
            )
        })?;
        slots.entry(layer.0).or_default().push(object.id);
    }
    let priorities = slots
        .into_values()
        .map(|mut objects| {
            objects.sort_by_key(|object| object.0);
            Ok(VisualOrderPriorityDef {
                objects: object_names(&objects, catalog, source_line)?,
                merge: false,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    Ok(VisualOrderDef {
        direction_priority: default_direction_priority(catalog),
        priorities,
    })
}

fn default_direction_priority(catalog: &Catalog) -> Vec<String> {
    let is_3d = catalog
        .value_sets
        .get("directions")
        .is_some_and(|directions| directions.iter().any(|direction| direction == "front"));
    if is_3d {
        vec!["down".to_string(), "right".to_string(), "front".to_string()]
    } else {
        vec!["down".to_string(), "right".to_string()]
    }
}

fn validate_direction_priority(
    directions: &[String],
    catalog: &Catalog,
    source_line: &str,
) -> Result<(), DiagnosticReport> {
    if directions.is_empty() {
        return Ok(());
    }
    let is_3d = catalog
        .value_sets
        .get("directions")
        .is_some_and(|directions| directions.iter().any(|direction| direction == "front"));
    let expected = if is_3d { 3 } else { 2 };
    if directions.len() != expected {
        return Err(parse_error(
            source_line,
            &format!("sprite order priority requires exactly {expected} directions for this model"),
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
                    &format!("unknown sprite order direction: {direction}"),
                ));
            }
        };
        if !axes.insert(axis) {
            return Err(parse_error(
                source_line,
                "sprite order priority must name each coordinate axis exactly once",
            ));
        }
    }
    Ok(())
}

fn resolve_order_selector(
    selector: &str,
    catalog: &Catalog,
    source_line: &str,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let slot = catalog.named_layers.get(selector).copied();
    let object = catalog.object_names.get(selector).copied();
    let group = catalog.object_groups.get(selector);
    let match_count =
        usize::from(slot.is_some()) + usize::from(object.is_some()) + usize::from(group.is_some());
    if match_count > 1 {
        return Err(parse_error(
            source_line,
            &format!("ambiguous sprite order reference: {selector}"),
        ));
    }
    if let Some(layer) = slot {
        let layer = LayerId(layer);
        let mut objects = catalog
            .object_layers
            .iter()
            .filter_map(|(object, object_layer)| (*object_layer == layer).then_some(*object))
            .collect::<Vec<_>>();
        objects.sort_by_key(|object| object.0);
        return Ok(objects);
    }
    if let Some(object) = object {
        return Ok(vec![object]);
    }
    if let Some(group) = group {
        return Ok(group.clone());
    }
    Err(parse_error(
        source_line,
        &format!("unknown sprite order slot, object, or group: {selector}"),
    ))
}

fn objects_share_slot(objects: &[ObjectId], catalog: &Catalog) -> bool {
    let Some(first) = objects
        .first()
        .and_then(|object| catalog.object_layers.get(object))
    else {
        return false;
    };
    objects
        .iter()
        .all(|object| catalog.object_layers.get(object) == Some(first))
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
                parse_error(
                    source_line,
                    "sprite order object is missing its canonical name",
                )
            })
        })
        .collect()
}
