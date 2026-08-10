import type { AnchorHTMLAttributes, ReactNode } from "react";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppBreadcrumbs } from "./AppBreadcrumbs";

let pathname = "/teams/team-1/incidents/incident-1";
let params = new URLSearchParams();

const messages: Record<string, string> = {
  "Incidents.breadcrumbLabel": "Breadcrumb",
  "Incidents.incidentBreadcrumb": "Incident incident",
  "Releases.releaseDetail": "Release release",
  "Sidebar.account": "Account",
  "Sidebar.incidents": "Incidents",
  "Sidebar.settings": "Settings",
  "Sidebar.team": "Team",
  "Sidebar.teams": "Workspace",
  "Sidebar.rules": "Rules",
  "Sidebar.runs": "Runs",
  "Sidebar.integrations": "Integrations",
  "TeamSwitcher.label": "Current team",
  "TeamSwitcher.noTeams": "No teams",
  "TeamSwitcher.allTeams": "All teams",
};

vi.mock("next-intl", () => ({
  useTranslations: (namespace: string) => (key: string) => messages[`${namespace}.${key}`] ?? key,
}));

vi.mock("next/navigation", () => ({
  useSearchParams: () => params,
}));

vi.mock("@/i18n/routing", () => ({
  usePathname: () => pathname,
  Link: ({
    children,
    href,
    ...props
  }: { children: ReactNode; href: string } & AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

vi.mock("@/components/teams/TeamScope", () => ({
  useTeamScope: () => ({
    activeTeam: { team_id: "team-1", name: "Platform" },
    teams: [{ team_id: "team-1", name: "Platform" }],
    isLoading: false,
    switchTeam: vi.fn(),
  }),
}));

afterEach(() => {
  cleanup();
  pathname = "/teams/team-1/incidents/incident-1";
  params = new URLSearchParams();
});

describe("AppBreadcrumbs", () => {
  it("gives the Incident War Room the full operational frame", () => {
    render(<AppBreadcrumbs />);

    expect(screen.queryByRole("navigation", { name: "Breadcrumb" })).not.toBeInTheDocument();
    expect(screen.queryByText("Platform")).not.toBeInTheDocument();
  });

  it("locates the legacy Members route without inventing a Settings parent", () => {
    pathname = "/teams/team-1/members";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "members" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toHaveTextContent(
      "Platform/members",
    );
    expect(screen.getByRole("link", { name: "members" })).toHaveAttribute("aria-current", "page");
  });

  it("keeps Team scope visible on Overview", () => {
    pathname = "/teams/team-1/overview";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "overview" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toHaveTextContent(
      "Platform/overview",
    );
  });

  it("keeps Team as one resource without a tab in the breadcrumb", () => {
    pathname = "/teams/team-1/team";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "Team" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toHaveTextContent(
      "Platform/Team",
    );
  });

  it("also locates global pages outside a team", () => {
    pathname = "/settings";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "Account" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Breadcrumb" })).not.toBeInTheDocument();
    expect(screen.queryByText("Platform")).not.toBeInTheDocument();
  });

  it("names automation routes as direct Team resources", () => {
    pathname = "/teams/team-1/integrations";
    const { unmount } = render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "Integrations" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toHaveTextContent(
      "Platform/Integrations",
    );
    unmount();

    pathname = "/teams/team-1/runs";
    render(<AppBreadcrumbs />);
    expect(screen.getByRole("heading", { level: 1, name: "Runs" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Breadcrumb" })).toHaveTextContent(
      "Platform/Runs",
    );
  });

  it("preserves list context in the collection link", () => {
    pathname = "/teams/team-1/incidents";
    params = new URLSearchParams("view=escalated&severity=critical");
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("link", { name: "Incidents" })).toHaveAttribute(
      "href",
      "/teams/team-1/incidents?view=escalated&severity=critical",
    );
  });
});
