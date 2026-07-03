function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

async function downloadHtml() {
  if (!latestHtml) {
    return;
  }
  const filename = htmlDownloadFileName();
  if (window.PuzzleStudioHost?.mode?.() === "tauri" && window.PuzzleStudioHost?.exportHtml) {
    try {
      const result = await window.PuzzleStudioHost.exportHtml({
        html: latestHtml,
        filename,
      });
      if (result?.canceled) {
        setEditorStatus("Export canceled");
        return;
      }
      if (result?.ok) {
        setEditorStatus(`Exported ${fileName(result.path) || filename}`);
        return;
      }
    } catch (error) {
      console.error(error);
      setEditorStatus(`Export failed: ${error.message || error}`, "is-error");
      return;
    }
  }
  const blob = new Blob([latestHtml], { type: "text/html;charset=utf-8" });
  downloadBlob(blob, filename);
}

function htmlDownloadFileName() {
  const previewDocument = activePreviewDocument();
  const path = previewDocument?.puzzlePath || previewDocument?.name || "";
  const sourceName = path ? fileName(path) : "";
  const baseName = sourceName
    .replace(/\.puzzle$/i, "")
    .replace(/\.html?$/i, "") || "game";
  return `${sanitizeFileName(baseName) || "game"}.html`;
}

function downloadPuzzle() {
  persistCurrentDocument();
  const selected = selectedTreeNode();
  if (selected?.kind === "folder") {
    downloadFolder(selected);
    return;
  }
  downloadFile(selected?.kind === "file" ? selected : documents[currentDocumentIndex]);
}

function downloadFile(document) {
  if (!document) {
    return;
  }
  const blob = document.encoding === "data_url"
    ? new Blob([bytesForDocument(document)], { type: document.mimeType || "application/octet-stream" })
    : new Blob([document.id === activeDocument()?.id && isTextDocument(document) ? sourceEditorDocumentValue() : document.source || ""], { type: `${document.mimeType || "text/plain"};charset=utf-8` });
  const name = document.name || fileName(document.puzzlePath);
  downloadBlob(blob, name || "file");
}

function downloadFolder(folder) {
  const entries = folderZipEntries(folder);
  if (!entries.length) {
    setEditorStatus("Folder is empty", "is-error");
    return;
  }
  const zip = zipBlob(entries);
  downloadBlob(zip, `${sanitizeFileName(folder.name || "folder") || "folder"}.zip`);
}

function folderZipEntries(folder) {
  const entries = [];
  const rootName = sanitizeFileName(folder.name || "folder") || "folder";
  collectFolderZipEntries(folder, rootName, entries);
  return entries;
}

function collectFolderZipEntries(node, parentPath, entries) {
  for (const child of node.children || []) {
    const childName = sanitizeZipPathSegment(child.name || fileName(child.puzzlePath));
    const childPath = joinPath(parentPath, childName);
    if (child.kind === "folder") {
      collectFolderZipEntries(child, childPath, entries);
      continue;
    }
    entries.push({
      path: childPath,
      bytes: bytesForDocument(child),
    });
  }
}

function bytesForDocument(document) {
  if (document.encoding === "data_url") {
    return dataUrlBytes(document.dataUrl || "");
  }
  return new TextEncoder().encode(document.source || "");
}

function dataUrlBytes(dataUrl) {
  const match = String(dataUrl).match(/^data:([^,]*),(.*)$/);
  if (!match) {
    return new Uint8Array();
  }
  const meta = match[1] || "";
  const data = match[2] || "";
  if (meta.includes(";base64")) {
    const raw = atob(data);
    const bytes = new Uint8Array(raw.length);
    for (let index = 0; index < raw.length; index += 1) {
      bytes[index] = raw.charCodeAt(index);
    }
    return bytes;
  }
  return new TextEncoder().encode(decodeURIComponent(data));
}

