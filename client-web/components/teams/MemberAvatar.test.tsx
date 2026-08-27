import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemberAvatar, memberDisplayName } from "./MemberAvatar";

describe("MemberAvatar", () => {
  it("uses the yellow RBAC role icon instead of email initials", () => {
    const { container, rerender } = render(
      <MemberAvatar email="manager@opswarden.local" role="manager" />,
    );
    expect(container.querySelector("svg")).toHaveClass("text-gold", "h-2/3", "w-2/3");
    expect(screen.queryByText("MA")).not.toBeInTheDocument();

    rerender(<MemberAvatar email="responder@opswarden.local" role="responder" />);
    expect(container.querySelector("svg")).toHaveClass("text-gold");

    rerender(<MemberAvatar email="observer@opswarden.local" role="observer" />);
    expect(container.querySelector("svg")).toHaveClass("text-gold");
  });

  it("turns email local parts into a readable display name", () => {
    expect(memberDisplayName("romeo.cavazza@opswarden.local")).toBe("Romeo Cavazza");
    expect(memberDisplayName("manager@opswarden.local")).toBe("Manager");
  });
});
