import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";
import { frontend, host, uiPort } from "./tests/e2e/environment";

// Test-only same-origin bridge; production/dev CORS and host contracts stay unchanged.
export default mergeConfig(base, defineConfig({
  server: {
    host: "127.0.0.1",
    port: uiPort,
    cors: false,
    proxy: {
      "^/__host/(call|health)$": {
        target: host,
        rewrite: (url) => url.replace(/^\/__host/, ""),
        bypass: (request) => {
          if (request.headers.origin && request.headers.origin !== frontend) return false;
        },
      },
    },
  },
}));
