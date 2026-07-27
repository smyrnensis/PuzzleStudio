class PuzzleRenderer {
  static rasterImages = new Map();

  constructor(root, options = {}) {
    this.root = root;
    this.options = options;
  }

  render(scene) {
    const view = this.resolvedView(scene?.view);
    if (!scene?.renderScene || typeof scene.renderScene !== "object") {
      throw new Error("2D renderer requires the typed renderScene contract.");
    }
    if (this.options.renderMode === "dom") {
      throw new Error("Typed render scenes require the Canvas renderer.");
    }
    if (typeof this.options.resolveRenderMoment !== "function") {
      throw new Error("2D renderer requires the Rust render-moment resolver.");
    }

    window.__PuzzleCurrentScene = scene;
    this.root.classList.add("is-canvas-renderer");
    this.root.style.setProperty("--cols", view.width);
    this.root.style.setProperty("--rows", view.height);
    this.root.dataset.viewportX = String(view.x);
    this.root.dataset.viewportY = String(view.y);
    this.root.dataset.viewportWidth = String(view.width);
    this.root.dataset.viewportHeight = String(view.height);
    this.root.replaceChildren();
    this.renderCanvas(scene, view);
  }

  resolvedView(value) {
    const origin = value?.origin;
    const size = value?.size;
    if (
      !Array.isArray(origin)
      || origin.length !== 2
      || !origin.every(Number.isSafeInteger)
      || !Array.isArray(size)
      || size.length !== 2
      || !size.every((entry) => Number.isSafeInteger(entry) && entry > 0)
    ) {
      throw new Error("2D renderer requires a valid typed view.");
    }
    return {
      x: origin[0],
      y: origin[1],
      width: size[0],
      height: size[1],
    };
  }

  renderCanvas(scene, view) {
    const canvas = document.createElement("canvas");
    canvas.className = "board-canvas";
    const presentationUnit = this.canvasPresentationCellUnit();
    canvas.width = Math.max(1, Math.ceil(view.width * presentationUnit));
    canvas.height = Math.max(1, Math.ceil(view.height * presentationUnit));
    canvas.setAttribute("aria-label", `Board ${view.width} by ${view.height}`);

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
      const metrics = this.canvasMetrics(canvas, view);
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
      if (!context) {
        drawing = false;
        this.options.onError?.(new Error("2D canvas context is unavailable."));
        return;
      }
      context.setTransform(metrics.scaleX, 0, 0, metrics.scaleY, 0, 0);
      context.__puzzleCanvasScaleX = metrics.scaleX;
      context.__puzzleCanvasScaleY = metrics.scaleY;
      const now = performance.now();
      startedAt ??= now;
      try {
        const resolved = await this.options.resolveRenderMoment(scene.renderScene, {
          clipElapsedMs: Math.max(0, Math.floor(now - this.renderClockEpochMs())),
          animationElapsedMs: Math.max(0, Math.floor(now - startedAt)),
          animations: Array.isArray(scene.animationEvents) ? scene.animationEvents : [],
        });
        context.clearRect(0, 0, metrics.cssWidth, metrics.cssHeight);
        await this.paintResolvedRenderFrame(context, view, metrics.unit, resolved);
        drawing = false;
        if (resolved?.continueAnimation === true && this.root.isConnected) {
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

  async paintResolvedRenderFrame(context, view, unit, frame) {
    if (!frame || !Array.isArray(frame.batches) || !Array.isArray(frame.decorations)) {
      throw new Error("Rust render-moment resolver returned an invalid frame.");
    }
    context.save();
    const floorColor = this.canvasFloorColor();
    if (floorColor && floorColor !== "transparent") {
      context.fillStyle = floorColor;
      this.fillCanvasRect(context, 0, 0, view.width * unit, view.height * unit);
    }
    context.beginPath();
    context.rect(0, 0, view.width * unit, view.height * unit);
    context.clip();
    for (const [index, batch] of frame.batches.entries()) {
      await this.paintResolvedBatch(context, view, unit, batch, index);
    }
    this.paintResolvedDecorations(context, view, unit, frame.decorations);
    context.restore();
  }

  async paintResolvedBatch(context, view, unit, batch, index) {
    const transform = batch?.transform;
    if (
      !Array.isArray(transform)
      || transform.length !== 4
      || transform.some((row) =>
        !Array.isArray(row) || row.length !== 4 || row.some((value) => !Number.isFinite(value)))
    ) {
      throw new Error(`Resolved render batch ${index} has an invalid affine transform.`);
    }
    if (
      !Array.isArray(batch.cell)
      || batch.cell.length !== 3
      || !batch.cell.every(Number.isSafeInteger)
    ) {
      throw new Error(`Resolved render batch ${index} has an invalid cell.`);
    }
    const opacity = Number(batch.opacity);
    if (!Number.isFinite(opacity) || opacity < 0 || opacity > 1) {
      throw new Error(`Resolved render batch ${index} has invalid opacity.`);
    }

    context.save();
    context.globalAlpha *= opacity;
    context.translate(
      (batch.cell[0] - view.x + 0.5) * unit,
      (batch.cell[1] - view.y + 0.5) * unit,
    );
    context.transform(
      Number(transform[0][0]),
      -Number(transform[1][0]),
      -Number(transform[0][1]),
      Number(transform[1][1]),
      Number(transform[0][3]) * unit,
      Number(transform[1][3]) * unit,
    );
    const content = batch.content;
    if (content?.kind === "pixels") {
      this.paintResolvedPixels(context, content, batch.pixelGeometry, unit, index);
    } else if (content?.kind === "raster_image") {
      await this.paintResolvedRasterImage(context, content, batch.pixelGeometry, unit, index);
    } else if (content?.kind === "voxels") {
      throw new Error(`2D renderer received voxel batch ${index}.`);
    } else {
      throw new Error(`2D renderer received unsupported resolved primitive: ${String(content?.kind)}`);
    }
    context.restore();
  }

  paintResolvedPixels(context, content, geometry, unit, batchIndex) {
    const width = Number(content.width);
    const height = Number(content.height);
    if (!Number.isSafeInteger(width) || width <= 0 || !Number.isSafeInteger(height) || height <= 0) {
      throw new Error(`Resolved pixel batch ${batchIndex} has invalid dimensions.`);
    }
    this.validateRect(geometry, `Resolved pixel batch ${batchIndex} geometry`);
    if (geometry.clip != null) {
      this.validateRect(geometry.clip, `Resolved pixel batch ${batchIndex} clip`);
      context.save();
      context.beginPath();
      context.rect(
        (geometry.clip.x - 0.5) * unit,
        (geometry.clip.y - 0.5) * unit,
        geometry.clip.width * unit,
        geometry.clip.height * unit,
      );
      context.clip();
    }
    const xEdges = Array.from(
      { length: width + 1 },
      (_, x) => (geometry.x - 0.5 + geometry.width * x / width) * unit,
    );
    const yEdges = Array.from(
      { length: height + 1 },
      (_, y) => (geometry.y - 0.5 + geometry.height * y / height) * unit,
    );
    const occupied = new Set();
    for (const [pixelIndex, pixel] of (content.pixels || []).entries()) {
      const x = Number(pixel?.position?.[0]);
      const y = Number(pixel?.position?.[1]);
      const key = `${x}:${y}`;
      if (
        !Number.isSafeInteger(x)
        || !Number.isSafeInteger(y)
        || x < 0
        || y < 0
        || x >= width
        || y >= height
        || occupied.has(key)
      ) {
        throw new Error(`Resolved pixel batch ${batchIndex} has invalid pixel ${pixelIndex}.`);
      }
      occupied.add(key);
      context.fillStyle = this.linearRgbaCss(pixel.color);
      this.fillCanvasRect(
        context,
        xEdges[x],
        yEdges[y],
        xEdges[x + 1] - xEdges[x],
        yEdges[y + 1] - yEdges[y],
      );
    }
    if (geometry.clip != null) {
      context.restore();
    }
  }

  async paintResolvedRasterImage(context, content, pixelGeometry, unit, batchIndex) {
    if (pixelGeometry != null) {
      throw new Error(`Resolved raster batch ${batchIndex} unexpectedly has pixel geometry.`);
    }
    const sourceSize = content.sourceSize;
    if (
      !Array.isArray(sourceSize)
      || sourceSize.length !== 2
      || !sourceSize.every((value) => Number.isSafeInteger(value) && value > 0)
    ) {
      throw new Error(`Resolved raster batch ${batchIndex} has invalid source dimensions.`);
    }
    this.validateRect(content.destination, `Resolved raster batch ${batchIndex} destination`);
    this.validateRect(content.uv, `Resolved raster batch ${batchIndex} UV`);
    const uv = content.uv;
    if (uv.x < 0 || uv.y < 0 || uv.x + uv.width > 1 || uv.y + uv.height > 1) {
      throw new Error(`Resolved raster batch ${batchIndex} has out-of-range UV coordinates.`);
    }
    if (!window.PuzzleAssets?.url) {
      throw new Error("Resolved raster rendering requires the typed browser asset host.");
    }
    const asset = String(content.asset || "");
    const revision = String(content.revision || "");
    if (!asset || !revision) {
      throw new Error(`Resolved raster batch ${batchIndex} is missing asset identity.`);
    }
    const source = window.PuzzleAssets.url(asset);
    const image = await this.rasterImage(asset, revision, source);
    if (image.naturalWidth !== sourceSize[0] || image.naturalHeight !== sourceSize[1]) {
      throw new Error(
        `Resolved raster asset ${asset} dimensions ${image.naturalWidth}x${image.naturalHeight}`
        + ` do not match ${sourceSize[0]}x${sourceSize[1]}.`,
      );
    }
    const destination = content.destination;
    context.imageSmoothingEnabled = content.sampling === "smooth";
    context.drawImage(
      image,
      uv.x * sourceSize[0],
      uv.y * sourceSize[1],
      uv.width * sourceSize[0],
      uv.height * sourceSize[1],
      (destination.x - 0.5) * unit,
      (destination.y - 0.5) * unit,
      destination.width * unit,
      destination.height * unit,
    );
  }

  rasterImage(asset, revision, source) {
    const key = `${asset}\u0000${revision}`;
    let promise = this.constructor.rasterImages.get(key);
    if (!promise) {
      promise = new Promise((resolve, reject) => {
        const image = new Image();
        image.decoding = "async";
        image.onload = () => resolve(image);
        image.onerror = () => reject(new Error(`Resolved raster asset failed to load: ${asset}.`));
        image.src = source;
      });
      this.constructor.rasterImages.set(key, promise);
      promise.catch(() => this.constructor.rasterImages.delete(key));
    }
    return promise;
  }

  paintResolvedDecorations(context, view, unit, decorations) {
    for (const [index, decoration] of decorations.entries()) {
      if (decoration?.kind !== "lines2d" || decoration.layer !== "overlay") {
        throw new Error(`2D renderer received unsupported decoration ${index}: ${String(decoration?.kind)}`);
      }
      const style = decoration.style;
      const width = style?.width;
      let lineWidth = 0;
      if (width?.kind === "cell_relative") {
        const fraction = Number(width.cellFraction);
        const minimum = Number(width.minPhysicalPixels);
        if (!Number.isFinite(fraction) || fraction <= 0 || !Number.isFinite(minimum) || minimum <= 0) {
          throw new Error(`Resolved 2D decoration ${index} has invalid relative width.`);
        }
        lineWidth = Math.max(fraction * unit, minimum / this.canvasAxisScale(context, "x"));
      } else if (width?.kind === "physical_pixels") {
        const pixels = Number(width.pixels);
        if (!Number.isFinite(pixels) || pixels <= 0) {
          throw new Error(`Resolved 2D decoration ${index} has invalid physical width.`);
        }
        lineWidth = pixels / this.canvasAxisScale(context, "x");
      } else {
        throw new Error(`Resolved 2D decoration ${index} is missing typed width.`);
      }
      context.save();
      context.strokeStyle = this.linearRgbaCss(style.color);
      context.lineWidth = lineWidth;
      context.beginPath();
      for (const [segmentIndex, segment] of (decoration.segments || []).entries()) {
        const start = segment?.start;
        const end = segment?.end;
        if (
          !Array.isArray(start)
          || start.length !== 2
          || !start.every(Number.isFinite)
          || !Array.isArray(end)
          || end.length !== 2
          || !end.every(Number.isFinite)
          || (start[0] === end[0] && start[1] === end[1])
          || (start[0] !== end[0] && start[1] !== end[1])
        ) {
          throw new Error(`Resolved 2D decoration ${index} has invalid segment ${segmentIndex}.`);
        }
        context.moveTo((start[0] - view.x) * unit, (start[1] - view.y) * unit);
        context.lineTo((end[0] - view.x) * unit, (end[1] - view.y) * unit);
      }
      context.stroke();
      context.restore();
    }
  }

  validateRect(value, label) {
    if (
      !value
      || !["x", "y", "width", "height"].every((field) => Number.isFinite(value[field]))
      || value.width <= 0
      || value.height <= 0
    ) {
      throw new Error(`${label} is invalid.`);
    }
  }

  linearRgbaCss(color) {
    if (
      !color
      || !["red", "green", "blue", "alpha"].every((field) =>
        Number.isFinite(color[field]) && color[field] >= 0 && color[field] <= 1)
    ) {
      throw new Error("Resolved render color is invalid.");
    }
    const byte = (value) => {
      const srgb = value <= 0.0031308
        ? 12.92 * value
        : 1.055 * (value ** (1 / 2.4)) - 0.055;
      return Math.round(srgb * 255);
    };
    return `rgba(${byte(color.red)}, ${byte(color.green)}, ${byte(color.blue)}, ${color.alpha})`;
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

  canvasMetrics(canvas, view) {
    const rect = canvas.getBoundingClientRect();
    const presentationUnit = this.canvasPresentationCellUnit();
    const cssWidth = rect.width > 0 ? rect.width : view.width * presentationUnit;
    const cssHeight = rect.height > 0 ? rect.height : view.height * presentationUnit;
    if (cssWidth <= 0 || cssHeight <= 0) {
      return null;
    }
    const unit = Math.max(0.0001, Math.min(cssWidth / view.width, cssHeight / view.height));
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
}

window.PuzzleRenderer = PuzzleRenderer;
