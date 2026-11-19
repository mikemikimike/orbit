"use client";

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Cable, KeyRound, Pencil, PlugZap, Plus, Trash2, Webhook } from "lucide-react";
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
import { api } from "@/lib/api";
import { request } from "@/lib/http";
import { queryKeys, unwrap, useConnections, useConnectors } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDate, relativeTime, titleCase } from "@/lib/utils";
import { capabilityLabel, effectLabel } from "@/lib/presentation";
import type { ConnectionView } from "@/lib/types";

export default function ConnectionsPage() {
  const connections = useConnections();
  const connectors = useConnectors();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<{ id: string; connectorSlug: string } | null>(null);
  const [connectorSlug, setConnectorSlug] = useState("telegram");
  const [name, setName] = useState("");
  const [credential, setCredential] = useState("");
  const [scopes, setScopes] = useState<string[]>([]);
  const [configText, setConfigText] = useState("{}");
  const [connectionUrl, setConnectionUrl] = useState("");
  const [schemaValues, setSchemaValues] = useState<Record<string, string>>({});

  const defaultConfig = (slug: string) => {
    if (slug === "gitlab") {
      return JSON.stringify(
        {
          baseUrl: "https://gitlab.com/api/v4",
          cloneBaseUrl: "https://gitlab.com",
        },
        null,
        2,
      );
    }
    if (slug === "webhook") {
      return JSON.stringify({ url: "" }, null, 2);
    }
    return "{}";
  };

  const selected = connectors.data?.find((connector) => connector.slug === connectorSlug);
  const capabilities = (selected?.capabilities ?? []) as Array<{
    id: string;
    name: string;
    effect: string;
  }>;
  const schema = (selected?.configSchema ?? {}) as {
    properties?: Record<
      string,
      {
        type?: string;
        title?: string;
        description?: string;
        default?: unknown;
        format?: string;
      }
    >;
    required?: string[];
  };
  const schemaFields = Object.entries(schema.properties ?? {}).filter(
    ([key]) => !["baseUrl", "cloneBaseUrl", "url"].includes(key),
  );

  function setSchemaField(key: string, value: string) {
    const next = { ...schemaValues, [key]: value };
    setSchemaValues(next);
    let config: Record<string, unknown> = {};
    try {
      config = JSON.parse(configText || "{}") as Record<string, unknown>;
    } catch {
      config = {};
    }
    for (const [field, raw] of Object.entries(next)) {
      const property = schema.properties?.[field];
      if (raw === "") continue;
      if (property?.type === "number" || property?.type === "integer") {
        config[field] = Number(raw);
      } else if (property?.type === "boolean") {
        config[field] = raw === "true";
      } else {
        config[field] = raw;
      }
    }
    setConfigText(JSON.stringify(config, null, 2));
  }

  const save = useMutation({
    mutationFn: async () => {
      let config: Record<string, unknown> = {};
      try {
        const parsed = configText.trim() ? JSON.parse(configText) : {};
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
          config = parsed as Record<string, unknown>;
        }
      } catch {
        throw new Error("Connection config must be valid JSON");
      }
      const url = connectionUrl.trim().replace(/\/+$/, "");
      if (url && !/^https?:\/\//.test(url)) {
        throw new Error("URL must start with https:// or http://");
      }
      if (connectorSlug === "gitlab" && url) {
        const root = url.replace(/\/api\/v4$/, "");
        config = { ...config, baseUrl: `${root}/api/v4`, cloneBaseUrl: root };
      }
      if (connectorSlug === "webhook" && url) {
        config = { ...config, url };
      }
      if (editing) {
        return unwrap(
          await api.PATCH("/api/v1/connections/{connection_id}", {
            params: { path: { connection_id: editing.id } },
            body: {
              name: name.trim(),
              scopes,
              config,
              ...(credential ? { credential } : {}),
            },
          }),
        );
      }
      return unwrap(
        await api.POST("/api/v1/connections", {
          body: {
            connectorSlug,
            name: name.trim(),
            ...(credential ? { credential } : {}),
            scopes,
            config,
          },
        }),
      );
    },
    onSuccess: () => {
      toast.success(editing ? "Connection updated" : "Connection created");
      setOpen(false);
      setEditing(null);
      setName("");
      setCredential("");
      setScopes([]);
      setConfigText("{}");
      setConnectionUrl("");
      void queryClient.invalidateQueries({ queryKey: queryKeys.connections });
      void queryClient.invalidateQueries({ queryKey: queryKeys.connectors });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setEditing(null);
    setConnectorSlug("telegram");
    setName("");
    setCredential("");
    setScopes([]);
    setConfigText(defaultConfig("telegram"));
    setConnectionUrl("");
    setSchemaValues({});
    setOpen(true);
  }

  function openEdit(connection: ConnectionView) {
    setEditing({ id: connection.id, connectorSlug: connection.connectorSlug });
    setConnectorSlug(connection.connectorSlug);
    setName(connection.name);
    setCredential("");
    setScopes(connection.scopes);
    setConfigText(JSON.stringify(connection.config ?? {}, null, 2));
    const config = (connection.config ?? {}) as Record<string, unknown>;
    setSchemaValues(
      Object.fromEntries(
        Object.entries(config).map(([key, value]) => [
          key,
          typeof value === "object" ? JSON.stringify(value) : String(value ?? ""),
        ]),
      ),
    );
    const baseUrl = String(config.baseUrl ?? "");
    const cloneBaseUrl = String(config.cloneBaseUrl ?? "");
    setConnectionUrl(
      connection.connectorSlug === "gitlab"
        ? cloneBaseUrl || baseUrl.replace(/\/api\/v4$/, "")
        : connection.connectorSlug === "webhook"
          ? String(config.url ?? "")
          : "",
    );
    setOpen(true);
  }

  const test = useMutation({
    mutationFn: (id: string) =>
      request<{ ok: boolean; latencyMs: number; message: string }>(
        `/api/v1/connections/${id}/test`,
        { method: "POST" },
      ),
    onSuccess: (result) => {
      if (result.ok) {
        toast.success(`${result.message} · ${result.latencyMs}ms`);
      } else {
        toast.error(result.message);
      }
      void queryClient.invalidateQueries({ queryKey: queryKeys.connections });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: async (id: string) =>
      unwrap(
        await api.DELETE("/api/v1/connections/{connection_id}", {
          params: { path: { connection_id: id } },
        }),
      ),
    onSuccess: () => {
      toast.success("Connection removed");
      void queryClient.invalidateQueries({ queryKey: queryKeys.connections });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Integrations"
        title="Connectors"
        description="Authorized accounts for external services. Credentials are encrypted at rest and capabilities are enforced by ORBIT."
        actions={
          <Button onClick={openCreate}>
            <Plus /> New connection
          </Button>
        }
      />

      <section className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Your connections
        </h2>
        {connections.isLoading ? (
          <div className="grid gap-3 sm:grid-cols-2">
            {Array.from({ length: 2 }).map((_, index) => (
              <Skeleton key={index} className="h-36 rounded-xl" />
            ))}
          </div>
        ) : connections.isError ? (
          <p className="text-sm text-destructive">{errorMessage(connections.error)}</p>
        ) : !connections.data?.length ? (
          <EmptyState
            icon={<Cable className="h-6 w-6" />}
            title="No connections yet"
            description="Connect a Telegram bot, GitLab account or generic webhook. An agent can then be granted specific capabilities."
            action={
              <Button onClick={openCreate}>
                <Plus /> New connection
              </Button>
            }
          />
        ) : (
          <div className="grid gap-3 sm:grid-cols-2">
            {connections.data.map((connection) => (
              <Card key={connection.id}>
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <CardTitle className="truncate">{connection.name}</CardTitle>
                      <p className="text-[11px] text-muted-foreground">
                        {connection.connectorName} · {titleCase(connection.status)}
                      </p>
                    </div>
                    <Badge variant="secondary">
                      {connection.hasCredential ? "credential set" : "no credential"}
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex flex-wrap gap-1">
                    {connection.scopes.length ? (
                      connection.scopes.map((scope) => (
                        <Badge key={scope} variant="outline" title={scope}>
                          {capabilityLabel(scope)}
                        </Badge>
                      ))
                    ) : (
                      <span className="text-[11px] text-muted-foreground">No scopes configured</span>
                    )}
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>created {formatDate(connection.createdAt)}</span>
                    <div className="flex flex-wrap items-center gap-1">
                    <ExportResourceButton
                      resource="connections"
                      resourceId={connection.id}
                      name={connection.name}
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => test.mutate(connection.id)}
                      loading={test.isPending && test.variables === connection.id}
                    >
                      <PlugZap /> Test
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => openEdit(connection)}
                    >
                      <Pencil /> Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Remove connection ${connection.name}?`)) {
                          remove.mutate(connection.id);
                        }
                      }}
                    >
                      <Trash2 /> Remove
                    </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Connector catalog
        </h2>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {(connectors.data ?? []).map((connector) => {
            const connectorCaps = (connector.capabilities ?? []) as Array<{ effect: string }>;
            const auth = (connector.auth ?? []) as string[];
            return (
              <Card key={connector.id}>
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between gap-2">
                    <CardTitle>{connector.name}</CardTitle>
                    {connector.webhooks ? (
                      <Badge variant="outline" className="gap-1">
                        <Webhook className="h-3 w-3" /> webhooks
                      </Badge>
                    ) : null}
                  </div>
                  <p className="text-[11px] text-muted-foreground">v{connector.version} · {connector.slug}</p>
                </CardHeader>
                <CardContent className="space-y-2">
                  <p className="text-xs text-muted-foreground">{connector.description}</p>
                  <div className="flex flex-wrap gap-1">
                    {auth.map((method) => (
                      <Badge key={method} variant="secondary">
                        {method}
                      </Badge>
                    ))}
                    <Badge variant="outline">{connectorCaps.length} capabilities</Badge>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      </section>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? "Edit connection" : "New connection"}</DialogTitle>
            <DialogDescription>
              Credentials are sent over the API and encrypted at rest with AES-256-GCM.
              {editing ? " Leave the credential blank to keep the current secret." : ""}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label>Connector</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={connectorSlug}
                disabled={Boolean(editing)}
                onChange={(event) => {
                  setConnectorSlug(event.target.value);
                  setScopes([]);
                  setConfigText(defaultConfig(event.target.value));
                  setSchemaValues({});
                  setConnectionUrl(
                    event.target.value === "gitlab" ? "https://gitlab.com" : "",
                  );
                }}
              >
                {(connectors.data ?? []).map((connector) => (
                  <option key={connector.slug} value={connector.slug}>
                    {connector.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input
                value={name}
                placeholder="Team GitLab"
                onChange={(event) => setName(event.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label className="flex items-center gap-1">
                <KeyRound className="h-3 w-3" /> Credential
              </Label>
              <Input
                type="password"
                value={credential}
                placeholder={editing ? "leave blank to keep current credential" : "token / bot token"}
                autoComplete="new-password"
                onChange={(event) => setCredential(event.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Capabilities</Label>
              <div className="flex flex-wrap gap-1.5">
                {capabilities.map((capability) => {
                  const selectedCap = scopes.includes(capability.id);
                  return (
                    <button
                      key={capability.id}
                      type="button"
                      onClick={() =>
                        setScopes((current) =>
                          selectedCap
                            ? current.filter((id) => id !== capability.id)
                            : [...current, capability.id],
                        )
                      }
                      className={
                        selectedCap
                          ? "rounded-full border border-primary/50 bg-primary/10 px-2 py-0.5 text-[11px] text-primary"
                          : "rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground hover:bg-muted"
                      }
                    >
                      {capability.name} · {effectLabel(capability.effect)}
                    </button>
                  );
                })}
              </div>
            </div>
            {schemaFields.length > 0 &&
            connectorSlug !== "gitlab" &&
            connectorSlug !== "webhook" ? (
              <div className="space-y-3 rounded-lg border p-3">
                <p className="text-xs font-medium text-muted-foreground">Configuration</p>
                {schemaFields.map(([key, property]) => (
                  <div key={key} className="space-y-1.5">
                    <Label>
                      {property.title || key}
                      {schema.required?.includes(key) ? " *" : ""}
                    </Label>
                    {property.type === "boolean" ? (
                      <select
                        className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                        value={schemaValues[key] ?? String(property.default ?? "false")}
                        onChange={(event) => setSchemaField(key, event.target.value)}
                      >
                        <option value="false">No</option>
                        <option value="true">Yes</option>
                      </select>
                    ) : (
                      <Input
                        type={
                          property.type === "integer" || property.type === "number"
                            ? "number"
                            : property.format === "password"
                              ? "password"
                              : "text"
                        }
                        value={schemaValues[key] ?? String(property.default ?? "")}
                        placeholder={String(property.default ?? "")}
                        onChange={(event) => setSchemaField(key, event.target.value)}
                      />
                    )}
                    {property.description ? (
                      <p className="text-[11px] text-muted-foreground">{property.description}</p>
                    ) : null}
                  </div>
                ))}
              </div>
            ) : null}
            {connectorSlug === "gitlab" || connectorSlug === "webhook" ? (
              <div className="space-y-1.5">
                <Label>
                  {connectorSlug === "gitlab" ? "GitLab URL" : "Webhook URL"}
                </Label>
                <Input
                  value={connectionUrl}
                  placeholder={
                    connectorSlug === "gitlab"
                      ? "https://gitlab.internal"
                      : "https://example.com/hooks/orbit"
                  }
                  onChange={(event) => setConnectionUrl(event.target.value)}
                />
                <p className="text-[11px] text-muted-foreground">
                  {connectorSlug === "gitlab"
                    ? "The base URL; ORBIT derives /api/v4 for API calls and the same host for git clone/push."
                    : "The endpoint ORBIT sends signed JSON to."}
                </p>
              </div>
            ) : null}
            <details className="text-xs">
              <summary className="cursor-pointer text-muted-foreground">Advanced config (JSON)</summary>
              <textarea
                className="mt-2 min-h-24 w-full rounded-lg border bg-card p-2 font-mono text-[11px]"
                value={configText}
                onChange={(event) => setConfigText(event.target.value)}
              />
            </details>
          </div>
          <ResourceAccessFields resource="connections" resourceId={editing?.id} />
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => save.mutate()}
              loading={save.isPending}
              disabled={!name.trim()}
            >
              {editing ? "Save changes" : "Create connection"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <p className="text-center text-[11px] text-muted-foreground">
        Connections updated {relativeTime(connections.data?.[0]?.updatedAt)}
      </p>
    </div>
  );
}
