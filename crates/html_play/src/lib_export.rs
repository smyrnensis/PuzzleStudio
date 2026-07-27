#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StandaloneHostMode {
    Export,
    EditorPreview,
}

#[derive(Clone, Copy, Debug)]
enum StandaloneRuntimeWasm<'a> {
    HostDefault,
    EmbeddedBase64 {
        module_source: &'a str,
        wasm_base64: &'a str,
    },
}

fn export_html_with_runtime_wasm(
    state: &EditorPreviewState,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    let runtime_export = runtime_export_json(&state.standalone_export)
        .expect("standalone runtime document should serialize");
    if host_mode == StandaloneHostMode::Export {
        return bevy_export_html(&runtime_export, runtime_wasm);
    }
    bevy_editor_preview_html(&runtime_export, runtime_wasm)
}

fn bevy_export_html(runtime_export: &str, runtime_wasm: StandaloneRuntimeWasm<'_>) -> String {
    let runtime_export = escape_script_json(runtime_export);
    let runtime_wasm_js = standalone_runtime_wasm_script(StandaloneHostMode::Export, runtime_wasm);
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>PuzzleStudio HTML Export</title>
    <style>
      html, body {{
        width: 100%;
        height: 100%;
        margin: 0;
        overflow: hidden;
        background: #000;
      }}
      #puzzle-bevy {{
        display: block;
        width: 100%;
        height: 100%;
      }}
      #puzzle-bevy-fatal {{
        position: fixed;
        inset: 0;
        display: none;
        box-sizing: border-box;
        padding: 2rem;
        overflow: auto;
        white-space: pre-wrap;
        color: #fff;
        background: #280b0b;
        font: 16px/1.5 system-ui, sans-serif;
      }}
    </style>
  </head>
  <body>
    <canvas id="puzzle-bevy" aria-label="Puzzle game"></canvas>
    <pre id="puzzle-bevy-fatal" role="alert"></pre>
    <output id="puzzle-bevy-status" hidden data-state="starting"></output>
    <script>
window.PuzzleRuntimeExportJson = "{runtime_export}";
{runtime_wasm_js}
    </script>
    <script>
      (async () => {{
        const fatal = document.getElementById("puzzle-bevy-fatal");
        const status = document.getElementById("puzzle-bevy-status");
        try {{
          const player = await window.PuzzleRuntimeWasmLoader.load("bevy-player");
          if (typeof player.startStandalonePlayer !== "function") {{
            throw new Error("Standalone player WASM is missing startStandalonePlayer.");
          }}
          await player.startStandalonePlayer(window.PuzzleRuntimeExportJson, "#puzzle-bevy");
        }} catch (error) {{
          const message = error?.stack || error?.message || String(error);
          fatal.textContent = `PuzzleStudio standalone player failed:\n${{message}}`;
          fatal.style.display = "block";
          status.dataset.state = "fatal";
          console.error(message);
        }}
      }})();
    </script>
  </body>
</html>
"##
    )
}

fn bevy_editor_preview_html(
    runtime_export: &str,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    let runtime_export = escape_script_json(runtime_export);
    let runtime_wasm_js =
        standalone_runtime_wasm_script(StandaloneHostMode::EditorPreview, runtime_wasm);
    let editor_preview_browser_bridge_js = editor_preview_browser_bridge_script();
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>PuzzleStudio Editor Preview</title>
    <style>
      html, body {{
        width: 100%;
        height: 100%;
        margin: 0;
        overflow: hidden;
        background: #000;
      }}
      #puzzle-bevy {{
        display: block;
        width: 100%;
        height: 100%;
      }}
      #puzzle-bevy-fatal {{
        position: fixed;
        inset: 0;
        display: none;
        box-sizing: border-box;
        padding: 2rem;
        overflow: auto;
        white-space: pre-wrap;
        color: #fff;
        background: #280b0b;
        font: 16px/1.5 system-ui, sans-serif;
      }}
    </style>
  </head>
  <body>
    <canvas id="puzzle-bevy" aria-label="Puzzle game editor preview"></canvas>
    <pre id="puzzle-bevy-fatal" role="alert"></pre>
    <output id="puzzle-bevy-status" hidden data-state="starting"></output>
    <script>
