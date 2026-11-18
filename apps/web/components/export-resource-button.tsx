"use client";

import { Download } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { request } from "@/lib/http";
import { errorMessage } from "@orbit/api-client";

/** Download a secret-free JSON export of a shareable resource. */
export function ExportResourceButton({
  resource,
  resourceId,
  name,
}: {
  resource: string;
  resourceId: string;
  name: string;
}) {
  async function download() {
    try {
      const payload = await request<unknown>(
        `/api/v1/export/${resource}/${resourceId}`,
      );
      const blob = new Blob([JSON.stringify(payload, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${name.replace(/[^a-z0-9._-]+/gi, "-").toLowerCase()}-${resourceId.slice(0, 8)}.json`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  return (
    <Button type="button" size="sm" variant="ghost" onClick={() => void download()}>
      <Download /> Export
    </Button>
  );
}
