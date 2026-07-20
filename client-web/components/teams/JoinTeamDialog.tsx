import React, { useRef, useState } from "react";
import { Users } from "lucide-react";
import { useJoinTeam } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";

export function JoinTeamDialog() {
  const [open, setOpen] = useState(false);
  const [code, setCode] = useState("");
  const codeRef = useRef<HTMLInputElement>(null);
  const joinTeam = useJoinTeam();
  const t = useTranslations("Teams");

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
      trigger={
        <Button>
          <Users className="h-4 w-4" aria-hidden="true" />
          {t("joinTeam")}
        </Button>
      }
      title={t("joinTitle")}
      description={t("joinDesc")}
      closeLabel={t("close")}
      initialFocus={codeRef}
      size="sm"
      icon={
        <div className="bg-panel-2 text-text flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <Users className="h-5 w-5" aria-hidden="true" />
        </div>
      }
      footer={
        <>
          <DialogClose>
            <Button size="lg">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="join-team-form"
            disabled={joinTeam.isPending || !code.trim()}
            loading={joinTeam.isPending}
            size="lg"
            variant="primary"
          >
            {joinTeam.isPending ? t("joining") : t("join")}
          </Button>
        </>
      }
    >
      <form id="join-team-form" onSubmit={handleSubmit}>
        <label htmlFor="join-code" className="text-muted mb-1 block font-sans text-xs">
          {t("colInvitationCode")}
        </label>
        <input
          ref={codeRef}
          id="join-code"
          type="text"
          value={code}
          onChange={(event) => setCode(event.target.value.toUpperCase())}
          className="ow-input flex h-10 w-full rounded-md px-3 py-2 font-mono text-sm tracking-widest uppercase transition-colors"
          placeholder="OPS-XXXXXX"
        />
        {joinTeam.isError ? (
          <p className="text-sev-critical mt-3 text-sm" role="alert">
            {joinTeam.error.message}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
