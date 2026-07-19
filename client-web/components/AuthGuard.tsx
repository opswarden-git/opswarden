"use client";

import type { ReactNode } from "react";
import { useEffect } from "react";
import { usePathname, useRouter } from "@/i18n/routing";
import { useAuthStore } from "@/store/auth";

const PUBLIC_AUTH_ROUTES = new Set(["/login", "/signup"]);

export function isPublicAuthRoute(pathname: string) {
  return PUBLIC_AUTH_ROUTES.has(pathname);
}

export function AuthGuard({ children }: { children: ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const { token, hasHydrated } = useAuthStore();
  const isAuthRoute = isPublicAuthRoute(pathname);

  useEffect(() => {
    if (!hasHydrated) return;

    if (!token && !isAuthRoute) {
      router.replace("/login");
    }
  }, [hasHydrated, isAuthRoute, router, token]);

  if (!hasHydrated || (!token && !isAuthRoute)) {
    return null;
  }

  return <>{children}</>;
}
