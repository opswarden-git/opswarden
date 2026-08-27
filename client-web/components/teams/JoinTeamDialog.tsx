import React, { useRef, useState } from "react";
import { useJoinTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";
import { FormField } from "@/components/ui/FormField";

export function JoinTeamDialog() {
  const [open, setOpen] = useState(false);
  const [code, setCode] = useState("");
  const codeRef = useRef<HTMLInputElement>(null);
  const joinTeam = useJoinTeam();
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");

  const handleOpenChange = (nextOpen: boolean) => {
    if (nextOpen) {
      setCode("");
      joinTeam.reset();
    }
    setOpen(nextOpen);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!code.trim()) return;
    joinTeam.mutate(code.trim(), {
      onSuccess: () => {
        setOpen(false);
        setCode("");
      },
    });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={handleOpenChange}
      trigger={<Button>{t("joinTeam")}</Button>}
      title={t("joinTitle")}
      closeLabel={t("close")}
      initialFocus={codeRef}
      size="sm"
      footer={
        <>
          <DialogClose>
            <Button size="md">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="join-team-form"
            disabled={joinTeam.isPending || !code.trim()}
            loading={joinTeam.isPending}
            size="md"
            variant="primary"
          >
            {joinTeam.isPending ? t("joining") : t("join")}
          </Button>
        </>
      }
    >
      <form id="join-team-form" onSubmit={handleSubmit} className="space-y-3">
        <FormField label={t("colInvitationCode")}>
          <input
            ref={codeRef}
            type="text"
            value={code}
            onChange={(event) => setCode(event.target.value.toUpperCase())}
            className="ow-input flex h-9 w-full rounded-md px-3 font-mono text-sm tracking-widest uppercase transition-colors"
            placeholder={t("invitationCodePlaceholder")}
          />
        </FormField>
        {joinTeam.isError ? (
          <p className="text-sev-critical text-xs" role="alert">
            {tErr.has(joinTeam.error.message) ? tErr(joinTeam.error.message) : t("actionFailed")}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
