import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TeamAutomationsPage } from "./TeamAutomationsPage";

vi.mock("next/navigation", () => ({ useSearchParams: () => new URLSearchParams() }));
vi.mock("next-intl", () => ({
  useTranslations: () => (key: string) => key,
}));
vi.mock("@/i18n/routing", () => ({
  useRouter: () => ({ push: vi.fn() }),
}));

const loadingQuery = { data: undefined, error: null, isLoading: true, isFetching: false };
vi.mock("@/lib/queries/teams", () => ({ useTeams: () => loadingQuery }));
vi.mock("@/lib/queries/automations", () => ({
  useAutomationCatalog: () => loadingQuery,
  useAutomationRules: () => loadingQuery,
  useAutomationRuns: () => ({ ...loadingQuery, refetch: vi.fn() }),
  useTeamConnections: () => loadingQuery,
}));

afterEach(cleanup);

describe("TeamAutomationsPage loading morphology", () => {
  it("keeps Rules and Runs on their real table column counts", () => {
    const rules = render(<TeamAutomationsPage teamId="team-1" resource="rules" />);
    expect(
      within(screen.getByRole("table", { name: "loading" })).getAllByRole("columnheader"),
    ).toHaveLength(7);
    rules.unmount();

    render(<TeamAutomationsPage teamId="team-1" resource="runs" />);
    expect(
      within(screen.getByRole("table", { name: "loading" })).getAllByRole("columnheader"),
    ).toHaveLength(6);
  });

  it("keeps Integrations as two groups with the complete six-service catalog shape", () => {
    render(<TeamAutomationsPage teamId="team-1" resource="integrations" />);

    const skeleton = screen.getByLabelText("loading");
    expect(skeleton.querySelectorAll("section")).toHaveLength(2);
    expect(skeleton.querySelectorAll("section > .surface > div")).toHaveLength(6);
  });
});
