"use client";

import React from "react";
import Image from "next/image";
import { Languages } from "lucide-react";
import { useParams } from "next/navigation";
import { useRouter as useIntlRouter, usePathname } from "@/i18n/routing";
import { type AppLocale, isAppLocale } from "@/i18n/locales";
import { useUpdateLocale } from "@/lib/queries/profile";
import { useTranslations } from "next-intl";
import { ToggleButton } from "@/components/ui/ToggleButton";

/** Interface language switch (FR/EN). */
export function LanguagePanel() {
  const t = useTranslations("Settings");
  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = params.locale as string;
  const updateLocale = useUpdateLocale();

  const switchLocale = (newLocale: AppLocale) => {
    if (!isAppLocale(newLocale) || newLocale === currentLocale) return;
    updateLocale.mutate(newLocale, {
      onSuccess: () => intlRouter.replace(pathname, { locale: newLocale }),
    });
  };

  return (
    <div className="surface rounded-md p-6">
      <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
        <Languages className="text-muted h-5 w-5" />
        {t("language")}
      </h2>
      <div className="mt-4 flex items-center justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-text text-sm font-medium">{t("interfaceLanguage")}</h3>
        </div>
        <div className="flex shrink-0 gap-2">
          <ToggleButton
            pressed={currentLocale === "en"}
            size="sm"
            onClick={() => switchLocale("en")}
            disabled={updateLocale.isPending}
            aria-label={t("english")}
          >
            <Image
              src="/assets/en.webp"
              alt={t("englishFlagAlt")}
              width={24}
              height={24}
              className="block object-cover"
            />
            {t("englishShort")}
          </ToggleButton>
          <ToggleButton
            pressed={currentLocale === "fr"}
            size="sm"
            onClick={() => switchLocale("fr")}
            disabled={updateLocale.isPending}
            aria-label={t("french")}
          >
            <Image
              src="/assets/fr.webp"
              alt={t("frenchFlagAlt")}
              width={24}
              height={24}
              className="block object-cover"
            />
            {t("frenchShort")}
          </ToggleButton>
        </div>
      </div>
      {updateLocale.isError && (
        <p className="mt-3 text-sm text-red-400" role="alert">
          {t("languageSaveError")}
        </p>
      )}
    </div>
  );
}
