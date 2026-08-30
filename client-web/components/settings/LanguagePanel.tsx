"use client";

import React from "react";
import { useLocale, useTranslations } from "next-intl";
import { useRouter as useIntlRouter, usePathname } from "@/i18n/routing";
import { type AppLocale, isAppLocale, LOCALE_OPTIONS } from "@/i18n/locales";
import { useUpdateLocale } from "@/lib/queries/profile";
import { cn } from "@/lib/utils";
import { SettingsRow, SettingsSection } from "./SettingsPrimitives";

/** Interface language switch (FR/EN) backed by central LOCALE_OPTIONS. */
export function LanguagePanel() {
  const t = useTranslations("Settings");
  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const currentLocale = useLocale();
  const updateLocale = useUpdateLocale();

  const switchLocale = (newLocale: AppLocale) => {
    if (!isAppLocale(newLocale) || newLocale === currentLocale) return;
    updateLocale.mutate(newLocale, {
      onSuccess: () => intlRouter.replace(pathname, { locale: newLocale }),
    });
  };

  const activeOption =
    LOCALE_OPTIONS.find((option) => option.code === currentLocale) ?? LOCALE_OPTIONS[0];

  return (
    <SettingsSection title={t("preferences")}>
      <SettingsRow
        label={t("interfaceLanguage")}
        action={
          <div className="flex shrink-0 items-center gap-4">
            {LOCALE_OPTIONS.map((option) => (
              <button
                key={option.code}
                type="button"
                aria-pressed={currentLocale === option.code}
                onClick={() => switchLocale(option.code)}
                disabled={updateLocale.isPending}
                aria-label={t(option.labelKey)}
                className={cn(
                  "text-muted hover:text-text text-xs font-medium transition-colors disabled:opacity-50",
                  currentLocale === option.code && "text-gold hover:text-gold",
                )}
              >
                {t(option.shortLabelKey)}
              </button>
            ))}
          </div>
        }
      >
        <span className="text-muted">{t(activeOption.labelKey)}</span>
      </SettingsRow>
      {updateLocale.isError && (
        <p className="py-3 text-sm text-red-400" role="alert">
          {t("languageSaveError")}
        </p>
      )}
    </SettingsSection>
  );
}
