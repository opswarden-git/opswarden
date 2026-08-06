import { TeamActivityPage } from "@/components/teams/TeamActivityPage";

export default async function ActivityRoute({ params }: { params: Promise<{ teamId: string }> }) {
  const { teamId } = await params;

  return <TeamActivityPage teamId={teamId} />;
}
