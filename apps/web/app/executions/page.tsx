"use client";

export const dynamic = 'force-dynamic';

import { useMemo, useState } from "react";
import Link from "next/link";
import { Activity, Filter, Users } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { ExecutionRow } from "@/components/executions/execution-row";
import { useExecutions } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { cn } from "@/lib/utils";

const STATUSES = [
  { value: "", label: "All" },
  { value: "running", label: "Running" },
  { value: "queued", label: "Queued" },
  { value: "succeeded", label: "Succeeded" },
  { value: "failed", label: "Failed" },
  { value: "cancelled", label: "Cancelled" },
];

export default function ExecutionsPage() {
  const [status, setStatus] = useState("");
  const [archived, setArchived] = useState<"active" | "archived" | "all">("active");
  const executions = useExecutions({
    status: status || undefined,
    limit: 50,
    archived: archived === "all" ? undefined : archived === "archived",
  });
  const groups = useMemo(() => {
    const items = executions.data?.items ?? [];
    const grouped = new Map<string, typeof items>();
    for (const execution of items) {
      const teamRunId = (execution as typeof execution & { teamRunId?: string | null }).teamRunId;
      const key = teamRunId ?? "standalone";
      const current = grouped.get(key) ?? [];
      current.push(execution);
      grouped.set(key, current);
    }
    return Array.from(grouped.entries());
  }, [executions.data?.items]);

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Observability"
        title="Executions"
        description="Every agent invocation: input, output, status, harness events and usage."
      />

      <div className="flex items-center gap-2">
        <Filter className="hidden h-4 w-4 text-muted-foreground sm:block" />
        <div className="no-scrollbar -mx-1 flex gap-1 overflow-x-auto px-1 py-0.5">
          {STATUSES.map((item) => (
            <Button
              key={item.value}
              size="sm"
              variant={status === item.value ? "default" : "outline"}
              onClick={() => setStatus(item.value)}
            >
              {item.label}
            </Button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">View:</span>
          {(["active", "archived", "all"] as const).map((value) => (
            <Button
              key={value}
              size="sm"
              variant={archived === value ? "default" : "outline"}
              onClick={() => setArchived(value)}
            >
              {value === "active" ? "Active" : value === "archived" ? "Archived" : "All"}
            </Button>
          ))}
        </div>
        {executions.data ? (
          <span className="ml-auto hidden text-xs text-muted-foreground sm:block">
            {executions.data.total} total
          </span>
        ) : null}
      </div>

      <Card className={cn("overflow-hidden")}>
        {executions.isLoading ? (
          <div className="space-y-2 p-4">
            {Array.from({ length: 6 }).map((_, index) => (
              <Skeleton key={index} className="h-14" />
            ))}
          </div>
        ) : executions.isError ? (
          <p className="p-4 text-sm text-destructive">{errorMessage(executions.error)}</p>
        ) : executions.data?.items.length ? (
          groups.map(([key, items]) =>
            key === "standalone" ? (
              items.map((execution) => (
                <ExecutionRow key={execution.id} execution={execution} />
              ))
            ) : (
              <div key={key}>
                <div className="flex items-center gap-2 border-b bg-muted/40 px-4 py-2">
                  <Users className="h-3.5 w-3.5 text-primary" />
                  <Link
                    href={`/agent-teams?run=${key}`}
                    className="text-xs font-medium text-primary hover:underline"
                  >
                    Agent Team run
                  </Link>
                  <span className="font-mono text-[10px] text-muted-foreground">
                    {key.slice(0, 8)}
                  </span>
                  <span className="ml-auto text-[10px] text-muted-foreground">
                    {items.length} step{items.length === 1 ? "" : "s"}
                  </span>
                </div>
                {items.map((execution) => (
                  <ExecutionRow key={execution.id} execution={execution} />
                ))}
              </div>
            ),
          )
        ) : (
          <EmptyState
            icon={<Activity className="h-6 w-6" />}
            title="No executions"
            description="Run an agent manually or configure a trigger. Executions appear here with live logs."
            action={
              <Button asChild size="sm">
                <Link href="/agents">Open agents</Link>
              </Button>
            }
            className="border-0"
          />
        )}
      </Card>
    </div>
  );
}
