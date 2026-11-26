// @vitest-environment node
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { serveBook } from "./docs-book";

let root: string;
let server: Server;
let base: string;

async function serve(dir: string) {
  const s = createServer(serveBook(dir));
  await new Promise<void>((done) => s.listen(0, "127.0.0.1", done));
  return { s, url: `http://127.0.0.1:${(s.address() as AddressInfo).port}` };
}

beforeAll(async () => {
  root = mkdtempSync(join(tmpdir(), "book-"));
  const book = join(root, "book");
  mkdirSync(join(book, "theme", "pkg"), { recursive: true });
  mkdirSync(join(book, "sub"));
  writeFileSync(join(book, "index.html"), "<h1>Book</h1>");
  writeFileSync(join(book, "sub", "index.html"), "<h1>Sub</h1>");
  writeFileSync(join(book, "theme", "pkg", "engine.wasm"), "\0asm");
  writeFileSync(join(root, "secret.txt"), "secret");
  const started = await serve(book);
  server = started.s;
  base = started.url;
});

afterAll(() => {
  server.close();
  rmSync(root, { recursive: true, force: true });
});

describe("the docs book under /docs", () => {
  it("answers the folder with its index.html", async () => {
    const res = await fetch(`${base}/`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/html");
    expect(await res.text()).toBe("<h1>Book</h1>");
  });

  it("serves the engine as application/wasm", async () => {
    const res = await fetch(`${base}/theme/pkg/engine.wasm`);
    expect(res.headers.get("content-type")).toBe("application/wasm");
  });

  it("sends a folder without a slash to the one with it, under /docs", async () => {
    const res = await fetch(`${base}/sub`, { redirect: "manual" });
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("/docs/sub/");
  });

  it("does not leave the book folder", async () => {
    const res = await fetch(`${base}/..%2Fsecret.txt`);
    expect(res.status).toBe(404);
  });

  it("answers an unknown page with 404", async () => {
    expect((await fetch(`${base}/nope.html`)).status).toBe(404);
  });

  it("says how to build the book when it is not built", async () => {
    const empty = await serve(join(root, "not-there"));
    const res = await fetch(`${empty.url}/`);
    expect(res.status).toBe(404);
    expect(await res.text()).toContain("make book");
    empty.s.close();
  });
});
