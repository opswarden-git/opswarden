export const appLocales = ["en", "fr"] as const;

export type AppLocale = (typeof appLocales)[number];

export function isAppLocale(value: string): value is AppLocale {
  return appLocales.some((locale) => locale === value);
}

export interface LocaleOption {
  code: AppLocale;
  labelKey: "english" | "french";
  shortLabelKey: "englishShort" | "frenchShort";
}

export const LOCALE_OPTIONS: readonly LocaleOption[] = [
  { code: "en", labelKey: "english", shortLabelKey: "englishShort" },
  { code: "fr", labelKey: "french", shortLabelKey: "frenchShort" },
] as const;
