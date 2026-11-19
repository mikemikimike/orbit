"use client";

import { useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { BookOpen, Pencil, Plus, Trash2 } from "lucide-react";
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
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import { queryKeys, unwrap, useSkills } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { relativeTime } from "@/lib/utils";
import type { Skill } from "@/lib/types";

const TEMPLATE = `---
name: example-skill
description: What this skill does and when the agent should use it.
---

# Instructions

Describe the workflow, constraints and any bundled files.
`;

export default function SkillsPage() {
  const skills = useSkills();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Skill | null>(null);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState(TEMPLATE);
  const [filesText, setFilesText] = useState("{}");
  const fileInputRef = useRef<HTMLInputElement>(null);

  function handleFileUpload(event: React.ChangeEvent<HTMLInputElement>) {
    const files = event.target.files;
    if (!files) return;
    let current: Record<string, string> = {};
    try { current = JSON.parse(filesText); } catch { current = {}; }
    Array.from(files).forEach((file) => {
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as string;
        current[file.name] = content;
        setFilesText(JSON.stringify(current, null, 2));
      };
      reader.readAsText(file);
    });
    // Reset input so the same file can be uploaded again.
    if (fileInputRef.current) fileInputRef.current.value = "";
  }

  const reset = () => {
    setEditing(null);
    setName("");
    setSlug("");
    setDescription("");
    setContent(TEMPLATE);
    setFilesText("{}");
  };

  const save = useMutation({
    mutationFn: async () => {
      if (editing) {
        return unwrap(
          await api.PATCH("/api/v1/skills/{skill_id}", {
            params: { path: { skill_id: editing.id } },
            body: { name, description, content, files: (() => { try { return JSON.parse(filesText); } catch { return undefined; } })() },
          }),
        );
      }
      return unwrap(
        await api.POST("/api/v1/skills", {
          body: { name, slug, description, content, files: (() => { try { return JSON.parse(filesText); } catch { return {}; } })() },
        }),
      );
    },
    onSuccess: () => {
      toast.success(editing ? "Skill updated" : "Skill created");
      setOpen(false);
      reset();
      void queryClient.invalidateQueries({ queryKey: queryKeys.skills });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: async (id: string) =>
      unwrap(
        await api.DELETE("/api/v1/skills/{skill_id}", {
          params: { path: { skill_id: id } },
        }),
      ),
    onSuccess: () => {
      toast.success("Skill deleted");
      void queryClient.invalidateQueries({ queryKey: queryKeys.skills });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Harness assets"
        title="Skills"
        description="Reusable SKILL.md packages materialized into the execution workspace. Attach them in Agent Studio."
        actions={
          <Button
            onClick={() => {
              reset();
              setOpen(true);
            }}
          >
            <Plus /> New skill
          </Button>
        }
      />

      {skills.isLoading ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {Array.from({ length: 2 }).map((_, index) => (
            <Skeleton key={index} className="h-40 rounded-xl" />
          ))}
        </div>
      ) : skills.isError ? (
        <p className="text-sm text-destructive">{errorMessage(skills.error)}</p>
      ) : !skills.data?.length ? (
        <EmptyState
          icon={<BookOpen className="h-6 w-6" />}
          title="No skills yet"
          description="Create a SKILL.md package, then attach it to an agent. Pi discovers it under .pi/skills; OpenCode workspaces get .opencode/skills."
          action={
            <Button
              onClick={() => {
                reset();
                setOpen(true);
              }}
            >
              <Plus /> Create skill
            </Button>
          }
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {skills.data.map((skill) => (
            <Card key={skill.id}>
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle>{skill.name}</CardTitle>
                  <Badge variant="outline">v{skill.version}</Badge>
                </div>
                <p className="font-mono text-[11px] text-muted-foreground">
                  {skill.slug} · updated {relativeTime(skill.updatedAt)}
                </p>
              </CardHeader>
              <CardContent className="space-y-3">
                <p className="line-clamp-2 text-sm text-muted-foreground">
                  {skill.description || "No description."}
                </p>
                <div className="flex flex-wrap gap-2">
                  <ExportResourceButton
                    resource="skills"
                    resourceId={skill.id}
                    name={skill.name}
                  />
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setEditing(skill);
                      setName(skill.name);
                      setSlug(skill.slug);
                      setDescription(skill.description);
                      setContent(skill.content);
    setFilesText(JSON.stringify(skill.files ?? {}, null, 2));
                      setOpen(true);
                    }}
                  >
                    <Pencil /> Edit
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-destructive"
                    onClick={() => {
                      if (window.confirm(`Delete skill ${skill.name}?`)) remove.mutate(skill.id);
                    }}
                  >
                    <Trash2 /> Delete
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) reset();
        }}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit skill" : "New skill"}</DialogTitle>
            <DialogDescription>
              SKILL.md follows the Agent Skills format. The description controls when the
              agent loads it.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input
                value={name}
                onChange={(event) => {
                  setName(event.target.value);
                  if (!editing) {
                    setSlug(
                      event.target.value
                        .toLowerCase()
                        .replace(/[^a-z0-9]+/g, "-")
                        .replace(/^-|-$/g, "")
                        .slice(0, 64),
                    );
                  }
                }}
                placeholder="GitLab triage"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Slug</Label>
              <Input
                value={slug}
                disabled={Boolean(editing)}
                onChange={(event) => setSlug(event.target.value)}
                placeholder="gitlab-triage"
              />
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <Label>Description (routing)</Label>
              <Input
                value={description}
                onChange={(event) => setDescription(event.target.value)}
                placeholder="Use when triaging GitLab issues and proposing labels."
              />
            </div>
            <div className="space-y-1.5 sm:col-span-2">
              <Label>SKILL.md</Label>
              <Textarea
                className="min-h-[280px] font-mono text-xs"
                value={content}
                onChange={(event) => setContent(event.target.value)}
              />
            </div>
          </div>
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <Label>Bundled files (JSON: path → content)</Label>
              <Button type="button" size="sm" variant="outline" onClick={() => fileInputRef.current?.click()}>
                Upload files
              </Button>
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="hidden"
                onChange={handleFileUpload}
              />
            </div>
            <Textarea
              className="min-h-[120px] font-mono text-xs"
              value={filesText}
              onChange={(event) => setFilesText(event.target.value)}
              placeholder='{"scripts/run.sh": "#!/bin/bash\necho hello"}'
            />
            <p className="text-[11px] text-muted-foreground">
              Include scripts under <code className="font-mono">scripts/</code>. Allowed extensions:
              .sh, .bash, .py, .js, .ts, .rb, .pl, .go.
            </p>
          </div>
          <ResourceAccessFields resource="skills" resourceId={editing?.id} />
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => save.mutate()}
              loading={save.isPending}
              disabled={!name.trim() || !slug.trim() || !content.trim()}
            >
              {editing ? "Save skill" : "Create skill"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
