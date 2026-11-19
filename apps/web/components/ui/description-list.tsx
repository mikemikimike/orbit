import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** Label/value facts. Stacks on narrow containers, aligns in two columns otherwise. */
export function DescriptionList({
  items,
  className,
}: {
  items: Array<{ label: string; value: ReactNode; mono?: boolean } | null | false | undefined | "" | 0>;
  className?: string;
}) {
  return (
    <dl className={cn("divide-y text-[13px]", className)}>
      {items.filter(Boolean).map((item) => {
        const row = item as { label: string; value: ReactNode; mono?: boolean };
        return (
          <div key={row.label} className="flex items-baseline justify-between gap-4 py-2 first:pt-0 last:pb-0">
            <dt className="shrink-0 text-muted-foreground">{row.label}</dt>
            <dd
              className={cn(
                "min-w-0 break-words text-right font-medium",
                row.mono && "break-all font-mono text-xs font-normal",
              )}
            >
              {row.value}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}
