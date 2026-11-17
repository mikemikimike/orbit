"use client";

export const dynamic = 'force-dynamic';

import Link from "next/link";
import { useState } from "react";
import { ArrowLeft, ArrowRight, Bot, CheckCircle2, Clock3, Cpu, MapPin, Play, Plug, Server, Users, Zap } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/ui/page-header";
import { Textarea } from "@/components/ui/textarea";
import { useAgents, useAgentTeams, useConnectors, useHarnesses, useOutputs, useWorkerProfiles } from "@/lib/queries";
import { api } from "@/lib/api";
import { unwrap } from "@/lib/queries";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { errorMessage } from "@orbit/api-client";
import { titleCase } from "@/lib/utils";
import { defaultWorkerImage } from "@/lib/worker-images";
import type { WorkerProfile } from "@/lib/types";

const STEPS = [
  { key: "when", label: "When", description: "When should this work start?" },
  { key: "where", label: "Where", description: "Where should it run?" },
  { key: "who", label: "Who", description: "Who does the work?" },
  { key: "what", label: "What", description: "What should be done?" },
  { key: "then", label: "Then", description: "What happens after?" },
  { key: "summary", label: "Summary", description: "Review and create" },
] as const;

type StepKey = (typeof STEPS)[number]["key"];
type WhoType = "agent" | "team";

