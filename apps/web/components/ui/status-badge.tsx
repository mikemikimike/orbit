import { Badge } from "@/components/ui/badge";
import { Status, agentStatusMeta, executionStatusMeta } from "@/components/ui/status";

export function ExecutionStatusBadge({
  status,
  className,
  hideLabel,
}: {
  status: string;
  className?: string;
  hideLabel?: boolean;
}) {
  return <Status meta={executionStatusMeta(status)} className={className} hideLabel={hideLabel} />;
}

export function AgentStatusBadge({
  status,
  className,
  hideLabel,
}: {
  status: string;
  className?: string;
  hideLabel?: boolean;
}) {
  return <Status meta={agentStatusMeta(status)} className={className} hideLabel={hideLabel} />;
}

/** How real a harness adapter is. Honest by contract: see AGENTS.md rule 6. */
export function ImplementationBadge({ status }: { status: string }) {
  const variant =
    status === "available" ? "success" : status === "scaffolded" ? "warning" : "outline";
  return (
    <Badge variant={variant} className="capitalize">
      {status}
    </Badge>
  );
}
