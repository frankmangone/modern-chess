import { defineConfig } from "vitest/config";
import solidPlugin from "vite-plugin-solid";

// In development the browser talks to one origin, and Vite forwards each API to its own
// process. In production a reverse proxy does the same job.
export default defineConfig({
  plugins: [solidPlugin()],
  server: {
    port: 3050,
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
  resolve: { conditions: ["development", "browser"] },
});
