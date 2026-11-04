import { useEffect, useRef, useState } from "react";
import { basicSetup } from "codemirror";
import { copyLineDown, deleteLine, indentWithTab } from "@codemirror/commands";
import { Compartment, EditorState, Prec, RangeSet, Transaction, type Text } from "@codemirror/state";
import { Decoration, EditorView, GutterMarker, WidgetType, gutter, hoverTooltip, keymap } from "@codemirror/view";
import { HighlightStyle, indentUnit, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import { gotoLine } from "@codemirror/search";
import { csharp } from "@codemirror/legacy-modes/mode/clike";
import type { EditorSettings, ProfileId } from "./actions/keymap";
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
import type { EditorDiagnostic } from "./diagnosticMarks";

const theme = EditorView.theme(
  {
    "&": { height: "100%", backgroundColor: "var(--bg)", color: "var(--fg)" },
    ".cm-scroller": {
      lineHeight: "1.65",
    },    ".cm-content": { caretColor: "var(--accent)" },
    ".cm-gutters": {
      backgroundColor: "var(--bg)",
      color: "#6f737a",
      border: "0",
    },
    ".cm-activeLine": { backgroundColor: "#26282c" },
    ".cm-activeLineGutter": { backgroundColor: "#26282c" },
    "&.cm-focused": { outline: "none" },
    ".cm-selectionBackground, ::selection": { backgroundColor: "#214283 !important" },
    ".cm-cursor": { borderLeftColor: "var(--accent)" },
    ".cm-diagnostic": { textUnderlineOffset: "3px" },
    ".cm-diagnostic-error": { textDecoration: "underline wavy #f75464" },
    ".cm-diagnostic-warning": { textDecoration: "underline wavy #e5b94e" },
    ".cm-diagnostic-information, .cm-diagnostic-hint": { textDecoration: "underline dotted #6f9fe0" },
    ".cm-diagnostic-gutter": { width: "1rem" },
    ".cm-diagnostic-marker": { fontSize: "0.7em", lineHeight: "inherit", cursor: "default" },
    ".cm-diagnostic-marker-error": { color: "#f75464" },
    ".cm-diagnostic-marker-warning": { color: "#e5b94e" },
    ".cm-diagnostic-marker-information, .cm-diagnostic-marker-hint": { color: "#6f9fe0" },
    ".cm-tooltip.cm-tooltip-hover": {
      backgroundColor: "var(--raised)",
      border: "1px solid var(--line-input)",
      color: "var(--fg)",
    },
    ".cm-debug-hover": { padding: "0.3rem 0.6rem", maxWidth: "36rem", whiteSpace: "pre-wrap" },
    ".cm-debug-hover.is-error": { color: "var(--error)" },
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

type HoverEvaluation = {
  value: string | null;
  typeName: string | null;
  error: string | null;
};

type Props = {
  value: string;
  path: string;
  platform: string;
  profile: ProfileId;
  revision: number;
  editor: EditorSettings;
  /** Diagnostics for this file: underline in the text, marker in the margin. */
  diagnostics?: EditorDiagnostic[];
  /** 1-based breakpoint lines for this file. */
  breakpoints?: number[];
  /** 1-based line of the current pause, when this file is the paused frame. */
  currentLine?: number | null;
  /** Run/test markers derived from document symbols; one per unambiguous line. */
  gutterActions?: GutterAction[];
  /** 1-based lines changed against HEAD, ascending. */
  changedLines?: ChangedLine[];
  /** Locals of the paused frame, printed after the current line. */
  inlayText?: string;
  /** Present only while paused: hover evaluates the word under the cursor in the adapter. */
  onHoverEvaluate?: (expression: string) => Promise<HoverEvaluation | null>;
  /** Incremented by the Rename action/menu; the inline field opens at the selection. */
  renameRequest?: number;
  /** Incremented by the Go to Declaration action/menu. */
  definitionRequest?: number;
  /** Incremented by Go to Line; opens the CodeMirror dialog at the current line. */
  gotoLineRequest?: number;
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
      fontFamily: `"${settings.fontFamily}", "JetBrains Mono Variable", ui-monospace, Menlo, monospace`,
      fontVariantLigatures: "none",
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

/** Rider-only editing keys; the IntelliJ profile keeps the CodeMirror defaults. */
function profileKeymap(profile: ProfileId) {
  if (profile !== "rider-macos") return [];
  return Prec.high(
    keymap.of([
      { key: "Mod-d", preventDefault: true, run: copyLineDown },
      { key: "Mod-Backspace", preventDefault: true, run: deleteLine },
    ]),
  );
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
  // Rider paints the whole breakpoint line and the paused line, not only the gutter dot.
  const lineTint = EditorView.decorations.of((view) => {
    const marks = [];
    const current = currentLineRef.current;
    for (const line of [...new Set(breakpointsRef.current)].sort((a, b) => a - b)) {
      if (line < 1 || line > view.state.doc.lines) continue;
      marks.push(Decoration.line({ class: line === current ? "cm-line-current" : "cm-line-breakpoint" }).range(view.state.doc.line(line).from));
    }
    if (current !== null && !breakpointsRef.current.includes(current) && current >= 1 && current <= view.state.doc.lines) {
      marks.push(Decoration.line({ class: "cm-line-current" }).range(view.state.doc.line(current).from));
      marks.sort((a, b) => a.from - b.from);
    }
    return Decoration.set(marks);
  });
  return [lineTint, gutter({
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
  })];
}

// Keywords, literals and built-in type names are never variables worth an adapter round-trip.
const notVariables = new Set([
  "abstract", "and", "as", "async", "await", "base", "bool", "break", "byte", "case",
  "catch", "char", "checked", "class", "const", "continue", "decimal", "default", "delegate",
  "do", "double", "dynamic", "else", "enum", "event", "explicit", "extern", "false", "finally",
  "fixed", "float", "for", "foreach", "get", "goto", "if", "implicit", "in", "int",
  "interface", "internal", "is", "lock", "long", "nameof", "namespace", "new", "not", "null",
  "object", "operator", "or", "out", "override", "params", "private", "protected", "public",
  "readonly", "record", "ref", "return", "sbyte", "sealed", "set", "short", "sizeof",
  "stackalloc", "static", "string", "struct", "switch", "this", "throw", "true", "try",
  "typeof", "uint", "ulong", "unchecked", "unsafe", "ushort", "using", "var", "virtual",
  "void", "volatile", "when", "while", "yield",
]);

/** Evaluates the word under the cursor through the host, only while a pause supplies a handler. */
function makeHoverEvaluate(onHoverRef: { current: Props["onHoverEvaluate"] }) {
  return hoverTooltip(
    async (view, pos) => {
      const evaluate = onHoverRef.current;
      if (!evaluate) return null;
      const line = view.state.doc.lineAt(pos);
      const offset = pos - line.from;
      const before = line.text.slice(0, offset).match(/[\w.]+$/)?.[0] ?? "";
      const after = line.text.slice(offset).match(/^[\w.]+/)?.[0] ?? "";
      const expression = `${before}${after}`.replace(/^\.+|\.+$/g, "");
      if (expression.length < 2 || notVariables.has(expression)) return null;
      const result = await evaluate(expression);
      // The pause may have ended while the adapter was answering.
      if (!result || !onHoverRef.current) return null;
      const text = result.error ?? [result.typeName, result.value].filter(Boolean).join(" ");
      return {
        pos: line.from + offset - before.length,
        end: line.from + offset + after.length,
        create() {
          const dom = document.createElement("div");
          dom.className = result.error ? "cm-debug-hover is-error" : "cm-debug-hover";
          dom.textContent = text || "(no value)";
          return { dom };
        },
      };
    },
    { hoverTime: 300 },
  );
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
    } else if (this.action.kind === "fix") {
      path.setAttribute("d", "M8 1.5a4.5 4.5 0 0 0-2.4 8.3c.5.4.9 1 .9 1.7h3c0-.7.4-1.3.9-1.7A4.5 4.5 0 0 0 8 1.5ZM6.5 13.5h3M7 15h2");
      path.setAttribute("fill", "none");
      path.setAttribute("stroke", "currentColor");
      path.setAttribute("stroke-width", "1.3");
      path.setAttribute("stroke-linecap", "round");
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

class DiagnosticMarker extends GutterMarker {
  constructor(
    private readonly severity: EditorDiagnostic["severity"],
    private readonly title: string,
  ) {
    super();
  }

  eq(other: DiagnosticMarker) {
    return other.severity === this.severity && other.title === this.title;
  }

  toDOM() {
    const span = document.createElement("span");
    span.className = `cm-diagnostic-marker cm-diagnostic-marker-${this.severity}`;
    span.textContent = "●";
    span.title = this.title;
    return span;
  }
}

const severityRank = { error: 0, warning: 1, information: 2, hint: 3 };

// Positions come straight from the server/MSBuild (0-based line and character); a missing end
// underlines the identifier that starts at the position.
function makeDiagnostics(list: EditorDiagnostic[], doc: Text) {
  const marks = [];
  const byLine = new Map<number, EditorDiagnostic[]>();
  for (const item of list) {
    if (item.line < 0 || item.line >= doc.lines) continue;
    const line = doc.line(item.line + 1);
    const from = Math.min(line.from + item.character, line.to);
    let to = from;
    if (item.endLine !== null && item.endCharacter !== null && item.endLine < doc.lines) {
      to = Math.min(doc.line(item.endLine + 1).from + item.endCharacter, doc.length);
    } else {
      to = from + (/^\w+/.exec(doc.sliceString(from, line.to))?.[0].length ?? 1);
    }
    to = Math.min(to, doc.length);
    const title = `${item.code ? `${item.code}: ` : ""}${item.message}`;
    if (to > from) {
      marks.push(
        Decoration.mark({
          class: `cm-diagnostic cm-diagnostic-${item.severity}`,
          attributes: { title },
        }).range(from, to),
      );
    }
    byLine.set(item.line + 1, [...(byLine.get(item.line + 1) ?? []), item]);
  }
  return [
    EditorView.decorations.of(Decoration.set(marks, true)),
    gutter({
      class: "cm-diagnostic-gutter",
      markers: (view) => {
        const markers = [];
        for (const [number, items] of byLine) {
          if (number > view.state.doc.lines) continue;
          const worst = items.reduce((a, b) => (severityRank[b.severity] < severityRank[a.severity] ? b : a));
          const title = items.map((i) => `${i.code ? `${i.code}: ` : ""}${i.message}`).join("\n");
          markers.push(new DiagnosticMarker(worst.severity, title).range(view.state.doc.line(number).from));
        }
        return RangeSet.of(markers, true);
      },
    }),
  ];
}

class ChangeMarker extends GutterMarker {
  constructor(private readonly kind: "add" | "mod") { super(); }
  eq(other: ChangeMarker) { return other.kind === this.kind; }
  elementClass = "";
  toDOM() {
    const bar = document.createElement("div");
    bar.className = `cm-change-bar cm-change-${this.kind}`;
    return bar;
  }
}

// Rider's green/blue bar for lines changed against HEAD (computed from the saved file by the host diff).
function makeChangeGutter(changes: ChangedLine[]) {
  return gutter({
    class: "cm-change-gutter",
    markers: (view) => {
      const markers = [];
      for (const [line, kind] of changes) {
        if (line < 1 || line > view.state.doc.lines) continue;
        markers.push(new ChangeMarker(kind).range(view.state.doc.line(line).from));
      }
      return RangeSet.of(markers, true);
    },
  });
}

class InlayWidget extends WidgetType {
  constructor(private readonly text: string) { super(); }
  eq(other: InlayWidget) { return other.text === this.text; }
  toDOM() {
    const span = document.createElement("span");
    span.className = "cm-debug-inlay";
    span.setAttribute("data-testid", "debug-inlay");
    span.textContent = this.text;
    return span;
  }
  ignoreEvent() { return true; }
}

// Rider prints the locals of the paused frame at the end of the current line.
function makeInlay(line: number | null, text: string) {
  return EditorView.decorations.of((view) => {
    if (line === null || text === "" || line < 1 || line > view.state.doc.lines) return Decoration.none;
    return Decoration.set([
      Decoration.widget({ widget: new InlayWidget(text), side: 1 }).range(view.state.doc.line(line).to),
    ]);
  });
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

export type ChangedLine = [line: number, kind: "add" | "mod"];

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
  profile,
  revision,
  editor,
  diagnostics = [],
  breakpoints = [],
  currentLine = null,
  gutterActions = [],
  changedLines = [],
  inlayText = "",
  renameRequest = 0,
  definitionRequest = 0,
  gotoLineRequest = 0,
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
  onHoverEvaluate,
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
  const onHoverEvaluateRef = useRef(onHoverEvaluate);
  const breakpointsRef = useRef<number[]>(breakpoints);
  const currentLineRef = useRef<number | null>(currentLine);
  const gutterActionsRef = useRef<GutterAction[]>(gutterActions);
  const onToggleBreakpointRef = useRef(onToggleBreakpoint);
  const gutterCompartment = useRef(new Compartment());
  const actionGutterCompartment = useRef(new Compartment());
  const changeCompartment = useRef(new Compartment());
  const diagnosticCompartment = useRef(new Compartment());
  const inlayCompartment = useRef(new Compartment());
  const changedLinesRef = useRef(changedLines);
  changedLinesRef.current = changedLines;
  const fontCompartment = useRef(new Compartment());
  const indentCompartment = useRef(new Compartment());
  const profileCompartment = useRef(new Compartment());
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
  onHoverEvaluateRef.current = onHoverEvaluate;
  pathRef.current = path;
  revisionRef.current = revision;
  const applyingExternal = useRef(false);
  const completionSeq = useRef(0);
  const definitionSeq = useRef(0);
  const inlineSeq = useRef(0);
  const inlineRef = useRef<InlineRename | null>(null);
  const handledRename = useRef(renameRequest);
  const handledDefinition = useRef(definitionRequest);
  const handledGotoLine = useRef(gotoLineRequest);
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
    setStatus({
      state: "idle",
      text: `Ctrl+Space: completion · ${platform === "macos" ? "Cmd+B" : "Ctrl+B"}: go to definition · Shift+F6: rename · Shift+F12: find usages · ${platform === "macos" ? "Option" : "Alt"}+Enter: code actions`,
    });
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
    const top = start.bottom - shell.top + 4;
    setInline({
      from: word.from,
      to: word.to,
      left,
      top: top + 180 > shell.height ? Math.max(8, start.top - shell.top - 44) : top,
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
        basicSetup,
        changeCompartment.current.of(makeChangeGutter(changedLinesRef.current)),
        diagnosticCompartment.current.of(makeDiagnostics(diagnostics, EditorState.create({ doc: value }).doc)),
        inlayCompartment.current.of(makeInlay(currentLine, inlayText)),
        makeHoverEvaluate(onHoverEvaluateRef),
        autocompletion({ override: [completionSource], activateOnTyping: false }),
        StreamLanguage.define(csharp),
        theme,
        fontCompartment.current.of(editorTheme(editor)),
        indentCompartment.current.of(editorIndent(editor)),
        profileCompartment.current.of(profileKeymap(profile)),
        syntaxHighlighting(syntaxColors),
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
    view.current?.dispatch({
      effects: profileCompartment.current.reconfigure(profileKeymap(profile)),
    });
  }, [profile]);

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
    view.current?.dispatch({ effects: inlayCompartment.current.reconfigure(makeInlay(currentLine, inlayText)) });
  }, [currentLine, inlayText]);

  // Reveal the paused line from any source (breakpoint, step, exception, run to cursor) without
  // taking focus from the Debug panel. An already visible line is left alone.
  useEffect(() => {
    const instance = view.current;
    if (!instance || currentLine === null) return;
    if (currentLine < 1 || currentLine > instance.state.doc.lines) return;
    const position = instance.state.doc.line(currentLine).from;
    const block = instance.lineBlockAt(position);
    const scroller = instance.scrollDOM;
    if (block.top >= scroller.scrollTop && block.bottom <= scroller.scrollTop + scroller.clientHeight) {
      return;
    }
    instance.dispatch({ effects: EditorView.scrollIntoView(position, { y: "nearest" }) });
  }, [currentLine]);

  const changeKey = changedLines.map(([line, kind]) => `${line}${kind}`).join(",");
  useEffect(() => {
    view.current?.dispatch({
      effects: changeCompartment.current.reconfigure(makeChangeGutter(changedLinesRef.current)),
    });
  }, [changeKey]);

  const diagnosticKey = JSON.stringify(diagnostics);
  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    instance.dispatch({
      effects: diagnosticCompartment.current.reconfigure(makeDiagnostics(diagnostics, instance.state.doc)),
    });
    // The serialized list is the dependency; the array identity changes on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [diagnosticKey]);

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
    if (gotoLineRequest === handledGotoLine.current) return;
    handledGotoLine.current = gotoLineRequest;
    const instance = view.current;
    if (!instance) return;
    // Focus first so the dialog keeps its own input focus when it opens.
    instance.focus();
    gotoLine(instance);
    // The nonce is the only trigger; the dialog starts at the current line.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gotoLineRequest]);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    const current = instance.state.doc.toString();
    if (current !== value) {
      // A clean reload must keep the caret where it was and must not become an undo step.
      const head = instance.state.selection.main.head;
      const before = instance.state.doc.lineAt(head);
      const line = before.number;
      const column = head - before.from;
      // Reloads set content in React state; the dispatch must not count as a user edit,
      // otherwise the reloaded buffer would immediately be dirty at a higher revision.
      applyingExternal.current = true;
      instance.dispatch({
        changes: { from: 0, to: current.length, insert: value },
        annotations: Transaction.addToHistory.of(false),
      });
      applyingExternal.current = false;
      const doc = instance.state.doc;
      const target = doc.line(Math.min(line, doc.lines));
      const position = Math.min(target.from + column, target.to);
      instance.dispatch({
        selection: { anchor: position },
        annotations: Transaction.addToHistory.of(false),
      });
      publishCursor(instance);
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
            {path} · {inline.line + 1}:{inline.character + 1} · <mark>{inline.source}</mark>
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
        title={status.text}
      >
        {status.text}
      </div>
    </div>
  );
}
