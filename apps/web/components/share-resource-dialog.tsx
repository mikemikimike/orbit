"use client";

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Share2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { request } from "@/lib/http";
import { listAccessTeams } from "@/lib/access-teams";
import { errorMessage } from "@orbit/api-client";

type Sharing = { visibility: string; ownerTeamId?: string | null };

/** Visibility/sharing for a shareable resource (agents, skills, connections, providers,
 * worker profiles, triggers, tool policies). */
export function ShareResourceDialog({
  resource,
  resourceId,
  resourceName,
}: {
  resource: string;
  resourceId: string;
  resourceName: string;
}) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [visibility, setVisibility] = useState("org");
  const [teamId, setTeamId] = useState("");
  const teams = useQuery({ queryKey: ["access-teams"], queryFn: listAccessTeams });

  const sharing = useQuery({
    queryKey: ["sharing", resource, resourceId],
    queryFn: () =>
      request<Sharing>(`/api/v1/sharing/${resource}/${resourceId}`),
    enabled: open,
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
        body: JSON.stringify({
          visibility,
          ownerTeamId: teamId || null,
        }),
      }),
    onSuccess: () => {
      toast.success("Sharing updated");
      setOpen(false);
      void queryClient.invalidateQueries({ queryKey: ["sharing", resource, resourceId] });
      void queryClient.invalidateQueries({ queryKey: [resource] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">
          <Share2 /> Share
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Share {resourceName}</DialogTitle>
          <DialogDescription>
            Private: only you and admins. Team: owning team. Organization: all members.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
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
        <DialogFooter>
          <Button variant="ghost" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={() => save.mutate()}
            loading={save.isPending}
            disabled={visibility === "team" && !teamId}
          >
            Save sharing
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
