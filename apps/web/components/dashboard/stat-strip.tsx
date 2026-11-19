import type { ReactNode } from "react";
import Link from "next/link";
import { cn } from "@/lib/utils";

export interface Stat {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  href?: string;
  tone?: "default" | "danger" | "info" | "warning";
  glyph?: ReactNode;
}

const VALUE_TONE = {
  default: "text-foreground",
  danger: "text-destructive",
  info: "text-info",
  warning: "text-warning",
};

/**
 * Headline figures as one instrument panel: a single surface divided into cells
 * rather than four separate cards, so the numbers read as one status line.
 */
export function StatStrip({ stats, className }: { stats: Stat[]; className?: string }) {
  return (
    <dl
      className={cn(
        "grid grid-cols-2 overflow-hidden rounded-xl border bg-card lg:grid-cols-4",
        "[&>*:nth-child(-n+2)]:border-b [&>*:nth-child(odd)]:border-r lg:[&>*:last-child]:border-r-0 lg:[&>*:nth-child(-n+2)]:border-b-0 lg:[&>*]:border-r",
        className,
      )}
    >
      {stats.map((stat) => {
        const body = (
          <>
            <dt className="text-[13px] text-muted-foreground">{stat.label}</dt>
            <dd className="mt-1.5">
              <span
                className={cn(
                  "font-display tabular flex items-center gap-2 text-[1.75rem] leading-none",
                  VALUE_TONE[stat.tone ?? "default"],
                )}
              >
                {stat.value}
                {stat.glyph}
              </span>
              {stat.detail ? (
                <span className="mt-1.5 block text-[13px] text-muted-foreground">{stat.detail}</span>
              ) : null}
            </dd>
          </>
        );
        return stat.href ? (
          <Link
            key={stat.label}
            href={stat.href}
            className="block p-4 transition-colors hover:bg-muted/50 focus-visible:outline-offset-[-2px] sm:p-5"
          >
            {body}
          </Link>
        ) : (
          <div key={stat.label} className="p-4 sm:p-5">
            {body}
          </div>
        );
      })}
    </dl>
  );
}
