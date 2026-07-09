class PuzzleRenderer {
  constructor(root, options = {}) {
    this.root = root;
    this.options = options;
    this.appliedThemeClass = "";
    this.activeTriggerAnimations = [];
    this.domAnimationToken = 0;
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
    this.domAnimationToken += 1;

    if (this.options.renderMode === "dom") {
      for (const cellData of this.viewportCells(scene, viewport)) {
        this.root.append(this.renderCell(cellData, scene));
      }
      this.startDomAnimationLoop();
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
    const baseFrame = this.firstVisualFrame(definition);
    const fit = this.spriteFit(baseFrame);
    const sampling = this.spriteSampling(baseFrame);
    sprite.className = `sprite visual-sprite visual-fit-${fit.mode} visual-sampling-${sampling} ${definition.className || ""} ${layer.sprite} visual-${this.classNameFor(visualKey)}`;
    sprite.dataset.object = layer.object;
    sprite.dataset.layer = layer.layer;
    sprite.style.zIndex = String(definition.zIndex ?? layer.layer + 1);
    const { cols: spriteCols, rows: spriteRows } = this.spritePatternSize(baseFrame);
    const { cols: boxCols, rows: boxRows } = this.spriteDrawBox(baseFrame);
    sprite.style.setProperty("--sprite-cols", String(spriteCols));
    sprite.style.setProperty("--sprite-rows", String(spriteRows));
    sprite.style.setProperty("--sprite-box-cols", String(boxCols));
    sprite.style.setProperty("--sprite-box-rows", String(boxRows));
    sprite.style.setProperty("--sprite-offset-x", String(Number(baseFrame.offset?.x) || 0));
    sprite.style.setProperty("--sprite-offset-y", String(Number(baseFrame.offset?.y) || 0));
    sprite.setAttribute("aria-hidden", "true");
    if (this.visualFrameCount(definition) > 1) {
      sprite.__visualAnimationDefinition = definition;
    }

    if (baseFrame.source) {
      sprite.classList.add("visual-image");
      const source = window.PuzzleAssets.url(baseFrame.source);
      sprite.style.backgroundImage = `url("${String(source).replace(/"/g, '\\"')}")`;
      return sprite;
    }

    const solidColor = this.solidPatternColor(baseFrame);
    if (solidColor && this.canPaintAsFullCellSolid(baseFrame)) {
      sprite.classList.add("visual-solid");
      sprite.style.backgroundColor = solidColor;
      return sprite;
    }

    sprite.classList.add("visual-pattern");
    sprite.style.backgroundImage = `url("${this.patternDataUrl(baseFrame)}")`;

    return sprite;
  }

  startDomAnimationLoop() {
    const token = this.domAnimationToken;
    const animatedSprites = [...this.root.querySelectorAll(".visual-sprite")]
      .filter((sprite) => sprite.__visualAnimationDefinition);
    if (!animatedSprites.length) {
      return;
    }
    const draw = () => {
      if (token !== this.domAnimationToken || !this.root.isConnected) {
        return;
      }
      const now = performance.now();
      for (const sprite of animatedSprites) {
        if (!sprite.isConnected) {
          continue;
        }
        const definition = sprite.__visualAnimationDefinition;
        const frame = this.resolveVisualFrame(definition, this.loopAnimationTimeMs(now, definition));
        this.applyDomVisualFrame(sprite, frame);
      }
      requestAnimationFrame(draw);
    };
    requestAnimationFrame(draw);
  }

  applyDomVisualFrame(sprite, frame) {
    sprite.classList.remove("visual-image", "visual-solid", "visual-pattern");
    for (const className of [...sprite.classList]) {
      if (className.startsWith("visual-fit-") || className.startsWith("visual-sampling-")) {
        sprite.classList.remove(className);
      }
    }
    const fit = this.spriteFit(frame);
    sprite.classList.add(`visual-fit-${fit.mode}`, `visual-sampling-${this.spriteSampling(frame)}`);
    const { cols: spriteCols, rows: spriteRows } = this.spritePatternSize(frame);
    const { cols: boxCols, rows: boxRows } = this.spriteDrawBox(frame);
    sprite.style.setProperty("--sprite-cols", String(spriteCols));
    sprite.style.setProperty("--sprite-rows", String(spriteRows));
    sprite.style.setProperty("--sprite-box-cols", String(boxCols));
    sprite.style.setProperty("--sprite-box-rows", String(boxRows));
    sprite.style.setProperty("--sprite-offset-x", String(Number(frame.offset?.x) || 0));
    sprite.style.setProperty("--sprite-offset-y", String(Number(frame.offset?.y) || 0));
    sprite.style.backgroundColor = "";
    sprite.style.backgroundImage = "";

    if (frame.source) {
      sprite.classList.add("visual-image");
      const source = window.PuzzleAssets.url(frame.source);
      sprite.style.backgroundImage = `url("${String(source).replace(/"/g, '\\"')}")`;
      return;
    }

    const solidColor = this.solidPatternColor(frame);
    if (solidColor && this.canPaintAsFullCellSolid(frame)) {
      sprite.classList.add("visual-solid");
      sprite.style.backgroundColor = solidColor;
      return;
    }

    sprite.classList.add("visual-pattern");
    sprite.style.backgroundImage = `url("${this.patternDataUrl(frame)}")`;
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
    this.recordTriggerAnimations(scene.animationEvents || [], frame);
    let startedAt = null;
    let animationFrameIndex = 0;
    const duration = this.animationDurationMs(scene);
    const hasLoopAnimations = this.sceneUsesTimeVaryingVisuals(scene, frame);
    const draw = () => {
      if (!canvas.isConnected) {
        return;
      }
      if (!this.root.isConnected) {
        requestAnimationFrame(draw);
        return;
      }
      const now = performance.now();
      startedAt ??= performance.now();
      const progress = animations.length
        ? this.animationProgressForFrame(performance.now() - startedAt, duration, animationFrameIndex)
        : 1;
      this.pruneTriggerAnimations(now);
      context.clearRect(0, 0, canvas.width, canvas.height);
      this.paintCanvas(context, scene, frame, unit, animations, progress, now);
      if (animations.length) {
        animationFrameIndex += 1;
      }
      if ((progress < 1 || hasLoopAnimations || this.activeTriggerAnimations.length) && this.root.isConnected) {
        requestAnimationFrame(draw);
      }
    };
    this.root.append(canvas);
    requestAnimationFrame(draw);
  }

  paintCanvas(context, scene, frame, unit, animations = [], progress = 1, now = performance.now()) {
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
        this.paintCanvasLayer(context, layer, x, y, unit, null, progress, now);
      }
    }
    for (const item of animatedLayers) {
      this.paintCanvasLayer(context, item.layer, item.x, item.y, unit, item.animation, progress);
    }
    this.paintTriggerAnimations(context, frame, unit, now);
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

  paintCanvasLayer(context, layer, x, y, unit, animation = null, progress = 1, now = performance.now()) {
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
    const frame = this.resolveVisualFrame(definition, this.loopAnimationTimeMs(now, definition));

    if (frame.source) {
      const image = this.cachedImage(frame.source);
      if (image?.complete && image.naturalWidth > 0) {
        const fit = this.visualSpriteFit(frame, unit, {
          cols: image.naturalWidth,
          rows: image.naturalHeight,
        });
        const clip = this.visualSpriteBox(frame, unit);
        context.save();
        context.imageSmoothingEnabled = this.spriteSampling(frame) === "smooth";
        context.beginPath();
        context.rect(x + clip.x, y + clip.y, clip.width, clip.height);
        context.clip();
        context.drawImage(
          image,
          x + fit.x,
          y + fit.y,
          fit.width,
          fit.height,
        );
        context.restore();
      }
      if (transform) {
        context.restore();
      }
      return;
    }

    const solidColor = this.solidPatternColor(frame);
    if (solidColor && this.canPaintAsFullCellSolid(frame)) {
      context.fillStyle = solidColor;
      context.fillRect(x, y, unit, unit);
      if (transform) {
        context.restore();
      }
      return;
    }

    const fit = this.visualSpriteFit(frame, unit);
    const clip = this.visualSpriteBox(frame, unit);
    context.save();
    context.imageSmoothingEnabled = this.spriteSampling(frame) === "smooth";
    context.beginPath();
    context.rect(x + clip.x, y + clip.y, clip.width, clip.height);
    context.clip();
    this.paintPattern(context, frame, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight);
    context.restore();
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
    if (names.has("turn")) {
      transform.angle = (animation.kind === "move" ? 1 - eased : Math.sin(eased * Math.PI)) * Math.PI * 0.5;
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
      const name = String(event.name || event.animation || event.sprite || "");
      const definition = this.resolveAnimationDefinition(name);
      if (!definition) {
        continue;
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
        startedAtMs: now,
        durationMs,
      });
    }
  }

  isTriggerAnimationEvent(event) {
    return event?.kind === "sprite_animation"
      || event?.kind === "trigger_animation"
      || event?.kind === "trigger"
      || event?.kind === "animation";
  }

  triggerAnimationPosition(event) {
    return event.position || event.at || event.cell || event.to || null;
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

  paintTriggerAnimations(context, frame, unit, now) {
    for (const instance of this.activeTriggerAnimations) {
      const elapsedMs = now - instance.startedAtMs;
      if (elapsedMs < 0 || elapsedMs >= instance.durationMs) {
        continue;
      }
      const visualFrame = this.resolveVisualFrame(instance.definition, elapsedMs, {
        playback: "once",
        durationMs: instance.durationMs,
      });
      const x = (instance.x - frame.x) * unit;
      const y = (instance.y - frame.y) * unit;
      this.paintResolvedCanvasFrame(context, visualFrame, x, y, unit);
    }
  }

  paintResolvedCanvasFrame(context, definition, x, y, unit) {
    if (definition.source) {
      const image = this.cachedImage(definition.source);
      if (image?.complete && image.naturalWidth > 0) {
        const fit = this.visualSpriteFit(definition, unit, {
          cols: image.naturalWidth,
          rows: image.naturalHeight,
        });
        const clip = this.visualSpriteBox(definition, unit);
        context.save();
        context.imageSmoothingEnabled = this.spriteSampling(definition) === "smooth";
        context.beginPath();
        context.rect(x + clip.x, y + clip.y, clip.width, clip.height);
        context.clip();
        context.drawImage(image, x + fit.x, y + fit.y, fit.width, fit.height);
        context.restore();
      }
      return;
    }

    const solidColor = this.solidPatternColor(definition);
    if (solidColor && this.canPaintAsFullCellSolid(definition)) {
      context.fillStyle = solidColor;
      context.fillRect(x, y, unit, unit);
      return;
    }

    const fit = this.visualSpriteFit(definition, unit);
    const clip = this.visualSpriteBox(definition, unit);
    context.save();
    context.imageSmoothingEnabled = this.spriteSampling(definition) === "smooth";
    context.beginPath();
    context.rect(x + clip.x, y + clip.y, clip.width, clip.height);
    context.clip();
    this.paintPattern(context, definition, x + fit.x, y + fit.y, fit.pixelWidth, fit.pixelHeight);
    context.restore();
  }

  resolveAnimationDefinition(name) {
    const visuals = this.visuals();
    return visuals.animations?.[name]
      || visuals.triggers?.[name]
      || visuals.sprites?.[name]
      || null;
  }

  sceneUsesTimeVaryingVisuals(scene, frame) {
    for (const cell of this.frameCells(scene, frame)) {
      for (const layer of cell.layers || []) {
        const definition = this.resolveVisualSprite(layer)?.definition;
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
      offset: frame.offset || definition.offset,
      sampling: frame.sampling || definition.sampling,
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

  paintPattern(context, definition, x, y, pixelWidth, pixelHeight = pixelWidth) {
    const pattern = definition.pattern || [];
    pattern.forEach((row, rowIndex) => {
      [...row].forEach((token, colIndex) => {
        const color = definition.colors?.[token] || "transparent";
        if (!color || color === "transparent") {
          return;
        }
        context.fillStyle = color;
        const left = Math.round(x + colIndex * pixelWidth);
        const right = Math.round(x + (colIndex + 1) * pixelWidth);
        const top = Math.round(y + rowIndex * pixelHeight);
        const bottom = Math.round(y + (rowIndex + 1) * pixelHeight);
        context.fillRect(
          left,
          top,
          Math.max(1, right - left),
          Math.max(1, bottom - top),
        );
      });
    });
  }

  visualSpriteFit(definition, unit, sourceSize = null) {
    const source = sourceSize || this.spritePatternSize(definition);
    const sourceCols = Math.max(1, Number(source.cols) || Number(source.width) || 1);
    const sourceRows = Math.max(1, Number(source.rows) || Number(source.height) || 1);
    const box = this.spriteDrawBox(definition);
    const boxWidth = box.cols * unit;
    const boxHeight = box.rows * unit;
    const mode = this.spriteFit(definition).mode;
    const scaleX = boxWidth / sourceCols;
    const scaleY = boxHeight / sourceRows;
    const scale = mode === "cover" ? Math.max(scaleX, scaleY) : Math.min(scaleX, scaleY);
    const width = mode === "stretch" ? boxWidth : sourceCols * scale;
    const height = mode === "stretch" ? boxHeight : sourceRows * scale;
    const offsetX = (Number(definition.offset?.x) || 0) * unit;
    const offsetY = (Number(definition.offset?.y) || 0) * unit;
    return {
      x: (unit - boxWidth) / 2 + (boxWidth - width) / 2 + offsetX,
      y: (unit - boxHeight) / 2 + (boxHeight - height) / 2 + offsetY,
      width,
      height,
      scale,
      pixelWidth: width / sourceCols,
      pixelHeight: height / sourceRows,
    };
  }

  visualSpriteBox(definition, unit) {
    const box = this.spriteDrawBox(definition);
    const width = box.cols * unit;
    const height = box.rows * unit;
    return {
      x: (unit - width) / 2 + (Number(definition.offset?.x) || 0) * unit,
      y: (unit - height) / 2 + (Number(definition.offset?.y) || 0) * unit,
      width,
      height,
    };
  }

  visualSpriteOffset(definition, unit) {
    return {
      x: (Number(definition.offset?.x) || 0) * unit,
      y: (Number(definition.offset?.y) || 0) * unit,
    };
  }

  cachedImage(source) {
    const url = window.PuzzleAssets.url(source);
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
        for (const frameDef of this.visualFrames(definition)) {
          if (frameDef.source) {
            hasImage = true;
            continue;
          }
          const { cols: cellCols, rows: cellRows } = this.spritePatternSize(frameDef);
          unit = Math.max(unit, cellCols, cellRows);
        }
      }
    }
    return hasImage ? Math.max(unit, 32) : unit;
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
    const { cols: width, rows: height } = this.spritePatternSize(definition);
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

  canPaintAsFullCellSolid(definition) {
    if (definition.pixelsPerCell || Number(definition.offset?.x) || Number(definition.offset?.y)) {
      return false;
    }
    const fit = this.spriteFit(definition);
    if (fit.width !== 1 || fit.height !== 1) {
      return false;
    }
    if (fit.mode !== "contain") {
      return true;
    }
    const pattern = this.spritePatternSize(definition);
    return pattern.cols === pattern.rows;
  }

  spritePatternSize(definition) {
    const pattern = definition.pattern || [];
    return {
      cols: Math.max(1, ...pattern.map((row) => String(row).length), 1),
      rows: Math.max(1, pattern.length || 1),
    };
  }

  spriteCellGrid(definition) {
    const pattern = this.spritePatternSize(definition);
    return {
      cols: Math.max(1, Number(definition.pixelsPerCell?.width) || pattern.cols),
      rows: Math.max(1, Number(definition.pixelsPerCell?.height) || pattern.rows),
    };
  }

  spriteDrawBox(definition) {
    const fit = this.spriteFit(definition);
    return {
      cols: Math.max(1, Number(fit.width) || 1),
      rows: Math.max(1, Number(fit.height) || 1),
    };
  }

  spriteFit(definition) {
    const fit = definition.fit || {};
    const mode = ["contain", "cover", "stretch"].includes(fit.mode) ? fit.mode : "contain";
    return {
      mode,
      width: Math.max(1, Number(fit.width) || 1),
      height: Math.max(1, Number(fit.height) || 1),
    };
  }

  spriteSampling(definition) {
    if (definition.sampling === "smooth" || definition.sampling === "pixelated") {
      return definition.sampling;
    }
    return definition.source && !/\.png$/i.test(definition.source) ? "smooth" : "pixelated";
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
