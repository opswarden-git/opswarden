import { getTranslations } from "next-intl/server";
import { setRequestLocale } from "next-intl/server";

export default async function HomePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  // Enable static rendering
  setRequestLocale(locale);

  const t = await getTranslations("Index");

  return (
    <div className="w-full max-w-7xl">
      <h1 className="text-text text-4xl font-bold tracking-tight">{t("title")}</h1>
      <p className="text-muted mt-4 font-mono">En construction…</p>
    </div>
  );
}
