import React, { useState } from "react";
import { AlertCircle, X } from "lucide-react";
import { useCreateIncident, IncidentSeverity } from "@/lib/queries/incidents";
import { useTranslations } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";

export function CreateIncidentDialog({ teamId }: { teamId: string }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [severity, setSeverity] = useState<IncidentSeverity>("medium");
  const createIncident = useCreateIncident();
  const t = useTranslations("Incidents");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !teamId) return;
    createIncident.mutate(
      { team_id: teamId, title: title.trim(), description: description.trim(), severity },
      {
        onSuccess: () => {
          setOpen(false);
          setTitle("");
          setDescription("");
          setSeverity("medium");
        },
      },
    );
  };

  return (
    <>
      <Button onClick={() => setOpen(true)} disabled={!teamId} variant="danger">
        <AlertCircle className="h-4 w-4" />
        {t("declareIncident")}
      </Button>

      {open && (
        <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="surface relative w-full max-w-lg rounded-md p-6 shadow-2xl">
            <IconButton
              onClick={() => setOpen(false)}
              className="absolute top-3 right-3"
              label="Close dialog"
              size="sm"
              variant="ghost"
            >
              <X className="h-5 w-5" />
            </IconButton>
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
                <label
                  htmlFor="inc-description"
                  className="text-muted mb-1 block font-sans text-xs"
                >
                  {t("fieldDescription")}
                </label>
                <textarea
                  id="inc-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={3}
                  className="ow-input flex w-full rounded-md px-3 py-2 text-sm transition-colors"
                  placeholder={t("descriptionPlaceholder")}
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
                <Button size="lg" onClick={() => setOpen(false)}>
                  {t("cancel")}
                </Button>
                <Button
                  type="submit"
                  disabled={createIncident.isPending || !title.trim()}
                  loading={createIncident.isPending}
                  size="lg"
                  variant="danger"
                >
                  {createIncident.isPending ? t("declaring") : t("declare")}
                </Button>
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
