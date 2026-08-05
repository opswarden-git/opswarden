import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "./OperationalTable";

afterEach(cleanup);

describe("OperationalTable", () => {
  it("provides a labelled table, shared density and a semantic row header", () => {
    render(
      <OperationalTable label="Incident queue" density="compact">
        <OperationalTableHead>
          <tr>
            <OperationalTableHeaderCell>Incident</OperationalTableHeaderCell>
            <OperationalTableHeaderCell>Status</OperationalTableHeaderCell>
          </tr>
        </OperationalTableHead>
        <OperationalTableBody>
          <OperationalTableRow>
            <OperationalTableRowHeader>Database outage</OperationalTableRowHeader>
            <OperationalTableCell>Open</OperationalTableCell>
          </OperationalTableRow>
        </OperationalTableBody>
      </OperationalTable>,
    );

    expect(screen.getByRole("table", { name: "Incident queue" })).toBeInTheDocument();
    expect(screen.getByRole("rowheader", { name: "Database outage" })).toHaveAttribute(
      "scope",
      "row",
    );
    // "Shared" is the contract, not the literal values: a header and a cell in
    // the same table must carry identical padding. Which step of the scale they
    // land on is the spacing-scale contract's business, so a density change
    // does not have to be restated here.
    const padding = (element: HTMLElement) =>
      element.className
        .split(/\s+/)
        .filter((token) => /^p[xy]?-/.test(token))
        .sort();

    const header = screen.getByRole("columnheader", { name: "Incident" });
    const cell = screen.getByRole("cell", { name: "Open" });
    expect(padding(header).length).toBeGreaterThan(0);
    expect(padding(header)).toEqual(padding(cell));
  });
});
