import { Icon, type IconName } from "./Icon";
import { CSharpIcon } from "./icons/CSharpIcon";

const byExtension: Record<string, IconName> = {
  csproj: "project",
  props: "project",
  targets: "project",
  sln: "solution",
  slnx: "solution",
  json: "braces",
  md: "text",
  txt: "text",
};

/** Neutral type glyphs: C# lettering for C# projects/sources, Lucide for the rest. Color is state-only. */
export function FileIcon({ path, project = false }: { path: string; project?: boolean }) {
  const ext = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
  if (project || ext === "cs") return <CSharpIcon />;
  return <Icon name={byExtension[ext] ?? "file"} />;
}
