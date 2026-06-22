let spriteActionClearTimer = 0;
let spriteBucketActive = false;
const spriteColorEditSessions = {
  sprite: null,
  sprite3d: null,
};
const SOLID_SPRITE_EDITOR_SIZE = 5;
const SPRITE_EDITOR_MAX_SIZE = 64;

function resetSpriteBuilder(size = sprite.size) {
  sprite.size = clampSpriteSize(size);
  sprite.cells = Array.from({ length: sprite.size * sprite.size }, () => null);
  sprite.paletteBind = null;
  sprite.shapeBind = null;
  sprite.solidSource = false;
  if (!Number.isInteger(sprite.selectedColorIndex) || !sprite.palette[sprite.selectedColorIndex]) {
    sprite.selectedColorIndex = 0;
  }
  renderSpriteBuilder();
}

function clampSpriteSize(value) {
  const size = Math.trunc(Number(value) || 5);
  return Math.max(1, Math.min(SPRITE_EDITOR_MAX_SIZE, size));
}

function renderSpriteBuilder() {
  if (!spriteBoard || !spritePalette) {
    return;
  }
  renderSpriteControls();
  renderSpritePalette();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
}

function renderSpriteControls() {
  spriteSizeInput.value = String(sprite.size);
  syncSpriteBucketButton();
  renderSpriteShapeBindRow(spriteShapeField);
  renderSpriteScaleControl({
    size: sprite.size,
    maxSize: SPRITE_EDITOR_MAX_SIZE,
    scaleInput: spriteScaleInput,
    scaleUpButton: spriteScaleUpButton,
    scaleDownButton: spriteScaleDownButton,
    canScaleDown: canScaleDownSprite,
    noun: "sprite",
  });
}

function syncSpriteBucketButton() {
  if (!spriteFillButton) {
    return;
  }
  spriteFillButton.classList.toggle("is-active", spriteBucketActive);
  spriteFillButton.setAttribute("aria-pressed", String(spriteBucketActive));
  spriteFillButton.setAttribute("aria-label", "Bucket fill");
  spriteFillButton.title = spriteBucketActive ? "Bucket active" : "Bucket fill";
}

function toggleSpriteBucketMode() {
  spriteBucketActive = !spriteBucketActive;
  syncSpriteBucketButton();
  setSpriteActionStatus(
    spriteBucketActive ? "Bucket: click a connected area" : "Brush: paint individual cells",
    "is-ok",
  );
}

function beginSpriteColorEditHistory(kind) {
  if (!spriteColorEditSessions[kind]) {
    spriteColorEditSessions[kind] = visualEditSnapshot(kind);
  }
}

function commitSpriteColorEditHistory(kind) {
  const before = spriteColorEditSessions[kind];
  spriteColorEditSessions[kind] = null;
  if (!before) {
    return false;
  }
  return pushVisualEditUndoSnapshot(kind, before);
}

function discardSpriteColorEditHistory(kind) {
  spriteColorEditSessions[kind] = null;
}

function renderSpriteColorAdjuster({ color, ariaLabel, onChange, onPickError = null }) {
  const editor = document.createElement("span");
  editor.className = "sprite-color-adjuster";
  const colorField = document.createElement("span");
  colorField.className = "sprite-color-field";
  colorField.setAttribute("role", "slider");
  colorField.tabIndex = 0;
  colorField.setAttribute("aria-label", ariaLabel);
  const colorThumb = document.createElement("span");
  colorThumb.className = "sprite-color-field-thumb";
  colorField.append(colorThumb);

  const hueInput = document.createElement("input");
  hueInput.type = "range";
  hueInput.className = "sprite-hue-input";
  hueInput.min = "0";
  hueInput.max = "360";
  hueInput.setAttribute("aria-label", `${ariaLabel} hue`);

  const valueRow = document.createElement("span");
  valueRow.className = "sprite-color-value-row";
  const eyedropperButton = document.createElement("button");
  eyedropperButton.type = "button";
  eyedropperButton.className = "sprite-eyedropper-button";
  eyedropperButton.title = "Pick color from screen";
  eyedropperButton.setAttribute("aria-label", eyedropperButton.title);
  eyedropperButton.innerHTML = spriteEyedropperIconSvg();
  const previewSwatch = document.createElement("span");
  previewSwatch.className = "sprite-color-preview-swatch sprite-color-swatch";
  previewSwatch.setAttribute("aria-hidden", "true");

  const alphaWrap = document.createElement("label");
  alphaWrap.className = "sprite-current-alpha-control";
  const alphaInput = document.createElement("input");
  alphaInput.type = "range";
  alphaInput.min = "0";
  alphaInput.max = "100";
  alphaInput.setAttribute("aria-label", `${ariaLabel} alpha`);
  const numberWrap = document.createElement("span");
  numberWrap.className = "sprite-color-numbers";
  const numberInputs = {};
  for (const [key, label, max] of [
    ["r", "R", 255],
    ["g", "G", 255],
    ["b", "B", 255],
    ["a", "A", 100],
  ]) {
    const wrap = document.createElement("label");
    wrap.className = "sprite-color-number";
    const text = document.createElement("span");
    text.textContent = label;
    const input = document.createElement("input");
    input.type = "number";
    input.min = "0";
    input.max = String(max);
    input.inputMode = "numeric";
    input.setAttribute("aria-label", `${ariaLabel} ${label}`);
    numberInputs[key] = input;
    wrap.append(text, input);
    numberWrap.append(wrap);
  }
  valueRow.append(eyedropperButton, previewSwatch, numberWrap);

  const hsv = spriteHsvFromColor(color);
  const clampNumber = (value, min, max) => Math.max(min, Math.min(max, Math.round(Number(value) || 0)));
  const syncUi = (nextColor) => {
    const normalized = normalizeSpriteColor(nextColor);
    const rgb = spriteRgbComponents(normalized);
    hueInput.value = String(Math.round(hsv.h));
    alphaInput.value = String(spriteAlphaPercent(normalized));
    numberInputs.r.value = String(rgb.r);
    numberInputs.g.value = String(rgb.g);
    numberInputs.b.value = String(rgb.b);
    numberInputs.a.value = String(spriteAlphaPercent(normalized));
    editor.style.setProperty("--sprite-alpha-color", spriteRgbHex(normalized));
    previewSwatch.style.setProperty("--sprite-swatch-color", normalized);
    colorField.style.setProperty("--sprite-picker-hue", `hsl(${hsv.h} 100% 50%)`);
    colorThumb.style.left = `${hsv.s * 100}%`;
    colorThumb.style.top = `${(1 - hsv.v) * 100}%`;
  };
  const sync = (nextColor = color) => {
    const normalized = normalizeSpriteColor(nextColor);
    const nextHsv = spriteHsvFromColor(normalized);
    hsv.h = nextHsv.h;
    hsv.s = nextHsv.s;
    hsv.v = nextHsv.v;
    syncUi(normalized);
  };
  const emit = () => {
    const next = spriteColorWithAlpha(spriteRgbFromHsv(hsv), alphaInput.value);
    syncUi(next);
    onChange(next);
  };
  const makeHueEditVisible = () => {
    let changed = false;
    if (hsv.s <= 0) {
      hsv.s = 1;
      changed = true;
    }
    if (hsv.v <= 0) {
      hsv.v = 1;
      changed = true;
    }
    return changed;
  };
  const syncHueFromInput = () => {
    hsv.h = Number(hueInput.value) || 0;
  };
  const activateHueInput = () => {
    syncHueFromInput();
    if (makeHueEditVisible()) {
      emit();
    }
  };
  const emitRgb = () => {
    const r = clampNumber(numberInputs.r.value, 0, 255).toString(16).padStart(2, "0");
    const g = clampNumber(numberInputs.g.value, 0, 255).toString(16).padStart(2, "0");
    const b = clampNumber(numberInputs.b.value, 0, 255).toString(16).padStart(2, "0");
    const next = spriteColorWithAlpha(`#${r}${g}${b}`, numberInputs.a.value);
    sync(next);
    onChange(next);
  };
  const setFromField = (event) => {
    const rect = colorField.getBoundingClientRect();
    hsv.s = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    hsv.v = 1 - Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height));
    emit();
  };

  colorField.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    colorField.setPointerCapture(event.pointerId);
    setFromField(event);
  });
  colorField.addEventListener("pointermove", (event) => {
    if (!colorField.hasPointerCapture(event.pointerId)) {
      return;
    }
    setFromField(event);
  });
  colorField.addEventListener("pointerup", (event) => {
    if (colorField.hasPointerCapture(event.pointerId)) {
      colorField.releasePointerCapture(event.pointerId);
    }
  });
  colorField.addEventListener("pointercancel", (event) => {
    if (colorField.hasPointerCapture(event.pointerId)) {
      colorField.releasePointerCapture(event.pointerId);
    }
  });
  hueInput.addEventListener("input", () => {
    syncHueFromInput();
    makeHueEditVisible();
    emit();
  });
  hueInput.addEventListener("pointerdown", () => {
    window.requestAnimationFrame(activateHueInput);
  });
  numberInputs.r.addEventListener("input", emitRgb);
  numberInputs.g.addEventListener("input", emitRgb);
  numberInputs.b.addEventListener("input", emitRgb);
  numberInputs.a.addEventListener("input", () => {
    alphaInput.value = String(clampNumber(numberInputs.a.value, 0, 100));
    emitRgb();
  });
  alphaInput.addEventListener("input", emit);
  alphaInput.addEventListener("change", emit);
  eyedropperButton.addEventListener("click", async () => {
    try {
      const result = await window.PuzzleStudioHost?.pickScreenColor?.();
      if (!result || result.canceled) {
        return;
      }
      const pickedColor = parseSpriteHexColor(result.color || result.sRGBHex);
      if (!pickedColor) {
        throw new Error("Screen color picker returned an invalid color.");
      }
      const picked = spriteColorWithAlpha(pickedColor, alphaInput.value);
      sync(picked);
      onChange(picked);
    } catch (error) {
      onPickError?.(error?.message || "Screen color picker is unavailable.", "is-error");
    }
  });
  alphaWrap.append(alphaInput);
  editor.append(colorField, hueInput, alphaWrap, valueRow);
  editor.syncColor = sync;
  sync(color);
  return editor;
}

function spriteEyedropperIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m12 9-8.414 8.414A2 2 0 0 0 3 18.828v1.344a2 2 0 0 1-.586 1.414A2 2 0 0 1 3.828 21h1.344a2 2 0 0 0 1.414-.586L15 12"></path>
      <path d="m18 9 .4.4a1 1 0 1 1-3 3l-3.8-3.8a1 1 0 1 1 3-3l.4.4 3.4-3.4a1 1 0 1 1 3 3z"></path>
      <path d="m2 22 .414-.414"></path>
    </svg>
  `;
}

function renderSpritePalette() {
  spritePalette.replaceChildren();
  const selectedIsTransparent = sprite.selectedColorIndex === null;
  if (selectedIsTransparent || validSpriteColorIndex(sprite.selectedColorIndex)) {
    const currentWrap = document.createElement("span");
    currentWrap.className = "sprite-current-color-wrap";
    const selected = selectedIsTransparent ? { color: "#00000000" } : sprite.palette[sprite.selectedColorIndex];
    const selectedBind = selectedIsTransparent ? { available: false, linked: false, label: "" } : spritePaletteEntryBindInfo(selected);
    const currentButton = document.createElement("button");
    currentButton.type = "button";
    currentButton.className = "sprite-current-color-button";
    currentButton.classList.toggle("is-transparent", selectedIsTransparent);
    currentButton.classList.toggle("is-bound", selectedBind.available && selectedBind.linked);
    currentButton.classList.toggle("is-unlinked", selectedBind.available && !selectedBind.linked);
    currentButton.style.setProperty("--sprite-current-color", normalizeSpriteColor(selected.color));
    currentButton.title = selectedIsTransparent
      ? "Transparent eraser cannot be edited"
      : selectedBind.available ? `Pick selected color (${selectedBind.label})` : "Pick selected color";
    currentButton.setAttribute(
      "aria-label",
      selectedIsTransparent ? "Selected transparent eraser color #00000000, not editable" : `Pick selected color ${selected.color}`,
    );
    currentButton.setAttribute("aria-disabled", String(selectedIsTransparent));
    currentButton.setAttribute("aria-expanded", String(!selectedIsTransparent && sprite.editPaletteOpen));
    currentButton.innerHTML = `
      <span class="sprite-current-color-swatch" aria-hidden="true"></span>
    `;
    if (selectedIsTransparent) {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-transparent-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"></path>
            <path d="M22 21H7"></path>
            <path d="m5 11 9 9"></path>
          </svg>
        </span>
      `);
    } else {
      currentButton.insertAdjacentHTML("beforeend", `
        <span class="sprite-current-edit-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="M12 20h9"></path>
            <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"></path>
          </svg>
        </span>
      `);
    }
    const currentHexInput = document.createElement("input");
    currentHexInput.type = "text";
    currentHexInput.className = "sprite-current-value-input sprite-current-hex-input";
    currentHexInput.value = selectedIsTransparent
      ? "#00000000"
      : normalizeSpriteColor(selected.color);
    currentHexInput.placeholder = "#rrggbbaa";
    currentHexInput.spellcheck = false;
    currentHexInput.autocomplete = "off";
    currentHexInput.readOnly = selectedIsTransparent;
    currentHexInput.setAttribute(
      "aria-label",
      selectedIsTransparent
        ? "Transparent color code"
        : "Selected color code",
    );
    const currentTagButton = selectedIsTransparent ? null : renderSpriteCurrentColorTagButton(selected);
    const currentTagUnlinkButton = !selectedIsTransparent && selectedBind.linked && selectedBind.name
      ? renderSpriteCurrentColorUnlinkButton(sprite.selectedColorIndex, selectedBind)
      : null;
    const colorNames = selectedIsTransparent ? [] : spriteColorAssetNames();
    const applyCurrentColorValue = (color) => {
      beginSpriteColorEditHistory("sprite");
      const normalized = normalizeSpriteColor(color);
      clearSpriteActionError();
      selected.color = normalized;
      updateSpriteBoundColorDefinition(selected, normalized);
      currentButton.style.setProperty("--sprite-current-color", normalized);
      currentButton.setAttribute("aria-label", `Pick selected color ${normalized}`);
      currentHexInput.value = normalized;
      renderSpriteColorSurfaces();
    };
    let pendingEditMenu = null;
    const applyCurrentHex = (options = {}) => {
      const parsed = parseSpriteHexColor(currentHexInput.value);
      if (!parsed) {
        if (options.reportError) {
          setSpriteActionStatus("Use #rrggbb or #rrggbbaa", "is-error");
        }
        return;
      }
      applyCurrentColorValue(parsed);
      if (options.commitHistory) {
        commitSpriteColorEditHistory("sprite");
      }
    };

    if (!selectedIsTransparent) {
      currentButton.addEventListener("click", () => {
        const opening = !sprite.editPaletteOpen;
        if (!opening) {
          commitSpriteColorEditHistory("sprite");
        }
        sprite.editPaletteOpen = opening;
        sprite.addPaletteOpen = false;
        sprite.addDraftColorIndex = null;
        sprite.customColorOpen = opening;
        renderSpritePalette();
      });
      currentHexInput.addEventListener("input", () => {
        applyCurrentHex();
      });
      currentHexInput.addEventListener("change", () => applyCurrentHex({ reportError: true, commitHistory: true }));
      currentHexInput.addEventListener("keydown", (event) => {
        event.stopPropagation();
        if (event.key !== "Enter") {
          return;
        }
        event.preventDefault();
        applyCurrentHex({ reportError: true, commitHistory: true });
      });
    }
    currentWrap.append(currentButton);
    currentWrap.append(currentHexInput);
    if (currentTagButton) {
      currentWrap.append(currentTagButton);
    }
    if (currentTagUnlinkButton) {
      currentWrap.append(currentTagUnlinkButton);
    }
    if (!selectedIsTransparent && sprite.colorTagPickerOpen) {
      const tagPicker = renderSpriteAssetNamePicker({
        className: "sprite-color-tag-picker",
        names: colorNames,
        value: selectedBind.name || defaultSpriteAssetName("color", sprite.selectedColorIndex),
        placeholder: "color_name",
        ariaLabel: "Color tag name",
        emptyText: "No named colors yet",
        onCommit: (name) => {
          const wasOpen = sprite.colorTagPickerOpen;
          sprite.colorTagPickerOpen = false;
          const ok = applyCurrentColorName(sprite.selectedColorIndex, name, { reportError: true });
          if (!ok) {
            sprite.colorTagPickerOpen = wasOpen;
          }
          return ok;
        },
        onCancel: () => {
          sprite.colorTagPickerOpen = false;
          renderSpritePalette();
        },
      });
      currentWrap.append(tagPicker);
      requestAnimationFrame(() => {
        focusSpriteTagPickerInput(tagPicker);
      });
    }
    if (!selectedIsTransparent && sprite.editPaletteOpen) {
      const editorPanel = document.createElement("span");
      editorPanel.className = "sprite-current-editor-panel";
      const editMenu = renderSpriteColorMenu({
        mode: "edit",
        customValue: selected.color,
        customOnly: true,
      });
      editorPanel.append(editMenu);
      currentWrap.append(editorPanel);
      pendingEditMenu = editMenu;
    }
    spritePalette.append(currentWrap);
    if (pendingEditMenu) {
      positionSpriteColorMenu(pendingEditMenu, currentButton, { side: "right" });
    }
  }

  const paletteGrid = document.createElement("span");
  paletteGrid.className = "sprite-palette-grid";

  if (spriteFillButton) {
    paletteGrid.append(spriteFillButton);
  }

  const eraseButton = document.createElement("button");
  eraseButton.type = "button";
  eraseButton.className = "sprite-token sprite-color-swatch sprite-token-erase";
  eraseButton.classList.toggle("is-selected", sprite.selectedColorIndex === null);
  eraseButton.dataset.colorIndex = "erase";
  eraseButton.style.setProperty("--sprite-swatch-color", "#00000000");
  eraseButton.title = "Paint transparent";
  eraseButton.setAttribute("aria-label", "Paint transparent sprite cell");
  eraseButton.innerHTML = `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"></path>
      <path d="M22 21H7"></path>
      <path d="m5 11 9 9"></path>
    </svg>
  `;
  eraseButton.addEventListener("click", () => selectSpriteColor(null));
  paletteGrid.append(eraseButton);

  for (const [index, entry] of sprite.palette.entries()) {
    const item = document.createElement("span");
    item.className = "sprite-token-item";
    item.classList.toggle("is-selected", index === sprite.selectedColorIndex);

    const button = document.createElement("button");
    button.type = "button";
    button.className = "sprite-token sprite-color-swatch";
    button.classList.toggle("is-selected", index === sprite.selectedColorIndex);
    button.dataset.colorIndex = String(index);
    button.style.setProperty("--sprite-swatch-color", normalizeSpriteColor(entry.color));
    button.style.setProperty("--sprite-token-ink", readableInkForColor(entry.color));
    const bind = spritePaletteEntryBindInfo(entry);
    button.classList.toggle("is-bound", bind.available && bind.linked);
    button.classList.toggle("is-unlinked", bind.available && !bind.linked);
    button.title = bind.available ? `Paint ${entry.color} (${bind.label})` : `Paint ${entry.color}`;
    button.setAttribute("aria-label", bind.available ? `Paint bound color ${index}: ${bind.label}` : `Paint color ${index}`);
    button.addEventListener("click", () => selectSpriteColor(index));
    item.append(button);

    const colorInput = document.createElement("input");
    colorInput.type = "color";
    colorInput.className = "sprite-token-color-input";
    colorInput.value = spriteRgbHex(entry.color);
    colorInput.setAttribute("aria-label", `Edit color ${index}`);
    colorInput.addEventListener("input", () => {
      sprite.selectedColorIndex = index;
      updateSelectedSpriteColor(colorInput.value, { deferHistory: true });
    });
    colorInput.addEventListener("change", () => {
      sprite.selectedColorIndex = index;
      updateSelectedSpriteColor(colorInput.value, { commitHistory: true });
    });
    item.append(colorInput);
    const bindMarker = renderSpriteBindMarker(entry);
    if (bindMarker) {
      item.append(bindMarker);
    }

    paletteGrid.append(item);
  }

  const addWrap = document.createElement("span");
  addWrap.className = "sprite-add-wrap";
  const addButton = document.createElement("button");
  addButton.type = "button";
  addButton.className = "sprite-token sprite-add-color-button";
  addButton.disabled = sprite.palette.length >= SPRITE_COLOR_TOKENS.length;
  addButton.title = "Add color";
  addButton.setAttribute("aria-label", "Add sprite color");
  addButton.setAttribute("aria-expanded", String(sprite.addPaletteOpen));
  addButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14"></path><path d="M5 12h14"></path></svg>`;
  addButton.addEventListener("click", toggleSpriteAddPalette);
  addWrap.append(addButton);
  paletteGrid.append(addWrap);

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className = "sprite-token sprite-remove-color-button";
  removeButton.disabled = !validSpriteColorIndex(sprite.selectedColorIndex) || sprite.palette.length <= 1;
  removeButton.title = "Remove selected color";
  removeButton.setAttribute("aria-label", "Remove selected color");
  removeButton.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14"></path></svg>`;
  removeButton.addEventListener("click", deleteSelectedSpriteColor);
  paletteGrid.append(removeButton);
  spritePalette.append(paletteGrid);

  if (sprite.addPaletteOpen) {
    const draft = validSpriteColorIndex(sprite.addDraftColorIndex)
      ? sprite.palette[sprite.addDraftColorIndex].color
      : nextSpritePresetColor();
    const addMenu = renderSpriteColorMenu({
      mode: "add",
      customValue: draft,
    });
    addMenu.classList.add("is-add-menu");
    spritePalette.append(addMenu);
    positionSpriteColorMenu(addMenu, paletteGrid, { side: "left" });
  }
}

function spritePaletteEntryBindInfo(entry) {
  const bind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!bind) {
    return { available: true, linked: false, name: "", label: "Unlinked color" };
  }
  if (typeof bind === "string") {
    return { available: true, linked: true, name: bind, label: `Bound to ${bind}` };
  }
  if (typeof bind === "object") {
    const name = bind.name || bind.ref || bind.source || bind.color || bind.palette || "";
    const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
    return { available: true, linked, name, label: name ? `Bound to ${name}` : "Bound color" };
  }
  return { available: true, linked: true, name: "", label: "Bound color" };
}

function renderSpriteCurrentColorTagButton(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-current-tag-button sprite-icon-button";
  button.classList.toggle("is-active", bind.linked);
  button.title = bind.name ? `Color tag: ${bind.name}` : "Tag selected color";
  button.setAttribute("aria-label", button.title);
  button.setAttribute("aria-pressed", String(bind.linked));
  button.setAttribute("aria-haspopup", "listbox");
  button.setAttribute("aria-expanded", String(Boolean(sprite.colorTagPickerOpen)));
  button.innerHTML = spriteTagIconSvg();
  button.addEventListener("click", () => {
    sprite.colorTagPickerOpen = !sprite.colorTagPickerOpen;
    renderSpritePalette();
  });
  return button;
}

function renderSpriteCurrentColorUnlinkButton(index, bind) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-current-tag-unlink-button sprite-icon-button";
  button.title = bind?.name ? `Unlink color tag ${bind.name}` : "Unlink color tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = spriteUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    sprite.colorTagPickerOpen = false;
    toggleSpritePaletteEntryBinding(index);
  });
  return button;
}

function renderSpriteAssetNamePicker({ className, names, value, placeholder, ariaLabel, emptyText, onCommit, onCancel }) {
  const picker = document.createElement("form");
  picker.className = ["sprite-tag-picker", className || ""].filter(Boolean).join(" ");
  picker.noValidate = true;
  const input = document.createElement("input");
  input.type = "text";
  input.className = "sprite-tag-picker-input";
  input.value = value || "";
  input.placeholder = placeholder;
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", ariaLabel);
  let submitted = false;
  const commit = (rawValue = input.value) => {
    if (submitted) {
      return;
    }
    submitted = Boolean(onCommit(rawValue) !== false);
  };
  picker.addEventListener("submit", (event) => {
    event.preventDefault();
    event.stopPropagation();
    commit();
  });
  input.addEventListener("change", () => commit());
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel?.();
      return;
    }
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    commit();
  });

  const options = document.createElement("span");
  options.className = "sprite-tag-options";
  options.setAttribute("role", "listbox");
  if (names.length) {
    for (const name of names) {
      const option = document.createElement("button");
      option.type = "button";
      option.className = "sprite-tag-option";
      option.textContent = name;
      option.setAttribute("role", "option");
      option.addEventListener("mousedown", (event) => event.preventDefault());
      option.addEventListener("click", () => commit(name));
      options.append(option);
    }
  } else {
    const empty = document.createElement("span");
    empty.className = "sprite-tag-empty";
    empty.textContent = emptyText;
    options.append(empty);
  }
  picker.append(input, options);
  return picker;
}

function focusSpriteTagPickerInput(tagPicker) {
  const input = tagPicker.querySelector(".sprite-tag-picker-input");
  if (!input) {
    return;
  }
  input.focus();
  input.select();
}

function spriteColorAssetNames() {
  return [...parseSpriteColorAssets(activeSpriteEditSource()).keys()].sort((a, b) => a.localeCompare(b));
}

function spriteShapeAssetNames() {
  return [...parseSpriteShapeAssets(activeSpriteEditSource()).keys()].sort((a, b) => a.localeCompare(b));
}

function applyCurrentColorName(index, rawName, options = {}) {
  if (!validSpriteColorIndex(index)) {
    return false;
  }
  const entry = sprite.palette[index];
  const name = sanitizeSpriteColorAssetRef(rawName);
  if (!name) {
    if (options.reportError) {
      setSpriteActionStatus("Enter a color name", "is-error");
    }
    return false;
  }
  const source = activeSpriteEditSource();
  const colorAssets = parseSpriteColorAssets(source);
  let status = `Using color ${name}`;
  if (colorAssets.has(name)) {
    const resolved = resolveSpriteColorAssetToken(name, colorAssets);
    if (!resolved) {
      if (options.reportError) {
        setSpriteActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    entry.color = resolved;
  } else {
    if (name.includes(":")) {
      if (options.reportError) {
        setSpriteActionStatus(`Cannot resolve color ${name}`, "is-error");
      }
      return false;
    }
    const staged = sprite.palette.find((candidate, candidateIndex) => {
      const bind = spritePaletteEntryBindInfo(candidate);
      return candidateIndex !== index && bind.linked && bind.name === name;
    });
    if (staged) {
      entry.color = normalizeSpriteColor(staged.color);
    }
    status = `Tagged color ${name}`;
  }
  entry.bind = { type: "color", name, linked: true };
  entry.editMode = "name";
  syncSpritePaletteEntriesForColorName(name, entry.color);
  setSpriteActionStatus(status, "is-ok");
  renderSpriteBuilder();
  return true;
}

function renderSpriteBindToggle(entry, index, options = {}) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.available) {
    return null;
  }
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["sprite-bind-toggle", options.className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", bind.linked);
  button.classList.toggle("is-unlinked", !bind.linked);
  button.dataset.colorIndex = String(index);
  button.title = bind.linked ? `Unlink ${bind.label}` : (bind.name ? `Relink ${bind.label}` : "Link color");
  button.setAttribute("aria-label", bind.linked ? `Unlink color ${index}` : (bind.name ? `Relink color ${index}` : `Link color ${index}`));
  button.innerHTML = spriteLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    toggleSpritePaletteEntryBinding(index);
  });
  return button;
}

function renderSpriteBindMarker(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.available || !bind.linked) {
    return null;
  }
  const marker = document.createElement("span");
  marker.className = "sprite-bind-marker is-linked";
  marker.title = bind.label;
  marker.setAttribute("aria-label", bind.label);
  marker.innerHTML = spriteTagIconSvg();
  return marker;
}

function renderSpriteAssetBindToggle({ bind, className, label, linkedTitle, unlinkedTitle, onClick }) {
  const info = spriteAssetBindInfo(bind, label);
  const button = document.createElement("button");
  button.type = "button";
  button.className = ["sprite-bind-toggle", "sprite-asset-bind-toggle", className || ""].filter(Boolean).join(" ");
  button.classList.toggle("is-linked", info.linked);
  button.classList.toggle("is-unlinked", !info.linked);
  button.title = info.linked ? `${linkedTitle}: ${info.name}` : unlinkedTitle;
  button.setAttribute("aria-label", info.linked ? `${linkedTitle} ${label}` : unlinkedTitle);
  button.innerHTML = spriteLinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return button;
}

