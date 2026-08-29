import { afterEach, describe, expect, it } from "vitest";
import {
  clearOnboardingDraft,
  ONBOARDING_DRAFT_STORAGE_KEY,
  persistOnboardingDraft,
  readOnboardingDraft,
} from "./onboardingDraft";

afterEach(() => {
  window.sessionStorage.clear();
});

describe("onboarding draft", () => {
  it("persists resumable fields without the password or current step", () => {
    persistOnboardingDraft({
      email: "operator@example.com",
      password: "correct-horse",
      mode: "join",
      teamName: "Platform",
      invitationCode: "OPS-ABC123",
    });

    const raw = window.sessionStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw ?? "{}")).toEqual({
      data: {
        email: "operator@example.com",
        mode: "join",
        teamName: "Platform",
        invitationCode: "OPS-ABC123",
      },
    });
    expect(raw).not.toContain("correct-horse");
    expect(raw).not.toContain("password");
    expect(raw).not.toContain("step");
  });

  it("purges a legacy password while restoring only non-sensitive fields", () => {
    window.sessionStorage.setItem(
      ONBOARDING_DRAFT_STORAGE_KEY,
      JSON.stringify({
        step: 3,
        data: {
          email: "operator@example.com",
          password: "legacy-secret",
          mode: "create",
          teamName: "Operations",
          invitationCode: "OPS-OLD123",
          injected: "ignored",
        },
      }),
    );

    expect(readOnboardingDraft()).toEqual({
      email: "operator@example.com",
      password: "",
      mode: "create",
      teamName: "Operations",
      invitationCode: "OPS-OLD123",
    });

    const rewritten = window.sessionStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY) ?? "";
    expect(rewritten).not.toContain("legacy-secret");
    expect(rewritten).not.toContain("password");
    expect(rewritten).not.toContain("injected");
    expect(rewritten).not.toContain("step");
  });

  it("removes malformed drafts and supports explicit cleanup", () => {
    window.sessionStorage.setItem(ONBOARDING_DRAFT_STORAGE_KEY, "not-json");
    expect(readOnboardingDraft()).toMatchObject({ password: "", mode: "create" });
    expect(window.sessionStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY)).toBeNull();

    persistOnboardingDraft({ email: "a@b.c", password: "secret" });
    clearOnboardingDraft();
    expect(window.sessionStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY)).toBeNull();
  });
});
