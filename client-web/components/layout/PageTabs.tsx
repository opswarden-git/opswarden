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
    <nav aria-label={ariaLabel} className="overflow-x-auto">
      <ul className="bg-panel border-border flex min-w-max gap-1 rounded-md border p-1">
        {tabs.map((tab) => (
          <li key={tab.href}>
            <Link
              href={tab.href}
              aria-current={tab.active ? "page" : undefined}
              className={cn(
                "text-muted hover:bg-panel-2 hover:text-text relative flex h-9 items-center gap-2 rounded px-3 text-sm font-medium transition-colors",
                tab.active && "bg-panel-2 text-text shadow-sm",
              )}
            >
              <span>{tab.label}</span>
              {tab.count !== undefined ? (
                <span
                  className={cn(
                    "min-w-5 rounded-full px-1.5 py-0.5 text-center text-xs tabular-nums",
                    tab.active ? "bg-gold/15 text-gold" : "bg-bg text-muted",
                  )}
                >
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
