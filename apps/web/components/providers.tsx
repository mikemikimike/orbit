"use client";

import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "sonner";
import { PwaRegister } from "@/components/pwa-register";
import { ConfirmProvider } from "@/components/ui/confirm";

export function Providers({ children }: { children: React.ReactNode }) {
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: 1,
            refetchOnWindowFocus: false,
            staleTime: 10_000,
          },
        },
      }),
  );
  return (
    <QueryClientProvider client={client}>
      <ConfirmProvider>{children}</ConfirmProvider>
      <Toaster
        position="bottom-right"
        offset={20}
        mobileOffset={{ bottom: 76 }}
        toastOptions={{
          classNames: {
            toast:
              "!rounded-lg !border !border-border !bg-popover !text-foreground !shadow-overlay !font-sans !text-sm",
            description: "!text-muted-foreground",
          },
        }}
        closeButton
      />
      <PwaRegister />
    </QueryClientProvider>
  );
}
