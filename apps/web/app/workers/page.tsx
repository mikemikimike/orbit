"use client";

import { useEffect, useState, Suspense } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Boxes, Copy, Pencil, Plus, RotateCw, Rocket, ScrollText, Server, Square, Trash2 } from "lucide-react";
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
import { ExportResourceButton } from "@/components/export-resource-button";
import { ResourceAccessFields } from "@/components/resource-access-fields";
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
  useWorkerLaunches,
  useConnections,
  useEnvMappings,
  useMeta,
  useWorkerProfileEnvMappings,
  useWorkerProfiles,
} from "@/lib/queries";
import {
  launchWorkerProfile,
  stopWorkerLaunch,
  workerLaunchLogs,
} from "@/lib/launcher";
import { errorMessage } from "@orbit/api-client";
import type { WorkerBootstrap, WorkerProfile } from "@/lib/types";
import { relativeTime, titleCase } from "@/lib/utils";
import { DEFAULT_WORKER_IMAGES } from "@/lib/worker-images";

const DEFAULT_IMAGES = DEFAULT_WORKER_IMAGES;

function WorkersPageInner() {
  const profiles = useWorkerProfiles();
  const [showInactive, setShowInactive] = useState(false);
  const launches = useWorkerLaunches(showInactive);
  const providers = useProviders();
  const agents = useAgents({ limit: 100 });
  const skills = useSkills();
  const connections = useConnections();
  const meta = useMeta();
  const platformVersion = meta.data?.version;
  const queryClient = useQueryClient();
  const searchParams = useSearchParams();
  const [open, setOpen] = useState(false);
  const [prefilled, setPrefilled] = useState(false);
  const [editing, setEditing] = useState<WorkerProfile | null>(null);
  const [bootstrapProfile, setBootstrapProfile] = useState<WorkerProfile | null>(null);
  const [logsLaunch, setLogsLaunch] = useState<{ id: string; name: string } | null>(null);
  const [logs, setLogs] = useState("");
  const [logsLoading, setLogsLoading] = useState(false);
  const [logsOpen, setLogsOpen] = useState(false);

  const [name, setName] = useState("OpenCode production");
  const [harness, setHarness] = useState("opencode");
  const [target, setTarget] = useState("docker");
  const [driver, setDriver] = useState("process");
  const [image, setImage] = useState(DEFAULT_IMAGES.opencode);
  const [tag, setTag] = useState("");
  const [provider, setProvider] = useState("opencode-go");
  const [model, setModel] = useState("deepseek-v4-pro");
  const [authSecret, setAuthSecret] = useState("");
  const [envMappings, setEnvMappings] = useState<Array<{ envVar: string; connectionId: string; secretKey: string }>>([]);
  const [envMappingIds, setEnvMappingIds] = useState<string[]>([]);
  const [agentScope, setAgentScope] = useState<"all" | "selected">("all");
  const [agentIds, setAgentIds] = useState<string[]>([]);
  const [skillSlugs, setSkillSlugs] = useState<string[]>([]);
  const [cpuRequest, setCpuRequest] = useState("100m");
  const [memoryRequest, setMemoryRequest] = useState("256Mi");
  const [cpuLimit, setCpuLimit] = useState("1");
  const [memoryLimit, setMemoryLimit] = useState("1Gi");
  const [workspaceSize, setWorkspaceSize] = useState("1Gi");
  const [allowWebSearch, setAllowWebSearch] = useState(false);
  const [allowWebFetch, setAllowWebFetch] = useState(false);
  const [allowGlab, setAllowGlab] = useState(false);
  const [allowGh, setAllowGh] = useState(false);
  const [nativeToolsText, setNativeToolsText] = useState("{}");
  const [confirmRelaunch, setConfirmRelaunch] = useState(false);

  // Pre-fill from Work wizard query params.
  if (searchParams.get("create") === "1" && !prefilled) {
    setPrefilled(true);
    const harnessParam = searchParams.get("harness");
    const placementParam = searchParams.get("placement");
    if (harnessParam) setHarness(harnessParam);
    if (placementParam) setTarget(placementParam === "kubernetes" ? "kubernetes" : placementParam === "docker" ? "docker" : "local");
    setOpen(true);
  }

  const providerOptionValue = (candidate: NonNullable<typeof providers.data>[number]) => {
    const config = (candidate.config ?? {}) as Record<string, unknown>;
    return candidate.kind === "harness_managed"
      ? String(config.provider ?? candidate.name)
      : candidate.name;
  };
  const workerProviders = providers.data ?? [];
  const selectedProvider = workerProviders.find(
    (candidate) => providerOptionValue(candidate) === provider,
  );
  const providerModels = (() => {
    const settings = (selectedProvider?.config as Record<string, unknown> | undefined)?.settings as
      | Record<string, unknown>
      | undefined;
    return Array.isArray(settings?.models) ? (settings?.models as string[]) : [];
  })();

  const envMappingLibrary = useEnvMappings();
  const profileEnvMappings = useWorkerProfileEnvMappings(editing?.id);
  useEffect(() => {
    if (editing && profileEnvMappings.data) {
      setEnvMappingIds(profileEnvMappings.data.map((mapping) => mapping.id));
    }
  }, [editing, profileEnvMappings.data]);

  const activeProfileLaunches = (launches.data ?? []).filter(
    (item) => editing && item.profileId === editing.id &&
      ["pending", "starting", "running"].includes(item.status),
  );

  function restartFieldsChanged() {
    if (!editing) return false;
    const existingResources = ((editing as WorkerProfile & { resources?: Record<string, unknown> }).resources ?? {}) as {
      requests?: { cpu?: string; memory?: string };
      limits?: { cpu?: string; memory?: string };
      workspace?: { sizeLimit?: string };
    };
    return (
      editing.image !== image ||
      editing.imageTag !== tag ||
      editing.target !== target ||
      editing.executionDriver !== driver ||
      editing.harnessKind !== harness ||
      (editing.authSecretName ?? "") !== authSecret ||
      (editing.provider ?? "") !== (provider || "") ||
      (editing.model ?? "") !== (model || "") ||
      JSON.stringify(existingResources) !==
        JSON.stringify({
          requests: { cpu: cpuRequest || undefined, memory: memoryRequest || undefined },
          limits: { cpu: cpuLimit || undefined, memory: memoryLimit || undefined },
          workspace: { sizeLimit: workspaceSize || undefined },
        })
    );
  }

  function requestSave() {
    if (editing && activeProfileLaunches.length && restartFieldsChanged()) {
      setConfirmRelaunch(true);
      return;
    }
    save.mutate(false);
  }

  const save = useMutation({
    mutationFn: async (applyNow: boolean) => {
      const body = {
        name,
        harnessKind: harness,
        target,
        executionDriver: driver,
        image,
        imageTag: tag,
        agentScope,
        agentIds: agentScope === "selected" ? agentIds : [],
        skillSlugs,
        provider: provider || undefined,
        model: model || undefined,
        authSecretName: authSecret || undefined,
        env: editing?.env ?? {},
        envMappings: envMappings.map((m) => ({
          envVar: m.envVar,
          connectionId: m.connectionId || null,
          secretKey: m.secretKey || null,
        })),
        envMappingIds,
        allowWebSearch,
        allowWebFetch,
        allowGlab,
        allowGh,
        nativeToolPermissions: JSON.parse(nativeToolsText || "{}"),
        resources: {
          requests: { cpu: cpuRequest || undefined, memory: memoryRequest || undefined },
          limits: { cpu: cpuLimit || undefined, memory: memoryLimit || undefined },
          workspace: { sizeLimit: workspaceSize || undefined },
        },
      };
      if (editing) {
        const profile = unwrap(
          await api.PATCH("/api/v1/worker-profiles/{profile_id}", {
            params: { path: { profile_id: editing.id } },
            body,
          }),
        );
        if (applyNow) {
          for (const item of activeProfileLaunches) {
            await stopWorkerLaunch(item.id);
          }
          await launchWorkerProfile(editing.id);
        }
        return profile;
      }
      return unwrap(await api.POST("/api/v1/worker-profiles", { body }));
    },
    onSuccess: (profile, applyNow) => {
      if (applyNow) {
        toast.success("Worker profile updated and relaunched");
      } else {
        toast.success(
          editing ? "Worker profile updated" : "Worker profile created",
          editing && activeProfileLaunches.length && restartFieldsChanged()
            ? { description: "Relaunch the worker to apply the new configuration." }
            : undefined,
        );
      }
      setOpen(false);
      setEditing(null);
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerProfiles });
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerLaunches });
      if (!editing) setBootstrapProfile(profile as WorkerProfile);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setEditing(null);
    setName("OpenCode production");
    setHarness("opencode");
    setTarget("docker");
    setDriver("process");
    setImage(DEFAULT_IMAGES.opencode);
    setTag("");
    setProvider("opencode-go");
    setModel("deepseek-v4-pro");
    setAuthSecret("");
    setEnvMappings([]);
    setEnvMappingIds([]);
    setAgentScope("all");
    setAgentIds([]);
    setSkillSlugs([]);
    setCpuRequest("100m");
    setMemoryRequest("256Mi");
    setCpuLimit("1");
    setMemoryLimit("1Gi");
    setWorkspaceSize("1Gi");
    setAllowWebSearch(false);
    setAllowWebFetch(false);
    setAllowGlab(false);
    setAllowGh(false);
    setNativeToolsText("{}");
    setOpen(true);
  }

  function openEdit(profile: WorkerProfile) {
    setEditing(profile);
    setName(profile.name);
    setHarness(profile.harnessKind);
    setTarget(profile.target);
    setDriver(profile.executionDriver);
    setImage(profile.image);
    setTag(profile.imageTag);
    setProvider(profile.provider ?? "");
    setModel(profile.model ?? "");
    setAuthSecret(profile.authSecretName ?? "");
    const mappings = (profile as { envMappings?: Array<{ envVar: string; connectionId?: string; secretKey?: string }> }).envMappings ?? [];
    setEnvMappings(mappings.map((m) => ({ envVar: m.envVar ?? "", connectionId: m.connectionId ?? "", secretKey: m.secretKey ?? "" })));
    const scoped = (profile as WorkerProfile & { agentScope?: string }).agentScope === "selected";
    setAgentScope(scoped ? "selected" : "all");
    setEnvMappingIds([]);
    const tools = profile as unknown as { allowWebSearch?: boolean; allowWebFetch?: boolean };
    setAllowWebSearch(tools.allowWebSearch ?? false);
    setAllowWebFetch(tools.allowWebFetch ?? false);
    setAllowGlab((profile as unknown as { allowGlab?: boolean }).allowGlab ?? false);
    setAllowGh((profile as unknown as { allowGh?: boolean }).allowGh ?? false);
    setNativeToolsText(
      JSON.stringify(
        (profile as unknown as { nativeToolPermissions?: unknown }).nativeToolPermissions ?? {},
        null,
        2,
      ),
    );
    setAgentIds(profile.agentIds);
    setSkillSlugs(profile.skillSlugs);
    const resources = ((profile as WorkerProfile & { resources?: Record<string, unknown> }).resources ?? {}) as {
      requests?: { cpu?: string; memory?: string };
      limits?: { cpu?: string; memory?: string };
      workspace?: { sizeLimit?: string };
    };
    setCpuRequest(resources.requests?.cpu ?? "100m");
    setMemoryRequest(resources.requests?.memory ?? "256Mi");
    setCpuLimit(resources.limits?.cpu ?? "1");
    setMemoryLimit(resources.limits?.memory ?? "1Gi");
    setWorkspaceSize(resources.workspace?.sizeLimit ?? "1Gi");
    setOpen(true);
  }

  const launch = useMutation({
    mutationFn: launchWorkerProfile,
    onSuccess: () => {
      toast.success("Worker launch requested");
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerLaunches });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const relaunch = useMutation({
    mutationFn: async (profileId: string) => {
      const active = (launches.data ?? []).filter(
        (item) => item.profileId === profileId &&
          ["pending", "starting", "running"].includes(item.status),
      );
      for (const item of active) {
        await stopWorkerLaunch(item.id);
      }
      return launchWorkerProfile(profileId);
    },
    onSuccess: () => {
      toast.success("Worker relaunch requested");
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerLaunches });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const stop = useMutation({
    mutationFn: stopWorkerLaunch,
    onSuccess: () => {
      toast.success("Worker launch stopped");
      void queryClient.invalidateQueries({ queryKey: queryKeys.workerLaunches });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  async function openLogs(launchId: string, name: string) {
    setLogsLaunch({ id: launchId, name });
    setLogsOpen(true);
    setLogs("");
    setLogsLoading(true);
    try {
      const result = await workerLaunchLogs(launchId);
      setLogs(result.logs || "No log output yet.");
    } catch (error) {
      setLogs(errorMessage(error));
    } finally {
      setLogsLoading(false);
    }
  }

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
          <Button onClick={openCreate}>
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
            <Button onClick={openCreate}>
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
                  {profile.executionDriver} · {profile.image}:
                  {profile.imageTag || `${platformVersion ?? "platform"} (auto)`}
                </p>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex flex-wrap gap-1 text-[11px]">
                  <Badge variant="secondary">
                    {(profile as WorkerProfile & { agentScope?: string }).agentScope === "selected"
                      ? `${profile.agentIds.length} agents`
                      : "all agents"}
                  </Badge>
                  <Badge variant="secondary">{profile.skillSlugs.length} skills</Badge>
                  {profile.imageTag && platformVersion && profile.imageTag !== platformVersion ? (
                    <Badge
                      variant="warning"
                      title={`Profile uses ${profile.imageTag}; platform is ${platformVersion}. Edit and relaunch to update.`}
                    >
                      image {profile.imageTag} · platform {platformVersion}
                    </Badge>
                  ) : null}
                  {profile.authSecretName ? (
                    <Badge variant="outline">auth: {profile.authSecretName}</Badge>
                  ) : null}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={launch.isPending || launches.data?.some((item) =>
                      item.profileId === profile.id &&
                      ["pending", "starting", "running"].includes(item.status),
                    )}
                    onClick={() => launch.mutate(profile.id)}
                  >
                    <Rocket /> Launch
                  </Button>
                  {launches.data?.some(
                    (item) => item.profileId === profile.id &&
                      ["pending", "starting", "running"].includes(item.status),
                  ) ? (
                    <Button
                      size="sm"
                      variant="outline"
                      loading={relaunch.isPending && relaunch.variables === profile.id}
                      onClick={() => {
                        if (
                          window.confirm(
                            `Relaunch ${profile.name}? Running agents on this worker may be interrupted.`,
                          )
                        ) {
                          relaunch.mutate(profile.id);
                        }
                      }}
                    >
                      <RotateCw /> Relaunch
                    </Button>
                  ) : null}
                  <Button size="sm" variant="ghost" onClick={() => openEdit(profile)}>
                    <Pencil /> Edit
                  </Button>
                  <details className="relative">
                    <summary className="cursor-pointer list-none rounded-md border px-3 py-1.5 text-xs hover:bg-muted">
                      More
                    </summary>
                    <div className="absolute right-0 z-20 mt-1 w-44 space-y-1 rounded-lg border bg-card p-1 shadow-lg">
                      <div onClick={() => setBootstrapProfile(profile)} className="cursor-pointer rounded px-2 py-1.5 text-xs hover:bg-muted">
                        Bootstrap output
                      </div>
                      <div onClick={() => void navigator.clipboard?.writeText(profile.id)} className="cursor-pointer rounded px-2 py-1.5 text-xs hover:bg-muted">
                        Copy profile ID
                      </div>
                      <div
                        className="cursor-pointer rounded px-2 py-1.5 text-xs hover:bg-muted"
                        onClick={() => toggle.mutate({ id: profile.id, enabled: !profile.enabled })}
                      >
                        {profile.enabled ? "Disable" : "Enable"}
                      </div>
                      <ExportResourceButton
                        resource="worker_profiles"
                        resourceId={profile.id}
                        name={profile.name}
                      />
                      <div
                        className={
                          profile.enabled
                            ? "cursor-not-allowed rounded px-2 py-1.5 text-xs text-muted-foreground"
                            : "cursor-pointer rounded px-2 py-1.5 text-xs text-destructive hover:bg-muted"
                        }
                        title={profile.enabled ? "Disable the profile before deleting it" : undefined}
                        onClick={() => {
                          if (!profile.enabled && window.confirm(`Delete profile ${profile.name}?`)) {
                            remove.mutate(profile.id);
                          }
                        }}
                      >
                        Delete
                      </div>
                    </div>
                  </details>
                </div>
                <p className="text-[10px] text-muted-foreground">
                  updated {relativeTime(profile.updatedAt)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <div className="space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold">Launches</h2>
            <p className="text-[11px] text-muted-foreground">
              Active worker instances started by the launcher.
            </p>
          </div>
          <Button
            size="sm"
            variant={showInactive ? "secondary" : "ghost"}
            onClick={() => setShowInactive((current) => !current)}
          >
            {showInactive ? "Active only" : "Show history"}
          </Button>
        </div>
        {launches.isLoading ? (
          <Skeleton className="h-20 rounded-xl" />
        ) : launches.isError ? (
          <p className="text-sm text-destructive">{errorMessage(launches.error)}</p>
        ) : !launches.data?.length ? (
          <p className="rounded-xl border border-dashed p-4 text-xs text-muted-foreground">
            {showInactive
              ? "No launches yet. Use Launch on an enabled profile."
              : "No active launches. Use Launch on an enabled profile or show history."}
          </p>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2">
            {launches.data.map((item) => (
              <Card key={item.id}>
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <CardTitle className="truncate">{item.name}</CardTitle>
                      <p className="text-[11px] text-muted-foreground">
                        {item.target} · {item.image}
                      </p>
                    </div>
                    <Badge
                      variant={
                        item.status === "running"
                          ? "success"
                          : item.status === "failed"
                            ? "destructive"
                            : item.status === "stopped"
                              ? "outline"
                              : "warning"
                      }
                    >
                      {item.status}
                    </Badge>
                    {(item as typeof item & { relaunchRequired?: boolean }).relaunchRequired ? (
                      <Badge variant="warning">relaunch required</Badge>
                    ) : null}
                  </div>
                </CardHeader>
                <CardContent className="space-y-2">
                  <p className="text-[11px] text-muted-foreground">
                    worker: <span className="font-mono">{item.workerId ?? "—"}</span>
                  </p>
                  {item.error ? (
                    <p className="text-[11px] text-destructive">{item.error}</p>
                  ) : null}
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => void openLogs(item.id, item.name)}
                    >
                      <ScrollText /> Logs
                    </Button>
                    {["pending", "starting", "running"].includes(item.status) ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={!["pending", "starting", "running"].includes(item.status)}
                        onClick={() => stop.mutate(item.id)}
                      >
                        <Square /> Stop
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => launch.mutate(item.profileId)}
                        loading={launch.isPending}
                      >
                        <Rocket /> Relaunch
                      </Button>
                    )}
                  </div>
                  <p className="text-[10px] text-muted-foreground">
                    updated {relativeTime(item.updatedAt)}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>

      <BootstrapDialog profile={bootstrapProfile} onClose={() => setBootstrapProfile(null)} />

      <Dialog open={logsOpen} onOpenChange={setLogsOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Logs · {logsLaunch?.name}</DialogTitle>
            <DialogDescription>Last 200 lines from the launcher runtime.</DialogDescription>
          </DialogHeader>
          <pre className="max-h-80 overflow-auto rounded-lg bg-muted/50 p-3 text-[10px] leading-relaxed">
            {logsLoading ? "Loading…" : logs}
          </pre>
        </DialogContent>
      </Dialog>

      <Dialog open={confirmRelaunch} onOpenChange={setConfirmRelaunch}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Relaunch worker now?</DialogTitle>
            <DialogDescription>
              This profile has a running worker and the change needs a restart. Running agents
              on this worker may be interrupted.
            </DialogDescription>
          </DialogHeader>
          <p className="text-xs text-muted-foreground">
            {activeProfileLaunches.length} active launch(es) will be stopped and relaunched.
          </p>
          <DialogFooter className="gap-2">
            <Button variant="ghost" onClick={() => setConfirmRelaunch(false)}>
              Cancel
            </Button>
            <Button
              variant="outline"
              onClick={() => {
                setConfirmRelaunch(false);
                save.mutate(false);
              }}
            >
              Save only
            </Button>
            <Button
              onClick={() => {
                setConfirmRelaunch(false);
                save.mutate(true);
              }}
            >
              Save &amp; relaunch
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-h-[90vh] max-w-4xl overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit worker profile" : "New worker profile"}</DialogTitle>
            <DialogDescription>
              The worker loads this profile at startup, claims only its agents and
              materializes its skills. Secrets stay external.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="border-b pb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground sm:col-span-2">
              Runtime
            </div>
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
                  const nextHarness = event.target.value;
                  setHarness(nextHarness);
                  setImage(DEFAULT_IMAGES[nextHarness] ?? "");
                  const currentConfig = (selectedProvider?.config ?? {}) as Record<string, unknown>;
                  if (selectedProvider && String(currentConfig.harness ?? "") !== nextHarness) {
                    setProvider("");
                    setModel("");
                  }
                  // Auto-suggest auth secret for harness-managed providers.
                  if (nextHarness === "opencode" && !authSecret) {
                    setAuthSecret("orbit-opencode-auth");
                  } else if (nextHarness === "pi" && !authSecret) {
                    setAuthSecret("orbit-pi-auth");
                  } else if (nextHarness === "mock") {
                    setAuthSecret("");
                  }
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
              <Input
                value={tag}
                placeholder="release version (defaults to the platform image tag)"
                onChange={(event) => setTag(event.target.value)}
              />
              <p className="text-[11px] text-muted-foreground">
                Leave blank to follow the platform tag and auto-relaunch after an upgrade.
              </p>
              {platformVersion && tag !== platformVersion ? (
                <button
                  type="button"
                  className="text-left text-[11px] text-primary underline-offset-2 hover:underline"
                  onClick={() => setTag(platformVersion)}
                >
                  Use platform tag {platformVersion}
                </button>
              ) : null}
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>CPU request</Label>
                <Input value={cpuRequest} onChange={(event) => setCpuRequest(event.target.value)} placeholder="100m" />
              </div>
              <div className="space-y-1.5">
                <Label>Memory request</Label>
                <Input value={memoryRequest} onChange={(event) => setMemoryRequest(event.target.value)} placeholder="256Mi" />
              </div>
              <div className="space-y-1.5">
                <Label>CPU limit</Label>
                <Input value={cpuLimit} onChange={(event) => setCpuLimit(event.target.value)} placeholder="1" />
              </div>
              <div className="space-y-1.5">
                <Label>Memory limit</Label>
                <Input value={memoryLimit} onChange={(event) => setMemoryLimit(event.target.value)} placeholder="1Gi" />
              </div>
              <div className="space-y-1.5">
                <Label>Workspace size</Label>
                <Input value={workspaceSize} onChange={(event) => setWorkspaceSize(event.target.value)} placeholder="1Gi" />
              </div>
            </div>
            <div className="space-y-1.5">
              <div className="border-b pb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground sm:col-span-2">
                Model &amp; auth
              </div>
              <Label>Provider</Label>
              {workerProviders.length ? (
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
                      const next = workerProviders.find(
                        (candidate) => providerOptionValue(candidate) === event.target.value,
                      );
                      const config = (next?.config ?? {}) as Record<string, unknown>;
                      const settings = config.settings as Record<string, unknown> | undefined;
                      const catalog = Array.isArray(settings?.models)
                        ? (settings.models as unknown[]).filter(
                            (value): value is string => typeof value === "string",
                          )
                        : [];
                      if (next?.kind === "harness_managed") {
                        const providerHarness = String(config.harness ?? "");
                        if (providerHarness) {
                          setHarness(providerHarness);
                          setImage(DEFAULT_IMAGES[providerHarness] ?? image);
                        }
                      }
                      setProvider(event.target.value);
                      setModel(String(config.model ?? "") || catalog[0] || "");
                    }}
                  >
                    <option value="__custom">Custom provider…</option>
                    {workerProviders.some((candidate) => candidate.kind === "harness_managed") ? (
                      <optgroup label="Harness-authenticated (worker handles auth)">
                        {workerProviders
                          .filter((candidate) => candidate.kind === "harness_managed")
                          .map((candidate) => {
                            const config = (candidate.config ?? {}) as Record<string, unknown>;
                            const name = providerOptionValue(candidate);
                            const providerHarness = String(config.harness ?? "");
                            return (
                              <option key={candidate.id} value={name}>
                                {candidate.name} · {name}
                                {providerHarness ? ` (${providerHarness})` : ""}
                              </option>
                            );
                          })}
                      </optgroup>
                    ) : null}
                    {workerProviders.some((candidate) => candidate.kind === "openai_compatible") ? (
                      <optgroup label="ORBIT-managed (API key stored in ORBIT)">
                        {workerProviders
                          .filter((candidate) => candidate.kind === "openai_compatible")
                          .map((candidate) => (
                            <option key={candidate.id} value={providerOptionValue(candidate)}>
                              {candidate.name}
                            </option>
                          ))}
                      </optgroup>
                    ) : null}
                  </select>
                  {selectedProvider ? null : (
                    <Input
                      value={provider}
                      placeholder="custom provider name"
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
              <p className="text-[11px] text-muted-foreground">
                Providers and model catalogs come from Models. Harness-managed providers
                use the harness auth secret; ORBIT-managed providers inject credentials.
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>Model</Label>
              {providerModels.length ? (
                <select
                  className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                  value={model}
                  onChange={(event) => setModel(event.target.value)}
                >
                  <option value="">Provider default</option>
                  {model && !providerModels.includes(model) ? (
                    <option value={model}>{model} (custom)</option>
                  ) : null}
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
              <Label>Harness auth secret (optional)</Label>
              <Input
                value={authSecret}
                onChange={(event) => setAuthSecret(event.target.value)}
                placeholder="e.g. orbit-opencode-auth"
              />
              <p className="text-[11px] text-muted-foreground">
                Only needed for harness-managed providers (e.g. OpenCode Go). Leave blank
                if you use ORBIT-managed providers (vLLM, OpenAI, etc.) — credentials are
                injected at runtime.
              </p>
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <div className="flex items-center justify-between">
                <div className="space-y-2 rounded-lg border p-3">
                  <div className="border-b pb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground sm:col-span-2">
                    Permissions
                  </div>
                  <Label>OpenCode web tools</Label>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="checkbox"
                      checked={allowWebSearch}
                      onChange={(event) => setAllowWebSearch(event.target.checked)}
                    />
                    Allow Web Search (non-interactive; needs a search provider)
                  </label>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="checkbox"
                      checked={allowWebFetch}
                      onChange={(event) => setAllowWebFetch(event.target.checked)}
                    />
                    Allow WebFetch (fetch known URLs)
                  </label>
                  <p className="text-[11px] text-muted-foreground">
                    Both are disabled by default so headless OpenCode runs do not abort on
                    an interactive Web Search request.
                  </p>
                </div>
                <div className="space-y-2 rounded-lg border p-3">
                  <Label>CLI tools (token exposure)</Label>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="checkbox"
                      checked={allowGlab}
                      onChange={(event) => setAllowGlab(event.target.checked)}
                    />
                    Allow glab with GITLAB_TOKEN from a team GitLab connection
                  </label>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="checkbox"
                      checked={allowGh}
                      onChange={(event) => setAllowGh(event.target.checked)}
                    />
                    Allow gh with GH_TOKEN from a team GitHub connection
                  </label>
                  <p className="text-[11px] text-muted-foreground">
                    Enabling this exposes the token to shell tools in the workspace. The
                    image must contain glab/gh.
                  </p>
                </div>
                <div className="space-y-1.5">
                  <Label>Native tool permissions (OpenCode)</Label>
                  <textarea
                    className="min-h-24 w-full rounded-lg border bg-card p-2 font-mono text-[11px]"
                    value={nativeToolsText}
                    onChange={(event) => setNativeToolsText(event.target.value)}
                    placeholder={'{\n  "bash": "deny",\n  "edit": "allow",\n  "shell": "allow"\n}'}
                  />
                  <p className="text-[11px] text-muted-foreground">
                    JSON map of tool name to allow/ask/deny. Example: shell allow, bash deny.
                  </p>
                </div>
                <div className="border-b pb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground sm:col-span-2">
                  Environment
                </div>
                <Label>Environment variable mappings</Label>
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => setEnvMappings((current) => [...current, { envVar: "", connectionId: "", secretKey: "" }])}
                >
                  + Add mapping
                </Button>
              </div>
              {envMappings.length === 0 ? (
                <p className="text-[11px] text-muted-foreground">
                  No env var mappings. Add one to inject connection secrets as environment variables.
                </p>
              ) : (
                <div className="space-y-2">
                  {envMappings.map((mapping, index) => (
                    <div key={index} className="grid gap-2 rounded-lg border p-2 sm:grid-cols-[1fr_1fr_1fr_auto]">
                      <Input
                        value={mapping.envVar}
                        onChange={(e) =>
                          setEnvMappings((current) =>
                            current.map((m, i) => (i === index ? { ...m, envVar: e.target.value } : m)),
                          )
                        }
                        placeholder="ENV_VAR_NAME"
                        className="font-mono text-xs"
                      />
                      <select
                        className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-xs"
                        value={mapping.connectionId}
                        onChange={(e) =>
                          setEnvMappings((current) =>
                            current.map((m, i) => (i === index ? { ...m, connectionId: e.target.value } : m)),
                          )
                        }
                      >
                        <option value="">Select connection…</option>
                        {(connections.data ?? []).map((c) => (
                          <option key={c.id} value={c.id}>
                            {c.name} · {c.connectorSlug}
                          </option>
                        ))}
                      </select>
                      <Input
                        value={mapping.secretKey}
                        onChange={(e) =>
                          setEnvMappings((current) =>
                            current.map((m, i) => (i === index ? { ...m, secretKey: e.target.value } : m)),
                          )
                        }
                        placeholder="secret key (e.g. password)"
                        className="text-xs"
                      />
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        onClick={() =>
                          setEnvMappings((current) => current.filter((_, i) => i !== index))
                        }
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
              <p className="text-[11px] text-muted-foreground">
                Maps connection credentials to environment variables. The worker fetches these
                at startup and injects them into the process and all spawned scripts.
              </p>
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <div className="flex items-center justify-between gap-2">
                <Label>Reusable environment mappings</Label>
                <Link
                  href="/environment-mappings"
                  className="text-[11px] text-primary hover:underline"
                >
                  Manage mappings
                </Link>
              </div>
              {envMappingLibrary.isLoading ? (
                <Skeleton className="h-16" />
              ) : !envMappingLibrary.data?.length ? (
                <p className="text-[11px] text-muted-foreground">
                  No reusable mappings yet. Create them under Library → Environments;
                  attached mappings are shared across profiles.
                </p>
              ) : (
                <div className="max-h-40 space-y-1 overflow-y-auto rounded-lg border p-2">
                  {envMappingLibrary.data.map((mapping) => (
                    <label key={mapping.id} className="flex items-center justify-between gap-2 text-xs">
                      <span className="min-w-0">
                        <span className="block truncate">{mapping.name}</span>
                        {mapping.description ? (
                          <span className="block truncate text-[10px] text-muted-foreground">
                            {mapping.description}
                          </span>
                        ) : null}
                      </span>
                      <Switch
                        checked={envMappingIds.includes(mapping.id)}
                        onCheckedChange={(checked) =>
                          setEnvMappingIds((current) =>
                            checked
                              ? [...current, mapping.id]
                              : current.filter((id) => id !== mapping.id),
                          )
                        }
                      />
                    </label>
                  ))}
                </div>
              )}
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <div className="flex items-center justify-between">
                <div className="border-b pb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground sm:col-span-2">
                  Scope
                </div>
                <Label>Agents</Label>
                <div className="flex gap-1">
                  <Button
                    type="button"
                    size="sm"
                    variant={agentScope === "all" ? "secondary" : "ghost"}
                    onClick={() => setAgentScope("all")}
                  >
                    All agents
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant={agentScope === "selected" ? "secondary" : "ghost"}
                    onClick={() => setAgentScope("selected")}
                  >
                    Selected agents
                  </Button>
                </div>
              </div>
              {agentScope === "all" ? (
                <p className="text-[11px] text-muted-foreground">
                  Dynamic scope: agents created later are automatically included.
                </p>
              ) : (
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
              )}
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <div className="flex items-center justify-between">
                <Label>Shared skills ({skillSlugs.length} selected)</Label>
                <div className="flex gap-1">
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => setSkillSlugs((skills.data ?? []).map((s) => s.slug))}
                  >
                    Select all
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => setSkillSlugs([])}
                  >
                    Clear
                  </Button>
                </div>
              </div>
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
          <ResourceAccessFields
            resource="worker_profiles"
            resourceId={editing?.id}
          />
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={requestSave} loading={save.isPending} disabled={!name.trim()}>
              {editing ? "Save changes" : "Create profile"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default function WorkersPage() {
  return (
    <Suspense fallback={<div className="p-8 text-center text-muted-foreground">Loading…</div>}>
      <WorkersPageInner />
    </Suspense>
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
