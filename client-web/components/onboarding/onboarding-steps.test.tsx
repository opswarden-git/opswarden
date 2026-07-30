import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { OnboardingData } from "./types";
import { StepCredentials } from "./StepCredentials";
import { StepIntegrations } from "./StepIntegrations";
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
  operatorName: "Operator",
  email: "operator@example.com",
  password: "password",
  stationName: "Operations",
  timezone: "Europe/Paris",
  clearance: "",
  integrations: [],
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
        data={{ ...data, operatorName: "", email: "", password: "" }}
        updateData={updateData}
        next={next}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "signup" }));
    expect(screen.getAllByText("required")).toHaveLength(2);
    expect(screen.getByText("passwordMin")).toBeInTheDocument();
    expect(next).not.toHaveBeenCalled();

    rerender(<StepCredentials data={data} updateData={updateData} next={next} />);
    fireEvent.change(screen.getByRole("textbox", { name: /operatorName/ }), {
      target: { value: "Incident Commander" },
    });
    expect(updateData).toHaveBeenCalledWith({ operatorName: "Incident Commander" });
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
    fireEvent.change(screen.getByRole("combobox", { name: "timezone" }), {
      target: { value: "Asia/Tokyo" },
    });
    expect(updateData).toHaveBeenCalledWith({ stationName: "Platform" });
    expect(updateData).toHaveBeenCalledWith({ timezone: "Asia/Tokyo" });
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    fireEvent.click(screen.getByRole("button", { name: "next" }));
    expect(back).toHaveBeenCalledOnce();
    expect(next).toHaveBeenCalledOnce();
  });

  it("renders the integration catalog and allows deferring configuration", () => {
    const next = vi.fn();
    const back = vi.fn();
    render(<StepIntegrations data={data} updateData={vi.fn()} next={next} back={back} />);
    for (const integration of [
      "GitHub",
      "GitLab",
      "Alertmanager",
      "Generic Webhook",
      "HTTP Request",
      "Email (SMTP)",
    ]) {
      expect(screen.getByText(integration)).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("button", { name: "back" }));
    fireEvent.click(screen.getByRole("button", { name: "skipForNow" }));
    expect(back).toHaveBeenCalledOnce();
    expect(next).toHaveBeenCalledOnce();
  });

  it("surfaces a stable signup failure in the verification console", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 409 })));
    render(<StepVerification data={data} />);
    expect(screen.getByText("bootLoader")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("[ERROR] signup_failed")).toBeInTheDocument());
  });
});
