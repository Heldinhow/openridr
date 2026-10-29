import { useEffect, useRef, useState } from "react";
import { basicSetup } from "codemirror";
import { indentWithTab } from "@codemirror/commands";
import { EditorState, type Text } from "@codemirror/state";
import { EditorView, GutterMarker, gutter, keymap } from "@codemirror/view";
import { Compartment, RangeSet } from "@codemirror/state";
import { HighlightStyle, indentUnit, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import { csharp } from "@codemirror/legacy-modes/mode/clike";
import type { EditorSettings } from "./actions/keymap";
import {
  autocompletion,
  insertCompletionText,
  pickedCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import {
  call,
  messageOf,
  type AnalysisResponse,
  type DefinitionLocation,
} from "./host/client";
import type { GutterAction } from "./codeNavigation";

const theme = EditorView.theme(
  {
    "&": { height: "100%", backgroundColor: "var(--bg)", color: "var(--fg)" },
    ".cm-scroller": {
      lineHeight: "1.65",
    },    ".cm-content": { caretColor: "var(--accent)" },
    ".cm-gutters": {
      backgroundColor: "var(--panel)",
      color: "var(--muted)",
      borderRight: "1px solid var(--line)",
    },
    ".cm-activeLine": { backgroundColor: "#171b21" },
    ".cm-activeLineGutter": { backgroundColor: "#1b2027" },
    "&.cm-focused": { outline: "none" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "#33404f !important" },
    ".cm-cursor": { borderLeftColor: "var(--accent)" },
  },
  { dark: true },
);

const syntaxColors = HighlightStyle.define([
  { tag: tags.keyword, color: "#c6a9ed" },
  { tag: [tags.typeName, tags.className], color: "#8ed4ca" },
  { tag: tags.string, color: "#a7cf9b" },
  { tag: [tags.number, tags.bool, tags.null], color: "#f0b789" },
  { tag: tags.comment, color: "#929eac", fontStyle: "italic" },
  { tag: tags.function(tags.variableName), color: "#f3cf7e" },
]);

type Target = { path: string; line: number; character: number; nonce: number };

type Props = {
  value: string;
  path: string;
  platform: string;
  revision: number;
  editor: EditorSettings;
  /** 1-based breakpoint lines for this file. */
  breakpoints?: number[];
  /** 1-based line of the current pause, when this file is the paused frame. */
  currentLine?: number | null;
  /** Run/test markers derived from document symbols; one per unambiguous line. */
  gutterActions?: GutterAction[];
  /** Incremented by the Rename action/menu; the inline field opens at the selection. */
  renameRequest?: number;
  /** Incremented by the Go to Declaration action/menu. */
  definitionRequest?: number;
  onToggleBreakpoint?: (line: number) => void;
  onChange: (value: string) => void;
  onSave: () => void;
  onDefinition: (location: DefinitionLocation) => void;
  onRename: (position: { line: number; character: number }) => void;
  onRenameSubmit: (
    position: { line: number; character: number },
    newName: string,
  ) => Promise<{ ok: boolean; error?: string }>;
  onRenameCancel: () => void;
  onCodeActions: (position: { line: number; character: number }) => void;
  onContextMenu: (
    position: { line: number; character: number },
    at: { x: number; y: number },
  ) => void;
  onGutterAction: (action: GutterAction) => void;
  onCursor: (position: { line: number; character: number }) => void;
  target?: Target | null;
};

type Status = { state: string; text: string };

function offsetAt(doc: Text, position: { line: number; character: number }): number {
  const lineNumber = Math.min(Math.max(position.line + 1, 1), doc.lines);
  const line = doc.line(lineNumber);
  return Math.min(line.from + Math.max(position.character, 0), line.to);
}

function failureText(kind: string, response: AnalysisResponse): string {
  return `${kind} ${response.state}: ${response.message ?? "csharp-ls did not answer."}`;
}

/** Editor font and indentation come from Settings and are reconfigured through compartments,
 * so changing them while a file is open applies without remounting the view. */
function editorTheme(settings: EditorSettings) {
  return EditorView.theme({
    ".cm-scroller": {
      fontFamily: `"${settings.fontFamily}", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`,
      fontSize: `${settings.fontSize}px`,
    },
  });
}

function editorIndent(settings: EditorSettings) {
  return [
    EditorState.tabSize.of(settings.indentSize),
    indentUnit.of(settings.useTabs ? "\t" : " ".repeat(settings.indentSize)),
  ];
}

class DebugMarker extends GutterMarker {
  constructor(
    private readonly text: string,
    private readonly kind: string,
  ) {
    super();
  }

  toDOM() {
    const span = document.createElement("span");
    span.className = `cm-debug-marker cm-debug-${this.kind}`;
    span.textContent = this.text;
    span.setAttribute("aria-hidden", "true");
    return span;
  }
}

function makeDebugGutter(
  breakpointsRef: { current: number[] },
  currentLineRef: { current: number | null },
  onToggleBreakpointRef: { current: ((line: number) => void) | undefined },
) {
  return gutter({
    class: "cm-debug-gutter",
    markers: (view) => {
      const markers = [];
      const lines = new Set<number>(breakpointsRef.current);
      const current = currentLineRef.current;
      if (current !== null) lines.add(current);
      for (const line of lines) {
        if (line < 1 || line > view.state.doc.lines) continue;
        const isBreakpoint = breakpointsRef.current.includes(line);
        const isCurrent = current === line;
        const text = `${isBreakpoint ? "●" : " "}${isCurrent ? "▶" : " "}`;
        markers.push(
          new DebugMarker(
            text,
            isBreakpoint && isCurrent
              ? "both"
              : isBreakpoint
                ? "breakpoint"
                : "current",
          ).range(view.state.doc.line(line).from),
        );
      }
      return RangeSet.of(markers, true);
    },
    domEventHandlers: {
      mousedown(view, line) {
        onToggleBreakpointRef.current?.(view.state.doc.lineAt(line.from).number);
        return true;
      },
    },
  });
}

class ActionMarker extends GutterMarker {
  constructor(
    private readonly action: GutterAction,
    private readonly run: () => void,
  ) {
    super();
  }

  toDOM() {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `cm-gutter-action cm-gutter-${this.action.kind}${
      this.action.disabledReason ? " is-unavailable" : ""
    }`;
    const title = this.action.disabledReason
      ? `${this.action.label} unavailable: ${this.action.disabledReason}`
      : this.action.label;
    button.title = title;
    button.setAttribute("aria-label", title);
    button.setAttribute("data-testid", `gutter-${this.action.kind}-${this.action.line}`);
    if (this.action.disabledReason) button.setAttribute("aria-disabled", "true");
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 16 16");
    svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    if (this.action.kind === "run") {
      path.setAttribute("d", "M4 2.5l9 5.5-9 5.5z");
      path.setAttribute("fill", "currentColor");
    } else {
      path.setAttribute("d", "M3 8.5l3 3L13 4.5");
      path.setAttribute("fill", "none");
      path.setAttribute("stroke", "currentColor");
      path.setAttribute("stroke-width", "1.8");
      path.setAttribute("stroke-linecap", "round");
      path.setAttribute("stroke-linejoin", "round");
    }
    svg.append(path);
    button.append(svg);
    button.addEventListener("mousedown", (event) => {
      // Do not move the CodeMirror selection or open a native drag.
      event.preventDefault();
      event.stopPropagation();
    });
    button.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.run();
    });
    return button;
  }
}

