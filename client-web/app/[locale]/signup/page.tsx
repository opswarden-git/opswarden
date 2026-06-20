"use client";

import React, { useState } from "react";
import Image from "next/image";
import { Link } from "@/i18n/routing";
import { StepCredentials } from "@/components/onboarding/StepCredentials";
import { StepStation } from "@/components/onboarding/StepStation";
import { StepIntegrations } from "@/components/onboarding/StepIntegrations";
import { StepVerification } from "@/components/onboarding/StepVerification";
import { useTranslations } from "next-intl";

export default function SignupPage() {
  const t = useTranslations("Auth");
  const [step, setStep] = useState(1);
  const [data, setData] = useState({
    operatorName: "",
    email: "",
    password: "",
    stationName: "",
    timezone: "Europe/Paris",
    clearance: "observer",
    integrations: [] as string[],
  });

  const updateData = (fields: Partial<typeof data>) => {
    setData((prev) => ({ ...prev, ...fields }));
  };

  const next = () => setStep((prev) => prev + 1);
  const back = () => setStep((prev) => prev - 1);

  return (
    <section className="flex min-h-screen items-center justify-center p-4">
      <div className="glass flex w-full max-w-sm flex-col items-center gap-y-8 rounded-md px-6 py-12 shadow-sm">
        <div className="flex flex-col items-center gap-y-2">
          <div className="flex items-center gap-1 lg:justify-start">
            <Link href="/" className="flex items-center justify-center gap-3">
              <Image
                src="/assets/logo-icon.png"
                alt="Icon"
                width={40}
                height={40}
                className="h-10 w-auto object-contain"
                style={{ width: "auto", height: "auto" }}
                priority
              />
              <Image
                src="/assets/logo-text-light.png"
                alt="OpsWarden"
                width={240}
                height={48}
                className="h-8 w-auto object-contain"
                style={{ width: "auto", height: "auto" }}
                priority
              />
            </Link>
          </div>
        </div>

        <div className="flex w-full flex-col gap-4">
          {step === 1 && <StepCredentials data={data} updateData={updateData} next={next} />}
          {step === 2 && (
            <StepStation data={data} updateData={updateData} next={next} back={back} />
          )}
          {step === 3 && (
            <StepIntegrations data={data} updateData={updateData} next={next} back={back} />
          )}
          {step === 4 && <StepVerification data={data} />}
        </div>

        {step === 1 && (
          <div className="text-muted flex justify-center gap-1 text-sm">
            <p>{t("alreadyAccount")}</p>
            <Link href="/login" className="text-gold font-medium hover:underline">
              {t("login")}
            </Link>
          </div>
        )}

        <div className="mt-2 flex items-center gap-4">
          {[1, 2, 3, 4].map((s) => (
            <div
              key={s}
              className={`h-2.5 w-2.5 rounded-full transition-colors duration-300 ${
                s <= step ? "bg-gold" : "bg-border"
              }`}
            />
          ))}
        </div>
      </div>
    </section>
  );
}
