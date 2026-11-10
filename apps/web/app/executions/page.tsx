"use client";

import { useState } from "react";
import Link from "next/link";
import { Activity, Filter } from "lucide-react";

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
  const executions = useExecutions({ status: status || undefined, limit: 50 });

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
          executions.data.items.map((execution) => (
            <ExecutionRow key={execution.id} execution={execution} />
          ))
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
