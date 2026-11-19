"use client";

import { useRef } from "react";
import { useRouter } from "next/navigation";
import { useQueryClient } from "@tanstack/react-query";
import { Upload } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { request } from "@/lib/http";
import { queryKeys } from "@/lib/queries";
import { errorMessage } from "@orbit/api-client";

/** Import an OpenCode agent .md or SKILL.md and create the matching ORBIT resource. */
export function ImportOpenCodeButton({ kind }: { kind: "agent" | "skill" }) {
  const input = useRef<HTMLInputElement>(null);
  const router = useRouter();
  const queryClient = useQueryClient();

  async function upload(file: File) {
    try {
      const content = await file.text();
      const result = await request<{
        id: string;
        name: string;
        warnings: string[];
      }>("/api/v1/import/opencode", {
        method: "POST",
        body: JSON.stringify({
          kind,
          filename: file.name,
          content,
          activate: false,
        }),
      });
      if (result.warnings.length) {
        toast.warning(`${result.name} imported with ${result.warnings.length} warning(s)`, {
          description: result.warnings.join(" · "),
        });
      } else {
        toast.success(`${result.name} imported`);
      }
      if (kind === "agent") {
        await queryClient.invalidateQueries({ queryKey: queryKeys.agents() });
        router.push(`/agents/${result.id}`);
      } else {
        await queryClient.invalidateQueries({ queryKey: queryKeys.skills });
      }
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  return (
    <>
      <input
        ref={input}
        type="file"
        accept=".md,.markdown,text/markdown"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file) void upload(file);
        }}
      />
      <Button variant="outline" onClick={() => input.current?.click()}>
        <Upload /> Import {kind === "agent" ? "agent .md" : "SKILL.md"}
      </Button>
    </>
  );
}
