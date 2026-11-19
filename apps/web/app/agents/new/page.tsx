"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";

import { AgentForm } from "@/components/agents/agent-form";
import { PageHeader } from "@/components/ui/page-header";
import { api } from "@/lib/api";
import { toAgentDefinitionBody } from "@/lib/agent-body";
import { unwrap } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";
import type { AgentDefinition } from "@orbit/contracts";

export default function NewAgentPage() {
  const router = useRouter();
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(definition: AgentDefinition) {
    setSubmitting(true);
    try {
      const created = unwrap(
        await api.POST("/api/v1/agents", {
          body: toAgentDefinitionBody(definition),
        }),
      );
      toast.success("Agent created");
      router.push(`/agents/${created.id}`);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        back={{ href: "/agents", label: "Agents" }}
        title="New agent"
        description="Define who the agent is, what it does and how much it may do alone. Saving creates version 1, which is immutable."
      />
      <AgentForm submitLabel="Create agent" submitting={submitting} onSubmit={handleSubmit} />
    </div>
  );
}
