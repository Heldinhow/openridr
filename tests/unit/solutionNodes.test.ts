import { describe, expect, test } from "bun:test";
import {
  dirKey,
  isAbsolutePath,
  nestSources,
  visibleSourceFiles,
  workspaceRelative,
} from "../../src/solutionNodes";

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

describe("paths of a project outside the solution directory", () => {
  test("an absolute model path is recognised and a relative one is not", () => {
    expect(isAbsolutePath("/work/lib/Lib.csproj")).toBe(true);
    expect(isAbsolutePath("C:\\work\\lib\\Lib.csproj")).toBe(true);
    expect(isAbsolutePath("src/App/App.csproj")).toBe(false);
    expect(isAbsolutePath("../lib/Lib.csproj")).toBe(false);
  });

  test("a tab keeps the absolute path when the file is outside the workspace root", () => {
    expect(workspaceRelative("/work/app/src/App.cs", "/work/app")).toBe("src/App.cs");
    expect(workspaceRelative("/work/lib/Greeting.cs", "/work/app")).toBe("/work/lib/Greeting.cs");
    expect(workspaceRelative("/work/app/src/App.cs", "/work/app/")).toBe("src/App.cs");
  });

  test("sources of an outside project nest under its own directory", () => {
    const nodes = nestSources("/work/lib/Lib.csproj", [
      "/work/lib/Greeting.cs",
      "/work/lib/Models/Order.cs",
    ]);
    expect(nodes.map((node) => node.kind === "dir" ? node.name : node.name)).toEqual([
      "Models",
      "Greeting.cs",
    ]);
  });
});
