import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import solidPlugin from "vite-plugin-solid";
import { docsBook } from "./docs-book";
import { devIdp } from "./dev-idp";

// In development the browser talks to one origin, and Vite forwards each API to its own
// process. In production a reverse proxy does the same job.
export default defineConfig({
  // The docs book, built by `make book`, is served at /docs/ like the share gateway does.
  plugins: [
    solidPlugin(),
    docsBook(fileURLToPath(new URL("../docs/book", import.meta.url))),
    // The fake sign-in provider for `make dev` (the default AUTH); not part of a build.
    devIdp(fileURLToPath(new URL(".dev-idp/key.pem", import.meta.url))),
  ],
  server: {
    port: 3050,
    // The board element lives with the docs theme, outside this app's folder.
    fs: { allow: [".", "../docs/theme", "../docs/examples"] },
    proxy: {
      "/api/game": "http://localhost:3001",
      "/api/social": "http://localhost:3002",
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}", "docs-book.test.ts", "dev-idp.test.ts"],
    setupFiles: ["src/test/setup.ts"],
    // The stylesheet is read as text by the theme test, which checks its walnut values.
    css: { include: [/styles\.css/] },
    server: { deps: { inline: [/solid-js/, /@solidjs/] } },
  },
  resolve: {
    conditions: ["development", "browser"],
    // The board element is shared with the docs, so it stays in the docs theme folder.
    alias: {
      "@board": fileURLToPath(new URL("../docs/theme", import.meta.url)),
      // The docs book's first game, which the workbench offers as somewhere to start.
      "@examples": fileURLToPath(new URL("../docs/examples", import.meta.url)),
    },
  },
});
