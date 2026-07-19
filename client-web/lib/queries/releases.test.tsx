import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import type { Release, ReleaseListItem } from "./releases";
import {
  useCancelRelease,
  useCreateRelease,
  useLinkIncident,
  useRelease,
  useReleases,
  useUnlinkIncident,
  useValidateStep,
} from "./releases";

vi.mock("../api", () => ({
  apiFetch: vi.fn(),
}));

const mockedApiFetch = vi.mocked(apiFetch);

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function release(overrides: Partial<Release> = {}): Release {
  return {
    release_id: "release-1",
    team_id: "team-1",
    title: "Deploy demo",
    state: "created",
    steps: [
      { position: 0, name: "build", validated: false, validated_by: null, validated_at: null },
    ],
    linked_incident_ids: [],
    created_at: "2026-06-26T00:00:00Z",
    updated_at: "2026-06-26T00:00:00Z",
    ...overrides,
  };
}

function releaseListItem(overrides: Partial<ReleaseListItem> = {}): ReleaseListItem {
  return {
    release_id: "release-1",
    team_id: "team-1",
    title: "Deploy demo",
    state: "created",
    progress: { completed: 0, total: 1 },
    next_step: { position: 0, name: "build" },
    blockers: [],
    linked_incident_ids: [],
    created_at: "2026-06-26T00:00:00Z",
    updated_at: "2026-06-26T00:00:00Z",
    ...overrides,
  };
}

async function expectMutationInvalidates<TVariables>(
  useHook: () => { mutateAsync: (variables: TVariables) => Promise<unknown> },
  variables: TVariables,
  url: string,
  method: string,
) {
  const queryClient = createTestQueryClient();
  const invalidate = vi.spyOn(queryClient, "invalidateQueries");
  mockedApiFetch.mockResolvedValueOnce(jsonResponse(release({ release_id: "release-1" })));

  const { result } = renderHook(() => useHook(), {
    wrapper: queryClientWrapper(queryClient),
  });

  await act(async () => {
    await result.current.mutateAsync(variables);
  });

  expect(mockedApiFetch).toHaveBeenCalledWith(url, { method });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["releases", { teamId: "team-1" }] });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["release", "release-1"] });
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("release query mutations", () => {
  it("loads the team's releases", async () => {
    const queryClient = createTestQueryClient();
    const releases = [releaseListItem({ release_id: "release-a" })];
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(releases));

    const { result } = renderHook(() => useReleases("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/releases?team_id=team-1");
    expect(result.current.data).toEqual(releases);
  });

  it("loads a single release detail", async () => {
    const queryClient = createTestQueryClient();
    const detail = release({ release_id: "release-detail" });
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(detail));

    const { result } = renderHook(() => useRelease("release-detail"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/releases/release-detail");
    expect(result.current.data).toEqual(detail);
  });

  it("stores created detail and refreshes the list read model", async () => {
    const queryClient = createTestQueryClient();
    const created = release({ release_id: "release-new", title: "New deploy" });
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(created, 201));

    const { result } = renderHook(() => useCreateRelease(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        team_id: "team-1",
        title: "New deploy",
        steps: ["build"],
      });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/releases", {
      method: "POST",
      body: JSON.stringify({ team_id: "team-1", title: "New deploy", steps: ["build"] }),
    });
    expect(queryClient.getQueryData<Release>(["release", "release-new"])).toEqual(created);
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["releases", { teamId: "team-1" }] });
  });

  it("surfaces backend error codes on release actions", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ code: "release_blocked" }, 409));

    const { result } = renderHook(() => useValidateStep(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await expect(
      result.current.mutateAsync({
        releaseId: "release-1",
        step: "production",
        teamId: "team-1",
      }),
    ).rejects.toThrow("release_blocked");
  });

  it("invalidates list and detail caches after validating a step", async () => {
    await expectMutationInvalidates(
      () => useValidateStep(),
      { releaseId: "release-1", step: "build", teamId: "team-1" },
      "/api/releases/release-1/steps/build/validate",
      "POST",
    );
  });

  it("invalidates list and detail caches after linking an incident", async () => {
    await expectMutationInvalidates(
      () => useLinkIncident(),
      { releaseId: "release-1", incidentId: "incident-1", teamId: "team-1" },
      "/api/releases/release-1/incidents/incident-1/link",
      "POST",
    );
  });

  it("invalidates list and detail caches after unlinking an incident", async () => {
    await expectMutationInvalidates(
      () => useUnlinkIncident(),
      { releaseId: "release-1", incidentId: "incident-1", teamId: "team-1" },
      "/api/releases/release-1/incidents/incident-1/link",
      "DELETE",
    );
  });

  it("invalidates list and detail caches after cancelling a release", async () => {
    await expectMutationInvalidates(
      () => useCancelRelease(),
      { releaseId: "release-1", teamId: "team-1" },
      "/api/releases/release-1/cancel",
      "POST",
    );
  });
});
