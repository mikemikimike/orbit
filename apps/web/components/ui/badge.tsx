import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

/**
 * Small label for a category or count. Tinted, never a solid fill: solid colour is
 * reserved for the primary action so a page has one loud thing at most.
 */
const badgeVariants = cva(
  "inline-flex max-w-full items-center gap-1 whitespace-nowrap rounded-[5px] border px-1.5 py-px text-xs font-medium leading-[1.125rem]",
  {
    variants: {
      variant: {
        default: "border-primary/20 bg-primary/10 text-accent-foreground",
        secondary: "border-transparent bg-muted text-muted-foreground",
        destructive: "border-destructive/25 bg-destructive/10 text-destructive",
        outline: "border-border bg-transparent text-muted-foreground",
        success: "border-success/25 bg-success/10 text-success",
        warning: "border-warning/30 bg-warning/10 text-warning",
        info: "border-info/25 bg-info/10 text-info",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
