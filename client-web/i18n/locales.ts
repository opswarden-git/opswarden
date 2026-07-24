export const appLocales = ["en", "fr"] as const;

export type AppLocale = (typeof appLocales)[number];

export function isAppLocale(value: string): value is AppLocale {
  return appLocales.some((locale) => locale === value);
}
