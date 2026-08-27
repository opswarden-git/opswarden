"use client";

import React from "react";
import { X } from "lucide-react";
import { useTranslations } from "next-intl";
import { useTeamScope } from "@/components/teams/TeamScope";
import { IconButton } from "@/components/ui/Button";
import { useFirstRunGuidance, type GuidedSection } from "@/lib/firstRunGuidance";

const STORAGE_PREFIX = "opswarden-first-step";

function dismissedKey(teamId: string, section: GuidedSection) {
  return `${STORAGE_PREFIX}:${teamId}:${section}`;
}

/**
 * Each key is written out, not built from the section name: the completeness
 * test only proves keys that appear as literals, and a key it cannot see is a
 * key that reaches the reader as `Sidebar.firstStep…`.
 */
function hint(t: ReturnType<typeof useTranslations<"Sidebar">>, section: GuidedSection) {
  switch (section) {
    case "incidents":
      return t("firstStepIncidents");
    case "releases":
      return t("firstStepReleases");
    case "integrations":
      return t("firstStepIntegrations");
    default:
      return t("firstStepRules");
  }
}

/**
 * One sentence telling a brand-new workspace what to do on this page, and a way
 * to say you have read it.
 *
 * It shows only while the section is genuinely empty and the member may fill it
 * — the same derivation the sidebar marker uses, so the two can never disagree.
 * Dismissal is the one piece of state here, kept per team and per section in
 * `localStorage`: it belongs to one reader on one machine, not to the workspace,
 * and a fresh browser starts the demo over without anything to reset.
 */
const ORDER: GuidedSection[] = ["incidents", "releases", "integrations", "rules"];

export function FirstStepHint() {
  const t = useTranslations("Sidebar");
  const { activeTeam } = useTeamScope();
  const guided = useFirstRunGuidance(activeTeam ?? undefined);
  const teamId = activeTeam?.team_id ?? "";
  const section = ORDER.find((candidate) => guided.has(candidate));
  const [dismissed, setDismissed] = React.useState(true);

  // `true` until mounted: the server cannot read storage, and flashing a hint
  // the reader already dismissed is worse than showing it a moment late.
  React.useEffect(() => {
    if (!teamId || !section) return;
    try {
      setDismissed(window.localStorage.getItem(dismissedKey(teamId, section)) === "1");
    } catch {
      setDismissed(false);
    }
  }, [section, teamId]);

  if (!section || dismissed) return null;

  const dismiss = () => {
    setDismissed(true);
    try {
      window.localStorage.setItem(dismissedKey(teamId, section), "1");
    } catch {
      // A reader who blocks storage simply sees the hint again next time.
    }
  };

  return (
    <div className="surface-subtle border-border-muted text-muted flex items-start gap-3 rounded-md border px-4 py-3 text-sm">
      <p className="min-w-0 flex-1">{hint(t, section)}</p>
      <IconButton label={t("firstStepDone")} size="sm" variant="ghost" onClick={dismiss}>
        <X className="h-4 w-4" aria-hidden="true" />
      </IconButton>
    </div>
  );
}
