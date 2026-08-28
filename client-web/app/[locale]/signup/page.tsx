"use client";

import React, { useState } from "react";
import Image from "next/image";
import { Link } from "@/i18n/routing";
import { StepCredentials } from "@/components/onboarding/StepCredentials";
import { StepTeam } from "@/components/onboarding/StepTeam";
import { StepVerification } from "@/components/onboarding/StepVerification";
import type { OnboardingData, UpdateOnboardingData } from "@/components/onboarding/types";
import { useTranslations } from "next-intl";

const STORAGE_KEY = "opswarden_onboarding_draft";

function getInitialOnboardingState(): { step: number; data: OnboardingData } {
  const defaultData: OnboardingData = {
    email: "",
    password: "",
    mode: "create",
    teamName: "",
    invitationCode: "",
  };
  if (typeof window === "undefined") return { step: 1, data: defaultData };
  try {
    const saved = sessionStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      return {
        step:
          typeof parsed.step === "number" && parsed.step >= 1 && parsed.step <= 3 ? parsed.step : 1,
        data: { ...defaultData, ...(parsed.data || {}) },
      };
    }
  } catch {
    // Ignore storage read error
  }
  return { step: 1, data: defaultData };
}

export default function SignupPage() {
  const t = useTranslations("Auth");
  const tOnboarding = useTranslations("Onboarding");
  const [initialState] = useState(getInitialOnboardingState);
  const [step, setStep] = useState(initialState.step);
  const [data, setData] = useState<OnboardingData>(initialState.data);

  React.useEffect(() => {
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify({ step, data }));
    } catch {
      // Ignore storage write error
    }
  }, [step, data]);

  const updateData: UpdateOnboardingData = (fields) => {
    setData((prev) => ({ ...prev, ...fields }));
  };

  const next = () => setStep((prev) => Math.min(3, prev + 1));
  const back = () => setStep((prev) => Math.max(1, prev - 1));

  return (
    <section className="flex min-h-screen items-center justify-center p-4">
      <div className="glass flex w-full max-w-sm flex-col items-center gap-y-8 rounded-md px-6 py-12 shadow-sm">
        <div className="flex flex-col items-center gap-y-2">
          <div className="flex items-center gap-1 lg:justify-start">
            <Link href="/" className="flex items-center justify-center gap-3">
              <Image
                src="/assets/logo-icon.png"
                alt={t("logoIconAlt")}
                width={49}
                height={40}
                className="object-contain"
                priority
              />
              <Image
                src="/assets/logo-text-light.png"
                alt={t("logoWordmarkAlt")}
                width={207}
                height={32}
                className="object-contain"
                priority
              />
            </Link>
          </div>
        </div>

        <div className="flex w-full flex-col gap-4">
          {step === 1 && <StepCredentials data={data} updateData={updateData} next={next} />}
          {step === 2 && <StepTeam data={data} updateData={updateData} next={next} back={back} />}
          {step === 3 && <StepVerification data={data} back={back} />}
        </div>

        {step === 1 && (
          <div className="text-muted flex justify-center gap-1 text-sm">
            <p>{t("alreadyAccount")}</p>
            <Link href="/login" className="text-gold font-medium hover:underline">
              {t("login")}
            </Link>
          </div>
        )}

        <p className="text-muted mt-2 text-xs">{tOnboarding("signupProgress", { step })}</p>
      </div>
    </section>
  );
}
