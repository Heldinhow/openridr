import { describe, expect, test } from "bun:test";
import { askTreeQuery, TREE_QUERY_RETRY_DELAYS_MS } from "../../src/treeQueryRetry";

// The waits are the policy under test in production; the tests that assert the retry itself do not
// spend them.
const NO_WAIT: readonly number[] = [0, 0];

describe("askTreeQuery", () => {
  test("an answer that arrives is returned without a retry", async () => {
    let calls = 0;
    const answer = await askTreeQuery(async () => {
      calls += 1;
      return "ok";
    }, () => false, NO_WAIT);
    expect(answer).toBe("ok");
    expect(calls).toBe(1);
  });

  test("a failed answer is asked again, and a later success is returned", async () => {
    let calls = 0;
    const answer = await askTreeQuery(async () => {
      calls += 1;
      if (calls < 3) throw new Error("lost");
      return "ok";
    }, () => false, NO_WAIT);
    expect(answer).toBe("ok");
    expect(calls).toBe(3);
  });

  test("a batch that never answers is given up after three attempts", async () => {
    let calls = 0;
    const answer = await askTreeQuery(async () => {
      calls += 1;
      throw new Error("lost");
    }, () => false, NO_WAIT);
    expect(answer).toBeNull();
    expect(calls).toBe(3);
  });

  test("a request that was replaced is not repeated", async () => {
    let calls = 0;
    let superseded = false;
    const answer = await askTreeQuery(async () => {
      calls += 1;
      superseded = true;
      throw new Error("lost");
    }, () => superseded, NO_WAIT);
    expect(answer).toBeNull();
    expect(calls).toBe(1);
  });

  test("a request replaced while the retry waits is not repeated", async () => {
    let calls = 0;
    let superseded = false;
    const answer = await askTreeQuery(async () => {
      calls += 1;
      // The identity changes while the helper is waiting to ask again.
      setTimeout(() => {
        superseded = true;
      }, 5);
      throw new Error("lost");
    }, () => superseded, [50, 50]);
    expect(answer).toBeNull();
    expect(calls).toBe(1);
  });

  test("the retry policy is three attempts with 200 ms and 400 ms waits", () => {
    expect(TREE_QUERY_RETRY_DELAYS_MS).toEqual([200, 400]);
  });
});
