mod ast;
mod catalog;
mod completion;
mod error;
mod highlight;
mod level;
mod loaded;
mod puzzlescript;
mod semantic;
mod source;
mod source_target;
mod surface;
mod syntax;

use std::collections::{BTreeSet, HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use ast::{
    ConditionAst, ConditionDefinitionAst, ConditionPatternAst, ConditionValueAst, Direction,
    DirectionName, EffectAst, FixDefaults, OrientationExpr, OrientedRewriteAst,
    PatternConditionAst, PatternPredicateAst, RuleDefinitionAst, RuleRole, StatementAst,
};
use catalog::{Catalog, ObjectSchema, ObjectVariant, ValueMap};
pub use completion::{
    CompletionItem, CompletionKind, CompletionList, completion_list_json,
    suggest_source_completions,
};
pub use error::{Diagnostic, DiagnosticReport, DiagnosticSeverity, DiagnosticSpan};
pub use highlight::{HighlightedSource, highlight_source};
use level::{LevelBlock, parse_level};
pub use loaded::{
    AnimationDef, ArrowKey, AsciiLegend, AssetDef, AssetKind, AssetsDef, Controls, ForSource,
    GoalClause, GoalCondition, GoalExpr, GoalValue, KeyBinding, KeyTrigger, Level, LevelMenuDef,
    LevelMenuLocked, LevelRegionDef, LoadedDocument, LoadedDocumentModel, LoadedGame,
    MusicSoundDef, PuzzleGridRenderDef, PuzzleRenderDef, PuzzleScreenDef, PuzzleViewDef,
    ResourceSelection, RuleAnimation, RuleAnimationTrigger, RuleEffect, SceneAlignDef,
    SceneAlignXDef, SceneAlignYDef, SceneButtonDef, SceneComponent, SceneConditionalDef,
    SceneContainerDef, SceneDef, SceneEffect, SceneEffectParam, SceneExpr, SceneForDef,
    SceneLayoutDef, ScenePuzzleDef, ScenePuzzleInitializer, ScenePuzzleRule, SceneResources,
    SceneRoutineDef, SceneSizeDef, SceneStateDef, SceneStateLifetime, SceneTextContent,
    SceneTextDef, SceneTitleDef, SceneTransition, SceneTransitionTrigger, SceneValue, SceneVarDef,
    SfxSoundDef, SoundsDef, ThemeDef, ThemeVariableDef, TweenAnimationDef, ViewportModeDef,
    ViewportSizeDef, VisualAliasDef, VisualColorDef, VisualSpriteDef, VisualSpriteKind,
    VisualSpriteOffset, VisualSpritePixelsPerCell, VisualsDef,
};

const BLOCK_CLOSE: &str = "}";

fn is_block_close_line(line: &str) -> bool {
    line == BLOCK_CLOSE
}

fn block_header_text(line: &str) -> &str {
    line.trim_end()
        .strip_suffix('{')
        .map(str::trim_end)
        .unwrap_or(line)
}
pub use puzzle_3d::{
    ParseError3, ParsedPuzzle3, VisualFixtureAnimation3, VisualFixtureExportError3,
    export_visual_fixture_json, export_visual_fixture_json_with_title,
    export_visual_fixture_json_with_title_and_scenes,
    export_visual_fixture_json_with_title_scenes_and_animation, parse_puzzle3d,
};
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionId, ConditionValueKind, Effect, GapTerm,
    GlobalId, GlobalUpdateOp, Guard, InputId, LayerId, LocalFrame, LocalFrameExtent, MatchCell,
    ObjectDef, ObjectId, ObjectSetMatcher, ObjectSetScratchPattern, Offset, Pattern,
    PatternComponent, Rule, RuleApplication, RuleCondition, RuleId, RuleStep, ScratchDef,
    ScratchId, ScratchKind, ScratchPattern, ScratchValueMatch, WriteOp,
};
pub use puzzlescript::translate_puzzlescript_to_canonical;
pub use semantic::{SemanticKind, SemanticToken, semantic_tokens};
use source::{
    SourceScope, SourceToken, logical_lines, scan_source_context, source_line_tokens,
    split_header_tokens, strip_line_comment,
};
pub use source_target::{
    SoundSourceTargetKind, SourceTarget, SourceTargetKind, resolve_source_target,
    source_target_json,
};
use surface::{
    SourceSpan, SurfaceDocument, SurfaceNodeKind, SurfaceRewriteEffect, SurfaceSceneEffect,
    SurfaceSemanticKind, SurfaceSemanticToken, SurfaceSink,
};
use syntax::puzzle_lifecycle_event;

const ANONYMOUS_MOVEMENT_SCRATCH: ScratchId =
    ScratchId(puzzle_authoring::ANONYMOUS_MOVEMENT_SCRATCH_INDEX);
const ANONYMOUS_BOOL_SCRATCH: ScratchId = ScratchId(1);
const ANONYMOUS_INT_SCRATCH: ScratchId = ScratchId(2);
const UNASSIGNED_LAYER: u16 = u16::MAX;
pub(crate) const THEME_PRESET_NAMES: &[&str] = &[
    "clean",
    "terminal",
    "paper",
    "pixel",
    "puzzlescript",
    "candy",
    "blueprint",
    "noir",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzleSourceProfile {
    Puzzle2d,
    Puzzle3d,
}

impl PuzzleSourceProfile {
    pub fn canonical_extension(self) -> &'static str {
        match self {
            PuzzleSourceProfile::Puzzle2d => "puzzle",
            PuzzleSourceProfile::Puzzle3d => "puzzle3",
        }
    }
}

pub fn puzzle_source_profile_for_extension(extension: &str) -> Option<PuzzleSourceProfile> {
    match extension {
        "puzzle" => Some(PuzzleSourceProfile::Puzzle2d),
        "puzzle3" => Some(PuzzleSourceProfile::Puzzle3d),
        _ => None,
    }
}

pub fn puzzle_source_profile_for_path(path: impl AsRef<Path>) -> Option<PuzzleSourceProfile> {
    path.as_ref()
        .extension()
        .and_then(|value| value.to_str())
        .and_then(puzzle_source_profile_for_extension)
}

pub fn is_puzzle_source_path(path: impl AsRef<Path>) -> bool {
    puzzle_source_profile_for_path(path).is_some()
}

pub(crate) struct ThemeSettingSpec {
    pub(crate) canonical: &'static str,
    pub(crate) css_variable: &'static str,
    pub(crate) aliases: &'static [&'static str],
}

pub(crate) const THEME_SETTING_SPECS: &[ThemeSettingSpec] = &[
    ThemeSettingSpec {
        canonical: "accent_color",
        css_variable: "accent",
        aliases: &["accent-color", "accent"],
    },
    ThemeSettingSpec {
        canonical: "background_color",
        css_variable: "background",
        aliases: &["background-color", "background", "bg"],
    },
    ThemeSettingSpec {
        canonical: "text_color",
        css_variable: "text",
        aliases: &["text-color", "text", "ink"],
    },
];

pub fn parse_game(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    parse_game_document(source)
}

pub fn parse_game_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<LoadedDocument, DiagnosticReport> {
    validate_source_profile_for_path(source, path)?;
    parse_game_document(source)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentRuntimeSources {
    pub model_2d: String,
    pub model_3d: String,
}

pub fn split_document_runtime_sources(
    source: &str,
) -> Result<DocumentRuntimeSources, DiagnosticReport> {
    let mixed_sources = split_mixed_game_document_source(source)?;
    Ok(DocumentRuntimeSources {
        model_2d: strip_document_shell_source(&mixed_sources.puzzle2d)?,
        model_3d: strip_document_shell_source_raw(&mixed_sources.puzzle3d),
    })
}

pub fn parse_game2d(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let _surface = parse_surface_document(source);
    parse_game2d_document(source)
}

pub(crate) fn parse_surface_document(source: &str) -> SurfaceDocument {
    let context = scan_source_context(source);
    let mut sink = SurfaceSink::default();
    for line in &context.lines {
        record_surface_document_line(line.scope, &line.token_spans, &mut sink);
    }
    sink.into_document()
}

pub(crate) fn surface_document_semantic_tokens(source: &str) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&parse_surface_document(source).semantic_tokens)
}

fn record_surface_document_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if record_document_prelude_surface_line(scope, tokens, sink) {
        return;
    }
    if tokens
        .first()
        .is_some_and(|token| token.text.as_str() == "scene")
    {
        record_scene_surface_line(scope, tokens, sink);
        return;
    }
    if is_scene_surface_scope(scope) {
        record_scene_surface_line(scope, tokens, sink);
        return;
    }
    record_rewrite_surface_line(scope, tokens, sink);
}

fn record_document_prelude_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    if scope.is_some() {
        return false;
    }
    let Some(first) = tokens.first() else {
        return false;
    };
    if matches!(
        first.text.as_str(),
        "title" | "subtitle" | "author" | "homepage"
    ) {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if let Some(value) = tokens.get(1) {
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::String);
        }
        return true;
    }
    false
}

fn is_scene_surface_scope(scope: Option<SourceScope>) -> bool {
    matches!(
        scope,
        Some(
            SourceScope::Scene
                | SourceScope::SceneLayout
                | SourceScope::SceneState
                | SourceScope::SceneKeys
                | SourceScope::SceneTransitions
                | SourceScope::LevelMenu
        )
    )
}

fn record_scene_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "scene" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
        if let Some(name) = tokens.get(1) {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::Scene);
        }
        return;
    }
    if scope == Some(SourceScope::SceneState) {
        return;
    }
    if scene_block_keyword(&first.text)
        || puzzle_scene::SceneComponentKind::from_keyword(&first.text).is_some()
    {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
    record_scene_layout_attr_surface_tokens(tokens, sink);
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        if tokens
            .iter()
            .any(|token| token.text.contains('[') || token.text.contains(']'))
        {
            return;
        }
        if matches!(first.text.as_str(), "button" | "choice") {
            sink.extend(scene_expr_surface_document(&tokens[1..arrow]));
        }
        record_scene_condition_surface_tokens(&tokens[..arrow], sink);
        sink.extend(scene_effect_surface_document(&tokens[arrow + 1..]));
        return;
    }
    match first.text.as_str() {
        "title" | "subtitle" | "text" if tokens.len() > 1 => {
            sink.extend(scene_expr_surface_document(&tokens[1..]));
            return;
        }
        _ => {}
    }
    if first.text == "button" || first.text == "choice" || scope == Some(SourceScope::LevelMenu) {
        return;
    }
    sink.extend(scene_effect_surface_document(tokens));
}

fn record_scene_layout_attr_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    let attr_start = match first.text.as_str() {
        "layout" | "row" | "column" | "box" => 1,
        "puzzle" | "puzzle3" | "frame" => 2,
        _ => return,
    };
    let attrs = tokens
        .iter()
        .skip(attr_start)
        .take_while(|token| token.text != "{")
        .cloned()
        .collect::<Vec<_>>();
    if attrs.is_empty() {
        return;
    }
    let attr_texts = attrs
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if puzzle_scene::parse_scene_layout_attrs(&attr_texts).is_err() {
        return;
    }
    for token in &attrs {
        mark_scene_layout_attr_token(token, sink);
    }
}

fn mark_scene_layout_attr_token(token: &SourceToken, sink: &mut SurfaceSink) {
    let (name, value) = token
        .text
        .split_once('=')
        .map_or((token.text.as_str(), None), |(name, value)| {
            (name, Some(value))
        });
    if !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
    {
        let name_start = token.text.find(name).unwrap_or(0);
        sink.mark(
            SourceSpan {
                start: token.start + name_start,
                end: token.start + name_start + name.len(),
            },
            SurfaceSemanticKind::Keyword,
        );
    }
    if let Some(value) = value
        && !value.is_empty()
    {
        let value_start = token.text.find(value).unwrap_or(token.text.len());
        let kind = if value.parse::<u16>().is_ok() {
            SurfaceSemanticKind::Number
        } else {
            SurfaceSemanticKind::Literal
        };
        sink.mark(
            SourceSpan {
                start: token.start + value_start,
                end: token.start + value_start + value.len(),
            },
            kind,
        );
    } else if token.text.parse::<u16>().is_ok() {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Number,
        );
    } else if matches!(
        token.text.as_str(),
        "left" | "right" | "center" | "top" | "bottom" | "true" | "false"
    ) {
        sink.mark(
            SourceSpan {
                start: token.start,
                end: token.end,
            },
            SurfaceSemanticKind::Literal,
        );
    }
}

fn scene_expr_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let Some((source, base_start)) = source_tokens_text(tokens) else {
        return sink.into_document();
    };
    let trimmed_start = source
        .find(|ch: char| !ch.is_whitespace())
        .unwrap_or(source.len());
    let trimmed_end = source
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| {
            index
                + source[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0)
        })
        .unwrap_or(trimmed_start);
    if trimmed_start >= trimmed_end {
        return sink.into_document();
    }
    let expr = &source[trimmed_start..trimmed_end];
    let expr_start = base_start + trimmed_start;
    if let Ok(parsed) = parse_scene_expr(expr, expr) {
        add_scene_expr_surface_tokens(&parsed, expr, expr_start, &mut sink);
    }
    sink.into_document()
}

fn source_tokens_text(tokens: &[SourceToken]) -> Option<(String, usize)> {
    let first = tokens.first()?;
    let mut out = String::new();
    let mut cursor = first.start;
    for token in tokens {
        if token.start > cursor {
            out.push_str(&" ".repeat(token.start - cursor));
        }
        out.push_str(&token.text);
        cursor = token.end;
    }
    Some((out, first.start))
}

fn add_scene_expr_surface_tokens(
    expr: &SceneExpr,
    source: &str,
    absolute_start: usize,
    sink: &mut SurfaceSink,
) {
    match expr {
        SceneExpr::Bool(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Literal);
        }
        SceneExpr::Int(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::Number);
        }
        SceneExpr::Text(_) => {
            mark_scene_expr_trimmed(sink, source, absolute_start, SurfaceSemanticKind::String);
        }
        SceneExpr::Path(parts) => {
            add_scene_path_surface_tokens(parts, absolute_start, sink);
        }
        SceneExpr::Call { name, args: _ } => {
            if let Some(name_start) = source.find(name) {
                sink.mark(
                    SourceSpan {
                        start: absolute_start + name_start,
                        end: absolute_start + name_start + name.len(),
                    },
                    SurfaceSemanticKind::Effect,
                );
            }
            for arg in scene_expr_call_arg_spans(source) {
                if let Ok(parsed) = parse_scene_expr(&source[arg.clone()], &source[arg.clone()]) {
                    add_scene_expr_surface_tokens(
                        &parsed,
                        &source[arg.clone()],
                        absolute_start + arg.start,
                        sink,
                    );
                } else {
                    let arg_source = &source[arg.clone()];
                    if let Some(parts) = parse_view_path(arg_source) {
                        add_scene_path_surface_tokens(&parts, absolute_start + arg.start, sink);
                    }
                }
            }
        }
    }
}

fn mark_scene_expr_trimmed(
    sink: &mut SurfaceSink,
    source: &str,
    absolute_start: usize,
    kind: SurfaceSemanticKind,
) {
    let Some(start) = source.find(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let Some(end_start) = source.rfind(|ch: char| !ch.is_whitespace()) else {
        return;
    };
    let end = end_start
        + source[end_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    if start < end {
        sink.mark(
            SourceSpan {
                start: absolute_start + start,
                end: absolute_start + end,
            },
            kind,
        );
    }
}

fn add_scene_path_surface_tokens(parts: &[String], absolute_start: usize, sink: &mut SurfaceSink) {
    let part_refs = parts.iter().map(String::as_str).collect::<Vec<_>>();
    let mut offset = 0usize;
    for (index, part) in part_refs.iter().enumerate() {
        if let Some(syntax) = level_path_part_syntax(&part_refs, index) {
            let kind = match syntax {
                LevelPathPartSyntax::Owner => SurfaceSemanticKind::State,
                LevelPathPartSyntax::TextProperty => SurfaceSemanticKind::String,
                LevelPathPartSyntax::NumberProperty => SurfaceSemanticKind::Number,
                LevelPathPartSyntax::ConditionProperty => SurfaceSemanticKind::Condition,
            };
            sink.mark(
                SourceSpan {
                    start: absolute_start + offset,
                    end: absolute_start + offset + part.len(),
                },
                kind,
            );
        }
        offset += part.len() + 1;
    }
}

fn scene_expr_call_arg_spans(source: &str) -> Vec<std::ops::Range<usize>> {
    let Some(open) = source.find('(') else {
        return Vec::new();
    };
    let Some(close) = source.rfind(')') else {
        return Vec::new();
    };
    if open >= close {
        return Vec::new();
    }
    let mut args = Vec::new();
    let mut start = open + 1;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (relative, ch) in source[open + 1..close].char_indices() {
        let index = open + 1 + relative;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if let Some(range) = trimmed_range(source, start, index) {
                    args.push(range);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if let Some(range) = trimmed_range(source, start, close) {
        args.push(range);
    }
    args
}

fn trimmed_range(source: &str, start: usize, end: usize) -> Option<std::ops::Range<usize>> {
    if start >= end {
        return None;
    }
    let slice = &source[start..end];
    let left = slice.find(|ch: char| !ch.is_whitespace())?;
    let right_start = slice.rfind(|ch: char| !ch.is_whitespace())?;
    let right = right_start
        + slice[right_start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    Some(start + left..start + right)
}

fn record_scene_condition_surface_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) {
    let Some(first) = tokens.first() else {
        return;
    };
    if first.text == "if" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Keyword);
    }
}

fn scene_block_keyword(value: &str) -> bool {
    matches!(
        value,
        "keys" | "on_scene_start" | "resources" | "rules" | "state" | "transitions" | "layout"
    )
}

fn record_rewrite_surface_line(
    scope: Option<SourceScope>,
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) {
    if let Some(arrow) = tokens.iter().position(|token| token.text == "->") {
        let rhs = &tokens[arrow + 1..];
        let effect_start = rhs
            .iter()
            .rposition(|token| token.text.contains(']'))
            .map_or(0, |index| index + 1);
        if effect_start < rhs.len() {
            sink.extend(rewrite_effect_surface_document(&rhs[effect_start..]));
        }
        return;
    }

    if scope == Some(SourceScope::Other) {
        sink.extend(rewrite_effect_surface_document(tokens));
    }
}

pub fn export_loaded_document_visual_fixture_json(
    document: &LoadedDocument,
) -> Result<String, DiagnosticReport> {
    let Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) = document.single_model() else {
        return Err(DiagnosticReport::error(
            "visual fixture export currently requires a single puzzle3 model".to_string(),
        ));
    };
    let (scene_fields, level_bundle_names) = puzzle3_scene_fixture_fields(document);
    export_visual_fixture_json_with_title_scenes_and_animation(
        puzzle,
        Some(&document.title),
        scene_fields.as_deref(),
        &level_bundle_names,
        VisualFixtureAnimation3 {
            tween_enabled: document.animation.tween.enabled,
            tween_interval_ms: document.animation.tween.interval_ms,
        },
    )
    .map_err(|error| {
        DiagnosticReport::error(format!("failed to export puzzle3 fixture: {error:?}"))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_game_file(path: impl AsRef<Path>) -> Result<LoadedDocument, DiagnosticReport> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })?;
    let profile = puzzle_source_profile_for_path(path).ok_or_else(|| {
        DiagnosticReport::error(format!(
            "game entry must be a .puzzle or .puzzle3 file: {}",
            path.display()
        ))
    })?;
    validate_source_profile(&source, profile)?;
    if profile == PuzzleSourceProfile::Puzzle3d {
        return parse_game_document(&source);
    }
    let expanded = expand_game_imports_for_file(&source, path)?;
    validate_source_profile(&expanded, profile)?;
    parse_game_document(&expanded)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_game2d_file(path: impl AsRef<Path>) -> Result<LoadedGame, DiagnosticReport> {
    let path = path.as_ref();
    if puzzle_source_profile_for_path(path) != Some(PuzzleSourceProfile::Puzzle2d) {
        return Err(DiagnosticReport::error(format!(
            "2D game entry must be a .puzzle file: {}",
            path.display()
        )));
    }
    let source = fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })?;
    let expanded = expand_game_imports_for_file(&source, path)?;
    parse_game2d_document(&expanded)
}

fn parse_game2d_document(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let parts = parse_document_source_parts(source)?;
    parse_game2d_from_document_parts(parts)
}

fn parse_game2d_from_document_parts(
    parts: DocumentSourceParts,
) -> Result<LoadedGame, DiagnosticReport> {
    let mut scenes = parts.scenes;
    let model_names = all_model_names(&parts.model_source, "puzzle");
    resolve_inferred_scene_puzzle_slots(
        &mut scenes,
        model_names.iter().map(|name| ("puzzle", name)),
    )?;
    let mut game =
        parse_game2d_expanded_with_shell(&parts.model_source_without_shell, &parts.shell)?;
    resolve_default_wait_in_scenes(&mut scenes, game.default_wait_ms);
    game.scenes =
        add_implicit_model_scenes(scenes, model_names.iter().map(|name| ("puzzle", name)));
    Ok(game)
}

fn parse_game_document(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    match detect_game_document_kind(source)? {
        GameDocumentKind::Puzzle2d => {
            let parts = parse_document_source_parts(source)?;
            let name = first_model_name(&parts.model_source, "puzzle")
                .unwrap_or_else(|| "default".to_string());
            let shell = parts.shell.clone();
            let game = parse_game2d_from_document_parts(parts)?;
            Ok(LoadedDocument {
                title: shell.title,
                subtitle: shell.subtitle,
                author: shell.author,
                homepage: shell.homepage,
                default_wait_ms: shell.default_wait_ms,
                default_again_ms: shell.default_again_ms,
                animation: shell.animation,
                sounds: shell.sounds,
                theme: shell.theme,
                assets: shell.assets,
                scenes: game.scenes.clone(),
                models: vec![LoadedDocumentModel::Puzzle2d {
                    name,
                    game: game.clone(),
                }],
            })
        }
        GameDocumentKind::Puzzle3d => {
            let parts = parse_document_source_parts(source)?;
            let name = first_model_name(&parts.raw_model_source_without_shell, "puzzle3")
                .unwrap_or_else(|| "default".to_string());
            let mut scenes = parts.scenes;
            resolve_inferred_scene_puzzle_slots(&mut scenes, std::iter::once(("puzzle3", &name)))?;
            let puzzle = parse_puzzle3d(&parts.raw_model_source_without_shell).map_err(
                |error| match error {
                    ParseError3::Message(message) => DiagnosticReport::error(message),
                },
            )?;
            let scenes = add_implicit_model_scenes(scenes, std::iter::once(("puzzle3", &name)));
            Ok(LoadedDocument {
                title: parts.shell.title,
                subtitle: parts.shell.subtitle,
                author: parts.shell.author,
                homepage: parts.shell.homepage,
                default_wait_ms: parts.shell.default_wait_ms,
                default_again_ms: parts.shell.default_again_ms,
                animation: parts.shell.animation,
                sounds: parts.shell.sounds,
                theme: parts.shell.theme,
                assets: parts.shell.assets,
                scenes,
                models: vec![LoadedDocumentModel::Puzzle3d { name, puzzle }],
            })
        }
        GameDocumentKind::Mixed => parse_mixed_game_document(source),
    }
}

fn parse_mixed_game_document(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    let parts = parse_document_source_parts(source)?;
    let sources = split_mixed_game_document_source(source)?;
    let model_2d_name =
        first_model_name(&sources.puzzle2d, "puzzle").unwrap_or_else(|| "default".to_string());
    let game_2d_source = strip_document_shell_source(&sources.puzzle2d)?;
    let game_2d = parse_game2d_expanded_with_shell(&game_2d_source, &parts.shell)?;
    let model_3d_name =
        first_model_name(&sources.puzzle3d, "puzzle3").unwrap_or_else(|| "default".to_string());
    let puzzle_3d_source = strip_document_shell_source_raw(&sources.puzzle3d);
    let puzzle_3d = parse_puzzle3d(&puzzle_3d_source).map_err(|error| match error {
        ParseError3::Message(message) => DiagnosticReport::error(message),
    })?;
    let mut scenes = parts.scenes;
    resolve_inferred_scene_puzzle_slots(
        &mut scenes,
        [("puzzle", &model_2d_name), ("puzzle3", &model_3d_name)].into_iter(),
    )?;

    Ok(LoadedDocument {
        title: parts.shell.title,
        subtitle: parts.shell.subtitle,
        author: parts.shell.author,
        homepage: parts.shell.homepage,
        default_wait_ms: parts.shell.default_wait_ms,
        default_again_ms: parts.shell.default_again_ms,
        animation: parts.shell.animation,
        sounds: parts.shell.sounds,
        theme: parts.shell.theme,
        assets: parts.shell.assets,
        scenes: add_implicit_model_scenes(
            scenes,
            [("puzzle", &model_2d_name), ("puzzle3", &model_3d_name)].into_iter(),
        ),
        models: vec![
            LoadedDocumentModel::Puzzle2d {
                name: model_2d_name,
                game: game_2d,
            },
            LoadedDocumentModel::Puzzle3d {
                name: model_3d_name,
                puzzle: puzzle_3d,
            },
        ],
    })
}

#[derive(Default)]
struct MixedDocumentSources {
    puzzle2d: String,
    puzzle3d: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixedSectionTarget {
    Puzzle2d,
    Puzzle3d,
    Shared,
}

fn split_mixed_game_document_source(
    source: &str,
) -> Result<MixedDocumentSources, DiagnosticReport> {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut sources = MixedDocumentSources::default();
    let mut index = 0usize;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if trimmed.is_empty() {
            push_raw_line(&mut sources.puzzle2d, line);
            push_raw_line(&mut sources.puzzle3d, line);
            index += 1;
            continue;
        }

        let tokens = split_header_tokens(trimmed);
        let target = match tokens.as_slice() {
            ["title", ..]
            | ["subtitle", ..]
            | ["author", ..]
            | ["homepage", ..]
            | ["default_wait_time", ..]
            | ["again_interval", ..]
            | ["animation", ..]
            | ["sounds", ..]
            | ["theme", ..]
            | ["assets", ..] => MixedSectionTarget::Shared,
            ["puzzle", ..] | ["levels", ..] | ["sprites", ..] | ["level", ..] => {
                MixedSectionTarget::Puzzle2d
            }
            ["puzzle3", ..] | ["levels3", ..] | ["sprites3", ..] => MixedSectionTarget::Puzzle3d,
            ["scene", ..] => {
                let next = skip_raw_top_level_block(&raw_lines, index);
                index = next;
                continue;
            }
            ["var", ..] | ["const", ..] | ["persistent", ..] => MixedSectionTarget::Puzzle2d,
            _ => MixedSectionTarget::Puzzle2d,
        };
        let is_block = mixed_section_is_block(trimmed);
        let next = if is_block {
            skip_raw_top_level_block(&raw_lines, index)
        } else {
            index + 1
        };
        if is_block && matches!(tokens.as_slice(), ["puzzle", ..] | ["puzzle3", ..]) {
            push_raw_model_block_without_default_scene_layouts(
                &raw_lines,
                index,
                target,
                &mut sources,
            );
        } else {
            push_raw_block(&raw_lines, index, next, target, &mut sources);
        }
        index = next;
    }
    Ok(sources)
}

fn push_raw_model_block_without_default_scene_layouts(
    raw_lines: &[&str],
    start: usize,
    target: MixedSectionTarget,
    sources: &mut MixedDocumentSources,
) {
    let mut stripped = Vec::new();
    push_raw_model_without_default_scene_layouts(raw_lines, start, &mut stripped);
    for line in stripped {
        match target {
            MixedSectionTarget::Puzzle2d => push_raw_line(&mut sources.puzzle2d, line),
            MixedSectionTarget::Puzzle3d => push_raw_line(&mut sources.puzzle3d, line),
            MixedSectionTarget::Shared => {
                push_raw_line(&mut sources.puzzle2d, line);
                push_raw_line(&mut sources.puzzle3d, line);
            }
        }
    }
}

fn mixed_section_is_block(trimmed: &str) -> bool {
    trimmed.ends_with('{')
}

fn push_raw_block(
    raw_lines: &[&str],
    start: usize,
    end: usize,
    target: MixedSectionTarget,
    sources: &mut MixedDocumentSources,
) {
    for line in &raw_lines[start..end] {
        match target {
            MixedSectionTarget::Puzzle2d => push_raw_line(&mut sources.puzzle2d, line),
            MixedSectionTarget::Puzzle3d => push_raw_line(&mut sources.puzzle3d, line),
            MixedSectionTarget::Shared => {
                push_raw_line(&mut sources.puzzle2d, line);
                push_raw_line(&mut sources.puzzle3d, line);
            }
        }
    }
}

fn push_raw_line(target: &mut String, line: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(line);
}

fn add_implicit_model_scenes<'a>(
    mut scenes: Vec<SceneDef>,
    models: impl IntoIterator<Item = (&'a str, &'a String)>,
) -> Vec<SceneDef> {
    let mut existing = scenes
        .iter()
        .map(|scene| scene.name.clone())
        .collect::<HashSet<_>>();
    for (kind, model_name) in models {
        if existing.contains(model_name) {
            continue;
        }
        scenes.push(implicit_model_scene(kind, model_name));
        existing.insert(model_name.clone());
    }
    scenes
}

const INFERRED_SCENE_PUZZLE_KIND: &str = "__model__";

fn resolve_inferred_scene_puzzle_slots<'a>(
    scenes: &mut [SceneDef],
    models: impl IntoIterator<Item = (&'a str, &'a String)>,
) -> Result<(), DiagnosticReport> {
    let mut model_kinds = HashMap::<String, String>::new();
    let mut ambiguous = HashSet::<String>::new();
    for (kind, model_name) in models {
        match model_kinds.get(model_name) {
            Some(existing) if existing != kind => {
                ambiguous.insert(model_name.clone());
            }
            Some(_) => {}
            None => {
                model_kinds.insert(model_name.clone(), kind.to_string());
            }
        }
    }

    for scene in scenes {
        for puzzle in &mut scene.state.puzzles {
            if puzzle.kind != INFERRED_SCENE_PUZZLE_KIND {
                continue;
            }
            if ambiguous.contains(&puzzle.model) {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` is ambiguous; use `puzzle <name>` or `puzzle3 <name>`",
                    puzzle.model
                )));
            }
            let Some(kind) = model_kinds.get(&puzzle.model) else {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` does not match a puzzle model",
                    puzzle.model
                )));
            };
            puzzle.kind = kind.clone();
        }
    }

    Ok(())
}

fn implicit_model_scene(kind: &str, model_name: &str) -> SceneDef {
    SceneDef {
        name: model_name.to_string(),
        layout: SceneLayoutDef::default(),
        resources: SceneResources::default(),
        state: SceneStateDef {
            variables: Vec::new(),
            puzzles: vec![ScenePuzzleDef {
                name: model_name.to_string(),
                kind: kind.to_string(),
                model: model_name.to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime: SceneStateLifetime::Instance,
            }],
        },
        components: vec![scene_frame_component(kind, model_name)],
        key_bindings: Vec::new(),
        routines: Vec::new(),
        transitions: Vec::new(),
        puzzle_rule: Some(ScenePuzzleRule {
            target: model_name.to_string(),
            rule: "rules".to_string(),
        }),
    }
}

fn puzzle3_scene_fixture_fields(document: &LoadedDocument) -> (Option<String>, Vec<String>) {
    if document.scenes.is_empty() {
        return (None, Vec::new());
    }
    let mut level_bundle_names = Vec::new();
    let current_scene = document
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .or_else(|| document.scenes.first())
        .map(|scene| scene.name.as_str())
        .unwrap_or("playing");
    let mut out = String::new();
    out.push_str("  \"currentScene\": ");
    out.push_str(&json_string(current_scene));
    out.push_str(",\n  \"scenes\": [\n");
    for (index, scene) in document.scenes.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        push_puzzle3_scene_json(&mut out, scene, &mut level_bundle_names);
    }
    out.push_str("\n  ],");
    (Some(out), level_bundle_names)
}

fn push_puzzle3_scene_json(
    out: &mut String,
    scene: &SceneDef,
    level_bundle_names: &mut Vec<String>,
) {
    out.push_str("    {\n");
    out.push_str("      \"name\": ");
    out.push_str(&json_string(&scene.name));
    out.push_str(",\n      \"layout\": ");
    push_puzzle3_layout_json(out, &scene.layout);
    out.push_str(",\n      \"puzzles\": [");
    let mut wrote_puzzle = false;
    for puzzle in &scene.state.puzzles {
        if puzzle.kind != "puzzle3" {
            continue;
        }
        if wrote_puzzle {
            out.push_str(", ");
        }
        wrote_puzzle = true;
        out.push_str("{ \"slot\": ");
        out.push_str(&json_string(&puzzle.name));
        out.push_str(", \"model\": ");
        out.push_str(&json_string(&puzzle.model));
        out.push_str(" }");
    }
    out.push_str("],\n      \"keys\": {");
    let mut wrote_key = false;
    for binding in &scene.key_bindings {
        let Some(action) = puzzle3_scene_action_json(&binding.effect, level_bundle_names) else {
            continue;
        };
        for key in &binding.keys {
            if wrote_key {
                out.push_str(", ");
            }
            wrote_key = true;
            out.push_str(&json_string(&key_trigger_name(key)));
            out.push_str(": ");
            out.push_str(&action);
        }
    }
    out.push_str("},\n      \"components\": [");
    let mut wrote_component = false;
    for component in &scene.components {
        if let Some(component_json) = puzzle3_scene_component_json(component, level_bundle_names) {
            if wrote_component {
                out.push_str(", ");
            }
            wrote_component = true;
            out.push_str(&component_json);
        }
    }
    out.push_str("]\n    }");
}

fn puzzle3_scene_component_json(
    component: &SceneComponent,
    level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    match component {
        SceneComponent::Frame(frame) if frame.kind == "puzzle3" => {
            let mut out = format!(
                "{{ \"kind\": \"puzzle3\", \"source\": {}",
                json_string(&frame.source)
            );
            push_puzzle3_inline_layout_json(&mut out, &frame.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Title(title) => {
            let mut out = format!(
                "{{ \"kind\": \"title\", \"text\": {}",
                json_string(&scene_expr_fixture_text(&title.content))
            );
            push_puzzle3_inline_layout_json(&mut out, &title.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            let action = puzzle3_scene_action_json(&button.effect, level_bundle_names)?;
            let kind = match component {
                SceneComponent::Choice(_) => "choice",
                _ => "button",
            };
            let mut out = format!(
                "{{ \"kind\": {}, \"label\": {}, \"action\": {}",
                json_string(kind),
                puzzle3_scene_expr_json(&button.label),
                action
            );
            push_puzzle3_inline_layout_json(&mut out, &button.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::LevelMenu(menu) => {
            let levels = menu.source.as_deref().unwrap_or("levels");
            push_unique_string(level_bundle_names, levels);
            let action = menu
                .action
                .as_ref()
                .and_then(|effect| puzzle3_scene_action_json(effect, level_bundle_names))
                .unwrap_or_else(|| {
                    "{ \"kind\": \"goto\", \"scene\": \"playing\", \"params\": [{ \"kind\": \"level\", \"value\": { \"kind\": \"path\", \"path\": \"level\" } }] }".to_string()
                });
            let mut out = format!(
                "{{ \"kind\": \"level_menu\", \"levels\": {}, \"action\": {}",
                json_string(levels),
                action
            );
            push_puzzle3_inline_layout_json(&mut out, &menu.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Row(container) => puzzle3_container_json(
            "row",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Column(container) => puzzle3_container_json(
            "column",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Box(container) => puzzle3_container_json(
            "box",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Conditional(conditional) => {
            let mut out = format!(
                "{{ \"kind\": \"conditional\", \"condition\": {}, \"children\": [",
                json_string(&conditional.condition)
            );
            let mut wrote = false;
            for child in &conditional.children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("], \"elseChildren\": [");
            wrote = false;
            for child in &conditional.else_children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("] }");
            Some(out)
        }
        SceneComponent::For(for_view) => {
            let mut out = format!(
                "{{ \"kind\": \"for\", \"binding\": {}, \"source\": {}, \"children\": [",
                json_string(&for_view.binding),
                json_string(for_view.source.as_str())
            );
            let mut wrote = false;
            for child in &for_view.children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("] }");
            Some(out)
        }
        _ => None,
    }
}

fn puzzle3_container_json(
    kind: &str,
    children: &[SceneComponent],
    layout: &SceneLayoutDef,
    level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    let mut out = format!("{{ \"kind\": {}, \"children\": [", json_string(kind));
    let mut wrote = false;
    for child in children {
        if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
            if wrote {
                out.push_str(", ");
            }
            wrote = true;
            out.push_str(&child_json);
        }
    }
    out.push(']');
    push_puzzle3_inline_layout_json(&mut out, layout);
    out.push_str(" }");
    Some(out)
}

fn puzzle3_scene_action_json(
    effect: &SceneEffect,
    _level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    match effect {
        SceneEffect::Goto { scene, params } => {
            let mut out = format!("{{ \"kind\": \"goto\", \"scene\": {}", json_string(scene));
            if !params.is_empty() {
                out.push_str(", \"params\": [");
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    match param {
                        SceneEffectParam::Level(value) => {
                            out.push_str("{ \"kind\": \"level\", \"value\": ");
                            out.push_str(&puzzle3_scene_expr_json(value));
                            out.push_str(" }");
                        }
                        SceneEffectParam::Named { name, value } => {
                            out.push_str("{ \"kind\": \"named\", \"name\": ");
                            out.push_str(&json_string(name));
                            out.push_str(", \"value\": ");
                            out.push_str(&puzzle3_scene_expr_json(value));
                            out.push_str(" }");
                        }
                    }
                }
                out.push(']');
            }
            out.push_str(" }");
            Some(out)
        }
        _ => None,
    }
}

fn puzzle3_scene_expr_json(expr: &SceneExpr) -> String {
    match expr {
        SceneExpr::Bool(value) => format!("{{ \"kind\": \"bool\", \"value\": {value} }}"),
        SceneExpr::Int(value) => format!("{{ \"kind\": \"int\", \"value\": {value} }}"),
        SceneExpr::Text(value) => format!(
            "{{ \"kind\": \"text\", \"value\": {} }}",
            json_string(value)
        ),
        SceneExpr::Path(path) => {
            format!(
                "{{ \"kind\": \"path\", \"path\": {} }}",
                json_string(&path.join("."))
            )
        }
        SceneExpr::Call { name, args } => {
            let mut out = format!(
                "{{ \"kind\": \"call\", \"name\": {}, \"args\": [",
                json_string(name)
            );
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&puzzle3_scene_expr_json(arg));
            }
            out.push_str("] }");
            out
        }
    }
}

fn scene_expr_fixture_text(expr: &SceneExpr) -> String {
    match expr {
        SceneExpr::Text(value) => value.clone(),
        SceneExpr::Path(path) => path.join("."),
        SceneExpr::Int(value) => value.to_string(),
        SceneExpr::Bool(value) => value.to_string(),
        SceneExpr::Call { name, .. } => name.clone(),
    }
}

fn push_puzzle3_inline_layout_json(out: &mut String, layout: &SceneLayoutDef) {
    if layout.size.is_none()
        && layout.gap.is_none()
        && layout.align == SceneLayoutDef::default().align
        && !layout.scroll
    {
        return;
    }
    out.push_str(", \"layout\": ");
    push_puzzle3_layout_json(out, layout);
}

fn push_puzzle3_layout_json(out: &mut String, layout: &SceneLayoutDef) {
    out.push('{');
    let mut wrote = false;
    if let Some(size) = layout.size {
        out.push_str("\"size\": { \"width\": ");
        out.push_str(&size.width.to_string());
        out.push_str(", \"height\": ");
        out.push_str(&size.height.to_string());
        out.push_str(" }");
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"gap\": ");
        out.push_str(&gap.to_string());
        wrote = true;
    }
    if layout.align != SceneLayoutDef::default().align {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"align\": { \"x\": ");
        out.push_str(&json_string(match layout.align.x {
            SceneAlignXDef::Left => "left",
            SceneAlignXDef::Center => "center",
            SceneAlignXDef::Right => "right",
        }));
        out.push_str(", \"y\": ");
        out.push_str(&json_string(match layout.align.y {
            SceneAlignYDef::Top => "top",
            SceneAlignYDef::Center => "center",
            SceneAlignYDef::Bottom => "bottom",
        }));
        out.push_str(" }");
        wrote = true;
    }
    if layout.scroll {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"scroll\": true");
    }
    out.push('}');
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}

#[derive(Clone, Debug)]
struct DocumentShell {
    title: String,
    subtitle: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    default_wait_ms: u64,
    default_again_ms: u64,
    animation: AnimationDef,
    sounds: SoundsDef,
    theme: ThemeDef,
    assets: AssetsDef,
}

#[derive(Clone, Debug)]
struct DocumentSourceParts {
    shell: DocumentShell,
    model_source: String,
    model_source_without_shell: String,
    raw_model_source_without_shell: String,
    scenes: Vec<SceneDef>,
}

impl Default for DocumentShell {
    fn default() -> Self {
        Self {
            title: "ASCII play".to_string(),
            subtitle: None,
            author: None,
            homepage: None,
            default_wait_ms: DEFAULT_WAIT_MS,
            default_again_ms: DEFAULT_AGAIN_MS,
            animation: AnimationDef::default(),
            sounds: SoundsDef::default(),
            theme: ThemeDef::default(),
            assets: AssetsDef::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelTopLevelDirective {
    Puzzle,
    RemovedModelPrefix,
    RemovedNameMetadata,
    Title,
    Subtitle,
    Author,
    Homepage,
    Variable,
    DefaultWaitTime,
    AgainInterval,
    Animation,
    Scene,
    Sounds,
    Theme,
    Assets,
    Close,
    Sprites,
    Levels,
    Level,
    PuzzleLifecycle,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelTopLevelExpectedGroup {
    Metadata,
    Variables,
    Model,
    Content,
    Config,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HeaderChoiceAlternative<Action, ExpectedGroup> {
    trigger: &'static str,
    label: &'static str,
    action: Action,
    expected_group: Option<ExpectedGroup>,
}

const MODEL_TOP_LEVEL_ALTERNATIVES: &[HeaderChoiceAlternative<
    ModelTopLevelDirective,
    ModelTopLevelExpectedGroup,
>] = &[
    HeaderChoiceAlternative {
        trigger: "puzzle",
        label: "puzzle",
        action: ModelTopLevelDirective::Puzzle,
        expected_group: Some(ModelTopLevelExpectedGroup::Model),
    },
    HeaderChoiceAlternative {
        trigger: "model",
        label: "model",
        action: ModelTopLevelDirective::RemovedModelPrefix,
        expected_group: None,
    },
    HeaderChoiceAlternative {
        trigger: "name",
        label: "name",
        action: ModelTopLevelDirective::RemovedNameMetadata,
        expected_group: None,
    },
    HeaderChoiceAlternative {
        trigger: "title",
        label: "title",
        action: ModelTopLevelDirective::Title,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
    },
    HeaderChoiceAlternative {
        trigger: "subtitle",
        label: "subtitle",
        action: ModelTopLevelDirective::Subtitle,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
    },
    HeaderChoiceAlternative {
        trigger: "author",
        label: "author",
        action: ModelTopLevelDirective::Author,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
    },
    HeaderChoiceAlternative {
        trigger: "homepage",
        label: "homepage",
        action: ModelTopLevelDirective::Homepage,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
    },
    HeaderChoiceAlternative {
        trigger: "var",
        label: "var",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
    },
    HeaderChoiceAlternative {
        trigger: "const",
        label: "const",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
    },
    HeaderChoiceAlternative {
        trigger: "persistent",
        label: "persistent var",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
    },
    HeaderChoiceAlternative {
        trigger: "default_wait_time",
        label: "default_wait_time",
        action: ModelTopLevelDirective::DefaultWaitTime,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
    },
    HeaderChoiceAlternative {
        trigger: "again_interval",
        label: "again_interval",
        action: ModelTopLevelDirective::AgainInterval,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
    },
    HeaderChoiceAlternative {
        trigger: "animation",
        label: "animation",
        action: ModelTopLevelDirective::Animation,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
    },
    HeaderChoiceAlternative {
        trigger: "scene",
        label: "scene",
        action: ModelTopLevelDirective::Scene,
        expected_group: None,
    },
    HeaderChoiceAlternative {
        trigger: "sounds",
        label: "sounds",
        action: ModelTopLevelDirective::Sounds,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
    HeaderChoiceAlternative {
        trigger: "theme",
        label: "theme",
        action: ModelTopLevelDirective::Theme,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
    HeaderChoiceAlternative {
        trigger: "assets",
        label: "assets",
        action: ModelTopLevelDirective::Assets,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
    HeaderChoiceAlternative {
        trigger: BLOCK_CLOSE,
        label: BLOCK_CLOSE,
        action: ModelTopLevelDirective::Close,
        expected_group: None,
    },
    HeaderChoiceAlternative {
        trigger: "sprites",
        label: "sprites",
        action: ModelTopLevelDirective::Sprites,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
    HeaderChoiceAlternative {
        trigger: "levels",
        label: "levels",
        action: ModelTopLevelDirective::Levels,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
    HeaderChoiceAlternative {
        trigger: "level",
        label: "level",
        action: ModelTopLevelDirective::Level,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
    },
];

fn classify_header_choice<Action: Copy, ExpectedGroup>(
    alternatives: &[HeaderChoiceAlternative<Action, ExpectedGroup>],
    trigger: &str,
) -> Option<Action> {
    alternatives
        .iter()
        .find(|alternative| alternative.trigger == trigger)
        .map(|alternative| alternative.action)
}

fn format_header_choice_expected_group<Action, ExpectedGroup: Copy + PartialEq>(
    alternatives: &[HeaderChoiceAlternative<Action, ExpectedGroup>],
    group: ExpectedGroup,
) -> String {
    alternatives
        .iter()
        .filter(|alternative| alternative.expected_group == Some(group))
        .map(|alternative| format!("`{}`", alternative.label))
        .collect::<Vec<_>>()
        .join(", ")
}

fn classify_model_top_level_directive(tokens: &[&str]) -> ModelTopLevelDirective {
    let Some(first) = tokens.first().copied() else {
        return ModelTopLevelDirective::Unknown;
    };
    if puzzle_lifecycle_event(first).is_some() {
        return ModelTopLevelDirective::PuzzleLifecycle;
    }
    classify_header_choice(MODEL_TOP_LEVEL_ALTERNATIVES, first)
        .unwrap_or(ModelTopLevelDirective::Unknown)
}

fn format_model_top_level_expected_group(group: ModelTopLevelExpectedGroup) -> String {
    format_header_choice_expected_group(MODEL_TOP_LEVEL_ALTERNATIVES, group)
}

fn model_top_level_expected_directives_message() -> String {
    format!(
        "metadata ({}), variables ({}), a model ({}), content ({}), or config ({})",
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Metadata),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Variables),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Model),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Content),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Config),
    )
}

fn unknown_model_top_level_directive_message(other: &str) -> String {
    format!(
        "unknown top-level directive `{other}`; expected {}",
        model_top_level_expected_directives_message()
    )
}

fn misplaced_puzzle_lifecycle_message(lifecycle_block: &str) -> String {
    format!(
        "{lifecycle_block} is a puzzle lifecycle block; put it inside `puzzle <name> {{ ... }}` next to `rules {{ ... }}`"
    )
}

fn parse_document_source_parts(source: &str) -> Result<DocumentSourceParts, DiagnosticReport> {
    let shell = parse_document_shell(source)?;
    let (model_source, scenes) = split_document_scene_source(source)?;
    let shell_stripped_source = strip_document_shell_source(source)?;
    let (model_source_without_shell, _) = split_document_scene_source(&shell_stripped_source)?;
    let raw_model_source_without_shell =
        strip_document_scene_source_raw(&strip_document_shell_source_raw(source));
    Ok(DocumentSourceParts {
        shell,
        model_source,
        model_source_without_shell,
        raw_model_source_without_shell,
        scenes,
    })
}

fn parse_document_shell(source: &str) -> Result<DocumentShell, DiagnosticReport> {
    let mut shell = DocumentShell::default();
    let lines = logical_lines(source)?;
    let mut index = 0;
    while index < lines.len() {
        let tokens = split_header_tokens(&lines[index]);
        match tokens.as_slice() {
            ["title", ..] => {
                shell.title = parse_metadata_text(&lines[index], "title")?;
                index += 1;
            }
            ["subtitle", ..] => {
                shell.subtitle = Some(parse_metadata_text(&lines[index], "subtitle")?);
                index += 1;
            }
            ["author", ..] => {
                shell.author = Some(parse_metadata_text(&lines[index], "author")?);
                index += 1;
            }
            ["homepage", ..] => {
                shell.homepage = Some(parse_metadata_text(&lines[index], "homepage")?);
                index += 1;
            }
            ["default_wait_time", ..] => {
                shell.default_wait_ms = parse_default_wait_time_directive(&tokens, &lines[index])?;
                index += 1;
            }
            ["again_interval", ..] => {
                shell.default_again_ms = parse_again_interval_directive(&tokens, &lines[index])?;
                index += 1;
            }
            ["animation", ..] => {
                index = parse_animation_block(&lines, index, &mut shell.animation)?;
            }
            ["sounds"] => {
                if model_sounds_block_starts(&lines, index) {
                    index = skip_logical_block(&lines, index);
                } else {
                    index = parse_sounds_block(&lines, index, &mut shell.sounds)?;
                }
            }
            ["theme", name] if next_line_is_not_block_body(&lines, index) => {
                parse_theme_name_directive(&lines[index], name, &mut shell.theme)?;
                index += 1;
            }
            ["theme"] | ["theme", ..] => {
                index = parse_theme_statement(&lines, index, &mut shell.theme)?;
            }
            ["assets"] => {
                index = parse_assets_block(&lines, index, &mut shell.assets)?;
            }
            _ if logical_line_opens_block(tokens.as_slice()) => {
                index = skip_logical_block(&lines, index);
            }
            _ => {
                index += 1;
            }
        }
    }
    Ok(shell)
}

fn strip_document_shell_source(source: &str) -> Result<String, DiagnosticReport> {
    let context = scan_source_context(source);
    let mut out = Vec::new();
    let mut index = 0;
    let mut shell_prefix = true;
    while index < context.lines.len() {
        let line = &context.lines[index];
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        if shell_prefix && line.scope.is_none() {
            match tokens.as_slice() {
                ["title", ..]
                | ["subtitle", ..]
                | ["author", ..]
                | ["homepage", ..]
                | ["default_wait_time", ..]
                | ["again_interval", ..] => {
                    index += 1;
                    continue;
                }
                ["animation", ..] | ["sounds", ..] | ["assets", ..] => {
                    index = skip_context_shell_block_by_syntax(&context, index);
                    continue;
                }
                ["theme", ..] => {
                    index = if context_theme_line_is_block(&context, index) {
                        skip_context_shell_block_by_syntax(&context, index)
                    } else {
                        index + 1
                    };
                    continue;
                }
                _ => {}
            }
            if !matches!(
                tokens.as_slice(),
                [] | ["var", ..]
                    | ["const", ..]
                    | ["persistent", "var", ..]
                    | ["title", ..]
                    | ["subtitle", ..]
                    | ["author", ..]
                    | ["homepage", ..]
                    | ["default_wait_time", ..]
                    | ["again_interval", ..]
                    | ["animation", ..]
                    | ["sounds", ..]
                    | ["assets", ..]
                    | ["theme", ..]
            ) {
                shell_prefix = false;
            }
        }

        out.push(line.content.clone());
        index += 1;
    }
    Ok(out.join("\n"))
}

fn skip_context_shell_block_by_syntax(context: &source::SourceContext, index: usize) -> usize {
    let trimmed = strip_line_comment(&context.lines[index].content).trim();
    let mut next = index + 1;
    let mut brace_depth = raw_brace_delta(trimmed);
    if brace_depth > 0 {
        while next < context.lines.len() && brace_depth > 0 {
            let trimmed = strip_line_comment(&context.lines[next].content).trim();
            brace_depth += raw_brace_delta(trimmed);
            next += 1;
        }
        return next;
    }

    while next < context.lines.len() {
        let trimmed = strip_line_comment(&context.lines[next].content).trim();
        next += 1;
        if trimmed == BLOCK_CLOSE {
            break;
        }
    }
    next
}

fn context_theme_line_is_block(context: &source::SourceContext, index: usize) -> bool {
    let trimmed = strip_line_comment(&context.lines[index].content).trim();
    if raw_brace_delta(trimmed) > 0 {
        return true;
    }
    context.lines.get(index + 1).is_some_and(|next| {
        let trimmed = strip_line_comment(&next.content).trim();
        trimmed == BLOCK_CLOSE || is_theme_setting_line(trimmed)
    })
}

fn strip_document_shell_source_raw(source: &str) -> String {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    let mut brace_depth = 0i32;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if brace_depth == 0 {
            let tokens = split_header_tokens(trimmed);
            match tokens.as_slice() {
                ["title", ..]
                | ["subtitle", ..]
                | ["author", ..]
                | ["homepage", ..]
                | ["default_wait_time", ..]
                | ["again_interval", ..] => {
                    index += 1;
                    continue;
                }
                ["animation"] | ["animation", ..] | ["sounds"] | ["assets"] => {
                    index = skip_raw_top_level_block(&raw_lines, index);
                    continue;
                }
                ["theme", _] if !trimmed.ends_with('{') => {
                    index += 1;
                    continue;
                }
                ["theme"] | ["theme", ..] => {
                    index = skip_raw_top_level_block(&raw_lines, index);
                    continue;
                }
                _ => {}
            }
        }
        out.push(line);
        brace_depth += raw_brace_delta(trimmed);
        brace_depth = brace_depth.max(0);
        index += 1;
    }
    out.join("\n")
}

fn next_line_is_not_block_body(lines: &[String], index: usize) -> bool {
    let Some(next) = lines.get(index + 1) else {
        return true;
    };
    if is_block_close_line(next) {
        return true;
    }
    let tokens = split_header_tokens(next);
    logical_line_starts_document_boundary(tokens.as_slice())
}

fn logical_line_starts_document_boundary(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["title", ..]
            | ["subtitle", ..]
            | ["author", ..]
            | ["homepage", ..]
            | ["default_wait_time", ..]
            | ["again_interval", ..]
            | ["puzzle", ..]
            | ["puzzle3", ..]
            | ["levels", ..]
            | ["levels3", ..]
            | ["sprites", ..]
            | ["sprites3", ..]
            | ["scene", ..]
            | ["sounds"]
            | ["theme", ..]
            | ["assets"]
    )
}

fn logical_line_opens_block(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["puzzle", ..]
            | ["puzzle3", ..]
            | ["levels", ..]
            | ["levels3", ..]
            | ["sprites", ..]
            | ["sprites3", ..]
            | ["scene", ..]
            | ["state", ..]
            | ["layout", ..]
            | ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["layers", ..]
            | ["tags", ..]
            | ["map", ..]
            | ["scratch", ..]
            | ["groups", ..]
            | ["legend", ..]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["routine", ..]
            | ["rules", ..]
            | ["on_display", ..]
            | ["on_level_start", ..]
            | ["on_level_clear", ..]
            | ["on_last_level_clear", ..]
            | ["keys", ..]
            | ["inputs", ..]
            | ["transitions", ..]
            | ["on_scene_start", ..]
            | ["if", ..]
            | ["sounds"]
            | ["theme", ..]
            | ["assets"]
    )
}

fn skip_logical_block(lines: &[String], start: usize) -> usize {
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < lines.len() {
        let tokens = split_header_tokens(&lines[index]);
        if is_block_close_line(&lines[index]) {
            depth = depth.saturating_sub(1);
            index += 1;
            if depth == 0 {
                break;
            }
            continue;
        }
        if logical_line_opens_block(tokens.as_slice()) && !logical_line_is_inline_if(&lines[index])
        {
            depth += 1;
        }
        index += 1;
    }
    index
}

fn recover_after_directive_error(lines: &[String], index: usize) -> usize {
    let tokens = split_header_tokens(&lines[index]);
    if logical_line_opens_block(tokens.as_slice()) && !logical_line_is_inline_if(&lines[index]) {
        skip_logical_block(lines, index)
    } else {
        index + 1
    }
}

fn logical_line_is_inline_if(line: &str) -> bool {
    split_header_tokens(line).first().copied() == Some("if") && line.contains("->")
}

fn strip_document_scene_source_raw(source: &str) -> String {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    let mut brace_depth = 0i32;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if brace_depth == 0 {
            let tokens = split_header_tokens(trimmed);
            if matches!(tokens.as_slice(), ["scene", ..]) {
                index = skip_raw_top_level_block(&raw_lines, index);
                continue;
            }
            if matches!(tokens.as_slice(), ["puzzle", ..] | ["puzzle3", ..])
                && trimmed.ends_with('{')
            {
                index = push_raw_model_without_default_scene_layouts(&raw_lines, index, &mut out);
                continue;
            }
        }
        out.push(line);
        brace_depth += raw_brace_delta(trimmed);
        brace_depth = brace_depth.max(0);
        index += 1;
    }
    out.join("\n")
}

fn push_raw_model_without_default_scene_layouts<'a>(
    raw_lines: &[&'a str],
    start: usize,
    out: &mut Vec<&'a str>,
) -> usize {
    out.push(raw_lines[start]);
    let mut index = start + 1;
    let mut depth = raw_brace_delta(strip_line_comment(raw_lines[start]).trim());
    while index < raw_lines.len() && depth > 0 {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if depth == 1 && matches!(split_header_tokens(trimmed).as_slice(), ["layout", ..]) {
            index = skip_raw_top_level_block(raw_lines, index);
            continue;
        }
        out.push(line);
        depth += raw_brace_delta(trimmed);
        index += 1;
    }
    index
}

fn split_document_scene_source(source: &str) -> Result<(String, Vec<SceneDef>), DiagnosticReport> {
    let lines = logical_lines(source)?;
    let mut model_lines = Vec::new();
    let mut scenes = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["scene", ..]) {
            let (scene, next_i) = parse_scene_definition(&lines, i)?;
            scenes.push(scene);
            i = next_i;
        } else if let Some((kind, name)) = model_header_name(tokens.as_slice()) {
            let (entry, default_scene, next_i) =
                extract_default_model_scene(&lines, i, kind, name)?;
            model_lines.extend(entry);
            if let Some(scene) = default_scene {
                scenes.push(scene);
            }
            i = next_i;
        } else {
            model_lines.push(lines[i].clone());
            i += 1;
        }
    }
    Ok((model_lines.join("\n"), scenes))
}

fn model_header_name<'a>(tokens: &'a [&'a str]) -> Option<(&'a str, &'a str)> {
    match tokens {
        ["puzzle", name, ..] | ["puzzle3", name, ..] => Some((tokens[0], *name)),
        _ => None,
    }
}

fn extract_default_model_scene(
    lines: &[String],
    start: usize,
    kind: &str,
    name: &str,
) -> Result<(Vec<String>, Option<SceneDef>, usize), DiagnosticReport> {
    let mut entry = vec![lines[start].clone()];
    let mut default_scene = None;
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) || line == "}" {
            depth = depth.saturating_sub(1);
            entry.push(line.clone());
            i += 1;
            if depth == 0 {
                return Ok((entry, default_scene, i));
            }
            continue;
        }
        if depth == 1 && matches!(tokens.as_slice(), ["layout", ..]) {
            if default_scene.is_some() {
                return Err(parse_error(
                    line,
                    "model default scene has duplicate layout block",
                ));
            }
            let next_i = skip_scene_layout_block(lines, i)?;
            default_scene = Some(parse_default_model_scene(lines, i, next_i, kind, name)?);
            i = next_i;
            continue;
        }
        if starts_model_nested_block(tokens.as_slice(), line) {
            depth += 1;
        }
        entry.push(line.clone());
        i += 1;
    }
    Ok((vec![lines[start].clone()], None, start + 1))
}

fn skip_scene_layout_block(lines: &[String], start: usize) -> Result<usize, DiagnosticReport> {
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            depth = depth.saturating_sub(1);
            i += 1;
            if depth == 0 {
                return Ok(i);
            }
            continue;
        }
        if starts_authoring_block(tokens.as_slice(), line) || line.trim_end().ends_with("->") {
            depth += 1;
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "layout missing closing brace"))
}

fn starts_model_nested_block(tokens: &[&str], line: &str) -> bool {
    logical_line_opens_block(tokens) && !logical_line_is_inline_if(line)
}

fn parse_default_model_scene(
    lines: &[String],
    start: usize,
    end: usize,
    kind: &str,
    name: &str,
) -> Result<SceneDef, DiagnosticReport> {
    let mut layout_lines = lines[start..end].to_vec();
    rewrite_default_model_layout_components(&mut layout_lines, kind, name);
    let (layout_block, next_i) = parse_screen_layout_block(&layout_lines, 0)?;
    debug_assert_eq!(next_i, layout_lines.len());
    let mut scene = implicit_model_scene(kind, name);
    scene.layout = layout_block.layout;
    scene.state.variables.extend(layout_block.state.variables);
    scene.state.puzzles.extend(layout_block.state.puzzles);
    scene.components = layout_block.components;
    Ok(scene)
}

fn rewrite_default_model_layout_components(lines: &mut [String], kind: &str, name: &str) {
    for line in lines {
        if split_header_tokens(line).as_slice() == [kind] {
            *line = format!("{kind} {name}");
        }
    }
}

fn skip_raw_top_level_block(raw_lines: &[&str], start: usize) -> usize {
    let first = strip_line_comment(raw_lines[start]).trim();
    if first.ends_with('{') {
        let mut depth = raw_brace_delta(first);
        let mut index = start + 1;
        while index < raw_lines.len() && depth > 0 {
            let trimmed = strip_line_comment(raw_lines[index]).trim();
            depth += raw_brace_delta(trimmed);
            index += 1;
        }
        index
    } else {
        let mut index = start + 1;
        while index < raw_lines.len() {
            let trimmed = strip_line_comment(raw_lines[index]).trim();
            index += 1;
            if trimmed == BLOCK_CLOSE {
                break;
            }
        }
        index
    }
}

fn raw_brace_delta(line: &str) -> i32 {
    line.chars().filter(|ch| *ch == '{').count() as i32
        - line.chars().filter(|ch| *ch == '}').count() as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameDocumentKind {
    Puzzle2d,
    Puzzle3d,
    Mixed,
}

fn detect_game_document_kind(source: &str) -> Result<GameDocumentKind, DiagnosticReport> {
    let mut has_2d = false;
    let mut has_3d = false;
    let context = scan_source_context(source);
    for line in &context.lines {
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        match (line.scope, tokens.as_slice()) {
            (None, ["puzzle", ..]) => has_2d = true,
            (None, ["puzzle3", ..]) => has_3d = true,
            (_, ["levels3", ..] | ["sprites3", ..]) => has_3d = true,
            _ => {}
        }
    }
    Ok(match (has_2d, has_3d) {
        (true, true) => GameDocumentKind::Mixed,
        (false, true) => GameDocumentKind::Puzzle3d,
        _ => GameDocumentKind::Puzzle2d,
    })
}

pub fn validate_source_profile_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<(), DiagnosticReport> {
    let path = path.as_ref();
    let profile = puzzle_source_profile_for_path(path).ok_or_else(|| {
        DiagnosticReport::error(format!(
            "puzzle source must use .puzzle or .puzzle3 extension: {}",
            path.display()
        ))
    })?;
    validate_source_profile(source, profile)
}

fn validate_source_profile(
    source: &str,
    profile: PuzzleSourceProfile,
) -> Result<(), DiagnosticReport> {
    let kind = detect_game_document_kind(source)?;
    match (profile, kind) {
        (PuzzleSourceProfile::Puzzle2d, GameDocumentKind::Puzzle2d)
        | (PuzzleSourceProfile::Puzzle3d, GameDocumentKind::Puzzle3d) => Ok(()),
        (_, GameDocumentKind::Mixed) => Err(DiagnosticReport::error(
            "mixed 2D/3D documents are no longer supported; split 2D .puzzle and 3D .puzzle3 sources"
                .to_string(),
        )),
        (PuzzleSourceProfile::Puzzle2d, GameDocumentKind::Puzzle3d) => Err(DiagnosticReport::error(
            ".puzzle files cannot contain 3D puzzle3, levels3, or sprites3 sections; use .puzzle3"
                .to_string(),
        )),
        (PuzzleSourceProfile::Puzzle3d, GameDocumentKind::Puzzle2d) => Err(DiagnosticReport::error(
            ".puzzle3 files must contain 3D puzzle3, levels3, or sprites3 sections".to_string(),
        )),
    }
}

fn first_model_name(source: &str, kind: &str) -> Option<String> {
    all_model_names(source, kind).into_iter().next()
}

fn all_model_names(source: &str, kind: &str) -> Vec<String> {
    let context = scan_source_context(source);
    let mut names = Vec::new();
    for line in &context.lines {
        if line.scope.is_some() {
            continue;
        }
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        if let [model_kind, name, ..] = tokens.as_slice()
            && *model_kind == kind
            && !names.iter().any(|existing| existing == name)
        {
            names.push((*name).to_string());
        }
    }
    names
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_game_entry(path: impl AsRef<Path>) -> Result<PathBuf, DiagnosticReport> {
    let path = path.as_ref();
    if path.is_dir() {
        if let Some(entry) = game_entry_in_directory(path)? {
            return Ok(entry);
        }
        return Err(DiagnosticReport::error(format!(
            "game folder must contain a .puzzle or .puzzle3 file with game prelude metadata such as title: {}",
            path.display()
        )));
    }

    if path.is_file() {
        if !is_puzzle_source_path(path) {
            return Err(DiagnosticReport::error(format!(
                "game entry must be a folder, .puzzle file, or .puzzle3 file: {}",
                path.display()
            )));
        }
        let source = fs::read_to_string(path).map_err(|error| {
            DiagnosticReport::error(format!(
                "failed to read game entry {}: {error}",
                path.display()
            ))
        })?;
        if source_has_game_prelude(&source) {
            return Ok(path.to_path_buf());
        }
        let mut dir = path.parent();
        while let Some(current) = dir {
            if let Some(entry) = game_entry_in_directory(current)? {
                return Ok(entry);
            }
            dir = current.parent();
        }
        return Err(DiagnosticReport::error(format!(
            "puzzle source file has no game prelude and no containing game entry was found: {}",
            path.display()
        )));
    }

    Err(DiagnosticReport::error(format!(
        "game entry not found: {}",
        path.display()
    )))
}

pub fn source_has_game_prelude(source: &str) -> bool {
    let mut depth = 0_i32;
    for raw_line in source.lines() {
        let code = raw_line.split("//").next().unwrap_or("");
        let trimmed = code.trim();
        if depth == 0 {
            let first = trimmed.split_whitespace().next().unwrap_or("");
            if matches!(first, "title" | "subtitle" | "author" | "homepage") {
                return true;
            }
        }
        for ch in code.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth = (depth - 1).max(0),
                _ => {}
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn game_entry_in_directory(dir: &Path) -> Result<Option<PathBuf>, DiagnosticReport> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| {
        DiagnosticReport::error(format!(
            "failed to read game entry directory {}: {error}",
            dir.display()
        ))
    })? {
        let path = entry
            .map_err(|error| {
                DiagnosticReport::error(format!("failed to read game entry: {error}"))
            })?
            .path();
        if !is_puzzle_source_path(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|error| {
            DiagnosticReport::error(format!(
                "failed to read game entry candidate {}: {error}",
                path.display()
            ))
        })?;
        if source_has_game_prelude(&source) {
            candidates.push(path);
        }
    }
    candidates.sort_by(|left, right| {
        let left_rank = game_entry_path_rank(left, dir);
        let right_rank = game_entry_path_rank(right, dir);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
    Ok(candidates.into_iter().next())
}

#[cfg(not(target_arch = "wasm32"))]
fn game_entry_path_rank(path: &Path, dir: &Path) -> usize {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let folder_name = dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if name == "game.puzzle" {
        0
    } else if name == "game.puzzle3" {
        1
    } else if !folder_name.is_empty() && name == format!("{folder_name}.puzzle") {
        2
    } else if !folder_name.is_empty() && name == format!("{folder_name}.puzzle3") {
        3
    } else if name == "main.puzzle" {
        4
    } else if name == "main.puzzle3" {
        5
    } else {
        6
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn discover_game_entries(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, DiagnosticReport> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(root.as_ref()).map_err(|error| {
        DiagnosticReport::error(format!(
            "failed to read game root {}: {error}",
            root.as_ref().display()
        ))
    })? {
        let path = entry
            .map_err(|error| {
                DiagnosticReport::error(format!("failed to read game entry: {error}"))
            })?
            .path();
        if path.is_dir() {
            if let Some(entry) = game_entry_in_directory(&path)? {
                candidates.push(entry);
            }
        } else if is_puzzle_source_path(&path) {
            let source = fs::read_to_string(&path).map_err(|error| {
                DiagnosticReport::error(format!(
                    "failed to read game entry candidate {}: {error}",
                    path.display()
                ))
            })?;
            if source_has_game_prelude(&source) {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    Ok(candidates)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expand_game_imports_for_file(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<String, DiagnosticReport> {
    let path = path.as_ref();
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut import_stack = vec![canonical_import_path(path)];
    expand_game_imports(source, base_dir, &mut import_stack, None)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expand_game_imports_for_file_under_root(
    source: &str,
    path: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> Result<String, DiagnosticReport> {
    let path = path.as_ref();
    let root = canonical_import_path(root.as_ref());
    let canonical_path = canonical_import_path(path);
    if !canonical_path.starts_with(&root) {
        return Err(DiagnosticReport::error(format!(
            "can only import puzzle files under {}",
            root.display()
        )));
    }
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut import_stack = vec![canonical_path];
    expand_game_imports(source, base_dir, &mut import_stack, Some(&root))
}

fn parse_game2d_expanded_with_shell(
    source: &str,
    shell: &DocumentShell,
) -> Result<LoadedGame, DiagnosticReport> {
    let lines = logical_lines(source)?;
    let mut title = shell.title.clone();
    let mut subtitle = shell.subtitle.clone();
    let mut author = shell.author.clone();
    let mut homepage = shell.homepage.clone();
    let mut layer_count = None;
    let mut empty_char = None;
    let mut named_layers = HashMap::<String, u16>::new();
    let mut catalog = Catalog::default();
    let mut condition_definitions = Vec::<ConditionDefinitionAst>::new();
    let mut controls = Controls::default();
    let mut directions = Vec::<Direction>::new();
    let mut rule_definitions = Vec::<RuleDefinitionAst>::new();
    let mut main_statements = None;
    let mut main_local_frame = None;
    let mut level_start_statements = None;
    let mut level_start_local_frame = None;
    let mut level_clear_statements = None;
    let mut level_clear_local_frame = None;
    let mut last_level_clear_statements = None;
    let mut last_level_clear_local_frame = None;
    let mut display_statements = None;
    let mut level_blocks = Vec::<LevelBlock>::new();
    let mut puzzle_models = Vec::<String>::new();
    let mut variables = Vec::<SceneVarDef>::new();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut model_sound_triggers = Vec::<ModelSoundTriggerSpec>::new();
    let mut named_conditions = HashMap::<String, (String, ConditionAst)>::new();
    let mut run_rules_on_level_start = false;
    let mut visuals = VisualsDef::default();
    let mut render = PuzzleRenderDef::default();
    let mut animation = shell.animation.clone();
    let mut sounds = shell.sounds.clone();
    let mut theme = shell.theme.clone();
    let mut assets = shell.assets.clone();
    let mut puzzle_screen = PuzzleScreenDef::default();
    let mut default_wait_ms = shell.default_wait_ms;
    let mut default_again_ms = shell.default_again_ms;

    let mut diagnostics = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }

        match classify_model_top_level_directive(tokens.as_slice()) {
            ModelTopLevelDirective::Puzzle => match parse_puzzle_definition(
                &lines,
                i,
                &mut layer_count,
                &mut empty_char,
                &mut named_layers,
                &mut catalog,
                &mut condition_definitions,
                &mut controls,
                &mut directions,
                &mut rule_definitions,
                &mut main_statements,
                &mut main_local_frame,
                &mut level_start_statements,
                &mut level_start_local_frame,
                &mut level_clear_statements,
                &mut level_clear_local_frame,
                &mut last_level_clear_statements,
                &mut last_level_clear_local_frame,
                &mut display_statements,
                &mut level_blocks,
                &mut render_overlays,
                &mut model_sound_triggers,
                &mut named_conditions,
                &mut run_rules_on_level_start,
                &mut visuals,
                &mut render,
                &mut animation,
                &mut puzzle_screen,
            ) {
                Ok((next_i, puzzle_name)) => {
                    puzzle_models.push(puzzle_name);
                    i = next_i;
                }
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    i = recover_after_directive_error(&lines, i);
                }
            },
            ModelTopLevelDirective::RemovedModelPrefix => {
                let message = match tokens.as_slice() {
                    ["model", "puzzle3", ..] => {
                        "top-level 3D puzzle definition must be: puzzle3 <name>"
                    }
                    _ => "top-level puzzle definition must be: puzzle <name>",
                };
                diagnostics.extend(parse_error(line, message).into_diagnostics());
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::RemovedNameMetadata => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "top-level `name` metadata was removed; use `title <text>`",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            ModelTopLevelDirective::Title => {
                title = parse_metadata_text(line, "title")?;
                i += 1;
            }
            ModelTopLevelDirective::Subtitle => {
                subtitle = Some(parse_metadata_text(line, "subtitle")?);
                i += 1;
            }
            ModelTopLevelDirective::Author => {
                author = Some(parse_metadata_text(line, "author")?);
                i += 1;
            }
            ModelTopLevelDirective::Homepage => {
                homepage = Some(parse_metadata_text(line, "homepage")?);
                i += 1;
            }
            ModelTopLevelDirective::Variable => {
                variables.push(parse_top_level_var_directive(&tokens, line)?);
                i += 1;
            }
            ModelTopLevelDirective::DefaultWaitTime => {
                default_wait_ms = parse_default_wait_time_directive(&tokens, line)?;
                i += 1;
            }
            ModelTopLevelDirective::AgainInterval => {
                default_again_ms = parse_again_interval_directive(&tokens, line)?;
                i += 1;
            }
            ModelTopLevelDirective::Animation => {
                i = parse_animation_block(&lines, i, &mut animation)?;
            }
            ModelTopLevelDirective::Scene => {
                diagnostics.extend(parse_error(
                    line,
                    "scene blocks are document-level syntax and must be parsed before the 2D model",
                ).into_diagnostics());
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::Sounds => {
                if model_sounds_block_starts(&lines, i) {
                    i = parse_model_sounds_block(&lines, i, &mut model_sound_triggers)?;
                } else {
                    i = parse_sounds_block(&lines, i, &mut sounds)?;
                }
            }
            ModelTopLevelDirective::Theme => {
                i = parse_theme_statement(&lines, i, &mut theme)?;
            }
            ModelTopLevelDirective::Assets => {
                i = parse_assets_block(&lines, i, &mut assets)?;
            }
            ModelTopLevelDirective::Close => {
                i += 1;
            }
            ModelTopLevelDirective::Sprites => {
                i = parse_visuals_block(&lines, i, &mut catalog, &mut visuals)?;
            }
            ModelTopLevelDirective::Levels => {
                i = parse_levels_block(
                    &lines,
                    i,
                    &mut level_blocks,
                    &mut catalog,
                    &mut render_overlays,
                    &mut empty_char,
                    None,
                )?;
            }
            ModelTopLevelDirective::Level => {
                let (level, next_i) = parse_level_block(&lines, i, level_blocks.len())?;
                level_blocks.push(level);
                i = next_i;
            }
            ModelTopLevelDirective::PuzzleLifecycle => {
                diagnostics.extend(
                    parse_error(line, &misplaced_puzzle_lifecycle_message(tokens[0]))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::Unknown => {
                diagnostics.extend(
                    parse_error(line, &unknown_model_top_level_directive_message(tokens[0]))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(&lines, i);
            }
        }
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    let empty_char = empty_char.ok_or_else(|| {
        DiagnosticReport::error(
            "missing empty char; use `levels { legend { . = empty } }`".to_string(),
        )
    })?;
    validate_layer_role_separation(&catalog, &named_layers)?;
    refresh_layer_tags_and_value_sets(&mut named_layers, &mut catalog);
    let layer_count =
        layer_count.ok_or_else(|| DiagnosticReport::error("missing layers".to_string()))?;
    if level_blocks.is_empty() {
        return Err(DiagnosticReport::error("missing level".to_string()));
    }
    resolve_level_block_puzzles(&mut level_blocks, &puzzle_models)?;
    let prepared_level_bodies = level_blocks
        .into_iter()
        .map(|level| {
            let puzzle = level
                .puzzle
                .clone()
                .expect("level puzzle was resolved before preparation");
            let body = parse_level_body(
                &level,
                &catalog,
                empty_char,
                default_wait_ms,
                &named_conditions,
            )?;
            let mut char_objects = catalog.char_objects.clone();
            char_objects.extend(body.local_char_objects.clone());
            Ok(PreparedLevelBody {
                name: level.name,
                pack: level.pack,
                puzzle,
                lines: body.lines,
                char_objects,
                level_start_statements: body.level_start_statements,
                level_clear_statements: body.level_clear_statements,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    add_default_restart_handler(main_statements.as_mut());
    add_implicit_input_guards_to_catalog(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        display_statements.as_deref(),
        &prepared_level_bodies,
        &named_conditions,
        &mut catalog,
    )?;
    if directions.is_empty()
        || (has_cardinal_input_names(&catalog.input_names)
            && !directions_include_all_cardinals(&directions, &catalog.input_names))
    {
        add_cardinal_directions("default inputs", &mut catalog, &mut directions)?;
    }
    add_default_non_direction_inputs("default inputs", &mut catalog)?;
    add_default_key_controls(&catalog.input_names, &mut controls);
    let effective_directions = if directions.is_empty() {
        default_cardinal_directions(&catalog.input_names)
    } else {
        directions.clone()
    };
    let value_sets = catalog_value_sets(&catalog);
    let visual_condition_reads =
        visual_condition_reads(&condition_definitions, &catalog.visual_objects);
    let condition_defs = lower_condition_defs(
        condition_definitions,
        &catalog.object_layers,
        &catalog.scratch_names,
        &value_sets,
        &catalog.input_names,
        &effective_directions,
    )?;
    let mut conditions = named_conditions
        .into_iter()
        .map(|(name, (description, condition))| {
            lower_goal_condition(
                description,
                &condition,
                &catalog.object_layers,
                &catalog.global_names,
                &catalog.condition_names,
                &visual_condition_reads,
                &catalog.scratch_names,
                &catalog.visual_objects,
                &value_sets,
                &catalog.input_names,
                &effective_directions,
            )
            .map(|condition| (name, condition))
        })
        .collect::<Result<HashMap<_, _>, DiagnosticReport>>()?;
    let goal = conditions
        .get("win_conditions")
        .or_else(|| conditions.get("goal"))
        .cloned();
    let lose = conditions
        .get("lose_conditions")
        .or_else(|| conditions.get("lose"))
        .cloned();
    conditions.remove("lose_conditions");
    conditions.remove("lose");
    if run_rules_on_level_start && level_start_statements.is_some() {
        return Err(DiagnosticReport::error(
            "run_rules_on_level_start cannot be combined with on_level_start".to_string(),
        ));
    }
    add_standard_move_rule_if_missing(
        &mut rule_definitions,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_layers,
        &catalog.visual_objects,
        &value_sets,
        &catalog.maps,
        &catalog.object_groups,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.condition_names,
    )?;
    let visual_objects = catalog.visual_objects.clone();
    let model_sound_triggers = resolve_model_sound_triggers(&model_sound_triggers, &catalog)?;
    let mut warnings = collect_dynamic_selector_warnings(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        last_level_clear_statements.as_deref(),
        display_statements.as_deref(),
        &prepared_level_bodies,
        &catalog.constant_globals,
    );
    let programs = lower_programs(
        rule_definitions,
        main_statements,
        main_local_frame,
        level_start_statements,
        level_start_local_frame,
        level_clear_statements,
        level_clear_local_frame,
        last_level_clear_statements,
        last_level_clear_local_frame,
        display_statements,
        &prepared_level_bodies,
        &catalog.object_layers,
        &visual_objects,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.constant_globals,
        &catalog.condition_names,
        &visual_condition_reads,
        &catalog.scratch_names,
        &model_sound_triggers,
        &animation,
        &value_sets,
        &effective_directions,
        default_wait_ms,
    )?;
    let game = CompiledGame::new_with_scratch_condition_defs_program_roles(
        layer_count,
        catalog.object_defs,
        catalog.scratch_defs,
        condition_defs,
        programs.main,
        visual_objects.clone(),
        programs.visual_rules.clone(),
    );
    let mut legend = AsciiLegend::new(game.object_count(), empty_char);
    for (object, ch) in &catalog.render_chars {
        legend.set(*object, *ch);
    }
    for object in &visual_objects {
        legend.ignore(*object);
    }
    for (objects, ch) in render_overlays {
        legend.add_overlay(objects, ch);
    }
    let levels = prepared_level_bodies
        .into_iter()
        .enumerate()
        .map(|(index, prepared)| {
            let parsed_level = parse_level(
                &game,
                &prepared.lines,
                empty_char,
                &prepared.char_objects,
                &catalog.global_defaults,
            )?;
            Ok(Level {
                name: prepared.name,
                pack: prepared.pack,
                puzzle: prepared.puzzle,
                initial_state: parsed_level.state,
                regions: parsed_level.regions,
                level_start_program: programs.level_starts[index].clone(),
                level_clear_program: programs.level_clears[index].clone(),
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    warnings.extend(collect_scratch_warnings(&game, &catalog.scratch_names));

    Ok(LoadedGame {
        title,
        subtitle,
        author,
        homepage,
        game,
        warnings,
        default_wait_ms,
        default_again_ms,
        animation: animation.clone(),
        rule_animations: programs.rule_animations,
        rule_effects: programs.rule_effects,
        level_start_program: programs.level_start,
        display_level_start_program: None,
        level_clear_program: programs.level_clear,
        last_level_clear_program: programs.last_level_clear,
        display_level_clear_program: None,
        display_program: programs.display,
        levels,
        run_rules_on_level_start,
        legend,
        controls,
        variables,
        scenes: Vec::new(),
        object_labels: catalog.object_labels,
        object_groups: catalog.object_groups,
        input_labels: catalog.input_labels,
        global_labels: catalog.global_labels,
        persistent_vars: catalog.persistent_vars,
        condition_labels: catalog.condition_labels,
        conditions,
        goal,
        lose,
        sounds,
        theme,
        assets,
        visuals,
        render,
        screen: puzzle_screen,
    })
}

fn collect_dynamic_selector_warnings(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
    display_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
    constant_globals: &[GlobalId],
) -> Vec<String> {
    let mut warnings = Vec::new();
    for definition in definitions {
        collect_dynamic_selector_statement_warnings(
            &definition.statements,
            constant_globals,
            &mut warnings,
        );
    }
    for statements in [
        main_statements,
        level_start_statements,
        level_clear_statements,
        last_level_clear_statements,
        display_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_dynamic_selector_statement_warnings(statements, constant_globals, &mut warnings);
    }
    for body in level_bodies {
        collect_dynamic_selector_statement_warnings(
            &body.level_start_statements,
            constant_globals,
            &mut warnings,
        );
        collect_dynamic_selector_statement_warnings(
            &body.level_clear_statements,
            constant_globals,
            &mut warnings,
        );
    }
    warnings
}

fn collect_dynamic_selector_statement_warnings(
    statements: &[StatementAst],
    constant_globals: &[GlobalId],
    warnings: &mut Vec<String>,
) {
    for statement in statements {
        match statement {
            StatementAst::Rewrite(rewrite) | StatementAst::DisplayRewrite(rewrite) => {
                collect_dynamic_selector_block_warnings(
                    &rewrite.before,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_block_warnings(&rewrite.after, constant_globals, warnings);
            }
            StatementAst::Conditional {
                condition,
                then_statements,
                else_statements,
            } => {
                collect_dynamic_selector_block_warnings(
                    &condition.pattern,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_globals,
                    warnings,
                );
            }
            StatementAst::Block { statements, .. }
            | StatementAst::DisplayBlock(statements)
            | StatementAst::Fix { statements, .. }
            | StatementAst::RepeatUntil { statements, .. } => {
                collect_dynamic_selector_statement_warnings(statements, constant_globals, warnings);
            }
            StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_globals,
                    warnings,
                );
            }
            StatementAst::Call { .. }
            | StatementAst::DisplayCall { .. }
            | StatementAst::Effect { .. } => {}
        }
    }
}

fn collect_dynamic_selector_block_warnings(
    block: &PatternBlock,
    constant_globals: &[GlobalId],
    warnings: &mut Vec<String>,
) {
    for component in &block.components {
        for row in &component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                for selector in cell.require.iter().chain(&cell.forbid) {
                    for guard in selector.dynamic_guards.values().flatten() {
                        if constant_globals.contains(&guard.global) {
                            continue;
                        }
                        push_unique_warning(
                            warnings,
                            format!(
                                "dynamic selector `{}` uses mutable var `{}`; if the var is outside the selector tag slot values, the selector does not match",
                                selector.token, guard.name
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn collect_scratch_warnings(
    game: &CompiledGame,
    scratch_names: &HashMap<String, ScratchDef>,
) -> Vec<String> {
    let labels = scratch_names
        .iter()
        .map(|(name, def)| (def.id, name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut warnings = Vec::new();
    for rule in game.rules() {
        for component in &rule.pattern.components {
            for cell in &component.cells {
                for cell_attr in cell
                    .require_scratch
                    .iter()
                    .filter(|attr| attr.object.is_empty())
                {
                    for object_attr in cell
                        .require_scratch
                        .iter()
                        .filter(|attr| !attr.object.is_empty())
                    {
                        if cell_attr.scratch == object_attr.scratch {
                            push_unique_warning(
                                &mut warnings,
                                format!(
                                    "scratch `{}` appears on both a cell and an object occurrence in the same cell pattern",
                                    scratch_label(cell_attr.scratch, &labels)
                                ),
                            );
                        }
                    }
                }
            }
        }

        for pattern_attr in rule
            .pattern
            .components
            .iter()
            .flat_map(|component| component.cells.iter())
            .flat_map(|cell| cell.require_scratch.iter())
        {
            for write in &rule.writes {
                let Some((write_object, write_scratch)) = write_scratch_target(write) else {
                    continue;
                };
                if pattern_attr.scratch != write_scratch {
                    continue;
                }
                if pattern_attr.object.is_empty() != write_object.is_empty() {
                    let from = if pattern_attr.object.is_empty() {
                        "cell"
                    } else {
                        "object occurrence"
                    };
                    let to = if write_object.is_empty() {
                        "cell"
                    } else {
                        "object occurrence"
                    };
                    push_unique_warning(
                        &mut warnings,
                        format!(
                            "scratch `{}` changes anchor from {from} to {to} in a rewrite",
                            scratch_label(pattern_attr.scratch, &labels)
                        ),
                    );
                }
            }
        }
    }
    warnings
}

fn write_scratch_target(write: &WriteOp) -> Option<(ObjectId, ScratchId)> {
    match write {
        WriteOp::SetScratch {
            object, scratch, ..
        } => Some((*object, *scratch)),
        _ => None,
    }
}

fn scratch_label<'a>(scratch: ScratchId, labels: &HashMap<ScratchId, &'a str>) -> String {
    labels
        .get(&scratch)
        .copied()
        .unwrap_or("__anonymous")
        .to_string()
}

fn push_unique_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn expand_game_imports(
    source: &str,
    base_dir: &Path,
    import_stack: &mut Vec<PathBuf>,
    root: Option<&Path>,
) -> Result<String, DiagnosticReport> {
    let mut out = String::new();
    for line in logical_lines(source)? {
        let tokens = split_header_tokens(&line);
        if matches!(tokens.as_slice(), ["import", _]) {
            let path = import_path(tokens[1], &line)?;
            let imported = read_import_expanded(base_dir, &path, import_stack, root)?;
            out.push_str(&imported);
            if !imported.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_import_expanded(
    base_dir: &Path,
    path: &Path,
    import_stack: &mut Vec<PathBuf>,
    root: Option<&Path>,
) -> Result<String, DiagnosticReport> {
    let resolved = resolve_import_path(base_dir, path);
    let canonical = canonical_import_path(&resolved);
    if let Some(root) = root {
        if !canonical.starts_with(root) {
            return Err(DiagnosticReport::error(format!(
                "can only import puzzle files under {}",
                root.display()
            )));
        }
    }
    if import_stack.contains(&canonical) {
        return Err(DiagnosticReport::error(format!(
            "cyclic import: {}",
            import_stack
                .iter()
                .chain(std::iter::once(&canonical))
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        )));
    }
    let source = match read_import_path(&resolved) {
        Ok(source) => source,
        Err(error) => return Err(error),
    };
    let nested_base = resolved.parent().unwrap_or(base_dir);
    import_stack.push(canonical);
    let expanded = expand_game_imports(&source, nested_base, import_stack, root);
    import_stack.pop();
    expanded
}

#[cfg(not(target_arch = "wasm32"))]
fn import_path(token: &str, line: &str) -> Result<PathBuf, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "import path must be quoted"))?;
    Ok(PathBuf::from(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_import_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn canonical_import_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_import_path(path: &Path) -> Result<String, DiagnosticReport> {
    fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })
}

fn scene_entry_is_component(tokens: &[&str]) -> bool {
    let Some(kind) = tokens
        .first()
        .and_then(|keyword| puzzle_scene::SceneComponentKind::from_keyword(keyword))
    else {
        return false;
    };
    match kind {
        puzzle_scene::SceneComponentKind::Button
        | puzzle_scene::SceneComponentKind::Choice
        | puzzle_scene::SceneComponentKind::Text
        | puzzle_scene::SceneComponentKind::Title
        | puzzle_scene::SceneComponentKind::Subtitle
        | puzzle_scene::SceneComponentKind::Row
        | puzzle_scene::SceneComponentKind::Column
        | puzzle_scene::SceneComponentKind::Box
        | puzzle_scene::SceneComponentKind::Conditional
        | puzzle_scene::SceneComponentKind::For => true,
        puzzle_scene::SceneComponentKind::LevelMenu => true,
        puzzle_scene::SceneComponentKind::Frame => tokens.len() >= 2,
    }
}

fn collect_authoring_entry(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let first = &lines[start];
    let tokens = split_header_tokens(first);
    if matches!(tokens.as_slice(), ["levels", ..]) {
        return collect_levels_authoring_entry(lines, start);
    }
    if !starts_authoring_block(&tokens, first) {
        return Ok((vec![first.clone()], start + 1));
    }

    let mut entry = Vec::new();
    let mut block_stack = vec![authoring_block_kind(&tokens)];
    let mut i = start;
    while i < lines.len() {
        let line = &lines[i];
        if i != start {
            let tokens = split_header_tokens(line);
            if tokens.first().copied() == Some(BLOCK_CLOSE) {
                let closed = block_stack
                    .pop()
                    .ok_or_else(|| parse_error(line, "closing brace without block"))?;
                entry.push(line.clone());
                i += 1;
                if block_stack.is_empty() {
                    return Ok((entry, i));
                }
                if closed == AuthoringBlockKind::If && next_line_is_else(lines, i) {
                    entry.push(lines[i].clone());
                    i += 1;
                    block_stack.push(AuthoringBlockKind::Other);
                }
                continue;
            }
            if let Some(kind) = authoring_nested_block_kind(&tokens, line) {
                block_stack.push(kind);
            }
        }
        entry.push(line.clone());
        i += 1;
    }
    Err(parse_error(first, "block missing closing brace"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthoringBlockKind {
    If,
    Other,
}

fn authoring_block_kind(tokens: &[&str]) -> AuthoringBlockKind {
    if tokens.first().copied() == Some("if") {
        AuthoringBlockKind::If
    } else {
        AuthoringBlockKind::Other
    }
}

fn authoring_nested_block_kind(tokens: &[&str], line: &str) -> Option<AuthoringBlockKind> {
    if starts_authoring_block(tokens, line) {
        Some(authoring_block_kind(tokens))
    } else if line.trim_end().ends_with("->") {
        Some(AuthoringBlockKind::Other)
    } else {
        None
    }
}

fn next_line_is_else(lines: &[String], index: usize) -> bool {
    lines
        .get(index)
        .is_some_and(|line| matches!(split_header_tokens(line).as_slice(), ["else"]))
}

fn collect_levels_authoring_entry(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let first = &lines[start];
    let mut entry = vec![first.clone()];
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            depth -= 1;
            entry.push(line.clone());
            if depth == 0 {
                return Ok((entry, i + 1));
            }
            i += 1;
            continue;
        }
        if depth == 1 && (is_braced_level_header(line) || matches!(tokens.as_slice(), ["{"])) {
            depth += 1;
        } else if !matches!(tokens.as_slice(), ["level", ..])
            && starts_authoring_block(&tokens, line)
        {
            depth += 1;
        }
        entry.push(line.clone());
        i += 1;
    }
    Err(parse_error(first, "levels block missing closing brace"))
}

fn starts_authoring_block(tokens: &[&str], line: &str) -> bool {
    match tokens {
        ["map", ..]
        | ["on_level_start"]
        | ["on_level_clear"]
        | ["on_last_level_clear"]
        | ["on_display"]
        | ["scratch"]
        | ["groups"]
        | ["layers"]
        | ["win_conditions", ..]
        | ["lose_conditions", ..]
        | ["sprites"]
        | ["sounds"]
        | ["screen"]
        | ["layout", ..]
        | ["routine", ..]
        | ["rules"]
        | ["levels", ..]
        | ["resources"]
        | ["level", ..]
        | ["state"]
        | ["keys"]
        | ["on_scene_start"]
        | ["input", ..]
        | ["action", ..]
        | ["if", ..]
        | ["row", ..]
        | ["column", ..]
        | ["box", ..]
        | ["for", ..]
        | ["level_menu"] => true,
        ["legend"] => true,
        ["button", ..] if line.trim_end().ends_with(" with") => true,
        ["choice", ..] if line.trim_end().ends_with(" with") => true,
        _ => false,
    }
}

fn parse_sounds_block(
    lines: &[String],
    start: usize,
    sounds: &mut SoundsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(&lines[start], "sounds header must be: sounds"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["sfx", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "sfx sounds name")?;
                if sounds.sfx.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate sfx sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                let type_target = optional_sound_setting(settings, "type").unwrap_or("random");
                validate_sound_atom(seed, line, "sfx seed")?;
                validate_sound_atom(type_target, line, "sfx type")?;
                sounds.sfx.push(SfxSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    type_target: type_target.to_string(),
                });
            }
            ["music", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "music sounds name")?;
                if sounds.music.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate music sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                validate_sound_atom(seed, line, "music seed")?;
                let height = parse_sound_f64(
                    optional_sound_setting(settings, "height")
                        .or_else(|| optional_sound_setting(settings, "tone"))
                        .unwrap_or("0.5"),
                    line,
                    "height",
                )?;
                let bars = parse_sound_u16(
                    optional_sound_setting(settings, "bars").unwrap_or("8"),
                    line,
                    "bars",
                )?;
                let bpm = parse_sound_u16(
                    optional_sound_setting(settings, "bpm").unwrap_or("110"),
                    line,
                    "bpm",
                )?;
                let volume = parse_sound_f64(
                    optional_sound_setting(settings, "volume").unwrap_or("0.5"),
                    line,
                    "volume",
                )?;
                if !(0.0..=1.0).contains(&height) {
                    return Err(parse_error(line, "music height must be between 0 and 1"));
                }
                if !matches!(bars, 8 | 16 | 32 | 64) {
                    return Err(parse_error(
                        line,
                        "music bars must be one of 8, 16, 32, or 64",
                    ));
                }
                if !(40..=180).contains(&bpm) {
                    return Err(parse_error(line, "music bpm must be between 40 and 180"));
                }
                if !(0.0..=1.0).contains(&volume) {
                    return Err(parse_error(line, "music volume must be between 0 and 1"));
                }
                sounds.music.push(MusicSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    height,
                    bars,
                    bpm,
                    volume,
                });
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sounds entry must be: sfx <name> seed=<seed> type=<type> | music <name> seed=<seed> bars=<8|16|32|64> height=<0..1> bpm=<40..180> volume=<0..1>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "sounds missing closing brace"))
}

#[derive(Clone, Debug)]
struct ModelSoundTrigger {
    kind: ModelSoundTriggerKind,
    objects: Vec<ObjectId>,
    sfx_name: String,
}

#[derive(Clone, Debug)]
struct ModelSoundTriggerSpec {
    kind: ModelSoundTriggerKind,
    selector: String,
    sfx_name: String,
    line: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelSoundTriggerKind {
    Move,
    CantMove,
}

fn model_sounds_block_starts(lines: &[String], start: usize) -> bool {
    lines.get(start + 1).is_some_and(|first| {
        matches!(
            split_header_tokens(first).as_slice(),
            ["move" | "cantmove", ..]
        )
    })
}

fn parse_model_sounds_block(
    lines: &[String],
    start: usize,
    triggers: &mut Vec<ModelSoundTriggerSpec>,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(
            &lines[start],
            "model sounds header must be: sounds",
        ));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        let trigger_kind = match tokens.as_slice() {
            ["move", ..] => Some(ModelSoundTriggerKind::Move),
            ["cantmove", ..] => Some(ModelSoundTriggerKind::CantMove),
            _ => None,
        };
        match (trigger_kind, tokens.as_slice()) {
            (Some(kind), [_, selector, "->", "sfx", name]) => {
                validate_qualified_identifier(name, line, "sfx name")?;
                triggers.push(ModelSoundTriggerSpec {
                    kind,
                    selector: (*selector).to_string(),
                    sfx_name: (*name).to_string(),
                    line: line.clone(),
                });
            }
            _ => {
                return Err(parse_error(
                    line,
                    "model sounds entry must be: move <object-selector> -> sfx <name> | cantmove <object-selector> -> sfx <name>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(
        &lines[start],
        "model sounds missing closing brace",
    ))
}

fn resolve_model_sound_triggers(
    specs: &[ModelSoundTriggerSpec],
    catalog: &Catalog,
) -> Result<Vec<ModelSoundTrigger>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    specs
        .iter()
        .map(|spec| {
            let selector = resolve_object_selector(
                &spec.selector,
                &spec.line,
                &catalog.object_names,
                &catalog.object_schemas,
                &value_sets,
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )
            .map_err(|error| model_sound_selector_error(error, spec))?;
            if selector
                .alternatives
                .iter()
                .any(|object| catalog.visual_objects.contains(object))
            {
                return Err(parse_error(
                    &spec.line,
                    "model sound triggers cannot target display objects",
                ));
            }
            Ok(ModelSoundTrigger {
                kind: spec.kind,
                objects: selector.alternatives,
                sfx_name: spec.sfx_name.clone(),
            })
        })
        .collect()
}

fn model_sound_selector_error(
    error: DiagnosticReport,
    spec: &ModelSoundTriggerSpec,
) -> DiagnosticReport {
    if error
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.message.starts_with("unknown object selector"))
    {
        parse_error(
            &spec.line,
            &format!(
                "unknown model sound trigger object selector `{}`",
                spec.selector
            ),
        )
    } else {
        error
    }
}

fn parse_theme_block(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    match header.as_slice() {
        ["theme", name] => {
            parse_theme_name_directive(&lines[start], name, theme)?;
        }
        _ => {
            return Err(parse_error(
                &lines[start],
                "theme header must be: theme <theme> or theme <theme> {",
            ));
        }
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match parse_theme_setting_tokens(tokens.as_slice(), line) {
            Ok(Some((name, value))) => upsert_theme_variable(theme, name, value),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "theme missing closing brace"))
}

fn parse_theme_setting_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Option<(String, String)>, DiagnosticReport> {
    match tokens {
        [name, value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        [name, "=", value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        _ => Err(parse_error(
            line,
            "theme entry must be: <setting> <value> or <setting> = <value>",
        )),
    }
}

fn parse_theme_statement(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["theme", name] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "theme header must be: theme <theme> or theme <theme> {",
        ));
    };
    if lines[start].trim_end().ends_with('{')
        || lines
            .get(start + 1)
            .is_some_and(|line| is_block_close_line(line) || is_theme_setting_line(line))
    {
        return parse_theme_block(lines, start, theme);
    }
    parse_theme_name_directive(&lines[start], name, theme)?;
    Ok(start + 1)
}

fn parse_theme_name_directive(
    line: &str,
    name: &str,
    theme: &mut ThemeDef,
) -> Result<(), DiagnosticReport> {
    validate_qualified_identifier(name, line, "theme name")?;
    theme.name = Some(name.to_string());
    Ok(())
}

fn is_theme_setting_line(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    parse_theme_setting_tokens(tokens.as_slice(), line).is_ok()
}

fn parse_assets_block(
    lines: &[String],
    start: usize,
    assets: &mut AssetsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["assets"]) {
        return Err(parse_error(&lines[start], "assets header must be: assets"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["css", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Css,
                path: parse_asset_path(path, line)?,
            }),
            ["script", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Script,
                path: parse_asset_path(path, line)?,
            }),
            _ => {
                return Err(parse_error(
                    line,
                    "assets entry must be: css \"path\" | script \"path\"",
                ));
            }
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "assets missing closing brace"))
}

fn parse_asset_path(token: &str, line: &str) -> Result<String, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "asset path must be quoted"))?;
    if path.is_empty() {
        return Err(parse_error(line, "asset path must not be empty"));
    }
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        return Err(parse_error(
            line,
            "asset path must be a game-folder relative path",
        ));
    }
    Ok(path.to_string())
}

fn parse_metadata_text(line: &str, keyword: &str) -> Result<String, DiagnosticReport> {
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            "metadata directive has the wrong keyword",
        ));
    };
    let value = rest.trim();
    if value.is_empty() {
        return Err(parse_error(line, "metadata value must not be empty"));
    }
    Ok(parse_quoted_text(value).unwrap_or_else(|| value.to_string()))
}

fn normalize_theme_setting_name(name: &str, line: &str) -> Result<String, DiagnosticReport> {
    let normalized = name
        .trim_start_matches("--")
        .replace('_', "-")
        .to_ascii_lowercase();
    for spec in THEME_SETTING_SPECS {
        if normalized == spec.canonical.replace('_', "-")
            || spec.aliases.iter().any(|alias| normalized == *alias)
        {
            return Ok(spec.css_variable.to_string());
        }
    }
    Err(parse_error(
        line,
        &format!(
            "theme setting must be one of: {}",
            THEME_SETTING_SPECS
                .iter()
                .map(|spec| spec.canonical)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))
}

fn validate_theme_value(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let is_safe_css_token = !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '#' | '.' | ',' | '%' | '(' | ')' | '-' | '_' | '/' | ':' | '+'
                )
        });
    if is_safe_css_token {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "theme setting value must be a compact CSS token without spaces",
        ))
    }
}

fn upsert_theme_variable(theme: &mut ThemeDef, name: String, value: String) {
    if let Some(existing) = theme
        .variables
        .iter_mut()
        .find(|variable| variable.name == name)
    {
        existing.value = value;
    } else {
        theme.variables.push(ThemeVariableDef { name, value });
    }
}

fn required_sound_setting<'a>(
    settings: &'a [&'a str],
    key: &str,
    line: &str,
) -> Result<&'a str, DiagnosticReport> {
    optional_sound_setting(settings, key)
        .ok_or_else(|| parse_error(line, &format!("sounds setting `{key}` is required")))
}

fn optional_sound_setting<'a>(settings: &'a [&'a str], key: &str) -> Option<&'a str> {
    settings.iter().find_map(|setting| {
        let (found_key, value) = setting.split_once('=')?;
        (found_key == key).then_some(value)
    })
}

fn validate_sound_atom(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a compact atom"),
        ))
    }
}

fn parse_sound_f64(value: &str, line: &str, label: &str) -> Result<f64, DiagnosticReport> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| parse_error(line, &format!("{label} must be a number")))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(parse_error(line, &format!("{label} must be finite")))
    }
}

fn parse_sound_u16(value: &str, line: &str, label: &str) -> Result<u16, DiagnosticReport> {
    value
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{label} must be u16")))
}

#[allow(clippy::too_many_arguments)]
fn parse_puzzle_definition(
    lines: &[String],
    start: usize,
    layer_count: &mut Option<u16>,
    empty_char: &mut Option<char>,
    named_layers: &mut HashMap<String, u16>,
    catalog: &mut Catalog,
    condition_definitions: &mut Vec<ConditionDefinitionAst>,
    controls: &mut Controls,
    directions: &mut Vec<Direction>,
    rule_definitions: &mut Vec<RuleDefinitionAst>,
    main_statements: &mut Option<Vec<StatementAst>>,
    main_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_start_statements: &mut Option<Vec<StatementAst>>,
    level_start_local_frame: &mut Option<LocalFrame<ObjectId>>,
    level_clear_statements: &mut Option<Vec<StatementAst>>,
    level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    last_level_clear_statements: &mut Option<Vec<StatementAst>>,
    last_level_clear_local_frame: &mut Option<LocalFrame<ObjectId>>,
    display_statements: &mut Option<Vec<StatementAst>>,
    level_blocks: &mut Vec<LevelBlock>,
    render_overlays: &mut OverlayDefs,
    model_sound_triggers: &mut Vec<ModelSoundTriggerSpec>,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
    run_rules_on_level_start: &mut bool,
    visuals: &mut VisualsDef,
    render: &mut PuzzleRenderDef,
    animation: &mut AnimationDef,
    puzzle_screen: &mut PuzzleScreenDef,
) -> Result<(usize, String), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let name = match header.as_slice() {
        ["puzzle", name] => *name,
        _ => {
            return Err(parse_error(
                &lines[start],
                "puzzle header must be: puzzle <name>",
            ));
        }
    };
    validate_qualified_identifier(name, &lines[start], "puzzle name")?;

    let mut i = start + 1;
    let mut diagnostics = Vec::new();
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }

        match tokens[0] {
            assignment_name if tokens.get(1).copied() == Some("=") => {
                parse_assignment_directive(assignment_name, line, catalog, named_conditions)?;
                i += 1;
            }
            "tags" => {
                if tokens.len() != 1 {
                    return Err(parse_error(line, "tags header must be: tags"));
                }
                i = parse_tags_block(lines, i, catalog)?;
            }
            "map" => {
                let value_sets = catalog_value_sets(catalog);
                let (map, next_i) = parse_map_definition(lines, i, &value_sets)?;
                if catalog.maps.insert(map.name.clone(), map).is_some() {
                    return Err(parse_error(line, "duplicate map"));
                }
                i = next_i;
            }
            "run_rules_on_level_start" => {
                if tokens.len() != 1 {
                    return Err(parse_error(
                        line,
                        "run_rules_on_level_start takes no values",
                    ));
                }
                *run_rules_on_level_start = true;
                i += 1;
            }
            lifecycle_block if puzzle_lifecycle_event(lifecycle_block).is_some() => {
                let local_frame = parse_program_local_frame_modifier(&tokens[1..], line, catalog)?;
                let lifecycle = puzzle_lifecycle_event(lifecycle_block).unwrap();
                let (event, statements, next_i) =
                    match parse_lifecycle_block(lines, i, lifecycle, catalog) {
                        Ok(parsed) => parsed,
                        Err(report) => {
                            diagnostics.extend(report.into_diagnostics());
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                    };
                match event.as_str() {
                    "level_start" => {
                        if level_start_statements.is_some() {
                            diagnostics.extend(
                                parse_error(line, "multiple level_start blocks are not supported")
                                    .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *level_start_statements = Some(statements);
                        *level_start_local_frame = local_frame;
                    }
                    "level_clear" => {
                        if level_clear_statements.is_some() {
                            diagnostics.extend(
                                parse_error(line, "multiple level_clear blocks are not supported")
                                    .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *level_clear_statements = Some(statements);
                        *level_clear_local_frame = local_frame;
                    }
                    "last_level_clear" => {
                        if last_level_clear_statements.is_some() {
                            diagnostics.extend(
                                parse_error(
                                    line,
                                    "multiple last_level_clear blocks are not supported",
                                )
                                .into_diagnostics(),
                            );
                            i = recover_after_directive_error(lines, i);
                            continue;
                        }
                        *last_level_clear_statements = Some(statements);
                        *last_level_clear_local_frame = local_frame;
                    }
                    _ => unreachable!("matched lifecycle event"),
                }
                i = next_i;
            }
            "layers" if tokens.len() == 1 => {
                i = parse_layers_block(lines, i + 1, named_layers, layer_count, catalog)?;
                refresh_layer_tags_and_value_sets(named_layers, catalog);
            }
            "layers" => {
                *layer_count = Some(parse_u16(tokens.get(1), line, "missing layer count")?);
                i += 1;
            }
            "collision_layers" => {
                diagnostics.extend(
                    parse_error(line, "`collision_layers` was removed; use `layers { ... }`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "empty" => {
                *empty_char = Some(parse_char(tokens.get(1), line, "missing empty char")?);
                i += 1;
            }
            "scratch" => {
                i = parse_scratch_block(lines, i, catalog)?;
            }
            "input" => {
                let (direction, next_i) = parse_command_definition(lines, i, catalog)?;
                if let Some(direction) = direction {
                    directions.push(direction);
                }
                i = next_i;
            }
            "inputs" => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                    )
                    .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "keys" => {
                i = parse_model_keys_block(lines, i, catalog, controls)?;
            }
            "var" | "const" | "persistent" => {
                parse_global_directive(
                    &tokens,
                    line,
                    &mut catalog.global_names,
                    &mut catalog.global_labels,
                    &mut catalog.global_defaults,
                    &mut catalog.numeric_global_defaults,
                    &mut catalog.persistent_vars,
                    &mut catalog.constant_globals,
                )?;
                i += 1;
            }
            "global" => {
                diagnostics.extend(
                    parse_error(line, "`global` was removed; use `var`").into_diagnostics(),
                );
                i += 1;
            }
            "condition" => {
                let definition = parse_condition_directive(
                    &tokens,
                    line,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(&catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &mut catalog.condition_names,
                    &mut catalog.condition_labels,
                )?;
                condition_definitions.push(definition);
                i += 1;
            }
            "effect" => {
                diagnostics.extend(
                    parse_error(line, "effect definitions are obsolete; use routine")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "groups" => {
                if tokens.len() == 1 {
                    i = parse_group_block(lines, i, catalog)?;
                } else {
                    return Err(parse_error(line, "groups block must be: groups { ... }"));
                }
            }
            "group" => {
                if tokens.len() == 1 {
                    diagnostics.extend(
                        parse_error(line, "`group { ... }` was removed; use `groups { ... }`")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                } else {
                    parse_group_directive(
                        &tokens,
                        line,
                        &catalog.object_names,
                        &catalog.object_schemas,
                        &catalog_value_sets(&catalog),
                        &catalog.maps,
                        &catalog.visual_objects,
                        &mut catalog.object_groups,
                    )?;
                    i += 1;
                }
            }
            "direction" => {
                if let Some(direction) = parse_direction_directive(&tokens, line, catalog)? {
                    directions.push(direction);
                }
                i += 1;
            }
            "legend" => {
                diagnostics.extend(
                    parse_error(line, "`legend` must be inside `levels { ... }`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "render_overlay" => {
                let (overlays, level_objects, ch) = parse_render_overlay(
                    &tokens,
                    line,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(&catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                )?;
                render_overlays.extend(overlays);
                if let Some(objects) = level_objects {
                    catalog.char_objects.insert(ch, objects);
                }
                i += 1;
            }
            "win_conditions" | "lose_conditions" => {
                i = parse_conditions_block(lines, i, catalog, named_conditions)?;
            }
            "sprites" => {
                i = parse_visuals_block(lines, i, catalog, visuals)?;
            }
            "render" => {
                i = parse_puzzle_render_block(lines, i, render)?;
            }
            "animation" => {
                i = parse_animation_block(lines, i, animation)?;
            }
            "sounds" => {
                i = parse_model_sounds_block(lines, i, model_sound_triggers)?;
            }
            "screen" | "layout" => {
                i = parse_puzzle_screen_block(lines, i, puzzle_screen)?;
            }
            "flickscreen" | "zoomscreen" | "screen_focus" => {
                parse_puzzle_screen_directive(line, puzzle_screen)?;
                i += 1;
            }
            "frame_focus" | "frame_size" | "switch_frame" | "follow_frame" => {
                diagnostics.extend(parse_error(
                    line,
                    "`frame_*` screen directives were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`",
                ).into_diagnostics());
                i += 1;
            }
            "routine" => {
                match parse_rule_definition(
                    lines,
                    i,
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                ) {
                    Ok((definition, next_i)) => {
                        rule_definitions.push(definition);
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "rule" => {
                diagnostics.extend(
                    parse_error(line, "`rule` was removed; use `routine`").into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "rules" => {
                let local_frame = parse_program_local_frame_modifier(&tokens[1..], line, catalog)?;
                if main_statements.is_some() {
                    diagnostics.extend(
                        parse_error(line, "multiple puzzle rules blocks are not supported")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
                match parse_statement_block(
                    lines,
                    i + 1,
                    &[BLOCK_CLOSE],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                    named_conditions,
                    &[],
                ) {
                    Ok((statements, next_i)) => {
                        *main_statements = Some(statements);
                        *main_local_frame = local_frame;
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "main" | "transitions" => {
                diagnostics.extend(
                    parse_error(line, "`main`/`transitions` were removed; use `rules`")
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
            "on_display" => {
                if tokens.len() != 1 {
                    return Err(parse_error(line, "display hook header must be: on_display"));
                }
                if display_statements.is_some() {
                    diagnostics.extend(
                        parse_error(line, "multiple on_display blocks are not supported")
                            .into_diagnostics(),
                    );
                    i = recover_after_directive_error(lines, i);
                    continue;
                }
                match parse_statement_block(
                    lines,
                    i + 1,
                    &[BLOCK_CLOSE],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &catalog.input_names,
                    &catalog.global_names,
                    &catalog.numeric_global_defaults,
                    &catalog.condition_names,
                    named_conditions,
                    &[],
                ) {
                    Ok((statements, next_i)) => {
                        validate_display_hook_statements(&statements)?;
                        *display_statements = Some(statements);
                        i = next_i;
                    }
                    Err(report) => {
                        diagnostics.extend(report.into_diagnostics());
                        i = recover_after_directive_error(lines, i);
                    }
                }
            }
            "display" => {
                diagnostics.extend(parse_error(
                    line,
                    "display blocks are not supported; use `display <rule>` inside transitions, on_level_start, or on_level_clear",
                ).into_diagnostics());
                i = recover_after_directive_error(lines, i);
            }
            "levels" => {
                i = parse_levels_block(
                    lines,
                    i,
                    level_blocks,
                    catalog,
                    render_overlays,
                    empty_char,
                    Some(name),
                )?;
            }
            "level" => {
                let (level, next_i) = parse_level_block(lines, i, level_blocks.len())?;
                level_blocks.push(level);
                i = next_i;
            }
            other => {
                diagnostics.extend(
                    parse_error(line, &format!("unknown puzzle directive {other}"))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(lines, i);
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "puzzle missing closing brace"));
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }
    validate_puzzle_screen(puzzle_screen, &lines[start])?;

    Ok((i + 1, name.to_string()))
}

fn parse_program_local_frame_modifier(
    tokens: &[&str],
    line: &str,
    catalog: &Catalog,
) -> Result<Option<LocalFrame<ObjectId>>, DiagnosticReport> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let focus_objects = default_local_frame_focus_objects(catalog, line)?;
    match tokens {
        ["local_radius", radius] => {
            let radius = parse_u16(Some(radius), line, "missing local radius")?;
            Ok(Some(LocalFrame::new(
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Full,
                focus_objects,
            )))
        }
        ["local_frame", x, y] => Ok(Some(LocalFrame::new(
            parse_local_frame_extent(x, line)?,
            parse_local_frame_extent(y, line)?,
            LocalFrameExtent::Full,
            focus_objects,
        ))),
        _ => Err(parse_error(
            line,
            "transition block header must be: rules [local_radius <n> | local_frame <x> <y>] | on_level_start [local_radius <n> | local_frame <x> <y>] | on_level_clear [local_radius <n> | local_frame <x> <y>]",
        )),
    }
}

fn parse_local_frame_extent(token: &str, line: &str) -> Result<LocalFrameExtent, DiagnosticReport> {
    if token == "full" {
        return Ok(LocalFrameExtent::Full);
    }
    parse_u16(Some(&token), line, "missing local frame extent").map(LocalFrameExtent::Radius)
}

fn default_local_frame_focus_objects(
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    for name in ["Player", "player"] {
        if let Some(object) = catalog.object_names.get(name) {
            return Ok(vec![*object]);
        }
    }
    Err(parse_error(
        line,
        "local_frame/local_radius requires an object named Player",
    ))
}

fn parse_level_block(
    lines: &[String],
    start: usize,
    existing_count: usize,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let level_name =
        parse_level_header_name_or_auto(&lines[start], unnamed_level_name(existing_count))?;
    parse_named_level_body(lines, start, level_name, &LevelsHeader::default())
}

fn parse_levels_block(
    lines: &[String],
    start: usize,
    level_blocks: &mut Vec<LevelBlock>,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
    default_puzzle: Option<&str>,
) -> Result<usize, DiagnosticReport> {
    let header = parse_levels_header(&lines[start], default_puzzle)?;
    let mut namespace_count = 0usize;
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["legend"] => {
                i = parse_legend_block(lines, i, catalog, render_overlays, empty_char)?;
            }
            ["legend", ..] => {
                parse_legend_directive(
                    &tokens,
                    &lines[i],
                    &catalog.object_names,
                    &catalog.object_schemas,
                    &catalog_value_sets(catalog),
                    &catalog.maps,
                    &catalog.object_groups,
                    &mut catalog.render_chars,
                    &mut catalog.char_objects,
                    render_overlays,
                )?;
                i += 1;
            }
            ["level", ..] => {
                namespace_count += 1;
                let auto_name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let level_name = parse_level_header_name_or_auto(&lines[i], auto_name)
                    .map(|name| namespaced_level_name_if_needed(header.pack.as_deref(), name))?;
                let (level, next_i) = if is_braced_level_header(&lines[i]) {
                    parse_named_level_body(lines, i, level_name, &header)?
                } else {
                    parse_unbraced_level_body(lines, i + 1, level_name, &header)?
                };
                level_blocks.push(level);
                i = next_i;
            }
            ["{"] => {
                namespace_count += 1;
                let name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let (level, next_i) = parse_named_level_body(lines, i, name, &header)?;
                level_blocks.push(level);
                i = next_i;
            }
            [] => i += 1,
            _ if lines[i].trim_end().ends_with('{') => {
                return Err(parse_error(
                    &lines[i],
                    "braced level header must be `level <name> {` or `{` for an unnamed level",
                ));
            }
            _ => {
                namespace_count += 1;
                let name = namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    level_blocks.len(),
                    namespace_count,
                );
                let (level, next_i) = parse_unbraced_level_body(lines, i, name, &header)?;
                level_blocks.push(level);
                i = next_i;
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "levels missing closing brace"));
    }

    Ok(i + 1)
}

#[derive(Clone, Debug, Default)]
struct LevelsHeader {
    pack: Option<String>,
    puzzle: Option<String>,
}

fn parse_levels_header(
    line: &str,
    default_puzzle: Option<&str>,
) -> Result<LevelsHeader, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["levels"] => Ok(LevelsHeader {
            pack: None,
            puzzle: default_puzzle.map(str::to_string),
        }),
        ["levels", "of", puzzle] => {
            validate_qualified_identifier(puzzle, line, "levels puzzle")?;
            Ok(LevelsHeader {
                pack: None,
                puzzle: Some((*puzzle).to_string()),
            })
        }
        ["levels", pack, "of", puzzle] => {
            validate_qualified_identifier(pack, line, "levels pack")?;
            validate_qualified_identifier(puzzle, line, "levels puzzle")?;
            Ok(LevelsHeader {
                pack: Some((*pack).to_string()),
                puzzle: Some((*puzzle).to_string()),
            })
        }
        _ => Err(parse_error(
            line,
            "levels header must be: levels, levels of <puzzle>, or levels <pack> of <puzzle>",
        )),
    }
}

fn resolve_level_block_puzzles(
    levels: &mut [LevelBlock],
    puzzle_models: &[String],
) -> Result<(), DiagnosticReport> {
    let unique_models = puzzle_models.iter().collect::<HashSet<_>>();
    for level in levels {
        if let Some(puzzle) = &level.puzzle {
            if !unique_models.contains(puzzle) {
                return Err(DiagnosticReport::error(format!(
                    "levels target unknown puzzle model: {puzzle}"
                )));
            }
            continue;
        }
        match unique_models.len() {
            0 => {
                return Err(DiagnosticReport::error(
                    "bare levels requires one puzzle definition".to_string(),
                ));
            }
            1 => {
                level.puzzle = unique_models.iter().next().map(|name| (*name).clone());
            }
            _ => {
                return Err(DiagnosticReport::error(
                    "bare levels is ambiguous with multiple puzzle models; use `levels of <puzzle>` or `levels <pack> of <puzzle>`".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn parse_level_header_name_or_auto(
    line: &str,
    auto_name: String,
) -> Result<String, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if tokens.len() == 1 {
        return Ok(auto_name);
    }
    if tokens.len() < 2 {
        return Err(parse_error(line, "level header must be: level <name>"));
    }
    Ok(tokens[1..].join(" "))
}

fn is_braced_level_header(line: &str) -> bool {
    line.trim_end().ends_with('{') && matches!(split_header_tokens(line).as_slice(), ["level", ..])
}

fn unnamed_level_name(existing_count: usize) -> String {
    format!("unnamed level {}", existing_count + 1)
}

fn namespaced_level_name_if_needed(namespace: Option<&str>, name: String) -> String {
    match namespace {
        Some(namespace) if !name.starts_with(&format!("{namespace}.")) => {
            format!("{namespace}.{name}")
        }
        _ => name,
    }
}

fn namespaced_unnamed_level_name(
    namespace: Option<&str>,
    existing_count: usize,
    namespace_count: usize,
) -> String {
    match namespace {
        Some(namespace) => format!("{namespace}.{namespace_count}"),
        None => unnamed_level_name(existing_count),
    }
}

fn parse_conditions_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
) -> Result<usize, DiagnosticReport> {
    let header_tokens = split_header_tokens(&lines[start]);
    let condition_name = header_tokens.first().copied().unwrap_or("win_conditions");
    let combinator = match header_tokens.as_slice() {
        [_] => ConditionBlockCombinator::All,
        [_, "all"] => ConditionBlockCombinator::All,
        [_, "any"] => ConditionBlockCombinator::Any,
        _ => {
            return Err(parse_error(
                &lines[start],
                &format!("{condition_name} block must be: {condition_name} [all | any]"),
            ));
        }
    };
    if named_conditions.contains_key(condition_name) {
        return Err(parse_error(
            &lines[start],
            &format!("duplicate {condition_name} definition"),
        ));
    }

    let mut conditions = Vec::new();
    let mut descriptions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        i = parse_condition_block_entry(
            lines,
            i,
            condition_name,
            catalog,
            &mut conditions,
            &mut descriptions,
        )?;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            &format!("{condition_name} missing closing brace"),
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            &format!("{condition_name} requires at least one condition"),
        ));
    }

    named_conditions.insert(
        condition_name.to_string(),
        (
            descriptions.join(combinator.description_joiner()),
            if conditions.len() == 1 {
                conditions.remove(0)
            } else {
                combinator.combine(conditions)
            },
        ),
    );
    Ok(i + 1)
}

fn parse_condition_block_entry(
    lines: &[String],
    start: usize,
    condition_name: &str,
    catalog: &Catalog,
    conditions: &mut Vec<ConditionAst>,
    descriptions: &mut Vec<String>,
) -> Result<usize, DiagnosticReport> {
    let line = &lines[start];
    let tokens = split_header_tokens(line);
    if matches!(tokens.as_slice(), ["for", _, "in", ..]) {
        let ["for", binding, "in", sources @ ..] = tokens.as_slice() else {
            unreachable!("checked by matches");
        };
        let value_sets = catalog_value_sets(catalog);
        let values =
            for_expansion_values(sources, &value_sets, &catalog.numeric_global_defaults, line)?;
        validate_identifier(binding, line, "expansion binding")?;
        let (body_lines, next_i) = collect_statement_block_lines(lines, start + 1, line)?;
        for value in values {
            let expanded_lines = expand_for_binding_lines(
                &body_lines,
                binding,
                value.axis.as_deref(),
                &value.value,
                &catalog.maps,
            )?;
            parse_condition_rows(
                &expanded_lines,
                condition_name,
                catalog,
                conditions,
                descriptions,
            )?;
        }
        return Ok(next_i);
    }

    let condition = parse_condition_block_row(line, condition_name, catalog)?;
    descriptions.push(line.clone());
    conditions.push(condition);
    Ok(start + 1)
}

fn parse_condition_rows(
    lines: &[String],
    condition_name: &str,
    catalog: &Catalog,
    conditions: &mut Vec<ConditionAst>,
    descriptions: &mut Vec<String>,
) -> Result<(), DiagnosticReport> {
    let mut i = 0;
    while i < lines.len() {
        i = parse_condition_block_entry(
            lines,
            i,
            condition_name,
            catalog,
            conditions,
            descriptions,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConditionBlockCombinator {
    All,
    Any,
}

impl ConditionBlockCombinator {
    fn description_joiner(self) -> &'static str {
        match self {
            Self::All => " and ",
            Self::Any => " or ",
        }
    }

    fn combine(self, conditions: Vec<ConditionAst>) -> ConditionAst {
        match self {
            Self::All => ConditionAst::All(conditions),
            Self::Any => ConditionAst::Any(conditions),
        }
    }
}

fn parse_puzzle_screen_block(
    lines: &[String],
    start: usize,
    puzzle_screen: &mut PuzzleScreenDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = puzzle_screen.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            _ => {
                parse_puzzle_screen_directive(line, &mut parsed)?;
                i += 1;
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "puzzle screen missing closing brace",
        ));
    }
    validate_puzzle_screen(&parsed, &lines[start])?;
    *puzzle_screen = parsed;
    Ok(i + 1)
}

fn parse_puzzle_render_block(
    lines: &[String],
    start: usize,
    render: &mut PuzzleRenderDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = render.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == PUZZLE_RENDER_BLOCK_OPTIONS[0] => {
                i = parse_puzzle_render_grid_block(lines, i, &mut parsed.grid)?;
            }
            [name, options @ ..] if *name == PUZZLE_RENDER_BLOCK_OPTIONS[0] => {
                parse_puzzle_render_grid_options(options, line, &mut parsed.grid)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown render directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "render block missing closing brace",
        ));
    }
    *render = parsed;
    Ok(i + 1)
}

pub(crate) const PUZZLE_RENDER_BLOCK_OPTIONS: &[&str] = &["grid"];
pub(crate) const PUZZLE_RENDER_GRID_OPTIONS: &[&str] = &["occupied_cells", "all_cells"];
pub(crate) const ANIMATION_BLOCK_OPTIONS: &[&str] = &["tween"];
pub(crate) const ANIMATION_TWEEN_OPTIONS: &[&str] = &["duration"];

fn parse_animation_block(
    lines: &[String],
    start: usize,
    animation: &mut AnimationDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["animation"]) {
        return Err(parse_error(
            &lines[start],
            "animation header must be: animation",
        ));
    }

    let mut parsed = animation.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == ANIMATION_BLOCK_OPTIONS[0] => {
                parsed.tween.enabled = true;
                if lines
                    .get(i + 1)
                    .is_some_and(|next| is_block_close_line(next))
                {
                    i += 1;
                } else {
                    i = parse_animation_tween_block(lines, i, &mut parsed.tween)?;
                }
            }
            [name, options @ ..] if *name == ANIMATION_BLOCK_OPTIONS[0] => {
                parsed.tween.enabled = true;
                parse_animation_tween_options(options, line, &mut parsed.tween)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown animation directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "animation block missing closing brace",
        ));
    }
    *animation = parsed;
    Ok(i + 1)
}

fn parse_animation_tween_block(
    lines: &[String],
    start: usize,
    tween: &mut TweenAnimationDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = tween.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", value] | [name, value] if *name == ANIMATION_TWEEN_OPTIONS[0] => {
                parsed.interval_ms = parse_animation_duration_ms(value, line)?;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(line, &format!("unknown tween setting {other}")));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "tween block missing closing brace",
        ));
    }
    *tween = parsed;
    Ok(i + 1)
}

fn parse_animation_tween_options(
    options: &[&str],
    line: &str,
    tween: &mut TweenAnimationDef,
) -> Result<(), DiagnosticReport> {
    if options.is_empty() {
        return Err(parse_error(
            line,
            "tween directive requires at least one option",
        ));
    }
    for option in options {
        let Some((name, value)) = option.split_once('=') else {
            return Err(parse_error(
                line,
                "tween option must be name=value in inline form",
            ));
        };
        match name {
            name if name == ANIMATION_TWEEN_OPTIONS[0] && !value.is_empty() => {
                tween.interval_ms = parse_animation_duration_ms(value, line)?;
            }
            name if name == ANIMATION_TWEEN_OPTIONS[0] => {
                return Err(parse_error(line, "tween duration must not be empty"));
            }
            other => return Err(parse_error(line, &format!("unknown tween setting {other}"))),
        }
    }
    Ok(())
}

fn parse_animation_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let milliseconds = parse_wait_duration_ms(value, line)?;
    if milliseconds == 0 {
        return Err(parse_error(line, "tween duration must be greater than 0"));
    }
    Ok(milliseconds)
}

fn parse_puzzle_render_grid_block(
    lines: &[String],
    start: usize,
    grid: &mut PuzzleGridRenderDef,
) -> Result<usize, DiagnosticReport> {
    let mut parsed = grid.clone();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if *name == PUZZLE_RENDER_GRID_OPTIONS[0] => {
                parsed.occupied_cells = true;
                i += 1;
            }
            [name] if *name == PUZZLE_RENDER_GRID_OPTIONS[1] => {
                parsed.all_cells = true;
                i += 1;
            }
            [other, ..] => {
                return Err(parse_error(line, &format!("unknown grid setting {other}")));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "grid block missing closing brace",
        ));
    }
    *grid = parsed;
    Ok(i + 1)
}

fn parse_puzzle_render_grid_options(
    options: &[&str],
    line: &str,
    grid: &mut PuzzleGridRenderDef,
) -> Result<(), DiagnosticReport> {
    if options.is_empty() {
        return Err(parse_error(
            line,
            "grid directive requires at least one option",
        ));
    }
    for option in options {
        match *option {
            option if option == PUZZLE_RENDER_GRID_OPTIONS[0] => grid.occupied_cells = true,
            option if option == PUZZLE_RENDER_GRID_OPTIONS[1] => grid.all_cells = true,
            other => return Err(parse_error(line, &format!("unknown grid setting {other}"))),
        }
    }
    Ok(())
}

fn parse_puzzle_screen_directive(
    line: &str,
    puzzle_screen: &mut PuzzleScreenDef,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["flickscreen", "full"] => {
            puzzle_screen.viewport_size = ViewportSizeDef::Full;
            puzzle_screen.viewport_mode = ViewportModeDef::Paged;
        }
        ["flickscreen", ..] => {
            let (width, height) = parse_screen_size_directive(line, "flickscreen")?;
            puzzle_screen.viewport_size = ViewportSizeDef::Size { width, height };
            puzzle_screen.viewport_mode = ViewportModeDef::Paged;
        }
        ["zoomscreen", ..] => {
            let (width, height) = parse_screen_size_directive(line, "zoomscreen")?;
            puzzle_screen.viewport_size = ViewportSizeDef::Size { width, height };
            puzzle_screen.viewport_mode = ViewportModeDef::Centered;
        }
        ["screen_focus", selector] => {
            validate_identifier(selector, line, "viewport focus selector")?;
            puzzle_screen.viewport_focus = (*selector).to_string();
        }
        ["frame_focus", ..] | ["frame_size", ..] | ["switch_frame"] | ["follow_frame"] => {
            return Err(parse_error(
                line,
                "`frame_*` screen directives were removed; use `flickscreen`, `zoomscreen`, or `screen_focus`",
            ));
        }
        [other, ..] => {
            return Err(parse_error(
                line,
                &format!("unknown puzzle screen directive {other}"),
            ));
        }
        [] => {}
    }
    Ok(())
}

fn validate_puzzle_screen(
    puzzle_screen: &PuzzleScreenDef,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if !matches!(puzzle_screen.viewport_size, ViewportSizeDef::Size { .. })
        && puzzle_screen.viewport_mode == ViewportModeDef::Centered
    {
        return Err(parse_error(
            line,
            "centered viewport requires `zoomscreen <w> <h>`",
        ));
    }
    Ok(())
}

fn parse_screen_size_directive(
    line: &str,
    directive: &str,
) -> Result<(u16, u16), DiagnosticReport> {
    let value = line
        .strip_prefix(directive)
        .map(str::trim)
        .unwrap_or_default();
    if value == "full" || value == "region" {
        return Err(parse_error(
            line,
            &format!("{directive} {value} is not supported"),
        ));
    }
    if value.starts_with('(') {
        return parse_u16_tuple(value, line, directive);
    }
    if let Some((width, height)) = value.split_once('x').or_else(|| value.split_once('X')) {
        return Ok((
            width
                .trim()
                .parse::<u16>()
                .map_err(|_| parse_error(line, &format!("{directive} width must be u16")))?,
            height
                .trim()
                .parse::<u16>()
                .map_err(|_| parse_error(line, &format!("{directive} height must be u16")))?,
        ));
    }
    let size_tokens = split_header_tokens(value);
    let [width, height] = size_tokens.as_slice() else {
        return Err(parse_error(
            line,
            &format!("{directive} must be: {directive} (w, h)"),
        ));
    };
    Ok((
        parse_u16(Some(width), line, "missing screen width")?,
        parse_u16(Some(height), line, "missing screen height")?,
    ))
}

fn parse_u16_tuple(value: &str, line: &str, name: &str) -> Result<(u16, u16), DiagnosticReport> {
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| parse_error(line, &format!("{name} tuple must be: (w,h)")))?;
    let Some((left, right)) = inner.split_once(',') else {
        return Err(parse_error(line, &format!("{name} tuple must be: (w,h)")));
    };
    let width = left
        .trim()
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{name} width must be u16")))?;
    let height = right
        .trim()
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{name} height must be u16")))?;
    Ok((width, height))
}

fn parse_condition_block_row(
    line: &str,
    condition_name: &str,
    catalog: &Catalog,
) -> Result<ConditionAst, DiagnosticReport> {
    if let Some(pattern) = line.trim().strip_prefix("some ") {
        let pattern = pattern.trim();
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::ExistsMatches(pattern),
            ));
        }
    }
    if let Some(pattern) = line.trim().strip_prefix("no ") {
        let pattern = pattern.trim();
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::NoneMatches(pattern),
            ));
        }
    }

    if let Ok(condition) = parse_condition_expr(
        line,
        line,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.condition_names,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
    ) {
        return Ok(condition);
    }

    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["all", target, "on", cover] => {
            let expr = format!("none([ {target} no {cover} ])");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["some", target, "on", cover] => {
            let expr = format!("exists([ {target} {cover} ])");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["some", target] => {
            let expr = format!("exists({target})");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        ["no", target] => {
            let expr = format!("none({target})");
            parse_condition_expr(
                &expr,
                line,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.condition_names,
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
            )
        }
        _ => Err(parse_error(
            line,
            &format!(
                "{condition_name} row must be a condition expression, all <object> on <object>, some/no [pattern], some <object> on <object>, or some/no <object>"
            ),
        )),
    }
}

fn parse_named_level_body(
    lines: &[String],
    start: usize,
    name: String,
    header: &LevelsHeader,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let mut level_lines = Vec::new();
    let mut i = start + 1;
    let mut nested_blocks = 0usize;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if nested_blocks == 0 {
                break;
            }
            nested_blocks -= 1;
            level_lines.push(lines[i].clone());
            i += 1;
            continue;
        }
        if is_level_body_block(&split_header_tokens(&lines[i])) {
            nested_blocks += 1;
        }
        level_lines.push(lines[i].clone());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "level missing closing brace"));
    }

    Ok((
        LevelBlock {
            name,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
        },
        i + 1,
    ))
}

fn parse_unbraced_level_body(
    lines: &[String],
    start: usize,
    name: String,
    header: &LevelsHeader,
) -> Result<(LevelBlock, usize), DiagnosticReport> {
    let mut level_lines = Vec::new();
    let mut i = start;
    let mut nested_blocks = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if nested_blocks == 0 && (line.is_empty() || is_block_close_line(line)) {
            break;
        }
        let tokens = split_header_tokens(line);
        if nested_blocks == 0 && matches!(tokens.as_slice(), ["level", ..]) {
            if !level_lines.is_empty() {
                break;
            }
            return Err(parse_error(
                line,
                "unbraced levels must be separated by a blank line",
            ));
        }
        if is_block_close_line(line) {
            nested_blocks = nested_blocks.saturating_sub(1);
            level_lines.push(line.clone());
            i += 1;
            continue;
        }
        if is_level_body_block(&tokens) {
            nested_blocks += 1;
        }
        level_lines.push(line.clone());
        i += 1;
    }
    if level_lines.is_empty() {
        return Err(parse_error(
            &lines[start.saturating_sub(1)],
            "level requires at least one row",
        ));
    }

    Ok((
        LevelBlock {
            name,
            pack: header.pack.clone(),
            puzzle: header.puzzle.clone(),
            lines: level_lines,
        },
        i,
    ))
}

fn is_level_body_block(tokens: &[&str]) -> bool {
    matches!(tokens, ["legend"] | ["on_level_start"] | ["on_level_clear"])
}

#[derive(Clone, Debug)]
struct PreparedLevelBody {
    name: String,
    pack: Option<String>,
    puzzle: String,
    lines: Vec<String>,
    char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
}

#[derive(Clone, Debug, Default)]
struct ParsedLevelBody {
    lines: Vec<String>,
    local_char_objects: HashMap<char, Vec<ObjectId>>,
    level_start_statements: Vec<StatementAst>,
    level_clear_statements: Vec<StatementAst>,
}

#[allow(clippy::too_many_arguments)]
fn parse_level_body(
    level: &LevelBlock,
    catalog: &Catalog,
    empty_char: char,
    default_wait_ms: u64,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
) -> Result<ParsedLevelBody, DiagnosticReport> {
    let mut body = ParsedLevelBody::default();
    let mut saw_map_row = false;
    let mut i = 0;
    while i < level.lines.len() {
        let line = &level.lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            if saw_map_row {
                body.lines.push(line.clone());
            }
            i += 1;
            continue;
        }

        if matches!(tokens.as_slice(), ["on_level_start"] | ["on_level_clear"]) {
            let (statements, next_i) = parse_statement_block(
                &level.lines,
                i + 1,
                &[BLOCK_CLOSE],
                &catalog.object_names,
                &catalog.object_schemas,
                &catalog_value_sets(catalog),
                &catalog.maps,
                &catalog.object_groups,
                &catalog.input_names,
                &catalog.global_names,
                &catalog.numeric_global_defaults,
                &catalog.condition_names,
                named_conditions,
                &[],
            )?;
            if tokens[0] == "on_level_start" {
                body.level_start_statements.extend(statements);
            } else {
                body.level_clear_statements.extend(statements);
            }
            i = next_i;
            continue;
        }
        if tokens[0] == "on_level_start" || tokens[0] == "on_level_clear" {
            return Err(parse_error(
                line,
                "level lifecycle block header must be: on_level_start | on_level_clear",
            ));
        }

        if let Some(statement) = parse_level_event_sugar(line, default_wait_ms)? {
            if saw_map_row {
                body.level_clear_statements.push(statement);
            } else {
                body.level_start_statements.push(statement);
            }
            i += 1;
            continue;
        }

        if tokens[0] != "legend" {
            saw_map_row = true;
            body.lines.push(line.clone());
            i += 1;
            continue;
        }

        if tokens.len() == 1 {
            i += 1;
            while i < level.lines.len() && !is_block_close_line(&level.lines[i]) {
                parse_level_legend_block_row(
                    &level.lines[i],
                    catalog,
                    empty_char,
                    &mut body.local_char_objects,
                )?;
                i += 1;
            }
            if i >= level.lines.len() {
                return Err(parse_error(line, "level legend missing closing brace"));
            }
            i += 1;
            continue;
        }

        let (ch, objects) = parse_level_legend_directive(&tokens, line, catalog, empty_char)?;
        body.local_char_objects.insert(ch, objects);
        i += 1;
    }

    Ok(body)
}

fn parse_level_event_sugar(
    line: &str,
    default_wait_ms: u64,
) -> Result<Option<StatementAst>, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    let is_level_event = matches!(tokens.as_slice(), ["sfx", _] | ["wait"] | ["wait", _])
        || line.strip_prefix("message ").is_some();
    if !is_level_event {
        return Ok(None);
    }
    let effects = parse_rewrite_effect(line, line)?;
    if effects.iter().any(|effect| {
        !matches!(
            effect,
            EffectAst::PlaySfx { .. }
                | EffectAst::Wait { .. }
                | EffectAst::WaitAnimation
                | EffectAst::Message { .. }
        )
    }) {
        return Err(parse_error(
            line,
            "level body sugar only supports message, sfx, and wait; put other behavior in on_level_start/on_level_clear",
        ));
    }
    let effects = effects
        .into_iter()
        .map(|effect| match effect {
            EffectAst::Wait { milliseconds: None } => EffectAst::Wait {
                milliseconds: Some(default_wait_ms),
            },
            other => other,
        })
        .collect();
    Ok(Some(StatementAst::Effect { effects }))
}

fn parse_level_legend_block_row(
    line: &str,
    catalog: &Catalog,
    empty_char: char,
    local_char_objects: &mut HashMap<char, Vec<ObjectId>>,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
        return Err(parse_error(
            line,
            "level legend row must be: <char> = <selector...>",
        ));
    }

    let mut directive_tokens = vec!["legend"];
    directive_tokens.extend(tokens);
    let (ch, objects) = parse_level_legend_directive(&directive_tokens, line, catalog, empty_char)?;
    local_char_objects.insert(ch, objects);
    Ok(())
}

fn parse_level_legend_directive(
    tokens: &[&str],
    line: &str,
    catalog: &Catalog,
    empty_char: char,
) -> Result<(char, Vec<ObjectId>), DiagnosticReport> {
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "level legend must be: legend <char> = <selector...>",
        ));
    }

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    if ch == empty_char || tokens[3..] == ["empty"] {
        return Err(parse_error(line, "level-local legend cannot define empty"));
    }
    let selector_sets = selector_sets(
        &tokens[3..],
        line,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);

    let level_objects = if selector_sets.len() == 1 && selector_sets[0].len() == 1 {
        Some(vec![selector_sets[0][0]])
    } else if selector_sets.len() > 1 && combinations.len() == 1 {
        Some(combinations[0].clone())
    } else {
        None
    };
    let Some(objects) = level_objects else {
        return Err(parse_error(
            line,
            "level-local legend must resolve to one concrete object set",
        ));
    };

    Ok((ch, objects))
}

fn parse_map_definition(
    lines: &[String],
    start: usize,
    value_sets: &HashMap<String, Vec<String>>,
) -> Result<(ValueMap, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let ["map", name, axis] = header.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "map header must be: map <name> <tag_set>",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(&lines[start], "map name must be an identifier"));
    }
    let value_set_values = value_sets
        .get(*axis)
        .ok_or_else(|| parse_error(&lines[start], "map tag set must name an existing tag set"))?;

    let mut values = HashMap::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            [from, "->", to] => {
                if !value_set_values.iter().any(|value| value == from) {
                    return Err(parse_error(&lines[i], "map input is not in tag set"));
                }
                if !value_set_values.iter().any(|value| value == to) {
                    return Err(parse_error(&lines[i], "map output is not in tag set"));
                }
                if values
                    .insert((*from).to_string(), (*to).to_string())
                    .is_some()
                {
                    return Err(parse_error(&lines[i], "duplicate map input"));
                }
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "map row must be: <value> -> <value>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "map missing closing brace"));
    }

    for value in value_set_values {
        if !values.contains_key(value) {
            return Err(parse_error(&lines[start], "map must cover every tag value"));
        }
    }

    Ok((
        ValueMap {
            name: (*name).to_string(),
            axis: (*axis).to_string(),
            values,
        },
        i + 1,
    ))
}

fn parse_tags_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => {}
            [name, "=", values @ ..] => {
                parse_tag_set_directive(name, values, line, catalog)?;
            }
            _ => {
                return Err(parse_error(line, "tag row must be: <name> = <value...>"));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "tags missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_tag_set_directive(
    name: &str,
    values: &[&str],
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    validate_identifier(name, line, "tag set name")?;
    if values.is_empty() {
        return Err(parse_error(line, "tag set must have at least one value"));
    }
    let expanded_values =
        expand_numeric_ranges_in_value_list(values, &catalog.numeric_global_defaults, line)?;
    if expanded_values.is_empty() {
        return Err(parse_error(line, "tag set must have at least one value"));
    }
    if is_builtin_value_set(name) {
        return Err(parse_error(line, "built-in tag set cannot be redefined"));
    }
    if catalog.value_sets.contains_key(name) || catalog.object_axes.contains_key(name) {
        return Err(parse_error(line, "duplicate tag set"));
    }
    catalog
        .object_axes
        .insert(name.to_string(), expanded_values);
    Ok(())
}

fn parse_assignment_directive(
    name: &str,
    line: &str,
    catalog: &mut Catalog,
    named_conditions: &mut HashMap<String, (String, ConditionAst)>,
) -> Result<(), DiagnosticReport> {
    if !is_identifier(name) {
        return Err(parse_error(line, "assignment name must be an identifier"));
    }
    let Some((_, expr)) = line.split_once('=') else {
        return Err(parse_error(line, "assignment must be: <name> = <value>"));
    };
    let expr = expr.trim();
    if looks_like_condition_expr(expr) {
        if named_conditions.contains_key(name) {
            return Err(parse_error(line, "duplicate condition"));
        }
        let condition = parse_condition_expr(
            expr,
            line,
            &catalog.input_names,
            &catalog.global_names,
            &catalog.condition_names,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
        )?;
        named_conditions.insert(name.to_string(), (expr.to_string(), condition));
        return Ok(());
    }

    Err(parse_error(
        line,
        "tag sets must be declared inside `tags { ... }`",
    ))
}

fn catalog_value_sets(catalog: &Catalog) -> HashMap<String, Vec<String>> {
    let mut values = catalog.value_sets.clone();
    for (name, value_set_values) in &catalog.object_axes {
        values.insert(name.clone(), value_set_values.clone());
    }
    values
}

fn catalog_value_set<'a>(catalog: &'a Catalog, name: &str) -> Option<&'a Vec<String>> {
    catalog
        .value_sets
        .get(name)
        .or_else(|| catalog.object_axes.get(name))
}

fn is_builtin_value_set(name: &str) -> bool {
    matches!(name, "directions" | "horizontal" | "vertical" | "layers")
}

fn looks_like_condition_expr(expr: &str) -> bool {
    expr.contains('(')
        || expr.contains("==")
        || expr.contains("!=")
        || expr.contains("<=")
        || expr.contains(">=")
        || expr.contains('<')
        || expr.contains('>')
        || expr
            .split_whitespace()
            .any(|token| matches!(token, "and" | "or"))
}

fn display_object_spec<'a>(
    tokens: &[&'a str],
    index: &mut usize,
    line: &str,
) -> Result<Option<&'a str>, DiagnosticReport> {
    if tokens.get(*index).copied() != Some("display") {
        return Ok(None);
    }
    *index += 1;
    let spec = tokens
        .get(*index)
        .copied()
        .ok_or_else(|| parse_error(line, "`display` must be followed by a display object"))?;
    if !is_display_role_token(spec) {
        return Err(parse_error(line, "display object must use an @ name"));
    }
    *index += 1;
    Ok(Some(spec))
}

fn is_display_role_token(token: &str) -> bool {
    puzzle_authoring::is_display_object_token(token)
}

fn validate_selector_alias_name(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    if is_display_role_token(value) || is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a qualified identifier or @name"),
        ))
    }
}

fn validate_rule_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if is_display_role_token(value) || is_qualified_identifier(value) {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "routine name must be a qualified identifier or @name",
        ))
    }
}

fn parse_layer_term(
    term: &str,
    line: &str,
    layer: u16,
    visual: bool,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let declared = if is_known_object_selector(
        term,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_groups,
    ) {
        let selector = resolve_object_selector(
            term,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &HashMap::new(),
        )?;
        for object in &selector.alternatives {
            assign_object_layer(*object, layer, catalog);
        }
        selector.alternatives
    } else {
        let value_sets = catalog_value_sets(catalog);
        define_object_spec(
            term,
            layer,
            None,
            line,
            &value_sets,
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    mark_visual_objects(&declared, visual || is_display_role_token(term), catalog);
    Ok(declared)
}

fn push_terms(objects: &mut Vec<ObjectId>, terms: &[ObjectId]) {
    for object in terms {
        push_unique_object(objects, *object);
    }
}

fn parse_scratch_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [name, "=", ty] => {
                parse_scratch_directive(name, Some(*ty), line, catalog)?;
                i += 1;
            }
            [spec] => {
                let (name, ty) = spec
                    .split_once('=')
                    .map_or((*spec, None), |(name, ty)| (name, Some(ty)));
                parse_scratch_directive(name, ty, line, catalog)?;
                i += 1;
            }
            [] => i += 1,
            _ => {
                return Err(parse_error(
                    line,
                    "scratch row must be: <name> or <name> = <type>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "scratch missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_scratch_directive(
    name: &str,
    ty: Option<&str>,
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let (name, kind, values) = if let Some(ty) = ty {
        validate_scratch_name(name, line)?;
        if ty.is_empty() {
            return Err(parse_error(line, "scratch type must not be empty"));
        }
        match ty {
            "int" => (name, ScratchKind::Int, Vec::new()),
            "bool" => (name, ScratchKind::Bool, Vec::new()),
            axis if catalog.value_sets.contains_key(axis)
                || catalog.object_axes.contains_key(axis) =>
            {
                (
                    name,
                    ScratchKind::Enum,
                    catalog
                        .value_sets
                        .get(axis)
                        .or_else(|| catalog.object_axes.get(axis))
                        .cloned()
                        .unwrap_or_default(),
                )
            }
            _ => return Err(parse_error(line, "unknown scratch type")),
        }
    } else {
        validate_scratch_name(name, line)?;
        (name, ScratchKind::Bool, Vec::new())
    };
    if catalog.scratch_names.contains_key(name) {
        return Err(parse_error(line, "duplicate scratch"));
    }
    let id = ScratchId(catalog.scratch_defs.len() as u16);
    let def = ScratchDef { id, kind, values };
    catalog.scratch_defs.push(def.clone());
    catalog.scratch_names.insert(name.to_string(), def);
    Ok(())
}

fn parse_layers_block(
    lines: &[String],
    start: usize,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        match tokens.as_slice() {
            ["for", binding, "in", sources @ ..] => {
                let value_sets = catalog_value_sets(catalog);
                let values = for_expansion_values(
                    sources,
                    &value_sets,
                    &catalog.numeric_global_defaults,
                    &lines[i],
                )?;
                validate_identifier(binding, &lines[i], "expansion binding")?;
                let (body_lines, next_i) = collect_statement_block_lines(lines, i + 1, &lines[i])?;
                for value in &values {
                    let mut expanded_lines = expand_for_binding_lines(
                        &body_lines,
                        binding,
                        value.axis.as_deref(),
                        &value.value,
                        &catalog.maps,
                    )?;
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    let parsed_i =
                        parse_layers_block(&expanded_lines, 0, named_layers, layer_count, catalog)?;
                    if parsed_i != expanded_lines.len() {
                        return Err(parse_error(&lines[i], "for expansion failed"));
                    }
                }
                i = next_i;
                continue;
            }
            ["for", ..] => {
                return Err(parse_error(
                    &lines[i],
                    "for directive must be: for <binding> in <source...>",
                ));
            }
            ["each", selectors @ ..] => {
                assign_selectors_to_separate_layers(
                    selectors,
                    &lines[i],
                    named_layers,
                    layer_count,
                    catalog,
                    false,
                )?;
            }
            [name, "=", selectors @ ..] => {
                let layer = layer_id_for_name(name, &lines[i], named_layers, layer_count, catalog)?;
                let objects =
                    define_or_assign_terms_to_layer(selectors, &lines[i], layer, catalog, false)?;
                validate_named_selector_role(
                    name,
                    &objects,
                    &catalog.visual_objects,
                    &lines[i],
                    "layer",
                )?;
                register_layer_tag_from_layer(name, layer, catalog);
            }
            _ => {
                assign_selectors_to_anonymous_layer(
                    &tokens,
                    &lines[i],
                    named_layers,
                    layer_count,
                    catalog,
                )?;
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start - 1],
            "layers missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn define_or_assign_terms_to_layer(
    terms: &[&str],
    line: &str,
    layer: u16,
    catalog: &mut Catalog,
    visual: bool,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    if terms.is_empty() {
        return Err(parse_error(
            line,
            "layer declaration must name at least one object",
        ));
    }

    let mut objects = Vec::new();
    let mut i = 0;
    while i < terms.len() {
        if let Some(term) = display_object_spec(terms, &mut i, line)? {
            let declared = parse_layer_term(term, line, layer, true, catalog)?;
            push_terms(&mut objects, &declared);
            continue;
        }
        let term = terms[i];
        let declared = parse_layer_term(term, line, layer, visual, catalog)?;
        push_terms(&mut objects, &declared);
        i += 1;
    }
    Ok(objects)
}

fn is_known_object_selector(
    selector: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> bool {
    let base = selector
        .split_once('{')
        .map_or(selector, |(base, _)| base)
        .split_once(':')
        .map_or(selector, |(base, _)| base);
    object_names.contains_key(selector)
        || object_groups.contains_key(selector)
        || object_schemas.contains_key(base)
        || (base == "*" && selector.contains(':') && !object_schemas.is_empty())
}

fn assign_selectors_to_separate_layers(
    selectors: &[&str],
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
    visual: bool,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    if selectors.is_empty() {
        return Err(parse_error(
            line,
            "each layer row must name at least one selector",
        ));
    }
    let selector_sets = selectors
        .iter()
        .map(|selector| resolve_or_declare_layer_selector(selector, line, visual, catalog))
        .collect::<Result<Vec<_>, _>>()?;
    let mut objects = Vec::new();
    for selector_set in selector_sets {
        for object in selector_set {
            let layer = anonymous_layer_id(named_layers, layer_count);
            assign_object_layer(object, layer, catalog);
            push_unique_object(&mut objects, object);
        }
    }
    Ok(objects)
}

fn resolve_or_declare_layer_selector(
    selector: &str,
    line: &str,
    visual: bool,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let declared = if is_known_object_selector(
        selector,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_groups,
    ) {
        resolve_object_selector(
            selector,
            line,
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.object_groups,
            &HashMap::new(),
        )?
        .alternatives
    } else {
        define_object_spec(
            selector,
            UNASSIGNED_LAYER,
            None,
            line,
            &catalog_value_sets(catalog),
            &mut catalog.object_schemas,
            &mut catalog.object_names,
            &mut catalog.object_labels,
            &mut catalog.object_layers,
            &mut catalog.object_defs,
            &mut catalog.render_chars,
            &mut catalog.char_objects,
        )?
    };
    mark_visual_objects(
        &declared,
        visual || is_display_role_token(selector),
        catalog,
    );
    Ok(declared)
}

fn assign_selectors_to_anonymous_layer(
    selectors: &[&str],
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &mut Catalog,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let layer = anonymous_layer_id(named_layers, layer_count);
    define_or_assign_terms_to_layer(selectors, line, layer, catalog, false)
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn mark_visual_objects(objects: &[ObjectId], visual: bool, catalog: &mut Catalog) {
    if !visual {
        return;
    }
    for object in objects {
        push_unique_object(&mut catalog.visual_objects, *object);
    }
}

fn assign_object_layer(object: ObjectId, layer: u16, catalog: &mut Catalog) {
    let layer = LayerId(layer);
    catalog.object_layers.insert(object, layer);
    if let Some(definition) = catalog
        .object_defs
        .iter_mut()
        .find(|definition| definition.id == object)
    {
        definition.layer_id = layer;
    }
}

fn register_layer_tag_from_layer(name: &str, layer: u16, catalog: &mut Catalog) {
    let layer = LayerId(layer);
    let objects = catalog
        .object_defs
        .iter()
        .filter_map(|definition| (definition.layer_id == layer).then_some(definition.id))
        .collect::<Vec<_>>();
    catalog.object_groups.insert(name.to_string(), objects);
}

fn validate_named_selector_role(
    name: &str,
    objects: &[ObjectId],
    visual_objects: &[ObjectId],
    line: &str,
    kind: &str,
) -> Result<(), DiagnosticReport> {
    let display_name = is_display_role_token(name);
    let has_main = objects
        .iter()
        .any(|object| !object.is_empty() && !visual_objects.contains(object));
    let has_display = objects.iter().any(|object| visual_objects.contains(object));
    if display_name && has_main {
        return Err(parse_error(
            line,
            &format!("@{kind} can only contain display objects"),
        ));
    }
    if !display_name && has_display {
        return Err(parse_error(
            line,
            &format!("{kind} containing display objects must use an @ name"),
        ));
    }
    Ok(())
}

fn validate_layer_role_separation(
    catalog: &Catalog,
    named_layers: &HashMap<String, u16>,
) -> Result<(), DiagnosticReport> {
    let mut layer_roles = HashMap::<LayerId, (bool, bool)>::new();
    for definition in &catalog.object_defs {
        if definition.layer_id.0 == UNASSIGNED_LAYER || definition.id.is_empty() {
            continue;
        }
        let visual = catalog.visual_objects.contains(&definition.id);
        let entry = layer_roles
            .entry(definition.layer_id)
            .or_insert((false, false));
        if visual {
            entry.1 = true;
        } else {
            entry.0 = true;
        }
    }

    for (layer, (has_main, has_visual)) in layer_roles {
        if has_main && has_visual {
            let name = named_layers
                .iter()
                .find_map(|(name, named_layer)| {
                    (*named_layer == layer.0 && !name.starts_with("__anonymous_layer_"))
                        .then_some(name.as_str())
                })
                .unwrap_or("<anonymous>");
            return Err(DiagnosticReport::error(format!(
                "layers cannot mix gameplay objects and display objects in the same storage layer ({name}); put display objects in a separate layer"
            )));
        }
    }
    Ok(())
}

fn refresh_layer_tags_and_value_sets(named_layers: &HashMap<String, u16>, catalog: &mut Catalog) {
    let mut layer_ids = catalog
        .object_defs
        .iter()
        .filter(|definition| definition.layer_id.0 != UNASSIGNED_LAYER)
        .map(|definition| definition.layer_id.0)
        .collect::<Vec<_>>();
    layer_ids.sort_unstable();
    layer_ids.dedup();

    let mut layer_names = layer_ids
        .into_iter()
        .map(|layer| {
            let name = named_layers
                .iter()
                .find_map(|(name, named_layer)| (*named_layer == layer).then_some(name.clone()))
                .unwrap_or_else(|| internal_layer_group_name(layer));
            (layer, name)
        })
        .collect::<Vec<_>>();
    layer_names.sort_by(|(left_layer, left_name), (right_layer, right_name)| {
        left_layer
            .cmp(right_layer)
            .then_with(|| left_name.cmp(right_name))
    });

    let values = layer_names
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<Vec<_>>();
    for (layer, name) in layer_names {
        register_layer_tag_from_layer(&name, layer, catalog);
    }
    catalog
        .value_sets
        .insert("layers".to_string(), values.clone());
}

fn internal_layer_group_name(layer: u16) -> String {
    format!("__anonymous_layer_{layer}")
}

fn anonymous_layer_id(
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
) -> u16 {
    let layer = named_layers.len() as u16;
    named_layers.insert(internal_layer_group_name(layer), layer);
    *layer_count = Some(layer.saturating_add(1));
    layer
}

fn layer_id_for_name(
    name: &str,
    line: &str,
    named_layers: &mut HashMap<String, u16>,
    layer_count: &mut Option<u16>,
    catalog: &Catalog,
) -> Result<u16, DiagnosticReport> {
    validate_selector_alias_name(name, line, "layer name")?;
    if let Some(layer) = named_layers.get(name).copied() {
        return Ok(layer);
    }
    if selector_name_conflicts(name, catalog) {
        return Err(parse_error(
            line,
            "layer name must not shadow another selector",
        ));
    }

    let layer = named_layers.len() as u16;
    named_layers.insert(name.to_string(), layer);
    *layer_count = Some(layer.saturating_add(1));
    Ok(layer)
}

fn selector_name_conflicts(name: &str, catalog: &Catalog) -> bool {
    selector_name_conflicts_with(
        name,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_groups,
    )
}

fn selector_name_conflicts_with(
    name: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> bool {
    object_names.contains_key(name)
        || object_schemas.contains_key(name)
        || object_groups.contains_key(name)
        || name.split_once(':').is_some_and(|(base, _)| {
            object_names.contains_key(base)
                || object_schemas.contains_key(base)
                || object_groups.contains_key(base)
        })
}

fn parse_legend_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        parse_legend_block_row(&lines[i], catalog, render_overlays, empty_char)?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "legend missing closing brace"));
    }

    Ok(i + 1)
}

fn parse_legend_block_row(
    line: &str,
    catalog: &mut Catalog,
    render_overlays: &mut OverlayDefs,
    empty_char: &mut Option<char>,
) -> Result<(), DiagnosticReport> {
    let tokens = split_header_tokens(line);
    if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
        return Err(parse_error(
            line,
            "legend row must be: <char> = <empty | selector...>",
        ));
    }

    let ch = parse_char(tokens.first(), line, "missing legend char")?;
    if tokens[2..] == ["empty"] {
        *empty_char = Some(ch);
        return Ok(());
    }

    let mut directive_tokens = vec!["legend"];
    directive_tokens.extend(tokens);
    parse_legend_directive(
        &directive_tokens,
        line,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &mut catalog.render_chars,
        &mut catalog.char_objects,
        render_overlays,
    )
}

fn add_input_name(
    name: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<InputId, DiagnosticReport> {
    if !is_identifier(name) {
        return Err(parse_error(line, "input name must be an identifier"));
    }
    if catalog.input_names.contains_key(name) {
        return Err(parse_error(line, "duplicate input"));
    }

    let id = InputId((catalog.input_names.len() + 1) as u16);
    catalog.input_names.insert(name.to_string(), id);
    catalog.input_labels.insert(id, name.to_string());
    Ok(id)
}

fn add_implicit_input_guards_to_catalog(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    display_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    let mut names = BTreeSet::<String>::new();
    for definition in definitions {
        collect_implicit_inputs_from_statements(&definition.statements, &mut names);
    }
    for statements in [
        main_statements,
        level_start_statements,
        level_clear_statements,
        display_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_implicit_inputs_from_statements(statements, &mut names);
    }
    for level in level_bodies {
        collect_implicit_inputs_from_statements(&level.level_start_statements, &mut names);
        collect_implicit_inputs_from_statements(&level.level_clear_statements, &mut names);
    }
    for (_, condition) in named_conditions.values() {
        collect_implicit_inputs_from_condition(condition, &mut names);
    }
    for name in names {
        if !catalog.input_names.contains_key(&name) {
            add_input_name(&name, "input guard", catalog)?;
        }
    }
    Ok(())
}

fn collect_implicit_inputs_from_statements(
    statements: &[StatementAst],
    names: &mut BTreeSet<String>,
) {
    for statement in statements {
        match statement {
            StatementAst::DisplayBlock(statements)
            | StatementAst::Block { statements, .. }
            | StatementAst::Fix { statements, .. } => {
                collect_implicit_inputs_from_statements(statements, names);
            }
            StatementAst::RepeatUntil {
                condition,
                statements,
            } => {
                collect_implicit_inputs_from_condition(condition, names);
                collect_implicit_inputs_from_statements(statements, names);
            }
            StatementAst::If {
                condition,
                then_statements,
                else_statements,
            } => {
                collect_implicit_inputs_from_condition(condition, names);
                collect_implicit_inputs_from_statements(then_statements, names);
                collect_implicit_inputs_from_statements(else_statements, names);
            }
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            } => {
                collect_implicit_inputs_from_statements(then_statements, names);
                collect_implicit_inputs_from_statements(else_statements, names);
            }
            StatementAst::Call { .. }
            | StatementAst::DisplayCall { .. }
            | StatementAst::DisplayRewrite(_)
            | StatementAst::Effect { .. }
            | StatementAst::Rewrite(_) => {}
        }
    }
}

fn collect_implicit_inputs_from_condition(condition: &ConditionAst, names: &mut BTreeSet<String>) {
    match condition {
        ConditionAst::All(conditions) | ConditionAst::Any(conditions) => {
            for condition in conditions {
                collect_implicit_inputs_from_condition(condition, names);
            }
        }
        ConditionAst::InputIs(name) => {
            names.insert(name.clone());
        }
        ConditionAst::InputIn(_)
        | ConditionAst::GlobalEquals { .. }
        | ConditionAst::GlobalCompare { .. }
        | ConditionAst::ConditionEquals { .. }
        | ConditionAst::ConditionNonZero(_)
        | ConditionAst::ConditionCompare { .. }
        | ConditionAst::InlineConditionValueEquals { .. }
        | ConditionAst::InlineConditionNonZero(_)
        | ConditionAst::InlineConditionCompare { .. } => {}
    }
}

fn add_default_restart_handler(main_statements: Option<&mut Vec<StatementAst>>) {
    let Some(statements) = main_statements else {
        return;
    };
    let mut inputs = BTreeSet::new();
    collect_implicit_inputs_from_statements(statements, &mut inputs);
    if inputs.contains("restart") {
        return;
    }
    statements.push(StatementAst::If {
        condition: ConditionAst::InputIs("restart".to_string()),
        then_statements: vec![StatementAst::Effect {
            effects: vec![EffectAst::Restart],
        }],
        else_statements: Vec::new(),
    });
}

fn add_cardinal_directions(
    line: &str,
    catalog: &mut Catalog,
    directions: &mut Vec<Direction>,
) -> Result<(), DiagnosticReport> {
    for (name, dx, dy) in [
        ("up", 0, -1),
        ("down", 0, 1),
        ("left", -1, 0),
        ("right", 1, 0),
    ] {
        let input = catalog
            .input_names
            .get(name)
            .copied()
            .map(Ok)
            .unwrap_or_else(|| add_input_name(name, line, catalog))?;
        if !directions.iter().any(|direction| direction.input == input) {
            directions.push(Direction { input, dx, dy });
        }
    }
    Ok(())
}

fn add_default_non_direction_inputs(
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    for name in ["restart"] {
        if !catalog.input_names.contains_key(name) {
            add_input_name(name, line, catalog)?;
        }
    }
    Ok(())
}

fn has_cardinal_input_names(input_names: &HashMap<String, InputId>) -> bool {
    ["up", "down", "left", "right"]
        .iter()
        .any(|name| input_names.contains_key(*name))
}

fn directions_include_all_cardinals(
    directions: &[Direction],
    input_names: &HashMap<String, InputId>,
) -> bool {
    ["up", "down", "left", "right"].iter().all(|name| {
        input_names
            .get(*name)
            .is_some_and(|input| directions.iter().any(|direction| direction.input == *input))
    })
}

fn parse_command_definition(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<(Option<Direction>, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let keyword = header.first().copied().unwrap_or("input");
    let name = expect(header.get(1), &lines[start], "missing input name")?;
    let input = if let Some(input) = catalog.input_names.get(name).copied() {
        input
    } else {
        add_input_name(name, &lines[start], catalog)?
    };

    match header.as_slice() {
        ["input", _] => {
            let next = start + 1;
            if next >= lines.len() || is_block_close_line(&lines[next]) {
                return Ok((None, next));
            }
            if !is_input_option(&split_header_tokens(&lines[next])) {
                return Ok((None, next));
            }

            let mut direction = None;
            let mut i = next;
            while i < lines.len() && !is_block_close_line(&lines[i]) {
                direction = Some(parse_input_option(&lines[i], input)?);
                i += 1;
            }
            if i >= lines.len() {
                return Err(parse_error(&lines[start], "input missing closing brace"));
            }
            Ok((direction, i + 1))
        }
        ["input", _, "direction", value] => {
            let (dx, dy) = named_direction_vector(value, &lines[start])?;
            Ok((Some(Direction { input, dx, dy }), start + 1))
        }
        _ => Err(parse_error(
            &lines[start],
            &format!("{keyword} must be: input <name> [direction <up|down|left|right>]"),
        )),
    }
}

fn is_input_option(tokens: &[&str]) -> bool {
    matches!(tokens, ["direction", ..])
}

fn parse_input_option(line: &str, input: InputId) -> Result<Direction, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["direction", value] => {
            let (dx, dy) = named_direction_vector(value, line)?;
            Ok(Direction { input, dx, dy })
        }
        _ => Err(parse_error(
            line,
            "input option must be: direction <up|down|left|right>",
        )),
    }
}

fn named_direction_vector(value: &str, line: &str) -> Result<(i16, i16), DiagnosticReport> {
    match value {
        "right" => Ok((1, 0)),
        "left" => Ok((-1, 0)),
        "up" => Ok((0, -1)),
        "down" => Ok((0, 1)),
        _ => Err(parse_error(line, "unknown direction name")),
    }
}

fn parse_scene_definition(
    lines: &[String],
    start: usize,
) -> Result<(SceneDef, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let name = match header.as_slice() {
        ["scene", "level_menu", ..] => {
            return Err(parse_error(
                &lines[start],
                "scene level_menu template is not supported; use scene <name> with layout { level_menu { ... } }",
            ));
        }
        ["scene", name] => *name,
        _ => {
            return Err(parse_error(
                &lines[start],
                "scene header must be: scene <name>[(param...)]",
            ));
        }
    };
    let (name, _params) = parse_scene_name_and_params(name, &lines[start])?;

    let mut screen = SceneDef {
        name: name.clone(),
        layout: SceneLayoutDef::default(),
        resources: SceneResources::default(),
        state: SceneStateDef::default(),
        components: Vec::new(),
        key_bindings: Vec::new(),
        routines: Vec::new(),
        transitions: Vec::new(),
        puzzle_rule: None,
    };
    let mut handler = Scene2dBlockHandler {
        screen: &mut screen,
    };
    let next = puzzle_scene::parse_scene_block_with_handler(
        lines,
        start + 1,
        &name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut handler,
    )?;
    validate_scene_routines(&screen)?;

    Ok((screen, next))
}

fn parse_scene_name_and_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<String>), DiagnosticReport> {
    let Some((name, params)) = value.split_once('(') else {
        validate_qualified_identifier(value, line, "scene name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(name, line, "scene name")?;
    let params = params
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "scene params must end with )"))?;
    let params = if params.trim().is_empty() {
        Vec::new()
    } else {
        params
            .split(',')
            .map(str::trim)
            .map(|param| {
                validate_identifier(param, line, "scene param")?;
                Ok(param.to_string())
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), params))
}

fn validate_scene_routines(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let routine_names = scene
        .routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<HashSet<_>>();
    for binding in &scene.key_bindings {
        validate_scene_effect_routine_calls(&binding.effect, &routine_names)?;
    }
    for transition in &scene.transitions {
        validate_scene_effect_routine_calls(&transition.effect, &routine_names)?;
    }
    for component in &scene.components {
        validate_scene_component_routine_calls(component, &routine_names)?;
    }

    let routines = scene
        .routines
        .iter()
        .map(|routine| (routine.name.as_str(), routine))
        .collect::<HashMap<_, _>>();
    let mut checked = HashSet::<String>::new();
    for routine in &scene.routines {
        validate_scene_routine_not_recursive(
            routine.name.as_str(),
            &routines,
            &mut Vec::new(),
            &mut checked,
        )?;
    }
    Ok(())
}

fn validate_scene_component_routine_calls(
    component: &SceneComponent,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            validate_scene_effect_routine_calls(&button.effect, routine_names)
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &container.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Conditional(conditional) => {
            for child in &conditional.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            for child in &conditional.else_children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::For(for_view) => {
            for child in &for_view.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::LevelMenu(menu) => {
            if let Some(effect) = &menu.action {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            for button in &menu.buttons {
                validate_scene_effect_routine_calls(&button.effect, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => Ok(()),
    }
}

fn validate_scene_effect_routine_calls(
    effect: &SceneEffect,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::RoutineCall(name) => {
            if !routine_names.contains(name) {
                return Err(DiagnosticReport::error(format!(
                    "unknown scene routine: {name}"
                )));
            }
            Ok(())
        }
        SceneEffect::Conditional { effect, .. } => {
            validate_scene_effect_routine_calls(effect, routine_names)
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::Message { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}

fn validate_scene_routine_not_recursive(
    name: &str,
    routines: &HashMap<&str, &SceneRoutineDef>,
    stack: &mut Vec<String>,
    checked: &mut HashSet<String>,
) -> Result<(), DiagnosticReport> {
    if checked.contains(name) {
        return Ok(());
    }
    if stack.iter().any(|active| active == name) {
        stack.push(name.to_string());
        return Err(DiagnosticReport::error(format!(
            "recursive scene routine call: {}",
            stack.join(" -> ")
        )));
    }
    let Some(routine) = routines.get(name) else {
        return Err(DiagnosticReport::error(format!(
            "unknown scene routine: {name}"
        )));
    };
    stack.push(name.to_string());
    for call in scene_effect_routine_calls(&routine.effect) {
        validate_scene_routine_not_recursive(call, routines, stack, checked)?;
    }
    stack.pop();
    checked.insert(name.to_string());
    Ok(())
}

fn scene_effect_routine_calls(effect: &SceneEffect) -> Vec<&str> {
    let mut calls = Vec::new();
    collect_scene_effect_routine_calls(effect, &mut calls);
    calls
}

fn collect_scene_effect_routine_calls<'a>(effect: &'a SceneEffect, calls: &mut Vec<&'a str>) {
    match effect {
        SceneEffect::RoutineCall(name) => calls.push(name.as_str()),
        SceneEffect::Conditional { effect, .. } => {
            collect_scene_effect_routine_calls(effect, calls);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                collect_scene_effect_routine_calls(effect, calls);
            }
        }
        _ => {}
    }
}

struct Scene2dBlockHandler<'a> {
    screen: &'a mut SceneDef,
}

impl puzzle_scene::SceneBlockHandler for Scene2dBlockHandler<'_> {
    type Error = DiagnosticReport;

    fn parse_state_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (state, next_i) = parse_scene_state_block(lines, start, SceneStateLifetime::Instance)?;
        self.screen.state.variables.extend(state.variables);
        self.screen.state.puzzles.extend(state.puzzles);
        Ok(next_i)
    }

    fn parse_layout_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (layout_block, next_i) = parse_screen_layout_block(lines, start)?;
        self.screen.layout = layout_block.layout;
        self.screen
            .state
            .variables
            .extend(layout_block.state.variables);
        self.screen.state.puzzles.extend(layout_block.state.puzzles);
        self.screen.components.extend(layout_block.components);
        Ok(next_i)
    }

    fn parse_inputs_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        Err(parse_error(
            &lines[start],
            "`inputs { ... }` was removed; use `keys { <key...> -> <routine-or-effect> }`",
        ))
    }

    fn parse_keys_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (bindings, next_i) = parse_scene_keys_block(lines, start)?;
        self.screen.key_bindings.extend(bindings);
        Ok(next_i)
    }

    fn parse_rules_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (block, next_i) = parse_screen_transitions_block(lines, start)?;
        self.screen.transitions.extend(block.transitions);
        if let Some(puzzle_rule) = block.puzzle_rule {
            self.screen.puzzle_rule = Some(puzzle_rule);
        }
        Ok(next_i)
    }

    fn parse_scene_start_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (transition, next_i) = parse_scene_lifecycle_block(lines, start)?;
        self.screen.transitions.push(transition);
        Ok(next_i)
    }

    fn parse_inline_directive(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let tokens = split_header_tokens(&lines[start]);
        match tokens.as_slice() {
            ["resources"] => parse_scene_resources_block(lines, start, &mut self.screen.resources),
            ["var", ..]
            | ["const", ..]
            | ["persistent", "var", ..]
            | ["persistent", "const", ..] => {
                match parse_scene_state_entry(&lines[start], SceneStateLifetime::Instance)? {
                    ParsedSceneStateEntry::Variable(variable) => {
                        self.screen.state.variables.push(variable);
                    }
                    ParsedSceneStateEntry::Puzzle(_) => {
                        return Err(parse_error(
                            &lines[start],
                            "var cannot define a puzzle slot",
                        ));
                    }
                }
                Ok(start + 1)
            }
            ["on_level_start" | "on_level_clear" | "on_last_level_clear"] => Err(parse_error(
                &lines[start],
                "level lifecycle blocks belong inside puzzle; scene lifecycle block must be on_scene_start",
            )),
            ["input", ..] => Err(parse_error(
                &lines[start],
                "scene input handlers are removed; use `keys { <key...> -> <routine-or-effect> }` and `routine <name> { ... }`",
            )),
            ["action", ..] => Err(parse_error(
                &lines[start],
                "`action` scene handlers were removed; use `routine <name> { ... }`",
            )),
            ["routine", ..] => {
                let (routine, next_i) = parse_scene_routine_block(lines, start)?;
                if self
                    .screen
                    .routines
                    .iter()
                    .any(|existing| existing.name == routine.name)
                {
                    return Err(parse_error(&lines[start], "duplicate scene routine"));
                }
                self.screen.routines.push(routine);
                Ok(next_i)
            }
            ["if", ..] => {
                let (transition, next_i) = parse_screen_condition_block(lines, start)?;
                self.screen.transitions.push(transition);
                Ok(next_i)
            }
            [] => Ok(start + 1),
            _ if scene_entry_is_component(&tokens) => {
                let (component, next_i) = parse_screen_component(lines, start)?;
                self.screen.components.push(component);
                Ok(next_i)
            }
            [other, ..] => Err(parse_error(
                &lines[start],
                &format!("unknown scene directive {other}"),
            )),
        }
    }
}

fn parse_scene_resources_block(
    lines: &[String],
    start: usize,
    resources: &mut SceneResources,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["levels", names @ ..] => {
                resources.levels = parse_resource_selection(names, &lines[i])?;
            }
            ["sprites", names @ ..] => {
                resources.sprites = parse_resource_selection(names, &lines[i])?;
            }
            [] => {}
            [other, ..] => {
                return Err(parse_error(
                    &lines[i],
                    &format!("unknown resources directive {other}"),
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "resources missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn parse_resource_selection(
    names: &[&str],
    line: &str,
) -> Result<ResourceSelection, DiagnosticReport> {
    match names {
        [] | ["all"] => Ok(ResourceSelection::All),
        ["none"] => Ok(ResourceSelection::Named(Vec::new())),
        names => {
            let mut selected = Vec::new();
            for name in names {
                if name.chars().any(|ch| matches!(ch, '{' | '}' | ',' | ';')) {
                    return Err(parse_error(
                        line,
                        "resource names must be whitespace-separated",
                    ));
                }
                selected.push((*name).to_string());
            }
            Ok(ResourceSelection::Named(selected))
        }
    }
}

struct ParsedScreenLayoutBlock {
    layout: SceneLayoutDef,
    state: ParsedScreenStateBlock,
    components: Vec<SceneComponent>,
}

fn parse_screen_layout_block(
    lines: &[String],
    start: usize,
) -> Result<(ParsedScreenLayoutBlock, usize), DiagnosticReport> {
    parse_screen_view_like_block(lines, start, "layout")
}

fn parse_screen_view_like_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(ParsedScreenLayoutBlock, usize), DiagnosticReport> {
    let layout = parse_scene_layout_from_header(&lines[start], block_name)?;
    let mut variables = Vec::new();
    let mut puzzles = Vec::new();
    let mut components = Vec::new();
    let mut hidden = Vec::<String>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if let Some((slot, visible)) = parse_layer_visibility(&lines[i])? {
            if visible {
                hidden.retain(|name| name != &slot);
                if puzzles
                    .iter()
                    .any(|puzzle: &ScenePuzzleDef| puzzle.name == slot)
                    && !components.iter().any(|component| {
                        scene_puzzle_component_source(component).is_some_and(|name| name == slot)
                    })
                {
                    components.push(scene_puzzle_component(slot));
                }
            } else {
                hidden.push(slot.clone());
                components.retain(|component| {
                    scene_puzzle_component_source(component) != Some(slot.as_str())
                });
            }
            i += 1;
            continue;
        }

        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["panel", ..]) {
            return Err(parse_error(&lines[i], "unknown layout directive panel"));
        }
        if matches!(tokens.as_slice(), ["if", ..]) {
            let (component, next_i) = parse_view_if_component(lines, i)?;
            components.push(component);
            i = next_i;
            continue;
        }
        if scene_entry_is_component(&tokens) || matches!(tokens.as_slice(), ["puzzle", ..]) {
            let (component, next_i) = parse_screen_component(lines, i)?;
            components.push(component);
            i = next_i;
            continue;
        }

        if lines[i].contains('=') {
            match parse_scene_state_entry(&lines[i], SceneStateLifetime::Instance)? {
                ParsedSceneStateEntry::Puzzle(puzzle) => {
                    if !hidden.iter().any(|name| name == &puzzle.name) {
                        components.push(scene_puzzle_component(puzzle.name.clone()));
                    }
                    puzzles.push(puzzle);
                }
                ParsedSceneStateEntry::Variable(variable) => variables.push(variable),
            }
            i += 1;
            continue;
        }

        let (component, next_i) = parse_screen_component(lines, i)?;
        components.push(component);
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            &format!("{block_name} missing closing brace"),
        ));
    }

    Ok((
        ParsedScreenLayoutBlock {
            layout,
            state: ParsedScreenStateBlock { variables, puzzles },
            components,
        },
        i + 1,
    ))
}

fn parse_scene_layout_from_header(
    line: &str,
    keyword: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_header(line, keyword, puzzle_scene::SceneBlockSyntax::Braces)
        .map_err(DiagnosticReport::from)
}

fn parse_layer_visibility(line: &str) -> Result<Option<(String, bool)>, DiagnosticReport> {
    let Some((name, value)) = line.split_once('=') else {
        return Ok(None);
    };
    let Some(slot) = name.trim().strip_suffix(".visible") else {
        return Ok(None);
    };
    validate_qualified_identifier(slot.trim(), line, "layer name")?;
    match value.trim() {
        "true" => Ok(Some((slot.trim().to_string(), true))),
        "false" => Ok(Some((slot.trim().to_string(), false))),
        _ => Err(parse_error(line, "layer visibility must be true or false")),
    }
}

fn scene_frame_component(kind: impl Into<String>, source: impl Into<String>) -> SceneComponent {
    scene_frame_component_with_layout(kind, source, SceneLayoutDef::default())
}

fn scene_frame_component_with_layout(
    kind: impl Into<String>,
    source: impl Into<String>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    SceneComponent::Frame(puzzle_scene::FrameComponent {
        kind: kind.into(),
        source: source.into(),
        inputs: Vec::new(),
        layout,
    })
}

fn scene_puzzle_component(source: impl Into<String>) -> SceneComponent {
    scene_frame_component("puzzle", source)
}

fn scene_puzzle_component_source(component: &SceneComponent) -> Option<&str> {
    match component {
        SceneComponent::Frame(frame) => Some(frame.source.as_str()),
        _ => None,
    }
}

fn parse_screen_components_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(Vec<SceneComponent>, usize), DiagnosticReport> {
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        lines,
        start + 1,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((components, next))
}

fn parse_screen_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, component) = puzzle_scene::parse_scene_component_at(
        lines,
        start,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((component, next))
}

fn build_scene_container_component(
    kind: puzzle_scene::SceneComponentKind,
    children: Vec<SceneComponent>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    match kind {
        puzzle_scene::SceneComponentKind::Row => {
            SceneComponent::Row(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Column => {
            SceneComponent::Column(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Box => {
            SceneComponent::Box(SceneContainerDef { children, layout })
        }
        _ => unreachable!("shared scene parser only builds generic containers"),
    }
}

fn parse_screen_leaf_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    match tokens.as_slice() {
        ["puzzle", "current_level"] => Err(parse_error(
            &lines[start],
            "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
        )),
        ["puzzle", state_name, attrs @ ..] => {
            if *state_name == "current_level" {
                return Err(parse_error(
                    &lines[start],
                    "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
                ));
            }
            if !is_identifier(state_name) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle state name must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle", (*state_name).to_string(), layout),
                start + 1,
            ))
        }
        ["frame", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("frame", (*source).to_string(), layout),
                start + 1,
            ))
        }
        ["puzzle3", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle3 frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle3", (*source).to_string(), layout),
                start + 1,
            ))
        }
        ["text", ..] => Ok((parse_text_component(&lines[start])?, start + 1)),
        ["title", ..] => Ok((parse_title_component(&lines[start], true)?, start + 1)),
        ["subtitle", ..] => Ok((parse_title_component(&lines[start], false)?, start + 1)),
        ["button", ..] => parse_button_component(lines, start),
        ["choice", ..] => parse_choice_component(lines, start),
        ["if", ..] => parse_view_if_component(lines, start),
        ["for", ..] => parse_for_component(lines, start),
        ["level_menu"] => {
            let (menu, next_i) = parse_level_menu_component(lines, start)?;
            Ok((SceneComponent::LevelMenu(menu), next_i))
        }
        ["level_menu", ..] => Err(parse_error(
            &lines[start],
            "level_menu takes no inline source or effect; use scene resources to choose levels",
        )),
        [state_name] if is_identifier(state_name) => Ok((
            scene_frame_component("puzzle", (*state_name).to_string()),
            start + 1,
        )),
        [other, ..] => Err(parse_error(
            &lines[start],
            &format!("unknown layout directive {other}"),
        )),
        [] => Err(parse_error(&lines[start], "empty layout directive")),
    }
}

fn parse_scene_layout_attrs_for_line(
    attrs: &[&str],
    line: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_attrs(attrs).map_err(|error| parse_error(line, &error.message))
}

fn parse_title_component(line: &str, is_title: bool) -> Result<SceneComponent, DiagnosticReport> {
    let keyword = if is_title { "title" } else { "subtitle" };
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(line, "title must be: title <text-or-path>"));
    };
    let rest = rest.trim();
    let content = if rest.is_empty() {
        SceneExpr::Path(vec![keyword.to_string()])
    } else {
        parse_scene_expr(rest, line)?
    };
    let title = SceneTitleDef {
        content,
        layout: SceneLayoutDef::default(),
    };
    Ok(if is_title {
        SceneComponent::Title(title)
    } else {
        SceneComponent::Subtitle(title)
    })
}

fn parse_text_component(line: &str) -> Result<SceneComponent, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("text") else {
        return Err(parse_error(
            line,
            "text must be: text \"<text>\" | text <state>",
        ));
    };
    let rest = rest.trim();
    if let Some(text) = parse_quoted_text(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Literal(text),
            layout: SceneLayoutDef::default(),
        }));
    }
    if let Some(path) = parse_view_path(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Path(path),
            layout: SceneLayoutDef::default(),
        }));
    }
    Err(parse_error(
        line,
        "text must be: text \"<text>\" | text <state>",
    ))
}

fn parse_button_like_def(
    lines: &[String],
    start: usize,
    keyword: &str,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    let line = &lines[start];
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    }

    let (label, effect, next_i) = if rest.contains('=') {
        return Err(parse_error(
            line,
            &format!("{keyword} command must use `->`; `=` action assignment was removed"),
        ));
    } else if let Some((label, effect)) = rest.split_once("->") {
        let effect_text = effect.trim();
        let (effect, next_i) = parse_scene_effect_with_optional_block(effect_text, lines, start)?;
        (parse_button_label(label.trim(), line)?, effect, next_i)
    } else {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    };

    Ok((
        SceneButtonDef {
            label,
            effect,
            layout: SceneLayoutDef::default(),
        },
        next_i,
    ))
}

fn parse_button_def(
    lines: &[String],
    start: usize,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    parse_button_like_def(lines, start, "button")
}

fn parse_button_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (button, next_i) = parse_button_def(lines, start)?;
    Ok((SceneComponent::Button(button), next_i))
}

fn parse_choice_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (choice, next_i) = parse_button_like_def(lines, start, "choice")?;
    Ok((SceneComponent::Choice(choice), next_i))
}

fn parse_view_if_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "layout condition must be: if <condition>"))?
        .trim();
    validate_screen_condition(condition, line)?;
    let (entry, next_i) = collect_authoring_entry(lines, start)?;
    let body = &entry[1..entry.len().saturating_sub(1)];
    let (else_body, next_i) = collect_view_else_body(lines, next_i, line)?;
    if body.is_empty() {
        return Err(parse_error(
            line,
            "layout condition requires at least one component",
        ));
    }
    let children = parse_screen_component_body(body, "if")?;
    let else_children = if else_body.is_empty() {
        Vec::new()
    } else {
        parse_screen_component_body(&else_body, "else")?
    };
    Ok((
        SceneComponent::Conditional(SceneConditionalDef {
            condition: condition.to_string(),
            children,
            else_children,
        }),
        next_i,
    ))
}

fn collect_view_else_body(
    lines: &[String],
    start: usize,
    header_line: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    if !next_line_is_else(lines, start) {
        return Ok((Vec::new(), start));
    }

    let mut body = Vec::new();
    let mut block_stack = vec![AuthoringBlockKind::Other];
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            let closed = block_stack
                .pop()
                .ok_or_else(|| parse_error(line, "closing brace without layout block"))?;
            i += 1;
            if block_stack.is_empty() {
                return Ok((body, i));
            }
            body.push(line.clone());
            if closed == AuthoringBlockKind::If && next_line_is_else(lines, i) {
                body.push(lines[i].clone());
                i += 1;
                block_stack.push(AuthoringBlockKind::Other);
            }
            continue;
        }
        if let Some(kind) = authoring_nested_block_kind(&tokens, line) {
            block_stack.push(kind);
        }
        body.push(line.clone());
        i += 1;
    }
    Err(parse_error(
        header_line,
        "layout else block missing closing brace",
    ))
}

fn parse_screen_component_body(
    body: &[String],
    block_name: &str,
) -> Result<Vec<SceneComponent>, DiagnosticReport> {
    let mut lines = body.to_vec();
    lines.push(BLOCK_CLOSE.to_string());
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next) = parse_screen_leaf_component(lines, index)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        &lines,
        0,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    debug_assert_eq!(next, lines.len());
    Ok(components)
}

fn parse_for_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["for", binding, "in", source] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "for layout must be: for <item> in <source>",
        ));
    };
    if !is_identifier(binding) {
        return Err(parse_error(
            &lines[start],
            "for binding must be an identifier",
        ));
    }
    let source = parse_for_source(source, &lines[start])?;
    let (children, next_i) = parse_screen_components_block(lines, start, "for")?;
    Ok((
        SceneComponent::For(SceneForDef {
            binding: (*binding).to_string(),
            source,
            children,
        }),
        next_i,
    ))
}

fn parse_for_source(value: &str, line: &str) -> Result<ForSource, DiagnosticReport> {
    if value == "levels" {
        return Ok(ForSource::Levels);
    }
    if is_identifier(value) {
        return Ok(ForSource::State(value.to_string()));
    }
    Err(parse_error(
        line,
        "for source must be levels or a state identifier",
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneEffectCommandSyntax {
    Plain,
    InputTarget,
    ComponentEffectTarget,
    SceneTarget,
    AssetTarget,
    OptionalAssetTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RewriteEffectCommandSyntax {
    Effect,
    Emission,
}

pub(crate) fn scene_effect_command_syntax(token: &str) -> Option<SceneEffectCommandSyntax> {
    match token {
        "input" => Some(SceneEffectCommandSyntax::InputTarget),
        "component_effect" => Some(SceneEffectCommandSyntax::ComponentEffectTarget),
        "goto" | "start" => Some(SceneEffectCommandSyntax::SceneTarget),
        "sfx" | "play_music" => Some(SceneEffectCommandSyntax::AssetTarget),
        "pause_music" | "resume_music" | "stop_music" => {
            Some(SceneEffectCommandSyntax::OptionalAssetTarget)
        }
        "apply"
        | "clear_history"
        | "clear_undo_history"
        | "clear_game_progress"
        | "clear"
        | "copy"
        | "load"
        | "message"
        | "set"
        | "wait" => Some(SceneEffectCommandSyntax::Plain),
        _ => None,
    }
}

pub(crate) fn scene_effect_semantic_tokens(tokens: &[SourceToken]) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&scene_effect_surface_document(tokens).semantic_tokens)
}

fn project_surface_semantic_tokens(
    tokens: &[SurfaceSemanticToken],
) -> Vec<semantic::SemanticToken> {
    tokens
        .iter()
        .map(|token| semantic::SemanticToken {
            start: token.span.start,
            end: token.span.end,
            kind: match token.kind {
                SurfaceSemanticKind::Keyword => semantic::SemanticKind::Keyword,
                SurfaceSemanticKind::Literal => semantic::SemanticKind::Literal,
                SurfaceSemanticKind::Binding => semantic::SemanticKind::Binding,
                SurfaceSemanticKind::Effect => semantic::SemanticKind::Effect,
                SurfaceSemanticKind::Emission => semantic::SemanticKind::Emission,
                SurfaceSemanticKind::Input => semantic::SemanticKind::Input,
                SurfaceSemanticKind::State => semantic::SemanticKind::State,
                SurfaceSemanticKind::Condition => semantic::SemanticKind::Condition,
                SurfaceSemanticKind::Scene => semantic::SemanticKind::Scene,
                SurfaceSemanticKind::Asset => semantic::SemanticKind::Asset,
                SurfaceSemanticKind::Number => semantic::SemanticKind::Number,
                SurfaceSemanticKind::String => semantic::SemanticKind::String,
            },
        })
        .collect()
}

fn scene_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    if let Some(parts) = split_scene_effect_token_sequence(tokens) {
        let mut sink = SurfaceSink::default();
        for part in parts {
            sink.extend(scene_effect_surface_document(part));
        }
        return sink.into_document();
    }

    let mut sink = SurfaceSink::default();
    let Some(first) = tokens.first() else {
        return sink.into_document();
    };
    let effect_span = source_tokens_span(tokens);

    if first.text.starts_with("cursor.") {
        add_cursor_scene_effect_token(&mut sink, first);
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text.contains('.') {
        let mut parts = first.text.split('.');
        if let Some(target) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, target, SurfaceSemanticKind::Scene);
        }
        if let Some(effect) = parts.next() {
            add_scene_effect_token_part(&mut sink, first, effect, SurfaceSemanticKind::Effect);
        }
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    if first.text == "start" && add_level_flow_scene_effect_tokens(tokens, &mut sink) {
        return surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span);
    }

    match scene_effect_command_syntax(&first.text) {
        Some(SceneEffectCommandSyntax::InputTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(input) = tokens.get(1) {
                add_scene_command_token(&mut sink, input);
            }
        }
        Some(SceneEffectCommandSyntax::ComponentEffectTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(effect) = tokens.get(1) {
                add_scene_command_token(&mut sink, effect);
            }
        }
        Some(SceneEffectCommandSyntax::SceneTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(scene) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, scene, SurfaceSemanticKind::Scene);
            }
        }
        Some(SceneEffectCommandSyntax::AssetTarget) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::OptionalAssetTarget) => {
            add_scene_effect_token_range(&mut sink, first, SurfaceSemanticKind::Effect);
            if let Some(asset) = tokens.get(1) {
                add_scene_effect_token_range(&mut sink, asset, SurfaceSemanticKind::Asset);
            }
        }
        Some(SceneEffectCommandSyntax::Plain) => {
            let kind = scene_effect_command_kind(&first.text);
            add_scene_effect_token_range(&mut sink, first, kind);
        }
        None => {}
    }

    surface_document_with_node(sink, SurfaceNodeKind::SceneEffect, effect_span)
}

fn source_tokens_span(tokens: &[SourceToken]) -> Option<SourceSpan> {
    let start = tokens.first()?.start;
    let end = tokens.last()?.end;
    (start < end).then_some(SourceSpan { start, end })
}

fn surface_document_with_node(
    mut sink: SurfaceSink,
    kind: SurfaceNodeKind,
    span: Option<SourceSpan>,
) -> SurfaceDocument {
    if sink.has_semantic_tokens()
        && let Some(span) = span
    {
        sink.node(kind, span);
    }
    sink.into_document()
}

fn scene_effect_command_kind(token: &str) -> SurfaceSemanticKind {
    if matches!(
        token,
        "sfx" | "play_music" | "pause_music" | "resume_music" | "stop_music"
    ) {
        return SurfaceSemanticKind::Effect;
    }
    if matches!(
        rewrite_effect_command_syntax(token),
        Some(RewriteEffectCommandSyntax::Emission)
    ) {
        SurfaceSemanticKind::Emission
    } else {
        SurfaceSemanticKind::Effect
    }
}

fn add_level_flow_scene_effect_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    match tokens {
        [command, levels, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        [command, levels, scope, in_keyword, scene]
            if levels.text == "levels" && in_keyword.text == "in" =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, levels, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scope, SurfaceSemanticKind::Scene);
            add_scene_effect_token_range(sink, in_keyword, SurfaceSemanticKind::Keyword);
            add_scene_effect_token_range(sink, scene, SurfaceSemanticKind::Scene);
            true
        }
        _ => false,
    }
}

fn add_scene_command_token(sink: &mut SurfaceSink, token: &SourceToken) {
    if let Some(cursor_offset) = token.text.find("cursor.") {
        add_scene_effect_token_subrange(
            sink,
            token,
            cursor_offset,
            cursor_offset + "cursor".len(),
            SurfaceSemanticKind::State,
        );
        let value_start = cursor_offset + "cursor.".len();
        if let Some(value_end) = scene_effect_identifier_end(&token.text, value_start) {
            let value = &token.text[value_start..value_end];
            let kind = if matches!(value, "prev" | "next") {
                SurfaceSemanticKind::Effect
            } else {
                SurfaceSemanticKind::Literal
            };
            add_scene_effect_token_subrange(sink, token, value_start, value_end, kind);
        }
    }

    let Some((first_start, first_end)) = scene_effect_first_identifier_bounds(&token.text) else {
        return;
    };
    let after_first = &token.text[first_end..];
    if after_first.starts_with('.') {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Scene,
        );
        let command_start = first_end + 1;
        if let Some(command_end) = scene_effect_identifier_end(&token.text, command_start) {
            add_scene_effect_token_subrange(
                sink,
                token,
                command_start,
                command_end,
                SurfaceSemanticKind::Effect,
            );
        }
    } else {
        add_scene_effect_token_subrange(
            sink,
            token,
            first_start,
            first_end,
            SurfaceSemanticKind::Input,
        );
        if after_first.starts_with(':') {
            let binding_start = first_end + 1;
            if let Some(binding_end) = scene_effect_identifier_end(&token.text, binding_start) {
                add_scene_effect_token_subrange(
                    sink,
                    token,
                    binding_start,
                    binding_end,
                    SurfaceSemanticKind::Binding,
                );
            }
        }
    }
}

fn add_cursor_scene_effect_token(sink: &mut SurfaceSink, token: &SourceToken) {
    add_scene_effect_token_part(sink, token, "cursor", SurfaceSemanticKind::State);
    if let Some((_, tail)) = token.text.split_once('.') {
        let kind = if matches!(tail, "prev" | "next") {
            SurfaceSemanticKind::Effect
        } else {
            SurfaceSemanticKind::Literal
        };
        add_scene_effect_token_part(sink, token, tail, kind);
    }
}

fn add_scene_effect_token_range(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    kind: SurfaceSemanticKind,
) {
    let Some((start, end)) = scene_effect_identifier_bounds(token) else {
        return;
    };
    sink.mark(SourceSpan { start, end }, kind);
}

fn add_scene_effect_token_part(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    part: &str,
    kind: SurfaceSemanticKind,
) {
    if part.is_empty() {
        return;
    }
    if let Some(relative) = token.text.find(part) {
        sink.mark(
            SourceSpan {
                start: token.start + relative,
                end: token.start + relative + part.len(),
            },
            kind,
        );
    }
}

fn add_scene_effect_token_subrange(
    sink: &mut SurfaceSink,
    token: &SourceToken,
    relative_start: usize,
    relative_end: usize,
    kind: SurfaceSemanticKind,
) {
    if relative_start >= relative_end || relative_end > token.text.len() {
        return;
    }
    sink.mark(
        SourceSpan {
            start: token.start + relative_start,
            end: token.start + relative_end,
        },
        kind,
    );
}

fn scene_effect_first_identifier_bounds(value: &str) -> Option<(usize, usize)> {
    let start = value
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    scene_effect_identifier_end(value, start).map(|end| (start, end))
}

fn scene_effect_identifier_end(value: &str, start: usize) -> Option<usize> {
    if start >= value.len() {
        return None;
    }
    let mut end = start;
    for (offset, ch) in value[start..].char_indices() {
        if offset == 0 {
            if !scene_effect_is_word_start(ch) {
                return None;
            }
        } else if !scene_effect_is_word_continue(ch) || matches!(ch, ':' | '.') {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    (end > start).then_some(end)
}

fn scene_effect_identifier_bounds(token: &SourceToken) -> Option<(usize, usize)> {
    let start_offset = token
        .text
        .char_indices()
        .find_map(|(index, ch)| scene_effect_is_word_start(ch).then_some(index))?;
    let end_offset = token.text.char_indices().rev().find_map(|(index, ch)| {
        scene_effect_is_word_continue(ch).then_some(index + ch.len_utf8())
    })?;
    let start = token.start + start_offset;
    let end = token.start + end_offset;
    debug_assert!(end <= token.end);
    (start < end).then_some((start, end))
}

fn scene_effect_is_word_start(ch: char) -> bool {
    ch == '@' || ch == '_' || ch.is_ascii_alphabetic()
}

fn scene_effect_is_word_continue(ch: char) -> bool {
    ch == '@' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

impl EffectAst {
    fn command_syntax(&self) -> RewriteEffectCommandSyntax {
        match self {
            EffectAst::PlaySfx { .. }
            | EffectAst::PlayMusic { .. }
            | EffectAst::PauseMusic { .. }
            | EffectAst::ResumeMusic { .. }
            | EffectAst::StopMusic { .. }
            | EffectAst::Wait { .. }
            | EffectAst::WaitAnimation
            | EffectAst::Message { .. }
            | EffectAst::Scene(_) => RewriteEffectCommandSyntax::Emission,
            EffectAst::Cancel
            | EffectAst::Win
            | EffectAst::Restart
            | EffectAst::NextLevel
            | EffectAst::Again
            | EffectAst::Checkpoint
            | EffectAst::ClearCheckpoint
            | EffectAst::UpdateGlobal { .. } => RewriteEffectCommandSyntax::Effect,
        }
    }
}

pub(crate) fn rewrite_effect_command_syntax(token: &str) -> Option<RewriteEffectCommandSyntax> {
    let probe = match token {
        "again" | "cancel" | "win" | "restart" | "next_level" | "checkpoint"
        | "clear_checkpoint" | "wait" => token.to_string(),
        "message" => "message \"text\"".to_string(),
        "sfx" => "sfx __highlight_probe".to_string(),
        "play_music" => "play_music __highlight_probe".to_string(),
        "pause_music" => token.to_string(),
        "resume_music" => token.to_string(),
        "stop_music" => token.to_string(),
        "set" => "set __highlight_probe = 0".to_string(),
        _ => return None,
    };
    parse_rewrite_effect_value(&probe, &probe)
        .ok()
        .and_then(|effects| effects.into_iter().next())
        .map(|effect| effect.command_syntax())
}

pub(crate) fn rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
) -> Vec<semantic::SemanticToken> {
    project_surface_semantic_tokens(&rewrite_effect_surface_document(tokens).semantic_tokens)
}

fn rewrite_effect_surface_document(tokens: &[SourceToken]) -> SurfaceDocument {
    let mut sink = SurfaceSink::default();
    let effect_span = source_tokens_span(tokens);
    add_rewrite_effect_semantic_tokens(tokens, &mut sink);
    surface_document_with_node(sink, SurfaceNodeKind::RewriteEffect, effect_span)
}

fn add_rewrite_effect_semantic_tokens(tokens: &[SourceToken], sink: &mut SurfaceSink) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };

    if first.text == "message" {
        add_scene_effect_token_range(sink, first, SurfaceSemanticKind::Emission);
        if tokens.len() > 1 {
            let text_start = tokens[1].start;
            let text_end = tokens.last().map(|token| token.end).unwrap_or(text_start);
            if text_start < text_end {
                sink.mark(
                    SourceSpan {
                        start: text_start,
                        end: text_end,
                    },
                    SurfaceSemanticKind::String,
                );
            }
        }
        return true;
    }

    if tokens.len() > 2
        && tokens
            .iter()
            .any(|token| is_rewrite_effect_command_token(&token.text))
    {
        return add_simple_rewrite_effect_semantic_tokens(tokens, sink);
    }

    match tokens {
        [command] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Effect,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            true
        }
        [command]
            if matches_rewrite_effect_command(
                &command.text,
                RewriteEffectCommandSyntax::Emission,
            ) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            true
        }
        [command, duration] if command.text == "wait" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Emission);
            add_scene_effect_token_range(sink, duration, SurfaceSemanticKind::Number);
            true
        }
        [command, asset] if command.text == "sfx" => {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
            true
        }
        [command, name, op, value]
            if command.text == "set" && is_global_update_operator(&op.text) =>
        {
            add_scene_effect_token_range(sink, command, SurfaceSemanticKind::Effect);
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
            true
        }
        [name, op, value] if is_global_update_operator(&op.text) => {
            add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
            add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
            true
        }
        _ => false,
    }
}

fn add_simple_rewrite_effect_semantic_tokens(
    tokens: &[SourceToken],
    sink: &mut SurfaceSink,
) -> bool {
    let mut index = 0usize;
    let mut parsed_any = false;
    while index < tokens.len() {
        match tokens[index].text.to_ascii_lowercase().as_str() {
            "cancel" | "win" | "restart" | "next_level" | "again" | "checkpoint"
            | "clear_checkpoint" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                index += 1;
                parsed_any = true;
            }
            "wait" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Number,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "sfx" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "play_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if let Some(asset) = tokens.get(index + 1) {
                    add_scene_effect_token_range(sink, asset, SurfaceSemanticKind::Asset);
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "pause_music" | "resume_music" | "stop_music" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Emission);
                if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(&tokens[index + 1].text)
                {
                    add_scene_effect_token_range(
                        sink,
                        &tokens[index + 1],
                        SurfaceSemanticKind::Asset,
                    );
                    index += 2;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            "set" => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::Effect);
                if let (Some(name), Some(value)) = (tokens.get(index + 1), tokens.get(index + 3)) {
                    add_scene_effect_token_range(sink, name, SurfaceSemanticKind::State);
                    add_scene_effect_token_range(sink, value, SurfaceSemanticKind::Number);
                    index += 4;
                } else {
                    index += 1;
                }
                parsed_any = true;
            }
            _ if index + 2 < tokens.len() && is_global_update_operator(&tokens[index + 1].text) => {
                add_scene_effect_token_range(sink, &tokens[index], SurfaceSemanticKind::State);
                add_scene_effect_token_range(sink, &tokens[index + 2], SurfaceSemanticKind::Number);
                index += 3;
                parsed_any = true;
            }
            _ => {
                return parsed_any;
            }
        }
    }
    parsed_any
}

fn matches_rewrite_effect_command(token: &str, syntax: RewriteEffectCommandSyntax) -> bool {
    rewrite_effect_command_syntax(&token.to_ascii_lowercase()) == Some(syntax)
}

pub(crate) fn rewrite_direction_prefix_token_index(tokens: &[&str]) -> Option<usize> {
    let mut index = 0usize;
    while tokens
        .get(index)
        .is_some_and(|token| rewrite_application_keyword(token))
    {
        index += 1;
    }
    let direction = tokens.get(index).copied()?;
    if !direction_word(direction) {
        return None;
    }
    tokens
        .get(index + 1)
        .is_some_and(|token| matches!(*token, "[" | "{"))
        .then_some(index)
}

fn rewrite_application_keyword(value: &str) -> bool {
    matches!(
        value,
        "fix" | "once" | "once_all" | "once_per_level" | "repeat"
    )
}

fn direction_word(value: &str) -> bool {
    matches!(value, "up" | "down" | "left" | "right")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoundSettingValueSyntax {
    String,
    Number,
}

pub(crate) fn sound_setting_value_syntax(key: &str) -> Option<SoundSettingValueSyntax> {
    match key {
        "seed" | "type" => Some(SoundSettingValueSyntax::String),
        "height" | "tone" | "bars" | "bpm" | "volume" => Some(SoundSettingValueSyntax::Number),
        _ => None,
    }
}

pub(crate) const SFX_SOUND_SETTING_OPTIONS: &[&str] = &["seed", "type"];
pub(crate) const MUSIC_SOUND_SETTING_OPTIONS: &[&str] =
    &["seed", "height", "bars", "bpm", "volume"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapHeaderTokenSyntax {
    Keyword,
    Name,
    Axis,
}

pub(crate) fn map_header_token_syntax(
    tokens: &[&str],
    index: usize,
) -> Option<MapHeaderTokenSyntax> {
    if !matches!(tokens, ["map", _, _]) {
        return None;
    }
    match index {
        0 => Some(MapHeaderTokenSyntax::Keyword),
        1 => Some(MapHeaderTokenSyntax::Name),
        2 => Some(MapHeaderTokenSyntax::Axis),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneStateLhsSyntax {
    PuzzleSlot,
    Variable,
}

pub(crate) fn scene_state_lhs_syntax(tokens: &[&str]) -> Option<(usize, SceneStateLhsSyntax)> {
    match tokens {
        [name, "=", "puzzle", ..] if is_identifier(name) => {
            Some((0, SceneStateLhsSyntax::PuzzleSlot))
        }
        ["var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        ["persistent", "var" | "const", name, "=", ..] if is_identifier(name) => {
            Some((2, SceneStateLhsSyntax::Variable))
        }
        ["persistent", name, "=", ..] if is_identifier(name) => {
            Some((1, SceneStateLhsSyntax::Variable))
        }
        [name, "=", ..] if is_identifier(name) => Some((0, SceneStateLhsSyntax::Variable)),
        _ => None,
    }
}

pub(crate) fn metadata_directive_value_token_index(tokens: &[&str]) -> Option<usize> {
    matches!(
        tokens,
        ["title" | "subtitle" | "author" | "homepage", _, ..]
    )
    .then_some(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LevelPathPartSyntax {
    Owner,
    TextProperty,
    NumberProperty,
    ConditionProperty,
}

pub(crate) fn level_path_part_syntax(parts: &[&str], index: usize) -> Option<LevelPathPartSyntax> {
    match parts {
        ["level", property] => match index {
            0 => None,
            1 => level_property_syntax(property),
            _ => None,
        },
        [_, "level", property] => match index {
            0 => Some(LevelPathPartSyntax::Owner),
            1 => None,
            2 => level_property_syntax(property),
            _ => None,
        },
        _ => None,
    }
}

fn level_property_syntax(property: &str) -> Option<LevelPathPartSyntax> {
    match property {
        "name" | "label" | "title" => Some(LevelPathPartSyntax::TextProperty),
        "index" | "num" => Some(LevelPathPartSyntax::NumberProperty),
        "cleared" | "solved" | "last" | "has_next" => Some(LevelPathPartSyntax::ConditionProperty),
        _ => None,
    }
}

fn parse_scene_effect_with_optional_block(
    value: &str,
    lines: &[String],
    start: usize,
) -> Result<(SceneEffect, usize), DiagnosticReport> {
    let line = &lines[start];
    if value.is_empty() {
        return Err(parse_error(
            line,
            "effect block must use `{ ... }`; `end` effect blocks were removed",
        ));
    }
    if value == "{" {
        let block_end = matching_effect_block_end(lines, start, lines.len())?;
        let body = lines[start + 1..block_end].to_vec();
        if body.is_empty() {
            return Err(parse_error(
                line,
                "effect block requires at least one effect",
            ));
        }
        return Ok((parse_scene_handler_effects(&body, line)?, block_end + 1));
    }

    Ok((parse_scene_effect(value, line)?, start + 1))
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedSceneEffect {
    pub(crate) surface: SurfaceSceneEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

fn parse_scene_effect(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    let parsed = parse_scene_effect_with_semantic_tokens(value, line)?;
    debug_assert!(
        parsed
            .semantic_tokens
            .iter()
            .all(|token| token.start < token.end)
    );
    Ok(parsed.surface.effect)
}

fn parse_scene_effect_with_semantic_tokens(
    value: &str,
    line: &str,
) -> Result<ParsedSceneEffect, DiagnosticReport> {
    let surface = parse_surface_scene_effect(value, line)?;
    let semantic_tokens = project_surface_semantic_tokens(&surface.document.semantic_tokens);
    Ok(ParsedSceneEffect {
        surface,
        semantic_tokens,
    })
}

fn parse_surface_scene_effect(
    value: &str,
    line: &str,
) -> Result<SurfaceSceneEffect, DiagnosticReport> {
    let tokens = source_line_tokens(strip_line_comment(value), 0);
    let document = scene_effect_surface_document(&tokens);
    let effect = parse_scene_effect_value(value, line)?;
    Ok(SurfaceSceneEffect { effect, document })
}

fn parse_scene_effect_value(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    if value.contains(" then ") {
        return Err(parse_error(
            line,
            "`then` effect sequences are not supported; use an effect block with one effect per line",
        ));
    }

    if let Some(parts) = split_scene_effect_sequence(value) {
        let mut effects = Vec::new();
        for part in parts {
            effects.push(parse_scene_effect_value(part, line)?);
        }
        return match effects.len() {
            0 => unreachable!("scene effect sequence splitter returned no effects"),
            1 => Ok(effects.remove(0)),
            _ => Ok(SceneEffect::Sequence(effects)),
        };
    }

    if let Some(text) = value.strip_prefix("message ") {
        return Ok(SceneEffect::Message {
            text: parse_scene_expr(text.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("set current_level = ") {
        return Ok(SceneEffect::SetCurrentLevel {
            level: parse_scene_level_expr(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("set level.cleared = ") {
        return Ok(SceneEffect::SetLevelCleared {
            level: None,
            cleared: parse_scene_effect_bool(rest.trim(), line)?,
        });
    }
    if let Some(rest) = value.strip_prefix("set level(") {
        if let Some((level, cleared)) = rest.split_once(").cleared = ") {
            return Ok(SceneEffect::SetLevelCleared {
                level: Some(parse_scene_level_expr(level.trim(), line)?),
                cleared: parse_scene_effect_bool(cleared.trim(), line)?,
            });
        }
    }
    if let Some(rest) = value.strip_prefix("goto ") {
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Goto { scene, params });
    }
    if let Some(rest) = value.strip_prefix("start ") {
        if rest.starts_with("levels ") || rest.contains(" in ") {
            return Err(legacy_start_levels_error(line));
        }
        let (scene, params) = parse_scene_target_params(rest, line)?;
        return Ok(SceneEffect::Sequence(vec![
            SceneEffect::Reset {
                scene: scene.clone(),
            },
            SceneEffect::Goto { scene, params },
        ]));
    }

    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["input", input] => Ok(SceneEffect::Input(
            parse_input_name(input, line)?.to_string(),
        )),
        ["component_effect", effect] => Ok(SceneEffect::ComponentEffect(
            parse_scene_signal_name(effect, line, "component effect")?.to_string(),
        )),
        ["apply", call, "to", target] => {
            validate_target_path(target, line, "apply target")?;
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: Some((*target).to_string()),
            })
        }
        ["apply", call] => {
            let (rule, args) = parse_rule_call_expr(call, line)?;
            Ok(SceneEffect::Apply {
                rule,
                args,
                target: None,
            })
        }
        ["copy", source, "to", target] => {
            validate_target_path(source, line, "copy source")?;
            validate_target_path(target, line, "copy target")?;
            Ok(SceneEffect::Copy {
                source: (*source).to_string(),
                target: (*target).to_string(),
            })
        }
        ["load", target, "from", source] => {
            validate_target_path(target, line, "load target")?;
            Ok(SceneEffect::LoadPuzzle {
                target: (*target).to_string(),
                source: (*source).to_string(),
            })
        }
        ["wait"] => Ok(SceneEffect::Wait { milliseconds: None }),
        ["wait", duration] => Ok(SceneEffect::Wait {
            milliseconds: Some(parse_wait_duration_ms(duration, line)?),
        }),
        ["clear_undo_history"] | ["clear_history"] => Ok(SceneEffect::ClearUndoHistory),
        ["clear_game_progress"] => Ok(SceneEffect::ClearGameProgress),
        ["clear", "current_level"] => Ok(SceneEffect::ClearCurrentLevel),
        ["reset", "persistent_vars"] => Ok(SceneEffect::ResetPersistentVars),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(SceneEffect::PlaySfx {
                name: (*name).to_string(),
            })
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PlayMusic {
                name: (*name).to_string(),
            })
        }
        ["pause_music"] => Ok(SceneEffect::PauseMusic { name: None }),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::PauseMusic {
                name: Some((*name).to_string()),
            })
        }
        ["resume_music"] => Ok(SceneEffect::ResumeMusic { name: None }),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::ResumeMusic {
                name: Some((*name).to_string()),
            })
        }
        ["stop_music"] => Ok(SceneEffect::StopMusic { name: None }),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(SceneEffect::StopMusic {
                name: Some((*name).to_string()),
            })
        }
        ["reset", target] if target.contains('.') => {
            validate_target_path(target, line, "reset target")?;
            Ok(SceneEffect::ResetPuzzle {
                target: (*target).to_string(),
            })
        }
        ["start", "levels", ..] | ["start", _, "in", _] | ["continue", "levels", ..] => {
            Err(legacy_start_levels_error(line))
        }
        [target_command, level] => {
            if let Some((target, command)) = parse_puzzle_command(target_command, line)? {
                if command == "goto" {
                    return Ok(SceneEffect::GotoLevel {
                        target,
                        level: parse_scene_level_expr(level, line)?,
                    });
                }
            }
            Err(parse_error(
                line,
                "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | <scene>.goto <level> | copy <puzzle> to <puzzle>",
            ))
        }
        ["input"] => Err(parse_error(line, "input effect must name an input")),
        ["component_effect"] => Err(parse_error(
            line,
            "component_effect must name a component effect",
        )),
        [command_text] => {
            if let Some((target, command)) = parse_puzzle_command(command_text, line)? {
                if command == "next_level" {
                    return Ok(SceneEffect::PuzzleNextLevel { target });
                }
                if command == "previous_level" {
                    return Ok(SceneEffect::PuzzlePreviousLevel { target });
                }
                if command == "restart" {
                    return Ok(SceneEffect::ResetPuzzle { target });
                }
            }
            if is_identifier(command_text) {
                return Ok(SceneEffect::RoutineCall((*command_text).to_string()));
            }
            Err(parse_error(
                line,
                "bare scene effect aliases were removed; use `input <name>`, `component_effect <name>`, a scene routine, or an explicit scene effect",
            ))
        }
        _ => Err(parse_error(
            line,
            "effect must be: input <name> | component_effect <name> | goto <scene> | goto <scene>(<level>) | start <scene> | start <scene>(<level>) | clear_undo_history | clear_game_progress | message <text> | wait <duration> | sfx <name> | play_music <name> | pause_music [name] | resume_music [name] | stop_music [name] | copy <puzzle> to <puzzle>",
        )),
    }
}

fn split_scene_effect_sequence(value: &str) -> Option<Vec<&str>> {
    let stripped = strip_line_comment(value);
    let tokens = source_line_tokens(stripped, 0);
    let parts = split_scene_effect_token_sequence(&tokens)?;
    Some(
        parts
            .into_iter()
            .map(|part| stripped[part.first().unwrap().start..part.last().unwrap().end].trim())
            .collect(),
    )
}

fn split_scene_effect_token_sequence(tokens: &[SourceToken]) -> Option<Vec<&[SourceToken]>> {
    let mut parts = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let length = scene_effect_token_length(&tokens[index..])?;
        parts.push(&tokens[index..index + length]);
        index += length;
    }
    (parts.len() > 1).then_some(parts)
}

fn scene_effect_token_length(tokens: &[SourceToken]) -> Option<usize> {
    let first = tokens.first()?.text.as_str();
    match first {
        "input" | "component_effect" | "sfx" | "play_music" => (tokens.len() >= 2).then_some(2),
        "pause_music" | "resume_music" | "stop_music" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "wait" => {
            if tokens
                .get(1)
                .is_some_and(|token| !is_scene_effect_command_start(&token.text))
            {
                Some(2)
            } else {
                Some(1)
            }
        }
        "clear_undo_history" | "clear_history" | "clear_game_progress" => Some(1),
        "clear" => (tokens.get(1)?.text == "current_level").then_some(2),
        "reset"
            if tokens
                .get(1)
                .is_some_and(|token| token.text == "persistent_vars") =>
        {
            Some(2)
        }
        "reset" if tokens.get(1).is_some_and(|token| token.text.contains('.')) => Some(2),
        "goto" | "start" => {
            if tokens.get(2).is_some_and(|token| token.text == "with") {
                None
            } else {
                (tokens.len() >= 2).then_some(2)
            }
        }
        _ if first.contains('.') => {
            let command = first.rsplit_once('.').map(|(_, command)| command)?;
            match command {
                "goto" | "goto_level" => (tokens.len() >= 2).then_some(2),
                "next_level" | "previous_level" | "restart" => Some(1),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_scene_effect_command_start(token: &str) -> bool {
    scene_effect_command_syntax(token).is_some()
        || token.rsplit_once('.').is_some_and(|(_, command)| {
            matches!(
                command,
                "goto" | "goto_level" | "next_level" | "previous_level" | "restart"
            )
        })
}

fn parse_scene_effect_bool(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            line,
            "boolean progress value must be true or false",
        )),
    }
}

fn legacy_start_levels_error(line: &str) -> DiagnosticReport {
    parse_error(
        line,
        "`start levels ... in <scene>` and `continue levels ... in <scene>` are no longer supported; use `goto <puzzle>` for the default playable scene, `goto <puzzle>(<level>)` for a specific level, or `goto <scene>(<level>)` for an explicit level scene",
    )
}

const DEFAULT_WAIT_MS: u64 = 200;
const DEFAULT_AGAIN_MS: u64 = 120;

fn resolve_default_wait_in_scenes(scenes: &mut [SceneDef], default_wait_ms: u64) {
    for scene in scenes {
        for component in &mut scene.components {
            resolve_default_wait_in_component(component, default_wait_ms);
        }
        for binding in &mut scene.key_bindings {
            resolve_default_wait_in_effect(&mut binding.effect, default_wait_ms);
        }
        for routine in &mut scene.routines {
            resolve_default_wait_in_effect(&mut routine.effect, default_wait_ms);
        }
        for transition in &mut scene.transitions {
            resolve_default_wait_in_effect(&mut transition.effect, default_wait_ms);
        }
    }
}

fn resolve_default_wait_in_component(component: &mut SceneComponent, default_wait_ms: u64) {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &mut container.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::Conditional(conditional) => {
            for child in &mut conditional.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::For(for_view) => {
            for child in &mut for_view.children {
                resolve_default_wait_in_component(child, default_wait_ms);
            }
        }
        SceneComponent::LevelMenu(menu) => {
            for button in &mut menu.buttons {
                resolve_default_wait_in_effect(&mut button.effect, default_wait_ms);
            }
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => {}
    }
}

fn resolve_default_wait_in_effect(effect: &mut SceneEffect, default_wait_ms: u64) {
    match effect {
        SceneEffect::Wait { milliseconds } => {
            if milliseconds.is_none() {
                *milliseconds = Some(default_wait_ms);
            }
        }
        SceneEffect::Conditional { effect, .. } => {
            resolve_default_wait_in_effect(effect, default_wait_ms);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                resolve_default_wait_in_effect(effect, default_wait_ms);
            }
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::RoutineCall(_)
        | SceneEffect::Message { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => {}
    }
}

fn parse_wait_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return parse_whole_milliseconds(milliseconds, line);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return parse_seconds_duration_ms(seconds, line);
    }
    Err(parse_error(
        line,
        "wait duration must use seconds or milliseconds, for example `wait 0.1s` or `wait 100ms`",
    ))
}

fn parse_whole_milliseconds(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait milliseconds must be a whole number",
        ));
    }
    value
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))
}

fn parse_seconds_duration_ms(value: &str, line: &str) -> Result<u64, DiagnosticReport> {
    let value = value.trim();
    let has_decimal = value.contains('.');
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if (has_decimal && fraction.is_empty()) || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(parse_error(
            line,
            "wait seconds must be a non-negative number",
        ));
    }
    if fraction.len() > 3 {
        return Err(parse_error(
            line,
            "wait seconds can use at most millisecond precision",
        ));
    }
    let whole_ms = whole
        .parse::<u64>()
        .map_err(|_| parse_error(line, "wait duration is too large"))?
        .checked_mul(1000)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))?;
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{fraction:0<3}");
        padded
            .parse::<u64>()
            .map_err(|_| parse_error(line, "wait duration is too large"))?
    };
    whole_ms
        .checked_add(fraction_ms)
        .ok_or_else(|| parse_error(line, "wait duration is too large"))
}

fn parse_rule_call_expr(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneExpr>), DiagnosticReport> {
    let Some((name, args)) = value.split_once('(') else {
        validate_qualified_identifier(value, line, "rule name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(name, line, "rule name")?;
    let args = args
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "rule call args must end with )"))?;
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        args.split(',')
            .map(str::trim)
            .map(|arg| parse_scene_expr(arg, line))
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), args))
}

fn parse_scene_call_params(
    value: &str,
    line: &str,
) -> Result<Option<(String, Vec<SceneEffectParam>)>, DiagnosticReport> {
    let Some(open) = value.find('(') else {
        return Ok(None);
    };
    if !value.ends_with(')') {
        return Err(parse_error(line, "scene call must close with `)`"));
    }
    let scene = value[..open].trim();
    validate_qualified_identifier(scene, line, "scene name")?;
    let args = value[open + 1..value.len() - 1].trim();
    if args.is_empty() {
        return Ok(Some((scene.to_string(), Vec::new())));
    }

    let parts = args.split(',').map(str::trim).collect::<Vec<_>>();
    let params = if parts.len() == 1 && !parts[0].contains('=') {
        vec![SceneEffectParam::Level(parse_scene_level_expr(
            parts[0], line,
        )?)]
    } else {
        parse_scene_named_params(&parts, line)?
    };
    Ok(Some((scene.to_string(), params)))
}

fn parse_scene_target_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<SceneEffectParam>), DiagnosticReport> {
    let value = value.trim();
    if let Some((scene, params)) = value.split_once(" with ") {
        let scene = scene.trim();
        validate_qualified_identifier(scene, line, "scene name")?;
        let parts = params.split(',').map(str::trim).collect::<Vec<_>>();
        return Ok((scene.to_string(), parse_scene_named_params(&parts, line)?));
    }
    if let Some((scene, params)) = parse_scene_call_params(value, line)? {
        return Ok((scene, params));
    }
    validate_qualified_identifier(value, line, "scene name")?;
    Ok((value.to_string(), Vec::new()))
}

fn parse_scene_named_params(
    parts: &[&str],
    line: &str,
) -> Result<Vec<SceneEffectParam>, DiagnosticReport> {
    let mut params = Vec::new();
    for part in parts {
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| parse_error(line, "scene params must be named `<name> = <expr>`"))?;
        let name = name.trim();
        validate_identifier(name, line, "scene param name")?;
        params.push(SceneEffectParam::Named {
            name: name.to_string(),
            value: parse_scene_expr(value.trim(), line)?,
        });
    }
    Ok(params)
}

fn parse_scene_level_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    if parse_quoted_text(value).is_some() {
        return Err(parse_error(
            line,
            "scene level arguments must not be quoted; use `goto <scene>(<level_name>)`",
        ));
    }
    if is_dotted_level_atom(value) {
        return Ok(SceneExpr::Text(value.to_string()));
    }
    match parse_scene_expr(value, line) {
        Ok(expr) => Ok(expr),
        Err(error) => Err(error),
    }
}

fn parse_scene_expr(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    if value == "true" {
        return Ok(SceneExpr::Bool(true));
    }
    if value == "false" {
        return Ok(SceneExpr::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneExpr::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneExpr::Text(text));
    }
    if value.contains('(') {
        let (name, args) = parse_rule_call_expr(value, line)?;
        return Ok(SceneExpr::Call { name, args });
    }
    if value.starts_with("join ") {
        return Err(parse_error(
            line,
            "`join` scene expression is not supported",
        ));
    }
    if let Some(path) = parse_view_path(value) {
        return Ok(SceneExpr::Path(path));
    }
    Err(parse_error(
        line,
        "expression must be true, false, integer, quoted text, or path",
    ))
}

fn is_dotted_level_atom(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() > 1
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        })
}

fn parse_input_name<'a>(value: &'a str, line: &str) -> Result<&'a str, DiagnosticReport> {
    validate_identifier(value, line, "input name")?;
    Ok(value)
}

fn parse_scene_signal_name<'a>(
    value: &'a str,
    line: &str,
    label: &str,
) -> Result<&'a str, DiagnosticReport> {
    validate_qualified_identifier(value, line, label)?;
    Ok(value)
}

fn parse_puzzle_command<'a>(
    value: &'a str,
    line: &str,
) -> Result<Option<(String, &'a str)>, DiagnosticReport> {
    let Some((target, command)) = value.split_once('.') else {
        return Ok(None);
    };
    validate_qualified_identifier(target, line, "puzzle target")?;
    validate_identifier(command, line, "puzzle command")?;
    Ok(Some((target.to_string(), command)))
}

fn validate_target_path(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if parse_view_path(value).is_some() {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be an identifier path"),
        ))
    }
}

#[derive(Clone, Copy)]
enum NameClass {
    Identifier,
    Qualified,
}

fn validate_name(
    value: &str,
    class: NameClass,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let valid = match class {
        NameClass::Identifier => is_identifier(value),
        NameClass::Qualified => is_qualified_identifier(value),
    };
    if valid {
        Ok(())
    } else {
        let expected = match class {
            NameClass::Identifier => "an identifier",
            NameClass::Qualified => "a qualified identifier",
        };
        Err(parse_error(line, &format!("{label} must be {expected}")))
    }
}

fn validate_identifier(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Identifier, line, label)
}

fn validate_qualified_identifier(
    value: &str,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    validate_name(value, NameClass::Qualified, line, label)
}

fn parse_view_path(value: &str) -> Option<Vec<String>> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || !parts.iter().all(|part| is_qualified_identifier(part)) {
        return None;
    }
    Some(parts.into_iter().map(ToString::to_string).collect())
}

fn parse_button_label(value: &str, line: &str) -> Result<SceneExpr, DiagnosticReport> {
    parse_scene_expr(value, line)
}

struct ParsedScreenStateBlock {
    variables: Vec<SceneVarDef>,
    puzzles: Vec<ScenePuzzleDef>,
}

enum ParsedSceneStateEntry {
    Variable(SceneVarDef),
    Puzzle(ScenePuzzleDef),
}

fn parse_scene_state_block(
    lines: &[String],
    start: usize,
    lifetime: SceneStateLifetime,
) -> Result<(ParsedScreenStateBlock, usize), DiagnosticReport> {
    let mut variables = Vec::new();
    let mut puzzles = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        match parse_scene_state_entry(&lines[i], lifetime)? {
            ParsedSceneStateEntry::Variable(variable) => variables.push(variable),
            ParsedSceneStateEntry::Puzzle(puzzle) => puzzles.push(puzzle),
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "state missing closing brace"));
    }
    Ok((ParsedScreenStateBlock { variables, puzzles }, i + 1))
}

fn parse_scene_state_entry(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<ParsedSceneStateEntry, DiagnosticReport> {
    let line = line.trim();
    if let Some(puzzle) = parse_implicit_scene_puzzle_state_entry(line, lifetime)? {
        return Ok(ParsedSceneStateEntry::Puzzle(puzzle));
    }
    let mut prefixed_variable = false;
    let (line, lifetime, mutable) = if let Some(rest) = line.strip_prefix("persistent var ") {
        prefixed_variable = true;
        (rest.trim_start(), SceneStateLifetime::Persistent, true)
    } else if let Some(rest) = line.strip_prefix("persistent const ") {
        prefixed_variable = true;
        (rest.trim_start(), SceneStateLifetime::Persistent, false)
    } else if let Some(rest) = line.strip_prefix("var ") {
        prefixed_variable = true;
        (rest.trim_start(), lifetime, true)
    } else if let Some(rest) = line.strip_prefix("const ") {
        prefixed_variable = true;
        (rest.trim_start(), lifetime, false)
    } else {
        (line, lifetime, true)
    };
    let Some((name, value)) = line.split_once('=') else {
        return Err(parse_error(line, "scene state must be: <name> = <value>"));
    };
    let name = name.trim();
    if !is_identifier(name) {
        return Err(parse_error(line, "scene state name must be an identifier"));
    }
    let value = value.trim();
    if let Some(initializer) = parse_screen_puzzle_initializer(value, line)? {
        if prefixed_variable {
            return Err(parse_error(
                line,
                "var or const cannot define a puzzle slot",
            ));
        }
        return Ok(ParsedSceneStateEntry::Puzzle(ScenePuzzleDef {
            name: name.to_string(),
            kind: initializer.kind,
            model: initializer.model,
            initializer: initializer.initializer,
            lifetime,
        }));
    }
    Ok(ParsedSceneStateEntry::Variable(SceneVarDef {
        name: name.to_string(),
        default: parse_scene_value(value, line)?,
        lifetime,
        mutable,
    }))
}

fn parse_implicit_scene_puzzle_state_entry(
    line: &str,
    lifetime: SceneStateLifetime,
) -> Result<Option<ScenePuzzleDef>, DiagnosticReport> {
    if let Some((puzzle_name, param)) = parse_scene_puzzle_state_call(line) {
        validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
        validate_identifier(param, line, "scene level param")?;
        if param != "level" {
            return Err(parse_error(
                line,
                "scene puzzle state call must be `<puzzle>(level)`",
            ));
        }
        return Ok(Some(ScenePuzzleDef {
            name: puzzle_name.to_string(),
            kind: INFERRED_SCENE_PUZZLE_KIND.to_string(),
            model: puzzle_name.to_string(),
            initializer: ScenePuzzleInitializer::CurrentLevel,
            lifetime,
        }));
    }
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        [puzzle_name] if is_qualified_identifier(puzzle_name) => Ok(Some(ScenePuzzleDef {
            name: (*puzzle_name).to_string(),
            kind: INFERRED_SCENE_PUZZLE_KIND.to_string(),
            model: (*puzzle_name).to_string(),
            initializer: ScenePuzzleInitializer::CurrentLevel,
            lifetime,
        })),
        ["puzzle", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime,
            }))
        }
        ["puzzle", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
                lifetime,
            }))
        }
        ["puzzle3", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime,
            }))
        }
        ["puzzle3", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ScenePuzzleDef {
                name: (*puzzle_name).to_string(),
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
                lifetime,
            }))
        }
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle state must be: puzzle <name> | puzzle <name> level <level>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "scene puzzle3 state must be: puzzle3 <name> | puzzle3 <name> level <level>",
        )),
        _ => Ok(None),
    }
}

fn parse_scene_puzzle_state_call(line: &str) -> Option<(&str, &str)> {
    let (name, rest) = line.split_once('(')?;
    let param = rest.strip_suffix(')')?;
    let name = name.trim();
    let param = param.trim();
    if name.is_empty() || param.is_empty() || param.contains(',') {
        return None;
    }
    Some((name, param))
}

fn parse_top_level_var_directive(
    _tokens: &[&str],
    line: &str,
) -> Result<SceneVarDef, DiagnosticReport> {
    let (rest, lifetime, mutable) = if let Some(rest) = line.trim().strip_prefix("persistent var ")
    {
        (rest.trim_start(), SceneStateLifetime::Persistent, true)
    } else if let Some(rest) = line.trim().strip_prefix("persistent const ") {
        (rest.trim_start(), SceneStateLifetime::Persistent, false)
    } else if let Some(rest) = line.trim().strip_prefix("var ") {
        (rest.trim_start(), SceneStateLifetime::Instance, true)
    } else if let Some(rest) = line.trim().strip_prefix("const ") {
        (rest.trim_start(), SceneStateLifetime::Instance, false)
    } else {
        return Err(parse_error(
            line,
            "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
        ));
    };
    let Some((name, value)) = rest.split_once('=') else {
        return Err(parse_error(
            line,
            "top-level variable must be: var <name> = <literal> or const <name> = <literal>",
        ));
    };
    let name = name.trim();
    let value = value.trim();
    validate_identifier(name, line, "variable name")?;
    Ok(SceneVarDef {
        name: name.to_string(),
        default: parse_scene_value(value, line)?,
        lifetime,
        mutable,
    })
}

fn parse_default_wait_time_directive(tokens: &[&str], line: &str) -> Result<u64, DiagnosticReport> {
    let ["default_wait_time", "=", duration] = tokens else {
        return Err(parse_error(
            line,
            "default_wait_time must be: default_wait_time = <duration>",
        ));
    };
    parse_wait_duration_ms(duration, line)
}

fn parse_again_interval_directive(tokens: &[&str], line: &str) -> Result<u64, DiagnosticReport> {
    match tokens {
        ["again_interval", "=", duration] => parse_wait_duration_ms(duration, line),
        ["again_interval", seconds] => parse_seconds_duration_ms(seconds, line),
        _ => Err(parse_error(
            line,
            "again_interval must be: again_interval = <duration> or again_interval <seconds>",
        )),
    }
}

#[derive(Clone, Debug)]
struct ParsedScenePuzzleInitializer {
    kind: String,
    model: String,
    initializer: ScenePuzzleInitializer,
}

fn parse_screen_puzzle_initializer(
    value: &str,
    line: &str,
) -> Result<Option<ParsedScenePuzzleInitializer>, DiagnosticReport> {
    let tokens = split_header_tokens(value);
    match tokens.as_slice() {
        ["puzzle", "current_level"] => Err(parse_error(
            line,
            "current_level is not scene syntax; use `puzzle <name>` for the current level",
        )),
        ["puzzle", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
            }))
        }
        ["puzzle", puzzle_name] => {
            if *puzzle_name == "current_level" {
                return Err(parse_error(
                    line,
                    "current_level is not scene syntax; use `puzzle <name>` for the current level",
                ));
            }
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
            }))
        }
        ["puzzle3", puzzle_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
            }))
        }
        ["puzzle3", puzzle_name, "level", level_name] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle3 model name")?;
            validate_qualified_identifier(level_name, line, "level name")?;
            Ok(Some(ParsedScenePuzzleInitializer {
                kind: "puzzle3".to_string(),
                model: (*puzzle_name).to_string(),
                initializer: ScenePuzzleInitializer::Level((*level_name).to_string()),
            }))
        }
        ["puzzle", puzzle_name, "current_level"] => {
            validate_qualified_identifier(puzzle_name, line, "puzzle name")?;
            Err(parse_error(
                line,
                "current_level is not scene syntax; use `puzzle <name>` for the current level",
            ))
        }
        ["puzzle", ..] => Err(parse_error(
            line,
            "scene puzzle initializer must be: puzzle <name> | puzzle <name> level <level>",
        )),
        ["puzzle3", ..] => Err(parse_error(
            line,
            "scene puzzle3 initializer must be: puzzle3 <name> | puzzle3 <name> level <level>",
        )),
        _ => Ok(None),
    }
}

fn parse_scene_value(value: &str, line: &str) -> Result<SceneValue, DiagnosticReport> {
    if value == "true" {
        return Ok(SceneValue::Bool(true));
    }
    if value == "false" {
        return Ok(SceneValue::Bool(false));
    }
    if let Ok(number) = value.parse::<i64>() {
        return Ok(SceneValue::Int(number));
    }
    if let Some(text) = parse_quoted_text(value) {
        return Ok(SceneValue::Text(text));
    }
    if is_identifier(value) {
        return Ok(SceneValue::Symbol(value.to_string()));
    }
    Err(parse_error(
        line,
        "scene state value must be true, false, integer, symbol, or quoted text",
    ))
}

fn parse_quoted_text(value: &str) -> Option<String> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\""))
}

struct ParsedScreenTransitionsBlock {
    transitions: Vec<SceneTransition>,
    puzzle_rule: Option<ScenePuzzleRule>,
}

fn parse_screen_transitions_block(
    lines: &[String],
    start: usize,
) -> Result<(ParsedScreenTransitionsBlock, usize), DiagnosticReport> {
    let mut transitions = Vec::new();
    let mut puzzle_rule = None;
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["step", target] => {
                validate_target_path(target, &lines[i], "step target")?;
                puzzle_rule = Some(ScenePuzzleRule {
                    target: (*target).to_string(),
                    rule: "rules".to_string(),
                });
            }
            [rule] if rule.contains('.') => {
                return Err(parse_error(
                    &lines[i],
                    "scene rules do not call component rules by path; use `step <puzzle>`",
                ));
            }
            ["if"] | ["if", "all"] if lines[i].trim_end().ends_with('{') => {
                let (transition, next_i) = parse_screen_condition_arrow_block(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ if lines[i].contains("->") => {
                let (transition, next_i) = parse_transition_row(lines, i)?;
                transitions.push(transition);
                i = next_i;
                continue;
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "transitions row must be: step <puzzle> | <input> -> <effect> | if <condition> -> <effect>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "transitions missing closing brace",
        ));
    }

    Ok((
        ParsedScreenTransitionsBlock {
            transitions,
            puzzle_rule,
        },
        i + 1,
    ))
}

fn parse_screen_condition_arrow_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let header = block_header_text(&lines[start]);
    match split_header_tokens(header).as_slice() {
        ["if"] | ["if", "all"] => {}
        ["if", "any"] => {
            return Err(parse_error(
                &lines[start],
                "scene condition blocks only support all conditions",
            ));
        }
        _ => {
            return Err(parse_error(
                &lines[start],
                "scene condition block must be: if [all] {",
            ));
        }
    }

    let mut conditions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        validate_screen_condition(&lines[i], &lines[i])?;
        conditions.push(lines[i].clone());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "scene condition block missing closing brace",
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene condition block requires at least one condition",
        ));
    }

    let arrow_i = i + 1;
    let Some(arrow_line) = lines.get(arrow_i) else {
        return Err(parse_error(
            &lines[start],
            "scene condition block must be followed by ->",
        ));
    };
    let Some((_, effect_text)) = arrow_line.split_once("->") else {
        return Err(parse_error(
            arrow_line,
            "scene condition block must be followed by ->",
        ));
    };
    let (effect, next_i) =
        parse_scene_effect_with_optional_block(effect_text.trim(), lines, arrow_i)?;
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(conditions.join(" and ")),
            effect,
        },
        next_i,
    ))
}

fn parse_screen_condition_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "condition block must be: if <condition>"))?
        .trim();
    validate_screen_condition(condition, line)?;
    let (body, next_i) = collect_authoring_entry(lines, start)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            line,
            "condition block requires at least one effect",
        ));
    }
    Ok((
        SceneTransition {
            trigger: SceneTransitionTrigger::Condition(condition.to_string()),
            effect: parse_scene_handler_effects(body, line)?,
        },
        next_i,
    ))
}

fn parse_scene_lifecycle_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let [lifecycle @ "on_scene_start"] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "scene lifecycle block must be: on_scene_start",
        ));
    };
    let (body, next_i) = collect_authoring_entry(lines, start)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene lifecycle block requires at least one effect",
        ));
    }
    let trigger = match *lifecycle {
        "on_scene_start" => SceneTransitionTrigger::SceneStart,
        _ => unreachable!(),
    };
    Ok((
        SceneTransition {
            trigger,
            effect: parse_scene_handler_effects(body, &lines[start])?,
        },
        next_i,
    ))
}

fn parse_scene_routine_block(
    lines: &[String],
    start: usize,
) -> Result<(SceneRoutineDef, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["routine", name] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "scene routine header must be: routine <name>",
        ));
    };
    validate_identifier(name, &lines[start], "scene routine name")?;
    let (body, next_i) = collect_authoring_entry(lines, start)?;
    let body = &body[1..body.len().saturating_sub(1)];
    if body.is_empty() {
        return Err(parse_error(
            &lines[start],
            "scene routine requires at least one effect",
        ));
    }
    Ok((
        SceneRoutineDef {
            name: (*name).to_string(),
            effect: parse_scene_handler_effects(body, &lines[start])?,
        },
        next_i,
    ))
}

fn parse_scene_handler_effects(
    lines: &[String],
    header_line: &str,
) -> Result<SceneEffect, DiagnosticReport> {
    parse_scene_handler_effects_range(lines, 0, lines.len(), header_line)
}

fn parse_scene_handler_effects_range(
    lines: &[String],
    start: usize,
    end: usize,
    header_line: &str,
) -> Result<SceneEffect, DiagnosticReport> {
    let mut effects = Vec::new();
    let mut i = start;
    while i < end {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["if", condition @ ..] => {
                let condition = condition.join(" ");
                validate_screen_condition(&condition, line)?;
                let block_end = matching_effect_block_end(lines, i, end)?;
                let effect = parse_scene_handler_effects_range(lines, i + 1, block_end, line)?;
                effects.push(SceneEffect::Conditional {
                    condition,
                    effect: Box::new(effect),
                });
                i = block_end + 1;
                continue;
            }
            ["update", _target] => {
                return Err(parse_error(
                    line,
                    "`update <target>` was removed; use `apply <rule> to <target>`",
                ));
            }
            _ => effects.push(parse_scene_effect(line, line)?),
        }
        i += 1;
    }
    match effects.len() {
        0 => Err(parse_error(
            header_line,
            "handler requires at least one effect",
        )),
        1 => Ok(effects.remove(0)),
        _ => Ok(SceneEffect::Sequence(effects)),
    }
}

fn matching_effect_block_end(
    lines: &[String],
    start: usize,
    end: usize,
) -> Result<usize, DiagnosticReport> {
    let mut depth = 0usize;
    for (i, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        let tokens = split_header_tokens(line);
        if matches!(tokens.as_slice(), ["if", ..]) {
            depth += 1;
            continue;
        }
        if is_block_close_line(line) {
            if depth == 0 {
                return Ok(i);
            }
            depth -= 1;
        }
    }
    Err(parse_error(
        &lines[start],
        "if effect block missing closing brace",
    ))
}

fn parse_transition_row(
    lines: &[String],
    start: usize,
) -> Result<(SceneTransition, usize), DiagnosticReport> {
    let Some((pattern, effect)) = lines[start].split_once("->") else {
        return Err(parse_error(
            &lines[start],
            "transition must be: if <condition> -> <effect>",
        ));
    };
    let (effect, next_i) = parse_scene_effect_with_optional_block(effect.trim(), lines, start)?;
    Ok((
        SceneTransition {
            trigger: parse_transition_trigger(pattern.trim(), &lines[start])?,
            effect,
        },
        next_i,
    ))
}

fn parse_transition_trigger(
    value: &str,
    line: &str,
) -> Result<SceneTransitionTrigger, DiagnosticReport> {
    if value == "scene_start" {
        return Err(parse_error(
            line,
            "scene_start is a lifecycle block; write `on_scene_start { ... }` instead",
        ));
    }
    if value == "on_scene_start" {
        return Err(parse_error(
            line,
            "on_scene_start is a lifecycle block; write `on_scene_start { ... }` instead",
        ));
    }
    if value == "level_start" {
        return Err(parse_error(
            line,
            "level_start is a puzzle lifecycle block; put `on_level_start { ... }` inside puzzle",
        ));
    }
    if matches!(
        value,
        "on_level_start" | "on_level_clear" | "on_last_level_clear"
    ) {
        return Err(parse_error(
            line,
            "level lifecycle blocks belong inside puzzle",
        ));
    }
    if let Some(condition) = value.strip_prefix("if ") {
        let condition = condition.trim();
        validate_screen_condition(condition, line)?;
        return Ok(SceneTransitionTrigger::Condition(condition.to_string()));
    }
    let tokens = split_header_tokens(value);
    if let [input] = tokens.as_slice() {
        let input = parse_input_name(input, line)?;
        return Ok(SceneTransitionTrigger::Condition(format!(
            "input == {input}"
        )));
    }
    Err(parse_error(
        line,
        "scene transition triggers must be `<input>` or `if <condition>`",
    ))
}

fn validate_screen_condition(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    if value.is_empty() {
        return Err(parse_error(line, "condition must not be empty"));
    }
    for part in value.split(" and ") {
        if validate_screen_condition_atom(part.trim()).is_err() {
            return Err(parse_error(
                line,
                "condition must be identifier paths or path comparisons joined by and",
            ));
        }
    }
    Ok(())
}

fn validate_screen_condition_atom(value: &str) -> Result<(), ()> {
    if parse_view_path(value).is_some() {
        return Ok(());
    }
    for op in [" == ", " != "] {
        if let Some((left, right)) = value.split_once(op) {
            if parse_view_path(left.trim()).is_none() {
                return Err(());
            }
            return validate_screen_condition_value(right.trim());
        }
    }
    Err(())
}

fn validate_screen_condition_value(value: &str) -> Result<(), ()> {
    if value == "true" || value == "false" || value.parse::<i64>().is_ok() {
        return Ok(());
    }
    if parse_quoted_text(value).is_some() {
        return Ok(());
    }
    parse_view_path(value).map(|_| ()).ok_or(())
}

fn parse_level_menu_component(
    lines: &[String],
    start: usize,
) -> Result<(LevelMenuDef, usize), DiagnosticReport> {
    let next = start + 1;
    if next >= lines.len() || !is_level_menu_option(&split_header_tokens(&lines[next])) {
        return Ok((LevelMenuDef::default(), next));
    }

    let mut menu = LevelMenuDef::default();
    let mut i = next;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["show_index", "=", value] => menu.show_index = parse_boolean_option(value, &lines[i])?,
            ["show_solved", "=", value] => {
                menu.show_cleared = parse_boolean_option(value, &lines[i])?
            }
            ["show_current" | "show_current_level", "=", _] => {
                return Err(parse_error(
                    &lines[i],
                    "level_menu no longer supports show_current_level",
                ));
            }
            ["layout", "=", "list"] => menu.columns = None,
            ["columns", "=", value] => {
                menu.columns = Some(parse_level_menu_columns(value, &lines[i])?)
            }
            ["wrap", "=", value] => menu.wrap = parse_boolean_option(value, &lines[i])?,
            ["locked", "=", "disabled"] => menu.locked = LevelMenuLocked::Disabled,
            ["locked", "=", "hidden"] => menu.locked = LevelMenuLocked::Hidden,
            ["button", ..] => {
                let (button, next_i) = parse_button_def(lines, i)?;
                menu.buttons.push(button);
                i = next_i;
                continue;
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "level_menu option must be: show_index = <true|false> | show_solved = <true|false> | show_current_level = <true|false> | layout = list | columns = <n> | wrap = <true|false> | locked = <disabled|hidden>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "level_menu missing closing brace",
        ));
    }

    Ok((menu, i + 1))
}

pub(crate) const LEVEL_MENU_OPTIONS: &[&str] = &[
    "show_index",
    "show_solved",
    "layout",
    "columns",
    "wrap",
    "locked",
];

fn is_level_menu_option(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["index", ..]
            | ["show_index", ..]
            | ["solved", ..]
            | ["show_solved", ..]
            | ["current", ..]
            | ["show_current", ..]
            | ["show_current_level", ..]
            | ["layout", ..]
            | ["columns", ..]
            | ["wrap", ..]
            | ["locked", ..]
    )
}

fn parse_boolean_option(value: &str, line: &str) -> Result<bool, DiagnosticReport> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(line, "boolean option must be true or false")),
    }
}

fn parse_level_menu_columns(value: &str, line: &str) -> Result<u16, DiagnosticReport> {
    let columns = value
        .parse::<u16>()
        .map_err(|_| parse_error(line, "columns must be an integer"))?;
    if columns == 0 {
        return Err(parse_error(
            line,
            "level_menu columns must be greater than 0",
        ));
    }
    Ok(columns)
}

fn parse_key_trigger(token: &str, line: &str) -> Result<KeyTrigger, DiagnosticReport> {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return Err(parse_error(line, "missing key"));
    };
    if chars.next().is_none() {
        return Ok(KeyTrigger::Char(first.to_ascii_lowercase()));
    }
    Ok(KeyTrigger::Named(token.to_string()))
}

fn parse_model_keys_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
    controls: &mut Controls,
) -> Result<usize, DiagnosticReport> {
    let mut seen_keys = HashSet::<KeyTrigger>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let Some((keys_text, input_text)) = lines[i].split_once("->") else {
            return Err(parse_error(
                &lines[i],
                "keys row must be: <key...> -> <input>",
            ));
        };
        let keys = keys_text.split_whitespace().collect::<Vec<_>>();
        let input_tokens = split_header_tokens(input_text.trim());
        match input_tokens.as_slice() {
            [input_name] if !keys.is_empty() => {
                let input = catalog
                    .input_names
                    .get(*input_name)
                    .copied()
                    .map(Ok)
                    .unwrap_or_else(|| add_input_name(input_name, &lines[i], catalog))?;
                for key in keys {
                    let trigger = parse_key_trigger(key, &lines[i])?;
                    if !seen_keys.insert(trigger.clone()) {
                        return Err(parse_error(&lines[i], "duplicate model input key"));
                    }
                    add_key_trigger_to_controls(&trigger, input, controls, &lines[i])?;
                }
            }
            _ => {
                return Err(parse_error(
                    &lines[i],
                    "keys row must be: <key...> -> <input>",
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_scene_keys_block(
    lines: &[String],
    start: usize,
) -> Result<(Vec<KeyBinding>, usize), DiagnosticReport> {
    let mut bindings = Vec::<KeyBinding>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].contains('=') {
            return Err(parse_error(
                &lines[i],
                "keys row must use `->`: <key...> -> <scene effect-or-input>",
            ));
        }
        let Some((key, effect)) = lines[i].split_once("->") else {
            return Err(parse_error(
                &lines[i],
                "keys row must be: <key...> -> <scene effect-or-input>",
            ));
        };
        let key_tokens = key.split_whitespace().collect::<Vec<_>>();
        if key_tokens.is_empty() {
            return Err(parse_error(
                &lines[i],
                "keys row must name at least one key",
            ));
        }
        let mut triggers = Vec::new();
        for key in key_tokens {
            let trigger = parse_key_trigger(key, &lines[i])?;
            validate_key_trigger_supported(&trigger, &lines[i])?;
            triggers.push(trigger);
        }
        let (effect, next_i) = parse_scene_effect_with_optional_block(effect.trim(), lines, i)?;
        bindings.push(KeyBinding {
            keys: triggers,
            effect,
        });
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "keys missing closing brace"));
    }
    Ok((bindings, i + 1))
}

fn add_key_trigger_to_controls(
    key: &KeyTrigger,
    input: InputId,
    controls: &mut Controls,
    line: &str,
) -> Result<(), DiagnosticReport> {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => {
            controls
                .keys
                .insert((*ch as u8).to_ascii_lowercase(), input);
        }
        KeyTrigger::Char(_) => {
            return Err(DiagnosticReport::error(
                "non-ascii model input key bindings are not supported yet".to_string(),
            ));
        }
        KeyTrigger::Named(name) => {
            validate_key_trigger_supported(key, line)?;
            if let Some(arrow) = named_key_to_arrow(name) {
                controls.arrows.insert(arrow, input);
            } else {
                controls.named.insert(name.clone(), input);
            }
        }
    }
    Ok(())
}

fn add_default_key_controls(input_names: &HashMap<String, InputId>, controls: &mut Controls) {
    for (name, key, arrow) in [
        ("up", b'w', Some(ArrowKey::Up)),
        ("down", b's', Some(ArrowKey::Down)),
        ("left", b'a', Some(ArrowKey::Left)),
        ("right", b'd', Some(ArrowKey::Right)),
        ("restart", b'r', None),
    ] {
        let Some(input) = input_names.get(name).copied() else {
            continue;
        };
        controls.keys.entry(key).or_insert(input);
        if let Some(arrow) = arrow {
            controls.arrows.entry(arrow).or_insert(input);
        }
    }
}

fn validate_key_trigger_supported(key: &KeyTrigger, line: &str) -> Result<(), DiagnosticReport> {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => Ok(()),
        KeyTrigger::Char(_) => Err(DiagnosticReport::error(
            "non-ascii input key bindings are not supported yet".to_string(),
        )),
        KeyTrigger::Named(name) if is_supported_named_key(name) => Ok(()),
        KeyTrigger::Named(_) => Err(parse_error(
            line,
            "inputs only support character keys, ArrowUp/ArrowDown/ArrowLeft/ArrowRight, Enter, Space, Escape, Tab, and Backspace",
        )),
    }
}

fn is_supported_named_key(name: &str) -> bool {
    named_key_to_arrow(name).is_some()
        || matches!(name, "Enter" | "Space" | "Escape" | "Tab" | "Backspace")
}

fn named_key_to_arrow(name: &str) -> Option<ArrowKey> {
    match name {
        "ArrowUp" | "arrow_up" => Some(ArrowKey::Up),
        "ArrowDown" | "arrow_down" => Some(ArrowKey::Down),
        "ArrowLeft" | "arrow_left" => Some(ArrowKey::Left),
        "ArrowRight" | "arrow_right" => Some(ArrowKey::Right),
        _ => None,
    }
}

fn define_object_spec(
    spec: &str,
    layer: u16,
    render_spec: Option<&str>,
    line: &str,
    value_sets: &HashMap<String, Vec<String>>,
    object_schemas: &mut HashMap<String, ObjectSchema>,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
    render_chars: &mut HashMap<ObjectId, char>,
    char_objects: &mut HashMap<char, Vec<ObjectId>>,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let base = parts[0];
    if parts.len() == 1 {
        if object_names.contains_key(spec) {
            return Err(parse_error(line, "duplicate object"));
        }
        if object_schemas.contains_key(spec) {
            return Err(parse_error(
                line,
                "object name must not shadow an object family selector",
            ));
        }
        let id = add_object_variant(
            spec,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(render) = render_spec {
            let ch = parse_render_chars(render, line)?
                .into_iter()
                .next()
                .ok_or_else(|| parse_error(line, "missing object render char"))?;
            render_chars.insert(id, ch);
            char_objects.insert(ch, vec![id]);
        }
        return Ok(vec![id]);
    }

    if object_names.contains_key(base) {
        return Err(parse_error(
            line,
            "object family name must not shadow an object",
        ));
    }
    if object_schemas.contains_key(base) {
        return Err(parse_error(line, "duplicate object family"));
    }

    let axes = parts[1..]
        .iter()
        .map(|axis| {
            if !value_sets.contains_key(*axis) {
                return Err(parse_error(
                    line,
                    "object schema tag slot must name a tag set",
                ));
            }
            Ok((*axis).to_string())
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    let value_combinations = expand_axis_values(&axes, value_sets, line)?;
    let render_chars_for_variants = render_spec
        .map(|render| render_chars_for_variants(render, value_combinations.len(), line))
        .transpose()?;
    let mut variants = Vec::with_capacity(value_combinations.len());
    let mut created = Vec::with_capacity(value_combinations.len());

    for (index, values) in value_combinations.into_iter().enumerate() {
        let name = format!("{base}:{}", values.join(":"));
        let id = add_object_variant(
            &name,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(chars) = &render_chars_for_variants {
            let ch = chars[index];
            render_chars.insert(id, ch);
            if index == 0 {
                char_objects.insert(ch, vec![id]);
            } else if chars.iter().filter(|candidate| **candidate == ch).count() == 1 {
                char_objects.insert(ch, vec![id]);
            }
        }
        created.push(id);
        variants.push(ObjectVariant { values, object: id });
    }

    object_schemas.insert(base.to_string(), ObjectSchema { axes, variants });
    Ok(created)
}

fn add_object_variant(
    name: &str,
    layer: u16,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
) -> ObjectId {
    let id = ObjectId((object_defs.len() + 1) as u16);
    object_names.insert(name.to_string(), id);
    object_labels.insert(id, name.to_string());
    object_layers.insert(id, LayerId(layer));
    object_defs.push(ObjectDef {
        id,
        layer_id: LayerId(layer),
    });
    id
}

fn expand_axis_values(
    axes: &[String],
    value_sets: &HashMap<String, Vec<String>>,
    line: &str,
) -> Result<Vec<Vec<String>>, DiagnosticReport> {
    let mut combinations = vec![Vec::<String>::new()];
    for axis in axes {
        let values = value_sets
            .get(axis)
            .ok_or_else(|| parse_error(line, "unknown object schema tag set"))?;
        let mut next = Vec::new();
        for prefix in &combinations {
            for value in values {
                let mut variant = prefix.clone();
                variant.push(value.clone());
                next.push(variant);
            }
        }
        combinations = next;
    }
    Ok(combinations)
}

fn render_chars_for_variants(
    render: &str,
    variant_count: usize,
    line: &str,
) -> Result<Vec<char>, DiagnosticReport> {
    let chars = parse_render_chars(render, line)?;
    if chars.len() == 1 {
        return Ok(vec![chars[0]; variant_count]);
    }
    if chars.len() == variant_count {
        return Ok(chars);
    }
    Err(parse_error(
        line,
        "object schema render chars must be one char or one char per variant",
    ))
}

fn parse_render_chars(render: &str, line: &str) -> Result<Vec<char>, DiagnosticReport> {
    let chars = render.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err(parse_error(line, "missing object render char"));
    }
    Ok(chars)
}

type OverlayDefs = Vec<(Vec<ObjectId>, char)>;

#[derive(Clone, Debug)]
struct VisualShapeTable {
    axis: String,
    entries: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct VisualShapeRotation {
    map: String,
    from: String,
}

impl VisualShapeRotation {
    fn new(map: &str, from: &str) -> Self {
        Self {
            map: map.to_string(),
            from: from.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct VisualColorTable {
    axis: String,
    entries: HashMap<String, String>,
}

#[derive(Clone, Debug)]
enum VisualPalette {
    Plain(Vec<String>),
    Table {
        axis: String,
        entries: HashMap<String, Vec<String>>,
    },
}

fn parse_visuals_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let mut shapes = HashMap::<String, VisualShapeTable>::new();
    let mut plain_shapes = HashMap::<String, Vec<String>>::new();
    let mut palettes = HashMap::<String, VisualPalette>::new();
    let mut color_aliases = HashMap::<String, String>::new();
    let mut colors = HashMap::<String, VisualColorTable>::new();
    let mut i = start + 1;

    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            ["colors"] => {
                i = parse_visual_colors_block(lines, i, catalog, &mut color_aliases, &mut colors)?;
            }
            ["palettes"] => {
                i = parse_visual_palettes_block(lines, i, catalog, &mut palettes)?;
            }
            ["shapes"] => {
                i = parse_visual_shapes_block(lines, i, catalog, &mut plain_shapes, &mut shapes)?;
            }
            ["shape", table_ref] => {
                if !table_ref.contains(':') {
                    if plain_shapes.contains_key(*table_ref) {
                        return Err(parse_error(line, "duplicate visual shape"));
                    }
                    let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                    plain_shapes.insert((*table_ref).to_string(), pattern);
                    i = next_i;
                    continue;
                }
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let (table, next_i) = parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::new("rotate", from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::new(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::new(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["colors", table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            [selector, source] if is_visual_image_source(source) => {
                add_image_visuals(selector, line, source, catalog, visuals)?;
                i += 1;
            }
            [selector, color] if is_visual_color_expr_token(color, &color_aliases, &colors) => {
                add_solid_visuals(
                    selector,
                    line,
                    color,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
                i += 1;
            }
            [selector] => {
                if let Some(next_i) = parse_palette_shape_sprite_entry(
                    lines,
                    i,
                    selector,
                    &palettes,
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )? {
                    i = next_i;
                    continue;
                }
                if let Some(next_i) = parse_canonical_sprite_entry(
                    lines,
                    i,
                    selector,
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )? {
                    i = next_i;
                    continue;
                }
                if let Some(source) = parse_line_style_image_sprite_source(lines, i) {
                    add_image_visuals(selector, line, source, catalog, visuals)?;
                    i += 2;
                    continue;
                }
                if let Some((shape_name, shape_value, color_exprs, offset, next_i)) =
                    parse_ps_style_shape_sprite(lines, i, line, &plain_shapes, &shapes, catalog)?
                {
                    if let Some(shape) = shapes.get(&shape_name) {
                        add_ascii_visuals(
                            selector,
                            line,
                            shape,
                            &shape_value,
                            &color_exprs,
                            offset,
                            None,
                            &color_aliases,
                            &colors,
                            catalog,
                            visuals,
                        )?;
                    } else {
                        let pattern = plain_shapes
                            .get(&shape_name)
                            .ok_or_else(|| parse_error(line, "unknown sprite shape"))?
                            .clone();
                        add_inline_ascii_visuals(
                            selector,
                            line,
                            &pattern,
                            &color_exprs,
                            offset,
                            None,
                            &color_aliases,
                            &colors,
                            catalog,
                            visuals,
                        )?;
                    }
                    i = next_i;
                    continue;
                }
                let (color_exprs, pattern, offset, next_i) =
                    parse_line_style_inline_sprite(lines, i, &color_aliases, &colors, catalog)?;
                if pattern.is_empty() {
                    let [(_, color)] = color_exprs.as_slice() else {
                        return Err(parse_error(line, "solid sprite requires exactly one color"));
                    };
                    add_solid_visuals(
                        selector,
                        line,
                        color,
                        &color_aliases,
                        &colors,
                        catalog,
                        visuals,
                    )?;
                } else {
                    add_inline_ascii_visuals(
                        selector,
                        line,
                        &pattern,
                        &color_exprs,
                        offset,
                        None,
                        &color_aliases,
                        &colors,
                        catalog,
                        visuals,
                    )?;
                }
                i = next_i;
            }
            [other, ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown sprites directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "sprites missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_colors_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    color_aliases: &mut HashMap<String, String>,
    colors: &mut HashMap<String, VisualColorTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", color] => {
                color_aliases.insert((*name).to_string(), (*color).to_string());
                i += 1;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "colors row must be: <name> = <color> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "colors missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_palettes_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    palettes: &mut HashMap<String, VisualPalette>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", colors @ ..] => {
                if colors.is_empty() {
                    return Err(parse_error(
                        line,
                        "palette row must name at least one color",
                    ));
                }
                palettes.insert(
                    (*name).to_string(),
                    VisualPalette::Plain(colors.iter().map(|value| (*value).to_string()).collect()),
                );
                i += 1;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let values = catalog_value_set(catalog, &axis).ok_or_else(|| {
                    parse_error(line, "palette tag set must name an existing tag set")
                })?;
                let mut entries = HashMap::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row = &lines[i];
                    let row_tokens = split_header_tokens(row);
                    let [value, "=", colors @ ..] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            row,
                            "palette row must be: <value> = <color...>",
                        ));
                    };
                    if colors.is_empty() {
                        return Err(parse_error(row, "palette row must name at least one color"));
                    }
                    if !values.iter().any(|candidate| candidate == value) {
                        return Err(parse_error(row, "palette value is not in tag set"));
                    }
                    entries.insert(
                        (*value).to_string(),
                        colors.iter().map(|value| (*value).to_string()).collect(),
                    );
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(line, "palette table missing closing brace"));
                }
                palettes.insert(name, VisualPalette::Table { axis, entries });
                i += 1;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "palette row must be: <name> = <color...> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "palettes missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_shapes_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    plain_shapes: &mut HashMap<String, Vec<String>>,
    shapes: &mut HashMap<String, VisualShapeTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if !name.contains(':') => {
                let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                plain_shapes.insert((*name).to_string(), pattern);
                i = next_i;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let (table, next_i) = parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::new("rotate", from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::new(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::new(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "shape row must be: <name> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "shapes missing closing brace"));
    }
    Ok(i + 1)
}

#[allow(clippy::too_many_arguments)]
fn parse_palette_shape_sprite_entry(
    lines: &[String],
    start: usize,
    selector: &str,
    palettes: &HashMap<String, VisualPalette>,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<Option<usize>, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i >= lines.len() || is_block_close_line(&lines[i]) {
        return Ok(None);
    }

    let first_tokens = split_header_tokens(&lines[i]);
    let first_is_new_style = matches!(first_tokens.as_slice(), ["palette", _] | ["shape", _])
        || first_tokens
            .first()
            .is_some_and(|token| palettes.contains_key(*token));
    if !first_is_new_style {
        return Ok(None);
    }

    let mut palette_ref = None::<String>;
    let mut shape_ref = None::<String>;
    let mut consumed_rows = 0usize;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if consumed_rows > 0 && starts_palette_shape_sprite_entry(lines, i, catalog) {
            break;
        }
        match tokens.as_slice() {
            [] => {}
            ["palette", value] => palette_ref = Some((*value).to_string()),
            ["shape", value] => shape_ref = Some((*value).to_string()),
            [value] if palettes.contains_key(*value) => palette_ref = Some((*value).to_string()),
            _ => {
                return Err(parse_error(
                    line,
                    "sprite entry row must be: palette <name> | shape <name>",
                ));
            }
        }
        consumed_rows += 1;
        i += 1;
    }

    let palette_ref =
        palette_ref.ok_or_else(|| parse_error(&lines[start], "sprite entry missing palette"))?;
    let targets = expand_visual_selector(selector, &lines[start], catalog)?;
    for target in targets {
        let palette = resolve_visual_palette(
            &palette_ref,
            &target.bindings,
            palettes,
            color_aliases,
            color_tables,
            &catalog.maps,
            &lines[start],
        )?;
        let pattern = match &shape_ref {
            Some(shape_ref) => Some(resolve_visual_shape(
                shape_ref,
                &target.bindings,
                plain_shapes,
                shapes,
                &catalog.maps,
                &lines[start],
            )?),
            None => None,
        };
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        if let Some(pattern) = pattern {
            validate_visual_pattern_palette(&pattern, &palette, &lines[start])?;
            visuals.sprites.push(VisualSpriteDef {
                name: sprite,
                offset: VisualSpriteOffset::default(),
                pixels_per_cell: None,
                kind: VisualSpriteKind::Ascii {
                    pattern,
                    colors: palette
                        .into_iter()
                        .map(|(token, color)| VisualColorDef { token, color })
                        .collect(),
                },
            });
        } else {
            let [(.., color)] = palette.as_slice() else {
                return Err(parse_error(
                    &lines[start],
                    "solid sprite requires exactly one color",
                ));
            };
            visuals.sprites.push(VisualSpriteDef {
                name: sprite,
                offset: VisualSpriteOffset::default(),
                pixels_per_cell: None,
                kind: VisualSpriteKind::Solid(color.clone()),
            });
        }
    }
    let next_i = if lines.get(i).map(String::as_str) == Some(BLOCK_CLOSE)
        && lines
            .get(i + 1)
            .is_some_and(|next| !starts_visual_outer_section(&split_header_tokens(next)))
    {
        i + 1
    } else {
        i
    };
    Ok(Some(next_i))
}

#[allow(clippy::too_many_arguments)]
fn parse_canonical_sprite_entry(
    lines: &[String],
    start: usize,
    selector: &str,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<Option<usize>, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i >= lines.len() || is_block_close_line(&lines[i]) {
        return Ok(None);
    }

    let first_tokens = split_header_tokens(&lines[i]);
    let canonical_start = match first_tokens.as_slice() {
        ["colors", ..]
        | ["pixels_per_cell", ..]
        | ["offset", ..]
        | ["shape", ..]
        | ["rotate", ..] => true,
        _ => false,
    };
    if !canonical_start {
        return Ok(None);
    }

    let mut color_exprs = None::<Vec<(char, String)>>;
    let mut offset = VisualSpriteOffset::default();
    let mut pixels_per_cell = None::<VisualSpritePixelsPerCell>;
    let mut shape_ref = None::<(String, ValueExpr)>;
    let mut inline_pattern = None::<Vec<String>>;
    let mut rotation = None::<VisualShapeRotation>;

    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["colors", colors @ ..] => {
                if colors.is_empty() {
                    return Err(parse_error(line, "sprite colors row must name colors"));
                }
                color_exprs = Some(visual_colors_from_tokens(colors, line)?);
                i += 1;
            }
            ["pixels_per_cell", width, height] => {
                pixels_per_cell = Some(VisualSpritePixelsPerCell {
                    width: parse_positive_u32(width, line, "pixels_per_cell width")?,
                    height: parse_positive_u32(height, line, "pixels_per_cell height")?,
                });
                i += 1;
            }
            ["offset", x, y] => {
                offset = VisualSpriteOffset {
                    x: parse_i32_value(x, line, "sprite offset x")?,
                    y: parse_i32_value(y, line, "sprite offset y")?,
                };
                i += 1;
            }
            _ if is_visual_translate_transform_row(line) => {
                let mut next_i = i;
                let transform_offset =
                    parse_visual_transform_offset(lines, &mut next_i, Some(catalog))?;
                offset.x += transform_offset.x;
                offset.y += transform_offset.y;
                i = next_i;
            }
            ["shape", shape] => {
                shape_ref = Some(parse_ps_style_shape_ref(shape, line)?);
                i += 1;
            }
            ["shape"] => {
                let (pattern, next_i) = parse_visual_rows_until_close(lines, i + 1, start)?;
                inline_pattern = Some(pattern);
                i = next_i;
            }
            ["rotate", ..] => {
                let Some(parsed_rotation) = parse_visual_shape_rotation_directive(line)? else {
                    return Err(parse_error(
                        line,
                        "sprite rotation must be: rotate from <value>",
                    ));
                };
                rotation = Some(parsed_rotation);
                i += 1;
            }
            [first, ..]
                if color_exprs.is_none()
                    && (is_visual_color_token(first)
                        || parse_visual_table_expr(first, line).is_ok()) =>
            {
                color_exprs = Some(visual_colors_from_row(line)?);
                i += 1;
            }
            [shape_ref_token]
                if color_exprs.is_some()
                    && inline_pattern.is_none()
                    && shape_ref.is_none()
                    && visual_shape_ref_exists(shape_ref_token, plain_shapes, shapes, line)? =>
            {
                shape_ref = Some(parse_ps_style_shape_ref(shape_ref_token, line)?);
                i += 1;
            }
            [_] if color_exprs.is_some() && inline_pattern.is_none() && shape_ref.is_none() => {
                let (pattern, next_i) = parse_visual_rows_until_close(lines, i, start)?;
                inline_pattern = Some(pattern);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sprite row must be colors, pixels_per_cell, offset, shape, or rotate",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "canonical sprite entry missing closing brace",
        ));
    }

    let color_exprs =
        color_exprs.ok_or_else(|| parse_error(&lines[start], "sprite entry missing colors"))?;
    let next_i = i + 1;
    if let Some(rotation) = rotation {
        let Some(pattern) = inline_pattern else {
            return Err(parse_error(
                &lines[start],
                "sprite rotation requires inline ASCII rows",
            ));
        };
        validate_visual_pattern_palette(&pattern, &color_exprs, &lines[start])?;
        let targets = expand_visual_selector(selector, &lines[start], catalog)?;
        let axis = visual_rotation_axis_for_targets(&targets, catalog, &rotation, &lines[start])?;
        let mut entries = HashMap::new();
        entries.insert(rotation.from.clone(), pattern);
        let values = catalog_value_set(catalog, &axis)
            .ok_or_else(|| parse_error(&lines[start], "visual rotation tag set must exist"))?;
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            &axis,
            &rotation,
            &lines[start],
        )?;
        let shape = VisualShapeTable { axis, entries };
        add_ascii_visuals(
            selector,
            &lines[start],
            &shape,
            &ValueExpr::Binding(shape.axis.clone()),
            &color_exprs,
            offset,
            pixels_per_cell,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else if let Some((shape_name, shape_value)) = shape_ref {
        if let Some(shape) = shapes.get(&shape_name) {
            add_ascii_visuals(
                selector,
                &lines[start],
                shape,
                &shape_value,
                &color_exprs,
                offset,
                pixels_per_cell,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        } else {
            let pattern = plain_shapes
                .get(&shape_name)
                .ok_or_else(|| parse_error(&lines[start], "unknown sprite shape"))?;
            add_inline_ascii_visuals(
                selector,
                &lines[start],
                pattern,
                &color_exprs,
                offset,
                pixels_per_cell,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        }
    } else if let Some(pattern) = inline_pattern {
        add_inline_ascii_visuals(
            selector,
            &lines[start],
            &pattern,
            &color_exprs,
            offset,
            pixels_per_cell,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else {
        return Err(parse_error(&lines[start], "sprite entry missing shape"));
    }

    Ok(Some(next_i))
}

fn visual_colors_from_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Vec<(char, String)>, DiagnosticReport> {
    tokens
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "sprite supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect()
}

fn parse_positive_u32(value: &str, line: &str, label: &str) -> Result<u32, DiagnosticReport> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| parse_error(line, &format!("{label} must be a positive integer")))?;
    if parsed == 0 {
        return Err(parse_error(
            line,
            &format!("{label} must be a positive integer"),
        ));
    }
    Ok(parsed)
}

fn parse_i32_value(value: &str, line: &str, label: &str) -> Result<i32, DiagnosticReport> {
    value
        .parse::<i32>()
        .map_err(|_| parse_error(line, &format!("{label} must be an integer")))
}

fn parse_visual_rows_until_close(
    lines: &[String],
    mut i: usize,
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut pattern = Vec::new();
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        if !pattern.is_empty() && is_visual_translate_transform_row(&lines[i]) {
            break;
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape rows missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[i])?;
    Ok((pattern, i))
}

fn visual_rotation_axis_for_targets(
    targets: &[VisualSelectorTarget],
    catalog: &Catalog,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<String, DiagnosticReport> {
    let first = targets
        .first()
        .ok_or_else(|| parse_error(line, "visual selector matched no objects"))?;
    let mut candidates = first
        .bindings
        .keys()
        .filter(|axis| {
            catalog_value_set(catalog, axis)
                .is_some_and(|values| values.iter().any(|value| value == &rotation.from))
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.retain(|axis| {
        targets
            .iter()
            .all(|target| target.bindings.contains_key(axis))
    });
    let [axis] = candidates.as_slice() else {
        return Err(parse_error(
            line,
            "sprite rotation requires exactly one matching selector tag set",
        ));
    };
    Ok(axis.clone())
}

fn starts_palette_shape_sprite_entry(lines: &[String], index: usize, catalog: &Catalog) -> bool {
    let tokens = split_header_tokens(&lines[index]);
    let [selector] = tokens.as_slice() else {
        return false;
    };
    if expand_visual_selector(selector, &lines[index], catalog).is_err() {
        return false;
    }
    let Some(next) = lines.get(index + 1) else {
        return false;
    };
    matches!(
        split_header_tokens(next).as_slice(),
        ["palette", _] | ["shape", _]
    )
}

fn resolve_visual_palette(
    value: &str,
    bindings: &HashMap<String, String>,
    palettes: &HashMap<String, VisualPalette>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<Vec<(char, String)>, DiagnosticReport> {
    let colors = if let Some((name, expr)) = parse_visual_table_expr(value, line).ok() {
        let Some(VisualPalette::Table { axis, entries }) = palettes.get(&name) else {
            return Err(parse_error(line, "unknown visual palette"));
        };
        let key = visual_table_key(&expr, axis, entries, bindings, maps, line)?;
        entries
            .get(&key)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual palette value missing"))?
    } else {
        match palettes.get(value) {
            Some(VisualPalette::Plain(colors)) => colors.clone(),
            Some(VisualPalette::Table { .. }) => {
                return Err(parse_error(
                    line,
                    "visual palette table requires a tag value",
                ));
            }
            None => return Err(parse_error(line, "unknown visual palette")),
        }
    };
    colors
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "sprite palette supports at most 62 colors"))?;
            Ok((
                token,
                resolve_visual_color_expr_with_aliases(
                    color,
                    bindings,
                    color_aliases,
                    color_tables,
                    maps,
                    line,
                )?,
            ))
        })
        .collect()
}

fn resolve_visual_shape(
    value: &str,
    bindings: &HashMap<String, String>,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    if let Some((name, expr)) = parse_visual_table_expr(value, line).ok() {
        let shape = shapes
            .get(&name)
            .ok_or_else(|| parse_error(line, "unknown visual shape"))?;
        let key = visual_table_key(&expr, &shape.axis, &shape.entries, bindings, maps, line)?;
        return shape
            .entries
            .get(&key)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual shape value missing"));
    }
    plain_shapes
        .get(value)
        .cloned()
        .ok_or_else(|| parse_error(line, "unknown visual shape"))
}

fn visual_table_key<T>(
    expr: &ValueExpr,
    axis: &str,
    entries: &HashMap<String, T>,
    bindings: &HashMap<String, String>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let ValueExpr::Binding(name) = expr {
        if let Some(value) = bindings.get(name) {
            return Ok(value.clone());
        }
        if name == axis
            && let Some(value) = bindings.get(axis)
        {
            return Ok(value.clone());
        }
        if entries.contains_key(name) {
            return Ok(name.clone());
        }
    }
    let env = visual_value_env(bindings);
    if value_expr_result_axis(expr, &env, maps, line)? != axis {
        return Err(parse_error(line, "visual table tag set mismatch"));
    }
    eval_bound_value_expr(expr, &env, maps, line)
}

fn parse_visual_plain_shape(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut pattern = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    Ok((pattern, i + 1))
}

fn parse_ps_style_shape_sprite(
    lines: &[String],
    start: usize,
    line: &str,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    catalog: &Catalog,
) -> Result<
    Option<(
        String,
        ValueExpr,
        Vec<(char, String)>,
        VisualSpriteOffset,
        usize,
    )>,
    DiagnosticReport,
> {
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i >= lines.len() || is_block_close_line(&lines[i]) {
        return Ok(None);
    }
    let colors = visual_colors_from_row(&lines[i])?;
    if colors.is_empty() {
        return Err(parse_error(
            &lines[i],
            "PS-style reusable sprite missing color row",
        ));
    }
    i += 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    let tokens = split_header_tokens(lines.get(i).map_or("", String::as_str));
    let shape_ref = match tokens.as_slice() {
        [shape_ref] if visual_shape_ref_exists(shape_ref, plain_shapes, shapes, &lines[i])? => {
            *shape_ref
        }
        ["ascii", _] => {
            return Err(parse_error(
                &lines[i],
                "sprite shape refs are bare; remove `ascii`",
            ));
        }
        _ => return Ok(None),
    };
    let shape_line_index = i;
    let (shape_name, shape_value) = parse_ps_style_shape_ref(shape_ref, &lines[shape_line_index])?;
    let mut next_i = shape_line_index + 1;
    let offset = parse_visual_transform_offset(lines, &mut next_i, Some(catalog))?;
    while next_i < lines.len() && lines[next_i].is_empty() {
        next_i += 1;
    }
    if next_i >= lines.len() || !is_block_close_line(&lines[next_i]) {
        return Err(parse_error(
            line,
            "PS-style reusable sprite missing closing brace",
        ));
    }
    Ok(Some((shape_name, shape_value, colors, offset, next_i + 1)))
}

fn visual_shape_ref_exists(
    shape_ref: &str,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let (shape_name, _) = parse_ps_style_shape_ref(shape_ref, line)?;
    Ok(plain_shapes.contains_key(&shape_name) || shapes.contains_key(&shape_name))
}

fn parse_ps_style_shape_ref(
    shape_ref: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let (shape_name, shape_value) = if shape_ref.contains(':') {
        parse_visual_table_expr(shape_ref, line)?
    } else {
        (shape_ref.to_string(), ValueExpr::Binding(String::new()))
    };
    Ok((shape_name, shape_value))
}

fn parse_line_style_inline_sprite(
    lines: &[String],
    start: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
) -> Result<(Vec<(char, String)>, Vec<String>, VisualSpriteOffset, usize), DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    if i >= lines.len() || is_block_close_line(&lines[i]) {
        return Err(parse_error(
            &lines[start],
            "PS-style sprite missing color row",
        ));
    }

    let colors = visual_colors_from_row(&lines[i])?;
    if colors.is_empty() {
        return Err(parse_error(&lines[i], "PS-style sprite missing color row"));
    }

    let mut pattern = Vec::new();
    i += 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if is_visual_entry_boundary_with_colors(lines, i, color_aliases, color_tables, catalog) {
            break;
        }
        if !pattern.is_empty() && is_visual_translate_transform_row(&lines[i]) {
            break;
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "PS-style sprite row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "PS-style sprite missing closing brace",
        ));
    }
    if !pattern.is_empty() {
        validate_visual_pattern(&pattern, &lines[start])?;
    }
    let offset = parse_visual_transform_offset(lines, &mut i, Some(catalog))?;
    if visual_end_closes_sprite_entry(lines, i) {
        i += 1;
    }
    Ok((colors, pattern, offset, i))
}

fn parse_visual_transform_offset(
    lines: &[String],
    index: &mut usize,
    catalog: Option<&Catalog>,
) -> Result<VisualSpriteOffset, DiagnosticReport> {
    let mut offset = VisualSpriteOffset::default();
    while *index < lines.len() && is_visual_translate_transform_row(&lines[*index]) {
        if catalog.is_some_and(|catalog| is_visual_entry_boundary(lines, *index, catalog)) {
            break;
        }
        for token in split_header_tokens(&lines[*index]) {
            let Some((direction, amount)) =
                parse_visual_translate_transform(token, &lines[*index])?
            else {
                return Err(parse_error(
                    &lines[*index],
                    "only translate:<direction>:<pixels> sprite transforms are supported",
                ));
            };
            match direction {
                VisualDirection::Up => offset.y -= amount,
                VisualDirection::Right => offset.x += amount,
                VisualDirection::Down => offset.y += amount,
                VisualDirection::Left => offset.x -= amount,
            }
        }
        *index += 1;
    }
    Ok(offset)
}

fn is_visual_translate_transform_row(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| token.to_ascii_lowercase().starts_with("translate:"))
}

#[derive(Clone, Copy)]
enum VisualDirection {
    Up,
    Right,
    Down,
    Left,
}

fn parse_visual_translate_transform(
    token: &str,
    line: &str,
) -> Result<Option<(VisualDirection, i32)>, DiagnosticReport> {
    let lower = token.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("translate:") else {
        return Ok(None);
    };
    let mut parts = rest.split(':');
    let direction = parts
        .next()
        .ok_or_else(|| parse_error(line, "translate transform missing direction"))?;
    let amount = parts
        .next()
        .ok_or_else(|| parse_error(line, "translate transform missing pixel amount"))?;
    if parts.next().is_some() {
        return Err(parse_error(line, "translate transform has too many fields"));
    }
    let amount = amount
        .parse::<i32>()
        .map_err(|_| parse_error(line, "translate transform pixel amount must be an integer"))?;
    let direction = match direction {
        "up" | "^" => VisualDirection::Up,
        "right" | ">" => VisualDirection::Right,
        "down" | "v" => VisualDirection::Down,
        "left" | "<" => VisualDirection::Left,
        _ => return Err(parse_error(line, "unknown translate transform direction")),
    };
    Ok(Some((direction, amount)))
}

fn visual_colors_from_row(line: &str) -> Result<Vec<(char, String)>, DiagnosticReport> {
    split_header_tokens(line)
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "PS-style sprite supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()
}

fn validate_visual_pattern_palette(
    pattern: &[String],
    color_exprs: &[(char, String)],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let colors = color_exprs
        .iter()
        .map(|(token, _)| *token)
        .collect::<HashSet<_>>();
    for row in pattern {
        for token in row.chars() {
            if token == '.' || colors.contains(&token) {
                continue;
            }
            return Err(parse_error(
                line,
                "sprite pattern references a color outside the color row",
            ));
        }
    }
    Ok(())
}

pub(crate) fn is_visual_color_token(value: &str) -> bool {
    value.starts_with('#') || crate::syntax::is_visual_named_color(value)
}

fn is_visual_color_expr_token(
    value: &str,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
) -> bool {
    if is_visual_color_token(value) || color_aliases.contains_key(value) {
        return true;
    }
    parse_visual_table_expr(value, value)
        .ok()
        .is_some_and(|(name, _)| color_tables.contains_key(&name))
}

fn parse_line_style_image_sprite_source(lines: &[String], start: usize) -> Option<&str> {
    let mut i = start + 1;
    while i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    let tokens = split_header_tokens(lines.get(i)?.as_str());
    let [source] = tokens.as_slice() else {
        return None;
    };
    is_visual_image_source(source).then_some(*source)
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn is_visual_entry_boundary(lines: &[String], index: usize, catalog: &Catalog) -> bool {
    is_visual_entry_boundary_with_colors(lines, index, &HashMap::new(), &HashMap::new(), catalog)
}

fn is_visual_entry_boundary_with_colors(
    lines: &[String],
    index: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
) -> bool {
    let tokens = split_header_tokens(&lines[index]);
    match tokens.as_slice() {
        ["shape", ..] | ["colors", ..] => true,
        [_, source]
            if is_visual_image_source(source)
                || is_visual_color_expr_token(source, color_aliases, color_tables) =>
        {
            true
        }
        [selector] => {
            if starts_line_style_visual_entry(lines, index, color_aliases, color_tables) {
                return true;
            }
            if expand_visual_selector(selector, &lines[index], catalog).is_err() {
                return false;
            }
            let Some(next) = lines.get(index + 1) else {
                return false;
            };
            if is_block_close_line(next) {
                return false;
            }
            let next_tokens = split_header_tokens(next);
            match next_tokens.as_slice() {
                [source] if is_visual_image_source(source) => true,
                [color, ..] if is_visual_color_expr_token(color, color_aliases, color_tables) => {
                    true
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn starts_line_style_visual_entry(
    lines: &[String],
    index: usize,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
) -> bool {
    let tokens = split_header_tokens(&lines[index]);
    let [selector] = tokens.as_slice() else {
        return false;
    };
    if selector.starts_with("translate:") {
        return false;
    }
    if !selector.contains(':') {
        return false;
    }
    let Some(next) = lines.get(index + 1) else {
        return false;
    };
    let next_tokens = split_header_tokens(next);
    match next_tokens.as_slice() {
        [source] if is_visual_image_source(source) => true,
        [color, ..] if is_visual_color_expr_token(color, color_aliases, color_tables) => true,
        _ => false,
    }
}

fn visual_end_closes_sprite_entry(lines: &[String], index: usize) -> bool {
    if !matches!(lines.get(index).map(String::as_str), Some(BLOCK_CLOSE)) {
        return false;
    }
    let Some(next) = lines.get(index + 1) else {
        return false;
    };
    is_block_close_line(next) || !starts_visual_outer_section(&split_header_tokens(next))
}

fn starts_visual_outer_section(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["map", ..]
            | ["on_level_start"]
            | ["on_level_clear"]
            | ["on_last_level_clear"]
            | ["on_display"]
            | ["scratch"]
            | ["groups"]
            | ["layers"]
            | ["collision_layers"]
            | ["legend"]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["sprites"]
            | ["sounds"]
            | ["screen"]
            | ["layout", ..]
            | ["keys"]
            | ["routine", ..]
            | ["rule", ..]
            | ["rules"]
            | ["levels"]
            | ["level", ..]
    )
}

pub(crate) fn visual_color_token_for_index(index: usize) -> Option<char> {
    const TOKENS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    TOKENS.get(index).map(|token| *token as char)
}

fn parse_visual_table_ref(value: &str, line: &str) -> Result<(String, String), DiagnosticReport> {
    let Some((name, axis)) = value.split_once(':') else {
        return Err(parse_error(line, "visual table must be: <name>:<tag_set>"));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    if !is_identifier(axis) {
        return Err(parse_error(
            line,
            "visual table tag set must be an identifier",
        ));
    }
    Ok((name.to_string(), axis.to_string()))
}

fn parse_visual_table_expr(
    value: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let Some((name, value)) = value.split_once(':') else {
        return Err(parse_error(
            line,
            "visual table must be: <name>:<value-expr>",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    Ok((name.to_string(), parse_value_expr(value, line)?))
}

fn parse_visual_shape_table(
    lines: &[String],
    start: usize,
    axis: &str,
    rotation: Option<VisualShapeRotation>,
    catalog: &Catalog,
) -> Result<(VisualShapeTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual shape tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    if let Some(rotation) = rotation {
        let mut pattern = Vec::new();
        while i < lines.len() && !is_block_close_line(&lines[i]) {
            let row_tokens = split_header_tokens(&lines[i]);
            let [row] = row_tokens.as_slice() else {
                return Err(parse_error(
                    &lines[i],
                    "visual shape row must be a single token row",
                ));
            };
            pattern.push((*row).to_string());
            i += 1;
        }
        if i >= lines.len() {
            return Err(parse_error(
                &lines[start],
                "visual shape missing closing brace",
            ));
        }
        validate_visual_pattern(&pattern, &lines[i])?;
        entries.insert(rotation.from.clone(), pattern);
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
        return Ok((
            VisualShapeTable {
                axis: axis.to_string(),
                entries,
            },
            i + 1,
        ));
    }
    let mut rotation = None::<VisualShapeRotation>;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if let Some(parsed_rotation) = parse_visual_shape_rotation_directive(&lines[i])? {
            if rotation.is_some() {
                return Err(parse_error(&lines[i], "duplicate visual shape rotation"));
            }
            if lines[i].trim_end().ends_with('{') {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                if entries
                    .insert(parsed_rotation.from.clone(), pattern)
                    .is_some()
                {
                    return Err(parse_error(
                        &lines[i],
                        "visual shape rotation source duplicates explicit shape value",
                    ));
                }
                rotation = Some(parsed_rotation);
                i += 1;
                continue;
            }
            if !entries.contains_key(&parsed_rotation.from) {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                entries.insert(parsed_rotation.from.clone(), pattern);
                rotation = Some(parsed_rotation);
                continue;
            }
            rotation = Some(parsed_rotation);
            i += 1;
            continue;
        }
        let value = block_header_text(&lines[i]);
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual shape value is not in tag set",
            ));
        }
        let mut pattern = Vec::new();
        i += 1;
        while i < lines.len() && !is_block_close_line(&lines[i]) {
            let row_tokens = split_header_tokens(&lines[i]);
            let [row] = row_tokens.as_slice() else {
                return Err(parse_error(
                    &lines[i],
                    "visual shape row must be a single token row",
                ));
            };
            pattern.push((*row).to_string());
            i += 1;
        }
        if i >= lines.len() {
            return Err(parse_error(
                &lines[start],
                "visual shape value missing closing brace",
            ));
        }
        validate_visual_pattern(&pattern, &lines[i])?;
        if entries.insert(value.to_string(), pattern).is_some() {
            return Err(parse_error(&lines[i], "duplicate visual shape value"));
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    if let Some(rotation) = rotation {
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
    }
    Ok((
        VisualShapeTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn parse_visual_shape_rotation_directive(
    line: &str,
) -> Result<Option<VisualShapeRotation>, DiagnosticReport> {
    let tokens = split_header_tokens(block_header_text(line));
    match tokens.as_slice() {
        ["rotate", "from", from] => Ok(Some(VisualShapeRotation::new("rotate", from))),
        ["rotate", "using", map, "from", from] => Ok(Some(VisualShapeRotation::new(map, from))),
        ["rotate", map, "from", from] => Ok(Some(VisualShapeRotation::new(map, from))),
        ["rotate", ..] => Err(parse_error(
            line,
            "visual shape rotation must be: rotate from <value> | rotate using <map> from <value>",
        )),
        _ => Ok(None),
    }
}

fn validate_visual_pattern(pattern: &[String], line: &str) -> Result<(), DiagnosticReport> {
    if pattern.is_empty() {
        return Err(parse_error(
            line,
            "visual shape value requires at least one row",
        ));
    }
    let width = pattern[0].chars().count();
    if width == 0
        || pattern
            .iter()
            .any(|row| row.chars().count() != width || !row.is_ascii())
    {
        return Err(parse_error(
            line,
            "visual shape rows must be equal-width ascii",
        ));
    }
    if pattern.iter().any(|row| row.contains(['{', '}'])) {
        return Err(parse_error(line, "ASCII rows cannot contain braces"));
    }
    Ok(())
}

fn expand_visual_shape_rotations(
    entries: &mut HashMap<String, Vec<String>>,
    values: &[String],
    catalog: &Catalog,
    axis: &str,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if !values.iter().any(|value| value == &rotation.from) {
        return Err(parse_error(
            line,
            "visual rotation source is not in tag set",
        ));
    }
    let map = catalog
        .maps
        .get(&rotation.map)
        .ok_or_else(|| parse_error(line, "unknown visual rotation map"))?;
    if map.axis != axis {
        return Err(parse_error(line, "visual rotation map tag set mismatch"));
    }
    let mut value = rotation.from.clone();
    let mut pattern = entries
        .get(&value)
        .cloned()
        .ok_or_else(|| parse_error(line, "visual rotation source shape missing"))?;
    let mut visited = Vec::new();

    loop {
        if visited.iter().any(|seen| seen == &value) {
            break;
        }
        visited.push(value.clone());
        let next = map
            .values
            .get(&value)
            .ok_or_else(|| parse_error(line, "visual rotation map value missing"))?
            .clone();
        let next_pattern = rotate_visual_pattern_clockwise(&pattern);
        if next == rotation.from {
            break;
        }
        if let Some(existing) = entries.get(&next) {
            if existing != &next_pattern {
                return Err(parse_error(
                    line,
                    "visual rotation conflicts with explicit shape value",
                ));
            }
        } else {
            entries.insert(next.clone(), next_pattern.clone());
        }
        value = next;
        pattern = next_pattern;
    }

    if visited.len() != values.len() || values.iter().any(|value| !entries.contains_key(value)) {
        return Err(parse_error(
            line,
            "visual rotation map must cycle through every shape tag value",
        ));
    }
    Ok(())
}

fn rotate_visual_pattern_clockwise(pattern: &[String]) -> Vec<String> {
    let rows = pattern
        .iter()
        .map(|row| row.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let height = rows.len();
    let width = rows.first().map_or(0, Vec::len);
    let mut rotated = Vec::with_capacity(width);
    for x in 0..width {
        let mut row = String::with_capacity(height);
        for y in (0..height).rev() {
            row.push(rows[y][x]);
        }
        rotated.push(row);
    }
    rotated
}

fn parse_visual_color_table(
    lines: &[String],
    start: usize,
    axis: &str,
    catalog: &Catalog,
) -> Result<(VisualColorTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual colors tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        let [value, "=", color] = tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual color row must be: <value> = <color>",
            ));
        };
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual color value is not in tag set",
            ));
        }
        if entries
            .insert((*value).to_string(), (*color).to_string())
            .is_some()
        {
            return Err(parse_error(&lines[i], "duplicate visual color value"));
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual colors missing closing brace",
        ));
    }
    Ok((
        VisualColorTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn add_ascii_visuals(
    selector: &str,
    line: &str,
    shape: &VisualShapeTable,
    shape_value_expr: &ValueExpr,
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    pixels_per_cell: Option<VisualSpritePixelsPerCell>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let env = visual_value_env(&target.bindings);
        if value_expr_result_axis(shape_value_expr, &env, &catalog.maps, line)? != shape.axis {
            return Err(parse_error(line, "visual shape tag set mismatch"));
        }
        let shape_value = eval_bound_value_expr(shape_value_expr, &env, &catalog.maps, line)?;
        if !catalog_value_set(catalog, &shape.axis)
            .is_some_and(|values| values.iter().any(|value| value == &shape_value))
        {
            return Err(parse_error(line, "visual shape value is not in tag set"));
        }
        let pattern = shape
            .entries
            .get(&shape_value)
            .ok_or_else(|| parse_error(line, "visual shape value missing"))?
            .clone();
        validate_visual_pattern_palette(&pattern, color_exprs, line)?;
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            pixels_per_cell,
            kind: VisualSpriteKind::Ascii { pattern, colors },
        });
    }
    Ok(())
}

fn add_inline_ascii_visuals(
    selector: &str,
    line: &str,
    pattern: &[String],
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    pixels_per_cell: Option<VisualSpritePixelsPerCell>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    validate_visual_pattern_palette(pattern, color_exprs, line)?;
    for target in expand_visual_selector(selector, line, catalog)? {
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            pixels_per_cell,
            kind: VisualSpriteKind::Ascii {
                pattern: pattern.to_vec(),
                colors,
            },
        });
    }
    Ok(())
}

fn add_solid_visuals(
    selector: &str,
    line: &str,
    color_expr: &str,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        let color = resolve_visual_color_expr(
            color_expr,
            &target.bindings,
            color_aliases,
            color_tables,
            &catalog.maps,
            line,
        )?;
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset: VisualSpriteOffset::default(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Solid(color),
        });
    }
    Ok(())
}

fn add_image_visuals(
    selector: &str,
    line: &str,
    source: &str,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset: VisualSpriteOffset::default(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Image {
                source: source.to_string(),
            },
        });
    }
    Ok(())
}

fn resolve_visual_color_expr(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    resolve_visual_color_expr_with_aliases(expr, bindings, color_aliases, color_tables, maps, line)
}

fn resolve_visual_color_expr_with_aliases(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let Some(color) = color_aliases.get(expr) {
        return Ok(color.clone());
    }
    if let Some((name, value_expr)) = parse_visual_table_expr(expr, line).ok() {
        let table = color_tables
            .get(&name)
            .ok_or_else(|| parse_error(line, "unknown visual colors"))?;
        let value = visual_table_key(
            &value_expr,
            &table.axis,
            &table.entries,
            bindings,
            maps,
            line,
        )?;
        return table
            .entries
            .get(&value)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual color value missing"));
    }
    Ok(expr.to_string())
}

#[derive(Clone, Debug)]
struct VisualSelectorTarget {
    object_name: String,
    bindings: HashMap<String, String>,
}

fn expand_visual_selector(
    selector: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Vec<VisualSelectorTarget>, DiagnosticReport> {
    let selector_base = selector.split_once(':').map_or(selector, |(base, _)| base);
    if !catalog.object_schemas.contains_key(selector_base) {
        if let Some(object) = catalog.object_names.get(selector).copied() {
            let name = catalog
                .object_labels
                .get(&object)
                .cloned()
                .unwrap_or_else(|| selector.to_string());
            return Ok(vec![VisualSelectorTarget {
                object_name: name,
                bindings: HashMap::new(),
            }]);
        }
    }
    if let Some(objects) = catalog.object_groups.get(selector) {
        return Ok(objects
            .iter()
            .filter_map(|object| catalog.object_labels.get(object).cloned())
            .map(|object_name| VisualSelectorTarget {
                object_name,
                bindings: HashMap::new(),
            })
            .collect());
    }

    let parts = selector.split(':').collect::<Vec<_>>();
    let Some(schema) = catalog.object_schemas.get(parts[0]) else {
        return Err(parse_error(line, "unknown visual object selector"));
    };
    if parts.len() - 1 > schema.axes.len() {
        return Err(parse_error(
            line,
            "visual object selector has too many tags",
        ));
    }

    let constraints = visual_selector_constraints(&parts, schema, catalog, line)?;
    let assignments = visual_selector_assignments(schema, &constraints, &catalog.maps, line)?;
    let mut targets = Vec::new();
    for (target_values, bindings) in assignments {
        let variant = schema
            .variants
            .iter()
            .find(|variant| variant.values == target_values)
            .ok_or_else(|| parse_error(line, "visual object selector target not found"))?;
        let object_name = catalog
            .object_labels
            .get(&variant.object)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual object label missing"))?;
        if targets
            .iter()
            .any(|target: &VisualSelectorTarget| target.object_name == object_name)
        {
            return Err(parse_error(
                line,
                "visual object selector maps multiple bindings to one object",
            ));
        }
        targets.push(VisualSelectorTarget {
            object_name,
            bindings,
        });
    }
    if targets.is_empty() {
        return Err(parse_error(
            line,
            "visual object selector matched no objects",
        ));
    }
    Ok(targets)
}

fn visual_selector_constraints(
    parts: &[&str],
    schema: &ObjectSchema,
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<VisualSelectorConstraint>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    schema
        .axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let Some(part) = parts.get(index + 1).copied() else {
                return Ok(VisualSelectorConstraint::Any);
            };
            let expr = parse_value_expr(part, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                return Ok(VisualSelectorConstraint::Any);
            }
            if let ValueExpr::MapCall { arg, .. } = &expr {
                if arg != axis {
                    return Err(parse_error(
                        line,
                        "map argument must match selector tag set",
                    ));
                }
                let ValueExpr::MapCall { name, .. } = &expr else {
                    unreachable!("map call branch only handles map calls");
                };
                let map = catalog
                    .maps
                    .get(name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                if map.axis != *axis {
                    return Err(parse_error(line, "map tag set must match argument tag set"));
                }
                return Ok(VisualSelectorConstraint::Mapped(expr));
            }
            let ValueExpr::Binding(name) = expr else {
                unreachable!("value expr is either binding or map call");
            };
            let axis_values = schema_axis_values(schema, index)?;
            if axis_values.contains(&name) && value_sets.contains_key(&name) {
                Err(ambiguous_selector_tag_error(&name, parts[0], axis, line))
            } else if let Some(values) = value_sets.get(&name) {
                validate_selector_subset(&name, values, &axis_values, parts[0], axis, line)?;
                Ok(VisualSelectorConstraint::ValueSet(values.clone()))
            } else if axis_values.contains(&name) {
                Ok(VisualSelectorConstraint::Fixed(name))
            } else {
                Ok(VisualSelectorConstraint::Fixed(name))
            }
        })
        .collect()
}

fn visual_selector_assignments(
    schema: &ObjectSchema,
    constraints: &[VisualSelectorConstraint],
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<Vec<(Vec<String>, HashMap<String, String>)>, DiagnosticReport> {
    let mut assignments = vec![(Vec::<String>::new(), HashMap::<String, String>::new())];
    for (index, axis) in schema.axes.iter().enumerate() {
        let axis_values = schema_axis_values(schema, index)?;
        let values = match &constraints[index] {
            VisualSelectorConstraint::Any | VisualSelectorConstraint::Mapped(_) => axis_values,
            VisualSelectorConstraint::Fixed(value) => vec![value.clone()],
            VisualSelectorConstraint::ValueSet(values) => values.clone(),
        };
        let mut next = Vec::new();
        for (target_prefix, bindings) in &assignments {
            for value in &values {
                let mut env = visual_value_env(bindings);
                env.bind(axis, axis, value);
                let target_value = match &constraints[index] {
                    VisualSelectorConstraint::Mapped(expr) => {
                        eval_bound_value_expr(expr, &env, maps, line)?
                    }
                    _ => value.clone(),
                };
                if !schema_axis_values(schema, index)?.contains(&target_value) {
                    return Err(parse_error(
                        line,
                        "visual object selector target value is not in tag slot",
                    ));
                }
                let mut target_values = target_prefix.clone();
                target_values.push(target_value);
                let mut next_bindings = bindings.clone();
                next_bindings.insert(axis.clone(), value.clone());
                next.push((target_values, next_bindings));
            }
        }
        assignments = next;
    }
    Ok(assignments)
}

#[derive(Clone, Debug)]
enum VisualSelectorConstraint {
    Any,
    Fixed(String),
    ValueSet(Vec<String>),
    Mapped(ValueExpr),
}

fn sprite_name_for_object(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "unknown".to_string()
    } else {
        sprite
    }
}

fn parse_group_directive(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    visual_objects: &[ObjectId],
    object_groups: &mut HashMap<String, Vec<ObjectId>>,
) -> Result<(), DiagnosticReport> {
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "group must be: group <name> = <selector...>",
        ));
    }

    let name = tokens[1];
    validate_selector_alias_name(name, line, "group name")?;
    if selector_name_conflicts_with(name, object_names, object_schemas, object_groups) {
        return Err(parse_error(
            line,
            "group name must not shadow another selector",
        ));
    }

    let selector_sets = selector_sets(
        &tokens[3..],
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let mut objects = Vec::new();
    for selector_set in selector_sets {
        for object in selector_set {
            if !objects.contains(&object) {
                objects.push(object);
            }
        }
    }
    if objects.is_empty() {
        return Err(parse_error(line, "group must contain at least one object"));
    }
    validate_named_selector_role(name, &objects, visual_objects, line, "group")?;

    object_groups.insert(name.to_string(), objects);
    Ok(())
}

fn parse_group_block(
    lines: &[String],
    start: usize,
    catalog: &mut Catalog,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        if tokens.len() < 3 || tokens.get(1).copied() != Some("=") {
            return Err(parse_error(
                &lines[i],
                "group row must be: <name> = <selector...>",
            ));
        }

        let mut group_tokens = vec!["group"];
        group_tokens.extend(tokens);
        parse_group_directive(
            &group_tokens,
            &lines[i],
            &catalog.object_names,
            &catalog.object_schemas,
            &catalog_value_sets(catalog),
            &catalog.maps,
            &catalog.visual_objects,
            &mut catalog.object_groups,
        )?;
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "groups missing closing brace"));
    }

    Ok(i + 1)
}

fn parse_legend_directive(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    render_chars: &mut HashMap<ObjectId, char>,
    char_objects: &mut HashMap<char, Vec<ObjectId>>,
    render_overlays: &mut OverlayDefs,
) -> Result<(), DiagnosticReport> {
    if tokens.len() < 4 || tokens.get(2).copied() != Some("=") {
        return Err(parse_error(
            line,
            "legend must be: legend <char> = <selector...>",
        ));
    }

    let ch = parse_char(tokens.get(1), line, "missing legend char")?;
    let selector_sets = selector_sets(
        &tokens[3..],
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);

    if selector_sets.len() == 1 {
        for object in &selector_sets[0] {
            render_chars.insert(*object, ch);
        }
        if selector_sets[0].len() == 1 {
            char_objects.insert(ch, vec![selector_sets[0][0]]);
        }
        return Ok(());
    }

    for objects in &combinations {
        render_overlays.push((objects.clone(), ch));
    }
    if combinations.len() == 1 {
        char_objects.insert(ch, combinations[0].clone());
    }

    Ok(())
}

fn parse_render_overlay(
    tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<(OverlayDefs, Option<Vec<ObjectId>>, char), DiagnosticReport> {
    if tokens.len() < 4 {
        return Err(parse_error(
            line,
            "render_overlay must be: render_overlay <object> <object> [object...] <char>",
        ));
    }

    let ch = parse_char(tokens.last(), line, "missing overlay char")?;
    let selector_sets = selector_sets(
        &tokens[1..tokens.len() - 1],
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    let combinations = cartesian_object_product(&selector_sets);
    let overlays = combinations
        .iter()
        .map(|objects| (objects.clone(), ch))
        .collect::<Vec<_>>();
    let level_objects = (combinations.len() == 1).then(|| combinations[0].clone());

    Ok((overlays, level_objects, ch))
}

fn selector_sets(
    selectors: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<Vec<Vec<ObjectId>>, DiagnosticReport> {
    selectors
        .iter()
        .map(|selector| {
            resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )
            .map(|selector| selector.alternatives)
        })
        .collect()
}

fn cartesian_object_product(sets: &[Vec<ObjectId>]) -> Vec<Vec<ObjectId>> {
    let mut combinations = vec![Vec::<ObjectId>::new()];
    for set in sets {
        let mut next = Vec::new();
        for prefix in &combinations {
            for object in set {
                let mut combination = prefix.clone();
                combination.push(*object);
                next.push(combination);
            }
        }
        combinations = next;
    }
    combinations
}

fn parse_direction_directive(
    tokens: &[&str],
    line: &str,
    catalog: &mut Catalog,
) -> Result<Option<Direction>, DiagnosticReport> {
    match tokens {
        ["direction", alias, canonical] => {
            add_direction_alias(alias, canonical, line, catalog)?;
            Ok(None)
        }
        _ => Err(parse_error(
            line,
            "direction must be: direction <alias> <up|down|left|right>",
        )),
    }
}

fn add_direction_alias(
    alias: &str,
    canonical: &str,
    line: &str,
    catalog: &mut Catalog,
) -> Result<(), DiagnosticReport> {
    validate_identifier(alias, line, "direction alias")?;
    named_direction_vector(canonical, line)?;
    let canonical_input = catalog
        .input_names
        .get(canonical)
        .copied()
        .map(Ok)
        .unwrap_or_else(|| add_input_name(canonical, line, catalog))?;
    if let Some(existing) = catalog.input_names.get(alias).copied() {
        if existing != canonical_input {
            return Err(parse_error(
                line,
                "direction alias must not redefine an existing input",
            ));
        }
        return Ok(());
    }
    catalog
        .input_names
        .insert(alias.to_string(), canonical_input);
    Ok(())
}

fn parse_global_directive(
    tokens: &[&str],
    line: &str,
    global_names: &mut HashMap<String, GlobalId>,
    global_labels: &mut HashMap<GlobalId, String>,
    global_defaults: &mut Vec<i64>,
    numeric_global_defaults: &mut HashMap<String, i64>,
    persistent_vars: &mut Vec<GlobalId>,
    constant_globals: &mut Vec<GlobalId>,
) -> Result<(), DiagnosticReport> {
    let parsed = match tokens {
        ["var", name, "=", value] => Some((*name, *value, false, false)),
        ["const", name, "=", value] => Some((*name, *value, false, true)),
        ["persistent", "var", name, "=", value] => Some((*name, *value, true, false)),
        ["persistent", "const", name, "=", value] => Some((*name, *value, true, true)),
        _ => None,
    };
    match parsed {
        Some((name, value, persistent, constant)) => {
            if !is_identifier(name) {
                return Err(parse_error(line, "var or const name must be an identifier"));
            }
            if global_names.contains_key(name) {
                return Err(parse_error(line, "duplicate var or const"));
            }
            let id = GlobalId(global_defaults.len() as u16);
            let default = parse_global_value(value, line)?;
            global_names.insert(name.to_string(), id);
            global_labels.insert(id, name.to_string());
            global_defaults.push(default);
            if value.parse::<i64>().is_ok() {
                numeric_global_defaults.insert(name.to_string(), default);
            }
            if persistent {
                persistent_vars.push(id);
            }
            if constant {
                constant_globals.push(id);
            }
            Ok(())
        }
        _ => Err(parse_error(
            line,
            "var or const must be: var <name> = <true | false | number> or const <name> = <true | false | number>",
        )),
    }
}

fn parse_condition_directive(
    _tokens: &[&str],
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    condition_names: &mut HashMap<String, ConditionId>,
    condition_labels: &mut HashMap<ConditionId, String>,
) -> Result<ConditionDefinitionAst, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("condition ") else {
        return Err(parse_error(
            line,
            "condition must be: condition <name> = <condition_expr>",
        ));
    };
    let Some((name, expr)) = rest.split_once('=') else {
        return Err(parse_error(
            line,
            "condition must be: condition <name> = <condition_expr>",
        ));
    };
    let name = name.trim();
    validate_qualified_identifier(name, line, "condition name")?;
    if condition_names.contains_key(name) {
        return Err(parse_error(line, "duplicate condition"));
    }
    let id = ConditionId(condition_names.len() as u16);
    let kind = parse_condition_value_expr(
        expr.trim(),
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    condition_names.insert(name.to_string(), id);
    condition_labels.insert(id, name.to_string());
    Ok(ConditionDefinitionAst { id, kind })
}

fn parse_condition_value_expr(
    expr: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionValueAst, DiagnosticReport> {
    let (name, arg) = parse_call_expr(expr, line)?;
    let pattern_arg = parse_condition_pattern_arg(
        arg,
        line,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )?;
    match name {
        "count" if pattern_arg.is_some() => Ok(ConditionValueAst::CountMatches(
            pattern_arg.expect("checked"),
        )),
        "count" => Ok(ConditionValueAst::CountObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "exists" | "some" if pattern_arg.is_some() => Ok(ConditionValueAst::ExistsMatches(
            pattern_arg.expect("checked"),
        )),
        "none" if pattern_arg.is_some() => Ok(ConditionValueAst::NoneMatches(
            pattern_arg.expect("checked"),
        )),
        "exists" => Ok(ConditionValueAst::ExistsObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "some" => Ok(ConditionValueAst::ExistsObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        "none" => Ok(ConditionValueAst::NoneObjects(
            resolve_object_selector(
                arg,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                &HashMap::new(),
            )?
            .alternatives,
        )),
        _ => Err(parse_error(line, "unknown condition function")),
    }
}

fn parse_condition_pattern_arg(
    arg: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<Option<ConditionPatternAst>, DiagnosticReport> {
    let Some((orientation, pattern)) = split_oriented_pattern_arg(arg, line)? else {
        return Ok(None);
    };
    Ok(Some(ConditionPatternAst {
        orientation,
        pattern: parse_pattern_side(
            &pattern,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
            false,
        )?,
    }))
}

fn split_oriented_pattern_arg(
    arg: &str,
    line: &str,
) -> Result<Option<(OrientationExpr, String)>, DiagnosticReport> {
    let trimmed = arg.trim();
    if trimmed.starts_with('[') {
        let (embedded_orientation, pattern) = normalize_embedded_direction_marker(trimmed);
        return Ok(Some((
            embedded_orientation.unwrap_or(OrientationExpr::Neutral),
            pattern,
        )));
    }

    let Some(open_index) = trimmed.find('[') else {
        return Ok(None);
    };
    let orientation = trimmed[..open_index].trim();
    let pattern = trimmed[open_index..].trim();
    if orientation.is_empty() {
        return Ok(Some((OrientationExpr::Neutral, pattern.to_string())));
    }
    let orientation = if let Some(axis) = orientation.strip_prefix("input ").map(str::trim) {
        if !is_identifier(axis) {
            return Err(parse_error(
                line,
                "input orientation set must be a single identifier",
            ));
        }
        OrientationExpr::InputSet(axis.to_string())
    } else if orientation == "input" {
        OrientationExpr::InputSet("directions".to_string())
    } else if !is_identifier(orientation) {
        return Err(parse_error(
            line,
            "pattern orientation must be a single identifier or input <set>",
        ));
    } else {
        parse_statement_orientation_expr(orientation, &[])
    };
    let (embedded_orientation, pattern) = normalize_embedded_direction_marker(pattern);
    if embedded_orientation.is_some() {
        return Err(parse_error(
            line,
            "pattern cannot combine orientation prefix and embedded direction marker",
        ));
    }
    Ok(Some((orientation, pattern)))
}

fn parse_call_expr<'a>(expr: &'a str, line: &str) -> Result<(&'a str, &'a str), DiagnosticReport> {
    let Some((name, rest)) = expr.split_once('(') else {
        return Err(parse_error(
            line,
            "condition expression must be a function call",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(
            line,
            "condition function name must be an identifier",
        ));
    }
    let Some(arg) = rest.strip_suffix(')') else {
        return Err(parse_error(line, "condition expression missing closing )"));
    };
    Ok((name, arg.trim()))
}

fn default_cardinal_directions(input_names: &HashMap<String, InputId>) -> Vec<Direction> {
    let Some(up) = input_names.get("up").copied() else {
        return Vec::new();
    };
    let Some(down) = input_names.get("down").copied() else {
        return Vec::new();
    };
    let Some(left) = input_names.get("left").copied() else {
        return Vec::new();
    };
    let Some(right) = input_names.get("right").copied() else {
        return Vec::new();
    };

    vec![
        Direction {
            input: up,
            dx: 0,
            dy: -1,
        },
        Direction {
            input: down,
            dx: 0,
            dy: 1,
        },
        Direction {
            input: left,
            dx: -1,
            dy: 0,
        },
        Direction {
            input: right,
            dx: 1,
            dy: 0,
        },
    ]
}

fn parse_rule_definition(
    lines: &[String],
    start: usize,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
) -> Result<(RuleDefinitionAst, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let declaration = header.first().copied().unwrap_or("routine");
    let role = if header.get(1).copied() == Some("display")
        || header
            .get(1)
            .is_some_and(|name| is_display_role_token(name))
    {
        RuleRole::Visual
    } else {
        RuleRole::Main
    };
    let name_index = if header.get(1).copied() == Some("display") {
        2
    } else {
        1
    };
    let name_spec = expect(
        header.get(name_index),
        &lines[start],
        "missing routine name",
    )?;
    let (name, params) = parse_rule_name_and_params(name_spec, &lines[start])?;
    let application = parse_rule_application(&header, declaration, role, &lines[start])?;

    let (statements, next_i) = parse_statement_block(
        lines,
        start + 1,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        input_names,
        global_names,
        numeric_globals,
        condition_names,
        &HashMap::new(),
        &params,
    )?;

    Ok((
        RuleDefinitionAst {
            name,
            role,
            application,
            statements,
        },
        next_i,
    ))
}

#[allow(clippy::too_many_arguments)]
fn add_standard_move_rule_if_missing(
    definitions: &mut Vec<RuleDefinitionAst>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    object_layers: &HashMap<ObjectId, LayerId>,
    visual_objects: &[ObjectId],
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
) -> Result<(), DiagnosticReport> {
    if definitions
        .iter()
        .any(|definition| definition.name == "move")
    {
        return Ok(());
    }
    let mut move_layer_groups = object_layers
        .iter()
        .filter_map(|(object, layer)| {
            (!visual_objects.contains(object)).then_some((*layer, *object))
        })
        .collect::<Vec<_>>();
    move_layer_groups.sort_by_key(|(layer, object)| (layer.0, object.0));

    let mut generated_groups = object_groups.clone();
    let mut generated_layer_names = Vec::new();
    let mut i = 0;
    while i < move_layer_groups.len() {
        let layer = move_layer_groups[i].0;
        let group_name = format!("__move_layer_{}", layer.0);
        let mut objects = Vec::new();
        while i < move_layer_groups.len() && move_layer_groups[i].0 == layer {
            objects.push(move_layer_groups[i].1);
            i += 1;
        }
        generated_groups.insert(group_name.clone(), objects);
        generated_layer_names.push(group_name);
    }

    if generated_layer_names.is_empty() {
        return Ok(());
    }
    let mut generated_value_sets = value_sets.clone();
    generated_value_sets.insert("__move_layers".to_string(), generated_layer_names);

    let lines = vec![
        "for l in __move_layers {".to_string(),
        "for d in directions {".to_string(),
        "once_all d [ d l | | < l ] -> [ l | {__move_collision} | l ]".to_string(),
        "once_all d [ d l | ; | ^ l ] -> [ l | {__move_collision} ; | l ]".to_string(),
        "once_all d [ | v l ; d l | ] -> [ | l ; l | {__move_collision} ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "for d in directions {".to_string(),
        "d [ d l | no l no {__move_collision} ] -> [ | l{no directions} ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "for d in directions {".to_string(),
        "once_all d [ d l ] -> [ l ]".to_string(),
        BLOCK_CLOSE.to_string(),
        "once_all [ {__move_collision} ] -> [ ]".to_string(),
        BLOCK_CLOSE.to_string(),
        BLOCK_CLOSE.to_string(),
    ];
    let (statements, next_i) = parse_statement_block(
        &lines,
        0,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        &generated_value_sets,
        maps,
        &generated_groups,
        input_names,
        global_names,
        &HashMap::new(),
        condition_names,
        &HashMap::new(),
        &[],
    )?;
    if next_i != lines.len() {
        return Err(DiagnosticReport::error(
            "standard move rule expansion failed".to_string(),
        ));
    }

    definitions.push(RuleDefinitionAst {
        name: "move".to_string(),
        role: RuleRole::Main,
        application: RuleApplication::UntilStable,
        statements,
    });
    Ok(())
}

fn parse_rule_name_and_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<String>), DiagnosticReport> {
    let Some((name, params)) = value.split_once('(') else {
        validate_rule_name(value, line)?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_rule_name(name, line)?;
    let params = params
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "routine params must end with )"))?;
    let params = if params.trim().is_empty() {
        Vec::new()
    } else {
        params
            .split(',')
            .map(str::trim)
            .map(|param| {
                validate_identifier(param, line, "routine param")?;
                Ok(param.to_string())
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), params))
}

fn parse_lifecycle_block(
    lines: &[String],
    start: usize,
    event: &str,
    catalog: &Catalog,
) -> Result<(String, Vec<StatementAst>, usize), DiagnosticReport> {
    let (statements, next_i) = parse_statement_block(
        lines,
        start + 1,
        &[BLOCK_CLOSE],
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog_value_sets(catalog),
        &catalog.maps,
        &catalog.object_groups,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.numeric_global_defaults,
        &catalog.condition_names,
        &HashMap::new(),
        &[],
    )?;
    Ok((event.to_string(), statements, next_i))
}

fn parse_rule_application(
    tokens: &[&str],
    declaration: &str,
    role: RuleRole,
    line: &str,
) -> Result<RuleApplication, DiagnosticReport> {
    match (role, tokens) {
        (RuleRole::Main, [kind, _]) if *kind == declaration => Ok(RuleApplication::Once),
        (RuleRole::Visual, [kind, "display", _]) if *kind == declaration => {
            Ok(RuleApplication::Once)
        }
        (RuleRole::Visual, [kind, name]) if *kind == declaration && is_display_role_token(name) => {
            Ok(RuleApplication::Once)
        }
        (RuleRole::Main, [kind, _, application]) if *kind == declaration => {
            parse_application_keyword(application, line)
        }
        (RuleRole::Visual, [kind, "display", _, application]) if *kind == declaration => {
            parse_application_keyword(application, line)
        }
        (RuleRole::Visual, [kind, name, application])
            if *kind == declaration && is_display_role_token(name) =>
        {
            parse_application_keyword(application, line)
        }
        _ => Err(parse_error(
            line,
            "routine header must be: routine [display] <name> [once | once_all | once_per_level | repeat]",
        )),
    }
}

fn parse_application_keyword(token: &str, line: &str) -> Result<RuleApplication, DiagnosticReport> {
    match token {
        "once" => Ok(RuleApplication::Once),
        "once_all" => Ok(RuleApplication::OnceAll),
        "once_per_level" => Ok(RuleApplication::OncePerLevel),
        "repeat" => Ok(RuleApplication::UntilStable),
        _ => Err(parse_error(
            line,
            "application must be one of: once, once_all, once_per_level, repeat",
        )),
    }
}

fn parse_fix_defaults(
    tokens: &[&str],
    line: &str,
    rule_params: &[String],
) -> Result<FixDefaults, DiagnosticReport> {
    if tokens.len() < 2 {
        return Err(parse_error(
            line,
            "fix block must be: fix <once | repeat | orientation...>",
        ));
    }

    let mut defaults = FixDefaults::default();
    for token in &tokens[1..] {
        match *token {
            "once" | "once_all" | "once_per_level" | "repeat" => {
                let application = parse_application_keyword(token, line)?;
                if defaults.application.replace(application).is_some() {
                    return Err(parse_error(line, "fix can specify application only once"));
                }
            }
            orientation => {
                if defaults
                    .orientation
                    .replace(parse_statement_orientation_expr(orientation, rule_params))
                    .is_some()
                {
                    return Err(parse_error(line, "fix can specify orientation only once"));
                }
            }
        }
    }

    Ok(defaults)
}

fn collect_statement_block_lines(
    lines: &[String],
    start: usize,
    line: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut body = Vec::new();
    let mut depth = 1i32;
    let mut i = start;
    while i < lines.len() {
        let nested_line = &lines[i];
        let delta = statement_block_line_delta(nested_line);
        let next_depth = depth + delta;
        if next_depth == 0 {
            return Ok((body, i + 1));
        }
        if next_depth < 0 {
            return Err(parse_error(
                line,
                "for block has an unmatched closing brace",
            ));
        }
        body.push(nested_line.clone());
        depth = next_depth;
        i += 1;
    }
    Err(parse_error(line, "for block missing closing brace"))
}

fn statement_block_line_delta(line: &str) -> i32 {
    raw_brace_delta(strip_line_comment(line))
}

fn parse_if_condition_block_header(
    line: &str,
) -> Result<Option<ConditionBlockCombinator>, DiagnosticReport> {
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        ["if"] => Ok(Some(ConditionBlockCombinator::All)),
        ["if", "all"] => Ok(Some(ConditionBlockCombinator::All)),
        ["if", "any"] => Ok(Some(ConditionBlockCombinator::Any)),
        ["if", ..] if line.trim_end().ends_with('{') => Err(parse_error(
            line,
            "if condition block must be: if [all | any] {",
        )),
        _ => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_condition_block(
    lines: &[String],
    start: usize,
    combinator: ConditionBlockCombinator,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<(ConditionAst, usize), DiagnosticReport> {
    let mut conditions = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let condition = parse_statement_condition(
            &lines[i],
            &lines[i],
            input_names,
            global_names,
            condition_names,
            named_conditions,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )?;
        conditions.push(condition);
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "if condition block missing closing brace",
        ));
    }
    if conditions.is_empty() {
        return Err(parse_error(
            &lines[start],
            "if condition block requires at least one condition",
        ));
    }
    let condition = if conditions.len() == 1 {
        conditions.remove(0)
    } else {
        combinator.combine(conditions)
    };
    Ok((condition, i + 1))
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_arrow_consequence(
    lines: &[String],
    start: usize,
    header_line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let Some(line) = lines.get(start) else {
        return Err(parse_error(
            header_line,
            "if condition block must be followed by ->",
        ));
    };
    let header = block_header_text(line);
    let Some((_, effect_text)) = header.split_once("->") else {
        return Err(parse_error(
            line,
            "if condition block must be followed by ->",
        ));
    };
    let effect_text = effect_text.trim();

    if line.trim_end().ends_with('{') {
        if !effect_text.is_empty() {
            return Err(parse_error(line, "if -> block header must be: -> {"));
        }
        return parse_statement_block(
            lines,
            start + 1,
            &["else", BLOCK_CLOSE],
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            input_names,
            global_names,
            numeric_globals,
            condition_names,
            named_conditions,
            rule_params,
        );
    }

    if effect_text.is_empty() {
        return Err(parse_error(
            line,
            "if -> must be followed by an effect or block",
        ));
    }
    if is_qualified_identifier(effect_text) && !is_builtin_rewrite_effect_text(effect_text) {
        return Ok((
            vec![StatementAst::Call {
                name: effect_text.to_string(),
                source_line: line.to_string(),
            }],
            start + 1,
        ));
    }
    let effects = parse_rewrite_effect(effect_text, line)?;
    Ok((vec![StatementAst::Effect { effects }], start + 1))
}

#[allow(clippy::too_many_arguments)]
fn parse_optional_else_statement_block(
    lines: &[String],
    next_i: usize,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let Some(else_start) = else_block_start(lines, next_i) else {
        return Ok((Vec::new(), next_i));
    };
    parse_statement_block(
        lines,
        else_start,
        &[BLOCK_CLOSE],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        input_names,
        global_names,
        numeric_globals,
        condition_names,
        named_conditions,
        rule_params,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueExpr {
    Binding(String),
    MapCall { name: String, arg: String },
}

#[derive(Clone, Debug, Default)]
struct ValueEnv {
    values: HashMap<String, String>,
    axes: HashMap<String, String>,
}

impl ValueEnv {
    fn bind(&mut self, name: &str, axis: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
        self.axes.insert(name.to_string(), axis.to_string());
    }

    fn bind_untyped(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }
}

fn visual_value_env(bindings: &HashMap<String, String>) -> ValueEnv {
    let mut env = ValueEnv::default();
    for (axis, value) in bindings {
        env.bind(axis, axis, value);
    }
    env
}

fn parse_value_expr(value: &str, line: &str) -> Result<ValueExpr, DiagnosticReport> {
    if let Some((name, arg)) = parse_map_call(value) {
        validate_identifier(name, line, "map name")?;
        validate_identifier(arg, line, "map argument")?;
        return Ok(ValueExpr::MapCall {
            name: name.to_string(),
            arg: arg.to_string(),
        });
    }
    if !is_value_atom(value) {
        return Err(parse_error(
            line,
            "value expression must be an identifier-like atom",
        ));
    }
    Ok(ValueExpr::Binding(value.to_string()))
}

fn eval_bound_value_expr(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    eval_value_expr(expr, env, maps, line, false)
}

fn eval_value_expr(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
    allow_literal: bool,
) -> Result<String, DiagnosticReport> {
    match expr {
        ValueExpr::Binding(name) => {
            if let Some(value) = env.values.get(name) {
                Ok(value.clone())
            } else if allow_literal {
                Ok(name.clone())
            } else {
                Err(parse_error(
                    line,
                    "value expression binding is not in scope",
                ))
            }
        }
        ValueExpr::MapCall { name, arg } => {
            let map = maps
                .get(name)
                .ok_or_else(|| parse_error(line, "unknown map"))?;
            let value = env
                .values
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument binding is not in scope"))?;
            let axis = env
                .axes
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument tag set is not known"))?;
            if map.axis != *axis {
                return Err(parse_error(line, "map tag set must match argument tag set"));
            }
            map.values
                .get(value)
                .cloned()
                .ok_or_else(|| parse_error(line, "map missing input value"))
        }
    }
}

fn value_expr_result_axis(
    expr: &ValueExpr,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    match expr {
        ValueExpr::Binding(name) => env
            .axes
            .get(name)
            .cloned()
            .ok_or_else(|| parse_error(line, "value expression binding tag set is not known")),
        ValueExpr::MapCall { name, arg } => {
            let map = maps
                .get(name)
                .ok_or_else(|| parse_error(line, "unknown map"))?;
            let axis = env
                .axes
                .get(arg)
                .ok_or_else(|| parse_error(line, "map argument tag set is not known"))?;
            if map.axis != *axis {
                return Err(parse_error(line, "map tag set must match argument tag set"));
            }
            Ok(map.axis.clone())
        }
    }
}

fn expand_for_binding_lines(
    lines: &[String],
    binding: &str,
    axis: Option<&str>,
    value: &str,
    maps: &HashMap<String, ValueMap>,
) -> Result<Vec<String>, DiagnosticReport> {
    lines
        .iter()
        .map(|line| expand_for_binding_line(line, binding, axis, value, maps))
        .collect()
}

fn expand_for_binding_line(
    line: &str,
    binding: &str,
    axis: Option<&str>,
    value: &str,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut env = ValueEnv::default();
    if let Some(axis) = axis {
        env.bind(binding, axis, value);
    } else {
        env.bind_untyped(binding, value);
    }
    let expanded = replace_map_call_tokens(line, &env, maps)?;
    Ok(replace_identifier_token(&expanded, binding, value))
}

fn replace_map_call_tokens(
    line: &str,
    env: &ValueEnv,
    maps: &HashMap<String, ValueMap>,
) -> Result<String, DiagnosticReport> {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        if is_identifier_start(chars[i]) {
            let name_start = i;
            i += 1;
            while i < chars.len() && is_identifier_continue(chars[i]) {
                i += 1;
            }
            if i < chars.len() && chars[i] == '(' {
                let arg_start = i + 1;
                let mut arg_end = arg_start;
                while arg_end < chars.len() && is_identifier_continue(chars[arg_end]) {
                    arg_end += 1;
                }
                if arg_end > arg_start && arg_end < chars.len() && chars[arg_end] == ')' {
                    let name = chars[name_start..i].iter().collect::<String>();
                    let arg = chars[arg_start..arg_end].iter().collect::<String>();
                    if maps.contains_key(&name) {
                        let expr = ValueExpr::MapCall { name, arg };
                        out.push_str(&eval_bound_value_expr(&expr, env, maps, line)?);
                        i = arg_end + 1;
                        continue;
                    }
                }
            }
            out.extend(chars[name_start..i].iter());
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn replace_identifier_token(line: &str, binding: &str, value: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token = String::new();
    for ch in line.chars() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            token.push(ch);
            continue;
        }
        flush_identifier_token(&mut out, &mut token, binding, value);
        out.push(ch);
    }
    flush_identifier_token(&mut out, &mut token, binding, value);
    out
}

fn flush_identifier_token(out: &mut String, token: &mut String, binding: &str, value: &str) {
    if token.is_empty() {
        return;
    }
    if token == binding {
        out.push_str(value);
    } else {
        out.push_str(token);
    }
    token.clear();
}

#[derive(Clone, Debug)]
struct ForExpansionValue {
    value: String,
    axis: Option<String>,
}

fn for_expansion_values(
    sources: &[&str],
    value_sets: &HashMap<String, Vec<String>>,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<ForExpansionValue>, DiagnosticReport> {
    if sources.is_empty() {
        return Err(parse_error(
            line,
            "for directive must be: for <binding> in <source...>",
        ));
    }
    if sources.len() == 1 {
        let source = sources[0];
        if let Some(values) = value_sets.get(source) {
            return Ok(values
                .iter()
                .map(|value| ForExpansionValue {
                    value: value.clone(),
                    axis: Some(source.to_string()),
                })
                .collect());
        }
        if let Some(values) = numeric_range_values(source, numeric_globals, line)? {
            return Ok(values
                .into_iter()
                .map(|value| ForExpansionValue { value, axis: None })
                .collect());
        }
        return Err(parse_error(
            line,
            "unknown expansion tag set or numeric range",
        ));
    }

    sources
        .iter()
        .flat_map(|source| {
            if let Some(values) = value_sets.get(*source) {
                return values
                    .iter()
                    .map(|value| {
                        Ok(ForExpansionValue {
                            value: value.clone(),
                            axis: Some((*source).to_string()),
                        })
                    })
                    .collect::<Vec<_>>();
            }
            match numeric_range_values(source, numeric_globals, line) {
                Ok(Some(values)) => values
                    .into_iter()
                    .map(|value| Ok(ForExpansionValue { value, axis: None }))
                    .collect(),
                Ok(None) => vec![Ok(ForExpansionValue {
                    value: (*source).to_string(),
                    axis: None,
                })],
                Err(error) => vec![Err(error)],
            }
        })
        .collect()
}

fn expand_numeric_ranges_in_value_list(
    values: &[&str],
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut expanded = Vec::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(parse_error(line, "tag value must not be empty"));
        }
        if let Some(range_values) = numeric_range_values(value, numeric_globals, line)? {
            expanded.extend(range_values);
        } else {
            expanded.push((*value).to_string());
        }
    }
    Ok(expanded)
}

fn numeric_range_values(
    source: &str,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<Option<Vec<String>>, DiagnosticReport> {
    let Some((start, end)) = source.split_once("...") else {
        return Ok(None);
    };
    if start.is_empty() || end.is_empty() || end.contains("...") {
        return Err(parse_error(
            line,
            "numeric range must be: <integer>...<integer>",
        ));
    }
    let start = parse_numeric_range_endpoint(start, numeric_globals, line)?;
    let end = parse_numeric_range_endpoint(end, numeric_globals, line)?;
    if start > end {
        return Err(parse_error(
            line,
            "numeric range start must be less than or equal to end",
        ));
    }
    Ok(Some((start..=end).map(|value| value.to_string()).collect()))
}

fn parse_numeric_range_endpoint(
    value: &str,
    numeric_globals: &HashMap<String, i64>,
    line: &str,
) -> Result<i64, DiagnosticReport> {
    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(parsed);
    }
    numeric_globals.get(value).copied().ok_or_else(|| {
        parse_error(
            line,
            "numeric range endpoints must be integer literals or integer vars",
        )
    })
}

fn collect_multiline_rewrite_statement(
    lines: &[String],
    start: usize,
) -> Result<Option<(String, usize)>, DiagnosticReport> {
    let line = lines[start].trim();
    if let Some(collected) = collect_bracket_multiline_rewrite_statement(lines, start, line)? {
        return Ok(Some(collected));
    }

    let Some(trailing) = rewrite_lhs_trailing(line) else {
        return Ok(None);
    };

    if trailing.is_empty() {
        let Some(next_line) = lines.get(start + 1).map(|line| line.trim()) else {
            return Ok(None);
        };
        let Some(rhs) = next_line.strip_prefix("->").map(str::trim_start) else {
            return Ok(None);
        };
        validate_rewrite_rhs_continuation(rhs, next_line)?;
        return Ok(Some((format!("{line} -> {rhs}"), start + 2)));
    }

    if trailing == "->" {
        let Some(rhs) = lines.get(start + 1).map(|line| line.trim()) else {
            return Ok(None);
        };
        validate_rewrite_rhs_continuation(rhs, line)?;
        return Ok(Some((format!("{line} {rhs}"), start + 2)));
    }

    Ok(None)
}

fn collect_bracket_multiline_rewrite_statement(
    lines: &[String],
    start: usize,
    first_line: &str,
) -> Result<Option<(String, usize)>, DiagnosticReport> {
    let Some(open_index) = first_line.find('[') else {
        return Ok(None);
    };
    let prefix = first_line[..open_index].trim();
    if !can_start_rewrite_lhs(prefix) {
        return Ok(None);
    }

    let mut joined = String::new();
    let mut bracket_depth = 0usize;
    let mut saw_arrow = false;
    let mut i = start;
    while i < lines.len() {
        let line = lines[i].trim();
        if i > start && bracket_depth == 0 && !saw_arrow && !line.starts_with("->") {
            return Ok(None);
        }
        if !joined.is_empty() {
            if bracket_depth > 0 {
                joined.push_str("; ");
            } else {
                joined.push(' ');
            }
        }
        joined.push_str(line);
        bracket_depth = update_square_bracket_depth(bracket_depth, line);
        saw_arrow |= line.contains("->");

        if i == start && bracket_depth == 0 {
            return Ok(None);
        }
        if i > start && bracket_depth == 0 && saw_arrow {
            validate_rewrite_rhs_continuation_after_join(&joined)?;
            return Ok(Some((joined, i + 1)));
        }
        i += 1;
    }

    Ok(None)
}

fn update_square_bracket_depth(mut depth: usize, line: &str) -> usize {
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn validate_rewrite_rhs_continuation_after_join(line: &str) -> Result<(), DiagnosticReport> {
    let Some((_, rhs)) = line.split_once("->") else {
        return Ok(());
    };
    validate_rewrite_rhs_continuation(rhs.trim_start(), line)
}

fn validate_rewrite_rhs_continuation(rhs: &str, line: &str) -> Result<(), DiagnosticReport> {
    if rhs.is_empty() || !rhs.starts_with('[') {
        return Err(parse_error(
            line,
            "rewrite continuation after -> must start with a pattern",
        ));
    }
    if rhs.contains("->") {
        return Err(parse_error(
            line,
            "rewrite continuation rhs cannot contain another ->",
        ));
    }
    Ok(())
}

fn rewrite_lhs_trailing(line: &str) -> Option<&str> {
    let open_index = line.find('[')?;
    let prefix = line[..open_index].trim();
    if !can_start_rewrite_lhs(prefix) {
        return None;
    }
    let lhs_end = open_index + pattern_side_syntax_end(&line[open_index..])?;
    Some(line[lhs_end..].trim())
}

fn can_start_rewrite_lhs(prefix: &str) -> bool {
    let tokens = split_header_tokens(prefix);
    match tokens.as_slice() {
        [] => true,
        ["input", axis] => is_identifier(axis),
        [application] if is_rewrite_application_prefix(application) => true,
        [application, "input", axis] if is_rewrite_application_prefix(application) => {
            is_identifier(axis)
        }
        [application, orientation]
            if is_rewrite_application_prefix(application) && is_identifier(orientation) =>
        {
            true
        }
        [orientation] if !is_non_rewrite_statement_prefix(orientation) => {
            is_identifier(orientation)
        }
        _ => false,
    }
}

fn is_rewrite_application_prefix(token: &str) -> bool {
    puzzle_authoring::rule_application_surface(token).is_some()
}

fn is_non_rewrite_statement_prefix(token: &str) -> bool {
    matches!(
        token,
        "for" | "fix" | "if" | "else" | "when" | "action" | "emit" | "do" | "display"
    )
}

fn pattern_side_syntax_end(value: &str) -> Option<usize> {
    let mut index = 0;
    let mut found_block = false;
    while index < value.len() {
        let after_space = value[index..].trim_start();
        index = value.len() - after_space.len();
        if !value[index..].starts_with('[') {
            break;
        }
        let after_open = index + 1;
        let close_offset = value[after_open..].find(']')?;
        index = after_open + close_offset + 1;
        found_block = true;
    }
    found_block.then_some(index)
}

fn else_block_start(lines: &[String], next_i: usize) -> Option<usize> {
    if next_i > 0 && is_else_block_marker(&lines[next_i - 1]) {
        Some(next_i)
    } else if next_i < lines.len() && is_else_block_marker(&lines[next_i]) {
        Some(next_i + 1)
    } else {
        None
    }
}

fn is_else_block_marker(line: &str) -> bool {
    line == "else" || line == "else {"
}

fn statement_block_terminator_matches(line: &str, terminators: &[&str]) -> bool {
    terminators.contains(&line) || (terminators.contains(&"else") && is_else_block_marker(line))
}

fn parse_statement_block(
    lines: &[String],
    start: usize,
    terminators: &[&str],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    numeric_globals: &HashMap<String, i64>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    rule_params: &[String],
) -> Result<(Vec<StatementAst>, usize), DiagnosticReport> {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let source_line = &lines[i];
        if statement_block_terminator_matches(source_line, terminators) {
            return if diagnostics.is_empty() {
                Ok((statements, i + 1))
            } else {
                Err(DiagnosticReport::from_diagnostics(diagnostics))
            };
        }

        let mut next_statement_i = i + 1;
        let joined_line;
        let line = if let Some((joined, next_i)) = collect_multiline_rewrite_statement(lines, i)? {
            next_statement_i = next_i;
            joined_line = joined;
            joined_line.as_str()
        } else {
            source_line.as_str()
        };
        let opens_block = line.trim_end().ends_with('{');
        let line = block_header_text(line);
        let tokens = split_header_tokens(line);
        match tokens.first().copied() {
            Some("for") => {
                if !opens_block {
                    return Err(parse_error(line, "for block must use `{ ... }`"));
                }
                let ["for", binding, "in", sources @ ..] = tokens.as_slice() else {
                    return Err(parse_error(
                        line,
                        "for directive must be: for <binding> in <source...>",
                    ));
                };
                let values = for_expansion_values(sources, value_sets, numeric_globals, line)?;
                validate_identifier(binding, line, "expansion binding")?;
                let (body_lines, next_i) = collect_statement_block_lines(lines, i + 1, line)?;
                for value in &values {
                    let mut expanded_lines = expand_for_binding_lines(
                        &body_lines,
                        binding,
                        value.axis.as_deref(),
                        &value.value,
                        maps,
                    )?;
                    expanded_lines.push(BLOCK_CLOSE.to_string());
                    let (nested, parsed_i) = parse_statement_block(
                        &expanded_lines,
                        0,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    if parsed_i != expanded_lines.len() {
                        return Err(parse_error(line, "for expansion failed"));
                    }
                    statements.extend(nested);
                }
                i = next_i;
                continue;
            }
            Some("fix") => {
                let defaults = parse_fix_defaults(&tokens, line, rule_params)?;
                let (nested, next_i) = parse_statement_block(
                    lines,
                    i + 1,
                    &[BLOCK_CLOSE],
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    input_names,
                    global_names,
                    numeric_globals,
                    condition_names,
                    named_conditions,
                    rule_params,
                )?;
                statements.push(StatementAst::Fix {
                    defaults,
                    statements: nested,
                });
                i = next_i;
            }
            Some("if") => {
                if let Some(combinator) = parse_if_condition_block_header(line)? {
                    let (condition, arrow_i) = parse_statement_condition_block(
                        lines,
                        i,
                        combinator,
                        input_names,
                        global_names,
                        condition_names,
                        named_conditions,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?;
                    let (then_statements, next_i) = parse_statement_arrow_consequence(
                        lines,
                        arrow_i,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    let (else_statements, next_i) = parse_optional_else_statement_block(
                        lines,
                        next_i,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::If {
                        condition,
                        then_statements,
                        else_statements,
                    });
                    i = next_i;
                    continue;
                }
                if let Some((condition, trailing)) = parse_pattern_if_header(
                    line,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                )? {
                    if trailing.is_empty() {
                        let (nested, next_i) = parse_statement_block(
                            lines,
                            i + 1,
                            &["else", BLOCK_CLOSE],
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            input_names,
                            global_names,
                            numeric_globals,
                            condition_names,
                            named_conditions,
                            rule_params,
                        )?;
                        let (then_statements, else_statements, after_i) =
                            if let Some(else_start) = else_block_start(lines, next_i) {
                                let (else_statements, after_else_i) = parse_statement_block(
                                    lines,
                                    else_start,
                                    &[BLOCK_CLOSE],
                                    object_names,
                                    object_schemas,
                                    value_sets,
                                    maps,
                                    object_groups,
                                    input_names,
                                    global_names,
                                    numeric_globals,
                                    condition_names,
                                    named_conditions,
                                    rule_params,
                                )?;
                                (nested, else_statements, after_else_i)
                            } else {
                                (nested, Vec::new(), next_i)
                            };
                        statements.push(StatementAst::Conditional {
                            condition,
                            then_statements,
                            else_statements,
                        });
                        i = after_i;
                    } else {
                        validate_qualified_identifier(trailing, line, "routine name")?;
                        statements.push(StatementAst::Conditional {
                            condition,
                            then_statements: vec![StatementAst::Call {
                                name: trailing.to_string(),
                                source_line: line.to_string(),
                            }],
                            else_statements: Vec::new(),
                        });
                        i += 1;
                    }
                    continue;
                }
                if let Some((condition_text, _)) = line
                    .strip_prefix("if")
                    .unwrap_or("")
                    .trim_start()
                    .split_once("->")
                {
                    let condition = parse_statement_condition(
                        condition_text.trim(),
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        named_conditions,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?;
                    let (then_statements, next_i) = parse_statement_arrow_consequence(
                        lines,
                        i,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    let (else_statements, next_i) = parse_optional_else_statement_block(
                        lines,
                        next_i,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::If {
                        condition,
                        then_statements,
                        else_statements,
                    });
                    i = next_i;
                    continue;
                }
                let condition = parse_statement_condition(
                    line.strip_prefix("if ").map(str::trim).unwrap_or(""),
                    line,
                    input_names,
                    global_names,
                    condition_names,
                    named_conditions,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                )?;
                let (then_statements, next_i) = parse_statement_block(
                    lines,
                    i + 1,
                    &["else", BLOCK_CLOSE],
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    input_names,
                    global_names,
                    numeric_globals,
                    condition_names,
                    named_conditions,
                    rule_params,
                )?;
                if next_i == 0 {
                    return Err(parse_error(line, "if block missing closing brace"));
                }
                let (else_statements, next_i) = parse_optional_else_statement_block(
                    lines,
                    next_i,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    input_names,
                    global_names,
                    numeric_globals,
                    condition_names,
                    named_conditions,
                    rule_params,
                )?;
                statements.push(StatementAst::If {
                    condition,
                    then_statements,
                    else_statements,
                });
                i = next_i;
            }
            Some("else") => {
                diagnostics.extend(parse_error(line, "else without if").into_diagnostics());
                i += 1;
            }
            Some("when") => {
                diagnostics.extend(parse_error(line, "use `if` for conditions").into_diagnostics());
                i += 1;
            }
            Some("action") if tokens.len() > 1 => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`action` statements were removed; use explicit input guards and rewrites",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            Some("emit") => {
                let effects = parse_rewrite_effect(line, line)?;
                statements.push(StatementAst::Effect { effects });
                i += 1;
            }
            Some("do") => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "`do` is obsolete; write the effect statement directly",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            _ if is_input_effect_statement(line) => {
                let (input_name, effect_text) = line
                    .split_once("->")
                    .expect("input effect statement contains arrow");
                let input_name = input_name.trim();
                validate_identifier(input_name, line, "input name")?;
                let condition = ConditionAst::InputIs(input_name.to_string());
                let effect_text = effect_text.trim();
                if effect_text.is_empty() || effect_text == "{" {
                    let (then_statements, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::If {
                        condition,
                        then_statements,
                        else_statements: Vec::new(),
                    });
                    i = next_i;
                } else {
                    let effects = parse_rewrite_effect(effect_text, line)?;
                    statements.push(StatementAst::If {
                        condition,
                        then_statements: vec![StatementAst::Effect { effects }],
                        else_statements: Vec::new(),
                    });
                    i += 1;
                }
            }
            _ if is_builtin_rewrite_effect_text(line) => {
                let effects = parse_rewrite_effect(line, line)?;
                statements.push(StatementAst::Effect { effects });
                i += 1;
            }
            Some("[") => {
                if let Some(statement) = parse_conditional_call_statement(
                    line,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                )? {
                    statements.push(statement);
                } else {
                    statements.push(StatementAst::Rewrite(parse_neutral_rewrite_statement(
                        line,
                        None,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    )?));
                }
                i = next_statement_i;
            }
            Some("once") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::Block {
                        application: RuleApplication::Once,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    statements.push(StatementAst::Rewrite(
                        parse_application_prefixed_rewrite_statement(
                            line,
                            "once",
                            RuleApplication::Once,
                            rule_params,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            global_names,
                        )?,
                    ));
                    i = next_statement_i;
                }
            }
            Some("once_all") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::Block {
                        application: RuleApplication::OnceAll,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    statements.push(StatementAst::Rewrite(
                        parse_application_prefixed_rewrite_statement(
                            line,
                            "once_all",
                            RuleApplication::OnceAll,
                            rule_params,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            global_names,
                        )?,
                    ));
                    i = next_statement_i;
                }
            }
            Some("once_per_level") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::Block {
                        application: RuleApplication::OncePerLevel,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    statements.push(StatementAst::Rewrite(
                        parse_application_prefixed_rewrite_statement(
                            line,
                            "once_per_level",
                            RuleApplication::OncePerLevel,
                            rule_params,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            global_names,
                        )?,
                    ));
                    i = next_statement_i;
                }
            }
            Some("repeat") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::Block {
                        application: RuleApplication::UntilStable,
                        statements: nested,
                    });
                    i = next_i;
                } else if tokens.get(1).copied() == Some("until") {
                    let condition_text = line
                        .strip_prefix("repeat")
                        .and_then(|rest| rest.trim_start().strip_prefix("until"))
                        .map(str::trim)
                        .unwrap_or("");
                    if condition_text.is_empty() {
                        return Err(parse_error(
                            line,
                            "repeat until block must be: repeat until <condition>",
                        ));
                    }
                    let condition = parse_statement_condition(
                        condition_text,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        named_conditions,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?;
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::RepeatUntil {
                        condition,
                        statements: nested,
                    });
                    i = next_i;
                } else {
                    statements.push(StatementAst::Rewrite(
                        parse_application_prefixed_rewrite_statement(
                            line,
                            "repeat",
                            RuleApplication::UntilStable,
                            rule_params,
                            object_names,
                            object_schemas,
                            value_sets,
                            maps,
                            object_groups,
                            global_names,
                        )?,
                    ));
                    i = next_statement_i;
                }
            }
            Some(_) if line.starts_with('[') => {
                if let Some(statement) = parse_conditional_call_statement(
                    line,
                    None,
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                )? {
                    statements.push(statement);
                } else {
                    statements.push(StatementAst::Rewrite(parse_neutral_rewrite_statement(
                        line,
                        None,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    )?));
                }
                i = next_statement_i;
            }
            Some("display") => {
                if tokens.len() == 1 {
                    let (nested, next_i) = parse_statement_block(
                        lines,
                        i + 1,
                        &[BLOCK_CLOSE],
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        input_names,
                        global_names,
                        numeric_globals,
                        condition_names,
                        named_conditions,
                        rule_params,
                    )?;
                    statements.push(StatementAst::DisplayBlock(nested));
                    i = next_i;
                } else {
                    statements.push(parse_display_statement(
                        line,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    )?);
                    i += 1;
                }
            }
            Some(first) if is_oriented_rewrite_line(line, first) => {
                if let Some(statement) = parse_conditional_call_statement(
                    line,
                    Some(first),
                    rule_params,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                )? {
                    statements.push(statement);
                } else {
                    statements.push(StatementAst::Rewrite(parse_oriented_rewrite_statement(
                        line,
                        first,
                        None,
                        rule_params,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                        global_names,
                    )?));
                }
                i = next_statement_i;
            }
            Some("move") if is_shared_standard_move_statement(line) => {
                statements.push(StatementAst::Call {
                    name: "move".to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(call) if tokens.len() == 1 && is_shared_rule_call_statement(line, call) => {
                statements.push(StatementAst::Call {
                    name: call.to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(call) if tokens.len() == 1 && is_display_role_token(call) => {
                statements.push(StatementAst::DisplayCall {
                    name: call.to_string(),
                    source_line: line.to_string(),
                });
                i += 1;
            }
            Some(other) if scene_effect_command_syntax(other).is_some() => {
                diagnostics.extend(
                    parse_error(
                        line,
                        &format!(
                            "scene effect `{other}` cannot be used in puzzle statement blocks; \
                         put scene effects in a scene lifecycle, scene routine, \
                         or scene component effect"
                        ),
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            Some(other) => {
                diagnostics.extend(
                    parse_error(line, &format!("unknown statement directive {other}"))
                        .into_diagnostics(),
                );
                i += 1;
            }
            None => i += 1,
        }
    }

    if !diagnostics.is_empty() {
        Err(DiagnosticReport::from_diagnostics(diagnostics))
    } else {
        Err(parse_error(
            &lines[start],
            "statement block missing closing brace",
        ))
    }
}

fn is_shared_standard_move_statement(line: &str) -> bool {
    matches!(
        puzzle_authoring::rule_statement_surface(line),
        Ok(puzzle_authoring::RuleStatementSurface::RuleLine(
            puzzle_authoring::RuleLineSurface::StandardStep(
                puzzle_authoring::StandardRuleStepSurface::Move
            )
        ))
    )
}

fn is_shared_rule_call_statement(line: &str, expected_name: &str) -> bool {
    matches!(
        puzzle_authoring::rule_statement_surface(line),
        Ok(puzzle_authoring::RuleStatementSurface::Call { name }) if name == expected_name
    )
}

#[allow(clippy::too_many_arguments)]
fn parse_display_statement(
    line: &str,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<StatementAst, DiagnosticReport> {
    let rest = line
        .strip_prefix("display")
        .ok_or_else(|| parse_error(line, "display statement must start with display"))?
        .trim_start();
    if rest.is_empty() {
        return Err(parse_error(
            line,
            "display statement must be: display <rule> or display <rewrite>",
        ));
    }

    let tokens = split_header_tokens(rest);
    if tokens.len() == 1 && (is_qualified_identifier(tokens[0]) || is_display_role_token(tokens[0]))
    {
        return Ok(StatementAst::DisplayCall {
            name: tokens[0].to_string(),
            source_line: line.to_string(),
        });
    }

    let rewrite = match tokens.first().copied() {
        Some("once") => parse_application_prefixed_rewrite_statement(
            rest,
            "once",
            RuleApplication::Once,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("once_all") => parse_application_prefixed_rewrite_statement(
            rest,
            "once_all",
            RuleApplication::OnceAll,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("once_per_level") => parse_application_prefixed_rewrite_statement(
            rest,
            "once_per_level",
            RuleApplication::OncePerLevel,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
        )?,
        Some("repeat") => parse_application_prefixed_rewrite_statement(
            rest,
            "repeat",
            RuleApplication::UntilStable,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(first) if is_oriented_rewrite_line(rest, first) => parse_oriented_rewrite_statement(
            rest,
            first,
            None,
            rule_params,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(_) if rest.starts_with('[') => parse_neutral_rewrite_statement(
            rest,
            None,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
        )?,
        Some(other) => {
            return Err(parse_error(
                line,
                &format!("unknown display statement directive {other}"),
            ));
        }
        None => unreachable!("empty display statement already rejected"),
    };

    Ok(StatementAst::DisplayRewrite(rewrite))
}

fn validate_display_hook_statements(statements: &[StatementAst]) -> Result<(), DiagnosticReport> {
    for statement in statements {
        match statement {
            StatementAst::DisplayCall { .. }
            | StatementAst::DisplayRewrite(_)
            | StatementAst::DisplayBlock(_) => {}
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            } => {
                validate_display_hook_statements(then_statements)?;
                validate_display_hook_statements(else_statements)?;
            }
            StatementAst::Block { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::RepeatUntil { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::Fix { statements, .. } => {
                validate_display_hook_statements(statements)?;
            }
            StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                validate_display_hook_statements(then_statements)?;
                validate_display_hook_statements(else_statements)?;
            }
            StatementAst::Call { .. } | StatementAst::Effect { .. } | StatementAst::Rewrite(_) => {
                return Err(DiagnosticReport::error(
                    "on_display can only contain display statements".to_string(),
                ));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn parse_conditional_call_statement(
    line: &str,
    orientation_token: Option<&str>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<Option<StatementAst>, DiagnosticReport> {
    let Some((left, right)) = line.split_once("->") else {
        return Ok(None);
    };
    let rule_name = right.trim();
    if !is_qualified_identifier(rule_name) {
        return Ok(None);
    }
    if is_builtin_rewrite_effect_text(rule_name) {
        return Ok(None);
    }

    let (pattern, orientation) = if let Some(orientation_token) = orientation_token {
        let (orientation, pattern) =
            parse_oriented_rewrite_prefix(left, orientation_token, rule_params)?;
        (pattern, Some(orientation))
    } else {
        (left.trim(), None)
    };
    let condition = parse_pattern_condition(
        PatternPredicateAst::Some,
        pattern,
        line,
        orientation,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;

    Ok(Some(StatementAst::Conditional {
        condition,
        then_statements: vec![StatementAst::Call {
            name: rule_name.to_string(),
            source_line: line.to_string(),
        }],
        else_statements: Vec::new(),
    }))
}

#[allow(clippy::too_many_arguments)]
fn parse_pattern_if_header<'a>(
    line: &'a str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<Option<(PatternConditionAst, &'a str)>, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("if") else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some((predicate, after_keyword)) = parse_pattern_predicate_keyword(rest) else {
        return Ok(None);
    };
    let after_keyword = after_keyword.trim_start();
    let Some(after_open) = after_keyword.strip_prefix('(') else {
        return Err(parse_error(line, "pattern condition must use parentheses"));
    };
    let close_index = matching_close_paren(after_open)
        .ok_or_else(|| parse_error(line, "pattern condition missing )"))?;
    let pattern = after_open[..close_index].trim();
    let trailing = after_open[close_index + 1..].trim();
    let condition = parse_pattern_condition(
        predicate,
        pattern,
        line,
        None,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    Ok(Some((condition, trailing)))
}

fn parse_pattern_predicate_keyword(value: &str) -> Option<(PatternPredicateAst, &str)> {
    if let Some(rest) = value.strip_prefix("some") {
        return Some((PatternPredicateAst::Some, rest));
    }
    if let Some(rest) = value.strip_prefix("none") {
        return Some((PatternPredicateAst::None, rest));
    }
    None
}

fn matching_close_paren(value: &str) -> Option<usize> {
    let mut depth = 1_u16;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn parse_pattern_condition(
    predicate: PatternPredicateAst,
    pattern: &str,
    line: &str,
    orientation: Option<OrientationExpr>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    _global_names: &HashMap<String, GlobalId>,
) -> Result<PatternConditionAst, DiagnosticReport> {
    let Some((pattern_orientation, pattern)) = split_oriented_pattern_arg(pattern, line)? else {
        return Err(parse_error(
            line,
            "pattern condition must contain a pattern",
        ));
    };
    if !matches!(pattern_orientation, OrientationExpr::Neutral) && orientation.is_some() {
        return Err(parse_error(
            line,
            "pattern condition cannot combine multiple orientation prefixes",
        ));
    }
    let orientation = if matches!(pattern_orientation, OrientationExpr::Neutral) {
        orientation.unwrap_or(OrientationExpr::Neutral)
    } else {
        pattern_orientation
    };
    Ok(PatternConditionAst {
        predicate,
        orientation,
        pattern: parse_pattern_side(
            &pattern,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            &HashMap::new(),
            false,
        )?,
    })
}

fn normalize_embedded_direction_marker(pattern: &str) -> (Option<OrientationExpr>, String) {
    let trimmed = pattern.trim();
    let Some(after_open) = trimmed.strip_prefix('[') else {
        return (None, trimmed.to_string());
    };
    let rest = after_open.trim_start();
    let Some(marker) = rest.chars().next() else {
        return (None, trimmed.to_string());
    };
    let Some(direction_name) = embedded_direction_name(marker) else {
        return (None, trimmed.to_string());
    };
    let marker_len = marker.len_utf8();
    let after_marker = &rest[marker_len..];
    if !after_marker.chars().next().is_some_and(char::is_whitespace) {
        return (None, trimmed.to_string());
    }
    let normalized = format!("[{}", after_marker.trim_start());
    (
        Some(OrientationExpr::Fixed(DirectionName(
            direction_name.to_string(),
        ))),
        normalized,
    )
}

fn embedded_direction_name(marker: char) -> Option<&'static str> {
    match marker {
        '>' => Some("right"),
        '<' => Some("left"),
        '^' => Some("up"),
        'v' => Some("down"),
        _ => None,
    }
}

fn is_oriented_rewrite_line(line: &str, orientation_token: &str) -> bool {
    if !line.trim_start().starts_with(orientation_token) {
        return false;
    }
    matches!(
        puzzle_authoring::rule_line_surface(line),
        Ok(puzzle_authoring::RuleLineSurface::InputRewrite { .. })
            | Ok(puzzle_authoring::RuleLineSurface::OrientedRewrite { .. })
    )
}

fn parse_oriented_rewrite_statement(
    line: &str,
    orientation_token: &str,
    application: Option<RuleApplication>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    if !line.trim_start().starts_with(orientation_token) {
        return Err(parse_error(line, "missing oriented rewrite"));
    }
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        rule_params,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if parsed.application.is_some() {
        return Err(parse_error(line, "unexpected application-prefixed rewrite"));
    }
    Ok(OrientedRewriteAst {
        application,
        ..parsed
    })
}

fn parse_neutral_rewrite_statement(
    line: &str,
    application: Option<RuleApplication>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        &[],
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if !matches!(parsed.orientation, OrientationExpr::Neutral) || parsed.application.is_some() {
        return Err(parse_error(line, "expected a neutral rewrite"));
    }
    Ok(OrientedRewriteAst {
        application,
        ..parsed
    })
}

fn parse_application_prefixed_rewrite_statement(
    line: &str,
    prefix: &str,
    application: RuleApplication,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    line.strip_prefix(prefix)
        .ok_or_else(|| parse_error(line, "missing application-prefixed rewrite"))?;
    let surface = puzzle_authoring::rule_line_surface(line)
        .map_err(|error| parse_error(line, error.message()))?;
    let parsed = parse_rule_line_rewrite_statement(
        line,
        surface,
        rule_params,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;
    if parsed.application != Some(application) {
        return Err(parse_error(
            line,
            "application prefix must be followed by a rewrite",
        ));
    }
    Ok(parsed)
}

#[allow(clippy::too_many_arguments)]
fn parse_rule_line_rewrite_statement(
    line: &str,
    surface: puzzle_authoring::RuleLineSurface<'_>,
    rule_params: &[String],
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<OrientedRewriteAst, DiagnosticReport> {
    let (orientation, application, rewrite) = match surface {
        puzzle_authoring::RuleLineSurface::InputRewrite {
            application,
            surface,
        } => {
            if let Some(axis) = surface.orientation {
                validate_identifier(axis, line, "input orientation")?;
            }
            (
                OrientationExpr::InputSet(surface.orientation.unwrap_or("directions").to_string()),
                application.map(rule_application_from_surface),
                surface.rewrite,
            )
        }
        puzzle_authoring::RuleLineSurface::NeutralRewrite {
            application,
            rewrite,
        } => (
            OrientationExpr::Neutral,
            application.map(rule_application_from_surface),
            rewrite,
        ),
        puzzle_authoring::RuleLineSurface::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => (
            parse_statement_orientation_expr(orientation, rule_params),
            application.map(rule_application_from_surface),
            rewrite,
        ),
        puzzle_authoring::RuleLineSurface::StandardStep(_) => {
            return Err(parse_error(line, "expected a rewrite statement"));
        }
    };
    let (before, after, effects, after_effects, after_call) = parse_inline_rewrite(
        rewrite,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
    )?;

    Ok(OrientedRewriteAst {
        source_line: line.to_string(),
        orientation,
        application,
        before,
        after,
        effects,
        after_effects,
        after_call,
    })
}

fn rule_application_from_surface(
    application: puzzle_authoring::RuleApplicationSurface,
) -> RuleApplication {
    match application {
        puzzle_authoring::RuleApplicationSurface::Once => RuleApplication::Once,
        puzzle_authoring::RuleApplicationSurface::OnceAll => RuleApplication::OnceAll,
        puzzle_authoring::RuleApplicationSurface::OncePerLevel => RuleApplication::OncePerLevel,
        puzzle_authoring::RuleApplicationSurface::Repeat => RuleApplication::UntilStable,
    }
}

fn parse_oriented_rewrite_prefix<'a>(
    line: &'a str,
    orientation_token: &str,
    rule_params: &[String],
) -> Result<(OrientationExpr, &'a str), DiagnosticReport> {
    let rest = line
        .strip_prefix(orientation_token)
        .map(str::trim_start)
        .ok_or_else(|| parse_error(line, "missing oriented rewrite"))?;
    if orientation_token == "input" {
        let surface = puzzle_authoring::input_rewrite_surface(line)
            .map_err(|error| parse_error(line, error.message()))?
            .ok_or_else(|| parse_error(line, "missing input-oriented rewrite"))?;
        if let Some(axis) = surface.orientation {
            validate_identifier(axis, line, "input orientation")?;
        }
        return Ok((
            OrientationExpr::InputSet(surface.orientation.unwrap_or("directions").to_string()),
            surface.rewrite,
        ));
    }
    if !rest.starts_with('[') {
        return Err(parse_error(line, "missing oriented rewrite"));
    }
    Ok((
        parse_statement_orientation_expr(orientation_token, rule_params),
        rest,
    ))
}

fn parse_statement_orientation_expr(token: &str, rule_params: &[String]) -> OrientationExpr {
    if token == "input" || rule_params.iter().any(|param| param == token) {
        return OrientationExpr::Input;
    }

    OrientationExpr::Fixed(DirectionName(token.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn parse_statement_condition(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    named_conditions: &HashMap<String, (String, ConditionAst)>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let condition = condition.trim();
    if let Some((_, named_condition)) = named_conditions.get(condition) {
        return Ok(named_condition.clone());
    }
    parse_condition_expr(
        condition,
        line,
        input_names,
        global_names,
        condition_names,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )
}

fn parse_condition_expr(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let or_parts = split_condition_keyword(condition, "or");
    if or_parts.len() > 1 {
        return Ok(ConditionAst::Any(
            or_parts
                .into_iter()
                .map(|part| {
                    parse_condition_expr(
                        &part,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        ));
    }

    let and_parts = split_condition_keyword(condition, "and");
    if and_parts.len() > 1 {
        return Ok(ConditionAst::All(
            and_parts
                .into_iter()
                .map(|part| {
                    parse_condition_expr(
                        &part,
                        line,
                        input_names,
                        global_names,
                        condition_names,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        ));
    }

    parse_condition_atom(
        condition.trim(),
        line,
        input_names,
        global_names,
        condition_names,
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
    )
}

fn split_condition_keyword(condition: &str, keyword: &str) -> Vec<String> {
    condition
        .split_whitespace()
        .collect::<Vec<_>>()
        .split(|token| *token == keyword)
        .map(|part| part.join(" "))
        .filter(|part| !part.trim().is_empty())
        .collect()
}

fn parse_condition_atom(
    condition: &str,
    line: &str,
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
) -> Result<ConditionAst, DiagnosticReport> {
    let tokens = condition.split_whitespace().collect::<Vec<_>>();
    if let ["input", "in", axis] = tokens.as_slice() {
        return Ok(ConditionAst::InputIn((*axis).to_string()));
    }

    if let Some(pattern) = condition.strip_prefix("some ") {
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern.trim(),
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::ExistsMatches(pattern),
            ));
        }
    }
    if let Some(pattern) = condition.strip_prefix("no ") {
        if let Some(pattern) = parse_condition_pattern_arg(
            pattern.trim(),
            line,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
        )? {
            return Ok(ConditionAst::InlineConditionNonZero(
                ConditionValueAst::NoneMatches(pattern),
            ));
        }
    }

    if let Some((left, op, right)) = split_comparison(condition) {
        let left = left.trim();
        let right = right.trim();
        if left == "input" {
            if op != ComparisonOp::Eq {
                return Err(parse_error(line, "input condition only supports =="));
            }
            if input_names.contains_key(right) || is_identifier(right) {
                return Ok(ConditionAst::InputIs(right.to_string()));
            }
            return Err(parse_error(line, "unknown input in condition"));
        }

        let value = parse_global_value(right, line)?;
        if global_names.contains_key(left) {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::GlobalEquals {
                    name: left.to_string(),
                    value,
                },
                op => ConditionAst::GlobalCompare {
                    name: left.to_string(),
                    op,
                    value,
                },
            });
        }
        if condition_names.contains_key(left) {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::ConditionEquals {
                    name: left.to_string(),
                    value,
                },
                op => ConditionAst::ConditionCompare {
                    name: left.to_string(),
                    op,
                    value,
                },
            });
        }
        if left.contains('(') {
            return Ok(match op {
                ComparisonOp::Eq => ConditionAst::InlineConditionValueEquals {
                    kind: parse_condition_value_expr(
                        left,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?,
                    value,
                },
                op => ConditionAst::InlineConditionCompare {
                    kind: parse_condition_value_expr(
                        left,
                        line,
                        object_names,
                        object_schemas,
                        value_sets,
                        maps,
                        object_groups,
                    )?,
                    op,
                    value,
                },
            });
        }
        return Err(parse_error(line, "unknown value in condition"));
    }

    if condition_names.contains_key(condition) {
        return Ok(ConditionAst::ConditionNonZero(condition.to_string()));
    }
    if global_names.contains_key(condition) {
        return Ok(ConditionAst::GlobalCompare {
            name: condition.to_string(),
            op: ComparisonOp::NotEq,
            value: 0,
        });
    }
    if condition.contains('(') {
        return Ok(ConditionAst::InlineConditionNonZero(
            parse_condition_value_expr(
                condition,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
            )?,
        ));
    }

    Err(parse_error(line, "unsupported condition"))
}

fn is_input_effect_statement(line: &str) -> bool {
    let Some((left, _)) = line.split_once("->") else {
        return false;
    };
    is_identifier(left.trim())
}

fn split_comparison(condition: &str) -> Option<(&str, ComparisonOp, &str)> {
    for (token, op) in [
        ("==", ComparisonOp::Eq),
        ("!=", ComparisonOp::NotEq),
        (">=", ComparisonOp::GreaterEq),
        ("<=", ComparisonOp::LessEq),
        (">", ComparisonOp::Greater),
        ("<", ComparisonOp::Less),
    ] {
        if let Some((left, right)) = condition.split_once(token) {
            return Some((left, op, right));
        }
    }
    None
}

struct ProgramLowerer<'a> {
    definitions: HashMap<String, RuleDefinitionAst>,
    object_layers: &'a HashMap<ObjectId, LayerId>,
    input_names: &'a HashMap<String, InputId>,
    global_names: &'a HashMap<String, GlobalId>,
    constant_globals: &'a [GlobalId],
    condition_names: &'a HashMap<String, ConditionId>,
    visual_condition_reads: &'a HashSet<ConditionId>,
    scratch_names: &'a HashMap<String, ScratchDef>,
    model_sound_triggers: &'a [ModelSoundTrigger],
    animation: &'a AnimationDef,
    value_sets: &'a HashMap<String, Vec<String>>,
    directions: &'a [Direction],
    visual_objects: &'a [ObjectId],
    default_wait_ms: u64,
    next_rule_id: u16,
    visual_rules: Vec<RuleId>,
    rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
}

#[derive(Clone, Debug, Default)]
struct StatementLoweringContext {
    guards: Vec<Guard>,
    call_stack: Vec<String>,
    application: RuleApplication,
    application_fixed: bool,
    orientation: Option<OrientationExpr>,
    input_allowed: bool,
    input_forbidden_context: Option<&'static str>,
    role: RuleRole,
}

struct LoweredPrograms {
    main: Vec<RuleStep>,
    level_start: Option<Vec<RuleStep>>,
    level_clear: Option<Vec<RuleStep>>,
    last_level_clear: Option<Vec<RuleStep>>,
    level_starts: Vec<Option<Vec<RuleStep>>>,
    level_clears: Vec<Option<Vec<RuleStep>>>,
    display: Option<Vec<RuleStep>>,
    visual_rules: Vec<RuleId>,
    rule_animations: HashMap<RuleId, Vec<RuleAnimation>>,
    rule_effects: HashMap<RuleId, Vec<RuleEffect>>,
}

#[derive(Clone, Debug, Default)]
struct LoweredEffects {
    core: Vec<Effect>,
    ordered: Vec<RuleEffect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClassifiedRuleRole {
    Main,
    Visual,
}

fn lower_programs(
    definitions: Vec<RuleDefinitionAst>,
    main_statements: Option<Vec<StatementAst>>,
    main_local_frame: Option<LocalFrame<ObjectId>>,
    level_start_statements: Option<Vec<StatementAst>>,
    level_start_local_frame: Option<LocalFrame<ObjectId>>,
    level_clear_statements: Option<Vec<StatementAst>>,
    level_clear_local_frame: Option<LocalFrame<ObjectId>>,
    last_level_clear_statements: Option<Vec<StatementAst>>,
    last_level_clear_local_frame: Option<LocalFrame<ObjectId>>,
    display_statements: Option<Vec<StatementAst>>,
    level_bodies: &[PreparedLevelBody],
    object_layers: &HashMap<ObjectId, LayerId>,
    visual_objects: &[ObjectId],
    input_names: &HashMap<String, InputId>,
    global_names: &HashMap<String, GlobalId>,
    constant_globals: &[GlobalId],
    condition_names: &HashMap<String, ConditionId>,
    visual_condition_reads: &HashSet<ConditionId>,
    scratch_names: &HashMap<String, ScratchDef>,
    model_sound_triggers: &[ModelSoundTrigger],
    animation: &AnimationDef,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[Direction],
    default_wait_ms: u64,
) -> Result<LoweredPrograms, DiagnosticReport> {
    let mut definitions_by_name = HashMap::new();
    for definition in definitions {
        if definitions_by_name
            .insert(definition.name.clone(), definition)
            .is_some()
        {
            return Err(DiagnosticReport::error(
                "duplicate routine definition".to_string(),
            ));
        }
    }
    let Some(main_statements) = main_statements else {
        return Err(DiagnosticReport::error("missing puzzle rules".to_string()));
    };
    let diagnostics = collect_program_reference_diagnostics(
        &definitions_by_name,
        &main_statements,
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        last_level_clear_statements.as_deref(),
        display_statements.as_deref(),
        level_bodies,
    );
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    let mut lowerer = ProgramLowerer {
        definitions: definitions_by_name,
        object_layers,
        input_names,
        global_names,
        constant_globals,
        condition_names,
        visual_condition_reads,
        scratch_names,
        model_sound_triggers,
        animation,
        value_sets,
        directions,
        visual_objects,
        default_wait_ms,
        next_rule_id: 1,
        visual_rules: Vec::new(),
        rule_animations: HashMap::new(),
        rule_effects: HashMap::new(),
    };
    let mut diagnostics = Vec::new();
    let mut context = StatementLoweringContext::default();
    context.input_allowed = true;
    let program = match lowerer.lower_statements(&main_statements, &context) {
        Ok(steps) => Some(wrap_program_local_frame(steps, main_local_frame)),
        Err(report) => {
            diagnostics.extend(report.into_diagnostics());
            None
        }
    };
    let level_start = if let Some(statements) = level_start_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_level_start");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(steps, level_start_local_frame)),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let level_clear = if let Some(statements) = level_clear_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_level_clear");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(steps, level_clear_local_frame)),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let last_level_clear = if let Some(statements) = last_level_clear_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_last_level_clear");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(wrap_program_local_frame(
                steps,
                last_level_clear_local_frame,
            )),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let display = if let Some(statements) = display_statements {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("on_display");
        match lowerer.lower_statements(&statements, &context) {
            Ok(steps) => Some(steps),
            Err(report) => {
                diagnostics.extend(report.into_diagnostics());
                None
            }
        }
    } else {
        None
    };
    let mut level_starts = Vec::with_capacity(level_bodies.len());
    let mut level_clears = Vec::with_capacity(level_bodies.len());
    for level in level_bodies {
        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("level on_level_start");
        level_starts.push(if level.level_start_statements.is_empty() {
            None
        } else {
            match lowerer.lower_statements(&level.level_start_statements, &context) {
                Ok(steps) => Some(steps),
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    None
                }
            }
        });

        let mut context = StatementLoweringContext::default();
        context.input_allowed = false;
        context.input_forbidden_context = Some("level on_level_clear");
        level_clears.push(if level.level_clear_statements.is_empty() {
            None
        } else {
            match lowerer.lower_statements(&level.level_clear_statements, &context) {
                Ok(steps) => Some(steps),
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    None
                }
            }
        });
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    Ok(LoweredPrograms {
        main: program.expect("main program lowered when no diagnostics were reported"),
        level_start,
        level_clear,
        last_level_clear,
        level_starts,
        level_clears,
        display,
        visual_rules: lowerer.visual_rules,
        rule_animations: lowerer.rule_animations,
        rule_effects: lowerer.rule_effects,
    })
}

fn collect_program_reference_diagnostics(
    definitions_by_name: &HashMap<String, RuleDefinitionAst>,
    main_statements: &[StatementAst],
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
    display_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for definition in definitions_by_name.values() {
        collect_statement_reference_diagnostics(
            &definition.statements,
            definitions_by_name,
            &mut diagnostics,
        );
    }
    collect_statement_reference_diagnostics(main_statements, definitions_by_name, &mut diagnostics);
    for statements in [
        level_start_statements,
        level_clear_statements,
        last_level_clear_statements,
        display_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_statement_reference_diagnostics(statements, definitions_by_name, &mut diagnostics);
    }
    for level in level_bodies {
        collect_statement_reference_diagnostics(
            &level.level_start_statements,
            definitions_by_name,
            &mut diagnostics,
        );
        collect_statement_reference_diagnostics(
            &level.level_clear_statements,
            definitions_by_name,
            &mut diagnostics,
        );
    }
    diagnostics
}

fn collect_statement_reference_diagnostics(
    statements: &[StatementAst],
    definitions_by_name: &HashMap<String, RuleDefinitionAst>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        match statement {
            StatementAst::Call { name, source_line } => {
                if !definitions_by_name.contains_key(name) {
                    diagnostics.push(
                        Diagnostic::error(format!("unknown routine call: {name}"))
                            .with_source_line(source_line.clone()),
                    );
                }
            }
            StatementAst::DisplayCall { name, source_line } => {
                if !definitions_by_name.contains_key(name) {
                    diagnostics.push(
                        Diagnostic::error(format!("unknown display routine call: {name}"))
                            .with_source_line(source_line.clone()),
                    );
                }
            }
            StatementAst::DisplayBlock(statements) | StatementAst::Block { statements, .. } => {
                collect_statement_reference_diagnostics(
                    statements,
                    definitions_by_name,
                    diagnostics,
                );
            }
            StatementAst::Conditional {
                then_statements,
                else_statements,
                ..
            }
            | StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_statement_reference_diagnostics(
                    then_statements,
                    definitions_by_name,
                    diagnostics,
                );
                collect_statement_reference_diagnostics(
                    else_statements,
                    definitions_by_name,
                    diagnostics,
                );
            }
            StatementAst::RepeatUntil { statements, .. } | StatementAst::Fix { statements, .. } => {
                collect_statement_reference_diagnostics(
                    statements,
                    definitions_by_name,
                    diagnostics,
                );
            }
            StatementAst::DisplayRewrite(rewrite) | StatementAst::Rewrite(rewrite) => {
                if let Some(name) = &rewrite.after_call {
                    if !definitions_by_name.contains_key(name) {
                        diagnostics.push(
                            Diagnostic::error(format!("unknown routine call: {name}"))
                                .with_source_line(rewrite.source_line.clone()),
                        );
                    }
                }
            }
            StatementAst::Effect { .. } => {}
        }
    }
}

fn wrap_program_local_frame(
    steps: Vec<RuleStep>,
    local_frame: Option<LocalFrame<ObjectId>>,
) -> Vec<RuleStep> {
    match local_frame {
        Some(frame) => vec![RuleStep::LocalFrame { frame, steps }],
        None => steps,
    }
}

fn input_dependency_error(context: &StatementLoweringContext) -> DiagnosticReport {
    let scope = context.input_forbidden_context.unwrap_or("this program");
    DiagnosticReport::error(format!("{scope} cannot depend on input"))
}

fn lower_condition_defs(
    definitions: Vec<ConditionDefinitionAst>,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Result<Vec<ConditionDef>, DiagnosticReport> {
    definitions
        .into_iter()
        .map(|definition| {
            let kind = lower_condition_value_kind(
                &definition.kind,
                input_names,
                object_layers,
                scratch_names,
                value_sets,
                directions,
            )?;
            Ok(ConditionDef {
                id: definition.id,
                kind,
            })
        })
        .collect()
}

fn lower_condition_value_kind(
    kind: &ConditionValueAst,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[Direction],
) -> Result<ConditionValueKind, DiagnosticReport> {
    match kind {
        ConditionValueAst::CountObjects(objects) => {
            Ok(ConditionValueKind::CountObjects(objects.clone()))
        }
        ConditionValueAst::ExistsObjects(objects) => {
            Ok(ConditionValueKind::ExistsObjects(objects.clone()))
        }
        ConditionValueAst::NoneObjects(objects) => {
            Ok(ConditionValueKind::NoneObjects(objects.clone()))
        }
        ConditionValueAst::CountMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::Count,
            input_names,
            object_layers,
            scratch_names,
            value_sets,
            directions,
        ),
        ConditionValueAst::ExistsMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::Exists,
            input_names,
            object_layers,
            scratch_names,
            value_sets,
            directions,
        ),
        ConditionValueAst::NoneMatches(pattern) => lower_condition_match_kind(
            pattern,
            ConditionMatchKind::None,
            input_names,
            object_layers,
            scratch_names,
            value_sets,
            directions,
        ),
    }
}

#[derive(Clone, Copy)]
enum ConditionMatchKind {
    Count,
    Exists,
    None,
}

fn lower_condition_match_kind(
    condition_pattern: &ConditionPatternAst,
    kind: ConditionMatchKind,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[Direction],
) -> Result<ConditionValueKind, DiagnosticReport> {
    if matches!(
        condition_pattern.orientation,
        OrientationExpr::Input | OrientationExpr::InputSet(_)
    ) {
        let patterns = lower_condition_input_patterns(
            condition_pattern,
            input_names,
            object_layers,
            scratch_names,
            value_sets,
            directions,
        )?;
        return Ok(match kind {
            ConditionMatchKind::Count => ConditionValueKind::CountInputMatches(patterns),
            ConditionMatchKind::Exists => ConditionValueKind::ExistsInputMatches(patterns),
            ConditionMatchKind::None => ConditionValueKind::NoneInputMatches(patterns),
        });
    }
    let patterns = lower_condition_patterns(
        condition_pattern,
        object_layers,
        scratch_names,
        value_sets,
        input_names,
        directions,
    )?;
    Ok(match kind {
        ConditionMatchKind::Count => ConditionValueKind::CountMatches(patterns),
        ConditionMatchKind::Exists => ConditionValueKind::ExistsMatches(patterns),
        ConditionMatchKind::None => ConditionValueKind::NoneMatches(patterns),
    })
}

fn lower_condition_patterns(
    condition_pattern: &ConditionPatternAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Result<Vec<Pattern>, DiagnosticReport> {
    let block = &condition_pattern.pattern;
    let alternatives = compile_before_after_blocks(
        block,
        block,
        object_layers,
        scratch_names,
        value_sets,
        "condition pattern",
    )?;
    match &condition_pattern.orientation {
        OrientationExpr::Neutral => {
            if pattern_block_requires_implicit_cardinal_expansion(block) {
                return patterns_from_alternatives(
                    &alternatives,
                    directions,
                    true,
                    "condition pattern",
                );
            }
            patterns_from_alternatives(
                &alternatives,
                &[neutral_direction()],
                false,
                "condition pattern",
            )
        }
        OrientationExpr::Input => {
            patterns_from_alternatives(&alternatives, directions, true, "condition pattern")
        }
        OrientationExpr::InputSet(axis) => {
            let directions =
                directions_for_orientation_name(axis, input_names, value_sets, directions)?
                    .ok_or_else(|| {
                        DiagnosticReport::error(format!("unknown input orientation set: {axis}"))
                    })?;
            patterns_from_alternatives(&alternatives, &directions, true, "condition pattern")
        }
        OrientationExpr::Fixed(direction_name) => {
            let directions = directions_for_orientation_name(
                &direction_name.0,
                input_names,
                value_sets,
                directions,
            )?
            .ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "unknown condition pattern orientation: {}",
                    direction_name.0
                ))
            })?;
            patterns_from_alternatives(&alternatives, &directions, true, "condition pattern")
        }
    }
}

fn lower_condition_input_patterns(
    condition_pattern: &ConditionPatternAst,
    input_names: &HashMap<String, InputId>,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[Direction],
) -> Result<Vec<(InputId, Pattern)>, DiagnosticReport> {
    let block = &condition_pattern.pattern;
    let alternatives = compile_before_after_blocks(
        block,
        block,
        object_layers,
        scratch_names,
        value_sets,
        "condition pattern",
    )?;
    let mut patterns = Vec::new();
    let input_directions = match &condition_pattern.orientation {
        OrientationExpr::Input => directions.to_vec(),
        OrientationExpr::InputSet(axis) => {
            directions_for_orientation_name(axis, input_names, value_sets, directions)?.ok_or_else(
                || DiagnosticReport::error(format!("unknown input orientation set: {axis}")),
            )?
        }
        OrientationExpr::Neutral | OrientationExpr::Fixed(_) => Vec::new(),
    };
    for direction in &input_directions {
        for pattern in
            patterns_from_alternatives(&alternatives, &[*direction], true, "condition pattern")?
        {
            patterns.push((direction.input, pattern));
        }
    }
    Ok(patterns)
}

fn directions_for_orientation_name(
    name: &str,
    input_names: &HashMap<String, InputId>,
    value_sets: &HashMap<String, Vec<String>>,
    directions: &[Direction],
) -> Result<Option<Vec<Direction>>, DiagnosticReport> {
    if let Some(direction) = direction_by_name(name, input_names, directions) {
        return Ok(Some(vec![direction]));
    }
    let Some(values) = value_sets.get(name) else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err(DiagnosticReport::error(format!(
            "empty orientation set: {name}"
        )));
    }
    values
        .iter()
        .map(|value| {
            direction_by_name(value, input_names, directions).ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "orientation set {name} contains non-direction value: {value}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn patterns_from_alternatives(
    alternatives: &[RuleBodyAlternative],
    directions: &[Direction],
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<Pattern>, DiagnosticReport> {
    let mut patterns = Vec::new();
    for direction in directions {
        for alternative in alternatives {
            if !alternative.guards.is_empty() {
                return Err(DiagnosticReport::error(
                    "dynamic object selectors are not supported in condition patterns yet"
                        .to_string(),
                ));
            }
            let components = alternative
                .components
                .iter()
                .map(|component| {
                    let cells = component
                        .cells
                        .iter()
                        .map(|cell| {
                            Ok(MatchCell {
                                offset: resolve_offset(
                                    cell.offset.clone(),
                                    *direction,
                                    direction_expanded,
                                    line,
                                )?,
                                require_objects: cell.require_objects.clone(),
                                require_object_sets: cell.require_object_sets.clone(),
                                forbid_objects: cell.forbid_objects.clone(),
                                require_scratch: resolve_scratch_patterns(
                                    cell.require_scratch.clone(),
                                    *direction,
                                    direction_expanded,
                                    line,
                                )?,
                                require_object_set_scratch: resolve_object_set_scratch_patterns(
                                    cell.require_object_set_scratch.clone(),
                                    *direction,
                                    direction_expanded,
                                    line,
                                )?,
                                forbid_scratch: resolve_scratch_patterns(
                                    cell.forbid_scratch.clone(),
                                    *direction,
                                    direction_expanded,
                                    line,
                                )?,
                                forbid_object_set_scratch: resolve_object_set_scratch_patterns(
                                    cell.forbid_object_set_scratch.clone(),
                                    *direction,
                                    direction_expanded,
                                    line,
                                )?,
                            })
                        })
                        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
                    Ok(PatternComponent {
                        cells,
                        gap_count: component.gap_count,
                    })
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            patterns.push(Pattern { components });
        }
    }
    Ok(patterns)
}

fn lower_goal_condition(
    description: String,
    condition: &ConditionAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    visual_condition_reads: &HashSet<ConditionId>,
    scratch_names: &HashMap<String, ScratchDef>,
    visual_objects: &[ObjectId],
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Result<GoalCondition, DiagnosticReport> {
    Ok(GoalCondition {
        description,
        expr: lower_goal_expr(
            condition,
            object_layers,
            global_names,
            condition_names,
            visual_condition_reads,
            scratch_names,
            visual_objects,
            value_sets,
            input_names,
            directions,
        )?,
    })
}

fn lower_goal_expr(
    condition: &ConditionAst,
    object_layers: &HashMap<ObjectId, LayerId>,
    global_names: &HashMap<String, GlobalId>,
    condition_names: &HashMap<String, ConditionId>,
    visual_condition_reads: &HashSet<ConditionId>,
    scratch_names: &HashMap<String, ScratchDef>,
    visual_objects: &[ObjectId],
    value_sets: &HashMap<String, Vec<String>>,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Result<GoalExpr, DiagnosticReport> {
    match condition {
        ConditionAst::All(conditions) => Ok(GoalExpr::All(
            conditions
                .iter()
                .map(|condition| {
                    lower_goal_expr(
                        condition,
                        object_layers,
                        global_names,
                        condition_names,
                        visual_condition_reads,
                        scratch_names,
                        visual_objects,
                        value_sets,
                        input_names,
                        directions,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        )),
        ConditionAst::Any(conditions) => Ok(GoalExpr::Any(
            conditions
                .iter()
                .map(|condition| {
                    lower_goal_expr(
                        condition,
                        object_layers,
                        global_names,
                        condition_names,
                        visual_condition_reads,
                        scratch_names,
                        visual_objects,
                        value_sets,
                        input_names,
                        directions,
                    )
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?,
        )),
        ConditionAst::GlobalEquals { name, value } => Ok(GoalExpr::Clause(GoalClause {
            value: GoalValue::Global(resolve_global_for_goal(name, global_names)?),
            op: ComparisonOp::Eq,
            expected: *value,
        })),
        ConditionAst::GlobalCompare { name, op, value } => Ok(GoalExpr::Clause(GoalClause {
            value: GoalValue::Global(resolve_global_for_goal(name, global_names)?),
            op: *op,
            expected: *value,
        })),
        ConditionAst::ConditionEquals { name, value } => Ok(GoalExpr::Clause(GoalClause {
            value: GoalValue::Condition(resolve_non_visual_condition_for_goal(
                name,
                condition_names,
                visual_condition_reads,
            )?),
            op: ComparisonOp::Eq,
            expected: *value,
        })),
        ConditionAst::ConditionNonZero(name) => Ok(GoalExpr::Clause(GoalClause {
            value: GoalValue::Condition(resolve_non_visual_condition_for_goal(
                name,
                condition_names,
                visual_condition_reads,
            )?),
            op: ComparisonOp::NotEq,
            expected: 0,
        })),
        ConditionAst::ConditionCompare { name, op, value } => Ok(GoalExpr::Clause(GoalClause {
            value: GoalValue::Condition(resolve_non_visual_condition_for_goal(
                name,
                condition_names,
                visual_condition_reads,
            )?),
            op: *op,
            expected: *value,
        })),
        ConditionAst::InlineConditionValueEquals { kind, value } => {
            validate_non_visual_condition_value(kind, visual_objects)?;
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                scratch_names,
                value_sets,
                directions,
            )?;
            Ok(GoalExpr::Clause(GoalClause {
                value: GoalValue::InlineConditionValue(kind),
                op: ComparisonOp::Eq,
                expected: *value,
            }))
        }
        ConditionAst::InlineConditionNonZero(kind) => {
            validate_non_visual_condition_value(kind, visual_objects)?;
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                scratch_names,
                value_sets,
                directions,
            )?;
            Ok(GoalExpr::Clause(GoalClause {
                value: GoalValue::InlineConditionValue(kind),
                op: ComparisonOp::NotEq,
                expected: 0,
            }))
        }
        ConditionAst::InlineConditionCompare { kind, op, value } => {
            validate_non_visual_condition_value(kind, visual_objects)?;
            let kind = lower_condition_value_kind(
                kind,
                input_names,
                object_layers,
                scratch_names,
                value_sets,
                directions,
            )?;
            Ok(GoalExpr::Clause(GoalClause {
                value: GoalValue::InlineConditionValue(kind),
                op: *op,
                expected: *value,
            }))
        }
        ConditionAst::InputIs(_) | ConditionAst::InputIn(_) => Err(DiagnosticReport::error(
            "goal cannot depend on input".to_string(),
        )),
    }
}

fn resolve_global_for_goal(
    name: &str,
    global_names: &HashMap<String, GlobalId>,
) -> Result<GlobalId, DiagnosticReport> {
    global_names
        .get(name)
        .copied()
        .ok_or_else(|| DiagnosticReport::error(format!("unknown global in goal: {name}")))
}

fn resolve_non_visual_condition_for_goal(
    name: &str,
    condition_names: &HashMap<String, ConditionId>,
    visual_condition_reads: &HashSet<ConditionId>,
) -> Result<ConditionId, DiagnosticReport> {
    let condition = condition_names
        .get(name)
        .copied()
        .ok_or_else(|| DiagnosticReport::error(format!("unknown condition in goal: {name}")))?;
    ensure_non_visual_condition_def(condition, visual_condition_reads)?;
    Ok(condition)
}

fn visual_condition_reads(
    condition_defs: &[ConditionDefinitionAst],
    visual_objects: &[ObjectId],
) -> HashSet<ConditionId> {
    condition_defs
        .iter()
        .filter_map(|condition| {
            condition_value_reads_visual_object(&condition.kind, visual_objects)
                .then_some(condition.id)
        })
        .collect()
}

fn ensure_non_visual_condition_def(
    condition: ConditionId,
    visual_condition_reads: &HashSet<ConditionId>,
) -> Result<(), DiagnosticReport> {
    if visual_condition_reads.contains(&condition) {
        return Err(main_visual_object_error("condition"));
    }
    Ok(())
}

fn validate_non_visual_condition_value(
    kind: &ConditionValueAst,
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    if condition_value_reads_visual_object(kind, visual_objects) {
        return Err(main_visual_object_error("condition"));
    }
    Ok(())
}

fn condition_value_reads_visual_object(
    kind: &ConditionValueAst,
    visual_objects: &[ObjectId],
) -> bool {
    match kind {
        ConditionValueAst::CountObjects(objects)
        | ConditionValueAst::ExistsObjects(objects)
        | ConditionValueAst::NoneObjects(objects) => objects
            .iter()
            .any(|object| object_is_visual(*object, visual_objects)),
        ConditionValueAst::CountMatches(pattern)
        | ConditionValueAst::ExistsMatches(pattern)
        | ConditionValueAst::NoneMatches(pattern) => {
            pattern_block_reads_visual_object(&pattern.pattern, visual_objects)
        }
    }
}

fn validate_non_visual_pattern_block(
    pattern: &PatternBlock,
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    if pattern_block_reads_visual_object(pattern, visual_objects) {
        return Err(main_visual_object_error("pattern"));
    }
    Ok(())
}

fn pattern_block_reads_visual_object(pattern: &PatternBlock, visual_objects: &[ObjectId]) -> bool {
    pattern.components.iter().any(|component| {
        component.rows.iter().flatten().any(|part| match part {
            BlockPart::Cell(cell) => {
                selectors_read_visual_object(&cell.require, visual_objects)
                    || selectors_read_visual_object(&cell.forbid, visual_objects)
            }
            BlockPart::Ellipsis => false,
        })
    })
}

fn selectors_read_visual_object(selectors: &[ObjectSelector], visual_objects: &[ObjectId]) -> bool {
    selectors.iter().any(|selector| {
        selector
            .alternatives
            .iter()
            .any(|object| object_is_visual(*object, visual_objects))
    })
}

fn object_is_visual(object: ObjectId, visual_objects: &[ObjectId]) -> bool {
    visual_objects.contains(&object)
}

fn main_visual_object_error(context: &str) -> DiagnosticReport {
    DiagnosticReport::error(format!(
        "main rules and conditions cannot read or write display objects ({context})"
    ))
}

fn classify_rewrite_role(
    before: &PatternBlock,
    alternatives: &[RuleBodyAlternative],
    effects: &LoweredEffects,
    visual_objects: &[ObjectId],
    context: &StatementLoweringContext,
) -> Result<ClassifiedRuleRole, DiagnosticReport> {
    let reads_visual = pattern_block_reads_visual_object(before, visual_objects);
    let writes_visual = alternatives_write_visual_objects(alternatives, visual_objects);
    let writes_main = alternatives_write_main_state(alternatives, visual_objects);
    let has_effects = !effects.core.is_empty() || !effects.ordered.is_empty();

    if reads_visual && (writes_main || has_effects) {
        return Err(DiagnosticReport::error(
            "display object matches cannot cause gameplay changes".to_string(),
        ));
    }

    let role = if reads_visual {
        ClassifiedRuleRole::Visual
    } else if writes_main || has_effects {
        ClassifiedRuleRole::Main
    } else if writes_visual {
        ClassifiedRuleRole::Visual
    } else {
        ClassifiedRuleRole::Main
    };

    if context.role == RuleRole::Visual && role == ClassifiedRuleRole::Main {
        return Err(DiagnosticReport::error(
            "display routines and display blocks can only contain display rules".to_string(),
        ));
    }

    Ok(role)
}

fn alternatives_write_visual_objects(
    alternatives: &[RuleBodyAlternative],
    visual_objects: &[ObjectId],
) -> bool {
    alternatives.iter().any(|alternative| {
        alternative
            .writes
            .iter()
            .any(|write| write_template_touches_visual_object(write, alternative, visual_objects))
    })
}

fn alternatives_write_main_state(
    alternatives: &[RuleBodyAlternative],
    visual_objects: &[ObjectId],
) -> bool {
    alternatives.iter().any(|alternative| {
        alternative
            .writes
            .iter()
            .any(|write| write_template_touches_main_state(write, alternative, visual_objects))
    })
}

fn write_template_touches_visual_object(
    write: &WriteOpTemplate,
    alternative: &RuleBodyAlternative,
    visual_objects: &[ObjectId],
) -> bool {
    match write {
        WriteOpTemplate::Add { object, .. }
        | WriteOpTemplate::Remove { object, .. }
        | WriteOpTemplate::Move { object, .. } => object_is_visual(*object, visual_objects),
        WriteOpTemplate::MoveObjectSet { objects, .. } => objects
            .iter()
            .any(|object| object_is_visual(*object, visual_objects)),
        WriteOpTemplate::AddObjectSet { objects, .. }
        | WriteOpTemplate::RemoveObjectSet { objects, .. } => objects
            .iter()
            .any(|object| object_is_visual(*object, visual_objects)),
        WriteOpTemplate::SetObjectSetScratch { binding, .. }
        | WriteOpTemplate::RemoveObjectSetScratch { binding, .. } => {
            object_set_binding_touches_visual_object(*binding, alternative, visual_objects)
        }
        WriteOpTemplate::SetScratch { object, .. }
        | WriteOpTemplate::RemoveScratch { object, .. } => {
            !object.is_empty() && object_is_visual(*object, visual_objects)
        }
    }
}

fn write_template_touches_main_state(
    write: &WriteOpTemplate,
    alternative: &RuleBodyAlternative,
    visual_objects: &[ObjectId],
) -> bool {
    match write {
        WriteOpTemplate::Add { object, .. }
        | WriteOpTemplate::Remove { object, .. }
        | WriteOpTemplate::Move { object, .. } => !object_is_visual(*object, visual_objects),
        WriteOpTemplate::MoveObjectSet { objects, .. } => objects
            .iter()
            .any(|object| !object_is_visual(*object, visual_objects)),
        WriteOpTemplate::AddObjectSet { objects, .. }
        | WriteOpTemplate::RemoveObjectSet { objects, .. } => objects
            .iter()
            .any(|object| !object_is_visual(*object, visual_objects)),
        WriteOpTemplate::SetObjectSetScratch { binding, .. }
        | WriteOpTemplate::RemoveObjectSetScratch { binding, .. } => {
            object_set_binding_touches_main_state(*binding, alternative, visual_objects)
        }
        WriteOpTemplate::SetScratch { object, .. }
        | WriteOpTemplate::RemoveScratch { object, .. } => {
            object.is_empty() || !object_is_visual(*object, visual_objects)
        }
    }
}

fn object_set_binding_objects(
    binding: u16,
    alternative: &RuleBodyAlternative,
) -> Option<Vec<ObjectId>> {
    let mut objects = Vec::new();
    for component in &alternative.components {
        for cell in &component.cells {
            for matcher in &cell.require_object_sets {
                if matcher.binding != binding {
                    continue;
                }
                for object in &matcher.objects {
                    if !objects.contains(object) {
                        objects.push(*object);
                    }
                }
            }
        }
    }
    (!objects.is_empty()).then_some(objects)
}

fn object_set_binding_touches_visual_object(
    binding: u16,
    alternative: &RuleBodyAlternative,
    visual_objects: &[ObjectId],
) -> bool {
    object_set_binding_objects(binding, alternative).map_or(true, |objects| {
        objects
            .iter()
            .any(|object| object_is_visual(*object, visual_objects))
    })
}

fn object_set_binding_touches_main_state(
    binding: u16,
    alternative: &RuleBodyAlternative,
    visual_objects: &[ObjectId],
) -> bool {
    object_set_binding_objects(binding, alternative).map_or(true, |objects| {
        objects
            .iter()
            .any(|object| !object_is_visual(*object, visual_objects))
    })
}

impl<'a> ProgramLowerer<'a> {
    fn lower_statements(
        &mut self,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut rules = Vec::new();
        for statement in statements {
            rules.extend(self.lower_statement(statement, context)?);
        }
        Ok(rules)
    }

    fn lower_statement(
        &mut self,
        statement: &StatementAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        match statement {
            StatementAst::Call { name, source_line } => self.lower_call(name, source_line, context),
            StatementAst::DisplayCall { name, source_line } => {
                self.lower_display_call(name, source_line, context)
            }
            StatementAst::DisplayRewrite(rewrite) => self.lower_display_rewrite(rewrite, context),
            StatementAst::DisplayBlock(statements) => self.lower_display_block(statements, context),
            StatementAst::Conditional {
                condition,
                then_statements,
                else_statements,
            } => self.lower_conditional(condition, then_statements, else_statements, context),
            StatementAst::Block {
                application,
                statements,
            } => self.lower_block(*application, statements, context),
            StatementAst::RepeatUntil {
                condition,
                statements,
            } => self.lower_repeat_until(condition, statements, context),
            StatementAst::Fix {
                defaults,
                statements,
            } => self.lower_fix(defaults, statements, context),
            StatementAst::If {
                condition,
                then_statements,
                else_statements,
            } => self.lower_if(condition, then_statements, else_statements, context),
            StatementAst::Effect { effects } => self.lower_effect_statement(effects, context),
            StatementAst::Rewrite(rewrite) => self.lower_rewrite(rewrite, context),
        }
    }

    fn lower_effect_statement(
        &mut self,
        effects: &[EffectAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let effects = self.lower_effects(effects)?;
        if context.role == RuleRole::Visual {
            validate_visual_effects(&effects)?;
        }
        let id = RuleId(self.next_rule_id);
        self.next_rule_id += 1;
        if context.role == RuleRole::Visual {
            self.visual_rules.push(id);
        }
        if !effects.ordered.is_empty() {
            self.rule_effects.insert(id, effects.ordered.clone());
        }
        Ok(vec![RuleStep::Rule(Rule {
            id,
            guards: context.guards.clone(),
            application: RuleApplication::Once,
            pattern: Pattern {
                components: Vec::new(),
            },
            writes: Vec::new(),
            effects: effects.core,
        })])
    }

    fn lower_conditional(
        &mut self,
        condition: &PatternConditionAst,
        then_statements: &[StatementAst],
        else_statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        if else_statements.is_empty() {
            return Ok(vec![RuleStep::ConditionalBlock {
                condition: self.lower_pattern_condition(condition, context)?,
                steps: self.lower_statements(then_statements, context)?,
            }]);
        }
        Ok(vec![RuleStep::ConditionalBranch {
            condition: self.lower_pattern_condition(condition, context)?,
            then_steps: self.lower_statements(then_statements, context)?,
            else_steps: self.lower_statements(else_statements, context)?,
        }])
    }

    fn lower_block(
        &mut self,
        application: RuleApplication,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if !nested_context.application_fixed {
            nested_context.application = RuleApplication::UntilStable;
        }
        let steps = self.lower_statements(statements, &nested_context)?;
        Ok(vec![RuleStep::Block {
            application,
            stop_condition: None,
            steps,
        }])
    }

    fn lower_repeat_until(
        &mut self,
        condition: &ConditionAst,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if !nested_context.application_fixed {
            nested_context.application = RuleApplication::UntilStable;
        }
        let stop_condition = self.lower_guard_condition(condition, context)?;
        let steps = self.lower_statements(statements, &nested_context)?;
        Ok(vec![RuleStep::Block {
            application: RuleApplication::UntilStable,
            stop_condition: Some(stop_condition),
            steps,
        }])
    }

    fn lower_fix(
        &mut self,
        defaults: &FixDefaults,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        if let Some(application) = defaults.application {
            nested_context.application = application;
            nested_context.application_fixed = true;
        }
        if let Some(orientation) = &defaults.orientation {
            nested_context.orientation = Some(orientation.clone());
        }
        self.lower_statements(statements, &nested_context)
    }

    fn lower_call(
        &mut self,
        name: &str,
        source_line: &str,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        if context.call_stack.iter().any(|active| active == name) {
            return Err(DiagnosticReport::error_at_line(
                format!("recursive routine call: {name}"),
                source_line,
            ));
        }
        let definition = self.definitions.get(name).cloned().ok_or_else(|| {
            DiagnosticReport::error_at_line(format!("unknown routine call: {name}"), source_line)
        })?;
        let mut nested_context = context.clone();
        if context.role == RuleRole::Visual || definition.role == RuleRole::Visual {
            nested_context.role = RuleRole::Visual;
        }
        nested_context.call_stack.push(name.to_string());
        nested_context.application = RuleApplication::UntilStable;
        nested_context.application_fixed = false;
        nested_context.orientation = None;
        let steps = self.lower_statements(&definition.statements, &nested_context)?;
        Ok(vec![RuleStep::Block {
            application: definition.application,
            stop_condition: None,
            steps,
        }])
    }

    fn lower_display_block(
        &mut self,
        statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        nested_context.role = RuleRole::Visual;
        self.lower_statements(statements, &nested_context)
    }

    fn lower_display_rewrite(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut nested_context = context.clone();
        nested_context.role = RuleRole::Visual;
        self.lower_rewrite(rewrite, &nested_context)
    }

    fn lower_display_call(
        &mut self,
        name: &str,
        source_line: &str,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        if context.call_stack.iter().any(|active| active == name) {
            return Err(DiagnosticReport::error_at_line(
                format!("recursive routine call: {name}"),
                source_line,
            ));
        }
        let definition = self.definitions.get(name).cloned().ok_or_else(|| {
            DiagnosticReport::error_at_line(
                format!("unknown display routine call: {name}"),
                source_line,
            )
        })?;
        let mut nested_context = context.clone();
        nested_context.role = RuleRole::Visual;
        nested_context.call_stack.push(name.to_string());
        nested_context.application = RuleApplication::UntilStable;
        nested_context.application_fixed = false;
        nested_context.orientation = None;
        let steps = self.lower_statements(&definition.statements, &nested_context)?;
        Ok(vec![RuleStep::Block {
            application: definition.application,
            stop_condition: None,
            steps,
        }])
    }

    fn lower_pattern_condition(
        &self,
        condition: &PatternConditionAst,
        context: &StatementLoweringContext,
    ) -> Result<RuleCondition, DiagnosticReport> {
        if context.role == RuleRole::Main {
            validate_non_visual_pattern_block(&condition.pattern, self.visual_objects)?;
        }
        let orientation = if matches!(condition.orientation, OrientationExpr::Neutral) {
            context
                .orientation
                .as_ref()
                .unwrap_or(&condition.orientation)
        } else {
            &condition.orientation
        };
        let patterns = match orientation {
            OrientationExpr::Neutral => {
                if pattern_block_requires_implicit_cardinal_expansion(&condition.pattern) {
                    self.condition_patterns_for_directions(
                        &condition.pattern,
                        self.directions,
                        true,
                        "implicit directional pattern condition",
                    )?
                } else {
                    self.condition_patterns(
                        &condition.pattern,
                        neutral_direction(),
                        false,
                        "neutral pattern condition",
                    )?
                }
            }
            OrientationExpr::Input => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                let mut patterns = Vec::new();
                for direction in self.directions {
                    for pattern in self.condition_patterns(
                        &condition.pattern,
                        *direction,
                        true,
                        "input pattern condition",
                    )? {
                        patterns.push((direction.input, pattern));
                    }
                }
                return Ok(match condition.predicate {
                    PatternPredicateAst::Some => RuleCondition::AnyInputMatches(patterns),
                    PatternPredicateAst::None => RuleCondition::NoInputMatches(patterns),
                });
            }
            OrientationExpr::InputSet(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                let directions = self.directions_for_orientation_name(axis)?.ok_or_else(|| {
                    DiagnosticReport::error(format!("unknown input orientation set: {axis}"))
                })?;
                let mut patterns = Vec::new();
                for direction in directions {
                    for pattern in self.condition_patterns(
                        &condition.pattern,
                        direction,
                        true,
                        "input pattern condition",
                    )? {
                        patterns.push((direction.input, pattern));
                    }
                }
                return Ok(match condition.predicate {
                    PatternPredicateAst::Some => RuleCondition::AnyInputMatches(patterns),
                    PatternPredicateAst::None => RuleCondition::NoInputMatches(patterns),
                });
            }
            OrientationExpr::Fixed(direction_name) => {
                let directions = self
                    .directions_for_orientation_name(&direction_name.0)?
                    .ok_or_else(|| {
                        DiagnosticReport::error(format!(
                            "unknown pattern condition orientation: {}",
                            direction_name.0
                        ))
                    })?;
                self.condition_patterns_for_directions(
                    &condition.pattern,
                    &directions,
                    true,
                    "fixed pattern condition",
                )?
            }
        };

        let condition = match condition.predicate {
            PatternPredicateAst::Some => RuleCondition::AnyMatches(patterns),
            PatternPredicateAst::None => RuleCondition::NoMatches(patterns),
        };
        Ok(condition)
    }

    fn lower_guard_condition(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
    ) -> Result<RuleCondition, DiagnosticReport> {
        Ok(RuleCondition::GuardBranches(
            self.lower_condition_branches(condition, context)?,
        ))
    }

    fn condition_patterns(
        &self,
        pattern: &PatternBlock,
        direction: Direction,
        direction_expanded: bool,
        line: &str,
    ) -> Result<Vec<Pattern>, DiagnosticReport> {
        let alternatives = compile_before_after_blocks(
            pattern,
            pattern,
            self.object_layers,
            self.scratch_names,
            self.value_sets,
            line,
        )?;
        patterns_from_alternatives(&alternatives, &[direction], direction_expanded, line)
    }

    fn condition_patterns_for_directions(
        &self,
        pattern: &PatternBlock,
        directions: &[Direction],
        direction_expanded: bool,
        line: &str,
    ) -> Result<Vec<Pattern>, DiagnosticReport> {
        let alternatives = compile_before_after_blocks(
            pattern,
            pattern,
            self.object_layers,
            self.scratch_names,
            self.value_sets,
            line,
        )?;
        patterns_from_alternatives(&alternatives, directions, direction_expanded, line)
    }

    fn lower_if(
        &mut self,
        condition: &ConditionAst,
        then_statements: &[StatementAst],
        else_statements: &[StatementAst],
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        if !else_statements.is_empty() {
            return Ok(vec![RuleStep::ConditionalBranch {
                condition: self.lower_guard_condition(condition, context)?,
                then_steps: self.lower_statements(then_statements, context)?,
                else_steps: self.lower_statements(else_statements, context)?,
            }]);
        }
        Ok(vec![RuleStep::ConditionalBlock {
            condition: self.lower_guard_condition(condition, context)?,
            steps: self.lower_statements(then_statements, context)?,
        }])
    }

    fn input_ids_for_value_set(&self, name: &str) -> Result<Vec<InputId>, DiagnosticReport> {
        let values = self
            .value_sets
            .get(name)
            .ok_or_else(|| DiagnosticReport::error(format!("unknown input tag set: {name}")))?;
        if values.is_empty() {
            return Err(DiagnosticReport::error(format!(
                "empty input tag set: {name}"
            )));
        }
        values
            .iter()
            .map(|value| {
                self.input_names.get(value).copied().ok_or_else(|| {
                    DiagnosticReport::error(format!("unknown input in tag set: {value}"))
                })
            })
            .collect()
    }

    fn directions_for_orientation_name(
        &self,
        name: &str,
    ) -> Result<Option<Vec<Direction>>, DiagnosticReport> {
        directions_for_orientation_name(name, self.input_names, self.value_sets, self.directions)
    }

    fn lower_condition_branches(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<Vec<Guard>>, DiagnosticReport> {
        match condition {
            ConditionAst::All(conditions) => {
                let mut branches = vec![Vec::<Guard>::new()];
                for condition in conditions {
                    let next_branches = self.lower_condition_branches(condition, context)?;
                    let mut combined = Vec::new();
                    for branch in &branches {
                        for next_branch in &next_branches {
                            let mut merged = branch.clone();
                            merged.extend(next_branch.clone());
                            combined.push(merged);
                        }
                    }
                    branches = combined;
                }
                Ok(branches)
            }
            ConditionAst::Any(conditions) => {
                let mut branches = Vec::new();
                for condition in conditions {
                    branches.extend(self.lower_condition_branches(condition, context)?);
                }
                Ok(branches)
            }
            ConditionAst::InputIn(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                Ok(self
                    .input_ids_for_value_set(axis)?
                    .into_iter()
                    .map(|input| vec![Guard::InputIs(input)])
                    .collect())
            }
            _ => Ok(vec![vec![self.lower_condition_clause(condition, context)?]]),
        }
    }

    fn lower_condition_clause(
        &self,
        condition: &ConditionAst,
        context: &StatementLoweringContext,
    ) -> Result<Guard, DiagnosticReport> {
        match condition {
            ConditionAst::InputIs(input_name) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                let input = *self.input_names.get(input_name).ok_or_else(|| {
                    DiagnosticReport::error(format!("unknown input: {input_name}"))
                })?;
                Ok(Guard::InputIs(input))
            }
            ConditionAst::InputIn(_) => Err(DiagnosticReport::error(
                "input tag-set condition was not expanded".to_string(),
            )),
            ConditionAst::GlobalEquals { name, value } => {
                let global = *self
                    .global_names
                    .get(name)
                    .ok_or_else(|| DiagnosticReport::error(format!("unknown global: {name}")))?;
                Ok(Guard::GlobalEquals {
                    global,
                    value: *value,
                })
            }
            ConditionAst::GlobalCompare { name, op, value } => {
                let global = *self
                    .global_names
                    .get(name)
                    .ok_or_else(|| DiagnosticReport::error(format!("unknown global: {name}")))?;
                Ok(Guard::GlobalCompare {
                    global,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::ConditionEquals { name, value } => {
                let condition = *self
                    .condition_names
                    .get(name)
                    .ok_or_else(|| DiagnosticReport::error(format!("unknown condition: {name}")))?;
                if context.role == RuleRole::Main {
                    ensure_non_visual_condition_def(condition, self.visual_condition_reads)?;
                }
                Ok(Guard::ConditionEquals {
                    condition,
                    value: *value,
                })
            }
            ConditionAst::ConditionNonZero(name) => {
                let condition = *self
                    .condition_names
                    .get(name)
                    .ok_or_else(|| DiagnosticReport::error(format!("unknown condition: {name}")))?;
                if context.role == RuleRole::Main {
                    ensure_non_visual_condition_def(condition, self.visual_condition_reads)?;
                }
                Ok(Guard::ConditionNonZero(condition))
            }
            ConditionAst::ConditionCompare { name, op, value } => {
                let condition = *self
                    .condition_names
                    .get(name)
                    .ok_or_else(|| DiagnosticReport::error(format!("unknown condition: {name}")))?;
                if context.role == RuleRole::Main {
                    ensure_non_visual_condition_def(condition, self.visual_condition_reads)?;
                }
                Ok(Guard::ConditionCompare {
                    condition,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::InlineConditionValueEquals { kind, value } => {
                if context.role == RuleRole::Main {
                    validate_non_visual_condition_value(kind, self.visual_objects)?;
                }
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    self.directions,
                )?;
                Ok(Guard::InlineConditionValue {
                    kind,
                    value: *value,
                })
            }
            ConditionAst::InlineConditionNonZero(kind) => {
                if context.role == RuleRole::Main {
                    validate_non_visual_condition_value(kind, self.visual_objects)?;
                }
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    self.directions,
                )?;
                Ok(Guard::InlineConditionNonZero(kind))
            }
            ConditionAst::InlineConditionCompare { kind, op, value } => {
                if context.role == RuleRole::Main {
                    validate_non_visual_condition_value(kind, self.visual_objects)?;
                }
                let kind = lower_condition_value_kind(
                    kind,
                    self.input_names,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    self.directions,
                )?;
                Ok(Guard::InlineConditionCompare {
                    kind,
                    op: *op,
                    value: *value,
                })
            }
            ConditionAst::All(_) | ConditionAst::Any(_) => Err(DiagnosticReport::error(
                "nested condition expression was not expanded".to_string(),
            )),
        }
    }

    fn lower_rewrite(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let steps = self.lower_rewrite_core(rewrite, context)?;
        if rewrite.after_effects.is_empty() && rewrite.after_call.is_none() {
            return Ok(steps);
        }

        let mut then_steps = Vec::new();
        if !rewrite.after_effects.is_empty() {
            then_steps.extend(self.lower_effect_statement(&rewrite.after_effects, context)?);
        }
        if let Some(after_call) = &rewrite.after_call {
            then_steps.extend(self.lower_call(after_call, &rewrite.source_line, context)?);
        }
        Ok(vec![RuleStep::AfterTriggered { steps, then_steps }])
    }

    fn lower_rewrite_core(
        &mut self,
        rewrite: &OrientedRewriteAst,
        context: &StatementLoweringContext,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let effects = self.lower_effects(&rewrite.effects)?;
        let application = if effects
            .core
            .iter()
            .any(|effect| matches!(effect, Effect::Win | Effect::NextLevel))
        {
            RuleApplication::Once
        } else {
            rewrite.application.unwrap_or(context.application)
        };
        let orientation = if matches!(rewrite.orientation, OrientationExpr::Neutral) {
            context.orientation.as_ref().unwrap_or(&rewrite.orientation)
        } else {
            &rewrite.orientation
        };
        match orientation {
            OrientationExpr::Neutral => {
                let alternatives = compile_before_after_blocks(
                    &rewrite.before,
                    &rewrite.after,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    &rewrite.source_line,
                )?;
                let role = classify_rewrite_role(
                    &rewrite.before,
                    &alternatives,
                    &effects,
                    self.visual_objects,
                    context,
                )?;
                if role == ClassifiedRuleRole::Visual {
                    validate_visual_effects(&effects)?;
                }
                if rewrite_requires_implicit_cardinal_expansion(rewrite) {
                    let mut rules = Vec::new();
                    for direction in self.directions {
                        rules.extend(self.rules_from_alternatives(
                            alternatives.clone(),
                            *direction,
                            true,
                            context.guards.clone(),
                            effects.core.clone(),
                            effects.ordered.clone(),
                            application,
                            role,
                        )?);
                    }
                    return Ok(wrap_rewrite_steps(application, rules));
                }
                self.rules_from_alternatives(
                    alternatives,
                    neutral_direction(),
                    false,
                    context.guards.clone(),
                    effects.core,
                    effects.ordered,
                    application,
                    role,
                )
                .map(|rules| wrap_rewrite_steps(application, rules))
            }
            OrientationExpr::Input => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                let alternatives = compile_before_after_blocks(
                    &rewrite.before,
                    &rewrite.after,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    &rewrite.source_line,
                )?;
                let role = classify_rewrite_role(
                    &rewrite.before,
                    &alternatives,
                    &effects,
                    self.visual_objects,
                    context,
                )?;
                if role == ClassifiedRuleRole::Visual {
                    validate_visual_effects(&effects)?;
                }
                let mut rules = Vec::new();
                for direction in self.directions {
                    let mut guards = context.guards.clone();
                    guards.push(Guard::InputIs(direction.input));
                    rules.extend(self.rules_from_alternatives(
                        alternatives.clone(),
                        *direction,
                        true,
                        guards,
                        effects.core.clone(),
                        effects.ordered.clone(),
                        application,
                        role,
                    )?);
                }
                Ok(wrap_rewrite_steps(application, rules))
            }
            OrientationExpr::InputSet(axis) => {
                if !context.input_allowed {
                    return Err(input_dependency_error(context));
                }
                let directions = self.directions_for_orientation_name(axis)?.ok_or_else(|| {
                    DiagnosticReport::error(format!("unknown input orientation set: {axis}"))
                })?;
                let alternatives = compile_before_after_blocks(
                    &rewrite.before,
                    &rewrite.after,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    &rewrite.source_line,
                )?;
                let role = classify_rewrite_role(
                    &rewrite.before,
                    &alternatives,
                    &effects,
                    self.visual_objects,
                    context,
                )?;
                if role == ClassifiedRuleRole::Visual {
                    validate_visual_effects(&effects)?;
                }
                let mut rules = Vec::new();
                for direction in directions {
                    let mut guards = context.guards.clone();
                    guards.push(Guard::InputIs(direction.input));
                    rules.extend(self.rules_from_alternatives(
                        alternatives.clone(),
                        direction,
                        true,
                        guards,
                        effects.core.clone(),
                        effects.ordered.clone(),
                        application,
                        role,
                    )?);
                }
                Ok(wrap_rewrite_steps(application, rules))
            }
            OrientationExpr::Fixed(direction_name) => {
                let directions = self
                    .directions_for_orientation_name(&direction_name.0)?
                    .ok_or_else(|| {
                        DiagnosticReport::error(format!(
                            "unknown orientation: {}",
                            direction_name.0
                        ))
                    })?;
                let alternatives = compile_before_after_blocks(
                    &rewrite.before,
                    &rewrite.after,
                    self.object_layers,
                    self.scratch_names,
                    self.value_sets,
                    &rewrite.source_line,
                )?;
                let role = classify_rewrite_role(
                    &rewrite.before,
                    &alternatives,
                    &effects,
                    self.visual_objects,
                    context,
                )?;
                if role == ClassifiedRuleRole::Visual {
                    validate_visual_effects(&effects)?;
                }
                let mut rules = Vec::new();
                for direction in directions {
                    rules.extend(self.rules_from_alternatives(
                        alternatives.clone(),
                        direction,
                        true,
                        context.guards.clone(),
                        effects.core.clone(),
                        effects.ordered.clone(),
                        application,
                        role,
                    )?);
                }
                Ok(wrap_rewrite_steps(application, rules))
            }
        }
    }

    fn lower_effects(&self, effects: &[EffectAst]) -> Result<LoweredEffects, DiagnosticReport> {
        let mut lowered = LoweredEffects::default();
        self.lower_effects_into(effects, &mut lowered)?;
        Ok(lowered)
    }

    fn lower_effects_into(
        &self,
        effects: &[EffectAst],
        lowered: &mut LoweredEffects,
    ) -> Result<(), DiagnosticReport> {
        for effect in effects {
            match effect {
                EffectAst::Cancel => lowered.core.push(Effect::Cancel),
                EffectAst::Win => {
                    lowered.core.push(Effect::Win);
                    lowered.ordered.push(RuleEffect::Win);
                }
                EffectAst::Restart => {
                    lowered.core.push(Effect::Restart);
                    lowered.ordered.push(RuleEffect::Restart);
                }
                EffectAst::NextLevel => {
                    lowered.core.push(Effect::NextLevel);
                    lowered.ordered.push(RuleEffect::NextLevel);
                }
                EffectAst::Again => {
                    lowered.core.push(Effect::Again);
                    lowered.ordered.push(RuleEffect::Again);
                }
                EffectAst::Checkpoint => {
                    lowered.core.push(Effect::Checkpoint);
                    lowered.ordered.push(RuleEffect::Checkpoint);
                }
                EffectAst::ClearCheckpoint => {
                    lowered.core.push(Effect::ClearCheckpoint);
                    lowered.ordered.push(RuleEffect::ClearCheckpoint);
                }
                EffectAst::PlaySfx { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PlaySfx { name: name.clone() });
                }
                EffectAst::PlayMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PlayMusic { name: name.clone() });
                }
                EffectAst::PauseMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::PauseMusic { name: name.clone() });
                }
                EffectAst::ResumeMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::ResumeMusic { name: name.clone() });
                }
                EffectAst::StopMusic { name } => {
                    lowered
                        .ordered
                        .push(RuleEffect::StopMusic { name: name.clone() });
                }
                EffectAst::Wait { milliseconds } => {
                    let milliseconds = milliseconds.unwrap_or(self.default_wait_ms);
                    lowered.ordered.push(RuleEffect::Wait { milliseconds });
                }
                EffectAst::WaitAnimation => {
                    lowered.ordered.push(RuleEffect::WaitAnimation);
                }
                EffectAst::Message { text, literal } => {
                    lowered.ordered.push(RuleEffect::Message {
                        text: text.clone(),
                        literal: *literal,
                    });
                }
                EffectAst::Scene(effect) => {
                    lowered.ordered.push(RuleEffect::Scene(effect.clone()));
                }
                EffectAst::UpdateGlobal { name, op, value } => {
                    let global = *self.global_names.get(name).ok_or_else(|| {
                        DiagnosticReport::error(format!("unknown global in effect: {name}"))
                    })?;
                    if self.constant_globals.contains(&global) {
                        return Err(DiagnosticReport::error(format!(
                            "cannot update const: {name}"
                        )));
                    }
                    lowered.core.push(Effect::UpdateGlobal {
                        global,
                        op: *op,
                        value: *value,
                    });
                }
            }
        }
        Ok(())
    }

    fn rules_from_alternatives(
        &mut self,
        alternatives: Vec<RuleBodyAlternative>,
        direction: Direction,
        direction_expanded: bool,
        guards: Vec<Guard>,
        effects: Vec<Effect>,
        ordered_effects: Vec<RuleEffect>,
        application: RuleApplication,
        role: ClassifiedRuleRole,
    ) -> Result<Vec<RuleStep>, DiagnosticReport> {
        let mut rules = Vec::with_capacity(alternatives.len());
        for alternative in alternatives {
            let mut guards = guards.clone();
            guards.extend(alternative.guards.clone());
            let mut rule_effects = ordered_effects.clone();
            append_move_sound_effects(
                &alternative.components,
                &alternative.writes,
                self.model_sound_triggers,
                &mut rule_effects,
            );
            let mut rule_animations = Vec::new();
            if role != ClassifiedRuleRole::Visual {
                append_tween_rule_animations(
                    &alternative.writes,
                    self.animation,
                    &mut rule_animations,
                );
            }
            let compiled_components = alternative
                .components
                .iter()
                .map(|component| {
                    let cells = component
                        .cells
                        .iter()
                        .map(|cell| {
                            Ok(MatchCell {
                                offset: resolve_offset(
                                    cell.offset.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                require_objects: cell.require_objects.clone(),
                                require_object_sets: cell.require_object_sets.clone(),
                                forbid_objects: cell.forbid_objects.clone(),
                                require_scratch: resolve_scratch_patterns(
                                    cell.require_scratch.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                require_object_set_scratch: resolve_object_set_scratch_patterns(
                                    cell.require_object_set_scratch.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                forbid_scratch: resolve_scratch_patterns(
                                    cell.forbid_scratch.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                                forbid_object_set_scratch: resolve_object_set_scratch_patterns(
                                    cell.forbid_object_set_scratch.clone(),
                                    direction,
                                    direction_expanded,
                                    "statement",
                                )?,
                            })
                        })
                        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
                    Ok(PatternComponent {
                        cells,
                        gap_count: component.gap_count,
                    })
                })
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            let compiled_writes = alternative
                .writes
                .iter()
                .map(|write| resolve_write(write, direction, direction_expanded, "statement"))
                .collect::<Result<Vec<_>, DiagnosticReport>>()?;
            if role == ClassifiedRuleRole::Visual {
                validate_visual_writes(&compiled_writes, self.visual_objects)?;
            }

            let id = RuleId(self.next_rule_id);
            self.next_rule_id += 1;
            if role == ClassifiedRuleRole::Visual {
                self.visual_rules.push(id);
            }
            if !rule_animations.is_empty() {
                self.rule_animations.insert(id, rule_animations);
            }
            if !rule_effects.is_empty() {
                self.rule_effects.insert(id, rule_effects);
            }
            rules.push(RuleStep::Rule(Rule {
                id,
                guards,
                application,
                pattern: Pattern {
                    components: compiled_components,
                },
                writes: compiled_writes,
                effects: effects.clone(),
            }));
        }
        Ok(rules)
    }
}

fn wrap_rewrite_steps(application: RuleApplication, steps: Vec<RuleStep>) -> Vec<RuleStep> {
    if application == RuleApplication::UntilStable {
        vec![RuleStep::Block {
            application,
            stop_condition: None,
            steps,
        }]
    } else {
        steps
    }
}

fn validate_visual_effects(effects: &LoweredEffects) -> Result<(), DiagnosticReport> {
    if effects.core.is_empty() && effects.ordered.is_empty() {
        return Ok(());
    }
    Err(DiagnosticReport::error(
        "display block rewrites cannot use effects".to_string(),
    ))
}

fn validate_visual_writes(
    writes: &[WriteOp],
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    for write in writes {
        match write {
            WriteOp::Add { object, .. }
            | WriteOp::Remove { object, .. }
            | WriteOp::Move { object, .. } => {
                ensure_visual_write_object(*object, visual_objects)?;
            }
            WriteOp::AddObjectSet { .. }
            | WriteOp::RemoveObjectSet { .. }
            | WriteOp::MoveObjectSet { .. }
            | WriteOp::SetObjectSetScratch { .. }
            | WriteOp::RemoveObjectSetScratch { .. } => {}
            WriteOp::Replace { remove, add, .. } => {
                ensure_visual_write_object(*remove, visual_objects)?;
                ensure_visual_write_object(*add, visual_objects)?;
            }
            WriteOp::SetScratch { object, .. } | WriteOp::RemoveScratch { object, .. } => {
                if !object.is_empty() {
                    ensure_visual_write_object(*object, visual_objects)?;
                }
            }
        }
    }
    Ok(())
}

fn ensure_visual_write_object(
    object: ObjectId,
    visual_objects: &[ObjectId],
) -> Result<(), DiagnosticReport> {
    if visual_objects.contains(&object) {
        return Ok(());
    }
    Err(DiagnosticReport::error(
        "display block can read main objects but can only write display objects".to_string(),
    ))
}

#[derive(Clone, Debug, Default)]
struct RuleBodyAlternative {
    guards: Vec<Guard>,
    components: Vec<PatternComponentTemplate>,
    writes: Vec<WriteOpTemplate>,
}

fn append_move_sound_effects(
    components: &[PatternComponentTemplate],
    writes: &[WriteOpTemplate],
    triggers: &[ModelSoundTrigger],
    ordered_effects: &mut Vec<RuleEffect>,
) {
    if triggers.is_empty() {
        return;
    }
    for trigger in triggers {
        let moves_trigger_object = writes.iter().any(|write| {
            matches!(
                write,
                WriteOpTemplate::Move { object, .. } if trigger.objects.contains(object)
            ) || matches!(
                write,
                WriteOpTemplate::MoveObjectSet { objects, .. }
                    if objects.iter().any(|object| trigger.objects.contains(object))
            )
        });
        let matches_trigger = writes.iter().any(|write| match (trigger.kind, write) {
            (ModelSoundTriggerKind::Move, WriteOpTemplate::Move { object, .. }) => {
                trigger.objects.contains(object)
            }
            (ModelSoundTriggerKind::Move, WriteOpTemplate::MoveObjectSet { objects, .. }) => {
                objects
                    .iter()
                    .any(|object| trigger.objects.contains(object))
            }
            _ => false,
        });
        let matches_cantmove_intent = trigger.kind == ModelSoundTriggerKind::CantMove
            && !moves_trigger_object
            && cantmove_intent_is_consumed(components, writes, trigger);
        let matches_trigger = matches_trigger || matches_cantmove_intent;
        if !matches_trigger {
            continue;
        }
        let name = &trigger.sfx_name;
        if !ordered_effects.iter().any(
            |effect| matches!(effect, RuleEffect::PlaySfx { name: existing } if existing == name),
        ) {
            ordered_effects.push(RuleEffect::PlaySfx { name: name.clone() });
        }
    }
}

fn cantmove_intent_is_consumed(
    components: &[PatternComponentTemplate],
    writes: &[WriteOpTemplate],
    trigger: &ModelSoundTrigger,
) -> bool {
    writes.iter().any(|write| match write {
        WriteOpTemplate::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            ..
        } => {
            *scratch == ANONYMOUS_MOVEMENT_SCRATCH
                && trigger.objects.contains(object)
                && component_cell_has_object_movement_intent(
                    components, *component, offset, *object,
                )
        }
        WriteOpTemplate::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            ..
        } => {
            *scratch == ANONYMOUS_MOVEMENT_SCRATCH
                && component_cell_has_object_set_movement_intent(
                    components, *component, offset, *binding, trigger,
                )
        }
        _ => false,
    })
}

fn component_cell_has_object_movement_intent(
    components: &[PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
    object: ObjectId,
) -> bool {
    component_cell(components, component, offset).is_some_and(|cell| {
        cell.require_scratch.iter().any(|scratch| {
            scratch.scratch == ANONYMOUS_MOVEMENT_SCRATCH && scratch.object == object
        })
    })
}

fn component_cell_has_object_set_movement_intent(
    components: &[PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
    binding: u16,
    trigger: &ModelSoundTrigger,
) -> bool {
    component_cell(components, component, offset).is_some_and(|cell| {
        let binding_matches_trigger = cell.require_object_sets.iter().any(|object_set| {
            object_set.binding == binding
                && object_set
                    .objects
                    .iter()
                    .any(|object| trigger.objects.contains(object))
        });
        binding_matches_trigger
            && cell.require_object_set_scratch.iter().any(|scratch| {
                scratch.scratch == ANONYMOUS_MOVEMENT_SCRATCH && scratch.binding == binding
            })
    })
}

fn component_cell<'a>(
    components: &'a [PatternComponentTemplate],
    component: u16,
    offset: &OffsetTemplate,
) -> Option<&'a MatchCellTemplate> {
    components
        .get(component as usize)?
        .cells
        .iter()
        .find(|cell| cell.offset == *offset)
}

fn append_tween_rule_animations(
    writes: &[WriteOpTemplate],
    animation: &AnimationDef,
    animations: &mut Vec<RuleAnimation>,
) {
    if !animation.tween.enabled {
        return;
    }
    let mut objects = Vec::new();
    for write in writes {
        match write {
            WriteOpTemplate::Move { object, .. } => {
                if !objects.contains(object) {
                    objects.push(*object);
                }
            }
            WriteOpTemplate::MoveObjectSet {
                objects: moved_objects,
                ..
            } => {
                for object in moved_objects {
                    if !objects.contains(object) {
                        objects.push(*object);
                    }
                }
            }
            _ => {}
        }
    }
    if objects.is_empty() {
        return;
    }
    if animations.iter().any(|animation| {
        animation.trigger == RuleAnimationTrigger::Move
            && animation.name == "tween"
            && animation.objects == objects
    }) {
        return;
    }
    animations.push(RuleAnimation {
        trigger: RuleAnimationTrigger::Move,
        name: "tween".to_string(),
        objects,
    });
}

fn parse_inline_rewrite(
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<
    (
        PatternBlock,
        PatternBlock,
        Vec<EffectAst>,
        Vec<EffectAst>,
        Option<String>,
    ),
    DiagnosticReport,
> {
    let (before, after) = line
        .split_once("->")
        .ok_or_else(|| parse_error(line, "inline rewrite must contain ->"))?;
    let before = parse_pattern_side(
        before.trim(),
        object_names,
        object_schemas,
        value_sets,
        maps,
        object_groups,
        global_names,
        false,
    )?;
    let (after, effects, after_effects, after_call) = split_rewrite_suffix(after.trim(), line)?;
    let after = if after.is_empty() {
        before.clone()
    } else {
        let after = parse_pattern_side(
            after,
            object_names,
            object_schemas,
            value_sets,
            maps,
            object_groups,
            global_names,
            true,
        )?;
        normalize_rhs_keep_cells(&before, after, line)?
    };

    Ok((before, after, effects, after_effects, after_call))
}

fn split_rewrite_suffix<'a>(
    after: &'a str,
    line: &str,
) -> Result<(&'a str, Vec<EffectAst>, Vec<EffectAst>, Option<String>), DiagnosticReport> {
    let Some(last_block_end) = after.rfind(']') else {
        return parse_rewrite_effect(after, line).map(|effects| ("", effects, Vec::new(), None));
    };
    let pattern = after[..=last_block_end].trim();
    let suffix = after[last_block_end + 1..].trim();
    if suffix.is_empty() {
        return Ok((pattern, Vec::new(), Vec::new(), None));
    }

    let tokens = split_header_tokens(suffix);
    if matches!(tokens.as_slice(), [name] if is_qualified_identifier(name))
        && !is_builtin_rewrite_effect_text(suffix)
    {
        return Ok((pattern, Vec::new(), Vec::new(), Some(suffix.to_string())));
    }

    parse_rewrite_effect(suffix, line).map(|effects| (pattern, Vec::new(), effects, None))
}

fn normalize_rhs_keep_cells(
    before: &PatternBlock,
    mut after: PatternBlock,
    line: &str,
) -> Result<PatternBlock, DiagnosticReport> {
    if before.components.len() != after.components.len() {
        return Err(parse_error(
            line,
            "before and after sides must have the same number of blocks",
        ));
    }

    for (before_component, after_component) in before.components.iter().zip(&mut after.components) {
        if !block_shapes_match(before_component, after_component) {
            return Err(parse_error(
                line,
                "before and after blocks must have matching cell and ellipsis layout",
            ));
        }
        for (before_row, after_row) in before_component.rows.iter().zip(&mut after_component.rows) {
            for (before_part, after_part) in before_row.iter().zip(after_row) {
                let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) =
                    (before_part, after_part)
                else {
                    continue;
                };
                if after_cell.keep {
                    *after_cell = before_cell.clone();
                }
            }
        }
    }

    Ok(after)
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedRewriteEffect {
    pub(crate) surface: SurfaceRewriteEffect,
    pub(crate) semantic_tokens: Vec<semantic::SemanticToken>,
}

fn parse_rewrite_effect(suffix: &str, line: &str) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let parsed = parse_rewrite_effect_with_semantic_tokens(suffix, line)?;
    debug_assert!(
        parsed
            .semantic_tokens
            .iter()
            .all(|token| token.start < token.end)
    );
    Ok(parsed.surface.effects)
}

fn parse_rewrite_effect_with_semantic_tokens(
    suffix: &str,
    line: &str,
) -> Result<ParsedRewriteEffect, DiagnosticReport> {
    let surface = parse_surface_rewrite_effect(suffix, line)?;
    let semantic_tokens = project_surface_semantic_tokens(&surface.document.semantic_tokens);
    Ok(ParsedRewriteEffect {
        surface,
        semantic_tokens,
    })
}

fn parse_surface_rewrite_effect(
    suffix: &str,
    line: &str,
) -> Result<SurfaceRewriteEffect, DiagnosticReport> {
    let tokens = source_line_tokens(strip_line_comment(suffix), 0);
    let document = rewrite_effect_surface_document(&tokens);
    let effects = parse_rewrite_effect_value(suffix, line)?;
    Ok(SurfaceRewriteEffect { effects, document })
}

fn parse_rewrite_effect_value(
    suffix: &str,
    line: &str,
) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let suffix = suffix.trim();
    if suffix.strip_prefix("emit ").is_some() {
        return Err(parse_error(
            line,
            "`emit` is obsolete; write the presentation effect directly",
        ));
    }
    if let Some(text) = suffix.strip_prefix("message ") {
        let text = text.trim();
        if let Some(text) = parse_quoted_text(text) {
            return Ok(vec![EffectAst::Message {
                text,
                literal: true,
            }]);
        }
        if parse_view_path(text).is_some() {
            return Ok(vec![EffectAst::Message {
                text: text.to_string(),
                literal: false,
            }]);
        }
        return Err(parse_error(
            line,
            "message effect must be: message \"text\" or message <path>",
        ));
    }

    let tokens = split_header_tokens(suffix);
    match tokens.as_slice() {
        [command] if command.eq_ignore_ascii_case("cancel") => Ok(vec![EffectAst::Cancel]),
        [command] if command.eq_ignore_ascii_case("win") => Ok(vec![EffectAst::Win]),
        [command] if command.eq_ignore_ascii_case("restart") => Ok(vec![EffectAst::Restart]),
        [command] if command.eq_ignore_ascii_case("next_level") => Ok(vec![EffectAst::NextLevel]),
        [command] if command.eq_ignore_ascii_case("again") => Ok(vec![EffectAst::Again]),
        [command] if command.eq_ignore_ascii_case("checkpoint") => Ok(vec![EffectAst::Checkpoint]),
        [command] if command.eq_ignore_ascii_case("clear_checkpoint") => {
            Ok(vec![EffectAst::ClearCheckpoint])
        }
        ["goto", ..] | ["start", ..] => Ok(vec![EffectAst::Scene(parse_puzzle_scene_effect(
            suffix, line,
        )?)]),
        tokens
            if tokens.len() > 2
                && tokens
                    .iter()
                    .any(|token| is_rewrite_effect_command_token(token)) =>
        {
            parse_simple_rewrite_effects(tokens, line)
        }
        ["wait"] => Ok(vec![EffectAst::Wait { milliseconds: None }]),
        ["wait", "animation"] | ["wait", "tween"] => Ok(vec![EffectAst::WaitAnimation]),
        ["wait", duration] => Ok(vec![EffectAst::Wait {
            milliseconds: Some(parse_wait_duration_ms(duration, line)?),
        }]),
        ["sfx", name] => {
            validate_qualified_identifier(name, line, "sfx sounds name")?;
            Ok(vec![EffectAst::PlaySfx {
                name: (*name).to_string(),
            }])
        }
        ["play_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::PlayMusic {
                name: (*name).to_string(),
            }])
        }
        ["pause_music"] => Ok(vec![EffectAst::PauseMusic { name: None }]),
        ["pause_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::PauseMusic {
                name: Some((*name).to_string()),
            }])
        }
        ["resume_music"] => Ok(vec![EffectAst::ResumeMusic { name: None }]),
        ["resume_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::ResumeMusic {
                name: Some((*name).to_string()),
            }])
        }
        ["stop_music"] => Ok(vec![EffectAst::StopMusic { name: None }]),
        ["stop_music", name] => {
            validate_qualified_identifier(name, line, "music sounds name")?;
            Ok(vec![EffectAst::StopMusic {
                name: Some((*name).to_string()),
            }])
        }
        ["set", name, op, value] if is_global_update_operator(op) => {
            Ok(vec![EffectAst::UpdateGlobal {
                name: (*name).to_string(),
                op: parse_global_update_op(op, line)?,
                value: parse_global_value(value, line)?,
            }])
        }
        [name, op, value] if is_global_update_operator(op) => Ok(vec![EffectAst::UpdateGlobal {
            name: (*name).to_string(),
            op: parse_global_update_op(op, line)?,
            value: parse_global_value(value, line)?,
        }]),
        _ => Err(parse_error(
            line,
            "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, set <global> <op> <value>, or <global> <op> <value> without set",
        )),
    }
}

fn parse_puzzle_scene_effect(value: &str, line: &str) -> Result<SceneEffect, DiagnosticReport> {
    let effect = parse_scene_effect(value, line)?;
    validate_puzzle_scene_effect(&effect, line)?;
    Ok(effect)
}

fn validate_puzzle_scene_effect(effect: &SceneEffect, line: &str) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::Goto { .. } | SceneEffect::Reset { .. } => Ok(()),
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                validate_puzzle_scene_effect(effect, line)?;
            }
            Ok(())
        }
        _ => Err(parse_error(
            line,
            "puzzle statement scene effects are limited to `goto <scene>` and `start <scene>`",
        )),
    }
}

fn parse_simple_rewrite_effects(
    tokens: &[&str],
    line: &str,
) -> Result<Vec<EffectAst>, DiagnosticReport> {
    let mut effects = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].to_ascii_lowercase().as_str() {
            "cancel" => {
                effects.push(EffectAst::Cancel);
                index += 1;
            }
            "win" => {
                effects.push(EffectAst::Win);
                index += 1;
            }
            "restart" => {
                effects.push(EffectAst::Restart);
                index += 1;
            }
            "next_level" => {
                effects.push(EffectAst::NextLevel);
                index += 1;
            }
            "again" => {
                effects.push(EffectAst::Again);
                index += 1;
            }
            "checkpoint" => {
                effects.push(EffectAst::Checkpoint);
                index += 1;
            }
            "clear_checkpoint" => {
                effects.push(EffectAst::ClearCheckpoint);
                index += 1;
            }
            "wait" => {
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("animation") || token.eq_ignore_ascii_case("tween")
                }) {
                    effects.push(EffectAst::WaitAnimation);
                    index += 2;
                } else if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    effects.push(EffectAst::Wait {
                        milliseconds: Some(parse_wait_duration_ms(tokens[index + 1], line)?),
                    });
                    index += 2;
                } else {
                    effects.push(EffectAst::Wait { milliseconds: None });
                    index += 1;
                }
            }
            "sfx" => {
                let Some(name) = tokens.get(index + 1) else {
                    return Err(parse_error(line, "sfx effect must include a name"));
                };
                validate_qualified_identifier(name, line, "sfx sounds name")?;
                effects.push(EffectAst::PlaySfx {
                    name: (*name).to_string(),
                });
                index += 2;
            }
            "play_music" => {
                let Some(name) = tokens.get(index + 1) else {
                    return Err(parse_error(line, "play_music effect must include a name"));
                };
                validate_qualified_identifier(name, line, "music sounds name")?;
                effects.push(EffectAst::PlayMusic {
                    name: (*name).to_string(),
                });
                index += 2;
            }
            "pause_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::PauseMusic { name });
            }
            "resume_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::ResumeMusic { name });
            }
            "stop_music" => {
                let name = if index + 1 < tokens.len()
                    && !is_rewrite_effect_command_token(tokens[index + 1])
                {
                    validate_qualified_identifier(tokens[index + 1], line, "music sounds name")?;
                    index += 2;
                    Some(tokens[index - 1].to_string())
                } else {
                    index += 1;
                    None
                };
                effects.push(EffectAst::StopMusic { name });
            }
            "set" => {
                let (Some(name), Some(op), Some(value)) = (
                    tokens.get(index + 1),
                    tokens.get(index + 2),
                    tokens.get(index + 3),
                ) else {
                    return Err(parse_error(
                        line,
                        "set effect must be: set <global> <op> <value>",
                    ));
                };
                if !is_global_update_operator(op) {
                    return Err(parse_error(
                        line,
                        "set effect must be: set <global> <op> <value>",
                    ));
                }
                effects.push(EffectAst::UpdateGlobal {
                    name: (*name).to_string(),
                    op: parse_global_update_op(op, line)?,
                    value: parse_global_value(value, line)?,
                });
                index += 4;
            }
            name if index + 2 < tokens.len() && is_global_update_operator(tokens[index + 1]) => {
                effects.push(EffectAst::UpdateGlobal {
                    name: name.to_string(),
                    op: parse_global_update_op(tokens[index + 1], line)?,
                    value: parse_global_value(tokens[index + 2], line)?,
                });
                index += 3;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "rewrite effect must be: cancel, win, restart, next_level, again, checkpoint, clear_checkpoint, sfx <name>, play_music <name>, pause_music [name], resume_music [name], stop_music [name], wait [duration], message <text>, set <global> <op> <value>, or <global> <op> <value> without set",
                ));
            }
        }
    }
    Ok(effects)
}

fn is_rewrite_effect_command_token(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "cancel"
            | "win"
            | "restart"
            | "next_level"
            | "again"
            | "checkpoint"
            | "clear_checkpoint"
            | "wait"
            | "sfx"
            | "play_music"
            | "pause_music"
            | "resume_music"
            | "stop_music"
            | "set"
    )
}

fn is_builtin_rewrite_effect_text(suffix: &str) -> bool {
    if suffix.strip_prefix("message ").is_some() || suffix.strip_prefix("emit ").is_some() {
        return true;
    }
    let tokens = split_header_tokens(suffix);
    matches!(
        tokens.as_slice(),
        [command] if command.eq_ignore_ascii_case("cancel") || command.eq_ignore_ascii_case("win") || command.eq_ignore_ascii_case("restart") || command.eq_ignore_ascii_case("next_level") || command.eq_ignore_ascii_case("again") || command.eq_ignore_ascii_case("checkpoint") || command.eq_ignore_ascii_case("clear_checkpoint")
    ) || matches!(tokens.as_slice(), ["goto", ..] | ["start", ..])
        || matches!(
            tokens.as_slice(),
            ["sfx", _]
                | ["play_music", _]
                | ["pause_music"]
                | ["pause_music", _]
                | ["resume_music"]
                | ["resume_music", _]
                | ["stop_music"]
                | ["stop_music", _]
                | ["wait"]
                | ["wait", _]
        )
        || matches!(tokens.as_slice(), [_, op, _] if is_global_update_operator(op))
        || matches!(tokens.as_slice(), ["set", _, op, _] if is_global_update_operator(op))
}

fn is_global_update_operator(op: &str) -> bool {
    matches!(op, "=" | "+=" | "-=" | "*=" | "/=" | "%=")
}

fn parse_global_update_op(op: &str, line: &str) -> Result<GlobalUpdateOp, DiagnosticReport> {
    match op {
        "=" => Ok(GlobalUpdateOp::Set),
        "+=" => Ok(GlobalUpdateOp::Add),
        "-=" => Ok(GlobalUpdateOp::Subtract),
        "*=" => Ok(GlobalUpdateOp::Multiply),
        "/=" => Ok(GlobalUpdateOp::Divide),
        "%=" => Ok(GlobalUpdateOp::Remainder),
        _ => Err(parse_error(line, "unknown global update operator")),
    }
}

fn neutral_direction() -> Direction {
    Direction {
        input: InputId(0),
        dx: 1,
        dy: 0,
    }
}

fn rewrite_requires_implicit_cardinal_expansion(rewrite: &OrientedRewriteAst) -> bool {
    pattern_block_requires_implicit_cardinal_expansion(&rewrite.before)
        || pattern_block_requires_implicit_cardinal_expansion(&rewrite.after)
}

fn pattern_block_requires_implicit_cardinal_expansion(block: &PatternBlock) -> bool {
    block.components.iter().any(|component| {
        component.rows.len() > 1
            || component.rows.iter().any(|row| {
                row.len() > 1
                    || row.iter().any(|part| match part {
                        BlockPart::Cell(cell) => block_cell_has_relative_direction(cell),
                        BlockPart::Ellipsis => true,
                    })
            })
    })
}

fn block_cell_has_relative_direction(cell: &BlockCell) -> bool {
    cell.require
        .iter()
        .chain(&cell.forbid)
        .any(selector_has_relative_direction)
}

fn selector_has_relative_direction(selector: &ObjectSelector) -> bool {
    selector.scratch.iter().any(|scratch| {
        scratch.value.as_deref().is_some_and(|value| {
            parse_relative_direction_value(value).is_some()
                || movement_scratch_set_values(value).is_some()
        })
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OffsetTemplate {
    oriented_x: i16,
    oriented_y: i16,
    gap_terms: Vec<u16>,
}

#[derive(Clone, Debug)]
struct PatternComponentTemplate {
    cells: Vec<MatchCellTemplate>,
    gap_count: u16,
}

#[derive(Clone, Debug)]
struct MatchCellTemplate {
    offset: OffsetTemplate,
    require_objects: Vec<ObjectId>,
    require_object_sets: Vec<ObjectSetMatcher>,
    forbid_objects: Vec<ObjectId>,
    require_scratch: Vec<ScratchPatternTemplate>,
    require_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
    forbid_scratch: Vec<ScratchPatternTemplate>,
    forbid_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScratchPatternTemplate {
    object: ObjectId,
    scratch: ScratchId,
    value: Option<ScratchValueTemplate>,
    match_value: ScratchValueMatch,
    is_marker: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjectSetScratchPatternTemplate {
    binding: u16,
    scratch: ScratchId,
    value: Option<ScratchValueTemplate>,
    match_value: ScratchValueMatch,
    is_marker: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScratchValueTemplate {
    Literal(i64),
    Relative(RelativeDirection),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelativeDirection {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Clone, Debug)]
enum WriteOpTemplate {
    Add {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
    },
    AddObjectSet {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        objects: Vec<ObjectId>,
    },
    Remove {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
    },
    RemoveObjectSet {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        objects: Vec<ObjectId>,
    },
    Move {
        component: u16,
        from_offset: OffsetTemplate,
        to_offset: OffsetTemplate,
        object: ObjectId,
    },
    MoveObjectSet {
        component: u16,
        from_offset: OffsetTemplate,
        to_offset: OffsetTemplate,
        binding: u16,
        objects: Vec<ObjectId>,
    },
    SetScratch {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
    },
    SetObjectSetScratch {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
    },
    RemoveScratch {
        component: u16,
        offset: OffsetTemplate,
        object: ObjectId,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
        match_value: ScratchValueMatch,
    },
    RemoveObjectSetScratch {
        component: u16,
        offset: OffsetTemplate,
        binding: u16,
        scratch: ScratchId,
        value: Option<ScratchValueTemplate>,
        match_value: ScratchValueMatch,
    },
}

#[derive(Clone, Debug)]
struct PatternBlock {
    components: Vec<BlockComponent>,
}

#[derive(Clone, Debug)]
struct BlockComponent {
    rows: Vec<Vec<BlockPart>>,
}

#[derive(Clone, Debug)]
enum BlockPart {
    Cell(BlockCell),
    Ellipsis,
}

#[derive(Clone, Debug, Default)]
struct BlockCell {
    keep: bool,
    require: Vec<ObjectSelector>,
    forbid: Vec<ObjectSelector>,
    require_cell_scratch: Vec<ParsedScratch>,
    forbid_cell_scratch: Vec<ParsedScratch>,
}

#[derive(Clone, Debug)]
struct ObjectSelector {
    token: String,
    alternatives: Vec<ObjectId>,
    transform: Option<SelectorTransform>,
    family_wildcard: Option<FamilyWildcardSelector>,
    dynamic_guards: HashMap<ObjectId, Vec<DynamicSelectorGuard>>,
    scratch: Vec<ParsedScratch>,
    occurrence_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DynamicSelectorGuard {
    name: String,
    global: GlobalId,
    value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct OccurrenceKey {
    token: String,
    ordinal: usize,
}

#[derive(Clone, Debug)]
struct ResolvedObjectOccurrence {
    token: String,
    matched: ResolvedObjectMatch,
    key: Option<OccurrenceKey>,
    from_multi_selector: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedObjectMatch {
    Object(ObjectId),
    ObjectSet {
        binding: u16,
        layer: LayerId,
        objects: Vec<ObjectId>,
    },
}

impl ResolvedObjectMatch {
    fn possible_objects(&self) -> Vec<ObjectId> {
        match self {
            ResolvedObjectMatch::Object(object) => vec![*object],
            ResolvedObjectMatch::ObjectSet { objects, .. } => objects.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct OccurrencePlacement {
    component: u16,
    offset: OffsetTemplate,
    matched: ResolvedObjectMatch,
    require_scratch: Vec<ScratchPatternTemplate>,
    require_object_set_scratch: Vec<ObjectSetScratchPatternTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedScratch {
    name: String,
    value: Option<String>,
    negated: bool,
    anonymous: Option<AnonymousScratch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AnonymousScratch {
    Movement,
    Bool,
    Int,
}

impl ParsedScratch {
    fn named(name: &str, value: Option<&str>, negated: bool) -> Self {
        Self {
            name: name.to_string(),
            value: value.map(str::to_string),
            negated,
            anonymous: None,
        }
    }

    fn anonymous(kind: AnonymousScratch, value: &str, negated: bool) -> Self {
        Self {
            name: String::new(),
            value: Some(value.to_string()),
            negated,
            anonymous: Some(kind),
        }
    }
}

#[derive(Clone, Debug)]
struct SelectorTransform {
    source_token: String,
    mapped_objects: HashMap<ObjectId, ObjectId>,
}

#[derive(Clone, Debug)]
struct FamilyWildcardSelector {
    mapped_objects: HashMap<ObjectId, ObjectId>,
}

fn parse_pattern_side(
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<PatternBlock, DiagnosticReport> {
    let mut components = Vec::new();
    let mut rest = line.trim();

    while !rest.is_empty() {
        let Some(inner_start) = rest.strip_prefix('[') else {
            return Err(parse_error(
                line,
                "pattern side must contain bracketed blocks",
            ));
        };
        let Some(close_index) = inner_start.find(']') else {
            return Err(parse_error(line, "pattern block missing ]"));
        };
        let inner = &inner_start[..close_index];
        components.push(BlockComponent {
            rows: parse_block_rows(
                inner,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
                allow_keep_marker,
            )?,
        });
        rest = inner_start[close_index + 1..].trim_start();
    }

    if components.is_empty() {
        return Err(parse_error(
            line,
            "pattern side must contain at least one block",
        ));
    }

    Ok(PatternBlock { components })
}

fn parse_block_rows(
    inner: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<Vec<Vec<BlockPart>>, DiagnosticReport> {
    let rows = inner
        .split(';')
        .map(str::trim)
        .map(|row| {
            parse_block_parts(
                row,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
                allow_keep_marker,
            )
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    if rows.is_empty() {
        return Err(parse_error(
            line,
            "pattern block must contain at least one row",
        ));
    }
    validate_rectangular_ellipsis_layout(&rows, line)?;

    Ok(rows)
}

fn validate_rectangular_ellipsis_layout(
    rows: &[Vec<BlockPart>],
    line: &str,
) -> Result<(), DiagnosticReport> {
    if rows.len() <= 1
        || !rows
            .iter()
            .flatten()
            .any(|part| matches!(part, BlockPart::Ellipsis))
    {
        return Ok(());
    }

    let first = rows
        .first()
        .expect("parse_block_rows already rejected empty blocks");
    for row in rows.iter().skip(1) {
        let same_ellipsis_columns = row.len() == first.len()
            && row.iter().zip(first).all(|(left, right)| {
                matches!(
                    (left, right),
                    (BlockPart::Ellipsis, BlockPart::Ellipsis)
                        | (BlockPart::Cell(_), BlockPart::Cell(_))
                )
            });
        if !same_ellipsis_columns {
            return Err(parse_error(
                line,
                "ellipsis inside rectangular blocks requires each row to use the same ellipsis columns",
            ));
        }
    }

    Ok(())
}

fn parse_block_parts(
    inner: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<Vec<BlockPart>, DiagnosticReport> {
    let parts = inner
        .split('|')
        .map(str::trim)
        .map(|cell| {
            if cell == "..." {
                Ok(BlockPart::Ellipsis)
            } else {
                Ok(BlockPart::Cell(parse_block_cell(
                    cell,
                    line,
                    object_names,
                    object_schemas,
                    value_sets,
                    maps,
                    object_groups,
                    global_names,
                    allow_keep_marker,
                )?))
            }
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    if parts.is_empty() {
        return Err(parse_error(
            line,
            "pattern block must contain at least one cell",
        ));
    }

    Ok(parts)
}

fn parse_block_cell(
    cell: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
    allow_keep_marker: bool,
) -> Result<BlockCell, DiagnosticReport> {
    let mut parsed = BlockCell::default();
    let cell_tokens = split_cell_tokens(cell, line)?;
    if cell_tokens.iter().any(|token| token == "=") {
        if !allow_keep_marker {
            return Err(parse_error(line, "`=` is only valid as a RHS cell"));
        }
        if cell_tokens.len() != 1 {
            return Err(parse_error(
                line,
                "`=` RHS cell cannot contain other tokens",
            ));
        }
        parsed.keep = true;
        return Ok(parsed);
    }
    let mut tokens = cell_tokens.iter().map(String::as_str).peekable();
    while let Some(token) = tokens.next() {
        if token == "display" {
            let selector = display_selector_token(&mut tokens, line)?;
            parsed.require.push(resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?);
            continue;
        }
        if let Some(scratch) = parse_cell_scratch_token(token, line)? {
            parsed.require_cell_scratch.extend(scratch);
            continue;
        }
        if let Some(anonymous) = anonymous_scratch_for_token(token) {
            let selector = tokens
                .next()
                .ok_or_else(|| parse_error(line, "scratch sugar must be followed by a selector"))?;
            let selector = if selector == "display" {
                display_selector_token(&mut tokens, line)?
            } else {
                selector
            };
            if selector == "no" || anonymous_scratch_for_token(selector).is_some() {
                return Err(parse_error(
                    line,
                    "scratch sugar must be followed by a selector",
                ));
            }
            let mut selector = resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?;
            selector
                .scratch
                .push(ParsedScratch::anonymous(anonymous, token, false));
            parsed.require.push(selector);
            continue;
        }
        if token == "no" {
            let selector = tokens
                .next()
                .ok_or_else(|| parse_error(line, "`no` must be followed by a selector"))?;
            let selector = if selector == "display" {
                display_selector_token(&mut tokens, line)?
            } else {
                selector
            };
            if selector == "no" {
                return Err(parse_error(line, "`no no` is not a valid cell pattern"));
            }
            if let Some(scratch) = parse_cell_scratch_token(selector, line)? {
                parsed.forbid_cell_scratch.extend(scratch);
                continue;
            }
            parsed.forbid.push(resolve_object_selector(
                selector,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?);
        } else {
            parsed.require.push(resolve_object_selector(
                token,
                line,
                object_names,
                object_schemas,
                value_sets,
                maps,
                object_groups,
                global_names,
            )?);
        }
    }

    Ok(parsed)
}

fn display_selector_token<'a, I>(
    tokens: &mut std::iter::Peekable<I>,
    line: &str,
) -> Result<&'a str, DiagnosticReport>
where
    I: Iterator<Item = &'a str>,
{
    let selector = tokens
        .next()
        .ok_or_else(|| parse_error(line, "`display` must be followed by a display object"))?;
    if !is_display_role_token(selector) {
        return Err(parse_error(line, "display object must use an @ name"));
    }
    Ok(selector)
}

fn parse_cell_scratch_token(
    token: &str,
    line: &str,
) -> Result<Option<Vec<ParsedScratch>>, DiagnosticReport> {
    let Some(inner) = token
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return Ok(None);
    };
    Ok(Some(parse_selector_scratch(inner, line)?))
}

fn anonymous_scratch_for_token(token: &str) -> Option<AnonymousScratch> {
    match puzzle_authoring::scratch_sugar_kind(token)? {
        puzzle_authoring::ScratchSugarKind::Movement => Some(AnonymousScratch::Movement),
        puzzle_authoring::ScratchSugarKind::Bool => Some(AnonymousScratch::Bool),
        puzzle_authoring::ScratchSugarKind::Int => Some(AnonymousScratch::Int),
    }
}

fn split_cell_tokens(cell: &str, line: &str) -> Result<Vec<String>, DiagnosticReport> {
    puzzle_authoring::split_cell_tokens(cell).map_err(|error| match error {
        puzzle_authoring::CellTokenError::UnmatchedCloseBrace => {
            parse_error(line, "scratch block has unmatched }")
        }
        puzzle_authoring::CellTokenError::MissingCloseBrace => {
            parse_error(line, "scratch block missing }")
        }
    })
}

fn resolve_object_selector(
    selector: &str,
    line: &str,
    object_names: &HashMap<String, ObjectId>,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    object_groups: &HashMap<String, Vec<ObjectId>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    let (selector, scratch) = split_selector_scratch(selector, line)?;
    let (selector, occurrence_label) = split_selector_occurrence_label(selector, line)?;
    let token = labeled_selector_token(selector, occurrence_label.as_deref());
    let selector_base = selector.split_once(':').map_or(selector, |(base, _)| base);
    if !object_schemas.contains_key(selector_base) {
        if let Some(object) = object_names.get(selector).copied() {
            return Ok(ObjectSelector {
                token,
                alternatives: vec![object],
                transform: None,
                family_wildcard: None,
                dynamic_guards: HashMap::new(),
                scratch,
                occurrence_label,
            });
        }
    }

    if let Some(objects) = object_groups.get(selector) {
        return Ok(ObjectSelector {
            token,
            alternatives: objects.clone(),
            transform: None,
            family_wildcard: None,
            dynamic_guards: HashMap::new(),
            scratch,
            occurrence_label,
        });
    }

    let parts = selector.split(':').collect::<Vec<_>>();
    if parts.first().copied() == Some("*") {
        return resolve_schema_family_wildcard_selector(
            &parts,
            token,
            scratch,
            occurrence_label,
            line,
            object_schemas,
            value_sets,
            global_names,
        );
    }
    let Some(schema) = object_schemas.get(parts[0]) else {
        return Err(parse_error(line, "unknown object selector"));
    };

    validate_schema_selector_arity(&parts, schema, line, "object selector")?;
    if parts.len() == 1 {
        return Err(parse_error(
            line,
            "object selector for variants must use :* or explicit variant tags",
        ));
    }

    let mut source_token_parts = Vec::new();
    let constraints = schema
        .axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let Some(value) = schema_selector_part(&parts, schema, index) else {
                source_token_parts.push(axis.clone());
                return Ok(None);
            };
            if value == "*" {
                source_token_parts.push("*".to_string());
                return Ok(None);
            }
            let expr = parse_value_expr(value, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                if global_names.contains_key(axis) {
                    return Err(ambiguous_selector_tag_error(axis, parts[0], axis, line));
                }
                source_token_parts.push(axis.clone());
                Ok(None)
            } else if let ValueExpr::MapCall { arg, .. } = &expr {
                if arg != axis {
                    return Err(parse_error(
                        line,
                        "map argument must match selector tag set",
                    ));
                }
                let ValueExpr::MapCall { name, .. } = &expr else {
                    unreachable!("map call branch only handles map calls");
                };
                let map = maps
                    .get(name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                if map.axis != *axis {
                    return Err(parse_error(line, "map tag set must match argument tag set"));
                }
                source_token_parts.push(axis.clone());
                Ok(Some(SelectorConstraint::Mapped {
                    axis_index: index,
                    expr,
                }))
            } else if let ValueExpr::Binding(name) = &expr {
                let axis_values = schema_axis_values(schema, index)?;
                let names_axis_value = axis_values.contains(name);
                let names_value_set = value_sets.contains_key(name);
                let global = global_names.get(name).copied();
                if (names_axis_value && names_value_set)
                    || (global.is_some() && (names_axis_value || names_value_set))
                {
                    return Err(ambiguous_selector_tag_error(name, parts[0], axis, line));
                }
                if let Some(values) = value_sets.get(name) {
                    validate_selector_subset(name, values, &axis_values, parts[0], axis, line)?;
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::ValueSet(values.clone())))
                } else if names_axis_value {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::Fixed(name.clone())))
                } else if let Some(global) = global {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::DynamicGlobal {
                        axis_index: index,
                        name: name.clone(),
                        global,
                    }))
                } else {
                    source_token_parts.push(name.clone());
                    Ok(Some(SelectorConstraint::Fixed(name.clone())))
                }
            } else {
                source_token_parts.push((*value).to_string());
                Ok(Some(SelectorConstraint::Fixed((*value).to_string())))
            }
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    let alternatives = schema
        .variants
        .iter()
        .filter(|variant| {
            constraints
                .iter()
                .enumerate()
                .all(|(index, constraint)| match constraint {
                    Some(SelectorConstraint::Fixed(value)) => variant.values[index] == *value,
                    Some(SelectorConstraint::ValueSet(values)) => {
                        values.contains(&variant.values[index])
                    }
                    Some(SelectorConstraint::Mapped { .. })
                    | Some(SelectorConstraint::DynamicGlobal { .. })
                    | None => true,
                })
        })
        .map(|variant| variant.object)
        .collect::<Vec<_>>();

    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }

    if constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::Mapped { .. })))
    {
        let source_token = labeled_selector_token(
            &format!("{}:{}", parts[0], source_token_parts.join(":")),
            occurrence_label.as_deref(),
        );
        let mut mapped_objects = HashMap::new();
        let mut target_objects = Vec::new();
        for source in &schema.variants {
            let mut values = source.values.clone();
            for constraint in constraints.iter().flatten() {
                if let SelectorConstraint::Mapped { axis_index, expr } = constraint {
                    let axis = &schema.axes[*axis_index];
                    let mut env = ValueEnv::default();
                    env.bind(axis, axis, &source.values[*axis_index]);
                    values[*axis_index] = eval_bound_value_expr(expr, &env, maps, line)?;
                }
            }
            let target = schema
                .variants
                .iter()
                .find(|variant| variant.values == values)
                .ok_or_else(|| parse_error(line, "mapped selector target not found"))?
                .object;
            mapped_objects.insert(source.object, target);
            if !target_objects.contains(&target) {
                target_objects.push(target);
            }
        }
        return Ok(ObjectSelector {
            token,
            alternatives: target_objects,
            transform: Some(SelectorTransform {
                source_token,
                mapped_objects,
            }),
            family_wildcard: None,
            dynamic_guards: HashMap::new(),
            scratch,
            occurrence_label,
        });
    }

    let dynamic_guards = dynamic_selector_guards(&constraints, schema, line)?;
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard: None,
        dynamic_guards,
        scratch,
        occurrence_label,
    })
}

fn resolve_schema_family_wildcard_selector(
    parts: &[&str],
    token: String,
    scratch: Vec<ParsedScratch>,
    occurrence_label: Option<String>,
    line: &str,
    object_schemas: &HashMap<String, ObjectSchema>,
    value_sets: &HashMap<String, Vec<String>>,
    global_names: &HashMap<String, GlobalId>,
) -> Result<ObjectSelector, DiagnosticReport> {
    if parts.len() != 2 {
        return Err(parse_error(
            line,
            "family wildcard object selector must be *:<tag>",
        ));
    }
    let tag = parts[1];
    if tag == "_" {
        return Err(parse_error(
            line,
            "object selector wildcard must use *; _ is reserved for completion",
        ));
    }

    let (mut alternatives, family_wildcard) = if tag == "*" {
        (
            schema_wildcard_alternatives(object_schemas, |_, _| true),
            None,
        )
    } else {
        let expr = parse_value_expr(tag, line)?;
        let ValueExpr::Binding(name) = expr else {
            return Err(parse_error(
                line,
                "family wildcard object selector cannot use map calls",
            ));
        };
        let names_schema_tag = schema_wildcard_tag_value_exists(object_schemas, &name);
        let value_set = value_sets.get(&name);
        if global_names.contains_key(&name) && (names_schema_tag || value_set.is_some()) {
            return Err(parse_error(
                line,
                &format!(
                    "selector tag {name} is ambiguous for family wildcard selector: it is both a schema tag and a global"
                ),
            ));
        }
        if global_names.contains_key(&name) {
            return Err(parse_error(
                line,
                "family wildcard object selector cannot use dynamic var tags",
            ));
        }
        if let Some(values) = value_set {
            for value in values {
                if !schema_wildcard_tag_value_exists(object_schemas, value) {
                    return Err(parse_error(
                        line,
                        &format!(
                            "tag set {name} contains value {value} that is not used by any schema object"
                        ),
                    ));
                }
            }
            (
                schema_wildcard_alternatives(object_schemas, |_, variant| {
                    variant.values.iter().any(|value| values.contains(value))
                }),
                Some(FamilyWildcardSelector {
                    mapped_objects: schema_wildcard_target_set_map(object_schemas, values, line)?,
                }),
            )
        } else {
            (
                schema_wildcard_alternatives(object_schemas, |_, variant| {
                    variant.values.iter().any(|value| value == &name)
                }),
                Some(FamilyWildcardSelector {
                    mapped_objects: schema_wildcard_target_map(object_schemas, &name, line)?,
                }),
            )
        }
    };

    alternatives.sort_by_key(|object| object.0);
    alternatives.dedup();
    if alternatives.is_empty() {
        return Err(parse_error(line, "object selector matched no objects"));
    }
    Ok(ObjectSelector {
        token,
        alternatives,
        transform: None,
        family_wildcard,
        dynamic_guards: HashMap::new(),
        scratch,
        occurrence_label,
    })
}

fn schema_wildcard_alternatives(
    object_schemas: &HashMap<String, ObjectSchema>,
    matches: impl Fn(&ObjectSchema, &ObjectVariant) -> bool,
) -> Vec<ObjectId> {
    let mut alternatives = Vec::new();
    for schema in object_schemas.values() {
        for variant in &schema.variants {
            if matches(schema, variant) {
                alternatives.push(variant.object);
            }
        }
    }
    alternatives
}

fn schema_wildcard_tag_value_exists(
    object_schemas: &HashMap<String, ObjectSchema>,
    tag: &str,
) -> bool {
    object_schemas.values().any(|schema| {
        schema
            .variants
            .iter()
            .any(|variant| variant.values.iter().any(|value| value == tag))
    })
}

fn schema_wildcard_target_map(
    object_schemas: &HashMap<String, ObjectSchema>,
    target_tag: &str,
    line: &str,
) -> Result<HashMap<ObjectId, ObjectId>, DiagnosticReport> {
    schema_wildcard_target_set_map(object_schemas, &[target_tag.to_string()], line)
}

fn schema_wildcard_target_set_map(
    object_schemas: &HashMap<String, ObjectSchema>,
    target_tags: &[String],
    line: &str,
) -> Result<HashMap<ObjectId, ObjectId>, DiagnosticReport> {
    let mut mapped = HashMap::new();
    for schema in object_schemas.values() {
        for source in &schema.variants {
            let mut targets = Vec::new();
            for axis_index in 0..schema.axes.len() {
                let axis_values = schema_axis_values(schema, axis_index)?;
                let target_axis_values = target_tags
                    .iter()
                    .filter(|target_tag| axis_values.iter().any(|value| value == *target_tag))
                    .collect::<Vec<_>>();
                if target_axis_values.is_empty() {
                    continue;
                }
                for target_tag in target_axis_values {
                    let mut target_values = source.values.clone();
                    target_values[axis_index] = (*target_tag).clone();
                    let Some(target) = schema
                        .variants
                        .iter()
                        .find(|variant| variant.values == target_values)
                        .map(|variant| variant.object)
                    else {
                        continue;
                    };
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
            match targets.as_slice() {
                [] => {}
                [target] => {
                    mapped.insert(source.object, *target);
                }
                _ => {
                    return Err(parse_error(
                        line,
                        "family wildcard target tag is ambiguous for a source object",
                    ));
                }
            }
        }
    }
    Ok(mapped)
}

fn split_selector_occurrence_label<'a>(
    selector: &'a str,
    line: &str,
) -> Result<(&'a str, Option<String>), DiagnosticReport> {
    let Some((base, label)) = selector.split_once('#') else {
        return Ok((selector, None));
    };
    if base.is_empty() || label.is_empty() || label.contains('#') {
        return Err(parse_error(
            line,
            "selector occurrence label must be: selector#label",
        ));
    }
    if !label
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err(parse_error(
            line,
            "selector occurrence label may only contain letters, numbers, and _",
        ));
    }
    Ok((base, Some(label.to_string())))
}

fn labeled_selector_token(selector: &str, occurrence_label: Option<&str>) -> String {
    match occurrence_label {
        Some(label) => format!("{selector}#{label}"),
        None => selector.to_string(),
    }
}

fn split_selector_scratch<'a>(
    selector: &'a str,
    line: &str,
) -> Result<(&'a str, Vec<ParsedScratch>), DiagnosticReport> {
    let Some(open_index) = selector.find('{') else {
        return Ok((selector, Vec::new()));
    };
    let base = &selector[..open_index];
    let attrs = selector[open_index + 1..]
        .strip_suffix('}')
        .ok_or_else(|| parse_error(line, "scratch selector must end with }"))?;
    if base.is_empty() {
        return Err(parse_error(
            line,
            "scratch selector must attach to an object",
        ));
    }
    let attrs = parse_selector_scratch(attrs, line)?;
    Ok((base, attrs))
}

fn parse_selector_scratch(attrs: &str, line: &str) -> Result<Vec<ParsedScratch>, DiagnosticReport> {
    let mut parsed = Vec::new();
    let mut tokens = attrs.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        let (negated, spec) = if token == "no" {
            let spec = tokens
                .next()
                .ok_or_else(|| parse_error(line, "`no` must be followed by an scratch"))?;
            (true, spec)
        } else {
            (false, token)
        };
        if let Some(anonymous) = anonymous_scratch_for_token(spec) {
            parsed.push(ParsedScratch::anonymous(anonymous, spec, negated));
            continue;
        }
        let (name, value) = spec
            .split_once('=')
            .map_or((spec, None), |(name, value)| (name, Some(value)));
        validate_scratch_name(name, line)?;
        if value.is_some_and(str::is_empty) {
            return Err(parse_error(line, "scratch value must not be empty"));
        }
        parsed.push(ParsedScratch::named(name, value, negated));
    }
    Ok(parsed)
}

fn validate_scratch_name(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return Err(parse_error(
            line,
            "scratch name must start with an identifier and may use :value parts",
        ));
    };
    if !is_identifier(first) || !parts.all(is_scratch_name_value_atom) {
        return Err(parse_error(
            line,
            "scratch name must start with an identifier and may use :value parts",
        ));
    }
    Ok(())
}

fn is_scratch_name_value_atom(value: &str) -> bool {
    is_value_atom(value) || matches!(value, ">" | "<" | "^" | "v")
}

#[derive(Clone, Debug)]
enum SelectorConstraint {
    Fixed(String),
    ValueSet(Vec<String>),
    Mapped {
        axis_index: usize,
        expr: ValueExpr,
    },
    DynamicGlobal {
        axis_index: usize,
        name: String,
        global: GlobalId,
    },
}

fn dynamic_selector_guards(
    constraints: &[Option<SelectorConstraint>],
    schema: &ObjectSchema,
    line: &str,
) -> Result<HashMap<ObjectId, Vec<DynamicSelectorGuard>>, DiagnosticReport> {
    if !constraints
        .iter()
        .any(|constraint| matches!(constraint, Some(SelectorConstraint::DynamicGlobal { .. })))
    {
        return Ok(HashMap::new());
    }

    let mut guards = HashMap::<ObjectId, Vec<DynamicSelectorGuard>>::new();
    for variant in &schema.variants {
        let mut variant_guards = Vec::new();
        for constraint in constraints.iter().flatten() {
            let SelectorConstraint::DynamicGlobal {
                axis_index,
                name,
                global,
            } = constraint
            else {
                continue;
            };
            let value = variant.values.get(*axis_index).ok_or_else(|| {
                DiagnosticReport::error("internal schema variant missing tag value".to_string())
            })?;
            variant_guards.push(DynamicSelectorGuard {
                name: name.clone(),
                global: *global,
                value: parse_global_value(value, line).map_err(|_| {
                    parse_error(
                        line,
                        "dynamic selector tag slot values must be true, false, or integers",
                    )
                })?,
            });
        }
        guards.insert(variant.object, variant_guards);
    }
    Ok(guards)
}

fn validate_schema_selector_arity(
    parts: &[&str],
    schema: &ObjectSchema,
    line: &str,
    label: &str,
) -> Result<(), DiagnosticReport> {
    let slot_count = parts.len().saturating_sub(1);
    if parts.iter().skip(1).any(|part| *part == "_") {
        return Err(parse_error(
            line,
            &format!("{label} wildcard must use *; _ is reserved for completion"),
        ));
    }
    if slot_count > schema.axes.len() {
        return Err(parse_error(line, &format!("{label} has too many tags")));
    }
    if slot_count == 0 {
        return Ok(());
    }
    if slot_count == 1 && parts[1] == "*" {
        return Ok(());
    }
    if slot_count < schema.axes.len() {
        return Err(parse_error(
            line,
            &format!("{label} must name every variant slot; use * for unconstrained slots"),
        ));
    }
    Ok(())
}

fn schema_selector_part<'a>(
    parts: &'a [&str],
    schema: &ObjectSchema,
    axis_index: usize,
) -> Option<&'a str> {
    if parts.len() == 2 && parts[1] == "*" && schema.axes.len() > 1 {
        return Some("*");
    }
    parts.get(axis_index + 1).copied()
}

fn schema_axis_values(
    schema: &ObjectSchema,
    axis_index: usize,
) -> Result<Vec<String>, DiagnosticReport> {
    let mut values = Vec::new();
    for variant in &schema.variants {
        let value = variant.values.get(axis_index).ok_or_else(|| {
            DiagnosticReport::error("internal schema variant missing tag value".to_string())
        })?;
        if !values.contains(value) {
            values.push(value.clone());
        }
    }
    Ok(values)
}

fn validate_selector_subset(
    value_set_name: &str,
    values: &[String],
    axis_values: &[String],
    family: &str,
    axis: &str,
    line: &str,
) -> Result<(), DiagnosticReport> {
    for value in values {
        if !axis_values.contains(value) {
            return Err(parse_error(
                line,
                &format!(
                    "tag set {value_set_name} contains value {value}, which is not valid for {family} tag slot {axis}",
                ),
            ));
        }
    }
    Ok(())
}

fn ambiguous_selector_tag_error(
    tag: &str,
    family: &str,
    axis: &str,
    line: &str,
) -> DiagnosticReport {
    parse_error(
        line,
        &format!(
            "selector tag {tag} is ambiguous for {family} tag slot {axis}: it is both a concrete value and a tag set",
        ),
    )
}

fn parse_map_call(value: &str) -> Option<(&str, &str)> {
    let (name, rest) = value.split_once('(')?;
    let arg = rest.strip_suffix(')')?;
    Some((name, arg))
}

fn compile_before_after_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
    object_layers: &HashMap<ObjectId, LayerId>,
    scratch_names: &HashMap<String, ScratchDef>,
    _value_sets: &HashMap<String, Vec<String>>,
    line: &str,
) -> Result<Vec<RuleBodyAlternative>, DiagnosticReport> {
    if before.components.len() != after.components.len() {
        return Err(parse_error(
            line,
            "before and after sides must have the same number of blocks",
        ));
    }
    for (before_component, after_component) in before.components.iter().zip(&after.components) {
        if !block_shapes_match(before_component, after_component) {
            return Err(parse_error(
                line,
                "before and after blocks must have matching cell and ellipsis layout",
            ));
        }
    }
    let occupancy_objects = object_layers
        .iter()
        .filter_map(|(object, layer)| (layer.0 > 0).then_some(*object))
        .collect::<Vec<_>>();

    let expanded_blocks = expand_movement_scratch_sets(before, after);
    let mut alternatives = Vec::new();

    for (before, after) in expanded_blocks {
        let dynamic_blocks = expand_dynamic_selector_blocks(&before, &after);
        for (dynamic_guards, before, after) in dynamic_blocks {
            let before_occurrences = collect_before_occurrences(&before);
            reject_duplicate_labeled_occurrences(&before_occurrences, line)?;
            let mut assignments =
                expand_selector_assignments(&before_occurrences, object_layers, line)?;
            if before_occurrences
                .iter()
                .any(|occurrence| occurrence.occurrence_label.is_some())
            {
                assignments.reverse();
            }
            let before_by_token = before_occurrences_by_token(&before_occurrences);
            'assignment_loop: for assignment in assignments {
                let all_before_occurrences = &before_occurrences;
                let mut components = Vec::new();
                let mut writes = Vec::new();
                let mut before_token_counts = HashMap::<String, usize>::new();
                let mut after_token_counts = HashMap::<String, usize>::new();
                let mut before_placements = HashMap::<OccurrenceKey, OccurrencePlacement>::new();
                let mut after_placements = HashMap::<OccurrenceKey, OccurrencePlacement>::new();
                let mut duplicate_after_keys = HashSet::<OccurrenceKey>::new();

                for (component_index, (before_component, after_component)) in
                    before.components.iter().zip(&after.components).enumerate()
                {
                    let component_index = component_index as u16;
                    let mut component_cells = Vec::new();
                    let shared_gap_indices = rectangular_shared_gap_indices(before_component);
                    let mut next_gap_index = shared_gap_indices.as_ref().map_or(0, |indices| {
                        indices.iter().filter(|index| index.is_some()).count() as u16
                    });

                    for (y, (before_row, after_row)) in before_component
                        .rows
                        .iter()
                        .zip(&after_component.rows)
                        .enumerate()
                    {
                        let mut concrete_x = 0_i16;
                        let mut active_gaps = Vec::<u16>::new();

                        for (part_index, (before_part, after_part)) in
                            before_row.iter().zip(after_row).enumerate()
                        {
                            if matches!(before_part, BlockPart::Ellipsis) {
                                let gap_index = shared_gap_indices
                                    .as_ref()
                                    .and_then(|indices| indices.get(part_index).copied().flatten())
                                    .unwrap_or_else(|| {
                                        let gap_index = next_gap_index;
                                        next_gap_index += 1;
                                        gap_index
                                    });
                                active_gaps.push(gap_index);
                                continue;
                            }

                            let (BlockPart::Cell(before_cell), BlockPart::Cell(after_cell)) =
                                (before_part, after_part)
                            else {
                                unreachable!(
                                    "block_shapes_match already validated matching part kinds"
                                );
                            };
                            let offset = OffsetTemplate {
                                oriented_x: concrete_x,
                                oriented_y: y as i16,
                                gap_terms: active_gaps.clone(),
                            };
                            let before_occurrences = block_cell_object_occurrences(
                                before_cell,
                                &assignment,
                                all_before_occurrences,
                                &before_by_token,
                                &mut before_token_counts,
                                line,
                            )?;
                            let mut after_occurrences = block_cell_object_occurrences(
                                after_cell,
                                &assignment,
                                all_before_occurrences,
                                &before_by_token,
                                &mut after_token_counts,
                                line,
                            )?;
                            prefer_same_cell_occurrence_keys(
                                &before_occurrences,
                                &mut after_occurrences,
                            );
                            if !validate_same_layer_cell_occurrences(
                                &before_occurrences,
                                object_layers,
                                line,
                            )? {
                                continue 'assignment_loop;
                            }
                            if !validate_same_layer_cell_occurrences(
                                &after_occurrences,
                                object_layers,
                                line,
                            )? {
                                continue 'assignment_loop;
                            }
                            let mut before_objects =
                                possible_objects_for_occurrences(&before_occurrences);
                            let mut after_objects =
                                possible_objects_for_occurrences(&after_occurrences);
                            let before_scratch = block_cell_scratch(
                                before_cell,
                                &before_occurrences,
                                scratch_names,
                                line,
                            )?;
                            let after_scratch = block_cell_scratch(
                                after_cell,
                                &after_occurrences,
                                scratch_names,
                                line,
                            )?;
                            dedup_objects(&mut before_objects);
                            dedup_objects(&mut after_objects);
                            let require_objects =
                                concrete_objects_for_occurrences(&before_occurrences);
                            let require_object_sets =
                                object_sets_for_occurrences(&before_occurrences);

                            for occurrence in &before_occurrences {
                                if let Some(key) = &occurrence.key {
                                    before_placements.insert(
                                        key.clone(),
                                        OccurrencePlacement {
                                            component: component_index,
                                            offset: offset.clone(),
                                            matched: occurrence.matched.clone(),
                                            require_scratch: before_scratch
                                                .require
                                                .iter()
                                                .filter(|attr| {
                                                    occurrence
                                                        .matched
                                                        .possible_objects()
                                                        .contains(&attr.object)
                                                })
                                                .cloned()
                                                .collect(),
                                            require_object_set_scratch: before_scratch
                                                .require_object_set
                                                .iter()
                                                .filter(|attr| {
                                                    matches!(
                                                        &occurrence.matched,
                                                        ResolvedObjectMatch::ObjectSet {
                                                            binding,
                                                            ..
                                                        } if *binding == attr.binding
                                                    )
                                                })
                                                .cloned()
                                                .collect(),
                                        },
                                    );
                                }
                            }
                            for occurrence in &after_occurrences {
                                if let Some(key) = &occurrence.key {
                                    if duplicate_after_keys.contains(key) {
                                        continue;
                                    }
                                    let placement = OccurrencePlacement {
                                        component: component_index,
                                        offset: offset.clone(),
                                        matched: occurrence.matched.clone(),
                                        require_scratch: after_scratch
                                            .require
                                            .iter()
                                            .filter(|attr| {
                                                occurrence
                                                    .matched
                                                    .possible_objects()
                                                    .contains(&attr.object)
                                            })
                                            .cloned()
                                            .collect(),
                                        require_object_set_scratch: after_scratch
                                            .require_object_set
                                            .iter()
                                            .filter(|attr| {
                                                matches!(
                                                    &occurrence.matched,
                                                    ResolvedObjectMatch::ObjectSet {
                                                        binding,
                                                        ..
                                                    } if *binding == attr.binding
                                                )
                                            })
                                            .cloned()
                                            .collect(),
                                    };
                                    if after_placements.insert(key.clone(), placement).is_some() {
                                        after_placements.remove(key);
                                        duplicate_after_keys.insert(key.clone());
                                    }
                                }
                            }
                            let mut forbid_objects = block_cell_forbid_objects(before_cell);
                            forbid_objects.extend(implicit_layer_forbids(
                                &before_objects,
                                &after_objects,
                                object_layers,
                                &occupancy_objects,
                            ));
                            dedup_objects(&mut forbid_objects);

                            component_cells.push(MatchCellTemplate {
                                offset: offset.clone(),
                                require_objects,
                                require_object_sets,
                                forbid_objects,
                                require_scratch: before_scratch.require.clone(),
                                require_object_set_scratch: before_scratch
                                    .require_object_set
                                    .clone(),
                                forbid_scratch: before_scratch.forbid.clone(),
                                forbid_object_set_scratch: before_scratch.forbid_object_set.clone(),
                            });

                            let before_object_set_objects =
                                object_set_objects_for_occurrences(&before_occurrences);
                            let after_object_set_objects =
                                object_set_objects_for_occurrences(&after_occurrences);

                            for object in before_objects.iter().filter(|object| {
                                !after_objects.contains(object)
                                    && !before_object_set_objects.contains(object)
                            }) {
                                writes.push(WriteOpTemplate::Remove {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: *object,
                                });
                            }

                            for object in after_objects.iter().filter(|object| {
                                !before_objects.contains(object)
                                    && !after_object_set_objects.contains(object)
                            }) {
                                writes.push(WriteOpTemplate::Add {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: *object,
                                });
                            }
                            append_object_set_presence_writes(
                                component_index,
                                &offset,
                                &before_occurrences,
                                &after_occurrences,
                                &mut writes,
                            );

                            for attr in scratch_to_set(
                                &after_scratch.require,
                                &before_scratch.require,
                                line,
                            )? {
                                writes.push(WriteOpTemplate::SetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                });
                            }
                            for attr in scratch_to_set_object_set(
                                &after_scratch.require_object_set,
                                &before_scratch.require_object_set,
                                line,
                            )? {
                                writes.push(WriteOpTemplate::SetObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                });
                            }

                            for attr in
                                scratch_to_remove(&before_scratch.require, &after_scratch.require)
                                    .into_iter()
                                    .filter(|attr| {
                                        attr.object.is_empty()
                                            || after_objects.contains(&attr.object)
                                    })
                            {
                                writes.push(WriteOpTemplate::RemoveScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in scratch_to_remove_object_set(
                                &before_scratch.require_object_set,
                                &after_scratch.require_object_set,
                            )
                            .into_iter()
                            .filter(|attr| {
                                after_occurrences.iter().any(|occurrence| {
                                    matches!(
                                        &occurrence.matched,
                                        ResolvedObjectMatch::ObjectSet { binding, .. }
                                            if *binding == attr.binding
                                    )
                                })
                            }) {
                                writes.push(WriteOpTemplate::RemoveObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }

                            for attr in &after_scratch.forbid {
                                writes.push(WriteOpTemplate::RemoveScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    object: attr.object,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }
                            for attr in &after_scratch.forbid_object_set {
                                writes.push(WriteOpTemplate::RemoveObjectSetScratch {
                                    component: component_index,
                                    offset: offset.clone(),
                                    binding: attr.binding,
                                    scratch: attr.scratch,
                                    value: attr.value.clone(),
                                    match_value: attr.match_value,
                                });
                            }

                            concrete_x += 1;
                        }
                    }

                    components.push(PatternComponentTemplate {
                        cells: component_cells,
                        gap_count: next_gap_index,
                    });
                }

                writes = preserve_moved_occurrence_scratch(
                    writes,
                    &before_placements,
                    &after_placements,
                    line,
                )?;

                alternatives.push(RuleBodyAlternative {
                    guards: dynamic_guards.clone(),
                    components,
                    writes,
                });
            }
        }
    }

    Ok(alternatives)
}

fn expand_dynamic_selector_blocks(
    before: &PatternBlock,
    after: &PatternBlock,
) -> Vec<(Vec<Guard>, PatternBlock, PatternBlock)> {
    let mut branches = vec![(Vec::new(), before.clone(), after.clone())];
    loop {
        let mut expanded = Vec::new();
        let mut changed = false;
        for (guards, before, after) in branches {
            if let Some(location) = first_dynamic_selector_location(&before) {
                changed = true;
                expand_dynamic_selector_branch(
                    guards,
                    before,
                    after,
                    true,
                    location,
                    &mut expanded,
                );
                continue;
            }
            if let Some(location) = first_dynamic_selector_location(&after) {
                changed = true;
                expand_dynamic_selector_branch(
                    guards,
                    before,
                    after,
                    false,
                    location,
                    &mut expanded,
                );
                continue;
            }
            expanded.push((guards, before, after));
        }
        if !changed {
            return expanded;
        }
        branches = expanded;
    }
}

#[derive(Clone, Copy)]
struct SelectorLocation {
    component: usize,
    row: usize,
    part: usize,
    require: bool,
    selector: usize,
}

fn first_dynamic_selector_location(block: &PatternBlock) -> Option<SelectorLocation> {
    for (component_index, component) in block.components.iter().enumerate() {
        for (row_index, row) in component.rows.iter().enumerate() {
            for (part_index, part) in row.iter().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                if let Some(selector_index) = cell
                    .require
                    .iter()
                    .position(|selector| !selector.dynamic_guards.is_empty())
                {
                    return Some(SelectorLocation {
                        component: component_index,
                        row: row_index,
                        part: part_index,
                        require: true,
                        selector: selector_index,
                    });
                }
                if let Some(selector_index) = cell
                    .forbid
                    .iter()
                    .position(|selector| !selector.dynamic_guards.is_empty())
                {
                    return Some(SelectorLocation {
                        component: component_index,
                        row: row_index,
                        part: part_index,
                        require: false,
                        selector: selector_index,
                    });
                }
            }
        }
    }
    None
}

fn expand_dynamic_selector_branch(
    guards: Vec<Guard>,
    before: PatternBlock,
    after: PatternBlock,
    in_before: bool,
    location: SelectorLocation,
    out: &mut Vec<(Vec<Guard>, PatternBlock, PatternBlock)>,
) {
    let selector = selector_at_location(if in_before { &before } else { &after }, location);
    for object in &selector.alternatives {
        let mut guards = guards.clone();
        if let Some(dynamic_guards) = selector.dynamic_guards.get(object) {
            guards.extend(dynamic_guards.iter().map(|guard| Guard::GlobalEquals {
                global: guard.global,
                value: guard.value,
            }));
        }
        let mut before = before.clone();
        let mut after = after.clone();
        let target = if in_before { &mut before } else { &mut after };
        let selector = selector_at_location_mut(target, location);
        selector.alternatives = vec![*object];
        selector.dynamic_guards.clear();
        out.push((guards, before, after));
    }
}

fn selector_at_location(block: &PatternBlock, location: SelectorLocation) -> &ObjectSelector {
    let BlockPart::Cell(cell) =
        &block.components[location.component].rows[location.row][location.part]
    else {
        unreachable!("selector locations only point to cells");
    };
    if location.require {
        &cell.require[location.selector]
    } else {
        &cell.forbid[location.selector]
    }
}

fn selector_at_location_mut(
    block: &mut PatternBlock,
    location: SelectorLocation,
) -> &mut ObjectSelector {
    let BlockPart::Cell(cell) =
        &mut block.components[location.component].rows[location.row][location.part]
    else {
        unreachable!("selector locations only point to cells");
    };
    if location.require {
        &mut cell.require[location.selector]
    } else {
        &mut cell.forbid[location.selector]
    }
}

#[derive(Clone, Debug)]
struct ScratchSetBinding {
    key: String,
    values: &'static [&'static str],
}

fn expand_movement_scratch_sets(
    before: &PatternBlock,
    after: &PatternBlock,
) -> Vec<(PatternBlock, PatternBlock)> {
    let before = expand_negated_movement_scratch_sets(before);
    let after = expand_negated_movement_scratch_sets(after);
    let mut bindings = Vec::<ScratchSetBinding>::new();
    collect_movement_scratch_set_bindings(&before, &mut bindings);
    collect_movement_scratch_set_bindings(&after, &mut bindings);
    dedup_scratch_set_bindings(&mut bindings);

    if bindings.is_empty() {
        return vec![(before, after)];
    }

    let mut assignments = Vec::<HashMap<String, String>>::new();
    expand_scratch_set_assignments(&bindings, 0, &mut HashMap::new(), &mut assignments);
    assignments
        .into_iter()
        .map(|assignment| {
            (
                apply_movement_scratch_set_assignment(&before, &assignment),
                apply_movement_scratch_set_assignment(&after, &assignment),
            )
        })
        .collect()
}

fn expand_negated_movement_scratch_sets(block: &PatternBlock) -> PatternBlock {
    let mut block = block.clone();
    for component in &mut block.components {
        for row in &mut component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                expand_negated_movement_scratch_set_list(&mut cell.require_cell_scratch);
                expand_negated_movement_scratch_set_list(&mut cell.forbid_cell_scratch);
                for selector in &mut cell.require {
                    expand_negated_movement_scratch_set_list(&mut selector.scratch);
                }
                for selector in &mut cell.forbid {
                    expand_negated_movement_scratch_set_list(&mut selector.scratch);
                }
            }
        }
    }
    block
}

fn expand_negated_movement_scratch_set_list(scratch: &mut Vec<ParsedScratch>) {
    let mut expanded = Vec::with_capacity(scratch.len());
    for scratch in scratch.drain(..) {
        if scratch.negated
            && let Some(value) = scratch.value.as_deref()
            && let Some(values) = movement_scratch_set_values(value)
        {
            expanded.extend(values.iter().map(|value| {
                let mut scratch = scratch.clone();
                scratch.value = Some((*value).to_string());
                scratch
            }));
        } else {
            expanded.push(scratch);
        }
    }
    *scratch = expanded;
}

fn collect_movement_scratch_set_bindings(
    block: &PatternBlock,
    bindings: &mut Vec<ScratchSetBinding>,
) {
    let mut selector_counts = HashMap::<String, usize>::new();
    for (component_index, component) in block.components.iter().enumerate() {
        for (row_index, row) in component.rows.iter().enumerate() {
            for (part_index, part) in row.iter().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                collect_cell_scratch_set_bindings(
                    &cell.require_cell_scratch,
                    format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    bindings,
                );
                collect_cell_scratch_set_bindings(
                    &cell.forbid_cell_scratch,
                    format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    bindings,
                );
                for selector in &cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    collect_cell_scratch_set_bindings(
                        &selector.scratch,
                        format!("object:{}:{ordinal}", selector.token),
                        bindings,
                    );
                }
            }
        }
    }
}

fn collect_cell_scratch_set_bindings(
    scratch: &[ParsedScratch],
    anchor: String,
    bindings: &mut Vec<ScratchSetBinding>,
) {
    for (scratch_index, scratch) in scratch.iter().enumerate() {
        let Some(value) = scratch.value.as_deref() else {
            continue;
        };
        let Some(values) = movement_scratch_set_values(value) else {
            continue;
        };
        bindings.push(ScratchSetBinding {
            key: format!("{anchor}:{scratch_index}:{value}"),
            values,
        });
    }
}

fn dedup_scratch_set_bindings(bindings: &mut Vec<ScratchSetBinding>) {
    let mut deduped = Vec::with_capacity(bindings.len());
    for binding in bindings.drain(..) {
        if !deduped
            .iter()
            .any(|existing: &ScratchSetBinding| existing.key == binding.key)
        {
            deduped.push(binding);
        }
    }
    *bindings = deduped;
}

fn expand_scratch_set_assignments(
    bindings: &[ScratchSetBinding],
    index: usize,
    current: &mut HashMap<String, String>,
    out: &mut Vec<HashMap<String, String>>,
) {
    if index == bindings.len() {
        out.push(current.clone());
        return;
    }
    let binding = &bindings[index];
    for value in binding.values {
        current.insert(binding.key.clone(), (*value).to_string());
        expand_scratch_set_assignments(bindings, index + 1, current, out);
    }
    current.remove(&binding.key);
}

fn apply_movement_scratch_set_assignment(
    block: &PatternBlock,
    assignment: &HashMap<String, String>,
) -> PatternBlock {
    let mut block = block.clone();
    let mut selector_counts = HashMap::<String, usize>::new();
    for (component_index, component) in block.components.iter_mut().enumerate() {
        for (row_index, row) in component.rows.iter_mut().enumerate() {
            for (part_index, part) in row.iter_mut().enumerate() {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                apply_cell_scratch_set_assignment(
                    &mut cell.require_cell_scratch,
                    &format!("cell:{component_index}:{row_index}:{part_index}:require"),
                    assignment,
                );
                apply_cell_scratch_set_assignment(
                    &mut cell.forbid_cell_scratch,
                    &format!("cell:{component_index}:{row_index}:{part_index}:forbid"),
                    assignment,
                );
                for selector in &mut cell.require {
                    let ordinal = *selector_counts.get(&selector.token).unwrap_or(&0);
                    selector_counts.insert(selector.token.clone(), ordinal + 1);
                    apply_cell_scratch_set_assignment(
                        &mut selector.scratch,
                        &format!("object:{}:{ordinal}", selector.token),
                        assignment,
                    );
                }
            }
        }
    }
    block
}

fn apply_cell_scratch_set_assignment(
    scratch: &mut [ParsedScratch],
    anchor: &str,
    assignment: &HashMap<String, String>,
) {
    for (scratch_index, scratch) in scratch.iter_mut().enumerate() {
        let Some(value) = scratch.value.as_deref() else {
            continue;
        };
        if movement_scratch_set_values(value).is_none() {
            continue;
        }
        let key = format!("{anchor}:{scratch_index}:{value}");
        if let Some(concrete) = assignment.get(&key) {
            scratch.value = Some(concrete.clone());
        }
    }
}

fn rectangular_shared_gap_indices(component: &BlockComponent) -> Option<Vec<Option<u16>>> {
    if component.rows.len() <= 1 {
        return None;
    }
    let first = component.rows.first()?;
    if !first.iter().any(|part| matches!(part, BlockPart::Ellipsis)) {
        return None;
    }

    let mut next_gap_index = 0_u16;
    Some(
        first
            .iter()
            .map(|part| {
                if matches!(part, BlockPart::Ellipsis) {
                    let gap_index = next_gap_index;
                    next_gap_index += 1;
                    Some(gap_index)
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn prefer_same_cell_occurrence_keys(
    before_occurrences: &[ResolvedObjectOccurrence],
    after_occurrences: &mut [ResolvedObjectOccurrence],
) {
    let mut used_after = Vec::<usize>::new();
    for before in before_occurrences {
        let Some(key) = before.key.clone() else {
            continue;
        };
        let Some((after_index, after)) =
            after_occurrences
                .iter_mut()
                .enumerate()
                .find(|(index, after)| {
                    !used_after.contains(index)
                        && after.matched == before.matched
                        && !occurrence_key_has_label(&key)
                        && !after.key.as_ref().is_some_and(occurrence_key_has_label)
                })
        else {
            continue;
        };
        after.key = Some(key);
        used_after.push(after_index);
    }
}

fn occurrence_key_has_label(key: &OccurrenceKey) -> bool {
    key.token.contains('#')
}

fn preserve_moved_occurrence_scratch(
    writes: Vec<WriteOpTemplate>,
    before_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    after_placements: &HashMap<OccurrenceKey, OccurrencePlacement>,
    _line: &str,
) -> Result<Vec<WriteOpTemplate>, DiagnosticReport> {
    let moves = before_placements
        .iter()
        .filter_map(|(key, before)| {
            let after = after_placements.get(key)?;
            (before.matched == after.matched
                && before.component == after.component
                && before.offset != after.offset)
                .then_some((before, after))
        })
        .collect::<Vec<_>>();

    if moves.is_empty() {
        return Ok(writes);
    }

    let mut out = Vec::new();
    for (before, after) in &moves {
        match &before.matched {
            ResolvedObjectMatch::Object(object) => {
                out.push(WriteOpTemplate::Move {
                    component: before.component,
                    from_offset: before.offset.clone(),
                    to_offset: after.offset.clone(),
                    object: *object,
                });
            }
            ResolvedObjectMatch::ObjectSet { binding, .. } => {
                out.push(WriteOpTemplate::MoveObjectSet {
                    component: before.component,
                    from_offset: before.offset.clone(),
                    to_offset: after.offset.clone(),
                    binding: *binding,
                    objects: before.matched.possible_objects(),
                });
            }
        }

        for attr in scratch_to_remove(&before.require_scratch, &after.require_scratch) {
            out.push(WriteOpTemplate::RemoveScratch {
                component: after.component,
                offset: after.offset.clone(),
                object: attr.object,
                scratch: attr.scratch,
                value: attr.value,
                match_value: attr.match_value,
            });
        }
        for attr in scratch_to_remove_object_set(
            &before.require_object_set_scratch,
            &after.require_object_set_scratch,
        ) {
            out.push(WriteOpTemplate::RemoveObjectSetScratch {
                component: after.component,
                offset: after.offset.clone(),
                binding: attr.binding,
                scratch: attr.scratch,
                value: attr.value,
                match_value: attr.match_value,
            });
        }
    }

    out.extend(writes.into_iter().filter(|write| {
        !moves.iter().any(|(before, after)| {
            write_removes_match_at(write, before)
                || write_adds_match_at(write, after)
                || write_removes_moved_scratch_at_before(write, before)
        })
    }));

    Ok(out)
}

fn write_removes_moved_scratch_at_before(
    write: &WriteOpTemplate,
    placement: &OccurrencePlacement,
) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::RemoveObjectSetScratch {
                component,
                offset,
                binding,
                scratch,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
                && placement
                    .require_object_set_scratch
                    .iter()
                    .any(|attr| attr.binding == *binding && attr.scratch == *scratch)
        }
        _ => false,
    }
}

fn write_removes_match_at(write: &WriteOpTemplate, placement: &OccurrencePlacement) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::Remove {
                component,
                offset,
                object,
            },
            ResolvedObjectMatch::Object(placement_object),
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && object == placement_object
        }
        (
            WriteOpTemplate::RemoveObjectSet {
                component,
                offset,
                binding,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
        }
        _ => false,
    }
}

fn write_adds_match_at(write: &WriteOpTemplate, placement: &OccurrencePlacement) -> bool {
    match (write, &placement.matched) {
        (
            WriteOpTemplate::Add {
                component,
                offset,
                object,
            },
            ResolvedObjectMatch::Object(placement_object),
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && object == placement_object
        }
        (
            WriteOpTemplate::AddObjectSet {
                component,
                offset,
                binding,
                ..
            },
            ResolvedObjectMatch::ObjectSet {
                binding: placement_binding,
                ..
            },
        ) => {
            *component == placement.component
                && offset == &placement.offset
                && binding == placement_binding
        }
        _ => false,
    }
}

fn block_shapes_match(before: &BlockComponent, after: &BlockComponent) -> bool {
    before.rows.len() == after.rows.len()
        && before.rows.iter().zip(&after.rows).all(|(before, after)| {
            before.len() == after.len()
                && before.iter().zip(after).all(|(before, after)| {
                    matches!(
                        (before, after),
                        (BlockPart::Cell(_), BlockPart::Cell(_))
                            | (BlockPart::Ellipsis, BlockPart::Ellipsis)
                    )
                })
        })
}

fn block_cell_forbid_objects(cell: &BlockCell) -> Vec<ObjectId> {
    let mut objects = Vec::new();
    for selector in &cell.forbid {
        objects.extend(selector.alternatives.iter().copied());
    }
    dedup_objects(&mut objects);
    objects
}

#[derive(Clone, Debug, Default)]
struct BlockCellScratch {
    require: Vec<ScratchPatternTemplate>,
    require_object_set: Vec<ObjectSetScratchPatternTemplate>,
    forbid: Vec<ScratchPatternTemplate>,
    forbid_object_set: Vec<ObjectSetScratchPatternTemplate>,
}

fn block_cell_scratch(
    cell: &BlockCell,
    occurrences: &[ResolvedObjectOccurrence],
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<BlockCellScratch, DiagnosticReport> {
    let mut out = BlockCellScratch::default();
    for scratch in &cell.require_cell_scratch {
        let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
        if scratch.negated {
            out.forbid.push(pattern);
        } else {
            out.require.push(pattern);
        }
    }
    for scratch in &cell.forbid_cell_scratch {
        let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
        out.forbid.push(pattern);
    }
    for (selector, occurrence) in cell.require.iter().zip(occurrences) {
        for scratch in &selector.scratch {
            match &occurrence.matched {
                ResolvedObjectMatch::Object(object) => {
                    let pattern = parsed_scratch_pattern(*object, scratch, scratch_names, line)?;
                    if scratch.negated {
                        out.forbid.push(pattern);
                    } else {
                        out.require.push(pattern);
                    }
                }
                ResolvedObjectMatch::ObjectSet { binding, .. } => {
                    let pattern =
                        parsed_object_set_scratch_pattern(*binding, scratch, scratch_names, line)?;
                    if scratch.negated {
                        out.forbid_object_set.push(pattern);
                    } else {
                        out.require_object_set.push(pattern);
                    }
                }
            }
        }
    }
    dedup_scratch_patterns(&mut out.require);
    dedup_scratch_patterns(&mut out.forbid);
    dedup_object_set_scratch_patterns(&mut out.require_object_set);
    dedup_object_set_scratch_patterns(&mut out.forbid_object_set);
    reject_duplicate_scratch_patterns(&out.require, line)?;
    reject_duplicate_object_set_scratch_patterns(&out.require_object_set, line)?;
    Ok(out)
}

fn dedup_scratch_patterns(patterns: &mut Vec<ScratchPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn dedup_object_set_scratch_patterns(patterns: &mut Vec<ObjectSetScratchPatternTemplate>) {
    let mut deduped = Vec::with_capacity(patterns.len());
    for pattern in patterns.drain(..) {
        if !deduped.contains(&pattern) {
            deduped.push(pattern);
        }
    }
    *patterns = deduped;
}

fn parsed_object_set_scratch_pattern(
    binding: u16,
    scratch: &ParsedScratch,
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<ObjectSetScratchPatternTemplate, DiagnosticReport> {
    let pattern = parsed_scratch_pattern(ObjectId::EMPTY, scratch, scratch_names, line)?;
    Ok(ObjectSetScratchPatternTemplate {
        binding,
        scratch: pattern.scratch,
        value: pattern.value,
        match_value: pattern.match_value,
        is_marker: pattern.is_marker,
    })
}

fn parsed_scratch_pattern(
    object: ObjectId,
    scratch: &ParsedScratch,
    scratch_names: &HashMap<String, ScratchDef>,
    line: &str,
) -> Result<ScratchPatternTemplate, DiagnosticReport> {
    if let Some(anonymous) = &scratch.anonymous {
        return parsed_anonymous_scratch_pattern(object, anonymous, scratch, line);
    }
    let def = scratch_names
        .get(&scratch.name)
        .ok_or_else(|| parse_error(line, "unknown scratch"))?;
    let value = match def.kind {
        ScratchKind::Marker => {
            if scratch.value.is_some() {
                return Err(parse_error(line, "marker scratch cannot have a value"));
            }
            None
        }
        ScratchKind::Bool => {
            if scratch.value.is_some() {
                return Err(parse_error(
                    line,
                    "bool scratch uses presence syntax; write `flag` or `no flag`",
                ));
            }
            Some(ScratchValueTemplate::Literal(1))
        }
        ScratchKind::Int => scratch
            .value
            .as_deref()
            .map(|value| {
                value
                    .parse::<i64>()
                    .map(ScratchValueTemplate::Literal)
                    .map_err(|_| parse_error(line, "expected integer scratch value"))
            })
            .transpose()?,
        ScratchKind::Enum => scratch
            .value
            .as_deref()
            .map(|value| parse_enum_scratch_value(value, def, line))
            .transpose()?,
    };
    let match_value = if value.is_some() {
        ScratchValueMatch::Exact
    } else {
        ScratchValueMatch::Any
    };
    Ok(ScratchPatternTemplate {
        object,
        scratch: def.id,
        value,
        match_value,
        is_marker: matches!(def.kind, ScratchKind::Marker | ScratchKind::Bool),
    })
}

fn parsed_anonymous_scratch_pattern(
    object: ObjectId,
    anonymous: &AnonymousScratch,
    scratch: &ParsedScratch,
    line: &str,
) -> Result<ScratchPatternTemplate, DiagnosticReport> {
    let value = scratch
        .value
        .as_deref()
        .ok_or_else(|| parse_error(line, "anonymous scratch must specify a value"))?;
    let (scratch_id, value, match_value) = match anonymous {
        AnonymousScratch::Movement if value == "directions" => {
            (ANONYMOUS_MOVEMENT_SCRATCH, None, ScratchValueMatch::Any)
        }
        AnonymousScratch::Movement => (
            ANONYMOUS_MOVEMENT_SCRATCH,
            Some(parse_anonymous_movement_value(value, line)?),
            ScratchValueMatch::Exact,
        ),
        AnonymousScratch::Bool => (
            ANONYMOUS_BOOL_SCRATCH,
            Some(ScratchValueTemplate::Literal(match value {
                "false" => 0,
                "true" => 1,
                _ => return Err(parse_error(line, "expected boolean scratch value")),
            })),
            ScratchValueMatch::Exact,
        ),
        AnonymousScratch::Int => (
            ANONYMOUS_INT_SCRATCH,
            Some(ScratchValueTemplate::Literal(
                value
                    .parse::<i64>()
                    .map_err(|_| parse_error(line, "expected integer scratch value"))?,
            )),
            ScratchValueMatch::Exact,
        ),
    };
    Ok(ScratchPatternTemplate {
        object,
        scratch: scratch_id,
        value,
        match_value,
        is_marker: false,
    })
}

fn parse_anonymous_movement_value(
    value: &str,
    line: &str,
) -> Result<ScratchValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(ScratchValueTemplate::Relative(relative));
    }
    puzzle_authoring::movement_scratch_index(value, puzzle_authoring::MOVEMENT_DIRECTIONS_2D)
        .map(|index| ScratchValueTemplate::Literal(i64::from(index)))
        .ok_or_else(|| parse_error(line, "unknown movement scratch value"))
}

fn movement_scratch_set_values(value: &str) -> Option<&'static [&'static str]> {
    puzzle_authoring::movement_scratch_set_values(value, 2)
}

fn parse_enum_scratch_value(
    value: &str,
    def: &ScratchDef,
    line: &str,
) -> Result<ScratchValueTemplate, DiagnosticReport> {
    if let Some(relative) = parse_relative_direction_value(value) {
        return Ok(ScratchValueTemplate::Relative(relative));
    }
    def.values
        .iter()
        .position(|candidate| candidate == value)
        .map(|index| ScratchValueTemplate::Literal(index as i64))
        .ok_or_else(|| parse_error(line, "unknown enum scratch value"))
}

fn parse_relative_direction_value(value: &str) -> Option<RelativeDirection> {
    match value {
        ">" => Some(RelativeDirection::Forward),
        "<" => Some(RelativeDirection::Backward),
        "^" => Some(RelativeDirection::Left),
        "v" => Some(RelativeDirection::Right),
        _ => None,
    }
}

fn reject_duplicate_scratch_patterns(
    scratch: &[ScratchPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(ObjectId, ScratchId)>::new();
    for attr in scratch {
        let key = (attr.object, attr.scratch);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same scratch twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn reject_duplicate_object_set_scratch_patterns(
    scratch: &[ObjectSetScratchPatternTemplate],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<(u16, ScratchId)>::new();
    for attr in scratch {
        let key = (attr.binding, attr.scratch);
        if seen.contains(&key) {
            return Err(parse_error(
                line,
                "same object occurrence cannot mention the same scratch twice",
            ));
        }
        seen.push(key);
    }
    Ok(())
}

fn scratch_to_set(
    after: &[ScratchPatternTemplate],
    before: &[ScratchPatternTemplate],
    line: &str,
) -> Result<Vec<ScratchPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_marker && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS scratch must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn scratch_to_set_object_set(
    after: &[ObjectSetScratchPatternTemplate],
    before: &[ObjectSetScratchPatternTemplate],
    line: &str,
) -> Result<Vec<ObjectSetScratchPatternTemplate>, DiagnosticReport> {
    let mut writes = Vec::new();
    for attr in after {
        if !attr.is_marker && attr.value.is_none() {
            return Err(parse_error(line, "valued RHS scratch must specify a value"));
        }
        if !before.iter().any(|before| before == attr) {
            writes.push(attr.clone());
        }
    }
    Ok(writes)
}

fn scratch_to_remove(
    before: &[ScratchPatternTemplate],
    after: &[ScratchPatternTemplate],
) -> Vec<ScratchPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn scratch_to_remove_object_set(
    before: &[ObjectSetScratchPatternTemplate],
    after: &[ObjectSetScratchPatternTemplate],
) -> Vec<ObjectSetScratchPatternTemplate> {
    before
        .iter()
        .filter(|before| !after.iter().any(|after| after == *before))
        .cloned()
        .collect()
}

fn implicit_layer_forbids(
    before_objects: &[ObjectId],
    after_objects: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
    occupancy_objects: &[ObjectId],
) -> Vec<ObjectId> {
    let mut forbids = Vec::new();
    for after_object in after_objects {
        if before_objects.contains(after_object) {
            continue;
        }
        let Some(after_layer) = object_layers.get(after_object) else {
            continue;
        };
        forbids.extend(occupancy_objects.iter().filter_map(|object| {
            let object_layer = object_layers.get(object)?;
            (object_layer == after_layer
                && !before_objects.contains(object)
                && !after_objects.contains(object))
            .then_some(*object)
        }));
    }
    dedup_objects(&mut forbids);
    forbids
}

fn dedup_objects(objects: &mut Vec<ObjectId>) {
    let mut deduped = Vec::with_capacity(objects.len());
    for object in objects.drain(..) {
        if !deduped.contains(&object) {
            deduped.push(object);
        }
    }
    *objects = deduped;
}

#[derive(Clone, Debug)]
struct SelectorOccurrence {
    token: String,
    alternatives: Vec<ObjectId>,
    occurrence_label: Option<String>,
    cell_index: usize,
    binding: u16,
}

#[derive(Clone, Debug)]
enum SelectorAssignmentValue {
    Object(ObjectId),
    ObjectSet {
        binding: u16,
        layer: LayerId,
        objects: Vec<ObjectId>,
    },
}

fn collect_before_occurrences(block: &PatternBlock) -> Vec<SelectorOccurrence> {
    let mut occurrences = Vec::new();
    let mut cell_index = 0usize;
    let mut next_binding = 0u16;
    for component in &block.components {
        for row in &component.rows {
            for part in row {
                if let BlockPart::Cell(cell) = part {
                    for selector in &cell.require {
                        occurrences.push(SelectorOccurrence {
                            token: selector.token.clone(),
                            alternatives: selector.alternatives.clone(),
                            occurrence_label: selector.occurrence_label.clone(),
                            cell_index,
                            binding: next_binding,
                        });
                        next_binding = next_binding.saturating_add(1);
                    }
                    cell_index += 1;
                }
            }
        }
    }
    occurrences
}

fn reject_duplicate_labeled_occurrences(
    occurrences: &[SelectorOccurrence],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let mut seen = Vec::<String>::new();
    for occurrence in occurrences {
        if occurrence.occurrence_label.is_none() {
            continue;
        }
        if seen.contains(&occurrence.token) {
            return Err(parse_error(
                line,
                "selector occurrence label must be unique within the before pattern",
            ));
        }
        seen.push(occurrence.token.clone());
    }
    Ok(())
}

fn expand_selector_assignments(
    occurrences: &[SelectorOccurrence],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<Vec<Vec<SelectorAssignmentValue>>, DiagnosticReport> {
    let mut assignments = vec![Vec::<SelectorAssignmentValue>::new()];
    for (index, occurrence) in occurrences.iter().enumerate() {
        if occurrence.occurrence_label.is_none()
            && !occurrence.token.contains('*')
            && !occurrence.token.contains(':')
            && let Some(layer) = same_layer_alternatives(&occurrence.alternatives, object_layers)
            && selector_occurrence_can_use_object_set(occurrences, index, layer, object_layers)
        {
            let mut next = Vec::new();
            for prefix in &assignments {
                if !selector_assignment_value_is_possible(
                    occurrences,
                    prefix,
                    index,
                    layer,
                    &occurrence.alternatives,
                    object_layers,
                    line,
                )? {
                    continue;
                }
                let mut assignment = prefix.clone();
                assignment.push(SelectorAssignmentValue::ObjectSet {
                    binding: occurrence.binding,
                    layer,
                    objects: occurrence.alternatives.clone(),
                });
                next.push(assignment);
            }
            assignments = next;
            continue;
        }
        let mut next = Vec::new();
        for prefix in &assignments {
            for object in &occurrence.alternatives {
                if !selector_assignment_object_is_possible(
                    occurrences,
                    prefix,
                    index,
                    *object,
                    object_layers,
                    line,
                )? {
                    continue;
                }
                let mut assignment = prefix.clone();
                assignment.push(SelectorAssignmentValue::Object(*object));
                next.push(assignment);
            }
        }
        assignments = next;
    }
    Ok(assignments)
}

fn selector_occurrence_can_use_object_set(
    occurrences: &[SelectorOccurrence],
    index: usize,
    layer: LayerId,
    object_layers: &HashMap<ObjectId, LayerId>,
) -> bool {
    let occurrence = &occurrences[index];
    !occurrences.iter().enumerate().any(|(other_index, other)| {
        other_index != index
            && other.cell_index == occurrence.cell_index
            && other.alternatives.len() > 1
            && (same_layer_alternatives(&other.alternatives, object_layers).is_none()
                || same_layer_alternatives(&other.alternatives, object_layers) == Some(layer))
    })
}

fn same_layer_alternatives(
    alternatives: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
) -> Option<LayerId> {
    if alternatives.len() <= 1 {
        return None;
    }
    puzzle_kernel::object_set_matcher_for_same_layer(0, alternatives, |object| {
        object_layers.get(&object).copied()
    })
    .map(|matcher| matcher.layer)
}

fn selector_assignment_value_is_possible(
    occurrences: &[SelectorOccurrence],
    prefix: &[SelectorAssignmentValue],
    index: usize,
    layer: LayerId,
    objects: &[ObjectId],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let occurrence = &occurrences[index];
    for (previous_index, previous_value) in prefix.iter().enumerate() {
        let previous = &occurrences[previous_index];
        if previous.cell_index != occurrence.cell_index {
            continue;
        }
        let previous_layer = match previous_value {
            SelectorAssignmentValue::Object(object) => {
                let Some(previous_layer) = object_layers.get(object).copied() else {
                    continue;
                };
                previous_layer
            }
            SelectorAssignmentValue::ObjectSet { layer, .. } => *layer,
        };
        if previous_layer != layer {
            continue;
        }
        if previous.alternatives.len() > 1 || objects.len() > 1 {
            return Ok(false);
        }
        return Err(parse_error(
            line,
            &format!(
                "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                previous.token, occurrence.token
            ),
        ));
    }
    Ok(true)
}

fn selector_assignment_object_is_possible(
    occurrences: &[SelectorOccurrence],
    prefix: &[SelectorAssignmentValue],
    index: usize,
    object: ObjectId,
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let occurrence = &occurrences[index];
    let Some(layer) = object_layers.get(&object) else {
        return Ok(true);
    };
    for (previous_index, previous_value) in prefix.iter().enumerate() {
        let previous = &occurrences[previous_index];
        if previous.cell_index != occurrence.cell_index {
            continue;
        }
        let previous_layer = match previous_value {
            SelectorAssignmentValue::Object(previous_object) => {
                let Some(previous_layer) = object_layers.get(previous_object) else {
                    continue;
                };
                *previous_layer
            }
            SelectorAssignmentValue::ObjectSet { layer, .. } => *layer,
        };
        if previous_layer != *layer {
            continue;
        }
        if previous.alternatives.len() > 1 || occurrence.alternatives.len() > 1 {
            return Ok(false);
        }
        if matches!(previous_value, SelectorAssignmentValue::Object(previous_object) if *previous_object == object)
        {
            continue;
        }
        return Err(parse_error(
            line,
            &format!(
                "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                previous.token, occurrence.token
            ),
        ));
    }
    Ok(true)
}

fn before_occurrences_by_token(occurrences: &[SelectorOccurrence]) -> HashMap<String, Vec<usize>> {
    let mut by_token = HashMap::<String, Vec<usize>>::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        by_token
            .entry(occurrence.token.clone())
            .or_default()
            .push(index);
    }
    by_token
}

fn block_cell_object_occurrences(
    cell: &BlockCell,
    assignment: &[SelectorAssignmentValue],
    before_occurrences: &[SelectorOccurrence],
    before_by_token: &HashMap<String, Vec<usize>>,
    token_counts: &mut HashMap<String, usize>,
    line: &str,
) -> Result<Vec<ResolvedObjectOccurrence>, DiagnosticReport> {
    cell.require
        .iter()
        .map(|selector| {
            let ordinal = if selector.occurrence_label.is_some() {
                0
            } else {
                let ordinal = *token_counts.get(&selector.token).unwrap_or(&0);
                token_counts.insert(selector.token.clone(), ordinal + 1);
                ordinal
            };
            if let Some(transform) = &selector.transform {
                let before_occurrences =
                    before_by_token
                        .get(&transform.source_token)
                        .ok_or_else(|| {
                            parse_error(line, "mapped selector source must appear in before")
                        })?;
                let before_index = before_occurrences.get(ordinal).ok_or_else(|| {
                    parse_error(line, "mapped selector source occurrence missing")
                })?;
                let source_object = assignment
                    .get(*before_index)
                    .and_then(assignment_concrete_object)
                    .ok_or_else(|| parse_error(line, "internal selector assignment missing"))?;
                return transform
                    .mapped_objects
                    .get(&source_object)
                    .copied()
                    .map(|object| ResolvedObjectOccurrence {
                        token: selector.token.clone(),
                        matched: ResolvedObjectMatch::Object(object),
                        key: None,
                        from_multi_selector: selector.alternatives.len() > 1,
                    })
                    .ok_or_else(|| parse_error(line, "mapped selector source object missing"));
            }
            if let Some(before_occurrences) = before_by_token.get(&selector.token) {
                if let Some(before_index) = before_occurrences.get(ordinal) {
                    return assignment
                        .get(*before_index)
                        .map(|value| ResolvedObjectOccurrence {
                            token: selector.token.clone(),
                            matched: assignment_value_to_match(value),
                            key: Some(OccurrenceKey {
                                token: selector.token.clone(),
                                ordinal,
                            }),
                            from_multi_selector: selector.alternatives.len() > 1,
                        })
                        .ok_or_else(|| parse_error(line, "internal selector assignment missing"));
                }
            }
            if let Some(family_wildcard) = &selector.family_wildcard {
                let candidates = before_occurrences
                    .iter()
                    .enumerate()
                    .filter_map(|(index, occurrence)| {
                        if let Some(label) = &selector.occurrence_label
                            && occurrence.occurrence_label.as_ref() != Some(label)
                        {
                            return None;
                        }
                        let source = assignment.get(index).and_then(assignment_concrete_object)?;
                        let target = family_wildcard.mapped_objects.get(&source).copied()?;
                        Some(target)
                    })
                    .collect::<Vec<_>>();
                let target = if selector.occurrence_label.is_some() {
                    match candidates.as_slice() {
                        [target] => *target,
                        [] => {
                            return Err(parse_error(
                                line,
                                "mapped selector source occurrence missing",
                            ));
                        }
                        _ => {
                            return Err(parse_error(
                                line,
                                "family wildcard selector source is ambiguous",
                            ));
                        }
                    }
                } else {
                    *candidates.get(ordinal).ok_or_else(|| {
                        parse_error(line, "mapped selector source occurrence missing")
                    })?
                };
                return Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(target),
                    key: None,
                    from_multi_selector: selector.alternatives.len() > 1,
                });
            }
            if selector.alternatives.len() == 1 {
                if selector.occurrence_label.is_some() {
                    return Err(parse_error(
                        line,
                        "after selector with an occurrence label must also appear in before",
                    ));
                }
                Ok(ResolvedObjectOccurrence {
                    token: selector.token.clone(),
                    matched: ResolvedObjectMatch::Object(selector.alternatives[0]),
                    key: None,
                    from_multi_selector: false,
                })
            } else {
                Err(parse_error(
                    line,
                    "after selector with alternatives must also appear in before",
                ))
            }
        })
        .collect()
}

fn assignment_concrete_object(value: &SelectorAssignmentValue) -> Option<ObjectId> {
    match value {
        SelectorAssignmentValue::Object(object) => Some(*object),
        SelectorAssignmentValue::ObjectSet { .. } => None,
    }
}

fn assignment_value_to_match(value: &SelectorAssignmentValue) -> ResolvedObjectMatch {
    match value {
        SelectorAssignmentValue::Object(object) => ResolvedObjectMatch::Object(*object),
        SelectorAssignmentValue::ObjectSet {
            binding,
            layer,
            objects,
        } => ResolvedObjectMatch::ObjectSet {
            binding: *binding,
            layer: *layer,
            objects: objects.clone(),
        },
    }
}

fn validate_same_layer_cell_occurrences(
    occurrences: &[ResolvedObjectOccurrence],
    object_layers: &HashMap<ObjectId, LayerId>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let mut seen = Vec::<(LayerId, &ResolvedObjectOccurrence)>::new();
    for occurrence in occurrences {
        let layer = match &occurrence.matched {
            ResolvedObjectMatch::Object(object) => {
                let Some(layer) = object_layers.get(object).copied() else {
                    continue;
                };
                layer
            }
            ResolvedObjectMatch::ObjectSet { layer, .. } => *layer,
        };
        if let Some((_, existing)) = seen
            .iter()
            .find(|(existing_layer, _)| *existing_layer == layer)
        {
            if existing.from_multi_selector || occurrence.from_multi_selector {
                return Ok(false);
            }
            if resolved_occurrences_may_be_same_object(existing, occurrence) {
                continue;
            }
            return Err(parse_error(
                line,
                &format!(
                    "cell pattern cannot contain both `{}` and `{}` because they are in the same collision layer",
                    existing.token, occurrence.token
                ),
            ));
        }
        seen.push((layer, occurrence));
    }
    Ok(true)
}

fn possible_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .flat_map(|occurrence| occurrence.matched.possible_objects())
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn concrete_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .filter_map(|occurrence| match occurrence.matched {
            ResolvedObjectMatch::Object(object) => Some(object),
            ResolvedObjectMatch::ObjectSet { .. } => None,
        })
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn object_sets_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectSetMatcher> {
    let mut out = Vec::new();
    for occurrence in occurrences {
        let ResolvedObjectMatch::ObjectSet {
            binding,
            layer,
            objects,
        } = &occurrence.matched
        else {
            continue;
        };
        if out
            .iter()
            .any(|existing: &ObjectSetMatcher| existing.binding == *binding)
        {
            continue;
        }
        out.push(ObjectSetMatcher {
            binding: *binding,
            layer: *layer,
            objects: objects.clone(),
        });
    }
    out
}

fn object_set_objects_for_occurrences(occurrences: &[ResolvedObjectOccurrence]) -> Vec<ObjectId> {
    let mut objects = occurrences
        .iter()
        .flat_map(|occurrence| match &occurrence.matched {
            ResolvedObjectMatch::Object(_) => Vec::new(),
            ResolvedObjectMatch::ObjectSet { objects, .. } => objects.clone(),
        })
        .collect::<Vec<_>>();
    dedup_objects(&mut objects);
    objects
}

fn append_object_set_presence_writes(
    component: u16,
    offset: &OffsetTemplate,
    before_occurrences: &[ResolvedObjectOccurrence],
    after_occurrences: &[ResolvedObjectOccurrence],
    writes: &mut Vec<WriteOpTemplate>,
) {
    for before in before_occurrences {
        let ResolvedObjectMatch::ObjectSet { binding, .. } = &before.matched else {
            continue;
        };
        if before.key.is_some()
            && after_occurrences
                .iter()
                .any(|after| after.key == before.key && after.matched == before.matched)
        {
            continue;
        }
        writes.push(WriteOpTemplate::RemoveObjectSet {
            component,
            offset: offset.clone(),
            binding: *binding,
            objects: before.matched.possible_objects(),
        });
    }
    for after in after_occurrences {
        let ResolvedObjectMatch::ObjectSet { binding, .. } = &after.matched else {
            continue;
        };
        if after.key.is_some()
            && before_occurrences
                .iter()
                .any(|before| before.key == after.key && before.matched == after.matched)
        {
            continue;
        }
        writes.push(WriteOpTemplate::AddObjectSet {
            component,
            offset: offset.clone(),
            binding: *binding,
            objects: after.matched.possible_objects(),
        });
    }
}

fn resolved_occurrences_may_be_same_object(
    left: &ResolvedObjectOccurrence,
    right: &ResolvedObjectOccurrence,
) -> bool {
    left.matched
        .possible_objects()
        .iter()
        .any(|object| right.matched.possible_objects().contains(object))
}

fn direction_by_name(
    name: &str,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Option<Direction> {
    let input = input_names.get(name)?;
    directions
        .iter()
        .copied()
        .find(|direction| direction.input == *input)
}

fn resolve_write(
    write: &WriteOpTemplate,
    direction: Direction,
    dir_any: bool,
    line: &str,
) -> Result<WriteOp, DiagnosticReport> {
    match write {
        WriteOpTemplate::Add {
            component,
            offset,
            object,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::Add {
                component: *component,
                offset,
                object: *object,
            })
        }
        WriteOpTemplate::AddObjectSet {
            component,
            offset,
            binding,
            ..
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::AddObjectSet {
                component: *component,
                offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::Remove {
            component,
            offset,
            object,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::Remove {
                component: *component,
                offset,
                object: *object,
            })
        }
        WriteOpTemplate::RemoveObjectSet {
            component,
            offset,
            binding,
            ..
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::RemoveObjectSet {
                component: *component,
                offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            let from_offset = resolve_offset(from_offset.clone(), direction, dir_any, line)?;
            let to_offset = resolve_offset(to_offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::Move {
                component: *component,
                from_offset,
                to_offset,
                object: *object,
            })
        }
        WriteOpTemplate::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
            ..
        } => {
            let from_offset = resolve_offset(from_offset.clone(), direction, dir_any, line)?;
            let to_offset = resolve_offset(to_offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::MoveObjectSet {
                component: *component,
                from_offset,
                to_offset,
                binding: *binding,
            })
        }
        WriteOpTemplate::SetScratch {
            component,
            offset,
            object,
            scratch,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::SetScratch {
                component: *component,
                offset,
                object: *object,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::SetObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::SetObjectSetScratch {
                component: *component,
                offset,
                binding: *binding,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
            })
        }
        WriteOpTemplate::RemoveScratch {
            component,
            offset,
            object,
            scratch,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::RemoveScratch {
                component: *component,
                offset,
                object: *object,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
        WriteOpTemplate::RemoveObjectSetScratch {
            component,
            offset,
            binding,
            scratch,
            value,
            match_value,
        } => {
            let offset = resolve_offset(offset.clone(), direction, dir_any, line)?;
            Ok(WriteOp::RemoveObjectSetScratch {
                component: *component,
                offset,
                binding: *binding,
                scratch: *scratch,
                value: resolve_scratch_value(value.as_ref(), direction, dir_any, line)?,
                match_value: *match_value,
            })
        }
    }
}

fn resolve_scratch_patterns(
    patterns: Vec<ScratchPatternTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<ScratchPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(ScratchPattern {
                object: pattern.object,
                scratch: pattern.scratch,
                value: resolve_scratch_value(
                    pattern.value.as_ref(),
                    direction,
                    direction_expanded,
                    line,
                )?,
                match_value: pattern.match_value,
            })
        })
        .collect()
}

fn resolve_object_set_scratch_patterns(
    patterns: Vec<ObjectSetScratchPatternTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Vec<ObjectSetScratchPattern>, DiagnosticReport> {
    patterns
        .into_iter()
        .map(|pattern| {
            Ok(ObjectSetScratchPattern {
                binding: pattern.binding,
                scratch: pattern.scratch,
                value: resolve_scratch_value(
                    pattern.value.as_ref(),
                    direction,
                    direction_expanded,
                    line,
                )?,
                match_value: pattern.match_value,
            })
        })
        .collect()
}

fn resolve_scratch_value(
    value: Option<&ScratchValueTemplate>,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Option<i64>, DiagnosticReport> {
    match value {
        Some(ScratchValueTemplate::Literal(value)) => Ok(Some(*value)),
        Some(ScratchValueTemplate::Relative(relative)) => {
            let direction =
                resolve_relative_direction(*relative, direction, direction_expanded, line)?;
            Ok(Some(direction_value(direction)?))
        }
        None => Ok(None),
    }
}

fn resolve_relative_direction(
    relative: RelativeDirection,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Direction, DiagnosticReport> {
    if !direction_expanded {
        return Err(parse_error(
            line,
            "relative direction scratch value requires an oriented rule",
        ));
    }
    let (dx, dy) = match relative {
        RelativeDirection::Forward => (direction.dx, direction.dy),
        RelativeDirection::Backward => (-direction.dx, -direction.dy),
        RelativeDirection::Left => (direction.dy, -direction.dx),
        RelativeDirection::Right => (-direction.dy, direction.dx),
    };
    Ok(Direction {
        input: InputId(0),
        dx,
        dy,
    })
}

fn direction_value(direction: Direction) -> Result<i64, DiagnosticReport> {
    match (direction.dx, direction.dy) {
        (0, -1) => Ok(0),
        (0, 1) => Ok(1),
        (-1, 0) => Ok(2),
        (1, 0) => Ok(3),
        _ => Err(DiagnosticReport::error(
            "unsupported direction scratch".to_string(),
        )),
    }
}

fn resolve_offset(
    offset: OffsetTemplate,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<Offset, DiagnosticReport> {
    let (base_dx, base_dy) = resolve_oriented_xy(
        offset.oriented_x,
        offset.oriented_y,
        direction,
        direction_expanded,
        line,
    )?;
    if offset.gap_terms.is_empty() {
        return Ok(Offset::Fixed {
            dx: base_dx,
            dy: base_dy,
        });
    }

    let (step_dx, step_dy) = resolve_oriented_xy(1, 0, direction, direction_expanded, line)?;
    Ok(Offset::Variable {
        base_dx,
        base_dy,
        gap_terms: offset
            .gap_terms
            .iter()
            .copied()
            .map(|gap_index| GapTerm {
                gap_index,
                dx: step_dx,
                dy: step_dy,
            })
            .collect(),
    })
}

fn resolve_oriented_xy(
    x: i16,
    y: i16,
    direction: Direction,
    direction_expanded: bool,
    line: &str,
) -> Result<(i16, i16), DiagnosticReport> {
    if !direction_expanded {
        return Ok((x, y));
    }

    Ok(match (direction.dx, direction.dy) {
        (1, 0) => (x, y),
        (-1, 0) => (-x, -y),
        (0, -1) => (y, -x),
        (0, 1) => (-y, x),
        _ => return Err(parse_error(line, "unsupported direction")),
    })
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_qualified_identifier(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier(first) && parts.all(is_identifier)
}

fn is_value_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_char(token: Option<&&str>, line: &str, message: &str) -> Result<char, DiagnosticReport> {
    let value = expect(token, line, message)?;
    let mut chars = value.chars();
    let ch = chars.next().ok_or_else(|| parse_error(line, message))?;
    if chars.next().is_some() {
        return Err(parse_error(line, "expected single character"));
    }
    Ok(ch)
}

fn parse_u16(token: Option<&&str>, line: &str, message: &str) -> Result<u16, DiagnosticReport> {
    expect(token, line, message)?
        .parse()
        .map_err(|_| parse_error(line, "expected u16"))
}

fn parse_global_value(token: &str, line: &str) -> Result<i64, DiagnosticReport> {
    match token {
        "true" => Ok(1),
        "false" => Ok(0),
        _ => token
            .parse()
            .map_err(|_| parse_error(line, "expected true, false, or integer")),
    }
}

fn expect<'a>(
    token: Option<&'a &str>,
    line: &str,
    message: &str,
) -> Result<&'a str, DiagnosticReport> {
    token.copied().ok_or_else(|| parse_error(line, message))
}

fn parse_error(line: &str, message: &str) -> DiagnosticReport {
    DiagnosticReport::error_at_line(message, line)
}

#[cfg(test)]
mod tests;
