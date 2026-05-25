use std::path::PathBuf;
use std::sync::Mutex;

use html_editor::{CreateSourceFileRequest, EditorService, PreviewRequest, SaveRequest};
use serde::Deserialize;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct DesktopState {
    service: Mutex<Option<EditorService>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenProjectCommandRequest {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewCommandRequest {
    source: String,
    puzzle_path: String,
    game_css: String,
    game_visuals_js: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HighlightCommandRequest {
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveCommandRequest {
    source: String,
    puzzle_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSourceFileCommandRequest {
    source: String,
    puzzle_path: String,
}

#[tauri::command]
fn load_source(state: tauri::State<'_, DesktopState>) -> Result<serde_json::Value, String> {
    let service = state
        .service
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    match service.as_ref() {
        Some(service) => editor_source_value(service),
        None => Ok(empty_source_value()),
    }
}

#[tauri::command]
async fn open_project(
    app: tauri::AppHandle,
    _request: OpenProjectCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let Some(path) = pick_project_folder(&app)? else {
        return Ok(serde_json::json!({ "canceled": true }));
    };
    let service = EditorService::open_game_entry(&path).map_err(|error| error.to_string())?;
    let payload = editor_source_value(&service)?;
    let mut current = state
        .service
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    *current = Some(service);
    Ok(payload)
}

#[tauri::command]
fn compile_preview(
    request: PreviewCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, String> {
    let request = PreviewRequest::new(
        request.source,
        request.puzzle_path,
        request.game_css,
        request.game_visuals_js,
    );
    let service = state
        .service
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service) = service.as_ref() else {
        return Err(
            "No project is open. Open a project folder before running preview.".to_string(),
        );
    };
    service
        .compile_preview(&request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn highlight_source(request: HighlightCommandRequest) -> String {
    EditorService::highlight_source_json(&request.source)
}

#[tauri::command]
fn save_source(
    request: SaveCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, String> {
    let service = state
        .service
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service) = service.as_ref() else {
        return Err("No project is open. Open a project folder before saving files.".to_string());
    };
    let request = SaveRequest::new(request.source, request.puzzle_path);
    service
        .save_source_file(&request)
        .map_err(|error| error.to_string())?;
    Ok("{\"ok\":true}".to_string())
}

#[tauri::command]
fn create_source_file(
    request: CreateSourceFileCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let service = state
        .service
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service) = service.as_ref() else {
        return Err("No project is open. Open a project folder before adding files.".to_string());
    };
    let request = CreateSourceFileRequest::new(request.source, request.puzzle_path);
    let path = service
        .create_source_file(&request)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "ok": true,
        "puzzlePath": path.display().to_string()
    }))
}

fn empty_source_value() -> serde_json::Value {
    serde_json::json!({
        "puzzlePath": "",
        "workspaceRoot": "",
        "source": "",
        "gameCss": "",
        "gameVisualsJs": "",
        "documents": [],
        "empty": true
    })
}

fn editor_source_value(service: &EditorService) -> Result<serde_json::Value, String> {
    serde_json::from_str(&service.source_json()).map_err(|error| error.to_string())
}

fn pick_project_folder(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    app.dialog()
        .file()
        .set_title("Open a PuzzleStudio project folder")
        .blocking_pick_folder()
        .map(|path| {
            path.into_path()
                .map_err(|error| format!("selected folder is not a local filesystem path: {error}"))
        })
        .transpose()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![
            load_source,
            open_project,
            compile_preview,
            highlight_source,
            save_source,
            create_source_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PuzzleStudio desktop app");
}
