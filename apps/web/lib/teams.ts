"use client";

import type { components } from "@orbit/api-client";
import { request } from "./http";
import type {
  AgentTeam,
  TeamRun,
  TeamRunDetail,
  TeamRepository,
  WorkSource,
} from "./types";

export type TeamStepInput = { agentId: string };

export function listAgentTeams(): Promise<AgentTeam[]> {
  return request<AgentTeam[]>("/api/v1/agent-teams");
}

type TeamInput = {
  name: string;
  description: string;
  mode: "pipeline" | "native";
  instructions: string;
  repository?: string;
  branch?: string;
  workSource?: WorkSource | null;
  repositories?: TeamRepository[];
  workerProfileId?: string;
  autoLaunch?: boolean;
  skills: string[];
  steps: TeamStepInput[];
};

export function createAgentTeam(input: TeamInput): Promise<AgentTeam> {
  return request<AgentTeam>("/api/v1/agent-teams", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function updateAgentTeam(
  teamId: string,
  input: TeamInput,
): Promise<AgentTeam> {
  return request<AgentTeam>(`/api/v1/agent-teams/${teamId}`, {
    method: "PATCH",
    body: JSON.stringify(input),
  });
}

export function deleteAgentTeam(teamId: string): Promise<void> {
  return request<void>(`/api/v1/agent-teams/${teamId}`, { method: "DELETE" });
}

export function startTeamRun(
  teamId: string,
  task: string,
  repository?: string,
  autoApprove = false,
  harnessOverride?: string,
): Promise<TeamRunDetail> {
  return request<TeamRunDetail>(`/api/v1/agent-teams/${teamId}/runs`, {
    method: "POST",
    body: JSON.stringify({
      task,
      payload: repository?.trim() ? { repository: repository.trim() } : {},
      autoApprove,
      harnessOverride: harnessOverride || undefined,
    }),
  });
}

export function listTeamRuns(archived?: boolean): Promise<TeamRun[]> {
  const query = archived === undefined ? "" : `?archived=${archived}`;
  return request<TeamRun[]>(`/api/v1/team-runs${query}`);
}

export function getTeamRun(runId: string): Promise<TeamRunDetail> {
  return request<TeamRunDetail>(`/api/v1/team-runs/${runId}`);
}

export type TeamQueueItem = components["schemas"]["TeamQueueItem"];

export function listTeamQueue(teamId: string): Promise<TeamQueueItem[]> {
  return request<TeamQueueItem[]>(`/api/v1/agent-teams/${teamId}/queue`);
}

export function enqueueTeamItems(
  teamId: string,
  items: Array<{ externalKey: string; repository?: string; payload?: Record<string, unknown> }>,
): Promise<TeamQueueItem[]> {
  return request<TeamQueueItem[]>(`/api/v1/agent-teams/${teamId}/queue`, {
    method: "POST",
    body: JSON.stringify({ items }),
  });
}

export function runTeamQueueItem(
  teamId: string,
  itemId: string,
): Promise<TeamRunDetail> {
  return request<TeamRunDetail>(
    `/api/v1/agent-teams/${teamId}/queue/${itemId}/run`,
    { method: "POST", body: "{}" },
  );
}

export function retryTeamQueueItem(
  teamId: string,
  itemId: string,
): Promise<TeamQueueItem> {
  return request<TeamQueueItem>(
    `/api/v1/agent-teams/${teamId}/queue/${itemId}/retry`,
    { method: "POST", body: "{}" },
  );
}

export function requeueTeamQueueItem(
  teamId: string,
  itemId: string,
): Promise<TeamQueueItem> {
  return request<TeamQueueItem>(
    `/api/v1/agent-teams/${teamId}/queue/${itemId}/requeue`,
    { method: "POST", body: "{}" },
  );
}

export function cancelTeamQueueItem(
  teamId: string,
  itemId: string,
): Promise<TeamQueueItem> {
  return request<TeamQueueItem>(
    `/api/v1/agent-teams/${teamId}/queue/${itemId}/cancel`,
    { method: "POST", body: "{}" },
  );
}
