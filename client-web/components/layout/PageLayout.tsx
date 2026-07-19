import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export type PageLayoutWidth = "standard" | "workspace";

const widthClasses: Record<PageLayoutWidth, string> = {
  standard: "max-w-6xl",
  workspace: "max-w-[90rem]",
};

export interface PageLayoutProps extends HTMLAttributes<HTMLDivElement> {
  width?: PageLayoutWidth;
}

/**
 * Shared container for routed product pages.
 *
 * Direct children are page regions (breadcrumb, header, tabs, toolbar and
 * content). PageLayout is their only owner of width, outer padding and rhythm.
 */
export function PageLayout({ children, className, width = "standard", ...props }: PageLayoutProps) {
  return (
    <div
      data-page-layout="true"
      data-page-width={width}
      className={cn(
        "mx-auto flex w-full flex-col gap-6 px-4 py-6 sm:px-6 md:px-8 md:py-8",
        widthClasses[width],
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
