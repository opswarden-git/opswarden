"use client";

import { useTranslations } from "next-intl";
import { ConversationMessage } from "@/components/messages/ConversationMessage";
import { memberDisplayName } from "@/components/teams/MemberAvatar";
import {
  downloadPrivateMessageAttachment,
  type PrivateMessage,
  useEditPrivateMessage,
} from "@/lib/queries/privateMessages";

export function DirectMessageItem({
  continuesAbove,
  message,
  mine,
  peerEmail,
  peerId,
}: {
  continuesAbove: boolean;
  message: PrivateMessage;
  mine: boolean;
  peerEmail: string;
  peerId: string;
}) {
  const t = useTranslations("DirectMessages");
  const tCommon = useTranslations("Common");
  const edit = useEditPrivateMessage(peerId);

  return (
    <ConversationMessage
      attachments={message.attachments.map((attachment) => ({
        id: attachment.id,
        fileName: attachment.file_name,
        sizeBytes: attachment.size_bytes,
      }))}
      authorLabel={memberDisplayName(peerEmail)}
      content={message.content}
      continuesAbove={continuesAbove}
      createdAt={message.created_at}
      editedAt={message.edited_at}
      editError={edit.error ? t("editFailed") : undefined}
      editPending={edit.isPending}
      labels={{
        cancel: t("cancel"),
        downloadFailed: t("downloadFailed"),
        edit: t("edit"),
        edited: t("edited"),
        editMessage: t("editMessage"),
        gifAlt: tCommon("gifAlt"),
        save: t("save"),
      }}
      mine={mine}
      surface="direct"
      onDownload={async (attachmentId) => {
        const attachment = message.attachments.find((item) => item.id === attachmentId);
        if (!attachment) throw new Error("attachment_not_found");
        await downloadPrivateMessageAttachment(attachment);
      }}
      onEdit={(content, onSuccess) =>
        edit.mutate({ messageId: message.id, content }, { onSuccess })
      }
      onResetEdit={edit.reset}
    />
  );
}
