import React from "react";
import { ChevronLeft, Building2 } from "lucide-react";
import { useTranslations } from "next-intl";

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
  next: () => void;
  back: () => void;
}

export function StepStation({ data, updateData, next, back }: StepProps) {
  const t = useTranslations("Onboarding");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!data.stationName) return;
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full space-y-6">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <label htmlFor="station-name" className="text-muted text-xs font-medium">
            {t("organization")}
          </label>
          <div className="relative">
            <Building2 className="text-muted pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
            <input
              id="station-name"
              type="text"
              required
              placeholder={t("organizationPlaceholder")}
              value={data.stationName || ""}
              onChange={(e) => updateData({ stationName: e.target.value })}
              className="ow-input flex h-10 w-full rounded-md py-2 pr-3 pl-10 text-sm transition-colors"
            />
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <label htmlFor="station-timezone" className="text-muted text-xs font-medium">
            {t("timezone")}
          </label>
          <select
            id="station-timezone"
            value={data.timezone || "Europe/Paris"}
            onChange={(e) => updateData({ timezone: e.target.value })}
            className="ow-input flex h-10 w-full cursor-pointer appearance-none rounded-md px-3 py-2 text-sm transition-colors"
          >
            <option value="Europe/Paris">Europe/Paris (CET)</option>
            <option value="Europe/London">Europe/London (GMT)</option>
            <option value="America/New_York">America/New_York (EST)</option>
            <option value="Asia/Tokyo">Asia/Tokyo (JST)</option>
          </select>
        </div>
      </div>

      <div className="mt-2 flex items-center justify-between pt-4">
        <button
          type="button"
          onClick={back}
          className="text-muted hover:text-text focus-visible:ring-gold flex h-10 items-center justify-center rounded-md px-3 text-sm font-medium transition-colors focus-visible:ring-1 focus-visible:outline-none"
        >
          <ChevronLeft className="mr-1 size-4" />
          {t("back")}
        </button>
        <button
          type="submit"
          className="ow-primary focus-visible:ring-gold focus-visible:ring-offset-bg inline-flex h-10 items-center justify-center rounded-md px-6 text-sm font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
        >
          {t("next")}
        </button>
      </div>
    </form>
  );
}
