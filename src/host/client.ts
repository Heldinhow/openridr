export type WorkspaceKind = "solution" | "project";

export type Candidate = {
  path: string;
  name: string;
  kind: WorkspaceKind;
};

export type OutlineProject = {
  name: string;
  path: string;
  kind: "console" | "library" | "test" | "unknown";
  tfms: string[];
  references: string[];
  files: string[];
};

export type GlobalJson = {
  path: string;
  version: string | null;
  rollForward: string | null;
};

export type SdkVersion = { version: string; path: string };

export type SdkRoot = {
  root: string;
  source: string;
  versions: SdkVersion[];
  exit: number | null;
  error: string | null;
};

export type HostSdk = {
  state: "found" | "missing";
  version: string | null;
  root: string | null;
  sdkPath: string | null;
  instructionsUrl: string;
  message: string | null;
};

export type ProjectSdk = {
  state:
    | "no_workspace"
    | "not_inspected"
    | "resolved"
    | "sdk_missing"
    | "no_global_json"
    | "not_pinned"
    | "invalid_global_json";
  globalJsonPath: string | null;
  requestedVersion: string | null;
  rollForward: string | null;
  allowPrerelease: boolean | null;
  resolvedVersion: string | null;
  root: string | null;
  sdkPath: string | null;
  instructionsUrl: string;
  message: string | null;
};

export type ProcessInfo = {
  id: number;
  kind: string;
  pid: number;
  workspaceId: number;
  state: "running" | "exited";
  exit: number | null;
  awaitingDecision: boolean;
  kept: boolean;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
};

export type EvaluatedProjectRef = { name: string; path: string };
export type EvaluatedPackageRef = { name: string; version: string | null };

export type EvaluationProject = {
  name: string;
  path: string;
  folder: string | null;
  state: "ok" | "failed";
  kind: "console" | "library" | "test" | "unknown";
  tfms: string[];
  evaluatedTfm: string | null;
  targetPath: string | null;
  outputType: string | null;
  projectRefs: EvaluatedProjectRef[];
  packageRefs: EvaluatedPackageRef[];
  sources: string[];
  error: string | null;
  log: string;
  exit: number | null;
  restored: "restored" | "not_restored" | "unknown";
  restoredReason: string | null;
  note: string | null;
};

export type Evaluation = {
  state: "idle" | "running" | "ok" | "partial" | "failed";
  context: { configuration: string; targetFramework: string | null };
  configurations: string[];
  availableTfms: string[];
  projects: EvaluationProject[];
  log: string;
  stale: boolean;
  staleReason: string | null;
  revision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
};

export type RestoreProject = {
  name: string;
  path: string;
  state: "restored" | "not_restored" | "unknown";
  reason: string | null;
  assetsPath: string | null;
  assetsModifiedAt: number | null;
  projectModifiedAt: number | null;
};

export type Restore = {
  state: "idle" | "running" | "ok" | "failed" | "cancelled";
  scope: "solution" | "project";
  target: string;
  command: string[];
  cwd: string;
  sdkRoot: string | null;
  sdkVersion: string | null;
  log: string;
  logTruncated: boolean;
  exit: number | null;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
  processId: number | null;
  awaitingDecision: boolean;
  projects: RestoreProject[];
  revision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  sealed: boolean;
  staleReason: string | null;
};

export type InstallSource = { name: string; url: string; packageUrl: string };

export type InstallExisting = {
  state: "present" | "absent" | "ready";
  binary: string;
  verifiedVersion: string | null;
  output: string | null;
  sha256: string | null;
  reason: string | null;
};

export type InstallPlan = {
  tool: string;
  version: string;
  package: string;
  source: InstallSource;
  os: string;
  arch: string;
  destination: string;
  staging: string;
  dotnetRoot: string;
  argv: string[];
  cwd: string;
  permissions: string[];
  integrity: string;
  existing: InstallExisting;
  planId: string;
};

export type ToolInstall = {
  state:
    | "absent"
    | "planned"
    | "downloading"
    | "installing"
    | "verifying"
    | "ready"
    | "cancelled"
    | "failed";
  version: string;
  destination: string;
  staging: string;
  plan: InstallPlan | null;
  log: string;
  logTruncated: boolean;
  exit: number | null;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
  processId: number | null;
  awaitingDecision: boolean;
  sha256: string | null;
  verifiedVersion: string | null;
  verifiedOutput: string | null;
  error: string | null;
  note: string | null;
  reused: boolean;
  revision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  sealed: boolean;
  staleReason: string | null;
  stagingPresent: boolean;
  destinationPresent: boolean;
};

export type LspCapabilities = {
  completion: boolean;
  definition: boolean;
  rename: boolean;
  codeActions: boolean;
  pushDiagnostics: boolean;
  pullDiagnostics: boolean;
  semanticTokens: boolean;
  synchronization: unknown;
  summary: string[];
};

export type ProblemItem = {
  id: string;
  severity: number | null;
  severityLabel: "error" | "warning" | "information" | "hint" | "unknown";
  severityRaw: number | null;
  code: string | number | null;
  source: string | null;
  category: "compiler" | "analyzer" | "unknown";
  message: string;
  position: { line: number; character: number } | null;
  range: unknown;
  codeDescription: string | null;
  stale: boolean;
};

export type DiagnosticSnapshot = {
  id: string;
  origin: { id: string; label: string };
  session: number;
  workspaceId: number;
  contextId: number;
  uri: string;
  path: string | null;
  receivedAt: number;
  clientRevision: number | null;
  currentRevision: number | null;
  revisionKnown: boolean;
  serverVersion: unknown;
  state: "current" | "awaiting" | "stale";
  staleReason: string | null;
  items: ProblemItem[];
};

