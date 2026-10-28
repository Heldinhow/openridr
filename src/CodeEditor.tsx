import { useEffect, useRef } from "react";
import { basicSetup } from "codemirror";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { StreamLanguage } from "@codemirror/language";
import { csharp } from "@codemirror/legacy-modes/mode/clike";

const theme = EditorView.theme(
  {
    "&": { height: "100%", backgroundColor: "var(--bg)", color: "var(--fg)" },
    ".cm-scroller": {
      fontFamily: "var(--mono)",
      fontSize: "13px",
      lineHeight: "1.65",
    },
    ".cm-content": { caretColor: "var(--accent)" },
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

type Target = { path: string; line: number; character: number; nonce: number };

type Props = {
  value: string;
  onChange: (value: string) => void;
  onSave: () => void;
  target?: Target | null;
};

export function CodeEditor({ value, onChange, onSave, target }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);
  onChangeRef.current = onChange;
  onSaveRef.current = onSave;
  const applyingExternal = useRef(false);

  function publishCursor(instance: EditorView) {
    const position = instance.state.selection.main.head;
    const line = instance.state.doc.lineAt(position);
    if (host.current) {
      host.current.dataset.cursor = `${line.number - 1}:${position - line.from}`;
    }
  }

  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        StreamLanguage.define(csharp),
        theme,
        EditorView.contentAttributes.of({
          "aria-label": "Editor",
          "aria-multiline": "true",
          "data-testid": "editor",
          spellcheck: "false",
        }),
        keymap.of([
          {
            key: "Mod-s",
            preventDefault: true,
            run: () => {
              onSaveRef.current();
              return true;
            },
          },
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !applyingExternal.current) {
            onChangeRef.current(update.state.doc.toString());
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

  return <div className="code-editor" data-testid="editor-shell" ref={host} />;
}