window.PuzzleRuntimeExportJson = "{runtime_export}";
{runtime_wasm_js}
    </script>
    <script>
      (async () => {{
        const fatal = document.getElementById("puzzle-bevy-fatal");
        const status = document.getElementById("puzzle-bevy-status");
        const parentOrigin = (() => {{
          try {{
            return new URL(document.referrer).origin;
          }} catch (_error) {{
            return "";
          }}
        }})();
        try {{
          if (!parentOrigin || parentOrigin === "null") {{
            throw new Error("Editor preview requires a concrete parent origin.");
          }}
          window.PuzzleEditorPreviewParentOrigin = parentOrigin;
{editor_preview_browser_bridge_js}
          const editorPreview = await window.PuzzleRuntimeWasmLoader.load("bevy-editor-preview");
          if (typeof editorPreview.startEditorPreview !== "function") {{
            throw new Error("Editor preview WASM is missing startEditorPreview.");
          }}
          if (typeof editorPreview.dispatchEditorPreviewCommand !== "function") {{
            throw new Error("Editor preview WASM is missing dispatchEditorPreviewCommand.");
          }}
          window.addEventListener("PuzzleStudioEditorPreviewObservation", (event) => {{
            if (!event.detail || typeof event.detail !== "object") {{
              return;
            }}
            window.parent.postMessage(event.detail, parentOrigin);
          }});
          const canvas = document.getElementById("puzzle-bevy");
          const forwardEditorPointer = (gesture) => (event) => {{
            const rect = canvas.getBoundingClientRect();
            window.parent.postMessage({{
              type: "PuzzleStudioEditorPointer",
              gesture,
              xCss: Number(event.clientX) - rect.left,
              yCss: Number(event.clientY) - rect.top,
            }}, parentOrigin);
          }};
          canvas.addEventListener("pointermove", forwardEditorPointer("move"));
          canvas.addEventListener("pointerdown", forwardEditorPointer("press"));
          canvas.addEventListener("pointerup", forwardEditorPointer("release"));
          canvas.addEventListener("pointercancel", forwardEditorPointer("leave"));
          canvas.addEventListener("pointerleave", forwardEditorPointer("leave"));
          window.addEventListener("message", async (event) => {{
            if (
              event.source !== window.parent
              || event.origin !== parentOrigin
            ) {{
              return;
            }}
            const envelope = event.data || {{}};
            if (envelope.type !== "PuzzleStudioEditorPreviewCommand") {{
              return;
            }}
            if (typeof envelope.commandJson !== "string") {{
              window.parent.postMessage({{
                type: "PuzzleStudioPreviewRuntimeError",
                commandId: envelope.commandId,
                label: "invalid editor preview command",
                message: "PuzzleStudioEditorPreviewCommand requires commandJson.",
              }}, parentOrigin);
              return;
            }}
            try {{
              await editorPreview.dispatchEditorPreviewCommand(envelope.commandJson);
            }} catch (error) {{
              window.parent.postMessage({{
                type: "PuzzleStudioPreviewRuntimeError",
                commandId: envelope.commandId,
                label: "editor preview command failed",
                message: error?.stack || error?.message || String(error),
              }}, parentOrigin);
            }}
          }});
          await editorPreview.startEditorPreview(
            window.PuzzleRuntimeExportJson,
            "#puzzle-bevy",
          );
        }} catch (error) {{
          const message = error?.stack || error?.message || String(error);
          fatal.textContent = `PuzzleStudio editor preview failed:\n${{message}}`;
          fatal.style.display = "block";
          status.dataset.state = "fatal";
          if (parentOrigin && parentOrigin !== "null") {{
            window.parent.postMessage({{
              type: "PuzzleStudioPreviewRuntimeError",
              label: "runtime initialization failed",
              message,
            }}, parentOrigin);
          }}
          console.error(message);
        }}
      }})();
    </script>
  </body>
</html>
"##
    )
}

