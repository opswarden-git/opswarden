"use client";

import { deriveCapabilities } from "@/lib/capabilities";
import { connectableServices } from "@/lib/automation-catalog";
import {
  useAutomationCatalog,
  useAutomationRules,
  useTeamConnections,
} from "@/lib/queries/automations";
import type { Team } from "@/lib/queries/teams";

/** Navigation entries that can carry a first-run marker, in the order they are done. */
export type GuidedSection = "incidents" | "releases" | "rules" | "integrations" | "teamSettings";

/**
 * Which parts of a new workspace still have nothing in them.
 *
 * Derived, never stored. A marker appears when the resource count is zero and
 * the member is allowed to create one, so it clears itself on the first
 * creation and comes back if a workspace is emptied — which is what a rehearsed
 * demo wants, and what no persisted "onboarding done" flag would give.
 *
 * The three actions behind these markers are Manager-only, so nothing shows for
 * a Responder or an Observer: pointing someone at four doors they cannot open
 * is worse than staying quiet.
 *
 * A rule needs a connection for its Action, so the dependency is carried by the
 * order rather than by hiding the step: integrations comes first, and by the
 * time rules is reached the previous bubble has already asked for a service.
 */
export function useFirstRunGuidance(team: Team | undefined): Set<GuidedSection> {
  const teamId = team?.team_id ?? "";
  const capabilities = deriveCapabilities(team?.role ?? "observer");
  const wanted = capabilities.canManageAutomations;

  const active = wanted && !!teamId;
  const catalog = useAutomationCatalog(active);
  const connections = useTeamConnections(teamId, active);
  const rules = useAutomationRules(teamId, active);

  const pending = new Set<GuidedSection>();
  if (!team || !wanted) return pending;

  if (capabilities.canCreateIncident && team.active_incident_count === 0) {
    pending.add("incidents");
  }
  if (capabilities.canCreateRelease && team.active_release_count === 0) {
    pending.add("releases");
  }
  // A war room with one person in it is a notebook. Only offered to someone who
  // can actually bring the second person in.
  if (capabilities.canManageMembers && team.member_count <= 1) {
    pending.add("teamSettings");
  }

  // Every team is born with the internal services already connected, so a raw
  // count is never zero. What "set up an integration" means is a connection to
  // one of the services the catalogue says a human can configure.
  const configurable = new Set(
    connectableServices(catalog.data ?? []).map((service) => service.name),
  );
  // Undefined means "not answered yet": stay quiet rather than flash a marker
  // that a resolved query is about to remove.
  const connected =
    catalog.data && connections.data
      ? connections.data.filter((connection) => configurable.has(connection.service)).length
      : undefined;
  const ruleCount = rules.data?.length;
  if (connected === 0) pending.add("integrations");
  if (ruleCount === 0) pending.add("rules");

  return pending;
}
