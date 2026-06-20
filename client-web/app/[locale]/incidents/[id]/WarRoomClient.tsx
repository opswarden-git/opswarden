"use client";

import React, { useEffect } from "react";
import { useAssignIncident, useIncident, useUpdateIncidentStatus } from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";
import { useWsStore, useWatchers } from "@/lib/ws";
import { StateChip } from "@/components/incidents/StateChip";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { Timeline } from "@/components/incidents/Timeline";
import { ArrowLeft, CheckCircle2, Clock, ShieldAlert, UserPlus } from "lucide-react";
import { Link } from "@/i18n/routing";

export function WarRoomClient({ id }: { id: string }) {
  const { data: incident, isLoading, error } = useIncident(id);
  const updateStatus = useUpdateIncidentStatus();
  const assignIncident = useAssignIncident();
  const user = useAuthStore((s) => s.user);

  const watch = useWsStore((s) => s.watch);
  const unwatch = useWsStore((s) => s.unwatch);
  const watchers = useWatchers(id);

  useEffect(() => {
    watch(id);
    return () => unwatch(id);
  }, [id, watch, unwatch]);

  if (isLoading) {
    return <div className="text-muted animate-pulse p-10 text-center">Loading War Room...</div>;
  }

  if (error || !incident) {
    return (
      <div className="mx-auto max-w-5xl p-6 text-center">
        <p className="text-sev-critical">Failed to load incident details.</p>
        <Link href="/incidents" className="text-gold mt-4 inline-block hover:underline">
          Return to Incidents
        </Link>
      </div>
    );
  }

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
            <span className="text-muted font-mono text-sm">ID: {incident.id.split("-")[0]}</span>
            <StateChip status={incident.status} />
            <SeverityChip severity={incident.severity} />
            <span className="text-muted/60 text-xs">
              T-Minus: {new Date(incident.created_at).toLocaleString()}
            </span>
            {incident.assignee ? (
              <span className="surface-subtle text-text border-border inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium">
                <UserPlus className="text-gold h-3 w-3" />
                {incident.assignee.split("-")[0]}
              </span>
            ) : (
              <span className="surface-subtle text-muted/60 border-border inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium">
                Unassigned
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
              <span className="text-text text-xs font-medium">{watchers.length} Watchers</span>
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
            <h3 className="text-text mb-4 font-bold">Command Actions</h3>
            <div className="space-y-3">
              <button
                onClick={() => {
                  if (user?.id) {
                    assignIncident.mutate({ incidentId: incident.id, assigneeId: user.id });
                  }
                }}
                disabled={!user?.id || incident.assignee === user.id || assignIncident.isPending}
                className="ow-primary flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <UserPlus className="h-4 w-4" />
                Assume Lead
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "acknowledged" })
                }
                disabled={incident.status !== "open" || updateStatus.isPending}
                className="ow-secondary text-text hover:border-st-ack/50 hover:bg-st-ack/10 flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <Clock className="text-st-ack h-4 w-4" />
                Acknowledge
              </button>

              <button
                onClick={() =>
                  updateStatus.mutate({ incidentId: incident.id, status: "escalated" })
                }
                disabled={incident.status !== "acknowledged" || updateStatus.isPending}
                className="border-st-esc/25 bg-st-esc/10 text-st-esc hover:bg-st-esc/20 flex h-10 w-full items-center justify-center gap-2 rounded-md border px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <ShieldAlert className="h-4 w-4" />
                Escalate
              </button>

              <button
                onClick={() => updateStatus.mutate({ incidentId: incident.id, status: "resolved" })}
                disabled={
                  incident.status === "resolved" ||
                  incident.status === "open" ||
                  updateStatus.isPending
                }
                className="ow-secondary text-text hover:border-st-res/50 hover:bg-st-res/10 flex h-10 w-full items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
              >
                <CheckCircle2 className="text-st-res h-4 w-4" />
                Resolve
              </button>
            </div>
            {updateStatus.isError && (
              <p className="text-sev-critical mt-4 text-center text-xs">
                {updateStatus.error.message}
              </p>
            )}
            {assignIncident.isError && (
              <p className="text-sev-critical mt-4 text-center text-xs">
                {assignIncident.error.message}
              </p>
            )}
          </div>

          <div className="surface flex-1 rounded-md p-6">
            <h3 className="text-text mb-4 font-bold">Payload Details</h3>
            <p className="text-muted text-sm whitespace-pre-wrap">
              <span className="text-text">Title:</span> {incident.title}
              <br />
              <br />
              <span className="text-text">Severity Level:</span> {incident.severity.toUpperCase()}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