function sanitizeZipPathSegment(name) {
  return sanitizeFileName(name).replace(/^\.|\.$/g, "") || "item";
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  window.setTimeout(() => {
    URL.revokeObjectURL(url);
    link.remove();
  }, 0);
}

function zipBlob(entries) {
  const encoder = new TextEncoder();
  const parts = [];
  const centralParts = [];
  let offset = 0;
  const now = new Date();
  const dosTime = ((now.getHours() & 31) << 11) | ((now.getMinutes() & 63) << 5) | ((Math.floor(now.getSeconds() / 2)) & 31);
  const dosDate = (((now.getFullYear() - 1980) & 127) << 9) | (((now.getMonth() + 1) & 15) << 5) | (now.getDate() & 31);

  for (const entry of entries) {
    const nameBytes = encoder.encode(normalizePath(entry.path));
    const dataBytes = entry.bytes || new Uint8Array();
    const crc = crc32(dataBytes);
    const localHeader = new Uint8Array(30 + nameBytes.length);
    const localView = new DataView(localHeader.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, 20, true);
    localView.setUint16(6, 0x0800, true);
    localView.setUint16(8, 0, true);
    localView.setUint16(10, dosTime, true);
    localView.setUint16(12, dosDate, true);
    localView.setUint32(14, crc, true);
    localView.setUint32(18, dataBytes.length, true);
    localView.setUint32(22, dataBytes.length, true);
    localView.setUint16(26, nameBytes.length, true);
    localHeader.set(nameBytes, 30);
    parts.push(localHeader, dataBytes);

    const centralHeader = new Uint8Array(46 + nameBytes.length);
    const centralView = new DataView(centralHeader.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, 20, true);
    centralView.setUint16(6, 20, true);
    centralView.setUint16(8, 0x0800, true);
    centralView.setUint16(10, 0, true);
    centralView.setUint16(12, dosTime, true);
    centralView.setUint16(14, dosDate, true);
    centralView.setUint32(16, crc, true);
    centralView.setUint32(20, dataBytes.length, true);
    centralView.setUint32(24, dataBytes.length, true);
    centralView.setUint16(28, nameBytes.length, true);
    centralView.setUint32(42, offset, true);
    centralHeader.set(nameBytes, 46);
    centralParts.push(centralHeader);
    offset += localHeader.length + dataBytes.length;
  }

  const centralSize = centralParts.reduce((sum, part) => sum + part.length, 0);
  const end = new Uint8Array(22);
  const endView = new DataView(end.buffer);
  endView.setUint32(0, 0x06054b50, true);
  endView.setUint16(8, entries.length, true);
  endView.setUint16(10, entries.length, true);
  endView.setUint32(12, centralSize, true);
  endView.setUint32(16, offset, true);

  return new Blob([...parts, ...centralParts, end], { type: "application/zip" });
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

async function importFiles(fileList) {
  return importFilesIntoFolder(fileList, activeFolder());
}

async function importFilesIntoFolder(fileList, targetFolder) {
  const files = Array.from(fileList || []);
  if (!files.length) {
    return;
  }
  persistCurrentDocument();
  selectedFolderId = targetFolder?.kind === "folder" && targetFolder !== fileTree ? targetFolder.id : "";
  selectedTreeId = selectedFolderId || selectedTreeId;
  let firstImportedPuzzleId = "";
  let importedCount = 0;
  for (const file of files) {
    if (isZipFileName(file.name, file.type)) {
      const result = await importZipFile(file, targetFolder);
      importedCount += result.count;
      if (!firstImportedPuzzleId && result.firstImportedPuzzleId) {
        firstImportedPuzzleId = result.firstImportedPuzzleId;
      }
      continue;
    }

    let imported = null;
    if (isTextFileName(file.name, file.type)) {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "text",
        source: await file.text(),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(file.webkitRelativePath || file.name, {
        encoding: "data_url",
        dataUrl: await readFileAsDataUrl(file),
        mimeType: file.type || mimeTypeForPath(file.name),
      }, targetFolder);
    }
    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      importedCount += 1;
    }
  }
  if (!importedCount) {
    setEditorStatus("No importable files", "is-error");
    return;
  }
  if (firstImportedPuzzleId) {
    activeFileId = firstImportedPuzzleId;
  }
  syncDocumentsFromTree();
  currentDocumentIndex = activeDocumentIndex();
  renderDocumentSelect();
  loadEmbeddedDocument(currentDocumentIndex);
  saveDocumentStore(false);
  const folderName = targetFolder && targetFolder !== fileTree ? folderPath(targetFolder) || targetFolder.name : "Files";
  setEditorStatus(`Imported to ${folderName}`, "is-ok");
  if (!editorSeed) {
    try {
      await renderPreview();
    } catch (error) {
      console.error(error);
      const message = importErrorMessage(error);
      setEditorStatus(`Imported; preview failed: ${message}`, "is-error");
      setStatus(`Preview failed: ${message}`, "is-error");
    }
  }
}

