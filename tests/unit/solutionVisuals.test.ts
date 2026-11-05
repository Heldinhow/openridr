import { describe, expect, test } from "bun:test";
import {
  dedupePackages,
  fileKind,
  frameworkName,
  frameworkRows,
  packageKey,
  projectKindOf,
  symbolKindOf,
} from "../../src/solutionVisuals";

describe("fileKind", () => {
  test("splits the file types the tree renders today", () => {
    expect(fileKind("src/App/Program.cs")).toBe("csharp");
    expect(fileKind("OrderDesk.slnx")).toBe("solution");
    expect(fileKind("src/App/App.csproj")).toBe("project");
    expect(fileKind("Directory.Build.props")).toBe("props");
    expect(fileKind("global.json")).toBe("json");
    expect(fileKind("App.xml")).toBe("xml");
    expect(fileKind("Pages/Index.razor")).toBe("razor");
    expect(fileKind("Views/Home.cshtml")).toBe("razor");
    expect(fileKind("Resources/Strings.resx")).toBe("resource");
    expect(fileKind("README.md")).toBe("markdown");
    expect(fileKind(".editorconfig")).toBe("config");
    expect(fileKind(".gitignore")).toBe("config");
    expect(fileKind("scripts/build.sh")).toBe("script");
    expect(fileKind("build.ps1")).toBe("script");
    expect(fileKind("ci.yml")).toBe("yaml");
    expect(fileKind("notes.txt")).toBe("text");
    expect(fileKind("LICENSE")).toBe("file");
  });

  test("unknown extensions stay neutral and extension matching is case-insensitive", () => {
    expect(fileKind("tool.exe")).toBe("file");
    expect(fileKind("Program.CS")).toBe("csharp");
    expect(fileKind("Web.RAZOR")).toBe("razor");
  });
});

describe("projectKindOf", () => {
  test("web from the project text wins over an evaluated Exe", () => {
    expect(projectKindOf("console", "web")).toBe("web");
    expect(projectKindOf("library", "web")).toBe("web");
    expect(projectKindOf("web", null)).toBe("web");
  });

  test("test wins over web and console", () => {
    expect(projectKindOf("test", "web")).toBe("test");
  });

  test("evaluated console/library survive when the text agrees or is missing", () => {
    expect(projectKindOf("console", null)).toBe("console");
    expect(projectKindOf("library", "library")).toBe("library");
  });

  test("an unknown evaluated model falls back to the text outline", () => {
    expect(projectKindOf("unknown", "console")).toBe("console");
    expect(projectKindOf(null, null)).toBe("unknown");
    expect(projectKindOf("", "garbage")).toBe("unknown");
  });
});

describe("frameworks", () => {
  test("net TFMs become the shared framework label, never an invented patch version", () => {
    expect(frameworkName("net8.0", "console")).toBe("Microsoft.NETCore.App (net8.0)");
    expect(frameworkName("net8.0", "web")).toBe("Microsoft.AspNetCore.App (net8.0)");
    expect(frameworkName("netstandard2.0", "library")).toBe(".NET Standard (netstandard2.0)");
    expect(frameworkName("net48", "library")).toBe(".NET Framework (net48)");
  });

  test("empty or unknown TFMs do not create rows", () => {
    expect(frameworkRows([], "console")).toEqual([]);
    expect(frameworkRows(["", "net8.0"], "console")).toEqual(["Microsoft.NETCore.App (net8.0)"]);
  });
});

describe("symbolKindOf", () => {
  test("accepts only the five host kinds", () => {
    for (const kind of ["class", "interface", "enum", "record", "struct"]) {
      expect(symbolKindOf(kind)).toBe(kind);
    }
    expect(symbolKindOf("namespace")).toBeUndefined();
    expect(symbolKindOf(null)).toBeUndefined();
    expect(symbolKindOf(3)).toBeUndefined();
  });
});

describe("packageKey", () => {
  test("versions and TFMs never share a row key", () => {
    expect(packageKey({ name: "xunit", version: "2.9.2" })).toBe("xunit@2.9.2");
    expect(packageKey({ name: "xunit", version: "2.9.2" })).not.toBe(
      packageKey({ name: "xunit", version: "2.8.2" }),
    );
    expect(packageKey({ name: "xunit", version: "2.9.2" }, "net8.0")).toBe("xunit@2.9.2:net8.0");
    expect(packageKey({ name: "xunit", version: null })).toBe("xunit@unknown");
  });

  test("duplicate package versions are collapsed, different versions are kept", () => {
    const packages = [
      { name: "xunit", version: "2.9.2" },
      { name: "xunit", version: "2.9.2" },
      { name: "xunit", version: "2.8.2" },
      { name: "coverlet.collector", version: null },
    ];
    expect(dedupePackages(packages)).toEqual([
      { name: "xunit", version: "2.9.2" },
      { name: "xunit", version: "2.8.2" },
      { name: "coverlet.collector", version: null },
    ]);
  });
});
