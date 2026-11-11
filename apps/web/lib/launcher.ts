"use client";

import { API_BASE_URL, getAccessToken } from "./api";
import type { WorkerLaunch } from "./types";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAccessToken();
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      ...(init?.headers ?? {}),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      "Content-Type": "application/json",
    },
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as
      | { error?: { message?: string } }
      | null;
    throw new Error(body?.error?.message ?? `HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

export function launchWorkerProfile(profileId: string): Promise<WorkerLaunch> {
  return request<WorkerLaunch>(`/api/v1/worker-profiles/${profileId}/launch`, {
    method: "POST",
    body: "{}",
  });
}

export function stopWorkerLaunch(launchId: string): Promise<WorkerLaunch> {
  return request<WorkerLaunch>(`/api/v1/worker-launches/${launchId}/stop`, {
    method: "POST",
    body: "{}",
  });
}

export function workerLaunchLogs(launchId: string): Promise<{ logs: string }> {
  return request<{ logs: string }>(`/api/v1/worker-launches/${launchId}/logs`);
}

export function workerLaunches(): Promise<WorkerLaunch[]> {
  return request<WorkerLaunch[]>("/api/v1/worker-launches");
}
