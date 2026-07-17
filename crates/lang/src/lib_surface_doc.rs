pub fn parse_game(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    parse_game_document(source)
}

pub fn parse_game_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<LoadedDocument, DiagnosticReport> {
    let profile = puzzle_source_profile_for_path(path.as_ref()).ok_or_else(|| {
        DiagnosticReport::error(format!(
            "puzzle source must use .puzzle or .puzzle3 extension: {}",
            path.as_ref().display()
        ))
    })?;
    validate_source_profile(source, profile)?;
    parse_game_document_with_profile(source, profile)
}

pub fn parse_game2d(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    parse_game2d_document(source)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceDocumentProducts {
    semantic_tokens: bool,
    completion_symbols: bool,
    highlight_ranges: bool,
    source_targets: bool,
}

impl SurfaceDocumentProducts {
    const FULL: Self = Self {
        semantic_tokens: true,
        completion_symbols: true,
        highlight_ranges: true,
        source_targets: true,
    };

    const STRUCTURE_ONLY: Self = Self {
        semantic_tokens: false,
        completion_symbols: false,
        highlight_ranges: false,
        source_targets: false,
    };

    const SOURCE_TARGET: Self = Self {
        semantic_tokens: false,
        completion_symbols: false,
        highlight_ranges: false,
        source_targets: true,
    };

    const COMPLETION_SYMBOLS: Self = Self {
        semantic_tokens: false,
        completion_symbols: true,
        highlight_ranges: false,
        source_targets: false,
    };

    fn needs_parser_catalog(self) -> bool {
        self.semantic_tokens || self.completion_symbols || self.source_targets
    }
}

/// One revision-local parser product shared by strict compilation and editor
/// projections. Migration-era scan, catalog, and document data are kept behind
/// this single authority so they cannot be cached or advanced independently.
pub(crate) struct ParseSnapshot {
    source_scan: source::SurfaceSourceScan,
    parser_catalog:
        Option<crate::surface::ParseProduct<Result<LevelEditorIntegration, DiagnosticReport>>>,
    document: SurfaceDocument,
    strict_diagnostic: Option<DiagnosticReport>,
}

impl ParseSnapshot {
    pub(crate) fn parse(source: &str, source_profile: Option<PuzzleSourceProfile>) -> Self {
        let source_scan = source::scan_surface_source(source);
        let parser_catalog = Some(parser_surface_catalog_from_source_scan(
            &source_scan,
            source_profile,
        ));
        Self::from_scan(source_profile, source_scan, parser_catalog)
    }

    fn parse_for_compile(source: &str) -> Self {
        let source_scan = source::scan_surface_source(source);
        Self::from_scan(None, source_scan, None)
    }

    fn from_scan(
        source_profile: Option<PuzzleSourceProfile>,
        source_scan: source::SurfaceSourceScan,
        parser_catalog: Option<
            crate::surface::ParseProduct<Result<LevelEditorIntegration, DiagnosticReport>>,
        >,
    ) -> Self {
        let parser_product = parser_catalog.as_ref();
        let mut document =
            build_surface_document_from_source_scan(&source_scan, parser_product, source_profile);
        let strict_diagnostic = match source_scan.strict_logical_lines() {
            Ok(logical_lines) => {
                document.logical_lines = logical_lines;
                None
            }
            Err(report) => {
                document
                    .diagnostics
                    .extend(report.diagnostics().iter().cloned());
                Some(report)
            }
        };
        if let Some(parser_product) = &parser_catalog
            && let Err(report) = &parser_product.value
        {
            document
                .diagnostics
                .extend(report.diagnostics().iter().cloned());
        }
        Self {
            source_scan,
            parser_catalog,
            document,
            strict_diagnostic,
        }
    }

    pub(crate) fn document(&self) -> &SurfaceDocument {
        &self.document
    }

    pub(crate) fn level_editor_integration(&self) -> Result<&LevelEditorIntegration, String> {
        self.parser_catalog
            .as_ref()
            .expect("source analysis snapshot requires parser product")
            .value
            .as_ref()
            .map_err(ToString::to_string)
    }

    fn into_strict_document(self) -> Result<SurfaceDocument, DiagnosticReport> {
        if let Some(report) = self.strict_diagnostic {
            return Err(report);
        }
        Ok(self.document)
    }

    pub(crate) fn apply_edit(
        &mut self,
        old_source: &str,
        new_source: &str,
        source_profile: Option<PuzzleSourceProfile>,
        edit_start: usize,
        edit_end: usize,
        insert_len: usize,
    ) -> (usize, bool) {
        let old_grammar_fingerprint = self.source_scan.grammar_fingerprint();
        let mut source_scan = std::mem::take(&mut self.source_scan);
        let rescanned_lines =
            source_scan.apply_edit(old_source, new_source, edit_start, edit_end, insert_len);
        let parser_catalog_reused = old_grammar_fingerprint == source_scan.grammar_fingerprint();
        let parser_catalog = if parser_catalog_reused {
            self.parser_catalog.take()
        } else {
            Some(parser_surface_catalog_from_source_scan(
                &source_scan,
                source_profile,
            ))
        };
        *self = Self::from_scan(source_profile, source_scan, parser_catalog);
        (rescanned_lines, parser_catalog_reused)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.source_scan.line_count()
    }
}

pub(crate) fn parse_surface_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::FULL)
}

