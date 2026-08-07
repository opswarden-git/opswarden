"use client";

import { ArrowRight, Siren, X } from "lucide-react";
import { useTranslations } from "next-intl";
import { Link, usePathname } from "@/i18n/routing";
import { useIncident } from "@/lib/queries/incidents";
import { parseTeamPath, teamPath } from "@/lib/team-routing";
import { useAuthStore } from "@/store/auth";
import { useIncidentContextStore } from "@/store/incident-context";
import { useTeamScope } from "@/components/teams/TeamScope";
import { SeverityChip } from "./SeverityChip";
import { StateChip } from "./StateChip";
import { buttonClassNames, IconButton } from "@/components/ui/Button";

export function ActiveIncidentContextBar() {
  const t = useTranslations("Incidents");
  const pathname = usePathname();
  const userId = useAuthStore((state) => state.user?.id);
  const activeIncident = useIncidentContextStore((state) => state.activeIncident);
  const hasHydrated = useIncidentContextStore((state) => state.hasHydrated);
  const clear = useIncidentContextStore((state) => state.clear);
  const { teams } = useTeamScope();

  const incidentId =
    hasHydrated && activeIncident && activeIncident.ownerId === userId
      ? activeIncident.incidentId
      : undefined;
  const { data: incident } = useIncident(incidentId);

  if (!activeIncident || !incidentId) return null;

  const incidentHref = teamPath(activeIncident.teamId, "incidents", activeIncident.incidentId);
  const route = parseTeamPath(pathname);
  const isCurrentIncident =
    route?.section === "incidents" && route.resourceId === activeIncident.incidentId;
  const team = teams.find((candidate) => candidate.team_id === activeIncident.teamId);
  const title =
    incident?.title ?? t("incidentBreadcrumb", { id: activeIncident.incidentId.slice(0, 8) });

  return (
    <section
      aria-label={t("activeIncidentContext")}
      data-active-incident-context="true"
      className="border-border bg-bg-2/95 relative z-30 shrink-0 border-b backdrop-blur-md"
    >
      <div className="mx-auto flex min-h-14 w-full max-w-[90rem] items-center gap-3 px-4 py-2 sm:px-6 md:px-8">
        <span className="bg-panel-2 text-gold flex h-8 w-8 shrink-0 items-center justify-center rounded-md">
          <Siren className="h-4 w-4" aria-hidden="true" />
        </span>

        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-2">
            <span className="text-muted-2 shrink-0 text-[10px] font-semibold tracking-[0.12em] uppercase">
              {t("activeIncidentContext")}
            </span>
            {team ? (
              <span className="text-muted-2 hidden truncate text-xs sm:inline">{team.name}</span>
            ) : null}
          </div>
          <div className="mt-0.5 flex min-w-0 items-center gap-2">
            {isCurrentIncident ? (
              <span className="truncate text-sm font-medium">{title}</span>
            ) : (
              <Link
                href={incidentHref}
                className="hover:text-gold truncate text-sm font-medium transition-colors"
              >
                {title}
              </Link>
            )}
            {incident ? (
              <div className="hidden shrink-0 items-center gap-2 lg:flex">
                <StateChip status={incident.status} />
                <SeverityChip severity={incident.severity} />
              </div>
            ) : null}
          </div>
        </div>

        {!isCurrentIncident ? (
          <Link
            href={incidentHref}
            aria-label={t("viewOpen")}
            className={buttonClassNames({ size: "sm", variant: "secondary" })}
          >
            <span className="hidden sm:inline">{t("viewOpen")}</span>
            <ArrowRight className="h-4 w-4" aria-hidden="true" />
          </Link>
        ) : null}
        <IconButton
          label={t("exitIncidentContext")}
          title={t("exitIncidentContext")}
          variant="ghost"
          size="sm"
          onClick={clear}
        >
          <X className="h-4 w-4" aria-hidden="true" />
        </IconButton>
      </div>
    </section>
  );
}
