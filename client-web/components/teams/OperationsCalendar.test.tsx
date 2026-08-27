import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OperationsCalendar } from "./OperationsCalendar";

vi.mock("@/i18n/routing", () => ({
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const labels = {
  calendar: "Operations calendar",
  today: "Today",
  previousMonth: "Previous month",
  nextMonth: "Next month",
  previousWeek: "Previous week",
  nextWeek: "Next week",
  month: "Month",
  week: "Week",
  incident: "Incident",
  less: "Show fewer",
  more: (count: number) => `+${count} more`,
  release: "Release",
  run: "Run",
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 7, 14, 12));
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("OperationsCalendar", () => {
  it("renders a complete month grid with linked operational events", () => {
    const view = render(
      <OperationsCalendar
        locale="en"
        labels={labels}
        events={[
          {
            id: "incident-1",
            occurredAt: "2026-08-14T10:00:00Z",
            href: "/teams/team-1/incidents/incident-1",
            title: "Database outage",
            type: "incident",
          },
        ]}
      />,
    );

    expect(screen.getByRole("heading", { name: "August 2026" })).toBeInTheDocument();
    expect(screen.getAllByRole("gridcell")).toHaveLength(42);
    const incident = screen.getByRole("link", { name: "Incident: Database outage" });
    expect(incident).toHaveAttribute(
      "href",
      "/teams/team-1/incidents/incident-1",
    );
    expect(incident).toHaveClass("bg-panel-2", "text-text", "border");
    expect(incident).not.toHaveClass("bg-status-danger", "bg-status-info", "bg-status-neutral");
    const today = view.container.querySelector('time[datetime="2026-08-14"]');
    expect(today).toHaveClass("text-gold");
    expect(today).not.toHaveClass("bg-gold", "text-gold-ink", "border");
  });

  it("derives distant month boundaries from the Gregorian calendar", () => {
    vi.setSystemTime(new Date(2033, 9, 15, 12));
    render(<OperationsCalendar locale="en" labels={labels} events={[]} />);

    expect(screen.getByRole("heading", { name: "October 2033" })).toBeInTheDocument();
    const cells = screen.getAllByRole("gridcell");
    expect(cells).toHaveLength(42);
    expect(cells[0]).toHaveAccessibleName("Monday, September 26, 2033");
    expect(cells[41]).toHaveAccessibleName("Sunday, November 6, 2033");
  });

  it("moves between months and returns to today", () => {
    render(<OperationsCalendar locale="en" labels={labels} events={[]} />);

    const viewButtons = screen.getAllByRole("button", { name: /Week|Month/ });
    expect(viewButtons.map((button) => button.textContent)).toEqual(["Week", "Month"]);
    expect(screen.getByRole("button", { name: "Month" })).toHaveClass("text-text");
    expect(screen.getByRole("button", { name: "Month" })).not.toHaveClass("bg-panel-2");
    expect(screen.queryByRole("button", { name: "Today" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Previous month" }));
    expect(screen.getByRole("heading", { name: "July 2026" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Today" }));
    expect(screen.getByRole("heading", { name: "August 2026" })).toBeInTheDocument();
  });

  it("keeps busy days compact until their remaining events are requested", () => {
    const events = Array.from({ length: 5 }, (_, index) => ({
      id: `incident-${index}`,
      occurredAt: `2026-08-14T1${index}:00:00Z`,
      href: `/incidents/${index}`,
      title: `Incident ${index}`,
      type: "incident" as const,
    }));
    render(<OperationsCalendar locale="en" labels={labels} events={events} />);

    expect(screen.getAllByRole("link", { name: /Incident: Incident/ })).toHaveLength(2);
    fireEvent.click(screen.getByRole("button", { name: "+3 more" }));
    expect(screen.getAllByRole("link", { name: /Incident: Incident/ })).toHaveLength(5);
    expect(screen.getByRole("button", { name: "Show fewer" })).toBeInTheDocument();
  });

  it("switches to an hourly week and navigates seven days at a time", () => {
    render(<OperationsCalendar locale="en" labels={labels} events={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Week" }));
    expect(screen.getByRole("button", { name: "Week" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("heading", { name: "Aug 10 – Aug 16, 2026" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Next week" }));
    expect(screen.getByRole("heading", { name: "Aug 17 – Aug 23, 2026" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Today" })).toBeInTheDocument();
  });
});
