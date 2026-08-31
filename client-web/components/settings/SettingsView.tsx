import React from "react";
import { ProfilePanel } from "./ProfilePanel";
import { LanguagePanel } from "./LanguagePanel";
import { NotificationsPanel } from "./NotificationsPanel";
import { AccountDangerZone } from "./AccountDangerZone";

export function SettingsView() {
  return (
    <div className="space-y-6">
      <ProfilePanel />
      <LanguagePanel />
      <NotificationsPanel />
      <AccountDangerZone />
    </div>
  );
}
