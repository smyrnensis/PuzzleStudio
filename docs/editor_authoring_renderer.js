class PuzzleAuthoringRenderer {
  constructor(root, options = {}) {
    this.root = root;
    this.options = options;
    this.appliedThemeClass = "";
    this.domAnimationToken = 0;
  }

  render(scene) {
    window.__PuzzleCurrentAuthoringScene = scene;
    this.lastScene = scene;
    const viewport = this.resolveViewport(scene);
    this.root.style.setProperty("--cols", viewport.width);
    this.root.style.setProperty("--rows", viewport.height);
    this.root.classList.remove("is-canvas-renderer");
    this.root.dataset.viewportX = String(viewport.x);
    this.root.dataset.viewportY = String(viewport.y);
    this.root.dataset.viewportWidth = String(viewport.width);
    this.root.dataset.viewportHeight = String(viewport.height);
    const visuals = this.visuals();
    const hasVisuals = this.hasVisualConfig(visuals) || this.usesVisuals(scene, visuals);
    const grid = this.gridSettings(scene);
    this.root.classList.toggle("has-occupied-cell-grid", grid.occupiedCells);
    this.root.classList.toggle("has-all-cell-grid", grid.allCells);
    if (visuals.boardClass) {
      this.root.classList.toggle(visuals.boardClass, hasVisuals);
    }
    this.applyThemeClass(visuals.themeClass, hasVisuals);
    this.root.replaceChildren();
    this.domAnimationToken += 1;
    for (const cellData of this.viewportCells(scene, viewport)) {
      this.root.append(this.renderCell(cellData, scene));
    }
    this.startDomAnimationLoop();
  }

  resolveViewport(scene) {
    const screen = scene.screen || scene.view || {};
    const viewportSize = screen.viewportSize || { kind: "full" };
    const mode = screen.viewportMode || "paged";
    const focus = this.focusCell(scene, screen.viewportFocus || "Player");
    const previous = this.viewport;

    if (viewportSize.kind === "full") {
      const viewport = { x: 0, y: 0, width: scene.width, height: scene.height };
      this.viewport = viewport;
      return viewport;
    }

    const width = Math.max(1, Number(viewportSize.width || scene.width || 1));
    const height = Math.max(1, Number(viewportSize.height || scene.height || 1));
    if (
      mode === "paged"
      && previous
      && previous.width === width
      && previous.height === height
      && this.viewportContains(previous, focus)
    ) {
      return previous;
    }

    const viewport = mode === "paged"
      ? this.pagedViewport(scene, focus, width, height)
      : this.centeredViewport(scene, focus, width, height);
    this.viewport = viewport;
    return viewport;
  }

  focusCell(scene, objectName) {
    const focusObjects = new Set((scene.screen?.viewportFocusObjects || scene.view?.viewportFocusObjects || [])
      .map((objectId) => Number(objectId))
      .filter((objectId) => Number.isFinite(objectId) && objectId > 0));
    for (const cell of scene.cells || []) {
      if (
        focusObjects.size > 0
        && cell.layers?.some((layer) => focusObjects.has(Number(layer.objectId)))
      ) {
        return cell;
      }
      if (cell.layers?.some((layer) => layer.object === objectName || layer.visual === objectName)) {
        return cell;
      }
    }
    return null;
  }

  viewportContains(viewport, cell) {
    return Boolean(
      cell
      && cell.x >= viewport.x
      && cell.y >= viewport.y
      && cell.x < viewport.x + viewport.width
      && cell.y < viewport.y + viewport.height
    );
  }

  centeredViewport(scene, focus, width, height) {
    const centerX = focus ? focus.x - Math.floor(width / 2) : 0;
    const centerY = focus ? focus.y - Math.floor(height / 2) : 0;
    const maxX = Math.max(0, Number(scene.width || 0) - width);
    const maxY = Math.max(0, Number(scene.height || 0) - height);
    return {
      x: Math.max(0, Math.min(maxX, centerX)),
      y: Math.max(0, Math.min(maxY, centerY)),
      width: Math.min(width, Number(scene.width || width)),
      height: Math.min(height, Number(scene.height || height)),
    };
  }

  pagedViewport(scene, focus, width, height) {
    const viewportX = focus ? Math.floor(focus.x / width) * width : 0;
    const viewportY = focus ? Math.floor(focus.y / height) * height : 0;
    const maxX = Math.max(0, Number(scene.width || 0) - width);
    const maxY = Math.max(0, Number(scene.height || 0) - height);
    return {
      x: Math.max(0, Math.min(maxX, viewportX)),
      y: Math.max(0, Math.min(maxY, viewportY)),
      width: Math.min(width, Number(scene.width || width)),
      height: Math.min(height, Number(scene.height || height)),
    };
  }

  viewportCells(scene, viewport) {
    return (scene.cells || []).filter((cell) =>
      cell.x >= viewport.x
      && cell.y >= viewport.y
      && cell.x < viewport.x + viewport.width
      && cell.y < viewport.y + viewport.height
    );
  }

  renderCell(cellData, scene) {
    const cell = document.createElement("div");
    cell.className = "cell";
    cell.dataset.x = cellData.x;
    cell.dataset.y = cellData.y;
    cell.setAttribute("aria-label", this.cellLabel(cellData));

    const layers = this.sortedLayers(cellData.layers);
    if (this.options.renderMode === "dom" && this.layersUseMerge(layers)) {
      throw new Error("visual merge requires the canvas renderer");
    }
    cell.classList.toggle("has-objects", layers.length > 0);
    for (const layer of layers) {
      if (this.resolveVisual(layer)) {
        cell.classList.add(`has-${layer.visual}`);
      }
    }

    for (const layer of layers) {
      const visual = this.renderLayerVisual(layer);
      if (visual) {
        const cellOrder = this.cellRenderIndex(cellData, scene);
        const priority = this.visualRenderPriority(layer);
        visual.style.zIndex = String((cellOrder * this.visualPriorityCount()) + priority + 1);
        cell.append(visual);
      }
    }

    return cell;
  }

  renderLayerVisual(layer) {
    const visual = this.resolveVisual(layer);
    if (visual) {
      return this.createVisualElement(layer, visual.definition, visual.key);
    }

    return null;
  }

  createVisualElement(layer, definition, visualKey) {
    const visual = document.createElement("span");
    const baseFrame = this.firstVisualFrame(definition);
    const fit = this.visualFit(baseFrame);
    const sampling = this.visualSampling(baseFrame);
    visual.className = `visual visual-fit-${fit.mode} visual-sampling-${sampling} ${definition.className || ""} ${layer.visual} visual-${this.classNameFor(visualKey)}`;
    visual.dataset.object = layer.object;
    visual.dataset.layer = layer.layer;
    const { cols: visualCols, rows: visualRows } = this.visualPatternSize(baseFrame);
    const { cols: boxCols, rows: boxRows } = this.visualDrawBox(baseFrame);
    visual.style.setProperty("--visual-cols", String(visualCols));
    visual.style.setProperty("--visual-rows", String(visualRows));
    visual.style.setProperty("--visual-box-cols", String(boxCols));
    visual.style.setProperty("--visual-box-rows", String(boxRows));
    this.applyDomVisualTransforms(visual, baseFrame, boxCols, boxRows);
    visual.setAttribute("aria-hidden", "true");
    if (this.visualFrameCount(definition) > 1) {
      visual.__visualAnimationDefinition = definition;
    }

    if (baseFrame.source) {
      visual.classList.add("visual-image");
      const source = window.PuzzleAssets.url(baseFrame.source);
      visual.style.backgroundImage = `url("${String(source).replace(/"/g, '\\"')}")`;
      return visual;
    }

    const solidColor = this.solidPatternColor(baseFrame);
    if (solidColor && this.canPaintAsFullCellSolid(baseFrame)) {
      visual.classList.add("visual-solid");
      visual.style.backgroundColor = solidColor;
      return visual;
    }

    visual.classList.add("visual-pattern");
    visual.style.backgroundImage = `url("${this.domPatternDataUrl(baseFrame)}")`;

    return visual;
  }

  startDomAnimationLoop() {
    const token = this.domAnimationToken;
    const animatedVisuals = [...this.root.querySelectorAll(".visual")]
      .filter((visual) => visual.__visualAnimationDefinition);
    if (!animatedVisuals.length) {
      return;
    }
    const draw = () => {
      if (token !== this.domAnimationToken || !this.root.isConnected) {
        return;
      }
      const now = performance.now();
      for (const visual of animatedVisuals) {
        if (!visual.isConnected) {
          continue;
        }
        const definition = visual.__visualAnimationDefinition;
        const frame = this.resolveVisualFrame(definition, this.loopAnimationTimeMs(now, definition));
        this.applyDomVisualFrame(visual, frame);
      }
      requestAnimationFrame(draw);
    };
    requestAnimationFrame(draw);
  }

  applyDomVisualFrame(visual, frame) {
    visual.classList.remove("visual-image", "visual-solid", "visual-pattern");
    for (const className of [...visual.classList]) {
      if (className.startsWith("visual-fit-") || className.startsWith("visual-sampling-")) {
        visual.classList.remove(className);
      }
    }
    const fit = this.visualFit(frame);
    visual.classList.add(`visual-fit-${fit.mode}`, `visual-sampling-${this.visualSampling(frame)}`);
    const { cols: visualCols, rows: visualRows } = this.visualPatternSize(frame);
    const { cols: boxCols, rows: boxRows } = this.visualDrawBox(frame);
    visual.style.setProperty("--visual-cols", String(visualCols));
    visual.style.setProperty("--visual-rows", String(visualRows));
    visual.style.setProperty("--visual-box-cols", String(boxCols));
    visual.style.setProperty("--visual-box-rows", String(boxRows));
    this.applyDomVisualTransforms(visual, frame, boxCols, boxRows);
    visual.style.backgroundColor = "";
    visual.style.backgroundImage = "";

    if (frame.source) {
      visual.classList.add("visual-image");
      const source = window.PuzzleAssets.url(frame.source);
      visual.style.backgroundImage = `url("${String(source).replace(/"/g, '\\"')}")`;
      return;
    }

    const solidColor = this.solidPatternColor(frame);
    if (solidColor && this.canPaintAsFullCellSolid(frame)) {
      visual.classList.add("visual-solid");
      visual.style.backgroundColor = solidColor;
      return;
    }

    visual.classList.add("visual-pattern");
    visual.style.backgroundImage = `url("${this.domPatternDataUrl(frame)}")`;
  }

  loopAnimationTimeMs(now, definition) {
    const epoch = this.constructor.renderClockEpochMs ?? (this.constructor.renderClockEpochMs = now);
    const phaseMs = Number(definition.phaseMs ?? definition.timing?.phaseMs ?? definition.animation?.phaseMs) || 0;
    return Math.max(0, now - epoch + phaseMs);
  }

  resolveVisualFrame(definition, localTimeMs, timingOverride = null) {
    const frames = this.visualFrames(definition);
    if (frames.length <= 1) {
      return frames[0];
    }
    const durationMs = this.animationDefinitionDurationMs(definition, timingOverride?.durationMs);
    const playback = timingOverride?.playback
      || definition.playback
      || definition.timing?.playback
      || definition.animation?.playback
      || "loop";
    const wrappedTime = playback === "loop"
      ? this.positiveModulo(localTimeMs, durationMs)
      : Math.min(Math.max(0, localTimeMs), Math.max(0, durationMs - 0.0001));
    const index = Math.min(frames.length - 1, Math.floor((wrappedTime / durationMs) * frames.length));
    return frames[index];
  }

  visualFrames(definition) {
    if (Array.isArray(definition.frames) && definition.frames.length) {
      return definition.frames.map((frame) => this.visualFrameFrom(definition, frame));
    }
    return [this.visualFrameFrom(definition, definition)];
  }

  firstVisualFrame(definition) {
    return this.visualFrames(definition)[0];
  }

  visualFrameFrom(definition, frame) {
    if (Array.isArray(frame)) {
      return { ...definition, frames: undefined, pattern: frame };
    }
    return {
      ...definition,
      ...frame,
      frames: undefined,
      colors: frame.colors || definition.colors || {},
      fit: frame.fit || definition.fit,
      transforms: frame.transforms || definition.transforms || [],
      sampling: frame.sampling || definition.sampling,
    };
  }

  applyDomVisualTransforms(visual, definition, boxCols, boxRows) {
    const transforms = Array.isArray(definition?.transforms) ? definition.transforms : [];
    const css = [...transforms].reverse().map((transform) => {
      if (transform?.kind === "rotate") {
        return `rotate(${-Number(transform.degrees || 0)}deg)`;
      }
      if (transform?.kind === "translate") {
        const x = (Number(transform.x) || 0) * 100 / Math.max(1, Number(boxCols) || 1);
        const y = (Number(transform.y) || 0) * 100 / Math.max(1, Number(boxRows) || 1);
        return `translate(${x}%, ${y}%)`;
      }
      if (transform?.kind === "flip") {
        return transform.enabled ? "scale(-1, -1)" : "";
      }
      throw new Error(`Unknown visual transform kind: ${String(transform?.kind)}`);
    });
    visual.style.transformOrigin = "50% 50%";
    visual.style.transform = css.join(" ");
  }

  visualFrameCount(definition) {
    return Math.max(1, Array.isArray(definition.frames) ? definition.frames.length : 1);
  }

  animationDefinitionDurationMs(definition, override = null) {
    return Math.max(
      1,
      Number(
        this.durationMsValue(override)
          ?? this.durationMsValue(definition.durationMs)
          ?? this.durationMsValue(definition.duration)
          ?? this.durationMsValue(definition.timing?.durationMs)
          ?? this.durationMsValue(definition.timing?.duration)
          ?? this.durationMsValue(definition.animation?.durationMs)
          ?? this.durationMsValue(definition.animation?.duration)
          ?? this.durationMsValue(this.visuals().animationDefaults?.durationMs)
          ?? this.durationMsValue(this.visuals().animationDefaults?.duration)
          ?? 250,
      ) || 250,
    );
  }

  durationMsValue(value) {
    if (value == null || value === "") {
      return null;
    }
    const number = Number(value);
    if (Number.isFinite(number)) {
      return number;
    }
    const match = String(value).trim().match(/^([0-9]+(?:\.[0-9]+)?)(ms|s)$/i);
    if (!match) {
      return null;
    }
    const amount = Number(match[1]);
    return match[2].toLowerCase() === "s" ? amount * 1000 : amount;
  }

  positiveModulo(value, size) {
    return ((value % size) + size) % size;
  }

  domPatternDataUrl(definition) {
    const key = JSON.stringify([definition.pattern || [], definition.colors || {}]);
    const cache = this.constructor.domPatternDataUrlCache
      || (this.constructor.domPatternDataUrlCache = new Map());
    const existing = cache.get(key);
    if (existing) {
      return existing;
    }
    const { cols: width, rows: height } = this.visualPatternSize(definition);
    const bitmap = document.createElement("canvas");
    bitmap.width = width;
    bitmap.height = height;
    const bitmapContext = bitmap.getContext("2d");
    bitmapContext.imageSmoothingEnabled = false;
    const pattern = definition.pattern || [];
    pattern.forEach((row, rowIndex) => {
      [...row].forEach((token, colIndex) => {
        const color = definition.colors?.[token] || "transparent";
        if (!color || color === "transparent") {
          return;
        }
        bitmapContext.fillStyle = color;
        bitmapContext.fillRect(colIndex, rowIndex, 1, 1);
      });
    });
    const url = bitmap.toDataURL("image/png");
    cache.set(key, url);
    return url;
  }

  solidPatternColor(definition) {
    const firstToken = definition.pattern?.[0]?.[0];
    if (!firstToken) {
      return null;
    }
    const isSolid = definition.pattern.every((row) =>
      [...row].every((token) => token === firstToken),
    );
    if (!isSolid) {
      return null;
    }
    const color = definition.colors[firstToken];
    return color && color !== "transparent" ? color : null;
  }

  canPaintAsFullCellSolid(definition) {
    if (definition.pixelsPerCell) {
      return false;
    }
    const fit = this.visualFit(definition);
    if (fit.width !== 1 || fit.height !== 1) {
      return false;
    }
    if (fit.mode !== "contain") {
      return true;
    }
    const pattern = this.visualPatternSize(definition);
    return pattern.cols === pattern.rows;
  }

  visualPatternSize(definition) {
    const pattern = definition.pattern || [];
    return {
      cols: Math.max(1, ...pattern.map((row) => String(row).length), 1),
      rows: Math.max(1, pattern.length || 1),
    };
  }

  visualDrawBox(definition) {
    const fit = this.visualFit(definition);
    return {
      cols: Math.max(1, Number(fit.width) || 1),
      rows: Math.max(1, Number(fit.height) || 1),
    };
  }

  visualFit(definition) {
    const fit = definition.fit || {};
    const mode = ["contain", "cover", "stretch"].includes(fit.mode) ? fit.mode : "contain";
    return {
      mode,
      width: Math.max(1, Number(fit.width) || 1),
      height: Math.max(1, Number(fit.height) || 1),
    };
  }

  visualSampling(definition) {
    if (definition.sampling === "smooth" || definition.sampling === "pixelated") {
      return definition.sampling;
    }
    return definition.source && !/\.png$/i.test(definition.source) ? "smooth" : "pixelated";
  }

  sortedLayers(layers) {
    return [...layers].sort((a, b) =>
      this.visualRenderPriority(a) - this.visualRenderPriority(b)
      || Number(a.objectId) - Number(b.objectId)
    );
  }

  visualOrder() {
    const order = this.visuals().order;
    if (!order || !Array.isArray(order.direction_priority) || !Array.isArray(order.priorities)) {
      throw new Error("compiled visual order contract is missing");
    }
    return order;
  }

  visualPriorityCount() {
    return Math.max(1, this.visualOrder().priorities.length);
  }

  layersUseMerge(layers) {
    return layers.some((layer) => this.visualOrder().priorities[this.visualRenderPriority(layer)]?.merge);
  }

  visualRenderPriority(layer) {
    const name = String(layer.object || "");
    const priority = this.visualOrder().priorities.findIndex((entry) =>
      Array.isArray(entry.objects) && entry.objects.includes(name)
    );
    if (priority < 0) {
      throw new Error(`compiled visual order does not cover object: ${name}`);
    }
    return priority;
  }

  cellRenderIndex(cell, scene) {
    const width = Math.max(1, Number(scene?.width) || 1);
    const height = Math.max(1, Number(scene?.height) || 1);
    const coordinates = {
      right: [Number(cell.x), width],
      left: [width - 1 - Number(cell.x), width],
      down: [Number(cell.y), height],
      up: [height - 1 - Number(cell.y), height],
    };
    let index = 0;
    for (const direction of this.visualOrder().direction_priority) {
      const coordinate = coordinates[direction];
      if (!coordinate) {
        throw new Error(`invalid 2D visual order direction: ${direction}`);
      }
      index = (index * coordinate[1]) + coordinate[0];
    }
    return index;
  }

  usesVisuals(scene, visuals) {
    return scene.cells.some((cell) =>
      cell.layers.some((layer) => Boolean(this.resolveVisual(layer, visuals))),
    );
  }

  hasVisualConfig(visuals) {
    return Boolean(
      visuals.boardClass
        || visuals.themeClass
        || Object.keys(visuals.entries || {}).length,
    );
  }

  applyThemeClass(themeClass, active) {
    if (this.options.applyTheme === false) {
      return;
    }

    const target = this.options.themeRoot || document.body;
    if (this.appliedThemeClass && this.appliedThemeClass !== themeClass) {
      target.classList.remove(this.appliedThemeClass);
    }

    if (themeClass) {
      target.classList.toggle(themeClass, active);
      this.appliedThemeClass = active ? themeClass : "";
    } else {
      this.appliedThemeClass = "";
    }
  }

  resolveVisual(layer, registry = this.visuals()) {
    const entries = registry.entries || {};
    for (const key of this.visualKeys(layer, registry.aliases || {})) {
      if (entries[key]) {
        return { key, definition: entries[key] };
      }
    }
    return null;
  }

  visualKeys(layer, aliases) {
    const keys = [];
    const add = (key) => {
      if (key && !keys.includes(key)) {
        keys.push(key);
      }
    };
    const addWithAlias = (key) => {
      add(key);
      const alias = aliases[key];
      if (Array.isArray(alias)) {
        alias.forEach(add);
      } else {
        add(alias);
      }
    };

    addWithAlias(layer.object);
    addWithAlias(layer.visual);

    return keys;
  }

  classNameFor(value) {
    return String(value || "")
      .replace(/[^a-zA-Z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "unknown";
  }

  visuals() {
    return window.GameVisuals || {};
  }

  gridSettings(scene) {
    const raw = scene.settings?.grid || scene.render?.grid || scene.screen?.grid;
    if (!raw || raw === false || raw === true) {
      return { occupiedCells: false };
    }
    return {
      occupiedCells: Boolean(raw.occupied_cells ?? raw.occupiedCells),
      allCells: Boolean(raw.all_cells ?? raw.allCells),
      color: raw.color,
    };
  }

  cellLabel(cellData) {
    if (cellData.layers.length === 0) {
      return `empty ${cellData.x},${cellData.y}`;
    }

    return this.sortedLayers(cellData.layers)
      .map((layer) => `${layer.object} layer ${layer.layer}`)
      .join(", ");
  }
}

window.PuzzleAuthoringRenderer = PuzzleAuthoringRenderer;
