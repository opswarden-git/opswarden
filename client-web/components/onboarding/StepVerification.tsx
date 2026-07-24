import React, { useEffect, useState, useRef } from "react";
import { useRouter } from "@/i18n/routing";
import type { OnboardingData } from "./types";
import { teamPath } from "@/lib/team-routing";
import { useTranslations } from "next-intl";

interface StepProps {
  data: OnboardingData;
}

const CONSOLE_LOG_KEYS = [
  "logInitializing",
  "logGeneratingKeypair",
  "logConnecting",
  "logAuthorizing",
  "logConnected",
  "logConfiguringMetrics",
  "logResolvingEndpoints",
  "logSystemOnline",
] as const;

export function StepVerification({ data }: StepProps) {
  const router = useRouter();
  const t = useTranslations("Onboarding");
  const [logs, setLogs] = useState<string[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);

  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let currentIdx = 0;
    let isCancelled = false;
    let interval: NodeJS.Timeout;

    const performAuth = async () => {
      try {
        // 1. Sign up
        const signupRes = await fetch("/api/auth/sign-up", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email: data.email, password: data.password }),
        });

        if (!signupRes.ok) {
          throw new Error("signup_failed");
        }

        // 2. Sign in
        const signinRes = await fetch("/api/auth/sign-in", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email: data.email, password: data.password }),
        });

        if (!signinRes.ok) {
          throw new Error("signin_after_signup_failed");
        }

        const { token } = await signinRes.json();

        const { useAuthStore } = await import("@/store/auth");
        const { apiFetch } = await import("@/lib/api");

        useAuthStore.getState().setToken(token);

        // 3. Fetch /me
        const meRes = await apiFetch("/api/me");
        if (meRes.ok) {
          const user = await meRes.json();
          useAuthStore.getState().setUser(user);

          // 4. Create the Team (stationName)
          let createdTeamId = "";
          if (data.stationName) {
            const teamRes = await apiFetch("/api/teams", {
              method: "POST",
              body: JSON.stringify({ name: data.stationName }),
            });
            if (teamRes.ok) createdTeamId = await teamRes.text();
          }

          if (!isCancelled) {
            // Delay redirection slightly so user sees system online
            setTimeout(() => {
              router.push(createdTeamId ? teamPath(createdTeamId) : "/", {
                locale: user.locale,
              });
            }, 1200);
          }
        } else {
          throw new Error("profile_load_failed");
        }
      } catch (err: unknown) {
        if (!isCancelled) {
          const code = err instanceof Error && err.message ? err.message : "unknown";
          const message = t.has(code) ? t(code) : t("unknownError");
          setError(message);
          clearInterval(interval);
          setLogs((prev) => [...prev, `[ERROR] ${message}`]);
        }
      }
    };

    interval = setInterval(() => {
      if (currentIdx < CONSOLE_LOG_KEYS.length) {
        setLogs((prev) => [
          ...prev,
          `[${new Date().toLocaleTimeString()}] ${t(CONSOLE_LOG_KEYS[currentIdx])}`,
        ]);
        currentIdx++;
      } else {
        clearInterval(interval);
      }
    }, 450);

    performAuth();

    return () => {
      isCancelled = true;
      clearInterval(interval);
    };
  }, [router, data, t]);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div className="mx-auto w-full space-y-6">
      <div
        ref={containerRef}
        className="scrollbar-thumb-muted/10 surface text-st-res h-64 w-full scrollbar-thin space-y-1.5 overflow-y-auto rounded-md p-4 font-mono text-[10px] shadow-inner"
      >
        <div>{t("bootLoader")}</div>
        <div>{t("operatorLog", { name: data.operatorName || t("unknown") })}</div>
        <div>{t("stationLog", { name: data.stationName || t("unknown") })}</div>
        <div className="border-border my-2 border-t"></div>
        {logs.map((log, i) => (
          <div key={i} className="animate-fade-in">
            {log}
          </div>
        ))}
        {logs.length < CONSOLE_LOG_KEYS.length && <div className="animate-pulse">_</div>}
      </div>
    </div>
  );
}
