import { useState } from "react";
import type { DapChildren, DapEvaluation, DapWatch } from "./host/client";
import { DebugVariable } from "./DebugVariable";

type Props = {
  paused: boolean;
  evaluation: DapEvaluation | null;
  watches: DapWatch[];
  expansions: DapChildren[];
  onEvaluate: (expression: string) => void;
  onAddWatch: (expression: string) => void;
  onRemoveWatch: (expression: string) => void;
  onExpand: (variablesReference: number) => void;
};

/** Expression box for the selected frame, plus the watch list that follows each pause. */
export function DebugEvaluate({
  paused,
  evaluation,
  watches,
  expansions,
  onEvaluate,
  onAddWatch,
  onRemoveWatch,
  onExpand,
}: Props) {
  const [expression, setExpression] = useState("");
  const [watchExpression, setWatchExpression] = useState("");

  return (
    <section aria-label="Evaluate and watches" data-testid="debug-evaluate">
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (expression.trim()) onEvaluate(expression);
        }}
      >
        <label>
          Evaluate expression{" "}
          <input
            value={expression}
            onChange={(event) => setExpression(event.target.value)}
            disabled={!paused}
            data-testid="debug-evaluate-input"
          />
        </label>{" "}
        <button type="submit" disabled={!paused} data-testid="debug-evaluate-run">
          Evaluate
        </button>
      </form>
      {!paused && <p className="muted">Expressions can be evaluated only while paused.</p>}
      {evaluation && (
        <div data-testid="debug-evaluate-result">
          {evaluation.error ? (
            <p className="warning">{evaluation.error}</p>
          ) : (
            <ul>
              <DebugVariable
                variable={{
                  name: evaluation.expression,
                  value: evaluation.value ?? "",
                  type: evaluation.typeName,
                  variablesReference: evaluation.variablesReference,
                }}
                testId="debug-evaluate-value"
                expansions={expansions}
                onExpand={onExpand}
              />
            </ul>
          )}
        </div>
      )}

      <h3>Watches</h3>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (watchExpression.trim()) {
            onAddWatch(watchExpression);
            setWatchExpression("");
          }
        }}
      >
        <label>
          Add watch{" "}
          <input
            value={watchExpression}
            onChange={(event) => setWatchExpression(event.target.value)}
            data-testid="debug-watch-input"
          />
        </label>{" "}
        <button type="submit" data-testid="debug-watch-add">
          Add
        </button>
      </form>
      {watches.length === 0 ? (
        <p className="muted" data-testid="debug-watches-empty">
          No watches.
        </p>
      ) : (
        <ul>
          {watches.map((watch, index) => (
            <li key={watch.expression} data-testid={`debug-watch-${index}`}>
              <code>{watch.expression}</code>
              {watch.result?.error ? (
                <span className="warning"> {watch.result.error}</span>
              ) : watch.result ? (
                <ul>
                  <DebugVariable
                    variable={{
                      name: "value",
                      value: watch.result.value ?? "",
                      type: watch.result.typeName,
                      variablesReference: watch.result.variablesReference,
                    }}
                    testId={`debug-watch-${index}-value`}
                    expansions={expansions}
                    onExpand={onExpand}
                  />
                </ul>
              ) : (
                <span className="muted"> not evaluated yet</span>
              )}{" "}
              <button
                type="button"
                onClick={() => onRemoveWatch(watch.expression)}
                data-testid={`debug-watch-remove-${index}`}
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
