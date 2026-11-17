"use client";

export const dynamic = 'force-dynamic';

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronRight, ListTodo, Pencil, Plus, Play, Trash2, Users, X } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import { Textarea } from "@/components/ui/textarea";
import {
  queryKeys,
  useAgents,
  useAgentTeams,
  useConnections,
  useSkills,
  useTeamRuns,
  useWorkerProfiles,
} from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import {
  createAgentTeam,
  deleteAgentTeam,
  getTeamRun,
  startTeamRun,
  updateAgentTeam,
  enqueueTeamItems,
  listTeamQueue,
  runTeamQueueItem,
  retryTeamQueueItem,
  requeueTeamQueueItem,
  cancelTeamQueueItem,
  type TeamQueueItem,
  type TeamStepInput,
} from "@/lib/teams";
import type { AgentTeam, TeamRepository, TeamRunDetail, WorkSource } from "@/lib/types";
import { relativeTime, titleCase } from "@/lib/utils";

export default function AgentTeamsPage() {
  const teams = useAgentTeams();
  const [runsArchived, setRunsArchived] = useState<"active" | "archived" | "all">("active");
  const runs = useTeamRuns(
    runsArchived === "all" ? undefined : runsArchived === "archived",
  );
  const agents = useAgents({ limit: 100 });
  const skills = useSkills();
  const workerProfiles = useWorkerProfiles();
  const connections = useConnections();
  const queryClient = useQueryClient();

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<AgentTeam | null>(null);
  const [mode, setMode] = useState<"pipeline" | "native">("native");
  const [name, setName] = useState("Coding team");
  const [description, setDescription] = useState(
    "Primary coding agent with specialist agents for implementation and review.",
  );
  const [instructions, setInstructions] = useState(
    "Pick up the next GitHub issue labelled agent-ready, implement it, test it and request review. Repeat until no ready issues remain.",
  );
  const [teamRepository, setTeamRepository] = useState("opsifyhq/orbit");
  const [teamBranch, setTeamBranch] = useState("");
  const [workSource, setWorkSource] = useState<WorkSource | null>(null);
  const [repositories, setRepositories] = useState<TeamRepository[]>([]);
  const [workerProfileId, setWorkerProfileId] = useState("");
  const [autoLaunch, setAutoLaunch] = useState(false);
  const [teamSkills, setTeamSkills] = useState<string[]>([]);
  const [teamTimeout, setTeamTimeout] = useState("");
  const [steps, setSteps] = useState<TeamStepInput[]>([
    { agentId: "" },
    { agentId: "" },
  ]);
  const [runTeam, setRunTeam] = useState<AgentTeam | null>(null);
  const [task, setTask] = useState("");
  const [repository, setRepository] = useState("");
  const [autoApprove, setAutoApprove] = useState(false);
  const [detail, setDetail] = useState<TeamRunDetail | null>(null);
  const [queueTeam, setQueueTeam] = useState<AgentTeam | null>(null);
  const [queueItems, setQueueItems] = useState<TeamQueueItem[]>([]);
  const [queueText, setQueueText] = useState("");
  const [queueBusy, setQueueBusy] = useState(false);

  const agentOptions = agents.data?.items ?? [];
  const skillOptions = skills.data ?? [];
  const primaryAgent = agentOptions.find((agent) => agent.id === steps[0]?.agentId);
  const nativeInvalid =
    mode === "native" && Boolean(primaryAgent) && primaryAgent?.harness !== "opencode";
  const canCreate =
    name.trim() &&
    steps.length > 0 &&
    !nativeInvalid &&
    steps.every((step) => step.agentId) &&
    new Set(steps.map((step) => step.agentId)).size === steps.length;

  const agentLabel = useMemo(() => {
    const map = new Map(
      agentOptions.map((agent) => [agent.id, `${agent.name} · ${agent.harness}`]),
    );
    return (id: string) => map.get(id) ?? id.slice(0, 8);
  }, [agentOptions]);

  async function openQueue(team: AgentTeam) {
    setQueueTeam(team);
    setQueueText("");
    setQueueBusy(true);
    try { setQueueItems(await listTeamQueue(team.id)); }
    catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function enqueueQueue() {
    if (!queueTeam) return;
    const items = queueText.split("\n").map((line) => line.trim()).filter(Boolean).map((line) => {
      const [externalKey, repository] = line.split("|").map((value) => value.trim());
      return { externalKey, repository: repository || undefined, payload: {} };
    });
    if (!items.length) return;
    setQueueBusy(true);
    try {
      await enqueueTeamItems(queueTeam.id, items);
      setQueueItems(await listTeamQueue(queueTeam.id));
      setQueueText("");
      toast.success("Queue updated");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function runPendingQueue() {
    if (!queueTeam) return;
    setQueueBusy(true);
    try {
      for (const item of queueItems.filter((entry) => entry.status === "pending")) {
        setDetail(await runTeamQueueItem(queueTeam.id, item.id));
      }
      setQueueItems(await listTeamQueue(queueTeam.id));
      toast.success("Pending queue items started");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function runQueueItem(item: TeamQueueItem) {
    if (!queueTeam) return;
    setQueueBusy(true);
    try {
      setDetail(await runTeamQueueItem(queueTeam.id, item.id));
      setQueueItems(await listTeamQueue(queueTeam.id));
      toast.success("Queue item started");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function retryQueueItem(item: TeamQueueItem) {
    if (!queueTeam) return;
    setQueueBusy(true);
    try {
      await retryTeamQueueItem(queueTeam.id, item.id);
      setQueueItems(await listTeamQueue(queueTeam.id));
      toast.success("Queue item queued for retry");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function requeueQueueItem(item: TeamQueueItem) {
    if (!queueTeam) return;
    setQueueBusy(true);
    try {
      await requeueTeamQueueItem(queueTeam.id, item.id);
      setQueueItems(await listTeamQueue(queueTeam.id));
      toast.success("Queue item requeued");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  async function cancelQueueItem(item: TeamQueueItem) {
    if (!queueTeam) return;
    setQueueBusy(true);
    try {
      await cancelTeamQueueItem(queueTeam.id, item.id);
      setQueueItems(await listTeamQueue(queueTeam.id));
      toast.success("Queue item cancelled");
    } catch (error) { toast.error(errorMessage(error)); }
    finally { setQueueBusy(false); }
  }

  const save = useMutation({
    mutationFn: () => {
      const payload = {
        name: name.trim(),
        description,
        mode,
        instructions,
        repository: teamRepository.trim() || undefined,
        branch: teamBranch.trim() || undefined,
        workSource,
        repositories,
        workerProfileId: workerProfileId || undefined,
        autoLaunch,
        skills: teamSkills,
        timeoutSeconds: teamTimeout ? Number(teamTimeout) : undefined,
        steps,
      };
      return editing
        ? updateAgentTeam(editing.id, payload)
        : createAgentTeam(payload);
    },
    onSuccess: () => {
      toast.success(editing ? "Agent team updated" : "Agent team created");
      setOpen(false);
      setEditing(null);
      void queryClient.invalidateQueries({ queryKey: queryKeys.agentTeams });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setEditing(null);
    setMode("native");
    setName("Coding team");
    setDescription("Primary coding agent with specialist agents for implementation and review.");
    setInstructions(
      "Pick up the next GitHub issue labelled agent-ready, implement it, test it and request review. Repeat until no ready issues remain.",
    );
    setTeamRepository("opsifyhq/orbit");
    setTeamBranch("");
    setWorkSource(null);
    setRepositories([]);
    setWorkerProfileId("");
    setAutoLaunch(false);
    setTeamSkills([]);
    setTeamTimeout("");
    setSteps([{ agentId: "" }, { agentId: "" }]);
    setOpen(true);
  }

  function openEdit(team: AgentTeam) {
    setEditing(team);
    setMode(team.definition.mode ?? "pipeline");
    setName(team.name);
    setDescription(team.description);
    setInstructions(team.definition.instructions ?? "");
    setTeamRepository(team.definition.repository ?? "");
    setTeamBranch(team.definition.branch ?? "");
    setWorkSource(team.definition.workSource ?? null);
    setRepositories(team.definition.repositories ?? []);
    setWorkerProfileId(team.definition.workerProfileId ?? "");
    setAutoLaunch(team.definition.autoLaunch ?? false);
    setTeamSkills(team.definition.skills ?? []);
    setTeamTimeout(
      String((team.definition as unknown as { timeoutSeconds?: number }).timeoutSeconds ?? ""),
    );
    setSteps((team.definition.steps ?? []).map((step) => ({ agentId: step.agentId })));
    setOpen(true);
  }

  const remove = useMutation({
    mutationFn: deleteAgentTeam,
    onSuccess: () => {
      toast.success("Agent team deleted");
      void queryClient.invalidateQueries({ queryKey: queryKeys.agentTeams });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const run = useMutation({
    mutationFn: () => startTeamRun(runTeam!.id, task.trim(), repository, autoApprove),
    onSuccess: (result) => {
      toast.success("Agent team run started");
      setRunTeam(null);
      setDetail(result);
      void queryClient.invalidateQueries({ queryKey: queryKeys.teamRuns });
      void queryClient.invalidateQueries({ queryKey: queryKeys.executions() });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  async function inspect(runId: string) {
    try {
      setDetail(await getTeamRun(runId));
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  useEffect(() => {
    const runId = new URLSearchParams(window.location.search).get("run");
    if (runId) void inspect(runId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Orchestration"
        title="Agent Teams"
        description="Ordered primary and specialist agents. Every step is a normal execution with its own connector grants and approvals; skills and the team goal are shared."
        actions={
          <Button onClick={openCreate}>
            <Plus /> New agent team
          </Button>
        }
      />

      {teams.isLoading ? (
        <Skeleton className="h-40 rounded-xl" />
      ) : teams.isError ? (
        <p className="text-sm text-destructive">{errorMessage(teams.error)}</p>
      ) : !teams.data?.length ? (
        <EmptyState
          icon={<Users className="h-6 w-6" />}
          title="No agent teams"
          description="Create a primary coding agent followed by builder, tester and reviewer agents."
          action={<Button onClick={openCreate}><Plus /> Create agent team</Button>}
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {teams.data.map((team) => {
            const steps = team.definition.steps ?? [];
            return (
              <Card key={team.id}>
                <CardHeader className="pb-2">
                  <CardTitle className="truncate">{team.name}</CardTitle>
                  <p className="text-[11px] text-muted-foreground">
                    {(team.definition.mode ?? "pipeline") === "native" ? "native subagents" : "pipeline"} ·{" "}
                    {steps.length} agents · {team.definition.skills?.length ?? 0} skills · updated{" "}
                    {relativeTime(team.updatedAt)}
                  </p>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex flex-wrap gap-1">
                    {steps.map((step, index) => (
                      <Badge key={`${step.agentId}-${index}`} variant={index === 0 ? "default" : "secondary"}>
                        {index === 0
                          ? "primary · "
                          : (team.definition.mode ?? "pipeline") === "native"
                            ? "subagent · "
                            : ""}
                        {agentLabel(step.agentId)}
                      </Badge>
                    ))}
                  </div>
                  {team.definition.repository ? (
                    <p className="text-[11px] text-muted-foreground">
                      repository: <span className="font-mono">{team.definition.repository}</span>
                      {team.definition.branch ? ` · ${team.definition.branch}` : ""}
                    </p>
                  ) : null}
                  {team.definition.model?.model ? (
                    <p className="text-[11px] text-muted-foreground">
                      team model: <span className="font-mono">{team.definition.model.provider ?? ""}/{team.definition.model.model}</span>
                    </p>
                  ) : null}
                  {(team.definition.steps ?? []).some((step) => step.model?.model) ? (
                    <p className="text-[11px] text-muted-foreground">
                      per-step models: {(team.definition.steps ?? []).filter((s) => s.model?.model).length}
                    </p>
                  ) : null}
                  {team.definition.workSource?.enabled ? (
                    <p className="text-[11px] text-muted-foreground">
                      work source: {team.definition.workSource.provider} ·{" "}
                      {team.definition.workSource.repository ??
                        team.definition.workSource.project ??
                        team.definition.workSource.groupId ??
                        ""}
                      {team.lastPollError ? " · last poll failed" : ""}
                    </p>
                  ) : null}
                  {team.definition.repositories?.length ? (
                    <p className="text-[11px] text-muted-foreground">
                      multi-repo: {team.definition.repositories.length} repositories
                    </p>
                  ) : null}
                  {team.definition.autoLaunch ? (
                    <p className="text-[11px] text-muted-foreground">auto-launch worker enabled</p>
                  ) : null}
                  {team.definition.skills?.length ? (
                    <p className="text-[11px] text-muted-foreground">
                      team skills: {team.definition.skills.join(", ")}
                    </p>
                  ) : null}
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      onClick={() => {
                        setRunTeam(team);
                        setTask(team.definition.instructions ?? "");
                        setRepository(team.definition.repository ?? "");
                        setAutoApprove(false);
                      }}
                    >
                      <Play /> Run
                    </Button>
                    <Button size="sm" variant="outline" onClick={() => void openQueue(team)}>
                      <ListTodo /> Queue
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => openEdit(team)}>
                      <Pencil /> Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Delete agent team ${team.name}?`)) remove.mutate(team.id);
                      }}
                    >
                      <Trash2 /> Delete
                    </Button>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <div className="space-y-3">
        <div>
          <h2 className="text-sm font-semibold">Agent team runs</h2>
          <p className="text-[11px] text-muted-foreground">
            Sequential handoffs; each step is recorded as a normal execution.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {(["active", "archived", "all"] as const).map((value) => (
            <Button
              key={value}
              size="sm"
              variant={runsArchived === value ? "default" : "outline"}
              onClick={() => setRunsArchived(value)}
            >
              {value === "active" ? "Active" : value === "archived" ? "Archived" : "All"}
            </Button>
          ))}
        </div>
        {runs.isLoading ? (
          <Skeleton className="h-20 rounded-xl" />
        ) : !runs.data?.length ? (
          <p className="rounded-xl border border-dashed p-4 text-xs text-muted-foreground">
            No agent team runs yet.
          </p>
        ) : (
          <div className="space-y-2">
            {runs.data.map((item) => (
              <Card key={item.id}>
                <CardContent className="flex flex-wrap items-center justify-between gap-3 p-4">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">
                      {String(item.input.task ?? "Agent team run")}
                    </p>
                    <p className="text-[11px] text-muted-foreground">
                      {relativeTime(item.createdAt)} · {titleCase(item.status)}
                    </p>
                    {item.error ? <p className="text-[11px] text-destructive">{item.error}</p> : null}
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={
                        item.status === "succeeded"
                          ? "success"
                          : item.status === "failed"
                            ? "destructive"
                            : "warning"
                      }
                    >
                      {item.status}
                    </Badge>
                    <Button size="sm" variant="outline" onClick={() => void inspect(item.id)}>
                      <ChevronRight /> Steps
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit agent team" : "New agent team"}</DialogTitle>
            <DialogDescription>
              The first agent is the primary agent. Its agent instructions define its role;
              the team instruction defines the shared goal.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Name</Label>
                <Input value={name} onChange={(event) => setName(event.target.value)} />
              </div>
              <div className="space-y-1.5">
                <Label>Description</Label>
                <Input value={description} onChange={(event) => setDescription(event.target.value)} />
              </div>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Repository</Label>
                <Input
                  value={teamRepository}
                  onChange={(event) => setTeamRepository(event.target.value)}
                  placeholder="owner/repo"
                />
              </div>
              <div className="space-y-1.5">
                <Label>Base branch (optional)</Label>
                <Input
                  value={teamBranch}
                  onChange={(event) => setTeamBranch(event.target.value)}
                  placeholder="main"
                />
              </div>
            </div>
            <p className="text-[11px] text-muted-foreground">
              Used as the structured checkout target. A run can override it. Putting a
              repository only in the instruction does not affect checkout.
            </p>
            <div className="space-y-2 rounded-lg border p-3">
              <label className="flex items-start gap-2 text-xs">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={Boolean(workSource?.enabled)}
                  onChange={(event) =>
                    setWorkSource((current) => ({
                      provider: current?.provider ?? "github",
                      enabled: event.target.checked,
                      connectionId: current?.connectionId ?? null,
                      labels: current?.labels ?? ["agent-ready"],
                      claimLabel: current?.claimLabel ?? "agent-in-progress",
                      doneLabel: current?.doneLabel ?? "agent-done",
                      failedLabel: current?.failedLabel ?? "agent-failed",
                      maxItems: current?.maxItems ?? 5,
                      maxAttempts: current?.maxAttempts ?? 1,
                      pollSeconds: current?.pollSeconds ?? 60,
                      autoRun: current?.autoRun ?? false,
                    }))
                  }
                />
                <span>
                  <span className="font-medium">Work source (automatic issue pickup)</span>
                  <span className="mt-0.5 block text-muted-foreground">
                    Polls the selected connector, claims matching issues and enqueues them.
                  </span>
                </span>
              </label>
              {workSource?.enabled ? (
                <div className="grid gap-2 sm:grid-cols-2">
                  <div className="space-y-1">
                    <Label>Provider</Label>
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                      value={workSource.provider}
                      onChange={(event) =>
                        setWorkSource((current) =>
                          current ? { ...current, provider: event.target.value as WorkSource["provider"] } : current,
                        )
                      }
                    >
                      <option value="github">GitHub</option>
                      <option value="gitlab">GitLab</option>
                    </select>
                  </div>
                  <div className="space-y-1">
                    <Label>Connection</Label>
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                      value={workSource.connectionId ?? ""}
                      onChange={(event) =>
                        setWorkSource((current) =>
                          current ? { ...current, connectionId: event.target.value || null } : current,
                        )
                      }
                    >
                      <option value="">Select connection…</option>
                      {(connections.data ?? [])
                        .filter((connection) => connection.connectorSlug === workSource.provider)
                        .map((connection) => (
                          <option key={connection.id} value={connection.id}>
                            {connection.name} · {connection.connectorSlug}
                          </option>
                        ))}
                    </select>
                  </div>
                  {workSource.provider === "github" ? (
                    <div className="space-y-1">
                      <Label>Repository</Label>
                      <Input
                        value={workSource.repository ?? ""}
                        onChange={(event) =>
                          setWorkSource((current) =>
                            current ? { ...current, repository: event.target.value } : current,
                          )
                        }
                        placeholder="owner/repo"
                      />
                    </div>
                  ) : (
                    <>
                      <div className="space-y-1">
                        <Label>Project</Label>
                        <Input
                          value={workSource.project ?? ""}
                          onChange={(event) =>
                            setWorkSource((current) =>
                              current ? { ...current, project: event.target.value } : current,
                            )
                          }
                          placeholder="group/project"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label>Group (alternative to project)</Label>
                        <Input
                          value={workSource.groupId ?? ""}
                          onChange={(event) =>
                            setWorkSource((current) =>
                              current ? { ...current, groupId: event.target.value } : current,
                            )
                          }
                          placeholder="group"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label>Board ID (optional)</Label>
                        <Input
                          value={workSource.boardId ?? ""}
                          onChange={(event) =>
                            setWorkSource((current) =>
                              current
                                ? { ...current, boardId: event.target.value ? Number(event.target.value) : null }
                                : current,
                            )
                          }
                          placeholder="123"
                        />
                      </div>
                    </>
                  )}
                  <div className="space-y-1">
                    <Label>Ready labels (comma-separated)</Label>
                    <Input
                      value={(workSource.labels ?? []).join(",")}
                      onChange={(event) =>
                        setWorkSource((current) =>
                          current
                            ? { ...current, labels: event.target.value.split(",").map((label) => label.trim()).filter(Boolean) }
                            : current,
                        )
                      }
                      placeholder="agent-ready"
                    />
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    <div className="space-y-1">
                      <Label>Max items</Label>
                      <Input
                        type="number"
                        value={workSource.maxItems ?? 5}
                        onChange={(event) =>
                          setWorkSource((current) =>
                            current ? { ...current, maxItems: Number(event.target.value) } : current,
                          )
                        }
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Max attempts</Label>
                      <Input
                        type="number"
                        value={workSource.maxAttempts ?? 1}
                        onChange={(event) =>
                          setWorkSource((current) =>
                            current ? { ...current, maxAttempts: Number(event.target.value) } : current,
                          )
                        }
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Poll seconds</Label>
                      <Input
                        type="number"
                        value={workSource.pollSeconds ?? 60}
                        onChange={(event) =>
                          setWorkSource((current) =>
                            current ? { ...current, pollSeconds: Number(event.target.value) } : current,
                          )
                        }
                      />
                    </div>
                  </div>
                  <label className="flex items-start gap-2 rounded-lg border p-3 text-xs">
                    <input
                      type="checkbox"
                      className="mt-0.5"
                      checked={Boolean(workSource.autoRun)}
                      onChange={(event) =>
                        setWorkSource((current) =>
                          current ? { ...current, autoRun: event.target.checked } : current,
                        )
                      }
                    />
                    <span>
                      <span className="font-medium">Auto-run</span>
                      <span className="mt-0.5 block text-muted-foreground">
                        Start pending queue items automatically after discovery.
                      </span>
                    </span>
                  </label>
                </div>
              ) : null}
            </div>
            <div className="space-y-2 rounded-lg border p-3">
              <div className="flex items-center justify-between">
                <Label>Multi-repo workspace</Label>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    setRepositories((current) => [
                      ...current,
                      { repository: "", path: "", primary: current.length === 0, connectionId: null },
                    ])
                  }
                >
                  <Plus /> Add repository
                </Button>
              </div>
              <p className="text-[11px] text-muted-foreground">
                Leave empty to use the single repository above. A single primary repository
                keeps the current root checkout layout.
              </p>
              {repositories.map((repository, index) => (
                <div key={index} className="grid gap-2 rounded-lg border p-2 sm:grid-cols-[1fr_1fr_auto_auto]">
                  <Input
                    value={repository.repository}
                    onChange={(event) =>
                      setRepositories((current) =>
                        current.map((item, i) => (i === index ? { ...item, repository: event.target.value } : item)),
                      )
                    }
                    placeholder="owner/repo"
                  />
                  <Input
                    value={repository.path ?? ""}
                    onChange={(event) =>
                      setRepositories((current) =>
                        current.map((item, i) => (i === index ? { ...item, path: event.target.value } : item)),
                      )
                    }
                    placeholder="workspace path (e.g. lib)"
                  />
                  <label className="flex items-center gap-1 text-xs">
                    <input
                      type="checkbox"
                      checked={Boolean(repository.primary)}
                      onChange={(event) =>
                        setRepositories((current) =>
                          current.map((item, i) => (i === index ? { ...item, primary: event.target.checked } : item)),
                        )
                      }
                    />
                    primary
                  </label>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    onClick={() => setRepositories((current) => current.filter((_, i) => i !== index))}
                  >
                    <X />
                  </Button>
                </div>
              ))}
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Worker profile (optional)</Label>
                <select
                  className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                  value={workerProfileId}
                  onChange={(event) => setWorkerProfileId(event.target.value)}
                >
                  <option value="">No automatic worker launch</option>
                  {(workerProfiles.data ?? []).filter((profile) => profile.enabled).map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name} · {profile.harnessKind} · {profile.target}
                    </option>
                  ))}
                </select>
              </div>
              <label className="flex items-start gap-2 rounded-lg border p-3 text-xs">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={autoLaunch}
                  onChange={(event) => setAutoLaunch(event.target.checked)}
                />
                <span>
                  <span className="font-medium">Auto-launch worker</span>
                  <span className="mt-0.5 block text-muted-foreground">
                    Starts the selected profile before the first step. The profile allowlist
                    must include the team agents.
                  </span>
                </span>
              </label>
            </div>
            <div className="space-y-1.5">
              <Label>Execution mode</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={mode}
                onChange={(event) => setMode(event.target.value as "pipeline" | "native")}
              >
                <option value="native">Native subagents — one OpenCode execution</option>
                <option value="pipeline">Pipeline — one execution per agent</option>
              </select>
              <p className="text-[11px] text-muted-foreground">
                Native mode runs the primary agent once and exposes later agents as OpenCode
                subagents. Pipeline mode gives every agent a separate ORBIT execution.
              </p>
              {nativeInvalid ? (
                <p className="text-[11px] text-destructive">
                  Native teams currently require an OpenCode primary agent. Pi and other
                  harnesses can use pipeline mode.
                </p>
              ) : null}
            </div>
            <div className="space-y-1.5">
              <Label>Team instruction / goal</Label>
              <Textarea
                className="min-h-20"
                value={instructions}
                onChange={(event) => setInstructions(event.target.value)}
                placeholder="Pick up issues labelled agent-ready, fix them in a loop, and stop when none remain."
              />
              <p className="text-[11px] text-muted-foreground">
                Passed to the primary agent and included in every later step with the previous output.
              </p>
            </div>
            <div className="space-y-1.5">
              <div className="flex items-center justify-between gap-2">
                <Label>Team skills</Label>
                <div className="flex gap-1">
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => setTeamSkills(skillOptions.map((skill) => skill.slug))}
                  >
                    Select all
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => setTeamSkills([])}
                  >
                    Clear
                  </Button>
                </div>
              </div>
              <div className="flex flex-wrap gap-2 rounded-lg border p-3">
                {skillOptions.length ? skillOptions.map((skill) => {
                  const checked = teamSkills.includes(skill.slug);
                  return (
                    <label key={skill.slug} className="flex items-center gap-1 text-xs">
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={(event) =>
                          setTeamSkills((current) =>
                            event.target.checked
                              ? [...current, skill.slug]
                              : current.filter((slug) => slug !== skill.slug),
                          )
                        }
                      />
                      {skill.name}
                    </label>
                  );
                }) : <span className="text-[11px] text-muted-foreground">No skills configured.</span>}
              </div>
            </div>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Agents in order</Label>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => setSteps((current) => [...current, { agentId: "" }])}
                >
                  <Plus /> Add agent
                </Button>
              </div>
              {steps.map((step, index) => (
                <div key={index} className="flex items-center gap-2">
                  <span className="w-16 text-[11px] text-muted-foreground">
                    {index === 0 ? "primary" : "subagent"}
                  </span>
                  <select
                    className="flex h-9 flex-1 rounded-lg border border-input bg-card px-3 text-sm"
                    value={step.agentId}
                    onChange={(event) =>
                      setSteps((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, agentId: event.target.value } : item,
                        ),
                      )
                    }
                  >
                    <option value="">Select agent…</option>
                    {agentOptions.map((agent) => (
                      <option key={agent.id} value={agent.id}>
                        {agent.name} · {agent.harness}
                      </option>
                    ))}
                  </select>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    disabled={steps.length <= 1}
                    onClick={() => setSteps((current) => current.filter((_, i) => i !== index))}
                  >
                    <X />
                  </Button>
                </div>
              ))}
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
            <Button onClick={() => save.mutate()} loading={save.isPending} disabled={!canCreate}>
              {editing ? "Save changes" : "Create agent team"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(runTeam)} onOpenChange={(next) => !next && setRunTeam(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Run {runTeam?.name}</DialogTitle>
            <DialogDescription>
              The primary agent receives this task. Later agents receive the previous output.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label>Task for this run</Label>
            <Textarea
              className="min-h-24"
              value={task}
              onChange={(event) => setTask(event.target.value)}
              placeholder={runTeam?.definition.instructions || "Team goal"}
            />
            <Label>Repository</Label>
            <Input
              value={repository}
              onChange={(event) => setRepository(event.target.value)}
              placeholder="owner/repo (optional if the connection sets defaultRepository)"
            />
            <label className="flex items-start gap-2 rounded-lg border border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10 p-3 text-xs">
              <input
                type="checkbox"
                className="mt-0.5 accent-[hsl(var(--primary))]"
                checked={autoApprove}
                onChange={(event) => setAutoApprove(event.target.checked)}
              />
              <span>
                <span className="font-medium">Auto-approve writes for this run (L4)</span>
                <span className="mt-0.5 block text-muted-foreground">
                  Bound GitHub writes execute without human approval. Capability allowlists
                  still apply. Use only for trusted repositories and tested agents.
                </span>
              </span>
            </label>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRunTeam(null)}>Cancel</Button>
            <Button
              onClick={() => run.mutate()}
              loading={run.isPending}
              disabled={!task.trim() && !runTeam?.definition.instructions}
            >
              Start run
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(queueTeam)} onOpenChange={(next) => !next && setQueueTeam(null)}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Queue · {queueTeam?.name}</DialogTitle>
            <DialogDescription>One line per item: externalKey|repository. Items are deduplicated by key.</DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <textarea
              className="min-h-24 w-full rounded-lg border bg-card p-2 font-mono text-xs"
              value={queueText}
              onChange={(event) => setQueueText(event.target.value)}
              placeholder={"opsifyhq/orbit#6|opsifyhq/orbit\nopsifyhq/orbit#7|opsifyhq/orbit"}
            />
            <div className="flex flex-wrap gap-2">
              <Button size="sm" onClick={() => void enqueueQueue()} loading={queueBusy}>
                <Plus /> Add to queue
              </Button>
              <Button size="sm" variant="outline" onClick={() => void runPendingQueue()} loading={queueBusy}>
                <Play /> Run pending
              </Button>
            </div>
            <div className="max-h-80 space-y-2 overflow-y-auto">
              {queueItems.length === 0 ? <p className="text-xs text-muted-foreground">Queue is empty.</p> : queueItems.map((item) => (
                <div key={item.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3">
                  <div>
                    <p className="font-mono text-xs">{item.externalKey}</p>
                    <p className="text-[11px] text-muted-foreground">{item.repository ?? "team repository"} · {titleCase(item.status)}</p>
                    {item.error ? <p className="text-[11px] text-destructive">{item.error}</p> : null}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button size="sm" variant="outline" disabled={item.status === "running" || queueBusy} onClick={() => void runQueueItem(item)}>
                      <Play /> Run
                    </Button>
                    {item.status === "failed" || item.status === "cancelled" ? (
                      <Button size="sm" variant="outline" disabled={queueBusy} onClick={() => void retryQueueItem(item)}>
                        Retry
                      </Button>
                    ) : null}
                    {item.status === "failed" || item.status === "cancelled" ? (
                      <Button size="sm" variant="ghost" disabled={queueBusy} onClick={() => void requeueQueueItem(item)}>
                        Requeue
                      </Button>
                    ) : null}
                    {item.status === "running" ? (
                      <Button size="sm" variant="ghost" className="text-destructive" disabled={queueBusy} onClick={() => void cancelQueueItem(item)}>
                        Cancel
                      </Button>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(detail)} onOpenChange={(next) => !next && setDetail(null)}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Agent team run</DialogTitle>
            <DialogDescription>{detail?.run.status}</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            {detail?.steps.map((step) => (
              <div key={step.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3">
                <div>
                  <p className="text-sm font-medium">
                    {step.stepIndex === 0 ? "primary · " : ""}
                    {step.name}
                  </p>
                  <p className="text-[11px] text-muted-foreground">
                    {step.skills.join(", ") || "no team skills"}
                  </p>
                  {step.error ? <p className="text-[11px] text-destructive">{step.error}</p> : null}
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant={step.status === "succeeded" ? "success" : step.status === "failed" ? "destructive" : "warning"}>
                    {step.status}
                  </Badge>
                  {step.executionId ? (
                    <a className="text-xs text-primary hover:underline" href={`/executions/${step.executionId}`}>
                      Execution
                    </a>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
