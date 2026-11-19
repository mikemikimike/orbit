import * as React from "react";
import { cn } from "@/lib/utils";

/** Native checkbox; the look comes from globals.css so raw inputs match too. */
const Checkbox = React.forwardRef<
  HTMLInputElement,
  Omit<React.InputHTMLAttributes<HTMLInputElement>, "type">
>(({ className, ...props }, ref) => (
  <input ref={ref} type="checkbox" className={cn(className)} {...props} />
));
Checkbox.displayName = "Checkbox";

/** A checkbox with a title and optional explanation, the whole row clickable. */
function CheckboxField({
  title,
  description,
  className,
  ...props
}: Omit<React.InputHTMLAttributes<HTMLInputElement>, "type" | "title"> & {
  title: React.ReactNode;
  description?: React.ReactNode;
}) {
  return (
    <label
      className={cn(
        "flex cursor-pointer items-start gap-3 rounded-lg border bg-card p-3 transition-colors hover:border-muted-foreground/60 has-[:checked]:border-primary/50 has-[:checked]:bg-accent/40 has-[:disabled]:cursor-not-allowed has-[:disabled]:bg-subtle has-[:disabled]:hover:border-border",
        className,
      )}
    >
      <Checkbox className="mt-0.5" {...props} />
      <span className="min-w-0">
        <span className="block text-sm font-medium">{title}</span>
        {description ? (
          <span className="mt-0.5 block text-[13px] text-muted-foreground">{description}</span>
        ) : null}
      </span>
    </label>
  );
}

export { Checkbox, CheckboxField };
