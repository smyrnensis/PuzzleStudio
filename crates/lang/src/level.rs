use std::collections::{HashMap, HashSet};

use puzzle_core::{CompiledGame, LayerId, ObjectId, State, StateError};

use crate::{
    DiagnosticReport, LevelRegionDef,
    model_syntax::RuleStatementsSyntax,
    source::LogicalLine,
    surface::{ParseProduct, ParserRecognition, SourceSpan, SurfaceDisplayFact},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LevelBlock {
    pub(crate) name: String,
    pub(crate) pack: Option<String>,
    pub(crate) puzzle: Option<String>,
    pub(crate) lines: Vec<LogicalLine>,
    pub(crate) legends: Vec<LevelLegendSyntax>,
    pub(crate) on_level_start: Vec<RuleStatementsSyntax>,
    pub(crate) on_level_clear: Vec<RuleStatementsSyntax>,
    pub(crate) rules_before: Option<RuleStatementsSyntax>,
    pub(crate) rules_after: Option<RuleStatementsSyntax>,
    pub(crate) level_start_effect_rows: Vec<LogicalLine>,
    pub(crate) level_clear_effect_rows: Vec<LogicalLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LevelLegendSyntax {
    pub(crate) source: LogicalLine,
    pub(crate) ch: char,
    pub(crate) selectors: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LevelResourceSyntax {
    pub(crate) legends: Vec<LevelLegendSyntax>,
    pub(crate) levels: Vec<LevelBlock>,
}

pub(crate) fn recognize_spatial_levels(
    model: &crate::model_syntax::PuzzleModelSyntax,
) -> ParseProduct<Result<(), DiagnosticReport>> {
    let mut recognition = ParserRecognition::default();
    let value = (|| {
        let known = model
            .body
            .levels
            .legends
            .iter()
            .map(|legend| legend.ch)
            .chain(std::iter::once('.'))
            .collect::<HashSet<_>>();
        for level in &model.body.levels.levels {
            for row in &level.lines {
                if row.text == "-" {
                    for token in &row.tokens {
                        if token.text == "-" {
                            recognition
                                .display_facts
                                .push(SurfaceDisplayFact::LevelSeparator {
                                    span: SourceSpan {
                                        start: token.start,
                                        end: token.end,
                                    },
                                });
                        }
                    }
                    continue;
                }
                for token in &row.tokens {
                    for (offset, ch) in token.text.char_indices() {
                        recognition.display_facts.push(SurfaceDisplayFact::LevelCell {
                            span: SourceSpan {
                                start: token.start + offset,
                                end: token.start + offset + ch.len_utf8(),
                            },
                            known: known.contains(&ch),
                        });
                    }
                }
            }
            split_spatial_level_slices(&level.lines)?;
        }
        Ok(())
    })();
    ParseProduct::new(value, recognition)
}

pub(crate) fn split_spatial_level_slices<Line: AsRef<str>>(
    rows: &[Line],
) -> Result<Vec<Vec<&Line>>, DiagnosticReport> {
    let mut slices = Vec::<Vec<&Line>>::new();
    let mut current = Vec::<&Line>::new();
    for row in rows {
        let text = row.as_ref();
        if text.trim().is_empty() {
            if !current.is_empty() {
                slices.push(std::mem::take(&mut current));
            }
            continue;
        }
        if text == "-" {
            if current.is_empty() {
                return Err(DiagnosticReport::error(
                    "3D level slice separator requires a preceding ASCII slice".to_string(),
                ));
            }
            slices.push(std::mem::take(&mut current));
            continue;
        }
        current.push(row);
    }
    if current.is_empty() && rows.last().is_some_and(|row| row.as_ref() == "-") {
        return Err(DiagnosticReport::error(
            "3D level slice separator requires a following ASCII slice".to_string(),
        ));
    }
    if !current.is_empty() {
        slices.push(current);
    }
    if slices.is_empty() {
        return Err(DiagnosticReport::error(
            "level requires at least one height slice".to_string(),
        ));
    }
    Ok(slices)
}

pub(crate) struct ParsedLevel {
    pub(crate) state: State,
    pub(crate) layer_states: Vec<State>,
    pub(crate) regions: Vec<LevelRegionDef>,
}

pub(crate) fn parse_level(
    game: &CompiledGame,
    lines: &[LogicalLine],
    empty: Option<char>,
    char_objects: &HashMap<char, Vec<ObjectId>>,
    variable_defaults: &[i64],
) -> ParseProduct<Result<ParsedLevel, DiagnosticReport>> {
    let recognition = recognize_level_display(lines, empty, char_objects);
    let value = (|| {
        let regions = split_regions(lines)?;
        let layout = lower_regions(&regions);
        let mut state = State::empty_with_variables(
            layout.width as u16,
            layout.height as u16,
            game.layer_count,
            game.object_count(),
            variable_defaults.to_vec(),
        )?;
        let authored_layer_count = layout
            .regions
            .iter()
            .map(|placed| placed.region.layers.len())
            .max()
            .unwrap_or(1);
        let mut layer_states = (0..authored_layer_count)
            .map(|_| {
                State::empty_with_variables(
                    layout.width as u16,
                    layout.height as u16,
                    game.layer_count,
                    game.object_count(),
                    variable_defaults.to_vec(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut placements = HashMap::<(usize, usize, LayerId), ObjectId>::new();
        for placed_region in &layout.regions {
            for (authored_layer, layer) in placed_region.region.layers.iter().enumerate() {
                for (y, row) in layer.iter().enumerate() {
                    for (x, ch) in row.chars().enumerate() {
                        if ch == '.' || Some(ch) == empty {
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
                                    position: puzzle_kernel::GridCoord::new([
                                        (placed_region.x + x) as u16,
                                        y as u16,
                                    ]),
                                    layer: object_layer,
                                    existing,
                                    attempted: *object,
                                }
                                .into());
                            }
                            placements.insert((placed_region.x + x, y, object_layer), *object);
                            layer_states[authored_layer].place_object(
                                game,
                                (placed_region.x + x) as u16,
                                y as u16,
                                *object,
                            )?;
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
            layer_states,
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
    })();
    ParseProduct::new(value, recognition)
}

fn recognize_level_display(
    lines: &[LogicalLine],
    empty: Option<char>,
    char_objects: &HashMap<char, Vec<ObjectId>>,
) -> ParserRecognition {
    let mut recognition = ParserRecognition::default();
    for line in lines {
        let Some(token) = line.tokens.iter().find(|token| token.text == line.text) else {
            continue;
        };
        if line.text == "+" {
            recognition
                .display_facts
                .push(SurfaceDisplayFact::LevelSeparator {
                    span: SourceSpan {
                        start: token.start,
                        end: token.end,
                    },
                });
            continue;
        }
        for (byte_offset, ch) in line.text.char_indices() {
            recognition
                .display_facts
                .push(SurfaceDisplayFact::LevelCell {
                    span: SourceSpan {
                        start: token.start + byte_offset,
                        end: token.start + byte_offset + ch.len_utf8(),
                    },
                    known: ch == '.' || Some(ch) == empty || char_objects.contains_key(&ch),
                });
        }
    }
    recognition
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

fn split_regions(lines: &[LogicalLine]) -> Result<Vec<LevelAsciiRegion>, DiagnosticReport> {
    let mut regions = Vec::<LevelAsciiRegion>::new();
    let mut current_layers = Vec::<Vec<String>>::new();
    let mut current_rows = Vec::<String>::new();
    for line in lines {
        if line.trim().is_empty() {
            flush_region(&mut regions, &mut current_layers, &mut current_rows)?;
            continue;
        }
        if line.as_ref() == "+" {
            if current_rows.is_empty() {
                return Err(DiagnosticReport::error(
                    "level layer separator requires a preceding ASCII layer".to_string(),
                ));
            }
            current_layers.push(std::mem::take(&mut current_rows));
            continue;
        }
        current_rows.push(line.text.clone());
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
