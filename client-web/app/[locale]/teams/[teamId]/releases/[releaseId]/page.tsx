import { notFound } from "next/navigation";
import { ReleaseDetailPage } from "@/components/releases/ReleaseDetailPage";
import { isUuid } from "@/lib/uuid";

export default async function TeamReleaseDetailRoute({
  params,
}: {
  params: Promise<{ teamId: string; releaseId: string }>;
}) {
  const { teamId, releaseId } = await params;
  if (!isUuid(releaseId)) notFound();

  return <ReleaseDetailPage teamId={teamId} releaseId={releaseId} />;
}
