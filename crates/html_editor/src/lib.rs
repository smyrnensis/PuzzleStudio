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

#[cfg(feature = "embedded-assets")]
const EDITOR_HTML: &str = include_str!("../static/editor.html");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_MARKDOWN: &str = include_str!("../docs/editor.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_METADATA_MARKDOWN: &str = include_str!("../docs/metadata.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_PUZZLE_BLOCK_MARKDOWN: &str = include_str!("../docs/puzzle-block.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_LAYERS_MARKDOWN: &str = include_str!("../docs/layers.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_GROUPS_MARKDOWN: &str = include_str!("../docs/groups.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_TAGS_MARKDOWN: &str = include_str!("../docs/tags.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_LEGEND_MARKDOWN: &str = include_str!("../docs/legend.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_LEVELS_MARKDOWN: &str = include_str!("../docs/levels.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_LEVEL_LOCAL_LEGEND_MARKDOWN: &str = include_str!("../docs/level-local-legend.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_MESSAGES_MARKDOWN: &str = include_str!("../docs/messages.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_REWRITE_RULES_MARKDOWN: &str = include_str!("../docs/rewrite-rules.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_INPUT_RULES_MARKDOWN: &str = include_str!("../docs/input-rules.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_MOVEMENT_MARKDOWN: &str = include_str!("../docs/movement.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_GUARDS_MARKDOWN: &str = include_str!("../docs/guards.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_FIX_MARKDOWN: &str = include_str!("../docs/fix.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_VARIABLES_MARKDOWN: &str = include_str!("../docs/variables.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SCRATCH_MARKDOWN: &str = include_str!("../docs/scratch.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_CONDITIONS_MARKDOWN: &str = include_str!("../docs/conditions.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_WIN_CONDITIONS_MARKDOWN: &str = include_str!("../docs/win-conditions.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SCENES_MARKDOWN: &str = include_str!("../docs/scenes.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SCENE_LAYOUT_MARKDOWN: &str = include_str!("../docs/scene-layout.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SEMANTIC_INPUTS_MARKDOWN: &str = include_str!("../docs/semantic-inputs.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_MENUS_MARKDOWN: &str = include_str!("../docs/menus.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_LIFECYCLE_MARKDOWN: &str = include_str!("../docs/lifecycle.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SPRITES_MARKDOWN: &str = include_str!("../docs/sprites.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_DISPLAY_MARKDOWN: &str = include_str!("../docs/display.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_THEME_MARKDOWN: &str = include_str!("../docs/theme.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_SOUNDS_MARKDOWN: &str = include_str!("../docs/sounds.md");
#[cfg(feature = "embedded-assets")]
const EDITOR_CSS: &str = include_str!("../static/editor.css");
#[cfg(feature = "embedded-assets")]
const EDITOR_RUNTIME_JS: &str = include_str!("../static/editor_runtime.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_BOOT_JS: &str = include_str!("../static/editor_boot.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_DOM_JS: &str = include_str!("../static/editor_dom.js");
#[cfg(feature = "embedded-assets")]
const EDITOR_WORKSPACE_JS: &str = include_str!("../static/editor_workspace.js");
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
#[cfg(feature = "embedded-assets")]
const VISUALS_JS: &str = include_str!("../../html_play/static/visuals.js");
#[cfg(feature = "embedded-assets")]
const RENDERER_JS: &str = include_str!("../../html_play/static/renderer.js");
#[cfg(feature = "embedded-assets")]
const PAGES_EXAMPLE_PUZZLE_PATH: &str = "examples/puzzlestudio-example.puzzle";
#[cfg(feature = "embedded-assets")]
const PAGES_EXAMPLE_PUZZLE_SOURCE: &str = r#"title "PuzzleStudio Example"
subtitle "Push the box onto the goal"

puzzle sokoban {
layers {
Floor
Goal
Player Box Wall
}

win_conditions {
all Goal on Box
}

on_level_start {
[ no Floor ] -> [ Floor ]
}

rules {
input [ Player ] -> [ > Player ]
[ > Player | Box ] -> [ > Player | > Box ]
move
}
}

levels {
legend {
. = empty
# = Wall
P = Player
B = Box
G = Goal
* = Box Goal
}

level start
#######
#.....#
#.PB.G#
#.....#
#######
}
"#;
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
        EditorService::open(puzzle_path)?
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

        let puzzle_path = puzzle_path
            .map(|path| {
                puzzle_lang::resolve_game_entry(&path)
                    .map_err(|error| AppError::Config(error.to_string()))
            })
            .transpose()?;

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
        let source = PAGES_EXAMPLE_PUZZLE_SOURCE.to_string();
        Self {
            state: EditorState {
                puzzle_path: PAGES_EXAMPLE_PUZZLE_PATH.to_string(),
                workspace_root: String::new(),
                source: source.clone(),
                game_css: String::new(),
                #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
                base_game_visuals_js: String::new(),
                documents: vec![EditorDocument {
                    puzzle_path: PAGES_EXAMPLE_PUZZLE_PATH.to_string(),
                    encoding: "text".to_string(),
                    mime_type: mime_type(Path::new(PAGES_EXAMPLE_PUZZLE_PATH)).to_string(),
                    source,
                    data_url: String::new(),
                    preview_html: String::new(),
                    preview_error: String::new(),
                    game_css: String::new(),
                    imported_by: Vec::new(),
                    parent_game_path: PAGES_EXAMPLE_PUZZLE_PATH.to_string(),
                }],
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
        let base_game_visuals_js = load_base_game_visuals_js(&puzzle_path, &workspace_root)?;
        Ok(Self {
            state: EditorState {
                puzzle_path: puzzle_path.display().to_string(),
                workspace_root: workspace_root.display().to_string(),
                source,
                game_css: load_game_css(&puzzle_path, &workspace_root)?,
                #[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
                base_game_visuals_js,
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

    #[cfg(feature = "native-preview")]
    pub fn compile_preview(&self, request: &PreviewRequest) -> Result<String, AppError> {
        let workspace_root = PathBuf::from(&self.state.workspace_root);
        let preview_path = resolve_workspace_request_path(&request.puzzle_path, &workspace_root)?;
        let expanded_source =
            expand_preview_source_under_root(&request.source, &preview_path, &workspace_root)?;
        html_play::export_editor_preview_html_from_source(
            &expanded_source,
            &preview_path.display().to_string(),
            &request.game_css,
            &self.state.base_game_visuals_js,
        )
        .map_err(|report| {
            AppError::Diagnostics(puzzle_lang::resolve_diagnostic_source_locations(
                report,
                &request.source,
            ))
        })
    }

    pub fn highlight_json(&self, source: &str) -> String {
        Self::highlight_source_json(source)
    }

    pub fn highlight_source_json(source: &str) -> String {
        highlight_json(&puzzle_lang::highlight_source(source))
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
    documents: Vec<EditorDocument>,
}

#[derive(Debug)]
pub struct EditorDocument {
    puzzle_path: String,
    encoding: String,
    mime_type: String,
    source: String,
    data_url: String,
    preview_html: String,
    preview_error: String,
    game_css: String,
    imported_by: Vec<String>,
    parent_game_path: String,
}

struct WorkspacePuzzleDocument {
    path: PathBuf,
    source: String,
    has_game_prelude: bool,
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
) -> Result<String, AppError> {
    let mut scripts = vec![asset_resolver_js(puzzle_path, workspace_root)?];
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
fn asset_resolver_js(puzzle_path: &Path, workspace_root: &Path) -> Result<String, AppError> {
    let parent = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut files = String::new();
    files.push('{');
    let mut first = true;
    collect_asset_resolver_entries(parent, parent, workspace_root, &mut files, &mut first)?;
    files.push('}');
    Ok(format!(
        "window.PuzzleAssets = {{ files: {files}, url(path) {{ return this.files[String(path || '').replaceAll('\\\\\\\\', '/')] || String(path || ''); }} }};"
    ))
}

#[cfg(any(feature = "native-preview", feature = "embedded-assets"))]
fn collect_asset_resolver_entries(
    root: &Path,
    dir: &Path,
    workspace_root: &Path,
    files: &mut String,
    first: &mut bool,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if should_skip_workspace_dir(&path) {
                continue;
            }
            collect_asset_resolver_entries(root, &path, workspace_root, files, first)?;
            continue;
        }
        if !file_type.is_file()
            || !is_workspace_file(&path)
            || puzzle_lang::is_puzzle_source_path(&path)
        {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let Some(name) = relative.to_str() else {
            continue;
        };
        if !*first {
            files.push(',');
        }
        *first = false;
        push_json_string(files, &name.replace('\\', "/"));
        files.push(':');
        let url = if is_text_file(&path) {
            format!(
                "data:{};charset=utf-8,{}",
                mime_type(&path),
                percent_encode(&read_workspace_text_file(&path, workspace_root)?)
            )
        } else {
            format!(
                "data:{};base64,{}",
                mime_type(&path),
                base64_encode(&read_workspace_bytes(&path, workspace_root)?)
            )
        };
        push_json_string(files, &url);
    }
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
            let game_css = parent_game_path
                .map(|entry_path| load_game_css(entry_path, workspace_root))
                .transpose()?
                .unwrap_or_default();
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "text".to_string(),
                mime_type: mime_type(&path).to_string(),
                source,
                data_url: String::new(),
                preview_html: String::new(),
                preview_error: String::new(),
                game_css,
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
                source: read_workspace_text_file(&path, workspace_root)?,
                data_url: String::new(),
                preview_html: String::new(),
                preview_error: String::new(),
                game_css: String::new(),
                imported_by: Vec::new(),
                parent_game_path: String::new(),
            });
        } else {
            let bytes = read_workspace_bytes(&path, workspace_root)?;
            let mime_type = mime_type(&path);
            documents.push(EditorDocument {
                puzzle_path: path.display().to_string(),
                encoding: "data_url".to_string(),
                mime_type: mime_type.to_string(),
                source: String::new(),
                data_url: format!("data:{mime_type};base64,{}", base64_encode(&bytes)),
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
            has_game_prelude: puzzle_lang::source_has_game_prelude(&source),
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
        .filter(|document| document.has_game_prelude)
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

#[cfg(feature = "embedded-assets")]
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
        ("GET", "/editor_runtime.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_RUNTIME_JS)
        }
        ("GET", "/editor_dom.js") => http_ok("text/javascript; charset=utf-8", EDITOR_DOM_JS),
        ("GET", "/editor_workspace.js") => {
            http_ok("text/javascript; charset=utf-8", &editor_workspace_js())
        }
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
        ("GET", "/editor_sprite.js") => http_ok("text/javascript; charset=utf-8", EDITOR_SPRITE_JS),
        ("GET", "/puzzle3_visual_core.js") => {
            http_ok("text/javascript; charset=utf-8", PUZZLE3_VISUAL_CORE_JS)
        }
        ("GET", "/editor_sprite3d.js") => {
            http_ok("text/javascript; charset=utf-8", EDITOR_SPRITE3D_JS)
        }
        ("GET", "/editor_sounds.js") => http_ok("text/javascript; charset=utf-8", EDITOR_SOUNDS_JS),
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
            http_ok(
                "application/json; charset=utf-8",
                &service.highlight_json(&source),
            )
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
}

impl SaveRequest {
    pub fn new(source: impl Into<String>, puzzle_path: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            puzzle_path: puzzle_path.into(),
        }
    }

    pub fn from_body(body: &str, state: &EditorState) -> Self {
        if body.trim_start().starts_with('{') {
            return Self {
                source: json_string_field(body, "source").unwrap_or_default(),
                puzzle_path: json_string_field(body, "puzzlePath")
                    .unwrap_or_else(|| state.puzzle_path.clone()),
            };
        }
        Self {
            source: body.to_string(),
            puzzle_path: state.puzzle_path.clone(),
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
    write_text_asset(output_dir, "editor_boot.js", EDITOR_BOOT_JS)?;
    write_text_asset(output_dir, "editor_dom.js", EDITOR_DOM_JS)?;
    write_text_asset(output_dir, "editor_workspace.js", &editor_workspace_js())?;
    write_text_asset(output_dir, "editor_source.js", EDITOR_SOURCE_JS)?;
    write_text_asset(output_dir, "editor_level3d.js", EDITOR_LEVEL3D_JS)?;
    write_text_asset(output_dir, "editor_workbench.js", EDITOR_WORKBENCH_JS)?;
    write_text_asset(
        output_dir,
        "editor_import_export.js",
        EDITOR_IMPORT_EXPORT_JS,
    )?;
    write_text_asset(output_dir, "editor.js", EDITOR_JS)?;
    write_text_asset(output_dir, "editor_sprite.js", EDITOR_SPRITE_JS)?;
    write_text_asset(output_dir, "editor_sprite3d.js", EDITOR_SPRITE3D_JS)?;
    write_text_asset(output_dir, "editor_sounds.js", EDITOR_SOUNDS_JS)?;
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
    EDITOR_HTML.replace("<!-- PUZZLESTUDIO_EDITOR_DOCS -->", &render_editor_docs())
}

#[cfg(feature = "embedded-assets")]
fn editor_workspace_js() -> String {
    const PLACEHOLDER: &str = "\"__PUZZLESTUDIO_NEW_PUZZLE_SOURCE__\"";
    if !EDITOR_WORKSPACE_JS.contains(PLACEHOLDER) {
        panic!("editor workspace JS must contain the new puzzle template placeholder");
    }
    EDITOR_WORKSPACE_JS.replace(
        PLACEHOLDER,
        &js_string_literal(puzzle_authoring::NEW_PUZZLE_TEMPLATE),
    )
}

#[cfg(feature = "embedded-assets")]
struct EditorDocsPage {
    id: &'static str,
    title: &'static str,
    markdown: &'static str,
}

#[cfg(feature = "embedded-assets")]
const EDITOR_DOCS_PAGES: &[EditorDocsPage] = &[
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
        id: "layers",
        title: "Layers",
        markdown: EDITOR_DOCS_LAYERS_MARKDOWN,
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
        id: "level-local-legend",
        title: "Level Legend",
        markdown: EDITOR_DOCS_LEVEL_LOCAL_LEGEND_MARKDOWN,
    },
    EditorDocsPage {
        id: "messages",
        title: "Messages",
        markdown: EDITOR_DOCS_MESSAGES_MARKDOWN,
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
        id: "variables",
        title: "Variables",
        markdown: EDITOR_DOCS_VARIABLES_MARKDOWN,
    },
    EditorDocsPage {
        id: "scratch",
        title: "Scratch",
        markdown: EDITOR_DOCS_SCRATCH_MARKDOWN,
    },
    EditorDocsPage {
        id: "conditions",
        title: "Conditions",
        markdown: EDITOR_DOCS_CONDITIONS_MARKDOWN,
    },
    EditorDocsPage {
        id: "win-conditions",
        title: "Win Conditions",
        markdown: EDITOR_DOCS_WIN_CONDITIONS_MARKDOWN,
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
        id: "lifecycle",
        title: "Lifecycle",
        markdown: EDITOR_DOCS_LIFECYCLE_MARKDOWN,
    },
    EditorDocsPage {
        id: "sprites",
        title: "Sprites",
        markdown: EDITOR_DOCS_SPRITES_MARKDOWN,
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
        id: "sounds",
        title: "Sounds",
        markdown: EDITOR_DOCS_SOUNDS_MARKDOWN,
    },
];

#[cfg(feature = "embedded-assets")]
fn render_editor_docs() -> String {
    let mut out = String::from(
        "<div class=\"docs-layout\">\n<nav class=\"docs-nav\" role=\"tablist\" aria-label=\"Documents\">\n",
    );
    for (index, page) in EDITOR_DOCS_PAGES.iter().enumerate() {
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
        out.push_str(&render_editor_docs_markdown(page, index == 0));
    }
    out.push_str("</div>\n</div>");
    out
}

#[cfg(feature = "embedded-assets")]
fn render_editor_docs_markdown(page: &EditorDocsPage, active: bool) -> String {
    let hidden = if active { "" } else { " hidden" };
    let mut out = format!(
        "<article class=\"docs-article\" data-docs-article=\"{}\"{hidden}>\n",
        escape_html(page.id)
    );
    let mut paragraph = Vec::new();
    let mut in_header = false;
    let mut header_closed = false;
    let mut in_section = false;
    let mut in_code = false;

    for line in page.markdown.lines() {
        if in_code {
            if line.trim_start().starts_with("```") {
                out.push_str("</code></pre>\n");
                in_code = false;
            } else {
                out.push_str(&escape_html(line));
                out.push('\n');
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
            out.push_str("<pre><code>");
            in_code = true;
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

    if in_code {
        out.push_str("</code></pre>\n");
    }
    flush_docs_paragraph(&mut out, &mut paragraph);
    close_docs_header(&mut out, &mut in_header, &mut header_closed);
    close_docs_section(&mut out, &mut in_section);
    out.push_str("</article>");
    out
}

#[cfg(feature = "embedded-assets")]
fn flush_docs_paragraph(out: &mut String, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    out.push_str("<p>");
    out.push_str(&render_docs_inline(&paragraph.join(" ")));
    out.push_str("</p>\n");
    paragraph.clear();
}

#[cfg(feature = "embedded-assets")]
fn close_docs_header(out: &mut String, in_header: &mut bool, header_closed: &mut bool) {
    if *in_header && !*header_closed {
        out.push_str("</header>\n");
        *header_closed = true;
        *in_header = false;
    }
}

#[cfg(feature = "embedded-assets")]
fn close_docs_section(out: &mut String, in_section: &mut bool) {
    if *in_section {
        out.push_str("</section>\n");
        *in_section = false;
    }
}

#[cfg(feature = "embedded-assets")]
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
                "generateSoundEffect",
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
    let source = if state.puzzle_path.trim().is_empty() {
        state.source.clone()
    } else {
        fs::read_to_string(&state.puzzle_path)?
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
    push_editor_documents_json(&mut out, state);
    out.push('}');
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
    out.push_str("\"activeDocumentIndex\":0");
    out.push(',');
    push_editor_documents_json(out, state);
    out.push('}');
}

fn push_editor_documents_json(out: &mut String, state: &EditorState) {
    out.push_str("\"documents\":[");
    let mut first = true;
    for document in &state.documents {
        if !first {
            out.push(',');
        }
        first = false;
        out.push('{');
        push_json_pair(out, "puzzlePath", &document.puzzle_path);
        out.push(',');
        push_json_pair(out, "workspaceRoot", &state.workspace_root);
        out.push(',');
        push_json_pair(out, "encoding", &document.encoding);
        out.push(',');
        push_json_pair(out, "mimeType", &document.mime_type);
        out.push(',');
        push_json_pair(out, "source", &document.source);
        out.push(',');
        push_json_pair(out, "dataUrl", &document.data_url);
        out.push(',');
        push_json_pair(out, "previewHtml", &document.preview_html);
        out.push(',');
        push_json_pair(out, "previewError", &document.preview_error);
        out.push(',');
        push_json_pair(out, "gameCss", &document.game_css);
        out.push(',');
        push_json_string_array(out, "importedBy", &document.imported_by);
        out.push(',');
        push_json_pair(out, "parentGamePath", &document.parent_game_path);
        out.push('}');
    }
    out.push(']');
}

fn highlight_json(highlighted: &puzzle_lang::HighlightedSource) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"parsed\":");
    out.push_str(if highlighted.parsed { "true" } else { "false" });
    out.push(',');
    push_json_pair(&mut out, "html", &highlighted.html);
    out.push('}');
    out
}

fn push_json_pair(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
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
fn js_string_literal(value: &str) -> String {
    let mut out = String::new();
    push_json_string(&mut out, value);
    out
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

    fn editor_fixture_source(title: &str) -> String {
        format!(
            r#"title "{title}"

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

level start {{
P
}}
}}

scene playing {{
state {{
board = puzzle default
}}
layout {{
puzzle board
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
        assert_eq!(state.documents.len(), 1);
        assert_eq!(state.documents[0].puzzle_path, PAGES_EXAMPLE_PUZZLE_PATH);
        assert!(state.source.contains("PuzzleStudio Example"));

        let html = service
            .export_pages_editor_html()
            .expect("export managed Pages editor html");
        assert!(html.contains("PuzzleStudio Example"));
        assert!(html.contains("Push the box onto the goal"));
        assert!(html.contains("all Goal on Box"));
        assert!(html.contains("input [ Player ]"));
        assert!(!html.contains("Player{&gt;}"));
        assert!(!html.contains("Player{>}"));
        assert!(html.contains(PAGES_EXAMPLE_PUZZLE_PATH));
        assert!(!html.contains("games/spec_2d.puzzle"));
        assert!(!html.contains("/games/"));
        assert!(!html.contains("Managed GitHub Pages starter"));
        assert!(!html.contains("levels demo of starter"));
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

        assert!(html.contains("PuzzleStudio Example"));
        assert!(html.contains("Push the box onto the goal"));
    }

    #[test]
    fn open_loads_workspace_documents_with_active_puzzle_first() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Editor Fixture"),
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
            "preview asset resolver should expose sibling workspace assets"
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
        let game_path = workspace.write("games/broken_visuals/game.puzzle", "title \"Broken\"\n");

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
                    "title \"Broken\"\n",
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
            include_str!("../../../games/spec_3d.puzzle3"),
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
        assert!(document.source.contains("puzzle3 sokoban"));
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
    fn source_json_reads_the_current_file_from_disk() {
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

        assert!(source.contains("title \"Changed Title\""));
        assert!(!source.contains("title \"Original Title\""));
    }

    #[test]
    fn source_json_reports_read_failure_instead_of_cached_source() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/editor_fixture/game.puzzle",
            editor_fixture_source("Original Title"),
        );
        let service = EditorService::open(&game_path).expect("open editor fixture");

        fs::remove_file(&game_path).expect("remove source after service open");

        assert!(
            service.source_json().is_err(),
            "source json must not fall back to the cached source when the file cannot be read"
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
    fn workspace_preview_generation_is_deferred_until_run() {
        let workspace = TestWorkspace::new();
        let game_path = workspace.write(
            "games/broken/game.puzzle",
            "title \"Broken\"\n\npuzzle main {\n",
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

        let service = EditorService::open_game_entry(&project_dir).expect("open empty project");
        let state = service.state();

        assert_eq!(
            PathBuf::from(&state.workspace_root),
            project_dir.canonicalize().expect("canonical project dir")
        );
        assert_eq!(state.puzzle_path, "");
        assert_eq!(state.source, "");
        assert!(state.documents.is_empty());
    }

    #[test]
    fn open_game_entry_accepts_project_folders_without_game_prelude() {
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
        assert!(!html.contains("Preview Before"));
    }

    #[test]
    fn compile_preview_accepts_top_level_sounds_without_duplicate_error() {
        let workspace = TestWorkspace::new();
        let source = editor_fixture_source("Top Level Sounds").replace(
            "puzzle default {",
            "sounds {\nsfx push seed=301193 type=hit volume=0.3\n}\n\npuzzle default {",
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
            "theme puzzlescript {\nbackground_color #ffffff\ntext_color #000000\n}\n\npuzzle default {",
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
title display_object_single_color_preview

puzzle default {
layers {
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
level start
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
title line_style_tagged_preview

puzzle default {
tags {
state = base movable
}
layers {
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
level start
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
title "Multi Error Probe"

puzzle main {
layers {
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
level first
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
title "Multi Lifecycle Error Probe"

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
level first
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
title "Multi Statement Parse Error Probe"

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
level first
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
title "Sibling Statement Block Error Probe"

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
level first
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
        let source = include_str!("../../../games/spec_3d.puzzle3");
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
title "Bare 3D Input"

puzzle3 push3 {
  layers {
    actor = Player
  }

  rules {
    input [ Player ] -> [ > Player ]
  }
}

levels3 demo of push3 {
  legend {
    . = empty
    P = Player
  }

  level start {
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
    fn source_run_button_opens_play_preview() {
        assert!(EDITOR_HTML.contains(r#"id="runButton""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Run preview""#));
        assert!(EDITOR_HTML.contains("lucide-play"));
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
        assert!(
            EDITOR_JS.contains("runButton.addEventListener(\"click\", runPreviewFromSourcePane);")
        );
        assert!(EDITOR_WORKSPACE_JS.contains("runButton.title = \"Run preview\";"));
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
        assert!(EDITOR_WORKSPACE_JS.contains("function confirmDesktopDeleteWorkspaceEntry(node)"));
        let confirm = EDITOR_WORKSPACE_JS
            .find("confirmDesktopDeleteWorkspaceEntry(target.node)")
            .expect("desktop delete confirms the selected workspace entry");
        let host_delete = EDITOR_WORKSPACE_JS
            .find("window.PuzzleStudioHost.deleteWorkspaceEntry({")
            .expect("desktop delete calls the host filesystem boundary");
        assert!(confirm < host_delete);
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
    fn puzzlescript_import_requires_explicit_convert_action() {
        assert!(EDITOR_HTML.contains(r#"id="psImportConvertButton""#));
        assert!(EDITOR_HTML.contains("lucide-file-plus-icon lucide-file-plus"));
        assert!(EDITOR_IMPORT_EXPORT_JS.contains("function resetPuzzleScriptImportConversion()"));
        assert!(EDITOR_JS.contains(
            "psImportSourceInput?.addEventListener(\"input\", resetPuzzleScriptImportConversion);"
        ));
        assert!(EDITOR_JS.contains("psImportConvertButton?.addEventListener(\"click\", () => {"));
        assert!(!EDITOR_JS.contains("schedulePuzzleScriptImportConversion"));
        assert!(!EDITOR_JS.contains("await convertPuzzleScriptImport()"));
        assert!(
            EDITOR_CSS.contains(".ps-import-actions .source-action-button:hover:not(:disabled)")
        );
    }

    #[test]
    fn level_editor_grid_is_owned_by_editor_toggle() {
        assert!(EDITOR_HTML.contains(r#"id="levelGridButton""#));
        assert!(
            EDITOR_DOM_JS
                .contains("const levelGridButton = document.querySelector(\"#levelGridButton\");")
        );
        assert!(EDITOR_JS.contains("let levelGridVisible = false;"));
        assert!(EDITOR_HTML.contains(r#"id="levelLayerVisibilityButton""#));
        assert!(EDITOR_HTML.contains("lucide-list-filter"));
        assert!(
            EDITOR_DOM_JS.contains(
                "const levelLayerVisibilityButton = document.querySelector(\"#levelLayerVisibilityButton\");"
            )
        );
        assert!(EDITOR_JS.contains("hiddenLayers: []"));
        assert!(!EDITOR_HTML.contains(r#"id="levelScopeLayerButton""#));
        assert!(!EDITOR_HTML.contains(r#"aria-label="Level edit scope""#));
        assert!(EDITOR_JS.contains("function levelVisibleCells("));
        assert!(EDITOR_JS.contains("function levelVisibleLayerIndexes("));
        assert!(EDITOR_JS.contains("function sameCellSlotsForVisibleLayers("));
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
        assert!(EDITOR_JS.contains(
            "levelRenderer.render(levelScene(visibleCells, exportData));\n    syncLevelGridVisibility();"
        ));
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
        assert!(EDITOR_JS.contains("function sourceLayerNameEntries("));
        assert!(EDITOR_JS.contains("label: layerNames.get(layerIndex) || \"\""));
        assert!(EDITOR_CSS.contains(".level-board.board.has-all-cell-grid .cell::after"));
        assert!(EDITOR_CSS.contains("z-index: 100;"));
    }

    #[test]
    fn level_editor_controls_are_ordered_before_palette_and_preview() {
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
        assert!(EDITOR_JS.contains(r#"postMessage({ type: "PuzzleStudioCommand", command }"#));
        assert!(EDITOR_JS.contains(r#"type: "PuzzleStudioKey","#));
        assert!(EDITOR_JS.contains("code: event.code"));
        assert!(EDITOR_JS.contains("acceptModelInput: true"));
        assert!(
            EDITOR_JS.contains("levelDisplayCells = stateDataToLevelCells(stateData, exportData);")
        );
        assert!(EDITOR_JS.contains(
            "return levelPlaytestActive && levelDisplayCells?.length === level.cells.length ? levelDisplayCells : level.cells;"
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
        assert!(EDITOR_JS.contains(
            "return !compiledPreviewStale && exportData?.levels?.length ? exportData : null;"
        ));
        assert!(
            EDITOR_JS.contains(
                "if (!compiledPreviewStale && exportData === currentPreviewExportData()) {"
            )
        );
        assert!(EDITOR_JS.contains("function levelEditorCurrentExportData()"));
        assert!(EDITOR_JS.contains("async function loadLevelSourceEntryAfterPreviewCompile("));
        assert!(EDITOR_JS.contains("function loadLevelSourceEntryWithExportData("));
        assert!(EDITOR_JS.contains("function reportLevelSourceLoadFailure("));
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
        assert!(EDITOR_JS.contains("sourceLevelRowGroups(parsed.rows)"));
        assert!(EDITOR_JS.contains(
            "if (!loadLevelFromSourceEntry(source, entry, { ...options, exportData, levelIndex, levelName }))"
        ));
        assert!(!EDITOR_JS.contains(
            "if (!loadLevelFromSourceEntry(source, sourceEntry, { levelIndex, levelName }))"
        ));
        assert!(EDITOR_JS.contains(
            "function levelSourceData(source = levelReferenceSource(currentLevelExportData()), exportData = currentLevelExportData())"
        ));
    }

    #[test]
    fn level_source_previews_do_not_indent_map_rows() {
        assert!(EDITOR_JS.contains(
            "levelDefinitionSource(levelName, levelSourceData(), \"\", { leadingBlank: false, bodyIndent: \"\" })"
        ));
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
    fn level_editor_allows_unnamed_2d_levels() {
        assert!(EDITOR_HTML.contains(r#"id="levelNameInput" type="text" value="""#));
        assert!(
            EDITOR_JS.contains(
                "const name = \"\";\n  const sourceData = defaultEmptyLevel2dSourceData();"
            )
        );
        assert!(EDITOR_JS.contains("return cleaned;\n}"));
        assert!(
            EDITOR_JS
                .contains("levelName ? `${levelIndent}level ${levelName} {` : `${levelIndent}{`")
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
    fn editor_source_line_numbers_are_loaded_by_static_editor() {
        assert!(EDITOR_HTML.contains(r#"id="sourceLineNumbers""#));
        assert!(EDITOR_DOM_JS.contains(
            r##"const sourceLineNumbers = document.querySelector("#sourceLineNumbers");"##
        ));
        assert!(EDITOR_CSS.contains(".source-line-numbers"));
        assert!(EDITOR_SOURCE_JS.contains(
            r#"const visibleLines = visibleSource.length ? visibleSource.split("\n") : [""];"#
        ));
        assert!(EDITOR_SOURCE_JS.contains("sourceFoldableBlocks(source)"));
        assert!(EDITOR_SOURCE_JS.contains("sourceFoldMarkerHighlightRuns(runs, range)"));
        assert!(EDITOR_SOURCE_JS.contains("range.closeOffset, range.closeOffset + 1"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceFoldStateForSource("));
        assert!(EDITOR_WORKSPACE_JS.contains("sourceFoldedBlockKeys"));
        assert!(
            EDITOR_WORKSPACE_JS.contains("restoreSourceFoldState(document.sourceFoldedBlockKeys)")
        );
        assert!(EDITOR_JS.contains("renderSourceLineNumbers();"));
    }

    #[test]
    fn solver_pane_has_level_selector() {
        assert!(EDITOR_HTML.contains(r#"id="solverLevelSelect""#));
        assert!(EDITOR_DOM_JS.contains("const solverLevelSelect = document.querySelector"));
        assert!(EDITOR_JS.contains("let solverLevelIndex = 0;"));
        assert!(EDITOR_JS.contains("function syncSolverLevelSelector("));
        assert!(EDITOR_JS.contains("function selectSolverLevel("));
        assert!(EDITOR_JS.contains("function setSolverTargetFromState("));
        assert!(EDITOR_JS.contains("function syncSolverTargetFromActiveLevel("));
        assert!(EDITOR_JS.contains("function syncSolverTargetFromSourceTarget("));
        assert!(EDITOR_JS.contains("async function syncSolverTargetFromSourceCursor("));
        assert!(EDITOR_JS.contains("function openSolverPaneForCurrentLevel("));
        assert!(
            EDITOR_JS
                .contains("const target = await resolveSourceTargetFromWasm(source, position);")
        );
        assert!(EDITOR_JS.contains("return syncSolverTargetFromSourceTarget(target, exportData);"));
        assert!(
            EDITOR_JS.contains(
                "solverModeButton.addEventListener(\"click\", () => {\n  openSolverPaneForCurrentLevel().catch((error) => {"
            )
        );
        assert!(EDITOR_JS.contains("compilingMessage: \"Compiling preview for solve\""));
        assert!(EDITOR_JS.contains("async function solveEditedLevelFromEditor()"));
        assert!(EDITOR_JS.contains("function compiledLevelStateData("));
        assert!(EDITOR_JS.contains("function solverPuzzle3dPreviewSnapshot("));
        assert!(!EDITOR_JS.contains("function solveLevelInMainThread("));
        assert!(!EDITOR_JS.contains("backend: \"wasm-main\""));
        assert!(!EDITOR_JS.contains("Solving in this browser tab"));
        assert!(EDITOR_JS.contains("Solver worker failed:"));
        assert!(EDITOR_JS.contains("return currentPreviewMode === \"edit\";"));
        assert!(!EDITOR_JS.contains("requestFocusedPreviewState();"));
        assert!(
            !EDITOR_JS.contains("syncPreviewStateFromLevel();\n  try {\n    worker.postMessage")
        );
        assert!(EDITOR_JS.contains("solverLevelSelect?.addEventListener(\"change\""));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dEditedSnapshotAppliesToLevel("));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dCellsWithObjectDescriptors("));
        assert!(
            EDITOR_JS
                .contains("isPuzzle3dExport(exportData) && typeof renderPuzzle3dSolverPreview")
        );
    }

    #[test]
    fn workbench_panes_can_be_maximized_without_replacing_normal_layout() {
        assert!(EDITOR_HTML.contains(r#"data-pane-maximize="source""#));
        assert!(EDITOR_HTML.contains(r#"data-pane-maximize="active-preview""#));
        assert!(EDITOR_WORKBENCH_JS.contains("let maximizedWorkPaneId = \"\";"));
        assert!(EDITOR_WORKBENCH_JS.contains("function toggleWorkPaneMaximized(paneId)"));
        assert!(EDITOR_WORKBENCH_JS.contains("function isPaneDisplayed(paneId)"));
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
    fn closing_preview_pane_terminates_preview_game() {
        assert!(EDITOR_JS.contains("function terminatePreviewGame()"));
        assert!(EDITOR_JS.contains("setPreviewFrameHtml(emptyPreviewDocument());"));
        assert!(
            EDITOR_WORKBENCH_JS.contains(
                "if (normalized === PREVIEW_WORK_PANE_ID && typeof terminatePreviewGame === \"function\")"
            )
        );
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
        assert!(EDITOR_SOURCE_JS.contains("sourceEditorContentChanged();\n  scheduleSourceCompletion();\n  syncPreviewModeFromSourceCursor();"));
        assert!(
            EDITOR_SOURCE_JS.contains("const predicted = sourcePredictedBeforeInputValue(event);")
        );
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
        assert!(EDITOR_JS.contains("setPreviewFrameHtml(editorPreviewDocument(latestHtml));"));
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
        assert!(EDITOR_JS.contains("const globalName = exportData?.__kind === \"puzzle3d\" ? \"Puzzle3DFrameFixture\" : \"PuzzleExport\";"));
        assert!(EDITOR_LEVEL3D_JS.contains("function level3dRuntimePreviewUpdate()"));
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
        assert!(EDITOR_LEVEL3D_JS.contains("sendLevel3dLayerSnapshotToRuntime();"));
        assert!(EDITOR_LEVEL3D_JS.contains(
            "renderLevel3dLayerBoard();\n  renderLevel3dStageOverlay();\n  refreshLevel3dRuntimePreviews();\n  return true;"
        ));
        assert!(EDITOR_JS.contains("currentPreviewMode === \"level3d\" && typeof sendLevel3dSnapshotToRuntime === \"function\""));
        assert!(
            EDITOR_JS
                .contains("const stateData = options.stateData || solverStateData(exportData);")
        );
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
                "\"camera\": { \"yawDegrees\": 10, \"pitchDegrees\": 55, \"zoom\": 1.1 }"
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
        assert!(
            EDITOR_JS
                .contains("sourceEditor.addEventListener(\"click\", loadLevelFromSourceClick);")
        );
        assert!(EDITOR_LEVEL3D_JS.contains("registerSourceEditableTarget?.(\"level3d\""));
        assert!(EDITOR_SOURCE_JS.contains("syncPreviewModeFromSourceCursor({ force: true });"));
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
            .find("function syncPreviewModeFromSourcePointer")
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
    fn sprite_source_click_uses_rust_target_sync() {
        let click_start = EDITOR_SPRITE_JS
            .find("function loadSpriteFromSourceClick(event = null)")
            .expect("sprite source click handler");
        let click_end = EDITOR_SPRITE_JS[click_start..]
            .find("function loadSpriteSourceTarget")
            .map(|index| click_start + index)
            .expect("sprite rust target loader");
        let click = &EDITOR_SPRITE_JS[click_start..click_end];

        assert!(click.contains("allowInactiveMode: true,"));
        assert!(click.contains("position: clickOffset ?? ("));
        assert!(
            click.contains(
                "sourceViewOffsetToDocumentOffset(sourceEditor.selectionStart, \"start\")"
            )
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("sourceEditor.addEventListener(\"click\", loadSpriteFromSourceClick);")
        );
    }

    #[test]
    fn sprite_header_duplicates_selected_sprite_below_source_range() {
        assert!(EDITOR_HTML.contains(r#"id="duplicateSpriteButton""#));
        assert!(EDITOR_HTML.contains(r#"aria-label="Duplicate sprite""#));
        assert!(!EDITOR_HTML.contains("newSpriteButton"));
        assert!(EDITOR_DOM_JS.contains(
            "const duplicateSpriteButton = document.querySelector(\"#duplicateSpriteButton\");"
        ));
        assert!(!EDITOR_DOM_JS.contains("newSpriteButton"));
        assert!(!EDITOR_JS.contains("addEmptySprite2dToFocusedSource"));
        assert!(!EDITOR_JS.contains("newSpriteButton"));
        assert!(EDITOR_SPRITE_JS.contains("function duplicateSpriteInSource()"));
        assert!(EDITOR_SPRITE_JS.contains(
            "duplicateSpriteButton?.addEventListener(\"click\", duplicateSpriteInSource);"
        ));
        assert!(
            EDITOR_SPRITE_JS
                .contains("const name = uniqueSpriteDuplicateName(source, originalName);")
        );
        assert!(EDITOR_SPRITE_JS.contains("spriteSourceInsertionLineEnd(source, entry.end),"));
        assert!(EDITOR_SPRITE_JS.contains(
            "setSpriteEditSource({ start: result.start, end: result.end, name: result.name }, document);"
        ));
        assert!(!EDITOR_SPRITE_JS.contains("function addEmptySpriteToSource"));
        assert!(!EDITOR_SPRITE_JS.contains("function insertEmptySpriteDefinition"));
    }

    #[test]
    fn sprite3d_source_focus_scans_all_sprites3_blocks() {
        assert!(EDITOR_SPRITE3D_JS.contains("function findSprites3dBlocks(source)"));
        assert!(EDITOR_SPRITE3D_JS.contains("while ((match = pattern.exec(source)))"));
        assert!(EDITOR_SPRITE3D_JS.contains("pattern.lastIndex = closeIndex + 1;"));
        assert!(EDITOR_SPRITE3D_JS.contains("function findSprite3dDefinitionByName(source, name)"));
        assert!(EDITOR_SPRITE3D_JS.contains("for (const block of findSprites3dBlocks(source))"));
        assert!(EDITOR_JS.contains("const blocks = typeof findSprites3dBlocks === \"function\""));
        assert!(EDITOR_JS.contains("findSprite3dDefinitionByName(source, name)"));
        assert!(!EDITOR_SPRITE3D_JS.contains("typeof spriteSourceCursorPosition"));
        assert!(!EDITOR_SPRITE3D_JS.contains("typeof spriteSourceTargetAtCursor"));
        assert!(!EDITOR_SPRITE3D_JS.contains(": source.length"));
    }

    #[test]
    fn sprite_color_edit_undo_batches_until_commit() {
        assert!(EDITOR_SPRITE_JS.contains("function beginSpriteColorEditHistory(kind)"));
        assert!(EDITOR_SPRITE_JS.contains("function commitSpriteColorEditHistory(kind)"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("updateSelectedSpriteColor(colorInput.value, { deferHistory: true })")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("updateSelectedSpriteColor(colorInput.value, { commitHistory: true })")
        );
        assert!(EDITOR_SPRITE_JS.contains("previewNewSpriteColor(color, { deferHistory: true })"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("updateSelectedSprite3dColor(colorInput.value, { deferHistory: true })")
        );
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("updateSelectedSprite3dColor(colorInput.value, { commitHistory: true })")
        );
        assert!(
            EDITOR_SPRITE3D_JS.contains("function previewNewSprite3dColor(color, options = {})")
        );
        assert!(EDITOR_SPRITE3D_JS.contains("onChange: previewNewSprite3dColor"));
        assert!(EDITOR_JS.contains("commitSpriteColorEditHistory(kind);"));
    }

    #[test]
    fn sprite_hue_edit_colorizes_neutral_starting_colors() {
        assert!(EDITOR_SPRITE_JS.contains("const makeHueEditVisible = () => {"));
        assert!(EDITOR_SPRITE_JS.contains("if (hsv.s <= 0)"));
        assert!(EDITOR_SPRITE_JS.contains("if (hsv.v <= 0)"));
        assert!(EDITOR_SPRITE_JS.contains("hueInput.addEventListener(\"pointerdown\", () => {"));
        assert!(EDITOR_SPRITE_JS.contains("window.requestAnimationFrame(activateHueInput);"));
        assert!(EDITOR_SPRITE_JS.contains("makeHueEditVisible();\n    emit();"));
    }

    #[test]
    fn sprite_color_adjuster_preserves_hue_for_self_emitted_color_echoes() {
        let adjuster_start = EDITOR_SPRITE_JS
            .find("function renderSpriteColorAdjuster")
            .expect("sprite color adjuster");
        let adjuster_end = EDITOR_SPRITE_JS[adjuster_start..]
            .find("function spriteEyedropperIconSvg")
            .map(|index| adjuster_start + index)
            .expect("sprite eyedropper icon after adjuster");
        let adjuster = &EDITOR_SPRITE_JS[adjuster_start..adjuster_end];

        assert!(adjuster.contains("let selfEmittedColor = \"\";"));
        assert!(adjuster.contains("if (normalized !== selfEmittedColor) {"));
        assert!(adjuster.contains("selfEmittedColor = \"\";"));
        assert!(
            adjuster.contains("selfEmittedColor = next;\n    syncUi(next);\n    onChange(next);")
        );
    }

    #[test]
    fn sprite_color_editor_eyedropper_uses_browser_eyedropper_only() {
        assert!(EDITOR_BOOT_JS.contains("async pickScreenColor()"));
        assert!(EDITOR_BOOT_JS.contains("function screenColorPickerAvailable()"));
        assert!(EDITOR_BOOT_JS.contains("canPickScreenColor()"));
        assert!(EDITOR_BOOT_JS.contains(r#"typeof window.EyeDropper === "function""#));
        assert!(EDITOR_BOOT_JS.contains("new window.EyeDropper().open()"));
        assert!(!EDITOR_BOOT_JS.contains(r#"invoke("pick_screen_color")"#));
        assert!(EDITOR_SPRITE_JS.contains("window.PuzzleStudioHost?.pickScreenColor?.()"));
        assert!(EDITOR_SPRITE_JS.contains("window.PuzzleStudioHost?.canPickScreenColor?.()"));
        assert!(EDITOR_SPRITE_JS.contains("eyedropperButton.disabled = !canPickScreenColor;"));
        assert!(EDITOR_SPRITE_JS.contains("function spriteEyedropperIconSvg()"));
        assert!(EDITOR_SPRITE_JS.contains(r#"<path d="m2 22 1-1h3l9-9"></path>"#));
        assert!(EDITOR_SPRITE_JS.contains(r#"<path d="M3 21v-3l9-9"></path>"#));
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
    fn sprite_brush_presets_are_relative_and_paint_updates_changed_cells() {
        assert!(EDITOR_CSS.contains(
            "#spriteBuilder .sprite-board {\n  --sprite-cell: clamp(8px, calc((100cqw - 2px) / var(--sprite-size)), 64px);\n}"
        ));
        assert!(EDITOR_SPRITE_JS.contains("let spriteBrushPreset = \"pixel\";"));
        assert!(EDITOR_SPRITE_JS.contains("pixel: { label: \"1px\", diameterCells: 1 },"));
        assert!(EDITOR_SPRITE_JS.contains("thin: { label: \"Marker S\", ratio: 1 / 32 },"));
        assert!(EDITOR_SPRITE_JS.contains("medium: { label: \"Marker M\", ratio: 1 / 20 },"));
        assert!(EDITOR_SPRITE_JS.contains("thick: { label: \"Marker L\", ratio: 1 / 12 },"));
        assert!(EDITOR_SPRITE_JS.contains(
            "if (spriteBrushIsPixel()) {\n    const index = spriteCellIndexFromPoint(point);\n    return index >= 0 ? [index] : [];\n  }"
        ));
        assert!(
            EDITOR_SPRITE_JS.contains("return Math.min(sprite.size, sprite.size * config.ratio);")
        );
        assert!(!EDITOR_SPRITE_JS.contains("Math.round(sprite.size * config.ratio)"));
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
        assert!(!EDITOR_SPRITE_JS.contains("spriteBrushPreviewElement"));
        assert!(!EDITOR_SPRITE_JS.contains("sprite-brush-preview"));
        assert!(!EDITOR_SPRITE_JS.contains("finishSpritePaintMutation();"));
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
    fn source_completion_keyboard_commit_splits_tab_from_enter() {
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionSelectedIndexForSession"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionSessionMatches"));
        assert!(EDITOR_SOURCE_JS.contains("function sourceCompletionItemsMatch"));
        assert!(EDITOR_SOURCE_JS.contains("keyboardCommit: Boolean(options.manual || sourceCompletionSessionMatches(previousState, {"));
        assert!(EDITOR_SOURCE_JS.contains("sourceCompletionState.keyboardCommit = true;"));
        assert!(EDITOR_SOURCE_JS.contains("if (event.key === \"Tab\")"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("if (event.key === \"Enter\" && sourceCompletionCanKeyboardCommit())")
        );
        let tab_handler = EDITOR_SOURCE_JS
            .find("if (event.key === \"Tab\")")
            .expect("source completion Tab handler");
        let enter_handler = EDITOR_SOURCE_JS
            .find("if (event.key === \"Enter\" && sourceCompletionCanKeyboardCommit())")
            .expect("source completion Enter handler");
        assert!(EDITOR_SOURCE_JS.contains("if (sourceCompletionState.mode === \"completion\")"));
        assert!(tab_handler < enter_handler);
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
    fn source_completion_auto_requires_typed_prefix() {
        assert!(
            EDITOR_SOURCE_JS.contains("function sourceCursorHasCompletionPrefix(source, cursor)")
        );
        assert!(
            EDITOR_SOURCE_JS.contains(
                "return sourceCursorHasCompletionPrefix(source, cursor)\n    || sourceCursorAfterSelectorTagSeparator(source, cursor)\n    || Boolean(sourceImportPathCompletionContext(source, cursor));"
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
                "return sourceCursorHasCompletionPrefix(source, cursor)\n    || sourceCursorAfterSelectorTagSeparator(source, cursor)\n    || Boolean(sourceImportPathCompletionContext(source, cursor));"
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
    fn source_completion_suggests_import_paths_from_workspace_documents() {
        assert!(EDITOR_SOURCE_JS.contains(
            "const list = suggestSourceCompletionsFromEditorContext(source, cursor, document)\n      || await suggestSourceCompletionsWithWasm(source, cursor);"
        ));
        assert!(
            EDITOR_SOURCE_JS.contains("function sourceImportPathCompletionContext(source, cursor)")
        );
        assert!(
            EDITOR_SOURCE_JS
                .contains("function sourceImportPathCompletionItems(context, document)")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "if (!candidate || !isPuzzleDocument(candidate) || !isTextDocument(candidate))"
        ));
        assert!(EDITOR_SOURCE_JS.contains("candidatePath === currentDocumentPath"));
        assert!(EDITOR_SOURCE_JS.contains(
            "if (preferredRoot && normalizePath(candidate.workspaceRoot || \"\") !== preferredRoot)"
        ));
        assert!(
            EDITOR_SOURCE_JS
                .contains("const relativePath = sourceRelativeImportPath(document, candidate);")
        );
        assert!(EDITOR_SOURCE_JS.contains(
            "insertText: context.needsClosingQuote ? `${relativePath}\"` : relativePath,"
        ));
        assert!(EDITOR_SOURCE_JS.contains("needsClosingQuote: closeQuoteColumn < 0"));
        assert!(EDITOR_SOURCE_JS.contains("detail: \"path\""));
        assert!(EDITOR_SOURCE_JS.contains(
            "if (sourceImportPathCompletionContext(source, cursor)) {\n    return \"completion\";\n  }"
        ));
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
        assert!(EDITOR_SOURCE_JS.contains("let lineHit = null;"));
        assert!(EDITOR_SOURCE_JS.contains("let bestInLine = null;"));
        assert!(EDITOR_SOURCE_JS.contains("if (lineDistance === 0 && char !== \"\\n\")"));
        assert!(EDITOR_SOURCE_JS.contains("if (clientX >= lineHit.right)"));
        assert!(
            EDITOR_SOURCE_JS
                .contains("return Math.max(0, Math.min(source.length, lineHit.endOffset));")
        );
        assert!(EDITOR_JS.contains("function sourceOffsetFromEditorClick(event, source)"));
        assert!(
            EDITOR_JS.contains(
                "return sourceOffsetFromVisualPoint(event.clientX, event.clientY, source);"
            )
        );
    }

    #[test]
    fn source_line_gutter_does_not_capture_text_selection_drag() {
        assert!(EDITOR_CSS.contains(
            ".source-line-numbers {\n  position: absolute;\n  top: 0;\n  left: 0;\n  z-index: 2;"
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
            EDITOR_SOURCE_JS.contains(
                "const visualOffset = sourceViewOffsetFromVisualPoint(clientX, clientY);"
            )
        );
    }

    #[test]
    fn source_pointer_sync_uses_visual_click_offset() {
        assert!(EDITOR_JS.contains("function syncPreviewModeFromSourcePointer(event)"));
        assert!(
            EDITOR_JS.contains("const clickOffset = sourceOffsetFromEditorClick(event, source);")
        );
        assert!(EDITOR_JS.contains("position: clickOffset"));
    }

    #[test]
    fn source_block_selection_uses_normal_selection_fill() {
        assert!(EDITOR_CSS.contains(
            "--source-selection-bg: color-mix(in srgb, var(--accent) 34%, transparent);"
        ));
        assert!(EDITOR_CSS.contains(".source-block-selection-range {\n  position: absolute;\n  min-width: 2px;\n  background: var(--source-selection-bg);\n}"));
        assert!(
            EDITOR_CSS.contains(
                "#sourceEditor::selection {\n  background: var(--source-selection-bg);\n}"
            )
        );
    }

    #[test]
    fn source_editor_keeps_native_caret_visible() {
        let start = EDITOR_CSS
            .find("#sourceEditor {")
            .expect("source editor CSS block");
        let end = EDITOR_CSS[start..]
            .find("\n}")
            .map(|offset| start + offset)
            .expect("source editor CSS block end");
        let block = &EDITOR_CSS[start..end];
        assert!(block.contains("color: transparent;"));
        assert!(block.contains("caret-color: var(--code-ink);"));
        assert!(!block.contains("-webkit-text-fill-color: transparent;"));
    }

    #[test]
    fn level_and_solver_boards_use_preview_theme_colors() {
        for selector in [".level-board-wrap {", ".solver-board-wrap {"] {
            let start = EDITOR_CSS.find(selector).expect("board wrap CSS block");
            let end = EDITOR_CSS[start..]
                .find("\n}")
                .map(|offset| start + offset)
                .expect("board wrap CSS block end");
            let block = &EDITOR_CSS[start..end];
            assert!(block.contains("border: 1px solid var(--preview-game-line);"));
            assert!(block.contains("background: var(--preview-game-background);"));
            assert!(block.contains("color: var(--preview-game-ink);"));
            assert!(!block.contains("background: var(--bg);"));
        }
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
    fn sprite_asset_tables_require_explicit_selector_keys() {
        assert!(!EDITOR_SPRITE_JS.contains("`${tableName}:*`"));
        assert!(!EDITOR_SPRITE_JS.contains(":*"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const key = assets.has(name) ? name : spriteTableAssetKey(name, assets, selectorName);"
        ));
        assert!(EDITOR_SPRITE_JS.contains("return \"\";\n}"));
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
        assert!(EDITOR_SPRITE3D_JS.contains("function sprite3dSourceChildIndent(indent = \"\")"));
        assert!(
            EDITOR_SPRITE3D_JS
                .contains("sprite3dObjectDefinitionText(sprite3dSourceIndent(entry.indent))")
        );
    }

    #[test]
    fn sprite_source_loader_accepts_bare_shape_refs() {
        assert!(EDITOR_SPRITE_JS.contains("const shapeAssets = parseSpriteShapeAssets(source);"));
        assert!(EDITOR_SPRITE_JS.contains("} else if (asciiRows.length === 1) {"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const shapeRows = resolveSpriteShapeAssetToken(shapeName, shapeAssets, selectorName);"
        ));
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
    fn sprite_source_loader_projects_generic_table_refs() {
        assert!(EDITOR_SPRITE_JS.contains("function spriteSelectorSingleTagBinding("));
        assert!(EDITOR_SPRITE_JS.contains("function firstSpriteTableAssetKey(tableName, assets)"));
        assert!(EDITOR_SPRITE_JS.contains(
            "const selectorBinding = spriteSelectorSingleTagBinding(selectorName, name.slice(separator + 1));"
        ));
        assert!(EDITOR_SPRITE_JS.contains("return firstSpriteTableAssetKey(tableName, assets);"));
    }

    #[test]
    fn sprite_source_loader_reads_rotated_shape_table_base_rows() {
        assert!(EDITOR_SPRITE_JS.contains(
            "const tablePattern = /(^|\\n)([\\t ]*)([A-Za-z_][\\w]*):([A-Za-z_][\\w]*)([^\\n{]*)\\{/g;"
        ));
        assert!(EDITOR_SPRITE_JS.contains("function parseSpriteValueMaps(source)"));
        assert!(EDITOR_SPRITE_JS.contains("function parseSpriteShapeRotationDirective(text)"));
        assert!(EDITOR_SPRITE_JS.contains("function expandSpriteShapeRotationRows("));
        assert!(EDITOR_SPRITE_JS.contains("function rotateSpriteAsciiRowsClockwise(rows)"));
        assert!(EDITOR_SPRITE_JS.contains("markSpriteTableBindingAsset(raw, bindingKey);"));
        assert!(
            EDITOR_SPRITE_JS
                .contains("if (assets.has(name) && !spriteTableAssetIsBinding(assets, name))")
        );
        assert!(
            EDITOR_SPRITE_JS
                .contains("if (selectorValue && assets.has(`${tableName}:${selectorValue}`))")
        );
    }

    #[test]
    fn sprite_source_update_reveals_and_preserves_target_boundary() {
        assert!(EDITOR_JS.contains("editSourceName: \"\""));
        assert!(EDITOR_JS.contains("editSourceEnd: null"));
        assert!(EDITOR_JS.contains("editSourceBodyStart: null"));
        assert!(EDITOR_JS.contains("editSourceBodyEnd: null"));
        assert!(EDITOR_SPRITE_JS.contains("function revealSpriteSourceResult"));
        assert!(EDITOR_SPRITE_JS.contains("const result = { source, start: inserted.start };"));
        assert!(EDITOR_SPRITE_JS.contains("revealSourceLocation({"));
        assert!(EDITOR_SPRITE_JS.contains("recordHistory: false"));
        assert!(EDITOR_SPRITE_JS.contains("sourceEditor.focus({ preventScroll: true });"));
        assert!(EDITOR_SPRITE_JS.contains("function currentSpriteEditSourceRange(source)"));
        assert!(EDITOR_SPRITE_JS.contains("end: entry.start + replacement.length"));
        assert!(EDITOR_SPRITE_JS.contains(
            "setSpriteEditSource({ start: result.start, end: result.end, name: spriteObjectName() }, document);"
        ));
        assert!(EDITOR_SPRITE_JS.contains("const start = sprite.editSourceStart;"));
        assert!(EDITOR_SPRITE_JS.contains("const end = sprite.editSourceEnd;"));
        assert!(EDITOR_JS.contains(
            "const trailingBoundary = removed.match(/((?:\\r?\\n[\\t ]*)+)$/)?.[1] || \"\";"
        ));
    }

    #[test]
    fn sprite_source_edit_invalidates_cached_target_until_rust_resync() {
        assert!(EDITOR_SPRITE_JS.contains("function clearSpriteEditSource()"));
        assert!(EDITOR_SPRITE_JS.contains(
            "function invalidateSpriteEditSourceForDocument(document = activeDocument())"
        ));
        assert!(EDITOR_SPRITE_JS.contains("clearSpriteEditSource();"));
        assert!(EDITOR_SPRITE_JS.contains(
            "sourceEditor.addEventListener(\"input\", () => {\n  invalidateSpriteEditSourceForDocument(activeDocument());\n  syncSpriteSourceActionButtons();\n});"
        ));
        assert!(EDITOR_SPRITE_JS.contains("function loadSpriteSourceTarget(target, options = {})"));
        assert!(EDITOR_SPRITE_JS.contains("setSpriteEditSource(target, activeDocument());"));
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
    fn new_puzzle_starter_source_is_injected_from_authoring_template() {
        let workspace_js = editor_workspace_js();

        assert!(EDITOR_WORKSPACE_JS.contains("\"__PUZZLESTUDIO_NEW_PUZZLE_SOURCE__\""));
        assert!(!workspace_js.contains("__PUZZLESTUDIO_NEW_PUZZLE_SOURCE__"));
        assert!(workspace_js.contains(&js_string_literal(puzzle_authoring::NEW_PUZZLE_TEMPLATE)));
        assert!(workspace_js.contains("function starterPuzzleSource(name)"));
        assert!(workspace_js.contains("STARTER_PUZZLE_SOURCE.slice(defaultTitleLine.length)"));
    }

    #[test]
    fn new_puzzle_creation_uses_browser_template_source_for_all_hosts() {
        assert!(EDITOR_BOOT_JS.contains("New puzzle source is browser-runtime owned"));
        assert!(!EDITOR_BOOT_JS.contains(r#"invoke("new_puzzle_source", { request: payload })"#));
        assert!(EDITOR_WORKSPACE_JS.contains("async function newPuzzleSourceForFile(name)"));
        assert!(!EDITOR_WORKSPACE_JS.contains("window.PuzzleStudioHost.newPuzzleSource"));
        assert!(EDITOR_WORKSPACE_JS.contains("return starterPuzzleSourceFromTitle(title);"));
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
        assert!(html.contains(
            r#"<link rel="stylesheet" href="editor.css?v=source-folding-tabs-compact-close-left">"#
        ));
        assert!(
            html.contains(r#"<script src="editor.js?v=level-editor-import-reference"></script>"#)
        );
        assert!(html.contains(r#"<script src="editor_import_export.js"></script>"#));
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
        assert!(EDITOR_RUNTIME_JS.contains("compile_preview"));
        assert!(EDITOR_RUNTIME_JS.contains("highlight_source_html"));
        assert!(EDITOR_HTML.contains("editor_runtime.js"));
        assert!(
            EDITOR_HTML.find("editor_runtime.js").unwrap()
                < EDITOR_HTML.find("editor_boot.js").unwrap()
        );
        assert!(EDITOR_BOOT_JS.contains("editorRuntime().compilePreview(payload)"));
        assert!(EDITOR_BOOT_JS.contains("editorRuntime().highlightSource(payload)"));
        assert!(EDITOR_RUNTIME_JS.contains("gameRuntimeAssets()"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_game/puzzle_wasm_game.js"));
        assert!(EDITOR_RUNTIME_JS.contains("./wasm_game/puzzle_wasm_game_bg.wasm"));
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
        assert!(!EDITOR_WORKSPACE_JS.contains("treeWithEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("mergeEmbeddedFallbacks"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewHtml"));
        assert!(!EDITOR_WORKSPACE_JS.contains("editorSeed.previewError"));
        assert!(!EDITOR_WORKSPACE_JS.contains("document.previewHtml ||"));
        assert!(!EDITOR_WORKSPACE_JS.contains("previewDocument?.previewHtml"));
        assert!(EDITOR_WORKSPACE_JS.contains("invalidateCompiledPreview(previewDocument);"));
        assert!(EDITOR_WORKSPACE_JS.contains("Run preview to compile."));
    }

    #[test]
    fn editor_wasm_surface_excludes_runtime_exports() {
        assert!(PUZZLE_WASM_JS.contains("export function compile_preview"));
        assert!(PUZZLE_WASM_JS.contains("export function solve_state"));
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
    fn tauri_static_editor_includes_puzzle3_visual_core_asset() {
        assert_eq!(EDITOR_STATIC_PUZZLE3_VISUAL_CORE_JS, PUZZLE3_VISUAL_CORE_JS);
        assert!(EDITOR_HTML.contains(r#"<script src="puzzle3_visual_core.js"></script>"#));
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
            .find(r#"<script src="editor.js?v=level-editor-import-reference"></script>"#)
            .expect("editor loads main editor script");
        let import_export = EDITOR_HTML
            .find(r#"<script src="editor_import_export.js"></script>"#)
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
            EDITOR_HTML.contains(r#"<script src="editor_boot.js?v=host-mode-required"></script>"#)
        );
        assert!(EDITOR_HTML.contains(
            r#"<link rel="stylesheet" href="editor.css?v=source-folding-tabs-compact-close-left">"#
        ));
        assert!(
            EDITOR_HTML
                .contains(r#"<script src="editor_source.js?v=source-folding-state"></script>"#)
        );
        assert!(
            EDITOR_HTML
                .contains(r#"<script src="editor_workspace.js?v=import-parent-preview"></script>"#)
        );
        assert!(EDITOR_HTML.contains(r#"<script src="editor_import_export.js"></script>"#));
        assert!(
            EDITOR_HTML
                .contains(r#"<script src="editor.js?v=level-editor-import-reference"></script>"#)
        );
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
            "return { yawDegrees: 0, pitchDegrees: 90, zoom: 1, projection: \"orthographic\" };"
        ));
        assert!(
            EDITOR_LEVEL3D_JS
                .contains("`${activePreviewDocument()?.id || \"\"}:puzzle3-layer-renderer:${currentLevel3dLayerZ()}`")
        );
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
            .find(r#"<script src="editor_workspace.js?v=import-parent-preview"></script>"#)
            .expect("seeded editor should load workspace code after seed data");

        assert!(
            workspace_root_index < embedded_documents_index,
            "seeded web editor must strip workspace root before building the file tree"
        );
    }
}
