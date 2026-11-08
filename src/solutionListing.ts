// One-level listing cache for the "Show all files" tree.
//
// A Git snapshot (commit/checkout) or a workspace change can turn a path into a link or back
// without changing the directory name, so a reload drops every cached level and the component asks
// again only for the levels the user has open. An answer started before the reload can never land
// in the new cache: the epoch it captured no longer matches. The cache is a mutable object so the
// component never reads a stale closure of its own state.
//
// A level that did not answer is not a level with no children: the read is repeated with the same
// limit as the tree-marker queries (#264) and, when every attempt fails, the level is left out of
// the cache so the next expansion asks again instead of showing an empty folder until a reload.

export type ListingEntry = {
  name: string;
  path: string;
  dir: boolean;
  hidden: boolean;
  symlink: boolean;
};

export type ListingLoader = (dir: string) => Promise<ListingEntry[]>;

/** Waits before each retry of a level that failed: the read plus two, as the #264 queries do. */
export const LISTING_RETRY_DELAYS_MS: readonly number[] = [200, 400];

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export function createListingCache(
  load: ListingLoader,
  retryDelaysMs: readonly number[] = LISTING_RETRY_DELAYS_MS,
) {
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
    // A request the reload replaced never repeats: its answer cannot land anywhere.
    async function read(): Promise<ListingEntry[]> {
      for (let attempt = 0; ; attempt += 1) {
        try {
          return await load(dir);
        } catch (error) {
          if (started !== epoch || attempt >= retryDelaysMs.length) throw error;
          await wait(retryDelaysMs[attempt]);
          if (started !== epoch) throw error;
        }
      }
    }
    const request = (async () => {
      try {
        const entries = await read();
        if (started !== epoch) return;
        dirs = { ...dirs, [dir]: entries };
      } catch {
        // Every attempt failed: nothing is cached for this level, so the next expansion asks again.
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
