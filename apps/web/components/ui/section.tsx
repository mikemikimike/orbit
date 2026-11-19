import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/** A titled region of a page. Sentence-case heading, optional count and actions. */
export function Section({
  title,
  description,
  count,
  actions,
  children,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  count?: number;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("space-y-3", className)}>
      <div className="flex flex-wrap items-end justify-between gap-x-4 gap-y-2">
        <div className="min-w-0">
          <h2 className="flex items-baseline gap-2 text-[15px] font-semibold">
            {title}
            {count !== undefined ? (
              <span className="tabular text-[13px] font-normal text-muted-foreground">{count}</span>
            ) : null}
          </h2>
          {description ? (
            <p className="mt-0.5 max-w-[68ch] text-[13px] text-muted-foreground">{description}</p>
          ) : null}
        </div>
        {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}
