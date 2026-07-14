use crate::{DiagnosticReport, source::LogicalLine};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelDimension {
    #[default]
    Two,
    Three,
}

impl ModelDimension {
    pub const fn number(self) -> u8 {
        match self {
            Self::Two => 2,
            Self::Three => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PuzzleModelSyntax {
    pub(crate) name: String,
    pub(crate) dimension: ModelDimension,
    pub(crate) dimension_is_explicit: bool,
    pub(crate) catalog_entries: Vec<PuzzleEntrySyntax>,
    pub(crate) entries: Vec<PuzzleEntrySyntax>,
    pub(crate) level_resources: Vec<PuzzleEntrySyntax>,
    pub(crate) sprite_resources: Vec<PuzzleEntrySyntax>,
    pub(crate) source_line: String,
    pub(crate) source_line_number: usize,
}

/// A direct child of a `puzzle { ... }` block.
///
/// This is deliberately dimension-independent. Spatial lowering may interpret
/// an entry's body differently, but it must not rescan the document to discover
/// puzzle ownership or select a different parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PuzzleEntrySyntax {
    pub(crate) header: LogicalLine,
    pub(crate) body: Vec<LogicalLine>,
    pub(crate) directive: puzzle_authoring::PuzzleDirectiveSurface,
}

pub(crate) fn parse_puzzle_model_syntax(
    lines: &[LogicalLine],
) -> Result<Vec<PuzzleModelSyntax>, DiagnosticReport> {
    parse_puzzle_models_from_document_entries(&parse_document_entries(lines)?)
}

pub(crate) fn parse_puzzle_models_from_document_entries(
    document_entries: &[PuzzleEntrySyntax],
) -> Result<Vec<PuzzleModelSyntax>, DiagnosticReport> {
    let mut models = Vec::new();
    for document_entry in document_entries {
        let line = &document_entry.header;
        let header = crate::split_header_tokens(&line.text);
        let Some(declaration) = crate::syntax::named_block_declaration_syntax(&header, "puzzle")
        else {
            continue;
        };
        if !line.text.trim_end().ends_with('{') {
            continue;
        }

        let all_entries = parse_owner_entries(&document_entry.body, "puzzle")?;
        let mut dimension = None;
        for entry in &all_entries {
            if !entry.body.is_empty() {
                continue;
            }
            let Some((name, value)) = puzzle_authoring::parse_assignment_row(&entry.header.text)
            else {
                continue;
            };
            if name != "dimension" {
                continue;
            }
            if dimension.is_some() {
                return Err(error_at("duplicate puzzle dimension", &entry.header));
            }
            dimension = Some(match value.trim() {
                "2" => ModelDimension::Two,
                "3" => ModelDimension::Three,
                _ => {
                    return Err(error_at(
                        "puzzle dimension must be `dimension = 2` or `dimension = 3`",
                        &entry.header,
                    ));
                }
            });
        }
        let (catalog_entries, entries) = all_entries
            .into_iter()
            .partition(|entry| entry.directive.is_catalog_owned());

        models.push(PuzzleModelSyntax {
            name: declaration.name.to_string(),
            dimension: dimension.unwrap_or_default(),
            dimension_is_explicit: dimension.is_some(),
            catalog_entries,
            entries,
            level_resources: Vec::new(),
            sprite_resources: Vec::new(),
            source_line: line.text.clone(),
            source_line_number: line.line,
        });
    }
    associate_model_resources(document_entries, &mut models)?;
    Ok(models)
}

fn associate_model_resources(
    document_entries: &[PuzzleEntrySyntax],
    models: &mut [PuzzleModelSyntax],
) -> Result<(), DiagnosticReport> {
    for entry in document_entries {
        let keyword = match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Levels => "levels",
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => "sprites",
            _ => continue,
        };
        let header = puzzle_authoring::resource_header_surface(&entry.header.text, keyword)
            .map_err(|error| error_at(error.message(), &entry.header))?;
        let model_index = match header.owner {
            Some(owner) => models
                .iter()
                .position(|model| model.name == owner)
                .ok_or_else(|| {
                    error_at(
                        format!("{keyword} resource refers to unknown puzzle `{owner}`"),
                        &entry.header,
                    )
                })?,
            None if models.len() == 1 => 0,
            None => {
                return Err(error_at(
                    format!(
                        "bare {keyword} resource is ambiguous with multiple puzzles; add `of <puzzle>`"
                    ),
                    &entry.header,
                ));
            }
        };
        match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                models[model_index].level_resources.push(entry.clone())
            }
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => {
                models[model_index].sprite_resources.push(entry.clone())
            }
            _ => unreachable!("resource entries were selected above"),
        }
    }
    Ok(())
}

pub(crate) fn parse_child_entries(
    entry: &PuzzleEntrySyntax,
) -> Result<Vec<PuzzleEntrySyntax>, DiagnosticReport> {
    parse_owner_entries(&entry.body, &entry.header.text)
}

