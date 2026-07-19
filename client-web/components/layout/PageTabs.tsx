import type { ReactNode } from "react";
import { Link } from "@/i18n/routing";
import { cn } from "@/lib/utils";

export interface PageTab {
  href: string;
  label: ReactNode;
  count?: number;
  active?: boolean;
}

/** URL-backed page-level views with a stable underline and optional counters. */
export function PageTabs({ ariaLabel, tabs }: { ariaLabel: string; tabs: PageTab[] }) {
  return (
    <nav aria-label={ariaLabel} className="border-border -mb-6 overflow-x-auto border-b">
      <ul className="flex min-w-max gap-1">
        {tabs.map((tab) => (
          <li key={tab.href}>
            <Link
              href={tab.href}
              aria-current={tab.active ? "page" : undefined}
              className={cn(
                "text-muted hover:text-text relative flex h-11 items-center gap-2 px-3 text-sm font-medium transition-colors",
                tab.active &&
                  "text-text after:bg-gold after:absolute after:inset-x-2 after:bottom-0 after:h-0.5",
              )}
            >
              <span>{tab.label}</span>
              {tab.count !== undefined ? (
                <span className="bg-panel-2 text-muted min-w-5 rounded-full px-1.5 py-0.5 text-center text-xs tabular-nums">
                  {tab.count}
                </span>
              ) : null}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