fn parse_surface_compile_document(source: &str) -> Result<SurfaceDocument, DiagnosticReport> {
    ParseSnapshot::parse_for_compile(source).into_strict_document()
}

pub fn validate_surface_document_projection(source: &str) -> Result<(), DiagnosticReport> {
    let document = try_build_surface_document(source, SurfaceDocumentProducts::FULL)?;
    validate_parser_recognition_completeness(&document)
}

fn validate_parser_recognition_completeness(
    document: &SurfaceDocument,
) -> Result<(), DiagnosticReport> {
    let Some(span) = document.syntax_error_spans.first() else {
        return Ok(());
    };
    Err(DiagnosticReport::error(format!(
        "canonical parser product contains token at {}..{} without a syntax disposition",
        span.start, span.end
    )))
}

pub(crate) fn parse_surface_structure_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)
}

pub(crate) fn parse_surface_completion_context_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)
}

pub(crate) fn parse_surface_completion_symbols_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::COMPLETION_SYMBOLS)
}

fn parse_surface_source_target_document(source: &str) -> SurfaceDocument {
    build_surface_document(source, SurfaceDocumentProducts::SOURCE_TARGET)
}

fn parse_surface_source_target_document_for_profile(
    source: &str,
    source_profile: PuzzleSourceProfile,
) -> SurfaceDocument {
    try_build_surface_document_with_profile(
        source,
        SurfaceDocumentProducts::SOURCE_TARGET,
        Some(source_profile),
    )
    .expect("surface document scan failed")
}

fn build_surface_document(source: &str, products: SurfaceDocumentProducts) -> SurfaceDocument {
    try_build_surface_document(source, products).expect("surface document scan failed")
}

fn try_build_surface_document(
    source: &str,
    products: SurfaceDocumentProducts,
) -> Result<SurfaceDocument, DiagnosticReport> {
    try_build_surface_document_with_profile(source, products, None)
}

fn try_build_surface_document_with_profile(
    source: &str,
    products: SurfaceDocumentProducts,
    source_profile: Option<PuzzleSourceProfile>,
) -> Result<SurfaceDocument, DiagnosticReport> {
    let source_scan = source::scan_surface_source(source);
    let parser_catalog = products
        .needs_parser_catalog()
        .then(|| parser_surface_catalog_from_source_scan(&source_scan, source_profile));
    let mut document = try_build_surface_document_from_scan(
        &source_scan,
        products,
        parser_catalog.as_ref(),
        source_profile,
    )?;
    if let Some(parser_product) = parser_catalog
        && let Err(report) = parser_product.value
    {
        document.diagnostics.extend(report.into_diagnostics());
    }
    Ok(document)
}

