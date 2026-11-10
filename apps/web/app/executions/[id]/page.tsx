"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Ban,
  CheckCircle2,
  CircleAlert,
  Clock3,
  Copy,
  Cpu,
  Radio,
  Terminal,
  Wrench,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { Skeleton } from "@/components/ui/skeleton";
import { ExecutionStatusBadge } from "@/components/ui/status-badge";
import { api } from "@/lib/api";
import { copyText } from "@/lib/browser";
import {
  queryKeys,
  unwrap,
  useExecution,
  useExecutionEvents,
} from "@/lib/queries";
import { useExecutionStream } from "@/lib/use-execution-stream";
import { errorMessage } from "@orbit/api-client";
import {
  formatDateTime,
  formatDuration,
  formatTokens,
  isTerminalStatus,
  relativeTime,
  titleCase,
} from "@/lib/utils";
import type { ExecutionEventRecord } from "@/lib/types";

export default function ExecutionDetailPage() {
  const params = useParams<{ id: string }>();
  const executionId = params.id;
  const queryClient = useQueryClient();
  const executionQuery = useExecution(executionId);
  const eventsQuery = useExecutionEvents(executionId);
  const live = useExecutionStream(executionId);
  const [autoScroll, setAutoScroll] = useState(true);
  const logRef = useRef<HTMLDivElement>(null);

  const execution = live.execution ?? executionQuery.data;
  const events = useMemo(() => {
    const merged = new Map<number, ExecutionEventRecord>();
    for (const event of eventsQuery.data?.items ?? []) merged.set(event.seq, event);
    for (const event of live.events) merged.set(event.seq, event);
    return [...merged.values()].sort((a, b) => a.seq - b.seq);
  }, [eventsQuery.data, live.events]);

  const cancel = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST("/api/v1/executions/{execution_id}/cancel", {
          params: { path: { execution_id: executionId } },
        }),
      ),
    onSuccess: () => {
      toast.success("Cancellation requested");
      void queryClient.invalidateQueries({ queryKey: queryKeys.execution(executionId) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.executions() });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  useEffect(() => {
    if (autoScroll && logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [events.length, autoScroll]);

  useEffect(() => {
    if (live.execution && isTerminalStatus(live.execution.status)) {
      void queryClient.invalidateQueries({ queryKey: queryKeys.execution(executionId) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.executions() });
    }
  }, [live.execution?.status, executionId, queryClient, live.execution]);

  if (executionQuery.isLoading && !execution) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-72" />
        <Skeleton className="h-28 rounded-xl" />
        <Skeleton className="h-80 rounded-xl" />
      </div>
    );
  }

  if (!execution) {
    return (
      <EmptyState
        title="Execution not found"
        description={executionQuery.error ? errorMessage(executionQuery.error) : undefined}
        action={
          <Button asChild>
            <Link href="/executions">Back to executions</Link>
          </Button>
        }
      />
    );
  }

  const terminal = isTerminalStatus(execution.status);
  const active = !terminal;
  const task =
    execution.input && typeof execution.input === "object" && "task" in execution.input
      ? String((execution.input as { task?: unknown }).task ?? "")
      : "";

  return (
    <div className="space-y-5 animate-fade-in">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2">
          <Link href="/executions">
            <ArrowLeft /> Executions
          </Link>
        </Button>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <ExecutionStatusBadge status={execution.status} />
              <span className="text-[11px] text-muted-foreground">
                {execution.id}
              </span>
              {live.connected ? (
                <span className="inline-flex items-center gap-1 text-[11px] text-[hsl(var(--success))]">
                  <Radio className="h-3 w-3 animate-pulse-soft" /> live
                </span>
              ) : null}
            </div>
            <h1 className="mt-2 truncate text-2xl font-semibold tracking-tight">
              {execution.agentName}
            </h1>
            <p className="mt-1 line-clamp-2 max-w-3xl text-sm text-muted-foreground">
              {task || "No task recorded"}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button asChild variant="outline">
              <Link href={`/agents/${execution.agentId}`}>Open agent</Link>
            </Button>
            <Button
              variant="outline"
              className="text-destructive"
              disabled={!active}
              loading={cancel.isPending}
              onClick={() => cancel.mutate()}
            >
              <Ban /> Cancel
            </Button>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <InfoTile label="Trigger" value={titleCase(execution.triggerSource)} icon={<Cpu className="h-3.5 w-3.5" />} />
        <InfoTile label="Runtime" value={titleCase(execution.runtimeKind)} icon={<Terminal className="h-3.5 w-3.5" />} />
        <InfoTile
          label="Duration"
          value={formatDuration(execution.startedAt, execution.finishedAt)}
          icon={<Clock3 className="h-3.5 w-3.5" />}
        />
        <InfoTile
          label="Tokens"
          value={`${formatTokens(
            execution.usage && typeof execution.usage === "object"
              ? (execution.usage as { inputTokens?: number }).inputTokens
              : null,
          )} / ${formatTokens(
            execution.usage && typeof execution.usage === "object"
              ? (execution.usage as { outputTokens?: number }).outputTokens
              : null,
          )}`}
          icon={<Cpu className="h-3.5 w-3.5" />}
        />
      </div>

      <div className="grid gap-4 lg:grid-cols-5">
        <Card className="lg:col-span-3">
          <CardHeader className="flex-row items-center justify-between pb-3">
            <CardTitle className="flex items-center gap-2">
              <Terminal className="h-4 w-4 text-primary" /> Runtime timeline
            </CardTitle>
            <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
              <span>{events.length} events</span>
              <label className="inline-flex cursor-pointer items-center gap-1">
                <input
                  type="checkbox"
                  className="accent-[hsl(var(--primary))]"
                  checked={autoScroll}
                  onChange={(event) => setAutoScroll(event.target.checked)}
                />
                follow
              </label>
            </div>
          </CardHeader>
          <CardContent className="p-0">
            <div
              ref={logRef}
              className="max-h-[480px] min-h-[220px] overflow-y-auto px-4 pb-4"
            >
              {events.length === 0 ? (
                <p className="py-10 text-center text-sm text-muted-foreground">
                  {active ? "Waiting for runtime events…" : "No events were recorded."}
                </p>
              ) : (
                events.map((event) => <EventLine key={event.seq} event={event} />)
              )}
              {active ? (
                <p className="pt-2 text-[11px] text-muted-foreground">
                  <span className="mr-1 inline-block h-1.5 w-1.5 animate-pulse-soft rounded-full bg-primary align-middle" />
                  streaming if worker is running
                </p>
              ) : null}
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4 lg:col-span-2">
          {execution.error ? (
            <Card className="border-destructive/40">
              <CardHeader className="pb-2">
                <CardTitle className="flex items-center gap-2 text-destructive">
                  <CircleAlert className="h-4 w-4" /> Error
                </CardTitle>
              </CardHeader>
              <CardContent>
                <pre className="overflow-auto whitespace-pre-wrap text-[11px] leading-relaxed text-destructive">
                  {JSON.stringify(execution.error, null, 2)}
                </pre>
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader className="flex-row items-center justify-between pb-2">
              <CardTitle className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-primary" /> Output
              </CardTitle>
              {execution.output ? (
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Copy output"
                  onClick={async () => {
                    const copied = await copyText(JSON.stringify(execution.output, null, 2));
                    if (copied) toast.success("Output copied");
                    else toast.error("Copy is unavailable in this browser context");
                  }}
                >
                  <Copy />
                </Button>
              ) : null}
            </CardHeader>
            <CardContent>
              {execution.output ? (
                <pre className="max-h-72 overflow-auto rounded-lg bg-muted/50 p-3 text-[11px] leading-relaxed">
                  {JSON.stringify(execution.output, null, 2)}
                </pre>
              ) : (
                <p className="text-sm text-muted-foreground">
                  {active ? "Output will appear when the runtime completes." : "No output."}
                </p>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle>Input</CardTitle>
            </CardHeader>
            <CardContent>
              <pre className="max-h-56 overflow-auto rounded-lg bg-muted/50 p-3 text-[11px] leading-relaxed">
                {JSON.stringify(execution.input, null, 2)}
              </pre>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle>Execution metadata</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1.5 text-xs">
              <MetaRow label="Created" value={formatDateTime(execution.createdAt)} />
              <MetaRow label="Started" value={formatDateTime(execution.startedAt)} />
              <MetaRow label="Finished" value={formatDateTime(execution.finishedAt)} />
              <MetaRow label="Session" value={execution.sessionRef ?? "—"} />
              <MetaRow label="Worker" value={execution.workerId ?? "—"} />
              <MetaRow label="Timeout" value={`${execution.timeoutSeconds}s`} />
              <MetaRow
                label="Idempotency"
                value={execution.idempotencyKey ?? "—"}
              />
            </CardContent>
          </Card>
        </div>
      </div>

      {live.error ? (
        <p className="text-xs text-[hsl(var(--warning))]">Live stream: {live.error}</p>
      ) : null}
    </div>
  );
}

function InfoTile({
  label,
  value,
  icon,
}: {
  label: string;
  value: string;
  icon: React.ReactNode;
}) {
  return (
    <Card>
      <CardContent className="p-3">
        <p className="flex items-center gap-1 text-[10px] uppercase tracking-wider text-muted-foreground">
          {icon} {label}
        </p>
        <p className="mt-1 truncate text-sm font-medium">{value}</p>
      </CardContent>
    </Card>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-[65%] break-all text-right font-mono text-[10px]">{value}</span>
    </div>
  );
}

function EventLine({ event }: { event: ExecutionEventRecord }) {
  const icon =
    event.kind === "tool_call" ? (
      <Wrench className="h-3.5 w-3.5" />
    ) : event.kind === "error" ? (
      <CircleAlert className="h-3.5 w-3.5" />
    ) : event.kind === "output" ? (
      <CheckCircle2 className="h-3.5 w-3.5" />
    ) : (
      <span className="mt-1 block h-1.5 w-1.5 rounded-full bg-muted-foreground/60" />
    );
  const levelClass =
    event.level === "error"
      ? "text-destructive"
      : event.level === "warn"
        ? "text-[hsl(var(--warning))]"
        : "text-foreground";

  return (
    <div className="log-line flex items-start gap-2.5">
      <span className="w-6 shrink-0 pt-0.5 text-right text-[10px] text-muted-foreground">
        {event.seq}
      </span>
      <span className="mt-0.5 shrink-0 text-muted-foreground">{icon}</span>
      <div className="min-w-0 flex-1">
        <p className={`whitespace-pre-wrap break-words ${levelClass}`}>{event.message}</p>
        {event.data && Object.keys(event.data as object).length > 0 ? (
          <details className="mt-1">
            <summary className="cursor-pointer text-[10px] text-muted-foreground hover:text-foreground">
              data
            </summary>
            <pre className="mt-1 overflow-auto whitespace-pre-wrap rounded bg-muted/40 p-2 text-[10px] text-muted-foreground">
              {JSON.stringify(event.data, null, 2)}
            </pre>
          </details>
        ) : null}
      </div>
      <span className="hidden shrink-0 pt-0.5 text-[10px] text-muted-foreground sm:block">
        {relativeTime(event.createdAt)}
      </span>
    </div>
  );
}
