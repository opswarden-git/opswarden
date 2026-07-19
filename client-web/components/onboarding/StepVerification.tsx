import React, { useEffect, useState, useRef } from "react";
import { useRouter } from "@/i18n/routing";
import type { OnboardingData } from "./types";
import { teamPath } from "@/lib/team-routing";

interface StepProps {
  data: OnboardingData;
}

const CONSOLE_LOGS = [
  "INITIALIZING SECURITY PROTOCOLS...",
  "GENERATING CRYPTOGRAPHIC KEYPAIR...",
  "CONNECTING TO ENCRYPTED STREAM SERVER...",
  "AUTHORIZING OPERATOR CLEARANCE...",
  "STREAM ESTABLISHED: Paris-Core-NOC-1 [CONNECTED]",
  "CONFIGURING PROMETHEUS SCRAPING STREAM...",
  "RESOLVING INGESTION ENDPOINTS...",
  "SYSTEM ONLINE. PREPARING LAUNCH CONTROLLER...",
];

export function StepVerification({ data }: StepProps) {
  const router = useRouter();
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
          throw new Error("Failed to create account (email might be taken)");
        }

        // 2. Sign in
        const signinRes = await fetch("/api/auth/sign-in", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email: data.email, password: data.password }),
        });

        if (!signinRes.ok) {
          throw new Error("Login failed after signup");
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
              router.push(createdTeamId ? teamPath(createdTeamId) : "/");
            }, 1200);
          }
        } else {
          throw new Error("Failed to load user profile");
        }
      } catch (err: unknown) {
        if (!isCancelled) {
          const message = err instanceof Error && err.message ? err.message : "An error occurred";
          setError(message);
          clearInterval(interval);
          setLogs((prev) => [...prev, `[ERROR] ${message}`]);
        }
      }
    };

    interval = setInterval(() => {
      if (currentIdx < CONSOLE_LOGS.length) {
        setLogs((prev) => [
          ...prev,
          `[${new Date().toLocaleTimeString()}] ${CONSOLE_LOGS[currentIdx]}`,
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
  }, [router, data]);

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
        <div>SYSTEM BOOT LOADER v1.2.0-STABLE</div>
        <div>OPERATOR: {data.operatorName || "UNKNOWN"}</div>
        <div>STATION: {data.stationName || "UNKNOWN"}</div>
        <div className="border-border my-2 border-t"></div>
        {logs.map((log, i) => (
          <div key={i} className="animate-fade-in">
            {log}
          </div>
        ))}
        {logs.length < CONSOLE_LOGS.length && <div className="animate-pulse">_</div>}
      </div>
    </div>
  );
}
