use std::collections::HashSet;

use puzzle_authoring::{EditorDraftCell2d, EditorDraftCell3d, EditorDraftState};
use puzzle_core::GridSize;
use puzzle_runtime_contract::{
    RuntimeModelKind, RuntimeStateSnapshot, RuntimeStateSnapshot2d, RuntimeStateSnapshot3d,
};

use crate::{LoadedDocument, LoadedDocumentModel, LoadedGridGame};

pub fn resolve_editor_draft(
    document: &LoadedDocument,
    model_name: &str,
    level_index: usize,
    draft: &EditorDraftState,
) -> Result<RuntimeStateSnapshot, String> {
    let model = document
        .models
        .iter()
        .find(|model| match model {
            LoadedDocumentModel::Puzzle2d { name, .. }
            | LoadedDocumentModel::Puzzle3d { name, .. } => name == model_name,
        })
        .ok_or_else(|| format!("editor draft model `{model_name}` does not exist"))?;

    match (model, draft) {
        (LoadedDocumentModel::Puzzle2d { game, .. }, EditorDraftState::Grid2d(level)) => {
            let resolved = resolve_grid_draft(
                game,
                level_index,
                [level.size.width, level.size.height],
                level.cells.iter().map(|cell: &EditorDraftCell2d| {
                    ([cell.position.x, cell.position.y], &cell.symbol)
                }),
            )?;
            Ok(RuntimeStateSnapshot::TwoD(RuntimeStateSnapshot2d {
                kind: RuntimeModelKind::TwoD,
                width: level.size.width,
                height: level.size.height,
                layer_count: resolved.layer_count,
                slots: resolved.slots,
                variables: resolved.variables,
                level_fired_rules: resolved.level_fired_rules,
            }))
        }
        (LoadedDocumentModel::Puzzle3d { game, .. }, EditorDraftState::Grid3d(level)) => {
            let resolved = resolve_grid_draft(
                game,
                level_index,
                [level.size.width, level.size.depth, level.size.height],
                level.cells.iter().map(|cell: &EditorDraftCell3d| {
                    (
                        [cell.position.x, cell.position.y, cell.position.z],
                        &cell.symbol,
                    )
                }),
            )?;
            Ok(RuntimeStateSnapshot::ThreeD(RuntimeStateSnapshot3d {
                kind: RuntimeModelKind::ThreeD,
                width: level.size.width,
                depth: level.size.depth,
                height: level.size.height,
                layer_count: resolved.layer_count,
                slots: resolved.slots,
                variables: resolved.variables,
                level_fired_rules: resolved.level_fired_rules,
            }))
        }
        (LoadedDocumentModel::Puzzle2d { .. }, EditorDraftState::Grid3d(_)) => Err(format!(
            "editor draft dimension mismatch: model `{model_name}` is grid2d but draft is grid3d"
        )),
        (LoadedDocumentModel::Puzzle3d { .. }, EditorDraftState::Grid2d(_)) => Err(format!(
            "editor draft dimension mismatch: model `{model_name}` is grid3d but draft is grid2d"
        )),
    }
}

struct ResolvedGridDraft {
    layer_count: u16,
    slots: Vec<u16>,
    variables: Vec<i64>,
    level_fired_rules: Vec<u16>,
}

