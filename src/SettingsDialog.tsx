import { useEffect, useMemo, useRef, useState } from "react";
import { call, HostCallError, type Snapshot } from "./host/client";
import {
  densityOptions,
  fontFamilyOptions,
  INDENT_MAX,
  INDENT_MIN,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  ZOOM_MAX,
  ZOOM_MIN,
  type AppSettings,
  type Density,
} from "./actions/keymap";
import { KeymapPanel } from "./actions/KeymapDialog";
import type { ResolvedAction } from "./actions/registry";

type PageId = "appearance" | "keymap" | "editor" | "build" | "vcs";

type Page = { id: PageId; title: string; keywords: string[] };
type Field = { id: string; label: string; keywords?: string[] };

const pages: Page[] = [
  {
    id: "appearance",
    title: "Appearance",
    keywords: ["theme", "dark", "light", "zoom", "density", "scale", "spacing", "ui"],
  },
  {
    id: "keymap",
    title: "Keymap",
    keywords: ["shortcut", "shortcuts", "profile", "rider", "intellij", "remap", "conflict"],
  },
  {
    id: "editor",
    title: "Editor",
    keywords: ["font", "family", "size", "indent", "indentation", "tabs", "spaces"],
  },
  {
    id: "build",
    title: "Build",
    keywords: ["sdk", "sdks", "dotnet", "analysis", "host", "redetect", "global.json"],
  },
  {
    id: "vcs",
    title: "Version Control",
    keywords: ["git", "path", "version", "vcs"],
  },
];

const fields: Record<PageId, Field[]> = {
  appearance: [
    { id: "theme", label: "Theme", keywords: ["dark", "light", "color"] },
    { id: "zoom", label: "UI zoom", keywords: ["scale", "size"] },
    { id: "density", label: "Density", keywords: ["spacing", "compact", "comfortable"] },
  ],
  keymap: [
    { id: "profile", label: "Profile", keywords: ["rider", "intellij"] },
    { id: "shortcuts", label: "Shortcuts", keywords: ["remap", "reset", "conflict", "keys"] },
  ],
  editor: [
    { id: "fontFamily", label: "Font family", keywords: ["font", "typeface"] },
    { id: "fontSize", label: "Font size", keywords: ["size", "text"] },
    { id: "indentSize", label: "Indent size", keywords: ["indentation", "spaces"] },
    { id: "indentStyle", label: "Indent style", keywords: ["tabs", "spaces"] },
  ],
  build: [
    { id: "sdks", label: "SDKs", keywords: ["dotnet", "detected", "versions"] },
    { id: "analysisHost", label: "Analysis host", keywords: ["csharp-ls", "host", "root"] },
    { id: "redetect", label: "Redetect", keywords: ["scan", "refresh"] },
  ],
  vcs: [{ id: "gitPath", label: "Git path", keywords: ["binary", "version"] }],
};

function textOf(...values: (string | undefined)[]) {
  return values.filter(Boolean).join(" ").toLowerCase();
}

function pageMatches(page: Page, query: string) {
  return textOf(page.title, page.id, ...page.keywords).includes(query);
}

function fieldMatches(field: Field, query: string) {
  return textOf(field.label, field.id, ...(field.keywords ?? [])).includes(query);
}

function messageOf(error: unknown) {
  return error instanceof HostCallError || error instanceof Error
    ? error.message
    : String(error);
}

type GitInfo = {
  state: "found" | "missing";
  path: string | null;
  version: string | null;
  message: string | null;
};

