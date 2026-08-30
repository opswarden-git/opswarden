"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import {
  notificationSoundsEnabled,
  playNotificationSound,
  setNotificationSoundsEnabled,
  subscribeToNotificationSounds,
} from "@/lib/notificationSounds";
import {
  getNotificationPermission,
  NotificationPermissionState,
  requestNotificationPermission,
} from "@/lib/desktopNotify";
import { SettingsRow, SettingsSection } from "./SettingsPrimitives";

const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

let cachedPermission: NotificationPermissionState = "default";

function readPermission(): NotificationPermissionState {
  void getNotificationPermission().then((perm) => {
    if (cachedPermission !== perm) {
      cachedPermission = perm;
      listeners.forEach((l) => l());
    }
  });
  return cachedPermission;
}

export function NotificationsPanel() {
  const t = useTranslations("Settings");
  const state = React.useSyncExternalStore(
    subscribe,
    readPermission,
    () => "default" as NotificationPermissionState,
  );
  const soundsEnabled = React.useSyncExternalStore(
    subscribeToNotificationSounds,
    notificationSoundsEnabled,
    () => false,
  );

  const request = async () => {
    try {
      const res = await requestNotificationPermission();
      cachedPermission = res;
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

  const toggleSounds = () => {
    const enabled = !soundsEnabled;
    setNotificationSoundsEnabled(enabled);
    if (enabled) void playNotificationSound("message");
  };

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
      <SettingsRow
        label={t("notificationSounds")}
        action={
          <Button
            size="sm"
            variant={soundsEnabled ? "secondary" : "primary"}
            onClick={toggleSounds}
          >
            {soundsEnabled ? t("soundsDisable") : t("soundsEnable")}
          </Button>
        }
      >
        <span className="text-muted">
          {soundsEnabled ? t("notificationsOn") : t("notificationsOff")}
        </span>
      </SettingsRow>
    </SettingsSection>
  );
}
