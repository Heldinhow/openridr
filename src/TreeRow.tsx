import { useId, type ReactNode } from "react";
import { Icon } from "./Icon";

/** Viewport position a context menu opens at. Rows compute it for the mouse and the keyboard. */
export type MenuPosition = { x: number; y: number };

type Props = {
  level: number;
  /** undefined = leaf (no chevron); boolean = expandable and open/closed. */
  open?: boolean;
  /** Omit for folders; the row supplies their folder glyph. */
  icon?: ReactNode;
  name: string;
  /** Accessible description read after the name, for information the row only shows as a glyph. */
  description?: string;
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
  /** The entry itself is a symbolic link; the target is never followed by the tree. */
  symlink?: boolean;
  /** Git state of the file; the word tag is the non-colour channel, the colour is reinforcement. */
  gitStatus?: string;
  /** The commit tool's granular label, shown as the tag's tooltip. */
  gitLabel?: string;
  onClick: () => void;
  /** Opens the row's menu; a row without one still cancels the browser menu. */
  onMenu?: (position: MenuPosition, row: HTMLElement) => void;
  children?: ReactNode;
};

/** Row indent per level and the left padding of the whole row; the guides mirror both. */
export const ROW_INDENT = 18;
export const ROW_PADDING = 8;

/**
 * One Rider-style tree row: chevron, icon, name and a dimmed suffix on a single line.
 *
 * Every row is the single place that decides about the context menu: a right-click, Shift+F10 and
 * the menu key all call `onMenu`, and the WebKit menu is always prevented here so no row can fall
 * back to "Reload / Share… / Services" (see nativeMenu.ts for the app-wide guard).
 */
export function TreeRow({
  level, open, icon, name, description, suffix, selected, current, noIndex, testId, title,
  fileKind, projectKind, projectTfms, symbolKind, symlink, gitStatus, gitLabel, onClick, onMenu, children,
}: Props) {
  // The description lives outside the row: `aria-describedby` then adds it to the description
  // without its text also becoming part of the button's accessible name.
  const descriptionId = useId();
  return (
    <>
    <button
      type="button"
      className={`tree-row sol-row${selected ? " selected" : ""}${noIndex ? " no-index" : ""}`}
      style={{ paddingLeft: ROW_PADDING + level * ROW_INDENT }}
      data-level={level}
      data-testid={testId}
      data-file-kind={fileKind}
      data-project-kind={projectKind}
      data-project-tfms={projectTfms}
      data-symbol-kind={symbolKind}
      data-symlink={symlink ? "true" : undefined}
      data-git-status={gitStatus}
      aria-expanded={open}
      aria-current={current ? "page" : selected ? "true" : undefined}
      aria-describedby={description ? descriptionId : undefined}
      title={title}
      onClick={onClick}
      onContextMenu={(event) => {
        event.preventDefault();
        // WKWebView does not move the focus on a right-click, so the row itself is what the menu
        // hands the focus back to when it closes.
        onMenu?.({ x: event.clientX, y: event.clientY }, event.currentTarget);
      }}
      onKeyDown={(event) => {
        // WebKit turns neither Shift+F10 nor the menu key into a contextmenu event, so the row
        // opens the menu itself, anchored on its own rectangle.
        if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
        event.preventDefault();
        const rect = event.currentTarget.getBoundingClientRect();
        onMenu?.({ x: rect.left + 16, y: rect.bottom - 4 }, event.currentTarget);
      }}
    >
      <span className={`tree-chevron${open ? " open" : ""}${open === undefined ? " leaf" : ""}`} aria-hidden="true">
        {open !== undefined && <Icon name="chevron" />}
      </span>
      {/* One hairline per expanded ancestor, in the chevron column of that ancestor. */}
      {Array.from({ length: Math.max(0, level) }, (_, index) => (
        <span
          key={`guide-${index}`}
          className="tree-guide"
          style={{ left: ROW_PADDING + index * ROW_INDENT + 6 }}
          aria-hidden="true"
        />
      ))}
      <span className="tree-icon">
        {!icon && open !== undefined ? (
          <span className="solution-folder-icon" aria-hidden="true">
            <Icon name="folder" />
            {name.toLowerCase() === "config" && <span className="folder-config-badge">↗</span>}
          </span>
        ) : testId?.startsWith("tree-deps-") && !testId.startsWith("tree-deps-group-") ? (
          <svg className="dependency-glyph" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M7 5.5 3.6 10M9 5.5l3.4 4.5" stroke="currentColor" strokeWidth="1.3" />
            <circle cx="8" cy="3.5" r="2" stroke="currentColor" strokeWidth="1.3" />
            <circle cx="3" cy="12" r="2" stroke="currentColor" strokeWidth="1.3" />
            <circle cx="13" cy="12" r="2" stroke="currentColor" strokeWidth="1.3" />
          </svg>
        ) : testId?.startsWith("tree-framework-") || (testId?.startsWith("tree-deps-group-") && testId.endsWith("-frameworks")) ? (
          <Icon name="settings" className="framework-glyph" />
        ) : icon}
        {symlink && (
          <span className="symlink-mark" aria-hidden="true">
            <Icon name="link" />
          </span>
        )}
      </span>
      <span className="tree-name">{name}</span>
      {noIndex && <span className="tag">· no index</span>}
      {gitStatus && (
        <span className="tag git-status" title={gitLabel}>
          · {gitStatus}
        </span>
      )}
      {suffix}
      {children}
    </button>
    {description && (
      <span id={descriptionId} className="sr-only">
        {description}
      </span>
    )}
    </>
  );
}
