import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsSection } from "./SettingsSection";

describe("SettingsSection", () => {
  it("renders title, description and content when non-collapsible", () => {
    render(
      <SettingsSection title="Identity" description="Team identity details">
        <div>Content Body</div>
      </SettingsSection>,
    );

    expect(screen.getByRole("heading", { name: "Identity" })).toBeInTheDocument();
    expect(screen.getByText("Team identity details")).toBeInTheDocument();
    expect(screen.getByText("Content Body")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("handles collapsible section toggling", () => {
    render(
      <SettingsSection
        title="Ownership"
        description="Transfer ownership"
        collapsible
        defaultOpen={false}
      >
        <div>Ownership Form</div>
      </SettingsSection>,
    );

    const button = screen.getByRole("button", { name: /Ownership/ });
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("Ownership Form")).not.toBeInTheDocument();

    fireEvent.click(button);

    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Ownership Form")).toBeInTheDocument();

    fireEvent.click(button);

    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("Ownership Form")).not.toBeInTheDocument();
  });

  it("forces section open if hasActiveError or isPending is true", () => {
    const { rerender } = render(
      <SettingsSection
        title="Bans"
        description="Banned members"
        collapsible
        defaultOpen={false}
        hasActiveError
      >
        <div>Banned List</div>
      </SettingsSection>,
    );

    const button = screen.getByRole("button", { name: /Bans/ });
    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Banned List")).toBeInTheDocument();

    rerender(
      <SettingsSection
        title="Bans"
        description="Banned members"
        collapsible
        defaultOpen={false}
        isPending
      >
        <div>Banned List</div>
      </SettingsSection>,
    );

    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Banned List")).toBeInTheDocument();
  });
});
