"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import {
  LayoutDashboard,
  ShieldAlert,
  Users,
  Settings,
  BotMessageSquare,
  CircleUser,
} from "lucide-react";
import Image from "next/image";
import { cn } from "@/lib/utils";

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();

  const links = [
    { href: "/", icon: LayoutDashboard, label: "Dashboard" },
    { href: "/incidents", icon: ShieldAlert, label: "Incidents" },
    { href: "/teams", icon: Users, label: "Teams" },
    { href: "/ai", icon: BotMessageSquare, label: "Warden AI" },
  ];

  const isSettingsActive = pathname === "/settings" || pathname.startsWith("/settings/");

  return (
    <aside className={cn("glass flex w-80 flex-col", className)}>
      <Link
        href="/"
        className="flex h-28 w-full shrink-0 items-center justify-start gap-4 px-8 pt-4 transition-opacity hover:opacity-80"
      >
        <Image
          src="/assets/logo-icon.png"
          alt="Icon"
          width={32}
          height={32}
          className="h-8 w-auto object-contain"
          style={{ width: "auto" }}
        />
        <Image
          src="/assets/logo-text-light.png"
          alt="OpsWarden"
          width={220}
          height={44}
          className="h-7 w-auto object-contain object-left"
          style={{ width: "auto" }}
        />
      </Link>

      <nav className="flex-1 space-y-2 overflow-y-auto px-4 py-6">
        {links.map((link) => {
          const isActive = pathname === link.href || pathname.startsWith(link.href + "/");

          return (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "group flex items-center gap-4 px-4 py-3 text-lg transition-colors",
                isActive ? "font-medium text-gold" : "text-muted hover:text-gold",
              )}
            >
              <link.icon className="h-6 w-6" />
              <span>{link.label}</span>
            </Link>
          );
        })}
      </nav>

      <div className="mt-auto shrink-0 p-6">
        <Link
          href="/settings"
          title="Settings"
          className={cn(
            "flex items-center gap-4 px-2 transition-colors",
            isSettingsActive ? "text-gold" : "text-text hover:text-gold",
          )}
        >
          <CircleUser className="h-9 w-9 shrink-0" strokeWidth={1.5} />
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-lg font-medium">Operator</span>
            <span className="truncate text-base">Level 1 NOC</span>
          </div>
          <Settings className="h-5 w-5 shrink-0" />
        </Link>
      </div>
    </aside>
  );
}
