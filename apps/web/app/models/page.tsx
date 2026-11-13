"use client";

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Boxes, Pencil, Plus, Trash2, Zap } from "lucide-react";
import { toast } from "sonner";

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
import { api } from "@/lib/api";
import { queryKeys, unwrap, useProviders } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import type { ProviderView } from "@/lib/types";
import { relativeTime } from "@/lib/utils";

interface ProviderForm {
  name: string;
  kind: "openai_compatible" | "harness_managed";
  baseUrl: string;
  model: string;
  models: string;
  credential: string;
  contextWindow: string;
  maxOutputTokens: string;
  tools: boolean;
  streaming: boolean;
  jsonMode: boolean;
  vision: boolean;
  modelsEndpoint: boolean;
  harnessProvider: string;
  harnessKind: string;
}

const EMPTY_FORM: ProviderForm = {
  name: "",
  kind: "openai_compatible",
  baseUrl: "",
  model: "",
  models: "",
  credential: "",
  contextWindow: "",
  maxOutputTokens: "",
  tools: false,
  streaming: true,
  jsonMode: false,
  vision: false,
  modelsEndpoint: true,
  harnessProvider: "opencode-go",
  harnessKind: "opencode",
};

export default function ModelsPage() {
  const providers = useProviders();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<ProviderView | null>(null);
  const [form, setForm] = useState<ProviderForm>(EMPTY_FORM);
  const [probing, setProbing] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: async () => {
      const config =
        form.kind === "openai_compatible"
          ? {
              baseUrl: form.baseUrl.trim().replace(/\/$/, ""),
              model: form.model.trim(),
              ...(form.contextWindow ? { contextWindow: Number(form.contextWindow) } : {}),
              ...(form.maxOutputTokens ? { maxOutputTokens: Number(form.maxOutputTokens) } : {}),
              headers: {},
              requestParams: {},
              compat: {
                chatCompletions: true,
                responsesApi: false,
                tools: form.tools,
                streaming: form.streaming,
                jsonMode: form.jsonMode,
                vision: form.vision,
                embeddings: false,
                modelsEndpoint: form.modelsEndpoint,
              },
            }
          : {
              harness: form.harnessKind,
              provider: form.harnessProvider.trim() || undefined,
              model: form.model.trim() || undefined,
              settings: {
                ...(form.models
                  .split(",")
                  .map((value) => value.trim())
                  .filter(Boolean).length
                  ? {
                      models: form.models
                        .split(",")
                        .map((value) => value.trim())
                        .filter(Boolean),
                    }
                  : {}),
              },
            };

      if (editing) {
        return unwrap(
          await api.PATCH("/api/v1/providers/{provider_id}", {
            params: { path: { provider_id: editing.id } },
            body: {
              name: form.name.trim(),
              config,
              ...(form.credential ? { credential: form.credential } : {}),
            },
          }),
        );
      }
      return unwrap(
        await api.POST("/api/v1/providers", {
          body: {
            name: form.name.trim(),
            kind: form.kind,
            config,
            ...(form.credential ? { credential: form.credential } : {}),
          },
        }),
      );
    },
    onSuccess: () => {
      toast.success(editing ? "Provider updated" : "Provider created");
      setOpen(false);
      setEditing(null);
      setForm(EMPTY_FORM);
      void queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: async (id: string) =>
      unwrap(
        await api.DELETE("/api/v1/providers/{provider_id}", {
          params: { path: { provider_id: id } },
        }),
      ),
    onSuccess: () => {
      toast.success("Provider deleted");
      void queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  async function probe(provider: ProviderView) {
    setProbing(provider.id);
    try {
      const result = unwrap(
        await api.POST("/api/v1/providers/{provider_id}/test", {
          params: { path: { provider_id: provider.id } },
        }),
      );
      if (result.ok) {
        toast.success(
          `Reachable in ${result.latencyMs}ms · ${result.models.length} model(s) discovered`,
        );
      } else {
        toast.error(result.error ?? "Probe failed");
      }
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setProbing(null);
    }
  }

  function openCreate() {
    setEditing(null);
    setForm(EMPTY_FORM);
    setOpen(true);
  }

  function openEdit(provider: ProviderView) {
    const config = (provider.config ?? {}) as Record<string, unknown>;
    const compat = (config.compat ?? {}) as Record<string, boolean>;
    setEditing(provider);
    setForm({
      ...EMPTY_FORM,
      name: provider.name,
      kind: provider.kind as ProviderForm["kind"],
      baseUrl: String(config.baseUrl ?? ""),
      model: String(config.model ?? ""),
      models: Array.isArray((config.settings as Record<string, unknown> | undefined)?.models)
        ? ((config.settings as Record<string, unknown>).models as string[]).join(", ")
        : "",
      contextWindow: config.contextWindow ? String(config.contextWindow) : "",
      maxOutputTokens: config.maxOutputTokens ? String(config.maxOutputTokens) : "",
      tools: Boolean(compat.tools),
      streaming: compat.streaming !== false,
      jsonMode: Boolean(compat.jsonMode),
      vision: Boolean(compat.vision),
      modelsEndpoint: compat.modelsEndpoint !== false,
      harnessProvider: String(config.provider ?? "opencode-go"),
      harnessKind: String(config.harness ?? "opencode"),
    });
    setOpen(true);
  }

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Model layer"
        title="Providers &amp; models"
        description="Configure OpenAI-compatible endpoints or harness-managed providers. Credentials are encrypted at rest and never returned by the API."
        actions={
          <Button onClick={openCreate}>
            <Plus /> Add provider
          </Button>
        }
      />

      {providers.isLoading ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {Array.from({ length: 2 }).map((_, index) => (
            <Skeleton key={index} className="h-40 rounded-xl" />
          ))}
        </div>
      ) : providers.isError ? (
        <p className="text-sm text-destructive">{errorMessage(providers.error)}</p>
      ) : !providers.data?.length ? (
        <EmptyState
          icon={<Boxes className="h-6 w-6" />}
          title="No providers configured"
          description="Add a vLLM, LiteLLM, API gateway or any OpenAI-compatible endpoint. Providers are separate from harnesses so agents can mix them freely."
          action={
            <Button onClick={openCreate}>
              <Plus /> Add provider
            </Button>
          }
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {providers.data.map((provider) => {
            const config = (provider.config ?? {}) as Record<string, unknown>;
            return (
              <Card key={provider.id}>
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <CardTitle className="truncate">{provider.name}</CardTitle>
                      <p className="text-[11px] text-muted-foreground">
                        {provider.kind.replace("_", " ")} · updated {relativeTime(provider.updatedAt)}
                      </p>
                    </div>
                    <span className="rounded bg-muted px-1.5 py-0.5 text-[10px]">
                      {provider.enabled ? "enabled" : "disabled"}
                    </span>
                  </div>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="space-y-1 text-xs">
                    {provider.kind === "openai_compatible" ? (
                      <>
                        <p className="truncate font-mono text-[11px] text-muted-foreground">
                          {String(config.baseUrl ?? "—")}
                        </p>
                        <p className="text-muted-foreground">
                          model: <span className="font-mono">{String(config.model ?? "—")}</span>
                        </p>
                      </>
                    ) : (
                      <>
                        <p className="text-muted-foreground">
                          harness: <span className="font-mono">{String(config.harness ?? "—")}</span>
                        </p>
                        <p className="text-muted-foreground">
                          provider: <span className="font-mono">{String(config.provider ?? "—")}</span>
                        </p>
                        <p className="text-muted-foreground">
                          model: <span className="font-mono">{String(config.model ?? "harness default")}</span>
                        </p>
                        {Array.isArray((config.settings as Record<string, unknown> | undefined)?.models) &&
                        ((config.settings as Record<string, unknown>).models as string[]).length ? (
                          <p
                            className="text-muted-foreground"
                            title={((config.settings as Record<string, unknown>).models as string[]).join(", ")}
                          >
                            catalog:{" "}
                            <span className="font-mono">
                              {((config.settings as Record<string, unknown>).models as string[]).length} models
                            </span>
                          </p>
                        ) : null}
                      </>
                    )}
                    <p className="text-muted-foreground">
                      credential: {provider.hasCredential ? "configured (encrypted)" : "none"}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {provider.kind === "openai_compatible" ? (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => probe(provider)}
                        loading={probing === provider.id}
                      >
                        <Zap /> Test
                      </Button>
                    ) : null}
                    <Button size="sm" variant="ghost" onClick={() => openEdit(provider)}>
                      <Pencil /> Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Delete provider ${provider.name}?`)) {
                          remove.mutate(provider.id);
                        }
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

      <Dialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) {
            setEditing(null);
            setForm(EMPTY_FORM);
          }
        }}
      >
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit provider" : "Add provider"}</DialogTitle>
            <DialogDescription>
              Credentials are write-only: rotate them here and the API never returns the value.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Name</Label>
                <Input
                  value={form.name}
                  placeholder="Company gateway"
                  onChange={(event) => setForm({ ...form, name: event.target.value })}
                />
              </div>
              <div className="space-y-1.5">
                <Label>Provider type</Label>
                <select
                  className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm disabled:opacity-60"
                  value={form.kind}
                  disabled={Boolean(editing)}
                  onChange={(event) =>
                    setForm({ ...form, kind: event.target.value as ProviderForm["kind"] })
                  }
                >
                  <option value="openai_compatible">OpenAI-compatible endpoint</option>
                  <option value="harness_managed">Harness-managed (e.g. OpenCode Go)</option>
                </select>
              </div>
            </div>

            {form.kind === "openai_compatible" ? (
              <>
                <div className="space-y-1.5">
                  <Label>Base URL</Label>
                  <Input
                    value={form.baseUrl}
                    placeholder="https://gateway.example.com/v1"
                    onChange={(event) => setForm({ ...form, baseUrl: event.target.value })}
                  />
                  <p className="text-[11px] text-muted-foreground">
                    No hardcoded provider list: vLLM, LiteLLM, APISIX, OpenRouter-style gateways all work.
                  </p>
                </div>
                <div className="grid gap-3 sm:grid-cols-3">
                  <div className="space-y-1.5 sm:col-span-1">
                    <Label>Default model</Label>
                    <Input
                      value={form.model}
                      placeholder="qwen3-coder"
                      onChange={(event) => setForm({ ...form, model: event.target.value })}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label>Context window</Label>
                    <Input
                      type="number"
                      value={form.contextWindow}
                      placeholder="optional"
                      onChange={(event) => setForm({ ...form, contextWindow: event.target.value })}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label>Max output</Label>
                    <Input
                      type="number"
                      value={form.maxOutputTokens}
                      placeholder="optional"
                      onChange={(event) => setForm({ ...form, maxOutputTokens: event.target.value })}
                    />
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-2 rounded-lg border p-3 text-sm sm:grid-cols-3">
                  {[
                    ["tools", "Tool calling"],
                    ["streaming", "Streaming"],
                    ["jsonMode", "JSON mode"],
                    ["vision", "Vision"],
                    ["modelsEndpoint", "/models endpoint"],
                  ].map(([key, label]) => (
                    <label key={key} className="flex items-center gap-2 text-xs">
                      <Switch
                        checked={form[key as keyof ProviderForm] as boolean}
                        onCheckedChange={(value) => setForm({ ...form, [key]: value })}
                      />
                      {label}
                    </label>
                  ))}
                </div>
              </>
            ) : (
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="space-y-1.5">
                  <Label>Harness</Label>
                  <select
                    className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                    value={form.harnessKind}
                    onChange={(event) => setForm({ ...form, harnessKind: event.target.value })}
                  >
                    <option value="opencode">OpenCode</option>
                    <option value="claude_code">Claude Code</option>
                    <option value="pi">Pi</option>
                  </select>
                </div>
                <div className="space-y-1.5">
                  <Label>Harness provider name</Label>
                  <Input
                    value={form.harnessProvider}
                    placeholder="opencode-go"
                    onChange={(event) => setForm({ ...form, harnessProvider: event.target.value })}
                  />
                </div>
                <div className="space-y-1.5">
                  <Label>Default model (optional)</Label>
                  {form.models.split(",").map((value) => value.trim()).filter(Boolean).length ? (
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                      value={form.model}
                      onChange={(event) => setForm({ ...form, model: event.target.value })}
                    >
                      <option value="">Harness default</option>
                      {form.models
                        .split(",")
                        .map((value) => value.trim())
                        .filter(Boolean)
                        .map((candidate) => (
                          <option key={candidate} value={candidate}>
                            {candidate}
                          </option>
                        ))}
                    </select>
                  ) : (
                    <Input
                      value={form.model}
                      placeholder="harness default"
                      onChange={(event) => setForm({ ...form, model: event.target.value })}
                    />
                  )}
                </div>
                <div className="space-y-1.5">
                  <Label>Model catalog (optional)</Label>
                  <Input
                    value={form.models}
                    placeholder="deepseek-v4-pro, claude-sonnet-4"
                    onChange={(event) => setForm({ ...form, models: event.target.value })}
                  />
                </div>
                <p className="text-[11px] text-muted-foreground sm:col-span-2">
                  Comma-separated models populate the model selectors in agent and worker forms.
                  Leave the catalog empty to allow any harness-defined model. Harness-managed
                  providers are authenticated by their harness; ORBIT stores only the credential
                  reference.
                </p>
              </div>
            )}

            <div className="space-y-1.5">
              <Label>{editing ? "Replace credential (optional)" : "Credential (optional)"}</Label>
              <Input
                type="password"
                value={form.credential}
                placeholder="sk-…"
                autoComplete="new-password"
                onChange={(event) => setForm({ ...form, credential: event.target.value })}
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => save.mutate()} loading={save.isPending} disabled={!form.name.trim()}>
              {editing ? "Save changes" : "Create provider"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
