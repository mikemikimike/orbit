import Link from "next/link";
import { ArrowRight, Clock3 } from "lucide-react";
import { ExecutionStatusBadge } from "@/components/ui/status-badge";
import { formatDuration, relativeTime, titleCase, truncate } from "@/lib/utils";
import type { ExecutionRecord } from "@/lib/types";
import { cn } from "@/lib/utils";

export function ExecutionRow({
  execution,
  showAgent = true,
  className,
}: {
  execution: ExecutionRecord;
  showAgent?: boolean;
  className?: string;
}) {
  const task =
    typeof execution.input === "object" && execution.input && "task" in execution.input
      ? String((execution.input as { task?: unknown }).task ?? "")
      : "";

  return (
    <Link
      href={`/executions/${execution.id}`}
      className={cn(
        "group flex items-center gap-3 border-b px-4 py-3 transition-colors last:border-b-0 hover:bg-muted/40 sm:px-5",
        className,
      )}
    >
      <ExecutionStatusBadge status={execution.status} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">
          {showAgent ? execution.agentName : truncate(task || "Execution", 80)}
        </p>
        <p className="truncate text-[11px] text-muted-foreground">
          {showAgent ? truncate(task || "No task recorded", 90) : titleCase(execution.triggerSource)}
          {" · "}
          {relativeTime(execution.createdAt)}
        </p>
      </div>
      <div className="hidden shrink-0 items-center gap-1 text-[11px] text-muted-foreground sm:flex">
        <Clock3 className="h-3 w-3" />
        {formatDuration(execution.startedAt, execution.finishedAt)}
      </div>
      <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
    </Link>
  );
}
