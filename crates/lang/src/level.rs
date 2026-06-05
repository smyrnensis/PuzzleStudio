use std::collections::HashMap;

use puzzle_core::{CompiledGame, ObjectId, State};

use crate::{AppError, LevelRegionDef};

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
    global_defaults: &[i64],
) -> Result<ParsedLevel, AppError> {
    let regions = split_regions(lines)?;
    let (lowered_lines, region_defs) = lower_regions(&regions, empty);
    let height = lowered_lines.len() as u16;
    let width = lowered_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let mut state = State::empty_with_globals(
        width,
        height,
        game.layer_count,
        game.object_count(),
        global_defaults.to_vec(),
    )?;

    for (y, line) in lowered_lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            if ch == empty {
                continue;
            }
            let objects = char_objects
                .get(&ch)
                .ok_or_else(|| AppError::Parse(format!("unknown level char '{ch}'")))?;
            for object in objects {
                state.place_object(game, x as u16, y as u16, *object)?;
            }
        }
    }

    Ok(ParsedLevel {
        state,
        regions: region_defs,
    })
}

fn split_regions(lines: &[String]) -> Result<Vec<Vec<String>>, AppError> {
    let mut regions = Vec::<Vec<String>>::new();
    let mut current = Vec::<String>::new();
    for line in lines {
        if line.trim().is_empty() {
            if !current.is_empty() {
                regions.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(line.clone());
    }
    if !current.is_empty() {
        regions.push(current);
    }
    if regions.is_empty() {
        return Err(AppError::Parse(
            "level requires at least one row".to_string(),
        ));
    }

    for region in &regions {
        let width = region[0].chars().count();
        if width == 0 || region.iter().any(|row| row.chars().count() != width) {
            return Err(AppError::Parse(
                "level regions must be rectangular".to_string(),
            ));
        }
        if region.iter().any(|row| row.contains(['{', '}'])) {
            return Err(AppError::Parse(
                "ASCII rows cannot contain braces".to_string(),
            ));
        }
    }
    Ok(regions)
}

fn lower_regions(regions: &[Vec<String>], empty: char) -> (Vec<String>, Vec<LevelRegionDef>) {
    let gap = 2usize;
    let height = regions.iter().map(Vec::len).max().unwrap_or(0);
    let total_width = regions
        .iter()
        .enumerate()
        .map(|(index, region)| {
            let gap_width = if index == 0 { 0 } else { gap };
            gap_width + region[0].chars().count()
        })
        .sum::<usize>();

    let mut lowered = vec![vec![empty; total_width]; height];
    let mut defs = Vec::new();
    let mut x_offset = 0usize;
    for (index, region) in regions.iter().enumerate() {
        if index > 0 {
            x_offset += gap;
        }
        let width = region[0].chars().count();
        for (y, row) in region.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                lowered[y][x_offset + x] = ch;
            }
        }
        defs.push(LevelRegionDef {
            index,
            x: x_offset as u16,
            y: 0,
            width: width as u16,
            height: region.len() as u16,
        });
        x_offset += width;
    }

    let rows = lowered
        .into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>();
    (rows, defs)
}
