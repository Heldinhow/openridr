import { useEffect, useState } from "react";
import type { DapBreakpoint } from "./host/client";

type Props = {
  breakpoint: DapBreakpoint;
  /** Why conditions cannot be edited, or null when the adapter accepts them. */
  conditionDisabledReason: string | null;
  busy: boolean;
  onSaveCondition: (condition: string) => void;
  onRemove: () => void;
};

/** One breakpoint in the Debug panel: state, condition editor and removal. */
export function DebugBreakpoint({
  breakpoint,
  conditionDisabledReason,
  busy,
  onSaveCondition,
  onRemove,
}: Props) {
  const key = `${breakpoint.path}:${breakpoint.line}`;
  const saved = breakpoint.condition ?? "";
  const [draft, setDraft] = useState(saved);

  useEffect(() => setDraft(saved), [saved]);

  return (
    <li data-testid={`debug-breakpoint-${key}`}>
      <code>{key}</code>{" "}
      <span className="muted">
        {breakpoint.verified
          ? "verified by the adapter"
          : `pending${breakpoint.message ? ` · ${breakpoint.message}` : ""}`}
      </span>
      <form
        className="debug-breakpoint-condition"
        onSubmit={(event) => {
          event.preventDefault();
          onSaveCondition(draft.trim());
        }}
      >
        <input
          aria-label={`Condition for ${key}`}
          data-testid={`debug-breakpoint-condition-${key}`}
          placeholder="Condition, e.g. index == 3"
          value={draft}
          disabled={conditionDisabledReason !== null || busy}
          title={conditionDisabledReason ?? "Stops only when this C# expression is true."}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button
          type="submit"
          data-testid={`debug-breakpoint-condition-save-${key}`}
          disabled={conditionDisabledReason !== null || busy || draft.trim() === saved}
        >
          Save condition
        </button>
      </form>
      <button
        type="button"
        data-testid={`debug-breakpoint-remove-${key}`}
        disabled={busy}
        onClick={onRemove}
      >
        Remove
      </button>
    </li>
  );
}
