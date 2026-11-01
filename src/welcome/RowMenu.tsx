import { useEffect, useRef, useState } from "react";
import type { Decision } from "./recent";

type Props = {
  testId: string;
  name: string;
  decision: Decision;
  disabled: boolean;
  onTrust: (decision: "trusted" | "restricted") => void;
  onRemove: () => void;
};

// The "⋯" menu of a recent project: change its Trust decision without opening it, or drop it from the list.
export function RowMenu({ testId, name, decision, disabled, onTrust, onRemove }: Props) {
  const [open, setOpen] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    wrap.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
    function onPointerDown(event: MouseEvent) {
      if (!wrap.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  function close() {
    setOpen(false);
    button.current?.focus();
  }

  function pick(action: () => void) {
    close();
    action();
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      event.stopPropagation();
      const items = [...(wrap.current?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? [])];
      const index = items.indexOf(document.activeElement as HTMLElement);
      items[(index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length]?.focus();
    } else if (event.key === "Tab") {
      setOpen(false);
    }
  }

  return (
    <div className="row-menu" ref={wrap} onKeyDown={onKeyDown}>
      <button
        ref={button}
        type="button"
        className="row-menu-button"
        aria-label={`Actions for ${name}`}
        aria-haspopup="menu"
        aria-expanded={open}
        data-testid={testId}
        disabled={disabled}
        onClick={() => setOpen((current) => !current)}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <circle cx="3" cy="8" r="1.4" /><circle cx="8" cy="8" r="1.4" /><circle cx="13" cy="8" r="1.4" />
        </svg>
      </button>
      {open && (
        <div className="row-menu-popover" role="menu" aria-label={`Actions for ${name}`}>
          {decision !== "trusted" && (
            <button type="button" role="menuitem" data-testid={`${testId}-trust`} onClick={() => pick(() => onTrust("trusted"))}>
              Trust
            </button>
          )}
          {decision !== "restricted" && (
            <button type="button" role="menuitem" data-testid={`${testId}-restrict`} onClick={() => pick(() => onTrust("restricted"))}>
              Restrict
            </button>
          )}
          <button type="button" role="menuitem" data-testid={`${testId}-remove`} onClick={() => pick(onRemove)}>
            Remove from list
          </button>
        </div>
      )}
    </div>
  );
}
