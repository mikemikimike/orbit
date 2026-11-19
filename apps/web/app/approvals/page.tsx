"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowUpRight, Check, ShieldCheck, X } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Callout } from "@/components/ui/callout";
import { Card } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { Field } from "@/components/ui/field";
import { PageHeader } from "@/components/ui/page-header";
import { Section } from "@/components/ui/section";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import { queryKeys, unwrap, useApprovals } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDateTime, relativeTime, titleCase } from "@/lib/utils";
import { brandName, capabilityLabel, effectLabel, effectTone } from "@/lib/presentation";
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
      toast.success(variables.decision === "approved" ? "Approved" : "Rejected");
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

  function open(approval: ApprovalWithExecution, decision: ReviewState["decision"]) {
    setComment("");
    setReview({ approval, decision });
  }

  const approving = review?.decision === "approved";

  return (
    <div className="space-y-7">
      <PageHeader
        title="Approvals"
        description="When an agent wants to change something outside ORBIT, it stops and asks here. You see the exact call before it happens."
      />

      <Section title="Waiting for a decision" count={pending.data?.length}>
        {pending.isLoading ? (
          <Skeleton className="h-40 rounded-xl" />
        ) : pending.isError ? (
          <Callout tone="danger" title="Approvals could not be loaded">
            {errorMessage(pending.error)}
          </Callout>
        ) : pending.data?.length ? (
          <ul className="space-y-3">
            {pending.data.map((approval) => (
              <li key={approval.id}>
                <ApprovalCard
                  approval={approval}
                  onApprove={() => open(approval, "approved")}
                  onReject={() => open(approval, "rejected")}
                />
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState
            icon={<ShieldCheck />}
            title="Nothing is waiting for you"
            description="An agent that prepares writes for approval pauses here before it changes, posts or sends anything outside ORBIT."
          />
        )}
      </Section>

      <Section title="Recent decisions">
        {decided.isLoading ? (
          <Skeleton className="h-24 rounded-xl" />
        ) : history.length ? (
          <Card className="overflow-hidden">
            <ul>
              {history.map((approval) => (
                <li
                  key={approval.id}
                  className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1 border-b px-4 py-3 last:border-b-0 sm:grid-cols-[6.5rem_minmax(0,1fr)_auto] sm:px-5"
                >
                  <Badge
                    className="hidden w-fit sm:inline-flex"
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
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{approvalTitle(approval)}</p>
                    <p className="truncate text-[13px] text-muted-foreground">
                      <span className="sm:hidden">{titleCase(approval.status)} · </span>
                      {approval.agentName}, {formatDateTime(approval.createdAt)}
                      {approval.comment ? `: “${approval.comment}”` : ""}
                    </p>
                  </div>
                  <Link
                    href={`/executions/${approval.executionId}`}
                    className="inline-flex items-center gap-1 text-[13px] font-medium text-primary underline-offset-4 hover:underline"
                  >
                    Execution <ArrowUpRight className="h-3.5 w-3.5" />
                  </Link>
                </li>
              ))}
            </ul>
          </Card>
        ) : (
          <p className="text-sm text-muted-foreground">No decisions have been made yet.</p>
        )}
      </Section>

      <Dialog open={Boolean(review)} onOpenChange={(next) => !next && setReview(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{approving ? "Approve this action?" : "Reject this action?"}</DialogTitle>
            <DialogDescription>
              {approving
                ? "The agent may make this exact call once. Anything else it tries is asked again."
                : "The call is not made. The agent is told it was refused and carries on without it."}
            </DialogDescription>
          </DialogHeader>
          {review ? <ApprovalDetails approval={review.approval} /> : null}
          <Field
            label="Comment"
            optional
            htmlFor="approval-comment"
            hint="Recorded in the audit trail with your decision."
          >
            <Textarea
              id="approval-comment"
              value={comment}
              onChange={(event) => setComment(event.target.value)}
              placeholder={approving ? "Why is this safe to run?" : "Why is this refused?"}
            />
          </Field>
          <DialogFooter>
            <Button variant="outline" onClick={() => setReview(null)}>
              Cancel
            </Button>
            <Button
              variant={approving ? "default" : "destructive"}
              loading={decide.isPending}
              onClick={() => review && decide.mutate(review)}
            >
              {approving ? (
                <>
                  <Check /> Approve
                </>
              ) : (
                <>
                  <X /> Reject
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
    <Card className="border-warning/40">
      <div className="flex flex-col gap-4 p-4 sm:p-5">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <p className="text-[15px] font-semibold">{approvalTitle(approval)}</p>
            <p className="mt-0.5 text-[13px] text-muted-foreground">
              Asked by{" "}
              <Link
                href={`/executions/${approval.executionId}`}
                className="font-medium text-foreground underline-offset-4 hover:underline"
              >
                {approval.agentName}
              </Link>{" "}
              {relativeTime(approval.createdAt)}
              {approval.expiresAt ? (
                <>
                  .{" "}
                  <span className="font-medium text-warning" title={formatDateTime(approval.expiresAt)}>
                    Expires {relativeTime(approval.expiresAt)}
                  </span>
                  ; after that the action is refused
                </>
              ) : null}
            </p>
          </div>
          <div className="flex shrink-0 gap-2">
            <Button
              variant="outline"
              className="flex-1 text-destructive hover:border-destructive/50 hover:bg-destructive/10 sm:flex-none"
              onClick={onReject}
            >
              <X /> Reject
            </Button>
            <Button className="flex-1 sm:flex-none" onClick={onApprove}>
              <Check /> Approve
            </Button>
          </div>
        </div>
        <ApprovalDetails approval={approval} />
      </div>
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
  const tone = effectTone(action.effect);
  return (
    <div className="space-y-3 rounded-lg border bg-subtle p-3 text-[13px]">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
        <Badge variant={tone === "safe" ? "secondary" : "warning"}>
          {effectLabel(action.effect)}
        </Badge>
        <span>
          <span className="text-muted-foreground">Agent</span> {approval.agentName}
        </span>
        {action.connectionName ? (
          <span>
            <span className="text-muted-foreground">Connection</span> {action.connectionName}
          </span>
        ) : null}
        {action.connector ? (
          <span>
            <span className="text-muted-foreground">Connector</span> {brandName(action.connector)}
          </span>
        ) : null}
      </div>
      {rows.length ? (
        <dl className="grid gap-x-4 gap-y-1.5 sm:grid-cols-[6rem_minmax(0,1fr)]">
          {rows.map(([label, value]) => (
            <div key={label} className="contents">
              <dt className="text-muted-foreground">{titleCase(label)}</dt>
              <dd className="break-words font-mono text-xs leading-relaxed">{value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {Object.keys(args).length ? (
        <details>
          <summary className="w-fit cursor-pointer select-none text-muted-foreground hover:text-foreground">
            Full arguments
          </summary>
          <pre className="code-block mt-1.5 max-h-48 whitespace-pre-wrap break-words bg-card">
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
  // Where it happens comes first: the same action on another repository is another decision.
  push("repository", [args.owner, args.repo ?? args.project].filter(Boolean).join("/"));
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
