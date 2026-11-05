import type { SymbolKind } from "../solutionVisuals";

const LETTER: Record<SymbolKind, string> = {
  class: "C",
  interface: "I",
  enum: "E",
  record: "R",
  struct: "S",
};

/** Rider-style letter badge for a C# type; the letter is ours, not an icon font. */
export function SymbolIcon({ kind }: { kind: SymbolKind }) {
  return (
    <span className={`symbol-badge symbol-${kind}`} aria-hidden="true">
      {LETTER[kind]}
    </span>
  );
}
