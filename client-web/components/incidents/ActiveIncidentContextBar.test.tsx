import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import { useIncidentContextStore } from "@/store/incident-context";
import { ActiveIncidentContextBar } from "./ActiveIncidentContextBar";

let pathname = "/teams/team-1/releases";

vi.mock("@/i18n/routing", () => ({
  usePathname: () => pathname,
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

vi.mock("next-intl", () => ({
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values ? `${key}:${Object.values(values).join(":")}` : key,
}));

vi.mock("@/lib/queries/incidents", () => ({
  useIncident: (id?: string) => ({
    data: id
      ? {
          id,
          team_id: "team-1",
          title: "Database outage",
          status: "open",
          severity: "critical",
        }
      : undefined,
  }),
}));

vi.mock("@/components/teams/TeamScope", () => ({
  useTeamScope: () => ({ teams: [{ team_id: "team-1", name: "Operations" }] }),
}));

beforeEach(() => {
  pathname = "/teams/team-1/releases";
  localStorage.clear();
  useAuthStore.setState({
    token: "token",
    user: { id: "operator-1", email: "operator@example.com", locale: "en" },
    hasHydrated: true,
  });
  useIncidentContextStore.setState({ activeIncident: null, hasHydrated: true });
});

afterEach(() => {
  cleanup();
  useIncidentContextStore.getState().clear();
});

describe("ActiveIncidentContextBar", () => {
  it("keeps a return path visible away from the incident and exits explicitly", () => {
    useIncidentContextStore.getState().activate({
      incidentId: "incident-1",
      teamId: "team-1",
      ownerId: "operator-1",
    });

    render(<ActiveIncidentContextBar />);

    expect(screen.getByRole("region", { name: "activeIncidentContext" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "viewOpen" })).toHaveAttribute(
      "href",
      "/teams/team-1/incidents/incident-1",
    );

    fireEvent.click(screen.getByRole("button", { name: "exitIncidentContext" }));
    expect(useIncidentContextStore.getState().activeIncident).toBeNull();
  });

  it("does not expose another operator's persisted incident", () => {
    useIncidentContextStore.getState().activate({
      incidentId: "incident-1",
      teamId: "team-1",
      ownerId: "another-operator",
    });

    render(<ActiveIncidentContextBar />);

    expect(screen.queryByRole("region", { name: "activeIncidentContext" })).toBeNull();
  });

  it("marks the current incident without offering a redundant return action", () => {
    pathname = "/teams/team-1/incidents/incident-1";
    useIncidentContextStore.getState().activate({
      incidentId: "incident-1",
      teamId: "team-1",
      ownerId: "operator-1",
    });

    render(<ActiveIncidentContextBar />);

    expect(screen.getByText("Database outage")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "viewOpen" })).toBeNull();
  });
});
