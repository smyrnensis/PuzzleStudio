import {
  Annotation,
  Compartment,
  EditorSelection,
  EditorState,
  StateEffect,
  StateField,
  Transaction,
} from "@codemirror/state";
import {
  crosshairCursor,
  Decoration,
  drawSelection,
  EditorView,
  keymap,
  lineNumbers,
  rectangularSelection,
  scrollPastEnd,
} from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";

const sourceEditorProgrammatic = Annotation.define();
// CodeMirror owns physical key arbitration, but the source workflow owns the
// meaning and state of completion commands. A command returns false unless that
// workflow explicitly consumes it, preserving the normal CodeMirror key action.
const sourceCompletionKeymap = [
  { key: "Ctrl-Space", run: (view) => dispatchSourceCompletionCommand(view, "show") },
  { key: "ArrowDown", run: (view) => dispatchSourceCompletionCommand(view, "next") },
  { key: "ArrowUp", run: (view) => dispatchSourceCompletionCommand(view, "previous") },
  { key: "Tab", run: (view) => dispatchSourceCompletionCommand(view, "commit") },
  { key: "Enter", run: (view) => dispatchSourceCompletionCommand(view, "commit") },
  { key: "Escape", run: (view) => dispatchSourceCompletionCommand(view, "close") },
];
// PuzzleStudio-specific edit policy stays in editor_source.js. This keymap only
// gives that workflow first refusal before CodeMirror applies its generic edit.
const sourceEditingKeymap = [
  { key: "{", run: (view) => dispatchSourceEditingCommand(view, "open-brace") },
  { key: "}", run: (view) => dispatchSourceEditingCommand(view, "close-brace") },
  { key: "[", run: (view) => dispatchSourceEditingCommand(view, "open-bracket") },
  { key: "Backspace", run: (view) => dispatchSourceEditingCommand(view, "backspace") },
  { key: "Tab", run: (view) => dispatchSourceEditingCommand(view, "tab") },
  { key: "Shift-Tab", run: (view) => dispatchSourceEditingCommand(view, "shift-tab") },
  { key: "Enter", run: (view) => dispatchSourceEditingCommand(view, "enter") },
];
const sourceHighlightClasses = Object.freeze({
  keyword: "syntax-keyword",
  literal: "syntax-literal",
  binding: "syntax-binding",
  effect: "syntax-effect",
  emission: "syntax-emission",
  object: "syntax-object",
  input: "syntax-input",
  state: "syntax-state",
  group: "syntax-group",
  mark: "syntax-mark",
  variant: "syntax-variant",
  condition: "syntax-condition",
  scene: "syntax-scene",
  theme: "syntax-theme",
  asset: "syntax-asset",
  color: "syntax-color",
  number: "syntax-number",
  string: "syntax-string",
  comment: "syntax-comment",
  operator: "syntax-operator",
  arrow: "syntax-arrow",
  "brace-depth-0": "syntax-brace-depth-0",
  "brace-depth-1": "syntax-brace-depth-1",
  "brace-depth-2": "syntax-brace-depth-2",
  "brace-depth-3": "syntax-brace-depth-3",
  "brace-depth-4": "syntax-brace-depth-4",
  "brace-depth-5": "syntax-brace-depth-5",
  "brace-invalid": "syntax-brace-invalid",
  "level-cell": "syntax-level-cell",
  "level-cell-invalid": "syntax-level-cell-invalid",
  "sprite-pixel": "syntax-sprite-pixel",
});
const replaceSourceHighlightRange = StateEffect.define();
const clearSourceHighlightDecorations = StateEffect.define();
const replaceSourceFindDecorations = StateEffect.define();
const sourceHighlightDecorations = StateField.define({
  create() {
    return Decoration.none;
  },
  update(decorations, transaction) {
    let next = decorations.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (effect.is(replaceSourceHighlightRange)) {
        const replacement = effect.value;
        next = Decoration.set(replacement.decorations, true);
      } else if (effect.is(clearSourceHighlightDecorations)) {
        next = Decoration.none;
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});
const sourceFindDecorations = StateField.define({
  create() {
    return Decoration.none;
  },
  update(decorations, transaction) {
    let next = decorations.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (effect.is(replaceSourceFindDecorations)) {
        next = effect.value;
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

function clampOffset(view, value) {
  const offset = Number.isFinite(Number(value)) ? Math.trunc(Number(value)) : 0;
  return Math.max(0, Math.min(view.state.doc.length, offset));
}

function dispatchSourceCompletionCommand(view, command) {
  const event = new CustomEvent("sourcecompletioncommand", {
    bubbles: true,
    cancelable: true,
    detail: { command },
  });
  view.contentDOM.dispatchEvent(event);
  return event.defaultPrevented;
}

function dispatchSourceEditingCommand(view, command) {
  const event = new CustomEvent("sourceeditingcommand", {
    bubbles: true,
    cancelable: true,
    detail: { command },
  });
  view.contentDOM.dispatchEvent(event);
  return event.defaultPrevented;
}

function selectionDirection(selection) {
  return selection.main.anchor > selection.main.head ? "backward" : "forward";
}

function sourceOffsetMaps(source) {
  const utf16ByUtf8 = new Map([[0, 0]]);
  const utf8ByUtf16 = new Map([[0, 0]]);
  let byteOffset = 0;
  for (let utf16Offset = 0; utf16Offset < source.length;) {
    const codePoint = source.codePointAt(utf16Offset);
    const utf16Length = codePoint > 0xffff ? 2 : 1;
    const utf8Length = codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
    byteOffset += utf8Length;
    utf16Offset += utf16Length;
    utf16ByUtf8.set(byteOffset, utf16Offset);
    utf8ByUtf16.set(utf16Offset, byteOffset);
  }
  return { utf16ByUtf8, utf8ByUtf16, byteLength: byteOffset };
}

function highlightDecorations(source, request, payload) {
  if (payload?.version !== 3 || payload?.offsetEncoding !== "utf8" || !Array.isArray(payload?.spans)) {
    throw new Error("Unsupported Rust source highlight span contract.");
  }
  const offsets = sourceOffsetMaps(source);
  const expectedStart = offsets.utf8ByUtf16.get(request.from);
  const expectedEnd = offsets.utf8ByUtf16.get(request.to);
  if (
    payload.sourceLength !== offsets.byteLength
    || payload?.range?.start !== expectedStart
    || payload?.range?.end !== expectedEnd
  ) {
    throw new Error("Rust source highlight range does not match the active CodeMirror viewport.");
  }
  const decorations = [];
  const validatedColors = new Set();
  let previousEnd = 0;
  for (const span of payload.spans) {
    const byteStart = Number(span?.start);
    const byteEnd = Number(span?.end);
    const className = sourceHighlightClasses[String(span?.kind || "")];
    const from = offsets.utf16ByUtf8.get(byteStart);
    const to = offsets.utf16ByUtf8.get(byteEnd);
    if (
      !Number.isInteger(byteStart)
      || !Number.isInteger(byteEnd)
      || byteStart < previousEnd
      || byteStart >= byteEnd
      || byteEnd <= expectedStart
      || byteStart >= expectedEnd
      || from === undefined
      || to === undefined
      || !className
    ) {
      throw new Error("Rust source highlight spans are invalid for the active CodeMirror document.");
    }
    let decorationClass = className;
    const spec = {};
    if (span.transparent === true) {
      if (span.kind !== "sprite-pixel") {
        throw new Error("Only sprite-pixel highlights may be transparent.");
      }
      decorationClass += " is-transparent";
    }
    spec.class = decorationClass;
    if (span.color !== null && span.color !== undefined) {
      const color = String(span.color);
      if (
        (span.kind !== "color" && span.kind !== "sprite-pixel")
        || (!validatedColors.has(color) && !CSS.supports("color", color))
      ) {
        throw new Error("Rust source highlight color is invalid.");
      }
      validatedColors.add(color);
      const property = span.kind === "sprite-pixel"
        ? "--syntax-sprite-pixel-color"
        : "--syntax-color-token";
      spec.attributes = { style: `${property}: ${color}` };
    }
    decorations.push(Decoration.mark(spec).range(from, to));
    previousEnd = byteEnd;
  }
  return decorations;
}

function createState(text, readOnlyCompartment, readOnly, inputListeners) {
  return EditorState.create({
    doc: String(text || ""),
    extensions: [
      lineNumbers(),
      history(),
      sourceHighlightDecorations,
      sourceFindDecorations,
      EditorState.allowMultipleSelections.of(true),
      drawSelection(),
      rectangularSelection(),
      crosshairCursor(),
      EditorView.lineWrapping,
      scrollPastEnd(),
      keymap.of([
        ...sourceCompletionKeymap,
        ...sourceEditingKeymap,
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      EditorView.contentAttributes.of({
        "aria-label": "Puzzle source",
        "aria-multiline": "true",
        autocapitalize: "off",
        autocomplete: "off",
        spellcheck: "false",
      }),
      readOnlyCompartment.of(EditorState.readOnly.of(Boolean(readOnly))),
      EditorView.updateListener.of((update) => {
        if (update.viewportChanged && !update.docChanged) {
          queueMicrotask(() => update.view.contentDOM.dispatchEvent(new Event("sourceviewportchange")));
        }
        if (!update.docChanged || update.transactions.some((transaction) => transaction.annotation(sourceEditorProgrammatic))) {
          return;
        }
        const changes = [];
        update.changes.iterChanges((from, to, _fromAfter, _toAfter, inserted) => {
          changes.push({ from, to, insert: inserted.toString() });
        });
        queueMicrotask(() => {
          const event = new CustomEvent("input", {
            bubbles: true,
            detail: { changes },
          });
          for (const listener of inputListeners) {
            if (typeof listener === "function") {
              listener.call(update.view.contentDOM, event);
            } else {
              listener?.handleEvent?.call(listener, event);
            }
          }
        });
      }),
      EditorView.theme({
        "&": { height: "100%" },
        ".cm-scroller": { overflow: "auto" },
      }),
    ],
  });
}

export function createSourceEditor(parent) {
  if (!(parent instanceof HTMLElement)) {
    throw new Error("CodeMirror source editor requires a mount element.");
  }

  const inputListeners = new Set();
  const readOnlyCompartment = new Compartment();
  const view = new EditorView({
    state: createState("", readOnlyCompartment, false, inputListeners),
    parent,
  });
  const content = view.contentDOM;
  const nativeAddEventListener = content.addEventListener.bind(content);
  const nativeRemoveEventListener = content.removeEventListener.bind(content);
  let readOnly = false;

  content.id = "sourceEditor";
  // This adapter is the migration boundary for existing editor consumers. It
  // can be removed once those consumers depend on SourceEditorPort directly
  // instead of textarea-shaped properties and events.
  content.sourceEditorPort = Object.freeze({
    kind: "codemirror",
    view,
    replaceDocument(text, options = {}) {
      const next = String(text || "");
      if (options.preserveHistory === true && next === view.state.doc.toString()) {
        return;
      }
      const selection = options.selection || { anchor: 0, head: 0 };
      view.setState(createState(next, readOnlyCompartment, readOnly, inputListeners));
      const anchor = clampOffset(view, selection.anchor);
      const head = clampOffset(view, selection.head ?? selection.anchor);
      view.dispatch({ selection: EditorSelection.single(anchor, head) });
      content.dispatchEvent(new CustomEvent("sourceanalysisreset", { detail: { source: next } }));
    },
    highlightViewportRange(overscanLines = 80) {
      const startLine = view.state.doc.lineAt(view.viewport.from).number;
      const endLine = view.state.doc.lineAt(view.viewport.to).number;
      const from = view.state.doc.line(Math.max(1, startLine - overscanLines)).from;
      const to = view.state.doc.line(Math.min(view.state.doc.lines, endLine + overscanLines)).to;
      return { from, to };
    },
    applyHighlightRange(source, request, payload) {
      const expected = String(source || "");
      if (expected !== view.state.doc.toString()) {
        throw new Error("Cannot apply stale source highlighting to CodeMirror.");
      }
      view.dispatch({
        effects: replaceSourceHighlightRange.of({
          from: request.from,
          to: request.to,
          decorations: highlightDecorations(expected, request, payload),
        }),
      });
    },
    clearHighlights() {
      view.dispatch({ effects: clearSourceHighlightDecorations.of(null) });
    },
    applyFindMatches(source, matches, selectedIndex) {
      const expected = String(source || "");
      if (expected !== view.state.doc.toString()) {
        throw new Error("Cannot apply stale source find matches to CodeMirror.");
      }
      const decorations = (Array.isArray(matches) ? matches : []).slice(0, 600).map((match, index) => {
        const from = clampOffset(view, match?.start);
        const to = clampOffset(view, match?.end);
        if (from >= to) {
          return null;
        }
        return Decoration.mark({
          class: `cm-source-find-match${index === selectedIndex ? " is-current" : ""}`,
        }).range(from, to);
      }).filter(Boolean);
      view.dispatch({
        effects: replaceSourceFindDecorations.of(Decoration.set(decorations, true)),
      });
    },
    scrollIntoView(offset, alignment = "nearest") {
      view.dispatch({ effects: EditorView.scrollIntoView(clampOffset(view, offset), { y: alignment, x: "nearest" }) });
    },
    coordsAtOffset(offset) {
      return view.coordsAtPos(clampOffset(view, offset));
    },
    offsetAtCoords(x, y) {
      return view.posAtCoords({ x: Number(x), y: Number(y) });
    },
    scrollTop(value) {
      if (value !== undefined) {
        view.scrollDOM.scrollTop = Math.max(0, Number(value) || 0);
      }
      return view.scrollDOM.scrollTop;
    },
    scrollLeft(value) {
      if (value !== undefined) {
        view.scrollDOM.scrollLeft = Math.max(0, Number(value) || 0);
      }
      return view.scrollDOM.scrollLeft;
    },
    viewportSize() {
      return {
        width: view.scrollDOM.clientWidth,
        height: view.scrollDOM.clientHeight,
      };
    },
  });

  Object.defineProperties(content, {
    value: {
      configurable: true,
      get() {
        return view.state.doc.toString();
      },
      set(value) {
        content.sourceEditorPort.replaceDocument(String(value || ""));
      },
    },
    selectionStart: {
      configurable: true,
      get() {
        return view.state.selection.main.from;
      },
    },
    selectionEnd: {
      configurable: true,
      get() {
        return view.state.selection.main.to;
      },
    },
    selectionDirection: {
      configurable: true,
      get() {
        return selectionDirection(view.state.selection);
      },
    },
    readOnly: {
      configurable: true,
      get() {
        return readOnly;
      },
      set(value) {
        readOnly = Boolean(value);
        view.dispatch({ effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(readOnly)) });
      },
    },
  });

  content.setSelectionRange = (start, end = start, direction = "none") => {
    const from = clampOffset(view, start);
    const to = clampOffset(view, end);
    const anchor = direction === "backward" ? to : from;
    const head = direction === "backward" ? from : to;
    view.dispatch({ selection: EditorSelection.single(anchor, head) });
  };
  content.setRangeText = (replacement, start, end, selectionMode = "preserve") => {
    const from = clampOffset(view, start);
    const to = Math.max(from, clampOffset(view, end));
    const insert = String(replacement || "");
    const previous = view.state.selection.main;
    let selection = null;
    if (selectionMode === "select") {
      selection = EditorSelection.single(from, from + insert.length);
    } else if (selectionMode === "start") {
      selection = EditorSelection.cursor(from);
    } else if (selectionMode === "end") {
      selection = EditorSelection.cursor(from + insert.length);
    } else {
      const delta = insert.length - (to - from);
      const map = (offset) => offset <= from ? offset : offset >= to ? offset + delta : from + insert.length;
      selection = EditorSelection.single(map(previous.anchor), map(previous.head));
    }
    view.dispatch({
      changes: { from, to, insert },
      selection,
      annotations: [sourceEditorProgrammatic.of(true), Transaction.userEvent.of("input")],
    });
    content.dispatchEvent(new CustomEvent("sourceanalysisedit", {
      detail: {
        changes: [{ from, to, insert }],
        source: view.state.doc.toString(),
      },
    }));
  };
  content.addEventListener = (type, listener, options) => {
    if (type === "input") {
      inputListeners.add(listener);
      return;
    }
    nativeAddEventListener(type, listener, options);
  };
  content.removeEventListener = (type, listener, options) => {
    if (type === "input") {
      inputListeners.delete(listener);
      return;
    }
    nativeRemoveEventListener(type, listener, options);
  };

  view.scrollDOM.addEventListener("scroll", () => {
    parent.parentElement?.dispatchEvent(new Event("scroll"));
  }, { passive: true });

  return content;
}
