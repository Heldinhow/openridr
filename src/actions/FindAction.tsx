import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { call, type ReadFile, type TextSearch, type WorkspaceFiles, type WorkspaceSymbol, type WorkspaceSymbolResult } from "../host/client";
import { profileTitle, type ProfileId } from "./keymap";
import type { ResolvedAction } from "./registry";

export type SearchTab = "all" | "classes" | "files" | "symbols" | "actions" | "text";

const TABS: { id: SearchTab; label: string }[] = [
  { id: "all", label: "All" },
  { id: "classes", label: "Classes" },
  { id: "files", label: "Files" },
  { id: "symbols", label: "Symbols" },
  { id: "actions", label: "Actions" },
  { id: "text", label: "Text" },
];

const STORAGE_KEY = "openridr.search";
const ALL_LIMIT = 300;
const PREVIEW_LINES = 9;

/** LSP SymbolKind: class, enum, interface, struct. Everything else is a member. */
const TYPE_KINDS = new Set([5, 10, 11, 23]);

function isSearchTab(value: unknown): value is SearchTab {
  return TABS.some((tab) => tab.id === value);
}

function loadSaved(): { tab: SearchTab; query: string } {
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null") as
      | { tab?: unknown; query?: unknown }
      | null;
    return {
      tab: isSearchTab(parsed?.tab) ? parsed.tab : "all",
      query: typeof parsed?.query === "string" ? parsed.query : "",
    };
  } catch {
    return { tab: "all", query: "" };
  }
}

function saveSaved(tab: SearchTab, query: string) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ tab, query }));
  } catch {
    // Storage may be unavailable; search still works without remembering.
  }
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function useDebounced(value: string, delay: number) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delay);
    return () => window.clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

function Highlight({ text, needle }: { text: string; needle: string }) {
  if (!needle) return <>{text}</>;
  const haystack = text.toLowerCase();
  const target = needle.toLowerCase();
  const parts: ReactNode[] = [];
  let from = 0;
  let index = haystack.indexOf(target);
  while (index >= 0) {
    if (index > from) parts.push(text.slice(from, index));
    parts.push(
      <mark className="search-hit" key={`${index}-${parts.length}`}>
        {text.slice(index, index + needle.length)}
      </mark>,
    );
    from = index + needle.length;
    index = haystack.indexOf(target, from);
  }
  parts.push(text.slice(from));
  return <>{parts}</>;
}

type Hit =
  | { key: string; type: "action"; action: ResolvedAction; title: string; detail: string }
  | { key: string; type: "file"; path: string; title: string; detail: string }
  | { key: string; type: "symbol"; symbol: WorkspaceSymbol; title: string; detail: string }
  | {
      key: string;
      type: "text";
      path: string;
      line: number;
      character: number;
      title: string;
      detail: string;
      text: string;
    };

