import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AuthGuard } from "./AuthGuard";

const mocks = vi.hoisted(() => ({
  pathname: vi.fn(),
  replace: vi.fn(),
  authState: vi.fn(),
  params: vi.fn(),
  profile: vi.fn(),
  setUser: vi.fn(),
}));

vi.mock("@/i18n/routing", () => ({
  usePathname: mocks.pathname,
  useRouter: () => ({ replace: mocks.replace }),
}));

vi.mock("@/store/auth", () => ({
  useAuthStore: mocks.authState,
}));

vi.mock("next/navigation", () => ({
  useParams: mocks.params,
}));

vi.mock("@/lib/queries/profile", () => ({
  useProfile: mocks.profile,
}));

function renderGuard() {
  return render(
    <AuthGuard>
      <div>Private content</div>
    </AuthGuard>,
  );
}

beforeEach(() => {
  mocks.pathname.mockReturnValue("/teams/team-1/incidents");
  mocks.params.mockReturnValue({ locale: "en" });
  mocks.profile.mockReturnValue({ data: undefined });
  mocks.authState.mockReturnValue({
    token: null,
    hasHydrated: true,
    setUser: mocks.setUser,
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("AuthGuard", () => {
  it("renders nothing before the persisted session has hydrated", () => {
    mocks.authState.mockReturnValue({
      token: null,
      hasHydrated: false,
      setUser: mocks.setUser,
    });

    renderGuard();

    expect(screen.queryByText("Private content")).not.toBeInTheDocument();
    expect(mocks.replace).not.toHaveBeenCalled();
  });

  it("redirects unauthenticated private routes without rendering their content", async () => {
    renderGuard();

    expect(screen.queryByText("Private content")).not.toBeInTheDocument();
    await waitFor(() => expect(mocks.replace).toHaveBeenCalledWith("/login"));
  });

  it("allows the exact login route", () => {
    mocks.pathname.mockReturnValue("/login");

    renderGuard();

    expect(screen.getByText("Private content")).toBeVisible();
    expect(mocks.replace).not.toHaveBeenCalled();
  });

  it("does not mistake a private path containing login for an auth route", async () => {
    mocks.pathname.mockReturnValue("/teams/login-history");

    renderGuard();

    expect(screen.queryByText("Private content")).not.toBeInTheDocument();
    await waitFor(() => expect(mocks.replace).toHaveBeenCalledWith("/login"));
  });

  it("renders private routes for authenticated users", () => {
    mocks.authState.mockReturnValue({
      token: "token",
      hasHydrated: true,
      setUser: mocks.setUser,
    });

    renderGuard();

    expect(screen.getByText("Private content")).toBeVisible();
    expect(mocks.replace).not.toHaveBeenCalled();
  });

  it("restores the persisted server locale on authenticated routes", async () => {
    const profile = { id: "me", email: "me@example.com", locale: "fr" };
    mocks.authState.mockReturnValue({
      token: "token",
      hasHydrated: true,
      setUser: mocks.setUser,
    });
    mocks.profile.mockReturnValue({ data: profile });

    renderGuard();

    await waitFor(() => {
      expect(mocks.setUser).toHaveBeenCalledWith(profile);
      expect(mocks.replace).toHaveBeenCalledWith("/teams/team-1/incidents", {
        locale: "fr",
      });
    });
  });
});
