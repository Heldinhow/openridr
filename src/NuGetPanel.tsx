import { useEffect, useMemo, useState } from "react";
import { Icon } from "./Icon";
import { call, messageOf, type OutlineProject } from "./host/client";

export type NuGetPackage = {
  id: string;
  requested: string | null;
  resolved: string | null;
  latestVersion: string | null;
  outdated: boolean;
};

export type NuGetFramework = {
  framework: string;
  packages: NuGetPackage[];
};

export type NuGetCommand = {
  state: "ok" | "failed";
  message: string | null;
  command: string;
  exit: number | null;
  log: string;
  frameworks: NuGetFramework[];
  truncated: boolean;
};

export type NuGetAssetsPackage = { id: string; version: string | null };
export type NuGetAssetsFramework = {
  framework: string;
  implicit: NuGetAssetsPackage[];
  transitive: NuGetAssetsPackage[];
};
export type NuGetAssets = {
  state: "ok" | "missing" | "failed" | "unsupported";
  message: string | null;
  path: string;
  frameworks: NuGetAssetsFramework[];
};

export type NuGetListing = {
  state: "ok" | "partial" | "failed";
  message: string | null;
  project: string;
  projectName: string;
  targetFramework: string | null;
  sdkVersion: string;
  sdkRoot: string;
  cpm: { enabled: boolean; path: string | null; message: string | null };
  installed: NuGetCommand;
  outdated: NuGetCommand;
  sources: {
    state: "ok" | "failed";
    message: string | null;
    items: { name: string; url: string | null }[];
  };
  frameworks: NuGetFramework[];
  totalPackages: number;
  outdatedCount: number;
  truncated: boolean;
  prerelease: boolean;
  selectedSource: string | null;
  assets: NuGetAssets;
  probe: boolean;
};

export type NuGetSolution = {
  state: "ok" | "partial" | "failed";
  message: string | null;
  targetFramework: string | null;
  prerelease: boolean;
  selectedSource: string | null;
  projects: NuGetListing[];
  totalPackages: number;
  outdatedCount: number;
};

export type NuGetPlan = {
  planId: string;
  project: string;
  projectName: string;
  package: string;
  fromVersion: string | null;
  version: string;
  targetFramework: string | null;
  command: string;
  argv: string[];
  install: boolean;
  restore: { command: string; argv: string[]; note: string };
  probe: boolean;
};

export type NuGetUpdate = {
  state: "ok" | "failed";
  message: string | null;
  exit: number | null;
  log: string;
  command: string;
  project: string;
  package: string;
  fromVersion: string | null;
  version: string;
  restore: { command: string; argv: string[]; note: string };
};

export type NuGetBatchItem = {
  project: string;
  package: string;
  fromVersion: string | null;
  version: string;
  targetFramework: string | null;
};

export type NuGetBatchPlan = {
  planId: string;
  count: number;
  items: NuGetPlan[];
};

export type NuGetBatchResult = {
  state: "ok" | "partial" | "failed";
  updated: number;
  failed: number;
  items: {
    state: "ok" | "failed";
    project: string;
    package: string;
    fromVersion: string | null;
    version: string;
    exit: number | null;
    command: string;
    message: string | null;
    log: string;
  }[];
  restores: {
    state: "ok" | "failed";
    message: string | null;
    command: string;
    exit: number | null;
    log: string;
  }[];
};

export type NuGetSearch = {
  state: "ok" | "failed";
  message: string | null;
  source: string;
  query: string;
  prerelease: boolean;
  items: {
    id: string;
    version: string | null;
    description: string | null;
    authors: string[];
    downloads: number | null;
    verified: boolean;
  }[];
};

type SelectedUpdate = NuGetBatchItem & { key: string; projectName: string };

const TRUST_MESSAGE = "Trust the workspace to use NuGet";
const CPM_REASON =
  "Central Package Management is enabled; this MVP lists packages but does not edit central versions.";

function versionText(pkg: NuGetPackage) {
  return pkg.resolved ?? pkg.requested ?? "unknown";
}

function exitText(exit: number | null) {
  return exit === null ? "exit unknown" : `exit ${exit}`;
}

function sourceOf(listing: NuGetListing | null | undefined) {
  if (!listing) return null;
  return listing.sources.state === "ok" && listing.sources.items.length > 0
    ? listing.sources.items
    : null;
}

/**
 * NuGet is read-only until an update/install is planned and confirmed. The development probe is
 * reachable only with `?nugetProbe=1` and an OPENRIDR_ENABLE_PROBE=1 host; it is labelled in the
 * panel and never runs dotnet.
 */
