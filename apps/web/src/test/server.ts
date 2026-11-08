import { vi } from "vitest";

// A stand-in for the two APIs: each route answers one "METHOD /path", and every call is
// recorded so a test can check what was asked, in what order, and with whose token.

export interface Call {
  route: string;
  authorization: string | null;
  body: unknown;
}

type Answer = () => Response | Promise<Response>;

export function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status });
}

export function stubFetch(routes: Record<string, Answer>): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string, init: RequestInit = {}) => {
      const route = `${init.method ?? "GET"} ${path}`;
      const headers = new Headers(init.headers);
      calls.push({
        route,
        authorization: headers.get("Authorization"),
        body: typeof init.body === "string" ? JSON.parse(init.body) : undefined,
      });
      const answer = routes[route];
      if (answer === undefined) return json({ error: `no stub for ${route}` }, 500);
      return answer();
    }),
  );
  return calls;
}
