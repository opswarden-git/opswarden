import React, { useRef, useState } from "react";
import { useCreateTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";
import { FormField } from "@/components/ui/FormField";

export function CreateTeamDialog() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const createTeam = useCreateTeam();
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");

  const handleOpenChange = (nextOpen: boolean) => {
    if (nextOpen) {
      setName("");
      createTeam.reset();
    }
    setOpen(nextOpen);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    createTeam.mutate(name.trim(), {
      onSuccess: () => {
        setOpen(false);
        setName("");
      },
    });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={handleOpenChange}
      trigger={<Button variant="primary">{t("createTeam")}</Button>}
      title={t("createTitle")}
      closeLabel={t("close")}
      initialFocus={nameRef}
      size="sm"
      footer={
        <>
          <DialogClose>
            <Button size="md">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="create-team-form"
            disabled={createTeam.isPending || !name.trim()}
            loading={createTeam.isPending}
            size="md"
            variant="primary"
          >
            {createTeam.isPending ? t("creating") : t("create")}
          </Button>
        </>
      }
    >
      <form id="create-team-form" onSubmit={handleSubmit} className="space-y-3">
        <FormField label={t("name")}>
          <input
            ref={nameRef}
            type="text"
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="ow-input flex h-9 w-full rounded-md px-3 text-sm transition-colors"
            placeholder={t("namePlaceholder")}
          />
        </FormField>
        {createTeam.isError ? (
          <p className="text-sev-critical text-xs" role="alert">
            {tErr.has(createTeam.error.message)
              ? tErr(createTeam.error.message)
              : t("actionFailed")}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
