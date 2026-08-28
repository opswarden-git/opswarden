import { useTranslations } from "next-intl";
import type { ReactNode } from "react";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
} from "@/components/ui/OperationalTable";
import type { IncidentListItem } from "@/lib/queries/incidents";
import { IncidentMobileRecord, IncidentRow } from "./IncidentRow";

const columns = ["colIncident", "colStatus", "colSeverity", "colAssignee", "colAge"];

export function IncidentTable({
  headers,
  incidents,
  ageSortDirection,
}: {
  ageSortDirection?: "ascending" | "descending";
  headers?: Partial<Record<(typeof columns)[number], ReactNode>>;
  incidents: IncidentListItem[];
}) {
  const t = useTranslations("Incidents");

  return (
    <>
      <div data-incident-layout="desktop" className="hidden lg:block">
        <OperationalTable label={t("tableLabel")}>
          <OperationalTableHead>
            <tr>
              {columns.map((column) => (
                <OperationalTableHeaderCell
                  key={column}
                  aria-sort={column === "colAge" ? (ageSortDirection ?? "none") : undefined}
                >
                  {headers?.[column] ?? t(column)}
                </OperationalTableHeaderCell>
              ))}
            </tr>
          </OperationalTableHead>
          <OperationalTableBody>
            {incidents.map((incident) => (
              <IncidentRow key={incident.id} incident={incident} />
            ))}
          </OperationalTableBody>
        </OperationalTable>
      </div>
      <div data-incident-layout="mobile" className="surface overflow-hidden rounded-md lg:hidden">
        <ul aria-label={t("tableLabel")} className="divide-border-muted divide-y">
          {incidents.map((incident) => (
            <IncidentMobileRecord key={incident.id} incident={incident} />
          ))}
        </ul>
      </div>
    </>
  );
}

/** Loading state that preserves the final responsive boundaries. */
export function IncidentTableSkeleton() {
  const t = useTranslations("Incidents");
  return (
    <>
      <div data-testid="incident-skeleton-desktop" className="hidden lg:block">
        <OperationalTable label={t("loading")} aria-busy="true">
          <OperationalTableHead>
            <tr>
              {columns.map((column) => (
                <OperationalTableHeaderCell key={column}>
                  <span className="bg-panel-2 block h-3 w-16 animate-pulse rounded" />
                </OperationalTableHeaderCell>
              ))}
            </tr>
          </OperationalTableHead>
          <OperationalTableBody>
            {Array.from({ length: 6 }, (_, rowIndex) => (
              <OperationalTableRow key={rowIndex} className="hover:bg-transparent">
                {columns.map((column, columnIndex) => (
                  <OperationalTableCell key={column}>
                    <span
                      className={`bg-panel-2 block animate-pulse rounded ${
                        columnIndex === 0 ? "h-4 w-3/4" : "h-5 w-16"
                      }`}
                    />
                  </OperationalTableCell>
                ))}
              </OperationalTableRow>
            ))}
          </OperationalTableBody>
        </OperationalTable>
      </div>
      <div
        data-testid="incident-skeleton-mobile"
        className="surface divide-border-muted divide-y overflow-hidden rounded-md lg:hidden"
        aria-label={t("loading")}
        aria-busy="true"
      >
        {Array.from({ length: 4 }, (_, index) => (
          <div key={index} className="animate-pulse px-4 py-4">
            <span className="bg-panel-2 block h-4 w-3/4 rounded" />
            <span className="bg-panel-2 mt-1 block h-3 w-24 rounded" />
            <div className="mt-3 flex gap-2">
              <span className="bg-panel-2 block h-5 w-16 rounded-full" />
              <span className="bg-panel-2 block h-5 w-20 rounded-full" />
            </div>
            <div className="mt-3 grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2">
              <span className="bg-panel-2 block h-3 w-14 rounded" />
              <span className="bg-panel-2 block h-6 w-2/3 rounded" />
            </div>
            <div className="mt-2 grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2">
              <span className="bg-panel-2 block h-3 w-10 rounded" />
              <span className="bg-panel-2 block h-4 w-20 rounded" />
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
