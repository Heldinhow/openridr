import { host } from "./environment";

export default async function globalTeardown() {
  const response = await fetch(`${host}/call`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ method: "workspace.close", params: {} }),
  });
  if (!response.ok) throw new Error(`Could not close E2E workspace: HTTP ${response.status}`);
  const result = await response.json();
  if (!result.ok) throw new Error(`Could not close E2E workspace: ${JSON.stringify(result.error)}`);
}
