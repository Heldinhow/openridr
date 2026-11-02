import { useRef, useState } from "react";
import { EditorContextMenu, type ContextMenuItem } from "./EditorContextMenu";
import { Icon } from "./Icon";

type Props = {
  showAll: boolean;
  searchOpen: boolean;
  locateDisabled: boolean;
  moreItems: ContextMenuItem[];
  onToggleShowAll: () => void;
  onToggleSearch: () => void;
  onLocate: () => void;
  onExpandAll: () => void;
  onCollapseAll: () => void;
  onMinimize: () => void;
};

/** Rider's Solution header tools: show all files, search, locate, expand, collapse, ⋮, minimize. */
export function SolutionToolbar(p: Props) {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const more = useRef<HTMLButtonElement>(null);
  return (
    <>
    <span className="explorer-tools">
      <button data-testid="tree-show-all" aria-label="Show All Files" aria-pressed={p.showAll} title="Show All Files" className={p.showAll ? "active" : undefined} onClick={p.onToggleShowAll}><Icon name="eye" /></button>
      <button data-testid="tree-search" aria-label="Search in tree" aria-pressed={p.searchOpen} title="Search in tree" className={p.searchOpen ? "active" : undefined} onClick={p.onToggleSearch}><Icon name="search" /></button>
      <button data-testid="tree-locate" aria-label="Select opened file" title="Select opened file" disabled={p.locateDisabled} onClick={p.onLocate}><Icon name="locate" /></button>
      <button data-testid="tree-expand-all" aria-label="Expand all" title="Expand all" onClick={p.onExpandAll}><Icon name="expandAll" /></button>
      <button data-testid="tree-collapse-all" aria-label="Collapse all" title="Collapse all" onClick={p.onCollapseAll}><Icon name="collapseAll" /></button>
      <button
        ref={more}
        data-testid="tree-more"
        aria-label="Options"
        aria-haspopup="menu"
        aria-expanded={menu !== null}
        title="Options"
        onMouseDown={(event) => event.stopPropagation()}
        onClick={() => {
          const box = more.current?.getBoundingClientRect();
          setMenu(menu ? null : { x: box?.left ?? 0, y: (box?.bottom ?? 0) + 2 });
        }}
      ><Icon name="more" /></button>
      <button data-testid="tree-minimize" aria-label="Hide Solution" title="Hide" onClick={p.onMinimize}><Icon name="minimize" /></button>
    </span>
    {menu && (
      <EditorContextMenu compact testId="tree-more-menu" itemPrefix="tree-more" x={menu.x} y={menu.y} items={p.moreItems} onClose={() => setMenu(null)} />
    )}
    </>
  );
}
