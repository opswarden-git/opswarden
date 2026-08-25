"use client";

import { useTranslations } from "next-intl";
import { ConversationMessage } from "@/components/messages/ConversationMessage";
import {
  type IncidentActivityItem,
  downloadTimelineAttachment,
  useEditTimelineEntry,
  useToggleTimelineReaction,
} from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";

export function HumanNoteItem({
  availableReactions,
  continuesAbove,
  incidentId,
  item,
}: {
  availableReactions: string[];
  continuesAbove: boolean;
  incidentId: string;
  item: Extract<IncidentActivityItem, { type: "human_note" }>;
}) {
  const t = useTranslations("Incidents");
  const tCommon = useTranslations("Common");
  const tErr = useTranslations("errors");
  const currentUserId = useAuthStore((state) => state.user?.id);
  const edit = useEditTimelineEntry();
  const toggle = useToggleTimelineReaction();
  const attachments = item.attachments ?? [];

  return (
    <ConversationMessage
      authorLabel={item.author?.email ?? t("deletedUser")}
      availableReactions={availableReactions}
      attachments={attachments.map((attachment) => ({
        id: attachment.id,
        fileName: attachment.file_name,
        sizeBytes: attachment.size_bytes,
      }))}
      content={item.content}
      continuesAbove={continuesAbove}
      createdAt={item.created_at}
      editedAt={item.edited_at}
      editError={
        edit.error
          ? tErr.has(edit.error.message)
            ? tErr(edit.error.message)
            : t("actionFailed")
          : undefined
      }
      editPending={edit.isPending}
      labels={{
        addReaction: t("addReaction"),
        cancel: t("cancel"),
        downloadFailed: t("downloadFailed"),
        edit: t("edit"),
        edited: t("edited"),
        editMessage: t("editNote"),
        gifAlt: tCommon("gifAlt"),
        save: t("save"),
      }}
      mine={item.author?.user_id === currentUserId}
      reactions={item.reactions ?? []}
      reactionPending={toggle.isPending}
      surface="incident"
      onEdit={(content, onSuccess) =>
        edit.mutate({ incidentId, entryId: item.entry_id, content }, { onSuccess })
      }
      onResetEdit={edit.reset}
      onToggleReaction={(emoji) => toggle.mutate({ incidentId, entryId: item.entry_id, emoji })}
      onDownload={async (attachmentId) => {
        const attachment = attachments.find((candidate) => candidate.id === attachmentId);
        if (!attachment) throw new Error("attachment_not_found");
        await downloadTimelineAttachment(attachment);
      }}
    />
  );
}
