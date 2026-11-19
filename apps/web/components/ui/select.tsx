import * as React from "react";
import { cn } from "@/lib/utils";
import { controlClass } from "@/components/ui/input";

/**
 * Native select with the shared control look. Native on purpose: it is keyboard- and
 * screen-reader-complete and opens the platform picker on phones.
 */
const Select = React.forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, children, ...props }, ref) => (
    <select ref={ref} className={cn(controlClass, "orbit-select h-9 pr-8", className)} {...props}>
      {children}
    </select>
  ),
);
Select.displayName = "Select";

export { Select };
