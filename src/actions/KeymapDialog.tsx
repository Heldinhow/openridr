import { useEffect, useState } from "react";
import {
  profileOptions,
  profileTitle,
  shortcutFromEvent,
  type Keymap,
  type ProfileId,
} from "./keymap";
import type { ResolvedAction } from "./registry";

type PanelProps = {
  actions: ResolvedAction[];
  keymap: Keymap;
  path: string | null;
  platform: string;
  onChange: (keymap: Keymap) => Promise<string | null>;
};

/** Profile, remap, reset and conflicts; shared by the Keymap dialog and the Settings page. */
export function KeymapPanel({ actions, keymap, path, platform, onChange }: PanelProps) {
  const [capturing, setCapturing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function apply(next: Keymap) {
    const failure = await onChange(next);
    setError(failure);
    if (!failure) setCapturing(null);
  }

  useEffect(() => {
    if (!capturing) return;
    const capture = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setCapturing(null);
        return;
      }
      const shortcut = shortcutFromEvent(event, platform);
      if (!shortcut) return;
      void apply({
        ...keymap,
        overrides: { ...keymap.overrides, [capturing]: [shortcut] },
      });
    };
    window.addEventListener("keydown", capture, true);
    return () => window.removeEventListener("keydown", capture, true);
  }, [capturing, keymap, platform]);

  function reset(id: string) {
    const overrides = { ...keymap.overrides };
    delete overrides[id];
    void apply({ ...keymap, overrides });
  }

  return (
    <>
      <p className="muted">
        Shortcuts are stored in {path ?? "settings.json"} and win over the profile. Two
        actions on one shortcut are both kept; the first registered runs.
      </p>
      <label className="rename-field">
        <span>Profile</span>
        <select
          data-testid="keymap-profile"
          value={keymap.profile}
          onChange={(event) =>
            void apply({ ...keymap, profile: event.target.value as ProfileId })
          }
        >
          {profileOptions.map((option) => (
            <option key={option.id} value={option.id}>
              {option.title}
            </option>
          ))}
        </select>
      </label>
      <p className="muted" data-testid="keymap-active-profile">
        Active profile: {profileTitle(keymap.profile)}
      </p>
      {error && (
        <p className="error" role="alert" data-testid="keymap-error">
          {error}
        </p>
      )}
      <ul className="action-list keymap-list">
        {actions.map((action) => (
          <li
            className="action-row keymap-row"
            key={action.id}
            data-testid={`keymap-row-${action.id}`}
          >
            <span className="action-title">{action.title}</span>
            <span className="muted action-group">{action.group}</span>
            <span className="action-keys" data-testid={`keymap-shortcut-${action.id}`}>
              {action.shortcutLabels.length > 0
                ? action.shortcutLabels.join(" / ")
                : action.chordLabels.join(" ") || "No shortcut"}
            </span>
            {action.conflicts.length > 0 && (
              <span
                className="warning"
                data-testid={`keymap-conflict-${action.id}`}
                title={`Same shortcut as ${action.conflicts.join(", ")}`}
              >
                Conflict: {action.conflicts.join(", ")}
              </span>
            )}
            <button
              type="button"
              data-testid={`keymap-change-${action.id}`}
              aria-pressed={capturing === action.id}
              onClick={() => {
                setError(null);
                setCapturing(capturing === action.id ? null : action.id);
              }}
            >
              {capturing === action.id ? "Press keys…" : "Change"}
            </button>
            <button
              type="button"
              data-testid={`keymap-reset-${action.id}`}
              disabled={!keymap.overrides[action.id]}
              onClick={() => reset(action.id)}
            >
              Reset
            </button>
          </li>
        ))}
      </ul>
    </>
  );
}

export function KeymapDialog({
  actions,
  keymap,
  path,
  platform,
  onChange,
  onClose,
}: PanelProps & { onClose: () => void }) {
  return (
    <div className="modal-backdrop">
      <div
        className="modal keymap-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="keymap-title"
        data-testid="keymap-dialog"
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
        }}
      >
        <h2 id="keymap-title">Keymap</h2>
        <KeymapPanel
          actions={actions}
          keymap={keymap}
          path={path}
          platform={platform}
          onChange={onChange}
        />
        <div className="actions">
          <button type="button" data-testid="keymap-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