fn editor_preview_browser_bridge_script() -> &'static str {
    r#"          const postParent = (payload) => {
            window.parent.postMessage(payload, parentOrigin);
          };
          const isEditorSaveShortcut = (event) => {
            if (!event || event.altKey) {
              return false;
            }
            const modifier = (event.metaKey && !event.ctrlKey)
              || (event.ctrlKey && !event.metaKey);
            const key = event.key && event.key.length === 1
              ? event.key.toLowerCase()
              : event.key;
            return modifier && key === "s";
          };
          document.addEventListener("keydown", (event) => {
            if (!isEditorSaveShortcut(event)) {
              return;
            }
            event.preventDefault();
            event.stopImmediatePropagation();
            try {
              postParent({ type: "PuzzleStudioEditorSaveShortcut" });
            } catch (_error) {
              // Editor shortcuts must not affect the preview runtime.
            }
          }, true);
          const formatPreviewLogArgument = (value, depth = 0) => {
            if (typeof value === "string") {
              return value;
            }
            if (value instanceof Error) {
              const headline = [value.name || "Error", value.message || ""]
                .filter(Boolean)
                .join(": ");
              const stack = String(value.stack || "");
              if (!stack) {
                return headline || String(value);
              }
              return value.message && stack.includes(value.message)
                ? stack
                : [headline, stack].filter(Boolean).join("\n");
            }
            if (value === undefined) {
              return "undefined";
            }
            if (
              value === null
              || typeof value === "number"
              || typeof value === "boolean"
              || typeof value === "bigint"
            ) {
              return String(value);
            }
            if (depth > 1) {
              return Object.prototype.toString.call(value);
            }
            try {
              return JSON.stringify(value, (_key, nested) => {
                if (typeof nested === "function") {
                  return "[Function]";
                }
                return nested;
              });
            } catch (_error) {
              return String(value);
            }
          };
          const previewLogStackOrigin = () => {
            const stack = new Error().stack || "";
            const lines = stack.split("\n").slice(1);
            for (const line of lines) {
              const text = String(line || "").trim();
              if (
                !text
                || text.includes("postPreviewLog")
                || text.includes("previewLogStackOrigin")
                || text.includes("console.")
              ) {
                continue;
              }
              const match = text.match(/(?:at\s+)?(?:.*?\()?([^()\s]+:\d+:\d+)\)?$/);
              if (match) {
                return match[1];
              }
            }
            return "";
          };
          const postPreviewLog = (level, args) => {
            try {
              postParent({
                type: "PuzzleStudioPreviewLog",
                level,
                source: "preview console",
                origin: previewLogStackOrigin(),
                message: Array.from(args || [])
                  .map((argument) => formatPreviewLogArgument(argument))
                  .join(" "),
              });
            } catch (_error) {
              // Logging must not affect the preview runtime.
            }
          };
          for (const level of ["debug", "log", "info", "warn", "error"]) {
            const original = console[level]?.bind(console);
            console[level] = (...args) => {
              postPreviewLog(level, args);
              if (original) {
                original(...args);
              }
            };
          }
          window.addEventListener("error", (event) => {
            if (
              window.PuzzleStudioPreviewRuntimeFailure
              && event.message === "Script error."
            ) {
              return;
            }
            try {
              postParent({
                type: "PuzzleStudioPreviewLog",
                level: "error",
                source: "preview runtime",
                origin: event.filename && event.lineno
                  ? String(event.filename) + ":" + event.lineno + ":" + (event.colno || 0)
                  : "",
                message: formatPreviewLogArgument(
                  event.error || event.message || "Runtime error",
                ),
              });
            } catch (_error) {
              // Logging must not affect the preview runtime.
            }
          });
          window.addEventListener("unhandledrejection", (event) => {
            const failure = window.PuzzleStudioPreviewRuntimeFailure;
            const reasonMessage = String(event.reason?.message || event.reason || "");
            if (failure?.message && failure.message === reasonMessage) {
              return;
            }
            try {
              postParent({
                type: "PuzzleStudioPreviewLog",
                level: "error",
                source: "preview promise",
                origin: "",
                message: formatPreviewLogArgument(
                  event.reason || "Unhandled promise rejection",
                ),
              });
            } catch (_error) {
              // Logging must not affect the preview runtime.
            }
          });
          const postPreviewLoaded = () => {
            try {
              postParent({
                type: "PuzzleStudioPreviewLoaded",
                title: document.title || "",
                href: location.href || "",
              });
            } catch (_error) {
              // Runtime observability must not affect the preview runtime.
            }
          };
          if (document.readyState === "complete") {
            queueMicrotask(postPreviewLoaded);
          } else {
            window.addEventListener("load", postPreviewLoaded, { once: true });
          }"#
}

