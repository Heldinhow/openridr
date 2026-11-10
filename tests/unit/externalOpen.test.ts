import { describe, expect, test } from "bun:test";
import { workspaceFileTarget } from "../../src/externalOpen";

const workspace = "/tmp/work/OrderDesk";

describe("workspaceFileTarget", () => {
  test("a code file inside the workspace becomes a tab path", () => {
    expect(workspaceFileTarget("/tmp/work/OrderDesk/src/OrderDesk.Cli/Program.cs", workspace)).toBe(
      "src/OrderDesk.Cli/Program.cs",
    );
    expect(workspaceFileTarget("/tmp/work/OrderDesk/README.md", workspace)).toBe("README.md");
    expect(workspaceFileTarget("/tmp/work/OrderDesk/global.json", workspace)).toBe("global.json");
  });

  test("the workspace file itself is not a tab", () => {
    expect(workspaceFileTarget("/tmp/work/OrderDesk/OrderDesk.sln", workspace)).toBe(null);
    expect(workspaceFileTarget("/tmp/work/OrderDesk/src/OrderDesk.Cli/OrderDesk.Cli.csproj", workspace)).toBe(null);
  });

  test("a binary or an unknown extension keeps the workspace error", () => {
    expect(workspaceFileTarget("/tmp/work/OrderDesk/bin/App.dll", workspace)).toBe(null);
    expect(workspaceFileTarget("/tmp/work/OrderDesk/logo.png", workspace)).toBe(null);
    expect(workspaceFileTarget("/tmp/work/OrderDesk/Makefile", workspace)).toBe(null);
  });

  test("a path outside the workspace is never made relative", () => {
    expect(workspaceFileTarget("/tmp/other/Proj/Program.cs", workspace)).toBe(null);
    // The shared prefix alone is not containment.
    expect(workspaceFileTarget("/tmp/work/OrderDesk-old/Program.cs", workspace)).toBe(null);
    expect(workspaceFileTarget("/tmp/work/OrderDesk/../Escape/Program.cs", workspace)).toBe(null);
  });

  test("the workspace root itself is not a file", () => {
    expect(workspaceFileTarget(workspace, workspace)).toBe(null);
    expect(workspaceFileTarget(`${workspace}/src`, workspace)).toBe(null);
  });
});
