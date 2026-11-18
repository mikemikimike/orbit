"use client";

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { listAccessTeams } from "@/lib/access-teams";
import { request } from "@/lib/http";
import { errorMessage } from "@orbit/api-client";

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

  if (!resourceId) {
    return (
      <p className="text-[11px] text-muted-foreground">
        Save the resource first to configure access.
      </p>
    );
  }

  return (
    <div className="space-y-3 rounded-lg border p-3">
      <div>
        <Label>Access</Label>
        <p className="text-[11px] text-muted-foreground">
          Visibility controls who can see and use this resource. Organization admins always
          see everything.
        </p>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="space-y-1.5">
          <Label>Visibility</Label>
          <select
            className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm"
            value={visibility}
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
      <Button
        type="button"
        size="sm"
        variant="outline"
        onClick={() => save.mutate()}
        loading={save.isPending}
        disabled={sharing.isLoading || (visibility === "team" && !teamId)}
      >
        Save access
      </Button>
    </div>
  );
}
