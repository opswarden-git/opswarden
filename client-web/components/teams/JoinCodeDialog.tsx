"use client";

import { KeyRound } from "lucide-react";
import { useState } from "react";
import { useTranslations } from "next-intl";
import { useInvitationCode } from "@/lib/queries/teams";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { Dialog } from "@/components/ui/Dialog";

export function JoinCodeDialog({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const tSidebar = useTranslations("Sidebar");
  const [open, setOpen] = useState(false);
  const invitation = useInvitationCode(teamId, open);

  return (
    <Dialog
      open={open}
      onOpenChange={setOpen}
      trigger={<Button variant="secondary">{t("shareJoinCode")}</Button>}
      title={t("shareJoinCode")}
      description={t("shareJoinCodeDescription")}
      closeLabel={tSidebar("close")}
      size="sm"
      icon={
        <div className="bg-gold/15 text-gold flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <KeyRound className="h-5 w-5" aria-hidden="true" />
        </div>
      }
    >
      {invitation.isLoading ? (
        <div className="bg-muted/20 h-10 w-full animate-pulse rounded-md" />
      ) : invitation.error || !invitation.data ? (
        <Alert tone="danger">{t("invitationFailed")}</Alert>
      ) : (
        <div className="flex items-center gap-2">
          <code className="surface-subtle border-border text-text min-w-0 flex-1 rounded-md border px-3 py-2 font-mono text-sm">
            {invitation.data.invitation_code}
          </code>
          <CopyButton
            value={invitation.data.invitation_code}
            label={t("copyInvitationCode")}
            copiedLabel={t("invitationCodeCopied")}
          />
        </div>
      )}
    </Dialog>
  );
}
