import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { IdentityHeader, SettingsRow, SettingsSection } from "./SettingsPrimitives";

describe("settings primitives", () => {
  it("keeps identity and settings rows flat and semantic", () => {
    render(
      <>
        <IdentityHeader mark="OW" title="OpsWarden" subtitle="Manager" />
        <SettingsSection title="Profile">
          <SettingsRow label="Email" action={<button type="button">Copy</button>}>
            manager@example.com
          </SettingsRow>
        </SettingsSection>
      </>,
    );

    expect(screen.getByRole("heading", { name: "OpsWarden" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Profile" })).toBeInTheDocument();
    expect(screen.getByText("manager@example.com")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy" })).toBeInTheDocument();
  });
});
