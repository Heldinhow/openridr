import { Icon } from "./Icon";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { call, messageOf, type Evaluation, type TestItem, type TestResult, type TestRun, type Tests, type TestsDiscovery } from "./host/client";
import {
  buildTestTree,
  filterTestTree,
  testCount,
  type TestGroup,
  type TestLeaf,
  type TreeNode,
} from "./testTree";

type Props = {
  tests: Tests;
  evaluation: Evaluation;
  editedAt: Record<string, number>;
  disabledReason: string | null;
  autoDiscoveryError: string | null;
  projectPath: string;
  onProjectChange: (path: string) => void;
  selected: Set<string>;
  onToggleSelected: (id: string) => void;
  restoreRequired: boolean;
  restoreDisabledReason: string | null;
  onRestore: () => void;
  onEvaluate: (next: { targetFramework?: string | null }) => void;
  onDiscover: (project: string) => void;
  onCancel: () => void;
  onRun: (project: string, selection: "all" | "selected", testIds: string[]) => void;
  onRunCancel: () => void;
  onNavigate: (path: string, position: { line: number; character: number }) => void;
};

export function discoveryStateLabel(discovery: TestsDiscovery) {
  const exit = discovery.exit === null ? "exit unknown" : `exit ${discovery.exit}`;
  switch (discovery.state) {
    case "discovering":
      return "Discovering…";
    case "completed":
      return `Discovered ${discovery.items.length} test${discovery.items.length === 1 ? "" : "s"} · ${exit}`;
    case "empty":
      return `No tests listed · ${exit}`;
    case "failed":
      return `Discovery failed · ${exit}`;
    case "cancelled":
      return `Discovery cancelled · ${exit}`;
    case "adapter_missing":
      return "No VSTest adapter";
    case "mtp_unsupported":
      return "Microsoft.Testing.Platform";
    default:
      return "No discovery has run";
  }
}

/** Unsaved edits or saves after the discovery started make the list stale, never failed. */
export function isDiscoveryObsolete(
  discovery: TestsDiscovery,
  editedAt: Record<string, number>,
) {
  if (discovery.revision === 0) return false;
  const reference = discovery.startedAt ?? discovery.finishedAt ?? 0;
  return Object.values(editedAt).some((edited) => edited > reference);
}

function itemLabel(item: TestItem) {
  return `${item.fullyQualifiedName} · ${item.project} · ${item.tfm ?? "TFM not selected"}${
    item.occurrences > 1 ? " · listed more than once by VSTest" : ""
  }`;
}

export function runStateLabel(run: TestRun) {
  const exit = run.exit === null ? "exit unknown" : `exit ${run.exit}`;
  switch (run.state) {
    case "running":
      return "Running tests…";
    case "completed":
      return `Run completed · ${run.counts.total} result${run.counts.total === 1 ? "" : "s"} · ${exit}`;
    case "failed":
      return `Run failed · ${run.infrastructureFailure ? "infrastructure" : "run"} · ${exit}`;
    case "cancelled":
      return `Run cancelled · ${run.counts.total} partial result${run.counts.total === 1 ? "" : "s"} · ${exit}`;
    default:
      return "No test run";
  }
}

/** Buffer edits after the run started make the results obsolete, never corrected. */
export function isRunObsolete(run: TestRun, editedAt: Record<string, number>) {
  if (run.revision === 0) return false;
  const reference = run.startedAt ?? run.finishedAt ?? 0;
  return Object.values(editedAt).some((edited) => edited > reference);
}

/** The host only launches an evaluated console assembly and has no testhost attach, so the
 * action stays visible and honest instead of pretending to debug with a plain run.
 * Decision: docs/decisions.md, 2026-11-02. */
export const testDebugUnavailable =
  "Debugging tests needs a test-host attach the debugger does not support yet. Use Run selected.";

export function testRunBlockedReason(args: {
  disabledReason: string | null;
  discovery: TestsDiscovery;
  run: TestRun;
  editedAt: Record<string, number>;
}): string | null {
  const { discovery, run, disabledReason, editedAt } = args;
  if (disabledReason) return disabledReason;
  if (discovery.state === "discovering") {
    return "A discovery is running; wait for it to finish before running tests.";
  }
  if (discovery.state !== "completed") {
    return "A completed, current discovery is required before running tests.";
  }
  if (discovery.stale || isDiscoveryObsolete(discovery, editedAt)) {
    return (
      discovery.staleReason ?? "A buffer was edited after discovery; the list may not match the saved code."
    );
  }
  if (discovery.sealed) return "The discovery was sealed; discover again before running tests.";
  if (discovery.items.length === 0) return "The discovery listed no tests.";
  if (run.state === "running") return "A test run is already running.";
  return null;
}

/** Only ids from the last completed run of the current discovery; unknown stays out. */
export function failedTestIds(run: TestRun, discovery: TestsDiscovery): string[] {
  if (run.state !== "completed" || run.resultsTruncated) return [];
  if (run.discoveryId !== discovery.discoveryId || run.discoveryRevision !== discovery.revision) {
    return [];
  }
  const known = new Set(discovery.items.map((item) => item.id));
  const ids = new Set<string>();
  for (const result of run.results) {
    if (result.outcome !== "failed" && result.outcome !== "notExecuted") continue;
    if (result.testId && known.has(result.testId)) ids.add(result.testId);
  }
  return [...ids];
}

