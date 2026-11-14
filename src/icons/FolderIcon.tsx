import { folderIcon } from "../solutionVisuals";
import { folderIconUrl } from "./iconAssets";

/**
 * Folder drawing from the Material Icon Theme, keyed by the folder's own name (`src`, `tests`,
 * `bin`, `.github`…) and drawn in two states, closed and open. A name the theme does not map gets
 * the generic folder — `obj` is one.
 */
export function FolderIcon({ name, open }: { name: string; open: boolean }) {
  const icon = folderIcon(name);
  return (
    <img
      className="folder-glyph"
      data-folder={open ? "open" : "closed"}
      data-folder-icon={icon}
      src={folderIconUrl(icon, open)}
      width={16}
      height={16}
      alt=""
      aria-hidden="true"
    />
  );
}
