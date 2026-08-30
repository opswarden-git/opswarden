"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";
import { registerSessionQueryClient } from "@/lib/sessionLifecycle";

function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 60 * 1000, // 1 minute
        refetchOnWindowFocus: false, // Less noisy defaults
      },
    },
  });
}

export function Providers({ children }: { children: React.ReactNode }) {
  // Ensure we only create the QueryClient once per session in the client
  const [queryClient, setQueryClient] = useState(createQueryClient);
  const replaceQueryClient = useCallback(() => {
    const replacement = createQueryClient();
    setQueryClient(replacement);
    return replacement;
  }, []);

  useEffect(
    () => registerSessionQueryClient(queryClient, replaceQueryClient),
    [queryClient, replaceQueryClient],
  );

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}
