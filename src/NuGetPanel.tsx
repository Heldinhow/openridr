import { useEffect, useMemo, useState } from "react";
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
  probe: boolean;
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

const TRUST_MESSAGE = "Trust the workspace to use NuGet";
const CPM_REASON =
  "Central Package Management is enabled; this MVP lists packages but does not edit central versions.";

function versionText(pkg: NuGetPackage) {
  return pkg.resolved ?? pkg.requested ?? "unknown";
}

function exitText(exit: number | null) {
  return exit === null ? "exit unknown" : `exit ${exit}`;
}

/**
 * NuGet is read-only until a package update is planned and confirmed. The development probe
 * is reachable only with `?nugetProbe=1` and an OPENRIDR_ENABLE_PROBE=1 host; it is labelled in
 * the panel and never runs dotnet.
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
  const [selected, setSelected] = useState(() => projects[0]?.path ?? "");
  const [listing, setListing] = useState<NuGetListing | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [plan, setPlan] = useState<NuGetPlan | null>(null);
  const [planError, setPlanError] = useState<string | null>(null);
  const [planBusy, setPlanBusy] = useState(false);
  const [update, setUpdate] = useState<NuGetUpdate | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [staleAfterUpdate, setStaleAfterUpdate] = useState(false);
  const [restoreStarted, setRestoreStarted] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    setSelected((current) =>
      current && projects.some((project) => project.path === current)
        ? current
        : (projects[0]?.path ?? ""),
    );
  }, [projects]);

  useEffect(() => {
    if (!trusted || selected === "") return;
    void load(selected);
    // A new project or trust state loads once; Refresh is the explicit reload.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [trusted, selected]);

  async function load(project: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const answer = await call<NuGetListing>(probe ? "nuget.probe" : "nuget.list", {
        project,
        ...(probe ? { cpm: probeCpm } : {}),
      });
      setListing(answer);
      setStaleAfterUpdate(false);
      if (!probe && answer.state === "failed" && answer.message) onError(answer.message);
    } catch (caught) {
      setListing(null);
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function requestPlan(pkg: NuGetPackage, tfm: string) {
    if (!pkg.latestVersion) return;
    setPlanError(null);
    setNotice(null);
    try {
      const answer = await call<NuGetPlan>("nuget.planUpdate", {
        project: selected,
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

  async function confirmPlan() {
    if (!plan || planBusy) return;
    setPlanBusy(true);
    setPlanError(null);
    setUpdateError(null);
    setNotice(null);
    try {
      const answer = await call<NuGetUpdate>("nuget.applyUpdate", {
        planId: plan.planId,
        confirm: true,
      });
      setUpdate(answer);
      setPlan(null);
      setRestoreStarted(false);
      if (answer.state === "ok") {
        setStaleAfterUpdate(true);
        onMessage(`Updated ${answer.package} to ${answer.version}. Restore explicitly before building.`);
        await load(selected);
      } else {
        setStaleAfterUpdate(true);
        onError(answer.message ?? "dotnet add failed.");
      }
    } catch (caught) {
      setPlan(null);
      setStaleAfterUpdate(true);
      setUpdateError(messageOf(caught));
      onError(messageOf(caught));
    } finally {
      setPlanBusy(false);
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
    listing?.sources.state === "ok" && listing.sources.items.length > 0
      ? listing.sources.items.map((source) => source.name).join(", ")
      : null;
  const sourceTitle =
    listing?.sources.state === "ok"
      ? listing.sources.items
          .map((source) => (source.url ? `${source.name}: ${source.url}` : source.name))
          .join("\n")
      : (listing?.sources.message ?? "The configured NuGet sources are unknown.");
  const updateBlocked =
    !listing || listing.state !== "ok"
      ? "Not all package data was read; refresh before updating."
      : listing.cpm.enabled
        ? CPM_REASON
        : staleAfterUpdate
          ? "The last update changed the project; refresh before another update."
          : null;

  return (
    <div className="nuget-panel" data-testid="nuget-panel">
      <div className="build-controls">
        <label className="eval-field">
          <span>Project</span>
          <select
            data-testid="nuget-project"
            value={selected}
            disabled={busy}
            onChange={(event) => {
              setSelected(event.target.value);
              setListing(null);
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
        <button
          type="button"
          data-testid="nuget-refresh"
          disabled={busy}
          onClick={() => void load(selected)}
        >
          {busy ? "Reading…" : "Refresh"}
        </button>
        {listing && (
          <span className="menu-note" data-testid="nuget-sdk">
            Project SDK {listing.sdkVersion} · {listing.sdkRoot}
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
      {listing?.message && listing.state !== "ok" && (
        <p className="error" role="alert" data-testid="nuget-state">
          {listing.message}
        </p>
      )}
      {!listing && !error && (
        <p className="hint" role="status" data-testid="nuget-state">
          Reading packages…
        </p>
      )}

      {listing && (
        <>
          <p className="hint" data-testid="nuget-summary">
            {listing.state === "ok"
              ? listing.totalPackages === 0
                ? "No packages are referenced by this project."
                : `${listing.totalPackages} package${listing.totalPackages === 1 ? "" : "s"} · ${
                    listing.outdatedCount === 0
                      ? "no outdated packages were reported"
                      : `${listing.outdatedCount} outdated`
                  }${listing.truncated ? " · list truncated at the declared limit" : ""}`
              : "Not all package data was read; this is not an all-up-to-date result."}
          </p>
          <p className="menu-note" data-testid="nuget-sources">
            Sources consulted: {sources ?? "unknown"}{" "}
            {listing.sources.state !== "ok" && listing.sources.message ? `— ${listing.sources.message}` : ""}
          </p>

          {listing.cpm.enabled && (
            <p className="hint" data-testid="nuget-cpm">
              {listing.cpm.message ?? CPM_REASON}
            </p>
          )}
          {updateBlocked && listing.state === "ok" && (
            <p className="menu-note" data-testid="nuget-update-reason">
              {updateBlocked}
            </p>
          )}
          {planError && !plan && (
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

          {listing.frameworks.map((framework) => (
            <section
              className="nuget-framework"
              data-testid={`nuget-tfm-${framework.framework}`}
              key={framework.framework}
              aria-labelledby={`nuget-tfm-title-${framework.framework}`}
            >
              <h3 id={`nuget-tfm-title-${framework.framework}`}>Target framework {framework.framework}</h3>
              <div className="nuget-table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th scope="col">Package</th>
                      <th scope="col">Current</th>
                      <th scope="col">Latest</th>
                      <th scope="col">Source</th>
                      <th scope="col">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    {framework.packages.length === 0 && (
                      <tr>
                        <td colSpan={5} className="menu-note">
                          No packages are referenced for this TFM.
                        </td>
                      </tr>
                    )}
                    {framework.packages.map((pkg) => {
                      const reason = busy ? "Reading packages…" : updateBlocked;
                      return (
                        <tr
                          key={pkg.id}
                          data-testid={`nuget-package-${framework.framework}-${pkg.id}`}
                          aria-current={pkg.outdated ? "true" : undefined}
                        >
                          <th scope="row">{pkg.id}</th>
                          <td>{versionText(pkg)}</td>
                          <td data-testid={`nuget-latest-${framework.framework}-${pkg.id}`}>
                            {pkg.outdated ? pkg.latestVersion : "—"}
                          </td>
                          <td
                            className="menu-note"
                            data-testid={`nuget-source-${framework.framework}-${pkg.id}`}
                            title={sourceTitle}
                          >
                            {pkg.outdated ? (sources ?? "unknown") : ""}
                          </td>
                          <td>
                            {pkg.outdated ? (
                              <button
                                type="button"
                                data-testid={`nuget-update-${framework.framework}-${pkg.id}`}
                                aria-label={`Update ${pkg.id} from ${versionText(pkg)} to ${pkg.latestVersion}`}
                                title={reason ?? `Plan an update of ${pkg.id} to ${pkg.latestVersion}`}
                                disabled={reason !== null}
                                onClick={() => void requestPlan(pkg, framework.framework)}
                              >
                                Update…
                              </button>
                            ) : (
                              <span className="menu-note">Current</span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </section>
          ))}

          <div className="nuget-logs">
            <details data-testid="nuget-installed-log">
              <summary>
                dotnet list package · {listing.installed.state} · {exitText(listing.installed.exit)}
              </summary>
              <p className="menu-note">{listing.installed.command}</p>
              {listing.installed.message && <p className="menu-note">{listing.installed.message}</p>}
              <pre className="build-log">{listing.installed.log}</pre>
            </details>
            <details data-testid="nuget-outdated-log">
              <summary>
                dotnet list package --outdated · {listing.outdated.state} ·{" "}
                {exitText(listing.outdated.exit)}
              </summary>
              <p className="menu-note">{listing.outdated.command}</p>
              {listing.outdated.message && <p className="menu-note">{listing.outdated.message}</p>}
              <pre className="build-log">{listing.outdated.log}</pre>
            </details>
          </div>
        </>
      )}

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

      {plan && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="nuget-plan-title"
            data-testid="nuget-plan-dialog"
          >
            <h2 id="nuget-plan-title">Update {plan.package}</h2>
            <dl>
              <dt>Project</dt>
              <dd data-testid="nuget-plan-project">{plan.project}</dd>
              <dt>TFM</dt>
              <dd>{plan.targetFramework ?? "not selected"}</dd>
              <dt>Current</dt>
              <dd>{plan.fromVersion ?? "unknown"}</dd>
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
                onClick={() => void confirmPlan()}
              >
                {planBusy ? "Updating…" : "Update package"}
              </button>
              <button
                type="button"
                data-testid="nuget-plan-cancel"
                disabled={planBusy}
                onClick={() => {
                  setPlan(null);
                  setPlanError(null);
                  setNotice("Update cancelled; no command was run.");
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
