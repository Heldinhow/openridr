/** The Dependencies node: one project connected to its frameworks, packages and project references. */
export function DependencyGlyph() {
  return (
    <svg className="dependency-glyph" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M7 5.5 3.6 10M9 5.5l3.4 4.5" stroke="currentColor" strokeWidth="1.3" />
      <circle cx="8" cy="3.5" r="2" stroke="currentColor" strokeWidth="1.3" />
      <circle cx="3" cy="12" r="2" stroke="currentColor" strokeWidth="1.3" />
      <circle cx="13" cy="12" r="2" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}
