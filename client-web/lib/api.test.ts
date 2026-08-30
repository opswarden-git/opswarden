import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "../store/auth";
import { apiFetch } from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
  useAuthStore.getState().logout();
});

describe("apiFetch", () => {
  it("injects authentication, JSON content type and no-store caching", async () => {
    useAuthStore.getState().setToken("jwt-token");
    const response = new Response(JSON.stringify({ ok: true }));
    const fetchMock = vi.fn().mockResolvedValue(response);
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiFetch("/api/me", { method: "GET" })).resolves.toBe(response);

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    const headers = new Headers(init.headers);
    expect(headers.get("Authorization")).toBe("Bearer jwt-token");
    expect(headers.get("Content-Type")).toBe("application/json");
    expect(init.cache).toBe("no-store");
  });

  it("preserves explicit headers and cache policy without inventing authentication", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await apiFetch("/about.json", {
      cache: "force-cache",
      headers: { "Content-Type": "text/plain", "X-Contract": "catalog" },
    });

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    const headers = new Headers(init.headers);
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("Content-Type")).toBe("text/plain");
    expect(headers.get("X-Contract")).toBe("catalog");
    expect(init.cache).toBe("force-cache");
  });

  it("ends the local session on an unauthorized response", async () => {
    useAuthStore.getState().setToken("expired-token");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 401 })));
    vi.stubGlobal("window", undefined);

    await apiFetch("/api/me");

    expect(useAuthStore.getState().token).toBeNull();
  });
});
