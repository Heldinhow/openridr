import type { ReactNode } from "react";
import { DocGlyph, TextGlyph } from "./icons/fileGlyphs";
import { SYMBOL_LABEL } from "./icons/SymbolMark";
import { fileGlyph, fileKind, fileToken, type FileGlyph, type SymbolKind } from "./solutionVisuals";

/**
 * Codepoints of the vendored Seti font (`src/assets/seti.woff`), read from the theme's
 * `iconDefinitions` (`vs-seti-icon-theme.json`). Provenance and license: THIRD-PARTY-NOTICES.md.
 */
const SETI: Partial<Record<FileGlyph, string>> = {
  csharp: "\uE00B",
  project: "\uE0A5",
  xml: "\uE0A5",
  json: "\uE055",
  razor: "\uE048",
  html: "\uE048",
  markdown: "\uE060",
  yaml: "\uE0A7",
  docker: "\uE025",
  git: "\uE034",
  script: "\uE089",
  powershell: "\uE074",
  batch: "\uE0A1",
  css: "\uE01D",
  image: "\uE04C",
  license: "\uE05A",
  file: "\uE023",
};

/**
 * Vendored @pierre/trees glyphs, kept only where Seti has no drawing: `.sln`, `.resx`,
 * `.editorconfig` and `.DotSettings`. They are drawn at full strength like the Seti glyphs.
 */
const CUSTOM: Partial<Record<FileGlyph, ReactNode>> = {
  doc: <DocGlyph />,
  resource: <DocGlyph />,
  text: <TextGlyph />,
  config: <TextGlyph />,
};

/**
 * Type glyph for a path; the tint is the table's token (`tint-<token>` → `--icon-<token>`).
 *
 * C# symbol kinds remain in the tooltip and row description, not a visual badge.
 */
export function FileIcon({ path, symbolKind }: { path: string; symbolKind?: SymbolKind }) {
  const kind = fileKind(path);
  const glyph = fileGlyph(kind);
  const character = SETI[glyph];
  return (
    <span
      className={`file-icon tint-${fileToken(kind)}`}
      data-icon={glyph}
      aria-hidden="true"
      title={glyph === "csharp" ? (symbolKind ? `C# ${SYMBOL_LABEL[symbolKind]}` : "C# file") : undefined}
    >
      {character ? <span className="seti-glyph">{character}</span> : CUSTOM[glyph]}
    </span>
  );
}
