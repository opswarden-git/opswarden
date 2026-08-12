"use client";

import { useEffect } from "react";
import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
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
      <PageContent className="w-full max-w-4xl space-y-8">
        <ProfilePanel />
        <LanguagePanel />
        <AccountDangerZone />
      </PageContent>
    </PageLayout>
  );
}
