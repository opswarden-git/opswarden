import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { OnboardingData } from "./types";
import { StepCredentials } from "./StepCredentials";
import { StepTeam } from "./StepTeam";
import { StepVerification } from "./StepVerification";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));
vi.mock("next/image", () => ({
  default: ({ alt, src }: { alt: string; src: string }) => createElement("img", { alt, src }),
}));
vi.mock("@/i18n/routing", () => ({ useRouter: () => ({ push: vi.fn() }) }));

const data: OnboardingData = {
  email: "operator@example.com",
  password: "password",
  mode: "create",
  teamName: "Operations",
  invitationCode: "OPS-KXP9DX",
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("onboarding steps", () => {
  it("validates credentials and forwards field updates", () => {
    const updateData = vi.fn();
    const next = vi.fn();
    const { rerender } = render(
      <StepCredentials
        data={{ ...data, email: "", password: "" }}
        updateData={updateData}
        next={next}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "signup" }));
    expect(screen.getByText("required")).toBeInTheDocument();
    expect(screen.getByText("passwordMin")).toBeInTheDocument();
    expect(next).not.toHaveBeenCalled();

    rerender(<StepCredentials data={data} updateData={updateData} next={next} />);
    expect(screen.getByRole("button", { name: "signupWithGithub" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "showPassword" }));
    expect(screen.getByDisplayValue("password")).toHaveAttribute("type", "text");
    fireEvent.click(screen.getByRole("button", { name: "signup" }));
    expect(next).toHaveBeenCalledOnce();
  });

  it("updates team metadata and supports create vs join mode navigation", () => {
    const updateData = vi.fn();
    const next = vi.fn();
    const back = vi.fn();
    const { rerender } = render(
      <StepTeam data={data} updateData={updateData} next={next} back={back} />,
    );

    fireEvent.change(screen.getByRole("textbox", { name: "teamName" }), {
      target: { value: "Platform" },
    });
    expect(updateData).toHaveBeenLastCalledWith({ teamName: "Platform" });
    fireEvent.click(screen.getByRole("button", { name: "modeJoin" }));
    expect(updateData).toHaveBeenLastCalledWith({ mode: "join" });

    rerender(
      <StepTeam
        data={{ ...data, mode: "join", invitationCode: "" }}
        updateData={updateData}
        next={next}
        back={back}
      />,
    );
    fireEvent.change(screen.getByRole("textbox", { name: "invitationCode" }), {
      target: { value: "OPS-NEW123" },
    });
    expect(updateData).toHaveBeenLastCalledWith({ invitationCode: "OPS-NEW123" });

    rerender(
      <StepTeam
        data={{ ...data, mode: "join", invitationCode: "OPS-NEW123" }}
        updateData={updateData}
        next={next}
        back={back}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    fireEvent.click(screen.getByRole("button", { name: "next" }));
    expect(back).toHaveBeenCalledOnce();
    expect(next).toHaveBeenCalledOnce();
  });

  it("surfaces a stable signup failure in the verification console", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 500 })));
    const back = vi.fn();
    render(<StepVerification data={data} back={back} />);
    expect(screen.getByText("creatingWorkspace")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("signup_failed")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    expect(back).toHaveBeenCalledOnce();
  });
});
