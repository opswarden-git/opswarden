import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ConversationTranscript } from "./ConversationTranscript";

afterEach(cleanup);

describe("ConversationTranscript", () => {
  it("anchors a short conversation above the composer edge", () => {
    render(
      <ConversationTranscript
        empty={<p>Empty</p>}
        error={<p>Error</p>}
        getCreatedAt={(item) => item.createdAt}
        getId={(item) => item.id}
        items={[{ id: "message-1", createdAt: "2026-08-27T08:00:00Z" }]}
        loading={<p>Loading</p>}
        locale="en"
        renderItem={(item) => <li>{item.id}</li>}
        surface="incident"
      />,
    );

    expect(screen.getByText("message-1").closest('[data-conversation-content="true"]')).toHaveClass(
      "min-h-full",
      "flex",
      "flex-col",
      "justify-end",
    );
  });
});