function importErrorMessage(error) {
  return String(error?.message || error || "unknown error");
}

async function importZipFile(file, targetFolder) {
  const entries = await unzipFileEntries(file);
  let firstImportedPuzzleId = "";
  let count = 0;
  for (const entry of entries) {
    const entryPath = safeZipEntryPath(entry.path);
    if (!entryPath) {
      continue;
    }

    let imported = null;
    if (isTextFileName(entryPath, entry.mimeType)) {
      imported = importWorkspaceFile(entryPath, {
        encoding: "text",
        source: new TextDecoder().decode(entry.bytes),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    } else {
      imported = importWorkspaceFile(entryPath, {
        encoding: "data_url",
        dataUrl: bytesToDataUrl(entry.bytes, entry.mimeType || mimeTypeForPath(entryPath)),
        mimeType: entry.mimeType || mimeTypeForPath(entryPath),
      }, targetFolder);
    }

    if (!firstImportedPuzzleId && isPuzzleDocument(imported)) {
      firstImportedPuzzleId = imported.id;
    }
    if (imported) {
      count += 1;
    }
  }
  return { count, firstImportedPuzzleId };
}

async function unzipFileEntries(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const endOffset = findZipEndOffset(view);
  if (endOffset < 0) {
    throw new Error("Invalid zip file");
  }

  const entryCount = view.getUint16(endOffset + 10, true);
  let centralOffset = view.getUint32(endOffset + 16, true);
  const entries = [];

  for (let index = 0; index < entryCount; index += 1) {
    if (centralOffset + 46 > bytes.length || view.getUint32(centralOffset, true) !== 0x02014b50) {
      throw new Error("Invalid zip directory");
    }
    const flags = view.getUint16(centralOffset + 8, true);
    const method = view.getUint16(centralOffset + 10, true);
    const compressedSize = view.getUint32(centralOffset + 20, true);
    const nameLength = view.getUint16(centralOffset + 28, true);
    const extraLength = view.getUint16(centralOffset + 30, true);
    const commentLength = view.getUint16(centralOffset + 32, true);
    const localOffset = view.getUint32(centralOffset + 42, true);
    const nameStart = centralOffset + 46;
    const nameBytes = bytes.slice(nameStart, nameStart + nameLength);
    const path = decodeZipName(nameBytes, flags);
    centralOffset = nameStart + nameLength + extraLength + commentLength;

    if (!path || path.endsWith("/")) {
      continue;
    }
    if (localOffset + 30 > bytes.length || view.getUint32(localOffset, true) !== 0x04034b50) {
      throw new Error("Invalid zip entry");
    }

    const localNameLength = view.getUint16(localOffset + 26, true);
    const localExtraLength = view.getUint16(localOffset + 28, true);
    const dataStart = localOffset + 30 + localNameLength + localExtraLength;
    const compressed = bytes.slice(dataStart, dataStart + compressedSize);
    const entryBytes = method === 0
      ? compressed
      : method === 8
        ? await inflateZipDeflate(compressed)
        : null;
    if (!entryBytes) {
      throw new Error(`Unsupported zip compression for ${path}`);
    }
    entries.push({
      path,
      bytes: entryBytes,
      mimeType: mimeTypeForPath(path),
    });
  }

  return entries;
}

function findZipEndOffset(view) {
  const minOffset = Math.max(0, view.byteLength - 0xffff - 22);
  for (let offset = view.byteLength - 22; offset >= minOffset; offset -= 1) {
    if (view.getUint32(offset, true) === 0x06054b50) {
      return offset;
    }
  }
  return -1;
}

function decodeZipName(bytes, flags) {
  const decoder = flags & 0x0800 ? new TextDecoder("utf-8") : new TextDecoder();
  return decoder.decode(bytes);
}

async function inflateZipDeflate(bytes) {
  if (typeof DecompressionStream !== "function") {
    throw new Error("Zip deflate is not supported in this browser");
  }
  try {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  } catch (error) {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate"));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }
}

function safeZipEntryPath(path) {
  const normalized = normalizePath(path);
  if (!normalized || normalized.startsWith("/") || /^[A-Za-z]:\//.test(normalized)) {
    return "";
  }
  const parts = normalized.split("/").filter(Boolean);
  if (!parts.length || parts.includes("..") || parts[0] === "__MACOSX" || parts.at(-1) === ".DS_Store") {
    return "";
  }
  return parts.map(sanitizeZipPathSegment).join("/");
}

function bytesToDataUrl(bytes, mimeType = "application/octet-stream") {
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.slice(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return `data:${mimeType};base64,${btoa(binary)}`;
}

function readFileAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => resolve(String(reader.result || "")));
    reader.addEventListener("error", () => reject(reader.error || new Error("File read failed")));
    reader.readAsDataURL(file);
  });
}

