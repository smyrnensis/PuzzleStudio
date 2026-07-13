use std::collections::{BTreeMap, BTreeSet};

use crate::ast::EffectAst;
use crate::loaded::SceneEffect;
use crate::source::{LogicalLine, SourceScope, SourceToken};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
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
    pub(crate) source_profile: Option<crate::PuzzleSourceProfile>,
    pub(crate) logical_lines: Vec<LogicalLine>,
    pub(crate) lines: Vec<SurfaceLine>,
    pub(crate) structural_blocks: Vec<SurfaceStructuralBlock>,
    pub(crate) nodes: Vec<SurfaceNode>,
    pub(crate) semantic_tokens: Vec<SurfaceSemanticToken>,
    pub(crate) completion_symbols: SurfaceCompletionSymbols,
    pub(crate) highlight_ranges: SurfaceHighlightRanges,
    pub(crate) visual_sprite_refs: SurfaceVisualSpriteRefs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceLine {
    pub(crate) tokens: Vec<String>,
    pub(crate) token_spans: Vec<SourceToken>,
    pub(crate) scope: Option<SourceScope>,
    pub(crate) start: usize,
    pub(crate) line: usize,
    pub(crate) content: String,
    pub(crate) option_block: Option<SurfaceOptionBlock>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceStructuralBlockRole {
    SourceTree,
    Statement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceStructuralBlock {
    pub(crate) header: String,
    pub(crate) scope: SourceScope,
    pub(crate) role: SurfaceStructuralBlockRole,
    pub(crate) authoring_kind: Option<crate::authoring_grammar::AuthoringKind>,
    pub(crate) authoring_content: Option<crate::authoring_grammar::AuthoringContentKind>,
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
    pub(crate) level_ascii_ranges: Vec<SurfaceAsciiRange>,
    pub(crate) visual_ascii_color_ranges: Vec<SurfaceVisualAsciiColorRange>,
    pub(crate) visual_named_color_ranges: Vec<SurfaceVisualNamedColorRange>,
    pub(crate) visual_separator_ranges: Vec<SourceSpan>,
}

impl SurfaceHighlightRanges {
    pub(crate) fn raw_range_starting_at(&self, start: usize) -> Option<usize> {
        self.raw_ranges
            .iter()
            .find_map(|range| (range.start == start).then_some(range.end))
    }

    pub(crate) fn is_plain_range(&self, start: usize, end: usize) -> bool {
        self.plain_ranges
            .iter()
            .any(|range| start >= range.start && end <= range.end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceAsciiRange {
    pub(crate) span: SourceSpan,
    pub(crate) known: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualAsciiColorRange {
    pub(crate) span: SourceSpan,
    pub(crate) color: String,
    pub(crate) transparent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceVisualNamedColorRange {
    pub(crate) span: SourceSpan,
    pub(crate) color: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceVisualSpriteRefs {
    pub(crate) color_names: BTreeSet<String>,
    pub(crate) shape_names: BTreeSet<String>,
    pub(crate) color_assets: BTreeMap<String, String>,
    pub(crate) shape_assets: BTreeMap<String, Vec<String>>,
}

impl SurfaceVisualSpriteRefs {
    pub(crate) fn contains_color(&self, value: &str) -> bool {
        self.color_names.contains(value)
    }

    pub(crate) fn merge(&mut self, other: SurfaceVisualSpriteRefs) {
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
    LevelMenu,
    Other,
}

impl SurfaceOptionBlock {
    pub(crate) fn authoring_parent_kind(self) -> Option<crate::authoring_grammar::AuthoringKind> {
        match self {
            SurfaceOptionBlock::Puzzle2 => Some(crate::authoring_grammar::AuthoringKind::Root),
            SurfaceOptionBlock::Authoring(kind) => Some(kind),
            SurfaceOptionBlock::LevelMenu | SurfaceOptionBlock::Other => None,
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
    pub(crate) sprites: BTreeSet<String>,
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
    pub(crate) fn line(
        &mut self,
        tokens: Vec<String>,
        token_spans: Vec<SourceToken>,
        scope: Option<SourceScope>,
        start: usize,
        line: usize,
        content: String,
        option_block: Option<SurfaceOptionBlock>,
    ) {
        self.document.lines.push(SurfaceLine {
            tokens,
            token_spans,
            scope,
            start,
            line,
            content,
            option_block,
        });
    }

    pub(crate) fn node(&mut self, kind: SurfaceNodeKind, span: SourceSpan) {
        self.document.nodes.push(SurfaceNode { kind, span });
    }

    pub(crate) fn set_structural_blocks(&mut self, blocks: Vec<SurfaceStructuralBlock>) {
        self.document.structural_blocks = blocks;
    }

    pub(crate) fn mark(&mut self, span: SourceSpan, kind: SurfaceSemanticKind) {
        if span.start < span.end {
            self.document
                .semantic_tokens
                .push(SurfaceSemanticToken { span, kind });
        }
    }

    pub(crate) fn completion_symbols_mut(&mut self) -> &mut SurfaceCompletionSymbols {
        &mut self.document.completion_symbols
    }

    pub(crate) fn set_highlight_ranges(&mut self, ranges: SurfaceHighlightRanges) {
        self.document.highlight_ranges = ranges;
    }

    pub(crate) fn visual_sprite_refs_mut(&mut self) -> &mut SurfaceVisualSpriteRefs {
        &mut self.document.visual_sprite_refs
    }

    pub(crate) fn into_document(self) -> SurfaceDocument {
        self.document
    }

    pub(crate) fn has_semantic_tokens(&self) -> bool {
        !self.document.semantic_tokens.is_empty()
    }

    pub(crate) fn extend(&mut self, document: SurfaceDocument) {
        self.document.lines.extend(document.lines);
        self.document
            .structural_blocks
            .extend(document.structural_blocks);
        self.document.nodes.extend(document.nodes);
        self.document
            .semantic_tokens
            .extend(document.semantic_tokens);
        self.document
            .completion_symbols
            .merge(document.completion_symbols);
        self.document
            .highlight_ranges
            .merge(document.highlight_ranges);
        self.document
            .visual_sprite_refs
            .merge(document.visual_sprite_refs);
    }
}

impl SurfaceHighlightRanges {
    pub(crate) fn merge(&mut self, other: SurfaceHighlightRanges) {
        self.raw_ranges.extend(other.raw_ranges);
        self.plain_ranges.extend(other.plain_ranges);
        self.level_ascii_ranges.extend(other.level_ascii_ranges);
        self.visual_ascii_color_ranges
            .extend(other.visual_ascii_color_ranges);
        self.visual_named_color_ranges
            .extend(other.visual_named_color_ranges);
        self.visual_separator_ranges
            .extend(other.visual_separator_ranges);
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
        self.sprites.extend(other.sprites);
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
