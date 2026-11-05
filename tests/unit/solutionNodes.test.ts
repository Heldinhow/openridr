import { describe, expect, test } from "bun:test";
import { dirKey, nestSources, visibleSourceFiles } from "../../src/solutionNodes";

describe("visibleSourceFiles", () => {
  const nodes = nestSources("src/App/App.csproj", [
    "src/App/A.cs",
    "src/App/Sub/B.cs",
    "src/App/Sub/Deep/C.cs",
  ]);

  test("closed folders hide their files; top-level files stay visible", () => {
    expect(visibleSourceFiles(nodes, () => false)).toEqual(["src/App/A.cs"]);
  });

  test("only the open branch contributes", () => {
    expect(visibleSourceFiles(nodes, (key) => key === dirKey("src/App/Sub"))).toEqual([
      "src/App/Sub/B.cs",
      "src/App/A.cs",
    ]);
  });

  test("nested open folders contribute their files", () => {
    expect(visibleSourceFiles(nodes, () => true)).toEqual([
      "src/App/Sub/Deep/C.cs",
      "src/App/Sub/B.cs",
      "src/App/A.cs",
    ]);
  });
});