export function FindAction({
  actions,
  platform,
  profile,
  initialTab,
  onExecute,
  onOpenFile,
  onNavigate,
  onClose,
}: {
  actions: ResolvedAction[];
  platform: string;
  profile: ProfileId;
  initialTab: SearchTab | "last";
  onExecute: (action: ResolvedAction) => void;
  onOpenFile: (path: string) => void;
  onNavigate: (path: string, line: number, character: number) => void;
  onClose: () => void;
}) {
  const saved = useRef(loadSaved()).current;
  const [tab, setTab] = useState<SearchTab>(initialTab === "last" ? saved.tab : initialTab);
  const [query, setQuery] = useState(saved.query);
  const [active, setActive] = useState(0);
  const [files, setFiles] = useState<WorkspaceFiles | null>(null);
  const [filesError, setFilesError] = useState<string | null>(null);
  const [symbols, setSymbols] = useState<WorkspaceSymbolResult | null>(null);
  const [symbolsError, setSymbolsError] = useState<string | null>(null);
  const [symbolsLoading, setSymbolsLoading] = useState(false);
  const [text, setText] = useState<TextSearch | null>(null);
  const [textError, setTextError] = useState<string | null>(null);
  const [textLoading, setTextLoading] = useState(false);
  const [preview, setPreview] = useState<{
    path: string;
    focus: number;
    lines: string[];
    needle: string;
  } | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const debounced = useDebounced(query, 180);
  const needle = query.trim().toLowerCase();
  const symbolsVisible = tab === "all" || tab === "classes" || tab === "symbols";
  const textVisible = tab === "all" || tab === "text";
  const symbolQuery = debounced.trim();
  const symbolRequest = useRef(0);
  const textRequest = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    saveSaved(tab, query);
  }, [tab, query]);

  useEffect(() => {
    let alive = true;
    call<WorkspaceFiles>("workspace.files")
      .then((result) => {
        if (alive) setFiles(result);
      })
      .catch((error) => {
        if (alive) setFilesError(errorMessage(error));
      });
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (!symbolsVisible || !symbolQuery) {
      setSymbolsLoading(false);
      setSymbols(null);
      setSymbolsError(null);
      return;
    }
    let alive = true;
    const seq = ++symbolRequest.current;
    setSymbolsLoading(true);
    setSymbols(null);
    setSymbolsError(null);
    call<WorkspaceSymbolResult>("lsp.workspaceSymbol", { query: symbolQuery })
      .then((result) => {
        if (alive && seq === symbolRequest.current) setSymbols(result);
      })
      .catch((error) => {
        if (alive && seq === symbolRequest.current) setSymbolsError(errorMessage(error));
      })
      .finally(() => {
        if (alive && seq === symbolRequest.current) setSymbolsLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [symbolsVisible, symbolQuery]);

  useEffect(() => {
    if (!textVisible || debounced.trim().length < 2) {
      setTextLoading(false);
      setText(null);
      setTextError(null);
      return;
    }
    let alive = true;
    const seq = ++textRequest.current;
    setTextLoading(true);
    setText(null);
    setTextError(null);
    call<TextSearch>("search.text", { query: debounced })
      .then((result) => {
        if (alive && seq === textRequest.current) setText(result);
      })
      .catch((error) => {
        if (alive && seq === textRequest.current) setTextError(errorMessage(error));
      })
      .finally(() => {
        if (alive && seq === textRequest.current) setTextLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [textVisible, debounced]);

  const actionMatches = useMemo(() => {
    if (!needle) return actions;
    return actions.filter((action) =>
      `${action.title} ${action.id} ${action.group}`.toLowerCase().includes(needle),
    );
  }, [actions, needle]);

  const fileMatches = useMemo(() => {
    if (!files) return [];
    if (!needle) return files.files;
    return files.files.filter((path) => path.toLowerCase().includes(needle));
  }, [files, needle]);

  const hitsResult = useMemo<{ hits: Hit[]; total: number }>(() => {
    const actionHits: Hit[] = actionMatches.map((action) => ({
      key: `action:${action.id}`,
      type: "action",
      action,
      title: action.title,
      detail: `${action.group} · ${action.shortcutLabels.join(" / ") || "No shortcut"}`,
    }));
    const fileHits: Hit[] = fileMatches.map((path) => ({
      key: `file:${path}`,
      type: "file",
      path,
      title: path.split("/").pop() ?? path,
      detail: path,
    }));
    const symbolHits: Hit[] =
      symbols?.state === "ok"
        ? symbols.items
            .filter((symbol) =>
              tab === "classes"
                ? TYPE_KINDS.has(symbol.kind)
                : tab === "symbols"
                  ? !TYPE_KINDS.has(symbol.kind)
                  : true,
            )
            .map((symbol) => ({
              key: `symbol:${symbol.path}:${symbol.range.start.line}:${symbol.name}:${symbol.kind}`,
              type: "symbol",
              symbol,
              title: symbol.name,
              detail: [symbol.containerName, symbol.path].filter(Boolean).join(" · "),
            }))
        : [];
    const textHits: Hit[] =
      text?.files.flatMap((file) =>
        file.matches.map((match) => ({
          key: `text:${file.path}:${match.line}:${match.character}`,
          type: "text" as const,
          path: file.path,
          line: match.line,
          character: match.character,
          title: match.text.trim() || file.path,
          detail: `${file.path}:${match.line + 1}`,
          text: match.text,
        })),
      ) ?? [];

    if (tab === "actions") return { hits: actionHits, total: actionHits.length };
    if (tab === "files") return { hits: fileHits, total: fileHits.length };
    if (tab === "classes" || tab === "symbols") {
      return { hits: symbolHits, total: symbolHits.length };
    }
    if (tab === "text") return { hits: textHits, total: textHits.length };
    const all = [...actionHits, ...fileHits, ...symbolHits, ...textHits];
    return { hits: all.slice(0, ALL_LIMIT), total: all.length };
  }, [actionMatches, fileMatches, symbols, text, tab]);

  const { hits, total: totalHits } = hitsResult;
  const selected = hits[Math.min(active, hits.length - 1)] ?? null;

  useEffect(() => {
    setActive(0);
  }, [tab, query]);

  useEffect(() => {
    if (!selected || selected.type === "action") {
      setPreview(null);
      setPreviewError(null);
      return;
    }
    const target =
      selected.type === "file"
        ? { path: selected.path, focus: 0, needle: "" }
        : selected.type === "symbol"
          ? {
              path: selected.symbol.path,
              focus: selected.symbol.range.start.line,
              needle: selected.symbol.name,
            }
          : { path: selected.path, focus: selected.line, needle: query.trim() };
    let alive = true;
    call<ReadFile>("file.read", { path: target.path })
      .then((file) => {
        if (!alive) return;
        setPreviewError(null);
        setPreview({
          path: file.path,
          focus: target.focus,
          lines: file.content.split(/\r?\n/),
          needle: target.needle,
        });
      })
      .catch((error) => {
        if (alive) {
          setPreview(null);
          setPreviewError(errorMessage(error));
        }
      });
    return () => {
      alive = false;
    };
  }, [selected?.key, query]);

  function choose(hit: Hit | null) {
    if (!hit) return;
    if (hit.type === "action") {
      if (hit.action.disabledReason) return;
      onExecute(hit.action);
    } else if (hit.type === "file") {
      onOpenFile(hit.path);
    } else if (hit.type === "symbol") {
      onNavigate(hit.symbol.path, hit.symbol.range.start.line, hit.symbol.range.start.character);
    } else {
      onNavigate(hit.path, hit.line, hit.character);
    }
  }

  function cycleTab(direction: number) {
    const index = TABS.findIndex((item) => item.id === tab);
    setTab(TABS[(index + direction + TABS.length) % TABS.length].id);
  }

  const symbolUnavailable =
    symbolsVisible && symbolQuery && symbols && symbols.state !== "ok"
      ? symbols
      : null;
  const symbolNote = symbolsError
    ? `Symbols are unavailable: ${symbolsError}`
    : symbolUnavailable
      ? symbolUnavailable.state === "analysis_not_ready"
        ? `Analysis not ready. ${symbolUnavailable.missingAction ?? "Start analysis"}.`
        : symbolUnavailable.state === "unsupported"
          ? "Symbol search is not supported by this analysis session."
          : symbolUnavailable.state === "stale"
            ? "Analysis changed while searching; type to search again."
            : `Symbol search failed. ${symbolUnavailable.message ?? ""}`
      : null;
  const textNote = textError
    ? `Text search is unavailable: ${textError}`
    : textLoading
      ? "Searching text…"
      : null;
  const filesNote = filesError
    ? `Files are unavailable: ${filesError}`
    : files === null
      ? "Loading files…"
      : null;
  const needsQuery =
    (tab === "classes" || tab === "symbols") && !symbolQuery
      ? "Type a name to search workspace symbols."
      : tab === "text" && query.trim().length < 2
        ? "Type at least two characters to search the workspace."
        : null;
  const allBlocked =
    tab === "all"
      ? (symbolNote ?? (files === null ? filesNote : null) ?? (textError ? textNote : null))
      : null;
  const blockedNote =
    tab === "classes" || tab === "symbols"
      ? (needsQuery ?? symbolNote)
      : tab === "text"
        ? (needsQuery ?? textNote)
        : tab === "files"
          ? filesNote
          : allBlocked;
  const loading =
    tab === "all"
      ? (files === null && !filesError) || symbolsLoading || textLoading
      : (tab === "classes" || tab === "symbols" ? symbolsLoading : false) ||
        (tab === "text" ? textLoading : false);
  const empty = hits.length === 0 && !loading && !blockedNote;
  const allLimited = tab === "all" && totalHits > hits.length;
  const truncatedNote =
    tab === "files" && files?.truncated
      ? `Showing the first ${files.returned} files; refine the query.`
      : tab === "text" && text?.truncated
        ? `Showing the first ${text.fileCount} files and ${text.matchCount} matches; refine the query.`
        : (tab === "classes" || tab === "symbols" || tab === "all") && symbols?.truncated
          ? `Showing ${symbols.items.length} of ${symbols.total} symbols.`
          : allLimited
            ? `Showing the first ${ALL_LIMIT} results; use a tab to narrow the search.`
            : null;

  return (
    <div className="modal-backdrop">
      <div
        className="modal find-action"
        role="dialog"
        aria-modal="true"
        aria-labelledby="find-action-title"
        data-testid="find-action"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <h2 id="find-action-title">Search Everywhere</h2>
        <p className="muted" data-testid="find-action-profile">
          Keymap: {profileTitle(profile)} · {platform === "macos" ? "macOS" : "Windows"} keys
        </p>
        <div className="search-tabs" role="tablist" aria-label="Search scope">
          {TABS.map((item) => (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={item.id === tab}
              data-testid={`search-tab-${item.id}`}
              onClick={() => {
                setTab(item.id);
                inputRef.current?.focus();
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
        <input
          autoFocus
          ref={inputRef}
          data-testid="find-action-input"
          aria-label="Search files, symbols, actions and text"
          placeholder="Search files, symbols, actions and text"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Tab") {
              event.preventDefault();
              cycleTab(event.shiftKey ? -1 : 1);
            } else if (event.key === "ArrowDown") {
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
        <div className="search-body">
          <ul className="action-list" data-testid="find-action-list">
            {hits.map((hit, index) => {
              const activeRow = hit.key === selected?.key;
              if (hit.type === "action") {
                const action = hit.action;
                return (
                  <li
                    key={hit.key}
                    className={`action-row ${activeRow ? "active" : ""}`}
                    data-testid={`find-action-item-${action.id}`}
                    onMouseEnter={() => setActive(index)}
                  >
                    <button
                      type="button"
                      disabled={Boolean(action.disabledReason)}
                      aria-current={activeRow ? "true" : undefined}
                      onClick={() => choose(hit)}
                      title={action.disabledReason ?? action.title}
                    >
                      <span className="action-title">{action.title}</span>
                      <span className="muted action-group">{action.group}</span>
                      <span
                        className="action-keys"
                        data-testid={`find-action-shortcut-${action.id}`}
                      >
                        {action.shortcutLabels.length > 0
                          ? action.shortcutLabels.join(" / ")
                          : "No shortcut"}
                      </span>
                      {action.conflicts.length > 0 && (
                        <span
                          className="warning"
                          data-testid={`find-action-conflict-${action.id}`}
                          title={`Same shortcut as ${action.conflicts.join(", ")}`}
                        >
                          Conflict: {action.conflicts.join(", ")}
                        </span>
                      )}
                      {action.disabledReason && (
                        <span
                          className="muted action-reason"
                          data-testid={`find-action-reason-${action.id}`}
                        >
                          {action.disabledReason}
                        </span>
                      )}
                    </button>
                  </li>
                );
              }
              return (
                <li
                  key={hit.key}
                  className={`action-row ${activeRow ? "active" : ""}`}
                  data-testid={`search-item-${index}`}
                  onMouseEnter={() => setActive(index)}
                >
                  <button
                    type="button"
                    aria-current={activeRow ? "true" : undefined}
                    onClick={() => choose(hit)}
                    title={hit.detail}
                  >
                    <span className="search-kind">{hit.type}</span>
                    <span className="action-title">{hit.title}</span>
                    <span className="muted action-group">{hit.detail}</span>
                  </button>
                </li>
              );
            })}
            {blockedNote && (
              <li className="muted search-state" data-testid="search-state">
                {blockedNote}
              </li>
            )}
            {loading && (
              <li className="muted" data-testid="search-loading">
                Searching…
              </li>
            )}
            {empty && (
              <li className="muted" data-testid="find-action-empty">
                <span data-testid="search-empty">No results</span>
              </li>
            )}
            {truncatedNote && (
              <li className="muted search-truncated" data-testid="search-truncated">
                {truncatedNote}
              </li>
            )}
          </ul>
          <aside className="search-preview" data-testid="search-preview">
            {preview && (
              <>
                <div className="search-preview-path" data-testid="search-preview-path">
                  {preview.path}
                </div>
                <pre data-testid="search-preview-code">
                  {preview.lines
                    .slice(
                      Math.max(0, preview.focus - Math.floor(PREVIEW_LINES / 2)),
                      Math.max(0, preview.focus - Math.floor(PREVIEW_LINES / 2)) + PREVIEW_LINES,
                    )
                    .map((line, offset) => {
                      const number = Math.max(0, preview.focus - Math.floor(PREVIEW_LINES / 2)) + offset;
                      return (
                        <span
                          key={number}
                          className={`search-preview-line ${number === preview.focus ? "active" : ""}`}
                        >
                          <span className="search-preview-number">{number + 1}</span>
                          <Highlight text={line} needle={preview.needle} />
                          {"\n"}
                        </span>
                      );
                    })}
                </pre>
              </>
            )}
            {!preview && selected?.type === "action" && (
              <div data-testid="search-preview-action">
                <div className="search-preview-path">{selected.action.group}</div>
                <p className="action-title">{selected.action.title}</p>
                <p className="muted">id: {selected.action.id}</p>
                <p className="muted">
                  {selected.action.shortcutTexts.join(" / ") || "No shortcut"}
                </p>
                {selected.action.disabledReason && (
                  <p className="warning">{selected.action.disabledReason}</p>
                )}
              </div>
            )}
            {previewError && (
              <p className="muted" data-testid="search-preview-error">
                Preview unavailable: {previewError}
              </p>
            )}
            {!selected && !blockedNote && !loading && (
              <p className="muted">Select a result to preview it.</p>
            )}
          </aside>
        </div>
        <div className="actions">
          <span className="muted" data-testid="search-status">
            {truncatedNote ?? "Enter opens the selected result · Tab changes scope · Esc closes"}
          </span>
        </div>
      </div>
    </div>
  );
}
