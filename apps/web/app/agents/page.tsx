"use client";

import { useState } from "react";
import Link from "next/link";
import { Bot, Plus, Search } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { AgentCard } from "@/components/agents/agent-card";
import { useAgents } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { Card, CardContent } from "@/components/ui/card";

const FILTERS = [
  { value: "", label: "All" },
  { value: "active", label: "Active" },
  { value: "draft", label: "Draft" },
  { value: "paused", label: "Paused" },
  { value: "archived", label: "Archived" },
];

export default function AgentsPage() {
  const [status, setStatus] = useState("");
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const agents = useAgents({ status: status || undefined, q: query || undefined, limit: 50 });

  return (
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Agent registry"
        title="Agents"
        description="Persistent agent definitions with instructions, runtimes, connectors, triggers and autonomy."
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
          <div className="grid gap-3 sm:grid-cols-2">
            {agents.data.items.map((agent) => (
              <AgentCard key={agent.id} agent={agent} />
            ))}
          </div>
          <p className="text-center text-xs text-muted-foreground">
            {agents.data.items.length} of {agents.data.total} agents
          </p>
        </>
      ) : (
        <EmptyState
          icon={<Bot className="h-7 w-7" />}
          title={query || status ? "No agents match this filter" : "No agents yet"}
          description="Create an agent with instructions, a runtime, a model and optional connectors. You can test it as a draft."
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
