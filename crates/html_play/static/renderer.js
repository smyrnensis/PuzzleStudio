class PuzzleRenderer {
  constructor(root, options = {}) {
    this.root = root;
    this.options = options;
    this.appliedThemeClass = "";
  }

  render(scene) {
    window.__PuzzleCurrentScene = scene;
    this.lastScene = scene;
    const viewport = this.resolveViewport(scene);
    this.root.style.setProperty("--cols", viewport.width);
    this.root.style.setProperty("--rows", viewport.height);
    this.root.classList.toggle("is-canvas-renderer", this.options.renderMode !== "dom");
    this.root.dataset.viewportX = String(viewport.x);
    this.root.dataset.viewportY = String(viewport.y);
    this.root.dataset.viewportWidth = String(viewport.width);
    this.root.dataset.viewportHeight = String(viewport.height);
    const visuals = this.visuals();
    const hasVisuals = this.hasVisualConfig(visuals) || this.usesVisualSprites(scene, visuals);
    const grid = this.gridSettings(scene);
    this.root.classList.toggle("has-occupied-cell-grid", grid.occupiedCells);
    this.root.classList.toggle("has-all-cell-grid", grid.allCells);
    if (visuals.boardClass) {
      this.root.classList.toggle(visuals.boardClass, hasVisuals);
    }
    this.applyThemeClass(visuals.themeClass, hasVisuals);
    this.root.replaceChildren();

    if (this.options.renderMode === "dom") {
      for (const cellData of this.viewportCells(scene, viewport)) {
        this.root.append(this.renderCell(cellData, scene));
      }
    } else {
      this.renderCanvas(scene, viewport);
    }

    window.PuzzleStudio?.dispatchRender({ scene, board: this.root });
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
      if (cell.layers?.some((layer) => layer.object === objectName || layer.sprite === objectName)) {
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

  frameCells(scene, frame) {
    return this.viewportCells(scene, frame);
  }

  renderCell(cellData, scene) {
    const cell = document.createElement("div");
    cell.className = "cell";
    cell.dataset.x = cellData.x;
    cell.dataset.y = cellData.y;
    cell.setAttribute("aria-label", this.cellLabel(cellData));

    const layers = this.sortedLayers(cellData.layers);
    cell.classList.toggle("has-objects", layers.length > 0);
    for (const layer of layers) {
      if (this.resolveVisualSprite(layer)) {
        cell.classList.add(`has-${layer.sprite}`);
      }
    }

    for (const layer of layers) {
      const sprite = this.renderSprite(layer);
      if (sprite) {
        cell.append(sprite);
      }
    }

    return cell;
  }

  renderSprite(layer) {
    const visualSprite = this.resolveVisualSprite(layer);
    if (visualSprite) {
      return this.renderVisualSprite(layer, visualSprite.definition, visualSprite.key);
    }

    return null;
  }

  renderVisualSprite(layer, definition, visualKey) {
    const sprite = document.createElement("span");
    sprite.className = `sprite visual-sprite ${definition.className || ""} ${layer.sprite} visual-${this.classNameFor(visualKey)}`;
    sprite.dataset.object = layer.object;
    sprite.dataset.layer = layer.layer;
    sprite.style.zIndex = String(definition.zIndex ?? layer.layer + 1);
    const spriteCols = Math.max(1, definition.pattern?.[0]?.length || 1);
    const spriteRows = Math.max(1, definition.pattern?.length || 1);
    sprite.style.setProperty("--sprite-cols", String(spriteCols));
    sprite.style.setProperty("--sprite-rows", String(spriteRows));
    sprite.style.setProperty("--sprite-cell-cols", String(definition.pixelsPerCell?.width || spriteCols));
    sprite.style.setProperty("--sprite-cell-rows", String(definition.pixelsPerCell?.height || spriteRows));
    sprite.style.setProperty("--sprite-offset-x", String(Number(definition.offset?.x) || 0));
    sprite.style.setProperty("--sprite-offset-y", String(Number(definition.offset?.y) || 0));
    sprite.setAttribute("aria-hidden", "true");

    if (definition.source) {
      sprite.classList.add("visual-image");
      const source = window.PuzzleAssets?.url(definition.source) || definition.source;
      sprite.style.backgroundImage = `url("${String(source).replace(/"/g, '\\"')}")`;
      return sprite;
    }

    const solidColor = this.solidPatternColor(definition);
    if (solidColor) {
      sprite.classList.add("visual-solid");
      sprite.style.backgroundColor = solidColor;
      return sprite;
    }

    sprite.classList.add("visual-pattern");
    sprite.style.backgroundImage = `url("${this.patternDataUrl(definition)}")`;

    return sprite;
  }

  renderCanvas(scene, frame) {
    const canvas = document.createElement("canvas");
    canvas.className = "board-canvas";
    const unit = this.canvasCellUnit(scene, frame);
    canvas.width = Math.max(1, frame.width * unit);
    canvas.height = Math.max(1, frame.height * unit);
    canvas.setAttribute("aria-label", this.boardLabel(scene, frame));
    const context = canvas.getContext("2d");
    context.imageSmoothingEnabled = false;
    const animations = this.prepareAnimations(scene.animationEvents || [], frame);
    let startedAt = null;
    const duration = this.animationDurationMs(scene);
    const draw = () => {
      if (!this.root.isConnected) {
        requestAnimationFrame(draw);
        return;
      }
      startedAt ??= performance.now();
      const progress = animations.length
        ? Math.min(1, Math.max(0, (performance.now() - startedAt) / duration))
        : 1;
      context.clearRect(0, 0, canvas.width, canvas.height);
      this.paintCanvas(context, scene, frame, unit, animations, progress);
      if (progress < 1 && this.root.isConnected) {
        requestAnimationFrame(draw);
      }
    };
    this.root.append(canvas);
    requestAnimationFrame(draw);
  }

  paintCanvas(context, scene, frame, unit, animations = [], progress = 1) {
    context.save();
    context.imageSmoothingEnabled = false;
    const floorColor = this.canvasFloorColor();
    if (floorColor && floorColor !== "transparent") {
      context.fillStyle = floorColor;
      context.fillRect(0, 0, frame.width * unit, frame.height * unit);
    }

    const animatedLayers = [];
    for (const cell of this.frameCells(scene, frame)) {
      const x = (cell.x - frame.x) * unit;
      const y = (cell.y - frame.y) * unit;
      for (const layer of this.sortedLayers(cell.layers)) {
        const animation = this.animationForLayer(animations, cell, layer);
        if (animation) {
          animatedLayers.push({ layer, x, y, animation });
          continue;
        }
        this.paintCanvasLayer(context, layer, x, y, unit, null, progress);
      }
    }
    for (const item of animatedLayers) {
      this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress);
    }
    this.paintCanvasGrid(context, scene, frame, unit);
    context.restore();
  }

  paintCanvasGrid(context, scene, frame, unit) {
    const grid = this.gridSettings(scene);
    if (!grid.occupiedCells && !grid.allCells) {
      return;
    }
    context.save();
    context.strokeStyle = grid.color || "rgba(30, 41, 59, 0.34)";
    context.lineWidth = Math.max(1, Math.floor(unit / 24));
    context.translate(0.5, 0.5);
    for (const cell of this.frameCells(scene, frame)) {
      if (!grid.allCells && !cell.layers?.length) {
        continue;
      }
      const x = (cell.x - frame.x) * unit;
      const y = (cell.y - frame.y) * unit;
      context.strokeRect(x, y, Math.max(1, unit - 1), Math.max(1, unit - 1));
    }
    context.restore();
  }

  prepareAnimations(events, frame) {
    return (events || [])
      .map((event) => ({ ...event, objectId: Number(event.objectId) }))
      .filter((event) => {
        if (!Number.isFinite(event.objectId) || event.objectId <= 0) {
          return false;
        }
        const x = Number(event.toX ?? event.x);
        const y = Number(event.toY ?? event.y);
        return x >= frame.x && y >= frame.y && x < frame.x + frame.width && y < frame.y + frame.height;
      });
  }

  animationDurationMs(scene) {
    const tween = scene?.settings?.animation?.tween || scene?.animation?.tween || null;
    return Math.max(1, Number(tween?.intervalMs || scene?.animationDurationMs || 250));
  }

  animationForLayer(animations, cell, layer) {
    const objectId = Number(layer.objectId);
    return animations.find((event) => {
      if (event.objectId !== objectId) {
        return false;
      }
      if (event.kind === "move") {
        return Number(event.toX) === cell.x && Number(event.toY) === cell.y;
      }
      return Number(event.x) === cell.x && Number(event.y) === cell.y;
    }) || null;
  }

  paintCanvasLayer(context, layer, x, y, unit, animation = null, progress = 1) {
    const visualSprite = this.resolveVisualSprite(layer);
    const definition = visualSprite?.definition;
    const transform = this.animationTransform(animation, progress, unit);
    if (transform) {
      context.save();
      context.globalAlpha *= transform.alpha;
      context.translate(x + unit / 2 + transform.x, y + unit / 2 + transform.y);
      context.rotate(transform.angle);
      context.scale(transform.scale, transform.scale);
      x = -unit / 2;
      y = -unit / 2;
    }
    if (!definition) {
      if (transform) {
        context.restore();
      }
      return;
    }

    if (definition.source) {
      const image = this.cachedImage(definition.source);
      if (image?.complete && image.naturalWidth > 0) {
        const offset = this.visualSpriteOffset(definition, unit);
        context.drawImage(image, x + offset.x, y + offset.y, unit, unit);
      }
      if (transform) {
        context.restore();
      }
      return;
    }

    const solidColor = this.solidPatternColor(definition);
    if (solidColor) {
      context.fillStyle = solidColor;
      context.fillRect(x, y, unit, unit);
      if (transform) {
        context.restore();
      }
      return;
    }

    const offset = this.visualSpriteOffset(definition, unit);
    this.paintPattern(context, definition, x + offset.x, y + offset.y, unit);
    if (transform) {
      context.restore();
    }
  }

  animationTransform(animation, progress, unit) {
    if (!animation) {
      return null;
    }
    const parts = String(animation.name || "slide").split(":").filter(Boolean);
    const names = new Set(parts.map((part) => part.split("=")[0]));
    const eased = progress;
    const transform = { x: 0, y: 0, scale: 1, alpha: 1, angle: 0 };
    if (names.has("slide") || names.has("tween") || parts.length === 0) {
      if (animation.kind === "move") {
        transform.x += (Number(animation.fromX) - Number(animation.toX)) * unit * (1 - eased);
        transform.y += (Number(animation.fromY) - Number(animation.toY)) * unit * (1 - eased);
      } else {
        transform.x += Math.sin(eased * Math.PI * 2) * unit * 0.12;
      }
    }
    if (names.has("zoom")) {
      transform.scale = animation.kind === "move"
        ? 0.85 + 0.15 * eased
        : 1 + Math.sin(eased * Math.PI) * 0.18;
    }
    if (names.has("fade")) {
      transform.alpha = animation.kind === "move"
        ? 0.35 + 0.65 * eased
        : 1 - Math.sin(eased * Math.PI) * 0.45;
    }
    if (names.has("turn")) {
      transform.angle = (animation.kind === "move" ? 1 - eased : Math.sin(eased * Math.PI)) * Math.PI * 0.5;
    }
    return transform;
  }

  paintPattern(context, definition, x, y, unit) {
    const pattern = definition.pattern || [];
    const rows = Math.max(1, pattern.length || 1);
    const cols = Math.max(1, pattern[0]?.length || 1);
    const cellCols = Math.max(1, Number(definition.pixelsPerCell?.width) || cols);
    const cellRows = Math.max(1, Number(definition.pixelsPerCell?.height) || rows);
    const pixelWidth = unit / cellCols;
    const pixelHeight = unit / cellRows;
    pattern.forEach((row, rowIndex) => {
      [...row].forEach((token, colIndex) => {
        const color = definition.colors?.[token] || "transparent";
        if (!color || color === "transparent") {
          return;
        }
        context.fillStyle = color;
        context.fillRect(
          x + colIndex * pixelWidth,
          y + rowIndex * pixelHeight,
          pixelWidth,
          pixelHeight,
        );
      });
    });
  }

  visualSpriteOffset(definition, unit) {
    const cols = Math.max(1, definition.pattern?.[0]?.length || 1);
    const rows = Math.max(1, definition.pattern?.length || 1);
    const cellCols = Math.max(1, Number(definition.pixelsPerCell?.width) || cols);
    const cellRows = Math.max(1, Number(definition.pixelsPerCell?.height) || rows);
    return {
      x: (Number(definition.offset?.x) || 0) * unit / cellCols,
      y: (Number(definition.offset?.y) || 0) * unit / cellRows,
    };
  }

  cachedImage(source) {
    const url = window.PuzzleAssets?.url(source) || source;
    const cache = this.constructor.imageCache || (this.constructor.imageCache = new Map());
    const existing = cache.get(url);
    if (existing) {
      return existing;
    }
    const image = new Image();
    image.addEventListener("load", () => {
      if (this.lastScene && this.root.isConnected) {
        this.render(this.lastScene);
      }
    }, { once: true });
    image.src = url;
    cache.set(url, image);
    return image;
  }

  canvasCellUnit(scene, frame) {
    let unit = 1;
    let hasImage = false;
    for (const cell of this.frameCells(scene, frame)) {
      for (const layer of cell.layers || []) {
        const definition = this.resolveVisualSprite(layer)?.definition;
        if (!definition) {
          continue;
        }
        if (definition.source) {
          hasImage = true;
          continue;
        }
        const cols = Math.max(1, definition.pattern?.[0]?.length || 1);
        const rows = Math.max(1, definition.pattern?.length || 1);
        const cellCols = Math.max(1, Number(definition.pixelsPerCell?.width) || cols);
        const cellRows = Math.max(1, Number(definition.pixelsPerCell?.height) || rows);
        unit = this.boundedLeastCommonMultiple(unit, cellCols, 128);
        unit = this.boundedLeastCommonMultiple(unit, cellRows, 128);
      }
    }
    return hasImage ? this.boundedLeastCommonMultiple(unit, 32, 128) : unit;
  }

  boundedLeastCommonMultiple(a, b, limit) {
    const left = Math.max(1, Math.trunc(Number(a) || 1));
    const right = Math.max(1, Math.trunc(Number(b) || 1));
    const value = (left * right) / this.greatestCommonDivisor(left, right);
    return value <= limit ? value : left;
  }

  greatestCommonDivisor(a, b) {
    let left = Math.abs(a);
    let right = Math.abs(b);
    while (right) {
      const next = left % right;
      left = right;
      right = next;
    }
    return left || 1;
  }

  canvasFloorColor() {
    const rootStyle = getComputedStyle(this.root);
    return (
      rootStyle.getPropertyValue("--cell-background").trim()
      || "transparent"
    );
  }

  boardLabel(scene, frame) {
    return `Board ${frame.width} by ${frame.height}`;
  }

  patternDataUrl(definition) {
    const pattern = definition.pattern || [];
    const width = Math.max(1, pattern[0]?.length || 1);
    const height = Math.max(1, pattern.length || 1);
    const rects = [];
    pattern.forEach((row, y) => {
      [...row].forEach((token, x) => {
        const color = definition.colors?.[token] || "transparent";
        if (!color || color === "transparent") {
          return;
        }
        rects.push(`<rect x="${x}" y="${y}" width="1" height="1" fill="${this.svgAttribute(color)}"/>`);
      });
    });
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" shape-rendering="crispEdges">${rects.join("")}</svg>`;
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  }

  svgAttribute(value) {
    return String(value)
      .replace(/&/g, "&amp;")
      .replace(/"/g, "&quot;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
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

  sortedLayers(layers) {
    return [...layers].sort((a, b) => a.layer - b.layer);
  }

  usesVisualSprites(scene, visuals) {
    return scene.cells.some((cell) =>
      cell.layers.some((layer) => Boolean(this.resolveVisualSprite(layer, visuals))),
    );
  }

  hasVisualConfig(visuals) {
    return Boolean(
      visuals.boardClass
        || visuals.themeClass
        || Object.keys(visuals.sprites || {}).length,
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

  resolveVisualSprite(layer, visuals = this.visuals()) {
    const sprites = visuals.sprites || {};
    for (const key of this.visualKeys(layer, visuals.aliases || {})) {
      if (sprites[key]) {
        return { key, definition: sprites[key] };
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
    addWithAlias(layer.sprite);

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

window.PuzzleRenderer = PuzzleRenderer;
