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
    DirectionName, EffectAst, FixDefaults, GlobalValueAst, OrientationExpr, OrientedRewriteAst,
    PatternConditionAst, PatternPredicateAst, RuleDefinitionAst, RuleRole, StatementAst,
};
use catalog::{AxisKind, Catalog, ObjectSchema, ObjectVariant, Rational, ValueMap};
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
    ModelOperationSound, ModelOperationSoundDef, MusicSoundDef, PuzzleGridRenderDef,
    PuzzleRenderDef, PuzzleScreenDef, PuzzleViewDef, ResourceSelection, RuleAnimation,
    RuleAnimationTrigger, RuleEffect, SceneAlignDef, SceneAlignXDef, SceneAlignYDef, SceneBinaryOp,
    SceneButtonDef, SceneComponent, SceneConditionalDef, SceneContainerDef, SceneDef, SceneEffect,
    SceneEffectParam, SceneExpr, SceneForDef, SceneLayoutDef, ScenePuzzleDef,
    ScenePuzzleInitializer, ScenePuzzleRule, SceneResources, SceneRoutineDef, SceneSizeDef,
    SceneStateDef, SceneStateLifetime, SceneTextContent, SceneTextDef, SceneTitleDef,
    SceneTransition, SceneTransitionTrigger, SceneValue, SceneVarDef, SfxSoundDef, SoundsDef,
    ThemeDef, ThemeVariableDef, TweenAnimationDef, ViewportModeDef, ViewportSizeDef,
    VisualAliasDef, VisualColorDef, VisualSpriteDef, VisualSpriteKind, VisualSpriteOffset,
    VisualSpritePixelsPerCell, VisualsDef,
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

fn is_block_header_line(line: &str) -> bool {
    line.trim_end().ends_with('{')
}
pub use puzzle_3d::{
    ParseError3, ParsedPuzzle3, VisualFixtureAnimation3, VisualFixtureExportError3,
    export_visual_fixture_json, export_visual_fixture_json_with_title,
    export_visual_fixture_json_with_title_and_scenes,
    export_visual_fixture_json_with_title_scenes_and_animation, parse_puzzle3d,
};
use puzzle_core::{
    ComparisonOp, CompiledGame, ConditionDef, ConditionId, ConditionValueKind, Effect, GapTerm,
    GlobalId, GlobalUpdateOp, Guard, InputId, LayerId, LocalFrame, LocalFrameExtent, MarkDef,
    MarkId, MarkKind, MarkPattern, MarkValueMatch, MatchCell, ObjectDef, ObjectId,
    ObjectSetMarkPattern, ObjectSetMatcher, Offset, Pattern, PatternComponent, Rule,
    RuleApplication, RuleCondition, RuleId, RuleStep, WriteOp,
};
pub use puzzlescript::translate_puzzlescript_to_canonical;
pub use semantic::{SemanticKind, SemanticToken, semantic_tokens};
use source::{
    SourceScope, SourceToken, logical_lines, logical_lines_with_locations, scan_source_context,
    source_line_tokens, split_header_tokens, strip_line_comment,
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
include!("lib_authoring_parse.rs");
include!("lib_lowering.rs");
include!("lib_patterns.rs");

#[cfg(test)]
mod tests;
