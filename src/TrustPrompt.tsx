import { useEffect, useRef, useState } from "react";
import { call, HostCallError, type Snapshot } from "./host/client";
import { RESTRICTED_EXPLAIN, TRUST_EXPLAIN } from "./TrustControls";

type Props = {
  snapshot: Snapshot;
  onDecided: (snapshot: Snapshot) => void;
  onCancel: () => void;
};

export function TrustPrompt({ snapshot, onDecided, onCancel }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const trustButton = useRef<HTMLButtonElement>(null);
  const workspace = snapshot.workspace;

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    trustButton.current?.focus();
    return () => previous?.focus?.();
  }, []);

  async function decide(decision: "trusted" | "restricted") {
    if (!workspace) return;
    setBusy(true);
    setError(null);
    try {
      onDecided(await call<Snapshot>("trust.set", { path: workspace.path, decision }));
    } catch (caught) {
      setError(caught instanceof HostCallError ? caught.message : String(caught));
      setBusy(false);
    }
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    } else if (event.key === "Tab") {
      // Focus is trapped: Tab cycles through the three buttons only.
      const buttons = [...(dialog.current?.querySelectorAll("button:not(:disabled)") ?? [])] as HTMLElement[];
      if (buttons.length === 0) return;
      const index = buttons.indexOf(document.activeElement as HTMLElement);
      const next = (index + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length;
      event.preventDefault();
      buttons[next].focus();
    }
  }

  return (
    <div className="modal-backdrop">
      <div
        ref={dialog}
        className="modal trust-prompt-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="trust-prompt-title"
        aria-describedby="trust-prompt-body"
        data-testid="trust-prompt-dialog"
        onKeyDown={onKeyDown}
      >
        <h2 id="trust-prompt-title">Trust “{workspace?.name}”?</h2>
        <div id="trust-prompt-body">
          <p className="muted"><code data-testid="trust-prompt-path">{workspace?.path}</code></p>
          <p>{TRUST_EXPLAIN}</p>
          <p className="muted">Restricted mode: {RESTRICTED_EXPLAIN}</p>
          <p className="hint">You can change this later; the choice is remembered for this project.</p>
        </div>
        {error && <p className="error" role="alert" data-testid="trust-prompt-error">{error}</p>}
        <div className="actions">
          <button type="button" className="primary" ref={trustButton} onClick={() => void decide("trusted")} disabled={busy} data-testid="trust-prompt-trust">
            Trust project
          </button>
          <button type="button" onClick={() => void decide("restricted")} disabled={busy} data-testid="trust-prompt-restricted">
            Open in Restricted mode
          </button>
          <button type="button" onClick={onCancel} disabled={busy} data-testid="trust-prompt-cancel">Cancel</button>
        </div>
      </div>
    </div>
  );
}
