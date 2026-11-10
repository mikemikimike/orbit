"use client";

import Link from "next/link";
import {
  Activity,
  ArrowRight,
  Bot,
  CircleAlert,
  Plus,
  ShieldCheck,
  Sparkles,
  Timer,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { StatCard } from "@/components/dashboard/stat-card";
import { UsageChart } from "@/components/dashboard/usage-chart";
import { AgentCard } from "@/components/agents/agent-card";
import { ExecutionRow } from "@/components/executions/execution-row";
import { useAgents, useDashboard } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";

export default function DashboardPage() {
  const dashboard = useDashboard();
  const agents = useAgents({ limit: 4 });
  const summary = dashboard.data?.summary;
  const usage = ((dashboard.data?.usage ?? {}) as { days?: Array<{ day: string; executions: number }> }).days ?? [];

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader
        eyebrow="Control plane"
        title="Good to see you"
        description="Monitor your agents, inspect recent runs and jump into configuration."
        actions={
          <Button asChild>
            <Link href="/agents/new">
              <Plus /> New agent
            </Link>
          </Button>
        }
      />

      {summary && summary.pendingApprovals > 0 ? (
        <Link
          href="/approvals"
          className="flex items-center justify-between gap-3 rounded-xl border border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10 px-4 py-3 text-sm transition-colors hover:bg-[hsl(var(--warning))]/15"
        >
          <span className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-[hsl(var(--warning))]" />
            <span className="font-medium">
              {summary.pendingApprovals} action{summary.pendingApprovals === 1 ? "" : "s"} waiting
              for approval
            </span>
          </span>
          <span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
            Review <ArrowRight className="h-3.5 w-3.5" />
          </span>
        </Link>
      ) : null}

      {dashboard.isError ? (
        <Card className="border-destructive/40">
          <CardContent className="flex items-center gap-2 p-4 text-sm text-destructive">
            <CircleAlert className="h-4 w-4" />
            {errorMessage(dashboard.error)}
          </CardContent>
        </Card>
      ) : null}

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        {dashboard.isLoading || !summary ? (
          Array.from({ length: 4 }).map((_, index) => (
            <Skeleton key={index} className="h-[92px] rounded-xl" />
          ))
        ) : (
          <>
            <StatCard
              label="Agents"
              value={summary.totalAgents}
              hint={`${summary.activeAgents} active · ${summary.draftAgents} draft`}
              icon={<Bot className="h-4 w-4" />}
              tone="primary"
            />
            <StatCard
              label="Executions · 24h"
              value={summary.executions24h}
              hint={`${summary.runningExecutions} currently active`}
              icon={<Activity className="h-4 w-4" />}
              tone="success"
            />
            <StatCard
              label="Failures · 24h"
              value={summary.failedExecutions24h}
              hint={summary.failedExecutions24h > 0 ? "Investigate recent runs" : "All clear"}
              icon={<CircleAlert className="h-4 w-4" />}
              tone={summary.failedExecutions24h > 0 ? "danger" : "default"}
            />
            <StatCard
              label="Connections"
              value={summary.totalConnections}
              hint={`${summary.enabledTriggers} enabled triggers`}
              icon={<Sparkles className="h-4 w-4" />}
              tone="warning"
            />
          </>
        )}
      </div>

      <div className="grid gap-4 lg:grid-cols-5">
        <div className="space-y-4 lg:col-span-3">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
              Recent executions
            </h2>
            <Button asChild variant="ghost" size="sm">
              <Link href="/executions">
                View all <ArrowRight />
              </Link>
            </Button>
          </div>
          <Card className="overflow-hidden">
            {dashboard.isLoading ? (
              <div className="space-y-3 p-4">
                {Array.from({ length: 4 }).map((_, index) => (
                  <Skeleton key={index} className="h-12" />
                ))}
              </div>
            ) : dashboard.data?.summary.recentExecutions.length ? (
              dashboard.data.summary.recentExecutions.map((execution) => (
                <ExecutionRow key={execution.id} execution={execution} />
              ))
            ) : (
              <EmptyState
                icon={<Timer className="h-6 w-6" />}
                title="No executions yet"
                description="Run an agent manually or configure a schedule trigger to see activity here."
                action={
                  <Button asChild size="sm">
                    <Link href="/agents">Open an agent</Link>
                  </Button>
                }
                className="border-0"
              />
            )}
          </Card>
        </div>

        <div className="space-y-4 lg:col-span-2">
          <UsageChart days={usage} />
          <Card>
            <CardHeader className="pb-2">
              <CardTitle>Agent pulse</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {agents.isLoading
                ? Array.from({ length: 3 }).map((_, index) => (
                    <Skeleton key={index} className="h-10" />
                  ))
                : agents.data?.items.slice(0, 3).map((agent) => (
                    <Link
                      key={agent.id}
                      href={`/agents/${agent.id}`}
                      className="flex items-center justify-between rounded-lg border px-3 py-2 text-sm transition-colors hover:bg-muted/50"
                    >
                      <span className="truncate font-medium">{agent.name}</span>
                      <span className="ml-2 shrink-0 text-[11px] text-muted-foreground">
                        {agent.status}
                      </span>
                    </Link>
                  ))}
              {!agents.isLoading && !agents.data?.items.length ? (
                <p className="py-2 text-sm text-muted-foreground">No agents yet.</p>
              ) : null}
            </CardContent>
          </Card>
        </div>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Your agents
          </h2>
          <Button asChild variant="ghost" size="sm">
            <Link href="/agents">
              Manage <ArrowRight />
            </Link>
          </Button>
        </div>
        <div className="grid gap-3 sm:grid-cols-2">
          {agents.data?.items.map((agent) => (
            <AgentCard key={agent.id} agent={agent} />
          ))}
        </div>
        {!agents.isLoading && !agents.data?.items.length ? (
          <EmptyState
            title="Create your first agent"
            description="Agents are persistent definitions: instructions, harness, model, connectors, triggers and autonomy."
            action={
              <Button asChild>
                <Link href="/agents/new">
                  <Plus /> Create agent
                </Link>
              </Button>
            }
          />
        ) : null}
      </div>
    </div>
  );
}
