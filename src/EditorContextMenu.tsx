import { useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";

export type ContextMenuItem = {
  id: string;
  label: string;
  shortcut?: string;
  disabledReason?: string | null;
  action: () => void;
  separatorBefore?: boolean;
};

type Props = {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
};

export function EditorContextMenu({ x, y, items, onClose }: Props) {
  const menu = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const first = menu.current?.querySelector<HTMLButtonElement>("button:not(:disabled)");
    (first ?? menu.current)?.focus();
    const onPointer = (event: MouseEvent) => {
      if (menu.current && !menu.current.contains(event.target as Node)) onClose();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      onClose();
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

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

  return (
    <div
      ref={menu}
      className="editor-context-menu"
      role="menu"
      aria-label="Editor context menu"
      data-testid="editor-context-menu"
      tabIndex={-1}
      style={{
        left: Math.max(8, Math.min(x, window.innerWidth - 320)),
        top: Math.max(8, Math.min(y, window.innerHeight - 420)),
      }}
      onKeyDown={moveFocus}
    >
      {items.map((item, index) => (
        <div key={item.id}>
          {item.separatorBefore && index > 0 && (
            <div className="context-menu-separator" role="separator" />
          )}
          <button
            type="button"
            role="menuitem"
            data-testid={`context-menu-${item.id}`}
            disabled={Boolean(item.disabledReason)}
            title={item.disabledReason ?? item.label}
            onClick={() => {
              item.action();
              onClose();
            }}
          >
            <span className="context-menu-label">{item.label}</span>
            {item.shortcut && !item.disabledReason && (
              <span className="context-menu-shortcut">{item.shortcut}</span>
            )}
            {item.disabledReason && (
              <span className="context-menu-reason">{item.disabledReason}</span>
            )}
          </button>
        </div>
      ))}
    </div>
  );
}
