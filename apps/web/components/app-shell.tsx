"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import {
  Activity,
  Bot,
  BookOpen,
  Boxes,
  Cable,
  ChevronDown,
  Cpu,
  LayoutDashboard,
  LogOut,
  Megaphone,
  Plus,
  Server,
  Terminal,
  UsersRound,
  Settings,
  ShieldCheck,
  Users,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { signOut } from "@/lib/session";
import { Button } from "@/components/ui/button";
import { ThemeToggle } from "@/components/theme-toggle";
import { getAccessToken } from "@/lib/api";
import { useDashboard, useMe, useMeta } from "@/lib/queries";

interface NavItem {
  href: string;
  label: string;
  icon: any;
}

interface NavGroup {
  label: string | null;
  items: NavItem[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    label: null,
    items: [
      { href: "/", label: "Dashboard", icon: LayoutDashboard },
      { href: "/work", label: "Work", icon: Activity },
      { href: "/agents", label: "Agents", icon: Bot },
      { href: "/agent-teams", label: "Agent Teams", icon: Users },
      { href: "/executions", label: "Executions", icon: Activity },
      { href: "/approvals", label: "Approvals", icon: ShieldCheck },
      { href: "/skills", label: "Skills", icon: BookOpen },
    ],
  },
  {
    label: "Library",
    items: [
      { href: "/connections", label: "Connectors", icon: Cable },
      { href: "/harnesses", label: "Harnesses", icon: Cpu },
      { href: "/models", label: "Providers", icon: Boxes },
      { href: "/environment-mappings", label: "Environments", icon: Terminal },
      { href: "/teams", label: "Teams", icon: UsersRound },
    ],
  },
  {
    label: "Operations",
    items: [
      { href: "/workers", label: "Workers", icon: Server },
      { href: "/outputs", label: "Outputs", icon: Megaphone },
      { href: "/settings", label: "Settings", icon: Settings },
    ],
  },
];

const MOBILE_NAV = [
  { href: "/", label: "Home", icon: LayoutDashboard },
  { href: "/agents", label: "Agents", icon: Bot },
  { href: "/agent-teams", label: "Teams", icon: Users },
  { href: "/executions", label: "Runs", icon: Activity },
  { href: "/approvals", label: "Approvals", icon: ShieldCheck },
  { href: "/settings", label: "Settings", icon: Settings },
];

function isActive(pathname: string, href: string) {
  if (href === "/") return pathname === "/";
  return pathname === href || pathname.startsWith(`${href}/`);
}

