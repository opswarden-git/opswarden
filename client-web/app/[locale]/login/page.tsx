"use client";

import React, { useCallback, useEffect, useState } from "react";
import Image from "next/image";
import { Link, useRouter } from "@/i18n/routing";
import { Eye, EyeOff } from "lucide-react";
import { FcGoogle } from "react-icons/fc";
import { Alert } from "@/components/ui/Alert";
import { Button, IconButton } from "@/components/ui/Button";
import { useTranslations } from "next-intl";
import { teamPath } from "@/lib/team-routing";
import type { Team } from "@/lib/queries/teams";

export default function LoginPage() {
  const router = useRouter();
  const t = useTranslations("Auth");
  const tErr = useTranslations("errors");
  const [showPassword, setShowPassword] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Map a backend error code (stable, snake_case) to a localized message,
  // falling back to the raw server message and then a generic one.
  const errorMessage = (code?: string, fallback?: string) =>
    code && tErr.has(code) ? tErr(code) : (fallback ?? tErr("unknown"));

  const completeLogin = useCallback(
    async (token: string) => {
      const { useAuthStore } = await import("@/store/auth");
      const { apiFetch } = await import("@/lib/api");

      useAuthStore.getState().setToken(token);

      const meRes = await apiFetch("/api/me");
      if (!meRes.ok) {
        throw new Error("Failed to fetch user profile");
      }

      const user = await meRes.json();
      useAuthStore.getState().setUser(user);

      const teamsRes = await apiFetch("/api/teams");
      if (teamsRes.ok) {
        const teams = (await teamsRes.json()) as Team[];
        router.push(teams[0] ? teamPath(teams[0].team_id) : "/settings?setup=station");
        return;
      }

      router.push("/");
    },
    [router],
  );

  useEffect(() => {
    const hash = new URLSearchParams(window.location.hash.replace(/^#/, ""));
    const oauthToken = hash.get("oauth_token");
    if (!oauthToken) return;

    window.history.replaceState(null, "", window.location.pathname);
    void (async () => {
      setLoading(true);
      try {
        await completeLogin(oauthToken);
      } catch {
        setError(tErr("unknown"));
      } finally {
        setLoading(false);
      }
    })();
  }, [completeLogin, tErr]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      const res = await fetch("/api/auth/sign-in", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      });

      if (!res.ok) {
        const body = await res.json().catch(() => null);
        setError(errorMessage(body?.code, body?.error));
        return;
      }

      const { token } = await res.json();
      await completeLogin(token);
    } catch {
      setError(tErr("unknown"));
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="flex min-h-screen items-center justify-center p-4">
      <div className="glass flex w-full max-w-sm flex-col items-center gap-y-8 rounded-md px-6 py-12 shadow-sm">
        <div className="flex flex-col items-center gap-y-2">
          <div className="flex items-center gap-1 lg:justify-start">
            <Link href="/" className="flex items-center justify-center gap-3">
              <Image
                src="/assets/logo-icon.png"
                alt="Icon"
                width={49}
                height={40}
                className="object-contain"
                priority
              />
              <Image
                src="/assets/logo-text-light.png"
                alt="OpsWarden"
                width={207}
                height={32}
                className="object-contain"
                priority
              />
            </Link>
          </div>
        </div>
        <div className="flex w-full flex-col gap-8">
          <div className="flex flex-col gap-4">
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <label htmlFor="login-email" className="text-muted text-xs font-medium">
                  {t("email")}
                </label>
                <input
                  id="login-email"
                  type="email"
                  placeholder={t("emailPlaceholder")}
                  required
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
                />
              </div>
              <div className="flex flex-col gap-2">
                <label htmlFor="login-password" className="text-muted text-xs font-medium">
                  {t("password")}
                </label>
                <div className="relative">
                  <input
                    id="login-password"
                    type={showPassword ? "text" : "password"}
                    placeholder="••••••••"
                    required
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className={`ow-input ${showPassword ? "text-text" : "text-muted-2"} caret-gold placeholder:text-muted-2 flex h-10 w-full rounded-md px-3 py-2 pr-10 text-sm transition-colors`}
                  />
                  <IconButton
                    label={showPassword ? t("hidePassword") : t("showPassword")}
                    size="sm"
                    variant="ghost"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute top-1/2 right-1 -translate-y-1/2"
                  >
                    {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                  </IconButton>
                </div>
              </div>
              {error ? (
                <Alert tone="danger" className="text-center">
                  {error}
                </Alert>
              ) : null}
              <div className="mt-2 flex flex-col gap-4">
                <Button type="submit" variant="primary" size="lg" fullWidth loading={loading}>
                  {loading ? t("loggingIn") : t("login")}
                </Button>
                <Button
                  size="lg"
                  fullWidth
                  onClick={() => {
                    const locale = window.location.pathname.startsWith("/fr") ? "fr" : "en";
                    window.location.href = `/api/auth/google/start?locale=${locale}`;
                  }}
                >
                  <FcGoogle className="size-5" />
                  {t("loginWithGoogle")}
                </Button>
              </div>
            </form>
          </div>
        </div>
        <div className="text-muted flex justify-center gap-1 text-sm">
          <p>{t("noAccount")}</p>
          <Link href="/signup" className="text-gold font-medium hover:underline">
            {t("signup")}
          </Link>
        </div>
      </div>
    </section>
  );
}
