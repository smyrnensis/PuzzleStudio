use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

use html_editor::{
    AppError as EditorAppError, CreateSourceFileRequest, CreateSourceFolderRequest,
    DeleteWorkspaceEntryRequest, EditorService, PreviewRequest, RenameWorkspaceEntryRequest,
    SaveRequest,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

const WORKSPACE_CHANGED_EVENT: &str = "puzzlestudio-workspace-changed";
const WORKSPACE_WATCH_INTERVAL: Duration = Duration::from_millis(700);
const WORKSPACE_WATCH_DEBOUNCE: Duration = Duration::from_millis(150);
const LOADED_WORKSPACES_FILE: &str = "loaded-workspaces.json";
const RECENT_WORKSPACES_FILE: &str = "recent-workspaces.json";
const MAX_RECENT_WORKSPACES: usize = 8;
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

#[derive(Default)]
struct DesktopState {
    services: Mutex<Vec<EditorService>>,
    watchers: Mutex<Vec<WorkspaceWatcher>>,
}

struct WorkspaceWatcher {
    workspace_root: String,
    stop: Arc<AtomicBool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileFingerprint {
    len: u64,
    modified_ns: u128,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenWorkspaceCommandRequest {
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRecentWorkspaceCommandRequest {
    workspace_root: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentWorkspaceEntry {
    workspace_root: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewCommandRequest {
    source: String,
    puzzle_path: String,
    workspace_root: Option<String>,
    game_css: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HighlightCommandRequest {
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewPuzzleSourceCommandRequest {
    title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveCommandRequest {
    source: String,
    puzzle_path: String,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportHtmlCommandRequest {
    html: String,
    filename: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSourceFileCommandRequest {
    source: String,
    puzzle_path: String,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSourceFolderCommandRequest {
    folder_path: String,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameWorkspaceEntryCommandRequest {
    from_path: String,
    to_path: String,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteWorkspaceEntryCommandRequest {
    entry_path: String,
    workspace_root: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveWorkspaceCommandRequest {
    workspace_root: String,
}

#[tauri::command]
fn load_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    {
        let services = state
            .services
            .lock()
            .map_err(|_| "desktop project state is unavailable".to_string())?;
        if !services.is_empty() {
            let workspaces = services
                .iter()
                .map(editor_source_value)
                .collect::<Result<Vec<_>, _>>()?;
            let loaded_entries = workspace_entries_for_services(&services);
            write_workspace_entries(&app, LOADED_WORKSPACES_FILE, &loaded_entries)?;
            return workspaces_source_value_with_recent(&app, workspaces, Vec::new());
        }
    }

    let loaded = read_loaded_workspaces(&app)?;
    let (workspaces, restore_errors) = restore_workspace_payloads(&loaded, |workspace_root| {
        open_workspace_path(&app, &state, &PathBuf::from(workspace_root), false).map_err(|error| {
            eprintln!("failed to restore loaded workspace {workspace_root}: {error}");
            error
        })
    });
    if !workspaces.is_empty() {
        return workspaces_source_value_with_recent(&app, workspaces, restore_errors);
    }

    workspaces_source_value_with_recent(&app, Vec::new(), restore_errors)
}

#[tauri::command]
async fn open_workspace(
    app: tauri::AppHandle,
    request: OpenWorkspaceCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let kind = request.kind.as_deref();
    let Some(path) = pick_workspace_path(&app, kind).await? else {
        return Ok(serde_json::json!({ "canceled": true }));
    };
    let record_loaded = kind != Some("file");
    open_workspace_path(&app, &state, &path, record_loaded)
}

#[tauri::command]
fn recent_workspaces(app: tauri::AppHandle) -> Result<Vec<RecentWorkspaceEntry>, String> {
    read_recent_workspaces(&app)
}

#[tauri::command]
fn open_recent_workspace(
    app: tauri::AppHandle,
    request: OpenRecentWorkspaceCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let recent = read_recent_workspaces(&app)?;
    if !recent
        .iter()
        .any(|entry| entry.workspace_root == request.workspace_root)
    {
        return Err("Workspace is not in recent folders.".to_string());
    }
    open_workspace_path(&app, &state, &PathBuf::from(request.workspace_root), true)
}

fn open_workspace_path(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, DesktopState>,
    path: &Path,
    record_loaded: bool,
) -> Result<serde_json::Value, String> {
    let service = EditorService::open_game_entry(path).map_err(|error| error.to_string())?;
    let workspace_root = service.workspace_root().to_string();
    let puzzle_path = service.puzzle_path().to_string();
    if record_loaded {
        if let Err(error) = record_recent_workspace(app, &workspace_root) {
            eprintln!("failed to record recent workspace: {error}");
        }
    }
    let payload = editor_source_value_with_recent(app, &service)?;
    let mut services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    if record_loaded {
        let loaded_entries = loaded_workspace_entries_after_open(&services, &workspace_root);
        write_workspace_entries(app, LOADED_WORKSPACES_FILE, &loaded_entries)?;
    }
    replace_workspace_service(&mut services, service);
    drop(services);
    restart_workspace_watcher(app, state, workspace_root, puzzle_path)?;
    Ok(payload)
}

#[tauri::command]
async fn open_project(
    app: tauri::AppHandle,
    _request: OpenWorkspaceCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    open_workspace(
        app,
        OpenWorkspaceCommandRequest {
            kind: Some("folder".to_string()),
        },
        state,
    )
    .await
}

#[tauri::command]
fn remove_workspace(
    app: tauri::AppHandle,
    request: RemoveWorkspaceCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let mut services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let before = services.len();
    if services
        .iter()
        .any(|service| service.workspace_root() == request.workspace_root)
    {
        let remaining_roots = services
            .iter()
            .map(|service| service.workspace_root().to_string())
            .filter(|workspace_root| workspace_root != &request.workspace_root)
            .collect::<Vec<_>>();
        let loaded_entries = loaded_workspace_entries_for_roots(&remaining_roots);
        write_workspace_entries(&app, LOADED_WORKSPACES_FILE, &loaded_entries)?;
    }
    services.retain(|service| service.workspace_root() != request.workspace_root);
    stop_workspace_watcher(&state, &request.workspace_root)?;
    Ok(serde_json::json!({ "ok": true, "removed": before != services.len() }))
}

#[tauri::command]
fn compile_preview(
    request: PreviewCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, serde_json::Value> {
    let workspace_root = request.workspace_root.clone();
    let request = PreviewRequest::new(request.source, request.puzzle_path, request.game_css);
    let services = state
        .services
        .lock()
        .map_err(|_| serde_json::json!({ "error": "desktop project state is unavailable" }))?;
    let Some(service) = service_for_workspace(&services, workspace_root.as_deref()) else {
        return Err(serde_json::json!({
            "error": "No project is open. Open a project folder before running preview."
        }));
    };
    service
        .compile_preview(&request)
        .map_err(preview_command_error)
}

fn preview_command_error(error: EditorAppError) -> serde_json::Value {
    match error {
        EditorAppError::Diagnostics(report) => {
            let diagnostics = report
                .diagnostics()
                .iter()
                .map(|diagnostic| {
                    let span = diagnostic.primary_span.as_ref();
                    serde_json::json!({
                        "severity": diagnostic.severity.as_str(),
                        "code": diagnostic.code,
                        "file": span.and_then(|span| span.file.as_deref()).unwrap_or(""),
                        "line": span.and_then(|span| span.line),
                        "column": span.and_then(|span| span.column),
                        "sourceLine": span.and_then(|span| span.source_line.as_deref()),
                        "message": diagnostic.message,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({ "diagnostics": diagnostics })
        }
        other => serde_json::json!({ "error": other.to_string() }),
    }
}

#[tauri::command]
fn highlight_source(request: HighlightCommandRequest) -> String {
    EditorService::highlight_source_json(&request.source)
}

#[tauri::command]
fn sound_tools() -> String {
    html_editor::sound_tools_script()
}

#[tauri::command]
fn new_puzzle_source(request: NewPuzzleSourceCommandRequest) -> String {
    html_editor::new_puzzle_source(&request.title)
}

#[tauri::command]
fn save_source(
    request: SaveCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<String, String> {
    let service = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service) = service_for_workspace(&service, request.workspace_root.as_deref()) else {
        return Err("No project is open. Open a project folder before saving files.".to_string());
    };
    let request = SaveRequest::new(request.source, request.puzzle_path);
    service
        .save_source_file(&request)
        .map_err(|error| error.to_string())?;
    Ok("{\"ok\":true}".to_string())
}

#[tauri::command]
async fn export_html(
    app: tauri::AppHandle,
    request: ExportHtmlCommandRequest,
) -> Result<serde_json::Value, String> {
    let filename = export_html_file_name(request.filename.as_deref());
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    app.dialog()
        .file()
        .set_title("Export HTML")
        .add_filter("HTML", &["html", "htm"])
        .set_file_name(filename)
        .save_file(move |path| {
            let _ = sender.blocking_send(path);
        });
    let Some(path) = receiver.recv().await else {
        return Err("export dialog closed before returning a path".to_string());
    };
    let Some(path) = path else {
        return Ok(serde_json::json!({ "canceled": true }));
    };
    let mut path = path
        .into_path()
        .map_err(|error| format!("selected path is not a local filesystem path: {error}"))?;
    ensure_html_extension(&mut path);
    fs::write(&path, request.html).map_err(|error| {
        format!(
            "failed to write exported HTML to {}: {error}",
            path.display()
        )
    })?;
    Ok(serde_json::json!({
        "ok": true,
        "path": path.display().to_string(),
    }))
}

#[tauri::command]
fn create_source_file(
    app: tauri::AppHandle,
    request: CreateSourceFileCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let mut services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service_index) =
        service_index_for_workspace(&services, request.workspace_root.as_deref())
    else {
        return Err("No project is open. Open a project folder before adding files.".to_string());
    };
    let active_entry_is_empty = services[service_index].puzzle_path().trim().is_empty();
    let request = CreateSourceFileRequest::new(request.source, request.puzzle_path);
    let path = services[service_index]
        .create_source_file(&request)
        .map_err(|error| error.to_string())?;

    if active_entry_is_empty && is_desktop_puzzle_source_path(&path) {
        if let Ok(service) = EditorService::open_game_entry(&path) {
            let workspace_root = service.workspace_root().to_string();
            let puzzle_path = service.puzzle_path().to_string();
            services[service_index] = service;
            drop(services);
            restart_workspace_watcher(&app, &state, workspace_root, puzzle_path)?;
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "puzzlePath": path.display().to_string()
    }))
}

#[tauri::command]
fn create_source_folder(
    request: CreateSourceFolderCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service) = service_for_workspace(&services, request.workspace_root.as_deref()) else {
        return Err("No project is open. Open a project folder before adding folders.".to_string());
    };
    let request = CreateSourceFolderRequest::new(request.folder_path);
    let path = service
        .create_source_folder(&request)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "ok": true,
        "folderPath": path.display().to_string()
    }))
}

#[tauri::command]
fn rename_workspace_entry(
    app: tauri::AppHandle,
    request: RenameWorkspaceEntryCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let mut services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service_index) =
        service_index_for_workspace(&services, request.workspace_root.as_deref())
    else {
        return Err("No project is open. Open a project folder before renaming files.".to_string());
    };

    let service = &services[service_index];
    let workspace_root = PathBuf::from(service.workspace_root());
    let from_path = resolve_desktop_workspace_command_path(&request.from_path, &workspace_root)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let to_path = resolve_desktop_workspace_command_path(&request.to_path, &workspace_root);
    let active_entry_tail = active_entry_tail_under_path(service.puzzle_path(), &from_path)?;
    if active_entry_tail.is_some()
        && from_path.is_file()
        && !is_desktop_puzzle_source_path(&to_path)
    {
        return Err("cannot rename the active game entry to a non-puzzle source file".to_string());
    }

    let request = RenameWorkspaceEntryRequest::new(request.from_path, request.to_path);
    let path = services[service_index]
        .rename_workspace_entry(&request)
        .map_err(|error| error.to_string())?;

    if let Some(tail) = active_entry_tail {
        let next_entry_path = if tail.as_os_str().is_empty() {
            path.clone()
        } else {
            path.join(tail)
        };
        let service =
            EditorService::open_game_entry(&next_entry_path).map_err(|error| error.to_string())?;
        let workspace_root = service.workspace_root().to_string();
        let puzzle_path = service.puzzle_path().to_string();
        services[service_index] = service;
        drop(services);
        restart_workspace_watcher(&app, &state, workspace_root, puzzle_path)?;
    }

    Ok(serde_json::json!({
        "ok": true,
        "path": path.display().to_string()
    }))
}

#[tauri::command]
fn delete_workspace_entry(
    app: tauri::AppHandle,
    request: DeleteWorkspaceEntryCommandRequest,
    state: tauri::State<'_, DesktopState>,
) -> Result<serde_json::Value, String> {
    let mut services = state
        .services
        .lock()
        .map_err(|_| "desktop project state is unavailable".to_string())?;
    let Some(service_index) =
        service_index_for_workspace(&services, request.workspace_root.as_deref())
    else {
        return Err("No project is open. Open a project folder before deleting files.".to_string());
    };
    let service = &services[service_index];
    let workspace_root = PathBuf::from(service.workspace_root());
    let entry_path = resolve_desktop_workspace_command_path(&request.entry_path, &workspace_root)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let deletes_active_entry =
        active_entry_tail_under_path(service.puzzle_path(), &entry_path)?.is_some();

    let request = DeleteWorkspaceEntryRequest::new(request.entry_path);
    services[service_index]
        .delete_workspace_entry(&request)
        .map_err(|error| error.to_string())?;

    if deletes_active_entry {
        let service = EditorService::open_workspace_root(&workspace_root)
            .map_err(|error| error.to_string())?;
        let workspace_root = service.workspace_root().to_string();
        let puzzle_path = service.puzzle_path().to_string();
        services[service_index] = service;
        drop(services);
        restart_workspace_watcher(&app, &state, workspace_root, puzzle_path)?;
    }

    Ok(serde_json::json!({ "ok": true }))
}

fn empty_source_value() -> serde_json::Value {
    serde_json::json!({
        "puzzlePath": "",
        "workspaceRoot": "",
        "source": "",
        "gameCss": "",
        "documents": [],
        "empty": true
    })
}

fn editor_source_value(service: &EditorService) -> Result<serde_json::Value, String> {
    let source = service.source_json().map_err(|error| error.to_string())?;
    serde_json::from_str(&source).map_err(|error| error.to_string())
}

fn editor_source_value_with_recent(
    app: &tauri::AppHandle,
    service: &EditorService,
) -> Result<serde_json::Value, String> {
    source_value_with_recent(app, editor_source_value(service)?)
}

fn workspaces_source_value_with_recent(
    app: &tauri::AppHandle,
    workspaces: Vec<serde_json::Value>,
    restore_errors: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut payload = empty_source_value();
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert(
            "empty".to_string(),
            serde_json::Value::Bool(workspaces.is_empty()),
        );
        object.insert(
            "workspaces".to_string(),
            serde_json::Value::Array(workspaces),
        );
        if !restore_errors.is_empty() {
            object.insert(
                "restoreErrors".to_string(),
                serde_json::Value::Array(restore_errors),
            );
        }
    }
    source_value_with_recent(app, payload)
}

fn source_value_with_recent(
    app: &tauri::AppHandle,
    mut payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let recent = read_recent_workspaces(app)?;
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert(
            "recentWorkspaces".to_string(),
            serde_json::to_value(recent).map_err(|error| error.to_string())?,
        );
    }
    Ok(payload)
}

fn replace_workspace_service(services: &mut Vec<EditorService>, service: EditorService) {
    let workspace_root = service.workspace_root().to_string();
    if let Some(index) = services
        .iter()
        .position(|item| item.workspace_root() == workspace_root)
    {
        services[index] = service;
    } else {
        services.push(service);
    }
}

fn service_for_workspace<'a>(
    services: &'a [EditorService],
    workspace_root: Option<&str>,
) -> Option<&'a EditorService> {
    service_index_for_workspace(services, workspace_root).map(|index| &services[index])
}

fn service_index_for_workspace(
    services: &[EditorService],
    workspace_root: Option<&str>,
) -> Option<usize> {
    let requested = workspace_root.unwrap_or_default();
    if requested.trim().is_empty() {
        return (!services.is_empty()).then_some(0);
    }
    services
        .iter()
        .position(|service| service.workspace_root() == requested)
}

fn resolve_desktop_workspace_command_path(path: &str, workspace_root: &Path) -> PathBuf {
    let requested = PathBuf::from(path);
    if requested.is_absolute() {
        requested
    } else {
        workspace_root.join(requested)
    }
}

fn active_entry_tail_under_path(
    active_entry: &str,
    ancestor: &Path,
) -> Result<Option<PathBuf>, String> {
    let active_entry = active_entry.trim();
    if active_entry.is_empty() {
        return Ok(None);
    }
    let active_entry = PathBuf::from(active_entry)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if active_entry == ancestor {
        return Ok(Some(PathBuf::new()));
    }
    if active_entry.starts_with(ancestor) {
        let tail = active_entry
            .strip_prefix(ancestor)
            .map_err(|error| error.to_string())?;
        return Ok(Some(tail.to_path_buf()));
    }
    Ok(None)
}

fn is_desktop_puzzle_source_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
        "puzzle" | "puzzle3"
    )
}

fn restart_workspace_watcher(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, DesktopState>,
    workspace_root: String,
    puzzle_path: String,
) -> Result<(), String> {
    stop_workspace_watcher(state, &workspace_root)?;
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut watchers = state
            .watchers
            .lock()
            .map_err(|_| "desktop watcher state is unavailable".to_string())?;
        watchers.push(WorkspaceWatcher {
            workspace_root: workspace_root.clone(),
            stop: Arc::clone(&stop),
        });
    }

    let app = app.clone();
    thread::spawn(move || {
        watch_workspace(app, workspace_root, puzzle_path, stop);
    });
    Ok(())
}

fn stop_workspace_watcher(
    state: &tauri::State<'_, DesktopState>,
    workspace_root: &str,
) -> Result<(), String> {
    let mut watchers = state
        .watchers
        .lock()
        .map_err(|_| "desktop watcher state is unavailable".to_string())?;
    for watcher in watchers
        .iter()
        .filter(|watcher| watcher.workspace_root == workspace_root)
    {
        watcher.stop.store(true, Ordering::Relaxed);
    }
    watchers.retain(|watcher| watcher.workspace_root != workspace_root);
    Ok(())
}

fn watch_workspace(
    app: tauri::AppHandle,
    workspace_root: String,
    puzzle_path: String,
    stop: Arc<AtomicBool>,
) {
    let workspace_path = PathBuf::from(&workspace_root);
    let puzzle_path = puzzle_path.trim();
    let puzzle_path = if puzzle_path.is_empty() {
        workspace_path.clone()
    } else {
        PathBuf::from(puzzle_path)
    };
    let mut snapshot = workspace_snapshot(&workspace_path);

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(WORKSPACE_WATCH_INTERVAL);
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let mut next = workspace_snapshot(&workspace_path);
        if next == snapshot {
            continue;
        }
        thread::sleep(WORKSPACE_WATCH_DEBOUNCE);
        next = workspace_snapshot(&workspace_path);
        if next == snapshot {
            continue;
        }
        snapshot = next;

        let service = EditorService::open_game_entry(&puzzle_path);
        match service {
            Ok(service) => {
                let payload = match editor_workspace_changed_value(&service) {
                    Ok(payload) => payload,
                    Err(error) => {
                        emit_workspace_watch_error(&app, &workspace_root, &error);
                        continue;
                    }
                };
                let state = app.state::<DesktopState>();
                match state.services.lock() {
                    Ok(mut services) => replace_workspace_service(&mut services, service),
                    Err(_) => {
                        emit_workspace_watch_error(
                            &app,
                            &workspace_root,
                            "desktop project state is unavailable",
                        );
                        continue;
                    }
                }
                if let Err(error) = app.emit(WORKSPACE_CHANGED_EVENT, payload) {
                    eprintln!("failed to emit workspace change: {error}");
                }
            }
            Err(error) => emit_workspace_watch_error(&app, &workspace_root, &error.to_string()),
        }
    }
}

fn editor_workspace_changed_value(service: &EditorService) -> Result<serde_json::Value, String> {
    let mut payload = editor_source_value(service)?;
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert("external".to_string(), serde_json::Value::Bool(true));
    }
    Ok(payload)
}

fn emit_workspace_watch_error(app: &tauri::AppHandle, workspace_root: &str, error: &str) {
    let payload = serde_json::json!({
        "workspaceRoot": workspace_root,
        "external": true,
        "error": error,
    });
    if let Err(error) = app.emit(WORKSPACE_CHANGED_EVENT, payload) {
        eprintln!("failed to emit workspace watch error: {error}");
    }
}

fn workspace_snapshot(root: &Path) -> BTreeMap<String, FileFingerprint> {
    let mut files = BTreeMap::new();
    collect_workspace_snapshot(root, root, &mut files);
    files
}

fn collect_workspace_snapshot(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, FileFingerprint>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if should_skip_workspace_watch_dir(&path) {
                continue;
            }
            collect_workspace_snapshot(root, &path, files);
            continue;
        }
        if !metadata.is_file() || !is_workspace_file(&path) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        files.insert(
            relative.display().to_string(),
            FileFingerprint {
                len: metadata.len(),
                modified_ns: metadata_modified_ns(&metadata),
            },
        );
    }
}

fn should_skip_workspace_watch_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    SKIPPED_WORKSPACE_DIRS.contains(&name)
}

