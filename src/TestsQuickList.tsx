import { useEffect } from "react";

export type TestQuickItem = {
  id: string;
  title: string;
  shortcut: string;
  disabledReason: string | null;
  execute: () => void;
};

/** Numbered popup: digits run an available item, Escape closes. Unavailable items stay
 * visible, dimmed, with the reason that keeps them unavailable. */
export function TestsQuickList({ items, onClose }: { items: TestQuickItem[]; onClose: () => void }) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
      const index = Number(event.key) - 1;
      const item = items[index];
      if (!item) return;
      event.preventDefault();
      if (item.disabledReason) return;
      onClose();
      item.execute();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [items, onClose]);

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        className="modal quick-list"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tests-quick-list-title"
        data-testid="tests-quick-list"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <h2 id="tests-quick-list-title">Tests</h2>
        <p className="muted">
          Press 1–5 to act; items are dimmed with the reason when unavailable. Test debugging is
          not part of this version.
        </p>
        <ol className="quick-list-items">
          {items.map((item, index) => {
            const unavailable = item.disabledReason !== null;
            return (
              <li key={item.id}>
                <button
                  type="button"
                  data-testid={`tests-quick-list-item-${item.id}`}
                  disabled={unavailable}
                  title={item.disabledReason ?? item.title}
                  onClick={() => {
                    onClose();
                    item.execute();
                  }}
                >
                  <span className="quick-list-index" aria-hidden="true">
                    {index + 1}
                  </span>
                  <span className="quick-list-title">{item.title}</span>
                  <span className="quick-list-keys">{item.shortcut}</span>
                </button>
                {item.disabledReason && (
                  <p
                    className="muted quick-list-reason"
                    data-testid={`tests-quick-list-reason-${item.id}`}
                  >
                    {item.disabledReason}
                  </p>
                )}
              </li>
            );
          })}
        </ol>
        <div className="actions">
          <button type="button" data-testid="tests-quick-list-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
