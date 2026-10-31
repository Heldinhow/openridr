import { Icon } from "./Icon";

/** Rider-style type badge: green "C#" for C# projects/sources, braces for JSON, plain file otherwise. */
export function FileIcon({ path, project = false }: { path: string; project?: boolean }) {
  if (project || path.endsWith(".cs")) {
    return <span className={`cs-badge${project ? " project" : ""}`} aria-hidden="true" />;
  }
  if (path.endsWith(".json")) return <span className="json-badge" aria-hidden="true" />;
  return <Icon name="file" />;
}
