"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";

import { AgentForm } from "@/components/agents/agent-form";
import { PageHeader } from "@/components/ui/page-header";
import { api } from "@/lib/api";
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
          body: definition as never,
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
    <div className="space-y-5 animate-fade-in">
      <PageHeader
        eyebrow="Agent Studio"
        title="New agent"
        description="Define identity, runtime, model and behaviour. Saving creates immutable version 1."
      />
      <AgentForm submitLabel="Create agent" submitting={submitting} onSubmit={handleSubmit} />
    </div>
  );
}
