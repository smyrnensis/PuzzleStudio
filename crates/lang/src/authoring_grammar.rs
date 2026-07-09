use crate::{
    DiagnosticReport, block_header_text, is_block_close_line, is_block_header_line,
    source::SourceToken,
    surface::{SourceSpan, SurfaceSemanticKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringKind {
    Root,
    TweenConfig,
    PuzzleRenderConfig,
    PuzzleRenderGridConfig,
    Puzzle3Root,
    Puzzle3RenderConfig,
    Puzzle3CameraConfig,
    Puzzle3GridConfig,
    Puzzle3PixelateConfig,
    Puzzle3ViewportConfig,
    SoundsConfig,
    SfxSoundConfig,
    MusicSoundConfig,
    InputBufferConfig,
    ThemeConfig,
    AssetsConfig,
    SpritesConfig,
    SpriteConfig,
    LevelsConfig,
    LevelConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringRowKind {
    VarDeclaration,
    ConstDeclaration,
    PersistentVarDeclaration,
    PersistentConstDeclaration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringBody {
    None,
    Content(AuthoringContentKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringContentKind {
    AssetsEntries,
    SpriteEntries,
    LevelEntries,
    Level3Entries,
    RuleStatements,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringContentRowKind {
    AssetPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContentRowSpec {
    pub(crate) kind: AuthoringContentRowKind,
    pub(crate) parts: &'static [RowPartSpec],
    pub(crate) usage: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContentSpec {
    pub(crate) kind: AuthoringContentKind,
    pub(crate) syntax: ContentSyntax,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContentSyntax {
    Rows(&'static [ContentRowSpec]),
    Attachment(ContentAttachment),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContentAttachment {
    SpriteEntries,
    Levels,
    Levels3,
    RuleStatements,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringBlockRole {
    Visuals,
    LevelList,
    LevelEntry,
    Rules,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringSourceBlockSpec {
    pub(crate) surface: &'static str,
    pub(crate) content: Option<AuthoringContentKind>,
    pub(crate) role: AuthoringBlockRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KindSpec {
    pub(crate) kind: AuthoringKind,
    pub(crate) header: HeaderSpec,
    pub(crate) definitions: &'static [DefinitionSpec],
    pub(crate) rows: &'static [RowSpec],
    pub(crate) body: AuthoringBody,
    pub(crate) symbol_exports: &'static [AuthoringSymbolExportSpec],
    pub(crate) block_role: Option<AuthoringBlockRole>,
    pub(crate) keyword_role: AuthoringSurfaceRole,
    pub(crate) outline_policy: AuthoringOutlinePolicy,
    pub(crate) missing_close_message: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionSpec {
    pub(crate) surface: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) values: DefinitionValueSpec,
    pub(crate) value_syntax: DefinitionValueSyntax,
    pub(crate) multiline_syntax: Option<DefinitionMultilineSyntax>,
    pub(crate) value_domain: DefinitionValueDomain,
    pub(crate) key_role: AuthoringSurfaceRole,
    pub(crate) value_role: Option<AuthoringSurfaceRole>,
    pub(crate) value_source: DefinitionValueSource,
}

impl DefinitionSpec {
    const fn value_role(
        surface: &'static str,
        values: DefinitionValueSpec,
        value_syntax: DefinitionValueSyntax,
        value_role: AuthoringSurfaceRole,
    ) -> Self {
        Self {
            surface,
            aliases: &[],
            values,
            value_syntax,
            multiline_syntax: None,
            value_domain: DefinitionValueDomain::None,
            key_role: AuthoringSurfaceRole::Setting,
            value_role: Some(value_role),
            value_source: DefinitionValueSource::Local,
        }
    }

    const fn keyed_value_role(
        surface: &'static str,
        values: DefinitionValueSpec,
        value_syntax: DefinitionValueSyntax,
        key_role: AuthoringSurfaceRole,
        value_role: AuthoringSurfaceRole,
    ) -> Self {
        Self {
            surface,
            aliases: &[],
            values,
            value_syntax,
            multiline_syntax: None,
            value_domain: DefinitionValueDomain::None,
            key_role,
            value_role: Some(value_role),
            value_source: DefinitionValueSource::Local,
        }
    }

    const fn typed_domain(
        surface: &'static str,
        values: DefinitionValueSpec,
        value_syntax: DefinitionValueSyntax,
        value_domain: DefinitionValueDomain,
        value_role: AuthoringSurfaceRole,
    ) -> Self {
        Self {
            surface,
            aliases: &[],
            values,
            value_syntax,
            multiline_syntax: None,
            value_domain,
            key_role: AuthoringSurfaceRole::Setting,
            value_role: Some(value_role),
            value_source: DefinitionValueSource::Local,
        }
    }

    const fn aliases(
        surface: &'static str,
        aliases: &'static [&'static str],
        values: DefinitionValueSpec,
        value_syntax: DefinitionValueSyntax,
        value_role: AuthoringSurfaceRole,
    ) -> Self {
        Self {
            surface,
            aliases,
            values,
            value_syntax,
            multiline_syntax: None,
            value_domain: DefinitionValueDomain::None,
            key_role: AuthoringSurfaceRole::Setting,
            value_role: Some(value_role),
            value_source: DefinitionValueSource::Local,
        }
    }

    const fn mirror(
        surface: &'static str,
        key_role: AuthoringSurfaceRole,
        target_kind: AuthoringKind,
        target_surface: &'static str,
    ) -> Self {
        Self {
            surface,
            aliases: &[],
            values: DefinitionValueSpec::None,
            value_syntax: DefinitionValueSyntax::Any,
            multiline_syntax: None,
            value_domain: DefinitionValueDomain::None,
            key_role,
            value_role: None,
            value_source: DefinitionValueSource::Mirror {
                kind: target_kind,
                surface: target_surface,
            },
        }
    }

    const fn multiline_value_role(
        surface: &'static str,
        values: DefinitionValueSpec,
        value_syntax: DefinitionValueSyntax,
        multiline_syntax: DefinitionMultilineSyntax,
        value_role: AuthoringSurfaceRole,
    ) -> Self {
        Self {
            surface,
            aliases: &[],
            values,
            value_syntax,
            multiline_syntax: Some(multiline_syntax),
            value_domain: DefinitionValueDomain::None,
            key_role: AuthoringSurfaceRole::Setting,
            value_role: Some(value_role),
            value_source: DefinitionValueSource::Local,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringSurfaceRole {
    // Universal authoring nodes project editor surface tokens from schema metadata:
    // node keywords use Keyword, header args use their declared role, definition keys
    // use Setting unless the definition overrides it, and values use the definition
    // value role. Owner attachments may add their own projection after the tree pass.
    Keyword,
    Setting,
    Object,
    State,
    Theme,
    Asset,
    String,
    Color,
    Number,
    Literal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringSymbolExportSpec {
    pub(crate) source: AuthoringSymbolExportSource,
    pub(crate) target: AuthoringSymbolExportTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringSymbolExportSource {
    HeaderArg(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringSymbolExportTarget {
    Sfx,
    Music,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringContentRowSurface {
    pub(crate) surface: &'static str,
    pub(crate) role: AuthoringSurfaceRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowSpec {
    pub(crate) kind: AuthoringRowKind,
    pub(crate) parts: &'static [RowPartSpec],
    pub(crate) usage: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowPartSpec {
    Keyword {
        surface: &'static str,
        role: AuthoringSurfaceRole,
    },
    Slot {
        name: &'static str,
        role: AuthoringSurfaceRole,
    },
    Equals,
    Rest {
        name: &'static str,
        role: AuthoringSurfaceRole,
    },
}

const fn row_keyword(surface: &'static str) -> RowPartSpec {
    RowPartSpec::Keyword {
        surface,
        role: AuthoringSurfaceRole::Keyword,
    }
}

const fn row_slot(name: &'static str, role: AuthoringSurfaceRole) -> RowPartSpec {
    RowPartSpec::Slot { name, role }
}

const fn row_rest(name: &'static str, role: AuthoringSurfaceRole) -> RowPartSpec {
    RowPartSpec::Rest { name, role }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringRow {
    pub(crate) kind: AuthoringRowKind,
    pub(crate) captures: Vec<AuthoringRowCapture>,
    pub(crate) source_line: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringRowCapture {
    pub(crate) name: &'static str,
    pub(crate) values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringContentRow {
    pub(crate) kind: AuthoringContentRowKind,
    pub(crate) captures: Vec<AuthoringRowCapture>,
    pub(crate) source_line: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum AuthoringOutlinePolicy {
    Hidden,
    Visible,
    CollapseChildren,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringSurfaceSpan {
    pub(crate) span: SourceSpan,
    pub(crate) role: AuthoringSurfaceRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefinitionValueSource {
    Local,
    Mirror {
        kind: AuthoringKind,
        surface: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DefinitionValueSpec {
    None,
    One,
    Many,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DefinitionValueSyntax {
    Any,
    Atom,
    Identifier,
    QuotedString,
    Color,
    Duration,
    Boolean,
    Number,
    PathString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DefinitionMultilineSyntax {
    Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DefinitionValueDomain {
    None,
    Builtin(DefinitionBuiltinDomain),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum DefinitionBuiltinDomain {
    ThemePreset,
    PuzzleRenderGridType,
    Puzzle3GridType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeaderSpec {
    pub(crate) min_args: usize,
    pub(crate) max_args: usize,
    pub(crate) usage: &'static str,
    pub(crate) arg_roles: &'static [AuthoringSurfaceRole],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlacementSpec {
    pub(crate) parent: AuthoringKind,
    pub(crate) surface: &'static str,
    pub(crate) child: AuthoringKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringNode {
    pub(crate) kind: AuthoringKind,
    pub(crate) surface: String,
    pub(crate) header_args: Vec<String>,
    pub(crate) definition_rows: Vec<AuthoringDefinitionRow>,
    pub(crate) rows: Vec<AuthoringRow>,
    pub(crate) children: Vec<AuthoringNode>,
    pub(crate) content_rows: Vec<AuthoringContentRow>,
    pub(crate) source_line: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoringDefinitionRow {
    pub(crate) key: String,
    pub(crate) op: Option<AuthoringDefinitionOp>,
    pub(crate) values: Vec<String>,
    pub(crate) value_kind: AuthoringDefinitionValueKind,
    pub(crate) source_line: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringDefinitionOp {
    Equals,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoringDefinitionValueKind {
    SingleLine,
    Multiline,
}

const NO_HEADER_ARGS: &[AuthoringSurfaceRole] = &[];
const ASSET_HEADER_ARG: &[AuthoringSurfaceRole] = &[AuthoringSurfaceRole::Asset];

const NO_DEFINITIONS: &[DefinitionSpec] = &[];
const NO_ROWS: &[RowSpec] = &[];
const NO_SYMBOL_EXPORTS: &[AuthoringSymbolExportSpec] = &[];
const ROOT_VARIABLE_NAME_SLOT: RowPartSpec = row_slot("name", AuthoringSurfaceRole::State);
const ROOT_VARIABLE_VALUE_REST: RowPartSpec = row_rest("value", AuthoringSurfaceRole::Literal);
const ROOT_VAR_ROW_PARTS: &[RowPartSpec] = &[
    row_keyword("var"),
    ROOT_VARIABLE_NAME_SLOT,
    RowPartSpec::Equals,
    ROOT_VARIABLE_VALUE_REST,
];
const ROOT_CONST_ROW_PARTS: &[RowPartSpec] = &[
    row_keyword("const"),
    ROOT_VARIABLE_NAME_SLOT,
    RowPartSpec::Equals,
    ROOT_VARIABLE_VALUE_REST,
];
const ROOT_PERSISTENT_VAR_ROW_PARTS: &[RowPartSpec] = &[
    row_keyword("persistent"),
    row_keyword("var"),
    ROOT_VARIABLE_NAME_SLOT,
    RowPartSpec::Equals,
    ROOT_VARIABLE_VALUE_REST,
];
const ROOT_PERSISTENT_CONST_ROW_PARTS: &[RowPartSpec] = &[
    row_keyword("persistent"),
    row_keyword("const"),
    ROOT_VARIABLE_NAME_SLOT,
    RowPartSpec::Equals,
    ROOT_VARIABLE_VALUE_REST,
];
const ROOT_ROWS: &[RowSpec] = &[
    RowSpec {
        kind: AuthoringRowKind::VarDeclaration,
        parts: ROOT_VAR_ROW_PARTS,
        usage: "var <name> = <literal>",
    },
    RowSpec {
        kind: AuthoringRowKind::ConstDeclaration,
        parts: ROOT_CONST_ROW_PARTS,
        usage: "const <name> = <literal>",
    },
    RowSpec {
        kind: AuthoringRowKind::PersistentVarDeclaration,
        parts: ROOT_PERSISTENT_VAR_ROW_PARTS,
        usage: "persistent var <name> = <literal>",
    },
    RowSpec {
        kind: AuthoringRowKind::PersistentConstDeclaration,
        parts: ROOT_PERSISTENT_CONST_ROW_PARTS,
        usage: "persistent const <name> = <literal>",
    },
];
const ASSETS_PATH_ROW_PARTS: &[RowPartSpec] = &[RowPartSpec::Slot {
    name: "path",
    role: AuthoringSurfaceRole::String,
}];
const ASSETS_ENTRY_ROWS: &[ContentRowSpec] = &[ContentRowSpec {
    kind: AuthoringContentRowKind::AssetPath,
    parts: ASSETS_PATH_ROW_PARTS,
    usage: "<string>",
}];
const CONTENT_SPECS: &[ContentSpec] = &[
    ContentSpec {
        kind: AuthoringContentKind::AssetsEntries,
        syntax: ContentSyntax::Rows(ASSETS_ENTRY_ROWS),
    },
    ContentSpec {
        kind: AuthoringContentKind::SpriteEntries,
        syntax: ContentSyntax::Attachment(ContentAttachment::SpriteEntries),
    },
    ContentSpec {
        kind: AuthoringContentKind::LevelEntries,
        syntax: ContentSyntax::Attachment(ContentAttachment::Levels),
    },
    ContentSpec {
        kind: AuthoringContentKind::Level3Entries,
        syntax: ContentSyntax::Attachment(ContentAttachment::Levels3),
    },
    ContentSpec {
        kind: AuthoringContentKind::RuleStatements,
        syntax: ContentSyntax::Attachment(ContentAttachment::RuleStatements),
    },
];
const AUTHORING_SOURCE_BLOCK_SPECS: &[AuthoringSourceBlockSpec] = &[
    AuthoringSourceBlockSpec {
        surface: "sprites",
        content: Some(AuthoringContentKind::SpriteEntries),
        role: AuthoringBlockRole::Visuals,
    },
    AuthoringSourceBlockSpec {
        surface: "sprite",
        content: None,
        role: AuthoringBlockRole::Visuals,
    },
    AuthoringSourceBlockSpec {
        surface: "sprites3",
        content: None,
        role: AuthoringBlockRole::Visuals,
    },
    AuthoringSourceBlockSpec {
        surface: "levels",
        content: None,
        role: AuthoringBlockRole::LevelList,
    },
    AuthoringSourceBlockSpec {
        surface: "level",
        content: Some(AuthoringContentKind::LevelEntries),
        role: AuthoringBlockRole::LevelEntry,
    },
    AuthoringSourceBlockSpec {
        surface: "levels3",
        content: None,
        role: AuthoringBlockRole::LevelList,
    },
    AuthoringSourceBlockSpec {
        surface: "rules",
        content: Some(AuthoringContentKind::RuleStatements),
        role: AuthoringBlockRole::Rules,
    },
];
const ROOT_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::keyed_value_role(
        "title",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::keyed_value_role(
        "subtitle",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::keyed_value_role(
        "author",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::keyed_value_role(
        "homepage",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::keyed_value_role(
        "default_wait_time",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Duration,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::keyed_value_role(
        "again_interval",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Duration,
        AuthoringSurfaceRole::Keyword,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::mirror(
        "theme",
        AuthoringSurfaceRole::Keyword,
        AuthoringKind::ThemeConfig,
        "preset",
    ),
];
const TWEEN_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[DefinitionSpec::value_role(
    "duration",
    DefinitionValueSpec::One,
    DefinitionValueSyntax::Duration,
    AuthoringSurfaceRole::Number,
)];
const PUZZLE_RENDER_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "cell_size",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "tween_duration",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Duration,
        AuthoringSurfaceRole::Number,
    ),
];
const PUZZLE_RENDER_GRID_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[DefinitionSpec::typed_domain(
    "type",
    DefinitionValueSpec::One,
    DefinitionValueSyntax::QuotedString,
    DefinitionValueDomain::Builtin(DefinitionBuiltinDomain::PuzzleRenderGridType),
    AuthoringSurfaceRole::Literal,
)];
const PUZZLE3_RENDER_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[DefinitionSpec::value_role(
    "shade",
    DefinitionValueSpec::One,
    DefinitionValueSyntax::Boolean,
    AuthoringSurfaceRole::Literal,
)];
const PUZZLE3_CAMERA_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "yaw",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "pitch",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "zoom",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "interactive_look",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
    DefinitionSpec::value_role(
        "interactive_zoom",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
];
const PUZZLE3_GRID_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[DefinitionSpec::typed_domain(
    "type",
    DefinitionValueSpec::One,
    DefinitionValueSyntax::QuotedString,
    DefinitionValueDomain::Builtin(DefinitionBuiltinDomain::Puzzle3GridType),
    AuthoringSurfaceRole::Literal,
)];
const PUZZLE3_PIXELATE_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "enabled",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
    DefinitionSpec::value_role(
        "scale",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "smoothing",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
];
const SFX_SOUND_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "seed",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Atom,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::value_role(
        "type",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Atom,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::value_role(
        "volume",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
];
const MUSIC_SOUND_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "seed",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Atom,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::value_role(
        "height",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "tone",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "bars",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "bpm",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "volume",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Number,
        AuthoringSurfaceRole::Number,
    ),
];
const SFX_SOUND_SYMBOL_EXPORTS: &[AuthoringSymbolExportSpec] = &[AuthoringSymbolExportSpec {
    source: AuthoringSymbolExportSource::HeaderArg(0),
    target: AuthoringSymbolExportTarget::Sfx,
}];
const MUSIC_SOUND_SYMBOL_EXPORTS: &[AuthoringSymbolExportSpec] = &[AuthoringSymbolExportSpec {
    source: AuthoringSymbolExportSource::HeaderArg(0),
    target: AuthoringSymbolExportTarget::Music,
}];
const INPUT_BUFFER_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::value_role(
        "queue_during_wait",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
    DefinitionSpec::value_role(
        "fast_forward_wait",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Boolean,
        AuthoringSurfaceRole::Literal,
    ),
    DefinitionSpec::value_role(
        "min_wait",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Duration,
        AuthoringSurfaceRole::Number,
    ),
];
const THEME_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::typed_domain(
        "preset",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::QuotedString,
        DefinitionValueDomain::Builtin(DefinitionBuiltinDomain::ThemePreset),
        AuthoringSurfaceRole::Theme,
    ),
    DefinitionSpec::aliases(
        "accent_color",
        &["accent-color", "accent", "--accent"],
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Color,
        AuthoringSurfaceRole::Color,
    ),
    DefinitionSpec::aliases(
        "background_color",
        &["background-color", "background", "bg", "--background"],
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Color,
        AuthoringSurfaceRole::Color,
    ),
    DefinitionSpec::aliases(
        "text_color",
        &["text-color", "text", "ink", "--text"],
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Color,
        AuthoringSurfaceRole::Color,
    ),
];
const SPRITE_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[
    DefinitionSpec::keyed_value_role(
        "selector",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Setting,
        AuthoringSurfaceRole::Object,
    ),
    DefinitionSpec::value_role(
        "colors",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Color,
        AuthoringSurfaceRole::Color,
    ),
    DefinitionSpec::value_role(
        "image",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::QuotedString,
        AuthoringSurfaceRole::String,
    ),
    DefinitionSpec::value_role(
        "offset",
        DefinitionValueSpec::Many,
        DefinitionValueSyntax::Any,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::value_role(
        "sampling",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Atom,
        AuthoringSurfaceRole::Literal,
    ),
    DefinitionSpec::value_role(
        "duration",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Duration,
        AuthoringSurfaceRole::Number,
    ),
    DefinitionSpec::multiline_value_role(
        "shape",
        DefinitionValueSpec::One,
        DefinitionValueSyntax::Any,
        DefinitionMultilineSyntax::Lines,
        AuthoringSurfaceRole::Literal,
    ),
];
const LEVEL_CONFIG_DEFINITIONS: &[DefinitionSpec] = &[DefinitionSpec::keyed_value_role(
    "name",
    DefinitionValueSpec::One,
    DefinitionValueSyntax::QuotedString,
    AuthoringSurfaceRole::Setting,
    AuthoringSurfaceRole::String,
)];
const ROOT_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "root",
    arg_roles: NO_HEADER_ARGS,
};
const TWEEN_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "tween",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE_RENDER_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "render",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE_RENDER_GRID_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "grid",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_ROOT_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "puzzle3",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_RENDER_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "render",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_CAMERA_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "camera",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_GRID_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "grid",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_PIXELATE_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "pixelate",
    arg_roles: NO_HEADER_ARGS,
};
const PUZZLE3_VIEWPORT_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "viewport",
    arg_roles: NO_HEADER_ARGS,
};
const SOUNDS_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "sounds",
    arg_roles: NO_HEADER_ARGS,
};
const NAMED_SFX_HEADER: HeaderSpec = HeaderSpec {
    min_args: 1,
    max_args: 1,
    usage: "sfx <name>",
    arg_roles: ASSET_HEADER_ARG,
};
const NAMED_MUSIC_HEADER: HeaderSpec = HeaderSpec {
    min_args: 1,
    max_args: 1,
    usage: "music <name>",
    arg_roles: ASSET_HEADER_ARG,
};
const INPUT_BUFFER_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "input_buffer",
    arg_roles: NO_HEADER_ARGS,
};
const THEME_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "theme",
    arg_roles: NO_HEADER_ARGS,
};
const ASSETS_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "assets",
    arg_roles: NO_HEADER_ARGS,
};
const SPRITES_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "sprites",
    arg_roles: NO_HEADER_ARGS,
};
const SPRITE_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "sprite",
    arg_roles: NO_HEADER_ARGS,
};
const LEVELS_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "levels",
    arg_roles: NO_HEADER_ARGS,
};
const LEVEL_HEADER: HeaderSpec = HeaderSpec {
    min_args: 0,
    max_args: 0,
    usage: "level",
    arg_roles: NO_HEADER_ARGS,
};

pub(crate) const KIND_SPECS: &[KindSpec] = &[
    KindSpec {
        kind: AuthoringKind::Root,
        header: ROOT_HEADER,
        definitions: ROOT_DEFINITIONS,
        rows: ROOT_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Hidden,
        missing_close_message: "block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::TweenConfig,
        header: TWEEN_HEADER,
        definitions: TWEEN_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "tween block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::PuzzleRenderConfig,
        header: PUZZLE_RENDER_HEADER,
        definitions: PUZZLE_RENDER_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "render block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::PuzzleRenderGridConfig,
        header: PUZZLE_RENDER_GRID_HEADER,
        definitions: PUZZLE_RENDER_GRID_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "grid block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3Root,
        header: PUZZLE3_ROOT_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Hidden,
        missing_close_message: "puzzle3 block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3RenderConfig,
        header: PUZZLE3_RENDER_HEADER,
        definitions: PUZZLE3_RENDER_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "render block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3CameraConfig,
        header: PUZZLE3_CAMERA_HEADER,
        definitions: PUZZLE3_CAMERA_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "camera block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3GridConfig,
        header: PUZZLE3_GRID_HEADER,
        definitions: PUZZLE3_GRID_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "grid block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3PixelateConfig,
        header: PUZZLE3_PIXELATE_HEADER,
        definitions: PUZZLE3_PIXELATE_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "pixelate block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::Puzzle3ViewportConfig,
        header: PUZZLE3_VIEWPORT_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "viewport block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::SoundsConfig,
        header: SOUNDS_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "sounds missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::SfxSoundConfig,
        header: NAMED_SFX_HEADER,
        definitions: SFX_SOUND_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: SFX_SOUND_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "sfx sound block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::MusicSoundConfig,
        header: NAMED_MUSIC_HEADER,
        definitions: MUSIC_SOUND_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: MUSIC_SOUND_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "music sound block missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::InputBufferConfig,
        header: INPUT_BUFFER_HEADER,
        definitions: INPUT_BUFFER_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "input_buffer missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::ThemeConfig,
        header: THEME_HEADER,
        definitions: THEME_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "theme missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::AssetsConfig,
        header: ASSETS_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::Content(AuthoringContentKind::AssetsEntries),
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: None,
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "assets missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::SpritesConfig,
        header: SPRITES_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::Content(AuthoringContentKind::SpriteEntries),
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: Some(AuthoringBlockRole::Visuals),
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "sprites missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::SpriteConfig,
        header: SPRITE_HEADER,
        definitions: SPRITE_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: Some(AuthoringBlockRole::Visuals),
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "sprite missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::LevelsConfig,
        header: LEVELS_HEADER,
        definitions: NO_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::None,
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: Some(AuthoringBlockRole::LevelList),
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "levels missing closing brace",
    },
    KindSpec {
        kind: AuthoringKind::LevelConfig,
        header: LEVEL_HEADER,
        definitions: LEVEL_CONFIG_DEFINITIONS,
        rows: NO_ROWS,
        body: AuthoringBody::Content(AuthoringContentKind::LevelEntries),
        symbol_exports: NO_SYMBOL_EXPORTS,
        block_role: Some(AuthoringBlockRole::LevelEntry),
        keyword_role: AuthoringSurfaceRole::Keyword,
        outline_policy: AuthoringOutlinePolicy::Visible,
        missing_close_message: "level missing closing brace",
    },
];

pub(crate) const PLACEMENT_SPECS: &[PlacementSpec] = &[
    PlacementSpec {
        parent: AuthoringKind::PuzzleRenderConfig,
        surface: "tween",
        child: AuthoringKind::TweenConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "render",
        child: AuthoringKind::PuzzleRenderConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::PuzzleRenderConfig,
        surface: "grid",
        child: AuthoringKind::PuzzleRenderGridConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Puzzle3Root,
        surface: "render",
        child: AuthoringKind::Puzzle3RenderConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Puzzle3RenderConfig,
        surface: "camera",
        child: AuthoringKind::Puzzle3CameraConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Puzzle3RenderConfig,
        surface: "grid",
        child: AuthoringKind::Puzzle3GridConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Puzzle3RenderConfig,
        surface: "pixelate",
        child: AuthoringKind::Puzzle3PixelateConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Puzzle3RenderConfig,
        surface: "viewport",
        child: AuthoringKind::Puzzle3ViewportConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "sounds",
        child: AuthoringKind::SoundsConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::SoundsConfig,
        surface: "sfx",
        child: AuthoringKind::SfxSoundConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::SoundsConfig,
        surface: "music",
        child: AuthoringKind::MusicSoundConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "input_buffer",
        child: AuthoringKind::InputBufferConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "theme",
        child: AuthoringKind::ThemeConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "assets",
        child: AuthoringKind::AssetsConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "sprites",
        child: AuthoringKind::SpritesConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::SpritesConfig,
        surface: "sprite",
        child: AuthoringKind::SpriteConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::Root,
        surface: "levels",
        child: AuthoringKind::LevelsConfig,
    },
    PlacementSpec {
        parent: AuthoringKind::LevelsConfig,
        surface: "level",
        child: AuthoringKind::LevelConfig,
    },
];

pub(crate) fn authoring_kind_spec(kind: AuthoringKind) -> &'static KindSpec {
    KIND_SPECS
        .iter()
        .find(|spec| spec.kind == kind)
        .expect("authoring kind has a spec")
}

pub(crate) fn authoring_child_surfaces(parent: AuthoringKind) -> Vec<&'static str> {
    PLACEMENT_SPECS
        .iter()
        .filter(|placement| placement.parent == parent)
        .map(|placement| placement.surface)
        .collect()
}

pub(crate) fn authoring_definition_specs(kind: AuthoringKind) -> &'static [DefinitionSpec] {
    authoring_kind_spec(kind).definitions
}

pub(crate) fn authoring_row_specs(kind: AuthoringKind) -> &'static [RowSpec] {
    authoring_kind_spec(kind).rows
}

pub(crate) fn authoring_symbol_exports(
    kind: AuthoringKind,
) -> &'static [AuthoringSymbolExportSpec] {
    authoring_kind_spec(kind).symbol_exports
}

pub(crate) fn authoring_block_role(kind: AuthoringKind) -> Option<AuthoringBlockRole> {
    authoring_kind_spec(kind).block_role
}

pub(crate) fn authoring_surface_role_semantic_kind(
    role: AuthoringSurfaceRole,
) -> SurfaceSemanticKind {
    match role {
        AuthoringSurfaceRole::Keyword => SurfaceSemanticKind::Keyword,
        AuthoringSurfaceRole::Setting => SurfaceSemanticKind::Setting,
        AuthoringSurfaceRole::Object => SurfaceSemanticKind::Object,
        AuthoringSurfaceRole::State => SurfaceSemanticKind::State,
        AuthoringSurfaceRole::Theme => SurfaceSemanticKind::Theme,
        AuthoringSurfaceRole::Asset => SurfaceSemanticKind::Asset,
        AuthoringSurfaceRole::String => SurfaceSemanticKind::String,
        AuthoringSurfaceRole::Color => SurfaceSemanticKind::Color,
        AuthoringSurfaceRole::Number => SurfaceSemanticKind::Number,
        AuthoringSurfaceRole::Literal => SurfaceSemanticKind::Literal,
    }
}

pub(crate) fn authoring_content_spec(content: AuthoringContentKind) -> &'static ContentSpec {
    CONTENT_SPECS
        .iter()
        .find(|spec| spec.kind == content)
        .expect("authoring content has a spec")
}

pub(crate) fn authoring_content_syntax(content: AuthoringContentKind) -> ContentSyntax {
    authoring_content_spec(content).syntax
}

pub(crate) fn authoring_kind_content(kind: AuthoringKind) -> Option<AuthoringContentKind> {
    match authoring_kind_spec(kind).body {
        AuthoringBody::None => None,
        AuthoringBody::Content(content) => Some(content),
    }
}

pub(crate) fn authoring_kind_content_attachment(kind: AuthoringKind) -> Option<ContentAttachment> {
    let content = authoring_kind_content(kind)?;
    match authoring_content_syntax(content) {
        ContentSyntax::Rows(_) => None,
        ContentSyntax::Attachment(attachment) => Some(attachment),
    }
}

pub(crate) fn authoring_source_block(surface: &str) -> Option<&'static AuthoringSourceBlockSpec> {
    AUTHORING_SOURCE_BLOCK_SPECS
        .iter()
        .find(|spec| spec.surface == surface)
}

pub(crate) fn authoring_content_row_specs(
    content: AuthoringContentKind,
) -> Option<&'static [ContentRowSpec]> {
    match authoring_content_syntax(content) {
        ContentSyntax::Rows(rows) => Some(rows),
        ContentSyntax::Attachment(_) => None,
    }
}

pub(crate) fn authoring_content_row_surfaces(
    content: AuthoringContentKind,
) -> Vec<AuthoringContentRowSurface> {
    let Some(rows) = authoring_content_row_specs(content) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|spec| match spec.parts.first()? {
            RowPartSpec::Keyword { surface, role } => Some(AuthoringContentRowSurface {
                surface,
                role: *role,
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn authoring_definition_surfaces(kind: AuthoringKind) -> Vec<&'static str> {
    authoring_definition_specs(kind)
        .iter()
        .map(|definition| definition.surface)
        .collect()
}

pub(crate) fn authoring_row_surfaces(kind: AuthoringKind) -> Vec<&'static str> {
    let mut surfaces = Vec::<&'static str>::new();
    for spec in authoring_row_specs(kind) {
        if let Some(surface) = row_spec_first_keyword(spec) {
            if !surfaces.contains(&surface) {
                surfaces.push(surface);
            }
        }
    }
    surfaces
}

pub(crate) fn authoring_head_surface(kind: AuthoringKind, surface: &str) -> bool {
    placed_authoring_kind(kind, surface).is_some()
        || authoring_definition_spec(kind, surface).is_some()
        || authoring_row_surfaces(kind).contains(&surface)
}

pub(crate) fn authoring_capture_values<'a>(
    captures: &'a [AuthoringRowCapture],
    name: &str,
) -> Option<&'a [String]> {
    captures
        .iter()
        .find(|capture| capture.name == name)
        .map(|capture| capture.values.as_slice())
}

pub(crate) fn authoring_capture_first<'a>(
    captures: &'a [AuthoringRowCapture],
    name: &str,
) -> Option<&'a str> {
    authoring_capture_values(captures, name)
        .and_then(|values| values.first())
        .map(String::as_str)
}

pub(crate) fn authoring_definition_spec(
    kind: AuthoringKind,
    surface: &str,
) -> Option<&'static DefinitionSpec> {
    authoring_definition_specs(kind)
        .iter()
        .find(|definition| authoring_definition_matches_surface(definition, surface))
}

pub(crate) fn authoring_definition_matches_surface(
    definition: &DefinitionSpec,
    surface: &str,
) -> bool {
    definition.surface == surface || definition.aliases.contains(&surface)
}

fn definition_value_target(spec: &DefinitionSpec) -> &DefinitionSpec {
    match spec.value_source {
        DefinitionValueSource::Local => spec,
        DefinitionValueSource::Mirror { kind, surface } => authoring_definition_spec(kind, surface)
            .expect("mirrored authoring definition target exists"),
    }
}

pub(crate) fn definition_values(spec: &DefinitionSpec) -> DefinitionValueSpec {
    definition_value_target(spec).values
}

pub(crate) fn definition_value_syntax(spec: &DefinitionSpec) -> DefinitionValueSyntax {
    definition_value_target(spec).value_syntax
}

pub(crate) fn definition_multiline_syntax(
    spec: &DefinitionSpec,
) -> Option<DefinitionMultilineSyntax> {
    definition_value_target(spec).multiline_syntax
}

pub(crate) fn definition_value_domain(spec: &DefinitionSpec) -> DefinitionValueDomain {
    definition_value_target(spec).value_domain
}

pub(crate) fn definition_value_role(spec: &DefinitionSpec) -> Option<AuthoringSurfaceRole> {
    definition_value_target(spec).value_role
}

pub(crate) fn project_authoring_header_surface(
    kind: AuthoringKind,
    tokens: &[SourceToken],
) -> Vec<AuthoringSurfaceSpan> {
    let spec = authoring_kind_spec(kind);
    let mut spans = Vec::<AuthoringSurfaceSpan>::new();
    if let Some(keyword) = tokens.first() {
        spans.push(AuthoringSurfaceSpan {
            span: token_span(keyword),
            role: spec.keyword_role,
        });
    }
    for (index, role) in spec.header.arg_roles.iter().enumerate() {
        if let Some(token) = tokens.get(index + 1) {
            spans.push(AuthoringSurfaceSpan {
                span: token_span(token),
                role: *role,
            });
        }
    }
    spans
}

pub(crate) fn project_authoring_definition_surface(
    kind: AuthoringKind,
    tokens: &[SourceToken],
) -> Vec<AuthoringSurfaceSpan> {
    let Some((key_index, key_surface, key_start, key_end)) =
        authoring_definition_key_surface(tokens)
    else {
        return Vec::new();
    };
    let Some(spec) = authoring_definition_spec(kind, key_surface) else {
        return Vec::new();
    };
    let mut spans = vec![AuthoringSurfaceSpan {
        span: SourceSpan {
            start: key_start,
            end: key_end,
        },
        role: spec.key_role,
    }];
    let Some(value_role) = definition_value_role(spec) else {
        return spans;
    };
    for span in authoring_definition_value_spans(tokens, key_index, key_surface, spec) {
        spans.push(AuthoringSurfaceSpan {
            span,
            role: value_role,
        });
    }
    spans
}

pub(crate) fn project_authoring_row_surface(
    kind: AuthoringKind,
    tokens: &[SourceToken],
) -> Vec<AuthoringSurfaceSpan> {
    let units = surface_units(tokens);
    let texts = units
        .iter()
        .map(|unit| unit.text.as_str())
        .collect::<Vec<_>>();
    for spec in authoring_row_specs(kind) {
        if let Some(spans) = project_row_spec_surface(spec, &texts, &units) {
            return spans;
        }
    }
    Vec::new()
}

pub(crate) fn project_authoring_content_surface(
    content: AuthoringContentKind,
    tokens: &[SourceToken],
) -> Vec<AuthoringSurfaceSpan> {
    let Some(rows) = authoring_content_row_specs(content) else {
        return Vec::new();
    };
    let units = surface_units(tokens);
    let texts = units
        .iter()
        .map(|unit| unit.text.as_str())
        .collect::<Vec<_>>();
    for spec in rows {
        if let Some(spans) = project_row_parts_surface(spec.parts, &texts, &units) {
            return spans;
        }
    }
    Vec::new()
}

fn authoring_definition_key_surface(tokens: &[SourceToken]) -> Option<(usize, &str, usize, usize)> {
    let token = tokens.first()?;
    let key_end = token.text.find('=').unwrap_or(token.text.len());
    if key_end == 0 {
        return None;
    }
    let key = &token.text[..key_end];
    Some((0, key, token.start, token.start + key_end))
}

fn authoring_definition_value_spans(
    tokens: &[SourceToken],
    key_index: usize,
    key_surface: &str,
    spec: &DefinitionSpec,
) -> Vec<SourceSpan> {
    let token = &tokens[key_index];
    if let Some((_, value)) = token.text.split_once('=') {
        return definition_value_span(token, value, spec)
            .into_iter()
            .collect();
    }
    let value_start = if token.text == key_surface
        && tokens
            .get(key_index + 1)
            .is_some_and(|token| token.text == "=")
    {
        key_index + 2
    } else {
        key_index + 1
    };
    tokens[value_start..]
        .iter()
        .flat_map(|token| definition_value_span(token, token.text.as_str(), spec))
        .collect()
}

fn definition_value_span(
    token: &SourceToken,
    value: &str,
    spec: &DefinitionSpec,
) -> Option<SourceSpan> {
    if value.is_empty() {
        return None;
    }
    let relative_start = token.text.find(value)?;
    let mut start = token.start + relative_start;
    let mut end = start + value.len();
    if definition_value_syntax(spec) == DefinitionValueSyntax::QuotedString
        && value.len() >= 2
        && value.starts_with('"')
        && value.ends_with('"')
    {
        start += 1;
        end -= 1;
    }
    (start < end).then_some(SourceSpan { start, end })
}

fn token_span(token: &SourceToken) -> SourceSpan {
    SourceSpan {
        start: token.start,
        end: token.end,
    }
}

#[derive(Clone, Debug)]
struct SurfaceUnit {
    text: String,
    span: SourceSpan,
}

fn surface_units(tokens: &[SourceToken]) -> Vec<SurfaceUnit> {
    let mut out = Vec::<SurfaceUnit>::new();
    for token in tokens {
        let mut start = 0;
        let mut in_quote = false;
        let mut escaped = false;
        for (index, ch) in token.text.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if in_quote && ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_quote = !in_quote;
                continue;
            }
            if !in_quote && ch == '=' {
                if start < index {
                    out.push(SurfaceUnit {
                        text: token.text[start..index].to_string(),
                        span: SourceSpan {
                            start: token.start + start,
                            end: token.start + index,
                        },
                    });
                }
                out.push(SurfaceUnit {
                    text: "=".to_string(),
                    span: SourceSpan {
                        start: token.start + index,
                        end: token.start + index + 1,
                    },
                });
                start = index + 1;
            }
        }
        if start < token.text.len() {
            out.push(SurfaceUnit {
                text: token.text[start..].to_string(),
                span: SourceSpan {
                    start: token.start + start,
                    end: token.end,
                },
            });
        }
    }
    out
}

fn project_row_spec_surface(
    spec: &RowSpec,
    texts: &[&str],
    units: &[SurfaceUnit],
) -> Option<Vec<AuthoringSurfaceSpan>> {
    project_row_parts_surface(spec.parts, texts, units)
}

fn project_row_parts_surface(
    parts: &[RowPartSpec],
    texts: &[&str],
    units: &[SurfaceUnit],
) -> Option<Vec<AuthoringSurfaceSpan>> {
    let mut spans = Vec::<AuthoringSurfaceSpan>::new();
    for row_match in match_row_parts(parts, texts)? {
        match row_match.part {
            RowPartSpec::Keyword { surface, role } => {
                let unit = units.get(row_match.start)?;
                debug_assert_eq!(unit.text, surface);
                spans.push(AuthoringSurfaceSpan {
                    span: unit.span,
                    role,
                });
            }
            RowPartSpec::Slot { role, .. } => {
                let unit = units.get(row_match.start)?;
                let span = if matches!(
                    role,
                    AuthoringSurfaceRole::Asset | AuthoringSurfaceRole::String
                ) {
                    quoted_inner_span(unit).unwrap_or(unit.span)
                } else {
                    unit.span
                };
                spans.push(AuthoringSurfaceSpan { span, role });
            }
            RowPartSpec::Equals => {}
            RowPartSpec::Rest { role, .. } => {
                for unit in &units[row_match.start..row_match.end] {
                    spans.push(AuthoringSurfaceSpan {
                        span: unit.span,
                        role,
                    });
                }
            }
        }
    }
    Some(spans)
}

fn quoted_inner_span(unit: &SurfaceUnit) -> Option<SourceSpan> {
    if unit.text.len() >= 2 && unit.text.starts_with('"') && unit.text.ends_with('"') {
        Some(SourceSpan {
            start: unit.span.start + 1,
            end: unit.span.end - 1,
        })
    } else {
        None
    }
}

pub(crate) fn definition_builtin_domain_values(
    domain: DefinitionBuiltinDomain,
) -> &'static [&'static str] {
    match domain {
        DefinitionBuiltinDomain::PuzzleRenderGridType => &["occupied_cells", "all_cells"],
        DefinitionBuiltinDomain::Puzzle3GridType => &["occupied_cells"],
        DefinitionBuiltinDomain::ThemePreset => crate::THEME_PRESET_NAMES,
    }
}

pub(crate) fn definition_value_literal<'a>(
    spec: &DefinitionSpec,
    value: &'a str,
    line: &str,
) -> Result<&'a str, DiagnosticReport> {
    match definition_value_syntax(spec) {
        DefinitionValueSyntax::QuotedString => value
            .strip_prefix('"')
            .and_then(|stripped| stripped.strip_suffix('"'))
            .ok_or_else(|| {
                DiagnosticReport::error_at_line(
                    format!("{} must be a quoted string", spec.surface),
                    line,
                )
            }),
        _ => Ok(value),
    }
}

pub(crate) fn validate_definition_value_domain(
    spec: &DefinitionSpec,
    value: &str,
    line: &str,
) -> Result<(), DiagnosticReport> {
    match definition_value_domain(spec) {
        DefinitionValueDomain::None => Ok(()),
        DefinitionValueDomain::Builtin(domain) => {
            let value = definition_value_literal(spec, value, line)?;
            let values = definition_builtin_domain_values(domain);
            if values.contains(&value) {
                Ok(())
            } else {
                Err(DiagnosticReport::error_at_line(
                    format!("{} must be one of: {}", spec.surface, values.join(", ")),
                    line,
                ))
            }
        }
    }
}

pub(crate) fn placed_authoring_kind(parent: AuthoringKind, surface: &str) -> Option<AuthoringKind> {
    PLACEMENT_SPECS
        .iter()
        .find(|placement| placement.parent == parent && placement.surface == surface)
        .map(|placement| placement.child)
}

pub(crate) fn parse_placed_authoring_node(
    lines: &[String],
    start: usize,
    parent: AuthoringKind,
    missing_close_message: &str,
) -> Result<(AuthoringNode, usize), DiagnosticReport> {
    let line = &lines[start];
    let header = split_authoring_tokens(block_header_text(line));
    let Some(surface) = header.first().map(String::as_str) else {
        return Err(DiagnosticReport::error_at_line("empty block header", line));
    };
    let Some(kind) = placed_authoring_kind(parent, surface) else {
        return Err(DiagnosticReport::error_at_line(
            format!("unknown authoring directive {surface}"),
            line,
        ));
    };
    parse_authoring_node_with_kind(lines, start, kind, missing_close_message)
}

pub(crate) fn parse_authoring_node_with_kind(
    lines: &[String],
    start: usize,
    kind: AuthoringKind,
    missing_close_message: &str,
) -> Result<(AuthoringNode, usize), DiagnosticReport> {
    let line = &lines[start];
    let header = split_authoring_tokens(block_header_text(line));
    let Some(surface) = header.first().map(String::as_str) else {
        return Err(DiagnosticReport::error_at_line("empty block header", line));
    };
    let mut node = AuthoringNode {
        kind,
        surface: surface.to_string(),
        header_args: Vec::new(),
        definition_rows: Vec::new(),
        rows: Vec::new(),
        children: Vec::new(),
        content_rows: Vec::new(),
        source_line: line.clone(),
    };

    if !is_block_header_line(line) {
        return Err(DiagnosticReport::error_at_line(
            format!("{} must use block form", node.surface),
            line,
        ));
    }

    node.header_args = header[1..]
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    validate_header_args(&node)?;

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok((node, i + 1));
        }
        let tokens = split_authoring_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }
        if let Some(child_kind) = placed_authoring_kind(kind, tokens[0].as_str()) {
            let (child, next_i) = parse_authoring_node_with_kind(
                lines,
                i,
                child_kind,
                missing_close_message_for_kind(child_kind),
            )?;
            node.children.push(child);
            i = next_i;
            continue;
        }
        if let Some(row) = parse_authoring_row(kind, line)? {
            node.rows.push(row);
            i += 1;
            continue;
        }
        match authoring_kind_spec(kind).body {
            AuthoringBody::None => {
                if let Some((definition, next_i)) =
                    parse_authoring_definition_block(kind, lines, i)?
                {
                    node.definition_rows.push(definition);
                    i = next_i;
                    continue;
                }
                return Err(DiagnosticReport::error_at_line(
                    format!("unknown {} directive {}", node.surface, tokens[0]),
                    line,
                ));
            }
            AuthoringBody::Content(content) => {
                if let Some(content_row) = parse_authoring_content_row(content, line)? {
                    node.content_rows.push(content_row);
                    i += 1;
                    continue;
                }
                if let Some((definition, next_i)) =
                    parse_authoring_definition_block(kind, lines, i)?
                {
                    node.definition_rows.push(definition);
                    i = next_i;
                    continue;
                }
                return Err(DiagnosticReport::error_at_line(
                    format!("unknown {} directive {}", node.surface, tokens[0]),
                    line,
                ));
            }
        }
    }
    Err(DiagnosticReport::error_at_line(
        missing_close_message,
        &lines[start],
    ))
}

pub(crate) fn parse_authoring_node_source(
    source: &str,
    kind: AuthoringKind,
) -> Result<AuthoringNode, DiagnosticReport> {
    let lines = crate::source::logical_lines_with_locations(source)?
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(DiagnosticReport::error_at_line(
            "authoring node source is empty",
            source,
        ));
    }
    let (node, next_i) =
        parse_authoring_node_with_kind(&lines, 0, kind, missing_close_message_for_kind(kind))?;
    if next_i == lines.len() {
        Ok(node)
    } else {
        Err(DiagnosticReport::error_at_line(
            "authoring node source must contain one node",
            &lines[next_i],
        ))
    }
}

pub(crate) fn parse_authoring_row(
    kind: AuthoringKind,
    line: &str,
) -> Result<Option<AuthoringRow>, DiagnosticReport> {
    let tokens = split_authoring_tokens(line);
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    let candidates = authoring_row_specs(kind)
        .iter()
        .filter(|spec| row_spec_first_keyword(spec) == Some(first.as_str()))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(None);
    }
    for spec in &candidates {
        if let Some(captures) = parse_row_spec(spec, &tokens) {
            return Ok(Some(AuthoringRow {
                kind: spec.kind,
                captures,
                source_line: line.to_string(),
            }));
        }
    }
    let usage = candidates
        .iter()
        .map(|spec| spec.usage)
        .collect::<Vec<_>>()
        .join(" or ");
    Err(DiagnosticReport::error_at_line(
        format!("{first} row must be: {usage}"),
        line,
    ))
}

pub(crate) fn parse_authoring_content_row(
    content: AuthoringContentKind,
    line: &str,
) -> Result<Option<AuthoringContentRow>, DiagnosticReport> {
    let tokens = split_authoring_tokens(line);
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    let Some(rows) = authoring_content_row_specs(content) else {
        return Ok(None);
    };
    let candidates = rows
        .iter()
        .filter(|spec| match row_parts_first_keyword(spec.parts) {
            Some(keyword) => keyword == first.as_str(),
            None => true,
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(None);
    }
    for spec in &candidates {
        if let Some(captures) = parse_row_parts_captures(spec.parts, &tokens) {
            return Ok(Some(AuthoringContentRow {
                kind: spec.kind,
                captures,
                source_line: line.to_string(),
            }));
        }
    }
    let usage = candidates
        .iter()
        .map(|spec| spec.usage)
        .collect::<Vec<_>>()
        .join(" or ");
    Err(DiagnosticReport::error_at_line(
        format!("{first} row must be: {usage}"),
        line,
    ))
}

fn row_spec_first_keyword(spec: &RowSpec) -> Option<&'static str> {
    row_parts_first_keyword(spec.parts)
}

fn row_parts_first_keyword(parts: &[RowPartSpec]) -> Option<&'static str> {
    match parts.first()? {
        RowPartSpec::Keyword { surface, .. } => Some(surface),
        _ => None,
    }
}

fn parse_row_spec(spec: &RowSpec, tokens: &[String]) -> Option<Vec<AuthoringRowCapture>> {
    parse_row_parts_captures(spec.parts, tokens)
}

fn parse_row_parts_captures(
    parts: &[RowPartSpec],
    tokens: &[String],
) -> Option<Vec<AuthoringRowCapture>> {
    let texts = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let mut captures = Vec::<AuthoringRowCapture>::new();
    for row_match in match_row_parts(parts, &texts)? {
        match row_match.part {
            RowPartSpec::Keyword { .. } => {}
            RowPartSpec::Slot { name, .. } => {
                let value = tokens.get(row_match.start)?;
                captures.push(AuthoringRowCapture {
                    name,
                    values: vec![value.clone()],
                });
            }
            RowPartSpec::Equals => {}
            RowPartSpec::Rest { name, .. } => {
                captures.push(AuthoringRowCapture {
                    name,
                    values: tokens[row_match.start..row_match.end].to_vec(),
                });
            }
        }
    }
    Some(captures)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowPartMatch {
    part: RowPartSpec,
    start: usize,
    end: usize,
}

fn match_row_parts(parts: &[RowPartSpec], texts: &[&str]) -> Option<Vec<RowPartMatch>> {
    let mut index = 0;
    let mut matches = Vec::<RowPartMatch>::new();
    for part in parts {
        match *part {
            RowPartSpec::Keyword { surface, .. } => {
                if texts.get(index).copied() != Some(surface) {
                    return None;
                }
                matches.push(RowPartMatch {
                    part: *part,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            RowPartSpec::Slot { .. } => {
                if texts.get(index).is_none_or(|text| *text == "=") {
                    return None;
                }
                matches.push(RowPartMatch {
                    part: *part,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            RowPartSpec::Equals => {
                if texts.get(index).copied() != Some("=") {
                    return None;
                }
                index += 1;
            }
            RowPartSpec::Rest { .. } => {
                if index >= texts.len() {
                    return None;
                }
                matches.push(RowPartMatch {
                    part: *part,
                    start: index,
                    end: texts.len(),
                });
                index = texts.len();
            }
        }
    }
    (index == texts.len()).then_some(matches)
}

pub(crate) fn parse_authoring_definition_body(
    kind: AuthoringKind,
    lines: &[String],
) -> Result<Vec<AuthoringDefinitionRow>, DiagnosticReport> {
    let mut rows = Vec::<AuthoringDefinitionRow>::new();
    let mut i = 0;
    while i < lines.len() {
        if split_authoring_tokens(&lines[i]).is_empty() {
            i += 1;
            continue;
        }
        if let Some((row, next_i)) = parse_authoring_definition_block(kind, lines, i)? {
            rows.push(row);
            i = next_i;
            continue;
        }
        return Err(DiagnosticReport::error_at_line(
            format!(
                "unknown {} property",
                authoring_kind_spec(kind).header.usage
            ),
            &lines[i],
        ));
    }
    Ok(rows)
}

pub(crate) fn parse_authoring_definition_block(
    kind: AuthoringKind,
    lines: &[String],
    start: usize,
) -> Result<Option<(AuthoringDefinitionRow, usize)>, DiagnosticReport> {
    let line = &lines[start];
    let tokens = split_authoring_tokens(line);
    let Some(key) = tokens.first() else {
        return Ok(None);
    };
    let has_equals = tokens.get(1).is_some_and(|token| token == "=");
    let Some(spec) = authoring_definition_spec(kind, key) else {
        if has_equals {
            return Ok(Some((
                authoring_definition_row(
                    key,
                    Some(AuthoringDefinitionOp::Equals),
                    tokens[2..].to_vec(),
                    AuthoringDefinitionValueKind::SingleLine,
                    line,
                ),
                start + 1,
            )));
        }
        return Ok(None);
    };
    let value_start = if has_equals { 2 } else { 1 };
    let values = tokens[value_start..].to_vec();
    if values.is_empty() && definition_multiline_syntax(spec).is_some() {
        let mut next_i = start + 1;
        let mut multiline_values = Vec::<String>::new();
        while next_i < lines.len() && !starts_authoring_definition_block(kind, &lines[next_i]) {
            if !split_authoring_tokens(&lines[next_i]).is_empty() {
                multiline_values.push(lines[next_i].clone());
            }
            next_i += 1;
        }
        validate_multiline_definition_values(spec, &multiline_values, line)?;
        return Ok(Some((
            authoring_definition_row(
                key,
                has_equals.then_some(AuthoringDefinitionOp::Equals),
                multiline_values,
                AuthoringDefinitionValueKind::Multiline,
                line,
            ),
            next_i,
        )));
    }
    validate_definition_values(spec, &values, line)?;
    Ok(Some((
        authoring_definition_row(
            key,
            has_equals.then_some(AuthoringDefinitionOp::Equals),
            values,
            AuthoringDefinitionValueKind::SingleLine,
            line,
        ),
        start + 1,
    )))
}

fn authoring_definition_row(
    key: &str,
    op: Option<AuthoringDefinitionOp>,
    values: Vec<String>,
    value_kind: AuthoringDefinitionValueKind,
    source_line: &str,
) -> AuthoringDefinitionRow {
    AuthoringDefinitionRow {
        key: key.to_string(),
        op,
        values,
        value_kind,
        source_line: source_line.to_string(),
    }
}

fn starts_authoring_definition_block(kind: AuthoringKind, line: &str) -> bool {
    let tokens = split_authoring_tokens(line);
    let Some(key) = tokens.first() else {
        return false;
    };
    authoring_definition_spec(kind, key).is_some()
}

pub(crate) fn parse_authoring_definition_row(
    kind: AuthoringKind,
    line: &str,
) -> Result<Option<AuthoringDefinitionRow>, DiagnosticReport> {
    let lines = [line.to_string()];
    parse_authoring_definition_block(kind, &lines, 0).map(|parsed| parsed.map(|(row, _)| row))
}

fn validate_definition_values(
    spec: &DefinitionSpec,
    values: &[String],
    line: &str,
) -> Result<(), DiagnosticReport> {
    match definition_values(spec) {
        DefinitionValueSpec::None if !values.is_empty() => Err(DiagnosticReport::error_at_line(
            format!("{} must not have a value", spec.surface),
            line,
        )),
        DefinitionValueSpec::One if values.len() != 1 => Err(DiagnosticReport::error_at_line(
            format!("{} must have one value", spec.surface),
            line,
        )),
        DefinitionValueSpec::Many if values.is_empty() => Err(DiagnosticReport::error_at_line(
            format!("{} must have at least one value", spec.surface),
            line,
        )),
        _ => {
            for value in values {
                definition_value_literal(spec, value, line)?;
                validate_definition_value_domain(spec, value, line)?;
            }
            Ok(())
        }
    }
}

fn validate_multiline_definition_values(
    spec: &DefinitionSpec,
    values: &[String],
    line: &str,
) -> Result<(), DiagnosticReport> {
    match definition_multiline_syntax(spec) {
        Some(DefinitionMultilineSyntax::Lines) => {
            if values.is_empty() {
                return Err(DiagnosticReport::error_at_line(
                    format!("{} requires at least one row", spec.surface),
                    line,
                ));
            }
            Ok(())
        }
        None => validate_definition_values(spec, values, line),
    }
}

pub(crate) fn split_authoring_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::<String>::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if in_quote && ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            current.push(ch);
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == '=' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push("=".to_string());
            continue;
        }
        if !in_quote && ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.len() > 1 && tokens.last().map(String::as_str) == Some("{") {
        tokens.pop();
    }
    tokens
}

impl AuthoringDefinitionRow {
    pub(crate) fn single_value(&self) -> Option<&str> {
        let [value] = self.values.as_slice() else {
            return None;
        };
        Some(value)
    }
}

impl AuthoringRow {
    pub(crate) fn single_capture(&self, name: &str) -> Option<&str> {
        let capture = self.captures.iter().find(|capture| capture.name == name)?;
        let [value] = capture.values.as_slice() else {
            return None;
        };
        Some(value)
    }

    pub(crate) fn joined_capture(&self, name: &str) -> Option<String> {
        let capture = self.captures.iter().find(|capture| capture.name == name)?;
        (!capture.values.is_empty()).then(|| capture.values.join(" "))
    }
}

fn validate_header_args(node: &AuthoringNode) -> Result<(), DiagnosticReport> {
    let header = authoring_kind_spec(node.kind).header;
    if (header.min_args..=header.max_args).contains(&node.header_args.len()) {
        Ok(())
    } else {
        Err(DiagnosticReport::error_at_line(
            format!("{} header must be: {}", node.surface, header.usage),
            &node.source_line,
        ))
    }
}

fn missing_close_message_for_kind(kind: AuthoringKind) -> &'static str {
    authoring_kind_spec(kind).missing_close_message
}

#[cfg(test)]
mod tests {
    use super::{
        AuthoringContentKind, AuthoringContentRowKind, AuthoringDefinitionValueKind, AuthoringKind,
        ContentSyntax, authoring_capture_first, authoring_child_surfaces, authoring_content_syntax,
        authoring_source_block, authoring_symbol_exports, parse_authoring_content_row,
        parse_authoring_definition_body, parse_authoring_node_with_kind,
        parse_placed_authoring_node, placed_authoring_kind,
    };

    #[test]
    fn render_tween_placement_is_data_driven() {
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "render"),
            Some(AuthoringKind::PuzzleRenderConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::PuzzleRenderConfig, "tween"),
            Some(AuthoringKind::TweenConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::PuzzleRenderConfig, "grid"),
            Some(AuthoringKind::PuzzleRenderGridConfig)
        );
        assert_eq!(
            authoring_child_surfaces(AuthoringKind::PuzzleRenderConfig),
            vec!["tween", "grid"]
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Puzzle3Root, "render"),
            Some(AuthoringKind::Puzzle3RenderConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Puzzle3RenderConfig, "camera"),
            Some(AuthoringKind::Puzzle3CameraConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Puzzle3RenderConfig, "grid"),
            Some(AuthoringKind::Puzzle3GridConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Puzzle3RenderConfig, "pixelate"),
            Some(AuthoringKind::Puzzle3PixelateConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Puzzle3RenderConfig, "viewport"),
            Some(AuthoringKind::Puzzle3ViewportConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "sounds"),
            Some(AuthoringKind::SoundsConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::SoundsConfig, "sfx"),
            Some(AuthoringKind::SfxSoundConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::SoundsConfig, "music"),
            Some(AuthoringKind::MusicSoundConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "input_buffer"),
            Some(AuthoringKind::InputBufferConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "theme"),
            Some(AuthoringKind::ThemeConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "sprites"),
            Some(AuthoringKind::SpritesConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::SpritesConfig, "sprite"),
            Some(AuthoringKind::SpriteConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::Root, "levels"),
            Some(AuthoringKind::LevelsConfig)
        );
        assert_eq!(
            placed_authoring_kind(AuthoringKind::LevelsConfig, "level"),
            Some(AuthoringKind::LevelConfig)
        );
    }

    #[test]
    fn parses_parent_assignment_definition() {
        let lines = vec![
            "render {".to_string(),
            "tween_duration = 90ms".to_string(),
            "}".to_string(),
        ];
        let (node, next_i) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "render block missing closing brace",
        )
        .unwrap();
        assert_eq!(next_i, 3);
        assert_eq!(node.kind, AuthoringKind::PuzzleRenderConfig);
        assert_eq!(node.definition_rows[0].key, "tween_duration");
        assert_eq!(
            node.definition_rows[0].op,
            Some(super::AuthoringDefinitionOp::Equals)
        );
        assert_eq!(node.definition_rows[0].values, vec!["90ms"]);
    }

    #[test]
    fn parses_bare_definition_rows_for_fixed_items() {
        let lines = vec![
            "render {".to_string(),
            "tween {".to_string(),
            "duration 90ms".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let (node, next_i) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "render block missing closing brace",
        )
        .unwrap();
        assert_eq!(next_i, 5);
        assert_eq!(node.children[0].definition_rows[0].key, "duration");
        assert_eq!(node.children[0].definition_rows[0].op, None);
        assert_eq!(node.children[0].definition_rows[0].values, vec!["90ms"]);
    }

    #[test]
    fn parses_grid_type_definition_as_enum_value() {
        let lines = vec![
            "render {".to_string(),
            "grid {".to_string(),
            "type = \"occupied_cells\"".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let (node, _) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "render block missing closing brace",
        )
        .unwrap();
        assert_eq!(node.children[0].definition_rows[0].key, "type");
        assert_eq!(
            node.children[0].definition_rows[0].op,
            Some(super::AuthoringDefinitionOp::Equals)
        );
        assert_eq!(
            node.children[0].definition_rows[0].values,
            vec!["\"occupied_cells\""]
        );
    }

    #[test]
    fn parses_equals_as_delimiter_outside_quotes() {
        let lines = vec![
            "render {".to_string(),
            "cell_size= 64".to_string(),
            "grid {".to_string(),
            "type = \"all_cells\"".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let (node, _) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "render block missing closing brace",
        )
        .unwrap();
        assert_eq!(node.kind, AuthoringKind::PuzzleRenderConfig);
        assert_eq!(node.definition_rows[0].key, "cell_size");
        assert_eq!(
            node.definition_rows[0].op,
            Some(super::AuthoringDefinitionOp::Equals)
        );
        assert_eq!(node.definition_rows[0].values, vec!["64"]);
        assert_eq!(node.children[0].kind, AuthoringKind::PuzzleRenderGridConfig);
        assert_eq!(node.children[0].definition_rows[0].key, "type");
        assert_eq!(
            node.children[0].definition_rows[0].values,
            vec!["\"all_cells\""]
        );
    }

    #[test]
    fn parses_named_sound_children() {
        let lines = vec![
            "sounds {".to_string(),
            "sfx effect {".to_string(),
            "seed = 746670".to_string(),
            "type = jump".to_string(),
            "}".to_string(),
            "music loop {".to_string(),
            "seed = 123456".to_string(),
            "bpm = 104".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        let (node, _) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "sounds missing closing brace",
        )
        .unwrap();
        assert_eq!(node.kind, AuthoringKind::SoundsConfig);
        assert_eq!(node.children[0].kind, AuthoringKind::SfxSoundConfig);
        assert_eq!(node.children[0].header_args, vec!["effect"]);
        assert!(node.children[0].content_rows.is_empty());
        assert_eq!(node.children[0].definition_rows[0].key, "seed");
        assert_eq!(
            node.children[0].definition_rows[0].op,
            Some(super::AuthoringDefinitionOp::Equals)
        );
        assert_eq!(node.children[0].definition_rows[0].values, vec!["746670"]);
        assert_eq!(node.children[1].kind, AuthoringKind::MusicSoundConfig);
        assert_eq!(node.children[1].header_args, vec!["loop"]);
        assert!(node.children[1].content_rows.is_empty());
        assert_eq!(node.children[1].definition_rows[1].key, "bpm");
    }

    #[test]
    fn sound_child_symbol_exports_are_schema_metadata() {
        assert_eq!(
            authoring_symbol_exports(AuthoringKind::SfxSoundConfig),
            &[super::AuthoringSymbolExportSpec {
                source: super::AuthoringSymbolExportSource::HeaderArg(0),
                target: super::AuthoringSymbolExportTarget::Sfx,
            }]
        );
        assert_eq!(
            authoring_symbol_exports(AuthoringKind::MusicSoundConfig),
            &[super::AuthoringSymbolExportSpec {
                source: super::AuthoringSymbolExportSource::HeaderArg(0),
                target: super::AuthoringSymbolExportTarget::Music,
            }]
        );
        assert!(authoring_symbol_exports(AuthoringKind::SoundsConfig).is_empty());
    }

    #[test]
    fn parses_assets_as_content_rows() {
        let lines = vec![
            "assets {".to_string(),
            "\"game.css\"".to_string(),
            "\"visuals.js\"".to_string(),
            "\"sprites/player.png\"".to_string(),
            "}".to_string(),
        ];
        let (node, next_i) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "assets missing closing brace",
        )
        .unwrap();
        assert_eq!(next_i, 5);
        assert_eq!(node.kind, AuthoringKind::AssetsConfig);
        assert_eq!(
            super::authoring_kind_spec(node.kind).body,
            super::AuthoringBody::Content(super::AuthoringContentKind::AssetsEntries)
        );
        assert!(node.definition_rows.is_empty());
        assert_eq!(node.content_rows.len(), 3);
        assert_eq!(
            authoring_capture_first(&node.content_rows[0].captures, "path"),
            Some("\"game.css\"")
        );
    }

    #[test]
    fn assets_content_syntax_is_string_row_schema() {
        let rows = match authoring_content_syntax(AuthoringContentKind::AssetsEntries) {
            ContentSyntax::Rows(rows) => rows,
            ContentSyntax::Attachment(_) => panic!("assets content must use row syntax"),
        };
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, AuthoringContentRowKind::AssetPath);
        assert_eq!(rows[0].usage, "<string>");

        let row = parse_authoring_content_row(AuthoringContentKind::AssetsEntries, "\"game.css\"")
            .unwrap()
            .unwrap();
        assert_eq!(
            authoring_capture_first(&row.captures, "path"),
            Some("\"game.css\"")
        );
    }

    #[test]
    fn source_blocks_distinguish_containers_from_content() {
        assert_eq!(authoring_source_block("levels").unwrap().content, None);
        assert_eq!(
            authoring_source_block("level").unwrap().content,
            Some(AuthoringContentKind::LevelEntries)
        );
        assert_eq!(
            authoring_source_block("sprites").unwrap().content,
            Some(AuthoringContentKind::SpriteEntries)
        );
        assert_eq!(authoring_source_block("sprite").unwrap().content, None);
        assert_eq!(
            authoring_source_block("rules").unwrap().content,
            Some(AuthoringContentKind::RuleStatements)
        );
        assert_eq!(
            authoring_content_syntax(AuthoringContentKind::SpriteEntries),
            ContentSyntax::Attachment(super::ContentAttachment::SpriteEntries)
        );
        assert_eq!(
            super::authoring_kind_content_attachment(AuthoringKind::SpritesConfig),
            Some(super::ContentAttachment::SpriteEntries)
        );
        assert_eq!(
            authoring_content_syntax(AuthoringContentKind::RuleStatements),
            ContentSyntax::Attachment(super::ContentAttachment::RuleStatements)
        );
    }

    #[test]
    fn sprite_properties_support_explicit_shape_ref_and_multiline_shape() {
        let rows = parse_authoring_definition_body(
            AuthoringKind::SpriteConfig,
            &[
                "selector = Player".to_string(),
                "colors = #fff #000".to_string(),
                "shape = BoxShape".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(rows[0].key, "selector");
        assert_eq!(rows[1].key, "colors");
        assert_eq!(rows[1].values, vec!["#fff", "#000"]);
        assert_eq!(rows[2].key, "shape");
        assert_eq!(rows[2].values, vec!["BoxShape"]);
        assert_eq!(rows[2].value_kind, AuthoringDefinitionValueKind::SingleLine);

        let rows = parse_authoring_definition_body(
            AuthoringKind::SpriteConfig,
            &[
                "selector = Player".to_string(),
                "colors = #fff #000".to_string(),
                "shape =".to_string(),
                "000".to_string(),
                "101".to_string(),
                "000".to_string(),
            ],
        )
        .unwrap();
        let shape = rows.iter().find(|row| row.key == "shape").unwrap();
        assert_eq!(shape.value_kind, AuthoringDefinitionValueKind::Multiline);
        assert_eq!(shape.values, vec!["000", "101", "000"]);
    }

    #[test]
    fn sprite_body_rejects_implicit_property_slots() {
        let error = parse_authoring_definition_body(
            AuthoringKind::SpriteConfig,
            &[
                "Player".to_string(),
                "#fff #000".to_string(),
                "000".to_string(),
                "101".to_string(),
                "000".to_string(),
            ],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unknown sprite property"));

        let rows = parse_authoring_definition_body(
            AuthoringKind::SpriteConfig,
            &[
                "selector = Player".to_string(),
                "shape = BoxShape".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(rows[1].value_kind, AuthoringDefinitionValueKind::SingleLine);
    }

    #[test]
    fn parses_theme_preset_domain_definition() {
        let lines = vec![
            "theme {".to_string(),
            "preset = \"clean\"".to_string(),
            "background_color = #123456".to_string(),
            "}".to_string(),
        ];
        let (node, next_i) = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "theme missing closing brace",
        )
        .unwrap();
        assert_eq!(next_i, 4);
        assert_eq!(node.kind, AuthoringKind::ThemeConfig);
        assert_eq!(node.definition_rows[0].key, "preset");
        let spec = super::authoring_definition_spec(AuthoringKind::ThemeConfig, "preset").unwrap();
        assert_eq!(
            super::definition_value_syntax(spec),
            super::DefinitionValueSyntax::QuotedString
        );
        assert_eq!(
            super::definition_value_domain(spec),
            super::DefinitionValueDomain::Builtin(super::DefinitionBuiltinDomain::ThemePreset)
        );
    }

    #[test]
    fn root_theme_assignment_mirrors_theme_preset_definition() {
        let row = super::parse_authoring_definition_row(AuthoringKind::Root, "theme = \"clean\"")
            .unwrap()
            .unwrap();
        assert_eq!(row.key, "theme");
        assert_eq!(row.op, Some(super::AuthoringDefinitionOp::Equals));
        assert_eq!(row.values, vec!["\"clean\""]);

        let root = super::authoring_definition_spec(AuthoringKind::Root, "theme").unwrap();
        let preset =
            super::authoring_definition_spec(AuthoringKind::ThemeConfig, "preset").unwrap();
        assert_eq!(
            super::definition_value_syntax(root),
            super::definition_value_syntax(preset)
        );
        assert_eq!(
            super::definition_value_domain(root),
            super::definition_value_domain(preset)
        );
        assert_eq!(
            super::definition_value_role(root),
            super::definition_value_role(preset)
        );
    }

    #[test]
    fn root_scalar_prelude_entries_are_schema_definitions() {
        let title = super::parse_authoring_definition_row(
            AuthoringKind::Root,
            "title = Tiny Metadata Game",
        )
        .unwrap()
        .unwrap();
        assert_eq!(title.key, "title");
        assert_eq!(title.op, Some(super::AuthoringDefinitionOp::Equals));
        assert_eq!(title.values, vec!["Tiny", "Metadata", "Game"]);

        let title_spec = super::authoring_definition_spec(AuthoringKind::Root, "title").unwrap();
        assert_eq!(
            super::definition_values(title_spec),
            super::DefinitionValueSpec::Many
        );
        assert_eq!(
            super::definition_value_role(title_spec),
            Some(super::AuthoringSurfaceRole::String)
        );

        let wait =
            super::authoring_definition_spec(AuthoringKind::Root, "default_wait_time").unwrap();
        assert_eq!(
            super::definition_values(wait),
            super::DefinitionValueSpec::One
        );
        assert_eq!(
            super::definition_value_syntax(wait),
            super::DefinitionValueSyntax::Duration
        );
    }

    #[test]
    fn root_variable_rows_are_schema_forms() {
        let row = super::parse_authoring_row(AuthoringKind::Root, "persistent const score=10")
            .unwrap()
            .unwrap();
        assert_eq!(
            row.kind,
            super::AuthoringRowKind::PersistentConstDeclaration
        );
        assert_eq!(row.single_capture("name"), Some("score"));
        assert_eq!(row.joined_capture("value").as_deref(), Some("10"));
    }

    #[test]
    fn rejects_unquoted_theme_preset_domain_value() {
        let lines = vec![
            "theme {".to_string(),
            "preset = clean".to_string(),
            "}".to_string(),
        ];
        let error = parse_placed_authoring_node(
            &lines,
            0,
            AuthoringKind::Root,
            "theme missing closing brace",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("preset must be a quoted string"));
    }

    #[test]
    fn rejects_whitespace_only_inline_node() {
        let lines = vec!["sfx effect seed=1".to_string()];
        let error =
            parse_authoring_node_with_kind(&lines, 0, AuthoringKind::SfxSoundConfig, "missing")
                .unwrap_err()
                .to_string();
        assert!(error.contains("sfx must use block form"));
    }
}
