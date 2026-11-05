import { describe, expect, test } from "bun:test";
import { testPanelFramework } from "../../src/testScope";

describe("testPanelFramework", () => {
  test("the current evaluation wins over an earlier discovery", () => {
    // Discovery ran on net8.0; the model was re-evaluated on net9.0. The panel operates on the
    // current model, so the selector cannot keep showing the old scope.
    expect(testPanelFramework({ contextFramework: "net9.0", projectTfms: ["net8.0", "net9.0"] })).toBe(
      "net9.0",
    );
  });

  test("a single-TFM project names its own framework when nothing is selected", () => {
    expect(testPanelFramework({ contextFramework: null, projectTfms: ["net8.0"] })).toBe("net8.0");
  });

  test("a project that declares no TFM never borrows another project's", () => {
    // The solution may declare exactly one TFM in total; that is not evidence about this project.
    expect(testPanelFramework({ contextFramework: null, projectTfms: [] })).toBe(null);
    expect(testPanelFramework({ contextFramework: null, projectTfms: [" ", ""] })).toBe(null);
  });

  test("a multi-TFM project keeps an honest empty selection", () => {
    expect(testPanelFramework({ contextFramework: null, projectTfms: ["net8.0", "netstandard2.0"] })).toBe(
      null,
    );
  });
});
