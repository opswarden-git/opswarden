"use client";

import React, { useState } from "react";
import { Sidebar } from "./Sidebar";
import { BottomBar } from "./BottomBar";
import { usePathname } from "next/navigation";
import { useRealtime } from "@/lib/ws";
import { TeamScopeProvider } from "@/components/teams/TeamScope";
import { MobileHeader } from "./MobileHeader";
import { AppBreadcrumbs } from "./AppBreadcrumbs";
import { PageActionsHostContext } from "./PageActionsRail";

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [pageActionsHost, setPageActionsHost] = useState<HTMLElement | null>(null);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const isAuthPage = pathname?.includes("/login") || pathname?.includes("/signup");

  // Global websocket hook (only active when not on auth pages and user is logged in)
  useRealtime();

  if (isAuthPage) {
    return <div className="text-text relative min-h-screen">{children}</div>;
  }

  return (
    <TeamScopeProvider>
      {/*
       * `h-dvh` rather than `min-h-screen`: a bounded shell is what lets a page
       * opt into filling the viewport and scrolling its own middle, which is
       * how the incident room keeps its composer reachable. Dynamic viewport
       * units track the mobile keyboard, which `vh` does not.
       *
       * Pages that simply grow are unaffected: `main` owns their scrolling.
       * The child wrapper must stay `min-h-0`, otherwise the breadcrumb plus a
       * full-height room exceed the viewport and push its composer off-screen.
       */}
      <div className="text-text flex h-dvh flex-col overflow-hidden md:flex-row">
        {/* Sidebar - hidden on mobile, visible on medium screens and up */}
        <Sidebar
          className="hidden md:flex"
          collapsed={isSidebarCollapsed}
          onCollapsedChange={setIsSidebarCollapsed}
        />

        {/* Main content area */}
        <main className="relative flex min-h-0 flex-1 flex-col overflow-y-auto pb-16 md:pb-0">
          <MobileHeader />
          <PageActionsHostContext.Provider value={pageActionsHost}>
            <AppBreadcrumbs onActionsHostChange={setPageActionsHost} />
            <div className="flex min-h-0 flex-1 flex-col">{children}</div>
          </PageActionsHostContext.Provider>
        </main>

        {/* Bottom Bar - visible on mobile, hidden on medium screens and up */}
        <BottomBar className="md:hidden" />
      </div>
    </TeamScopeProvider>
  );
}
