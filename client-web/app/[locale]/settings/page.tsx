import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";
import { useTranslations } from "next-intl";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";

export default function SettingsPage() {
  const t = useTranslations("Settings");

  return (
    <PageLayout>
      <PageHeader title={t("title")} />
      <PageContent className="space-y-6">
        <ProfilePanel />
        <LanguagePanel />
        <AccountDangerZone />
      </PageContent>
    </PageLayout>
  );
}
