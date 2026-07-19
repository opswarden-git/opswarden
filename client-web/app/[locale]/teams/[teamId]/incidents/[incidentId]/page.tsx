import { notFound } from "next/navigation";
import { IncidentDetailPage } from "@/components/incidents/IncidentDetailPage";
import { isUuid } from "@/lib/uuid";

export default async function TeamIncidentPage({
  params,
}: {
  params: Promise<{ teamId: string; incidentId: string }>;
}) {
  const { teamId, incidentId } = await params;
  if (!isUuid(incidentId)) notFound();

  return <IncidentDetailPage incidentId={incidentId} teamId={teamId} />;
}