export function NuGetPanel({
  trusted,
  projects,
  restoreRunning,
  onRestore,
  onMessage,
  onError,
}: {
  trusted: boolean;
  projects: OutlineProject[];
  restoreRunning: boolean;
  onRestore: (project: string) => void;
  onMessage: (text: string) => void;
  onError: (text: string) => void;
}) {
  const probe = useMemo(
    () => new URLSearchParams(globalThis.location?.search ?? "").get("nugetProbe") === "1",
    [],
  );
  const probeCpm = useMemo(
    () => probe && new URLSearchParams(globalThis.location?.search ?? "").get("nugetProbeCpm") === "1",
    [probe],
  );
  const probeEmpty = useMemo(
    () => probe && new URLSearchParams(globalThis.location?.search ?? "").get("nugetProbeEmpty") === "1",
    [probe],
  );
  const probeSourcesFailed = useMemo(
    () => probe && new URLSearchParams(globalThis.location?.search ?? "").get("nugetProbeSourcesFailed") === "1",
    [probe],
  );
  const [tab, setTab] = useState<"packages" | "sources" | "log">("packages");
  const [scope, setScope] = useState<"project" | "solution">("project");
  const [mode, setMode] = useState<"installed" | "discover">("installed");
  const [selected, setSelected] = useState(() => projects[0]?.path ?? "");
  const [listing, setListing] = useState<NuGetListing | null>(null);
  const [solution, setSolution] = useState<NuGetSolution | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [prerelease, setPrerelease] = useState(false);
  const [selectedSource, setSelectedSource] = useState("");
  const [plan, setPlan] = useState<NuGetPlan | null>(null);
  const [planError, setPlanError] = useState<string | null>(null);
  const [planBusy, setPlanBusy] = useState(false);
  const [update, setUpdate] = useState<NuGetUpdate | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [staleAfterUpdate, setStaleAfterUpdate] = useState(false);
  const [restoreStarted, setRestoreStarted] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [selectedUpdates, setSelectedUpdates] = useState<Map<string, SelectedUpdate>>(new Map());
  const [batchPlan, setBatchPlan] = useState<NuGetBatchPlan | null>(null);
  const [batchResult, setBatchResult] = useState<NuGetBatchResult | null>(null);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [search, setSearch] = useState<NuGetSearch | null>(null);
  const [searchBusy, setSearchBusy] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [installPlan, setInstallPlan] = useState<{ plan: NuGetPlan; itemId: string } | null>(null);
  const [sortByName, setSortByName] = useState(false);
  const [updatesOpen, setUpdatesOpen] = useState(true);
  const [installedOpen, setInstalledOpen] = useState(true);
  const [implicitOpen, setImplicitOpen] = useState(false);

  useEffect(() => {
    setSelected((current) =>
      current && projects.some((project) => project.path === current)
        ? current
        : (projects[0]?.path ?? ""),
    );
  }, [projects]);

  const projectPaths = useMemo(() => projects.map((project) => project.path), [projects]);
  const activeListing =
    scope === "solution"
      ? solution?.projects.find((item) => item.project === selected) ?? solution?.projects[0]
      : listing;
  const enabledSources = sourceOf(activeListing);

  useEffect(() => {
    if (!trusted) return;
    if (scope === "project" && selected === "") return;
    void load();
    // A new project, scope, source or prerelease reloads once; Refresh is the explicit reload.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [trusted, selected, scope, prerelease, selectedSource]);

  async function load() {
    setBusy(true);
    setError(null);
    setNotice(null);
    setSelectedUpdates(new Map());
    try {
      if (scope === "solution") {
        const answer = await call<NuGetSolution>("nuget.listSolution", {
          projects: projectPaths,
          ...(prerelease ? { prerelease: true } : {}),
          ...(selectedSource ? { source: selectedSource } : {}),
        });
        setSolution(answer);
        setListing(null);
        if (answer.state === "failed" && answer.message) onError(answer.message);
      } else {
        const answer = await call<NuGetListing>(probe ? "nuget.probe" : "nuget.list", {
          project: selected,
          ...(prerelease ? { prerelease: true } : {}),
          ...(selectedSource ? { source: selectedSource } : {}),
          ...(probe
            ? {
                cpm: probeCpm,
                empty: probeEmpty,
                sourcesFailed: probeSourcesFailed,
                source: selectedSource || "https://api.nuget.org/v3/index.json",
              }
            : {}),
        });
        setListing(answer);
        setSolution(null);
        if (!probe && answer.state === "failed" && answer.message) onError(answer.message);
      }
      setStaleAfterUpdate(false);
    } catch (caught) {
      setListing(null);
      setSolution(null);
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function requestPlan(pkg: NuGetPackage, tfm: string, projectPath = selected) {
    if (!pkg.latestVersion) return;
    setPlanError(null);
    setNotice(null);
    setInstallPlan(null);
    try {
      const answer = await call<NuGetPlan>("nuget.planUpdate", {
        project: projectPath,
        id: pkg.id,
        version: pkg.latestVersion,
        fromVersion: pkg.requested ?? pkg.resolved ?? undefined,
        targetFramework: tfm,
      });
      setPlan(answer);
    } catch (caught) {
      setPlanError(messageOf(caught));
    }
  }

  async function requestInstall(itemId: string, version: string | null) {
    if (!version || scope !== "project" || selected === "") return;
    setPlanError(null);
    setNotice(null);
    setPlan(null);
    try {
      const answer = await call<NuGetPlan>("nuget.planUpdate", {
        project: selected,
        id: itemId,
        version,
        install: true,
      });
      setInstallPlan({ plan: answer, itemId });
    } catch (caught) {
      setPlanError(messageOf(caught));
    }
  }

  async function confirmPlan(target: NuGetPlan) {
    if (planBusy) return;
    setPlanBusy(true);
    setPlanError(null);
    setUpdateError(null);
    setNotice(null);
    try {
      const answer = await call<NuGetUpdate>("nuget.applyUpdate", {
        planId: target.planId,
        confirm: true,
      });
      setUpdate(answer);
      setPlan(null);
      setInstallPlan(null);
      setRestoreStarted(false);
      if (answer.state === "ok") {
        setStaleAfterUpdate(true);
        onMessage(`Updated ${answer.package} to ${answer.version}. Restore explicitly before building.`);
        await load();
      } else {
        setStaleAfterUpdate(true);
        onError(answer.message ?? "dotnet add failed.");
      }
    } catch (caught) {
      setPlan(null);
      setInstallPlan(null);
      setStaleAfterUpdate(true);
      setUpdateError(messageOf(caught));
      onError(messageOf(caught));
    } finally {
      setPlanBusy(false);
    }
  }

  function toggleUpdate(item: SelectedUpdate) {
    setSelectedUpdates((current) => {
      const next = new Map(current);
      if (next.has(item.key)) next.delete(item.key);
      else next.set(item.key, item);
      return next;
    });
  }

  async function planBatch() {
    if (selectedUpdates.size === 0) return;
    setBatchError(null);
    setBatchResult(null);
    setNotice(null);
    try {
      const items = [...selectedUpdates.values()].map((item) => ({
        project: item.project,
        id: item.package,
        version: item.version,
        fromVersion: item.fromVersion,
        targetFramework: item.targetFramework,
      }));
      setBatchPlan(await call<NuGetBatchPlan>("nuget.planBatch", { items }));
    } catch (caught) {
      setBatchError(messageOf(caught));
    }
  }

  async function confirmBatch() {
    if (!batchPlan || batchBusy) return;
    setBatchBusy(true);
    setBatchError(null);
    try {
      const answer = await call<NuGetBatchResult>("nuget.applyBatch", {
        planId: batchPlan.planId,
        confirm: true,
      });
      setBatchResult(answer);
      setBatchPlan(null);
      setStaleAfterUpdate(true);
      if (answer.state === "ok") {
        onMessage(`Updated ${answer.updated} packages and ran ${answer.restores.length} restore command(s).`);
      } else {
        onError(`Batch update: ${answer.updated} updated, ${answer.failed} failed. Check each package result.`);
      }
      await load();
    } catch (caught) {
      setBatchError(messageOf(caught));
      onError(messageOf(caught));
    } finally {
      setBatchBusy(false);
    }
  }

  async function runSearch() {
    const configured = enabledSources ?? [];
    const candidate = configured.find((item) => {
      const value = item.url ?? item.name;
      return value.startsWith("https://") || value.startsWith("http://");
    });
    const source = selectedSource || candidate?.url || candidate?.name || "";
    if (!source) {
      setSearchError("Select a configured source with an HTTP v3 URL before searching.");
      return;
    }
    setSearchBusy(true);
    setSearchError(null);
    try {
      setSearch(
        await call<NuGetSearch>("nuget.search", {
          project: selected,
          source,
          query: searchQuery,
          prerelease,
          take: 30,
        }),
      );
    } catch (caught) {
      setSearch(null);
      setSearchError(messageOf(caught));
    } finally {
      setSearchBusy(false);
    }
  }

  if (!trusted) {
    return (
      <div data-testid="nuget-panel">
        <p className="hint" data-testid="nuget-disabled-reason">
          {TRUST_MESSAGE}
        </p>
      </div>
    );
  }

  if (projects.length === 0 || selected === "") {
    return (
      <div data-testid="nuget-panel">
        <p className="hint" data-testid="nuget-disabled-reason">
          No project of the evaluated solution is available for NuGet.
        </p>
      </div>
    );
  }

  const sources =
    enabledSources && enabledSources.length > 0
      ? enabledSources.map((source) => source.name).join(", ")
      : null;
  const sourceTitle = enabledSources
    ? enabledSources
        .map((source) => (source.url ? `${source.name}: ${source.url}` : source.name))
        .join("\n")
    : (activeListing?.sources.message ?? "The configured NuGet sources are unknown.");
  const updateBlocked =
    !activeListing || activeListing.state !== "ok"
      ? "Not all package data was read; refresh before updating."
      : activeListing.cpm.enabled
        ? CPM_REASON
        : staleAfterUpdate
          ? "The last update changed the project; refresh before another update."
          : null;
  const totalPackages = scope === "solution" ? (solution?.totalPackages ?? 0) : (listing?.totalPackages ?? 0);
  const outdatedCount = scope === "solution" ? (solution?.outdatedCount ?? 0) : (listing?.outdatedCount ?? 0);
  const aggregateState = scope === "solution" ? solution?.state : listing?.state;
  const solutionListings = solution?.projects ?? [];
  const renderListings = scope === "solution" ? solutionListings : listing ? [listing] : [];
  const scopeLabel = scope === "solution" ? "Solution" : activeListing?.projectName ?? "Project";
  const updateRows = renderListings.flatMap((item) =>
    item.frameworks.flatMap((framework) =>
      framework.packages
        .filter((pkg) => pkg.outdated && pkg.latestVersion)
        .map((pkg) => ({ item, framework, pkg })),
    ),
  );
  const allUpdateKeys = updateRows.map(
    ({ item, framework, pkg }) => `${item.project}|${framework.framework}|${pkg.id}`,
  );
  const allUpdatesSelected =
    allUpdateKeys.length > 0 && allUpdateKeys.every((key) => selectedUpdates.has(key));
  const installedCount = renderListings.reduce((total, item) => total + item.totalPackages, 0);
  const implicitRows = renderListings.flatMap((item) =>
    item.assets.frameworks.flatMap((assets) =>
      assets.implicit.map((pkg) => ({ item, assets, pkg })),
    ),
  );

  return (
    <div className="nuget-panel" data-testid="nuget-panel">
      <div className="tests-tabs" role="tablist" aria-label="NuGet views">
        {(["packages", "sources", "log"] as const).map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={`tab-label ${tab === id ? "active" : ""}`}
            data-testid={`nuget-tab-${id}`}
            onClick={() => setTab(id)}
          >
            {id === "packages" ? "Packages" : id === "sources" ? "Sources" : "Log"}
          </button>
        ))}
      </div>

      <div className="nuget-body">
        <nav className="nuget-actions" aria-label="NuGet actions">
          <button
            type="button"
            data-testid="nuget-action-restore"
            disabled={restoreRunning || !selected}
            onClick={() => onRestore(selected)}
            title={
              restoreRunning
                ? "A restore is already running."
                : "Runs the explicit restore for the selected project."
            }
            aria-label="Restore project"
          >
            <Icon name="refresh" />
            <span>Restore</span>
          </button>
          <button
            type="button"
            data-testid="nuget-action-refresh"
            disabled={busy}
            onClick={() => void load()}
            title="Reads the package listing again."
            aria-label="Refresh packages"
          >
            <Icon name="refresh" />
            <span>Refresh</span>
          </button>
          <button
            type="button"
            data-testid="nuget-action-sort"
            aria-pressed={sortByName}
            onClick={() => setSortByName((sort) => !sort)}
            title="Sort package rows by id."
            aria-label="Sort packages by name"
          >
            <Icon name="layers" />
            <span>Sort</span>
          </button>
          <button
            type="button"
            data-testid="nuget-action-settings"
            disabled
            title="NuGet source configuration lives in the project's NuGet.config; editing it is not implemented."
            aria-label="NuGet settings (configuration editing unavailable)"
          >
            <Icon name="settings" />
            <span>Settings</span>
          </button>
        </nav>
        <div className="nuget-content">
      {tab === "packages" && (
        <>
          <div className="build-controls">
            <label className="eval-field">
              <span>Scope</span>
              <select
                data-testid="nuget-scope"
                value={scope}
                disabled={busy || probe}
                onChange={(event) => {
                  setScope(event.target.value as "project" | "solution");
                  setMode("installed");
                  setListing(null);
                  setSolution(null);
                  setUpdate(null);
                  setUpdateError(null);
                }}
                title="Solution aggregates each evaluated project; it never invents a project that was not evaluated."
              >
                <option value="project">Project</option>
                <option value="solution">Solution</option>
              </select>
            </label>
            <label className="eval-field">
              <span>Project</span>
              <select
                data-testid="nuget-project"
                value={selected}
                disabled={busy}
                onChange={(event) => {
                  setSelected(event.target.value);
                  setListing(null);
                  setSolution(null);
                  setUpdate(null);
                  setUpdateError(null);
                }}
                title="NuGet is read per evaluated project; packages from other projects are not mixed in."
              >
                {projects.map((project) => (
                  <option key={project.path} value={project.path}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="eval-field">
              <span>Source</span>
              <select
                data-testid="nuget-source-select"
                value={selectedSource}
                disabled={busy}
                onChange={(event) => setSelectedSource(event.target.value)}
                title="The selected source also affects the outdated query and Discover. All configured sources are used when none is selected."
              >
                <option value="">All configured sources</option>
                {(enabledSources ?? []).map((source) => (
                  <option key={`${source.name}:${source.url ?? ""}`} value={source.url ?? source.name}>
                    {source.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="check-row" title="Include pre-release versions in outdated and Discover queries.">
              <input
                type="checkbox"
                data-testid="nuget-prerelease"
                checked={prerelease}
                disabled={busy}
                onChange={(event) => setPrerelease(event.target.checked)}
              />
              Prerelease
            </label>
            <button
              type="button"
              data-testid="nuget-refresh"
              disabled={busy}
              onClick={() => void load()}
            >
              {busy ? "Reading…" : "Refresh"}
            </button>
            <span className="push" />
            <div role="tablist" aria-label="Package mode" className="nuget-mode">
              <button
                type="button"
                role="tab"
                aria-selected={mode === "installed"}
                data-testid="nuget-mode-installed"
                className={mode === "installed" ? "active" : ""}
                onClick={() => setMode("installed")}
              >
                Installed
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={mode === "discover"}
                data-testid="nuget-mode-discover"
                className={mode === "discover" ? "active" : ""}
                onClick={() => setMode("discover")}
              >
                Discover
              </button>
            </div>
            {activeListing && (
              <span className="menu-note nuget-sdk" data-testid="nuget-sdk" title={activeListing.sdkRoot}>
                Project SDK {activeListing.sdkVersion} · {activeListing.sdkRoot}
              </span>
            )}
          </div>

          {probe && (
            <p className="hint" data-testid="nuget-probe">
              Development probe data: dotnet was not run. Set OPENRIDR_ENABLE_PROBE=0 to hide it.
            </p>
          )}
          {error && (
            <p className="error" role="alert" data-testid="nuget-error">
              {error}
            </p>
          )}
          {(listing?.message || solution?.message) && aggregateState !== "ok" && (
            <p className="error" role="alert" data-testid="nuget-state">
              {listing?.message ?? solution?.message}
            </p>
          )}
          {!listing && !solution && !error && (
            <p className="hint" role="status" data-testid="nuget-state">
              Reading packages…
            </p>
          )}

          {mode === "discover" ? (
            <section className="nuget-discover" data-testid="nuget-discover">
              <div className="build-controls">
                <label className="eval-field nuget-search-field">
                  <span>Search</span>
                  <input
                    data-testid="nuget-search"
                    value={searchQuery}
                    placeholder="Package id or keywords"
                    onChange={(event) => setSearchQuery(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") void runSearch();
                    }}
                  />
                </label>
                <button
                  type="button"
                  data-testid="nuget-search-run"
                  disabled={searchBusy || searchQuery.trim() === ""}
                  onClick={() => void runSearch()}
                >
                  {searchBusy ? "Searching…" : "Search"}
                </button>
                <span className="menu-note">
                  Source:{" "}
                  {selectedSource ||
                    enabledSources?.find((item) =>
                      (item.url ?? item.name).startsWith("http"),
                    )?.name ||
                    "none"}
                </span>
              </div>
              {searchError && (
                <p className="error" role="alert" data-testid="nuget-search-error">
                  {searchError}
                </p>
              )}
              {search?.state === "failed" && (
                <p className="error" role="alert" data-testid="nuget-search-state">
                  {search.message}
                </p>
              )}
              {search?.state === "ok" && (
                <>
                  <p className="hint" data-testid="nuget-search-summary">
                    {search.items.length} package{search.items.length === 1 ? "" : "s"} from{" "}
                    <code>{search.source}</code> for “{search.query}”
                    {search.prerelease ? " · prerelease included" : ""}
                  </p>
                  {search.items.length === 0 && (
                    <p className="hint" data-testid="nuget-search-empty">
                      The source returned no package for this query.
                    </p>
                  )}
                  <div className="nuget-table-wrap">
                    <table>
                      <thead>
                        <tr>
                          <th scope="col">Package</th>
                          <th scope="col">Version</th>
                          <th scope="col">Authors</th>
                          <th scope="col">Action</th>
                        </tr>
                      </thead>
                      <tbody>
                        {search.items.map((item) => (
                          <tr key={item.id} data-testid={`nuget-discover-${item.id}`}>
                            <th scope="row">
                              {item.id}
                              {item.verified && <span className="tag"> verified</span>}
                              {item.description && (
                                <span className="menu-note nuget-description">{item.description}</span>
                              )}
                            </th>
                            <td>{item.version ?? "unknown"}</td>
                            <td className="menu-note">{item.authors.join(", ") || "unknown"}</td>
                            <td>
                              <button
                                type="button"
                                data-testid={`nuget-install-${item.id}`}
                                disabled={
                                  updateBlocked !== null ||
                                  scope !== "project" ||
                                  !item.version ||
                                  probe
                                }
                                title={
                                  probe
                                    ? "Discover install is disabled in the probe; dotnet is not run."
                                    : updateBlocked ??
                                      (scope !== "project"
                                        ? "Switch to Project scope to install into one project."
                                        : `Plan an install of ${item.id} ${item.version ?? ""}`)
                                }
                                onClick={() => void requestInstall(item.id, item.version)}
                              >
                                Install…
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
            </section>
          ) : (
            <>
              {solution?.state === "partial" && (
                <p className="warning" role="status" data-testid="nuget-solution-partial">
                  The solution aggregate is incomplete; expand each project below for its real state.
                </p>
              )}
              <p className={aggregateState === "ok" && totalPackages === 0 ? "sr-only" : "hint"} data-testid="nuget-summary">
                {aggregateState === "ok"
                  ? totalPackages === 0
                    ? "No packages are referenced by this project."
                    : `${totalPackages} package${totalPackages === 1 ? "" : "s"} · ${
                        outdatedCount === 0
                          ? "no outdated packages were reported"
                          : `${outdatedCount} outdated`
                      }${activeListing?.truncated ? " · list truncated at the declared limit" : ""}`
                  : "Not all package data was read; this is not an all-up-to-date result."}
              </p>
              {batchResult && (
                <p className="hint" role="status" data-testid="nuget-batch-summary">
                  Batch update: {batchResult.updated} updated · {batchResult.failed} failed ·{" "}
                  {batchResult.restores.length} restore command(s)
                  {batchResult.restores.map(
                    (restore) => ` · restore ${restore.state}`,
                  )}
                </p>
              )}
              <p className="menu-note" data-testid="nuget-sources">
                Enabled configured sources: {sources ?? (activeListing?.sources.state === "ok" ? "none" : "unknown")}. Package origin is not reported.{" "}
                {activeListing?.sources.state !== "ok"
                  ? `· package data is available; source query failed${
                      activeListing?.sources.message ? `: ${activeListing.sources.message}` : "."
                    }`
                  : ""}
              </p>

              {activeListing?.cpm.enabled && (
                <p className="hint" data-testid="nuget-cpm">
                  {activeListing.cpm.message ?? CPM_REASON}
                </p>
              )}
              {updateBlocked && aggregateState === "ok" && (
                <p className="menu-note" data-testid="nuget-update-reason">
                  {updateBlocked}
                </p>
              )}
              {batchError && !batchPlan && (
                <p className="error" role="alert" data-testid="nuget-batch-error">
                  {batchError}
                </p>
              )}
              {planError && !plan && !installPlan && (
                <p className="error" role="alert" data-testid="nuget-plan-error">
                  {planError}
                </p>
              )}
              {updateError && (
                <p className="error" role="alert" data-testid="nuget-update-error">
                  {updateError}
                </p>
              )}
              {notice && (
                <p className="hint" role="status" data-testid="nuget-cancelled">
                  {notice}
                </p>
              )}

              {aggregateState === "ok" && totalPackages === 0 && (
                <p className="hint" role="status" data-testid="nuget-no-packages">
                  No packages are referenced by this project.
                </p>
              )}

              {updateRows.length > 0 && (
                <section className="nuget-section" data-testid="nuget-updates-section">
                  <div className="nuget-section-head">
                    <input
                      type="checkbox"
                      data-testid="nuget-select-all"
                      aria-label={`Select all ${updateRows.length} updates`}
                      checked={allUpdatesSelected}
                      disabled={updateBlocked !== null}
                      onChange={() => {
                        if (allUpdatesSelected) {
                          setSelectedUpdates(new Map());
                        } else {
                          setSelectedUpdates(
                            new Map(
                              updateRows.map(({ item, framework, pkg }) => {
                                const key = `${item.project}|${framework.framework}|${pkg.id}`;
                                return [
                                  key,
                                  {
                                    key,
                                    projectName: item.projectName,
                                    project: item.project,
                                    package: pkg.id,
                                    fromVersion: pkg.requested ?? pkg.resolved,
                                    version: pkg.latestVersion ?? "",
                                    targetFramework: framework.framework,
                                  },
                                ];
                              }),
                            ),
                          );
                        }
                      }}
                    />
                    <button
                      type="button"
                      className="nuget-section-title"
                      aria-expanded={updatesOpen}
                      data-testid="nuget-updates-toggle"
                      onClick={() => setUpdatesOpen((open) => !open)}
                    >
                      Packages to Update in {scopeLabel}: {updateRows.length}
                    </button>
                    <span className="push" />
                    <button
                      type="button"
                      data-testid="nuget-batch-update"
                      disabled={selectedUpdates.size === 0 || updateBlocked !== null}
                      onClick={() => void planBatch()}
                      title={
                        updateBlocked ??
                        "Plans each selected package with its own command; one restore per project follows after confirmation."
                      }
                    >
                      Update
                    </button>
                  </div>
                  {updatesOpen && (
                    <div className="nuget-list">
                      {updateRows.map(({ item, framework, pkg }) => {
                        const key = `${item.project}|${framework.framework}|${pkg.id}`;
                        const reason = busy ? "Reading packages…" : updateBlocked;
                        const suffix = scope === "solution" ? `-${item.projectName}` : "";
                        return (
                          <div
                            className="nuget-row nuget-update-row"
                            data-testid={`nuget-update-package-${framework.framework}-${pkg.id}${suffix}`}
                            key={key}
                          >
                            <input
                              type="checkbox"
                              data-testid={`nuget-select-${framework.framework}-${pkg.id}${suffix}`}
                              aria-label={`Select ${pkg.id} for the batch update`}
                              checked={selectedUpdates.has(key)}
                              disabled={reason !== null}
                              onChange={() =>
                                toggleUpdate({
                                  key,
                                  projectName: item.projectName,
                                  project: item.project,
                                  package: pkg.id,
                                  fromVersion: pkg.requested ?? pkg.resolved,
                                  version: pkg.latestVersion ?? "",
                                  targetFramework: framework.framework,
                                })
                              }
                            />
                            <Icon name="package" />
                            <span className="nuget-package-name">
                              {pkg.id} <span className="nuget-current">• {versionText(pkg)}</span>
                            </span>
                            <span
                              className="nuget-new-version"
                              data-testid={`nuget-latest-${framework.framework}-${pkg.id}${suffix}`}
                            >
                              {pkg.latestVersion}
                            </span>
                            <span
                              className="nuget-row-source sr-only"
                              data-testid={`nuget-source-${framework.framework}-${pkg.id}${suffix}`}
                              title={sourceTitle}
                            >
                              {sources ?? "unknown"}
                            </span>
                            <button
                              type="button"
                              data-testid={`nuget-update-${framework.framework}-${pkg.id}${suffix}`}
                              aria-label={`Update ${pkg.id} from ${versionText(pkg)} to ${pkg.latestVersion}`}
                              title={reason ?? `Plan an update of ${pkg.id} to ${pkg.latestVersion}`}
                              disabled={reason !== null}
                              onClick={() => void requestPlan(pkg, framework.framework, item.project)}
                            >
                              Update…
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </section>
              )}

              {installedCount > 0 && (
                <section className="nuget-section" data-testid="nuget-installed-section">
                  <div className="nuget-section-head">
                    <button
                      type="button"
                      className="nuget-section-title"
                      aria-expanded={installedOpen}
                      data-testid="nuget-installed-toggle"
                      onClick={() => setInstalledOpen((open) => !open)}
                    >
                      Installed Packages in {scopeLabel}: {installedCount}
                    </button>
                    <span className="nuget-badge">{installedCount}</span>
                  </div>
                  {installedOpen && (
                    <div className="nuget-list">
                      {renderListings.map((item) =>
                        item.frameworks.map((framework) => (
                          <div
                            className="nuget-framework"
                            data-testid={`nuget-tfm-${framework.framework}`}
                            key={`${item.project}:${framework.framework}`}
                          >
                            {scope === "solution" && (
                              <p className="nuget-project-label">{item.projectName}</p>
                            )}
                            <p className="nuget-framework-label">Target framework {framework.framework}</p>
                            {framework.packages.length === 0 && (
                              <p className="menu-note">No packages are referenced for this TFM.</p>
                            )}
                            {[...framework.packages]
                              .sort((left, right) =>
                                sortByName ? left.id.localeCompare(right.id) : 0,
                              )
                              .map((pkg) => (
                                <div
                                  className="nuget-row"
                                  data-testid={`nuget-package-${framework.framework}-${pkg.id}${
                                    scope === "solution" ? `-${item.projectName}` : ""
                                  }`}
                                  key={`${item.project}:${framework.framework}:${pkg.id}`}
                                  aria-current={pkg.outdated ? "true" : undefined}
                                >
                                  <Icon name="package" />
                                  <span className="nuget-package-name">
                                    {pkg.id} <span className="nuget-current">• {versionText(pkg)}</span>
                                  </span>
                                  {pkg.outdated && (
                                    <span className="nuget-new-version">{pkg.latestVersion}</span>
                                  )}
                                </div>
                              ))}
                          </div>
                        )),
                      )}
                    </div>
                  )}
                </section>
              )}

              <section className="nuget-section" data-testid="nuget-implicit-section">
                <div className="nuget-section-head">
                  <button
                    type="button"
                    className="nuget-section-title"
                    aria-expanded={implicitOpen}
                    data-testid="nuget-implicit-toggle"
                    onClick={() => setImplicitOpen((open) => !open)}
                  >
                    Implicitly Installed Packages in {scopeLabel}: {implicitRows.length}
                  </button>
                  <span className="nuget-badge">{implicitRows.length}</span>
                </div>
                {renderListings
                  .filter((item) => item.assets.state === "unsupported")
                  .map((item) => (
                    <p className="menu-note" data-testid="nuget-assets-unavailable" key={item.project}>
                      {item.projectName}: {item.assets.message}
                    </p>
                  ))}
                {implicitOpen && (
                  <div className="nuget-list">
                    {renderListings.map((item) => (
                      <div key={item.project}>
                        {item.assets.state !== "ok" && (
                          <p className="menu-note">{item.assets.message}</p>
                        )}
                        {item.assets.frameworks.map((assets) => (
                          <div key={`${item.project}:${assets.framework}`}>
                            <p className="nuget-framework-label">
                              {item.projectName} · {assets.framework}
                            </p>
                            <p className="menu-note" data-testid="nuget-implicit">
                              Implicitly installed ({assets.implicit.length}):
                            </p>
                            <ul className="plain-list">
                              {assets.implicit.map((pkg) => (
                                <li key={pkg.id}>
                                  <Icon name="package" /> {pkg.id} {pkg.version ?? ""}
                                </li>
                              ))}
                              {assets.implicit.length === 0 && <li>none reported</li>}
                            </ul>
                            <p className="menu-note" data-testid="nuget-transitive">
                              Transitive ({assets.transitive.length}):
                            </p>
                            <ul className="plain-list">
                              {assets.transitive.map((pkg) => (
                                <li key={pkg.id}>
                                  <Icon name="package" /> {pkg.id} {pkg.version ?? ""}
                                </li>
                              ))}
                              {assets.transitive.length === 0 && <li>none reported</li>}
                            </ul>
                          </div>
                        ))}
                        {item.assets.state === "ok" && item.assets.frameworks.length === 0 && (
                          <p className="menu-note">No implicit packages are reported for this project.</p>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </section>

              <div className="build-controls nuget-batch-bar">
                <span className="menu-note" data-testid="nuget-selected-count">
                  {selectedUpdates.size} selected for update
                </span>
              </div>
              <div className="build-controls nuget-batch-bar">
                <span className="menu-note" data-testid="nuget-selected-count">
                  {selectedUpdates.size} selected for update
                </span>
              </div>

              {activeListing && (
                <div className="nuget-logs">
                  <details data-testid="nuget-installed-log">
                    <summary>
                      dotnet list package · {activeListing.installed.state} ·{" "}
                      {exitText(activeListing.installed.exit)}
                    </summary>
                    <p className="menu-note">{activeListing.installed.command}</p>
                    {activeListing.installed.message && (
                      <p className="menu-note">{activeListing.installed.message}</p>
                    )}
                    <pre className="build-log">{activeListing.installed.log}</pre>
                  </details>
                  <details data-testid="nuget-outdated-log">
                    <summary>
                      dotnet list package --outdated · {activeListing.outdated.state} ·{" "}
                      {exitText(activeListing.outdated.exit)}
                    </summary>
                    <p className="menu-note">{activeListing.outdated.command}</p>
                    {activeListing.outdated.message && (
                      <p className="menu-note">{activeListing.outdated.message}</p>
                    )}
                    <pre className="build-log">{activeListing.outdated.log}</pre>
                  </details>
                </div>
              )}
            </>
          )}
        </>
      )}

      {tab === "sources" && (
        <section className="nuget-sources" data-testid="nuget-sources-tab">
          <p className="hint">
            Sources come from the project's NuGet configuration. Selecting one affects the
            outdated query and Discover; installation still uses the configured sources.
          </p>
          {activeListing?.sources.state === "failed" && (
            <p className="error" role="alert">
              {activeListing.sources.message ?? "The configured sources are unknown."}
            </p>
          )}
          <ul className="plain-list">
            {(enabledSources ?? []).map((source) => (
              <li key={`${source.name}:${source.url ?? ""}`} data-testid={`nuget-source-item-${source.name}`}>
                <label>
                  <input
                    type="radio"
                    name="nuget-source"
                    checked={selectedSource === (source.url ?? source.name)}
                    onChange={() => setSelectedSource(source.url ?? source.name)}
                  />{" "}
                  <strong>{source.name}</strong>{" "}
                  <code>{source.url ?? "no URL"}</code>
                </label>
              </li>
            ))}
          </ul>
          <button type="button" data-testid="nuget-source-all" onClick={() => setSelectedSource("")}>
            Use all configured sources
          </button>
        </section>
      )}

      {tab === "log" && (
        <section data-testid="nuget-log-tab">
          {renderListings.map((item) => (
            <div key={item.project} data-testid={`nuget-log-${item.projectName}`}>
              <h3>{item.projectName}</h3>
              <details data-testid="nuget-installed-log" open>
                <summary>
                  dotnet list package · {item.installed.state} · {exitText(item.installed.exit)}
                </summary>
                <p className="menu-note">{item.installed.command}</p>
                {item.installed.message && <p className="menu-note">{item.installed.message}</p>}
                <pre className="build-log">{item.installed.log}</pre>
              </details>
              <details data-testid="nuget-outdated-log">
                <summary>
                  dotnet list package --outdated · {item.outdated.state} ·{" "}
                  {exitText(item.outdated.exit)}
                </summary>
                <p className="menu-note">{item.outdated.command}</p>
                {item.outdated.message && <p className="menu-note">{item.outdated.message}</p>}
                <pre className="build-log">{item.outdated.log}</pre>
              </details>
            </div>
          ))}
          {renderListings.length === 0 && <p className="hint">No package listing has been read.</p>}
        </section>
      )}

        </div>
      </div>

      {update && (
        <section className="nuget-update" data-testid="nuget-update-state" aria-label="Update result">
          <p
            className={update.state === "ok" ? "hint" : "error"}
            role={update.state === "ok" ? "status" : "alert"}
          >
            {update.state === "ok"
              ? `Updated ${update.package} from ${update.fromVersion ?? "unknown"} to ${update.version}.`
              : (update.message ?? "dotnet add failed.")}{" "}
            <span data-testid="nuget-update-exit">{exitText(update.exit)}</span>
          </p>
          <p className="menu-note">{update.command}</p>
          <details>
            <summary>Update log</summary>
            <pre className="build-log" data-testid="nuget-update-log">{update.log}</pre>
          </details>
          {update.state === "ok" && (
            <div className="nuget-restore" data-testid="nuget-restore-pending">
              <p className="hint">
                Packages changed; build, run and tests need an explicit restore. OpenRidr never
                restores implicitly.
              </p>
              <p className="menu-note">{update.restore.command}</p>
              <button
                type="button"
                data-testid="nuget-restore"
                disabled={restoreRunning}
                title={
                  restoreRunning
                    ? "A restore is already running; wait for it to finish."
                    : "Run the explicit restore for this project."
                }
                onClick={() => {
                  onRestore(update.project);
                  setRestoreStarted(true);
                }}
              >
                Restore project
              </button>
              {restoreRunning && (
                <p className="menu-note" data-testid="nuget-restore-reason">
                  A restore is already running; wait for it to finish.
                </p>
              )}
              {restoreStarted && !restoreRunning && (
                <p className="hint" role="status" data-testid="nuget-restore-state">
                  Restore started. Its result appears in the Build panel; OpenRidr never restores
                  implicitly.
                </p>
              )}
            </div>
          )}
        </section>
      )}

      {batchResult && (
        <section className="nuget-update" data-testid="nuget-batch-results" aria-label="Batch result">
          <p className={batchResult.state === "ok" ? "hint" : "warning"} role="status">
            Batch update: {batchResult.updated} updated · {batchResult.failed} failed ·{" "}
            {batchResult.restores.length} restore command(s)
          </p>
          <ul className="plain-list">
            {batchResult.items.map((item) => (
              <li key={`${item.project}:${item.package}`} data-testid={`nuget-batch-item-${item.package}`}>
                <span className={`tag ${item.state === "ok" ? "success" : "error"}`}>{item.state}</span>{" "}
                <code>{item.package}</code> {item.fromVersion ?? "unknown"} → {item.version} ·{" "}
                {item.project}
                {item.message && <span className="error"> · {item.message}</span>}
              </li>
            ))}
          </ul>
          {batchResult.restores.map((restore) => (
            <details key={restore.command} data-testid="nuget-batch-restore">
              <summary>
                Restore · {restore.state} · {exitText(restore.exit)}
              </summary>
              <p className="menu-note">{restore.command}</p>
              {restore.message && <p className="error">{restore.message}</p>}
              <pre className="build-log">{restore.log}</pre>
            </details>
          ))}
        </section>
      )}

      {(plan || installPlan) && (
        <PlanDialog
          plan={plan ?? installPlan!.plan}
          planBusy={planBusy}
          planError={planError}
          onConfirm={() => void confirmPlan(plan ?? installPlan!.plan)}
          onCancel={() => {
            setPlan(null);
            setInstallPlan(null);
            setPlanError(null);
            setNotice("Update cancelled; no command was run.");
          }}
        />
      )}

      {batchPlan && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="nuget-batch-title"
            data-testid="nuget-batch-dialog"
          >
            <h2 id="nuget-batch-title">Update {batchPlan.count} packages</h2>
            <p className="hint">
              Runs one `dotnet add` per package with the existing plan gates, then one explicit
              restore per project. A failure is reported per package, never masked.
            </p>
            <ul className="plain-list nuget-command-list">
              {batchPlan.items.map((item) => (
                <li key={item.planId}>
                  <code>{item.command}</code>
                </li>
              ))}
            </ul>
            {batchError && (
              <p className="error" role="alert" data-testid="nuget-batch-error">
                {batchError}
              </p>
            )}
            <div className="actions">
              <button
                type="button"
                data-testid="nuget-batch-confirm"
                disabled={batchBusy}
                onClick={() => void confirmBatch()}
              >
                {batchBusy ? "Updating…" : "Update packages"}
              </button>
              <button
                type="button"
                data-testid="nuget-batch-cancel"
                disabled={batchBusy}
                onClick={() => {
                  setBatchPlan(null);
                  setBatchError(null);
                  setNotice("Batch update cancelled; no command was run.");
                }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function PlanDialog({
  plan,
  planBusy,
  planError,
  onConfirm,
  onCancel,
}: {
  plan: NuGetPlan;
  planBusy: boolean;
  planError: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-backdrop">
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="nuget-plan-title"
        data-testid="nuget-plan-dialog"
      >
        <h2 id="nuget-plan-title">
          {plan.install ? "Install" : "Update"} {plan.package}
        </h2>
        <dl>
          <dt>Project</dt>
          <dd data-testid="nuget-plan-project">{plan.project}</dd>
          <dt>TFM</dt>
          <dd>{plan.targetFramework ?? "not selected"}</dd>
          <dt>Current</dt>
          <dd>{plan.fromVersion ?? "not referenced"}</dd>
          <dt>New</dt>
          <dd data-testid="nuget-plan-version">{plan.version}</dd>
        </dl>
        <p className="hint">
          Runs exactly this command. Nothing runs before you confirm, and no restore follows.
        </p>
        <pre className="nuget-command" data-testid="nuget-plan-command">{plan.command}</pre>
        <p className="menu-note">{plan.restore.note}</p>
        {planError && (
          <p className="error" role="alert" data-testid="nuget-plan-error">
            {planError}
          </p>
        )}
        <div className="actions">
          <button
            type="button"
            data-testid="nuget-plan-confirm"
            disabled={planBusy}
            onClick={onConfirm}
          >
            {planBusy ? "Updating…" : plan.install ? "Install package" : "Update package"}
          </button>
          <button
            type="button"
            data-testid="nuget-plan-cancel"
            disabled={planBusy}
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
