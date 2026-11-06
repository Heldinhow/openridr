import { useEffect, useState } from "react";
import type { DapBreakpoint } from "./host/client";

type Props = {
  breakpoint: DapBreakpoint;
  /** Why conditions cannot be edited, or null when the adapter accepts them. */
  conditionDisabledReason: string | null;
  busy: boolean;
  onSaveCondition: (condition: string) => void;
  /** Raw text from the hit count field; the panel validates and converts it. */
  onSaveHitCount: (hitCount: string) => void;
  onRemove: () => void;
};

/** One breakpoint in the Debug panel: state, condition, hit count and removal. */
export function DebugBreakpoint({
  breakpoint,
  conditionDisabledReason,
  busy,
  onSaveCondition,
  onSaveHitCount,
  onRemove,
}: Props) {
  const key = `${breakpoint.path}:${breakpoint.line}`;
  const saved = breakpoint.condition ?? "";
  const savedHit = breakpoint.hitCount === null ? "" : String(breakpoint.hitCount);
  const [draft, setDraft] = useState(saved);
  const [hitDraft, setHitDraft] = useState(savedHit);

  useEffect(() => setDraft(saved), [saved]);
  useEffect(() => setHitDraft(savedHit), [savedHit]);

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
      <form
        className="debug-breakpoint-hit"
        onSubmit={(event) => {
          event.preventDefault();
          onSaveHitCount(hitDraft.trim());
        }}
      >
        <input
          aria-label={`Hit count for ${key}`}
          data-testid={`debug-breakpoint-hit-${key}`}
          placeholder="Hit count, e.g. 3"
          inputMode="numeric"
          value={hitDraft}
          disabled={busy}
          title="Stops on the Nth counted hit. Empty stops on every hit."
          onChange={(event) => setHitDraft(event.target.value)}
        />
        <button
          type="submit"
          data-testid={`debug-breakpoint-hit-save-${key}`}
          disabled={busy || hitDraft.trim() === savedHit}
        >
          Save hit count
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
