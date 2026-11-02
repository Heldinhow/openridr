import { expect, test } from "@playwright/test";
import { workspaceRelative } from "../../src/solutionNodes";

// Path normalization for tabs: pure functions, no host and no browser (see execution-gates.spec.ts).
const workspace = "/tmp/Order Desk";

test("an absolute path inside the workspace becomes relative", () => {
  expect(workspaceRelative("/tmp/Order Desk/src/OrderDesk.Cli/Program.cs", workspace)).toBe(
    "src/OrderDesk.Cli/Program.cs",
  );
});

test("backslashes are normalized before the workspace prefix is removed", () => {
  expect(workspaceRelative("C:\\work\\OrderDesk\\src\\Program.cs", "C:\\work\\OrderDesk")).toBe(
    "src/Program.cs",
  );
});

test("an already relative path is unchanged", () => {
  expect(workspaceRelative("src/OrderDesk.Cli/Program.cs", workspace)).toBe(
    "src/OrderDesk.Cli/Program.cs",
  );
});

test("an absolute path outside the workspace is unchanged", () => {
  expect(workspaceRelative("/tmp/Other/src/Program.cs", workspace)).toBe("/tmp/Other/src/Program.cs");
  // The prefix must end at a separator: a sibling folder with a shared name is not inside.
  expect(workspaceRelative("/tmp/Order Desk2/Program.cs", workspace)).toBe(
    "/tmp/Order Desk2/Program.cs",
  );
});

test("the workspace root itself becomes the empty relative path", () => {
  expect(workspaceRelative(workspace, workspace)).toBe("");
});

test("a workspace with no directory keeps absolute paths relative to the filesystem root", () => {
  // A solution opened as /OrderDesk.sln has dirOf "" and the adapter still sends /Program.cs.
  expect(workspaceRelative("/Program.cs", "")).toBe("Program.cs");
  expect(workspaceRelative("/src/OrderDesk.Cli/Program.cs", "")).toBe(
    "src/OrderDesk.Cli/Program.cs",
  );
  // Already-relative paths do not gain or lose a separator.
  expect(workspaceRelative("src/OrderDesk.Cli/Program.cs", "")).toBe(
    "src/OrderDesk.Cli/Program.cs",
  );
  expect(workspaceRelative("/", "/")).toBe("");
  expect(workspaceRelative("/Program.cs", "/")).toBe("Program.cs");
});
