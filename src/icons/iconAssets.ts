// URLs of the vendored Material Icon Theme SVGs.
//
// They live in `public/`, which Vite serves at the root and copies into the bundle verbatim, so a
// drawing is a plain URL: no import, no bundler transform, and the file stays exactly as the theme
// ships it (the fills are the theme's own colours, not a `currentColor` this app tints).
// Provenance, commit and license: THIRD-PARTY-NOTICES.md.

import type { FileIconName, FolderIconName } from "../solutionVisuals";

export function fileIconUrl(name: FileIconName): string {
  return `/file-icons/${name}.svg`;
}

export function folderIconUrl(name: FolderIconName, open: boolean): string {
  return `/folder-icons/${open ? `${name}-open` : name}.svg`;
}
