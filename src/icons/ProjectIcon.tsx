import type { ProjectKind } from "../solutionVisuals";

/** Original outlined C# project mark; project kind remains available on the row. */
export function ProjectIcon({ kind }: { kind: ProjectKind }) {
  return (
    <svg className={`project-icon project-${kind}`} width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <rect x="1" y="2.5" width="14" height="11" rx="1.5" stroke="currentColor" strokeWidth="1.2" />
      <text x="2.5" y="10.7" fill="currentColor" fontFamily="Arial, sans-serif" fontSize="7.5" fontWeight="700">C#</text>
    </svg>
  );
}
