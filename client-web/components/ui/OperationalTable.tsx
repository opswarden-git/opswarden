"use client";

import {
  createContext,
  useContext,
  type HTMLAttributes,
  type ReactNode,
  type TableHTMLAttributes,
  type TdHTMLAttributes,
  type ThHTMLAttributes,
} from "react";
import { cn } from "@/lib/utils";

type OperationalTableDensity = "compact" | "normal";

const DensityContext = createContext<OperationalTableDensity>("normal");

const cellDensityClasses: Record<OperationalTableDensity, string> = {
  compact: "px-4 py-3",
  normal: "px-5 py-3.5",
};

/** Desktop-only table chrome. Data mapping, actions, destinations and mobile
 * morphology deliberately remain in each product feature. */
export function OperationalTable({
  children,
  className,
  density = "normal",
  label,
  ...props
}: TableHTMLAttributes<HTMLTableElement> & {
  density?: OperationalTableDensity;
  label: ReactNode;
}) {
  return (
    <DensityContext.Provider value={density}>
      <div className="surface overflow-hidden rounded-md">
        <table className={cn("w-full text-left text-sm", className)} {...props}>
          <caption className="sr-only">{label}</caption>
          {children}
        </table>
      </div>
    </DensityContext.Provider>
  );
}

export function OperationalTableHead({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <thead
      className={cn("surface-subtle border-border border-b text-xs uppercase", className)}
      {...props}
    >
      {children}
    </thead>
  );
}

export function OperationalTableBody({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tbody className={cn("divide-border divide-y", className)} {...props}>
      {children}
    </tbody>
  );
}

export function OperationalTableRow({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLTableRowElement>) {
  return (
    <tr className={cn("group transition-colors hover:bg-white/[0.04]", className)} {...props}>
      {children}
    </tr>
  );
}

export function OperationalTableHeaderCell({
  children,
  className,
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  const density = useContext(DensityContext);
  return (
    <th className={cn("text-muted font-medium", cellDensityClasses[density], className)} {...props}>
      {children}
    </th>
  );
}

export function OperationalTableCell({
  children,
  className,
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  const density = useContext(DensityContext);
  return (
    <td className={cn("align-middle", cellDensityClasses[density], className)} {...props}>
      {children}
    </td>
  );
}

export function OperationalTableRowHeader({
  children,
  className,
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  const density = useContext(DensityContext);
  return (
    <th
      className={cn("text-left align-middle font-normal", cellDensityClasses[density], className)}
      {...props}
      scope="row"
    >
      {children}
    </th>
  );
}
