import { useEffect, useMemo, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { Decoration, EditorView, lineNumbers } from "@codemirror/view";
import { projectUnified, type DiffRow } from "./commitDiffRows";
import { editorTheme } from "./commitDiffTheme";

export function UnifiedDiff({ rows, difference }: { rows: DiffRow[]; difference: number }) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const projected = useMemo(() => projectUnified(rows), [rows]);
  useEffect(() => {
    const doc = projected.lines.map((line) => `${line.prefix}${line.text}`).join("\n");
    // The prefix is part of the document line, so intraline marks shift by one character.
    const ranges: ReturnType<ReturnType<typeof Decoration.line>["range"]>[] = [];
    let pos = 0;
    for (const line of projected.lines) {
      if (line.kind !== "context") ranges.push(Decoration.line({ class: `git-diff-line-${line.kind}` }).range(pos));
      const shift = line.prefix.length;
      if (line.mark && line.mark[0] < line.mark[1]) ranges.push(Decoration.mark({ class: "git-diff-intraline" }).range(pos + shift + line.mark[0], pos + shift + line.mark[1]));
      pos += line.text.length + shift + 1;
    }
    view.current = new EditorView({ parent: host.current!, state: EditorState.create({ doc, extensions: [
      EditorState.readOnly.of(true), EditorView.editable.of(false),
      lineNumbers({ formatNumber: (number) => String(projected.lines[number - 1]?.number ?? "") }),
      EditorView.decorations.of(Decoration.set(ranges, true)),
      editorTheme,
    ] }) });
    return () => { view.current?.destroy(); view.current = null; };
  }, [projected]);
  useEffect(() => {
    const index = projected.differenceLines[difference];
    const current = view.current;
    if (index === undefined || !current) return;
    const line = current.state.doc.line(Math.min(index + 1, current.state.doc.lines));
    current.dispatch({ effects: EditorView.scrollIntoView(line.from, { y: "center" }) });
  }, [projected, difference]);
  return <div className="git-diff-unified" data-testid="commit-diff-unified"><div ref={host} aria-label="Unified diff editor" /></div>;
}
