"use client";

import React, { useMemo, useState } from "react";
import { MessageSquare, Search, Users } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import {
  type BanKindInput,
  type Team,
  type TeamMember,
  useBanMember,
  useKickMember,
  useSetMemberRole,
  useTeamMembers,
  useTransferManager,
} from "@/lib/queries/teams";
import { useTeamOnline } from "@/lib/ws";
import { useAuthStore } from "@/store/auth";
import { deriveCapabilities } from "@/lib/capabilities";
import { Alert } from "@/components/ui/Alert";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { IconButton } from "@/components/ui/Button";
import { PageToolbar } from "@/components/layout/PageToolbar";
import { DirectMessageDialog } from "./DirectMessageDialog";
import { MemberRowActions } from "./MemberRowActions";
import { RoleChip } from "./RoleChip";

function initials(email: string): string {
  const local = email.split("@")[0] ?? email;
  const parts = local.split(/[._-]+/).filter(Boolean);
  const letters = parts.length >= 2 ? parts[0][0] + parts[1][0] : local.slice(0, 2);
  return letters.toUpperCase();
}

type Dialog = "makeManager" | "kick" | "ban" | null;
type BanDuration = "permanent" | "1h" | "24h" | "7d";

function durationToBan(duration: BanDuration): BanKindInput {
  if (duration === "permanent") return { kind: "permanent" };
  const hours = duration === "1h" ? 1 : duration === "24h" ? 24 : 24 * 7;
  return {
    kind: "temporary",
    expires_at: new Date(Date.now() + hours * 3_600_000).toISOString(),
  };
}

/** Searchable operational roster. Team-level ownership and danger actions live
 * in Settings; row-level member actions stay beside the member they affect. */
