import Link from "next/link";
import { Bot, ChevronRight, Plug, RotateCcw, ShieldCheck, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { AgentStatusBadge } from "@/components/ui/status-badge";
import { relativeTime, titleCase } from "@/lib/utils";
import { autonomyName } from "@/lib/presentation";
import type { AgentSummary } from "@/lib/types";

export function AgentCard({
  agent,
  onRestore,
  onDelete,
}: {
  agent: AgentSummary;
  onRestore?: (id: string) => void;
  onDelete?: (id: string) => void;
}) {
  const archived = agent.status === "archived";

  return (
    <Card className="flex h-full flex-col transition-all hover:border-primary/40 hover:shadow-md">
      <Link href={`/agents/${agent.id}`} className="group block flex-1">
        <CardContent className="flex h-full flex-col gap-3 p-4 sm:p-5">
          <div className="flex items-start justify-between gap-2">
            <div className="flex min-w-0 items-center gap-2.5">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Bot className="h-5 w-5" />
              </span>
              <div className="min-w-0">
                <p className="truncate font-semibold leading-tight">{agent.name}</p>
                <p className="truncate text-[11px] text-muted-foreground">v{agent.currentVersion}</p>
              </div>
            </div>
            <AgentStatusBadge status={agent.status} />
          </div>

          <p className="line-clamp-2 min-h-10 text-sm text-muted-foreground">
            {agent.description || "No description yet."}
          </p>

          <div className="mt-auto flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
            <span className="rounded bg-muted px-1.5 py-0.5 font-medium">
              {agent.harness ? titleCase(agent.harness) : "harness?"}
            </span>
            <span className="inline-flex items-center gap-1">
              <ShieldCheck className="h-3 w-3" /> {autonomyName(agent.autonomyLevel)}
            </span>
            <span className="inline-flex items-center gap-1">
              <Plug className="h-3 w-3" /> {agent.connectorCount}
            </span>
            <span className="ml-auto">{relativeTime(agent.updatedAt)}</span>
            <ChevronRight className="h-4 w-4 opacity-0 transition-opacity group-hover:opacity-100" />
          </div>
        </CardContent>
      </Link>

      {archived ? (
        <div className="flex flex-wrap gap-2 border-t px-4 py-3 sm:px-5">
          <Button size="sm" variant="outline" onClick={() => onRestore?.(agent.id)}>
            <RotateCcw /> Restore
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="text-destructive"
            onClick={() => onDelete?.(agent.id)}
          >
            <Trash2 /> Delete permanently
          </Button>
        </div>
      ) : null}
    </Card>
  );
}
