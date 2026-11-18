import { cn } from "@/lib/utils";

export function Kbd({ className, ...props }: React.HTMLAttributes<HTMLElement>) {
  return (
    <kbd
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded border border-border-strong px-1 font-sans text-xs font-medium leading-none opacity-80",
        className,
      )}
      {...props}
    />
  );
}
