"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import { useAuthStore } from "@/store/auth";

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const { token, hasHydrated } = useAuthStore();

  useEffect(() => {
    if (!hasHydrated) return;

    // We don't want to redirect if they are already on login or signup
    const isAuthRoute = pathname.includes("/login") || pathname.includes("/signup");

    if (!token && !isAuthRoute) {
      // Extract locale (e.g., "/fr/teams" -> "/fr")
      const locale = pathname.startsWith("/fr") ? "/fr" : "/en";
      router.replace(`${locale}/login`);
    }
  }, [token, hasHydrated, pathname, router]);

  // Optionally show a loading spinner while waiting for hydration,
  // but keeping it null prevents hydration mismatch flashes.
  if (!hasHydrated) {
    return null;
  }

  return <>{children}</>;
}
