"use client";

import React from 'react';
import { Link, usePathname } from '@/i18n/routing';
import { LayoutDashboard, ShieldAlert, Users, Settings, BotMessageSquare } from 'lucide-react';
import { cn } from '@/lib/utils';

export function BottomBar({ className }: { className?: string }) {
  const pathname = usePathname();

  const links = [
    { href: '/', icon: LayoutDashboard, label: 'Dash' },
    { href: '/incidents', icon: ShieldAlert, label: 'Incidents' },
    { href: '/teams', icon: Users, label: 'Teams' },
    { href: '/ai', icon: BotMessageSquare, label: 'AI' },
    { href: '/settings', icon: Settings, label: 'Settings' },
  ];

  return (
    <nav className={cn('fixed bottom-0 left-0 right-0 h-16 glass flex items-center justify-around px-2 z-50', className)}>
      {links.map((link) => {
        const isActive = pathname === link.href || pathname.startsWith(link.href + '/');
        
        return (
          <Link
            key={link.href}
            href={link.href}
            className={cn(
              "flex flex-col items-center justify-center gap-1 w-16 h-full transition-colors",
              isActive 
                ? "text-gold" 
                : "text-muted hover:text-gold"
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
