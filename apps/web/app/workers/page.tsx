"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Boxes, Copy, Plus, Server, Trash2 } from "lucide-react";
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api } from "@/lib/api";
import { copyText } from "@/lib/browser";
import {
  queryKeys,
  unwrap,
  useAgents,
  useProviders,
  useSkills,
  useWorkerProfiles,
} from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import type { WorkerBootstrap, WorkerProfile } from "@/lib/types";
import { relativeTime, titleCase } from "@/lib/utils";

const DEFAULT_IMAGES: Record<string, string> = {
  pi: "192.168.1.201:5000/orbit-worker-pi",
  opencode: "192.168.1.201:5000/orbit-worker-opencode",
  mock: "192.168.1.201:5000/orbit-worker",
};

export default function WorkersPage() {
  const profiles = useWorkerProfiles();
  const providers = useProviders();
  const agents = useAgents({ limit: 100 });
  const skills = useSkills();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [bootstrapProfile, setBootstrapProfile] = useState<WorkerProfile | null>(null);

  const [name, setName] = useState("Pi production");
  const [harness, setHarness] = useState("pi");
  const [target, setTarget] = useState("docker");
  const [driver, setDriver] = useState("process");
  const [image, setImage] = useState(DEFAULT_IMAGES.pi);
  const [tag, setTag] = useState("0.4.0");
  const [provider, setProvider] = useState("opencode-go");
  const [model, setModel] = useState("deepseek-v4-pro");
  const [authSecret, setAuthSecret] = useState("");
  const [agentIds, setAgentIds] = useState<string[]>([]);
  const [skillSlugs, setSkillSlugs] = useState<string[]>([]);

  const harnessProviders = (providers.data ?? []).filter((candidate) => {
    const config = (candidate.config ?? {}) as Record<string, unknown>;
    return candidate.kind === "harness_managed" && String(config.harness ?? "") === harness;
  });
  const selectedProvider = harnessProviders.find((candidate) => {
    const config = (candidate.config ?? {}) as Record<string, unknown>;
    return String(config.provider ?? "") === provider;
  });
  const providerModels = (() => {
    const settings = (selectedProvider?.config as Record<string, unknown> | undefined)?.settings as
      | Record<string, unknown>
      | undefined;
    return Array.isArray(settings?.models) ? (settings?.models as string[]) : [];
  })();

  const create = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST("/api/v1/worker-profiles", {
          body: {
            name,
            harnessKind: harness,
            target,
            executionDriver: driver,
            image,
            imageTag: tag,
            agentIds,
            skillSlugs,
            provider: provider || undefined,
            model: model || undefined,
            authSecretName: authSecret || undefined,
            env: {},
            resources: {},
          } as never,
        }),
      ),
    onSuccess: (profile) => {
      toast.success("Worker profile created");
      setOpen(false);
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerProfiles });
      setBootstrapProfile(profile as WorkerProfile);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const toggle = useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }) =>
      unwrap(
        await api.POST(
          enabled
            ? "/api/v1/worker-profiles/{profile_id}/enable"
            : "/api/v1/worker-profiles/{profile_id}/disable",
          { params: { path: { profile_id: id } } },
        ),
      ),
    onSuccess: () => {
      toast.success("Worker profile updated");
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerProfiles });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: async (id: string) =>
      unwrap(
        await api.DELETE("/api/v1/worker-profiles/{profile_id}", {
          params: { path: { profile_id: id } },
        }),
      ),
    onSuccess: () => {
      toast.success("Worker profile deleted");
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerProfiles });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Execution plane"
        title="Workers"
        description="Bootstrap profiles: one image, one target, a fixed set of agents, skills and harness auth. Generate Docker, Compose or Kubernetes output."
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus /> New worker profile
          </Button>
        }
      />

      {profiles.isLoading ? (
        <Skeleton className="h-40 rounded-xl" />
      ) : profiles.isError ? (
        <p className="text-sm text-destructive">{errorMessage(profiles.error)}</p>
      ) : !profiles.data?.length ? (
        <EmptyState
          icon={<Server className="h-6 w-6" />}
          title="No worker profiles"
          description="A profile pins the harness, image, target, agents, skills and auth secret. Workers start with ORBIT_WORKER_PROFILE_ID."
          action={
            <Button onClick={() => setOpen(true)}>
              <Plus /> Create profile
            </Button>
          }
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {profiles.data.map((profile) => (
            <Card key={profile.id}>
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle>{profile.name}</CardTitle>
                  <Badge variant={profile.enabled ? "success" : "outline"}>
                    {profile.enabled ? "enabled" : "disabled"}
                  </Badge>
                </div>
                <p className="text-[11px] text-muted-foreground">
                  {titleCase(profile.harnessKind)} · {profile.target} ·{" "}
                  {profile.executionDriver} · {profile.image}:{profile.imageTag}
                </p>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex flex-wrap gap-1 text-[11px]">
                  <Badge variant="secondary">{profile.agentIds.length} agents</Badge>
                  <Badge variant="secondary">{profile.skillSlugs.length} skills</Badge>
                  {profile.authSecretName ? (
                    <Badge variant="outline">auth: {profile.authSecretName}</Badge>
                  ) : null}
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" onClick={() => setBootstrapProfile(profile)}>
                    <Boxes /> Bootstrap
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => toggle.mutate({ id: profile.id, enabled: !profile.enabled })}
                  >
                    {profile.enabled ? "Disable" : "Enable"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-destructive"
                    disabled={profile.enabled}
                    title={profile.enabled ? "Disable the profile before deleting it" : undefined}
                    onClick={() => {
                      if (window.confirm(`Delete profile ${profile.name}?`)) remove.mutate(profile.id);
                    }}
                  >
                    <Trash2 /> Delete
                  </Button>
                </div>
                <p className="text-[10px] text-muted-foreground">
                  updated {relativeTime(profile.updatedAt)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <BootstrapDialog profile={bootstrapProfile} onClose={() => setBootstrapProfile(null)} />

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>New worker profile</DialogTitle>
            <DialogDescription>
              The worker loads this profile at startup, claims only its agents and
              materializes its skills. Secrets stay external.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5 sm:col-span-2">
              <Label>Name</Label>
              <Input value={name} onChange={(event) => setName(event.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label>Harness</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={harness}
                onChange={(event) => {
                  setHarness(event.target.value);
                  setImage(DEFAULT_IMAGES[event.target.value] ?? "");
                }}
              >
                <option value="pi">Pi</option>
                <option value="opencode">OpenCode</option>
                <option value="mock">Mock</option>
              </select>
            </div>
            <div className="space-y-1.5">
              <Label>Target</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={target}
                onChange={(event) => setTarget(event.target.value)}
              >
                <option value="docker">Docker</option>
                <option value="kubernetes">Kubernetes</option>
                <option value="local">Local</option>
              </select>
            </div>
            <div className="space-y-1.5">
              <Label>Execution driver</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={driver}
                onChange={(event) => setDriver(event.target.value)}
              >
                <option value="process">Process (long-lived worker)</option>
                <option value="docker">Docker (container per execution)</option>
              </select>
            </div>
            <div className="space-y-1.5">
              <Label>Image</Label>
              <Input value={image} onChange={(event) => setImage(event.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label>Tag</Label>
              <Input value={tag} onChange={(event) => setTag(event.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label>Provider</Label>
              {harnessProviders.length ? (
                <>
                  <select
                    className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                    value={selectedProvider ? provider : "__custom"}
                    onChange={(event) => {
                      if (event.target.value === "__custom") {
                        setProvider("");
                        setModel("");
                        return;
                      }
                      const next = harnessProviders.find((candidate) => {
                        const config = (candidate.config ?? {}) as Record<string, unknown>;
                        return String(config.provider ?? "") === event.target.value;
                      });
                      const config = (next?.config ?? {}) as Record<string, unknown>;
                      setProvider(event.target.value);
                      setModel(String(config.model ?? ""));
                    }}
                  >
                    <option value="__custom">Custom provider…</option>
                    {harnessProviders.map((candidate) => {
                      const config = (candidate.config ?? {}) as Record<string, unknown>;
                      const name = String(config.provider ?? candidate.name);
                      return (
                        <option key={candidate.id} value={name}>
                          {candidate.name} · {name}
                        </option>
                      );
                    })}
                  </select>
                  {selectedProvider ? null : (
                    <Input
                      value={provider}
                      placeholder="harness provider name, e.g. opencode-go"
                      onChange={(event) => setProvider(event.target.value)}
                    />
                  )}
                </>
              ) : (
                <Input
                  value={provider}
                  placeholder="opencode-go"
                  onChange={(event) => setProvider(event.target.value)}
                />
              )}
            </div>
            <div className="space-y-1.5">
              <Label>Model</Label>
              {providerModels.length ? (
                <select
                  className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                  value={model}
                  onChange={(event) => setModel(event.target.value)}
                >
                  {providerModels.includes(model) ? null : (
                    <option value={model}>{model || "Provider default"}</option>
                  )}
                  {providerModels.map((candidate) => (
                    <option key={candidate} value={candidate}>
                      {candidate}
                    </option>
                  ))}
                </select>
              ) : (
                <Input value={model} onChange={(event) => setModel(event.target.value)} />
              )}
              <p className="text-[11px] text-muted-foreground">
                {selectedProvider
                  ? "Default model from the selected provider; override if needed."
                  : "Free-form model id for custom or harness-default providers."}
              </p>
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <Label>Auth secret name (external)</Label>
              <Input
                value={authSecret}
                onChange={(event) => setAuthSecret(event.target.value)}
                placeholder="orbit-pi-auth or orbit-opencode-auth"
              />
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <Label>Agents ({agentIds.length} selected)</Label>
              <div className="max-h-40 space-y-1 overflow-y-auto rounded-lg border p-2">
                {(agents.data?.items ?? []).map((agent) => (
                  <label key={agent.id} className="flex items-center justify-between gap-2 text-xs">
                    <span className="truncate">{agent.name}</span>
                    <Switch
                      checked={agentIds.includes(agent.id)}
                      onCheckedChange={(checked) =>
                        setAgentIds((current) =>
                          checked ? [...current, agent.id] : current.filter((id) => id !== agent.id),
                        )
                      }
                    />
                  </label>
                ))}
              </div>
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <Label>Shared skills ({skillSlugs.length} selected)</Label>
              <div className="max-h-32 space-y-1 overflow-y-auto rounded-lg border p-2">
                {(skills.data ?? []).map((skill) => (
                  <label key={skill.id} className="flex items-center justify-between gap-2 text-xs">
                    <span className="truncate">{skill.name}</span>
                    <Switch
                      checked={skillSlugs.includes(skill.slug)}
                      onCheckedChange={(checked) =>
                        setSkillSlugs((current) =>
                          checked ? [...current, skill.slug] : current.filter((s) => s !== skill.slug),
                        )
                      }
                    />
                  </label>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => create.mutate()} loading={create.isPending} disabled={!name.trim()}>
              Create profile
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function BootstrapDialog({
  profile,
  onClose,
}: {
  profile: WorkerProfile | null;
  onClose: () => void;
}) {
  const bootstrap = useQuery<WorkerBootstrap>({
    queryKey: ["worker-bootstrap", profile?.id],
    queryFn: async () =>
      unwrap(
        await api.GET("/api/v1/worker-profiles/{profile_id}/bootstrap", {
          params: { path: { profile_id: profile!.id } },
        }),
      ),
    enabled: Boolean(profile),
  });

  async function copy(value: string) {
    if (await copyText(value)) toast.success("Copied");
    else toast.error("Copy unavailable");
  }

  return (
    <Dialog open={Boolean(profile)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>Bootstrap {profile?.name}</DialogTitle>
          <DialogDescription>
            Set <code className="font-mono">ORBIT_WORKER_PROFILE_ID</code> (included below).
            Provide database, NATS, master key and harness auth from your secret store.
          </DialogDescription>
        </DialogHeader>
        {bootstrap.isLoading ? (
          <Skeleton className="h-64" />
        ) : bootstrap.data ? (
          <Tabs defaultValue="docker">
            <TabsList>
              <TabsTrigger value="docker">Docker</TabsTrigger>
              <TabsTrigger value="compose">Compose</TabsTrigger>
              <TabsTrigger value="kubernetes">Kubernetes</TabsTrigger>
            </TabsList>
            {(["docker", "compose", "kubernetes"] as const).map((mode) => (
              <TabsContent key={mode} value={mode} className="space-y-2">
                <div className="flex justify-end">
                  <Button size="sm" variant="outline" onClick={() => copy(bootstrap.data![mode])}>
                    <Copy /> Copy
                  </Button>
                </div>
                <pre className="max-h-80 overflow-auto rounded-lg bg-muted/50 p-3 text-[10px] leading-relaxed">
                  {bootstrap.data[mode]}
                </pre>
              </TabsContent>
            ))}
            <ul className="list-inside list-disc text-[11px] text-muted-foreground">
              {bootstrap.data.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          </Tabs>
        ) : (
          <p className="text-sm text-destructive">{errorMessage(bootstrap.error)}</p>
        )}
      </DialogContent>
    </Dialog>
  );
}
