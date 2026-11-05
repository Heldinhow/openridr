import { Icon, type IconName } from "../Icon";
import type { ProjectKind } from "../solutionVisuals";

const GLYPH: Record<ProjectKind, IconName> = {
  console: "terminal",
  library: "library",
  test: "tests",
  web: "web",
  unknown: "project",
};

/** One glyph per project kind; the tint comes from `.project-<kind>` in styles.css. */
export function ProjectIcon({ kind }: { kind: ProjectKind }) {
  return <Icon name={GLYPH[kind]} className={`project-icon project-${kind}`} />;
}
