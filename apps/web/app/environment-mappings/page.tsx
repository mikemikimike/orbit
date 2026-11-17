"use client";

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Plus, Server, Trash2 } from "lucide-react";
import { toast } from "sonner";

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
import { createEnvMapping, deleteEnvMapping } from "@/lib/env-mappings";
import { useConnections, useEnvMappings } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import Link from "next/link";

type Row = { envVar: string; connectionId: string; secretKey: string };

const EMPTY_ROW: Row = { envVar: "", connectionId: "", secretKey: "" };

export default function EnvironmentMappingsPage() {
  const queryClient = useQueryClient();
  const mappings = useEnvMappings();
  const connections = useConnections();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [rows, setRows] = useState<Row[]>([EMPTY_ROW]);

  const create = useMutation({
    mutationFn: async () => {
      const entries = rows
        .filter((row) => row.envVar.trim())
        .map((row) => ({
          envVar: row.envVar.trim(),
          connectionId: row.connectionId || null,
          secretKey: row.secretKey.trim() || null,
        }));
      if (!entries.length) throw new Error("Add at least one mapping");
      return createEnvMapping({
        name: name.trim(),
        description: description.trim(),
        mappings: entries,
      });
    },
    onSuccess: () => {
      toast.success("Environment mapping created");
      setOpen(false);
      setName("");
      setDescription("");
      setRows([EMPTY_ROW]);
      void queryClient.invalidateQueries({ queryKey: ["env-mappings"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const remove = useMutation({
    mutationFn: deleteEnvMapping,
    onSuccess: () => {
      toast.success("Environment mapping deleted");
      void queryClient.invalidateQueries({ queryKey: ["env-mappings"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  function openCreate() {
    setName("");
    setDescription("");
    setRows([EMPTY_ROW]);
    setOpen(true);
  }

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Library"
        title="Environment mappings"
        description="Reusable connection or secret mappings shared by worker profiles. Attach them under Workers → profile."
        actions={
          <Button onClick={openCreate}>
            <Plus /> New mapping
          </Button>
        }
      />

      {mappings.isLoading ? (
        <Skeleton className="h-40" />
      ) : mappings.isError ? (
        <p className="text-sm text-destructive">{errorMessage(mappings.error)}</p>
      ) : !mappings.data?.length ? (
        <div className="rounded-xl border border-dashed p-6 text-sm text-muted-foreground">
          No reusable mappings yet. Create one, then attach it to one or more worker profiles
          on the <Link href="/workers" className="text-primary hover:underline">Workers</Link> page.
        </div>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {mappings.data.map((mapping) => (
            <Card key={mapping.id}>
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle className="flex min-w-0 items-center gap-2">
                    <Server className="h-4 w-4 shrink-0 text-primary" />
                    <span className="truncate">{mapping.name}</span>
                  </CardTitle>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="text-destructive"
                    aria-label={`Delete ${mapping.name}`}
                    onClick={() => {
                      if (window.confirm(`Delete mapping ${mapping.name}?`)) {
                        remove.mutate(mapping.id);
                      }
                    }}
                  >
                    <Trash2 />
                  </Button>
                </div>
                {mapping.description ? (
                  <p className="text-[11px] text-muted-foreground">{mapping.description}</p>
                ) : null}
              </CardHeader>
              <CardContent className="space-y-1 text-xs">
                {mapping.mappings.map((entry, index) => (
                  <div key={index} className="flex items-center justify-between gap-2">
                    <code className="font-mono">{entry.envVar}</code>
                    <span className="truncate text-[11px] text-muted-foreground">
                      {entry.connectionId ? "connection" : entry.secretKey ? "secret" : "—"}
                    </span>
                  </div>
                ))}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>New environment mapping</DialogTitle>
            <DialogDescription>
              One reusable resource can hold several env vars. Attach it to any number of
              worker profiles.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input
                value={name}
                placeholder="GitLab + notifications"
                onChange={(event) => setName(event.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Description</Label>
              <Input
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Mappings</Label>
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => setRows((current) => [...current, EMPTY_ROW])}
                >
                  <Plus /> Add mapping
                </Button>
              </div>
              {rows.map((row, index) => (
                <div
                  key={index}
                  className="grid gap-2 rounded-lg border p-2 sm:grid-cols-[1fr_1fr_1fr_auto]"
                >
                  <Input
                    value={row.envVar}
                    placeholder="ENV_VAR_NAME"
                    className="font-mono text-xs"
                    onChange={(event) =>
                      setRows((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, envVar: event.target.value } : item,
                        ),
                      )
                    }
                  />
                  <select
                    className="flex h-9 w-full rounded-lg border border-input bg-card px-2 text-xs"
                    value={row.connectionId}
                    onChange={(event) =>
                      setRows((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, connectionId: event.target.value } : item,
                        ),
                      )
                    }
                  >
                    <option value="">No connection</option>
                    {(connections.data ?? []).map((connection) => (
                      <option key={connection.id} value={connection.id}>
                        {connection.name} · {connection.connectorSlug}
                      </option>
                    ))}
                  </select>
                  <Input
                    value={row.secretKey}
                    placeholder="secret ref (optional)"
                    className="text-xs"
                    onChange={(event) =>
                      setRows((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, secretKey: event.target.value } : item,
                        ),
                      )
                    }
                  />
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    onClick={() =>
                      setRows((current) => current.filter((_, i) => i !== index))
                    }
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))}
              <p className="text-[11px] text-muted-foreground">
                Use a connection for its credential, or reference a stored secret. Inline
                mappings on a worker profile override these.
              </p>
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
    </div>
  );
}
