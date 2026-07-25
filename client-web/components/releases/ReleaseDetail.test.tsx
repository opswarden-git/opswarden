import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Release } from "@/lib/queries/releases";
import { ReleaseDetail } from "./ReleaseDetail";

vi.mock("next-intl", () => ({
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

vi.mock("@/i18n/routing", () => ({
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const incidents = [
  {
    id: "incident-1",
    team_id: "team-1",
    title: "Database outage",
    description: "",
    status: "escalated" as const,
    severity: "critical" as const,
    assignee: null,
    created_at: "2026-07-25T10:00:00Z",
    created_by: null,
    updated_at: "2026-07-25T10:00:00Z",
  },
  {
    id: "incident-2",
    team_id: "team-1",
    title: "API latency",
    description: "",
    status: "open" as const,
    severity: "high" as const,
    assignee: null,
    created_at: "2026-07-25T10:00:00Z",
    created_by: null,
    updated_at: "2026-07-25T10:00:00Z",
  },
];

const validateStep = { error: null, isPending: false, mutate: vi.fn() };
const linkIncident = { error: null, isPending: false, mutate: vi.fn() };
const unlinkIncident = { error: null, isPending: false, mutate: vi.fn() };

vi.mock("@/lib/queries/incidents", () => ({ useIncidents: () => ({ data: incidents }) }));
vi.mock("@/lib/queries/teams", () => ({
  useTeamMembers: () => ({
    data: [{ user_id: "responder-1", email: "responder@example.com", role: "responder" }],
  }),
}));
vi.mock("@/lib/queries/releases", () => ({
  useValidateStep: () => validateStep,
  useLinkIncident: () => linkIncident,
  useUnlinkIncident: () => unlinkIncident,
}));

function release(overrides: Partial<Release> = {}): Release {
  return {
    release_id: "release-1",
    team_id: "team-1",
    title: "Production deployment",
    state: "in_progress",
    steps: [
      {
        position: 0,
        name: "Build",
        validated: true,
        validated_by: "responder-1",
        validated_at: "2026-07-25T10:00:00Z",
      },
      {
        position: 1,
        name: "Deploy",
        validated: false,
        validated_by: null,
        validated_at: null,
      },
      {
        position: 2,
        name: "Verify",
        validated: false,
        validated_by: null,
        validated_at: null,
      },
    ],
    linked_incident_ids: ["incident-1", "missing-incident"],
    created_at: "2026-07-25T09:00:00Z",
    updated_at: "2026-07-25T10:00:00Z",
    ...overrides,
  };
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("ReleaseDetail", () => {
  it("validates only the next ordered step", () => {
    render(<ReleaseDetail release={release()} teamId="team-1" role="manager" />);

    expect(screen.getAllByText("Deploy")).toHaveLength(2);
    expect(screen.getByText(/validatedBy:responder@example.com/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "validateNextStep" }));
    expect(validateStep.mutate).toHaveBeenCalledWith({
      releaseId: "release-1",
      step: "Deploy",
      teamId: "team-1",
    });
  });

  it("links and unlinks active incidents while preserving unknown linked ids", () => {
    render(<ReleaseDetail release={release()} teamId="team-1" role="manager" />);

    expect(screen.getByText("Database outage")).toBeInTheDocument();
    expect(screen.getByText("unknownIncident")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /unlinkIncident:Database outage/ }));
    expect(unlinkIncident.mutate).toHaveBeenCalledWith({
      releaseId: "release-1",
      incidentId: "incident-1",
      teamId: "team-1",
    });

    fireEvent.change(screen.getByRole("combobox", { name: "linkIncident" }), {
      target: { value: "incident-2" },
    });
    expect(linkIncident.mutate).toHaveBeenCalledWith({
      releaseId: "release-1",
      incidentId: "incident-2",
      teamId: "team-1",
    });
  });

  it("shows blockers and prevents progression while blocked", () => {
    render(
      <ReleaseDetail
        release={release({ state: "blocked", linked_incident_ids: ["incident-1"] })}
        teamId="team-1"
        role="responder"
      />,
    );

    expect(screen.getByText("blockedBannerTitle")).toBeInTheDocument();
    expect(screen.getByText("resolveBlockersFirst")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "validateNextStep" })).not.toBeInTheDocument();
  });

  it("hides mutating controls for terminal releases", () => {
    render(
      <ReleaseDetail
        release={release({ state: "completed", steps: [], linked_incident_ids: [] })}
        teamId="team-1"
        role="manager"
      />,
    );
    expect(screen.getByText("noLinkedIncidents")).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "linkIncident" })).not.toBeInTheDocument();
  });
});
