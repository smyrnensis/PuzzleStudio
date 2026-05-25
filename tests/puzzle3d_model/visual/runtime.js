window.Puzzle3DTestRuntime = {
  create(initialSnapshot) {
    return new Puzzle3DTestRuntime(initialSnapshot);
  },
};

const PUZZLE3_UNTIL_STABLE_REPEAT_LIMIT = 200;

class Puzzle3DTestRuntime {
  constructor(initialSnapshot) {
    this.base = cloneSnapshot(initialSnapshot);
    this.levels = cloneLevels(initialSnapshot.levels?.length
      ? initialSnapshot.levels
      : [snapshotLevel(initialSnapshot)]);
    this.levelIndex = clampIndex(initialSnapshot.levelIndex || 0, this.levels.length);
    this.undoStack = [];
    this.moveCount = 0;
    this.levelFiredRules = new Set();
    this.cells = [];
    this.loadLevel(this.levelIndex);
    this.completed = this.isComplete();
  }

  snapshot() {
    const level = this.currentLevel();
    return {
      ...this.base,
      size: { ...level.size },
      cells: cloneCells(this.cells),
      levelIndex: this.levelIndex,
      levelCount: this.levels.length,
      levelName: level.name,
      levelLabel: level.label || level.name,
      hasNextLevel: this.hasNextLevel(),
      hasPreviousLevel: this.hasPreviousLevel(),
      moveCount: this.moveCount,
      completed: this.completed,
    };
  }

  applyInput(inputName) {
    const inputId = this.inputIdForName(inputName);
    if (inputId === undefined) {
      return false;
    }
    const before = this.historyEntry();
    const next = this.transitionProgram(this.cells, this.base.rules || [], inputId);
    if (sameCells(next, this.cells)) {
      return false;
    }
    this.undoStack.push(before);
    this.cells = next;
    this.moveCount += 1;
    const wasCompleted = this.completed;
    this.completed = this.isComplete();
    if (!wasCompleted && this.completed) {
      this.runLevelClearLifecycle();
    }
    return true;
  }

  undo() {
    const previous = this.undoStack.pop();
    if (!previous) {
      return false;
    }
    this.cells = cloneCells(previous.cells);
    this.moveCount = previous.moveCount;
    this.completed = previous.completed;
    this.levelFiredRules = new Set(previous.levelFiredRules || []);
    return true;
  }

  restart() {
    const initialCells = this.initialCellsForCurrentLevel();
    const changed = !sameCells(this.cells, initialCells)
      || this.moveCount !== 0
      || this.completed
      || this.undoStack.length > 0
      || this.levelFiredRules.size > 0;
    if (!changed) {
      return false;
    }
    this.undoStack.push(this.historyEntry());
    this.cells = initialCells;
    this.levelFiredRules = new Set();
    this.moveCount = 0;
    this.completed = this.isComplete();
    return true;
  }

  historyEntry() {
    return {
      cells: cloneCells(this.cells),
      moveCount: this.moveCount,
      completed: this.completed,
      levelFiredRules: [...this.levelFiredRules],
    };
  }

  nextLevel() {
    if (!this.hasNextLevel()) {
      return false;
    }
    this.loadLevel(this.levelIndex + 1);
    return true;
  }

  previousLevel() {
    if (!this.hasPreviousLevel()) {
      return false;
    }
    this.loadLevel(this.levelIndex - 1);
    return true;
  }

  hasNextLevel() {
    return this.levelIndex + 1 < this.levels.length;
  }

  hasPreviousLevel() {
    return this.levelIndex > 0;
  }

  loadLevel(levelIndex) {
    this.levelIndex = clampIndex(levelIndex, this.levels.length);
    this.levelFiredRules = new Set();
    this.cells = this.initialCellsForCurrentLevel();
    this.undoStack = [];
    this.moveCount = 0;
    this.completed = this.isComplete();
  }

  initialCellsForCurrentLevel() {
    const raw = cloneCells(this.currentLevel().cells);
    return this.transitionProgram(raw, this.base.lifecycle?.onLevelStart || [], undefined);
  }

  currentLevel() {
    return this.levels[this.levelIndex];
  }

  runLevelClearLifecycle() {
    for (const command of this.base.lifecycle?.onLevelClear || []) {
      if (command === "next_level") {
        this.nextLevel();
      }
    }
  }

