use crate::{
    DiagnosticReport,
    source::LogicalLine,
    surface::{ParserRecognition, SourceSpan, SurfaceSemanticKind},
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

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Two => "2d",
            Self::Three => "3d",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PuzzleModelSyntax {
    pub(crate) name: String,
    pub(crate) dimension: ModelDimension,
    pub(crate) dimension_is_explicit: bool,
    pub(crate) catalog_entries: Vec<PuzzleEntrySyntax>,
    pub(crate) body: PuzzleBodySyntax,
    pub(crate) diagnostics: Vec<DiagnosticReport>,
    pub(crate) source_line: String,
    pub(crate) source_line_number: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PuzzleBodySyntax {
    pub(crate) semantics: SyntaxSemantics,
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
    pub(crate) variables: Vec<VariableDeclarationSyntax>,
    pub(crate) named_conditions: Vec<NamedConditionSyntax>,
    pub(crate) inputs: Vec<InputDeclarationSyntax>,
    pub(crate) direction_aliases: Vec<DirectionAliasSyntax>,
    pub(crate) render_overlays: Vec<RenderOverlaySyntax>,
    pub(crate) lose_conditions: Option<WinConditionsSyntax>,
    pub(crate) sounds: ModelSoundsSyntax,
    pub(crate) screen: crate::PuzzleScreenDef,
    pub(crate) run_rules_on_level_start: bool,
    pub(crate) empty_char: Option<char>,
    pub(crate) sprite_resources: Vec<PuzzleEntrySyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VariableDeclarationSyntax {
    pub(crate) name: String,
    pub(crate) default: i64,
    pub(crate) numeric: bool,
    pub(crate) persistent: bool,
    pub(crate) constant: bool,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NamedConditionSyntax {
    pub(crate) name: String,
    pub(crate) expression: String,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputDeclarationSyntax {
    pub(crate) name: String,
    pub(crate) direction: Option<String>,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectionAliasSyntax {
    pub(crate) alias: String,
    pub(crate) canonical: String,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RenderOverlaySyntax {
    pub(crate) selectors: Vec<String>,
    pub(crate) character: char,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModelSoundsSyntax {
    pub(crate) triggers: Vec<ModelSoundTriggerSyntax>,
    pub(crate) operations: Vec<ModelOperationSoundSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelSoundTriggerSyntax {
    pub(crate) selector: String,
    pub(crate) sfx_name: String,
    pub(crate) source: LogicalLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelOperationSoundSyntax {
    pub(crate) operation: crate::ModelOperationSound,
    pub(crate) sfx_name: String,
    pub(crate) source: LogicalLine,
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
    pub(crate) semantics: SyntaxSemantics,
}

impl RuleStatementsSyntax {
    pub(crate) fn new(
        header: LogicalLine,
        modifier: Option<String>,
        statements: Vec<puzzle_authoring::RuleStatementSyntax<LogicalLine>>,
    ) -> Self {
        let semantics = rule_program_semantics(&header, &statements, false);
        Self {
            header,
            modifier,
            statements,
            semantics,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleRoutineSyntax {
    pub(crate) statement: puzzle_authoring::RuleStatementSyntax<LogicalLine>,
    pub(crate) semantics: SyntaxSemantics,
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
    pub(crate) semantics: SyntaxSemantics,
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
    pub(crate) closing: Option<LogicalLine>,
    pub(crate) directive: puzzle_authoring::PuzzleDirectiveSurface,
    pub(crate) semantics: SyntaxSemantics,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SyntaxSemantics {
    pub(crate) fixed: ParserRecognition,
    pub(crate) selectors: Vec<SelectorOccurrenceSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectorOccurrenceSyntax {
    pub(crate) span: SourceSpan,
    pub(crate) text: String,
}

#[cfg(test)]
fn parse_puzzle_model_syntax(
    lines: &[LogicalLine],
) -> Result<Vec<PuzzleModelSyntax>, DiagnosticReport> {
    let entries = parse_document_entries(lines)?;
    validate_closed_entries(&entries, "document")?;
    let models = parse_puzzle_models_from_document_entries(&entries)?;
    validate_puzzle_model_diagnostics(&models)?;
    Ok(models)
}

pub(crate) fn validate_closed_entries(
    entries: &[PuzzleEntrySyntax],
    owner: &str,
) -> Result<(), DiagnosticReport> {
    let Some(entry) = entries
        .iter()
        .find(|entry| entry.header.structural_brace_delta() > 0 && entry.closing.is_none())
    else {
        return Ok(());
    };
    Err(error_at(
        format!("{owner} entry missing closing brace"),
        &entry.header,
    ))
}

pub(crate) fn validate_puzzle_model_diagnostics(
    models: &[PuzzleModelSyntax],
) -> Result<(), DiagnosticReport> {
    let diagnostics = models
        .iter()
        .flat_map(|model| &model.diagnostics)
        .flat_map(|report| report.diagnostics().iter().cloned())
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(DiagnosticReport::from_diagnostics(diagnostics))
    }
}

pub(crate) fn parse_puzzle_models_from_document_entries(
    document_entries: &[PuzzleEntrySyntax],
) -> Result<Vec<PuzzleModelSyntax>, DiagnosticReport> {
    (|| {
        let mut models = Vec::new();
        for document_entry in document_entries {
            validate_document_entry(document_entry)?;
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
            let mut model_diagnostics = Vec::new();
            if let Err(report) = validate_closed_entries(&all_entries, "puzzle") {
                model_diagnostics.push(report);
            }
            let fallback_entries = all_entries.clone();
            let (catalog_entries, body, dimension) =
                match parse_model_body_syntax(all_entries, declaration.name) {
                    Ok(product) => product,
                    Err(report) => {
                        model_diagnostics.push(report);
                        let catalog_entries = fallback_entries
                            .into_iter()
                            .filter(|entry| entry.directive.is_catalog_owned())
                            .collect();
                        (catalog_entries, PuzzleBodySyntax::default(), None)
                    }
                };

            models.push(PuzzleModelSyntax {
                name: declaration.name.to_string(),
                dimension: dimension.unwrap_or_default(),
                dimension_is_explicit: dimension.is_some(),
                catalog_entries,
                body,
                diagnostics: model_diagnostics,
                source_line: line.text.clone(),
                source_line_number: line.line,
            });
        }
        associate_model_resources(document_entries, &mut models)?;
        Ok(models)
    })()
}

fn validate_document_entry(entry: &PuzzleEntrySyntax) -> Result<(), DiagnosticReport> {
    use puzzle_authoring::PuzzleDirectiveSurface as Directive;

    match entry.directive {
        Directive::Empty
        | Directive::Metadata
        | Directive::DocumentShell
        | Directive::InputBuffer
        | Directive::Variable
        | Directive::Legend
        | Directive::Levels
        | Directive::Level
        | Directive::Sprites
        | Directive::Scene => Ok(()),
        Directive::Model => {
            let tokens = crate::split_header_tokens(&entry.header.text);
            if crate::syntax::named_block_declaration_syntax(&tokens, "puzzle").is_none()
                || !entry.header.text.trim_end().ends_with('{')
            {
                return Err(error_at(
                    "top-level puzzle definition must be: puzzle <name>",
                    &entry.header,
                ));
            }
            Ok(())
        }
        Directive::RemovedModelPrefix => Err(error_at(
            "top-level puzzle definition must be: puzzle <name>",
            &entry.header,
        )),
        Directive::RemovedNameMetadata => Err(error_at(
            "top-level `name` metadata was removed; use `title = <text>`",
            &entry.header,
        )),
        Directive::RemovedAnimation => Err(error_at(
            "top-level animation block was removed; put tween_duration under puzzle render",
            &entry.header,
        )),
        Directive::RemovedLevels3 => Err(error_at(
            "`levels3` was removed; use `levels`",
            &entry.header,
        )),
        Directive::RuleProgram => {
            let lifecycle = crate::split_header_tokens(&entry.header.text)
                .first()
                .copied()
                .unwrap_or("");
            if matches!(
                lifecycle,
                "on_level_start" | "on_level_clear" | "on_last_level_clear"
            ) {
                Err(error_at(
                    format!(
                        "{lifecycle} is a puzzle lifecycle block; put it inside `puzzle <name> {{ ... }}` next to `rules {{ ... }}`"
                    ),
                    &entry.header,
                ))
            } else {
                Err(unknown_top_level_entry(entry))
            }
        }
        _ => Err(unknown_top_level_entry(entry)),
    }
}

fn unknown_top_level_entry(entry: &PuzzleEntrySyntax) -> DiagnosticReport {
    let directive = crate::split_header_tokens(&entry.header.text)
        .first()
        .copied()
        .unwrap_or("");
    error_at(
        format!("unknown top-level directive `{directive}`"),
        &entry.header,
    )
}

fn parse_model_body_syntax(
    entries: Vec<PuzzleEntrySyntax>,
    puzzle_name: &str,
) -> Result<
    (
        Vec<PuzzleEntrySyntax>,
        PuzzleBodySyntax,
        Option<ModelDimension>,
    ),
    DiagnosticReport,
> {
    let mut catalog_entries = Vec::new();
    let mut body = PuzzleBodySyntax::default();
    let mut dimension = None;
    let mut query_names = std::collections::HashSet::new();
    let mut routine_names = std::collections::HashSet::new();
    let mut variable_names = std::collections::HashSet::new();
    let mut named_condition_names = std::collections::HashSet::new();
    for entry in entries {
        if entry.directive.is_catalog_owned() {
            catalog_entries.push(entry);
            continue;
        }
        match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Dimension => {
                if dimension.is_some() {
                    return Err(error_at("duplicate puzzle dimension", &entry.header));
                }
                let Some((name, value)) =
                    puzzle_authoring::parse_assignment_row(&entry.header.text)
                else {
                    return Err(error_at(
                        "puzzle dimension must be `dimension = 2` or `dimension = 3`",
                        &entry.header,
                    ));
                };
                if name != "dimension" || !entry.body.is_empty() {
                    return Err(error_at(
                        "puzzle dimension must be `dimension = 2` or `dimension = 3`",
                        &entry.header,
                    ));
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
            puzzle_authoring::PuzzleDirectiveSurface::Variable => {
                let declaration = parse_variable_declaration(&entry)?;
                if !variable_names.insert(declaration.name.clone()) {
                    return Err(error_at("duplicate var or const", &entry.header));
                }
                body.variables.push(declaration);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Assignment => {
                let condition = parse_named_condition(&entry)?;
                if !named_condition_names.insert(condition.name.clone()) {
                    return Err(error_at("duplicate condition", &entry.header));
                }
                body.named_conditions.push(condition);
            }
            puzzle_authoring::PuzzleDirectiveSurface::RunRulesOnLevelStart => {
                if crate::split_header_tokens(&entry.header.text).as_slice()
                    != ["run_rules_on_level_start"]
                {
                    return Err(error_at(
                        "run_rules_on_level_start takes no values",
                        &entry.header,
                    ));
                }
                body.run_rules_on_level_start = true;
            }
            puzzle_authoring::PuzzleDirectiveSurface::EmptyCell => {
                let tokens = crate::split_header_tokens(&entry.header.text);
                let ["empty", value] = tokens.as_slice() else {
                    return Err(error_at("missing empty char", &entry.header));
                };
                let mut chars = value.chars();
                let Some(character) = chars.next() else {
                    return Err(error_at("missing empty char", &entry.header));
                };
                if chars.next().is_some() {
                    return Err(error_at("expected single character", &entry.header));
                }
                if character != crate::syntax::DEFAULT_LEVEL_EMPTY_CHAR {
                    return Err(error_at("levels use `.` for empty", &entry.header));
                }
                body.empty_char = Some(character);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Input => {
                body.inputs.push(parse_input_declaration(&entry)?);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Direction => {
                body.direction_aliases.push(parse_direction_alias(&entry)?);
            }
            puzzle_authoring::PuzzleDirectiveSurface::RenderOverlay => {
                body.render_overlays
                    .push(parse_render_overlay_syntax(&entry)?);
            }
            puzzle_authoring::PuzzleDirectiveSurface::LoseConditions => {
                if !entry.header.text.trim_end().ends_with('{') {
                    return Err(error_at("lose_conditions must be a block", &entry.header));
                }
                set_once(
                    &mut body.lose_conditions,
                    WinConditionsSyntax {
                        semantics: win_condition_semantics(&entry.header, &entry.body),
                        header: entry.header.clone(),
                        rows: entry.body,
                    },
                    "duplicate lose_conditions block",
                    &entry.header,
                )?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::DocumentShell
                if entry.header.text.starts_with("sounds") =>
            {
                parse_model_sounds_syntax(&entry, &mut body.sounds)?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::PuzzleScreen => {
                if !entry.header.text.trim_end().ends_with('{') {
                    return Err(error_at("puzzle screen must be a block", &entry.header));
                }
                for line in &entry.body {
                    crate::parse_puzzle_screen_directive(line, &mut body.screen)?;
                }
                crate::validate_puzzle_screen(&body.screen, &entry.header)?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::PuzzleScreenDirective => {
                crate::parse_puzzle_screen_directive(&entry.header, &mut body.screen)?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => {
                body.sprite_resources.push(entry);
            }
            puzzle_authoring::PuzzleDirectiveSurface::Legend
            | puzzle_authoring::PuzzleDirectiveSurface::Level
            | puzzle_authoring::PuzzleDirectiveSurface::Levels => {
                let resource = crate::parse_level_resource_entry(
                    &entry,
                    body.levels.levels.len(),
                    Some(puzzle_name),
                )?;
                body.semantics
                    .fixed
                    .merge(level_resource_header_semantics(&entry.header));
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
                        semantics: win_condition_semantics(&entry.header, &entry.body),
                        header: entry.header.clone(),
                        rows: entry.body,
                    },
                    "duplicate win_conditions block",
                    &entry.header,
                )?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::WinConditions => {
                let condition = parse_named_condition(&entry)?;
                if !named_condition_names.insert(condition.name.clone()) {
                    return Err(error_at("duplicate condition", &entry.header));
                }
                body.named_conditions.push(condition);
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
            puzzle_authoring::PuzzleDirectiveSurface::Routine => {
                let parsed = puzzle_authoring::collect_rule_program_entry_body(
                    &entry.body,
                    puzzle_authoring::RuleProgramBlockSurface::Rules { modifier: "" },
                )
                .map_err(|error| error_at(error.message(), &entry.header))?;
                let puzzle_authoring::RuleProgramBlockBody::RuleStatements(statements) = parsed;
                let semantics = rule_program_semantics(&entry.header, &statements, true);
                let statement_text = entry
                    .header
                    .text
                    .strip_suffix('{')
                    .unwrap_or(&entry.header.text)
                    .trim_end()
                    .to_string();
                let statement = puzzle_authoring::RuleStatementSyntax::new_block(
                    entry.header,
                    statement_text,
                    statements,
                );
                let name = match statement.tokens() {
                    [keyword, name] if keyword == "routine" => name.clone(),
                    [keyword, name, application] if keyword == "routine" => {
                        puzzle_authoring::rule_application_surface(application).ok_or_else(
                            || {
                                error_at(
                                    "routine application must be once, once_all, once_per_level, random, or repeat",
                                    statement.source(),
                                )
                            },
                        )?;
                        name.clone()
                    }
                    _ => {
                        return Err(error_at(
                            "routine header must be: routine <name> [once | once_all | once_per_level | random | repeat]",
                            statement.source(),
                        ));
                    }
                };
                if !routine_names.insert(name.clone()) {
                    return Err(error_at("duplicate routine definition", statement.source()));
                }
                body.routines.push(RuleRoutineSyntax {
                    statement,
                    semantics,
                });
            }
            puzzle_authoring::PuzzleDirectiveSurface::CollisionLayers => {
                return Err(error_at(
                    "`collision_layers` was removed; use `slots { ... }`",
                    &entry.header,
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::Inputs => {
                return Err(error_at(
                    "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                    &entry.header,
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedVariable => {
                return Err(error_at("`variable` was removed; use `var`", &entry.header));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedCondition => {
                return Err(error_at(
                    "`condition` declarations were removed; use `query`",
                    &entry.header,
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedEffect => {
                return Err(error_at(
                    "effect definitions are obsolete; use routine",
                    &entry.header,
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::SingularGroup => {
                let message = if crate::split_header_tokens(&entry.header.text).len() == 1 {
                    "`group { ... }` was removed; use `groups { ... }`"
                } else {
                    "`group <name> = ...` was removed; use `groups { <name> = ... }`"
                };
                return Err(error_at(message, &entry.header));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedFrameScreen => {
                return Err(error_at(
                    "`frame_*` screen directives were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`",
                    &entry.header,
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedRule => {
                return Err(error_at("`rule` was removed; use `routine`", &entry.header));
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedMain => {
                return Err(error_at("`main` was removed; use `rules`", &entry.header));
            }
            other => {
                let directive = crate::split_header_tokens(&entry.header.text)
                    .first()
                    .copied()
                    .unwrap_or("");
                return Err(error_at(
                    format!("unknown puzzle directive {directive} ({other:?})"),
                    &entry.header,
                ));
            }
        }
    }
    Ok((catalog_entries, body, dimension))
}

fn parse_variable_declaration(
    entry: &PuzzleEntrySyntax,
) -> Result<VariableDeclarationSyntax, DiagnosticReport> {
    if !entry.body.is_empty() {
        return Err(error_at("var or const cannot own a block", &entry.header));
    }
    let tokens = crate::split_header_tokens(&entry.header.text);
    let (name, value, persistent, constant) = match tokens.as_slice() {
        ["var", name, "=", value] => (*name, *value, false, false),
        ["const", name, "=", value] => (*name, *value, false, true),
        ["persistent", "var", name, "=", value] => (*name, *value, true, false),
        ["persistent", "const", name, "=", value] => (*name, *value, true, true),
        _ => {
            return Err(error_at(
                "var or const must be: var <name> = <true | false | number> or const <name> = <true | false | number>",
                &entry.header,
            ));
        }
    };
    if !puzzle_authoring::is_identifier(name) {
        return Err(error_at(
            "var or const name must be an identifier",
            &entry.header,
        ));
    }
    Ok(VariableDeclarationSyntax {
        name: name.to_string(),
        default: crate::parse_variable_value(value, &entry.header)?,
        numeric: value.parse::<i64>().is_ok(),
        persistent,
        constant,
        source: entry.header.clone(),
    })
}

fn parse_named_condition(
    entry: &PuzzleEntrySyntax,
) -> Result<NamedConditionSyntax, DiagnosticReport> {
    if !entry.body.is_empty() {
        return Err(error_at("assignment cannot own a block", &entry.header));
    }
    let Some((name, expression)) = puzzle_authoring::parse_assignment_row(&entry.header.text)
    else {
        return Err(error_at(
            "assignment must be: <name> = <value>",
            &entry.header,
        ));
    };
    if !puzzle_authoring::is_identifier(name) {
        return Err(error_at(
            "assignment name must be an identifier",
            &entry.header,
        ));
    }
    if !crate::looks_like_condition_expr(expression) {
        return Err(error_at(
            "tag sets must be declared inside `tags { ... }`",
            &entry.header,
        ));
    }
    Ok(NamedConditionSyntax {
        name: name.to_string(),
        expression: expression.to_string(),
        source: entry.header.clone(),
    })
}

fn parse_input_declaration(
    entry: &PuzzleEntrySyntax,
) -> Result<InputDeclarationSyntax, DiagnosticReport> {
    let tokens = crate::split_header_tokens(&entry.header.text);
    let (name, header_direction) = match tokens.as_slice() {
        ["input", name] => (*name, None),
        ["input", name, "direction", direction] => (*name, Some((*direction).to_string())),
        _ => {
            return Err(error_at(
                "input must be: input <name> [direction <direction>]",
                &entry.header,
            ));
        }
    };
    if !puzzle_authoring::is_identifier(name) {
        return Err(error_at("input name must be an identifier", &entry.header));
    }
    let mut body_direction = None;
    for line in entry
        .body
        .iter()
        .filter(|line| !line.text.trim().is_empty())
    {
        let tokens = crate::split_header_tokens(&line.text);
        let ["direction", direction] = tokens.as_slice() else {
            return Err(error_at(
                "input option must be: direction <direction>",
                line,
            ));
        };
        if body_direction.replace((*direction).to_string()).is_some() {
            return Err(error_at("duplicate input direction", line));
        }
    }
    if header_direction.is_some() && body_direction.is_some() {
        return Err(error_at("duplicate input direction", &entry.header));
    }
    Ok(InputDeclarationSyntax {
        name: name.to_string(),
        direction: header_direction.or(body_direction),
        source: entry.header.clone(),
    })
}

fn parse_direction_alias(
    entry: &PuzzleEntrySyntax,
) -> Result<DirectionAliasSyntax, DiagnosticReport> {
    if !entry.body.is_empty() {
        return Err(error_at("direction cannot own a block", &entry.header));
    }
    let tokens = crate::split_header_tokens(&entry.header.text);
    let ["direction", alias, canonical] = tokens.as_slice() else {
        return Err(error_at(
            "direction must be: direction <alias> <direction>",
            &entry.header,
        ));
    };
    if !puzzle_authoring::is_identifier(alias) {
        return Err(error_at(
            "direction alias must be an identifier",
            &entry.header,
        ));
    }
    Ok(DirectionAliasSyntax {
        alias: (*alias).to_string(),
        canonical: (*canonical).to_string(),
        source: entry.header.clone(),
    })
}

fn parse_render_overlay_syntax(
    entry: &PuzzleEntrySyntax,
) -> Result<RenderOverlaySyntax, DiagnosticReport> {
    if !entry.body.is_empty() {
        return Err(error_at("render_overlay cannot own a block", &entry.header));
    }
    let tokens = crate::split_header_tokens(&entry.header.text);
    if tokens.len() < 4 {
        return Err(error_at(
            "render_overlay must be: render_overlay <object> <object> [object...] <char>",
            &entry.header,
        ));
    }
    let value = tokens.last().expect("overlay token count checked");
    let mut chars = value.chars();
    let Some(character) = chars.next() else {
        return Err(error_at("missing overlay char", &entry.header));
    };
    if chars.next().is_some() {
        return Err(error_at("expected single character", &entry.header));
    }
    Ok(RenderOverlaySyntax {
        selectors: tokens[1..tokens.len() - 1]
            .iter()
            .map(|selector| (*selector).to_string())
            .collect(),
        character,
        source: entry.header.clone(),
    })
}

fn parse_model_sounds_syntax(
    entry: &PuzzleEntrySyntax,
    sounds: &mut ModelSoundsSyntax,
) -> Result<(), DiagnosticReport> {
    if crate::split_header_tokens(&entry.header.text).as_slice() != ["sounds"] {
        return Err(error_at(
            "model sounds header must be: sounds",
            &entry.header,
        ));
    }
    for line in entry
        .body
        .iter()
        .filter(|line| !line.text.trim().is_empty())
    {
        let tokens = crate::split_header_tokens(&line.text);
        match tokens.as_slice() {
            ["move", selector, "->", "sfx", name] => {
                validate_qualified_name(name, line, "sfx name")?;
                sounds.triggers.push(ModelSoundTriggerSyntax {
                    selector: (*selector).to_string(),
                    sfx_name: (*name).to_string(),
                    source: line.clone(),
                });
            }
            [operation @ ("undo" | "restart"), "->", "sfx", name] => {
                validate_qualified_name(name, line, "sfx name")?;
                sounds.operations.push(ModelOperationSoundSyntax {
                    operation: if *operation == "undo" {
                        crate::ModelOperationSound::Undo
                    } else {
                        crate::ModelOperationSound::Restart
                    },
                    sfx_name: (*name).to_string(),
                    source: line.clone(),
                });
            }
            _ => {
                return Err(error_at(
                    "model sounds entry must be: move <object-selector> -> sfx <name> | undo -> sfx <name> | restart -> sfx <name>",
                    line,
                ));
            }
        }
    }
    Ok(())
}

fn validate_qualified_name(
    value: &str,
    line: &LogicalLine,
    label: &str,
) -> Result<(), DiagnosticReport> {
    if value.split('.').all(puzzle_authoring::is_identifier) {
        Ok(())
    } else {
        Err(error_at(
            format!("{label} must be a qualified identifier"),
            line,
        ))
    }
}

fn level_resource_header_semantics(line: &LogicalLine) -> ParserRecognition {
    let mut recognition = ParserRecognition::default();
    for (index, token) in line
        .tokens
        .iter()
        .filter(|token| token.text != "{" && token.text != "}")
        .enumerate()
    {
        recognition.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            if index == 0 || token.text == "of" {
                SurfaceSemanticKind::Keyword
            } else {
                SurfaceSemanticKind::State
            },
        );
    }
    recognition
}

fn rule_program_semantics(
    header: &LogicalLine,
    statements: &[puzzle_authoring::RuleStatementSyntax<LogicalLine>],
    routine: bool,
) -> SyntaxSemantics {
    let mut semantics = SyntaxSemantics::default();
    if routine {
        if let Some(spans) = puzzle_authoring::rule_routine_block_header_surface_spans(&header.text)
        {
            mark_relative(
                &mut semantics.fixed,
                header,
                spans.keyword,
                SurfaceSemanticKind::Keyword,
            );
            if let Some(name) = spans.name {
                mark_relative(
                    &mut semantics.fixed,
                    header,
                    name,
                    SurfaceSemanticKind::Effect,
                );
            }
            for modifier in spans.modifiers {
                mark_relative(
                    &mut semantics.fixed,
                    header,
                    modifier,
                    SurfaceSemanticKind::Keyword,
                );
            }
        }
    } else {
        for token in header
            .tokens
            .iter()
            .filter(|token| token.text != "{" && token.text != "}")
        {
            mark_token(
                &mut semantics.fixed,
                Some(token),
                SurfaceSemanticKind::Keyword,
            );
        }
    }
    collect_statement_semantics(statements, &mut semantics);
    semantics
}

fn collect_statement_semantics(
    statements: &[puzzle_authoring::RuleStatementSyntax<LogicalLine>],
    semantics: &mut SyntaxSemantics,
) {
    for statement in statements {
        project_statement_semantics(statement, semantics);
        if let Some(statements) = statement.statements() {
            collect_statement_semantics(statements, semantics);
        }
    }
}

fn project_statement_semantics(
    line: &puzzle_authoring::RuleStatementSyntax<LogicalLine>,
    semantics: &mut SyntaxSemantics,
) {
    for source in line.sources() {
        project_statement_source_semantics(source.line(), source.facts(), semantics);
    }
}

fn project_statement_source_semantics(
    source: &LogicalLine,
    source_semantics: &puzzle_authoring::RuleStatementFacts,
    semantics: &mut SyntaxSemantics,
) {
    project_rule_facts(source, &source_semantics.spans, semantics);
}

fn project_rule_facts(
    source: &LogicalLine,
    facts: &[puzzle_authoring::RuleSyntaxFact],
    semantics: &mut SyntaxSemantics,
) {
    for span in facts {
        match span.kind {
            puzzle_authoring::RuleSyntaxFactKind::Keyword => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Keyword,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Mark => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Mark,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Selector => {
                push_relative_selector(semantics, source, span.span.clone())
            }
            puzzle_authoring::RuleSyntaxFactKind::Variant => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Variant,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Binding => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Binding,
            ),
            puzzle_authoring::RuleSyntaxFactKind::State => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::State,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Input => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Input,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Call => mark_relative(
                &mut semantics.fixed,
                source,
                span.span.clone(),
                SurfaceSemanticKind::Effect,
            ),
            puzzle_authoring::RuleSyntaxFactKind::Effect => {
                let text = &source.text[span.span.clone()];
                collect_effect_semantics(source, text, span.span.start, semantics);
            }
        }
    }
}

fn win_condition_semantics(header: &LogicalLine, rows: &[LogicalLine]) -> SyntaxSemantics {
    let mut semantics = SyntaxSemantics::default();
    mark_token(
        &mut semantics.fixed,
        header.tokens.first(),
        SurfaceSemanticKind::Condition,
    );
    for line in rows {
        let Ok(surface) = puzzle_authoring::win_condition_row_surface(&line.text) else {
            continue;
        };
        match surface {
            puzzle_authoring::WinConditionRowSurface::AllOn { subject, cover }
            | puzzle_authoring::WinConditionRowSurface::SomeOn { subject, cover } => {
                mark_token(
                    &mut semantics.fixed,
                    line.tokens.first(),
                    SurfaceSemanticKind::Keyword,
                );
                push_selector(&mut semantics, line, subject);
                mark_text(
                    &mut semantics.fixed,
                    line,
                    "on",
                    SurfaceSemanticKind::Keyword,
                );
                push_selector(&mut semantics, line, cover);
            }
            puzzle_authoring::WinConditionRowSurface::Query { argument, .. } => {
                mark_token(
                    &mut semantics.fixed,
                    line.tokens.first(),
                    SurfaceSemanticKind::Keyword,
                );
                push_selector(&mut semantics, line, argument);
                project_rule_facts(
                    line,
                    &puzzle_authoring::pattern_semantic_surface_spans(&line.text),
                    &mut semantics,
                );
            }
            puzzle_authoring::WinConditionRowSurface::Expression(_) => {}
        }
    }
    semantics
}

fn collect_effect_semantics(
    line: &LogicalLine,
    text: &str,
    relative_start: usize,
    semantics: &mut SyntaxSemantics,
) {
    let Some(base) = logical_line_source_base(line) else {
        return;
    };
    let tokens = crate::source_line_tokens(text, base + relative_start);
    let document = crate::rewrite_effect_surface_document(&tokens);
    semantics.fixed.merge_surface_document(document);
}

fn mark_relative(
    recognition: &mut ParserRecognition,
    line: &LogicalLine,
    span: std::ops::Range<usize>,
    kind: SurfaceSemanticKind,
) {
    let Some(base) = logical_line_source_base(line) else {
        return;
    };
    recognition.mark(
        SourceSpan {
            start: base + span.start,
            end: base + span.end,
        },
        kind,
    );
}

fn push_relative_selector(
    semantics: &mut SyntaxSemantics,
    line: &LogicalLine,
    span: std::ops::Range<usize>,
) {
    let Some(base) = logical_line_source_base(line) else {
        return;
    };
    semantics.selectors.push(SelectorOccurrenceSyntax {
        span: SourceSpan {
            start: base + span.start,
            end: base + span.end,
        },
        text: line.text[span].to_string(),
    });
}

fn logical_line_source_base(line: &LogicalLine) -> Option<usize> {
    let first = line.tokens.first()?;
    let relative = line.text.find(&first.text)?;
    first.start.checked_sub(relative)
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
        RuleStatementsSyntax::new(
            line.clone(),
            (!modifier.trim().is_empty()).then(|| modifier.trim().to_string()),
            statements,
        ),
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
                    .semantics
                    .fixed
                    .merge(level_resource_header_semantics(&entry.header));
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
            puzzle_authoring::PuzzleDirectiveSurface::Sprites => models[model_index]
                .body
                .sprite_resources
                .push(entry.clone()),
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
    let mut depth = header.structural_brace_delta();
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
                semantics: entry_semantic_syntax(&header, &[], directive),
                header,
                body: Vec::new(),
                closing: None,
                directive,
            },
            start + 1,
        ));
    }

    let body_start = start + 1;
    let mut index = body_start;
    while index < lines.len() && depth > 0 {
        depth += lines[index].structural_brace_delta();
        if depth < 0 {
            return Err(error_at(
                format!("{owner} entry has an unmatched }}"),
                &lines[index],
            ));
        }
        index += 1;
    }
    if depth != 0 {
        let directive = puzzle_authoring::puzzle_directive_surface(&header.text);
        let body = lines[body_start..].to_vec();
        return Ok((
            PuzzleEntrySyntax {
                semantics: entry_semantic_syntax(&header, &body, directive),
                header,
                body,
                closing: None,
                directive,
            },
            lines.len(),
        ));
    }
    let directive = puzzle_authoring::puzzle_directive_surface(&header.text);
    let body = lines[body_start..index - 1].to_vec();
    let closing = lines.get(index - 1).cloned();
    Ok((
        PuzzleEntrySyntax {
            semantics: entry_semantic_syntax(&header, &body, directive),
            header,
            body,
            closing,
            directive,
        },
        index,
    ))
}

fn entry_semantic_syntax(
    header: &LogicalLine,
    body: &[LogicalLine],
    directive: puzzle_authoring::PuzzleDirectiveSurface,
) -> SyntaxSemantics {
    let mut semantics = SyntaxSemantics::default();
    mark_token(
        &mut semantics.fixed,
        header.tokens.first(),
        SurfaceSemanticKind::Keyword,
    );
    match directive {
        puzzle_authoring::PuzzleDirectiveSurface::Tags => {
            for line in body {
                let Some(assignment) = puzzle_authoring::selector_assignment_surface(&line.text)
                else {
                    continue;
                };
                semantics
                    .fixed
                    .completion_symbols
                    .value_set_names
                    .insert(assignment.name.to_string());
                semantics.fixed.completion_symbols.value_sets.insert(
                    assignment.name.to_string(),
                    assignment
                        .selectors
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect(),
                );
                mark_text(
                    &mut semantics.fixed,
                    line,
                    assignment.name,
                    SurfaceSemanticKind::Group,
                );
                for value in assignment.selectors {
                    mark_text(
                        &mut semantics.fixed,
                        line,
                        value,
                        SurfaceSemanticKind::Variant,
                    );
                }
            }
        }
        puzzle_authoring::PuzzleDirectiveSurface::Groups => {
            for line in body {
                let Some(assignment) = puzzle_authoring::selector_assignment_surface(&line.text)
                else {
                    continue;
                };
                semantics
                    .fixed
                    .completion_symbols
                    .groups
                    .insert(assignment.name.to_string());
                mark_text(
                    &mut semantics.fixed,
                    line,
                    assignment.name,
                    SurfaceSemanticKind::Group,
                );
                for selector in assignment.selectors {
                    push_selector(&mut semantics, line, selector);
                }
            }
        }
        puzzle_authoring::PuzzleDirectiveSurface::Slots => {
            for line in body {
                let Some(row) = puzzle_authoring::slot_row_surface(&line.text) else {
                    continue;
                };
                let selectors = match row {
                    puzzle_authoring::SlotRowSurface::Named(assignment) => {
                        semantics
                            .fixed
                            .completion_symbols
                            .groups
                            .insert(assignment.name.to_string());
                        mark_text(
                            &mut semantics.fixed,
                            line,
                            assignment.name,
                            SurfaceSemanticKind::Group,
                        );
                        assignment.selectors
                    }
                    puzzle_authoring::SlotRowSurface::Each { selectors } => {
                        mark_text(
                            &mut semantics.fixed,
                            line,
                            "each",
                            SurfaceSemanticKind::Keyword,
                        );
                        selectors
                    }
                    puzzle_authoring::SlotRowSurface::Anonymous { selectors } => selectors,
                };
                for selector in selectors {
                    register_slot_completion_selector(&mut semantics.fixed, selector);
                    push_selector(&mut semantics, line, selector);
                }
            }
        }
        puzzle_authoring::PuzzleDirectiveSurface::Marks => {
            for line in body {
                mark_token(
                    &mut semantics.fixed,
                    line.tokens.first(),
                    SurfaceSemanticKind::Mark,
                );
            }
        }
        puzzle_authoring::PuzzleDirectiveSurface::Map => {
            let tokens = crate::split_header_tokens(&header.text);
            if let [_, name, axis] = tokens.as_slice() {
                for value in [*name, *axis] {
                    mark_text(
                        &mut semantics.fixed,
                        header,
                        value,
                        SurfaceSemanticKind::Group,
                    );
                }
            }
            for line in body {
                if let [from, "->", to] = crate::split_header_tokens(&line.text).as_slice() {
                    for value in [*from, *to] {
                        mark_text(
                            &mut semantics.fixed,
                            line,
                            value,
                            SurfaceSemanticKind::Variant,
                        );
                    }
                }
            }
        }
        _ => {}
    }
    semantics
}

fn register_slot_completion_selector(recognition: &mut ParserRecognition, selector: &str) {
    let mut parts = selector.split(':');
    let Some(object) = parts.next().filter(|name| !name.is_empty()) else {
        return;
    };
    recognition
        .completion_symbols
        .objects
        .insert(object.to_string());
    recognition
        .completion_symbols
        .object_name_atoms
        .insert(object.to_string());
    let axes = parts
        .filter(|axis| !axis.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !axes.is_empty() {
        recognition
            .completion_symbols
            .object_axes
            .entry(object.to_string())
            .or_default()
            .extend(axes);
    }
}

fn mark_token(
    recognition: &mut ParserRecognition,
    token: Option<&crate::source::SourceToken>,
    kind: SurfaceSemanticKind,
) {
    if let Some(token) = token {
        recognition.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            kind,
        );
    }
}

fn mark_text(
    recognition: &mut ParserRecognition,
    line: &LogicalLine,
    text: &str,
    kind: SurfaceSemanticKind,
) {
    for token in &line.tokens {
        if token.text == text {
            mark_token(recognition, Some(token), kind);
        }
    }
}

fn push_selector(semantics: &mut SyntaxSemantics, line: &LogicalLine, selector: &str) {
    for token in &line.tokens {
        if token.text == selector {
            semantics.selectors.push(SelectorOccurrenceSyntax {
                span: SourceSpan {
                    start: token.start,
                    end: token.end,
                },
                text: selector.to_string(),
            });
        }
    }
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
        assert!(models[1].body.rules.is_some());
    }

    #[test]
    fn nested_dimension_is_rejected_by_the_nested_owner() {
        let error = parse(
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
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("unknown render setting dimension"),
            "{error}"
        );
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
        assert_eq!(model.body.levels.levels.len(), 1);
        assert_eq!(model.body.sprite_resources.len(), 1);
    }
}
