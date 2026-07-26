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
mod puzzle3_visual;
mod puzzle3_visual_fixture;
mod puzzlescript;
mod rule_syntax;
mod semantic;
mod solver_surface;
mod source;
mod source_analysis;
mod source_folding;
mod source_import;
mod source_level_edit;
mod source_outline;
mod source_sound_edit;
mod source_target;
mod source_visual_edit;
mod spatial_materialize2;
mod spatial_materialize3;
mod spatial_orientation;
mod surface;
mod surface_completion;
mod syntax;
mod visual_authoring;
mod visual_spatial;
mod workspace;

use solver_surface::{SolverSurfacePatternArg, SolverSurfaceQueryArg};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use puzzle_authoring::{SelectorMark, is_variable_update_operator};
pub use puzzle_scene::{
    STANDARD_MESSAGE_COMPONENT, STANDARD_MESSAGE_DISMISS_EVENT, STANDARD_MESSAGE_TEXT_PROPERTY,
    standard_message_effect,
};

use ast::{
    ConditionAst, ConditionDefinitionAst, ConditionPatternAst, ConditionValueAst, DirectionName,
    DirectionalInput, EffectAst, FixDefaults, OrientationExpr, OrientedRewriteAst,
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
    HighlightedSource, HighlightedSourceWithOutline, SourceHighlightColor, SourceHighlightKind,
    SourceHighlightSpan, highlight_source, highlight_source_with_outline,
};
use level::{LevelBlock, parse_level};
pub use loaded::{
    AnimationDef, ArrowKey, AsciiLegend, AssetDef, AssetKind, AssetsDef, ComponentDef,
    ComponentOrder, ComponentPlacement, ComponentProperty, ComponentVisibility, Controls,
    GoalClause, GoalClauseOf, GoalCondition, GoalConditionOf, GoalExpr, GoalExprOf, GoalValue,
    GoalValueOf, GridQueryExpr, GridSolverStrategy, InputBufferDef, KeyBinding, KeyTrigger, Level,
    LevelId, LevelRegionDef, LoadedDocument, LoadedDocumentModel, LoadedGame, LoadedGridGame,
    LoadedGridLevel, ModelOperationSound, ModelOperationSoundDef, MusicSoundDef, PuzzleGridMode,
    PuzzleRenderDef, PuzzleScreenDef, PuzzleViewDef, QueryExpr, QueryExprOf, ResourceSelection,
    RuleAnimation, RuleAnimationTrigger, RuleDebugInfo, RuleEffect, RuleVisualRewrite,
    RuntimeEffect, SceneAlignDef, SceneAspectRatioDef, SceneBinaryOp, SceneButtonDef,
    SceneComponent, SceneConditionalDef, SceneContainerDef, SceneDef, SceneDistributionDef,
    SceneEffect, SceneEffectParam, SceneExpr, SceneLayoutDef, ScenePuzzleDef,
    ScenePuzzleInitializer, ScenePuzzleRule, SceneResources, SceneRoutineDef, SceneSpaceDef,
    SceneStateDef, SceneStateLifetime, SceneTextAlignDef, SceneTextContent, SceneTextDef,
    SceneTextRoleDef, SceneTransition, SceneTransitionTrigger, SceneValue, SceneVarDef,
    SceneVarKind, SfxSoundDef, SolverDeadendOf, SolverStrategy, SolverStrategyDirection,
    SolverStrategyOf, SolverStrategyTerm, SolverStrategyTermOf, SoundsDef, ThemeDef,
    ThemeVariableDef, TriggerAnimationDef, TriggerAnimationKind, TweenAnimationDef,
    ViewportModeDef, ViewportProjectionDef, ViewportSizeDef, VisualAliasDef, VisualColorDef,
    VisualDef, VisualFit, VisualFitMode, VisualFrameDef, VisualKind, VisualOrderDef,
    VisualOrderPriorityDef, VisualPixelsPerCell, VisualSampling, VisualSpace, VisualTransform,
    VisualsDef, scene_level_record_key,
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
    ComparisonOp, CompiledGame, ConditionId, Effect, GridConditionDef, GridConditionValueKind,
    GridGapTerm, GridGuard, GridMatchCell, GridOffset, GridPattern, GridPatternComponent, GridRule,
    GridRuleCondition, GridRuleStep, GridWriteOp, InputId, LayerId, LocalFrame, LocalFrameExtent,
    MarkDef, MarkId, MarkKind, MarkPattern, MarkValueMatch, ObjectDef, ObjectId,
    ObjectSetMarkPattern, ObjectSetMatcher, RuleApplication, RuleId, State, VariableId,
    VariableUpdateOp, WriteOp,
};
#[cfg(test)]
use puzzle_core::{ConditionValueKind, Guard, Offset};
use spatial_orientation::{OrientationEnvironment, SpatialDomain};

