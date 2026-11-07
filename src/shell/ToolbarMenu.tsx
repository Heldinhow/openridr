import { useEffect, useRef, useState, type ReactNode } from "react";

type Props = {
  className: string;
  testId?: string;
  contentTestId?: string;
  triggerClassName?: string;
  triggerTestId?: string;
  /** The button's accessible name; also its tooltip unless `title` differs. */
  label: string;
  title?: string;
  /** Called on each open, for menus whose content needs a refresh. */
  onOpen?: () => void;
  trigger: ReactNode;
  /** The popover; receives the close callback so an item can dismiss the menu after acting. */
  children: (close: () => void) => ReactNode;
};

/**
 * A toolbar menu whose trigger is a real button: WKWebView exposes <summary> as a disclosure
 * without a ref_id, so a screen reader cannot open it by role and name.
 */
export function ToolbarMenu({
  className,
  testId,
  contentTestId,
  triggerClassName,
  triggerTestId,
  label,
  title,
  onOpen,
  trigger,
  children,
}: Props) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    function onPointerDown(event: PointerEvent) {
      if (!wrapRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  return (
    <div
      ref={wrapRef}
      className={className}
      data-testid={testId}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setOpen(false);
          triggerRef.current?.focus();
        }
      }}
    >
      <button
        ref={triggerRef}
        type="button"
        className={triggerClassName}
        data-testid={triggerTestId}
        aria-label={label}
        title={title ?? label}
        aria-expanded={open}
        onClick={() => {
          if (open) {
            setOpen(false);
            return;
          }
          onOpen?.();
          setOpen(true);
        }}
      >
        {trigger}
      </button>
      {open && (
        <div className="menu-content" data-testid={contentTestId}>
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  );
}