function importWorkspaceFile(fileNameValue, fileData, targetFolder = activeFolder()) {
  const current = documents[currentDocumentIndex] || {};
  const parts = String(fileNameValue || "imported.file").split(/[\\/]/).filter(Boolean);
  const name = sanitizeFileName(parts.pop() || "imported.file");
  let folder = targetFolder || fileTree;
  for (const part of parts) {
    folder = childFolder(folder, part, workspaceRootForFolder(folder));
  }
  const file = makeFile(uniqueChildName(folder, name), fileData.source || "", {
    parentPath: folderPath(folder),
    workspaceRoot: workspaceRootForFolder(folder),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
  });
  file.encoding = fileData.encoding || "text";
  file.mimeType = fileData.mimeType || mimeTypeForPath(name);
  file.source = fileData.source || "";
  file.dataUrl = fileData.dataUrl || "";
  if (!isPuzzleDocument(file)) {
    file.previewHtml = "";
    file.gameCss = "";
  }
  folder.children.push(file);
  selectedFolderId = folder.id;
  activeFileId = file.id;
  return file;
}

function setPuzzleScriptImportStatus(message, tone = "") {
  if (!psImportStatus) {
    return;
  }
  psImportStatus.textContent = message;
  psImportStatus.className = "ps-import-status tool-feedback-bar";
  psImportStatus.classList.toggle("is-ok", tone === "is-ok");
  psImportStatus.classList.toggle("is-error", tone === "is-error");
  setPaneStatus("psimport", message, tone);
}

function resetPuzzleScriptImportConversion() {
  if (psImportOutput) {
    psImportOutput.value = "";
  }
  if (psImportCopyButton) {
    psImportCopyButton.disabled = true;
  }
  if (psImportAddFileButton) {
    psImportAddFileButton.disabled = true;
  }
  setPuzzleScriptImportStatus("", "");
}

