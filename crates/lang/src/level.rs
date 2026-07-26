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
    pub(crate) source: LogicalLine,
    pub(crate) name: String,
    pub(crate) source_name: String,
    pub(crate) source_span: SourceSpan,
    pub(crate) body_span: SourceSpan,
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
                        recognition
                            .display_facts
                            .push(SurfaceDisplayFact::LevelCell {
                                span: SourceSpan {
                                    start: token.start + offset,
                                    end: token.start + offset + ch.len_utf8(),
                                },
                                known: known.contains(&ch),
                            });
                    }
                }
            }
            split_spatial_level_slices(&level.source, &level.lines)?;
        }
        Ok(())
    })();
    ParseProduct::new(value, recognition)
}

pub(crate) fn split_spatial_level_slices<'a>(
    source: &LogicalLine,
    rows: &'a [LogicalLine],
) -> Result<Vec<Vec<&'a LogicalLine>>, DiagnosticReport> {
    let mut slices = Vec::<Vec<&'a LogicalLine>>::new();
    let mut current = Vec::<&'a LogicalLine>::new();
    for row in rows {
        let text = row.text.as_str();
        if text.trim().is_empty() {
            if !current.is_empty() {
                slices.push(std::mem::take(&mut current));
            }
            continue;
        }
        if text == "-" {
            if current.is_empty() {
                return Err(error_at(
                    row,
                    "3D level slice separator requires a preceding ASCII slice",
                ));
            }
            slices.push(std::mem::take(&mut current));
            continue;
        }
        current.push(row);
    }
    if current.is_empty() && rows.last().is_some_and(|row| row.text == "-") {
        return Err(error_at(
            rows.last().expect("a trailing separator exists"),
            "3D level slice separator requires a following ASCII slice",
        ));
    }
    if !current.is_empty() {
        slices.push(current);
    }
    if slices.is_empty() {
        return Err(error_at(source, "level requires at least one height slice"));
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
    source: &LogicalLine,
    lines: &[LogicalLine],
    empty: Option<char>,
    char_objects: &HashMap<char, Vec<ObjectId>>,
    variable_defaults: &[i64],
) -> ParseProduct<Result<ParsedLevel, DiagnosticReport>> {
    let recognition = recognize_level_display(lines, empty, char_objects);
    let value = (|| {
        let regions = split_regions(source, lines)?;
        let layout = lower_regions(&regions);
        let mut state = State::empty_with_variables(
            layout.width as u16,
            layout.height as u16,
            game.layer_count,
            game.object_count(),
            variable_defaults.to_vec(),
        )
        .map_err(|error| error_at(source, format!("invalid level dimensions: {error:?}")))?;
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
                .map_err(|error| error_at(source, format!("invalid level dimensions: {error:?}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut placements = HashMap::<(usize, usize, LayerId), (ObjectId, LogicalLine)>::new();
        for placed_region in &layout.regions {
            for (authored_layer, layer) in placed_region.region.layers.iter().enumerate() {
                for (y, row) in layer.iter().enumerate() {
                    for (x, ch) in row.text.chars().enumerate() {
                        let Some(objects) = level_cell_objects(ch, empty, char_objects)
                            .map_err(|error| error_at(row, error.message()))?
                        else {
                            continue;
                        };
                        let mut char_layers = HashMap::<LayerId, ObjectId>::new();
                        for object in objects {
                            let object_layer = game
                                .object_layer(*object)
                                .ok_or(StateError::UnknownObject { object: *object })
                                .map_err(|error| {
                                    error_at(row, format!("invalid level cell: {error:?}"))
                                })?;
                            if let Some(existing) = char_layers.insert(object_layer, *object) {
                                return Err(error_at(
                                    row,
                                    format!(
                                        "invalid level cell: {:?}",
                                        StateError::LayerOccupied {
                                            position: puzzle_kernel::GridCoord::new([
                                                (placed_region.x + x) as u16,
                                                y as u16,
                                            ]),
                                            layer: object_layer,
                                            existing,
                                            attempted: *object,
                                        }
                                    ),
                                ));
                            }
                            placements.insert(
                                (placed_region.x + x, y, object_layer),
                                (*object, row.clone()),
                            );
                            layer_states[authored_layer]
                                .place_object(game, (placed_region.x + x) as u16, y as u16, *object)
                                .map_err(|error| {
                                    error_at(row, format!("invalid level cell: {error:?}"))
                                })?;
                        }
                    }
                }
            }
        }
        for ((x, y, _), (object, row)) in placements {
            state
                .place_object(game, x as u16, y as u16, object)
                .map_err(|error| error_at(&row, format!("invalid level cell: {error:?}")))?;
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
                    known: char_objects.contains_key(&ch) || Some(ch) == empty,
                });
        }
    }
    recognition
}

pub(crate) fn level_cell_objects<'a>(
    ch: char,
    empty: Option<char>,
    char_objects: &'a HashMap<char, Vec<ObjectId>>,
) -> Result<Option<&'a [ObjectId]>, UnknownLevelChar> {
    if let Some(objects) = char_objects.get(&ch) {
        return Ok(Some(objects));
    }
    if Some(ch) == empty {
        return Ok(None);
    }
    Err(UnknownLevelChar { ch })
}