fn metadata_modified_ns(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
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

fn export_html_file_name(filename: Option<&str>) -> String {
    let candidate = filename
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("game.html")
        .trim();
    let candidate = if candidate.is_empty() {
        "game.html"
    } else {
        candidate
    };
    if Path::new(candidate).extension().is_some() {
        candidate.to_string()
    } else {
        format!("{candidate}.html")
    }
}

fn ensure_html_extension(path: &mut PathBuf) {
    if path.extension().is_none() {
        path.set_extension("html");
    }
}

async fn pick_workspace_path(
    app: &tauri::AppHandle,
    kind: Option<&str>,
) -> Result<Option<PathBuf>, String> {
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let file_dialog = app.dialog().file();
    if kind == Some("file") {
        file_dialog
            .set_title("Open a PuzzleStudio file")
            .add_filter("PuzzleStudio puzzle", &["puzzle", "puzzle3"])
            .pick_file(move |path| {
                let _ = sender.blocking_send(path);
            });
    } else {
        file_dialog
            .set_title("Open a PuzzleStudio workspace folder")
            .pick_folder(move |path| {
                let _ = sender.blocking_send(path);
            });
    };
    let Some(picked) = receiver.recv().await else {
        return Err("open dialog closed before returning a path".to_string());
    };
    picked
        .map(|path| {
            path.into_path()
                .map_err(|error| format!("selected path is not a local filesystem path: {error}"))
        })
        .transpose()
}

fn read_recent_workspaces(app: &tauri::AppHandle) -> Result<Vec<RecentWorkspaceEntry>, String> {
    read_workspace_entries(app, RECENT_WORKSPACES_FILE, Some(MAX_RECENT_WORKSPACES))
}

fn read_loaded_workspaces(app: &tauri::AppHandle) -> Result<Vec<RecentWorkspaceEntry>, String> {
    read_workspace_entries(app, LOADED_WORKSPACES_FILE, None)
}

fn read_workspace_entries(
    app: &tauri::AppHandle,
    filename: &str,
    max_entries: Option<usize>,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    let path = workspace_entries_path(app, filename)?;
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };
    let entries: Vec<RecentWorkspaceEntry> = serde_json::from_str(&source)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(clean_workspace_entries(entries, max_entries))
}

