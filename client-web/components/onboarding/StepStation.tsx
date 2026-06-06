import React from "react";
import { ChevronLeft, Building2 } from "lucide-react";

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
  next: () => void;
  back: () => void;
}

const ROLES = [
  { id: "observer", label: "Observer", desc: "Read-only monitoring" },
  { id: "responder", label: "Responder", desc: "Incident response" },
  { id: "manager", label: "Manager", desc: "Team & incident lead" },
];

export function StepStation({ data, updateData, next, back }: StepProps) {
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!data.stationName) return;
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full max-w-sm space-y-6">
      <div className="mb-8 text-center">
        <div className="bg-gold/10 mb-4 inline-flex h-20 w-20 items-center justify-center rounded-full text-gold">
          <Building2 className="h-10 w-10" />
        </div>
        <h2 className="text-xl font-bold tracking-tight text-text">Set up your organization</h2>
      </div>

      <div className="space-y-4">
        <div>
          <div className="relative">
            <Building2 className="pointer-events-none absolute left-3 top-1/2 h-5 w-5 -translate-y-1/2 text-muted" />
            <input
              type="text"
              required
              placeholder="Organization"
              value={data.stationName || ""}
              onChange={(e) => updateData({ stationName: e.target.value })}
              className="placeholder:text-muted/40 w-full rounded-md border border-transparent bg-white/5 py-3 pl-11 pr-4 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
            />
          </div>
        </div>

        <div>
          <select
            value={data.timezone || "Europe/Paris"}
            onChange={(e) => updateData({ timezone: e.target.value })}
            className="w-full cursor-pointer rounded-md border border-transparent bg-white/5 px-4 py-3 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
          >
            <option value="Europe/Paris">Europe/Paris (CET)</option>
            <option value="Europe/London">Europe/London (GMT)</option>
            <option value="America/New_York">America/New_York (EST)</option>
            <option value="Asia/Tokyo">Asia/Tokyo (JST)</option>
          </select>
        </div>

        <div>
          <label className="mb-2 block font-sans text-xs font-bold uppercase tracking-wider text-muted">
            Operator Role
          </label>
          <div className="grid grid-cols-3 gap-3">
            {ROLES.map((role) => {
              const isActive = (data.clearance || "observer") === role.id;
              return (
                <button
                  key={role.id}
                  type="button"
                  onClick={() => updateData({ clearance: role.id })}
                  className={`rounded border p-3 text-left font-sans transition-all ${
                    isActive
                      ? "bg-gold/5 border-gold text-gold"
                      : "border-transparent bg-white/5 text-muted hover:bg-white/10"
                  }`}
                >
                  <div className="text-sm font-bold">{role.label}</div>
                  <div className="mt-1 text-xs opacity-80">{role.desc}</div>
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
          className="flex shrink-0 items-center justify-center text-muted transition-colors hover:text-text"
        >
          <ChevronLeft className="h-6 w-6" />
        </button>
        <button
          type="submit"
          className="hover:bg-gold-hover rounded-md bg-gold px-6 py-2 font-sans text-sm font-bold uppercase tracking-wider text-bg transition-colors"
        >
          Next
        </button>
      </div>
    </form>
  );
}
