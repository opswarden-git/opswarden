import { redirect } from "next/navigation";

export default async function TeamSettingsRoute({
  params,
}: {
  params: Promise<{ locale: string; teamId: string }>;
}) {
  const { locale, teamId } = await params;
  redirect(`/${locale}/teams/${teamId}/team`);
}
