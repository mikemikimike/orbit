"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "./api";
import type {
  AgentDetail,
  AgentPage,
  AgentVersionView,
  ConnectionView,
  ConnectorView,
  DashboardResponse,
  ExecutionEventRecord,
  ExecutionPage,
  ExecutionRecord,
  MeResponse,
  MetaResponse,
  ProviderView,
  HarnessView,
  TriggerRecord,
  ApiTokenView,
  AgentInstanceView,
  ApprovalWithExecution,
  WorkerView,
  Skill,
  WorkerProfile,
  WorkerLaunch,
} from "./types";
import { workerLaunches } from "./launcher";

export function unwrap<T>(result: {
  data?: T;
  error?: unknown;
  response: Response;
}): T {
  if (result.error) throw result.error;
  if (result.data === undefined) {
    throw new Error(`HTTP ${result.response.status} ${result.response.statusText}`);
  }
  return result.data;
}

export const queryKeys = {
  meta: ["meta"] as const,
  me: ["me"] as const,
  dashboard: ["dashboard"] as const,
  agents: (params?: Record<string, unknown>) => ["agents", params ?? {}] as const,
  agent: (id: string) => ["agent", id] as const,
  agentVersions: (id: string) => ["agent", id, "versions"] as const,
  executions: (params?: Record<string, unknown>) => ["executions", params ?? {}] as const,
  execution: (id: string) => ["execution", id] as const,
  executionEvents: (id: string) => ["execution", id, "events"] as const,
  harnesses: ["harnesses"] as const,
  providers: ["providers"] as const,
  connectors: ["connectors"] as const,
  connections: ["connections"] as const,
  triggers: (agentId?: string) => ["triggers", agentId ?? "all"] as const,
  tokens: ["api-tokens"] as const,
  instances: ["agent-instances"] as const,
  approvals: (status?: string) => ["approvals", status ?? "all"] as const,
  workers: ["workers"] as const,
  skills: ["skills"] as const,
  workerProfiles: ["worker-profiles"] as const,
  workerLaunches: ["worker-launches"] as const,
};

export function useMeta() {
  return useQuery<MetaResponse>({
    queryKey: queryKeys.meta,
    queryFn: async () => unwrap(await api.GET("/api/v1/meta")),
    staleTime: 5 * 60_000,
  });
}

export function useMe() {
  return useQuery<MeResponse>({
    queryKey: queryKeys.me,
    queryFn: async () => unwrap(await api.GET("/api/v1/me")),
    staleTime: 60_000,
  });
}

export function useDashboard() {
  return useQuery<DashboardResponse>({
    queryKey: queryKeys.dashboard,
    queryFn: async () => unwrap(await api.GET("/api/v1/dashboard/summary")),
    refetchInterval: 15_000,
  });
}

export function useAgents(params: {
  status?: string;
  q?: string;
  limit?: number;
  cursor?: string;
} = {}) {
  return useQuery<AgentPage>({
    queryKey: queryKeys.agents(params),
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/agents", {
          params: {
            query: {
              limit: params.limit,
              cursor: params.cursor,
              status: params.status,
              q: params.q,
            },
          },
        }),
      ),
    placeholderData: (previous) => previous,
  });
}

export function useAgent(id: string | undefined) {
  return useQuery<AgentDetail>({
    queryKey: queryKeys.agent(id ?? ""),
    queryFn: async () => unwrap(await api.GET("/api/v1/agents/{agent_id}", { params: { path: { agent_id: id! } } })),
    enabled: Boolean(id),
    refetchInterval: 20_000,
  });
}

export function useAgentVersions(id: string | undefined) {
  return useQuery<AgentVersionView[]>({
    queryKey: queryKeys.agentVersions(id ?? ""),
    queryFn: async () =>
      unwrap(await api.GET("/api/v1/agents/{agent_id}/versions", { params: { path: { agent_id: id! } } })),
    enabled: Boolean(id),
  });
}

export function useExecutions(params: {
  status?: string;
  agentId?: string;
  limit?: number;
  cursor?: string;
} = {}) {
  return useQuery<ExecutionPage>({
    queryKey: queryKeys.executions(params),
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/executions", {
          params: {
            query: {
              limit: params.limit,
              cursor: params.cursor,
              status: params.status,
              agent_id: params.agentId,
            },
          },
        }),
      ),
    refetchInterval: 10_000,
    placeholderData: (previous) => previous,
  });
}

