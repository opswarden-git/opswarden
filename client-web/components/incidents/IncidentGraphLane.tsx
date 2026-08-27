"use client";

import { cn } from "@/lib/utils";

/** VS Code's swimlane pitch, which is also its row half-height (`Xh = 11`). */
const LANE = 11;
/** VS Code's node radius (`E3 = 4`); the release node uses its outer size. */
const NODE = 4;
const ROW = 32;

/**
 * One swimlane cell of the incident graph, transposed from the VS Code Source
 * Control graph: the same lane pitch and node radius, stretched to our taller
 * rows. The run spans the full cell so consecutive cells abut without a seam.
 *
 * A release is the hollow, larger node its incidents hang from; each incident
 * is a filled node on the run. An unlinked incident keeps the lane column so
 * every node in the list shares one centre line, and simply carries no run.
 *
 * Runs stop at the node edge rather than passing under it. VS Code masks the
 * run with a circle stroked in the background colour, which cannot work here:
 * the rail is translucent, so no opaque stroke would match it — and a hollow
 * release node has to read as genuinely empty.
 */
export function IncidentGraphLane({
  runsDown = false,
  runsUp = false,
  tone = "gold",
  variant,
}: {
  runsDown?: boolean;
  runsUp?: boolean;
  tone?: "gold" | "muted";
  variant: "release" | "incident" | "loose";
}) {
  const radius = variant === "release" ? NODE + 1 : variant === "loose" ? NODE - 1 : NODE;

  return (
    <svg
      viewBox={`0 0 ${LANE * 2} ${ROW}`}
      width={LANE * 2}
      height={ROW}
      className={cn("shrink-0", tone === "gold" ? "text-gold" : "text-muted-2")}
      aria-hidden="true"
    >
      {runsUp ? (
        <line
          x1={LANE}
          y1={0}
          x2={LANE}
          y2={ROW / 2 - radius}
          stroke="currentColor"
          strokeWidth={1}
        />
      ) : null}
      {runsDown ? (
        <line
          x1={LANE}
          y1={ROW / 2 + radius}
          x2={LANE}
          y2={ROW}
          stroke="currentColor"
          strokeWidth={1}
        />
      ) : null}
      <circle
        cx={LANE}
        cy={ROW / 2}
        r={radius}
        fill={variant === "release" ? "none" : "currentColor"}
        stroke="currentColor"
        strokeWidth={variant === "release" ? 2 : 0}
      />
    </svg>
  );
}
