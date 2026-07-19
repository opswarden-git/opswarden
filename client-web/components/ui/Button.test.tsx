import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Button, IconButton } from "./Button";

afterEach(cleanup);

describe("Button", () => {
  it("defaults to a safe non-submit secondary button", () => {
    render(<Button>Cancel</Button>);

    const button = screen.getByRole("button", { name: "Cancel" });
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveClass("bg-panel", "border-border", "h-9");
  });

  it("forwards clicks and supports explicit variants and sizes", () => {
    const onClick = vi.fn();
    render(
      <Button variant="danger" size="lg" onClick={onClick}>
        Delete
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Delete" });
    fireEvent.click(button);

    expect(onClick).toHaveBeenCalledOnce();
    expect(button).toHaveClass("bg-danger", "text-danger-ink", "h-10");
  });

  it("disables interaction and exposes its busy state while loading", () => {
    const onClick = vi.fn();
    render(
      <Button loading onClick={onClick}>
        Saving
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Saving" });
    fireEvent.click(button);

    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("IconButton", () => {
  it("uses its label as an accessible name and keeps square geometry", () => {
    render(
      <IconButton label="Close" size="sm">
        <span aria-hidden="true">x</span>
      </IconButton>,
    );

    expect(screen.getByRole("button", { name: "Close" })).toHaveClass("h-8", "w-8", "p-0");
  });

  it("supports an orthogonal danger tone without creating another variant", () => {
    render(
      <IconButton label="Remove link" size="sm" variant="ghost" tone="danger">
        <span aria-hidden="true">x</span>
      </IconButton>,
    );

    expect(screen.getByRole("button", { name: "Remove link" })).toHaveClass(
      "text-sev-critical",
      "hover:bg-sev-critical/10",
    );
  });
});
