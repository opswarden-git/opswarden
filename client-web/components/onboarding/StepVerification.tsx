import { useEffect, useRef, useState } from "react";
import { LoaderCircle } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { useRouter } from "@/i18n/routing";
import { teamPath } from "@/lib/team-routing";
import { clearOnboardingDraft } from "./onboardingDraft";
import { completeOnboarding, completeTeamOnboarding } from "./onboardingFlow";
import type { OnboardingData } from "./types";

export function StepVerification({
  data,
  back,
  existingSession = false,
}: {
  data: OnboardingData;
  back: () => void;
  existingSession?: boolean;
}) {
  const router = useRouter();
  const locale = useLocale();
  const t = useTranslations("Onboarding");
  const [error, setError] = useState<string | null>(null);
  /**
   * This step signs up, signs in and creates or joins a team — none of it
   * idempotent. React invokes an effect twice on mount in development, which
   * is exactly how this surfaced: the first pass created the account and the
   * workspace, the second got a 409 on the same email, and its failure landed
   * last. The user was told account creation failed while both had succeeded.
   */
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    // No cancellation: by the time this runs an account is being created, and
    // abandoning halfway leaves it orphaned. The `started` ref above is what
    // keeps it to one run; the cleanup used to cancel the first pass's own
    // redirect and leave the spinner turning forever.

    const createWorkspace = async () => {
      try {
        const result = existingSession
          ? { teamId: await completeTeamOnboarding(data), locale }
          : await completeOnboarding(data).then(({ user, teamId }) => ({
              teamId,
              locale: user.locale,
            }));
        clearOnboardingDraft();
        setTimeout(() => {
          router.push(teamPath(result.teamId), { locale: result.locale });
        }, 300);
      } catch (caught: unknown) {
        const code = caught instanceof Error && caught.message ? caught.message : "unknown";
        setError(t.has(code) ? t(code) : t("unknownError"));
      }
    };

    createWorkspace();
  }, [data, existingSession, locale, router, t]);

  const mode = data.mode || "create";
  const loadingText = mode === "join" ? t("joiningWorkspace") : t("creatingWorkspace");

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
          <p className="text-text font-medium">{loadingText}</p>
        </div>
      )}
    </div>
  );
}