type CanonicalConditionDef = GridConditionDef<3>;
type CanonicalConditionValueKind = GridConditionValueKind<3>;
type CanonicalGoalCondition = GoalConditionOf<CanonicalConditionValueKind>;
type CanonicalGoalExpr = GoalExprOf<CanonicalConditionValueKind>;
type CanonicalGoalClause = GoalClauseOf<CanonicalConditionValueKind>;
type CanonicalGoalValue = GoalValueOf<CanonicalConditionValueKind>;
type CanonicalQueryExpr = QueryExprOf<ObjectId, CanonicalConditionValueKind, VariableId>;
type CanonicalSolverStrategy = SolverStrategyOf<CanonicalQueryExpr>;
type CanonicalSolverStrategyTerm = SolverStrategyTermOf<CanonicalQueryExpr>;
type CanonicalGapTerm = GridGapTerm<3>;
type CanonicalGuard = GridGuard<3>;
type CanonicalMatchCell = GridMatchCell<3>;
type CanonicalOffset = GridOffset<3>;
type CanonicalPattern = GridPattern<3>;
type CanonicalPatternComponent = GridPatternComponent<3>;
type CanonicalRule = GridRule<3>;
type CanonicalRuleCondition = GridRuleCondition<3>;
type CanonicalRuleStep = GridRuleStep<3>;
type CanonicalWriteOp = GridWriteOp<3>;
pub use puzzle3_model::{
    CameraProjection3, CameraSettings3, LightingSettings3, PixelateRenderSettings3,
    SpatialPresentation, ViewportFollow3, ViewportFraming3, ViewportHeight3, ViewportMode3,
    ViewportSettings3, VisualRenderSettings3,
};
pub use puzzle3_visual::{VoxelColor, VoxelFrame, VoxelVisual, VoxelVisualSet};
pub use puzzle3_visual_fixture::{
    VisualFixtureExportError, export_visual_fixture_json, export_visual_fixture_json_with_scenes,
    export_visual_fixture_json_with_title, export_visual_fixture_json_with_title_and_scenes,
    runtime_puzzle3_cells, runtime_puzzle3_resources, runtime_puzzle3_size,
};
pub use puzzlescript::translate_puzzlescript_to_canonical;
pub use semantic::{SemanticKind, SemanticToken, semantic_tokens};
use source::{
    SourceScope, SourceToken, logical_lines_with_locations, source_line_tokens,
    split_header_tokens, strip_line_comment,
};
pub use visual_spatial::{SpatialVisualAffine, evaluate_spatial_visual_transforms};
pub use workspace::{
    WorkspaceAnalysis, WorkspaceGraphDiagnostic, WorkspaceImportEdge, WorkspaceImportStatus,
    WorkspaceIndex, WorkspaceIndexDocument, WorkspacePath, WorkspacePresentationManifest,
    WorkspaceSourceDocument, loaded_document_presentation_manifest,
    workspace_presentation_manifest, workspace_presentation_manifest_from_document,
};

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
    let source = lines
        .first()
        .cloned()
        .unwrap_or_else(|| source::LogicalLine::new("level", 1));
    let parsed = parse_level(
        game,
        &source,
        &lines,
        Some(empty),
        char_objects,
        variable_defaults,
    )
    .value?;
    Ok((parsed.state, parsed.regions))
}
pub use source_analysis::{
    SourceAnalysis, SourceAnalysisEdit, SourceAnalysisEditResult, analyze_source,
    analyze_source_for_owner_dimension, analyze_source_json,
};
pub use source_import::{SourceImportDeclaration, SourceImportRange, SourceImportReference};
pub use source_level_edit::{LevelLegendDraft, LevelSourceRequest, LevelSourceResponse};
pub use source_outline::{SourceOutlineItem, source_outline, source_outline_json};
pub use source_sound_edit::{
    SoundDefinitionDraft, SoundDefinitionInspection, SoundDefinitionKind,
    SoundSourceMutationResult, SoundSourceRequest, SoundSourceResponse,
};
pub use source_target::{
    SoundSourceTargetKind, SourceLevelDocument, SourceLevelLegendEntry, SourceTarget,
    SourceTargetKind, SourceVisualColorAsset, SourceVisualDocument, SourceVisualPaletteEntry,
    SourceVisualShapeAsset, SourceVisualStatus, SourceVisualTarget, resolve_source_target,
    resolve_source_target_for_owner_dimension, source_entries_json, source_target_json,
};
pub use source_visual_edit::{VisualEditMutationResult, mutate_visual_source};
use surface::{
    SourceSpan, SurfaceDisplayFact, SurfaceDocument, SurfaceHighlightRanges, SurfaceNodeKind,
    SurfaceOptionBlock, SurfaceOutlineBlock, SurfaceRewriteEffect, SurfaceSceneEffect,
    SurfaceSemanticKind, SurfaceSemanticToken, SurfaceSink, SurfaceStructuralBlock,
    SurfaceStructuralBlockRole,
};
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

pub fn is_puzzle_source_path(path: impl AsRef<Path>) -> bool {
    path.as_ref().extension().and_then(|value| value.to_str()) == Some("puzzle")
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