fn try_build_surface_document_from_scan(
    scan: &source::SurfaceSourceScan,
    products: SurfaceDocumentProducts,
    parser_catalog: Option<
        &crate::surface::ParseProduct<Result<LevelEditorIntegration, DiagnosticReport>>,
    >,
    source_profile: Option<PuzzleSourceProfile>,
) -> Result<SurfaceDocument, DiagnosticReport> {
    let mut sink = SurfaceSink::default();
    let structural_blocks = surface_structural_blocks(&scan);
    sink.set_structural_blocks(structural_blocks.clone());
    sink.set_unmatched_open_braces(
        scan.unmatched_open_braces()
            .iter()
            .map(|brace| brace.start)
            .collect(),
    );
    if products.completion_symbols {
        project_builtin_completion_symbols(&mut sink);
    }
    for line in &scan.lines {
        sink.line(
            line.tokens.clone(),
            line.token_spans.clone(),
            line.scope,
            line.start,
            line.line,
            line.content.clone(),
            line.lexical_facts.clone(),
            line.option_block,
        );
        if products.semantic_tokens {
            for piece in &line.structural_pieces {
                sink.project_parser_recognition(&piece.product.recognition);
            }
        }
        if products.completion_symbols {
            for piece in &line.structural_pieces {
                sink.project_parser_completion(&piece.product.recognition);
            }
        }
    }
    if let Some(parser_product) = parser_catalog {
        if products.semantic_tokens {
            sink.project_parser_recognition(&parser_product.recognition);
        }
        if products.source_targets {
            sink.project_parser_source_targets(&parser_product.recognition);
        }
        if products.completion_symbols {
            sink.project_parser_completion(&parser_product.recognition);
        }
    }
    if products.highlight_ranges {
        sink.set_highlight_ranges(surface_highlight_ranges(&scan));
    }
    if products.completion_symbols {
        normalize_surface_completion_symbols(&mut sink);
    }
    let mut document = sink.into_document();
    document.source_profile = source_profile;
    if products.semantic_tokens {
        document.syntax_error_spans = syntax_error_spans(&document);
    }
    Ok(document)
}

fn syntax_error_spans(document: &SurfaceDocument) -> Vec<SourceSpan> {
    document
        .lines
        .iter()
        .flat_map(|line| &line.lexical_facts)
        .filter(|fact| matches!(fact.kind, source::lexer::SourceLexicalKind::Word))
        .filter(|fact| {
            !document
                .semantic_tokens
                .iter()
                .any(|token| token.span.start <= fact.start && token.span.end >= fact.end)
                && !document
                    .highlight_ranges
                    .raw_ranges
                    .iter()
                    .any(|span| span.start <= fact.start && span.end >= fact.end)
                && !document
                    .highlight_ranges
                    .display_facts
                    .iter()
                    .any(|display| {
                        let span = display.span();
                        span.start <= fact.start && span.end >= fact.end
                    })
        })
        .map(|fact| SourceSpan {
            start: fact.start,
            end: fact.end,
        })
        .collect()
}

pub(crate) fn build_surface_document_from_source_scan(
    source_scan: &source::SurfaceSourceScan,
    parser_catalog: Option<
        &crate::surface::ParseProduct<Result<LevelEditorIntegration, DiagnosticReport>>,
    >,
    source_profile: Option<PuzzleSourceProfile>,
) -> SurfaceDocument {
    try_build_surface_document_from_scan(
        source_scan,
        SurfaceDocumentProducts::FULL,
        parser_catalog,
        source_profile,
    )
    .expect("surface document scan failed")
}

