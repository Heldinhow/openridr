import { useEffect, useState } from "react";
import { call, HostCallError, type ProcessInfo, type Snapshot } from "./host/client";

type Props = {
  snapshot: Snapshot;
  onChange: (snapshot: Snapshot) => void;
};

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

function stateText(process: ProcessInfo) {
  if (process.state === "running") {
    return process.awaitingDecision ? "running · awaiting decision" : "running";
  }
  return process.exit === null ? "exited · exit unknown" : `exited · exit ${process.exit}`;
}

export function ProcessPanel({ snapshot, onChange }: Props) {
  const [busy, setBusy] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const processes = snapshot.processes ?? [];
  const awaitingIds = processes
    .filter((process) => process.awaitingDecision)
    .map((process) => process.id)
    .sort((a, b) => a - b)
    .join(",");
  const awaitingDecision = awaitingIds.length > 0;
  const [expanded, setExpanded] = useState(awaitingDecision);

  useEffect(() => {
    if (awaitingIds) setExpanded(true);
  }, [awaitingIds]);

  if (processes.length === 0) {
    return null;
  }

  async function act(method: "processes.terminate" | "processes.keep", id: number) {
    setBusy(id);
    setError(null);
    try {
      onChange(await call<Snapshot>(method, { id }));
    } catch (caught) {
      setError(messageOf(caught));
      setExpanded(true);
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className={`process-panel ${expanded ? "is-expanded" : "is-collapsed"}`} aria-labelledby="processes-title" data-testid="processes-panel">
      <h2 id="processes-title">
        <button type="button" className="process-toggle" data-testid="processes-toggle" aria-expanded={expanded} onClick={() => setExpanded((open) => !open)}>
          Processes <span className="muted">· {processes.length}</span>
          {awaitingDecision && <span className="warning"> · Decision needed</span>}
        </button>
      </h2>
      {expanded && <>
      <p className="muted">
        OpenRidr tracks only processes the host started. Termination is shown after the host
        observes it; a missing exit code stays "exit unknown".
      </p>
      <ul className="process-list">
        {processes.map((process) => (
          <li className="process-row" key={process.id} data-testid={`processes-row-${process.id}`}>
            <div>
              <strong>{process.kind}</strong> <span className="muted">pid {process.pid}</span>{" "}
              <span className="tag" data-testid={`processes-state-${process.id}`}>
                {stateText(process)}
              </span>
            </div>
            {process.awaitingDecision && (
              <p className="warning" data-testid={`processes-awaiting-${process.id}`}>
                Awaiting your decision. The host has not confirmed this process ended.
              </p>
            )}
            {process.kept && (
              <p className="info" data-testid={`processes-kept-${process.id}`}>
                Kept running by your decision. OpenRidr makes no cleanup claim.
              </p>
            )}
            {process.termination && (
              <p className="muted" data-testid={`processes-termination-${process.id}`}>
                Termination requested: {process.termination.requested}. Observed:{" "}
                {process.termination.observed}
                {process.termination.killedAfterGrace ? " after the grace period" : ""}.
              </p>
            )}
            {process.state === "running" && (
              <div className="actions">
                <button
                  data-testid={`processes-terminate-${process.id}`}
                  onClick={() => void act("processes.terminate", process.id)}
                  disabled={busy === process.id}
                  title="Sends TERM, waits about 2 seconds, then KILL; reports what was observed."
                >
                  Terminate
                </button>
                <button
                  data-testid={`processes-keep-${process.id}`}
                  onClick={() => void act("processes.keep", process.id)}
                  disabled={busy === process.id}
                  title="Leaves the process running and keeps it visible."
                >
                  Keep running
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>
      {error && (
        <p className="error" role="alert" data-testid="processes-error">
          {error}
        </p>
      )}
      </>}
    </section>
  );
}
