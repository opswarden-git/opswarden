import { LegacyTeamRedirect } from "@/components/teams/LegacyTeamRedirect";

/** Compatibility redirect for bookmarks created before Warden AI was removed. */
export default function RemovedAIPage() {
  return <LegacyTeamRedirect />;
}