function makeActionGutter(
  actionsRef: { current: GutterAction[] },
  onActionRef: { current: ((action: GutterAction) => void) | undefined },
) {
  return gutter({
    class: "cm-action-gutter",
    markers: (view) => {
      const markers = [];
      for (const action of actionsRef.current) {
        if (action.line < 1 || action.line > view.state.doc.lines) continue;
        markers.push(
          new ActionMarker(action, () => onActionRef.current?.(action)).range(
            view.state.doc.line(action.line).from,
          ),
        );
      }
      return RangeSet.of(markers, true);
    },
  });
}

type InlineRename = {
  from: number;
  to: number;
  left: number;
  top: number;
  width: number;
  line: number;
  character: number;
  source: string;
  value: string;
  busy: boolean;
  error: string | null;
};

export function CodeEditor({
  value,
  path,
  platform,
  revision,
  editor,
  breakpoints = [],
  currentLine = null,
  gutterActions = [],
  renameRequest = 0,
  definitionRequest = 0,
  onToggleBreakpoint,
  onChange,
  onSave,
  onDefinition,
  onRename,
  onRenameSubmit,
  onRenameCancel,
  onCodeActions,
  onContextMenu,
  onGutterAction,
  onCursor,
  target,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);
  const onDefinitionRef = useRef(onDefinition);
  const onRenameRef = useRef(onRename);
  const onRenameSubmitRef = useRef(onRenameSubmit);
  const onRenameCancelRef = useRef(onRenameCancel);
  const onCodeActionsRef = useRef(onCodeActions);
  const onContextMenuRef = useRef(onContextMenu);
  const onGutterActionRef = useRef(onGutterAction);
  const onCursorRef = useRef(onCursor);
  const breakpointsRef = useRef<number[]>(breakpoints);
  const currentLineRef = useRef<number | null>(currentLine);
  const gutterActionsRef = useRef<GutterAction[]>(gutterActions);
  const onToggleBreakpointRef = useRef(onToggleBreakpoint);
  const gutterCompartment = useRef(new Compartment());
  const actionGutterCompartment = useRef(new Compartment());
  const fontCompartment = useRef(new Compartment());
  const indentCompartment = useRef(new Compartment());
  breakpointsRef.current = breakpoints;
  currentLineRef.current = currentLine;
  gutterActionsRef.current = gutterActions;
  onToggleBreakpointRef.current = onToggleBreakpoint;
  const pathRef = useRef(path);
  const revisionRef = useRef(revision);
  onChangeRef.current = onChange;
  onSaveRef.current = onSave;
  onDefinitionRef.current = onDefinition;
  onRenameRef.current = onRename;
  onRenameSubmitRef.current = onRenameSubmit;
  onRenameCancelRef.current = onRenameCancel;
  onCodeActionsRef.current = onCodeActions;
  onContextMenuRef.current = onContextMenu;
  onGutterActionRef.current = onGutterAction;
  onCursorRef.current = onCursor;
  pathRef.current = path;
  revisionRef.current = revision;
  const applyingExternal = useRef(false);
  const completionSeq = useRef(0);
  const definitionSeq = useRef(0);
  const inlineSeq = useRef(0);
  const inlineRef = useRef<InlineRename | null>(null);
  const handledRename = useRef(renameRequest);
  const handledDefinition = useRef(definitionRequest);
  const inputRef = useRef<HTMLInputElement>(null);
  const [inline, setInline] = useState<InlineRename | null>(null);
  inlineRef.current = inline;
  const actionKey = gutterActions
    .map((action) => `${action.line}:${action.kind}:${action.disabledReason ?? ""}`)
    .join("|");
  const [status, setStatus] = useState<Status>({
    state: "idle",
    text: `Ctrl+Space: completion · ${platform === "macos" ? "Cmd+B" : "Ctrl+B"}: go to definition · Shift+F6: rename · Shift+F12: find usages · ${platform === "macos" ? "Option" : "Alt"}+Enter: code actions`,
  });

  function publishCursor(instance: EditorView) {
    const position = instance.state.selection.main.head;
    const line = instance.state.doc.lineAt(position);
    if (host.current) {
      host.current.dataset.cursor = `${line.number - 1}:${position - line.from}`;
    }
    onCursorRef.current({ line: line.number - 1, character: position - line.from });
  }

  async function completionSource(context: CompletionContext): Promise<CompletionResult | null> {
    const instance = view.current;
    if (!instance) return null;
    const doc = instance.state.doc;
    const position = context.pos;
    const line = doc.lineAt(position);
    const requestedRevision = revisionRef.current;
    const sequence = ++completionSeq.current;
    setStatus({ state: "loading", text: "Completion: waiting for csharp-ls…" });

    let response: AnalysisResponse;
    try {
      response = await call<AnalysisResponse>("lsp.completion", {
        path: pathRef.current,
        line: line.number - 1,
        character: position - line.from,
        revision: requestedRevision,
      });
    } catch (error) {
      if (completionSeq.current === sequence) {
        setStatus({
          state: "unavailable",
          text: `Completion unavailable: ${messageOf(error)}`,
        });
      }
      return null;
    }
    if (completionSeq.current !== sequence) return null;
    if (instance.state.doc !== doc || revisionRef.current !== requestedRevision) {
      setStatus({
        state: "stale",
        text: "Completion discarded: the buffer changed while csharp-ls was answering.",
      });
      return null;
    }
    if (response.state !== "ok") {
      setStatus({ state: response.state, text: failureText("Completion", response) });
      return null;
    }
    if (response.items.length === 0) {
      setStatus({
        state: "ok",
        text: `Completion: csharp-ls returned no items · revision ${requestedRevision}`,
      });
      return null;
    }

    const from = response.replaceRange ? offsetAt(doc, response.replaceRange.start) : position;
    const to = response.replaceRange ? offsetAt(doc, response.replaceRange.end) : position;
    const prefix = doc.sliceString(0, from);
    const suffix = doc.sliceString(to);
    const options: Completion[] = response.items.map((item) => {
      const insert = item.textEdit?.newText ?? item.insertText ?? item.label;
      const completion: Completion = {
        label: item.label,
        detail: item.detail ?? undefined,
        type: item.kind ?? undefined,
        info: item.documentation ?? undefined,
        apply: (instance, picked, start, end) => {
          const current = instance.state.doc;
          // Only the text between the requested range and the cursor's filter prefix may
          // differ; anything outside it is a concurrent edit and discards the completion.
          if (current.sliceString(0, start) !== prefix || current.sliceString(end) !== suffix) {
            setStatus({
              state: "stale",
              text: "Completion discarded: the buffer changed outside the completion range.",
            });
            return;
          }
          instance.dispatch({
            ...insertCompletionText(instance.state, insert, start, end),
            annotations: pickedCompletion.of(picked),
          });
        },
      };
      return completion;
    });
    const extras = [
      `session ${response.session}`,
      `revision ${requestedRevision}`,
      response.coverage === "partial" ? "partial coverage" : null,
      response.truncated
        ? `${Math.max(response.totalItems - response.itemCount, 0)} not shown`
        : null,
    ].filter(Boolean);
    setStatus({
      state: "ok",
      text: `Completion: ${response.itemCount} item${response.itemCount === 1 ? "" : "s"} from csharp-ls · ${extras.join(" · ")}`,
    });
    return { from, to, options, validFor: /^[\w$]*$/u };
  }

  async function requestDefinition() {
    const instance = view.current;
    if (!instance) return;
    const doc = instance.state.doc;
    const head = instance.state.selection.main.head;
    const line = doc.lineAt(head);
    const requestedRevision = revisionRef.current;
    const sequence = ++definitionSeq.current;
    setStatus({ state: "loading", text: "Definition: waiting for csharp-ls…" });

    let response: AnalysisResponse;
    try {
      response = await call<AnalysisResponse>("lsp.definition", {
        path: pathRef.current,
        line: line.number - 1,
        character: head - line.from,
        revision: requestedRevision,
      });
    } catch (error) {
      if (definitionSeq.current === sequence) {
        setStatus({
          state: "unavailable",
          text: `Definition unavailable: ${messageOf(error)}`,
        });
      }
      return;
    }
    if (definitionSeq.current !== sequence) return;
    if (
      instance.state.doc !== doc ||
      instance.state.selection.main.head !== head ||
      revisionRef.current !== requestedRevision
    ) {
      setStatus({
        state: "stale",
        text: "Definition discarded: the buffer or selection changed while csharp-ls was answering.",
      });
      return;
    }
    if (response.state !== "ok") {
      setStatus({ state: response.state, text: failureText("Definition", response) });
      return;
    }
    const location = response.locations[0];
    if (!location) {
      setStatus({
        state: "ok",
        text: `Definition: csharp-ls returned no location${response.rejectedTargets > 0 ? ` (${response.rejectedTargets} rejected)` : ""}.`,
      });
      return;
    }
    setStatus({
      state: "ok",
      text: `Definition: ${location.path}:${location.line + 1}:${location.character + 1} · session ${response.session}${response.message ? ` · ${response.message}` : ""}`,
    });
    onDefinitionRef.current(location);
  }

  function cancelInlineRename(notifyParent: boolean) {
    inlineSeq.current += 1;
    setInline(null);
    if (notifyParent) onRenameCancelRef.current();
  }

  function startInlineRename(instance: EditorView) {
    const head = instance.state.selection.main.head;
    const word = instance.state.wordAt(head);
    if (!word || word.from === word.to) {
      setStatus({
        state: "unavailable",
        text: "Rename needs an identifier under the cursor; nothing was requested.",
      });
      return;
    }
    const doc = instance.state.doc;
    const line = doc.lineAt(word.from);
    const start = instance.coordsAtPos(word.from);
    const end = instance.coordsAtPos(word.to);
    const shell = host.current?.getBoundingClientRect();
    if (!start || !shell) return;
    inlineSeq.current += 1;
    const left = Math.max(8, Math.min(start.left - shell.left, shell.width - 280));
    setInline({
      from: word.from,
      to: word.to,
      left,
      top: start.bottom - shell.top + 4,
      width: Math.max(240, (end?.right ?? start.right) - start.left + 24),
      line: line.number - 1,
      character: word.from - line.from,
      source: doc.sliceString(word.from, word.to),
      value: doc.sliceString(word.from, word.to),
      busy: false,
      error: null,
    });
    setStatus({
      state: "ok",
      text: `Rename ${doc.sliceString(word.from, word.to)} at ${line.number}:${word.from - line.from + 1}; Enter previews, Esc cancels.`,
    });
    window.setTimeout(() => inputRef.current?.focus(), 0);
  }

  async function submitInlineRename() {
    const current = inlineRef.current;
    const instance = view.current;
    if (!current || current.busy) return;
    if (!instance || instance.state.doc.sliceString(current.from, current.to) !== current.source) {
      cancelInlineRename(true);
      setStatus({
        state: "stale",
        text: "Rename discarded: the buffer changed while the field was open; nothing was requested.",
      });
      return;
    }
    const name = current.value.trim();
    if (!name) {
      setInline({ ...current, error: "Enter the new name." });
      return;
    }
    const sequence = ++inlineSeq.current;
    setInline({ ...current, busy: true, error: null });
    let result: { ok: boolean; error?: string };
    try {
      result = await onRenameSubmitRef.current(
        { line: current.line, character: current.character },
        name,
      );
    } catch (error) {
      result = { ok: false, error: messageOf(error) };
    }
    if (sequence !== inlineSeq.current) return;
    if (result.ok) {
      setInline(null);
      return;
    }
    setInline((live) =>
      live ? { ...live, busy: false, error: result.error ?? "Rename failed." } : live,
    );
  }

  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        autocompletion({ override: [completionSource], activateOnTyping: false }),
        StreamLanguage.define(csharp),
        theme,
        fontCompartment.current.of(editorTheme(editor)),
        indentCompartment.current.of(editorIndent(editor)),
        syntaxHighlighting(syntaxColors),
        actionGutterCompartment.current.of(
          makeActionGutter(gutterActionsRef, onGutterActionRef),
        ),
        gutterCompartment.current.of(
          makeDebugGutter(
            breakpointsRef,
            currentLineRef,
            onToggleBreakpointRef,
          ),
        ),
        EditorView.contentAttributes.of({
          "aria-label": "Editor",
          "aria-multiline": "true",
          "data-testid": "editor",
          spellcheck: "false",
        }),
        keymap.of([
          indentWithTab,
          {
            key: "Mod-s",
            preventDefault: true,
            run: () => {
              onSaveRef.current();
              return true;
            },
          },
          {
            // Rider/IntelliJ keymap: go to declaration. The host gates trust and session.
            key: "Mod-b",
            preventDefault: true,
            run: () => {
              void requestDefinition();
              return true;
            },
          },
          {
            // Rider/IntelliJ keymap: rename. Opening the prompt never changes the buffer;
            // the host gates trust, capability and the open document.
            key: "Shift-F6",
            preventDefault: true,
            run: (instance) => {
              const head = instance.state.selection.main.head;
              const line = instance.state.doc.lineAt(head);
              onRenameRef.current({
                line: line.number - 1,
                character: head - line.from,
              });
              return true;
            },
          },
          {
            // Option+Enter on macOS, Alt+Enter elsewhere. The host gates trust,
            // capability, document version and position; an empty answer stays honest.
            key: "Alt-Enter",
            preventDefault: true,
            run: (instance) => {
              const head = instance.state.selection.main.head;
              const line = instance.state.doc.lineAt(head);
              onCodeActionsRef.current({
                line: line.number - 1,
                character: head - line.from,
              });
              return true;
            },
          },
        ]),
        EditorView.domEventHandlers({
          contextmenu(event, instance) {
            const position =
              instance.posAtCoords({ x: event.clientX, y: event.clientY }) ??
              instance.state.selection.main.head;
            const line = instance.state.doc.lineAt(position);
            instance.dispatch({ selection: { anchor: position } });
            instance.focus();
            event.preventDefault();
            onContextMenuRef.current(
              { line: line.number - 1, character: position - line.from },
              { x: event.clientX, y: event.clientY },
            );
            return true;
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !applyingExternal.current) {
            onChangeRef.current(update.state.doc.toString());
          }
          if (inlineRef.current && (update.docChanged || update.selectionSet)) {
            const wasBusy = inlineRef.current.busy;
            inlineSeq.current += 1;
            setInline(null);
            if (wasBusy) onRenameCancelRef.current();
            setStatus({
              state: "stale",
              text: "Rename field discarded: the buffer or selection changed.",
            });
          }
          if (update.selectionSet || update.docChanged) {
            publishCursor(update.view);
          }
        }),
      ],
    });
    const instance = new EditorView({ state, parent: host.current });
    view.current = instance;
    publishCursor(instance);
    return () => {
      instance.destroy();
      view.current = null;
    };
    // The editor is remounted with a key per file; external values are applied below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    instance.dispatch({
      effects: [
        fontCompartment.current.reconfigure(editorTheme(editor)),
        indentCompartment.current.reconfigure(editorIndent(editor)),
      ],
    });
  }, [editor.fontFamily, editor.fontSize, editor.indentSize, editor.useTabs]);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    instance.dispatch({
      effects: gutterCompartment.current.reconfigure(
        makeDebugGutter(breakpointsRef, currentLineRef, onToggleBreakpointRef),
      ),
    });
  }, [breakpoints.join(","), currentLine]);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    instance.dispatch({
      effects: actionGutterCompartment.current.reconfigure(
        makeActionGutter(gutterActionsRef, onGutterActionRef),
      ),
    });
  }, [actionKey]);

  useEffect(() => {
    if (renameRequest === handledRename.current) return;
    handledRename.current = renameRequest;
    const instance = view.current;
    if (instance) startInlineRename(instance);
    // The nonce is the only trigger; the selection is read when the action runs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [renameRequest]);

  useEffect(() => {
    if (definitionRequest === handledDefinition.current) return;
    handledDefinition.current = definitionRequest;
    void requestDefinition();
    // The nonce is the only trigger; the cursor is read when the action runs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [definitionRequest]);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    const current = instance.state.doc.toString();
    if (current !== value) {
      // Reloads set content in React state; the dispatch must not count as a user edit,
      // otherwise the reloaded buffer would immediately be dirty at a higher revision.
      applyingExternal.current = true;
      instance.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
      applyingExternal.current = false;
    }
  }, [value]);

  useEffect(() => {
    const instance = view.current;
    if (!instance || !target) return;
    const lineNumber = Math.min(Math.max(target.line + 1, 1), instance.state.doc.lines);
    const line = instance.state.doc.line(lineNumber);
    const position = Math.min(line.from + Math.max(target.character, 0), line.to);
    instance.dispatch({
      selection: { anchor: position },
      effects: EditorView.scrollIntoView(position, { y: "center" }),
    });
    instance.focus();
    publishCursor(instance);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target]);

  return (
    <div className="editor-stack">
      <div className="code-editor" data-testid="editor-shell" ref={host} />
      {inline && (
        <form
          className="inline-rename"
          data-testid="inline-rename"
          style={{ left: inline.left, top: inline.top, width: inline.width }}
          onSubmit={(event) => {
            event.preventDefault();
            void submitInlineRename();
          }}
          onBlur={(event) => {
            // The submit button is disabled while waiting, so it stops holding focus and the
            // form blurs. Do not cancel the request the user just submitted.
            if (inline.busy) return;
            const next = event.relatedTarget as Node | null;
            if (next && event.currentTarget.contains(next)) return;
            cancelInlineRename(true);
          }}
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            event.preventDefault();
            event.stopPropagation();
            cancelInlineRename(true);
            view.current?.focus();
          }}
        >
          <span className="inline-rename-source" data-testid="rename-source">
            {path} · {inline.line + 1}:{inline.character + 1}
          </span>
          <div className="inline-rename-row">
            <input
              ref={inputRef}
              data-testid="rename-input"
              autoFocus
              value={inline.value}
              aria-label={`New name for ${inline.source}`}
              aria-describedby={inline.error ? "rename-error" : undefined}
              readOnly={inline.busy}
              onChange={(event) =>
                setInline((live) => (live ? { ...live, value: event.target.value } : live))
              }
            />
            <button type="submit" data-testid="rename-submit" disabled={inline.busy}>
              {inline.busy ? "Waiting for csharp-ls…" : "Preview rename"}
            </button>
            <button
              type="button"
              data-testid="rename-cancel"
              onClick={() => {
                cancelInlineRename(true);
                view.current?.focus();
              }}
            >
              Cancel
            </button>
          </div>
          {inline.error && (
            <p className="error" role="alert" id="rename-error" data-testid="rename-error">
              {inline.error}
            </p>
          )}
        </form>
      )}
      <div
        className="editor-lsp-status"
        data-testid="editor-lsp-status"
        data-state={status.state}
        role="status"
      >
        {status.text}
      </div>
    </div>
  );
}
