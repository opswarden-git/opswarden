"use client";

import { ExternalLink, Plug } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { ConnectionsView } from "@/components/automations/ConnectionsView";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { Alert } from "@/components/ui/Alert";
import { buttonClassNames } from "@/components/ui/Button";
import { Link, useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import {
  useAutomationCatalog,
  useAutomationRules,
  useTeamConnections,
} from "@/lib/queries/automations";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";

function ConnectionsLoading({ label }: { label: string }) {
  return (
    <div className="grid gap-4 lg:grid-cols-2" aria-label={label}>
      {[0, 1].map((index) => (
        <div key={index} className="surface h-64 animate-pulse rounded-md p-5">
          <div className="bg-panel-2 h-10 w-10 rounded-md" />
          <div className="bg-panel-2 mt-5 h-4 w-1/3 rounded" />
          <div className="bg-panel-2 mt-3 h-3 w-3/4 rounded" />
        </div>
      ))}
    </div>
  );
}

/** Account-level entry point over the canonical Team-scoped connections.
 * Secrets and mutations stay attached to a Team; this view only selects the
 * Team and reuses the same connection controls as the automation workspace. */
export function SettingsConnectorsPanel() {
  const t = useTranslations("Settings");
  const tAutomations = useTranslations("Automations");
  const searchParams = useSearchParams();
  const router = useRouter();
  const teams = useTeams();
  const manageableTeams = (teams.data ?? []).filter(
    (team) => deriveCapabilities(team.role).canManageAutomations,
  );
  const requestedTeamId = searchParams.get("team");
  const selectedTeam =
    manageableTeams.find((team) => team.team_id === requestedTeamId) ?? manageableTeams[0];
  const teamId = selectedTeam?.team_id ?? "";
  const enabled = !!teamId;
  const catalog = useAutomationCatalog(enabled);
  const connections = useTeamConnections(teamId, enabled);
  const rules = useAutomationRules(teamId, enabled);

  const state: PageContentState =
    teams.isLoading || (enabled && (catalog.isLoading || connections.isLoading || rules.isLoading))
      ? "loading"
      : teams.error || (enabled && (catalog.error || connections.error || rules.error))
        ? "error"
        : "ready";

  const selectTeam = (nextTeamId: string) => {
    const params = new URLSearchParams(searchParams.toString());
    params.set("view", "connectors");
    params.set("team", nextTeamId);
    router.replace(`/settings?${params.toString()}`);
  };

  return (
    <PageContent
      state={state}
      loadingFallback={<ConnectionsLoading label={t("loadingConnectors")} />}
      errorFallback={<Alert tone="danger">{t("connectorsUnavailable")}</Alert>}
    >
      <div className="space-y-6">
        <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
          <div>
            <div className="flex items-center gap-2">
              <Plug className="text-muted h-5 w-5" aria-hidden="true" />
              <h2 className="text-text text-xl font-semibold">{t("connectors")}</h2>
            </div>
            <p className="text-muted mt-1 max-w-3xl text-sm">{t("connectorsDescription")}</p>
          </div>

          {selectedTeam ? (
            <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
              <label className="space-y-1.5">
                <span className="text-muted block text-xs font-medium">{t("connectorTeam")}</span>
                <select
                  value={selectedTeam.team_id}
                  onChange={(event) => selectTeam(event.target.value)}
                  className="ow-input h-9 min-w-56 rounded-md px-3 text-sm"
                >
                  {manageableTeams.map((team) => (
                    <option key={team.team_id} value={team.team_id}>
                      {team.name}
                    </option>
                  ))}
                </select>
              </label>
              <Link
                href={`${teamPath(selectedTeam.team_id, "automations")}?view=connections`}
                className={buttonClassNames({ size: "md" })}
              >
                {t("openAutomations")}
                <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
              </Link>
            </div>
          ) : null}
        </header>

        <Alert tone="info">{t("connectorsScope")}</Alert>

        {!selectedTeam ? (
          <Alert tone="warning" title={tAutomations("managerOnlyTitle")}>
            {t("noManagedTeams")}
          </Alert>
        ) : (
          <ConnectionsView
            teamId={selectedTeam.team_id}
            catalog={catalog.data ?? []}
            connections={connections.data ?? []}
            rules={rules.data ?? []}
          />
        )}
      </div>
    </PageContent>
  );
}
