"use client";

import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

/**
 * A short row of tags that stays short: the first few are shown and the rest fold
 * behind "+N more", so a long list never turns a card into a wall of text.
 */
export function TagList({
  items,
  max = 6,
  className,
  empty = "None",
}: {
  items: Array<{ key: string; label: string; title?: string }>;
  max?: number;
  className?: string;
  empty?: string;
}) {
  const [expanded, setExpanded] = useState(false);
  if (items.length === 0) return <span className="text-[13px] text-muted-foreground">{empty}</span>;
  const visible = expanded ? items : items.slice(0, max);
  const hidden = items.length - visible.length;
  return (
    <div className={cn("flex flex-wrap items-center gap-1.5", className)}>
      {visible.map((item) => (
        <Badge key={item.key} variant="secondary" title={item.title ?? item.label} className="max-w-[16rem] truncate">
          {item.label}
        </Badge>
      ))}
      {hidden > 0 || expanded ? (
        <button
          type="button"
          onClick={() => setExpanded((value) => !value)}
          className="rounded text-xs font-medium text-primary hover:underline"
          aria-expanded={expanded}
        >
          {expanded ? "Show less" : `+${hidden} more`}
        </button>
      ) : null}
    </div>
  );
}
