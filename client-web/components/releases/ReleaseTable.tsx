import { useTranslations } from "next-intl";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
} from "@/components/ui/OperationalTable";
import type { ReleaseListItem } from "@/lib/queries/releases";
import { ReleaseMobileRecord, ReleaseRow } from "./ReleaseRow";

const columns = ["colRelease", "colState", "colProgress", "colNextStep", "colAge", "colBlockers"];

export function ReleaseTable({
  hrefFor,
  releases,
}: {
  hrefFor: (releaseId: string) => string;
  releases: ReleaseListItem[];
}) {
  const t = useTranslations("Releases");

  return (
    <>
      <div data-release-layout="desktop" className="hidden lg:block">
        <OperationalTable label={t("tableLabel")}>
          <OperationalTableHead>
            <tr>
              {columns.map((column) => (
                <OperationalTableHeaderCell key={column}>{t(column)}</OperationalTableHeaderCell>
              ))}
            </tr>
          </OperationalTableHead>
          <OperationalTableBody>
            {releases.map((release) => (
              <ReleaseRow key={release.release_id} release={release} hrefFor={hrefFor} />
            ))}
          </OperationalTableBody>
        </OperationalTable>
      </div>
      <div data-release-layout="mobile" className="surface overflow-hidden rounded-md lg:hidden">
        <ul aria-label={t("tableLabel")} className="divide-border divide-y">
          {releases.map((release) => (
            <ReleaseMobileRecord key={release.release_id} release={release} hrefFor={hrefFor} />
          ))}
        </ul>
      </div>
    </>
  );
}

export function ReleaseTableSkeleton() {
  const t = useTranslations("Releases");
  return (
    <>
      <div data-testid="release-skeleton-desktop" className="hidden lg:block">
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
            {Array.from({ length: 5 }, (_, rowIndex) => (
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
        data-testid="release-skeleton-mobile"
        className="surface divide-border divide-y overflow-hidden rounded-md lg:hidden"
        aria-label={t("loading")}
        aria-busy="true"
      >
        {Array.from({ length: 4 }, (_, index) => (
          <div key={index} className="animate-pulse space-y-3 px-4 py-4">
            <span className="bg-panel-2 block h-4 w-3/4 rounded" />
            <span className="bg-panel-2 block h-5 w-16 rounded-full" />
            <span className="bg-panel-2 block h-3 w-24 rounded" />
            <div className="flex gap-2">
              <span className="bg-panel-2 block h-4 w-1/3 rounded" />
              <span className="bg-panel-2 block h-4 w-1/4 rounded" />
            </div>
            <span className="bg-panel-2 block h-4 w-1/2 rounded" />
          </div>
        ))}
      </div>
    </>
  );
}
