import type { SymbolKind } from "../solutionVisuals";

/** The letter each symbol kind shows in the corner mark of a `.cs` icon. */
export const SYMBOL_LETTER: Record<SymbolKind, string> = {
  class: "C",
  interface: "I",
  enum: "E",
  record: "R",
  struct: "S",
};

/** Spoken/written form of a symbol kind, for the icon tooltip. */
export const SYMBOL_LABEL: Record<SymbolKind, string> = {
  class: "class",
  interface: "interface",
  enum: "enum",
  record: "record",
  struct: "struct",
};

/**
 * Type mark for a C# file: a 9px corner sticker on the `C#` hexagon.
 *
 * The hexagon is the file icon (SPEC-SOLUTION-FILE-ICONS D2); the kind stays visible as this mark
 * plus the row's `data-symbol-kind` and `title`, so PR #23's information is not lost.
 */
export function SymbolMark({ kind }: { kind: SymbolKind }) {
  return (
    <span className={`symbol-mark symbol-${kind}`} aria-hidden="true">
      {SYMBOL_LETTER[kind]}
    </span>
  );
}
