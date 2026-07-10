import { Annotation, Compartment, EditorSelection, EditorState, Transaction } from "@codemirror/state";
import {
  EditorView,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";

const sourceEditorProgrammatic = Annotation.define();

function clampOffset(view, value) {
  const offset = Number.isFinite(Number(value)) ? Math.trunc(Number(value)) : 0;
  return Math.max(0, Math.min(view.state.doc.length, offset));
}

function selectionDirection(selection) {
  return selection.main.anchor > selection.main.head ? "backward" : "forward";
}

function createState(text, readOnlyCompartment, readOnly, inputListeners) {
  return EditorState.create({
    doc: String(text || ""),
    extensions: [
      lineNumbers(),
      history(),
      EditorView.lineWrapping,
      keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap]),
      EditorView.contentAttributes.of({
        "aria-label": "Puzzle source",
        "aria-multiline": "true",
        autocapitalize: "off",
        autocomplete: "off",
        spellcheck: "false",
      }),
      readOnlyCompartment.of(EditorState.readOnly.of(Boolean(readOnly))),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged || update.transactions.some((transaction) => transaction.annotation(sourceEditorProgrammatic))) {
          return;
        }
        queueMicrotask(() => {
          const event = new Event("input", { bubbles: true });
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
    },
    scrollIntoView(offset) {
      view.dispatch({ effects: EditorView.scrollIntoView(clampOffset(view, offset), { y: "nearest", x: "nearest" }) });
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
