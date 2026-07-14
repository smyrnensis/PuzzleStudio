use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(feature = "embedded-assets")]
use std::env;
use std::fs;
use std::io;
#[cfg(feature = "embedded-assets")]
use std::io::{Read, Write};
#[cfg(feature = "embedded-assets")]
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
#[cfg(feature = "embedded-assets")]
use std::sync::Arc;

#[cfg(feature = "embedded-assets")]
use puzzle_lang::Diagnostic;
use puzzle_lang::DiagnosticReport;
#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
use puzzle_lang::{AssetKind, AssetsDef};

#[cfg(feature = "embedded-assets")]
const EDITOR_HTML: &str = include_str!("../static/editor.html");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MARKDOWN: &str = include_str!("../docs/editor.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_METADATA_MARKDOWN: &str = include_str!("../docs/metadata.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN: &str = include_str!("../docs/puzzle-block.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SLOTS_MARKDOWN: &str = include_str!("../docs/slots.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_GROUPS_MARKDOWN: &str = include_str!("../docs/groups.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_TAGS_MARKDOWN: &str = include_str!("../docs/tags.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_LEGEND_MARKDOWN: &str = include_str!("../docs/legend.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_LEVELS_MARKDOWN: &str = include_str!("../docs/levels.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_LEVEL_LOCAL_LEGEND_MARKDOWN: &str = include_str!("../docs/level-local-legend.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MESSAGES_MARKDOWN: &str = include_str!("../docs/messages.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_REWRITE_RULES_MARKDOWN: &str = include_str!("../docs/rewrite-rules.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_INPUT_RULES_MARKDOWN: &str = include_str!("../docs/input-rules.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MOVEMENT_MARKDOWN: &str = include_str!("../docs/movement.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_GUARDS_MARKDOWN: &str = include_str!("../docs/guards.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_FIX_MARKDOWN: &str = include_str!("../docs/fix.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_VARIABLES_MARKDOWN: &str = include_str!("../docs/variables.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MARK_MARKDOWN: &str = include_str!("../docs/mark.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_CONDITIONS_MARKDOWN: &str = include_str!("../docs/conditions.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_WIN_CONDITIONS_MARKDOWN: &str = include_str!("../docs/win-conditions.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SCENES_MARKDOWN: &str = include_str!("../docs/scenes.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SCENE_LAYOUT_MARKDOWN: &str = include_str!("../docs/scene-layout.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SEMANTIC_INPUTS_MARKDOWN: &str = include_str!("../docs/semantic-inputs.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MENUS_MARKDOWN: &str = include_str!("../docs/menus.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_LIFECYCLE_MARKDOWN: &str = include_str!("../docs/lifecycle.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SPRITES_MARKDOWN: &str = include_str!("../docs/sprites.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_DISPLAY_MARKDOWN: &str = include_str!("../docs/display.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_THEME_MARKDOWN: &str = include_str!("../docs/theme.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SOUNDS_MARKDOWN: &str = include_str!("../docs/sounds.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_ROUTINES_MARKDOWN: &str = include_str!("../docs/routines.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_RULE_APPLICATION_MARKDOWN: &str = include_str!("../docs/rule-application.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_PATTERNS_MARKDOWN: &str = include_str!("../docs/patterns.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_IMPORTS_MARKDOWN: &str = include_str!("../docs/imports.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_RENDERING_MARKDOWN: &str = include_str!("../docs/rendering.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_3D_MARKDOWN: &str = include_str!("../docs/3d.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_ASSETS_MARKDOWN: &str = include_str!("../docs/assets.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_RULE_EFFECTS_MARKDOWN: &str = include_str!("../docs/rule-effects.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SPRITE_SHAPES_MARKDOWN: &str = include_str!("../docs/sprite-shapes.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_SCENE_STATE_EFFECTS_MARKDOWN: &str =
    include_str!("../docs/scene-state-effects.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_MAPS_EXPANSION_MARKDOWN: &str = include_str!("../docs/maps-expansion.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_CSS: &str = include_str!("../static/editor.css");
#[cfg(feature = "embedded-assets")]
const EDITOR_RUNTIME_JS: &str = include_str!("../static/editor_runtime.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_ANALYSIS_WORKER_JS: &str = include_str!("../static/editor_analysis_worker.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_BOOT_JS: &str = include_str!("../static/editor_boot.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_CODEMIRROR_JS: &str = include_str!("../static/editor_codemirror.js");
#[cfg(test)]
const EDITOR_CODEMIRROR_SOURCE_JS: &str = include_str!("../web/src/editor_codemirror.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOM_JS: &str = include_str!("../static/editor_dom.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_WORKSPACE_JS: &str = include_str!("../static/editor_workspace.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_COLOR_JS: &str = include_str!("../static/editor_color.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_SOURCE_JS: &str = include_str!("../static/editor_source.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_LEVEL3D_JS: &str = include_str!("../static/editor_level3d.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_WORKBENCH_JS: &str = include_str!("../static/editor_workbench.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_IMPORT_EXPORT_JS: &str = include_str!("../static/editor_import_export.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_JS: &str = include_str!("../static/editor.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_SPRITE_DOCUMENT_JS: &str = include_str!("../static/editor_sprite_document.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_SPRITE_JS: &str = include_str!("../static/editor_sprite.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../../html_play/static/puzzle3_visual_core.js");
#[cfg(all(test, feature = "embedded-assets"))]
const EDITOR_STATIC_PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_SPRITE3D_JS: &str = include_str!("../static/editor_sprite3d.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_SOUNDS_JS: &str = include_str!("../static/editor_sounds.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_COMMANDS_JS: &str = include_str!("../static/editor_commands.js");
#[cfg(feature = "embedded-assets")]
const FAVICON_SVG: &str = include_str!("../static/favicon.svg");
#[cfg(feature = "embedded-assets")]
const PUZZLE_WASM_JS: &str = include_str!("../static/wasm/puzzle_wasm.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE_WASM_BG: &[u8] = include_bytes!("../static/wasm/puzzle_wasm_bg.wasm");
#[cfg(feature = "embedded-assets")]
const PUZZLE_GAME_WASM_JS: &str =
    include_str!("../../html_play/static/wasm_game/puzzle_wasm_game.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE_GAME_WASM_BG: &[u8] =
    include_bytes!("../../html_play/static/wasm_game/puzzle_wasm_game_bg.wasm");
#[cfg(feature = "embedded-assets")]
const PUZZLE_CORE_WASM_JS: &str = include_str!("../../wasm_core/static/puzzle_core_wasm.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE_CORE_WASM_BG: &[u8] =
    include_bytes!("../../wasm_core/static/puzzle_core_wasm_bg.wasm");
#[cfg(feature = "sound-tools")]
const SEEDED_SFX_JS: &str = include_str!("../../../tools/music_generator/seeded_sfx.mjs");
#[cfg(feature = "sound-tools")]
const SEEDED_MUSIC_JS: &str = include_str!("../../../tools/music_generator/seeded_music.mjs");
#[cfg(feature = "sound-tools")]
const SEEDED_MUSIC_PLAYER_JS: &str =
    include_str!("../../../tools/music_generator/seeded_music_player.mjs");
#[cfg(feature = "sound-tools")]
const SEEDED_TIMBRE_FIELDS_JS: &str =
    include_str!("../../../tools/music_generator/seeded_timbre_fields.mjs");
#[cfg(feature = "sound-tools")]
const SOUND_EXPORT_JS: &str = include_str!("../../../tools/music_generator/audio_export.mjs");
#[cfg(feature = "embedded-assets")]
const RENDERER_CSS: &str = include_str!("../../html_play/static/renderer.css");
#[cfg(all(test, feature = "embedded-assets"))]
const EDITOR_STATIC_RENDERER_CSS: &str = include_str!("../static/renderer.css");
#[cfg(feature = "embedded-assets")]
const VISUALS_JS: &str = include_str!("../../html_play/static/visuals.js");
#[cfg(feature = "embedded-assets")]
const RENDERER_JS: &str = include_str!("../../html_play/static/renderer.js");
#[cfg(all(test, feature = "embedded-assets"))]
const EDITOR_STATIC_RENDERER_JS: &str = include_str!("../static/renderer.js");
#[cfg(feature = "embedded-assets")]
const PAGES_EXAMPLE_PUZZLE_PATH: &str = "starter/01-basic.puzzle";
#[cfg(feature = "embedded-assets")]
const PAGES_EXAMPLE_PUZZLE_SOURCE: &str = include_str!("../starter/01-basic.puzzle");
#[cfg(feature = "embedded-assets")]
const PAGES_STARTER_DOCUMENTS: &[(&str, &str, bool)] = &[
    (
        PAGES_EXAMPLE_PUZZLE_PATH,
        include_str!("../starter/01-basic.puzzle"),
        true,
    ),
    (
        "starter/README.md",
        include_str!("../starter/README.md"),
        false,
    ),
    (
        "starter/02-scenes-and-theme.puzzle",
        include_str!("../starter/02-scenes-and-theme.puzzle"),
        true,
    ),
    (
        "starter/03-sound.puzzle",
        include_str!("../starter/03-sound.puzzle"),
        true,
    ),
    (
        "starter/04-animation.puzzle",
        include_str!("../starter/04-animation.puzzle"),
        true,
    ),
    (
        "starter/05-tags-marks-and-routines.puzzle",
        include_str!("../starter/05-tags-marks-and-routines.puzzle"),
        true,
    ),
    (
        "starter/06-3d.puzzle3",
        include_str!("../starter/06-3d.puzzle3"),
        true,
    ),
    (
        "starter/07-meta-level.puzzle",
        include_str!("../starter/07-meta-level.puzzle"),
        true,
    ),
];
const SKIPPED_WORKSPACE_DIRS: &[&str] = &[
    ".cache",
    ".git",
    ".next",
    ".turbo",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "target",
];

#[cfg(feature = "embedded-assets")]
pub fn run_cli() -> Result<(), AppError> {
    run_cli_with_args(env::args().skip(1))
}

#[cfg(feature = "embedded-assets")]
pub fn run_cli_with_args(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    run(args)
}

#[cfg(feature = "embedded-assets")]
fn run(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    let config = Config::from_args(args)?;
    let service = if let Some(puzzle_path) = &config.puzzle_path {
        EditorService::open_game_entry(puzzle_path)?
    } else if config.serve {
        let puzzle_path = puzzle_lang::resolve_game_entry(&PathBuf::from("games/spec_2d.puzzle"))
            .map_err(|error| AppError::Config(error.to_string()))?;
        EditorService::open(&puzzle_path)?
    } else {
        EditorService::open_pages_example()
    };

    if !config.serve {
        let output_path = config.output_path();
        write_pages_editor_site(&output_path, service.export_pages_editor_html()?)?;
        println!("exported {}", output_path.display());
        return Ok(());
    }

    let service = Arc::new(service);
    let (listener, port) = bind_listener(config.port)?;

    println!("html-editor serving http://127.0.0.1:{port}/editor");
    println!("puzzle: {}", service.puzzle_path());

    for stream in listener.incoming() {
        let stream = stream?;
        let service = Arc::clone(&service);
        if let Err(error) = handle_connection(stream, service) {
            eprintln!("request error: {error}");
        }
    }

    Ok(())
}

#[cfg(feature = "embedded-assets")]
#[derive(Clone, Debug)]
struct Config {
    puzzle_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    serve: bool,
    port: u16,
}

#[cfg(feature = "embedded-assets")]
impl Config {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, AppError> {
        let mut puzzle_path = None;
        let mut output_path = None;
        let mut serve = false;
        let mut port = 8787;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--serve" => {
                    serve = true;
                }
                "--output" | "-o" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config(format!("{arg} requires a value")));
                    };
                    output_path = Some(PathBuf::from(value));
                }
                "--port" => {
                    let Some(value) = args.next() else {
                        return Err(AppError::Config("--port requires a value".to_string()));
                    };
                    serve = true;
                    port = value
                        .parse()
                        .map_err(|_| AppError::Config("port must be a u16".to_string()))?;
                }
                "--help" | "-h" => {
                    return Err(AppError::Config(
                        "usage: html-editor [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [-o docs/index.html] [--serve] [--port 8787]"
                            .to_string(),
                    ));
                }
                value => puzzle_path = Some(PathBuf::from(value)),
            }
        }

        Ok(Self {
            puzzle_path,
            output_path,
            serve,
            port,
        })
    }

    fn output_path(&self) -> PathBuf {
        if let Some(output_path) = &self.output_path {
            return output_path.clone();
        }
        PathBuf::from("docs/index.html")
    }
}

pub struct EditorService {
    state: EditorState,
}

impl EditorService {
    #[cfg(feature = "embedded-assets")]
    fn open_pages_example() -> Self {
        let source = PAGES_STARTER_DOCUMENTS
            .iter()
            .find(|(path, _, _)| *path == PAGES_EXAMPLE_PUZZLE_PATH)
            .map(|(_, source, _)| (*source).to_string())
            .expect("Pages starter must contain its default puzzle");
        Self {
            state: EditorState {
                puzzle_path: PAGES_EXAMPLE_PUZZLE_PATH.to_string(),
                workspace_root: String::new(),
                source: source.clone(),
                game_css: String::new(),
                #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
                base_game_visuals_js: String::new(),
                folders: vec!["starter".to_string()],
                documents: PAGES_STARTER_DOCUMENTS
                    .iter()
                    .map(
                        |(path, document_source, declares_game_entry)| EditorDocument {
                            puzzle_path: (*path).to_string(),
                            encoding: "text".to_string(),
                            mime_type: mime_type(Path::new(path)).to_string(),
                            source: (*document_source).to_string(),
                            data_url: String::new(),
                            content_loaded: true,
                            declares_game_entry: *declares_game_entry,
                            preview_html: String::new(),
                            preview_error: String::new(),
                            game_css: String::new(),
                            imported_by: Vec::new(),
                            parent_game_path: if *declares_game_entry {
                                (*path).to_string()
                            } else {
                                String::new()
                            },
                        },
                    )
                    .collect(),
            },
        }
    }

    pub fn open_game_entry(path: &Path) -> Result<Self, AppError> {
        let puzzle_path = match puzzle_lang::resolve_game_entry(path) {
            Ok(puzzle_path) => puzzle_path,
            Err(error) if path.is_dir() => {
                let message = error.to_string();
                if message.contains("game folder must contain a .puzzle or .puzzle3 file") {
                    return Self::open_workspace_root(path);
                }
                return Err(AppError::Config(message));
            }
            Err(error) => return Err(AppError::Config(error.to_string())),
        };
        let workspace_root = if path.is_dir() {
            path.to_path_buf()
        } else {
            puzzle_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        };
        Self::open_with_workspace_root(&puzzle_path, &workspace_root)
    }

    pub fn open_workspace_root(workspace_root: &Path) -> Result<Self, AppError> {
        let workspace_root = workspace_root.canonicalize()?;
        if !workspace_root.is_dir() {
            return Err(AppError::Config(format!(
                "workspace folder not found: {}",
                workspace_root.display()
            )));
        }
        Ok(Self {
            state: EditorState {
                puzzle_path: String::new(),
                workspace_root: workspace_root.display().to_string(),
                source: String::new(),
                game_css: String::new(),
                #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
                base_game_visuals_js: String::new(),
                folders: load_workspace_folders(&workspace_root)?,
                documents: load_editor_documents(&workspace_root, &workspace_root)?,
            },
        })
    }

    pub fn open(puzzle_path: &Path) -> Result<Self, AppError> {
        let workspace_root = editor_document_root(puzzle_path);
        Self::open_with_workspace_root(puzzle_path, &workspace_root)
    }

    fn open_with_workspace_root(
        puzzle_path: &Path,
        workspace_root: &Path,
    ) -> Result<Self, AppError> {
        let puzzle_path = puzzle_path.canonicalize()?;
        let workspace_root = workspace_root.canonicalize()?;
        if !puzzle_path.starts_with(&workspace_root) {
            return Err(AppError::Config(format!(
                "game entry must be under opened project root {}",
                workspace_root.display()
            )));
        }
        let source = fs::read_to_string(&puzzle_path)?;
        #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
        let base_game_visuals_js =
            load_base_game_visuals_js(&puzzle_path, &workspace_root, &source)?;
        Ok(Self {
            state: EditorState {
                puzzle_path: puzzle_path.display().to_string(),
                workspace_root: workspace_root.display().to_string(),
                source,
                game_css: load_game_css(&puzzle_path, &workspace_root)?,
                #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
                base_game_visuals_js,
                folders: load_workspace_folders(&workspace_root)?,
                documents: load_editor_documents(&puzzle_path, &workspace_root)?,
            },
        })
    }

    pub fn state(&self) -> &EditorState {
        &self.state
    }

    pub fn workspace_root(&self) -> &str {
        &self.state.workspace_root
    }

    pub fn puzzle_path(&self) -> &str {
        &self.state.puzzle_path
    }

    pub fn source_json(&self) -> Result<String, AppError> {
        source_json(&self.state)
    }

    pub fn source_json_with_content(&self) -> Result<String, AppError> {
        source_json_with_content(&self.state)
    }

    pub fn load_workspace_document_json(
        &self,
        request: &LoadWorkspaceDocumentRequest,
    ) -> Result<String, AppError> {
        load_workspace_document_json(request, &self.state)
    }

    #[cfg(feature = "native-preview")]
    pub fn compile_preview(&self, request: &PreviewRequest) -> Result<String, AppError> {
        let workspace_root = PathBuf::from(&self.state.workspace_root);
        let preview_path = resolve_workspace_request_path(&request.puzzle_path, &workspace_root)?;
        let expanded_source =
            expand_preview_source_under_root(&request.source, &preview_path, &workspace_root)?;
        let game_visuals_js =
            load_base_game_visuals_js(&preview_path, &workspace_root, &expanded_source)?;
        html_play::export_editor_preview_html_from_source(
            &expanded_source,
            &preview_path.display().to_string(),
            &request.game_css,
            &game_visuals_js,
        )
        .map_err(AppError::Diagnostics)
    }

    pub fn highlight_json(&self, source: &str) -> Result<String, AppError> {
        let profile = puzzle_lang::puzzle_source_profile_for_path(&self.state.puzzle_path)
            .ok_or_else(|| {
                AppError::Config(
                    "source highlighting requires an active .puzzle or .puzzle3 document profile"
                        .to_string(),
                )
            })?;
        Ok(Self::highlight_source_json(source, profile))
    }

    pub fn highlight_source_json(
        source: &str,
        profile: puzzle_lang::PuzzleSourceProfile,
    ) -> String {
        puzzle_lang::analyze_source_for_profile(source, profile).highlight_json(false)
    }

    pub fn save_source_file(&self, request: &SaveRequest) -> Result<(), AppError> {
        save_source_file(request, &self.state)
    }

    pub fn create_source_file(
        &self,
        request: &CreateSourceFileRequest,
    ) -> Result<PathBuf, AppError> {
        create_source_file(request, &self.state)
    }

    pub fn create_source_folder(
        &self,
        request: &CreateSourceFolderRequest,
    ) -> Result<PathBuf, AppError> {
        create_source_folder(request, &self.state)
    }

    pub fn rename_workspace_entry(
        &self,
        request: &RenameWorkspaceEntryRequest,
    ) -> Result<PathBuf, AppError> {
        rename_workspace_entry(request, &self.state)
    }

    pub fn delete_workspace_entry(
        &self,
        request: &DeleteWorkspaceEntryRequest,
    ) -> Result<(), AppError> {
        delete_workspace_entry(request, &self.state)
    }

    #[cfg(feature = "embedded-assets")]
    pub fn export_pages_editor_html(&self) -> Result<String, AppError> {
        export_pages_editor_html(&self.state)
    }
}

#[cfg(feature = "sound-tools")]
pub fn sound_tools_script() -> String {
    sound_tools_js()
}

pub fn new_puzzle_source(title: &str) -> String {
    puzzle_authoring::new_puzzle_source(title)
}

#[derive(Debug)]
pub struct EditorState {
    puzzle_path: String,
    workspace_root: String,
    source: String,
    game_css: String,
    #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
    base_game_visuals_js: String,
    folders: Vec<String>,
    documents: Vec<EditorDocument>,
}

#[derive(Debug)]
pub struct EditorDocument {
    puzzle_path: String,
    encoding: String,
    mime_type: String,
    source: String,
    data_url: String,
    content_loaded: bool,
    declares_game_entry: bool,
    preview_html: String,
    preview_error: String,
    game_css: String,
    imported_by: Vec<String>,
    parent_game_path: String,
}

struct WorkspacePuzzleDocument {
    path: PathBuf,
    source: String,
    declares_game_entry: bool,
    imports: Vec<PathBuf>,
}

#[derive(Default)]
struct WorkspaceImportGraph {
    imported_by: HashMap<PathBuf, Vec<PathBuf>>,
    parent_game_by_path: HashMap<PathBuf, PathBuf>,
}

fn load_game_css(puzzle_path: &Path, workspace_root: &Path) -> Result<String, AppError> {
    let css_path = puzzle_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("game.css");
    if css_path.exists() {
        let css = read_workspace_text_file(&css_path, workspace_root)?;
        inline_css_urls(
            &css,
            css_path.parent().unwrap_or_else(|| Path::new(".")),
            workspace_root,
        )
    } else {
        Ok(String::new())
    }
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn load_base_game_visuals_js(
    puzzle_path: &Path,
    workspace_root: &Path,
    source: &str,
) -> Result<String, AppError> {
    let assets = puzzle_lang::parse_document_assets(source).map_err(AppError::Diagnostics)?;
    let image_paths = sprite_image_asset_paths(source);
    let mut scripts = vec![asset_resolver_js(
        puzzle_path,
        workspace_root,
        &assets,
        &image_paths,
    )?];
    #[cfg(feature = "embedded-assets")]
    scripts.push(VISUALS_JS.to_string());
    let visuals_path = puzzle_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("visuals.js");
    if visuals_path.exists() {
        scripts.push(read_workspace_text_file(&visuals_path, workspace_root)?);
    }
    Ok(scripts.join("\n"))
}

fn inline_css_urls(css: &str, base_dir: &Path, workspace_root: &Path) -> Result<String, AppError> {
    let mut out = String::new();
    let mut rest = css;
    while let Some(start) = rest.find("url(") {
        let (before, after_start) = rest.split_at(start);
        out.push_str(before);
        let Some(end) = after_start.find(')') else {
            out.push_str(after_start);
            return Ok(out);
        };
        let raw = after_start[4..end].trim().trim_matches(['"', '\'']);
        if raw.starts_with("data:")
            || raw.starts_with("http:")
            || raw.starts_with("https:")
            || raw.starts_with('#')
        {
            out.push_str(&after_start[..=end]);
        } else {
            let asset_path = base_dir.join(raw);
            if asset_path.exists() {
                let mime_type = mime_type(&asset_path);
                match read_workspace_bytes(&asset_path, workspace_root) {
                    Ok(bytes) => {
                        let encoded = base64_encode(&bytes);
                        out.push_str(&format!("url(\"data:{mime_type};base64,{encoded}\")"));
                    }
                    Err(_) => out.push_str(&after_start[..=end]),
                }
            } else {
                out.push_str(&after_start[..=end]);
            }
        }
        rest = &after_start[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn asset_resolver_js(
    puzzle_path: &Path,
    workspace_root: &Path,
    assets: &AssetsDef,
    image_paths: &[String],
) -> Result<String, AppError> {
    let parent = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = String::new();
    files.push('{');
    let mut first = true;
    let mut paths = assets
        .entries
        .iter()
        .filter(|asset| asset.kind == AssetKind::File)
        .map(|asset| asset.path.clone())
        .collect::<Vec<_>>();
    for image_path in image_paths {
        if !paths.iter().any(|path| path == image_path) {
            paths.push(image_path.clone());
        }
    }
    for asset_path in paths {
        let path = resolve_asset_path(parent, &asset_path)?;
        push_asset_resolver_entry(parent, &path, workspace_root, &mut files, &mut first)?;
    }
    files.push('}');
    Ok(format!(
        "window.PuzzleAssets = {{ files: {files}, url(path) {{ const key = String(path || '').replaceAll('\\\\\\\\', '/'); if (Object.prototype.hasOwnProperty.call(this.files, key)) return this.files[key]; if (/^(?:data:|https?:|#)/.test(key)) return key; throw new Error(`Puzzle asset is not embedded: ${{key}}. Declare it with file \\\"${{key}}\\\" in assets.`); }} }};"
    ))
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn sprite_image_asset_paths(source: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("image ") else {
            continue;
        };
        let rest = rest.trim();
        let Some(path) = rest
            .strip_prefix('"')
            .and_then(|value| value.split_once('"').map(|(path, _)| path))
        else {
            continue;
        };
        if !paths.iter().any(|existing| existing == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn resolve_asset_path(base_dir: &Path, asset_path: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(asset_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AppError::Config(format!(
            "asset path must be game-folder relative: {asset_path}"
        )));
    }
    let resolved = base_dir.join(path);
    if !resolved.exists() {
        return Err(AppError::Config(format!(
            "asset file not found: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn push_asset_resolver_entry(
    root: &Path,
    path: &Path,
    workspace_root: &Path,
    files: &mut String,
    first: &mut bool,
) -> Result<(), AppError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        AppError::Config(format!(
            "asset file is outside game folder: {}",
            path.display()
        ))
    })?;
    let name = relative.to_str().ok_or_else(|| {
        AppError::Config(format!(
            "asset file path is not valid UTF-8: {}",
            path.display()
        ))
    })?;
    if !*first {
        files.push(',');
    }
    *first = false;
    push_json_string(files, &name.replace('\\', "/"));
    files.push(':');
    let url = if is_text_file(path) {
        format!(
            "data:{};charset=utf-8,{}",
            mime_type(path),
            percent_encode(&read_workspace_text_file(path, workspace_root)?)
        )
    } else {
        format!(
            "data:{};base64,{}",
            mime_type(path),
            base64_encode(&read_workspace_bytes(path, workspace_root)?)
        )
    };
    push_json_string(files, &url);
    Ok(())
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(feature = "native-preview")]
fn expand_preview_source_under_root(
    source: &str,
    puzzle_path: &Path,
    workspace_root: &Path,
) -> Result<String, AppError> {
    puzzle_lang::validate_source_profile_for_path(source, puzzle_path)
        .map_err(|error| AppError::Config(error.to_string()))?;
    match puzzle_lang::puzzle_source_profile_for_path(puzzle_path) {
        Some(puzzle_lang::PuzzleSourceProfile::Puzzle3d) => {
            let workspace_root = workspace_root.canonicalize()?;
            let preview_path = puzzle_path.canonicalize()?;
            if !preview_path.starts_with(&workspace_root) {
                return Err(AppError::Config(format!(
                    "can only import puzzle files under {}",
                    workspace_root.display()
                )));
            }
            Ok(source.to_string())
        }
        Some(puzzle_lang::PuzzleSourceProfile::Puzzle2d) => {
            let expanded = puzzle_lang::expand_game_imports_for_file_under_root(
                source,
                puzzle_path,
                workspace_root,
            )
            .map_err(|error| AppError::Config(error.to_string()))?;
            puzzle_lang::validate_source_profile_for_path(&expanded, puzzle_path)
                .map_err(|error| AppError::Config(error.to_string()))?;
            Ok(expanded)
        }
        None => Err(AppError::Config(format!(
            "preview source must be .puzzle or .puzzle3: {}",
            puzzle_path.display()
        ))),
    }
}

fn load_editor_documents(
    active_path: &Path,
    workspace_root: &Path,
) -> Result<Vec<EditorDocument>, AppError> {
    let parent = workspace_root;
    let mut paths = Vec::new();
    collect_workspace_files(parent, &mut paths)?;
    paths.sort_by(|left, right| {
        let left_key = if left == active_path { "" } else { "z" };
        let right_key = if right == active_path { "" } else { "z" };
        (left_key, left.display().to_string()).cmp(&(right_key, right.display().to_string()))
    });

    let puzzle_documents = load_workspace_puzzle_documents(&paths, workspace_root)?;
    let import_graph = build_workspace_import_graph(&puzzle_documents);
    let puzzle_sources_by_path = puzzle_documents
        .into_iter()
        .map(|document| (document.path, document.source))
        .collect::<HashMap<_, _>>();

    let mut documents = Vec::new();
    for path in paths {
        if puzzle_lang::is_puzzle_source_path(&path) {
            let canonical_path = path.canonicalize()?;
            let source = puzzle_sources_by_path
                .get(&canonical_path)
                .cloned()
                .ok_or_else(|| {
                    AppError::Config(format!(
                        "workspace puzzle source was not indexed: {}",
                        path.display()
                    ))
                })?;
            let parent_game_path = import_graph.parent_game_by_path.get(&canonical_path);
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "text".to_string(),
                mime_type: mime_type(&path).to_string(),
                source: String::new(),
                data_url: String::new(),
                content_loaded: false,
                declares_game_entry: puzzle_lang::source_declares_game_entry(&source),
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: import_graph
                    .imported_by
                    .get(&canonical_path)
                    .map(|paths| display_paths(paths))
                    .unwrap_or_default(),
                parent_game_path: parent_game_path
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            });
        } else if is_text_file(&path) {
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "text".to_string(),
                mime_type: mime_type(&path).to_string(),
                source: String::new(),
                data_url: String::new(),
                content_loaded: false,
                declares_game_entry: false,
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: Vec::new(),
                parent_game_path: String::new(),
            });
        } else {
            let mime_type = mime_type(&path);
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "data_url".to_string(),
                mime_type: mime_type.to_string(),
                source: String::new(),
                data_url: String::new(),
                content_loaded: false,
                declares_game_entry: false,
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: Vec::new(),
                parent_game_path: String::new(),
            });
        }
    }
    Ok(documents)
}

fn load_workspace_puzzle_documents(
    paths: &[PathBuf],
    workspace_root: &Path,
) -> Result<Vec<WorkspacePuzzleDocument>, AppError> {
    let workspace_root = workspace_root.canonicalize()?;
    let mut documents = Vec::new();
    for path in paths {
        if !puzzle_lang::is_puzzle_source_path(path) {
            continue;
        }
        let canonical_path = path.canonicalize()?;
        if !canonical_path.starts_with(&workspace_root) {
            continue;
        }
        let source = read_workspace_text_file(&canonical_path, &workspace_root)?;
        let imports = workspace_import_paths(&source, &canonical_path, &workspace_root);
        documents.push(WorkspacePuzzleDocument {
            path: canonical_path,
            declares_game_entry: puzzle_lang::source_declares_game_entry(&source),
            source,
            imports,
        });
    }
    documents.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(documents)
}

fn workspace_import_paths(source: &str, path: &Path, workspace_root: &Path) -> Vec<PathBuf> {
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(imports) = puzzle_lang::game_import_paths(source) else {
        return Vec::new();
    };
    imports
        .into_iter()
        .filter_map(|import| canonical_workspace_import_path(base_dir, &import, workspace_root))
        .collect()
}

fn canonical_workspace_import_path(
    base_dir: &Path,
    import: &Path,
    workspace_root: &Path,
) -> Option<PathBuf> {
    let resolved = if import.is_absolute() {
        import.to_path_buf()
    } else {
        base_dir.join(import)
    };
    let canonical = resolved.canonicalize().ok()?;
    if !canonical.starts_with(workspace_root) || !puzzle_lang::is_puzzle_source_path(&canonical) {
        return None;
    }
    Some(canonical)
}

fn build_workspace_import_graph(documents: &[WorkspacePuzzleDocument]) -> WorkspaceImportGraph {
    let mut graph = WorkspaceImportGraph::default();
    let paths = documents
        .iter()
        .map(|document| document.path.clone())
        .collect::<HashSet<_>>();
    let document_by_path = documents
        .iter()
        .map(|document| (document.path.clone(), document))
        .collect::<HashMap<_, _>>();

    for document in documents {
        for import in &document.imports {
            if paths.contains(import) {
                graph
                    .imported_by
                    .entry(import.clone())
                    .or_default()
                    .push(document.path.clone());
            }
        }
    }
    for imported_by in graph.imported_by.values_mut() {
        sort_parent_game_paths(imported_by);
        imported_by.dedup();
    }

    let mut game_entries = documents
        .iter()
        .filter(|document| document.declares_game_entry)
        .map(|document| document.path.clone())
        .collect::<Vec<_>>();
    sort_parent_game_paths(&mut game_entries);
    for game_path in game_entries {
        graph
            .parent_game_by_path
            .entry(game_path.clone())
            .or_insert_with(|| game_path.clone());
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();
        seen.insert(game_path.clone());
        if let Some(game_document) = document_by_path.get(&game_path) {
            queue.extend(game_document.imports.iter().cloned());
        }
        while let Some(import_path) = queue.pop_front() {
            if !paths.contains(&import_path) || !seen.insert(import_path.clone()) {
                continue;
            }
            graph
                .parent_game_by_path
                .entry(import_path.clone())
                .or_insert_with(|| game_path.clone());
            if let Some(document) = document_by_path.get(&import_path) {
                queue.extend(document.imports.iter().cloned());
            }
        }
    }

    graph
}

fn sort_parent_game_paths(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| {
        let left_dir = left.parent().unwrap_or_else(|| Path::new(""));
        let right_dir = right.parent().unwrap_or_else(|| Path::new(""));
        preview_entry_rank(left, left_dir)
            .cmp(&preview_entry_rank(right, right_dir))
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
}

fn display_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn preview_entry_rank(path: &Path, dir: &Path) -> usize {
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

fn editor_document_root(active_path: &Path) -> PathBuf {
    let mut root = PathBuf::new();
    for component in active_path.components() {
        root.push(component.as_os_str());
        if component.as_os_str() == "games" {
            return root;
        }
    }
    active_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn collect_workspace_files(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if should_skip_workspace_dir(&path) {
                continue;
            }
            collect_workspace_files(&path, paths)?;
        } else if file_type.is_file() && is_workspace_file(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn load_workspace_folders(workspace_root: &Path) -> Result<Vec<String>, AppError> {
    let mut paths = Vec::new();
    collect_workspace_folders(workspace_root, workspace_root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_workspace_folders(
    root: &Path,
    dir: &Path,
    paths: &mut Vec<String>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if !file_type.is_dir() || should_skip_workspace_dir(&path) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(|error| {
            AppError::Config(format!(
                "workspace folder is outside {}: {error}",
                root.display()
            ))
        })?;
        paths.push(relative.display().to_string().replace('\\', "/"));
        collect_workspace_folders(root, &path, paths)?;
    }
    Ok(())
}

fn read_workspace_text_file(path: &Path, workspace_root: &Path) -> Result<String, AppError> {
    ensure_path_under_root(path, workspace_root)?;
    Ok(fs::read_to_string(path)?)
}

fn read_workspace_bytes(path: &Path, workspace_root: &Path) -> Result<Vec<u8>, AppError> {
    ensure_path_under_root(path, workspace_root)?;
    Ok(fs::read(path)?)
}

fn ensure_path_under_root(path: &Path, workspace_root: &Path) -> Result<PathBuf, AppError> {
    let workspace_root = workspace_root.canonicalize()?;
    let canonical_path = path.canonicalize()?;
    if canonical_path.starts_with(&workspace_root) {
        Ok(canonical_path)
    } else {
        Err(AppError::Config(format!(
            "can only read files under {}",
            workspace_root.display()
        )))
    }
}

fn should_skip_workspace_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    SKIPPED_WORKSPACE_DIRS.contains(&name)
}

fn is_workspace_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
        "puzzle"
            | "puzzle3"
            | "css"
            | "js"
            | "mjs"
            | "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "gif"
            | "mp3"
            | "wav"
            | "ogg"
            | "json"
            | "txt"
            | "md"
    )
}

fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
        "puzzle" | "puzzle3" | "css" | "js" | "mjs" | "svg" | "json" | "txt" | "md"
    )
}

fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "css" => "text/css",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "png" => "image/png",
        "puzzle" | "puzzle3" | "txt" | "md" => "text/plain",
        "svg" => "image/svg+xml",
        "wav" => "audio/wav",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(feature = "editor-docs")]
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(feature = "embedded-assets")]
fn bind_listener(preferred_port: u16) -> io::Result<(TcpListener, u16)> {
    for offset in 0..100 {
        let port = preferred_port.saturating_add(offset);
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return Ok((listener, port)),
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "no available port in requested range",
    ))
}

#[cfg(feature = "embedded-assets")]
fn handle_connection(mut stream: TcpStream, service: Arc<EditorService>) -> Result<(), AppError> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };
    let response = route(&request, &service);
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

#[cfg(feature = "embedded-assets")]
#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

#[cfg(feature = "embedded-assets")]
fn read_request(stream: &mut TcpStream) -> Result<Option<HttpRequest>, AppError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut header_end = None;
    let mut content_length = 0;

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);

        if header_end.is_none() {
            if let Some(end) = find_header_end(&bytes) {
                header_end = Some(end);
                content_length = parse_content_length(&bytes[..end]);
            }
        }

        if let Some(end) = header_end {
            if bytes.len() >= end + 4 + content_length {
                break;
            }
        }
    }

    let Some(end) = header_end else {
        return Err(AppError::Config("invalid HTTP request".to_string()));
    };
    let header = String::from_utf8_lossy(&bytes[..end]);
    let mut request_line = header.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let mut path = request_line.next().unwrap_or("/").to_string();
    if let Some(query_index) = path.find('?') {
        path.truncate(query_index);
    }

    let body_start = end + 4;
    let body_end = body_start.saturating_add(content_length).min(bytes.len());
    let body = String::from_utf8_lossy(&bytes[body_start..body_end]).into_owned();

    Ok(Some(HttpRequest { method, path, body }))
}

#[cfg(feature = "embedded-assets")]
fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(feature = "embedded-assets")]
fn parse_content_length(header: &[u8]) -> usize {
    String::from_utf8_lossy(header)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

#[cfg(feature = "embedded-assets")]
fn route(request: &HttpRequest, service: &EditorService) -> Vec<u8> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") | ("GET", "/editor") => {
            let html = editor_html_with_docs();
            http_ok("text/html; charset=utf-8", &html)
        }
        ("GET", "/favicon.svg") => http_ok("image/svg+xml", FAVICON_SVG),
        ("GET", "/editor.css") => http_ok("text/css; charset=utf-8", EDITOR_CSS),
        ("GET", "/sound-tools.js") | ("GET", "/sound-generator.js") => {
            http_ok("text/javascript; charset=utf-8", &sound_tools_js())
        }
        ("GET", "/renderer.css") => http_ok("text/css; charset=utf-8", RENDERER_CSS),
        ("GET", "/game.css") => http_ok("text/css; charset=utf-8", &service.state().game_css),
        ("GET", "/editor_boot.js") => http_ok("text/javascript; charset=utf-8", EDITOR_BOOT_JS),
        ("GET", "/editor_codemirror.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_CODEMIRROR_JS)
        }
        ("GET", "/editor_runtime.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_RUNTIME_JS)
        }
        ("GET", "/editor_analysis_worker.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_ANALYSIS_WORKER_JS)
        }
        ("GET", "/editor_dom.js") => http_ok("text/javascript; charset=utf-8", EDITOR_DOM_JS),
        ("GET", "/editor_workspace.js") => {
            http_ok("text/javascript; charset=utf-8", &editor_workspace_js())
        }
        ("GET", "/editor_color.js") => http_ok("text/javascript; charset=utf-8", EDITOR_COLOR_JS),
        ("GET", "/editor_source.js") => http_ok("text/javascript; charset=utf-8", EDITOR_SOURCE_JS),
        ("GET", "/editor_level3d.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_LEVEL3D_JS)
        }
        ("GET", "/editor_workbench.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_WORKBENCH_JS)
        }
        ("GET", "/editor_import_export.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_IMPORT_EXPORT_JS)
        }
        ("GET", "/editor.js") => http_ok("text/javascript; charset=utf-8", EDITOR_JS),
        ("GET", "/editor_sprite_document.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_SPRITE_DOCUMENT_JS)
        }
        ("GET", "/editor_sprite.js") => http_ok("text/javascript; charset=utf-8", EDITOR_SPRITE_JS),
        ("GET", "/puzzle3_visual_core.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE3_VISUAL_CORE_JS)
        }
        ("GET", "/editor_sprite3d.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_SPRITE3D_JS)
        }
        ("GET", "/editor_sounds.js") => http_ok("text/javascript; charset=utf-8", EDITOR_SOUNDS_JS),
        ("GET", "/editor_commands.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_COMMANDS_JS)
        }
        ("GET", "/wasm/puzzle_wasm.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE_WASM_JS)
        }
        ("GET", "/wasm/puzzle_wasm_bg.wasm") => http_bytes("application/wasm", PUZZLE_WASM_BG),
        ("GET", "/wasm_game/puzzle_wasm_game.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE_GAME_WASM_JS)
        }
        ("GET", "/wasm_game/puzzle_wasm_game_bg.wasm") => {
            http_bytes("application/wasm", PUZZLE_GAME_WASM_BG)
        }
        ("GET", "/wasm_core/puzzle_core_wasm.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE_CORE_WASM_JS)
        }
        ("GET", "/wasm_core/puzzle_core_wasm_bg.wasm") => {
            http_bytes("application/wasm", PUZZLE_CORE_WASM_BG)
        }
        ("GET", "/renderer.js") => http_ok("text/javascript; charset=utf-8", RENDERER_JS),
        ("GET", "/game.visuals.js") => http_ok(
            "text/javascript; charset=utf-8",
            &service.state().base_game_visuals_js,
        ),
        ("GET", "/api/source") => match service.source_json() {
            Ok(source) => http_ok("application/json; charset=utf-8", &source),
            Err(error) => http_error(500, &error.to_string()),
        },
        ("POST", "/api/load-workspace-document") => {
            let load = LoadWorkspaceDocumentRequest::from_body(&request.body);
            match service.load_workspace_document_json(&load) {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/preview") => {
            let preview = PreviewRequest::from_body(&request.body, service.state());
            match service.compile_preview(&preview) {
                Ok(body) => http_ok("text/html; charset=utf-8", &body),
                Err(AppError::Diagnostics(report)) => http_diagnostic_error(400, &report),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/highlight") => {
            let source = if request.body.trim_start().starts_with('{') {
                json_string_field(&request.body, "source").unwrap_or_default()
            } else {
                request.body.clone()
            };
            match service.highlight_json(&source) {
                Ok(body) => http_ok("application/json; charset=utf-8", &body),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/save") => {
            let save = SaveRequest::from_body(&request.body, service.state());
            match service.save_source_file(&save) {
                Ok(()) => http_ok("application/json; charset=utf-8", "{\"ok\":true}"),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/create-source-file") => {
            let create = CreateSourceFileRequest::from_body(&request.body);
            match service.create_source_file(&create) {
                Ok(path) => {
                    let mut body = String::from("{\"ok\":true,\"puzzlePath\":");
                    push_json_string(&mut body, &path.display().to_string());
                    body.push('}');
                    http_ok("application/json; charset=utf-8", &body)
                }
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/create-source-folder") => {
            let create = CreateSourceFolderRequest::from_body(&request.body);
            match service.create_source_folder(&create) {
                Ok(path) => {
                    let mut body = String::from("{\"ok\":true,\"folderPath\":");
                    push_json_string(&mut body, &path.display().to_string());
                    body.push('}');
                    http_ok("application/json; charset=utf-8", &body)
                }
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/rename-workspace-entry") => {
            let rename = RenameWorkspaceEntryRequest::from_body(&request.body);
            match service.rename_workspace_entry(&rename) {
                Ok(path) => {
                    let mut body = String::from("{\"ok\":true,\"path\":");
                    push_json_string(&mut body, &path.display().to_string());
                    body.push('}');
                    http_ok("application/json; charset=utf-8", &body)
                }
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        ("POST", "/api/delete-workspace-entry") => {
            let delete = DeleteWorkspaceEntryRequest::from_body(&request.body);
            match service.delete_workspace_entry(&delete) {
                Ok(()) => http_ok("application/json; charset=utf-8", "{\"ok\":true}"),
                Err(error) => http_error(400, &error.to_string()),
            }
        }
        _ => http_error(404, "not found"),
    }
}

pub struct PreviewRequest {
    pub source: String,
    pub puzzle_path: String,
    pub game_css: String,
}

impl PreviewRequest {
    pub fn new(
        source: impl Into<String>,
        puzzle_path: impl Into<String>,
        game_css: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            puzzle_path: puzzle_path.into(),
            game_css: game_css.into(),
        }
    }

    pub fn from_body(body: &str, state: &EditorState) -> Self {
        if body.trim_start().starts_with('{') {
            return Self {
                source: json_string_field(body, "source").unwrap_or_default(),
                puzzle_path: json_string_field(body, "puzzlePath")
                    .unwrap_or_else(|| state.puzzle_path.clone()),
                game_css: json_string_field(body, "gameCss")
                    .unwrap_or_else(|| state.game_css.clone()),
            };
        }
        Self {
            source: body.to_string(),
            puzzle_path: state.puzzle_path.clone(),
            game_css: state.game_css.clone(),
        }
    }
}

pub struct SaveRequest {
    pub source: String,
    pub puzzle_path: String,
    pub content_loaded: bool,
}

impl SaveRequest {
    pub fn new(source: impl Into<String>, puzzle_path: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            puzzle_path: puzzle_path.into(),
            content_loaded: true,
        }
    }

    pub fn from_body(body: &str, state: &EditorState) -> Self {
        if body.trim_start().starts_with('{') {
            return Self {
                source: json_string_field(body, "source").unwrap_or_default(),
                puzzle_path: json_string_field(body, "puzzlePath")
                    .unwrap_or_else(|| state.puzzle_path.clone()),
                content_loaded: json_bool_field(body, "contentLoaded").unwrap_or(false),
            };
        }
        Self {
            source: body.to_string(),
            puzzle_path: state.puzzle_path.clone(),
            content_loaded: true,
        }
    }
}

pub struct LoadWorkspaceDocumentRequest {
    pub puzzle_path: String,
}

impl LoadWorkspaceDocumentRequest {
    pub fn new(puzzle_path: impl Into<String>) -> Self {
        Self {
            puzzle_path: puzzle_path.into(),
        }
    }

    pub fn from_body(body: &str) -> Self {
        Self {
            puzzle_path: json_string_field(body, "puzzlePath").unwrap_or_default(),
        }
    }
}

pub struct CreateSourceFileRequest {
    pub source: String,
    pub puzzle_path: String,
}

impl CreateSourceFileRequest {
    pub fn new(source: impl Into<String>, puzzle_path: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            puzzle_path: puzzle_path.into(),
        }
    }

    pub fn from_body(body: &str) -> Self {
        Self {
            source: json_string_field(body, "source").unwrap_or_default(),
            puzzle_path: json_string_field(body, "puzzlePath").unwrap_or_default(),
        }
    }
}

pub struct CreateSourceFolderRequest {
    pub folder_path: String,
}

impl CreateSourceFolderRequest {
    pub fn new(folder_path: impl Into<String>) -> Self {
        Self {
            folder_path: folder_path.into(),
        }
    }

    pub fn from_body(body: &str) -> Self {
        Self {
            folder_path: json_string_field(body, "folderPath").unwrap_or_default(),
        }
    }
}

pub struct RenameWorkspaceEntryRequest {
    pub from_path: String,
    pub to_path: String,
}

impl RenameWorkspaceEntryRequest {
    pub fn new(from_path: impl Into<String>, to_path: impl Into<String>) -> Self {
        Self {
            from_path: from_path.into(),
            to_path: to_path.into(),
        }
    }

    pub fn from_body(body: &str) -> Self {
        Self {
            from_path: json_string_field(body, "fromPath").unwrap_or_default(),
            to_path: json_string_field(body, "toPath").unwrap_or_default(),
        }
    }
}

pub struct DeleteWorkspaceEntryRequest {
    pub entry_path: String,
}

impl DeleteWorkspaceEntryRequest {
    pub fn new(entry_path: impl Into<String>) -> Self {
        Self {
            entry_path: entry_path.into(),
        }
    }

    pub fn from_body(body: &str) -> Self {
        Self {
            entry_path: json_string_field(body, "entryPath").unwrap_or_default(),
        }
    }
}

fn save_source_file(request: &SaveRequest, state: &EditorState) -> Result<(), AppError> {
    if !request.content_loaded {
        return Err(AppError::Config(
            "cannot save unloaded workspace document".to_string(),
        ));
    }
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let requested_path = if request.puzzle_path.trim().is_empty() {
        PathBuf::from(&state.puzzle_path)
    } else {
        resolve_workspace_request_path(&request.puzzle_path, &workspace_root_path)?
    };
    let canonical_requested = requested_path.canonicalize()?;
    if !canonical_requested.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only save files under {}",
            workspace_root.display()
        )));
    }
    if !is_text_file(&canonical_requested) {
        return Err(AppError::Config(
            "can only save text workspace files".to_string(),
        ));
    }
    fs::write(canonical_requested, &request.source)?;
    Ok(())
}

fn load_workspace_document(
    requested_path: &Path,
    state: &EditorState,
) -> Result<EditorDocument, AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let requested_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        resolve_workspace_request_path(&requested_path.display().to_string(), &workspace_root_path)?
    };
    let canonical_requested = requested_path.canonicalize()?;
    if !canonical_requested.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only load files under {}",
            workspace_root.display()
        )));
    }
    if !canonical_requested.is_file() || !is_workspace_file(&canonical_requested) {
        return Err(AppError::Config(format!(
            "can only load workspace files: {}",
            canonical_requested.display()
        )));
    }

    let metadata = state.documents.iter().find(|document| {
        PathBuf::from(&document.puzzle_path)
            .canonicalize()
            .map(|path| path == canonical_requested)
            .unwrap_or(false)
    });
    let mime_type = metadata
        .map(|document| document.mime_type.clone())
        .unwrap_or_else(|| mime_type(&canonical_requested).to_string());
    let imported_by = metadata
        .map(|document| document.imported_by.clone())
        .unwrap_or_default();
    let parent_game_path = metadata
        .map(|document| document.parent_game_path.clone())
        .unwrap_or_default();
    let game_css_path = if parent_game_path.trim().is_empty() {
        canonical_requested.clone()
    } else {
        resolve_workspace_request_path(&parent_game_path, &workspace_root_path)?
    };

    if is_text_file(&canonical_requested) {
        let source = read_workspace_text_file(&canonical_requested, &workspace_root)?;
        let declares_game_entry = puzzle_lang::is_puzzle_source_path(&canonical_requested)
            && puzzle_lang::source_declares_game_entry(&source);
        let game_css = if puzzle_lang::is_puzzle_source_path(&canonical_requested) {
            load_game_css(&game_css_path, &workspace_root)?
        } else {
            String::new()
        };
        return Ok(EditorDocument {
            puzzle_path: canonical_requested.display().to_string(),
            encoding: "text".to_string(),
            mime_type,
            source,
            data_url: String::new(),
            content_loaded: true,
            declares_game_entry,
            preview_html: String::new(),
            preview_error: String::new(),
            game_css,
            imported_by,
            parent_game_path,
        });
    }

    let bytes = read_workspace_bytes(&canonical_requested, &workspace_root)?;
    Ok(EditorDocument {
        puzzle_path: canonical_requested.display().to_string(),
        encoding: "data_url".to_string(),
        mime_type: mime_type.clone(),
        source: String::new(),
        data_url: format!("data:{mime_type};base64,{}", base64_encode(&bytes)),
        content_loaded: true,
        declares_game_entry: false,
        preview_html: String::new(),
        preview_error: String::new(),
        game_css: String::new(),
        imported_by,
        parent_game_path,
    })
}

fn create_source_file(
    request: &CreateSourceFileRequest,
    state: &EditorState,
) -> Result<PathBuf, AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let requested_path =
        resolve_workspace_request_path(&request.puzzle_path, &workspace_root_path)?;
    if !puzzle_lang::is_puzzle_source_path(&requested_path) {
        return Err(AppError::Config(
            "can only create .puzzle or .puzzle3 source files".to_string(),
        ));
    }
    if requested_path.exists() {
        return Err(AppError::Config(format!(
            "file already exists: {}",
            requested_path.display()
        )));
    }
    let parent = requested_path
        .parent()
        .ok_or_else(|| AppError::Config("new file needs a parent folder".to_string()))?
        .canonicalize()?;
    if !parent.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only create files under {}",
            workspace_root.display()
        )));
    }
    fs::write(&requested_path, &request.source)?;
    requested_path.canonicalize().map_err(AppError::Io)
}

fn create_source_folder(
    request: &CreateSourceFolderRequest,
    state: &EditorState,
) -> Result<PathBuf, AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let requested_path =
        resolve_workspace_request_path(&request.folder_path, &workspace_root_path)?;
    if requested_path.exists() {
        return Err(AppError::Config(format!(
            "folder already exists: {}",
            requested_path.display()
        )));
    }
    let parent = requested_path
        .parent()
        .ok_or_else(|| AppError::Config("new folder needs a parent folder".to_string()))?
        .canonicalize()?;
    if !parent.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only create folders under {}",
            workspace_root.display()
        )));
    }
    fs::create_dir(&requested_path)?;
    requested_path.canonicalize().map_err(AppError::Io)
}

fn rename_workspace_entry(
    request: &RenameWorkspaceEntryRequest,
    state: &EditorState,
) -> Result<PathBuf, AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let from_path = resolve_workspace_request_path(&request.from_path, &workspace_root_path)?;
    let to_path = resolve_workspace_request_path(&request.to_path, &workspace_root_path)?;
    let canonical_from = from_path.canonicalize()?;
    if canonical_from == workspace_root {
        return Err(AppError::Config(
            "cannot rename the workspace root".to_string(),
        ));
    }
    if !canonical_from.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only rename files under {}",
            workspace_root.display()
        )));
    }
    if to_path.exists() {
        return Err(AppError::Config(format!(
            "destination already exists: {}",
            to_path.display()
        )));
    }
    let parent = to_path
        .parent()
        .ok_or_else(|| AppError::Config("renamed entry needs a parent folder".to_string()))?
        .canonicalize()?;
    if !parent.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only rename files under {}",
            workspace_root.display()
        )));
    }
    let metadata = fs::metadata(&canonical_from)?;
    if metadata.is_file() && !is_workspace_file(&canonical_from) {
        return Err(AppError::Config(
            "can only rename workspace files".to_string(),
        ));
    }
    fs::rename(&canonical_from, &to_path)?;
    to_path.canonicalize().map_err(AppError::Io)
}

fn delete_workspace_entry(
    request: &DeleteWorkspaceEntryRequest,
    state: &EditorState,
) -> Result<(), AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let workspace_root = workspace_root_path.canonicalize()?;
    let entry_path = resolve_workspace_request_path(&request.entry_path, &workspace_root_path)?;
    let canonical_entry = entry_path.canonicalize()?;
    if canonical_entry == workspace_root {
        return Err(AppError::Config(
            "cannot delete the workspace root".to_string(),
        ));
    }
    if !canonical_entry.starts_with(&workspace_root) {
        return Err(AppError::Config(format!(
            "can only delete files under {}",
            workspace_root.display()
        )));
    }
    let metadata = fs::metadata(&canonical_entry)?;
    if metadata.is_file() {
        if !is_workspace_file(&canonical_entry) {
            return Err(AppError::Config(
                "can only delete workspace files".to_string(),
            ));
        }
        fs::remove_file(&canonical_entry)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(&canonical_entry)?;
    } else {
        return Err(AppError::Config(
            "can only delete workspace files or folders".to_string(),
        ));
    }
    Ok(())
}

fn resolve_workspace_request_path(
    requested_path: &str,
    workspace_root: &Path,
) -> Result<PathBuf, AppError> {
    let requested_path = requested_path.trim();
    if requested_path.is_empty() {
        return Err(AppError::Config("missing workspace file path".to_string()));
    }
    let raw = PathBuf::from(requested_path);
    if raw.is_absolute() {
        return Ok(raw);
    }

    let normalized = requested_path.replace('\\', "/");
    let root = workspace_root.canonicalize()?;
    let root_text = root.display().to_string().replace('\\', "/");
    let root_without_slash = root_text.trim_start_matches('/');
    if !root_without_slash.is_empty()
        && (normalized == root_without_slash
            || normalized.starts_with(&format!("{root_without_slash}/")))
        && root_text.starts_with('/')
    {
        return Ok(PathBuf::from(format!("/{normalized}")));
    }

    Ok(root.join(raw))
}

fn json_string_field(source: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut index = source.find(&needle)? + needle.len();
    let bytes = source.as_bytes();
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b':') {
        return None;
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    index += 1;
    let mut out = String::new();
    let mut chars = source[index..].chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let mut value = 0_u32;
                    for _ in 0..4 {
                        value = value.checked_mul(16)?;
                        value += chars.next()?.to_digit(16)?;
                    }
                    out.push(char::from_u32(value)?);
                }
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

fn json_bool_field(source: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let mut index = source.find(&needle)? + needle.len();
    let bytes = source.as_bytes();
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b':') {
        return None;
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if source[index..].starts_with("true") {
        return Some(true);
    }
    if source[index..].starts_with("false") {
        return Some(false);
    }
    None
}

#[cfg(feature = "embedded-assets")]
fn export_pages_editor_html(state: &EditorState) -> Result<String, AppError> {
    let mut data = String::new();
    editor_seed_json(&mut data, state);
    let data = escape_script_json(&data);
    let sound_tools_js = escape_script(&sound_tools_js());

    let editor_html = editor_html_with_docs();

    Ok(editor_html
        .replace(
            r#"<script src="sound-generator.js"></script>"#,
            &format!("<script>\n{sound_tools_js}\n</script>"),
        )
        .replace(
            r#"<script src="editor_dom.js"></script>"#,
            &format!(
                "<script>\nwindow.PuzzleEditorSeed = JSON.parse(\"{data}\");\n</script>\n<script src=\"editor_dom.js\"></script>"
            ),
        ))
}

#[cfg(feature = "embedded-assets")]
fn write_pages_editor_site(output_path: &Path, html: String) -> Result<(), AppError> {
    let output_dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir)?;
    fs::write(output_path, html)?;

    write_text_asset(output_dir, "favicon.svg", FAVICON_SVG)?;
    write_text_asset(output_dir, "editor.css", EDITOR_CSS)?;
    write_text_asset(output_dir, "editor_runtime.js", EDITOR_RUNTIME_JS)?;
    write_text_asset(
        output_dir,
        "editor_analysis_worker.js",
        EDITOR_ANALYSIS_WORKER_JS,
    )?;
    write_text_asset(output_dir, "editor_boot.js", EDITOR_BOOT_JS)?;
    write_text_asset(output_dir, "editor_codemirror.js", EDITOR_CODEMIRROR_JS)?;
    write_text_asset(output_dir, "editor_dom.js", EDITOR_DOM_JS)?;
    write_text_asset(output_dir, "editor_workspace.js", &editor_workspace_js())?;
    write_text_asset(output_dir, "editor_color.js", EDITOR_COLOR_JS)?;
    write_text_asset(output_dir, "editor_source.js", EDITOR_SOURCE_JS)?;
    write_text_asset(output_dir, "editor_level3d.js", EDITOR_LEVEL3D_JS)?;
    write_text_asset(output_dir, "editor_workbench.js", EDITOR_WORKBENCH_JS)?;
    write_text_asset(
        output_dir,
        "editor_import_export.js",
        EDITOR_IMPORT_EXPORT_JS,
    )?;
    write_text_asset(output_dir, "editor.js", EDITOR_JS)?;
    write_text_asset(
        output_dir,
        "editor_sprite_document.js",
        EDITOR_SPRITE_DOCUMENT_JS,
    )?;
    write_text_asset(output_dir, "editor_sprite.js", EDITOR_SPRITE_JS)?;
    write_text_asset(output_dir, "editor_sprite3d.js", EDITOR_SPRITE3D_JS)?;
    write_text_asset(output_dir, "editor_sounds.js", EDITOR_SOUNDS_JS)?;
    write_text_asset(output_dir, "editor_commands.js", EDITOR_COMMANDS_JS)?;
    write_text_asset(output_dir, "renderer.css", RENDERER_CSS)?;
    write_text_asset(output_dir, "renderer.js", RENDERER_JS)?;
    write_text_asset(output_dir, "puzzle3_visual_core.js", PUZZLE3_VISUAL_CORE_JS)?;

    let wasm_dir = output_dir.join("wasm");
    fs::create_dir_all(&wasm_dir)?;
    fs::write(wasm_dir.join("puzzle_wasm.js"), PUZZLE_WASM_JS)?;
    fs::write(wasm_dir.join("puzzle_wasm_bg.wasm"), PUZZLE_WASM_BG)?;

    let game_wasm_dir = output_dir.join("wasm_game");
    fs::create_dir_all(&game_wasm_dir)?;
    fs::write(
        game_wasm_dir.join("puzzle_wasm_game.js"),
        PUZZLE_GAME_WASM_JS,
    )?;
    fs::write(
        game_wasm_dir.join("puzzle_wasm_game_bg.wasm"),
        PUZZLE_GAME_WASM_BG,
    )?;

    let core_wasm_dir = output_dir.join("wasm_core");
    fs::create_dir_all(&core_wasm_dir)?;
    fs::write(
        core_wasm_dir.join("puzzle_core_wasm.js"),
        PUZZLE_CORE_WASM_JS,
    )?;
    fs::write(
        core_wasm_dir.join("puzzle_core_wasm_bg.wasm"),
        PUZZLE_CORE_WASM_BG,
    )?;

    Ok(())
}

#[cfg(feature = "embedded-assets")]
fn write_text_asset(output_dir: &Path, name: &str, contents: &str) -> Result<(), AppError> {
    fs::write(output_dir.join(name), contents)?;
    Ok(())
}

#[cfg(feature = "embedded-assets")]
fn editor_html_with_docs() -> String {
    EDITOR_HTML.replace("<!-- PUZZLESTUDIO_EDITOR_DOCS -->", &editor_docs_html())
}

#[cfg(feature = "embedded-assets")]
fn editor_workspace_js() -> String {
    EDITOR_WORKSPACE_JS.to_string()
}

#[cfg(feature = "editor-docs")]
pub fn editor_docs_html() -> String {
    render_editor_docs()
}

#[cfg(feature = "editor-docs")]
struct EditorDocsPage {
    id: &'static str,
    title: &'static str,
    markdown: &'static str,
}

#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_PAGES: &[EditorDocsPage] = &[
    // Basic is ordered as a shortest path from an empty file to a playable game.
    EditorDocsPage {
        id: "start",
        title: "Start",
        markdown: EDITOR_DOCS_MARKDOWN,
    },
    EditorDocsPage {
        id: "metadata",
        title: "Metadata",
        markdown: EDITOR_DOCS_METADATA_MARKDOWN,
    },
    EditorDocsPage {
        id: "puzzle-block",
        title: "Puzzle Block",
        markdown: EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN,
    },
    EditorDocsPage {
        id: "slots",
        title: "Slots",
        markdown: EDITOR_DOCS_SLOTS_MARKDOWN,
    },
    EditorDocsPage {
        id: "legend",
        title: "Legend",
        markdown: EDITOR_DOCS_LEGEND_MARKDOWN,
    },
    EditorDocsPage {
        id: "levels",
        title: "Levels",
        markdown: EDITOR_DOCS_LEVELS_MARKDOWN,
    },
    EditorDocsPage {
        id: "rewrite-rules",
        title: "Rewrite Rules",
        markdown: EDITOR_DOCS_REWRITE_RULES_MARKDOWN,
    },
    EditorDocsPage {
        id: "input-rules",
        title: "Input Rules",
        markdown: EDITOR_DOCS_INPUT_RULES_MARKDOWN,
    },
    EditorDocsPage {
        id: "movement",
        title: "Movement",
        markdown: EDITOR_DOCS_MOVEMENT_MARKDOWN,
    },
    EditorDocsPage {
        id: "win-conditions",
        title: "Win Conditions",
        markdown: EDITOR_DOCS_WIN_CONDITIONS_MARKDOWN,
    },
    EditorDocsPage {
        id: "sprites",
        title: "Sprites",
        markdown: EDITOR_DOCS_SPRITES_MARKDOWN,
    },
    // Advanced is grouped by the responsibility an author is extending.
    EditorDocsPage {
        id: "guards",
        title: "Guards",
        markdown: EDITOR_DOCS_GUARDS_MARKDOWN,
    },
    EditorDocsPage {
        id: "fix",
        title: "Fix",
        markdown: EDITOR_DOCS_FIX_MARKDOWN,
    },
    EditorDocsPage {
        id: "routines",
        title: "Routines",
        markdown: EDITOR_DOCS_ROUTINES_MARKDOWN,
    },
    EditorDocsPage {
        id: "rule-application",
        title: "Rule Application",
        markdown: EDITOR_DOCS_RULE_APPLICATION_MARKDOWN,
    },
    EditorDocsPage {
        id: "patterns",
        title: "Patterns",
        markdown: EDITOR_DOCS_PATTERNS_MARKDOWN,
    },
    EditorDocsPage {
        id: "rule-effects",
        title: "Rule Effects",
        markdown: EDITOR_DOCS_RULE_EFFECTS_MARKDOWN,
    },
    EditorDocsPage {
        id: "messages",
        title: "Messages",
        markdown: EDITOR_DOCS_MESSAGES_MARKDOWN,
    },
    EditorDocsPage {
        id: "variables",
        title: "Variables",
        markdown: EDITOR_DOCS_VARIABLES_MARKDOWN,
    },
    EditorDocsPage {
        id: "mark",
        title: "Marks",
        markdown: EDITOR_DOCS_MARK_MARKDOWN,
    },
    EditorDocsPage {
        id: "conditions",
        title: "Conditions",
        markdown: EDITOR_DOCS_CONDITIONS_MARKDOWN,
    },
    EditorDocsPage {
        id: "lifecycle",
        title: "Lifecycle",
        markdown: EDITOR_DOCS_LIFECYCLE_MARKDOWN,
    },
    EditorDocsPage {
        id: "groups",
        title: "Groups",
        markdown: EDITOR_DOCS_GROUPS_MARKDOWN,
    },
    EditorDocsPage {
        id: "tags",
        title: "Tags",
        markdown: EDITOR_DOCS_TAGS_MARKDOWN,
    },
    EditorDocsPage {
        id: "maps-expansion",
        title: "Maps & Expansion",
        markdown: EDITOR_DOCS_MAPS_EXPANSION_MARKDOWN,
    },
    EditorDocsPage {
        id: "level-local-legend",
        title: "Level Legend",
        markdown: EDITOR_DOCS_LEVEL_LOCAL_LEGEND_MARKDOWN,
    },
    EditorDocsPage {
        id: "imports",
        title: "Imports",
        markdown: EDITOR_DOCS_IMPORTS_MARKDOWN,
    },
    EditorDocsPage {
        id: "scenes",
        title: "Scenes",
        markdown: EDITOR_DOCS_SCENES_MARKDOWN,
    },
    EditorDocsPage {
        id: "scene-layout",
        title: "Scene Layout",
        markdown: EDITOR_DOCS_SCENE_LAYOUT_MARKDOWN,
    },
    EditorDocsPage {
        id: "semantic-inputs",
        title: "Inputs",
        markdown: EDITOR_DOCS_SEMANTIC_INPUTS_MARKDOWN,
    },
    EditorDocsPage {
        id: "menus",
        title: "Menus",
        markdown: EDITOR_DOCS_MENUS_MARKDOWN,
    },
    EditorDocsPage {
        id: "scene-state-effects",
        title: "Scene State & Effects",
        markdown: EDITOR_DOCS_SCENE_STATE_EFFECTS_MARKDOWN,
    },
    EditorDocsPage {
        id: "display",
        title: "Display",
        markdown: EDITOR_DOCS_DISPLAY_MARKDOWN,
    },
    EditorDocsPage {
        id: "theme",
        title: "Theme",
        markdown: EDITOR_DOCS_THEME_MARKDOWN,
    },
    EditorDocsPage {
        id: "rendering",
        title: "Rendering",
        markdown: EDITOR_DOCS_RENDERING_MARKDOWN,
    },
    EditorDocsPage {
        id: "sprite-shapes",
        title: "Sprite Shapes & Animation",
        markdown: EDITOR_DOCS_SPRITE_SHAPES_MARKDOWN,
    },
    EditorDocsPage {
        id: "3d",
        title: "3D",
        markdown: EDITOR_DOCS_3D_MARKDOWN,
    },
    EditorDocsPage {
        id: "sounds",
        title: "Sounds",
        markdown: EDITOR_DOCS_SOUNDS_MARKDOWN,
    },
    EditorDocsPage {
        id: "assets",
        title: "Assets",
        markdown: EDITOR_DOCS_ASSETS_MARKDOWN,
    },
];

#[cfg(feature = "editor-docs")]
fn editor_docs_level(page: &EditorDocsPage) -> &'static str {
    match page.id {
        "start" | "metadata" | "puzzle-block" | "slots" | "legend" | "levels" | "rewrite-rules"
        | "input-rules" | "movement" | "win-conditions" | "sprites" => "Basic",
        _ => "Advanced",
    }
}

#[cfg(feature = "editor-docs")]
fn editor_docs_advanced_chapter(page: &EditorDocsPage) -> Option<&'static str> {
    match page.id {
        "guards" | "fix" | "routines" | "rule-application" | "patterns" | "rule-effects" => {
            Some("Rules & Patterns")
        }
        "messages" | "variables" | "mark" | "conditions" | "lifecycle" => Some("State & Lifecycle"),
        "groups" | "tags" | "maps-expansion" => Some("Objects & Selectors"),
        "level-local-legend" => Some("Levels"),
        "imports" => Some("Project Structure"),
        "scenes" | "scene-layout" | "semantic-inputs" | "menus" | "scene-state-effects" => {
            Some("Scenes & UI")
        }
        "display" | "theme" | "rendering" | "sprite-shapes" => Some("Visuals"),
        "3d" => Some("3D"),
        "sounds" | "assets" => Some("Assets & Sound"),
        _ => None,
    }
}

#[cfg(feature = "editor-docs")]
fn render_editor_docs() -> String {
    let mut out = String::from(
        "<div class=\"docs-layout\">\n<nav class=\"docs-nav\" role=\"tablist\" aria-label=\"Documents\">\n",
    );
    let mut previous_level = None;
    let mut previous_chapter = None;
    for (index, page) in EDITOR_DOCS_PAGES.iter().enumerate() {
        let level = editor_docs_level(page);
        if previous_level != Some(level) {
            out.push_str(&format!(
                "<div class=\"docs-nav-level\">{}</div>\n",
                escape_html(level)
            ));
            previous_level = Some(level);
        }
        let chapter = editor_docs_advanced_chapter(page);
        if chapter != previous_chapter {
            if let Some(chapter) = chapter {
                out.push_str(&format!(
                    "<div class=\"docs-nav-chapter\">{}</div>\n",
                    escape_html(chapter)
                ));
            }
            previous_chapter = chapter;
        }
        let selected = if index == 0 {
            " aria-selected=\"true\""
        } else {
            ""
        };
        let active_class = if index == 0 { " is-active" } else { "" };
        out.push_str(&format!(
            "<button class=\"docs-nav-button{active_class}\" type=\"button\" data-docs-page=\"{}\" role=\"tab\"{selected}>{}</button>\n",
            escape_html(page.id),
            escape_html(page.title)
        ));
    }
    out.push_str("</nav>\n<div class=\"docs-pages\">\n");
    for (index, page) in EDITOR_DOCS_PAGES.iter().enumerate() {
        out.push_str(&render_editor_docs_markdown(page, index, index == 0));
    }
    out.push_str("</div>\n</div>");
    out
}

#[cfg(feature = "editor-docs")]
fn render_editor_docs_markdown(page: &EditorDocsPage, page_index: usize, active: bool) -> String {
    let hidden = if active { "" } else { " hidden" };
    let mut out = format!(
        "<article class=\"docs-article\" data-docs-article=\"{}\"{hidden}>\n",
        escape_html(page.id)
    );
    let mut paragraph = Vec::new();
    let mut in_header = false;
    let mut header_closed = false;
    let mut in_section = false;
    let mut code_language = None::<String>;
    let mut code_lines = Vec::<String>::new();

    for line in page.markdown.lines() {
        if let Some(language) = &code_language {
            if line.trim_start().starts_with("```") {
                let source = code_lines.join("\n");
                render_docs_code_block(&mut out, language, &source);
                code_language = None;
                code_lines.clear();
            } else {
                code_lines.push(line.to_string());
            }
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_docs_paragraph(&mut out, &mut paragraph);
            continue;
        }

        if trimmed.starts_with("```") {
            flush_docs_paragraph(&mut out, &mut paragraph);
            close_docs_header(&mut out, &mut in_header, &mut header_closed);
            code_language = Some(docs_code_language(trimmed).to_string());
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("# ") {
            flush_docs_paragraph(&mut out, &mut paragraph);
            close_docs_section(&mut out, &mut in_section);
            close_docs_header(&mut out, &mut in_header, &mut header_closed);
            out.push_str("<header class=\"docs-header\">\n");
            out.push_str("<p class=\"docs-kicker\">PuzzleStudio Documents</p>\n");
            out.push_str("<h2>");
            out.push_str(&render_docs_inline(title));
            out.push_str("</h2>\n");
            in_header = true;
            header_closed = false;
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("## ") {
            flush_docs_paragraph(&mut out, &mut paragraph);
            close_docs_header(&mut out, &mut in_header, &mut header_closed);
            close_docs_section(&mut out, &mut in_section);
            let notes_class = if title == "What matters first" {
                " docs-notes"
            } else {
                ""
            };
            out.push_str(&format!("<section class=\"docs-section{notes_class}\">\n"));
            out.push_str("<h3>");
            out.push_str(&render_docs_inline(title));
            out.push_str("</h3>\n");
            in_section = true;
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    if let Some(language) = &code_language {
        let source = code_lines.join("\n");
        render_docs_code_block(&mut out, language, &source);
    }
    flush_docs_paragraph(&mut out, &mut paragraph);
    close_docs_header(&mut out, &mut in_header, &mut header_closed);
    close_docs_section(&mut out, &mut in_section);
    out.push_str(&render_editor_docs_page_links(page_index));
    out.push_str("</article>");
    out
}

#[cfg(feature = "editor-docs")]
fn docs_code_language(fence: &str) -> &str {
    fence.trim_start_matches('`').trim()
}

#[cfg(feature = "editor-docs")]
fn render_docs_code_block(out: &mut String, language: &str, source: &str) {
    out.push_str("<pre><code");
    if is_puzzle_docs_code_language(language) {
        out.push_str(" class=\"language-puzzle\"");
        out.push('>');
        out.push_str(&render_source_highlight_html(
            source,
            &puzzle_lang::highlight_source(source, puzzle_lang::PuzzleSourceProfile::Puzzle2d),
        ));
    } else {
        out.push('>');
        out.push_str(&escape_html(source));
    }
    out.push_str("</code></pre>\n");
}

#[cfg(feature = "editor-docs")]
fn is_puzzle_docs_code_language(language: &str) -> bool {
    matches!(language.trim(), "puzzle" | ".puzzle")
}

#[cfg(feature = "editor-docs")]
fn render_source_highlight_html(
    source: &str,
    highlighted: &puzzle_lang::HighlightedSource,
) -> String {
    let mut out = String::with_capacity(source.len().saturating_add(source.len() / 8));
    let mut cursor = 0usize;
    for span in &highlighted.spans {
        assert!(
            cursor <= span.start
                && span.start < span.end
                && span.end <= source.len()
                && source.is_char_boundary(span.start)
                && source.is_char_boundary(span.end),
            "language highlight spans must be ordered UTF-8 source ranges"
        );
        out.push_str(&escape_html(&source[cursor..span.start]));
        out.push_str("<span class=\"syntax-");
        out.push_str(span.kind.as_str());
        if span.transparent {
            out.push_str(" is-transparent");
        }
        out.push('"');
        if let Some(color) = &span.color {
            let property = if span.kind == puzzle_lang::SourceHighlightKind::SpritePixel {
                "--syntax-sprite-pixel-color"
            } else {
                "--syntax-color-token"
            };
            out.push_str(" style=\"");
            out.push_str(property);
            out.push_str(": ");
            out.push_str(&escape_html(color));
            out.push('"');
        }
        out.push('>');
        out.push_str(&escape_html(&source[span.start..span.end]));
        out.push_str("</span>");
        cursor = span.end;
    }
    out.push_str(&escape_html(&source[cursor..]));
    if source.ends_with('\n') {
        out.push(' ');
    }
    out
}

#[cfg(feature = "editor-docs")]
fn render_editor_docs_page_links(page_index: usize) -> String {
    let previous = page_index
        .checked_sub(1)
        .and_then(|index| EDITOR_DOCS_PAGES.get(index));
    let next = EDITOR_DOCS_PAGES.get(page_index + 1);
    if previous.is_none() && next.is_none() {
        return String::new();
    }

    let mut out =
        String::from("<footer class=\"docs-page-links\" aria-label=\"Related documents\">\n");
    if let Some(page) = previous {
        out.push_str(&render_editor_docs_page_link(
            page,
            "docs-page-link-previous",
            "Previous",
        ));
    }
    if let Some(page) = next {
        out.push_str(&render_editor_docs_page_link(
            page,
            "docs-page-link-next",
            "Next",
        ));
    }
    out.push_str("</footer>\n");
    out
}

#[cfg(feature = "editor-docs")]
fn render_editor_docs_page_link(
    page: &EditorDocsPage,
    direction_class: &str,
    direction_label: &str,
) -> String {
    format!(
        "<button class=\"docs-page-link {direction_class}\" type=\"button\" data-docs-page=\"{}\"><span class=\"docs-page-link-label\">{}</span><span class=\"docs-page-link-title\">{}</span></button>\n",
        escape_html(page.id),
        escape_html(direction_label),
        escape_html(page.title),
    )
}

#[cfg(feature = "editor-docs")]
fn flush_docs_paragraph(out: &mut String, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    out.push_str("<p>");
    out.push_str(&render_docs_inline(&paragraph.join(" ")));
    out.push_str("</p>\n");
    paragraph.clear();
}

#[cfg(feature = "editor-docs")]
fn close_docs_header(out: &mut String, in_header: &mut bool, header_closed: &mut bool) {
    if *in_header && !*header_closed {
        out.push_str("</header>\n");
        *header_closed = true;
        *in_header = false;
    }
}

#[cfg(feature = "editor-docs")]
fn close_docs_section(out: &mut String, in_section: &mut bool) {
    if *in_section {
        out.push_str("</section>\n");
        *in_section = false;
    }
}

#[cfg(feature = "editor-docs")]
fn render_docs_inline(value: &str) -> String {
    let mut out = String::new();
    for (index, part) in value.split('`').enumerate() {
        if index % 2 == 0 {
            out.push_str(&escape_html(part));
        } else {
            out.push_str("<code>");
            out.push_str(&escape_html(part));
            out.push_str("</code>");
        }
    }
    out
}

#[cfg(feature = "sound-tools")]
fn sound_tools_js() -> String {
    fn module_body(source: &str) -> String {
        source
            .lines()
            .filter(|line| !line.trim_start().starts_with("import "))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("export const ", "const ")
            .replace("export async function ", "async function ")
            .replace("export function ", "function ")
    }

    fn expose_module(source: &str, exports: &[&str]) -> String {
        let body = module_body(source);
        format!("{body}\nreturn {{{}}};", exports.join(","))
    }

    format!(
        "(() => {{
  const sfx = (() => {{
{}
  }})();
  const musicPlayer = (() => {{
{}
  }})();
  const musicGenerator = (() => {{
{}
{}
  }})();
  const music = {{ createPlayer: musicPlayer.createPlayer, generateSong: musicGenerator.generateSong, randomPreset: musicGenerator.randomPreset }};
  const soundExport = (() => {{
{}
  }})();
  window.PuzzleSoundTools = {{ ...sfx, ...music, ...soundExport }};
  window.PuzzleSoundGenerator = window.PuzzleSoundTools;
  window.dispatchEvent(new CustomEvent(\"PuzzleSoundToolsReady\"));
}})();",
        expose_module(
            SEEDED_SFX_JS,
            &[
                "SFX_TYPE_OPTIONS",
                "createSfxPlayer",
                "createPuzzleScriptSfxPlayer",
                "generateSoundEffect",
                "generatePuzzleScriptSoundEffect",
                "randomSfxPreset",
            ]
        ),
        expose_module(SEEDED_MUSIC_PLAYER_JS, &["createPlayer"]),
        module_body(SEEDED_TIMBRE_FIELDS_JS),
        expose_module(SEEDED_MUSIC_JS, &["generateSong", "randomPreset"]),
        expose_module(SOUND_EXPORT_JS, &["exportMusicLoop", "exportSoundEffect",]),
    )
}

fn source_json(state: &EditorState) -> Result<String, AppError> {
    source_json_for_payload(state, false)
}

fn source_json_with_content(state: &EditorState) -> Result<String, AppError> {
    source_json_for_payload(state, true)
}

fn source_json_for_payload(state: &EditorState, include_content: bool) -> Result<String, AppError> {
    let source = if state.puzzle_path.trim().is_empty() {
        state.source.clone()
    } else if include_content {
        fs::read_to_string(&state.puzzle_path)?
    } else {
        String::new()
    };
    let mut out = String::new();
    out.push('{');
    push_json_pair(&mut out, "puzzlePath", &state.puzzle_path);
    out.push(',');
    push_json_pair(&mut out, "workspaceRoot", &state.workspace_root);
    out.push(',');
    push_json_pair(&mut out, "source", &source);
    out.push(',');
    push_json_pair(&mut out, "gameCss", &state.game_css);
    out.push(',');
    push_editor_folders_json(&mut out, state);
    out.push(',');
    push_editor_documents_json(&mut out, state, include_content)?;
    out.push('}');
    Ok(out)
}

fn load_workspace_document_json(
    request: &LoadWorkspaceDocumentRequest,
    state: &EditorState,
) -> Result<String, AppError> {
    let workspace_root_path = PathBuf::from(&state.workspace_root);
    let requested_path =
        resolve_workspace_request_path(&request.puzzle_path, &workspace_root_path)?;
    let document = load_workspace_document(&requested_path, state)?;
    let mut out = String::new();
    push_editor_document_json(&mut out, state, &document, true)?;
    Ok(out)
}

#[cfg(feature = "embedded-assets")]
fn editor_seed_json(out: &mut String, state: &EditorState) {
    out.push('{');
    push_json_pair(out, "puzzlePath", &state.puzzle_path);
    out.push(',');
    push_json_pair(out, "workspaceRoot", &state.workspace_root);
    out.push(',');
    push_json_pair(out, "source", &state.source);
    out.push(',');
    push_json_pair(out, "previewHtml", "");
    out.push(',');
    push_json_pair(out, "gameCss", &state.game_css);
    out.push(',');
    push_editor_folders_json(out, state);
    out.push(',');
    out.push_str("\"activeDocumentIndex\":0");
    out.push(',');
    push_editor_documents_json(out, state, true)
        .expect("editor seed document serialization should not fail");
    out.push('}');
}

fn push_editor_folders_json(out: &mut String, state: &EditorState) {
    out.push_str("\"folders\":[");
    for (index, folder) in state.folders.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, folder);
    }
    out.push(']');
}

fn push_editor_documents_json(
    out: &mut String,
    state: &EditorState,
    include_content: bool,
) -> Result<(), AppError> {
    out.push_str("\"documents\":[");
    let mut first = true;
    for document in &state.documents {
        if !first {
            out.push(',');
        }
        first = false;
        push_editor_document_json(out, state, document, include_content)?;
    }
    out.push(']');
    Ok(())
}

fn push_editor_document_json(
    out: &mut String,
    state: &EditorState,
    document: &EditorDocument,
    include_content: bool,
) -> Result<(), AppError> {
    let loaded_document;
    let document = if include_content && !document.content_loaded {
        loaded_document = load_workspace_document(Path::new(&document.puzzle_path), state)?;
        &loaded_document
    } else {
        document
    };
    out.push('{');
    push_json_pair(out, "puzzlePath", &document.puzzle_path);
    out.push(',');
    push_json_pair(out, "workspaceRoot", &state.workspace_root);
    out.push(',');
    push_json_pair(out, "encoding", &document.encoding);
    out.push(',');
    push_json_pair(out, "mimeType", &document.mime_type);
    out.push(',');
    push_json_pair(
        out,
        "source",
        if include_content {
            &document.source
        } else {
            ""
        },
    );
    out.push(',');
    push_json_pair(
        out,
        "dataUrl",
        if include_content {
            &document.data_url
        } else {
            ""
        },
    );
    out.push(',');
    push_json_bool(
        out,
        "contentLoaded",
        include_content || document.content_loaded,
    );
    out.push(',');
    push_json_bool(out, "declaresGameEntry", document.declares_game_entry);
    out.push(',');
    push_json_pair(out, "previewHtml", &document.preview_html);
    out.push(',');
    push_json_pair(out, "previewError", &document.preview_error);
    out.push(',');
    push_json_pair(
        out,
        "gameCss",
        if include_content {
            &document.game_css
        } else {
            ""
        },
    );
    out.push(',');
    push_json_string_array(out, "importedBy", &document.imported_by);
    out.push(',');
    push_json_pair(out, "parentGamePath", &document.parent_game_path);
    out.push('}');
    Ok(())
}

fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_bool(out: &mut String, key: &str, value: bool) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn push_json_string_array(out: &mut String, key: &str, values: &[String]) {
    push_json_string(out, key);
    out.push_str(":[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, value);
    }
    out.push(']');
}

#[cfg(feature = "embedded-assets")]
fn push_diagnostics_json(out: &mut String, diagnostics: &[Diagnostic]) {
    out.push_str("{\"diagnostics\":[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_diagnostic_json(out, diagnostic);
    }
    out.push_str("]}");
}

#[cfg(feature = "embedded-assets")]
fn push_diagnostic_json(out: &mut String, diagnostic: &Diagnostic) {
    let span = diagnostic.primary_span.as_ref();
    out.push('{');
    push_json_pair(out, "severity", diagnostic.severity.as_str());
    out.push(',');
    push_json_pair(out, "code", diagnostic.code);
    out.push(',');
    push_json_pair(
        out,
        "file",
        span.and_then(|span| span.file.as_deref()).unwrap_or(""),
    );
    out.push(',');
    push_json_option_number(out, "line", span.and_then(|span| span.line));
    out.push(',');
    push_json_option_number(out, "column", span.and_then(|span| span.column));
    out.push(',');
    push_json_option_string(
        out,
        "sourceLine",
        span.and_then(|span| span.source_line.as_deref()),
    );
    out.push(',');
    push_json_pair(out, "message", &diagnostic.message);
    out.push('}');
}

#[cfg(feature = "embedded-assets")]
fn push_json_option_number(out: &mut String, key: &str, value: Option<usize>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

#[cfg(feature = "embedded-assets")]
fn push_json_option_string(out: &mut String, key: &str, value: Option<&str>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => push_json_string(out, value),
        None => out.push_str("null"),
    }
}

#[cfg(feature = "embedded-assets")]
fn escape_script_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(feature = "embedded-assets")]
fn escape_script(value: &str) -> String {
    value.replace("</script", "<\\/script")
}

#[cfg(feature = "embedded-assets")]
fn http_ok(content_type: &str, body: &str) -> Vec<u8> {
    http_response(200, "OK", content_type, body)
}

#[cfg(feature = "embedded-assets")]
fn http_error(status: u16, message: &str) -> Vec<u8> {
    let mut body = String::new();
    body.push_str("{\"error\":");
    push_json_string(&mut body, message);
    body.push('}');
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    http_response(status, reason, "application/json; charset=utf-8", &body)
}

#[cfg(feature = "embedded-assets")]
fn http_diagnostic_error(status: u16, report: &DiagnosticReport) -> Vec<u8> {
    let mut body = String::new();
    push_diagnostics_json(&mut body, report.diagnostics());
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    http_response(status, reason, "application/json; charset=utf-8", &body)
}

#[cfg(feature = "embedded-assets")]
fn http_response(status: u16, reason: &str, content_type: &str, body: &str) -> Vec<u8> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    response.into_bytes()
}

#[cfg(feature = "embedded-assets")]
fn http_bytes(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Config(String),
    Diagnostics(DiagnosticReport),
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
            Self::Diagnostics(report) => write!(f, "{report}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static WORKSPACE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestWorkspace {
        root: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let counter = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "puzzlebuilder-html-editor-test-{}-{timestamp}-{counter}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("create temp test workspace");
            Self { root }
        }

        fn write(&self, path: &str, contents: impl AsRef<[u8]>) -> PathBuf {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture parent directory");
            }
            fs::write(&path, contents).expect("write fixture file");
            path
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn files_header_owns_creation_and_open_actions() {
        let explorer_header = EDITOR_HTML
            .find(r#"class="explorer-header""#)
            .expect("explorer header");
        let files_header = EDITOR_HTML
            .find(r#"class="explorer-section-header explorer-files-header""#)
            .expect("files header");
        let explorer = &EDITOR_HTML[explorer_header..files_header];
        assert!(explorer.contains("<span>Explorer</span>"));
        assert!(explorer.contains(r#"data-pane-toggle="explorer""#));
        assert!(!explorer.contains(r#"id="newDocumentButton""#));
        assert!(!explorer.contains(r#"id="newFolderButton""#));
        assert!(!explorer.contains(r#"id="fileActionsButton""#));
        let document_list = EDITOR_HTML
            .find(r#"id="documentList""#)
            .expect("document list");
        let header = &EDITOR_HTML[files_header..document_list];
        assert!(header.contains(r#"id="newDocumentButton""#));
        assert!(header.contains(r#"id="newFolderButton""#));
        assert!(header.contains(r#"id="fileActionsButton""#));
        assert!(header.contains(">Open Files</button>"));
        assert!(header.contains(">Open Folder</button>"));
        assert!(!header.contains(">Import files</button>"));
        assert!(!header.contains(">Import folder</button>"));
    }

    #[test]
    fn workspace_file_actions_wait_for_the_workspace_tree() {
        assert!(
            EDITOR_HTML.contains(r#"id="importButton" type="button" role="menuitem" disabled"#)
        );
        assert!(
            EDITOR_HTML
                .contains(r#"id="importFolderButton" type="button" role="menuitem" disabled"#)
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "const disabled = !fileTree || (isDesktopHost() && !hasWritableWorkspace());"
        ));
        assert!(EDITOR_WORKSPACE_JS.contains("function setWorkspaceFileActionsReady()"));
        assert!(EDITOR_JS.contains("loadSource().then(() => {\n  setWorkspaceFileActionsReady();"));
    }

    #[test]
    fn workbench_does_not_reference_removed_sprite_mode_switch() {
        assert!(!EDITOR_WORKBENCH_JS.contains("spritePaneModeSwitch"));
        assert!(EDITOR_DOM_JS.contains(
            "const spriteDimensionButtons = document.querySelectorAll(\"[data-sprite-dimension]\");"
        ));
        assert!(EDITOR_DOM_JS.contains(
            "const spritePaneModeButtons = document.querySelectorAll(\"[data-sprite-pane-mode]\");"
        ));
    }

    fn js_object_string_map(source: &str, const_name: &str) -> HashMap<String, String> {
        let marker = format!("const {const_name} = Object.freeze({{");
        let start = source.find(&marker).expect("find JS object map") + marker.len();
        let end = source[start..]
            .find("\n});")
            .map(|offset| start + offset)
            .expect("find JS object map end");
        source[start..end]
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let line = line.strip_suffix(',').unwrap_or(line);
                let (key, value) = line.split_once(": ")?;
                Some((js_quoted_string(key), js_quoted_string(value)))
            })
            .collect()
    }

    fn js_const_string(source: &str, const_name: &str) -> String {
        let marker = format!("const {const_name} = \"");
        let start = source.find(&marker).expect("find JS string const") + marker.len();
        let end = source[start..]
            .find('"')
            .map(|offset| start + offset)
            .expect("find JS string const end");
        source[start..end].to_string()
    }

    fn source_outline_icon_names() -> HashSet<String> {
        let marker = "  const icons = {\n";
        let start = EDITOR_SOURCE_JS
            .find(marker)
            .expect("find outline icon registry")
            + marker.len();
        let end = EDITOR_SOURCE_JS[start..]
            .find("\n  };\n  const paths")
            .map(|offset| start + offset)
            .expect("find outline icon registry end");
        EDITOR_SOURCE_JS[start..end]
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                let key = trimmed.strip_suffix(": `")?;
                Some(js_property_name(key))
            })
            .collect()
    }

    fn js_quoted_string(value: &str) -> String {
        value
            .trim()
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .expect("quoted JS string")
            .to_string()
    }

    fn js_property_name(value: &str) -> String {
        let value = value.trim();
        if value.starts_with('"') {
            js_quoted_string(value)
        } else {
            value.to_string()
        }
    }

    fn collect_outline_kinds_from_source(
        source: &str,
        profile: puzzle_lang::PuzzleSourceProfile,
        kinds: &mut BTreeSet<String>,
    ) {
        for item in puzzle_lang::source_outline(source, profile) {
            kinds.insert(item.kind);
        }
    }

    fn collect_puzzle_fence_outline_kinds(
        markdown: &str,
        profile: puzzle_lang::PuzzleSourceProfile,
        kinds: &mut BTreeSet<String>,
    ) {
        let mut in_puzzle_fence = false;
        let mut block = String::new();
        for line in markdown.lines() {
            let trimmed = line.trim_start();
            if in_puzzle_fence {
                if trimmed.starts_with("```") {
                    collect_outline_kinds_from_source(&block, profile, kinds);
                    block.clear();
                    in_puzzle_fence = false;
                } else {
                    block.push_str(line);
                    block.push('\n');
                }
            } else if trimmed.starts_with("```puzzle") {
                in_puzzle_fence = true;
            }
        }
    }

    fn outline_kind_requires_explicit_icon(kind: &str) -> bool {
        if kind.starts_with("on_") || kind.contains(':') {
            return false;
        }
        if kind.chars().all(|ch| matches!(ch, '.' | '#')) {
            return false;
        }
        !kind
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_uppercase())
    }

    fn editor_fixture_source(title: &str) -> String {
        format!(
            r#"title = "{title}"

puzzle default {{
slots {{
actor = Player
}}

rules {{
}}
}}

levels demo of default {{
legend {{
. = empty
P = Player
}}

level "start" {{
P
}}
}}

scene playing {{
layout {{
puzzle board = default
}}
rules {{
step board
}}
}}
"#
        )
    }

    fn paths_contain(paths: &[EditorDocument], suffix: &str) -> bool {
        paths
            .iter()
            .any(|document| document.puzzle_path.replace('\\', "/").ends_with(suffix))
    }

    fn document_with_suffix<'a>(paths: &'a [EditorDocument], suffix: &str) -> &'a EditorDocument {
        paths
            .iter()
            .find(|document| document.puzzle_path.replace('\\', "/").ends_with(suffix))
            .expect("document with suffix")
    }

    #[test]
    fn pages_export_default_does_not_resolve_games_workspace() {
        let config = Config::from_args(Vec::<String>::new()).expect("default config");
        assert!(
            config.puzzle_path.is_none(),
            "default Pages export must use the managed example seed, not games/"
        );

        let service = EditorService::open_pages_example();
        let state = service.state();
        assert_eq!(state.workspace_root, "");
        assert_eq!(state.puzzle_path, PAGES_EXAMPLE_PUZZLE_PATH);
        assert_eq!(state.folders, ["starter"]);
        assert_eq!(state.documents.len(), PAGES_STARTER_DOCUMENTS.len());
        assert_eq!(state.documents[0].puzzle_path, PAGES_EXAMPLE_PUZZLE_PATH);
        assert_eq!(state.documents[1].puzzle_path, "starter/README.md");
        assert!(state.source.contains("01 — Basic"));
        for (path, _, declares_game_entry) in PAGES_STARTER_DOCUMENTS {
            let document = state
                .documents
                .iter()
                .find(|document| document.puzzle_path == *path)
                .expect("starter manifest document should be embedded");
            assert_eq!(document.declares_game_entry, *declares_game_entry);
        }

        let html = service
            .export_pages_editor_html()
            .expect("export managed Pages editor html");
        assert!(html.contains("01 — Basic"));
        assert!(html.contains("07-meta-level.puzzle"));
        assert!(html.contains("all Goal on Box"));
        assert!(html.contains("input [ Player | Box | no Wall ]"));
        assert!(!html.contains("Player{&gt;}"));
        assert!(!html.contains("Player{>}"));
        assert!(html.contains(PAGES_EXAMPLE_PUZZLE_PATH));
        assert!(html.contains("starter/README.md"));
        assert!(!html.contains("games/spec_2d.puzzle"));
        assert!(!html.contains("/games/"));
        assert!(!html.contains("/private/"));
        assert!(!html.contains("Managed GitHub Pages starter"));
        assert!(!html.contains("levels demo of starter"));
    }

    #[test]
    fn cli_config_preserves_requested_project_folder() {
        let config = Config::from_args(vec![
            "games/microban".to_string(),
            "--serve".to_string(),
            "--port".to_string(),
            "8906".to_string(),
        ])
        .expect("parse editor config");

        assert_eq!(config.puzzle_path, Some(PathBuf::from("games/microban")));
        assert!(config.serve);
        assert_eq!(config.port, 8906);
    }

    #[test]
    fn pages_example_compiles_as_playable_sokoban() {
        let workspace = TestWorkspace::new();
        let example_path = workspace.write(PAGES_EXAMPLE_PUZZLE_PATH, PAGES_EXAMPLE_PUZZLE_SOURCE);
        let service = EditorService::open(&example_path).expect("open managed Pages example");
        let html = service
            .compile_preview(&PreviewRequest::new(
                PAGES_EXAMPLE_PUZZLE_SOURCE,
                example_path.display().to_string(),
                String::new(),
            ))
            .expect("managed Pages example should compile");

        assert!(html.contains("01 — Basic"));
        assert!(html.contains("Rules, collision, winning, sprites, and levels"));
    }

    #[test]
    fn every_pages_starter_game_entry_compiles() {
        let workspace = TestWorkspace::new();
        for (path, source, declares_game_entry) in PAGES_STARTER_DOCUMENTS {
            if !declares_game_entry {
                continue;
            }
            let example_path = workspace.write(path, source);
            let service = EditorService::open(&example_path)
                .unwrap_or_else(|error| panic!("{path} should open: {error}"));
            service
                .compile_preview(&PreviewRequest::new(
                    *source,
                    example_path.display().to_string(),
                    String::new(),
                ))
                .unwrap_or_else(|error| panic!("{path} should compile: {error}"));
        }
    }

    #[test]
    fn open_loads_workspace_documents_with_active_puzzle_first() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Editor Fixture").replace(
                "\npuzzle default {",
                "\nassets {\n\"tile.svg\"\n}\n\npuzzle default {",
            ),
        );
        workspace.write("games/editor_fixture/notes.md", "# Notes\n");
        workspace.write(
            "games/editor_fixture/visuals.js",
            "window.GameVisuals = {};\n",
        );
        workspace.write(
            "games/editor_fixture/game.css",
            ".board { background-image: url(\"tile.svg\"); }\n",
        );
        workspace.write(
            "games/editor_fixture/tile.svg",
            r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
        );
        workspace.write(
            "games/editor_fixture/fragments/extra.puzzle",
            "const imported_label = \"Imported\"\n",
        );

        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");
        let state = service.state();

        let canonical_game_path = game_path.canonicalize().expect("canonical game path");
        assert_eq!(PathBuf::from(&state.puzzle_path), canonical_game_path);
        assert_eq!(
            PathBuf::from(&state.documents[0].puzzle_path),
            canonical_game_path,
            "the active game must stay first so editor tabs and saves target the loaded file"
        );
        assert!(paths_contain(
            &state.documents,
            "games/editor_fixture/notes.md"
        ));
        assert!(paths_contain(
            &state.documents,
            "games/editor_fixture/fragments/extra.puzzle"
        ));
        assert!(
            state.game_css.contains("data:image/svg+xml"),
            "editor preview CSS should inline local url() assets"
        );
        assert!(
            state.base_game_visuals_js.contains("tile.svg"),
            "preview asset resolver should expose declared file assets"
        );
        assert!(
            !state.base_game_visuals_js.contains("notes.md"),
            "preview asset resolver must not expose undeclared workspace files"
        );
        let source_json = service.source_json().expect("source json");
        assert!(
            !source_json.contains("gameVisualsJs"),
            "visual scripts are service-owned assets, not document JSON state"
        );
    }

    #[test]
    fn open_defers_preview_import_expansion_failure_until_compile() {
        let workspace = TestWorkspace::new();
        let source = format!(
            "import \"missing.puzzle\"\n\n{}",
            editor_fixture_source("Broken Import")
        );
        let game_path = workspace.write("games/broken_import/game.puzzle", source);
        let project_dir = game_path.parent().expect("project dir");

        let service =
            EditorService::open_game_entry(project_dir).expect("open workspace with broken import");
        let error = service
            .compile_preview(&PreviewRequest::new(
                fs::read_to_string(&game_path).expect("read broken import source"),
                game_path.display().to_string(),
                String::new(),
            ))
            .expect_err("broken preview import must fail while compiling preview");
        let message = error.to_string();

        assert!(
            !message.trim().is_empty(),
            "compile error should be reported by preview compile, not editor open"
        );
    }

    #[test]
    fn open_defers_editor_visuals_generation_failure_until_compile() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write("games/broken_visuals/game.puzzle", "title = \"Broken\"\n");

        let service =
            EditorService::open(&game_path).expect("open editor with invalid preview source");
        let document = document_with_suffix(
            &service.state().documents,
            "games/broken_visuals/game.puzzle",
        );

        assert_eq!(document.preview_html, "");
        assert_eq!(document.preview_error, "");
        assert!(
            service
                .compile_preview(&PreviewRequest::new(
                    "title = \"Broken\"\n",
                    game_path.display().to_string(),
                    String::new(),
                ))
                .is_err()
        );
    }

    #[test]
    fn open_loads_puzzle3_workspace_documents() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/puzzle3_editor_fixture/game.puzzle3",
            include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3"),
        );
        workspace.write("games/puzzle3_editor_fixture/notes.md", "# Notes\n");

        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open puzzle3 fixture");
        let state = service.state();

        let canonical_game_path = game_path.canonicalize().expect("canonical game path");
        assert_eq!(PathBuf::from(&state.puzzle_path), canonical_game_path);
        assert_eq!(
            PathBuf::from(&state.documents[0].puzzle_path),
            canonical_game_path
        );
        let document = document_with_suffix(
            &state.documents,
            "games/puzzle3_editor_fixture/game.puzzle3",
        );
        assert_eq!(document.mime_type, "text/plain");
        assert!(!document.content_loaded);
        assert!(document.source.is_empty());
        assert!(document.declares_game_entry);
        assert!(paths_contain(
            &state.documents,
            "games/puzzle3_editor_fixture/notes.md"
        ));
    }

    #[test]
    fn open_skips_generated_and_dependency_directories() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Editor Fixture"),
        );
        workspace.write(
            "games/editor_fixture/target/generated.puzzle",
            "title \"Generated\"\n",
        );
        workspace.write(
            "games/editor_fixture/node_modules/library/readme.md",
            "# Dependency\n",
        );
        workspace.write(
            "games/editor_fixture/dist/generated.js",
            "window.Generated = true;\n",
        );

        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");
        let state = service.state();

        assert!(paths_contain(
            &state.documents,
            "games/editor_fixture/game.puzzle"
        ));
        assert!(
            !paths_contain(
                &state.documents,
                "games/editor_fixture/target/generated.puzzle"
            ),
            "generated target files must not become editable workspace documents"
        );
        assert!(
            !paths_contain(
                &state.documents,
                "games/editor_fixture/node_modules/library/readme.md"
            ),
            "dependency files must not become editable workspace documents"
        );
        assert!(
            !state.base_game_visuals_js.contains("generated.js"),
            "preview asset resolver must not embed generated dependency/build files"
        );
    }

    #[test]
    fn source_json_defers_file_source_until_document_load() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Original Title"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        fs::write(&game_path, editor_fixture_source("Changed Title"))
            .expect("update source after service open");
        let source_json = service.source_json().expect("source json");
        let source = json_string_field(&source_json, "source").expect("source field");
        let loaded_json = service
            .load_workspace_document_json(&LoadWorkspaceDocumentRequest::new(
                game_path.display().to_string(),
            ))
            .expect("load document");
        let loaded_source = json_string_field(&loaded_json, "source").expect("loaded source field");

        assert!(source.is_empty());
        assert!(loaded_source.contains("title = \"Changed Title\""));
        assert!(!loaded_source.contains("title = \"Original Title\""));
    }

    #[test]
    fn load_workspace_document_reports_read_failure_instead_of_cached_source() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Original Title"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        fs::remove_file(&game_path).expect("remove source after service open");

        assert!(
            service
                .load_workspace_document_json(&LoadWorkspaceDocumentRequest::new(
                    game_path.display().to_string(),
                ))
                .is_err(),
            "document load must not fall back to the cached source when the file cannot be read"
        );
    }

    #[test]
    fn workspace_preview_entries_use_prelude_not_game_puzzle_name() {
        let workspace = TestWorkspace::new();
        let entry_path = workspace.write(
            "games/custom_entry/arcade.puzzle",
            editor_fixture_source("ArcadeEntry"),
        );
        let fragment_path =
            workspace.write("games/custom_entry/fragments/levels.puzzle", "levels {}\n");
        let project_dir = entry_path.parent().expect("project dir");

        let service = EditorService::open_game_entry(project_dir).expect("open custom project");
        let state = service.state();

        assert_eq!(
            PathBuf::from(&state.puzzle_path),
            entry_path.canonicalize().expect("canonical entry path")
        );
        let entry_doc = document_with_suffix(&state.documents, "games/custom_entry/arcade.puzzle");
        assert!(
            entry_doc.preview_html.is_empty(),
            "workspace documents should not embed generated preview HTML before the browser needs it"
        );
        let fragment_doc = document_with_suffix(
            &state.documents,
            "games/custom_entry/fragments/levels.puzzle",
        );
        assert_eq!(
            fragment_doc.preview_html, "",
            "fragment documents should also defer preview generation"
        );
        assert_eq!(
            fragment_path.file_name().and_then(|value| value.to_str()),
            Some("levels.puzzle")
        );
    }

    #[test]
    fn workspace_import_graph_tracks_multiple_parent_game_candidates() {
        let workspace = TestWorkspace::new();
        let fragment_path =
            workspace.write("games/multiple_parents/shared/levels.puzzle", "levels {}\n");
        let game_path = workspace.write(
            "games/multiple_parents/game.puzzle",
            format!(
                "import \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Game Parent")
            ),
        );
        workspace.write(
            "games/multiple_parents/main.puzzle",
            format!(
                "import \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Main Parent")
            ),
        );
        workspace.write(
            "games/multiple_parents/third_parent.puzzle",
            format!(
                "import \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Third Parent")
            ),
        );
        let project_dir = game_path.parent().expect("project dir");

        let service =
            EditorService::open_game_entry(project_dir).expect("open multi-parent project");
        let state = service.state();
        let fragment_doc = document_with_suffix(
            &state.documents,
            "games/multiple_parents/shared/levels.puzzle",
        );

        assert_eq!(
            fragment_doc.imported_by.len(),
            3,
            "all direct parent candidates should be visible to the editor"
        );
        assert_eq!(
            PathBuf::from(&fragment_doc.imported_by[0]),
            game_path.canonicalize().expect("canonical game path"),
            "multiple parent candidates should be sorted so the normal game entry wins"
        );
        assert_eq!(
            PathBuf::from(&fragment_doc.parent_game_path),
            game_path.canonicalize().expect("canonical parent game")
        );
        assert_eq!(
            PathBuf::from(&fragment_path)
                .canonicalize()
                .expect("canonical fragment"),
            PathBuf::from(&fragment_doc.puzzle_path)
        );
    }

    #[test]
    fn editor_workspace_preview_selects_first_parent_game_candidate() {
        assert!(EDITOR_WORKSPACE_JS.contains("function parentGameCandidatesForDocument(document)"));
        assert!(EDITOR_WORKSPACE_JS.contains(".sort(comparePuzzleEntryDocuments);"));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("return parentGameCandidatesForDocument(document)[0] || null;")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("Preview uses: ${parentGames[0].puzzlePath"));
    }

    #[test]
    fn editor_workspace_uses_puzzle_declaration_as_game_entry() {
        assert!(EDITOR_WORKSPACE_JS.contains("/^(puzzle|puzzle3)(?:\\s|$)/"));
    }

    #[test]
    fn workspace_preview_generation_is_deferred_until_run() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/broken/game.puzzle",
            "title = \"Broken\"\n\npuzzle main {\n",
        );

        let service = EditorService::open(&game_path).expect("open broken editor fixture");
        let document = document_with_suffix(&service.state().documents, "games/broken/game.puzzle");

        assert_eq!(
            document.preview_html, "",
            "workspace loading should not generate preview HTML"
        );
        assert_eq!(
            document.preview_error, "",
            "compile errors should be reported by the compile path, not while seeding the editor"
        );
        let source_json = service.source_json().expect("source json");
        assert!(source_json.contains("\"previewHtml\":\"\""));
        assert!(source_json.contains("\"previewError\":\"\""));
    }

    #[test]
    fn open_game_entry_accepts_empty_project_folders() {
        let workspace = TestWorkspace::new();
        let project_dir = workspace.root.join("games/empty_project");
        fs::create_dir_all(&project_dir).expect("create empty project folder");
        fs::create_dir_all(project_dir.join("levels/empty")).expect("create nested empty folder");

        let service = EditorService::open_game_entry(&project_dir).expect("open empty project");
        let state = service.state();

        assert_eq!(
            PathBuf::from(&state.workspace_root),
            project_dir.canonicalize().expect("canonical project dir")
        );
        assert_eq!(state.puzzle_path, "");
        assert_eq!(state.source, "");
        assert!(state.documents.is_empty());
        assert!(state.folders.contains(&"levels".to_string()));
        assert!(state.folders.contains(&"levels/empty".to_string()));
        let source_json = service.source_json().expect("source json");
        assert!(source_json.contains("\"folders\":["));
        assert!(source_json.contains("\"levels/empty\""));
    }

    #[test]
    fn open_game_entry_accepts_project_folders_without_puzzle_model() {
        let workspace = TestWorkspace::new();
        let fragment_path = workspace.write("games/fragments/levels.puzzle", "levels {}\n");
        workspace.write("games/fragments/notes.md", "# Notes\n");
        let project_dir = fragment_path.parent().expect("project dir");

        let service =
            EditorService::open_game_entry(project_dir).expect("open non-entry project folder");
        let state = service.state();

        assert_eq!(state.puzzle_path, "");
        assert!(paths_contain(
            &state.documents,
            "games/fragments/levels.puzzle"
        ));
        assert!(paths_contain(&state.documents, "games/fragments/notes.md"));
        let fragment_doc = document_with_suffix(&state.documents, "games/fragments/levels.puzzle");
        assert_eq!(fragment_doc.preview_html, "");
    }

    #[test]
    fn compile_preview_uses_the_request_source_and_editor_assets() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Preview Before"),
        );
        workspace.write(
            "games/editor_fixture/game.css",
            "body { color: #123456; }\n",
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                editor_fixture_source("Preview After"),
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("Preview After"));
        assert!(
            html.contains("#123456"),
            "request CSS should flow into the generated preview"
        );
        assert!(
            html.contains("window.PuzzleRuntimeExportJson = "),
            "editor preview HTML must expose the runtime export metadata consumed by the level editor"
        );
        assert!(
            html.contains(r#"\"engine\""#),
            "editor preview metadata must include engine data for level editing"
        );
        assert!(!html.contains("PuzzleEditorSolverRulesJson"));
        assert!(!html.contains(r#"\"solver_strategy\""#));
        assert!(
            EDITOR_JS.contains("extractAssignedStringLiteral(source, \"PuzzleRuntimeExportJson\")"),
            "editor metadata extraction must read the preview runtime contract"
        );
        assert!(!EDITOR_JS.contains("PuzzleEditorSolverRulesJson"));
        assert!(!html.contains("Preview Before"));
    }

    #[test]
    fn compile_preview_accepts_top_level_sounds_without_duplicate_error() {
        let workspace = TestWorkspace::new();
        let source = editor_fixture_source("Top Level Sounds").replace(
            "puzzle default {",
            "sounds {\nsfx push { seed = 301193; type = hit; volume = 0.3 }\n}\n\npuzzle default {",
        );
        let game_path = workspace.write("games/top_level_sounds/game.puzzle", &source);
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile preview with top-level sounds");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains(r#"\"name\":\"push\""#));
    }

    #[test]
    fn desktop_compile_preview_exports_current_theme() {
        let workspace = TestWorkspace::new();
        let themed_source = editor_fixture_source("Themed Preview").replace(
            "puzzle default {",
            "theme {\npreset = \"puzzlescript\"\nbackground_color = #ffffff\ntext_color = #000000\n}\n\npuzzle default {",
        );
        let game_path = workspace.write("games/themed_preview/game.puzzle", &themed_source);
        let service = EditorService::open(&game_path).expect("open themed editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                themed_source,
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile themed preview");

        assert!(html.contains(r#"<body class="theme-puzzlescript""#));
        assert!(html.contains("--background:#ffffff;"));
        assert!(html.contains("--text:#000000;"));
    }

    #[test]
    fn compile_preview_accepts_display_object_single_color_sprite() {
        let workspace = TestWorkspace::new();
        let source = r##"
title = display_object_single_color_preview

puzzle default {
slots {
@display_floor = @Floor
}
sprites {
@Floor
#eeeeee
}
rules {

}
levels {
legend {
. = empty
}
level "start"
.
}
}
"##;
        let game_path = workspace.write("games/floor_color/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("#eeeeee"));
    }

    #[test]
    fn compile_preview_accepts_line_style_tagged_sprite_after_pattern() {
        let workspace = TestWorkspace::new();
        let source = r##"
title = line_style_tagged_preview

puzzle default {
tags {
state = base movable
}
slots {
actor = Box:state
}
sprites {
Box:base
#aaa
0
Box:movable
#bbb
0
}
rules {

}
levels {
legend {
. = empty
}
legend B = Box:base
level "start"
B
}
}
"##;
        let game_path = workspace.write("games/tagged_sprite/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile tagged sprite preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("line_style_tagged_preview"));
    }

    #[test]
    fn compile_preview_preserves_language_diagnostics() {
        let workspace = TestWorkspace::new();
        let source = r#"
title = "Multi Error Probe"

puzzle main {
slots {
base = Floor
}

sprites {
}

rules {
unknown_statement_one
unknown_statement_two
}

levels {
legend {
. = empty
}
level "first"
.
}
}
"#;
        let game_path = workspace.write("games/multi_error/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor");
        let error = service
            .compile_preview(&PreviewRequest::new(
                source.to_string(),
                game_path.display().to_string(),
                String::new(),
            ))
            .expect_err("invalid source should fail preview compile");

        let AppError::Diagnostics(report) = error else {
            panic!("preview compile should preserve language diagnostics");
        };
        let diagnostics = report.diagnostics();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].message,
            "unknown routine call: unknown_statement_one"
        );
        assert_eq!(
            diagnostics[0]
                .primary_span
                .as_ref()
                .and_then(|span| span.line),
            Some(13)
        );
        assert_eq!(
            diagnostics[1].message,
            "unknown routine call: unknown_statement_two"
        );
        assert_eq!(
            diagnostics[1]
                .primary_span
                .as_ref()
                .and_then(|span| span.line),
            Some(14)
        );
    }

    #[test]
    fn compile_preview_reports_independent_lifecycle_diagnostics_together() {
        let workspace = TestWorkspace::new();
        let source = r#"
title = "Multi Lifecycle Error Probe"

puzzle main {
slots {
actor = Player
}
slots {
base = Player actor
}

on_level_start {
input directions [ Player | ] -> [ | Player ]
}

on_level_clear {
input directions [ Player | ] -> [ | Player ]
}

rules {
}

levels {
legend {
. = empty
P = Player
}
level "first"
P.
}
}
"#;
        let game_path = workspace.write("games/multi_lifecycle_error/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor");
        let error = service
            .compile_preview(&PreviewRequest::new(
                source.to_string(),
                game_path.display().to_string(),
                String::new(),
            ))
            .expect_err("invalid source should fail preview compile");

        let AppError::Diagnostics(report) = error else {
            panic!("preview compile should preserve language diagnostics");
        };
        let messages = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages.contains(&"on_level_start cannot depend on input"),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"on_level_clear cannot depend on input"),
            "{messages:?}"
        );
        let level_start = report
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.message == "on_level_start cannot depend on input")
            .expect("on_level_start diagnostic");
        assert_eq!(
            level_start
                .primary_span
                .as_ref()
                .and_then(|span| span.source_line.as_deref()),
            Some("input directions [ Player | ] -> [ | Player ]")
        );
        let level_clear = report
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.message == "on_level_clear cannot depend on input")
            .expect("on_level_clear diagnostic");
        assert_eq!(
            level_clear
                .primary_span
                .as_ref()
                .and_then(|span| span.source_line.as_deref()),
            Some("input directions [ Player | ] -> [ | Player ]")
        );
    }

    #[test]
    fn compile_preview_reports_independent_statement_parse_errors_together() {
        let workspace = TestWorkspace::new();
        let source = r#"
title = "Multi Statement Parse Error Probe"

puzzle main {
slots {
base = Player
}

rules {
action push
do win
banana split
}

levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#;
        let game_path = workspace.write("games/multi_statement_error/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor");
        let error = service
            .compile_preview(&PreviewRequest::new(
                source.to_string(),
                game_path.display().to_string(),
                String::new(),
            ))
            .expect_err("invalid source should fail preview compile");

        let AppError::Diagnostics(report) = error else {
            panic!("preview compile should preserve language diagnostics");
        };
        let messages = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages.contains(
                &"`action` statements were removed; use explicit input guards and rewrites"
            ),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"`do` is obsolete; write the effect statement directly"),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"unknown statement directive banana"),
            "{messages:?}"
        );
    }

    #[test]
    fn compile_preview_reports_sibling_statement_block_errors_together() {
        let workspace = TestWorkspace::new();
        let source = r#"
title = "Sibling Statement Block Error Probe"

puzzle main {
slots {
base = Player
}

rules {
once {
action push
}

repeat {
do win
}

banana split
}

levels {
legend {
. = empty
P = Player
}
level "first"
P
}
}
"#;
        let game_path = workspace.write("games/sibling_statement_block_error/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor");
        let error = service
            .compile_preview(&PreviewRequest::new(
                source.to_string(),
                game_path.display().to_string(),
                String::new(),
            ))
            .expect_err("invalid source should fail preview compile");

        let AppError::Diagnostics(report) = error else {
            panic!("preview compile should preserve language diagnostics");
        };
        let messages = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages.contains(
                &"`action` statements were removed; use explicit input guards and rewrites"
            ),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"`do` is obsolete; write the effect statement directly"),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"unknown statement directive banana"),
            "{messages:?}"
        );
    }

    #[test]
    fn compile_preview_supports_puzzle3_documents() {
        let workspace = TestWorkspace::new();
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3");
        let game_path = workspace.write("games/puzzle3_fixture/game.puzzle3", source);
        let service = EditorService::open(&game_path).expect("open puzzle3 fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                String::new(),
            ))
            .expect("compile puzzle3 preview");

        assert!(html.contains("window.Puzzle3DFixture"));
        assert!(html.contains("WasmPuzzle3Runtime"));
        assert!(html.contains("window.Puzzle3ThreeModuleSource = "));
        assert!(html.contains("window.Puzzle3ThreeRenderer"));
        assert!(html.contains("return text === \"canvas\" ? \"canvas\" : \"three\";"));
        assert!(html.contains("Microban 3D"));
    }

    #[test]
    fn compile_preview_accepts_3d_input_rule_without_orientation_set() {
        let workspace = TestWorkspace::new();
        let source = r#"
title = "Bare 3D Input"

puzzle push3 {
  slots {
    actor = Player
  }

  rules {
    input [ Player ] -> [ > Player ]
  }
}

levels demo of push3 {
  legend {
    . = empty
    P = Player
  }

  level "start" {
    P.
  }
}
"#;
        let game_path = workspace.write("games/puzzle3_input_rule/game.puzzle3", source);
        let service = EditorService::open(&game_path).expect("open puzzle3 input fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                String::new(),
            ))
            .expect("compile puzzle3 input preview");

        assert!(html.contains("window.Puzzle3DFixture"));
        assert!(html.contains("Bare 3D Input"));
    }

    #[test]
    fn editor_uses_dom_sprite_rendering_only_for_level_editing() {
        assert!(
            EDITOR_DOM_JS.contains("new window.PuzzleRenderer(levelBoard, { renderMode: \"dom\"")
        );
        assert!(
            EDITOR_DOM_JS
                .contains("new window.PuzzleRenderer(solverBoard, { renderMode: \"canvas\"")
        );
    }

    #[test]
    fn source_play_button_opens_play_preview() {
        assert!(EDITOR_HTML.contains(r#"id="runButton""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Play preview""#));
        assert!(EDITOR_HTML.contains("source-preview-play-icon"));
        assert!(EDITOR_HTML.contains("source-preview-refresh-icon"));
        assert!(!EDITOR_HTML.contains("source-preview-stop-icon"));
        assert!(EDITOR_JS.contains("async function runPreviewFromSourcePane()"));
        assert!(EDITOR_JS.contains(
            "async function runPreviewFromSourcePane() {\n  ensurePreviewTargetsActiveDocument();"
        ));
        assert!(EDITOR_JS.contains("setStatus(\"Saving before preview\", \"\");"));
        assert!(EDITOR_JS.contains("saved = await saveCurrentDocument(true);"));
        assert!(EDITOR_JS.contains("if (!saved) {\n    setStatus(\"Save failed\", \"is-error\");"));
        let save = EDITOR_JS
            .find("saved = await saveCurrentDocument(true);")
            .expect("run preview saves before compiling");
        let compile_after_save = EDITOR_JS[save..]
            .find("await renderPreview();")
            .expect("run preview compiles after save");
        assert!(compile_after_save > 0);
        assert!(
            EDITOR_WORKSPACE_JS.contains("async function saveCurrentDocument(showStatus = true)")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("return true;\n  } catch (error) {"));
        assert!(EDITOR_WORKSPACE_JS.contains("throw error;\n  } finally {"));
        assert!(EDITOR_JS.contains("openPreviewModePane(\"play\", { focus: false });"));
        assert!(EDITOR_JS.contains("function previewRuntimeIsRunning()"));
        assert!(EDITOR_JS.contains("function syncSourcePreviewRunButton()"));
        assert!(EDITOR_JS.contains("activePreviewRequest = controller;\n  runButton.disabled = false;\n  syncSourcePreviewRunButton();"));
        assert!(EDITOR_JS.contains(
            "runButton.addEventListener(\"click\", () => {\n  runPreviewFromSourcePane();\n});"
        ));
        assert!(EDITOR_WORKSPACE_JS.contains("runButton.title = \"Play preview\";"));
    }

    #[test]
    fn save_loads_workspace_document_before_file_write() {
        assert!(EDITOR_WORKSPACE_JS.contains("setEditorStatus(\"Loading before save\", \"\");"));
        let load_before_save = EDITOR_WORKSPACE_JS
            .find("await ensureDocumentContentLoaded(document);")
            .expect("save should load deferred workspace document content before saving");
        let host_save = EDITOR_WORKSPACE_JS
            .find("await window.PuzzleStudioHost.save({")
            .expect("save should still call the host save boundary");
        assert!(load_before_save < host_save);
        assert!(EDITOR_WORKSPACE_JS.contains("contentLoaded: document.contentLoaded !== false,"));
        assert!(EDITOR_BOOT_JS.contains("contentLoaded: payload?.contentLoaded === true,"));
    }

    #[test]
    fn editor_preview_iframe_allows_audio_playback() {
        let preview_frame = EDITOR_HTML
            .find(r#"id="previewFrame""#)
            .expect("editor preview iframe");
        let preview_frame_tail = &EDITOR_HTML[preview_frame..];
        assert!(
            preview_frame_tail.contains(r#"sandbox="allow-scripts""#),
            "preview iframe should remain script-sandboxed"
        );
        assert!(
            preview_frame_tail.contains(r#"allow="autoplay""#),
            "preview iframe must delegate autoplay for sfx and music playback in Tauri/WebView"
        );
        assert!(EDITOR_JS.contains(r#"nextFrame.setAttribute("allow", "autoplay");"#));
    }

    #[test]
    fn editor_preview_save_shortcut_is_not_game_input() {
        assert!(EDITOR_JS.contains("const isEditorSaveShortcut = (event) => {"));
        assert!(EDITOR_JS.contains("document.addEventListener(\"keydown\", (event) => {"));
        assert!(EDITOR_JS.contains("event.stopImmediatePropagation();"));
        assert!(EDITOR_JS.contains(
            "window.parent.postMessage({ type: \"PuzzleStudioEditorSaveShortcut\" }, \"*\");"
        ));
        assert!(EDITOR_JS.contains("event.data?.type === \"PuzzleStudioEditorSaveShortcut\""));
        assert!(EDITOR_JS.contains("saveCurrentDocument(true).catch((error) => {"));
    }

    #[test]
    fn tool_pane_save_shortcut_updates_source_before_file_save() {
        let tool_pane_hook = EDITOR_WORKSPACE_JS
            .find("handleToolPaneSaveShortcut(event)")
            .expect("save shortcut delegates to tool panes");
        let normal_save = EDITOR_WORKSPACE_JS[tool_pane_hook..]
            .find("saveCurrentDocument(true)")
            .expect("document save remains the non-tool-pane path");
        assert!(normal_save > 0);
        assert!(EDITOR_WORKSPACE_JS.contains(
            "if (typeof handleToolPaneSaveShortcut === \"function\" && handleToolPaneSaveShortcut(event)) {\n    event.preventDefault();\n    event.stopImmediatePropagation();\n    return true;\n  }"
        ));
        assert!(EDITOR_JS.contains("function handleToolPaneSaveShortcut(event)"));
        assert!(EDITOR_JS.contains("function currentToolPaneSaveShortcutMode(event)"));
        assert!(EDITOR_JS.contains("rememberToolPaneSaveShortcutContext(event.target);"));
        assert!(EDITOR_JS.contains("updateLevelInSource();"));
        assert!(EDITOR_JS.contains("updateLevel3dInSource();"));
        assert!(EDITOR_JS.contains("updateSpriteInSource();"));
        assert!(EDITOR_JS.contains("updateSprite3dInSource();"));
        assert!(
            EDITOR_JS.contains(
                "updateSoundsDefinition(sounds.mode === \"music\" ? \"music\" : \"sfx\");"
            )
        );
        assert!(EDITOR_JS.contains("Level source update unavailable"));
        assert!(EDITOR_JS.contains("3D level source update unavailable"));
        assert!(EDITOR_JS.contains("Sprite source update unavailable"));
        assert!(EDITOR_JS.contains("3D sprite source update unavailable"));
        assert!(EDITOR_JS.contains("Sound source update unavailable"));
    }

    #[test]
    fn editor_preview_dirty_status_stays_on_preview_pane() {
        assert!(EDITOR_JS.contains("let compiledPreviewStale = false;"));
        assert!(EDITOR_JS.contains("compiledPreviewStale = Boolean(latestHtml || previewExport);"));
        assert!(!EDITOR_JS.contains("invalidateCompiledPreview(activePreviewDocument());"));
        assert!(EDITOR_JS.contains(r#"setPaneStatus("preview", "Preview requires compile", "");"#));
        assert!(!EDITOR_JS.contains(r#"setStatus("Preview requires compile", "");"#));
    }

    #[test]
    fn editor_discards_last_preview_when_compile_diagnostics_fail() {
        assert!(
            EDITOR_JS
                .contains("function invalidateCompiledPreview(document = activePreviewDocument())")
        );
        assert!(EDITOR_JS.contains("appendCompileDiagnostics(error, { source: \"compiler\", document, sourceText: requestSource });"));
        assert!(EDITOR_JS.contains("invalidateCompiledPreview(document);"));
        assert!(
            EDITOR_JS
                .contains(r#"applyGameVisuals(document ? effectiveGameVisualsJs(document) : "");"#)
        );
        assert!(EDITOR_JS.contains(r#"setStatus("Compile error", "is-error");"#));
        assert!(!EDITOR_JS.contains("function preserveCompiledPreviewAfterCompileError(document)"));
        assert!(!EDITOR_JS.contains("Keeping last successful preview"));
        assert!(!EDITOR_JS.contains("Preview kept with compile errors"));
        assert!(
            !EDITOR_JS
                .contains(r#"setPaneStatus("preview", "Preview has compile errors", "is-error");"#)
        );
    }

    #[test]
    fn desktop_workspace_delete_requires_confirmation() {
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("function confirmDeleteWorkspaceEntry(node, options = {})")
        );
        let confirm = EDITOR_WORKSPACE_JS
            .find("confirmDeleteWorkspaceEntry(target.node, {")
            .expect("desktop delete confirms the selected workspace entry");
        let host_delete = EDITOR_WORKSPACE_JS
            .find("window.PuzzleStudioHost.deleteWorkspaceEntry({")
            .expect("desktop delete calls the host filesystem boundary");
        assert!(confirm < host_delete);
    }

    #[test]
    fn desktop_workspace_delete_does_not_persist_old_source_into_next_active_file() {
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("function saveDocumentStore(showStatus = true, options = {})")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("if (options.persistCurrent !== false)"));
        let delete_start = EDITOR_WORKSPACE_JS
            .find("async function deleteTreeNode(nodeId)")
            .expect("delete tree node handler");
        let delete_end = EDITOR_WORKSPACE_JS[delete_start..]
            .find("async function removeWorkspaceNode(nodeId)")
            .expect("delete handler end")
            + delete_start;
        let delete_handler = &EDITOR_WORKSPACE_JS[delete_start..delete_end];
        let active_switch = delete_handler
            .find("activeFileId = documents[0]?.id || \"\";")
            .expect("delete selects a replacement active document");
        let load_next = delete_handler
            .find("loadEmbeddedDocument(currentDocumentIndex);")
            .expect("delete loads the replacement document before persisting the tree");
        let persist_tree = delete_handler
            .find("saveDocumentStore(false, { persistCurrent: false });")
            .expect("delete persists the tree without reading the stale source editor");
        assert!(active_switch < load_next);
        assert!(load_next < persist_tree);
        assert!(!delete_handler.contains(
            "currentDocumentIndex = activeDocumentIndex();\n  saveDocumentStore(false);"
        ));
    }

    #[test]
    fn editor_active_document_tracks_active_file_id_for_preview() {
        assert!(EDITOR_WORKSPACE_JS.contains(
            "function activeDocument() {\n  currentDocumentIndex = activeDocumentIndex();\n  return documents[currentDocumentIndex] || null;\n}"
        ));
        assert!(
            EDITOR_WORKSPACE_JS.contains(
                "function persistCurrentDocument() {\n  const document = activeDocument();"
            )
        );
    }

    #[test]
    fn desktop_workspace_mutations_defer_external_reload() {
        assert!(EDITOR_WORKSPACE_JS.contains("let workspaceHostMutationDepth = 0;"));
        assert!(EDITOR_WORKSPACE_JS.contains("let deferredWorkspaceChangedPayload = null;"));
        assert!(EDITOR_WORKSPACE_JS.contains("function externalReloadErrorMessage(error)"));
        assert!(EDITOR_WORKSPACE_JS.contains("External reload failed: ${message}"));
        assert!(EDITOR_WORKSPACE_JS.contains("function beginWorkspaceHostMutation()"));
        assert!(EDITOR_WORKSPACE_JS.contains("function endWorkspaceHostMutation()"));
        assert!(EDITOR_WORKSPACE_JS.contains("if (workspaceHostMutationDepth > 0)"));
        assert!(EDITOR_WORKSPACE_JS.contains("deferredWorkspaceChangedPayload = payload;"));
        assert!(EDITOR_WORKSPACE_JS.contains("queueMicrotask(() => {"));
        assert!(EDITOR_WORKSPACE_JS.contains("const previousActive = activeDocument();"));
        assert!(EDITOR_WORKSPACE_JS.contains(
            "const previousActiveSource = previousActive && isTextDocument(previousActive)"
        ));
        assert!(EDITOR_WORKSPACE_JS.contains("const preserveActiveView = previousActive"));
        assert!(EDITOR_WORKSPACE_JS.contains("renderDocumentTabs();"));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("} else {\n      loadEmbeddedDocument(currentDocumentIndex);\n    }")
        );
        assert!(
            !EDITOR_WORKSPACE_JS
                .contains("const previewInputsUnchanged = externalSource === localSource")
        );
        assert!(!EDITOR_WORKSPACE_JS.contains("normalized.previewHtml = previous.previewHtml"));
        assert!(!EDITOR_WORKSPACE_JS.contains("normalized.previewError = previous.previewError"));
        let save_guard = EDITOR_WORKSPACE_JS
            .find("if (isDesktopHost()) {\n      beginWorkspaceHostMutation();\n    }\n    try {\n      await window.PuzzleStudioHost.save({")
            .expect("desktop save is guarded while host IO runs");
        let save_release = EDITOR_WORKSPACE_JS[save_guard..]
            .find("endWorkspaceHostMutation();")
            .expect("desktop save releases the host mutation guard");
        assert!(save_release > 0);
        let create_file_guard = EDITOR_WORKSPACE_JS
            .find("beginWorkspaceHostMutation();\n    try {\n      await window.PuzzleStudioHost.createSourceFile({")
            .expect("desktop file creation is guarded while host IO runs");
        let create_file_release = EDITOR_WORKSPACE_JS[create_file_guard..]
            .find("endWorkspaceHostMutation();")
            .expect("desktop file creation releases the host mutation guard");
        assert!(create_file_release > 0);
        let delete_guard = EDITOR_WORKSPACE_JS
            .find("beginWorkspaceHostMutation();\n    try {\n      await window.PuzzleStudioHost.deleteWorkspaceEntry({")
            .expect("desktop delete is guarded while host IO runs");
        let delete_release = EDITOR_WORKSPACE_JS[delete_guard..]
            .find("endWorkspaceHostMutation();")
            .expect("desktop delete releases the host mutation guard");
        assert!(delete_release > 0);
        let move_start = EDITOR_WORKSPACE_JS
            .find("async function moveNodeToFolder(nodeId, targetFolderId)")
            .expect("drag/drop move handler");
        let move_end = EDITOR_WORKSPACE_JS[move_start..]
            .find("function dropFolderIdForEvent(event)")
            .expect("drag/drop move handler end")
            + move_start;
        let move_handler = &EDITOR_WORKSPACE_JS[move_start..move_end];
        let move_guard = move_handler
            .find("beginWorkspaceHostMutation();\n    try {\n      await window.PuzzleStudioHost.renameWorkspaceEntry({")
            .expect("desktop drag/drop move is guarded while host IO runs");
        let move_release = move_handler[move_guard..]
            .find("endWorkspaceHostMutation();")
            .expect("desktop drag/drop move releases the host mutation guard");
        assert!(move_release > 0);
    }

    #[test]
    fn source_switch_stops_live_preview_even_when_preview_target_is_unchanged() {
        let load_document = EDITOR_WORKSPACE_JS
            .split("function loadEmbeddedDocument(index) {")
            .nth(1)
            .and_then(|tail| tail.split("\nfunction loadFolderPreview").next())
            .expect("loadEmbeddedDocument source");
        assert!(load_document.contains("const previousPreviewDocument = activePreviewDocument();"));
        assert!(load_document.contains(
            "const previewTargetUnchanged = previewDocument\n    && previousPreviewKey\n    && documentIdentityKey(previewDocument) === previousPreviewKey;"
        ));
        assert!(load_document.contains(
            "if (activeSourceChanged) {\n    invalidateCompiledPreview(previewDocument);\n  } else if (previewTargetUnchanged) {\n    markPreviewDirty();"
        ));
    }

    #[test]
    fn closing_preview_pane_stops_preview_runtime() {
        assert!(EDITOR_JS.contains("function stopPreviewRuntime()"));
        let close_pane = EDITOR_WORKBENCH_JS
            .split("function closeWorkPane(paneId) {")
            .nth(1)
            .and_then(|tail| tail.split("\nfunction showWorkPane").next())
            .expect("closeWorkPane source");
        assert!(close_pane.contains(
            "if (normalized === PREVIEW_WORK_PANE_ID) {\n    stopPreviewRuntime();\n  }"
        ));
    }

    #[test]
    fn empty_folder_preview_selection_falls_back_to_active_document() {
        let active_preview_document = EDITOR_WORKSPACE_JS
            .split("function activePreviewDocument() {")
            .nth(1)
            .and_then(|tail| tail.split("\nfunction previewDocumentForFolder").next())
            .expect("activePreviewDocument source");
        assert!(
            active_preview_document
                .contains("const folderPreview = previewDocumentForFolder(selected);")
        );
        assert!(
            active_preview_document
                .contains("if (folderPreview) {\n      return folderPreview;\n    }")
        );
        assert!(active_preview_document.contains("return previewDocumentFor(activeDocument());"));
    }

    #[test]
    fn workspace_drag_drop_moves_entries_through_host_boundary() {
        assert!(
            EDITOR_WORKSPACE_JS.contains("async function moveNodeToFolder(nodeId, targetFolderId)")
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "throw new Error(\"Desktop workspace move requires the host rename command.\");"
        ));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("fromPath: hostPathForEditorPath(sourcePath, sourceWorkspaceRoot),")
        );
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("toPath: hostPathForEditorPath(targetPath, sourceWorkspaceRoot),")
                || EDITOR_WORKSPACE_JS
                    .contains("toPath: hostPathForEditorPath(targetPath, targetWorkspaceRoot),")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("workspaceRoot: sourceWorkspaceRoot,"));
        assert!(EDITOR_WORKSPACE_JS.contains("targetWorkspaceRoot,"));
        assert!(EDITOR_WORKSPACE_JS.contains("function dropFolderIdForPoint(x, y)"));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("function moveTargetFolderForSource(sourceNode, targetFolderId)")
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "const sourceWorkspaceFolder = sourceWorkspaceRoot ? workspaceRootFolder(sourceWorkspaceRoot) : null;"
        ));
        assert!(EDITOR_WORKSPACE_JS.contains(
            "if (sourceWorkspaceFolder) {\n    return sourceWorkspaceFolder;\n  }\n  return fileTree;"
        ));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("selectedFolderId = targetFolder === fileTree ? \"\" : targetFolder.id;")
        );
        assert!(!EDITOR_WORKSPACE_JS.contains("row.draggable = true;"));
        let tree_row_css = EDITOR_CSS
            .split(".tree-row {")
            .nth(1)
            .and_then(|tail| tail.split("\n}").next())
            .expect("tree row CSS");
        assert!(tree_row_css.contains("user-select: none;"));
        assert!(EDITOR_CSS.contains(".tree-row[data-drag-id] {\n  cursor: grab;"));
        assert!(EDITOR_CSS.contains(".tree-row.is-dragging {\n  cursor: grabbing;"));
        assert!(EDITOR_CSS.contains(".tree-drag-preview {"));
        assert!(EDITOR_CSS.contains("pointer-events: none;"));
        assert!(EDITOR_JS.contains("function finishTreeMove(nodeId, targetFolderId)"));
        assert!(EDITOR_JS.contains("function createTreeDragPreview(drag)"));
        assert!(EDITOR_JS.contains("function updateTreeDragFeedback(drag, clientX, clientY)"));
        assert!(EDITOR_JS.contains("function clearTreeDragFeedback(drag)"));
        assert!(EDITOR_JS.contains("documentList.addEventListener(\"pointerdown\", (event) => {"));
        assert!(
            EDITOR_JS
                .contains("event.preventDefault();\n  row.setPointerCapture?.(event.pointerId);")
        );
        assert!(EDITOR_JS.contains(
            "const targetFolderId = dropFolderIdForPoint(event.clientX, event.clientY);"
        ));
        assert!(
            EDITOR_JS.contains(
                "markDropTarget(resolvedDropFolderIdForNode(drag.nodeId, targetFolderId));"
            )
        );
        assert!(EDITOR_JS.contains(
            "folder.expanded = folder.expanded === false;\n      loadFolderPreview(folder);"
        ));
        assert!(!EDITOR_JS.contains("if (event.target.closest(\".tree-chevron, .tree-icon\"))"));
        assert!(!EDITOR_JS.contains("documentList.addEventListener(\"dragstart\""));
        assert!(EDITOR_JS.contains("if (!hasExternalFiles) {\n    return;\n  }"));
        assert!(EDITOR_JS.contains("event.dataTransfer.dropEffect = \"copy\";"));
        assert!(EDITOR_JS.contains("markDropTarget(targetFolderId);"));
        assert!(!EDITOR_JS.contains("finishTreeMove(draggedNodeId"));
        assert!(EDITOR_JS.contains(
            "setEditorStatus(workspaceMutationErrorMessage(\"Move failed\", error), \"is-error\");"
        ));
    }

    #[test]
    fn editor_does_not_expose_scene_preview_pane() {
        assert!(!EDITOR_HTML.contains(r#"id="sceneModeButton""#));
        assert!(!EDITOR_HTML.contains(r#"id="scenePanel""#));
        assert!(!EDITOR_HTML.contains("Scene preview"));
        assert!(!EDITOR_CSS.contains(".scene-preview"));
        assert!(!EDITOR_JS.contains("PuzzleStudioSetScenePreview"));
        assert!(!EDITOR_WORKBENCH_JS.contains("scene: \"scene\""));
    }

    #[test]
    fn puzzlescript_import_converts_from_source_input() {
        assert!(!EDITOR_HTML.contains(r#"id="psImportConvertButton""#));
        assert!(!EDITOR_DOM_JS.contains("psImportConvertButton"));
        assert!(!EDITOR_JS.contains("Generate .puzzle translation"));
        assert!(EDITOR_HTML.contains("lucide-file-plus-icon lucide-file-plus"));
        assert!(EDITOR_HTML.contains("ps-import-output-actions"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function resetPuzzleScriptImportConversion()"));
        assert!(
            EDITOR_IMPORT_EXPORT_JS.contains("function schedulePuzzleScriptImportConversion()")
        );
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function puzzleScriptSourceTitle(source)"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function puzzleStudioMetadataTitle(canonical)"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains(r#"/^title\s*=\s*(.+)$/"#));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains(r#".replace(/^title\s*/, "")"#));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("convertPuzzleScriptImport(generation)"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("window.PuzzleStudioImportExport = {"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("schedulePuzzleScriptImportConversion,"));
        assert!(
            EDITOR_JS.contains("typeof api?.schedulePuzzleScriptImportConversion === \"function\"")
        );
        assert!(EDITOR_JS.contains("api.schedulePuzzleScriptImportConversion();"));
        assert!(!EDITOR_JS.contains("psImportConvertButton?.addEventListener"));
        assert!(!EDITOR_JS.contains("await convertPuzzleScriptImport()"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("No converted .puzzle yet"));
        assert!(
            EDITOR_CSS.contains(".ps-import-actions .source-action-button:hover:not(:disabled)")
        );
        assert!(EDITOR_CSS.contains("#psImportStatus,\n#levelSolveStatus"));
    }

    #[test]
    fn source_action_buttons_do_not_hold_mouse_focus_as_toggle_state() {
        assert!(EDITOR_CSS.contains(".source-action-button:focus-visible"));
        assert!(EDITOR_CSS.contains("outline: 2px solid color-mix"));
        assert!(!EDITOR_CSS.contains(".source-action-button:focus {\n"));
        assert!(
            !EDITOR_CSS
                .contains(".source-action-button:hover,\n.source-action-button:focus-visible")
        );
        assert!(EDITOR_CSS.contains(".source-action-button[aria-pressed=\"true\"]"));
    }

    #[test]
    fn file_import_commits_workspace_without_preview_compile() {
        let load_imported = EDITOR_IMPORT_EXPORT_JS
            .find("loadEmbeddedDocument(currentDocumentIndex);")
            .expect("file import loads the imported document");
        let save_imported = EDITOR_IMPORT_EXPORT_JS[load_imported..]
            .find("saveDocumentStore(false);")
            .expect("file import persists the imported workspace");
        let status_imported = EDITOR_IMPORT_EXPORT_JS[load_imported..]
            .find("setEditorStatus(`Imported to ${folderName}`, \"is-ok\");")
            .expect("file import reports import success");

        assert!(save_imported < status_imported);
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains("await renderPreview();"));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains("Imported; preview failed: ${message}"));
        assert!(EDITOR_JS.contains("Import failed: ${importErrorMessage(error)}"));
    }

    #[test]
    fn desktop_export_reports_clickable_exported_file() {
        assert!(EDITOR_BOOT_JS.contains(r#"invoke("open_exported_file", { request: payload })"#));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("setExportedFileStatus(result.path, filename);"));
        assert!(
            EDITOR_IMPORT_EXPORT_JS.contains("function setExportedFileStatus(path, fallbackName)")
        );
        assert!(
            EDITOR_IMPORT_EXPORT_JS
                .contains("await window.PuzzleStudioHost.openExportedFile({ path });")
        );
        assert!(
            EDITOR_IMPORT_EXPORT_JS.contains(
                "setEditorStatus(`Open failed: ${error.message || error}`, \"is-error\");"
            )
        );
        assert!(
            EDITOR_JS.contains("function setEditorStatusLink(prefixText, linkText, options = {})")
        );
        assert!(
            EDITOR_JS
                .contains("function setPaneStatusLink(paneId, prefixText, linkText, options = {})")
        );
        assert!(
            EDITOR_IMPORT_EXPORT_JS.contains(
                "setPaneStatusLink(activeStatusPaneId(), \"Exported \", label, options);"
            )
        );
        assert!(EDITOR_CSS.contains(".pane-footer .document-status a"));
    }

    #[test]
    fn level_editor_grid_is_owned_by_editor_toggle() {
        assert!(EDITOR_HTML.contains(r#"id="levelGridButton""#));
        assert!(
            EDITOR_DOM_JS
                .contains("const levelGridButton = document.querySelector(\"#levelGridButton\");")
        );
        assert!(EDITOR_JS.contains("let levelGridVisible = false;"));
        assert!(EDITOR_HTML.contains(r#"id="levelLayerControls""#));
        assert!(EDITOR_HTML.contains(r#"id="levelLayerPreviewPanel""#));
        assert!(EDITOR_HTML.contains(r#"id="levelLayerPreviewStrip""#));
        assert!(EDITOR_DOM_JS.contains(
            "const levelLayerControls = document.querySelector(\"#levelLayerControls\");"
        ));
        assert!(EDITOR_DOM_JS.contains(
            "const levelLayerPreviewPanel = document.querySelector(\"#levelLayerPreviewPanel\");"
        ));
        assert!(EDITOR_DOM_JS.contains(
            "const levelLayerPreviewStrip = document.querySelector(\"#levelLayerPreviewStrip\");"
        ));
        assert!(!EDITOR_DOM_JS.contains("levelLayerVisibilityButton"));
        assert!(!EDITOR_DOM_JS.contains("levelLayerVisibilityMenu"));
        assert!(EDITOR_JS.contains("layerMode: false,"));
        assert!(EDITOR_JS.contains("showCompositeLayers: false,"));
        assert!(!EDITOR_HTML.contains(r#"id="levelScopeLayerButton""#));
        assert!(!EDITOR_HTML.contains(r#"aria-label="Level edit scope""#));
        assert!(EDITOR_JS.contains("function renderLevelLayerControls()"));
        assert!(EDITOR_JS.contains("function renderLevelLayerPreviews()"));
        assert!(EDITOR_JS.contains("function levelLayerCells("));
        assert!(EDITOR_JS.contains("function levelCompositeCells(options = {})"));
        assert!(EDITOR_JS.contains("levelCompositeCells({ includeHidden: true"));
        assert!(EDITOR_JS.contains(
            "function paintCellSlots(slots, objectId, exportData = currentLevelExportData())"
        ));
        assert!(EDITOR_JS.contains("function syncLevelGridVisibility()"));
        assert!(EDITOR_JS.contains(
            "levelBoard?.classList.remove(\"has-occupied-cell-grid\", \"has-all-cell-grid\");"
        ));
        assert!(
            EDITOR_JS
                .contains("levelBoard?.classList.toggle(\"has-all-cell-grid\", levelGridVisible);")
        );
        assert!(EDITOR_JS.contains("levelRenderer.render(levelScene(cells, exportData));"));
        assert!(!EDITOR_JS.contains("renderEditorLevelBoardDom(cells, exportData);"));
        assert!(
            EDITOR_JS
                .contains("syncLevelGridVisibility();\n  levelBoard.querySelectorAll(\".cell\")")
        );
        assert!(
            EDITOR_JS.contains("levelGridButton?.addEventListener(\"click\", toggleLevelGrid);")
        );
        assert!(EDITOR_JS.contains("function compiledPreviewGameVisualsJs(html)"));
        assert!(
            !EDITOR_JS.contains("function previewGameVisualsJsForCompiledHtml(html, document)")
        );
        assert!(
            EDITOR_JS
                .contains("throw new Error(\"Compiled preview is missing GameVisuals script.\");")
        );
        assert!(!EDITOR_JS.contains("function renderLevelEditorFromSourceOnly(document, source)"));
        assert!(!EDITOR_JS.contains("function levelEditorSourceExportData(source, entry)"));
        assert!(!EDITOR_JS.contains("function sourceGameVisualsJs(source)"));
        assert!(EDITOR_JS.contains("applyGameVisuals(compiledPreviewGameVisualsJs(html));"));
        assert!(EDITOR_JS.contains("label.textContent = `Layer ${index + 1}`;"));
        assert!(EDITOR_JS.contains("levelLayerPreviewStrip.replaceChildren(fragment);"));
        assert!(EDITOR_CSS.contains(".level-board.board.has-all-cell-grid .cell::after"));
        assert!(EDITOR_CSS.contains("z-index: 100;"));
    }

    #[test]
    fn level_editor_board_uses_continuous_sprite_checkerboard_background() {
        assert!(EDITOR_CSS.contains("--sprite-swatch-checker: url("));
        assert!(EDITOR_CSS.contains(
            ".level-board.board {\n  background-color: var(--sprite-swatch-bg);\n  background-image: var(--sprite-swatch-checker);\n  background-size: 8px 8px;\n  box-shadow:"
        ));
        assert!(
            !EDITOR_CSS.contains(
                ".level-board.board .cell {\n  background-color: var(--sprite-swatch-bg);"
            )
        );
    }

    #[test]
    fn level_editor_controls_are_ordered_before_palette_and_preview() {
        assert!(!EDITOR_HTML.contains("<span>Levels</span>"));
        assert!(EDITOR_HTML.contains(r#"id="levelNamespaceInput" type="hidden""#));
        assert!(EDITOR_HTML.contains(r#"id="level3dBundleInput" type="hidden""#));
        assert!(EDITOR_CSS.contains("grid-template-columns: minmax(120px, 1fr);"));

        let level_name = EDITOR_HTML.find(r#"id="levelNameInput""#).unwrap();
        let play = EDITOR_HTML.find(r#"id="levelPlaytestButton""#).unwrap();
        let expand = EDITOR_HTML.find(r#"id="levelExpandButton""#).unwrap();
        let palette = EDITOR_HTML.find(r#"id="levelPalette""#).unwrap();
        let board = EDITOR_HTML.find(r#"id="levelBoardViewport""#).unwrap();

        assert!(level_name < play);
        assert!(play < expand);
        assert!(expand < palette);
        assert!(palette < board);
    }

    #[test]
    fn level_editor_palette_has_no_hide_button() {
        assert!(!EDITOR_HTML.contains("levelPaletteCollapseButton"));
        assert!(!EDITOR_DOM_JS.contains("levelPaletteCollapseButton"));
        assert!(!EDITOR_JS.contains("levelPaletteCollapseButton"));
        assert!(!EDITOR_JS.contains("paletteCollapsed"));
    }

    #[test]
    fn level_editor_playtest_is_explicit_and_does_not_overwrite_authored_cells() {
        assert!(EDITOR_HTML.contains(r#"id="levelPlaytestButton""#));
        assert!(EDITOR_HTML.contains(r#"id="levelBoard" class="level-board board" tabindex="0""#));
        assert!(EDITOR_JS.contains("let levelPlaytestActive = false;"));
        assert!(
            EDITOR_JS.contains("async function ensurePreviewExportForLevelAction(options = {})")
        );
        assert!(EDITOR_JS.contains("function startLevelPlaytest()"));
        assert!(EDITOR_JS.contains("compilingMessage: \"Compiling preview for play\""));
        assert!(EDITOR_JS.contains("function stopLevelPlaytest(options = {})"));
        assert!(!EDITOR_JS.contains("PuzzleStudioStopPreviewSession"));
        assert!(EDITOR_JS.contains(
            "if (options.syncPreview !== false) {\n    restoreCompiledGamePreview();\n  }"
        ));
        assert!(EDITOR_JS.contains("function focusLevelInputTarget()"));
        assert!(EDITOR_JS.contains("const stateData = levelStateData(exportData);"));
        assert!(
            EDITOR_JS
                .contains("function stateDataToLevelCells(stateData, exportData = previewExport)")
        );
        assert!(!EDITOR_JS.contains("function transitionPlaytestProgram("));
        assert!(!EDITOR_JS.contains("function levelPlaytestCoreRuntime("));
        assert!(!EDITOR_JS.contains("function applyLevelPlaytestKey(event)"));
        assert!(!EDITOR_JS.contains("WasmCompiledCoreRuntime"));
        assert!(!EDITOR_JS.contains("transition_current_outcome"));
        assert!(EDITOR_JS.contains("function levelPlaytestCommandForKey(event)"));
        assert!(EDITOR_JS.contains(r#"return "undo";"#));
        assert!(EDITOR_JS.contains(r#"return "redo";"#));
        assert!(EDITOR_JS.contains(r#"return "restart";"#));
        assert!(EDITOR_JS.contains("function levelPlaytestInputForKey(event"));
        assert!(EDITOR_JS.contains("const inputs = latestPreviewState?.inputs?.length"));
        assert!(EDITOR_JS.contains(": exportData?.inputs || [];"));
        assert!(EDITOR_JS.contains(
            "const command = levelPlaytestCommandForKey(event) || levelPlaytestInputForKey(event);"
        ));
        assert!(EDITOR_JS.contains(r#"postMessage({ type: "PuzzleStudioCommand", command }"#));
        assert!(EDITOR_JS.contains(r#"type: "PuzzleStudioKey","#));
        assert!(EDITOR_JS.contains("code: event.code"));
        assert!(EDITOR_JS.contains("acceptModelInput: true"));
        assert!(
            EDITOR_JS.contains("levelDisplayCells = stateDataToLevelCells(stateData, exportData);")
        );
        assert!(EDITOR_JS.contains("function displayedLevelCells()"));
        assert!(EDITOR_JS.contains(
            "if (levelPlaytestActive) {\n    if (levelDisplayCells?.length === level.cells.length)"
        ));
        assert!(EDITOR_JS.contains(
            "return level.showCompositeLayers ? levelCompositeCells() : levelLayerCells();"
        ));
        assert!(EDITOR_JS.contains(
            "if (!levelBuilder.hidden && levelPlaytestActive && pendingPreviewKeyStateSync > 0)"
        ));
        assert!(EDITOR_JS.contains("materializeLevelStart: true"));
        assert!(EDITOR_JS.contains("materializeDisplay: true"));
        assert!(EDITOR_JS.contains("levelIndex,"));
    }

    #[test]
    fn level3d_editor_playtest_uses_runtime_preview_contract() {
        assert!(EDITOR_HTML.contains(r#"id="level3dPlaytestButton""#));
        assert!(EDITOR_DOM_JS.contains("const level3dPlaytestButton = document.querySelector"));
        assert!(EDITOR_LEVEL3D_JS.contains("let level3dPlaytestActive = false;"));
        assert!(EDITOR_LEVEL3D_JS.contains("function startLevel3dPlaytest()"));
        assert!(EDITOR_LEVEL3D_JS.contains("await ensurePreviewExportForLevelAction({"));
        assert!(EDITOR_LEVEL3D_JS.contains("compilingMessage: \"Compiling preview for play\""));
        assert!(EDITOR_LEVEL3D_JS.contains("function stopLevel3dPlaytest(options = {})"));
        assert!(!EDITOR_LEVEL3D_JS.contains("PuzzleStudioStopPreviewSession"));
        assert!(EDITOR_LEVEL3D_JS.contains("function sendLevel3dPlaytestKey(event)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains(
                "target.postMessage({ type: \"PuzzleStudioCommand\", command: \"undo\" }"
            )
        );
        assert!(EDITOR_LEVEL3D_JS.contains("type: \"PuzzleStudioRequestPuzzle3State\""));
        assert!(EDITOR_LEVEL3D_JS.contains("function handleLevel3dPlaytestStateMessage(event)"));
        assert!(EDITOR_LEVEL3D_JS.contains("event.data?.type !== \"PuzzleStudioPuzzle3State\""));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3dPlaytestButton?.addEventListener(\"click\", toggleLevel3dPlaytest);"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("level3dCameraYawScrub"));
        assert!(EDITOR_LEVEL3D_JS.contains("if (level3dPlaytestActive) {\n    return;\n  }\n  const target = level3dPreviewScrubTarget(event);"));
        assert!(EDITOR_CSS.contains(".level-builder.is-playtesting .level3d-stage-canvas"));
        assert!(EDITOR_CSS.contains(".level-builder.is-playtesting .level3d-preview-controls"));
    }

    #[test]
    fn level3d_editor_horizontal_input_moves_slice() {
        assert!(EDITOR_LEVEL3D_JS.contains("function handleLevel3dSliceHorizontalInput(event)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("moveLevel3dLayer(event.key === \"ArrowLeft\" ? -1 : 1);")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("setLevel3dLayer(level3d.slice + delta);"));
        assert!(EDITOR_LEVEL3D_JS.contains("level3dLayerBoard?.addEventListener(\"keydown\", (event) => {\n  if (handleLevel3dSliceHorizontalInput(event))"));
        assert!(EDITOR_LEVEL3D_JS.contains("document.addEventListener(\"keydown\", (event) => {\n  handleLevel3dSliceHorizontalInput(event);"));
        assert!(EDITOR_LEVEL3D_JS.contains("level3dPlaytestActive\n    || (event.key !== \"ArrowLeft\" && event.key !== \"ArrowRight\")"));
    }

    #[test]
    fn level3d_editor_can_insert_slices_from_layer_toolbar() {
        assert!(EDITOR_HTML.contains(r#"id="level3dAddSliceAboveButton""#));
        assert!(EDITOR_HTML.contains(r#"id="level3dAddSliceBelowButton""#));
        assert!(
            EDITOR_DOM_JS.contains("const level3dAddSliceAboveButton = document.querySelector")
        );
        assert!(
            EDITOR_DOM_JS.contains("const level3dAddSliceBelowButton = document.querySelector")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("function insertLevel3dSlice(relative)"));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("const insertIndex = relative === \"below\" ? current + 1 : current;")
        );
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3d.slices.splice(insertIndex, 0, emptyLevel3dSlice(level3dEmptyChar()));"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("level3d.slice = insertIndex;"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3dAddSliceAboveButton?.addEventListener(\"click\", () => insertLevel3dSlice(\"above\"));"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3dAddSliceBelowButton?.addEventListener(\"click\", () => insertLevel3dSlice(\"below\"));"
        ));
        assert!(EDITOR_CSS.contains(".level3d-layer-toolbar .level3d-slice-add-button"));
    }

    #[test]
    fn level3d_layer_empty_cells_show_checkerboard_background() {
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("cell.classList.toggle(\"is-empty\", !entry || !entry.objects?.length);")
        );
        assert!(EDITOR_CSS.contains(".level3d-layer-board {\n  --level3d-layer-width: 1;"));
        assert!(EDITOR_CSS.contains("background-color: var(--sprite-swatch-bg);"));
        assert!(EDITOR_CSS.contains("background-image: var(--sprite-swatch-checker);"));
        assert!(
            EDITOR_CSS.contains(
                ".level3d-layer-board.is-grid-board .level3d-layer-cell {\n  min-width: 0;"
            )
        );
        assert!(EDITOR_CSS.contains("background: transparent;"));
        assert!(
            EDITOR_CSS
                .contains(".level3d-layer-board.is-grid-board .level3d-layer-cell:not(.is-empty)")
        );
    }

    #[test]
    fn level3d_slice_overlay_has_readable_contrast() {
        assert!(EDITOR_CSS.contains(".level3d-layer-toolbar.sprite3d-slice-axis-control {"));
        assert!(EDITOR_CSS.contains("background: rgb(34 38 44 / 88%);"));
        assert!(EDITOR_CSS.contains("box-shadow: 0 2px 10px rgb(0 0 0 / 35%);"));
        assert!(EDITOR_CSS.contains(".level3d-layer-toolbar .sprite3d-layer-axis-label,"));
        assert!(EDITOR_CSS.contains("color: #f6f8fb;"));
        assert!(EDITOR_CSS.contains("color: rgb(246 248 251 / 72%);"));
    }

    #[test]
    fn level_editor_loads_cells_from_source_target() {
        assert!(
            EDITOR_JS
                .contains("function levelReferenceSource(exportData = currentLevelExportData())")
        );
        assert!(EDITOR_JS.contains(
            "level.palette = levelPaletteFromExport(levelReferenceSource(exportData), exportData);"
        ));
        assert!(EDITOR_JS.contains(
            "function previewLevelIndexForSourceEntry(entry, exportData = previewExport)"
        ));
        assert!(EDITOR_JS.contains("sourceTitleMatches(requestedName, level.name)"));
        assert!(EDITOR_JS.contains("openPreviewModePane(\"edit\");"));
        assert!(
            EDITOR_JS.contains("function loadLevelFromSourceEntry(source, entry, options = {})")
        );
        assert!(EDITOR_JS.contains("const referenceSource = levelReferenceSource(exportData);"));
        assert!(EDITOR_JS.contains(
            "sourceLevelStateFromEntry(source, entry, exportData, { ...options, referenceSource })"
        ));
        assert!(
            EDITOR_JS
                .contains("level.palette = levelPaletteFromExport(referenceSource, exportData);")
        );
        assert!(EDITOR_JS.contains("...sourceCharEntries(referenceSource, exportData),"));
        assert!(!EDITOR_JS.contains("level.palette = levelPaletteFromExport(source, exportData);"));
        assert!(!EDITOR_JS.contains("previewDirty"));
        assert!(EDITOR_JS.contains("previewExport = null;"));
        assert!(!EDITOR_JS.contains("function levelEditorCurrentExportData()"));
        let level_source_loader = EDITOR_JS
            .find("function loadLevelSourceEntry(source, entry, options = {})")
            .expect("level source loader");
        let level_source_loader_end = EDITOR_JS[level_source_loader..]
            .find("function levelEditorSourceExportData(source)")
            .map(|index| level_source_loader + index)
            .expect("level source loader end");
        let level_source_loader = &EDITOR_JS[level_source_loader..level_source_loader_end];
        assert!(level_source_loader.contains("levelEditorSourceExportData(source)"));
        assert!(!level_source_loader.contains("PuzzleStudioHost.preview"));
        assert!(!level_source_loader.contains("renderPreview"));
        assert!(
            EDITOR_JS.contains(
                "if (levelPlaytestActive && !compiledPreviewStale && exportData === currentPreviewExportData()) {"
            )
        );
        assert!(EDITOR_JS.contains(
            "function activePreviewModeAcceptsLevelState() {\n  return currentPreviewMode === \"edit\" && levelPlaytestActive;\n}"
        ));
        assert!(EDITOR_JS.contains("function levelEditorSourceExportData(source)"));
        assert!(EDITOR_RUNTIME_JS.contains("levelEditorSourceSession(source)"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_manifest_json"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_level_slots"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_sprite_json"));
        assert!(!EDITOR_JS.contains("loadLevelSourceEntryAfterPreviewCompile"));
        assert!(EDITOR_JS.contains("applyLevelEditorContractVisuals(session, objects)"));
        assert!(EDITOR_JS.contains("session.levelSlots(levelIndex, authoredLayer)"));
        assert!(!EDITOR_JS.contains("function levelEditorRuntimeSprite("));
        assert!(EDITOR_JS.contains("if (exportData.editorSourceContract) {"));
        assert!(EDITOR_JS.contains("stateDataToEditorCells(integrated.initialState, exportData)"));
        assert!(!EDITOR_JS.contains("async function compileSolverPreviewData()"));
        assert!(EDITOR_JS.contains("function prepareEditorSolverArtifact("));
        assert!(EDITOR_JS.contains("module.compile_workspace_solver_rules_json"));
        assert!(!EDITOR_JS.contains("function expandPuzzleImportsForPreviewRequest("));
        assert!(!EDITOR_JS.contains("function expandPuzzleImportsForWasm("));
        assert!(!EDITOR_WORKSPACE_JS.contains("function ensurePuzzleImportDocumentsLoaded("));
        assert!(EDITOR_JS.contains("workspaceDocuments: workspaceCompilerDocuments(document)"));
        assert!(EDITOR_JS.contains("module.editor_solver_cache_policy_json"));
        assert!(EDITOR_JS.contains("function loadLevelSourceEntryWithExportData("));
        assert!(EDITOR_JS.contains("function reportLevelSourceLoadFailure("));
        assert!(EDITOR_JS.contains("const artifacts = new Map();"));
        assert!(EDITOR_JS.contains("entry.id !== displayedArtifactId"));
        assert!(EDITOR_JS.contains("entry.activeSolves === 0"));
        assert!(EDITOR_JS.contains("window.PuzzleRuntimeExportJson"));
        assert!(EDITOR_JS.contains(
            "const runtimeExportLiteral = extractAssignedStringLiteral(source, \"PuzzleRuntimeExportJson\");"
        ));
        assert!(EDITOR_JS.contains("function extractAssignedStringLiteral(source, windowName)"));
        assert!(EDITOR_JS.contains("function extractStringLiteralAt(source, start)"));
        assert!(EDITOR_JS.contains("requirePreviewFrame: true,"));
        assert!(!EDITOR_JS.contains("Compiling level metadata"));
        assert!(EDITOR_JS.contains("Could not load level editor source contract:"));
        assert!(EDITOR_JS.contains(
            "const levels = exportData?.levels || [];\n  let levelIndex = levels.length"
        ));
        assert!(EDITOR_JS.contains("activeLevelIndex = levelIndex;"));
        assert!(
            EDITOR_JS
                .contains("return generatedUnnamedLevelName(fallbackName) ? \"\" : fallbackName;")
        );
        assert!(EDITOR_JS.contains("function generatedUnnamedLevelName(value)"));
        assert!(!EDITOR_JS.contains("function renderEditorLevelBoardDom("));
        assert!(!EDITOR_CSS.contains(".level-board .level-cell-token"));
        assert!(EDITOR_JS.contains("function currentFocused2dLevelEntry("));
        assert!(EDITOR_JS.contains("function focusedLevelEntryForPaneMode("));
        assert!(EDITOR_JS.contains("function loadLevelPaneEntryForMode("));
        assert!(EDITOR_JS.contains("const current = currentFocused2dLevelEntry(context);"));
        assert!(EDITOR_JS.contains("currentLevelSourceLocation()"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("function sourceEditableEntryFromTarget(source, target, options = {})")
        );
        assert!(
            EDITOR_JS
                .contains("const sourceEntry = sourceEditableEntryFromTarget(source, target, {")
        );
        assert!(EDITOR_JS.contains("function sourceLevelStateFromEntry(source, entry, exportData = currentLevelExportData(), options = {})"));
        assert!(
            !EDITOR_JS
                .contains("const sourceExportData = levelEditorSourceExportData(source, entry);")
        );
        assert!(EDITOR_JS.contains("function sourceLevelRowsAndLocalLegends(source, entry)"));
        assert!(EDITOR_JS.contains("function sourceLevelEntryHasHeader(tokens)"));
        assert!(EDITOR_JS.contains("sourceLevelRegionGroups(parsed.rows)"));
        assert!(EDITOR_JS.contains("if (text === \"+\")"));
        assert!(EDITOR_JS.contains(
            "if (!loadLevelFromSourceEntry(source, entry, { ...options, exportData, levelIndex, levelName }))"
        ));
        assert!(!EDITOR_JS.contains(
            "if (!loadLevelFromSourceEntry(source, sourceEntry, { levelIndex, levelName }))"
        ));
        assert!(EDITOR_JS.contains(
            "function levelSourceData(source = currentLevelAuthoringSource(), exportData = currentLevelExportData())"
        ));
    }

    #[test]
    fn level_source_previews_do_not_indent_map_rows() {
        assert!(EDITOR_JS.contains(
            "levelDefinitionSource(levelName, levelSourceData(currentLevelAuthoringSource()), \"\", { leadingBlank: false, bodyIndent: \"\" })"
        ));
        assert!(EDITOR_SOURCE_JS.contains("function sourcePuzzleLevelHeaderSource("));
        assert!(EDITOR_JS.contains("sourcePuzzleLevelHeaderSource(levelName, levelIndent"));
        assert!(EDITOR_LEVEL3D_JS.contains("sourcePuzzleLevelHeaderSource(levelName, indent"));
        assert!(EDITOR_JS.contains(
            "const rowIndent = Object.prototype.hasOwnProperty.call(options, \"bodyIndent\") ? options.bodyIndent : levelIndent;"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3dSourcePreview.textContent = level3dSnippetSource(levelName, sourceData, \"\", { bodyIndent: \"\" });"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "const bodyIndent = Object.prototype.hasOwnProperty.call(options, \"bodyIndent\") ? options.bodyIndent : `${indent}  `;"
        ));
    }

    #[test]
    fn level_source_previews_use_canonical_quoted_level_headers() {
        assert!(EDITOR_SOURCE_JS.contains("function sourcePuzzleQuotedText("));
        assert!(EDITOR_SOURCE_JS.contains("function sourcePuzzleLevelHeaderName("));
        assert!(EDITOR_JS.contains(
            "sourcePuzzleLevelHeaderSource(levelName, levelIndent, { openBlock: true })"
        ));
        assert!(EDITOR_JS.contains("sourcePuzzleLevelHeaderSource(levelName, levelIndent)"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "sourcePuzzleLevelHeaderSource(levelName, indent, { defaultName: \"level 1\", openBlock: true })"
        ));
        assert!(EDITOR_HTML.contains(r#"id="level3dNameInput" type="text" value="level 1""#));
        assert!(!EDITOR_LEVEL3D_JS.contains("level ${sanitizeLevel3dName(name)}"));
    }

    #[test]
    fn level_name_picker_uses_parsed_level_definitions_only() {
        assert!(EDITOR_JS.contains("function findLevelSourceEntries(source, document)"));
        assert!(EDITOR_JS.contains(
            "for (const entry of surfaceEntriesForSource(source).filter((candidate) => candidate.kind === \"level\"))"
        ));
        assert!(EDITOR_JS.contains(
            "sourceName: Object.prototype.hasOwnProperty.call(entry, \"sourceName\") ? entry.sourceName : entry.name || \"\","
        ));
        assert!(
            !EDITOR_JS.contains("for (const entry of findLevelDefinitions(source, range) || [])")
        );
        assert!(!EDITOR_JS.contains("code.match(/^level(?:\\\\s+(.+?))?\\\\s*(?:\\\\{|$)/)"));
        assert!(!EDITOR_JS.contains("const rawName = String(match[1] || \"\").trim();"));
        assert!(!EDITOR_JS.contains("name: rawName.replace"));
    }

    #[test]
    fn level_editor_allows_unnamed_2d_levels() {
        assert!(EDITOR_HTML.contains(r#"id="levelNameInput" type="text" value="""#));
        assert!(
            EDITOR_JS.contains(
                "const name = \"\";\n  const sourceData = defaultEmptyLevel2dSourceData();"
            )
        );
        assert!(EDITOR_JS.contains("return sourcePuzzleLevelName(editableLevelName(value));"));
        assert!(
            EDITOR_JS
                .contains("levelName ? sourcePuzzleLevelHeaderSource(levelName, levelIndent, { openBlock: true }) : `${levelIndent}{`")
        );
        assert!(EDITOR_JS.contains("setStatus(levelName ? `Updated level ${levelName}` : \"Updated unnamed level\", \"is-ok\");"));
    }

    #[test]
    fn level_pane_loads_existing_source_levels_instead_of_add_choice_screen() {
        assert!(!EDITOR_HTML.contains(r#"id="levelEmptyPane""#));
        assert!(!EDITOR_HTML.contains("Add 2D level"));
        assert!(!EDITOR_HTML.contains("Add 3D level"));
        assert!(EDITOR_JS.contains("function loadAvailableLevelPaneEntry("));
        assert!(EDITOR_JS.contains("function loadFocusedLevelPaneEntry("));
        assert!(
            EDITOR_JS.contains(
                "if (!loadAvailableLevelPaneEntry(focusedPuzzleSourceContext(document), {"
            )
        );
        assert!(EDITOR_JS.contains("const loadedSourceLevel = loadLevelPaneEntryForMode(\"edit\", focusedPuzzleSourceContext(), {"));
    }

    #[test]
    fn focused_puzzle_entries_consume_wasm_surface_entries() {
        assert!(EDITOR_JS.contains("function focusedPuzzleSurfaceEntries("));
        assert!(EDITOR_RUNTIME_JS.contains("let activeSourceAnalysis = null;"));
        assert!(EDITOR_RUNTIME_JS.contains(
            "const activate = requireSourceAnalysisFunction(module, \"activate_source_analysis\");"
        ));
        assert!(!EDITOR_RUNTIME_JS.contains("free_source_analysis_handle"));
        assert!(!EDITOR_RUNTIME_JS.contains("analysis.handle"));
        assert!(EDITOR_RUNTIME_JS.contains("return querySynchronizedAnalysisWorker(\"outline\""));
        assert!(EDITOR_RUNTIME_JS.contains(
            "return querySynchronizedAnalysisWorker(\"outline\", source, {\n        sourceProfile: asString(payload.sourceProfile),"
        ));
        assert!(
            EDITOR_SOURCE_JS.contains("sourceProfile: puzzleSourceProfile(document),\n    });")
        );
        assert!(
            EDITOR_RUNTIME_JS.contains("new Worker(wasmModuleUrl(\"./editor_analysis_worker.js\")")
        );
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_highlight_range_json"));
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_outline_json"));
        assert!(
            EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_entries_json\")(revision)")
        );
        assert!(
            EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_suggest_source_completions")
        );
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_resolve_source_target"));
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("apply_source_analysis_edit"));
        assert!(EDITOR_SOURCE_JS.contains("applySourceAnalysisEditorChanges(sourceChanges"));
        assert!(EDITOR_CODEMIRROR_JS.contains("sourceanalysisreset"));
        assert!(EDITOR_RUNTIME_JS.contains("async sourceEntries(source)"));
        assert!(
            EDITOR_RUNTIME_JS
                .contains("await querySynchronizedAnalysisWorker(\"entries\", asString(source))")
        );
        assert!(EDITOR_RUNTIME_JS.contains("payload: null,"));
        assert!(!EDITOR_RUNTIME_JS.contains(
            "const raw = querySourceAnalysis(module, revision, \"active_source_analysis_json\");"
        ));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime?.sourceEntries"));
        assert!(EDITOR_JS.contains("await loadSurfaceEntriesForSource(context.source"));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime.sourceEntries(text)"));
        assert!(EDITOR_JS.contains(
            "currentContext?.document?.id !== documentId || currentContext.source !== context.source"
        ));
        assert!(EDITOR_WORKSPACE_JS.contains("async function ensureEditorWasmParserLoaded()"));
        assert!(EDITOR_WORKSPACE_JS.contains(
            "const wasmParserLoad = ensureEditorWasmParserLoaded();\n  void wasmParserLoad.catch"
        ));
        assert!(
            !EDITOR_WORKSPACE_JS
                .contains("await ensureEditorWasmParserLoaded();\n  if (editorSeed)")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("await loadWasmCompiler();"));
        assert!(EDITOR_JS.contains("throw new Error(message);"));
        assert!(EDITOR_JS.contains("console.warn(\"Focused source entries unavailable\", error);"));
        assert!(
            EDITOR_JS.contains("return [];\n  }\n}\n\nfunction focusedPuzzleSurfaceEntriesByKind")
        );
        assert!(
            !EDITOR_JS
                .contains("return [];\n  }\n  const raw = compiler.source_entries_json(text);")
        );
        assert!(!EDITOR_JS.contains("compiler.source_entries_json(text)"));
        assert!(EDITOR_JS.contains("focusedPuzzleSurfaceEntriesByKind(\"level\""));
        assert!(EDITOR_JS.contains("focusedPuzzleSurfaceEntriesByKind(\"level3d\""));
        assert!(EDITOR_JS.contains("focusedPuzzleSurfaceEntriesByKind(\"sprite\""));
        assert!(EDITOR_JS.contains("focusedPuzzleSurfaceEntriesByKind(\"sprite3d\""));
        assert!(!EDITOR_JS.contains("sourceSprite3dTargetAtPosition("));
        assert!(!EDITOR_JS.contains("const sprite3dTarget = sourceSprite3dTargetAtPosition"));
        assert!(!EDITOR_JS.contains("for (const range of findLevelsRanges(source) || []) {\n      if (sourcePositionInsideRanges(range.start, level3dRanges))"));
        assert!(!EDITOR_JS.contains("for (const range of findLevels3Ranges(source) || []) {\n    entries.push(...(findLevel3dDefinitions(source, range)"));
        assert!(
            !EDITOR_JS.contains("entries.push(...(findSprite3dDefinitions(source, block) || []));")
        );
    }

    #[test]
    fn editor_dimension_follows_active_source_profile() {
        assert!(EDITOR_WORKSPACE_JS.contains("function puzzleSourceProfile(document)"));
        assert!(
            EDITOR_JS.contains("function editorDimensionForDocument(document = activeDocument())")
        );
        assert!(
            EDITOR_JS.contains(
                "const documentDimension = editorDimensionForDocument(context.document);"
            )
        );
        assert!(EDITOR_JS.contains(
            "if (documentDimension && normalized !== documentDimension) {\n    return [];\n  }"
        ));
        assert!(
            EDITOR_JS
                .contains("const sourceDimension = editorDimensionForDocument(context?.document);")
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true, loadFirst: false });"
        ));
    }

    #[test]
    fn level_name_picker_does_not_write_dimension_prefix_into_value() {
        assert!(
            !EDITOR_JS.contains(
                "value: `${editorDimensionLabel(item.dimension)} ${displayName || name}`"
            )
        );
        assert!(
            EDITOR_JS.contains("value: displayName || name,\n      label: displayName || name,")
        );
        assert!(EDITOR_SOURCE_JS.contains("const optionLabel = config.optionLabel || null;"));
        assert!(EDITOR_SOURCE_JS.contains("button.textContent = entry.label || entry.value;"));
    }

    #[test]
    fn level_source_preview_generated_legends_reserve_existing_legend_chars() {
        assert!(EDITOR_JS.contains(
            "createLevelLegendAllocator(charEntries, sourceReservedLegendChars(source))"
        ));
        assert!(EDITOR_JS.contains("const candidates = levelLegendCandidateChars();"));
        assert!(EDITOR_JS.contains("function levelLegendCandidateChars()"));
        assert!(EDITOR_JS.contains("[0x2500, 0x257F]"));
        assert!(EDITOR_JS.contains("[0x2600, 0x26FF]"));
        assert!(EDITOR_JS.contains("function sourceReservedLegendChars(source)"));
        assert!(EDITOR_JS.contains("function sourceAllLegendRows(source)"));
        assert!(EDITOR_JS.contains("No unused single-character legend symbol is available"));
        assert!(!EDITOR_JS.contains("return \".\";\n      }\n      usedChars.add(ch);"));
    }

    #[test]
    fn level_editor_palette_can_add_common_legend_entries() {
        assert!(EDITOR_JS.contains("addPaletteOpen: false"));
        assert!(EDITOR_JS.contains("function renderLevelAddLegendButton()"));
        assert!(EDITOR_JS.contains("function levelPaletteAddCandidates("));
        assert!(EDITOR_JS.contains("function addLevelPaletteObjectToLegend(object)"));
        assert!(EDITOR_JS.contains("function insertCommonLegendEntry(source, entry)"));
        assert!(EDITOR_JS.contains("levelPalette.append(renderLevelAddLegendButton());"));
        assert!(EDITOR_JS.contains("sourcePlaceableObjectNames(source, exportData)"));
        assert!(EDITOR_JS.contains("!String(object.name || \"\").startsWith(\"@\")"));
        assert!(EDITOR_JS.contains("function legendBlockInsertionIndent("));
        assert!(EDITOR_JS.contains("return lineIndent(lines[index].raw);"));
        assert!(EDITOR_JS.contains("schedulePreview();"));
        assert!(EDITOR_JS.contains("No editable puzzle source for tile legend"));
        assert!(EDITOR_CSS.contains(".level-palette-add-menu"));
        assert!(EDITOR_CSS.contains(".level-palette-add-menu-item"));
    }

    #[test]
    fn editor_compile_log_splits_plain_multiline_errors() {
        assert!(EDITOR_JS.contains("function appendPlainCompileError("));
        assert!(EDITOR_JS.contains("function plainCompileErrorMessages("));
        assert!(EDITOR_JS.contains(".split(/\\r?\\n/)"));
        assert!(EDITOR_JS.contains("appendPlainCompileError(error, options);"));
    }

    #[test]
    fn editor_compile_diagnostic_links_require_explicit_line_numbers() {
        assert!(EDITOR_JS.contains("const line = positiveInteger(diagnostic?.line);"));
        assert!(EDITOR_JS.contains("if (!line) {\n    return null;\n  }"));
        assert!(!EDITOR_JS.contains("sourceLocationForDiagnosticLine"));
        assert!(!EDITOR_JS.contains("sourceCodeForDiagnosticMatch"));
        assert!(!EDITOR_JS.contains("searchStart"));
    }

    #[test]
    fn editor_source_uses_bundled_codemirror() {
        assert!(EDITOR_HTML.contains(r#"id="sourceEditorMount""#));
        assert!(!EDITOR_HTML.contains(r#"id="sourceLineNumbers""#));
        assert!(!EDITOR_HTML.contains(r#"id="sourceHighlight""#));
        assert!(!EDITOR_HTML.contains(r#"<textarea id="sourceEditor""#));
        let bundle = EDITOR_HTML
            .find(r#"<script src="editor_codemirror.js"></script>"#)
            .expect("CodeMirror bundle script");
        let dom = EDITOR_HTML
            .find(r#"<script src="editor_dom.js"></script>"#)
            .expect("editor DOM script");
        assert!(bundle < dom);
        assert!(EDITOR_DOM_JS.contains("PuzzleSourceEditorBundle.createSourceEditor"));
        assert!(EDITOR_CODEMIRROR_JS.contains("createSourceEditor"));
        assert!(
            EDITOR_SOURCE_JS.contains("sourceEditor.sourceEditorPort?.kind === \"codemirror\"")
        );
    }

    #[test]
    fn codemirror_highlight_consumes_typed_rust_spans_as_decorations() {
        let payload = EditorService::highlight_source_json(
            "title = \"Demo\"\n",
            puzzle_lang::PuzzleSourceProfile::Puzzle2d,
        );
        assert!(payload.contains("\"version\":3"));
        assert!(payload.contains("\"offsetEncoding\":\"utf8\""));
        assert!(payload.contains("\"range\":{\"start\":0,"));
        assert!(payload.contains("\"kind\":\"keyword\""));
        assert!(!payload.contains("\"html\""));
        assert!(EDITOR_SOURCE_JS.contains(
            "sourceEditor.sourceEditorPort.applyHighlightRange(source, range, payload);"
        ));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditorPort.highlightViewportRange()"));
        assert!(EDITOR_SOURCE_JS.contains("sourceOutlineStructureSignature(sourceOutlineItems)"));
        assert!(EDITOR_SOURCE_JS.contains("syncSourceOutlineRowOffsets();"));
        assert!(!EDITOR_SOURCE_JS.contains("sourceHighlightRunsFromHtml"));
        assert!(!EDITOR_SOURCE_JS.contains("payload.html"));
        assert!(EDITOR_SOURCE_JS.contains(
            "requestId !== sourceHighlightRequestId\n      || source !== sourceEditorDocumentValue()"
        ));
        assert!(EDITOR_CODEMIRROR_JS.contains("Unsupported Rust source highlight span contract."));
        assert!(EDITOR_CODEMIRROR_JS.contains("offsetEncoding"));
        assert!(EDITOR_CODEMIRROR_JS.contains("utf8"));
        assert!(
            EDITOR_CODEMIRROR_JS.contains("Cannot apply stale source highlighting to CodeMirror.")
        );
    }

    #[test]
    fn codemirror_folding_consumes_parser_owned_ranges() {
        let payload = puzzle_lang::analyze_source("puzzle demo {\n  rules {\n    move\n  }\n}\n")
            .outline_json();
        assert!(payload.contains("\"folds\":[{"));
        assert!(payload.contains("\"version\":1"));
        assert!(payload.contains("\"offsetEncoding\":\"utf8\""));
        assert!(EDITOR_CODEMIRROR_JS.contains("foldService.of(sourceFoldRangeForLine)"));
        assert!(EDITOR_CODEMIRROR_JS.contains("foldGutter({"));
        assert!(EDITOR_CODEMIRROR_JS.contains("...foldKeymap"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.sourceEditorPort.applyFoldRanges(source, payload);")
        );
        assert!(!EDITOR_CODEMIRROR_JS.contains("sourceFoldableBlocks"));
    }

    #[test]
    fn solver_pane_consumes_explicit_solver_task() {
        assert!(EDITOR_HTML.contains(r#"id="previewSolveButton""#));
        assert!(EDITOR_DOM_JS.contains("const previewSolveButton = document.querySelector"));
        assert!(EDITOR_CSS.contains(".preview-solve-button.is-solving"));
        assert!(EDITOR_HTML.contains(r#"id="solverLevelSelect""#));
        assert!(
            EDITOR_HTML.contains(r#"class="solver-task-readout" aria-label="Current solver task""#)
        );
        assert!(EDITOR_HTML.contains(r#"id="solverTargetName""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Load level""#));
        assert!(!EDITOR_HTML.contains("solver-load-icon"));
        assert!(EDITOR_DOM_JS.contains("const solverLevelSelect = document.querySelector"));
        assert!(EDITOR_DOM_JS.contains("const solverTargetName = document.querySelector"));
        assert!(EDITOR_CSS.contains(".solver-level-select"));
        assert!(EDITOR_CSS.contains(".solver-task-readout"));
        assert!(EDITOR_CSS.contains(".solver-load-control"));
        assert!(EDITOR_JS.contains("let activeSolverTask = null;"));
        assert!(EDITOR_JS.contains("let solverSelectedLevelIndex = null;"));
        assert!(EDITOR_JS.contains("let activeSolverDisplaySceneRequestKey = \"\";"));
        assert!(EDITOR_JS.contains("let completedSolverTaskKey = \"\";"));
        assert!(!EDITOR_JS.contains("let solverLevelIndex = 0;"));
        assert!(!EDITOR_JS.contains("solverStateOverride"));
        assert!(!EDITOR_JS.contains("solverSceneOverride"));
        assert!(!EDITOR_JS.contains("stagedSolverCells"));
        assert!(EDITOR_JS.contains("function syncSolverTaskReadout("));
        assert!(EDITOR_JS.contains("function createSolverTask("));
        assert!(EDITOR_JS.contains("function createPreviewSolverTask("));
        assert!(EDITOR_JS.contains("function createEditorSolverTask("));
        assert!(EDITOR_JS.contains("function setActiveSolverTask("));
        assert!(EDITOR_JS.contains("function solverTaskRunKey("));
        assert!(EDITOR_JS.contains("function isSolverTaskComplete("));
        assert!(EDITOR_JS.contains("function markActiveSolverTaskComplete("));
        assert!(EDITOR_JS.contains("function refreshVisiblePreviewSolverTask("));
        assert!(EDITOR_JS.contains("const solverObservationLiveIntervalMs = 500;"));
        assert!(EDITOR_JS.contains("progressIntervalMs: solverObservationLiveIntervalMs,"));
        assert!(EDITOR_JS.contains("throw new Error(\"Solver progress interval is invalid\");"));
        assert!(EDITOR_JS.contains("function previewStateMatchesSolverTask("));
        assert!(EDITOR_JS.contains("function applyPreviewSceneToActiveSolverTask("));
        assert!(EDITOR_JS.contains("activeSolverTask.scene = cloneJson(previewState.scene);"));
        assert!(EDITOR_JS.contains("function refreshActiveSolverTaskDisplayScene("));
        assert!(EDITOR_JS.contains("solverTaskInitialDisplayState(JSON.stringify(request))"));
        assert!(EDITOR_JS.contains("Solver display failed:"));
        assert!(EDITOR_JS.contains("return null;"));
        assert!(!EDITOR_JS.contains("scene: previewSceneForLevel(targetIndex, exportData)"));
        assert!(EDITOR_JS.contains("requestFocusedPreviewState();"));
        assert!(EDITOR_JS.contains("return \"Current board\";"));
        assert!(EDITOR_JS.contains("function solverTaskLevelLabel("));
        assert!(EDITOR_JS.contains("solverTargetName.textContent = solverTaskLevelLabel();"));
        assert!(EDITOR_JS.contains("return level?.name || `Level ${index + 1}`;"));
        assert!(EDITOR_JS.contains(
            "placeholder.textContent = levels.length ? \"Load\" : \"No level to solve\";"
        ));
        assert!(EDITOR_JS.contains("solverLevelSelect.value = \"\";"));
        assert!(
            !EDITOR_JS.contains("solverLevelSelect.value = levels.length ? selectedValue : \"\";")
        );
        assert!(!EDITOR_JS.contains("Edited level"));
        assert!(!EDITOR_JS.contains("active-task"));
        assert!(!EDITOR_JS.contains("return \"Preview state\";"));
        assert!(!EDITOR_JS.contains("const state = task.state?.kind || \"state\";"));
        assert!(!EDITOR_JS.contains("return `${producer}: ${level} (${state})`;"));
        assert!(EDITOR_JS.contains("refreshVisiblePreviewSolverTask(previewExport);"));
        assert!(EDITOR_JS.contains("if (!activeSolverTask && currentPreviewMode === \"solver\")"));
        assert!(EDITOR_JS.contains("function syncSolverLevelSelector("));
        assert!(EDITOR_JS.contains("function selectSolverLevel("));
        assert!(EDITOR_JS.contains("solverSelectedLevelIndex = levelIndex;"));
        assert!(!EDITOR_JS.contains("const levelIndex = setActiveLevelIndex(index, exportData);"));
        assert!(
            EDITOR_JS.contains("const task = createPreviewSolverTask(exportData, levelIndex);")
        );
        assert!(EDITOR_JS.contains("if (solverLevelSelect.value === \"\")"));
        assert!(EDITOR_JS.contains("solverLevelSelect?.addEventListener(\"change\""));
        assert!(!EDITOR_JS.contains("function createPreviewSolverTarget("));
        assert!(!EDITOR_JS.contains("function createEditorSolverTarget("));
        assert!(!EDITOR_JS.contains("function setActiveSolverTarget("));
        assert!(!EDITOR_JS.contains("function syncSolverTargetFromActiveLevel("));
        assert!(!EDITOR_JS.contains("function syncSolverTargetFromSourceTarget("));
        assert!(!EDITOR_JS.contains("async function syncSolverTargetFromSourceCursor("));
        assert!(EDITOR_JS.contains("function openSolverPaneForCurrentLevel("));
        let open_solver = EDITOR_JS
            .find("async function openSolverPaneForCurrentLevel()")
            .expect("solver pane opener");
        let open_solver_end = EDITOR_JS[open_solver..]
            .find("function levelRows(")
            .map(|index| open_solver + index)
            .expect("solver pane opener end");
        let open_solver_source = &EDITOR_JS[open_solver..open_solver_end];
        assert!(!open_solver_source.contains("ensurePreviewTargetsActiveDocument();"));
        assert!(!open_solver_source.contains("syncSourceFromPreviewPane(\"solver\")"));
        assert!(open_solver_source.contains("solverSelectedLevelIndex = null;"));
        assert!(open_solver_source.contains("await ensurePreviewSolverExportData();"));
        assert!(EDITOR_JS.contains("async function ensurePreviewSolverExportData()"));
        assert!(EDITOR_JS.contains("await prepareEditorSolverArtifact({"));
        assert!(EDITOR_JS.contains("setLevelSolveStatus(\"Preparing solver\", \"\");"));
        assert!(EDITOR_JS.contains("exportData.__solverArtifactId = prepared.artifactId;"));
        assert!(!EDITOR_JS.contains("Preview failed: ${userFacingRuntimeError(error)}"));
        assert!(open_solver_source.contains("return Boolean(activeSolverTask);"));
        assert!(EDITOR_JS.contains("async function solvePreviewPaneCurrentLevel()"));
        assert!(EDITOR_JS.contains("const ready = await openSolverPaneForCurrentLevel();"));
        assert!(EDITOR_JS.contains("if (!ready) {"));
        assert!(!open_solver_source.contains("resolveSourceTargetFromWasm("));
        assert!(
            EDITOR_JS.contains(
                "solverModeButton.addEventListener(\"click\", () => {\n  openSolverPaneForCurrentLevel().catch((error) => {"
            )
        );
        assert!(EDITOR_JS.contains(
            "previewSolveButton?.addEventListener(\"click\", () => {\n  solvePreviewPaneCurrentLevel().catch((error) => {"
        ));
        assert!(!EDITOR_JS.contains("compilingMessage: \"Compiling preview for solve\""));
        assert!(EDITOR_JS.contains("async function solveEditedLevelFromEditor()"));
        assert!(EDITOR_JS.contains("function compiledLevelStateData("));
        assert!(EDITOR_JS.contains("function solverPuzzle3dPreviewSnapshot("));
        assert!(EDITOR_JS.contains("const solve = module.solve_solver_task_json_with_progress;"));
        assert!(EDITOR_JS.contains("const solutionJson = solve(JSON.stringify(request),"));
        assert!(
            EDITOR_JS.contains("const entry = artifacts.get(String(data.artifactId || \"\"));")
        );
        assert!(EDITOR_JS.contains("function solverRequestForTask(task)"));
        assert!(EDITOR_JS.contains("if (isSolverTaskComplete(task))"));
        assert!(EDITOR_JS.contains(
            "setLevelSolveStatus(\"This level has already been solved\", \"is-error\");"
        ));
        assert!(EDITOR_JS.contains("markActiveSolverTaskComplete();"));
        assert!(EDITOR_JS.contains("button.disabled = taskComplete;"));
        assert!(!EDITOR_JS.contains("function solverRequestForTarget(target)"));
        assert!(!EDITOR_JS.contains("rules.source"));
        assert!(!EDITOR_JS.contains("task.rules.source"));
        assert!(!EDITOR_JS.contains("const solve = module.solve_state;"));
        assert!(!EDITOR_JS.contains("stateJson: JSON.stringify(stateData)"));
        assert!(!EDITOR_JS.contains("function solveLevelInMainThread("));
        assert!(!EDITOR_JS.contains("backend: \"wasm-main\""));
        assert!(!EDITOR_JS.contains("Solving in this browser tab"));
        assert!(EDITOR_JS.contains("Solver worker failed:"));
        assert!(
            EDITOR_JS.contains("return currentPreviewMode === \"edit\" && levelPlaytestActive;")
        );
        assert!(EDITOR_JS.contains("requestFocusedPreviewState();"));
        assert!(
            !EDITOR_JS.contains("syncPreviewStateFromLevel();\n  try {\n    worker.postMessage")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dEditedSnapshotAppliesToLevel("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dCellsWithObjectDescriptors("));
        assert!(
            EDITOR_JS
                .contains("isPuzzle3dExport(exportData) && typeof renderPuzzle3dSolverPreview")
        );
    }

    #[test]
    fn preview_solve_uses_runtime_current_level() {
        assert!(EDITOR_JS.contains("function previewSolverTaskLevelIndex("));
        assert!(EDITOR_JS.contains("const levelIndex = previewSolverTaskLevelIndex(exportData);"));
        assert!(EDITOR_JS.contains("latestPreviewState?.screenHasPuzzle !== false"));
        assert!(EDITOR_JS.contains("Number.isInteger(Number(latestPreviewState?.levelIndex))"));
        assert!(
            EDITOR_JS.contains(
                "return normalizedLevelIndex(Math.trunc(Number(latestPreviewState.levelIndex)), exportData);"
            )
        );
        assert!(
            EDITOR_JS
                .contains("return normalizedLevelIndex(solverSelectedLevelIndex, exportData);")
        );
        assert!(EDITOR_JS.contains("return currentEditableLevelIndex(exportData);"));
        let resolver = EDITOR_JS
            .split("function previewSolverTaskLevelIndex(")
            .nth(1)
            .expect("preview solver level resolver");
        let runtime_level = resolver
            .find("latestPreviewState?.levelIndex")
            .expect("runtime level branch");
        let selected_level = resolver
            .find("solverSelectedLevelIndex")
            .expect("selected level branch");
        let active_level = resolver
            .find("currentEditableLevelIndex")
            .expect("editor active level branch");
        assert!(runtime_level < selected_level);
        assert!(selected_level < active_level);
    }

    #[test]
    fn preview_edit_opens_runtime_current_level() {
        assert!(EDITOR_HTML.contains(r#"id="previewEditButton""#));
        assert!(EDITOR_HTML.contains("lucide-pencil"));
        assert!(EDITOR_DOM_JS.contains("const previewEditButton = document.querySelector"));
        assert!(EDITOR_JS.contains("function currentPreviewRuntimeLevelIndex("));
        assert!(EDITOR_JS.contains("function currentLevel3dSourceLocationForIndex("));
        assert!(EDITOR_JS.contains("function openLevelPaneForCurrentPreviewLevel("));
        assert!(
            EDITOR_JS.contains("const levelIndex = currentPreviewRuntimeLevelIndex(exportData);")
        );
        assert!(EDITOR_JS.contains("setActiveLevelIndex(levelIndex, exportData);"));
        assert!(EDITOR_JS.contains("currentLevel3dSourceLocationForIndex(levelIndex, exportData)"));
        assert!(EDITOR_JS.contains("currentLevelSourceLocation();"));
        assert!(EDITOR_JS.contains("kind: targetMode === \"level3d\" ? \"level3d\" : \"level\","));
        assert!(EDITOR_JS.contains("previewEditButton?.addEventListener(\"click\""));
        assert!(EDITOR_JS.contains("openLevelPaneForCurrentPreviewLevel();"));
        assert!(EDITOR_JS.contains("requestFocusedPreviewState();"));
    }

    #[test]
    fn preview_debug_mode_uses_lucide_icon_and_runtime_trace_contract() {
        assert!(!EDITOR_HTML.contains(r#"id="previewDebugToolbar""#));
        assert!(EDITOR_HTML.contains(r#"id="previewDebugToggleButton""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Debug""#));
        assert!(
            EDITOR_HTML.contains(r#"class="pane-header-icon-button preview-debug-toggle-button""#)
        );
        assert!(EDITOR_HTML.contains("lucide lucide-bug-icon lucide-bug"));
        assert!(
            EDITOR_HTML
                .contains(r#"id="previewDebugControls" class="preview-debug-controls" hidden"#)
        );
        assert!(EDITOR_HTML.contains(r#"id="previewDebugPrevButton""#));
        assert!(EDITOR_HTML.contains(r#"id="previewDebugNextButton""#));
        assert!(EDITOR_HTML.contains(r#"id="previewDebugLatestButton""#));
        assert!(EDITOR_HTML.contains(r#"id="previewLogTitle""#));
        assert!(EDITOR_DOM_JS.contains("const previewDebugToggleButton = document.querySelector"));
        assert!(EDITOR_DOM_JS.contains("const previewDebugControls = document.querySelector"));
        assert!(EDITOR_JS.contains("let previewDebugEnabled = false;"));
        assert!(EDITOR_JS.contains("function setPreviewDebugEnabled(enabled)"));
        assert!(EDITOR_JS.contains("function handlePreviewDebugTrace(debug, snapshot = null)"));
        assert!(EDITOR_JS.contains("type: \"PuzzleStudioSetPreviewDebugMode\""));
        assert!(EDITOR_JS.contains("event.data?.type === \"PuzzleStudioPreviewDebugTrace\""));
        assert!(EDITOR_JS.contains("previewDebugToggleButton?.addEventListener(\"click\""));
        assert!(EDITOR_JS.contains("previewDebugControls.hidden = !previewDebugEnabled;"));
        assert!(
            !EDITOR_JS.contains("previewDebugEnabled && previewFrameHasCurrentCompiledPreview")
        );
        assert!(EDITOR_JS.contains(
            "previewLogTitle.textContent = previewDebugEnabled ? \"Debug log\" : \"Log\";"
        ));
        assert!(!EDITOR_JS.contains("function previewDebugReservedBlockSize()"));
        assert!(EDITOR_CSS.contains(".preview-debug-toggle-button[aria-pressed=\"true\"]"));
        assert!(EDITOR_CSS.contains(".preview-debug-controls[hidden]"));
        assert!(EDITOR_CSS.contains(".preview-debug-control-button"));
    }

    #[test]
    fn workbench_panes_can_be_maximized_without_replacing_normal_layout() {
        assert!(EDITOR_HTML.contains(r#"data-pane-maximize="source""#));
        assert!(EDITOR_HTML.contains(r#"data-pane-maximize="preview""#));
        assert!(!EDITOR_HTML.contains("active-preview"));
        assert!(EDITOR_WORKBENCH_JS.contains("let maximizedWorkPaneId = \"\";"));
        assert!(EDITOR_WORKBENCH_JS.contains("function toggleWorkPaneMaximized(paneId)"));
        assert!(EDITOR_WORKBENCH_JS.contains("function isPaneDisplayed(paneId)"));
        assert!(EDITOR_HTML.contains("lucide-maximize-icon lucide-maximize"));
        assert!(EDITOR_WORKBENCH_JS.contains("lucide-maximize-icon lucide-maximize"));
        assert!(!EDITOR_WORKBENCH_JS.contains("lucide-maximize-2-icon lucide-maximize-2"));
        assert!(EDITOR_WORKBENCH_JS.contains("<path d=\"M8 3H5a2 2 0 0 0-2 2v3\"></path>"));
        assert!(EDITOR_WORKBENCH_JS.contains("return [maximizedWorkPaneId];"));
        assert!(
            EDITOR_WORKBENCH_JS
                .contains("maximizedWorkPaneId && maximizedWorkPaneId !== normalized")
        );
        assert!(EDITOR_CSS.contains(".workbench.is-pane-maximized .explorer-pane"));
        assert!(
            EDITOR_JS
                .contains("const maximizeButton = event.target.closest(\"[data-pane-maximize]\");")
        );
    }

    #[test]
    fn explorer_keeps_files_and_outline_in_one_visible_column() {
        assert!(EDITOR_HTML.contains(r#"class="explorer-sections""#));
        assert!(EDITOR_HTML.contains(r#"<aside class="explorer-pane" aria-label="Explorer">"#));
        assert!(EDITOR_HTML.contains("<span>Explorer</span>"));
        assert!(!EDITOR_HTML.contains(r#"<aside class="explorer-pane" aria-label="Files">"#));
        assert!(EDITOR_HTML.contains(r#"data-explorer-section="files""#));
        assert!(EDITOR_HTML.contains(r#"id="sourceOutlineList""#));
        assert!(EDITOR_CSS.contains(".explorer-sections {\n  grid-row: 2;"));
        assert!(EDITOR_CSS.contains(
            "grid-template-rows: minmax(88px, calc(100% - var(--source-outline-height) - 6px)) 6px minmax(88px, var(--source-outline-height));"
        ));
        assert!(EDITOR_CSS.contains("grid-template-rows: minmax(0, 1fr) 0 26px;"));
        assert!(EDITOR_CSS.contains(".explorer-files-section {\n  grid-row: 1;"));
        assert!(EDITOR_CSS.contains(".outline-splitter {\n  grid-row: 2;"));
        assert!(EDITOR_CSS.contains(".explorer-outline-section {\n  grid-row: 3;"));
        assert!(EDITOR_CSS.contains(".explorer-section.is-collapsed {\n  min-height: 26px;"));
        assert!(EDITOR_CSS.contains(".source-outline-chevron"));
        assert!(EDITOR_SOURCE_JS.contains("const SOURCE_OUTLINE_KIND_ICON_NAMES"));
        assert!(EDITOR_SOURCE_JS.contains("const SOURCE_OUTLINE_LIFECYCLE_ICON_NAME"));
        assert!(EDITOR_SOURCE_JS.contains("const SOURCE_OUTLINE_DEFAULT_ICON_NAME"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceOutlineKindIconSvg(kind)"));
        assert!(EDITOR_SOURCE_JS.contains("let sourceOutlineExpandedItemIds = new Set();"));
        assert!(EDITOR_SOURCE_JS.contains("function visibleSourceOutlineItems()"));
        assert!(
            EDITOR_SOURCE_JS.contains("button.setAttribute(\"aria-expanded\", String(expanded));")
        );
        assert!(EDITOR_SOURCE_JS.contains("chevron.dataset.sourceOutlineToggle = item.id;"));
        assert!(EDITOR_SOURCE_JS.contains("\"ArrowRight\", \"ArrowLeft\""));
        assert!(EDITOR_SOURCE_JS.contains("kind.innerHTML = sourceOutlineKindIconSvg(item.kind);"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "class=\"source-outline-icon lucide lucide-${name}-icon lucide-${name}\""
            )
        );
        assert!(!EDITOR_SOURCE_JS.contains("function sourceOutlineKindInitial(kind)"));
        assert!(
            !EDITOR_SOURCE_JS.contains("kind.textContent = sourceOutlineKindInitial(item.kind);")
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "if (explorerFilesCollapsed && explorerOutlineCollapsed) {\n    explorerOutlineCollapsed = false;"
        ));
        assert!(
            EDITOR_WORKSPACE_JS.contains("const outlineWasCollapsed = explorerOutlineCollapsed;")
        );
        assert!(
            EDITOR_WORKSPACE_JS.contains("scheduleSourceOutlineRefresh(true, { force: true });")
        );
        assert!(
            EDITOR_WORKSPACE_JS.contains(
                "&& (outlineWasCollapsed || (id === \"files\" && explorerFilesCollapsed));"
            )
        );
        assert!(EDITOR_WORKSPACE_JS.contains(
            "document.querySelectorAll(\"[data-explorer-section-toggle]\").forEach((toggle) => {"
        ));
    }

    #[test]
    fn source_outline_icon_mapping_matches_lucide_registry() {
        let kind_icons = js_object_string_map(EDITOR_SOURCE_JS, "SOURCE_OUTLINE_KIND_ICON_NAMES");
        let lifecycle_icon =
            js_const_string(EDITOR_SOURCE_JS, "SOURCE_OUTLINE_LIFECYCLE_ICON_NAME");
        let default_icon = js_const_string(EDITOR_SOURCE_JS, "SOURCE_OUTLINE_DEFAULT_ICON_NAME");
        let icon_names = source_outline_icon_names();
        let used_icons = kind_icons
            .values()
            .cloned()
            .chain([lifecycle_icon, default_icon])
            .collect::<HashSet<_>>();

        let missing_icons = used_icons
            .difference(&icon_names)
            .cloned()
            .collect::<BTreeSet<_>>();
        assert!(
            missing_icons.is_empty(),
            "source outline icon mapping references missing SVG definitions: {missing_icons:?}"
        );

        let unused_icons = icon_names
            .difference(&used_icons)
            .cloned()
            .collect::<BTreeSet<_>>();
        assert!(
            unused_icons.is_empty(),
            "source outline SVG definitions are not reachable from kind mapping: {unused_icons:?}"
        );

        assert_eq!(kind_icons.get("puzzle").map(String::as_str), Some("puzzle"));
        assert_eq!(
            kind_icons.get("puzzle3").map(String::as_str),
            Some("puzzle")
        );
        assert_eq!(kind_icons.get("levels").map(String::as_str), Some("map"));
        assert_eq!(kind_icons.get("levels").map(String::as_str), Some("map"));
        assert_eq!(kind_icons.get("level").map(String::as_str), Some("map"));
        assert_eq!(kind_icons.get("tags").map(String::as_str), Some("tag"));
        assert_eq!(kind_icons.get("groups").map(String::as_str), Some("group"));
        assert_eq!(
            kind_icons.get("win_conditions").map(String::as_str),
            Some("flag")
        );
        assert_eq!(
            kind_icons.get("lose_conditions").map(String::as_str),
            Some("flag-off")
        );
        assert_eq!(
            kind_icons.get("theme").map(String::as_str),
            Some("swatch-book")
        );
        assert_eq!(kind_icons.get("shapes").map(String::as_str), Some("shapes"));
        assert!(!kind_icons.contains_key("arrow"));
        assert_eq!(kind_icons.get("grid").map(String::as_str), Some("grid-2x2"));
        assert_eq!(kind_icons.get("viewport").map(String::as_str), Some("view"));
        assert_eq!(
            kind_icons.get("state").map(String::as_str),
            Some("database")
        );
        assert_eq!(
            kind_icons.get("pixelate").map(String::as_str),
            Some("file-code-2")
        );
        assert_eq!(
            kind_icons.get("animation").map(String::as_str),
            Some("circle-play")
        );
        assert_eq!(
            kind_icons.get("tween").map(String::as_str),
            Some("chart-spline")
        );
        assert_eq!(
            kind_icons.get("routine").map(String::as_str),
            Some("workflow")
        );
        assert_eq!(
            kind_icons.get("scene").map(String::as_str),
            Some("clapperboard")
        );
        assert_eq!(
            kind_icons.get("screen").map(String::as_str),
            Some("panels-top-left")
        );
    }

    #[test]
    fn source_outline_icon_mapping_covers_canonical_examples() {
        let kind_icons = js_object_string_map(EDITOR_SOURCE_JS, "SOURCE_OUTLINE_KIND_ICON_NAMES");
        let mut kinds = BTreeSet::new();

        for markdown in [
            EDITOR_DOCS_MARKDOWN,
            EDITOR_DOCS_METADATA_MARKDOWN,
            EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN,
            EDITOR_DOCS_SLOTS_MARKDOWN,
            EDITOR_DOCS_GROUPS_MARKDOWN,
            EDITOR_DOCS_TAGS_MARKDOWN,
            EDITOR_DOCS_LEGEND_MARKDOWN,
            EDITOR_DOCS_LEVELS_MARKDOWN,
            EDITOR_DOCS_LEVEL_LOCAL_LEGEND_MARKDOWN,
            EDITOR_DOCS_MESSAGES_MARKDOWN,
            EDITOR_DOCS_REWRITE_RULES_MARKDOWN,
            EDITOR_DOCS_INPUT_RULES_MARKDOWN,
            EDITOR_DOCS_MOVEMENT_MARKDOWN,
            EDITOR_DOCS_GUARDS_MARKDOWN,
            EDITOR_DOCS_FIX_MARKDOWN,
            EDITOR_DOCS_VARIABLES_MARKDOWN,
            EDITOR_DOCS_MARK_MARKDOWN,
            EDITOR_DOCS_CONDITIONS_MARKDOWN,
            EDITOR_DOCS_WIN_CONDITIONS_MARKDOWN,
            EDITOR_DOCS_SCENES_MARKDOWN,
            EDITOR_DOCS_SCENE_LAYOUT_MARKDOWN,
            EDITOR_DOCS_SEMANTIC_INPUTS_MARKDOWN,
            EDITOR_DOCS_MENUS_MARKDOWN,
            EDITOR_DOCS_LIFECYCLE_MARKDOWN,
            EDITOR_DOCS_SPRITES_MARKDOWN,
            EDITOR_DOCS_DISPLAY_MARKDOWN,
            EDITOR_DOCS_THEME_MARKDOWN,
            EDITOR_DOCS_SOUNDS_MARKDOWN,
            EDITOR_DOCS_ROUTINES_MARKDOWN,
            EDITOR_DOCS_RULE_APPLICATION_MARKDOWN,
            EDITOR_DOCS_PATTERNS_MARKDOWN,
            EDITOR_DOCS_IMPORTS_MARKDOWN,
            EDITOR_DOCS_RENDERING_MARKDOWN,
            EDITOR_DOCS_ASSETS_MARKDOWN,
            EDITOR_DOCS_RULE_EFFECTS_MARKDOWN,
            EDITOR_DOCS_SPRITE_SHAPES_MARKDOWN,
            EDITOR_DOCS_SCENE_STATE_EFFECTS_MARKDOWN,
            EDITOR_DOCS_MAPS_EXPANSION_MARKDOWN,
        ] {
            collect_puzzle_fence_outline_kinds(
                markdown,
                puzzle_lang::PuzzleSourceProfile::Puzzle2d,
                &mut kinds,
            );
        }
        collect_puzzle_fence_outline_kinds(
            EDITOR_DOCS_3D_MARKDOWN,
            puzzle_lang::PuzzleSourceProfile::Puzzle3d,
            &mut kinds,
        );

        for (source, profile) in [
            (
                r#"
title = "Outline 2D"

puzzle outline {
slots {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
move
}
}
"#,
                puzzle_lang::PuzzleSourceProfile::Puzzle2d,
            ),
            (
                include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle3"),
                puzzle_lang::PuzzleSourceProfile::Puzzle3d,
            ),
            (
                include_str!("../../lang/tests/fixtures/spec_3d_preview_contract.puzzle3"),
                puzzle_lang::PuzzleSourceProfile::Puzzle3d,
            ),
            (
                include_str!("../../lang/tests/fixtures/puzzlescript/basic_sokoban.puzzle"),
                puzzle_lang::PuzzleSourceProfile::Puzzle2d,
            ),
        ] {
            collect_outline_kinds_from_source(source, profile, &mut kinds);
        }

        let missing_kinds = kinds
            .into_iter()
            .filter(|kind| outline_kind_requires_explicit_icon(kind))
            .filter(|kind| !kind_icons.contains_key(kind))
            .collect::<BTreeSet<_>>();
        assert!(
            missing_kinds.is_empty(),
            "canonical outline examples emit kinds with no explicit icon mapping: {missing_kinds:?}"
        );
    }

    #[test]
    fn source_document_load_forces_outline_refresh() {
        assert!(EDITOR_SOURCE_JS.contains("function setSourceEditorValue(value, options = {})"));
        assert!(EDITOR_SOURCE_JS.contains("scheduleSourceOutlineRefresh(true, { force: true });"));
    }

    #[test]
    fn closing_active_tab_does_not_select_hidden_workspace_document() {
        let close_start = EDITOR_WORKSPACE_JS
            .find("function closeDocumentTab(documentId)")
            .expect("close tab handler");
        let close_end = EDITOR_WORKSPACE_JS[close_start..]
            .find("function renderDocumentTabs()")
            .expect("close tab handler end")
            + close_start;
        let close_handler = &EDITOR_WORKSPACE_JS[close_start..close_end];
        assert!(
            close_handler.contains("const nextId = openTabIds[openTabIds.length - 1] || \"\";")
        );
        assert!(
            !close_handler.contains("documents.find((document) => document.id !== documentId)")
        );
    }

    #[test]
    fn preview_run_control_becomes_refresh_without_a_stop_action() {
        assert!(!EDITOR_JS.contains("function terminatePreviewGame()"));
        assert!(!EDITOR_HTML.contains(r#"id="sourceRefreshButton""#));
        assert!(!EDITOR_DOM_JS.contains("const sourceRefreshButton = document.querySelector"));
        assert!(
            EDITOR_JS.contains("const label = running ? \"Refresh preview\" : \"Play preview\";")
        );
        assert!(EDITOR_JS.contains(
            "runButton.addEventListener(\"click\", () => {\n  runPreviewFromSourcePane();\n});"
        ));
        assert!(
            EDITOR_CSS
                .contains(".source-preview-toggle-button.is-running .source-preview-refresh-icon")
        );
        assert!(EDITOR_CSS.contains("fill: none;"));
        assert!(!EDITOR_HTML.contains("sourceStopButton"));
        assert!(!EDITOR_HTML.contains("previewPlayButton"));
        assert!(!EDITOR_HTML.contains("previewStopButton"));
        assert!(!EDITOR_HTML.contains("previewRefreshButton"));
        assert!(!EDITOR_WORKBENCH_JS.contains("terminatePreviewGame"));
        assert!(!EDITOR_WORKBENCH_JS.contains("syncPreviewFrameLifecycleForPaneVisibility"));
        assert!(!EDITOR_JS.contains("function suspendCompiledPreviewRuntime"));
    }

    #[test]
    fn unloaded_preview_hides_the_runtime_frame_until_the_replacement_loads() {
        assert!(
            EDITOR_HTML
                .contains("<div id=\"playPreview\" class=\"play-preview is-preview-unloaded\">")
        );
        let empty_document = EDITOR_JS
            .split("function emptyPreviewDocument() {")
            .nth(1)
            .and_then(|tail| tail.split("\nfunction setPreviewFrameHtml").next())
            .expect("empty preview document source");
        assert!(empty_document.contains("background: transparent;"));
        assert!(EDITOR_CSS.contains(
            ".play-preview.is-preview-unloaded .preview-frame {\n  visibility: hidden;\n}"
        ));
        assert!(EDITOR_JS.contains("function setPreviewFrameHtml(html, options = {})"));
        assert!(EDITOR_JS.contains(
            "if (options.markDocumentLoaded) {\n      setPreviewDocumentLoaded(true);\n    }"
        ));
        assert!(EDITOR_JS.contains(
            "setPreviewFrameHtml(editorPreviewDocument(html), { markDocumentLoaded: true });"
        ));
    }

    #[test]
    fn source_pane_stays_left_when_tool_panes_open() {
        assert!(EDITOR_WORKBENCH_JS.contains("next.splice(next.indexOf(SOURCE_WORK_PANE_ID), 1);"));
        assert!(EDITOR_WORKBENCH_JS.contains("next.unshift(SOURCE_WORK_PANE_ID);"));
        assert!(EDITOR_WORKBENCH_JS.contains(
            "visibleWorkPanes = normalizeVisibleWorkPaneList([SOURCE_WORK_PANE_ID, normalized]);"
        ));
    }

    #[test]
    fn source_wrap_layout_syncs_with_pane_resize() {
        assert!(
            EDITOR_SOURCE_JS.contains("function scheduleSourceEditorLayoutSync(frameCount = 1)")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "const sourceEditorWrapObserver = new ResizeObserver(() => scheduleSourceEditorLayoutSync(2));"
        ));
        assert!(EDITOR_WORKBENCH_JS.contains(
            "if (typeof scheduleSourceEditorLayoutSync === \"function\") {\n    scheduleSourceEditorLayoutSync(2);\n  }\n  syncPreviewViewportScale();"
        ));
    }

    #[test]
    fn source_highlight_refresh_preserves_current_render_while_pending() {
        assert!(
            EDITOR_SOURCE_JS
                .contains("function scheduleSourceHighlight(immediate = false, options = {})")
        );
        assert!(
            EDITOR_SOURCE_JS.contains("const preserveCurrent = options.preserveCurrent !== false;")
        );
        assert!(EDITOR_SOURCE_JS.contains("if (preserveCurrent && sourceHighlightMode)"));
        assert!(EDITOR_SOURCE_JS.contains("renderOptimisticSourceHighlight()"));
        assert!(
            EDITOR_SOURCE_JS.contains("function scheduleOptimisticSourceHighlight(source = null)")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceOptimisticHighlightFrame = window.requestAnimationFrame(() => {")
        );
        assert!(EDITOR_SOURCE_JS.contains("scheduleOptimisticSourceHighlight(predicted);"));
        assert!(!EDITOR_SOURCE_JS.contains("renderPredictedSourceHighlight(predicted);"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceHighlightRunsFromDom()"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceHighlightStyleAtOffset(runs, offset)"));
        assert!(EDITOR_SOURCE_JS.contains("function sourcePredictedBeforeInputValue(event)"));
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceBeforeInputTextInsert(event)"));
        assert!(EDITOR_SOURCE_JS.contains("function sourcePrintableKeydownEdit(event)"));
        assert!(EDITOR_SOURCE_JS.contains("if (event.key !== \"\\\"\")"));
        assert!(EDITOR_SOURCE_JS.contains("const replacement = `\"${selection}\"`;"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.setRangeText(\n    edit.replacement,"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "sourceEditor.setSelectionRange(edit.selectionStart, edit.selectionEnd);"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("sourceEditorContentChanged();"));
        assert!(EDITOR_SOURCE_JS.contains("scheduleSourceCompletion();"));
        assert!(EDITOR_SOURCE_JS.contains("syncPreviewModeFromSourceCursor();"));
        assert!(
            EDITOR_SOURCE_JS.contains("const predicted = sourceDocumentSupportsEditableTargets()")
        );
        assert!(EDITOR_SOURCE_JS.contains("? sourcePredictedBeforeInputValue(event)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const preserveCurrentHighlight = options.preserveHighlight !== false;")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "if (sameUnfoldedValue && preserveCurrentHighlight && sourceHighlightSource === nextValue)"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "scheduleSourceHighlight(true, { preserveCurrent: preserveCurrentHighlight });"
        ));
        assert!(!EDITOR_SOURCE_JS.contains("Boolean(options.preserveHighlight)"));
    }

    #[test]
    fn compiled_preview_preserves_level_only_for_the_same_document() {
        assert!(EDITOR_JS.contains("let compiledPreviewDocumentId = null;"));
        assert!(EDITOR_JS.contains(
            "const previousLevelIndex = compiledPreviewDocumentId === document.id\n    ? currentEditableLevelIndex(previewExport)\n    : null;"
        ));
        assert!(EDITOR_JS.contains("compiledPreviewDocumentId = document.id;"));
        assert!(EDITOR_JS.contains(
            "setActiveLevelIndex(previousLevelIndex ?? previewExport?.initialLevelIndex ?? 0, previewExport);"
        ));
    }

    #[test]
    fn level_editor_draft_edits_do_not_commit_preview_or_source() {
        assert!(EDITOR_JS.contains("function addLevelToSource()"));
        assert!(EDITOR_JS.contains("function updateLevelInSource()"));
        assert!(!EDITOR_JS.contains("syncTopbarEditorActions"));
        assert!(!EDITOR_JS.contains("topbar-editor-actions"));
        assert!(!EDITOR_CSS.contains("topbar-editor-actions"));
        assert!(EDITOR_WORKBENCH_JS.contains("function toolPaneHeaderActionGroups(paneId)"));
        assert!(EDITOR_WORKBENCH_JS.contains("const actions = document.createElement(\"div\");"));
        assert!(EDITOR_WORKBENCH_JS.contains("actions.className = \"pane-actions\";"));
        assert!(EDITOR_WORKBENCH_JS.contains("actions.append(group);"));
        assert!(EDITOR_WORKBENCH_JS.contains("header.append(title, actions);"));
        assert!(EDITOR_WORKBENCH_JS.contains("syncToolPaneHeaderActionGroups();"));
        assert!(EDITOR_JS.contains("const tracksSource = kind === \"level3d\" || kind === \"sprite\" || kind === \"sprite3d\";"));
        assert!(!EDITOR_JS.contains("nextExport.levels[levelIndex].initialState = stateData"));
        assert!(!EDITOR_JS.contains("previewMode === \"play\" && wasLevelMode"));
        assert!(EDITOR_JS.contains("let previewFrameHasEditorLevelState = false;"));
        assert!(EDITOR_JS.contains("function restoreCompiledGamePreview()"));
        assert!(EDITOR_JS.contains("if (previewMode === \"play\")"));
        assert!(EDITOR_JS.contains(
            "setPreviewFrameHtml(editorPreviewDocument(latestHtml), { markDocumentLoaded: true });"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("function addLevel3dToSource()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function updateLevel3dInSource()"));
        assert!(EDITOR_LEVEL3D_JS.contains("sourceEditableEntryFromTarget(source, target, {"));
        assert!(EDITOR_LEVEL3D_JS.contains("if (entry.rows?.length) {\n    loadLevel3dFromSourceDefinition(entry, source, sourceKey, sourceDocument);"));
        assert!(!EDITOR_LEVEL3D_JS.contains("syncLevel3dSourceFromState"));
        assert!(!EDITOR_LEVEL3D_JS.contains("syncPreviewStateFromLevel3d"));
        assert!(!EDITOR_LEVEL3D_JS.contains("nextExport.levels[levelIndex].size = edited.size"));
    }

    #[test]
    fn level3d_editor_updates_runtime_through_preview_contract() {
        let frame_fixture = EDITOR_JS
            .find("window\\.Puzzle3DFrameFixture")
            .expect("3D frame fixture extractor candidate");
        let puzzle_export = EDITOR_JS
            .find("window\\.PuzzleExport")
            .expect("2D export extractor candidate");
        assert!(
            frame_fixture < puzzle_export,
            "3D editor previews must extract the 3D frame fixture before the outer scene export"
        );
        assert!(EDITOR_JS.contains("const windowName = exportData?.__kind === \"puzzle3d\" ? \"Puzzle3DFrameFixture\" : \"PuzzleExport\";"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dRuntimePreviewUpdate()"));
        assert!(EDITOR_JS.contains("function ensureLevel3dRuntimePreviewForOpenPane()"));
        assert!(EDITOR_JS.contains("requireFresh: true,"));
        assert!(EDITOR_JS.contains("compilingMessage: \"Compiling 3D preview\""));
        assert!(
            EDITOR_JS
                .contains("renderLevel3dBuilder();\n    ensureLevel3dRuntimePreviewForOpenPane();")
        );
        assert!(EDITOR_LEVEL3D_JS.contains(
            "const LEVEL3D_PREVIEW_SURFACE_MESSAGE = \"PuzzleStudioPreviewSurfaceUpdate\";"
        ));
        assert!(
            EDITOR_LEVEL3D_JS.contains("const LEVEL3D_PREVIEW_SURFACE_KIND = \"puzzle3-level\";")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_PREVIEW_SURFACE_MODE = \"isolated\";"));
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_MODEL_COMPONENT_PREVIEW_MESSAGE = \"PuzzleStudioRenderPuzzle3ModelComponent\";"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPreviewSurfaceMessage(update)"));
        assert!(EDITOR_LEVEL3D_JS.contains("type: LEVEL3D_PREVIEW_SURFACE_MESSAGE"));
        assert!(EDITOR_LEVEL3D_JS.contains("kind: LEVEL3D_PREVIEW_SURFACE_KIND"));
        assert!(EDITOR_LEVEL3D_JS.contains("mode: LEVEL3D_PREVIEW_SURFACE_MODE"));
        assert!(EDITOR_LEVEL3D_JS.contains("component: update.component"));
        assert!(EDITOR_LEVEL3D_JS.contains("payload: level3dPreviewSurfacePayload(update)"));
        assert!(EDITOR_LEVEL3D_JS.contains("camera: update.camera"));
        assert!(EDITOR_LEVEL3D_JS.contains("view: update.view"));
        assert!(EDITOR_LEVEL3D_JS.contains("settings: update.settings || {}"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dRuntimePreviewDocument(update)"));
        assert!(EDITOR_LEVEL3D_JS.contains("window.PuzzleStudioInitialPreviewSurface = update;"));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("window.PuzzleStudioModelComponentPreviewFixture = function")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("Object.defineProperty(window, \"Puzzle3DFixture\""));
        assert!(EDITOR_LEVEL3D_JS.contains("next.currentScene = sceneName;"));
        assert!(EDITOR_LEVEL3D_JS.contains("puzzle-studio-initial-model-preview-boot"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("window.PuzzleStudioInitialPreviewSurfaceConsumed === true")
        );
        assert!(!EDITOR_LEVEL3D_JS.contains("type: \"PuzzleStudioSetPuzzle3Snapshot\""));
        assert!(EDITOR_LEVEL3D_JS.contains("level: {"));
        assert!(EDITOR_LEVEL3D_JS.contains("component: level3dModelPreviewComponent()"));
        assert!(EDITOR_LEVEL3D_JS.contains("const snapshot = level3dRuntimeSnapshot();"));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("if (!isPuzzle3dExport(exportData)) {\n    return null;\n  }")
        );
        assert!(!EDITOR_LEVEL3D_JS.contains("fallbackLevel3dRuntimeSnapshot"));
        assert!(EDITOR_LEVEL3D_JS.contains("resources: level3dRuntimePreviewResources(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("function showBlankLevel3dRuntimeFrame(frame)"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dDefaultPreviewTarget("));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("level3dPreviewOrigin = level3dDefaultPreviewTarget(source);")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("x: Number(previewOrigin.x) || 0,"));
        assert!(EDITOR_LEVEL3D_JS.contains("x: width / 2,"));
        assert!(EDITOR_HTML.contains("id=\"level3dRuntimeFrame\""));
        assert!(EDITOR_CSS.contains(".level3d-runtime-frame"));
        assert!(EDITOR_LEVEL3D_JS.contains("showBlankLevel3dRuntimeFrame(level3dLayerFrame);"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "function defaultLevel3dSourceDefinition(source, ranges = findLevels3Ranges(source))"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("sourceLevel3dRangeHasReadableLegend(source, range)"));
        assert!(!EDITOR_LEVEL3D_JS.contains("function sendLevel3dSnapshotToPreviewFrame"));
        assert!(EDITOR_LEVEL3D_JS.contains("function refreshLevel3dRuntimePreviews()"));
        assert!(EDITOR_LEVEL3D_JS.contains("renderLevel3dRuntime();"));
        assert!(EDITOR_LEVEL3D_JS.contains("renderLevel3dLayerRuntime();"));
        assert!(EDITOR_LEVEL3D_JS.contains("sendLevel3dLayerSnapshotToRuntime();"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "renderLevel3dLayerBoard();\n  renderLevel3dStageOverlay();\n  refreshLevel3dRuntimePreviews();\n  return true;"
        ));
        assert!(EDITOR_JS.contains("currentPreviewMode === \"level3d\" && typeof sendLevel3dSnapshotToRuntime === \"function\""));
        assert!(EDITOR_JS.contains("const task = activeSolverTask;"));
        assert!(!EDITOR_JS.contains("solverStateData(exportData)"));
        assert!(EDITOR_JS.contains(
            "isPuzzle3dExport(exportData) && typeof sendLevel3dSnapshotToRuntime === \"function\""
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dRuntimePreviewResources"));
        assert!(EDITOR_LEVEL3D_JS.contains("sprites: level3dPreviewSprites(exportData)"));
        assert!(EDITOR_LEVEL3D_JS.contains("camera: level3dRuntimePreviewCamera(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("zoom: camera.zoom,"));
        assert!(EDITOR_LEVEL3D_JS.contains("view: level3dRuntimePreviewView(snapshot)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("settings: level3dPreviewSettings(snapshot.settings || {})")
        );
        assert!(!EDITOR_LEVEL3D_JS.contains("previewFrameHasEditorLevelState = true;"));
        assert!(EDITOR_LEVEL3D_JS.contains("function renderPuzzle3dSolverPreview()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function sendPuzzle3dSolutionToSolverRuntime()"));
        assert!(EDITOR_LEVEL3D_JS.contains("level3dSolverFrame.contentWindow.postMessage(level3dPreviewSurfaceMessage(update), \"*\");"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPreviewUpdateFromSnapshot(snapshot)"));
        assert!(EDITOR_JS.contains(
            "isPuzzle3dExport(exportData) && typeof renderPuzzle3dSolverPreview === \"function\""
        ));
        assert!(EDITOR_JS.contains("typeof clearPuzzle3dSolverPreview === \"function\""));
        assert!(EDITOR_CSS.contains(".solver-board-viewport.is-puzzle3d"));
        assert!(EDITOR_CSS.contains(".solver3d-frame"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("function level3dLayerUsesRuntimeScreenFootprints(view)")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("view?.coordinateSpace === \"canvas-css-px\""));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("level3dLayerScreenPointToFootprint({ x, y }, view, width, height)")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("return height - 1 - slice;"));
        assert!(EDITOR_LEVEL3D_JS.contains("sprite: object?.sprite ?? descriptor.sprite ?? null,"));
        assert!(!EDITOR_LEVEL3D_JS.contains(
            "sprite: object?.sprite || descriptor.sprite || object?.name || descriptor.name"
        ));
    }

    #[test]
    fn level3d_microban_01_supplies_preview_contract_data() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_preview_contract.puzzle3");
        let document = puzzle_lang::parse_game(source).expect("parse Microban 3D fixture");
        let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(&document)
            .expect("export Microban 3D fixture");

        assert!(fixture_json.contains("\"levelIndex\": 0"));
        assert!(fixture_json.contains("\"name\": \"microban_01\""));
        assert!(fixture_json.contains("\"label\": \"Microban 01\""));
        assert!(fixture_json.contains("\"size\": { \"width\": 6, \"depth\": 7, \"height\": 2 }"));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 2, \"y\": 3, \"z\": 1 }, \"objects\": [{ \"id\": 3, \"name\": \"Player\", \"sprite\": null }]"
        ));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 1, \"y\": 3, \"z\": 1 }, \"objects\": [{ \"id\": 4, \"name\": \"Box\", \"sprite\": null }]"
        ));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 2, \"y\": 5, \"z\": 0 }, \"objects\": [{ \"id\": 1, \"name\": \"Floor\", \"sprite\": null }, { \"id\": 2, \"name\": \"Goal\", \"sprite\": null }]"
        ));

        assert!(fixture_json.contains("\"layerCount\": 3"));
        assert!(fixture_json.contains(
            "\"Player\": { \"id\": 3, \"name\": \"Player\", \"sprite\": null, \"layer\": 2 }"
        ));
        assert!(
            fixture_json.contains(
                "\"Box\": { \"id\": 4, \"name\": \"Box\", \"sprite\": null, \"layer\": 2 }"
            )
        );
        assert!(fixture_json.contains("\"sprites\": {"));
        assert!(
            fixture_json.contains(
                "\"camera\": { \"yawDegrees\": 10, \"pitchDegrees\": 55, \"rollDegrees\": 20, \"zoom\": 1.1 }"
            )
        );
        assert!(fixture_json.contains("\"settings\": {"));
        assert!(fixture_json.contains("\"interactiveLook\": false"));
        assert!(fixture_json.contains("\"interactiveZoom\": false"));
        assert!(fixture_json.contains("\"shade\": true"));

        assert!(EDITOR_LEVEL3D_JS.contains("level: {"));
        assert!(EDITOR_LEVEL3D_JS.contains("resources: level3dRuntimePreviewResources(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("camera: level3dRuntimePreviewCamera(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("view: level3dRuntimePreviewView(snapshot)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("settings: level3dPreviewSettings(snapshot.settings || {})")
        );
    }

    #[test]
    fn level3d_editor_syncs_source_focus_and_click_targets() {
        assert!(EDITOR_SOURCE_JS.contains("sourceInteractionFromPointer(event)"));
        assert!(EDITOR_SOURCE_JS.contains("syncPreviewModeFromSourceCursor({"));
        assert!(EDITOR_LEVEL3D_JS.contains("registerSourceEditableTarget?.(\"level3d\""));
        assert!(EDITOR_SOURCE_JS.contains("scheduleSourceCursorPreviewSync();"));
        assert!(EDITOR_JS.contains("function syncPreviewModeFromSourceCursor(options = {})"));
        assert!(EDITOR_JS.contains("[\"edit\", \"level3d\", \"sprite\", \"sprite3d\", \"sounds\"].includes(currentPreviewMode)"));
        assert!(!EDITOR_JS.contains("loadSourceTargetWithJsFallback"));
        assert!(EDITOR_JS.contains("resolveSourceTargetFromWasm(source, position)"));
        assert!(EDITOR_JS.contains("Source target sync failed:"));
        assert!(EDITOR_JS.contains(
            "currentPreviewMode === \"level3d\" && typeof renderLevel3dBuilder === \"function\""
        ));
    }

    #[test]
    fn source_cursor_target_sync_fails_fast_without_js_fallback() {
        let sync_start = EDITOR_JS
            .find("function syncPreviewModeFromSourceCursor(options = {})")
            .expect("source cursor target sync function");
        let sync_end = EDITOR_JS[sync_start..]
            .find("function syncSourceFromPreviewPane")
            .map(|index| sync_start + index)
            .expect("next source cursor sync function");
        let sync = &EDITOR_JS[sync_start..sync_end];
        let catch_start = sync
            .find(".catch((error) => {")
            .expect("WASM source target failure path");
        let catch = &sync[catch_start..];

        assert!(sync.contains("resolveSourceTargetFromWasm(source, position)"));
        assert!(!sync.contains("loadSourceTargetWithJsFallback"));
        assert!(!EDITOR_JS.contains("function loadSourceTargetWithJsFallback"));
        assert!(catch.contains("sourceCursorPreviewKey = \"\";"));
        assert!(catch.contains(
            "setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, \"is-error\");"
        ));
        assert!(catch.contains("return false;"));
        assert!(!catch.contains("loadResolvedSourceTarget("));
        assert!(!catch.contains("finishSourceTargetSync("));
    }

    #[test]
    fn source_import_navigation_consumes_typed_rust_reference_without_asset_links() {
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_import_at_json"));
        assert!(EDITOR_SOURCE_JS.contains("sourceImportReference?.("));
        assert!(!EDITOR_SOURCE_JS.contains("sourceLineIsInAssetsBlock"));
        assert!(!EDITOR_SOURCE_JS.contains("sourceQuotedPathLinkForMatch"));
        assert!(!EDITOR_SOURCE_JS.contains("Asset not found"));
        assert!(!EDITOR_SOURCE_JS.contains("(?:css|script|file)"));
    }

    #[test]
    fn editor_js_does_not_embed_raw_nul_bytes() {
        assert!(!EDITOR_JS.as_bytes().contains(&0));
        assert!(EDITOR_JS.contains(
            r#"const resolveSignature = `${position}\u0000${activePaneSignature}\u0000${source}`;"#
        ));
    }

    #[test]
    fn source_click_has_one_semantic_target_sync_owner() {
        assert!(
            EDITOR_SOURCE_JS.contains("const interaction = sourceInteractionFromPointer(event);")
        );
        assert!(EDITOR_SOURCE_JS.contains("position: interaction.documentOffset,"));
        assert!(EDITOR_SOURCE_JS.contains("allowInactiveMode: true,"));
        assert!(EDITOR_SOURCE_JS.contains("recordHistory: true,"));
        assert!(!EDITOR_SOURCE_JS.contains("force: true,\n      recordHistory: true,"));
        assert!(!EDITOR_JS.contains("function loadLevelFromSourceClick"));
        assert!(!EDITOR_SPRITE_JS.contains("function loadSpriteFromSourceClick"));
        assert!(
            !EDITOR_JS
                .contains("sourceEditor.addEventListener(\"click\", loadLevelFromSourceClick);")
        );
        assert!(
            !EDITOR_SPRITE_JS
                .contains("sourceEditor.addEventListener(\"click\", loadSpriteFromSourceClick);")
        );
        assert!(EDITOR_JS.contains("function loadResolvedSourceTarget(target, options = {})"));
        assert!(EDITOR_JS.contains("function previewModeForSourceTargetKind(kind)"));
        assert!(EDITOR_JS.contains("kind: target.kind, start: target.start, end: target.end"));
        assert!(EDITOR_JS.contains("currentPreviewMode === resolvedMode"));
        assert!(EDITOR_SPRITE_JS.contains("function loadSpriteSourceTarget(target, options = {})"));
    }

    #[test]
    fn sprite_name_row_owns_new_add_and_save_lifecycle() {
        assert!(EDITOR_HTML.contains(r#"id="newSpriteButton""#));
        assert!(EDITOR_HTML.contains(r#"id="spriteInsertButton""#));
        assert!(EDITOR_HTML.contains(r#"id="spriteUpdateButton""#));
        assert!(!EDITOR_HTML.contains("duplicateSpriteButton"));
        assert!(EDITOR_HTML.contains("lucide lucide-image-plus-icon lucide-image-plus"));
        assert!(
            EDITOR_HTML.contains("lucide lucide-file-plus-corner-icon lucide-file-plus-corner")
        );
        assert!(
            EDITOR_DOM_JS
                .contains("const newSpriteButton = document.querySelector(\"#newSpriteButton\");")
        );
        assert!(!EDITOR_JS.contains("addEmptySprite2dToFocusedSource"));
        assert!(EDITOR_SPRITE_JS.contains("function newSpriteDraft()"));
        assert!(EDITOR_SPRITE_JS.contains("function addSpriteToSource()"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("canReplaceCurrentSpriteDefinition(source) ? \"duplicate\" : \"insert\"")
        );
        assert!(EDITOR_SPRITE_JS.contains("nameRow.append(labeledControl(\"Sprite for\""));
        assert!(EDITOR_SPRITE_JS.contains(
            "sourceActions.append(controls.newButton, controls.addButton, controls.saveButton);"
        ));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains(
            "setSpriteEditorSourceTarget(state, { start: result.start, end: result.end, name: result.name }, document);"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("function addEmptySpriteToSource"));
        assert!(!EDITOR_SPRITE_JS.contains("function insertEmptySpriteDefinition"));
    }

    #[test]
    fn sprite3d_source_tools_use_shared_sprite_target_contract() {
        assert!(!EDITOR_SPRITE3D_JS.contains("function findSprites3dBlocks(source)"));
        assert!(!EDITOR_SPRITE3D_JS.contains("pattern.exec(source)"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("function projectSpriteDocumentContract"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("async function commitSpriteEditorMutation"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function findSprite3dDefinitionByName"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function findSprite3dDefinitionAtPosition"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function findSprite3dDefinitions"));
        assert!(EDITOR_JS.contains("focusedPuzzleSurfaceEntriesByKind(\"sprite3d\""));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime.sourceEntries(text)"));
        assert!(!EDITOR_JS.contains("findSprite3dDefinitionByName(source, name)"));
        assert!(EDITOR_SPRITE3D_JS.contains("function sprite3dTargetPayload(target)"));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "target?.sourceSprite?.dimension === \"3d\" && target.sourceSprite.status === \"incomplete\""
        ));
        assert!(!EDITOR_SPRITE3D_JS.contains("sourceSprite3d"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function parseSprite3dDefinitionSource"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function parseSprite3dRows"));
        assert!(!EDITOR_SPRITE3D_JS.contains("typeof spriteSourceCursorPosition"));
        assert!(!EDITOR_SPRITE3D_JS.contains("typeof spriteSourceTargetAtCursor"));
        assert!(!EDITOR_SPRITE3D_JS.contains(": source.length"));
    }

    #[test]
    fn sprite3d_source_mutation_serializes_z_slices_in_source_order() {
        assert!(EDITOR_SPRITE3D_JS.contains("const worldZ = sprite3d.depth - 1 - sourceZ;"));
        assert!(EDITOR_SPRITE3D_JS.contains("frame[sprite3dCellIndex(x, y, worldZ)]"));
    }

    #[test]
    fn sprite_color_edit_undo_batches_until_commit() {
        assert!(EDITOR_SPRITE_JS.contains("function beginSpriteColorEditHistory(kind)"));
        assert!(EDITOR_SPRITE_JS.contains("function commitSpriteColorEditHistory(kind)"));
        assert!(EDITOR_SPRITE_JS.contains("updateSelectedSpriteColor(value, options = {})"));
        assert!(EDITOR_SPRITE_JS.contains("renderSpriteColorAdjuster({"));
        assert!(EDITOR_SPRITE_JS.contains("onInput: onChange,"));
        assert!(EDITOR_SPRITE_JS.contains("previewNewSpriteColor(color, { deferHistory: true })"));
        assert!(EDITOR_SPRITE3D_JS.contains("updateSelectedSprite3dColor(value, options = {})"));
        assert!(
            EDITOR_SPRITE3D_JS.contains("function previewNewSprite3dColor(color, options = {})")
        );
        assert!(EDITOR_SPRITE3D_JS.contains("onChange: previewNewSprite3dColor"));
        assert!(EDITOR_JS.contains("commitSpriteColorEditHistory(kind);"));
    }

    #[test]
    fn sprite_color_adjuster_uses_shared_custom_editor() {
        let adjuster_start = EDITOR_SPRITE_JS
            .find("function renderSpriteColorAdjuster")
            .expect("sprite color adjuster");
        let adjuster_end = EDITOR_SPRITE_JS[adjuster_start..]
            .find("function renderSpritePalette")
            .map(|index| adjuster_start + index)
            .expect("sprite palette after adjuster");
        let adjuster = &EDITOR_SPRITE_JS[adjuster_start..adjuster_end];

        assert!(adjuster.contains("window.PuzzleStudioColorEditor.create({"));
        assert!(adjuster.contains("className: \"sprite-color-adjuster\""));
        assert!(adjuster.contains("onInput: onChange"));
        assert!(EDITOR_COLOR_JS.contains("window.PuzzleStudioColorEditor = {"));
        assert!(EDITOR_COLOR_JS.contains("function create(options = {})"));
        assert!(EDITOR_COLOR_JS.contains("color-editor-plane"));
        assert!(EDITOR_COLOR_JS.contains("color-editor-hue"));
        assert!(EDITOR_COLOR_JS.contains("color-editor-alpha"));
        assert!(EDITOR_COLOR_JS.contains("color-editor-hex"));
        assert!(EDITOR_HTML.contains(r#"<script src="editor_color.js"></script>"#));
        assert!(
            EDITOR_HTML.find("editor_color.js").unwrap()
                < EDITOR_HTML.find("editor_source.js").unwrap()
        );
        assert!(
            EDITOR_HTML.find("editor_color.js").unwrap()
                < EDITOR_HTML.find("editor_sprite.js").unwrap()
        );
        assert!(!adjuster.contains("colorInput.type = \"color\";"));
        assert!(!adjuster.contains("sprite-native-color-input"));
        assert!(!EDITOR_SPRITE_JS.contains("showPicker"));
        assert!(!adjuster.contains("window.PuzzleStudioHost?.pickScreenColor"));
        assert!(!adjuster.contains("EyeDropper"));
    }

    #[test]
    fn source_color_editor_uses_shared_custom_popover() {
        assert!(
            EDITOR_SOURCE_JS.contains("const sourceColorPopover = createSourceColorPopover();")
        );
        assert!(EDITOR_SOURCE_JS.contains("function createSourceColorPopover()"));
        assert!(EDITOR_SOURCE_JS.contains("popover.className = \"source-color-popover\";"));
        assert!(EDITOR_SOURCE_JS.contains("function renderSourceColorPopover(color, token)"));
        assert!(EDITOR_SOURCE_JS.contains("window.PuzzleStudioColorEditor.create({"));
        assert!(EDITOR_SOURCE_JS.contains("onInput: applySourceColorRgb"));
        assert!(EDITOR_SOURCE_JS.contains("function applySourceColorRgb(rgb)"));
        assert!(EDITOR_SOURCE_JS.contains("function positionSourceColorPopoverForToken(token)"));
        assert!(EDITOR_SOURCE_JS.contains("positionSourceColorPopoverForToken(token);"));
        assert!(EDITOR_SOURCE_JS.contains("sourceColorPopover.hidden = false;"));
        assert!(EDITOR_SOURCE_JS.contains("sourceColorPopover.hidden = true;"));
        assert!(
            EDITOR_SOURCE_JS.contains("function hideSourceColorEditorForOutsidePointer(event)")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "document.addEventListener(\"pointerdown\", hideSourceColorEditorForOutsidePointer);"
        ));
        assert!(EDITOR_SOURCE_JS.contains("function sourceColorSelectionTargetsToken(token)"));
        assert!(EDITOR_SOURCE_JS.contains(
            "document.activeElement === sourceEditor && !sourceColorSelectionTargetsToken(current)"
        ));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "function sourceColorEventTargetsToken(event, token, visualOffset = null)"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("const offset = Number.isInteger(visualOffset)"));
        assert!(!EDITOR_SOURCE_JS.contains("sourceColorInput"));
        assert!(!EDITOR_SOURCE_JS.contains("source-native-color-input"));
        assert!(!EDITOR_SOURCE_JS.contains("showPicker"));
        assert!(!EDITOR_SOURCE_JS.contains("pickSourceColor"));
        assert!(!EDITOR_BOOT_JS.contains("async pickSourceColor(payload)"));
        assert!(!EDITOR_BOOT_JS.contains("pick_source_color"));
        assert!(!EDITOR_SOURCE_JS.contains("renderSourceColorAdjuster"));
    }

    #[test]
    fn color_editor_does_not_depend_on_eyedropper_host_api() {
        assert!(!EDITOR_BOOT_JS.contains(r#"invoke("pick_screen_color")"#));
        assert!(!EDITOR_BOOT_JS.contains(r#"invoke("pick_source_color")"#));
        assert!(!EDITOR_BOOT_JS.contains("async pickScreenColor()"));
        assert!(!EDITOR_BOOT_JS.contains("async pickSourceColor("));
        assert!(!EDITOR_BOOT_JS.contains("canPickScreenColor()"));
        assert!(!EDITOR_BOOT_JS.contains("EyeDropper"));
        assert!(!EDITOR_SPRITE_JS.contains("window.PuzzleStudioHost?.pickScreenColor"));
        assert!(!EDITOR_SPRITE_JS.contains("window.PuzzleStudioHost?.canPickScreenColor"));
        assert!(!EDITOR_SOURCE_JS.contains("window.PuzzleStudioHost.pickSourceColor"));
        assert!(!EDITOR_SOURCE_JS.contains("showPicker"));
        assert!(!EDITOR_SPRITE_JS.contains("showPicker"));
        assert!(!EDITOR_SPRITE3D_JS.contains("showPicker"));
        assert!(!EDITOR_SPRITE_JS.contains("function spriteEyedropperIconSvg()"));
        assert!(!EDITOR_SPRITE_JS.contains("sprite-palette-eyedropper-button"));
        assert!(!EDITOR_SPRITE_JS.contains("spriteEyedropperActive"));
        assert!(!EDITOR_SPRITE3D_JS.contains("sprite3dEyedropperActive"));
    }

    #[test]
    fn sprite_palette_keyboard_shortcuts_do_not_turn_tool_buttons_into_erasers() {
        assert!(EDITOR_SPRITE_JS.contains(
            "if (rawIndex === undefined) {\n      return;\n    }\n    event.preventDefault();"
        ));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "if (rawIndex === undefined) {\n    return;\n  }\n  event.preventDefault();"
        ));
    }

    #[test]
    fn editor_bucket_tools_deactivate_after_handled_fill_action() {
        assert!(EDITOR_JS.contains("function deactivateLevelBucketModeAfterUse()"));
        assert!(EDITOR_JS.contains(
            "renderLevelBoard();\n  deactivateLevelBucketModeAfterUse();\n  setStatus(level.selectedObjectId ? \"Filled connected area\""
        ));
        assert!(EDITOR_JS.contains(
            "setStatus(\"Connected area already has that tile\", \"is-ok\");\n    deactivateLevelBucketModeAfterUse();\n    return true;"
        ));
        assert!(EDITOR_SPRITE_JS.contains("function deactivateSpriteBucketModeAfterUse()"));
        assert!(EDITOR_SPRITE_JS.contains(
            "setSpriteActionStatus(\"Connected area already has that color\", \"is-ok\");\n    deactivateSpriteBucketModeAfterUse();\n    return false;"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "const message = colorIndex === null ? \"Filled connected area with transparent\" : \"Filled connected area\";\n  deactivateSpriteBucketModeAfterUse();"
        ));
        assert!(EDITOR_SPRITE3D_JS.contains("function deactivateSprite3dBucketModeAfterUse()"));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "setSprite3dActionStatus(\"Connected component already has that color\", \"is-ok\");\n    deactivateSprite3dBucketModeAfterUse();\n    return true;"
        ));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "sprite3d.hoverSlice = null;\n  deactivateSprite3dBucketModeAfterUse();\n  renderSprite3dBuilder();"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("function deactivateLevel3dLayerFillModeAfterUse()"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "return level3d.layerFillActive\n    ? bucketFillLevel3dLayerFromPosition(level3dLayerHover)\n    : paintLevel3dCellAtPosition(level3dLayerHover);"
        ));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("deactivateLevel3dLayerFillModeAfterUse();\n  if (changed) {")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("return true;\n}"));
    }

    #[test]
    fn sprite_cell_hover_preserves_pixel_color_surface() {
        assert!(EDITOR_CSS.contains("--sprite-swatch-checker: url("));
        assert!(EDITOR_CSS.contains(
            ".sprite-cell:focus-visible {\n  background-color: var(--sprite-swatch-bg);\n  background-image: var(--sprite-swatch-checker);"
        ));
        assert!(
            !EDITOR_CSS.contains(
                ".sprite-cell:hover,\n.sprite-cell:focus-visible,\n.sprite-cell:active {"
            )
        );
        assert!(!EDITOR_CSS.contains(".sprite-brush-preview"));
    }

    #[test]
    fn sprite_clip_selection_is_positioned_overlay_not_cell_paint() {
        assert!(
            EDITOR_SPRITE_JS
                .contains("frame.style.setProperty(\"--sprite-clip-x\", String(rect.x));")
        );
        assert!(
            EDITOR_SPRITE_JS.contains(
                "frame.style.setProperty(\"--sprite-clip-height\", String(rect.height));"
            )
        );
        assert!(!EDITOR_SPRITE_JS.contains("frame.style.gridColumn"));
        assert!(!EDITOR_SPRITE_JS.contains("frame.style.gridRow"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("button.classList.toggle(\"is-clip-selected\", isClipSelected);")
        );
        assert!(EDITOR_CSS.contains(
            ".sprite-board.is-clip-active .sprite-cell.is-clip-selected {\n  cursor: grab;"
        ));
        assert!(EDITOR_CSS.contains(".sprite-cell.is-clip-selected {\n  box-shadow: none;"));
        assert!(EDITOR_CSS.contains(".sprite-clip-selection-frame {\n  position: absolute;"));
        assert!(EDITOR_CSS.contains("left: calc(var(--sprite-clip-x) * var(--sprite-cell));"));
        assert!(EDITOR_CSS.contains("background: transparent;"));
        let clip_paste_cell = EDITOR_SPRITE_JS
            .split_once("function pasteSpriteClipCell(index, clipboardValue) {")
            .expect("2D clip paste cell owner exists")
            .1
            .split_once("function spriteClipCellsForCurrentPalette(clipboard) {")
            .expect("2D clip paste cell owner closes")
            .0;
        assert!(clip_paste_cell.contains("if (clipboardValue === null)"));
        assert!(clip_paste_cell.contains("return false;"));
        assert!(clip_paste_cell.contains("validSpriteColorIndex(clipboardValue)"));
        assert!(clip_paste_cell.contains("setSpriteCellColorAtIndex(index, clipboardValue)"));
        assert!(!clip_paste_cell.contains("#00000000"));
    }

    #[test]
    fn sprite_board_rerender_does_not_empty_scroll_content_before_replace() {
        assert!(EDITOR_SPRITE_JS.contains("const nextBoard = document.createDocumentFragment();"));
        assert!(EDITOR_SPRITE_JS.contains("renderSpriteClipSelectionFrame(nextBoard);"));
        assert!(EDITOR_SPRITE_JS.contains("spriteBoard.replaceChildren(nextBoard);"));
        assert!(!EDITOR_SPRITE_JS.contains("spriteBoard.replaceChildren();"));
    }

    #[test]
    fn sprite_pane_rerenders_share_scroll_preservation() {
        assert!(EDITOR_SPRITE_JS.contains("function withSpritePaneScrollPreserved("));
        assert!(EDITOR_SPRITE_JS.contains(
            "function renderSpriteControls() {\n  withSprite2dPaneScrollPreserved(() => renderSpriteControlsContent());"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "function renderSpritePalette() {\n  withSprite2dPaneScrollPreserved(() => renderSpritePaletteContent());"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "function renderSpriteBoard() {\n  withSprite2dPaneScrollPreserved(() => renderSpriteBoardContent());"
        ));
        assert!(
            EDITOR_SPRITE_JS
                .contains("function renderSpriteAnimationControls() {\n  if (!spriteBuilder) {")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("return withSpritePaneScrollPreserved(sprite3dBuilder, render);")
        );
        let capture_scroll = EDITOR_SPRITE_JS
            .split_once("function captureSpritePaneScroll(builder) {")
            .expect("shared sprite scroll capture exists")
            .1
            .split_once("function restoreSpritePaneScroll")
            .expect("shared sprite scroll capture closes")
            .0;
        assert!(!capture_scroll.contains("document.activeElement"));
    }

    #[test]
    fn sprite_translate_releases_its_own_pointer_capture_before_committing() {
        let stop_translate = EDITOR_SPRITE_JS
            .split_once("function stopSpriteTranslate(event) {")
            .expect("2D sprite translate stop handler exists")
            .1
            .split_once("function renderSpriteClipButton")
            .expect("2D sprite translate stop handler closes")
            .0;
        assert!(stop_translate.contains("spriteBoard.hasPointerCapture?.(event.pointerId)"));
        assert!(stop_translate.contains("spriteBoard.releasePointerCapture(event.pointerId)"));
        assert!(!stop_translate.contains("spritePaintDrag.pointerId"));
        assert!(
            stop_translate.contains("pushVisualEditUndoSnapshot(\"sprite\", drag.beforeSnapshot)")
        );
    }

    #[test]
    fn sprite_clip_is_a_stable_edit_region_toggle_with_permanent_commands() {
        assert!(EDITOR_SPRITE_JS.contains("const SPRITE_EDITOR_TOOL_SCHEMA = Object.freeze(["));
        assert!(EDITOR_SPRITE_JS.contains(
            "renderSpriteEditorToolbar({ dimension: \"2d\", target: spriteToolbarHost });"
        ));
        for command in ["copy", "cut", "paste", "delete"] {
            assert!(
                EDITOR_SPRITE_JS
                    .contains(&format!("{{ key: \"{command}\", group: \"clipboard\" }}"))
            );
        }
        assert!(EDITOR_SPRITE_JS.contains("function runSpriteEditCommand(dimension, command)"));
        assert!(EDITOR_SPRITE3D_JS.contains("function runSprite3dEditCommand(command)"));
        assert!(!EDITOR_HTML.contains(r#"id="sprite3dCopySliceButton""#));
        assert!(!EDITOR_HTML.contains(r#"id="sprite3dPasteSliceButton""#));
        assert!(!EDITOR_JS.contains("sliceClipboard"));
        assert!(!EDITOR_SPRITE3D_JS.contains("copySprite3dSlice"));
        assert!(!EDITOR_SPRITE3D_JS.contains("pasteSprite3dSlice"));
        assert!(EDITOR_HTML.contains(r#"id="spriteToolbarHost" class="sprite-toolbar-host""#));
        assert!(EDITOR_HTML.contains(r#"id="sprite3dToolbarHost" class="sprite-toolbar-host""#));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "renderSpriteEditorToolbar({ dimension: \"3d\", target: sprite3dToolbarHost });"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("paletteGrid.append(clipActions);"));
        assert!(EDITOR_CSS.contains(".sprite-clip-actions {\n  position: relative;"));
        assert!(EDITOR_CSS.contains("width: 26px;\n  min-width: 26px;"));
        assert!(EDITOR_CSS.contains("height: 26px;\n  min-height: 26px;"));
        assert!(!EDITOR_SPRITE_JS.contains("sprite-clip-expanded-actions"));
        assert!(!EDITOR_SPRITE3D_JS.contains("sprite-clip-expanded-actions"));
        assert!(!EDITOR_CSS.contains(".sprite-clip-expanded-actions"));
        assert!(EDITOR_SPRITE_JS.contains("spriteClipActive ? normalizeSpriteClipRect(spriteClipSelection) : spriteWholeEditRect()"));
        assert!(EDITOR_SPRITE3D_JS.contains("sprite3dClipActive ? normalizeSprite3dClipBox(sprite3dClipSelection) : sprite3dWholeEditBox()"));
        assert!(EDITOR_SPRITE_JS.contains("!spriteClipRectContainsIndex(region, current)"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("region && !sprite3dClipBoxContainsCoords(region, current)")
        );
        assert!(
            EDITOR_SPRITE3D_JS.contains("region && !sprite3dClipBoxContainsCoords(region, coords)")
        );
    }

    #[test]
    fn sprite3d_clip_uses_scope_owned_world_box_selection() {
        assert!(EDITOR_HTML.contains(r#"id="sprite3dClipActions""#));
        assert!(!EDITOR_HTML.contains(r#"id="sprite3dClearButton""#));
        assert!(!EDITOR_HTML.contains(r#"id="spriteClearButton""#));
        assert!(
            EDITOR_SPRITE_JS
                .contains("delete: renderSpriteEditCommandButton(dimension, \"delete\")")
        );
        assert!(EDITOR_SPRITE3D_JS.contains("function normalizeSprite3dClipBox(box)"));
        assert!(EDITOR_SPRITE3D_JS.contains("fullDepth: sprite3dEditScope() === \"all\""));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("box[`min${worldAxis.toUpperCase()}`] = fullDepth ? 0 : fixedStack;")
        );
        assert!(EDITOR_SPRITE3D_JS.contains(
            "box[`max${worldAxis.toUpperCase()}`] = fullDepth ? sprite3dAxisSize(worldAxis) - 1 : fixedStack;"
        ));
        assert!(
            EDITOR_SPRITE3D_JS.contains(
                "sprite3dClipBoxFromPlaneRect(rect, { base: sprite3dClipDrag.originBox })"
            )
        );
        assert!(EDITOR_SPRITE3D_JS.contains("sprite3dClipClipboardFromSelection(box, dimensions)"));
        assert!(EDITOR_SPRITE3D_JS.contains("if (clipboard.scope === \"slice\")"));
        assert!(EDITOR_SPRITE3D_JS.contains("renderSprite3dClipFloatingPreview(rect);"));
        assert!(EDITOR_SPRITE3D_JS.contains("drawSprite3dClipBounds(ctx, view);"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("const box = normalizeSprite3dClipBox(sprite3dClipSelection);")
        );
        assert!(EDITOR_CSS.contains("--sprite3d-clip-stroke:"));
        let clip_paste_cell = EDITOR_SPRITE3D_JS
            .split_once("function pasteSprite3dClipCell(index, clipboardValue) {")
            .expect("3D clip paste cell owner exists")
            .1
            .split_once("function sprite3dClipForCurrentPalette(clipboard) {")
            .expect("3D clip paste cell owner closes")
            .0;
        assert!(clip_paste_cell.contains("if (clipboardValue === null)"));
        assert!(clip_paste_cell.contains("return false;"));
        assert!(clip_paste_cell.contains("validSprite3dColorIndex(clipboardValue)"));
        assert!(!clip_paste_cell.contains("sprite3dColorForColorIndex"));
        assert!(!clip_paste_cell.contains("#00000000"));
        assert_eq!(
            EDITOR_SPRITE3D_JS
                .matches("pasteSprite3dClipCell(index, clipboard.cells[offset])")
                .count(),
            2,
            "whole and slice paste share transparent-hole semantics"
        );
        let clip_drag = EDITOR_SPRITE3D_JS
            .split_once("function continueSprite3dClip(event) {")
            .expect("3D clip drag handler exists")
            .1
            .split_once("function stopSprite3dClip(event) {")
            .expect("3D clip drag handler closes")
            .0;
        assert!(
            clip_drag.contains("renderSprite3dPreview();"),
            "3D preview follows clip selection, move, and resize while dragging"
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("sprite3dEditScope() === \"slice\" && nextAxis !== sprite3d.axis")
        );
    }

    #[test]
    fn sprite_source_actions_move_into_the_responsive_name_row() {
        assert!(EDITOR_HTML.contains(r#"id="spriteSourceActionBank" hidden"#));
        assert!(EDITOR_HTML.contains(r#"id="sprite3dSourceActionBank" hidden"#));
        assert!(EDITOR_HTML.contains(r#"id="spritePaneHeaderActions""#));
        assert!(
            EDITOR_WORKBENCH_JS.contains("document.querySelector(\"#spritePaneHeaderActions\")")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("root.append(nameRow, controls.shapeField, geometry, animation);")
        );
        assert!(EDITOR_CSS.contains(".sprite-editor-name-row {\n  flex: 0 1 330px;"));
        assert!(
            EDITOR_CSS.contains(
                ".sprite-editor-upper-controls > .sprite-shape-field {\n  flex: 0 1 200px;"
            )
        );
        assert!(EDITOR_SPRITE3D_JS.contains("function newSprite3dDraft()"));
        assert!(EDITOR_SPRITE3D_JS.contains("function addSprite3dToSource()"));
        assert!(
            EDITOR_SPRITE3D_JS.contains(
                "canReplaceCurrentSprite3dDefinition(source) ? \"duplicate\" : \"insert\""
            )
        );
    }

    #[test]
    fn sprite_size_inputs_refresh_the_preview_while_editing() {
        assert!(EDITOR_SPRITE_JS.contains(
            "spriteSizeInput.addEventListener(\"input\", () => {\n  if (spriteSizeInput.validity.valid && spriteSizeInput.value !== \"\") {\n    updateSpriteSize(spriteSizeInput.value);"
        ));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "sprite3dSizeInput?.addEventListener(\"input\", () => {\n  if (sprite3dSizeInput.validity.valid && sprite3dSizeInput.value !== \"\") {\n    updateSprite3dSize(sprite3dSizeInput.value);"
        ));
    }

    #[test]
    fn sprite_brush_size_is_pixel_based_and_paint_updates_changed_cells() {
        assert!(EDITOR_CSS.contains(
            "#spriteBuilder .sprite-board {\n  --sprite-cell: clamp(8px, calc(100cqw / var(--sprite-size)), 64px);\n}"
        ));
        assert!(EDITOR_CSS.contains("--sprite-puzzle-line: #1d242b;"));
        assert!(EDITOR_CSS.contains(
            "box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--sprite-puzzle-line) 38%, transparent);"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "button.style.setProperty(\"--sprite-puzzle-line\", spriteGridLineForColorIndex(colorIndex));"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "return validSpriteColorIndex(index) ? readableInkForColor(sprite.palette[index].color) : \"#1d242b\";"
        ));
        assert!(EDITOR_SPRITE_JS.contains("let spriteBrushSizePx = 1;"));
        assert!(EDITOR_HTML.contains(r#"id="spriteBrushSizeInput" class="sprite-brush-size-input" type="number" min="1" max="64" step="1""#));
        assert!(EDITOR_HTML.contains(
            r#"xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="sprite-marker-icon lucide lucide-highlighter-icon lucide-highlighter""#
        ));
        assert!(EDITOR_HTML.contains(r#"<path d="m9 11-6 6v3h9l3-3"></path>"#));
        assert!(EDITOR_HTML.contains(
            r#"<path d="m22 12-4.6 4.6a2 2 0 0 1-2.8 0l-5.2-5.2a2 2 0 0 1 0-2.8L14 4"></path>"#
        ));
        assert!(!EDITOR_HTML.contains(r#"<path d="m14 4 6 6"></path>"#));
        assert!(!EDITOR_HTML.contains(r#"<path d="m5 17 4 4"></path>"#));
        assert!(!EDITOR_HTML.contains("data-sprite-brush-preset"));
        assert!(EDITOR_SPRITE_JS.contains(
            "if (spriteBrushSizePx === 1) {\n    const index = spriteCellIndexFromPoint(point);\n    return index >= 0 ? [index] : [];\n  }"
        ));
        assert!(EDITOR_SPRITE_JS.contains("return Math.min(sprite.size, spriteBrushSizePx);"));
        assert!(EDITOR_SPRITE_JS.contains("return Math.min(size, spriteBrushSizePx);"));
        assert!(EDITOR_SPRITE_JS.contains("function renderSpriteCellsAtIndices(indices)"));
        assert!(EDITOR_SPRITE_JS.contains("finishSpritePaintMutation(changedIndices);"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("finishSpritePaintMutation(changedIndices, { deferSourceSync: true });")
        );
        assert!(EDITOR_SPRITE_JS.contains(
            "if (!options.deferSourceSync) {\n    updateSpriteBoundShapeDefinition();\n  }"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "if (!options.deferSourceSync) {\n    syncSpriteSourceActionButtons();\n  }"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "updateSpriteBoundShapeDefinition();\n    syncSpriteSourceActionButtons();\n    pushVisualEditUndoSnapshot(\"sprite\", spritePaintDrag.beforeSnapshot);"
        ));
        assert!(EDITOR_JS.contains(
            "if (sprite.animationMode) {\n      if (typeof ensureSpriteAnimationFrames === \"function\") {"
        ));
        assert!(EDITOR_JS.contains(
            "} else if (typeof resetSpriteAnimationFramesFromCurrentCells === \"function\") {\n      resetSpriteAnimationFramesFromCurrentCells();\n    }"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("spriteBrushPreviewElement"));
        assert!(!EDITOR_SPRITE_JS.contains("sprite-brush-preview"));
        assert!(!EDITOR_SPRITE_JS.contains("finishSpritePaintMutation();"));
    }

    #[test]
    fn solid_sprite_source_loads_without_fabricated_editor_grid() {
        assert!(!EDITOR_SPRITE_JS.contains("SOLID_SPRITE_EDITOR_SIZE"));
        assert!(EDITOR_SPRITE_JS.contains("const size = Number.isFinite(parsed) ? parsed : 5;"));
        assert!(!EDITOR_SPRITE_JS.contains("Math.trunc(Number(value) || 5)"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("solid: width === 1 && height === 1 && parsedFrames[0][0] === 0,")
        );
        assert!(!EDITOR_SPRITE_JS.contains("const size = 5;"));
    }

    #[test]
    fn sprite_marker_preserves_paint_material_and_fill_owns_fill_mode() {
        assert!(EDITOR_SPRITE3D_JS.contains("function selectSprite3dBrushSize(size)"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const wasBucketActive = spriteBucketActive;\n  const wasClipActive = spriteClipActive || spriteClipSelection;\n  spriteBrushSizePx = normalizeSpriteBrushSize(size);\n  spriteBucketActive = false;\n  deactivateSpriteClipMode({ render: false });"
        ));
        assert!(
            EDITOR_SPRITE_JS.contains(
                "if (wasBucketActive || wasClipActive) {\n    renderSpritePalette();\n  }"
            )
        );
        assert!(EDITOR_SPRITE_JS.contains(
            "spriteBucketActive = !spriteBucketActive;\n  syncSpritePaintToolControls();\n  renderSpritePalette();"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "if (!validSpriteColorIndex(sprite.selectedColorIndex)) {\n    sprite.selectedColorIndex = validSpriteColorIndex(spriteLastPaintColorIndex) ? spriteLastPaintColorIndex : 0;\n  }"
        ));
    }

    #[test]
    fn sprite_marker_uses_compact_numeric_input() {
        assert!(EDITOR_CSS.contains(".sprite-brush-size-input {\n  width: 28px;"));
        assert!(
            EDITOR_CSS.contains("border: 0;\n  border-radius: 4px;\n  background: transparent;")
        );
        assert!(EDITOR_CSS.contains("font: 800 11px/24px ui-monospace"));
        assert!(EDITOR_CSS.contains(".sprite-brush-size-input:hover,\n.sprite-brush-size-input:focus {\n  background: var(--input-bg);"));
        assert!(EDITOR_CSS.contains(".sprite-marker-icon {\n  width: 20px;"));
        assert!(EDITOR_JS.contains("|| element.classList.contains(\"sprite-brush-size-input\");"));
    }

    #[test]
    fn sprite_palette_owns_marker_and_toolbar_orders_scope_grid_clip() {
        assert!(EDITOR_HTML.contains(r#"id="spriteTransformActionBank" hidden"#));
        assert!(!EDITOR_HTML.contains("sprite-toolbar sprite-edit-actions"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("paletteGrid.append(leadingControl);\n  }\n  const eraseButton")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("target: spritePalette,\n    leadingControl: spriteMarkerTool,")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("target: sprite3dPalette,\n    leadingControl: spriteMarkerTool,")
        );
        assert!(EDITOR_SPRITE_JS.contains("{ key: \"scope\", group: \"context\" },\n  { key: \"grid\", group: \"context\" },\n  { key: \"clip\", group: \"context\" },"));
        assert!(!EDITOR_SPRITE_JS.contains("{ key: \"marker\", group: \"context\" }"));
        assert!(EDITOR_SPRITE_JS.contains(
            "{ key: \"fill\", group: \"paint\" },\n  { key: \"translate\", group: \"paint\" },"
        ));
        assert!(EDITOR_SPRITE_JS.contains("{ key: \"flip-vertical\", group: \"transform\" },\n  { key: \"copy\", group: \"clipboard\" },"));
        assert!(EDITOR_SPRITE_JS.contains("{ key: \"paste\", group: \"clipboard\" },\n  { key: \"delete\", group: \"clipboard\" },"));
        assert!(EDITOR_SPRITE_JS.contains(
            "\"flip-vertical\": is3d ? sprite3dFlipPlaneVerticalButton : spriteFlipVerticalButton,"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("sprite3dClearButton"));
        assert!(!EDITOR_SPRITE_JS.contains("spriteClearButton"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteEditCommandLabel(dimension, command)"));
        assert!(EDITOR_SPRITE3D_JS.contains("syncSpriteEditCommandLabels(\"3d\");"));
        assert!(EDITOR_CSS.contains(".sprite-editor-toolbar {\n  align-items: flex-start;\n  flex-direction: column;\n  flex-wrap: nowrap;\n  gap: 10px;"));
        assert!(EDITOR_CSS.contains(".sprite-toolbar-context-row {\n  gap: 10px;"));
        assert!(EDITOR_CSS.contains(".sprite-toolbar-operation-row {\n  gap: 12px;"));
        assert!(EDITOR_HTML.contains("lucide lucide-square-icon lucide-square"));
        assert!(EDITOR_HTML.contains("lucide lucide-box-icon lucide-box"));
        assert!(!EDITOR_HTML.contains("sprite3d-scope-toggle-label"));
        assert!(
            EDITOR_CSS
                .contains(".sprite-paint-tool-button {\n  border: 0;\n  background: transparent;")
        );
        assert!(!EDITOR_CSS.contains(".sprite-paint-tool-button {\n  background: var(--bar-bg);"));
    }

    #[test]
    fn sprite_2d_and_3d_share_toolbar_marker_grid_and_tag_ui() {
        assert!(EDITOR_HTML.contains(r#"data-sprite-dimension="2d""#));
        assert!(EDITOR_HTML.contains(r#"data-sprite-dimension="3d""#));
        assert!(EDITOR_HTML.contains(r#"id="spriteBrushSizeInput""#));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("spriteBrushDiameterForSize(Math.min(plane.width, plane.height))")
        );
        assert!(EDITOR_SPRITE_JS.contains("const SPRITE_EDITOR_TOOL_SCHEMA = Object.freeze(["));
        assert!(EDITOR_SPRITE_JS.contains("function spriteEditorToolbarParts(dimension)"));
        assert!(EDITOR_SPRITE_JS.contains("grid: spriteGridButton,"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("clip: is3d ? sprite3dClipActions : renderSpriteClipActions(),")
        );
        assert!(
            EDITOR_SPRITE_JS.contains("const groups = { context, paint, transform, clipboard };")
        );
        assert!(EDITOR_SPRITE_JS.contains("row.append(contextRow, operationRow);"));
        assert!(EDITOR_SPRITE_JS.contains("operationRow.append(paint, transform, clipboard);"));
        assert!(
            !EDITOR_SPRITE_JS
                .contains("global.querySelector(\".sprite3d-scope-toggle, .sprite-clip-actions\")")
        );
        assert!(EDITOR_SPRITE_JS.contains("function renderSpritePaletteGrid({"));
        assert!(EDITOR_SPRITE_JS.contains("renderSpritePaletteGrid({\n    target: spritePalette,"));
        assert!(
            EDITOR_SPRITE3D_JS.contains("renderSpritePaletteGrid({\n    target: sprite3dPalette,")
        );
        assert!(!EDITOR_SPRITE3D_JS.contains("const paletteGrid = document.createElement"));
        assert!(
            EDITOR_SPRITE_JS.contains("function renderSpriteShapeBindControl(target, options)")
        );
        assert!(EDITOR_SPRITE_JS.contains("renderSpriteShapeBindControl(spriteShapeField,"));
        assert!(EDITOR_SPRITE3D_JS.contains("renderSpriteShapeBindControl(sprite3dShapeField,"));
        assert!(
            EDITOR_SPRITE_JS.contains("function renderSpriteEditorUpperControls(target, controls)")
        );
        assert!(EDITOR_SPRITE_JS.contains("spriteEditorUpperControls2d(),"));
        assert!(EDITOR_SPRITE3D_JS.contains("spriteEditorUpperControls3d(),"));
        assert!(!EDITOR_SPRITE_JS.contains("controls.depthInput"));
        assert!(!EDITOR_SPRITE3D_JS.contains("document.createElement(\"label\")"));
        assert!(EDITOR_CSS.contains(".sprite-editor-name-row,\n.sprite-editor-geometry-group,"));
        assert!(EDITOR_CSS.contains(
            ".sprite-editor-upper-controls {\n  width: 100%;\n  min-width: 0;\n  display: flex;\n  flex-wrap: wrap;"
        ));
        assert!(
            EDITOR_CSS
                .contains(".sprite-editor-name-row {\n  flex: 0 1 330px;\n  flex-wrap: nowrap;")
        );
        assert!(
            EDITOR_CSS.contains(
                ".sprite-builder:not(.is-animation-mode) .sprite-editor-animation-group,"
            )
        );
        assert!(!EDITOR_CSS.contains(".sprite3d-animation-control"));
        assert!(EDITOR_SPRITE3D_JS.contains("renderSpriteCurrentColorTagButton({"));
        assert!(EDITOR_SPRITE3D_JS.contains("if (sprite3dGridVisible) {"));
        assert!(EDITOR_SPRITE3D_JS.contains("--sprite3d-voxel-grid-stroke"));
        assert!(
            EDITOR_CSS.contains(".sprite-duration-input {\n  min-height: var(--icon-button-size);")
        );
        assert!(EDITOR_CSS.contains(".sprite-controls .sprite-duration-input input {\n  min-height: calc(var(--icon-button-size) - 2px);"));
        let toolbar_2d = EDITOR_HTML
            .find(r#"id="spriteToolbarHost""#)
            .expect("2D toolbar host");
        let source_2d = EDITOR_HTML
            .find(r#"id="spriteSourceActionBank""#)
            .expect("2D source action bank");
        let toolbar_3d = EDITOR_HTML
            .find(r#"id="sprite3dToolbarHost""#)
            .expect("3D toolbar host");
        let source_3d = EDITOR_HTML
            .find(r#"id="sprite3dSourceActionBank""#)
            .expect("3D source action bank");
        let palette_3d = EDITOR_HTML
            .find(r#"id="sprite3dPalette""#)
            .expect("3D palette");
        let controls_3d = EDITOR_HTML[..palette_3d]
            .rfind(r#"<div class="sprite-controls">"#)
            .expect("3D sprite controls");
        let depth_state_3d = EDITOR_HTML
            .find(r#"id="sprite3dDepthInput" type="hidden""#)
            .expect("3D depth state");
        assert!(controls_3d < depth_state_3d);
        assert!(depth_state_3d < palette_3d);
        assert!(
            EDITOR_HTML[controls_3d..depth_state_3d]
                .ends_with("              </div>\n              <input ")
        );
        assert!(toolbar_2d < source_2d);
        assert!(toolbar_3d < source_3d);
        assert!(EDITOR_HTML.contains(
            r#"id="sprite3dUpdateButton" class="source-action-button sprite-update-source-button""#
        ));
        assert!(EDITOR_CSS.contains(".sprite-editor-source-actions .source-action-button {\n  width: var(--icon-button-size);"));
        assert!(EDITOR_CSS.contains(".sprite-controls .sprite-shape-name-input {"));
        assert!(EDITOR_CSS.contains("font: inherit;\n  font-size: 13px;\n  font-weight: 800;"));
        let board_3d = EDITOR_HTML
            .find(r#"id="sprite3dSliceBoard""#)
            .expect("3D sprite board");
        let preview_3d = EDITOR_HTML
            .find(r#"class="sprite3d-preview-wrap""#)
            .expect("3D preview");
        assert!(board_3d < preview_3d);
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="spriteAnimationFrameInput""#)
                .count(),
            1
        );
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="spriteAnimationFrameStrip""#)
                .count(),
            1
        );
        assert!(!EDITOR_HTML.contains(r#"id="sprite3dAnimationFrameInput""#));
        assert!(
            EDITOR_DOM_JS
                .contains("const sprite3dAnimationFrameInput = spriteAnimationFrameInput;")
        );
        assert!(EDITOR_SPRITE_JS.contains("previewColumn.insertBefore(toolbar, previewStage);"));
        assert!(
            EDITOR_CSS
                .contains(".sprite-builder:not(.is-animation-mode) .sprite-animation-toolbar,")
        );
    }

    #[test]
    fn sprite3d_editor_accepts_depth_one_and_animation_frames() {
        assert!(EDITOR_HTML.contains(r#"id="sprite3dDepthInput" type="hidden""#));
        assert!(
            EDITOR_SPRITE3D_JS.contains("return sprite3d.size * sprite3d.size * sprite3d.depth;")
        );
        assert!(EDITOR_SPRITE3D_JS.contains("if (width < 1 || depth < 1 || width !== height"));
        assert!(EDITOR_SPRITE3D_JS.contains("sprite3d.animationMode = loaded.frames.length > 1"));
        assert!(EDITOR_SPRITE3D_JS.contains("function setSprite3dAnimationFrame(index)"));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "durationMs: sprite3d.animationMode ? normalizedSprite3dAnimationDuration() : null"
        ));
    }

    #[test]
    fn source_editor_completes_rewrite_rhs_from_lhs_pattern() {
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceRewriteRhsPatternAssist(event)"));
        assert!(
            EDITOR_SOURCE_JS.contains("function sourceRewriteStatementBounds(code, cursorColumn)")
        );
        assert!(EDITOR_SOURCE_JS.contains("sourceRewritePatternBeforeArrow(lineBeforeArrow)"));
        assert!(EDITOR_SOURCE_JS.contains("function sourcePatternCellSeparator(char)"));
        assert!(EDITOR_SOURCE_JS.contains("return char === \"|\" || char === \";\";"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceEmptyRewritePattern(pattern)"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "const separators = Array.from(body).filter(sourcePatternCellSeparator);"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "const emptyBody = separators.map((separator) => separator === \"|\" ? \" | \" : \";\").join(\"\");"
        ));
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceRewritePatternTab(event)"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "const statementBounds = sourceRewriteStatementBounds(code, cursorColumn);"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("const statementEnd = statementBounds.end;"));
        assert!(EDITOR_SOURCE_JS.contains(
            "const lhsPattern = sourceRewritePatternBeforeArrow(statement.slice(0, arrow));"
        ));
        assert!(EDITOR_SOURCE_JS.contains("const rhsEnd = lineStart + statementBounds.end;"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.setRangeText(rhsPattern, cursor, cursor, \"end\")")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.setRangeText(lhsPattern, cursor, cursor, \"end\")")
        );
        assert!(EDITOR_SOURCE_JS.contains("if (handleSourceRewriteRhsPatternAssist(event))"));
    }

    #[test]
    fn source_editor_completes_rewrite_lhs_bracket_cell() {
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceRewriteLhsBracketAssist(event)"));
        assert!(EDITOR_SOURCE_JS.contains("function insertSourceRewritePatternCell(start, end)"));
        assert!(EDITOR_SOURCE_JS.contains("const replacement = `[ ${selection} ]`;"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.setSelectionRange(innerStart, innerEnd"));
        assert!(EDITOR_SOURCE_JS.contains("if (handleSourceRewriteLhsBracketAssist(event))"));
    }

    #[test]
    fn source_editor_tab_exits_rule_bracket_cell() {
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceRuleBracketCellSlotTab(event)"));
        assert!(EDITOR_SOURCE_JS.contains("sourcePatternCellSeparator(body[index])"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "const targetIndex = event.shiftKey\n    ? (currentIndex + slots.length - 1) % slots.length\n    : (currentIndex + 1) % slots.length;"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceRuleBracketCellTabExit(event)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const replacement = hasTrailingHorizontalSpace ? \"[  ]\" : \"[  ] \";")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.setRangeText(replacement, open, close + 1, \"end\")")
        );
        assert!(EDITOR_SOURCE_JS.contains("if (handleSourceRuleBracketCellTabExit(event))"));
        let bracket_tab_handler = EDITOR_SOURCE_JS
            .find("if (handleSourceRuleBracketCellTabExit(event))")
            .unwrap();
        let slot_tab_handler = EDITOR_SOURCE_JS
            .find("if (handleSourceRuleBracketCellSlotTab(event))")
            .unwrap();
        let rewrite_tab_handler = EDITOR_SOURCE_JS
            .find("if (handleSourceRewritePatternTab(event))")
            .unwrap();
        assert!(slot_tab_handler < bracket_tab_handler);
        assert!(bracket_tab_handler < rewrite_tab_handler);
    }

    #[test]
    fn source_completion_keyboard_commands_precede_codemirror_defaults() {
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionSelectedIndexForSession"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionSessionMatches"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionItemsMatch"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "sourceEditor.addEventListener(\"sourcecompletioncommand\", (event) => {"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("if (command === \"show\")"));
        assert!(EDITOR_SOURCE_JS.contains("if (command === \"next\" || command === \"previous\")"));
        assert!(EDITOR_SOURCE_JS.contains(
            "if (command === \"commit\" && sourceCompletionState.mode === \"completion\")"
        ));
        assert!(EDITOR_SOURCE_JS.contains("if (acceptSourceCompletion())"));
        assert!(EDITOR_SOURCE_JS.contains("if (event.key === \"Tab\")"));
        assert!(EDITOR_SOURCE_JS.contains("if (event.key === \"Enter\")"));
        assert!(EDITOR_SOURCE_JS.contains("if (sourceCompletionState.mode === \"completion\")"));
        assert!(!EDITOR_SOURCE_JS.contains("sourceCompletionCanKeyboardCommit"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("const sourceCompletionKeymap = ["));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains(
            "{ key: \"Tab\", run: (view) => dispatchSourceCompletionCommand(view, \"commit\") }"
        ));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains(
            "{ key: \"Enter\", run: (view) => dispatchSourceCompletionCommand(view, \"commit\") }"
        ));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains(
            "keymap.of([...sourceCompletionKeymap, indentWithTab, ...defaultKeymap, ...historyKeymap])"
        ));
        assert!(EDITOR_CODEMIRROR_JS.contains("sourcecompletioncommand"));
    }

    #[test]
    fn source_editing_commands_precede_codemirror_defaults_without_owning_policy() {
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("const sourceEditingKeymap = ["));
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS
                .contains("dispatchSourceEditingCommand(view, \"open-brace\")")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS
                .contains("dispatchSourceEditingCommand(view, \"open-bracket\")")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS
                .contains("dispatchSourceEditingCommand(view, \"shift-tab\")")
        );
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains(
            "...sourceCompletionKeymap,\n        ...sourceEditingKeymap,\n        indentWithTab,"
        ));
        assert!(!EDITOR_CODEMIRROR_SOURCE_JS.contains("handleSourceRuleBracketCell"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.addEventListener(\"sourceeditingcommand\", (event) => {")
        );
        assert!(EDITOR_SOURCE_JS.contains("handleSourceBraceAssist(keyEvent)"));
        assert!(EDITOR_SOURCE_JS.contains("handleSourceRewriteLhsBracketAssist(keyEvent)"));
        assert!(EDITOR_SOURCE_JS.contains("handleSourceRuleBracketCellSlotTab(keyEvent)"));
        assert!(EDITOR_SOURCE_JS.contains("handleSourceRewritePatternTab(keyEvent)"));
    }

    #[test]
    fn source_find_supports_command_g_match_navigation() {
        assert!(EDITOR_SOURCE_JS.contains("function sourceFindMoveShortcutRequested(event)"));
        assert!(EDITOR_SOURCE_JS.contains("!event.metaKey || event.ctrlKey || event.altKey"));
        assert!(EDITOR_SOURCE_JS.contains("return key === \"g\" || event.code === \"KeyG\";"));
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceFindMoveShortcut(event)"));
        assert!(EDITOR_SOURCE_JS.contains("moveSourceFindSelection(event.shiftKey ? -1 : 1);"));
        assert!(EDITOR_SOURCE_JS.contains("if (handleSourceFindMoveShortcut(event))"));
    }

    #[test]
    fn source_find_uses_codemirror_decorations_and_reserves_panel_space() {
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("const sourceFindDecorations = StateField.define")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS
                .contains("applyFindMatches(source, matches, selectedIndex)")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains(
                "cm-source-find-match${index === selectedIndex ? \" is-current\" : \"\"}"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("function syncSourceFindPanelLayout()"));
        assert!(EDITOR_SOURCE_JS.contains("--source-find-panel-space"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.sourceEditorPort.applyFindMatches("));
        assert!(EDITOR_SOURCE_JS.contains("scrollSourceOffsetIntoView(match.start, \"start\")"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("scrollPastEnd(),"));
        assert!(
            EDITOR_CSS.contains(".source-editor-wrap.has-source-find-panel .source-editor-mount")
        );
        assert!(EDITOR_CSS.contains(".cm-source-find-match.is-current"));
    }

    #[test]
    fn source_save_reload_preserves_undo_for_same_active_document() {
        assert!(EDITOR_SOURCE_JS.contains("preserveUndoOnSameValue"));
        assert!(EDITOR_SOURCE_JS.contains("ensureSourceUndoHistory();"));
        assert!(EDITOR_WORKSPACE_JS.contains("const previousActiveFileId = activeFileId;"));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("preserveUndoOnSameValue: document.id === previousActiveFileId")
        );
    }

    #[test]
    fn source_undo_redo_reveals_restored_selection() {
        assert!(EDITOR_SOURCE_JS.contains("function restoreSourceEditorSnapshot(snapshot)"));
        assert!(EDITOR_SOURCE_JS.contains("scrollSourceOffsetIntoView(start);"));
        let selection_restore = EDITOR_SOURCE_JS
            .find("sourceEditor.setSelectionRange(start, end, snapshot.selectionDirection || \"none\");")
            .expect("source snapshot selection restore");
        let reveal = EDITOR_SOURCE_JS
            .find("scrollSourceOffsetIntoView(start);")
            .expect("source snapshot reveal");
        assert!(selection_restore < reveal);
    }

    #[test]
    fn source_location_reveal_uses_editor_geometry_for_wrapped_lines() {
        let reveal_start = EDITOR_JS
            .find("function revealSourceLocation(target, options = {})")
            .expect("source location reveal");
        let reveal_end = EDITOR_JS[reveal_start..]
            .find("function currentLevelSourceLocation()")
            .map(|index| reveal_start + index)
            .expect("source location reveal end");
        let reveal = &EDITOR_JS[reveal_start..reveal_end];
        assert!(reveal.contains("scrollSourceOffsetIntoView(start, options.scrollAlignment);"));
        assert!(!reveal.contains("lineIndex * lineHeight"));
        assert!(!EDITOR_JS.contains("function scrollSourceEditorToPosition("));
    }

    #[test]
    fn preview_error_source_reveal_centers_the_target_line() {
        assert!(EDITOR_JS.contains("{ recordHistory: true, scrollAlignment: \"center\" },"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("function scrollSourceOffsetIntoView(offset, alignment = \"nearest\")")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("scrollIntoView(offset, alignment = \"nearest\")")
        );
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("{ y: alignment, x: \"nearest\" }"));
    }

    #[test]
    fn source_completion_auto_requires_typed_prefix() {
        assert!(
            EDITOR_SOURCE_JS.contains("function sourceCursorHasCompletionPrefix(source, cursor)")
        );
        assert!(
            EDITOR_SOURCE_JS.contains(
                "return sourceCursorHasCompletionPrefix(source, cursor)\n    || sourceCursorAfterSelectorTagSeparator(source, cursor);"
            )
        );
        assert!(!EDITOR_SOURCE_JS.contains("function sourceCursorAtBareLineTail"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("if (!options.manual && !sourceAutoCompletionEligible(source, cursor))")
        );
    }

    #[test]
    fn source_completion_auto_allows_selector_tag_separator() {
        assert!(
            EDITOR_SOURCE_JS
                .contains("function sourceCursorAfterSelectorTagSeparator(source, cursor)")
        );
        assert!(
            EDITOR_SOURCE_JS.contains(
                "return sourceCursorHasCompletionPrefix(source, cursor)\n    || sourceCursorAfterSelectorTagSeparator(source, cursor);"
            )
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("/(?:^|[^\\w@.-])[@A-Za-z_][\\w@.-]*(?::[@A-Za-z_][\\w@.-]*)*:$/.test")
        );
    }

    #[test]
    fn source_completion_filters_full_replacement_token() {
        assert!(EDITOR_SOURCE_JS.contains(
            "const items = filterSourceCompletionsForTypedReplacement(\n      filterSourceCompletionsForDocument(list?.items || [], document),\n      list,\n      source,\n      cursor,\n    );"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "function filterSourceCompletionsForTypedReplacement(items, list, source, cursor)"
        ));
        assert!(
            EDITOR_SOURCE_JS.contains("const current = source.slice(replaceStart, replaceEnd);")
        );
        assert!(!EDITOR_SOURCE_JS.contains("function filterSourceCompletionsForCurrentRange"));
        assert!(
            EDITOR_SOURCE_JS.contains("full token under the caret, not just the prefix before it")
        );
    }

    #[test]
    fn source_completion_uses_wasm_entrypoint_only() {
        assert!(
            EDITOR_SOURCE_JS
                .contains("const list = await suggestSourceCompletionsWithWasm(source, cursor);")
        );
        assert!(!EDITOR_SOURCE_JS.contains("suggestSourceCompletionsFromEditorContext"));
        assert!(!EDITOR_SOURCE_JS.contains("function sourceImportPathCompletionContext"));
        assert!(!EDITOR_SOURCE_JS.contains("function sourceImportPathCompletionItems"));
    }

    #[test]
    fn source_completion_popover_stays_within_editor_and_flips_above_caret() {
        assert!(
            EDITOR_SOURCE_JS.contains("const wrapRect = sourceEditorWrap.getBoundingClientRect();")
        );
        assert!(EDITOR_SOURCE_JS.contains("const anchorRect = sourceCaretRectForOffset(anchor);"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "const cursorRect = sourceCaretRectForOffset(sourceEditor.selectionStart);"
            )
        );
        assert!(EDITOR_SOURCE_JS.contains("const left = wrapRect.left + anchorRect.left;"));
        assert!(EDITOR_SOURCE_JS.contains("const viewportBottom = Math.min(window.innerHeight - margin, wrapRect.bottom - margin);"));
        assert!(
            EDITOR_SOURCE_JS.contains(
                "const availableBelow = Math.max(0, viewportBottom - caretBottom - gap);"
            )
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("const availableAbove = Math.max(0, caretTop - viewportTop - gap);")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "sourceCompletionPopover.dataset.placement = placeBelow ? \"below\" : \"above\";"
        ));
        let completion_position = EDITOR_SOURCE_JS
            .find("function positionSourceCompletionPopover()")
            .expect("source completion popover positioning function");
        let completion_position_end = EDITOR_SOURCE_JS[completion_position..]
            .find("\n}")
            .map(|offset| completion_position + offset)
            .expect("source completion popover positioning function end");
        let block = &EDITOR_SOURCE_JS[completion_position..completion_position_end];
        assert!(!block.contains("sourceEditor.getBoundingClientRect()"));
    }

    #[test]
    fn source_editor_backspace_removes_one_indent_unit() {
        assert!(EDITOR_SOURCE_JS.contains("function handleSourceIndentBackspace(event)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const targetColumn = Math.max(0, column - (column % 4 || 4));")
        );
        assert!(EDITOR_SOURCE_JS.contains("if (handleSourceIndentBackspace(event))"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.setRangeText(\"\", removeStart, start, \"start\")")
        );
    }

    #[test]
    fn source_editor_enter_inside_braces_keeps_cursor_on_inner_line() {
        assert!(EDITOR_SOURCE_JS.contains("function insertSourceNewlineAtSelection()"));
        assert!(
            EDITOR_SOURCE_JS.contains("const cursorOffset = sourceNewlineCursorOffset(insert);")
        );
        assert!(EDITOR_SOURCE_JS.contains("const cursor = start + cursorOffset;"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.setSelectionRange(cursor, cursor);"));
        assert!(EDITOR_SOURCE_JS.contains(
            "return firstNewline >= 0 && lastNewline > firstNewline ? lastNewline : null;"
        ));
    }

    #[test]
    fn source_editor_clicks_right_of_text_stay_on_visual_line_end() {
        assert!(EDITOR_SOURCE_JS.contains("const textNodes = [];"));
        assert!(EDITOR_SOURCE_JS.contains("range.selectNodeContents(node);"));
        assert!(EDITOR_SOURCE_JS.contains("entry.lineDistance > nearestLineDistance + 0.5"));
        assert!(EDITOR_SOURCE_JS.contains("let lineHit = null;"));
        assert!(EDITOR_SOURCE_JS.contains("let bestInLine = null;"));
        assert!(EDITOR_SOURCE_JS.contains("if (lineDistance === 0 && char !== \"\\n\")"));
        assert!(EDITOR_SOURCE_JS.contains("if (clientX >= lineHit.right)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("return Math.max(0, Math.min(source.length, lineHit.endOffset));")
        );
        assert!(EDITOR_SOURCE_JS.contains("function sourceInteractionFromPointer(event"));
        assert!(EDITOR_SOURCE_JS.contains("visualOffset: viewOffset,"));
    }

    #[test]
    fn source_line_gutter_does_not_capture_text_selection_drag() {
        assert!(EDITOR_CSS.contains(
            ".source-line-numbers {\n  box-sizing: border-box;\n  position: absolute;\n  top: 0;\n  left: 0;\n  z-index: 2;"
        ));
        assert!(EDITOR_CSS.contains("  pointer-events: none;\n  will-change: transform;"));
        assert!(
            EDITOR_CSS.contains(".source-fold-button {\n  padding: 0;\n  display: inline-grid;")
        );
        assert!(EDITOR_CSS.contains("  cursor: pointer;\n  pointer-events: auto;"));
        assert!(EDITOR_CSS.contains(
            ".source-editor-wrap.is-source-selection-dragging .source-fold-button {\n  pointer-events: none;\n}"
        ));
        assert!(EDITOR_SOURCE_JS.contains("function beginSourceNativeSelectionDrag(event)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditorWrap?.classList.add(\"is-source-selection-dragging\");")
        );
        assert!(EDITOR_SOURCE_JS.contains("function endSourceNativeSelectionDrag()"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditorWrap?.classList.remove(\"is-source-selection-dragging\");")
        );
        assert!(
            EDITOR_SOURCE_JS.contains(
                "document.addEventListener(\"pointerup\", endSourceNativeSelectionDrag);"
            )
        );
    }

    #[test]
    fn source_editor_folded_selection_uses_view_offsets_for_caret_motion() {
        assert!(
            EDITOR_SOURCE_JS.contains("function sourceViewOffsetFromVisualPoint(clientX, clientY)")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "function sourceOffsetFromVisualPointer(event, source = sourceEditorDocumentValue())"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "function sourceOffsetFromVisualPoint(clientX, clientY, source = sourceEditorDocumentValue())"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "const documentOffset = sourceFoldsActive()\n    ? sourceViewOffsetToDocumentOffset(offset, \"start\")\n    : offset;"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "const next = sourceViewOffsetFromVisualPoint(targetClientX, targetClientY);"
        ));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const visualOffset = Number.isInteger(options.visualOffset)")
        );
        assert!(EDITOR_SOURCE_JS.contains(": sourceViewOffsetFromVisualPoint(clientX, clientY);"));
    }

    #[test]
    fn source_pointer_sync_uses_visual_click_offset() {
        assert!(EDITOR_SOURCE_JS.contains("function sourceInteractionFromPointer(event"));
        assert!(EDITOR_SOURCE_JS.contains(
            "const viewOffset = sourceViewOffsetFromVisualPoint(event.clientX, event.clientY);"
        ));
        assert!(EDITOR_SOURCE_JS.contains("position: interaction.documentOffset,"));
        assert!(!EDITOR_JS.contains("function syncPreviewModeFromSourcePointer(event)"));
    }

    #[test]
    fn source_editor_uses_codemirror_scroll_port() {
        assert!(EDITOR_CSS.contains(".source-editor-wrap {\n  min-width: 0;\n  min-height: 0;\n  position: relative;\n  overflow: hidden;"));
        assert!(EDITOR_CSS.contains(".source-editor-mount .cm-scroller"));
        assert!(EDITOR_CSS.contains(".source-editor-mount .cm-content#sourceEditor"));
        assert!(EDITOR_SOURCE_JS.contains("return sourceEditor.sourceEditorPort.scrollTop();"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.sourceEditorPort.scrollTop(value);"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("function syncSourceOverlayLayerMetrics(clientWidth, scrollHeight)")
        );
        assert!(EDITOR_SOURCE_JS.contains("return sourceEditorWrap.scrollTop || 0;"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditorWrap.scrollTop = Math.max(0, value || 0);"));
        assert!(EDITOR_SOURCE_JS.contains("sourceHighlight.style.transform = \"\";"));
        assert!(EDITOR_SOURCE_JS.contains("sourceBlockSelectionLayer.style.transform = \"\";"));
        assert!(EDITOR_SOURCE_JS.contains("sourceFindMatchLayer.style.transform = \"\";"));
        assert!(
            EDITOR_SOURCE_JS.contains("caret.style.left = `${rect.left + sourceScrollLeft()}px`;")
        );
        assert!(
            EDITOR_SOURCE_JS.contains("caret.style.top = `${rect.top + sourceScrollTop()}px`;")
        );
        assert!(EDITOR_SOURCE_JS.contains("left: rect.left - wrapRect.left + sourceScrollLeft(),"));
        assert!(EDITOR_SOURCE_JS.contains("top: rect.top - wrapRect.top + sourceScrollTop()"));
        assert!(
            !EDITOR_SOURCE_JS
                .contains("sourceEditor.addEventListener(\"scroll\", syncSourceHighlightScroll);")
        );
        assert!(
            !EDITOR_SOURCE_JS.contains(
                "sourceEditor.addEventListener(\"scroll\", syncSourceEditorScrollCursor);"
            )
        );
    }

    #[test]
    fn source_block_selection_uses_normal_selection_fill() {
        assert!(EDITOR_CSS.contains(
            "--source-selection-bg: color-mix(in srgb, var(--accent) 34%, transparent);"
        ));
        assert!(EDITOR_CSS.contains(".source-block-selection-range {\n  position: absolute;\n  min-width: 2px;\n  background: var(--source-selection-bg);\n}"));
        assert!(EDITOR_CSS.contains(
            ".source-editor-mount > .cm-editor.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground {\n  background: var(--source-selection-bg);\n}"
        ));
        assert!(!EDITOR_CSS.contains(".source-editor-mount .cm-content ::selection"));
    }

    #[test]
    fn source_editor_keeps_codemirror_caret_visible() {
        let start = EDITOR_CSS
            .find(".source-editor-mount .cm-content#sourceEditor {")
            .expect("source editor CSS block");
        let end = EDITOR_CSS[start..]
            .find("\n}")
            .map(|offset| start + offset)
            .expect("source editor CSS block end");
        let block = &EDITOR_CSS[start..end];
        assert!(block.contains("color: var(--code-ink);"));
        assert!(block.contains("caret-color: var(--code-ink);"));
        assert!(!block.contains("color: transparent;"));
    }

    #[test]
    fn level_and_solver_boards_use_preview_theme_colors() {
        let level_start = EDITOR_CSS
            .find(".level-board-wrap {")
            .expect("level board wrap CSS block");
        let level_end = EDITOR_CSS[level_start..]
            .find("\n}")
            .map(|offset| level_start + offset)
            .expect("level board wrap CSS block end");
        let level_block = &EDITOR_CSS[level_start..level_end];
        assert!(level_block.contains("border: 1px solid var(--preview-game-line);"));
        assert!(level_block.contains("background-color: var(--sprite-swatch-bg);"));
        assert!(level_block.contains("background-image: var(--sprite-swatch-checker);"));
        assert!(level_block.contains("color: var(--preview-game-ink);"));
        assert!(!level_block.contains("background: var(--bg);"));

        let solver_start = EDITOR_CSS
            .find(".solver-board-wrap {")
            .expect("solver board wrap CSS block");
        let solver_end = EDITOR_CSS[solver_start..]
            .find("\n}")
            .map(|offset| solver_start + offset)
            .expect("solver board wrap CSS block end");
        let solver_block = &EDITOR_CSS[solver_start..solver_end];
        assert!(solver_block.contains("border: 1px solid var(--preview-game-line);"));
        assert!(solver_block.contains("background: var(--preview-game-background);"));
        assert!(solver_block.contains("color: var(--preview-game-ink);"));
        assert!(!solver_block.contains("background: var(--bg);"));
    }

    #[test]
    fn editor_preview_theme_resolver_accepts_canonical_theme_variables() {
        assert!(EDITOR_JS.contains("accent: \"var(--preview-game-ink)\","));
        assert!(EDITOR_JS.contains(
            "root.style.setProperty(\"--preview-game-accent\", theme.accent || theme.ink);"
        ));
        assert!(
            EDITOR_CSS.contains(
                "--accent: var(--preview-game-accent, var(--preview-game-ink, #1f2428));"
            )
        );
        assert!(EDITOR_JS.contains("if (name === \"bg\" || name === \"background\") {"));
        assert!(EDITOR_JS.contains("} else if (name === \"ink\" || name === \"text\") {"));
        assert!(
            EDITOR_JS.contains(
                "} else if (name === \"accent\") {\n      resolved.accent = value;\n      resolved.line = value;"
            )
        );
    }

    #[test]
    fn editor_workspace_includes_named_theme_css_in_effective_game_css() {
        assert!(EDITOR_WORKSPACE_JS.contains(
            "for (const themeDocument of effectiveThemeCssDocuments(document, effectiveThemeName(document)))"
        ));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("parts.push(rewriteCssAssetUrls(\n      themeDocument.source || \"\",")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("activeTheme = trimmed.endsWith(\"{\");"));
    }

    #[test]
    fn source_color_highlight_is_not_bold() {
        let start = EDITOR_CSS
            .find(".syntax-color {")
            .expect("syntax color CSS block");
        let end = EDITOR_CSS[start..]
            .find("\n}")
            .map(|offset| start + offset)
            .expect("syntax color CSS block end");
        let block = &EDITOR_CSS[start..end];
        assert!(!block.contains("font-weight"));
        assert!(block.contains("text-decoration-skip-ink: none;"));
    }

    #[test]
    fn source_highlight_has_no_tag_slot_color_surface() {
        assert!(!EDITOR_CSS.contains("--syntax-tag-"));
        assert!(!EDITOR_CSS.contains(".syntax-tag-"));
    }

    #[test]
    fn sprite_source_loader_consumes_lang_sprite_contract_instead_of_source_parsing() {
        assert!(!EDITOR_SPRITE_JS.contains("`${tableName}:*`"));
        assert!(!EDITOR_SPRITE_JS.contains(":*"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains(
            "state.sourceSpriteContract = target?.sourceSprite && typeof target.sourceSprite === \"object\""
        ));
        assert!(EDITOR_SPRITE_JS.contains("function spriteSourceColorAssets()"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteSourceShapeAssets()"));
        assert!(EDITOR_SPRITE_JS.contains("Array.isArray(contract?.colorAssets)"));
        assert!(EDITOR_SPRITE_JS.contains("Array.isArray(contract?.shapeAssets)"));
        assert!(EDITOR_SPRITE_JS.contains("Array.isArray(contract?.resolvedPalette)"));
        assert!(EDITOR_SPRITE_JS.contains("Array.isArray(contract?.resolvedShapeRows)"));
        for forbidden in [
            "parseSpriteColorAssets",
            "parseSpriteShapeAssets",
            "resolveSpriteColorAssetToken",
            "resolveSpriteShapeAssetToken",
            "spritePaletteEntryFromSourceToken",
            "parseSpriteValueMaps",
            "collectSpriteShapeTableRows",
            "collectSpriteShapeRotationBlocks",
            "parseSpriteShapeRotationDirective",
            "expandSpriteShapeRotationRows",
            "spriteTableAssetKey",
            "firstSpriteTableAssetKey",
            "spriteSelectorSingleTagBinding",
        ] {
            assert!(
                !EDITOR_SPRITE_JS.contains(forbidden),
                "{forbidden} should not exist in editor sprite source loading"
            );
        }
    }

    #[test]
    fn sprite_color_default_names_use_object_base_without_tag_or_color_suffix() {
        assert!(EDITOR_SPRITE_JS.contains("if (kind === \"color\") {"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("const objectName = String(spriteObjectName()).split(\":\")[0];")
        );
        assert!(EDITOR_SPRITE_JS.contains("return `${base}_${Number(index) + 1}`;"));
        assert!(EDITOR_SPRITE_JS.contains("return `${base}_${kind}_${Number(index) + 1}`;"));
    }

    #[test]
    fn sprite_source_generation_does_not_add_indents() {
        assert!(EDITOR_SPRITE_JS.contains("const SPRITE_SOURCE_INDENT = \"\";"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteSourceChildIndent(indent = \"\")"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("spriteObjectDefinitionText(spriteSourceIndent(entry.indent))")
        );
        assert!(!EDITOR_SPRITE_JS.contains("spriteObjectDefinitionText(\"\\t\")"));
        assert!(!EDITOR_SPRITE_JS.contains("const rowIndent = `${indent}\\t`;"));

        assert!(EDITOR_SPRITE3D_JS.contains("const SPRITE3D_SOURCE_INDENT = \"\";"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function sprite3dSourceChildIndent"));
        assert!(!EDITOR_SPRITE3D_JS.contains("replaceSprite3dDefinition"));
    }

    #[test]
    fn sprite_source_loader_accepts_bare_shape_refs() {
        assert!(EDITOR_SPRITE_JS.contains(
            "const loaded = parseSpriteDefinitionSource(target.sourceSprite, targetName);"
        ));
        assert!(
            EDITOR_SPRITE_DOCUMENT_JS
                .contains("const resolvedPalette = Array.isArray(contract.resolvedPalette)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("const shapeRows = Array.isArray(contract.resolvedShapeRows)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("shapeBind = { type: \"shape\", name: shapeName, linked: true };")
        );
    }

    #[test]
    fn sprite_shape_registration_rejects_empty_shape_rows() {
        assert!(EDITOR_SPRITE_JS.contains("function spriteShapeDefinitionRows(rows)"));
        assert!(EDITOR_SPRITE_JS.contains("const shapeRows = spriteShapeDefinitionRows(rows);"));
        assert!(EDITOR_SPRITE_JS.contains("/[0-9A-Za-z]/.test(row)"));
        assert!(EDITOR_SPRITE_JS.contains("Draw shape pixels before registering shape"));
        assert!(EDITOR_SPRITE_JS.contains("Draw shape pixels before updating shape"));
    }

    #[test]
    fn sprite_shape_sync_stages_missing_plain_shape_names() {
        assert!(EDITOR_SPRITE_JS.contains("status = `Tagged shape ${name}`;"));
        assert!(EDITOR_SPRITE_JS.contains("function sanitizeSpriteShapeRef(value)"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("return isSpriteShapeTableRef(parts[0], parts[1]) ? raw : \"\";")
        );
        assert!(EDITOR_SPRITE_JS.contains("function isSpritePlainShapeName(value)"));
        assert!(EDITOR_SPRITE_JS.contains("shape:tag"));
        assert!(EDITOR_SPRITE_JS.contains("const tableSeparator = name.indexOf(\":\");"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const withShape = ensureSpriteShapeDefinition(nextSource, shape.name, spriteAscii().split(\"\\n\"));"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "if (shape.linked && shape.name && !findSpriteShapeDefinitionRange(nextSource, shape.name))"
        ));
    }

    #[test]
    fn sprite_shape_registration_uses_unbraced_plain_shapes() {
        assert!(
            EDITOR_SPRITE_JS
                .contains("function spritePlainShapeDefinitionText(indent, name, rows)")
        );
        assert!(
            EDITOR_SPRITE_JS.contains("spritePlainShapeDefinitionText(indent, name, shapeRows)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("spritePlainShapeDefinitionText(shapeIndent, name, shapeRows)")
        );
        assert!(EDITOR_SPRITE_JS.contains("range.braced && !range.tableRow"));
        assert!(!EDITOR_SPRITE_JS.contains("${name} {\\n${shapeRows.map"));
    }

    #[test]
    fn sprite_shape_update_preserves_following_shape_header_boundary() {
        assert!(EDITOR_SPRITE_JS.contains("function spriteUnbracedShapeRowIsBoundary("));
        assert!(EDITOR_SPRITE_JS.contains("row.includes(\"{\") || row.includes(\"}\")"));
        assert!(EDITOR_SPRITE_JS.contains("spriteAsciiRowWidth(next) !== width"));
        assert!(EDITOR_SPRITE_JS.contains("function spritePlainShapeDefinitionTrailingBoundary("));
        assert!(EDITOR_SPRITE_JS.contains(
            "const boundary = spritePlainShapeDefinitionTrailingBoundary(source, range.declarationEnd);"
        ));
    }

    #[test]
    fn sprite_source_loader_projects_generic_refs_from_lang_contract() {
        assert!(EDITOR_SPRITE_JS.contains(
            "for (const entry of Array.isArray(contract?.resolvedPalette) ? contract.resolvedPalette : [])"
        ));
        assert!(EDITOR_SPRITE_JS.contains("if (entry?.linked && name && color)"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const source = String(entry?.source || paletteTokens[index] || \"\").trim();"
        ));
        assert!(
            EDITOR_SPRITE_JS
                .contains("paletteEntry.bind = { type: \"color\", name: source, linked: true };")
        );
    }

    #[test]
    fn sprite_source_color_staging_uses_palette_block() {
        assert!(EDITOR_SPRITE_JS.contains(
            "const paletteBlock = findVisualAssetBlock(source, spritesBlock, \"palette\");"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "const paletteText = `\\n${blockIndent}palette {\\n${rowIndent}${name} = ${normalized}\\n${blockIndent}}\\n`;"
        ));
        assert!(
            !EDITOR_SPRITE_JS.contains("findVisualAssetBlock(source, spritesBlock, \"colors\")")
        );
        assert!(!EDITOR_SPRITE_JS.contains("${blockIndent}colors {"));
    }

    #[test]
    fn sprite_color_tag_picker_shows_color_values() {
        assert!(EDITOR_SPRITE_JS.contains("const colorAssets = spriteSourceColorAssets();"));
        assert!(
            EDITOR_SPRITE_JS.contains("optionMeta: (name) => ({ color: colorAssets.get(name) })")
        );
        assert!(EDITOR_SPRITE_JS.contains("className = \"sprite-tag-option-swatch\""));
        assert!(EDITOR_SPRITE_JS.contains("className = \"sprite-tag-option-value\""));
        assert!(EDITOR_CSS.contains(".sprite-tag-option.has-color"));
        assert!(EDITOR_CSS.contains(".sprite-tag-option.has-invalid-color"));
        assert!(EDITOR_CSS.contains(".sprite-tag-option-swatch"));
        assert!(EDITOR_CSS.contains(".sprite-tag-option-value"));
    }

    #[test]
    fn sprite_source_loader_reads_resolved_shape_rows_from_lang_contract() {
        assert!(EDITOR_SPRITE_JS.contains("const shapes = spriteSourceShapeAssets();"));
        assert!(EDITOR_SPRITE_JS.contains("const rows = Array.isArray(entry?.rows)"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("const resolvedRows = Array.isArray(contract?.resolvedShapeRows)")
        );
        assert!(EDITOR_SPRITE_JS.contains("assets.set(shapeName, resolvedRows);"));
        let contract_error_start = EDITOR_SPRITE_JS
            .find("function spriteSourceContractError(contract)")
            .expect("sprite source contract validation");
        let contract_error_end = EDITOR_SPRITE_JS[contract_error_start..]
            .find("function spritePaletteEntrySourceToken")
            .map(|offset| contract_error_start + offset)
            .expect("sprite source contract validation end");
        let contract_error = &EDITOR_SPRITE_JS[contract_error_start..contract_error_end];
        assert!(contract_error.contains("if (shapeName && !shapeRows.length)"));
        assert!(contract_error.contains("return `Cannot resolve shape ${shapeName}`;"));

        let loader_start = EDITOR_SPRITE_JS
            .find("function loadSpriteSourceTarget(target, options = {})")
            .expect("sprite source target loader");
        let loader_end = EDITOR_SPRITE_JS[loader_start..]
            .find("function isIncompleteSpriteSourceTarget")
            .map(|offset| loader_start + offset)
            .expect("sprite source target loader end");
        let loader = &EDITOR_SPRITE_JS[loader_start..loader_end];
        assert!(
            loader
                .contains("const contractError = spriteSourceContractError(target.sourceSprite);")
        );
        assert!(loader.contains("setSpriteActionStatus(contractError, \"is-error\");"));
        assert!(loader.contains("setStatus(contractError, \"is-error\");"));
    }

    #[test]
    fn sprite_source_loader_preserves_sprite_prelude_rows() {
        assert!(EDITOR_JS.contains("sourcePreludeRows: [],"));
        assert!(
            EDITOR_JS.contains(
                "sourcePreludeRows: cloneVisualEditValue(sprite.sourcePreludeRows || []),"
            )
        );
        assert!(EDITOR_SPRITE_JS.contains(
            "const loaded = parseSpriteDefinitionSource(target.sourceSprite, targetName);"
        ));
        assert!(!EDITOR_SPRITE_JS.contains(
            "parseSpriteDefinitionSource(source.slice(target.bodyStart, target.bodyEnd)"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("function isSpriteSourcePreludeRow(row)"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("const sourcePreludeRows = Array.isArray(contract.preludeRows)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("const paletteTokens = Array.isArray(contract.paletteTokens)")
        );
        assert!(
            EDITOR_SPRITE_JS.contains("const shapeName = typeof contract.shapeRef === \"string\"")
        );
        assert!(EDITOR_SPRITE_JS.contains("sourcePreludeRows,"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const preludeRows = spriteSourcePreludeRows({ omitDuration: Boolean(animationSource) }).map((row) => `${rowIndent}${row}`);"
        ));
        assert!(EDITOR_SPRITE_JS.contains("...preludeRows,"));
    }

    #[test]
    fn sprite_source_loader_handles_animation_frames() {
        assert!(EDITOR_SPRITE_JS.contains(
            "const semanticFrames = Array.isArray(contract.frames) ? contract.frames : [];"
        ));
        assert!(EDITOR_SPRITE_JS.contains("animationMode: true,"));
        assert!(
            EDITOR_SPRITE_JS.contains(
                "const frameDurationMs = Number.isFinite(Number(contract.frameDurationMs))"
            )
        );
        assert!(EDITOR_SPRITE_JS.contains("frameDurationMs * parsedFrames.length"));
        assert!(EDITOR_SPRITE_JS.contains("animationDurationMs: durationMs,"));
        assert!(EDITOR_SPRITE_JS.contains("animationFrames: parsedFrames,"));
        assert!(
            EDITOR_SPRITE_JS.contains("const animationSource = spriteAnimationSourceFrames();")
        );
        assert!(EDITOR_SPRITE_JS.contains("lines.push(`${rowIndent}>`);"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("spriteSourcePreludeRows({ omitDuration: Boolean(animationSource) })")
        );
        assert!(EDITOR_SPRITE_JS.contains("function isSpriteTimingPreludeRow(row)"));
        assert!(EDITOR_SPRITE_JS.contains("duration|frame_duration"));
    }

    #[test]
    fn sprite_animation_settings_are_visual_undo_state() {
        assert!(EDITOR_JS.contains("animationDurationMs: sprite.animationDurationMs,"));
        assert!(EDITOR_JS.contains("animationFrameCount: sprite.animationFrameCount,"));
        assert!(
            EDITOR_JS
                .contains("animationFrames: cloneVisualEditValue(sprite.animationFrames || []),")
        );
        assert!(EDITOR_JS.contains(
            "sprite.animationDurationMs = Number.isFinite(Number(state.animationDurationMs))"
        ));
        assert!(EDITOR_SPRITE_JS.contains("const before = visualEditSnapshot(\"sprite\");\n  sprite.animationFrameCount = normalizedSpriteAnimationFrameCount(value);"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const nextDuration = normalizedSpriteAnimationDuration(value);\n  const changed = nextDuration !== sprite.animationDurationMs;"
        ));
        assert!(EDITOR_SPRITE_JS.contains(
            "const before = options.recordHistory === false || !changed ? null : visualEditSnapshot(\"sprite\");\n  sprite.animationDurationMs = nextDuration;"
        ));
        assert!(
            EDITOR_SPRITE_JS.contains(
                "if (before) {\n    pushVisualEditUndoSnapshot(\"sprite\", before);\n  }"
            )
        );
        assert!(EDITOR_SPRITE_JS.contains("function isSpriteVisualEditUndoTarget(target)"));
        assert!(EDITOR_SPRITE_JS.contains("function syncSpriteAnimationInputValues(options = {})"));
        assert!(EDITOR_JS.contains("syncSpriteAnimationInputValues();"));
        assert!(EDITOR_JS.contains("isSpriteVisualEditUndoTarget(target)"));
    }

    #[test]
    fn sprite_animation_playback_view_is_separate_from_frame_panel() {
        assert!(EDITOR_HTML.contains(r#"aria-label="Sprite animation frames""#));
        assert!(EDITOR_HTML.contains(r#"class="sprite-animation-sidecar""#));
        assert!(EDITOR_HTML.contains(r#"class="sprite-animation-playback-panel""#));
        assert!(EDITOR_HTML.contains("spriteAnimationPlaybackView"));
        assert!(EDITOR_HTML.contains("sprite-animation-playback-view-label"));
        assert!(EDITOR_CSS.contains(".sprite-animation-playback-panel {\n  position: relative;"));
        assert!(
            EDITOR_CSS.contains(".sprite-animation-playback-view-label {\n  position: absolute;")
        );
        let playback_panel = EDITOR_HTML
            .find(r#"class="sprite-animation-playback-panel""#)
            .expect("sprite animation playback panel");
        let frame_panel = EDITOR_HTML
            .find(r#"id="spriteAnimationPanel""#)
            .expect("sprite animation frame panel");
        assert!(playback_panel < frame_panel);
        assert!(EDITOR_CSS.contains(".sprite-animation-sidecar {\n  min-width: 72px;"));
        assert!(EDITOR_SPRITE_JS.contains("function renderSpriteAnimationPlaybackView(cells)"));
        assert!(EDITOR_SPRITE_JS.contains("function syncSpriteAnimationPlayback()"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteAnimationFrameDelayMs()"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("Math.round(context.durationMs() / context.state.animationFrameCount)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("spriteAnimationPlaybackDurationMs !== spriteAnimationFrameDelayMs()")
        );
        assert!(
            EDITOR_SPRITE_JS.contains("spriteAnimationDurationInput?.addEventListener(\"input\"")
        );
        assert!(EDITOR_SPRITE_JS.contains("recordHistory: false"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteAnimationFrameCells(cells)"));
        assert!(EDITOR_SPRITE_JS.contains("button.classList.toggle(\"is-playing-frame\""));
        assert!(!EDITOR_HTML.contains("spriteAnimationPlayButton"));
        assert!(!EDITOR_DOM_JS.contains("spriteAnimationPlayButton"));
        assert!(!EDITOR_SPRITE_JS.contains("toggleSpriteAnimationPlayback"));
        assert!(!EDITOR_HTML.contains("spriteAnimationCurrentPreview"));
        assert!(!EDITOR_HTML.contains("sprite-animation-preview-label"));
        assert!(!EDITOR_HTML.contains("spriteAnimationPlaybackPreview"));
        assert!(!EDITOR_CSS.contains(".sprite-animation-preview-box"));
        assert!(!EDITOR_CSS.contains(".sprite-animation-preview,"));
        assert!(!EDITOR_SPRITE_JS.contains("renderSpriteAnimationPreview"));
        assert!(!EDITOR_HTML.contains(r#"aria-label="Sprite animation playback and frames""#));
    }

    #[test]
    fn level3d_frame_surface_is_square_cornered() {
        assert!(EDITOR_CSS.contains(
            ".level3d-frame-surface {\n  position: absolute;\n  inset: 0 auto auto 0;\n  width: var(--level3d-frame-virtual-width);\n  height: var(--level3d-frame-virtual-height);\n  border: 0;\n  border-radius: 0;"
        ));
    }

    #[test]
    fn sprite_source_update_reveals_and_preserves_target_boundary() {
        assert!(EDITOR_JS.contains("editSourceName: \"\""));
        assert!(EDITOR_JS.contains("editSourceEnd: null"));
        assert!(EDITOR_JS.contains("editSourceBodyStart: null"));
        assert!(EDITOR_JS.contains("editSourceBodyEnd: null"));
        assert!(EDITOR_SPRITE_JS.contains("function revealSpriteSourceResult"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("async function commitSpriteEditorMutation"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("revealSpriteSourceResult(document, result);"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("sourceEditor.focus({ preventScroll: true });"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("function spriteEditorSourceRange"));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("state.editSourceEnd"));
        assert!(EDITOR_SPRITE_JS.contains("function currentSpriteEditSourceRange(source)"));
        assert!(EDITOR_SPRITE_JS.contains("commitSpriteEditorMutation({"));
        assert!(EDITOR_JS.contains(
            "const trailingBoundary = removed.match(/((?:\\r?\\n[\\t ]*)+)$/)?.[1] || \"\";"
        ));
    }

    #[test]
    fn shared_sprite_document_controller_loads_before_dimension_views() {
        let document = EDITOR_HTML
            .find(r#"<script src="editor_sprite_document.js"></script>"#)
            .expect("editor loads shared sprite document controller");
        let sprite2d = EDITOR_HTML
            .find(r#"<script src="editor_sprite.js"></script>"#)
            .expect("editor loads 2D sprite view");
        let sprite3d = EDITOR_HTML
            .find(r#"<script src="editor_sprite3d.js"#)
            .expect("editor loads 3D sprite view");
        assert!(document < sprite2d);
        assert!(document < sprite3d);
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("function projectSpriteDocumentContract"));
        assert!(!EDITOR_SPRITE_DOCUMENT_JS.contains("findMatchingBrace"));
    }

    #[test]
    fn sprite_source_edit_invalidates_cached_target_until_rust_resync() {
        assert!(EDITOR_SPRITE_JS.contains("function clearSpriteEditSource()"));
        assert!(EDITOR_SPRITE_JS.contains(
            "function invalidateSpriteEditSourceForDocument(document = activeDocument())"
        ));
        assert!(EDITOR_SPRITE_DOCUMENT_JS.contains("clearSpriteEditorSourceTarget(state);"));
        assert!(EDITOR_SPRITE_JS.contains(
            "sourceEditor.addEventListener(\"input\", () => {\n  invalidateSpriteEditSourceForDocument(activeDocument());\n  syncSpriteSourceActionButtons();\n});"
        ));
        assert!(EDITOR_SPRITE_JS.contains("function loadSpriteSourceTarget(target, options = {})"));
        assert!(EDITOR_SPRITE_JS.contains("setSpriteEditSource(target, activeDocument());"));
    }

    #[test]
    fn level_source_update_tracks_loaded_source_range() {
        assert!(EDITOR_JS.contains("editDocumentId: null"));
        assert!(EDITOR_JS.contains("editSourceStart: null"));
        assert!(
            EDITOR_JS.contains("function setLevelEditSource(entry, document = activeDocument())")
        );
        assert!(EDITOR_JS.contains(
            "function resetLevelBuilderFromSource(resetCells = true) {\n  clearLevelEditSource();"
        ));
        assert!(EDITOR_JS.contains("function currentLevelEditSourceRange(source)"));
        assert!(EDITOR_JS.contains("const editDocument = activeLevelEditDocument();"));
        assert!(EDITOR_JS.contains("const entry = currentLevelEditSourceRange(source);"));
        assert!(EDITOR_JS.contains(
            "const result = replaceLevelSourceEntry(source, entry, levelName, sourceData);"
        ));
        assert!(EDITOR_JS.contains("editDocument.source = result.source;"));
        assert!(EDITOR_JS.contains("setLevelEditSource({"));
        assert!(
            EDITOR_JS.contains("setLevelEditSource(entry, options.document || activeDocument());")
        );
        assert!(EDITOR_JS.contains(
            "sourceEditor.addEventListener(\"input\", () => {\n  invalidateLevelEditSourceForDocument(activeDocument());\n});"
        ));
        assert!(!EDITOR_JS.contains("levelName\n    ? replaceLevelByName"));
    }

    #[test]
    fn sprite_source_has_no_legacy_js_target_scanner() {
        for forbidden in [
            "findSpriteDefinitionAtPosition",
            "findSpriteDefinitionBlock",
            "findUnbracedSpriteDefinition",
            "isSpriteDefinitionBoundary",
            "isLineStyleSpriteDefinitionBoundary",
            "isSpriteDefinitionNameToken",
            "isSpriteColorRow(",
            "loadSpriteFromSourcePosition",
            "registerSourceEditableTarget?.(\"sprite\"",
        ] {
            assert!(
                !EDITOR_SPRITE_JS.contains(forbidden),
                "legacy sprite source scanner remains: {forbidden}"
            );
        }
    }

    #[test]
    fn new_puzzle_source_is_blank_in_editor_workspace() {
        let workspace_js = editor_workspace_js();

        assert!(!workspace_js.contains("__PUZZLESTUDIO_NEW_PUZZLE_SOURCE__"));
        assert!(workspace_js.contains("const STARTER_PUZZLE_SOURCE = \"\";"));
        assert!(workspace_js.contains("return STARTER_PUZZLE_SOURCE;"));
        assert!(!workspace_js.contains("starterPuzzleSourceFromTitle"));
    }

    #[test]
    fn new_puzzle_creation_uses_blank_browser_source_for_all_hosts() {
        assert!(EDITOR_BOOT_JS.contains("New puzzle source is browser-runtime owned"));
        assert!(!EDITOR_BOOT_JS.contains(r#"invoke("new_puzzle_source", { request: payload })"#));
        assert!(EDITOR_WORKSPACE_JS.contains("async function newPuzzleSourceForFile(_name)"));
        assert!(EDITOR_WORKSPACE_JS.contains("const name = sanitizeFileName(rawName);"));
        assert!(!EDITOR_WORKSPACE_JS.contains("function ensurePuzzleExtension(name)"));
        assert!(!EDITOR_WORKSPACE_JS.contains("`${cleaned}.puzzle`"));
        assert!(!EDITOR_WORKSPACE_JS.contains("window.PuzzleStudioHost.newPuzzleSource"));
        assert!(
            EDITOR_WORKSPACE_JS.contains("name: kind === \"folder\" ? \"folder\" : \"untitled\",")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("const STARTER_PUZZLE_SOURCE = \"\";"));
        assert!(EDITOR_WORKSPACE_JS.contains("return STARTER_PUZZLE_SOURCE;"));
        assert!(!EDITOR_WORKSPACE_JS.contains("starterPuzzleTitle"));
        assert!(!EDITOR_WORKSPACE_JS.contains("starterPuzzleSource("));
        assert!(
            !EDITOR_WORKSPACE_JS.contains("!documents.length && (editorSeed || !isDesktopHost())")
        );
    }

    #[test]
    fn level3d_stage_resize_tools_support_expand_and_shrink() {
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dShrinkModeButton()"));
        assert!(EDITOR_LEVEL3D_JS.contains("mode: \"shrink\""));
        assert!(EDITOR_LEVEL3D_JS.contains("dimension: \"width\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"left\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"right\""));
        assert!(EDITOR_LEVEL3D_JS.contains("dimension: \"depth\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"front\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"back\""));
        assert!(EDITOR_LEVEL3D_JS.contains("dimension: \"height\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"down\""));
        assert!(EDITOR_LEVEL3D_JS.contains("edge: \"up\""));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dStageResizeFrameEdges(size, hit)"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "function level3dResizeSliceFrameBounds(size, dimension, edge, mode = \"shrink\")"
        ));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("function level3dExpandedSliceFrameBounds(size, dimension, edge)")
        );
        assert!(
            EDITOR_LEVEL3D_JS.contains("function level3dSliceFrameBounds(size, dimension, edge)")
        );
        assert!(EDITOR_CSS.contains(".level3d-shrink-button"));
    }

    #[test]
    fn save_source_file_only_writes_existing_text_files_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Save Before"),
        );
        let notes_path = workspace.write("games/editor_fixture/notes.md", "before\n");
        let image_path = workspace.write("games/editor_fixture/tile.png", [0_u8, 1, 2, 3]);
        let outside_path = workspace.write("outside.puzzle", editor_fixture_source("Outside"));
        let service = EditorService::open(&game_path).expect("open editor fixture");

        service
            .save_source_file(&SaveRequest::new(
                "after\n",
                notes_path.display().to_string(),
            ))
            .expect("save text file in workspace");
        assert_eq!(
            fs::read_to_string(&notes_path).expect("read saved notes"),
            "after\n"
        );

        let binary_error = service
            .save_source_file(&SaveRequest::new(
                "not really png",
                image_path.display().to_string(),
            ))
            .expect_err("binary workspace files are not editable source documents")
            .to_string();
        assert!(binary_error.contains("can only save text workspace files"));

        let outside_error = service
            .save_source_file(&SaveRequest::new(
                editor_fixture_source("Outside Changed"),
                outside_path.display().to_string(),
            ))
            .expect_err("saving outside the editor workspace should be rejected")
            .to_string();
        assert!(outside_error.contains("can only save files under"));
    }

    #[test]
    fn save_source_file_rejects_unloaded_workspace_document_payload() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Save Before"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");
        let body = format!(
            "{{\"source\":\"\",\"puzzlePath\":\"{}\",\"contentLoaded\":false}}",
            game_path.display()
        );
        let request = SaveRequest::from_body(&body, service.state());

        let error = service
            .save_source_file(&request)
            .expect_err("server save must reject unloaded document payloads")
            .to_string();

        assert!(error.contains("cannot save unloaded workspace document"));
        assert_eq!(
            fs::read_to_string(&game_path).expect("read unchanged game"),
            editor_fixture_source("Save Before")
        );
    }

    #[test]
    fn create_source_file_only_adds_new_puzzle_files_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Create Before"),
        );
        let outside_path = workspace.root.join("outside.puzzle");
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let created = service
            .create_source_file(&CreateSourceFileRequest::new(
                editor_fixture_source("Imported"),
                "imported.puzzle",
            ))
            .expect("create new puzzle file");
        assert_eq!(
            fs::read_to_string(&created).expect("read created file"),
            editor_fixture_source("Imported")
        );

        let created_puzzle3 = service
            .create_source_file(&CreateSourceFileRequest::new(
                "puzzle3 imported3 {}\n",
                "imported3.puzzle3",
            ))
            .expect("create new puzzle3 file");
        assert!(created_puzzle3.ends_with("imported3.puzzle3"));
        assert_eq!(
            fs::read_to_string(&created_puzzle3).expect("read created puzzle3 file"),
            "puzzle3 imported3 {}\n"
        );

        let outside_error = service
            .create_source_file(&CreateSourceFileRequest::new(
                editor_fixture_source("Outside"),
                outside_path.display().to_string(),
            ))
            .expect_err("creating outside the editor workspace should be rejected")
            .to_string();
        assert!(outside_error.contains("can only create files under"));

        let text_error = service
            .create_source_file(&CreateSourceFileRequest::new("notes\n", "notes.md"))
            .expect_err("import creates puzzle files only")
            .to_string();
        assert!(text_error.contains("can only create .puzzle"));
    }

    #[test]
    fn rename_workspace_entry_renames_real_files_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Rename Before"),
        );
        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");

        let renamed = service
            .rename_workspace_entry(&RenameWorkspaceEntryRequest::new(
                "game.puzzle",
                "renamed.puzzle",
            ))
            .expect("rename puzzle file");

        assert!(renamed.ends_with("renamed.puzzle"));
        assert!(!game_path.exists());
        assert!(game_path.with_file_name("renamed.puzzle").exists());
    }

    #[test]
    fn rename_workspace_entry_moves_real_folders_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Move Folder"),
        );
        let fragment_path = workspace.write(
            "games/editor_fixture/old/fragment.puzzle",
            "title \"Fragment\"",
        );
        let project_dir = game_path.parent().expect("project dir");
        fs::create_dir(project_dir.join("new")).expect("create destination parent");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");

        let moved = service
            .rename_workspace_entry(&RenameWorkspaceEntryRequest::new("old", "new/old"))
            .expect("move folder");

        assert!(moved.ends_with("new/old"));
        assert!(!fragment_path.exists());
        assert!(project_dir.join("new/old/fragment.puzzle").exists());
    }

    #[test]
    fn rename_workspace_entry_stays_under_workspace_root() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Rename Before"),
        );
        let outside_path = workspace.root.join("outside.puzzle");
        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");

        let outside_error = service
            .rename_workspace_entry(&RenameWorkspaceEntryRequest::new(
                "game.puzzle",
                outside_path.display().to_string(),
            ))
            .expect_err("renaming outside the editor workspace should be rejected")
            .to_string();

        assert!(outside_error.contains("can only rename files under"));
        assert!(game_path.exists());
        assert!(!outside_path.exists());
    }

    #[test]
    fn delete_workspace_entry_removes_real_files_and_folders_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Delete Before"),
        );
        let folder_file = workspace.write(
            "games/editor_fixture/old/fragment.puzzle",
            editor_fixture_source("Fragment"),
        );
        let folder_path = folder_file.parent().expect("folder").to_path_buf();
        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");

        service
            .delete_workspace_entry(&DeleteWorkspaceEntryRequest::new("old/fragment.puzzle"))
            .expect("delete puzzle file");
        assert!(!folder_file.exists());

        let other_file = workspace.write(
            "games/editor_fixture/old/nested.puzzle",
            editor_fixture_source("Nested"),
        );
        assert!(other_file.exists());
        service
            .delete_workspace_entry(&DeleteWorkspaceEntryRequest::new("old"))
            .expect("delete workspace folder");
        assert!(!folder_path.exists());
    }

    #[test]
    fn delete_workspace_entry_stays_under_workspace_root() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Delete Before"),
        );
        let outside_path = workspace.write("outside.puzzle", editor_fixture_source("Outside"));
        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_game_entry(project_dir).expect("open editor fixture");

        let outside_error = service
            .delete_workspace_entry(&DeleteWorkspaceEntryRequest::new(
                outside_path.display().to_string(),
            ))
            .expect_err("deleting outside the editor workspace should be rejected")
            .to_string();
        assert!(outside_error.contains("can only delete files under"));
        assert!(outside_path.exists());

        let root_error = service
            .delete_workspace_entry(&DeleteWorkspaceEntryRequest::new(
                project_dir.display().to_string(),
            ))
            .expect_err("deleting the workspace root should be rejected")
            .to_string();
        assert!(root_error.contains("cannot delete the workspace root"));
        assert!(project_dir.exists());
    }

    #[test]
    fn open_game_entry_scopes_workspace_to_selected_project_folder() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/project_a/game.puzzle",
            editor_fixture_source("Project A"),
        );
        let project_dir = game_path.parent().expect("project dir");
        let notes_path = workspace.write("games/project_a/notes.md", "inside\n");
        let sibling_path = workspace.write(
            "games/project_b/game.puzzle",
            editor_fixture_source("Project B"),
        );

        let service = EditorService::open_game_entry(project_dir).expect("open project folder");
        let state = service.state();

        assert_eq!(
            PathBuf::from(&state.workspace_root),
            project_dir.canonicalize().expect("canonical project dir")
        );
        assert!(paths_contain(&state.documents, "games/project_a/notes.md"));
        assert!(
            !paths_contain(&state.documents, "games/project_b/game.puzzle"),
            "desktop project folders must not expose sibling projects"
        );

        service
            .save_source_file(&SaveRequest::new(
                "inside changed\n",
                notes_path.display().to_string(),
            ))
            .expect("save inside opened project");
        let outside_error = service
            .save_source_file(&SaveRequest::new(
                editor_fixture_source("Project B changed"),
                sibling_path.display().to_string(),
            ))
            .expect_err("sibling project writes must be rejected")
            .to_string();
        assert!(outside_error.contains("can only save files under"));
    }

    #[test]
    fn compile_preview_rejects_preview_paths_outside_open_project() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/project_a/game.puzzle",
            editor_fixture_source("Project A"),
        );
        let project_dir = game_path.parent().expect("project dir");
        let outside_path = workspace.write(
            "games/project_b/game.puzzle",
            editor_fixture_source("Project B"),
        );
        let service = EditorService::open_game_entry(project_dir).expect("open project folder");

        let error = service
            .compile_preview(&PreviewRequest::new(
                editor_fixture_source("Project B changed"),
                outside_path.display().to_string(),
                String::new(),
            ))
            .expect_err("preview paths outside the opened project must be rejected")
            .to_string();
        assert!(error.contains("can only import puzzle files under"));
    }

    #[cfg(unix)]
    #[test]
    fn workspace_document_scan_skips_symlinks_to_files_outside_project() {
        use std::os::unix::fs::symlink;

        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/project_a/game.puzzle",
            editor_fixture_source("Project A"),
        );
        let project_dir = game_path.parent().expect("project dir");
        let outside_path = workspace.write("outside.md", "outside\n");
        let link_path = project_dir.join("outside-link.md");
        symlink(&outside_path, &link_path).expect("create outside symlink");

        let service = EditorService::open_game_entry(project_dir).expect("open project folder");

        assert!(
            !paths_contain(&service.state().documents, "outside-link.md"),
            "workspace document loading must not follow symlinks out of the opened project"
        );
    }

    #[test]
    fn exported_pages_editor_html_seeds_data_and_uses_external_assets() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Exported Editor"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .export_pages_editor_html()
            .expect("export pages editor html");

        assert!(html.contains("window.PuzzleEditorSeed = JSON.parse"));
        assert!(html.contains(r#"<html lang="en">"#));
        assert!(!html.contains("window.PuzzleStudioGameWasmAssets = {"));
        assert!(html.contains("Exported Editor"));
        assert!(!html.contains("gameVisualsJs"));
        assert!(html.contains(r#"<link rel="icon" type="image/svg+xml" href="favicon.svg">"#));
        assert!(
            html.contains(r#"<link rel="stylesheet" href="editor.css?v=desktop-export-link">"#)
        );
        assert!(html.contains(r#"<script src="editor.js?v=import-export-api"></script>"#));
        assert!(
            html.contains(r#"<script src="editor_import_export.js?v=import-export-api"></script>"#)
        );
        assert!(html.contains(r#"<script src="editor_dom.js"></script>"#));
        assert!(!html.contains("<script>\nwindow.PuzzleAssets ="));
        assert!(!html.contains("PuzzleEditorThemeImports"));
        assert!(!html.contains("PuzzleStudioEmbeddedWasm"));
        assert!(!html.contains("PuzzleStudioEmbeddedGameWasm"));
        assert!(!html.contains("wasmBase64"));
        assert!(!html.contains(r#"href="/editor.css""#));
        assert!(!html.contains(r#"src="/editor.js""#));
    }

    #[test]
    fn served_editor_does_not_route_to_editor_html_path() {
        let source = include_str!("lib.rs");
        let removed_url = format!(
            "html-editor serving http://127.0.0.1:{{port}}/{}",
            "editor.html"
        );
        let removed_route = format!("(\"GET\", \"/{}\")", "editor.html");
        assert!(source.contains("html-editor serving http://127.0.0.1:{port}/editor"));
        assert!(!source.contains(&removed_url));
        assert!(source.contains("(\"GET\", \"/\") | (\"GET\", \"/editor\")"));
        assert!(!source.contains(&removed_route));
    }

    #[test]
    fn browser_preview_compile_uses_browser_runtime_not_host_api() {
        assert!(EDITOR_RUNTIME_JS.contains("window.PuzzleStudioRuntime"));
        assert!(EDITOR_RUNTIME_JS.contains("compile_workspace_preview"));
        assert!(EDITOR_RUNTIME_JS.contains("export_workspace_html"));
        assert!(EDITOR_RUNTIME_JS.contains("querySynchronizedAnalysisWorker(\"highlightRange\""));
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_highlight_range_json"));
        assert!(EDITOR_RUNTIME_JS.contains("solver_task_initial_display_state_json"));
        assert!(EDITOR_HTML.contains("editor_runtime.js"));
        assert!(
            EDITOR_HTML.find("editor_runtime.js").unwrap()
                < EDITOR_HTML.find("editor_boot.js").unwrap()
        );
        assert!(EDITOR_BOOT_JS.contains("editorRuntime().compilePreview(payload)"));
        assert!(EDITOR_BOOT_JS.contains("editorRuntime().exportHtml(payload)"));
        assert!(EDITOR_BOOT_JS.contains("editorRuntime().highlightSource(payload)"));
        assert!(EDITOR_RUNTIME_JS.contains("gameRuntimeAssets()"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_game/puzzle_wasm_game.js"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_game/puzzle_wasm_game_bg.wasm"));
        assert!(EDITOR_RUNTIME_JS.contains("gameRuntimeAssetsPromise"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("exportStandaloneHtml({"));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains("html: latestHtml"));
        assert!(EDITOR_JS.contains("PuzzleStudioRuntimeAssetRequest"));
        assert!(EDITOR_JS.contains("PuzzleStudioRuntimeAssetResponse"));
        assert!(EDITOR_JS.contains("previewRuntimeAssetWindows"));
        assert!(!EDITOR_BOOT_JS.contains("invoke(\"compile_preview\""));
        assert!(!EDITOR_BOOT_JS.contains("invoke(\"highlight_source\""));
        assert!(!EDITOR_BOOT_JS.contains("fetchText(\"/api/preview\""));
        assert!(!EDITOR_BOOT_JS.contains("fetchText(\"/api/highlight\""));
        assert!(!EDITOR_JS.contains("function compilePreviewWithWasm"));
        assert!(!EDITOR_JS.contains("function embedStandaloneRuntimeWasm"));
        assert!(!EDITOR_JS.contains("window.PuzzleStudioGameWasmAssets"));
        assert!(!EDITOR_JS.contains("previewDirty"));
        assert!(!EDITOR_JS.contains("document.previewHtml = html"));
        assert!(!EDITOR_JS.contains("previewDocument.previewHtml = nextHtml"));
        assert!(!EDITOR_WORKSPACE_JS.contains("Preview will compile in browser"));
        assert!(!EDITOR_WORKSPACE_JS.contains("queuePreviewCompile"));
        assert!(!EDITOR_WORKSPACE_JS.contains("await renderPreview();"));
        assert!(!EDITOR_WORKSPACE_JS.contains("renderPreview();"));
        assert!(!EDITOR_WORKSPACE_JS.contains("renderActivePreviewAfterWorkspaceSelection"));
        assert!(!EDITOR_WORKSPACE_JS.contains("treeWithEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("mergeEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewHtml"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewError"));
        assert!(!EDITOR_WORKSPACE_JS.contains("document.previewHtml ||"));
        assert!(!EDITOR_WORKSPACE_JS.contains("previewDocument?.previewHtml"));
        assert!(EDITOR_WORKSPACE_JS.contains("const previewTargetUnchanged = previewDocument"));
        assert!(EDITOR_WORKSPACE_JS.contains("markPreviewDirty();"));
        assert!(
            EDITOR_WORKSPACE_JS
                .contains("} else {\n    invalidateCompiledPreview(previewDocument);\n  }")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("Run preview to compile."));
    }

    #[test]
    fn editor_wasm_surface_excludes_runtime_exports() {
        assert!(PUZZLE_WASM_JS.contains("export function compile_preview"));
        assert!(PUZZLE_WASM_JS.contains("export function export_html"));
        assert!(PUZZLE_WASM_JS.contains("export function solve_state"));
        assert!(PUZZLE_WASM_JS.contains("export function solver_task_initial_display_state_json"));
        assert!(!PUZZLE_WASM_JS.contains("export function solve_state_with_progress"));
        assert!(!PUZZLE_WASM_JS.contains("WasmCoreRuntime"));
        assert!(!PUZZLE_WASM_JS.contains("WasmPuzzle3Runtime"));
        assert!(!PUZZLE_WASM_JS.contains("WasmStandaloneSession"));
        assert!(!PUZZLE_WASM_JS.contains("export function transition_program_outcome"));

        assert!(!EDITOR_JS.contains("./wasm_core/puzzle_core_wasm.js"));
        assert!(!EDITOR_JS.contains("new module.WasmCompiledCoreRuntime"));
        assert!(!EDITOR_JS.contains("new compiler.WasmCoreRuntime"));
    }

    #[test]
    fn sound_tools_script_exposes_editor_sound_api() {
        let script = sound_tools_script();
        assert!(script.contains("window.PuzzleSoundGenerator"));
        assert!(script.contains("generateSoundEffect"));
        assert!(script.contains("generatePuzzleScriptSoundEffect"));
        assert!(script.contains("createPuzzleScriptSfxPlayer"));
        assert!(script.contains("generateSong"));
        assert!(script.contains("exportSoundEffect"));
        assert!(script.contains("PuzzleSoundToolsReady"));
        assert!(script.contains("ui-tap"));
        assert!(script.contains("buildSelectLayers"));
    }

    #[test]
    fn sound_builder_exposes_sfx_volume() {
        assert!(EDITOR_HTML.contains(r#"id="soundsSfxVolumeInput""#));
        assert!(EDITOR_SOUNDS_JS.contains("function soundSfxVolume()"));
        assert!(EDITOR_SOUNDS_JS.contains("volume=${soundSfxVolume().toFixed(2)}"));
        assert!(EDITOR_SOUNDS_JS.contains("function soundSfxType()"));
        assert!(
            EDITOR_SOUNDS_JS.contains(
                "generateSoundEffect(soundsSfxSeedInput.value, { type: soundSfxType() })"
            )
        );
        assert!(!EDITOR_SOUNDS_JS.contains("generatePuzzleScriptSoundEffect"));
        assert!(!EDITOR_SOUNDS_JS.contains("createPuzzleScriptSfxPlayer"));
        assert!(EDITOR_SOUNDS_JS.contains(
            "createSfxPlayer(sounds.context, soundSfxEffect(), { volume: soundSfxVolume() })"
        ));
        assert!(EDITOR_SOUNDS_JS.contains("sounds.sfxPlayer.start(sounds.context.currentTime);"));
    }

    #[test]
    fn sound_source_sync_scans_nested_sounds_blocks() {
        assert!(EDITOR_SOUNDS_JS.contains("function findSoundsBlocks(lines)"));
        assert!(
            EDITOR_SOUNDS_JS
                .contains("const soundsBlock = findSoundsBlockAtPosition(lines, position);")
        );
        assert!(EDITOR_SOUNDS_JS.contains("for (const soundsBlock of findSoundsBlocks(lines))"));
        assert!(!EDITOR_SOUNDS_JS.contains("function findTopLevelSoundsBlock"));
    }

    #[test]
    fn sound_source_edits_generate_unindented_definition_lines() {
        assert!(EDITOR_SOUNDS_JS.contains("const insertText = `${line}\\n`;"));
        assert!(EDITOR_SOUNDS_JS.contains("const block = `sounds {\\n${line}\\n}\\n`;"));
        assert!(
            EDITOR_SOUNDS_JS.contains(
                "const replacement = `${definition.line}${hasNewline ? \"\\n\" : \"\"}`;"
            )
        );
        assert!(!EDITOR_SOUNDS_JS.contains("`sounds {\\n\\t${line}\\n}\\n`"));
        assert!(!EDITOR_SOUNDS_JS.contains("`${soundsBlock.indent}\\t`"));
        assert!(!EDITOR_SOUNDS_JS.contains("function soundDefinitionIndent("));
    }

    #[test]
    fn sprite3d_preview_uses_runtime_visual_ordering_contract() {
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function comparePrimitiveOrder(a, b)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function faceGridOrder(corners, view)"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("sceneFaces.sort(Puzzle3VisualCore.comparePrimitiveOrder);")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("return Puzzle3VisualCore.faceGridOrder(corners, sprite3dVisualView());")
        );
        assert!(EDITOR_SPRITE3D_JS.contains("const previewOwner = sprite3dPreviewRenderOwner();"));
        assert!(EDITOR_SPRITE3D_JS.contains("ownerCell: previewOwner"));
        assert!(EDITOR_SPRITE3D_JS.contains("renderPriority: order"));
        assert!(EDITOR_SPRITE3D_JS.contains("assignSprite3dPrimitiveOrder(sceneFaces);"));
        assert!(EDITOR_SPRITE3D_JS.contains("primitive.frameIndex = index;"));
        assert!(EDITOR_SPRITE3D_JS.contains("primitive.stableKey = occurrence === 0 ? baseKey"));
        assert!(EDITOR_SPRITE3D_JS.contains("rectsFromCells: sprite3dUnitFaceRects"));
        assert!(!EDITOR_SPRITE3D_JS.contains("function compareSprite3dSceneFaceOrder"));
    }

    #[test]
    fn sprite3d_presentation_changes_redraw_all_animation_previews() {
        let redraw = EDITOR_SPRITE3D_JS
            .split_once("function renderSprite3dPresentationSurfaces() {")
            .expect("shared 3D presentation redraw")
            .1
            .split_once("\n}")
            .expect("shared 3D presentation redraw end")
            .0;
        assert!(redraw.contains("renderSprite3dPreview();"));
        assert!(redraw.contains("renderSprite3dAnimationFrameStrip();"));
        assert!(redraw.contains("renderSharedSpriteAnimationPlaybackView(context, frame);"));

        for function_name in [
            "toggleSprite3dGrid",
            "resetSprite3dCamera",
            "setSprite3dCameraValue",
        ] {
            let body = EDITOR_SPRITE3D_JS
                .split_once(&format!("function {function_name}"))
                .unwrap_or_else(|| panic!("missing {function_name}"))
                .1
                .split_once("\n}")
                .unwrap_or_else(|| panic!("missing {function_name} end"))
                .0;
            assert!(
                body.contains("renderSprite3dPresentationSurfaces();"),
                "{function_name} must redraw every 3D animation preview surface"
            );
        }
    }

    #[test]
    fn tauri_static_editor_includes_puzzle3_visual_core_asset() {
        assert_eq!(EDITOR_STATIC_PUZZLE3_VISUAL_CORE_JS, PUZZLE3_VISUAL_CORE_JS);
        assert!(EDITOR_HTML.contains(r#"<script src="puzzle3_visual_core.js"></script>"#));
    }

    #[test]
    fn tauri_static_editor_includes_renderer_assets() {
        assert_eq!(EDITOR_STATIC_RENDERER_JS, RENDERER_JS);
        assert_eq!(EDITOR_STATIC_RENDERER_CSS, RENDERER_CSS);
        assert!(EDITOR_HTML.contains(r#"<link rel="stylesheet" href="renderer.css">"#));
        assert!(
            EDITOR_HTML.contains(r#"<script src="renderer.js?v=board-canvas-visuals"></script>"#)
        );
    }

    #[test]
    fn sprite3d_editor_resyncs_if_script_loads_after_pane_selection() {
        assert!(EDITOR_SPRITE3D_JS.contains("function syncSprite3dBuilderAfterScriptLoad()"));
        assert!(EDITOR_SPRITE3D_JS.contains("currentPreviewMode === \"sprite3d\""));
        assert!(
            EDITOR_SPRITE3D_JS.contains("loadFirstFocusedPuzzleEntry(\"sprite\", \"sprite3d\")")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("resetSprite3dBuilder();\nsyncSprite3dBuilderAfterScriptLoad();")
        );
    }

    #[test]
    fn editor_loads_puzzle3_visual_core_before_3d_editor_renderers() {
        let core = EDITOR_HTML
            .find(r#"<script src="puzzle3_visual_core.js"></script>"#)
            .expect("editor loads puzzle3 visual core");
        let level3d = EDITOR_HTML
            .find(r#"<script src="editor_level3d.js"></script>"#)
            .expect("editor loads 3D level editor");
        let editor = EDITOR_HTML
            .find(r#"<script src="editor.js?v=import-export-api"></script>"#)
            .expect("editor loads main editor script");
        let import_export = EDITOR_HTML
            .find(r#"<script src="editor_import_export.js?v=import-export-api"></script>"#)
            .expect("editor loads import/export helpers");
        let sprite3d = EDITOR_HTML
            .find(r#"<script src="editor_sprite3d.js"#)
            .expect("editor loads 3D sprite editor");

        assert!(core < level3d);
        assert!(level3d < import_export);
        assert!(import_export < editor);
        assert!(core < editor);
        assert!(core < sprite3d);
    }

    #[test]
    fn tauri_editor_busts_cache_for_theme_css_and_tab_unsaved_assets() {
        assert!(
            EDITOR_HTML.contains(r#"<script src="editor_boot.js?v=desktop-export-link"></script>"#)
        );
        assert!(
            EDITOR_HTML.contains(r#"<script src="renderer.js?v=board-canvas-visuals"></script>"#)
        );
        assert!(
            EDITOR_HTML
                .contains(r#"<link rel="stylesheet" href="editor.css?v=desktop-export-link">"#)
        );
        assert!(
            EDITOR_HTML.contains(
                r#"<script src="editor_source.js?v=source-overlay-scroll-sync"></script>"#
            )
        );
        assert!(
            EDITOR_HTML
                .contains(r#"<script src="editor_workspace.js?v=outline-pane-layout"></script>"#)
        );
        assert!(
            EDITOR_HTML
                .contains(r#"<script src="editor_import_export.js?v=import-export-api"></script>"#)
        );
        assert!(EDITOR_HTML.contains(r#"<script src="editor.js?v=import-export-api"></script>"#));
        assert!(EDITOR_WORKSPACE_JS.contains("document-tab-unsaved-dot"));
        assert!(EDITOR_WORKSPACE_JS.contains("updateDocumentTabUnsavedStates"));
        assert!(EDITOR_WORKSPACE_JS.contains("setSourceEditorValue(sourceText, {\n    preserveUndoOnSameValue: document.id === previousActiveFileId,\n  });\n  if (isTextDocument(document)) {\n    restoreSourceFoldState(document.sourceFoldedBlockKeys);\n  }\n  updateDocumentTabUnsavedStates();"));
        assert!(EDITOR_CSS.contains(".document-tab.is-unsaved .document-tab-unsaved-dot"));
    }

    #[test]
    fn sprite3d_preview_slice_selection_uses_ray_hits_before_height_fallback() {
        assert!(EDITOR_SPRITE3D_JS.contains("function sprite3dPreviewRay(point, view)"));
        assert!(EDITOR_SPRITE3D_JS.contains("function sprite3dRaycastOccupiedVoxel(ray)"));
        assert!(EDITOR_SPRITE3D_JS.contains("const voxelHit = sprite3dRaycastOccupiedVoxel(ray);"));
        assert!(EDITOR_SPRITE3D_JS.contains("return sprite3dApproximateSliceFromRay(ray);"));
    }

    #[test]
    fn visual_editors_allow_vertical_camera_pitch() {
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_CAMERA_MIN_PITCH_DEGREES = -90;"));
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_CAMERA_MAX_PITCH_DEGREES = 90;"));
        assert!(EDITOR_SPRITE3D_JS.contains("const SPRITE3D_CAMERA_MIN_PITCH_DEGREES = -90;"));
        assert!(EDITOR_SPRITE3D_JS.contains("const SPRITE3D_CAMERA_MAX_PITCH_DEGREES = 90;"));
        assert!(EDITOR_LEVEL3D_JS.contains("LEVEL3D_CAMERA_MAX_PITCH_DEGREES"));
        assert!(EDITOR_SPRITE3D_JS.contains("SPRITE3D_CAMERA_MAX_PITCH_DEGREES"));
        assert!(!EDITOR_LEVEL3D_JS.contains("level3dClampNumber(value, -80, 80)"));
        assert!(!EDITOR_SPRITE3D_JS.contains("sprite3dClampNumber(value, -80, 80)"));
        assert!(EDITOR_HTML.contains("id=\"level3dCameraRollScrub\""));
        assert!(EDITOR_DOM_JS.contains(
            "const level3dCameraRollScrub = document.querySelector(\"#level3dCameraRollScrub\");"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("rollDegrees: Number(camera.rollDegrees ?? 0)"));
    }

    #[test]
    fn level3d_layer_editor_uses_orthographic_top_down_camera() {
        assert!(EDITOR_HTML.contains("id=\"level3dLayerPalette\""));
        assert!(EDITOR_DOM_JS.contains(
            "const level3dLayerPalette = document.querySelector(\"#level3dLayerPalette\");"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("function renderLevel3dLayerPalette()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerResizeModeButton("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerGridButton("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerTransformButton("));
        assert!(EDITOR_LEVEL3D_JS.contains("level3d-layer-transform-row"));
        assert!(EDITOR_LEVEL3D_JS.contains("level3d-layer-edit-row"));
        assert!(!EDITOR_LEVEL3D_JS.contains("function level3dLayerScopeToggle("));
        assert!(!EDITOR_LEVEL3D_JS.contains("Top-down 3D level edit scope"));
        assert!(!EDITOR_LEVEL3D_JS.contains("function level3dLayerTransformScopeToggle("));
        assert!(!EDITOR_LEVEL3D_JS.contains("Current slice"));
        assert!(!EDITOR_LEVEL3D_JS.contains("All slices"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerFillButton()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerEraserButton()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dVisiblePaletteEntries("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dVisibleObjectKeyForChar("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dObjectNameIsVisible("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerVisibilityEntries("));
        assert!(EDITOR_LEVEL3D_JS.contains("label: layerNames.get(layerIndex) || \"\""));
        assert!(EDITOR_LEVEL3D_JS.contains("level3dObjectIsVisible(object, snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("function bucketFillLevel3dLayerFromPosition("));
        assert!(EDITOR_LEVEL3D_JS.contains("function resizeLevel3dLayerEdge("));
        assert!(EDITOR_LEVEL3D_JS.contains("function transformLevel3dRowsWithMap("));
        assert!(EDITOR_LEVEL3D_JS.contains("function rotateLevel3dLayerLeft("));
        assert!(EDITOR_LEVEL3D_JS.contains("function drawLevel3dTopDownTilePreview("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dTopDownSpriteProjection("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dLayerCamera()"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "return { yawDegrees: 0, pitchDegrees: 90, rollDegrees: 0, zoom: 1, projection: \"orthographic\" };"
        ));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("`${activePreviewDocument()?.id || \"\"}:puzzle3-layer-renderer:${currentLevel3dLayerZ()}`")
        );
    }

    #[test]
    fn editor_docs_html_renders_documents_nav() {
        let html = editor_docs_html();
        assert!(html.contains("class=\"docs-layout\""));
        assert!(html.contains("class=\"docs-nav-level\">Basic</div>"));
        assert!(html.contains("class=\"docs-nav-level\">Advanced</div>"));
        assert!(html.contains("data-docs-article=\"start\""));
        assert!(html.contains("PuzzleStudio Documents"));
        assert!(html.contains("class=\"language-puzzle\""));
        assert!(html.contains("syntax-keyword"));
        assert!(
            html.find("class=\"docs-nav-level\">Basic</div>")
                < html.find("class=\"docs-nav-level\">Advanced</div>")
        );
    }

    #[test]
    fn editor_docs_articles_have_mutual_adjacent_links() {
        let html = editor_docs_html();
        for (index, pair) in EDITOR_DOCS_PAGES.windows(2).enumerate() {
            let current = &pair[0];
            let next = &pair[1];
            let current_article = editor_docs_article_html(&html, index);
            let next_article = editor_docs_article_html(&html, index + 1);
            assert!(
                current_article.contains(&format!(
                    "class=\"docs-page-link docs-page-link-next\" type=\"button\" data-docs-page=\"{}\"",
                    next.id
                )),
                "{} should link forward to {}",
                current.id,
                next.id
            );
            assert!(
                next_article.contains(&format!(
                    "class=\"docs-page-link docs-page-link-previous\" type=\"button\" data-docs-page=\"{}\"",
                    current.id
                )),
                "{} should link back to {}",
                next.id,
                current.id
            );
        }
    }

    #[cfg(feature = "editor-docs")]
    fn editor_docs_article_html(html: &str, page_index: usize) -> &str {
        let page = &EDITOR_DOCS_PAGES[page_index];
        let start = html
            .find(&format!("data-docs-article=\"{}\"", page.id))
            .expect("docs article should be rendered");
        let end = EDITOR_DOCS_PAGES
            .get(page_index + 1)
            .and_then(|next| html[start + 1..].find(&format!("data-docs-article=\"{}\"", next.id)))
            .map(|offset| start + 1 + offset)
            .unwrap_or(html.len());
        &html[start..end]
    }

    #[test]
    fn static_editor_loads_desktop_documents_from_host() {
        assert!(EDITOR_HTML.contains("<!-- PUZZLESTUDIO_EDITOR_DOCS -->"));
        assert!(EDITOR_BOOT_JS.contains("async editorDocsHtml()"));
        assert!(EDITOR_BOOT_JS.contains("invoke(\"editor_docs\")"));
        assert!(EDITOR_JS.contains("function ensureEditorDocsLoaded()"));
        assert!(EDITOR_JS.contains("window.PuzzleStudioHost.editorDocsHtml()"));
    }

    #[test]
    fn level3d_temporary_legend_chars_stay_out_of_palette() {
        assert!(
            EDITOR_LEVEL3D_JS.contains("const LEVEL3D_LEGEND_CHAR_CANDIDATES = \"@$%&?!~^:;_+-*/")
        );
        assert!(!EDITOR_LEVEL3D_JS.contains("const LEVEL3D_LEGEND_CHAR_CANDIDATES = \"xyz"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "level3d.palette.push({ char: ch, objects: [...cleanObjects], temporary: true });"
        ));
        assert!(EDITOR_LEVEL3D_JS.contains("for (const entry of level3dVisiblePaletteEntries())"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dTemporaryLegendEntriesForLevelData("));
    }

    #[test]
    fn sprite3d_camera_default_starts_at_y15_p30() {
        assert!(EDITOR_SPRITE3D_JS.contains("yawDegrees: 15,"));
        assert!(EDITOR_SPRITE3D_JS.contains("pitchDegrees: 30,"));
    }

    #[test]
    fn sprite3d_preview_is_square_and_reserves_overlay_bars() {
        assert!(EDITOR_CSS.contains("--sprite3d-preview-width: 320px;"));
        assert!(EDITOR_CSS.contains("--sprite3d-preview-height: var(--sprite3d-preview-width);"));
        assert!(EDITOR_CSS.contains(".sprite3d-preview-wrap {\n  position: relative;"));
        assert!(EDITOR_CSS.contains("aspect-ratio: 1 / 1;"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("const safeHeight = Math.max(1, height - safeTop - safeBottom);")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("originY: safeTop + safeHeight / 2 - ((minY + maxY) / 2) * scale,")
        );
        assert!(EDITOR_HTML.contains(
            r#"id="sprite3dPreviewCanvas" class="sprite3d-preview-canvas" width="320" height="320""#
        ));
        assert!(EDITOR_HTML.contains(r#"id="sprite3dPreviewCanvas""#));
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="spriteAnimationFrameStrip""#)
                .count(),
            1
        );
        assert!(!EDITOR_HTML.contains(r#"id="sprite3dAnimationFrameStrip""#));
        assert!(EDITOR_SPRITE_JS.contains("function mountSharedSpriteAnimationUi(dimension)"));
        assert!(EDITOR_SPRITE_JS.contains("previewColumn.insertBefore(toolbar, previewStage);"));
        assert!(EDITOR_SPRITE_JS.contains("previewStage.append(sidecar);"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("function renderSharedSpriteAnimationPlaybackView(context, frame)")
        );
        assert!(EDITOR_SPRITE_JS.contains("renderPlaybackFrame: is3d"));
        assert!(EDITOR_SPRITE3D_JS.contains("syncSpriteAnimationPlayback();"));
        assert!(EDITOR_SPRITE_JS.contains(
            "function sharedSpriteAnimationController(dimension = currentSpritePaneMode)"
        ));
        assert!(
            EDITOR_SPRITE_JS
                .contains("function insertSharedSpriteAnimationFrameAt(dimension, index)")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("function removeSharedSpriteAnimationFrameAt(dimension, index)")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("return insertSharedSpriteAnimationFrameAt(\"sprite3d\", index);")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("return removeSharedSpriteAnimationFrameAt(\"sprite3d\", index);")
        );
        assert!(!EDITOR_SPRITE3D_JS.contains("frames.splice(insertIndex"));
        assert!(!EDITOR_SPRITE3D_JS.contains("frames.splice(removeIndex"));
        assert!(EDITOR_CSS.contains(
            ".sprite3d-preview-column > .sprite-animation-toolbar.is-sprite3d-shared {\n  width: max-content;"
        ));
        assert!(EDITOR_CSS.contains(
            "@container (min-width: 704px) {\n  .sprite3d-workspace {\n    grid-template-columns: var(--sprite3d-slice-size) max-content;\n  }\n\n  .sprite3d-builder.is-animation-mode .sprite3d-slice-wrap {\n    padding-top: calc(var(--sprite3d-overlay-control-height) + 10px);"
        ));
        assert!(EDITOR_SPRITE_JS.contains("function renderSpriteAnimationFrameStripView(options)"));
        assert!(EDITOR_SPRITE3D_JS.contains("renderSpriteAnimationFrameStripView({"));
        assert!(EDITOR_SPRITE3D_JS.contains(
            "renderCells: (index) => sprite3dAnimationFramePreview(sprite3d.frames[index])"
        ));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("renderSprite3dPreviewCanvas(canvas, frame, { overlays: false });")
        );
        assert!(EDITOR_CSS.contains(
            ".sprite3d-builder.is-animation-mode .sprite3d-preview-stage {\n  grid-template-columns: var(--sprite3d-preview-width) 52px;"
        ));
        assert!(EDITOR_SPRITE3D_JS.contains("const SPRITE3D_PREVIEW_BASE_ZOOM = 1;"));
        assert!(EDITOR_SPRITE3D_JS.contains("const padding = 0;"));
        assert!(
            EDITOR_SPRITE3D_JS.contains("const overlaySafeInset = 8 + overlayControlHeight + 4;")
        );
    }

    #[test]
    fn level3d_palette_preview_ignores_camera_zoom_and_origin() {
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPalettePreviewCamera(source)"));
        assert!(EDITOR_LEVEL3D_JS.contains("zoom: 1,"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPalettePreviewOptions(camera)"));
        assert!(EDITOR_LEVEL3D_JS.contains("origin: { x: 0, y: 0, z: 0 },"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPaletteObjectDescriptor("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPreviewSprites("));
        assert!(EDITOR_LEVEL3D_JS.contains("function sourceLevel3dSprites(source)"));
        assert!(EDITOR_LEVEL3D_JS.contains("...sourceLevel3dSprites(source),"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "return level3dObjectHasPreviewSprite(object, exportData, sprites) ? object : null;"
        ));
        assert!(
            EDITOR_LEVEL3D_JS.contains("drawLevel3dCellsPreview(ctx, width, height, snapshot, [{")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("}], level3dPalettePreviewOptions(snapshot.camera));"));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("const camera = options.camera || level3dPreviewCamera(snapshot);")
        );
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("const previewOrigin = options.origin || level3dPreviewOriginState();")
        );
    }

    #[test]
    fn exported_pages_editor_html_seeds_workspace_root_before_document_tree() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Seeded Editor"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .export_pages_editor_html()
            .expect("export pages editor html");
        let workspace_root_index = html
            .find("window.PuzzleEditorSeed = JSON.parse")
            .expect("seeded editor should define seed before workspace scripts load");
        let embedded_documents_index = html
            .find(r#"<script src="editor_workspace.js?v=outline-pane-layout"></script>"#)
            .expect("seeded editor should load workspace code after seed data");

        assert!(
            workspace_root_index < embedded_documents_index,
            "seeded web editor must strip workspace root before building the file tree"
        );
    }
}