export function rerunFailedReason(args: {
  disabledReason: string | null;
  discovery: TestsDiscovery;
  run: TestRun;
  editedAt: Record<string, number>;
}): string | null {
  const blocked = testRunBlockedReason(args);
  if (blocked) return blocked;
  const { discovery, run, editedAt } = args;
  if (run.revision === 0 || run.state === "idle") {
    return "No completed run yet; run all tests first.";
  }
  if (run.discoveryId !== discovery.discoveryId || run.discoveryRevision !== discovery.revision) {
    return "The last run belongs to another discovery; run again from the current list.";
  }
  if (run.sealed) {
    return "The last run was sealed when the context changed; rerun failed is unavailable from it.";
  }
  if (run.stale || isRunObsolete(run, editedAt)) {
    return run.staleReason
      ? `The last run is stale: ${run.staleReason}`
      : "A buffer was edited after the last run; rerun failed is unavailable from it.";
  }
  if (run.state !== "completed") {
    return `The last run did not complete (${run.state}); only a completed run can rerun failed tests.`;
  }
  if (run.resultsTruncated) {
    return "The last run's result list was truncated; rerun failed cannot prove the full failed set.";
  }
  if (failedTestIds(run, discovery).length === 0) {
    return "The last completed run has no failed or not-executed results.";
  }
  return null;
}

/** Every tree state has its own text; an empty or missing tree is never reported as success. */
export function treeStateLabel(discovery: TestsDiscovery, stale: boolean, sealed: boolean) {
  switch (discovery.state) {
    case "discovering":
      return "Discovering tests — the tree is not current until discovery finishes.";
    case "empty":
      return "No tests listed — an empty tree is not a passing suite.";
    case "adapter_missing":
      return "No VSTest adapter — there is no test tree to show.";
    case "mtp_unsupported":
      return "Microsoft.Testing.Platform is not supported — there is no test tree to show.";
    case "failed":
      return "Discovery failed — there is no current test tree.";
    case "cancelled":
      return `Partial discovery — only ${discovery.items.length} test${
        discovery.items.length === 1 ? "" : "s"
      } were listed before cancellation.`;
    case "completed":
      if (sealed) return "Sealed discovery — this tree belongs to a previous context.";
      if (stale) return "Stale tree — a buffer changed after discovery; discover again.";
      if (discovery.partial) return "Partial discovery — the tree is incomplete and cannot be run.";
      return `${discovery.items.length} test${discovery.items.length === 1 ? "" : "s"} grouped from their fully qualified names.`;
    default:
      return "No discovery has run — the tree is empty.";
  }
}

function outcomeTone(result: TestResult) {
  return result.outcome === "failed" ? "error" : result.outcome === "passed" ? "success" : "";
}

function resultDuration(result: TestResult) {
  return result.durationMs === null ? "duration unknown" : `${result.durationMs} ms`;
}

