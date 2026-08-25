"use client";

import { Ban, Check } from "lucide-react";
import { useTranslations } from "next-intl";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { Alert } from "@/components/ui/Alert";
import { actionButtonClassNames, Button } from "@/components/ui/Button";
import { Link } from "@/i18n/routing";
import { deriveCapabilities, type TeamRole } from "@/lib/capabilities";
import { useIncidents } from "@/lib/queries/incidents";
import {
  type Release,
  useLinkIncident,
  useUnlinkIncident,
  useValidateStep,
} from "@/lib/queries/releases";
import { useTeamMembers } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ReleaseStateChip } from "./ReleaseStateChip";

export function ReleaseDetail({
  release,
  teamId,
  role,
  cancelPending = false,
  onCancel,
}: {
  release: Release;
  teamId: string;
  role: TeamRole;
  cancelPending?: boolean;
  onCancel?: () => void;
}) {
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");
  const { data: incidents } = useIncidents(teamId);
  const { data: members } = useTeamMembers(teamId);
  const validateStep = useValidateStep();
  const linkIncident = useLinkIncident();
  const unlinkIncident = useUnlinkIncident();
  const capabilities = deriveCapabilities(role);
  const terminal = release.state === "completed" || release.state === "cancelled";
  const validatable = release.state === "created" || release.state === "in_progress";
  const steps = [...release.steps].sort((left, right) => left.position - right.position);
  const nextStepIndex = steps.findIndex((step) => !step.validated);
  const nextStep = nextStepIndex >= 0 ? steps[nextStepIndex] : null;
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const lastError = validateStep.error || linkIncident.error || unlinkIncident.error;
  const incidentById = (id: string) => (incidents ?? []).find((incident) => incident.id === id);
  const linkedIncidents = release.linked_incident_ids
    .map(incidentById)
    .filter((incident) => incident !== undefined);
  const blockingIncidents = linkedIncidents.filter((incident) => incident.status !== "resolved");
  const linkable = (incidents ?? []).filter(
    (incident) =>
      incident.status !== "resolved" && !release.linked_incident_ids.includes(incident.id),
  );
  const memberEmail = (userId: string | null) =>
    members?.find((member) => member.user_id === userId)?.email ?? t("unknownValidator");

  return (
    <div className="space-y-6">
      {release.state === "blocked" ? (
        <Alert tone="danger" title={t("blockedBannerTitle")} className="p-4">
          <p>{t("blockedBannerDescription", { count: blockingIncidents.length })}</p>
          {blockingIncidents.length > 0 ? (
            <ul className="mt-3 space-y-2">
              {blockingIncidents.map((incident) => (
                <li key={incident.id} className="flex min-w-0 flex-wrap items-center gap-2">
                  <Link
                    href={teamPath(teamId, "incidents", incident.id)}
                    className="text-text min-w-0 truncate font-medium underline underline-offset-2"
                  >
                    {incident.title}
                  </Link>
                  <SeverityChip severity={incident.severity} />
                  <StateChip status={incident.status} />
                </li>
              ))}
            </ul>
          ) : null}
        </Alert>
      ) : null}

      {lastError ? <Alert tone="danger">{errorText(lastError.message)}</Alert> : null}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <section className="surface rounded-md p-5 sm:p-6" aria-labelledby="release-steps-title">
          <div>
            <h2 id="release-steps-title" className="text-text text-lg font-semibold">
              {t("deploymentSteps")}
            </h2>
            <p className="text-muted mt-1 text-sm">{t("deploymentStepsDescription")}</p>
          </div>

          <ol className="mt-6" aria-label={t("deploymentSteps")}>
            {steps.map((step, index) => {
              const isNext = index === nextStepIndex;
              const isLast = index === steps.length - 1;
              return (
                <li
                  key={`${step.position}-${step.name}`}
                  className="relative flex gap-4 pb-6 last:pb-0"
                >
                  <div className="relative flex w-8 shrink-0 justify-center">
                    {!isLast ? (
                      <span
                        className="bg-border absolute top-8 bottom-0 left-1/2 w-px -translate-x-1/2"
                        aria-hidden="true"
                      />
                    ) : null}
                    <span
                      className={`relative z-10 flex h-8 w-8 items-center justify-center rounded-full border text-xs font-semibold ${
                        step.validated
                          ? "border-rel-completed/40 bg-rel-completed/15 text-rel-completed"
                          : isNext
                            ? "border-gold bg-gold/10 text-gold"
                            : "border-border bg-panel text-muted"
                      }`}
                    >
                      {step.validated ? (
                        <Check className="h-4 w-4" aria-hidden="true" />
                      ) : (
                        index + 1
                      )}
                    </span>
                  </div>
                  <div className="min-w-0 flex-1 pt-1">
                    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
                      <span className="text-text min-w-0 truncate font-medium">{step.name}</span>
                      <span className="text-muted text-xs">
                        {step.validated
                          ? t("stepCompleted")
                          : isNext
                            ? t("stepNext")
                            : t("stepPending")}
                      </span>
                    </div>
                    {step.validated && step.validated_at ? (
                      <p className="text-muted mt-1 text-xs">
                        {t("validatedBy", {
                          who: memberEmail(step.validated_by),
                          when: new Date(step.validated_at).toLocaleString(),
                        })}
                      </p>
                    ) : null}
                  </div>
                </li>
              );
            })}
          </ol>
        </section>

        <aside
          className="surface h-fit overflow-hidden rounded-md"
          aria-labelledby="release-actions-title"
        >
          <header className="border-border border-b px-4 py-3">
            <h2 id="release-actions-title" className="text-text text-sm font-semibold">
              {t("releaseDetail")}
            </h2>
          </header>

          <div className="divide-border divide-y text-sm">
            <section className="space-y-3 p-4">
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted text-xs font-medium">{t("colStatus")}</span>
                <ReleaseStateChip state={release.state} />
              </div>
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted text-xs font-medium">{t("colProgress")}</span>
                <span className="text-text text-xs tabular-nums">
                  {t("progressCount", {
                    completed: steps.filter((step) => step.validated).length,
                    total: steps.length,
                  })}
                </span>
              </div>
            </section>

            {nextStep && !terminal ? (
              <section className="space-y-3 p-4">
                <div>
                  <h3 className="text-muted text-xs font-medium">{t("nextStep")}</h3>
                  <p className="text-text mt-1 truncate font-medium">{nextStep.name}</p>
                </div>
                {release.state === "blocked" ? (
                  <p className="text-rel-blocked flex items-center gap-1.5 text-xs">
                    <Ban className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("resolveBlockersFirst")}
                  </p>
                ) : null}
                {capabilities.canProgressRelease && validatable ? (
                  <Button
                    fullWidth
                    variant="primary"
                    className={actionButtonClassNames()}
                    onClick={() =>
                      validateStep.mutate({
                        releaseId: release.release_id,
                        step: nextStep.name,
                        teamId,
                      })
                    }
                    loading={validateStep.isPending}
                  >
                    {t("validateNextStep")}
                  </Button>
                ) : null}
              </section>
            ) : null}

            <section className="p-4" aria-labelledby="linked-incidents-title">
              <h3 id="linked-incidents-title" className="text-muted text-xs font-medium">
                {t("linkedIncidents")}
              </h3>

              {release.linked_incident_ids.length === 0 ? (
                <p className="text-muted mt-3 text-sm">{t("noLinkedIncidents")}</p>
              ) : (
                <ul className="divide-border mt-2 divide-y">
                  {release.linked_incident_ids.map((id) => {
                    const incident = incidentById(id);
                    return (
                      <li key={id} className="min-w-0 py-3">
                        <div className="flex min-w-0 items-center gap-2">
                          <Link
                            href={teamPath(teamId, "incidents", id)}
                            className="text-text hover:text-gold min-w-0 flex-1 truncate text-sm font-medium transition-colors"
                          >
                            {incident ? incident.title : t("unknownIncident")}
                          </Link>
                          {incident ? <StateChip status={incident.status} /> : null}
                        </div>
                        {capabilities.canLinkReleaseIncident ? (
                          <Button
                            className="mt-2"
                            fullWidth
                            size="sm"
                            variant="secondary"
                            onClick={() =>
                              unlinkIncident.mutate({
                                releaseId: release.release_id,
                                incidentId: id,
                                teamId,
                              })
                            }
                            disabled={unlinkIncident.isPending}
                          >
                            {t("unlinkIncident", {
                              title: incident?.title ?? t("unknownIncident"),
                            })}
                          </Button>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              )}

              {capabilities.canLinkReleaseIncident && !terminal ? (
                <label className="mt-3 block">
                  <span className="sr-only">{t("linkIncident")}</span>
                  <select
                    value=""
                    onChange={(event) => {
                      const incidentId = event.target.value;
                      if (incidentId) {
                        linkIncident.mutate({ releaseId: release.release_id, incidentId, teamId });
                      }
                    }}
                    disabled={linkIncident.isPending || linkable.length === 0}
                    className="ow-input h-9 w-full min-w-0 rounded-md px-3 text-sm disabled:opacity-50"
                  >
                    <option value="">
                      {linkable.length === 0
                        ? t("noLinkableIncidents")
                        : t("linkIncidentPlaceholder")}
                    </option>
                    {linkable.map((incident) => (
                      <option key={incident.id} value={incident.id}>
                        {incident.title}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
            </section>

            {onCancel && capabilities.canCancelRelease && !terminal ? (
              <section className="p-4">
                <Button
                  fullWidth
                  variant="danger"
                  className={actionButtonClassNames()}
                  loading={cancelPending}
                  onClick={onCancel}
                >
                  {t("cancelRelease")}
                </Button>
              </section>
            ) : null}
          </div>
        </aside>
      </div>
    </div>
  );
}
