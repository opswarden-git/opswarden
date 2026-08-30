import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { NotificationsPanel } from "@/components/settings/NotificationsPanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";
import { PageContent } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";

export default function SettingsPage() {
  return (
    <PageLayout>
      <PageContent className="surface mx-auto w-full max-w-3xl rounded-md p-6">
        <ProfilePanel />
        <LanguagePanel />
        <NotificationsPanel />
        <AccountDangerZone />
      </PageContent>
    </PageLayout>
  );
}
