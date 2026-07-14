use crate::{
    DiagnosticReport,
    source::LogicalLine,
    surface::{ParseProduct, ParserRecognition, SourceSpan, SurfaceSemanticKind},
};

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
    pub(crate) sprite_resources: Vec<PuzzleEntrySyntax>,
    pub(crate) body: PuzzleBodySyntax,
    pub(crate) source_line: String,
    pub(crate) source_line_number: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PuzzleBodySyntax {
    pub(crate) keys: Vec<ModelKeyBindingSyntax>,
    pub(crate) routines: Vec<RuleRoutineSyntax>,
    pub(crate) rules: Option<RuleStatementsSyntax>,
    pub(crate) on_level_start: Option<RuleStatementsSyntax>,
    pub(crate) on_level_clear: Option<RuleStatementsSyntax>,
    pub(crate) on_last_level_clear: Option<RuleStatementsSyntax>,
    pub(crate) win_conditions: Option<WinConditionsSyntax>,
    pub(crate) queries: Vec<QueryDefinitionSyntax>,
    pub(crate) solver: Option<crate::solver_surface::SolverSurfaceStrategy>,
    pub(crate) render: Option<PuzzleRenderProduct>,
    pub(crate) levels: crate::level::LevelResourceSyntax,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PuzzleRenderProduct {
    pub(crate) render: crate::PuzzleRenderDef,
    pub(crate) animation: crate::AnimationDef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelKeyBindingSyntax {
    pub(crate) keys: Vec<String>,
    pub(crate) target: String,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleStatementsSyntax {
    pub(crate) header: LogicalLine,
    pub(crate) modifier: Option<String>,
    pub(crate) statements: Vec<puzzle_authoring::RuleStatementSyntax<LogicalLine>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleRoutineSyntax {
    pub(crate) header: LogicalLine,
    pub(crate) name: String,
    pub(crate) application: puzzle_authoring::RuleApplicationSurface,
    pub(crate) statements: Vec<puzzle_authoring::RuleStatementSyntax<LogicalLine>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QueryDefinitionSyntax {
    pub(crate) source: LogicalLine,
    pub(crate) definition: crate::solver_surface::SolverSurfaceQueryDefinition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WinConditionsSyntax {
    pub(crate) header: LogicalLine,
    pub(crate) rows: Vec<LogicalLine>,
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
) -> ParseProduct<Result<Vec<PuzzleModelSyntax>, DiagnosticReport>> {
    let entries = match parse_document_entries(lines) {
        Ok(entries) => entries,
        Err(error) => return ParseProduct::new(Err(error), ParserRecognition::default()),
    };
    parse_puzzle_models_from_document_entries(&entries)
}

pub(crate) fn parse_puzzle_models_from_document_entries(
    document_entries: &[PuzzleEntrySyntax],
) -> ParseProduct<Result<Vec<PuzzleModelSyntax>, DiagnosticReport>> {
    let mut recognition = ParserRecognition::default();
    let value = (|| {
        let mut models = Vec::new();
        for document_entry in document_entries {
            if document_entry.directive == puzzle_authoring::PuzzleDirectiveSurface::RemovedLevels3
            {
                return Err(error_at(
                    "`levels3` was removed; use `levels`",
                    &document_entry.header,
                ));
            }
            let line = &document_entry.header;
            let header = crate::split_header_tokens(&line.text);
            let Some(declaration) =
                crate::syntax::named_block_declaration_syntax(&header, "puzzle")
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
                let Some((name, value)) =
                    puzzle_authoring::parse_assignment_row(&entry.header.text)
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
            let (catalog_entries, entries, body) =
                parse_model_body_syntax(all_entries, &mut recognition, declaration.name)?;

            models.push(PuzzleModelSyntax {
                name: declaration.name.to_string(),
                dimension: dimension.unwrap_or_default(),
                dimension_is_explicit: dimension.is_some(),
                catalog_entries,
                entries,
                sprite_resources: Vec::new(),
                body,
                source_line: line.text.clone(),
                source_line_number: line.line,
            });
        }
        associate_model_resources(document_entries, &mut models)?;
        Ok(models)
    })();
    ParseProduct::new(value, recognition)
}

fn parse_model_body_syntax(
    entries: Vec<PuzzleEntrySyntax>,
    recognition: &mut ParserRecognition,
    puzzle_name: &str,
) -> Result<
    (
        Vec<PuzzleEntrySyntax>,
        Vec<PuzzleEntrySyntax>,
        PuzzleBodySyntax,
    ),
    DiagnosticReport,
> {
    let mut catalog_entries = Vec::new();
    let mut residual_entries = Vec::new();
    let mut body = PuzzleBodySyntax::default();
    let mut query_names = std::collections::HashSet::new();
    let mut routine_names = std::collections::HashSet::new();
    for entry in entries {
        if entry.directive.is_catalog_owned() {
            catalog_entries.push(entry);
            continue;
        }
        match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Legend
            | puzzle_authoring::PuzzleDirectiveSurface::Level
            | puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                let resource = crate::parse_level_resource_entry(
                    &entry,
                    body.levels.levels.len(),
                    Some(puzzle_name),
                )?;
                body.levels.legends.extend(resource.legends);
                body.levels.levels.extend(resource.levels);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Keys if entry.header.text == "keys {" => {
                for row in &entry.body {
                    if row.text.ends_with('{') {
                        return Err(error_at("keys accepts rows, not nested blocks", row));
                    }
                    let binding = puzzle_authoring::key_binding_surface(&row.text)
                        .map_err(|error| error_at(error.message(), row))?;
                    let target_tokens = crate::split_header_tokens(binding.target);
                    let [target] = target_tokens.as_slice() else {
                        return Err(error_at("keys row must name one input target", row));
                    };
                    body.keys.push(ModelKeyBindingSyntax {
                        keys: binding.keys.into_iter().map(str::to_string).collect(),
                        target: (*target).to_string(),
                        source: row.clone(),
                    });
                }
            }
            puzzle_authoring::PuzzleDirectiveSurface::RuleProgram => {
                let block = puzzle_authoring::rule_program_block_surface(&entry.header.text)
                    .ok_or_else(|| error_at("invalid rule program header", &entry.header))?;
                mark_rule_program_header(recognition, &entry.header);
                let parsed = puzzle_authoring::collect_rule_program_entry_body(&entry.body, block)
                    .map_err(|error| {
                        let source = error
                            .line_index()
                            .and_then(|index| entry.body.get(index))
                            .unwrap_or(&entry.header);
                        error_at(error.message(), source)
                    })?;
                match (block, parsed) {
                    (
                        puzzle_authoring::RuleProgramBlockSurface::Rules { modifier },
                        puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements),
                    ) => set_rule_statements(
                        &mut body.rules,
                        modifier,
                        statements,
                        "multiple puzzle rules blocks are not supported",
                        &entry.header,
                    )?,
                    (
                        puzzle_authoring::RuleProgramBlockSurface::OnLevelStart { modifier },
                        puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements),
                    ) => set_rule_statements(
                        &mut body.on_level_start,
                        modifier,
                        statements,
                        "multiple level_start blocks are not supported",
                        &entry.header,
                    )?,
                    (
                        puzzle_authoring::RuleProgramBlockSurface::OnLevelClear,
                        puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements),
                    ) => set_rule_statements(
                        &mut body.on_level_clear,
                        "",
                        statements,
                        "multiple level_clear blocks are not supported",
                        &entry.header,
                    )?,
                    (
                        puzzle_authoring::RuleProgramBlockSurface::OnLastLevelClear,
                        puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements),
                    ) => set_rule_statements(
                        &mut body.on_last_level_clear,
                        "",
                        statements,
                        "multiple last_level_clear blocks are not supported",
                        &entry.header,
                    )?,
                }
            }
            puzzle_authoring::PuzzleDirectiveSurface::WinConditions
                if entry.header.text.trim_end().ends_with('{') =>
            {
                set_once(
                    &mut body.win_conditions,
                    WinConditionsSyntax {
                        header: entry.header.clone(),
                        rows: entry.body,
                    },
                    "duplicate win_conditions block",
                    &entry.header,
                )?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Query => {
                let query = crate::solver_surface::parse_query_definition(&entry.header.text)
                    .map_err(|error| error_at(error.to_string(), &entry.header))?;
                if !query_names.insert(query.name.clone()) {
                    return Err(error_at("duplicate query", &entry.header));
                }
                body.queries.push(QueryDefinitionSyntax {
                    source: entry.header,
                    definition: query,
                });
            }
            puzzle_authoring::PuzzleDirectiveSurface::Solver => {
                let lines = entry
                    .body
                    .iter()
                    .map(|line| line.text.clone())
                    .collect::<Vec<_>>();
                let solver = crate::solver_surface::parse_solver_entry_body(&lines)
                    .map_err(|error| error_at(error.to_string(), &entry.header))?;
                set_once(
                    &mut body.solver,
                    solver,
                    "duplicate solver block",
                    &entry.header,
                )?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Render => {
                let mut lines = Vec::with_capacity(entry.body.len() + 2);
                lines.push(entry.header.clone());
                lines.extend(entry.body.iter().cloned());
                lines.push(LogicalLine::new("}", entry.header.line));
                let (render, next) = crate::authoring_grammar::parse_placed_authoring_node(
                    &lines,
                    0,
                    crate::authoring_grammar::AuthoringKind::Root,
                    "render block missing closing brace",
                )?;
                if next != lines.len() {
                    return Err(error_at(
                        "render block was not fully consumed",
                        &entry.header,
                    ));
                }
                let (render, animation) = crate::lower_puzzle_render_node(&render)?;
                set_once(
                    &mut body.render,
                    PuzzleRenderProduct { render, animation },
                    "duplicate render block",
                    &entry.header,
                )?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Unknown
                if matches!(
                    puzzle_authoring::rule_statement_block_surface(&entry.header.text, false),
                    Some(puzzle_authoring::RuleStatementBlockSurface::Routine)
                ) =>
            {
                let parsed = puzzle_authoring::collect_rule_program_entry_body(
                    &entry.body,
                    puzzle_authoring::RuleProgramBlockSurface::Rules { modifier: "" },
                )
                .map_err(|error| error_at(error.message(), &entry.header))?;
                let puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements) = parsed;
                let tokens = crate::split_header_tokens(&entry.header.text);
                let (name, application) = match tokens.as_slice() {
                    ["routine", name] => (
                        (*name).to_string(),
                        puzzle_authoring::RuleApplicationSurface::Once,
                    ),
                    ["routine", name, application] => (
                        (*name).to_string(),
                        puzzle_authoring::rule_application_surface(application).ok_or_else(
                            || {
                                error_at(
                                    "routine application must be once, once_all, once_per_level, random, or repeat",
                                    &entry.header,
                                )
                            },
                        )?,
                    ),
                    _ => {
                        return Err(error_at(
                            "routine header must be: routine <name> [once | once_all | once_per_level | random | repeat]",
                            &entry.header,
                        ));
                    }
                };
                if !routine_names.insert(name.clone()) {
                    return Err(error_at("duplicate routine definition", &entry.header));
                }
                body.routines.push(RuleRoutineSyntax {
                    header: entry.header,
                    name,
                    application,
                    statements,
                });
            }
            _ => residual_entries.push(entry),
        }
    }
    Ok((catalog_entries, residual_entries, body))
}

fn mark_rule_program_header(recognition: &mut ParserRecognition, line: &LogicalLine) {
    for token in line
        .tokens
        .iter()
        .filter(|token| token.text != "{" && token.text != "}")
    {
        recognition.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Keyword,
        );
    }
}

fn set_rule_statements(
    target: &mut Option<RuleStatementsSyntax>,
    modifier: &str,
    statements: Vec<puzzle_authoring::RuleStatementSyntax<LogicalLine>>,
    duplicate_message: &str,
    line: &LogicalLine,
) -> Result<(), DiagnosticReport> {
    set_once(
        target,
        RuleStatementsSyntax {
            header: line.clone(),
            modifier: (!modifier.trim().is_empty()).then(|| modifier.trim().to_string()),
            statements,
        },
        duplicate_message,
        line,
    )
}

fn set_once<T>(
    target: &mut Option<T>,
    value: T,
    duplicate_message: &str,
    line: &LogicalLine,
) -> Result<(), DiagnosticReport> {
    if target.is_some() {
        return Err(error_at(duplicate_message, line));
    }
    *target = Some(value);
    Ok(())
}

fn associate_model_resources(
    document_entries: &[PuzzleEntrySyntax],
    models: &mut [PuzzleModelSyntax],
) -> Result<(), DiagnosticReport> {
    for entry in document_entries {
        let keyword = match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Legend => "legend",
            puzzle_authoring::PuzzleDirectiveSurface::Level => "level",
            puzzle_authoring::PuzzleDirectiveSurface::Levels => "levels",
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => "sprites",
            _ => continue,
        };
        let owner = if matches!(
            entry.directive,
            puzzle_authoring::PuzzleDirectiveSurface::Levels
                | puzzle_authoring::PuzzleDirectiveSurface::Sprites
        ) {
            puzzle_authoring::resource_header_surface(&entry.header.text, keyword)
                .map_err(|error| error_at(error.message(), &entry.header))?
                .owner
        } else {
            None
        };
        let model_index = match owner {
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
            puzzle_authoring::PuzzleDirectiveSurface::Legend
            | puzzle_authoring::PuzzleDirectiveSurface::Level
            | puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                let resource = crate::parse_level_resource_entry(
                    entry,
                    models[model_index].body.levels.levels.len(),
                    Some(&models[model_index].name),
                )?;
                models[model_index]
                    .body
                    .levels
                    .legends
                    .extend(resource.legends);
                models[model_index]
                    .body
                    .levels
                    .levels
                    .extend(resource.levels);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => {
                models[model_index].sprite_resources.push(entry.clone())
            }
            _ => unreachable!("resource entries were selected above"),
        }
    }
    Ok(())
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
        parse_puzzle_model_syntax(&lines).value
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
        assert!(models[1].body.rules.is_some());
        assert_eq!(models[1].entries.len(), 1);
        assert_eq!(models[1].entries[0].header.text, "dimension = 3");
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
title = render_model

puzzle default {
slots 1
empty .

render {
tween = true
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
level "empty" {
.
}
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
        assert_eq!(model.body.levels.levels.len(), 1);
        assert_eq!(model.sprite_resources.len(), 1);
    }
}