  transitionProgram(initialCells, rules, inputId) {
    let current = cloneCells(initialCells);
    for (const rule of rules || []) {
      if (!this.guardsAccept(rule, inputId)) {
        continue;
      }
      if (rule.application === "once_all") {
        current = this.transitionOnceAll(current, rule, inputId);
      } else if (rule.application === "once_per_level") {
        current = this.transitionOncePerLevel(current, rule, inputId);
      } else if (rule.application === "until_stable") {
        current = this.transitionUntilStable(current, rule, inputId);
      } else {
        current = this.transitionOnce(current, rule, inputId);
      }
    }
    return current;
  }

  transitionOnce(cells, rule, inputId) {
    if (!this.guardsAccept(rule, inputId)) {
      return cloneCells(cells);
    }
    const origin = this.firstMatch(cells, rule.pattern);
    if (!origin) {
      return cloneCells(cells);
    }
    return this.applyPatch(cells, this.buildPatch(origin, rule.writes || []), false);
  }

  transitionOnceAll(cells, rule, inputId) {
    if (!this.guardsAccept(rule, inputId)) {
      return cloneCells(cells);
    }
    const origins = this.allMatches(cells, rule.pattern);
    let current = cloneCells(cells);
    for (const origin of origins) {
      if (!this.patternMatchesAt(current, rule.pattern, origin)) {
        continue;
      }
      try {
        current = this.applyPatch(current, this.buildPatch(origin, rule.writes || []), true);
      } catch (error) {
        if (!error?.stale) {
          throw error;
        }
      }
    }
    return current;
  }

  transitionOncePerLevel(cells, rule, inputId) {
    const ruleId = rule.id ?? 0;
    if (this.levelFiredRules.has(ruleId) || !this.guardsAccept(rule, inputId)) {
      return cloneCells(cells);
    }
    const origin = this.firstMatch(cells, rule.pattern);
    if (!origin) {
      return cloneCells(cells);
    }
    const next = this.applyPatch(cells, this.buildPatch(origin, rule.writes || []), false);
    this.levelFiredRules.add(ruleId);
    return next;
  }

  transitionUntilStable(cells, rule, inputId) {
    let current = cloneCells(cells);
    const seen = new Set([canonicalCellsKey(current)]);
    let repeatCount = 0;
    for (;;) {
      const next = this.transitionOnceAll(current, rule, inputId);
      if (sameCells(next, current)) {
        return current;
      }
      const key = canonicalCellsKey(next);
      if (seen.has(key)) {
        console.warn(`until_stable cycle in rule ${rule.id ?? 0}; ending repeat at current state`);
        return next;
      }
      seen.add(key);
      current = next;
      repeatCount += 1;
      if (repeatCount >= PUZZLE3_UNTIL_STABLE_REPEAT_LIMIT) {
        console.warn(`until_stable repeat limit reached in rule ${rule.id ?? 0}; ending repeat at current state`);
        return current;
      }
    }
  }

  guardsAccept(rule, inputId) {
    return (rule.guards || []).every((guard) => {
      if (guard.kind === "input_is") {
        return inputId !== undefined && Number(guard.input) === Number(inputId);
      }
      return true;
    });
  }

  firstMatch(cells, pattern) {
    return this.allMatches(cells, pattern)[0] || null;
  }

  allMatches(cells, pattern) {
    const matches = [];
    const size = this.currentLevel().size;
    for (let z = 0; z < size.height; z += 1) {
      for (let y = 0; y < size.depth; y += 1) {
        for (let x = 0; x < size.width; x += 1) {
          const origin = { x, y, z };
          if (this.patternMatchesAt(cells, pattern, origin)) {
            matches.push(origin);
          }
        }
      }
    }
    return matches;
  }

  patternMatchesAt(cells, pattern, origin) {
    return (pattern?.cells || []).every((cell) => {
      const position = offsetPosition(origin, cell.offset || {});
      if (!position || !this.isInside(position)) {
        return false;
      }
      return (cell.require || []).every((objectId) => this.hasObject(cells, position, objectId))
        && (cell.forbid || []).every((objectId) => !this.hasObject(cells, position, objectId));
    });
  }

  buildPatch(origin, writes) {
    return writes.map((write) => {
      if (write.kind === "add") {
        return {
          kind: "add",
          position: checkedOffsetPosition(origin, write.offset),
          object: Number(write.object),
        };
      }
      if (write.kind === "remove") {
        return {
          kind: "remove",
          position: checkedOffsetPosition(origin, write.offset),
          object: Number(write.object),
        };
      }
      if (write.kind === "replace") {
        return {
          kind: "replace",
          position: checkedOffsetPosition(origin, write.offset),
          remove: Number(write.remove),
          add: Number(write.add),
        };
      }
      if (write.kind === "move") {
        return {
          kind: "move",
          from: checkedOffsetPosition(origin, write.fromOffset),
          to: checkedOffsetPosition(origin, write.toOffset),
          object: Number(write.object),
        };
      }
      throw new Error(`unknown write op: ${write.kind}`);
    });
  }

