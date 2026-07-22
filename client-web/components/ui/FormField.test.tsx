import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FormField } from "./FormField";

afterEach(cleanup);

describe("FormField", () => {
  it("associates label with control via htmlFor/id", () => {
    render(
      <FormField label="Email">
        <input type="email" />
      </FormField>,
    );
    const input = screen.getByLabelText("Email");
    expect(input).toBeDefined();
    expect(input.tagName).toBe("INPUT");
  });

  it("renders caption and links it via aria-describedby", () => {
    render(
      <FormField label="Secret" caption="Keep it safe">
        <input type="password" />
      </FormField>,
    );
    const input = screen.getByLabelText("Secret");
    const captionId = input.getAttribute("aria-describedby");
    expect(captionId).toBeTruthy();

    const caption = document.getElementById(captionId!);
    expect(caption).toBeTruthy();
    expect(caption!.textContent).toBe("Keep it safe");
  });

  it("renders error, hides caption and sets aria-invalid", () => {
    render(
      <FormField label="Name" caption="Your full name" error="Required">
        <input type="text" />
      </FormField>,
    );
    const input = screen.getByLabelText(/Name/);
    expect(input.getAttribute("aria-invalid")).toBe("true");

    // Error is visible
    expect(screen.getByRole("alert").textContent).toBe("Required");

    // Caption is hidden when error is shown
    expect(screen.queryByText("Your full name")).toBeNull();

    // aria-describedby points to error, not caption
    const describedBy = input.getAttribute("aria-describedby")!;
    const errorEl = document.getElementById(describedBy);
    expect(errorEl).toBeTruthy();
    expect(errorEl!.textContent).toBe("Required");
  });

  it("does not set aria-invalid when there is no error", () => {
    render(
      <FormField label="Name">
        <input type="text" />
      </FormField>,
    );
    expect(screen.getByLabelText("Name").getAttribute("aria-invalid")).toBeNull();
  });

  it("shows required indicator and sets aria-required", () => {
    render(
      <FormField label="Password" required>
        <input type="password" />
      </FormField>,
    );
    const input = screen.getByLabelText(/Password/);
    expect(input.getAttribute("aria-required")).toBe("true");
    expect(screen.getByText("*")).toBeTruthy();
  });

  it("does not set aria-required when not required", () => {
    render(
      <FormField label="Notes">
        <textarea />
      </FormField>,
    );
    expect(screen.getByLabelText("Notes").getAttribute("aria-required")).toBeNull();
  });
});
