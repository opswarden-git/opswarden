import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemberAvatar, memberDisplayName, memberInitials } from "./MemberAvatar";

describe("MemberAvatar", () => {
  it("uses both parts of a separated email local name", () => {
    expect(memberInitials("ada.lovelace@opswarden.local")).toBe("AL");
  });

  it("matches the compact roster avatar for a single-part local name", () => {
    render(<MemberAvatar email="manager@opswarden.local" />);
    expect(screen.getByText("MA")).toBeInTheDocument();
  });

  it("turns email local parts into a readable display name", () => {
    expect(memberDisplayName("romeo.cavazza@opswarden.local")).toBe("Romeo Cavazza");
    expect(memberDisplayName("manager@opswarden.local")).toBe("Manager");
  });
});
