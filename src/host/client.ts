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
  assemblyPath: string | null;
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

export type BuildProject = {
  name: string;
  path: string;
  kind: "console" | "library" | "test" | "unknown";
  targetPath: string | null;
};

/** One real MSBuild diagnostic parsed from the raw text. The position is zero-based and
 * exists only when MSBuild printed one; `category` stays unknown without evidence. */
export type BuildProblem = {
  id: string;
  severity: "error" | "warning";
  severityLabel: "error" | "warning";
  severityRaw: number;
  code: string | null;
  source: "dotnet";
  category: "unknown";
  message: string;
  path: string;
  position: { line: number; character: number } | null;
  range: LspRange | null;
  buildId: string;
  revision: number;
  raw: string;
};

export type Build = {
  state: "idle" | "running" | "ok" | "failed" | "cancelled";
  project: BuildProject | null;
  configuration: string;
  targetFramework: string | null;
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
  revision: number;
  buildId: string | null;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  stale: boolean;
  staleReason: string | null;
  sealed: boolean;
  savedNote: string | null;
  problems: BuildProblem[];
  problemsTruncated: boolean;
};

/** One test listed by the project's VSTest adapter. The fully-qualified name is the identity;
 * `displayName` is only what the runner printed. */
export type TestItem = {
  id: string;
  fullyQualifiedName: string;
  displayName: string;
  source: "vstest";
  project: string;
  tfm: string | null;
  discoveryId: string;
  revision: number;
  /** How many times VSTest printed this name; >1 means it is not a unique identity. */
  occurrences: number;
};

/** Discovery is one explicit `dotnet test --list-tests` operation. Preflight states
 * (`adapter_missing`, `mtp_unsupported`) have no command and never ran a VSTest process. */
export type TestsDiscovery = {
  state:
    | "idle"
    | "discovering"
    | "completed"
    | "empty"
    | "failed"
    | "cancelled"
    | "adapter_missing"
    | "mtp_unsupported";
  project: { name: string; path: string } | null;
  targetFramework: string | null;
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
  revision: number;
  discoveryId: string | null;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  items: TestItem[];
  partial: boolean;
  error: string | null;
  mtpMessage: string | null;
  stale: boolean;
  staleReason: string | null;
  sealed: boolean;
};

export type Tests = { discovery: TestsDiscovery; run: TestRun };

/** A source position derived only from a TRX stack trace `in <file>:line <n>` that resolves
 * inside the workspace. `line` is 0-based. */
export type TestPosition = { path: string; line: number; character: number };

/** One real TRX UnitTestResult, mapped back to a discovery identity through TestDefinitions.
 * `outcome` is the source's own outcome; absence stays unknown, never passed. */
export type TestResult = {
  testId: string | null;
  fullyQualifiedName: string;
  displayName: string;
  outcome: "passed" | "failed" | "skipped" | "notExecuted" | "unknown";
  outcomeRaw: string;
  durationMs: number | null;
  message: string | null;
  stackTrace: string | null;
  stdout: string | null;
  stderr: string | null;
  matched: boolean;
  position: TestPosition | null;
};

export type TestRun = {
  state: "idle" | "running" | "completed" | "failed" | "cancelled";
  project: { name: string; path: string } | null;
  configuration: string;
  targetFramework: string | null;
  selection: "all" | "selected";
  selectedTestIds: string[];
  requestedCount: number;
  command: string[];
  cwd: string;
  resultsDirectory: string | null;
  trxPath: string | null;
  logPath: string | null;
  trxState: "none" | "ok" | "partial" | "missing" | "corrupt";
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
  revision: number;
  runId: string | null;
  discoveryId: string | null;
  discoveryRevision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  results: TestResult[];
  resultsTruncated: boolean;
  counts: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
    notExecuted: number;
    unknown: number;
    matched: number;
  };
  infrastructureFailure: boolean;
  error: string | null;
  sealed: boolean;
  stale: boolean;
  staleReason: string | null;
};

/** The program launched from the identified successful build. `exited` is neutral: the
 * exit code is shown as observed and `null` stays unknown. */
export type Run = {
  state: "idle" | "running" | "exited" | "failed" | "cancelled";
  project: { name: string; path: string } | null;
  targetPath: string | null;
  buildId: string | null;
  configuration: string;
  targetFramework: string | null;
  command: string[];
  cwd: string;
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
  revision: number;
  runId: string | null;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  sealed: boolean;
  stale: boolean;
  staleReason: string | null;
};

