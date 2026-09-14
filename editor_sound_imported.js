// Imported Sound owns file resolution and browser audition for one selected source asset.
(() => {
  let selected = null;
  let player = null;

  function selectedFile() {
    if (!selected?.params?.path) return null;
    const document = activeSoundEditDocument();
    if (!document) return null;
    return documentByPathForWorkspace(
      documentAssetCompilerPath(document, selected.params.path),
      document.workspaceRoot || workspaceRoot || "",
    );
  }

  function canPlay(entry) {
    if (!entry?.params?.path) return false;
    const previous = selected;
    selected = entry;
    const playable = Boolean(selectedFile()?.dataUrl);
    selected = previous;
    return playable;
  }

  function stop() {
    if (!player) return;
    player.pause();
    player.currentTime = 0;
    player = null;
  }

  function render() {
    const path = selected?.params?.path || "—";
    const file = selectedFile();
    soundsImportedPath.textContent = path;
    soundsImportedFormat.textContent = path === "—" ? "—" : String(path).split(".").pop().toUpperCase();
    soundsImportedStatus.textContent = file?.dataUrl ? "Ready to audition" : "File is missing from this workspace";
    soundsImportedPlayButton.disabled = !file?.dataUrl;
    soundsImportedStopButton.disabled = !player;
  }

  function load(entry) {
    if (selected?.start !== entry.start || selected?.name !== entry.name) stop();
    selected = entry;
    render();
  }

  function clear() {
    stop();
    selected = null;
  }

  async function play() {
    const file = selectedFile();
    if (!file?.dataUrl) {
      render();
      return;
    }
    stopSoundPlayback();
    stop();
    const audio = new Audio(file.dataUrl);
    audio.loop = selected.soundKind === "music";
    audio.volume = 1;
    audio.addEventListener("ended", () => {
      if (player === audio) {
        player = null;
        render();
      }
    }, { once: true });
    player = audio;
    try {
      await audio.play();
      render();
    } catch (error) {
      if (player === audio) player = null;
      render();
      setStatus(`Could not play ${selected.name}: ${error?.message || error}`, "is-error");
    }
  }

  async function playEntry(entry) {
    load(entry);
    await play();
  }

  function locate() {
    if (!selected) return;
    showWorkPane(SOURCE_WORK_PANE_ID, { focus: true });
    sourceEditor.setSelection(selected.start, selected.end);
    sourceEditor.scrollIntoView(selected.start);
    sourceEditor.focus();
  }

  soundsImportedPlayButton.addEventListener("click", () => { void play(); });
  soundsImportedStopButton.addEventListener("click", () => { stop(); render(); });
  soundsImportedLocateButton.addEventListener("click", locate);

  window.PuzzleSoundImported = Object.freeze({ load, clear, stop, render, canPlay, playEntry });
})();
