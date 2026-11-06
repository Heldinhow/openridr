import { useEffect, useRef } from "react";
import { EditorState, RangeSet } from "@codemirror/state";
import { Decoration, EditorView, GutterMarker, gutter, lineNumbers } from "@codemirror/view";
import type { DiffRow } from "./commitDiffRows";
import { editorTheme } from "./commitDiffTheme";

// One marker per changed line, on the right edge of the base pane: half a bar for a one-sided
// line, the full bar for a modified pair. The gutter aligns by construction, no pixel math.
class ConnectorMarker extends GutterMarker {
  constructor(private readonly kind: "removed" | "added" | "modified") { super(); }
  eq(other: ConnectorMarker) { return other.kind === this.kind; }
  toDOM() {
    const bar = document.createElement("div");
    bar.className = `cm-diff-connector cm-diff-connector-${this.kind}`;
    return bar;
  }
}

function connectorGutter(rows: DiffRow[]) {
  return gutter({
    class: "cm-diff-connectors",
    side: "after",
    markers: (view) => {
      const markers: ReturnType<ConnectorMarker["range"]>[] = [];
      rows.forEach((row, index) => {
        if (index >= view.state.doc.lines) return;
        const kind = row.left.kind === "modified" && row.right.kind === "modified" ? "modified"
          : row.left.kind === "removed" ? "removed"
          : row.right.kind === "added" ? "added" : null;
        if (!kind) return;
        markers.push(new ConnectorMarker(kind).range(view.state.doc.line(index + 1).from));
      });
      return RangeSet.of(markers, true);
    },
  });
}

function SplitDiff({ rows, difference }: { rows: DiffRow[]; difference: number }) {
  const left = useRef<HTMLDivElement>(null), right = useRef<HTMLDivElement>(null);
  const views = useRef<EditorView[]>([]);
  useEffect(() => {
    const parents = [left.current, right.current];
    const instances = parents.map((parent, side) => {
      const cells = rows.map((row) => side === 0 ? row.left : row.right);
      const doc = cells.map((cell) => cell.text).join("\n");
      const ranges: ReturnType<ReturnType<typeof Decoration.line>["range"]>[] = [];
      let pos = 0;
      for (const cell of cells) {
        if (cell.kind !== "context") ranges.push(Decoration.line({ class: `git-diff-line-${cell.kind}` }).range(pos));
        if (cell.mark && cell.mark[0] < cell.mark[1]) ranges.push(Decoration.mark({ class: "git-diff-intraline" }).range(pos + cell.mark[0], pos + cell.mark[1]));
        pos += cell.text.length + 1;
      }
      return new EditorView({ parent: parent!, state: EditorState.create({ doc, extensions: [
        EditorState.readOnly.of(true), EditorView.editable.of(false),
        lineNumbers({ formatNumber: (number) => String(cells[number - 1]?.number ?? "") }),
        EditorView.decorations.of(Decoration.set(ranges, true)),
        editorTheme,
        ...(side === 0 ? [connectorGutter(rows)] : []),
      ] }) });
    });
    views.current = instances;
    let syncing = false;
    const handlers = instances.map((view, i) => () => {
      if (syncing) return;
      syncing = true;
      instances[1 - i].scrollDOM.scrollTop = view.scrollDOM.scrollTop;
      syncing = false;
    });
    instances.forEach((view, i) => view.scrollDOM.addEventListener("scroll", handlers[i]));
    return () => { instances.forEach((view, i) => { view.scrollDOM.removeEventListener("scroll", handlers[i]); view.destroy(); }); views.current = []; };
  }, [rows]);
  useEffect(() => {
    const index = rows.findIndex((row) => row.difference === difference);
    if (index < 0) return;
    for (const view of views.current) {
      const line = view.state.doc.line(Math.min(index + 1, view.state.doc.lines));
      view.dispatch({ effects: EditorView.scrollIntoView(line.from, { y: "center" }) });
    }
  }, [rows, difference]);
  return <div className="git-diff-editors"><div ref={left} aria-label="Base diff editor" /><div ref={right} aria-label="Current diff editor" /></div>;
}

export { SplitDiff };
