"use client";

import React, { useMemo, useState } from "react";
import { HelpCircle, Search, ShieldOff } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import {
  type BanKindInput,
  type Team,
  type TeamMember,
  useBanMember,
  useKickMember,
  useSetMemberRole,
  useTeamBans,
  useTeamMembers,
  useTransferManager,
  useUnbanMember,
} from "@/lib/queries/teams";
import { useTeamOnline } from "@/lib/ws";
import { useUnreadPrivateMessages } from "@/lib/queries/privateMessages";
import { useAuthStore } from "@/store/auth";
import { deriveCapabilities } from "@/lib/capabilities";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";
import { Alert } from "@/components/ui/Alert";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { IconButton } from "@/components/ui/Button";
import { TableFilterControl } from "@/components/ui/CollectionControls";
import { Skeleton } from "@/components/ui/Skeleton";
import { MemberAvatar, memberDisplayName } from "./MemberAvatar";
import { MemberRowActions } from "./MemberRowActions";
import { RoleChip } from "./RoleChip";

type Dialog = "makeManager" | "kick" | "ban" | null;
type BanDuration = "permanent" | "1h" | "24h" | "7d";
type RoleFilter = "all" | "manager" | "responder" | "observer";

export function TeamRosterRowsSkeleton({ rows = 3 }: { rows?: number }) {
  return (
    <div className="divide-border divide-y" aria-busy="true" data-testid="team-roster-skeleton">
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} className="flex items-center gap-3 px-4 py-4">
          <Skeleton className="h-9 w-9 shrink-0 rounded-full" />
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <Skeleton className="h-4 w-48 max-w-full" />
            <Skeleton className="h-2.5 w-20" />
          </div>
          <Skeleton className="h-8 w-16 shrink-0" />
        </div>
      ))}
    </div>
  );
}

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
  const { data: unreadData } = useUnreadPrivateMessages(capabilities.canSendPrivateMessage);
  const unreadPeerSet = useMemo(
    () => new Set(unreadData?.unread_peer_ids ?? []),
    [unreadData?.unread_peer_ids],
  );

  const setRole = useSetMemberRole(team.team_id);
  const transfer = useTransferManager(team.team_id);
  const kick = useKickMember(team.team_id);
  const ban = useBanMember(team.team_id);
  const bans = useTeamBans(team.team_id, capabilities.canManageMembers);
  const unban = useUnbanMember(team.team_id);

  const [query, setQuery] = useState("");
  const [roleFilter, setRoleFilter] = useState<RoleFilter>("all");
  const [dialog, setDialog] = useState<Dialog>(null);
  const [target, setTarget] = useState<TeamMember | null>(null);
  const [banDuration, setBanDuration] = useState<BanDuration>("permanent");

  const visibleMembers = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    return (members ?? []).filter(
      (member) =>
        (roleFilter === "all" || member.role === roleFilter) &&
        (!normalized ||
          member.email.toLocaleLowerCase().includes(normalized) ||
          member.role.toLocaleLowerCase().includes(normalized)),
    );
  }, [members, query, roleFilter]);
  const visibleActiveMembers = visibleMembers.filter(
    (member) => member.user_id === currentUserId || onlineSet.has(member.user_id),
  );
  const visibleInactiveMembers = visibleMembers.filter(
    (member) => member.user_id !== currentUserId && !onlineSet.has(member.user_id),
  );
  const visibleBans = useMemo(() => {
    if (!capabilities.canManageMembers) return [];
    const normalized = query.trim().toLocaleLowerCase();
    return (bans.data ?? []).filter(
      (entry) =>
        entry.active && (!normalized || entry.user.email.toLocaleLowerCase().includes(normalized)),
    );
  }, [bans.data, capabilities.canManageMembers, query]);
  const hasBans = (bans.data ?? []).some((entry) => entry.active);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const close = () => setDialog(null);

  const openDialog = (next: Exclude<Dialog, null>, member: TeamMember) => {
    setTarget(member);
    if (next === "ban") setBanDuration("permanent");
    setDialog(next);
  };

  const memberList = (items: TeamMember[]) => (
    <div className="surface overflow-hidden rounded-md">
      <ul className="divide-border divide-y">
        {items.map((member) => {
          const active = member.user_id === currentUserId || onlineSet.has(member.user_id);
          const hasUnread = unreadPeerSet.has(member.user_id);
          const displayName = memberDisplayName(member.email);
          const conversationHref =
            member.user_id !== currentUserId && capabilities.canSendPrivateMessage
              ? teamPath(team.team_id, "messages", member.user_id)
              : null;
          const rowActions = capabilities.canManageMembers ? (
            <MemberRowActions
              member={member}
              pending={setRole.isPending || transfer.isPending || kick.isPending || ban.isPending}
              onSetRole={(role) => setRole.mutate({ userId: member.user_id, role })}
              onMakeManager={() => openDialog("makeManager", member)}
              onKick={() => openDialog("kick", member)}
              onBan={() => openDialog("ban", member)}
            />
          ) : null;

          return (
            <li
              key={member.user_id}
              className="relative flex items-center gap-3 px-4 py-4 transition-colors hover:bg-white/[0.03]"
            >
              {conversationHref ? (
                <Link
                  href={conversationHref}
                  className="focus-visible:ring-gold/50 absolute inset-0 rounded-md focus-visible:ring-2 focus-visible:outline-none"
                >
                  <span className="sr-only">
                    {tDm("openConversation", { email: member.email })}
                  </span>
                </Link>
              ) : null}
              <span className="relative shrink-0">
                <MemberAvatar email={member.email} role={member.role} />
                <span
                  title={active ? t("online") : t("offline")}
                  className={`border-bg absolute -right-0.5 -bottom-0.5 h-2.5 w-2.5 rounded-full border-2 ${
                    active ? "bg-st-res" : "bg-muted/40"
                  }`}
                />
              </span>
              <div className="flex min-w-0 flex-1 flex-col gap-1" title={member.email}>
                <div className="flex items-center gap-2">
                  <span
                    className={cn(
                      "text-text truncate text-sm leading-4",
                      hasUnread ? "font-bold" : "font-medium",
                    )}
                  >
                    {displayName}
                  </span>
                  {hasUnread ? (
                    <span className="text-muted inline-flex shrink-0 items-center gap-1 text-[11px] font-semibold tracking-wider uppercase">
                      <span>{tDm("newMessages")}</span>
                      <HelpCircle className="h-3 w-3 opacity-70" aria-hidden="true" />
                    </span>
                  ) : null}
                </div>
                <RoleChip role={member.role} showIcon={false} className="text-[11px] leading-3" />
              </div>
              <div className="relative z-10 flex shrink-0 items-center gap-1">{rowActions}</div>
            </li>
          );
        })}
      </ul>
    </div>
  );

  return (
    <section id="members" aria-label={t("members")} className="scroll-mt-24 space-y-4">
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-3">
        {visibleActiveMembers.length > 0 ? (
          <div className="flex items-baseline gap-2 px-1">
            <h2 id="active-members" className="text-text text-sm font-semibold">
              {t("activeMembers", { count: visibleActiveMembers.length })}
            </h2>
            <span className="text-muted text-xs tabular-nums">{visibleActiveMembers.length}</span>
          </div>
        ) : (
          <span aria-hidden="true" />
        )}
        <div className="ml-auto flex min-w-0 items-center gap-3">
          <label className="relative min-w-36 flex-1 sm:w-56 sm:flex-none">
            <span className="sr-only">{t("searchMembers")}</span>
            <Search
              className="text-muted pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2"
              aria-hidden="true"
            />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("searchMembers")}
              className="ow-input h-9 w-full rounded-md pr-3 pl-9 text-sm"
            />
          </label>
          <TableFilterControl
            label={t("roleFilter")}
            value={roleFilter === "all" ? "" : roleFilter}
            activeLabel={
              roleFilter === "all"
                ? undefined
                : t(`role${roleFilter[0].toUpperCase()}${roleFilter.slice(1)}`)
            }
            onChange={(value) => setRoleFilter((value || "all") as RoleFilter)}
            options={[
              { value: "", label: t("allRoles") },
              { value: "manager", label: t("roleManager") },
              { value: "responder", label: t("roleResponder") },
              { value: "observer", label: t("roleObserver") },
            ]}
          />
        </div>
      </div>

      {setRole.error ? <Alert tone="danger">{errorText(setRole.error.message)}</Alert> : null}
      {isLoading ? <TeamRosterRowsSkeleton /> : null}
      {error ? <Alert tone="danger">{t("membersFailed")}</Alert> : null}
      {!isLoading && !error && visibleMembers.length === 0 && visibleBans.length === 0 ? (
        <div className="text-muted border-border border-y px-4 py-3 text-sm">
          {query || roleFilter !== "all" ? t("noMatchingMembers") : t("noMembers")}
        </div>
      ) : null}

      {visibleActiveMembers.length > 0 ? (
        <section aria-labelledby="active-members">{memberList(visibleActiveMembers)}</section>
      ) : null}

      {visibleInactiveMembers.length > 0 ? (
        <section aria-labelledby="inactive-members" className="space-y-2">
          <div className="flex items-baseline gap-2 px-1">
            <h2 id="inactive-members" className="text-text text-sm font-semibold">
              {t("inactiveMembers", { count: visibleInactiveMembers.length })}
            </h2>
            <span className="text-muted text-xs tabular-nums">{visibleInactiveMembers.length}</span>
          </div>
          {memberList(visibleInactiveMembers)}
        </section>
      ) : null}

      {capabilities.canManageMembers &&
      (bans.isLoading || bans.error || unban.error || (hasBans && visibleBans.length > 0)) ? (
        <section aria-labelledby="banned-members" className="space-y-2">
          <div className="flex items-baseline gap-2 px-1">
            <h2 id="banned-members" className="text-text text-sm font-semibold">
              {t("bannedMembers", { count: visibleBans.length })}
            </h2>
            <span className="text-muted text-xs tabular-nums">{visibleBans.length}</span>
          </div>
          <div
            className={
              bans.isLoading || bans.error || unban.error || visibleBans.length > 0
                ? "surface overflow-hidden rounded-md"
                : "border-border border-y"
            }
          >
            {bans.error || unban.error ? (
              <Alert tone="danger" className="m-4">
                {bans.error ? t("bansFailed") : errorText(unban.error!.message)}
              </Alert>
            ) : null}
            {bans.isLoading ? (
              <TeamRosterRowsSkeleton rows={1} />
            ) : (
              <ul className="divide-border divide-y">
                {visibleBans.map((entry) => (
                  <li
                    key={`ban:${entry.user.user_id}:${entry.created_at}`}
                    className="flex flex-col gap-3 px-4 py-4 md:grid md:grid-cols-[auto_minmax(0,1fr)_auto] md:items-center"
                  >
                    <MemberAvatar email={entry.user.email} />
                    <div className="min-w-0">
                      <div className="text-text truncate font-medium" title={entry.user.email}>
                        {memberDisplayName(entry.user.email)}
                      </div>
                      <div className="text-muted mt-0.5 text-xs">
                        {entry.kind === "permanent"
                          ? t("permanentBan")
                          : t("banExpires", {
                              date: new Intl.DateTimeFormat(locale, {
                                dateStyle: "medium",
                                timeStyle: "short",
                              }).format(new Date(entry.expires_at!)),
                            })}
                      </div>
                    </div>
                    <div className="flex justify-end">
                      <IconButton
                        label={t("unban")}
                        size="sm"
                        variant="ghost"
                        loading={unban.isPending && unban.variables === entry.user.user_id}
                        onClick={() => unban.mutate(entry.user.user_id)}
                      >
                        <ShieldOff className="h-4 w-4" aria-hidden="true" />
                      </IconButton>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>
      ) : null}

      <ConfirmDialog
        open={dialog === "makeManager"}
        title={t("makeManager")}
        description={t("transferConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("makeManager")}
        cancelLabel={t("cancel")}
        intent="standard"
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
        intent="destructive"
        pendingLabel={t("processing")}
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
        intent="destructive"
        pendingLabel={t("processing")}
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
    </section>
  );
}
