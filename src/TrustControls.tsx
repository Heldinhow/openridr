import { useState } from "react";
import { call, messageOf, type Snapshot } from "./host/client";

type Props = {
  snapshot: Snapshot;
  onChange: (snapshot: Snapshot) => void;
  compact?: boolean;
};

// Shared with the Trust prompt so the explanation of what trust allows lives in one place.
export const RESTRICTED_EXPLAIN = "Files can be read as text. Project code will not run.";
export const TRUST_EXPLAIN =
  "Trust authorizes executable inspection of this solution and of the project directories it references.";

export function TrustControls({ snapshot, onChange, compact = false }: Props) {
  const [checked, setChecked] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const workspace = snapshot.workspace;

  async function grant() {
    setBusy(true);
    setError(null);
    try {
      onChange(await call<Snapshot>("trust.grant", { remember: true }));
      setChecked(false);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function revoke() {
    setBusy(true);
    setError(null);
    try {
      onChange(await call<Snapshot>("trust.revoke"));
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  const errorLine = error && (
    <p className="error" role="alert" data-testid="trust-error">
      {error}
    </p>
  );

  if (snapshot.trust.granted) {
    return (
      <div className={`trust ${compact ? "compact" : ""}`} data-testid="trust-panel">
        <strong className="success" data-testid="mode-trusted">
          Trusted mode
        </strong>
        {compact ? (
          <span className="muted">
            {" "}
            Executable inspection is authorized for this solution and the project directories it
            references.{" "}
          </span>
        ) : (
          <p>
            Executable inspection is authorized for this solution and the project directories it
            references.
          </p>
        )}
        <p className="hint" data-testid="trust-note">
          Trust is remembered for this project path, also after restarting the app. Revoking forgets it and clears the executable
          context immediately; it does not install tools or enable experimental debuggers.
        </p>
        <div className="actions">
          <button
            className="danger"
            data-testid="trust-revoke"
            onClick={() => void revoke()}
            disabled={busy}
          >
            {busy ? "Revoking…" : "Revoke trust"}
          </button>
        </div>
        {errorLine}
      </div>
    );
  }

  const reason = !workspace
    ? "Open a workspace before trusting it."
    : !checked
      ? "Select the trust checkbox to enable this action."
      : TRUST_EXPLAIN;

  return (
    <div className={`trust ${compact ? "compact" : ""}`} data-testid="trust-panel">
      <strong className="warning" data-testid="mode-restricted">
        Restricted mode
      </strong>
      {compact ? (
        <span className="muted"> {RESTRICTED_EXPLAIN}</span>
      ) : (
        <p>{RESTRICTED_EXPLAIN}</p>
      )}
      <label className="checkbox-label">
        <input
          type="checkbox"
          data-testid="trust-checkbox"
          checked={checked}
          disabled={!workspace || busy}
          onChange={(event) => setChecked(event.target.checked)}
        />
        I trust this repository to execute code
      </label>
      <p className="hint" id="trust-grant-reason" data-testid="trust-grant-reason">
        {reason}
      </p>
      <div className="actions">
        <button
          type="button"
          className="primary"
          data-testid="trust-grant"
          disabled={!workspace || !checked || busy}
          aria-describedby="trust-grant-reason"
          onClick={() => void grant()}
        >
          {busy ? "Trusting…" : "Trust and inspect project"}
        </button>
      </div>
      {errorLine}
    </div>
  );
}