fn standalone_runtime_wasm_script(
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> String {
    if let StandaloneRuntimeWasm::EmbeddedBase64 {
        module_source,
        wasm_base64,
    } = runtime_wasm
    {
        return embedded_base64_wasm_loader_script(module_source, wasm_base64);
    }
    match host_mode {
        StandaloneHostMode::Export => embedded_player_wasm_script(),
        StandaloneHostMode::EditorPreview => editor_preview_runtime_wasm_script(),
    }
}

fn embedded_player_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_wasm_loader_script(PUZZLE_PLAYER_WASM_JS, PUZZLE_PLAYER_WASM_BG)
    }
    #[cfg(target_arch = "wasm32")]
    {
        missing_embedded_wasm_loader_script(
            "Standalone HTML export requires embedded puzzle_wasm_player assets.",
        )
    }
}

fn editor_preview_runtime_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_game_wasm_script()
    }
    #[cfg(target_arch = "wasm32")]
    {
        editor_preview_parent_wasm_loader_script()
    }
}

fn embedded_game_wasm_script() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        embedded_wasm_loader_script(PUZZLE_GAME_WASM_JS, PUZZLE_GAME_WASM_BG)
    }
    #[cfg(target_arch = "wasm32")]
    {
        missing_embedded_wasm_loader_script(
            "Editor preview requires puzzle_wasm_game assets from its editor host.",
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn embedded_wasm_loader_script(module_source: &str, wasm: &[u8]) -> String {
    let wasm_base64 = base64_encode(wasm);
    embedded_base64_wasm_loader_script(module_source, &wasm_base64)
}

fn embedded_base64_wasm_loader_script(module_source: &str, wasm_base64: &str) -> String {
    let module_source = escape_script_json(module_source);
    let wasm_base64 = escape_script_json(wasm_base64);
    format!(
        r#"window.PuzzleStandaloneEmbeddedWasm = {{ moduleSource: "{module_source}", wasmBase64: "{wasm_base64}" }};
window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {{
  let modulePromise = null;
  function decodeWasmBase64(value) {{
    if (typeof Uint8Array.fromBase64 !== "function") {{
      throw new Error("Standalone HTML export requires Uint8Array.fromBase64 for embedded WASM decoding.");
    }}
    return Uint8Array.fromBase64(value);
  }}
  return {{
    async load(version = "embedded") {{
      if (!modulePromise) {{
        const embedded = window.PuzzleStandaloneEmbeddedWasm;
        const moduleUrl = URL.createObjectURL(new Blob([embedded.moduleSource], {{ type: "text/javascript" }}));
        modulePromise = import(`${{moduleUrl}}#${{encodeURIComponent(String(version))}}`)
          .then(async (module) => {{
            const wasmBytes = decodeWasmBase64(embedded.wasmBase64);
            embedded.wasmBase64 = "";
            await module.default({{ module_or_path: wasmBytes }});
            return module;
          }})
          .finally(() => URL.revokeObjectURL(moduleUrl));
      }}
      return modulePromise;
    }},
  }};
}})();"#
    )
}

#[cfg(target_arch = "wasm32")]
fn missing_embedded_wasm_loader_script(message: &str) -> String {
    let message = escape_script_json(message);
    format!(
        r#"window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || {{
  async load() {{
    throw new Error("{message}");
  }},
}};"#
    )
}

