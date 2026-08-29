"use client";

import { Ban, Circle, CircleCheck, CircleDot, X } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { StateChip } from "@/components/incidents/StateChip";
import { Alert } from "@/components/ui/Alert";
import { actionButtonClassNames, Button, IconButton } from "@/components/ui/Button";
import { Link } from "@/i18n/routing";
import { deriveCapabilities, type TeamRole } from "@/lib/capabilities";
import {
  type Release,
  useLinkIncident,
  useUnlinkIncident,
  useValidateStep,
} from "@/lib/queries/releases";
import { useTeamMembers } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";

/** Emails are the only identity this product carries; the local part reads as a name. */
function shortName(email: string) {
  return email.split("@")[0];
}

function ReleaseProgress({ steps }: { steps: Release["steps"] }) {
  const t = useTranslations("Releases");
  const completed = steps.filter((step) => step.validated).length;
  const total = steps.length;
  const percent = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <span className="text-muted flex shrink-0 items-center gap-2 text-sm font-normal">
      <span
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={total}
        aria-valuenow={completed}
        aria-label={t("progressSteps", { completed, total })}
        className="bg-panel-2 relative block h-1.5 w-24 overflow-hidden rounded-full"
      >
        <span
          className="bg-gold absolute inset-y-0 left-0 rounded-full"
          style={{ width: `${percent}%` }}
        />
      </span>
      <span className="tabular-nums">{t("progressCount", { completed, total })}</span>
    </span>
  );
}

export function ReleaseDetail({
  release,
  teamId,
  role,
}: {
  release: Release;
  teamId: string;
  role: TeamRole;
}) {
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const { data: members } = useTeamMembers(teamId);
  const validateStep = useValidateStep();
  const linkIncident = useLinkIncident();
  const unlinkIncident = useUnlinkIncident();
  const capabilities = deriveCapabilities(role);
  const terminal = release.state === "completed" || release.state === "cancelled";
  const validatable = release.state === "created" || release.state === "in_progress";
  const steps = [...release.steps].sort((left, right) => left.position - right.position);
  const nextStepIndex = steps.findIndex((step) => !step.validated);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const lastError = validateStep.error || linkIncident.error || unlinkIncident.error;
  const memberEmail = (userId: string | null) =>
    members?.find((member) => member.user_id === userId)?.email ?? t("unknownValidator");
  const canLink = capabilities.canLinkReleaseIncident && !terminal;
  const showLinked = release.linked_incidents.length > 0 || canLink;
  const stamp = (value: string) =>
    new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(
      new Date(value),
    );

  return (
    <div className="space-y-6">
      {lastError ? <Alert tone="danger">{errorText(lastError.message)}</Alert> : null}

      <section className="surface rounded-md p-4 sm:p-6" aria-labelledby="release-steps-title">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <h2 id="release-steps-title" className="text-text text-lg font-semibold">
            {t("deploymentSteps")}
          </h2>
          <ReleaseProgress steps={steps} />
        </div>

        <ol className="mt-4" aria-label={t("deploymentSteps")}>
          {steps.map((step, index) => {
            const isNext = index === nextStepIndex;
            const StepIcon = step.validated ? CircleCheck : isNext ? CircleDot : Circle;
            const canValidate =
              isNext && capabilities.canProgressRelease && validatable && !terminal;
            return (
              <li key={`${step.position}-${step.name}`} className="flex items-start gap-3 py-2">
                <StepIcon
                  className={`mt-0.5 h-4 w-4 shrink-0 ${
                    step.validated ? "text-rel-completed" : isNext ? "text-gold" : "text-muted-2"
                  }`}
                  aria-hidden="true"
                />
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 flex-wrap items-baseline gap-x-3 gap-y-1">
                    <span className="text-text min-w-0 truncate font-medium">{step.name}</span>
                    <span className="sr-only">
                      {step.validated
                        ? t("stepCompleted")
                        : isNext
                          ? t("stepNext")
                          : t("stepPending")}
                    </span>
                  </div>
                  {step.validated && step.validated_at ? (
                    <p className="text-muted-2 mt-1 text-xs">
                      {t("validatedBy", { who: shortName(memberEmail(step.validated_by)) })}
                    </p>
                  ) : null}
                  {isNext && release.state === "blocked" ? (
                    <p className="text-rel-blocked mt-1 flex items-center gap-1.5 text-xs">
                      <Ban className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                      <a href="#linked-incidents" className="underline underline-offset-2">
                        {t("blockedByCount", { count: release.blockers.length })}
                      </a>
                    </p>
                  ) : null}
                </div>
                {step.validated && step.validated_at ? (
                  <time
                    dateTime={step.validated_at}
                    className="text-muted-2 mt-0.5 shrink-0 text-xs tabular-nums"
                  >
                    {stamp(step.validated_at)}
                  </time>
                ) : null}
                {canValidate ? (
                  <Button
                    className={actionButtonClassNames()}
                    size="sm"
                    variant="primary"
                    loading={validateStep.isPending}
                    onClick={() =>
                      validateStep.mutate({
                        releaseId: release.release_id,
                        step: step.name,
                        teamId,
                      })
                    }
                  >
                    {t("validateNextStep")}
                  </Button>
                ) : null}
              </li>
            );
          })}
        </ol>
      </section>

      {showLinked ? (
        <section
          id="linked-incidents"
          className="surface scroll-mt-24 rounded-md p-4 sm:p-6"
          aria-labelledby="linked-incidents-title"
        >
          <h2 id="linked-incidents-title" className="text-text text-lg font-semibold">
            {t("linkedIncidents")}
          </h2>

          {release.linked_incidents.length > 0 ? (
            <ul className="divide-border-muted mt-2 divide-y">
              {release.linked_incidents.map((incident) => {
                return (
                  <li key={incident.incident_id} className="flex min-w-0 items-center gap-3 py-3">
                    <Link
                      href={teamPath(teamId, "incidents", incident.incident_id)}
                      className="text-text hover:text-gold min-w-0 flex-1 truncate text-sm font-medium transition-colors"
                    >
                      {incident.title}
                    </Link>
                    <StateChip status={incident.status} />
                    {canLink ? (
                      <IconButton
                        label={t("unlinkIncident", {
                          title: incident.title,
                        })}
                        size="sm"
                        variant="ghost"
                        disabled={unlinkIncident.isPending}
                        onClick={() =>
                          unlinkIncident.mutate({
                            releaseId: release.release_id,
                            incidentId: incident.incident_id,
                            teamId,
                          })
                        }
                      >
                        <X className="h-4 w-4" aria-hidden="true" />
                      </IconButton>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          ) : null}

          {canLink ? (
            <label className="mt-4 block max-w-sm">
              <span className="sr-only">{t("linkIncident")}</span>
              <select
                value=""
                onChange={(event) => {
                  const incidentId = event.target.value;
                  if (incidentId) {
                    linkIncident.mutate({ releaseId: release.release_id, incidentId, teamId });
                  }
                }}
                disabled={linkIncident.isPending || release.linkable_incidents.length === 0}
                className="ow-input h-9 w-full min-w-0 rounded-md px-3 text-sm disabled:opacity-50"
              >
                <option value="">
                  {release.linkable_incidents.length === 0
                    ? t("noLinkableIncidents")
                    : t("linkIncidentPlaceholder")}
                </option>
                {release.linkable_incidents.map((incident) => (
                  <option key={incident.incident_id} value={incident.incident_id}>
                    {incident.title}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}
