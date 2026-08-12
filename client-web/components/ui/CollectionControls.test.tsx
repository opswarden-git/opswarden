import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TableFilterControl, TableSortControl } from "./CollectionControls";

afterEach(cleanup);

describe("table collection controls", () => {
  it("keeps active filters grey without a status dot", () => {
    const view = render(
      <TableFilterControl
        label="Status"
        value="open"
        activeLabel="Open"
        onChange={vi.fn()}
        options={[{ value: "open", label: "Open" }]}
      />,
    );

    expect(screen.getByText("Status").closest("label")).toHaveClass("text-muted");
    expect(screen.getByText("Status").closest("label")).toHaveClass("uppercase");
    expect(view.container.querySelector(".bg-gold")).not.toBeInTheDocument();
    expect(view.container.querySelector(".lucide-chevron-down")).toBeInTheDocument();
  });

  it("uses the same compact chevron family for both sort directions", () => {
    const view = render(
      <>
        <TableSortControl label="Newest" direction="ascending" onToggle={vi.fn()} />
        <TableSortControl label="Oldest" direction="descending" onToggle={vi.fn()} />
      </>,
    );

    expect(view.container.querySelector(".lucide-chevron-up")).toBeInTheDocument();
    expect(view.container.querySelector(".lucide-chevron-down")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Newest" })).toHaveClass("text-muted");
    expect(screen.getByRole("button", { name: "Newest" })).toHaveClass("uppercase");
    expect(screen.getByRole("button", { name: "Oldest" })).toHaveClass("text-muted");
  });
});