#[cfg(target_arch = "wasm32")]
fn editor_preview_parent_wasm_loader_script() -> String {
    r#"window.PuzzleRuntimeWasmLoader = window.PuzzleRuntimeWasmLoader || (() => {
  let modulePromise = null;
  let nextRequestId = 1;

  function requiredParentOrigin() {
    const origin = window.PuzzleEditorPreviewParentOrigin;
    if (typeof origin !== "string" || !origin || origin === "null") {
      throw new Error("Editor preview runtime assets require a concrete parent origin.");
    }
    return origin;
  }

  function reportProgress(stage) {
    window.parent.postMessage({
      type: "PuzzleStudioPreviewRuntimeProgress",
      stage,
    }, requiredParentOrigin());
  }

  function requestAsset(kind) {
    return new Promise((resolve, reject) => {
      const requestId = `runtime-asset-${Date.now()}-${nextRequestId++}`;
      const timeout = window.setTimeout(() => {
        window.removeEventListener("message", handleMessage);
        reject(new Error(`Timed out waiting for editor preview runtime asset: ${kind}`));
      }, 15000);
      function handleMessage(event) {
        const parentOrigin = requiredParentOrigin();
        if (event.source !== window.parent || event.origin !== parentOrigin) {
          return;
        }
        const data = event.data || {};
        if (data.type !== "PuzzleStudioRuntimeAssetResponse" || data.requestId !== requestId) {
          return;
        }
        window.clearTimeout(timeout);
        window.removeEventListener("message", handleMessage);
        if (!data.ok) {
          reject(new Error(data.error || `Editor preview runtime asset is unavailable: ${kind}`));
          return;
        }
        resolve(String(data.value || ""));
      }
      window.addEventListener("message", handleMessage);
      window.parent.postMessage({
        type: "PuzzleStudioRuntimeAssetRequest",
        requestId,
        kind,
      }, requiredParentOrigin());
    });
  }

  function decodeWasmBase64(value) {
    if (typeof Uint8Array.fromBase64 !== "function") {
      throw new Error("Editor preview requires Uint8Array.fromBase64 for WASM decoding.");
    }
    return Uint8Array.fromBase64(value);
  }

  return {
    async load(version = "editor-preview") {
      if (!modulePromise) {
        modulePromise = Promise.all([
          requestAsset("puzzle_wasm_game.js"),
          requestAsset("puzzle_wasm_game_bg.wasm.base64"),
        ]).then(async ([moduleSource, wasmBase64]) => {
          reportProgress("assets-received");
          const moduleUrl = URL.createObjectURL(new Blob([moduleSource], { type: "text/javascript" }));
          try {
            const module = await import(`${moduleUrl}#${encodeURIComponent(String(version))}`);
            reportProgress("module-imported");
            const wasmBytes = decodeWasmBase64(wasmBase64);
            reportProgress("wasm-decoded");
            await module.default({ module_or_path: wasmBytes });
            reportProgress("wasm-initialized");
            return module;
          } finally {
            URL.revokeObjectURL(moduleUrl);
          }
        }).catch((error) => {
          modulePromise = null;
          throw error;
        });
      }
      return modulePromise;
    },
  };
})();"#.to_string()
}

fn export_bevy_document_html(
    document: &puzzle_lang::LoadedDocument,
    puzzle_path: &str,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, String> {
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)
        .map_err(|error| error.to_string())?;
    let progress_storage = standalone_progress_storage(document);
    let standalone_export =
        StandaloneRuntimeExport::new(document.clone(), visual_images, progress_storage);
    let runtime_export =
        runtime_export_json(&standalone_export).map_err(|error| error.to_string())?;
    Ok(bevy_export_html(&runtime_export, runtime_wasm))
}

fn document_uses_puzzle3_renderer(document: &puzzle_lang::LoadedDocument) -> bool {
    document
        .models
        .iter()
        .any(|model| matches!(model, LoadedDocumentModel::Puzzle3d { .. }))
}

pub fn export_html_from_source(
    source: &str,
    puzzle_path: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_host_mode(
        source,
        puzzle_path,
        StandaloneHostMode::Export,
    )
}

pub fn export_html_from_source_with_embedded_wasm(
    source: &str,
    puzzle_path: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, DiagnosticReport> {
    if player_runtime_module_js.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player.js content.",
        ));
    }
    if player_runtime_wasm_base64.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player_bg.wasm content.",
        ));
    }
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    export_html_from_document_with_runtime_wasm(
        &document,
        source,
        puzzle_path,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::EmbeddedBase64 {
            module_source: player_runtime_module_js,
            wasm_base64: player_runtime_wasm_base64,
        },
    )
}