fn surface_structural_blocks(scan: &source::SurfaceSourceScan) -> Vec<SurfaceStructuralBlock> {
    let mut blocks = Vec::<SurfaceStructuralBlock>::new();
    let mut stack = Vec::<usize>::new();
    for line in &scan.lines {
        if line.tokens.first().is_some_and(|token| token == "selector") {
            if let Some(index) = stack.iter().rev().copied().find(|index| {
                blocks[*index].authoring_kind
                    == Some(authoring_grammar::AuthoringKind::SpriteConfig)
            }) && let Some(selector) = authoring_grammar::authoring_definition_single_value(
                authoring_grammar::AuthoringKind::SpriteConfig,
                "selector",
                &line.content,
            )
            .ok()
            .flatten()
                && let Some(outline) = blocks[index].outline.as_mut()
            {
                outline.label = selector;
            }
        }
        for event in &line.structural_events {
            match event {
                source::SourceStructureEvent::Open {
                    header,
                    scope,
                    role,
                    virtual_braces,
                    option_block,
                } => {
                    let tokens = split_header_tokens(header)
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    let authoring_content = tokens.first().and_then(|surface| {
                        authoring_grammar::authoring_source_block(surface)
                            .and_then(|spec| spec.content)
                    });
                    let parent = stack.iter().rev().find_map(|index| {
                        matches!(blocks[*index].role, SurfaceStructuralBlockRole::SourceTree)
                            .then_some(*index)
                    });
                    let authoring_kind = match *option_block {
                        SurfaceOptionBlock::Authoring(kind) => Some(kind),
                        _ => None,
                    };
                    let role = surface_structural_block_role(*role);
                    let outline = surface_outline_block(
                        header,
                        &tokens,
                        *scope,
                        role,
                        authoring_kind,
                        *virtual_braces,
                        parent,
                        &blocks,
                    );
                    let block = SurfaceStructuralBlock {
                        header: header.clone(),
                        scope: *scope,
                        role,
                        authoring_kind,
                        authoring_content,
                        outline,
                        virtual_braces: *virtual_braces,
                        open_brace: (!virtual_braces)
                            .then(|| line.content.rfind('{').map(|offset| line.start + offset))
                            .flatten(),
                        close_brace: None,
                        start: line_start_offset(&line.content, line.start),
                        end: line.start + line.content.len(),
                        depth: stack
                            .iter()
                            .filter(|index| {
                                matches!(
                                    blocks[**index].role,
                                    SurfaceStructuralBlockRole::SourceTree
                                )
                            })
                            .count(),
                        parent,
                    };
                    let index = blocks.len();
                    blocks.push(block);
                    stack.push(index);
                }
                source::SourceStructureEvent::Close => {
                    if let Some(index) = stack.pop() {
                        blocks[index].close_brace =
                            line.content.find('}').map(|offset| line.start + offset);
                    }
                }
            }
        }
    }
    blocks
}

fn surface_outline_block(
    header: &str,
    tokens: &[String],
    scope: SourceScope,
    role: SurfaceStructuralBlockRole,
    authoring_kind: Option<authoring_grammar::AuthoringKind>,
    virtual_braces: bool,
    parent: Option<usize>,
    blocks: &[SurfaceStructuralBlock],
) -> Option<SurfaceOutlineBlock> {
    if role != SurfaceStructuralBlockRole::SourceTree {
        return None;
    }
    let policy = authoring_kind
        .map(|kind| authoring_grammar::authoring_kind_spec(kind).outline_policy)
        .unwrap_or(authoring_grammar::AuthoringOutlinePolicy::Visible);
    if policy == authoring_grammar::AuthoringOutlinePolicy::Hidden {
        return None;
    }
    let first = tokens.first().cloned().unwrap_or_default();
    let visual_shape_entry = scope == SourceScope::VisualShapeEntry;
    let sprite_entry = authoring_kind == Some(authoring_grammar::AuthoringKind::SpriteConfig)
        || (visual_shape_entry
            && parent
                .and_then(|index| blocks.get(index))
                .is_some_and(|block| {
                    block.authoring_kind == Some(authoring_grammar::AuthoringKind::SpritesConfig)
                }));
    let kind = if sprite_entry {
        "sprite".to_string()
    } else if visual_shape_entry {
        "shape".to_string()
    } else if let Some(authoring_kind) = authoring_kind {
        authoring_grammar::authoring_kind_spec(authoring_kind)
            .header
            .usage
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        first.clone()
    };
    let label = if visual_shape_entry
        && authoring_kind != Some(authoring_grammar::AuthoringKind::SpriteConfig)
    {
        first.clone()
    } else {
        header.to_string()
    };
    let suppress_children = policy == authoring_grammar::AuthoringOutlinePolicy::CollapseChildren
        || (authoring_kind.is_none()
            && (matches!(
                first.as_str(),
                "keys" | "inputs" | "routine" | "query" | "fix"
            ) || first.starts_with("on_")
                || virtual_braces));
    Some(SurfaceOutlineBlock {
        kind,
        label,
        suppress_children,
    })
}

