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

export type Snapshot = {
  app: { home: string; platform: string; arch: string; version: string };
  workspace: null | {
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
  trust: { granted: boolean };
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
