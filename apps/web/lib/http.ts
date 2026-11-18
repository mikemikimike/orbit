"use client";

import { API_BASE_URL, getAccessToken, setAccessToken } from "./api";

/**
 * Authenticated JSON request against the control plane.
 *
 * Used by the hand-written API modules (agent teams, launcher) that predate the
 * generated client. Reads 204 as `undefined` and surfaces the error envelope message.
 */
export async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAccessToken();
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    cache: "no-store",
    headers: {
      ...(init?.headers ?? {}),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      "Content-Type": "application/json",
    },
  });
  if (response.status === 401 && token) {
    setAccessToken(null);
    if (typeof window !== "undefined" && !window.location.pathname.startsWith("/login")) {
      window.location.assign(
        `/login?next=${encodeURIComponent(window.location.pathname)}`,
      );
    }
  }
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as
      | { error?: { message?: string } }
      | null;
    throw new Error(body?.error?.message ?? `HTTP ${response.status}`);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}
