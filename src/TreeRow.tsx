import type { ReactNode } from "react";
import { Icon } from "./Icon";

/** Viewport position a context menu opens at. Rows compute it for the mouse and the keyboard. */
export type MenuPosition = { x: number; y: number };

type Props = {
  level: number;
  /** undefined = leaf (no chevron); boolean = expandable and open/closed. */
  open?: boolean;
  icon: ReactNode;
  name: string;
  /** Dim text after the name, e.g. "· 2 projects". */
  suffix?: ReactNode;
  selected?: boolean;
  current?: boolean;
  noIndex?: boolean;
  testId?: string;
  title?: string;
  /** Contract for tests/verification: the visual kind, never used for layout. */
  fileKind?: string;
  projectKind?: string;
  projectTfms?: string;
  symbolKind?: string;
  onClick: () => void;
  /** Opens the row's menu; a row without one still cancels the browser menu. */
  onMenu?: (position: MenuPosition) => void;
  children?: ReactNode;
};

/**
 * One Rider-style tree row: chevron, icon, name and a dimmed suffix on a single line.
 *
 * Every row is the single place that decides about the context menu: a right-click, Shift+F10 and
 * the menu key all call `onMenu`, and the WebKit menu is always prevented here so no row can fall
 * back to "Reload / Share… / Services" (see nativeMenu.ts for the app-wide guard).
 */
export function TreeRow({
  level, open, icon, name, suffix, selected, current, noIndex, testId, title,
  fileKind, projectKind, projectTfms, symbolKind, onClick, onMenu, children,
}: Props) {
  function menuAt(position: MenuPosition) {
    onMenu?.(position);
  }

  return (
    <button
      type="button"
      className={`tree-row sol-row${selected ? " selected" : ""}${noIndex ? " no-index" : ""}`}
      style={{ paddingLeft: 8 + level * 18 }}
      data-testid={testId}
      data-file-kind={fileKind}
      data-project-kind={projectKind}
      data-project-tfms={projectTfms}
      data-symbol-kind={symbolKind}
      aria-expanded={open}
      aria-current={current ? "page" : selected ? "true" : undefined}
      title={title}
      onClick={onClick}
      onContextMenu={(event) => {
        event.preventDefault();
        menuAt({ x: event.clientX, y: event.clientY });
      }}
      onKeyDown={(event) => {
        // WebKit turns neither Shift+F10 nor the menu key into a contextmenu event, so the row
        // opens the menu itself, anchored on its own rectangle.
        if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
        event.preventDefault();
        const rect = event.currentTarget.getBoundingClientRect();
        menuAt({ x: rect.left + 16, y: rect.bottom - 4 });
      }}
    >
      <span className={`tree-chevron${open ? " open" : ""}${open === undefined ? " leaf" : ""}`} aria-hidden="true">
        {open !== undefined && <Icon name="chevron" />}
      </span>
      <span className="tree-icon">{icon}</span>
      <span className="tree-name">{name}</span>
      {noIndex && <span className="tag">· no index</span>}
      {suffix}
      {children}
    </button>
  );
}
