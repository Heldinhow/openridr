import { useState } from "react";
import type { DapChildren, DapVariable } from "./host/client";

type Props = {
  variable: DapVariable;
  testId: string;
  expansions: DapChildren[];
  onExpand: (variablesReference: number) => void;
};

/** One variable row. Expandable rows fetch their children on first open, once per pause. */
export function DebugVariable({ variable, testId, expansions, onExpand }: Props) {
  const [open, setOpen] = useState(false);
  const expandable = variable.variablesReference > 0;
  const loaded = expansions.find(
    (expansion) => expansion.variablesReference === variable.variablesReference,
  );

  function toggle() {
    const next = !open;
    setOpen(next);
    if (next && !loaded) onExpand(variable.variablesReference);
  }

  return (
    <li data-testid={testId}>
      {expandable ? (
        <button
          type="button"
          aria-expanded={open}
          aria-label={`Expand ${variable.name}`}
          data-testid={`${testId}-toggle`}
          onClick={toggle}
        >
          {open ? "▾" : "▸"}
        </button>
      ) : null}
      <code>{variable.name}</code>
      {variable.type ? <span className="muted"> · {variable.type}</span> : null}
      {" = "}
      <span>{variable.value}</span>
      {open && !loaded ? <span className="muted"> loading…</span> : null}
      {open && loaded?.error ? (
        <p className="warning" data-testid={`${testId}-error`}>
          Could not read the children: {loaded.error}
        </p>
      ) : null}
      {open && loaded && !loaded.error ? (
        <ul>
          {loaded.variables.map((child, position) => (
            <DebugVariable
              key={`${child.name}-${position}`}
              variable={child}
              testId={`${testId}-${position}`}
              expansions={expansions}
              onExpand={onExpand}
            />
          ))}
          {loaded.variables.length === 0 && <li className="muted">No children.</li>}
        </ul>
      ) : null}
    </li>
  );
}
