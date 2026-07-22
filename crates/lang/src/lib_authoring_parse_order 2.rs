use std::collections::{BTreeMap, HashSet};

use puzzle_core::{LayerId, ObjectId};

use crate::{Catalog, DiagnosticReport, VisualOrderDef, VisualOrderPriorityDef, parse_error};

pub(crate) fn lower_visual_order(
    node: Option<&crate::authoring_grammar::AuthoringNode>,
    catalog: &Catalog,
    source_line: &str,
) -> Result<VisualOrderDef, DiagnosticReport> {
    let Some(node) = node else {
        return generated_order_from_slots(catalog, source_line);
    };
    let (direction_priority, items) = order_tree(node)?;

    validate_direction_priority(&direction_priority, catalog, source_line)?;
    if items.is_empty() {
        let mut generated = generated_order_from_slots(catalog, source_line)?;
        if !direction_priority.is_empty() {
            generated.direction_priority = direction_priority;
        }
        return Ok(generated);
    }
    let mut covered = HashSet::<ObjectId>::new();
    let mut priorities = Vec::new();
    for (_, selectors, merge) in items {
        let mut objects = Vec::new();
        for selector in &selectors {
            for object in resolve_order_selector(selector, catalog, source_line)? {
                if !objects.contains(&object) {
                    objects.push(object);
                }
            }
        }
        if objects.is_empty() {
            return Err(parse_error(
                source_line,
                "visual order priority matched no objects",
            ));
        }
        if !merge && !objects_share_slot(&objects, catalog) {
            return Err(parse_error(
                source_line,
                "plain visual order priority may contain only objects from one slot; use + or merge { ... } for cross-slot composition",
            ));
        }
        for object in &objects {
            if !covered.insert(*object) {
                return Err(parse_error(
                    source_line,
                    "visual order object is declared in more than one priority",
                ));
            }
        }
        let mut names = object_names(&objects, catalog, source_line)?;
        if merge {
            names.sort();
        }
        priorities.push(VisualOrderPriorityDef {
            objects: names,
            animations: Vec::new(),
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
            &format!("explicit visual order must cover every object; missing: {missing}"),
        ));
    }

    Ok(VisualOrderDef {
        direction_priority: if direction_priority.is_empty() {
            default_direction_priority(catalog)
        } else {
            direction_priority
        },
        priorities,
    })
}

fn order_tree(
    node: &crate::authoring_grammar::AuthoringNode,
) -> Result<(Vec<String>, Vec<(usize, Vec<String>, bool)>), DiagnosticReport> {
    use crate::authoring_grammar::{AuthoringDefinitionOp, AuthoringKind};

    let mut direction_priority = None;
    for definition in &node.definition_rows {
        if definition.key != "priority" || definition.op != Some(AuthoringDefinitionOp::Equals) {
            return Err(parse_error(
                &definition.source_line,
                "visual order property must be: priority = <direction...>",
            ));
        }
        if direction_priority
            .replace(definition.values.clone())
            .is_some()
        {
            return Err(parse_error(
                &definition.source_line,
                "visual order may declare priority only once",
            ));
        }
    }

    let mut items = node
        .content_rows
        .iter()
        .map(|row| {
            let merge = row.source_line.contains('+');
            let selectors = if merge {
                plus_operands(&row.source_line)?
            } else {
                order_row_selectors(row).to_vec()
            };
            Ok((row.source_index, selectors, merge))
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    for merge in &node.children {
        if merge.kind != AuthoringKind::VisualMergeConfig || merge.content_rows.is_empty() {
            return Err(parse_error(
                &merge.source_line,
                "visual merge must not be empty",
            ));
        }
        let selectors = merge
            .content_rows
            .iter()
            .flat_map(order_row_selectors)
            .cloned()
            .collect();
        items.push((merge.source_index, selectors, true));
    }
    items.sort_by_key(|(source_index, _, _)| *source_index);
    Ok((direction_priority.unwrap_or_default(), items))
}

fn order_row_selectors(row: &crate::authoring_grammar::AuthoringContentRow) -> &[String] {
    row.captures
        .iter()
        .find(|capture| capture.name == "selectors")
        .map_or(&[], |capture| capture.values.as_slice())
}

fn plus_operands(line: &str) -> Result<Vec<String>, DiagnosticReport> {
    line.split('+')
        .map(|operand| {
            let values = operand.split_whitespace().collect::<Vec<_>>();
            match values.as_slice() {
                [value] => Ok((*value).to_string()),
                _ => Err(parse_error(
                    line,
                    "visual merge + requires exactly one selector on each side",
                )),
            }
        })
        .collect()
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
                "cannot generate visual order for object without a slot",
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
                animations: Vec::new(),
                merge: false,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    Ok(VisualOrderDef {
        direction_priority: default_direction_priority(catalog),
        priorities,
    })
}

pub(crate) fn default_direction_priority(catalog: &Catalog) -> Vec<String> {
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

pub(crate) fn validate_direction_priority(
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
            &format!("visual order priority requires exactly {expected} directions for this model"),
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
                    &format!("unknown visual order direction: {direction}"),
                ));
            }
        };
        if !axes.insert(axis) {
            return Err(parse_error(
                source_line,
                "visual order priority must name each coordinate axis exactly once",
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
            &format!("ambiguous visual order reference: {selector}"),
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
        &format!("unknown visual order slot, object, or group: {selector}"),
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
                    "visual order object is missing its canonical name",
                )
            })
        })
        .collect()
}
