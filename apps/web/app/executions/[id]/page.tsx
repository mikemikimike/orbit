"use client";


export const dynamic = 'force-dynamic';
export const dynamicParams = true;

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
  RotateCw,
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
import { request } from "@/lib/http";
import { copyText } from "@/lib/browser";
import {
  queryKeys,
  unwrap,
  useApprovals,
  useExecution,
  useExecutionEvents,
  useWorkers,
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
import { capabilityLabel, eventKindLabel, toolLabel } from "@/lib/presentation";
import type { ExecutionEventRecord } from "@/lib/types";

export default function ExecutionDetailPage() {
  const params = useParams<{ id: string }>();
  const executionId = params.id;
  const queryClient = useQueryClient();
  const executionQuery = useExecution(executionId);
  const eventsQuery = useExecutionEvents(executionId);
  const live = useExecutionStream(executionId);
  const workers = useWorkers();
  const pendingApprovals = useApprovals("pending");
  const pendingApproval = pendingApprovals.data?.find(
    (approval) => approval.executionId === executionId,
  );
  const [autoScroll, setAutoScroll] = useState(true);
  const [showVerbose, setShowVerbose] = useState(false);
  const [showRawOutput, setShowRawOutput] = useState(false);
  const [showRawInput, setShowRawInput] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);

  const execution = live.execution ?? executionQuery.data;
  const events = useMemo(() => {
    const merged = new Map<number, ExecutionEventRecord>();
    for (const event of eventsQuery.data?.items ?? []) merged.set(event.seq, event);
    for (const event of live.events) merged.set(event.seq, event);
    return [...merged.values()].sort((a, b) => a.seq - b.seq);
  }, [eventsQuery.data, live.events]);

  const visibleEvents = useMemo(
    () => (showVerbose ? events : events.filter(isImportantEvent)),
    [events, showVerbose],
  );
  const internalToolEvents = useMemo(
    () => events.filter(isInternalToolCall),
    [events],
  );
  const latestInternalTool = internalToolEvents.at(-1);
  const hiddenCount = events.length - visibleEvents.length;

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

  const rerun = useMutation({
    mutationFn: async () =>
      request<{ id: string }>(`/api/v1/executions/${executionId}/rerun`, {
        method: "POST",
      }),
    onSuccess: (result) => {
      toast.success("Execution queued again");
      window.location.assign(`/executions/${result.id}`);
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
              {(() => {
                const input = execution.input as { name?: string };
                return input.name || execution.agentName;
              })()}
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
              disabled={!terminal}
              loading={rerun.isPending}
              onClick={() => rerun.mutate()}
            >
              <RotateCw /> Rerun
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

      {pendingApproval ? (
        <Card className="border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10">
          <CardContent className="flex flex-wrap items-center justify-between gap-3 p-4">
            <div>
              <p className="text-sm font-medium">Waiting for approval</p>
              <p className="text-[11px] text-muted-foreground">
                {String(
                  (pendingApproval.requestedAction as { capability?: string })?.capability ??
                    "write action",
                )}{" "}
                · this execution is paused until a human decides.
              </p>
            </div>
            <Button asChild size="sm">
              <Link href="/approvals">Review approval</Link>
            </Button>
          </CardContent>
        </Card>
      ) : null}

      {execution.error ? (
        <Card className="border-destructive/40 bg-destructive/5">
          <CardContent className="space-y-2 p-4">
            <p className="flex items-center gap-2 text-sm font-medium text-destructive">
              <CircleAlert className="h-4 w-4" />
              Execution {execution.status.replace("_", " ")}
            </p>
            <p className="text-sm text-destructive/90">
              {String((execution.error as { message?: string }).message ?? "Unknown error")}
            </p>
            {execution.status === "timed_out" ? (
              <p className="text-[11px] text-muted-foreground">
                The harness exceeded the agent timeout of {execution.timeoutSeconds}s.
                Raise the agent resource timeout or reduce the task scope.
              </p>
            ) : null}
          </CardContent>
        </Card>
      ) : null}

      {execution.status === "queued" ? (
        <Card className="border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10">
          <CardContent className="space-y-2 p-4 text-sm">
            <p className="font-medium">Waiting for a worker to claim this execution</p>
            <p className="text-muted-foreground">
              Harness <span className="font-mono">{execution.harnessKind}</span> · placement{" "}
              <span className="font-mono">{execution.placement ?? "auto"}</span>.
              {(() => {
                const liveWorkers = (workers.data ?? []).filter((worker) => worker.live);
                const matching = liveWorkers.filter(
                  (worker) =>
                    worker.harnesses.includes(execution.harnessKind) &&
                    (execution.placement === "auto" ||
                      worker.target === execution.placement),
                );
                if (workers.isLoading) return " Checking workers…";
                if (matching.length > 0)
                  return ` ${matching.length} matching worker(s) online; waiting for a free slot.`;
                if (liveWorkers.length === 0)
                  return " No live workers found. Start a worker for this harness.";
                return ` No live worker matches this harness/placement. Online: ${liveWorkers
                  .map((worker) => `${worker.target}:${worker.harnesses.join("+")}`)
                  .join(", ")}.`;
              })()}
            </p>
            <p className="text-[11px] text-muted-foreground">
              Local Compose only runs a mock worker. Start OpenCode with{" "}
              <code className="font-mono">make worker-opencode</code> or{" "}
              <code className="font-mono">make worker-docker</code>, or deploy the
              Kubernetes OpenCode worker.
            </p>
          </CardContent>
        </Card>
      ) : null}

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <InfoTile label="Trigger" value={titleCase(execution.triggerSource)} icon={<Cpu className="h-3.5 w-3.5" />} />
        <InfoTile label="Harness" value={titleCase(execution.harnessKind)} icon={<Terminal className="h-3.5 w-3.5" />} />
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

      <div className="grid gap-4 lg:grid-cols-3">
        <Card className="min-w-0 lg:col-span-3">
          <CardHeader className="flex-row items-center justify-between pb-3">
            <CardTitle className="flex items-center gap-2">
              <Terminal className="h-4 w-4 text-primary" /> Harness timeline
            </CardTitle>
            <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
              <span>{visibleEvents.length} events</span>
              {events.at(-1) ? (
                <span>last activity {relativeTime(events.at(-1)!.createdAt)}</span>
              ) : null}
              {hiddenCount > 0 ? (
                <button
                  type="button"
                  className="rounded border px-1.5 py-0.5 text-[10px] font-medium hover:bg-muted"
                  onClick={() => setShowVerbose((value) => !value)}
                >
                  {showVerbose ? "Hide verbose" : `${hiddenCount} hidden · show verbose`}
                </button>
              ) : null}
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
              className="max-h-[640px] min-h-[220px] overflow-y-auto px-4 pb-4"
            >
              {visibleEvents.length === 0 ? (
                <p className="py-10 text-center text-sm text-muted-foreground">
                  {events.length > 0
                    ? `${events.length} verbose event(s) hidden.`
                    : active
                      ? "Waiting for harness events…"
                      : "No events were recorded."}
                </p>
              ) : (
                visibleEvents.map((event) => <EventLine key={event.seq} event={event} />)
              )}
              {internalToolEvents.length > 0 ? (
                <div className="mt-3 flex flex-wrap items-center gap-2 rounded-lg bg-muted/40 px-3 py-2 text-[11px] text-muted-foreground">
                  <Terminal className="h-3.5 w-3.5 text-primary" />
                  <span>
                    Harness activity:{" "}
                    <span className="font-medium text-foreground">
                      {latestInternalTool
                        ? eventTitle(latestInternalTool, null, toolName(latestInternalTool), toolInput(latestInternalTool))
                        : "working"}
                    </span>{" "}
                    · {internalToolEvents.length} internal call(s) ·{" "}
                    {latestInternalTool ? relativeTime(latestInternalTool.createdAt) : ""}
                  </span>
                  <button
                    type="button"
                    className="ml-auto rounded border px-1.5 py-0.5 text-[10px] font-medium hover:bg-muted"
                    onClick={() => setShowVerbose(true)}
                  >
                    show
                  </button>
                </div>
              ) : null}
              {active ? (
                <p className="pt-2 text-[11px] text-muted-foreground">
                  <span className="mr-1 inline-block h-1.5 w-1.5 animate-pulse-soft rounded-full bg-primary align-middle" />
                  streaming if worker is running
                </p>
              ) : null}
              {hiddenCount > 0 ? (
                <p className="pt-2 text-[11px] text-muted-foreground">
                  Reasoning, internal steps, usage metrics and generic tool calls are hidden
                  by default. Use “show verbose” for the raw harness stream.
                </p>
              ) : null}
            </div>
          </CardContent>
        </Card>

      </div>

      <Card className="min-w-0 overflow-hidden">
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
        <CardContent className="min-w-0 space-y-2">
          {execution.output ? (
            <>
              <div className="flex items-center justify-between">
                <p className="text-[11px] text-muted-foreground">
                  {countFields(execution.output)} field(s) · {JSON.stringify(execution.output).length} chars
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setShowRawOutput((v) => !v)}
                >
                  {showRawOutput ? "Show rendered" : "Show raw JSON"}
                </Button>
              </div>
              {showRawOutput ? (
                <pre className="max-h-[600px] w-full max-w-full overflow-auto rounded-lg bg-muted/50 p-3 text-[11px] leading-relaxed">
                  {JSON.stringify(execution.output, null, 2)}
                </pre>
              ) : (
                <div className="max-h-[600px] w-full max-w-full overflow-auto break-words rounded-lg bg-muted/50 p-4 text-sm leading-relaxed whitespace-pre-wrap">
                  {renderOutput(execution.output)}
                </div>
              )}
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              {active ? "Output will appear when the harness completes." : "No output."}
            </p>
          )}
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="space-y-4 lg:col-span-2">
          {pendingApproval ? (
        <Card className="border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10">
          <CardContent className="flex flex-wrap items-center justify-between gap-3 p-4">
            <div>
              <p className="text-sm font-medium">Waiting for approval</p>
              <p className="text-[11px] text-muted-foreground">
                {String(
                  (pendingApproval.requestedAction as { capability?: string })?.capability ??
                    "write action",
                )}{" "}
                · this execution is paused until a human decides.
              </p>
            </div>
            <Button asChild size="sm">
              <Link href="/approvals">Review approval</Link>
            </Button>
          </CardContent>
        </Card>
      ) : null}

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
            <CardHeader className="pb-2">
              <CardTitle>Input</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-[11px] text-muted-foreground">
                  {countFields(execution.input)} field(s) · {JSON.stringify(execution.input).length} chars
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setShowRawInput((v) => !v)}
                >
                  {showRawInput ? "Show rendered" : "Show raw JSON"}
                </Button>
              </div>
              {showRawInput ? (
                <pre className="max-h-[600px] overflow-auto rounded-lg bg-muted/50 p-3 text-[11px] leading-relaxed">
                  {JSON.stringify(execution.input, null, 2)}
                </pre>
              ) : (
                <div className="max-h-[600px] overflow-auto rounded-lg bg-muted/50 p-4 text-sm leading-relaxed whitespace-pre-wrap">
                  {(() => {
                    const input = execution.input as { task?: string; instructions?: string };
                    return input.task || input.instructions || renderOutput(execution.input);
                  })()}
                </div>
              )}
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

function isImportantEvent(event: ExecutionEventRecord): boolean {
  if (event.kind === "error" || event.kind === "status") return true;
  if (event.kind === "log") return event.level === "warn" || event.level === "error";
  if (event.kind === "tool_call") {
    const capability =
      event.data && typeof event.data === "object"
        ? (event.data as { capability?: string }).capability
        : undefined;
    return (
      event.level === "error" ||
      event.level === "warn" ||
      Boolean(capability) ||
      /approval/i.test(event.message)
    );
  }
  // output deltas, metrics and debug logs are verbose-only
  return false;
}

function isInternalToolCall(event: ExecutionEventRecord): boolean {
  if (event.kind !== "tool_call") return false;
  const data =
    event.data && typeof event.data === "object"
      ? (event.data as Record<string, unknown>)
      : {};
  return !data.capability && Boolean(data.tool);
}

function toolName(event: ExecutionEventRecord): string | null {
  const data =
    event.data && typeof event.data === "object"
      ? (event.data as Record<string, unknown>)
      : {};
  return typeof data.tool === "string" ? data.tool : null;
}

function toolInput(event: ExecutionEventRecord): Record<string, unknown> {
  const data =
    event.data && typeof event.data === "object"
      ? (event.data as Record<string, unknown>)
      : {};
  return data.input && typeof data.input === "object"
    ? (data.input as Record<string, unknown>)
    : {};
}

function EventLine({ event }: { event: ExecutionEventRecord }) {
  const data =
    event.data && typeof event.data === "object"
      ? (event.data as Record<string, unknown>)
      : {};
  const capability = typeof data.capability === "string" ? data.capability : null;
  const tool = typeof data.tool === "string" ? data.tool : null;
  const input =
    data.input && typeof data.input === "object"
      ? (data.input as Record<string, unknown>)
      : data.arguments && typeof data.arguments === "object"
        ? (data.arguments as Record<string, unknown>)
        : {};
  const title = eventTitle(event, capability, tool, input);
  const detail = eventDetail(capability, tool, input, data);
  const icon =
    capability ? (
      <Wrench className="h-3.5 w-3.5 text-primary" />
    ) : event.kind === "tool_call" ? (
      <Terminal className="h-3.5 w-3.5" />
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
    <div
      className={`log-line flex items-start gap-2.5 border-l-2 pl-3 ${
        event.level === "error"
          ? "border-destructive/50"
          : event.level === "warn"
            ? "border-[hsl(var(--warning))]/50"
            : "border-transparent"
      }`}
    >
      <span className="w-16 shrink-0 pt-0.5 text-right font-mono text-[10px] text-muted-foreground">
        {formatClock(event.createdAt)}
      </span>
      <span className="mt-0.5 shrink-0 text-muted-foreground">{icon}</span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="rounded bg-muted px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wider text-muted-foreground">
            {eventKindLabel(event.kind)}
          </span>
          <p className={`whitespace-pre-wrap break-words ${levelClass}`}>{title}</p>
        </div>
        {detail ? (
          <p className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">
            {detail}
          </p>
        ) : null}
        {event.data && Object.keys(data).length > 0 ? (
          <details className="mt-1">
            <summary className="cursor-pointer text-[10px] text-muted-foreground hover:text-foreground">
              raw event
            </summary>
            <pre className="mt-1 max-h-64 overflow-auto whitespace-pre-wrap rounded bg-muted/40 p-2 text-[10px] text-muted-foreground">
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

function eventTitle(
  event: ExecutionEventRecord,
  capability: string | null,
  tool: string | null,
  input: Record<string, unknown>,
): string {
  if (capability) {
    const target = [input.owner, input.repo, input.project].filter(Boolean).join("/");
    return `${capabilityLabel(capability)}${target ? ` · ${target}` : ""}`;
  }
  if (tool) {
    return `${toolLabel(tool)}${toolDetail(tool, input) ? ` · ${toolDetail(tool, input)}` : ""}`;
  }
  return event.message;
}

function eventDetail(
  capability: string | null,
  tool: string | null,
  input: Record<string, unknown>,
  data: Record<string, unknown>,
): string | null {
  if (capability) {
    const entries = Object.entries(input)
      .filter(([key]) => !["owner", "repo"].includes(key))
      .slice(0, 3)
      .map(([key, value]) => `${key}=${String(value).slice(0, 80)}`);
    return entries.join(" · ") || null;
  }
  if (tool) return toolDetail(tool, input);
  const output = typeof data.output === "string" ? data.output : null;
  if (output) return output.split("\n")[0]?.slice(0, 160) ?? null;
  return null;
}

function toolDetail(tool: string, input: Record<string, unknown>): string | null {
  const value = input.path ?? input.command ?? input.pattern ?? input.query;
  if (typeof value !== "string") return null;
  return value.replace(/\s+/g, " ").slice(0, 140);
}

function countFields(value: unknown): number {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return Object.keys(value as Record<string, unknown>).length;
  }
  if (Array.isArray(value)) return value.length;
  return 1;
}

function renderOutput(value: unknown): string {
  if (typeof value === "string") return value;
  if (value === null || value === undefined) return "—";
  if (typeof value !== "object") return String(value);

  // Try common text fields.
  const obj = value as Record<string, unknown>;
  for (const key of ["text", "output", "result", "message", "summary", "content", "answer", "response"]) {
    if (typeof obj[key] === "string" && (obj[key] as string).trim()) {
      return obj[key] as string;
    }
  }
  // Nested output/result.
  if (obj.output && typeof obj.output === "object") {
    const nested = renderOutput(obj.output);
    if (nested !== JSON.stringify(obj.output)) return nested;
  }
  // Array of items — join text fields.
  if (Array.isArray(value)) {
    return value
      .map((item) => (typeof item === "string" ? item : renderOutput(item)))
      .join("\n\n");
  }
  // Fallback: pretty-print JSON.
  return JSON.stringify(value, null, 2);
}

function formatClock(value: string): string {
  return new Date(value).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}
