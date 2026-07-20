"use client";

import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";
import { SettingsConnectorsPanel } from "@/components/settings/SettingsConnectorsPanel";
import { useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { PageTabs } from "@/components/layout/PageTabs";
import { settingsView } from "@/lib/settings-routing";

export default function SettingsPage() {
  const t = useTranslations("Settings");
  const searchParams = useSearchParams();
  const view = settingsView(searchParams.get("view"));

  return (
    <PageLayout>
      <PageHeader title={t("title")} />
      <PageTabs
        ariaLabel={t("viewsLabel")}
        tabs={[
          { href: "/settings", label: t("general"), active: view === "general" },
          {
            href: "/settings?view=connectors",
            label: t("connectors"),
            active: view === "connectors",
          },
        ]}
      />
      {view === "general" ? (
        <PageContent className="space-y-6">
          <ProfilePanel />
          <LanguagePanel />
          <AccountDangerZone />
        </PageContent>
      ) : (
        <SettingsConnectorsPanel />
      )}
    </PageLayout>
  );
}
