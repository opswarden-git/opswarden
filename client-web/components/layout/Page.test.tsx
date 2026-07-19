import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PageContent } from "./PageContent";
import { PageHeader } from "./PageHeader";
import { PageLayout } from "./PageLayout";
import { Button } from "@/components/ui/Button";

afterEach(cleanup);

describe("PageLayout", () => {
  it("owns the standard page width, padding and region rhythm", () => {
    const { container } = render(<PageLayout>Content</PageLayout>);

    expect(container.firstChild).toHaveClass("max-w-6xl", "px-4", "md:px-8", "gap-6");
    expect(container.firstChild).toHaveAttribute("data-page-layout", "true");
    expect(container.firstChild).toHaveAttribute("data-page-width", "standard");
  });

  it("supports an explicit workspace width without changing its spacing contract", () => {
    const { container } = render(<PageLayout width="workspace">Content</PageLayout>);

    expect(container.firstChild).toHaveClass("max-w-[90rem]", "px-4", "gap-6");
    expect(container.firstChild).toHaveAttribute("data-page-width", "workspace");
  });
});

describe("PageHeader", () => {
  it("renders one page heading with optional context and actions", () => {
    render(
      <PageHeader
        title="Incidents"
        description="Operational incidents"
        metadata="Updated now"
        actions={<Button>Create incident</Button>}
      />,
    );

    expect(screen.getByRole("heading", { level: 1, name: "Incidents" })).toBeInTheDocument();
    expect(screen.getByText("Operational incidents")).toBeInTheDocument();
    expect(screen.getByText("Updated now")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create incident" })).toBeInTheDocument();
  });
});

describe("PageContent", () => {
  it("renders only the fallback for the active state", () => {
    const { rerender } = render(
      <PageContent state="loading" loadingFallback="Loading" emptyFallback="Nothing here">
        Ready content
      </PageContent>,
    );

    expect(screen.getByText("Loading")).toBeInTheDocument();
    expect(screen.queryByText("Ready content")).not.toBeInTheDocument();
    expect(screen.getByText("Loading")).toHaveAttribute("aria-busy", "true");

    rerender(
      <PageContent state="empty" loadingFallback="Loading" emptyFallback="Nothing here">
        Ready content
      </PageContent>,
    );

    expect(screen.getByText("Nothing here")).toBeInTheDocument();
    expect(screen.queryByText("Loading")).not.toBeInTheDocument();
  });
});
