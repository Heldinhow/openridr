import { expect, test } from "@playwright/test";
import { e2eHome, frontend, host, port } from "./environment";

test("E2E ports are validated and the loopback proxy rejects foreign origins", async ({ request }) => {
  expect(port(undefined, 1420)).toBe(1420);
  expect(port("15420", 1420)).toBe(15420);
  for (const value of ["", "0", "65536", "1.5", "1420;echo", "-1"]) {
    expect(() => port(value, 1420)).toThrow("Invalid E2E port");
  }
  const proxied = await request.get(`${frontend}/__host/health`, { headers: { Origin: frontend } });
  expect(proxied.ok()).toBe(true);
  expect((await proxied.json()).service).toBe("openridr-host-http");
  const refused = await request.post(`${frontend}/__host/call`, {
    headers: { Origin: "https://untrusted.example" },
    data: { method: "app.state", params: {} },
  });
  expect(refused.status()).toBe(404);
  expect(refused.headers()["access-control-allow-origin"]).toBeUndefined();
  const direct = await request.get(`${host}/health`, { headers: { Origin: frontend } });
  if (!frontend.endsWith(":1420")) expect(direct.headers()["access-control-allow-origin"]).toBeUndefined();
});

test("the E2E host uses the isolated app home", async ({ request }) => {
  const response = await request.post(`${host}/call`, {
    data: { method: "app.state", params: {} },
  });
  expect(response.ok()).toBe(true);
  const body = (await response.json()) as {
    ok: boolean;
    data?: { app?: { appHome?: string } };
  };
  expect(body.ok).toBe(true);
  // The real app home holds the user's trusted.json/recent.json; E2E must never write there.
  expect(body.data?.app?.appHome).toBe(e2eHome);
});
