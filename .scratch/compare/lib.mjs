import { chromium } from "@playwright/test";
export async function start() {
  const b = await chromium.launch({ channel: "chrome", headless: false, slowMo: 300 });
  const p = await b.newPage({ viewport: { width: 1600, height: 1000 } });
  await p.addInitScript(() => {});
  p.on("pageerror", e => console.log("PAGEERROR", String(e).slice(0,200)));
  return { b, p };
}
export const shot = (p, n) => p.screenshot({ path: `/tmp/cmp/${n}.png` });
export const txt = async (p, n=1200) => (await p.locator("body").innerText()).replace(/\n+/g," | ").slice(0,n);
