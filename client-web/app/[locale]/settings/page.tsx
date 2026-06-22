"use client";

import React, { useState } from "react";
import { Sliders, Workflow } from "lucide-react";
import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";
import { IntegrationsPanel } from "@/components/settings/IntegrationsPanel";
import { useTranslations } from "next-intl";

export default function SettingsPage() {
  const t = useTranslations("Settings");
  const [activeTab, setActiveTab] = useState<"profile" | "integrations">("profile");

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex flex-col justify-between gap-4 md:flex-row md:items-center">
        <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>

        {/* Horizontal Pill Tabs */}
        <div className="surface-subtle border-border flex items-center gap-1 rounded-md border p-1">
          <button
            onClick={() => setActiveTab("profile")}
            className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              activeTab === "profile"
                ? "text-text bg-white/[0.07] shadow-sm"
                : "text-muted hover:text-text hover:bg-white/[0.045]"
            }`}
          >
            <Sliders className="h-4 w-4" />
            {t("general")}
          </button>
          <button
            onClick={() => setActiveTab("integrations")}
            className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              activeTab === "integrations"
                ? "text-text bg-white/[0.07] shadow-sm"
                : "text-muted hover:text-text hover:bg-white/[0.045]"
            }`}
          >
            <Workflow className="h-4 w-4" />
            {t("connectors")}
          </button>
        </div>
      </div>

      <div className="space-y-6">
        {activeTab === "profile" && (
          <>
            <ProfilePanel />
            <LanguagePanel />
            <AccountDangerZone />
          </>
        )}
        {activeTab === "integrations" && <IntegrationsPanel />}
      </div>
    </div>
  );
}
