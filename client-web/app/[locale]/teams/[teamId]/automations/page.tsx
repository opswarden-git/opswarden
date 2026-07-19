import { notFound } from "next/navigation";
import { TeamAutomationsPage } from "@/components/automations/TeamAutomationsPage";
import { isUuid } from "@/lib/uuid";

export default async function AutomationsRoute({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  if (!isUuid(teamId)) notFound();

  return <TeamAutomationsPage teamId={teamId} />;
}
