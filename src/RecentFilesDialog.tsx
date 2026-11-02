import { useEffect, useRef, useState } from "react";

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

/** Session-only list: most recent first, no persistence. */
export function RecentFilesDialog({
  files,
  onOpen,
  onClose,
}: {
  files: string[];
  onOpen: (path: string) => void;
  onClose: () => void;
}) {
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);
  const selected = files[Math.min(active, files.length - 1)] ?? null;

  useEffect(() => {
    // autoFocus on a list is not applied by every browser; Esc must reach the dialog.
    listRef.current?.focus();
  }, []);

  return (
    <div className="modal-backdrop">
      <div
        className="modal find-action"
        role="dialog"
        aria-modal="true"
        aria-labelledby="recent-files-title"
        data-testid="recent-files-dialog"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            setActive((current) => Math.min(current + 1, Math.max(files.length - 1, 0)));
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setActive((current) => Math.max(current - 1, 0));
          } else if (event.key === "Enter") {
            event.preventDefault();
            if (selected) onOpen(selected);
          }
        }}
      >
        <h2 id="recent-files-title">Recent Files</h2>
        <ul className="action-list" data-testid="recent-files" ref={listRef} tabIndex={0} autoFocus>
          {files.map((path, index) => (
            <li
              key={path}
              className={`action-row ${path === selected ? "active" : ""}`}
              data-testid={`recent-file-${index}`}
              onMouseEnter={() => setActive(index)}
            >
              <button
                type="button"
                aria-current={path === selected ? "true" : undefined}
                onClick={() => onOpen(path)}
                title={path}
              >
                <span className="action-title">{baseName(path)}</span>
                <span className="muted action-group">{path}</span>
              </button>
            </li>
          ))}
          {files.length === 0 && (
            <li className="muted" data-testid="recent-files-empty">
              No recent files.
            </li>
          )}
        </ul>
      </div>
    </div>
  );
}