fn record_recent_workspace(app: &tauri::AppHandle, workspace_root: &str) -> Result<(), String> {
    let mut entries = read_recent_workspaces(app)?;
    push_recent_workspace(&mut entries, workspace_root);
    write_workspace_entries(app, RECENT_WORKSPACES_FILE, &entries)
}

fn write_workspace_entries(
    app: &tauri::AppHandle,
    filename: &str,
    entries: &[RecentWorkspaceEntry],
) -> Result<(), String> {
    let path = workspace_entries_path(app, filename)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let source = serde_json::to_string_pretty(entries).map_err(|error| error.to_string())?;
    fs::write(path, source).map_err(|error| error.to_string())
}

fn workspace_entries_path(app: &tauri::AppHandle, filename: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join(filename))
}

fn push_recent_workspace(entries: &mut Vec<RecentWorkspaceEntry>, workspace_root: &str) {
    push_workspace_entry(entries, workspace_root);
    entries.truncate(MAX_RECENT_WORKSPACES);
}

fn push_loaded_workspace(entries: &mut Vec<RecentWorkspaceEntry>, workspace_root: &str) {
    append_workspace_entry(entries, workspace_root);
}

fn push_workspace_entry(entries: &mut Vec<RecentWorkspaceEntry>, workspace_root: &str) {
    let workspace_root = workspace_root.trim();
    if workspace_root.is_empty() {
        return;
    }
    entries.retain(|entry| entry.workspace_root != workspace_root);
    entries.insert(0, workspace_entry(workspace_root));
}

