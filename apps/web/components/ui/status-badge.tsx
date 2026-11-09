import { Badge } from "@/components/ui/badge";
import { AGENT_STATUS_TONE, EXECUTION_STATUS_TONE, titleCase } from "@/lib/utils";

export function ExecutionStatusBadge({ status }: { status: string }) {
  const active = ["queued", "dispatched", "running", "awaiting_approval"].includes(status);
  return (
    <Badge variant={EXECUTION_STATUS_TONE[status] ?? "secondary"} className="capitalize">
      {active ? (
        <span className="mr-0.5 inline-block h-1.5 w-1.5 animate-pulse-soft rounded-full bg-current" />
      ) : null}
      {titleCase(status)}
    </Badge>
  );
}

export function AgentStatusBadge({ status }: { status: string }) {
  return (
    <Badge variant={AGENT_STATUS_TONE[status] ?? "secondary"} className="capitalize">
      {titleCase(status)}
    </Badge>
  );
}

export function ImplementationBadge({ status }: { status: string }) {
  const variant =
    status === "available" ? "success" : status === "scaffolded" ? "warning" : "outline";
  return (
    <Badge variant={variant} className="capitalize">
      {status}
    </Badge>
  );
}
