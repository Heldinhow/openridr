// One serial Git read for the workspace header: a request made while another is in flight queues
// exactly one rerun, and a response for a workspace/trust that is no longer current is dropped.
// The loop reads the context again after every await, so a queued rerun always runs under the
// identity that is current then — never the closure that started it.

export type GitRefreshContext = {
  workspaceId: string | number;
  trusted: boolean;
};

export type GitRefresherOptions<T> = {
  /** The workspace/trust the UI is showing right now. */
  context: () => GitRefreshContext;
  fetch: () => Promise<T>;
  apply: (value: T, context: GitRefreshContext) => void;
  /** Trust was revoked: there is nothing to show and no Git call to make. */
  clear: () => void;
  fail: (error: unknown, context: GitRefreshContext) => void;
};

const sameContext = (a: GitRefreshContext, b: GitRefreshContext) =>
  a.workspaceId === b.workspaceId && a.trusted === b.trusted;

export function createGitRefresher<T>(options: GitRefresherOptions<T>) {
  let inFlight = false;
  let queued = false;

  async function run(): Promise<void> {
    if (inFlight) {
      queued = true;
      return;
    }
    inFlight = true;
    try {
      do {
        queued = false;
        const context = options.context();
        if (!context.trusted) {
          options.clear();
          continue;
        }
        try {
          const value = await options.fetch();
          if (!sameContext(options.context(), context)) {
            // The answer belongs to a workspace/trust that is gone; rerun for the current one.
            queued = true;
            continue;
          }
          options.apply(value, context);
        } catch (error) {
          if (!sameContext(options.context(), context)) {
            queued = true;
            continue;
          }
          options.fail(error, context);
        }
      } while (queued);
    } finally {
      inFlight = false;
    }
  }

  return { run };
}
