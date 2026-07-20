import React, { useRef, useState } from "react";
import { AlertCircle } from "lucide-react";
import { useCreateIncident, IncidentSeverity } from "@/lib/queries/incidents";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";

export function CreateIncidentDialog({ teamId }: { teamId: string }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [severity, setSeverity] = useState<IncidentSeverity>("medium");
  const titleRef = useRef<HTMLInputElement>(null);
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
    <Dialog
      open={open}
      onOpenChange={setOpen}
      trigger={
        <Button disabled={!teamId} variant="danger">
          <AlertCircle className="h-4 w-4" aria-hidden="true" />
          {t("declareIncident")}
        </Button>
      }
      title={t("declareTitle")}
      description={t("declareWarning")}
      closeLabel={t("close")}
      initialFocus={titleRef}
      icon={
        <div className="bg-sev-critical/15 text-sev-critical flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <AlertCircle className="h-5 w-5" aria-hidden="true" />
        </div>
      }
      footer={
        <>
          <DialogClose>
            <Button size="lg">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="create-incident-form"
            disabled={createIncident.isPending || !title.trim()}
            loading={createIncident.isPending}
            size="lg"
            variant="danger"
          >
            {createIncident.isPending ? t("declaring") : t("declare")}
          </Button>
        </>
      }
    >
      <form id="create-incident-form" onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="inc-title" className="text-muted mb-1 block font-sans text-xs">
            {t("colTitle")}
          </label>
          <input
            ref={titleRef}
            id="inc-title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
            placeholder={t("titlePlaceholder")}
          />
        </div>

        <div>
          <label htmlFor="inc-description" className="text-muted mb-1 block font-sans text-xs">
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

        {createIncident.isError ? (
          <p className="text-sev-critical text-sm" role="alert">
            {createIncident.error.message}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
