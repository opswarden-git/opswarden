"use client";

import { useTranslations } from "next-intl";
import type { ReactNode, RefObject } from "react";
import { Dialog } from "@/components/ui/Dialog";

export function AutomationDialog({
  children,
  description,
  initialFocus,
  onClose,
  open,
  title,
  footer,
}: {
  children: ReactNode;
  description: string;
  initialFocus?: RefObject<HTMLElement | null>;
  onClose: () => void;
  open: boolean;
  title: string;
  footer?: ReactNode;
}) {
  const t = useTranslations("Automations");
  
  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => !nextOpen && onClose()}
      title={title}
      description={description}
      closeLabel={t("close")}
      initialFocus={initialFocus}
      size="lg"
      bodyClassName="p-0"
      footer={footer}
    >
      {children}
    </Dialog>
  );
}
