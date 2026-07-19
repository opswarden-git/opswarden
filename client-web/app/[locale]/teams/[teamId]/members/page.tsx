import { notFound } from "next/navigation";
import { TeamMembersPage } from "@/components/teams/TeamMembersPage";
import { isUuid } from "@/lib/uuid";

export default async function MembersPage({ params }: { params: Promise<{ teamId: string }> }) {
  const { teamId } = await params;
  if (!isUuid(teamId)) notFound();

  return <TeamMembersPage teamId={teamId} />;
}
