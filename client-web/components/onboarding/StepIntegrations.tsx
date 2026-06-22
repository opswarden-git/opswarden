import React from "react";
import { ChevronLeft } from "lucide-react";
import Image from "next/image";
import type { OnboardingData, UpdateOnboardingData } from "./types";

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
    desc: "Link actions & deployment flows",
    icon: "/assets/github-patched.webp",
  },
  {
    id: "gitlab",
    name: "GitLab",
    desc: "Sync pipelines and issue boards",
    icon: "/assets/gitlab.webp",
  },
  {
    id: "k8s",
    name: "Kubernetes",
    desc: "Deploy container metrics monitor",
    icon: "/assets/kubernetes.webp",
  },
  {
    id: "sentry",
    name: "Sentry",
    desc: "Track application exceptions & crashes",
    icon: "/assets/sentry.webp",
  },
  {
    id: "datadog",
    name: "Datadog",
    desc: "Sync system APM telemetry data",
    icon: "/assets/datadog.webp",
  },
  {
    id: "pagerduty",
    name: "PagerDuty",
    desc: "Sync incident & rotation escalations",
    icon: "/assets/pagerduty.webp",
  },
];

export function StepIntegrations({ next, back }: StepProps) {
  return (
    <div className="mx-auto w-full space-y-6">
      <p className="text-muted text-xs leading-relaxed">
        Workspace creation is live now. Service tokens are configured after onboarding through the
        server-side vault, so this step is only a connector preview.
      </p>
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
                  <p className="text-muted mt-0.5 truncate text-xs">{integ.desc}</p>
                </div>
              </div>

              <button
                type="button"
                disabled
                className="text-muted surface-subtle border-border shrink-0 cursor-not-allowed rounded-md border px-3 py-1.5 text-xs font-medium"
              >
                Configure later
              </button>
            </div>
          );
        })}
      </div>

      <div className="mt-2 flex items-center justify-between pt-4">
        <button
          type="button"
          onClick={back}
          className="text-muted hover:text-text focus-visible:ring-gold flex h-10 items-center justify-center rounded-md px-3 text-sm font-medium transition-colors focus-visible:ring-1 focus-visible:outline-none"
        >
          <ChevronLeft className="mr-1 size-4" />
          Back
        </button>
        <button
          type="button"
          onClick={next}
          className="ow-primary focus-visible:ring-gold focus-visible:ring-offset-bg inline-flex h-10 items-center justify-center rounded-md px-6 text-sm font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
        >
          Skip for now
        </button>
      </div>
    </div>
  );
}
