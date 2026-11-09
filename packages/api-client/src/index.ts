import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";

export type { paths } from "./schema";
export type { components, operations } from "./schema";

export interface OrbitClientOptions {
  baseUrl: string;
  /** Returns the bearer token for the next request (dev token or OIDC access token). */
  token?: () => string | null | undefined;
  /** Extra headers, e.g. trace propagation. */
  headers?: Record<string, string>;
}

/**
 * Typed ORBIT API client generated from the backend OpenAPI document.
 *
 * The same client is intended for the web app and future native clients: it depends only
 * on `fetch`, supports a bearer-token provider, and has no browser-only assumptions.
 */
export function createOrbitClient(options: OrbitClientOptions) {
  const client = createClient<paths>({
    baseUrl: options.baseUrl.replace(/\/$/, ""),
    headers: options.headers,
  });

  const auth: Middleware = {
    async onRequest({ request }) {
      const token = options.token?.();
      if (token) request.headers.set("authorization", `Bearer ${token}`);
      return request;
    },
  };
  client.use(auth);
  return client;
}

export type OrbitClient = ReturnType<typeof createOrbitClient>;

/** Error envelope returned by every failing ORBIT API call. */
export interface OrbitApiErrorBody {
  error: {
    code: string;
    message: string;
    details?: Array<{ path: string; message: string }>;
    requestId?: string;
  };
}

export function isOrbitApiError(value: unknown): value is OrbitApiErrorBody {
  return Boolean(
    value &&
      typeof value === "object" &&
      "error" in value &&
      (value as OrbitApiErrorBody).error &&
      typeof (value as OrbitApiErrorBody).error.message === "string",
  );
}

/** Extract a human-readable message from an API error, network error or unknown value. */
export function errorMessage(error: unknown): string {
  if (isOrbitApiError(error)) return error.error.message;
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unexpected error";
}