pub(crate) struct UnknownLevelChar {
    ch: char,
}

impl UnknownLevelChar {
    pub(crate) fn message(&self) -> String {
        format!("unknown level char '{}'", self.ch)
    }
}

#[derive(Clone, Debug)]
struct LevelAsciiRegion {
    layers: Vec<Vec<LogicalLine>>,
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

fn split_regions(
    source: &LogicalLine,
    lines: &[LogicalLine],
) -> Result<Vec<LevelAsciiRegion>, DiagnosticReport> {
    let mut regions = Vec::<LevelAsciiRegion>::new();
    let mut current_layers = Vec::<Vec<LogicalLine>>::new();
    let mut current_rows = Vec::<LogicalLine>::new();
    let mut trailing_separator = None::<LogicalLine>;
    for line in lines {
        if line.trim().is_empty() {
            flush_region(
                &mut regions,
                &mut current_layers,
                &mut current_rows,
                trailing_separator.take(),
            )?;
            continue;
        }
        if line.as_ref() == "+" {
            if current_rows.is_empty() {
                return Err(error_at(
                    line,
                    "level layer separator requires a preceding ASCII layer",
                ));
            }
            current_layers.push(std::mem::take(&mut current_rows));
            trailing_separator = Some(line.clone());
            continue;
        }
        trailing_separator = None;
        current_rows.push(line.clone());
    }
    flush_region(
        &mut regions,
        &mut current_layers,
        &mut current_rows,
        trailing_separator,
    )?;
    if regions.is_empty() {
        return Err(error_at(source, "level requires at least one row"));
    }
    Ok(regions)
}

fn flush_region(
    regions: &mut Vec<LevelAsciiRegion>,
    current_layers: &mut Vec<Vec<LogicalLine>>,
    current_rows: &mut Vec<LogicalLine>,
    trailing_separator: Option<LogicalLine>,
) -> Result<(), DiagnosticReport> {
    if !current_rows.is_empty() {
        current_layers.push(std::mem::take(current_rows));
    } else if !current_layers.is_empty() {
        let separator = trailing_separator
            .as_ref()
            .expect("a separator exists when its following layer is missing");
        return Err(error_at(
            separator,
            "level layer separator requires a following ASCII layer",
        ));
    }
    if current_layers.is_empty() {
        return Ok(());
    }
    let region = validate_region_layers(std::mem::take(current_layers))?;
    regions.push(region);
    Ok(())
}

fn validate_region_layers(
    layers: Vec<Vec<LogicalLine>>,
) -> Result<LevelAsciiRegion, DiagnosticReport> {
    let mut expected_size = None::<(usize, usize)>;
    for layer in &layers {
        let Some(first_row) = layer.first() else {
            unreachable!("empty level layers are rejected before region validation");
        };
        let width = first_row.text.chars().count();
        if let Some(row) = layer
            .iter()
            .find(|row| width == 0 || row.text.chars().count() != width)
        {
            return Err(error_at(row, "level regions must be rectangular"));
        }
        if let Some(row) = layer.iter().find(|row| row.text.contains(['{', '}'])) {
            return Err(error_at(row, "ASCII rows cannot contain braces"));
        }
        let size = (width, layer.len());
        if let Some(expected) = expected_size {
            if size != expected {
                return Err(error_at(
                    first_row,
                    "level ASCII layers in the same region must have the same size",
                ));
            }
        } else {
            expected_size = Some(size);
        }
    }
    let (width, height) = expected_size.expect("validated regions contain at least one layer");
    Ok(LevelAsciiRegion {
        layers,
        width,
        height,
    })
}

pub(crate) fn error_at(line: &LogicalLine, message: impl Into<String>) -> DiagnosticReport {
    DiagnosticReport::error_at_source_line_number(message, line.text.clone(), line.line)
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
