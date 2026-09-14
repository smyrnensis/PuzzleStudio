// Sound Library owns source-backed sound entry discovery and selection.
// It consumes the Rust source-entry projection; it never parses .puzzle text.
(() => {
  const state = {
    source: "",
    entries: [],
    selectedKey: "",
    draftKind: null,
    requestId: 0,
  };

  function entryKey(entry) {
    return `${entry.soundKind}:${entry.name}:${entry.start}`;
  }

  function entryOrigin(entry) {
    return entry?.params?.path ? "imported" : "generated";
  }

  function entryIcon(entry) {
    if (entry.soundKind === "music") return "music";
    return entryOrigin(entry) === "imported" ? "file" : "zap";
  }

  function selectedEntry() {
    return state.entries.find((entry) => entryKey(entry) === state.selectedKey) || null;
  }

  function updateInspector(entry = selectedEntry()) {
    const imported = entry && entryOrigin(entry) === "imported";
    soundsSfxPanel.hidden = !entry || imported || entry.soundKind !== "sfx";
    soundsMusicPanel.hidden = !entry || imported || entry.soundKind !== "music";
    soundsImportedPanel.hidden = !imported;
    if (!entry) {
      soundsInspectorKind.textContent = "Sound";
      soundsInspectorTitle.textContent = "Select a sound";
      soundsInspectorMeta.textContent = "Choose an item from the library to audition or edit it.";
      window.PuzzleSoundImported?.clear();
      return;
    }
    soundsInspectorKind.textContent = entry.soundKind === "music" ? "Music" : "SFX";
    soundsInspectorTitle.textContent = entry.name;
    soundsInspectorMeta.textContent = imported
      ? `Imported · ${String(entry.params.path).split(".").pop().toUpperCase()}`
      : "Generated from seed";
    if (imported) {
      window.PuzzleSoundImported?.load(entry);
    } else {
      window.PuzzleSoundImported?.clear();
    }
  }

  function render() {
    soundsLibraryList.replaceChildren();
    soundsLibraryEmpty.hidden = state.entries.length > 0;
    for (const entry of state.entries) {
      const key = entryKey(entry);
      const item = document.createElement("div");
      item.className = "sounds-library-item";
      item.setAttribute("role", "listitem");
      item.classList.toggle("is-selected", key === state.selectedKey);
      const selectButton = document.createElement("button");
      selectButton.type = "button";
      selectButton.className = "sounds-library-select";
      selectButton.setAttribute("aria-current", String(key === state.selectedKey));
      selectButton.title = entry.name;
      const icon = document.createElement("span");
      icon.className = "sounds-library-item-icon";
      icon.append(editorIconElement(entryIcon(entry)));
      const copy = document.createElement("span");
      const name = document.createElement("span");
      name.className = "sounds-library-item-name";
      name.textContent = entry.name;
      copy.append(name);
      selectButton.append(icon, copy);
      selectButton.addEventListener("click", () => { void select(entry); });
      const playButton = document.createElement("button");
      playButton.type = "button";
      playButton.className = "icon-button sounds-library-play";
      playButton.setAttribute("aria-label", `Play ${entry.name}`);
      playButton.title = `Play ${entry.name}`;
      playButton.append(editorIconElement("play"));
      playButton.disabled = entryOrigin(entry) === "imported" && !window.PuzzleSoundImported?.canPlay(entry);
      playButton.addEventListener("click", () => { void preview(entry); });
      item.append(selectButton, playButton);
      soundsLibraryList.append(item);
    }
    const entry = selectedEntry() || (state.draftKind ? {
      soundKind: state.draftKind,
      name: state.draftKind,
      params: {},
    } : null);
    updateInspector(entry);
  }

  async function select(entry) {
    state.selectedKey = entryKey(entry);
    state.draftKind = null;
    if (entryOrigin(entry) === "generated") {
      await loadSoundFromSourcePosition(entry.start, { silent: true });
    }
    render();
  }

  async function preview(entry) {
    try {
      if (entryOrigin(entry) === "imported") {
        await window.PuzzleSoundImported?.playEntry(entry);
        return;
      }
      await select(entry);
      if (entry.soundKind === "music") {
        await playSoundMusicFromStart();
      } else {
        await playSoundSfx();
      }
    } catch (error) {
      setStatus(`Could not play ${entry.name}: ${error?.message || error}`, "is-error");
    }
  }

  async function refresh() {
    const source = activeSoundEditSource();
    const requestId = ++state.requestId;
    if (source === state.source) {
      render();
      return;
    }
    if (!source) {
      state.source = source;
      state.entries = [];
      state.selectedKey = "";
      render();
      return;
    }
    try {
      const response = await window.PuzzleStudioRuntime.sourceEntryInfo(source);
      if (requestId !== state.requestId || activeSoundEditSource() !== source) return;
      state.source = source;
      state.entries = (response?.entries || [])
        .filter((entry) => entry?.kind === "sounds" && (entry.soundKind === "sfx" || entry.soundKind === "music"));
      if (!state.entries.some((entry) => entryKey(entry) === state.selectedKey)) {
        const target = sounds.editTarget;
        const matchingTarget = state.entries.find((entry) =>
          entry.soundKind === target?.kind && entry.name === target?.name && entry.start === target?.start
        );
        state.selectedKey = matchingTarget ? entryKey(matchingTarget) : "";
      }
      render();
    } catch (error) {
      if (requestId !== state.requestId) return;
      state.entries = [];
      state.selectedKey = "";
      render();
      setStatus(`Sound library unavailable: ${error?.message || error}`, "is-error");
    }
  }

  function beginGenerated(kind) {
    state.selectedKey = "";
    state.draftKind = kind;
    sounds.mode = kind;
    window.PuzzleSoundImported?.clear();
    if (kind === "music") {
      soundsMusicTitleInput.value = "music";
      soundsMusicSeedInput.value = soundRandomSeed();
      setSoundProgress(0);
    } else {
      soundsSfxTitleInput.value = "sfx";
      soundsSfxSeedInput.value = soundRandomSeed();
    }
    soundsAddMenu.hidden = true;
    soundsAddButton.setAttribute("aria-expanded", "false");
    updateInspector({ soundKind: kind, name: kind, params: {} });
    renderSoundsBuilder();
  }

  soundsAddButton.addEventListener("click", () => {
    const nextHidden = !soundsAddMenu.hidden;
    soundsAddMenu.hidden = nextHidden;
    soundsAddButton.setAttribute("aria-expanded", String(!nextHidden));
  });
  soundsAddMenuItems.forEach((item) => item.addEventListener("click", () => beginGenerated(item.dataset.soundsAdd)));
  document.addEventListener("pointerdown", (event) => {
    if (!soundsAddMenu.hidden && !soundsAddMenu.contains(event.target) && event.target !== soundsAddButton) {
      soundsAddMenu.hidden = true;
      soundsAddButton.setAttribute("aria-expanded", "false");
    }
  });

  window.PuzzleSoundLibrary = Object.freeze({ refresh, render, select, preview, selectedEntry });
})();
