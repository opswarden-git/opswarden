import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { isUuid } from "@/lib/uuid";

export default async function TeamLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ teamId: string }>;
}) {
  const { teamId } = await params;
  if (!isUuid(teamId)) notFound();

  return children;
}
