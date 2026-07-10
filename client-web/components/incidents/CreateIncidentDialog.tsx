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
        className="ow-danger flex h-9 items-center gap-2 rounded-md px-3.5 text-sm font-medium transition-colors disabled:opacity-50"
      >
        <AlertCircle className="h-4 w-4" />
        {t("declareIncident")}
      </button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface relative w-full max-w-lg rounded-md p-6 shadow-2xl">
            <button
              onClick={() => setOpen(false)}
              className="text-muted hover:text-text absolute top-4 right-4"
              aria-label="Close dialog"
            >
              <X className="h-5 w-5" />
            </button>
            <div className="mb-6 flex items-center gap-3">
              <div className="bg-sev-critical/15 text-sev-critical flex h-10 w-10 items-center justify-center rounded-full">
                <AlertCircle className="h-5 w-5" />
              </div>
              <div>
                <h2 className="text-text font-sans text-lg font-bold">{t("declareTitle")}</h2>
                <p className="text-muted text-xs">{t("declareWarning")}</p>
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
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
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
                  className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
                >
                  <option value="low" className="bg-bg text-text">
                    {t("sevLowDesc")}
                  </option>
                  <option value="medium" className="bg-bg text-text">
                    {t("sevMediumDesc")}
                  </option>
                  <option value="high" className="bg-bg text-text">
                    {t("sevHighDesc")}
                  </option>
                  <option value="critical" className="bg-bg text-text">
                    {t("sevCriticalDesc")}
                  </option>
                </select>
              </div>

              <div className="flex justify-end gap-2 pt-4">
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="ow-secondary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors"
                >
                  {t("cancel")}
                </button>
                <button
                  type="submit"
                  disabled={createIncident.isPending || !title.trim()}
                  className="ow-danger inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {createIncident.isPending ? t("declaring") : t("declare")}
                </button>
              </div>
              {createIncident.isError && (
                <p className="text-sev-critical mt-2 text-sm">{createIncident.error.message}</p>
              )}
            </form>
          </div>
        </div>
      )}
    </>
  );
}
