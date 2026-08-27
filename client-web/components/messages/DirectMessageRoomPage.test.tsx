import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import { DirectMessageRoomPage } from "./DirectMessageRoomPage";

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

vi.mock("@/components/incidents/WarRoomNavigation", () => ({
  WarRoomNavigation: () => <aside aria-label="roomNavigation" />,
}));

const peer = {
  user_id: "peer-1",
  email: "peer@example.com",
  role: "responder" as const,
  joined_at: "2026-08-01T10:00:00Z",
};
let membersQuery: { data: (typeof peer)[]; isLoading: boolean; error: Error | null } = {
  data: [peer],
  isLoading: false,
  error: null,
};
let messagesQuery: {
  data: {
    pages: Array<{
      messages: Array<{
        id: string;
        sender_id: string;
        recipient_id: string;
        content: string;
        created_at: string;
        edited_at: null;
        attachments: never[];
        reactions: never[];
      }>;
      next_cursor: null;
      features: Array<
        | "send_text"
        | "send_gif"
        | "edit_own_message"
        | "react"
        | "attach_files"
        | "paginated_history"
        | "presence"
        | "typing"
      >;
    }>;
  };
  isLoading: boolean;
  isFetching: boolean;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: ReturnType<typeof vi.fn>;
  error: Error | null;
} = {
  data: {
    pages: [
      {
        messages: [
          {
            id: "mine",
            sender_id: "me-1",
            recipient_id: "peer-1",
            content: "My update",
            created_at: "2026-08-10T10:01:00Z",
            edited_at: null,
            attachments: [],
            reactions: [],
          },
          {
            id: "theirs",
            sender_id: "peer-1",
            recipient_id: "me-1",
            content: "Their update",
            created_at: "2026-08-10T10:00:00Z",
            edited_at: null,
            attachments: [],
            reactions: [],
          },
          {
            id: "gif",
            sender_id: "peer-1",
            recipient_id: "me-1",
            content: "giphy:https://media.giphy.com/media/abc/giphy.gif",
            created_at: "2026-08-10T09:59:00Z",
            edited_at: null,
            attachments: [],
            reactions: [],
          },
        ],
        next_cursor: null,
        features: [
          "send_text",
          "send_gif",
          "edit_own_message",
          "react",
          "attach_files",
          "paginated_history",
          "presence",
          "typing",
        ],
      },
    ],
  },
  isLoading: false,
  isFetching: false,
  hasNextPage: false,
  isFetchingNextPage: false,
  fetchNextPage: vi.fn(),
  error: null,
};
const send = { error: null, isPending: false, mutate: vi.fn(), reset: vi.fn() };

vi.mock("@/lib/queries/teams", () => ({
  useTeamMembers: () => membersQuery,
}));

vi.mock("@/lib/queries/privateMessages", () => ({
  usePrivateMessages: () => messagesQuery,
  useSendPrivateMessage: () => send,
  useEditPrivateMessage: () => ({ error: null, isPending: false, mutate: vi.fn(), reset: vi.fn() }),
  useTogglePrivateMessageReaction: () => ({ isPending: false, mutate: vi.fn() }),
  useMarkPrivateMessageRead: () => ({ mutate: vi.fn() }),
  useUnreadPrivateMessages: () => ({ data: { unread_peer_ids: [] } }),
  downloadPrivateMessageAttachment: vi.fn(),
}));

vi.mock("@/lib/queries/incidents", () => ({
  useAvailableReactions: () => ({ data: ["👍", "✅"] }),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAuthStore.getState().logout();
  membersQuery = { data: [peer], isLoading: false, error: null };
  messagesQuery = { ...messagesQuery, isLoading: false, isFetching: false, error: null };
});

describe("DirectMessageRoomPage", () => {
  it("renders a routed, full conversation with left and right speakers", () => {
    useAuthStore.getState().setUser({ id: "me-1", email: "me@example.com", locale: "en" });
    render(<DirectMessageRoomPage teamId="team-1" peerId="peer-1" />);

    expect(screen.getByRole("heading", { name: "peer@example.com" })).toHaveClass("sr-only");
    expect(screen.getByRole("region", { name: "peer@example.com" })).toBeVisible();
    expect(screen.getByText("My update").closest("li")).toHaveAttribute(
      "data-direct-message-owner",
      "current",
    );
    expect(screen.getByText("My update").parentElement).toHaveClass("bg-gold");
    expect(screen.getByText("Their update").closest("li")).toHaveAttribute(
      "data-direct-message-owner",
      "peer",
    );
    expect(screen.getByRole("img", { name: "gifAlt" })).toHaveAttribute(
      "src",
      "https://media.giphy.com/media/abc/giphy.gif",
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("sends trimmed content to the routed peer", async () => {
    useAuthStore.getState().setUser({ id: "me-1", locale: "en" });
    render(<DirectMessageRoomPage teamId="team-1" peerId="peer-1" />);

    fireEvent.change(screen.getByPlaceholderText("messagePlaceholder"), {
      target: { value: "  Status checked  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "send" }));

    await waitFor(() =>
      expect(send.mutate).toHaveBeenCalledWith(
        { recipientId: "peer-1", content: "Status checked", attachments: [] },
        expect.objectContaining({ onSuccess: expect.any(Function) }),
      ),
    );
  });

  it("can fully retract and restore room navigation", () => {
    useAuthStore.getState().setUser({ id: "me-1", locale: "en" });
    render(<DirectMessageRoomPage teamId="team-1" peerId="peer-1" />);

    fireEvent.click(screen.getByRole("button", { name: "collapseRooms" }));
    expect(document.querySelector('[data-rooms-rail-open="false"]')).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "expandRooms" }));
    expect(document.querySelector('[data-rooms-rail-open="true"]')).toBeInTheDocument();
  });

  it("loads earlier messages without replacing the current conversation", async () => {
    useAuthStore.getState().setUser({ id: "me-1", locale: "en" });
    messagesQuery = { ...messagesQuery, hasNextPage: true };
    render(<DirectMessageRoomPage teamId="team-1" peerId="peer-1" />);

    const transcript = document.querySelector(
      '[data-direct-message-transcript="true"]',
    ) as HTMLDivElement;
    Object.defineProperty(transcript, "scrollTop", { value: 0, writable: true });
    fireEvent.scroll(transcript);

    await waitFor(() => expect(messagesQuery.fetchNextPage).toHaveBeenCalledOnce());
    expect(screen.queryByRole("button", { name: "loadEarlier" })).not.toBeInTheDocument();
    expect(screen.getByText("My update")).toBeVisible();
  });

  it("rejects a peer outside the current Team", () => {
    render(<DirectMessageRoomPage teamId="team-1" peerId="unknown" />);
    expect(screen.getByRole("alert")).toHaveTextContent("loadFailed");
  });

  it("preserves the conversation layout while Team members load", () => {
    membersQuery = { data: [], isLoading: true, error: null };
    render(<DirectMessageRoomPage teamId="team-1" peerId="peer-1" />);

    expect(screen.getByTestId("conversation-room-skeleton")).toBeInTheDocument();
  });
});
