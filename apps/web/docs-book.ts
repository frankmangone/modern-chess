import { createReadStream, statSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { extname, join, normalize, sep } from "node:path";
import type { Plugin } from "vite";

// What the book's own files need. The board engine is only instantiated from `application/wasm`.
const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".mjs": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
  ".txt": "text/plain; charset=utf-8",
};

/** Serves the built book (`apps/docs/book`) under `/docs/`, as the share gateway does. `url` is
 *  what is left of the address after `/docs`. A folder answers with its `index.html`. */
export function serveBook(dir: string) {
  return (req: IncomingMessage, res: ServerResponse) => {
    const path = decodeURIComponent((req.url ?? "/").split("?")[0]);
    const withoutDots = normalize(path);
    // Nothing outside the book folder, whatever the address says.
    if (withoutDots.split(sep).includes("..")) return notFound(res);
    let file = join(dir, withoutDots);
    try {
      if (statSync(file).isDirectory()) {
        // `/docs` and `/docs/chapter` have no trailing slash; relative links need one.
        // Connect strips the mount from `url` (so `/docs` reads as `/`) and keeps the whole address
        // in `originalUrl`.
        const asked = (((req as { originalUrl?: string }).originalUrl ?? `/docs${req.url}`) as string).split("?")[0];
        if (!asked.endsWith("/")) {
          res.statusCode = 301;
          res.setHeader("Location", `${asked}/`);
          return res.end();
        }
        file = join(file, "index.html");
      }
      const size = statSync(file).size;
      res.setHeader("Content-Type", TYPES[extname(file)] ?? "application/octet-stream");
      res.setHeader("Content-Length", size);
      // The book is rebuilt while developing, and a cached page would show the old one.
      res.setHeader("Cache-Control", "no-cache");
      if (req.method === "HEAD") return res.end();
      createReadStream(file).pipe(res);
    } catch {
      if (!statBookExists(dir)) return missing(res);
      notFound(res);
    }
  };
}

function statBookExists(dir: string): boolean {
  try {
    return statSync(dir).isDirectory();
  } catch {
    return false;
  }
}

function notFound(res: ServerResponse) {
  res.statusCode = 404;
  res.setHeader("Content-Type", "text/plain; charset=utf-8");
  res.end("Not found");
}

function missing(res: ServerResponse) {
  res.statusCode = 404;
  res.setHeader("Content-Type", "text/plain; charset=utf-8");
  res.end("The book is not built. Run `make book` (it needs mdbook: brew install mdbook).");
}

/** Vite's dev server serves the book at /docs/, ahead of the app's own pages. */
export function docsBook(dir: string): Plugin {
  const handler = serveBook(dir);
  return {
    name: "docs-book",
    configureServer(server) {
      server.middlewares.use("/docs", handler);
    },
  };
}
