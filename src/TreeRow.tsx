import type { MouseEvent, ReactNode } from "react";
import { Icon } from "./Icon";

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
  onClick: () => void;
  onContextMenu?: (event: MouseEvent) => void;
  children?: ReactNode;
};

/** One Rider-style tree row: chevron, icon, name and a dimmed suffix on a single line. */
export function TreeRow({
  level, open, icon, name, suffix, selected, current, noIndex, testId, title, onClick, onContextMenu, children,
}: Props) {
  return (
    <button
      type="button"
      className={`tree-row sol-row${selected ? " selected" : ""}${noIndex ? " no-index" : ""}`}
      style={{ paddingLeft: 8 + level * 18 }}
      data-testid={testId}
      aria-expanded={open}
      aria-current={current ? "page" : selected ? "true" : undefined}
      title={title}
      onClick={onClick}
      onContextMenu={onContextMenu}
    >
      <span className={`tree-chevron${open ? " open" : ""}${open === undefined ? " leaf" : ""}`} aria-hidden="true">
        {open !== undefined && <Icon name="chevron" />}
      </span>
      {icon}
      <span className="tree-name">{name}</span>
      {noIndex && <span className="tag">· no index</span>}
      {suffix}
      {children}
    </button>
  );
}
