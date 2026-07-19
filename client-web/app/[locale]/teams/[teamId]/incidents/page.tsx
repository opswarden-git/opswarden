import { notFound } from "next/navigation";
import { IncidentsPage } from "@/components/incidents/IncidentsPage";
import { isUuid } from "@/lib/uuid";

export default async function TeamIncidentsPage({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  if (!isUuid(teamId)) notFound();

  return <IncidentsPage teamId={teamId} />;
}
