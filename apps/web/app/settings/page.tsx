"use client";

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  Copy,
  Cpu,
  KeyRound,
  LogOut,
  Plus,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import Link from "next/link";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { api, setAccessToken } from "@/lib/api";
import {
  queryKeys,
  unwrap,
  useApiTokens,
  useAuditLogs,
  useMe,
  useMeta,
} from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDateTime, relativeTime, titleCase } from "@/lib/utils";

export default function SettingsPage() {
  const me = useMe();
  const meta = useMeta();
  const tokens = useApiTokens();
  const audit = useAuditLogs();
  const queryClient = useQueryClient();
  const [tokenDialogOpen, setTokenDialogOpen] = useState(false);
  const [tokenName, setTokenName] = useState("automation");
  const [createdToken, setCreatedToken] = useState<string | null>(null);

  const createToken = useMutation({
    mutationFn: async () =>
      unwrap(
        await api.POST("/api/v1/api-tokens", {
          body: { name: tokenName.trim() || "automation" },
        }),
      ),
    onSuccess: (result) => {
      setCreatedToken(result.token);
      setTokenDialogOpen(false);
      setTokenName("automation");
      void queryClient.invalidateQueries({ queryKey: queryKeys.tokens });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const revokeToken = useMutation({
    mutationFn: async (id: string) =>
      unwrap(
        await api.DELETE("/api/v1/api-tokens/{token_id}", {
          params: { path: { token_id: id } },
        }),
      ),
    onSuccess: () => {
      toast.success("Token revoked");
      void queryClient.invalidateQueries({ queryKey: queryKeys.tokens });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function signOutLocal() {
    setAccessToken(null);
    toast.success("Local token cleared");
    window.location.reload();
  }

  const authMode = String((meta.data?.auth as { mode?: string } | undefined)?.mode ?? "—");

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Platform"
        title="Settings"
        description="Identity, authentication, API tokens, audit trail and platform metadata."
        actions={
          <Button variant="outline" onClick={signOutLocal}>
            <LogOut /> Clear local token
          </Button>
        }
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-4 w-4 text-primary" /> Identity
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm">
            {me.isLoading ? (
              <Skeleton className="h-24" />
            ) : me.data ? (
              <>
                <Row label="User" value={me.data.user.displayName} />
                <Row label="Email" value={me.data.user.email} />
                <Row label="Role" value={titleCase(me.data.user.role)} />
                <Row label="Organization" value={me.data.organization.name} />
                <Row label="Org slug" value={me.data.organization.slug} />
                <Row label="Org mode" value={titleCase(me.data.organization.mode)} />
                <Row label="Auth method" value={titleCase(me.data.authMethod)} />
                <div className="pt-2">
                  <p className="mb-1 text-[11px] uppercase tracking-wider text-muted-foreground">
                    Permissions
                  </p>
                  <div className="flex flex-wrap gap-1">
                    {me.data.permissions.map((permission) => (
                      <Badge key={permission} variant="outline">
                        {permission}
                      </Badge>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <p className="text-destructive">{errorMessage(me.error)}</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Platform</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm">
            {meta.data ? (
              <>
                <Row label="Version" value={meta.data.version} />
                <Row label="API" value={meta.data.apiVersion} />
                <Row label="Environment" value={meta.data.environment} />
                <Row label="Auth mode" value={authMode} />
                <Row label="Event transport" value={meta.data.eventsDriver} />
                <div className="pt-2">
                  <p className="mb-1 text-[11px] uppercase tracking-wider text-muted-foreground">
                    Feature readiness
                  </p>
                  <div className="flex flex-wrap gap-1">
                    {Object.entries(meta.data.features).map(([feature, enabled]) => (
                      <Badge key={feature} variant={enabled ? "success" : "outline"}>
                        {enabled ? "✓" : "×"} {feature}
                      </Badge>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <Skeleton className="h-24" />
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <ShortcutCard
          href="/agents"
          title="Agents"
          description="Create and configure agents"
          icon={<ShieldCheck className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/runtimes"
          title="Runtimes"
          description="Capabilities and adapter status"
          icon={<Cpu className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/models"
          title="Models"
          description="Providers and credentials"
          icon={<Boxes className="h-4 w-4" />}
        />
      </div>

      <Card>
        <CardHeader className="flex-row items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="h-4 w-4 text-primary" /> API tokens
          </CardTitle>
          <Button size="sm" onClick={() => setTokenDialogOpen(true)}>
            <Plus /> New token
          </Button>
        </CardHeader>
        <CardContent className="space-y-2">
          {tokens.isLoading ? (
            <Skeleton className="h-16" />
          ) : tokens.data?.length ? (
            tokens.data.map((token) => (
              <div
                key={token.id}
                className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2"
              >
                <div>
                  <p className="text-sm font-medium">{token.name}</p>
                  <p className="font-mono text-[11px] text-muted-foreground">
                    {token.tokenPrefix}… · {token.scopes.length} scopes
                  </p>
                </div>
                <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
                  <span>
                    {token.revokedAt
                      ? "revoked"
                      : token.lastUsedAt
                        ? `used ${relativeTime(token.lastUsedAt)}`
                        : "never used"}
                  </span>
                  {!token.revokedAt ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Revoke token ${token.name}?`)) {
                          revokeToken.mutate(token.id);
                        }
                      }}
                    >
                      <Trash2 /> Revoke
                    </Button>
                  ) : null}
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">
              No tokens. Create one for CI, scripts or future native clients.
            </p>
          )}
          <p className="text-[11px] text-muted-foreground">
            Tokens are shown once and stored hashed. Use them as{" "}
            <code className="font-mono">Authorization: Bearer orbit_sk_…</code>.
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between">
          <CardTitle>Recent audit activity</CardTitle>
          <Link href="/executions" className="text-xs text-primary hover:underline">
            See executions
          </Link>
        </CardHeader>
        <CardContent>
          {audit.isLoading ? (
            <Skeleton className="h-32" />
          ) : audit.data?.items.length ? (
            <div className="space-y-1.5">
              {audit.data.items.slice(0, 12).map((entry) => (
                <div
                  key={entry.id}
                  className="flex flex-wrap items-center justify-between gap-2 border-b py-1.5 text-xs last:border-b-0"
                >
                  <div className="min-w-0">
                    <span className="font-medium">{entry.action}</span>
                    <span className="text-muted-foreground">
                      {" "}
                      · {entry.actorEmail ?? entry.actorType}
                      {entry.targetType ? ` · ${entry.targetType}` : ""}
                    </span>
                  </div>
                  <span className="text-[11px] text-muted-foreground">
                    {formatDateTime(entry.createdAt)}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No audit activity yet.</p>
          )}
        </CardContent>
      </Card>

      <Dialog open={tokenDialogOpen} onOpenChange={setTokenDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create API token</DialogTitle>
            <DialogDescription>
              The token inherits your current role permissions unless scopes are restricted.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-1.5">
            <Label>Name</Label>
            <Input
              value={tokenName}
              onChange={(event) => setTokenName(event.target.value)}
              placeholder="ci-deploy"
            />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setTokenDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => createToken.mutate()} loading={createToken.isPending}>
              Create token
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(createdToken)} onOpenChange={(next) => !next && setCreatedToken(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Token created</DialogTitle>
            <DialogDescription>
              Copy it now. It cannot be retrieved again; only its hash is stored.
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-center gap-2 rounded-lg border bg-muted/40 p-2">
            <code className="min-w-0 flex-1 break-all font-mono text-xs">{createdToken}</code>
            <Button
              size="icon"
              variant="outline"
              onClick={() => {
                if (createdToken) {
                  void navigator.clipboard.writeText(createdToken);
                  toast.success("Copied");
                }
              }}
            >
              <Copy />
            </Button>
          </div>
          <DialogFooter>
            <Button onClick={() => setCreatedToken(null)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="max-w-[65%] break-all text-right text-xs font-medium">{value}</span>
    </div>
  );
}

function ShortcutCard({
  href,
  title,
  description,
  icon,
}: {
  href: string;
  title: string;
  description: string;
  icon: React.ReactNode;
}) {
  return (
    <Link href={href}>
      <Card className="h-full transition-colors hover:border-primary/40 hover:bg-muted/40">
        <CardContent className="flex items-center gap-3 p-4">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
            {icon}
          </span>
          <div>
            <p className="text-sm font-medium">{title}</p>
            <p className="text-[11px] text-muted-foreground">{description}</p>
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}
