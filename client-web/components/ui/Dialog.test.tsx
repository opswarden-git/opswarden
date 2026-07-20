import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useRef, useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Button } from "./Button";
import { Dialog, DialogClose } from "./Dialog";

afterEach(cleanup);

function DialogHarness() {
  const [open, setOpen] = useState(false);
  const initialFocus = useRef<HTMLInputElement>(null);

  return (
    <Dialog
      open={open}
      onOpenChange={setOpen}
      trigger={<Button>Open dialog</Button>}
      title="Edit incident"
      description="Change the operational incident."
      closeLabel="Close dialog"
      initialFocus={initialFocus}
      footer={
        <DialogClose>
          <Button>Cancel</Button>
        </DialogClose>
      }
    >
      <label>
        Incident title
        <input ref={initialFocus} />
      </label>
    </Dialog>
  );
}

describe("Dialog", () => {
  it("owns naming, initial focus, Escape and focus restoration", async () => {
    render(<DialogHarness />);

    const trigger = screen.getByRole("button", { name: "Open dialog" });
    fireEvent.click(trigger);

    const dialog = screen.getByRole("dialog", { name: "Edit incident" });
    expect(dialog).toHaveAccessibleDescription("Change the operational incident.");
    await waitFor(() =>
      expect(screen.getByRole("textbox", { name: "Incident title" })).toHaveFocus(),
    );

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("keeps the scrolling body separate from the visible footer", () => {
    render(<DialogHarness />);
    fireEvent.click(screen.getByRole("button", { name: "Open dialog" }));

    expect(document.querySelector('[data-dialog-part="content"]')).toHaveClass(
      "max-h-[calc(100dvh-2rem)]",
      "flex-col",
    );
    expect(document.querySelector('[data-dialog-part="body"]')).toHaveClass(
      "min-h-0",
      "overflow-y-auto",
    );
    expect(document.querySelector('[data-dialog-part="footer"]')).toBeVisible();
    expect(screen.getByRole("button", { name: "Close dialog" })).toBeVisible();
  });
});
