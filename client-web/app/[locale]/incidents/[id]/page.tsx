import { notFound } from "next/navigation";
import { IncidentDetailPage } from "@/components/incidents/IncidentDetailPage";
import { isUuid } from "@/lib/uuid";

export default async function WarRoomPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;

  if (!isUuid(id)) {
    notFound();
  }

  return <IncidentDetailPage incidentId={id} />;
}
