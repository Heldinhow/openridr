import { useMemo, useState } from "react";
import { profileTitle, type ProfileId } from "./keymap";
import type { ResolvedAction } from "./registry";

export function FindAction({
  actions,
  platform,
  profile,
  onExecute,
  onClose,
}: {
  actions: ResolvedAction[];
  platform: string;
  profile: ProfileId;
  onExecute: (action: ResolvedAction) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return actions;
    return actions.filter((action) =>
      `${action.title} ${action.id} ${action.group}`.toLowerCase().includes(needle),
    );
  }, [actions, query]);
  const selected = matches[Math.min(active, matches.length - 1)] ?? null;

  function choose(action: ResolvedAction | null) {
    if (!action || action.disabledReason) return;
    onExecute(action);
  }

  return (
    <div className="modal-backdrop">
      <div
        className="modal find-action"
        role="dialog"
        aria-modal="true"
        aria-labelledby="find-action-title"
        data-testid="find-action"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <h2 id="find-action-title">Find Action</h2>
        <p className="muted" data-testid="find-action-profile">
          Keymap: {profileTitle(profile)} · {platform === "macos" ? "macOS" : "Windows"} keys
        </p>
        <input
          autoFocus
          data-testid="find-action-input"
          aria-label="Action name, id or group"
          placeholder="Type an action name, id or group"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((current) => Math.min(current + 1, matches.length - 1));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((current) => Math.max(current - 1, 0));
            } else if (event.key === "Enter") {
              event.preventDefault();
              choose(selected);
            }
          }}
        />
        <ul className="action-list" data-testid="find-action-list">
          {matches.map((action, index) => (
            <li
              key={action.id}
              className={`action-row ${index === active ? "active" : ""}`}
              data-testid={`find-action-item-${action.id}`}
              onMouseEnter={() => setActive(index)}
            >
              <button
                type="button"
                disabled={Boolean(action.disabledReason)}
                aria-current={index === active ? "true" : undefined}
                onClick={() => choose(action)}
                title={action.disabledReason ?? action.title}
              >
                <span className="action-title">{action.title}</span>
                <span className="muted action-group">{action.group}</span>
                <span className="action-keys" data-testid={`find-action-shortcut-${action.id}`}>
                  {action.shortcutLabels.length > 0
                    ? action.shortcutLabels.join(" / ")
                    : "No shortcut"}
                </span>
                {action.conflicts.length > 0 && (
                  <span
                    className="warning"
                    data-testid={`find-action-conflict-${action.id}`}
                    title={`Same shortcut as ${action.conflicts.join(", ")}`}
                  >
                    Conflict: {action.conflicts.join(", ")}
                  </span>
                )}
                {action.disabledReason && (
                  <span
                    className="muted action-reason"
                    data-testid={`find-action-reason-${action.id}`}
                  >
                    {action.disabledReason}
                  </span>
                )}
              </button>
            </li>
          ))}
          {matches.length === 0 && (
            <li className="muted" data-testid="find-action-empty">
              No actions match “{query}”.
            </li>
          )}
        </ul>
        <div className="actions">
          <span className="muted">Enter runs the selected action · Esc closes</span>
        </div>
      </div>
    </div>
  );
}
