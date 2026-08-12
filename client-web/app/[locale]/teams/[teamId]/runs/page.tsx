import { TeamAutomationsPage } from "@/components/automations/TeamAutomationsPage";

export default async function RunsRoute({ params }: { params: Promise<{ teamId: string }> }) {
  const { teamId } = await params;
  return <TeamAutomationsPage teamId={teamId} resource="runs" />;
}
