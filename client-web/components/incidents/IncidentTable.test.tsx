import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { IncidentListItem } from "@/lib/queries/incidents";
import { IncidentTable, IncidentTableSkeleton } from "./IncidentTable";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) => {
    const messages: Record<string, string> = {
      tableLabel: "Incident queue",
      loading: "Loading incidents...",
      colStatus: "Status",
      colTitleId: "Title & ID",
      colAssignee: "Assignee",
      colSeverity: "Severity",
      colAge: "Age",
      unassigned: "Unassigned",
    };
    if (key === "shortId") return `ID: ${values?.id}`;
    return messages[key] ?? key;
  },
}));

vi.mock("@/i18n/routing", () => ({
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const incident: IncidentListItem = {
  id: "10000000-0000-4000-8000-000000000001",
  team_id: "39aa8884-22cc-4764-a9e7-7df7c7619ba6",
  title: "Database outage",
  description: "Primary unavailable",
  status: "open",
  severity: "critical",
  assignee: { user_id: "user-1", email: "responder@opswarden.local" },
  created_at: "2026-07-20T08:00:00Z",
  created_by: null,
  updated_at: "2026-07-20T08:00:00Z",
};

afterEach(cleanup);

describe("IncidentTable", () => {
  it("keeps one destination and a row header in the desktop morphology", () => {
    render(<IncidentTable incidents={[incident]} />);
    const table = screen.getByRole("table", { name: "Incident queue" });
    const rowHeader = within(table).getByRole("rowheader", { name: /Database outage/ });

    expect(rowHeader).toHaveAttribute("scope", "row");
    expect(within(table).getAllByRole("link")).toHaveLength(1);
    expect(within(table).queryByText("Open", { selector: "a" })).not.toBeInTheDocument();
  });

  it("orders every named mobile field without duplicating the destination", () => {
    render(<IncidentTable incidents={[incident]} />);
    const list = screen.getByRole("list", { name: "Incident queue" });
    const record = within(list).getByRole("listitem");
    const fields = Array.from(record.querySelectorAll<HTMLElement>("[data-incident-field]"));

    expect(fields.map((field) => field.dataset.incidentField)).toEqual([
      "identity",
      "state",
      "assignee",
      "age",
    ]);
    expect(within(record).getAllByRole("link")).toHaveLength(1);
    expect(record).toHaveTextContent("Database outage");
    expect(record).toHaveTextContent("responder@opswarden.local");
  });

  it("gives loading the same desktop and mobile boundaries", () => {
    render(<IncidentTableSkeleton />);

    expect(screen.getByTestId("incident-skeleton-desktop")).toHaveClass("hidden", "lg:block");
    expect(screen.getByTestId("incident-skeleton-mobile")).toHaveClass("lg:hidden");
  });
});
