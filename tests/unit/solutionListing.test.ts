import { describe, expect, test } from "bun:test";
import { createListingCache, type ListingEntry } from "../../src/solutionListing";

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

  test("a failed level caches empty for that epoch and is asked again after a reload", async () => {
    let calls = 0;
    const cache = createListingCache(async () => {
      calls += 1;
      if (calls === 1) throw new Error("unreadable");
      return [fileEntry("App.cs")];
    });
    await cache.ensure("");
    expect(cache.snapshot()[""]).toEqual([]);
    cache.reload();
    await cache.ensure("", true);
    expect(cache.snapshot()[""][0].path).toBe("App.cs");
  });
});
