import React, { useRef, useState } from "react";
import { Plus } from "lucide-react";
import { useCreateTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";

export function CreateTeamDialog() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const createTeam = useCreateTeam();
  const t = useTranslations("Teams");

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
      trigger={
        <Button variant="primary">
          <Plus className="h-4 w-4" aria-hidden="true" />
          {t("createTeam")}
        </Button>
      }
      title={t("createTitle")}
      description={t("createDesc")}
      closeLabel={t("close")}
      initialFocus={nameRef}
      size="sm"
      icon={
        <div className="bg-gold/15 text-gold flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <Plus className="h-5 w-5" aria-hidden="true" />
        </div>
      }
      footer={
        <>
          <DialogClose>
            <Button size="lg">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="create-team-form"
            disabled={createTeam.isPending || !name.trim()}
            loading={createTeam.isPending}
            size="lg"
            variant="primary"
          >
            {createTeam.isPending ? t("creating") : t("create")}
          </Button>
        </>
      }
    >
      <form id="create-team-form" onSubmit={handleSubmit}>
        <label htmlFor="team-name" className="text-muted mb-1 block font-sans text-xs">
          {t("name")}
        </label>
        <input
          ref={nameRef}
          id="team-name"
          type="text"
          value={name}
          onChange={(event) => setName(event.target.value)}
          className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          placeholder={t("namePlaceholder")}
        />
        {createTeam.isError ? (
          <p className="text-sev-critical mt-3 text-sm" role="alert">
            {createTeam.error.message}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
