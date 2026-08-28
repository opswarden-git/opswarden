import React from "react";
import { Building2, KeyRound } from "lucide-react";
import { useTranslations } from "next-intl";
import type { OnboardingData, UpdateOnboardingData } from "./types";
import { Button } from "@/components/ui/Button";

interface StepProps {
  data: OnboardingData;
  updateData: UpdateOnboardingData;
  next: () => void;
  back: () => void;
}

export function StepTeam({ data, updateData, next, back }: StepProps) {
  const t = useTranslations("Onboarding");
  const mode = data.mode || "create";

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (mode === "create" && !data.teamName?.trim()) return;
    if (mode === "join" && !data.invitationCode?.trim()) return;
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full space-y-6">
      <div className="flex flex-col gap-4">
        <div className="surface-subtle border-border/60 grid grid-cols-2 gap-1 rounded-md border p-1 text-xs font-medium">
          <button
            type="button"
            onClick={() => updateData({ mode: "create" })}
            className={`flex items-center justify-center gap-1.5 rounded py-1.5 transition-colors ${
              mode === "create"
                ? "bg-panel text-text font-semibold shadow-xs"
                : "text-muted hover:text-text"
            }`}
          >
            <Building2 className="size-3.5" />
            <span>{t("modeCreate")}</span>
          </button>
          <button
            type="button"
            onClick={() => updateData({ mode: "join" })}
            className={`flex items-center justify-center gap-1.5 rounded py-1.5 transition-colors ${
              mode === "join"
                ? "bg-panel text-text font-semibold shadow-xs"
                : "text-muted hover:text-text"
            }`}
          >
            <KeyRound className="size-3.5" />
            <span>{t("modeJoin")}</span>
          </button>
        </div>

        {mode === "create" ? (
          <div className="flex flex-col gap-2">
            <label htmlFor="team-name" className="text-muted text-xs font-medium">
              {t("teamName")}
            </label>
            <div className="relative">
              <Building2 className="text-muted pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
              <input
                key="input-team-name"
                id="team-name"
                type="text"
                required
                placeholder={t("teamNamePlaceholder")}
                value={data.teamName || ""}
                onChange={(e) => updateData({ teamName: e.target.value })}
                className="ow-input flex h-10 w-full rounded-md py-2 pr-3 pl-10 text-sm transition-colors"
              />
            </div>
            <p className="text-muted text-xs">{t("teamNameHelp")}</p>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <label htmlFor="invitation-code" className="text-muted text-xs font-medium">
              {t("invitationCode")}
            </label>
            <div className="relative">
              <KeyRound className="text-muted pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
              <input
                key="input-invitation-code"
                id="invitation-code"
                type="text"
                required
                placeholder={t("invitationCodePlaceholder")}
                value={data.invitationCode || ""}
                onChange={(e) =>
                  updateData({ invitationCode: e.target.value.toUpperCase().trim() })
                }
                className="ow-input flex h-10 w-full rounded-md py-2 pr-3 pl-10 font-mono text-sm uppercase transition-colors"
              />
            </div>
            <p className="text-muted text-xs">{t("invitationCodeHelp")}</p>
          </div>
        )}
      </div>

      <div className="mt-2 flex items-center justify-between pt-4">
        <Button variant="ghost" size="lg" onClick={back}>
          {t("back")}
        </Button>
        <Button type="submit" variant="primary" size="lg">
          {t("next")}
        </Button>
      </div>
    </form>
  );
}
