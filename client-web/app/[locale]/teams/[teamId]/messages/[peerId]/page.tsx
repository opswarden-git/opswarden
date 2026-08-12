import { notFound } from "next/navigation";
import { DirectMessageRoomPage } from "@/components/messages/DirectMessageRoomPage";
import { isUuid } from "@/lib/uuid";

export default async function TeamDirectMessagePage({
  params,
}: {
  params: Promise<{ teamId: string; peerId: string }>;
}) {
  const { teamId, peerId } = await params;
  if (!isUuid(peerId)) notFound();

  return <DirectMessageRoomPage key={peerId} peerId={peerId} teamId={teamId} />;
}
