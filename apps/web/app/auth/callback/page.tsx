"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { completeOidcLogin } from "@/lib/oidc";

export default function OidcCallbackPage() {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void completeOidcLogin()
      .then((next) => router.replace(next))
      .catch((reason) =>
        setError(reason instanceof Error ? reason.message : String(reason)),
      );
  }, [router]);

  return (
    <div className="mx-auto flex min-h-[70vh] max-w-md items-center">
      <Card className="w-full">
        <CardHeader>
          <CardTitle>Completing sign-in</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {error ? (
            <>
              <p className="text-sm text-destructive">{error}</p>
              <Button onClick={() => router.replace("/login")}>Try again</Button>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">Exchanging the authorization code…</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
