import React, { useState } from "react";
import { AlertCircle, X } from "lucide-react";
import { useCreateIncident, IncidentSeverity } from "@/lib/queries/incidents";
import { useTranslations } from "next-intl";

export function CreateIncidentDialog({ teamId }: { teamId: string }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [severity, setSeverity] = useState<IncidentSeverity>("medium");
  const createIncident = useCreateIncident();
  const t = useTranslations("Incidents");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !teamId) return;
    createIncident.mutate(
      { team_id: teamId, title: title.trim(), severity },
      {
        onSuccess: () => {
          setOpen(false);
          setTitle("");
          setSeverity("medium");
        },
      },
    );
  };

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        disabled={!teamId}
        className="text-white disabled:text-white/50 flex items-center gap-2 rounded-lg bg-red-600 px-4 py-2 text-sm font-bold transition-colors hover:bg-red-700 disabled:bg-red-600/50"
      >
        <AlertCircle className="h-4 w-4" />
        {t("declareIncident")}
      </button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="glass relative w-full max-w-lg rounded-xl border border-red-500/20 p-6 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              className="text-muted hover:text-text absolute top-4 right-4"
              aria-label="Close dialog"
            >
              <X className="h-5 w-5" />
            </button>
            <div className="mb-6 flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-red-500/20 text-red-500">
                <AlertCircle className="h-5 w-5" />
              </div>
              <div>
                <h2 className="text-text font-sans text-lg font-bold">{t("declareTitle")}</h2>
                <p className="text-muted text-xs">
                  {t("declareWarning")}
                </p>
              </div>
            </div>

            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <label htmlFor="inc-title" className="text-muted mb-1 block font-sans text-xs">
                  {t("colTitle")}
                </label>
                <input
                  id="inc-title"
                  type="text"
                  autoFocus
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="w-full rounded-md border border-white/10 bg-black/50 px-3 py-2 text-sm text-white focus:border-red-500/50 focus:outline-none"
                  placeholder={t("titlePlaceholder")}
                />
              </div>

              <div>
                <label htmlFor="inc-sev" className="text-muted mb-1 block font-sans text-xs">
                  {t("severity")}
                </label>
                <select
                  id="inc-sev"
                  value={severity}
                  onChange={(e) => setSeverity(e.target.value as IncidentSeverity)}
                  className="w-full rounded-md border border-white/10 bg-black/50 px-3 py-2 text-sm text-white focus:border-red-500/50 focus:outline-none"
                >
                  <option value="low">{t("sevLowDesc")}</option>
                  <option value="medium">{t("sevMediumDesc")}</option>
                  <option value="high">{t("sevHighDesc")}</option>
                  <option value="critical">{t("sevCriticalDesc")}</option>
                </select>
              </div>

              <div className="flex justify-end gap-2 pt-4">
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="text-muted hover:text-text px-4 py-2 text-sm font-medium transition-colors"
                >
                  {t("cancel")}
                </button>
                <button
                  type="submit"
                  disabled={createIncident.isPending || !title.trim()}
                  className="text-white rounded-lg bg-red-600 px-4 py-2 text-sm font-bold transition-colors hover:bg-red-700 disabled:bg-red-600/50 disabled:opacity-50"
                >
                  {createIncident.isPending ? t("declaring") : t("declare")}
                </button>
              </div>
              {createIncident.isError && (
                <p className="mt-2 text-sm text-red-500">{createIncident.error.message}</p>
              )}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
