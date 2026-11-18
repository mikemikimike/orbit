"use client";

import { useState } from "react";
import Link from "next/link";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Ellipsis, KeyRound, Plus, Trash2, X } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Callout } from "@/components/ui/callout";
import { Card } from "@/components/ui/card";
import { useConfirm } from "@/components/ui/confirm";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { EmptyState } from "@/components/ui/empty-state";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/ui/page-header";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { createEnvMapping, deleteEnvMapping } from "@/lib/env-mappings";
import { useConnections, useEnvMappings } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";

type Row = { envVar: string; connectionId: string; secretKey: string };

const EMPTY_ROW: Row = { envVar: "", connectionId: "", secretKey: "" };

export default function EnvironmentMappingsPage() {
  const queryClient = useQueryClient();
  const confirm = useConfirm();
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
      toast.success("Mapping created");
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
      toast.success("Mapping deleted");
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

  function updateRow(index: number, patch: Partial<Row>) {
    setRows((current) => current.map((item, i) => (i === index ? { ...item, ...patch } : item)));
  }

  async function confirmDelete(id: string, mappingName: string) {
    const confirmed = await confirm({
      title: `Delete ${mappingName}?`,
      description:
        "Worker profiles that use this mapping stop receiving its variables. This cannot be undone.",
      confirmLabel: "Delete mapping",
      destructive: true,
    });
    if (confirmed) remove.mutate(id);
  }

  const connectionName = (id?: string | null) =>
    (connections.data ?? []).find((connection) => connection.id === id)?.name;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Environments"
        description="An environment mapping gives worker processes environment variables whose values come from a connection or a stored secret. Define it once and attach it to as many worker profiles as you like."
        actions={
          <Button onClick={openCreate}>
            <Plus /> New mapping
          </Button>
        }
      />

      {mappings.isLoading ? (
        <Card className="overflow-hidden">
          {Array.from({ length: 3 }).map((_, index) => (
            <div key={index} className="border-b px-5 py-4 last:border-b-0">
              <Skeleton className="h-12" />
            </div>
          ))}
        </Card>
      ) : mappings.isError ? (
        <Callout tone="danger" title="Mappings could not be loaded">
          {errorMessage(mappings.error)}
        </Callout>
      ) : !mappings.data?.length ? (
        <EmptyState
          icon={<KeyRound />}
          title="No reusable mappings yet"
          description="Create one, then attach it to worker profiles in the profile's advanced settings."
          action={
            <div className="flex flex-wrap justify-center gap-2">
              <Button variant="outline" onClick={openCreate}>
                <Plus /> New mapping
              </Button>
              <Button asChild variant="outline">
                <Link href="/workers">Go to workers</Link>
              </Button>
            </div>
          }
        />
      ) : (
        <Card className="overflow-hidden">
          <ul>
            {mappings.data.map((mapping) => (
              <li key={mapping.id} className="border-b px-4 py-4 last:border-b-0 sm:px-5">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="break-words text-sm font-semibold">{mapping.name}</p>
                    {mapping.description ? (
                      <p className="mt-0.5 text-[13px] text-muted-foreground">{mapping.description}</p>
                    ) : null}
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label={`More actions for ${mapping.name}`}
                      >
                        <Ellipsis />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem
                        destructive
                        onSelect={() => void confirmDelete(mapping.id, mapping.name)}
                      >
                        <Trash2 /> Delete mapping
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                <ul className="mt-3 divide-y rounded-lg border bg-subtle/50">
                  {mapping.mappings.map((entry, index) => {
                    const source = entry.connectionId
                      ? `From connection ${connectionName(entry.connectionId) ?? "(unknown)"}`
                      : entry.secretKey
                        ? "From a stored secret"
                        : "No source set";
                    return (
                      <li
                        key={index}
                        className="flex flex-col gap-0.5 px-3 py-2 sm:flex-row sm:items-baseline sm:justify-between sm:gap-4"
                      >
                        <code className="break-all font-mono text-[13px]">{entry.envVar}</code>
                        <span className="break-words text-[13px] text-muted-foreground">{source}</span>
                      </li>
                    );
                  })}
                </ul>
              </li>
            ))}
          </ul>
        </Card>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>New environment mapping</DialogTitle>
            <DialogDescription>
              One mapping can hold several variables. Attach it to any number of worker profiles.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-5">
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Name" htmlFor="em-name">
                <Input
                  id="em-name"
                  value={name}
                  placeholder="GitLab and notifications"
                  onChange={(event) => setName(event.target.value)}
                />
              </Field>
              <Field label="Description" htmlFor="em-description" optional>
                <Input
                  id="em-description"
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
              </Field>
            </div>

            <div className="space-y-3 border-t pt-4">
              <div className="flex flex-wrap items-end justify-between gap-2">
                <div>
                  <h3 className="text-[15px] font-semibold">Variables</h3>
                  <p className="mt-0.5 text-[13px] text-muted-foreground">
                    Take a value from a connection, or reference a stored secret. Inline mappings on
                    a worker profile override these.
                  </p>
                </div>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => setRows((current) => [...current, EMPTY_ROW])}
                >
                  <Plus /> Add variable
                </Button>
              </div>
              <ul className="space-y-2">
                {rows.map((row, index) => (
                  <li key={index} className="rounded-lg border bg-card p-3">
                    <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end">
                      <Field label="Variable" htmlFor={`em-${index}-var`}>
                        <Input
                          id={`em-${index}-var`}
                          value={row.envVar}
                          placeholder="ENV_VAR_NAME"
                          className="font-mono text-[13px]"
                          onChange={(event) => updateRow(index, { envVar: event.target.value })}
                        />
                      </Field>
                      <Field label="Connection" htmlFor={`em-${index}-conn`}>
                        <Select
                          id={`em-${index}-conn`}
                          value={row.connectionId}
                          onChange={(event) => updateRow(index, { connectionId: event.target.value })}
                        >
                          <option value="">No connection</option>
                          {(connections.data ?? []).map((connection) => (
                            <option key={connection.id} value={connection.id}>
                              {connection.name} ({connection.connectorSlug})
                            </option>
                          ))}
                        </Select>
                      </Field>
                      <Field label="Secret reference" htmlFor={`em-${index}-secret`} optional>
                        <Input
                          id={`em-${index}-secret`}
                          value={row.secretKey}
                          onChange={(event) => updateRow(index, { secretKey: event.target.value })}
                        />
                      </Field>
                      <Button
                        type="button"
                        size="icon"
                        variant="ghost"
                        className="justify-self-start sm:justify-self-auto"
                        aria-label={`Remove variable ${row.envVar || index + 1}`}
                        onClick={() => setRows((current) => current.filter((_, i) => i !== index))}
                      >
                        <X />
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => create.mutate()} loading={create.isPending} disabled={!name.trim()}>
              Create mapping
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
