import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export type PageLayoutWidth = "standard" | "workspace";

const widthClasses: Record<PageLayoutWidth, string> = {
  standard: "max-w-6xl",
  workspace: "max-w-[90rem]",
};

export interface PageLayoutProps extends HTMLAttributes<HTMLDivElement> {
  width?: PageLayoutWidth;
  /**
   * Fill the shell instead of growing with content, so a region inside can own
   * the scrolling. A room keeps its header and composer in place while only the
   * transcript moves; a queue or a form has nothing to gain from it.
   */
  fill?: boolean;
}

/**
 * Shared container for routed product pages.
 *
 * Direct children are page regions (breadcrumb, header, tabs, toolbar and
 * content). PageLayout is their only owner of width, outer padding and rhythm.
 */
export function PageLayout({
  children,
  className,
  width = "standard",
  fill = false,
  ...props
}: PageLayoutProps) {
  return (
    <div
      data-page-layout="true"
      data-page-width={width}
      data-page-fill={fill ? "true" : undefined}
      className={cn(
        "mx-auto flex w-full flex-col gap-6 px-4 py-6 sm:px-6 md:px-8 md:py-8",
        widthClasses[width],
        fill && "min-h-0 flex-1",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
