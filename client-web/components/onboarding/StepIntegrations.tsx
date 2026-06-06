import React from 'react';
import { Workflow, ChevronLeft } from 'lucide-react';
import Image from 'next/image';

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
  next: () => void;
  back: () => void;
}

const AVAILABLE_INTEGRATIONS = [
  { id: 'github', name: 'GitHub', desc: 'Link actions & deployment flows', icon: '/assets/github-patched.webp' },
  { id: 'gitlab', name: 'GitLab', desc: 'Sync pipelines and issue boards', icon: '/assets/gitlab.webp' },
  { id: 'k8s', name: 'Kubernetes', desc: 'Deploy container metrics monitor', icon: '/assets/kubernetes.webp' },
  { id: 'sentry', name: 'Sentry', desc: 'Track application exceptions & crashes', icon: '/assets/sentry.webp' },
  { id: 'datadog', name: 'Datadog', desc: 'Sync system APM telemetry data', icon: '/assets/datadog.webp' },
  { id: 'pagerduty', name: 'PagerDuty', desc: 'Sync incident & rotation escalations', icon: '/assets/pagerduty.webp' },
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
    <div className="space-y-6 max-w-sm mx-auto w-full">
      <div className="text-center mb-8">
        <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-gold/10 text-gold mb-4">
          <Workflow className="h-10 w-10" />
        </div>
        <h2 className="text-xl font-bold tracking-tight text-text">Connect your integrations</h2>
      </div>

      <div className="space-y-4">
        {AVAILABLE_INTEGRATIONS.map((integ) => {
          const isActive = selected.includes(integ.id);
          return (
            <div
              key={integ.id}
              className="p-4 rounded-lg flex items-center justify-between transition-colors hover:bg-white/5"
            >
              <div className="flex items-center gap-4 min-w-0 pr-4">
                <div className="shrink-0 flex items-center justify-center">
                  <Image src={integ.icon} alt={integ.name} width={24} height={24} className="h-6 w-6 object-contain" />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-sans text-sm font-bold text-text truncate">
                      {integ.name}
                    </span>
                  </div>
                  <p className="text-xs text-muted mt-0.5 truncate">{integ.desc}</p>
                </div>
              </div>
              
              <button
                type="button"
                onClick={() => toggle(integ.id)}
                className={`px-4 py-2 text-xs font-sans font-bold uppercase rounded transition-all shrink-0 ${
                  isActive
                    ? 'bg-white/5 text-muted hover:text-text hover:bg-white/10'
                    : 'bg-gold text-bg hover:bg-gold-hover'
                }`}
              >
                {isActive ? 'Connected' : 'Connect'}
              </button>
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between pt-4">
        <button
          type="button"
          onClick={back}
          className="flex items-center justify-center text-muted hover:text-text transition-colors shrink-0"
        >
          <ChevronLeft className="h-6 w-6" />
        </button>
        <button
          type="button"
          onClick={next}
          className="px-6 py-2 bg-gold text-bg font-bold rounded-md hover:bg-gold-hover transition-colors font-sans text-sm uppercase tracking-wider"
        >
          Next
        </button>
      </div>
    </div>
  );
}