function BrandMark() {
  return (
    <span className="relative flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
      <span className="absolute h-5 w-5 rounded-full border-2 border-primary" />
      <span className="absolute h-1.5 w-1.5 -translate-y-2 rounded-full bg-primary" />
    </span>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname() || "/";
  const router = useRouter();
  const meta = useMeta();
  const dashboard = useDashboard();
  const me = useMe();
  const currentUser = me.data?.user as
    | { displayName?: string; email?: string; role?: string }
    | undefined;
  const displayName =
    currentUser?.displayName?.trim() ||
    currentUser?.email?.split("@")[0] ||
    "Account";
  const initials = displayName
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("");
  const pendingApprovals = dashboard.data?.summary.pendingApprovals ?? 0;
  const hideNewAgent = pathname.startsWith("/agents/new");
  const current = NAV_GROUPS.flatMap((g) => g.items).filter((item) => isActive(pathname, item.href)).at(-1);

  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  useEffect(() => {
    const auth = (meta.data?.auth ?? {}) as { mode?: string };
    const publicPath = pathname.startsWith("/login") || pathname.startsWith("/auth/");
    if (auth.mode === "oidc" && !publicPath && !getAccessToken()) {
      const next = pathname === "/" ? "/" : pathname;
      router.replace(`/login?next=${encodeURIComponent(next)}`);
    }
  }, [meta.data?.auth, pathname, router]);

  return (
    <div className="min-h-dvh bg-background">
      {/* Desktop sidebar */}
      <aside className="fixed inset-y-0 left-0 z-40 hidden w-60 flex-col border-r bg-card/60 backdrop-blur lg:flex">
        <div className="flex h-16 items-center gap-2.5 border-b px-5">
          <BrandMark />
          <div className="leading-tight">
            <p className="text-sm font-semibold tracking-wide">ORBIT</p>
            <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
              Agent platform
            </p>
          </div>
        </div>
        <nav className="flex flex-1 flex-col gap-1 overflow-y-auto p-3">
          {NAV_GROUPS.map((group, idx) => {
            const containsActive = group.items.some((item) => isActive(pathname, item.href));
            const isOpen = group.label === null || containsActive || !collapsed[group.label];
            return (
              <div key={group.label ?? `group-${idx}`} className={idx > 0 ? "mt-4" : ""}>
                {group.label ? (
                  <button
                    type="button"
                    onClick={() =>
                      setCollapsed((prev) => ({ ...prev, [group.label!]: !prev[group.label!] }))
                    }
                    className="flex w-full items-center justify-between px-3 py-1.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground hover:text-foreground"
                  >
                    {group.label}
                    <ChevronDown className={cn("h-3 w-3 transition-transform", !isOpen && "-rotate-90")} />
                  </button>
                ) : null}
                {isOpen ? (
                  <div className="space-y-1">
                    {group.items.map((item) => {
                      const active = isActive(pathname, item.href);
                      return (
                        <Link
                          key={item.href}
                          href={item.href}
                          className={cn(
                            "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                            active
                              ? "bg-accent text-accent-foreground"
                              : "text-muted-foreground hover:bg-muted hover:text-foreground",
                          )}
                        >
                          <item.icon className="h-4 w-4" />
                          <span className="flex-1">{item.label}</span>
                          {item.href === "/approvals" && pendingApprovals > 0 ? (
                            <span className="rounded-full bg-[hsl(var(--warning))] px-1.5 text-[10px] font-semibold text-white">
                              {pendingApprovals}
                            </span>
                          ) : null}
                        </Link>
                      );
                    })}
                  </div>
                ) : null}
              </div>
            );
          })}
        </nav>
        <div className="border-t p-3">
          <div className="mb-2 flex items-center justify-between gap-2">
            <Link
              href="/settings"
              className="flex min-w-0 items-center gap-2 rounded-lg px-1 py-1 text-left hover:bg-muted"
              title={currentUser?.email ?? displayName}
            >
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-muted text-[11px] font-semibold">
                {initials || "?"}
              </span>
              <span className="min-w-0 leading-tight">
                <span className="block truncate text-xs font-medium">{displayName}</span>
                <span className="block truncate text-[10px] text-muted-foreground">
                  {currentUser?.role ?? "View account"}
                </span>
              </span>
            </Link>
            <div className="flex items-center">
              <ThemeToggle />
              <Button
                variant="ghost"
                size="icon"
                aria-label="Sign out"
                title="Sign out"
                onClick={signOut}
              >
                <LogOut />
              </Button>
            </div>
          </div>
          <div className="rounded-lg bg-muted/60 p-3">
            <p className="text-xs font-medium">Self-hosted</p>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              Your agents, harnesses and credentials stay in your infrastructure.
            </p>
          </div>
        </div>
      </aside>

      {/* Mobile header */}
      <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b bg-background/85 px-4 backdrop-blur lg:hidden">
        <Link href="/" className="flex items-center gap-2">
          <BrandMark />
          <span className="text-sm font-semibold">ORBIT</span>
          <span className="text-xs text-muted-foreground">/ {current?.label ?? "Dashboard"}</span>
        </Link>
        <div className="flex items-center gap-1">
          <Link
            href="/settings"
            className="flex max-w-[8.5rem] items-center gap-1.5 rounded-lg px-1 py-1 text-xs hover:bg-muted"
            title={currentUser?.email ?? displayName}
          >
            <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-muted text-[10px] font-semibold">
              {initials || "?"}
            </span>
            <span className="truncate">{displayName}</span>
          </Link>
          {!hideNewAgent ? (
            <Button asChild size="icon" variant="default" aria-label="New agent">
              <Link href="/agents/new">
                <Plus />
              </Link>
            </Button>
          ) : null}
          <Button
            variant="ghost"
            size="icon"
            aria-label="Sign out"
            title="Sign out"
            onClick={signOut}
          >
            <LogOut />
          </Button>
          <ThemeToggle />
        </div>
      </header>

      <div className="lg:pl-60">
        <div className="orbit-grid-bg min-h-dvh">
          <main className="mx-auto w-full max-w-6xl px-4 pb-24 pt-5 sm:px-6 lg:pb-10 lg:pt-8">
            {children}
          </main>
        </div>
      </div>

      {/* Mobile bottom navigation */}
      <nav className="safe-bottom fixed inset-x-0 bottom-0 z-40 grid grid-cols-6 border-t bg-background/95 backdrop-blur lg:hidden">
        {MOBILE_NAV.map((item) => {
          const active = isActive(pathname, item.href);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "flex flex-col items-center gap-1 px-1 pt-2 text-[10px] font-medium",
                active ? "text-primary" : "text-muted-foreground",
              )}
            >
              <span className="relative">
                <item.icon className={cn("h-5 w-5", active && "fill-primary/10")} />
                {item.href === "/approvals" && pendingApprovals > 0 ? (
                  <span className="absolute -right-2 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-[hsl(var(--warning))] px-1 text-[9px] font-semibold text-white">
                    {pendingApprovals > 9 ? "9+" : pendingApprovals}
                  </span>
                ) : null}
              </span>
              {item.label}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