export default function WorkPage() {
  const [step, setStep] = useState<StepKey>("when");
  const [when, setWhen] = useState<"manual" | "schedule" | "event" | "connector">("manual");
  const [cron, setCron] = useState("*/30 * * * *");
  const [subject, setSubject] = useState("connector.telegram.message");
  const [connectorSlug, setConnectorSlug] = useState("");
  const [where, setWhere] = useState<"auto" | "local" | "docker" | "kubernetes">("auto");
  const [harness, setHarness] = useState("");
  const [whoType, setWhoType] = useState<WhoType>("agent");
  const [who, setWho] = useState("");
  const [name, setName] = useState("");
  const [task, setTask] = useState("");
  const [repository, setRepository] = useState("");
  const [autoApprove, setAutoApprove] = useState(false);
  const [longRunning, setLongRunning] = useState(false);

  const agents = useAgents({ status: "active", limit: 100 });
  const teams = useAgentTeams();
  const harnesses = useHarnesses();
  const workerProfiles = useWorkerProfiles();
  const connectors = useConnectors();
  const outputs = useOutputs();

  const currentIndex = STEPS.findIndex((s) => s.key === step);
  const canNext = (() => {
    if (step === "when") return true;
    if (step === "where") return true;
    if (step === "who") return Boolean(who);
    if (step === "what") return Boolean(task.trim());
    return true;
  })();

  function goNext() {
    const next = STEPS[currentIndex + 1];
    if (next) setStep(next.key);
  }

  function goPrev() {
    const prev = STEPS[currentIndex - 1];
    if (prev) setStep(prev.key);
  }

  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);

  async function createWork() {
    if (!who || !task.trim()) return;
    setCreating(true);
    try {
      if (whoType === "agent") {
        const result = unwrap(
          await api.POST("/api/v1/agents/{agent_id}/executions", {
            params: { path: { agent_id: who } },
            body: { task: task.trim(), payload: repository.trim() ? { repository: repository.trim() } : {}, harnessOverride: harness || undefined, name: name.trim() || undefined },
          }),
        );
        const effectiveHarness = harness || selectedAgent?.harness || "mock";
        const target = where === "kubernetes" ? "kubernetes" : where === "docker" ? "docker" : "local";

        // Reuse an existing profile for this harness, otherwise create and enable one.
        let profileId = (workerProfiles.data ?? []).find(
          (p) => p.harnessKind === effectiveHarness,
        )?.id;
        if (!profileId) {
          try {
            const created = unwrap(
              await api.POST("/api/v1/worker-profiles", {
                body: {
                  name: `Auto: ${effectiveHarness}`,
                  harnessKind: effectiveHarness,
                  target,
                  executionDriver: "process",
                  image: defaultWorkerImage(effectiveHarness),
                  imageTag: "latest",
                  agentIds: [who],
                  skillSlugs: [],
                  env: {},
                  resources: {},
                },
              }),
            );
            profileId = created.id;
            await api.POST("/api/v1/worker-profiles/{profile_id}/enable", {
              params: { path: { profile_id: created.id } },
            });
            toast.success("Work created — worker profile created");
          } catch (error) {
            toast.error(errorMessage(error));
          }
        }

        if (profileId) {
          try {
            await api.POST("/api/v1/worker-profiles/{profile_id}/launch", {
              params: { path: { profile_id: profileId } },
            });
            toast.success("Work created — worker launched");
          } catch (error) {
            toast.error(errorMessage(error));
          }
        }
        window.location.href = `/executions/${result.id}`;
      } else {
        const { startTeamRun } = await import("@/lib/teams");
        const result = await startTeamRun(who, task.trim(), repository.trim() || undefined, false, harness || undefined);
        toast.success("Team run created");
        window.location.href = `/agent-teams?run=${result.run.id}`;
      }
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setCreating(false);
    }
  }

  const selectedAgent = agents.data?.items.find((a) => a.id === who);
  const visibleOutputs = (outputs.data ?? []).filter(
    (output) =>
      output.enabled &&
      (!output.agentId || (whoType === "agent" && output.agentId === who)),
  );

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Work"
        title="Create work"
        description="Walk through when, where, who, what and then to define a complete piece of work."
      />

      <div className="flex items-center gap-2 overflow-x-auto pb-1">
        {STEPS.map((s, index) => (
          <button
            key={s.key}
            type="button"
            onClick={() => setStep(s.key)}
            className={`flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
              step === s.key
                ? "bg-primary text-primary-foreground"
                : index < currentIndex
                  ? "bg-primary/10 text-primary"
                  : "bg-muted text-muted-foreground"
            }`}
          >
            {s.key === "when" ? <Clock3 className="h-4 w-4" /> : null}
            {s.key === "where" ? <MapPin className="h-4 w-4" /> : null}
            {s.key === "who" ? <Bot className="h-4 w-4" /> : null}
            {s.key === "what" ? <Zap className="h-4 w-4" /> : null}
            {s.key === "then" ? <CheckCircle2 className="h-4 w-4" /> : null}
            <span className="flex h-5 w-5 items-center justify-center rounded-full bg-background/20 text-[11px]">
              {index + 1}
            </span>
            {s.label}
          </button>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {step === "when" ? <Clock3 className="h-4 w-4 text-primary" /> : null}
            {step === "where" ? <MapPin className="h-4 w-4 text-primary" /> : null}
            {step === "who" ? <Bot className="h-4 w-4 text-primary" /> : null}
            {step === "what" ? <Zap className="h-4 w-4 text-primary" /> : null}
            {step === "then" ? <CheckCircle2 className="h-4 w-4 text-primary" /> : null}
            {STEPS[currentIndex].label}
          </CardTitle>
          <p className="text-sm text-muted-foreground">{STEPS[currentIndex].description}</p>
        </CardHeader>
        <CardContent className="space-y-4">
          {step === "when" ? (
            <div className="space-y-3">
              <Label>Trigger type</Label>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                {([
                  { value: "manual", label: "Manual", desc: "Run on demand", icon: Play },
                  { value: "schedule", label: "Schedule", desc: "Run on a cron schedule", icon: Clock3 },
                  { value: "event", label: "Event", desc: "Run when an event arrives", icon: Zap },
                  { value: "connector", label: "Connector", desc: "Triggered by a connector", icon: Plug },
                ] as const).map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => setWhen(opt.value)}
                    className={`rounded-lg border p-3 text-left text-sm ${
                      when === opt.value ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                    }`}
                  >
                    <opt.icon className="mb-1 h-4 w-4 text-primary" />
                    <p className="font-medium">{opt.label}</p>
                    <p className="text-[11px] text-muted-foreground">{opt.desc}</p>
                  </button>
                ))}
              </div>
              {when === "schedule" ? (
                <div className="space-y-1.5">
                  <Label>Cron (UTC, 5 fields)</Label>
                  <Input value={cron} onChange={(e) => setCron(e.target.value)} placeholder="*/30 * * * *" />
                </div>
              ) : null}
              {when === "event" ? (
                <div className="space-y-1.5">
                  <Label>Event subject</Label>
                  <Input value={subject} onChange={(e) => setSubject(e.target.value)} placeholder="connector.telegram.message" />
                </div>
              ) : null}
              {when === "connector" ? (
                <div className="space-y-1.5">
                  <Label>Connector</Label>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {(connectors.data ?? []).map((c) => (
                      <button
                        key={c.slug}
                        type="button"
                        onClick={() => setConnectorSlug(c.slug)}
                        className={`flex items-center gap-2 rounded-lg border p-3 text-left text-sm ${
                          connectorSlug === c.slug ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                        }`}
                      >
                        <Plug className="h-4 w-4 text-primary" />
                        <div>
                          <p className="font-medium">{c.name}</p>
                          <p className="text-[11px] text-muted-foreground">
                            {(Array.isArray(c.capabilities) ? c.capabilities.length : 0)} capabilities
                          </p>
                        </div>
                      </button>
                    ))}
                  </div>
                </div>
              ) : null}
            </div>
          ) : null}

          {step === "where" ? (
            <div className="space-y-3">
              <Label>Placement</Label>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                {([
                  { value: "auto", label: "Auto", desc: "Any matching worker", icon: Zap },
                  { value: "local", label: "Local", desc: "Local host worker", icon: Server },
                  { value: "docker", label: "Docker", desc: "Docker worker", icon: Server },
                  { value: "kubernetes", label: "Kubernetes", desc: "Kubernetes worker", icon: Server },
                ] as const).map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => setWhere(opt.value)}
                    className={`rounded-lg border p-3 text-left text-sm ${
                      where === opt.value ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                    }`}
                  >
                    <opt.icon className="mb-1 h-4 w-4 text-primary" />
                    <p className="font-medium">{opt.label}</p>
                    <p className="text-[11px] text-muted-foreground">{opt.desc}</p>
                  </button>
                ))}
              </div>
              <div className="space-y-1.5">
                <Label>Harness</Label>
                <div className="grid gap-2 sm:grid-cols-2">
                  {(harnesses.data ?? []).map((h) => (
                    <button
                      key={h.kind}
                      type="button"
                      onClick={() => setHarness(h.kind)}
                      className={`rounded-lg border p-3 text-left text-sm ${
                        harness === h.kind ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                      }`}
                    >
                      <p className="font-medium">{h.name}</p>
                      <p className="text-[11px] text-muted-foreground">{h.implementationStatus}</p>
                    </button>
                  ))}
                </div>
              </div>

              {harness ? (
                <div className="space-y-1.5">
                  <Label>Worker profile for {harness}</Label>
                  {(workerProfiles.data ?? []).filter((p) => p.harnessKind === harness).length ? (
                    <div className="grid gap-2 sm:grid-cols-2">
                      {(workerProfiles.data ?? [])
                        .filter((p) => p.harnessKind === harness)
                        .map((p) => (
                          <div
                            key={p.id}
                            className={`rounded-lg border p-3 text-sm ${
                              p.enabled ? "border-primary/40" : "opacity-60"
                            }`}
                          >
                            <p className="font-medium">{p.name}</p>
                            <p className="text-[11px] text-muted-foreground">
                              {p.enabled ? "enabled" : "disabled"} · {p.target} · {p.agentIds.length} agents
                            </p>
                          </div>
                        ))}
                    </div>
                  ) : (
                    <div className="rounded-lg border border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/10 p-3 text-sm">
                      <p className="font-medium">No worker profile for {harness}</p>
                      <p className="mt-0.5 text-[11px] text-muted-foreground">
                        A worker profile defines the image, auth, and resources for this harness.
                      </p>
                      <Link
                        href={`/workers?create=1&harness=${harness}&placement=${where}`}
                        className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
                      >
                        Create worker profile →
                      </Link>
                    </div>
                  )}
                </div>
              ) : null}
            </div>
          ) : null}

          {step === "who" ? (
            <div className="space-y-3">
              <Label>Executor type</Label>
              <div className="grid gap-2 sm:grid-cols-2">
                <button
                  type="button"
                  onClick={() => { setWhoType("agent"); setWho(""); }}
                  className={`rounded-lg border p-3 text-left text-sm ${
                    whoType === "agent" ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                  }`}
                >
                  <p className="font-medium">Agent</p>
                  <p className="text-[11px] text-muted-foreground">A single persistent agent</p>
                </button>
                <button
                  type="button"
                  onClick={() => { setWhoType("team"); setWho(""); }}
                  className={`rounded-lg border p-3 text-left text-sm ${
                    whoType === "team" ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                  }`}
                >
                  <p className="font-medium">Agent Team</p>
                  <p className="text-[11px] text-muted-foreground">A coordinated team of agents</p>
                </button>
              </div>

              <Label>Select {whoType === "agent" ? "agent" : "team"}</Label>
              {whoType === "agent" ? (
                <>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {(agents.data?.items ?? [])
                      .filter((agent) => !harness || agent.harness === harness)
                      .map((agent) => (
                      <button
                        key={agent.id}
                        type="button"
                        onClick={() => setWho(agent.id)}
                        className={`rounded-lg border p-3 text-left text-sm ${
                          who === agent.id ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                        }`}
                      >
                        <p className="font-medium">{agent.name}</p>
                        <p className="text-[11px] text-muted-foreground">
                          {titleCase(agent.harness ?? "")} · Autonomy L{agent.autonomyLevel}
                        </p>
                      </button>
                    ))}
                  </div>
                  {whoType === "agent" && harness && !(agents.data?.items ?? []).some((a) => a.harness === harness) ? (
                    <p className="text-sm text-muted-foreground">
                      No agents use the {harness} harness.{" "}
                      <Link href="/agents/new" className="text-primary hover:underline">
                        Create one
                      </Link>{" "}
                      or go back and change the harness.
                    </p>
                  ) : null}
                </>
              ) : (
                <div className="grid gap-2 sm:grid-cols-2">
                  {(teams.data ?? []).map((team) => (
                    <button
                      key={team.id}
                      type="button"
                      onClick={() => setWho(team.id)}
                      className={`rounded-lg border p-3 text-left text-sm ${
                        who === team.id ? "border-primary bg-accent/60" : "hover:bg-muted/50"
                      }`}
                    >
                      <p className="font-medium">{team.name}</p>
                      <p className="text-[11px] text-muted-foreground">
                        {team.definition.mode ?? "pipeline"} ·{" "}
                        {team.definition.steps?.length ?? 0} agents
                      </p>
                    </button>
                  ))}
                  {!(teams.data ?? []).length ? (
                    <p className="text-sm text-muted-foreground">
                      No agent teams yet.{" "}
                      <Link href="/agent-teams" className="text-primary hover:underline">
                        Create one first
                      </Link>
                      .
                    </p>
                  ) : null}
                </div>
              )}
              {whoType === "agent" && !agents.data?.items.length ? (
                <p className="text-sm text-muted-foreground">
                  No active agents.{" "}
                  <Link href="/agents/new" className="text-primary hover:underline">
                    Create one first
                  </Link>
                  .
                </p>
              ) : null}
            </div>
          ) : null}

          {step === "what" ? (
            <div className="space-y-3">
              <div className="space-y-1.5">
                <Label>Name (optional)</Label>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Evaluate article X"
                />
              </div>
              <div className="space-y-1.5">
                <Label>Task</Label>
                <Textarea
                  className="min-h-[120px]"
                  value={task}
                  onChange={(e) => setTask(e.target.value)}
                  placeholder="Describe what the agent should do…"
                />
              </div>
              <div className="space-y-1.5">
                <Label>Repository (optional)</Label>
                <Input
                  value={repository}
                  onChange={(e) => setRepository(e.target.value)}
                  placeholder="owner/repo"
                />
              </div>
            </div>
          ) : null}

          {step === "then" ? (
            <div className="space-y-3">
              <label className="flex items-start gap-2 rounded-lg border p-3 text-sm">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={autoApprove}
                  onChange={(e) => setAutoApprove(e.target.checked)}
                />
                <span>
                  <span className="font-medium">Auto-approve writes</span>
                  <span className="mt-0.5 block text-[11px] text-muted-foreground">
                    Bound writes execute without human approval. Capability allowlists still apply.
                  </span>
                </span>
              </label>
              <label className="flex items-start gap-2 rounded-lg border p-3 text-sm">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={longRunning}
                  onChange={(e) => setLongRunning(e.target.checked)}
                />
                <span>
                  <span className="font-medium">24/7 agent</span>
                  <span className="mt-0.5 block text-[11px] text-muted-foreground">
                    Set execution mode to long-running (24h timeout). The agent stays
                    alive between triggers. Requires a harness that supports instances.
                  </span>
                </span>
              </label>

              <div className="rounded-lg border p-3 text-sm">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">Outputs</span>
                  <Link href="/outputs" className="text-[11px] text-primary hover:underline">
                    Manage outputs
                  </Link>
                </div>
                {outputs.isLoading ? (
                  <p className="mt-2 text-[11px] text-muted-foreground">Loading outputs…</p>
                ) : visibleOutputs.length ? (
                  <ul className="mt-2 space-y-1 text-[11px] text-muted-foreground">
                    {visibleOutputs.map((output) => (
                      <li key={output.id} className="flex items-center justify-between gap-2">
                        <span className="truncate">{output.name}</span>
                        <span className="shrink-0">
                          {output.kind} · {output.events.join(", ")}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="mt-2 text-[11px] text-muted-foreground">
                    No outputs configured for this {whoType === "agent" ? "agent" : "team"}.
                    Add Slack, Mattermost or webhook outputs under Outputs.
                  </p>
                )}
              </div>
            </div>
          ) : null}

          {step === "summary" ? (
            <div className="rounded-xl border bg-gradient-to-br from-primary/5 to-muted/30 p-5">
              <p className="mb-4 text-base font-semibold">Summary</p>
              <div className="space-y-3">
                <SummaryRow icon={<Clock3 className="h-4 w-4" />} label="When" value={when === "manual" ? "Manual" : when === "schedule" ? `Schedule (${cron})` : when === "event" ? `Event (${subject})` : `Connector (${connectorSlug || "—"})`} />
                <SummaryRow icon={<MapPin className="h-4 w-4" />} label="Where" value={`${titleCase(where)}${harness ? ` · ${harness}` : ""}`} />
                <SummaryRow icon={<Bot className="h-4 w-4" />} label="Who" value={whoType === "agent" ? (selectedAgent?.name ?? "—") : (teams.data?.find((t) => t.id === who)?.name ?? "—")} />
                <SummaryRow icon={<Zap className="h-4 w-4" />} label="What" value={name.trim() || task.slice(0, 60) || "—"} />
                <SummaryRow icon={<CheckCircle2 className="h-4 w-4" />} label="Then" value={`${autoApprove ? "Auto-approve writes" : "Require approval"}${longRunning ? " · 24/7 agent" : ""}${visibleOutputs.length ? ` · ${visibleOutputs.length} output(s)` : ""}`} />
              </div>
            </div>
          ) : null}
        </CardContent>
      </Card>

      <div className="flex items-center justify-between">
        <Button variant="ghost" onClick={goPrev} disabled={currentIndex === 0}>
          <ArrowLeft /> Back
        </Button>
        {currentIndex < STEPS.length - 1 ? (
          <Button onClick={goNext} disabled={!canNext}>
            Next <ArrowRight />
          </Button>
        ) : (
          <Button disabled={!who || !task.trim() || creating} loading={creating} onClick={() => void createWork()}>
            <Play /> Create work
          </Button>
        )}
      </div>
    </div>
  );
}


function SummaryRow({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="flex items-center gap-3 rounded-lg bg-background/60 px-4 py-3">
      <span className="flex h-8 w-8 items-center justify-center rounded-md bg-primary/10 text-primary">
        {icon}
      </span>
      <span className="w-16 text-sm font-medium text-muted-foreground">{label}</span>
      <span className="flex-1 text-sm font-medium">{value}</span>
    </div>
  );
}
