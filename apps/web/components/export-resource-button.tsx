"use client";

import { Download } from "lucide-react";
import { toast } from "sonner";

import { Button, type ButtonProps } from "@/components/ui/button";
import { request } from "@/lib/http";
import { errorMessage } from "@orbit/api-client";

/** Download a secret-free JSON export of a shareable resource. */
export async function exportResource(resource: string, resourceId: string, name: string) {
  try {
    const payload = await request<unknown>(`/api/v1/export/${resource}/${resourceId}`);
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${name.replace(/[^a-z0-9._-]+/gi, "-").toLowerCase()}-${resourceId.slice(0, 8)}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    toast.success("Export downloaded");
  } catch (error) {
    toast.error(`Export failed: ${errorMessage(error)}`);
  }
}

/**
 * Export as a button. Inside an overflow menu, call `exportResource` from a
 * `DropdownMenuItem` instead.
 */
export function ExportResourceButton({
  resource,
  resourceId,
  name,
  size = "sm",
  variant = "ghost",
}: {
  resource: string;
  resourceId: string;
  name: string;
  size?: ButtonProps["size"];
  variant?: ButtonProps["variant"];
}) {
  return (
    <Button
      type="button"
      size={size}
      variant={variant}
      onClick={() => void exportResource(resource, resourceId, name)}
    >
      <Download /> Export
    </Button>
  );
}
