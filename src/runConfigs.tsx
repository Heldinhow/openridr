import { useState, type MouseEvent, type ReactNode } from "react";
import {
  HostCallError,
  messageOf,
  type Evaluation,
  type EvaluationProject,
  type RunConfig,
  type RunConfigIssue,
} from "./host/client";
import { Icon } from "./Icon";
import { MidText } from "./shell/MidText";

export const DEFAULT_RUN_CONFIG_ID = "default";

export type RunConfigView = RunConfig & { readOnly?: boolean };

function projectDirectory(projectPath: string) {
  const index = projectPath.lastIndexOf("/");
  return index === -1 ? "" : projectPath.slice(0, index);
}

export function deriveDefaultRunConfig(
  project: EvaluationProject | null,
  evaluation: Evaluation,
): RunConfigView | null {
  if (!project) return null;
  return {
    id: DEFAULT_RUN_CONFIG_ID,
    name: `Run ${project.name}`,
    project: project.path,
    configuration: evaluation.context.configuration,
    targetFramework: evaluation.context.targetFramework,
    args: [],
    workingDirectory: projectDirectory(project.path),
    env: {},
    envFiles: [],
    readOnly: true,
  };
}

function configIssues(issues: RunConfigIssue[], id: string) {
  return issues.filter((issue) => issue.configId === id);
}

function errorField(error: unknown) {
  if (!(error instanceof HostCallError)) return null;
  const details = error.details as { field?: string } | undefined;
  return details?.field ?? null;
}

export function RunConfigWidget({
  configs,
  defaultConfig,
  activeId,
  disabledReason,
  onSelect,
  onEdit,
}: {
  configs: RunConfig[];
  defaultConfig: RunConfigView | null;
  activeId: string;
  disabledReason: string | null;
  onSelect: (id: string) => void;
  onEdit: () => void;
}) {
  const available = defaultConfig !== null || configs.length > 0;
  const active: RunConfigView | null =
    activeId === DEFAULT_RUN_CONFIG_ID
      ? defaultConfig
      : configs.find((config) => config.id === activeId) ?? defaultConfig;
  const reason =
    disabledReason ??
    (available
      ? null
      : "No run configuration is available; evaluate a solution with a console startup project.");
  const title = reason ?? `Run configuration: ${active?.name ?? "none"}. Run and Debug use it.`;
  if (reason) {
    return (
      <span className="toolbar-disabled" title={title}>
        <button
          type="button"
          data-testid="toolbar-run-config"
          disabled
          aria-label={title}
          title={title}
        >
          <Icon name="runconfig" /> {active?.name ?? "Run config"}
        </button>
      </span>
    );
  }
  const choose = (id: string, event: MouseEvent<HTMLButtonElement>) => {
    event.currentTarget.closest("details")?.removeAttribute("open");
    onSelect(id);
  };
  return (
    <details className="toolbar-menu run-config-menu" data-testid="run-config-menu">
      <summary data-testid="toolbar-run-config" title={title} aria-label={title}>
        <Icon name="runconfig" />
        <span data-testid="run-config-active"><MidText text={active?.name ?? "Run config"} /></span>
        {active?.readOnly && <span className="run-config-tag">default</span>}
        <Icon name="chevron" />
      </summary>
      <div className="menu-content">
        <span className="menu-heading">Run configuration</span>
        {defaultConfig && (
          <button
            type="button"
            data-testid="run-config-option-default"
            aria-current={activeId === DEFAULT_RUN_CONFIG_ID ? "true" : undefined}
            title="Derived from the current evaluation; read only."
            onClick={(event) => choose(DEFAULT_RUN_CONFIG_ID, event)}
          >
            {defaultConfig.name}
            <span className="menu-note">Derived from the evaluation; read only.</span>
          </button>
        )}
        {configs.map((config) => (
          <button
            key={config.id}
            type="button"
            data-testid={`run-config-option-${config.id}`}
            aria-current={activeId === config.id ? "true" : undefined}
            title={`${config.project} · ${config.configuration}${config.targetFramework ? ` · ${config.targetFramework}` : ""}`}
            onClick={(event) => choose(config.id, event)}
          >
            {config.name}
          </button>
        ))}
        <button
          type="button"
          data-testid="run-config-edit"
          onClick={(event) => {
            event.currentTarget.closest("details")?.removeAttribute("open");
            onEdit();
          }}
        >
          Edit run configurations…
        </button>
      </div>
    </details>
  );
}

type Draft = {
  id: string;
  name: string;
  project: string;
  configuration: string;
  targetFramework: string;
  argumentsText: string;
  workingDirectory: string;
  envRows: { key: string; value: string }[];
  envFiles: string[];
};

