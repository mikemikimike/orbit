"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Clock3, ShieldCheck, XCircle } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { ExecutionStatusBadge } from "@/components/ui/status-badge";
import { api } from "@/lib/api";
import { queryKeys, unwrap, useApprovals } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDateTime, relativeTime, titleCase } from "@/lib/utils";
import { capabilityLabel, effectLabel } from "@/lib/presentation";
import type { ApprovalWithExecution } from "@/lib/types";

interface ReviewState {
  approval: ApprovalWithExecution;
  decision: "approved" | "rejected";
}

export default function ApprovalsPage() {
  const pending = useApprovals("pending");
  const decided = useApprovals(undefined);
  const queryClient = useQueryClient();
  const [review, setReview] = useState<ReviewState | null>(null);
  const [comment, setComment] = useState("");

  const history = useMemo(
    () => (decided.data ?? []).filter((item) => item.status !== "pending").slice(0, 30),
    [decided.data],
  );

  const decide = useMutation({
    mutationFn: async ({ approval, decision }: ReviewState) =>
      unwrap(
        await api.POST(
          decision === "approved"
            ? "/api/v1/approvals/{approval_id}/approve"
            : "/api/v1/approvals/{approval_id}/reject",
          {
            params: { path: { approval_id: approval.id } },
            body: { comment: comment.trim() || undefined },
          },
        ),
      ),
    onSuccess: (_, variables) => {
      toast.success(
        variables.decision === "approved" ? "Approval granted" : "Approval rejected",
      );
      setReview(null);
      setComment("");
      void queryClient.invalidateQueries({ queryKey: queryKeys.approvals("pending") });
      void queryClient.invalidateQueries({ queryKey: queryKeys.approvals(undefined) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.execution(variables.approval.executionId),
      });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Human control"
        title="Approvals"
        description="Writes and external side effects pause here until a human decides. The tool call, agent version and argument payload are recorded."
      />

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Pending
          </h2>
          <Badge variant={pending.data?.length ? "warning" : "secondary"}>
            {pending.data?.length ?? 0}
          </Badge>
        </div>
        {pending.isLoading ? (
          <Skeleton className="h-32 rounded-xl" />
        ) : pending.isError ? (
          <p className="text-sm text-destructive">{errorMessage(pending.error)}</p>
        ) : pending.data?.length ? (
          <div className="space-y-3">
            {pending.data.map((approval) => (
              <ApprovalCard
                key={approval.id}
                approval={approval}
                onApprove={() => {
                  setComment("");
                  setReview({ approval, decision: "approved" });
                }}
                onReject={() => {
                  setComment("");
                  setReview({ approval, decision: "rejected" });
                }}
              />
            ))}
          </div>
        ) : (
          <EmptyState
            icon={<ShieldCheck className="h-6 w-6" />}
            title="Nothing waiting for you"
            description="Agents running below autonomy L4 will pause here before write actions."
          />
        )}
      </section>

      <section className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Recent decisions
        </h2>
        {history.length ? (
          <Card className="overflow-hidden">
            {history.map((approval) => (
              <div
                key={approval.id}
                className="flex flex-wrap items-center justify-between gap-2 border-b px-4 py-3 text-sm last:border-b-0"
              >
                <div className="min-w-0">
                  <p className="flex flex-wrap items-center gap-2">
                    <span className="font-medium">{approval.agentName}</span>
                    <Badge
                      variant={
                        approval.status === "approved"
                          ? "success"
                          : approval.status === "rejected"
                            ? "destructive"
                            : "outline"
                      }
                    >
                      {titleCase(approval.status)}
                    </Badge>
                  </p>
                  <p className="truncate text-[11px] text-muted-foreground">
                    {capabilityOf(approval)} · {formatDateTime(approval.createdAt)}
                    {approval.comment ? ` · “${approval.comment}”` : ""}
                  </p>
                </div>
                <Link
                  href={`/executions/${approval.executionId}`}
                  className="text-xs text-primary hover:underline"
                >
                  Open execution
                </Link>
              </div>
            ))}
          </Card>
        ) : (
          <p className="text-sm text-muted-foreground">No decisions yet.</p>
        )}
      </section>

      <Dialog open={Boolean(review)} onOpenChange={(open) => !open && setReview(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {review?.decision === "approved"
                ? `Approve: ${review ? approvalTitle(review.approval) : "action"}`
                : "Reject action"}
            </DialogTitle>
            <DialogDescription>
              {review ? `${review.approval.agentName} · ${capabilityOf(review.approval)}` : ""}
            </DialogDescription>
          </DialogHeader>
          {review ? <ApprovalDetails approval={review.approval} /> : null}
          <div className="space-y-1.5">
            <p className="text-xs font-medium text-muted-foreground">Comment (optional)</p>
            <Textarea
              value={comment}
              onChange={(event) => setComment(event.target.value)}
              placeholder={
                review?.decision === "approved"
                  ? "Why is this action safe?"
                  : "Why is this action rejected?"
              }
            />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setReview(null)}>
              Cancel
            </Button>
            <Button
              variant={review?.decision === "rejected" ? "destructive" : "default"}
              loading={decide.isPending}
              onClick={() => review && decide.mutate(review)}
            >
              {review?.decision === "approved" ? (
                <>
                  <CheckCircle2 /> Approve and execute
                </>
              ) : (
                <>
                  <XCircle /> Reject
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ApprovalCard({
  approval,
  onApprove,
  onReject,
}: {
  approval: ApprovalWithExecution;
  onApprove: () => void;
  onReject: () => void;
}) {
  return (
    <Card>
      <CardContent className="space-y-3 p-4 sm:p-5">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="min-w-0">
            <p className="flex flex-wrap items-center gap-2 text-sm font-semibold">
              {approval.agentName}
              <ExecutionStatusBadge status={approval.executionStatus} />
            </p>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              {approvalTitle(approval)} · requested {relativeTime(approval.createdAt)}
            </p>
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" className="text-destructive" onClick={onReject}>
              <XCircle /> Reject
            </Button>
            <Button size="sm" onClick={onApprove}>
              <CheckCircle2 /> Approve
            </Button>
          </div>
        </div>
        <ApprovalDetails approval={approval} />
        <div className="flex flex-wrap items-center gap-3 text-[11px] text-muted-foreground">
          <Link
            href={`/executions/${approval.executionId}`}
            className="inline-flex items-center gap-1 text-primary hover:underline"
          >
            <Clock3 className="h-3 w-3" /> Execution timeline
          </Link>
          {approval.expiresAt ? <span>expires {formatDateTime(approval.expiresAt)}</span> : null}
        </div>
      </CardContent>
    </Card>
  );
}

function ApprovalDetails({ approval }: { approval: ApprovalWithExecution }) {
  const action = (approval.requestedAction ?? {}) as {
    capability?: string;
    connector?: string;
    connectionName?: string;
    effect?: string;
    arguments?: Record<string, unknown>;
  };
  const args = action.arguments ?? {};
  const rows = actionRows(action.capability, args);
  return (
    <div className="space-y-2 rounded-lg border bg-muted/30 p-3 text-xs">
      <p className="text-sm font-medium">{approvalTitle(approval)}</p>
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px]">
        <span>
          <span className="text-muted-foreground">agent:</span> {approval.agentName}
        </span>
        <span>
          <span className="text-muted-foreground">connection:</span>{" "}
          {action.connectionName ?? "—"}
        </span>
        <span>
          <span className="text-muted-foreground">connector:</span>{" "}
          {action.connector ?? "—"}
        </span>
        <span>
          <span className="text-muted-foreground">effect:</span>{" "}
          {effectLabel(action.effect)}
        </span>
      </div>
      {rows.length ? (
        <dl className="grid gap-1 sm:grid-cols-[7rem_1fr]">
          {rows.map(([label, value]) => (
            <div key={label} className="contents">
              <dt className="text-muted-foreground">{label}</dt>
              <dd className="break-words font-mono text-[11px]">{value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      <p className="text-[10px] text-muted-foreground">
        Approving allows this exact call once. Rejecting returns a policy error to the
        harness without executing it.
      </p>
      {Object.keys(args).length ? (
        <details>
          <summary className="cursor-pointer text-[10px] text-muted-foreground hover:text-foreground">
            Raw arguments
          </summary>
          <pre className="mt-1 max-h-40 overflow-auto whitespace-pre-wrap break-words font-mono text-[10px] text-muted-foreground">
            {JSON.stringify(args, null, 2)}
          </pre>
        </details>
      ) : null}
    </div>
  );
}

function approvalTitle(approval: ApprovalWithExecution): string {
  const action = (approval.requestedAction ?? {}) as {
    capability?: string;
    arguments?: Record<string, unknown>;
  };
  const args = action.arguments ?? {};
  switch (action.capability) {
    case "repo.commit":
      return `Commit changes${args.message ? `: ${String(args.message).slice(0, 80)}` : ""}`;
    case "repo.push":
      return `Push branch${args.branch ? ` ${args.branch}` : ""}`;
    case "pulls.create":
      return `Open pull request${args.title ? `: ${String(args.title).slice(0, 80)}` : ""}`;
    case "issues.comment":
      return `Comment on issue${args.number ? ` #${args.number}` : ""}`;
    case "issues.update":
      return `Update issue${args.number ? ` #${args.number}` : ""}`;
    case "issues.create":
      return `Create issue${args.title ? `: ${String(args.title).slice(0, 80)}` : ""}`;
    default:
      return capabilityOf(approval);
  }
}

function actionRows(
  capability: string | undefined,
  args: Record<string, unknown>,
): Array<[string, string]> {
  const rows: Array<[string, string]> = [];
  const push = (label: string, value: unknown) => {
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      rows.push([label, String(value).slice(0, 1000)]);
    }
  };
  if (capability === "repo.commit") push("message", args.message);
  if (capability === "repo.push") push("branch", args.branch);
  if (capability === "pulls.create") {
    push("title", args.title);
    push("head", args.head);
    push("base", args.base);
    push("draft", args.draft);
  }
  if (capability?.startsWith("issues.")) push("number", args.number);
  if (capability === "issues.update" || capability === "issues.create") {
    push("state", args.state);
    push("labels", Array.isArray(args.labels) ? args.labels.join(", ") : args.labels);
    push("assignees", Array.isArray(args.assignees) ? args.assignees.join(", ") : args.assignees);
  }
  if (capability === "issues.comment") push("body", args.body);
  return rows;
}

function capabilityOf(approval: ApprovalWithExecution): string {
  const action = (approval.requestedAction ?? {}) as { capability?: string };
  return capabilityLabel(action.capability);
}
