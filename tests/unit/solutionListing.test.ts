import { describe, expect, test } from "bun:test";
import {
  createListingCache,
  LISTING_RETRY_DELAYS_MS,
  type ListingEntry,
} from "../../src/solutionListing";

// The waits are the policy under test in production; the tests that assert the retry itself do not
// spend them.
const NO_WAIT: readonly number[] = [0, 0];

const linkEntry = (path: string): ListingEntry => ({
  name: path,
  path,
  dir: false,
  hidden: false,
  symlink: true,
});
const fileEntry = (path: string): ListingEntry => ({
  name: path,
  path,
  dir: false,
  hidden: false,
  symlink: false,
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("createListingCache", () => {
  test("a reload asks the open level again and the new type replaces the old", async () => {
    const responses = [[linkEntry("Linked.cs")], [fileEntry("Linked.cs")]];
    let calls = 0;
    const cache = createListingCache(async () => responses[calls++]);

    await cache.ensure("");
    expect(cache.snapshot()[""][0].symlink).toBe(true);
    await cache.ensure(""); // cached: no second read
    expect(calls).toBe(1);

    cache.reload();
    expect(cache.snapshot()).toEqual({});
    await cache.ensure("", true);
    expect(calls).toBe(2);
    expect(cache.snapshot()[""][0].symlink).toBe(false);
  });

  test("an answer started before the reload never lands in the new cache", async () => {
    const calls: Array<ReturnType<typeof deferred<ListingEntry[]>>> = [];
    const cache = createListingCache(() => {
      const pending = deferred<ListingEntry[]>();
      calls.push(pending);
      return pending.promise;
    });

    const oldRun = cache.ensure("");
    expect(calls.length).toBe(1);
    cache.reload();
    calls[0].resolve([linkEntry("Linked.cs")]);
    await oldRun;
    expect(cache.snapshot()[""]).toBeUndefined();

    const newRun = cache.ensure("", true);
    expect(calls.length).toBe(2);
    calls[1].resolve([fileEntry("Linked.cs")]);
    await newRun;
    expect(cache.snapshot()[""][0].symlink).toBe(false);
  });

  test("concurrent reads of the same level share one request", async () => {
    const pending = deferred<ListingEntry[]>();
    let calls = 0;
    const cache = createListingCache(() => {
      calls += 1;
      return pending.promise;
    });
    const first = cache.ensure("src");
    const second = cache.ensure("src");
    expect(calls).toBe(1);
    pending.resolve([fileEntry("src/App.cs")]);
    await Promise.all([first, second]);
    expect(cache.snapshot()["src"]).toHaveLength(1);
  });

  test("a level that fails once is read again and its children land", async () => {
    let calls = 0;
    const cache = createListingCache(async () => {
      calls += 1;
      if (calls === 1) throw new Error("unreadable");
      return [fileEntry("App.cs")];
    }, NO_WAIT);
    await cache.ensure("");
    expect(calls).toBe(2);
    expect(cache.snapshot()[""][0].path).toBe("App.cs");
  });

  test("a level that never answers is asked three times and is not cached as an empty folder", async () => {
    let calls = 0;
    const cache = createListingCache(async () => {
      calls += 1;
      throw new Error("unreadable");
    }, NO_WAIT);
    await cache.ensure("");
    expect(calls).toBe(3); // the read plus the two retries
    expect(cache.snapshot()[""]).toBeUndefined(); // no empty level to hit the cache guard
    await cache.ensure("");
    expect(calls).toBe(6); // the next expansion asks again instead of showing nothing until a reload
  });

  test("a level that failed for good does not stop the next level from loading", async () => {
    const cache = createListingCache(async (dir) => {
      if (dir === "bin") throw new Error("unreadable");
      return [fileEntry(`${dir}/App.cs`)];
    }, NO_WAIT);
    await Promise.all([cache.ensure("bin"), cache.ensure("src")]);
    expect(cache.snapshot()["bin"]).toBeUndefined();
    expect(cache.snapshot()["src"][0].path).toBe("src/App.cs");
  });

  test("a reload while the level is failing stops the repetition", async () => {
    let calls = 0;
    const cache = createListingCache(async () => {
      calls += 1;
      throw new Error("unreadable");
    }, NO_WAIT);
    const run = cache.ensure("");
    cache.reload(); // the request was replaced before its first answer arrived
    await run;
    expect(calls).toBe(1);
    expect(cache.snapshot()[""]).toBeUndefined();
  });

  test("the retry policy is the one the tree-marker queries use (#264): three attempts, 200/400 ms", () => {
    expect(LISTING_RETRY_DELAYS_MS).toEqual([200, 400]);
  });
});
