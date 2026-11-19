"use client";

import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";

import { BrandMark, Wordmark } from "@/components/shell/brand";
import { CommandPaletteProvider } from "@/components/shell/command-palette";
import { MobileHeader, MobileTabBar } from "@/components/shell/mobile-nav";
import { pageLabel } from "@/components/shell/nav";
import { Sidebar } from "@/components/shell/sidebar";
import { getAccessToken, setAccessToken } from "@/lib/api";
import { useDashboard, useMe, useMeta } from "@/lib/queries";

function isPublicPath(pathname: string) {
  return pathname.startsWith("/login") || pathname.startsWith("/auth/");
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname() || "/";
  const router = useRouter();
  const meta = useMeta();
  const publicPath = isPublicPath(pathname);

  const me = useMe();
  useEffect(() => {
    const auth = (meta.data?.auth ?? {}) as { mode?: string };
    if (auth.mode !== "oidc" || publicPath) return;
    const login = `/login?next=${encodeURIComponent(pathname)}`;
    if (!getAccessToken()) {
      router.replace(login);
    } else if (me.isError) {
      // Expired or foreign token: clear it instead of treating it as a session.
      setAccessToken(null);
      router.replace(login);
    }
  }, [meta.data?.auth, me.isError, pathname, publicPath, router]);

  const title = publicPath ? "Sign in · ORBIT" : `${pageLabel(pathname)} · ORBIT`;

  if (publicPath) {
    // Signed-out screens get no navigation: nothing behind it is reachable yet.
    return (
      <div data-ink className="flex min-h-dvh flex-col bg-background text-ink-foreground">
        <title>{title}</title>
        <div className="flex h-14 items-center gap-2.5 px-5">
          <BrandMark />
          <Wordmark />
        </div>
        <main id="main" className="flex flex-1 items-center justify-center px-4 pb-20">
          {children}
        </main>
      </div>
    );
  }

  return (
    <>
      {/* React hoists <title> into <head>; the root layout sets none, so this is the only one. */}
      <title>{title}</title>
      <SignedInShell pathname={pathname}>{children}</SignedInShell>
    </>
  );
}

function SignedInShell({ pathname, children }: { pathname: string; children: React.ReactNode }) {
  const dashboard = useDashboard();
  const signals = {
    pendingApprovals: dashboard.data?.summary.pendingApprovals ?? 0,
    runningExecutions: dashboard.data?.summary.runningExecutions ?? 0,
  };

  return (
    <CommandPaletteProvider>
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-[60] focus:rounded-md focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:font-medium focus:text-primary-foreground"
      >
        Skip to content
      </a>
      <div className="min-h-dvh bg-background">
        <Sidebar pathname={pathname} signals={signals} />
        <MobileHeader />
        <div className="lg:pl-[15.5rem]">
          <main
            id="main"
            className="mx-auto w-full max-w-[1180px] px-4 pb-28 pt-5 sm:px-6 lg:px-10 lg:pb-14 lg:pt-14"
          >
            {children}
          </main>
        </div>
        <MobileTabBar pathname={pathname} signals={signals} />
      </div>
    </CommandPaletteProvider>
  );
}
