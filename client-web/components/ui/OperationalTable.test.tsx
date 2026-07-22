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
    expect(screen.getByRole("columnheader", { name: "Incident" })).toHaveClass("px-4", "py-3");
    expect(screen.getByRole("cell", { name: "Open" })).toHaveClass("px-4", "py-3");
  });
});