pub fn export_html_from_document_with_embedded_wasm(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    player_runtime_module_js: &str,
    player_runtime_wasm_base64: &str,
) -> Result<String, DiagnosticReport> {
    if player_runtime_module_js.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player.js content.",
        ));
    }
    if player_runtime_wasm_base64.trim().is_empty() {
        return Err(DiagnosticReport::error(
            "Standalone HTML export requires puzzle_wasm_player_bg.wasm content.",
        ));
    }
    export_html_from_document_with_runtime_wasm(
        document,
        entry_source,
        puzzle_path,
        StandaloneHostMode::Export,
        StandaloneRuntimeWasm::EmbeddedBase64 {
            module_source: player_runtime_module_js,
            wasm_base64: player_runtime_wasm_base64,
        },
    )
}

pub fn export_editor_preview_html_from_source(
    source: &str,
    puzzle_path: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_host_mode(
        source,
        puzzle_path,
        StandaloneHostMode::EditorPreview,
    )
}

pub fn export_editor_preview_html_from_document(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
) -> Result<String, DiagnosticReport> {
    export_html_from_document_with_runtime_wasm(
        document,
        entry_source,
        puzzle_path,
        StandaloneHostMode::EditorPreview,
        StandaloneRuntimeWasm::HostDefault,
    )
}

pub fn export_editor_preview_build_from_document(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
) -> Result<String, DiagnosticReport> {
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)?;
    let state = EditorPreviewState::new(
        document.clone(),
        entry_source.to_string(),
        puzzle_path.to_string(),
        visual_images,
    )
    .map_err(DiagnosticReport::error)?;
    let html = export_html_with_runtime_wasm(
        &state,
        StandaloneHostMode::EditorPreview,
        StandaloneRuntimeWasm::HostDefault,
    );
    Ok(editor_preview_build_json(&html, &state))
}

fn export_html_from_source_with_host_mode(
    source: &str,
    puzzle_path: &str,
    host_mode: StandaloneHostMode,
) -> Result<String, DiagnosticReport> {
    export_html_from_source_with_runtime_wasm(
        source,
        puzzle_path,
        host_mode,
        StandaloneRuntimeWasm::HostDefault,
    )
}

fn export_html_from_source_with_runtime_wasm(
    source: &str,
    puzzle_path: &str,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, DiagnosticReport> {
    let document = puzzle_lang::parse_game_for_path(source, puzzle_path)?;
    export_html_from_document_with_runtime_wasm(
        &document,
        source,
        puzzle_path,
        host_mode,
        runtime_wasm,
    )
}

fn export_html_from_document_with_runtime_wasm(
    document: &puzzle_lang::LoadedDocument,
    entry_source: &str,
    puzzle_path: &str,
    host_mode: StandaloneHostMode,
    runtime_wasm: StandaloneRuntimeWasm<'_>,
) -> Result<String, DiagnosticReport> {
    if host_mode == StandaloneHostMode::Export {
        return export_bevy_document_html(document, puzzle_path, runtime_wasm)
            .map_err(DiagnosticReport::error);
    }
    let visual_images = load_visual_image_bundle_for_export(document, puzzle_path)?;
    let state = EditorPreviewState::new(
        document.clone(),
        entry_source.to_string(),
        puzzle_path.to_string(),
        visual_images,
    )
    .map_err(DiagnosticReport::error)?;
    Ok(export_html_with_runtime_wasm(
        &state,
        host_mode,
        runtime_wasm,
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_html_file(path: impl AsRef<Path>) -> Result<String, String> {
    let puzzle_path = resolve_game_entry(path).map_err(|error| error.to_string())?;
    let root = puzzle_path.parent().unwrap_or_else(|| Path::new("."));
    let workspace = puzzle_workspace::FileWorkspace::load(&puzzle_path, root)?;
    let document = workspace.compile().map_err(|error| error.to_string())?;
    export_bevy_document_html(
        &document,
        &puzzle_path.display().to_string(),
        StandaloneRuntimeWasm::HostDefault,
    )
}
