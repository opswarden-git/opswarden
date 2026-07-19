import { notFound } from "next/navigation";
import { WarRoomClient } from "@/app/[locale]/incidents/[id]/WarRoomClient";
import { isUuid } from "@/lib/uuid";

export default async function TeamIncidentPage({
  params,
}: {
  params: Promise<{ teamId: string; incidentId: string }>;
}) {
  const { teamId, incidentId } = await params;
  if (!isUuid(teamId) || !isUuid(incidentId)) notFound();

  return <WarRoomClient id={incidentId} teamId={teamId} />;
}
