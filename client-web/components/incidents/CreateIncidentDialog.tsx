import React, { useRef, useState } from "react";
import { useCreateIncident, IncidentSeverity } from "@/lib/queries/incidents";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";
import { FormField } from "@/components/ui/FormField";

export function CreateIncidentDialog({ teamId }: { teamId: string }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [severity, setSeverity] = useState<IncidentSeverity>("medium");
  const titleRef = useRef<HTMLInputElement>(null);
  const createIncident = useCreateIncident();
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");

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
          {t("newIncident")}
        </Button>
      }
      title={t("declareTitle")}
      closeLabel={t("cancel")}
      initialFocus={titleRef}
      footer={
        <>
          <DialogClose>
            <Button size="md">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="create-incident-form"
            disabled={createIncident.isPending || !title.trim()}
            loading={createIncident.isPending}
            size="md"
            variant="danger"
          >
            {createIncident.isPending ? t("declaring") : t("declare")}
          </Button>
        </>
      }
    >
      <form id="create-incident-form" onSubmit={handleSubmit} className="space-y-3">
        <FormField label={t("colTitle")}>
          <input
            ref={titleRef}
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="ow-input flex h-9 w-full rounded-md px-3 text-sm transition-colors"
          />
        </FormField>

        <FormField label={t("fieldDescription")}>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={3}
            className="ow-input flex w-full rounded-md px-3 py-2 text-sm transition-colors"
            placeholder={t("descriptionPlaceholder")}
          />
        </FormField>

        <FormField label={t("severity")}>
          <select
            value={severity}
            onChange={(e) => setSeverity(e.target.value as IncidentSeverity)}
            className="ow-input flex h-9 w-full rounded-md px-3 text-sm transition-colors"
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
        </FormField>

        {createIncident.isError ? (
          <p className="text-sev-critical text-xs" role="alert">
            {tErr.has(createIncident.error.message)
              ? tErr(createIncident.error.message)
              : t("actionFailed")}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
