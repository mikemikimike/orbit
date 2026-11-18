"use client";

import { useEffect, useId, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { ChoiceCard } from "@/components/ui/choice-card";
import { Field } from "@/components/ui/field";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { listAccessTeams } from "@/lib/access-teams";
import { request } from "@/lib/http";
import { errorMessage } from "@orbit/api-client";

type Sharing = { visibility: string; ownerTeamId?: string | null };

const VISIBILITY_OPTIONS = [
  { value: "org", title: "Organization", description: "Everyone in the organization." },
  { value: "team", title: "Team", description: "Members of the team that owns it." },
  { value: "private", title: "Private", description: "Only you and admins." },
] as const;

/**
 * Who can see and use a shareable resource. Lives in the resource's edit form and
 * saves on its own, because access is stored separately from the resource itself.
 */
export function ResourceAccessFields({
  resource,
  resourceId,
}: {
  resource: string;
  resourceId?: string;
}) {
  const queryClient = useQueryClient();
  const labelId = useId();
  const teamFieldId = useId();
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
      void queryClient.invalidateQueries({ queryKey: ["sharing", resource, resourceId] });
      void queryClient.invalidateQueries({ queryKey: [resource] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  if (!resourceId) {
    return (
      <p className="text-[13px] text-muted-foreground">
        Access can be set after the first save. Until then it is visible to the whole organization.
      </p>
    );
  }

  const changed =
    sharing.data !== undefined &&
    (sharing.data.visibility !== visibility || (sharing.data.ownerTeamId ?? "") !== teamId);

  return (
    <div className="space-y-3 rounded-lg border p-3 sm:p-4">
      <div>
        <p id={labelId} className="text-sm font-medium">
          Access
        </p>
        <p className="mt-0.5 text-[13px] text-muted-foreground">
          Who can see and use this. Organization admins always can.
        </p>
      </div>
      {sharing.isLoading ? (
        <Skeleton className="h-16" />
      ) : (
        <>
          <div role="radiogroup" aria-labelledby={labelId} className="grid gap-2 sm:grid-cols-3">
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
          {visibility === "team" ? (
            <Field label="Owning team" htmlFor={teamFieldId} hint="Members of this team get access.">
              <Select id={teamFieldId} value={teamId} onChange={(event) => setTeamId(event.target.value)}>
                <option value="">Select a team</option>
                {(teams.data ?? []).map((team) => (
                  <option key={team.id} value={team.id}>
                    {team.name}
                  </option>
                ))}
              </Select>
            </Field>
          ) : null}
          <div className="flex flex-wrap items-center gap-3">
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => save.mutate()}
              loading={save.isPending}
              disabled={!changed || (visibility === "team" && !teamId)}
            >
              Save access
            </Button>
            <span className="text-[13px] text-muted-foreground">
              {visibility === "team" && !teamId
                ? "Choose the team that owns it."
                : changed
                  ? "Access is saved separately from the rest of this form."
                  : "Saved."}
            </span>
          </div>
        </>
      )}
    </div>
  );
}
