import React from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EmptyState } from "./EmptyState";

describe("EmptyState", () => {
  it("renders title, description and action", () => {
    render(
      <EmptyState
        title="No items found"
        description="Try adjusting your search filters"
        action={<button>Create item</button>}
      />,
    );

    expect(screen.getByRole("heading", { name: "No items found" })).toBeInTheDocument();
    expect(screen.getByText("Try adjusting your search filters")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create item" })).toBeInTheDocument();
  });
});
