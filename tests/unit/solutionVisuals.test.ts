import { describe, expect, test } from "bun:test";
import {
  dedupePackages,
  fileIcon,
  fileKind,
  folderIcon,
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

describe("fileIcon", () => {
  test("the theme's answer for every case the tree renders", () => {
    const cases: [string, string][] = [
      // C# and the solution structure
      ["Program.cs", "csharp"],
      ["Script.csx", "csharp"],
      ["OrderDesk.sln", "visualstudio"],
      ["OrderDesk.slnx", "visualstudio"],
      ["App.csproj", "visualstudio"],
      ["App.fsproj", "fsharp"],
      ["App.vbproj", "xml"],
      ["Directory.Build.props", "settings"],
      ["Directory.Build.targets", "xml"],
      ["Pages/Index.razor", "razor"],
      ["Views/Home.cshtml", "razor"],
      // JSON and the names the theme singles out
      ["appsettings.json", "json"],
      ["appsettings.Development.json", "json"],
      ["global.json", "json"],
      ["launchSettings.json", "json"],
      ["data.jsonc", "json"],
      ["package.json", "nodejs"],
      ["tsconfig.json", "tsconfig"],
      // XML, config, resources
      ["App.xml", "xml"],
      ["App.config", "settings"],
      ["nuget.config", "nuget"],
      ["Strings.resx", "xml"],
      // Documents
      ["notes.md", "markdown"],
      ["notes.markdown", "markdown"],
      ["README.md", "readme"],
      ["page.mdx", "mdx"],
      ["notes.txt", "document"],
      ["LICENSE", "license"],
      ["LICENSE.md", "license"],
      ["LICENSE.txt", "license"],
      ["COPYING", "license"],
      // Data, CI and dotfiles
      ["ci.yml", "yaml"],
      ["ci.yaml", "yaml"],
      [".editorconfig", "editorconfig"],
      [".gitignore", "git"],
      [".gitattributes", "git"],
      [".gitmodules", "git"],
      [".env", "tune"],
      // Containers and scripts
      ["Dockerfile", "docker"],
      ["docker/Dockerfile.dev", "docker"],
      ["docker-compose.yml", "docker"],
      ["docker-compose.override.yaml", "docker"],
      [".dockerignore", "docker"],
      ["scripts/build.sh", "console"],
      ["run.bash", "console"],
      ["run.zsh", "console"],
      ["run.bat", "console"],
      ["build.cmd", "console"],
      ["build.ps1", "powershell"],
      ["build.psm1", "powershell"],
      // Web and assets
      ["site.css", "css"],
      ["style.scss", "sass"],
      ["less.less", "less"],
      ["index.html", "html"],
      ["index.htm", "html"],
      ["App.xaml", "xaml"],
      ["logo.svg", "svg"],
      ["logo.png", "image"],
      ["photo.jpg", "image"],
      ["photo.jpeg", "image"],
      ["anim.gif", "image"],
      ["pic.webp", "image"],
      ["pic.bmp", "image"],
      ["icon.ico", "image"],
      // ReSharper settings: the theme only knows the solution-scoped names
      ["OrderDesk.sln.DotSettings", "settings"],
      ["OrderDesk.sln.DotSettings.user", "settings"],
      ["App.DotSettings", "file"],
      ["App.DotSettings.user", "file"],
      // Not in the theme: the generic drawing
      ["Strings.resources", "file"],
      ["tool.exe", "file"],
    ];
    for (const [path, icon] of cases) expect(`${path}: ${fileIcon(path)}`).toBe(`${path}: ${icon}`);
  });

  test("the whole name beats the extension, and the longest extension beats the shorter one", () => {
    // `.config` alone is settings, `.json` alone is json, `.md` alone is markdown.
    expect(fileIcon("nuget.config")).toBe("nuget");
    expect(fileIcon("package.json")).toBe("nodejs");
    expect(fileIcon("README.md")).toBe("readme");
    // `.dotsettings` alone is not in the theme; `sln.dotsettings` is.
    expect(fileIcon("OrderDesk.sln.DotSettings")).toBe("settings");
    expect(fileIcon("App.DotSettings")).toBe("file");
  });

  test("the kind and the drawing are separate: one kind can draw two icons", () => {
    expect(fileKind("global.json")).toBe("json");
    expect(fileKind("package.json")).toBe("json");
    expect(fileIcon("global.json")).toBe("json");
    expect(fileIcon("package.json")).toBe("nodejs");
    // `.sln` and `.csproj` are two kinds and one drawing, exactly like the theme.
    expect(fileKind("OrderDesk.sln")).not.toBe(fileKind("App.csproj"));
    expect(fileIcon("OrderDesk.sln")).toBe(fileIcon("App.csproj"));
  });
});

describe("folderIcon", () => {
  test("the theme's answer for every folder name the tree meets", () => {
    const cases: [string, string][] = [
      ["src", "folder-src"],
      ["Source", "folder-src"],
      ["test", "folder-test"],
      ["tests", "folder-test"],
      ["docs", "folder-docs"],
      ["Properties", "folder-config"],
      ["wwwroot", "folder-public"],
      ["bin", "folder-dist"],
      [".github", "folder-github"],
      [".vscode", "folder-vscode"],
      ["scripts", "folder-scripts"],
      ["assets", "folder-resource"],
      ["Controllers", "folder-controller"],
      ["Models", "folder-class"],
      ["Views", "folder-views"],
      ["Services", "folder-controller"],
      ["Migrations", "folder-migrations"],
    ];
    for (const [name, icon] of cases) expect(`${name}: ${folderIcon(name)}`).toBe(`${name}: ${icon}`);
  });

  test("a name the theme does not map gets the generic folder", () => {
    // `obj` is the one folder a .NET solution has that the theme has no drawing for.
    expect(folderIcon("obj")).toBe("folder");
    expect(folderIcon("OrderDesk.Core")).toBe("folder");
    expect(folderIcon("")).toBe("folder");
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
