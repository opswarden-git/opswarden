import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActionMenu } from "./ActionMenu";

afterEach(cleanup);

describe("ActionMenu", () => {
  it("renders actions in a portal and invokes the selected action", async () => {
    const onSelect = vi.fn();
    render(
      <ActionMenu
        label="Member actions"
        items={[
          { id: "role", label: "Change role", onSelect },
          { id: "separator", separator: true },
          { id: "ban", label: "Ban member", tone: "danger", onSelect: vi.fn() },
        ]}
      />,
    );

    fireEvent.pointerDown(screen.getByRole("button", { name: "Member actions" }), {
      button: 0,
      ctrlKey: false,
    });
    const item = await screen.findByRole("menuitem", { name: "Change role" });
    expect(item).toHaveClass("ow-action-menu-item");
    expect(screen.getByRole("menu")).toHaveClass("ow-action-menu");
    fireEvent.click(item);

    expect(onSelect).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menuitem", { name: "Change role" })).not.toBeInTheDocument();
  });

  it("supports disabled state on trigger button", () => {
    render(<ActionMenu disabled label="Disabled actions" items={[]} />);
    const trigger = screen.getByRole("button", { name: "Disabled actions" });
    expect(trigger).toBeDisabled();
  });

  it("opens from the keyboard, moves focus and restores it on Escape", async () => {
    render(
      <ActionMenu
        label="Keyboard actions"
        items={[
          { id: "edit", label: "Edit item", onSelect: vi.fn() },
          { id: "delete", label: "Delete item", onSelect: vi.fn() },
        ]}
      />,
    );

    const trigger = screen.getByRole("button", { name: "Keyboard actions" });
    trigger.focus();
    fireEvent.keyDown(trigger, { key: "Enter" });

    const firstItem = await screen.findByRole("menuitem", { name: "Edit item" });
    expect(firstItem).toHaveFocus();

    fireEvent.keyDown(firstItem, { key: "ArrowDown" });
    await waitFor(() =>
      expect(screen.getByRole("menuitem", { name: "Delete item" })).toHaveFocus(),
    );

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("menuitem", { name: "Edit item" })).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("handles disabled items and separators correctly", async () => {
    const activeSelect = vi.fn();
    const disabledSelect = vi.fn();
    render(
      <ActionMenu
        label="Options"
        items={[
          { id: "edit", label: "Edit item", onSelect: activeSelect },
          { id: "sep1", separator: true },
          { id: "delete", label: "Delete item", disabled: true, onSelect: disabledSelect },
        ]}
      />,
    );

    fireEvent.pointerDown(screen.getByRole("button", { name: "Options" }), {
      button: 0,
      ctrlKey: false,
    });

    const activeItem = await screen.findByRole("menuitem", { name: "Edit item" });
    const disabledItem = screen.getByRole("menuitem", { name: "Delete item" });
    const separator = screen.getByRole("separator");

    expect(activeItem).toBeInTheDocument();
    expect(disabledItem).toHaveAttribute("data-disabled");
    expect(separator).toBeInTheDocument();

    fireEvent.click(disabledItem);
    expect(disabledSelect).not.toHaveBeenCalled();
  });
});
