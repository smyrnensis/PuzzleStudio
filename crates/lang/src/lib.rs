mod ast;
mod authoring_grammar;
mod catalog;
mod completion;
mod error;
mod frame3_literal;
mod highlight;
mod level;
mod level_editor_source;
mod lib_authoring_parse_order;
mod loaded;
mod model_syntax;
mod puzzle3_model;
mod spatial_materialize3;
mod puzzle3_sprite;
mod puzzle3_visual_fixture;
mod puzzlescript;
mod rule_syntax;
mod semantic;
mod solver_surface;
mod source;
mod source_analysis;
mod source_folding;
mod source_import;
mod source_outline;
mod source_sprite_edit;
mod source_target;
mod sprite_authoring;
mod sprite_spatial;
mod surface;
mod surface_completion;
mod syntax;

use solver_surface::{SolverSurfacePatternArg, SolverSurfaceQueryArg};
use std::collections::{BTreeSet, HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Component, Path, PathBuf};

use ast::{
    ConditionAst, ConditionDefinitionAst, ConditionPatternAst, ConditionValueAst, Direction,
    DirectionName, EffectAst, FixDefaults, OrientationExpr, OrientedRewriteAst,
    PatternConditionAst, PatternPredicateAst, QueryDefinitionAst, RuleDefinitionAst,
    SolverStrategyAst, StatementAst, VariableValueAst,
};
use catalog::{Catalog, ObjectSchema, ObjectVariant, Rational, ValueMap, ValueType};
pub use completion::{
    CompletionItem, CompletionKind, CompletionList, completion_list_json,
    suggest_source_completions,
};
pub use error::{Diagnostic, DiagnosticReport, DiagnosticSeverity, DiagnosticSpan};
pub use highlight::{
    HighlightedSource, HighlightedSourceWithOutline, SourceHighlightKind, SourceHighlightSpan,
    highlight_source, highlight_source_with_outline,
};
use level::{LevelBlock, parse_level};
pub use loaded::{
    AnimationDef, ArrowKey, AsciiLegend, AssetDef, AssetKind, AssetsDef, Controls, ForSource,
    GoalClause, GoalCondition, GoalExpr, GoalValue, InputBufferDef, KeyBinding, KeyTrigger, Level,
    LevelMenuDef, LevelMenuLocked, LevelRegionDef, LoadedDocument, LoadedDocumentModel, LoadedGame,
    ModelOperationSound, ModelOperationSoundDef, MusicSoundDef, PuzzleGridRenderDef,
    PuzzleRenderDef, PuzzleScreenDef, PuzzleViewDef, QueryExpr, QueryExpr3, QueryExprOf,
    ResourceSelection, RuleAnimation, RuleAnimationTrigger, RuleDebugInfo, RuleEffect,
    SceneAlignDef, SceneAspectRatioDef, SceneBinaryOp, SceneButtonDef, SceneComponent,
    SceneConditionalDef, SceneContainerDef, SceneDef, SceneDistributionDef, SceneEffect,
    SceneEffectParam, SceneExpr, SceneForDef, SceneLayoutDef, SceneLevelKey, ScenePuzzleDef,
    ScenePuzzleInitializer, ScenePuzzleRule, SceneResources, SceneRoutineDef, SceneSpaceDef,
    SceneStateDef, SceneStateLifetime, SceneTextAlignDef, SceneTextContent, SceneTextDef,
    SceneTextRoleDef, SceneTransition, SceneTransitionTrigger, SceneValue, SceneVarDef,
    SceneVarKind, SfxSoundDef, SolverDeadendOf, SolverStrategy, SolverStrategy3,
    SolverStrategyDirection, SolverStrategyOf, SolverStrategyTerm, SolverStrategyTerm3,
    SolverStrategyTermOf, SoundsDef, ThemeDef, ThemeVariableDef, TriggerAnimationDef,
    TriggerAnimationKind, TweenAnimationDef, ViewportModeDef, ViewportSizeDef, VisualAliasDef,
    VisualColorDef, VisualOrderDef, VisualOrderPriorityDef, VisualSpriteDef, VisualSpriteFit,
    VisualSpriteFitMode, VisualSpriteKind, VisualSpriteLoopDef, VisualSpritePixelsPerCell,
    VisualSpriteSampling, VisualSpriteSpace, VisualSpriteSpatialDef, VisualSpriteTransform,
    VisualsDef,
};
pub use model_syntax::ModelDimension;

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

