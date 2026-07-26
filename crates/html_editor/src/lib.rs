use std::collections::HashMap;
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
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN: &str = include_str!("../docs/puzzle-block.md");
#[cfg(feature = "editor-docs")]
const EDITOR_DOCS_LAYERS_MARKDOWN: &str = include_str!("../docs/layers.md");
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
const EDITOR_DOCS_VISUALS_MARKDOWN: &str = include_str!("../docs/visuals.md");
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
const EDITOR_DOCS_VISUAL_SHAPES_MARKDOWN: &str = include_str!("../docs/visual-shapes.md");
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
const EDITOR_SOLVER_WORKER_JS: &str = include_str!("../static/editor_solver_worker.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_BOOT_JS: &str = include_str!("../static/editor_boot.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_ICONS_JS: &str = include_str!("../static/editor_icons.js");
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
const EDITOR_VISUAL_DOCUMENT_JS: &str = include_str!("../static/editor_visual_document.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_VISUAL_JS: &str = include_str!("../static/editor_visual.js");
#[cfg(feature = "embedded-assets")]
const VISUAL_TWEEN_CORE_JS: &str = include_str!("../../html_play/static/visual_tween_core.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../../html_play/static/puzzle3_visual_core.js");
#[cfg(all(test, feature = "embedded-assets"))]
const EDITOR_STATIC_PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_VISUAL3D_JS: &str = include_str!("../static/editor_visual3d.js");
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
const PUZZLE_PLAYER_WASM_JS: &str =
    include_str!("../../html_play/static/wasm_player/puzzle_wasm_player.js");
#[cfg(feature = "embedded-assets")]
const PUZZLE_PLAYER_WASM_BG: &[u8] =
    include_bytes!("../../html_play/static/wasm_player/puzzle_wasm_player_bg.wasm");
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
#[cfg(feature = "embedded-assets")]
const RENDER_ASSET_DECODER_JS: &str =
    include_str!("../../html_play/static/render_asset_decoder.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_AUTHORING_RENDERER_JS: &str = include_str!("../static/editor_authoring_renderer.js");
#[cfg(feature = "embedded-assets")]
const PAGES_EXAMPLE_PUZZLE_PATH: &str = "starter/01-basic.puzzle";
#[cfg(all(test, feature = "embedded-assets"))]
const PAGES_EXAMPLE_PUZZLE_SOURCE: &str = include_str!("../starter/01-basic.puzzle");
#[cfg(feature = "embedded-assets")]
const PAGES_STARTER_DOCUMENTS: &[(&str, &str)] = &[
    (
        PAGES_EXAMPLE_PUZZLE_PATH,
        include_str!("../starter/01-basic.puzzle"),
    ),
    ("starter/README.md", include_str!("../starter/README.md")),
    (
        "starter/02-scenes-and-theme.puzzle",
        include_str!("../starter/02-scenes-and-theme.puzzle"),
    ),
    (
        "starter/03-sound.puzzle",
        include_str!("../starter/03-sound.puzzle"),
    ),
    (
        "starter/04-animation.puzzle",
        include_str!("../starter/04-animation.puzzle"),
    ),
    (
        "starter/05-tags-marks-and-routines.puzzle",
        include_str!("../starter/05-tags-marks-and-routines.puzzle"),
    ),
    (
        "starter/06-3d.puzzle",
        include_str!("../starter/06-3d.puzzle"),
    ),
    (
        "starter/07-meta-level.puzzle",
        include_str!("../starter/07-meta-level.puzzle"),
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
        EditorService::open_path(puzzle_path)?
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
                        "usage: html-editor [path/to/workspace-or-game.puzzle] [-o docs/index.html] [--serve] [--port 8787]"
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
            .find(|(path, _)| *path == PAGES_EXAMPLE_PUZZLE_PATH)
            .map(|(_, source)| (*source).to_string())
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
                    .map(|(path, document_source)| EditorDocument {
                        puzzle_path: (*path).to_string(),
                        encoding: "text".to_string(),
                        mime_type: mime_type(Path::new(path)).to_string(),
                        source: (*document_source).to_string(),
                        data_url: String::new(),
                        content_loaded: true,
                        preview_html: String::new(),
                        preview_error: String::new(),
                        game_css: String::new(),
                        imported_by: Vec::new(),
                    })
                    .collect(),
            },
        }
    }

    pub fn open_path(path: &Path) -> Result<Self, AppError> {
        if path.is_dir() {
            return Self::open_workspace_root(path);
        }
        let puzzle_path = puzzle_lang::resolve_game_entry(path)
            .map_err(|error| AppError::Config(error.to_string()))?;
        let workspace_root = puzzle_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
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
            load_base_game_visuals_js(&puzzle_path, &workspace_root, &AssetsDef::default(), &[])?;
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
        let workspace = puzzle_workspace::FileWorkspace::load_with_entry_source(
            &preview_path,
            &workspace_root,
            Some(&request.source),
        )
        .map_err(AppError::Config)?;
        let document = workspace.compile().map_err(AppError::Diagnostics)?;
        let manifest = puzzle_lang::workspace_presentation_manifest_from_document(&document);
        let game_visuals_js = load_base_game_visuals_js(
            &preview_path,
            &workspace_root,
            &document.assets,
            &manifest.visual_image_paths,
        )?;
        html_play::export_editor_preview_html_from_document(
            &document,
            workspace.entry_source(),
            &preview_path.display().to_string(),
            &request.game_css,
            &game_visuals_js,
        )
        .map_err(AppError::Diagnostics)
    }

    pub fn highlight_json(&self, source: &str) -> Result<String, AppError> {
        Ok(Self::highlight_source_json(source))
    }

    pub fn highlight_source_json(source: &str) -> String {
        puzzle_lang::analyze_source(source).highlight_json(false)
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
    preview_html: String,
    preview_error: String,
    game_css: String,
    imported_by: Vec<String>,
}

struct WorkspacePuzzleDocument {
    path: PathBuf,
    workspace_path: String,
    source: String,
}

#[derive(Default)]
struct WorkspaceImportGraph {
    imported_by: HashMap<PathBuf, Vec<PathBuf>>,
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
    assets: &AssetsDef,
    image_paths: &[String],
) -> Result<String, AppError> {
    let mut scripts = vec![asset_resolver_js(workspace_root, assets, image_paths)?];
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
    workspace_root: &Path,
    assets: &AssetsDef,
    image_paths: &[String],
) -> Result<String, AppError> {
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
        let path = resolve_asset_path(workspace_root, &asset_path)?;
        push_asset_resolver_entry(
            workspace_root,
            &path,
            workspace_root,
            &mut files,
            &mut first,
        )?;
    }
    files.push('}');
    Ok(format!(
        "window.PuzzleAssets = {{ files: {files}, url(path) {{ const key = String(path || '').replaceAll('\\\\\\\\', '/'); if (Object.prototype.hasOwnProperty.call(this.files, key)) return this.files[key]; if (/^(?:data:|https?:|#)/.test(key)) return key; throw new Error(`Puzzle asset is not embedded: ${{key}}. Declare it with file \\\"${{key}}\\\" in assets.`); }} }};"
    ))
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
            "asset path must be workspace-relative: {asset_path}"
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
            "asset file is outside workspace: {}",
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
    let import_graph = build_workspace_import_graph(&puzzle_documents)?;
    let puzzle_sources_by_path = puzzle_documents
        .into_iter()
        .map(|document| (document.path, document.source))
        .collect::<HashMap<_, _>>();

    let mut documents = Vec::new();
    for path in paths {
        if puzzle_lang::is_puzzle_source_path(&path) {
            let canonical_path = path.canonicalize()?;
            if !puzzle_sources_by_path.contains_key(&canonical_path) {
                return Err(AppError::Config(format!(
                    "workspace puzzle source was not indexed: {}",
                    path.display()
                )));
            }
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "text".to_string(),
                mime_type: mime_type(&path).to_string(),
                source: String::new(),
                data_url: String::new(),
                content_loaded: false,
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: import_graph
                    .imported_by
                    .get(&canonical_path)
                    .map(|paths| display_paths(paths))
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
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: Vec::new(),
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
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: Vec::new(),
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
        let workspace_path = canonical_path
            .strip_prefix(&workspace_root)
            .map_err(|_| {
                AppError::Config(format!(
                    "workspace puzzle path is outside its root: {}",
                    canonical_path.display()
                ))
            })?
            .to_string_lossy()
            .replace('\\', "/");
        documents.push(WorkspacePuzzleDocument {
            path: canonical_path,
            workspace_path,
            source,
        });
    }
    documents.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(documents)
}

fn build_workspace_import_graph(
    documents: &[WorkspacePuzzleDocument],
) -> Result<WorkspaceImportGraph, AppError> {
    let mut graph = WorkspaceImportGraph::default();
    let paths = documents
        .iter()
        .map(|document| (document.workspace_path.clone(), document.path.clone()))
        .collect::<HashMap<_, _>>();
    let workspace_documents = documents
        .iter()
        .map(|document| puzzle_lang::WorkspaceSourceDocument {
            path: document.workspace_path.clone(),
            source: document.source.clone(),
        })
        .collect::<Vec<_>>();
    let workspace = puzzle_lang::WorkspaceAnalysis::new(&workspace_documents)
        .map_err(|error| AppError::Config(error.to_string()))?;
    for document in &workspace.index().documents {
        let Some(path) = paths.get(&document.path).cloned() else {
            return Err(AppError::Config(format!(
                "workspace analysis returned an unknown document: {}",
                document.path
            )));
        };
        let importers = document
            .direct_importers
            .iter()
            .filter_map(|importer| paths.get(importer).cloned())
            .collect::<Vec<_>>();
        if !importers.is_empty() {
            graph.imported_by.insert(path.clone(), importers);
        }
    }
    Ok(graph)
}

fn display_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
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
        "puzzle" | "css" | "js" | "mjs" | "svg" | "json" | "txt" | "md"
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
        "puzzle" | "txt" | "md" => "text/plain",
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
        ("GET", "/editor_icons.js") => http_ok("text/javascript; charset=utf-8", EDITOR_ICONS_JS),
        ("GET", "/editor_codemirror.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_CODEMIRROR_JS)
        }
        ("GET", "/editor_runtime.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_RUNTIME_JS)
        }
        ("GET", "/editor_analysis_worker.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_ANALYSIS_WORKER_JS)
        }
        ("GET", "/editor_solver_worker.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_SOLVER_WORKER_JS)
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
        ("GET", "/editor_visual_document.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_VISUAL_DOCUMENT_JS)
        }
        ("GET", "/editor_visual.js") => http_ok("text/javascript; charset=utf-8", EDITOR_VISUAL_JS),
        ("GET", "/visual_tween_core.js") => {
            http_ok("text/javascript; charset=utf-8", VISUAL_TWEEN_CORE_JS)
        }
        ("GET", "/puzzle3_visual_core.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE3_VISUAL_CORE_JS)
        }
        ("GET", "/editor_visual3d.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_VISUAL3D_JS)
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
        ("GET", "/wasm_player/puzzle_wasm_player.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE_PLAYER_WASM_JS)
        }
        ("GET", "/wasm_player/puzzle_wasm_player_bg.wasm") => {
            http_bytes("application/wasm", PUZZLE_PLAYER_WASM_BG)
        }
        ("GET", "/renderer.js") => http_ok("text/javascript; charset=utf-8", RENDERER_JS),
        ("GET", "/render_asset_decoder.js") => {
            http_ok("text/javascript; charset=utf-8", RENDER_ASSET_DECODER_JS)
        }
        ("GET", "/editor_authoring_renderer.js") => http_ok(
            "text/javascript; charset=utf-8",
            EDITOR_AUTHORING_RENDERER_JS,
        ),
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
    if is_text_file(&canonical_requested) {
        let source = read_workspace_text_file(&canonical_requested, &workspace_root)?;
        let game_css = if puzzle_lang::is_puzzle_source_path(&canonical_requested) {
            load_game_css(&canonical_requested, &workspace_root)?
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
            preview_html: String::new(),
            preview_error: String::new(),
            game_css,
            imported_by,
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
        preview_html: String::new(),
        preview_error: String::new(),
        game_css: String::new(),
        imported_by,
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
    write_text_asset(
        output_dir,
        "editor_solver_worker.js",
        EDITOR_SOLVER_WORKER_JS,
    )?;
    write_text_asset(output_dir, "editor_boot.js", EDITOR_BOOT_JS)?;
    write_text_asset(output_dir, "editor_icons.js", EDITOR_ICONS_JS)?;
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
        "editor_visual_document.js",
        EDITOR_VISUAL_DOCUMENT_JS,
    )?;
    write_text_asset(output_dir, "editor_visual.js", EDITOR_VISUAL_JS)?;
    write_text_asset(output_dir, "editor_visual3d.js", EDITOR_VISUAL3D_JS)?;
    write_text_asset(output_dir, "editor_sounds.js", EDITOR_SOUNDS_JS)?;
    write_text_asset(output_dir, "editor_commands.js", EDITOR_COMMANDS_JS)?;
    write_text_asset(output_dir, "renderer.css", RENDERER_CSS)?;
    write_text_asset(output_dir, "renderer.js", RENDERER_JS)?;
    write_text_asset(
        output_dir,
        "render_asset_decoder.js",
        RENDER_ASSET_DECODER_JS,
    )?;
    write_text_asset(
        output_dir,
        "editor_authoring_renderer.js",
        EDITOR_AUTHORING_RENDERER_JS,
    )?;
    write_text_asset(output_dir, "visual_tween_core.js", VISUAL_TWEEN_CORE_JS)?;
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

    let player_wasm_dir = output_dir.join("wasm_player");
    fs::create_dir_all(&player_wasm_dir)?;
    fs::write(
        player_wasm_dir.join("puzzle_wasm_player.js"),
        PUZZLE_PLAYER_WASM_JS,
    )?;
    fs::write(
        player_wasm_dir.join("puzzle_wasm_player_bg.wasm"),
        PUZZLE_PLAYER_WASM_BG,
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
        id: "puzzle-block",
        title: "Puzzle Block",
        markdown: EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN,
    },
    EditorDocsPage {
        id: "layers",
        title: "Layers",
        markdown: EDITOR_DOCS_LAYERS_MARKDOWN,
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
        id: "visuals",
        title: "Visuals",
        markdown: EDITOR_DOCS_VISUALS_MARKDOWN,
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
        id: "visual-shapes",
        title: "Visual Shapes & Animation",
        markdown: EDITOR_DOCS_VISUAL_SHAPES_MARKDOWN,
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
        "start" | "puzzle-block" | "layers" | "legend" | "levels" | "rewrite-rules"
        | "input-rules" | "movement" | "win-conditions" | "visuals" => "Basic",
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
        "display" | "theme" | "rendering" | "visual-shapes" => Some("Visuals"),
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
            &puzzle_lang::highlight_source(source),
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
        if span.is_transparent() {
            out.push_str(" is-transparent");
        }
        out.push('"');
        if let Some(color) = span.color() {
            let property = if span.kind == puzzle_lang::SourceHighlightKind::VisualPixel {
                "--syntax-visual-pixel-color"
            } else {
                "--syntax-color-token"
            };
            out.push_str(" style=\"");
            out.push_str(property);
            out.push_str(": ");
            out.push_str(&escape_html(color.as_str()));
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
    use std::collections::{BTreeSet, HashSet};
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
    fn workbench_does_not_reference_removed_visual_mode_switch() {
        assert!(!EDITOR_WORKBENCH_JS.contains("visualPaneModeSwitch"));
        assert!(EDITOR_DOM_JS.contains(
            "const visualDimensionButtons = document.querySelectorAll(\"[data-visual-dimension]\");"
        ));
        assert!(EDITOR_DOM_JS.contains(
            "const visualPaneModeButtons = document.querySelectorAll(\"[data-visual-pane-mode]\");"
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
        let marker = "  const EDITOR_ICON_GEOMETRY = Object.freeze({\n";
        let start = EDITOR_ICONS_JS
            .find(marker)
            .expect("find shared editor icon registry")
            + marker.len();
        let end = EDITOR_ICONS_JS[start..]
            .find("\n  });")
            .map(|offset| start + offset)
            .expect("find shared editor icon registry end");
        EDITOR_ICONS_JS[start..end]
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

    fn collect_outline_kinds_from_source(source: &str, kinds: &mut BTreeSet<String>) {
        for item in puzzle_lang::source_outline(source) {
            kinds.insert(item.kind);
        }
    }

    fn collect_puzzle_fence_outline_kinds(markdown: &str, kinds: &mut BTreeSet<String>) {
        let mut in_puzzle_fence = false;
        let mut block = String::new();
        for line in markdown.lines() {
            let trimmed = line.trim_start();
            if in_puzzle_fence {
                if trimmed.starts_with("```") {
                    collect_outline_kinds_from_source(&block, kinds);
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
            r#"const title = "{title}"

puzzle default {{
layers {{
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

        let html = service
            .export_pages_editor_html()
            .expect("export managed Pages editor html");
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
    fn pages_example_compiles_without_a_title_constant() {
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

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("window.PuzzleRuntimeExportJson = "));
    }

    #[test]
    fn every_pages_starter_game_entry_compiles() {
        let workspace = TestWorkspace::new();
        for (path, source) in PAGES_STARTER_DOCUMENTS {
            if !puzzle_lang::is_puzzle_source_path(Path::new(path)) {
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

        let service = EditorService::open_path(&game_path).expect("open editor fixture");
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
            !state.base_game_visuals_js.contains("tile.svg"),
            "opening the editor must not compile preview-owned assets"
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
    fn open_defers_preview_import_failure_until_compile() {
        let workspace = TestWorkspace::new();
        let source = format!(
            "import missing = \"missing.puzzle\"\n\n{}",
            editor_fixture_source("Broken Import")
        );
        let game_path = workspace.write("games/broken_import/game.puzzle", source);
        let service =
            EditorService::open_path(&game_path).expect("open workspace with broken import");
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
            "games/puzzle3_editor_fixture/game.puzzle",
            include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle"),
        );
        workspace.write("games/puzzle3_editor_fixture/notes.md", "# Notes\n");

        let service = EditorService::open_path(&game_path).expect("open puzzle3 fixture");
        let state = service.state();

        let canonical_game_path = game_path.canonicalize().expect("canonical game path");
        assert_eq!(PathBuf::from(&state.puzzle_path), canonical_game_path);
        assert_eq!(
            PathBuf::from(&state.documents[0].puzzle_path),
            canonical_game_path
        );
        let document =
            document_with_suffix(&state.documents, "games/puzzle3_editor_fixture/game.puzzle");
        assert_eq!(document.mime_type, "text/plain");
        assert!(!document.content_loaded);
        assert!(document.source.is_empty());
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

        let service = EditorService::open_path(&game_path).expect("open editor fixture");
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
        assert!(loaded_source.contains("const title = \"Changed Title\""));
        assert!(!loaded_source.contains("const title = \"Original Title\""));
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
    fn workspace_preview_uses_the_explicitly_opened_puzzle_path() {
        let workspace = TestWorkspace::new();
        let entry_path = workspace.write(
            "games/custom_entry/arcade.puzzle",
            editor_fixture_source("ArcadeEntry"),
        );
        let fragment_path =
            workspace.write("games/custom_entry/fragments/levels.puzzle", "levels {}\n");
        let service = EditorService::open_path(&entry_path).expect("open custom project");
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
    fn workspace_import_graph_tracks_direct_importers_without_selecting_an_entry() {
        let workspace = TestWorkspace::new();
        let fragment_path =
            workspace.write("games/multiple_parents/shared/levels.puzzle", "levels {}\n");
        let game_path = workspace.write(
            "games/multiple_parents/game.puzzle",
            format!(
                "import shared = \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Game Parent")
            ),
        );
        workspace.write(
            "games/multiple_parents/main.puzzle",
            format!(
                "import shared = \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Main Parent")
            ),
        );
        workspace.write(
            "games/multiple_parents/third_parent.puzzle",
            format!(
                "import shared = \"shared/levels.puzzle\"\n\n{}",
                editor_fixture_source("Third Parent")
            ),
        );
        let project_dir = game_path.parent().expect("project dir");

        let service = EditorService::open_path(project_dir).expect("open multi-parent project");
        let state = service.state();
        let fragment_doc = document_with_suffix(
            &state.documents,
            "games/multiple_parents/shared/levels.puzzle",
        );

        assert_eq!(
            fragment_doc.imported_by.len(),
            3,
            "all direct importers should be visible to the editor"
        );
        assert_eq!(
            PathBuf::from(&fragment_doc.imported_by[0]),
            game_path.canonicalize().expect("canonical game path"),
            "direct importers have a stable path order"
        );
        assert_eq!(
            PathBuf::from(&fragment_path)
                .canonicalize()
                .expect("canonical fragment"),
            PathBuf::from(&fragment_doc.puzzle_path)
        );
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
    fn open_path_accepts_empty_project_folders() {
        let workspace = TestWorkspace::new();
        let project_dir = workspace.root.join("games/empty_project");
        fs::create_dir_all(&project_dir).expect("create empty project folder");
        fs::create_dir_all(project_dir.join("levels/empty")).expect("create nested empty folder");

        let service = EditorService::open_path(&project_dir).expect("open empty project");
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
    fn open_path_accepts_project_folders_without_puzzle_model() {
        let workspace = TestWorkspace::new();
        let fragment_path = workspace.write("games/fragments/levels.puzzle", "levels {}\n");
        workspace.write("games/fragments/notes.md", "# Notes\n");
        let project_dir = fragment_path.parent().expect("project dir");

        let service = EditorService::open_path(project_dir).expect("open non-entry project folder");
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
            "editor preview HTML must expose the source-free runtime export"
        );
        assert!(
            html.contains("window.PuzzleEditorPreviewExportJson = "),
            "editor preview HTML must expose its editor-owned metadata separately"
        );
        assert!(
            html.contains(r#"\"engine\""#),
            "editor preview metadata must include engine data for level editing"
        );
        assert!(!html.contains("PuzzleEditorSolverRulesJson"));
        assert!(!html.contains(r#"\"solver_strategy\""#));
        assert!(
            EDITOR_JS.contains(
                "extractAssignedStringLiteral(source, \"PuzzleEditorPreviewExportJson\")"
            ),
            "editor metadata extraction must read the editor preview contract"
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
    fn compile_preview_accepts_at_prefixed_object_single_color_visual() {
        let workspace = TestWorkspace::new();
        let source = r##"
const title = at_prefixed_object_single_color_preview

puzzle default {
layers {
@floor_slot = @Floor
}
visuals {
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
    fn compile_preview_accepts_line_style_tagged_visual_after_pattern() {
        let workspace = TestWorkspace::new();
        let source = r##"
const title = line_style_tagged_preview

puzzle default {
tags {
state = base movable
}
layers {
actor = Box:state
}
visuals {
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
        let game_path = workspace.write("games/tagged_visual/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open editor fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                service.state().game_css.clone(),
            ))
            .expect("compile tagged visual preview");

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("line_style_tagged_preview"));
    }

    #[test]
    fn compile_preview_preserves_language_diagnostics() {
        let workspace = TestWorkspace::new();
        let source = r#"
const title = "Multi Error Probe"

puzzle main {
layers {
base = Floor
}

visuals {
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
const title = "Multi Lifecycle Error Probe"

puzzle main {
layers {
actor = Player
}
layers {
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
const title = "Multi Statement Parse Error Probe"

puzzle main {
layers {
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
const title = "Sibling Statement Block Error Probe"

puzzle main {
layers {
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
        let source = include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle");
        let game_path = workspace.write("games/puzzle3_fixture/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open puzzle3 fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                String::new(),
            ))
            .expect("compile puzzle3 preview");

        assert!(html.contains("window.Puzzle3DFrameFixture"));
        assert!(html.contains("WasmStandaloneSession"));
        assert!(html.contains("window.Puzzle3Component"));
        assert!(!html.contains("sessionManaged"));
        assert!(html.contains("window.Puzzle3ThreeModuleSource = "));
        assert!(html.contains("window.Puzzle3ThreeRenderer"));
        assert!(html.contains("return text === \"canvas\" ? \"canvas\" : \"three\";"));
        assert!(html.contains("Microban 3D"));
    }

    #[test]
    fn preview_status_and_errors_follow_runtime_completion() {
        assert!(EDITOR_JS.contains(
            "appendPreviewLog(\"system\", \"Preview compiled\", { source: \"compiler\" });"
        ));
        assert!(EDITOR_JS.contains("setStatus(\"Starting preview\", \"\");"));
        assert!(EDITOR_JS.contains("event.data?.type === \"PuzzleStudioPreviewRuntimeReady\""));
        assert!(EDITOR_JS.contains("event.data?.type === \"PuzzleStudioPreviewRuntimeError\""));
        assert!(EDITOR_JS.contains("[headline, stack].filter(Boolean).join(\"\\\\n\")"));
        assert!(EDITOR_JS.contains(
            "window.PuzzleStudioPreviewRuntimeFailure && event.message === \"Script error.\""
        ));
        assert!(!EDITOR_JS.contains(
            "appendPreviewLog(\"system\", \"Preview ready\", { source: \"compiler\" });"
        ));
    }

    #[test]
    fn compile_preview_accepts_3d_input_rule_without_orientation_set() {
        let workspace = TestWorkspace::new();
        let source = r#"
const title = "Bare 3D Input"

puzzle push3 {
  dimension = 3

  layers {
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
        let game_path = workspace.write("games/puzzle3_input_rule/game.puzzle", source);
        let service = EditorService::open(&game_path).expect("open puzzle3 input fixture");

        let html = service
            .compile_preview(&PreviewRequest::new(
                source,
                game_path.display().to_string(),
                String::new(),
            ))
            .expect("compile puzzle3 input preview");

        assert!(html.contains("window.Puzzle3DFrameFixture"));
        assert!(html.contains("WasmStandaloneSession"));
        assert!(html.contains("Bare 3D Input"));
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
            "async function runPreviewFromSourcePane() {\n  selectPreviewEntryDocument(activeDocument());"
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
        assert!(EDITOR_JS.contains("\"workspace.save\","));
        assert!(EDITOR_JS.contains("editorCommandContext(null, previewFrame, \"button\")"));
    }

    #[test]
    fn pane_save_shortcuts_route_through_workbench_command_context() {
        assert!(
            EDITOR_WORKBENCH_JS
                .contains("function workbenchCommandContext(source = \"keyboard\", target = null)")
        );
        assert!(
            EDITOR_COMMANDS_JS.contains("const route = workbenchCommandContext(source, target);")
        );
        for id in ["workspace.save", "level.save", "visual.save", "sounds.save"] {
            assert!(EDITOR_COMMANDS_JS.contains(&format!("id: \"{id}\"")));
        }
        assert!(!EDITOR_COMMANDS_JS.contains("id: \"editor.save\""));
        assert!(!EDITOR_JS.contains("function handleToolPaneSaveShortcut(event)"));
    }

    #[test]
    fn editor_preview_dirty_status_stays_on_preview_pane() {
        assert!(EDITOR_JS.contains("let previewBuild = null;"));
        assert!(EDITOR_JS.contains("let previewBuildIsStale = false;"));
        assert!(EDITOR_JS.contains("previewBuildIsStale = true;"));
        assert!(!EDITOR_JS.contains("invalidateCompiledPreview(activePreviewDocument());"));
        assert!(EDITOR_JS.contains(
            r#"setPaneStatus("preview", previewBuild ? "Preview is out of date" : "Preview requires compile", "");"#
        ));
        assert!(!EDITOR_JS.contains(r#"setStatus("Preview requires compile", "");"#));
    }

    #[test]
    fn editor_keeps_last_preview_when_recompile_diagnostics_fail() {
        assert!(
            EDITOR_JS
                .contains("function invalidateCompiledPreview(document = activePreviewDocument())")
        );
        assert!(EDITOR_JS.contains("appendCompileDiagnostics(error, { source: \"compiler\", document, sourceText: requestSource });"));
        let compile_failure = EDITOR_JS
            .split("appendCompileDiagnostics(error, { source: \"compiler\", document, sourceText: requestSource });")
            .nth(1)
            .expect("preview compile failure branch");
        let compile_failure = compile_failure
            .split("} finally {")
            .next()
            .expect("preview compile failure branch end");
        assert!(!compile_failure.contains("invalidateCompiledPreview"));
        assert!(EDITOR_JS.contains(r#"applyGameCss("");"#));
        assert!(EDITOR_JS.contains(r#"applyGameVisuals("");"#));
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
    fn source_switch_preserves_same_preview_target_and_recompiles_a_new_target() {
        let load_document = EDITOR_WORKSPACE_JS
            .split("function loadEmbeddedDocument(index) {")
            .nth(1)
            .and_then(|tail| tail.split("\nfunction loadFolderPreview").next())
            .expect("loadEmbeddedDocument source");
        assert!(load_document.contains("const previousPreviewKey = loadedPreviewTargetKey;"));
        assert!(load_document.contains(
            "const previewTargetUnchanged = previewDocument\n    && previousPreviewKey\n    && previewTargetKey === previousPreviewKey;"
        ));
        assert!(load_document.contains("const previewTargetChanged = activeSourceChanged"));
        assert!(load_document.contains(
            "if (previewTargetChanged) {\n    invalidateCompiledPreview(previewDocument);\n  } else if (previewTargetUnchanged) {\n    syncPreviewLevelActionButtons();"
        ));
        assert!(load_document.contains("if (previewTargetChanged) {"));
        assert!(load_document.contains("renderPreview().catch((error) => {"));
        let preview_compile_schedule = load_document
            .find("Promise.resolve().then(() => {")
            .expect("preview compilation schedule");
        let source_editor_load = load_document
            .find("setSourceEditorValue(sourceText, {")
            .expect("source editor load");
        let level_editor_reset = load_document
            .find("resetLevelBuilderFromPreviewSource();")
            .expect("level editor reset");
        assert!(preview_compile_schedule < source_editor_load);
        assert!(preview_compile_schedule < level_editor_reset);
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
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="file-plus""#));
        assert!(EDITOR_HTML.contains("ps-import-output-actions"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function resetPuzzleScriptImportConversion()"));
        assert!(
            EDITOR_IMPORT_EXPORT_JS.contains("function schedulePuzzleScriptImportConversion()")
        );
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function puzzleScriptSourceTitle(source)"));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains("puzzleStudioMetadataTitle"));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains(r#"/^title\s*=\s*(.+)$/"#));
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
        assert!(EDITOR_CSS.contains("#psImportStatus,\n#levelSolveStatus"));
        assert!(!EDITOR_CSS.contains(".ps-import-output-field textarea"));
    }

    #[test]
    fn general_icon_controls_share_hover_focus_and_active_states() {
        let icon_primitive = EDITOR_CSS
            .find(".icon-button {")
            .expect("shared icon primitive");
        let pane_restore = EDITOR_CSS
            .find(".pane-restore-button {")
            .expect("pane restore visibility owner");
        assert!(icon_primitive < pane_restore);
        assert!(EDITOR_CSS.contains("button.icon-button:hover:not(:disabled),"));
        assert!(EDITOR_CSS.contains("button.icon-button:focus-visible"));
        assert!(EDITOR_CSS.contains("button.icon-button[aria-pressed=\"true\"]"));
        assert!(EDITOR_CSS.contains(".editor-hover-tooltip[hidden] {\n  display: none;"));
        assert!(EDITOR_CSS.contains("outline: 2px solid color-mix"));
        assert!(!EDITOR_CSS.contains(":is(\n  .icon-button,"));

        let icon_roles = [
            "pane-restore-button",
            "pane-close-button",
            "pane-maximize-button",
            "tree-action-button",
            "pane-header-icon-button",
            "document-tab-close",
            "source-find-icon-button",
            "preview-debug-control-button",
            "source-action-button",
            "solution-copy-button",
            "solution-step-button",
            "visual-icon-button",
            "visual-scale-step-button",
            "visual-bind-toggle",
            "visual-color-action-button",
            "sounds-icon-button",
        ];
        for source in [
            EDITOR_HTML,
            EDITOR_JS,
            EDITOR_SOURCE_JS,
            EDITOR_LEVEL3D_JS,
            EDITOR_WORKSPACE_JS,
            EDITOR_WORKBENCH_JS,
            EDITOR_VISUAL_JS,
            EDITOR_VISUAL3D_JS,
        ] {
            for line in source
                .lines()
                .filter(|line| line.contains("class=\"") || line.contains("className ="))
            {
                if icon_roles.iter().any(|role| line.contains(role)) {
                    assert!(
                        line.contains("icon-button"),
                        "icon control must use the shared class: {line}"
                    );
                }
            }
        }
    }

    #[test]
    fn repeated_control_states_use_semantic_shared_owners() {
        assert!(EDITOR_CSS.contains(".option-button:hover,"));
        assert!(EDITOR_CSS.contains(".navigation-row:hover,"));
        assert!(EDITOR_CSS.contains(".accent-splitter:hover,"));
        assert!(EDITOR_CSS.contains("button.icon-button.is-danger:hover:not(:disabled),"));
        assert!(EDITOR_CSS.contains(".visual-duration-input:focus-within {"));
        assert!(EDITOR_CSS.contains(".visual3d-camera-scrub.is-dragging {"));

        assert!(!EDITOR_CSS.contains(".source-level-name-option:hover,"));
        assert!(!EDITOR_CSS.contains(".tree-row:hover {"));
        assert!(!EDITOR_CSS.contains(".visual-current-tag-unlink-button:hover:not(:disabled),"));
        assert!(!EDITOR_CSS.contains(".preview-log-splitter"));
        assert!(!EDITOR_CSS.contains(".visual3d-clear-button"));

        assert!(EDITOR_WORKSPACE_JS.contains("option-button explorer-empty-recent-button"));
        assert!(EDITOR_WORKSPACE_JS.contains("navigation-row tree-row"));
        assert!(EDITOR_SOURCE_JS.contains("navigation-row source-outline-row"));
        assert!(EDITOR_SOURCE_JS.contains("option-button source-level-name-option"));
        assert!(EDITOR_HTML.contains("accent-splitter explorer-splitter"));
        assert!(EDITOR_VISUAL_JS.contains("icon-button is-danger"));
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
            .find("setEditorStatus(`Opened in ${folderName}`, \"is-ok\");")
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
        assert!(EDITOR_JS.contains(
            "order: {\n            direction_priority: [...(config.order?.direction_priority || [])],\n            priorities: [...(config.order?.priorities || [])],\n          },"
        ));
        assert!(EDITOR_JS.contains("animations: { ...(config.animations || {}) },"));
        assert!(EDITOR_JS.contains("triggers: { ...(config.triggers || {}) },"));
        assert!(EDITOR_JS.contains("animationDefaults: { ...(config.animationDefaults || {}) },"));
        assert!(EDITOR_JS.contains("  Function(script)();\n}"));
        assert!(!EDITOR_JS.contains(
            "window.PuzzleStudio.disposeAssetScripts();\n    window.GameVisuals = window.PuzzleVisualRegistry.create();\n    console.error(error);"
        ));
        assert!(EDITOR_JS.contains("label.textContent = `Layer ${index + 1}`;"));
        assert!(EDITOR_JS.contains("levelLayerPreviewStrip.replaceChildren(fragment);"));
        assert!(EDITOR_CSS.contains(".level-board.board.has-all-cell-grid .cell::after"));
        assert!(EDITOR_CSS.contains("z-index: 100;"));
    }

    #[test]
    fn level_editor_board_uses_continuous_visual_checkerboard_background() {
        assert!(EDITOR_CSS.contains("--visual-swatch-checker: url("));
        assert!(EDITOR_CSS.contains(
            ".level-board.board {\n  background-color: var(--visual-swatch-bg);\n  background-image: var(--visual-swatch-checker);\n  background-size: 8px 8px;\n  box-shadow:"
        ));
        assert!(
            !EDITOR_CSS.contains(
                ".level-board.board .cell {\n  background-color: var(--visual-swatch-bg);"
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
        assert!(EDITOR_JS.contains(
            "function stateDataToLevelCells(stateData, exportData = previewBuild?.exportData)"
        ));
        assert!(!EDITOR_JS.contains("function transitionPlaytestProgram("));
        assert!(!EDITOR_JS.contains("function levelPlaytestCoreRuntime("));
        assert!(!EDITOR_JS.contains("function applyLevelPlaytestKey(event)"));
        assert!(!EDITOR_JS.contains("WasmCompiledCoreRuntime"));
        assert!(!EDITOR_JS.contains("transition_current_outcome"));
        assert!(!EDITOR_JS.contains("function levelPlaytestCommandForKey(event)"));
        assert!(!EDITOR_JS.contains("function levelPlaytestInputForKey(event"));
        assert!(!EDITOR_JS.contains("pendingPreviewKeyStateSync"));
        assert!(!EDITOR_JS.contains("code === \"KeyZ\""));
        assert!(!EDITOR_JS.contains("code.startsWith(\"Key\")"));
        assert!(EDITOR_JS.contains(r#"type: "PuzzleStudioKey","#));
        assert!(EDITOR_JS.contains("code: event.code"));
        assert!(EDITOR_JS.contains("repeat: event.repeat"));
        assert!(EDITOR_JS.contains("altKey: event.altKey"));
        assert!(EDITOR_JS.contains("ctrlKey: event.ctrlKey"));
        assert!(EDITOR_JS.contains("metaKey: event.metaKey"));
        assert!(EDITOR_JS.contains("shiftKey: event.shiftKey"));
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
        let send_key_start = EDITOR_LEVEL3D_JS
            .find("function sendLevel3dPlaytestKey(event) {")
            .unwrap();
        let send_key_end = EDITOR_LEVEL3D_JS[send_key_start..]
            .find("function handleLevel3dPlaytestStateMessage(event) {")
            .map(|offset| send_key_start + offset)
            .unwrap();
        let send_key_body = &EDITOR_LEVEL3D_JS[send_key_start..send_key_end];
        assert!(send_key_body.contains("type: \"PuzzleStudioKey\""));
        assert!(!send_key_body.contains("PuzzleStudioCommand"));
        assert!(send_key_body.contains("repeat: event.repeat"));
        assert!(send_key_body.contains("altKey: event.altKey"));
        assert!(send_key_body.contains("ctrlKey: event.ctrlKey"));
        assert!(send_key_body.contains("metaKey: event.metaKey"));
        assert!(send_key_body.contains("shiftKey: event.shiftKey"));
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
        assert!(EDITOR_CSS.contains("background-color: var(--visual-swatch-bg);"));
        assert!(EDITOR_CSS.contains("background-image: var(--visual-swatch-checker);"));
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
        assert!(EDITOR_CSS.contains(".level3d-layer-toolbar.visual3d-slice-axis-control {"));
        assert!(EDITOR_CSS.contains("background: rgb(34 38 44 / 88%);"));
        assert!(EDITOR_CSS.contains("box-shadow: 0 2px 10px rgb(0 0 0 / 35%);"));
        assert!(EDITOR_CSS.contains(".level3d-layer-toolbar .visual3d-layer-axis-label,"));
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
            "function previewLevelIndexForSourceEntry(entry, exportData = previewBuild?.exportData)"
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
        assert!(EDITOR_JS.contains("previewBuild = null;"));
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
                "if (levelPlaytestActive && !previewBuildIsStale && exportData === currentPreviewExportData()) {"
            )
        );
        assert!(EDITOR_JS.contains(
            "function activePreviewModeAcceptsLevelState() {\n  return currentPreviewMode === \"edit\" && levelPlaytestActive;\n}"
        ));
        assert!(EDITOR_JS.contains("function levelEditorSourceExportData(source)"));
        assert!(EDITOR_RUNTIME_JS.contains("levelEditorSourceSession(source)"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_manifest_json"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_level_slots"));
        assert!(EDITOR_RUNTIME_JS.contains("active_source_analysis_level_editor_visual_json"));
        assert!(!EDITOR_JS.contains("loadLevelSourceEntryAfterPreviewCompile"));
        assert!(!EDITOR_JS.contains("applyLevelEditorContractVisuals"));
        assert!(EDITOR_JS.contains("session.levelSlots(levelIndex, authoredLayer)"));
        assert!(!EDITOR_JS.contains("function levelEditorRuntimeVisual("));
        assert!(EDITOR_JS.contains("if (exportData.editorSourceContract) {"));
        assert!(EDITOR_JS.contains("stateDataToEditorCells(integrated.initialState, exportData)"));
        assert!(!EDITOR_JS.contains("async function compileSolverPreviewData()"));
        assert!(EDITOR_JS.contains("function prepareEditorSolverArtifact("));
        assert!(EDITOR_SOLVER_WORKER_JS.contains("new module.WasmSolverService()"));
        assert!(!EDITOR_JS.contains("function expandPuzzleImportsForPreviewRequest("));
        assert!(!EDITOR_JS.contains("function expandPuzzleImportsForWasm("));
        assert!(!EDITOR_WORKSPACE_JS.contains("function ensurePuzzleImportDocumentsLoaded("));
        assert!(
            EDITOR_JS
                .contains("workspaceDocuments: compilerDocumentsForSnapshot(buildInput.documents)")
        );
        assert!(EDITOR_SOLVER_WORKER_JS.contains("service.prepare_workspace("));
        assert!(EDITOR_JS.contains("function loadLevelSourceEntryWithExportData("));
        assert!(EDITOR_JS.contains("function reportLevelSourceLoadFailure("));
        assert!(!EDITOR_JS.contains("const artifacts = new Map();"));
        assert!(!EDITOR_SOLVER_WORKER_JS.contains("JSON.stringify"));
        assert!(!EDITOR_SOLVER_WORKER_JS.contains("JSON.parse"));
        assert!(EDITOR_JS.contains("window.PuzzleEditorPreviewExportJson"));
        assert!(EDITOR_JS.contains(
            "const editorPreviewExportLiteral = extractAssignedStringLiteral(source, \"PuzzleEditorPreviewExportJson\");"
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
        assert!(EDITOR_JS.contains("currentLevelSourceLocation({ sourceScope: \"workspace\" })"));
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
            "for (const entry of surfaceEntriesForSource(source).filter((candidate) => sourceTargetMatches(candidate, \"level\", \"2d\")))"
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
        assert!(
            EDITOR_RUNTIME_JS
                .contains("return querySynchronizedAnalysisWorker(\"outline\", source);")
        );
        assert!(!EDITOR_SOURCE_JS.contains("sourceProfile"));
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
        assert!(EDITOR_SOURCE_JS.contains("syncSourceAnalysisEditorChanges(sourceChanges"));
        assert!(EDITOR_CODEMIRROR_JS.contains("sourceanalysisreset"));
        assert!(EDITOR_RUNTIME_JS.contains("async sourceEntries(source)"));
        assert!(EDITOR_RUNTIME_JS.contains("async sourceEntryInfo(source)"));
        assert!(
            EDITOR_RUNTIME_JS
                .contains("await querySynchronizedAnalysisWorker(\"entries\", asString(source))")
        );
        assert!(EDITOR_RUNTIME_JS.contains("payload: null,"));
        assert!(!EDITOR_RUNTIME_JS.contains(
            "const raw = querySourceAnalysis(module, revision, \"active_source_analysis_json\");"
        ));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime?.sourceEntryInfo"));
        assert!(EDITOR_JS.contains("await loadSurfaceEntriesForSource(context.source"));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime.sourceEntryInfo(text)"));
        assert!(!EDITOR_JS.contains("declaresGameEntry"));
        assert!(!EDITOR_WORKSPACE_JS.contains("declaresGameEntry"));
        assert!(!EDITOR_WORKSPACE_JS.contains("parentGamePath"));
        assert!(
            EDITOR_WORKSPACE_JS.contains("const previewEntryDocumentIdByWorkspace = new Map();")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("function selectPreviewEntryDocument(document)"));
        assert!(EDITOR_WORKSPACE_JS.contains("function previewEntryDocumentForWorkspace(root)"));
        assert!(!EDITOR_JS.contains("previewTargetKey()"));
        assert!(!EDITOR_WORKSPACE_JS.contains("function sourceDeclaresGameEntry("));
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
        assert!(EDITOR_JS.contains(
            "return uniqueFocusedPuzzleEntries(focusedPuzzleSurfaceEntriesByKind(kind, context))"
        ));
        assert!(
            !EDITOR_JS
                .contains("return [];\n  }\n  const raw = compiler.source_entries_json(text);")
        );
        assert!(!EDITOR_JS.contains("compiler.source_entries_json(text)"));
        assert!(EDITOR_JS.contains(
            "focusedPuzzleSurfaceEntriesByKind(\"level\", { document, source }, \"2d\")"
        ));
        assert!(EDITOR_JS.contains(
            "focusedPuzzleSurfaceEntriesByKind(\"level\", { document: activeDocument(), source }, \"3d\")"
        ));
        assert!(EDITOR_JS.contains(
            "focusedPuzzleSurfaceEntriesByKind(\"visual\", { document: activeDocument(), source }, \"2d\")"
        ));
        assert!(EDITOR_JS.contains(
            "focusedPuzzleSurfaceEntriesByKind(\"visual\", { document: activeDocument(), source }, \"3d\")"
        ));
        assert!(!EDITOR_JS.contains("sourceVisual3dTargetAtPosition("));
        assert!(!EDITOR_JS.contains("const visual3dTarget = sourceVisual3dTargetAtPosition"));
        assert!(!EDITOR_JS.contains("for (const range of findLevelsRanges(source) || []) {\n      if (sourcePositionInsideRanges(range.start, level3dRanges))"));
        assert!(!EDITOR_JS.contains("for (const range of findLevels3Ranges(source) || []) {\n    entries.push(...(findLevel3dDefinitions(source, range)"));
        assert!(
            !EDITOR_JS.contains("entries.push(...(findVisual3dDefinitions(source, block) || []));")
        );
    }

    #[test]
    fn editor_dimension_follows_canonical_source_products() {
        assert!(!EDITOR_WORKSPACE_JS.contains("puzzleSourceProfile"));
        assert!(!EDITOR_JS.contains("function editorDimensionForDocument("));
        assert!(!EDITOR_JS.contains("editorDimensionForPuzzleSourceProfile("));
        assert!(EDITOR_JS.contains("dimension: entry.dimension,"));
        assert!(EDITOR_JS.contains(
            "throw new Error(`Source ${kind} entry is missing its canonical dimension.`);"
        ));
        assert!(EDITOR_JS.contains(".filter((item) => item.dimension === normalized);"));
        assert!(EDITOR_WORKSPACE_JS.contains(
            "void syncPaneModesFromFocusedPuzzleSource({ switchOpenPane: true, loadFirst: false })"
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
    fn editor_compile_diagnostic_links_target_the_diagnostic_document() {
        assert!(
            EDITOR_JS.contains("const diagnosticFile = String(diagnostic?.file || \"\").trim();")
        );
        assert!(EDITOR_JS.contains("? documentByPath(diagnosticFile)"));
        assert!(EDITOR_JS.contains("if (diagnosticFile && !document) {\n    return null;\n  }"));
        assert!(EDITOR_JS.contains("? currentSourceForDocument(document)"));
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
    fn source_add_control_overlays_the_active_empty_line_without_a_gutter_column() {
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("class SourceAddLineWidget extends WidgetType")
        );
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("Decoration.widget({"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("button.innerHTML = editorIconSvg(\"plus\")"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("anchor.append(button);"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("width: \"0\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("verticalAlign: \"top\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("lineHeight: \"inherit\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("width: \"1lh\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("height: \"1lh\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("height: \"100%\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("minHeight: \"0\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("flex: \"none\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("width: \"1em\""));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("opacity: \"0.55\""));
        assert!(!EDITOR_CODEMIRROR_SOURCE_JS.contains("cm-source-add-gutter"));
        assert!(EDITOR_SOURCE_JS.contains("source.slice(lineStart, lineEnd).trim() === \"\""));
        assert!(EDITOR_SOURCE_JS.contains("document.activeElement !== sourceEditor"));
        assert!(
            EDITOR_SOURCE_JS.contains("setSourceLineAddVisible(source, cursor, items.length > 0);")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.addEventListener(\"sourceselectionchange\", () => {")
        );
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("if (update.selectionSet) {"));
        assert!(EDITOR_SOURCE_JS.contains("sourceEditor.sourceEditorPort.setAddLineOverlay("));
        assert!(
            EDITOR_SOURCE_JS.contains("sourceEditor.addEventListener(\"sourcelineaddrequest\"")
        );
    }

    #[test]
    fn codemirror_highlight_consumes_typed_rust_spans_as_decorations() {
        let payload = EditorService::highlight_source_json("const title = \"Demo\"\n");
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
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("foldService.of(sourceFoldRangeForLine)"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("foldGutter({"));
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("...foldKeymap"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.sourceEditorPort.applyFoldRanges(source, payload);")
        );
        assert!(!EDITOR_CODEMIRROR_SOURCE_JS.contains("sourceFoldableBlocks"));
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
        assert!(EDITOR_JS.contains("function previewStateMatchesSolverTask("));
        assert!(EDITOR_JS.contains("function applyPreviewSceneToActiveSolverTask("));
        assert!(EDITOR_JS.contains("activeSolverTask.scene = cloneJson(previewState.scene);"));
        assert!(EDITOR_JS.contains("function refreshActiveSolverTaskDisplayScene("));
        assert!(EDITOR_JS.contains("materializeEditorSolverState(activeSolverTask)"));
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
        assert!(EDITOR_JS.contains("refreshVisiblePreviewSolverTask(previewBuild?.exportData);"));
        assert!(EDITOR_JS.contains("if (!activeSolverTask && currentPreviewMode === \"solver\")"));
        assert!(EDITOR_JS.contains("function syncSolverLevelSelector("));
        assert!(EDITOR_JS.contains("function selectSolverLevel("));
        assert!(EDITOR_JS.contains("solverSelectedLevelIndex = levelIndex;"));
        assert!(!EDITOR_JS.contains("const levelIndex = setActiveLevelIndex(index, exportData);"));
        assert!(
            EDITOR_JS.contains("const task = createPreviewSolverTask(previewBuild, levelIndex);")
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
        assert!(open_solver_source.contains("await ensurePreviewSolverBuild();"));
        assert!(EDITOR_JS.contains("async function ensurePreviewSolverBuild()"));
        assert!(EDITOR_JS.contains("await prepareEditorSolverArtifact({"));
        assert!(!EDITOR_JS.contains("if (isPuzzle3dExport(exportData)) return exportData;"));
        assert!(EDITOR_JS.contains("const prepared = solverBuild?.solverPrepared;"));
        assert!(EDITOR_JS.contains("prepared.modelKind !== modelKind"));
        assert!(!EDITOR_JS.contains("task.rules.loadedGame"));
        assert!(!EDITOR_JS.contains("task.rules.compiledPlay"));
        assert!(EDITOR_JS.contains("status(\"Preparing solver\", \"\");"));
        assert!(EDITOR_JS.contains("const solverPreparedByBuildId = new Map();"));
        assert!(EDITOR_JS.contains("solverPreparedByBuildId.set(build.id, prepared);"));
        assert!(!EDITOR_JS.contains("build.solverPrepared = prepared;"));
        assert!(EDITOR_JS.contains("documents: compilerDocumentsForSnapshot(build.documents),"));
        assert!(!open_solver_source.contains("renderPreview()"));
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
        assert!(EDITOR_JS.contains("function puzzle3dSnapshotForActiveSolverTask("));
        assert!(EDITOR_SOLVER_WORKER_JS.contains("service.start("));
        assert!(EDITOR_SOLVER_WORKER_JS.contains("service.advance("));
        assert!(EDITOR_SOLVER_WORKER_JS.contains("service.cancel("));
        assert!(EDITOR_JS.contains("if (isSolverTaskComplete(task))"));
        assert!(EDITOR_JS.contains(
            "setLevelSolveStatus(\"This level has already been solved\", \"is-error\");"
        ));
        assert!(EDITOR_JS.contains("markActiveSolverTaskComplete();"));
        assert!(EDITOR_JS.contains("button.disabled = taskComplete || previewHasNoLevel;"));
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
            EDITOR_JS.contains("isPuzzle3dExport(exportData) && typeof renderSolverRuntimePreview")
        );
    }

    #[test]
    fn editor_solver_uses_its_product_budget() {
        let solver_request = EDITOR_JS
            .find("function solverRequestForTask(task)")
            .expect("solver request builder");
        let solver_request_end = EDITOR_JS[solver_request..]
            .find("\n}\n\nasync function solveEditedLevelFromEditor()")
            .map(|index| solver_request + index)
            .expect("solver request builder end");
        let solver_request_source = &EDITOR_JS[solver_request..solver_request_end];
        assert!(solver_request_source.contains("maxStoredNodes: 5_000_000,"));
        assert!(!solver_request_source.contains("maxStoredNodes: 1000,"));
        assert!(
            EDITOR_SOLVER_WORKER_JS
                .contains("maxStoredNodes: Number(request.maxStoredNodes),")
        );
        assert!(!EDITOR_SOLVER_WORKER_JS.contains("maxNodes:"));
    }

    #[test]
    fn editor_solver_state_omits_transition_scratch_marks() {
        assert!(!EDITOR_SOLVER_WORKER_JS.contains("slotMarks"));
        assert!(!EDITOR_SOLVER_WORKER_JS.contains("cellMarks"));
        assert!(!EDITOR_JS.contains("slotMarks:"));
        assert!(!EDITOR_JS.contains("cellMarks:"));
    }

    #[test]
    fn preview_solve_uses_runtime_current_level() {
        assert!(EDITOR_JS.contains("function previewSolverTaskLevelIndex("));
        assert!(EDITOR_JS.contains("const levelIndex = previewSolverTaskLevelIndex(exportData);"));
        assert!(EDITOR_JS.contains("const state = previewSessionState();"));
        assert!(EDITOR_JS.contains("state.screenHasPuzzle !== false"));
        assert!(EDITOR_JS.contains("Number.isInteger(Number(state.levelIndex))"));
        assert!(EDITOR_JS.contains(
            "return normalizedLevelIndex(Math.trunc(Number(state.levelIndex)), exportData);"
        ));
        assert!(
            EDITOR_JS
                .contains("return normalizedLevelIndex(solverSelectedLevelIndex, exportData);")
        );
        assert!(EDITOR_JS.contains("return null;"));
        let resolver = EDITOR_JS
            .split("function previewSolverTaskLevelIndex(")
            .nth(1)
            .expect("preview solver level resolver");
        let runtime_level = resolver
            .find("state.levelIndex")
            .expect("runtime level branch");
        let selected_level = resolver
            .find("solverSelectedLevelIndex")
            .expect("selected level branch");
        assert!(runtime_level < selected_level);
        assert!(
            !resolver[..resolver.find("\n}").expect("resolver end")]
                .contains("currentEditableLevelIndex")
        );
    }

    #[test]
    fn preview_edit_opens_runtime_current_level() {
        assert!(EDITOR_HTML.contains(r#"id="previewEditButton""#));
        assert!(EDITOR_HTML.contains(
            r#"id="previewEditButton" class="icon-button pane-header-icon-button preview-edit-button" type="button" aria-label="Edit level" title="Edit level" disabled"#
        ));
        assert!(EDITOR_HTML.contains(
            r#"id="previewSolveButton" class="icon-button pane-header-icon-button preview-solve-button" type="button" aria-label="Solve" title="Solve" disabled"#
        ));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="pencil""#));
        assert!(EDITOR_DOM_JS.contains("const previewEditButton = document.querySelector"));
        assert!(EDITOR_JS.contains("function currentPreviewRuntimeLevelIndex("));
        assert!(EDITOR_JS.contains("function currentLevel3dSourceLocationForIndex("));
        assert!(EDITOR_JS.contains("function openLevelPaneForCurrentPreviewLevel("));
        assert!(
            EDITOR_JS.contains("const levelIndex = currentPreviewRuntimeLevelIndex(exportData);")
        );
        assert!(EDITOR_JS.contains("setActiveLevelIndex(levelIndex, exportData);"));
        assert!(
            EDITOR_JS.contains(
                "currentLevel3dSourceLocationForIndex(levelIndex, exportData, { build })"
            )
        );
        assert!(
            EDITOR_JS.contains("currentLevelSourceLocation({ build, exportData, levelIndex });")
        );
        assert!(EDITOR_JS.contains("previewSession?.buildId !== build.id"));
        assert!(
            EDITOR_JS.contains("const sourceDocuments = sourceDocumentsForLevelLocation(options);")
        );
        assert!(EDITOR_JS.contains("return sourceDocumentsForPreviewBuild(options.build);"));
        assert!(EDITOR_JS.contains("Preview build is missing its source snapshot."));
        assert!(EDITOR_JS.contains("source: snapshot.source,"));
        assert!(
            EDITOR_JS
                .contains("currentSourceForDocument(target.document) !== target.sourceSnapshot")
        );
        assert!(
            EDITOR_JS.contains("Preview source changed. Run Preview before editing this level.")
        );
        assert!(EDITOR_JS.contains("kind: \"level\","));
        assert!(EDITOR_JS.contains("dimension: targetMode === \"level3d\" ? \"3d\" : \"2d\","));
        assert!(EDITOR_JS.contains("previewEditButton?.addEventListener(\"click\""));
        assert!(EDITOR_JS.contains("openLevelPaneForCurrentPreviewLevel();"));
        assert!(EDITOR_JS.contains("requestFocusedPreviewState();"));
    }

    #[test]
    fn preview_debug_mode_uses_lucide_icon_and_runtime_trace_contract() {
        assert!(!EDITOR_HTML.contains(r#"id="previewDebugToolbar""#));
        assert!(EDITOR_HTML.contains(r#"id="previewDebugToggleButton""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Debug""#));
        assert!(EDITOR_HTML.contains(
            r#"class="icon-button pane-header-icon-button preview-debug-toggle-button""#
        ));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="bug""#));
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
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="maximize""#));
        assert!(
            EDITOR_WORKBENCH_JS
                .contains("return editorIconSvg(isRestore ? \"minimize\" : \"maximize\");")
        );
        assert!(EDITOR_ICONS_JS.contains(r#""maximize": `"#));
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
        assert!(EDITOR_SOURCE_JS.contains("editorIconSvg(sourceOutlineKindIconName(kind)"));
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
    fn editor_ui_icons_use_one_shared_lucide_geometry_registry() {
        assert!(EDITOR_ICONS_JS.contains("lucide-static@1.24.0"));
        assert!(EDITOR_ICONS_JS.contains("const EDITOR_ICON_GEOMETRY = Object.freeze({"));
        assert!(EDITOR_ICONS_JS.contains("throw new Error(`Unknown editor Lucide icon: ${name}`)"));
        assert_eq!(
            EDITOR_HTML.matches("<path").count(),
            1,
            "only the brand mark stays inline"
        );
        assert!(EDITOR_HTML.contains(r#"<script src="editor_icons.js"></script>"#));
        assert!(EDITOR_ICONS_JS.contains("hydrateEditorIcons();"));
        assert!(
            EDITOR_HTML.find("editor_icons.js").unwrap()
                < EDITOR_HTML.find("editor_codemirror.js").unwrap()
        );
        for source in [
            EDITOR_CODEMIRROR_SOURCE_JS,
            EDITOR_WORKSPACE_JS,
            EDITOR_SOURCE_JS,
            EDITOR_LEVEL3D_JS,
            EDITOR_WORKBENCH_JS,
            EDITOR_JS,
            EDITOR_VISUAL_JS,
            EDITOR_VISUAL3D_JS,
            EDITOR_SOUNDS_JS,
        ] {
            assert!(
                !source.contains("<svg"),
                "editor UI geometry must stay in editor_icons.js"
            );
        }
    }

    #[test]
    fn source_outline_icon_mapping_covers_canonical_examples() {
        let kind_icons = js_object_string_map(EDITOR_SOURCE_JS, "SOURCE_OUTLINE_KIND_ICON_NAMES");
        let mut kinds = BTreeSet::new();

        for markdown in [
            EDITOR_DOCS_MARKDOWN,
            EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN,
            EDITOR_DOCS_LAYERS_MARKDOWN,
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
            EDITOR_DOCS_VISUALS_MARKDOWN,
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
            EDITOR_DOCS_VISUAL_SHAPES_MARKDOWN,
            EDITOR_DOCS_SCENE_STATE_EFFECTS_MARKDOWN,
            EDITOR_DOCS_MAPS_EXPANSION_MARKDOWN,
        ] {
            collect_puzzle_fence_outline_kinds(markdown, &mut kinds);
        }
        collect_puzzle_fence_outline_kinds(EDITOR_DOCS_3D_MARKDOWN, &mut kinds);

        for source in [
            r#"
const title = "Outline 2D"

puzzle outline {
layers {
actor = Player
}

rules {
input [ Player ] -> [ > Player ]
move
}
}
"#,
            include_str!("../../lang/tests/fixtures/spec_3d_full.puzzle"),
            include_str!("../../lang/tests/fixtures/spec_3d_preview_contract.puzzle"),
            include_str!("../../lang/tests/fixtures/puzzlescript/basic_sokoban.puzzle"),
        ] {
            collect_outline_kinds_from_source(source, &mut kinds);
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
        assert!(EDITOR_JS.contains("let previewBuild = null;"));
        assert!(EDITOR_JS.contains(
            "const previousLevelIndex = previewBuild?.documentId === buildInput.documentId\n    ? currentPreviewRuntimeLevelIndex(previewBuild?.exportData)\n    : null;"
        ));
        assert!(EDITOR_JS.contains("previewBuild = {\n    ...buildInput,"));
        assert!(EDITOR_JS.contains(
            "setActiveLevelIndex(previousLevelIndex ?? exportData?.initialLevelIndex ?? 0, exportData);"
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
        assert!(EDITOR_JS.contains("const tracksSource = kind === \"level3d\" || kind === \"visual\" || kind === \"visual3d\";"));
        assert!(!EDITOR_JS.contains("nextExport.levels[levelIndex].initialState = stateData"));
        assert!(!EDITOR_JS.contains("previewMode === \"play\" && wasLevelMode"));
        assert!(EDITOR_JS.contains("let previewFrameHasEditorLevelState = false;"));
        assert!(EDITOR_JS.contains("function restoreCompiledGamePreview()"));
        assert!(EDITOR_JS.contains("if (previewMode === \"play\")"));
        assert!(EDITOR_JS.contains(
            "setPreviewFrameHtml(editorPreviewDocument(previewBuild?.html), { markDocumentLoaded: true });"
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
        let inspector = EDITOR_JS
            .split("function inspectPreviewExport(html) {")
            .nth(1)
            .expect("compiled preview export inspector");
        let frame_fixture = inspector
            .find("{ kind: \"puzzle3d\", windowName: \"Puzzle3DFrameFixture\" }")
            .expect("3D frame fixture extractor candidate");
        let puzzle_export = inspector
            .find("{ kind: \"puzzle2d\", windowName: \"PuzzleExport\" }")
            .expect("2D export extractor candidate");
        assert!(
            frame_fixture < puzzle_export,
            "3D editor previews must extract the 3D frame fixture before the outer scene export"
        );
        assert!(!EDITOR_JS.contains("function replacePreviewExport("));
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
        assert!(EDITOR_LEVEL3D_JS.contains("focus: sceneName,"));
        assert!(!EDITOR_LEVEL3D_JS.contains("next.currentScene = sceneName;"));
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
        assert!(EDITOR_LEVEL3D_JS.contains("visuals: level3dPreviewVisuals(exportData)"));
        assert!(EDITOR_LEVEL3D_JS.contains("camera: level3dRuntimePreviewCamera(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("zoom: camera.zoom,"));
        assert!(EDITOR_LEVEL3D_JS.contains("view: level3dRuntimePreviewView(snapshot)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("settings: level3dPreviewSettings(snapshot.render || {})")
        );
        assert!(!EDITOR_LEVEL3D_JS.contains("previewFrameHasEditorLevelState = true;"));
        assert!(EDITOR_LEVEL3D_JS.contains("function renderSolverRuntimePreview()"));
        assert!(EDITOR_LEVEL3D_JS.contains("function sendSolverPreviewToRuntime()"));
        assert!(EDITOR_LEVEL3D_JS.contains("solverPreviewFrame.contentWindow.postMessage(level3dPreviewSurfaceMessage(update), \"*\");"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPreviewUpdateFromSnapshot(snapshot)"));
        assert!(EDITOR_JS.contains(
            "isPuzzle3dExport(exportData) && typeof renderSolverRuntimePreview === \"function\""
        ));
        assert!(EDITOR_JS.contains("typeof clearSolverRuntimePreview === \"function\""));
        assert!(EDITOR_CSS.contains(".solver-board-viewport.is-puzzle3d"));
        assert!(EDITOR_CSS.contains(".solver-preview-frame"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("function level3dLayerUsesRuntimeScreenFootprints(view)")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("view?.coordinateSpace === \"canvas-css-px\""));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("level3dLayerScreenPointToFootprint({ x, y }, view, width, height)")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("function currentLevel3dLayerZ()"));
        assert!(EDITOR_LEVEL3D_JS.contains("return slice;"));
        assert!(EDITOR_LEVEL3D_JS.contains("visual: object?.visual ?? descriptor.visual ?? null,"));
        assert!(!EDITOR_LEVEL3D_JS.contains(
            "visual: object?.visual || descriptor.visual || object?.name || descriptor.name"
        ));
    }

    #[test]
    fn level3d_microban_01_supplies_preview_contract_data() {
        let source = include_str!("../../lang/tests/fixtures/spec_3d_preview_contract.puzzle");
        let document = puzzle_lang::parse_game(source).expect("parse Microban 3D fixture");
        let fixture_json = puzzle_lang::export_loaded_document_visual_fixture_json(&document)
            .expect("export Microban 3D fixture");

        assert!(fixture_json.contains("\"levelIndex\": 0"));
        assert!(fixture_json.contains("\"name\": \"microban_01\""));
        assert!(fixture_json.contains("\"label\": \"Microban 01\""));
        assert!(fixture_json.contains("\"size\": { \"width\": 6, \"depth\": 7, \"height\": 2 }"));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 2, \"y\": 3, \"z\": 1 }, \"objects\": [{ \"id\": 3, \"name\": \"Player\", \"visual\": null }]"
        ));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 1, \"y\": 3, \"z\": 1 }, \"objects\": [{ \"id\": 4, \"name\": \"Box\", \"visual\": null }]"
        ));
        assert!(fixture_json.contains(
            "\"position\": { \"x\": 2, \"y\": 5, \"z\": 0 }, \"objects\": [{ \"id\": 1, \"name\": \"Floor\", \"visual\": null }, { \"id\": 2, \"name\": \"Goal\", \"visual\": null }]"
        ));

        assert!(fixture_json.contains("\"layerCount\": 3"));
        assert!(fixture_json.contains(
            "\"Player\": { \"id\": 3, \"name\": \"Player\", \"visual\": null, \"layer\": 2 }"
        ));
        assert!(
            fixture_json.contains(
                "\"Box\": { \"id\": 4, \"name\": \"Box\", \"visual\": null, \"layer\": 2 }"
            )
        );
        assert!(fixture_json.contains("\"visuals\": {"));
        assert!(
            fixture_json.contains(
                "\"camera\": { \"projection\": \"orthographic\", \"yawDegrees\": 10, \"pitchDegrees\": 55, \"rollDegrees\": 20, \"zoom\": 1.1, \"interactiveLook\": false, \"interactiveZoom\": false }"
            )
        );
        assert!(fixture_json.contains("\"render\": {"));
        assert!(fixture_json.contains("\"interactiveLook\": false"));
        assert!(fixture_json.contains("\"interactiveZoom\": false"));
        assert!(fixture_json.contains("\"shade\": true"));

        assert!(EDITOR_LEVEL3D_JS.contains("level: {"));
        assert!(EDITOR_LEVEL3D_JS.contains("resources: level3dRuntimePreviewResources(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("camera: level3dRuntimePreviewCamera(snapshot)"));
        assert!(EDITOR_LEVEL3D_JS.contains("projection: camera.projection,"));
        assert!(EDITOR_LEVEL3D_JS.contains("view: level3dRuntimePreviewView(snapshot)"));
        assert!(
            EDITOR_LEVEL3D_JS.contains("settings: level3dPreviewSettings(snapshot.render || {})")
        );
    }

    #[test]
    fn level3d_editor_syncs_source_focus_and_click_targets() {
        assert!(EDITOR_SOURCE_JS.contains("sourceInteractionFromPointer(event)"));
        assert!(EDITOR_SOURCE_JS.contains("syncPreviewModeFromSourceCursor({"));
        assert!(EDITOR_LEVEL3D_JS.contains("registerSourceEditableTarget?.(\"level3d\""));
        assert!(EDITOR_SOURCE_JS.contains("scheduleSourceCursorPreviewSync();"));
        assert!(EDITOR_JS.contains("function syncPreviewModeFromSourceCursor(options = {})"));
        assert!(EDITOR_JS.contains("[\"edit\", \"level3d\", \"visual\", \"visual3d\", \"sounds\"].includes(currentPreviewMode)"));
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
        assert!(!EDITOR_VISUAL_JS.contains("function loadVisualFromSourceClick"));
        assert!(
            !EDITOR_JS
                .contains("sourceEditor.addEventListener(\"click\", loadLevelFromSourceClick);")
        );
        assert!(
            !EDITOR_VISUAL_JS
                .contains("sourceEditor.addEventListener(\"click\", loadVisualFromSourceClick);")
        );
        assert!(EDITOR_JS.contains("function loadResolvedSourceTarget(target, options = {})"));
        assert!(EDITOR_JS.contains("function previewModeForSourceTarget(target)"));
        assert!(EDITOR_JS.contains(
            "kind: target.kind, dimension: target.dimension, start: target.start, end: target.end"
        ));
        assert!(EDITOR_JS.contains("currentPreviewMode === resolvedMode"));
        assert!(EDITOR_VISUAL_JS.contains("function loadVisualSourceTarget(target, options = {})"));
    }

    #[test]
    fn visual_pane_top_bar_owns_new_add_and_save_lifecycle() {
        assert!(EDITOR_HTML.contains(r#"id="newVisualButton""#));
        assert!(EDITOR_HTML.contains(r#"id="visualInsertButton""#));
        assert!(EDITOR_HTML.contains(r#"id="visualUpdateButton""#));
        assert!(!EDITOR_HTML.contains("duplicateVisualButton"));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="image-plus""#));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="file-plus-corner""#));
        assert!(
            EDITOR_DOM_JS
                .contains("const newVisualButton = document.querySelector(\"#newVisualButton\");")
        );
        assert!(!EDITOR_JS.contains("addEmptyVisual2dToFocusedSource"));
        assert!(EDITOR_VISUAL_JS.contains("function newVisualDraft()"));
        assert!(EDITOR_VISUAL_JS.contains("function addVisualToSource()"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("canReplaceCurrentVisualDefinition(source) ? \"duplicate\" : \"insert\"")
        );
        let visual_header = EDITOR_HTML
            .split_once(r#"id="visualPaneHeaderActions""#)
            .and_then(|(_, tail)| tail.split_once(r#"<div class="tool-pane-scroll">"#))
            .map(|(header, _)| header)
            .expect("visual pane top bar");
        for id in [
            "newVisualButton",
            "visualInsertButton",
            "visualUpdateButton",
            "newVisual3dButton",
            "visual3dInsertButton",
            "visual3dUpdateButton",
        ] {
            assert!(visual_header.contains(&format!(r#"id="{id}""#)));
        }
        assert!(!EDITOR_VISUAL_JS.contains("sourceActions.append("));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains(
            "setVisualEditorSourceTarget(state, { start: result.start, end: result.end, name: result.name }, document);"
        ));
        assert!(!EDITOR_VISUAL_JS.contains("function addEmptyVisualToSource"));
        assert!(!EDITOR_VISUAL_JS.contains("function insertEmptyVisualDefinition"));
    }

    #[test]
    fn visual3d_source_tools_use_shared_visual_target_contract() {
        assert!(!EDITOR_VISUAL3D_JS.contains("function findVisuals3dBlocks(source)"));
        assert!(!EDITOR_VISUAL3D_JS.contains("pattern.exec(source)"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("function projectVisualDocumentContract"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("function commitVisualEditorMutation(options)"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains(
            "async function commitVisualEditorMutationNow({ state, request, allowActiveDocument = false })"
        ));
        assert!(!EDITOR_VISUAL3D_JS.contains("function findVisual3dDefinitionByName"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function findVisual3dDefinitionAtPosition"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function findVisual3dDefinitions"));
        assert!(EDITOR_JS.contains(
            "focusedPuzzleSurfaceEntriesByKind(\"visual\", { document: activeDocument(), source }, \"3d\")"
        ));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime.sourceEntryInfo(text)"));
        assert!(!EDITOR_JS.contains("findVisual3dDefinitionByName(source, name)"));
        assert!(EDITOR_VISUAL3D_JS.contains("function visual3dTargetPayload(target)"));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "target?.sourceVisual?.dimension === \"3d\" && target.sourceVisual.status === \"incomplete\""
        ));
        assert!(!EDITOR_VISUAL3D_JS.contains("sourceVisual3d"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function parseVisual3dDefinitionSource"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function parseVisual3dRows"));
        assert!(!EDITOR_VISUAL3D_JS.contains("typeof visualSourceCursorPosition"));
        assert!(!EDITOR_VISUAL3D_JS.contains("typeof visualSourceTargetAtCursor"));
        assert!(!EDITOR_VISUAL3D_JS.contains(": source.length"));
    }

    #[test]
    fn visual3d_source_mutation_serializes_z_slices_in_source_order() {
        assert!(EDITOR_VISUAL3D_JS.contains("const worldZ = visual3d.depth - 1 - sourceZ;"));
        assert!(EDITOR_VISUAL3D_JS.contains("frame[visual3dCellIndex(x, y, worldZ)]"));
    }

    #[test]
    fn visual_color_edit_undo_batches_until_commit() {
        assert!(EDITOR_VISUAL_JS.contains("function beginVisualColorEditHistory(kind)"));
        assert!(EDITOR_VISUAL_JS.contains("function commitVisualColorEditHistory(kind)"));
        assert!(EDITOR_VISUAL_JS.contains("updateSelectedVisualColor(value, options = {})"));
        assert!(EDITOR_VISUAL_JS.contains("renderVisualColorAdjuster({"));
        assert!(EDITOR_VISUAL_JS.contains("onInput: onChange,"));
        assert!(EDITOR_VISUAL_JS.contains("previewNewVisualColor(color, { deferHistory: true })"));
        assert!(EDITOR_VISUAL3D_JS.contains("updateSelectedVisual3dColor(value, options = {})"));
        assert!(
            EDITOR_VISUAL3D_JS.contains("function previewNewVisual3dColor(color, options = {})")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("onChange: previewNewVisual3dColor"));
        assert!(EDITOR_JS.contains("commitVisualColorEditHistory(kind);"));
    }

    #[test]
    fn visual_color_adjuster_uses_shared_custom_editor() {
        let adjuster_start = EDITOR_VISUAL_JS
            .find("function renderVisualColorAdjuster")
            .expect("visual color adjuster");
        let adjuster_end = EDITOR_VISUAL_JS[adjuster_start..]
            .find("function renderVisualPalette")
            .map(|index| adjuster_start + index)
            .expect("visual palette after adjuster");
        let adjuster = &EDITOR_VISUAL_JS[adjuster_start..adjuster_end];

        assert!(adjuster.contains("window.PuzzleStudioColorEditor.create({"));
        assert!(adjuster.contains("className: \"visual-color-adjuster\""));
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
                < EDITOR_HTML.find("editor_visual.js").unwrap()
        );
        assert!(!adjuster.contains("colorInput.type = \"color\";"));
        assert!(!adjuster.contains("visual-native-color-input"));
        assert!(!EDITOR_VISUAL_JS.contains("showPicker"));
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
        assert!(!EDITOR_VISUAL_JS.contains("window.PuzzleStudioHost?.pickScreenColor"));
        assert!(!EDITOR_VISUAL_JS.contains("window.PuzzleStudioHost?.canPickScreenColor"));
        assert!(!EDITOR_SOURCE_JS.contains("window.PuzzleStudioHost.pickSourceColor"));
        assert!(!EDITOR_SOURCE_JS.contains("showPicker"));
        assert!(!EDITOR_VISUAL_JS.contains("showPicker"));
        assert!(!EDITOR_VISUAL3D_JS.contains("showPicker"));
        assert!(!EDITOR_VISUAL_JS.contains("function visualEyedropperIconSvg()"));
        assert!(!EDITOR_VISUAL_JS.contains("visual-palette-eyedropper-button"));
        assert!(!EDITOR_VISUAL_JS.contains("visualEyedropperActive"));
        assert!(!EDITOR_VISUAL3D_JS.contains("visual3dEyedropperActive"));
    }

    #[test]
    fn visual_palette_keyboard_shortcuts_do_not_turn_tool_buttons_into_erasers() {
        assert!(EDITOR_VISUAL_JS.contains(
            "if (rawIndex === undefined) {\n      return;\n    }\n    event.preventDefault();"
        ));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "if (rawIndex === undefined) {\n    return;\n  }\n  event.preventDefault();"
        ));
    }

    #[test]
    fn visual_palette_uses_dot_for_eraser_and_zero_through_nine_for_colors() {
        assert!(
            EDITOR_COMMANDS_JS
                .contains(r#"shortcuts: [{ key: visualExportCharForColorIndex(null) }],"#)
        );
        assert!(EDITOR_COMMANDS_JS.contains(r#"for (let index = 0; index < 10; index += 1) {"#));
        assert!(
            EDITOR_COMMANDS_JS.contains(r#"shortcuts: [{ key: VISUAL_COLOR_TOKENS[index] }],"#)
        );
        assert!(EDITOR_COMMANDS_JS.contains("shortcutOnly: true,"));
        assert!(EDITOR_JS.contains(r#"if (element?.dataset?.shortcutOnly === "true") {"#));
        assert!(EDITOR_JS.contains("if (!text && !shortcuts.length) {"));
        assert!(
            !EDITOR_VISUAL_JS.contains(r#"setEditorShortcutHint(leadingControl, { key: "b" });"#)
        );
        assert!(!EDITOR_VISUAL_JS.contains(r#"leadingControl.dataset.tooltip = "Brush";"#));
        assert!(
            !EDITOR_VISUAL_JS
                .contains(r#"if (key === "b") activateVisualBrushShortcut(dimension);"#)
        );
        assert!(!EDITOR_VISUAL_JS.contains("selectVisualPaletteShortcut"));
        assert!(!EDITOR_VISUAL_JS.contains(
            r#"button.title = displayName ? `Paint ${displayName} (${entry.color})` : `Paint ${entry.color}`;"#
        ));
    }

    #[test]
    fn visual_brush_size_omits_px_unit() {
        assert!(!EDITOR_HTML.contains("visual-brush-size-unit"));
        assert!(!EDITOR_CSS.contains(".visual-brush-size-unit"));
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
        assert!(EDITOR_VISUAL_JS.contains("function deactivateVisualBucketModeAfterUse()"));
        assert!(EDITOR_VISUAL_JS.contains(
            "setVisualActionStatus(\"Connected area already has that color\", \"is-ok\");\n    deactivateVisualBucketModeAfterUse();\n    return false;"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "const message = colorIndex === null ? \"Filled connected area with transparent\" : \"Filled connected area\";\n  deactivateVisualBucketModeAfterUse();"
        ));
        assert!(EDITOR_VISUAL3D_JS.contains("function deactivateVisual3dBucketModeAfterUse()"));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "setVisual3dActionStatus(\"Connected component already has that color\", \"is-ok\");\n    deactivateVisual3dBucketModeAfterUse();\n    return true;"
        ));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "visual3d.hoverSlice = null;\n  deactivateVisual3dBucketModeAfterUse();\n  renderVisual3dBuilder();"
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
    fn visual_cell_hover_preserves_pixel_color_surface() {
        assert!(EDITOR_CSS.contains("--visual-swatch-checker: url("));
        assert!(EDITOR_CSS.contains(
            ".visual-cell:focus-visible {\n  background-color: var(--visual-swatch-bg);\n  background-image: var(--visual-swatch-checker);"
        ));
        assert!(
            !EDITOR_CSS.contains(
                ".visual-cell:hover,\n.visual-cell:focus-visible,\n.visual-cell:active {"
            )
        );
        assert!(!EDITOR_CSS.contains(".visual-brush-preview"));
    }

    #[test]
    fn visual_clip_selection_is_positioned_overlay_not_cell_paint() {
        assert!(
            EDITOR_VISUAL_JS
                .contains("frame.style.setProperty(\"--visual-clip-x\", String(rect.x));")
        );
        assert!(
            EDITOR_VISUAL_JS.contains(
                "frame.style.setProperty(\"--visual-clip-height\", String(rect.height));"
            )
        );
        assert!(!EDITOR_VISUAL_JS.contains("frame.style.gridColumn"));
        assert!(!EDITOR_VISUAL_JS.contains("frame.style.gridRow"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("button.classList.toggle(\"is-clip-selected\", isClipSelected);")
        );
        assert!(EDITOR_CSS.contains(
            ".visual-board.is-clip-active .visual-cell.is-clip-selected {\n  cursor: grab;"
        ));
        assert!(EDITOR_CSS.contains(".visual-cell.is-clip-selected {\n  box-shadow: none;"));
        assert!(EDITOR_CSS.contains(".visual-clip-selection-frame {\n  position: absolute;"));
        assert!(EDITOR_CSS.contains("left: calc(var(--visual-clip-x) * var(--visual-cell));"));
        assert!(EDITOR_CSS.contains("background: transparent;"));
        let clip_paste_cell = EDITOR_VISUAL_JS
            .split_once("function pasteVisualClipCell(index, clipboardValue) {")
            .expect("2D clip paste cell owner exists")
            .1
            .split_once("function visualClipCellsForCurrentPalette(clipboard) {")
            .expect("2D clip paste cell owner closes")
            .0;
        assert!(clip_paste_cell.contains("if (clipboardValue === null)"));
        assert!(clip_paste_cell.contains("return false;"));
        assert!(clip_paste_cell.contains("validVisualColorIndex(clipboardValue)"));
        assert!(clip_paste_cell.contains("setVisualCellColorAtIndex(index, clipboardValue)"));
        assert!(!clip_paste_cell.contains("#00000000"));
    }

    #[test]
    fn visual_board_rerender_does_not_empty_scroll_content_before_replace() {
        assert!(EDITOR_VISUAL_JS.contains("const nextBoard = document.createDocumentFragment();"));
        assert!(EDITOR_VISUAL_JS.contains("renderVisualClipSelectionFrame(nextBoard);"));
        assert!(EDITOR_VISUAL_JS.contains("visualBoard.replaceChildren(nextBoard);"));
        assert!(!EDITOR_VISUAL_JS.contains("visualBoard.replaceChildren();"));
    }

    #[test]
    fn visual_pane_rerenders_share_scroll_preservation() {
        assert!(EDITOR_VISUAL_JS.contains("function withVisualPaneScrollPreserved("));
        assert!(EDITOR_VISUAL_JS.contains(
            "function renderVisualControls() {\n  withVisual2dPaneScrollPreserved(() => renderVisualControlsContent());"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "function renderVisualPalette() {\n  withVisual2dPaneScrollPreserved(() => renderVisualPaletteContent());"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "function renderVisualBoard() {\n  withVisual2dPaneScrollPreserved(() => renderVisualBoardContent());"
        ));
        assert!(
            EDITOR_VISUAL_JS
                .contains("function renderVisualAnimationControls() {\n  if (!visualBuilder) {")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("return withVisualPaneScrollPreserved(visual3dBuilder, render);")
        );
        let capture_scroll = EDITOR_VISUAL_JS
            .split_once("function captureVisualPaneScroll(builder) {")
            .expect("shared visual scroll capture exists")
            .1
            .split_once("function restoreVisualPaneScroll")
            .expect("shared visual scroll capture closes")
            .0;
        assert!(!capture_scroll.contains("document.activeElement"));
    }

    #[test]
    fn visual_translate_releases_its_own_pointer_capture_before_committing() {
        let stop_translate = EDITOR_VISUAL_JS
            .split_once("function stopVisualTranslate(event) {")
            .expect("2D visual translate stop handler exists")
            .1
            .split_once("function renderVisualClipButton")
            .expect("2D visual translate stop handler closes")
            .0;
        assert!(stop_translate.contains("visualBoard.hasPointerCapture?.(event.pointerId)"));
        assert!(stop_translate.contains("visualBoard.releasePointerCapture(event.pointerId)"));
        assert!(!stop_translate.contains("visualPaintDrag.pointerId"));
        assert!(
            stop_translate.contains("pushVisualEditUndoSnapshot(\"visual\", drag.beforeSnapshot)")
        );
    }

    #[test]
    fn visual_clip_is_a_stable_edit_region_toggle_with_permanent_commands() {
        assert!(EDITOR_VISUAL_JS.contains("const VISUAL_EDITOR_TOOL_SCHEMA = Object.freeze(["));
        assert!(EDITOR_VISUAL_JS.contains(
            "renderVisualEditorToolbar({ dimension: \"2d\", target: visualToolbarHost });"
        ));
        for command in ["copy", "cut", "paste", "delete"] {
            assert!(
                EDITOR_VISUAL_JS
                    .contains(&format!("id: \"{command}\",\n    group: \"clipboard\","))
            );
        }
        assert!(EDITOR_VISUAL_JS.contains(
            "...VISUAL_EDIT_COMMANDS.map(({ id, group }) => Object.freeze({ key: id, group }))"
        ));
        assert!(EDITOR_VISUAL_JS.contains("function runVisualEditCommand(dimension, command)"));
        assert!(EDITOR_VISUAL3D_JS.contains("function runVisual3dEditCommand(command)"));
        assert!(!EDITOR_HTML.contains(r#"id="visual3dCopySliceButton""#));
        assert!(!EDITOR_HTML.contains(r#"id="visual3dPasteSliceButton""#));
        assert!(!EDITOR_JS.contains("sliceClipboard"));
        assert!(!EDITOR_VISUAL3D_JS.contains("copyVisual3dSlice"));
        assert!(!EDITOR_VISUAL3D_JS.contains("pasteVisual3dSlice"));
        assert!(EDITOR_HTML.contains(r#"id="visualToolbarHost" class="visual-toolbar-host""#));
        assert!(EDITOR_HTML.contains(r#"id="visual3dToolbarHost" class="visual-toolbar-host""#));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "renderVisualEditorToolbar({ dimension: \"3d\", target: visual3dToolbarHost });"
        ));
        assert!(!EDITOR_VISUAL_JS.contains("paletteGrid.append(clipActions);"));
        assert!(EDITOR_CSS.contains(".visual-clip-actions {\n  position: relative;"));
        assert!(EDITOR_CSS.contains("width: 26px;\n  min-width: 26px;"));
        assert!(EDITOR_CSS.contains("height: 26px;\n  min-height: 26px;"));
        assert!(!EDITOR_VISUAL_JS.contains("visual-clip-expanded-actions"));
        assert!(!EDITOR_VISUAL3D_JS.contains("visual-clip-expanded-actions"));
        assert!(!EDITOR_CSS.contains(".visual-clip-expanded-actions"));
        assert!(EDITOR_VISUAL_JS.contains("visualClipActive ? normalizeVisualClipRect(visualClipSelection) : visualWholeEditRect()"));
        assert!(EDITOR_VISUAL3D_JS.contains("visual3dClipActive ? normalizeVisual3dClipBox(visual3dClipSelection) : visual3dWholeEditBox()"));
        assert!(EDITOR_VISUAL_JS.contains("!visualClipRectContainsIndex(region, current)"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("region && !visual3dClipBoxContainsCoords(region, current)")
        );
        assert!(
            EDITOR_VISUAL3D_JS.contains("region && !visual3dClipBoxContainsCoords(region, coords)")
        );
    }

    #[test]
    fn visual3d_clip_uses_scope_owned_world_box_selection() {
        assert!(EDITOR_HTML.contains(r#"id="visual3dClipActions""#));
        assert!(!EDITOR_HTML.contains(r#"id="visual3dClearButton""#));
        assert!(!EDITOR_HTML.contains(r#"id="visualClearButton""#));
        assert!(
            EDITOR_VISUAL_JS
                .contains("delete: renderVisualEditCommandButton(dimension, \"delete\")")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("function normalizeVisual3dClipBox(box)"));
        assert!(EDITOR_VISUAL3D_JS.contains("fullDepth: visual3dEditScope() === \"all\""));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("box[`min${worldAxis.toUpperCase()}`] = fullDepth ? 0 : fixedStack;")
        );
        assert!(EDITOR_VISUAL3D_JS.contains(
            "box[`max${worldAxis.toUpperCase()}`] = fullDepth ? visual3dAxisSize(worldAxis) - 1 : fixedStack;"
        ));
        assert!(
            EDITOR_VISUAL3D_JS.contains(
                "visual3dClipBoxFromPlaneRect(rect, { base: visual3dClipDrag.originBox })"
            )
        );
        assert!(EDITOR_VISUAL3D_JS.contains("visual3dClipClipboardFromSelection(box, dimensions)"));
        assert!(EDITOR_VISUAL3D_JS.contains("if (clipboard.scope === \"slice\")"));
        assert!(EDITOR_VISUAL3D_JS.contains("renderVisual3dClipFloatingPreview(rect);"));
        assert!(EDITOR_VISUAL3D_JS.contains("drawVisual3dClipBounds(ctx, view);"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("const box = normalizeVisual3dClipBox(visual3dClipSelection);")
        );
        assert!(EDITOR_CSS.contains("--visual3d-clip-stroke:"));
        let clip_paste_cell = EDITOR_VISUAL3D_JS
            .split_once("function pasteVisual3dClipCell(index, clipboardValue) {")
            .expect("3D clip paste cell owner exists")
            .1
            .split_once("function visual3dClipForCurrentPalette(clipboard) {")
            .expect("3D clip paste cell owner closes")
            .0;
        assert!(clip_paste_cell.contains("if (clipboardValue === null)"));
        assert!(clip_paste_cell.contains("return false;"));
        assert!(clip_paste_cell.contains("validVisual3dColorIndex(clipboardValue)"));
        assert!(!clip_paste_cell.contains("visual3dColorForColorIndex"));
        assert!(!clip_paste_cell.contains("#00000000"));
        assert_eq!(
            EDITOR_VISUAL3D_JS
                .matches("pasteVisual3dClipCell(index, clipboard.cells[offset])")
                .count(),
            2,
            "whole and slice paste share transparent-hole semantics"
        );
        let clip_drag = EDITOR_VISUAL3D_JS
            .split_once("function continueVisual3dClip(event) {")
            .expect("3D clip drag handler exists")
            .1
            .split_once("function stopVisual3dClip(event) {")
            .expect("3D clip drag handler closes")
            .0;
        assert!(
            clip_drag.contains("renderVisual3dPreview();"),
            "3D preview follows clip selection, move, and resize while dragging"
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("visual3dEditScope() === \"slice\" && nextAxis !== visual3d.axis")
        );
    }

    #[test]
    fn visual_source_actions_live_in_the_visual_pane_top_bar() {
        assert!(EDITOR_HTML.contains(
            r#"id="visualSourceActionBank" class="visual-pane-source-actions" role="group" aria-label="2D visual source actions""#
        ));
        assert!(EDITOR_HTML.contains(
            r#"id="visual3dSourceActionBank" class="visual-pane-source-actions" role="group" aria-label="3D visual source actions" hidden"#
        ));
        assert!(EDITOR_HTML.contains(r#"id="visualPaneHeaderActions""#));
        assert!(
            EDITOR_WORKBENCH_JS.contains("document.querySelector(\"#visualPaneHeaderActions\")")
        );
        assert!(EDITOR_DOM_JS.contains(
            "const visualSourceActionBank = document.querySelector(\"#visualSourceActionBank\");"
        ));
        assert!(EDITOR_DOM_JS.contains(
            "const visual3dSourceActionBank = document.querySelector(\"#visual3dSourceActionBank\");"
        ));
        assert!(EDITOR_JS.contains(
            "visualSourceActionBank.hidden = !visualPaneVisible || currentVisualPaneMode !== \"visual\";"
        ));
        assert!(EDITOR_JS.contains(
            "visual3dSourceActionBank.hidden = !visualPaneVisible || currentVisualPaneMode !== \"visual3d\";"
        ));
        assert!(EDITOR_VISUAL_JS.contains("root.append(nameRow, geometry, animation);"));
        assert!(EDITOR_VISUAL_JS.contains("currentWrap.append(visualShapeField);"));
        assert!(EDITOR_VISUAL3D_JS.contains("currentWrap.append(visual3dShapeField);"));
        assert_eq!(
            EDITOR_VISUAL_JS
                .matches("input.placeholder = \"shape\";")
                .count(),
            2
        );
        assert!(!EDITOR_VISUAL_JS.contains("visual-shape-bind-label"));
        assert!(EDITOR_CSS.contains(".visual-editor-name-row {\n  flex: 0 1 470px;"));
        assert!(
            EDITOR_CSS
                .contains(".visual-current-color-wrap > .visual-shape-field {\n  flex: 0 0 auto;")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("function newVisual3dDraft()"));
        assert!(EDITOR_VISUAL3D_JS.contains("function addVisual3dToSource()"));
        assert!(
            EDITOR_VISUAL3D_JS.contains(
                "canReplaceCurrentVisual3dDefinition(source) ? \"duplicate\" : \"insert\""
            )
        );
    }

    #[test]
    fn visual_size_inputs_refresh_the_preview_while_editing() {
        assert!(EDITOR_HTML.contains(r#"id="visualWidthInput" type="number""#));
        assert!(EDITOR_HTML.contains(r#"id="visualHeightInput" type="number""#));
        assert!(EDITOR_VISUAL_JS.contains("function bindVisualDimensionInput(input, axis)"));
        assert!(EDITOR_VISUAL_JS.contains(
            "bindVisualDimensionInput(visualWidthInput, \"width\");\nbindVisualDimensionInput(visualHeightInput, \"height\");"
        ));
        assert!(EDITOR_VISUAL3D_JS.contains("function bindVisual3dDimensionInput(input, axis)"));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "bindVisual3dDimensionInput(visual3dWidthInput, \"width\");\nbindVisual3dDimensionInput(visual3dHeightInput, \"height\");\nbindVisual3dDimensionInput(visual3dDepthInput, \"depth\");"
        ));
        assert!(
            EDITOR_VISUAL_JS
                .contains("sizeBindButton.innerHTML = visualLucideIconSvg(\"link-2\");")
        );
        assert!(EDITOR_VISUAL_JS.contains("visual.sizeBound = !visual.sizeBound;"));
        assert!(EDITOR_VISUAL_JS.contains("visual3d.sizeBound = !visual3d.sizeBound;"));
        assert!(EDITOR_CSS.contains(
            ".visual-editor-name-row .visual-size-control .visual-extent-inputs input,\n.visual-editor-name-row .visual-size-control .visual3d-extent-inputs input {\n  width: 24px;"
        ));
        assert!(
            EDITOR_CSS.contains("  border: 0;\n  border-radius: 0;\n  background: transparent;")
        );
    }

    #[test]
    fn visual_brush_size_is_pixel_based_and_paint_updates_changed_cells() {
        assert!(EDITOR_CSS.contains(
            "#visualBuilder .visual-board {\n  --visual-cell: clamp(8px, calc(100cqw / var(--visual-size)), 64px);\n}"
        ));
        assert!(EDITOR_CSS.contains("--visual-puzzle-line: #1d242b;"));
        assert!(EDITOR_CSS.contains(
            "box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--visual-puzzle-line) 38%, transparent);"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "button.style.setProperty(\"--visual-puzzle-line\", visualGridLineForColorIndex(colorIndex));"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "return validVisualColorIndex(index) ? readableInkForColor(visual.palette[index].color) : \"#1d242b\";"
        ));
        assert!(EDITOR_VISUAL_JS.contains("let visualBrushSizePx = 1;"));
        assert!(EDITOR_HTML.contains(r#"id="visualBrushSizeInput" class="visual-brush-size-input" type="number" min="1" max="64" step="1""#));
        assert!(
            EDITOR_HTML.contains(r#"data-editor-icon="highlighter" class="visual-marker-icon""#)
        );
        assert!(EDITOR_ICONS_JS.contains(r#""highlighter": `"#));
        assert!(!EDITOR_HTML.contains("data-visual-brush-preset"));
        assert!(EDITOR_VISUAL_JS.contains(
            "if (visualBrushSizePx === 1) {\n    const index = visualCellIndexFromPoint(point);\n    return index >= 0 ? [index] : [];\n  }"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "return Math.min(Math.max(visual.width, visual.height), visualBrushSizePx);"
        ));
        assert!(EDITOR_VISUAL_JS.contains("return Math.min(size, visualBrushSizePx);"));
        assert!(EDITOR_VISUAL_JS.contains("function renderVisualCellsAtIndices(indices)"));
        assert!(EDITOR_VISUAL_JS.contains("finishVisualPaintMutation(changedIndices);"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("finishVisualPaintMutation(changedIndices, { deferSourceSync: true });")
        );
        assert!(EDITOR_VISUAL_JS.contains(
            "if (!options.deferSourceSync) {\n    updateVisualBoundShapeDefinition();\n  }"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "if (!options.deferSourceSync) {\n    syncVisualSourceActionButtons();\n  }"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "updateVisualBoundShapeDefinition();\n    syncVisualSourceActionButtons();\n    pushVisualEditUndoSnapshot(\"visual\", visualPaintDrag.beforeSnapshot);"
        ));
        assert!(EDITOR_JS.contains(
            "if (visual.animationMode) {\n      if (typeof ensureVisualAnimationFrames === \"function\") {"
        ));
        assert!(EDITOR_JS.contains(
            "} else if (typeof resetVisualAnimationFramesFromCurrentCells === \"function\") {\n      resetVisualAnimationFramesFromCurrentCells();\n    }"
        ));
        assert!(!EDITOR_VISUAL_JS.contains("visualBrushPreviewElement"));
        assert!(!EDITOR_VISUAL_JS.contains("visual-brush-preview"));
        assert!(!EDITOR_VISUAL_JS.contains("finishVisualPaintMutation();"));
    }

    #[test]
    fn solid_visual_source_loads_without_fabricated_editor_grid() {
        assert!(!EDITOR_VISUAL_JS.contains("SOLID_VISUAL_EDITOR_SIZE"));
        assert!(EDITOR_VISUAL_JS.contains("const size = Number.isFinite(parsed) ? parsed : 5;"));
        assert!(!EDITOR_VISUAL_JS.contains("Math.trunc(Number(value) || 5)"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("solid: width === 1 && height === 1 && parsedFrames[0][0] === 0,")
        );
        assert!(!EDITOR_VISUAL_JS.contains("const size = 5;"));
    }

    #[test]
    fn visual_marker_preserves_paint_material_and_fill_owns_fill_mode() {
        assert!(EDITOR_VISUAL3D_JS.contains("function selectVisual3dBrushSize(size)"));
        assert!(EDITOR_VISUAL_JS.contains(
            "const wasBucketActive = visualBucketActive;\n  const wasClipActive = visualClipActive || visualClipSelection;\n  visualBrushSizePx = normalizeVisualBrushSize(size);\n  visualBucketActive = false;\n  deactivateVisualClipMode({ render: false });"
        ));
        assert!(
            EDITOR_VISUAL_JS.contains(
                "if (wasBucketActive || wasClipActive) {\n    renderVisualPalette();\n  }"
            )
        );
        assert!(EDITOR_VISUAL_JS.contains(
            "visualBucketActive = !visualBucketActive;\n  syncVisualPaintToolControls();\n  renderVisualPalette();"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "if (!validVisualColorIndex(visual.selectedColorIndex)) {\n    visual.selectedColorIndex = validVisualColorIndex(visualLastPaintColorIndex) ? visualLastPaintColorIndex : 0;\n  }"
        ));
    }

    #[test]
    fn visual_marker_uses_compact_numeric_input() {
        assert!(EDITOR_CSS.contains(".visual-brush-size-input {\n  width: 28px;"));
        assert!(
            EDITOR_CSS.contains("border: 0;\n  border-radius: 4px;\n  background: transparent;")
        );
        assert!(EDITOR_CSS.contains("font: 800 11px/24px ui-monospace"));
        assert!(EDITOR_CSS.contains(".visual-brush-size-input:hover,\n.visual-brush-size-input:focus {\n  background: var(--input-bg);"));
        assert!(EDITOR_CSS.contains(".visual-marker-icon {\n  width: 20px;"));
        assert!(EDITOR_JS.contains("|| element.classList.contains(\"visual-brush-size-input\");"));
    }

    #[test]
    fn visual_palette_owns_marker_and_toolbar_orders_scope_grid_clip() {
        assert!(EDITOR_HTML.contains(r#"id="visualTransformActionBank" hidden"#));
        assert!(!EDITOR_HTML.contains("visual-toolbar visual-edit-actions"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("paletteGrid.append(leadingControl);\n  }\n  const eraseButton")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("target: visualPalette,\n    leadingControl: visualMarkerTool,")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("target: visual3dPalette,\n    leadingControl: visualMarkerTool,")
        );
        assert!(EDITOR_VISUAL_JS.contains("{ key: \"scope\", group: \"context\" },\n  { key: \"grid\", group: \"context\" },\n  { key: \"clip\", group: \"context\" },"));
        assert!(!EDITOR_VISUAL_JS.contains("{ key: \"marker\", group: \"context\" }"));
        assert!(EDITOR_VISUAL_JS.contains(
            "{ key: \"fill\", group: \"paint\" },\n  { key: \"translate\", group: \"paint\" },"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "{ key: \"flip-vertical\", group: \"transform\" },\n  ...VISUAL_EDIT_COMMANDS.map(({ id, group }) => Object.freeze({ key: id, group })),"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "\"flip-vertical\": is3d ? visual3dFlipPlaneVerticalButton : visualFlipVerticalButton,"
        ));
        assert!(!EDITOR_VISUAL_JS.contains("visual3dClearButton"));
        assert!(!EDITOR_VISUAL_JS.contains("visualClearButton"));
        assert!(EDITOR_VISUAL_JS.contains("function visualEditCommandLabel(dimension, command)"));
        assert!(EDITOR_VISUAL3D_JS.contains("syncVisualEditCommandLabels(\"3d\");"));
        assert!(EDITOR_CSS.contains(".visual-editor-toolbar {\n  align-items: flex-start;\n  flex-direction: column;\n  flex-wrap: nowrap;\n  gap: 10px;"));
        assert!(EDITOR_CSS.contains(".visual-toolbar-context-row {\n  gap: 10px;"));
        assert!(EDITOR_CSS.contains(".visual-toolbar-operation-row {\n  gap: 12px;"));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="square""#));
        assert!(EDITOR_HTML.contains(r#"data-editor-icon="box""#));
        assert!(!EDITOR_HTML.contains("visual3d-scope-toggle-label"));
        assert!(
            EDITOR_CSS
                .contains(".visual-paint-tool-button {\n  border: 0;\n  background: transparent;")
        );
        assert!(!EDITOR_CSS.contains(".visual-paint-tool-button {\n  background: var(--bar-bg);"));
    }

    #[test]
    fn visual_2d_and_3d_share_toolbar_marker_grid_and_tag_ui() {
        assert!(EDITOR_HTML.contains(r#"data-visual-dimension="2d""#));
        assert!(EDITOR_HTML.contains(r#"data-visual-dimension="3d""#));
        assert!(EDITOR_HTML.contains(r#"id="visualBrushSizeInput""#));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("visualBrushDiameterForSize(Math.min(plane.width, plane.height))")
        );
        assert!(EDITOR_VISUAL_JS.contains("const VISUAL_EDITOR_TOOL_SCHEMA = Object.freeze(["));
        assert!(EDITOR_VISUAL_JS.contains("function visualEditorToolbarParts(dimension)"));
        assert!(EDITOR_VISUAL_JS.contains("grid: visualGridButton,"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("clip: is3d ? visual3dClipActions : renderVisualClipActions(),")
        );
        assert!(
            EDITOR_VISUAL_JS.contains("const groups = { context, paint, transform, clipboard };")
        );
        assert!(EDITOR_VISUAL_JS.contains("row.append(contextRow, operationRow);"));
        assert!(EDITOR_VISUAL_JS.contains("operationRow.append(paint, transform, clipboard);"));
        assert!(
            !EDITOR_VISUAL_JS
                .contains("global.querySelector(\".visual3d-scope-toggle, .visual-clip-actions\")")
        );
        assert!(EDITOR_VISUAL_JS.contains("function renderVisualPaletteGrid({"));
        assert!(EDITOR_VISUAL_JS.contains("renderVisualPaletteGrid({\n    target: visualPalette,"));
        assert!(
            EDITOR_VISUAL3D_JS.contains("renderVisualPaletteGrid({\n    target: visual3dPalette,")
        );
        assert!(!EDITOR_VISUAL3D_JS.contains("const paletteGrid = document.createElement"));
        assert!(
            EDITOR_VISUAL_JS.contains("function renderVisualShapeBindControl(target, options)")
        );
        assert!(EDITOR_VISUAL_JS.contains("renderVisualShapeBindControl(visualShapeField,"));
        assert!(EDITOR_VISUAL3D_JS.contains("renderVisualShapeBindControl(visual3dShapeField,"));
        assert!(
            EDITOR_VISUAL_JS.contains("function renderVisualEditorUpperControls(target, controls)")
        );
        assert!(EDITOR_VISUAL_JS.contains("visualEditorUpperControls2d(),"));
        assert!(EDITOR_VISUAL3D_JS.contains("visualEditorUpperControls3d(),"));
        assert!(!EDITOR_VISUAL_JS.contains("controls.depthInput"));
        assert!(!EDITOR_VISUAL3D_JS.contains("document.createElement(\"label\")"));
        assert!(EDITOR_CSS.contains(".visual-editor-name-row,\n.visual-editor-geometry-group,"));
        assert!(EDITOR_CSS.contains(
            ".visual-editor-upper-controls {\n  width: 100%;\n  min-width: 0;\n  display: flex;\n  flex-wrap: wrap;"
        ));
        assert!(
            EDITOR_CSS
                .contains(".visual-editor-name-row {\n  flex: 0 1 470px;\n  flex-wrap: nowrap;")
        );
        assert!(
            EDITOR_CSS.contains(
                ".visual-builder:not(.is-animation-mode) .visual-editor-animation-group,"
            )
        );
        assert!(!EDITOR_CSS.contains(".visual3d-animation-control"));
        assert!(EDITOR_VISUAL3D_JS.contains("renderVisualCurrentColorTagButton({"));
        assert!(EDITOR_VISUAL3D_JS.contains("if (visual3dGridVisible) {"));
        assert!(EDITOR_VISUAL3D_JS.contains("--visual3d-voxel-grid-stroke"));
        assert!(
            EDITOR_CSS.contains(".visual-duration-input {\n  min-height: var(--icon-button-size);")
        );
        assert!(EDITOR_CSS.contains(".visual-controls .visual-duration-input input {\n  min-height: calc(var(--icon-button-size) - 2px);"));
        let toolbar_2d = EDITOR_HTML
            .find(r#"id="visualToolbarHost""#)
            .expect("2D toolbar host");
        let source_2d = EDITOR_HTML
            .find(r#"id="visualSourceActionBank""#)
            .expect("2D source action bank");
        let toolbar_3d = EDITOR_HTML
            .find(r#"id="visual3dToolbarHost""#)
            .expect("3D toolbar host");
        let source_3d = EDITOR_HTML
            .find(r#"id="visual3dSourceActionBank""#)
            .expect("3D source action bank");
        let palette_3d = EDITOR_HTML
            .find(r#"id="visual3dPalette""#)
            .expect("3D palette");
        let controls_3d = EDITOR_HTML[..palette_3d]
            .rfind(r#"<div class="visual-controls">"#)
            .expect("3D visual controls");
        let width_input_3d = EDITOR_HTML
            .find(r#"id="visual3dWidthInput" type="number""#)
            .expect("3D width input");
        let height_input_3d = EDITOR_HTML
            .find(r#"id="visual3dHeightInput" type="number""#)
            .expect("3D height input");
        let depth_input_3d = EDITOR_HTML
            .find(r#"id="visual3dDepthInput" type="number""#)
            .expect("3D depth input");
        assert!(controls_3d < width_input_3d);
        assert!(width_input_3d < height_input_3d);
        assert!(height_input_3d < depth_input_3d);
        assert!(depth_input_3d < palette_3d);
        assert!(source_2d < toolbar_2d);
        assert!(source_3d < toolbar_3d);
        assert!(EDITOR_HTML.contains(
            r#"id="visual3dUpdateButton" class="icon-button source-action-button visual-update-source-button""#
        ));
        assert!(EDITOR_CSS.contains(
            ".visual-pane-source-actions {\n  display: inline-flex;\n  align-items: center;"
        ));
        assert!(EDITOR_CSS.contains(".visual-builder .visual-shape-name-input {"));
        assert!(EDITOR_CSS.contains("font: inherit;\n  font-size: 13px;\n  font-weight: 800;"));
        let board_3d = EDITOR_HTML
            .find(r#"id="visual3dSliceBoard""#)
            .expect("3D visual board");
        let preview_3d = EDITOR_HTML
            .find(r#"class="visual3d-preview-wrap""#)
            .expect("3D preview");
        assert!(board_3d < preview_3d);
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="visualAnimationFrameInput""#)
                .count(),
            1
        );
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="visualAnimationFrameStrip""#)
                .count(),
            1
        );
        assert!(!EDITOR_HTML.contains(r#"id="visual3dAnimationFrameInput""#));
        assert!(
            EDITOR_DOM_JS
                .contains("const visual3dAnimationFrameInput = visualAnimationFrameInput;")
        );
        assert!(EDITOR_VISUAL_JS.contains("previewColumn.insertBefore(toolbar, previewStage);"));
        assert!(
            EDITOR_CSS
                .contains(".visual-builder:not(.is-animation-mode) .visual-animation-toolbar,")
        );
    }

    #[test]
    fn visual3d_editor_accepts_depth_one_and_animation_frames() {
        assert!(EDITOR_HTML.contains(r#"id="visual3dWidthInput" type="number""#));
        assert!(EDITOR_HTML.contains(r#"id="visual3dHeightInput" type="number""#));
        assert!(EDITOR_HTML.contains(r#"id="visual3dDepthInput" type="number""#));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("return visual3d.width * visual3d.height * visual3d.depth;")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("if (width < 1 || height < 1 || depth < 1"));
        assert!(!EDITOR_VISUAL3D_JS.contains("width !== height"));
        assert!(EDITOR_VISUAL3D_JS.contains("visual3d.animationMode = loaded.frames.length > 1"));
        assert!(EDITOR_VISUAL3D_JS.contains("function setVisual3dAnimationFrame(index)"));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "durationMs: visual3d.animationMode ? normalizedVisual3dAnimationDuration() : null"
        ));
    }

    #[test]
    fn visual3d_size_and_scale_remap_all_frames_with_explicit_dimensions() {
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("function remapVisual3dFrames(nextExtent, sourceCoordinates)")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("visual3d.frames = frames.map(remap);"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("height: axis === \"height\" ? nextValue : visual3d.height,")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("height: visual3d.height * factor,"));
        assert!(EDITOR_VISUAL3D_JS.contains("height: visual3d.height / factor,"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("size: Math.max(visual3d.width, visual3d.height, visual3d.depth),")
        );
        assert!(EDITOR_JS.contains("visual3d.slice = Math.max(0, Math.min(visual3dAxisSize() - 1"));
        assert!(!EDITOR_VISUAL3D_JS.contains("visual3d.size ="));
        assert!(!EDITOR_VISUAL3D_JS.contains("visual3d.size *"));
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
            "keymap.of([\n        ...sourceCompletionKeymap,\n        ...sourceEditingKeymap,\n        ...foldKeymap,\n        indentWithTab,"
        ));
        assert!(EDITOR_CODEMIRROR_JS.contains("sourcecompletioncommand"));
    }

    #[test]
    fn source_setting_line_add_uses_current_rust_completion_items() {
        assert!(EDITOR_SOURCE_JS.contains("async function refreshSourceLineAdd()"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const list = await suggestSourceCompletionsWithWasm(source, cursor);")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("? (list?.items || []).filter((item) => item?.kind === \"setting\")")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("sourceEditor.addEventListener(\"sourcelineaddrequest\", (event) => {")
        );
        let line_add_flow = EDITOR_SOURCE_JS
            .split("async function refreshSourceLineAdd()")
            .nth(1)
            .and_then(|source| source.split("\n}").next())
            .expect("source line add refresh flow");
        assert!(line_add_flow.contains("suggestSourceCompletionsWithWasm(source, cursor)"));
        assert!(
            line_add_flow.contains("setSourceLineAddVisible(source, cursor, items.length > 0);")
        );
        for forbidden in ["camera", "yaw", "pitch", "zoom", "render"] {
            assert!(
                !line_add_flow.contains(forbidden),
                "source line add must not own the authoring word {forbidden}"
            );
        }
    }

    #[test]
    fn codemirror_setting_line_add_owns_only_widget_mechanics() {
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("class SourceAddLineWidget extends WidgetType")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("button.innerHTML = editorIconSvg(\"plus\");")
        );
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS.contains("anchor.className = \"cm-source-add-anchor\"")
        );
        assert!(EDITOR_CODEMIRROR_SOURCE_JS.contains("new CustomEvent(\"sourcelineaddrequest\""));
        assert!(
            EDITOR_CODEMIRROR_SOURCE_JS
                .contains("setAddLineOverlay(source, cursorOffset, visible)")
        );
        for forbidden in ["camera", "yaw", "pitch", "zoom", "render"] {
            assert!(
                !EDITOR_CODEMIRROR_SOURCE_JS.contains(forbidden),
                "CodeMirror line add must not own the authoring word {forbidden}"
            );
        }
    }

    #[test]
    fn source_entry_refresh_waits_for_the_matching_analysis_edit() {
        assert!(EDITOR_SOURCE_JS.contains("function syncSourceAnalysisEditorChanges("));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const entriesRefreshRequestId = ++sourceEntriesRefreshRequestId;")
        );
        assert!(EDITOR_SOURCE_JS.contains("void analysisEdit.then(() => {"));
        assert!(EDITOR_SOURCE_JS.contains(
            "entriesRefreshRequestId !== sourceEntriesRefreshRequestId\n      || editedSource !== sourceEditorDocumentValue()"
        ));
        assert!(EDITOR_JS.contains("const currentRequest = surfaceEntriesRequest === request;"));
        assert!(EDITOR_JS.contains("&& activeContext?.source === text"));
    }

    #[test]
    fn source_analysis_queries_follow_the_mutation_captured_for_their_source() {
        assert!(EDITOR_RUNTIME_JS.contains("const synchronizedMutation = analysisWorkerMutation;"));
        assert!(EDITOR_RUNTIME_JS.contains("await synchronizedMutation;"));
        assert!(
            !EDITOR_RUNTIME_JS.contains("Editor source analysis changed before the query started.")
        );
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
            "...sourceCompletionKeymap,\n        ...sourceEditingKeymap,\n        ...foldKeymap,\n        indentWithTab,"
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
        assert!(
            EDITOR_WORKSPACE_JS.contains("const previousActiveFileId = loadedSourceDocumentId;")
        );
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
            .find("function sourceDocumentsForPreviewBuild(build)")
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
            "function sourceCompletionItemsForRequest(list, source, cursor, options = {})"
        ));
        assert!(EDITOR_SOURCE_JS.contains(
            "const items = filterSourceCompletionsForTypedReplacement(\n    candidates,\n    list,\n    source,\n    cursor,\n  );"
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
        assert!(!EDITOR_SOURCE_JS.contains("filterSourceCompletionsForDocument"));
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
        assert!(level_block.contains("background-color: var(--visual-swatch-bg);"));
        assert!(level_block.contains("background-image: var(--visual-swatch-checker);"));
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
    fn visual_source_loader_consumes_lang_visual_contract_instead_of_source_parsing() {
        assert!(!EDITOR_VISUAL_JS.contains("`${tableName}:*`"));
        assert!(!EDITOR_VISUAL_JS.contains(":*"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains(
            "state.sourceVisualContract = target?.sourceVisual && typeof target.sourceVisual === \"object\""
        ));
        assert!(EDITOR_VISUAL_JS.contains("function visualSourceColorAssets()"));
        assert!(EDITOR_VISUAL_JS.contains("function visualSourceShapeAssets()"));
        assert!(EDITOR_VISUAL_JS.contains("Array.isArray(contract?.colorAssets)"));
        assert!(EDITOR_VISUAL_JS.contains("Array.isArray(contract?.shapeAssets)"));
        assert!(EDITOR_VISUAL_JS.contains("Array.isArray(contract?.resolvedPalette)"));
        assert!(EDITOR_VISUAL_JS.contains("Array.isArray(contract?.resolvedShapeRows)"));
        for forbidden in [
            "parseVisualColorAssets",
            "parseVisualShapeAssets",
            "resolveVisualColorAssetToken",
            "resolveVisualShapeAssetToken",
            "visualPaletteEntryFromSourceToken",
            "parseVisualValueMaps",
            "collectVisualShapeTableRows",
            "collectVisualShapeRotationBlocks",
            "parseVisualShapeRotationDirective",
            "expandVisualShapeRotationRows",
            "visualTableAssetKey",
            "firstVisualTableAssetKey",
            "visualSelectorSingleTagBinding",
        ] {
            assert!(
                !EDITOR_VISUAL_JS.contains(forbidden),
                "{forbidden} should not exist in editor visual source loading"
            );
        }
    }

    #[test]
    fn visual_color_default_names_use_object_base_without_tag_or_color_suffix() {
        assert!(EDITOR_VISUAL_JS.contains("if (kind === \"color\") {"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("const objectName = String(visualObjectName()).split(\":\")[0];")
        );
        assert!(EDITOR_VISUAL_JS.contains("return `${base}_${Number(index) + 1}`;"));
        assert!(EDITOR_VISUAL_JS.contains("return `${base}_${kind}_${Number(index) + 1}`;"));
    }

    #[test]
    fn visual3d_source_generation_does_not_add_indents() {
        assert!(EDITOR_VISUAL3D_JS.contains("const VISUAL3D_SOURCE_INDENT = \"\";"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function visual3dSourceChildIndent"));
        assert!(!EDITOR_VISUAL3D_JS.contains("replaceVisual3dDefinition"));
    }

    #[test]
    fn visual_source_loader_accepts_bare_shape_refs() {
        assert!(EDITOR_VISUAL_JS.contains(
            "const loaded = parseVisualDefinitionSource(target.sourceVisual, targetName);"
        ));
        assert!(
            EDITOR_VISUAL_DOCUMENT_JS
                .contains("const resolvedPalette = Array.isArray(contract.resolvedPalette)")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("const shapeRows = Array.isArray(contract.resolvedShapeRows)")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("shapeBind = { type: \"shape\", name: shapeName, linked: true };")
        );
    }

    #[test]
    fn visual_source_mutation_is_owned_by_the_lang_contract() {
        assert!(EDITOR_VISUAL_JS.contains("function syncCurrentVisualDefinitionFromBuilder("));
        assert!(EDITOR_VISUAL_JS.contains("await commitVisualEditorMutation({"));
        assert!(EDITOR_VISUAL_JS.contains("request: () => visualEditMutationRequest(\"update\")"));

        for forbidden in [
            "findVisualsBlock",
            "findVisualAssetBlock",
            "findVisualColorDefinitionRange",
            "findVisualShapeDefinitionRange",
            "ensureVisualColorDefinition",
            "ensureVisualShapeDefinition",
            "replaceVisualColorDefinition",
            "replaceVisualShapeDefinition",
            "visualObjectDefinitionText",
            "topLevelDepthAt",
        ] {
            assert!(
                !EDITOR_VISUAL_JS.contains(forbidden),
                "{forbidden} must remain owned by puzzle-lang"
            );
        }
    }

    #[test]
    fn visual_source_loader_projects_generic_refs_from_lang_contract() {
        assert!(EDITOR_VISUAL_JS.contains(
            "for (const entry of Array.isArray(contract?.resolvedPalette) ? contract.resolvedPalette : [])"
        ));
        assert!(EDITOR_VISUAL_JS.contains("if (entry?.linked && name && color)"));
        assert!(EDITOR_VISUAL_JS.contains(
            "const source = String(entry?.source || paletteTokens[index] || \"\").trim();"
        ));
        assert!(
            EDITOR_VISUAL_JS
                .contains("paletteEntry.bind = { type: \"color\", name: source, linked: true };")
        );
    }

    #[test]
    fn visual_color_tag_picker_shows_color_values() {
        assert!(EDITOR_VISUAL_JS.contains("const colorAssets = visualSourceColorAssets();"));
        assert!(
            EDITOR_VISUAL_JS.contains("optionMeta: (name) => ({ color: colorAssets.get(name) })")
        );
        assert!(EDITOR_VISUAL_JS.contains("className = \"visual-tag-option-swatch\""));
        assert!(EDITOR_VISUAL_JS.contains("className = \"visual-tag-option-value\""));
        assert!(EDITOR_CSS.contains(".visual-tag-option.has-color"));
        assert!(EDITOR_CSS.contains(".visual-tag-option.has-invalid-color"));
        assert!(EDITOR_CSS.contains(".visual-tag-option-swatch"));
        assert!(EDITOR_CSS.contains(".visual-tag-option-value"));
    }

    #[test]
    fn visual_source_loader_reads_resolved_shape_rows_from_lang_contract() {
        assert!(EDITOR_VISUAL_JS.contains("const shapes = visualSourceShapeAssets();"));
        assert!(EDITOR_VISUAL_JS.contains("const rows = Array.isArray(entry?.rows)"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("const resolvedRows = Array.isArray(contract?.resolvedShapeRows)")
        );
        assert!(EDITOR_VISUAL_JS.contains("assets.set(shapeName, resolvedRows);"));
        let contract_error_start = EDITOR_VISUAL_JS
            .find("function visualSourceContractError(contract)")
            .expect("visual source contract validation");
        let contract_error_end = EDITOR_VISUAL_JS[contract_error_start..]
            .find("function visualPaletteEntrySourceToken")
            .map(|offset| contract_error_start + offset)
            .expect("visual source contract validation end");
        let contract_error = &EDITOR_VISUAL_JS[contract_error_start..contract_error_end];
        assert!(contract_error.contains("if (shapeName && !shapeRows.length)"));
        assert!(contract_error.contains("return `Cannot resolve shape ${shapeName}`;"));

        let loader_start = EDITOR_VISUAL_JS
            .find("function loadVisualSourceTarget(target, options = {})")
            .expect("visual source target loader");
        let loader_end = EDITOR_VISUAL_JS[loader_start..]
            .find("function visualSourceContractError")
            .map(|offset| loader_start + offset)
            .expect("visual source target loader end");
        let loader = &EDITOR_VISUAL_JS[loader_start..loader_end];
        assert!(
            loader
                .contains("const contractError = visualSourceContractError(target.sourceVisual);")
        );
        assert!(loader.contains("const message = contractError"));
        assert!(loader.contains("setVisualActionStatus(message, status);"));
        assert!(loader.contains("setStatus(message, status);"));
    }

    #[test]
    fn visual_source_loader_preserves_visual_prelude_rows() {
        assert!(EDITOR_JS.contains("sourcePreludeRows: [],"));
        assert!(
            EDITOR_JS.contains(
                "sourcePreludeRows: cloneVisualEditValue(visual.sourcePreludeRows || []),"
            )
        );
        assert!(EDITOR_VISUAL_JS.contains(
            "const loaded = parseVisualDefinitionSource(target.sourceVisual, targetName);"
        ));
        assert!(!EDITOR_VISUAL_JS.contains(
            "parseVisualDefinitionSource(source.slice(target.bodyStart, target.bodyEnd)"
        ));
        assert!(!EDITOR_VISUAL_JS.contains("function isVisualSourcePreludeRow(row)"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("const sourcePreludeRows = Array.isArray(contract.preludeRows)")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("const paletteTokens = Array.isArray(contract.paletteTokens)")
        );
        assert!(
            EDITOR_VISUAL_JS.contains("const shapeName = typeof contract.shapeRef === \"string\"")
        );
        assert!(EDITOR_VISUAL_JS.contains("sourcePreludeRows,"));
        assert!(EDITOR_VISUAL_JS.contains("preludeRows: visual.sourcePreludeRows || [],"));
    }

    #[test]
    fn visual_source_loader_handles_animation_frames() {
        assert!(EDITOR_VISUAL_JS.contains(
            "const semanticFrames = Array.isArray(contract.frames) ? contract.frames : [];"
        ));
        assert!(EDITOR_VISUAL_JS.contains("animationMode: true,"));
        assert!(
            EDITOR_VISUAL_JS.contains(
                "const frameDurationMs = Number.isFinite(Number(contract.frameDurationMs))"
            )
        );
        assert!(EDITOR_VISUAL_JS.contains("frameDurationMs * parsedFrames.length"));
        assert!(EDITOR_VISUAL_JS.contains("animationDurationMs: durationMs,"));
        assert!(EDITOR_VISUAL_JS.contains("animationFrames: parsedFrames,"));
        assert!(EDITOR_VISUAL_JS.contains("frames: visualEditFrames(),"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("durationMs: visual.animationMode ? visual.animationDurationMs : null,")
        );
    }

    #[test]
    fn visual_animation_settings_are_visual_undo_state() {
        assert!(EDITOR_JS.contains("animationDurationMs: visual.animationDurationMs,"));
        assert!(EDITOR_JS.contains("animationFrameCount: visual.animationFrameCount,"));
        assert!(
            EDITOR_JS
                .contains("animationFrames: cloneVisualEditValue(visual.animationFrames || []),")
        );
        assert!(EDITOR_JS.contains(
            "visual.animationDurationMs = Number.isFinite(Number(state.animationDurationMs))"
        ));
        assert!(EDITOR_VISUAL_JS.contains("const before = visualEditSnapshot(\"visual\");\n  visual.animationFrameCount = normalizedVisualAnimationFrameCount(value);"));
        assert!(EDITOR_VISUAL_JS.contains(
            "const nextDuration = normalizedVisualAnimationDuration(value);\n  const changed = nextDuration !== visual.animationDurationMs;"
        ));
        assert!(EDITOR_VISUAL_JS.contains(
            "const before = options.recordHistory === false || !changed ? null : visualEditSnapshot(\"visual\");\n  visual.animationDurationMs = nextDuration;"
        ));
        assert!(
            EDITOR_VISUAL_JS.contains(
                "if (before) {\n    pushVisualEditUndoSnapshot(\"visual\", before);\n  }"
            )
        );
        assert!(EDITOR_VISUAL_JS.contains("function isVisualEditUndoTarget(target)"));
        assert!(EDITOR_VISUAL_JS.contains("function syncVisualAnimationInputValues(options = {})"));
        assert!(EDITOR_JS.contains("syncVisualAnimationInputValues();"));
        assert!(EDITOR_JS.contains("isVisualEditUndoTarget(target)"));
    }

    #[test]
    fn visual_animation_playback_view_is_separate_from_frame_panel() {
        assert!(EDITOR_HTML.contains(r#"aria-label="Visual animation frames""#));
        assert!(EDITOR_HTML.contains(r#"class="visual-animation-sidecar""#));
        assert!(EDITOR_HTML.contains(r#"class="visual-animation-playback-panel""#));
        assert!(EDITOR_HTML.contains("visualAnimationPlaybackView"));
        assert!(EDITOR_HTML.contains("visual-animation-playback-view-label"));
        assert!(EDITOR_CSS.contains(".visual-animation-playback-panel {\n  position: relative;"));
        assert!(
            EDITOR_CSS.contains(".visual-animation-playback-view-label {\n  position: absolute;")
        );
        let playback_panel = EDITOR_HTML
            .find(r#"class="visual-animation-playback-panel""#)
            .expect("visual animation playback panel");
        let frame_panel = EDITOR_HTML
            .find(r#"id="visualAnimationPanel""#)
            .expect("visual animation frame panel");
        assert!(playback_panel < frame_panel);
        assert!(EDITOR_CSS.contains(".visual-animation-sidecar {\n  min-width: 72px;"));
        assert!(EDITOR_VISUAL_JS.contains("function renderVisualAnimationPlaybackView(cells)"));
        assert!(EDITOR_VISUAL_JS.contains("function syncVisualAnimationPlayback()"));
        assert!(EDITOR_VISUAL_JS.contains("function visualAnimationFrameDelayMs()"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("Math.round(context.durationMs() / context.state.animationFrameCount)")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("visualAnimationPlaybackDurationMs !== visualAnimationFrameDelayMs()")
        );
        assert!(
            EDITOR_VISUAL_JS.contains("visualAnimationDurationInput?.addEventListener(\"input\"")
        );
        assert!(EDITOR_VISUAL_JS.contains("recordHistory: false"));
        assert!(EDITOR_VISUAL_JS.contains("function visualAnimationFrameCells(cells)"));
        assert!(EDITOR_VISUAL_JS.contains("button.classList.toggle(\"is-playing-frame\""));
        assert!(!EDITOR_HTML.contains("visualAnimationPlayButton"));
        assert!(!EDITOR_DOM_JS.contains("visualAnimationPlayButton"));
        assert!(!EDITOR_VISUAL_JS.contains("toggleVisualAnimationPlayback"));
        assert!(!EDITOR_HTML.contains("visualAnimationCurrentPreview"));
        assert!(!EDITOR_HTML.contains("visual-animation-preview-label"));
        assert!(!EDITOR_HTML.contains("visualAnimationPlaybackPreview"));
        assert!(!EDITOR_CSS.contains(".visual-animation-preview-box"));
        assert!(!EDITOR_CSS.contains(".visual-animation-preview,"));
        assert!(!EDITOR_VISUAL_JS.contains("renderVisualAnimationPreview"));
        assert!(!EDITOR_HTML.contains(r#"aria-label="Visual animation playback and frames""#));
    }

    #[test]
    fn level3d_frame_surface_is_square_cornered() {
        assert!(EDITOR_CSS.contains(
            ".level3d-frame-surface {\n  position: absolute;\n  inset: 0 auto auto 0;\n  width: var(--level3d-frame-virtual-width);\n  height: var(--level3d-frame-virtual-height);\n  border: 0;\n  border-radius: 0;"
        ));
    }

    #[test]
    fn visual_source_update_reveals_and_preserves_target_boundary() {
        assert!(EDITOR_JS.contains("editSourceName: \"\""));
        assert!(EDITOR_JS.contains("editSourceEnd: null"));
        assert!(EDITOR_JS.contains("editSourceBodyStart: null"));
        assert!(EDITOR_JS.contains("editSourceBodyEnd: null"));
        assert!(EDITOR_VISUAL_JS.contains("function revealVisualSourceResult"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("function commitVisualEditorMutation(options)"));
        assert!(
            EDITOR_VISUAL_DOCUMENT_JS.contains("const visualEditorMutationQueues = new WeakMap();")
        );
        assert!(
            EDITOR_VISUAL_DOCUMENT_JS
                .contains(".then(() => commitVisualEditorMutationNow(options));")
        );
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("revealVisualSourceResult(document, result);"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("sourceEditor.focus({ preventScroll: true });"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("function visualEditorSourceRange"));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("state.editSourceEnd"));
        assert!(EDITOR_VISUAL_JS.contains("function currentVisualEditSourceRange(source)"));
        assert!(EDITOR_VISUAL_JS.contains("commitVisualEditorMutation({"));
        assert!(EDITOR_JS.contains(
            "const trailingBoundary = removed.match(/((?:\\r?\\n[\\t ]*)+)$/)?.[1] || \"\";"
        ));
    }

    #[test]
    fn shared_visual_document_controller_loads_before_dimension_views() {
        let document = EDITOR_HTML
            .find(r#"<script src="editor_visual_document.js"></script>"#)
            .expect("editor loads shared visual document controller");
        let visual2d = EDITOR_HTML
            .find(r#"<script src="editor_visual.js"></script>"#)
            .expect("editor loads 2D visual view");
        let visual3d = EDITOR_HTML
            .find(r#"<script src="editor_visual3d.js"#)
            .expect("editor loads 3D visual view");
        assert!(document < visual2d);
        assert!(document < visual3d);
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("function projectVisualDocumentContract"));
        assert!(!EDITOR_VISUAL_DOCUMENT_JS.contains("findMatchingBrace"));
    }

    #[test]
    fn visual_source_edit_invalidates_cached_target_until_rust_resync() {
        assert!(EDITOR_VISUAL_JS.contains("function clearVisualEditSource()"));
        assert!(EDITOR_VISUAL_JS.contains(
            "function invalidateVisualEditSourceForDocument(document = activeDocument())"
        ));
        assert!(EDITOR_VISUAL_DOCUMENT_JS.contains("clearVisualEditorSourceTarget(state);"));
        assert!(EDITOR_VISUAL_JS.contains(
            "sourceEditor.addEventListener(\"input\", () => {\n  invalidateVisualEditSourceForDocument(activeDocument());\n  syncVisualSourceActionButtons();\n});"
        ));
        assert!(EDITOR_VISUAL_JS.contains("function loadVisualSourceTarget(target, options = {})"));
        assert!(EDITOR_VISUAL_JS.contains("setVisualEditSource(target, activeDocument());"));
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
    fn visual_source_has_no_legacy_js_target_scanner() {
        for forbidden in [
            "findVisualDefinitionAtPosition",
            "findVisualDefinitionBlock",
            "findUnbracedVisualDefinition",
            "isVisualDefinitionBoundary",
            "isLineStyleVisualDefinitionBoundary",
            "isVisualDefinitionNameToken",
            "isVisualColorRow(",
            "loadVisualFromSourcePosition",
            "registerSourceEditableTarget?.(\"visual\"",
        ] {
            assert!(
                !EDITOR_VISUAL_JS.contains(forbidden),
                "legacy visual source scanner remains: {forbidden}"
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
    fn create_source_file_adds_new_files_inside_workspace() {
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

        let created_3d = service
            .create_source_file(&CreateSourceFileRequest::new(
                "puzzle imported3 {\ndimension = 3\n}\n",
                "imported3.puzzle",
            ))
            .expect("create new 3D puzzle file");
        assert!(created_3d.ends_with("imported3.puzzle"));
        assert_eq!(
            fs::read_to_string(&created_3d).expect("read created 3D puzzle file"),
            "puzzle imported3 {\ndimension = 3\n}\n"
        );

        let outside_error = service
            .create_source_file(&CreateSourceFileRequest::new(
                editor_fixture_source("Outside"),
                outside_path.display().to_string(),
            ))
            .expect_err("creating outside the editor workspace should be rejected")
            .to_string();
        assert!(outside_error.contains("can only create files under"));

        let created_text = service
            .create_source_file(&CreateSourceFileRequest::new("notes\n", "notes.md"))
            .expect("create new text file");
        assert!(created_text.ends_with("notes.md"));
        assert_eq!(
            fs::read_to_string(&created_text).expect("read created text file"),
            "notes\n"
        );
    }

    #[test]
    fn rename_workspace_entry_renames_real_files_inside_workspace() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Rename Before"),
        );
        let project_dir = game_path.parent().expect("project dir");
        let service = EditorService::open_path(project_dir).expect("open editor fixture");

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
        let service = EditorService::open_path(project_dir).expect("open editor fixture");

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
        let service = EditorService::open_path(project_dir).expect("open editor fixture");

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
        let service = EditorService::open_path(project_dir).expect("open editor fixture");

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
        let service = EditorService::open_path(project_dir).expect("open editor fixture");

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
    fn open_path_scopes_workspace_to_selected_project_folder() {
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

        let service = EditorService::open_path(project_dir).expect("open project folder");
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
        let service = EditorService::open_path(project_dir).expect("open project folder");

        let error = service
            .compile_preview(&PreviewRequest::new(
                editor_fixture_source("Project B changed"),
                outside_path.display().to_string(),
                String::new(),
            ))
            .expect_err("preview paths outside the opened project must be rejected")
            .to_string();
        assert!(error.contains("workspace entry is outside root"), "{error}");
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

        let service = EditorService::open_path(project_dir).expect("open project folder");

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
        assert!(EDITOR_RUNTIME_JS.contains("WasmWorkspaceSession"));
        assert!(EDITOR_RUNTIME_JS.contains("session.compile_preview"));
        assert!(EDITOR_RUNTIME_JS.contains("session.export_html"));
        assert!(EDITOR_RUNTIME_JS.contains("session.presentation_manifest"));
        assert!(!EDITOR_RUNTIME_JS.contains("compile_workspace_preview"));
        assert!(!EDITOR_RUNTIME_JS.contains("export_workspace_html"));
        assert!(EDITOR_RUNTIME_JS.contains("session.index_json"));
        assert!(!EDITOR_WORKSPACE_JS.contains("puzzleImportPathsForDocument"));
        assert!(!EDITOR_WORKSPACE_JS.contains("documentImportClosureContains"));
        assert!(!EDITOR_WORKSPACE_JS.contains("expandedWorkspaceSourceForEditor"));
        assert!(!EDITOR_WORKSPACE_JS.contains("declaredAssetPaths"));
        assert!(!EDITOR_WORKSPACE_JS.contains("themeNameFromPuzzleSource"));
        assert!(EDITOR_RUNTIME_JS.contains("querySynchronizedAnalysisWorker(\"highlightRange\""));
        assert!(EDITOR_ANALYSIS_WORKER_JS.contains("active_source_analysis_highlight_range_json"));
        assert!(!EDITOR_RUNTIME_JS.contains("solver_task_initial_display_state_json"));
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
        assert!(EDITOR_RUNTIME_JS.contains("playerRuntimeAssets()"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_player/puzzle_wasm_player.js"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_player/puzzle_wasm_player_bg.wasm"));
        assert!(EDITOR_RUNTIME_JS.contains("playerRuntimeAssetsPromise"));
        assert!(EDITOR_RUNTIME_JS.contains(
            "const runtimeAssets = await window.PuzzleStudioRuntime.playerRuntimeAssets();"
        ));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("exportStandaloneHtml({"));
        assert!(!EDITOR_IMPORT_EXPORT_JS.contains("html: previewBuild?.html"));
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
        assert!(EDITOR_WORKSPACE_JS.contains("if (previewTargetChanged) {"));
        assert!(EDITOR_WORKSPACE_JS.contains("renderPreview().catch((error) => {"));
        assert!(!EDITOR_WORKSPACE_JS.contains("renderActivePreviewAfterWorkspaceSelection"));
        assert!(!EDITOR_WORKSPACE_JS.contains("treeWithEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("mergeEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewHtml"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewError"));
        assert!(!EDITOR_WORKSPACE_JS.contains("document.previewHtml ||"));
        assert!(!EDITOR_WORKSPACE_JS.contains("previewDocument?.previewHtml"));
        assert!(EDITOR_WORKSPACE_JS.contains("const previewTargetUnchanged = previewDocument"));
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
        assert!(PUZZLE_WASM_JS.contains("export class WasmSolverService"));
        assert!(!PUZZLE_WASM_JS.contains("export function solve_state"));
        assert!(!PUZZLE_WASM_JS.contains("solver_task_initial_display_state_json"));
        assert!(!PUZZLE_WASM_JS.contains("compile_workspace_solver_rules_json"));
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
    fn visual3d_preview_uses_runtime_visual_ordering_contract() {
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function comparePrimitiveOrder(a, b)"));
        assert!(PUZZLE3_VISUAL_CORE_JS.contains("function faceGridOrder(corners, view)"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("sceneFaces.sort(Puzzle3VisualCore.comparePrimitiveOrder);")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("return Puzzle3VisualCore.faceGridOrder(corners, visual3dVisualView());")
        );
        assert!(EDITOR_VISUAL3D_JS.contains("const previewOwner = visual3dPreviewRenderOwner();"));
        assert!(EDITOR_VISUAL3D_JS.contains("ownerCell: previewOwner"));
        assert!(EDITOR_VISUAL3D_JS.contains("renderPriority: order"));
        assert!(EDITOR_VISUAL3D_JS.contains("assignVisual3dPrimitiveOrder(sceneFaces);"));
        assert!(EDITOR_VISUAL3D_JS.contains("primitive.frameIndex = index;"));
        assert!(EDITOR_VISUAL3D_JS.contains("primitive.stableKey = occurrence === 0 ? baseKey"));
        assert!(EDITOR_VISUAL3D_JS.contains("rectsFromCells: visual3dUnitFaceRects"));
        assert!(!EDITOR_VISUAL3D_JS.contains("function compareVisual3dSceneFaceOrder"));
    }

    #[test]
    fn visual3d_presentation_changes_redraw_all_animation_previews() {
        let redraw = EDITOR_VISUAL3D_JS
            .split_once("function renderVisual3dPresentationSurfaces() {")
            .expect("shared 3D presentation redraw")
            .1
            .split_once("\n}")
            .expect("shared 3D presentation redraw end")
            .0;
        assert!(redraw.contains("renderVisual3dPreview();"));
        assert!(redraw.contains("renderVisual3dAnimationFrameStrip();"));
        assert!(redraw.contains("renderSharedVisualAnimationPlaybackView(context, frame);"));

        for function_name in [
            "toggleVisual3dGrid",
            "resetVisual3dCamera",
            "setVisual3dCameraValue",
        ] {
            let body = EDITOR_VISUAL3D_JS
                .split_once(&format!("function {function_name}"))
                .unwrap_or_else(|| panic!("missing {function_name}"))
                .1
                .split_once("\n}")
                .unwrap_or_else(|| panic!("missing {function_name} end"))
                .0;
            assert!(
                body.contains("renderVisual3dPresentationSurfaces();"),
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
        assert_eq!(EDITOR_STATIC_RENDERER_CSS, RENDERER_CSS);
        assert!(EDITOR_HTML.contains(r#"<link rel="stylesheet" href="renderer.css">"#));
        assert!(EDITOR_HTML.contains(r#"<script src="render_asset_decoder.js"></script>"#));
        assert!(
            EDITOR_HTML.contains(r#"<script src="renderer.js?v=typed-render-scene"></script>"#)
        );
        assert!(EDITOR_HTML.contains(r#"<script src="editor_authoring_renderer.js"></script>"#));
        assert!(EDITOR_AUTHORING_RENDERER_JS.contains("class PuzzleAuthoringRenderer"));
        assert!(!EDITOR_AUTHORING_RENDERER_JS.contains("paintCanvas"));
    }

    #[test]
    fn editor_routes_only_authoring_grids_to_the_dom_renderer() {
        assert!(EDITOR_DOM_JS.contains("new window.PuzzleAuthoringRenderer(levelBoard"));
        assert!(EDITOR_JS.contains("new window.PuzzleAuthoringRenderer(view"));
        assert!(EDITOR_JS.contains("new window.PuzzleAuthoringRenderer(root"));
        assert!(!EDITOR_AUTHORING_RENDERER_JS.contains("resolveRenderMoment"));
        assert!(!EDITOR_AUTHORING_RENDERER_JS.contains("projectRendererState"));
    }

    #[test]
    fn editor_solver_projects_every_unresolved_state_through_rust() {
        assert!(EDITOR_DOM_JS.contains("new window.PuzzleRenderer(solverBoard"));
        assert!(EDITOR_RUNTIME_JS.contains("async projectRendererState(payload = {})"));
        assert!(EDITOR_RUNTIME_JS.contains("module.project_renderer_state("));
        assert!(EDITOR_JS.contains("if (solverRenderer && scene.renderScene)"));
        assert!(EDITOR_JS.contains("window.PuzzleStudioRuntime.projectRendererState({"));
        assert!(EDITOR_JS.contains("solverRenderer.render(solverRenderProjectionScene)"));
        assert!(!EDITOR_JS.contains("new window.PuzzleRenderer(levelBoard"));
    }

    #[test]
    fn visual3d_editor_resyncs_if_script_loads_after_pane_selection() {
        assert!(EDITOR_VISUAL3D_JS.contains("function syncVisual3dBuilderAfterScriptLoad()"));
        assert!(EDITOR_VISUAL3D_JS.contains("currentPreviewMode === \"visual3d\""));
        assert!(
            EDITOR_VISUAL3D_JS.contains("loadFirstFocusedPuzzleEntry(\"visual\", \"visual3d\")")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("resetVisual3dBuilder();\nsyncVisual3dBuilderAfterScriptLoad();")
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
        let visual3d = EDITOR_HTML
            .find(r#"<script src="editor_visual3d.js"#)
            .expect("editor loads 3D visual editor");

        assert!(core < level3d);
        assert!(level3d < import_export);
        assert!(import_export < editor);
        assert!(core < editor);
        assert!(core < visual3d);
    }

    #[test]
    fn tauri_editor_busts_cache_for_theme_css_and_tab_unsaved_assets() {
        assert!(
            EDITOR_HTML.contains(r#"<script src="editor_boot.js?v=desktop-export-link"></script>"#)
        );
        assert!(
            EDITOR_HTML.contains(r#"<script src="renderer.js?v=typed-render-scene"></script>"#)
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
    fn visual3d_preview_slice_selection_uses_ray_hits_before_height_fallback() {
        assert!(EDITOR_VISUAL3D_JS.contains("function visual3dPreviewRay(point, view)"));
        assert!(EDITOR_VISUAL3D_JS.contains("function visual3dRaycastOccupiedVoxel(ray)"));
        assert!(EDITOR_VISUAL3D_JS.contains("const voxelHit = visual3dRaycastOccupiedVoxel(ray);"));
        assert!(EDITOR_VISUAL3D_JS.contains("return visual3dApproximateSliceFromRay(ray);"));
    }

    #[test]
    fn visual_editors_allow_vertical_camera_pitch() {
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_CAMERA_MIN_PITCH_DEGREES = -90;"));
        assert!(EDITOR_LEVEL3D_JS.contains("const LEVEL3D_CAMERA_MAX_PITCH_DEGREES = 90;"));
        assert!(EDITOR_VISUAL3D_JS.contains("const VISUAL3D_CAMERA_MIN_PITCH_DEGREES = -90;"));
        assert!(EDITOR_VISUAL3D_JS.contains("const VISUAL3D_CAMERA_MAX_PITCH_DEGREES = 90;"));
        assert!(EDITOR_LEVEL3D_JS.contains("LEVEL3D_CAMERA_MAX_PITCH_DEGREES"));
        assert!(EDITOR_VISUAL3D_JS.contains("VISUAL3D_CAMERA_MAX_PITCH_DEGREES"));
        assert!(!EDITOR_LEVEL3D_JS.contains("level3dClampNumber(value, -80, 80)"));
        assert!(!EDITOR_VISUAL3D_JS.contains("visual3dClampNumber(value, -80, 80)"));
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
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dTopDownVisualProjection("));
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
    fn visual3d_camera_default_starts_at_y15_p30() {
        assert!(EDITOR_VISUAL3D_JS.contains("yawDegrees: 15,"));
        assert!(EDITOR_VISUAL3D_JS.contains("pitchDegrees: 30,"));
    }

    #[test]
    fn visual3d_preview_is_square_and_reserves_overlay_bars() {
        assert!(EDITOR_CSS.contains("--visual3d-preview-width: 320px;"));
        assert!(EDITOR_CSS.contains("--visual3d-preview-height: var(--visual3d-preview-width);"));
        assert!(EDITOR_CSS.contains(".visual3d-preview-wrap {\n  position: relative;"));
        assert!(EDITOR_CSS.contains("aspect-ratio: 1 / 1;"));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("const safeHeight = Math.max(1, height - safeTop - safeBottom);")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("originY: safeTop + safeHeight / 2 - ((minY + maxY) / 2) * scale,")
        );
        assert!(EDITOR_HTML.contains(
            r#"id="visual3dPreviewCanvas" class="visual3d-preview-canvas" width="320" height="320""#
        ));
        assert!(EDITOR_HTML.contains(r#"id="visual3dPreviewCanvas""#));
        assert_eq!(
            EDITOR_HTML
                .matches(r#"id="visualAnimationFrameStrip""#)
                .count(),
            1
        );
        assert!(!EDITOR_HTML.contains(r#"id="visual3dAnimationFrameStrip""#));
        assert!(EDITOR_VISUAL_JS.contains("function mountSharedVisualAnimationUi(dimension)"));
        assert!(EDITOR_VISUAL_JS.contains("previewColumn.insertBefore(toolbar, previewStage);"));
        assert!(EDITOR_VISUAL_JS.contains("previewStage.append(sidecar);"));
        assert!(
            EDITOR_VISUAL_JS
                .contains("function renderSharedVisualAnimationPlaybackView(context, frame)")
        );
        assert!(EDITOR_VISUAL_JS.contains("renderPlaybackFrame: is3d"));
        assert!(EDITOR_VISUAL3D_JS.contains("syncVisualAnimationPlayback();"));
        assert!(EDITOR_VISUAL_JS.contains(
            "function sharedVisualAnimationController(dimension = currentVisualPaneMode)"
        ));
        assert!(
            EDITOR_VISUAL_JS
                .contains("function insertSharedVisualAnimationFrameAt(dimension, index)")
        );
        assert!(
            EDITOR_VISUAL_JS
                .contains("function removeSharedVisualAnimationFrameAt(dimension, index)")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("return insertSharedVisualAnimationFrameAt(\"visual3d\", index);")
        );
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("return removeSharedVisualAnimationFrameAt(\"visual3d\", index);")
        );
        assert!(!EDITOR_VISUAL3D_JS.contains("frames.splice(insertIndex"));
        assert!(!EDITOR_VISUAL3D_JS.contains("frames.splice(removeIndex"));
        assert!(EDITOR_CSS.contains(
            ".visual3d-preview-column > .visual-animation-toolbar.is-visual3d-shared {\n  width: max-content;"
        ));
        assert!(EDITOR_CSS.contains(
            "@container (min-width: 704px) {\n  .visual3d-workspace {\n    grid-template-columns: var(--visual3d-slice-size) max-content;\n  }\n\n  .visual3d-builder.is-animation-mode .visual3d-slice-wrap {\n    padding-top: calc(var(--visual3d-overlay-control-height) + 10px);"
        ));
        assert!(EDITOR_VISUAL_JS.contains("function renderVisualAnimationFrameStripView(options)"));
        assert!(EDITOR_VISUAL3D_JS.contains("renderVisualAnimationFrameStripView({"));
        assert!(EDITOR_VISUAL3D_JS.contains(
            "renderCells: (index) => visual3dAnimationFramePreview(visual3d.frames[index])"
        ));
        assert!(
            EDITOR_VISUAL3D_JS
                .contains("renderVisual3dPreviewCanvas(canvas, frame, { overlays: false });")
        );
        assert!(EDITOR_CSS.contains(
            ".visual3d-builder.is-animation-mode .visual3d-preview-stage {\n  grid-template-columns: var(--visual3d-preview-width) 52px;"
        ));
        assert!(EDITOR_VISUAL3D_JS.contains("const VISUAL3D_PREVIEW_BASE_ZOOM = 1;"));
        assert!(EDITOR_VISUAL3D_JS.contains("const padding = 0;"));
        assert!(
            EDITOR_VISUAL3D_JS.contains("const overlaySafeInset = 8 + overlayControlHeight + 4;")
        );
    }

    #[test]
    fn level3d_palette_preview_ignores_camera_zoom_and_origin() {
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPalettePreviewCamera(source)"));
        assert!(EDITOR_LEVEL3D_JS.contains("zoom: 1,"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPalettePreviewOptions(camera)"));
        assert!(EDITOR_LEVEL3D_JS.contains("origin: { x: 0, y: 0, z: 0 },"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPaletteObjectDescriptor("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dPreviewVisuals("));
        assert!(EDITOR_LEVEL3D_JS.contains("function sourceLevel3dVisuals(source)"));
        assert!(EDITOR_LEVEL3D_JS.contains("...sourceLevel3dVisuals(source),"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "return level3dObjectHasPreviewVisual(object, exportData, visuals) ? object : null;"
        ));
        assert!(
            EDITOR_LEVEL3D_JS.contains("drawLevel3dCellsPreview(ctx, width, height, snapshot, [{")
        );
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("}], level3dPalettePreviewOptions(snapshot.render.camera));")
        );
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
