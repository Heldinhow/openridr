import { start, shot, txt } from "./lib.mjs";
const { b, p } = await start();
await p.goto("http://127.0.0.1:1420");
await p.getByTestId("workspace-path-input").fill("/tmp/OrderDesk/OrderDesk.sln");
await p.getByTestId("workspace-open").click();
await p.getByTestId("trust-checkbox").check();
await p.getByTestId("trust-grant").click();
await p.waitForTimeout(3000);
await p.getByRole("button",{name:"Open workspace"}).click();
await p.waitForTimeout(8000);
await shot(p, "o04-workspace");
console.log("A:", await txt(p, 1500));
// toolbar state
for (const id of ["toolbar-run","toolbar-debug","toolbar-build"]) {
  const l = p.getByTestId(id);
  if (await l.count()) console.log(id, await l.isDisabled(), await l.getAttribute("title"));
}
await p.getByTestId("toolbar-run").click().catch(e=>console.log("runclick",String(e).slice(0,150)));
await p.waitForTimeout(15000);
await shot(p, "o05-after-run");
console.log("B:", await txt(p, 1800));
await b.close();
