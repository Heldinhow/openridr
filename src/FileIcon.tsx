import { Icon, type IconName } from "./Icon";
import { CSharpIcon } from "./icons/CSharpIcon";
import { ProjectIcon } from "./icons/ProjectIcon";
import { SymbolIcon } from "./icons/SymbolIcon";
import { fileKind, type FileKind, type ProjectKind, type SymbolKind } from "./solutionVisuals";

const GLYPH: Partial<Record<FileKind, IconName>> = {
  solution: "solution",
  props: "code",
  json: "braces",
  xml: "code",
  razor: "code",
  resource: "package",
  markdown: "text",
  text: "text",
  yaml: "runconfig",
  config: "settings",
  script: "terminal",
  file: "file",
};

/** Type glyph for a path; color is a token (`--icon-*`) applied by the `tint-*` class. */
export function FileIcon({
  path,
  project = false,
  projectKind = "unknown",
  symbolKind,
}: {
  path: string;
  project?: boolean;
  projectKind?: ProjectKind;
  symbolKind?: SymbolKind;
}) {
  if (symbolKind) return <SymbolIcon kind={symbolKind} />;
  const kind = project ? "project" : fileKind(path);
  if (kind === "csharp") {
    return (
      <span className="file-icon tint-csharp" aria-hidden="true">
        <CSharpIcon />
      </span>
    );
  }
  if (kind === "project") return <ProjectIcon kind={projectKind} />;
  return <Icon name={GLYPH[kind] ?? "file"} className={`tint-${kind}`} />;
}
