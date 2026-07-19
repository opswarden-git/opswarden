"use client";

import { LogOut, Shield, Trash2, UserRoundCog } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import React, { useMemo, useState } from "react";
import { useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import {
  type Team,
  useDeleteTeam,
  useInvitationCode,
  useLeaveTeam,
  useTeamBans,
  useTeamMembers,
  useTeams,
  useTransferManager,
  useUnbanMember,
} from "@/lib/queries/teams";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { CopyButton } from "@/components/ui/CopyButton";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { RoleChip } from "./RoleChip";
import { TeamHeader } from "./TeamHeader";

type Dialog = "transfer" | "leave" | "delete" | null;
type BanView = "active" | "expired";

function SettingsSection({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="surface rounded-md">
      <div className="border-border border-b px-6 py-4">
        <h2 className="text-text font-semibold">{title}</h2>
        <p className="text-muted mt-1 text-sm">{description}</p>
      </div>
      <div className="p-6">{children}</div>
    </section>
  );
}

function TeamSettings({ team }: { team: Team }) {
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const router = useRouter();
  const capabilities = deriveCapabilities(team.role);
  const { data: members = [] } = useTeamMembers(team.team_id);
  const invitation = useInvitationCode(team.team_id, capabilities.canViewInvitationCode);
  const bans = useTeamBans(team.team_id, capabilities.canManageMembers);
  const transfer = useTransferManager(team.team_id);
  const leave = useLeaveTeam(team.team_id);
  const remove = useDeleteTeam(team.team_id);
  const unban = useUnbanMember(team.team_id);

  const [dialog, setDialog] = useState<Dialog>(null);
  const [newManagerId, setNewManagerId] = useState("");
  const [banView, setBanView] = useState<BanView>("active");
  const transferCandidates = members.filter((member) => member.role !== "manager");
  const visibleBans = useMemo(
    () => (bans.data ?? []).filter((entry) => entry.active === (banView === "active")),
    [banView, bans.data],
  );
  const targetManager = members.find((member) => member.user_id === newManagerId);
  const errorText = (error: Error | null) =>
    error ? (tErr.has(error.message) ? tErr(error.message) : t("actionFailed")) : null;

  const leaveOrDeleteDone = () => router.replace("/teams");

  return (
    <div className="space-y-6">
      <SettingsSection title={t("teamIdentity")} description={t("teamIdentityDescription")}>
        <dl className="grid gap-5 sm:grid-cols-2">
          <div>
            <dt className="text-muted text-xs font-medium tracking-wide uppercase">{t("name")}</dt>
            <dd className="text-text mt-1 font-medium">{team.name}</dd>
          </div>
          <div>
            <dt className="text-muted text-xs font-medium tracking-wide uppercase">
              {t("yourRole")}
            </dt>
            <dd className="mt-1">
              <RoleChip role={team.role} />
            </dd>
          </div>
        </dl>
      </SettingsSection>

      {capabilities.canViewInvitationCode ? (
        <SettingsSection title={t("invitation")} description={t("invitationDescription")}>
          {invitation.isLoading ? (
            <div className="bg-muted/20 h-10 w-64 animate-pulse rounded-md" />
          ) : invitation.error ? (
            <Alert tone="danger">{t("invitationFailed")}</Alert>
          ) : (
            <div className="flex max-w-lg items-center gap-2">
              <code className="surface-subtle border-border text-text min-w-0 flex-1 rounded-md border px-3 py-2 font-mono text-sm">
                {invitation.data?.invitation_code}
              </code>
              {invitation.data ? (
                <CopyButton
                  value={invitation.data.invitation_code}
                  label={t("copyInvitationCode")}
                  copiedLabel={t("invitationCodeCopied")}
                />
              ) : null}
            </div>
          )}
        </SettingsSection>
      ) : null}

      {capabilities.canManageMembers ? (
        <SettingsSection title={t("ownership")} description={t("ownershipDescription")}>
          <div className="flex max-w-2xl flex-col gap-3 sm:flex-row sm:items-end">
            <label className="min-w-0 flex-1 space-y-2">
              <span className="text-muted text-sm">{t("transferPickMember")}</span>
              <select
                value={newManagerId}
                onChange={(event) => setNewManagerId(event.target.value)}
                className="ow-input h-10 w-full rounded-md px-3 text-sm"
              >
                <option value="">{t("transferPickMember")}</option>
                {transferCandidates.map((member) => (
                  <option key={member.user_id} value={member.user_id}>
                    {member.email}
                  </option>
                ))}
              </select>
            </label>
            <Button
              onClick={() => {
                transfer.reset();
                setDialog("transfer");
              }}
              disabled={!newManagerId}
            >
              <UserRoundCog className="h-4 w-4" />
              {t("transferManager")}
            </Button>
          </div>
          {transferCandidates.length === 0 ? (
            <p className="text-muted mt-3 text-sm">{t("transferNeedsMember")}</p>
          ) : null}
        </SettingsSection>
      ) : null}

      {capabilities.canManageMembers ? (
        <SettingsSection title={t("bannedMembers")} description={t("bannedMembersDescription")}>
          <div
            className="border-border mb-4 flex gap-1 border-b"
            role="tablist"
            aria-label={t("banViews")}
          >
            {(["active", "expired"] as const).map((view) => (
              <Button
                key={view}
                variant="ghost"
                size="sm"
                role="tab"
                aria-selected={banView === view}
                onClick={() => setBanView(view)}
                className={
                  banView === view
                    ? "text-text border-gold rounded-b-none border-b-2"
                    : "rounded-b-none"
                }
              >
                {view === "active" ? t("activeBans") : t("expiredBans")}
              </Button>
            ))}
          </div>
          {bans.isLoading ? (
            <div className="text-muted py-6 text-sm">{t("loadingBans")}</div>
          ) : bans.error ? (
            <Alert tone="danger">{t("bansFailed")}</Alert>
          ) : visibleBans.length === 0 ? (
            <div className="text-muted py-6 text-center text-sm">{t("noBansInView")}</div>
          ) : (
            <ul className="divide-border divide-y">
              {visibleBans.map((entry) => (
                <li
                  key={entry.user.user_id}
                  className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center"
                >
                  <div className="min-w-0 flex-1">
                    <div className="text-text truncate font-medium">{entry.user.email}</div>
                    <div className="text-muted mt-1 text-xs">
                      {entry.kind === "permanent"
                        ? t("permanentBan")
                        : t("banExpires", {
                            date: new Intl.DateTimeFormat(locale, {
                              dateStyle: "medium",
                              timeStyle: "short",
                            }).format(new Date(entry.expires_at!)),
                          })}
                      {entry.moderator
                        ? ` · ${t("bannedBy", { email: entry.moderator.email })}`
                        : ""}
                    </div>
                    {entry.reason ? (
                      <p className="text-muted mt-1 text-sm">{entry.reason}</p>
                    ) : null}
                  </div>
                  <Button
                    size="sm"
                    onClick={() => unban.mutate(entry.user.user_id)}
                    loading={unban.isPending && unban.variables === entry.user.user_id}
                  >
                    <Shield className="h-4 w-4" />
                    {t("unban")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {unban.error ? (
            <Alert tone="danger" className="mt-4">
              {errorText(unban.error)}
            </Alert>
          ) : null}
        </SettingsSection>
      ) : null}

      {capabilities.canDeleteTeam ? (
        <section className="border-sev-critical/40 rounded-md border">
          <div className="border-sev-critical/30 border-b px-6 py-4">
            <h2 className="text-sev-critical font-semibold">{t("dangerZone")}</h2>
            <p className="text-muted mt-1 text-sm">{t("deleteTeamDesc")}</p>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-4 p-6">
            <div className="text-muted text-sm">{t("deleteTeamWarning")}</div>
            <Button
              variant="danger"
              onClick={() => {
                remove.reset();
                setDialog("delete");
              }}
            >
              <Trash2 className="h-4 w-4" />
              {t("deleteTeam")}
            </Button>
          </div>
        </section>
      ) : (
        <SettingsSection title={t("membership")} description={t("membershipDescription")}>
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="text-muted text-sm">{t("leaveTeamWarning")}</div>
            <Button
              onClick={() => {
                leave.reset();
                setDialog("leave");
              }}
            >
              <LogOut className="h-4 w-4" />
              {t("leaveTeam")}
            </Button>
          </div>
        </SettingsSection>
      )}

      <ConfirmDialog
        open={dialog === "transfer"}
        title={t("transferManager")}
        description={t("transferConfirm", { email: targetManager?.email ?? "" })}
        confirmLabel={t("transfer")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        pending={transfer.isPending}
        error={errorText(transfer.error)}
        onConfirm={() =>
          newManagerId && transfer.mutate(newManagerId, { onSuccess: () => setDialog(null) })
        }
        onClose={() => setDialog(null)}
      />
      <ConfirmDialog
        open={dialog === "leave"}
        title={t("leaveTeam")}
        description={t("leaveConfirm", { name: team.name })}
        confirmLabel={t("leaveTeam")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        danger
        pending={leave.isPending}
        error={errorText(leave.error)}
        onConfirm={() => leave.mutate(undefined, { onSuccess: leaveOrDeleteDone })}
        onClose={() => setDialog(null)}
      />
      <ConfirmDialog
        open={dialog === "delete"}
        title={t("deleteTeam")}
        description={t("deleteConfirm", { name: team.name })}
        confirmLabel={t("deleteTeam")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        requireType="DELETE"
        danger
        pending={remove.isPending}
        error={errorText(remove.error)}
        onConfirm={() => remove.mutate(undefined, { onSuccess: leaveOrDeleteDone })}
        onClose={() => setDialog(null)}
      />
    </div>
  );
}

export function TeamSettingsPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const { data: teams, isLoading, error } = useTeams();
  const team = teams?.find((candidate) => candidate.team_id === teamId);
  const state: PageContentState = isLoading ? "loading" : error || !team ? "error" : "ready";

  return (
    <PageLayout>
      {team ? <TeamHeader team={team} /> : null}
      <PageContent
        state={state}
        loadingFallback={
          <div className="text-muted animate-pulse py-12 text-center">{t("loading")}</div>
        }
        errorFallback={<Alert tone="danger">{t("teamUnavailable")}</Alert>}
      >
        {team ? <TeamSettings team={team} /> : null}
      </PageContent>
    </PageLayout>
  );
}