export function TeamRoster({ team }: { team: Team }) {
  const t = useTranslations("Teams");
  const tDm = useTranslations("DirectMessages");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const currentUserId = useAuthStore((state) => state.user?.id);
  const { data: members, isLoading, error } = useTeamMembers(team.team_id);
  const onlineSet = new Set(useTeamOnline(team.team_id));
  const capabilities = deriveCapabilities(team.role);

  const setRole = useSetMemberRole(team.team_id);
  const transfer = useTransferManager(team.team_id);
  const kick = useKickMember(team.team_id);
  const ban = useBanMember(team.team_id);

  const [query, setQuery] = useState("");
  const [dialog, setDialog] = useState<Dialog>(null);
  const [target, setTarget] = useState<TeamMember | null>(null);
  const [messageTarget, setMessageTarget] = useState<TeamMember | null>(null);
  const [banDuration, setBanDuration] = useState<BanDuration>("permanent");

  const visibleMembers = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    if (!normalized) return members ?? [];
    return (members ?? []).filter(
      (member) =>
        member.email.toLocaleLowerCase().includes(normalized) ||
        member.role.toLocaleLowerCase().includes(normalized),
    );
  }, [members, query]);
  const onlineCount = (members ?? []).filter((member) => onlineSet.has(member.user_id)).length;
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const close = () => setDialog(null);

  const openDialog = (next: Exclude<Dialog, null>, member: TeamMember) => {
    setTarget(member);
    if (next === "ban") setBanDuration("permanent");
    setDialog(next);
  };

  return (
    <div className="space-y-4">
      <PageToolbar>
        <label className="relative min-w-0 flex-1">
          <span className="sr-only">{t("searchMembers")}</span>
          <Search
            className="text-muted pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2"
            aria-hidden="true"
          />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("searchMembers")}
            className="ow-input h-10 w-full rounded-md pr-3 pl-10 text-sm"
          />
        </label>
        <div className="text-muted flex items-center gap-3 px-1 text-sm">
          <span>{t("memberCount", { count: members?.length ?? team.member_count })}</span>
          <span className="inline-flex items-center gap-1.5">
            <span className="bg-st-res h-1.5 w-1.5 rounded-full" />
            {t("onlineCount", { count: onlineCount })}
          </span>
        </div>
      </PageToolbar>

      <div className="surface overflow-hidden rounded-md">
        <div className="border-border flex items-center gap-3 border-b px-5 py-4">
          <Users className="text-muted h-5 w-5" aria-hidden="true" />
          <h2 className="text-text font-semibold">{t("members")}</h2>
        </div>

        {setRole.error ? (
          <Alert tone="danger" className="m-4">
            {errorText(setRole.error.message)}
          </Alert>
        ) : null}

        {isLoading ? (
          <div className="divide-border divide-y">
            {[0, 1, 2].map((index) => (
              <div key={index} className="flex items-center gap-3 px-5 py-4">
                <div className="bg-muted/20 h-9 w-9 animate-pulse rounded-full" />
                <div className="bg-muted/20 h-4 w-48 animate-pulse rounded" />
                <div className="bg-muted/20 ml-auto h-5 w-20 animate-pulse rounded-full" />
              </div>
            ))}
          </div>
        ) : error ? (
          <Alert tone="danger" className="m-4">
            {t("membersFailed")}
          </Alert>
        ) : visibleMembers.length === 0 ? (
          <div className="text-muted px-6 py-10 text-center text-sm">
            {query ? t("noMatchingMembers") : t("noMembers")}
          </div>
        ) : (
          <ul className="divide-border divide-y">
            {visibleMembers.map((member) => (
              <li
                key={member.user_id}
                className="grid grid-cols-[auto_minmax(0,1fr)_auto_auto] items-center gap-3 px-5 py-4 transition-colors hover:bg-white/[0.03]"
              >
                <span className="relative shrink-0">
                  <span className="surface-subtle text-muted border-border flex h-9 w-9 items-center justify-center rounded-full border text-xs font-semibold">
                    {initials(member.email)}
                  </span>
                  <span
                    title={onlineSet.has(member.user_id) ? t("online") : t("offline")}
                    className={`border-bg absolute -right-0.5 -bottom-0.5 h-2.5 w-2.5 rounded-full border-2 ${
                      onlineSet.has(member.user_id) ? "bg-st-res" : "bg-muted/40"
                    }`}
                  />
                </span>
                <div className="min-w-0">
                  <div className="text-text truncate font-medium">{member.email}</div>
                  <div className="text-muted mt-0.5 text-xs">
                    {t("joinedOn", {
                      date: new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                        new Date(member.joined_at),
                      ),
                    })}
                  </div>
                </div>
                <RoleChip role={member.role} />
                <div className="flex items-center gap-1">
                  {member.user_id !== currentUserId && capabilities.canSendPrivateMessage ? (
                    <IconButton
                      onClick={() => setMessageTarget(member)}
                      label={tDm("message")}
                      size="sm"
                      variant="ghost"
                    >
                      <MessageSquare className="h-4 w-4" />
                    </IconButton>
                  ) : null}
                  {capabilities.canManageMembers ? (
                    <MemberRowActions
                      member={member}
                      pending={
                        setRole.isPending || transfer.isPending || kick.isPending || ban.isPending
                      }
                      onSetRole={(role) => setRole.mutate({ userId: member.user_id, role })}
                      onMakeManager={() => openDialog("makeManager", member)}
                      onKick={() => openDialog("kick", member)}
                      onBan={() => openDialog("ban", member)}
                    />
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      <ConfirmDialog
        open={dialog === "makeManager"}
        title={t("makeManager")}
        description={t("transferConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("makeManager")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        pending={transfer.isPending}
        error={transfer.error ? errorText(transfer.error.message) : null}
        onConfirm={() => target && transfer.mutate(target.user_id, { onSuccess: close })}
        onClose={close}
      />
      <ConfirmDialog
        open={dialog === "kick"}
        title={t("kick")}
        description={t("kickConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("kick")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        danger
        pending={kick.isPending}
        error={kick.error ? errorText(kick.error.message) : null}
        onConfirm={() => target && kick.mutate(target.user_id, { onSuccess: close })}
        onClose={close}
      />
      <ConfirmDialog
        open={dialog === "ban"}
        title={t("banMember")}
        description={t("banConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("ban")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        danger
        pending={ban.isPending}
        error={ban.error ? errorText(ban.error.message) : null}
        onConfirm={() =>
          target &&
          ban.mutate(
            { userId: target.user_id, ban: durationToBan(banDuration) },
            { onSuccess: close },
          )
        }
        onClose={close}
      >
        <label className="space-y-2">
          <span className="text-muted text-sm">{t("banDuration")}</span>
          <select
            value={banDuration}
            onChange={(event) => setBanDuration(event.target.value as BanDuration)}
            className="ow-input h-10 w-full rounded-md px-3 text-sm"
          >
            <option value="permanent">{t("banPermanent")}</option>
            <option value="1h">{t("ban1h")}</option>
            <option value="24h">{t("ban24h")}</option>
            <option value="7d">{t("ban7d")}</option>
          </select>
        </label>
      </ConfirmDialog>

      {messageTarget ? (
        <DirectMessageDialog
          peer={{ user_id: messageTarget.user_id, email: messageTarget.email }}
          onClose={() => setMessageTarget(null)}
        />
      ) : null}
    </div>
  );
}
