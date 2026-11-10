import { describe, expect, test } from "bun:test";
import {
  dedupePackages,
  fileGlyph,
  fileKind,
  fileToken,
  frameworkName,
  frameworkRows,
  packageKey,
  projectMeta,
  projectKindOf,
  symbolKindOf,
} from "../../src/solutionVisuals";

describe("fileKind", () => {
  test("splits the file types the tree renders today", () => {
    expect(fileKind("src/App/Program.cs")).toBe("csharp");
    expect(fileKind("OrderDesk.slnx")).toBe("solution");
    expect(fileKind("OrderDesk.sln")).toBe("solution");
    expect(fileKind("src/App/App.csproj")).toBe("project");
    expect(fileKind("Directory.Build.props")).toBe("props");
    expect(fileKind("Directory.Build.targets")).toBe("props");
    expect(fileKind("global.json")).toBe("json");
    expect(fileKind("appsettings.json")).toBe("json");
    expect(fileKind("appsettings.Development.json")).toBe("json");
    expect(fileKind("App.xml")).toBe("xml");
    expect(fileKind("App.config")).toBe("xml");
    expect(fileKind("Pages/Index.razor")).toBe("razor");
    expect(fileKind("Views/Home.cshtml")).toBe("razor");
    expect(fileKind("Resources/Strings.resx")).toBe("resource");
    expect(fileKind("README.md")).toBe("markdown");
    expect(fileKind(".editorconfig")).toBe("config");
    expect(fileKind(".gitignore")).toBe("git");
    expect(fileKind(".gitattributes")).toBe("git");
    expect(fileKind("scripts/build.sh")).toBe("script");
    expect(fileKind("build.ps1")).toBe("powershell");
    expect(fileKind("run.bat")).toBe("batch");
    expect(fileKind("ci.yml")).toBe("yaml");
    expect(fileKind("ci.yaml")).toBe("yaml");
    expect(fileKind("notes.txt")).toBe("text");
    expect(fileKind("LICENSE")).toBe("license");
    expect(fileKind("logo.png")).toBe("image");
    expect(fileKind("site.css")).toBe("css");
    expect(fileKind("index.html")).toBe("html");
  });

  test("names that are not extensions win over the suffix", () => {
    expect(fileKind("Dockerfile")).toBe("docker");
    expect(fileKind("docker/Dockerfile.dev")).toBe("docker");
    expect(fileKind("docker-compose.yml")).toBe("docker");
    expect(fileKind("docker-compose.override.yaml")).toBe("docker");
    expect(fileKind("OrderDesk.sln.DotSettings.user")).toBe("dotsettings");
    expect(fileKind("OrderDesk.sln.DotSettings")).toBe("dotsettings");
    expect(fileKind("pipeline.yaml")).toBe("yaml");
    expect(fileKind("global.json")).toBe("json");
    expect(fileKind("license.md")).toBe("license");
  });

  test("the longest matching suffix wins", () => {
    // `docker-compose.override.yml` is Docker, even though it ends in `yml`; a plain `pipeline.yml`
    // is YAML. This is the VS Code rule the table is derived from.
    expect(fileKind("docker-compose.override.yml")).toBe("docker");
    expect(fileKind("deploy/pipeline.yml")).toBe("yaml");
    // A multi-segment suffix beats the last segment alone.
    expect(fileKind("OrderDesk.sln.DotSettings.user")).toBe("dotsettings");
    expect(fileKind("Settings.user")).toBe("file");
  });

  test("unknown extensions stay neutral and extension matching is case-insensitive", () => {
    expect(fileKind("tool.exe")).toBe("file");
    expect(fileKind("Program.CS")).toBe("csharp");
    expect(fileKind("Web.RAZOR")).toBe("razor");
    expect(fileKind("DOCKERFILE")).toBe("docker");
    expect(fileKind("LICENSE.TXT")).toBe("license");
  });
});

describe("fileGlyph", () => {
  test("maps every kind to the glyph the row draws", () => {
    const glyphOf = (path: string) => fileGlyph(fileKind(path));
    expect(glyphOf("Program.cs")).toBe("csharp");
    expect(glyphOf("OrderDesk.sln")).toBe("doc");
    expect(glyphOf("App.csproj")).toBe("project");
    expect(glyphOf("App.sln.DotSettings.user")).toBe("text");
    expect(glyphOf("LICENSE")).toBe("license");
    expect(glyphOf("global.json")).toBe("json");
    expect(glyphOf("README.md")).toBe("markdown");
    expect(glyphOf("Dockerfile")).toBe("docker");
    expect(glyphOf("docker-compose.yml")).toBe("docker");
    expect(glyphOf("ci.yml")).toBe("yaml");
    expect(glyphOf("notes.txt")).toBe("text");
    expect(glyphOf(".gitignore")).toBe("git");
    expect(glyphOf("App.xml")).toBe("xml");
    expect(glyphOf("Directory.Build.props")).toBe("xml");
    expect(glyphOf("Index.razor")).toBe("razor");
    expect(glyphOf("Strings.resx")).toBe("resource");
    expect(glyphOf(".editorconfig")).toBe("config");
    expect(glyphOf("build.sh")).toBe("script");
    expect(glyphOf("tool.exe")).toBe("file");
  });

  test("every kind names a glyph and a token, so a new type cannot render blank", () => {
    const kinds = [
      "Program.cs", "App.sln", "App.csproj", "App.sln.DotSettings", "Directory.Build.props",
      "global.json", "App.xml", "Index.razor", "Strings.resx", "README.md", "notes.txt",
      "ci.yml", "Dockerfile", ".gitignore", ".editorconfig", "build.sh", "build.ps1", "run.bat",
      "site.css", "index.html", "logo.png", "LICENSE", "tool.exe",
    ] as const;
    for (const path of kinds) {
      const kind = fileKind(path);
      expect(fileGlyph(kind)).not.toBeUndefined();
      expect(fileToken(kind)).not.toBeUndefined();
    }
  });

  test("structure and reference-less types are muted, types keep a Seti colour", () => {
    expect(fileToken(fileKind("Program.cs"))).toBe("seti-blue");
    expect(fileToken(fileKind("global.json"))).toBe("seti-yellow");
    expect(fileToken(fileKind("ci.yml"))).toBe("seti-purple");
    expect(fileToken(fileKind("build.sh"))).toBe("seti-green");
    expect(fileToken(fileKind("App.xml"))).toBe("seti-orange");
    expect(fileToken(fileKind("App.sln"))).toBe("muted");
    expect(fileToken(fileKind("tool.exe"))).toBe("muted");
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

describe("projectMeta", () => {
  test("the second layer carries only the parts that exist", () => {
    expect(projectMeta("console", ["net8.0"], null)).toBe("console · net8.0");
    expect(projectMeta("library", ["net8.0", "netstandard2.0"], "restore incomplete")).toBe(
      "library · net8.0, netstandard2.0 · restore incomplete",
    );
    // An unknown kind is not invented; the TFM and the state still show.
    expect(projectMeta("unknown", ["net8.0"], "partial context")).toBe("net8.0 · partial context");
    expect(projectMeta("unknown", [], null)).toBe("");
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
