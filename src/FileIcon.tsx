import { fileIconUrl } from "./icons/iconAssets";
import { SYMBOL_LABEL } from "./icons/symbolKind";
import { fileIcon, type SymbolKind } from "./solutionVisuals";

/**
 * Type icon for a path: the Material Icon Theme drawing for that name, in the theme's own colours.
 *
 * The drawing is one `<img>` of a vendored SVG, so the file kind (`data-file-kind`, on the row) and
 * the icon (`data-icon`) stay separate: several kinds share an icon, and one kind can draw more
 * than one (`.json` and `package.json`).
 *
 * C# symbol kinds remain in the tooltip and row description, not a visual badge.
 */
export function FileIcon({ path, symbolKind }: { path: string; symbolKind?: SymbolKind }) {
  const icon = fileIcon(path);
  return (
    <span
      className="file-icon"
      data-icon={icon}
      aria-hidden="true"
      title={icon === "csharp" ? (symbolKind ? `C# ${SYMBOL_LABEL[symbolKind]}` : "C# file") : undefined}
    >
      <img className="file-glyph" src={fileIconUrl(icon)} width={16} height={16} alt="" />
    </span>
  );
}
