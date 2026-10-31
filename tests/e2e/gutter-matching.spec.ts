import { expect, test } from "@playwright/test";
import { gutterActionsFor } from "../../src/codeNavigation";
import type { DocumentSymbol, TestItem } from "../../src/host/client";

// Pure matching rules of the Run/Test gutter (no host, no browser): csharp-ls sends a flat symbol list
// with names like "M()" and no containerName, so the owner class comes from line ranges.
const project = "tests/T/T.csproj";

function range(start: number, end: number) {
  return { start: { line: start, character: 0 }, end: { line: end, character: 0 } };
}
function symbol(name: string, kind: number, start: number, end: number, containerName = ""): DocumentSymbol {
  return { name, kind, containerName, range: range(start, end), selectionRange: range(start, start), children: [] };
}
function item(fqn: string, occurrences = 1): TestItem {
  return { id: `id:${fqn}`, fullyQualifiedName: fqn, displayName: fqn, source: "vstest", project, tfm: "net8.0", discoveryId: "d", revision: 1, occurrences };
}
function testLines(symbols: DocumentSymbol[], items: TestItem[]) {
  return gutterActionsFor({
    symbols, projectPath: project, startupProjectPath: null, testItems: items, runDisabledReason: null, testDisabledReason: null,
  }).filter((action) => action.kind === "test").map((action) => [action.line, action.testId]);
}

test("a neighbouring class with a same-named method only marks its own test", () => {
  const symbols = [symbol("A", 5, 0, 5), symbol("M()", 6, 2, 3), symbol("B", 5, 6, 11), symbol("M()", 6, 8, 9)];
  expect(testLines(symbols, [item("N.B.M")])).toEqual([[9, "id:N.B.M"]]);
  expect(testLines(symbols, [item("N.A.M")])).toEqual([[3, "id:N.A.M"]]);
});

test("a nested class does not mark a test that belongs to its outer class", () => {
  const symbols = [symbol("Outer", 5, 0, 12), symbol("Inner", 5, 2, 6), symbol("M()", 6, 4, 5)];
  expect(testLines(symbols, [item("N.Outer.M")])).toEqual([]);
  expect(testLines(symbols, [item("N.Outer+Inner.M")])).toEqual([[5, "id:N.Outer+Inner.M"]]);
  const withOuterMethod = [...symbols, symbol("M()", 6, 8, 9)];
  expect(testLines(withOuterMethod, [item("N.Outer.M")])).toEqual([[9, "id:N.Outer.M"]]);
});

test("overloads share one identity: the first one is marked", () => {
  const symbols = [symbol("A", 5, 0, 9), symbol("M(int)", 6, 2, 3), symbol("M(string)", 6, 5, 6)];
  expect(testLines(symbols, [item("N.A.M")])).toEqual([[3, "id:N.A.M"]]);
});

test("symbols with a containerName keep matching by that name", () => {
  const symbols = [symbol("A", 5, 0, 5), symbol("M", 6, 2, 3, "A"), symbol("M", 6, 8, 9, "B")];
  expect(testLines(symbols, [item("N.B.M")])).toEqual([[9, "id:N.B.M"]]);
});

test("a struct or record owner is not matched (safe false negative)", () => {
  const symbols = [symbol("S", 23, 0, 5), symbol("M()", 6, 2, 3)];
  expect(testLines(symbols, [item("N.S.M")])).toEqual([]);
});
