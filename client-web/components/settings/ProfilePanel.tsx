"use client";

import React from "react";
import { UserRound } from "lucide-react";
import { useAuthStore } from "@/store/auth";
import { useLocale, useTranslations } from "next-intl";
import { CopyButton } from "@/components/ui/CopyButton";
import { memberDisplayName } from "@/components/teams/MemberAvatar";
import { IdentityHeader, SettingsRow, SettingsSection } from "./SettingsPrimitives";

/** Read-only user identity card. */
export function ProfilePanel({ showIdentityHeader = true }: { showIdentityHeader?: boolean }) {
  const t = useTranslations("Settings");
  const locale = useLocale();
  const user = useAuthStore((state) => state.user);
  const email = user?.email ?? t("unknown");

  return (
    <>
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
