use std::collections::HashMap;

use puzzle_core::{CompiledGame, LayerId, ObjectId, State, StateError};

use crate::{DiagnosticReport, LevelRegionDef};

#[derive(Clone, Debug)]
pub(crate) struct LevelBlock {
    pub(crate) name: String,
    pub(crate) pack: Option<String>,
    pub(crate) puzzle: Option<String>,
    pub(crate) lines: Vec<String>,
}

pub(crate) struct ParsedLevel {
    pub(crate) state: State,
    pub(crate) regions: Vec<LevelRegionDef>,
}

pub(crate) fn parse_level(
    game: &CompiledGame,
    lines: &[String],
    empty: char,
    char_objects: &HashMap<char, Vec<ObjectId>>,
    variable_defaults: &[i64],
) -> Result<ParsedLevel, DiagnosticReport> {
    let regions = split_regions(lines)?;
    let layout = lower_regions(&regions);
    let mut state = State::empty_with_variables(
        layout.width as u16,
        layout.height as u16,
        game.layer_count,
        game.object_count(),
        variable_defaults.to_vec(),
    )?;

    let mut placements = HashMap::<(usize, usize, LayerId), ObjectId>::new();
    for placed_region in &layout.regions {
        for layer in &placed_region.region.layers {
            for (y, row) in layer.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if ch == empty {
                        continue;
                    }
                    let objects = char_objects.get(&ch).ok_or_else(|| {
                        DiagnosticReport::error(format!("unknown level char '{ch}'"))
                    })?;
                    let mut char_layers = HashMap::<LayerId, ObjectId>::new();
                    for object in objects {
                        let object_layer = game
                            .object_layer(*object)
                            .ok_or(StateError::UnknownObject { object: *object })?;
                        if let Some(existing) = char_layers.insert(object_layer, *object) {
                            return Err(StateError::LayerOccupied {
                                x: (placed_region.x + x) as u16,
                                y: y as u16,
                                layer: object_layer,
                                existing,
                                attempted: *object,
                            }
                            .into());
                        }
                        placements.insert((placed_region.x + x, y, object_layer), *object);
                    }
                }
            }
        }
    }
    for ((x, y, _), object) in placements {
        state.place_object(game, x as u16, y as u16, object)?;
    }

    Ok(ParsedLevel {
        state,
        regions: layout
            .regions
            .into_iter()
            .map(|placed| LevelRegionDef {
                index: placed.index,
                x: placed.x as u16,
                y: 0,
                width: placed.region.width as u16,
                height: placed.region.height as u16,
            })
            .collect(),
    })
}

#[derive(Clone, Debug)]
struct LevelAsciiRegion {
    layers: Vec<Vec<String>>,
    width: usize,
    height: usize,
}

#[derive(Clone, Debug)]
struct PlacedLevelAsciiRegion {
    index: usize,
    x: usize,
    region: LevelAsciiRegion,
}

#[derive(Clone, Debug)]
struct LoweredLevelLayout {
    width: usize,
    height: usize,
    regions: Vec<PlacedLevelAsciiRegion>,
}

fn split_regions(lines: &[String]) -> Result<Vec<LevelAsciiRegion>, DiagnosticReport> {
    let mut regions = Vec::<LevelAsciiRegion>::new();
    let mut current_layers = Vec::<Vec<String>>::new();
    let mut current_rows = Vec::<String>::new();
    for line in lines {
        if line.trim().is_empty() {
            flush_region(&mut regions, &mut current_layers, &mut current_rows)?;
            continue;
        }
        if line == "+" {
            if current_rows.is_empty() {
                return Err(DiagnosticReport::error(
                    "level layer separator requires a preceding ASCII layer".to_string(),
                ));
            }
            current_layers.push(std::mem::take(&mut current_rows));
            continue;
        }
        current_rows.push(line.clone());
    }
    flush_region(&mut regions, &mut current_layers, &mut current_rows)?;
    if regions.is_empty() {
        return Err(DiagnosticReport::error(
            "level requires at least one row".to_string(),
        ));
    }
    Ok(regions)
}

fn flush_region(
    regions: &mut Vec<LevelAsciiRegion>,
    current_layers: &mut Vec<Vec<String>>,
    current_rows: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    if !current_rows.is_empty() {
        current_layers.push(std::mem::take(current_rows));
    } else if !current_layers.is_empty() {
        return Err(DiagnosticReport::error(
            "level layer separator requires a following ASCII layer".to_string(),
        ));
    }
    if current_layers.is_empty() {
        return Ok(());
    }
    let region = validate_region_layers(std::mem::take(current_layers))?;
    regions.push(region);
    Ok(())
}

fn validate_region_layers(layers: Vec<Vec<String>>) -> Result<LevelAsciiRegion, DiagnosticReport> {
    let mut expected_size = None::<(usize, usize)>;
    for layer in &layers {
        let Some(first_row) = layer.first() else {
            return Err(DiagnosticReport::error(
                "level layer requires at least one row".to_string(),
            ));
        };
        let width = first_row.chars().count();
        if width == 0 || layer.iter().any(|row| row.chars().count() != width) {
            return Err(DiagnosticReport::error(
                "level regions must be rectangular".to_string(),
            ));
        }
        if layer.iter().any(|row| row.contains(['{', '}'])) {
            return Err(DiagnosticReport::error(
                "ASCII rows cannot contain braces".to_string(),
            ));
        }
        let size = (width, layer.len());
        if let Some(expected) = expected_size {
            if size != expected {
                return Err(DiagnosticReport::error(
                    "level ASCII layers in the same region must have the same size".to_string(),
                ));
            }
        } else {
            expected_size = Some(size);
        }
    }
    let (width, height) = expected_size.ok_or_else(|| {
        DiagnosticReport::error("level region requires at least one layer".to_string())
    })?;
    Ok(LevelAsciiRegion {
        layers,
        width,
        height,
    })
}

fn lower_regions(regions: &[LevelAsciiRegion]) -> LoweredLevelLayout {
    let gap = 2usize;
    let height = regions
        .iter()
        .map(|region| region.height)
        .max()
        .unwrap_or(0);
    let total_width = regions
        .iter()
        .enumerate()
        .map(|(index, region)| {
            let gap_width = if index == 0 { 0 } else { gap };
            gap_width + region.width
        })
        .sum::<usize>();

    let mut placed_regions = Vec::new();
    let mut x_offset = 0usize;
    for (index, region) in regions.iter().cloned().enumerate() {
        if index > 0 {
            x_offset += gap;
        }
        let width = region.width;
        placed_regions.push(PlacedLevelAsciiRegion {
            index,
            x: x_offset,
            region,
        });
        x_offset += width;
    }

    LoweredLevelLayout {
        width: total_width,
        height,
        regions: placed_regions,
    }
}
