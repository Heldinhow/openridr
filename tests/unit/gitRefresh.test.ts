import { describe, expect, test } from "bun:test";
import { createGitRefresher, type GitRefreshContext } from "../../src/gitRefresh";

/** Lets the refresher's await chain run: a settled fetch is followed by the next call. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

type Call = { context: GitRefreshContext; pending: ReturnType<typeof deferred<string>> };

/** A harness with a controlled fetch: each call is a promise the test resolves by hand. */
function harness(initial: GitRefreshContext) {
  let context = initial;
  const calls: Call[] = [];
  const applied: Array<{ value: string; workspaceId: string | number }> = [];
  const failures: unknown[] = [];
  let cleared = 0;
  let concurrent = 0;
  let maxConcurrent = 0;
  const refresher = createGitRefresher<string>({
    context: () => context,
    fetch: () => {
      concurrent += 1;
      maxConcurrent = Math.max(maxConcurrent, concurrent);
      const pending = deferred<string>();
      calls.push({ context, pending });
      return pending.promise.finally(() => {
        concurrent -= 1;
      });
    },
    apply: (value, appliedContext) =>
      applied.push({ value, workspaceId: appliedContext.workspaceId }),
    clear: () => {
      cleared += 1;
    },
    fail: (error) => failures.push(error),
  });
  return {
    refresher,
    calls,
    applied,
    failures,
    setContext: (next: GitRefreshContext) => {
      context = next;
    },
    get cleared() {
      return cleared;
    },
    get maxConcurrent() {
      return maxConcurrent;
    },
  };
}

const A: GitRefreshContext = { workspaceId: "A", trusted: true };
const B: GitRefreshContext = { workspaceId: "B", trusted: true };

describe("createGitRefresher", () => {
  test("a request made while one is in flight runs again under the current workspace", async () => {
    const test1 = harness(A);
    void test1.refresher.run();
    expect(test1.calls.length).toBe(1);
    expect(test1.calls[0].context.workspaceId).toBe("A");

    test1.setContext(B);
    void test1.refresher.run(); // queued: the answer for A is already stale
    test1.calls[0].pending.resolve("from A");
    await flush();

    expect(test1.calls.length).toBe(2);
    expect(test1.calls[1].context.workspaceId).toBe("B");
    expect(test1.applied).toEqual([]);

    test1.calls[1].pending.resolve("from B");
    await flush();
    expect(test1.applied).toEqual([{ value: "from B", workspaceId: "B" }]);
    expect(test1.maxConcurrent).toBe(1);
  });

  test("revoking trust clears without applying, and re-authorizing fetches again", async () => {
    const test2 = harness(A);
    void test2.refresher.run();
    test2.setContext({ workspaceId: "A", trusted: false });
    void test2.refresher.run();
    test2.calls[0].pending.resolve("from A");
    await flush();

    expect(test2.applied).toEqual([]);
    expect(test2.cleared).toBe(1);
    expect(test2.calls.length).toBe(1);

    test2.setContext(A);
    void test2.refresher.run();
    expect(test2.calls.length).toBe(2);
    test2.calls[1].pending.resolve("after trust");
    await flush();
    expect(test2.applied).toEqual([{ value: "after trust", workspaceId: "A" }]);
  });

  test("a stale failure is dropped and the current workspace still loads", async () => {
    const test3 = harness(A);
    void test3.refresher.run();
    test3.setContext(B);
    void test3.refresher.run();
    test3.calls[0].pending.reject(new Error("A is gone"));
    await flush();

    expect(test3.failures).toEqual([]);
    expect(test3.calls.length).toBe(2);
    test3.calls[1].pending.resolve("from B");
    await flush();
    expect(test3.applied).toEqual([{ value: "from B", workspaceId: "B" }]);
  });

  test("one refresh at a time: a burst of calls coalesces into one rerun", async () => {
    const test4 = harness(A);
    void test4.refresher.run();
    void test4.refresher.run();
    void test4.refresher.run();
    void test4.refresher.run();
    test4.calls[0].pending.resolve("first");
    await flush();

    expect(test4.calls.length).toBe(2);
    expect(test4.maxConcurrent).toBe(1);
    expect(test4.applied).toEqual([{ value: "first", workspaceId: "A" }]);

    test4.calls[1].pending.resolve("second");
    await flush();
    expect(test4.applied).toEqual([
      { value: "first", workspaceId: "A" },
      { value: "second", workspaceId: "A" },
    ]);
  });

  test("a current failure is reported for the context it belongs to", async () => {
    const test5 = harness(A);
    const running = test5.refresher.run();
    const error = new Error("git failed");
    test5.calls[0].pending.reject(error);
    await running;
    expect(test5.failures).toEqual([error]);
    expect(test5.applied).toEqual([]);
  });
});
