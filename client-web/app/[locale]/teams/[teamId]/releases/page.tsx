import { ReleasesPage } from "@/components/releases/ReleasesPage";

export default async function TeamReleasesPage({
  params,
}: {
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;

  return <ReleasesPage teamId={teamId} />;
}