fn surface_structural_block_role(role: source::SourceBlockRole) -> SurfaceStructuralBlockRole {
    match role {
        source::SourceBlockRole::SourceTree => SurfaceStructuralBlockRole::SourceTree,
        source::SourceBlockRole::Statement => SurfaceStructuralBlockRole::Statement,
    }
}

fn line_start_offset(content: &str, line_start: usize) -> usize {
    line_start + content.len() - content.trim_start().len()
}

fn surface_highlight_ranges(scan: &source::SurfaceSourceScan) -> SurfaceHighlightRanges {
    SurfaceHighlightRanges {
        raw_ranges: scan
            .raw_ranges()
            .iter()
            .map(|(start, end)| SourceSpan {
                start: *start,
                end: *end,
            })
            .collect(),
        plain_ranges: scan
            .plain_ranges()
            .iter()
            .map(|(start, end)| SourceSpan {
                start: *start,
                end: *end,
            })
            .collect(),
        display_facts: Vec::new(),
    }
}

pub(crate) struct SurfaceDocumentSemantics {
    pub(crate) tokens: Vec<semantic::SemanticToken>,
}

pub(crate) fn surface_document_semantics(source: &str) -> SurfaceDocumentSemantics {
    let document = parse_surface_document(source);
    SurfaceDocumentSemantics {
        tokens: surface_document_semantic_tokens(&document),
    }
}

pub(crate) fn surface_document_semantic_tokens(
    document: &SurfaceDocument,
) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&document.semantic_tokens)
}

#[cfg(test)]
mod surface_document_flow_tests {
    use super::{
        parse_surface_completion_context_document, parse_surface_completion_symbols_document,
        parse_surface_document, parse_surface_structure_document, source_line_tokens,
        validate_surface_document_projection,
    };
    use crate::surface::SurfaceSemanticKind;

    #[test]
    fn bare_shape_reference_is_not_highlighted_as_inline_pixels() {
        let source = r##"
puzzle board {
slots {
actors = Box
}
sprites {
Box
#111 #eee
box_shape

shapes {
box_shape {
010
111
010
}
}
}
rules {
}
levels {
legend {
. = empty
B = Box
}
level "one"
B
}
}
"##;
        let reference_start = source.find("box_shape\n\nshapes").unwrap();
        let reference_end = reference_start + "box_shape".len();
        let document = parse_surface_document(source);

        assert!(
            document
                .visual_sprite_refs
                .shape_names
                .contains("box_shape")
        );
        assert!(
            document.highlight_ranges.display_facts.iter().all(|fact| {
                !matches!(fact, crate::SurfaceDisplayFact::SpritePixel { span, .. }
                    if span.end > reference_start && span.start < reference_end)
            }),
            "shape references must not be projected as inline pixel rows"
        );
    }

