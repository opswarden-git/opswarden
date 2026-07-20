"use client";

import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { MoreHorizontal, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { IconButton } from "./Button";

export type ActionMenuItem = {
  id: string;
  label: string;
  icon?: LucideIcon;
  tone?: "neutral" | "danger";
  disabled?: boolean;
  onSelect: () => void;
};

export type ActionMenuEntry = ActionMenuItem | { id: string; separator: true };

export function ActionMenu({
  disabled = false,
  items,
  label,
}: {
  disabled?: boolean;
  items: ActionMenuEntry[];
  label: string;
}) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild disabled={disabled}>
        <IconButton label={label} size="sm" variant="ghost">
          <MoreHorizontal className="h-4 w-4" aria-hidden="true" />
        </IconButton>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          className="ow-action-menu surface z-50 min-w-48 rounded-md p-1 shadow-xl outline-none"
        >
          {items.map((item) => {
            if ("separator" in item) {
              return <DropdownMenu.Separator key={item.id} className="bg-border my-1 h-px" />;
            }

            const Icon = item.icon;
            return (
              <DropdownMenu.Item
                key={item.id}
                disabled={item.disabled}
                data-tone={item.tone ?? "neutral"}
                onSelect={item.onSelect}
                className={cn(
                  "ow-action-menu-item data-[highlighted]:bg-panel-2 flex cursor-default items-center gap-2 rounded px-2.5 py-2 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
                  item.tone === "danger"
                    ? "text-sev-critical data-[highlighted]:bg-sev-critical/10"
                    : "text-text",
                )}
              >
                {Icon ? <Icon className="h-4 w-4 shrink-0" aria-hidden="true" /> : null}
                <span>{item.label}</span>
              </DropdownMenu.Item>
            );
          })}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
