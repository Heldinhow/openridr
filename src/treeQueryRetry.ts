// A tree-marker query that fails is asked again before its batch is given up: an answer that never
// arrived must not look like "this path is not a link/ignored". A permanent failure still ends
// without a marker (never with a wrong one) and never drops the batches that follow.

/** Waits before each retry: the original call plus two, as the listing cache does (#282). */
export const TREE_QUERY_RETRY_DELAYS_MS: readonly number[] = [200, 400];

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/**
 * Ask one batch, retrying a failed answer. `null` means the batch was given up — the caller keeps
 * what the previous batches proved and moves on to the next one. A request that was replaced while
 * the call was in the air is not repeated: its answer could not land anywhere.
 */
export async function askTreeQuery<T>(
  ask: () => Promise<T>,
  superseded: () => boolean,
  delays: readonly number[] = TREE_QUERY_RETRY_DELAYS_MS,
): Promise<T | null> {
  for (let attempt = 0; ; attempt += 1) {
    try {
      return await ask();
    } catch {
      if (attempt >= delays.length || superseded()) return null;
      await wait(delays[attempt]);
      if (superseded()) return null;
    }
  }
}