function newId() {
  return `config-${crypto.randomUUID()}`;
}

function toDraft(config: RunConfigView): Draft {
  return {
    id: config.id,
    name: config.name,
    project: config.project,
    configuration: config.configuration,
    targetFramework: config.targetFramework ?? "",
    argumentsText: config.args.join("\n"),
    workingDirectory: config.workingDirectory,
    envRows: Object.entries(config.env).map(([key, value]) => ({ key, value })),
    envFiles: [...config.envFiles],
  };
}

function blankDraft(
  evaluation: Evaluation,
  defaultConfig: RunConfigView | null,
): Draft {
  const project =
    defaultConfig?.project ??
    evaluation.projects.find((item) => item.state === "ok")?.path ??
    "";
  const selected = evaluation.projects.find((item) => item.path === project);
  return {
    id: newId(),
    name: "New configuration",
    project,
    configuration: evaluation.context.configuration,
    targetFramework:
      selected && selected.tfms.includes(evaluation.context.targetFramework ?? "")
        ? evaluation.context.targetFramework ?? ""
        : selected?.tfms[0] ?? "",
    argumentsText: "",
    workingDirectory: projectDirectory(project),
    envRows: [],
    envFiles: [],
  };
}

function validEnvName(name: string) {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(name);
}

/**
 * The dialog validates the fields it owns; cwd and `.env` contents are validated by the host
 * on save so there is one parser and one path rule for dialog and execution.
 */
