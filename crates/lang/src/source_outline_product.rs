use crate::{
    source_outline::SourceOutlineItem,
    surface::{SurfaceDocument, SurfaceStructuralBlockRole},
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct OutlineStackEntry {
    id: Option<String>,
    suppress_children: bool,
}

/// Builds the parser-owned outline product from typed canonical block facts.
///
/// This is intentionally nested under `source`: outline consumers must project
/// the cached result and must not recognize source syntax themselves.
pub(crate) fn build_surface_outline_items(document: &SurfaceDocument) -> Vec<SourceOutlineItem> {
    let mut items = Vec::new();
    let mut stack = Vec::<OutlineStackEntry>::new();
    let mut ids_by_item_key = HashMap::<(usize, String, String), String>::new();
    let mut next_id = 0usize;

    for block in document
        .structural_blocks
        .iter()
        .filter(|block| matches!(block.role, SurfaceStructuralBlockRole::SourceTree))
    {
        while stack.len() > block.depth {
            stack.pop();
        }
        if stack.iter().any(|entry| entry.suppress_children) {
            stack.push(OutlineStackEntry {
                id: None,
                suppress_children: true,
            });
            continue;
        }
        let Some(outline) = &block.outline else {
            stack.push(OutlineStackEntry {
                id: None,
                suppress_children: true,
            });
            continue;
        };
        let id = push_item(
            &mut items,
            &mut ids_by_item_key,
            &stack,
            &mut next_id,
            block.start,
            block.end,
            outline.kind.clone(),
            outline.label.clone(),
        );
        stack.push(OutlineStackEntry {
            id: Some(id),
            suppress_children: outline.suppress_children,
        });
    }

    items
}

fn push_item(
    items: &mut Vec<SourceOutlineItem>,
    ids_by_item_key: &mut HashMap<(usize, String, String), String>,
    stack: &[OutlineStackEntry],
    next_id: &mut usize,
    start: usize,
    end: usize,
    kind: String,
    label: String,
) -> String {
    let key = (start, kind.clone(), label.clone());
    if let Some(id) = ids_by_item_key.get(&key) {
        return id.clone();
    }
    let id = format!("outline-{next_id}");
    *next_id += 1;
    let parent = stack.iter().rev().find_map(|entry| entry.id.clone());
    items.push(SourceOutlineItem {
        id: id.clone(),
        kind,
        label,
        start,
        end,
        depth: stack.iter().filter(|entry| entry.id.is_some()).count(),
        parent,
    });
    ids_by_item_key.insert(key, id.clone());
    id
}

#[cfg(test)]
mod tests {
    #[test]
    fn outline_product_is_constructed_only_by_source_analysis_cache() {
        let owner = include_str!("source_analysis.rs");
        assert_eq!(owner.matches("build_surface_outline_items").count(), 1);
        for (name, consumer) in [
            ("source_outline.rs", include_str!("source_outline.rs")),
            ("highlight.rs", include_str!("highlight.rs")),
            ("source_target.rs", include_str!("source_target.rs")),
            (
                "surface_completion.rs",
                include_str!("surface_completion.rs"),
            ),
        ] {
            let production = consumer
                .split("#[cfg(test)]")
                .next()
                .expect("consumer production source");
            assert!(
                !production.contains("build_surface_outline_items"),
                "{name} must consume the cached parser-owned outline product"
            );
        }
    }

    #[test]
    fn outline_product_reads_only_typed_block_facts() {
        let source = include_str!("source_outline_product.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("outline product source");
        for forbidden in [
            ".header",
            ".lines",
            ".content",
            "split_whitespace",
            "starts_with",
            "authoring_grammar",
            "AuthoringKind",
            "SourceScope",
            "selector",
        ] {
            assert!(
                !production.contains(forbidden),
                "outline product must consume typed block facts, not recognize source via {forbidden}"
            );
        }
    }
}