function spriteAssetBindInfo(bind, label) {
  if (!bind) {
    return { linked: false, name: "", label: `Unlinked ${label}` };
  }
  if (typeof bind === "string") {
    return { linked: true, name: bind, label: `Bound to ${bind}` };
  }
  const name = bind.name || bind.ref || bind.source || "";
  const linked = !(bind.linked === false || bind.unlinked === true || bind.detached === true);
  return { linked, name, label: name ? `Bound to ${name}` : `Bound ${label}` };
}

function toggleSpritePaletteEntryBinding(index) {
  if (!validSpriteColorIndex(index)) {
    return;
  }
  const entry = sprite.palette[index];
  const rawBind = entry?.bind ?? entry?.bound ?? entry?.sourceRef ?? null;
  if (!rawBind) {
    linkSpritePaletteEntryToNewColor(index);
    return;
  }
  if (typeof rawBind === "string") {
    entry.bind = { type: "color", name: rawBind, linked: false };
  } else {
    rawBind.linked = rawBind.linked === false || rawBind.unlinked === true || rawBind.detached === true;
    delete rawBind.unlinked;
    delete rawBind.detached;
  }
  sprite.selectedColorIndex = index;
  const bind = spritePaletteEntryBindInfo(entry);
  rewriteCurrentSpriteDefinitionFromBuilder(bind.linked ? "Linked color" : "Unlinked color");
  renderSpritePalette();
  renderSpriteColorSurfaces();
}

function linkSpritePaletteEntryToNewColor(index) {
  const entry = sprite.palette[index];
  if (!entry) {
    return;
  }
  const name = promptSpriteAssetName("Color name", defaultSpriteAssetName("color", index));
  if (!name) {
    return;
  }
  const source = activeSpriteEditSource();
  const nextSource = ensureSpriteColorDefinition(source, name, normalizeSpriteColor(entry.color));
  if (!nextSource) {
    return;
  }
  entry.bind = { type: "color", name, linked: true };
  const rewritten = replaceSpriteDefinition(nextSource);
  if (!rewritten) {
    entry.bind = null;
    setSpriteActionStatus(`No sprite named ${spriteObjectName()}`, "is-error");
    return;
  }
  sprite.selectedColorIndex = index;
  applySpriteSourceChange(rewritten.source, `Linked color ${name}`);
  renderSpriteBuilder();
}

function rewriteCurrentSpriteDefinitionFromBuilder(status) {
  const result = replaceSpriteDefinition(activeSpriteEditSource());
  if (!result) {
    return false;
  }
  applySpriteSourceChange(result.source, status);
  return true;
}

function updateSpriteBoundColorDefinition(entry, color) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (!bind.linked || !bind.name) {
    return false;
  }
  const source = activeSpriteEditSource();
  const nextSource = replaceSpriteColorDefinition(source, bind.name, color);
  if (!nextSource || nextSource === source) {
    return false;
  }
  const applied = applySpriteSourceChange(nextSource);
  if (applied) {
    syncSpritePaletteEntriesForColorName(bind.name, color);
  }
  return applied;
}

function toggleSpritePaletteBinding() {
  const info = spriteAssetBindInfo(sprite.paletteBind, "palette");
  if (!info.name) {
    linkSpritePaletteToNewPalette();
    return;
  }
  sprite.paletteBind = { type: "palette", name: info.name, linked: !info.linked };
  rewriteCurrentSpriteDefinitionFromBuilder(sprite.paletteBind.linked ? "Linked palette" : "Unlinked palette");
  renderSpriteBuilder();
}

function linkSpritePaletteToNewPalette() {
  const name = promptSpriteAssetName("Palette name", defaultSpriteAssetName("palette"));
  if (!name) {
    return;
  }
  const source = activeSpriteEditSource();
  const nextSource = ensureSpritePaletteDefinition(source, name, spritePaletteSourceTokens());
  if (!nextSource) {
    return;
  }
  sprite.paletteBind = { type: "palette", name, linked: true };
  const rewritten = replaceSpriteDefinition(nextSource);
  if (!rewritten) {
    sprite.paletteBind = null;
    setSpriteActionStatus(`No sprite named ${spriteObjectName()}`, "is-error");
    return;
  }
  applySpriteSourceChange(rewritten.source, `Linked palette ${name}`);
  renderSpriteBuilder();
}

function updateSpriteBoundPaletteDefinition() {
  const info = spriteAssetBindInfo(sprite.paletteBind, "palette");
  if (!info.linked || !info.name) {
    return false;
  }
  const source = activeSpriteEditSource();
  const nextSource = replaceSpritePaletteDefinition(source, info.name, spritePaletteSourceTokens());
  if (!nextSource || nextSource === source) {
    return false;
  }
  applySpriteSourceChange(nextSource);
  return true;
}

function toggleSpriteShapeBinding() {
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!info.name) {
    linkSpriteShapeToNewShape();
    return;
  }
  sprite.shapeBind = { type: "shape", name: info.name, linked: !info.linked };
  rewriteCurrentSpriteDefinitionFromBuilder(sprite.shapeBind.linked ? "Linked shape" : "Unlinked shape");
  renderSpriteBuilder();
}

function linkSpriteShapeToNewShape() {
  const name = promptSpriteAssetName("Shape name", defaultSpriteAssetName("shape"));
  if (!name) {
    return;
  }
  const source = activeSpriteEditSource();
  const nextSource = ensureSpriteShapeDefinition(source, name, spriteAscii().split("\n"));
  if (!nextSource) {
    return;
  }
  sprite.shapeBind = { type: "shape", name, linked: true };
  const rewritten = replaceSpriteDefinition(nextSource);
  if (!rewritten) {
    sprite.shapeBind = null;
    setSpriteActionStatus(`No sprite named ${spriteObjectName()}`, "is-error");
    return;
  }
  applySpriteSourceChange(rewritten.source, `Linked shape ${name}`);
  renderSpriteBuilder();
}

function updateSpriteBoundShapeDefinition() {
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!info.linked || !info.name) {
    return false;
  }
  const source = activeSpriteEditSource();
  const nextSource = replaceSpriteShapeDefinition(source, info.name, spriteAscii().split("\n"));
  if (!nextSource || nextSource === source) {
    return false;
  }
  applySpriteSourceChange(nextSource);
  return true;
}

function promptSpriteAssetName(label, defaultValue) {
  let raw = defaultValue;
  try {
    raw = window.prompt(label, defaultValue);
  } catch {
    raw = defaultValue;
  }
  if (raw === null) {
    return null;
  }
  const name = sanitizeSpriteAssetName(raw);
  if (!name) {
    setSpriteActionStatus("Use an asset name like wall_color", "is-error");
    return null;
  }
  return name;
}

function sanitizeSpriteAssetName(value) {
  const cleaned = String(value || "")
    .trim()
    .replace(/[^\w]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!cleaned) {
    return "";
  }
  return /^[A-Za-z_]/.test(cleaned) ? cleaned : `color_${cleaned}`;
}

function sanitizeSpriteColorAssetRef(value) {
  const raw = String(value || "").trim();
  if (!raw.includes(":")) {
    return sanitizeSpriteAssetName(raw);
  }
  const parts = raw.split(":");
  if (parts.length !== 2) {
    return "";
  }
  const tableName = sanitizeSpriteAssetName(parts[0]);
  const rowName = sanitizeSpriteAssetName(parts[1]);
  return tableName && rowName ? `${tableName}:${rowName}` : "";
}

function defaultSpriteAssetName(kind, index = 0) {
  const base = sanitizeSpriteAssetName(spriteObjectName()).replace(new RegExp(`_${kind}$`), "") || "sprite";
  return `${base}_${kind}_${Number(index) + 1}`;
}

function syncSpritePaletteEntriesForColorName(name, color) {
  const normalized = normalizeSpriteColor(color);
  for (const entry of sprite.palette) {
    const bind = spritePaletteEntryBindInfo(entry);
    if (bind.linked && bind.name === name) {
      entry.color = normalized;
    }
  }
}

function applySpriteSourceChange(source, statusText = "") {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return false;
  }
  document.source = source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(source, { resetUndo: false });
  }
  scheduleLocalSave();
  schedulePreview();
  if (statusText) {
    setSpriteActionStatus(statusText, "is-ok");
    setStatus(statusText, "is-ok");
  }
  return true;
}

function activeSpriteEditDocument() {
  const editDocument = sprite.editDocumentId
    ? documents.find((candidate) => candidate.id === sprite.editDocumentId)
    : null;
  if (editDocument && isTextDocument(editDocument) && isPuzzleDocument(editDocument)) {
    return editDocument;
  }
  const document = activeDocument();
  if (document && isTextDocument(document) && isPuzzleDocument(document)) {
    return document;
  }
  return activePreviewDocument();
}

function setSpriteEditDocument(document = activeDocument()) {
  sprite.editDocumentId = document && isTextDocument(document) && isPuzzleDocument(document)
    ? document.id
    : null;
}

function setSpriteEditSource(entry, document = activeDocument()) {
  setSpriteEditDocument(document);
  sprite.editSourceStart = Number.isInteger(entry?.start)
    ? entry.start
    : Number.isInteger(entry?.openIndex)
      ? entry.openIndex
      : null;
  sprite.editSourceEnd = Number.isInteger(entry?.end) ? entry.end : null;
  sprite.editSourceBodyStart = Number.isInteger(entry?.bodyStart) ? entry.bodyStart : null;
  sprite.editSourceBodyEnd = Number.isInteger(entry?.bodyEnd) ? entry.bodyEnd : null;
  sprite.editSourceName = entry?.name || "";
}

function clearSpriteEditSource() {
  sprite.editSourceStart = null;
  sprite.editSourceEnd = null;
  sprite.editSourceBodyStart = null;
  sprite.editSourceBodyEnd = null;
  sprite.editSourceName = "";
}

function invalidateSpriteEditSourceForDocument(document = activeDocument()) {
  if (!document || !sprite.editDocumentId || document.id !== sprite.editDocumentId) {
    return false;
  }
  clearSpriteEditSource();
  return true;
}

function activeSpriteEditSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    return "";
  }
  return document.id === activeDocument()?.id ? sourceEditor.value : document.source || "";
}

function spriteLinkIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M10 13a5 5 0 0 0 7.1.1l2-2a5 5 0 0 0-7.1-7.1l-1.1 1.1"></path>
      <path d="M14 11a5 5 0 0 0-7.1-.1l-2 2a5 5 0 0 0 7.1 7.1l1.1-1.1"></path>
    </svg>
  `;
}

function spriteUnlinkIconSvg() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"></path>
      <circle cx="7.5" cy="7.5" r=".5" fill="currentColor"></circle>
      <path d="M3 21 21 3"></path>
    </svg>
  `;
}

