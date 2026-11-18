"use client";

import Link from "next/link";
import { Check, ChevronsUpDown, LogOut, Monitor, Moon, Settings, Sun } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useTheme, type ThemeMode } from "@/lib/theme";
import { useMe } from "@/lib/queries";
import { signOut } from "@/lib/session";
import { cn, titleCase } from "@/lib/utils";

export function useCurrentUser() {
  const me = useMe();
  const user = me.data?.user as { displayName?: string; email?: string; role?: string } | undefined;
  const name = user?.displayName?.trim() || user?.email?.split("@")[0] || "Account";
  const initials =
    name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase())
      .join("") || "?";
  return { name, initials, email: user?.email, role: user?.role };
}

export function Avatar({ initials, className }: { initials: string; className?: string }) {
  return (
    <span
      className={cn(
        "flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-ink-accent/20 text-xs font-semibold text-ink-accent",
        className,
      )}
    >
      {initials}
    </span>
  );
}

const THEMES: Array<{ value: ThemeMode; label: string; icon: typeof Sun }> = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "Match system", icon: Monitor },
];

function MenuBody() {
  const user = useCurrentUser();
  const { mode, setMode } = useTheme();
  return (
    <>
      <div className="px-2.5 pb-2 pt-1.5">
        <p className="truncate text-sm font-medium">{user.name}</p>
        <p className="truncate text-xs text-muted-foreground">
          {[user.email, user.role ? titleCase(user.role) : null].filter(Boolean).join(" · ")}
        </p>
      </div>
      <DropdownMenuSeparator />
      <DropdownMenuLabel>Theme</DropdownMenuLabel>
      {THEMES.map((theme) => (
        <DropdownMenuItem
          key={theme.value}
          role="menuitemradio"
          aria-checked={mode === theme.value}
          // Keep the menu open so the change is seen where it was made.
          onSelect={(event) => {
            event.preventDefault();
            setMode(theme.value);
          }}
        >
          <theme.icon /> {theme.label}
          {mode === theme.value ? <Check className="ml-auto !text-primary" /> : null}
        </DropdownMenuItem>
      ))}
      <DropdownMenuSeparator />
      <DropdownMenuItem asChild>
        <Link href="/settings">
          <Settings /> Settings
        </Link>
      </DropdownMenuItem>
      <DropdownMenuItem onSelect={signOut}>
        <LogOut /> Sign out
      </DropdownMenuItem>
    </>
  );
}

/** Sidebar footer: who is signed in, with theme, settings and sign-out behind it. */
export function UserMenu() {
  const user = useCurrentUser();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-ink-raised data-[state=open]:bg-ink-raised">
        <Avatar initials={user.initials} />
        <span className="min-w-0 flex-1 leading-tight">
          <span className="block truncate text-[13px] font-medium text-ink-foreground">{user.name}</span>
          <span className="block truncate text-xs text-ink-muted">
            {user.role ? titleCase(user.role) : "Account"}
          </span>
        </span>
        <ChevronsUpDown className="h-3.5 w-3.5 shrink-0 text-ink-muted" />
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="start" className="w-64">
        <MenuBody />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Compact avatar trigger for the mobile top bar. */
export function UserMenuCompact() {
  const user = useCurrentUser();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Account: ${user.name}`}
        className="flex h-9 w-9 items-center justify-center rounded-md transition-colors hover:bg-ink-raised"
      >
        <Avatar initials={user.initials} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <MenuBody />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
