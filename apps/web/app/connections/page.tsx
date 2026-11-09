"use client";

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Cable, KeyRound, Plus, Trash2, Webhook } from "lucide-react";
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
import { api } from "@/lib/api";
import { queryKeys, unwrap, useConnections, useConnectors } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDate, relativeTime, titleCase } from "@/lib/utils";

export default function ConnectionsPage() {
  const connections = useConnections();
  const connectors = useConnectors();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [connectorSlug, setConnectorSlug] = useState("telegram");
  const [name, setName] = useState("");
  const [credential, setCredential] = useState("");
  const [scopes, setScopes] = useState<string[]>([]);
  const [configText, setConfigText] = useState("{}");

  const selected = connectors.data?.find((connector) => connector.slug === connectorSlug);
  const capabilities = (selected?.capabilities ?? []) as Array<{
    id: string;
    name: string;
    effect: string;
  }>;

  const create = useMutation({
    mutationFn: async () => {
      let config: unknown = {};
      try {
        config = configText.trim() ? JSON.parse(configText) : {};
      } catch {
        throw new Error("Connection config must be valid JSON");
      }
      return unwrap(
        await api.POST("/api/v1/connections", {
          body: {
            connectorSlug,
            name: name.trim(),
            ...(credential ? { credential } : {}),
            scopes,
            config: config as never,
          },
        }),
      );
    },
    onSuccess: () => {
      toast.success("Connection created");
      setOpen(false);
      setName("");
      setCredential("");
      setScopes([]);
      setConfigText("{}");
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
        title="Connections"
        description="Authorized accounts for external services. Credentials are encrypted at rest and capabilities are enforced by ORBIT."
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus /> New connection
          </Button>
        }
      />

      <Card className="border-primary/20 bg-primary/5">
        <CardContent className="p-4 text-sm">
          <p className="font-medium">Connector capabilities arrive in the connected-agents phase</p>
          <p className="mt-1 text-muted-foreground">
            Phase 0 stores connections and credentials securely and exposes the connector catalog.
            Capability invocation, OAuth flows and inbound webhook verification are implemented in
            Phase 2. No external service is called yet.
          </p>
        </CardContent>
      </Card>

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
              <Button onClick={() => setOpen(true)}>
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
                        <Badge key={scope} variant="outline">
                          {scope}
                        </Badge>
                      ))
                    ) : (
                      <span className="text-[11px] text-muted-foreground">No scopes configured</span>
                    )}
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>created {formatDate(connection.createdAt)}</span>
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
            <DialogTitle>New connection</DialogTitle>
            <DialogDescription>
              Credentials are sent over the API and encrypted at rest with AES-256-GCM.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label>Connector</Label>
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
                value={connectorSlug}
                onChange={(event) => {
                  setConnectorSlug(event.target.value);
                  setScopes([]);
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
                placeholder="token / bot token"
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
                      {capability.id} · {capability.effect.replace("_", " ")}
                    </button>
                  );
                })}
              </div>
            </div>
            <details className="text-xs">
              <summary className="cursor-pointer text-muted-foreground">Advanced config (JSON)</summary>
              <textarea
                className="mt-2 min-h-24 w-full rounded-lg border bg-card p-2 font-mono text-[11px]"
                value={configText}
                onChange={(event) => setConfigText(event.target.value)}
              />
            </details>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => create.mutate()}
              loading={create.isPending}
              disabled={!name.trim()}
            >
              Create connection
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
