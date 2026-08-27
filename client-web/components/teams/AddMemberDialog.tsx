"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { useAddTeamMember } from "@/lib/queries/teams";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { FormField } from "@/components/ui/FormField";
import { Dialog, DialogClose } from "@/components/ui/Dialog";

const USER_ID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export function AddMemberDialog({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");
  const tSidebar = useTranslations("Sidebar");
  const [open, setOpen] = useState(false);
  const [userId, setUserId] = useState("");
  const addMember = useAddTeamMember(teamId);
  const normalizedId = userId.trim();
  const valid = USER_ID_PATTERN.test(normalizedId);

  const close = () => {
    setOpen(false);
    setUserId("");
    addMember.reset();
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => (next ? setOpen(true) : close())}
      trigger={<Button variant="primary">{t("addMember")}</Button>}
      title={t("addMember")}
      description={t("addMemberDescription")}
      closeLabel={tSidebar("close")}
      size="sm"
      footer={
        <>
          <DialogClose>
            <Button size="md" onClick={close}>
              {t("cancel")}
            </Button>
          </DialogClose>
          <Button
            type="submit"
            form="add-member-form"
            variant="primary"
            size="md"
            loading={addMember.isPending}
            disabled={!valid}
          >
            {t("addMember")}
          </Button>
        </>
      }
    >
      <form
        id="add-member-form"
        className="space-y-3"
        onSubmit={(event) => {
          event.preventDefault();
          if (!valid) return;
          addMember.mutate(normalizedId, { onSuccess: close });
        }}
      >
        <FormField label={t("userId")}>
          <input
            autoFocus
            value={userId}
            onChange={(event) => {
              setUserId(event.target.value);
              addMember.reset();
            }}
            placeholder="00000000-0000-4000-8000-000000000000"
            className="ow-input flex h-9 w-full rounded-md px-3 font-mono text-sm"
            aria-invalid={normalizedId.length > 0 && !valid}
          />
        </FormField>
        {addMember.error ? (
          <Alert tone="danger">
            {tErr.has(addMember.error.message)
              ? tErr(addMember.error.message)
              : t("addMemberFailed")}
          </Alert>
        ) : null}
      </form>
    </Dialog>
  );
}
