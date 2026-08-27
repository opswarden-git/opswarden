"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { SettingsRow, SettingsSection } from "./SettingsPrimitives";

type PermissionState = "unsupported" | "default" | "granted" | "denied";

function readPermission(): PermissionState {
  if (typeof window === "undefined" || !("Notification" in window)) return "unsupported";
  return window.Notification.permission as PermissionState;
}

/**
 * The one place a member can grant notification permission.
 *
 * Browsers only honour `Notification.requestPermission()` when it follows a
 * real click — Firefox rejects it outright otherwise. Asking from the realtime
 * handler, at the moment an event arrives, therefore never granted anything and
 * failed silently: permission stayed `default` forever and every notification
 * was dropped. This asks from a button instead, and says out loud what the
 * current answer is, including the one state the app cannot recover from on its
 * own.
 */
/**
 * `Notification.permission` is an external store that never announces its own
 * changes, so the subscription exists only to let us re-read after we asked.
 */
const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function NotificationsPanel() {
  const t = useTranslations("Settings");
  const state = React.useSyncExternalStore(
    subscribe,
    readPermission,
    // The server has no `Notification`; the client corrects on hydration.
    () => "default" as PermissionState,
  );

  const request = async () => {
    try {
      await window.Notification.requestPermission();
    } catch {
      // A refusal is an answer; re-reading tells us which one.
    }
    listeners.forEach((listener) => listener());
  };

  const label =
    state === "granted"
      ? t("notificationsOn")
      : state === "denied"
        ? t("notificationsBlocked")
        : state === "unsupported"
          ? t("notificationsUnsupported")
          : t("notificationsOff");

  return (
    <SettingsSection title={t("notifications")}>
      <SettingsRow
        label={t("desktopNotifications")}
        action={
          state === "default" ? (
            <Button size="sm" variant="primary" onClick={request}>
              {t("notificationsEnable")}
            </Button>
          ) : null
        }
      >
        <span className="text-muted">{label}</span>
      </SettingsRow>
    </SettingsSection>
  );
}