export type Diagnostics = {
  session: number;
  origin: { id: string; label: string };
  snapshots: DiagnosticSnapshot[];
  awaiting: { origin: { id: string; label: string }; session: number; path: string; revision: number }[];
  counts: { error: number; warning: number; information: number; hint: number; unknown: number };
  summaries: {
    origin: { id: string; label: string };
    session: number;
    currentItems: number;
    staleItems: number;
    unavailable: string | null;
  }[];
  currentItems: number;
  staleItems: number;
  coverage: "full" | "partial";
  coverageNote: string | null;
  analyzerCoverage: "not_verified";
  analysisUnavailable: string | null;
};

export type LspPosition = { line: number; character: number };

export type LspRange = { start: LspPosition; end: LspPosition };

export type CompletionItem = {
  label: string;
  detail: string | null;
  kind: string | null;
  kindRaw: number | null;
  insertText: string | null;
  textEdit: { range: LspRange; newText: string } | null;
  documentation: string | null;
  sortText: string | null;
};

export type DefinitionLocation = {
  path: string;
  uri: string;
  line: number;
  character: number;
  range: LspRange;
  selectionRange: LspRange | null;
};

/** Shared answer for `lsp.completion` and `lsp.definition`. Unavailable/unsupported answers
 * are data with empty lists, never fabricated items. */
export type AnalysisResponse = {
  method: "lsp.completion" | "lsp.definition";
  state: "ok" | "unsupported" | "unavailable" | "stale" | "timeout" | "failed";
  message: string | null;
  session: number;
  workspaceId: number;
  contextId: number;
  path: string;
  line: number;
  character: number;
  revision: number | null;
  clientRevision: number | null;
  documentKnown: boolean;
  coverage: "full" | "partial";
  coverageNote: string | null;
  items: CompletionItem[];
  itemCount: number;
  totalItems: number;
  truncated: boolean;
  isIncomplete: boolean;
  replaceRange: LspRange | null;
  locations: DefinitionLocation[];
  rejectedTargets: number;
};

export type Lsp = {
  state:
    | "unavailable"
    | "starting"
    | "loading"
    | "ready"
    | "partial"
    | "awaiting_diagnostics"
    | "failed"
    | "stopped";
  session: number;
  logLevel: "info" | "debug";
  log: string;
  logTruncated: boolean;
  error: string | null;
  capabilities: LspCapabilities | null;
  serverInfo: { name?: string; version?: string } | null;
  hostRoot: string;
  hostRootRebuilt: boolean;
  hostRootForced: boolean;
  hostRootCheck: string;
  solution: string | null;
  command: string[];
  cwd: string;
  processId: number | null;
  pid: number | null;
  exit: number | null;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
  coverage: "full" | "partial";
  coverageNote: string | null;
  publicationCount: number;
  documents: number;
  awaitingDocuments: string[];
  diagnosticItems: number;
  outputClosed: boolean;
  startedAt: number | null;
  finishedAt: number | null;
  stoppedReason: string | null;
  workspaceId: number | null;
  contextId: number | null;
};

export type Snapshot = {
  contextId: number;
  app: { home: string; appHome: string; platform: string; arch: string; version: string };
  workspace: null | {
    id: number;
    path: string;
    name: string;
    kind: WorkspaceKind;
    mode: "restricted" | "trusted";
    candidates: Candidate[];
  };
  outline: null | {
    source: "text";
    projects: OutlineProject[];
    globalJson: GlobalJson | null;
  };
  trust: {
    granted: boolean;
    workspaceId: number | null;
    decidedAt: number | null;
    revokedAt: number | null;
  };
  sdk: {
    host: HostSdk;
    project: ProjectSdk;
    roots: SdkRoot[];
    inspectedAt: number | null;
  };
  debug: {
    adapterState: "not_checked";
    adapterMessage: string;
    appleSiliconExperimental: boolean;
  };
  evaluation: Evaluation;
  processes: ProcessInfo[];
  restore: Restore;
  tools: { "csharp-ls": ToolInstall };
  lsp: Lsp;
  diagnostics: Diagnostics;
};

export type ReadFile = { path: string; content: string; hash: string };
export type WriteFile = { path: string; hash: string };

export class HostCallError extends Error {
  code: string;
  details?: unknown;

  constructor(error: { code?: string; message?: string; details?: unknown }) {
    super(error.message ?? "Host call failed");
    this.name = "HostCallError";
    this.code = error.code ?? "host_error";
    this.details = error.details;
  }
}

function toHostError(error: unknown): HostCallError {
  if (error instanceof HostCallError) {
    return error;
  }
  if (typeof error === "object" && error !== null) {
    const value = error as { code?: string; message?: string; details?: unknown };
    if (value.code || value.message) {
      return new HostCallError(value);
    }
  }
  return new HostCallError({ message: String(error) });
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const hostUrl = import.meta.env.VITE_HOST_URL ?? "http://127.0.0.1:4317";

export async function call<T>(method: string, params: unknown = {}): Promise<T> {
  if (window.__TAURI_INTERNALS__) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return await invoke<T>("host_call", { method, params });
    } catch (error) {
      throw toHostError(error);
    }
  }

  let response: Response;
  try {
    response = await fetch(`${hostUrl}/call`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ method, params }),
    });
  } catch {
    throw new HostCallError({
      code: "host_unreachable",
      message: `Cannot reach the host bridge at ${hostUrl}. Start it with: bun run host:http`,
    });
  }

  const body = (await response.json().catch(() => null)) as
    | { ok: true; data: T }
    | { ok: false; error: { code?: string; message?: string; details?: unknown } }
    | null;
  if (!body) {
    throw new HostCallError({ code: "host_error", message: "Host returned an invalid response." });
  }
  if (!body.ok) {
    throw new HostCallError(body.error);
  }
  return body.data;
}
