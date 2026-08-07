"use client";

import React from "react";
import { Sidebar } from "./Sidebar";
import { BottomBar } from "./BottomBar";
import { usePathname } from "next/navigation";
import { useRealtime } from "@/lib/ws";
import { TeamScopeProvider } from "@/components/teams/TeamScope";
import { MobileHeader } from "./MobileHeader";
import { ActiveIncidentContextBar } from "@/components/incidents/ActiveIncidentContextBar";

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
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
       * Pages that simply grow are unaffected: the inner content layer owns
       * their scrolling, while persistent shell context remains outside it.
       */}
      <div className="text-text flex h-dvh flex-col overflow-hidden md:flex-row">
        {/* Sidebar - hidden on mobile, visible on medium screens and up */}
        <Sidebar className="hidden md:flex" />

        {/* Main content area */}
        <main className="relative flex min-h-0 flex-1 flex-col overflow-hidden pb-16 md:pb-0">
          <MobileHeader />
          <ActiveIncidentContextBar />
          <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">{children}</div>
        </main>

        {/* Bottom Bar - visible on mobile, hidden on medium screens and up */}
        <BottomBar className="md:hidden" />
      </div>
    </TeamScopeProvider>
  );
}
