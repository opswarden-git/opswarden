import { notFound } from "next/navigation";
import { WarRoomClient } from "./WarRoomClient";
import { isUuid } from "@/lib/uuid";

export default async function WarRoomPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;

  if (!isUuid(id)) {
    notFound();
  }

  return <WarRoomClient id={id} />;
}
