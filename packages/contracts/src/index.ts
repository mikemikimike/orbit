import { z } from "zod";

/**
 * Shared domain schemas. Used by the web app today and by the planned Expo app later.
 * Keep these aligned with `crates/orbit-domain` (camelCase JSON, orbit.io/v1).
 */

export const runtimeKindSchema = z.enum(["mock", "opencode", "claude_code", "pi", "generic"]);
export type RuntimeKind = z.infer<typeof runtimeKindSchema>;

export const executionModeSchema = z.enum(["on_demand", "long_running"]);
export const agentStatusSchema = z.enum(["draft", "active", "paused", "archived"]);
export const executionStatusSchema = z.enum([
  "queued",
  "dispatched",
  "running",
  "awaiting_approval",
  "succeeded",
  "failed",
  "timed_out",
  "cancelled",
  "rejected",
]);
export type ExecutionStatus = z.infer<typeof executionStatusSchema>;

export const modelManagementSchema = z.enum(["runtime", "platform"]);

export const cronSchema = z
  .string()
  .trim()
  .min(1, "cron expression is required")
  .refine((value) => value.split(/\s+/).length === 5, "use a 5-field cron expression (UTC)");

export const connectorBindingSchema = z.object({
  connector: z
    .string()
    .min(1, "connector is required")
    .regex(/^[a-z0-9][a-z0-9_-]{0,62}$/, "invalid connector slug"),
  connectionId: z.string().uuid().optional(),
  capabilities: z.array(z.string()).default([]),
});

export const triggerSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("schedule"), cron: cronSchema, timezone: z.string().optional() }),
  z.object({ type: z.literal("webhook"), secretRef: z.string().optional() }),
  z.object({ type: z.literal("event"), subject: z.string().min(1, "event subject is required") }),
  z.object({ type: z.literal("manual") }),
]);

export const autonomySchema = z.object({
  level: z.number().int().min(1).max(4),
  approvalMode: z.enum(["auto", "always", "writes"]).default("auto"),
  maxExecutionsPerHour: z.number().int().min(1).max(10000).optional(),
});

export const resourceLimitsSchema = z.object({
  timeoutSeconds: z.number().int().min(1).max(86400),
  maxConcurrentExecutions: z.number().int().min(1).max(20),
  memoryMb: z.number().int().min(64).max(65536).optional(),
  cpuMillis: z.number().int().min(100).max(64000).optional(),
  maxOutputTokens: z.number().int().min(1).max(1_000_000).optional(),
});

export const agentSpecSchema = z.object({
  runtime: z.object({
    type: runtimeKindSchema,
    executionMode: executionModeSchema.default("on_demand"),
    config: z.record(z.string(), z.unknown()).default({}),
  }),
  model: z
    .object({
      management: modelManagementSchema.default("runtime"),
      provider: z.string().optional(),
      model: z.string().optional(),
      params: z.record(z.string(), z.unknown()).default({}),
    })
    .default({ management: "runtime", params: {} }),
  instructions: z.object({
    system: z.string().min(1, "system instructions are required").max(200_000),
    constraints: z.array(z.string()).default([]),
  }),
  connectors: z.array(connectorBindingSchema).default([]),
  triggers: z.array(triggerSchema).default([]),
  autonomy: autonomySchema.default({ level: 2, approvalMode: "auto" }),
  resources: resourceLimitsSchema.default({
    timeoutSeconds: 300,
    maxConcurrentExecutions: 1,
  }),
  memory: z
    .object({
      enabled: z.boolean().default(false),
      strategy: z.enum(["none", "short_term", "summary"]).default("none"),
      retentionDays: z.number().int().min(1).max(3650).optional(),
    })
    .default({ enabled: false, strategy: "none" }),
});

export const agentDefinitionSchema = z.object({
  apiVersion: z.literal("orbit.io/v1"),
  kind: z.literal("Agent"),
  metadata: z.object({
    name: z.string().trim().min(1, "name is required").max(120),
    description: z.string().max(2000).default(""),
    labels: z.record(z.string(), z.string()).default({}),
  }),
  spec: agentSpecSchema,
});
export type AgentDefinition = z.infer<typeof agentDefinitionSchema>;

export const providerKindSchema = z.enum(["openai_compatible", "runtime_managed"]);

export const providerConfigSchema = z
  .object({
    baseUrl: z
      .string()
      .url("base URL must be a valid URL")
      .refine(
        (value) => value.startsWith("http://") || value.startsWith("https://"),
        "base URL must use http(s)",
      ),
    model: z.string().min(1, "model id is required"),
    contextWindow: z.number().int().min(128).optional(),
    maxOutputTokens: z.number().int().min(1).optional(),
    headers: z.record(z.string(), z.string()).default({}),
    requestParams: z.record(z.string(), z.unknown()).default({}),
    compat: z
      .object({
        chatCompletions: z.boolean().default(true),
        responsesApi: z.boolean().default(false),
        tools: z.boolean().default(false),
        streaming: z.boolean().default(true),
        jsonMode: z.boolean().default(false),
        vision: z.boolean().default(false),
        embeddings: z.boolean().default(false),
        modelsEndpoint: z.boolean().default(true),
      })
      .default({
        chatCompletions: true,
        responsesApi: false,
        tools: false,
        streaming: true,
        jsonMode: false,
        vision: false,
        embeddings: false,
        modelsEndpoint: true,
      }),
  })
  .strict();

export const AUTONOMY_LEVELS = [
  { level: 1, name: "Observe", description: "Read-only access to connected services." },
  { level: 2, name: "Recommend", description: "Analyse and propose actions; writes are blocked." },
  { level: 3, name: "Prepare", description: "Prepare writes and side effects; humans approve." },
  { level: 4, name: "Autonomous", description: "Execute allowlisted actions within policy." },
] as const;

export function emptyAgentDefinition(): AgentDefinition {
  return {
    apiVersion: "orbit.io/v1",
    kind: "Agent",
    metadata: { name: "", description: "", labels: {} },
    spec: {
      runtime: { type: "mock", executionMode: "on_demand", config: {} },
      model: { management: "runtime", params: {} },
      instructions: { system: "", constraints: [] },
      connectors: [],
      triggers: [],
      autonomy: { level: 2, approvalMode: "auto" },
      resources: { timeoutSeconds: 300, maxConcurrentExecutions: 1 },
      memory: { enabled: false, strategy: "none" },
    },
  };
}
