use std::collections::{BTreeMap, BTreeSet};

use crate::ast::EffectAst;
use crate::loaded::SceneEffect;
use crate::source::{LogicalLine, SourceScope, SourceToken};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Facts emitted while the canonical parser accepts syntax. Parser functions
/// return these facts with their semantic value; editor products only project
/// them and never recognize source text independently.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParserRecognition {
    pub(crate) nodes: Vec<SurfaceNode>,
    pub(crate) token_dispositions: Vec<ParserTokenDisposition>,
    pub(crate) display_facts: Vec<SurfaceDisplayFact>,
    pub(crate) imports: Vec<crate::SourceImportDeclaration>,
    pub(crate) completion_symbols: SurfaceCompletionSymbols,
    pub(crate) visual_refs: SurfaceVisualRefs,
    pub(crate) sound_products: Vec<SurfaceSoundProduct>,
    pub(crate) level_products: Vec<SurfaceLevelProduct>,
    pub(crate) visual_resources: Vec<SurfaceVisualResourceProduct>,
    pub(crate) visual_asset_blocks: Vec<SurfaceVisualAssetBlockProduct>,
    pub(crate) visual_color_definitions: Vec<SurfaceVisualColorDefinitionProduct>,
    pub(crate) visual_shape_definitions: Vec<SurfaceVisualShapeDefinitionProduct>,
    pub(crate) visual_products: Vec<SurfaceVisualProduct>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceSoundKind {
    Sfx,
    Music,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceSoundProduct {
    pub(crate) span: SourceSpan,
    pub(crate) kind: SurfaceSoundKind,
    pub(crate) name: String,
    pub(crate) params: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceLevelProduct {
    pub(crate) span: SourceSpan,
    pub(crate) body_span: SourceSpan,
    pub(crate) name: String,
    pub(crate) dimension: crate::ModelDimension,
    pub(crate) pack: Option<String>,
    pub(crate) puzzle: Option<String>,
    pub(crate) level_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualResourceProduct {
    pub(crate) span: SourceSpan,
    pub(crate) open_brace: usize,
    pub(crate) close_brace: usize,
    pub(crate) dimension: crate::ModelDimension,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceVisualAssetBlockKind {
    Palette,
    Shapes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualAssetBlockProduct {
    pub(crate) span: SourceSpan,
    pub(crate) open_brace: usize,
    pub(crate) close_brace: usize,
    pub(crate) kind: SurfaceVisualAssetBlockKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualColorDefinitionProduct {
    pub(crate) name: String,
    pub(crate) value_span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualShapeDefinitionProduct {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
    pub(crate) header: String,
    pub(crate) braced: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualProduct {
    pub(crate) span: SourceSpan,
    pub(crate) body_span: SourceSpan,
    pub(crate) name: String,
    pub(crate) dimension: crate::ModelDimension,
    pub(crate) body: crate::visual_authoring::VisualBodyProduct,
    pub(crate) shape_asset_name: Option<String>,
}

impl ParserRecognition {
    pub(crate) fn mark(&mut self, span: SourceSpan, kind: SurfaceSemanticKind) {
        if span.start < span.end {
            self.token_dispositions.push(ParserTokenDisposition {
                span,
                kind: ParserTokenDispositionKind::Semantic(kind),
                resolution: None,
            });
        }
    }

    pub(crate) fn mark_invalid(&mut self, span: SourceSpan) {
        if span.start < span.end {
            self.token_dispositions.push(ParserTokenDisposition {
                span,
                kind: ParserTokenDispositionKind::InvalidSyntax,
                resolution: None,
            });
        }
    }

    pub(crate) fn mark_resolved(
        &mut self,
        span: SourceSpan,
        kind: SurfaceSemanticKind,
        resolution: ParserTokenResolution,
    ) {
        if span.start < span.end {
            self.token_dispositions.push(ParserTokenDisposition {
                span,
                kind: ParserTokenDispositionKind::Semantic(kind),
                resolution: Some(resolution),
            });
        }
    }

    pub(crate) fn node(&mut self, kind: SurfaceNodeKind, span: SourceSpan) {
        self.nodes.push(SurfaceNode { kind, span });
    }

    pub(crate) fn finish(mut self) -> Self {
        self.nodes
            .sort_by_key(|node| (node.span.start, node.span.end));
        self.nodes.dedup();
        self.token_dispositions
            .sort_by_key(|token| (token.span.start, token.span.end));
        self.token_dispositions.dedup();
        self.display_facts.sort_by_key(|fact| {
            let span = fact.span();
            (span.start, span.end)
        });
        self.imports
            .sort_by_key(|import| (import.range.start, import.range.end));
        self
    }

    pub(crate) fn merge(&mut self, other: ParserRecognition) {
        self.nodes.extend(other.nodes);
        self.token_dispositions.extend(other.token_dispositions);
        self.display_facts.extend(other.display_facts);
        self.imports.extend(other.imports);
        self.completion_symbols.merge(other.completion_symbols);
        self.visual_refs.merge(other.visual_refs);
        self.sound_products.extend(other.sound_products);
        self.level_products.extend(other.level_products);
        self.visual_resources.extend(other.visual_resources);
        self.visual_asset_blocks.extend(other.visual_asset_blocks);
        self.visual_color_definitions
            .extend(other.visual_color_definitions);
        self.visual_shape_definitions
            .extend(other.visual_shape_definitions);
        self.visual_products.extend(other.visual_products);
    }

    pub(crate) fn shift_offsets(&mut self, threshold: usize, delta: i64) {
        for node in &mut self.nodes {
            shift_span(&mut node.span, threshold, delta);
        }
        for token in &mut self.token_dispositions {
            shift_span(&mut token.span, threshold, delta);
        }
        for fact in &mut self.display_facts {
            match fact {
                SurfaceDisplayFact::LevelCell { span, .. }
                | SurfaceDisplayFact::VisualPixel { span, .. }
                | SurfaceDisplayFact::Color { span, .. }
                | SurfaceDisplayFact::LevelSeparator { span }
                | SurfaceDisplayFact::VisualSeparator { span } => {
                    shift_span(span, threshold, delta);
                }
            }
        }
        for import in &mut self.imports {
            import.shift_offsets(threshold, delta);
        }
        for product in &mut self.visual_products {
            shift_span(&mut product.span, threshold, delta);
            shift_span(&mut product.body_span, threshold, delta);
        }
        for product in &mut self.level_products {
            shift_span(&mut product.span, threshold, delta);
            shift_span(&mut product.body_span, threshold, delta);
        }
        for product in &mut self.sound_products {
            shift_span(&mut product.span, threshold, delta);
        }
        for resource in &mut self.visual_resources {
            shift_span(&mut resource.span, threshold, delta);
            shift_offset(&mut resource.open_brace, threshold, delta);
            shift_offset(&mut resource.close_brace, threshold, delta);
        }
        for block in &mut self.visual_asset_blocks {
            shift_span(&mut block.span, threshold, delta);
            shift_offset(&mut block.open_brace, threshold, delta);
            shift_offset(&mut block.close_brace, threshold, delta);
        }
        for definition in &mut self.visual_color_definitions {
            shift_span(&mut definition.value_span, threshold, delta);
        }
        for definition in &mut self.visual_shape_definitions {
            shift_span(&mut definition.span, threshold, delta);
        }
    }
}

fn shift_offset(offset: &mut usize, threshold: usize, delta: i64) {
    if *offset >= threshold {
        *offset =
            usize::try_from(*offset as i64 + delta).expect("incremental parser offset underflow");
    }
}

fn shift_span(span: &mut SourceSpan, threshold: usize, delta: i64) {
    if span.start >= threshold {
        span.start = usize::try_from(span.start as i64 + delta)
            .expect("incremental parser span start underflow");
        span.end = usize::try_from(span.end as i64 + delta)
            .expect("incremental parser span end underflow");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParseProduct<T> {
    pub(crate) value: T,
    pub(crate) recognition: ParserRecognition,
}

impl<T> ParseProduct<T> {
    pub(crate) fn new(value: T, recognition: ParserRecognition) -> Self {
        Self {
            value,
            recognition: recognition.finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceSemanticKind {
    Keyword,
    Literal,
    Binding,
    Effect,
    Emission,
    Object,
    Input,
    State,
    Group,
    Mark,
    Variant,
    Condition,
    Scene,
    Theme,
    Asset,
    Setting,
    Color,
    Number,
    String,
}

/// Symbol target selected by canonical parser resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParserTokenResolution {
    Object(String),
    ObjectGroup(String),
    ValueSet(String),
    ObjectAxis(String),
    Variant(String),
    ValueMap(String),
    Binding(String),
    Sfx(String),
    Music(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParserTokenDispositionKind {
    Semantic(SurfaceSemanticKind),
    InvalidSyntax,
}

/// Canonical parser-owned terminal disposition for a source token or token
/// component. Highlighting may project this fact, but does not own its span,
/// semantic role, invalidity, or resolved symbol target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParserTokenDisposition {
    pub(crate) span: SourceSpan,
    pub(crate) kind: ParserTokenDispositionKind,
    pub(crate) resolution: Option<ParserTokenResolution>,
}

/// Editor-facing semantic projection. This type never feeds parser acceptance
/// or lowering decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceSemanticToken {
    pub(crate) span: SourceSpan,
    pub(crate) kind: SurfaceSemanticKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceNodeKind {
    SceneEffect,
    RewriteEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceNode {
    pub(crate) kind: SurfaceNodeKind,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceDocument {
    pub(crate) logical_lines: Vec<LogicalLine>,
    pub(crate) lines: Vec<SurfaceLine>,
    pub(crate) structural_blocks: Vec<SurfaceStructuralBlock>,
    pub(crate) nodes: Vec<SurfaceNode>,
    pub(crate) semantic_tokens: Vec<SurfaceSemanticToken>,
    pub(crate) invalid_syntax_spans: Vec<SourceSpan>,
    pub(crate) unclassified_highlight_spans: Vec<SourceSpan>,
    pub(crate) unmatched_open_braces: BTreeSet<usize>,
    pub(crate) completion_symbols: SurfaceCompletionSymbols,
    pub(crate) highlight_ranges: SurfaceHighlightRanges,
    pub(crate) imports: Vec<crate::SourceImportDeclaration>,
    pub(crate) visual_refs: SurfaceVisualRefs,
    pub(crate) sound_products: Vec<SurfaceSoundProduct>,
    pub(crate) level_products: Vec<SurfaceLevelProduct>,
    pub(crate) visual_resources: Vec<SurfaceVisualResourceProduct>,
    pub(crate) visual_asset_blocks: Vec<SurfaceVisualAssetBlockProduct>,
    pub(crate) visual_color_definitions: Vec<SurfaceVisualColorDefinitionProduct>,
    pub(crate) visual_shape_definitions: Vec<SurfaceVisualShapeDefinitionProduct>,
    pub(crate) visual_products: Vec<SurfaceVisualProduct>,
    pub(crate) diagnostics: Vec<crate::Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceLine {
    pub(crate) tokens: Vec<String>,
    pub(crate) token_spans: Vec<SourceToken>,
    pub(crate) scope: Option<SourceScope>,
    pub(crate) start: usize,
    pub(crate) line: usize,
    pub(crate) content: String,
    pub(crate) lexical_facts: Vec<crate::source::lexer::SourceLexicalFact>,
    pub(crate) option_block: Option<SurfaceOptionBlock>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceStructuralBlockRole {
    SourceTree,
    Statement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceOutlinePolicy {
    Hidden,
    Visible,
    CollapseChildren,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceOutlineBlock {
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) suppress_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceStructuralBlock {
    pub(crate) header: String,
    pub(crate) scope: SourceScope,
    pub(crate) role: SurfaceStructuralBlockRole,
    pub(crate) authoring_kind: Option<crate::authoring_grammar::AuthoringKind>,
    pub(crate) authoring_content: Option<crate::authoring_grammar::AuthoringContentKind>,
    pub(crate) outline: Option<SurfaceOutlineBlock>,
    pub(crate) virtual_braces: bool,
    pub(crate) open_brace: Option<usize>,
    pub(crate) close_brace: Option<usize>,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) depth: usize,
    pub(crate) parent: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceHighlightRanges {
    pub(crate) raw_ranges: Vec<SourceSpan>,
    pub(crate) plain_ranges: Vec<SourceSpan>,
    pub(crate) display_facts: Vec<SurfaceDisplayFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceDisplayFact {
    LevelCell {
        span: SourceSpan,
        known: bool,
    },
    VisualPixel {
        span: SourceSpan,
        color: Option<crate::highlight::SourceHighlightColor>,
        transparent: bool,
    },
    Color {
        span: SourceSpan,
        color: crate::highlight::SourceHighlightColor,
    },
    LevelSeparator {
        span: SourceSpan,
    },
    VisualSeparator {
        span: SourceSpan,
    },
}

impl SurfaceDisplayFact {
    pub(crate) fn span(&self) -> SourceSpan {
        match self {
            Self::LevelCell { span, .. }
            | Self::VisualPixel { span, .. }
            | Self::Color { span, .. }
            | Self::LevelSeparator { span }
            | Self::VisualSeparator { span } => *span,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceVisualRefs {
    pub(crate) color_names: BTreeSet<String>,
    pub(crate) shape_names: BTreeSet<String>,
    pub(crate) color_assets: BTreeMap<String, String>,
    pub(crate) shape_assets: BTreeMap<String, SurfaceVisualShapeAsset>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceVisualShapeAsset {
    Plain {
        frames: Vec<crate::visual_authoring::VisualFrameSyntax>,
    },
    Table {
        axis: String,
        variants: BTreeMap<String, crate::visual_authoring::VisualFrameSyntax>,
    },
}

impl SurfaceVisualRefs {
    pub(crate) fn merge(&mut self, other: SurfaceVisualRefs) {
        self.color_names.extend(other.color_names);
        self.shape_names.extend(other.shape_names);
        self.color_assets.extend(other.color_assets);
        self.shape_assets.extend(other.shape_assets);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceOptionBlock {
    Puzzle2,
    Authoring(crate::authoring_grammar::AuthoringKind),
    Other,
}

impl SurfaceOptionBlock {
    pub(crate) fn authoring_parent_kind(self) -> Option<crate::authoring_grammar::AuthoringKind> {
        match self {
            SurfaceOptionBlock::Puzzle2 => Some(crate::authoring_grammar::AuthoringKind::Root),
            SurfaceOptionBlock::Authoring(kind) => Some(kind),
            SurfaceOptionBlock::Other => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceCompletionSymbols {
    pub(crate) objects: BTreeSet<String>,
    pub(crate) groups: BTreeSet<String>,
    pub(crate) states: BTreeSet<String>,
    pub(crate) markes: BTreeSet<String>,
    pub(crate) value_set_names: BTreeSet<String>,
    pub(crate) object_name_atoms: BTreeSet<String>,
    pub(crate) directions: BTreeSet<String>,
    pub(crate) direction_sets: BTreeSet<String>,
    pub(crate) inputs: BTreeSet<String>,
    pub(crate) commands: BTreeSet<String>,
    pub(crate) effects: BTreeSet<String>,
    pub(crate) model_effects: BTreeSet<String>,
    pub(crate) scene_effects: BTreeSet<String>,
    pub(crate) emissions: BTreeSet<String>,
    pub(crate) routines: BTreeSet<String>,
    pub(crate) condition_defs: BTreeSet<String>,
    pub(crate) puzzles: BTreeSet<String>,
    pub(crate) scenes: BTreeSet<String>,
    pub(crate) levels: BTreeSet<String>,
    pub(crate) sfx: BTreeSet<String>,
    pub(crate) music: BTreeSet<String>,
    pub(crate) visuals: BTreeSet<String>,
    pub(crate) assets: BTreeSet<String>,
    pub(crate) shapes: BTreeSet<String>,
    pub(crate) colors: BTreeSet<String>,
    pub(crate) value_sets: BTreeMap<String, Vec<String>>,
    pub(crate) object_axes: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SurfaceSink {
    document: SurfaceDocument,
}

impl SurfaceSink {
    pub(crate) fn project_parser_recognition(&mut self, recognition: &ParserRecognition) {
        self.document
            .nodes
            .extend(recognition.nodes.iter().copied());
        for disposition in &recognition.token_dispositions {
            match disposition.kind {
                ParserTokenDispositionKind::Semantic(kind) => {
                    self.document.semantic_tokens.push(SurfaceSemanticToken {
                        span: disposition.span,
                        kind,
                    });
                }
                ParserTokenDispositionKind::InvalidSyntax => {
                    self.document.invalid_syntax_spans.push(disposition.span);
                }
            }
        }
        self.document
            .highlight_ranges
            .display_facts
            .extend(recognition.display_facts.iter().cloned());
    }

    pub(crate) fn project_parser_completion(&mut self, recognition: &ParserRecognition) {
        self.document
            .completion_symbols
            .merge(recognition.completion_symbols.clone());
    }

    pub(crate) fn project_parser_source_targets(&mut self, recognition: &ParserRecognition) {
        self.document
            .imports
            .extend(recognition.imports.iter().cloned());
        self.document
            .visual_refs
            .merge(recognition.visual_refs.clone());
        self.document
            .sound_products
            .extend(recognition.sound_products.iter().cloned());
        self.document
            .level_products
            .extend(recognition.level_products.iter().cloned());
        self.document
            .visual_products
            .extend(recognition.visual_products.iter().cloned());
        self.document
            .visual_resources
            .extend(recognition.visual_resources.iter().cloned());
        self.document
            .visual_asset_blocks
            .extend(recognition.visual_asset_blocks.iter().cloned());
        self.document
            .visual_color_definitions
            .extend(recognition.visual_color_definitions.iter().cloned());
        self.document
            .visual_shape_definitions
            .extend(recognition.visual_shape_definitions.iter().cloned());
    }

    pub(crate) fn line(
        &mut self,
        tokens: Vec<String>,
        token_spans: Vec<SourceToken>,
        scope: Option<SourceScope>,
        start: usize,
        line: usize,
        content: String,
        lexical_facts: Vec<crate::source::lexer::SourceLexicalFact>,
        option_block: Option<SurfaceOptionBlock>,
    ) {
        self.document.lines.push(SurfaceLine {
            tokens,
            token_spans,
            scope,
            start,
            line,
            content,
            lexical_facts,
            option_block,
        });
    }

    pub(crate) fn set_structural_blocks(&mut self, blocks: Vec<SurfaceStructuralBlock>) {
        self.document.structural_blocks = blocks;
    }

    pub(crate) fn set_unmatched_open_braces(&mut self, braces: BTreeSet<usize>) {
        self.document.unmatched_open_braces = braces;
    }

    pub(crate) fn completion_symbols_mut(&mut self) -> &mut SurfaceCompletionSymbols {
        &mut self.document.completion_symbols
    }

    pub(crate) fn set_highlight_ranges(&mut self, ranges: SurfaceHighlightRanges) {
        self.document.highlight_ranges.merge(ranges);
    }

    pub(crate) fn into_document(mut self) -> SurfaceDocument {
        self.document
            .semantic_tokens
            .sort_by_key(|token| (token.span.start, token.span.end));
        self.document
            .invalid_syntax_spans
            .sort_by_key(|span| (span.start, span.end));
        self.document
            .unclassified_highlight_spans
            .sort_by_key(|span| (span.start, span.end));
        self.document.highlight_ranges.sort_by_source();
        self.document
    }
}

impl SurfaceHighlightRanges {
    fn sort_by_source(&mut self) {
        self.raw_ranges
            .sort_by_key(|range| (range.start, range.end));
        self.plain_ranges
            .sort_by_key(|range| (range.start, range.end));
        self.display_facts.sort_by_key(|fact| {
            let span = fact.span();
            (span.start, span.end)
        });
    }

    pub(crate) fn merge(&mut self, other: SurfaceHighlightRanges) {
        self.raw_ranges.extend(other.raw_ranges);
        self.plain_ranges.extend(other.plain_ranges);
        self.display_facts.extend(other.display_facts);
    }
}

impl SurfaceCompletionSymbols {
    pub(crate) fn merge(&mut self, other: SurfaceCompletionSymbols) {
        self.objects.extend(other.objects);
        self.groups.extend(other.groups);
        self.states.extend(other.states);
        self.markes.extend(other.markes);
        self.value_set_names.extend(other.value_set_names);
        self.object_name_atoms.extend(other.object_name_atoms);
        self.directions.extend(other.directions);
        self.direction_sets.extend(other.direction_sets);
        self.inputs.extend(other.inputs);
        self.commands.extend(other.commands);
        self.effects.extend(other.effects);
        self.model_effects.extend(other.model_effects);
        self.scene_effects.extend(other.scene_effects);
        self.emissions.extend(other.emissions);
        self.routines.extend(other.routines);
        self.condition_defs.extend(other.condition_defs);
        self.puzzles.extend(other.puzzles);
        self.scenes.extend(other.scenes);
        self.levels.extend(other.levels);
        self.sfx.extend(other.sfx);
        self.music.extend(other.music);
        self.visuals.extend(other.visuals);
        self.assets.extend(other.assets);
        self.shapes.extend(other.shapes);
        self.colors.extend(other.colors);
        for (name, values) in other.value_sets {
            self.value_sets.entry(name).or_insert(values);
        }
        for (name, axes) in other.object_axes {
            self.object_axes.entry(name).or_insert(axes);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceSceneEffect {
    pub(crate) effect: SceneEffect,
    pub(crate) document: SurfaceDocument,
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceRewriteEffect {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) document: SurfaceDocument,
}
