import { describe, expect, test } from "bun:test";
import { effectiveTfm } from "../../src/tfm";

describe("effectiveTfm", () => {
  test("the explicit selection wins over the declared TFMs", () => {
    expect(effectiveTfm("net9.0", ["net8.0", "net9.0"])).toBe("net9.0");
    expect(effectiveTfm("netstandard2.0", ["net8.0"])).toBe("netstandard2.0");
  });

  test("a single declared TFM is named instead of the empty placeholder", () => {
    expect(effectiveTfm(null, ["net8.0"])).toBe("net8.0");
  });

  test("a multi-target project keeps the placeholder until the user picks one", () => {
    expect(effectiveTfm(null, ["net8.0", "net9.0"])).toBe(null);
  });

  test("no declared TFM and blank entries stay empty", () => {
    expect(effectiveTfm(null, [])).toBe(null);
    expect(effectiveTfm(null, [" ", ""])).toBe(null);
    expect(effectiveTfm(null, ["net8.0", " "])).toBe("net8.0");
  });
});
