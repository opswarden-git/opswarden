import { afterEach, describe, expect, it, vi } from "vitest";
import { apiFetch } from "@/lib/api";
import { establishSession } from "@/lib/sessionLifecycle";
import type { OnboardingData } from "./types";
import { completeOnboarding, completeTeamOnboarding } from "./onboardingFlow";

vi.mock("@/lib/api", () => ({ apiFetch: vi.fn() }));
vi.mock("@/lib/sessionLifecycle", () => ({ establishSession: vi.fn() }));

const mockedApiFetch = vi.mocked(apiFetch);
const mockedEstablishSession = vi.mocked(establishSession);
const data: OnboardingData = {
  email: "operator@example.com",
  password: "correct-password",
  mode: "create",
  teamName: "Operations",
};
const user = { id: "user-1", email: data.email, locale: "en" as const };

function authResponses(signupStatus = 201) {
  const fetchMock = vi
    .fn()
    .mockResolvedValueOnce(new Response(null, { status: signupStatus }))
    .mockResolvedValueOnce(Response.json({ token: "new-token" }));
  vi.stubGlobal("fetch", fetchMock);
  mockedEstablishSession.mockResolvedValue(user);
  return fetchMock;
}

afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe("completeOnboarding", () => {
  it("completes team setup without replaying authentication for an existing session", async () => {
    vi.stubGlobal("fetch", vi.fn());
    mockedApiFetch
      .mockResolvedValueOnce(Response.json([]))
      .mockResolvedValueOnce(Response.json({ team_id: "team-1" }, { status: 201 }));

    await expect(completeTeamOnboarding(data)).resolves.toBe("team-1");

    expect(fetch).not.toHaveBeenCalled();
    expect(mockedEstablishSession).not.toHaveBeenCalled();
    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams");
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams", {
      method: "POST",
      body: JSON.stringify({ name: "Operations" }),
    });
  });

  it("creates the account, establishes its session and creates its first team", async () => {
    authResponses();
    mockedApiFetch
      .mockResolvedValueOnce(Response.json([]))
      .mockResolvedValueOnce(Response.json({ team_id: "team-1" }, { status: 201 }));

    await expect(completeOnboarding(data)).resolves.toEqual({ user, teamId: "team-1" });

    expect(mockedEstablishSession).toHaveBeenCalledWith("new-token");
    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams");
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams", {
      method: "POST",
      body: JSON.stringify({ name: "Operations" }),
    });
  });

  it("resumes after account creation and does not replay a completed team mutation", async () => {
    authResponses(409);
    mockedApiFetch.mockResolvedValueOnce(Response.json([{ team_id: "persisted-team" }]));

    await expect(completeOnboarding(data)).resolves.toEqual({
      user,
      teamId: "persisted-team",
    });

    expect(mockedApiFetch).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["signup", 500, "signup_failed"],
    ["signin", 401, "signin_after_signup_failed"],
  ])("reports a stable %s boundary failure", async (boundary, status, error) => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(null, { status: boundary === "signup" ? status : 201 }));
    if (boundary === "signin") {
      fetchMock.mockResolvedValueOnce(new Response(null, { status }));
    }
    vi.stubGlobal("fetch", fetchMock);

    await expect(completeOnboarding(data)).rejects.toThrow(error);
  });

  it("stops at profile validation without attempting workspace changes", async () => {
    authResponses();
    mockedEstablishSession.mockRejectedValueOnce(new Error("profile_load_failed"));

    await expect(completeOnboarding(data)).rejects.toThrow("profile_load_failed");
    expect(mockedApiFetch).not.toHaveBeenCalled();
  });

  it("distinguishes workspace recovery, creation and join failures", async () => {
    authResponses();
    mockedApiFetch.mockResolvedValueOnce(new Response(null, { status: 503 }));
    await expect(completeOnboarding(data)).rejects.toThrow("workspace_resume_failed");

    vi.clearAllMocks();
    authResponses(409);
    mockedApiFetch
      .mockResolvedValueOnce(Response.json([]))
      .mockResolvedValueOnce(new Response(null, { status: 500 }));
    await expect(completeOnboarding(data)).rejects.toThrow("create_team_failed");

    vi.clearAllMocks();
    authResponses(409);
    mockedApiFetch
      .mockResolvedValueOnce(Response.json([]))
      .mockResolvedValueOnce(new Response(null, { status: 404 }));
    await expect(completeOnboarding({ ...data, mode: "join" })).rejects.toThrow("join_team_failed");
  });
});
