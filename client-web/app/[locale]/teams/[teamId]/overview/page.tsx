import { TeamOverviewPage } from "@/components/teams/TeamOverviewPage";

export default async function TeamOverviewRoute({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  return <TeamOverviewPage teamId={teamId} />;
}
