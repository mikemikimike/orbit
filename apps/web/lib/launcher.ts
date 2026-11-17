"use client";

import { request } from "./http";
import type { WorkerLaunch } from "./types";

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

export function workerLaunches(includeInactive = false): Promise<WorkerLaunch[]> {
  const query = includeInactive ? "?includeInactive=true" : "";
  return request<WorkerLaunch[]>(`/api/v1/worker-launches${query}`);
}
