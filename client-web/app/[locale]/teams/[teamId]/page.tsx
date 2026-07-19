import { redirect } from "next/navigation";
import { teamPath } from "@/lib/team-routing";

export default async function TeamPage({
  params,
}: {
  params: Promise<{ locale: string; teamId: string }>;
}) {
  const { locale, teamId } = await params;
  redirect(`/${locale}${teamPath(teamId, "overview")}`);
}