export function useExecution(id: string | undefined) {
  return useQuery<ExecutionRecord>({
    queryKey: queryKeys.execution(id ?? ""),
    queryFn: async () =>
      unwrap(await api.GET("/api/v1/executions/{execution_id}", { params: { path: { execution_id: id! } } })),
    enabled: Boolean(id),
  });
}

export function useExecutionEvents(id: string | undefined, options?: { enabled?: boolean }) {
  return useQuery<{ items: ExecutionEventRecord[] }>({
    queryKey: queryKeys.executionEvents(id ?? ""),
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/executions/{execution_id}/events", {
          params: { path: { execution_id: id! }, query: { limit: 1000 } },
        }),
      ),
    enabled: Boolean(id) && (options?.enabled ?? true),
  });
}

export function useHarnesses() {
  return useQuery<HarnessView[]>({
    queryKey: queryKeys.harnesses,
    queryFn: async () => unwrap(await api.GET("/api/v1/harnesses")),
    staleTime: 60_000,
  });
}

export function useProviders() {
  return useQuery<ProviderView[]>({
    queryKey: queryKeys.providers,
    queryFn: async () => unwrap(await api.GET("/api/v1/providers")),
  });
}

export function useConnectors() {
  return useQuery<ConnectorView[]>({
    queryKey: queryKeys.connectors,
    queryFn: async () => unwrap(await api.GET("/api/v1/connectors")),
    staleTime: 5 * 60_000,
  });
}

export function useConnections() {
  return useQuery<ConnectionView[]>({
    queryKey: queryKeys.connections,
    queryFn: async () => unwrap(await api.GET("/api/v1/connections")),
  });
}

export function useTriggers(agentId?: string) {
  return useQuery<TriggerRecord[]>({
    queryKey: queryKeys.triggers(agentId),
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/triggers", {
          params: { query: { agentId: agentId } },
        }),
      ),
  });
}

export function useApiTokens() {
  return useQuery<ApiTokenView[]>({
    queryKey: queryKeys.tokens,
    queryFn: async () => unwrap(await api.GET("/api/v1/api-tokens")),
  });
}

export function useAgentInstances() {
  return useQuery<{ items: AgentInstanceView[]; total: number }>({
    queryKey: queryKeys.instances,
    queryFn: async () => unwrap(await api.GET("/api/v1/agent-instances")),
  });
}

export function useWorkers() {
  return useQuery<WorkerView[]>({
    queryKey: queryKeys.workers,
    queryFn: async () => unwrap(await api.GET("/api/v1/workers")),
    refetchInterval: 10_000,
  });
}

export function useSkills() {
  return useQuery<Skill[]>({
    queryKey: queryKeys.skills,
    queryFn: async () => unwrap(await api.GET("/api/v1/skills")),
  });
}

export function useWorkerProfiles() {
  return useQuery<WorkerProfile[]>({
    queryKey: queryKeys.workerProfiles,
    queryFn: async () => unwrap(await api.GET("/api/v1/worker-profiles")),
  });
}

export function useWorkerLaunches() {
  return useQuery<WorkerLaunch[]>({
    queryKey: queryKeys.workerLaunches,
    queryFn: workerLaunches,
    refetchInterval: 5_000,
  });
}

export function useApprovals(status?: string) {
  return useQuery<ApprovalWithExecution[]>({
    queryKey: queryKeys.approvals(status),
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/approvals", {
          params: { query: { status, limit: 100 } },
        }),
      ),
    refetchInterval: 5_000,
  });
}

export function useAuditLogs() {
  return useQuery<{
    items: Array<{
      id: number;
      action: string;
      actorType: string;
      actorEmail?: string | null;
      targetType?: string | null;
      targetId?: string | null;
      createdAt: string;
      metadata?: unknown;
    }>;
    nextCursor?: string | null;
  }>({
    queryKey: ["audit-logs"] as const,
    queryFn: async () =>
      unwrap(await api.GET("/api/v1/audit-logs", { params: { query: { limit: 25 } } })) as never,
  });
}
