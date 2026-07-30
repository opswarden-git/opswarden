import React from "react";
import { ChevronLeft } from "lucide-react";
import Image from "next/image";
import type { OnboardingData, UpdateOnboardingData } from "./types";
import { Button } from "@/components/ui/Button";
import { useTranslations } from "next-intl";

interface StepProps {
  data: OnboardingData;
  updateData: UpdateOnboardingData;
  next: () => void;
  back: () => void;
}

const AVAILABLE_INTEGRATIONS = [
  {
    id: "github",
    name: "GitHub",
    descriptionKey: "githubDescription",
    icon: "/assets/github-patched.webp",
  },
  {
    id: "gitlab",
    name: "GitLab",
    descriptionKey: "gitlabDescription",
    icon: "/assets/gitlab.webp",
  },
  {
    id: "k8s",
    name: "Kubernetes",
    descriptionKey: "kubernetesDescription",
    icon: "/assets/kubernetes.webp",
  },
  {
    id: "sentry",
    name: "Sentry",
    descriptionKey: "sentryDescription",
    icon: "/assets/sentry.webp",
  },
  {
    id: "datadog",
    name: "Datadog",
    descriptionKey: "datadogDescription",
    icon: "/assets/datadog.webp",
  },
  {
    id: "pagerduty",
    name: "PagerDuty",
    descriptionKey: "pagerDutyDescription",
    icon: "/assets/pagerduty.webp",
  },
];

export function StepIntegrations({ next, back }: StepProps) {
  const t = useTranslations("Onboarding");

  return (
    <div className="mx-auto w-full space-y-6">
      <p className="text-muted text-xs leading-relaxed">{t("integrationsPreview")}</p>
      <div className="flex flex-col gap-2">
        {AVAILABLE_INTEGRATIONS.map((integ) => {
          return (
            <div
              key={integ.id}
              className="surface-subtle border-border flex items-center justify-between rounded-md border p-3 transition-colors hover:bg-white/[0.055]"
            >
              <div className="flex min-w-0 items-center gap-4 pr-4">
                <div className="flex shrink-0 items-center justify-center">
                  <Image
                    src={integ.icon}
                    alt={integ.name}
                    width={24}
                    height={24}
                    className="size-5 object-contain"
                  />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-text truncate text-sm font-medium">{integ.name}</span>
                  </div>
                  <p className="text-muted mt-0.5 truncate text-xs">{t(integ.descriptionKey)}</p>
                </div>
              </div>

              <Button size="sm" disabled>
                {t("configureLater")}
              </Button>
            </div>
          );
        })}
      </div>

      <div className="mt-2 flex items-center justify-between pt-4">
        <Button variant="ghost" size="lg" onClick={back}>
          <ChevronLeft className="size-4" />
          {t("back")}
        </Button>
        <Button variant="primary" size="lg" onClick={next}>
          {t("skipForNow")}
        </Button>
      </div>
    </div>
  );
}
