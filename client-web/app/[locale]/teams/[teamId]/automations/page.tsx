import { redirect } from "next/navigation";

export default async function AutomationsRoute({
  params,
  searchParams,
}: {
  params: Promise<{ locale: string; teamId: string }>;
  searchParams: Promise<{ view?: string | string[] }>;
}) {
  const { locale, teamId } = await params;
  const { view: rawView } = await searchParams;
  const view = Array.isArray(rawView) ? rawView[0] : rawView;
  const destination = view === "connections" ? "integrations" : view === "runs" ? "runs" : "rules";

  redirect(`/${locale}/teams/${teamId}/${destination}`);
}
