"use client";

import React, { useState } from "react";
import {
  useAssignIncident,
  useDeleteIncident,
  useIncident,
  useUpdateIncidentStatus,
  type IncidentSeverity,
} from "@/lib/queries/incidents";
import { useTeamMembers } from "@/lib/queries/teams";
import { useWsStore, useWatchers } from "@/lib/ws";
import { StateChip } from "@/components/incidents/StateChip";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { Timeline } from "@/components/incidents/Timeline";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { ArrowLeft, CheckCircle2, Clock, ShieldAlert, Trash2, UserPlus } from "lucide-react";
import { Link, useRouter } from "@/i18n/routing";
import { useTranslations } from "next-intl";

const SEVERITY_KEY: Record<IncidentSeverity, string> = {
  low: "severityLow",
  medium: "severityMedium",
  high: "severityHigh",
  critical: "severityCritical",
};

export function WarRoomClient({ id }: { id: string }) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const router = useRouter();

  const { data: incident, isLoading, error } = useIncident(id);
  const { data: members } = useTeamMembers(incident?.team_id);
  const updateStatus = useUpdateIncidentStatus();
  const assignIncident = useAssignIncident();
  const deleteIncident = useDeleteIncident();

  const [assigneeId, setAssigneeId] = useState("");
  const [deleteOpen, setDeleteOpen] = useState(false);

  const watch = useWsStore((s) => s.watch);
  const unwatch = useWsStore((s) => s.unwatch);
  const watchers = useWatchers(id);

  React.useEffect(() => {
    watch(id);
    return () => unwatch(id);
  }, [id, watch, unwatch]);

  if (isLoading) {
    return <div className="text-muted animate-pulse p-10 text-center">{t("loadingWarRoom")}</div>;
  }

  if (error || !incident) {
    return (
      <div className="mx-auto max-w-5xl p-6 text-center">
        <p className="text-sev-critical">{t("failedToLoadIncident")}</p>
        <Link href="/incidents" className="text-gold mt-4 inline-block hover:underline">
          {t("returnToIncidents")}
        </Link>
      </div>
    );
  }

  // Only Managers and Responders may carry an incident; never Observers.
  const eligibleAssignees = (members ?? []).filter(
    (m) => m.role === "manager" || m.role === "responder",
  );
  const assigneeMember = (members ?? []).find((m) => m.user_id === incident.assignee);
  const assigneeLabel = assigneeMember?.email ?? incident.assignee?.split("-")[0];

  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const onAssign = () => {
    if (!assigneeId) return;
    assignIncident.mutate({ incidentId: incident.id, assigneeId });
  };

  const onDelete = () =>
    deleteIncident.mutate(incident.id, {
      onSuccess: () => router.push("/incidents"),
    });

  return (
    <div className="mx-auto flex h-[calc(100vh-80px)] max-w-6xl flex-col space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Link
          href="/incidents"
          className="ow-secondary text-muted hover:text-text hover:border-gold rounded-md p-2 transition-colors"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div className="flex-1">
          <h1 className="text-text text-2xl font-bold tracking-tight">{incident.title}</h1>
          <div className="mt-2 flex items-center gap-4">
            <span className="text-muted font-mono text-sm">
              {t("idLabel")}: {incident.id.split("-")[0]}
            </span>
            <StateChip status={incident.status} />
            <SeverityChip severity={incident.severity} />
            <span className="text-muted/60 text-xs">
              {t("openedAt")}: {new Date(incident.created_at).toLocaleString()}
            </span>
            {incident.assignee ? (
              <span className="surface-subtle text-text border-border inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium">
                <UserPlus className="text-gold h-3 w-3" />
                {assigneeLabel}
              </span>
            ) : (
              <span className="surface-subtle text-muted/60 border-border inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium">
                {t("unassigned")}
              </span>
            )}
            <div className="surface-subtle border-border ml-auto flex items-center gap-2 rounded-full border px-3 py-1">
              <span className="relative flex h-2 w-2">
                {watchers.length > 0 ? (
                  <>
                    <span className="bg-st-res absolute inline-flex h-full w-full animate-ping rounded-full opacity-75"></span>
                    <span className="bg-st-res relative inline-flex h-2 w-2 rounded-full"></span>
                  </>
                ) : (
                  <span className="bg-muted/50 relative inline-flex h-2 w-2 rounded-full"></span>
                )}
              </span>
              <span className="text-text text-xs font-medium">
                {t("watchers", { count: watchers.length })}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-3 gap-6">
        <div className="surface col-span-2 flex flex-col overflow-hidden rounded-md">
          <Timeline incidentId={incident.id} />
        </div>

        <div className="col-span-1 flex flex-col space-y-4">
          <div className="surface rounded-md p-6">
            <h3 className="text-text mb-4 font-bold">{t("commandActions")}</h3>
            <div className="space-y-3">
              <div className="flex gap-2">
                <select
                  value={assigneeId}
                  onChange={(e) => setAssigneeId(e.target.value)}
                  disabled={eligibleAssignees.length === 0}
                  className="ow-input flex h-10 min-w-0 flex-1 rounded-md px-3 py-2 text-sm transition-colors disabled:opacity-50"
                >
                  <option value="" className="bg-bg text-text">
                    {t("assignResponder")}
                  </option>
                  {eligibleAssignees.map((member) => (
                    <option key={member.user_id} value={member.user_id} className="bg-bg text-text">
                      {member.email}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  onClick={onAssign}
                  disabled={
                    !assigneeId ||
                    assigneeId === incident.assignee ||
                    assignIncident.isPending ||
                    eligibleAssignees.length === 0
                  }
                  className="ow-primary flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
                >
                  <UserPlus className="h-4 w-4" />
                  {t("assign")}
                </button>
              </div>
              {eligibleAssignees.length === 0 ? (
                <p className="text-muted/60 text-xs">{t("noEligibleAssignee")}</p>
              ) : null}

              <button
                type="button"
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "acknowledged" })
                }
                disabled={incident.status !== "open" || updateStatus.isPending}
                className="ow-secondary text-text hover:border-st-ack/50 hover:bg-st-ack/10 flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <Clock className="text-st-ack h-4 w-4" />
                {t("acknowledge")}
              </button>

              <button
                type="button"
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "escalated" })
                }
                disabled={incident.status !== "acknowledged" || updateStatus.isPending}
                className="border-st-esc/25 bg-st-esc/10 text-st-esc hover:bg-st-esc/20 flex h-10 w-full items-center justify-center gap-2 rounded-md border px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <ShieldAlert className="h-4 w-4" />
                {t("escalate")}
              </button>

              <button
                type="button"
                onClick={() => updateStatus.mutate({ incidentId: incident.id, status: "resolved" })}
                disabled={
                  incident.status === "resolved" ||
                  incident.status === "open" ||
                  updateStatus.isPending
                }
                className="ow-secondary text-text hover:border-st-res/50 hover:bg-st-res/10 flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <CheckCircle2 className="text-st-res h-4 w-4" />
                {t("resolve")}
              </button>
            </div>
            {updateStatus.error ? (
              <p className="text-sev-critical mt-4 text-center text-xs">
                {errorText(updateStatus.error.message)}
              </p>
            ) : null}
            {assignIncident.error ? (
              <p className="text-sev-critical mt-4 text-center text-xs">
                {errorText(assignIncident.error.message)}
              </p>
            ) : null}
          </div>

          <div className="surface flex-1 rounded-md p-6">
            <h3 className="text-text mb-4 font-bold">{t("payloadDetails")}</h3>
            <p className="text-muted text-sm whitespace-pre-wrap">
              <span className="text-text">{t("fieldTitle")}:</span> {incident.title}
              <br />
              <br />
              <span className="text-text">{t("fieldSeverity")}:</span>{" "}
              {t(SEVERITY_KEY[incident.severity])}
            </p>
          </div>

          <div className="surface rounded-md p-6">
            <button
              type="button"
              onClick={() => {
                deleteIncident.reset();
                setDeleteOpen(true);
              }}
              className="ow-danger flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors"
            >
              <Trash2 className="h-4 w-4" />
              {t("deleteIncident")}
            </button>
          </div>
        </div>
      </div>

      <ConfirmDialog
        open={deleteOpen}
        title={t("deleteIncident")}
        description={t("deleteIncidentConfirm", { title: incident.title })}
        confirmLabel={t("deleteIncident")}
        cancelLabel={t("cancel")}
        pendingLabel={t("processing")}
        danger
        requireType="DELETE"
        pending={deleteIncident.isPending}
        error={deleteIncident.error ? errorText(deleteIncident.error.message) : null}
        onConfirm={onDelete}
        onClose={() => setDeleteOpen(false)}
      />
    </div>
  );
}
