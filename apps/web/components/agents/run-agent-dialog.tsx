"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Play } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import { randomId } from "@/lib/browser";
import { unwrap } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";

export function RunAgentDialog({
  agentId,
  agentName,
  disabled,
}: {
  agentId: string;
  agentName: string;
  disabled?: boolean;
}) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [task, setTask] = useState("Summarize the current state and report anything important.");
  const [payloadText, setPayloadText] = useState("{}");
  const [idempotencyKey, setIdempotencyKey] = useState(() => randomId());
  const [submitting, setSubmitting] = useState(false);

  async function run() {
    if (!task.trim()) {
      toast.error("Provide a task for the agent");
      return;
    }
    let payload: unknown = null;
    try {
      payload = payloadText.trim() ? JSON.parse(payloadText) : null;
    } catch {
      toast.error("Payload must be valid JSON");
      return;
    }
    setSubmitting(true);
    try {
      const execution = unwrap(
        await api.POST("/api/v1/agents/{agent_id}/executions", {
          params: { path: { agent_id: agentId } },
          body: { task, payload, idempotencyKey },
          headers: { "idempotency-key": idempotencyKey },
        }),
      );
      toast.success("Execution queued");
      setOpen(false);
      router.push(`/executions/${execution.id}`);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (next) setIdempotencyKey(randomId());
      }}
    >
      <DialogTrigger asChild>
        <Button disabled={disabled}>
          <Play /> Run agent
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Run {agentName}</DialogTitle>
          <DialogDescription>
            The task is queued durably and executed by the harness worker. Live events appear on the
            execution page.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-2">
          <Label htmlFor="task">Task</Label>
          <Textarea
            id="task"
            className="min-h-[130px]"
            value={task}
            onChange={(event) => setTask(event.target.value)}
          />
          <details>
            <summary className="cursor-pointer text-xs text-muted-foreground">
              Payload / connector query (JSON, optional)
            </summary>
            <Textarea
              className="mt-2 min-h-[70px] font-mono text-xs"
              value={payloadText}
              onChange={(event) => setPayloadText(event.target.value)}
              placeholder='{"query":"#ORBIT"}'
            />
            <p className="mt-1 text-[11px] text-muted-foreground">
              X reads use <code className="font-mono">payload.query</code>, then the connection&apos;s
              <code className="font-mono"> config.query</code>, then the task text.
            </p>
          </details>
          <p className="text-[11px] text-muted-foreground">
            Idempotency key <span className="font-mono">{idempotencyKey.slice(0, 8)}…</span> prevents
            duplicate runs on retries.
          </p>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button onClick={run} loading={submitting}>
            <Play /> Queue execution
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
