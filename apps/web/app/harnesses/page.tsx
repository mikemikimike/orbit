"use client";

import { Cpu, ExternalLink, Info, ShieldCheck, Sparkles } from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { ImplementationBadge } from "@/components/ui/status-badge";
import { useMeta, useHarnesses, useWorkers } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { titleCase } from "@/lib/utils";

export default function HarnessesPage() {
  const harnesses = useHarnesses();
  const meta = useMeta();
  const workers = useWorkers();

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Execution plane"
        title="Harnesses"
        description="Harness adapters available to the worker. Capabilities are advertised by the adapter, not assumed by the UI."
      />

      <Card className="border-primary/20 bg-primary/5">
        <CardContent className="flex items-start gap-3 p-4 text-sm">
          <Info className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
          <div>
            <p className="font-medium">Implementation status is explicit</p>
            <p className="mt-0.5 text-muted-foreground">
              <span className="font-medium">Available</span> adapters are covered by contract
              tests. <span className="font-medium">Scaffolded</span> adapters validate configuration
              and preflight the CLI but do not execute tasks yet. Planned adapters are registered so
              agents can reference them, and fail closed at execution time.
            </p>
          </div>
        </CardContent>
      </Card>

      {harnesses.isLoading ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {Array.from({ length: 4 }).map((_, index) => (
            <Skeleton key={index} className="h-64 rounded-xl" />
          ))}
        </div>
      ) : harnesses.isError ? (
        <p className="text-sm text-destructive">{errorMessage(harnesses.error)}</p>
      ) : !harnesses.data?.length ? (
        <EmptyState icon={<Cpu className="h-6 w-6" />} title="No harnesses registered" />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {harnesses.data.map((harness) => {
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

            return (
              <Card key={harness.kind} className="flex flex-col">
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-2.5">
                      <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
                        <Cpu className="h-5 w-5" />
                      </span>
                      <div>
                        <CardTitle>{harness.name}</CardTitle>
                        <p className="text-[11px] text-muted-foreground">{harness.kind}</p>
                      </div>
                    </div>
                    <ImplementationBadge status={harness.implementationStatus} />
                  </div>
                </CardHeader>
                <CardContent className="flex flex-1 flex-col gap-3">
                  <p className="text-sm text-muted-foreground">{harness.description}</p>

                  <div className="flex flex-wrap gap-1.5">
                    {harness.descriptor?.capabilities?.executionModes?.map((mode) => (
                      <Badge key={mode} variant="secondary">
                        {titleCase(mode)}
                      </Badge>
                    ))}
                    {harness.descriptor?.capabilities?.maxTimeoutSeconds ? (
                      <Badge variant="outline">
                        timeout ≤ {harness.descriptor.capabilities.maxTimeoutSeconds}s
                      </Badge>
                    ) : null}
                  </div>

                  <div className="grid grid-cols-2 gap-1.5 text-[11px]">
                    {flags.map(([label, enabled]) => (
                      <span
                        key={label}
                        className={enabled ? "text-foreground" : "text-muted-foreground line-through"}
                      >
                        {enabled ? "✓" : "×"} {label}
                      </span>
                    ))}
                  </div>

                  <div className="mt-auto flex items-center justify-between border-t pt-3 text-[11px] text-muted-foreground">
                    <span className="inline-flex items-center gap-1">
                      <ShieldCheck className="h-3 w-3" />
                      {harness.enabled ? "enabled" : "disabled"}
                    </span>
                    <span className="inline-flex items-center gap-1">
                      <Sparkles className="h-3 w-3" /> health: {harness.health.status}
                    </span>
                    {harness.descriptor?.docsUrl ? (
                      <a
                        href={harness.descriptor.docsUrl}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-1 text-primary hover:underline"
                      >
                        docs <ExternalLink className="h-3 w-3" />
                      </a>
                    ) : null}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <section className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Active workers
        </h2>
        {workers.data?.length ? (
          <div className="grid gap-2 sm:grid-cols-2">
            {workers.data.map((worker) => (
              <Card key={worker.workerId}>
                <CardContent className="flex flex-wrap items-center justify-between gap-2 p-3 text-xs">
                  <div className="min-w-0">
                    <p className="truncate font-mono text-[11px]">{worker.workerId}</p>
                    <p className="text-muted-foreground">
                      {worker.harnesses.join(", ")} · target {worker.target}
                      {worker.hostname ? ` · ${worker.hostname}` : ""}
                    </p>
                  </div>
                  <Badge variant={worker.live ? "success" : "destructive"}>
                    {worker.live ? "live" : "stale"}
                  </Badge>
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No workers have reported a heartbeat yet.</p>
        )}
      </section>

      {meta.data ? (
        <p className="text-center text-[11px] text-muted-foreground">
          Event transport: {meta.data.eventsDriver} · environment: {meta.data.environment} ·
          harness health reporting arrives with worker heartbeats.
        </p>
      ) : null}
    </div>
  );
}
