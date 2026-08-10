import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TeamSettingsPage } from "./TeamSettingsPage";

const replace = vi.fn();
vi.mock("@/i18n/routing", () => ({
  useRouter: () => ({ replace }),
  usePathname: () => "/teams/team-1/team",
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

const mutation = () => ({
  error: null,
  isPending: false,
  mutate: vi.fn(),
  reset: vi.fn(),
  variables: undefined,
});

const transfer = mutation();
const leave = mutation();
const remove = mutation();
const unban = mutation();
const invitationCode = vi.fn();

vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({
    data: [
      {
        team_id: "team-1",
        name: "Operations",
        role: "manager",
        created_at: "2026-07-25T10:00:00Z",
        member_count: 2,
        active_incident_count: 1,
        active_release_count: 1,
        blocked_release_count: 0,
      },
    ],
    isLoading: false,
    error: null,
  }),
  useTeamMembers: () => ({
    data: [
      {
        user_id: "manager-1",
        email: "manager@example.com",
        role: "manager",
        joined_at: "2026-07-25T10:00:00Z",
      },
      {
        user_id: "responder-1",
        email: "responder@example.com",
        role: "responder",
        joined_at: "2026-07-25T10:00:00Z",
      },
    ],
  }),
  useInvitationCode: (teamId: string, enabled: boolean) => {
    invitationCode(teamId, enabled);
    return {
      data: { invitation_code: "invite-secret" },
      isLoading: false,
      error: null,
    };
  },
  useTeamBans: () => ({
    data: [
      {
        user: { user_id: "banned-1", email: "banned@example.com" },
        kind: "permanent",
        expires_at: null,
        reason: "Abuse",
        moderator: { user_id: "manager-1", email: "manager@example.com" },
        created_at: "2026-07-25T10:00:00Z",
        active: true,
      },
      {
        user: { user_id: "expired-1", email: "expired@example.com" },
        kind: "temporary",
        expires_at: "2026-07-24T10:00:00Z",
        reason: null,
        moderator: null,
        created_at: "2026-07-23T10:00:00Z",
        active: false,
      },
    ],
    isLoading: false,
    error: null,
  }),
  useTransferManager: () => transfer,
  useSetMemberRole: () => mutation(),
  useKickMember: () => mutation(),
  useBanMember: () => mutation(),
  useLeaveTeam: () => leave,
  useDeleteTeam: () => remove,
  useUnbanMember: () => unban,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("TeamSettingsPage", () => {
  it("renders the Team identity, Members and Danger as one flat page", () => {
    render(<TeamSettingsPage teamId="team-1" />);

    const identity = screen.getByRole("banner");
    expect(within(identity).getByRole("heading", { name: "Operations" })).toBeInTheDocument();
    expect(within(identity).getByText("roleManager")).toBeInTheDocument();
    expect(within(identity).getByText(/createdOn:/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "membersWithCount:2" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "danger" })).toBeInTheDocument();
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
    expect(screen.queryByText("invite-secret")).not.toBeInTheDocument();
  });

  it("keeps active and banned accounts in two distinct rosters", () => {
    render(<TeamSettingsPage teamId="team-1" />);

    expect(screen.getByRole("heading", { name: "activeMembers" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "bannedMembers" })).toBeInTheDocument();
    expect(screen.getByText("banned@example.com")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "unban" }));
    expect(unban.mutate).toHaveBeenCalledWith("banned-1");
  });

  it("requires typed confirmation before deleting the team", () => {
    render(<TeamSettingsPage teamId="team-1" />);
    fireEvent.click(screen.getByRole("button", { name: "deleteTeam" }));
    const dialog = screen.getByRole("dialog", { name: "deleteTeam" });
    const confirm = screen.getByRole("button", { name: "deleteTeam" });
    expect(confirm).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox", { name: "DELETE" }), {
      target: { value: "DELETE" },
    });
    fireEvent.click(confirm);
    expect(remove.mutate).toHaveBeenCalledWith(
      undefined,
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
    expect(dialog).toBeInTheDocument();
  });

  it("keeps the join code behind the role-gated Members action", () => {
    render(<TeamSettingsPage teamId="team-1" />);

    expect(screen.queryByText("invite-secret")).not.toBeInTheDocument();
    expect(invitationCode).toHaveBeenCalledWith("team-1", false);

    fireEvent.click(screen.getByRole("button", { name: "shareJoinCode" }));
    expect(screen.getByRole("dialog", { name: "shareJoinCode" })).toBeInTheDocument();
    expect(screen.getByText("invite-secret")).toBeInTheDocument();
    expect(invitationCode).toHaveBeenCalledWith("team-1", true);
  });
});
