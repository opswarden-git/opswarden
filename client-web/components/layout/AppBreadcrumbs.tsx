"use client";

import { useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { Link, usePathname } from "@/i18n/routing";
import { useTeamScope } from "@/components/teams/TeamScope";
import { TeamSwitcher } from "@/components/teams/TeamSwitcher";
import { parseTeamPath, teamPath } from "@/lib/team-routing";

type BreadcrumbItem = {
  href: string;
  label: string;
};

/** A compact, stable location trail for every Team route. */
export function AppBreadcrumbs({
  onActionsHostChange,
}: {
  onActionsHostChange?: (host: HTMLDivElement | null) => void;
} = {}) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const { activeTeam } = useTeamScope();
  const tIncidents = useTranslations("Incidents");
  const tReleases = useTranslations("Releases");
  const tSidebar = useTranslations("Sidebar");
  const route = parseTeamPath(pathname);

  // The Incident War Room owns the whole operational frame. Repeating Team,
  // collection and Incident identity above its local navigation wastes height
  // and creates two competing navigation systems.
  if ((route?.section === "incidents" || route?.section === "messages") && route.resourceId) {
    return null;
  }

  let heading: string | null = null;
  let items: BreadcrumbItem[] = [];

  if (pathname === "/teams") {
    heading = tSidebar("teams");
  } else if (pathname === "/settings") {
    heading = tSidebar("account");
  } else if (route) {
    const automationView = searchParams.get("view") ?? "rules";
    const sectionLabel =
      route.section === "automations"
        ? automationView === "connections"
          ? tSidebar("integrations")
          : automationView === "runs"
            ? tSidebar("runs")
            : tSidebar("rules")
        : route.section === "activity"
          ? tSidebar("runs")
          : route.section === "settings"
            ? tSidebar("team")
            : tSidebar(route.section);

    heading = route.resourceId ? null : sectionLabel;

    if (activeTeam) {
      const sectionHref = teamPath(route.teamId, route.section);
      const preservedQuery = searchParams.toString();
      const currentSectionHref = preservedQuery ? `${sectionHref}?${preservedQuery}` : sectionHref;
      items = [];

      items.push({ href: currentSectionHref, label: sectionLabel });

      if (route.resourceId) {
        const resourceLabel =
          route.section === "incidents"
            ? tIncidents("incidentBreadcrumb", { id: route.resourceId.slice(0, 8) })
            : tReleases("releaseDetail");

        items.push({
          href: teamPath(route.teamId, route.section, route.resourceId),
          label: resourceLabel,
        });
      }
    }
  }

  if (items.length === 0) return heading ? <h1 className="sr-only">{heading}</h1> : null;

  return (
    <div data-page-topbar="true" className="workspace-frame px-4 pt-6 sm:px-6 md:px-8 md:pt-8">
      {heading ? <h1 className="sr-only">{heading}</h1> : null}
      <div className="flex min-h-9 min-w-0 flex-wrap items-center justify-between gap-4">
        <nav aria-label={tIncidents("breadcrumbLabel")} className="min-w-0 flex-1 text-sm">
          <ol className="text-muted flex min-w-0 items-center gap-2 overflow-hidden whitespace-nowrap">
            <li className="min-w-0">
              <TeamSwitcher presentation="breadcrumb" />
            </li>
            {items.map((item, index) => {
              const current = index === items.length - 1;

              return (
                <li key={`${item.href}:${index}`} className="contents">
                  <span aria-hidden="true">/</span>
                  <Link
                    href={item.href}
                    aria-current={current ? "page" : undefined}
                    className={
                      current
                        ? "text-text min-w-0 truncate font-medium"
                        : "hover:text-text min-w-0 truncate transition-colors"
                    }
                  >
                    {item.label}
                  </Link>
                </li>
              );
            })}
          </ol>
        </nav>
        <div
          ref={onActionsHostChange}
          data-page-actions-host="true"
          className="flex min-w-0 flex-wrap items-center justify-end gap-2"
        />
      </div>
    </div>
  );
}
