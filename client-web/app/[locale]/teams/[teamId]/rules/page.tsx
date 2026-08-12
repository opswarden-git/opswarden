import { redirect } from "next/navigation";
import { TeamAutomationsPage } from "@/components/automations/TeamAutomationsPage";

export default async function RulesRoute({
  params,
  searchParams,
}: {
  params: Promise<{ locale: string; teamId: string }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { locale, teamId } = await params;
  const query = await searchParams;
  const view = Array.isArray(query.view) ? query.view[0] : query.view;

  if (view === "runs") {
    const preserved = new URLSearchParams();
    for (const [name, rawValue] of Object.entries(query)) {
      if (name === "view" || rawValue === undefined) continue;
      for (const value of Array.isArray(rawValue) ? rawValue : [rawValue]) {
        preserved.append(name, value);
      }
    }
    const suffix = preserved.toString();
    redirect(`/${locale}/teams/${teamId}/runs${suffix ? `?${suffix}` : ""}`);
  }

  return <TeamAutomationsPage teamId={teamId} resource="rules" />;
}
