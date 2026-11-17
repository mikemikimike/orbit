"use client";

import { request } from "./http";

export interface OutputView {
  id: string;
  name: string;
  kind: "slack" | "mattermost" | "webhook";
  agentId?: string | null;
  teamId?: string | null;
  events: string[];
  template?: string | null;
  enabled: boolean;
  hasWebhook: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface OutputInput {
  name: string;
  kind: "slack" | "mattermost" | "webhook";
  agentId?: string | null;
  teamId?: string | null;
  events: string[];
  webhookUrl?: string;
  template?: string | null;
  enabled: boolean;
}

export function listOutputs(): Promise<OutputView[]> {
  return request<OutputView[]>("/api/v1/outputs");
}

export function createOutput(body: OutputInput): Promise<OutputView> {
  return request<OutputView>("/api/v1/outputs", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function updateOutput(id: string, body: OutputInput): Promise<OutputView> {
  return request<OutputView>(`/api/v1/outputs/${id}`, {
    method: "PATCH",
    body: JSON.stringify(body),
  });
}

export function deleteOutput(id: string): Promise<void> {
  return request<void>(`/api/v1/outputs/${id}`, { method: "DELETE" });
}

export function testOutput(id: string): Promise<void> {
  return request<void>(`/api/v1/outputs/${id}/test`, {
    method: "POST",
    body: "{}",
  });
}
