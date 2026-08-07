import type { AnchorHTMLAttributes, ReactNode } from "react";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppBreadcrumbs } from "./AppBreadcrumbs";

let pathname = "/teams/team-1/incidents/incident-1";
let params = new URLSearchParams();

const messages: Record<string, string> = {
  "Automations.runHistory": "Run history",
  "Incidents.breadcrumbLabel": "Breadcrumb",
  "Incidents.incidentBreadcrumb": "Incident incident",
  "Sidebar.incidents": "Incidents",
  "Sidebar.settings": "Settings",
  "Sidebar.teams": "Workspace",
  "Sidebar.rules": "Rules",
  "Sidebar.integrations": "Integrations",
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
  useTeamScope: () => ({ activeTeam: { team_id: "team-1", name: "Platform" } }),
}));

afterEach(() => {
  cleanup();
  pathname = "/teams/team-1/incidents/incident-1";
  params = new URLSearchParams();
});

describe("AppBreadcrumbs", () => {
  it("derives a resource trail from the current route", () => {
    render(<AppBreadcrumbs />);

    const breadcrumb = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(breadcrumb).toHaveTextContent("Platform/Incidents/Incident incident");
    expect(screen.getByRole("link", { name: "Incident incident" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("makes a collection crumb the page heading", () => {
    pathname = "/teams/team-1/members";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "members" })).toBeInTheDocument();
    expect(screen.getAllByText("members")).toHaveLength(1);
  });

  it("supports an overview hierarchy whose levels share a destination", () => {
    pathname = "/teams/team-1/overview";
    render(<AppBreadcrumbs />);

    expect(
      screen.getByRole("navigation", { name: "Breadcrumb" }).getElementsByTagName("li"),
    ).toHaveLength(2);
    expect(screen.getByRole("heading", { level: 1, name: "overview" })).toBeInTheDocument();
  });

  it("also locates global pages outside a team", () => {
    pathname = "/settings";
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("link", { name: "Settings" })).toHaveAttribute("aria-current", "page");
    expect(screen.queryByText("Platform")).not.toBeInTheDocument();
  });

  it("names shared automation routes by their actual product destination", () => {
    pathname = "/teams/team-1/automations";
    params = new URLSearchParams("view=connections");
    const { unmount } = render(<AppBreadcrumbs />);

    expect(screen.getByRole("heading", { level: 1, name: "Integrations" })).toBeInTheDocument();
    unmount();

    params = new URLSearchParams("view=runs");
    render(<AppBreadcrumbs />);
    const breadcrumb = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(breadcrumb).toHaveTextContent("Platform/Rules/Run history");
    expect(screen.getByRole("heading", { level: 1, name: "Run history" })).toBeInTheDocument();
  });

  it("preserves list context in the collection link", () => {
    params = new URLSearchParams("view=escalated&severity=critical");
    render(<AppBreadcrumbs />);

    expect(screen.getByRole("link", { name: "Incidents" })).toHaveAttribute(
      "href",
      "/teams/team-1/incidents?view=escalated&severity=critical",
    );
  });
});
