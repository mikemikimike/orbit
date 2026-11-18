"use client";

import { useId, useState, type ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * A collapsed group of settings most people can skip. The summary line says what
 * is inside (or what is currently set) so nobody has to open it to find out.
 */
export function Disclosure({
  title,
  summary,
  open: controlledOpen,
  onOpenChange,
  defaultOpen,
  children,
  className,
}: {
  title: ReactNode;
  summary?: ReactNode;
  /** Controlled state. Omit to let the component manage itself. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  defaultOpen?: boolean;
  children: ReactNode;
  className?: string;
}) {
  const [internalOpen, setInternalOpen] = useState(Boolean(defaultOpen));
  const open = controlledOpen ?? internalOpen;
  const panelId = useId();

  function toggle() {
    const next = !open;
    if (controlledOpen === undefined) setInternalOpen(next);
    onOpenChange?.(next);
  }

  return (
    <div className={cn("rounded-lg border", className)}>
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={toggle}
        className="flex min-h-10 w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors hover:bg-muted/50"
      >
        <ChevronRight
          className={cn("h-4 w-4 shrink-0 text-muted-foreground transition-transform", open && "rotate-90")}
        />
        <span className="flex min-w-0 flex-1 flex-wrap items-baseline gap-x-2">
          <span className="text-sm font-medium">{title}</span>
          {summary ? (
            <span className="min-w-0 truncate text-[13px] text-muted-foreground">{summary}</span>
          ) : null}
        </span>
      </button>
      {open ? (
        <div id={panelId} className="space-y-4 border-t p-3 sm:p-4">
          {children}
        </div>
      ) : null}
    </div>
  );
}