function spriteTagIconSvg() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-tag-icon lucide-tag" aria-hidden="true">
      <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"></path>
      <circle cx="7.5" cy="7.5" r=".5" fill="currentColor"></circle>
    </svg>
  `;
}

function positionSpriteColorMenu(menu, anchor, options = {}) {
  const menuRect = menu.getBoundingClientRect();
  const anchorRect = anchor.getBoundingClientRect();
  const gap = 6;
  const margin = 8;
  const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
  const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
  const menuWidth = menuRect.width > 0 && menuRect.width < 420 ? menuRect.width : 274;
  const menuHeight = menuRect.height > 0 && menuRect.height < viewportHeight ? menuRect.height : 224;
  const preferLeft = options.side === "left";
  let left = preferLeft
    ? anchorRect.left - menuWidth - gap
    : anchorRect.right + gap;
  if (left < margin) {
    left = anchorRect.right + gap;
  }
  if (left + menuWidth > viewportWidth - margin) {
    left = anchorRect.left - menuWidth - gap;
  }
  left = Math.max(margin, Math.min(left, viewportWidth - menuWidth - margin));
  const top = Math.max(
    margin,
    Math.min(anchorRect.top, viewportHeight - menuHeight - margin),
  );
  menu.style.position = "fixed";
  menu.style.left = `${left}px`;
  menu.style.right = "auto";
  menu.style.top = `${top}px`;
  menu.style.zIndex = "50";
}

function renderSpriteColorMenu({
  mode,
  customValue,
  customOnly = false,
  inline = false,
  onPickError = setSpriteActionStatus,
  onPreset = null,
  onChange = null,
  onDiscard = cancelSpriteColorAdd,
  renderPalette = renderSpritePalette,
}) {
  const presetList = document.createElement("span");
  presetList.className = [
    "sprite-color-menu",
    "is-adjuster",
    customOnly ? "is-custom-only" : "",
    inline ? "is-inline-custom" : "",
  ].filter(Boolean).join(" ");

  if (!customOnly) {
    const presetGrid = document.createElement("span");
    presetGrid.className = "sprite-preset-grid";
    for (const color of SPRITE_COLOR_PRESETS) {
      const preset = document.createElement("button");
      preset.type = "button";
      preset.className = "sprite-color-preset sprite-color-swatch";
      preset.classList.toggle("is-selected", normalizeSpriteColor(color) === normalizeSpriteColor(customValue));
      preset.style.setProperty("--sprite-swatch-color", normalizeSpriteColor(color));
      preset.title = mode === "add" ? `Start from ${color}` : `Use ${color}`;
      preset.setAttribute("aria-label", mode === "add" ? `Start from color ${color}` : `Use color ${color}`);
      preset.addEventListener("click", () => {
        if (onPreset) {
          onPreset(color, { deferHistory: true });
        } else if (mode === "add") {
          previewNewSpriteColor(color, { deferHistory: true });
        } else {
          updateSelectedSpriteColor(color, { deferHistory: true });
        }
        renderPalette();
      });
      presetGrid.append(preset);
    }
    presetList.append(presetGrid);
  }
  presetList.append(renderSpriteColorAdjuster({
    color: customValue,
    ariaLabel: mode === "add" ? "New color" : "Selected color",
    onPickError,
    onChange: (color) => {
      if (onChange) {
        onChange(color, { deferHistory: true });
      } else if (mode === "add") {
        previewNewSpriteColor(color, { deferHistory: true });
      } else {
        updateSelectedSpriteColor(color, { deferHistory: true });
      }
    },
  }));
  const actionRow = document.createElement("span");
  actionRow.className = "sprite-color-actions";
  if (mode === "add") {
    actionRow.classList.add("is-floating");
    const discardButton = document.createElement("button");
    discardButton.type = "button";
    discardButton.className = "sprite-color-action-button sprite-color-trash-button";
    discardButton.title = "Discard new color";
    discardButton.setAttribute("aria-label", "Discard new color");
    discardButton.innerHTML = spriteTrashIconSvg();
    discardButton.addEventListener("click", onDiscard);
    actionRow.append(discardButton);
  } else {
    actionRow.hidden = true;
  }
  presetList.append(actionRow);
  return presetList;
}

function spriteTrashIconSvg() {
  return `
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 6h18"></path>
      <path d="M8 6V4h8v2"></path>
      <path d="M6 6l1 15h10l1-15"></path>
      <path d="M10 11v6"></path>
      <path d="M14 11v6"></path>
    </svg>
  `;
}

function renderSpriteBoard() {
  spriteBoard.replaceChildren();
  spriteBoard.style.setProperty("--sprite-size", sprite.size);
  for (let index = 0; index < sprite.cells.length; index += 1) {
    const button = document.createElement("button");
    const colorIndex = validSpriteColorIndex(sprite.cells[index]) ? sprite.cells[index] : null;
    const char = spriteExportCharForColorIndex(colorIndex);
    button.type = "button";
    button.className = "sprite-cell sprite-color-swatch";
    button.dataset.index = String(index);
    button.dataset.colorIndex = colorIndex === null ? "erase" : String(colorIndex);
    button.style.setProperty("--sprite-swatch-color", spriteColorForColorIndex(colorIndex));
    button.style.setProperty("--sprite-cell-ink", spriteInkForColorIndex(colorIndex));
    button.setAttribute("aria-label", `Sprite cell ${index + 1}: ${char}`);
    spriteBoard.append(button);
  }
}

function selectSpriteColor(index) {
  commitSpriteColorEditHistory("sprite");
  sprite.selectedColorIndex = validSpriteColorIndex(index) ? index : null;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteControls();
  renderSpritePalette();
}

function updateSelectedSpriteColor(value, options = {}) {
  const before = options.deferHistory || options.commitHistory ? null : visualEditSnapshot("sprite");
  if (options.deferHistory || options.commitHistory) {
    beginSpriteColorEditHistory("sprite");
  }
  if (!validSpriteColorIndex(sprite.selectedColorIndex)) {
    sprite.selectedColorIndex = 0;
  }
  const selected = sprite.palette[sprite.selectedColorIndex];
  if (!selected) {
    return;
  }
  const normalized = normalizeSpriteColor(value);
  selected.color = normalized;
  updateSpriteBoundColorDefinition(selected, normalized);
  updateSpriteBoundPaletteDefinition();
  if (options.closeMenu) {
    sprite.editPaletteOpen = false;
    sprite.customColorOpen = false;
    sprite.addDraftColorIndex = null;
    renderSpriteBuilder();
    if (options.deferHistory || options.commitHistory) {
      commitSpriteColorEditHistory("sprite");
    } else {
      pushVisualEditUndoSnapshot("sprite", before);
    }
    return;
  }
  renderSpriteColorSurfaces();
  if (options.deferHistory) {
    return;
  }
  if (options.commitHistory) {
    commitSpriteColorEditHistory("sprite");
    return;
  }
  pushVisualEditUndoSnapshot("sprite", before);
}

function openColorInput(input) {
  input.focus({ preventScroll: true });
  if (typeof input.showPicker === "function") {
    try {
      input.showPicker();
      return;
    } catch (_error) {
      // Fall through to click for browsers that expose showPicker but reject it.
    }
  }
  input.click();
}

function toggleSpriteAddPalette() {
  commitSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  const opening = !sprite.addPaletteOpen;
  if (opening && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    setSpriteActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  sprite.addPaletteOpen = opening;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = opening;
  if (opening) {
    if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
      sprite.palette.push({ color: normalizeSpriteColor(nextSpritePresetColor()) });
      sprite.addDraftColorIndex = sprite.palette.length - 1;
    }
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteBuilder();
    pushVisualEditUndoSnapshot("sprite", before);
    return;
  }
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function addSpriteColor(color = nextSpritePresetColor()) {
  const before = visualEditSnapshot("sprite");
  const draftIndex = validSpriteColorIndex(sprite.addDraftColorIndex) ? sprite.addDraftColorIndex : null;
  if (draftIndex === null && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    setSpriteActionStatus(`Palette limit is ${SPRITE_COLOR_TOKENS.length} colors`, "is-error");
    return;
  }
  if (draftIndex === null) {
    sprite.palette.push({ color: normalizeSpriteColor(color) });
    sprite.selectedColorIndex = sprite.palette.length - 1;
  } else {
    sprite.palette[draftIndex].color = normalizeSpriteColor(color);
    sprite.selectedColorIndex = draftIndex;
  }
  updateSpriteBoundPaletteDefinition();
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function previewNewSpriteColor(color, options = {}) {
  const before = options.deferHistory ? null : visualEditSnapshot("sprite");
  if (options.deferHistory) {
    beginSpriteColorEditHistory("sprite");
  }
  if (!validSpriteColorIndex(sprite.addDraftColorIndex) && sprite.palette.length >= SPRITE_COLOR_TOKENS.length) {
    return;
  }
  if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
    sprite.palette.push({ color: normalizeSpriteColor(color) });
    sprite.addDraftColorIndex = sprite.palette.length - 1;
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteBuilder();
  } else {
    sprite.palette[sprite.addDraftColorIndex].color = normalizeSpriteColor(color);
    sprite.selectedColorIndex = sprite.addDraftColorIndex;
    renderSpriteColorSurfaces();
  }
  if (options.closeMenu) {
    sprite.addPaletteOpen = false;
    sprite.editPaletteOpen = false;
    sprite.customColorOpen = false;
    sprite.addDraftColorIndex = null;
    renderSpriteBuilder();
  }
  if (options.deferHistory) {
    return;
  }
  pushVisualEditUndoSnapshot("sprite", before);
}

function closeSpriteColorEditor() {
  commitSpriteColorEditHistory("sprite");
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpritePalette();
}

function confirmSpriteColorAdd() {
  if (!validSpriteColorIndex(sprite.addDraftColorIndex)) {
    return;
  }
  commitSpriteColorEditHistory("sprite");
  sprite.selectedColorIndex = sprite.addDraftColorIndex;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
}

function cancelSpriteColorAdd() {
  discardSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  if (validSpriteColorIndex(sprite.addDraftColorIndex)) {
    removeSpritePaletteColor(sprite.addDraftColorIndex);
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  updateSpriteBoundPaletteDefinition();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function closeSpriteColorEditorFromOutside(event) {
  if (!sprite.addPaletteOpen && !sprite.editPaletteOpen && !sprite3d.addPaletteOpen && !sprite3d.editPaletteOpen) {
    return;
  }
  if (spritePalette.contains(event.target)) {
    return;
  }
  if (sprite3dPalette?.contains(event.target)) {
    return;
  }
  closeSpriteColorEditor();
  if (typeof closeSprite3dColorEditor === "function") {
    closeSprite3dColorEditor();
  }
}

function nextSpritePresetColor(palette = sprite.palette) {
  const used = new Set(palette.map((entry) => normalizeSpriteColor(entry.color)));
  return SPRITE_COLOR_PRESETS.find((color) => !used.has(color)) || "#e94f64";
}

function deleteSelectedSpriteColor() {
  commitSpriteColorEditHistory("sprite");
  const before = visualEditSnapshot("sprite");
  if (!validSpriteColorIndex(sprite.selectedColorIndex) || sprite.palette.length <= 1) {
    return;
  }
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  removeSpritePaletteColor(sprite.selectedColorIndex);
  updateSpriteBoundPaletteDefinition();
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function removeSpritePaletteColor(deletedIndex) {
  if (!validSpriteColorIndex(deletedIndex) || sprite.palette.length <= 1) {
    return;
  }
  const oldPaletteLength = sprite.palette.length;
  sprite.palette.splice(deletedIndex, 1);
  sprite.cells = sprite.cells.map((colorIndex) => {
    if (!Number.isInteger(colorIndex) || colorIndex < 0 || colorIndex >= oldPaletteLength) {
      return null;
    }
    if (colorIndex === deletedIndex) {
      return null;
    }
    return colorIndex > deletedIndex ? colorIndex - 1 : colorIndex;
  });
  sprite.selectedColorIndex = Math.min(deletedIndex, sprite.palette.length - 1);
}

function normalizeSpriteColor(value) {
  return parseSpriteHexColor(value) || "#e94f64";
}

function parseSpriteHexColor(value) {
  const color = String(value || "").trim();
  if (color.toLowerCase() === "transparent") {
    return "#00000000";
  }
  const full = color.startsWith("#") ? color : `#${color}`;
  if (/^#[0-9a-f]{8}$/i.test(full)) {
    return full.toLowerCase();
  }
  if (/^#[0-9a-f]{6}$/i.test(full)) {
    return full.toLowerCase();
  }
  const shortAlpha = full.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  if (shortAlpha) {
    return `#${shortAlpha[1]}${shortAlpha[1]}${shortAlpha[2]}${shortAlpha[2]}${shortAlpha[3]}${shortAlpha[3]}${shortAlpha[4]}${shortAlpha[4]}`.toLowerCase();
  }
  const short = full.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i);
  if (short) {
    return `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}`.toLowerCase();
  }
  return "";
}

function spriteRgbHex(value) {
  return normalizeSpriteColor(value).slice(0, 7);
}

function spriteRgbComponents(value) {
  const rgb = spriteRgbHex(value);
  return {
    r: Number.parseInt(rgb.slice(1, 3), 16),
    g: Number.parseInt(rgb.slice(3, 5), 16),
    b: Number.parseInt(rgb.slice(5, 7), 16),
  };
}

function spriteHsvFromColor(value) {
  const { r, g, b } = spriteRgbComponents(value);
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;
  let h = 0;
  if (delta !== 0) {
    if (max === rn) {
      h = 60 * (((gn - bn) / delta) % 6);
    } else if (max === gn) {
      h = 60 * ((bn - rn) / delta + 2);
    } else {
      h = 60 * ((rn - gn) / delta + 4);
    }
  }
  if (h < 0) {
    h += 360;
  }
  return {
    h,
    s: max === 0 ? 0 : delta / max,
    v: max,
  };
}

function spriteRgbFromHsv(hsv) {
  const h = ((Number(hsv.h) || 0) % 360 + 360) % 360;
  const s = Math.max(0, Math.min(1, Number(hsv.s) || 0));
  const v = Math.max(0, Math.min(1, Number(hsv.v) || 0));
  const c = v * s;
  const x = c * (1 - Math.abs((h / 60) % 2 - 1));
  const m = v - c;
  let rn = 0;
  let gn = 0;
  let bn = 0;
  if (h < 60) {
    rn = c; gn = x;
  } else if (h < 120) {
    rn = x; gn = c;
  } else if (h < 180) {
    gn = c; bn = x;
  } else if (h < 240) {
    gn = x; bn = c;
  } else if (h < 300) {
    rn = x; bn = c;
  } else {
    rn = c; bn = x;
  }
  const toHex = (value) => Math.round((value + m) * 255).toString(16).padStart(2, "0");
  return `#${toHex(rn)}${toHex(gn)}${toHex(bn)}`;
}

function spriteAlphaPercent(value) {
  const normalized = normalizeSpriteColor(value);
  if (normalized.length !== 9) {
    return 100;
  }
  return Math.round((Number.parseInt(normalized.slice(7, 9), 16) / 255) * 100);
}

function spriteColorWithAlpha(rgb, alphaPercent) {
  const base = spriteRgbHex(rgb);
  const percent = Math.max(0, Math.min(100, Math.round(Number(alphaPercent) || 0)));
  if (percent >= 100) {
    return base;
  }
  const alpha = Math.round((percent / 100) * 255).toString(16).padStart(2, "0");
  return `${base}${alpha}`;
}

function renderSpriteColorSurfaces() {
  syncSpritePaletteSwatches();
  syncSpriteColorAdjusters();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
}

function syncSpritePaletteSwatches() {
  for (const [index, entry] of sprite.palette.entries()) {
    const color = normalizeSpriteColor(entry.color);
    for (const token of spritePalette.querySelectorAll(`[data-color-index="${index}"]`)) {
      token.style.setProperty("--sprite-swatch-color", color);
      token.style.setProperty("--sprite-token-ink", readableInkForColor(color));
      token.title = `Paint ${color}`;
    }
  }
  const selected = sprite.palette[sprite.selectedColorIndex];
  const currentButton = spritePalette.querySelector(".sprite-current-color-button");
  if (currentButton && selected) {
    const normalized = normalizeSpriteColor(selected.color);
    currentButton.style.setProperty("--sprite-current-color", normalized);
    currentButton.setAttribute("aria-label", `Pick selected color ${normalized}`);
    const currentHexInput = spritePalette.querySelector(".sprite-current-hex-input");
    if (currentHexInput && !currentHexInput.classList.contains("is-name-mode") && document.activeElement !== currentHexInput) {
      currentHexInput.value = normalized;
    }
  }
}

function syncSpriteColorAdjusters() {
  const selected = validSpriteColorIndex(sprite.selectedColorIndex)
    ? sprite.palette[sprite.selectedColorIndex]
    : null;
  if (!selected) {
    return;
  }
  const normalized = normalizeSpriteColor(selected.color);
  for (const adjuster of spritePalette.querySelectorAll(".sprite-color-adjuster")) {
    if (adjuster.contains(document.activeElement)) {
      continue;
    }
    adjuster.syncColor?.(normalized);
  }
}

function validSpriteColorIndex(index) {
  return Number.isInteger(index) && index >= 0 && index < sprite.palette.length;
}

function spriteExportCharForColorIndex(index) {
  if (!validSpriteColorIndex(index)) {
    return ".";
  }
  return SPRITE_COLOR_TOKENS[index] || ".";
}

function spriteColorForColorIndex(index) {
  return validSpriteColorIndex(index) ? normalizeSpriteColor(sprite.palette[index].color) : "#00000000";
}

function spriteInkForColorIndex(index) {
  return validSpriteColorIndex(index) ? readableInkForColor(sprite.palette[index].color) : "#8d969f";
}

function readableInkForColor(color) {
  const normalized = normalizeSpriteColor(color).slice(1);
  const red = Number.parseInt(normalized.slice(0, 2), 16);
  const green = Number.parseInt(normalized.slice(2, 4), 16);
  const blue = Number.parseInt(normalized.slice(4, 6), 16);
  const alpha = normalized.length >= 8 ? Number.parseInt(normalized.slice(6, 8), 16) / 255 : 1;
  const base = 190;
  const mixedRed = red * alpha + base * (1 - alpha);
  const mixedGreen = green * alpha + base * (1 - alpha);
  const mixedBlue = blue * alpha + base * (1 - alpha);
  const luminance = (mixedRed * 299 + mixedGreen * 587 + mixedBlue * 114) / 1000;
  return luminance > 150 ? "#1d242b" : "#ffffff";
}

