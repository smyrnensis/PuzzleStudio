(function () {
  const CORE_STATE_HASH_PROPERTY = "__puzzleCoreStateHash";

  class PuzzleStandaloneRuntime {
    constructor(exportData) {
      this.data = exportData;
      this.data.scenes = this.data.scenes || this.data.screens || [];
      this.data.screens = this.data.screens || this.data.scenes;
      this.engine = exportData.engine;
      this.compiledPlay = exportData.compiledPlay || null;
      this.objectsById = new Map(this.engine.objects.map((object) => [object.id, object]));
      this.objectLayers = new Map(this.engine.objects.map((object) => [object.id, object.layer]));
      this.queriesById = new Map(this.engine.queries.map((query) => [query.id, query.queryKind]));
      this.globalIdsByName = new Map((this.engine.globals || []).map((global) => [global.name, global.id]));
      this.varNamesById = new Map((this.engine.globals || []).map((global) => [global.id, global.name]));
      this.inputIdsByName = new Map(exportData.inputs.map((input) => [input.name, input.id]));
      this.inputNamesById = new Map(exportData.inputs.map((input) => [input.id, input.name]));
      this.visualObjectIds = new Set(this.engine.visualObjects || []);
      this.persistentVarIds = [...(this.engine.persistentVars || [])];
      this.levelIndex = null;
      this.state = this.neutralState();
      this.levelCheckpointState = null;
      this.persistentVars = this.persistentVarIds.map((varId) => this.persistentVarDefaultValue(varId));
      this.undoStack = [];
      this.redoStack = [];
      this.clearedLevels = new Array(this.data.levels.length).fill(false);
      this.restoredLevelIndex = null;
      this.hasProgressSave = false;
      this.restoreProgressSave();
      this.focusedScene = this.initialSceneName();
      this.visibleScenes = [this.focusedScene];
      this.focusHistory = [];
      this.sceneStates = new Map();
      this.selectedLevelIndex = this.clampLevelIndex(this.restoredLevelIndex ?? exportData.initialLevelIndex ?? 0);
      this.sessionValues = {};
      for (const variable of this.data.variables || []) {
        this.sessionValues[variable.name] = variable.default;
      }
      this.soundEvents = [];
      this.messageEvents = [];
      this.animationEvents = [];
      this.pendingWaits = 0;
      this.pendingAgainTurns = 0;
      this.againRunToken = 0;
      this.defaultAgainMs = Number(exportData.defaultAgainMs ?? 120);
      this.currentInput = null;
      this.currentTurnSfx = null;
      this.maxAgainTurnsPerInput = 256;
      this.coreRuntime = null;
      this.wasmModule = null;
      this.sessionRuntime = null;
      this.usesRustSession = false;
      this.coreRuntimeStateHash = null;
      this.editorPreviewSceneEnabled = false;
      this.editorPreviewInputEnabled = false;
      this.initialized = false;
      this.initializationPromise = this.initializeRuntime();
    }

    async requestJson(url, options = {}) {
      await this.ensureInitialized();
      const method = options.method || "GET";
      if (this.usesRustSession && this.sessionRuntime) {
        return this.sessionRequestJson(method, url);
      }
      if (method === "GET" && url === "/api/state") {
        return this.snapshot();
      }
      if (method === "POST" && url === "/api/solve") {
        throw new Error("Built-in solver has been removed; use the current solver instead.");
      }
      if (method === "POST" && url === "/api/command/undo") {
        this.undo();
        return this.snapshot();
      }
      if (method === "POST" && url === "/api/command/redo") {
        this.redo();
        return this.snapshot();
      }
      if (method === "POST" && url === "/api/command/restart") {
        this.restartLevel();
        return this.snapshot();
      }
      if (method === "POST" && url === "/api/command/next") {
        this.advanceLevel();
        return this.snapshot();
      }
      if (method === "POST" && url.startsWith("/api/input/")) {
        if (this.hasPendingWait()) {
          return this.snapshot();
        }
        this.applyInputName(decodeURIComponent(url.slice("/api/input/".length)));
        return this.snapshot();
      }
      if (method === "POST" && url.startsWith("/api/command/")) {
        if (this.hasPendingWait()) {
          return this.snapshot();
        }
        this.applyCommandName(decodeURIComponent(url.slice("/api/command/".length)));
        return this.snapshot();
      }
      throw new Error(`Unsupported exported HTML request: ${method} ${url}`);
    }

    async initializeRuntime() {
      await this.loadCoreRuntime();
      if (this.initializeSessionRuntime()) {
        this.initialized = true;
        return;
      }
      if (!this.gameHasSceneLevelOwner()) {
        this.activateLevel(this.selectedLevelIndex, true);
      }
      this.createScene(this.focusedScene);
      this.applySceneStartTransition();
      this.applyLevelStartTransition();
      this.initialized = true;
    }

    async ensureInitialized() {
      if (!this.initialized) {
        await this.initializationPromise;
      }
    }

    async loadCoreRuntime() {
      const version = String(this.data?.engineVersion || Date.now());
      const module = await window.PuzzleRuntimeWasmLoader.load(version);
      if (typeof module.WasmCompiledCoreRuntime === "function") {
        this.wasmModule = module;
        this.coreRuntime = new module.WasmCompiledCoreRuntime(JSON.stringify(this.data || {}));
        this.coreRuntimeStateHash = null;
        return;
      }
      if (typeof module.WasmCoreRuntime !== "function") {
        throw new Error("Puzzle core WASM runtime is unavailable.");
      }
      this.wasmModule = module;
      this.coreRuntime = new module.WasmCoreRuntime(this.data.source || "", this.data.puzzlePath || "game.puzzle");
      this.coreRuntimeStateHash = null;
    }

    grid2TransitionData() {
      if (this.compiledPlay?.model !== "grid2") {
        return null;
      }
      return Array.isArray(this.compiledPlay.transition) ? this.compiledPlay.transition : null;
    }

    transitionProgramLength(programKey, levelIndex = -1) {
      const transition = this.grid2TransitionData();
      if (transition) {
        const programs = Array.isArray(transition[4]) ? transition[4] : [];
        const levels = Array.isArray(transition[5]) ? transition[5] : [];
        const globalIndexes = {
          main: 0,
          run_rules_on_level_start: 0,
          level_start: 1,
          level_clear: 2,
          display_level_start: 3,
          display_level_clear: 4,
          display: 5,
        };
        if (programKey === "level_start_local" || programKey === "level_clear_local") {
          const index = Number.isFinite(levelIndex) ? Math.trunc(levelIndex) : -1;
          const local = index >= 0 ? levels[index] : null;
          const program = Array.isArray(local) ? local[programKey === "level_start_local" ? 0 : 1] : null;
          return Array.isArray(program) ? program.length : 0;
        }
        const program = programs[globalIndexes[programKey]];
        return Array.isArray(program) ? program.length : 0;
      }
      if (programKey === "main" || programKey === "run_rules_on_level_start") {
        return (this.engine.program || []).length;
      }
      if (programKey === "level_start") {
        return (this.engine.levelStartProgram || []).length;
      }
      if (programKey === "level_clear") {
        return (this.engine.levelClearProgram || []).length;
      }
      if (programKey === "display_level_start") {
        return (this.engine.displayLevelStartProgram || []).length;
      }
      if (programKey === "display_level_clear") {
        return (this.engine.displayLevelClearProgram || []).length;
      }
      if (programKey === "display") {
        return (this.engine.displayProgram || []).length;
      }
      if (programKey === "level_start_local") {
        return (this.data.levels[levelIndex]?.levelStartProgram || []).length;
      }
      if (programKey === "level_clear_local") {
        return (this.data.levels[levelIndex]?.levelClearProgram || []).length;
      }
      return 0;
    }

    hasTransitionProgram(programKey, levelIndex = -1) {
      return this.transitionProgramLength(programKey, levelIndex) > 0;
    }

    initializeSessionRuntime() {
      if (typeof this.wasmModule?.WasmStandaloneSession !== "function") {
        return false;
      }
      this.sessionRuntime = new this.wasmModule.WasmStandaloneSession(
        this.data.source || "",
        this.data.puzzlePath || "game.puzzle",
      );
      this.restoreSessionProgressSave();
      this.usesRustSession = true;
      return true;
    }

    sessionRequestJson(method, url) {
      const raw = this.sessionRuntime.request_json(method, url);
      const next = JSON.parse(raw);
      if (method === "POST") {
        if (url === "/api/command/clear_game_progress") {
          this.clearSessionProgressSave();
        } else if (url !== "/api/solve") {
          this.writeSessionProgressSave();
        }
      }
      return next;
    }

    snapshot() {
      if (this.usesRustSession && this.sessionRuntime) {
        return JSON.parse(this.sessionRuntime.snapshot());
      }
      const soundEvents = this.soundEvents.splice(0);
      const messageEvents = this.messageEvents.splice(0);
      const animationEvents = this.animationEvents.splice(0);
      const focusedPuzzle = this.scenePuzzleState(this.focusedScene);
      const presentation = focusedPuzzle
        ? this.presentationSnapshotForPuzzle(focusedPuzzle)
        : this.editorPresentationSnapshot();
      if (presentation?.scene) {
        presentation.scene.animationEvents = animationEvents;
      }
      return {
        game: {
          title: this.data.title,
          has_progress_save: this.hasProgressSaveData(),
        },
        sounds: this.data.sounds || { sfx: [], music: [] },
        soundEvents,
        messageEvents,
        animationEvents,
        level: this.levelContext(),
        levelIndex: this.levelIndex,
        levelCount: this.data.levels.length,
        screen: this.focusedScene,
        currentScene: this.focusedScene,
        focusedScreen: this.focusedScene,
        focusedScene: this.focusedScene,
        visibleScreens: [...this.visibleScenes],
        visibleScenes: [...this.visibleScenes],
        gameState: this.sessionValues,
        screenState: this.focusedSceneRuntime().values,
        sceneState: this.focusedSceneRuntime().values,
        screenPuzzles: Object.keys(this.focusedSceneRuntime().puzzles),
        scenePuzzles: Object.keys(this.focusedSceneRuntime().puzzles),
        scenePuzzleState: this.scenePuzzleRefs(),
        selectedLevelIndex: this.selectedLevelIndex,
        busy: this.hasPendingWait(),
        canUndo: this.undoStack.length > 0,
        canRedo: this.redoStack.length > 0,
        rawScene: presentation?.rawScene || null,
        scene: presentation?.scene || null,
        sceneLayers: this.sceneLayers(),
        inputs: this.data.inputs,
        clearedLevels: [...this.clearedLevels],
        levels: this.data.levels.map((level, index) => ({
          index,
          name: level.name,
          cleared: this.clearedLevels[index] === true,
          solved: this.clearedLevels[index] === true,
        })),
        scenes: this.data.scenes,
        screens: this.data.screens,
      };
    }

    presentationSnapshotForPuzzle(puzzle) {
      if (!puzzle) {
        return null;
      }
      return this.presentationSnapshotForState(puzzle.state, {
        levelIndex: puzzle.levelIndex,
      });
    }

    editorPresentationSnapshot() {
      if (!this.editorPreviewSceneEnabled || this.levelIndex === null || this.levelIndex === undefined || !this.state) {
        return null;
      }
      return this.presentationSnapshotForState(this.state, {
        rawState: this.currentLevel()?.initialState || this.state,
        levelIndex: this.levelIndex,
      });
    }

    presentationSnapshotForState(state, options = {}) {
      if (!state) {
        return null;
      }
      const levelIndex = options.levelIndex ?? this.levelIndex;
      const rawState = options.rawState || state;
      const displayState = options.materializeDisplay === false ? state : this.displayState(state);
      return {
        rawScene: this.sceneFromState(rawState, levelIndex),
        scene: this.sceneFromState(displayState, levelIndex),
      };
    }

    levelContext() {
      const level = this.currentLevel();
      if (!level || this.levelIndex === null || this.levelIndex === undefined) {
        return null;
      }
      return {
        index: this.levelIndex,
        number: this.levelIndex + 1,
        count: this.data.levels.length,
        name: level.name,
        label: level.name,
      };
    }

    applyInputName(inputName) {
      if (this.usesRustSession && this.sessionRuntime) {
        this.sessionRuntime.apply_input_name(inputName);
        this.writeSessionProgressSave();
        return;
      }
      const input = this.inputIdsByName.get(inputName);
      if (input === undefined) {
        throw new Error(`unknown input: ${inputName}`);
      }
      if (this.currentSceneAcceptsModelInput() || this.editorPreviewInputEnabled) {
        this.applyInput(input);
        return;
      }
      const previousInput = this.currentInput;
      this.currentInput = inputName;
      const ownsTurnSfx = this.beginTurnSfx();
      try {
        this.applyTurnCompletion([]);
      } finally {
        this.currentInput = previousInput;
        this.endTurnSfx(ownsTurnSfx);
      }
    }

    applyInput(input) {
      const previousInput = this.currentInput;
      this.currentInput = this.inputNamesById.get(input) ?? null;
      const ownsTurnSfx = this.beginTurnSfx();
      try {
        const result = this.applyModelInput(input);
        if (!result?.cancelled) {
          this.applyTurnCompletion(result?.commands || []);
        }
      } finally {
        this.currentInput = previousInput;
        this.endTurnSfx(ownsTurnSfx);
      }
    }

    applyModelInput(input) {
      const target = this.currentSceneDef()?.puzzleRule?.target;
      if (target) {
        return this.applyModelInputToTarget(target, input);
      }
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return { cancelled: false, commands: [] };
      }
      const state = this.cloneState(this.state);
      this.applyPersistentVars(state);
      const outcome = this.transitionOutcome(state, input);
      this.replaceStateIfChanged(outcome.state, {
        previousStateHandle: outcome.previousStateHandle,
        changed: outcome.changed,
      });
      this.syncCurrentLevelPuzzles();
      this.animationEvents.push(...(outcome.animations || []));
      return {
        cancelled: !!outcome.cancelled,
        commands: this.queueTransitionCommands(null, outcome.commands || []),
      };
    }

    applyModelInputToTarget(target, input) {
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        if (this.levelIndex === null || this.levelIndex === undefined) {
          return { cancelled: false, commands: [] };
        }
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionOutcome(state, input);
        this.replaceStateIfChanged(outcome.state, {
          previousStateHandle: outcome.previousStateHandle,
          changed: outcome.changed,
        });
        this.syncCurrentLevelPuzzles();
        this.animationEvents.push(...(outcome.animations || []));
        return {
          cancelled: !!outcome.cancelled,
          commands: this.queueTransitionCommands(null, outcome.commands || []),
        };
      }
      const { sceneName, puzzleName } = resolved;
      const initializer = this.scenePuzzleInitializer(sceneName, puzzleName);
      if (!initializer) {
        if (this.levelIndex === null || this.levelIndex === undefined) {
          return { cancelled: false, commands: [] };
        }
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionOutcome(state, input);
        this.replaceStateIfChanged(outcome.state, {
          previousStateHandle: outcome.previousStateHandle,
          changed: outcome.changed,
        });
        this.syncCurrentLevelPuzzles();
        this.animationEvents.push(...(outcome.animations || []));
        return {
          cancelled: !!outcome.cancelled,
          commands: this.queueTransitionCommands(null, outcome.commands || []),
        };
      }
      this.createScene(sceneName);
      const runtime = this.sceneStates.get(sceneName);
      const puzzle = runtime?.puzzles?.[puzzleName];
      if (!puzzle) {
        return { cancelled: false, handledFlow: false };
      }
      const state = this.cloneState(puzzle.state);
      this.applyPersistentVars(state);
      const outcome = this.transitionOutcome(state, input);
      const next = outcome.state;
      this.capturePersistentVars(next);
      this.applyPersistentVars(next);
      puzzle.state = next;
      if (initializer.initializer === "current_level" && sceneName === this.focusedScene) {
        this.replaceStateIfChanged(this.cloneState(next), { changed: outcome.changed });
      } else {
        this.syncPersistentVarsToStates();
      }
      this.animationEvents.push(...(outcome.animations || []));
      return {
        cancelled: !!outcome.cancelled,
        commands: this.queueTransitionCommands(target, outcome.commands || []),
      };
    }

    queueTransitionCommands(target, commands) {
      return (commands || []).map((command) => ({ target, command }));
    }

    applyTurnCompletion(commands) {
      const conditionEffect = this.conditionTransitionEffect();
      const forceClear = (commands || []).some((queued) => {
        const command = queued?.command;
        return command === "win" || command?.kind === "win";
      });
      const clearCommands = this.applyModelLevelClear(forceClear);
      this.resolveTurnCommands([...(commands || []), ...(clearCommands || [])], conditionEffect);
    }

    resolveTurnCommands(commands, conditionEffect) {
      const pendingNextLevel = { queued: false, target: null };
      const pendingAgain = { queued: false, target: null };
      const pendingRestart = { queued: false, target: null };
      const queue = commands || [];
      for (let index = 0; index < queue.length; index += 1) {
        const queued = queue[index];
        const command = queued?.command;
        if (command === "win" || command?.kind === "win") {
          continue;
        }
        if (command === "restart" || command?.kind === "restart") {
          if (!pendingRestart.queued) {
            pendingRestart.queued = true;
            pendingRestart.target = queued?.target ?? null;
          }
          continue;
        }
        if (command === "next_level" || command?.kind === "next_level") {
          this.queueNextLevel(pendingNextLevel, queued?.target ?? null);
          continue;
        }
        if (command === "again" || command?.kind === "again") {
          this.queueAgain(pendingAgain, queued?.target ?? null);
          continue;
        }
        if (command === "checkpoint" || command?.kind === "checkpoint") {
          this.saveCheckpoint(queued?.target ?? null);
          continue;
        }
        if (command === "clear_checkpoint" || command?.kind === "clear_checkpoint") {
          this.clearCheckpoint(queued?.target ?? null);
          continue;
        }
        if (command?.kind === "play_sfx") {
          this.emitTurnSfx(command.name);
          continue;
        }
        if (command?.kind === "wait") {
          const remaining = queue.slice(index + 1);
          if (remaining.length || conditionEffect) {
            this.queueEffectContinuation(command.milliseconds || command.ms || 0, remaining, conditionEffect);
          } else {
            this.queueWait(command.milliseconds || command.ms || 0);
          }
          return this.resolvePendingTurnCommands(pendingRestart, pendingNextLevel, pendingAgain, null);
        }
        if (command?.kind === "message") {
          this.messageEvents.push({
            kind: "message",
            text: this.resolveMessageText(command.text, command.literal),
          });
        }
      }
      this.resolvePendingTurnCommands(pendingRestart, pendingNextLevel, pendingAgain, conditionEffect);
    }

    resolvePendingTurnCommands(pendingRestart, pendingNextLevel, pendingAgain, conditionEffect) {
      if (pendingRestart.queued) {
        if (pendingRestart.target) {
          this.restartLevelTarget(pendingRestart.target);
        } else {
          this.restartLevel();
        }
        return;
      }
      if (conditionEffect) {
        this.applySceneEffectDuringTurn(conditionEffect, {}, pendingNextLevel);
      }
      if (pendingNextLevel.queued) {
        if (pendingNextLevel.target) {
          this.advanceLevelFromTarget(pendingNextLevel.target);
        } else {
          this.advanceLevel();
        }
      } else if (pendingAgain.queued) {
        this.applyAgainTurns(pendingAgain.target || null);
      }
    }

    queueEffectContinuation(milliseconds, commands, conditionEffect) {
      this.pendingWaits += 1;
      setTimeout(() => {
        try {
          this.resolveTurnCommands(commands, conditionEffect || null);
        } finally {
          this.pendingWaits = Math.max(0, this.pendingWaits - 1);
          this.notifyStateChanged();
        }
      }, Math.max(0, Number(milliseconds || 0)));
    }

    queueNextLevel(pendingNextLevel, target) {
      if (!pendingNextLevel.queued) {
        pendingNextLevel.queued = true;
        pendingNextLevel.target = target || null;
      }
    }

    queueAgain(pendingAgain, target) {
      if (!pendingAgain.queued) {
        pendingAgain.queued = true;
        pendingAgain.target = target || null;
      }
    }

    applyAgainTurns(target) {
      const token = (this.againRunToken || 0) + 1;
      this.againRunToken = token;
      this.scheduleAgainTurn(target, 0, token);
    }

    scheduleAgainTurn(target, count, token) {
      if (count >= this.maxAgainTurnsPerInput) {
        console.warn(`again turn limit reached (${this.maxAgainTurnsPerInput}); stopping automatic turns`);
        return;
      }
      this.pendingAgainTurns += 1;
      setTimeout(() => {
        try {
          if (token !== this.againRunToken) {
            return;
          }
          const result = this.runAgainTurn(target);
          if (!result?.continueAgain) {
            return;
          }
          this.scheduleAgainTurn(target, count + 1, token);
        } finally {
          this.pendingAgainTurns = Math.max(0, this.pendingAgainTurns - 1);
          this.notifyStateChanged();
        }
      }, Math.max(0, this.defaultAgainMs));
    }

    runAgainTurn(target) {
      const previousTurnSfx = this.beginSeparateTurnSfx();
      try {
        const result = target
          ? this.applyModelInputToTarget(target, 0)
          : this.applyModelInput(0);
        if (result?.cancelled) {
          return { continueAgain: false };
        }
        const commands = result?.commands || [];
        const hasAgain = commands.some((queued) => {
          const command = queued?.command;
          return command === "again" || command?.kind === "again";
        });
        this.applyTurnCompletion(commands.filter((queued) => {
          const command = queued?.command;
          return command !== "again" && command?.kind !== "again";
        }));
        return { continueAgain: hasAgain };
      } finally {
        this.endSeparateTurnSfx(previousTurnSfx);
      }
    }

    beginTurnSfx() {
      if (this.currentTurnSfx) {
        return false;
      }
      this.currentTurnSfx = new Set();
      return true;
    }

    endTurnSfx(owned) {
      if (owned) {
        this.currentTurnSfx = null;
      }
    }

    beginSeparateTurnSfx() {
      const previous = this.currentTurnSfx;
      this.currentTurnSfx = new Set();
      return previous;
    }

    endSeparateTurnSfx(previous) {
      this.currentTurnSfx = previous || null;
    }

    emitTurnSfx(name) {
      if (!this.currentTurnSfx) {
        this.soundEvents.push({ kind: "play_sfx", name });
        return;
      }
      if (this.currentTurnSfx.has(name)) {
        return;
      }
      this.currentTurnSfx.add(name);
      this.soundEvents.push({ kind: "play_sfx", name });
    }

    applyCommandName(command) {
      if (this.usesRustSession && this.sessionRuntime) {
        this.sessionRuntime.apply_command_name(command);
        if (command === "clear_game_progress") {
          this.clearSessionProgressSave();
        } else {
          this.writeSessionProgressSave();
        }
        return;
      }
      if (command === "undo") {
        this.undo();
        return;
      }
      if (command === "redo") {
        this.redo();
        return;
      }
      const runtimeEffect = this.parseRuntimeCommand(command) || this.parsePuzzleRuntimeCommand(command);
      if (runtimeEffect) {
        this.applySceneEffect(runtimeEffect, {});
        return;
      }
      if (this.applySceneTransition(command)) {
        return;
      }
      if (command === "restart") {
        this.restartLevel();
        return;
      }
      if (this.currentSceneHasLevelMenu() && this.applyLevelMenuCommand(command)) {
        return;
      }
      if (this.currentSceneAcceptsModelInput() && this.inputIdsByName.has(command)) {
        this.applyInputName(command);
        return;
      }
      this.applySceneInputName(command);
    }

    applySceneInputName(input) {
      const previousInput = this.currentInput;
      this.currentInput = input;
      try {
        if (this.applySceneTransition(input)) {
          return;
        }
        if (this.currentSceneAcceptsModelInput() && this.inputIdsByName.has(input)) {
          this.applyInputName(input);
          return;
        }
        this.applyTurnCompletion([]);
      } finally {
        this.currentInput = previousInput;
      }
    }

    applyComponentEffectName(effect) {
      if (this.currentSceneHasLevelMenu() && this.applyLevelMenuCommand(effect)) {
        return;
      }
    }

    transition(initialState, input) {
      return this.transitionOutcome(initialState, input).state;
    }

    transitionOutcome(initialState, input) {
      return this.transitionProgramOutcome(null, initialState, input, "main");
    }

    solveCurrentState(options = {}) {
      throw new Error("Built-in solver has been removed; use the current solver instead.");
    }

    async solveCurrentStateAsync(options = {}, onProgress = null, shouldCancel = null) {
      throw new Error("Built-in solver has been removed; use the current solver instead.");
    }

    transitionProgram(program, initialState, input, programKey = "custom", levelIndex = -1) {
      return this.transitionProgramOutcome(program, initialState, input, programKey, levelIndex).state;
    }

    transitionProgramOutcome(program, initialState, input, programKey = "custom", levelIndex = -1) {
      if (programKey === "custom") {
        throw new Error("Standalone runtime requires the WASM core runtime; JavaScript transition programs are unsupported.");
      }
      return this.coreTransitionProgramOutcome(programKey, levelIndex, initialState, input);
    }

    coreTransitionProgramOutcome(programKey, levelIndex, initialState, input) {
      if (!this.coreRuntime) {
        throw new Error("Puzzle core WASM runtime has not been initialized.");
      }
      if (typeof this.coreRuntime.transition_current_outcome === "function"
        && typeof this.coreRuntime.set_state === "function") {
        this.syncCoreRuntimeState(initialState);
        const raw = this.coreRuntime.transition_current_outcome(
          programKey,
          Number.isFinite(levelIndex) ? Math.trunc(levelIndex) : -1,
          Number(input || 0),
        );
        const outcome = JSON.parse(raw);
        const nextState = outcome.state || this.applyCoreOutcomeToState(initialState, outcome);
        this.coreRuntimeStateHash = this.coreHashKey(outcome.stateHashKey ?? outcome.stateHash);
        this.setCoreStateHash(nextState, this.coreRuntimeStateHash);
        return {
          state: nextState,
          previousStateHandle: Number.isFinite(Number(outcome.previousStateHandle))
            ? Number(outcome.previousStateHandle)
            : undefined,
          changed: outcome.changed === true,
          cancelled: outcome.cancelled === true,
          commands: this.commandsForCoreOutcome(outcome),
          animations: this.normalizeAnimationEvents(outcome.animationEvents),
        };
      }
      const raw = this.coreRuntime.transition_program_outcome(
        programKey,
        Number.isFinite(levelIndex) ? Math.trunc(levelIndex) : -1,
        JSON.stringify(initialState),
        Number(input || 0),
      );
      const outcome = JSON.parse(raw);
      return {
        state: outcome.state,
        previousStateHandle: undefined,
        changed: undefined,
        cancelled: outcome.cancelled === true,
        commands: this.commandsForCoreOutcome(outcome),
        animations: this.normalizeAnimationEvents(outcome.animationEvents),
      };
    }

    normalizeAnimationEvents(events) {
      return Array.isArray(events) ? events : [];
    }

    syncCoreRuntimeState(state) {
      const hash = this.coreStateHash(state);
      if (hash !== null && this.coreRuntimeStateHash !== null && this.coreRuntimeStateHash === hash) {
        return;
      }
      this.coreRuntime.set_state(JSON.stringify(state));
      this.coreRuntimeStateHash = typeof this.coreRuntime.current_state_hash === "function"
        ? this.coreHashKey(this.coreRuntime.current_state_hash())
        : null;
      this.setCoreStateHash(state, this.coreRuntimeStateHash);
    }

    applyCoreOutcomeToState(state, outcome) {
      this.clearScratch(state);
      if (Array.isArray(outcome.changedCells)) {
        for (const cell of outcome.changedCells) {
          this.applyChangedCell(state, cell);
        }
      }
      if (Array.isArray(outcome.globals)) {
        state.globals = [...outcome.globals];
      }
      if (Array.isArray(outcome.levelFiredRules)) {
        state.levelFiredRules = [...outcome.levelFiredRules];
      }
      return state;
    }

    applyChangedCell(state, cell) {
      const x = Math.trunc(Number(cell?.x) || 0);
      const y = Math.trunc(Number(cell?.y) || 0);
      if (x < 0 || y < 0 || x >= state.width || y >= state.height) {
        return;
      }
      this.clearCoreStateHash(state);
      const cellIndex = this.cellIndex(state, x, y);
      const start = cellIndex * state.layerCount;
      for (let layer = 0; layer < state.layerCount; layer += 1) {
        state.slots[start + layer] = 0;
      }
      for (const object of cell?.objects || []) {
        const objectId = Number(object || 0);
        const layer = this.objectLayers.get(objectId);
        if (layer === undefined || layer < 0 || layer >= state.layerCount) {
          continue;
        }
        state.slots[start + layer] = objectId;
      }
    }

    commandsForCoreOutcome(outcome) {
      const commands = [];
      for (const ruleId of outcome.firedRules || []) {
        commands.push(...this.ruleEffectCommands(ruleId));
      }
      if (!commands.length) {
        commands.push(...(outcome.commands || []));
      }
      return commands;
    }

    ruleEffectCommands(ruleId) {
      return [...(this.engine.ruleEffects?.[String(ruleId)] || this.engine.ruleEffects?.[ruleId] || [])];
    }

    ruleEmissionCommands(ruleId) {
      return (this.engine.ruleEmissions?.[String(ruleId)] || this.engine.ruleEmissions?.[ruleId] || [])
        .map((effect) => {
          if (effect.kind === "message") {
            return {
              kind: "message",
              text: effect.text,
              literal: effect.literal === true,
            };
          }
          if (effect.kind === "play_sfx") {
            return { kind: "play_sfx", name: effect.name };
          }
          if (effect.kind === "wait") {
            return { kind: "wait", milliseconds: effect.milliseconds ?? effect.ms ?? 0 };
          }
          return null;
        })
        .filter(Boolean);
    }

    completeComponentPlacements(state, rule, componentIndex, components) {
      if (componentIndex === rule.pattern.components.length) {
        return true;
      }
      const component = rule.pattern.components[componentIndex];
      for (const [x, y] of this.componentCandidateOrigins(state, component)) {
        const placement = this.componentPlacementAt(state, component, x, y);
        if (!placement) {
          continue;
        }
        components.push(placement);
        if (this.completeComponentPlacements(state, rule, componentIndex + 1, components)) {
          return true;
        }
        components.pop();
      }
      return false;
    }

    componentCandidateOrigins(state, component) {
      const anchor = this.componentAnchorCell(component);
      if (!anchor) {
        return this.allOrigins(state);
      }
      const [dx, dy] = this.resolveOffset(anchor.cell.offset, []);
      const layer = this.objectLayers.get(anchor.object);
      if (layer === undefined || layer >= state.layerCount) {
        return this.allOrigins(state);
      }
      const origins = [];
      for (let y = 0; y < state.height; y += 1) {
        for (let x = 0; x < state.width; x += 1) {
          if (state.slots[this.slotIndex(state, x, y, layer)] === anchor.object) {
            origins.push([x - dx, y - dy]);
          }
        }
      }
      return origins;
    }

    componentAnchorCell(component) {
      if (component.gapCount !== 0) {
        return null;
      }
      for (const cell of component.cells) {
        if (cell.offset.kind === "fixed" && cell.requireObjects.length) {
          return { cell, object: cell.requireObjects[0] };
        }
      }
      return null;
    }

    allOrigins(state) {
      const origins = [];
      for (let y = 0; y < state.height; y += 1) {
        for (let x = 0; x < state.width; x += 1) {
          origins.push([x, y]);
        }
      }
      return origins;
    }

    componentPlacementAt(state, component, originX, originY) {
      if (component.gapCount === 0) {
        const gaps = [];
        return this.componentMatchesWithGaps(state, component, originX, originY, gaps)
          ? { originX, originY, gaps }
          : null;
      }
      const maxGap = Math.max(state.width, state.height);
      for (let totalGap = 0; totalGap <= maxGap * component.gapCount; totalGap += 1) {
        const gaps = [];
        if (this.findGapAssignment(state, component, originX, originY, maxGap, totalGap, gaps)) {
          return { originX, originY, gaps };
        }
      }
      return null;
    }

    findGapAssignment(state, component, originX, originY, maxGap, remainingTotal, gaps) {
      if (gaps.length === component.gapCount) {
        return remainingTotal === 0
          && this.componentMatchesWithGaps(state, component, originX, originY, gaps);
      }
      for (let gap = 0; gap <= Math.min(maxGap, remainingTotal); gap += 1) {
        gaps.push(gap);
        if (this.findGapAssignment(state, component, originX, originY, maxGap, remainingTotal - gap, gaps)) {
          return true;
        }
        gaps.pop();
      }
      return false;
    }

    componentMatchesWithGaps(state, component, originX, originY, gaps) {
      return component.cells.every((cell) => this.matchCell(state, originX, originY, gaps, cell));
    }

    matchCell(state, originX, originY, gaps, cell) {
      const [dx, dy] = this.resolveOffset(cell.offset, gaps);
      const x = originX + dx;
      const y = originY + dy;
      if (x < 0 || y < 0 || x >= state.width || y >= state.height) {
        return false;
      }
      return cell.requireObjects.every((object) => this.hasObject(state, x, y, object))
        && cell.forbidObjects.every((object) => !this.hasObject(state, x, y, object))
        && (cell.requireScratch || []).every((attr) => this.hasScratchPattern(state, x, y, attr))
        && (cell.forbidScratch || []).every((attr) => !this.hasScratchPattern(state, x, y, attr));
    }

    resolveOffset(offset, gaps) {
      if (offset.kind === "fixed") {
        return [offset.dx, offset.dy];
      }
      let dx = offset.baseDx;
      let dy = offset.baseDy;
      for (const term of offset.gapTerms) {
        const gap = gaps[term.gapIndex];
        dx += term.dx * gap;
        dy += term.dy * gap;
      }
      return [dx, dy];
    }

    hasAllObjects(state, x, y, objects) {
      return (objects || []).every((object) => this.hasObject(state, x, y, object));
    }

    evalQuery(state, queryId, input = 0) {
      return this.evalQueryKind(state, this.queriesById.get(queryId), input);
    }

    evalQueryKind(state, kind, input = 0) {
      if (!kind) {
        return 0;
      }
      if (kind.kind === "count_objects") {
        return kind.objects.reduce((sum, object) => sum + this.objectCount(state, object), 0);
      }
      if (kind.kind === "exists_objects") {
        return kind.objects.some((object) => this.objectCount(state, object) > 0) ? 1 : 0;
      }
      if (kind.kind === "none_objects") {
        return kind.objects.some((object) => this.objectCount(state, object) > 0) ? 0 : 1;
      }
      const patterns = kind.patterns || [];
      if (kind.kind === "count_matches") {
        return patterns.reduce((sum, entry) => sum + this.countPatternMatches(state, entry.pattern || entry), 0);
      }
      if (kind.kind === "exists_matches") {
        return patterns.some((entry) => this.hasPatternMatch(state, entry.pattern || entry)) ? 1 : 0;
      }
      if (kind.kind === "none_matches") {
        return patterns.some((entry) => this.hasPatternMatch(state, entry.pattern || entry)) ? 0 : 1;
      }
      if (kind.kind === "count_input_matches") {
        return patterns
          .filter((entry) => entry.input === input)
          .reduce((sum, entry) => sum + this.countPatternMatches(state, entry.pattern || entry), 0);
      }
      if (kind.kind === "exists_input_matches") {
        return patterns.some((entry) => entry.input === input && this.hasPatternMatch(state, entry.pattern || entry)) ? 1 : 0;
      }
      if (kind.kind === "none_input_matches") {
        return patterns.some((entry) => entry.input === input && this.hasPatternMatch(state, entry.pattern || entry)) ? 0 : 1;
      }
      return 0;
    }

    hasPatternMatch(state, pattern) {
      const rule = {
        id: 0,
        guards: [],
        application: "once",
        pattern,
        writes: [],
        effects: [],
      };
      for (const [x, y] of this.componentCandidateOrigins(state, pattern.components[0])) {
        const first = this.componentPlacementAt(state, pattern.components[0], x, y);
        if (!first) {
          continue;
        }
        const components = [first];
        if (this.completeComponentPlacements(state, rule, 1, components)) {
          return true;
        }
      }
      return false;
    }

    countPatternMatches(state, pattern) {
      const rule = {
        id: 0,
        guards: [],
        application: "once",
        pattern,
        writes: [],
        effects: [],
      };
      let count = 0;
      for (const [x, y] of this.componentCandidateOrigins(state, pattern.components[0])) {
        const first = this.componentPlacementAt(state, pattern.components[0], x, y);
        if (!first) {
          continue;
        }
        const components = [first];
        if (this.completeComponentPlacements(state, rule, 1, components)) {
          count += 1;
        }
      }
      return count;
    }

    compare(left, op, right) {
      if (op === "eq") return left === right;
      if (op === "not_eq") return left !== right;
      if (op === "greater") return left > right;
      if (op === "greater_eq") return left >= right;
      if (op === "less") return left < right;
      if (op === "less_eq") return left <= right;
      return false;
    }

    isGoalComplete(state) {
      return this.data.goal ? this.evalGoalExpr(state, this.data.goal.expr) : false;
    }

    isLoseComplete(state) {
      return this.data.lose ? this.evalGoalExpr(state, this.data.lose.expr) : false;
    }

    isConditionTrue(name, state) {
      const condition = this.data.conditions?.[this.conditionName(name)];
      return condition ? this.evalGoalExpr(state, condition.expr) : false;
    }

    isGlobalTruthy(name, state) {
      const global = this.globalIdsByName.get(this.conditionName(name));
      return global !== undefined && (state.globals?.[global] ?? 0) !== 0;
    }

    evalGoalExpr(state, expr) {
      if (expr.kind === "all") {
        return expr.exprs.every((child) => this.evalGoalExpr(state, child));
      }
      if (expr.kind === "any") {
        return expr.exprs.some((child) => this.evalGoalExpr(state, child));
      }
      return this.compare(this.evalGoalValue(state, expr.value), expr.op, expr.expected);
    }

    evalGoalValue(state, value) {
      if (value.kind === "global") {
        return state.globals[value.global] ?? 0;
      }
      if (value.kind === "query") {
        return this.evalQuery(state, value.query);
      }
      if (value.kind === "query_value") {
        return this.evalQueryKind(state, value.queryKind);
      }
      return 0;
    }

    applySceneTransition(command) {
      const screen = this.currentSceneDef();
      if (!screen) {
        return false;
      }
      const [commandName, commandPayload] = this.splitCommandValue(command);
      for (const transition of screen.transitions || []) {
        if (!transition.pattern || transition.pattern.name !== commandName) {
          continue;
        }
        const bindings = {};
        if (transition.pattern.payload && commandPayload !== undefined) {
          bindings[transition.pattern.payload] = commandPayload;
        } else if (transition.pattern.payload || commandPayload !== undefined) {
          continue;
        }
        this.applySceneEffect(transition.effect, bindings);
        this.applyTurnCompletion([]);
        return true;
      }
      return false;
    }

    applyConditionTransitions() {
      this.applyTurnCompletion([]);
    }

    conditionTransitionEffect() {
      const screen = this.currentSceneDef();
      const transition = (screen?.transitions || []).find((candidate) =>
        candidate.condition && this.isSceneConditionTrue(candidate.condition),
      );
      return transition?.effect || null;
    }

    applyModelLevelClear(forceClear = false) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return [];
      }
      if (forceClear || this.isGoalComplete(this.state)) {
        this.markCurrentLevelCleared();
        return this.applyLevelClearHook(forceClear);
      }
      return [];
    }

    applySceneEffectDuringTurn(effect, bindings, pendingNextLevel) {
      if (!effect) {
        return;
      }
      if (!this.sceneEffectContainsPuzzleNextLevel(effect)) {
        this.applySceneEffect(effect, bindings);
        return;
      }
      if (effect.kind === "puzzle_next_level") {
        this.queueNextLevel(pendingNextLevel, effect.target);
        return;
      }
      if (effect.kind === "conditional") {
        if (this.isSceneConditionTrue(effect.condition)) {
          this.applySceneEffectDuringTurn(effect.effect?.effect || effect.effect, bindings, pendingNextLevel);
        }
        return;
      }
      if (effect.kind === "sequence") {
        for (const child of effect.effects || []) {
          this.applySceneEffectDuringTurn(child?.effect || child, bindings, pendingNextLevel);
        }
        return;
      }
      this.applySceneEffect(effect, bindings);
    }

    sceneEffectContainsPuzzleNextLevel(effect) {
      if (!effect) {
        return false;
      }
      if (effect.kind === "puzzle_next_level") {
        return true;
      }
      if (effect.kind === "conditional") {
        return this.sceneEffectContainsPuzzleNextLevel(effect.effect?.effect || effect.effect);
      }
      if (effect.kind === "sequence") {
        return (effect.effects || []).some((child) =>
          this.sceneEffectContainsPuzzleNextLevel(child?.effect || child),
        );
      }
      return false;
    }

    applyLevelClearHook(forceClear = false) {
      const hasProgram = this.hasTransitionProgram("level_clear");
      const hasLevelProgram = this.hasTransitionProgram("level_clear_local", this.levelIndex);
      const hasDisplayProgram = this.hasTransitionProgram("display_level_clear");
      if ((!hasProgram && !hasLevelProgram && !hasDisplayProgram) || (!forceClear && !this.isGoalComplete(this.state))) {
        return [];
      }
      const commands = [];
      if (hasProgram) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionProgramOutcome(null, state, 0, "level_clear");
        this.state = this.cloneState(outcome.state);
        this.capturePersistentVars(this.state);
        if (!outcome.cancelled) {
          commands.push(...this.queueTransitionCommands(null, outcome.commands || []));
        }
      }
      if (hasLevelProgram) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionProgramOutcome(null, state, 0, "level_clear_local", this.levelIndex);
        this.state = this.cloneState(outcome.state);
        this.capturePersistentVars(this.state);
        if (!outcome.cancelled) {
          commands.push(...this.queueTransitionCommands(null, outcome.commands || []));
        }
      }
      if (hasDisplayProgram) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        this.state = this.materializeDisplayProgram(state, "display_level_clear");
        this.capturePersistentVars(this.state);
      }
      this.syncPersistentVarsToStates();
      this.syncCurrentLevelPuzzles();
      return commands;
    }

    isSceneConditionTrue(condition) {
      return String(condition || "")
        .split(" and ")
        .every((part) => this.isSceneConditionAtomTrue(part.trim()));
    }

    isSceneConditionAtomTrue(condition) {
      const equalMatch = String(condition || "").match(/^(.+?)\s*==\s*(.+)$/);
      if (equalMatch) {
        const left = this.sceneConditionValue(equalMatch[1].trim());
        const right = this.sceneConditionValue(equalMatch[2].trim());
        return left !== undefined && right !== undefined && left === right;
      }
      const notEqualMatch = String(condition || "").match(/^(.+?)\s*!=\s*(.+)$/);
      if (notEqualMatch) {
        const left = this.sceneConditionValue(notEqualMatch[1].trim());
        const right = this.sceneConditionValue(notEqualMatch[2].trim());
        return left !== undefined && right !== undefined && left !== right;
      }
      const levelValue = this.levelPathValue(condition);
      if (typeof levelValue === "boolean") {
        return levelValue;
      }
      const resolved = this.conditionStateAndName(condition);
      if (!resolved) {
        return false;
      }
      return this.isConditionTrue(resolved.name, resolved.state)
        || this.isGlobalTruthy(resolved.name, resolved.state);
    }

    sceneConditionValue(value) {
      if (value === "input") {
        return this.currentInput;
      }
      if (value === "true" || value === "false") {
        return value;
      }
      const levelValue = this.levelPathValue(value);
      if (levelValue !== undefined && levelValue !== null && typeof levelValue !== "object") {
        return String(levelValue);
      }
      if (/^-?\d+$/.test(String(value))) {
        return String(Number(value));
      }
      const quoted = String(value).match(/^"(.*)"$/);
      if (quoted) {
        return quoted[1].replace(/\\"/g, "\"");
      }
      if (/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(String(value))) {
        return String(value);
      }
      return undefined;
    }

    conditionStateAndName(condition) {
      const parts = String(condition || "").split(".").filter(Boolean);
      if (parts.length === 2) {
        const puzzle = this.focusedSceneRuntime().puzzles?.[parts[0]];
        if (puzzle) {
          return { state: puzzle.state, name: parts[1] };
        }
      }
      if (parts.length === 3) {
        const puzzle = this.sceneStates.get(parts[0])?.puzzles?.[parts[1]];
        if (puzzle) {
          return { state: puzzle.state, name: parts[2] };
        }
      }
      if (this.levelIndex !== null && this.levelIndex !== undefined) {
        return { state: this.state, name: this.conditionName(condition) };
      }
      return null;
    }

    applySceneEffect(effect, bindings) {
      if (!effect) {
        return;
      }
      if (effect.kind === "input") {
        this.applySceneInputName(effect.name);
      } else if (effect.kind === "component_effect") {
        this.applyComponentEffectName(effect.name);
      } else if (effect.kind === "command") {
        this.applyCommandName(effect.name);
      } else if (effect.kind === "message") {
        const text = this.evalEffectString(effect.text, bindings);
        if (text !== undefined) {
          this.messageEvents.push({ kind: "message", text });
        }
      } else if (effect.kind === "wait") {
        return Number(effect.milliseconds || effect.ms || 0);
      } else if (effect.kind === "conditional") {
        if (this.isSceneConditionTrue(effect.condition)) {
          return this.applySceneEffect(effect.effect?.effect || effect.effect, bindings);
        }
      } else if (effect.kind === "play_sfx") {
        this.emitTurnSfx(effect.name);
      } else if (effect.kind === "play_music") {
        this.soundEvents.push({ kind: "play_music", name: effect.name });
      } else if (effect.kind === "pause_music") {
        this.soundEvents.push({ kind: "pause_music", name: effect.name ?? null });
      } else if (effect.kind === "resume_music") {
        this.soundEvents.push({ kind: "resume_music", name: effect.name ?? null });
      } else if (effect.kind === "stop_music") {
        this.soundEvents.push({ kind: "stop_music", name: effect.name ?? null });
      } else if (effect.kind === "goto") {
        this.applySceneParams(effect.scene || effect.screen, effect.params || [], bindings);
        this.gotoScene(effect.scene || effect.screen);
      } else if (effect.kind === "enter") {
        this.applySceneParams(effect.scene || effect.screen, effect.params || [], bindings);
        this.enterScene(effect.scene || effect.screen);
      } else if (effect.kind === "back") {
        this.backOrInitial();
      } else if (effect.kind === "create") {
        this.createScene(effect.scene || effect.screen);
      } else if (effect.kind === "reset") {
        const name = effect.scene || effect.screen;
        if (this.sceneDef(name)) {
          this.resetSceneState(name);
        } else if (!this.resetPersistentVar(name)) {
          this.resetPuzzleState(name);
        }
      } else if (effect.kind === "clear_undo_history" || effect.kind === "clear_history") {
        this.clearUndoHistory();
      } else if (effect.kind === "clear_game_progress") {
        this.clearGameProgress();
      } else if (effect.kind === "set_current_level") {
        this.setCurrentLevelProgress(effect.level, bindings);
      } else if (effect.kind === "clear_current_level") {
        this.clearCurrentLevelProgress();
      } else if (effect.kind === "set_level_cleared") {
        this.setLevelClearedProgress(effect.level, effect.cleared === true, bindings);
      } else if (effect.kind === "reset_persistent_vars") {
        this.resetPersistentVars();
      } else if (effect.kind === "delete") {
        this.deleteScene(effect.scene || effect.screen);
      } else if (effect.kind === "show") {
        this.showScene(effect.scene || effect.screen);
      } else if (effect.kind === "hide") {
        this.hideScene(effect.scene || effect.screen);
      } else if (effect.kind === "toggle") {
        this.toggleScene(effect.scene || effect.screen);
      } else if (effect.kind === "focus") {
        this.focusScene(effect.scene || effect.screen);
      } else if (effect.kind === "start_level") {
        this.startLevelScene(effect.scene || effect.screen, effect.scope ?? null);
      } else if (effect.kind === "continue_level") {
        this.continueLevelScene(effect.scene || effect.screen, effect.scope ?? null);
      } else if (effect.kind === "puzzle_next_level") {
        this.advanceLevelFromTarget(effect.target);
      } else if (effect.kind === "puzzle_previous_level") {
        this.previousLevelFromTarget(effect.target);
      } else if (effect.kind === "puzzle_goto_level") {
        this.gotoLevelTarget(effect.target, effect.level, bindings);
      } else if (effect.kind === "puzzle_reset") {
        this.restartLevelTarget(effect.target);
      } else if (effect.kind === "puzzle_load") {
        this.loadPuzzleState(effect.target, effect.source, bindings);
      } else if (effect.kind === "apply") {
        const inputName = this.evalEffectString(effect.args?.[0], bindings) ?? effect.rule;
        const input = this.inputIdsByName.get(inputName);
        if (input !== undefined) {
          if (effect.target) {
            this.applyModelInputToTarget(effect.target, input);
          } else {
            this.applyInput(input);
          }
        }
      } else if (effect.kind === "copy") {
        this.copyPuzzleState(effect.source, effect.target);
      } else if (effect.kind === "sequence") {
        this.applySceneEffectSequence(effect.effects || [], bindings, 0);
      }
    }

    applySceneEffectSequence(effects, bindings, start) {
      for (let index = start; index < effects.length; index += 1) {
        const child = effects[index]?.effect || effects[index];
        const waitMs = this.applySceneEffect(child, bindings);
        if (Number.isFinite(waitMs)) {
          this.pendingWaits += 1;
          setTimeout(() => {
            try {
              this.applySceneEffectSequence(effects, bindings, index + 1);
            } finally {
              this.pendingWaits = Math.max(0, this.pendingWaits - 1);
              this.notifyStateChanged();
            }
          }, Math.max(0, waitMs));
          return;
        }
      }
    }

    queueWait(milliseconds) {
      this.pendingWaits += 1;
      setTimeout(() => {
        this.pendingWaits = Math.max(0, this.pendingWaits - 1);
        this.notifyStateChanged();
      }, Math.max(0, Number(milliseconds || 0)));
    }

    hasPendingWait() {
      return (this.pendingWaits || 0) > 0 || (this.pendingAgainTurns || 0) > 0;
    }

    notifyStateChanged() {
      window.dispatchEvent(new CustomEvent("PuzzleStandaloneStateChanged"));
    }

    applySceneParams(sceneName, params, bindings) {
      let levelChanged = false;
      for (const param of params) {
        const value = this.evalEffectValue(param.value, bindings);
        if (value === undefined) {
          continue;
        }
        const index = this.levelIndexFromValue(value);
        if (this.isLevelRef(value) && index !== undefined && this.sceneAcceptsLevel(sceneName, index)) {
          this.activateLevel(index, true);
          this.undoStack = [];
          this.redoStack = [];
          levelChanged = true;
        }
        this.createScene(sceneName);
        const variable = (this.sceneDef(sceneName)?.state?.variables || []).find((entry) => entry.name === param.name);
        if (variable && variable.mutable === false) {
          continue;
        }
        this.sceneStates.get(sceneName).values[param.name] = value;
      }
      if (levelChanged) {
        this.syncCurrentLevelPuzzles(sceneName);
      }
    }

    evalEffectString(expr, bindings) {
      const value = this.evalEffectValue(expr, bindings);
      return value === undefined ? undefined : this.sceneValueStringValue(value);
    }

    evalEffectValue(expr, bindings) {
      if (!expr) {
        return undefined;
      }
      if (expr.kind === "bool" || expr.kind === "int") {
        return expr.value;
      }
      if (expr.kind === "text") {
        return expr.value || "";
      }
      if (expr.kind === "path") {
        const parts = String(expr.path || "").split(".").filter(Boolean);
        if (parts.length === 1) {
          const bound = bindings[parts[0]];
          if (bound !== undefined) {
            return this.levelValueFromAtom(bound);
          }
          if (parts[0] === "level" && this.levelIndex !== null && this.levelIndex !== undefined) {
            return this.levelRef(this.focusedScene, this.levelIndex);
          }
          return this.sceneValue(parts[0])
            ?? parts[0];
        }
        if (parts.length === 2) {
          const receiver = this.evalEffectValue({ kind: "path", path: parts[0] }, bindings);
          return this.sceneValueField(receiver, parts[1]);
        }
        const levelValue = this.levelPathValue(expr.path);
        if (levelValue !== undefined && levelValue !== null) {
          return levelValue;
        }
        return parts.join(".");
      }
      if (expr.kind === "call" && expr.name === "next" && expr.args?.length === 1) {
        const level = this.evalEffectValue(expr.args[0], bindings);
        const index = this.levelIndexFromValue(level);
        return index === undefined ? undefined : this.levelRef(this.focusedScene, Math.min(index + 1, this.data.levels.length - 1));
      }
      return undefined;
    }

    resolveMessageText(text, literal) {
      if (literal) {
        return String(text ?? "");
      }
      return this.sceneValueString(text) ?? String(text ?? "");
    }

    sceneValueString(name) {
      const value = this.sceneValue(name);
      return value === undefined ? undefined : this.sceneValueStringValue(value);
    }

    sceneValue(name) {
      const value = this.focusedSceneRuntime()?.values?.[name] ?? this.sessionValues?.[name];
      if (value === undefined || value === null) {
        return undefined;
      }
      return value;
    }

    sceneValueStringValue(value) {
      if (this.isLevelRef(value)) {
        return String(value.index);
      }
      return String(value);
    }

    sceneValueField(value, field) {
      if (this.isLevelRef(value)) {
        return value[field];
      }
      return undefined;
    }

    isLevelRef(value) {
      return Boolean(value && typeof value === "object" && value.kind === "level");
    }

    restartLevel() {
      const level = this.currentLevel();
      if (!level) {
        return;
      }
      if (this.levelCheckpointState) {
        const next = this.cloneState(this.levelCheckpointState);
        this.applyPersistentVars(next);
        this.replaceStateIfChanged(next);
        this.syncCurrentLevelPuzzles();
        return;
      }
      const next = this.cloneState(level.initialState);
      this.applyPersistentVars(next);
      this.replaceStateIfChanged(next);
      this.applyModelLevelStart(true);
      this.applyLevelStartTransition();
      this.syncCurrentLevelPuzzles();
    }

    materializeLevelStart(state) {
      const outcome = this.levelStartOutcome(state);
      let next = outcome ? this.cloneState(outcome.state) : this.cloneState(state);
      if (this.hasTransitionProgram("display_level_start", this.levelIndex)) {
        next = this.materializeDisplayProgram(
          next,
          "display_level_start",
          this.levelIndex,
        );
      }
      return next;
    }

    levelStartOutcome(state, levelIndex = this.levelIndex) {
      if (levelIndex === null || levelIndex === undefined) {
        return null;
      }
      const outcome = {
        state: this.cloneState(state),
        cancelled: false,
        commands: [],
      };
      let ran = false;
      if (this.hasTransitionProgram("level_start", levelIndex)) {
        const next = this.transitionProgramOutcome(null, outcome.state, 0, "level_start", levelIndex);
        outcome.state = this.cloneState(next.state);
        outcome.cancelled = outcome.cancelled || !!next.cancelled;
        outcome.commands.push(...(next.commands || []));
        ran = true;
      } else if (this.engine.runRulesOnLevelStart) {
        const next = this.transitionProgramOutcome(null, outcome.state, 0, "run_rules_on_level_start", levelIndex);
        outcome.state = this.cloneState(next.state);
        outcome.cancelled = outcome.cancelled || !!next.cancelled;
        outcome.commands.push(...(next.commands || []));
        ran = true;
      }
      if (!outcome.cancelled && this.hasTransitionProgram("level_start_local", levelIndex)) {
        const next = this.transitionProgramOutcome(null, outcome.state, 0, "level_start_local", levelIndex);
        outcome.state = this.cloneState(next.state);
        outcome.cancelled = outcome.cancelled || !!next.cancelled;
        outcome.commands.push(...(next.commands || []));
        ran = true;
      }
      return ran ? outcome : null;
    }

    applyModelLevelStart(emitEvents = true) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      const state = this.cloneState(this.state);
      this.applyPersistentVars(state);
      const outcome = this.levelStartOutcome(state);
      if (!outcome) {
        this.state = state;
        this.syncPersistentVarsToStates();
        return;
      }
      this.state = this.cloneState(outcome.state);
      this.capturePersistentVars(this.state);
      this.applyPersistentVars(this.state);
      this.syncPersistentVarsToStates();
      if (emitEvents && !outcome.cancelled) {
        this.resolveTurnCommands(this.queueTransitionCommands(null, outcome.commands || []), null);
      }
    }

    activateLevel(levelIndex, emitEvents = true) {
      if (levelIndex < 0 || levelIndex >= this.data.levels.length) {
        return false;
      }
      this.levelIndex = levelIndex;
      this.selectedLevelIndex = levelIndex;
      this.levelCheckpointState = null;
      this.state = this.cloneState(this.data.levels[levelIndex].initialState);
      this.applyPersistentVars(this.state);
      this.applyModelLevelStart(emitEvents);
      this.writeProgressSave();
      return true;
    }

    materializedLevelInitialState(levelIndex) {
      const level = this.data.levels[levelIndex];
      if (!level) {
        return this.neutralState();
      }
      let state = this.cloneState(level.initialState);
      this.applyPersistentVars(state);
      const outcome = this.levelStartOutcome(state, levelIndex);
      let next = outcome ? this.cloneState(outcome.state) : this.cloneState(state);
      if (this.hasTransitionProgram("display_level_start", levelIndex)) {
        next = this.materializeDisplayProgram(
          next,
          "display_level_start",
          levelIndex,
        );
      }
      return next;
    }

    displayState(state) {
      if (!this.hasTransitionProgram("display")) {
        return state;
      }
      return this.materializeDisplayProgram(state, "display");
    }

    materializeDisplayProgram(state, programKey, levelIndex = -1) {
      const base = this.cloneState(state);
      return this.transitionProgram(null, base, 0, programKey, levelIndex);
    }

    setCurrentState(state, options = {}) {
      this.editorPreviewSceneEnabled = true;
      this.editorPreviewInputEnabled = options.acceptModelInput === true;
      if (options.levelIndex !== undefined) {
        this.levelIndex = this.clampLevelIndex(options.levelIndex);
        this.selectedLevelIndex = this.levelIndex;
      }
      const level = this.currentLevel();
      if (level) {
        level.initialState = this.cloneState(state);
        this.levelCheckpointState = null;
        if (Array.isArray(options.regions)) {
          level.regions = options.regions.map((region, index) => ({
            index: Number.isInteger(region?.index) ? region.index : index,
            x: Math.max(0, Math.trunc(Number(region?.x) || 0)),
            y: Math.max(0, Math.trunc(Number(region?.y) || 0)),
            width: Math.max(0, Math.trunc(Number(region?.width) || 0)),
            height: Math.max(0, Math.trunc(Number(region?.height) || 0)),
          }));
        }
      }
      this.state = options.materializeLevelStart
        ? this.materializeLevelStart(state)
        : this.cloneState(state);
      if ((options.materializeDisplay || options.materializeTurnStart) && options.acceptModelInput !== true) {
        this.state = this.displayState(this.state);
      }
      this.capturePersistentVars(this.state);
      this.applyPersistentVars(this.state);
      this.undoStack = [];
      this.redoStack = [];
      this.syncCurrentLevelPuzzles();
    }

    advanceLevel() {
      const scene = this.isLevelScene(this.focusedScene) ? this.focusedScene : this.initialLevelSceneName();
      this.advanceLevelInScene(scene);
    }

    advanceLevelFromTarget(target) {
      const scene = this.levelSceneFromTarget(target);
      this.advanceLevelInScene(scene);
    }

    previousLevelFromTarget(target) {
      const scene = this.levelSceneFromTarget(target);
      this.previousLevelInScene(scene);
    }

    restartLevelTarget(target) {
      if (this.isLevelScene(target)) {
        this.restartLevelInScene(target);
        return;
      }
      this.resetPuzzleState(target);
    }

    levelSceneFromTarget(target) {
      if (this.isLevelScene(target)) {
        return target;
      }
      const resolved = this.resolvePuzzleTarget(target);
      return resolved && this.isLevelScene(resolved.sceneName) ? resolved.sceneName : this.focusedScene;
    }

    restartLevelInScene(sceneName) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      this.activateLevel(this.levelIndex, false);
      this.undoStack = [];
      this.redoStack = [];
      this.startScene(sceneName);
      this.syncCurrentLevelPuzzles();
      this.selectedLevelIndex = this.levelIndex;
    }

    advanceLevelInScene(sceneName) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      const indices = this.sceneLevelIndices(sceneName);
      const position = indices.indexOf(this.levelIndex);
      const nextLevel = position >= 0 ? indices[position + 1] : undefined;
      if (nextLevel === undefined) {
        return;
      }
      this.activateLevel(nextLevel, true);
      this.undoStack = [];
      this.redoStack = [];
      this.startScene(sceneName);
      this.syncCurrentLevelPuzzles();
      this.selectedLevelIndex = this.levelIndex;
    }

    previousLevelInScene(sceneName) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      const indices = this.sceneLevelIndices(sceneName);
      const position = indices.indexOf(this.levelIndex);
      const previousLevel = position > 0 ? indices[position - 1] : undefined;
      if (previousLevel === undefined) {
        return;
      }
      this.activateLevel(previousLevel, true);
      this.undoStack = [];
      this.redoStack = [];
      this.startScene(sceneName);
      this.syncCurrentLevelPuzzles();
      this.selectedLevelIndex = this.levelIndex;
    }

    startLevel(levelIndex) {
      if (levelIndex < 0 || levelIndex >= this.data.levels.length) {
        return;
      }
      this.activateLevel(levelIndex, true);
      this.undoStack = [];
      this.redoStack = [];
      this.startScene(this.initialLevelSceneName());
      this.syncCurrentLevelPuzzles();
    }

    gotoLevelTarget(target, level, bindings = {}) {
      const value = this.evalEffectString(level, bindings);
      const index = this.levelIndexFromValue(value);
      if (index === undefined) {
        return;
      }
      if (this.sceneDef(target)) {
        if (!this.sceneAcceptsLevel(target, index)) {
          return;
        }
        this.activateLevel(index, true);
        this.undoStack = [];
        this.redoStack = [];
        this.gotoScene(target);
        this.syncCurrentLevelPuzzles();
        return;
      }
      this.loadPuzzleState(target, value, bindings);
    }

    startLevelScene(sceneName, scope = null) {
      const index = this.firstLevelIndexForScene(sceneName, scope);
      if (index === undefined) {
        return;
      }
      this.activateLevel(index, true);
      this.undoStack = [];
      this.redoStack = [];
      this.gotoScene(sceneName);
      this.syncCurrentLevelPuzzles();
    }

    continueLevelScene(sceneName, scope = null) {
      const index = this.levelIndexForSceneContinue(sceneName, scope);
      if (index === undefined) {
        return;
      }
      this.activateLevel(index, true);
      this.undoStack = [];
      this.redoStack = [];
      this.gotoScene(sceneName);
      this.syncCurrentLevelPuzzles();
    }

    undo() {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      const previous = this.undoStack.pop();
      if (!previous) {
        return;
      }
      this.redoStack.push(this.saveCurrentHistoryEntry());
      this.state = this.restoreHistoryEntry(previous);
      this.applyPersistentVars(this.state);
      this.syncCurrentLevelPuzzles();
    }

    redo() {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      const next = this.redoStack.pop();
      if (!next) {
        return;
      }
      this.undoStack.push(this.saveCurrentHistoryEntry());
      this.state = this.restoreHistoryEntry(next);
      this.applyPersistentVars(this.state);
      this.syncCurrentLevelPuzzles();
    }

    replaceStateIfChanged(next, options = {}) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        this.state = this.cloneState(next);
        return;
      }
      this.capturePersistentVars(next);
      this.applyPersistentVars(next);
      const hasAuthoritativeChanged = typeof options.changed === "boolean" && !this.persistentVarIds.length;
      const changed = hasAuthoritativeChanged
        ? options.changed
        : this.stateKeyIgnoringPersistent(next) !== this.stateKeyIgnoringPersistent(this.state);
      if (!changed) {
        this.state = next;
        this.syncPersistentVarsToStates();
        return;
      }
      this.undoStack.push(this.historyEntryForCurrentState(options.previousStateHandle));
      this.state = next;
      this.redoStack = [];
      this.syncPersistentVarsToStates();
    }

    historyEntryForCurrentState(handle) {
      const numericHandle = Number(handle);
      if (Number.isInteger(numericHandle) && numericHandle >= 0) {
        return { handle: numericHandle };
      }
      return this.cloneState(this.state);
    }

    saveCurrentHistoryEntry() {
      if (!this.coreRuntime || typeof this.coreRuntime.save_current_state !== "function") {
        return this.cloneState(this.state);
      }
      this.syncCoreRuntimeState(this.state);
      return { handle: this.coreRuntime.save_current_state() };
    }

    restoreHistoryEntry(entry) {
      if (entry?.handle !== undefined
        && this.coreRuntime
        && typeof this.coreRuntime.restore_saved_state === "function"
        && typeof this.coreRuntime.current_state === "function") {
        this.coreRuntime.restore_saved_state(Number(entry.handle));
        const state = JSON.parse(this.coreRuntime.current_state());
        this.coreRuntimeStateHash = typeof this.coreRuntime.current_state_hash === "function"
          ? this.coreHashKey(this.coreRuntime.current_state_hash())
          : null;
        this.setCoreStateHash(state, this.coreRuntimeStateHash);
        return state;
      }
      return this.cloneState(entry);
    }

    gotoScene(name) {
      this.createScene(name);
      this.visibleScenes = [];
      this.showScene(name);
      this.focusHistory = [];
      this.focusScene(name);
    }

    enterScene(name) {
      this.createScene(name);
      if (this.focusedScene !== name) {
        this.focusHistory.push(this.focusedScene);
      }
      this.showScene(name);
      this.focusScene(name);
    }

    startScene(name) {
      this.resetSceneState(name);
      this.visibleScenes = [];
      this.showScene(name);
      this.focusHistory = [];
      this.focusScene(name);
    }

    backOrInitial() {
      const current = this.focusedScene;
      const previous = this.focusHistory.pop() || this.initialSceneName();
      this.hideSceneOnly(current);
      this.focusScene(previous);
    }

    createScene(name) {
      if (!this.sceneStates.has(name)) {
        this.ensureActiveLevelForScene(name, true);
        this.resetSceneState(name);
      }
    }

    ensureActiveLevelForScene(name, emitEvents = true) {
      const screen = this.sceneDef(name);
      const needsCurrentLevel = Boolean(screen?.puzzleRule)
        || (screen?.state?.puzzles || []).some((puzzle) => puzzle.initializer === "current_level");
      if (!needsCurrentLevel) {
        return;
      }
      if (this.levelIndex !== null && this.levelIndex !== undefined && this.sceneAcceptsLevel(name, this.levelIndex)) {
        return;
      }
      const selected = this.selectedLevelIndex;
      const index = Number.isInteger(selected) && this.sceneAcceptsLevel(name, selected)
        ? selected
        : this.firstLevelIndexForScene(name, null);
      if (index !== undefined) {
        this.activateLevel(index, emitEvents);
      }
    }

    firstLevelIndexForScene(sceneName, scope = null) {
      return this.sceneLevelIndices(sceneName)
        .find((candidate) => !scope || this.resourceMatches(scope, this.data.levels[candidate]?.name || ""));
    }

    levelIndexForSceneContinue(sceneName, scope = null) {
      const indices = this.sceneLevelIndices(sceneName)
        .filter((index) => !scope || this.resourceMatches(scope, this.data.levels[index]?.name || ""));
      const preferred = this.selectedLevelIndex >= 0
        && this.selectedLevelIndex < this.data.levels.length
        && indices.includes(this.selectedLevelIndex)
        ? this.selectedLevelIndex
        : undefined;
      if (preferred !== undefined) {
        if (this.clearedLevels[preferred] !== true) {
          return preferred;
        }
        const position = indices.indexOf(preferred);
        const nextUncleared = indices
          .slice(position + 1)
          .find((index) => this.clearedLevels[index] !== true);
        if (nextUncleared !== undefined) {
          return nextUncleared;
        }
      }
      return indices.find((index) => this.clearedLevels[index] !== true)
        ?? preferred
        ?? indices[0];
    }

    resetSceneState(name) {
      const previousValues = this.sceneStates.get(name)?.values || {};
      const next = this.defaultSceneState(name);
      const screen = this.sceneDef(name);
      for (const variable of screen?.state?.variables || []) {
        if (variable.lifetime === "persistent" && Object.prototype.hasOwnProperty.call(previousValues, variable.name)) {
          next.values[variable.name] = previousValues[variable.name];
        }
      }
      this.sceneStates.set(name, next);
    }

    copyPuzzleState(source, target) {
      const sourceTarget = this.resolvePuzzleTarget(source);
      const targetTarget = this.resolvePuzzleTarget(target);
      if (!sourceTarget || !targetTarget) {
        return;
      }
      this.createScene(sourceTarget.sceneName);
      this.createScene(targetTarget.sceneName);
      const sourcePuzzle = this.sceneStates.get(sourceTarget.sceneName)?.puzzles?.[sourceTarget.puzzleName];
      const targetPuzzle = this.sceneStates.get(targetTarget.sceneName)?.puzzles?.[targetTarget.puzzleName];
      if (!sourcePuzzle || !targetPuzzle) {
        return;
      }
      targetPuzzle.state = this.cloneState(sourcePuzzle.state);
      this.applyPersistentVars(targetPuzzle.state);
    }

    resetPuzzleState(target) {
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        return;
      }
      this.createScene(resolved.sceneName);
      const puzzle = this.sceneStates.get(resolved.sceneName)?.puzzles?.[resolved.puzzleName];
      if (!puzzle) {
        return;
      }
      puzzle.state = this.cloneState(puzzle.initialState);
      if (puzzle.checkpointState) {
        puzzle.state = this.cloneState(puzzle.checkpointState);
      }
      this.applyPersistentVars(puzzle.state);
      if (this.scenePuzzleInitializer(resolved.sceneName, resolved.puzzleName)?.initializer === "current_level") {
        this.replaceStateIfChanged(this.cloneState(puzzle.state));
        this.undoStack = [];
        this.redoStack = [];
        if (puzzle.levelIndex !== undefined) {
          this.levelIndex = puzzle.levelIndex;
          this.selectedLevelIndex = puzzle.levelIndex;
        }
        this.syncCurrentLevelPuzzles(resolved.sceneName);
      }
    }

    saveCheckpoint(target = null) {
      if (!target) {
        if (this.levelIndex !== null && this.levelIndex !== undefined) {
          this.levelCheckpointState = this.cloneState(this.state);
          this.syncCurrentLevelPuzzles();
        }
        return;
      }
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        if (this.levelIndex !== null && this.levelIndex !== undefined) {
          this.levelCheckpointState = this.cloneState(this.state);
          this.syncCurrentLevelPuzzles();
        }
        return;
      }
      this.createScene(resolved.sceneName);
      const puzzle = this.sceneStates.get(resolved.sceneName)?.puzzles?.[resolved.puzzleName];
      if (!puzzle) {
        return;
      }
      puzzle.checkpointState = this.cloneState(puzzle.state);
      if (this.scenePuzzleInitializer(resolved.sceneName, resolved.puzzleName)?.initializer === "current_level") {
        this.levelCheckpointState = this.cloneState(puzzle.state);
        this.syncCurrentLevelPuzzles(resolved.sceneName);
      }
    }

    clearCheckpoint(target = null) {
      if (!target) {
        this.levelCheckpointState = null;
        this.syncCurrentLevelPuzzles();
        return;
      }
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        this.levelCheckpointState = null;
        this.syncCurrentLevelPuzzles();
        return;
      }
      this.createScene(resolved.sceneName);
      const puzzle = this.sceneStates.get(resolved.sceneName)?.puzzles?.[resolved.puzzleName];
      if (puzzle) {
        puzzle.checkpointState = null;
      }
      if (this.scenePuzzleInitializer(resolved.sceneName, resolved.puzzleName)?.initializer === "current_level") {
        this.levelCheckpointState = null;
        this.syncCurrentLevelPuzzles(resolved.sceneName);
      }
    }

    loadPuzzleState(target, source, bindings = {}) {
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        return;
      }
      const levelIndex = this.evalPuzzleLevelRef(target, source, bindings);
      if (levelIndex === undefined) {
        return;
      }
      this.createScene(resolved.sceneName);
      const state = this.materializedLevelInitialState(levelIndex);
      this.sceneStates.get(resolved.sceneName).puzzles[resolved.puzzleName] = {
        state: this.cloneState(state),
        initialState: this.cloneState(state),
        checkpointState: null,
        levelIndex,
      };
      if (this.scenePuzzleInitializer(resolved.sceneName, resolved.puzzleName)?.initializer === "current_level") {
        this.levelCheckpointState = null;
        this.levelIndex = levelIndex;
        this.selectedLevelIndex = levelIndex;
        this.state = this.cloneState(state);
        this.undoStack = [];
        this.redoStack = [];
        this.syncCurrentLevelPuzzles(resolved.sceneName);
      }
    }

    evalPuzzleLevelRef(target, source, bindings = {}) {
      const text = String(source || "").trim();
      const nextMatch = text.match(/^next\((.*)\)$/);
      if (nextMatch) {
        const index = this.evalPuzzleLevelRef(target, nextMatch[1], bindings);
        return index === undefined ? undefined : Math.min(index + 1, this.data.levels.length - 1);
      }
      const resolved = this.resolvePuzzleTarget(target);
      if (!resolved) {
        return undefined;
      }
      const puzzle = this.sceneStates.get(resolved.sceneName)?.puzzles?.[resolved.puzzleName];
      if (text === `${resolved.puzzleName}.level` || text === `${resolved.sceneName}.${resolved.puzzleName}.level`) {
        return puzzle?.levelIndex;
      }
      const prefixes = [`${resolved.puzzleName}.levels[`, `${resolved.sceneName}.${resolved.puzzleName}.levels[`];
      for (const prefix of prefixes) {
        if (text.startsWith(prefix) && text.endsWith("]")) {
          const index = Number(text.slice(prefix.length, -1));
          if (Number.isInteger(index) && index >= 0 && index < this.data.levels.length) {
            return index;
          }
          const binding = bindings[text.slice(prefix.length, -1)];
          const boundIndex = Number(binding);
          return Number.isInteger(boundIndex) && boundIndex >= 0 && boundIndex < this.data.levels.length
            ? boundIndex
            : undefined;
        }
      }
      return this.levelIndexFromValue(bindings[text] ?? text);
    }

    defaultSceneState(name) {
      const screen = this.sceneDef(name);
      const values = {};
      const puzzles = {};
      for (const variable of screen?.state?.variables || []) {
        values[variable.name] = variable.default;
      }
      for (const puzzle of screen?.state?.puzzles || []) {
        if (puzzle.initializer === "current_level") {
          const levelIndex = this.levelIndex ?? this.sceneLevelIndices(name)[0];
          const state = levelIndex === undefined
            ? this.neutralState()
            : (this.levelIndex === levelIndex ? this.cloneState(this.state) : this.materializedLevelInitialState(levelIndex));
          puzzles[puzzle.name] = {
            state,
            initialState: levelIndex === undefined ? this.neutralState() : this.materializedLevelInitialState(levelIndex),
            checkpointState: this.levelIndex === levelIndex && this.levelCheckpointState
              ? this.cloneState(this.levelCheckpointState)
              : null,
            levelIndex,
          };
        } else if (puzzle.initializer === "level") {
          const index = this.levelIndexFromValue(puzzle.level);
          const state = index === undefined
            ? this.neutralState()
            : this.materializedLevelInitialState(index);
          puzzles[puzzle.name] = {
            state,
            initialState: this.cloneState(state),
            checkpointState: null,
            levelIndex: index,
          };
        }
      }
      return { values, puzzles };
    }

    deleteScene(name) {
      this.sceneStates.delete(name);
      this.visibleScenes = this.visibleScenes.filter((screen) => screen !== name);
      this.focusHistory = this.focusHistory.filter((screen) => screen !== name);
      if (this.focusedScene === name) {
        const previous = this.focusHistory.pop() || this.initialSceneName();
        this.createScene(previous);
        this.showScene(previous);
        this.focusedScene = previous;
      }
    }

    showScene(name) {
      this.createScene(name);
      if (!this.visibleScenes.includes(name)) {
        this.visibleScenes.push(name);
      }
    }

    hideScene(name) {
      this.hideSceneOnly(name);
      if (this.focusedScene === name) {
        const previous = this.visibleScenes.at(-1) || this.focusHistory.pop() || this.initialSceneName();
        this.createScene(previous);
        this.showScene(previous);
        this.focusedScene = previous;
      }
    }

    hideSceneOnly(name) {
      this.visibleScenes = this.visibleScenes.filter((screen) => screen !== name);
    }

    toggleScene(name) {
      if (this.visibleScenes.includes(name)) {
        this.hideScene(name);
      } else {
        this.showScene(name);
      }
    }

    focusScene(name) {
      this.createScene(name);
      this.showScene(name);
      this.focusedScene = name;
      this.applySceneStartTransition();
      this.applyLevelStartTransition();
    }

    applySceneStartTransition() {
      this.applyLifecycleTransition("scene_start");
    }

    applyLevelStartTransition() {
      this.applyLifecycleTransition("level_start");
    }

    applyLifecycleTransition(lifecycle) {
      const transition = (this.currentSceneDef()?.transitions || []).find((candidate) =>
        candidate.lifecycle === lifecycle,
      );
      if (transition) {
        this.applySceneEffect(transition.effect, {});
      }
    }

    syncCurrentLevelPuzzles(sceneName = this.focusedScene) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return;
      }
      for (const screen of this.data.screens.filter((screen) => screen.name === sceneName)) {
        const runtime = this.sceneStates.get(screen.name);
        if (!runtime) {
          continue;
        }
        for (const puzzle of screen.state?.puzzles || []) {
          if (puzzle.initializer === "current_level") {
            const initialState = this.materializedLevelInitialState(this.levelIndex);
            runtime.puzzles[puzzle.name] = {
              state: this.cloneState(this.state),
              initialState,
              checkpointState: this.levelCheckpointState ? this.cloneState(this.levelCheckpointState) : null,
              levelIndex: this.levelIndex,
            };
          }
        }
      }
    }

    focusedSceneState() {
      const screen = this.currentSceneDef();
      const runtime = this.focusedSceneRuntime();
      const target = screen?.puzzleRule?.target;
      if (target) {
        const resolved = this.resolvePuzzleTarget(target);
        const puzzle = resolved?.sceneName === this.focusedScene ? runtime.puzzles?.[resolved.puzzleName] : undefined;
        if (puzzle) {
          return puzzle.state;
        }
      }
      const puzzleName = this.firstPuzzleComponent(screen?.components || []);
      return puzzleName ? runtime.puzzles?.[puzzleName]?.state : undefined;
    }

    sceneLayers() {
      return this.visibleScenes.map((name) => {
        this.createScene(name);
        const runtime = this.sceneStates.get(name) || { values: {}, puzzles: {} };
        const puzzle = this.scenePuzzleState(name);
        return {
          name,
          focused: name === this.focusedScene,
          sceneState: runtime.values,
          scenePuzzles: Object.keys(runtime.puzzles || {}),
          scene: this.presentationSnapshotForPuzzle(puzzle)?.scene || null,
        };
      });
    }

    scenePuzzleState(name) {
      const screen = this.sceneDef(name);
      const runtime = this.sceneStates.get(name);
      const target = screen?.puzzleRule?.target;
      if (target) {
        const resolved = this.resolvePuzzleTarget(target);
        const puzzle = resolved?.sceneName === name ? runtime?.puzzles?.[resolved.puzzleName] : undefined;
        if (puzzle) {
          return puzzle;
        }
      }
      const puzzleName = this.firstPuzzleComponent(screen?.components || []);
      return puzzleName ? runtime?.puzzles?.[puzzleName] : undefined;
    }

    firstPuzzleComponent(components) {
      for (const component of components || []) {
        if ((component.kind === "puzzle" || component.kind === "frame") && component.source && component.source !== "current_level") {
          return component.source;
        }
        const child = this.firstPuzzleComponent(component.children || []);
        if (child) {
          return child;
        }
      }
      return undefined;
    }

    resolvePuzzleTarget(target) {
      const parts = String(target || "").split(".").filter(Boolean);
      if (parts.length === 1) {
        return { sceneName: this.focusedScene, puzzleName: parts[0] };
      }
      if (parts.length === 2) {
        return { sceneName: parts[0], puzzleName: parts[1] };
      }
      return undefined;
    }

    scenePuzzleInitializer(sceneName, puzzleName) {
      return (this.sceneDef(sceneName)?.state?.puzzles || []).find((puzzle) => puzzle.name === puzzleName);
    }

    currentSceneHasLevelMenu() {
      return (this.currentSceneDef()?.components || []).some((component) => this.componentHasLevelMenu(component));
    }

    currentSceneAcceptsModelInput() {
      return Boolean(this.currentSceneDef()?.puzzleRule);
    }

    componentHasLevelMenu(component) {
      if (component.kind === "level_menu") {
        return true;
      }
      return (component.children || []).some((child) => this.componentHasLevelMenu(child));
    }

    currentLevelMenuDef() {
      const find = (components) => {
        for (const component of components || []) {
          if (component.kind === "level_menu") {
            return component;
          }
          const child = find(component.children || []);
          if (child) {
            return child;
          }
        }
        return null;
      };
      return find(this.currentSceneDef()?.components || []);
    }

    applyLevelMenuCommand(command) {
      const [name, rawCursor] = String(command || "").split(":");
      const menu = this.currentLevelMenuDef();
      const levelIndices = this.sceneLevelIndices(this.focusedScene);
      const itemCount = levelIndices.length + (menu?.buttons?.length || 0);
      if (!menu || itemCount === 0) {
        this.selectedLevelIndex = 0;
        return false;
      }
      if (/^\d+$/.test(rawCursor || "")) {
        this.setLevelMenuCursorPosition(levelIndices, Math.min(itemCount - 1, Number(rawCursor)));
      }
      if (name === "up") {
        this.moveLevelMenuCursor(menu, levelIndices, -this.levelMenuColumns(menu));
        return true;
      }
      if (name === "down") {
        this.moveLevelMenuCursor(menu, levelIndices, this.levelMenuColumns(menu));
        return true;
      }
      if (name === "left") {
        this.moveLevelMenuCursor(menu, levelIndices, -1);
        return true;
      }
      if (name === "right") {
        this.moveLevelMenuCursor(menu, levelIndices, 1);
        return true;
      }
      if (name === "enter") {
        const cursor = this.levelMenuCursorPosition(levelIndices);
        if (levelIndices[cursor] !== undefined) {
          this.startLevel(levelIndices[cursor]);
        } else {
          const item = menu.buttons?.[cursor - levelIndices.length];
          this.applySceneEffect(item?.effect, {});
        }
        return true;
      }
      return false;
    }

    levelMenuColumns(menu) {
      return Math.max(1, Number(menu?.columns || 1));
    }

    moveLevelMenuCursor(menu, levelIndices, delta) {
      const itemCount = levelIndices.length + (menu?.buttons?.length || 0);
      if (!itemCount || !delta) {
        return;
      }
      const current = this.levelMenuCursorPosition(levelIndices);
      if (menu?.wrap) {
        let next = (current + delta) % itemCount;
        if (next < 0) {
          next += itemCount;
        }
        this.setLevelMenuCursorPosition(levelIndices, next);
        return;
      }
      this.setLevelMenuCursorPosition(levelIndices, Math.max(0, Math.min(itemCount - 1, current + delta)));
    }

    levelMenuCursorPosition(levelIndices) {
      const levelPosition = levelIndices.indexOf(this.selectedLevelIndex);
      if (levelPosition >= 0) {
        return levelPosition;
      }
      if (this.selectedLevelIndex >= this.data.levels.length) {
        return levelIndices.length + this.selectedLevelIndex - this.data.levels.length;
      }
      return 0;
    }

    setLevelMenuCursorPosition(levelIndices, position) {
      this.selectedLevelIndex = levelIndices[position] ?? (this.data.levels.length + Math.max(0, position - levelIndices.length));
    }

    parseRuntimeCommand(command) {
      const trimmed = String(command || "").trim();
      if (trimmed === "back" || trimmed === "close") {
        return { kind: "back" };
      }
      if (trimmed === "clear_undo_history" || trimmed === "clear_history") {
        return { kind: "clear_undo_history" };
      }
      if (trimmed === "clear_game_progress") {
        return { kind: "clear_game_progress" };
      }
      if (trimmed === "clear current_level") {
        return { kind: "clear_current_level" };
      }
      if (trimmed === "reset persistent_vars") {
        return { kind: "reset_persistent_vars" };
      }
      const setCurrentLevelMatch = trimmed.match(/^set\s+current_level\s*=\s*(.+)$/);
      if (setCurrentLevelMatch) {
        return { kind: "set_current_level", level: this.parseRuntimeExpr(setCurrentLevelMatch[1].trim()) };
      }
      const setCurrentLevelClearedMatch = trimmed.match(/^set\s+level\.cleared\s*=\s*(true|false)$/);
      if (setCurrentLevelClearedMatch) {
        return { kind: "set_level_cleared", cleared: setCurrentLevelClearedMatch[1] === "true" };
      }
      const setLevelClearedMatch = trimmed.match(/^set\s+level\((.+)\)\.cleared\s*=\s*(true|false)$/);
      if (setLevelClearedMatch) {
        return {
          kind: "set_level_cleared",
          level: this.parseRuntimeExpr(setLevelClearedMatch[1].trim()),
          cleared: setLevelClearedMatch[2] === "true",
        };
      }
      const messageMatch = trimmed.match(/^message\s+(.+)$/);
      if (messageMatch) {
        return { kind: "message", text: this.parseRuntimeExpr(messageMatch[1].trim()) };
      }
      if (trimmed === "wait") {
        return { kind: "wait", milliseconds: this.data.defaultWaitMs ?? 200 };
      }
      const waitMatch = trimmed.match(/^wait\s+([0-9]+(?:\.[0-9]{1,3})?s|[0-9]+ms)$/);
      if (waitMatch) {
        return { kind: "wait", milliseconds: this.parseWaitMilliseconds(waitMatch[1]) };
      }
      let soundMatch = trimmed.match(/^sfx\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      if (soundMatch) {
        return { kind: "play_sfx", name: soundMatch[1] };
      }
      soundMatch = trimmed.match(/^play_music\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      if (soundMatch) {
        return { kind: "play_music", name: soundMatch[1] };
      }
      soundMatch = trimmed.match(/^pause_music(?:\s+([A-Za-z_][A-Za-z0-9_]*))?$/);
      if (soundMatch) {
        return { kind: "pause_music", name: soundMatch[1] ?? null };
      }
      soundMatch = trimmed.match(/^resume_music(?:\s+([A-Za-z_][A-Za-z0-9_]*))?$/);
      if (soundMatch) {
        return { kind: "resume_music", name: soundMatch[1] ?? null };
      }
      soundMatch = trimmed.match(/^stop_music(?:\s+([A-Za-z_][A-Za-z0-9_]*))?$/);
      if (soundMatch) {
        return { kind: "stop_music", name: soundMatch[1] ?? null };
      }
      const loadMatch = trimmed.match(/^load\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s+from\s+(.+)$/);
      if (loadMatch) {
        return { kind: "puzzle_load", target: loadMatch[1], source: loadMatch[2].trim() };
      }
      const startMatch = trimmed.match(/^start\s+levels(?:\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*))?\s+in\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      if (startMatch) {
        return { kind: "start_level", scope: startMatch[1] ?? null, scene: startMatch[2] };
      }
      const continueMatch = trimmed.match(/^continue\s+levels(?:\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*))?\s+in\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      if (continueMatch) {
        return { kind: "continue_level", scope: continueMatch[1] ?? null, scene: continueMatch[2] };
      }
      const match = trimmed.match(/^(goto|resume|enter|open|start|create|reset|delete|show|hide|toggle|focus)\s+(.+)$/);
      if (!match) {
        return null;
      }
      let [, kind, targetText] = match;
      const target = this.parseRuntimeSceneTarget(targetText.trim());
      if (!target) {
        return null;
      }
      const { screen, params: parsedParams } = target;
      if (["create", "reset", "delete", "show", "hide", "toggle", "focus"].includes(kind) && parsedParams.length) {
        return null;
      }
      if (kind === "resume") {
        kind = "goto";
      } else if (kind === "open") {
        kind = "enter";
      } else if (kind === "start") {
        return {
          kind: "sequence",
          effects: [
            { kind: "reset", screen },
            { kind: "goto", screen, params: parsedParams },
          ],
        };
      }
      return { kind, screen, params: parsedParams };
    }

    parseRuntimeSceneTarget(value) {
      const identifier = /^[A-Za-z_][A-Za-z0-9_]*$/;
      const withIndex = value.indexOf(" with ");
      if (withIndex >= 0) {
        const screen = value.slice(0, withIndex).trim();
        if (!identifier.test(screen)) {
          return null;
        }
        return {
          screen,
          params: this.parseRuntimeParams(value.slice(withIndex + " with ".length).trim()),
        };
      }
      const argsStart = value.indexOf("(");
      if (argsStart >= 0) {
        if (!value.endsWith(")")) {
          return null;
        }
        const screen = value.slice(0, argsStart).trim();
        if (!identifier.test(screen)) {
          return null;
        }
        const args = value.slice(argsStart + 1, -1).trim();
        if (!args) {
          return { screen, params: [] };
        }
        if (!args.includes("=") && !args.includes(",")) {
          return {
            screen,
            params: [{ name: "level", value: this.parseRuntimeExpr(args) }],
          };
        }
        return { screen, params: this.parseRuntimeParams(args) };
      }
      if (!identifier.test(value)) {
        return null;
      }
      return { screen: value, params: [] };
    }

    parseWaitMilliseconds(value) {
      if (String(value).endsWith("ms")) {
        return Number.parseInt(value, 10);
      }
      const seconds = String(value).slice(0, -1);
      const [whole, fraction = ""] = seconds.split(".");
      return Number.parseInt(whole, 10) * 1000 + Number.parseInt(fraction.padEnd(3, "0") || "0", 10);
    }

    parsePuzzleRuntimeCommand(command) {
      const trimmed = String(command || "").trim();
      const gotoMatch = trimmed.match(/^([A-Za-z_][A-Za-z0-9_]*)\.goto\s+(.+)$/);
      if (gotoMatch) {
        return { kind: "puzzle_goto_level", target: gotoMatch[1], level: this.parseRuntimeExpr(gotoMatch[2].trim()) };
      }
      const [target, puzzleCommand] = trimmed.split(".");
      if (!target) {
        return null;
      }
      if (puzzleCommand === "next_level") {
        return { kind: "puzzle_next_level", target };
      }
      if (puzzleCommand === "previous_level") {
        return { kind: "puzzle_previous_level", target };
      }
      if (puzzleCommand === "restart") {
        return { kind: "puzzle_reset", target };
      }
      return null;
    }

    parseRuntimeParams(value) {
      return String(value)
        .split(",")
        .map((part) => part.trim())
        .filter(Boolean)
        .map((part) => {
          const [name, rawValue] = part.split("=").map((segment) => segment.trim());
          return { name, value: this.parseRuntimeExpr(rawValue) };
        });
    }

    parseRuntimeExpr(value) {
      if (value === "true") return { kind: "bool", value: true };
      if (value === "false") return { kind: "bool", value: false };
      if (/^-?\d+$/.test(value)) return { kind: "int", value: Number(value) };
      const quoted = String(value).match(/^"(.*)"$/);
      if (quoted) return { kind: "text", value: quoted[1].replace(/\\"/g, "\"") };
      const call = String(value).match(/^([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$/);
      if (call) {
        const args = call[2].trim()
          ? call[2].split(",").map((arg) => this.parseRuntimeExpr(arg.trim()))
          : [];
        return { kind: "call", name: call[1], args };
      }
      return { kind: "path", path: String(value || "") };
    }

    currentLevel() {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return null;
      }
      return this.data.levels[this.levelIndex];
    }

    markCurrentLevelCleared() {
      if (this.levelIndex !== null && this.levelIndex >= 0 && this.levelIndex < this.clearedLevels.length) {
        this.clearedLevels[this.levelIndex] = true;
        this.writeProgressSave();
      }
    }

    clearUndoHistory() {
      this.undoStack = [];
      this.redoStack = [];
    }

    clearGameProgress() {
      this.clearedLevels = new Array(this.data.levels.length).fill(false);
      this.selectedLevelIndex = 0;
      this.restoredLevelIndex = null;
      this.hasProgressSave = false;
      this.resetPersistentVars({ write: false });
      this.clearUndoHistory();
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (_error) {
        // Ignore storage failures; the in-memory progress was already cleared.
      }
    }

    clearCurrentLevelProgress() {
      this.selectedLevelIndex = 0;
      this.restoredLevelIndex = null;
      this.writeProgressSave();
    }

    setCurrentLevelProgress(level, bindings = {}) {
      const value = this.evalEffectString(level, bindings);
      const index = this.levelIndexFromValue(value);
      if (index === undefined) {
        return;
      }
      this.selectedLevelIndex = index;
      this.restoredLevelIndex = index;
      this.writeProgressSave();
    }

    setLevelClearedProgress(level, cleared, bindings = {}) {
      const index = level === undefined || level === null
        ? (this.levelIndex ?? this.selectedLevelIndex)
        : this.levelIndexFromValue(this.evalEffectString(level, bindings));
      if (index === undefined || index < 0 || index >= this.clearedLevels.length) {
        return;
      }
      this.clearedLevels[index] = cleared === true;
      this.writeProgressSave();
    }

    resetPersistentVars(options = {}) {
      this.persistentVars = this.persistentVarIds.map((varId) => this.persistentVarDefaultValue(varId));
      this.syncPersistentVarsToStates();
      if (options.write !== false) {
        this.writeProgressSave();
      }
    }

    resetPersistentVar(name, options = {}) {
      const varId = this.globalIdsByName.get(name);
      const index = this.persistentVarIds.indexOf(varId);
      if (index < 0) {
        return false;
      }
      this.persistentVars[index] = this.persistentVarDefaultValue(varId);
      this.syncPersistentVarsToStates();
      if (options.write !== false) {
        this.writeProgressSave();
      }
      return true;
    }

    persistentVarDefaultValue(varId) {
      const state = this.data.levels?.[0]?.initialState;
      const value = state?.globals?.[varId];
      return Number.isFinite(Number(value)) ? Math.trunc(Number(value)) : 0;
    }

    progressSaveVersion() {
      return Number(this.data.progressSaveVersion || 1);
    }

    progressSaveStorageKey() {
      const key = this.data.saveKey || this.data.puzzlePath || this.data.title || "untitled";
      return `PuzzleStudio.progress.v${this.progressSaveVersion()}:${key}`;
    }

    progressSaveData() {
      return {
        version: this.progressSaveVersion(),
        levels: this.data.levels.map((level, index) => ({
          name: level.name,
          cleared: this.clearedLevels[index] === true,
        })),
        currentLevel: this.currentSaveLevelName(),
        persistentVars: this.persistentVarSaveData(),
      };
    }

    currentSaveLevelName() {
      const index = this.levelIndex ?? this.selectedLevelIndex;
      return this.data.levels[index]?.name || null;
    }

    persistentVarSaveData() {
      return this.persistentVarIds
        .map((varId, index) => ({
          name: this.varNamesById.get(varId) || "",
          value: this.persistentVars[index] ?? 0,
        }))
        .filter((entry) => entry.name);
    }

    restoreProgressSave() {
      let raw;
      try {
        raw = window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
        return;
      }
      if (!raw) {
        return;
      }

      let save;
      try {
        save = JSON.parse(raw);
      } catch (_error) {
        return;
      }
      if (!save || Number(save.version) !== this.progressSaveVersion() || !Array.isArray(save.levels)) {
        return;
      }
      this.hasProgressSave = true;

      const levelIndexByName = new Map(this.data.levels.map((level, index) => [level.name, index]));
      for (const entry of save.levels) {
        if (!entry?.cleared) {
          continue;
        }
        const index = levelIndexByName.get(entry.name);
        if (index !== undefined && index >= 0 && index < this.clearedLevels.length) {
          this.clearedLevels[index] = true;
        }
      }
      const currentLevelIndex = levelIndexByName.get(save.currentLevel);
      if (currentLevelIndex !== undefined) {
        this.restoredLevelIndex = currentLevelIndex;
      }
      if (Array.isArray(save.persistentVars)) {
        const varIndexByName = new Map(this.persistentVarIds.map((varId, index) => [this.varNamesById.get(varId), index]));
        for (const entry of save.persistentVars) {
          const index = varIndexByName.get(entry?.name);
          const value = Number(entry?.value);
          if (index !== undefined && Number.isFinite(value)) {
            this.persistentVars[index] = Math.trunc(value);
          }
        }
      }
    }

    restoreSessionProgressSave() {
      if (!this.sessionRuntime) {
        return;
      }
      let raw;
      try {
        raw = window.localStorage?.getItem(this.progressSaveStorageKey());
      } catch (_error) {
        return;
      }
      if (!raw) {
        return;
      }
      try {
        this.sessionRuntime.restore_progress_save(raw);
        this.sessionRuntime.mark_progress_save_written();
      } catch (_error) {
        // Ignore incompatible saves; the Rust session will start from defaults.
      }
    }

    writeSessionProgressSave() {
      if (!this.sessionRuntime) {
        return;
      }
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), this.sessionRuntime.progress_save());
        this.sessionRuntime.mark_progress_save_written();
      } catch (_error) {
        // Browsers can deny storage for local files, private windows, or quota limits.
      }
    }

    clearSessionProgressSave() {
      if (this.sessionRuntime) {
        this.sessionRuntime.clear_progress_save();
      }
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (_error) {
        // Ignore storage failures; the in-memory progress was already cleared.
      }
    }

    writeProgressSave() {
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), JSON.stringify(this.progressSaveData()));
        this.hasProgressSave = true;
      } catch (_error) {
        // Browsers can deny storage for local files, private windows, or quota limits.
      }
    }

    hasProgressSaveData() {
      return this.hasProgressSave === true;
    }

    clearProgressSave() {
      this.clearGameProgress();
    }

    clampLevelIndex(index) {
      const number = Number(index);
      if (!Number.isFinite(number)) {
        return 0;
      }
      return Math.max(0, Math.min(this.data.levels.length - 1, Math.trunc(number)));
    }

    hasNextLevel() {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        return false;
      }
      const indices = this.sceneLevelIndices(this.focusedScene);
      const position = indices.indexOf(this.levelIndex);
      return position >= 0 && position + 1 < indices.length;
    }

    hasNextLevelInScene(sceneName, levelIndex) {
      const indices = this.sceneLevelIndices(sceneName);
      const position = indices.indexOf(levelIndex);
      return position >= 0 && position + 1 < indices.length;
    }

    scenePuzzleRefs() {
      const runtime = this.focusedSceneRuntime();
      const refs = {};
      for (const [name, puzzle] of Object.entries(runtime.puzzles || {})) {
        refs[name] = {
          level: this.levelRef(this.focusedScene, puzzle.levelIndex),
        };
      }
      return refs;
    }

    levelRef(sceneName, levelIndex) {
      if (levelIndex === undefined || levelIndex === null) {
        return null;
      }
      const level = this.data.levels[levelIndex];
      const hasNext = this.hasNextLevelInScene(sceneName, levelIndex);
      return {
        kind: "level",
        index: levelIndex,
        num: levelIndex + 1,
        number: levelIndex + 1,
        name: level?.name,
        label: level?.label || level?.name,
        title: level?.title || level?.label || level?.name,
        cleared: this.clearedLevels[levelIndex] === true,
        solved: this.clearedLevels[levelIndex] === true,
        has_next: hasNext,
        last: !hasNext,
      };
    }

    levelPathValue(path) {
      const parts = String(path || "").split(".").filter(Boolean);
      if (parts.length !== 3 || parts[1] !== "level") {
        return undefined;
      }
      const puzzle = this.focusedSceneRuntime().puzzles?.[parts[0]];
      const level = this.levelRef(this.focusedScene, puzzle?.levelIndex);
      return level?.[parts[2]];
    }

    currentSceneDef() {
      return this.sceneDef(this.focusedScene);
    }

    sceneDef(name) {
      const scenes = this.data.screens || this.data.scenes || [];
      return scenes.find((screen) => screen.name === name) || null;
    }

    sceneLevelIndices(sceneName = this.focusedScene) {
      const scene = this.sceneDef(sceneName);
      const resources = scene?.resources || {};
      if ((resources.levelsMode || "all") !== "named") {
        return this.data.levels.map((_, index) => index);
      }
      const names = resources.levels || [];
      return this.data.levels
        .map((level, index) => (names.some((name) => this.resourceMatches(name, level?.name || "")) ? index : -1))
        .filter((index) => index >= 0);
    }

    sceneAcceptsLevel(sceneName, levelIndex) {
      return this.sceneLevelIndices(sceneName).includes(levelIndex);
    }

    resourceMatches(resource, name) {
      return name === resource || String(name || "").startsWith(`${resource}.`);
    }

    focusedSceneRuntime() {
      this.createScene(this.focusedScene);
      return this.sceneStates.get(this.focusedScene);
    }

    initialSceneName() {
      return this.data.screens[0]?.name || "playing";
    }

    initialLevelSceneName() {
      return (
        this.data.screens.find((screen) => this.isLevelScene(screen.name))?.name || this.initialSceneName()
      );
    }

    isLevelScene(name) {
      const screen = this.data.screens.find((candidate) => candidate.name === name);
      return Boolean(
        screen?.puzzleRule || (screen?.state?.puzzles || []).some((puzzle) => puzzle.initializer === "current_level"),
      );
    }

    gameHasSceneLevelOwner() {
      return this.data.screens.some((screen) =>
        screen?.puzzleRule || (screen?.state?.puzzles || []).some((puzzle) => puzzle.initializer === "current_level"),
      );
    }

    levelIndexFromValue(value) {
      if (this.isLevelRef(value)) {
        return value.index;
      }
      if (/^\d+$/.test(String(value))) {
        const index = Number(value);
        return index < this.data.levels.length ? index : undefined;
      }
      const found = this.data.levels.findIndex((level) => level.name === value);
      return found >= 0 ? found : undefined;
    }

    levelValueFromAtom(value) {
      const index = this.levelIndexFromValue(value);
      return index === undefined ? value : this.levelRef(this.focusedScene, index);
    }

    splitCommandValue(command) {
      const index = String(command).indexOf(":");
      return index < 0 ? [command, undefined] : [command.slice(0, index), command.slice(index + 1)];
    }

    conditionName(value) {
      const parts = String(value).split(".");
      return parts[parts.length - 1];
    }

    cloneState(state) {
      const cloned = {
        width: state.width,
        height: state.height,
        layerCount: state.layerCount,
        slots: [...state.slots],
        cellScratch: this.cloneScratchStore(state.cellScratch),
        scratch: this.cloneScratchStore(state.scratch),
        globals: [...(state.globals || [])],
        levelFiredRules: [...(state.levelFiredRules || [])],
      };
      this.setCoreStateHash(cloned, this.coreStateHash(state));
      return cloned;
    }

    coreHashKey(value) {
      if (value === undefined || value === null) {
        return null;
      }
      return String(value);
    }

    coreStateHash(state) {
      if (!state || !Object.prototype.hasOwnProperty.call(state, CORE_STATE_HASH_PROPERTY)) {
        return null;
      }
      return this.coreHashKey(state[CORE_STATE_HASH_PROPERTY]);
    }

    setCoreStateHash(state, hash) {
      if (!state) {
        return;
      }
      const key = this.coreHashKey(hash);
      if (key === null) {
        this.clearCoreStateHash(state);
        return;
      }
      Object.defineProperty(state, CORE_STATE_HASH_PROPERTY, {
        value: key,
        enumerable: false,
        configurable: true,
        writable: true,
      });
    }

    clearCoreStateHash(state) {
      if (state && Object.prototype.hasOwnProperty.call(state, CORE_STATE_HASH_PROPERTY)) {
        delete state[CORE_STATE_HASH_PROPERTY];
      }
    }

    neutralState() {
      return {
        width: 1,
        height: 1,
        layerCount: this.engine.layerCount || 1,
        slots: new Array(Math.max(1, this.engine.layerCount || 1)).fill(0),
        cellScratch: [],
        scratch: [],
        globals: [],
        levelFiredRules: [],
      };
    }

    cloneScratchStore(store) {
      if (!store?.some((attrs) => attrs?.length)) {
        return [];
      }
      const cloned = store.map((attrs) => attrs?.length ? attrs.map((attr) => ({ ...attr })) : []);
      while (cloned.length && !cloned[cloned.length - 1]?.length) {
        cloned.pop();
      }
      return cloned;
    }

    scratchStoreKey(store) {
      if (!store?.some((attrs) => attrs?.length)) {
        return "";
      }
      let last = store.length - 1;
      while (last >= 0 && !store[last]?.length) {
        last -= 1;
      }
      const parts = [];
      for (let index = 0; index <= last; index += 1) {
        parts.push((store[index] || [])
          .map((attr) => `${attr.scratch}:${Object.hasOwn(attr, "value") ? attr.value : ""}`)
          .join("."));
      }
      return parts.join(",");
    }

    readPersistentVars(state) {
      return this.persistentVarIds.map((varId) => state.globals?.[varId] ?? 0);
    }

    capturePersistentVars(state) {
      this.persistentVars = this.readPersistentVars(state);
      this.writeProgressSave();
    }

    applyPersistentVars(state) {
      if (!this.persistentVarIds.length) {
        return;
      }
      state.globals = [...(state.globals || [])];
      let changed = false;
      this.persistentVarIds.forEach((varId, index) => {
        const value = this.persistentVars[index] ?? 0;
        if (state.globals[varId] !== value) {
          changed = true;
          state.globals[varId] = value;
        }
      });
      if (changed) {
        this.clearCoreStateHash(state);
      }
    }

    syncPersistentVarsToStates() {
      this.applyPersistentVars(this.state);
      if (this.levelCheckpointState) {
        this.applyPersistentVars(this.levelCheckpointState);
      }
      for (const runtime of this.sceneStates.values()) {
        for (const puzzle of Object.values(runtime.puzzles || {})) {
          this.applyPersistentVars(puzzle.state);
          this.applyPersistentVars(puzzle.initialState);
          if (puzzle.checkpointState) {
            this.applyPersistentVars(puzzle.checkpointState);
          }
        }
      }
    }

    stateKeyIgnoringPersistent(state) {
      const clone = this.cloneState(state);
      for (const varId of this.persistentVarIds) {
        clone.globals[varId] = 0;
      }
      return this.stateKey(clone);
    }

    stateKey(state) {
      const cellAttrs = this.scratchStoreKey(state.cellScratch);
      const attrs = this.scratchStoreKey(state.scratch);
      return `${state.width}x${state.height}x${state.layerCount}|${state.slots.join(",")}|${cellAttrs}|${attrs}|${state.globals.join(",")}|${(state.levelFiredRules || []).join(",")}`;
    }

    slotIndex(state, x, y, layer) {
      return ((y * state.width + x) * state.layerCount) + layer;
    }

    cellIndex(state, x, y) {
      return (y * state.width) + x;
    }

    hasObject(state, x, y, object) {
      const layer = this.objectLayers.get(object);
      if (layer === undefined) {
        return false;
      }
      return state.slots[this.slotIndex(state, x, y, layer)] === object;
    }

    hasScratchPattern(state, x, y, attr) {
      if (!attr.object) {
        const attrs = state.cellScratch?.[this.cellIndex(state, x, y)] || [];
        if (attr.match === "any") {
          return attrs.some((entry) => entry.scratch === attr.scratch);
        }
        const value = Object.hasOwn(attr, "value") ? attr.value : null;
        return attrs.some((entry) => entry.scratch === attr.scratch && (Object.hasOwn(entry, "value") ? entry.value : null) === value);
      }
      const layer = this.objectLayers.get(attr.object);
      if (layer === undefined) {
        return false;
      }
      const index = this.slotIndex(state, x, y, layer);
      if (state.slots[index] !== attr.object) {
        return false;
      }
      const attrs = state.scratch?.[index] || [];
      if (attr.match === "any") {
        return attrs.some((entry) => entry.scratch === attr.scratch);
      }
      const value = Object.hasOwn(attr, "value") ? attr.value : null;
      return attrs.some((entry) => entry.scratch === attr.scratch && (Object.hasOwn(entry, "value") ? entry.value : null) === value);
    }

    setObject(state, x, y, object) {
      const layer = this.objectLayers.get(object);
      if (layer === undefined) {
        throw this.patchError("unknown_object", `unknown object: ${object}`);
      }
      const index = this.slotIndex(state, x, y, layer);
      const existing = state.slots[index];
      if (existing === object) {
        return;
      }
      if (existing) {
        throw this.patchError("layer_occupied", `layer occupied at ${x},${y}`);
      }
      this.clearCoreStateHash(state);
      state.slots[index] = object;
      if (state.scratch?.[index]?.length) {
        state.scratch[index] = [];
      }
    }

    removeObject(state, x, y, object) {
      const layer = this.objectLayers.get(object);
      if (layer === undefined) {
        throw this.patchError("unknown_object", `unknown object: ${object}`);
      }
      const index = this.slotIndex(state, x, y, layer);
      if (state.slots[index] !== object) {
        throw this.patchError("expected_object", `expected object ${object} at ${x},${y}`);
      }
      this.clearCoreStateHash(state);
      state.slots[index] = 0;
      if (state.scratch?.[index]?.length) {
        state.scratch[index] = [];
      }
    }

    moveObject(state, fromX, fromY, toX, toY, object) {
      const layer = this.objectLayers.get(object);
      const fromIndex = this.slotIndex(state, fromX, fromY, layer);
      const toIndex = this.slotIndex(state, toX, toY, layer);
      if (state.slots[fromIndex] !== object) {
        return;
      }
      this.clearCoreStateHash(state);
      state.slots[fromIndex] = 0;
      state.slots[toIndex] = object;
      const scratch = state.scratch?.[fromIndex] || [];
      if (scratch.length) {
        state.scratch = state.scratch || [];
        state.scratch[toIndex] = scratch;
        state.scratch[fromIndex] = [];
      } else if (state.scratch?.[toIndex]?.length) {
        state.scratch[toIndex] = [];
      }
    }

    setScratch(state, x, y, object, scratch, value) {
      if (!object) {
        const index = this.cellIndex(state, x, y);
        this.clearCoreStateHash(state);
        state.cellScratch = state.cellScratch || [];
        const attrs = state.cellScratch[index] || [];
        const found = attrs.find((entry) => entry.scratch === scratch);
        if (found) {
          if (value === null) {
            delete found.value;
          } else {
            found.value = value;
          }
        } else {
          const entry = { scratch };
          if (value !== null) {
            entry.value = value;
          }
          attrs.push(entry);
        }
        state.cellScratch[index] = attrs;
        return;
      }
      const layer = this.objectLayers.get(object);
      if (layer === undefined) {
        throw this.patchError("unknown_object", `unknown object: ${object}`);
      }
      const index = this.slotIndex(state, x, y, layer);
      if (state.slots[index] !== object) {
        throw this.patchError("expected_object", `expected object ${object} at ${x},${y}`);
      }
      this.clearCoreStateHash(state);
      state.scratch = state.scratch || [];
      const attrs = state.scratch[index] || [];
      const found = attrs.find((entry) => entry.scratch === scratch);
      if (found) {
        if (value === null) {
          delete found.value;
        } else {
          found.value = value;
        }
      } else {
        const entry = { scratch };
        if (value !== null) {
          entry.value = value;
        }
        attrs.push(entry);
      }
      state.scratch[index] = attrs;
    }

    removeScratch(state, x, y, object, scratch, value, matchMode) {
      if (!object) {
        const index = this.cellIndex(state, x, y);
        const attrs = state.cellScratch?.[index] || [];
        if (!attrs.length) {
          return;
        }
        this.clearCoreStateHash(state);
        state.cellScratch[index] = attrs.filter((entry) => {
          if (entry.scratch !== scratch) {
            return true;
          }
          if (matchMode === "any") {
            return false;
          }
          return (Object.hasOwn(entry, "value") ? entry.value : null) !== value;
        });
        return;
      }
      const layer = this.objectLayers.get(object);
      if (layer === undefined) {
        throw this.patchError("unknown_object", `unknown object: ${object}`);
      }
      const index = this.slotIndex(state, x, y, layer);
      if (state.slots[index] !== object) {
        throw this.patchError("expected_object", `expected object ${object} at ${x},${y}`);
      }
      const attrs = state.scratch?.[index] || [];
      if (!attrs.length) {
        return;
      }
      this.clearCoreStateHash(state);
      state.scratch[index] = attrs.filter((entry) => {
        if (entry.scratch !== scratch) {
          return true;
        }
        if (matchMode === "any") {
          return false;
        }
        return (Object.hasOwn(entry, "value") ? entry.value : null) !== value;
      });
    }

    clearScratch(state) {
      if (state?.cellScratch?.some((attrs) => attrs?.length)
        || state?.scratch?.some((attrs) => attrs?.length)) {
        this.clearCoreStateHash(state);
      }
      state.cellScratch = [];
      state.scratch = [];
    }

    objectCount(state, object) {
      return state.slots.reduce((count, found) => count + (found === object ? 1 : 0), 0);
    }

    patchError(code, message) {
      const error = new Error(message);
      error.code = code;
      return error;
    }

    sceneFromState(state, levelIndex = this.levelIndex) {
      const cells = [];
      for (let y = 0; y < state.height; y += 1) {
        for (let x = 0; x < state.width; x += 1) {
          const layers = [];
          for (let layer = 0; layer < state.layerCount; layer += 1) {
            const objectId = state.slots[this.slotIndex(state, x, y, layer)];
            if (!objectId) {
              continue;
            }
            const object = this.objectsById.get(objectId);
            layers.push({
              layer,
              objectId,
              object: object?.name || "?",
              sprite: object?.sprite || "unknown",
            });
          }
          cells.push({ x, y, layers });
        }
      }
      return {
        width: state.width,
        height: state.height,
        layerCount: state.layerCount,
        settings: {
          animation: this.data.animation || null,
        },
        animation: this.data.animation || null,
        screen: this.data.screen,
        regions: this.data.levels[levelIndex]?.regions || [],
        cells,
      };
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
