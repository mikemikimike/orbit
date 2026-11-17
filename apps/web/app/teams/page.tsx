"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Trash2, Users } from "lucide-react";
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
import {
  addTeamMember,
  createAccessTeam,
  deleteAccessTeam,
  listAccessTeams,
  listTeamMembers as listMembers,
  removeTeamMember,
  type AccessTeam,
} from "@/lib/access-teams";
import { errorMessage } from "@orbit/api-client";
import { api } from "@/lib/api";
import { unwrap } from "@/lib/queries";
import { relativeTime } from "@/lib/utils";

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export default function TeamsPage() {
  const queryClient = useQueryClient();
  const teams = useQuery({ queryKey: ["access-teams"], queryFn: listAccessTeams });
  const users = useQuery({
    queryKey: ["users"],
    queryFn: async () => unwrap(await api.GET("/api/v1/users")),
  });
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [membersTeam, setMembersTeam] = useState<AccessTeam | null>(null);
  const [userId, setUserId] = useState("");
  const [role, setRole] = useState("member");
  const members = useQuery({
    queryKey: ["access-team-members", membersTeam?.id],
    queryFn: () => listMembers(membersTeam!.id),
    enabled: Boolean(membersTeam),
  });

  const create = useMutation({
    mutationFn: () => createAccessTeam({ name: name.trim(), slug: slug.trim() || slugify(name) }),
    onSuccess: () => {
      toast.success("Team created");
      setOpen(false);
      setName("");
      setSlug("");
      void queryClient.invalidateQueries({ queryKey: ["access-teams"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: deleteAccessTeam,
    onSuccess: () => {
      toast.success("Team deleted");
      void queryClient.invalidateQueries({ queryKey: ["access-teams"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const addMember = useMutation({
    mutationFn: () => addTeamMember(membersTeam!.id, { userId, role }),
    onSuccess: () => {
      toast.success("Member added");
      setUserId("");
      void queryClient.invalidateQueries({ queryKey: ["access-team-members", membersTeam?.id] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const removeMember = useMutation({
    mutationFn: (memberId: string) => removeTeamMember(membersTeam!.id, memberId),
    onSuccess: () => {
      toast.success("Member removed");
      void queryClient.invalidateQueries({ queryKey: ["access-team-members", membersTeam?.id] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const userName = (id: string) =>
    users.data?.find((user) => user.id === id)?.displayName ??
    users.data?.find((user) => user.id === id)?.email ??
    id.slice(0, 8);

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Library"
        title="Teams"
        description="Access teams for resource visibility. Agent workflows are managed separately under Agent Teams."
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus /> New team
          </Button>
        }
      />

      {teams.isLoading ? (
        <Skeleton className="h-40" />
      ) : teams.isError ? (
        <p className="text-sm text-destructive">{errorMessage(teams.error)}</p>
      ) : !teams.data?.length ? (
        <div className="rounded-xl border border-dashed p-6 text-sm text-muted-foreground">
          No teams yet.
        </div>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {teams.data.map((team) => (
            <Card key={team.id}>
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle className="flex min-w-0 items-center gap-2">
                    <Users className="h-4 w-4 shrink-0 text-primary" />
                    <span className="truncate">{team.name}</span>
                  </CardTitle>
                  {team.slug === "default" ? (
                    <Badge variant="secondary">default</Badge>
                  ) : (
                    <Button
                      size="icon"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => {
                        if (window.confirm(`Delete team ${team.name}?`)) remove.mutate(team.id);
                      }}
                    >
                      <Trash2 />
                    </Button>
                  )}
                </div>
                <p className="font-mono text-[11px] text-muted-foreground">{team.slug}</p>
              </CardHeader>
              <CardContent className="space-y-2">
                <Button size="sm" variant="outline" onClick={() => setMembersTeam(team)}>
                  <Users /> Members
                </Button>
                <p className="text-[10px] text-muted-foreground">
                  updated {relativeTime(team.updatedAt)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New access team</DialogTitle>
            <DialogDescription>
              Teams control private/team visibility for agents, skills, connections and
              workers. They are not execution workflows.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input
                value={name}
                onChange={(event) => {
                  setName(event.target.value);
                  setSlug(slugify(event.target.value));
                }}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Slug</Label>
              <Input value={slug} onChange={(event) => setSlug(event.target.value)} />
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => create.mutate()} loading={create.isPending} disabled={!name.trim()}>
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(membersTeam)} onOpenChange={(next) => !next && setMembersTeam(null)}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>Members · {membersTeam?.name}</DialogTitle>
            <DialogDescription>Team leads manage resources; members can use them.</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            {members.isLoading ? (
              <Skeleton className="h-16" />
            ) : members.data?.length ? (
              members.data.map((member) => (
                <div
                  key={member.userId}
                  className="flex items-center justify-between gap-2 rounded-lg border p-2 text-xs"
                >
                  <span className="min-w-0 truncate">{userName(member.userId)}</span>
                  <span className="flex items-center gap-2">
                    <Badge variant="outline">{member.role}</Badge>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="text-destructive"
                      onClick={() => removeMember.mutate(member.userId)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </span>
                </div>
              ))
            ) : (
              <p className="text-xs text-muted-foreground">No members.</p>
            )}
            <div className="grid gap-2 sm:grid-cols-[1fr_auto_auto]">
              <select
                className="flex h-9 w-full rounded-lg border border-input bg-card px-2 text-xs"
                value={userId}
                onChange={(event) => setUserId(event.target.value)}
              >
                <option value="">Select user…</option>
                {(users.data ?? []).map((user) => (
                  <option key={user.id} value={user.id}>
                    {user.displayName} · {user.email}
                  </option>
                ))}
              </select>
              <select
                className="flex h-9 rounded-lg border border-input bg-card px-2 text-xs"
                value={role}
                onChange={(event) => setRole(event.target.value)}
              >
                <option value="lead">lead</option>
                <option value="member">member</option>
                <option value="viewer">viewer</option>
              </select>
              <Button
                size="sm"
                onClick={() => addMember.mutate()}
                disabled={!userId}
                loading={addMember.isPending}
              >
                Add
              </Button>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setMembersTeam(null)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
