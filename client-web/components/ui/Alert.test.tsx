import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Alert } from "./Alert";

afterEach(cleanup);

describe("Alert", () => {
  it("renders danger messages as alerts without button styling", () => {
    render(<Alert tone="danger">Could not load incidents</Alert>);

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Could not load incidents");
    expect(alert).toHaveClass("border-feedback-danger/30", "bg-feedback-danger/10");
    expect(alert).not.toHaveClass("bg-action-danger");
  });

  it("renders non-urgent information as a status", () => {
    render(<Alert title="Connected">GitHub is ready</Alert>);

    expect(screen.getByRole("status")).toHaveTextContent("ConnectedGitHub is ready");
  });
});
