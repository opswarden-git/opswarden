import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";

afterEach(cleanup);

describe("ConfirmDialog", () => {
  it("names the modal and starts on the least destructive action", async () => {
    render(
      <ConfirmDialog
        open
        title="Delete incident"
        description="This permanently deletes Database outage."
        confirmLabel="Delete incident"
        cancelLabel="Cancel"
        intent="destructive"
        onConfirm={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole("dialog", { name: "Delete incident" })).toBeInTheDocument();
    const cancel = screen.getByRole("button", { name: "Cancel" });
    const confirm = screen.getByRole("button", { name: "Delete incident" });
    await waitFor(() => expect(cancel).toHaveFocus());
    expect(confirm).toHaveClass("bg-action-danger");
    expect(cancel).not.toHaveClass("bg-action-danger");
  });

  it("requires the named confirmation value and closes with Escape", async () => {
    const onClose = vi.fn();
    render(
      <ConfirmDialog
        open
        title="Delete incident"
        description="Permanent action"
        confirmLabel="Delete incident"
        cancelLabel="Cancel"
        intent="destructive"
        requireType="DELETE"
        requireTypeLabel="Type DELETE to confirm"
        onConfirm={vi.fn()}
        onClose={onClose}
      />,
    );

    const confirm = screen.getByRole("button", { name: "Delete incident" });
    expect(confirm).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox", { name: "Type DELETE to confirm" }), {
      target: { value: "DELETE" },
    });
    expect(confirm).toBeEnabled();

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
  });
});
