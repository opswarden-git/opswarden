import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ConversationTranscriptSkeleton } from "./ConversationTranscriptSkeleton";

afterEach(cleanup);

describe("ConversationTranscriptSkeleton", () => {
  it("preserves date, alternating speakers and an optional system event", () => {
    render(<ConversationTranscriptSkeleton label="Loading conversation" systemEvents />);

    const skeleton = screen.getByTestId("conversation-transcript-skeleton");
    expect(skeleton.querySelectorAll(".justify-start")).toHaveLength(2);
    expect(skeleton.querySelectorAll(".justify-end")).toHaveLength(1);
    expect(skeleton.querySelector(".justify-center")).toBeInTheDocument();
  });
});