function updateSpriteSize(value) {
  const before = visualEditSnapshot("sprite");
  const nextSize = clampSpriteSize(value);
  if (nextSize === sprite.size) {
    renderSpriteControls();
    return;
  }
  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  const copySize = Math.min(sprite.size, nextSize);
  for (let y = 0; y < copySize; y += 1) {
    for (let x = 0; x < copySize; x += 1) {
      const value = sprite.cells[y * sprite.size + x];
      nextCells[y * nextSize + x] = validSpriteColorIndex(value) ? value : null;
    }
  }
  sprite.size = nextSize;
  sprite.cells = nextCells;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  pushVisualEditUndoSnapshot("sprite", before);
}

function spriteScaleFactor() {
  return spriteEditorScaleFactor(spriteScaleInput, SPRITE_EDITOR_MAX_SIZE);
}

function canScaleDownSprite(factor = spriteScaleFactor()) {
  return factor > 1 && sprite.size >= factor && sprite.size % factor === 0;
}

function scaleUpSprite() {
  const before = visualEditSnapshot("sprite");
  const factor = spriteScaleFactor();
  const nextSize = sprite.size * factor;
  if (nextSize > SPRITE_EDITOR_MAX_SIZE) {
    setSpriteActionStatus(`Sprite size limit is ${SPRITE_EDITOR_MAX_SIZE}`, "is-error");
    renderSpriteControls();
    return;
  }

  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  for (let y = 0; y < sprite.size; y += 1) {
    for (let x = 0; x < sprite.size; x += 1) {
      const colorIndex = validSpriteColorIndex(sprite.cells[y * sprite.size + x])
        ? sprite.cells[y * sprite.size + x]
        : null;
      const nextX = x * factor;
      const nextY = y * factor;
      for (let dy = 0; dy < factor; dy += 1) {
        for (let dx = 0; dx < factor; dx += 1) {
          nextCells[(nextY + dy) * nextSize + nextX + dx] = colorIndex;
        }
      }
    }
  }

  sprite.size = nextSize;
  sprite.cells = nextCells;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  const message = `Scaled ${factor}x to ${nextSize}x${nextSize}`;
  setSpriteActionStatus(message, "is-ok");
  setStatus(`Scaled sprite ${factor}x to ${nextSize}x${nextSize}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function scaleDownSprite() {
  const before = visualEditSnapshot("sprite");
  const factor = spriteScaleFactor();
  if (!canScaleDownSprite(factor)) {
    setSpriteActionStatus(`Size ${sprite.size} is not divisible by ${factor}`, "is-error");
    renderSpriteControls();
    return;
  }

  const nextSize = sprite.size / factor;
  const nextCells = Array.from({ length: nextSize * nextSize }, () => null);
  for (let y = 0; y < nextSize; y += 1) {
    for (let x = 0; x < nextSize; x += 1) {
      nextCells[y * nextSize + x] = validSpriteColorIndex(sprite.cells[(y * factor) * sprite.size + (x * factor)])
        ? sprite.cells[(y * factor) * sprite.size + (x * factor)]
        : null;
    }
  }

  sprite.size = nextSize;
  sprite.cells = nextCells;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  const message = `Scaled down ${factor}x to ${nextSize}x${nextSize}`;
  setSpriteActionStatus(message, "is-ok");
  setStatus(`Scaled sprite down ${factor}x to ${nextSize}x${nextSize}`, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function transformSpriteCells(mapper, message) {
  const before = visualEditSnapshot("sprite");
  const size = sprite.size;
  const previousCells = sprite.cells;
  const nextCells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const source = mapper(x, y, size);
      const colorIndex = previousCells[source.y * size + source.x];
      nextCells[y * size + x] = validSpriteColorIndex(colorIndex) ? colorIndex : null;
    }
  }
  sprite.cells = nextCells;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  updateSpriteBoundShapeDefinition();
  renderSpriteBuilder();
  setSpriteActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function rotateSpriteLeft() {
  transformSpriteCells((x, y, size) => ({ x: size - 1 - y, y: x }), "Rotated left");
}

function rotateSpriteRight() {
  transformSpriteCells((x, y, size) => ({ x: y, y: size - 1 - x }), "Rotated right");
}

function flipSpriteHorizontal() {
  transformSpriteCells((x, y, size) => ({ x: size - 1 - x, y }), "Flipped horizontal");
}

function flipSpriteVertical() {
  transformSpriteCells((x, y, size) => ({ x, y: size - 1 - y }), "Flipped vertical");
}

function normalizedSpriteCellColorIndex(index) {
  const colorIndex = sprite.cells[index];
  return validSpriteColorIndex(colorIndex) ? colorIndex : null;
}

function floodFillSpriteComponentAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite.cells.length) {
    return 0;
  }
  const nextColorIndex = validSpriteColorIndex(colorIndex) ? colorIndex : null;
  const targetColorIndex = normalizedSpriteCellColorIndex(index);
  if (targetColorIndex === nextColorIndex) {
    return 0;
  }
  const size = sprite.size;
  const visited = new Uint8Array(sprite.cells.length);
  const stack = [index];
  let changed = 0;
  while (stack.length) {
    const current = stack.pop();
    if (visited[current] || normalizedSpriteCellColorIndex(current) !== targetColorIndex) {
      continue;
    }
    visited[current] = 1;
    sprite.cells[current] = nextColorIndex;
    changed += 1;
    const x = current % size;
    const y = Math.floor(current / size);
    if (x > 0) {
      stack.push(current - 1);
    }
    if (x < size - 1) {
      stack.push(current + 1);
    }
    if (y > 0) {
      stack.push(current - size);
    }
    if (y < size - 1) {
      stack.push(current + size);
    }
  }
  if (!changed) {
    return 0;
  }
  sprite.solidSource = false;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  updateSpriteBoundShapeDefinition();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
  return changed;
}

function bucketFillSpriteFromIndex(index) {
  const count = floodFillSpriteComponentAtIndex(index, sprite.selectedColorIndex);
  if (!count) {
    setSpriteActionStatus("Connected area already has that color", "is-ok");
    return false;
  }
  const colorIndex = validSpriteColorIndex(sprite.selectedColorIndex) ? sprite.selectedColorIndex : null;
  const message = colorIndex === null ? "Filled connected area with transparent" : "Filled connected area";
  setSpriteActionStatus(message, "is-ok");
  setStatus(message, "is-ok");
  return true;
}

function bucketFillSpriteFromElement(element) {
  return bucketFillSpriteFromIndex(spriteCellIndexFromElement(element));
}

function paintSpriteCellFromElement(element) {
  const index = spriteCellIndexFromElement(element);
  return paintSpriteCellAtIndex(index, sprite.selectedColorIndex);
}

function spriteCellIndexFromElement(element) {
  const cell = element?.closest?.(".sprite-cell");
  if (!cell || !spriteBoard.contains(cell)) {
    return -1;
  }
  const index = Number(cell.dataset.index);
  return Number.isInteger(index) && index >= 0 && index < sprite.cells.length ? index : -1;
}

function paintSpriteCellAtIndex(index, colorIndex) {
  if (!Number.isInteger(index) || index < 0 || index >= sprite.cells.length) {
    return false;
  }
  const nextColorIndex = validSpriteColorIndex(colorIndex) ? colorIndex : null;
  if (sprite.cells[index] === nextColorIndex) {
    return false;
  }
  sprite.cells[index] = nextColorIndex;
  sprite.solidSource = false;
  updateSpriteBoundShapeDefinition();
  renderSpriteBoard();
  syncSpriteSourceActionButtons();
  return true;
}

function paintSpriteCellFromPoint(clientX, clientY, colorIndex) {
  return paintSpriteCellAtIndex(
    spriteCellIndexFromElement(document.elementFromPoint(clientX, clientY)),
    colorIndex,
  );
}

function startSpritePaint(event) {
  if (event.button !== 0) {
    return;
  }
  const index = spriteCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY));
  if (!Number.isInteger(index) || index < 0) {
    return;
  }
  event.preventDefault();
  if (spriteBucketActive) {
    const before = visualEditSnapshot("sprite");
    if (bucketFillSpriteFromIndex(index)) {
      pushVisualEditUndoSnapshot("sprite", before);
    }
    return;
  }
  spritePaintDrag = {
    pointerId: event.pointerId,
    colorIndex: sprite.selectedColorIndex,
    lastIndex: -1,
    beforeSnapshot: visualEditSnapshot("sprite"),
    changed: false,
  };
  if (spriteBoard.setPointerCapture) {
    spriteBoard.setPointerCapture(event.pointerId);
  }
  paintSpriteDragIndex(index);
}

function continueSpritePaint(event) {
  if (!spritePaintDrag || spritePaintDrag.pointerId !== event.pointerId) {
    return;
  }
  event.preventDefault();
  paintSpriteDragIndex(spriteCellIndexFromElement(document.elementFromPoint(event.clientX, event.clientY)));
}

function stopSpritePaint(event) {
  if (!spritePaintDrag || spritePaintDrag.pointerId !== event.pointerId) {
    return;
  }
  if (spriteBoard.hasPointerCapture?.(event.pointerId)) {
    spriteBoard.releasePointerCapture(event.pointerId);
  }
  if (spritePaintDrag.changed) {
    pushVisualEditUndoSnapshot("sprite", spritePaintDrag.beforeSnapshot);
  }
  spritePaintDrag = null;
}

function paintSpriteDragIndex(index) {
  if (!spritePaintDrag || !Number.isInteger(index) || index < 0 || index === spritePaintDrag.lastIndex) {
    return;
  }
  spritePaintDrag.lastIndex = index;
  if (paintSpriteCellAtIndex(index, spritePaintDrag.colorIndex)) {
    spritePaintDrag.changed = true;
  }
}

function spriteAscii() {
  const rows = [];
  for (let y = 0; y < sprite.size; y += 1) {
    const row = [];
    for (let x = 0; x < sprite.size; x += 1) {
      row.push(spriteExportCharForColorIndex(sprite.cells[y * sprite.size + x]));
    }
    rows.push(row.join(""));
  }
  return rows.join("\n");
}

function spriteClipboardText() {
  return puzzleSpriteText();
}

function puzzleSpriteText() {
  return spriteObjectDefinitionText("");
}

function spriteObjectName() {
  const cleaned = String(spriteNameInput.value || "")
    .trim()
    .replace(/[^\w:@]+/g, "_")
    .replace(/(?!^)@/g, "_")
    .replace(/^_+|_+$/g, "");
  return cleaned || "Sprite";
}

function renderSpriteShapeBindRow(target) {
  if (!target) {
    return;
  }
  target.replaceChildren();
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  const row = document.createElement("div");
  row.className = "sprite-shape-bind-row";
  row.classList.toggle("has-unlink", info.linked && info.name);
  const label = document.createElement("span");
  label.className = "sprite-shape-bind-label";
  label.textContent = "Shape";
  const input = document.createElement("input");
  input.type = "text";
  input.className = "sprite-shape-name-input";
  input.value = info.name || "";
  input.placeholder = "";
  input.spellcheck = false;
  input.autocomplete = "off";
  input.setAttribute("aria-label", "Shape name");
  const tagButton = document.createElement("button");
  tagButton.type = "button";
  tagButton.className = "sprite-shape-tag-button sprite-icon-button";
  tagButton.classList.toggle("is-active", info.linked);
  tagButton.innerHTML = spriteTagIconSvg();
  tagButton.setAttribute("aria-pressed", String(info.linked));
  tagButton.setAttribute("aria-haspopup", "listbox");
  tagButton.setAttribute("aria-expanded", String(Boolean(sprite.shapeTagPickerOpen)));
  tagButton.title = info.name ? `Shape tag: ${info.name}` : "Tag shape by name";
  tagButton.setAttribute("aria-label", tagButton.title);
  const commitName = (options = {}) => {
    commitSpriteShapeName(input.value, {
      sync: Boolean(sprite.shapeTagPickerOpen) || spriteAssetBindInfo(sprite.shapeBind, "shape").linked,
      ...options,
    });
  };
  input.addEventListener("change", () => commitName({ reportError: true }));
  input.addEventListener("blur", () => commitName({ reportError: false }));
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    commitName({ reportError: true });
  });
  tagButton.addEventListener("click", () => {
    sprite.shapeTagPickerOpen = !sprite.shapeTagPickerOpen;
    renderSpriteControls();
  });
  row.append(label, input, tagButton);
  if (info.linked && info.name) {
    row.append(renderSpriteShapeUnlinkButton(info));
  }
  if (sprite.shapeTagPickerOpen) {
    const tagPicker = renderSpriteAssetNamePicker({
      className: "sprite-shape-tag-picker",
      names: spriteShapeAssetNames(),
      value: info.name || "",
      placeholder: "",
      ariaLabel: "Shape tag name",
      emptyText: "No named shapes yet",
      onCommit: (name) => {
        const wasOpen = sprite.shapeTagPickerOpen;
        sprite.shapeTagPickerOpen = false;
        const ok = setSpriteShapeSync(true, name);
        if (!ok) {
          sprite.shapeTagPickerOpen = wasOpen;
        }
        return ok;
      },
      onCancel: () => {
        sprite.shapeTagPickerOpen = false;
        renderSpriteControls();
      },
    });
    row.append(tagPicker);
    requestAnimationFrame(() => {
      focusSpriteTagPickerInput(tagPicker);
    });
  }
  target.append(row);
}

function renderSpriteShapeUnlinkButton(info) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sprite-shape-tag-unlink-button sprite-icon-button";
  button.title = info?.name ? `Unlink shape tag ${info.name}` : "Unlink shape tag";
  button.setAttribute("aria-label", button.title);
  button.innerHTML = spriteUnlinkIconSvg();
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    sprite.shapeTagPickerOpen = false;
    toggleSpriteShapeBinding();
  });
  return button;
}

function commitSpriteShapeName(rawName, options = {}) {
  const name = sanitizeSpriteAssetName(rawName);
  const info = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (!name) {
    if (info.name) {
      sprite.shapeBind = null;
      rewriteCurrentSpriteDefinitionFromBuilder("Shape sync off");
      renderSpriteBuilder();
    } else if (options.reportError && options.sync) {
      setSpriteActionStatus("Enter a shape name", "is-error");
    }
    return false;
  }
  sprite.shapeBind = { type: "shape", name, linked: Boolean(options.sync) };
  if (options.sync) {
    return setSpriteShapeSync(true, name);
  }
  syncSpriteSourceActionButtons();
  return true;
}

function setSpriteShapeSync(sync, rawName) {
  const name = sanitizeSpriteAssetName(rawName || spriteAssetBindInfo(sprite.shapeBind, "shape").name);
  if (!sync) {
    sprite.shapeBind = name ? { type: "shape", name, linked: false } : null;
    renderSpriteBuilder();
    return true;
  }
  if (!name) {
    setSpriteActionStatus("Enter a shape name", "is-error");
    return false;
  }
  const source = activeSpriteEditSource();
  const shapes = parseSpriteShapeAssets(source);
  let status = `Using shape ${name}`;
  if (shapes.has(name)) {
    const parsed = spriteCellsFromAsciiRows(shapes.get(name), sprite.palette.length);
    if (!parsed) {
      setSpriteActionStatus(`Cannot use shape ${name}`, "is-error");
      return false;
    }
    sprite.size = parsed.size;
    sprite.cells = parsed.cells;
  } else {
    status = `Tagged shape ${name}`;
  }
  sprite.shapeBind = { type: "shape", name, linked: true };
  setSpriteActionStatus(status, "is-ok");
  renderSpriteBuilder();
  return true;
}

function spriteCellsFromAsciiRows(rows, paletteLength) {
  if (!Array.isArray(rows) || rows.length === 0) {
    return null;
  }
  const width = Math.max(...rows.map((row) => row.length));
  const height = rows.length;
  const size = clampSpriteSize(Math.max(width, height));
  const cells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < Math.min(height, size); y += 1) {
    for (let x = 0; x < Math.min(rows[y].length, size); x += 1) {
      const colorIndex = spriteColorIndexForPaletteChar(rows[y][x], paletteLength);
      if (colorIndex === undefined) {
        return null;
      }
      cells[y * size + x] = colorIndex;
    }
  }
  return { size, cells };
}

function loadSpriteFromSourceClick(event = null) {
  if (event?.defaultPrevented) {
    return;
  }
  if (typeof syncPreviewModeFromSourceCursor !== "function") {
    setSpriteActionStatus("Source target sync unavailable", "is-error");
    return;
  }
  const source = sourceEditor.value || "";
  const clickOffset = typeof sourceOffsetFromEditorClick === "function"
    ? sourceOffsetFromEditorClick(event, source)
    : null;
  syncPreviewModeFromSourceCursor({
    force: true,
    recordHistory: true,
    allowInactiveMode: true,
    position: clickOffset ?? sourceEditor.selectionStart,
  });
}

function loadSpriteSourceTarget(target, options = {}) {
  if (!isPuzzleDocument(activeDocument()) || !isTextDocument(activeDocument())) {
    return null;
  }
  const source = sourceEditor.value || "";
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return null;
  }
  const targetName = target.name || spriteObjectName();
  const loaded = parseSpriteDefinitionSource(source.slice(target.bodyStart, target.bodyEnd), source, targetName);
  if (!loaded) {
    if (isIncompleteSpriteSourceTarget(source, target)) {
      applyIncompleteSpriteSourceTarget(targetName, target);
      if (!options.silent) {
        setSpriteActionStatus(`Loaded unfinished ${spriteNameInput.value || "sprite"}`, "is-ok");
        setStatus(`Loaded unfinished sprite ${spriteNameInput.value || ""}`.trim(), "is-ok");
      }
      return `sprite:${targetName}:${target.start ?? target.bodyStart}`;
    } else if (!options.silent) {
      setSpriteActionStatus("No editable sprite here", "is-error");
    }
    return null;
  }
  if (options.recordHistory && typeof pushSourceNavigationHistory === "function") {
    pushSourceNavigationHistory();
  }
  if (options.switchMode && currentPreviewMode !== "sprite") {
    setPreviewMode("sprite");
  }
  spriteNameInput.value = targetName || "Sprite";
  setSpriteEditSource(target, activeDocument());
  sprite.size = loaded.size;
  sprite.palette = loaded.palette;
  sprite.paletteBind = loaded.paletteBind || null;
  sprite.shapeBind = loaded.shapeBind || null;
  sprite.solidSource = Boolean(loaded.solid);
  sprite.cells = loaded.cells;
  sprite.selectedColorIndex = sprite.palette.length ? 0 : null;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
  if (!options.silent) {
    setSpriteActionStatus(`Loaded ${spriteNameInput.value}`, "is-ok");
    setStatus(`Loaded sprite ${spriteNameInput.value}`, "is-ok");
  }
  return `sprite:${targetName}:${target.start ?? target.bodyStart}`;
}

function isIncompleteSpriteSourceTarget(source, target) {
  if (!Number.isInteger(target?.bodyStart) || !Number.isInteger(target?.bodyEnd)) {
    return false;
  }
  const body = String(source || "").slice(target.bodyStart, target.bodyEnd);
  const rows = body
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  if (!rows.length) {
    return true;
  }
  if (rows[0].startsWith("palette ")) {
    rows[0] = rows[0].slice("palette ".length).trim();
  }
  return !rows[0]
    .split(/\s+/)
    .filter(Boolean)
    .every((token) => Boolean(spritePaletteEntryFromSourceToken(token, parseSpriteColorAssets(source), target.name || "")));
}

function applyIncompleteSpriteSourceTarget(name, target) {
  if (target && typeof target === "object") {
    setSpriteEditSource(target, activeDocument());
  }
  spriteNameInput.value = name || "";
  sprite.size = clampSpriteSize(sprite.size);
  sprite.palette = [];
  sprite.paletteBind = null;
  sprite.shapeBind = null;
  sprite.solidSource = false;
  sprite.cells = Array.from({ length: sprite.size * sprite.size }, () => null);
  sprite.selectedColorIndex = null;
  sprite.addPaletteOpen = false;
  sprite.editPaletteOpen = false;
  sprite.customColorOpen = false;
  sprite.addDraftColorIndex = null;
  renderSpriteBuilder();
}

function parseSpriteDefinitionSource(body, source = "", selectorName = "") {
  const rows = body
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  if (rows.length < 1) {
    return null;
  }
  const colorAssets = parseSpriteColorAssets(source);
  let paletteBind = null;
  let shapeBind = null;
  let paletteRows = rows;
  if (paletteRows[0].startsWith("palette ")) {
    paletteRows = [paletteRows[0].slice("palette ".length).trim(), ...paletteRows.slice(1)];
  }
  const paletteAssets = parseSpritePaletteAssets(source);
  const paletteRef = paletteRows[0].split(/\s+/).filter(Boolean);
  const palette = paletteRef.length === 1 && spritePaletteAssetHasReference(paletteRef[0], paletteAssets, selectorName)
    ? resolveSpritePaletteAssetToken(paletteRef[0], paletteAssets, colorAssets, selectorName)
    : paletteRows[0]
      .split(/\s+/)
      .map((token) => spritePaletteEntryFromSourceToken(token, colorAssets, selectorName));
  if (paletteRef.length === 1 && spritePaletteAssetHasReference(paletteRef[0], paletteAssets, selectorName)) {
    paletteBind = { type: "palette", name: paletteRef[0], linked: true };
  }
  if (!palette.length || palette.some((entry) => !entry)) {
    return null;
  }
  let asciiRows = paletteRows.slice(1);
  const shapeAssets = parseSpriteShapeAssets(source);
  if (asciiRows[0]?.startsWith("shape ")) {
    const shapeName = asciiRows[0].slice("shape ".length).trim();
    const shapeRows = resolveSpriteShapeAssetToken(shapeName, shapeAssets, selectorName);
    if (!shapeRows) {
      return null;
    }
    shapeBind = { type: "shape", name: shapeName, linked: true };
    asciiRows = shapeRows;
  } else if (asciiRows.length === 1) {
    const shapeName = asciiRows[0].trim();
    const shapeRows = resolveSpriteShapeAssetToken(shapeName, shapeAssets, selectorName);
    if (shapeRows) {
      shapeBind = { type: "shape", name: shapeName, linked: true };
      asciiRows = shapeRows;
    }
  }
  if (asciiRows.length === 0) {
    if (palette.length !== 1) {
      return null;
    }
    const size = SOLID_SPRITE_EDITOR_SIZE;
    return {
      size,
      palette,
      paletteBind,
      shapeBind: null,
      solid: true,
      cells: Array.from({ length: size * size }, () => 0),
    };
  }
  const width = Math.max(...asciiRows.map((row) => row.length));
  const height = asciiRows.length;
  const size = clampSpriteSize(Math.max(width, height));
  const cells = Array.from({ length: size * size }, () => null);
  for (let y = 0; y < Math.min(height, size); y += 1) {
    for (let x = 0; x < Math.min(asciiRows[y].length, size); x += 1) {
      const colorIndex = spriteColorIndexForPaletteChar(asciiRows[y][x], palette.length);
      if (colorIndex === undefined) {
        return null;
      }
      cells[y * size + x] = colorIndex;
    }
  }
  return {
    size,
    palette,
    paletteBind,
    shapeBind,
    cells,
  };
}

function spritePaletteEntryFromSourceToken(token, colorAssets = null, selectorName = "") {
  const color = parseSpriteHexColor(token);
  if (color) {
    return { color };
  }
  const resolved = resolveSpriteColorAssetToken(token, colorAssets, [], selectorName);
  if (!resolved) {
    return null;
  }
  return {
    color: resolved,
    bind: { type: "color", name: token, linked: true },
  };
}

function spritePaletteEntrySourceToken(entry) {
  const bind = spritePaletteEntryBindInfo(entry);
  if (bind.linked && bind.name) {
    return bind.name;
  }
  return normalizeSpriteColor(entry.color);
}

function parseSpriteColorAssets(source) {
  const raw = new Map();
  const spritesBlock = findSpritesBlock(source);
  const colorsBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "colors") : null;
  if (!colorsBlock) {
    return raw;
  }
  collectSpriteFlatAssetRows(source, colorsBlock, (name, value) => {
    raw.set(name, value);
  });
  collectSpriteAssetTables(source, colorsBlock, (tableName, rowName, value) => {
    raw.set(`${tableName}:${rowName}`, value);
  });
  return raw;
}

function parseSpritePaletteAssets(source) {
  const raw = new Map();
  const spritesBlock = findSpritesBlock(source);
  const palettesBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "palettes") : null;
  if (!palettesBlock) {
    return raw;
  }
  collectSpriteFlatAssetRows(source, palettesBlock, (name, value) => {
    raw.set(name, value.split(/\s+/).filter(Boolean));
  });
  collectSpriteAssetTables(source, palettesBlock, (tableName, rowName, value) => {
    const tokens = value.split(/\s+/).filter(Boolean);
    raw.set(`${tableName}:${rowName}`, tokens);
  });
  return raw;
}

function spritePaletteAssetHasReference(token, paletteAssets, selectorName = "") {
  return Boolean(spriteTableAssetKey(token, paletteAssets, selectorName));
}

function resolveSpritePaletteAssetToken(token, paletteAssets, colorAssets, selectorName = "") {
  const key = spriteTableAssetKey(token, paletteAssets, selectorName);
  const names = key ? paletteAssets.get(key) || [] : [];
  return names.map((name) => spritePaletteEntryFromSourceToken(name, colorAssets, selectorName));
}

function parseSpriteShapeAssets(source) {
  const raw = new Map();
  const spritesBlock = findSpritesBlock(source);
  const shapesBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "shapes") : null;
  if (!shapesBlock) {
    return raw;
  }
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const tablePattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*):([A-Za-z_][\w]*)[^\n{]*\{/g;
  let tableMatch = null;
  while ((tableMatch = tablePattern.exec(body))) {
    const bodyMatchStart = tableMatch.index + tableMatch[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", shapesBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > shapesBlock.bodyEnd) {
      continue;
    }
    const tableBody = source.slice(openIndex + 1, closeIndex);
    collectSpriteShapeTableRows(source, openIndex + 1, closeIndex, tableMatch[3], tableBody, raw);
    if (!tableBody.includes("{")) {
      const rows = tableBody
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);
      if (rows.length) {
        raw.set(`${tableMatch[3]}:${tableMatch[4]}`, rows);
      }
    }
  }

  const pattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*\{/g;
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", shapesBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > shapesBlock.bodyEnd) {
      continue;
    }
    const rows = source.slice(openIndex + 1, closeIndex)
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    if (rows.length) {
      raw.set(match[3], rows);
    }
  }
  return raw;
}

function collectSpriteShapeTableRows(source, bodyStart, bodyEnd, tableName, tableBody, raw) {
  const rowPattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*\{/g;
  let match = null;
  while ((match = rowPattern.exec(tableBody))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(tableBody, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > bodyEnd) {
      continue;
    }
    const rows = source.slice(openIndex + 1, closeIndex)
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    if (!rows.length) {
      continue;
    }
    raw.set(`${tableName}:${match[3]}`, rows);
  }
}

function resolveSpriteShapeAssetToken(token, shapeAssets, selectorName = "") {
  const key = spriteTableAssetKey(token, shapeAssets, selectorName);
  return key ? shapeAssets.get(key) : null;
}

function spriteTableAssetKey(token, assets, selectorName = "") {
  const name = String(token || "").trim();
  if (!name) {
    return "";
  }
  if (assets.has(name)) {
    return name;
  }
  const separator = name.indexOf(":");
  if (separator < 1) {
    return "";
  }
  const tableName = name.slice(0, separator);
  const selectorValue = spriteSelectorSingleTagValue(selectorName, name.slice(separator + 1));
  if (selectorValue && assets.has(`${tableName}:${selectorValue}`)) {
    return `${tableName}:${selectorValue}`;
  }
  const selectorBinding = spriteSelectorSingleTagBinding(selectorName, name.slice(separator + 1));
  if (selectorBinding) {
    return firstSpriteTableAssetKey(tableName, assets);
  }
  return "";
}

function spriteSelectorSingleTagValue(selectorName, bindingName = "") {
  const parts = String(selectorName || "").split(":").filter(Boolean);
  if (parts.length !== 2) {
    return "";
  }
  const value = parts[1];
  return value && value !== bindingName ? value : "";
}

function spriteSelectorSingleTagBinding(selectorName, bindingName = "") {
  const parts = String(selectorName || "").split(":").filter(Boolean);
  if (parts.length !== 2 || !bindingName) {
    return "";
  }
  const value = parts[1];
  return value === bindingName ? value : "";
}

function firstSpriteTableAssetKey(tableName, assets) {
  const prefix = `${tableName}:`;
  for (const key of assets.keys()) {
    if (key.startsWith(prefix)) {
      return key;
    }
  }
  return "";
}

function resolveSpriteColorAssetToken(token, colorAssets = null, stack = [], selectorName = "") {
  const direct = parseSpriteHexColor(token);
  if (direct) {
    return direct;
  }
  const assets = colorAssets || parseSpriteColorAssets(activePreviewSource());
  const name = String(token || "").trim();
  const key = assets.has(name) ? name : spriteTableAssetKey(name, assets, selectorName);
  if (!name || stack.includes(name) || !key || !assets.has(key)) {
    return null;
  }
  const raw = String(assets.get(key) || "").trim().split(/\s+/)[0];
  return resolveSpriteColorAssetToken(raw, assets, [...stack, name], selectorName);
}

function collectSpriteFlatAssetRows(source, block, callback) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*=\s*([^\n{}]+)/g;
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    callback(match[3], stripSpriteAssetComment(match[4]).trim(), {
      lineStart: block.bodyStart + bodyMatchStart,
      valueStart: block.bodyStart + match.index + match[0].lastIndexOf(match[4]),
      valueEnd: block.bodyStart + match.index + match[0].length,
    });
  }
}

function collectSpriteAssetTables(source, block, callback) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)(?::[A-Za-z_][\w]*)?\s*\{/g;
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", block.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > block.bodyEnd) {
      continue;
    }
    const tableBlock = {
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
    collectSpriteFlatAssetRows(source, tableBlock, (rowName, value, range) => {
      callback(match[3], rowName, value, range);
    });
  }
}

function stripSpriteAssetComment(value) {
  return String(value || "").replace(/\s+\/\/.*$/, "");
}

function spriteColorIndexForPaletteChar(char, paletteLength) {
  if (char === ".") {
    return null;
  }
  const index = SPRITE_COLOR_TOKENS.indexOf(char);
  return index >= 0 && index < paletteLength ? index : undefined;
}

async function exportSpriteAscii() {
  const text = spriteClipboardText();
  try {
    window.focus();
    spriteExportButton.focus({ preventScroll: true });
    await copyTextToClipboard(text);
    setSpriteActionStatus("Copied", "is-ok");
    setStatus("Copied sprite", "is-ok");
  } catch (error) {
    setSpriteActionStatus("Copy failed", "is-error");
    setStatus(`Could not copy sprite: ${error?.message || error}`, "is-error");
  }
}

function addSpriteToSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return;
  }

  const inserted = insertSpriteDefinition(activeSpriteEditSource());
  const source = sourceWithStagedSpriteAssetDefinitions(inserted.source);
  if (!source) {
    return;
  }
  const result = { source, start: inserted.start };
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  setSpriteEditSource({ start: result.start, name: spriteObjectName() }, document);
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Added sprite", "is-ok");
  setStatus("Added sprite", "is-ok");
}

async function addEmptySpriteToSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return false;
  }

  const source = activeSpriteEditSource();
  const cursor = spriteSourceCursorPosition(source, document);
  let target = null;
  try {
    target = await spriteSourceTargetAtCursor(source, cursor);
  } catch (error) {
    setSpriteActionStatus("Source target sync failed", "is-error");
    setStatus(`Source target sync failed: ${userFacingRuntimeError(error)}`, "is-error");
    return false;
  }
  const result = insertEmptySpriteDefinition(source, { cursor, target });
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  applyIncompleteSpriteSourceTarget("", { start: result.start, end: result.end, bodyStart: result.start, bodyEnd: result.end });
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Added unfinished sprite", "is-ok");
  setStatus("Added unfinished sprite", "is-ok");
  return true;
}

function updateSpriteInSource() {
  const document = activeSpriteEditDocument();
  if (!document || !isTextDocument(document)) {
    setSpriteActionStatus("No puzzle source", "is-error");
    setStatus("No puzzle source for sprite", "is-error");
    return;
  }

  const replaced = replaceSpriteDefinition(activeSpriteEditSource());
  if (!replaced) {
    setSpriteActionStatus("No selected sprite source range", "is-error");
    setStatus("No selected sprite source range", "is-error");
    return;
  }
  const stagedSource = sourceWithStagedSpriteAssetDefinitions(replaced.source);
  if (!stagedSource) {
    return;
  }
  const result = { ...replaced, source: stagedSource };
  document.source = result.source;
  if (document.id === activeDocument()?.id) {
    setSourceEditorValue(result.source, { resetUndo: false });
    revealSpriteSourceResult(document, result);
  }
  scheduleLocalSave();
  schedulePreview();
  setSpriteEditSource({ start: result.start, name: spriteObjectName() }, document);
  sourceEditor.focus({ preventScroll: true });
  setSpriteActionStatus("Updated sprite", "is-ok");
  setStatus("Updated sprite", "is-ok");
}

function sourceWithStagedSpriteAssetDefinitions(source) {
  let nextSource = source;
  for (const entry of sprite.palette) {
    const bind = spritePaletteEntryBindInfo(entry);
    if (!bind.linked || !bind.name || findSpriteColorDefinitionRange(nextSource, bind.name)) {
      continue;
    }
    const withColor = ensureSpriteColorDefinition(nextSource, bind.name, normalizeSpriteColor(entry.color));
    if (!withColor) {
      return null;
    }
    nextSource = withColor;
  }

  const shape = spriteAssetBindInfo(sprite.shapeBind, "shape");
  if (shape.linked && shape.name && !findSpriteShapeDefinitionRange(nextSource, shape.name)) {
    const withShape = ensureSpriteShapeDefinition(nextSource, shape.name, spriteAscii().split("\n"));
    if (!withShape) {
      return null;
    }
    nextSource = withShape;
  }
  return nextSource;
}

function clearSpriteBuilder() {
  const before = visualEditSnapshot("sprite");
  resetSpriteBuilder(sprite.size);
  setSpriteActionStatus("Cleared", "is-ok");
  pushVisualEditUndoSnapshot("sprite", before);
}

function setSpriteActionStatus(text, className = "") {
  if (!spriteActionStatus) {
    return;
  }
  window.clearTimeout(spriteActionClearTimer);
  spriteActionStatus.className = `sprite-action-status tool-feedback-bar ${className}`.trim();
  spriteActionStatus.textContent = text;
  setPaneStatus("sprite", text, className);
  if (text && className === "is-ok") {
    spriteActionClearTimer = window.setTimeout(() => {
      if (spriteActionStatus.textContent === text && spriteActionStatus.classList.contains("is-ok")) {
        spriteActionStatus.className = "sprite-action-status tool-feedback-bar";
        spriteActionStatus.textContent = "";
      }
    }, 1800);
  }
}

function clearSpriteActionError() {
  if (!spriteActionStatus?.classList.contains("is-error")) {
    return;
  }
  setSpriteActionStatus("");
}

function insertSpriteDefinition(source) {
  const block = findSpritesBlock(source);
  if (!block) {
    const puzzleBlock = findPuzzleBlock(source);
    if (puzzleBlock) {
      const blockIndent = spriteSourceChildIndent(puzzleBlock.indent);
      const insertStart = source.slice(0, puzzleBlock.bodyEnd).trimEnd().length + 2 + blockIndent.length + "sprites {\n".length;
      return {
        source: `${source.slice(0, puzzleBlock.bodyEnd).trimEnd()}\n\n${blockIndent}sprites {\n${spriteObjectDefinitionText(spriteSourceChildIndent(blockIndent))}\n${blockIndent}}\n${source.slice(puzzleBlock.bodyEnd)}`,
        start: insertStart,
        updated: false,
      };
    }

    const prefix = source.trimEnd() ? `${source.trimEnd()}\n\n` : "";
    return {
      source: `${prefix}sprites {\n${spriteObjectDefinitionText(SPRITE_SOURCE_INDENT)}\n}\n`,
      start: `${prefix}sprites {\n`.length,
      updated: false,
    };
  }

  const indent = spriteSourceChildIndent(block.indent);
  const before = source.slice(0, block.bodyEnd).trimEnd();
  return {
    source: `${before}\n\n${spriteObjectDefinitionText(indent)}\n${source.slice(block.bodyEnd)}`,
    start: before.length + 2,
  };
}

function insertEmptySpriteDefinition(source, options = {}) {
  const block = findSpritesBlock(source);
  const cursor = Number.isInteger(options.cursor) ? options.cursor : spriteSourceCursorPosition(source);
  const target = options.target?.kind === "sprite" ? options.target : null;
  if (!block) {
    const position = spriteSourceInsertionLineEnd(source, cursor);
    const text = "sprites {\n\n}";
    return insertSpriteSourceTextAt(source, position, text, "sprites {\n".length);
  }
  const insideBlock = cursor > block.openIndex && cursor < block.closeIndex;
  const position = target && target.start >= block.bodyStart && target.end <= block.bodyEnd
    ? spriteSourceInsertionLineEnd(source, target.end)
    : insideBlock
      ? spriteSourceInsertionLineEnd(source, cursor)
      : block.bodyEnd;
  const inserted = insertSpriteSourceTextAt(source, position, "", 0);
  return {
    ...inserted,
    source: inserted.source,
  };
}

function spriteSourceCursorPosition(source, document = activeDocument()) {
  if (document?.id === activeDocument()?.id && sourceEditor) {
    return Math.max(
      0,
      Math.min(String(source || "").length, Math.max(sourceEditor.selectionStart || 0, sourceEditor.selectionEnd || 0)),
    );
  }
  return String(source || "").length;
}

async function spriteSourceTargetAtCursor(source, cursor) {
  if (typeof resolveSourceTargetFromWasm !== "function") {
    return null;
  }
  return resolveSourceTargetFromWasm(source, cursor);
}

function spriteSourceInsertionLineEnd(source, position) {
  const text = String(source || "");
  const safePosition = Math.max(0, Math.min(text.length, Math.trunc(Number(position) || 0)));
  const newline = text.indexOf("\n", safePosition);
  return newline < 0 ? text.length : newline + 1;
}

function insertSpriteSourceTextAt(source, position, text, innerOffset = 0) {
  const original = String(source || "");
  const safePosition = Math.max(0, Math.min(original.length, Math.trunc(Number(position) || 0)));
  const before = original.slice(0, safePosition).trimEnd();
  const after = original.slice(safePosition).replace(/^[\t ]*\n?/, "");
  const snippet = String(text || "").trimEnd();
  let next = before;
  if (next) {
    next += "\n\n";
  }
  const start = next.length + Math.max(0, Math.min(snippet.length, innerOffset));
  next += snippet;
  const end = next.length;
  if (after) {
    next += `${snippet ? "\n\n" : "\n"}${after}`;
  } else {
    next += "\n";
  }
  return { source: next, start, end };
}

function replaceSpriteDefinition(source) {
  const entry = currentSpriteEditSourceRange(source);
  if (!entry) {
    return null;
  }
  const replacement = spriteObjectDefinitionText(spriteSourceIndent(entry.indent));
  return {
    source: replaceEditorSourceRangePreservingLineBoundary(source, entry.start, entry.end, replacement),
    start: entry.start,
  };
}

function canReplaceCurrentSpriteDefinition(source) {
  return Boolean(currentSpriteEditSourceRange(source));
}

function syncSpriteSourceActionButtons() {
  if (spriteUpdateButton) {
    spriteUpdateButton.disabled = !canReplaceCurrentSpriteDefinition(activeSpriteEditSource());
  }
}

function currentSpriteEditSourceRange(source) {
  const start = sprite.editSourceStart;
  const end = sprite.editSourceEnd;
  if (
    !Number.isInteger(start)
    || !Number.isInteger(end)
    || start < 0
    || end < start
    || end > String(source || "").length
  ) {
    return null;
  }
  return {
    start,
    end,
    indent: spriteSourceIndent(source.slice(source.lastIndexOf("\n", start - 1) + 1, start)),
  };
}

function revealSpriteSourceResult(document, result) {
  if (!Number.isInteger(result?.start) || typeof revealSourceLocation !== "function") {
    return;
  }
  revealSourceLocation({
    document,
    start: result.start,
    end: Number.isInteger(result.end) ? result.end : result.start,
  }, {
    recordHistory: false,
    revealPane: false,
  });
}

function findSpritesBlock(source) {
  const pattern = /(^|\n)([\t ]*)sprites(?:\s+[^\n{]+)?\s*\{/m;
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const start = match.index + match[1].length;
  const openIndex = source.indexOf("{", start);
  const closeIndex = findMatchingBrace(source, openIndex);
  if (openIndex < 0 || closeIndex < 0) {
    return null;
  }
  return {
    start,
    openIndex,
    closeIndex,
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

function findVisualAssetBlock(source, spritesBlock, name) {
  const body = source.slice(spritesBlock.bodyStart, spritesBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const start = spritesBlock.bodyStart + bodyMatchStart;
    const openIndex = source.indexOf("{", start);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > spritesBlock.bodyEnd) {
      continue;
    }
    return {
      start,
      openIndex,
      closeIndex,
      indent: match[2] || "",
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
  }
  return null;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function ensureSpriteColorDefinition(source, name, color) {
  const spritesBlock = findSpritesBlock(source);
  if (!spritesBlock) {
    setSpriteActionStatus("No sprites block", "is-error");
    return null;
  }
  if (findSpriteColorDefinitionRange(source, name)) {
    setSpriteActionStatus(`${name} already exists`, "is-error");
    return null;
  }
  const normalized = normalizeSpriteColor(color);
  const colorsBlock = findVisualAssetBlock(source, spritesBlock, "colors");
  if (colorsBlock) {
    const rowIndent = spriteSourceChildIndent(colorsBlock.indent);
    return `${source.slice(0, colorsBlock.bodyEnd).trimEnd()}\n${rowIndent}${name} = ${normalized}\n${source.slice(colorsBlock.bodyEnd)}`;
  }
  const blockIndent = spriteSourceChildIndent(spritesBlock.indent);
  const rowIndent = spriteSourceChildIndent(blockIndent);
  const colorsText = `\n${blockIndent}colors {\n${rowIndent}${name} = ${normalized}\n${blockIndent}}\n`;
  return `${source.slice(0, spritesBlock.bodyStart)}${colorsText}${source.slice(spritesBlock.bodyStart)}`;
}

function replaceSpriteColorDefinition(source, name, color) {
  const range = findSpriteColorDefinitionRange(source, name);
  if (!range) {
    return null;
  }
  return `${source.slice(0, range.valueStart)} ${normalizeSpriteColor(color)}${source.slice(range.valueEnd)}`;
}

function ensureSpritePaletteDefinition(source, name, tokens) {
  const spritesBlock = findSpritesBlock(source);
  if (!spritesBlock) {
    setSpriteActionStatus("No sprites block", "is-error");
    return null;
  }
  if (findSpritePaletteDefinitionRange(source, name)) {
    setSpriteActionStatus(`${name} already exists`, "is-error");
    return null;
  }
  const palettesBlock = findVisualAssetBlock(source, spritesBlock, "palettes");
  const row = `${name} = ${tokens.join(" ")}`;
  if (palettesBlock) {
    const rowIndent = spriteSourceChildIndent(palettesBlock.indent);
    return `${source.slice(0, palettesBlock.bodyEnd).trimEnd()}\n${rowIndent}${row}\n${source.slice(palettesBlock.bodyEnd)}`;
  }
  const blockIndent = spriteSourceChildIndent(spritesBlock.indent);
  const rowIndent = spriteSourceChildIndent(blockIndent);
  const text = `\n${blockIndent}palettes {\n${rowIndent}${row}\n${blockIndent}}\n`;
  return `${source.slice(0, spritesBlock.bodyStart)}${text}${source.slice(spritesBlock.bodyStart)}`;
}

function replaceSpritePaletteDefinition(source, name, tokens) {
  const range = findSpritePaletteDefinitionRange(source, name);
  if (!range) {
    return null;
  }
  return `${source.slice(0, range.valueStart)} ${tokens.join(" ")}${source.slice(range.valueEnd)}`;
}

function findSpritePaletteDefinitionRange(source, name) {
  const spritesBlock = findSpritesBlock(source);
  const palettesBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "palettes") : null;
  return palettesBlock ? findSpriteFlatAssetRowRange(source, palettesBlock, name) : null;
}

function ensureSpriteShapeDefinition(source, name, rows) {
  const spritesBlock = findSpritesBlock(source);
  if (!spritesBlock) {
    setSpriteActionStatus("No sprites block", "is-error");
    return null;
  }
  if (findSpriteShapeDefinitionRange(source, name)) {
    setSpriteActionStatus(`${name} already exists`, "is-error");
    return null;
  }
  const shapesBlock = findVisualAssetBlock(source, spritesBlock, "shapes");
  if (shapesBlock) {
    const indent = spriteSourceChildIndent(shapesBlock.indent);
    const rowIndent = spriteSourceChildIndent(indent);
    const text = `\n${indent}${name} {\n${rows.map((row) => `${rowIndent}${row}`).join("\n")}\n${indent}}\n`;
    return `${source.slice(0, shapesBlock.bodyEnd).trimEnd()}${text}${source.slice(shapesBlock.bodyEnd)}`;
  }
  const blockIndent = spriteSourceChildIndent(spritesBlock.indent);
  const shapeIndent = spriteSourceChildIndent(blockIndent);
  const rowIndent = spriteSourceChildIndent(shapeIndent);
  const text = `\n${blockIndent}shapes {\n${shapeIndent}${name} {\n${rows.map((row) => `${rowIndent}${row}`).join("\n")}\n${shapeIndent}}\n${blockIndent}}\n`;
  return `${source.slice(0, spritesBlock.bodyStart)}${text}${source.slice(spritesBlock.bodyStart)}`;
}

function replaceSpriteShapeDefinition(source, name, rows) {
  const range = findSpriteShapeDefinitionRange(source, name);
  if (!range) {
    return null;
  }
  const rangeIndent = spriteSourceIndent(range.indent);
  const indent = spriteSourceChildIndent(rangeIndent);
  const body = `\n${rows.map((row) => `${indent}${row}`).join("\n")}\n${rangeIndent}`;
  return `${source.slice(0, range.bodyStart)}${body}${source.slice(range.bodyEnd)}`;
}

function findSpriteShapeDefinitionRange(source, name) {
  const spritesBlock = findSpritesBlock(source);
  const shapesBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "shapes") : null;
  if (!shapesBlock) {
    return null;
  }
  const tableSeparator = name.indexOf(":");
  if (tableSeparator > 0) {
    const tableName = name.slice(0, tableSeparator);
    const value = spriteSelectorSingleTagValue(spriteObjectName(), name.slice(tableSeparator + 1));
    return findSpriteShapeTableRowRange(source, shapesBlock, tableName, value);
  }
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", shapesBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > shapesBlock.bodyEnd) {
      continue;
    }
    return {
      indent: match[2] || spriteSourceChildIndent(shapesBlock.indent),
      bodyStart: openIndex + 1,
      bodyEnd: closeIndex,
    };
  }
  return null;
}

function findSpriteShapeTableRowRange(source, shapesBlock, tableName, value = "") {
  const body = source.slice(shapesBlock.bodyStart, shapesBlock.bodyEnd);
  const tablePattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(tableName)}:[A-Za-z_][\\w]*\\s*\\{`, "g");
  let tableMatch = null;
  while ((tableMatch = tablePattern.exec(body))) {
    const tableMatchStart = tableMatch.index + tableMatch[1].length;
    if (topLevelDepthAt(body, tableMatchStart) !== 0) {
      continue;
    }
    const tableOpenIndex = source.indexOf("{", shapesBlock.bodyStart + tableMatchStart);
    const tableCloseIndex = findMatchingBrace(source, tableOpenIndex);
    if (tableOpenIndex < 0 || tableCloseIndex < 0 || tableCloseIndex > shapesBlock.bodyEnd) {
      continue;
    }
    const tableBodyStart = tableOpenIndex + 1;
    const tableBody = source.slice(tableBodyStart, tableCloseIndex);
    const rowPattern = /(^|\n)([\t ]*)([A-Za-z_][\w]*)\s*\{/g;
    let firstRange = null;
    let rowMatch = null;
    while ((rowMatch = rowPattern.exec(tableBody))) {
      const rowMatchStart = rowMatch.index + rowMatch[1].length;
      if (topLevelDepthAt(tableBody, rowMatchStart) !== 0) {
        continue;
      }
      const openIndex = source.indexOf("{", tableBodyStart + rowMatchStart);
      const closeIndex = findMatchingBrace(source, openIndex);
      if (openIndex < 0 || closeIndex < 0 || closeIndex > tableCloseIndex) {
        continue;
      }
      const range = {
        indent: rowMatch[2] || spriteSourceChildIndent(tableMatch[2]),
        bodyStart: openIndex + 1,
        bodyEnd: closeIndex,
      };
      if (!firstRange) {
        firstRange = range;
      }
      if (value && rowMatch[3] === value) {
        return range;
      }
    }
    return firstRange;
  }
  return null;
}

function findSpriteColorDefinitionRange(source, name) {
  const spritesBlock = findSpritesBlock(source);
  const colorsBlock = spritesBlock ? findVisualAssetBlock(source, spritesBlock, "colors") : null;
  if (!colorsBlock) {
    return null;
  }
  const tableSeparator = name.indexOf(":");
  if (tableSeparator > 0) {
    return findSpriteColorTableRowRange(source, colorsBlock, name.slice(0, tableSeparator), name.slice(tableSeparator + 1));
  }
  return findSpriteFlatAssetRowRange(source, colorsBlock, name);
}

function findSpriteColorTableRowRange(source, colorsBlock, tableName, rowName) {
  const body = source.slice(colorsBlock.bodyStart, colorsBlock.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(tableName)}(?::[A-Za-z_][\\w]*)?\\s*\\{`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const openIndex = source.indexOf("{", colorsBlock.bodyStart + bodyMatchStart);
    const closeIndex = findMatchingBrace(source, openIndex);
    if (openIndex < 0 || closeIndex < 0 || closeIndex > colorsBlock.bodyEnd) {
      continue;
    }
    const rowRange = findSpriteFlatAssetRowRange(source, { bodyStart: openIndex + 1, bodyEnd: closeIndex }, rowName);
    if (rowRange) {
      return rowRange;
    }
  }
  return null;
}

function findSpriteFlatAssetRowRange(source, block, name) {
  const body = source.slice(block.bodyStart, block.bodyEnd);
  const pattern = new RegExp(`(^|\\n)([\\t ]*)${escapeRegExp(name)}\\s*=\\s*([^\\n{}]+)`, "g");
  let match = null;
  while ((match = pattern.exec(body))) {
    const bodyMatchStart = match.index + match[1].length;
    if (topLevelDepthAt(body, bodyMatchStart) !== 0) {
      continue;
    }
    const lineStart = block.bodyStart + bodyMatchStart;
    const lineEndIndex = source.indexOf("\n", lineStart);
    const lineEnd = lineEndIndex < 0 || lineEndIndex > block.bodyEnd ? block.bodyEnd : lineEndIndex;
    const equalsIndex = source.indexOf("=", lineStart);
    if (equalsIndex < 0 || equalsIndex > lineEnd) {
      continue;
    }
    return {
      lineStart,
      lineEnd,
      valueStart: equalsIndex + 1,
      valueEnd: lineEnd,
    };
  }
  return null;
}

function findPuzzleBlock(source) {
  const pattern = /(^|\n)([\t ]*)puzzle(?:\s+[^\s{]+)?\s*\{/m;
  const match = pattern.exec(source);
  if (!match) {
    return null;
  }
  const openIndex = source.indexOf("{", match.index + match[0].lastIndexOf("puzzle"));
  const closeIndex = findMatchingBrace(source, openIndex);
  if (closeIndex < 0) {
    return null;
  }
  return {
    indent: match[2] || "",
    bodyStart: openIndex + 1,
    bodyEnd: closeIndex,
  };
}

const SPRITE_SOURCE_INDENT = "";

function spriteSourceIndent(indent = "") {
  return String(indent || "").replace(/\t/g, SPRITE_SOURCE_INDENT);
}

function spriteSourceChildIndent(indent = "") {
  return `${spriteSourceIndent(indent)}${SPRITE_SOURCE_INDENT}`;
}

function spriteObjectDefinitionText(indent) {
  const normalizedIndent = spriteSourceIndent(indent);
  const rowIndent = spriteSourceChildIndent(normalizedIndent);
  const paletteInfo = spriteAssetBindInfo(sprite.paletteBind, "palette");
  const shapeInfo = spriteAssetBindInfo(sprite.shapeBind, "shape");
  const colorRow = paletteInfo.linked && paletteInfo.name
    ? paletteInfo.name
    : spritePaletteSourceTokens().join(" ");
  const solidRow = sprite.solidSource ? spriteSolidDefinitionRow(paletteInfo, shapeInfo) : null;
  if (solidRow) {
    return [
      `${normalizedIndent}${spriteObjectName()}`,
      `${rowIndent}${solidRow}`,
    ].join("\n");
  }
  const lines = [
    `${normalizedIndent}${spriteObjectName()}`,
    `${rowIndent}${colorRow}`,
  ];
  if (shapeInfo.linked && shapeInfo.name) {
    lines.push(`${rowIndent}shape ${shapeInfo.name}`);
  } else {
    lines.push(...spriteAscii().split("\n").map((row) => `${rowIndent}${row}`));
  }
  return lines.join("\n");
}

function spriteSolidDefinitionRow(paletteInfo, shapeInfo) {
  if (shapeInfo.linked || sprite.palette.length !== 1 || !sprite.cells.length) {
    return null;
  }
  if (!sprite.cells.every((cell) => cell === 0)) {
    return null;
  }
  if (paletteInfo.linked && paletteInfo.name) {
    return `palette ${paletteInfo.name}`;
  }
  return spritePaletteSourceTokens()[0] || null;
}

function spritePaletteSourceTokens() {
  return sprite.palette.map((entry) => spritePaletteEntrySourceToken(entry));
}

function topLevelDepthAt(text, endIndex) {
  let depth = 0;
  for (let index = 0; index < endIndex; index += 1) {
    if (text[index] === "{") {
      depth += 1;
    } else if (text[index] === "}") {
      depth = Math.max(0, depth - 1);
    }
  }
  return depth;
}

for (const input of [spriteNameInput, spriteSizeInput, spriteScaleInput]) {
  installSelectAllOnFocus(input);
}
spriteSizeInput.addEventListener("change", () => updateSpriteSize(spriteSizeInput.value));
spriteSizeInput.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  updateSpriteSize(spriteSizeInput.value);
});
spriteScaleInput.addEventListener("input", () => {
  clearSpriteActionError();
  renderSpriteControls();
});
spriteScaleInput.addEventListener("keydown", (event) => {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
});
spriteNameInput.addEventListener("input", syncSpriteSourceActionButtons);
sourceEditor.addEventListener("input", () => {
  invalidateSpriteEditSourceForDocument(activeDocument());
  syncSpriteSourceActionButtons();
});
spritePalette.addEventListener("keydown", (event) => {
  const token = event.target.closest(".sprite-token");
  if (!token) {
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    const rawIndex = token.dataset.colorIndex;
    const nextIndex = rawIndex === "erase" ? null : Number(rawIndex);
    selectSpriteColor(nextIndex);
  }
});
spriteBoard.addEventListener("pointerdown", startSpritePaint);
spriteBoard.addEventListener("pointermove", continueSpritePaint);
spriteBoard.addEventListener("pointerup", stopSpritePaint);
spriteBoard.addEventListener("pointercancel", stopSpritePaint);
spriteBoard.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") {
    const mutate = spriteBucketActive ? bucketFillSpriteFromElement : paintSpriteCellFromElement;
    if (withVisualEditHistory("sprite", () => mutate(event.target))) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
});
document.addEventListener("pointerdown", closeSpriteColorEditorFromOutside);
spriteClearButton.addEventListener("click", clearSpriteBuilder);
spriteExportButton.addEventListener("click", exportSpriteAscii);
spriteInsertButton.addEventListener("click", addSpriteToSource);
spriteUpdateButton.addEventListener("click", updateSpriteInSource);
spriteScaleDownButton.addEventListener("click", scaleDownSprite);
spriteScaleUpButton.addEventListener("click", scaleUpSprite);
spriteRotateLeftButton.addEventListener("click", rotateSpriteLeft);
spriteRotateRightButton.addEventListener("click", rotateSpriteRight);
spriteFlipHorizontalButton.addEventListener("click", flipSpriteHorizontal);
spriteFlipVerticalButton.addEventListener("click", flipSpriteVertical);
spriteFillButton.addEventListener("click", toggleSpriteBucketMode);
sourceEditor.addEventListener("click", loadSpriteFromSourceClick);
resetSpriteBuilder();
