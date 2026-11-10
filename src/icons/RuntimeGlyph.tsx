/**
 * A framework row: the .NET runtime silhouette (a filled hexagon ring), not a settings gear.
 * Drawn in this repository, like the folder glyphs.
 */
export function RuntimeGlyph() {
  return (
    <svg className="runtime-glyph" width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M8 1.1 14.2 4.7v6.6L8 14.9 1.8 11.3V4.7zM8 3.3 3.5 5.9v4.2L8 12.7l4.5-2.6V5.9z"
      />
    </svg>
  );
}
