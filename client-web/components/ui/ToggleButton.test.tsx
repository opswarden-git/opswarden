import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CopyButton } from "./CopyButton";
import { ReactionToggle } from "./ReactionToggle";
import { ToggleButton } from "./ToggleButton";
import { NextIntlClientProvider } from "next-intl";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("ToggleButton", () => {
  it("exposes its binary state", () => {
    render(<ToggleButton pressed>GIF</ToggleButton>);

    expect(screen.getByRole("button", { name: "GIF" })).toHaveAttribute("aria-pressed", "true");
  });
});

describe("ReactionToggle", () => {
  it("renders an accessible pressed reaction and its count", () => {
    render(<ReactionToggle emoji="👍" count={3} label="React with thumbs up" pressed />);

    const reaction = screen.getByRole("button", { name: "React with thumbs up" });
    expect(reaction).toHaveAttribute("aria-pressed", "true");
    expect(reaction).toHaveTextContent("👍3");
    expect(reaction).toHaveClass("h-6");
  });
});

describe("CopyButton", () => {
  it("copies its value and exposes success feedback", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <NextIntlClientProvider locale="en" messages={{ Common: { copyFailed: "Copy failed" } }}>
        <CopyButton value="OPS-123" label="Copy code" copiedLabel="Code copied" />
      </NextIntlClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy code" }));

    await waitFor(() => expect(writeText).toHaveBeenCalledWith("OPS-123"));
    expect(screen.getByRole("button", { name: "Code copied" })).toBeInTheDocument();
  });
});
