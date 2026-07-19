import { useTranslations } from "next-intl";
import type { IncidentListItem } from "@/lib/queries/incidents";
import { IncidentRow } from "./IncidentRow";

const columns = ["colStatus", "colTitleId", "colAssignee", "colSeverity", "colAge", "colAction"];

export function IncidentTable({ incidents }: { incidents: IncidentListItem[] }) {
  const t = useTranslations("Incidents");

  return (
    <div className="surface overflow-x-auto rounded-md">
      <table className="w-full min-w-[880px] text-left text-sm">
        <thead className="surface-subtle border-border border-b text-xs uppercase">
          <tr>
            {columns.map((column, index) => (
              <th
                key={column}
                className={`text-muted px-5 py-3.5 font-medium ${index === columns.length - 1 ? "text-right" : ""}`}
              >
                {t(column)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-border divide-y">
          {incidents.map((incident) => (
            <IncidentRow key={incident.id} incident={incident} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Loading state that preserves the table's final geometry. */
export function IncidentTableSkeleton() {
  return (
    <div className="surface overflow-hidden rounded-md" aria-label="Loading incidents">
      <div className="surface-subtle border-border grid h-11 grid-cols-[8rem_2fr_1.5fr_7rem_7rem_5rem] gap-5 border-b px-5" />
      <div className="divide-border divide-y">
        {Array.from({ length: 6 }, (_, index) => (
          <div
            key={index}
            className="grid h-[73px] animate-pulse grid-cols-[8rem_2fr_1.5fr_7rem_7rem_5rem] items-center gap-5 px-5"
          >
            <span className="bg-panel-2 h-5 w-20 rounded-full" />
            <span className="bg-panel-2 h-4 w-3/4 rounded" />
            <span className="bg-panel-2 h-4 w-4/5 rounded" />
            <span className="bg-panel-2 h-5 w-16 rounded-full" />
            <span className="bg-panel-2 h-4 w-14 rounded" />
            <span className="bg-panel-2 ml-auto h-8 w-16 rounded-md" />
          </div>
        ))}
      </div>
    </div>
  );
}
