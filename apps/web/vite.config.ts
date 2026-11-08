import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import solidPlugin from "vite-plugin-solid";

// In development the browser talks to one origin, and Vite forwards each API to its own
// process. In production a reverse proxy does the same job.
export default defineConfig({
  plugins: [solidPlugin()],
  server: {
    port: 3050,
    // The board element lives with the docs theme, outside this app's folder.
    fs: { allow: [".", "../docs/theme"] },
    proxy: {
      "/api/game": "http://localhost:3001",
      "/api/social": "http://localhost:3002",
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test/setup.ts"],
    server: { deps: { inline: [/solid-js/, /@solidjs/] } },
  },
  resolve: {
    conditions: ["development", "browser"],
    // The board element is shared with the docs, so it stays in the docs theme folder.
    alias: { "@board": fileURLToPath(new URL("../docs/theme", import.meta.url)) },
  },
});
