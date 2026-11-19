"use client";

import Link from "next/link";
import { Check, Cpu, ExternalLink, Minus, Server } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Callout } from "@/components/ui/callout";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Section } from "@/components/ui/section";
import { Skeleton } from "@/components/ui/skeleton";
import { Status } from "@/components/ui/status";
import { ImplementationBadge } from "@/components/ui/status-badge";
import { useMeta, useHarnesses, useWorkers } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { cn, relativeTime, titleCase } from "@/lib/utils";

const STATUS_ORDER = ["available", "scaffolded", "planned"] as const;

const STATUS_SECTION: Record<string, { title: string; description: string }> = {
  available: {
    title: "Available",
    description: "Covered by contract tests and verified against the real harness.",
  },
  scaffolded: {
    title: "Scaffolded",
    description: "Validates configuration and checks the CLI, but does not run tasks yet.",
  },
  planned: {
    title: "Planned",
    description: "Registered so agents can reference them. Execution fails closed until they ship.",
  },
};

function statusRank(status: string) {
  const index = STATUS_ORDER.indexOf(status as (typeof STATUS_ORDER)[number]);
  return index === -1 ? STATUS_ORDER.length : index;
}

export default function HarnessesPage() {
  const harnesses = useHarnesses();
  const meta = useMeta();
  const workers = useWorkers();

  const groups = (() => {
    const byStatus = new Map<string, NonNullable<typeof harnesses.data>>();
    for (const harness of harnesses.data ?? []) {
      const key = harness.implementationStatus;
      byStatus.set(key, [...(byStatus.get(key) ?? []), harness]);
    }
    return [...byStatus.entries()].sort((a, b) => statusRank(a[0]) - statusRank(b[0]));
  })();

  return (
    <div className="space-y-6">
      <PageHeader
        title="Harnesses"
        description="The adapters workers can use to run agents, and what each one can do. Capabilities come from the adapter itself, not from this page."
      />

      <Callout tone="info" title="Implementation status is explicit">
        Only adapters marked available are safe to run agents on. Scaffolded and planned adapters
        can be referenced by agents but will not execute tasks.
      </Callout>

      {harnesses.isLoading ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {Array.from({ length: 4 }).map((_, index) => (
            <Skeleton key={index} className="h-64 rounded-xl" />
          ))}
        </div>
      ) : harnesses.isError ? (
        <Callout tone="danger" title="Harnesses could not be loaded">
          {errorMessage(harnesses.error)}
        </Callout>
      ) : !harnesses.data?.length ? (
        <EmptyState
          icon={<Cpu />}
          title="No harnesses registered"
          description="Harness adapters are registered by the platform. None were reported."
        />
      ) : (
        groups.map(([status, items]) => {
          const section = STATUS_SECTION[status] ?? { title: titleCase(status), description: "" };
          return (
            <Section
              key={status}
              title={section.title}
              count={items.length}
              description={section.description}
            >
              <div className="grid gap-3 sm:grid-cols-2">
                {items.map((harness) => {
                  const capabilities = (harness.descriptor?.capabilities ?? {}) as Record<
                    string,
                    unknown
                  >;
                  const flags = [
                    ["Sessions", capabilities.supportsSessions],
                    ["Resume", capabilities.supportsResume],
                    ["Streaming", capabilities.supportsStreaming],
                    ["Cancel", capabilities.supportsCancel],
                    ["Long-running", capabilities.supportsInstances],
                    ["Harness providers", capabilities.harnessManagedProviders],
                    ["Platform providers", capabilities.supportsPlatformProviders],
                    ["Usage reporting", capabilities.supportsUsageReporting],
                  ] as const;
                  const modes = harness.descriptor?.capabilities?.executionModes ?? [];
                  const timeout = harness.descriptor?.capabilities?.maxTimeoutSeconds;
                  const unavailable = harness.implementationStatus !== "available";

                  return (
                    <Card key={harness.kind} className="flex flex-col p-5">
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex min-w-0 items-center gap-3">
                          <span
                            className={cn(
                              "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg",
                              unavailable ? "bg-muted text-muted-foreground" : "bg-primary/10 text-primary",
                            )}
                          >
                            <Cpu className="h-5 w-5" />
                          </span>
                          <div className="min-w-0">
                            <h3 className="truncate text-[15px] font-semibold">{harness.name}</h3>
                            <p className="truncate font-mono text-xs text-muted-foreground">
                              {harness.kind}
                            </p>
                          </div>
                        </div>
                        <ImplementationBadge status={harness.implementationStatus} />
                      </div>

                      <p className="mt-3 text-sm text-muted-foreground">{harness.description}</p>

                      {modes.length || timeout ? (
                        <div className="mt-3 flex flex-wrap gap-1.5">
                          {modes.map((mode) => (
                            <Badge key={mode} variant="secondary">
                              {titleCase(mode)}
                            </Badge>
                          ))}
                          {timeout ? <Badge variant="outline">Timeout up to {timeout} s</Badge> : null}
                        </div>
                      ) : null}

                      <ul
                        aria-label={`${harness.name} capabilities`}
                        className="mt-4 grid grid-cols-1 gap-x-4 gap-y-1.5 border-t pt-4 min-[420px]:grid-cols-2"
                      >
                        {flags.map(([label, enabled]) => (
                          <li
                            key={label}
                            className={cn(
                              "flex items-center gap-2 text-[13px]",
                              enabled ? "text-foreground" : "text-muted-foreground",
                            )}
                          >
                            {enabled ? (
                              <Check className="h-4 w-4 shrink-0 text-success" aria-hidden />
                            ) : (
                              <Minus className="h-4 w-4 shrink-0 opacity-60" aria-hidden />
                            )}
                            <span>
                              {label}
                              <span className="sr-only">{enabled ? ": supported" : ": not supported"}</span>
                            </span>
                          </li>
                        ))}
                      </ul>

                      <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 border-t pt-4 text-[13px] text-muted-foreground">
                        <span>{harness.enabled ? "Enabled" : "Disabled"}</span>
                        <span>Health: {titleCase(harness.health.status)}</span>
                        {harness.descriptor?.docsUrl ? (
                          <a
                            href={harness.descriptor.docsUrl}
                            target="_blank"
                            rel="noreferrer"
                            className="ml-auto inline-flex items-center gap-1 font-medium text-primary hover:underline"
                          >
                            Documentation <ExternalLink className="h-3.5 w-3.5" aria-hidden />
                            <span className="sr-only">(opens in a new tab)</span>
                          </a>
                        ) : null}
                      </div>
                    </Card>
                  );
                })}
              </div>
            </Section>
          );
        })
      )}

      <Section
        title="Workers reporting in"
        count={workers.data?.length}
        description="Which harnesses each connected worker offers."
        actions={
          <Link href="/workers" className="text-[13px] font-medium text-primary hover:underline">
            Manage workers
          </Link>
        }
      >
        {workers.isError ? (
          <Callout tone="danger" title="Workers could not be loaded">
            {errorMessage(workers.error)}
          </Callout>
        ) : workers.data?.length ? (
          <Card className="overflow-hidden">
            <ul>
              {workers.data.map((worker) => (
                <li
                  key={worker.workerId}
                  className="flex flex-col gap-1.5 border-b px-4 py-3 last:border-b-0 sm:flex-row sm:items-center sm:justify-between sm:gap-4 sm:px-5"
                >
                  <div className="min-w-0">
                    <p className="truncate font-mono text-[13px]" title={worker.workerId}>
                      {worker.workerId}
                    </p>
                    <p className="mt-0.5 text-[13px] text-muted-foreground">
                      {worker.harnesses.length
                        ? worker.harnesses.map((item) => titleCase(item)).join(", ")
                        : "No harnesses"}{" "}
                      on {worker.target}
                      {worker.hostname ? `, host ${worker.hostname}` : ""}
                    </p>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="tabular text-xs text-muted-foreground">
                      {relativeTime(worker.lastHeartbeatAt)}
                    </span>
                    <Status
                      meta={
                        worker.live
                          ? { label: "Live", tone: "success", shape: "solid" }
                          : { label: "Stale", tone: "warning", shape: "alert" }
                      }
                    />
                  </div>
                </li>
              ))}
            </ul>
          </Card>
        ) : (
          <EmptyState
            icon={<Server />}
            title="No workers have reported yet"
            description="Harness health appears here once a worker sends its first heartbeat."
          />
        )}
      </Section>

      {meta.data ? (
        <p className="text-[13px] text-muted-foreground">
          Event transport is {meta.data.eventsDriver} in the {meta.data.environment} environment.
          Harness health arrives with worker heartbeats.
        </p>
      ) : null}
    </div>
  );
}