fn resolve_grid_draft<'a, const D: usize, Size, Cells>(
    game: &LoadedGridGame<D, Size>,
    level_index: usize,
    axes: [u16; D],
    cells: Cells,
) -> Result<ResolvedGridDraft, String>
where
    Size: GridSize<D>,
    Cells: IntoIterator<Item = ([u16; D], &'a String)>,
{
    let level = game
        .levels
        .get(level_index)
        .ok_or_else(|| format!("editor draft level index {level_index} is out of range"))?;
    if axes.iter().any(|axis| *axis == 0) {
        return Err("editor draft dimensions must be positive".to_string());
    }
    let cell_count = axes
        .iter()
        .try_fold(1usize, |count, axis| count.checked_mul(usize::from(*axis)))
        .ok_or_else(|| "editor draft dimensions are too large".to_string())?;
    let layer_count = game.game.layer_count;
    let slot_count = cell_count
        .checked_mul(usize::from(layer_count))
        .ok_or_else(|| "editor draft dimensions are too large".to_string())?;

    let mut slots = vec![0; slot_count];
    let mut occupied_cells = HashSet::new();
    for (position, symbol_text) in cells {
        let symbol = one_symbol(symbol_text)?;
        let objects = level.legend.get(&symbol).ok_or_else(|| {
            format!(
                "editor draft cell references symbol `{symbol}` not defined by compiled level {level_index}"
            )
        })?;
        let cell = cell_index(&axes, position)?;
        if !occupied_cells.insert(cell) {
            return Err(format!(
                "editor draft defines cell {} more than once",
                format_position(position)
            ));
        }
        for object in objects {
            let layer = game
                .game
                .object_layer(*object)
                .ok_or_else(|| format!("editor draft object {} has no compiled layer", object.0))?;
            slots[cell * usize::from(layer_count) + usize::from(layer.0)] = object.0;
        }
    }

    Ok(ResolvedGridDraft {
        layer_count,
        slots,
        variables: level.initial_state.visible_variables().to_vec(),
        level_fired_rules: level
            .initial_state
            .level_fired_rules()
            .iter()
            .map(|rule| rule.0)
            .collect(),
    })
}

fn one_symbol(value: &str) -> Result<char, String> {
    let mut chars = value.chars();
    let symbol = chars
        .next()
        .ok_or_else(|| "editor draft symbol must contain exactly one character".to_string())?;
    if chars.next().is_some() {
        return Err(format!(
            "editor draft symbol `{value}` must contain exactly one character"
        ));
    }
    Ok(symbol)
}

fn cell_index<const D: usize>(axes: &[u16; D], position: [u16; D]) -> Result<usize, String> {
    let mut index = 0usize;
    let mut stride = 1usize;
    for axis in 0..D {
        if position[axis] >= axes[axis] {
            return Err(format!(
                "editor draft cell {} is outside dimensions {}",
                format_position(position),
                format_position(*axes)
            ));
        }
        index += usize::from(position[axis]) * stride;
        stride *= usize::from(axes[axis]);
    }
    Ok(index)
}

fn format_position<const D: usize>(position: [u16; D]) -> String {
    let coordinates = position
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("({coordinates})")
}

#[cfg(test)]
mod tests {
    use puzzle_authoring::{
        EditorDraftCell2d, EditorDraftCell3d, EditorDraftLevel2d, EditorDraftLevel3d,
        EditorDraftPosition2d, EditorDraftPosition3d, EditorDraftSize2d, EditorDraftSize3d,
        EditorDraftState,
    };
    use puzzle_runtime_contract::RuntimeStateSnapshot;

    use super::resolve_editor_draft;

    fn document() -> crate::LoadedDocument {
        crate::parse_game_for_path(
            r#"
puzzle board2 {
  layers {
    floor = Floor
    actor = Player Box
  }
  rules {}
}

levels demo2 of board2 {
  legend {
    . = Floor
    P = Floor Player
  }
  level "one" {
    P
  }
}

puzzle board3 {
  dimension = 3
  layers {
    floor = Floor3
    actor = Player3 Box3
  }
  rules {}
}

levels demo3 of board3 {
  legend {
    . = Floor3
    P = Floor3 Player3
  }
  level "one" {
    P
  }
}
"#,
            "editor_draft.puzzle",
        )
        .unwrap()
    }

    #[test]
    fn resolves_grid2d_and_grid3d_through_one_semantic_owner() {
        let document = document();
        let grid2d = EditorDraftState::Grid2d(EditorDraftLevel2d {
            size: EditorDraftSize2d {
                width: 2,
                height: 1,
            },
            cells: vec![
                EditorDraftCell2d {
                    position: EditorDraftPosition2d { x: 0, y: 0 },
                    symbol: "P".to_string(),
                },
                EditorDraftCell2d {
                    position: EditorDraftPosition2d { x: 1, y: 0 },
                    symbol: ".".to_string(),
                },
            ],
        });
        let grid3d = EditorDraftState::Grid3d(EditorDraftLevel3d {
            size: EditorDraftSize3d {
                width: 1,
                depth: 1,
                height: 2,
            },
            cells: vec![
                EditorDraftCell3d {
                    position: EditorDraftPosition3d { x: 0, y: 0, z: 0 },
                    symbol: "P".to_string(),
                },
                EditorDraftCell3d {
                    position: EditorDraftPosition3d { x: 0, y: 0, z: 1 },
                    symbol: ".".to_string(),
                },
            ],
        });

        let RuntimeStateSnapshot::TwoD(two_d) =
            resolve_editor_draft(&document, "board2", 0, &grid2d).unwrap()
        else {
            panic!("expected grid2d state");
        };
        assert_eq!((two_d.width, two_d.height), (2, 1));
        assert_eq!(two_d.slots.len(), 4);
        assert_ne!(two_d.slots[0], 0);
        assert_ne!(two_d.slots[1], 0);
        assert_ne!(two_d.slots[2], 0);
        assert_eq!(two_d.slots[3], 0);

        let RuntimeStateSnapshot::ThreeD(three_d) =
            resolve_editor_draft(&document, "board3", 0, &grid3d).unwrap()
        else {
            panic!("expected grid3d state");
        };
        assert_eq!((three_d.width, three_d.depth, three_d.height), (1, 1, 2));
        assert_eq!(three_d.slots.len(), 4);
        assert_ne!(three_d.slots[0], 0);
        assert_ne!(three_d.slots[1], 0);
        assert_ne!(three_d.slots[2], 0);
        assert_eq!(three_d.slots[3], 0);
    }

    #[test]
    fn rejects_dimension_mismatch_before_resolving_cells() {
        let draft = EditorDraftState::Grid2d(EditorDraftLevel2d {
            size: EditorDraftSize2d {
                width: 1,
                height: 1,
            },
            cells: Vec::new(),
        });
        assert_eq!(
            resolve_editor_draft(&document(), "board3", 0, &draft),
            Err(
                "editor draft dimension mismatch: model `board3` is grid3d but draft is grid2d"
                    .to_string()
            )
        );
    }

    #[test]
    fn rejects_symbols_not_owned_by_the_compiled_level() {
        let draft = EditorDraftState::Grid3d(EditorDraftLevel3d {
            size: EditorDraftSize3d {
                width: 1,
                depth: 1,
                height: 1,
            },
            cells: vec![EditorDraftCell3d {
                position: EditorDraftPosition3d { x: 0, y: 0, z: 0 },
                symbol: "X".to_string(),
            }],
        });
        assert_eq!(
            resolve_editor_draft(&document(), "board3", 0, &draft),
            Err(
                "editor draft cell references symbol `X` not defined by compiled level 0"
                    .to_string()
            )
        );
    }
}
