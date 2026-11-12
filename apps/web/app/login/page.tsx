"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { KeyRound, ShieldCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getAccessToken } from "@/lib/api";
import { beginOidcLogin } from "@/lib/oidc";
import { useMeta } from "@/lib/queries";

export default function LoginPage() {
  const meta = useMeta();
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);
  const auth = (meta.data?.auth ?? {}) as { mode?: string; issuer?: string };
  const oidc = auth.mode === "oidc";

  useEffect(() => {
    if (getAccessToken()) router.replace("/");
  }, [router]);

  return (
    <div className="mx-auto flex min-h-[70vh] max-w-md items-center">
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="h-5 w-5 text-primary" /> Sign in to ORBIT
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {oidc ? (
            <>
              <p className="text-sm text-muted-foreground">
                Continue with your organization identity provider.
              </p>
              {auth.issuer ? (
                <p className="break-all text-[11px] text-muted-foreground">{auth.issuer}</p>
              ) : null}
              <Button
                className="w-full"
                onClick={() =>
                  void beginOidcLogin(
                    new URLSearchParams(window.location.search).get("next") ?? "/",
                  ).catch((reason) =>
                    setError(reason instanceof Error ? reason.message : String(reason)),
                  )
                }
              >
                <KeyRound /> Continue with SSO
              </Button>
            </>
          ) : (
            <>
              <p className="text-sm text-muted-foreground">
                Development authentication is active. Continue to the control plane, or
                configure OIDC/Dex/Keycloak for production.
              </p>
              <Button asChild className="w-full">
                <Link href="/">Continue to ORBIT</Link>
              </Button>
            </>
          )}
          {error ? <p className="text-xs text-destructive">{error}</p> : null}
        </CardContent>
      </Card>
    </div>
  );
}
