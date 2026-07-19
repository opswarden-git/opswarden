import { TeamMembersPage } from "@/components/teams/TeamMembersPage";

export default async function MembersPage({ params }: { params: Promise<{ teamId: string }> }) {
  const { teamId } = await params;

  return <TeamMembersPage teamId={teamId} />;
}
