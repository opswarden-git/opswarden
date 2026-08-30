import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { apiFetch } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { AccountDangerZone } from "./AccountDangerZone";
import { LanguagePanel } from "./LanguagePanel";
import { ProfilePanel } from "./ProfilePanel";

const push = vi.fn();
const nativeReplace = vi.fn();
vi.mock("next/navigation", () => ({
  useParams: () => ({ locale: "en" }),
  useRouter: () => ({ push, replace: nativeReplace }),
}));

const intlReplace = vi.fn();
vi.mock("@/i18n/routing", () => ({
  useRouter: () => ({ replace: intlReplace }),
  usePathname: () => "/settings",
}));

vi.mock("next/image", () => ({
  default: ({ alt, src }: { alt: string; src: string }) => createElement("img", { alt, src }),
}));

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

const updateLocale = { isPending: false, isError: false, mutate: vi.fn() };
vi.mock("@/lib/queries/profile", () => ({ useUpdateLocale: () => updateLocale }));
vi.mock("@/lib/api", () => ({ apiFetch: vi.fn() }));
const mockedApiFetch = vi.mocked(apiFetch);

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAuthStore.getState().logout();
});

describe("settings panels", () => {
  it("persists a language change before replacing the localized route", () => {
    render(<LanguagePanel />);
    const english = screen.getByRole("button", { name: "english" });
    expect(english).toHaveAttribute("aria-pressed", "true");
    expect(english).toHaveClass("text-gold");
    expect(english).not.toHaveClass("border");
    expect(english).not.toHaveClass("bg-gold/10");
    fireEvent.click(screen.getByRole("button", { name: "french" }));
    expect(updateLocale.mutate).toHaveBeenCalledWith(
      "fr",
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
    const options = updateLocale.mutate.mock.calls[0][1];
    options.onSuccess();
    expect(intlReplace).toHaveBeenCalledWith("/settings", { locale: "fr" });
  });

  it("displays persisted identity", () => {
    useAuthStore.getState().setUser({
      id: "user-1",
      email: "operator@example.com",
      locale: "en",
      created_at: "2026-05-28T10:00:00Z",
    });
    render(<ProfilePanel />);
    expect(screen.getByText("operator@example.com")).toBeInTheDocument();
    expect(screen.getByText("user-1")).toBeInTheDocument();
    expect(screen.getByText("memberSince")).toBeInTheDocument();
  });

  it("logs out through the server and clears the local session", async () => {
    useAuthStore.getState().setToken("jwt-token");
    useAuthStore.getState().setUser({ id: "user-1", email: "operator@example.com", locale: "en" });
    mockedApiFetch.mockResolvedValueOnce(new Response(null, { status: 204 }));
    render(<AccountDangerZone />);

    fireEvent.click(screen.getByRole("button", { name: "logOut" }));
    await waitFor(() =>
      expect(mockedApiFetch).toHaveBeenCalledWith("/api/auth/logout", { method: "POST" }),
    );
    expect(useAuthStore.getState().token).toBeNull();
    expect(push).toHaveBeenCalledWith("/en/login");
  });

  it("requires typed confirmation and deletes the account", async () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "operator@example.com", locale: "en" });
    mockedApiFetch.mockResolvedValueOnce(new Response(null, { status: 204 }));
    render(<AccountDangerZone />);

    fireEvent.click(screen.getByRole("button", { name: "deleteAccount" }));
    const confirmation = screen.getByRole("textbox", { name: "DELETE" });
    fireEvent.change(confirmation, { target: { value: "DELETE" } });
    fireEvent.click(screen.getByRole("button", { name: "deleteAccount" }));

    await waitFor(() =>
      expect(mockedApiFetch).toHaveBeenCalledWith("/api/me", { method: "DELETE" }),
    );
    expect(push).toHaveBeenCalledWith("/en/signup");
  });
});
