"use client";

import React from "react";
import Image from "next/image";
import { Workflow } from "lucide-react";
import { GithubIntegration } from "@/components/settings/GithubIntegration";
import { useTranslations } from "next-intl";

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

/** Third-party connectors: the live GitHub integration + coming-soon placeholders. */
export function IntegrationsPanel() {
  const t = useTranslations("Settings");

  return (
    <div className="surface rounded-md p-6">
      <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
        <Workflow className="text-muted h-5 w-5" />
        {t("connectors")}
      </h2>

      <div className="mt-4 space-y-3">
        <GithubIntegration />

        {AVAILABLE_INTEGRATIONS.filter((integ) => integ.id !== "github").map((integ) => (
          <div
            key={integ.id}
            className="surface-subtle border-border flex items-center justify-between gap-4 rounded-md border p-4 transition-colors hover:bg-white/[0.04]"
          >
            <div className="flex min-w-0 items-center gap-3">
              <Image
                src={integ.icon}
                alt={integ.name}
                width={24}
                height={24}
                className="h-7 w-7 shrink-0 object-contain"
              />
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-text truncate font-medium">{integ.name}</span>
                  <span className="border-border text-muted shrink-0 rounded border px-1.5 py-0.5 text-[10px] font-medium">
                    Connector
                  </span>
                </div>
                <p className="text-muted/70 truncate text-xs">{integ.desc}</p>
              </div>
            </div>

            <span className="surface border-border text-muted inline-flex h-9 shrink-0 items-center rounded-md border px-3 text-xs font-medium select-none">
              {t("comingSoon")}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