async function convertPuzzleScriptImport() {
  const source = psImportSourceInput?.value || "";
  if (!source.trim()) {
    if (psImportOutput) {
      psImportOutput.value = "";
    }
    if (psImportCopyButton) {
      psImportCopyButton.disabled = true;
    }
    if (psImportAddFileButton) {
      psImportAddFileButton.disabled = true;
    }
    setPuzzleScriptImportStatus("", "");
    return "";
  }
  setPuzzleScriptImportStatus("Converting", "");
  const compiler = await loadWasmCompiler();
  if (typeof compiler.translate_puzzlescript !== "function") {
    throw new Error("PuzzleScript import is unavailable in this editor build.");
  }
  const canonical = compiler.translate_puzzlescript(source);
  if (psImportOutput) {
    psImportOutput.value = canonical;
  }
  if (psImportCopyButton) {
    psImportCopyButton.disabled = false;
  }
  if (psImportAddFileButton) {
    psImportAddFileButton.disabled = false;
  }
  setPuzzleScriptImportStatus("Converted", "is-ok");
  return canonical;
}

function puzzleScriptImportTitle(source, canonical) {
  const explicitTitle = String(source || "")
    .split("\n")
    .map((line) => line.split("//", 1)[0].trim())
    .find((line) => /^title(?:\s+|$)/i.test(line))
    ?.replace(/^title\s*/i, "")
    .trim();
  if (explicitTitle) {
    return explicitTitle;
  }
  const canonicalTitle = String(canonical || "")
    .split("\n")
    .find((line) => /^title(?:\s+|$)/.test(line.trim()))
    ?.trim()
    .replace(/^title\s*/, "")
    .trim();
  if (canonicalTitle) {
    try {
      return JSON.parse(canonicalTitle);
    } catch {
      return canonicalTitle.replace(/^"|"$/g, "");
    }
  }
  return "PuzzleScript import";
}

async function copyPuzzleScriptImportOutput() {
  const output = psImportOutput?.value || "";
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Generate import first", "is-error");
    return;
  }
  try {
    psImportCopyButton?.focus({ preventScroll: true });
    await copyTextToClipboard(output);
    setPuzzleScriptImportStatus("Copied", "is-ok");
  } catch (error) {
    setPuzzleScriptImportStatus("Copy failed", "is-error");
    setStatus(`Could not copy PuzzleScript import: ${error?.message || error}`, "is-error");
  }
}

async function addPuzzleScriptImportFile() {
  const output = psImportOutput?.value || "";
  if (!output.trim()) {
    setPuzzleScriptImportStatus("Generate import first", "is-error");
    return;
  }

  persistCurrentDocument();
  const targetFolder = activeFolder();
  targetFolder.expanded = true;
  const title = puzzleScriptImportTitle(psImportSourceInput?.value || "", output);
  const fileNameValue = uniqueChildName(targetFolder, ensurePuzzleExtension(title || "PuzzleScript import"));
  const parentPath = folderPath(targetFolder);
  const editorPath = joinPath(parentPath, fileNameValue);

  if (!editorSeed && typeof window.PuzzleStudioHost.createSourceFile === "function") {
    await window.PuzzleStudioHost.createSourceFile({
      source: output,
      puzzlePath: hostPathForEditorPath(editorPath, workspaceRootForFolder(targetFolder)),
      workspaceRoot: workspaceRootForFolder(targetFolder),
    });
  }

  const current = documents[currentDocumentIndex] || {};
  const file = makeFile(fileNameValue, output, {
    parentPath,
    workspaceRoot: workspaceRootForFolder(targetFolder),
    gameCss: current.gameCss || editorSeed?.gameCss || "",
  });
  targetFolder.children.push(file);
  activeFileId = file.id;
  selectedTreeId = file.id;
  selectedFolderId = targetFolder === fileTree ? "" : targetFolder.id;
  syncDocumentsFromTree();
  loadEmbeddedDocument(activeDocumentIndex());
  saveDocumentStore(false);
  setPuzzleScriptImportStatus(`Added ${fileNameValue}`, "is-ok");
}
