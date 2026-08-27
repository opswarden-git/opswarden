"use client";

import { useEffect } from "react";
import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { NotificationsPanel } from "@/components/settings/NotificationsPanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";
import { useSearchParams } from "next/navigation";
import { PageContent } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { useRouter } from "@/i18n/routing";

export default function SettingsPage() {
  const searchParams = useSearchParams();
  const router = useRouter();

  useEffect(() => {
    if (searchParams.size > 0) router.replace("/settings");
  }, [router, searchParams]);

  return (
    <PageLayout>
      <PageContent className="surface mx-auto w-full max-w-3xl space-y-8 rounded-md p-6">
        <ProfilePanel />
        <LanguagePanel />
        <NotificationsPanel />
        <AccountDangerZone />
      </PageContent>
    </PageLayout>
  );
}
