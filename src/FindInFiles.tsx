import { useEffect, useMemo, useRef, useState } from "react";
import { FileIcon } from "./FileIcon";
import { call, messageOf, type TextSearch } from "./host/client";

type LineHit = { path: string; line: number; character: number; text: string };

function useDebounced(value: string, delay: number) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delay);
    return () => window.clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

/** One entry per file and line; the first match keeps the column to navigate to. */
function lineHits(text: TextSearch | null, scope: string): LineHit[] {
  if (!text) return [];
  const hits: LineHit[] = [];
  for (const file of text.files) {
    if (scope && file.path !== scope && !file.path.startsWith(`${scope}/`)) continue;
    const seen = new Set<number>();
    for (const match of file.matches) {
      if (seen.has(match.line)) continue;
      seen.add(match.line);
      hits.push({
        path: file.path,
        line: match.line,
        character: match.character,
        text: match.text,
      });
    }
  }
  return hits;
}

export function FindInFiles({
  scope,
  onNavigate,
  onClose,
}: {
  scope: string;
  onNavigate: (path: string, line: number, character: number) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [text, setText] = useState<TextSearch | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [active, setActive] = useState(0);
  const debounced = useDebounced(query, 180);
  const request = useRef(0);

  useEffect(() => {
    if (debounced.trim().length < 2) {
      setLoading(false);
      setText(null);
      setError(null);
      return;
    }
    let alive = true;
    const seq = ++request.current;
    setLoading(true);
    setText(null);
    setError(null);
    call<TextSearch>("search.text", { query: debounced, caseSensitive })
      .then((result) => {
        if (alive && seq === request.current) setText(result);
      })
      .catch((failure) => {
        if (alive && seq === request.current) setError(messageOf(failure));
      })
      .finally(() => {
        if (alive && seq === request.current) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [debounced, caseSensitive]);

  const hits = useMemo(() => lineHits(text, scope), [text, scope]);
  const selected = hits[Math.min(active, hits.length - 1)] ?? null;

  useEffect(() => {
    setActive(0);
  }, [debounced, caseSensitive]);

  function choose(hit: LineHit | null) {
    if (hit) onNavigate(hit.path, hit.line, hit.character);
  }

  const shortQuery = query.trim().length < 2;
  const status = error
    ? `Find in Files is unavailable: ${error}`
    : loading
      ? "Searching…"
      : shortQuery
        ? "Type at least two characters to search the workspace."
        : `${hits.length} result${hits.length === 1 ? "" : "s"}`;

  return (
    <div className="modal-backdrop">
      <div
        className="modal find-action"
        role="dialog"
        aria-modal="true"
        aria-labelledby="find-in-files-title"
        data-testid="find-in-files"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <h2 id="find-in-files-title">Find in Files</h2>
        <p className="muted" data-testid="find-in-files-scope" title={scope || "Workspace root"}>
          Scope: {scope || "Workspace root"}
        </p>
        <input
          autoFocus
          data-testid="find-in-files-input"
          aria-label="Search text in files"
          placeholder="Search text"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((current) => Math.min(current + 1, Math.max(hits.length - 1, 0)));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((current) => Math.max(current - 1, 0));
            } else if (event.key === "Enter") {
              event.preventDefault();
              choose(selected);
            }
          }}
        />
        <label className="muted">
          <input
            type="checkbox"
            data-testid="find-in-files-case"
            checked={caseSensitive}
            onChange={(event) => setCaseSensitive(event.target.checked)}
          />{" "}
          Match case
        </label>
        <ul className="action-list" data-testid="find-in-files-results">
          {hits.map((hit, index) => (
            <li
              key={`${hit.path}:${hit.line}`}
              className={`action-row ${hit === selected ? "active" : ""}`}
              data-testid={`find-in-files-item-${index}`}
              onMouseEnter={() => setActive(index)}
            >
              <button
                type="button"
                aria-current={hit === selected ? "true" : undefined}
                onClick={() => choose(hit)}
                title={`${hit.path}:${hit.line + 1}`}
              >
                <FileIcon path={hit.path} />
                <span className="search-kind">line</span>
                <span className="action-title">{hit.text.trim() || hit.path}</span>
                <span className="muted action-group">
                  {hit.path}:{hit.line + 1}
                </span>
              </button>
            </li>
          ))}
          {!loading && !error && !shortQuery && text && hits.length === 0 && (
            <li className="muted" data-testid="find-in-files-empty">
              No results
            </li>
          )}
        </ul>
        <div className="actions">
          <span className="muted" data-testid="find-in-files-status">
            {status}
          </span>
          <button type="button" data-testid="find-in-files-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
