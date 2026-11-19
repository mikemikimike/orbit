import type { ReactNode } from "react";
import Link from "next/link";
import { ChevronLeft } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Page title block. `back` renders the parent as a real link (not a decorative
 * eyebrow), and `meta` holds facts about the thing itself, such as status.
 */
export function PageHeader({
  title,
  description,
  actions,
  back,
  meta,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  back?: { href: string; label: string };
  meta?: ReactNode;
  className?: string;
}) {
  return (
    <header className={cn("space-y-3", className)}>
      {back ? (
        <Link
          href={back.href}
          className="-ml-1 inline-flex min-h-8 items-center gap-0.5 rounded-md pr-2 text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
        >
          <ChevronLeft className="h-4 w-4" />
          {back.label}
        </Link>
      ) : null}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-6">
        <div className="min-w-0">
          <h1 className="font-display break-words text-2xl leading-tight sm:text-[1.75rem]">{title}</h1>
          {description ? (
            <p className="mt-1.5 max-w-[68ch] text-sm text-muted-foreground">{description}</p>
          ) : null}
          {meta ? <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5">{meta}</div> : null}
        </div>
        {actions ? (
          <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>
        ) : null}
      </div>
    </header>
  );
}
