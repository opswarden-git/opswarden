"use client";

import * as Dialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { useTranslations } from "next-intl";
import type { ReactNode, RefObject } from "react";
import { IconButton } from "@/components/ui/Button";

export function AutomationDialog({
  children,
  description,
  initialFocus,
  onClose,
  open,
  title,
}: {
  children: ReactNode;
  description: string;
  initialFocus?: RefObject<HTMLElement | null>;
  onClose: () => void;
  open: boolean;
  title: string;
}) {
  const t = useTranslations("Automations");
  return (
    <Dialog.Root open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="bg-bg/80 fixed inset-0 z-50 backdrop-blur-sm" />
        <Dialog.Content
          className="surface fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100vh-2rem)] w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-md shadow-2xl outline-none"
          onOpenAutoFocus={(event) => {
            if (!initialFocus?.current) return;
            event.preventDefault();
            initialFocus.current.focus();
          }}
        >
          <header className="border-border relative border-b p-6 pr-14">
            <Dialog.Title className="text-text text-lg font-semibold">{title}</Dialog.Title>
            <Dialog.Description className="text-muted mt-1 text-sm">
              {description}
            </Dialog.Description>
            <Dialog.Close asChild>
              <IconButton
                className="absolute top-4 right-4"
                label={t("close")}
                size="sm"
                variant="ghost"
              >
                <X className="h-4 w-4" aria-hidden="true" />
              </IconButton>
            </Dialog.Close>
          </header>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
