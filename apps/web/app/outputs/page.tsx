"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Megaphone, Pencil, Plus, Send, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useAgents, useAgentTeams } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import {
  createOutput,
  deleteOutput,
  listOutputs,
  testOutput,
  updateOutput,
  type OutputInput,
  type OutputView,
} from "@/lib/outputs";
import { relativeTime } from "@/lib/utils";

const EVENTS = ["succeeded", "failed", "cancelled"] as const;

type Form = {
  name: string;
  kind: OutputInput["kind"];
  agentId: string;
  teamId: string;
  events: string[];
  webhookUrl: string;
  template: string;
  enabled: boolean;
};

const EMPTY: Form = {
  name: "",
  kind: "slack",
  agentId: "",
  teamId: "",
  events: [...EVENTS],
  webhookUrl: "",
  template: "",
  enabled: true,
};

export default function OutputsPage() {
  const queryClient = useQueryClient();
  const outputs = useQuery({ queryKey: ["outputs"], queryFn: listOutputs, staleTime: 15_000 });
  const agents = useAgents({ limit: 100 });
  const teams = useAgentTeams();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<OutputView | null>(null);
  const [form, setForm] = useState<Form>(EMPTY);

  const save = useMutation({
    mutationFn: async () => {
      const body: OutputInput = {
        name: form.name.trim(),
        kind: form.kind,
        agentId: form.agentId || null,
        teamId: form.teamId || null,
        events: form.events,
        webhookUrl: form.webhookUrl.trim() || undefined,
        template: form.template.trim() || null,
        enabled: form.enabled,
      };
      return editing ? updateOutput(editing.id, body) : createOutput(body);
    },
    onSuccess: () => {
      toast.success(editing ? "Output updated" : "Output created");
      setOpen(false);
      setEditing(null);
      void queryClient.invalidateQueries({ queryKey: ["outputs"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: deleteOutput,
    onSuccess: () => {
      toast.success("Output deleted");
      void queryClient.invalidateQueries({ queryKey: ["outputs"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const test = useMutation({
    mutationFn: testOutput,
    onSuccess: () => toast.success("Test notification sent"),
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setEditing(null);
    setForm(EMPTY);
    setOpen(true);
  }

  function openEdit(output: OutputView) {
    setEditing(output);
    setForm({
      name: output.name,
      kind: output.kind,
      agentId: output.agentId ?? "",
      teamId: output.teamId ?? "",
      events: output.events,
      webhookUrl: "",
      template: output.template ?? "",
      enabled: output.enabled,
    });
    setOpen(true);
  }

  const targetName = (output: OutputView) =>
    output.teamId
      ? (teams.data?.find((team) => team.id === output.teamId)?.name ?? "Team")
      : output.agentId
        ? (agents.data?.items.find((agent) => agent.id === output.agentId)?.name ?? "Agent")
        : "All agents";

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Notifications"
        title="Outputs"
        description="Send execution results to Slack, Mattermost or a webhook after a run finishes."
        actions={
          <Button onClick={openCreate}>
            <Plus /> New output
          </Button>
        }
      />

      {outputs.isLoading ? (
        <Skeleton className="h-40" />
      ) : outputs.isError ? (
        <p className="text-sm text-destructive">{errorMessage(outputs.error)}</p>
      ) : !outputs.data?.length ? (
        <EmptyState
          icon={<Megaphone className="h-6 w-6" />}
          title="No outputs"
          description="Create an output to notify Slack, Mattermost or a webhook when an execution succeeds or fails."
          action={<Button onClick={openCreate}>Create output</Button>}
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {outputs.data.map((output) => (
            <Card key={output.id}>
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle className="flex min-w-0 items-center gap-2">
                    <Megaphone className="h-4 w-4 shrink-0 text-primary" />
                    <span className="truncate">{output.name}</span>
                  </CardTitle>
                  <Badge variant={output.enabled ? "success" : "outline"}>
                    {output.enabled ? "enabled" : "disabled"}
                  </Badge>
                </div>
                <p className="text-[11px] text-muted-foreground">
                  {output.kind} · {targetName(output)} · {output.events.join(", ")}
                </p>
              </CardHeader>
              <CardContent className="space-y-2">
                {!output.hasWebhook ? (
                  <p className="text-[11px] text-destructive">Webhook URL is missing.</p>
                ) : null}
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => test.mutate(output.id)}
                    loading={test.isPending && test.variables === output.id}
                  >
                    <Send /> Test
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => openEdit(output)}>
                    <Pencil /> Edit
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-destructive"
                    onClick={() => {
                      if (window.confirm(`Delete output ${output.name}?`)) remove.mutate(output.id);
                    }}
                  >
                    <Trash2 /> Delete
                  </Button>
                </div>
                <p className="text-[10px] text-muted-foreground">
                  updated {relativeTime(output.updatedAt)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit output" : "New output"}</DialogTitle>
            <DialogDescription>
              Slack/Mattermost use an incoming webhook URL. The URL is stored encrypted and
              never returned.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Name</Label>
                <Input
                  value={form.name}
                  onChange={(event) => setForm({ ...form, name: event.target.value })}
                />
              </div>
              <div className="space-y-1.5">
                <Label>Kind</Label>
                <select
                  className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                  value={form.kind}
                  onChange={(event) =>
                    setForm({ ...form, kind: event.target.value as OutputInput["kind"] })
                  }
                >
                  <option value="slack">Slack</option>
                  <option value="mattermost">Mattermost</option>
                  <option value="webhook">Webhook</option>
                </select>
              </div>
            </div>
            <div className="space-y-1.5">
              <Label>Target</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={
                  form.teamId ? `team:${form.teamId}` : form.agentId ? `agent:${form.agentId}` : ""
                }
                onChange={(event) => {
                  const value = event.target.value;
                  if (value.startsWith("team:")) {
                    setForm({ ...form, teamId: value.slice(5), agentId: "" });
                  } else if (value.startsWith("agent:")) {
                    setForm({ ...form, agentId: value.slice(6), teamId: "" });
                  } else {
                    setForm({ ...form, agentId: "", teamId: "" });
                  }
                }}
              >
                <option value="">All agents (executions)</option>
                <optgroup label="Agents">
                  {(agents.data?.items ?? []).map((agent) => (
                    <option key={agent.id} value={`agent:${agent.id}`}>
                      {agent.name}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="Agent teams (team runs)">
                  {(teams.data ?? []).map((team) => (
                    <option key={team.id} value={`team:${team.id}`}>
                      {team.name}
                    </option>
                  ))}
                </optgroup>
              </select>
              <p className="text-[11px] text-muted-foreground">
                Agent targets fire per execution; team targets fire once when a team run
                finishes.
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>Send on</Label>
              <div className="flex flex-wrap gap-3 text-xs">
                {EVENTS.map((event) => (
                  <label key={event} className="flex items-center gap-1.5">
                    <input
                      type="checkbox"
                      checked={form.events.includes(event)}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          events: checked.target.checked
                            ? [...form.events, event]
                            : form.events.filter((item) => item !== event),
                        })
                      }
                    />
                    {event}
                  </label>
                ))}
              </div>
            </div>
            <div className="space-y-1.5">
              <Label>
                {form.kind === "slack"
                  ? "Slack incoming webhook URL"
                  : form.kind === "mattermost"
                    ? "Mattermost incoming webhook URL"
                    : "Webhook URL"}
              </Label>
              <Input
                type="password"
                value={form.webhookUrl}
                placeholder={editing?.hasWebhook ? "leave blank to keep the current URL" : "https://…"}
                onChange={(event) => setForm({ ...form, webhookUrl: event.target.value })}
              />
              <p className="text-[11px] text-muted-foreground">
                {form.kind === "webhook"
                  ? "ORBIT POSTs JSON with the event, execution and text."
                  : "Use the incoming-webhook URL from your Slack/Mattermost integration."}
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>Template (optional)</Label>
              <Textarea
                className="min-h-20 text-xs"
                value={form.template}
                onChange={(event) => setForm({ ...form, template: event.target.value })}
                placeholder="Optional note shown with the summary"
              />
            </div>
            <label className="flex items-center gap-2 text-sm">
              <Switch
                checked={form.enabled}
                onCheckedChange={(enabled) => setForm({ ...form, enabled })}
              />
              Enabled
            </label>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => save.mutate()}
              loading={save.isPending}
              disabled={!form.name.trim() || form.events.length === 0}
            >
              {editing ? "Save changes" : "Create output"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
