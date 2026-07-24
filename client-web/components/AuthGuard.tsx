"use client";

import type { ReactNode } from "react";
import { useEffect } from "react";
import { useParams } from "next/navigation";
import { usePathname, useRouter } from "@/i18n/routing";
import { isAppLocale } from "@/i18n/locales";
import { useProfile } from "@/lib/queries/profile";
import { useAuthStore } from "@/store/auth";

const PUBLIC_AUTH_ROUTES = new Set(["/login", "/signup"]);

export function isPublicAuthRoute(pathname: string) {
  return PUBLIC_AUTH_ROUTES.has(pathname);
}

export function AuthGuard({ children }: { children: ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = typeof params.locale === "string" ? params.locale : "";
  const { token, hasHydrated, setUser } = useAuthStore();
  const isAuthRoute = isPublicAuthRoute(pathname);
  const profile = useProfile(hasHydrated && !!token);

  useEffect(() => {
    if (!hasHydrated) return;

    if (!token && !isAuthRoute) {
      router.replace("/login");
    }
  }, [hasHydrated, isAuthRoute, router, token]);

  useEffect(() => {
    if (!profile.data) return;
    setUser(profile.data);
    if (isAppLocale(currentLocale) && profile.data.locale !== currentLocale) {
      router.replace(pathname, { locale: profile.data.locale });
    }
  }, [currentLocale, pathname, profile.data, router, setUser]);

  if (!hasHydrated || (!token && !isAuthRoute)) {
    return null;
  }

  return <>{children}</>;
}
