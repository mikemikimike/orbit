import type { ReactNode } from "react";
import { CircleAlert, Info, ShieldAlert, TriangleAlert } from "lucide-react";
import { cn } from "@/lib/utils";

type Tone = "info" | "warning" | "danger" | "neutral";

const TONE: Record<Tone, { box: string; icon: string; Icon: typeof Info }> = {
  info: { box: "border-info/25 bg-info/[0.06]", icon: "text-info", Icon: Info },
  warning: { box: "border-warning/35 bg-warning/[0.08]", icon: "text-warning", Icon: TriangleAlert },
  danger: { box: "border-destructive/30 bg-destructive/[0.06]", icon: "text-destructive", Icon: CircleAlert },
  neutral: { box: "border-border bg-subtle", icon: "text-muted-foreground", Icon: ShieldAlert },
};

/** An inline message about the state of the page: what it is, and what to do. */
export function Callout({
  tone = "info",
  title,
  children,
  action,
  className,
  icon,
}: {
  tone?: Tone;
  title?: ReactNode;
  children?: ReactNode;
  action?: ReactNode;
  className?: string;
  icon?: ReactNode;
}) {
  const { box, icon: iconClass, Icon } = TONE[tone];
  return (
    <div
      role={tone === "danger" ? "alert" : "status"}
      className={cn("flex flex-col gap-3 rounded-lg border px-4 py-3 sm:flex-row sm:items-center", box, className)}
    >
      <div className="flex min-w-0 flex-1 gap-3">
        <span className={cn("mt-0.5 shrink-0 [&_svg]:h-4 [&_svg]:w-4", iconClass)}>
          {icon ?? <Icon />}
        </span>
        <div className="min-w-0 space-y-0.5">
          {title ? <p className="text-sm font-medium text-foreground">{title}</p> : null}
          {children ? <div className="text-[13px] text-muted-foreground">{children}</div> : null}
        </div>
      </div>
      {action ? <div className="flex shrink-0 gap-2 pl-7 sm:pl-0">{action}</div> : null}
    </div>
  );
}