export function SettingsDialog({
  actions,
  settings,
  path,
  platform,
  snapshot,
  onChange,
  onSave,
  onCancel,
  onRedetect,
}: {
  actions: ResolvedAction[];
  settings: AppSettings;
  path: string | null;
  platform: string;
  snapshot: Snapshot;
  onChange: (settings: AppSettings) => void;
  onSave: () => Promise<string | null>;
  onCancel: () => void;
  onRedetect: () => Promise<void>;
}) {
  const [page, setPage] = useState<PageId>("appearance");
  const [query, setQuery] = useState("");
  const [invalid, setInvalid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [zoomText, setZoomText] = useState(String(settings.appearance.zoom));
  const [fontSizeText, setFontSizeText] = useState(String(settings.editor.fontSize));
  const [indentSizeText, setIndentSizeText] = useState(String(settings.editor.indentSize));
  const [git, setGit] = useState<GitInfo | null>(null);
  const [gitError, setGitError] = useState<string | null>(null);
  const [redetectMessage, setRedetectMessage] = useState<string | null>(null);
  const [redetecting, setRedetecting] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!snapshot.trust.granted) {
      setGitError("Trust the workspace to use Git.");
      return;
    }
    let active = true;
    call<GitInfo>("git.info")
      .then((info) => {
        if (active) setGit(info);
      })
      .catch((failure) => {
        if (active) setGitError(messageOf(failure));
      });
    return () => {
      active = false;
    };
  }, [snapshot.trust.granted]);

  const matchedFields = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) return new Set<string>();
    return new Set(
      Object.values(fields)
        .flat()
        .filter((field) => fieldMatches(field, trimmed))
        .map((field) => field.id),
    );
  }, [query]);

  const visiblePages = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) return pages;
    return pages.filter(
      (page) =>
        pageMatches(page, trimmed) ||
        fields[page.id].some((field) => fieldMatches(field, trimmed)),
    );
  }, [query]);

  function updateQuery(value: string) {
    setQuery(value);
    const trimmed = value.trim().toLowerCase();
    if (!trimmed) return;
    const fieldHits = pages.filter((candidate) =>
      fields[candidate.id].some((field) => fieldMatches(field, trimmed)),
    );
    if (fieldHits.length === 1) {
      setPage(fieldHits[0].id);
      return;
    }
    const visible = pages.filter(
      (candidate) =>
        pageMatches(candidate, trimmed) ||
        fields[candidate.id].some((field) => fieldMatches(field, trimmed)),
    );
    const currentMatches = visible.some((candidate) => candidate.id === page);
    if (!currentMatches && visible[0]) setPage(visible[0].id);
  }

  function updateAppearance(next: Partial<AppSettings["appearance"]>) {
    onChange({ ...settings, appearance: { ...settings.appearance, ...next } });
  }

  function updateEditor(next: Partial<AppSettings["editor"]>) {
    onChange({ ...settings, editor: { ...settings.editor, ...next } });
  }

  function setNumber(
    raw: string,
    min: number,
    max: number,
    label: string,
    setText: (value: string) => void,
    apply: (value: number) => void,
  ) {
    setText(raw);
    const value = Number(raw);
    if (raw.trim() === "" || !Number.isInteger(value) || value < min || value > max) {
      setInvalid(`${label} must be a whole number between ${min} and ${max}.`);
      return;
    }
    setInvalid(null);
    apply(value);
  }

  async function save() {
    if (invalid) {
      setError("Fix the invalid value before saving; nothing was written.");
      return;
    }
    setSaving(true);
    setError(null);
    const failure = await onSave();
    setSaving(false);
    if (failure) setError(failure);
  }

  async function redetect() {
    setRedetecting(true);
    setRedetectMessage(null);
    try {
      await onRedetect();
      setRedetectMessage("SDK inventory refreshed. No project file was read or written.");
    } catch (failure) {
      setRedetectMessage(`SDK detection failed: ${messageOf(failure)}`);
    } finally {
      setRedetecting(false);
    }
  }

  const field = (id: string, label: string, control: React.ReactNode) => (
    <label
      className={`settings-field${matchedFields.has(id) ? " match" : ""}`}
      data-settings-field={id}
      data-testid={`settings-field-${id}`}
    >
      <span className="settings-label">{label}</span>
      {control}
    </label>
  );

  return (
    <div className="modal-backdrop">
      <div
        className="modal settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        data-testid="settings-dialog"
        onKeyDown={(event) => {
          if (event.key === "Escape") onCancel();
        }}
      >
        <h2 id="settings-title">Settings</h2>
        <p className="muted" data-testid="settings-scope">
          Global settings for the whole app, saved in {path ?? "settings.json"}. There is no
          per-project override.
        </p>
        <input
          ref={searchRef}
          data-testid="settings-search"
          className="settings-search"
          type="search"
          placeholder="Search settings"
          aria-label="Search settings"
          value={query}
          onChange={(event) => updateQuery(event.target.value)}
        />
        <div className="settings-layout">
          <nav className="settings-tree" aria-label="Settings pages">
            {visiblePages.map((candidate) => (
              <button
                key={candidate.id}
                type="button"
                data-testid={`settings-page-${candidate.id}`}
                aria-current={page === candidate.id ? "page" : undefined}
                onClick={() => setPage(candidate.id)}
              >
                {candidate.title}
              </button>
            ))}
            {visiblePages.length === 0 && (
              <p className="muted" data-testid="settings-no-results">
                No setting matches “{query}”.
              </p>
            )}
          </nav>
          <section className="settings-page" data-testid="settings-page">
            <h3 data-testid="settings-page-title">
              {pages.find((candidate) => candidate.id === page)?.title}
            </h3>
            {page === "appearance" && (
              <>
                {field(
                  "theme",
                  "Theme",
                  <select data-testid="settings-theme" value="dark" onChange={() => undefined}>
                    <option value="dark">Dark</option>
                    <option value="light" disabled>
                      Light — unavailable
                    </option>
                  </select>,
                )}
                <p className="muted" data-testid="settings-theme-note">
                  Light theme is unavailable in this build: OpenRidr ships one approved dark
                  token set. The value is not selectable, not hidden.
                </p>
                {field(
                  "zoom",
                  "UI zoom (%)",
                  <input
                    data-testid="settings-zoom"
                    type="number"
                    min={ZOOM_MIN}
                    max={ZOOM_MAX}
                    value={zoomText}
                    onChange={(event) =>
                      setNumber(
                        event.target.value,
                        ZOOM_MIN,
                        ZOOM_MAX,
                        "UI zoom",
                        setZoomText,
                        (zoom) => updateAppearance({ zoom }),
                      )
                    }
                  />,
                )}
                {field(
                  "density",
                  "Density",
                  <select
                    data-testid="settings-density"
                    value={settings.appearance.density}
                    onChange={(event) =>
                      updateAppearance({ density: event.target.value as Density })
                    }
                  >
                    {densityOptions.map((option) => (
                      <option key={option.id} value={option.id}>
                        {option.title}
                      </option>
                    ))}
                  </select>,
                )}
              </>
            )}
            {page === "keymap" && (
              <KeymapPanel
                actions={actions}
                keymap={settings.keymap}
                path={path}
                platform={platform}
                onChange={(keymap) => {
                  onChange({ ...settings, keymap });
                  return Promise.resolve(null);
                }}
              />
            )}
            {page === "editor" && (
              <>
                {field(
                  "fontFamily",
                  "Font family",
                  <select
                    data-testid="settings-font-family"
                    value={settings.editor.fontFamily}
                    onChange={(event) => updateEditor({ fontFamily: event.target.value })}
                  >
                    {fontFamilyOptions.map((option) => (
                      <option key={option.id} value={option.id}>
                        {option.title}
                      </option>
                    ))}
                  </select>,
                )}
                {field(
                  "fontSize",
                  "Font size (px)",
                  <input
                    data-testid="settings-font-size"
                    type="number"
                    min={FONT_SIZE_MIN}
                    max={FONT_SIZE_MAX}
                    value={fontSizeText}
                    onChange={(event) =>
                      setNumber(
                        event.target.value,
                        FONT_SIZE_MIN,
                        FONT_SIZE_MAX,
                        "Font size",
                        setFontSizeText,
                        (fontSize) => updateEditor({ fontSize }),
                      )
                    }
                  />,
                )}
                {field(
                  "indentStyle",
                  "Indent style",
                  <select
                    data-testid="settings-indent-style"
                    value={settings.editor.useTabs ? "tabs" : "spaces"}
                    onChange={(event) => updateEditor({ useTabs: event.target.value === "tabs" })}
                  >
                    <option value="spaces">Spaces</option>
                    <option value="tabs">Tabs</option>
                  </select>,
                )}
                {field(
                  "indentSize",
                  "Indent size",
                  <input
                    data-testid="settings-indent-size"
                    type="number"
                    min={INDENT_MIN}
                    max={INDENT_MAX}
                    value={indentSizeText}
                    onChange={(event) =>
                      setNumber(
                        event.target.value,
                        INDENT_MIN,
                        INDENT_MAX,
                        "Indent size",
                        setIndentSizeText,
                        (indentSize) => updateEditor({ indentSize }),
                      )
                    }
                  />,
                )}
                <p className="muted">
                  Applies to the open editor immediately; the status bar reports the same
                  indentation.
                </p>
              </>
            )}
            {page === "build" && (
              <>
                <div
                  className={`settings-block${matchedFields.has("sdks") ? " match" : ""}`}
                  data-settings-field="sdks"
                  data-testid="settings-field-sdks"
                >
                  <span className="settings-label">Detected SDKs</span>
                  <ul data-testid="settings-sdk-list">
                    <li>
                      Host SDK:{" "}
                      {snapshot.sdk.host.state === "found"
                        ? `${snapshot.sdk.host.version} · ${snapshot.sdk.host.sdkPath ?? snapshot.sdk.host.root ?? ""}`
                        : (snapshot.sdk.host.message ?? "not found")}
                    </li>
                    <li>
                      Project SDK:{" "}
                      {snapshot.sdk.project.state === "resolved"
                        ? snapshot.sdk.project.resolvedVersion
                        : (snapshot.sdk.project.message ?? snapshot.sdk.project.state)}
                    </li>
                    {snapshot.sdk.roots.map((root) => (
                      <li key={root.root}>
                        {root.root} · {root.source} ·{" "}
                        {root.versions.length > 0
                          ? root.versions.map((version) => version.version).join(", ")
                          : (root.error ?? "no SDK found")}
                      </li>
                    ))}
                  </ul>
                </div>
                <div
                  className={`settings-block${matchedFields.has("analysisHost") ? " match" : ""}`}
                  data-settings-field="analysisHost"
                  data-testid="settings-field-analysisHost"
                >
                  <span className="settings-label">Analysis host</span>
                  <p data-testid="settings-analysis-host">
                    {snapshot.lsp.command.length > 0
                      ? snapshot.lsp.command.join(" ")
                      : "csharp-ls is not running."}
                  </p>
                  <p className="muted">
                    Host root: <code data-testid="settings-host-root">{snapshot.lsp.hostRoot}</code>
                  </p>
                </div>
                <div className="actions">
                  <button
                    type="button"
                    data-testid="settings-redetect"
                    disabled={redetecting}
                    onClick={() => void redetect()}
                    title="Runs the existing SDK detection again. It never installs an SDK and never edits global.json."
                  >
                    Redetect
                  </button>
                </div>
                {redetectMessage && (
                  <p className="muted" role="status" data-testid="settings-redetect-message">
                    {redetectMessage}
                  </p>
                )}
                <p className="muted">
                  Settings never install a SDK and never change <code>global.json</code>.
                </p>
              </>
            )}
            {page === "vcs" && (
              <>
                <div
                  className={`settings-block${matchedFields.has("gitPath") ? " match" : ""}`}
                  data-settings-field="gitPath"
                  data-testid="settings-field-gitPath"
                >
                  <span className="settings-label">Git in use</span>
                  {git ? (
                    <>
                      <p data-testid="settings-git-path">
                        {git.state === "found" ? (git.path ?? "git from PATH") : "Not detected"}
                      </p>
                      <p className="muted" data-testid="settings-git-version">
                        {git.version ?? (git.message ?? "git was not found on PATH.")}
                      </p>
                    </>
                  ) : (
                    <p className="muted" data-testid="settings-git-path">
                      {gitError ?? "Detecting…"}
                    </p>
                  )}
                  <p className="muted" data-testid="settings-git-message">
                    Read-only: the path comes from PATH and cannot be changed here.
                  </p>
                </div>
              </>
            )}
            {invalid && (
              <p className="error" role="alert" data-testid="settings-error">
                {invalid}
              </p>
            )}
            {error && (
              <p className="error" role="alert" data-testid="settings-save-error">
                {error}
              </p>
            )}
          </section>
        </div>
        <div className="actions">
          <button type="button" data-testid="settings-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className="primary"
            data-testid="settings-save"
            disabled={saving || invalid !== null}
            onClick={() => void save()}
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
