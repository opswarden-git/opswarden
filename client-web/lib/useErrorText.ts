import { useTranslations } from "next-intl";

/** Resolve stable server/client error codes without ever exposing them as UI copy. */
export function useErrorText() {
  const t = useTranslations("errors");

  return (code: string) => (t.has(code) ? t(code) : t("unknown"));
}
