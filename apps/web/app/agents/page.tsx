"use client";

import { useState } from "react";
import Link from "next/link";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Bot, LayoutGrid, List, Plus, Search } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { AgentCard } from "@/components/agents/agent-card";
import { api } from "@/lib/api";
import { queryKeys, unwrap, useAgents } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { Card, CardContent } from "@/components/ui/card";
import { AgentStatusBadge } from "@/components/ui/status-badge";
import { relativeTime, titleCase } from "@/lib/utils";

const FILTERS = [
  { value: "", label: "All" },
  { value: "active", label: "Active" },
  { value: "draft", label: "Draft" },
  { value: "paused", label: "Paused" },
  { value: "archived", label: "Archived" },
];

export default function AgentsPage() {
  const [status, setStatus] = useState("active");
  const [search, setSearch] = useState("");
  const [view, setView] = useState<"grid" | "list">("grid");
  const [query, setQuery] = useState("");
  const agents = useAgents({ status: status || undefined, q: query || undefined, limit: 50 });
  const queryClient = useQueryClient();

  const invalidateAgents = () =>
    queryClient.invalidateQueries({ queryKey: queryKeys.agents() });

  const restore = useMutation({
    mutationFn: async (agentId: string) =>
      unwrap(
        await api.POST("/api/v1/agents/{agent_id}/activate", {
          params: { path: { agent_id: agentId } },
        }),
      ),
    onSuccess: () => {
      toast.success("Agent restored");
      void invalidateAgents();
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const hardDelete = useMutation({
    mutationFn: async (agentId: string) =>
      unwrap(
        await api.DELETE("/api/v1/agents/{agent_id}", {
          params: { path: { agent_id: agentId }, query: { hard: true } },
        }),
      ),
    onSuccess: () => {
      toast.success("Agent deleted permanently");
      void invalidateAgents();
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Agent registry"
        title="Agents"
        description="Persistent agent definitions with instructions, harnesses, connectors, triggers and autonomy."
        actions={
          <Button asChild>
            <Link href="/agents/new">
              <Plus /> New agent
            </Link>
          </Button>
        }
      />

      <form
        className="flex flex-col gap-3 sm:flex-row sm:items-center"
        onSubmit={(event) => {
          event.preventDefault();
          setQuery(search.trim());
        }}
      >
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-9"
            placeholder="Search agents by name or description…"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </div>
        <div className="flex rounded-lg border p-0.5">
          <Button
            type="button"
            size="sm"
            variant={view === "grid" ? "secondary" : "ghost"}
            onClick={() => setView("grid")}
            title="Card view"
          >
            <LayoutGrid />
          </Button>
          <Button
            type="button"
            size="sm"
            variant={view === "list" ? "secondary" : "ghost"}
            onClick={() => setView("list")}
            title="Compact list"
          >
            <List />
          </Button>
        </div>
        <div className="no-scrollbar -mx-1 flex gap-1 overflow-x-auto px-1 py-0.5">
          {FILTERS.map((filter) => (
            <Button
              key={filter.value}
              type="button"
              size="sm"
              variant={status === filter.value ? "default" : "outline"}
              onClick={() => setStatus(filter.value)}
            >
              {filter.label}
            </Button>
          ))}
        </div>
      </form>

      {agents.isLoading ? (
        <div className="grid gap-3 sm:grid-cols-2">
          {Array.from({ length: 4 }).map((_, index) => (
            <Skeleton key={index} className="h-44 rounded-xl" />
          ))}
        </div>
      ) : agents.isError ? (
        <Card className="border-destructive/40">
          <CardContent className="p-4 text-sm text-destructive">{errorMessage(agents.error)}</CardContent>
        </Card>
      ) : agents.data?.items.length ? (
        <>
          {view === "grid" ? (
            <div className="grid gap-3 sm:grid-cols-2">
              {agents.data.items.map((agent) => (
                <AgentCard
                  key={agent.id}
                  agent={agent}
                  onRestore={(id) => restore.mutate(id)}
                  onDelete={(id) => {
                    if (
                      window.confirm(
                        `Permanently delete ${agent.name}? This removes the agent and its execution history and cannot be undone.`,
                      )
                    ) {
                      hardDelete.mutate(id);
                    }
                  }}
                />
              ))}
            </div>
          ) : (
            <Card className="overflow-hidden">
              {agents.data.items.map((agent) => (
                <Link
                  key={agent.id}
                  href={`/agents/${agent.id}`}
                  className="flex items-center gap-3 border-b px-4 py-3 transition-colors last:border-b-0 hover:bg-muted/40"
                >
                  <AgentStatusBadge status={agent.status} />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">{agent.name}</p>
                    <p className="truncate text-[11px] text-muted-foreground">
                      {agent.harness ? titleCase(agent.harness) : "harness at run time"} ·{" "}
                      {agent.connectorCount} connectors
                    </p>
                  </div>
                  <span className="hidden shrink-0 text-[11px] text-muted-foreground sm:block">
                    v{agent.currentVersion} · {relativeTime(agent.updatedAt)}
                  </span>
                </Link>
              ))}
            </Card>
          )}
          <p className="text-center text-xs text-muted-foreground">
            {agents.data.items.length} of {agents.data.total} agents
          </p>
        </>
      ) : (
        <EmptyState
          icon={<Bot className="h-7 w-7" />}
          title={
            query
              ? "No agents match this search"
              : status === "active"
                ? "No active agents"
                : status
                  ? "No agents match this filter"
                  : "No agents yet"
          }
          description="Create an agent with instructions, a harness, a model and optional connectors. You can test it as a draft."
          action={
            <Button asChild>
              <Link href="/agents/new">
                <Plus /> Create agent
              </Link>
            </Button>
          }
        />
      )}
    </div>
  );
}