pub(crate) fn parse_child_entry_at(
    entry: &PuzzleEntrySyntax,
    index: usize,
) -> Result<(PuzzleEntrySyntax, usize), DiagnosticReport> {
    parse_owner_entry_at(&entry.body, index, &entry.header.text)
}

pub(crate) fn parse_document_entries(
    lines: &[LogicalLine],
) -> Result<Vec<PuzzleEntrySyntax>, DiagnosticReport> {
    parse_owner_entries(lines, "document")
}

fn parse_owner_entries(
    lines: &[LogicalLine],
    owner: &str,
) -> Result<Vec<PuzzleEntrySyntax>, DiagnosticReport> {
    let mut entries = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let (entry, next) = parse_owner_entry_at(lines, index, owner)?;
        entries.push(entry);
        index = next;
    }
    Ok(entries)
}

fn parse_owner_entry_at(
    lines: &[LogicalLine],
    start: usize,
    owner: &str,
) -> Result<(PuzzleEntrySyntax, usize), DiagnosticReport> {
    let header = lines[start].clone();
    let mut depth = crate::authoring_line_brace_delta(&header.text);
    if depth < 0 {
        return Err(error_at(
            format!("{owner} entry has an unmatched }}"),
            &header,
        ));
    }
    if depth == 0 {
        let directive = puzzle_authoring::puzzle_directive_surface(&header.text);
        return Ok((
            PuzzleEntrySyntax {
                header,
                body: Vec::new(),
                directive,
            },
            start + 1,
        ));
    }

    let body_start = start + 1;
    let mut index = body_start;
    while index < lines.len() && depth > 0 {
        depth += crate::authoring_line_brace_delta(&lines[index].text);
        if depth < 0 {
            return Err(error_at(
                format!("{owner} entry has an unmatched }}"),
                &lines[index],
            ));
        }
        index += 1;
    }
    if depth != 0 {
        return Err(error_at(
            format!("{owner} entry missing closing brace"),
            &header,
        ));
    }
    let directive = puzzle_authoring::puzzle_directive_surface(&header.text);
    Ok((
        PuzzleEntrySyntax {
            header,
            body: lines[body_start..index - 1].to_vec(),
            directive,
        },
        index,
    ))
}

fn error_at(message: impl Into<String>, line: &LogicalLine) -> DiagnosticReport {
    DiagnosticReport::error_at_source_line_number(message, line.text.clone(), line.line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Result<Vec<PuzzleModelSyntax>, DiagnosticReport> {
        let lines = crate::source::logical_lines_with_locations(source)?;
        parse_puzzle_model_syntax(&lines)
    }

    #[test]
    fn puzzle_dimension_is_owner_scoped_and_order_independent() {
        let models = parse(
            r#"
puzzle flat {
rules {
}
}

puzzle space {
rules {
}
dimension = 3
}
"#,
        )
        .unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "flat");
        assert_eq!(models[0].dimension, ModelDimension::Two);
        assert!(!models[0].dimension_is_explicit);
        assert_eq!(models[1].name, "space");
        assert_eq!(models[1].dimension, ModelDimension::Three);
        assert!(models[1].dimension_is_explicit);
        assert_eq!(models[1].entries.len(), 2);
        assert_eq!(models[1].entries[0].header.text, "rules {");
        assert!(models[1].entries[0].body.is_empty());
        assert_eq!(models[1].entries[1].header.text, "dimension = 3");
    }

    #[test]
    fn nested_dimension_does_not_set_the_puzzle_dimension() {
        let models = parse(
            r#"
puzzle flat {
render {
dimension = 3
}
rules {
}
}
"#,
        )
        .unwrap();

        assert_eq!(models[0].dimension, ModelDimension::Two);
        assert!(!models[0].dimension_is_explicit);
    }

    #[test]
    fn puzzle_dimension_rejects_duplicates_and_unknown_values() {
        for (source, expected) in [
            (
                "puzzle p {\ndimension = 3\ndimension = 3\n}",
                "duplicate puzzle dimension",
            ),
            ("puzzle p {\ndimension = 4\n}", "puzzle dimension must be"),
        ] {
            let error = parse(source).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn model_owner_contains_nested_2d_blocks() {
        let models = parse(
            r#"
title = cell_size_render

puzzle default {
slots 1
empty .

render {
cell_size = 64
}

rules {
}

level "start" {
.
}
}
"#,
        )
        .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "default");
        assert_eq!(models[0].dimension, ModelDimension::Two);
    }

    #[test]
    fn model_product_owns_catalog_declarations_and_external_resources() {
        let models = parse(
            r#"
puzzle space {
dimension = 3
slots 1
map turn axis {
}
rules {
}
}

levels demo of space {
}
sprites art of space {
}
"#,
        )
        .unwrap();

        let [model] = models.as_slice() else {
            panic!("expected one model")
        };
        assert_eq!(model.catalog_entries.len(), 2);
        assert!(
            model
                .entries
                .iter()
                .all(|entry| !entry.directive.is_catalog_owned())
        );
        assert_eq!(model.level_resources.len(), 1);
        assert_eq!(model.sprite_resources.len(), 1);
    }
}
