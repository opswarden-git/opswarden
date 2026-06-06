"use client";

import React from "react";
import { Sidebar } from "./Sidebar";
import { BottomBar } from "./BottomBar";
import { usePathname } from "next/navigation";

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isAuthPage = pathname?.includes("/login") || pathname?.includes("/signup");

  if (isAuthPage) {
    return <div className="relative min-h-screen text-text">{children}</div>;
  }

  return (
    <div className="flex min-h-screen flex-col text-text md:flex-row">
      {/* Sidebar - hidden on mobile, visible on medium screens and up */}
      <Sidebar className="hidden md:flex" />

      {/* Main content area */}
      <main className="relative flex min-h-0 flex-1 flex-col overflow-y-auto">
        <div className="flex-1 p-4 md:p-8">{children}</div>
      </main>

      {/* Bottom Bar - visible on mobile, hidden on medium screens and up */}
      <BottomBar className="md:hidden" />
    </div>
  );
}
