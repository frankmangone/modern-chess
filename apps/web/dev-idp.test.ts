// @vitest-environment node
import { createPublicKey, generateKeyPairSync, verify } from "node:crypto";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { jwks, loadKey, mint } from "./dev-idp";

const dirs: string[] = [];
afterEach(() => {
  for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});

function keyFile(): string {
  const dir = mkdtempSync(join(tmpdir(), "dev-idp-"));
  dirs.push(dir);
  return join(dir, "key.pem");
}

function parts(token: string) {
  const [header, claims, signature] = token.split(".") as [string, string, string];
  return {
    header: JSON.parse(Buffer.from(header, "base64url").toString()) as { alg: string; kid: string },
    claims: JSON.parse(Buffer.from(claims, "base64url").toString()) as Record<string, unknown>,
    signed: `${header}.${claims}`,
    signature: Buffer.from(signature, "base64url"),
  };
}

describe("the fake sign-in provider", () => {
  it("signs a token that its own published key verifies", () => {
    const key = loadKey(keyFile());
    const { header, claims, signed, signature } = parts(mint(key, "http://localhost:3050/dev-idp", "Ada@Example.com", 1000));

    const [published] = jwks(key).keys as [{ kid: string } & Record<string, string>];
    expect(header).toMatchObject({ alg: "RS256", kid: published.kid });
    expect(verify("RSA-SHA256", Buffer.from(signed), createPublicKey({ key: published as never, format: "jwk" }), signature)).toBe(true);
    expect(claims).toMatchObject({ iss: "http://localhost:3050/dev-idp", email: "ada@example.com", email_verified: true, iat: 1000, nbf: 1000, exp: 1060 });
  });

  it("gives the same subject for the same email, in any case, and another for another email", () => {
    const key = loadKey(keyFile());
    const sub = (email: string) => parts(mint(key, "i", email)).claims.sub;

    expect(sub("ada@example.com")).toBe(sub("ADA@example.com"));
    expect(sub("ada@example.com")).not.toBe(sub("bob@example.com"));
  });

  it("refuses what is not an email", () => {
    const key = loadKey(keyFile());

    expect(() => mint(key, "i", "not an email")).toThrow("not an email");
  });

  it("keeps its key across restarts, so the key id the social API caches stays the same", () => {
    const file = keyFile();

    expect(jwks(loadKey(file))).toEqual(jwks(loadKey(file)));
    expect(jwks(loadKey(file))).not.toEqual(jwks(generateKeyPairSync("rsa", { modulusLength: 2048 }).privateKey));
  });
});
