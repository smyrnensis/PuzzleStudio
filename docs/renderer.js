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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
        visual.style.zIndex = String(this.layerRenderOrder(layer) + 1);
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
=======
      && cell.y < viewport.y + viewport.height);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6

    for (const item of this.canvasDisplayList(scene, frame, unit, animations, progress, now)) {
      this.paintCanvasItem(context, item, unit, progress, now);
=======
    for (const batch of resolved?.batches || []) {
      this.paintResolvedBatch(context, frame, unit, batch);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
    }
    this.paintCanvasGrid(context, scene, frame, unit);
    context.restore();
  }

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  canvasDisplayList(scene, frame, unit, animations = [], progress = 1, now = performance.now()) {
    const items = [];
    const animationPaintOrder = this.animationPaintOrderByCell(scene, frame, animations, progress);
    let order = 0;
    for (const cell of this.frameCells(scene, frame)) {
      const x = (cell.x - frame.x) * unit;
      const y = (cell.y - frame.y) * unit;
      const sourceCellOrder = this.cellRenderOrder(cell);
      const cellOrder = animationPaintOrder.get(`${cell.x},${cell.y}`) ?? sourceCellOrder;
      const layers = this.sortedLayers(cell.layers);
      for (let index = 0; index < layers.length;) {
        const priority = this.layerRenderPriority(layers[index]);
        const composition = this.layerComposition(layers[index]);
        const priorityLayers = [];
        while (index < layers.length && this.layerRenderPriority(layers[index]) === priority) {
          if (this.layerComposition(layers[index]) !== composition) {
            throw new Error(`resolved visual composition conflicts at priority ${priority}`);
          }
          priorityLayers.push(layers[index]);
          index += 1;
        }
        if (composition === "average") {
          items.push({
            kind: "merge",
            layers: priorityLayers,
            cellOrder,
            sourceCellOrder,
            layerOrder: priority,
            order: order++,
            x,
            y,
            animations: priorityLayers.map((layer) => this.animationForLayer(animations, cell, layer)),
          });
          continue;
        }
        for (const layer of priorityLayers) {
          const animation = this.animationForLayer(animations, cell, layer);
          const item = {
            kind: "layer",
            layer,
            cellOrder,
            sourceCellOrder,
            layerOrder: this.layerRenderPriority(layer),
            order: order++,
            x,
            y,
            animation: animation && progress < 1 ? animation : null,
          };
          items.push(item);
        }
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
      }
      bitmapContext.putImageData(image, 0, 0);
      context.imageSmoothingEnabled = geometry.sampling === "smooth";
      context.drawImage(bitmap, x, y, drawWidth, drawHeight);
      return;
    }
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    for (const instance of this.activeTriggerAnimations) {
      const elapsedMs = now - instance.startedAtMs;
      if (elapsedMs < 0 || elapsedMs >= instance.durationMs) {
        continue;
      }
      const layerOrder = instance.renderPriority;
      const sourceCell = (scene.cells || []).find((cell) =>
        Number(cell.x) === instance.x && Number(cell.y) === instance.y);
      if (!sourceCell) {
        throw new Error("animation event references a cell outside the resolved scene");
      }
      const sourceCellOrder = this.cellRenderOrder(sourceCell);
      const x = (instance.x - frame.x) * unit;
      const y = (instance.y - frame.y) * unit;
      if (instance.composition === "average") {
        const merged = items.find((item) => item.kind === "merge"
          && item.layerOrder === layerOrder
          && item.x === x
          && item.y === y);
        if (merged) {
          merged.triggerInstances ||= [];
          merged.triggerInstances.push(instance);
        } else {
          items.push({
            kind: "merge",
            layers: [],
            triggerInstances: [instance],
            cellOrder: sourceCellOrder,
            sourceCellOrder,
            layerOrder,
            order: order++,
            x,
            y,
            animations: [],
          });
        }
        continue;
      }
      items.push({
        kind: "trigger",
        instance,
        cellOrder: sourceCellOrder,
        sourceCellOrder,
        layerOrder,
        order: order++,
        x,
        y,
      });
    }
    const compare = (a, b) => a.cellOrder - b.cellOrder
      || a.layerOrder - b.layerOrder
      || a.sourceCellOrder - b.sourceCellOrder
      || a.order - b.order;
    return items.sort(compare);
=======
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
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
  }

  animationPaintOrderByCell(scene, frame, animations, progress) {
    if (progress >= 1) {
      return new Map();
    }
    const cells = this.frameCells(scene, frame);
    const cellsByKey = new Map(cells.map((cell) => [`${cell.x},${cell.y}`, cell]));
    const groups = [];
    for (const animation of animations) {
      if (animation?.kind !== "move") {
        continue;
      }
      const fromX = Number(animation.from?.x);
      const fromY = Number(animation.from?.y);
      const toX = Number(animation.to?.x);
      const toY = Number(animation.to?.y);
      if (![fromX, fromY, toX, toY].every(Number.isFinite)
        || (fromX === toX && fromY === toY)) {
        continue;
      }
      const minX = Math.min(fromX, toX);
      const maxX = Math.max(fromX, toX);
      const minY = Math.min(fromY, toY);
      const maxY = Math.max(fromY, toY);
      const group = new Set(cells
        .filter((cell) => cell.x >= minX && cell.x <= maxX && cell.y >= minY && cell.y <= maxY)
        .map((cell) => `${cell.x},${cell.y}`));
      if (group.size === 0) {
        continue;
      }
      for (let index = groups.length - 1; index >= 0; index -= 1) {
        if (![...group].some((key) => groups[index].has(key))) {
          continue;
        }
        for (const key of groups[index]) {
          group.add(key);
        }
        groups.splice(index, 1);
      }
      groups.push(group);
    }

    const paintOrder = new Map();
    for (const group of groups) {
      const groupOrder = Math.max(...[...group].map((key) =>
        this.cellRenderOrder(cellsByKey.get(key))));
      for (const key of group) {
        paintOrder.set(key, groupOrder);
      }
    }
    return paintOrder;
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

<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
  prepareAnimations(events, frame) {
    return (events || [])
      .map((event) => ({ ...event, objectId: Number(event.objectId) }))
      .filter((event) => {
        if (this.isTriggerAnimationEvent(event)) {
          return false;
        }
        if (!Number.isFinite(event.objectId) || event.objectId <= 0) {
          return false;
        }
        const target = event.kind === "move" ? event.to : event.position;
        const x = Number(target?.x);
        const y = Number(target?.y);
        return x >= frame.x && y >= frame.y && x < frame.x + frame.width && y < frame.y + frame.height;
      });
  }

  animationDurationMs(scene) {
    const tween = scene?.settings?.animation?.tween || scene?.animation?.tween || null;
    return Math.max(1, Number(tween?.intervalMs || scene?.animationDurationMs || 250));
  }

  animationProgressForFrame(elapsedMs, durationMs, frameIndex) {
    const timeProgress = Math.min(1, Math.max(0, elapsedMs / durationMs));
    const finalFrameIndex = this.minimumAnimationFrameCount() - 1;
    if (frameIndex < finalFrameIndex && timeProgress >= 1) {
      return frameIndex / finalFrameIndex;
    }
    return timeProgress;
  }

  minimumAnimationFrameCount() {
    return 3;
  }

  animationForLayer(animations, cell, layer) {
    const objectId = Number(layer.objectId);
    return animations.find((event) => {
      if (event.objectId !== objectId) {
        return false;
      }
      if (event.kind === "move") {
        return Number(event.to?.x) === cell.x && Number(event.to?.y) === cell.y;
      }
      return Number(event.position?.x) === cell.x && Number(event.position?.y) === cell.y;
    }) || null;
  }

  paintCanvasItem(context, item, unit, progress = 1, now = performance.now()) {
    if (item.kind === "merge") {
      this.paintCanvasMergedLayers(context, item, unit, progress, now);
      return;
    }
    if (item.kind === "trigger") {
      this.paintTriggerAnimationInstance(context, item.instance, item.x, item.y, unit, now);
      return;
    }
    this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress, now);
  }

  paintCanvasMergedLayers(context, item, unit, progress, now) {
    const samples = item.layers.map((layer, index) => {
      const canvas = document.createElement("canvas");
      canvas.width = context.canvas.width;
      canvas.height = context.canvas.height;
      const sample = canvas.getContext("2d");
      sample.setTransform(context.getTransform());
      sample.imageSmoothingEnabled = context.imageSmoothingEnabled;
      this.paintCanvasLayer(sample, layer, item.x, item.y, unit, item.animations[index], progress, now);
      return sample.getImageData(0, 0, canvas.width, canvas.height);
    });
    for (const instance of item.triggerInstances || []) {
      const canvas = document.createElement("canvas");
      canvas.width = context.canvas.width;
      canvas.height = context.canvas.height;
      const sample = canvas.getContext("2d");
      sample.setTransform(context.getTransform());
      sample.imageSmoothingEnabled = context.imageSmoothingEnabled;
      this.paintTriggerAnimationInstance(sample, instance, item.x, item.y, unit, now);
      samples.push(sample.getImageData(0, 0, canvas.width, canvas.height));
    }
    const merged = context.createImageData(context.canvas.width, context.canvas.height);
    for (let offset = 0; offset < merged.data.length; offset += 4) {
      let count = 0;
      let red = 0;
      let green = 0;
      let blue = 0;
      let alpha = 0;
      for (const sample of samples) {
        if (sample.data[offset + 3] === 0) {
          continue;
        }
        count += 1;
        red += sample.data[offset];
        green += sample.data[offset + 1];
        blue += sample.data[offset + 2];
        alpha += sample.data[offset + 3];
      }
      if (count > 0) {
        merged.data[offset] = Math.round(red / count);
        merged.data[offset + 1] = Math.round(green / count);
        merged.data[offset + 2] = Math.round(blue / count);
        merged.data[offset + 3] = Math.round(alpha / count);
      }
    }
    const output = document.createElement("canvas");
    output.width = context.canvas.width;
    output.height = context.canvas.height;
    output.getContext("2d").putImageData(merged, 0, 0);
    context.save();
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.drawImage(output, 0, 0);
    context.restore();
  }

  paintCanvasLayer(context, layer, x, y, unit, animation = null, progress = 1, now = performance.now()) {
    const visual = this.resolveVisual(layer);
    const definition = visual?.definition;
    if (!definition) {
      return;
    }
    const frame = this.resolveVisualFrame(definition, this.loopAnimationTimeMs(now, definition));
    const transform = this.animationTransform(animation, progress, unit);
    const usesVisualTransforms = this.hasVisualTransforms(frame) || Boolean(animation?.visualTween);
    const usesTransformStack = usesVisualTransforms
      || (transform && this.requiresCanvasTransformStack(transform));
    if (transform && !usesTransformStack) {
      x += transform.x;
      y += transform.y;
    }
    if (usesTransformStack) {
      context.save();
      context.globalAlpha *= transform?.alpha ?? 1;
      context.translate(x + unit / 2 + (transform?.x || 0), y + unit / 2 + (transform?.y || 0));
      context.rotate(transform?.angle || 0);
      context.scale(transform?.scale ?? 1, transform?.scale ?? 1);
      this.applyCanvasVisualTransforms(context, frame, unit, animation, progress, now);
      x = -unit / 2;
      y = -unit / 2;
    }

    if (frame.source) {
      const image = this.cachedImage(frame.source);
      if (image?.complete && image.naturalWidth > 0) {
        const fit = this.visualCanvasFit(frame, unit, {
          cols: image.naturalWidth,
          rows: image.naturalHeight,
        });
        context.save();
        context.imageSmoothingEnabled = this.visualSampling(frame) === "smooth";
        context.drawImage(
          image,
          x + fit.x,
          y + fit.y,
          fit.width,
          fit.height,
        );
        context.restore();
      }
      if (usesTransformStack) {
        context.restore();
      }
      return;
    }

    const solidColor = this.solidPatternColor(frame);
    if (solidColor && this.canPaintAsFullCellSolid(frame)) {
      context.fillStyle = solidColor;
      this.fillCanvasRect(context, x, y, unit, unit);
      if (usesTransformStack) {
        context.restore();
      }
      return;
    }

    const fit = this.visualCanvasFit(frame, unit);
    context.save();
    context.imageSmoothingEnabled = this.visualSampling(frame) === "smooth";
    this.paintLogicalPatternToCanvas(context, frame, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight);
    context.restore();
    if (usesTransformStack) {
      context.restore();
    }
  }

  requiresCanvasTransformStack(transform) {
    return transform.alpha !== 1
      || transform.scale !== 1
      || transform.angle !== 0;
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
        transform.x += (Number(animation.from?.x) - Number(animation.to?.x)) * unit * (1 - eased);
        transform.y += (Number(animation.from?.y) - Number(animation.to?.y)) * unit * (1 - eased);
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
    return transform;
  }

  recordTriggerAnimations(events, frame) {
    const now = performance.now();
    for (const event of events || []) {
      if (!this.isTriggerAnimationEvent(event)) {
        continue;
      }
      const position = this.triggerAnimationPosition(event);
      const x = Number(position?.x);
      const y = Number(position?.y);
      if (!(x >= frame.x && y >= frame.y && x < frame.x + frame.width && y < frame.y + frame.height)) {
        continue;
      }
      const name = String(event.name || "");
      if (!name) {
        throw new Error("animation event is missing its visual name");
      }
      const definition = this.resolveAnimationDefinition(name);
      const renderPriority = Number(event.resolvedVisual?.renderPriority);
      const composition = String(event.resolvedVisual?.composition || "");
      if (!Number.isSafeInteger(renderPriority) || renderPriority < 0) {
        throw new Error(`animation event is missing resolved render priority: !${name}`);
      }
      if (composition !== "ordered" && composition !== "average") {
        throw new Error(`animation event has invalid resolved composition: !${name}`);
      }
      const durationMs = this.animationDefinitionDurationMs(definition, event.durationMs ?? event.duration);
      const id = this.triggerAnimationEventId(event, name, x, y);
      if (this.activeTriggerAnimations.some((instance) => instance.id === id)) {
        continue;
      }
      this.activeTriggerAnimations.push({
        id,
        name,
        definition,
        x,
        y,
        renderPriority,
        composition,
        startedAtMs: now,
        durationMs,
      });
    }
  }

  isTriggerAnimationEvent(event) {
    return event?.kind === "animation";
  }

  triggerAnimationPosition(event) {
    return event.position || null;
  }

  triggerAnimationEventId(event, name, x, y) {
    if (event.id != null) {
      return String(event.id);
    }
    const position = this.triggerAnimationPosition(event) || {};
    return [
      name,
      Number(position.x),
      Number(position.y),
      Number(position.z || 0),
      event.durationMs ?? "",
      event.sequence ?? event.seq ?? event.ruleId ?? "",
    ].join(":");
  }

  pruneTriggerAnimations(now) {
    this.activeTriggerAnimations = this.activeTriggerAnimations.filter((instance) =>
      now - instance.startedAtMs < instance.durationMs
    );
  }

  paintTriggerAnimationInstance(context, instance, x, y, unit, now) {
    const elapsedMs = now - instance.startedAtMs;
    if (elapsedMs < 0 || elapsedMs >= instance.durationMs) {
      return;
    }
    const visualFrame = this.resolveVisualFrame(instance.definition, elapsedMs, {
      playback: "once",
      durationMs: instance.durationMs,
    });
    this.paintResolvedCanvasFrame(context, visualFrame, x, y, unit);
  }

  paintResolvedCanvasFrame(context, definition, x, y, unit) {
    const usesTransformStack = this.hasVisualTransforms(definition);
    if (usesTransformStack) {
      context.save();
      context.translate(x + unit / 2, y + unit / 2);
      this.applyCanvasVisualTransforms(context, definition, unit);
      x = -unit / 2;
      y = -unit / 2;
    }
    if (definition.source) {
      const image = this.cachedImage(definition.source);
      if (image?.complete && image.naturalWidth > 0) {
        const fit = this.visualCanvasFit(definition, unit, {
          cols: image.naturalWidth,
          rows: image.naturalHeight,
        });
        context.save();
        context.imageSmoothingEnabled = this.visualSampling(definition) === "smooth";
        context.drawImage(image, x + fit.x, y + fit.y, fit.width, fit.height);
        context.restore();
      }
      if (usesTransformStack) {
        context.restore();
      }
      return;
    }

    const solidColor = this.solidPatternColor(definition);
    if (solidColor && this.canPaintAsFullCellSolid(definition)) {
      context.fillStyle = solidColor;
      this.fillCanvasRect(context, x, y, unit, unit);
      if (usesTransformStack) {
        context.restore();
      }
      return;
    }

    const fit = this.visualCanvasFit(definition, unit);
    context.save();
    context.imageSmoothingEnabled = this.visualSampling(definition) === "smooth";
    this.paintLogicalPatternToCanvas(context, definition, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight);
    context.restore();
    if (usesTransformStack) {
      context.restore();
    }
  }

  resolveAnimationDefinition(name) {
    const visuals = this.visuals();
    const definition = visuals.entries?.[name];
    if (!definition) {
      throw new Error(`compiled animation visual is missing: !${name}`);
    }
    return definition;
  }

  sceneUsesTimeVaryingVisuals(scene, frame) {
    for (const cell of this.frameCells(scene, frame)) {
      for (const layer of cell.layers || []) {
        const definition = this.resolveVisual(layer)?.definition;
        if (definition && this.visualFrameCount(definition) > 1) {
          return true;
        }
      }
    }
    return false;
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

  hasVisualTransforms(definition) {
    return Array.isArray(definition?.transforms) && definition.transforms.length > 0;
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

  applyCanvasVisualTransforms(context, definition, unit, animation = null, progress = 1, now = performance.now()) {
    const state = this.tweenedVisualState(definition, animation, progress);
    const transforms = state.transforms;
    if (state.opacity !== undefined) {
      context.globalAlpha *= Math.min(1, Math.max(0, Number(state.opacity)));
    }
    for (const transform of [...transforms].reverse()) {
      if (transform?.kind === "rotate") {
        context.rotate(-Number(transform.degrees || 0) * Math.PI / 180);
      } else if (transform?.kind === "translate") {
        context.translate((Number(transform.x) || 0) * unit, (Number(transform.y) || 0) * unit);
      } else if (transform?.kind === "scale") {
        context.scale(Number(transform.x), Number(transform.y));
      } else if (transform?.kind === "flip") {
        if (transform.enabled) {
          context.scale(-1, -1);
        }
      } else {
        throw new Error(`Unknown visual transform kind: ${String(transform?.kind)}`);
      }
    }
  }

  tweenedVisualState(definition, animation, progress) {
    const target = Array.isArray(definition?.transforms) ? definition.transforms : [];
    if (!animation?.visualTween || progress >= 1) {
      return { transforms: target, opacity: undefined };
    }
    if (!window.PuzzleVisualTweenCore) {
      throw new Error("Visual tween core is unavailable.");
    }
    const state = window.PuzzleVisualTweenCore.interpolate(animation.visualTween, progress);
    return {
      opacity: state.opacity,
      transforms: state.transforms.map((transform) => {
        if (transform.kind === "rotate") {
          const axis = transform.axis || [];
          if (Math.abs(Number(axis[0])) > 0.000000001
              || Math.abs(Number(axis[1])) > 0.000000001
              || Math.abs(Math.abs(Number(axis[2])) - 1) > 0.000000001) {
            throw new Error("2D visual tween rotation axis must be +Z or -Z.");
          }
          return { ...transform, degrees: Number(transform.degrees) * Number(axis[2]) };
        }
        if (transform.kind === "translate" || transform.kind === "scale") {
          const value = transform.value || [];
          if (Math.abs(Number(value[2])) > 0.000000001) {
            throw new Error(`2D visual tween ${transform.kind} must stay in the XY plane.`);
          }
          return { ...transform, x: Number(value[0]), y: Number(value[1]) };
        }
        if (transform.kind === "flip") {
          return transform;
        }
        throw new Error(`Unknown 2D visual tween transform: ${String(transform.kind)}`);
      }),
    };
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

  paintLogicalPatternToCanvas(context, definition, x, y, pixelWidth, pixelHeight = pixelWidth) {
    const pattern = definition.pattern || [];
    pattern.forEach((row, rowIndex) => {
      [...row].forEach((token, colIndex) => {
        const color = definition.colors?.[token] || "transparent";
        if (!color || color === "transparent") {
          return;
        }
        context.fillStyle = color;
        const left = x + colIndex * pixelWidth;
        const right = x + (colIndex + 1) * pixelWidth;
        const top = y + rowIndex * pixelHeight;
        const bottom = y + (rowIndex + 1) * pixelHeight;
        this.fillCanvasRect(context, left, top, right - left, bottom - top);
      });
    });
  }

=======
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
    const rootStyle = getComputedStyle(this.root);
    return (
      rootStyle.getPropertyValue("--cell-background").trim()
      || "transparent"
    );
  }

  boardLabel(scene, frame) {
    return `Board ${frame.width} by ${frame.height}`;
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

  visualCellGrid(definition) {
    const pattern = this.visualPatternSize(definition);
    return {
      cols: Math.max(1, Number(definition.pixelsPerCell?.width) || pattern.cols),
      rows: Math.max(1, Number(definition.pixelsPerCell?.height) || pattern.rows),
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
      this.layerRenderPriority(a) - this.layerRenderPriority(b)
      || Number(a.objectId) - Number(b.objectId)
    );
  }

  layersUseMerge(layers) {
    return layers.some((layer) => this.layerComposition(layer) === "average");
  }

  layerRenderPriority(layer) {
    const priority = Number(layer?.renderPriority);
    if (!Number.isSafeInteger(priority) || priority < 0) {
      throw new Error("resolved visual layer is missing renderPriority");
    }
    return priority;
  }

  layerRenderOrder(layer) {
    const order = Number(layer?.renderOrder);
    if (!Number.isSafeInteger(order) || order < 0) {
      throw new Error("resolved visual layer is missing renderOrder");
    }
    return order;
  }

  layerComposition(layer) {
    const composition = String(layer?.composition || "");
    if (composition !== "ordered" && composition !== "average") {
      throw new Error("resolved visual layer has invalid composition");
    }
    return composition;
  }

  cellRenderOrder(cell) {
    const order = Number(cell?.renderOrder);
    if (!Number.isSafeInteger(order) || order < 0) {
      throw new Error("resolved visual cell is missing renderOrder");
    }
    return order;
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
    const visuals = this.lastScene?.visuals;
    if (!visuals || typeof visuals !== "object" || !visuals.entries || !visuals.aliases) {
      throw new Error("runtime scene is missing its typed 2D visual catalog");
    }
    return visuals;
=======
    return getComputedStyle(this.root).getPropertyValue("--cell-background").trim()
      || "transparent";
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
