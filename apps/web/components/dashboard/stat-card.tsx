import type { ReactNode } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export function StatCard({
  label,
  value,
  hint,
  icon,
  tone = "default",
}: {
  label: string;
  value: ReactNode;
  hint?: string;
  icon?: ReactNode;
  tone?: "default" | "primary" | "warning" | "danger" | "success";
}) {
  const toneClass = {
    default: "bg-muted text-muted-foreground",
    primary: "bg-primary/10 text-primary",
    warning: "bg-[hsl(var(--warning))]/10 text-[hsl(var(--warning))]",
    danger: "bg-destructive/10 text-destructive",
    success: "bg-[hsl(var(--success))]/10 text-[hsl(var(--success))]",
  }[tone];

  return (
    <Card className="overflow-hidden">
      <CardContent className="flex items-start justify-between gap-3 p-4 sm:p-5">
        <div className="min-w-0">
          <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            {label}
          </p>
          <p className="mt-1 text-2xl font-semibold tracking-tight">{value}</p>
          {hint ? <p className="mt-0.5 truncate text-xs text-muted-foreground">{hint}</p> : null}
        </div>
        {icon ? (
          <span className={cn("flex h-9 w-9 shrink-0 items-center justify-center rounded-lg", toneClass)}>
            {icon}
          </span>
        ) : null}
      </CardContent>
    </Card>
  );
}
