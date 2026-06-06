import React from 'react';
import { ChevronLeft, Building2 } from 'lucide-react';

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
  next: () => void;
  back: () => void;
}

const ROLES = [
  { id: 'observer', label: 'Observer', desc: 'Read-only monitoring' },
  { id: 'responder', label: 'Responder', desc: 'Incident response' },
  { id: 'manager', label: 'Manager', desc: 'Team & incident lead' },
];

export function StepStation({ data, updateData, next, back }: StepProps) {
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!data.stationName) return;
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6 max-w-sm mx-auto w-full">
      <div className="text-center mb-8">
        <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-gold/10 text-gold mb-4">
          <Building2 className="h-10 w-10" />
        </div>
        <h2 className="text-xl font-bold tracking-tight text-text">Set up your organization</h2>
      </div>

      <div className="space-y-4">
        <div>
          <div className="relative">
            <Building2 className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-muted pointer-events-none" />
            <input
              type="text"
              required
              placeholder="Organization"
              value={data.stationName || ''}
              onChange={e => updateData({ stationName: e.target.value })}
              className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md pl-11 pr-4 py-3 text-base text-text focus:outline-none font-sans placeholder:text-muted/40 transition-colors"
            />
          </div>
        </div>

        <div>
          <select
            value={data.timezone || 'Europe/Paris'}
            onChange={e => updateData({ timezone: e.target.value })}
            className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md px-4 py-3 text-base text-text focus:outline-none font-sans transition-colors cursor-pointer"
          >
            <option value="Europe/Paris">Europe/Paris (CET)</option>
            <option value="Europe/London">Europe/London (GMT)</option>
            <option value="America/New_York">America/New_York (EST)</option>
            <option value="Asia/Tokyo">Asia/Tokyo (JST)</option>
          </select>
        </div>

        <div>
          <label className="block text-xs uppercase tracking-wider text-muted font-sans font-bold mb-2">
            Operator Role
          </label>
          <div className="grid grid-cols-3 gap-3">
            {ROLES.map((role) => {
              const isActive = (data.clearance || 'observer') === role.id;
              return (
                <button
                  key={role.id}
                  type="button"
                  onClick={() => updateData({ clearance: role.id })}
                  className={`p-3 rounded border text-left transition-all font-sans ${
                    isActive
                      ? 'border-gold bg-gold/5 text-gold'
                      : 'border-transparent bg-white/5 text-muted hover:bg-white/10'
                  }`}
                >
                  <div className="text-sm font-bold">{role.label}</div>
                  <div className="text-xs opacity-80 mt-1">{role.desc}</div>
                </button>
              );
            })}
          </div>
        </div>
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
          type="submit"
          className="px-6 py-2 bg-gold text-bg font-bold rounded-md hover:bg-gold-hover transition-colors font-sans text-sm uppercase tracking-wider"
        >
          Next
        </button>
      </div>
    </form>
  );
}
