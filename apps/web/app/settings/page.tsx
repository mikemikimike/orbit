"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BookOpen,
  Boxes,
  Cable,
  Copy,
  Cpu,
  KeyRound,
  LogOut,
  Plus,
  Server,
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
import { api } from "@/lib/api";
import { request } from "@/lib/http";
import { signOut } from "@/lib/session";
import { copyText } from "@/lib/browser";
import {
  queryKeys,
  unwrap,
  useApiTokens,
  useAuditLogs,
  useLicense,
  useMe,
  useMeta,
} from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { formatDateTime, relativeTime, titleCase } from "@/lib/utils";

export default function SettingsPage() {
  const me = useMe();
  const meta = useMeta();
  const license = useLicense();
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
    signOut();
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
            <LogOut /> Sign out
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

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <ShortcutCard
          href="/agents"
          title="Agents"
          description="Create and configure agents"
          icon={<ShieldCheck className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/harnesses"
          title="Harnesses"
          description="Capabilities and harness status"
          icon={<Cpu className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/models"
          title="Models"
          description="Providers and credentials"
          icon={<Boxes className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/connections"
          title="Connections"
          description="External service accounts"
          icon={<Cable className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/skills"
          title="Skills"
          description="Reusable SKILL.md packages"
          icon={<BookOpen className="h-4 w-4" />}
        />
        <ShortcutCard
          href="/workers"
          title="Workers"
          description="Bootstrap profiles and targets"
          icon={<Server className="h-4 w-4" />}
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-primary" /> OIDC / SSO
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-[11px] text-muted-foreground">
            Configure OpenID Connect for single sign-on. Stored in the platform settings,
            merged over environment values in OIDC mode and applied when the API starts
            (restart the API after saving). Leave blank to keep environment defaults.
          </p>
          <OidcSettingsForm />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-primary" /> License
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm">
          {license.isLoading ? (
            <Skeleton className="h-20" />
          ) : license.data ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={license.data.license.valid ? "success" : "outline"}>
                  {license.data.license.tier}
                </Badge>
                {license.data.license.customer ? (
                  <span className="text-xs text-muted-foreground">
                    {license.data.license.customer}
                  </span>
                ) : null}
                {license.data.license.grace ? (
                  <Badge variant="outline">grace period</Badge>
                ) : null}
              </div>
              {license.data.license.valid ? (
                <>
                  <p className="text-[11px] text-muted-foreground">
                    Expires {license.data.license.expiresAt ?? "never"}
                    {typeof license.data.license.daysRemaining === "number"
                      ? ` · ${license.data.license.daysRemaining} day(s) remaining`
                      : ""}
                  </p>
                  <div className="grid gap-1 text-[11px] text-muted-foreground sm:grid-cols-2">
                    {Object.entries(license.data.usage).map(([key, value]) => {
                      const limitKey = key === "executionsToday" ? "executions_per_day" : key;
                      const limit = license.data?.license.limits[limitKey];
                      return (
                        <span key={key}>
                          {key}: {String(value)}
                          {limit ? ` / ${String(limit)}` : ""}
                        </span>
                      );
                    })}
                  </div>
                  {license.data.warnings.length ? (
                    <p className="text-[11px] text-[hsl(var(--warning))]">
                      {license.data.warnings.join(" · ")}
                    </p>
                  ) : null}
                  <p className="text-[11px] text-muted-foreground">
                    Features: {license.data.license.features.join(", ") || "none"}
                  </p>
                </>
              ) : (
                <p className="text-[11px] text-muted-foreground">
                  Community edition. Install an offline license Secret to unlock enterprise
                  features (see docs/development/licensing.md).
                </p>
              )}
            </>
          ) : (
            <p className="text-[11px] text-muted-foreground">License status unavailable.</p>
          )}
        </CardContent>
      </Card>

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
              onClick={async () => {
                if (!createdToken) return;
                const copied = await copyText(createdToken);
                if (copied) toast.success("Copied");
                else toast.error("Copy is unavailable in this browser context");
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


function OidcSettingsForm() {
  const [issuer, setIssuer] = useState("");
  const [clientId, setClientId] = useState("");
  const [redirectUri, setRedirectUri] = useState("");
  const [scopes, setScopes] = useState("openid profile email");
  const [audience, setAudience] = useState("");
  const [jwksUrl, setJwksUrl] = useState("");
  const [autoProvision, setAutoProvision] = useState(true);
  const [defaultRole, setDefaultRole] = useState("viewer");
  const [emailClaim, setEmailClaim] = useState("email");
  const [loaded, setLoaded] = useState(false);

  const load = async () => {
    try {
      const { data } = await api.GET("/api/v1/settings/oidc");
      if (data) {
        setIssuer(data.issuer ?? "");
        setClientId(data.clientId ?? "");
        setRedirectUri(data.redirectUri ?? "");
        setScopes(data.scopes || "openid profile email");
        setAudience(data.audience ?? "");
        setJwksUrl(data.jwksUrl ?? "");
        setAutoProvision(data.autoProvision ?? true);
        setDefaultRole(data.defaultRole || "viewer");
        setEmailClaim((data as { emailClaim?: string }).emailClaim || "email");
      }
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setLoaded(true);
    }
  };

  if (!loaded) {
    void load();
    return <p className="text-sm text-muted-foreground">Loading…</p>;
  }

  async function save() {
    try {
      await api.PUT("/api/v1/settings/oidc", {
        body: {
          settings: {
            issuer,
            clientId,
            redirectUri,
            scopes,
            audience,
            jwksUrl,
            autoProvision,
            defaultRole,
            emailClaim,
          },
        },
      });
      toast.success("OIDC settings saved — restart the API to apply");
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <div className="space-y-1.5 sm:col-span-2">
        <Label>Issuer URL</Label>
        <Input value={issuer} onChange={(e) => setIssuer(e.target.value)} placeholder="https://accounts.example.com" />
      </div>
      <div className="space-y-1.5">
        <Label>Client ID</Label>
        <Input value={clientId} onChange={(e) => setClientId(e.target.value)} placeholder="orbit-web" />
      </div>
      <div className="space-y-1.5">
        <Label>Redirect URI</Label>
        <Input value={redirectUri} onChange={(e) => setRedirectUri(e.target.value)} placeholder="http://localhost:3000/auth/callback" />
      </div>
      <div className="space-y-1.5">
        <Label>Scopes</Label>
        <Input value={scopes} onChange={(e) => setScopes(e.target.value)} placeholder="openid profile email" />
      </div>
      <div className="space-y-1.5">
        <Label>Audience</Label>
        <Input value={audience} onChange={(e) => setAudience(e.target.value)} placeholder="orbit-api" />
      </div>
      <div className="space-y-1.5">
        <Label>JWKS URL</Label>
        <Input value={jwksUrl} onChange={(e) => setJwksUrl(e.target.value)} placeholder="https://accounts.example.com/.well-known/jwks.json" />
      </div>
      <div className="space-y-1.5">
        <Label>Default role</Label>
        <Input value={defaultRole} onChange={(e) => setDefaultRole(e.target.value)} placeholder="viewer" />
      </div>
      <div className="space-y-1.5">
        <Label>Identity claim</Label>
        <Input value={emailClaim} onChange={(e) => setEmailClaim(e.target.value)} placeholder="email or name" />
        <p className="text-[11px] text-muted-foreground">
          Use name when multiple IdP accounts share the same email address.
        </p>
      </div>
      <label className="flex items-start gap-2 rounded-lg border p-3 text-xs sm:col-span-2">
        <input
          type="checkbox"
          className="mt-0.5"
          checked={autoProvision}
          onChange={(e) => setAutoProvision(e.target.checked)}
        />
        <span>
          <span className="font-medium">Auto-provision users</span>
          <span className="mt-0.5 block text-muted-foreground">
            Create a local user on first OIDC login.
          </span>
        </span>
      </label>
      <div className="sm:col-span-2">
        <Button onClick={() => void save()}>Save OIDC settings</Button>
      </div>
    </div>
  );
}


function AccessPolicyCard() {
  const queryClient = useQueryClient();
  const me = useMe();
  const canManage = me.data?.permissions.includes("settings:write") ?? false;
  const policy = useQuery({
    queryKey: ["access-settings"],
    queryFn: () =>
      request<{ teamIsolation: boolean }>("/api/v1/settings/access"),
    enabled: canManage,
  });
  const save = useMutation({
    mutationFn: (teamIsolation: boolean) =>
      request("/api/v1/settings/access", {
        method: "PUT",
        body: JSON.stringify({ teamIsolation }),
      }),
    onSuccess: () => {
      toast.success("Access policy updated");
      void queryClient.invalidateQueries({ queryKey: ["access-settings"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
  if (!canManage) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-primary" /> Access policy
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <label className="flex items-start gap-2 rounded-lg border p-3 text-xs">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={policy.data?.teamIsolation ?? true}
            disabled={policy.isLoading || save.isPending}
            onChange={(event) => save.mutate(event.target.checked)}
          />
          <span>
            <span className="font-medium">Strict team isolation</span>
            <span className="mt-0.5 block text-muted-foreground">
              Non-admin users only see private resources or resources owned by their access
              teams. Organization owners/admins still see everything. Existing users are
              members of the Default team.
            </span>
          </span>
        </label>
      </CardContent>
    </Card>
  );
}

const PERMISSIONS = [
  "agents:read",
  "agents:write",
  "executions:read",
  "executions:create",
  "executions:cancel",
  "approvals:decide",
  "harnesses:read",
  "providers:read",
  "providers:write",
  "connections:read",
  "connections:write",
  "triggers:write",
  "settings:read",
  "settings:write",
  "members:read",
  "audit:read",
];

type UserRow = {
  id: string;
  email: string;
  displayName: string;
  role: string;
  status: string;
  oidcSubject?: string | null;
};
type RoleRow = { id: string; name: string; permissions: string[] };
const BUILTIN_ROLES = ["owner", "admin", "operator", "viewer"];

function UsersRolesCard() {
  const queryClient = useQueryClient();
  const me = useMe();
  const canManage = me.data?.permissions.includes("settings:write") ?? false;
  const users = useQuery({
    queryKey: ["users"],
    queryFn: () => request<UserRow[]>("/api/v1/users"),
    enabled: canManage,
  });
  const roles = useQuery({
    queryKey: ["custom-roles"],
    queryFn: () => request<RoleRow[]>("/api/v1/roles"),
    enabled: canManage,
  });
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<RoleRow | null>(null);
  const [name, setName] = useState("");
  const [permissions, setPermissions] = useState<string[]>([]);

  const setRole = useMutation({
    mutationFn: ({ id, role }: { id: string; role: string }) =>
      request(`/api/v1/users/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ role }),
      }),
    onSuccess: () => {
      toast.success("Role updated");
      void queryClient.invalidateQueries({ queryKey: ["users"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const saveRole = useMutation({
    mutationFn: () =>
      request(editing ? `/api/v1/roles/${editing.id}` : "/api/v1/roles", {
        method: editing ? "PATCH" : "POST",
        body: JSON.stringify({ name: name.trim(), permissions }),
      }),
    onSuccess: () => {
      toast.success(editing ? "Role updated" : "Role created");
      setOpen(false);
      setEditing(null);
      void queryClient.invalidateQueries({ queryKey: ["custom-roles"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const removeRole = useMutation({
    mutationFn: (id: string) =>
      request(`/api/v1/roles/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      toast.success("Role deleted");
      void queryClient.invalidateQueries({ queryKey: ["custom-roles"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setEditing(null);
    setName("");
    setPermissions([]);
    setOpen(true);
  }
  function openEdit(role: RoleRow) {
    setEditing(role);
    setName(role.name);
    setPermissions(role.permissions);
    setOpen(true);
  }

  if (!canManage) return null;

  const roleOptions = [
    ...BUILTIN_ROLES,
    ...((roles.data ?? []).map((role) => role.name)),
  ];

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-primary" /> Users &amp; roles
        </CardTitle>
        <Button size="sm" variant="outline" onClick={openCreate}>
          <Plus /> New role
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">Organization users</p>
          {users.isLoading ? <Skeleton className="h-16" /> : null}
          {users.data?.map((user) => (
            <div
              key={user.id}
              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2"
            >
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{user.displayName}</p>
                <p className="truncate text-[11px] text-muted-foreground">
                  {user.email} · {user.status}
                  {user.oidcSubject ? " · OIDC" : ""}
                </p>
              </div>
              <select
                className="h-8 rounded-lg border bg-card px-2 text-xs"
                value={user.role}
                onChange={(event) =>
                  setRole.mutate({ id: user.id, role: event.target.value })
                }
              >
                {[...new Set([user.role, ...roleOptions])].map((role) => (
                  <option key={role} value={role}>
                    {role}
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>

        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">Custom roles</p>
          {(roles.data ?? []).length ? (
            roles.data!.map((role) => (
              <div
                key={role.id}
                className="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium">{role.name}</p>
                  <p className="truncate text-[11px] text-muted-foreground">
                    {role.permissions.join(", ")}
                  </p>
                </div>
                <div className="flex gap-1">
                  <Button size="sm" variant="ghost" onClick={() => openEdit(role)}>
                    Edit
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-destructive"
                    onClick={() => {
                      if (window.confirm(`Delete role ${role.name}?`)) {
                        removeRole.mutate(role.id);
                      }
                    }}
                  >
                    Delete
                  </Button>
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">
              No custom roles. Custom roles require the custom_roles license feature.
            </p>
          )}
        </div>
      </CardContent>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? "Edit role" : "New custom role"}</DialogTitle>
            <DialogDescription>
              Permissions are additive to the organization role. Team access is separate.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input value={name} onChange={(event) => setName(event.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label>Permissions</Label>
              <div className="grid max-h-56 gap-1 overflow-y-auto rounded-lg border p-3 sm:grid-cols-2">
                {PERMISSIONS.map((permission) => (
                  <label key={permission} className="flex items-center gap-2 text-xs">
                    <input
                      type="checkbox"
                      checked={permissions.includes(permission)}
                      onChange={(event) =>
                        setPermissions((current) =>
                          event.target.checked
                            ? [...current, permission]
                            : current.filter((item) => item !== permission),
                        )
                      }
                    />
                    {permission}
                  </label>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => saveRole.mutate()}
              loading={saveRole.isPending}
              disabled={!name.trim() || !permissions.length}
            >
              Save role
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
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
