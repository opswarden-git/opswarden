import { notFound } from "next/navigation";
import { ReleasesPage } from "@/components/releases/ReleasesPage";
import { isUuid } from "@/lib/uuid";

export default async function TeamReleasesPage({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  if (!isUuid(teamId)) notFound();

  return <ReleasesPage teamId={teamId} />;
}
