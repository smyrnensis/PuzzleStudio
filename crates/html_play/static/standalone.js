(function () {
  const UNTIL_STABLE_REPEAT_LIMIT = 200;

  class PuzzleStandaloneRuntime {
    constructor(exportData) {
      this.data = exportData;
      this.data.scenes = this.data.scenes || this.data.screens || [];
      this.data.screens = this.data.screens || this.data.scenes;
      this.engine = exportData.engine;
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
      this.persistentVars = new Array(this.persistentVarIds.length).fill(0);
      this.undoStack = [];
      this.redoStack = [];
      this.clearedLevels = new Array(this.data.levels.length).fill(false);
      this.restoredLevelIndex = null;
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
      this.pendingWaits = 0;
      this.pendingAgainTurns = 0;
      this.againRunToken = 0;
      this.defaultAgainMs = Number(exportData.defaultAgainMs ?? 120);
      this.currentInput = null;
      this.maxAgainTurnsPerInput = 256;
      this.coreRuntime = null;
      this.editorPreviewSceneEnabled = false;
      this.editorPreviewInputEnabled = false;
      this.initialized = false;
      this.initializationPromise = this.initializeRuntime();
    }

    async requestJson(url, options = {}) {
      await this.ensureInitialized();
      const method = options.method || "GET";
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
      const embedded = window.PuzzleStandaloneEmbeddedWasm;
      if (!embedded?.moduleSource || !embedded?.wasmBase64) {
        throw new Error("Puzzle core WASM is not embedded in this standalone export.");
      }
      const version = String(this.data?.engineVersion || Date.now());
      const url = URL.createObjectURL(new Blob([embedded.moduleSource], {
        type: "text/javascript",
      }));
      try {
        const module = await import(`${url}#${encodeURIComponent(version)}`);
        await module.default({ module_or_path: this.base64ToUint8Array(embedded.wasmBase64) });
        if (typeof module.WasmCoreRuntime !== "function") {
          throw new Error("Puzzle core WASM runtime is unavailable.");
        }
        this.coreRuntime = new module.WasmCoreRuntime(this.data.source || "", this.data.puzzlePath || "game.puzzle");
      } finally {
        URL.revokeObjectURL(url);
      }
    }

    base64ToUint8Array(value) {
      const binary = atob(value || "");
      const bytes = new Uint8Array(binary.length);
      for (let index = 0; index < binary.length; index += 1) {
        bytes[index] = binary.charCodeAt(index);
      }
      return bytes;
    }

    snapshot() {
      const soundEvents = this.soundEvents.splice(0);
      const messageEvents = this.messageEvents.splice(0);
      const focusedPuzzle = this.scenePuzzleState(this.focusedScene);
      const presentation = focusedPuzzle
        ? this.presentationSnapshotForPuzzle(focusedPuzzle)
        : this.editorPresentationSnapshot();
      return {
        game: {
          title: this.data.title,
        },
        sounds: this.data.sounds || { sfx: [], music: [] },
        soundEvents,
        messageEvents,
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
      try {
        this.applyTurnCompletion([]);
      } finally {
        this.currentInput = previousInput;
      }
    }

    applyInput(input) {
      const previousInput = this.currentInput;
      this.currentInput = this.inputNamesById.get(input) ?? null;
      try {
        const result = this.applyModelInput(input);
        if (!result?.cancelled) {
          this.applyTurnCompletion(result?.commands || []);
        }
      } finally {
        this.currentInput = previousInput;
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
      this.replaceStateIfChanged(outcome.state);
      this.syncCurrentLevelPuzzles();
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
        this.replaceStateIfChanged(outcome.state);
        this.syncCurrentLevelPuzzles();
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
        this.replaceStateIfChanged(outcome.state);
        this.syncCurrentLevelPuzzles();
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
        this.replaceStateIfChanged(this.cloneState(next));
      } else {
        this.syncPersistentVarsToStates();
      }
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
      for (const queued of commands || []) {
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
        if (command?.kind === "play_sfx") {
          this.soundEvents.push({ kind: "play_sfx", name: command.name });
          continue;
        }
        if (command?.kind === "wait") {
          this.queueWait(command.milliseconds || command.ms || 0);
          continue;
        }
        if (command?.kind === "message") {
          this.messageEvents.push({
            kind: "message",
            text: this.resolveMessageText(command.text, command.literal),
          });
        }
      }
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
    }

    applyCommandName(command) {
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
      return this.transitionProgramOutcome(this.engine.program || [], initialState, input, "main");
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
      if (programKey !== "custom") {
        return this.coreTransitionProgramOutcome(programKey, levelIndex, initialState, input);
      }
      const original = this.cloneState(initialState);
      this.clearScratch(original);
      let current = this.cloneState(initialState);
      this.clearScratch(current);
      const commands = [];
      for (const step of program) {
        const result = this.applyStep(step, input, current);
        if (result.cancelled) {
          return { state: original, cancelled: true, commands: [] };
        }
        current = result.state;
        commands.push(...(result.commands || []));
      }
      this.clearScratch(current);
      return { state: current, cancelled: false, commands };
    }

    coreTransitionProgramOutcome(programKey, levelIndex, initialState, input) {
      if (!this.coreRuntime) {
        throw new Error("Puzzle core WASM runtime has not been initialized.");
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
        cancelled: outcome.cancelled === true,
        commands: this.commandsForCoreOutcome(outcome),
      };
    }

    commandsForCoreOutcome(outcome) {
      const commands = [...(outcome.commands || [])];
      for (const ruleId of outcome.firedRules || []) {
        commands.push(...this.ruleEmissionCommands(ruleId));
      }
      return commands;
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

    applyStep(step, input, state) {
      if (step.kind === "rule") {
        return this.applyRuleStep(step.rule, input, state);
      }
      if (step.kind === "conditional") {
        if (!this.conditionAccepts(step.condition, state, input)) {
          return { fired: false, state };
        }
        let current = state;
        let fired = false;
        const commands = [];
        for (const child of step.steps) {
          const result = this.applyStep(child, input, current);
          if (result.cancelled) {
            return { fired: true, state: current, cancelled: true };
          }
          current = result.state;
          commands.push(...(result.commands || []));
          fired = fired || result.fired;
        }
        return { fired, state: current, commands };
      }
      if (step.application === "once" || step.application === "once_all" || step.application === "once_per_level") {
        let current = state;
        let fired = false;
        const commands = [];
        for (const child of step.steps) {
          const result = this.applyStep(child, input, current);
          if (result.cancelled) {
            return { fired: true, state: current, cancelled: true };
          }
          current = result.state;
          commands.push(...(result.commands || []));
          fired = fired || result.fired;
        }
        return { fired, state: current, commands };
      }

      let current = state;
      let firedAny = false;
      const commands = [];
      const seen = new Set([this.stateKey(current)]);
      let repeatCount = 0;
      while (true) {
        if (step.condition && this.conditionAccepts(step.condition, current, input)) {
          break;
        }
        const beforeKey = this.stateKey(current);
        let passFired = false;
        for (const child of step.steps) {
          const result = this.applyStep(child, input, current);
          if (result.cancelled) {
            return { fired: true, state: current, cancelled: true };
          }
          current = result.state;
          commands.push(...(result.commands || []));
          passFired = passFired || result.fired;
        }
        if (!passFired) {
          break;
        }
        const key = this.stateKey(current);
        if (key === beforeKey) {
          break;
        }
        firedAny = true;
        if (seen.has(key)) {
          console.warn("until-stable block cycle; ending repeat at current state");
          break;
        }
        seen.add(key);
        repeatCount += 1;
        if (repeatCount >= UNTIL_STABLE_REPEAT_LIMIT) {
          console.warn("until-stable block reached repeat limit; ending repeat at current state");
          break;
        }
      }
      return { fired: firedAny, state: current, commands };
    }

    conditionAccepts(condition, state, input = 0) {
      const patterns = condition?.patterns || [];
      if (condition?.kind === "any_matches") {
        return patterns.some((entry) => this.hasPatternMatch(state, entry.pattern || entry));
      }
      if (condition?.kind === "no_matches") {
        return patterns.every((entry) => !this.hasPatternMatch(state, entry.pattern || entry));
      }
      if (condition?.kind === "any_input_matches") {
        return patterns.some((entry) => entry.input === input && this.hasPatternMatch(state, entry.pattern || entry));
      }
      if (condition?.kind === "no_input_matches") {
        return patterns.every((entry) => entry.input !== input || !this.hasPatternMatch(state, entry.pattern || entry));
      }
      if (condition?.kind === "guard_branches") {
        return (condition.branches || []).some((branch) =>
          branch.every((guard) => this.guardAccepts(guard, input, state))
        );
      }
      return false;
    }

    applyRuleStep(rule, input, state) {
      if (!this.guardsAccept(rule, input, state)) {
        return { fired: false, state };
      }
      if (rule.application === "once") {
        const placement = this.findFirstMatch(state, rule);
        if (!placement) {
          return { fired: false, state };
        }
        if (this.ruleCancels(rule)) {
          return { fired: true, state, cancelled: true };
        }
        return { fired: true, state: this.applyWrites(state, rule, placement), commands: this.ruleCommands(rule) };
      }
      if (rule.application === "once_all") {
        const placements = this.findAllMatches(state, rule);
        if (!placements.length) {
          return { fired: false, state };
        }
        let current = state;
        let fired = false;
        for (const placement of placements) {
          if (!this.placementMatches(current, rule, placement)) {
            continue;
          }
          let next = null;
          try {
            next = this.applyWrites(current, rule, placement);
          } catch (error) {
            if (this.onceAllPatchBecameStale(error)) {
              continue;
            }
            throw error;
          }
          fired = true;
          if (this.ruleCancels(rule)) {
            return { fired: true, state: current, cancelled: true };
          }
          current = next;
        }
        if (!fired) {
          return { fired: false, state };
        }
        return { fired: true, state: current, commands: this.ruleCommands(rule) };
      }
      if (rule.application === "once_per_level") {
        const firedRules = state.levelFiredRules || [];
        if (firedRules.includes(rule.id)) {
          return { fired: false, state };
        }
        const placement = this.findFirstMatch(state, rule);
        if (!placement) {
          return { fired: false, state };
        }
        if (this.ruleCancels(rule)) {
          return { fired: true, state, cancelled: true };
        }
        const next = this.applyWrites(state, rule, placement);
        next.levelFiredRules = [...(next.levelFiredRules || []), rule.id].sort((a, b) => a - b);
        return { fired: true, state: next, commands: this.ruleCommands(rule) };
      }

      let current = state;
      let fired = false;
      const commands = [];
      const seen = new Set([this.stateKey(current)]);
      let repeatCount = 0;
      while (true) {
        const placements = this.findAllMatches(current, rule);
        if (!placements.length) {
          break;
        }
        let advanced = false;
        let shouldStop = false;
        const currentKey = this.stateKey(current);
        for (const placement of placements) {
          if (this.ruleCancels(rule)) {
            return { fired: true, state: current, cancelled: true };
          }
          const next = this.applyWrites(current, rule, placement);
          const key = this.stateKey(next);
          const ruleCommands = this.ruleCommands(rule);
          if (key === currentKey) {
            if (ruleCommands.length) {
              commands.push(...ruleCommands);
              fired = true;
            }
            continue;
          }
          current = next;
          commands.push(...ruleCommands);
          fired = true;
          advanced = true;
          if (seen.has(key)) {
            console.warn(`until-stable cycle in rule ${rule.id}; ending repeat at current state`);
            shouldStop = true;
            break;
          }
          seen.add(key);
          repeatCount += 1;
          if (repeatCount >= UNTIL_STABLE_REPEAT_LIMIT) {
            console.warn(`until-stable repeat limit reached in rule ${rule.id}; ending repeat at current state`);
            shouldStop = true;
            break;
          }
          break;
        }
        if (shouldStop) {
          break;
        }
        if (!advanced) {
          break;
        }
        if (repeatCount >= UNTIL_STABLE_REPEAT_LIMIT) {
          break;
        }
      }
      return { fired, state: current, commands };
    }

    ruleCancels(rule) {
      return (rule.effects || []).some((effect) => effect.kind === "cancel");
    }

    ruleCommands(rule) {
      const emissions = this.engine.ruleEmissions?.[String(rule.id)] || this.engine.ruleEmissions?.[rule.id] || [];
      return [...(rule.effects || []), ...emissions]
        .filter((effect) => effect.kind === "win" || effect.kind === "restart" || effect.kind === "next_level" || effect.kind === "again" || effect.kind === "message" || effect.kind === "play_sfx" || effect.kind === "wait")
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
          if (effect.kind === "again") {
            return { kind: "again" };
          }
          if (effect.kind === "win") {
            return { kind: "win" };
          }
          if (effect.kind === "restart") {
            return { kind: "restart" };
          }
          return { kind: "next_level" };
        });
    }

    guardsAccept(rule, input, state) {
      return rule.guards.every((guard) => this.guardAccepts(guard, input, state));
    }

    guardAccepts(guard, input, state) {
      if (guard.kind === "input_is") {
        return input === guard.input;
      }
      if (guard.kind === "global_compare") {
        return this.compare(state.globals[guard.global] ?? 0, guard.op, guard.value);
      }
      if (guard.kind === "query_compare") {
        return this.compare(this.evalQuery(state, guard.query, input), guard.op, guard.value);
      }
      if (guard.kind === "query_nonzero") {
        return this.evalQuery(state, guard.query, input) !== 0;
      }
      if (guard.kind === "query_value_compare") {
        return this.compare(this.evalQueryKind(state, guard.queryKind, input), guard.op, guard.value);
      }
      if (guard.kind === "query_value_nonzero") {
        return this.evalQueryKind(state, guard.queryKind, input) !== 0;
      }
      return true;
    }

    findFirstMatch(state, rule) {
      if (!rule.pattern.components.length) {
        return { components: [] };
      }
      for (const [x, y] of this.componentCandidateOrigins(state, rule.pattern.components[0])) {
        const first = this.componentPlacementAt(state, rule.pattern.components[0], x, y);
        if (!first) {
          continue;
        }
        const components = [first];
        if (this.completeComponentPlacements(state, rule, 1, components)) {
          return { components };
        }
      }
      return null;
    }

    findAllMatches(state, rule) {
      if (!rule.pattern.components.length) {
        return [{ components: [] }];
      }
      const matches = [];
      for (const [x, y] of this.componentCandidateOrigins(state, rule.pattern.components[0])) {
        const first = this.componentPlacementAt(state, rule.pattern.components[0], x, y);
        if (!first) {
          continue;
        }
        const components = [first];
        this.collectComponentPlacements(state, rule, 1, components, matches);
      }
      return matches;
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

    collectComponentPlacements(state, rule, componentIndex, components, matches) {
      if (componentIndex === rule.pattern.components.length) {
        matches.push({ components: components.map((component) => ({
          originX: component.originX,
          originY: component.originY,
          gaps: [...(component.gaps || [])],
        })) });
        return;
      }
      const component = rule.pattern.components[componentIndex];
      for (const [x, y] of this.componentCandidateOrigins(state, component)) {
        const placement = this.componentPlacementAt(state, component, x, y);
        if (!placement) {
          continue;
        }
        components.push(placement);
        this.collectComponentPlacements(state, rule, componentIndex + 1, components, matches);
        components.pop();
      }
    }

    placementMatches(state, rule, placement) {
      if ((placement.components || []).length !== rule.pattern.components.length) {
        return false;
      }
      return rule.pattern.components.every((component, index) => {
        const placed = placement.components[index];
        return this.componentMatchesWithGaps(
          state,
          component,
          placed.originX,
          placed.originY,
          placed.gaps || [],
        );
      });
    }

    onceAllPatchBecameStale(error) {
      return error?.code === "expected_object" || error?.code === "layer_occupied";
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

    applyWrites(state, rule, placement) {
      const next = this.cloneState(state);
      this.applyMoveWrites(next, rule.writes || [], placement);
      for (const write of rule.writes) {
        if (write.kind === "add") {
          const [x, y] = this.writePosition(placement, write.component, write.offset);
          this.setObject(next, x, y, write.object);
        } else if (write.kind === "remove") {
          const [x, y] = this.writePosition(placement, write.component, write.offset);
          this.removeObject(next, x, y, write.object);
        } else if (write.kind === "move") {
          continue;
        } else if (write.kind === "replace") {
          const [x, y] = this.writePosition(placement, write.component, write.offset);
          this.removeObject(next, x, y, write.remove);
          this.setObject(next, x, y, write.add);
        } else if (write.kind === "set_scratch") {
          const [x, y] = this.writePosition(placement, write.component, write.offset);
          this.setScratch(next, x, y, write.object, write.scratch, Object.hasOwn(write, "value") ? write.value : null);
        } else if (write.kind === "remove_scratch") {
          const [x, y] = this.writePosition(placement, write.component, write.offset);
          this.removeScratch(
            next,
            x,
            y,
            write.object,
            write.scratch,
            Object.hasOwn(write, "value") ? write.value : null,
            write.match,
          );
        }
      }
      for (const effect of rule.effects) {
        if (effect.kind === "update_global") {
          next.globals[effect.global] = this.applyGlobalUpdate(next.globals[effect.global] ?? 0, effect.op, effect.value);
        }
      }
      return next;
    }

    applyWritesOnceAll(state, rule, placements) {
      const next = this.cloneState(state);
      const objectProposals = new Map();
      const deferredWrites = [];
      const deferredEffects = [];

      for (const placement of placements) {
        this.applyWrites(state, rule, placement);
        for (const write of rule.writes) {
          if (write.kind === "add") {
            const [x, y] = this.writePosition(placement, write.component, write.offset);
            const layer = this.objectLayers.get(write.object);
            objectProposals.set(`${x},${y},${layer}`, { x, y, layer, object: write.object });
          } else if (write.kind === "remove") {
            const [x, y] = this.writePosition(placement, write.component, write.offset);
            const layer = this.objectLayers.get(write.object);
            objectProposals.set(`${x},${y},${layer}`, { x, y, layer, object: 0 });
          } else if (write.kind === "move") {
            const [fromX, fromY] = this.writePosition(placement, write.component, write.fromOffset);
            const [toX, toY] = this.writePosition(placement, write.component, write.toOffset);
            const layer = this.objectLayers.get(write.object);
            objectProposals.set(`${fromX},${fromY},${layer}`, { x: fromX, y: fromY, layer, object: 0 });
            objectProposals.set(`${toX},${toY},${layer}`, { x: toX, y: toY, layer, object: write.object });
          } else if (write.kind === "replace") {
            const [x, y] = this.writePosition(placement, write.component, write.offset);
            const removeLayer = this.objectLayers.get(write.remove);
            const addLayer = this.objectLayers.get(write.add);
            objectProposals.set(`${x},${y},${removeLayer}`, { x, y, layer: removeLayer, object: 0 });
            objectProposals.set(`${x},${y},${addLayer}`, { x, y, layer: addLayer, object: write.add });
          } else if (write.kind === "set_scratch" || write.kind === "remove_scratch") {
            const [x, y] = this.writePosition(placement, write.component, write.offset);
            deferredWrites.push({ write, x, y });
          }
        }
        deferredEffects.push(...(rule.effects || []));
      }

      for (const proposal of objectProposals.values()) {
        const index = this.slotIndex(next, proposal.x, proposal.y, proposal.layer);
        if (next.slots[index] !== proposal.object) {
          next.slots[index] = proposal.object;
          if (next.scratch?.[index]?.length) {
            next.scratch[index] = [];
          }
        }
      }
      for (const { write, x, y } of deferredWrites) {
        if (write.kind === "set_scratch") {
          this.setScratch(next, x, y, write.object, write.scratch, Object.hasOwn(write, "value") ? write.value : null);
        } else {
          this.removeScratch(
            next,
            x,
            y,
            write.object,
            write.scratch,
            Object.hasOwn(write, "value") ? write.value : null,
            write.match,
          );
        }
      }
      for (const effect of deferredEffects) {
        if (effect.kind === "update_global") {
          next.globals[effect.global] = this.applyGlobalUpdate(next.globals[effect.global] ?? 0, effect.op, effect.value);
        }
      }
      return next;
    }

    applyMoveWrites(state, writes, placement) {
      const moves = [];
      const sources = new Set();
      const destinations = new Set();

      for (const write of writes) {
        if (write.kind !== "move") {
          continue;
        }
        const layer = this.objectLayers.get(write.object);
        if (layer === undefined) {
          throw this.patchError("unknown_object", `unknown object in move: ${write.object}`);
        }
        const [fromX, fromY] = this.writePosition(placement, write.component, write.fromOffset);
        const [toX, toY] = this.writePosition(placement, write.component, write.toOffset);
        const fromIndex = this.slotIndex(state, fromX, fromY, layer);
        if (state.slots[fromIndex] !== write.object) {
          throw this.patchError("expected_object", `expected object ${write.object} at ${fromX},${fromY}`);
        }
        const destinationKey = `${toX},${toY},${layer}`;
        if (destinations.has(destinationKey)) {
          throw this.patchError("layer_occupied", `move destination already occupied: ${toX},${toY}`);
        }
        sources.add(`${fromX},${fromY},${layer}`);
        destinations.add(destinationKey);
        moves.push({ fromX, fromY, toX, toY, layer, object: write.object });
      }

      for (const move of moves) {
        const toIndex = this.slotIndex(state, move.toX, move.toY, move.layer);
        const existing = state.slots[toIndex];
        if (existing && !sources.has(`${move.toX},${move.toY},${move.layer}`)) {
          throw this.patchError("layer_occupied", `move destination occupied: ${move.toX},${move.toY}`);
        }
      }

      const moved = moves.map((move) => {
        const fromIndex = this.slotIndex(state, move.fromX, move.fromY, move.layer);
        const scratch = state.scratch?.[fromIndex]?.length
          ? state.scratch[fromIndex].map((entry) => ({ ...entry }))
          : [];
        state.slots[fromIndex] = 0;
        if (state.scratch?.[fromIndex]?.length) {
          state.scratch[fromIndex] = [];
        }
        return { ...move, scratch };
      });
      for (const move of moved) {
        const toIndex = this.slotIndex(state, move.toX, move.toY, move.layer);
        state.slots[toIndex] = move.object;
        if (move.scratch.length) {
          state.scratch = state.scratch || [];
          state.scratch[toIndex] = move.scratch;
        } else if (state.scratch?.[toIndex]?.length) {
          state.scratch[toIndex] = [];
        }
      }
    }

    writePosition(placement, componentIndex, offset) {
      const component = placement.components[componentIndex];
      const [dx, dy] = this.resolveOffset(offset, component.gaps);
      return [component.originX + dx, component.originY + dy];
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

    applyGlobalUpdate(current, op, value) {
      if (op === "set") return value;
      if (op === "add") return current + value;
      if (op === "subtract") return current - value;
      if (op === "multiply") return current * value;
      if (op === "divide") return Math.trunc(current / value);
      if (op === "remainder") return current % value;
      return current;
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
      const program = this.engine.levelClearProgram || [];
      const levelProgram = this.currentLevel()?.levelClearProgram || [];
      const displayProgram = this.engine.displayLevelClearProgram || [];
      if ((!program.length && !levelProgram.length && !displayProgram.length) || (!forceClear && !this.isGoalComplete(this.state))) {
        return [];
      }
      const commands = [];
      if (program.length) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionProgramOutcome(program, state, 0, "level_clear");
        this.state = this.cloneState(outcome.state);
        this.capturePersistentVars(this.state);
        if (!outcome.cancelled) {
          commands.push(...this.queueTransitionCommands(null, outcome.commands || []));
        }
      }
      if (levelProgram.length) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        const outcome = this.transitionProgramOutcome(levelProgram, state, 0, "level_clear_local", this.levelIndex);
        this.state = this.cloneState(outcome.state);
        this.capturePersistentVars(this.state);
        if (!outcome.cancelled) {
          commands.push(...this.queueTransitionCommands(null, outcome.commands || []));
        }
      }
      if (displayProgram.length) {
        const state = this.cloneState(this.state);
        this.applyPersistentVars(state);
        this.state = this.materializeDisplayProgram(displayProgram, state, "display_level_clear");
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
        this.soundEvents.push({ kind: "play_sfx", name: effect.name });
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
        } else {
          this.resetPuzzleState(name);
        }
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
      } else if (effect.kind === "clear_history") {
        this.undoStack = [];
        this.redoStack = [];
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
        const value = this.evalEffectString(param.value, bindings);
        if (value === undefined) {
          continue;
        }
        if (param.name === "level") {
          const index = this.levelIndexFromValue(value);
          if (index !== undefined && this.sceneAcceptsLevel(sceneName, index)) {
            this.activateLevel(index, true);
            this.undoStack = [];
            this.redoStack = [];
            levelChanged = true;
          }
          continue;
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
      if (!expr) {
        return undefined;
      }
      if (expr.kind === "bool" || expr.kind === "int") {
        return String(expr.value);
      }
      if (expr.kind === "text") {
        return expr.value || "";
      }
      if (expr.kind === "path") {
        const parts = String(expr.path || "").split(".").filter(Boolean);
        if (parts.length === 1) {
          return bindings[parts[0]]
            ?? (parts[0] === "level" && this.levelIndex !== null && this.levelIndex !== undefined
              ? String(this.levelIndex)
              : undefined)
            ?? this.sceneValueString(parts[0])
            ?? parts[0];
        }
        const levelValue = this.levelPathValue(expr.path);
        if (levelValue !== undefined && levelValue !== null) {
          return String(levelValue);
        }
        return parts.join(".");
      }
      if (expr.kind === "call" && expr.name === "next" && expr.args?.length === 1) {
        const level = this.evalEffectString(expr.args[0], bindings);
        const index = this.levelIndexFromValue(level);
        return index === undefined ? undefined : String(Math.min(index + 1, this.data.levels.length - 1));
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
      const value = this.focusedSceneRuntime()?.values?.[name] ?? this.sessionValues?.[name];
      if (value === undefined || value === null) {
        return undefined;
      }
      return String(value);
    }

    restartLevel() {
      const level = this.currentLevel();
      if (!level) {
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
      const displayLevelStartProgram = this.engine.displayLevelStartProgram || [];
      if (displayLevelStartProgram.length) {
        next = this.materializeDisplayProgram(
          displayLevelStartProgram,
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
      const levelStartProgram = this.engine.levelStartProgram || [];
      if (levelStartProgram.length) {
        const next = this.transitionProgramOutcome(levelStartProgram, outcome.state, 0, "level_start", levelIndex);
        outcome.state = this.cloneState(next.state);
        outcome.cancelled = outcome.cancelled || !!next.cancelled;
        outcome.commands.push(...(next.commands || []));
        ran = true;
      } else if (this.engine.runRulesOnLevelStart) {
        const next = this.transitionProgramOutcome(this.engine.program || [], outcome.state, 0, "run_rules_on_level_start", levelIndex);
        outcome.state = this.cloneState(next.state);
        outcome.cancelled = outcome.cancelled || !!next.cancelled;
        outcome.commands.push(...(next.commands || []));
        ran = true;
      }
      const levelProgram = this.data.levels[levelIndex]?.levelStartProgram || [];
      if (!outcome.cancelled && levelProgram.length) {
        const next = this.transitionProgramOutcome(levelProgram, outcome.state, 0, "level_start_local", levelIndex);
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
      const displayLevelStartProgram = this.engine.displayLevelStartProgram || [];
      if (displayLevelStartProgram.length) {
        next = this.materializeDisplayProgram(
          displayLevelStartProgram,
          next,
          "display_level_start",
          levelIndex,
        );
      }
      return next;
    }

    displayState(state) {
      const displayProgram = this.engine.displayProgram || [];
      if (!displayProgram.length) {
        return state;
      }
      return this.materializeDisplayProgram(displayProgram, state, "display");
    }

    materializeDisplayProgram(program, state, programKey, levelIndex = -1) {
      const base = this.cloneState(state);
      try {
        return this.transitionProgram(program, base, 0, programKey, levelIndex);
      } catch (error) {
        console.warn(`${programKey} projection failed; using source state`, error);
        return this.cloneState(state);
      }
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
      this.redoStack.push(this.cloneState(this.state));
      this.state = previous;
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
      this.undoStack.push(this.cloneState(this.state));
      this.state = next;
      this.applyPersistentVars(this.state);
      this.syncCurrentLevelPuzzles();
    }

    replaceStateIfChanged(next) {
      if (this.levelIndex === null || this.levelIndex === undefined) {
        this.state = this.cloneState(next);
        return;
      }
      this.capturePersistentVars(next);
      this.applyPersistentVars(next);
      if (this.stateKeyIgnoringPersistent(next) === this.stateKeyIgnoringPersistent(this.state)) {
        this.state = next;
        this.syncPersistentVarsToStates();
        return;
      }
      this.undoStack.push(this.cloneState(this.state));
      this.state = next;
      this.redoStack = [];
      this.syncPersistentVarsToStates();
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
      const index = this.firstLevelIndexForScene(name, null);
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
        levelIndex,
      };
      if (this.scenePuzzleInitializer(resolved.sceneName, resolved.puzzleName)?.initializer === "current_level") {
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
      if (trimmed === "back") {
        return { kind: "back" };
      }
      if (trimmed === "clear_history") {
        return { kind: "clear_history" };
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
      const match = trimmed.match(/^(goto|enter|create|reset|delete|show|hide|toggle|focus)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+with\s+(.+))?$/);
      if (!match) {
        return null;
      }
      const [, kind, screen, params] = match;
      const parsedParams = params ? this.parseRuntimeParams(params) : [];
      if (["create", "reset", "delete", "show", "hide", "toggle", "focus"].includes(kind) && parsedParams.length) {
        return null;
      }
      return { kind, screen, params: parsedParams };
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

    writeProgressSave() {
      try {
        window.localStorage?.setItem(this.progressSaveStorageKey(), JSON.stringify(this.progressSaveData()));
      } catch (_error) {
        // Browsers can deny storage for local files, private windows, or quota limits.
      }
    }

    clearProgressSave() {
      this.clearedLevels = new Array(this.data.levels.length).fill(false);
      this.persistentVars = new Array(this.persistentVarIds.length).fill(0);
      this.restoredLevelIndex = null;
      try {
        window.localStorage?.removeItem(this.progressSaveStorageKey());
      } catch (_error) {
        // Ignore storage failures; the in-memory progress was already cleared.
      }
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
        index: levelIndex,
        name: level?.name,
        label: level?.label || level?.name,
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
      if (/^\d+$/.test(String(value))) {
        const index = Number(value);
        return index < this.data.levels.length ? index : undefined;
      }
      const found = this.data.levels.findIndex((level) => level.name === value);
      return found >= 0 ? found : undefined;
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
      return {
        width: state.width,
        height: state.height,
        layerCount: state.layerCount,
        slots: [...state.slots],
        cellScratch: this.cloneScratchStore(state.cellScratch),
        scratch: this.cloneScratchStore(state.scratch),
        globals: [...(state.globals || [])],
        levelFiredRules: [...(state.levelFiredRules || [])],
      };
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
      this.persistentVarIds.forEach((varId, index) => {
        state.globals[varId] = this.persistentVars[index] ?? 0;
      });
    }

    syncPersistentVarsToStates() {
      this.applyPersistentVars(this.state);
      for (const runtime of this.sceneStates.values()) {
        for (const puzzle of Object.values(runtime.puzzles || {})) {
          this.applyPersistentVars(puzzle.state);
          this.applyPersistentVars(puzzle.initialState);
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
        screen: this.data.screen,
        regions: this.data.levels[levelIndex]?.regions || [],
        cells,
      };
    }
  }

  window.PuzzleStandaloneRuntime = PuzzleStandaloneRuntime;
}());
