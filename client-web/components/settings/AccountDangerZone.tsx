"use client";

import React, { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { LogOut, PencilLine, Trash2 } from "lucide-react";
import { apiFetch } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { useTranslations } from "next-intl";

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
        setDeleteError(
          body?.code && tErr.has(body.code) ? tErr(body.code) : (body?.error ?? t("deleteFailed")),
        );
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
    <div className="surface rounded-md p-6">
      <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
        <PencilLine className="text-muted h-5 w-5" />
        {t("accountActions")}
      </h2>
      <div className="mt-4 space-y-4">
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-sm font-medium text-red-400">{t("logOutSession")}</h3>
          </div>
          <button
            onClick={handleLogout}
            className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50"
          >
            <LogOut className="h-4 w-4" />
            {t("logOut")}
          </button>
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-sm font-medium text-red-400">{t("deleteAccountTitle")}</h3>
          </div>
          <button
            onClick={() => {
              setDeleteError(null);
              setDeleteOpen(true);
            }}
            className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50"
          >
            <Trash2 className="h-4 w-4" />
            {t("deleteAccount")}
          </button>
        </div>
      </div>

      <ConfirmDialog
        open={deleteOpen}
        title={t("deleteAccount")}
        description={t("deleteModalDesc", { email: user?.email ?? "—" })}
        confirmLabel={t("deleteAccount")}
        cancelLabel={t("cancel")}
        pendingLabel={t("deleting")}
        danger
        requireType="DELETE"
        pending={deletePending}
        error={deleteError}
        onConfirm={handleDeleteAccount}
        onClose={() => {
          setDeleteOpen(false);
          setDeleteError(null);
        }}
      />
    </div>
  );
}
