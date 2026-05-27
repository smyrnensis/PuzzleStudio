(() => {
  if (window.PuzzleSoundGenerator || window.PuzzleSoundTools) {
    return;
  }

  async function loadSoundToolsScript() {
    if (window.PuzzleStudioHost?.soundTools) {
      return window.PuzzleStudioHost.soundTools();
    }
    const response = await fetch("/sound-tools.js");
    if (!response.ok) {
      throw new Error(response.statusText || `HTTP ${response.status}`);
    }
    return response.text();
  }

  loadSoundToolsScript()
    .then((script) => {
      const element = document.createElement("script");
      element.textContent = script;
      const current = document.currentScript;
      if (current?.parentNode) {
        current.parentNode.insertBefore(element, current.nextSibling);
      } else {
        document.head.append(element);
      }
    })
    .catch((error) => {
      console.error("Could not load PuzzleStudio sound tools:", error);
      window.dispatchEvent(new CustomEvent("PuzzleSoundToolsError", {
        detail: { message: error?.message || String(error) },
      }));
    });
})();
