import { IncidentsPage } from "@/components/incidents/IncidentsPage";

export default async function TeamIncidentsPage({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;

  return <IncidentsPage teamId={teamId} />;
}