fn append_workspace_entry(entries: &mut Vec<RecentWorkspaceEntry>, workspace_root: &str) {
    let workspace_root = workspace_root.trim();
    if workspace_root.is_empty()
        || entries
            .iter()
            .any(|entry| entry.workspace_root == workspace_root)
    {
        return;
    }
    entries.push(workspace_entry(workspace_root));
}

fn workspace_entry(workspace_root: &str) -> RecentWorkspaceEntry {
    RecentWorkspaceEntry {
        workspace_root: workspace_root.to_string(),
        name: recent_workspace_name(workspace_root),
    }
}

fn restore_workspace_payloads<F>(
    entries: &[RecentWorkspaceEntry],
    mut open_workspace: F,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>)
where
    F: FnMut(&str) -> Result<serde_json::Value, String>,
{
    let mut workspaces = Vec::new();
    let mut errors = Vec::new();
    for entry in entries {
        match open_workspace(&entry.workspace_root) {
            Ok(payload) => workspaces.push(payload),
            Err(error) => errors.push(serde_json::json!({
                "workspaceRoot": entry.workspace_root,
                "message": error,
            })),
        }
    }
    (workspaces, errors)
}

fn workspace_entries_for_services(services: &[EditorService]) -> Vec<RecentWorkspaceEntry> {
    services
        .iter()
        .map(|service| service.workspace_root().to_string())
        .fold(Vec::new(), |mut entries, workspace_root| {
            push_loaded_workspace(&mut entries, &workspace_root);
            entries
        })
}

