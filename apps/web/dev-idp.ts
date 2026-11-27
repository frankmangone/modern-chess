import { createHash, createPrivateKey, createPublicKey, generateKeyPairSync, sign, type KeyObject } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { dirname } from "node:path";
import type { Plugin } from "vite";

// A fake sign-in provider for local development, run by the dev server. It signs tokens the way
// Clerk does (RS256, published keys), so the social API trusts it through the same settings:
// SOCIAL_PROVIDER_ISSUER=http://localhost:3050/dev-idp. It hands a token to anyone who asks, so
// it must never run anywhere but a developer's machine: it is a dev-server plugin (`apply: "serve"`)
// and the built app has no such route.

export const ISSUER_PATH = "/dev-idp";
const EMAIL = /^[^@\s]+@[^@\s]+$/;
const LIFETIME_SECONDS = 60;

function base64url(value: Buffer | string): string {
  return Buffer.from(value).toString("base64url");
}

/** The key, made once and kept in a file so that restarting the dev server does not change the
 *  key id, which the social API caches. */
export function loadKey(file: string): KeyObject {
  if (existsSync(file)) return createPrivateKey(readFileSync(file, "utf8"));
  const { privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, privateKey.export({ type: "pkcs8", format: "pem" }), { mode: 0o600 });
  return privateKey;
}

export function jwks(key: KeyObject): { keys: Record<string, unknown>[] } {
  const jwk = createPublicKey(key).export({ format: "jwk" });
  const kid = createHash("sha256").update(JSON.stringify({ e: jwk.e, kty: jwk.kty, n: jwk.n })).digest("base64url").slice(0, 16);
  return { keys: [{ ...jwk, kid, alg: "RS256", use: "sig" }] };
}

/** A signed token for an email: the subject is stable for the same email, so signing in again
 *  is the same account. Throws on something that is not an email. */
export function mint(key: KeyObject, issuer: string, email: string, nowSeconds = Math.floor(Date.now() / 1000)): string {
  if (!EMAIL.test(email)) throw new Error("not an email");
  const [{ kid }] = jwks(key).keys as [{ kid: string }];
  const header = { alg: "RS256", typ: "JWT", kid };
  const sub = `dev_${createHash("sha256").update(email.toLowerCase()).digest("hex").slice(0, 24)}`;
  const claims = { iss: issuer, sub, email: email.toLowerCase(), email_verified: true, iat: nowSeconds, nbf: nowSeconds, exp: nowSeconds + LIFETIME_SECONDS };
  const signed = `${base64url(JSON.stringify(header))}.${base64url(JSON.stringify(claims))}`;
  return `${signed}.${base64url(sign("RSA-SHA256", Buffer.from(signed), key))}`;
}

async function readBody(request: IncomingMessage): Promise<string> {
  let body = "";
  for await (const chunk of request) {
    body += String(chunk);
    if (body.length > 4096) break;
  }
  return body;
}

function send(response: ServerResponse, status: number, body: unknown): void {
  response.statusCode = status;
  response.setHeader("Content-Type", "application/json");
  response.end(JSON.stringify(body));
}

export function devIdp(keyFile: string): Plugin {
  return {
    name: "dev-idp",
    apply: "serve",
    configureServer(server) {
      const key = loadKey(keyFile);
      server.middlewares.use(async (request, response, next) => {
        const path = (request.url ?? "").split("?")[0];
        if (path === `${ISSUER_PATH}/.well-known/jwks.json` && request.method === "GET") return send(response, 200, jwks(key));
        if (path === `${ISSUER_PATH}/token` && request.method === "POST") {
          try {
            const { email } = JSON.parse(await readBody(request)) as { email?: unknown };
            if (typeof email !== "string") return send(response, 400, { error: "an email is needed" });
            const port = server.config.server.port ?? 3050;
            return send(response, 200, { token: mint(key, `http://localhost:${port}${ISSUER_PATH}`, email) });
          } catch {
            return send(response, 400, { error: "that is not an email" });
          }
        }
        next();
      });
    },
  };
}
