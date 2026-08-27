import { useEffect, useState } from "react";
import { LoaderCircle } from "lucide-react";
import { useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { useRouter } from "@/i18n/routing";
import { teamPath } from "@/lib/team-routing";
import type { OnboardingData } from "./types";

export function StepVerification({ data, back }: { data: OnboardingData; back: () => void }) {
  const router = useRouter();
  const t = useTranslations("Onboarding");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isCancelled = false;
    let redirectTimer: ReturnType<typeof setTimeout> | undefined;

    const createWorkspace = async () => {
      try {
        const signupRes = await fetch("/api/auth/sign-up", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email: data.email, password: data.password }),
        });
        if (!signupRes.ok) throw new Error("signup_failed");

        const signinRes = await fetch("/api/auth/sign-in", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ email: data.email, password: data.password }),
        });
        if (!signinRes.ok) throw new Error("signin_after_signup_failed");

        const { token } = await signinRes.json();
        const { useAuthStore } = await import("@/store/auth");
        const { apiFetch } = await import("@/lib/api");
        useAuthStore.getState().setToken(token);

        const meRes = await apiFetch("/api/me");
        if (!meRes.ok) throw new Error("profile_load_failed");

        const user = await meRes.json();
        useAuthStore.getState().setUser(user);

        let createdTeamId = "";
        if (data.teamName) {
          const teamRes = await apiFetch("/api/teams", {
            method: "POST",
            body: JSON.stringify({ name: data.teamName }),
          });
          if (teamRes.ok) createdTeamId = await teamRes.text();
        }

        if (!isCancelled) {
          redirectTimer = setTimeout(() => {
            router.push(createdTeamId ? teamPath(createdTeamId) : "/", {
              locale: user.locale,
            });
          }, 300);
        }
      } catch (caught: unknown) {
        if (isCancelled) return;
        const code = caught instanceof Error && caught.message ? caught.message : "unknown";
        setError(t.has(code) ? t(code) : t("unknownError"));
      }
    };

    createWorkspace();
    return () => {
      isCancelled = true;
      if (redirectTimer) clearTimeout(redirectTimer);
    };
  }, [data, router, t]);

  return (
    <div className="mx-auto w-full space-y-4">
      {error ? (
        <>
          <Alert tone="danger">{error}</Alert>
          <Button fullWidth size="lg" onClick={back}>
            {t("back")}
          </Button>
        </>
      ) : (
        <div className="surface flex min-h-40 flex-col items-center justify-center gap-3 rounded-md p-6 text-center">
          <LoaderCircle className="text-gold h-6 w-6 animate-spin" aria-hidden="true" />
          <p className="text-text font-medium">{t("creatingWorkspace")}</p>
        </div>
      )}
    </div>
  );
}
