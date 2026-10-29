import { useEffect, useMemo, useRef } from "react";
import type { ReferencesResponse } from "./host/client";

type Props = {
  response: ReferencesResponse;
  snippets: Record<string, string | null>;
  onNavigate: (path: string, position: { line: number; character: number }) => void;
  onClose: () => void;
};

/** Read-only list grouped by file. The host already resolved every range; clicking only
 * navigates and never edits a buffer. */
export function FindUsagesPanel({ response, snippets, onNavigate, onClose }: Props) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, [response]);

  const groups = useMemo(() => {
    const map = new Map<string, ReferencesResponse["references"]>();
    for (const reference of response.references) {
      map.set(reference.path, [...(map.get(reference.path) ?? []), reference]);
    }
    return [...map.entries()];
  }, [response]);

  return (
    <section
      className="rename-preview find-usages"
      role="dialog"
      aria-modal="false"
      aria-labelledby="find-usages-title"
      data-testid="find-usages"
      data-state={response.state}
      data-count={response.referenceCount}
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
    >
      <h2 id="find-usages-title" ref={heading} tabIndex={-1}>
        Find Usages
      </h2>
      <p className="muted" data-testid="find-usages-summary">
        {response.state === "ok"
          ? `${response.referenceCount} reference${response.referenceCount === 1 ? "" : "s"} · session ${response.session} · ${response.path}:${response.line + 1}:${response.character + 1}`
          : `Find Usages ${response.state}: ${response.message ?? "csharp-ls did not answer."}`}
      </p>
      {response.truncated && (
        <p className="muted" data-testid="find-usages-truncated">
          Showing {response.referenceCount} of {response.totalReferences} references returned by
          csharp-ls.
        </p>
      )}
      {response.rejectedReferences > 0 && (
        <p className="muted" data-testid="find-usages-rejected">
          {response.rejectedReferences} reference(s) were rejected because they are outside the
          workspace, missing or malformed.
        </p>
      )}
      {groups.length === 0 ? (
        <p className="muted" data-testid="find-usages-empty">
          {response.message ?? "No references were returned for this position."}
        </p>
      ) : (
        <div className="find-usages-groups" data-testid="find-usages-list">
          {groups.map(([path, references]) => (
            <section key={path} className="find-usages-file">
              <h3 data-testid={`find-usages-file-${path}`}>
                {path} <span className="muted">({references.length})</span>
              </h3>
              <ul>
                {references.map((reference, index) => {
                  const key = `${reference.path}:${reference.line}`;
                  const snippet = snippets[key];
                  return (
                    <li key={`${key}:${reference.character}:${index}`}>
                      <button
                        type="button"
                        data-testid={`find-usage-${index}`}
                        title={`${reference.path}:${reference.line + 1}:${reference.character + 1}`}
                        onClick={() =>
                          onNavigate(reference.path, {
                            line: reference.line,
                            character: reference.character,
                          })
                        }
                      >
                        <span className="find-usage-line">{reference.line + 1}</span>
                        <span className="find-usage-snippet">
                          {snippet === null || snippet === ""
                            ? "(line not readable)"
                            : snippet}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </div>
      )}
      <div className="actions">
        <button type="button" data-testid="find-usages-close" onClick={onClose}>
          Close
        </button>
      </div>
    </section>
  );
}