  applyPatch(cells, patch, staleIsError) {
    const next = cloneCells(cells);
    try {
      for (const op of patch) {
        if (op.kind === "remove") {
          this.removeObject(next, op.position, op.object);
        } else if (op.kind === "move") {
          this.removeObject(next, op.from, op.object);
        } else if (op.kind === "replace") {
          this.removeObject(next, op.position, op.remove);
        }
      }
      for (const op of patch) {
        if (op.kind === "add") {
          this.placeObject(next, op.position, op.object);
        } else if (op.kind === "move") {
          this.placeObject(next, op.to, op.object);
        } else if (op.kind === "replace") {
          this.placeObject(next, op.position, op.add);
        }
      }
    } catch (error) {
      if (staleIsError && (error.code === "object_not_present" || error.code === "layer_occupied")) {
        error.stale = true;
      }
      throw error;
    }
    return next;
  }

  hasObject(cells, position, objectId) {
    return cellAt(cells, position)?.objects.some((object) => object.id === Number(objectId)) || false;
  }

  placeObject(cells, position, objectId) {
    if (!this.isInside(position)) {
      throw transitionError("position_out_of_bounds");
    }
    const layer = this.objectLayer(objectId);
    const cell = ensureCell(cells, position);
    const existing = cell.objects.find((object) => this.objectLayer(object.id) === layer);
    if (existing && existing.id !== Number(objectId)) {
      throw transitionError("layer_occupied");
    }
    if (!existing) {
      cell.objects.push(this.objectForId(objectId));
    }
  }

  removeObject(cells, position, objectId) {
    const cell = cellAt(cells, position);
    const index = cell?.objects.findIndex((object) => object.id === Number(objectId)) ?? -1;
    if (!cell || index < 0) {
      throw transitionError("object_not_present");
    }
    cell.objects.splice(index, 1);
    pruneEmptyCells(cells);
  }

  objectForId(objectId) {
    const object = Object.values(this.base.objects || {}).find((candidate) => candidate.id === Number(objectId));
    return object ? { ...object } : { id: Number(objectId), name: `Object ${objectId}`, sprite: `Object ${objectId}` };
  }

  objectLayer(objectId) {
    const object = Object.values(this.base.objects || {}).find((candidate) => candidate.id === Number(objectId));
    return Number(object?.layer ?? 0);
  }

  inputIdForName(inputName) {
    const input = (this.base.inputs || []).find((candidate) => candidate.name === inputName);
    return input ? Number(input.id) : undefined;
  }

  isInside(position) {
    const size = this.currentLevel().size;
    return position.x >= 0
      && position.x < size.width
      && position.y >= 0
      && position.y < size.depth
      && position.z >= 0
      && position.z < size.height;
  }

  isComplete() {
    return evaluateWinCondition(this, this.base.winCondition, this.cells);
  }
}

function evaluateWinCondition(runtime, condition, cells) {
  if (!condition) {
    return false;
  }
  if (condition.kind === "all") {
    return (condition.conditions || []).every((child) => evaluateWinCondition(runtime, child, cells));
  }
  if (condition.kind === "any") {
    return (condition.conditions || []).some((child) => evaluateWinCondition(runtime, child, cells));
  }
  if (condition.kind === "some_object") {
    return runtime.firstMatch(cells, singleObjectPattern(condition.object)) !== null;
  }
  if (condition.kind === "no_object") {
    return runtime.firstMatch(cells, singleObjectPattern(condition.object)) === null;
  }
  if (condition.kind === "some_pattern") {
    return runtime.firstMatch(cells, condition.pattern) !== null;
  }
  if (condition.kind === "no_pattern") {
    return runtime.firstMatch(cells, condition.pattern) === null;
  }
  if (condition.kind === "all_objects_covered_by_pattern") {
    return runtime.allMatches(cells, singleObjectPattern(condition.object)).length
      === runtime.allMatches(cells, condition.coverPattern).length;
  }
  return false;
}

function singleObjectPattern(objectId) {
  return {
    cells: [{
      offset: { dx: 0, dy: 0, dz: 0 },
      require: [Number(objectId)],
      forbid: [],
    }],
  };
}

