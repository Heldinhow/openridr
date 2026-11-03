import { expect, test, type APIRequestContext } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Paths arrive from Finder or the terminal: the host resolves them, the UI only forwards them.
const repo = process.cwd();
const fixture = path.resolve(repo, "fixtures/OrderDesk");
const solution = fs.realpathSync(path.join(fixture, "OrderDesk.sln"));
const project = fs.realpathSync(path.join(fixture, "src/OrderDesk.Cli/OrderDesk.Cli.csproj"));
const temporary: string[] = [];

type Envelope = { ok: boolean; data?: any; error?: { code: string; message: string; details?: any } };

async function call(request: APIRequestContext, method: string, params: object = {}): Promise<Envelope> {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

function tempDir(): string {
  const dir = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "openridr-open-")));
  temporary.push(dir);
  return dir;
}

test.afterEach(async ({ request }) => {
  await call(request, "workspace.close");
  for (const dir of temporary.splice(0)) fs.rmSync(dir, { recursive: true, force: true });
});

test("a relative solution path resolved against baseDir opens the solution", async ({ request }) => {
  const opened = await call(request, "workspace.open", {
    path: "fixtures/OrderDesk/OrderDesk.sln",
    baseDir: repo,
  });
  expect(opened.ok, JSON.stringify(opened.error)).toBe(true);
  expect(opened.data.workspace.kind).toBe("solution");
  expect(opened.data.workspace.path).toBe(solution);
});

test("an uppercase extension opens and a .csproj stays a project", async ({ request }) => {
  const upper = tempDir();
  fs.copyFileSync(solution, path.join(upper, "OrderDesk.SLN"));
  const fromUpper = await call(request, "workspace.open", { path: path.join(upper, "OrderDesk.SLN") });
  expect(fromUpper.ok, JSON.stringify(fromUpper.error)).toBe(true);
  expect(fromUpper.data.workspace.kind).toBe("solution");

  const opened = await call(request, "workspace.open", { path: project });
  expect(opened.ok, JSON.stringify(opened.error)).toBe(true);
  expect(opened.data.workspace.kind).toBe("project");
  expect(opened.data.workspace.path).toBe(project);
});

test(".slnx opens as a solution and the outline lists its projects in file order", async ({ request }) => {
  const dir = tempDir();
  const csproj = `<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n`;
  fs.mkdirSync(path.join(dir, "App"));
  fs.mkdirSync(path.join(dir, "Lib"));
  fs.writeFileSync(path.join(dir, "App/App.csproj"), csproj);
  fs.writeFileSync(path.join(dir, "Lib/Lib.csproj"), csproj);
  const slnx = path.join(dir, "Modern.slnx");
  fs.writeFileSync(
    slnx,
    `<Solution>\n  <Folder Name="/src/">\n    <Project Path="App/App.csproj" />\n  </Folder>\n  <Project\n    Path="Lib/Lib.csproj" />\n</Solution>\n`,
  );

  const opened = await call(request, "workspace.open", { path: slnx });
  expect(opened.ok, JSON.stringify(opened.error)).toBe(true);
  expect(opened.data.workspace.kind).toBe("solution");

  const state = await call(request, "workspace.state");
  expect(state.data.outline.projects.map((item: { path: string }) => item.path)).toEqual([
    "App/App.csproj",
    "Lib/Lib.csproj",
  ]);
});

test("a directory with two solutions asks for a choice and lists the candidates", async ({ request }) => {
  const dir = tempDir();
  fs.copyFileSync(solution, path.join(dir, "First.sln"));
  fs.copyFileSync(solution, path.join(dir, "Second.sln"));

  const refused = await call(request, "workspace.open", { path: dir });
  expect(refused.ok).toBe(false);
  expect(refused.error?.code).toBe("ambiguous_workspace");
  const names = refused.error?.details.candidates.map((item: { name: string }) => item.name);
  expect(names).toEqual(["First.sln", "Second.sln"]);
});

test("refused paths keep the open workspace and never replace it", async ({ request }) => {
  const opened = await call(request, "workspace.open", { path: solution });
  expect(opened.ok, JSON.stringify(opened.error)).toBe(true);

  const notAllowed = await call(request, "workspace.open", { path: "package.json", baseDir: repo });
  expect(notAllowed.error?.code).toBe("path_not_allowed");
  const missing = await call(request, "workspace.open", { path: "Missing.sln", baseDir: repo });
  expect(missing.error?.code).toBe("not_found");

  const state = await call(request, "workspace.state");
  expect(state.data.workspace.path).toBe(solution);
});
