"use client";

import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronRight, Plus, Play, Trash2, Users } from "lucide-react";
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
import {
  queryKeys,
  unwrap,
  useAgents,
  useAgentTeams,
  useSkills,
  useTeamRuns,
} from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import {
  createAgentTeam,
  deleteAgentTeam,
  getTeamRun,
  startTeamRun,
  type TeamStepInput,
} from "@/lib/teams";
import type { AgentTeam, TeamRunDetail } from "@/lib/types";
import { relativeTime } from "@/lib/utils";

export default function TeamsPage() {
  const teams = useAgentTeams();
  const runs = useTeamRuns();
  const agents = useAgents({ limit: 100 });
  const skills = useSkills();
  const queryClient = useQueryClient();

  const [open, setOpen] = useState(false);
  const [name, setName] = useState("Coding team");
  const [description, setDescription] = useState("Architect, builder, tester and reviewer.");
  const [steps, setSteps] = useState<TeamStepInput[]>([
    { name: "architect", agentId: "", skills: [] },
    { name: "builder", agentId: "", skills: [] },
    { name: "tester", agentId: "", skills: [] },
    { name: "reviewer", agentId: "", skills: [] },
  ]);
  const [runTeam, setRunTeam] = useState<AgentTeam | null>(null);
  const [task, setTask] = useState("Pick up the next ready GitHub issue and implement it.");
  const [detail, setDetail] = useState<TeamRunDetail | null>(null);

  const agentOptions = agents.data?.items ?? [];
  const skillOptions = skills.data ?? [];
  const canCreate = name.trim() && steps.every((step) => step.name.trim() && step.agentId);

  const create = useMutation({
    mutationFn: () =>
      createAgentTeam({ name: name.trim(), description, steps }),
    onSuccess: () => {
      toast.success("Team created");
      setOpen(false);
      void queryClient.invalidateQueries({ queryKey: queryKeys.agentTeams });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: deleteAgentTeam,
    onSuccess: () => {
      toast.success("Team deleted");
      void queryClient.invalidateQueries({ queryKey: queryKeys.agentTeams });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const run = useMutation({
    mutationFn: () => startTeamRun(runTeam!.id, task.trim()),
    onSuccess: (result) => {
      toast.success("Team run started");
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

  const stepLabel = useMemo(() => {
    const map = new Map(agentOptions.map((agent) => [agent.id, `${agent.name} (${agent.harness})`]));
    return (id: string) => map.get(id) ?? id.slice(0, 8);
  }, [agentOptions]);

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Orchestration"
        title="Teams"
        description="Sequential role handoffs. Every step is a normal execution with its own agent, skills, connectors and approvals."
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus /> New team
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
          description="Create an architect → builder → tester → reviewer team."
          action={<Button onClick={() => setOpen(true)}><Plus /> Create team</Button>}
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {teams.data.map((team) => {
            const definitionSteps = team.definition.steps ?? [];
            return (
              <Card key={team.id}>
                <CardHeader className="pb-2">
                  <CardTitle className="truncate">{team.name}</CardTitle>
                  <p className="text-[11px] text-muted-foreground">
                    {definitionSteps.length} steps · updated {relativeTime(team.updatedAt)}
                  </p>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex flex-wrap gap-1">
                    {definitionSteps.map((step) => (
                      <Badge key={step.name} variant="secondary">
                        {step.name}
                      </Badge>
                    ))}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      onClick={() => {
                        setRunTeam(team);
                        setTask("Pick up the next ready GitHub issue and implement it.");
                      }}
                    >
                      <Play /> Run
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Delete team ${team.name}?`)) remove.mutate(team.id);
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
        <h2 className="text-sm font-semibold">Team runs</h2>
        {runs.isLoading ? (
          <Skeleton className="h-20 rounded-xl" />
        ) : !runs.data?.length ? (
          <p className="rounded-xl border border-dashed p-4 text-xs text-muted-foreground">
            No team runs yet.
          </p>
        ) : (
          <div className="space-y-2">
            {runs.data.map((item) => (
              <Card key={item.id}>
                <CardContent className="flex flex-wrap items-center justify-between gap-3 p-4">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">
                      {String(item.input.task ?? "Team run")}
                    </p>
                    <p className="text-[11px] text-muted-foreground">
                      {relativeTime(item.createdAt)} · {item.status}
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
            <DialogTitle>New agent team</DialogTitle>
            <DialogDescription>
              Steps run in order. The previous step output is passed as context to the next.
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
            {steps.map((step, index) => (
              <div key={index} className="rounded-lg border p-3">
                <div className="grid gap-2 sm:grid-cols-2">
                  <div className="space-y-1">
                    <Label>Role</Label>
                    <Input
                      value={step.name}
                      onChange={(event) =>
                        setSteps((current) =>
                          current.map((item, i) =>
                            i === index ? { ...item, name: event.target.value } : item,
                          ),
                        )
                      }
                    />
                  </div>
                  <div className="space-y-1">
                    <Label>Agent</Label>
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
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
                          {agent.name} ({agent.harness})
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
                <div className="mt-2 space-y-1">
                  <Label>Skills for this step</Label>
                  <div className="flex flex-wrap gap-2">
                    {skillOptions.map((skill) => {
                      const checked = step.skills.includes(skill.slug);
                      return (
                        <label key={skill.slug} className="flex items-center gap-1 text-xs">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) =>
                              setSteps((current) =>
                                current.map((item, i) =>
                                  i === index
                                    ? {
                                        ...item,
                                        skills: event.target.checked
                                          ? [...item.skills, skill.slug]
                                          : item.skills.filter((slug) => slug !== skill.slug),
                                      }
                                    : item,
                                ),
                              )
                            }
                          />
                          {skill.name}
                        </label>
                      );
                    })}
                  </div>
                </div>
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
            <Button
              onClick={() => create.mutate()}
              loading={create.isPending}
              disabled={!canCreate}
            >
              Create team
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(runTeam)} onOpenChange={(next) => !next && setRunTeam(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Run {runTeam?.name}</DialogTitle>
            <DialogDescription>Starts the first role; the rest run after each success.</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label>Task</Label>
            <Input value={task} onChange={(event) => setTask(event.target.value)} />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRunTeam(null)}>Cancel</Button>
            <Button onClick={() => run.mutate()} loading={run.isPending} disabled={!task.trim()}>
              Start run
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(detail)} onOpenChange={(next) => !next && setDetail(null)}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Team run</DialogTitle>
            <DialogDescription>{detail?.run.status}</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            {detail?.steps.map((step) => (
              <div key={step.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3">
                <div>
                  <p className="text-sm font-medium">
                    {step.stepIndex + 1}. {step.name}
                  </p>
                  <p className="text-[11px] text-muted-foreground">
                    {stepLabel(step.agentId)} · {step.skills.join(", ") || "agent skills"}
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
