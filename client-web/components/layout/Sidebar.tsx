"use client";

import React from 'react';
import { Link, usePathname } from '@/i18n/routing';
import { LayoutDashboard, ShieldAlert, Users, Settings, BotMessageSquare, CircleUser } from 'lucide-react';
import Image from 'next/image';
import { cn } from '@/lib/utils';

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();

  const links = [
    { href: '/', icon: LayoutDashboard, label: 'Dashboard' },
    { href: '/incidents', icon: ShieldAlert, label: 'Incidents' },
    { href: '/teams', icon: Users, label: 'Teams' },
    { href: '/ai', icon: BotMessageSquare, label: 'Warden AI' },
  ];

  const isSettingsActive = pathname === '/settings' || pathname.startsWith('/settings/');

  return (
    <aside className={cn('w-80 glass flex flex-col', className)}>
      <Link href="/" className="h-28 flex items-center gap-4 justify-start px-8 shrink-0 w-full pt-4 transition-opacity hover:opacity-80">
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
      
      <nav className="flex-1 overflow-y-auto py-6 px-4 space-y-2">
        {links.map((link) => {
          const isActive = pathname === link.href || pathname.startsWith(link.href + '/');
          
          return (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "flex items-center gap-4 px-4 py-3 text-lg transition-colors group",
                isActive 
                  ? "text-gold font-medium" 
                  : "text-muted hover:text-gold"
              )}
            >
              <link.icon className="h-6 w-6" />
              <span>{link.label}</span>
            </Link>
          );
        })}
      </nav>
      
      <div className="p-6 shrink-0 mt-auto">
        <Link
          href="/settings"
          title="Settings"
          className={cn(
            "flex items-center gap-4 px-2 transition-colors",
            isSettingsActive ? "text-gold" : "text-text hover:text-gold"
          )}
        >
          <CircleUser className="h-9 w-9 shrink-0" strokeWidth={1.5} />
          <div className="flex flex-col flex-1 min-w-0">
            <span className="text-lg font-medium truncate">Operator</span>
            <span className="text-base truncate">Level 1 NOC</span>
          </div>
          <Settings className="h-5 w-5 shrink-0" />
        </Link>
      </div>
    </aside>
  );
}
