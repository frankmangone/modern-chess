/** The server answered, and the answer was an error. `message` is safe to show a person. */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

/** The server could not be reached at all (it is down, or the network is). */
export class NetworkError extends Error {
  constructor() {
    super("the server could not be reached");
    this.name = "NetworkError";
  }
}

interface RequestOptions {
  method?: "GET" | "POST";
  token?: string;
  body?: unknown;
}

async function errorMessage(response: Response): Promise<string> {
  const fallback = `the server could not answer (${response.status})`;
  try {
    const body: unknown = await response.json();
    if (typeof body === "object" && body !== null && "error" in body) {
      const message = (body as { error: unknown }).error;
      if (typeof message === "string" && message !== "") return message;
    }
  } catch {
    // Not JSON: a proxy's error page, for instance.
  }
  return fallback;
}

/** One JSON call. Throws `NetworkError` or `ApiError`, never a bare failure. */
export async function requestJson<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  if (options.token !== undefined) headers.Authorization = `Bearer ${options.token}`;
  if (options.body !== undefined) headers["Content-Type"] = "application/json";

  let response: Response;
  try {
    response = await fetch(path, {
      method: options.method ?? "GET",
      headers,
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });
  } catch {
    throw new NetworkError();
  }

  if (!response.ok) throw new ApiError(response.status, await errorMessage(response));
  return (await response.json()) as T;
}
