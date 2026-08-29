import type { OnboardingData } from "./types";

export const ONBOARDING_DRAFT_STORAGE_KEY = "opswarden_onboarding_draft";

const EMPTY_ONBOARDING_DATA: OnboardingData = {
  email: "",
  password: "",
  mode: "create",
  teamName: "",
  invitationCode: "",
};

type PersistedOnboardingData = Omit<OnboardingData, "password">;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function persistedData(data: OnboardingData): PersistedOnboardingData {
  return {
    email: data.email,
    mode: data.mode === "join" ? "join" : "create",
    teamName: data.teamName ?? "",
    invitationCode: data.invitationCode ?? "",
  };
}

/**
 * Restore only resumable, non-sensitive fields. A reload always returns to the
 * credentials step because the password deliberately exists in memory only.
 * Rewriting the draft here also purges passwords left by older versions.
 */
export function readOnboardingDraft(): OnboardingData {
  if (typeof window === "undefined") return { ...EMPTY_ONBOARDING_DATA };

  try {
    const saved = window.sessionStorage.getItem(ONBOARDING_DRAFT_STORAGE_KEY);
    if (!saved) return { ...EMPTY_ONBOARDING_DATA };

    const parsed: unknown = JSON.parse(saved);
    const source = isRecord(parsed) && isRecord(parsed.data) ? parsed.data : {};
    const data: OnboardingData = {
      email: stringValue(source.email),
      password: "",
      mode: source.mode === "join" ? "join" : "create",
      teamName: stringValue(source.teamName),
      invitationCode: stringValue(source.invitationCode),
    };

    persistOnboardingDraft(data);
    return data;
  } catch {
    clearOnboardingDraft();
    return { ...EMPTY_ONBOARDING_DATA };
  }
}

export function persistOnboardingDraft(data: OnboardingData): void {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(
      ONBOARDING_DRAFT_STORAGE_KEY,
      JSON.stringify({ data: persistedData(data) }),
    );
  } catch {
    // Storage is optional; onboarding remains usable in memory.
  }
}

export function clearOnboardingDraft(): void {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.removeItem(ONBOARDING_DRAFT_STORAGE_KEY);
  } catch {
    // Storage cleanup must not interrupt the completed onboarding flow.
  }
}
