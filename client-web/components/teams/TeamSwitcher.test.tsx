import type { AnchorHTMLAttributes, ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TeamSwitcher } from "./TeamSwitcher";

const messages: Record<string, string> = {
  label: "Current team",
  noTeams: "No teams",
  directory: "Team directory",
  allTeams: "All teams",
};
const switchTeam = vi.fn();

vi.mock("next-intl", () => ({
  useTranslations: () => (key: string) => messages[key] ?? key,
}));

vi.mock("@/i18n/routing", () => ({
  usePathname: () => "/teams/team-1/overview",
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

vi.mock("./TeamScope", () => ({
  useTeamScope: () => ({
    teams: [
      { team_id: "team-1", name: "Platform" },
      { team_id: "team-2", name: "Security Lab" },
    ],
    activeTeam: { team_id: "team-1", name: "Platform" },
    isLoading: false,
    switchTeam,
  }),
}));

afterEach(cleanup);

describe("TeamSwitcher", () => {
  it("opens Team scope from the breadcrumb", async () => {
    render(<TeamSwitcher presentation="breadcrumb" />);

    fireEvent.pointerDown(screen.getByRole("button", { name: "Current team: Platform" }), {
      button: 0,
      ctrlKey: false,
    });
    const currentTeam = await screen.findByRole("menuitemcheckbox", { name: "Platform" });
    expect(currentTeam).toBeChecked();
    expect(currentTeam.firstElementChild?.tagName).toBe("SPAN");
    expect(currentTeam.lastElementChild?.tagName).toBe("svg");
    fireEvent.click(screen.getByRole("menuitemcheckbox", { name: "Security Lab" }));
    expect(switchTeam).toHaveBeenCalledWith("team-2");

    fireEvent.pointerDown(screen.getByRole("button", { name: "Current team: Platform" }), {
      button: 0,
      ctrlKey: false,
    });
    const allTeams = await screen.findByRole("menuitem", { name: "All teams" });
    expect(allTeams).toHaveAttribute("href", "/teams");
    expect(allTeams.querySelector("svg")).toBeNull();
  });

  it("keeps the existing compact presentation as the default", () => {
    render(<TeamSwitcher compact />);

    expect(screen.getByRole("link", { name: "Team directory" })).toHaveAttribute("href", "/teams");
    expect(screen.queryByText("Workspace")).not.toBeInTheDocument();
  });
});
