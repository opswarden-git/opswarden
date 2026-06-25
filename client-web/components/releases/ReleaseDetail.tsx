"use client";

import React, { useState } from "react";
import { useTranslations } from "next-intl";
import { Check, CircleDashed, Link2, Link2Off, Rocket, Ban as BanIcon } from "lucide-react";
import {
  type Release,
  useCancelRelease,
  useLinkIncident,
  useUnlinkIncident,
  useValidateStep,
} from "@/lib/queries/releases";
import { useIncidents } from "@/lib/queries/incidents";
import { ReleaseStateChip } from "./ReleaseStateChip";
import { StateChip } from "@/components/incidents/StateChip";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";

type Role = "manager" | "responder" | "observer";

export function ReleaseDetail({
  release,
  teamId,
  role,
}: {
  release: Release;
  teamId: string;
  role: Role;
}) {
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");
  const { data: incidents } = useIncidents(teamId);

  const validateStep = useValidateStep();
  const linkIncident = useLinkIncident();
  const unlinkIncident = useUnlinkIncident();
  const cancelRelease = useCancelRelease();

  const [confirmCancel, setConfirmCancel] = useState(false);

  const canAct = role === "manager" || role === "responder";
  const canCancel = role === "manager";
  const terminal = release.state === "completed" || release.state === "cancelled";
  const validatable = release.state === "created" || release.state === "in_progress";

  const nextStepIndex = release.steps.findIndex((s) => !s.validated);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const lastError =
    validateStep.error || linkIncident.error || unlinkIncident.error || cancelRelease.error;

  // Incidents eligible to link: active (not resolved) and not already linked.
  const linkable = (incidents ?? []).filter(
    (incident) =>
      incident.status !== "resolved" && !release.linked_incident_ids.includes(incident.id),
  );
  const incidentById = (id: string) => (incidents ?? []).find((incident) => incident.id === id);

  return (
    <div className="surface overflow-hidden rounded-md">
      <div className="border-border flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-6 py-4">
        <Rocket className="text-gold h-5 w-5" />
        <h2 className="text-text truncate text-lg font-semibold tracking-tight">{release.title}</h2>
        <ReleaseStateChip state={release.state} />
        <span className="text-muted/60 ml-auto text-xs">
          {new Date(release.created_at).toLocaleString()}
        </span>
      </div>

      {lastError ? (
        <div className="ow-danger m-4 rounded-md p-3 text-sm">{errorText(lastError.message)}</div>
      ) : null}

      {/* Steps */}
      <div className="border-border border-b px-6 py-4">
        <h3 className="text-muted/70 mb-3 text-xs font-medium tracking-wider uppercase">
          {t("steps")}
        </h3>
        <ol className="space-y-2">
          {release.steps.map((step, i) => {
            const isNext = i === nextStepIndex;
            return (
              <li
                key={step.name}
                className="surface-subtle border-border flex items-center gap-3 rounded-md border px-3 py-2"
              >
                <span
                  className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-xs ${
                    step.validated ? "bg-st-res/15 text-st-res" : "border-border text-muted border"
                  }`}
                >
                  {step.validated ? <Check className="h-3.5 w-3.5" /> : i + 1}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="text-text truncate text-sm font-medium">{step.name}</div>
                  {step.validated && step.validated_at ? (
                    <div className="text-muted/50 text-xs">
                      {t("validatedBy", {
                        who: step.validated_by?.split("-")[0] ?? "—",
                        when: new Date(step.validated_at).toLocaleString(),
                      })}
                    </div>
                  ) : null}
                </div>
                {isNext && canAct && validatable ? (
                  <button
                    type="button"
                    onClick={() =>
                      validateStep.mutate({
                        releaseId: release.release_id,
                        step: step.name,
                        teamId,
                      })
                    }
                    disabled={validateStep.isPending}
                    className="ow-primary h-8 shrink-0 rounded-md px-3 text-xs font-medium transition-colors disabled:opacity-50"
                  >
                    {t("validate")}
                  </button>
                ) : isNext && release.state === "blocked" ? (
                  <span className="text-sev-critical inline-flex items-center gap-1 text-xs">
                    <BanIcon className="h-3.5 w-3.5" />
                    {t("blockedShort")}
                  </span>
                ) : !step.validated ? (
                  <CircleDashed className="text-muted/40 h-4 w-4 shrink-0" />
                ) : null}
              </li>
            );
          })}
        </ol>
      </div>

      {/* Linked incidents */}
      <div className="px-6 py-4">
        <h3 className="text-muted/70 mb-3 text-xs font-medium tracking-wider uppercase">
          {t("linkedIncidents")}
        </h3>
        {release.linked_incident_ids.length === 0 ? (
          <p className="text-muted/60 text-sm">{t("noLinkedIncidents")}</p>
        ) : (
          <ul className="space-y-2">
            {release.linked_incident_ids.map((id) => {
              const incident = incidentById(id);
              return (
                <li
                  key={id}
                  className="surface-subtle border-border flex items-center gap-3 rounded-md border px-3 py-2"
                >
                  <div className="min-w-0 flex-1">
                    <div className="text-text truncate text-sm">
                      {incident ? incident.title : `${id.split("-")[0]}…`}
                    </div>
                  </div>
                  {incident ? <StateChip status={incident.status} /> : null}
                  {canAct ? (
                    <button
                      type="button"
                      onClick={() =>
                        unlinkIncident.mutate({
                          releaseId: release.release_id,
                          incidentId: id,
                          teamId,
                        })
                      }
                      disabled={unlinkIncident.isPending}
                      title={t("unlink")}
                      aria-label={t("unlink")}
                      className="text-muted hover:text-sev-high rounded-md p-1.5 transition-colors disabled:opacity-40"
                    >
                      <Link2Off className="h-4 w-4" />
                    </button>
                  ) : null}
                </li>
              );
            })}
          </ul>
        )}

        {canAct && !terminal ? (
          <div className="mt-3 flex items-center gap-2">
            <Link2 className="text-muted h-4 w-4 shrink-0" />
            <select
              value=""
              onChange={(e) => {
                const incidentId = e.target.value;
                if (incidentId)
                  linkIncident.mutate({ releaseId: release.release_id, incidentId, teamId });
              }}
              disabled={linkIncident.isPending || linkable.length === 0}
              className="ow-input flex h-9 flex-1 cursor-pointer rounded-md px-3 py-2 text-sm transition-colors disabled:opacity-50"
            >
              <option value="" className="bg-bg text-text">
                {linkable.length === 0 ? t("noLinkableIncidents") : t("linkIncidentPlaceholder")}
              </option>
              {linkable.map((incident) => (
                <option key={incident.id} value={incident.id} className="bg-bg text-text">
                  {incident.title}
                </option>
              ))}
            </select>
          </div>
        ) : null}
      </div>

      {/* Cancel (Manager-only) */}
      {canCancel && !terminal ? (
        <div className="border-border flex items-center justify-between gap-4 border-t bg-white/[0.015] px-6 py-4">
          <p className="text-muted/60 text-xs">{t("cancelReleaseDesc")}</p>
          <button
            type="button"
            onClick={() => {
              cancelRelease.reset();
              setConfirmCancel(true);
            }}
            className="ow-danger inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md px-3 text-sm font-medium transition-colors"
          >
            <BanIcon className="h-4 w-4" />
            {t("cancelRelease")}
          </button>
        </div>
      ) : null}

      <ConfirmDialog
        open={confirmCancel}
        title={t("cancelRelease")}
        description={t("cancelConfirm", { title: release.title })}
        confirmLabel={t("cancelRelease")}
        cancelLabel={t("keep")}
        pendingLabel={t("processing")}
        danger
        pending={cancelRelease.isPending}
        error={cancelRelease.error ? errorText(cancelRelease.error.message) : null}
        onConfirm={() =>
          cancelRelease.mutate(
            { releaseId: release.release_id, teamId },
            { onSuccess: () => setConfirmCancel(false) },
          )
        }
        onClose={() => setConfirmCancel(false)}
      />
    </div>
  );
}
