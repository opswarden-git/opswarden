import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Team } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { TeamRoster } from "./TeamRoster";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

vi.mock("@/lib/ws", () => ({ useTeamOnline: () => ["manager-1", "responder-1"] }));

const setRole = { error: null, isPending: false, mutate: vi.fn() };
const transfer = { error: null, isPending: false, mutate: vi.fn() };
const kick = { error: null, isPending: false, mutate: vi.fn() };
const ban = { error: null, isPending: false, mutate: vi.fn() };
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
}));
vi.mock("@/lib/queries/privateMessages", () => ({
  usePrivateMessages: () => ({ data: [], isLoading: false, error: null }),
  useSendPrivateMessage: () => ({ error: null, isPending: false, mutate: vi.fn() }),
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
});

describe("TeamRoster", () => {
  it("renders online state, roles and private-message affordances", () => {
    useAuthStore
      .getState()
      .setUser({ id: "manager-1", email: "manager@example.com", locale: "en" });
    render(<TeamRoster team={team} />);

    expect(screen.getAllByText("first.responder@example.com").length).toBeGreaterThan(0);
    expect(screen.getAllByText("observer@example.com").length).toBeGreaterThan(0);
    expect(screen.getAllByRole("button", { name: "message" })).toHaveLength(4);
    expect(screen.getByText("onlineCount:2")).toBeInTheDocument();
  });

  it("filters members by email and role", () => {
    render(<TeamRoster team={team} />);
    const search = screen.getByRole("textbox", { name: "searchMembers" });
    fireEvent.change(search, { target: { value: "observer" } });
    expect(screen.getAllByText("observer@example.com").length).toBeGreaterThan(0);
    expect(screen.queryByText("first.responder@example.com")).not.toBeInTheDocument();

    fireEvent.change(search, { target: { value: "nobody" } });
    expect(screen.getByText("noMatchingMembers")).toBeInTheDocument();
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
