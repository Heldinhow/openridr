/**
 * Folder glyphs drawn in this repository (no third-party asset): the Seti theme has no folder
 * drawing, so the tree keeps its own two-state folder. Filled like the file glyphs, one per state,
 * and never keyed to the folder name — `src`, `tests`, `bin` and `obj` look the same.
 */
export function FolderIcon({ open }: { open: boolean }) {
  return (
    <svg
      className="folder-glyph"
      data-folder={open ? "open" : "closed"}
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
    >
      {open ? (
        <>
          <path d="M1.5 3A1.5 1.5 0 0 1 3 1.5h3.1c.4 0 .78.16 1.06.44l1.1 1.1H13A1.5 1.5 0 0 1 14.5 4.5v1.2H1.5z" />
          <path d="M1.5 6.2h13.3l-1.05 5.4A1.5 1.5 0 0 1 12.28 13H3.2a1.5 1.5 0 0 1-1.47-1.2z" />
        </>
      ) : (
        <path d="M1.5 3A1.5 1.5 0 0 1 3 1.5h3.1c.4 0 .78.16 1.06.44l1.1 1.1H13A1.5 1.5 0 0 1 14.5 4.5v7A1.5 1.5 0 0 1 13 13H3a1.5 1.5 0 0 1-1.5-1.5z" />
      )}
    </svg>
  );
}
