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
  platform: string;
  onChange: (keymap: Keymap) => Promise<string | null>;
};

/** Titles repeat across groups ("Run", "Save"); the group, or the id when it equals the
 * title, keeps each row and its buttons distinct for screen readers. */
function accessibleName(action: ResolvedAction) {
  return action.group !== action.title
    ? `${action.title} — ${action.group}`
    : `${action.title} — ${action.id}`;
}

/** Profile, remap, reset and conflicts; shared by the Keymap dialog and the Settings page. */
export function KeymapPanel({ actions, keymap, platform, onChange }: PanelProps) {
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
        Custom shortcuts override the profile. Conflicts keep both actions; the first registered runs.
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
      <div className="keymap-table-scroll">
        <table className="keymap-list" data-testid="keymap-table">
          <thead>
            <tr><th scope="col">Action</th><th scope="col">Group / shortcut</th><th scope="col">Actions</th></tr>
          </thead>
          <tbody>
        {actions.map((action) => (
          <tr
            className="keymap-row"
            key={action.id}
            data-testid={`keymap-row-${action.id}`}
          >
            <th scope="row" aria-label={accessibleName(action)}>{action.title}</th>
            <td>
              <span className="keymap-group">{action.group === action.title ? "—" : action.group}</span>
              <span className="action-keys" data-testid={`keymap-shortcut-${action.id}`}>
                {action.shortcutLabels.length > 0
                  ? action.shortcutLabels.join(" / ")
                  : action.chordLabels.join(" ") || "No shortcut"}
              </span>
              {action.conflicts.length > 0 && (
                <span
                  className="warning keymap-conflict"
                  data-testid={`keymap-conflict-${action.id}`}
                  title={`Same shortcut as ${action.conflicts.join(", ")}`}
                >
                  Conflict: {action.conflicts.join(", ")}
                </span>
              )}
            </td>
            <td className="keymap-actions">
              <button
                type="button"
                data-testid={`keymap-change-${action.id}`}
                aria-pressed={capturing === action.id}
                aria-label={`${capturing === action.id ? "Choose shortcut for" : "Change shortcut for"} ${accessibleName(action)}`}
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
                aria-label={`Reset shortcut for ${accessibleName(action)}`}
                disabled={!keymap.overrides[action.id]}
                onClick={() => reset(action.id)}
              >
                Reset
              </button>
            </td>
          </tr>
        ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

export function KeymapDialog({
  actions,
  keymap,
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
