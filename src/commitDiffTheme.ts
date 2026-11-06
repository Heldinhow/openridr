import { EditorView } from "@codemirror/view";

export const editorTheme = EditorView.theme({
  "&": { height: "100%" },
  ".cm-scroller": { overflow: "auto", fontFamily: "var(--font-mono, monospace)" },
});