    #[test]
    fn canonical_sprite_parser_projects_palette_and_pixel_facts() {
        let source = r##"
puzzle board {
slots {
actors = Box
}
sprites {
Box
#111 #eee
01.
}
}
"##;
        let document = parse_surface_document(source);
        let color_start = source.find("#111").unwrap();
        let row_start = source.find("01.").unwrap();

        assert!(
            document
                .highlight_ranges
                .display_facts
                .contains(&crate::SurfaceDisplayFact::Color {
                    span: crate::surface::SourceSpan {
                        start: color_start,
                        end: color_start + 4,
                    },
                    color: "#111".to_string(),
                }),
            "facts={:?} diagnostics={:?}",
            document.highlight_ranges.display_facts,
            document.diagnostics
        );
        assert!(document.highlight_ranges.display_facts.contains(
            &crate::SurfaceDisplayFact::SpritePixel {
                span: crate::surface::SourceSpan {
                    start: row_start + 1,
                    end: row_start + 2,
                },
                color: "#eee".to_string(),
                transparent: false,
            }
        ));
        assert!(document.highlight_ranges.display_facts.contains(
            &crate::SurfaceDisplayFact::SpritePixel {
                span: crate::surface::SourceSpan {
                    start: row_start + 2,
                    end: row_start + 3,
                },
                color: "transparent".to_string(),
                transparent: true,
            }
        ));
    }

    #[test]
    fn canonical_level_parser_projects_cell_display_facts() {
        let source = "puzzle default {\nslots {\nactor = Box\n}\n}\nlevels {\nlegend {\nB = Box\n. = empty\n}\nlevel \"one\"\nB?\n}\n";
        let document = parse_surface_document(source);
        let row_start = source.find("B?\n").unwrap();

        assert!(
            document.highlight_ranges.display_facts.contains(
                &crate::SurfaceDisplayFact::LevelCell {
                    span: crate::surface::SourceSpan {
                        start: row_start,
                        end: row_start + 1,
                    },
                    known: true,
                }
            ),
            "facts={:?} diagnostics={:?}",
            document.highlight_ranges.display_facts,
            document.diagnostics
        );
        assert!(document.highlight_ranges.display_facts.contains(
            &crate::SurfaceDisplayFact::LevelCell {
                span: crate::surface::SourceSpan {
                    start: row_start + 1,
                    end: row_start + 2,
                },
                known: false,
            }
        ));
    }

    #[test]
    fn structure_only_surface_document_shares_full_structural_product() {
        let source = r#"
puzzle board {
  rules {
    routine push {
      if some([ Player ]) {
        [ Player ] -> [ Player ]
      }
    }
  }
  levels {
    level "one"
    P
  }
}
"#;
        let full = parse_surface_document(source);
        let structure = parse_surface_structure_document(source);
        assert_eq!(structure.lines, full.lines);
        assert_eq!(structure.structural_blocks, full.structural_blocks);
        assert!(structure.semantic_tokens.is_empty());
        assert!(structure.highlight_ranges.raw_ranges.is_empty());
        assert!(structure.completion_symbols.objects.is_empty());
    }

    #[test]
    fn completion_context_surface_document_skips_derived_products() {
        let source = r#"
puzzle board {
  sounds {
    sfx click = "click.wav"
  }
  rules {
  }
}
"#;
        let full = parse_surface_document(source);
        let context = parse_surface_completion_context_document(source);
        assert_eq!(context.lines, full.lines);
        assert_eq!(context.structural_blocks, full.structural_blocks);
        assert!(context.semantic_tokens.is_empty());
        assert!(context.highlight_ranges.raw_ranges.is_empty());
        assert!(context.completion_symbols.sfx.is_empty());
        assert!(context.visual_sprite_refs.color_names.is_empty());
    }

    #[test]
    fn completion_symbols_surface_document_skips_non_completion_products() {
        let source = r#"
puzzle board {
  sounds {
    sfx click = "click.wav"
  }
}
"#;
        let symbols = parse_surface_completion_symbols_document(source);
        assert!(symbols.completion_symbols.sfx.contains("click"));
        assert!(symbols.semantic_tokens.is_empty());
        assert!(symbols.highlight_ranges.raw_ranges.is_empty());
        assert!(symbols.visual_sprite_refs.color_names.is_empty());
    }

