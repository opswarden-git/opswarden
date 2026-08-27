"use client";

import React, { useState } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import { ShieldAlert, UserRound } from "lucide-react";
import { useCreateTeam, useTeams } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { useLocale, useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { memberDisplayName } from "@/components/teams/MemberAvatar";
import { IdentityHeader, SettingsRow, SettingsSection } from "./SettingsPrimitives";

/** Team setup (when the user has no team yet) + read-only user identity card. */
export function ProfilePanel({
  showIdentityHeader = true,
  showTeamSetup = true,
}: {
  showIdentityHeader?: boolean;
  showTeamSetup?: boolean;
}) {
  const t = useTranslations("Settings");
  const locale = useLocale();
  const tErr = useTranslations("errors");
  const router = useRouter();
  const params = useParams();
  const currentLocale = params.locale as string;
  const searchParams = useSearchParams();
  const [teamName, setTeamName] = useState("");
  const user = useAuthStore((state) => state.user);
  const email = user?.email ?? t("unknown");
  const { data: teams } = useTeams();
  const createTeam = useCreateTeam();
  const needsTeamSetup =
    showTeamSetup && (searchParams.get("setup") === "team" || teams?.length === 0);

  const handleCreateTeam = (e: React.FormEvent) => {
    e.preventDefault();
    const name = teamName.trim();
    if (!name) return;

    createTeam.mutate(name, {
      onSuccess: () => {
        setTeamName("");
        router.replace(`/${currentLocale}/settings`);
      },
    });
  };

  return (
    <>
      {needsTeamSetup && (
        <div className="surface border-gold/30 rounded-md p-6">
          <div className="mb-4 flex items-start gap-3">
            <ShieldAlert className="text-gold mt-0.5 h-5 w-5 shrink-0" />
            <div>
              <h2 className="text-text text-lg font-semibold tracking-tight">{t("setupTitle")}</h2>
              <p className="text-gold/70 mt-1 text-sm">{t("setupDesc")}</p>
            </div>
          </div>
          <form onSubmit={handleCreateTeam} className="flex flex-col gap-3 sm:flex-row">
            <label className="min-w-0 flex-1">
              <span className="text-text mb-2 block text-sm font-medium">{t("teamName")}</span>
              <input
                type="text"
                value={teamName}
                onChange={(e) => setTeamName(e.target.value)}
                className="ow-input flex h-10 w-full min-w-0 rounded-md px-3 py-2 text-sm transition-colors"
              />
            </label>
            <Button
              type="submit"
              variant="primary"
              size="lg"
              loading={createTeam.isPending}
              disabled={!teamName.trim()}
            >
              {createTeam.isPending ? t("creating") : t("createTeam")}
            </Button>
          </form>
          {createTeam.isError && (
            <p className="mt-2 text-sm text-red-400" role="alert">
              {tErr.has(createTeam.error.message)
                ? tErr(createTeam.error.message)
                : t("actionFailed")}
            </p>
          )}
        </div>
      )}

      {showIdentityHeader ? (
        <IdentityHeader
          mark={<UserRound className="text-gold h-7 w-7" aria-hidden="true" />}
          title={user?.email ? memberDisplayName(user.email) : t("user")}
        />
      ) : null}

      <SettingsSection title={t("profile")}>
        <SettingsRow label={t("emailLabel")}>
          <span className="font-medium break-all">{email}</span>
        </SettingsRow>
        <SettingsRow
          label={t("userId")}
          action={
            user?.id ? (
              <CopyButton
                value={user.id}
                label={t("copyUserId")}
                copiedLabel={t("userIdCopied")}
                size="sm"
              />
            ) : null
          }
        >
          <span className="text-muted font-mono text-xs break-all">{user?.id ?? t("unknown")}</span>
        </SettingsRow>
        {user?.created_at ? (
          <SettingsRow label={t("memberSince")}>
            <time dateTime={user.created_at}>
              {new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(
                new Date(user.created_at),
              )}
            </time>
          </SettingsRow>
        ) : null}
      </SettingsSection>
    </>
  );
}
