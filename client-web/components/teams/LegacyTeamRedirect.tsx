"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { useRouter } from "@/i18n/routing";
import { useTeams } from "@/lib/queries/teams";
import { teamPath, type TeamSection } from "@/lib/team-routing";

export function LegacyTeamRedirect({ section = "incidents" }: { section?: TeamSection }) {
  const t = useTranslations("TeamSwitcher");
  const router = useRouter();
  const { data: teams, isLoading } = useTeams();

  React.useEffect(() => {
    if (isLoading || !teams) return;
    router.replace(teams[0] ? teamPath(teams[0].team_id, section) : "/teams");
  }, [isLoading, router, section, teams]);

  return (
    <PageLayout>
      <PageHeader title={t("openingTeam")} />
      <PageContent
        state="loading"
        loadingFallback={
          <div className="text-muted animate-pulse py-10 text-center text-sm">
            {t("redirecting")}
          </div>
        }
      />
    </PageLayout>
  );
}