    #[test]
    fn surface_document_entrypoints_share_single_builder() {
        let source = include_str!("lib_surface_doc.rs");
        let required = [
            "build_surface_document(source, SurfaceDocumentProducts::FULL)",
            "build_surface_document(source, SurfaceDocumentProducts::STRUCTURE_ONLY)",
            "build_surface_document(source, SurfaceDocumentProducts::COMPLETION_SYMBOLS)",
            "build_surface_document(source, SurfaceDocumentProducts::SOURCE_TARGET)",
        ];
        for required in required {
            assert!(
                source.contains(required),
                "surface document entrypoints must delegate to one builder via {required}"
            );
        }
    }

    #[test]
    fn surface_products_consume_parser_scope_without_a_mirror_scan() {
        let surface_doc = include_str!("lib_surface_doc.rs");
        assert!(
            surface_doc.contains("scan: &source::SurfaceSourceScan"),
            "surface products must consume the canonical source scan directly"
        );
        assert!(
            !surface_doc.contains(concat!("Surface", "VisualScope"))
                && !surface_doc.contains(concat!("recognize_", "surface_scan_lines")),
            "surface products must not rebuild parser-owned visual scope"
        );
    }

    #[test]
    fn semantic_tokens_follow_structural_blocks_not_header_whitelists() {
        let surface_doc_source = include_str!("lib_surface_doc.rs");
        assert!(
            surface_doc_source.contains("project_parser_recognition(&piece.product.recognition)"),
            "semantic tokens must project the parser product"
        );
        assert!(
            !surface_doc_source.contains(concat!("record_structural_block_", "surface_tokens"))
                && !surface_doc_source.contains(concat!("record_", "surface_document_line"))
                && !surface_doc_source.contains(concat!("scan_level_ascii_", "surface_ranges")),
            "surface documents must not retain grammar recognizers"
        );
        let source_scanner_source = include_str!("source.rs");
        assert!(
            !source_scanner_source.contains("source_tree_header_keyword"),
            "source scanner must not own highlight header whitelist decisions"
        );
    }

    #[test]
    fn every_structural_header_token_receives_a_surface_token() {
        let source = r#"
puzzle board {
rules {
routine Push once {
[ Player ] -> [ > Player ]
}
}
}
"#;
        let document = parse_surface_document(source);

        for block in document.structural_blocks.iter() {
            for header_token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|token| {
                        token.span.start == header_token.start && token.span.end == header_token.end
                    }),
                    "structural block `{}` left header token `{}` without a surface token",
                    block.header,
                    header_token.text
                );
            }
        }
    }

    #[test]
    fn puzzle_statement_headers_receive_surface_tokens() {
        let source = r#"
puzzle board {
rules {
}
on_level_start {
}
on_level_clear {
}
}
"#;
        let document = parse_surface_document(source);

        for header in ["rules", "on_level_start", "on_level_clear"] {
            let start = source.find(&format!("{header} {{")).unwrap();
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + header.len()
                        && token.kind == SurfaceSemanticKind::Keyword
                }),
                "statement block header `{header}` did not receive a keyword surface token"
            );
        }
    }

    #[test]
    fn scene_keys_and_routine_headers_receive_surface_tokens() {
        let source = r#"
scene title {
keys {
Enter -> start playing
}
routine continue_game {
goto playing
}
}
"#;
        let document = parse_surface_document(source);

        for (needle, text, kind) in [
            ("keys {", "keys", SurfaceSemanticKind::Keyword),
            (
                "routine continue_game",
                "routine",
                SurfaceSemanticKind::Keyword,
            ),
            (
                "routine continue_game",
                "continue_game",
                SurfaceSemanticKind::Binding,
            ),
        ] {
            let needle_start = source.find(needle).unwrap();
            let start = source[needle_start..].find(text).unwrap() + needle_start;
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + text.len()
                        && token.kind == kind
                }),
                "scene token `{text}` did not receive {kind:?}"
            );
        }
    }

    #[test]
    fn reserved_literals_and_scene_options_receive_surface_tokens() {
        let source = r#"
puzzle board {
levels {
legend {
. = empty
}
level "start" {
.
}
}
}

scene level_select {
layout {
level_menu {
show_index = true
show_solved=false
columns = 3
}
}
}
"#;
        let document = parse_surface_document(source);

        for (needle, text, kind) in [
            (". = empty", "empty", SurfaceSemanticKind::Literal),
            (
                "show_index = true",
                "show_index",
                SurfaceSemanticKind::Setting,
            ),
            ("show_index = true", "true", SurfaceSemanticKind::Literal),
            (
                "show_solved=false",
                "show_solved",
                SurfaceSemanticKind::Setting,
            ),
            ("show_solved=false", "false", SurfaceSemanticKind::Literal),
            ("columns = 3", "columns", SurfaceSemanticKind::Setting),
            ("columns = 3", "3", SurfaceSemanticKind::Number),
        ] {
            let needle_start = source.find(needle).unwrap();
            let start = source[needle_start..].find(text).unwrap() + needle_start;
            assert!(
                document.semantic_tokens.iter().any(|token| {
                    token.span.start == start
                        && token.span.end == start + text.len()
                        && token.kind == kind
                }),
                "token `{text}` in `{needle}` did not receive {kind:?}"
            );
        }
    }

    #[test]
    fn condition_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
