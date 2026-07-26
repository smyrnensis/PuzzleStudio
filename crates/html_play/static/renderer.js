class PuzzleRenderer {
  constructor(root, options = {}) {
    this.root = root;
    this.options = options;
  }

  render(scene) {
    if (!scene?.renderScene || typeof scene.renderScene !== "object") {
      throw new Error("2D renderer requires the typed renderScene contract.");
    }
    if (this.options.renderMode === "dom") {
      throw new Error("Typed render scenes require the Canvas renderer.");
    }
    if (typeof this.options.prepareRenderScene !== "function"
        || typeof this.options.resolveRenderMoment !== "function") {
      throw new Error("2D renderer requires the Rust render-scene bridge.");
    }
    window.__PuzzleCurrentScene = scene;
    this.lastScene = scene;
    const viewport = this.resolveViewport(scene);
    this.root.style.setProperty("--cols", viewport.width);
    this.root.style.setProperty("--rows", viewport.height);
    this.root.classList.add("is-canvas-renderer");
    this.root.dataset.viewportX = String(viewport.x);
    this.root.dataset.viewportY = String(viewport.y);
    this.root.dataset.viewportWidth = String(viewport.width);
    this.root.dataset.viewportHeight = String(viewport.height);
    const grid = this.gridSettings(scene);
    this.root.classList.toggle("has-occupied-cell-grid", grid.occupiedCells);
    this.root.classList.toggle("has-all-cell-grid", grid.allCells);
    this.root.replaceChildren();
    this.renderCanvas(scene, viewport);
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
    if (mode === "paged" && previous
        && previous.width === width && previous.height === height
        && this.viewportContains(previous, focus)) {
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
      if (focusObjects.size > 0
          && cell.layers?.some((layer) => focusObjects.has(Number(layer.objectId)))) {
        return cell;
      }
      if (cell.layers?.some((layer) => layer.object === objectName)) {
        return cell;
      }
    }
    return null;
  }

  viewportContains(viewport, cell) {
    return Boolean(cell
      && cell.x >= viewport.x && cell.y >= viewport.y
      && cell.x < viewport.x + viewport.width
      && cell.y < viewport.y + viewport.height);
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
      cell.x >= viewport.x && cell.y >= viewport.y
      && cell.x < viewport.x + viewport.width
      && cell.y < viewport.y + viewport.height);
  }

  renderCanvas(scene, frame) {
    const canvas = document.createElement("canvas");
    canvas.className = "board-canvas";
    const presentationUnit = this.canvasPresentationCellUnit();
    canvas.width = Math.max(1, Math.ceil(frame.width * presentationUnit));
    canvas.height = Math.max(1, Math.ceil(frame.height * presentationUnit));
    canvas.setAttribute("aria-label", `Board ${frame.width} by ${frame.height}`);
    const animations = scene.animationEvents || [];
    const renderScenePromise = Promise.resolve(this.options.prepareRenderScene(scene.renderScene));
    let startedAt = null;
    let queuedFrame = false;
    let drawing = false;
    const draw = async () => {
      queuedFrame = false;
      if (drawing) {
        queueDraw();
        return;
      }
      drawing = true;
      if (!canvas.isConnected) {
        resizeObserver?.disconnect();
        drawing = false;
        return;
      }
      if (!this.root.isConnected) {
        drawing = false;
        queueDraw();
        return;
      }
      const metrics = this.canvasMetrics(canvas, frame);
      if (!metrics) {
        drawing = false;
        queueDraw();
        return;
      }
      if (canvas.width !== metrics.pixelWidth || canvas.height !== metrics.pixelHeight) {
        canvas.width = metrics.pixelWidth;
        canvas.height = metrics.pixelHeight;
      }
      const context = canvas.getContext("2d");
      context.setTransform(metrics.scaleX, 0, 0, metrics.scaleY, 0, 0);
      context.__puzzleCanvasScaleX = metrics.scaleX;
      context.__puzzleCanvasScaleY = metrics.scaleY;
      context.imageSmoothingEnabled = false;
      const now = performance.now();
      startedAt ??= now;
      const animationElapsedMs = Math.max(0, now - startedAt);
      try {
        const renderScene = await renderScenePromise;
        const resolved = await this.options.resolveRenderMoment(renderScene, {
          clipElapsedMs: Math.max(0, Math.floor(now - this.renderClockEpochMs())),
          animationElapsedMs: Math.floor(animationElapsedMs),
          animations,
        });
        context.clearRect(0, 0, metrics.cssWidth, metrics.cssHeight);
        this.paintResolvedRenderFrame(context, scene, frame, metrics.unit, resolved);
        drawing = false;
        if (resolved.continueAnimation && this.root.isConnected) {
          queueDraw();
        }
      } catch (error) {
        resizeObserver?.disconnect();
        drawing = false;
        this.options.onError?.(error);
      }
    };
    const queueDraw = () => {
      if (!queuedFrame) {
        queuedFrame = true;
        requestAnimationFrame(draw);
      }
    };
    const resizeObserver = typeof ResizeObserver === "function"
      ? new ResizeObserver(queueDraw)
      : null;
    this.root.append(canvas);
    resizeObserver?.observe(canvas);
    queueDraw();
  }

  renderClockEpochMs() {
    return this.constructor.renderClockEpochMs
      ?? (this.constructor.renderClockEpochMs = performance.now());
  }

  paintResolvedRenderFrame(context, scene, frame, unit, resolved) {
    context.save();
    const floorColor = this.canvasFloorColor();
    if (floorColor && floorColor !== "transparent") {
      context.fillStyle = floorColor;
      this.fillCanvasRect(context, 0, 0, frame.width * unit, frame.height * unit);
    }
    context.beginPath();
    context.rect(0, 0, frame.width * unit, frame.height * unit);
    context.clip();
    for (const batch of resolved?.batches || []) {
      this.paintResolvedBatch(context, frame, unit, batch);
    }
    this.paintCanvasGrid(context, scene, frame, unit);
    context.restore();
  }

  paintResolvedBatch(context, frame, unit, batch) {
    const content = batch?.content;
    if (content?.kind !== "pixels") {
      throw new Error(`2D renderer received unsupported resolved primitive: ${String(content?.kind)}`);
    }
    const transform = batch.transform;
    if (!Array.isArray(transform) || transform.length !== 4) {
      throw new Error("Resolved render batch is missing its affine transform.");
    }
    const cellX = (Number(batch.cell?.[0]) - frame.x) * unit;
    const cellY = (Number(batch.cell?.[1]) - frame.y) * unit;
    context.save();
    context.globalAlpha *= Math.min(1, Math.max(0, Number(batch.opacity)));
    context.translate(cellX + unit / 2, cellY + unit / 2);
    context.transform(
      Number(transform[0][0]), -Number(transform[1][0]),
      -Number(transform[0][1]), Number(transform[1][1]),
      Number(transform[0][3]) * unit, Number(transform[1][3]) * unit,
    );
    const geometry = batch.pixelGeometry;
    if (!geometry) {
      throw new Error("Resolved pixel batch is missing its geometry.");
    }
    this.paintResolvedPixels(context, content, geometry, unit);
    context.restore();
  }

  paintResolvedPixels(context, content, geometry, unit) {
    const width = Math.max(1, Number(content.width));
    const height = Math.max(1, Number(content.height));
    const x = (-0.5 + Number(geometry.x)) * unit;
    const y = (-0.5 + Number(geometry.y)) * unit;
    const drawWidth = Number(geometry.width) * unit;
    const drawHeight = Number(geometry.height) * unit;
    if (geometry.raster) {
      const bitmap = document.createElement("canvas");
      bitmap.width = width;
      bitmap.height = height;
      const bitmapContext = bitmap.getContext("2d");
      const image = bitmapContext.createImageData(width, height);
      for (const pixel of content.pixels || []) {
        const px = Number(pixel.position?.[0]);
        const py = Number(pixel.position?.[1]);
        if (px < 0 || py < 0 || px >= width || py >= height) {
          continue;
        }
        const offset = (py * width + px) * 4;
        image.data[offset] = this.linearSrgbByte(pixel.color?.red);
        image.data[offset + 1] = this.linearSrgbByte(pixel.color?.green);
        image.data[offset + 2] = this.linearSrgbByte(pixel.color?.blue);
        image.data[offset + 3] = Math.round(Math.min(1, Math.max(0, Number(pixel.color?.alpha))) * 255);
      }
      bitmapContext.putImageData(image, 0, 0);
      context.imageSmoothingEnabled = geometry.sampling === "smooth";
      context.drawImage(bitmap, x, y, drawWidth, drawHeight);
      return;
    }
    const pixelWidth = drawWidth / width;
    const pixelHeight = drawHeight / height;
    for (const pixel of content.pixels || []) {
      context.fillStyle = this.linearRgbaCss(pixel.color);
      this.fillCanvasRect(
        context,
        x + Number(pixel.position?.[0]) * pixelWidth,
        y + Number(pixel.position?.[1]) * pixelHeight,
        pixelWidth,
        pixelHeight,
      );
    }
  }

  linearRgbaCss(color) {
    const alpha = Math.min(1, Math.max(0, Number(color?.alpha)));
    return `rgba(${this.linearSrgbByte(color?.red)}, ${this.linearSrgbByte(color?.green)}, ${this.linearSrgbByte(color?.blue)}, ${alpha})`;
  }

  linearSrgbByte(value) {
    const linear = Math.min(1, Math.max(0, Number(value)));
    const srgb = linear <= 0.0031308
      ? 12.92 * linear
      : 1.055 * (linear ** (1 / 2.4)) - 0.055;
    return Math.round(srgb * 255);
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
    for (const cell of this.viewportCells(scene, frame)) {
      if (!grid.allCells && !cell.layers?.length) {
        continue;
      }
      const x = (cell.x - frame.x) * unit;
      const y = (cell.y - frame.y) * unit;
      context.strokeRect(x, y, Math.max(1, unit - 1), Math.max(1, unit - 1));
    }
    context.restore();
  }

  fillCanvasRect(context, x, y, width, height) {
    const left = this.canvasPixelEdge(context, x, "x");
    const right = this.canvasPixelEdge(context, x + width, "x");
    const top = this.canvasPixelEdge(context, y, "y");
    const bottom = this.canvasPixelEdge(context, y + height, "y");
    context.fillRect(
      left,
      top,
      Math.max(1 / this.canvasAxisScale(context, "x"), right - left),
      Math.max(1 / this.canvasAxisScale(context, "y"), bottom - top),
    );
  }

  canvasPixelEdge(context, value, axis) {
    return Math.round(value * this.canvasAxisScale(context, axis))
      / this.canvasAxisScale(context, axis);
  }

  canvasAxisScale(context, axis) {
    const value = axis === "y" ? context.__puzzleCanvasScaleY : context.__puzzleCanvasScaleX;
    return Math.max(1, Number(value) || 1);
  }

  canvasMetrics(canvas, frame) {
    const rect = canvas.getBoundingClientRect();
    const presentationUnit = this.canvasPresentationCellUnit();
    const cssWidth = rect.width > 0 ? rect.width : frame.width * presentationUnit;
    const cssHeight = rect.height > 0 ? rect.height : frame.height * presentationUnit;
    if (cssWidth <= 0 || cssHeight <= 0) {
      return null;
    }
    const unit = Math.max(0.0001, Math.min(cssWidth / frame.width, cssHeight / frame.height));
    const targetScale = Math.max(1, Number(window.devicePixelRatio) || 1);
    const pixelWidth = Math.max(1, Math.round(cssWidth * targetScale));
    const pixelHeight = Math.max(1, Math.round(cssHeight * targetScale));
    return {
      cssWidth,
      cssHeight,
      unit,
      scaleX: pixelWidth / cssWidth,
      scaleY: pixelHeight / cssHeight,
      pixelWidth,
      pixelHeight,
    };
  }

  canvasPresentationCellUnit() {
    const parsed = Number.parseFloat(getComputedStyle(this.root).getPropertyValue("--cell-size"));
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 56;
  }

  canvasFloorColor() {
    return getComputedStyle(this.root).getPropertyValue("--cell-background").trim()
      || "transparent";
  }

  gridSettings(scene) {
    const raw = scene.settings?.grid || scene.render?.grid || scene.screen?.grid;
    if (!raw || raw === false || raw === true) {
      return { occupiedCells: false, allCells: false };
    }
    return {
      occupiedCells: Boolean(raw.occupied_cells ?? raw.occupiedCells),
      allCells: Boolean(raw.all_cells ?? raw.allCells),
      color: raw.color,
    };
  }
}

window.PuzzleRenderer = PuzzleRenderer;
