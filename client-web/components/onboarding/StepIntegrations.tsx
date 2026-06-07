import React from "react";
import { Workflow, ChevronLeft } from "lucide-react";
import Image from "next/image";

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
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

export function StepIntegrations({ data, updateData, next, back }: StepProps) {
  const selected = data.integrations || [];

  const toggle = (id: string) => {
    if (selected.includes(id)) {
      updateData({ integrations: selected.filter((x: string) => x !== id) });
    } else {
      updateData({ integrations: [...selected, id] });
    }
  };

  return (
    <div className="mx-auto w-full max-w-sm space-y-6">
      <div className="mb-8 text-center">
        <div className="bg-gold/10 text-gold mb-4 inline-flex h-20 w-20 items-center justify-center rounded-full">
          <Workflow className="h-10 w-10" />
        </div>
        <h2 className="text-text text-xl font-bold tracking-tight">Connect your integrations</h2>
      </div>

      <div className="space-y-4">
        {AVAILABLE_INTEGRATIONS.map((integ) => {
          const isActive = selected.includes(integ.id);
          return (
            <div
              key={integ.id}
              className="flex items-center justify-between rounded-lg p-4 transition-colors hover:bg-white/5"
            >
              <div className="flex min-w-0 items-center gap-4 pr-4">
                <div className="flex shrink-0 items-center justify-center">
                  <Image
                    src={integ.icon}
                    alt={integ.name}
                    width={24}
                    height={24}
                    className="h-6 w-6 object-contain"
                  />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-text truncate font-sans text-sm font-bold">
                      {integ.name}
                    </span>
                  </div>
                  <p className="text-muted mt-0.5 truncate text-xs">{integ.desc}</p>
                </div>
              </div>

              <button
                type="button"
                onClick={() => toggle(integ.id)}
                className={`shrink-0 rounded px-4 py-2 font-sans text-xs font-bold uppercase transition-all ${
                  isActive
                    ? "text-muted hover:text-text bg-white/5 hover:bg-white/10"
                    : "hover:bg-gold-hover bg-gold text-bg"
                }`}
              >
                {isActive ? "Connected" : "Connect"}
              </button>
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between pt-4">
        <button
          type="button"
          onClick={back}
          className="text-muted hover:text-text flex shrink-0 items-center justify-center transition-colors"
        >
          <ChevronLeft className="h-6 w-6" />
        </button>
        <button
          type="button"
          onClick={next}
          className="hover:bg-gold-hover bg-gold text-bg rounded-md px-6 py-2 font-sans text-sm font-bold tracking-wider uppercase transition-colors"
        >
          Next
        </button>
      </div>
    </div>
  );
}
