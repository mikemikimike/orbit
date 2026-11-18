"use client";

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Share2 } from "lucide-react";
import { toast } from "sonner";

import { Button, type ButtonProps } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { ChoiceCard } from "@/components/ui/choice-card";
import { Field } from "@/components/ui/field";
import { request } from "@/lib/http";
import { listAccessTeams } from "@/lib/access-teams";
import { errorMessage } from "@orbit/api-client";

const VISIBILITY_OPTIONS = [
  { value: "private", title: "Private", description: "Only you and admins." },
  { value: "team", title: "Team", description: "Members of the team that owns it." },
  { value: "org", title: "Organization", description: "Everyone in the organization." },
] as const;

type Sharing = { visibility: string; ownerTeamId?: string | null };

/** Visibility/sharing for a shareable resource (agents, skills, connections, providers,
 * worker profiles, triggers, tool policies). */
export function ShareResourceDialog({
  resource,
  resourceId,
  resourceName,
  size = "default",
  variant = "outline",
}: {
  resource: string;
  resourceId: string;
  resourceName: string;
  /** Match the buttons the trigger sits beside. */
  size?: ButtonProps["size"];
  variant?: ButtonProps["variant"];
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
        <Button variant={variant} size={size}>
          <Share2 /> Share
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Share {resourceName}</DialogTitle>
          <DialogDescription>Choose who can see and use it.</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <p className="text-sm font-medium" id="share-visibility-label">
              Visibility
            </p>
            <div role="radiogroup" aria-labelledby="share-visibility-label" className="space-y-2">
              {VISIBILITY_OPTIONS.map((option) => (
                <ChoiceCard
                  key={option.value}
                  selected={visibility === option.value}
                  onSelect={() => setVisibility(option.value)}
                  title={option.title}
                  description={option.description}
                />
              ))}
            </div>
          </div>
          {visibility === "team" ? (
            <Field
              label="Owning team"
              htmlFor="share-team"
              hint="Members of this team get access."
            >
              <Select
                id="share-team"
                value={teamId}
                onChange={(event) => setTeamId(event.target.value)}
              >
                <option value="">Select a team</option>
                {(teams.data ?? []).map((team) => (
                  <option key={team.id} value={team.id}>
                    {team.name}
                  </option>
                ))}
              </Select>
            </Field>
          ) : null}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
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
