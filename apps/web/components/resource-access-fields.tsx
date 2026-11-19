"use client";

import { useEffect, useState } from "react";
import { Lock, ShieldCheck } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { listAccessTeams } from "@/lib/access-teams";
import { request } from "@/lib/http";
import { errorMessage } from "@orbit/api-client";
import { useLicense } from "@/lib/queries";

type Sharing = { visibility: string; ownerTeamId?: string | null };

/** Inline access/visibility editor for a shareable resource's edit form. */
export function ResourceAccessFields({
  resource,
  resourceId,
}: {
  resource: string;
  resourceId?: string;
}) {
  const queryClient = useQueryClient();
  const license = useLicense();
  const features = license.data?.license.features ?? [];
  const enabled =
    features.includes("teams") && features.includes("resource_permissions");
  const [visibility, setVisibility] = useState("org");
  const [teamId, setTeamId] = useState("");
  const teams = useQuery({
    queryKey: ["access-teams"],
    queryFn: listAccessTeams,
    enabled: Boolean(resourceId),
  });
  const sharing = useQuery({
    queryKey: ["sharing", resource, resourceId],
    queryFn: () => request<Sharing>(`/api/v1/sharing/${resource}/${resourceId}`),
    enabled: Boolean(resourceId),
  });

  useEffect(() => {
    if (sharing.data) {
      setVisibility(sharing.data.visibility);
      setTeamId(sharing.data.ownerTeamId ?? "");
    }
  }, [sharing.data]);

  const save = useMutation({
    mutationFn: () =>
      request<void>(`/api/v1/sharing/${resource}/${resourceId}`, {
        method: "PATCH",
        body: JSON.stringify({ visibility, ownerTeamId: teamId || null }),
      }),
    onSuccess: () => {
      toast.success("Access updated");
      void queryClient.invalidateQueries({
        queryKey: ["sharing", resource, resourceId],
      });
      void queryClient.invalidateQueries({ queryKey: [resource] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  if (!enabled) {
    return (
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2">
            <Lock className="h-4 w-4 text-muted-foreground" /> Access
          </CardTitle>
          <p className="text-[11px] text-muted-foreground">
            Team access is an enterprise feature. Activate a license with `teams` and
            `resource_permissions` to configure visibility and team ownership.
          </p>
        </CardHeader>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-primary" /> Access
        </CardTitle>
        <p className="text-[11px] text-muted-foreground">
          Controls who can see and use this resource. Organization admins always see
          everything. Changes save immediately and are audited.
          {!resourceId ? " Save the resource first." : ""}
        </p>
      </CardHeader>
      <CardContent className="space-y-3">
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="space-y-1.5">
          <Label>Visibility</Label>
          <select
            className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
            value={visibility}
            disabled={!resourceId}
            onChange={(event) => setVisibility(event.target.value)}
          >
            <option value="org">Organization</option>
            <option value="team">Team</option>
            <option value="private">Private</option>
          </select>
        </div>
        {visibility === "team" ? (
          <div className="space-y-1.5">
            <Label>Owning team</Label>
            <select
              className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
              value={teamId}
              disabled={!resourceId}
              onChange={(event) => setTeamId(event.target.value)}
            >
              <option value="">Select team…</option>
              {(teams.data ?? []).map((team) => (
                <option key={team.id} value={team.id}>
                  {team.name}
                </option>
              ))}
            </select>
          </div>
        ) : null}
      </div>
      <div className="flex items-center gap-2">
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() => save.mutate()}
          loading={save.isPending}
          disabled={!resourceId || sharing.isLoading || (visibility === "team" && !teamId)}
        >
          Save access
        </Button>
        {sharing.isLoading ? (
          <span className="text-[11px] text-muted-foreground">Loading…</span>
        ) : null}
      </div>
      </CardContent>
    </Card>
  );
}
