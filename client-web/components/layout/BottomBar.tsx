"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import { LayoutDashboard, ShieldAlert, Users, Settings, BotMessageSquare } from "lucide-react";
import { cn } from "@/lib/utils";

export function BottomBar({ className }: { className?: string }) {
  const pathname = usePathname();

  const links = [
    { href: "/", icon: LayoutDashboard, label: "Dash" },
    { href: "/incidents", icon: ShieldAlert, label: "Incidents" },
    { href: "/teams", icon: Users, label: "Teams" },
    { href: "/ai", icon: BotMessageSquare, label: "AI" },
    { href: "/settings", icon: Settings, label: "Settings" },
  ];

  return (
    <nav
      className={cn(
        "glass fixed right-0 bottom-0 left-0 z-50 flex h-16 items-center justify-around px-2",
        className,
      )}
    >
      {links.map((link) => {
        const isActive = pathname === link.href || pathname.startsWith(link.href + "/");

        return (
          <Link
            key={link.href}
            href={link.href}
            className={cn(
              "flex h-full w-16 flex-col items-center justify-center gap-1 transition-colors",
              isActive ? "text-gold" : "text-muted hover:text-gold",
            )}
          >
            <link.icon className="h-5 w-5" />
            <span className="text-[10px] font-medium">{link.label}</span>
          </Link>
        );
      })}
    </nav>
  );
}