export function RunConfigDialog({
  evaluation,
  configs,
  defaultConfig,
  activeId,
  issues,
  initialId,
  onCommit,
  onRun,
  onClose,
}: {
  evaluation: Evaluation;
  configs: RunConfig[];
  defaultConfig: RunConfigView | null;
  activeId: string;
  issues: RunConfigIssue[];
  initialId: string;
  onCommit: (configs: RunConfig[], activeId: string) => Promise<RunConfigIssue[]>;
  onRun: (config: RunConfig) => void;
  onClose: () => void;
}) {
  const initial =
    (initialId !== DEFAULT_RUN_CONFIG_ID
      ? configs.find((config) => config.id === initialId)
      : null) ??
    defaultConfig ??
    (configs[0] ?? null);
  const [selectedId, setSelectedId] = useState(
    initial?.id ?? DEFAULT_RUN_CONFIG_ID,
  );
  const [draft, setDraft] = useState<Draft>(() =>
    initial ? toDraft(initial) : blankDraft(evaluation, null),
  );
  const [isNew, setIsNew] = useState(initial === null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [summary, setSummary] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const derived = !isNew && selectedId === DEFAULT_RUN_CONFIG_ID;
  const savedIssues = configIssues(issues, draft.id);

  function update<K extends keyof Draft>(key: K, value: Draft[K]) {
    setDraft((current) => ({ ...current, [key]: value }));
    setErrors((current) => {
      const next = { ...current };
      delete next[key as string];
      return next;
    });
  }

  function select(id: string) {
    if (id === DEFAULT_RUN_CONFIG_ID) {
      if (!defaultConfig) {
        setDraft(blankDraft(evaluation, null));
        setIsNew(true);
        setSelectedId(id);
        setSummary("The default configuration needs an evaluated console startup project.");
        return;
      }
      setDraft(toDraft(defaultConfig));
      setIsNew(false);
      setSelectedId(id);
    } else {
      const config = configs.find((item) => item.id === id);
      if (!config) return;
      setDraft(toDraft(config));
      setIsNew(false);
      setSelectedId(id);
    }
    setErrors({});
    setSummary(null);
  }

  function create() {
    const next = blankDraft(evaluation, defaultConfig);
    setDraft(next);
    setIsNew(true);
    setSelectedId(next.id);
    setErrors({});
    setSummary(null);
  }

  function duplicate() {
    const next: Draft = {
      ...draft,
      id: newId(),
      name: draft.name.trim() ? `${draft.name} copy` : "",
      envRows: draft.envRows.map((row) => ({ ...row })),
      envFiles: [...draft.envFiles],
    };
    setDraft(next);
    setIsNew(true);
    setSelectedId(next.id);
    setErrors({});
    setSummary(null);
  }

  function validate() {
    const next: Record<string, string> = {};
    const name = draft.name.trim();
    if (!name) {
      next.name = "Name is required.";
    } else if (
      configs.some(
        (config) =>
          config.id !== draft.id &&
          config.name.trim().toLowerCase() === name.toLowerCase(),
      )
    ) {
      next.name = `Another configuration is already named "${name}".`;
    }
    const project = evaluation.projects.find((item) => item.path === draft.project);
    if (!project) {
      next.project =
        "Choose a project from the current evaluation; nothing was saved.";
    }
    if (!draft.configuration.trim()) {
      next.configuration = "Choose a Configuration.";
    }
    if (
      draft.targetFramework &&
      project &&
      project.tfms.length > 0 &&
      !project.tfms.includes(draft.targetFramework)
    ) {
      next.targetFramework = `TFM ${draft.targetFramework} is not declared by ${project.name} (${project.tfms.join(", ")}). Choose a declared TFM.`;
    }
    const keys = new Set<string>();
    for (const row of draft.envRows) {
      const key = row.key.trim();
      if (!key) {
        if (row.value) next.env = "Every environment value needs a variable name.";
        continue;
      }
      if (!validEnvName(key)) {
        next.env = `"${key}" is not a valid environment variable name.`;
      } else if (keys.has(key)) {
        next.env = `The variable ${key} is listed twice.`;
      }
      keys.add(key);
    }
    if (draft.envFiles.some((file) => !file.trim())) {
      next.envFiles = "Remove the empty .env file row or fill its path.";
    }
    return next;
  }

  function buildConfig(): RunConfig {
    return {
      id: draft.id,
      name: draft.name.trim(),
      project: draft.project,
      configuration: draft.configuration,
      targetFramework: draft.targetFramework || null,
      args: draft.argumentsText
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean),
      workingDirectory: draft.workingDirectory.trim(),
      env: Object.fromEntries(
        draft.envRows
          .map((row) => [row.key.trim(), row.value] as const)
          .filter(([key]) => key),
      ),
      envFiles: draft.envFiles.map((file) => file.trim()).filter(Boolean),
    };
  }

  async function save(close = true, run = false) {
    const validation = validate();
    if (Object.keys(validation).length > 0) {
      setErrors(validation);
      setSummary("Fix the highlighted fields before saving; nothing was saved.");
      return;
    }
    const config = buildConfig();
    const nextConfigs = isNew
      ? [...configs, config]
      : configs.map((item) => (item.id === config.id ? config : item));
    setBusy(true);
    setSummary(null);
    try {
      const returned = await onCommit(nextConfigs, config.id);
      const mine = returned.filter((issue) => issue.configId === config.id);
      setIsNew(false);
      setSelectedId(config.id);
      if (mine.length > 0) {
        setErrors(
          Object.fromEntries(mine.map((issue) => [issue.field, issue.message])),
        );
        setSummary(
          "The configuration was saved but cannot run until these fields are fixed.",
        );
        return;
      }
      if (close) onClose();
      if (run) onRun(config);
    } catch (error) {
      const field = errorField(error);
      setErrors(field ? { [field]: messageOf(error) } : {});
      setSummary(messageOf(error));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (isNew) {
      const fallback = defaultConfig ?? configs[0] ?? null;
      if (fallback) select(fallback.id);
      else create();
      return;
    }
    if (derived) return;
    const nextConfigs = configs.filter((item) => item.id !== draft.id);
    const nextActive =
      activeId === draft.id
        ? defaultConfig
          ? DEFAULT_RUN_CONFIG_ID
          : nextConfigs[0]?.id ?? DEFAULT_RUN_CONFIG_ID
        : activeId;
    setBusy(true);
    try {
      await onCommit(nextConfigs, nextActive);
      const fallback = defaultConfig ?? nextConfigs[0] ?? null;
      if (fallback) select(fallback.id);
      else create();
    } catch (error) {
      setSummary(messageOf(error));
    } finally {
      setBusy(false);
    }
  }

  const list: RunConfigView[] = [
    ...(defaultConfig ? [defaultConfig] : []),
    ...configs,
  ];
  if (isNew && !list.some((item) => item.id === draft.id)) {
    list.push({ ...buildConfig(), readOnly: false });
  }
  const selectedProject = evaluation.projects.find(
    (item) => item.path === draft.project,
  );
  const tfms =
    selectedProject && selectedProject.tfms.length > 0
      ? selectedProject.tfms
      : evaluation.availableTfms;
  const tfmOptions =
    draft.targetFramework && !tfms.includes(draft.targetFramework)
      ? [draft.targetFramework, ...tfms]
      : tfms;
  const configurations =
    evaluation.configurations.length > 0
      ? evaluation.configurations
      : ["Debug", "Release"];
  const configurationOptions = configurations.includes(draft.configuration)
    ? configurations
    : [draft.configuration, ...configurations];
  const savedIssue = (field: string) =>
    savedIssues.find((issue) => issue.field === field)?.message ?? null;
  const fieldError = (field: string) => errors[field] ?? savedIssue(field);

  function field(
    name: keyof Draft,
    label: string,
    testId: string,
    control: ReactNode,
  ) {
    const message = fieldError(name);
    return (
      <label className="run-config-field">
        <span>{label}</span>
        {control}
        {message && (
          <span
            className="error"
            role="alert"
            data-testid={`run-config-error-${testId}`}
          >
            {message}
          </span>
        )}
      </label>
    );
  }

  return (
    <div className="modal-backdrop">
      <div
        className="modal run-config-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="run-config-title"
        data-testid="run-config-dialog"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !busy) onClose();
        }}
      >
        <h2 id="run-config-title">Run configurations</h2>
        <div className="run-config-layout">
          <nav className="run-config-list" aria-label="Run configurations">
            {list.map((config) => (
              <button
                key={config.id}
                type="button"
                data-testid={`run-config-item-${config.id}`}
                aria-current={config.id === selectedId ? "true" : undefined}
                onClick={() => select(config.id)}
                title={config.readOnly ? "Derived from the evaluation; read only." : config.project}
              >
                {config.name}
                {config.readOnly && <span className="run-config-tag">default</span>}
              </button>
            ))}
            <button type="button" className="run-config-add" data-testid="run-config-new" onClick={create}>
              <Icon name="plus" /> New configuration
            </button>
            <div className="run-config-list-actions">
              <button
                type="button"
                data-testid="run-config-duplicate"
                disabled={busy}
                onClick={duplicate}
              >
                Duplicate
              </button>
              <button
                type="button"
                className="danger"
                data-testid="run-config-remove"
                disabled={derived || busy || (!isNew && configs.length === 0)}
                onClick={() => void remove()}
              >
                Remove
              </button>
            </div>
          </nav>
          <form
            className="run-config-form"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            {derived && (
              <p className="hint" data-testid="run-config-default-note">
                This default configuration is derived from the current evaluation and is
                read only. Duplicate it to edit a named copy.{" "}
                <button
                  type="button"
                  data-testid="run-config-duplicate-to-edit"
                  disabled={busy}
                  onClick={duplicate}
                >
                  Duplicate to edit
                </button>
              </p>
            )}
            {field(
              "name",
              "Name",
              "name",
              <input
                type="text"
                data-testid="run-config-name"
                value={draft.name}
                disabled={derived || busy}
                onChange={(event) => update("name", event.target.value)}
              />,
            )}
            {field(
              "project",
              "Project",
              "project",
              <select
                data-testid="run-config-project"
                value={draft.project}
                disabled={derived || busy}
                onChange={(event) => update("project", event.target.value)}
              >
                {!selectedProject && <option value={draft.project}>{draft.project || "No project selected"}</option>}
                {evaluation.projects.map((project) => (
                  <option key={project.path} value={project.path}>
                    {project.name} ({project.kind})
                  </option>
                ))}
              </select>,
            )}
            {field(
              "configuration",
              "Configuration",
              "configuration",
              <select
                data-testid="run-config-configuration"
                value={draft.configuration}
                disabled={derived || busy}
                onChange={(event) => update("configuration", event.target.value)}
              >
                {configurationOptions.map((configuration) => (
                  <option key={configuration} value={configuration}>
                    {configuration}
                  </option>
                ))}
              </select>,
            )}
            {field(
              "targetFramework",
              "TFM",
              "tfm",
              <select
                data-testid="run-config-tfm"
                value={draft.targetFramework}
                disabled={derived || busy}
                onChange={(event) => update("targetFramework", event.target.value)}
              >
                <option value="">Project default</option>
                {tfmOptions.map((tfm) => (
                  <option key={tfm} value={tfm}>
                    {tfm}
                  </option>
                ))}
              </select>,
            )}
            <label className="run-config-field">
              <span>Exe path (from evaluation)</span>
              <input
                type="text"
                readOnly
                data-testid="run-config-exe"
                value={selectedProject?.targetPath ?? ""}
                placeholder="Evaluate the project to resolve the output assembly"
              />
            </label>
            {field(
              "argumentsText",
              "Arguments (one per line, passed as separate argv entries)",
              "args",
              <textarea
                data-testid="run-config-args"
                rows={2}
                value={draft.argumentsText}
                disabled={derived || busy}
                onChange={(event) => update("argumentsText", event.target.value)}
              />,
            )}
            {field(
              "workingDirectory",
              "Working directory (relative to the solution)",
              "cwd",
              <input
                type="text"
                data-testid="run-config-cwd"
                value={draft.workingDirectory}
                disabled={derived || busy}
                placeholder="Project directory"
                onChange={(event) => update("workingDirectory", event.target.value)}
              />,
            )}
            <div className="run-config-field">
              <span>Environment variables</span>
              {draft.envRows.map((row, index) => (
                <div className="run-config-row" key={index}>
                  <input
                    type="text"
                    aria-label={`Environment variable ${index + 1} name`}
                    data-testid={`run-config-env-key-${index}`}
                    value={row.key}
                    disabled={derived || busy}
                    placeholder="KEY"
                    onChange={(event) => {
                      const envRows = draft.envRows.map((item, itemIndex) =>
                        itemIndex === index
                          ? { ...item, key: event.target.value }
                          : item,
                      );
                      update("envRows", envRows);
                    }}
                  />
                  <input
                    type="text"
                    aria-label={`Environment variable ${index + 1} value`}
                    data-testid={`run-config-env-value-${index}`}
                    value={row.value}
                    disabled={derived || busy}
                    placeholder="VALUE"
                    onChange={(event) => {
                      const envRows = draft.envRows.map((item, itemIndex) =>
                        itemIndex === index
                          ? { ...item, value: event.target.value }
                          : item,
                      );
                      update("envRows", envRows);
                    }}
                  />
                  <button
                    type="button"
                    data-testid={`run-config-env-remove-${index}`}
                    disabled={derived || busy}
                    onClick={() =>
                      update(
                        "envRows",
                        draft.envRows.filter((_, itemIndex) => itemIndex !== index),
                      )
                    }
                  >
                    Remove
                  </button>
                </div>
              ))}
              <button
                type="button"
                data-testid="run-config-env-add"
                disabled={derived || busy}
                onClick={() =>
                  update("envRows", [...draft.envRows, { key: "", value: "" }])
                }
              >
                Add variable
              </button>
              {fieldError("env") && (
                <span className="error" role="alert" data-testid="run-config-error-env">
                  {fieldError("env")}
                </span>
              )}
              <span className="hint">
                Explicit variables win over the .env files. No expansion and no shell.
              </span>
            </div>
            <div className="run-config-field">
              <span>.env files (one path per line, relative to the solution)</span>
              {draft.envFiles.map((file, index) => (
                <div className="run-config-row" key={index}>
                  <input
                    type="text"
                    aria-label={`.env file ${index + 1}`}
                    data-testid={`run-config-env-file-${index}`}
                    value={file}
                    disabled={derived || busy}
                    placeholder=".env"
                    onChange={(event) => {
                      const envFiles = draft.envFiles.map((item, itemIndex) =>
                        itemIndex === index ? event.target.value : item,
                      );
                      update("envFiles", envFiles);
                    }}
                  />
                  <button
                    type="button"
                    data-testid={`run-config-env-file-remove-${index}`}
                    disabled={derived || busy}
                    onClick={() =>
                      update(
                        "envFiles",
                        draft.envFiles.filter((_, itemIndex) => itemIndex !== index),
                      )
                    }
                  >
                    Remove
                  </button>
                </div>
              ))}
              <button
                type="button"
                data-testid="run-config-env-file-add"
                disabled={derived || busy}
                onClick={() => update("envFiles", [...draft.envFiles, ""])}
              >
                Add .env file
              </button>
              {fieldError("envFiles") && (
                <span
                  className="error"
                  role="alert"
                  data-testid="run-config-error-envFiles"
                >
                  {fieldError("envFiles")}
                </span>
              )}
              <span className="hint">
                KEY=VALUE per line; # starts a comment. Invalid lines are reported here
                and the configuration does not run.
              </span>
            </div>
            {summary && (
              <p className="error" role="alert" data-testid="run-config-error">
                {summary}
              </p>
            )}
            {savedIssues.length > 0 && !summary && (
              <p className="error" role="alert" data-testid="run-config-issues">
                This configuration cannot run until its fields are fixed.
              </p>
            )}
            <div className="actions">
              <button
                type="submit"
                className="primary"
                data-testid="run-config-save"
                disabled={derived || busy}
              >
                {busy ? "Saving…" : "OK"}
              </button>
              <button type="button" data-testid="run-config-run" disabled={derived || busy} onClick={() => void save(true, true)}>Run</button>
              <button type="button" data-testid="run-config-apply" disabled={derived || busy} onClick={() => void save(false)}>Apply</button>
              <button
                type="button"
                data-testid="run-config-cancel"
                disabled={busy}
                onClick={onClose}
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
