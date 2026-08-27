"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { useTeamScope } from "@/components/teams/TeamScope";
import { useFirstRunGuidance, type GuidedSection } from "@/lib/firstRunGuidance";

/** The order a workspace is actually set up in, not the order of the menu. */
const ORDER: GuidedSection[] = ["incidents", "releases", "integrations", "rules", "teamSettings"];

const STORAGE_PREFIX = "opswarden-tour";

/**
 * Each key is written out rather than built from the section name: the
 * completeness test only proves keys it can see as literals, and one it cannot
 * see reaches the reader as `Sidebar.tour…`.
 */
function stepText(t: ReturnType<typeof useTranslations<"Sidebar">>, section: GuidedSection) {
  switch (section) {
    case "incidents":
      return t("tourIncidents");
    case "releases":
      return t("tourReleases");
    case "rules":
      return t("tourRules");
    case "integrations":
      return t("tourIntegrations");
    default:
      return t("tourTeam");
  }
}

/**
 * A short guided pass over the parts of a brand-new workspace, one bubble at a
 * time, anchored beside the navigation entry it talks about.
 *
 * The steps are the same derivation the markers use, so the tour can only ever
 * point at something genuinely empty that this member is allowed to fill; a
 * workspace that already has incidents simply starts further down the list, and
 * one that has everything gets no tour at all.
 *
 * Position is measured from the anchor and the bubble is `fixed`, because the
 * navigation scrolls inside its own overflow container and anything positioned
 * within it would be clipped at the rail's edge.
 */
export function GuidedTour() {
  const t = useTranslations("Sidebar");
  const { activeTeam } = useTeamScope();
  const guided = useFirstRunGuidance(activeTeam ?? undefined);
  const teamId = activeTeam?.team_id ?? "";

  const steps = React.useMemo(
    () => ORDER.filter((section) => guided.has(section)),
    // A Set is a new object on every render; its contents are what matter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [ORDER.map((section) => (guided.has(section) ? "1" : "0")).join("")],
  );

  const [index, setIndex] = React.useState(0);
  const [finished, setFinished] = React.useState(true);
  const [spot, setSpot] = React.useState<{ top: number; left: number } | null>(null);

  // `true` until mounted: the server cannot read storage, and a tour that
  // flashes for someone who already finished it is worse than one that starts a
  // moment late.
  React.useEffect(() => {
    if (!teamId) return;
    try {
      setFinished(window.localStorage.getItem(`${STORAGE_PREFIX}:${teamId}`) === "1");
    } catch {
      setFinished(false);
    }
  }, [teamId]);

  const section = finished ? undefined : steps[index];

  React.useEffect(() => {
    if (!section) return;
    const place = () => {
      const anchor = document.querySelector(`[data-guide-target="${section}"]`);
      if (!anchor) return setSpot(null);
      const box = anchor.getBoundingClientRect();
      setSpot({ top: box.top + box.height / 2, left: box.right + 12 });
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [section]);

  if (!section || !spot) return null;

  const last = index === steps.length - 1;
  const finish = () => {
    setFinished(true);
    try {
      window.localStorage.setItem(`${STORAGE_PREFIX}:${teamId}`, "1");
    } catch {
      // A reader who blocks storage takes the tour again next time.
    }
  };

  return (
    <div
      role="dialog"
      aria-label={t("tourLabel")}
      className="bg-gold text-gold-ink fixed z-50 flex w-60 -translate-y-1/2 flex-col gap-2 rounded-md px-3 py-2 text-sm shadow-lg"
      style={{ top: spot.top, left: spot.left }}
    >
      {/* The tail, pointing back at the entry this bubble is about. */}
      <span
        aria-hidden="true"
        className="bg-gold absolute top-1/2 -left-1 h-2 w-2 -translate-y-1/2 rotate-45"
      />
      <p>{stepText(t, section)}</p>
      <div className="flex items-center justify-between gap-2">
        <span className="text-gold-ink/70 text-xs tabular-nums">
          {t("tourProgress", { step: index + 1, total: steps.length })}
        </span>
        <div className="flex items-center gap-1">
          {/* Buttons on a gold surface: the shared variants are drawn for the
              dark plane, where their greys would sink into this one. */}
          <button
            type="button"
            onClick={finish}
            className="text-gold-ink/70 hover:text-gold-ink rounded px-2 py-1 text-xs underline-offset-2 hover:underline"
          >
            {t("tourSkip")}
          </button>
          <button
            type="button"
            onClick={() => (last ? finish() : setIndex(index + 1))}
            className="bg-gold-ink text-gold hover:bg-gold-ink/90 rounded px-2 py-1 text-xs font-semibold"
          >
            {last ? t("tourDone") : t("tourNext")}
          </button>
        </div>
      </div>
    </div>
  );
}
