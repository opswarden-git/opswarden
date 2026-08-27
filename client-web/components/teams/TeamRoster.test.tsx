import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Team } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { TeamRoster, TeamRosterRowsSkeleton } from "./TeamRoster";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
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

let onlineMembers = ["manager-1", "responder-1"];
vi.mock("@/lib/ws", () => ({ useTeamOnline: () => onlineMembers }));

const setRole = { error: null, isPending: false, mutate: vi.fn() };
const transfer = { error: null, isPending: false, mutate: vi.fn() };
const kick = { error: null, isPending: false, mutate: vi.fn() };
const ban = { error: null, isPending: false, mutate: vi.fn() };
const unban = { error: null, isPending: false, variables: undefined, mutate: vi.fn() };
const bannedMember = {
  user: { user_id: "banned-1", email: "banned@example.com" },
  kind: "permanent",
  expires_at: null,
  reason: null,
  moderator: null,
  created_at: "2026-07-23T10:00:00Z",
  active: true,
};
let teamBans = [bannedMember];
const members = [
  {
    user_id: "manager-1",
    email: "manager@example.com",
    role: "manager" as const,
    joined_at: "2026-07-20T10:00:00Z",
  },
  {
    user_id: "responder-1",
    email: "first.responder@example.com",
    role: "responder" as const,
    joined_at: "2026-07-21T10:00:00Z",
  },
  {
    user_id: "observer-1",
    email: "observer@example.com",
    role: "observer" as const,
    joined_at: "2026-07-22T10:00:00Z",
  },
];

vi.mock("@/lib/queries/teams", () => ({
  useTeamMembers: () => ({ data: members, isLoading: false, error: null }),
  useSetMemberRole: () => setRole,
  useTransferManager: () => transfer,
  useKickMember: () => kick,
  useBanMember: () => ban,
  useTeamBans: () => ({ data: teamBans, isLoading: false, error: null }),
  useUnbanMember: () => unban,
}));
const team: Team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager",
  created_at: "2026-07-20T10:00:00Z",
  member_count: 3,
  active_incident_count: 1,
  active_release_count: 1,
  blocked_release_count: 0,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAuthStore.getState().logout();
  teamBans = [bannedMember];
  onlineMembers = ["manager-1", "responder-1"];
});

describe("TeamRoster", () => {
  it("keeps loading rows aligned with the compact identity layout", () => {
    render(<TeamRosterRowsSkeleton />);

    const skeleton = screen.getByTestId("team-roster-skeleton");
    expect(skeleton.children).toHaveLength(3);
    expect(skeleton.firstElementChild).toHaveClass("flex", "items-center");
  });

  it("makes each peer row a direct link to its conversation", () => {
    useAuthStore
      .getState()
      .setUser({ id: "manager-1", email: "manager@example.com", locale: "en" });
    render(<TeamRoster team={team} />);

    expect(screen.getAllByText("First Responder").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Observer").length).toBeGreaterThan(0);
    expect(
      screen.getByRole("link", { name: "openConversation:first.responder@example.com" }),
    ).toHaveAttribute("href", "/teams/team-1/messages/responder-1");
    expect(
      screen.getByRole("link", { name: "openConversation:observer@example.com" }),
    ).toHaveAttribute("href", "/teams/team-1/messages/observer-1");
    expect(
      screen.queryByRole("link", { name: "openConversation:manager@example.com" }),
    ).not.toBeInTheDocument();
    expect(
      within(screen.getByRole("heading", { name: "activeMembers:2" }).parentElement!).getByText(
        "2",
      ),
    ).toBeInTheDocument();
    expect(
      within(screen.getByRole("heading", { name: "inactiveMembers:1" }).parentElement!).getByText(
        "1",
      ),
    ).toBeInTheDocument();
  });

  it("filters members by email and role", () => {
    render(<TeamRoster team={team} />);
    const search = screen.getByRole("textbox", { name: "searchMembers" });
    fireEvent.change(search, { target: { value: "observer" } });
    expect(screen.getAllByText("Observer").length).toBeGreaterThan(0);
    expect(screen.queryByText("First Responder")).not.toBeInTheDocument();

    fireEvent.change(search, { target: { value: "nobody" } });
    expect(screen.getByText("noMatchingMembers")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /^bannedMembers/ })).not.toBeInTheDocument();
  });

  it("keeps members flat and separates banned accounts when present", () => {
    render(<TeamRoster team={team} />);

    expect(screen.getByRole("heading", { name: "activeMembers:2" })).toBeInTheDocument();
    expect(screen.getAllByText("Manager").length).toBeGreaterThan(0);
    expect(screen.getByRole("heading", { name: "bannedMembers:1" })).toBeInTheDocument();
    expect(screen.getByText("Banned")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "unban" }));
    expect(unban.mutate).toHaveBeenCalledWith("banned-1");
  });

  it("omits the banned section when the Team has no bans", () => {
    teamBans = [];
    render(<TeamRoster team={team} />);

    expect(screen.queryByRole("heading", { name: /^bannedMembers/ })).not.toBeInTheDocument();
  });

  it("omits the inactive section when every member is present", () => {
    onlineMembers = members.map((member) => member.user_id);
    render(<TeamRoster team={team} />);

    expect(screen.getByRole("heading", { name: "activeMembers:3" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /^inactiveMembers/ })).not.toBeInTheDocument();
  });

  it("changes a peer role through its row action menu", async () => {
    useAuthStore.getState().setUser({ id: "manager-1", locale: "en" });
    render(<TeamRoster team={team} />);
    fireEvent.change(screen.getByRole("textbox", { name: "searchMembers" }), {
      target: { value: "observer" },
    });
    const menus = screen.getAllByRole("button", { name: "actionsTitle" });
    fireEvent.pointerDown(menus[0], { button: 0, ctrlKey: false });
    const promote = await screen.findByRole("menuitem", { name: "makeResponder" });
    fireEvent.click(promote);
    expect(setRole.mutate).toHaveBeenCalledWith({ userId: "observer-1", role: "responder" });
  });
});
