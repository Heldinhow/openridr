import type { SymbolKind } from "../solutionVisuals";

/** Spoken/written form of a symbol kind: the row description, and the text the filter shows. */
export const SYMBOL_LABEL: Record<SymbolKind, string> = {
  class: "class",
  interface: "interface",
  enum: "enum",
  record: "record",
  struct: "struct",
};
