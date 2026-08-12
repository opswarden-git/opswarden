import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { OnboardingData } from "./types";
import { StepCredentials } from "./StepCredentials";
import { StepStation } from "./StepStation";
import { StepVerification } from "./StepVerification";

vi.mock("next-intl", () => ({
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
  stationName: "Operations",
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
    fireEvent.click(screen.getByRole("button", { name: "showPassword" }));
    expect(screen.getByDisplayValue("password")).toHaveAttribute("type", "text");
    fireEvent.click(screen.getByRole("button", { name: "signup" }));
    expect(next).toHaveBeenCalledOnce();
  });

  it("updates station metadata and supports back/next navigation", () => {
    const updateData = vi.fn();
    const next = vi.fn();
    const back = vi.fn();
    render(<StepStation data={data} updateData={updateData} next={next} back={back} />);

    fireEvent.change(screen.getByRole("textbox", { name: "organization" }), {
      target: { value: "Platform" },
    });
    expect(updateData).toHaveBeenCalledWith({ stationName: "Platform" });
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    fireEvent.click(screen.getByRole("button", { name: "next" }));
    expect(back).toHaveBeenCalledOnce();
    expect(next).toHaveBeenCalledOnce();
  });

  it("surfaces a stable signup failure in the verification console", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 409 })));
    const back = vi.fn();
    render(<StepVerification data={data} back={back} />);
    expect(screen.getByText("creatingWorkspace")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("signup_failed")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    expect(back).toHaveBeenCalledOnce();
  });
});
