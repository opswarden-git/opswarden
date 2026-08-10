"use client";

import React, { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { SettingsRow, SettingsSection } from "./SettingsPrimitives";

/** Logout + delete-account (typed-confirm). */
export function AccountDangerZone() {
  const t = useTranslations("Settings");
  const tErr = useTranslations("errors");
  const router = useRouter();
  const params = useParams();
  const currentLocale = params.locale as string;
  const user = useAuthStore((state) => state.user);
  const logoutLocal = useAuthStore((state) => state.logout);

  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deletePending, setDeletePending] = useState(false);

  const handleLogout = async () => {
    await apiFetch("/api/auth/logout", { method: "POST" }).catch(() => undefined);
    logoutLocal();
    router.push(`/${currentLocale}/login`);
  };

  const handleDeleteAccount = async () => {
    setDeletePending(true);
    setDeleteError(null);

    try {
      const res = await apiFetch("/api/me", { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        setDeleteError(body?.code && tErr.has(body.code) ? tErr(body.code) : t("deleteFailed"));
        return;
      }
      logoutLocal();
      router.push(`/${currentLocale}/signup`);
    } catch {
      setDeleteError(t("deleteFailed"));
    } finally {
      setDeletePending(false);
    }
  };

  return (
    <SettingsSection title={t("accountActions")}>
      <SettingsRow
        label={t("logOutSession")}
        action={
          <Button variant="secondary" onClick={handleLogout}>
            {t("logOut")}
          </Button>
        }
      >
        <span className="text-muted">{user?.email ?? t("unknown")}</span>
      </SettingsRow>
      <SettingsRow
        label={<span className="text-red-400">{t("deleteAccountTitle")}</span>}
        action={
          <Button
            variant="danger"
            onClick={() => {
              setDeleteError(null);
              setDeleteOpen(true);
            }}
          >
            {t("deleteAccount")}
          </Button>
        }
      >
        <span className="text-muted">{t("deleteAccountSummary")}</span>
      </SettingsRow>

      <ConfirmDialog
        open={deleteOpen}
        title={t("deleteAccount")}
        description={t("deleteModalDesc", { email: user?.email ?? "—" })}
        confirmLabel={t("deleteAccount")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("deleting")}
        requireType="DELETE"
        pending={deletePending}
        error={deleteError}
        onConfirm={handleDeleteAccount}
        onClose={() => {
          setDeleteOpen(false);
          setDeleteError(null);
        }}
      />
    </SettingsSection>
  );
}