function cloneSnapshot(snapshot) {
  return {
    ...snapshot,
    size: { ...snapshot.size },
    camera: snapshot.camera ? { ...snapshot.camera } : undefined,
    settings: { ...(snapshot.settings || {}) },
    directions: cloneRecord(snapshot.directions || {}),
    directionSets: cloneRecord(snapshot.directionSets || {}),
    controls: {
      keys: { ...(snapshot.controls?.keys || {}) },
    },
    inputs: cloneJson(snapshot.inputs || []),
    rules: cloneJson(snapshot.rules || []),
    winCondition: snapshot.winCondition ? cloneJson(snapshot.winCondition) : undefined,
    lifecycle: cloneLifecycle(snapshot.lifecycle || {}),
    objects: cloneObjects(snapshot.objects || {}),
    sprites: cloneSprites(snapshot.sprites || {}),
    cells: cloneCells(snapshot.cells || []),
    levels: cloneLevels(snapshot.levels || []),
    levelBundles: cloneLevelBundles(snapshot.levelBundles || {}),
  };
}

function cloneLifecycle(lifecycle) {
  return {
    onLevelStart: cloneJson(lifecycle.onLevelStart || []),
    onLevelClear: [...(lifecycle.onLevelClear || [])],
  };
}

function cloneRecord(record) {
  return Object.fromEntries(
    Object.entries(record).map(([key, value]) => [
      key,
      Array.isArray(value) ? [...value] : { ...value },
    ]),
  );
}

function cloneObjects(objects) {
  return Object.fromEntries(
    Object.entries(objects).map(([name, object]) => [name, { ...object }]),
  );
}

function cloneSprites(sprites) {
  return Object.fromEntries(
    Object.entries(sprites).map(([name, sprite]) => [
      name,
      {
        ...sprite,
        palette: { ...(sprite.palette || {}) },
        bitmap: [...(sprite.bitmap || [])],
      },
    ]),
  );
}

function cloneCells(cells) {
  return cells.map((cell) => ({
    position: { ...cell.position },
    objects: cell.objects.map((object) => ({ ...object })),
  }));
}

function cloneLevels(levels) {
  return levels.map((level, index) => ({
    name: level.name || `level_${index + 1}`,
    label: level.label || level.name || `Level ${index + 1}`,
    size: { ...level.size },
    cells: cloneCells(level.cells || []),
  }));
}

function cloneLevelBundles(levelBundles) {
  return Object.fromEntries(
    Object.entries(levelBundles).map(([name, indexes]) => [
      name,
      Array.isArray(indexes) ? indexes.map((index) => Number(index)) : [],
    ]),
  );
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function snapshotLevel(snapshot) {
  return {
    name: snapshot.levelName || "level_1",
    label: snapshot.levelLabel || snapshot.levelName || "Level 1",
    size: { ...snapshot.size },
    cells: cloneCells(snapshot.cells || []),
  };
}

function clampIndex(index, length) {
  return Math.max(0, Math.min(Math.max(0, length - 1), index));
}

function offsetPosition(position, offset) {
  const x = position.x + Number(offset?.dx || 0);
  const y = position.y + Number(offset?.dy || 0);
  const z = position.z + Number(offset?.dz || 0);
  if (x < 0 || y < 0 || z < 0) {
    return null;
  }
  return { x, y, z };
}

function checkedOffsetPosition(position, offset) {
  const next = offsetPosition(position, offset);
  if (!next) {
    throw transitionError("offset_out_of_bounds");
  }
  return next;
}

function cellAt(cells, position) {
  return cells.find((cell) => samePosition(cell.position, position)) || null;
}

function ensureCell(cells, position) {
  let cell = cellAt(cells, position);
  if (!cell) {
    cell = { position: { ...position }, objects: [] };
    cells.push(cell);
  }
  return cell;
}

function pruneEmptyCells(cells) {
  for (let index = cells.length - 1; index >= 0; index -= 1) {
    if (cells[index].objects.length === 0) {
      cells.splice(index, 1);
    }
  }
}

function samePosition(a, b) {
  return a.x === b.x && a.y === b.y && a.z === b.z;
}

function sameCells(a, b) {
  return canonicalCellsKey(a) === canonicalCellsKey(b);
}

function canonicalCellsKey(cells) {
  return JSON.stringify(canonicalCells(cells));
}

function canonicalCells(cells) {
  return cloneCells(cells)
    .map((cell) => ({
      position: cell.position,
      objects: cell.objects.sort((a, b) => a.id - b.id || String(a.name).localeCompare(String(b.name))),
    }))
    .sort((a, b) => a.position.z - b.position.z
      || a.position.y - b.position.y
      || a.position.x - b.position.x);
}

function transitionError(code) {
  const error = new Error(code);
  error.code = code;
  return error;
}
