import { TeamSettingsPage } from "@/components/teams/TeamSettingsPage";

export default async function TeamSettingsRoute({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  return <TeamSettingsPage teamId={teamId} />;
}
