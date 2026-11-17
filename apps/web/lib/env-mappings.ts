"use client";

import { request } from "./http";

export interface EnvMappingEntry {
  envVar: string;
  connectionId?: string | null;
  secretKey?: string | null;
  secretRef?: string | null;
}

export interface EnvMappingRecord {
  id: string;
  name: string;
  description: string;
  mappings: EnvMappingEntry[];
  createdAt: string;
  updatedAt: string;
}

export function listEnvMappings(): Promise<EnvMappingRecord[]> {
  return request<EnvMappingRecord[]>("/api/v1/env-mappings");
}

export function workerProfileEnvMappings(profileId: string): Promise<EnvMappingRecord[]> {
  return request<EnvMappingRecord[]>(`/api/v1/worker-profiles/${profileId}/env-mappings`);
}

export function createEnvMapping(body: {
  name: string;
  description: string;
  mappings: EnvMappingEntry[];
}): Promise<EnvMappingRecord> {
  return request<EnvMappingRecord>("/api/v1/env-mappings", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function deleteEnvMapping(id: string): Promise<void> {
  return request<void>(`/api/v1/env-mappings/${id}`, { method: "DELETE" });
}
