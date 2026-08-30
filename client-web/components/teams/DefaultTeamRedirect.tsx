"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { PageContent } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { useRouter } from "@/i18n/routing";
import { teamPath } from "@/lib/team-routing";
import { useTeamScope } from "./TeamScope";

export function DefaultTeamRedirect() {
  const t = useTranslations("TeamSwitcher");
  const tTeams = useTranslations("Teams");
  const router = useRouter();
  const { teams, isLoading, error } = useTeamScope();

  React.useEffect(() => {
    if (isLoading || error || !teams) return;
    router.replace(teams[0] ? teamPath(teams[0].team_id, "incidents") : "/teams");
  }, [error, isLoading, router, teams]);

  const state = error ? "error" : "loading";

  return (
    <PageLayout>
      <PageHeader title={t("openingTeam")} />
      <PageContent
        state={state}
        loadingFallback={
          <div className="text-muted animate-pulse py-10 text-center text-sm">
            {t("redirecting")}
          </div>
        }
        errorFallback={
          <Alert tone="danger" className="mx-auto max-w-md my-8">
            {tTeams("failedToLoad")}
          </Alert>
        }
      />
    </PageLayout>
  );
}
