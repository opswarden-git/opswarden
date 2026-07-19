"use client";

import { Shield } from "lucide-react";
import { useTranslations } from "next-intl";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { useTeams } from "@/lib/queries/teams";
import { TeamRoster } from "./TeamRoster";
import { TeamHeader } from "./TeamHeader";

export function TeamMembersPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const { data: teams, isLoading, error } = useTeams();
  const team = teams?.find((candidate) => candidate.team_id === teamId);
  const contentState: PageContentState = isLoading ? "loading" : error || !team ? "error" : "ready";

  return (
    <PageLayout>
      {team ? <TeamHeader team={team} showTeamSwitcher /> : null}
      <PageContent
        state={contentState}
        loadingFallback={
          <div className="text-muted animate-pulse py-10 text-center text-sm">{t("loading")}</div>
        }
        errorFallback={
          error ? (
            <Alert tone="danger">{t("failedToLoad")}</Alert>
          ) : (
            <div className="surface rounded-md p-12 text-center">
              <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
              <p className="text-muted text-sm">{t("teamUnavailable")}</p>
            </div>
          )
        }
      >
        {team ? <TeamRoster team={team} /> : null}
      </PageContent>
    </PageLayout>
  );
}
