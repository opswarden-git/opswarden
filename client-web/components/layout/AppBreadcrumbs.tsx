"use client";

import { useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { Link, usePathname } from "@/i18n/routing";
import { parseTeamPath, teamPath } from "@/lib/team-routing";
import { useTeamScope } from "@/components/teams/TeamScope";

type BreadcrumbItem = {
  href: string;
  label: string;
};

export function AppBreadcrumbs() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const tIncidents = useTranslations("Incidents");
  const tReleases = useTranslations("Releases");
  const tSidebar = useTranslations("Sidebar");
  const tAutomations = useTranslations("Automations");
  const { activeTeam } = useTeamScope();
  const route = parseTeamPath(pathname);

  let items: BreadcrumbItem[] = [];

  if (pathname === "/teams") {
    items = [{ href: "/teams", label: tSidebar("teams") }];
  } else if (pathname === "/settings") {
    items = [{ href: "/settings", label: tSidebar("settings") }];
  } else if (route) {
    const automationView = searchParams.get("view") ?? "rules";
    const sectionLabel =
      route.section === "automations"
        ? automationView === "connections"
          ? tSidebar("integrations")
          : automationView === "runs"
            ? tAutomations("runHistory")
            : tSidebar("rules")
        : tSidebar(route.section === "settings" ? "teamSettings" : route.section);
    const sectionHref =
      route.section === "automations" && automationView !== "rules"
        ? `${teamPath(route.teamId, route.section)}?view=${automationView}`
        : teamPath(route.teamId, route.section);
    const preservedQuery = route.resourceId ? searchParams.toString() : "";
    const teamItem = {
      href: teamPath(route.teamId, "overview"),
      label: activeTeam?.name ?? tSidebar("teams"),
    };
    items = [
      teamItem,
      {
        href: preservedQuery ? `${sectionHref}?${preservedQuery}` : sectionHref,
        label: sectionLabel,
      },
    ];

    if (route.section === "automations" && automationView === "runs") {
      items.splice(1, 0, {
        href: teamPath(route.teamId, "automations"),
        label: tSidebar("rules"),
      });
    }

    if (route.resourceId) {
      const fallback =
        route.section === "incidents"
          ? tIncidents("incidentBreadcrumb", { id: route.resourceId.slice(0, 8) })
          : tReleases("releaseDetail");
      items.push({
        href: teamPath(route.teamId, route.section, route.resourceId),
        label: fallback,
      });
    }
  }

  if (items.length === 0) return null;

  const resourceDetail = !!route?.resourceId;

  return (
    <div className="mx-auto w-full max-w-[90rem] px-4 pt-6 sm:px-6 md:px-8 md:pt-8">
      <nav aria-label={tIncidents("breadcrumbLabel")} className="min-w-0 text-sm">
        <ol className="text-muted flex min-w-0 items-center gap-2">
          {items.map((item, index) => {
            const current = index === items.length - 1;
            const hideTeamOnNarrowDetail = resourceDetail && index === 0;
            return (
              <li
                key={`${index}:${item.href}`}
                className={hideTeamOnNarrowDetail ? "hidden sm:contents" : "contents"}
              >
                {index > 0 ? (
                  <span
                    aria-hidden="true"
                    className={resourceDetail && index === 1 ? "hidden sm:inline" : undefined}
                  >
                    /
                  </span>
                ) : null}
                {current && !resourceDetail ? (
                  <h1 className="min-w-0 truncate text-sm font-medium">
                    <Link href={item.href} aria-current="page" className="text-text">
                      {item.label}
                    </Link>
                  </h1>
                ) : (
                  <Link
                    href={item.href}
                    aria-current={current ? "page" : undefined}
                    className={
                      current
                        ? "text-text min-w-0 shrink-0 truncate font-medium"
                        : "hover:text-text min-w-0 truncate transition-colors"
                    }
                  >
                    {item.label}
                  </Link>
                )}
              </li>
            );
          })}
        </ol>
      </nav>
    </div>
  );
}
