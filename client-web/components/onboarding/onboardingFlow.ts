import { apiFetch } from "@/lib/api";
import { establishSession } from "@/lib/sessionLifecycle";
import type { User } from "@/store/auth";
import type { OnboardingData } from "./types";

interface OnboardingResult {
  user: User;
  teamId: string;
}

async function signUpOrResume(data: OnboardingData) {
  const response = await fetch("/api/auth/sign-up", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: data.email, password: data.password }),
  });

  // A previous attempt may have created the account before failing later.
  if (!response.ok && response.status !== 409) throw new Error("signup_failed");
}

async function signIn(data: OnboardingData) {
  const response = await fetch("/api/auth/sign-in", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: data.email, password: data.password }),
  });
  if (!response.ok) throw new Error("signin_after_signup_failed");

  const body = (await response.json().catch(() => null)) as { token?: string } | null;
  if (!body?.token) throw new Error("signin_after_signup_failed");
  return body.token;
}

async function existingTeamId() {
  const response = await apiFetch("/api/teams");
  if (!response.ok) throw new Error("workspace_resume_failed");
  const teams = (await response.json().catch(() => null)) as Array<{ team_id?: string }> | null;
  if (!teams) throw new Error("workspace_resume_failed");
  return teams.find((team) => team.team_id)?.team_id ?? "";
}

async function createOrJoinTeam(data: OnboardingData) {
  const mode = data.mode ?? "create";
  const response =
    mode === "join"
      ? await apiFetch("/api/teams/join", {
          method: "POST",
          body: JSON.stringify({ invitation_code: data.invitationCode }),
        })
      : await apiFetch("/api/teams", {
          method: "POST",
          body: JSON.stringify({ name: data.teamName }),
        });

  if (!response.ok) {
    throw new Error(mode === "join" ? "join_team_failed" : "create_team_failed");
  }

  const body = (await response.json().catch(() => null)) as { team_id?: string } | null;
  if (!body?.team_id) {
    throw new Error(mode === "join" ? "join_team_failed" : "create_team_failed");
  }
  return body.team_id;
}

/**
 * Completes signup as a resumable workflow. Every retry authenticates the same
 * credentials and checks persisted membership before replaying create/join.
 */
export async function completeOnboarding(data: OnboardingData): Promise<OnboardingResult> {
  await signUpOrResume(data);
  const token = await signIn(data);
  const user = await establishSession(token);
  const resumedTeamId = await existingTeamId();
  const teamId = resumedTeamId || (await createOrJoinTeam(data));
  return { user, teamId };
}
