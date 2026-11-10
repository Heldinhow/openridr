// File glyphs for the Solution tree that the Seti theme has no drawing for: the `.sln`, `.resx`,
// `.editorconfig` and `.DotSettings` rows.
//
// Vendored from @pierre/trees 1.0.0-beta.6 (`dist/builtInIcons.js`), the icon set the T3 Code tree
// uses. @pierre/trees is Apache-2.0; only the two symbols below are copied, unmodified except for
// dropping the sprite wrapper. Full license text and provenance: THIRD-PARTY-NOTICES.md.
//
// Colors are never hardcoded here: every path uses `currentColor` and the row's `tint-*` class
// supplies the token from styles.css.

import type { ReactNode } from "react";

type GlyphProps = { className?: string };

function Svg({ className, children }: GlyphProps & { children: ReactNode }) {
  return (
    <svg
      className={`file-glyph${className ? ` ${className}` : ""}`}
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

/** Generic document: page with a folded corner (`.glyph-page` is the page body). */
export function DocGlyph(props: GlyphProps) {
  return (
    <Svg {...props}>
      <path className="glyph-page" d="M8 1v3a3 3 0 0 0 3 3h3v5.5a2.5 2.5 0 0 1-2.5 2.5h-7A2.5 2.5 0 0 1 2 12.5v-9A2.5 2.5 0 0 1 4.5 1z" />
      <path d="M9.5 1a.5.5 0 0 1 .354.146l4 4A.5.5 0 0 1 14 5.5V6h-3a2 2 0 0 1-2-2V1z" />
    </Svg>
  );
}

/** Text page: the same page with ruled lines, for settings and plain-text rows. */
export function TextGlyph(props: GlyphProps) {
  return (
    <Svg {...props}>
      <path className="glyph-page" fillRule="evenodd" clipRule="evenodd" d="M8 4a3 3 0 0 0 3 3h3v5.5a2.5 2.5 0 0 1-2.5 2.5h-7A2.5 2.5 0 0 1 2 12.5v-9A2.5 2.5 0 0 1 4.5 1H8z" />
      <path d="M8.5 11a.5.5 0 0 1 0 1h-3a.5.5 0 0 1 0-1zm2-2a.5.5 0 0 1 0 1h-5a.5.5 0 0 1 0-1zm-1-8a.5.5 0 0 1 .354.146l4 4A.5.5 0 0 1 14 5.5V6h-3a2 2 0 0 1-2-2V1z" />
    </Svg>
  );
}
