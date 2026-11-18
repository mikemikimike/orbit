"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { toast } from "sonner";

import { AgentForm } from "@/components/agents/agent-form";
import { ResourceAccessFields } from "@/components/resource-access-fields";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { PageHeader } from "@/components/ui/page-header";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { toAgentDefinitionBody } from "@/lib/agent-body";
import { queryKeys, unwrap, useAgent } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import { agentSpecSchema, type AgentDefinition } from "@orbit/contracts";
import { useQueryClient } from "@tanstack/react-query";

export default function EditAgentPage() {
  const params = useParams<{ id: string }>();
  const agentId = params.id;
  const router = useRouter();
  const queryClient = useQueryClient();
  const agent = useAgent(agentId);
  const [submitting, setSubmitting] = useState(false);

  if (agent.isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-96 rounded-xl" />
      </div>
    );
  }

  if (agent.isError || !agent.data) {
    return (
      <EmptyState
        title="Agent not found"
        description={agent.error ? errorMessage(agent.error) : undefined}
        action={
          <Button asChild>
            <Link href="/agents">Back to agents</Link>
          </Button>
        }
      />
    );
  }

  const parsedSpec = agentSpecSchema.safeParse(agent.data.spec);
  if (!parsedSpec.success) {
    return (
      <EmptyState
        title="Configuration cannot be loaded"
        description={parsedSpec.error.issues.map((issue) => `${issue.path.join(".")}: ${issue.message}`).join(" · ")}
        action={<Button asChild><Link href={`/agents/${agentId}`}>Back to agent</Link></Button>}
      />
    );
  }
  const initial: AgentDefinition = {
    apiVersion: "orbit.io/v1",
    kind: "Agent",
    metadata: {
      name: agent.data.name,
      description: agent.data.description,
      labels: (agent.data.labels as Record<string, string>) ?? {},
    },
    spec: parsedSpec.data,
  };

  async function handleSubmit(definition: AgentDefinition) {
    setSubmitting(true);
    try {
      unwrap(
        await api.PATCH("/api/v1/agents/{agent_id}", {
          params: { path: { agent_id: agentId } },
          body: { definition: toAgentDefinitionBody(definition) },
        }),
      );
      await queryClient.invalidateQueries({ queryKey: queryKeys.agent(agentId) });
      toast.success("New version created");
      router.push(`/agents/${agentId}`);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="space-y-5 animate-fade-in">
      <div>
        <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2">
          <Link href={`/agents/${agentId}`}>
            <ArrowLeft /> {agent.data.name}
          </Link>
        </Button>
        <PageHeader
          eyebrow={`Editing version ${agent.data.currentVersion} → new version`}
          title="Agent Studio"
          description="Saving creates an immutable version. Historical executions keep their original configuration."
        />
      </div>
      <div className="max-w-3xl">
        <ResourceAccessFields resource="agents" resourceId={agentId} />
      </div>
      <AgentForm
        initial={initial}
        submitLabel="Save new version"
        submitting={submitting}
        onSubmit={handleSubmit}
      />
    </div>
  );
}
