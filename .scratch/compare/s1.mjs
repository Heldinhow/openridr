import { chromium } from "@playwright/test";
const b = await chromium.launch({ channel: "chrome", headless: false, slowMo: 300 });
const p = await b.newPage({ viewport: { width: 1600, height: 1000 } });
p.on("console", m => { if (m.type()==="error") console.log("CONSOLE", m.text().slice(0,200)); });
await p.goto("http://127.0.0.1:1420");
await p.waitForTimeout(1500);
await p.screenshot({ path: "/tmp/cmp/o01-start.png" });
console.log((await p.locator("body").innerText()).slice(0,1500));
await b.close();