puzzle board {
win_conditions {
some([ Goal ])
}
lose_conditions any {
some([ Trap ])
}
}
"#;
        let document = parse_surface_document(source);

        for header in ["win_conditions", "lose_conditions any"] {
            let block = document
                .structural_blocks
                .iter()
                .find(|block| block.header == header)
                .expect("condition structural block");
            for token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|semantic| {
                        semantic.span.start == token.start && semantic.span.end == token.end
                    }),
                    "condition block `{}` left header token `{}` without a surface token",
                    block.header,
                    token.text
                );
            }
        }
    }

    #[test]
    fn unbraced_level_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
puzzle board {
levels {
legend {
P = Player
. = empty
x = Wall
}
level one
P.x

P.x
}
}
"#;
        let document = parse_surface_document(source);

        assert!(
            document
                .structural_blocks
                .iter()
                .any(|block| block.header == "level one"),
            "named unbraced level block should be present"
        );
        for header in ["P.x"] {
            let block = document
                .structural_blocks
                .iter()
                .find(|block| block.header == header)
                .expect("unbraced level structural block");
            for token in source_line_tokens(&block.header, block.start) {
                assert!(
                    document.semantic_tokens.iter().any(|semantic| {
                        semantic.span.start == token.start && semantic.span.end == token.end
                    }),
                    "unbraced level block `{}` left header token `{}` without a surface token",
                    block.header,
                    token.text
                );
            }
        }
    }

    #[test]
    fn scene_layout_source_tree_headers_are_owned_surface_tokens() {
        let source = r#"
scene playing {
layout {
row {
text "Ready"
}
}
}
"#;
        let document = parse_surface_document(source);
        let block = document
            .structural_blocks
            .iter()
            .find(|block| block.header == "layout")
            .expect("layout structural block");

        assert!(
            source_line_tokens(&block.header, block.start)
                .into_iter()
                .all(
                    |header_token| document.semantic_tokens.iter().any(|semantic| {
                        semantic.span.start == header_token.start
                            && semantic.span.end == header_token.end
                    })
                ),
            "layout header should be owned by scene surface projection"
        );
    }

    #[test]
    fn unowned_source_tree_header_reports_surface_projection_error() {
        let source = r#"
puzzle board {
__invalid_unowned_surface_node__ {
}
}
"#;
        let error =
            validate_surface_document_projection(source).expect_err("unowned header should fail");

        assert!(
            error.to_string().contains("without a syntax disposition"),
            "{error}"
        );
    }
}
