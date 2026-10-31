import type { DocumentSymbol, TestItem } from "./host/client";
import { splitTestName } from "./testTree";

export type GutterAction = {
  /** 1-based line in the active file. */
  line: number;
  kind: "run" | "test" | "fix";
  /** 0-based column, only for "fix" (the diagnostic position). */
  character?: number;
  label: string;
  disabledReason: string | null;
  testId: string | null;
  testProject: string | null;
};

type FlatSymbol = {
  name: string;
  kind: number;
  containerName: string;
  line: number;
  character: number;
  /** Whole-symbol line span, used when the server gives no containerName (csharp-ls sends a flat list). */
  startLine: number;
  endLine: number;
};

/** Document symbols are nested; matching only needs the immediate parent name. */
export function flattenDocumentSymbols(symbols: DocumentSymbol[]): FlatSymbol[] {
  const flat: FlatSymbol[] = [];
  const walk = (nodes: DocumentSymbol[]) => {
    for (const node of nodes) {
      flat.push({
        name: node.name,
        kind: node.kind,
        containerName: node.containerName,
        line: node.selectionRange.start.line,
        character: node.selectionRange.start.character,
        startLine: node.range.start.line,
        endLine: node.range.end.line,
      });
      walk(node.children);
    }
  };
  walk(symbols);
  return flat;
}

function simpleName(name: string) {
  const last = name.split(".").pop() ?? name;
  return last.split("+").pop() ?? last;
}

/** csharp-ls names methods with their parameter list ("Id_is_preserved()"). */
function bareName(name: string) {
  const open = name.indexOf("(");
  return open < 0 ? name : name.slice(0, open);
}

function methodSymbol(
  symbols: FlatSymbol[],
  className: string,
  method: string,
) {
  const kind = simpleName(className);
  return symbols.find(
    (symbol) =>
      symbol.kind === 6 &&
      bareName(symbol.name) === method &&
      (symbol.containerName
        ? simpleName(symbol.containerName) === kind
        : symbols.some(
            (owner) =>
              owner.kind === 5 &&
              owner.name === kind &&
              owner.startLine <= symbol.line &&
              symbol.line <= owner.endLine,
          )),
  );
}

/** Run is shown only on the single `Main` of the startup project's file; a second `Main`
 * (or none) is ambiguous and gets no icon. Tests match class+method of the FQN against the
 * file's document symbols, and a name VSTest could not identify is ambiguous. */
export function gutterActionsFor({
  symbols,
  projectPath,
  startupProjectPath,
  testItems,
  runDisabledReason,
  testDisabledReason,
}: {
  symbols: DocumentSymbol[];
  projectPath: string | null;
  startupProjectPath: string | null;
  testItems: TestItem[];
  runDisabledReason: string | null;
  testDisabledReason: string | null;
}): GutterAction[] {
  const flat = flattenDocumentSymbols(symbols);
  const byLine = new Map<number, GutterAction>();
  let runLine: number | null = null;
  if (projectPath !== null && projectPath === startupProjectPath) {
    const mains = new Set(
      flat
        .filter((symbol) => symbol.kind === 6 && bareName(symbol.name) === "Main")
        .map((symbol) => symbol.line + 1),
    );
    if (mains.size === 1) runLine = [...mains][0];
  }

  const testsByLine = new Map<number, TestItem[]>();
  for (const item of testItems) {
    if (projectPath !== null && item.project !== projectPath) continue;
    if (item.occurrences > 1) continue;
    const parts = splitTestName(item.fullyQualifiedName);
    const symbol = methodSymbol(flat, parts.className, parts.method);
    if (!symbol) continue;
    const line = symbol.line + 1;
    testsByLine.set(line, [...(testsByLine.get(line) ?? []), item]);
  }
  for (const [line, items] of testsByLine) {
    if (items.length !== 1) continue;
    const item = items[0];
    byLine.set(line, {
      line,
      kind: "test",
      label: `Run test ${item.fullyQualifiedName}`,
      disabledReason: testDisabledReason,
      testId: item.id,
      testProject: item.project,
    });
  }
  if (runLine !== null && !byLine.has(runLine)) {
    byLine.set(runLine, {
      line: runLine,
      kind: "run",
      label: "Run the startup project",
      disabledReason: runDisabledReason,
      testId: null,
      testProject: null,
    });
  }
  return [...byLine.values()].sort((left, right) => left.line - right.line);
}

export function gutterActionAt(actions: GutterAction[], line: number) {
  return actions.find((action) => action.line === line) ?? null;
}

/** Case-insensitive snippet for the Find Usages list; the host owns the canonical position. */
export function lineSnippet(content: string, line: number) {
  const text = content.split("\n")[line] ?? "";
  const trimmed = text.trim();
  return trimmed.length > 240 ? `${trimmed.slice(0, 240)}…` : trimmed;
}
