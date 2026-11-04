import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { createPortal } from "react-dom";

export type ContextMenuItem = {
  id: string;
  label: string;
  shortcut?: string;
  disabledReason?: string | null;
  action: () => void;
  separatorBefore?: boolean;
  /** One level of submenu (Solution tree); the parent row then only opens it. */
  children?: ContextMenuItem[];
};

type Props = {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  /** Tree menus keep disabled reasons in the tooltip only, like the Rider menu. */
  compact?: boolean;
  testId?: string;
  itemPrefix?: string;
  /** Accessible name when "Solution context menu" would be wrong; visible labels stay put. */
  label?: string;
};

export function EditorContextMenu({
  x, y, items, onClose, compact = false, testId = "editor-context-menu", itemPrefix = "context-menu",
  label,
}: Props) {
  const menu = useRef<HTMLDivElement>(null);
  const [openSub, setOpenSub] = useState<string | null>(null);
  // Inline onClose callbacks must not re-run the focus effect on every parent render.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const first = menu.current?.querySelector<HTMLButtonElement>("button:not(:disabled)");
    (first ?? menu.current)?.focus();
    const onPointer = (event: MouseEvent) => {
      if (menu.current && !menu.current.contains(event.target as Node)) closeRef.current();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      closeRef.current();
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, []);

  function moveFocus(event: ReactKeyboardEvent<HTMLDivElement>) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const buttons = Array.from(
      menu.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [],
    );
    if (buttons.length === 0) return;
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next =
      event.key === "ArrowDown"
        ? (index + 1 + buttons.length) % buttons.length
        : (index - 1 + buttons.length) % buttons.length;
    buttons[next].focus();
  }

  function row(item: ContextMenuItem, index: number, nested = false) {
    const sub = item.children;
    return (
      <div key={item.id} className={sub ? "context-menu-parent" : undefined}>
        {item.separatorBefore && index > 0 && <div className="context-menu-separator" role="separator" />}
        <button
          type="button"
          role="menuitem"
          aria-haspopup={sub ? "menu" : undefined}
          aria-expanded={sub ? openSub === item.id : undefined}
          data-testid={`${itemPrefix}-${item.id}`}
          disabled={Boolean(item.disabledReason)}
          title={item.disabledReason ?? item.label}
          onMouseEnter={() => !nested && setOpenSub(sub ? item.id : null)}
          onKeyDown={(event) => {
            if (sub && event.key === "ArrowRight") {
              event.preventDefault();
              setOpenSub(item.id);
              requestAnimationFrame(() =>
                menu.current?.querySelector<HTMLButtonElement>(".context-submenu button:not(:disabled)")?.focus(),
              );
            } else if (nested && event.key === "ArrowLeft") {
              event.preventDefault();
              setOpenSub(null);
              (event.currentTarget.closest(".context-menu-parent")?.querySelector(":scope > button") as HTMLElement | null)?.focus();
            }
          }}
          onClick={() => {
            if (sub) {
              setOpenSub(item.id);
              return;
            }
            item.action();
            onClose();
          }}
        >
          <span className="context-menu-label">{item.label}</span>
          {sub && <span className="context-menu-shortcut" aria-hidden="true">›</span>}
          {item.shortcut && !item.disabledReason && !sub && (
            <span className="context-menu-shortcut">{item.shortcut}</span>
          )}
          {item.disabledReason && !compact && (
            <span className="context-menu-reason">{item.disabledReason}</span>
          )}
        </button>
        {sub && openSub === item.id && (
          <div className="editor-context-menu context-submenu" role="menu" aria-label={item.label}>
            {sub.map((child, childIndex) => row(child, childIndex, true))}
          </div>
        )}
      </div>
    );
  }

  const popup = (
    <div
      ref={menu}
      className={`editor-context-menu${items.some((item) => item.children) ? " has-submenu" : ""}`}
      role="menu"
      aria-label={label ?? (compact ? "Solution context menu" : "Editor context menu")}
      data-testid={testId}
      tabIndex={-1}
      style={{
        left: Math.max(8, Math.min(x, window.innerWidth - 320)),
        top: Math.max(8, Math.min(y, window.innerHeight - 420)),
      }}
      onKeyDown={moveFocus}
    >
      {items.map((item, index) => row(item, index))}
    </div>
  );
  // Tree menus leave the scrolling Solution panel so focusing them never scrolls it.
  return compact ? createPortal(popup, document.body) : popup;
}
