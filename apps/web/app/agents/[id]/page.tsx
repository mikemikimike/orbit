"use client";

export const dynamic = 'force-dynamic';

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Cable,
  Clock3,
  Download,
  History,
  Pause,
  Pencil,
  Play,
  ShieldCheck,
  Trash2,
  Zap,
} from "lucide-react";
import { toast } from "sonner";

import { RunAgentDialog } from "@/components/agents/run-agent-dialog";
import { ShareResourceDialog } from "@/components/share-resource-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { AgentStatusBadge, ExecutionStatusBadge } from "@/components/ui/status-badge";
import { api } from "@/lib/api";
import { agentSpecSchema } from "@orbit/contracts";
import { queryKeys, unwrap, useAgent, useAgentVersions, useExecutions } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDateTime, formatDuration, relativeTime, titleCase } from "@/lib/utils";
import { autonomyName, cronDescription } from "@/lib/presentation";

export default function AgentDetailPage() {
  const params = useParams<{ id: string }>();
  const agentId = params.id;
  const router = useRouter();
  const queryClient = useQueryClient();
  const agent = useAgent(agentId);
  const versions = useAgentVersions(agentId);
  const executions = useExecutions({ agentId, limit: 20 });
  const [confirmDelete, setConfirmDelete] = useState(false);

  const lifecycle = useMutation({
    mutationFn: async (action: "activate" | "pause") => {
      if (action === "activate") {
        return unwrap(
          await api.POST("/api/v1/agents/{agent_id}/activate", {
            params: { path: { agent_id: agentId } },
          }),
        );
      }
      return unwrap(
        await api.POST("/api/v1/agents/{agent_id}/pause", {
          params: { path: { agent_id: agentId } },
        }),
      );
    },
    onSuccess: () => {
      toast.success("Agent updated");
      void queryClient.invalidateQueries({ queryKey: queryKeys.agent(agentId) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents() });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const archive = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.DELETE("/api/v1/agents/{agent_id}", {
          params: { path: { agent_id: agentId }, query: { hard: false } },
        }),
      ),
    onSuccess: () => {
      toast.success("Agent archived");
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents() });
      router.push("/agents");
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const hardDelete = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.DELETE("/api/v1/agents/{agent_id}", {
          params: { path: { agent_id: agentId }, query: { hard: true } },
        }),
      ),
    onSuccess: () => {
      toast.success("Agent deleted permanently");
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents() });
      router.push("/agents");
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  if (agent.isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-40 rounded-xl" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  if (agent.isError || !agent.data) {
    return (
      <EmptyState
        title="Agent not found"
        description={agent.error ? errorMessage(agent.error) : "This agent does not exist."}
        action={
          <Button asChild>
            <Link href="/agents">Back to agents</Link>
          </Button>
        }
      />
    );
  }

  const data = agent.data;
  const parsedSpec = agentSpecSchema.safeParse(data.spec);
  if (!parsedSpec.success) {
    return (
      <div className="space-y-3">
        <Button asChild variant="ghost" size="sm" className="-ml-2">
          <Link href="/agents"><ArrowLeft /> Agents</Link>
        </Button>
        <EmptyState
          title="This agent's configuration cannot be rendered"
          description={parsedSpec.error.issues.map((issue) => `${issue.path.join(".")}: ${issue.message}`).join(" · ")}
          action={<Button asChild><Link href={`/agents/${data.id}/edit`}>Open Agent Studio</Link></Button>}
        />
      </div>
    );
  }
  const spec = parsedSpec.data;
  const canRun = data.status !== "archived";
  const autonomy = spec.autonomy;

  return (
    <div className="space-y-5 animate-fade-in">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2">
          <Link href="/agents">
            <ArrowLeft /> Agents
          </Link>
        </Button>
        <PageHeader
          eyebrow={`Version ${data.currentVersion} · updated ${relativeTime(data.updatedAt)}`}
          title={data.name}
          description={data.description || "No description yet."}
          actions={
            <>
              <ShareResourceDialog
                resource="agents"
                resourceId={data.id}
                resourceName={data.name}
              />
              <Button asChild variant="outline">
                <Link href={`/agents/${data.id}/edit`}>
                  <Pencil /> Edit
                </Link>
              </Button>
              {data.status === "active" ? (
                <Button
                  variant="outline"
                  onClick={() => lifecycle.mutate("pause")}
                  loading={lifecycle.isPending}
                >
                  <Pause /> Pause
                </Button>
              ) : data.status !== "archived" ? (
                <Button
                  variant="outline"
                  onClick={() => lifecycle.mutate("activate")}
                  loading={lifecycle.isPending}
                >
                  <Zap /> Activate
                </Button>
              ) : (
                <Button
                  variant="outline"
                  onClick={() => lifecycle.mutate("activate")}
                  loading={lifecycle.isPending}
                >
                  <Zap /> Restore
                </Button>
              )}
              <RunAgentDialog
                agentId={data.id}
                agentName={data.name}
                disabled={!canRun}
                defaultTask={
                  (data.spec as unknown as { runDefaults?: { task?: string } }).runDefaults
                    ?.task
                }
              />
            </>
          }
        />
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <AgentStatusBadge status={data.status} />
        <span className="rounded bg-muted px-2 py-0.5 text-[11px] font-medium">
          {titleCase(spec.harness.type)}
        </span>
        <span className="inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-[11px]">
          <ShieldCheck className="h-3 w-3" /> {autonomyName(autonomy.level)}
        </span>
        <span className="inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-[11px]">
          <Cable className="h-3 w-3" /> {spec.connectors.length} connectors
        </span>
        <span className="inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-[11px]">
          <Clock3 className="h-3 w-3" /> {spec.resources.timeoutSeconds}s timeout
        </span>
        <Button asChild variant="ghost" size="sm" className="ml-auto">
          <a href={`${process.env.NEXT_PUBLIC_ORBIT_API_URL ?? "http://localhost:8080"}/api/v1/agents/${data.id}/export`} target="_blank" rel="noreferrer">
            <Download /> YAML
          </a>
        </Button>
      </div>

      <Tabs defaultValue="overview">
        <TabsList className="w-full justify-start overflow-x-auto sm:w-auto">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="executions">Executions</TabsTrigger>
          <TabsTrigger value="versions">Versions</TabsTrigger>
          <TabsTrigger value="configuration">Configuration</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          <div className="grid gap-4 lg:grid-cols-3">
            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>Instructions</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <pre className="max-h-80 overflow-auto whitespace-pre-wrap rounded-lg bg-muted/50 p-3 text-[12px] leading-relaxed">
                  {spec.instructions.system || "No instructions configured."}
                </pre>
                {spec.instructions.constraints.length > 0 ? (
                  <ul className="list-inside list-disc space-y-1 text-xs text-muted-foreground">
                    {spec.instructions.constraints.map((constraint) => (
                      <li key={constraint}>{constraint}</li>
                    ))}
                  </ul>
                ) : null}
              </CardContent>
            </Card>

            <div className="space-y-4">
              <Card>
                <CardHeader>
                  <CardTitle>Harness</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  <Row label="Harness" value={titleCase(spec.harness.type)} />
                  <Row label="Mode" value={titleCase(spec.harness.executionMode)} />
                  <Row label="Placement" value={titleCase(spec.harness.placement)} />
                  <Row
                    label="Model management"
                    value={titleCase(spec.model.management)}
                  />
                  <Row label="Skills" value={spec.skills.length ? spec.skills.join(", ") : "none"} />
                  <Row label="Provider" value={spec.model.provider ?? "harness default"} />
                  <Row label="Model" value={spec.model.model ?? "harness default"} />
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Autonomy &amp; limits</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  <Row label="Level" value={`L${autonomy.level}`} />
                  <Row label="Approval" value={titleCase(autonomy.approvalMode)} />
                  <Row
                    label="Quota"
                    value={
                      autonomy.maxExecutionsPerHour
                        ? `${autonomy.maxExecutionsPerHour}/hour`
                        : "unlimited"
                    }
                  />
                  <Row label="Concurrency" value={String(spec.resources.maxConcurrentExecutions)} />
                </CardContent>
              </Card>
            </div>
          </div>

          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>Triggers</CardTitle>
              <span className="text-xs text-muted-foreground">{data.triggers.length} configured</span>
            </CardHeader>
            <CardContent>
              {data.triggers.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Manual execution only. Add schedules or events in Agent Studio.
                </p>
              ) : (
                <div className="space-y-2">
                  {data.triggers.map((trigger) => (
                    <div
                      key={trigger.id}
                      className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2 text-sm"
                    >
                      <div className="flex items-center gap-2">
                        <span className="rounded bg-muted px-1.5 py-0.5 text-[11px] font-medium">
                          {titleCase(trigger.kind)}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {trigger.kind === "schedule"
                            ? cronDescription(
                                String((trigger.config as { cron?: string }).cron ?? ""),
                              )
                            : trigger.kind === "event"
                              ? String((trigger.config as { subject?: string }).subject ?? "")
                              : ""}
                        </span>
                      </div>
                      <span className="text-[11px] text-muted-foreground">
                        {trigger.enabled ? "enabled" : "disabled"}
                        {trigger.nextRunAt ? ` · next ${formatDateTime(trigger.nextRunAt)}` : ""}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          <Card className="border-destructive/30">
            <CardContent className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="text-sm font-medium">
                  {data.status === "archived" ? "Delete permanently" : "Archive agent"}
                </p>
                <p className="text-xs text-muted-foreground">
                  {data.status === "archived"
                    ? "This removes the agent and its execution history. This cannot be undone."
                    : "Archiving preserves execution history. Hard deletion removes everything and cannot be undone."}
                </p>
              </div>
              <div className="flex gap-2">
                {confirmDelete ? (
                  <>
                    <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
                      Cancel
                    </Button>
                    <Button
                      variant="destructive"
                      onClick={() =>
                        data.status === "archived" ? hardDelete.mutate() : archive.mutate()
                      }
                      loading={archive.isPending || hardDelete.isPending}
                    >
                      <Trash2 />{" "}
                      {data.status === "archived" ? "Confirm delete" : "Confirm archive"}
                    </Button>
                  </>
                ) : (
                  <Button variant="outline" onClick={() => setConfirmDelete(true)}>
                    <Trash2 />{" "}
                    {data.status === "archived" ? "Delete permanently" : "Archive"}
                  </Button>
                )}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="executions">
          <Card className="overflow-hidden">
            {executions.isLoading ? (
              <div className="space-y-2 p-4">
                {Array.from({ length: 3 }).map((_, index) => (
                  <Skeleton key={index} className="h-12" />
                ))}
              </div>
            ) : executions.data?.items.length ? (
              executions.data.items.map((execution) => (
                <Link
                  key={execution.id}
                  href={`/executions/${execution.id}`}
                  className="flex items-center gap-3 border-b px-4 py-3 last:border-b-0 hover:bg-muted/40"
                >
                  <ExecutionStatusBadge status={execution.status} />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm">{titleCase(execution.triggerSource)}</p>
                    <p className="text-[11px] text-muted-foreground">
                      {relativeTime(execution.createdAt)} ·{" "}
                      {formatDuration(execution.startedAt, execution.finishedAt)}
                    </p>
                  </div>
                </Link>
              ))
            ) : (
              <EmptyState
                icon={<Play className="h-6 w-6" />}
                title="No executions"
                description="Run this agent to see its execution history and logs."
                className="border-0"
              />
            )}
          </Card>
        </TabsContent>

        <TabsContent value="versions">
          <Card className="overflow-hidden">
            {(versions.data ?? []).map((version) => (
              <div
                key={version.id}
                className="flex flex-wrap items-center justify-between gap-2 border-b px-4 py-3 last:border-b-0"
              >
                <div className="flex items-center gap-2">
                  <History className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">Version {version.version}</span>
                  {version.version === data.currentVersion ? (
                    <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
                      current
                    </span>
                  ) : null}
                </div>
                <div className="text-right text-[11px] text-muted-foreground">
                  <p>{formatDateTime(version.createdAt)}</p>
                  <p className="font-mono">{version.checksum.slice(0, 12)}…</p>
                </div>
              </div>
            ))}
          </Card>
        </TabsContent>

        <TabsContent value="configuration">
          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>Definition (YAML)</CardTitle>
              <span className="text-[11px] text-muted-foreground">portable orbit.io/v1</span>
            </CardHeader>
            <CardContent>
              <pre className="max-h-[520px] overflow-auto rounded-lg bg-muted/60 p-3 text-[11px] leading-relaxed">
                {data.definitionYaml || "# definition unavailable"}
              </pre>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="text-right text-xs font-medium">{value}</span>
    </div>
  );
}