fn is_block_header_line(line: &str) -> bool {
    line.trim_end().ends_with('{')
}
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionId, ConditionValueKind, Effect, GapTerm,
    Guard, InputId, LayerId, LocalFrame, LocalFrameExtent, MarkDef, MarkId, MarkKind, MarkPattern,
    MarkValueMatch, MatchCell, ObjectDef, ObjectId, ObjectSetMarkPattern, ObjectSetMatcher, Offset,
    Pattern, PatternComponent, Rule, RuleApplication, RuleCondition, RuleId, RuleStep, State,
    VariableId, VariableUpdateOp, WriteOp,
};
pub use puzzle3_model::{
    CameraSettings3, ParsedPuzzle3, PixelateRenderSettings3, SpriteRenderSettings3,
    ViewportFollow3, ViewportFraming3, ViewportHeight3, ViewportMode3, ViewportSettings3,
};
pub type ParseError3 = DiagnosticReport;
pub use puzzle3_sprite::{
    Sprite3, SpriteColor3, SpriteSet3, SpriteSpace3, SpriteSpatialOp3, SpriteVoxels3,
};
pub use puzzle3_visual_fixture::{
    VisualFixtureExportError3, export_visual_fixture_json, export_visual_fixture_json_with_title,
    export_visual_fixture_json_with_title_and_scenes,
};
pub use puzzlescript::translate_puzzlescript_to_canonical;
pub use semantic::{SemanticKind, SemanticToken, semantic_tokens};
use source::{
    SourceScope, SourceToken, logical_lines_with_locations, source_line_tokens,
    split_header_tokens, strip_line_comment,
};
pub use sprite_spatial::{SpriteAffine3, evaluate_sprite_spatial_ops3};

pub fn parse_level_ascii_state(
    game: &CompiledGame,
    lines: &[String],
    empty: char,
    char_objects: &HashMap<char, Vec<ObjectId>>,
    variable_defaults: &[i64],
) -> Result<(State, Vec<LevelRegionDef>), DiagnosticReport> {
    let lines = lines
        .iter()
        .enumerate()
        .map(|(index, line)| source::LogicalLine::new(line, index + 1))
        .collect::<Vec<_>>();
    let parsed = parse_level(game, &lines, Some(empty), char_objects, variable_defaults).value?;
    Ok((parsed.state, parsed.regions))
}
pub use source_analysis::{
    SourceAnalysis, SourceAnalysisEdit, SourceAnalysisEditResult, analyze_source,
    analyze_source_for_profile, analyze_source_json,
};
pub use source_import::{SourceImportRange, SourceImportReference};
pub use source_outline::{SourceOutlineItem, source_outline, source_outline_json};
pub use source_sprite_edit::{SpriteEditMutationResult, mutate_sprite_source};
pub use source_target::{
    SoundSourceTargetKind, SourceSprite3dStatus, SourceSpriteColorAsset, SourceSpriteDimension,
    SourceSpriteDocument, SourceSpritePaletteEntry, SourceSpriteShapeAsset, SourceSpriteStatus,
    SourceSpriteTarget, SourceTarget, SourceTargetKind, resolve_source_target,
    resolve_source_target_for_profile, source_entries_json, source_target_json,
};
use surface::{
    SourceSpan, SurfaceDisplayFact, SurfaceDocument, SurfaceHighlightRanges, SurfaceNodeKind,
    SurfaceOptionBlock, SurfaceOutlineBlock, SurfaceRewriteEffect, SurfaceSceneEffect,
    SurfaceSemanticKind, SurfaceSemanticToken, SurfaceSink, SurfaceStructuralBlock,
    SurfaceStructuralBlockRole,
};
use syntax::puzzle_lifecycle_event;

const ANONYMOUS_MOVEMENT_MARK: MarkId = MarkId(puzzle_authoring::ANONYMOUS_MOVEMENT_MARK_INDEX);
const ANONYMOUS_BOOL_MARK: MarkId = MarkId(1);
const ANONYMOUS_INT_MARK: MarkId = MarkId(2);
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

include!("lib_surface_doc.rs");
include!("lib_document.rs");
include!("lib_authoring_parse_syntax.rs");
include!("lib_authoring_parse.rs");
include!("lib_authoring_parse_audio.rs");
include!("lib_authoring_parse_metadata.rs");
include!("lib_authoring_parse_levels.rs");
include!("lib_authoring_parse_catalog.rs");
include!("lib_authoring_parse_scene.rs");
include!("lib_authoring_parse_effects.rs");
include!("lib_authoring_parse_scene_state.rs");
include!("lib_authoring_parse_visuals.rs");
include!("lib_authoring_parse_rules.rs");
include!("lib_authoring_parse_statements.rs");
include!("lib_lowering.rs");
include!("lib_patterns.rs");

#[cfg(test)]
mod tests;
