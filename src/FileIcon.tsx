import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import {
  CSharpGlyph, DockerGlyph, DocGlyph, GitGlyph, JsonGlyph, MarkdownGlyph, TextGlyph, YamlGlyph,
} from "./icons/fileGlyphs";
import { SYMBOL_LABEL, SymbolMark } from "./icons/SymbolMark";
import { fileGlyph, fileKind, type FileGlyph, type SymbolKind } from "./solutionVisuals";

/** Vendored SVGs (see THIRD-PARTY-NOTICES); shared elements are immutable, so they live here. */
const CUSTOM: Partial<Record<FileGlyph, ReactNode>> = {
  doc: <DocGlyph />,
  json: <JsonGlyph />,
  markdown: <MarkdownGlyph />,
  docker: <DockerGlyph />,
  yaml: <YamlGlyph />,
  text: <TextGlyph />,
  git: <GitGlyph />,
};

/** Kinds the bundled Lucide set already draws well. */
const LUCIDE: Partial<Record<FileGlyph, IconName>> = {
  xml: "code",
  razor: "code",
  resource: "package",
  config: "settings",
  script: "terminal",
};

/**
 * Type glyph for a path; color is a token (`--icon-*`) applied by the `tint-*` class.
 *
 * A `.cs` file is always the `C#` hexagon (SPEC-SOLUTION-FILE-ICONS D2); a known symbol kind is a
 * small corner mark plus the tooltip, never a replacement for the file type.
 */
export function FileIcon({ path, symbolKind }: { path: string; symbolKind?: SymbolKind }) {
  const glyph = fileGlyph(fileKind(path));
  if (glyph === "csharp") {
    return (
      <span
        className="file-icon tint-csharp"
        data-icon={glyph}
        title={symbolKind ? `C# ${SYMBOL_LABEL[symbolKind]}` : "C# file"}
      >
        <CSharpGlyph />
        {symbolKind && <SymbolMark kind={symbolKind} />}
      </span>
    );
  }
  return (
    <span className={`file-icon tint-${glyph}`} data-icon={glyph} aria-hidden="true">
      {CUSTOM[glyph] ?? <Icon name={LUCIDE[glyph] ?? "file"} />}
    </span>
  );
}
