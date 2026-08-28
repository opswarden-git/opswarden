import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GuidedTour } from "./GuidedTour";

const mocks = vi.hoisted(() => ({
  guided: new Set(["incidents", "releases"]),
}));

vi.mock("next-intl", () => ({
  useTranslations: () => (key: string, values?: { step: number; total: number }) =>
    values ? `${values.step}/${values.total}` : key,
}));

vi.mock("@/components/teams/TeamScope", () => ({
  useTeamScope: () => ({ activeTeam: { team_id: "team-1" } }),
}));

vi.mock("@/lib/firstRunGuidance", () => ({
  useFirstRunGuidance: () => mocks.guided,
}));

beforeEach(() => {
  window.localStorage.clear();
  document.body.innerHTML =
    '<a data-guide-target="incidents"></a><a data-guide-target="releases"></a>';
  for (const anchor of document.querySelectorAll("a")) {
    vi.spyOn(anchor, "getBoundingClientRect").mockReturnValue({
      top: 40,
      right: 120,
      bottom: 60,
      left: 20,
      width: 100,
      height: 20,
      x: 20,
      y: 40,
      toJSON: () => undefined,
    });
  }
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("GuidedTour", () => {
  it("advances through applicable steps and persists completion", async () => {
    render(<GuidedTour />);

    expect(await screen.findByRole("dialog", { name: "tourLabel" })).toHaveTextContent(
      "tourIncidents",
    );
    expect(screen.getByText("1/2")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "tourNext" }));
    expect(screen.getByRole("dialog", { name: "tourLabel" })).toHaveTextContent("tourReleases");

    fireEvent.click(screen.getByRole("button", { name: "tourDone" }));
    expect(screen.queryByRole("dialog", { name: "tourLabel" })).not.toBeInTheDocument();
    expect(window.localStorage.getItem("opswarden-tour:team-1")).toBe("1");
  });

  it("does not reopen a completed tour", () => {
    window.localStorage.setItem("opswarden-tour:team-1", "1");
    render(<GuidedTour />);

    expect(screen.queryByRole("dialog", { name: "tourLabel" })).not.toBeInTheDocument();
  });
});