export function TestsPanel({
  tests,
  evaluation,
  editedAt,
  disabledReason,
  autoDiscoveryError,
  projectPath,
  onProjectChange,
  selected,
  onToggleSelected,
  restoreRequired,
  restoreDisabledReason,
  onRestore,
  onEvaluate,
  onDiscover,
  onCancel,
  onRun,
  onRunCancel,
  onNavigate,
}: Props) {
  const discovery = tests.discovery;
  const run = tests.run;
  const candidates = evaluation.projects.filter(
    (project) => project.kind === "test" && project.state === "ok",
  );
  const selectedProject = candidates.find((project) => project.path === projectPath) ?? null;

  const staleByEdit = isDiscoveryObsolete(discovery, editedAt);
  const stale = discovery.stale || staleByEdit;
  const staleReason =
    discovery.staleReason ??
    "A buffer was edited after discovery; the list may not match the saved code.";
  const running = discovery.state === "discovering";
  const selectionBlocked =
    disabledReason !== null || running || stale || discovery.sealed || discovery.state !== "completed";
  const tfmOptions =
    evaluation.context.targetFramework &&
    !evaluation.availableTfms.includes(evaluation.context.targetFramework)
      ? [evaluation.context.targetFramework, ...evaluation.availableTfms]
      : evaluation.availableTfms;
  // The TFM this panel operates on: the discovery's own scope once it exists, otherwise the
  // evaluated selection. A solution whose projects declare a single framework has exactly one
  // possible TFM, so it is named instead of showing "Not selected" next to every test.
  const tfmValue =
    discovery.targetFramework ??
    evaluation.context.targetFramework ??
    (evaluation.availableTfms.length === 1 ? evaluation.availableTfms[0] : null);
  const terminalFailure = [
    "failed",
    "cancelled",
    "empty",
    "adapter_missing",
    "mtp_unsupported",
  ].includes(discovery.state);
  const discoverReason =
    disabledReason ??
    (!selectedProject
      ? "Select an evaluated test project explicitly; discovery never guesses."
      : null);
  const items = discovery.items;
  const runRunning = run.state === "running";
  const runObsolete = isRunObsolete(run, editedAt);
  const runBlocked = testRunBlockedReason({ disabledReason, discovery, run, editedAt });
  const retryBlocked =
    runBlocked ??
    (run.discoveryId !== discovery.discoveryId || run.discoveryRevision !== discovery.revision
      ? "This run belongs to another discovery; run again from the current list."
      : null);
  const failedIds = failedTestIds(run, discovery);
  const rerunBlocked = rerunFailedReason({ disabledReason, discovery, run, editedAt });
  const [filter, setFilter] = useState("");
  const [testsTab, setTestsTab] = useState<"explorer" | "session">("explorer");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [exported, setExported] = useState<{ path: string; bytes: number } | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const tree = buildTestTree(items);
  const visibleTree = filterTestTree(tree, filter);
  const visibleCount = visibleTree.reduce((total, group) => total + testCount(group), 0);
  const currentResults = new Map<string, TestResult>();
  if (run.discoveryId === discovery.discoveryId && run.discoveryRevision === discovery.revision) {
    for (const result of run.results) {
      if (result.testId) currentResults.set(result.testId, result);
    }
  }

  // G12: only failed outcomes are cycled; a new run id resets the cursor to the first failure.
  const failures = run.results
    .map((result, index) => ({ result, index }))
    .filter(({ result }) => result.outcome === "failed");
  const [failureCursor, setFailureCursor] = useState(-1);
  const resultRows = useRef(new Map<number, HTMLLIElement>());
  useEffect(() => {
    setFailureCursor(-1);
  }, [run.runId]);

  // Rider keeps the tree and shows the first failure in the right detail after a run.
  useEffect(() => {
    if (run.revision === 0 || selectedKey) return;
    if (!["completed", "failed", "cancelled"].includes(run.state)) return;
    const failed =
      run.results.find((result) => result.outcome === "failed" && result.testId) ??
      run.results.find((result) => result.testId);
    if (!failed?.testId) return;
    const findLeaf = (nodes: TreeNode[]): TestLeaf | null => {
      for (const node of nodes) {
        if (node.kind === "test") {
          if (node.item.id === failed.testId) return node;
        } else {
          const found = findLeaf(node.children);
          if (found) return found;
        }
      }
      return null;
    };
    const leaf = findLeaf(tree);
    if (leaf) setSelectedKey(leaf.key);
    // A completed run selects the first failure once; a user selection is never replaced.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run.revision, run.state]);

  function nextFailure() {
    if (failures.length === 0) return;
    const cursor = (failureCursor + 1) % failures.length;
    setFailureCursor(cursor);
    const { result, index } = failures[cursor];
    const row = resultRows.current.get(index);
    row?.querySelector("details")?.setAttribute("open", "");
    row?.scrollIntoView({ block: "nearest" });
    if (result.position) {
      onNavigate(result.position.path, {
        line: result.position.line,
        character: result.position.character,
      });
    }
  }

  function testStatus(id: string) {
    const result = currentResults.get(id);
    if (result) return { text: result.outcome, tone: outcomeTone(result) };
    if (run.revision === 0) return { text: "not run", tone: "" };
    if (run.discoveryId !== discovery.discoveryId || run.discoveryRevision !== discovery.revision) {
      return { text: "not in this run", tone: "" };
    }
    return { text: "not in last run", tone: "" };
  }

  function toggleGroup(key: string) {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  function groupKeys(nodes: TreeNode[]): string[] {
    return nodes.flatMap((node) =>
      node.kind === "group" ? [node.key, ...groupKeys(node.children)] : [],
    );
  }

  function leavesOf(node: TreeNode): TestLeaf[] {
    return node.kind === "test" ? [node] : node.children.flatMap(leavesOf);
  }

  function nodeTotals(node: TreeNode) {
    const totals = { passed: 0, failed: 0, skipped: 0, other: 0, notRun: 0, total: 0 };
    for (const leaf of leavesOf(node)) {
      totals.total += 1;
      const result = currentResults.get(leaf.item.id);
      if (result?.outcome === "passed") totals.passed += 1;
      else if (result?.outcome === "failed") totals.failed += 1;
      else if (result?.outcome === "skipped" || result?.outcome === "notExecuted") totals.skipped += 1;
      else if (result) totals.other += 1;
      else totals.notRun += 1;
    }
    return totals;
  }

  function groupTone(node: TreeNode) {
    const totals = nodeTotals(node);
    if (totals.failed > 0) return "error";
    if (totals.total > 0 && totals.passed === totals.total) return "success";
    return "";
  }

  async function exportResults() {
    setExporting(true);
    setExportError(null);
    try {
      setExported(
        await call<{ path: string; bytes: number }>("tests.export"),
      );
    } catch (error) {
      setExportError(messageOf(error));
    } finally {
      setExporting(false);
    }
  }

  function renderLeaf(leaf: TestLeaf) {
    const item = leaf.item;
    const index = leaf.index;
    const status = testStatus(item.id);
    return (
      <ul className="test-list test-tree-leaf" key={leaf.key}>
        <li
          data-testid={`test-item-${index}`}
          title={item.fullyQualifiedName}
          className={selectedKey === leaf.key ? "selected" : undefined}
          aria-selected={selectedKey === leaf.key}
          onClick={() => setSelectedKey(leaf.key)}
        >
          <label>
            <input
              type="checkbox"
              data-testid={`test-checkbox-${index}`}
              checked={selected.has(item.id)}
              disabled={selectionBlocked || item.occurrences > 1}
              onChange={() => onToggleSelected(item.id)}
              aria-label={itemLabel(item)}
            />
            <span>
              <span
                className={`tag ${status.tone}`}
                data-testid={`test-status-${index}`}
                title="Outcome from the last run of this discovery; absent results stay unknown."
              >
                {status.text}
              </span>{" "}
              <code data-testid={`test-identity-${index}`} title={item.fullyQualifiedName}>
                {item.fullyQualifiedName}
              </code>
              {item.occurrences > 1 && (
                <span className="warning" data-testid={`test-ambiguous-${index}`}>
                  {" "}
                  listed {item.occurrences} times by VSTest; the runner gave no unique identity, so it
                  cannot be run individually.
                </span>
              )}
              <span className="muted"> display name: {item.displayName}</span>
              <span className="muted">
                {" "}
                · {item.project} · {item.tfm ?? "TFM not selected"} · {item.source}
              </span>
            </span>
          </label>
        </li>
      </ul>
    );
  }

  function renderGroup(group: TestGroup, depth: number, parentTitle?: string): React.ReactNode {
    // A namespace prefix that repeats its project name adds a level and no information.
    if (group.level === "prefix" && group.title === parentTitle) {
      return group.children.map((node) =>
        node.kind === "group" ? renderGroup(node, depth, parentTitle) : renderLeaf(node),
      );
    }
    const folding = filter.trim() === "";
    const isCollapsed = folding && collapsed.has(group.key);
    const totals = nodeTotals(group);
    return (
      <div
        className={`test-group${selectedKey === group.key ? " selected" : ""}`}
        key={group.key}
        data-depth={depth}
      >
        <button
          type="button"
          className="test-group-title"
          aria-expanded={!isCollapsed}
          aria-selected={selectedKey === group.key}
          title={group.tooltip}
          onClick={() => {
            setSelectedKey(group.key);
            toggleGroup(group.key);
          }}
        >
          <span aria-hidden="true" className={`test-group-chevron ${isCollapsed ? "" : "open"}`}>
            <Icon name="chevron" />
          </span>
          <span className="test-group-name">{group.title}</span>
          <span
            className={`tag ${groupTone(group)}`}
            data-testid={`test-group-status-${group.key}`}
            title="Outcome aggregated from the last run of each test under this node."
          >
            {totals.failed > 0 ? `${totals.failed} failed` : totals.passed > 0 ? `${totals.passed} passed` : "not run"}
          </span>
          <span className="muted">
            {testCount(group)} test{testCount(group) === 1 ? "" : "s"}
          </span>
        </button>
        {!isCollapsed && (
          <div className="test-group-body">
            {group.children.map((node) =>
              node.kind === "group" ? renderGroup(node, depth + 1, group.title) : renderLeaf(node),
            )}
          </div>
        )}
      </div>
    );
  }

  function findNode(nodes: TreeNode[], key: string): TreeNode | null {
    for (const node of nodes) {
      if (node.key === key) return node;
      if (node.kind === "group") {
        const found = findNode(node.children, key);
        if (found) return found;
      }
    }
    return null;
  }

  // The detail panel is one named region in every state, so a reader (or a screen reader) lands
  // on the same place whether a test, a group or nothing is selected.
  function detailPanel(content: ReactNode) {
    return (
      <aside
        className="tests-detail"
        data-testid="tests-detail"
        role="region"
        aria-label="Test details"
        aria-live="polite"
      >
        {content}
      </aside>
    );
  }

  function renderDetail() {
    if (!selectedKey) {
      return detailPanel(
        <p className="muted">Select a test or group to see its last result and duration.</p>,
      );
    }
    const node = findNode(tree, selectedKey);
    if (!node) {
      return detailPanel(<p className="muted">The selected test is not in the current tree.</p>);
    }
    if (node.kind === "group") {
      const totals = nodeTotals(node);
      return detailPanel(
        <>
          <h3 data-testid="tests-detail-name">{node.title}</h3>
          <p className="menu-note">{node.tooltip}</p>
          <p className="muted" data-testid="tests-detail-counts">
            {totals.total} test{totals.total === 1 ? "" : "s"} · {totals.passed} passed ·{" "}
            {totals.failed} failed · {totals.skipped} skipped · {totals.other} unknown ·{" "}
            {totals.notRun} not run
          </p>
        </>,
      );
    }
    const result = currentResults.get(node.item.id);
    const status = testStatus(node.item.id);
    return detailPanel(
      <>
        <h3 data-testid="tests-detail-name">{node.item.fullyQualifiedName}</h3>
        <dl className="restore-meta">
          <dt>Status</dt>
          <dd data-testid="tests-detail-status">{status.text}</dd>
          <dt>Duration</dt>
          <dd data-testid="tests-detail-duration">
            {result ? resultDuration(result) : "not run"}
          </dd>
          <dt>Project</dt>
          <dd>{node.item.project}</dd>
          <dt>TFM</dt>
          <dd>{node.item.tfm ?? "TFM not selected"}</dd>
          <dt>Source</dt>
          <dd>
            <code>{node.item.source}</code>
          </dd>
        </dl>
        {result?.message && (
          <pre className="eval-pre" data-testid="tests-detail-message">
            {result.message}
          </pre>
        )}
        {result?.stackTrace && (
          <pre className="eval-pre" data-testid="tests-detail-stack">
            {result.stackTrace}
          </pre>
        )}
        {result?.position && (
          <button
            className="link-button"
            data-testid="tests-detail-navigate"
            onClick={() =>
              result.position &&
              onNavigate(result.position.path, {
                line: result.position.line,
                character: result.position.character,
              })
            }
          >
            {result.position.path}:{result.position.line + 1}
          </button>
        )}
      </>,
    );
  }

  function renderRunPanel() {
    if (run.revision === 0) return null;
    return (
        <section className="build-panel" aria-label="Test run" data-testid="tests-run-panel">
          <div className="build-controls">
            <span
              className={`tag ${run.state === "failed" ? "error" : ""}`}
              data-testid="tests-run-state"
              role="status"
            >
              {runStateLabel(run)}
            </span>
            <span className="sr-only" data-testid="tests-run-id">
              {run.runId ?? "no run"} · {run.discoveryId ?? "no discovery"} · discovery revision{" "}
              {run.discoveryRevision} · run revision {run.revision} · context{" "}
              {run.contextId ?? "unknown"}
            </span>
            <span className="muted" data-testid="tests-run-selection">
              {run.selection === "all"
                ? "Selection: all discovered tests"
                : `Selection: ${run.requestedCount} selected identit${run.requestedCount === 1 ? "y" : "ies"}`}
            </span>
            <button
              data-testid="tests-next-failure"
              disabled={failures.length === 0}
              onClick={nextFailure}
              title={
                failures.length === 0
                  ? "The last run has no failed results to navigate."
                  : `Expands the next failed result and opens its source position when one exists (${failures.length} failed in this run).`
              }
            >
              Next failure
            </button>
          </div>
          {run.stale && (
            <p className="warning" data-testid="tests-run-stale" role="status">
              These results are preserved but not current: {run.staleReason}
            </p>
          )}
          {!run.stale && runObsolete && (
            <p className="warning" data-testid="tests-run-stale" role="status">
              These results are preserved but not current: a buffer was edited after this run.
              Run tests again for the saved code.
            </p>
          )}
          {run.sealed && (
            <p className="warning" data-testid="tests-run-sealed">
              Previous run, not current: {run.staleReason}
            </p>
          )}
          {run.error && (
            <p
              className={run.infrastructureFailure ? "warning" : "muted"}
              data-testid={run.infrastructureFailure ? "tests-run-infrastructure" : "tests-run-note"}
              role="status"
            >
              {run.error}
            </p>
          )}
          <p className="muted" data-testid="tests-run-counts">
            {run.counts.total} result{run.counts.total === 1 ? "" : "s"} · {run.counts.passed} passed ·{" "}
            {run.counts.failed} failed · {run.counts.skipped} skipped · {run.counts.notExecuted} not
            executed · {run.counts.unknown} unknown
            {run.resultsTruncated ? " · results list truncated" : ""}
          </p>
          {(run.state === "cancelled" || run.resultsTruncated) && (
            <p className="warning" data-testid="tests-run-partial" role="status">
              {run.state === "cancelled"
                ? "Partial results: the run was cancelled; only the recorded results exist and they are not a passing suite."
                : "Partial results: the results list was truncated; the counts come from the run summary, not from the full list."}
            </p>
          )}
          <details className="build-context-details" aria-label="Test run details">
            <summary>Test run details</summary>
            <dl className="restore-meta build-meta">
            <dt>Command</dt>
            <dd>
              <code data-testid="tests-run-command">
                {run.command.length > 0 ? run.command.join(" ") : "No command was run."}
              </code>
            </dd>
            <dt>Exit</dt>
            <dd data-testid="tests-run-exit">
              {run.exit === null ? "exit unknown" : `exit ${run.exit}`}
            </dd>
            <dt>TRX</dt>
            <dd data-testid="tests-run-trx">
              <code>{run.trxPath ?? "no TRX"}</code>{" "}
              <span className="muted" data-testid="tests-run-trx-state">
                {run.trxState}
              </span>
            </dd>
            <dt>Raw log</dt>
            <dd data-testid="tests-run-log-path">
              <code>{run.logPath ?? "no log file"}</code>
            </dd>
            <dt>Results directory</dt>
            <dd>
              <code data-testid="tests-run-results">{run.resultsDirectory ?? "unknown"}</code>
            </dd>
            </dl>
          </details>
          {run.termination && (
            <p className="muted" data-testid="tests-run-termination">
              Termination requested: {run.termination.requested}. Observed:{" "}
              {run.termination.observed}
              {run.termination.killedAfterGrace ? " after the grace period" : ""}.
            </p>
          )}
          {run.awaitingDecision && (
            <p className="warning" data-testid="tests-run-awaiting">
              Still running; the host has not confirmed termination. Decide in the process panel.
            </p>
          )}
          {run.results.length > 0 && (
            <ul className="test-list" data-testid="tests-run-results-list" aria-label="Test results">
              {run.results.map((result, index) => (
                <li
                  key={`${run.runId}:${index}`}
                  data-testid={`test-result-${index}`}
                  ref={(element) => {
                    if (element) resultRows.current.set(index, element);
                    else resultRows.current.delete(index);
                  }}
                >
                  <span className={`tag ${outcomeTone(result)}`} data-testid={`test-result-outcome-${index}`}>
                    {result.outcome}
                  </span>{" "}
                  <code data-testid={`test-result-name-${index}`}>
                    {result.fullyQualifiedName || "unattributed (no TestDefinitions identity)"}
                  </code>{" "}
                  <span className="muted">
                    {resultDuration(result)} · source {result.outcomeRaw}
                    {result.matched
                      ? ""
                      : result.fullyQualifiedName
                        ? " · not matched to the discovery"
                        : ` · TRX testName only: ${result.displayName}`}
                  </span>
                  <details data-testid={`test-result-details-${index}`}>
                    <summary>Details</summary>
                    {result.message && (
                      <pre className="eval-pre" data-testid={`test-result-message-${index}`}>
                        {result.message}
                      </pre>
                    )}
                    {result.stackTrace && (
                      <>
                        <strong>Stack trace</strong>
                        <pre className="eval-pre" data-testid={`test-result-stack-${index}`}>
                          {result.stackTrace}
                        </pre>
                      </>
                    )}
                    {result.stdout && (
                      <>
                        <strong>stdout</strong>
                        <pre className="eval-pre" data-testid={`test-result-stdout-${index}`}>
                          {result.stdout}
                        </pre>
                      </>
                    )}
                    {result.stderr && (
                      <>
                        <strong>stderr</strong>
                        <pre className="eval-pre" data-testid={`test-result-stderr-${index}`}>
                          {result.stderr}
                        </pre>
                      </>
                    )}
                    {result.position ? (
                      <button
                        className="link-button"
                        data-testid={`test-result-navigate-${index}`}
                        onClick={() =>
                          result.position &&
                          onNavigate(result.position.path, {
                            line: result.position.line,
                            character: result.position.character,
                          })
                        }
                        title="Opens the file at the line the stack trace reported."
                      >
                        {result.position.path}:{result.position.line + 1}
                      </button>
                    ) : (
                      <p className="muted" data-testid={`test-result-no-position-${index}`}>
                        The source reported no `in &lt;file&gt;:line &lt;n&gt;` position inside the
                        workspace; use the raw run log instead.
                      </p>
                    )}
                  </details>
                </li>
              ))}
            </ul>
          )}
          <pre className="eval-pre build-log" data-testid="tests-run-log">
            {run.log || "No output yet."}
          </pre>
          {run.logTruncated && (
            <p className="muted" data-testid="tests-run-log-truncated">
              The run log is bounded and the earliest lines were dropped; the note at the top of
              the log says how many.
            </p>
          )}
        </section>

    );
  }

  return (
    <div className="build-panel" data-testid="tests-panel">
      <div className="tests-tabs" role="tablist" aria-label="Test views">
        <button
          type="button"
          role="tab"
          aria-selected={testsTab === "explorer"}
          className={`tab-label ${testsTab === "explorer" ? "active" : ""}`}
          data-testid="tests-tab-explorer"
          onClick={() => setTestsTab("explorer")}
        >
          Explorer
        </button>
        {discovery.project && (
          <button
            type="button"
            role="tab"
            aria-selected={testsTab === "session"}
            className={`tab-label ${testsTab === "session" ? "active" : ""}`}
            data-testid="tests-tab-session"
            onClick={() => setTestsTab("session")}
            title="Execution session for the current project discovery. A solution-wide session is not represented by the host model."
          >
            All tests from {discovery.project.name}
          </button>
        )}
        <button
          type="button"
          role="tab"
          aria-selected="false"
          disabled
          data-testid="tests-tab-continuous"
          title="Continuous Testing needs a file watcher and an isolated test session the host does not provide yet."
        >
          Continuous Testing
        </button>
        <button
          type="button"
          role="tab"
          aria-selected="false"
          disabled
          data-testid="tests-tab-coverage"
          title="Coverage needs a collector and a coverage report model the host does not provide yet."
        >
          Coverage
        </button>
      </div>

      <div className="tests-toolbar" role="toolbar" aria-label="Test actions">
        <span className="tag success" data-testid="tests-count-passed" title="Passed in the last run of this discovery">
          ✓ {run.counts.passed}
        </span>
        <span className="tag error" data-testid="tests-count-failed" title="Failed in the last run of this discovery">
          ✗ {run.counts.failed}
        </span>
        <span className="tag" data-testid="tests-count-skipped" title="Skipped or not executed in the last run">
          – {run.counts.skipped + run.counts.notExecuted}
        </span>
        <span className="tag" data-testid="tests-count-not-run" title="No result in the last run of this discovery">
          ? {run.revision === 0 ? items.length : Math.max(0, items.length - run.counts.matched)}
        </span>
        <label className="tests-filter-row">
          <span>Filter</span>
          <input
            data-testid="tests-filter"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder="Filter tests by name"
            title="Hides tests that do not match; groups with matches stay and the selection is unchanged."
          />
        </label>
        <span className="push" />
        <button
          type="button"
          data-testid="tests-expand-all"
          onClick={() => setCollapsed(new Set())}
          title="Expands every group in the tree."
        >
          <Icon name="expandAll" /> Expand all
        </button>
        <button
          type="button"
          data-testid="tests-collapse-all"
          onClick={() => setCollapsed(new Set(groupKeys(tree)))}
          title="Collapses every group; the tree and selection stay the same."
        >
          <Icon name="collapseAll" /> Collapse all
        </button>
        <button
          type="button"
          data-testid="tests-export"
          disabled={exporting || (discovery.revision === 0 && run.revision === 0)}
          onClick={() => void exportResults()}
          title="Writes the current discovery and the last run to a JSON file in the OpenRidr app home."
        >
          <Icon name="text" /> {exporting ? "Exporting…" : "Export"}
        </button>
        {exported && (
          <span className="muted" data-testid="tests-export-path" title={exported.path}>
            Exported {exported.bytes} bytes to <code>{exported.path}</code>
          </span>
        )}
        {exportError && (
          <span className="error" role="alert" data-testid="tests-export-error">
            {exportError}
          </span>
        )}
      </div>

      <div className="build-controls">
        <label className="eval-field">
          <span>Test project</span>
          <select
            data-testid="tests-project"
            value={selectedProject?.path ?? ""}
            disabled={running || candidates.length === 0}
            onChange={(event) => onProjectChange(event.target.value)}
            title={
              candidates.length === 0
                ? "No evaluated test project is available; evaluate a project whose IsTestProject is true."
                : "The test project is selected explicitly; discovery never scans other projects."
            }
          >
            {candidates.length === 0 && <option value="">No test project</option>}
            {candidates.length > 1 && !selectedProject && (
              <option value="">Select a test project</option>
            )}
            {candidates.map((project) => (
              <option key={project.path} value={project.path}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
        <label className="eval-field">
          <span>TFM</span>
          <select
            data-testid="tests-tfm"
            value={tfmValue ?? ""}
            disabled={running || evaluation.state === "running"}
            onChange={(event) =>
              onEvaluate({ targetFramework: event.target.value === "" ? null : event.target.value })
            }
            title="The TFM is part of the evaluated model; changing it re-evaluates before discovery."
          >
            <option value="">Not selected</option>
            {tfmOptions.map((tfm) => (
              <option key={tfm} value={tfm}>
                {tfm}
              </option>
            ))}
          </select>
        </label>
        <button
          data-testid="tests-discover"
          disabled={discoverReason !== null || running}
          onClick={() => selectedProject && onDiscover(selectedProject.path)}
          title={
            discoverReason ??
            "Runs the project's VSTest adapter through dotnet test --list-tests. Never restores, builds a solution or edits packages."
          }
        >
          {running ? "Discovering…" : discovery.revision > 0 ? "Refresh tests" : "Discover tests"}
        </button>
        {running && (
          <button
            data-testid="tests-cancel"
            onClick={onCancel}
            title="Requests termination of the discovery process group and keeps the partial list."
          >
            Stop discovery
          </button>
        )}
        {terminalFailure && !running && (
          <button
            data-testid="tests-retry"
            disabled={discoverReason !== null}
            onClick={() => selectedProject && onDiscover(selectedProject.path)}
            title={discoverReason ?? "Runs a new discovery of the same project and TFM."}
          >
            Retry discovery
          </button>
        )}
        <span
          className={`tag ${discovery.state === "failed" ? "error" : ""}`}
          data-testid="tests-state"
          role="status"
        >
          {discoveryStateLabel(discovery)}
        </span>
      </div>

      <div className="tests-body">
        <nav className="tests-actions" aria-label="Test actions">
          <button
            data-testid="tests-run-all"
            disabled={runBlocked !== null}
            onClick={() => selectedProject && onRun(selectedProject.path, "all", [])}
            title={
              runBlocked ??
              "Runs every identity of the current discovery with VSTest --no-build --no-restore and the built-in TRX logger."
            }
            aria-label="Run all tests"
          >
            <Icon name="run" />
            <span>Run</span>
          </button>
          <button
            data-testid="tests-run-selected"
            disabled={runBlocked !== null || selected.size === 0}
            onClick={() =>
              selectedProject && onRun(selectedProject.path, "selected", Array.from(selected))
            }
            title={
              runBlocked ??
              (selected.size === 0
                ? "Select at least one discovered test; an empty selection never runs all tests."
                : `Runs exactly ${selected.size} selected identit${selected.size === 1 ? "y" : "ies"}.`)
            }
            aria-label="Run selected tests"
          >
            <Icon name="tests" />
            <span>Selected</span>
          </button>
          <button
            data-testid="tests-debug-selected"
            disabled
            title={testDebugUnavailable}
            aria-label="Debug selected test (unavailable)"
          >
            <Icon name="debug" />
            <span>Debug</span>
          </button>
          <button
            data-testid="tests-rerun-failed"
            disabled={rerunBlocked !== null}
            onClick={() =>
              selectedProject && onRun(selectedProject.path, "selected", failedIds)
            }
            title={
              rerunBlocked ??
              `Reruns the ${failedIds.length} failed or not-executed identit${
                failedIds.length === 1 ? "y" : "ies"
              } from the last completed run of this discovery.`
            }
            aria-label="Rerun failed tests"
          >
            <Icon name="refresh" />
            <span>Rerun failed</span>
          </button>
          {runRunning && (
            <button
              data-testid="tests-run-cancel"
              onClick={onRunCancel}
              title="Requests termination of the test process group and keeps the partial results, TRX and log."
              aria-label="Stop test run"
            >
              <Icon name="stop" />
              <span>Stop</span>
            </button>
          )}
          {(run.state === "failed" || run.state === "cancelled") && !runRunning && (
            <button
              data-testid="tests-run-retry"
              disabled={retryBlocked !== null}
              onClick={() =>
                selectedProject &&
                onRun(
                  selectedProject.path,
                  run.selection,
                  run.selection === "selected" ? run.selectedTestIds : [],
                )
              }
              title={
                retryBlocked ??
                "Starts a new explicit run with the same selection; nothing is retried automatically."
              }
              aria-label="Retry test run"
            >
              <Icon name="restart" />
              <span>Retry</span>
            </button>
          )}
        </nav>
        <div className="tests-content">
          {testsTab === "session" ? (
            <div className="tests-session-view">
              <p className="muted">
                Session of {discovery.project?.name ?? "the current discovery"} — run output and
                per-test results. Switch to Explorer to inspect the tree.
              </p>
              {renderRunPanel()}
            </div>
          ) : (
            <>
      {disabledReason && (
        <p className="muted" data-testid="tests-disabled-reason">
          {disabledReason}
        </p>
      )}

      {autoDiscoveryError && (
        <p className="warning" data-testid="tests-auto-error" role="status">
          Automatic discovery did not start: {autoDiscoveryError}
        </p>
      )}

      {restoreRequired && (
        <div className="notice warning" data-testid="tests-restore-required" role="status">
          <strong>Test discovery needs restored assets.</strong> OpenRidr never restores inside
          discovery; run the explicit restore first.
          <div className="actions">
            <button
              data-testid="tests-restore-action"
              disabled={restoreDisabledReason !== null}
              onClick={onRestore}
              title={restoreDisabledReason ?? "Calls restore.start for this workspace."}
            >
              Restore now
            </button>
          </div>
        </div>
      )}

      {discovery.state === "mtp_unsupported" && (
        <p className="warning" data-testid="tests-mtp-message" role="status">
          {discovery.mtpMessage}
        </p>
      )}
      {discovery.state === "adapter_missing" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          {discovery.error}
        </p>
      )}
      {discovery.state === "empty" && (
        <p className="notice" data-testid="tests-guidance" role="status">
          {discovery.error}
        </p>
      )}
      {discovery.state === "failed" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          {discovery.error} The raw log below is complete for this run; Retry starts a new
          discovery.
        </p>
      )}
      {discovery.state === "cancelled" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          Discovery was cancelled; the {discovery.items.length} test
          {discovery.items.length === 1 ? "" : "s"} found so far {discovery.partial ? "are" : "is"}{" "}
          partial and cannot be selected. The log is preserved.
        </p>
      )}

      {discovery.revision > 0 && (
        <>
          <details className="build-context-details" aria-label="Discovery details">
            <summary>Discovery details</summary>
            <dl className="restore-meta build-meta">
            {discovery.discoveryId && (
              <>
                <dt>Discovery</dt>
                <dd data-testid="tests-discovery-id">
                  {discovery.discoveryId} · revision {discovery.revision} · context{" "}
                  {discovery.contextId ?? "unknown"}
                </dd>
              </>
            )}
            <dt>Tree</dt>
            <dd data-testid="tests-tree-state" role="status">
              {treeStateLabel(discovery, stale, discovery.sealed)}
            </dd>
            <dt>Grouping</dt>
            <dd data-testid="tests-grouping">
              Grouping derived from test names: project &gt; namespace prefix &gt; class &gt; method.
              Nothing is parsed from source files; every tooltip carries the full name.
            </dd>
            <dt>Scope</dt>
            <dd data-testid="tests-scope">
              {discovery.project?.name ?? "unknown"} ·{" "}
              <code>{discovery.project?.path ?? "unknown"}</code> · TFM{" "}
              {discovery.targetFramework ?? tfmValue ?? "not selected"}
            </dd>
            <dt>Command</dt>
            <dd>
              <code data-testid="tests-command">
                {discovery.command.length > 0
                  ? discovery.command.join(" ")
                  : "No command was run."}
              </code>
            </dd>
            <dt>Working directory</dt>
            <dd>
              <code data-testid="tests-cwd">{discovery.cwd || "unknown"}</code>
            </dd>
            <dt>Project SDK</dt>
            <dd data-testid="tests-sdk">
              {discovery.sdkVersion ?? "unknown"} ·{" "}
              <code>{discovery.sdkRoot ?? "unknown"}</code>
            </dd>
            <dt>Exit</dt>
            <dd data-testid="tests-exit">
              {discovery.exit === null ? "exit unknown" : `exit ${discovery.exit}`}
            </dd>
            </dl>
          </details>

          {discovery.sealed && (
            <p className="warning" data-testid="tests-sealed">
              Previous discovery, not current: {discovery.staleReason}
            </p>
          )}
          {!discovery.sealed && stale && (
            <p className="warning" data-testid="tests-stale" role="status">
              This discovery is stale, not failed: {staleReason} Discover again before selecting
              tests.
            </p>
          )}
          {discovery.termination && (
            <p className="muted" data-testid="tests-termination">
              Termination requested: {discovery.termination.requested}. Observed:{" "}
              {discovery.termination.observed}
              {discovery.termination.killedAfterGrace ? " after the grace period" : ""}.
            </p>
          )}
          {discovery.awaitingDecision && (
            <p className="warning" data-testid="tests-awaiting">
              Still running; the host has not confirmed termination. Decide in the process panel.
            </p>
          )}

          <p className="muted" data-testid="tests-counts">
            {items.length} test{items.length === 1 ? "" : "s"} · {selected.size} selected ·{" "}
            <span className="sr-only">{discovery.discoveryId ?? "no discovery"} · revision {discovery.revision}</span>
            {discovery.partial ? " · partial" : ""}
          </p>
          {items.length > 0 && visibleCount === 0 && (
            <p className="notice" data-testid="tests-filter-empty" role="status">
              No test matches “{filter}”. Clear the filter to see the {items.length} discovered
              test{items.length === 1 ? "" : "s"}; the selection was not changed.
            </p>
          )}
          {items.length > 0 && visibleCount > 0 && (
            <div className="test-tree" data-testid="tests-items" aria-label="Discovered tests">
              {visibleTree.map((group) => renderGroup(group, 0))}
            </div>
          )}
          {selected.size > 0 && stale && (
            <p className="muted" data-testid="tests-selection-blocked">
              Selection is disabled until discovery runs again for the current code.
            </p>
          )}
          <pre className="eval-pre build-log" data-testid="tests-log">
            {discovery.log || "No output yet."}
          </pre>
          {discovery.logTruncated && (
            <p className="muted" data-testid="tests-log-truncated">
              The discovery log is bounded and the earliest lines were dropped; the note at the top
              of the log says how many.
            </p>
          )}
        </>
      )}
            </>
          )}
        </div>
        <aside className="tests-side">
          {renderDetail()}
          {testsTab === "explorer" && renderRunPanel()}
        </aside>
      </div>
    </div>
  );
}