fn loaded_workspace_entries_after_open(
    services: &[EditorService],
    opened_root: &str,
) -> Vec<RecentWorkspaceEntry> {
    let mut open_roots = services
        .iter()
        .map(|service| service.workspace_root().to_string())
        .collect::<Vec<_>>();
    if !open_roots
        .iter()
        .any(|workspace_root| workspace_root == opened_root)
    {
        open_roots.push(opened_root.to_string());
    }
    loaded_workspace_entries_for_roots(&open_roots)
}

fn loaded_workspace_entries_for_roots(open_roots: &[String]) -> Vec<RecentWorkspaceEntry> {
    open_roots
        .iter()
        .fold(Vec::new(), |mut entries, workspace_root| {
            push_loaded_workspace(&mut entries, workspace_root);
            entries
        })
}

#[cfg(test)]
fn clean_recent_workspaces(entries: Vec<RecentWorkspaceEntry>) -> Vec<RecentWorkspaceEntry> {
    clean_workspace_entries(entries, Some(MAX_RECENT_WORKSPACES))
}

fn clean_workspace_entries(
    entries: Vec<RecentWorkspaceEntry>,
    max_entries: Option<usize>,
) -> Vec<RecentWorkspaceEntry> {
    let mut clean = Vec::new();
    for entry in entries {
        let workspace_root = entry.workspace_root.trim();
        if workspace_root.is_empty()
            || clean
                .iter()
                .any(|item: &RecentWorkspaceEntry| item.workspace_root == workspace_root)
        {
            continue;
        }
        clean.push(RecentWorkspaceEntry {
            workspace_root: workspace_root.to_string(),
            name: if entry.name.trim().is_empty() {
                recent_workspace_name(workspace_root)
            } else {
                entry.name
            },
        });
        if max_entries.is_some_and(|limit| clean.len() >= limit) {
            break;
        }
    }
    clean
}