/** One real PTY shell. `exit:null` and `exit unknown` are the same fact: never a success. */
export type Terminal = {
  state: "idle" | "running" | "exited" | "stopped" | "failed";
  shell: string;
  command: string[];
  cwd: string;
  cols: number;
  rows: number;
  log: string;
  logTruncated: boolean;
  outputSeq: number;
  exit: number | null;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
  processId: number | null;
  awaitingDecision: boolean;
  kept: boolean;
  revision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  staleReason: string | null;
  sealed: boolean;
};

export type InstallSource = { name: string; url: string; packageUrl: string };export type InstallExisting = {
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

/** netcoredbg asset as published by the approved official release. Only assets actually
 * present in the release metadata are listed; `availability` never implies DAP support. */
export type DebugAsset = {
  platform: string;
  name: string;
  size: number;
  sha256: string;
  archive: string;
  support: string;
  availability: "available" | "unverified" | "unsupported_archive" | "absent";
  reason: string | null;
  url: string;
  testOverride: boolean;
};

export type DebugExisting = {
  state: "present" | "absent" | "ready";
  binary: string;
  verifiedVersion: string | null;
  sha256: string | null;
  reason: string | null;
};

export type DebugPlan = {
  tool: string;
  version: string;
  source: { name: string; url: string };
  os: string;
  arch: string;
  platform: string;
  asset: DebugAsset | null;
  availability: DebugAsset["availability"];
  availabilityReason: string | null;
  destination: string;
  staging: string;
  operation: string;
  permissions: string[];
  integrity: string;
  limits: string[];
  blockedReason: string | null;
  blockedCode: string | null;
  existing: DebugExisting;
  planId: string;
};

/** The debugger adapter state. `ready` proves the executable and version, never DAP. */
export type DebugAdapter = {
  adapterState:
    | "not_checked"
    | "unavailable"
    | "not_installed"
    | "planned"
    | "downloading"
    | "installing"
    | "verifying"
    | "ready"
    | "cancelled"
    | "failed"
    | "experimental_disabled";
  adapterMessage: string;
  appleSiliconExperimental: boolean;
  version: string;
  source: { name: string; url: string };
  platform: string;
  os: string;
  arch: string;
  assets: DebugAsset[];
  asset: DebugAsset | null;
  availability: DebugAsset["availability"];
  availabilityReason: string | null;
  installable: boolean;
  gateReason: string | null;
  dapProven: false;
  destination: string;
  staging: string;
  plan: DebugPlan | null;
  log: string;
  logTruncated: boolean;
  progress: { downloaded: number; total: number | null } | null;
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
  busy: boolean;
};

export type DapBreakpoint = {
  path: string;
  line: number;
  verified: boolean;
  message: string | null;
};

export type DapThread = { id: number; name: string };

export type DapFrame = {
  id: number;
  name: string;
  line: number;
  column: number;
  path: string | null;
  sourceName: string | null;
};

export type DapVariable = {
  name: string;
  value: string;
  type: string | null;
  variablesReference: number;
};

export type DapScope = {
  name: string;
  expensive: boolean;
  variables: DapVariable[];
};

export type DapLocals = { frameId: number; scopes: DapScope[] };

/** One real DAP session. `exitKnown:false` means the adapter never reported an exit code. */
export type Dap = {
  state: "idle" | "preparing" | "running" | "paused" | "terminated" | "cancelled" | "failed";
  phase: string;
  session: number;
  project: { name: string; path: string } | null;
  buildId: string | null;
  assembly: string | null;
  configuration: string;
  targetFramework: string | null;
  adapter: { binary: string; version: string; platform: string; argv: string[] };
  command: string[];
  cwd: string;
  processId: number | null;
  pid: number | null;
  debuggeePid: number | null;
  exit: number | null;
  exitKnown: boolean;
  adapterExit: number | null;
  termination: null | {
    requested: string;
    observed: string;
    exit: number | null;
    killedAfterGrace: boolean;
  };
  threads: DapThread[];
  frames: DapFrame[];
  selectedFrameId: number | null;
  locals: DapLocals | null;
  stopped: null | { reason: string; threadId: number | null; allThreadsStopped: boolean };
  breakpoints: DapBreakpoint[];
  terminals: {
    processId: number;
    pid: number;
    command: string[];
    state: string;
    exit: number | null;
  }[];
  log: string;
  logTruncated: boolean;
  output: string;
  outputTruncated: boolean;
  revision: number;
  workspaceId: number | null;
  contextId: number | null;
  startedAt: number | null;
  finishedAt: number | null;
  sealed: boolean;
  staleReason: string | null;
};

export type LspCapabilities = {
  completion: boolean;
  definition: boolean;
  rename: boolean;
  prepareRename: boolean;
  codeActions: boolean;
  workspaceSymbol: boolean;
  references: boolean;
  documentSymbol: boolean;
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

export type WorkspaceFiles = {
  files: string[];
  returned: number;
  total: number;
  truncated: boolean;
};

export type TextMatch = { line: number; character: number; text: string };
export type TextFileResult = { path: string; matches: TextMatch[] };

export type TextSearch = {
  query: string;
  caseSensitive: boolean;
  files: TextFileResult[];
  fileCount: number;
  matchCount: number;
  truncated: boolean;
};

export type WorkspaceSymbol = {
  name: string;
  kind: number;
  containerName: string;
  uri: string;
  path: string;
  range: LspRange;
};

export type WorkspaceSymbolResult = {
  state: "ok" | "analysis_not_ready" | "unsupported" | "stale" | "failed";
  message: string | null;
  missingAction: string | null;
  session: number;
  items: WorkspaceSymbol[];
  total: number;
  truncated: boolean;
};

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

export type RenameChange = {
  line: number;
  before: string;
  after: string;
  displayTruncated: boolean;
};

/** One document of a rename preview. Text is computed by the host from the LSP UTF-16
 * ranges; the UI never does offset math and never writes these contents to disk. */
export type RenameDocument = {
  path: string;
  uri: string;
  opened: boolean;
  documentVersion: number | null;
  editCount: number;
  edits: { range: LspRange; newText: string }[];
  changes: RenameChange[];
  displayTruncated: boolean;
  baseContent: string;
  newContent: string;
  baseHash: string;
};

export type RenameResponse = AnalysisResponse & {
  method: "lsp.rename";
  newName: string;
  renameId: string;
  prepared: boolean;
  documents: RenameDocument[];
  documentCount: number;
  editCount: number;
};

export type CodeActionCommand = {
  title: string;
  command: string;
};

/** One real action from csharp-ls. Unsupported actions keep their reason and never carry a
 * preview; no command is ever executed. */
export type CodeAction = {
  actionId: string;
  title: string;
  kind: string | null;
  isPreferred: boolean;
  disabled: string | null;
  supported: boolean;
  reason: string | null;
  command: CodeActionCommand | null;
  hasData: boolean;
  documents: RenameDocument[];
  documentCount: number;
  editCount: number;
};

export type CodeActionsResponse = AnalysisResponse & {
  method: "lsp.codeActions";
  actionId: string | null;
  actions: CodeAction[];
  actionCount: number;
  totalActions: number;
  truncated: boolean;
  rejectedActions: number;
  unsupportedActions: number;
  diagnosticsUsed: number;
};

export type DocumentSymbol = {
  name: string;
  kind: number;
  containerName: string;
  range: LspRange;
  selectionRange: LspRange;
  children: DocumentSymbol[];
};

export type DocumentSymbolResponse = AnalysisResponse & {
  method: "lsp.documentSymbol";
  symbols: DocumentSymbol[];
  symbolCount: number;
  totalSymbols: number;
  rejectedSymbols: number;
};

export type ReferencesResponse = AnalysisResponse & {
  method: "lsp.references";
  references: DefinitionLocation[];
  referenceCount: number;
  totalReferences: number;
  rejectedReferences: number;
};

/** Shared answer for `lsp.completion`, `lsp.definition`, `lsp.rename`, `lsp.codeActions`,
 * `lsp.references` and `lsp.documentSymbol`. Unavailable/unsupported answers are data with
 * empty lists, never fabricated items. */
export type AnalysisResponse = {
  method:
    | "lsp.completion"
    | "lsp.definition"
    | "lsp.rename"
    | "lsp.codeActions"
    | "lsp.references"
    | "lsp.documentSymbol";
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
  documentVersion: number | null;
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

export type RunConfig = {
  id: string;
  name: string;
  project: string;
  configuration: string;
  targetFramework: string | null;
  args: string[];
  workingDirectory: string;
  env: Record<string, string>;
  envFiles: string[];
};

export type RunConfigIssue = {
  configId: string;
  field: string;
  reason: string;
  message: string;
};

export type RunConfigStore = {
  solution: string;
  path: string;
  activeId: string;
  configs: RunConfig[];
  issues: RunConfigIssue[];
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
  debug: DebugAdapter;
  evaluation: Evaluation;
  processes: ProcessInfo[];
  restore: Restore;
  build: Build;
  run: Run;
  terminal: Terminal;
  tests: Tests;
  dap: Dap;
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
