import { start, shot, txt } from "./lib.mjs";
const { b, p } = await start();
await p.goto("http://127.0.0.1:1420");
await p.getByTestId("workspace-path-input").fill("/tmp/OrderDesk/OrderDesk.sln");
await p.getByTestId("workspace-open").click();
await p.getByTestId("trust-checkbox").check();
await p.getByTestId("trust-grant").click();
await p.waitForTimeout(2000);
await p.getByRole("button",{name:"Open workspace"}).click();
await p.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
await p.waitForTimeout(1000);
const t0 = Date.now();
await p.getByTestId("toolbar-run").click({force:true});
for (let i=0;i<60;i++){ await p.waitForTimeout(1000); if ((await txt(p,9000)).includes("Process finished")) break; }
console.log("run output after", (Date.now()-t0)/1000, "s;", (await txt(p,9000)).includes("Order A-104: R$ 42,00"));
await shot(p, "o08-run-oneclick");
// breakpoint + debug
const line = p.locator(".cm-line").nth(2);
const lb = await line.boundingBox(); const gb = await p.locator(".cm-debug-gutter").boundingBox();
await p.mouse.click(gb.x+5, lb.y+lb.height/2);
await p.getByTestId("toolbar-debug").click({force:true});
await shot(p, "o09-debug-consent");
const consent = await p.getByTestId("debug-consent-dialog").isVisible().catch(()=>false);
console.log("consent dialog visible:", consent);
if (consent) await p.getByTestId("debug-consent-accept").click();
for (let i=0;i<80;i++){ await p.waitForTimeout(1000); const s=await p.getByTestId("dap-state").textContent().catch(()=>""); if (/Paused/i.test(s)) break; }
console.log("dap:", await p.getByTestId("dap-state").textContent());
await shot(p, "o10-debug-paused");
await b.close();
