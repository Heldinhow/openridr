// One-level listing cache for the "Show all files" tree.
//
// A Git snapshot (commit/checkout) or a workspace change can turn a path into a link or back
// without changing the directory name, so a reload drops every cached level and the component asks
// again only for the levels the user has open. An answer started before the reload can never land
// in the new cache: the epoch it captured no longer matches. The cache is a mutable object so the
// component never reads a stale closure of its own state.

export type ListingEntry = {
  name: string;
  path: string;
  dir: boolean;
  hidden: boolean;
  symlink: boolean;
};

export type ListingLoader = (dir: string) => Promise<ListingEntry[]>;

export function createListingCache(load: ListingLoader) {
  let epoch = 0;
  let dirs: Record<string, ListingEntry[]> = {};
  const pending = new Map<string, Promise<void>>();

  const snapshot = () => dirs;

  async function ensure(dir: string, force = false): Promise<void> {
    if (!force && dirs[dir]) return;
    const key = `${epoch}:${dir}`;
    const running = pending.get(key);
    if (running) return running;
    const started = epoch;
    const request = (async () => {
      try {
        const entries = await load(dir);
        if (started !== epoch) return;
        dirs = { ...dirs, [dir]: entries };
      } catch {
        if (started !== epoch) return;
        dirs = { ...dirs, [dir]: [] };
      } finally {
        pending.delete(key);
      }
    })();
    pending.set(key, request);
    return request;
  }

  /** Drop every cached level; the next `ensure` of a directory reads it again. */
  function reload(): void {
    epoch += 1;
    dirs = {};
  }

  return { ensure, reload, snapshot };
}