fn recent_workspace_name(workspace_root: &str) -> String {
    Path::new(workspace_root)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(workspace_root)
        .to_string()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![
            load_source,
            open_workspace,
            open_project,
            recent_workspaces,
            open_recent_workspace,
            remove_workspace,
            compile_preview,
            highlight_source,
            sound_tools,
            new_puzzle_source,
            save_source,
            export_html,
            create_source_file,
            create_source_folder,
            rename_workspace_entry,
            delete_workspace_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PuzzleStudio desktop app");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_workspace_push_is_most_recent_first_and_deduped() {
        let mut entries = vec![
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-a".to_string(),
                name: "project-a".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-b".to_string(),
                name: "project-b".to_string(),
            },
        ];

        push_recent_workspace(&mut entries, "/tmp/project-b");

        assert_eq!(
            entries,
            vec![
                RecentWorkspaceEntry {
                    workspace_root: "/tmp/project-b".to_string(),
                    name: "project-b".to_string(),
                },
                RecentWorkspaceEntry {
                    workspace_root: "/tmp/project-a".to_string(),
                    name: "project-a".to_string(),
                },
            ]
        );
    }

    #[test]
    fn recent_workspace_cleanup_preserves_stored_order() {
        let entries = clean_recent_workspaces(vec![
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-a".to_string(),
                name: "A".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-b".to_string(),
                name: "B".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-a".to_string(),
                name: "duplicate".to_string(),
            },
        ]);

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.workspace_root.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-a", "/tmp/project-b"]
        );
    }

    #[test]
    fn loaded_workspace_push_appends_without_recent_reordering() {
        let mut entries = (0..MAX_RECENT_WORKSPACES)
            .map(|index| RecentWorkspaceEntry {
                workspace_root: format!("/tmp/project-{index}"),
                name: format!("project-{index}"),
            })
            .collect::<Vec<_>>();

        push_loaded_workspace(&mut entries, "/tmp/project-extra");

        assert_eq!(entries.len(), MAX_RECENT_WORKSPACES + 1);
        assert_eq!(entries[0].workspace_root, "/tmp/project-0");
        assert_eq!(
            entries[MAX_RECENT_WORKSPACES].workspace_root,
            "/tmp/project-extra"
        );
    }

    #[test]
    fn loaded_workspace_cleanup_preserves_all_loaded_entries() {
        let entries = clean_workspace_entries(
            (0..MAX_RECENT_WORKSPACES + 2)
                .map(|index| RecentWorkspaceEntry {
                    workspace_root: format!("/tmp/project-{index}"),
                    name: format!("project-{index}"),
                })
                .collect(),
            None,
        );

        assert_eq!(entries.len(), MAX_RECENT_WORKSPACES + 2);
    }

    #[test]
    fn loaded_workspace_entries_follow_open_root_order_and_dedupe() {
        let open_roots = vec![
            "/tmp/project-a".to_string(),
            "/tmp/project-b".to_string(),
            "/tmp/project-c".to_string(),
            "/tmp/project-b".to_string(),
            " ".to_string(),
        ];

        let entries = loaded_workspace_entries_for_roots(&open_roots);

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.workspace_root.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-a", "/tmp/project-b", "/tmp/project-c"]
        );
    }

    #[test]
    fn restore_workspace_payloads_attempts_every_loaded_entry_in_order() {
        let entries = vec![
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-a".to_string(),
                name: "A".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-b".to_string(),
                name: "B".to_string(),
            },
        ];
        let mut opened = Vec::new();

        let (workspaces, errors) = restore_workspace_payloads(&entries, |workspace_root| {
            opened.push(workspace_root.to_string());
            Ok(serde_json::json!({ "workspaceRoot": workspace_root }))
        });

        assert_eq!(opened, vec!["/tmp/project-a", "/tmp/project-b"]);
        assert_eq!(workspaces.len(), 2);
        assert!(errors.is_empty());
    }

    #[test]
    fn restore_workspace_payloads_reports_failures_and_continues() {
        let entries = vec![
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-a".to_string(),
                name: "A".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-b".to_string(),
                name: "B".to_string(),
            },
            RecentWorkspaceEntry {
                workspace_root: "/tmp/project-c".to_string(),
                name: "C".to_string(),
            },
        ];
        let mut opened = Vec::new();

        let (workspaces, errors) = restore_workspace_payloads(&entries, |workspace_root| {
            opened.push(workspace_root.to_string());
            if workspace_root == "/tmp/project-b" {
                Err("missing folder".to_string())
            } else {
                Ok(serde_json::json!({ "workspaceRoot": workspace_root }))
            }
        });

        assert_eq!(
            opened,
            vec!["/tmp/project-a", "/tmp/project-b", "/tmp/project-c"]
        );
        assert_eq!(
            workspaces
                .iter()
                .filter_map(|payload| payload.get("workspaceRoot")?.as_str())
                .collect::<Vec<_>>(),
            vec!["/tmp/project-a", "/tmp/project-c"]
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0]
                .get("workspaceRoot")
                .and_then(|value| value.as_str()),
            Some("/tmp/project-b")
        );
        assert_eq!(
            errors[0].get("message").and_then(|value| value.as_str()),
            Some("missing folder")
        );
    }

    #[test]
    fn export_html_file_name_uses_leaf_and_html_default() {
        assert_eq!(export_html_file_name(Some("my-game.html")), "my-game.html");
        assert_eq!(export_html_file_name(Some("/tmp/my-game")), "my-game.html");
        assert_eq!(export_html_file_name(Some("")), "game.html");
        assert_eq!(export_html_file_name(None), "game.html");
    }

    #[test]
    fn ensure_html_extension_only_adds_missing_extension() {
        let mut missing = PathBuf::from("/tmp/game");
        ensure_html_extension(&mut missing);
        assert_eq!(missing, PathBuf::from("/tmp/game.html"));

        let mut existing = PathBuf::from("/tmp/game.htm");
        ensure_html_extension(&mut existing);
        assert_eq!(existing, PathBuf::from("/tmp/game.htm"));
    }

    #[test]
    fn desktop_new_puzzle_source_uses_authoring_template() {
        let source = new_puzzle_source(NewPuzzleSourceCommandRequest {
            title: "Desktop Test".to_string(),
        });

        assert!(source.starts_with("title \"Desktop Test\"\n"));
        assert!(source.contains("puzzle main {"));
        assert!(source.contains("scene playing {"));
    }

    #[test]
    fn active_entry_tail_under_path_tracks_exact_file_mutation() {
        let root = std::env::temp_dir().join(format!(
            "puzzlestudio-desktop-active-file-{}",
            std::process::id()
        ));
        let old_path = root.join("old.puzzle");
        fs::create_dir_all(&root).expect("create test root");
        fs::write(&old_path, "title \"Old\"\n").expect("write active entry");

        let tail = active_entry_tail_under_path(
            old_path.to_str().expect("utf-8 path"),
            &old_path.canonicalize().expect("canonical old path"),
        )
        .expect("resolve active entry tail");

        assert_eq!(tail, Some(PathBuf::new()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_entry_tail_under_path_tracks_parent_folder_mutation() {
        let root = std::env::temp_dir().join(format!(
            "puzzlestudio-desktop-active-folder-{}",
            std::process::id()
        ));
        let folder = root.join("folder");
        let old_path = folder.join("game.puzzle");
        fs::create_dir_all(&folder).expect("create test folder");
        fs::write(&old_path, "title \"Old\"\n").expect("write active entry");

        let tail = active_entry_tail_under_path(
            old_path.to_str().expect("utf-8 path"),
            &folder.canonicalize().expect("canonical folder"),
        )
        .expect("resolve active entry tail");

        assert_eq!(tail, Some(PathBuf::from("game.puzzle")));
        let _ = fs::remove_dir_all(root);
    }
}
