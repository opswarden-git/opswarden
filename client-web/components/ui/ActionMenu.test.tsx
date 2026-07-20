import { cleanup, fireEvent, render, screen } from "@testing-library/react";
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
});
