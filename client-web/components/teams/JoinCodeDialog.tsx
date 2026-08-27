"use client";

import { useState } from "react";
import { useTranslations } from "next-intl";
import { useInvitationCode } from "@/lib/queries/teams";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { FormField } from "@/components/ui/FormField";
import { Dialog } from "@/components/ui/Dialog";
import { Skeleton } from "@/components/ui/Skeleton";

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
    >
      {invitation.isLoading ? (
        <div className="flex items-center gap-2" aria-busy="true" aria-label={t("loading")}>
          <Skeleton className="h-10 min-w-0 flex-1 rounded-md" />
          <Skeleton className="h-10 w-10 rounded-md" />
        </div>
      ) : invitation.error || !invitation.data ? (
        <Alert tone="danger">{t("invitationFailed")}</Alert>
      ) : (
        <FormField label={<span className="sr-only">{t("invitationCodeLabel")}</span>}>
          <div className="flex items-center gap-2">
            <code className="ow-input text-text flex h-10 min-w-0 flex-1 items-center rounded-md px-3 font-mono text-sm">
              {invitation.data.invitation_code}
            </code>
            <CopyButton
              className="h-10 w-10"
              value={invitation.data.invitation_code}
              label={t("copyInvitationCode")}
              copiedLabel={t("invitationCodeCopied")}
            />
          </div>
        </FormField>
      )}
    </Dialog>
  );
}
